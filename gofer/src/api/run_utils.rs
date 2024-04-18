use crate::{
    api::{
        epoch_milli, event_utils, in_progress_runs_key, interpolate_vars, objects, pipelines, runs,
        task_executions, tasks, ApiState, Variable, VariableSource, GOFER_EOF,
    },
    scheduler, storage,
};
use anyhow::{bail, Context, Result};
use dashmap::DashMap;
use futures::future::join_all;
use futures::StreamExt;
use gofer_sdk::config::pipeline_secret;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::{atomic, Arc};
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;
use tracing::{debug, error};

fn run_specific_api_key_id(run_id: u64) -> String {
    format!("gofer_api_token_run_id_{run_id}")
}

/// The shepherd is a run specific object that guides Gofer runs and tasks through their execution.
/// It's a core construct within the Gofer execution model and contains most of the logic of how a run operates with
/// is mostly consisted of state-machine like actions.
#[derive(Debug)]
pub struct Shepherd {
    pub api_state: Arc<ApiState>,
    pub pipeline: pipelines::Pipeline,
    pub run: runs::Run,
    pub task_executions: DashMap<String, task_executions::TaskExecution>,
    pub stop_run: atomic::AtomicBool,
}

impl Shepherd {
    pub fn new(api_state: Arc<ApiState>, pipeline: pipelines::Pipeline, run: runs::Run) -> Self {
        api_state
            .in_progress_runs
            .entry(in_progress_runs_key(
                &pipeline.metadata.namespace_id,
                &pipeline.metadata.pipeline_id,
            ))
            .and_modify(|value| {
                value.fetch_add(1, atomic::Ordering::SeqCst);
            })
            .or_insert(atomic::AtomicU64::from(1));

        api_state
            .event_bus
            .clone()
            .publish(event_utils::Kind::StartedRun {
                namespace_id: pipeline.metadata.namespace_id.clone(),
                pipeline_id: pipeline.metadata.pipeline_id.clone(),
                run_id: run.run_id,
            });

        Self {
            api_state,
            pipeline,
            run,
            task_executions: DashMap::new(),
            stop_run: false.into(),
        }
    }

    async fn set_task_execution_complete(
        &self,
        id: &str,
        exit_code: u8,
        status: task_executions::Status,
        reason: Option<task_executions::StatusReason>,
    ) -> Result<()> {
        if !self.task_executions.contains_key(id) {
            bail!("Could not find task execution");
        }

        self.task_executions.alter(id, |_, mut value| {
            value.state = task_executions::State::Complete;
            value.status = status.clone();
            value
        });

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

        self.task_executions
            .alter(&task_execution.task_id, |_, mut value| {
                value.state = state.clone();
                value
            });

        Ok(())
    }

    /// Check the dependency tree of a task to see if all it's parents have finished.
    fn parent_tasks_complete(
        &self,
        dependency_map: &HashMap<String, tasks::RequiredParentStatus>,
    ) -> bool {
        for parent_id in dependency_map.keys() {
            let parent = match self.task_executions.get(parent_id) {
                Some(parent) => parent,
                None => return false,
            };

            if parent.state != task_executions::State::Complete {
                return false;
            }
        }

        true
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
        dependency_map: &HashMap<String, tasks::RequiredParentStatus>,
    ) -> Result<()> {
        for (parent, required_status) in dependency_map {
            let parent_execution = match self.task_executions.get(parent) {
                Some(p) => p,
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
                    if parent_execution.status != task_executions::Status::Successful
                        && parent_execution.status != task_executions::Status::Failed
                        && parent_execution.status != task_executions::Status::Skipped
                    {
                        bail!("Parent '{:#?}' has incorrect status '{}' for required 'any' dependency",
                        parent_execution, parent_execution.status);
                    }
                }
                tasks::RequiredParentStatus::Success => {
                    if parent_execution.status != task_executions::Status::Successful {
                        bail!("Parent '{:#?}' has incorrect status '{}' for required 'successful' dependency",
                        parent_execution, parent_execution.status);
                    }
                }
                tasks::RequiredParentStatus::Failure => {
                    if parent_execution.status != task_executions::Status::Failed {
                        bail!("Parent '{:#?}' has incorrect status '{}' for required 'failed' dependency",
                        parent_execution, parent_execution.status);
                    }
                }
            }
        }

