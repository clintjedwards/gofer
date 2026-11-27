use super::{
    RunShepherd,
    variables::{combine_variables, task_api_base_url},
};
use crate::{
    api::{
        GOFER_EOF, Variable, epoch_milli,
        event_utils::{self, EventListener},
        interpolate_vars, task_executions, tasks,
    },
    containers, images, storage,
};

use anyhow::{Context, Result, bail};
use futures::StreamExt;
use sqlx::SqliteConnection;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::{
    io::AsyncWriteExt,
    sync::{Barrier, Mutex, mpsc},
};
use tracing::{error, trace};

impl RunShepherd {
    /// Launches a brand new task execution as part of a larger run for a specific task.
    /// It waits for the task's parents, starts its container, and then hands it off to be monitored until it
    /// completes, reading and posting to the event bus to facilitate further run actions.
    pub(super) async fn launch_task_execution(
        self,
        barrier: Arc<Barrier>,
        event_stream: Box<dyn EventListener>,
        task: tasks::Task,
    ) {
        // wait for all the other threads to get to this point so everyone starts out at the same point in the event bus.
        barrier.wait().await;

        if let Err(e) = self.run_task_execution(event_stream, &task).await {
            error!(namespace_id = &self.pipeline.metadata.namespace_id,
                pipeline_id = &self.pipeline.metadata.pipeline_id,
                run_id = self.run.run_id,
                task_id = task.id,
                error = %format!("{e:#}"), "Could not launch task execution");
        }
    }

    async fn run_task_execution(
        &self,
        mut event_stream: Box<dyn EventListener>,
        task: &tasks::Task,
    ) -> Result<()> {
        let (mut task_execution, env_vars) = self.create_task_execution(task).await?;

        // On recovery the task execution starts over, so its timeline does too.
        let timeline = self.spawn_timeline_recorder(&task.id);

        // <task_execution_id> => <status>
        let completed_tasks = if task.depends_on.is_empty() {
            HashMap::new()
        } else {
            _ = timeline.send(task_executions::TimelineEntry::now(
                task_executions::Phase::WaitingOnDependencies,
            ));

            match self.wait_for_parents(&mut event_stream, task).await {
                Some(completed_tasks) => completed_tasks,
                // It was cancelled while it waited.
                None => return Ok(()),
            }
        };

        let mut conn = self
            .api_state
            .storage
            .write_conn()
            .await
            .context("Could not establish connection to database")?;

        self.set_task_execution_state(
            &mut conn,
            &task_execution,
            task_executions::State::Processing,
        )
        .await
        .context("Could not update task_execution state to processing")?;

        drop(conn);

        // Then check to make sure that the parents all finished in the required states. If not
        // we'll mark this task as skipped since it's requirements for running weren't met.
        if let Err(e) = self.task_dependencies_satisfied(completed_tasks, &task.depends_on) {
            self.end_task_execution(
                &task.id,
                task_executions::Status::Skipped,
                Some(task_executions::StatusReason {
                    reason: task_executions::StatusReasonType::FailedPrecondition,
                    description: format!("Task could not be run due to unmet dependencies; {}", e),
                }),
            )
            .await
            .context("Could not mark task execution as skipped during the processing of task dependencies")?;

            return Ok(());
        }

        // After this point we're sure the task is in a state to be run. So we attempt to
        // start its container.

        _ = timeline.send(task_executions::TimelineEntry::now(
            task_executions::Phase::Preparing,
        ));

        let env_vars = match self.resolve_variables(env_vars).await {
            Ok(resolved) => resolved,
            Err(e) => {
                self.end_task_execution(
                    &task.id,
                    task_executions::Status::Failed,
                    Some(task_executions::StatusReason {
                        reason: task_executions::StatusReasonType::FailedPrecondition,
                        description: format!(
                            "Task could not be run due to inability to retrieve interpolated variables; {}",
                            e
                        ),
                    }),
                )
                .await
                .context("Could not mark task execution as failed during the processing of task env vars")?;

                return Err(e)
                    .context("Could not properly interpolate variables for task execution");
            }
        };

        let container_id = task_executions::task_execution_container_id(
            &self.pipeline.metadata.namespace_id,
            &self.pipeline.metadata.pipeline_id,
            self.run.run_id,
            &task.id,
        );

        // Captured before scheduling so the task's duration includes loading its image and starting the container;
        // short tasks can finish before start_container even returns.
        let started = epoch_milli();

        let image = match self.load_image(task, &timeline).await {
            Ok(image) => image,
            Err(reason) => {
                self.end_task_execution(&task.id, task_executions::Status::Failed, Some(reason))
                    .await
                    .context("Could not mark task execution as failed after its image could not be loaded")?;

                return Ok(());
            }
        };

        if let Err(e) = self
            .start_container(&image, &container_id, env_vars, &timeline)
            .await
        {
            // The error is in the task execution's status reason, so there's nothing more to log.
            self.end_task_execution(
                &task.id,
                task_executions::Status::Failed,
                Some(task_executions::StatusReason {
                    reason: task_executions::StatusReasonType::ContainerError,
                    description: format!(
                        "Task could not be run because its container could not be started; {}",
                        e
                    ),
                }),
            )
            .await
            .context(
                "Could not mark task execution as failed after its container could not be started",
            )?;

            return Ok(());
        }

        trace!(
            namespace_id = self.pipeline.metadata.namespace_id.clone(),
            pipeline_id = self.pipeline.metadata.pipeline_id.clone(),
            run_id = self.run.run_id,
            task_execution_id = &task.id,
            "Started task execution"
        );

        let mut conn = self
            .api_state
            .storage
            .write_conn()
            .await
            .context("Could not establish connection to database")?;

        storage::task_executions::update(
            &mut conn,
            &self.pipeline.metadata.namespace_id,
            &self.pipeline.metadata.pipeline_id,
            self.run.run_id.try_into()?,
            &task.id,
            storage::task_executions::UpdatableFields {
                state: Some(task_executions::State::Running.to_string()),
                started: Some(started.to_string()),
                ..Default::default()
            },
        )
        .await
        .context("Could not update task execution while attempting to launch task")?;

        // Drop the database connection so we don't hold it while long running processes below are happening.
        drop(conn);

        task_execution.state = task_executions::State::Running;

        self.api_state
            .event_bus
            .clone()
            .publish(event_utils::Kind::StartedTaskExecution {
                namespace_id: self.pipeline.metadata.namespace_id.clone(),
                pipeline_id: self.pipeline.metadata.pipeline_id.clone(),
                run_id: self.run.run_id,
                task_execution_id: task.id.clone(),
            });

        self.spawn_task_execution_monitors(event_stream, container_id, task_execution);

        Ok(())
    }

