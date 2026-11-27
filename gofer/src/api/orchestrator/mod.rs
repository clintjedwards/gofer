//! Everything that happens to a run between someone asking for it and it finishing. Three layers share the work, and
//! each one only hands work down to the next:
//!
//! - **[`Orchestrator`]** is the front door, owned by [`ApiState`]. [`Orchestrator::launch_new_run`] checks the
//!   request, writes the run as `pending`, and wakes the dispatcher. It never starts anything itself, which is why
//!   starting a run returns right away. At boot, [`Orchestrator::recover_runs`] picks back up the runs that were
//!   `running` when Gofer stopped.
//!
//! - **The dispatcher** ([`dispatch_queued_runs`]) is a single loop and the only place a run gets started. Each pass
//!   reads the queue and the running counts from the database, picks the runs that fit under their pipeline's limit
//!   and the global one ([`dispatcher::select_runs_to_start`]), and claims each with a conditional update so a run can't be both
//!   started and cancelled. There's only one, so two runs can never take the same slot. It wakes whenever a run is
//!   queued or finishes, plus a periodic check in case a wake-up goes missing.
//!
//! - **The run shepherd** ([`RunShepherd`]) is one per running run. It starts a task per task execution, plus
//!   `monitor_run`, which watches the event bus until every task is done. Then it marks the run complete, which frees
//!   the slot and wakes the dispatcher again.
//!
//! A run's state follows those layers:
//!
//! ```text
//! pending --(dispatcher claims it)--> running --(shepherd finishes it)--> complete
//! pending --(cancelled, or its pipeline was disabled, before it started)--> complete
//! ```
//!
//! A run asked for on one of its pipeline's skip dates is the exception. It's written as `complete` right away, so it
//! never reaches the dispatcher.

mod cleanup;
mod dispatcher;
mod expiry;
mod run;
mod task_execution;
mod variables;

pub use cleanup::sweep_finished_runs;
pub use dispatcher::complete_queued_run;
pub use variables::task_api_base_url;

use dispatcher::dispatch_queued_runs;
use run::RunShepherd;

use crate::{
    api::{
        ApiState, Variable, VariableSource, event_utils, generate_inject_api_token_role_id,
        pipeline_configs, pipelines, runs, secrets, tokens,
    },
    secret_store, storage,
};

use std::collections::HashMap;
use std::sync::{Arc, atomic};
use thiserror::Error;
use tokio::sync::Notify;
use tracing::{debug, error, info};

pub fn run_specific_api_key_id(run_id: u64) -> String {
    format!("gofer_api_token_run_id_{run_id}")
}

#[derive(Debug, Error)]
pub enum OrchestratorError {
    #[error("Pipeline run request ignored due to 'ignore_pipeline_run_events' being true")]
    PipelineRunIgnored,

