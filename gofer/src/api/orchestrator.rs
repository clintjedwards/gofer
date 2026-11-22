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
//!   and the global one ([`select_runs_to_start`]), and claims each with a conditional update so a run can't be both
//!   started and cancelled. There's only one, so two runs can never take the same slot. It wakes whenever a run is
//!   queued or finishes, plus a periodic check in case a wake-up goes missing.
//!
//! - **The run shepherd** ([`Run`]) is one per running run. It starts a task per task execution, plus
//!   `monitor_run`, which watches the event bus until every task is done. Then it marks the run complete, which frees
//!   the slot and wakes the dispatcher again.
//!
//! A run's state follows those layers:
//!
//! ```text
//! pending --(dispatcher claims it)--> running --(shepherd finishes it)--> complete
//! pending --(cancelled, or its pipeline was disabled, before it started)--> complete
//! ```

use crate::{
    api::{
        ApiState, GOFER_EOF, Variable, VariableSource, epoch_milli,
        event_utils::{self, EventListener},
        generate_inject_api_token_role_id, interpolate_vars, objects, pipeline_configs, pipelines,
        resolve_registry_auth, runs, secrets, task_executions, tasks, tokens,
    },
    conf, scheduler, secret_store, storage,
};

use anyhow::{Context, Result, bail};
use futures::StreamExt;
use gofer_sdk::config::pipeline_secret;
use sqlx::SqliteConnection;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::{Arc, atomic};
use thiserror::Error;
use tokio::{
    io::AsyncWriteExt,
    sync::{Barrier, Mutex, Notify, mpsc},
};
use tracing::{debug, error, info, trace};

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

        let pipeline_metadata = pipelines::Metadata::try_from(storage_pipeline_metadata)
            .map_err(OrchestratorError::DatabaseSerializationError)?;

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

        let token_required = pipeline_tasks.iter().any(|task| task.inject_api_token);

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

        let new_run = runs::Run::new(
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

            let new_run_shepard = Run::load(api_state.clone(), &mut conn, run).await?;

            tokio::spawn(new_run_shepard.start_run_recovery(run_event_id));
        }

        Ok(())
    }
}

/// The most runs a pipeline can have going at once: its own `parallelism`, capped by the server's per-pipeline limit.
/// 0 means unlimited, both for the inputs and the result.
fn pipeline_run_limit(pipeline_parallelism: u64, server_pipeline_limit: u64) -> u64 {
    match (pipeline_parallelism, server_pipeline_limit) {
        (0, server) => server,
        (pipeline, 0) => pipeline,
        (pipeline, server) => pipeline.min(server),
    }
}

/// Picks which queued runs can start right now. `queued` must be oldest first; runs are taken in that order, so the
/// queue is first come first served across all of Gofer. A pipeline that's at its limit is skipped instead of holding
/// up runs of other pipelines behind it. `global_limit` of 0 means unlimited.
fn select_runs_to_start(
    queued: &[storage::runs::QueuedRun],
    running: &HashMap<(String, String), u64>,
    server_pipeline_limit: u64,
    global_limit: u64,
) -> Vec<storage::runs::QueuedRun> {
    let mut running = running.clone();
    let mut total_running: u64 = running.values().sum();
    let mut selected = vec![];

    for run in queued {
        if global_limit != 0 && total_running >= global_limit {
            break;
        }

        let limit = pipeline_run_limit(
            run.parallelism.try_into().unwrap_or_default(),
            server_pipeline_limit,
        );

        let pipeline_running = running
            .entry((run.namespace_id.clone(), run.pipeline_id.clone()))
            .or_default();

        if limit != 0 && *pipeline_running >= limit {
            continue;
        }

        *pipeline_running += 1;
        total_running += 1;
        selected.push(run.clone());
    }

    selected
}

/// Finishes a run that never started, as long as it's still pending. Returns false if the dispatcher started it in the
/// meantime, in which case it has to be cancelled like any other running run.
pub async fn complete_queued_run(
    api_state: &ApiState,
    conn: &mut SqliteConnection,
    namespace_id: &str,
    pipeline_id: &str,
    run_id: u64,
    status: runs::Status,
    reason: runs::StatusReason,
) -> Result<bool> {
    let reason = serde_json::to_string(&reason).context("Could not serialize run status reason")?;

    let completed = storage::runs::complete_pending(
        conn,
        namespace_id,
        pipeline_id,
        run_id.try_into()?,
        &epoch_milli().to_string(),
        &status.to_string(),
        &reason,
    )
    .await
    .context("Could not update run in database")?;

    if completed {
        api_state
            .event_bus
            .clone()
            .publish(event_utils::Kind::CompletedRun {
                namespace_id: namespace_id.into(),
                pipeline_id: pipeline_id.into(),
                run_id,
                status,
            });
    }

    Ok(completed)
}

/// Starts every queued run that has a free slot. Only the dispatcher calls this, one call at a time.
async fn dispatch_queued_runs(api_state: &Arc<ApiState>) -> Result<()> {
    // Pausing run events holds queued runs too, so the switch can quiet Gofer down before maintenance.
    if api_state
        .ignore_pipeline_run_events
        .load(atomic::Ordering::SeqCst)
    {
        return Ok(());
    }

    let mut conn = api_state
        .storage
        .write_conn()
        .await
        .context("Could not open connection to database")?;

    let queued = storage::runs::list_queued(&mut conn)
        .await
        .context("Could not list queued runs")?;

    if queued.is_empty() {
        return Ok(());
    }

    let running: HashMap<(String, String), u64> = storage::runs::count_running(&mut conn)
        .await
        .context("Could not count running runs")?
        .into_iter()
        .map(|count| {
            (
                (count.namespace_id, count.pipeline_id),
                count.count.try_into().unwrap_or_default(),
            )
        })
        .collect();

    // A run waiting on a pipeline that has since been disabled would never start, so it's cancelled instead.
    let active = pipelines::PipelineState::Active.to_string();
    let mut startable = vec![];

    for run in queued {
        if run.pipeline_state == active {
            startable.push(run);
            continue;
        }

        complete_queued_run(
            api_state,
            &mut conn,
            &run.namespace_id,
            &run.pipeline_id,
            run.run_id.try_into()?,
            runs::Status::Cancelled,
            runs::StatusReason {
                reason: runs::StatusReasonType::AdminCancelled,
                description: "Pipeline was disabled before the run started".into(),
            },
        )
        .await?;
    }

    let selected = select_runs_to_start(
        &startable,
        &running,
        api_state.config.api.pipeline_run_concurrency_limit,
        api_state.config.api.global_run_concurrency_limit,
    );

    for queued_run in selected {
        let run_id: u64 = queued_run.run_id.try_into()?;

        let stored_run = storage::runs::get(
            &mut conn,
            &queued_run.namespace_id,
            &queued_run.pipeline_id,
            queued_run.run_id,
        )
        .await
        .context("Could not get queued run from database")?;

        let run: runs::Run = stored_run.try_into()?;

        // A run whose pipeline can't be loaded would fail the same way every time, so it's failed instead of being
        // left in the queue.
        let mut shepard = match Run::load(api_state.clone(), &mut conn, run).await {
            Ok(shepard) => shepard,
            Err(e) => {
                error!(namespace_id = queued_run.namespace_id, pipeline_id = queued_run.pipeline_id,
                    run_id, error = %e, "Could not load queued run");

                complete_queued_run(
                    api_state,
                    &mut conn,
                    &queued_run.namespace_id,
                    &queued_run.pipeline_id,
                    run_id,
                    runs::Status::Failed,
                    runs::StatusReason {
                        reason: runs::StatusReasonType::FailedPrecondition,
                        description: format!("Could not load the run's pipeline; {e}"),
                    },
                )
                .await?;

                continue;
            }
        };

        let started = epoch_milli();

        let claimed = storage::runs::claim(
            &mut conn,
            &queued_run.namespace_id,
            &queued_run.pipeline_id,
            queued_run.run_id,
            &started.to_string(),
        )
        .await
        .context("Could not claim queued run")?;

        // It was cancelled while it waited.
        if !claimed {
            continue;
        }

        shepard.run.state = runs::State::Running;
        shepard.run.started = started;

        tokio::spawn(shepard.start_run());
    }

    Ok(())
}

/// A run specific object that guides Gofer runs and tasks through their execution.
/// It's a core construct within the Gofer execution model and contains most of the logic of how a run operates.
#[derive(Debug, Clone)]
struct Run {
    api_state: Arc<ApiState>,
    pipeline: pipelines::Pipeline,
    run: runs::Run,
}

impl Run {
    pub fn new(api_state: Arc<ApiState>, pipeline: pipelines::Pipeline, run: runs::Run) -> Self {
        Self {
            api_state,
            pipeline,
            run,
        }
    }

