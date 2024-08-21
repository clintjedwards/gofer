use crate::{
    api::{
        epoch_milli, event_utils, in_progress_runs_key, interpolate_vars, objects, pipelines, runs,
        task_executions, tasks, ApiState, Variable, VariableSource, GOFER_EOF,
    },
    scheduler, storage,
};
use anyhow::{bail, Context, Result};
use futures::StreamExt;
use gofer_sdk::config::pipeline_secret;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::{atomic, Arc, Barrier};
use tokio::{io::AsyncWriteExt, sync::Mutex};
use tracing::{debug, error, trace};

fn run_specific_api_key_id(run_id: u64) -> String {
    format!("gofer_api_token_run_id_{run_id}")
}

/// Shepherd is a run specific object that guides Gofer runs and tasks through their execution.
/// It's a core construct within the Gofer execution model and contains most of the logic of how a run operates.
///
/// TODO(): Explain more about how the shepard arch works.
#[derive(Debug, Clone)]
pub struct Shepherd {
    pub api_state: Arc<ApiState>,
    pub pipeline: pipelines::Pipeline,
    pub run: runs::Run,
}

impl Shepherd {
    pub fn new(api_state: Arc<ApiState>, pipeline: pipelines::Pipeline, run: runs::Run) -> Self {
        Self {
            api_state,
            pipeline,
            run,
        }
    }

    /// Start run launches the run and its tasks and then listens to the event bus for related run events and task events.
    /// Upon recieving one of these events it then appropriately updates the run entry in the database with the
    /// correct data.
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
            .in_progress_runs
            .entry(in_progress_runs_key(
                &self.pipeline.metadata.namespace_id,
                &self.pipeline.metadata.pipeline_id,
            ))
            .and_modify(|value| {
                value.fetch_add(1, atomic::Ordering::SeqCst);
            })
            .or_insert(atomic::AtomicU64::from(1));

        self.api_state
            .event_bus
            .clone()
            .publish(event_utils::Kind::StartedRun {
                namespace_id: self.pipeline.metadata.namespace_id.clone(),
                pipeline_id: self.pipeline.metadata.pipeline_id.clone(),
                run_id: self.run.run_id,
            });

        let fields = storage::runs::UpdatableFields {
            state: Some(runs::State::Running.to_string()),
            ..Default::default()
        };

        let mut conn = self
            .api_state
            .storage
            .conn()
            .await
            .context("Could not open connection to database")?;

        if let Err(e) = storage::runs::update(
            &mut conn,
            &self.pipeline.metadata.namespace_id,
            &self.pipeline.metadata.pipeline_id,
            self.run.run_id.try_into().unwrap_or_default(),
            fields,
        )
        .await
        {
            bail!(
                "Could not update run while attempting to start run; {:#?}",
                e
            )
        };

        // Lastly start the run monitor and the launch the task executions. We use a barrier here to make sure
        // that all threads are able to grab event bus listeners at roughly the same point so that they don't
        // end up missing any messages from threads that might be faster.

        // The barrier knows when to tell all tasks to continue by waiting until all tasks check in.
        // We calculate this value by taking the number of all the tasks we are about to start and then adding
        // one more for the run monitor itself.
        let barrier_threshold = self.pipeline.config.tasks.len() + 1;

        let barrier = Arc::new(std::sync::Barrier::new(barrier_threshold));

        for task in self.pipeline.config.tasks.values() {
            let thread_barrier = barrier.clone();
            tokio::spawn(
                self.clone()
                    .launch_task_execution(thread_barrier, task.clone()),
            );
        }

        let thread_barrier = barrier.clone();
        let self_clone = self.clone();
        tokio::spawn(async move { self_clone.monitor_run(thread_barrier).await });