    #[error("Database error occurred")]
    DatabaseGeneralError(#[source] storage::StorageError),

    #[error("Could not serialize object from database")]
    DatabaseSerializationError(#[source] anyhow::Error),

    #[error("Could not find pipeline metadata")]
    PipelineMetadataNotFound,

    #[error("Cannot start run due to pipeline being in inactive state")]
    PipelineInactive,

    #[error("Cannot start run since the pipeline has no deployed config")]
    NoLiveConfig,

    #[error("An error occurred that should generally never happen and cannot be recovered from")]
    UnrecoverableError(String),

    #[error("Object store error occurred")]
    #[allow(dead_code)]
    ObjectStoreGeneralError(#[source] object_store::Error),

    #[error("Secret store error occurred")]
    SecretStoreGeneralError(#[source] secret_store::SecretStoreError),
}

/// How often the dispatcher checks the queue even when nothing woke it. It's only a safety net; queuing or finishing
/// a run wakes it straight away.
const DISPATCH_INTERVAL: std::time::Duration = std::time::Duration::from_secs(5);

#[derive(Debug, Default)]
pub struct Orchestrator {
    /// Wakes the dispatcher when something it cares about changes: a run was queued or finished, or runs were
    /// unpaused.
    dispatch_notify: Arc<Notify>,
}

impl Orchestrator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Asks the dispatcher to look at the queue. If it's busy, it looks again once it's done, so a wake-up is never
    /// lost.
    pub fn notify_dispatcher(&self) {
        self.dispatch_notify.notify_one();
    }

    /// Starts the dispatcher, the one place runs are started from. New runs only get queued; the dispatcher decides
    /// when each one has a free slot. Having a single dispatcher is what keeps two runs from taking the same slot.
    pub fn start_dispatcher(&self, api_state: Arc<ApiState>) {
        let notify = self.dispatch_notify.clone();

        tokio::spawn(async move {
            loop {
                if let Err(e) = dispatch_queued_runs(&api_state).await {
                    error!(error = %e, "Could not dispatch queued runs");
                }

                tokio::select! {
                    _ = notify.notified() => {}
                    _ = tokio::time::sleep(DISPATCH_INTERVAL) => {}
                }
            }
        });
    }

    pub async fn launch_new_run(
        &self,
        api_state: Arc<ApiState>,
        namespace_id: &str,
        pipeline_id: &str,
        initiator: runs::Initiator,
        variables: HashMap<String, String>,
        force: bool,
    ) -> std::result::Result<runs::Run, OrchestratorError> {
        if api_state
            .ignore_pipeline_run_events
            .load(atomic::Ordering::SeqCst)
        {
            debug!(
                "Ignoring pipeline run due to api setting 'ignore_pipeline_run_events' in state 'true'"
            );
            return Err(OrchestratorError::PipelineRunIgnored);
        }

        let mut tx = match api_state.storage.open_tx().await {
            Ok(conn) => conn,
            Err(e) => {
                return Err(OrchestratorError::DatabaseGeneralError(e));
            }
        };

        // First we need to confirm that the pipeline exists and is not in an inactive state.

        let storage_pipeline_metadata =
            match storage::pipeline_metadata::get(&mut tx, namespace_id, pipeline_id).await {
                Ok(pipeline) => pipeline,
                Err(e) => match e {
                    storage::StorageError::NotFound => {
                        return Err(OrchestratorError::PipelineMetadataNotFound);
                    }
                    _ => {
                        return Err(OrchestratorError::DatabaseGeneralError(e));
                    }
                },
            };

        let stored_state = storage_pipeline_metadata.state.clone();

        let pipeline_metadata = pipelines::Metadata::try_from(storage_pipeline_metadata)
            .map_err(OrchestratorError::DatabaseSerializationError)?;

        // The pipeline's timed disable has run out. Reads already treat it as active; this just tidies up what's stored.
        // It happens here since this transaction holds the write lock, so it can't undo someone disabling it again.
        if pipeline_metadata.state.to_string() != stored_state
            && let Err(e) = storage::pipeline_metadata::clear_expired_disable(
                &mut tx,
                namespace_id,
                pipeline_id,
            )
            .await
        {
            return Err(OrchestratorError::DatabaseGeneralError(e));
        }

        if pipeline_metadata.state != pipelines::PipelineState::Active {
            return Err(OrchestratorError::PipelineInactive);
        };

        // Runs use the deployed (live) config, not just the newest one; a config can be registered without being
        // deployed and that shouldn't change what runs.
        let latest_pipeline_config_storage = match storage::pipeline_configs::get_latest_w_state(
            &mut tx,
            namespace_id,
            pipeline_id,
            &pipeline_configs::ConfigState::Live.to_string(),
        )
        .await
        {
            Ok(config) => config,
            Err(storage::StorageError::NotFound) => {
                return Err(OrchestratorError::NoLiveConfig);
            }
            Err(e) => {
                return Err(OrchestratorError::DatabaseGeneralError(e));
            }
        };

        let live_config =
            pipeline_configs::Config::from_storage(latest_pipeline_config_storage.clone(), vec![])
                .map_err(OrchestratorError::DatabaseSerializationError)?;

        let today = chrono::Utc::now().date_naive();
        let skip_date = if force {
            None
        } else {
            live_config.skip_date_on(today)
        };

        let pipeline_tasks = match storage::tasks::list(
            &mut tx,
            namespace_id,
            pipeline_id,
            latest_pipeline_config_storage.version,
        )
        .await
        {
            Ok(tasks) => tasks,
            Err(e) => {
                return Err(OrchestratorError::DatabaseGeneralError(e));
            }
        };

        // A skipped run never starts, so it has no use for a token.
        let token_required =
            skip_date.is_none() && pipeline_tasks.iter().any(|task| task.inject_api_token);

        let latest_run_id =
            match storage::runs::get_latest(&mut tx, namespace_id, pipeline_id).await {
                Ok(latest_run) => latest_run.run_id,
                Err(err) => match err {
                    storage::StorageError::NotFound => 0,
                    _ => {
                        return Err(OrchestratorError::DatabaseGeneralError(err));
                    }
                },
            };

        let new_run_id = latest_run_id + 1;

        // Next we need to determine if we need to generate a new api key for this run for the purposes of the
        // inject_api_token feature, that allows users to easily use the Gofer cli from inside their run.
        //
        // If the pipeline has just one task that requires an API token, we generate that API token for the entire run
        // and associate it with the run. Downstream, the function that policies if the task needs the api token will
        // decide whether to inject it in the env vars or not.

        let mut token_id: Option<String> = None;

        if token_required {
            let (token, hash) = tokens::create_new_api_token();

            let new_token = tokens::Token::new(
                &hash,
                HashMap::from([
                    ("namespace".into(), namespace_id.to_string()),
                    ("pipeline".into(), pipeline_id.to_string()),
                    ("run".into(), format!("{}", new_run_id)),
                ]),
                1_814_400, // 3 weeks in seconds.
                format!("{namespace_id}/{pipeline_id}#{new_run_id}"),
                tokens::TokenKind::Run,
                vec![generate_inject_api_token_role_id(namespace_id, pipeline_id)],
            );

            let new_token_storage = match new_token.clone().try_into() {
                Ok(token) => token,
                Err(e) => {
                    return Err(OrchestratorError::DatabaseSerializationError(e));
                }
            };

            if let Err(e) = storage::tokens::insert(&mut tx, &new_token_storage).await {
                match e {
                    storage::StorageError::Exists => {
                        return Err(OrchestratorError::UnrecoverableError(
                            "token entry already exists while attempting \
                            to create new 'inject_api_token' for run"
                                .to_string(),
                        ));
                    }
                    _ => return Err(OrchestratorError::DatabaseGeneralError(e)),
                }
            };

            let new_secret =
                secrets::Secret::new(&run_specific_api_key_id(new_run_id as u64), vec![]);

            let new_secret_storage =
                match new_secret.to_pipeline_secret_storage(namespace_id, pipeline_id) {
                    Ok(secret) => secret,
                    Err(e) => {
                        return Err(OrchestratorError::DatabaseSerializationError(e));
                    }
                };

            if let Err(e) =
                storage::secret_store_pipeline_keys::insert(&mut tx, &new_secret_storage).await
            {
                match e {
                    storage::StorageError::Exists => {
                        return Err(OrchestratorError::UnrecoverableError(
                            "secret entry already exists while attempting \
                            to create new 'inject_api_token' for run"
                                .to_string(),
                        ));
                    }
                    _ => {
                        return Err(OrchestratorError::DatabaseGeneralError(e));
                    }
                }
            };

            if let Err(e) = api_state
                .secret_store
                .put(
                    &secrets::pipeline_secret_store_key(
                        namespace_id,
                        pipeline_id,
                        &run_specific_api_key_id(new_run_id as u64),
                    ),
                    token.as_bytes().to_vec(),
                    false,
                )
                .await
            {
                return Err(OrchestratorError::SecretStoreGeneralError(e));
            };

            token_id = Some(new_token.id);
        }

        let mut new_run = runs::Run::new(
            namespace_id,
            pipeline_id,
            latest_pipeline_config_storage.version.try_into().unwrap(),
            new_run_id.try_into().unwrap(),
            initiator,
            variables
                .into_iter()
                .map(|(key, value)| Variable {
                    key,
                    value,
                    source: VariableSource::RunOptions,
                })
                .collect(),
            token_id,
        );

        // A skipped run is still recorded, so it's clear why nothing ran that day.
        if let Some(skip_date) = skip_date {
            new_run.state = runs::State::Complete;
            new_run.status = runs::Status::Skipped;
            new_run.status_reason = Some(runs::StatusReason {
                reason: runs::StatusReasonType::SkipDate,
                description: format!(
                    "{today} (UTC) is covered by the pipeline's skip date '{skip_date}'"
                ),
            });
            new_run.ended = new_run.created;
        }

        let new_run_storage = new_run
            .clone()
            .try_into()
            .map_err(|err: anyhow::Error| OrchestratorError::DatabaseSerializationError(err))?;

        if let Err(e) = storage::runs::insert(&mut tx, &new_run_storage).await {
            match e {
                storage::StorageError::Exists => {
                    return Err(OrchestratorError::UnrecoverableError(
                        "run entry already exists".to_string(),
                    ));
                }
                _ => {
                    return Err(OrchestratorError::DatabaseGeneralError(e));
                }
            }
        };

        // It was never queued, so there's nothing to recover and nothing for the dispatcher to start.
        if skip_date.is_some() {
            if let Err(e) = tx.commit().await {
                return Err(OrchestratorError::DatabaseGeneralError(
                    storage::StorageError::Connection(e.to_string()),
                ));
            };

            api_state
                .event_bus
                .clone()
                .publish(event_utils::Kind::CompletedRun {
                    namespace_id: namespace_id.to_string(),
                    pipeline_id: pipeline_id.to_string(),
                    run_id: new_run.run_id,
                    status: new_run.status.clone(),
                });

            return Ok(new_run);
        }

        // We emit the QueuedRun event so that the event stream is aware of the impending run. Then we insert that event_id
        // into the database for the associated run. This helps us with detecting which runs are incomplete due to
        // a crash within Gofer and helps us restore those runs.
        let queued_run_event = api_state
            .event_bus
            .clone()
            .publish(event_utils::Kind::QueuedRun {
                namespace_id: namespace_id.to_string().clone(),
                pipeline_id: pipeline_id.to_string().clone(),
                run_id: new_run.run_id,
            });

        let fields = storage::runs::UpdatableFields {
            event_id: Some(queued_run_event.id),
            ..Default::default()
        };

        if let Err(e) = storage::runs::update(
            &mut tx,
            namespace_id,
            pipeline_id,
            new_run.run_id as i64,
            fields,
        )
        .await
        {
            return Err(OrchestratorError::DatabaseGeneralError(e));
        };

        if let Err(e) = tx.commit().await {
            return Err(OrchestratorError::DatabaseGeneralError(
                storage::StorageError::Connection(e.to_string()),
            ));
        };

        // The run is queued as pending; the dispatcher starts it once its pipeline has a free slot.
        self.notify_dispatcher();

        Ok(new_run)
    }

    pub async fn recover_runs(
        &self,
        api_state: Arc<ApiState>,
    ) -> std::result::Result<(), OrchestratorError> {
        let mut conn = match api_state.storage.read_conn().await {
            Ok(conn) => conn,
            Err(e) => {
                return Err(OrchestratorError::DatabaseGeneralError(e));
            }
        };

        // Only runs that were already running need recovering. Pending runs never started, so the dispatcher just
        // picks them up like any other queued run.
        let mut running_runs = vec![];
        let page_size = 100;
        loop {
            let page =
                match storage::runs::list_running(&mut conn, running_runs.len() as i64, page_size)
                    .await
                {
                    Ok(val) => val,
                    Err(e) => {
                        return Err(OrchestratorError::DatabaseGeneralError(e));
                    }
                };

            let last_page = (page.len() as i64) < page_size;
            running_runs.extend(page);

            if last_page {
                break;
            }
        }

        for stored_run in running_runs {
            info!(
                namespace_id = stored_run.namespace_id.clone(),
                pipeline_id = stored_run.pipeline_id.clone(),
                run_id = stored_run.run_id,
                "Recovering unfinished run"
            );

            let run: runs::Run = stored_run
                .try_into()
                .map_err(|err: anyhow::Error| OrchestratorError::DatabaseSerializationError(err))?;

            let run_event_id = run.event_id.clone().unwrap_or_default();

            let new_run_shepherd = RunShepherd::load(api_state.clone(), &mut conn, run).await?;

            tokio::spawn(new_run_shepherd.start_run_recovery(run_event_id));
        }

        Ok(())
    }
}