    /// Saves a new task execution, along with the variables it'll run with, and marks it as waiting. Returns those
    /// variables, which still need their secrets and objects filled in before the container can start.
    async fn create_task_execution(
        &self,
        task: &tasks::Task,
    ) -> Result<(task_executions::TaskExecution, Vec<Variable>)> {
        let task_execution = task_executions::TaskExecution::new(
            &self.pipeline.metadata.namespace_id,
            &self.pipeline.metadata.pipeline_id,
            self.run.run_id,
            task.clone(),
        );

        let mut conn = self
            .api_state
            .storage
            .write_conn()
            .await
            .context("Could not establish connection to database")?;

        let storage_task_execution = task_execution
            .clone()
            .try_into()
            .context("Could not serialize task execution to storage object")?;

        match storage::task_executions::insert(&mut conn, &storage_task_execution).await {
            Ok(()) => {}
            // If the task execution already exists then we're probably attempting to recover it.
            Err(storage::StorageError::Exists) => {}
            Err(e) => return Err(e).context("Could not insert new task_execution into storage"),
        }

        // Determine the task executions final variable set and pass them in.
        let env_vars = combine_variables(
            &self.run,
            task,
            &task_api_base_url(&self.api_state.config.server),
        );

        let env_vars_json =
            serde_json::to_string(&env_vars).context("Could not serialize env vars into json")?;

        storage::task_executions::update(
            &mut conn,
            &self.pipeline.metadata.namespace_id,
            &self.pipeline.metadata.pipeline_id,
            self.run
                .run_id
                .try_into()
                .context("Could not convert run id to appropriate integer type")?,
            &task.id,
            storage::task_executions::UpdatableFields {
                variables: Some(env_vars_json),
                ..Default::default()
            },
        )
        .await
        .context("Could not update task_execution with correct variables")?;

        self.set_task_execution_state(&mut conn, &task_execution, task_executions::State::Waiting)
            .await
            .context("Could not update task_execution state to waiting")?;

        Ok((task_execution, env_vars))
    }