        Ok(())
    }

    /// Listens to messages from the event bus and updates the status of the run in-progress.
    async fn monitor_run(&self, barrier: Arc<std::sync::Barrier>) {
        let mut event_stream = self.api_state.event_bus.subscribe();

        // wait for all the other threads to get to this point so everyone starts out at the same point in the event bus.
        barrier.wait();

        let mut completed_tasks = std::collections::HashMap::new();
        let mut is_cancelled = false;
        let mut is_failed = false;

        // Wait for events and then process what should happen after we recieve them.
        loop {
            let event = match event_stream.recv().await {
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
                        self.api_state.event_bus.clone().publish(
                            event_utils::Kind::StartedTaskExecutionCancellation {
                                namespace_id: self.pipeline.metadata.namespace_id.clone(),
                                pipeline_id: self.pipeline.metadata.pipeline_id.clone(),
                                run_id: self.run.run_id,
                                task_execution_id: task.id.clone(),
                                timeout: self.api_state.config.api.task_execution_stop_timeout,
                            },
                        )
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

        if is_cancelled {
            let result = self
                .set_run_complete(
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
            let result = self
                .set_run_complete(
                    runs::Status::Failed,
                    Some(runs::StatusReason {
                        reason: runs::StatusReasonType::AbnormalExit,
                        description: "One or more task executions failed during execution".into(),
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

        if let Err(e) = self.set_run_complete(runs::Status::Successful, None).await {
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
    async fn launch_task_execution(self, barrier: Arc<Barrier>, task: tasks::Task) {
        let mut event_stream = self.api_state.event_bus.subscribe();

        // wait for all the other threads to get to this point so everyone starts out at the same point in the event bus.
        barrier.wait();

        // Start by creating a new task execution and saving it to the state machine and disk.
        let new_task_execution = task_executions::TaskExecution::new(
            &self.pipeline.metadata.namespace_id,
            &self.pipeline.metadata.pipeline_id,
            self.run.run_id,
            task.clone(),
        );

        let mut conn = match self.api_state.storage.conn().await {
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

        let env_vars = combine_variables(&self.run, &task);
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
            .set_task_execution_state(&new_task_execution, task_executions::State::Waiting)
            .await
        {
            error!(namespace_id = &self.pipeline.metadata.namespace_id,
                pipeline_id = &self.pipeline.metadata.pipeline_id,
                run_id = self.run.run_id,
                task_id = task.id,
                error = %e, "Could not update task_execution state to waiting");
            return;
        };

        // <task_execution_id> => <status>
        let mut completed_tasks = HashMap::new();

        // Wait for events and then process what should happen after we recieve them.
        // When we get an event see if its an event that pertains to us and then handle it.
        // At this stage we haven't started the task execution yet, we're simply checking to see
        // if we should start it.
        if !new_task_execution.task.depends_on.is_empty() {
            'main_loop: loop {
                let event = match event_stream.recv().await {
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

                        if let Err(e) = self
                            .set_task_execution_complete(
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

        if let Err(e) = self
            .set_task_execution_state(&new_task_execution, task_executions::State::Processing)
            .await
        {
            error!(namespace_id = &self.pipeline.metadata.namespace_id,
                pipeline_id = &self.pipeline.metadata.pipeline_id,
                run_id = self.run.run_id,
                task_id = task.id,
                error = %e, "Could not update task_execution state to processing");
            return;
        };

        // Then check to make sure that the parents all finished in the required states. If not
        // we'll mark this task as skipped since it's requirements for running weren't met.
        if let Err(e) =
            self.task_dependencies_satisfied(completed_tasks, &new_task_execution.task.depends_on)
        {
            if let Err(e) = self
                .set_task_execution_complete(
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

        // First we attempt to find any object/secret store variables and replace them
        // with the correct var. At first glance this may seem like a task that can move upwards
        // but it's important that this run only after a task's parents have already run
        // this enables users to be sure that one task can pass variables to other downstream tasks.

        // We create a copy of variables so that we can substitute in secrets and objects.
        // to eventually pass them into the start container function.
        let env_vars = match interpolate_vars(
            &self.api_state,
            &self.pipeline.metadata.namespace_id,
            &self.pipeline.metadata.pipeline_id,
            Some(self.run.run_id),
            &env_vars,
        )
        .await
        {
            Ok(env_vars) => env_vars,
            Err(e) => {
                if let Err(e) = self
                    .set_task_execution_complete(
                        &new_task_execution.task_id,
                        1,
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

        if let Err(e) = self
            .api_state
            .scheduler
            .start_container(scheduler::StartContainerRequest {
                id: container_name.clone(),
                image: new_task_execution.task.image.clone(),
                variables: env_vars
                    .into_iter()
                    .map(|var| (var.key, var.value))
                    .collect(),
                registry_auth: new_task_execution
                    .task
                    .registry_auth
                    .clone()
                    .map(|auth| auth.into()),
                always_pull: false,
                networking: None,
                entrypoint: new_task_execution.task.entrypoint.clone(),
                command: new_task_execution.task.command.clone(),
            })
            .await
        {
            if let Err(e) = self
                .set_task_execution_complete(
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
        };

        trace!(
            namespace_id = self.pipeline.metadata.namespace_id.clone(),
            pipeline_id = self.pipeline.metadata.pipeline_id.clone(),
            run_id = self.run.run_id,
            task_execution_id = &new_task_execution.task_id,
            "Started task execution"
        );

        if let Err(e) = storage::task_executions::update(
            &mut conn,
            &self.pipeline.metadata.namespace_id,
            &self.pipeline.metadata.pipeline_id,
            run_id_i64,
            &new_task_execution.task_id,
            storage::task_executions::UpdatableFields {
                state: Some(task_executions::State::Running.to_string()),
                started: Some(epoch_milli().to_string()),
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

    async fn set_task_execution_complete(
        &self,
        id: &str,
        exit_code: u8,
        status: task_executions::Status,
        reason: Option<task_executions::StatusReason>,
    ) -> Result<()> {
        let mut conn = self
            .api_state
            .storage
            .conn()
            .await
            .context("Could not open connection to database")?;

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
            &mut conn,
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
        status: runs::Status,
        reason: Option<runs::StatusReason>,
    ) -> Result<()> {
        self.api_state.in_progress_runs.alter(
            &in_progress_runs_key(
                &self.pipeline.metadata.namespace_id,
                &self.pipeline.metadata.pipeline_id,
            ),
            |_, value| {
                value.fetch_sub(1, atomic::Ordering::SeqCst);
                value
            },
        );

        let mut conn = self
            .api_state
            .storage
            .conn()
            .await
            .context("Could not open connection to database")?;

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
            &mut conn,
            &self.pipeline.metadata.namespace_id,
            &self.pipeline.metadata.pipeline_id,
            self.run.run_id.try_into()?,
            fields,
        )
        .await
        .context("Could not update run status in storage")?;

        self.api_state
            .event_bus
            .clone()
            .publish(event_utils::Kind::CompletedRun {
                namespace_id: self.pipeline.metadata.namespace_id.clone(),
                pipeline_id: self.pipeline.metadata.pipeline_id.clone(),
                run_id: self.run.run_id,
                status: self.run.status.clone(),
            });

        Ok(())
    }

    async fn set_task_execution_state(
        &self,
        task_execution: &task_executions::TaskExecution,
        state: task_executions::State,
    ) -> Result<()> {
        let mut conn = self
            .api_state
            .storage
            .conn()
            .await
            .context("Could not open connection to database")?;

        let fields = storage::task_executions::UpdatableFields {
            state: Some(state.to_string()),
            ..Default::default()
        };

        storage::task_executions::update(
            &mut conn,
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

    pub async fn parallelism_limit_exceeded(&self) -> bool {
        let pipeline_run_limit = self.pipeline.config.parallelism;
        let global_run_limit = self.api_state.config.api.run_parallelism_limit;

        if pipeline_run_limit == 0 && global_run_limit == 0 {
            return false;
        }

        let mut limit = pipeline_run_limit;

        if pipeline_run_limit > global_run_limit {
            limit = global_run_limit
        }

        if limit == 0 {
            return false;
        }

        let runs_key = in_progress_runs_key(
            &self.pipeline.metadata.namespace_id,
            &self.pipeline.metadata.pipeline_id,
        );
        let runs_in_progress: u64 = match self.api_state.in_progress_runs.get(&runs_key) {
            Some(runs_in_progress) => runs_in_progress.value().load(atomic::Ordering::SeqCst),
            None => 0,
        };

        runs_in_progress >= limit
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
                        bail!("Parent '{}' has incorrect status '{}' for required 'successful' dependency",
                        parent, parent_status);
                    }
                }
                tasks::RequiredParentStatus::Failure => {
                    if parent_status != task_executions::Status::Failed {
                        bail!("Parent '{}' has incorrect status '{}' for required 'failed' dependency",
                        parent, parent_status);
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
        mut event_stream: tokio::sync::broadcast::Receiver<event_utils::Event>,
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

        loop {
            tokio::select! {
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
                            if let Err(e) = self
                                .set_task_execution_complete(
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
                            if let Err(e) = self
                                .set_task_execution_complete(
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

                            if exit_code == 0 {
                                if let Err(e) = self
                                    .set_task_execution_complete(
                                        &task_id,
                                        exit_code,
                                        task_executions::Status::Successful,
                                        None,
                                    )
                                    .await
                                {
                                    bail!("Could not update task execution while attempting to set execution as complete; {:#?}", e)
                                };
                            } else if let Err(e) = self
                                .set_task_execution_complete(
                                    &task_id,
                                    exit_code,
                                    task_executions::Status::Failed,
                                    Some(task_executions::StatusReason {
                                        reason: task_executions::StatusReasonType::AbnormalExit,
                                        description:
                                            "Task execution has exited with an abnormal exit code.".into(),
                                    }),
                                )
                                .await
                            {
                                bail!("Could not update task execution while attempting to set execution as complete; {:#?}", e)
                            }

                            return Ok(());
                        }
                    }
                },
                result = event_stream.recv() => {
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

                        if let Err(e) = self
                            .set_task_execution_complete(
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

    /// Removes run level objects from the object store once a run is past it's expiry threshold.
    async fn handle_run_object_expiry(self) {
        let limit = self.api_state.config.object_store.run_object_expiry;

        let mut conn = match self.api_state.storage.conn().await {
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
        let limit = self.api_state.config.api.task_execution_log_retention;

        let mut conn = match self.api_state.storage.conn().await {
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
/// or the user when they attempt to start a new run manually. Since these are the most likely to be
/// edited adhoc they are treated as the most important.
pub fn combine_variables(run: &runs::Run, task: &tasks::Task) -> Vec<Variable> {
    let system_injected_vars = system_injected_vars(run, task, task.inject_api_token);

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

/// On every run Gofer injects some vars that are determined by the system.
/// These are usually meant to give the user some basic information that they can pull
/// into their program about the details of the run.
fn system_injected_vars(
    run: &runs::Run,
    task: &tasks::Task,
    inject_api_token: bool,
) -> HashMap<String, Variable> {
    let mut vars = HashMap::from([
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

    if inject_api_token {
        vars.insert(
            "GOFER_API_TOKEN".into(),
            Variable {
                key: "GOFER_API_TOKEN".into(),
                value: pipeline_secret(&run_specific_api_key_id(run.run_id)),
                source: VariableSource::System,
            },
        );
    }

    vars
}