    /// Builds the shepherd for a run that's already in the database, using the config version it was created with
    /// so a config deployed later doesn't change a run that's already queued or running.
    pub async fn load(
        api_state: Arc<ApiState>,
        conn: &mut SqliteConnection,
        run: runs::Run,
    ) -> std::result::Result<Self, OrchestratorError> {
        let storage_pipeline_metadata = match storage::pipeline_metadata::get(
            conn,
            &run.namespace_id,
            &run.pipeline_id,
        )
        .await
        {
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

        let pipeline_metadata = pipelines::Metadata::try_from(storage_pipeline_metadata)
            .map_err(OrchestratorError::DatabaseSerializationError)?;

        let pipeline_config_storage = match storage::pipeline_configs::get(
            conn,
            &run.namespace_id,
            &run.pipeline_id,
            run.pipeline_config_version as i64,
        )
        .await
        {
            Ok(config) => config,
            Err(e) => return Err(OrchestratorError::DatabaseGeneralError(e)),
        };

        let pipeline_tasks = match storage::tasks::list(
            conn,
            &run.namespace_id,
            &run.pipeline_id,
            run.pipeline_config_version as i64,
        )
        .await
        {
            Ok(tasks) => tasks,
            Err(e) => return Err(OrchestratorError::DatabaseGeneralError(e)),
        };

        let pipeline_config =
            pipeline_configs::Config::from_storage(pipeline_config_storage, pipeline_tasks)
                .map_err(OrchestratorError::DatabaseSerializationError)?;

        Ok(Self::new(
            api_state,
            pipelines::Pipeline {
                metadata: pipeline_metadata,
                config: pipeline_config,
            },
            run,
        ))
    }

    /// Start run launches the run and its tasks and then listens to the event bus for related run events and task events.
    /// Upon receiving one of these events it then appropriately updates the run entry in the database with the
    /// correct data.
    ///
    /// Only the dispatcher calls this, after it has claimed the run, so the run is already marked as running.
    pub async fn start_run(self) -> Result<()> {
        trace!(
            namespace_id = self.pipeline.metadata.namespace_id.clone(),
            pipeline_id = self.pipeline.metadata.pipeline_id.clone(),
            run_id = self.run.run_id,
            "Starting run"
        );

        // First launch per-run clean up jobs.
        // These jobs help keep resources from filling up.
        tokio::spawn(self.clone().handle_run_object_expiry());
        tokio::spawn(self.clone().handle_run_log_expiry());

        // Then make sure people who need to know that this run is starting are informed.
        self.api_state
            .event_bus
            .clone()
            .publish(event_utils::Kind::StartedRun {
                namespace_id: self.pipeline.metadata.namespace_id.clone(),
                pipeline_id: self.pipeline.metadata.pipeline_id.clone(),
                run_id: self.run.run_id,
            });

        // Configs are checked when they're registered, but a bad dependency graph that slipped through anyway (a bug,
        // or a config from before the check existed) would leave tasks waiting on each other forever. Failing the
        // whole run up front beats running part of it.
        let dag_errors = gofer_sdk::dag::validate(
            self.pipeline
                .config
                .tasks
                .values()
                .map(|task| (task.id.as_str(), task.depends_on.keys().map(String::as_str))),
        );

        if !dag_errors.is_empty() {
            let problems: Vec<String> = dag_errors.iter().map(|e| e.to_string()).collect();

            let mut conn = self
                .api_state
                .storage
                .write_conn()
                .await
                .context("Could not open connection to database")?;

            self.set_run_complete(
                &mut conn,
                runs::Status::Failed,
                Some(runs::StatusReason {
                    reason: runs::StatusReasonType::FailedPrecondition,
                    description: format!(
                        "Pipeline config has an invalid task graph; {}",
                        problems.join("; ")
                    ),
                }),
            )
            .await?;

            return Ok(());
        }

        // Lastly start the run monitor and the launch the task executions. We use a barrier here to make sure
        // that all threads are able to grab event bus listeners at roughly the same point so that they don't
        // end up missing any messages from threads that might be faster.

        // The barrier knows when to tell all tasks to continue by waiting until all tasks check in.
        // We calculate this value by taking the number of all the tasks we are about to start and then adding
        // one more for the run monitor itself.
        let barrier_threshold = self.pipeline.config.tasks.len() + 1;

        let barrier = Arc::new(tokio::sync::Barrier::new(barrier_threshold));

        for task in self.pipeline.config.tasks.values() {
            let event_stream = self.api_state.event_bus.subscribe_live();

            let thread_barrier = barrier.clone();
            tokio::spawn(self.clone().launch_task_execution(
                thread_barrier,
                Box::new(event_stream),
                task.clone(),
            ));
        }

        let thread_barrier = barrier.clone();
        let self_clone = self.clone();
        let event_stream = self.api_state.event_bus.subscribe_live();
        tokio::spawn(async move {
            self_clone
                .monitor_run(thread_barrier, Box::new(event_stream))
                .await
        });

        Ok(())
    }

    /// Performs all actions that [`start_run`] does, but with the added context of attempting to recover the run
    /// rather than starting a brand new one. This means that only a subset of the tasks will be executed and the event
    /// stream starts from a historical point rather than the most up to date.
    ///
    /// run_event_id is the UUID of the "Started_Run" event for the run being recovered.
    pub async fn start_run_recovery(self, run_event_id: String) -> Result<()> {
        trace!(
            namespace_id = self.pipeline.metadata.namespace_id.clone(),
            pipeline_id = self.pipeline.metadata.pipeline_id.clone(),
            run_id = self.run.run_id,
            run_event_id = run_event_id,
            "Recovering run"
        );

        // First launch per-run clean up jobs.
        // These jobs help keep resources from filling up.
        tokio::spawn(self.clone().handle_run_object_expiry());
        tokio::spawn(self.clone().handle_run_log_expiry());

        // Then make sure people who need to know that this run is starting are informed.
        self.api_state
            .event_bus
            .clone()
            .publish(event_utils::Kind::StartedRun {
                namespace_id: self.pipeline.metadata.namespace_id.clone(),
                pipeline_id: self.pipeline.metadata.pipeline_id.clone(),
                run_id: self.run.run_id,
            });

        // Lastly start the run monitor and the launch the task executions. We use a barrier here to make sure
        // that all threads are able to grab event bus listeners at roughly the same point so that they don't
        // end up missing any messages from threads that might be faster.

        // The barrier knows when to tell all tasks to continue by waiting until all tasks check in.
        // We calculate this value by taking the number of all the tasks we are about to start and then adding
        // one more for the run monitor itself.
        let mut unfinished_task_list = vec![];

        {
            let mut conn = self
                .api_state
                .storage
                .read_conn()
                .await
                .context("Could not open connection to database")?;

            let task_executions = storage::task_executions::list(
                &mut conn,
                &self.pipeline.metadata.namespace_id,
                &self.pipeline.metadata.pipeline_id,
                self.run.run_id.try_into().unwrap_or_default(),
            )
            .await
            .map_err(|e| {
                anyhow::anyhow!(
                    "Could not get task execution list while attempting to recover run; {:#?}",
                    e
                )
            })?;

            for task in task_executions {
                let task: task_executions::TaskExecution = task.try_into().map_err(|e| {
                    anyhow::anyhow!(
                        "Could not serialize task execution while attempting to query for unfinished tasks; {:#?}", e
                    )
                })?;

                if task.state == task_executions::State::Complete {
                    continue;
                }

                unfinished_task_list.push(task);
            }
        }

        let barrier_threshold = unfinished_task_list.len() + 1;

        let barrier = Arc::new(tokio::sync::Barrier::new(barrier_threshold));

        for task in unfinished_task_list {
            let thread_barrier = barrier.clone();
            let event_stream = self
                .api_state
                .event_bus
                .subscribe_historical(Some(run_event_id.to_string()))
                .await
                .map_err(|e| {
                    anyhow::anyhow!(
                        "Could not establish event stream while attempting to recover unfinished run; {:#?}", e
                    )
                })?;

            tokio::spawn(self.clone().launch_task_execution(
                thread_barrier,
                Box::new(event_stream),
                task.task.clone(),
            ));
        }

        let thread_barrier = barrier.clone();
        let self_clone = self.clone();
        let event_stream = self
                .api_state
                .event_bus
                .subscribe_historical(Some(run_event_id.to_string()))
                .await
                .map_err(|e| {
                    anyhow::anyhow!(
                        "Could not establish event stream while attempting to recover unfinished run; {:#?}", e
                    )
                })?;

        tokio::spawn(async move {
            self_clone
                .monitor_run(thread_barrier, Box::new(event_stream))
                .await
        });

        Ok(())
    }

    /// Listens to messages from the event bus and updates the status of the run in-progress.
    async fn monitor_run(
        &self,
        barrier: Arc<tokio::sync::Barrier>,
        mut event_stream: Box<dyn EventListener>,
    ) {
        // wait for all the other threads to get to this point so everyone starts out at the same point in the event bus.
        barrier.wait().await;

        let mut completed_tasks = std::collections::HashMap::new();
        let mut is_cancelled = false;
        let mut is_failed = false;

        // Wait for events and then process what should happen after we recieve them.
        loop {
            let event = match event_stream.next().await {
                Ok(event) => event,
                Err(err) => {
                    error!(error = %err,
                           namespace_id = self.pipeline.metadata.namespace_id.clone(),
                           pipeline_id = self.pipeline.metadata.pipeline_id.clone(),
                           run_id = self.run.run_id,
                           "Could not receive event from event stream during run monitoring.");
                    continue;
                }
            };

            // When we get an event see if its an event that pertains to us and then handle it.
            match event.kind {
                event_utils::Kind::CompletedTaskExecution {
                    namespace_id,
                    pipeline_id,
                    run_id,
                    task_execution_id,
                    status,
                } => {
                    // Make sure we only handle events for our specific run.
                    if namespace_id != self.pipeline.metadata.namespace_id
                        || pipeline_id != self.pipeline.metadata.pipeline_id
                        || run_id != self.run.run_id
                    {
                        continue;
                    }

                    completed_tasks.insert(task_execution_id, status);
                }
                event_utils::Kind::StartedTaskExecutionCancellation {
                    namespace_id,
                    pipeline_id,
                    run_id,
                    ..
                } => {
                    // Make sure we only handle events for our specific run.
                    if namespace_id != self.pipeline.metadata.namespace_id
                        || pipeline_id != self.pipeline.metadata.pipeline_id
                        || run_id != self.run.run_id
                    {
                        continue;
                    }
                    is_cancelled = true;
                }
                event_utils::Kind::StartedRunCancellation {
                    namespace_id,
                    pipeline_id,
                    run_id,
                } => {
                    // Make sure we only handle events for our specific run.
                    if namespace_id != self.pipeline.metadata.namespace_id
                        || pipeline_id != self.pipeline.metadata.pipeline_id
                        || run_id != self.run.run_id
                    {
                        continue;
                    }

                    // When we get a notification of a run cancellation we start issuing cancellation requests to all
                    // task executions.
                    //
                    // When they are all completed we then mark the run as cancelled.

                    for task in self.pipeline.config.tasks.values() {
                        _ = self.api_state.event_bus.clone().publish(
                            event_utils::Kind::StartedTaskExecutionCancellation {
                                namespace_id: self.pipeline.metadata.namespace_id.clone(),
                                pipeline_id: self.pipeline.metadata.pipeline_id.clone(),
                                run_id: self.run.run_id,
                                task_execution_id: task.id.clone(),
                                timeout: self
                                    .api_state
                                    .config
                                    .api
                                    .task_execution_stop_timeout_seconds,
                            },
                        );
                    }

                    is_cancelled = true;
                }
                _ => {}
            }

            // If we are under the total amount of tasks then we still need to wait for the tasks to complete.
            // If we aren't then we can just break and mark the run as complete.
            if completed_tasks.len() == self.pipeline.config.tasks.len() {
                break;
            }
        }

        // Run is complete so we determine which state it finished in and stop.

        for status in completed_tasks.values() {
            if *status == task_executions::Status::Failed {
                is_failed = true;
            }
        }

        let mut conn = match self.api_state.storage.write_conn().await {
            Ok(conn) => conn,
            Err(e) => {
                error!(namespace_id = &self.pipeline.metadata.namespace_id,
                    pipeline_id = &self.pipeline.metadata.pipeline_id,
                    run_id = self.run.run_id,
                    error = %e, "Could not establish connection to database while attempting to complete complete run");
                return;
            }
        };

        if is_cancelled {
            let result = self
                .set_run_complete(
                    &mut conn,
                    runs::Status::Cancelled,
                    Some(runs::StatusReason {
                        reason: runs::StatusReasonType::AbnormalExit,
                        description: "One or more task executions were cancelled during execution"
                            .into(),
                    }),
                )
                .await;

            if let Err(e) = result {
                error!(
                namespace_id = &self.pipeline.metadata.namespace_id,
                pipeline_id = &self.pipeline.metadata.pipeline_id,
                error = %e,
                "Could not set run finished while attempting to wait for finish");
            }
            return;
        }

        if is_failed {
            // A timeout is easy to mistake for the task's own failure, so when one happened the run says so and
            // which timeout it was, instead of the generic message.
            let timed_out = self.timed_out_task_executions(&mut conn).await;
            let reason = if timed_out.is_empty() {
                runs::StatusReason {
                    reason: runs::StatusReasonType::AbnormalExit,
                    description: "One or more task executions failed during execution".into(),
                }
            } else {
                runs::StatusReason {
                    reason: runs::StatusReasonType::Timeout,
                    description: timed_out.join("; "),
                }
            };

            let result = self
                .set_run_complete(&mut conn, runs::Status::Failed, Some(reason))
                .await;

            if let Err(e) = result {
                error!(
                    namespace_id = &self.pipeline.metadata.namespace_id,
                    pipeline_id = &self.pipeline.metadata.pipeline_id,
                    error = %e,
                    "Could not set run finished while attempting to wait for finish");
            }
            return;
        }

        if let Err(e) = self
            .set_run_complete(&mut conn, runs::Status::Successful, None)
            .await
        {
            error!(
                namespace_id = &self.pipeline.metadata.namespace_id,
                pipeline_id = &self.pipeline.metadata.pipeline_id,
                run_id = &self.run.run_id,
                error = %e,
                "Could not set run finished while attempting to wait for finish");
        }
    }

    /// Launches a brand new task execution as part of a larger run for a specific task.
    /// It blocks until the task execution has completed, reading and posting to the event bus to facilitate further
    /// run actions.
    async fn launch_task_execution(
        self,
        barrier: Arc<Barrier>,
        mut event_stream: Box<dyn EventListener>,
        task: tasks::Task,
    ) {
        // wait for all the other threads to get to this point so everyone starts out at the same point in the event bus.
        barrier.wait().await;

        // Start by creating a new task execution and saving it to the state machine and disk.
        let new_task_execution = task_executions::TaskExecution::new(
            &self.pipeline.metadata.namespace_id,
            &self.pipeline.metadata.pipeline_id,
            self.run.run_id,
            task.clone(),
        );

        let mut conn = match self.api_state.storage.write_conn().await {
            Ok(conn) => conn,
            Err(e) => {
                error!(namespace_id = &self.pipeline.metadata.namespace_id,
                    pipeline_id = &self.pipeline.metadata.pipeline_id,
                    run_id = self.run.run_id,
                    task_id = task.id,
                    error = %e, "Could not establish connection to database");
                return;
            }
        };

        let storage_task_execution = match new_task_execution.clone().try_into() {
            Ok(execution) => execution,
            Err(e) => {
                error!(namespace_id = &self.pipeline.metadata.namespace_id,
                    pipeline_id = &self.pipeline.metadata.pipeline_id,
                    run_id = self.run.run_id,
                    task_id = task.id,
                    error = %e, "Could not serialize task execution to storage object");
                return;
            }
        };

        if let Err(e) = storage::task_executions::insert(&mut conn, &storage_task_execution).await {
            match e {
                // If the task execution already exists then we're probably attempting to recover it.
                storage::StorageError::Exists => {}

                // If it's any other error we probably want to return.
                _ => {
                    error!(namespace_id = &self.pipeline.metadata.namespace_id,
                            pipeline_id = &self.pipeline.metadata.pipeline_id,
                            run_id = self.run.run_id,
                            task_id = task.id,
                            error = %e, "Could not insert new task_execution into storage");
                    return;
                }
            }
        }

        let env_vars = combine_variables(
            &self.run,
            &task,
            &task_api_base_url(&self.api_state.config.server),
        );
        let env_vars_json = match serde_json::to_string(&env_vars) {
            Ok(env_vars) => env_vars,
            Err(e) => {
                error!(namespace_id = &self.pipeline.metadata.namespace_id,
                    pipeline_id = &self.pipeline.metadata.pipeline_id,
                    run_id = self.run.run_id,
                    task_id = task.id,
                    error = %e, "Could not serialize env vars into json");
                return;
            }
        };

        // Determine the task executions final variable set and pass them in.
        let run_id_i64 = match self.run.run_id.try_into() {
            Ok(id) => id,
            Err(e) => {
                error!(namespace_id = &self.pipeline.metadata.namespace_id,
                    pipeline_id = &self.pipeline.metadata.pipeline_id,
                    run_id = self.run.run_id,
                    task_id = task.id,
                    error = %e, "Could not convert run id to appropriate integer type");
                return;
            }
        };

        if let Err(e) = storage::task_executions::update(
            &mut conn,
            &self.pipeline.metadata.namespace_id,
            &self.pipeline.metadata.pipeline_id,
            run_id_i64,
            &task.id,
            storage::task_executions::UpdatableFields {
                variables: Some(env_vars_json),
                ..Default::default()
            },
        )
        .await
        {
            error!(namespace_id = &self.pipeline.metadata.namespace_id,
                pipeline_id = &self.pipeline.metadata.pipeline_id,
                run_id = self.run.run_id,
                task_id = task.id,
                error = %e, "Could not update task_execution with correct variables");
            return;
        };

        if let Err(e) = self
            .set_task_execution_state(
                &mut conn,
                &new_task_execution,
                task_executions::State::Waiting,
            )
            .await
        {
            error!(namespace_id = &self.pipeline.metadata.namespace_id,
                pipeline_id = &self.pipeline.metadata.pipeline_id,
                run_id = self.run.run_id,
                task_id = task.id,
                error = %e, "Could not update task_execution state to waiting");
            return;
        };

        // We drop the write connection here to not hold onto it while we wait for events.
        drop(conn);

        // On recovery the task execution starts over, so its timeline does too.
        let timeline = self.spawn_timeline_recorder(&task.id);

        if !new_task_execution.task.depends_on.is_empty() {
            _ = timeline.send(task_executions::TimelineEntry::now(
                task_executions::Phase::WaitingOnDependencies,
            ));
        }

        // <task_execution_id> => <status>
        let mut completed_tasks = HashMap::new();

        // Wait for events and then process what should happen after we recieve them.
        // When we get an event see if its an event that pertains to us and then handle it.
        // At this stage we haven't started the task execution yet, we're simply checking to see
        // if we should start it.
        if !new_task_execution.task.depends_on.is_empty() {
            'main_loop: loop {
                let event = match event_stream.next().await {
                    Ok(event) => event,
                    Err(err) => {
                        error!(error = %err,
                           namespace_id = self.pipeline.metadata.namespace_id.clone(),
                           pipeline_id = self.pipeline.metadata.pipeline_id.clone(),
                           run_id = self.run.run_id,
                           "Could not recieve event from event stream during run monitoring.");
                        continue;
                    }
                };

                match event.kind {
                    // Listen for parent task executions and log them to see when we should run.
                    event_utils::Kind::CompletedTaskExecution {
                        namespace_id,
                        pipeline_id,
                        run_id,
                        task_execution_id,
                        status,
                    } => {
                        // Make sure we only handle events for our specific run's tasks.
                        if namespace_id != self.pipeline.metadata.namespace_id
                            || pipeline_id != self.pipeline.metadata.pipeline_id
                            || run_id != self.run.run_id
                        {
                            continue;
                        }

                        completed_tasks.insert(task_execution_id, status);
                    }

                    // Listen to see if we should stop the container and set task execution as cancelled.
                    event_utils::Kind::StartedTaskExecutionCancellation {
                        namespace_id,
                        pipeline_id,
                        run_id,
                        task_execution_id,
                        ..
                    } => {
                        // Make sure we only handle events for our specific task execution.
                        if namespace_id != self.pipeline.metadata.namespace_id
                            || pipeline_id != self.pipeline.metadata.pipeline_id
                            || run_id != self.run.run_id
                            || task_execution_id != task.id
                        {
                            continue;
                        }

                        let mut conn = match self.api_state.storage.write_conn().await {
                            Ok(conn) => conn,
                            Err(e) => {
                                error!(namespace_id = &self.pipeline.metadata.namespace_id,
                                    pipeline_id = &self.pipeline.metadata.pipeline_id,
                                    run_id = self.run.run_id,
                                    task_id = task.id,
                                    error = %e, "Could not establish connection to database");
                                return;
                            }
                        };

                        if let Err(e) = self
                            .set_task_execution_complete(
                                &mut conn,
                                &task.id,
                                1,
                                task_executions::Status::Cancelled,
                                None,
                            )
                            .await
                        {
                            error!(error = %e,
                                   namespace_id = self.pipeline.metadata.namespace_id.clone(),
                                   pipeline_id = self.pipeline.metadata.pipeline_id.clone(),
                                   run_id = self.run.run_id,
                                   task_execution_id = task.id.clone(),
                                   "Could not recieve event from event stream during run monitoring.");
                        }

                        // Close the connection here to not hold it during publish.
                        drop(conn);

                        self.api_state.event_bus.clone().publish(
                            event_utils::Kind::CompletedTaskExecution {
                                namespace_id: self.pipeline.metadata.namespace_id.clone(),
                                pipeline_id: self.pipeline.metadata.pipeline_id.clone(),
                                run_id: self.run.run_id,
                                task_execution_id: new_task_execution.task_id.clone(),
                                status: task_executions::Status::Cancelled,
                            },
                        );

                        return;
                    }
                    _ => {}
                }

                // Here we need to see if our parents exist in the set that contains completed tasks.
                // If it does we launch our own, if it doesn't we continue the loop.
                for parent_id in new_task_execution.task.depends_on.keys() {
                    if !completed_tasks.contains_key(parent_id) {
                        continue 'main_loop;
                    }
                }

                break;
            }
        }

        let mut conn = match self.api_state.storage.write_conn().await {
            Ok(conn) => conn,
            Err(e) => {
                error!(namespace_id = &self.pipeline.metadata.namespace_id,
                       pipeline_id = &self.pipeline.metadata.pipeline_id,
                       run_id = self.run.run_id,
                       task_id = task.id,
                       error = %e, "Could not establish connection to database");
                return;
            }
        };

        if let Err(e) = self
            .set_task_execution_state(
                &mut conn,
                &new_task_execution,
                task_executions::State::Processing,
            )
            .await
        {
            error!(namespace_id = &self.pipeline.metadata.namespace_id,
                pipeline_id = &self.pipeline.metadata.pipeline_id,
                run_id = self.run.run_id,
                task_id = task.id,
                error = %e, "Could not update task_execution state to processing");
            return;
        };

        drop(conn);

        // Then check to make sure that the parents all finished in the required states. If not
        // we'll mark this task as skipped since it's requirements for running weren't met.
        if let Err(e) =
            self.task_dependencies_satisfied(completed_tasks, &new_task_execution.task.depends_on)
        {
            let mut conn = match self.api_state.storage.write_conn().await {
                Ok(conn) => conn,
                Err(e) => {
                    error!(namespace_id = &self.pipeline.metadata.namespace_id,
                            pipeline_id = &self.pipeline.metadata.pipeline_id,
                            run_id = self.run.run_id,
                            task_id = task.id,
                            error = %e, "Could not establish connection to database");
                    return;
                }
            };

            if let Err(e) = self
                .set_task_execution_complete(
                    &mut conn,
                    &new_task_execution.task_id,
                    1,
                    task_executions::Status::Skipped,
                    Some(task_executions::StatusReason {
                        reason: task_executions::StatusReasonType::FailedPrecondition,
                        description: format!(
                            "Task could not be run due to unmet dependencies; {}",
                            e
                        ),
                    }),
                )
                .await
            {
                error!(namespace_id = &self.pipeline.metadata.namespace_id,
                    pipeline_id = &self.pipeline.metadata.pipeline_id,
                    run_id = self.run.run_id,
                    task_id = task.id,
                    error = %e, "Could not mark task execution as skipped during the processing of task dependencies");
                return;
            }

            return;
        };

        // After this point we're sure the task is in a state to be run. So we attempt to
        // contact the scheduler and start the container.

        _ = timeline.send(task_executions::TimelineEntry::now(
            task_executions::Phase::Preparing,
        ));

        // First we attempt to find any object/secret store variables and replace them
        // with the correct var. At first glance this may seem like a task that can move upwards
        // but it's important that this run only after a task's parents have already run
        // this enables users to be sure that one task can pass variables to other downstream tasks.

        // We create a copy of variables so that we can substitute in secrets and objects.
        // to eventually pass them into the start container function. Registry auth can reference secrets too, so it
        // gets resolved here as well and fails the task the same way.
        let resolved = async {
            let env_vars = interpolate_vars(
                &self.api_state.storage,
                self.api_state.secret_store.as_ref(),
                self.api_state.object_store.as_ref(),
                &self.pipeline.metadata.namespace_id,
                &self.pipeline.metadata.pipeline_id,
                Some(self.run.run_id),
                &env_vars,
            )
            .await?;

            let registry_auth = match &new_task_execution.task.registry_auth {
                Some(auth) => Some(
                    resolve_registry_auth(
                        &self.api_state.storage,
                        self.api_state.secret_store.as_ref(),
                        &self.pipeline.metadata.namespace_id,
                        &self.pipeline.metadata.pipeline_id,
                        auth,
                    )
                    .await?,
                ),
                None => None,
            };

            Ok::<_, anyhow::Error>((env_vars, registry_auth))
        }
        .await;

        let (env_vars, registry_auth) = match resolved {
            Ok(resolved) => resolved,
            Err(e) => {
                let mut conn = match self.api_state.storage.write_conn().await {
                    Ok(conn) => conn,
                    Err(e) => {
                        error!(namespace_id = &self.pipeline.metadata.namespace_id,
                            pipeline_id = &self.pipeline.metadata.pipeline_id,
                            run_id = self.run.run_id,
                            task_id = task.id,
                            error = %e, "Could not establish connection to database");
                        return;
                    }
                };

                if let Err(e) = self
                    .set_task_execution_complete(
                        &mut conn,
                        &new_task_execution.task_id,
                        1,
                        task_executions::Status::Failed,
                        Some(task_executions::StatusReason {
                            reason: task_executions::StatusReasonType::FailedPrecondition,
                            description: format!(
                                "Task could not be run due to inability to retrieve interpolated variables or registry auth; {}",
                                e
                            ),
                        }),
                    )
                    .await
                {
                    error!(namespace_id = &self.pipeline.metadata.namespace_id,
                        pipeline_id = &self.pipeline.metadata.pipeline_id,
                        run_id = self.run.run_id,
                        task_id = task.id,
                        error = %e, "Could not mark task execution as failed during the processing of task env vars");
                    return;
                };

                error!(namespace_id = &self.pipeline.metadata.namespace_id,
                    pipeline_id = &self.pipeline.metadata.pipeline_id,
                    run_id = self.run.run_id,
                    task_id = task.id,
                    error = %e, "Could not properly interpolate variables for task execution");
                return;
            }
        };

        let container_name = task_executions::task_execution_container_id(
            &self.pipeline.metadata.namespace_id,
            &self.pipeline.metadata.pipeline_id,
            self.run.run_id,
            &new_task_execution.task_id,
        );

        // Captured before scheduling so the task's duration includes pulling its image and starting the container;
        // short tasks can finish before start_container even returns.
        let started = epoch_milli();

        let start_response = match self
            .api_state
            .scheduler
            .start_container(scheduler::StartContainerRequest {
                id: container_name.clone(),
                image: new_task_execution.task.image.clone(),
                variables: env_vars
                    .into_iter()
                    .map(|var| (var.key, var.value))
                    .collect(),
                registry_auth: registry_auth.map(|auth| auth.into()),
                always_pull: new_task_execution.task.always_pull_newest_image,
                networking: None,
                entrypoint: new_task_execution.task.entrypoint.clone(),
                command: new_task_execution.task.command.clone(),
                timeline: Some(timeline.clone()),
            })
            .await
        {
            Ok(response) => {
                _ = timeline.send(task_executions::TimelineEntry::now(
                    task_executions::Phase::Running,
                ));
                response
            }
            Err(e) => {
                let mut conn = match self.api_state.storage.write_conn().await {
                    Ok(conn) => conn,
                    Err(e) => {
                        error!(namespace_id = &self.pipeline.metadata.namespace_id,
                            pipeline_id = &self.pipeline.metadata.pipeline_id,
                            run_id = self.run.run_id,
                            task_id = task.id,
                            error = %e, "Could not establish connection to database");
                        return;
                    }
                };

                if let Err(e) = self
                    .set_task_execution_complete(
                        &mut conn,
                        &new_task_execution.task_id,
                        1,
                        task_executions::Status::Failed,
                        Some(task_executions::StatusReason {
                            reason: task_executions::StatusReasonType::SchedulerError,
                            description: format!(
                                "Task could not be run due to inability to be scheduled; {}",
                                e
                            ),
                        }),
                    )
                    .await
                {
                    error!(namespace_id = &self.pipeline.metadata.namespace_id,
                    pipeline_id = &self.pipeline.metadata.pipeline_id,
                    run_id = self.run.run_id,
                    task_id = task.id,
                    error = %e, "Could not mark task execution as failed during scheduling of task");
                };
                return;
            }
        };

        trace!(
            namespace_id = self.pipeline.metadata.namespace_id.clone(),
            pipeline_id = self.pipeline.metadata.pipeline_id.clone(),
            run_id = self.run.run_id,
            task_execution_id = &new_task_execution.task_id,
            "Started task execution"
        );

        let mut conn = match self.api_state.storage.write_conn().await {
            Ok(conn) => conn,
            Err(e) => {
                error!(namespace_id = &self.pipeline.metadata.namespace_id,
                            pipeline_id = &self.pipeline.metadata.pipeline_id,
                            run_id = self.run.run_id,
                            task_id = task.id,
                            error = %e, "Could not establish connection to database");
                return;
            }
        };

        if let Err(e) = storage::task_executions::update(
            &mut conn,
            &self.pipeline.metadata.namespace_id,
            &self.pipeline.metadata.pipeline_id,
            run_id_i64,
            &new_task_execution.task_id,
            storage::task_executions::UpdatableFields {
                state: Some(task_executions::State::Running.to_string()),
                started: Some(started.to_string()),
                image_digest: start_response.image_digest,
                ..Default::default()
            },
        )
        .await
        {
            error!(namespace_id = &self.pipeline.metadata.namespace_id,
                pipeline_id = &self.pipeline.metadata.pipeline_id,
                run_id = self.run.run_id,
                task_id = task.id,
                error = %e, "Could not update task execution while attempting to launch task");
            return;
        }

        // Drop the database connection so we don't hold it while long running processes below are happening.
        drop(conn);

        let mut new_task_execution = new_task_execution;
        new_task_execution.state = task_executions::State::Running;

        self.api_state
            .event_bus
            .clone()
            .publish(event_utils::Kind::StartedTaskExecution {
                namespace_id: self.pipeline.metadata.namespace_id.clone(),
                pipeline_id: self.pipeline.metadata.pipeline_id.clone(),
                run_id: self.run.run_id,
                task_execution_id: new_task_execution.task_id.clone(),
            });

        let self_clone = self.clone();
        let container_name_clone = container_name.clone();
        let task_id_clone = new_task_execution.task_id.clone();
        tokio::spawn(async move {
            self_clone
                .handle_log_updates(container_name_clone, task_id_clone)
                .await;
        });

        tokio::spawn(async move {
            if let Err(e) = self
                .clone()
                .monitor_task_execution(event_stream, container_name.clone(), new_task_execution)
                .await
            {
                error!(namespace_id = &self.pipeline.metadata.namespace_id,
                        pipeline_id = &self.pipeline.metadata.pipeline_id,
                        run_id = self.run.run_id,
                        task_id = task.id,
                        error = %e, "Error occurred during monitoring of task execution");
            }
        });
    }