    /// Waits until every one of the task's parents has finished, and returns how each of them ended. Returns None if
    /// the task execution was cancelled while it waited, in which case it's already been marked as cancelled.
    async fn wait_for_parents(
        &self,
        event_stream: &mut Box<dyn EventListener>,
        task: &tasks::Task,
    ) -> Option<HashMap<String, task_executions::Status>> {
        // <task_execution_id> => <status>
        let mut completed_tasks = HashMap::new();

        loop {
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

                // Listen to see if we should stop waiting and set task execution as cancelled.
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

                    if let Err(e) = self
                        .end_task_execution(&task.id, task_executions::Status::Cancelled, None)
                        .await
                    {
                        error!(error = %format!("{e:#}"),
                               namespace_id = self.pipeline.metadata.namespace_id.clone(),
                               pipeline_id = self.pipeline.metadata.pipeline_id.clone(),
                               run_id = self.run.run_id,
                               task_execution_id = task.id.clone(),
                               "Could not mark task execution as cancelled");

                        // Saving it is what normally announces it finished, so announce it here instead; otherwise
                        // the run would wait on this task forever.
                        self.api_state.event_bus.clone().publish(
                            event_utils::Kind::CompletedTaskExecution {
                                namespace_id: self.pipeline.metadata.namespace_id.clone(),
                                pipeline_id: self.pipeline.metadata.pipeline_id.clone(),
                                run_id: self.run.run_id,
                                task_execution_id: task.id.clone(),
                                status: task_executions::Status::Cancelled,
                            },
                        );
                    }

                    return None;
                }
                _ => {}
            }

