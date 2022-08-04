use super::{combine_variables, parse_interpolation_syntax, InterpolationKind};
use crate::api::{epoch, fmt, Api, GOFER_EOF};
use crate::{scheduler, storage};
use anyhow::Result;
use dashmap::DashMap;
use futures::StreamExt;
use gofer_models::Variable;
use gofer_models::{event, pipeline, run, task, task_run};
use slog_scope::{debug, error};
use sqlx::SqliteConnection;
use std::{collections::HashMap, sync::Arc};
use tokio::io::AsyncWriteExt;

/// Used to keep track of a run as it progresses through the necessary states.
#[derive(Debug, Clone)]
pub struct RunStateMachine {
    api: Arc<Api>,
    pipeline: pipeline::Pipeline,
    run: run::Run,
    task_runs: DashMap<String, task_run::TaskRun>,
}

impl RunStateMachine {
    pub async fn new(api: Arc<Api>, pipeline: pipeline::Pipeline, run: run::Run) -> Self {
        Self {
            api,
            pipeline,
            run,
            task_runs: DashMap::new(),
        }
    }

    /// Returns true if there are more runs in progress than the parallelism limit
    /// of a pipeline allows.
    /// If there was an error getting the current number of runs, we fail closed as the
    /// functionality of failing a parallelism_limit is usually retrying until it succeeds.
    pub async fn parallelism_limit_exceeded(&self, conn: &mut SqliteConnection) -> bool {
        let mut limit = self.pipeline.parallelism;

        if limit == 0 && self.api.conf.general.run_parallelism_limit == 0 {
            return false;
        }

        if limit > self.api.conf.general.run_parallelism_limit {
            limit = self.api.conf.general.run_parallelism_limit
        }

        let runs = match storage::runs::list(
            conn,
            0,
            0,
            &self.pipeline.namespace,
            &self.pipeline.id,
        )
        .await
        {
            Ok(runs) => runs,
            Err(_) => return true,
        };

        let mut runs_in_progress = 0;
        for run in runs {
            if run.state != run::State::Complete {
                runs_in_progress += 1;
            }
        }

        if runs_in_progress >= limit {
            return true;
        }

        false
    }

    /// Mark a task run object as finished.
    pub async fn set_task_run_finished(
        &self,
        id: &str,
        code: Option<u8>,
        status: task_run::Status,
        failure: Option<task_run::StatusReason>,
    ) {
        // Update the task run's status inside the map first.
        self.task_runs.alter(id, |_, mut task_run| {
            task_run.state = task_run::State::Complete;
            task_run.status = status.clone();

            task_run
        });

        let task_run = match self.task_runs.get(id) {
            Some(task_run) => task_run,
            None => {
                error!("could not set task run finished; task_run does not exist");
                return;
            }
        };

        let mut conn = match self.api.storage.conn().await {
            Ok(conn) => conn,
            Err(e) => {
                error!("could not set task run finished; database connection error"; "error" => format!("{:?}", e));
                return;
            }
        };

        // Then update the task_run's status on disk.
        if let Err(e) = storage::task_runs::update(
            &mut conn,
            &task_run,
            storage::task_runs::UpdatableFields {
                exit_code: code,
                status: Some(status.clone()),
                state: Some(task_run::State::Complete),
                ended: Some(epoch()),
                failure,
                ..Default::default()
            },
        )
        .await
        {
            error!("could not set task run finished; database error"; "error" => format!("{:?}", e));
            return;
        };

        // Lastly publish to the event bus that the task_run has finished.
        let namespace_id = self.run.namespace.clone();
        let pipeline_id = self.run.pipeline.clone();
        let run_id = self.run.id;
        let task_run_id = task_run.task.id.clone();
        let status = task_run.status.clone();
        let api = self.api.clone();

        tokio::spawn(async move {
            api.event_bus
                .publish(event::Kind::CompletedTaskRun {
                    namespace_id,
                    pipeline_id,
                    run_id,
                    task_run_id,
                    status,
                })
                .await;
        });
    }