    /// Describes each of this run's task executions that failed by hitting a timeout, like
    /// "build: Task ran longer than its own timeout of 10m and was stopped".
    async fn timed_out_task_executions(&self, conn: &mut SqliteConnection) -> Vec<String> {
        let task_executions = match storage::task_executions::list(
            conn,
            &self.pipeline.metadata.namespace_id,
            &self.pipeline.metadata.pipeline_id,
            self.run.run_id.try_into().unwrap_or_default(),
        )
        .await
        {
            Ok(task_executions) => task_executions,
            Err(e) => {
                error!(namespace_id = &self.pipeline.metadata.namespace_id,
                    pipeline_id = &self.pipeline.metadata.pipeline_id,
                    run_id = self.run.run_id,
                    error = %e, "Could not list task executions while checking for timeouts");
                return vec![];
            }
        };

        task_executions
            .into_iter()
            .filter_map(|task_execution| {
                let task_execution: task_executions::TaskExecution =
                    task_execution.try_into().ok()?;
                let reason = task_execution.status_reason?;
                (reason.reason == task_executions::StatusReasonType::Timeout)
                    .then(|| format!("{}: {}", task_execution.task_id, reason.description))
            })
            .collect()
    }

    async fn set_task_execution_complete(
        &self,
        conn: &mut SqliteConnection,
        id: &str,
        exit_code: u8,
        status: task_executions::Status,
        reason: Option<task_executions::StatusReason>,
    ) -> Result<()> {
        let status_reason = reason.map(|value| {
            serde_json::to_string(&value)
                .context("Could not parse field 'reason' into storage value")
                .unwrap_or_default()
        });

        let fields = storage::task_executions::UpdatableFields {
            ended: Some(epoch_milli().to_string()),
            exit_code: Some(exit_code.into()),
            state: Some(task_executions::State::Complete.to_string()),
            status: Some(status.to_string()),
            status_reason,
            ..Default::default()
        };

        storage::task_executions::update(
            conn,
            &self.pipeline.metadata.namespace_id,
            &self.pipeline.metadata.pipeline_id,
            self.run.run_id.try_into()?,
            id,
            fields,
        )
        .await
        .context("Could not update task execution status in storage")?;

        self.api_state
            .event_bus
            .clone()
            .publish(event_utils::Kind::CompletedTaskExecution {
                namespace_id: self.pipeline.metadata.namespace_id.clone(),
                pipeline_id: self.pipeline.metadata.pipeline_id.clone(),
                run_id: self.run.run_id,
                task_execution_id: id.to_string(),
                status: status.clone(),
            });

        Ok(())
    }