        Ok(())
    }

    /// Determines the final run status based on all finished task executions.
    async fn process_run_finish(&self) {
        // A run is only successful if all task_executions were successful. If any task_execution is in an
        // unknown or failed state we fail the run, if any task_execution is cancelled we mark the run as cancelled.

        for execution in self.task_executions.iter() {
            let task_execution = execution.value();

            match task_execution.status {
                task_executions::Status::Unknown | task_executions::Status::Failed => {
                    let result = self
                        .set_run_complete(
                            runs::Status::Failed,
                            Some(runs::StatusReason {
                                reason: runs::StatusReasonType::AbnormalExit,
                                description: "One or more task executions failed during execution"
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
                task_executions::Status::Successful => {}
                task_executions::Status::Cancelled => {
                    let result = self
                        .set_run_complete(
                            runs::Status::Cancelled,
                            Some(runs::StatusReason {
                                reason: runs::StatusReasonType::AbnormalExit,
                                description:
                                    "One or more task executions were cancelled during execution"
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
                task_executions::Status::Skipped => {}
            }
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

    /// Monitors all task execution statuses and determines the final run status based on all
    /// finished task executions. It will block until all task executions have finished.
    async fn wait_task_execution_finish(&self, container_id: &str, task_id: &str) -> Result<()> {
        loop {
            let response = match self
                .api_state
                .scheduler
                .get_state(scheduler::GetStateRequest {
                    id: container_id.into(),
                })
                .await
            {
                Ok(resp) => resp,
                Err(err) => {
                    if let Err(e) = self
                        .set_task_execution_complete(
                            task_id,
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
                            task_id,
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
                | scheduler::ContainerState::Restarting => continue,
                scheduler::ContainerState::Exited => {
                    // We determine if something worked based on the exit code of the container.
                    let mut exit_code = 1;

                    if let Some(code) = response.exit_code {
                        exit_code = code
                    }

                    if exit_code == 0 {
                        if let Err(e) = self
                            .set_task_execution_complete(
                                task_id,
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
                            task_id,
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
                scheduler::ContainerState::Cancelled => {
                    if let Err(e) = self
                        .set_task_execution_complete(
                            task_id,
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

    /// Tracks state and log progress of a task execution. It automatically updates the provided task execution
    /// with the resulting state change(s). This function will block until the task-run has
    /// reached a terminal state.
    async fn monitor_task_execution(
        self: Arc<Self>,
        container_id: &str,
        task_id: &str,
    ) -> Result<()> {
        let container_id_clone = container_id.to_owned();
        let task_id_clone = task_id.to_owned();
        let self_clone = self.clone();

        tokio::spawn(async move {
            self_clone
                .handle_log_updates(container_id_clone, task_id_clone)
                .await
        });

        self.wait_task_execution_finish(container_id, task_id)
            .await
            .context("Encountered error while waiting for task execution result")?;

        Ok(())
    }

    /// Removes run level objects from the object store once a run is past it's expiry threshold.
    async fn handle_run_object_expiry(self: Arc<Self>) {
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

    async fn handle_run_log_expiry(self: Arc<Self>) {
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

    /// Registers[^1] and launches a brand new task execution as part of a larger run for a specific task.
    /// It blocks until the task execution has completed.
    ///
    /// [^1]: The register parameter controls whether the task is registered in the database, announces it's creation
    /// via events. It's useful to turn this off when we're trying to revive a task execution that is previously lost.
    async fn launch_task_execution(self: Arc<Self>, task: tasks::Task, register: bool) {
        // Start by creating a new task execution and saving it to the state machine and disk.
        let new_task_execution = task_executions::TaskExecution::new(
            &self.pipeline.metadata.namespace_id,
            &self.pipeline.metadata.pipeline_id,
            self.run.run_id,
            task.clone(),
        );

        self.task_executions.insert(
            new_task_execution.task_id.clone(),
            new_task_execution.clone(),
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

        if register {
            if let Err(e) =
                storage::task_executions::insert(&mut conn, &storage_task_execution).await
            {
                error!(namespace_id = &self.pipeline.metadata.namespace_id,
                    pipeline_id = &self.pipeline.metadata.pipeline_id,
                    run_id = self.run.run_id,
                    task_id = task.id,
                    error = %e, "Could not insert new task_execution into storage");
                return;
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

        // Now we examine the validity of the task execution to be started and wait for it's dependents to finish running.
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

        // First we need to make sure all the parents of the current task are in a finished state.
        while !self.parent_tasks_complete(&new_task_execution.task.depends_on) {
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
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
        if let Err(e) = self.task_dependencies_satisfied(&new_task_execution.task.depends_on) {
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
        self.task_executions.insert(
            new_task_execution.task_id.clone(),
            new_task_execution.clone(),
        );

        self.api_state
            .event_bus
            .clone()
            .publish(event_utils::Kind::StartedTaskExecution {
                namespace_id: self.pipeline.metadata.namespace_id.clone(),
                pipeline_id: self.pipeline.metadata.pipeline_id.clone(),
                run_id: self.run.run_id,
                task_execution_id: new_task_execution.task_id.clone(),
            });

        // Since we move self we just copy these values so we can put it in the error log.
        // There is probably a better way to do this.
        let namespace_id = self.pipeline.metadata.namespace_id.clone();
        let pipeline_id = self.pipeline.metadata.pipeline_id.clone();
        let run_id = self.run.run_id;

        // Block until task_execution is finished and log results.
        if let Err(e) = self
            .monitor_task_execution(&container_name, &new_task_execution.task_id)
            .await
        {
            error!(namespace_id = namespace_id,
                pipeline_id = pipeline_id,
                run_id = run_id,
                task_id = task.id,
                error = %e, "Encountered error while waiting for task_execution to finish");
        };
    }

    pub async fn execute_task_tree(self: Arc<Self>) {
        // Launch per-run clean up jobs.
        tokio::spawn(self.clone().handle_run_object_expiry());
        tokio::spawn(self.clone().handle_run_log_expiry());

        let mut conn = match self.api_state.storage.conn().await {
            Ok(conn) => conn,
            Err(e) => {
                error!(error = %e, "Could not establish connection to database while attempting to wait for run finish");
                return;
            }
        };

        let fields = storage::runs::UpdatableFields {
            state: Some(runs::State::Running.to_string()),
            ..Default::default()
        };

        if let Err(e) = storage::runs::update(
            &mut conn,
            &self.pipeline.metadata.namespace_id,
            &self.pipeline.metadata.pipeline_id,
            self.run.run_id.try_into().unwrap_or_default(),
            fields,
        )
        .await
        {
            error!(namespace_id = &self.pipeline.metadata.namespace_id,
                   pipeline_id = &self.pipeline.metadata.pipeline_id,
                   run_id = self.run.run_id,
                   error = %e, "Could not update run while attempting to execute task tree");
            return;
        };

        let mut task_handles = vec![];

        for task in self.pipeline.config.tasks.values() {
            let handle = tokio::spawn(self.clone().launch_task_execution(task.clone(), true));
            task_handles.push(handle);
        }

        // Wait for all the task executions to finish.
        join_all(task_handles).await;

        // Finally process the run now that all the tasks have finished.
        self.process_run_finish().await
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