    /// Mark a run object as finished.
    pub async fn set_run_finished(&self, status: run::Status, failure: Option<run::StatusReason>) {
        let mut conn = match self.api.storage.conn().await {
            Ok(conn) => conn,
            Err(e) => {
                error!("could not set run finished"; "error" => format!("{:?}", e));
                return;
            }
        };

        if let Err(e) = storage::runs::update(
            &mut conn,
            &self.run,
            storage::runs::UpdatableFields {
                state: Some(run::State::Complete),
                status: Some(status.clone()),
                failure_info: failure,
                ended: Some(epoch()),
                ..Default::default()
            },
        )
        .await
        {
            error!("could not not update run during run monitoring"; "error" => format!("{:?}", e));
        };

        // Lastly publish to the event bus that the task_run has finished.
        let namespace_id = self.run.namespace.clone();
        let pipeline_id = self.run.pipeline.clone();
        let run_id = self.run.id;
        let api = self.api.clone();

        tokio::spawn(async move {
            api.event_bus
                .publish(event::Kind::CompletedRun {
                    namespace_id,
                    pipeline_id,
                    run_id,
                    status,
                })
                .await;
        });
    }

    /// Creates all child task_runs for a given run. After creating all task runs it then
    /// blocks and monitors the run until it is finished.
    pub async fn execute_task_tree(self) {
        let state_machine = Arc::new(self);
        let object_expiry_clone = state_machine.clone();
        let log_expiry_clone = state_machine.clone();

        // Launch per-run clean up jobs.
        tokio::spawn(object_expiry_clone.handle_run_object_expiry());
        tokio::spawn(log_expiry_clone.handle_run_log_expiry());

        // TODO(clintjedwards): create token here.

        // TODO(clintjedwards): include notifiers here.

        // Launch a new task run for each task found.
        for task in state_machine.pipeline.tasks.values() {
            let task_clone = state_machine.clone();
            let task = task.clone();
            tokio::spawn(async move { task_clone.launch_task_run(task).await });
        }

        // Finally monitor the entire run until it finishes. This will block until the run has ended.
        state_machine.wait_run_finish().await;
    }

    /// Check a dependency tree to see if all parents tasks have been finished.
    fn parent_tasks_finished(
        &self,
        dependencies: &HashMap<String, task::RequiredParentStatus>,
    ) -> bool {
        for parent in dependencies.keys() {
            if let Some(parent_task_run) = self.task_runs.get(parent) {
                if parent_task_run.state != task_run::State::Complete {
                    return false;
                }
            } else {
                return false;
            }
        }

        true
    }

    /// Check a dependency tree to see if all parent tasks are in the correct states.
    fn task_dependencies_satisfied(
        &self,
        dependencies: &HashMap<String, task::RequiredParentStatus>,
    ) -> anyhow::Result<()> {
        for (parent, required_status) in dependencies {
            if let Some(parent_task_run) = self.task_runs.get(parent) {
                match required_status {
                    task::RequiredParentStatus::Unknown => {
                        return Err(anyhow::anyhow!(
                            "A parent dependency should never be in the state 'Unknown'"
                        ));
                    }
                    task::RequiredParentStatus::Any => {
                        if parent_task_run.status != task_run::Status::Successful
                            && parent_task_run.status != task_run::Status::Failed
                            && parent_task_run.status != task_run::Status::Skipped
                        {
                            return Err(anyhow::anyhow!(
                                "parent '{parent}' is in incorrect state '{}'
                            for required 'any' dependency",
                                parent_task_run.status
                            ));
                        }
                    }
                    task::RequiredParentStatus::Success => {
                        if parent_task_run.status != task_run::Status::Successful {
                            return Err(anyhow::anyhow!(
                                "parent '{parent}' is in incorrect state '{}';
                            task requires it to be in state '{}'",
                                parent_task_run.status,
                                task_run::Status::Successful.to_string()
                            ));
                        }
                    }
                    task::RequiredParentStatus::Failure => {
                        if parent_task_run.status != task_run::Status::Failed {
                            return Err(anyhow::anyhow!(
                                "parent '{parent}' is in incorrect state '{}';
                            task requires it to be in state '{}'",
                                parent_task_run.status,
                                task_run::Status::Failed.to_string()
                            ));
                        }
                    }
                }
            } else {
                return Err(anyhow::anyhow!(
                    "parent '{}' was not found in completed tasks but is required for task",
                    parent
                ));
            }
        }

        Ok(())
    }

