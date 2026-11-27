use super::OrchestratorError;
use crate::{
    api::{
        ApiState, epoch_milli,
        event_utils::{self, EventListener},
        pipeline_configs, pipelines, runs, task_executions, tasks,
    },
    containers::RunFolder,
    storage,
};

use anyhow::{Context, Result};
use sqlx::SqliteConnection;
use std::sync::Arc;
use tokio::sync::Barrier;
use tokio_util::task::TaskTracker;
use tracing::{error, trace};

/// A run specific object that guides Gofer runs and tasks through their execution.
/// It's a core construct within the Gofer execution model and contains most of the logic of how a run operates.
#[derive(Debug, Clone)]
pub(super) struct RunShepherd {
    pub(super) api_state: Arc<ApiState>,
    pub(super) pipeline: pipelines::Pipeline,
    pub(super) run: runs::Run,

    /// The tasks writing each task execution's logs to disk. Every clone of the shepherd shares it.
    pub(super) log_streams: TaskTracker,
}

impl RunShepherd {
    pub fn new(api_state: Arc<ApiState>, pipeline: pipelines::Pipeline, run: runs::Run) -> Self {
        Self {
            api_state,
            pipeline,
            run,
            log_streams: TaskTracker::new(),
        }
    }

    /// The folder this run's tasks share.
    pub(super) fn run_folder(&self) -> RunFolder {
        RunFolder {
            namespace_id: self.pipeline.metadata.namespace_id.clone(),
            pipeline_id: self.pipeline.metadata.pipeline_id.clone(),
            run_id: self.run.run_id,
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

        self.announce_start();

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

        let tasks = self.pipeline.config.tasks.values().cloned().collect();

        self.launch_tasks(tasks, None).await
    }

    /// Picks a run that was going when Gofer stopped back up. It goes through the same steps as [`Self::start_run`],
    /// except that only the task executions that hadn't finished are launched again, and everything reads the event
    /// bus from where the run began instead of from now, so events that happened while Gofer was down aren't missed.
    ///
    /// run_event_id is the id of the event published when the run was queued.
    pub async fn start_run_recovery(self, run_event_id: String) -> Result<()> {
        trace!(
            namespace_id = self.pipeline.metadata.namespace_id.clone(),
            pipeline_id = self.pipeline.metadata.pipeline_id.clone(),
            run_id = self.run.run_id,
            run_event_id = run_event_id,
            "Recovering run"
        );

        self.announce_start();

        let tasks = self.unfinished_tasks().await?;

        self.launch_tasks(tasks, Some(run_event_id)).await
    }

    /// Starts the run's cleanup jobs and lets everyone listening know the run has started.
    fn announce_start(&self) {
        // This job helps keep resources from filling up.
        tokio::spawn(self.clone().handle_run_log_expiry());

        self.api_state
            .event_bus
            .clone()
            .publish(event_utils::Kind::StartedRun {
                namespace_id: self.pipeline.metadata.namespace_id.clone(),
                pipeline_id: self.pipeline.metadata.pipeline_id.clone(),
                run_id: self.run.run_id,
            });
    }

    /// The tasks whose task executions hadn't completed when Gofer stopped.
    async fn unfinished_tasks(&self) -> Result<Vec<tasks::Task>> {
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
        .context("Could not get task execution list while attempting to recover run")?;

        let mut unfinished_tasks = vec![];

        for task_execution in task_executions {
            let task_execution: task_executions::TaskExecution =
                task_execution.try_into().context(
                    "Could not serialize task execution while attempting to query for unfinished tasks",
                )?;

            if task_execution.state != task_executions::State::Complete {
                unfinished_tasks.push(task_execution.task);
            }
        }

        Ok(unfinished_tasks)
    }

    /// Launches a task execution for each task, plus the run monitor that waits for them all to finish. Everything
    /// reads the event bus live, or from `replay_from` onward when a run is being recovered.
    async fn launch_tasks(
        self,
        tasks: Vec<tasks::Task>,
        replay_from: Option<String>,
    ) -> Result<()> {
        // We use a barrier here to make sure that all threads are able to grab event bus listeners at roughly the
        // same point so that they don't end up missing any messages from threads that might be faster.
        //
        // The barrier knows when to tell all tasks to continue by waiting until all tasks check in.
        // We calculate this value by taking the number of all the tasks we are about to start and then adding
        // one more for the run monitor itself.
        let barrier = Arc::new(Barrier::new(tasks.len() + 1));

        for task in tasks {
            let event_stream = self.subscribe(replay_from.as_deref()).await?;

            tokio::spawn(
                self.clone()
                    .launch_task_execution(barrier.clone(), event_stream, task),
            );
        }

        let event_stream = self.subscribe(replay_from.as_deref()).await?;

        tokio::spawn(async move { self.monitor_run(barrier, event_stream).await });

        Ok(())
    }

    async fn subscribe(&self, replay_from: Option<&str>) -> Result<Box<dyn EventListener>> {
        let Some(event_id) = replay_from else {
            return Ok(Box::new(self.api_state.event_bus.subscribe_live()));
        };

        let event_stream = self
            .api_state
            .event_bus
            .subscribe_historical(Some(event_id.to_string()))
            .await
            .context(
                "Could not establish event stream while attempting to recover unfinished run",
            )?;

        Ok(Box::new(event_stream))
    }

    /// Listens to messages from the event bus and updates the status of the run in-progress.
    async fn monitor_run(&self, barrier: Arc<Barrier>, mut event_stream: Box<dyn EventListener>) {
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

        tokio::spawn(self.clone().clean_up());

        Ok(())
    }
}