            if task
                .depends_on
                .keys()
                .all(|parent_id| completed_tasks.contains_key(parent_id))
            {
                return Some(completed_tasks);
            }
        }
    }

    /// Fills in the secrets and objects the task's variables refer to.
    ///
    /// At first glance this may seem like it could happen earlier, but it's important that it runs only after a
    /// task's parents have finished; this lets one task pass variables to the tasks downstream of it.
    async fn resolve_variables(&self, env_vars: Vec<Variable>) -> Result<Vec<Variable>> {
        interpolate_vars(
            &self.api_state.storage,
            self.api_state.secret_store.as_ref(),
            self.api_state.object_store.as_ref(),
            &self.pipeline.metadata.namespace_id,
            &self.pipeline.metadata.pipeline_id,
            &env_vars,
        )
        .await
    }

    /// Makes sure Docker has the task's image, loading it from Gofer's image store if it doesn't, and returns the name
    /// Docker knows it by.
    ///
    /// Docker's copy has to have exactly the layers the image was registered with. Docker finds a layer by its diff
    /// id, and its JSON parser reads `rootfs` and `RootFS` as the same field where ours doesn't, so only Docker's own
    /// answer settles what it would run.
    async fn load_image(
        &self,
        task: &tasks::Task,
        timeline: &mpsc::UnboundedSender<task_executions::TimelineEntry>,
    ) -> std::result::Result<String, task_executions::StatusReason> {
        let container_error = |description: String| task_executions::StatusReason {
            reason: task_executions::StatusReasonType::ContainerError,
            description,
        };

        let digest: images::Digest =
            task.image.digest.parse().map_err(|e| {
                container_error(format!("The task's image has an invalid digest; {e}"))
            })?;

        let image = match self.api_state.images.stored_image(&digest).await {
            Ok(Some(image)) => image,
            Ok(None) => {
                return Err(task_executions::StatusReason {
                    reason: task_executions::StatusReasonType::FailedPrecondition,
                    description: format!(
                        "The image for version {} of this pipeline is no longer stored, so the task can't run. \
                         Running `gofer up` builds a new version.",
                        self.pipeline.config.version
                    ),
                });
            }
            Err(e) => {
                return Err(container_error(format!(
                    "Could not read the task's image from Gofer's image store; {e}"
                )));
            }
        };

        let name = digest.docker_name();

        // A copy that doesn't match, like something tagged with this name by hand, gets replaced once.
        for replace in [false, true] {
            let layers = (self.api_state.containers)
                .load_image(
                    &name,
                    replace,
                    || {
                        _ = timeline.send(task_executions::TimelineEntry::now(
                            task_executions::Phase::LoadingImage,
                        ));
                    },
                    || self.api_state.images.docker_archive(&image),
                )
                .await
                .map_err(|e| {
                    container_error(format!("Could not load the task's image into Docker; {e}"))
                })?;

            if layers == image.diff_ids {
                return Ok(name);
            }
        }

        Err(container_error(
            "Docker's copy of the task's image doesn't have the layers it was registered with, so it wasn't run. \
             Docker read the image differently from how Gofer checked it."
                .into(),
        ))
    }

    /// Starts the task execution's container from its image, which [`Self::load_image`] made sure Docker has.
    async fn start_container(
        &self,
        image: &str,
        container_id: &str,
        env_vars: Vec<Variable>,
        timeline: &mpsc::UnboundedSender<task_executions::TimelineEntry>,
    ) -> Result<containers::StartContainerResponse> {
        let response = self
            .api_state
            .containers
            .start_container(containers::StartContainerRequest {
                id: container_id.to_string(),
                image: containers::Image::Loaded(image.to_string()),
                variables: env_vars
                    .into_iter()
                    .map(|var| (var.key, var.value))
                    .collect(),
                networking: None,
                // How the task runs is built into its image.
                entrypoint: None,
                command: None,
                run_folder: Some(self.run_folder()),
                timeline: Some(timeline.clone()),
            })
            .await?;

        _ = timeline.send(task_executions::TimelineEntry::now(
            task_executions::Phase::Running,
        ));

        Ok(response)
    }

    /// Hands a running task execution off to be watched: one task collects its logs and another follows its
    /// container until it exits or is cancelled.
    fn spawn_task_execution_monitors(
        &self,
        event_stream: Box<dyn EventListener>,
        container_id: String,
        task_execution: task_executions::TaskExecution,
    ) {
        // Tracked so the run can wait for every task's logs to be written before it removes their containers.
        let self_clone = self.clone();
        let container_id_clone = container_id.clone();
        let task_id = task_execution.task_id.clone();
        self.log_streams.spawn(async move {
            self_clone
                .handle_log_updates(container_id_clone, task_id)
                .await;
        });

        let self_clone = self.clone();
        tokio::spawn(async move {
            let task_id = task_execution.task_id.clone();

            if let Err(e) = self_clone
                .monitor_task_execution(event_stream, container_id, task_execution)
                .await
            {
                error!(namespace_id = &self_clone.pipeline.metadata.namespace_id,
                        pipeline_id = &self_clone.pipeline.metadata.pipeline_id,
                        run_id = self_clone.run.run_id,
                        task_id,
                        error = %e, "Error occurred during monitoring of task execution");
            }
        });
    }

    /// Completes a task execution that ended before its container ran, so there's no real exit code to record.
    async fn end_task_execution(
        &self,
        task_id: &str,
        status: task_executions::Status,
        reason: Option<task_executions::StatusReason>,
    ) -> Result<()> {
        let mut conn = self
            .api_state
            .storage
            .write_conn()
            .await
            .context("Could not establish connection to database")?;

        self.set_task_execution_complete(&mut conn, task_id, 1, status, reason)
            .await
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
    /// single writer so they're saved in order without racing each other, and so starting a container can report its
    /// own phases (like pulling an image) while they happen rather than after. The writer stops once every sender is
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

    /// Monitors the container status from Docker and listens for any relevant events.
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
                        .containers
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
                    // TODO(): This should be more robust. Docker can fail a single status request, and we don't want to
                    // fail the entire thing just because it didn't return an update once.
                    let response = match self
                        .api_state
                        .containers
                        .get_state(containers::GetStateRequest {
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
                                        reason: task_executions::StatusReasonType::ContainerError,
                                        description:
                                            "Could not query Docker for the task execution state"
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
                        containers::ContainerState::Unknown => {
                            let mut conn = self.api_state.storage.write_conn().await?;

                            if let Err(e) = self
                                .set_task_execution_complete(
                                    &mut conn,
                                    &task_id,
                                    1,
                                    task_executions::Status::Unknown,
                                    Some(task_executions::StatusReason {
                                        reason: task_executions::StatusReasonType::ContainerError,
                                        description:
                                            "An unknown error has occurred on the container level; This should (ideally) never happen. Please contact support or file a bug."
                                                .into(),
                                    }),
                                )
                                .await
                            {
                                bail!("Could not update task execution while attempting to set execution as complete; {:#?}", e)
                            };
                            return Ok(());
                        }
                        containers::ContainerState::Running
                        | containers::ContainerState::Paused
                        | containers::ContainerState::Restarting => {}
                        containers::ContainerState::Exited => {
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
                            .containers
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
            .containers
            .get_logs(containers::GetLogsRequest {
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
                                error = %e, "Failed to parse log stream; container error encountered.");
                            return;
                        },
                    };

                    let mut file = file.lock().await;

                    match log_object {
                        containers::Log::Unknown => {
                            error!(namespace_id = &self.pipeline.metadata.namespace_id,
                                pipeline_id = &self.pipeline.metadata.pipeline_id,
                                run_id = self.run.run_id,
                                task_id = &task_id,
                                "Received malformed log from container (Unknown Log type); aborting");
                        },
                        containers::Log::Stdout(log) => {
                            if let Err(e) = file.write_all(&log).await {
                                error!(namespace_id = &self.pipeline.metadata.namespace_id,
                                    pipeline_id = &self.pipeline.metadata.pipeline_id,
                                    run_id = self.run.run_id,
                                    task_id = &task_id,
                                    error = %e, path = path.to_string_lossy().to_string(),
                                    "Failed to write stdout log for container");
                            }
                        },
                        containers::Log::Stderr(log) => {
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
}

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

#[cfg(test)]
mod tests {
    use super::*;

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
}