    async fn set_run_complete(
        &self,
        conn: &mut SqliteConnection,
        status: runs::Status,
        reason: Option<runs::StatusReason>,
    ) -> Result<()> {
        let status_reason = reason.map(|value| {
            serde_json::to_string(&value)
                .context("Could not parse field 'reason' into storage value")
                .unwrap_or_default()
        });

        let fields = storage::runs::UpdatableFields {
            ended: Some(epoch_milli().to_string()),
            state: Some(runs::State::Complete.to_string()),
            status: Some(status.to_string()),
            status_reason,
            ..Default::default()
        };

        storage::runs::update(
            conn,
            &self.pipeline.metadata.namespace_id,
            &self.pipeline.metadata.pipeline_id,
            self.run.run_id.try_into()?,
            fields,
        )
        .await
        .context("Could not update run status in storage")?;

        // This run's slot is free now, so a queued run might be able to start.
        self.api_state.orchestrator.notify_dispatcher();

        self.api_state
            .event_bus
            .clone()
            .publish(event_utils::Kind::CompletedRun {
                namespace_id: self.pipeline.metadata.namespace_id.clone(),
                pipeline_id: self.pipeline.metadata.pipeline_id.clone(),
                run_id: self.run.run_id,
                status,
            });

        Ok(())
    }