    /// Monitors all task run statuses and determines the final run status based on all
    /// finished task runs. It will block until all task runs have finished.
    pub async fn wait_run_finish(&self) {
        // First update the run to be in the correct state of running.
        let mut conn = match self.api.storage.conn().await {
            Ok(conn) => conn,
            Err(e) => {
                error!("could not not update run during run monitoring"; "error" => format!("{:?}", e));
                return;
            }
        };

        if let Err(e) = storage::runs::update(
            &mut conn,
            &self.run,
            storage::runs::UpdatableFields {
                state: Some(run::State::Running),
                ..Default::default()
            },
        )
        .await
        {
            error!("could not not update run during run monitoring"; "error" => format!("{:?}", e));
            return;
        }

        // If the task_run map hasn't had all the entries come in we should wait until it does.
        loop {
            if self.task_runs.len() != self.pipeline.tasks.len() {
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                continue;
            }

            break;
        }

        // We loop over the task_runs to make sure all are complete.
        'outer: loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            for item in &self.task_runs {
                let task_run = item.value();
                if task_run.state != task_run::State::Complete {
                    continue 'outer;
                }
            }

            break;
        }

        // When all are finished we now need to get a final tallying of what the run's result is.
        // A run is only successful if all task_runs were successful. If any task_run is in an
        // unknown or failed state we fail the run, if any task_run is cancelled we mark the run as cancelled.
        for item in &self.task_runs {
            let task_run = item.value();
            match task_run.status {
                task_run::Status::Unknown | task_run::Status::Failed => {
                    self.set_run_finished(
                        run::Status::Failed,
                        Some(run::StatusReason {
                            reason: run::Reason::AbnormalExit,
                            description: "One or more task runs failed during execution"
                                .to_string(),
                        }),
                    )
                    .await;
                    return;
                }
                task_run::Status::Successful => continue,
                task_run::Status::Cancelled => {
                    self.set_run_finished(
                        run::Status::Cancelled,
                        Some(run::StatusReason {
                            reason: run::Reason::AbnormalExit,
                            description: "One or more task runs were cancelled during execution"
                                .to_string(),
                        }),
                    )
                    .await;
                    return;
                }
                task_run::Status::Skipped => continue,
            }
        }

        self.set_run_finished(run::Status::Successful, None).await;
    }

    pub async fn wait_task_run_finish(&self, container_name: String, id: String) -> Result<()> {
        loop {
            let resp = match self
                .api
                .scheduler
                .get_state(scheduler::GetStateRequest {
                    name: container_name.clone(),
                })
                .await
            {
                Ok(resp) => resp,
                Err(e) => {
                    self.set_task_run_finished(
                        &id,
                        None,
                        task_run::Status::Unknown,
                        Some(task_run::StatusReason {
                            reason: task_run::Reason::SchedulerError,
                            description: format!(
                                "Could not query the scheduler for task run state; {}.",
                                e
                            ),
                        }),
                    )
                    .await;
                    return Err(anyhow::anyhow!(format!("{:?}", e)));
                }
            };

            match resp.state {
                scheduler::ContainerState::Unknown => {
                    self.set_task_run_finished(
                        &id,
                        resp.exit_code,
                        task_run::Status::Unknown,
                        Some(task_run::StatusReason {
                            reason: task_run::Reason::SchedulerError,
                            description: "An unknown error has occurred on the scheduler level;
                                This should never happen."
                                .to_string(),
                        }),
                    )
                    .await;
                    return Ok(());
                }
                scheduler::ContainerState::Running
                | scheduler::ContainerState::Restarting
                | scheduler::ContainerState::Paused => {
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                    continue;
                }
                scheduler::ContainerState::Exited => {
                    if let Some(exit_code) = resp.exit_code {
                        if exit_code == 0 {
                            self.set_task_run_finished(
                                &id,
                                Some(0),
                                task_run::Status::Successful,
                                None,
                            )
                            .await;
                            return Ok(());
                        }

                        self.set_task_run_finished(
                            &id,
                            Some(exit_code),
                            task_run::Status::Failed,
                            Some(task_run::StatusReason {
                                reason: task_run::Reason::AbnormalExit,
                                description: "Task run exited with abnormal exit code.".to_string(),
                            }),
                        )
                        .await;

                        return Ok(());
                    }

                    self.set_task_run_finished(
                        &id,
                        None,
                        task_run::Status::Unknown,
                        Some(task_run::StatusReason {
                            reason: task_run::Reason::AbnormalExit,
                            description: "Task run exited without an exit code.".to_string(),
                        }),
                    )
                    .await;

                    return Ok(());
                }
            }
        }
    }

    /// Tracks state and log progress of a task_run. It automatically updates the provided task-run
    /// with the resulting state change(s). This function will block until the task-run has
    /// reached a terminal state.
    pub async fn monitor_task_run(self: Arc<Self>, container_name: String, id: String) {
        let self_clone = self.clone();
        let container_name_clone = container_name.clone();
        let id_clone = id.clone();

        tokio::spawn(async move {
            self_clone
                .handle_log_updates(container_name_clone, id_clone)
                .await
        });

        if let Err(e) = self.wait_task_run_finish(container_name, id.clone()).await {
            error!("could not get state for container update";
                    "task_run" => id, "error" => format!("{:?}", e));
        }
    }

    /// Removes run level object_store objects once a run is past it's expiry threshold.
    pub async fn handle_run_object_expiry(self: Arc<Self>) {
        let limit = self.api.conf.object_store.run_object_expiry;

        let mut conn = match self.api.storage.conn().await {
            Ok(conn) => conn,
            Err(e) => {
                error!("could not get runs for run expiry processing; db connection error"; "error" => format!("{:?}", e));
                return;
            }
        };

        // We ask for the limit of runs plus one extra.
        let runs = match storage::runs::list(
            &mut conn,
            0,
            limit + 1,
            &self.pipeline.namespace,
            &self.pipeline.id,
        )
        .await
        {
            Ok(runs) => runs,
            Err(e) => {
                error!("could not get runs for run expiry processing; db error"; "error" => format!("{:?}", e));
                return;
            }
        };

        // If there aren't enough runs to reach the limit there is nothing to remove.
        if limit > (runs.len() as u64) {
            return;
        }

        if runs.last().is_none() {
            return;
        }

        let mut expired_run = runs.last().unwrap().to_owned();

        // If the run is still in progress wait for it to be done.
        while expired_run.state != run::State::Complete {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

            expired_run = match storage::runs::get(
                &mut conn,
                &self.pipeline.namespace,
                &self.pipeline.id,
                expired_run.id,
            )
            .await
            {
                Ok(run) => run,
                Err(e) => {
                    error!("could not get run while performing run object expiry"; "error" => format!("{:?}", e));
                    continue;
                }
            };
        }

        if expired_run.store_info.is_none() {
            return;
        };

        let mut store_info = expired_run.store_info.clone().unwrap();

        if store_info.is_expired {
            return;
        };

        for object_key in &store_info.keys {
            if let Err(e) = self
                .api
                .object_store
                .delete_object(&fmt::run_object_key(
                    &self.pipeline.namespace,
                    &self.pipeline.id,
                    expired_run.id,
                    object_key,
                ))
                .await
            {
                error!("could not delete run object for expiry processing"; "error" => format!("{:?}", e));
            }
        }

        store_info.is_expired = true;

        expired_run.store_info = Some(store_info.clone());
        if let Err(e) = storage::runs::update(
            &mut conn,
            &expired_run,
            storage::runs::UpdatableFields {
                store_info: Some(store_info.clone()),
                ..Default::default()
            },
        )
        .await
        {
            error!("could not not update run for expiry processing"; "error" => format!("{:?}", e));
        }

        debug!("old run objects removed";
            "run_age_limit" => limit,
            "run_id" => expired_run.id,
            "removed_objects" => format!("{:?}", store_info.keys),
        );
    }

    pub async fn handle_run_log_expiry(self: Arc<Self>) {
        let limit = self.api.conf.general.task_run_log_expiry;

        let mut conn = match self.api.storage.conn().await {
            Ok(conn) => conn,
            Err(e) => {
                error!("could not get runs for run log expiry processing"; "error" => format!("{:?}", e));
                return;
            }
        };

        // We ask for the limit of runs plus one extra.
        let runs = match storage::runs::list(
            &mut conn,
            0,
            limit,
            &self.pipeline.namespace,
            &self.pipeline.id,
        )
        .await
        {
            Ok(runs) => runs,
            Err(e) => {
                error!("could not get runs for run log expiry processing"; "error" => format!("{:?}", e));
                return;
            }
        };

        // If there aren't enough runs to reach the limit there is nothing to remove.
        if limit > (runs.len() as u64) {
            return;
        }

        if runs.last().is_none() {
            return;
        }

        let mut expired_run = runs.last().unwrap().to_owned();

        // If the run is still in progress wait for it to be done.
        while expired_run.state != run::State::Complete {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

            expired_run = match storage::runs::get(
                &mut conn,
                &self.pipeline.namespace,
                &self.pipeline.id,
                expired_run.id,
            )
            .await
            {
                Ok(run) => run,
                Err(e) => {
                    error!("could not get run while performing run log expiry"; "error" => format!("{:?}", e));
                    continue;
                }
            };
        }

        let mut task_runs: Vec<task_run::TaskRun>;

        'outer: loop {
            task_runs = match storage::task_runs::list(
                &mut conn,
                0,
                0,
                &self.pipeline.namespace,
                &self.pipeline.id,
                expired_run.id,
            )
            .await
            {
                Ok(task_runs) => task_runs,
                Err(e) => {
                    error!("could not get task run while performing run log expiry"; "error" => format!("{:?}", e));
                    continue;
                }
            };

            for task_run in &task_runs {
                if task_run.state != task_run::State::Complete {
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                    continue 'outer;
                }
            }

            break;
        }

        let mut removed_files = vec![];

        for task_run in &task_runs {
            if task_run.logs_expired || task_run.logs_removed {
                continue;
            }

            let log_file_path =
                fmt::task_run_log_path(&self.api.conf.general.task_run_logs_dir, task_run);

            if let Err(e) = tokio::fs::remove_file(&log_file_path).await {
                error!("io error while deleting log file";
                        "path" => log_file_path, "error" => format!("{:?}", e));
                continue;
            };

            if let Err(e) = storage::task_runs::update(
                &mut conn,
                task_run,
                storage::task_runs::UpdatableFields {
                    logs_expired: Some(true),
                    logs_removed: Some(true),
                    ..Default::default()
                },
            )
            .await
            {
                error!("could not update task run while removing log files";
                        "task run id" => task_run.id.clone(), "error" => format!("{:?}", e));
                continue;
            }

            removed_files.push(log_file_path);
        }

        debug!("old run logs removed";
            "log_age_limit" => limit,
            "run_id" => expired_run.id,
            "removed_files" => format!("{:?}", removed_files),
        );
    }

    pub async fn handle_log_updates(&self, container_name: String, task_run_id: String) {
        let task_run = match self.task_runs.get(&task_run_id) {
            Some(task_run) => task_run,
            None => {
                error!("could not track log updates for task run; task run does not exist");
                return;
            }
        };

        let mut log_stream = self.api.scheduler.get_logs(scheduler::GetLogsRequest {
            name: container_name,
        });

        let log_path = fmt::task_run_log_path(&self.api.conf.general.task_run_logs_dir, &task_run);

        drop(task_run);

        let mut log_file = match tokio::fs::File::create(&log_path).await {
            Ok(log_file) => log_file,
            Err(e) => {
                error!("could not open task run log file for writing"; "error" => format!("{:?}", e));
                return;
            }
        };

        while let Some(log) = log_stream.next().await {
            let log = match log {
                Ok(log) => log,
                Err(e) => {
                    error!("encountered error while writing log file";
                            "file_path" => log_path, "error" => format!("{:?}", e));
                    return;
                }
            };

            match log {
                scheduler::Log::Unknown => {
                    error!("encountered error while writing log file; log line unknown but should be stdout/stderr";
                            "file_path" => log_path);
                    return;
                }
                scheduler::Log::Stderr(log) | scheduler::Log::Stdout(log) => {
                    if let Err(e) = log_file.write_all(&log).await {
                        error!("encountered error while writing log file;";
                                "file_path" => log_path, "error" => format!("{:?}", e));
                        return;
                    };
                }
            }
        }

        if let Err(e) = log_file.write_all(GOFER_EOF.as_bytes()).await {
            error!("encountered error while writing log file;";
            "file_path" => log_path, "error" => format!("{:?}", e));
        }
    }

    /// Takes in a map of mixed plaintext and raw secret/store strings and populates it with
    /// the fetched strings for each type.
    pub async fn interpolate_vars(&self, variables: &mut Vec<Variable>) -> Result<()> {
        for variable in variables {
            if let Some(secret_key) =
                parse_interpolation_syntax(InterpolationKind::Secret, &variable.value)
            {
                let secret = match self
                    .api
                    .secret_store
                    .get_secret(&fmt::secret_key(
                        &self.pipeline.namespace,
                        &self.pipeline.id,
                        &secret_key,
                    ))
                    .await
                {
                    Ok(secret) => secret,
                    Err(e) => {
                        return Err(anyhow::anyhow!(
                            "could not get secret '{}'; {}",
                            secret_key,
                            e
                        ))
                    }
                };

                variable.value = String::from_utf8_lossy(&secret).to_string();
            };

            if let Some(pipeline_key) =
                parse_interpolation_syntax(InterpolationKind::Pipeline, &variable.value)
            {
                let pipeline = match self
                    .api
                    .object_store
                    .get_object(&fmt::pipeline_object_key(
                        &self.pipeline.namespace,
                        &self.pipeline.id,
                        &pipeline_key,
                    ))
                    .await
                {
                    Ok(pipeline) => pipeline,
                    Err(e) => {
                        return Err(anyhow::anyhow!(
                            "could not get pipeline '{}'; {}",
                            pipeline_key,
                            e
                        ))
                    }
                };

                variable.value = String::from_utf8_lossy(&pipeline).to_string();
            };

            if let Some(run_key) =
                parse_interpolation_syntax(InterpolationKind::Run, &variable.value)
            {
                let run = match self
                    .api
                    .object_store
                    .get_object(&fmt::run_object_key(
                        &self.pipeline.namespace,
                        &self.pipeline.id,
                        self.run.id,
                        &run_key,
                    ))
                    .await
                {
                    Ok(run) => run,
                    Err(e) => {
                        return Err(anyhow::anyhow!("could not get run '{}'; {}", run_key, e))
                    }
                };

                variable.value = String::from_utf8_lossy(&run).to_string();
            };
        }

        Ok(())
    }

    pub async fn set_task_run_state(
        &self,
        conn: &mut SqliteConnection,
        task_run: &task_run::TaskRun,
        state: task_run::State,
    ) {
        if let Err(e) = storage::task_runs::update(
            conn,
            task_run,
            storage::task_runs::UpdatableFields {
                state: Some(state.clone()),
                ..Default::default()
            },
        )
        .await
        {
            error!("could not update task run"; "error" => format!("{:?}", e));
        }

        // Update the task run's status inside the map first.
        self.task_runs.alter(&task_run.id, |_, mut taskrun| {
            taskrun.state = state;

            taskrun
        });
    }

    /// Launches a brand new task run as part of a larger run for a specific task.
    /// It blocks until the task run has completed.
    pub async fn launch_task_run(self: Arc<Self>, task: task::Task) {
        // Start by creating new task run and saving it to the state machine and disk.
        let mut conn = match self.api.storage.conn().await {
            Ok(conn) => conn,
            Err(e) => {
                error!("could not launch task; database connection error"; "error" => format!("{:?}", e));
                return;
            }
        };

        let new_task_run = task_run::TaskRun::new(
            &self.pipeline.namespace,
            &self.pipeline.id,
            self.run.id,
            task.clone(),
        );

        self.task_runs
            .insert(new_task_run.id.clone(), new_task_run.clone());

        if let Err(e) = storage::task_runs::insert(&mut conn, &new_task_run).await {
            error!("could not add task run to storage"; "error" => format!("{:?}", e));
            return;
        }

        // Alert the event bus that a new task run is being started.
        let self_clone = self.clone();
        let namespace_id = self.pipeline.namespace.to_string();
        let pipeline_id = self.pipeline.id.to_string();
        let run_id = self.run.id;
        let task_run_id = task.id.clone();

        tokio::spawn(async move {
            self_clone
                .api
                .event_bus
                .publish(event::Kind::CreatedTaskRun {
                    namespace_id,
                    pipeline_id,
                    run_id,
                    task_run_id,
                })
                .await;
        });

        let mut env_vars = combine_variables(&self.run, &task);

        // Determine the task run's final variable set and pass them in.
        if let Err(e) = storage::task_runs::update(
            &mut conn,
            &new_task_run,
            storage::task_runs::UpdatableFields {
                variables: Some(env_vars.clone()),
                ..Default::default()
            },
        )
        .await
        {
            error!("could not update task run"; "error" => format!("{:?}", e));
            return;
        }

        // Now we examine the validity of the task run to be started and wait for it's dependents to
        // finish running.

        self.set_task_run_state(&mut conn, &new_task_run, task_run::State::Waiting)
            .await;

        // First we need to make sure all the parents of the current task are in a finished state.
        while !self.parent_tasks_finished(&new_task_run.task.depends_on) {
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }

        self.set_task_run_state(&mut conn, &new_task_run, task_run::State::Processing)
            .await;

        // Then check to make sure that the parents all finished in the required states. If not
        // we'll have to mark this task as skipped.
        if let Err(e) = self.task_dependencies_satisfied(&new_task_run.task.depends_on) {
            self.set_task_run_finished(
                &new_task_run.id,
                None,
                task_run::Status::Skipped,
                Some(task_run::StatusReason {
                    reason: task_run::Reason::FailedPrecondition,
                    description: format!("task could not be run due to unmet dependencies; {}", e),
                }),
            )
            .await;

            return;
        }

        // After this point we're sure the task is in a state to be run. So we attempt to
        // contact the scheduler and start the container.

        // First we attempt to find any object/secret store variables and replace them
        // with the correct var. At first glance this may seem like a task that can move upwards
        // but it's important that this run only after a task's parents have already run
        // this enables users to be sure that one task can pass variables to other downstream tasks.

        // We create a copy of variables so that we can substitute in secrets and objects.
        // to eventually pass them into the start container function.
        if let Err(e) = self.interpolate_vars(&mut env_vars).await {
            self.set_task_run_finished(&new_task_run.id, None,
                task_run::Status::Failed,
                Some(task_run::StatusReason {
                        reason: task_run::Reason::FailedPrecondition,
                        description: format!(
                            "task could not be run due to inability to retrieve interpolated variables; {}",
                            e
                        )
                    }),
                ).await;

            return;
        };

        let env_vars: HashMap<String, String> = env_vars
            .into_iter()
            .map(|variable| (variable.key, variable.value))
            .collect();

        let container_name = fmt::task_container_id(
            &self.pipeline.namespace,
            &self.pipeline.id,
            self.run.id,
            &new_task_run.id,
        );

        if let Err(e) = self
            .api
            .scheduler
            .start_container(scheduler::StartContainerRequest {
                name: container_name.clone(),
                image: new_task_run.task.image.clone(),
                variables: env_vars,
                registry_auth: {
                    if let Some(mut auth) = new_task_run.task.registry_auth.clone() {
                        if let Some(secret) =
                            parse_interpolation_syntax(InterpolationKind::Secret, &auth.pass)
                        {
                            auth.pass = secret;
                        };

                        Some(auth.into())
                    } else {
                        None
                    }
                },
                always_pull: false,
                enable_networking: false,
                entrypoint: new_task_run.task.entrypoint.clone(),
                command: new_task_run.task.command.clone(),
            })
            .await
        {
            self.set_task_run_finished(
                &new_task_run.id,
                None,
                task_run::Status::Failed,
                Some(task_run::StatusReason {
                    reason: task_run::Reason::SchedulerError,
                    description: format!(
                        "task could not be run due to inability to be scheduled; {}",
                        e
                    ),
                }),
            )
            .await;
            return;
        };

        if let Err(e) = storage::task_runs::update(
            &mut conn,
            &new_task_run,
            storage::task_runs::UpdatableFields {
                state: Some(task_run::State::Running),
                started: Some(epoch()),
                ..Default::default()
            },
        )
        .await
        {
            error!("could not add task run to storage"; "error" => format!("{:?}", e));
            return;
        }

        // Alert the event bus that a new task run is being started.
        let self_clone = self.clone();
        let namespace_id = self.pipeline.namespace.to_string();
        let pipeline_id = self.pipeline.id.to_string();
        let run_id = self.run.id;
        let task_run_id = task.id.clone();

        tokio::spawn(async move {
            self_clone
                .api
                .event_bus
                .publish(event::Kind::StartedTaskRun {
                    namespace_id,
                    pipeline_id,
                    run_id,
                    task_run_id,
                })
                .await;
        });

        // Update the task run's status inside the map first.
        self.task_runs.alter(&new_task_run.id, |_, mut task_run| {
            task_run.state = task_run::State::Running;

            task_run
        });

        // Block until task_run is finished and log results.
        self.monitor_task_run(container_name, new_task_run.id).await;
    }
}