    async fn set_task_execution_state(
        &self,
        conn: &mut SqliteConnection,
        task_execution: &task_executions::TaskExecution,
        state: task_executions::State,
    ) -> Result<()> {
        let fields = storage::task_executions::UpdatableFields {
            state: Some(state.to_string()),
            ..Default::default()
        };

        storage::task_executions::update(
            conn,
            &self.pipeline.metadata.namespace_id,
            &self.pipeline.metadata.pipeline_id,
            self.run.run_id.try_into()?,
            &task_execution.task_id,
            fields,
        )
        .await
        .context("Could not update task execution status in storage")?;

        Ok(())
    }

    /// Starts a writer that persists a task execution's timeline as phases come in. Phases go through a channel to a
    /// single writer so they're saved in order without racing each other, and so the scheduler can report its own
    /// phases (like pulling an image) while they happen rather than after. The writer stops once every sender is
    /// dropped.
    fn spawn_timeline_recorder(
        &self,
        task_id: &str,
    ) -> mpsc::UnboundedSender<task_executions::TimelineEntry> {
        let (sender, mut receiver) = mpsc::unbounded_channel::<task_executions::TimelineEntry>();
        let api_state = self.api_state.clone();
        let namespace_id = self.pipeline.metadata.namespace_id.clone();
        let pipeline_id = self.pipeline.metadata.pipeline_id.clone();
        let run_id = self.run.run_id;
        let task_id = task_id.to_string();

        tokio::spawn(async move {
            let mut timeline = vec![];

            while let Some(entry) = receiver.recv().await {
                timeline.push(entry);

                let timeline_json = match serde_json::to_string(&timeline) {
                    Ok(json) => json,
                    Err(e) => {
                        error!(namespace_id, pipeline_id, run_id, task_id, error = %e,
                            "Could not serialize task execution timeline");
                        continue;
                    }
                };

                let mut conn = match api_state.storage.write_conn().await {
                    Ok(conn) => conn,
                    Err(e) => {
                        error!(namespace_id, pipeline_id, run_id, task_id, error = %e,
                            "Could not establish connection to database");
                        continue;
                    }
                };

                if let Err(e) = storage::task_executions::update(
                    &mut conn,
                    &namespace_id,
                    &pipeline_id,
                    run_id as i64,
                    &task_id,
                    storage::task_executions::UpdatableFields {
                        timeline: Some(timeline_json),
                        ..Default::default()
                    },
                )
                .await
                {
                    error!(namespace_id, pipeline_id, run_id, task_id, error = %e,
                        "Could not update task execution timeline");
                }
            }
        });

        sender
    }

    /// Check a dependency tree to see if all parent tasks are in the correct states.
    fn task_dependencies_satisfied(
        &self,
        completed_tasks_map: HashMap<String, task_executions::Status>,
        dependency_map: &HashMap<String, tasks::RequiredParentStatus>,
    ) -> Result<()> {
        for (parent, required_status) in dependency_map {
            let parent_status = match completed_tasks_map.get(parent) {
                Some(p) => p.clone(),
                None => bail!(
                    "Could not find parent dependency in task execution list while attempting to \
                verify task dependency satisfaction"
                ),
            };

            match required_status {
                tasks::RequiredParentStatus::Unknown => {
                    bail!("Found a parent dependency in state 'Unknown'; Invalid state")
                }
                tasks::RequiredParentStatus::Any => {
                    if parent_status != task_executions::Status::Successful
                        && parent_status != task_executions::Status::Failed
                        && parent_status != task_executions::Status::Skipped
                    {
                        bail!(
                            "Parent '{}' has incorrect status '{}' for required 'any' dependency",
                            parent,
                            parent_status
                        );
                    }
                }
                tasks::RequiredParentStatus::Success => {
                    if parent_status != task_executions::Status::Successful {
                        bail!(
                            "Parent '{}' has incorrect status '{}' for required 'successful' dependency",
                            parent,
                            parent_status
                        );
                    }
                }
                tasks::RequiredParentStatus::Failure => {
                    if parent_status != task_executions::Status::Failed {
                        bail!(
                            "Parent '{}' has incorrect status '{}' for required 'failed' dependency",
                            parent,
                            parent_status
                        );
                    }
                }
            }
        }

        Ok(())
    }

    /// Monitors the container status from the scheduler and listens for any relevant events.
    /// Update the status of the task accordingly.
    async fn monitor_task_execution(
        &self,
        mut event_stream: Box<dyn EventListener>,
        container_id: String,
        task_execution: task_executions::TaskExecution,
    ) -> Result<()> {
        trace!(
            namespace_id = self.pipeline.metadata.namespace_id.clone(),
            pipeline_id = self.pipeline.metadata.pipeline_id.clone(),
            run_id = self.run.run_id,
            task_execution_id = &task_execution.task_id,
            "Monitoring task execution"
        );

        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(1));
        let task_id = task_execution.task_id.clone();

        // The clock starts once the container is running, so a slow image pull doesn't count against the task.
        let timeout = task_timeout(
            task_execution.task.timeout_seconds,
            self.api_state.config.api.task_execution_max_timeout_seconds,
        );
        let deadline = timeout.as_ref().map(|timeout| {
            tokio::time::Instant::now() + tokio::time::Duration::from_secs(timeout.seconds)
        });

        loop {
            tokio::select! {
                _ = async {
                    match deadline {
                        Some(deadline) => tokio::time::sleep_until(deadline).await,
                        None => std::future::pending().await,
                    }
                } => {
                    // Only reachable when there's a deadline, so there's always a timeout here.
                    let description = timeout.as_ref().map(|t| t.description.clone()).unwrap_or_default();

                    _ = self.api_state
                        .scheduler
                        .cancel_task_execution(
                            task_execution.clone(),
                            self.api_state.config.api.task_execution_stop_timeout_seconds as i64,
                        )
                        .await
                        .map_err(|err| {
                            let all_errors = err
                                .chain()
                                .map(|e| e.to_string())
                                .collect::<Vec<_>>()
                                .join(" | ");

                            error!(error = %all_errors,
                                   namespace_id = self.pipeline.metadata.namespace_id.clone(),
                                   pipeline_id = self.pipeline.metadata.pipeline_id.clone(),
                                   run_id = self.run.run_id,
                                   task_execution_id = &task_id,
                                   "Could not stop task execution after it timed out");
                        });

                    let mut conn = self.api_state.storage.write_conn().await?;

                    if let Err(e) = self
                        .set_task_execution_complete(
                            &mut conn,
                            &task_id,
                            1,
                            task_executions::Status::Failed,
                            Some(task_executions::StatusReason {
                                reason: task_executions::StatusReasonType::Timeout,
                                description,
                            }),
                        )
                        .await
                    {
                        bail!("Could not update task execution while attempting to set execution as complete; {:#?}", e)
                    };

                    return Ok(());
                },
                _ = interval.tick() => {
                    // TODO(): This should be more robust, if when we start implementing networked schedulers there is a chance
                    // that they can fail when we request a status update and we don't want to fail the entire thing just
                    // because they didn't return an update once.
                    let response = match self
                        .api_state
                        .scheduler
                        .get_state(scheduler::GetStateRequest {
                            id: container_id.clone(),
                        })
                        .await
                    {
                        Ok(resp) => resp,
                        Err(err) => {
                            let mut conn = self.api_state.storage.write_conn().await?;


                            if let Err(e) = self
                                .set_task_execution_complete(
                                    &mut conn,
                                    &task_id,
                                    1,
                                    task_executions::Status::Unknown,
                                    Some(task_executions::StatusReason {
                                        reason: task_executions::StatusReasonType::SchedulerError,
                                        description:
                                            "Could not query the scheduler for the task execution state"
                                                .into(),
                                    }),
                                )
                                .await
                            {
                                error!(error = %e, "Could not update task execution while attempting to set execution as complete")
                            };
                            bail!("Could not update task execution while attempting to set execution as complete; {:#?}", err)
                        }
                    };

                    match response.state {
                        scheduler::ContainerState::Unknown => {
                            let mut conn = self.api_state.storage.write_conn().await?;

                            if let Err(e) = self
                                .set_task_execution_complete(
                                    &mut conn,
                                    &task_id,
                                    1,
                                    task_executions::Status::Unknown,
                                    Some(task_executions::StatusReason {
                                        reason: task_executions::StatusReasonType::SchedulerError,
                                        description:
                                            "An unknown error has occurred on the scheduler level; This should (ideally) never happen. Please contact support or file a bug."
                                                .into(),
                                    }),
                                )
                                .await
                            {
                                bail!("Could not update task execution while attempting to set execution as complete; {:#?}", e)
                            };
                            return Ok(());
                        }
                        scheduler::ContainerState::Running
                        | scheduler::ContainerState::Paused
                        | scheduler::ContainerState::Restarting => {}
                        scheduler::ContainerState::Exited => {
                            // We determine if something worked based on the exit code of the container.
                            let mut exit_code = 1;

                            if let Some(code) = response.exit_code {
                                exit_code = code
                            }

                            let mut conn = self.api_state.storage.write_conn().await?;

                            if exit_code == 0 {
                                if let Err(e) = self
                                    .set_task_execution_complete(
                                        &mut conn,
                                        &task_id,
                                        exit_code,
                                        task_executions::Status::Successful,
                                        None,
                                    )
                                    .await
                                {
                                    bail!("Could not update task execution while attempting to set execution as complete; {:#?}", e)
                                };
                            } else { if let Err(e) = self
                                .set_task_execution_complete(
                                    &mut conn,
                                    &task_id,
                                    exit_code,
                                    task_executions::Status::Failed,
                                    Some(task_executions::StatusReason {
                                        reason: task_executions::StatusReasonType::AbnormalExit,
                                        description:
                                            "Task execution has exited with an abnormal exit code.".into(),
                                    }),
                                )
                                .await {
                                bail!("Could not update task execution while attempting to set execution as complete; {:#?}", e)
                            }}

                            return Ok(());
                        }
                    }
                },
                result = event_stream.next() => {
                    let event = match result {
                        Ok(event) => event,
                        Err(err) => {
                            error!(error = %err,
                                   namespace_id = self.pipeline.metadata.namespace_id.clone(),
                                   pipeline_id = self.pipeline.metadata.pipeline_id.clone(),
                                   run_id = self.run.run_id,
                                   task_execution_id = &task_execution.task_id,
                                   "Could not recieve event from event stream during task execution monitoring.");
                            continue;
                        }
                    };

                    // We're specifically looking for cancellation events so that we can stop the container, set the
                    // task as completed and then exit.
                    if let event_utils::Kind::StartedTaskExecutionCancellation {
                        namespace_id,
                        pipeline_id,
                        run_id,
                        task_execution_id,
                        timeout,
                    } = event.kind
                    {
                        // Make sure we only handle events for our specific task execution.
                        if namespace_id != self.pipeline.metadata.namespace_id
                            || pipeline_id != self.pipeline.metadata.pipeline_id
                            || run_id != self.run.run_id
                            || task_execution_id != task_id
                        {
                            continue;
                        }

                        // TODO(): We should probably log the result of this, but for now best effort is fine.
                        _ = self.api_state
                            .scheduler
                            .cancel_task_execution(task_execution.clone(), timeout as i64)
                            .await
                            .map_err(|err| {
                                let all_errors = err
                                    .chain()
                                    .map(|e| e.to_string())
                                    .collect::<Vec<_>>()
                                    .join(" | ");

                                error!(error = %all_errors,
                                       namespace_id = self.pipeline.metadata.namespace_id.clone(),
                                       pipeline_id = self.pipeline.metadata.pipeline_id.clone(),
                                       run_id = self.run.run_id,
                                       task_execution_id = &task_id,
                                       "Could not cancel task execution");
                            });


                        // If we try to cancel the task and it doesn't actually cancel we still need to mark
                        // it as cancelled.

                        let mut conn = self.api_state.storage.write_conn().await?;

                        if let Err(e) = self
                            .set_task_execution_complete(
                                &mut conn,
                                &task_id,
                                1,
                                task_executions::Status::Cancelled,
                                Some(task_executions::StatusReason {
                                    reason: task_executions::StatusReasonType::Cancelled,
                                    description: "The task execution was cancelled".into(),
                                }),
                            )
                            .await
                        {
                            bail!("Could not update task execution while attempting to set execution as complete; {:#?}", e)
                        };

                        return Ok(());
                    }
                }
            }
        }
    }

    async fn handle_log_updates(&self, container_id: String, task_id: String) {
        let log_stream = self
            .api_state
            .scheduler
            .get_logs(scheduler::GetLogsRequest {
                id: container_id.to_string(),
            });

        let path = task_executions::task_execution_log_path(
            &self.api_state.config.api.task_execution_logs_dir,
            &self.pipeline.metadata.namespace_id,
            &self.pipeline.metadata.pipeline_id,
            self.run.run_id,
            &task_id,
        );

        let file = match tokio::fs::File::create(path.clone()).await {
            Ok(file) => Arc::new(Mutex::new(file)),
            Err(e) => {
                error!(namespace_id = &self.pipeline.metadata.namespace_id,
                    pipeline_id = &self.pipeline.metadata.pipeline_id,
                    run_id = self.run.run_id,
                    task_id = &task_id,
                    error = %e,
                    path = path.to_string_lossy().to_string(),
                    "Failed to open file for writing while attempting to write logs for container");
                return;
            }
        };

        log_stream
            .for_each(|item| {
                let file = Arc::clone(&file);
                let path = path.clone();
                let task_id = task_id.clone();

                async move {
                    let log_object = match item {
                        Ok(log_object) => log_object,
                        Err(e) => {
                            error!(
                                namespace_id = &self.pipeline.metadata.namespace_id,
                                pipeline_id = &self.pipeline.metadata.pipeline_id,
                                run_id = self.run.run_id,
                                task_id = &task_id,
                                error = %e, "Failed to parse log stream; scheduler error encountered.");
                            return;
                        },
                    };

                    let mut file = file.lock().await;

                    match log_object {
                        scheduler::Log::Unknown => {
                            error!(namespace_id = &self.pipeline.metadata.namespace_id,
                                pipeline_id = &self.pipeline.metadata.pipeline_id,
                                run_id = self.run.run_id,
                                task_id = &task_id,
                                "Received malformed log from scheduler (Unknown Log type); aborting");
                        },
                        scheduler::Log::Stdout(log) => {
                            if let Err(e) = file.write_all(&log).await {
                                error!(namespace_id = &self.pipeline.metadata.namespace_id,
                                    pipeline_id = &self.pipeline.metadata.pipeline_id,
                                    run_id = self.run.run_id,
                                    task_id = &task_id,
                                    error = %e, path = path.to_string_lossy().to_string(),
                                    "Failed to write stdout log for container");
                            }
                        },
                        scheduler::Log::Stderr(log) => {
                            if let Err(e) = file.write_all(&log).await {
                                error!(namespace_id = &self.pipeline.metadata.namespace_id,
                                    pipeline_id = &self.pipeline.metadata.pipeline_id,
                                    run_id = self.run.run_id,
                                    task_id = &task_id,
                                    error = %e, path = path.to_string_lossy().to_string(),
                                    "Failed to write stderr log for container");
                            }
                        },
                        _ => {
                            // There should be no other types of logs that emit from this call so we ignore everything
                            // else. Alternatively we can just print anything that isn't an "unknown" type.
                        }
                    };
                }
            }).await;

        // When the reader is finished we place a special marker to signify that this file is finished with.
        // This allows other readers of the file within Gofer to know the difference between a file that is still being
        // written to and a file that will not be written to any further.
        let mut file = file.lock().await;

        if let Err(e) = file.write_all(GOFER_EOF.as_bytes()).await {
            error!(namespace_id = &self.pipeline.metadata.namespace_id,
                pipeline_id = &self.pipeline.metadata.pipeline_id,
                run_id = self.run.run_id,
                task_id = &task_id,
                error = %e, path = path.to_string_lossy().to_string(),
            "Failed to write GOFER_EOF to container log");
        }
    }

    /// Removes run level objects(including tokens) from the object store once a run is past it's expiry threshold.
    async fn handle_run_object_expiry(self) {
        let limit = self.api_state.config.object_store.run_object_expiry_runs;

        let mut conn = match self.api_state.storage.read_conn().await {
            Ok(conn) => conn,
            Err(e) => {
                error!(namespace_id = &self.pipeline.metadata.namespace_id,
                    pipeline_id = &self.pipeline.metadata.pipeline_id,
                    run_id = self.run.run_id,
                    error = %e, "Could not establish connection to database while attempting to wait for run finish");
                return;
            }
        };

        let runs = match storage::runs::list(
            &mut conn,
            &self.pipeline.metadata.namespace_id,
            &self.pipeline.metadata.pipeline_id,
            0,
            limit as i64 + 1,
            true,
            0,
        )
        .await
        {
            Ok(runs) => runs,
            Err(e) => {
                error!(namespace_id = &self.pipeline.metadata.namespace_id,
                    pipeline_id = &self.pipeline.metadata.pipeline_id,
                    run_id = self.run.run_id,
                    error = %e, "Could not retrieve runs for run expiry processing");
                return;
            }
        };

        drop(conn);

        // if there aren't enough runs to reach the limit there is nothing to remove
        if limit > runs.len() as u64 {
            return;
        }

        if runs.is_empty() {
            return;
        }

        let run = runs.last().unwrap();
        let mut expired_run: runs::Run = match run.to_owned().try_into() {
            Ok(run) => run,
            Err(e) => {
                error!(namespace_id = &self.pipeline.metadata.namespace_id,
                    pipeline_id = &self.pipeline.metadata.pipeline_id,
                    run_id = self.run.run_id,
                    error = %e, "Could not serialize run while attempting to process run expiry");
                return;
            }
        };

        // If the run is still in progress we wait for it to be done
        loop {
            if expired_run.state == runs::State::Complete {
                break;
            }

            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

            let run_id = match i64::try_from(expired_run.run_id) {
                Ok(i) => i,
                Err(e) => {
                    error!(namespace_id = &self.pipeline.metadata.namespace_id,
                        pipeline_id = &self.pipeline.metadata.pipeline_id,
                        run_id = self.run.run_id,
                        error = %e, "Could not serialize run id while attempting to process run expiry");
                    return;
                }
            };

            let mut conn = match self.api_state.storage.read_conn().await {
                Ok(conn) => conn,
                Err(e) => {
                    error!(namespace_id = &self.pipeline.metadata.namespace_id,
                    pipeline_id = &self.pipeline.metadata.pipeline_id,
                    run_id = self.run.run_id,
                    error = %e, "Could not establish connection to database while attempting to wait for run finish");
                    return;
                }
            };

            let updated_run = match storage::runs::get(
                &mut conn,
                &self.pipeline.metadata.namespace_id,
                &self.pipeline.metadata.pipeline_id,
                run_id,
            )
            .await
            {
                Ok(updated_run) => updated_run,
                Err(e) => {
                    error!(namespace_id = &self.pipeline.metadata.namespace_id,
                        pipeline_id = &self.pipeline.metadata.pipeline_id,
                        run_id = self.run.run_id,
                        error = %e, "Could not get updated run state while attempting to process run expiry");
                    return;
                }
            };

            let updated_expired_run: runs::Run = match updated_run.try_into() {
                Ok(run) => run,
                Err(e) => {
                    error!(namespace_id = &self.pipeline.metadata.namespace_id,
                        pipeline_id = &self.pipeline.metadata.pipeline_id,
                        run_id = self.run.run_id,
                        error = %e, "Could not serialize updated run while attempting to process run expiry");
                    return;
                }
            };

            expired_run = updated_expired_run
        }

        // Remove the mut from expired run since we don't need it anymore.
        let expired_run = expired_run;

        let mut conn = match self.api_state.storage.write_conn().await {
            Ok(conn) => conn,
            Err(e) => {
                error!(namespace_id = &self.pipeline.metadata.namespace_id,
                    pipeline_id = &self.pipeline.metadata.pipeline_id,
                    run_id = self.run.run_id,
                    error = %e, "Could not establish connection to database while attempting to wait for run finish");
                return;
            }
        };

        // We also need to delete automatically generated (inject_api_token) pipeline secret keys.

        if let Some(token_id) = expired_run.token_id {
            // The first step here is to delete the pipeline secret reference.
            if let Err(e) = storage::secret_store_pipeline_keys::delete(
                &mut conn,
                &expired_run.namespace_id,
                &expired_run.pipeline_id,
                &run_specific_api_key_id(expired_run.run_id),
            )
            .await
            {
                match e {
                    storage::StorageError::NotFound => {
                        // If the key doesn't even exist, print an error but don't stop the process.
                        error!(
                            namespace_id = &expired_run.namespace_id,
                            pipeline_id = &expired_run.pipeline_id,
                            run_id = expired_run.run_id,
                            key_id = run_specific_api_key_id(expired_run.run_id),
                            "Could not find pipeline secret key while attempting to remove inject_api_token"
                        );
                    }
                    _ => {
                        error!(namespace_id = &expired_run.namespace_id,
                        pipeline_id = &expired_run.pipeline_id,
                        run_id = expired_run.run_id,
                        key_id = run_specific_api_key_id(expired_run.run_id),
                        error = %e, "Could not remove inject_api_token pipeline secret key");
                        return;
                    }
                }
            };

            // The second step is to delete the key itself.
            if let Err(e) = storage::tokens::delete(&mut conn, &token_id).await {
                match e {
                    storage::StorageError::NotFound => {
                        // If the key doesn't even exist, print an error but don't stop the process.
                        error!(
                            namespace_id = &expired_run.namespace_id,
                            pipeline_id = &expired_run.pipeline_id,
                            run_id = expired_run.run_id,
                            token_id = token_id,
                            "Could not find token while attempting to remove inject_api_token"
                        );
                    }
                    _ => {
                        error!(namespace_id = &expired_run.namespace_id,
                        pipeline_id = &expired_run.pipeline_id,
                        run_id = expired_run.run_id,
                        token_id = token_id,
                        error = %e, "Could not remove inject_api_token");
                        return;
                    }
                }
            };

            // The last step is to delete the token object.
            if let Err(e) = self
                .api_state
                .secret_store
                .delete(&secrets::pipeline_secret_store_key(
                    &expired_run.namespace_id,
                    &expired_run.pipeline_id,
                    &run_specific_api_key_id(expired_run.run_id),
                ))
                .await
            {
                error!(namespace_id = &expired_run.namespace_id,
                        pipeline_id = &expired_run.pipeline_id,
                        run_id = expired_run.run_id,
                        token_id = token_id,
                        error = %e, "Could not remove insert_api_token object from secret_store");
                return;
            };
        }

        if expired_run.store_objects_expired {
            return;
        }

        let expired_run_id = match i64::try_from(expired_run.run_id) {
            Ok(i) => i,
            Err(e) => {
                error!(namespace_id = &self.pipeline.metadata.namespace_id,
                    pipeline_id = &self.pipeline.metadata.pipeline_id,
                    run_id = self.run.run_id,
                    error = %e, "Could not serialize run id while attempting to process run expiry");
                return;
            }
        };

        let objects = match storage::object_store_run_keys::list(
            &mut conn,
            &self.pipeline.metadata.namespace_id,
            &self.pipeline.metadata.pipeline_id,
            expired_run_id,
        )
        .await
        {
            Ok(objects) => objects,
            Err(e) => {
                error!(namespace_id = &self.pipeline.metadata.namespace_id,
                    pipeline_id = &self.pipeline.metadata.pipeline_id,
                    run_id = self.run.run_id,
                    error = %e, "Could not get updated run state while attempting to process run expiry");
                return;
            }
        };

        for object in objects {
            // Delete it from the object store
            if let Err(e) = self
                .api_state
                .object_store
                .delete(&objects::run_object_store_key(
                    &self.pipeline.metadata.namespace_id,
                    &self.pipeline.metadata.pipeline_id,
                    expired_run.run_id,
                    &object.key,
                ))
                .await
            {
                error!(namespace_id = &self.pipeline.metadata.namespace_id,
                    pipeline_id = &self.pipeline.metadata.pipeline_id,
                    run_id = self.run.run_id,
                    error = %e, "Could not delete object from store while attempting to process run expiry");
                return;
            };

            // Delete it from the run's records
            if let Err(e) = storage::object_store_run_keys::delete(
                &mut conn,
                &self.pipeline.metadata.namespace_id,
                &self.pipeline.metadata.pipeline_id,
                expired_run_id,
                &object.key,
            )
            .await
            {
                error!(namespace_id = &self.pipeline.metadata.namespace_id,
                    pipeline_id = &self.pipeline.metadata.pipeline_id,
                    run_id = self.run.run_id,
                    error = %e, "Could not remove object store reference in run while attempting to process run expiry");
                return;
            };
        }

        if let Err(e) = storage::runs::update(
            &mut conn,
            &self.pipeline.metadata.namespace_id,
            &self.pipeline.metadata.pipeline_id,
            expired_run_id,
            storage::runs::UpdatableFields {
                store_objects_expired: Some(true),
                ..Default::default()
            },
        )
        .await
        {
            error!(namespace_id = &self.pipeline.metadata.namespace_id,
                pipeline_id = &self.pipeline.metadata.pipeline_id,
                run_id = self.run.run_id,
                error = %e, "Could not update run while attempting to process run expiry");
        }
    }

    async fn handle_run_log_expiry(self) {
        let limit = self.api_state.config.api.task_execution_log_retention_runs;

        let mut conn = match self.api_state.storage.read_conn().await {
            Ok(conn) => conn,
            Err(e) => {
                error!(namespace_id = &self.pipeline.metadata.namespace_id,
                    pipeline_id = &self.pipeline.metadata.pipeline_id,
                    run_id = self.run.run_id,
                    error = %e, "Could not establish connection to database while attempting to wait for run finish");
                return;
            }
        };

        let runs = match storage::runs::list(
            &mut conn,
            &self.pipeline.metadata.namespace_id,
            &self.pipeline.metadata.pipeline_id,
            0,
            limit as i64 + 1,
            true,
            0,
        )
        .await
        {
            Ok(runs) => runs,
            Err(e) => {
                error!(namespace_id = &self.pipeline.metadata.namespace_id,
                    pipeline_id = &self.pipeline.metadata.pipeline_id,
                    run_id = self.run.run_id,
                    error = %e, "Could not retrieve runs for run expiry processing");
                return;
            }
        };

        // Don't hold connection over loop point.
        drop(conn);

        // if there aren't enough runs to reach the limit there is nothing to remove
        if limit > runs.len() as u64 {
            return;
        }

        if runs.is_empty() {
            return;
        }

        let run = runs.last().unwrap();
        let mut expired_run: runs::Run = match run.to_owned().try_into() {
            Ok(run) => run,
            Err(e) => {
                error!(namespace_id = &self.pipeline.metadata.namespace_id,
                    pipeline_id = &self.pipeline.metadata.pipeline_id,
                    run_id = self.run.run_id,
                    error = %e, "Could not serialize run while attempting to process run log expiry");
                return;
            }
        };

        // If the run is still in progress we wait for it to be done
        loop {
            if expired_run.state == runs::State::Complete {
                break;
            }

            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

            let run_id = match i64::try_from(expired_run.run_id) {
                Ok(i) => i,
                Err(e) => {
                    error!(namespace_id = &self.pipeline.metadata.namespace_id,
                        pipeline_id = &self.pipeline.metadata.pipeline_id,
                        run_id = self.run.run_id,
                        error = %e, "Could not serialize run id while attempting to process run log expiry");
                    return;
                }
            };

            let mut conn = match self.api_state.storage.read_conn().await {
                Ok(conn) => conn,
                Err(e) => {
                    error!(namespace_id = &self.pipeline.metadata.namespace_id,
                    pipeline_id = &self.pipeline.metadata.pipeline_id,
                    run_id = self.run.run_id,
                    error = %e, "Could not establish connection to database while attempting to wait for run finish");
                    return;
                }
            };

            let updated_run = match storage::runs::get(
                &mut conn,
                &self.pipeline.metadata.namespace_id,
                &self.pipeline.metadata.pipeline_id,
                run_id,
            )
            .await
            {
                Ok(updated_run) => updated_run,
                Err(e) => {
                    error!(namespace_id = &self.pipeline.metadata.namespace_id,
                        pipeline_id = &self.pipeline.metadata.pipeline_id,
                        run_id = self.run.run_id,
                        error = %e, "Could not get updated run state while attempting to process run log expiry");
                    return;
                }
            };

            let updated_expired_run: runs::Run = match updated_run.try_into() {
                Ok(run) => run,
                Err(e) => {
                    error!(namespace_id = &self.pipeline.metadata.namespace_id,
                        pipeline_id = &self.pipeline.metadata.pipeline_id,
                        run_id = self.run.run_id,
                        error = %e, "Could not serialize updated run while attempting to process run log expiry");
                    return;
                }
            };

            expired_run = updated_expired_run
        }

        // If the task executions are in progress we wait for them to be finished also.

        let expired_run_id = match i64::try_from(expired_run.run_id) {
            Ok(i) => i,
            Err(e) => {
                error!(namespace_id = &self.pipeline.metadata.namespace_id,
                    pipeline_id = &self.pipeline.metadata.pipeline_id,
                    run_id = self.run.run_id,
                    error = %e, "Could not serialize run id while attempting to process run log expiry");
                return;
            }
        };

        let mut chopping_block_ids = HashMap::new();

        loop {
            let mut conn = match self.api_state.storage.read_conn().await {
                Ok(conn) => conn,
                Err(e) => {
                    error!(namespace_id = &self.pipeline.metadata.namespace_id,
                    pipeline_id = &self.pipeline.metadata.pipeline_id,
                    run_id = self.run.run_id,
                    error = %e, "Could not establish connection to database while attempting to wait for run finish");
                    return;
                }
            };

            let task_executions_raw = match storage::task_executions::list(
                &mut conn,
                &self.pipeline.metadata.namespace_id,
                &self.pipeline.metadata.pipeline_id,
                expired_run_id,
            )
            .await
            {
                Ok(executions) => executions,
                Err(e) => {
                    error!(namespace_id = &self.pipeline.metadata.namespace_id,
                        pipeline_id = &self.pipeline.metadata.pipeline_id,
                        run_id = self.run.run_id,
                        error = %e, "Could not get task executions while attempting to process run log expiry");
                    return;
                }
            };

            // Don't hold database connection across loop.
            drop(conn);

            for execution in task_executions_raw.iter() {
                let execution_state = match task_executions::State::from_str(&execution.state) {
                    Ok(state) => state,
                    Err(e) => {
                        error!(namespace_id = &self.pipeline.metadata.namespace_id,
                            pipeline_id = &self.pipeline.metadata.pipeline_id,
                            run_id = self.run.run_id,
                            error = %e, storage_state = execution.state,
                            "Could not parse state while attempting to process run log expiry");
                        continue;
                    }
                };

                // If the task execution is complete we put it on the chopping block.
                if execution_state == task_executions::State::Complete {
                    chopping_block_ids.insert(execution.task_id.clone(), execution.logs_removed);
                    continue;
                }
            }

            if chopping_block_ids.len() != task_executions_raw.len() {
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                continue;
            } else {
                break;
            }
        }

        let mut removed_files = vec![];

        for (id, logs_removed) in chopping_block_ids {
            if logs_removed {
                continue;
            }

            let log_path = task_executions::task_execution_log_path(
                &self.api_state.config.api.task_execution_logs_dir,
                &self.pipeline.metadata.namespace_id,
                &self.pipeline.metadata.pipeline_id,
                expired_run.run_id,
                &id,
            );

            if let Err(e) = tokio::fs::remove_file(log_path.clone()).await {
                debug!(namespace_id = &self.pipeline.metadata.namespace_id,
                    pipeline_id = &self.pipeline.metadata.pipeline_id,
                    run_id = self.run.run_id,
                    error = %e, path = ?log_path, "Could not remove task execution log file");
            }

            removed_files.push(log_path.to_string_lossy().to_string());

            let mut conn = match self.api_state.storage.write_conn().await {
                Ok(conn) => conn,
                Err(e) => {
                    error!(namespace_id = &self.pipeline.metadata.namespace_id,
                    pipeline_id = &self.pipeline.metadata.pipeline_id,
                    run_id = self.run.run_id,
                    error = %e, "Could not establish connection to database while attempting to wait for run finish");
                    return;
                }
            };

            if let Err(e) = storage::task_executions::update(
                &mut conn,
                &self.pipeline.metadata.namespace_id,
                &self.pipeline.metadata.pipeline_id,
                expired_run_id,
                &id,
                storage::task_executions::UpdatableFields {
                    logs_expired: Some(true),
                    logs_removed: Some(true),
                    ..Default::default()
                },
            )
            .await
            {
                error!(namespace_id = &self.pipeline.metadata.namespace_id,
                    pipeline_id = &self.pipeline.metadata.pipeline_id,
                    run_id = self.run.run_id,
                    error = %e, task_id = id, "Could not update task execution while attempting to process run log expiry");
                continue;
            };
        }

        debug!(namespace_id = &self.pipeline.metadata.namespace_id,
            pipeline_id = &self.pipeline.metadata.pipeline_id,
            run_id = self.run.run_id,
            removed_files = ?removed_files, "Removed task execution log files");
    }
}

/// We need to combine the environment variables we get from multiple sources in order to pass them
/// finally to the task execution. The order in which they are passed is very important as they can and should
/// overwrite each other, even though the intention of prefixing the environment variables is to prevent
/// the chance of overwriting. The order in which they are passed into the extend function
/// determines the priority in reverse order. Last in the stack will overwrite any conflicts from the others.
///
/// There are many places a task_execution could potentially get env vars from:
/// 1) Right before the task_execution starts, from Gofer itself.
/// 2) At the time of run inception, either by the user manually or the trigger.
/// 3) From the pipeline's configuration file.
///
/// The order in which the env vars are stacked are as such:
/// 1) We first pass in the Gofer system specific envvars as these are the most replaceable on the totem pole.
/// 2) We pass in the task specific envvars defined by the user in the pipeline config.
/// 3) Lastly we pass in the run specific defined envvars. These are usually provided by either a trigger
///    or the user when they attempt to start a new run manually. Since these are the most likely to be
///    edited adhoc they are treated as the most important.
/// The timeout a task execution runs under, and what to tell the user if it hits it.
#[derive(Debug, PartialEq, Eq)]
struct TaskTimeout {
    seconds: u64,
    description: String,
}

/// Picks between the task's own timeout and the server's max, where 0 means unset for both. The description names
/// the one that applies, since "the task's timeout" and "the server's cap" point at different people to fix it.
fn task_timeout(task_seconds: u64, max_seconds: u64) -> Option<TaskTimeout> {
    let server_max = format!(
        "the server's max task execution timeout of {} (api.task_execution_max_timeout_seconds)",
        humanize_seconds(max_seconds)
    );

    let own = |task: u64| format!("its own timeout of {}", humanize_seconds(task));

    let (seconds, limit, note) = match (task_seconds, max_seconds) {
        (0, 0) => return None,
        (0, max) => (
            max,
            server_max,
            "; the task doesn't set its own timeout".to_string(),
        ),
        (task, max) if max == 0 || task <= max => (task, own(task), String::new()),
        (task, max) => (
            max,
            server_max,
            format!(
                "; the task asked for {} but the server caps it",
                humanize_seconds(task)
            ),
        ),
    };

    Some(TaskTimeout {
        seconds,
        description: format!("Task ran longer than {limit} and was stopped{note}"),
    })
}

/// Turns 5400 into "1h30m", which reads better than seconds in a status reason.
fn humanize_seconds(seconds: u64) -> String {
    let (hours, minutes, seconds) = (seconds / 3600, seconds % 3600 / 60, seconds % 60);
    let mut out = String::new();

    if hours > 0 {
        out.push_str(&format!("{hours}h"));
    }
    if minutes > 0 {
        out.push_str(&format!("{minutes}m"));
    }
    if seconds > 0 || out.is_empty() {
        out.push_str(&format!("{seconds}s"));
    }

    out
}

pub fn combine_variables(run: &runs::Run, task: &tasks::Task, api_base_url: &str) -> Vec<Variable> {
    let system_injected_vars = system_injected_vars(run, task, api_base_url);

    let task_vars: HashMap<String, Variable> = task
        .variables
        .iter()
        .map(|variable| (variable.key.to_uppercase(), variable.clone()))
        .collect();

    let run_vars: HashMap<String, Variable> = run
        .variables
        .iter()
        .map(|variable| (variable.key.to_uppercase(), variable.clone()))
        .collect();

    let mut task_execution_vars = system_injected_vars; // Gofer provided env vars first.
    task_execution_vars.extend(task_vars); // then we vars that come from the pipeline config.
    task_execution_vars.extend(run_vars); // then finally vars that come from the user or the trigger.

    // It is possible for the user to enter an empty key, but that would be an error when
    // attempting to pass it to the docker container.
    task_execution_vars = task_execution_vars
        .into_iter()
        .filter_map(|(key, value)| {
            if key.is_empty() {
                None
            } else {
                Some((key, value))
            }
        })
        .collect();

    task_execution_vars.into_values().collect()
}

/// The URL tasks and extensions use to reach Gofer. They run on the same container network, so they use the
/// extension address rather than the one users reach Gofer at. It is usually stored without a scheme, so we add one
/// based on whether Gofer serves TLS.
pub fn task_api_base_url(server: &conf::api::Server) -> String {
    if server.extension_address.contains("://") {
        return server.extension_address.clone();
    }

    let scheme = if server.use_tls { "https" } else { "http" };
    format!("{scheme}://{}", server.extension_address)
}

/// On every run Gofer injects some vars that are determined by the system.
/// These are usually meant to give the user some basic information that they can pull
/// into their program about the details of the run.
fn system_injected_vars(
    run: &runs::Run,
    task: &tasks::Task,
    api_base_url: &str,
) -> HashMap<String, Variable> {
    // GOFER_NAMESPACE and GOFER_API_BASE_URL use the same names the Gofer CLI reads its config from, so a task
    // that has the CLI and an injected token can talk to Gofer without any extra setup.
    let mut vars = HashMap::from([
        (
            "GOFER_NAMESPACE".to_string(),
            Variable {
                key: "GOFER_NAMESPACE".to_string(),
                value: run.namespace_id.clone(),
                source: VariableSource::System,
            },
        ),
        (
            "GOFER_API_BASE_URL".to_string(),
            Variable {
                key: "GOFER_API_BASE_URL".to_string(),
                value: api_base_url.to_string(),
                source: VariableSource::System,
            },
        ),
        (
            "GOFER_PIPELINE_ID".to_string(),
            Variable {
                key: "GOFER_PIPELINE_ID".to_string(),
                value: run.pipeline_id.clone(),
                source: VariableSource::System,
            },
        ),
        (
            "GOFER_RUN_ID".to_string(),
            Variable {
                key: "GOFER_RUN_ID".to_string(),
                value: run.run_id.to_string(),
                source: VariableSource::System,
            },
        ),
        (
            "GOFER_TASK_ID".to_string(),
            Variable {
                key: "GOFER_TASK_ID".to_string(),
                value: task.id.clone(),
                source: VariableSource::System,
            },
        ),
        (
            "GOFER_TASK_IMAGE".to_string(),
            Variable {
                key: "GOFER_TASK_IMAGE".to_string(),
                value: task.image.clone(),
                source: VariableSource::System,
            },
        ),
    ]);

    if task.inject_api_token {
        vars.insert(
            // The exact key 'GOFER_TOKEN' is what is expected by the Gofer CLI. This enables the user to not have
            // to edit anything to be able to use an embedded gofer cli.
            "GOFER_TOKEN".into(),
            Variable {
                key: "GOFER_TOKEN".into(),
                value: pipeline_secret(&run_specific_api_key_id(run.run_id)),
                source: VariableSource::System,
            },
        );
    }

    vars
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_api_base_url_adds_scheme() {
        let mut server = conf::api::Server {
            extension_address: "172.17.0.1:8080".into(),
            ..Default::default()
        };
        assert_eq!(task_api_base_url(&server), "http://172.17.0.1:8080");

        server.use_tls = true;
        assert_eq!(task_api_base_url(&server), "https://172.17.0.1:8080");

        server.extension_address = "http://gofer.internal:8080".into();
        assert_eq!(task_api_base_url(&server), "http://gofer.internal:8080");
    }

    #[test]
    fn task_timeout_picks_and_explains_the_limit() {
        assert_eq!(task_timeout(0, 0), None);

        let own = task_timeout(600, 0).unwrap();
        assert_eq!(own.seconds, 600);
        assert_eq!(
            own.description,
            "Task ran longer than its own timeout of 10m and was stopped"
        );

        assert_eq!(task_timeout(600, 3600).unwrap().seconds, 600);

        let default = task_timeout(0, 86400).unwrap();
        assert_eq!(default.seconds, 86400);
        assert_eq!(
            default.description,
            "Task ran longer than the server's max task execution timeout of 24h \
             (api.task_execution_max_timeout_seconds) and was stopped; the task doesn't set its own timeout"
        );

        let capped = task_timeout(7200, 3600).unwrap();
        assert_eq!(capped.seconds, 3600);
        assert!(
            capped
                .description
                .contains("the task asked for 2h but the server caps it")
        );
    }

    #[test]
    fn humanize_seconds_formats() {
        assert_eq!(humanize_seconds(0), "0s");
        assert_eq!(humanize_seconds(45), "45s");
        assert_eq!(humanize_seconds(5400), "1h30m");
        assert_eq!(humanize_seconds(3661), "1h1m1s");
    }

    fn queued(pipeline_id: &str, run_id: i64, parallelism: i64) -> storage::runs::QueuedRun {
        storage::runs::QueuedRun {
            namespace_id: "default".into(),
            pipeline_id: pipeline_id.into(),
            run_id,
            parallelism,
            pipeline_state: "active".into(),
        }
    }

    fn selected_ids(selected: &[storage::runs::QueuedRun]) -> Vec<(String, i64)> {
        selected
            .iter()
            .map(|run| (run.pipeline_id.clone(), run.run_id))
            .collect()
    }

    #[test]
    fn pipeline_run_limit_combines_pipeline_and_server_limits() {
        assert_eq!(pipeline_run_limit(0, 200), 200);
        assert_eq!(pipeline_run_limit(3, 0), 3);
        assert_eq!(pipeline_run_limit(3, 200), 3);
        assert_eq!(pipeline_run_limit(500, 200), 200);
        assert_eq!(pipeline_run_limit(0, 0), 0);
    }

    #[test]
    fn select_runs_takes_queue_in_order() {
        let queue = [queued("a", 1, 0), queued("b", 1, 0), queued("a", 2, 0)];

        let selected = select_runs_to_start(&queue, &HashMap::new(), 0, 2);

        assert_eq!(
            selected_ids(&selected),
            vec![("a".to_string(), 1), ("b".to_string(), 1)]
        );
    }

    #[test]
    fn select_runs_skips_pipeline_at_its_limit_without_blocking_others() {
        let queue = [queued("a", 2, 1), queued("a", 3, 1), queued("b", 1, 1)];
        let running = HashMap::from([(("default".to_string(), "a".to_string()), 1)]);

        let selected = select_runs_to_start(&queue, &running, 0, 0);

        assert_eq!(selected_ids(&selected), vec![("b".to_string(), 1)]);
    }

    #[test]
    fn select_runs_fills_free_slots_of_one_pipeline() {
        let queue = [queued("a", 1, 2), queued("a", 2, 2), queued("a", 3, 2)];

        let selected = select_runs_to_start(&queue, &HashMap::new(), 0, 0);

        assert_eq!(
            selected_ids(&selected),
            vec![("a".to_string(), 1), ("a".to_string(), 2)]
        );
    }

    #[test]
    fn select_runs_counts_running_runs_against_the_global_limit() {
        let queue = [queued("a", 1, 0), queued("b", 1, 0)];
        let running = HashMap::from([(("default".to_string(), "c".to_string()), 2)]);

        let selected = select_runs_to_start(&queue, &running, 0, 3);

        assert_eq!(selected_ids(&selected), vec![("a".to_string(), 1)]);
    }

    #[test]
    fn select_runs_uses_server_limit_when_pipeline_has_none() {
        let queue = [queued("a", 1, 0), queued("a", 2, 0)];

        let selected = select_runs_to_start(&queue, &HashMap::new(), 1, 0);

        assert_eq!(selected_ids(&selected), vec![("a".to_string(), 1)]);
    }

    #[test]
    fn select_runs_caps_pipeline_limit_at_server_limit() {
        let queue = [queued("a", 1, 5), queued("a", 2, 5)];

        let selected = select_runs_to_start(&queue, &HashMap::new(), 1, 0);

        assert_eq!(selected_ids(&selected), vec![("a".to_string(), 1)]);
    }
}
