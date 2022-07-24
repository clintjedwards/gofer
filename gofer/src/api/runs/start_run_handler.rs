use super::variables_to_vec;
use crate::api::{fmt, validate, Api, GOFER_EOF};
use crate::{scheduler, storage};
use anyhow::Result;
use dashmap::DashMap;
use futures::StreamExt;
use gofer_models::{event, pipeline, run, task, task_run};
use gofer_models::{Variable, VariableOwner, VariableSensitivity};
use gofer_proto::{StartRunRequest, StartRunResponse};
use slog_scope::{debug, error};
use sqlx::Acquire;
use std::collections::HashMap;
use std::sync::Arc;
use strum::{Display, EnumString};
use tokio::io::AsyncWriteExt;
use tonic::{Response, Status};

/// Gofer allows users to enter special interpolation strings such that
/// special functionality is substituted when Gofer reads these strings
/// in a user's pipeline configuration.
#[derive(Debug, Display, EnumString)]
pub enum InterpolationKind {
    Unknown,
    /// secret{{\<key\>]}}
    Secret,
    /// pipeline{{\<key\>]}}
    Pipeline,
    /// run{{\<key\>}}
    Run,
}

type StatusMap = dashmap::DashMap<String, (task_run::Status, task_run::State)>;

/// Checks a string for the existence of a interpolation format. ex: "secret{{ example }}".
/// If an interpolation was found we returns some, if not we return none.
///
/// Currently the supported interpolation syntaxes are:
/// `secret{{ example }}` for inserting from the secret store.
/// `pipeline{{ example }}` for inserting from the pipeline object store.
/// `run{{ example }}` for inserting from the run object store.
pub fn parse_interpolation_syntax(kind: InterpolationKind, input: &str) -> Option<String> {
    let mut variable = input.trim();
    let interpolation_prefix = format!("{}{{", kind.to_string().to_lowercase());
    let interpolation_suffix = "}}";
    if variable.starts_with(&interpolation_prefix) && variable.ends_with(interpolation_suffix) {
        variable = variable.strip_prefix(&interpolation_prefix).unwrap();
        variable = variable.strip_suffix(interpolation_suffix).unwrap();
        return Some(variable.trim().to_string());
    }

    None
}

/// Check a dependency tree to see if all parents tasks have been finished.
fn parent_tasks_finished(
    status_map: &StatusMap,
    dependencies: &HashMap<String, task::RequiredParentStatus>,
) -> bool {
    for parent in dependencies.keys() {
        if let Some(status_entry) = status_map.get(parent) {
            if status_entry.1 != task_run::State::Complete {
                return false;
            }
        } else {
            return false;
        }
    }

    true
}

/// Check a dependency tree to see if all parent task are in the correct states.
fn task_dependencies_satisfied(
    status_map: &StatusMap,
    dependencies: &HashMap<String, task::RequiredParentStatus>,
) -> anyhow::Result<()> {
    for (parent, required_status) in dependencies {
        if let Some(status_entry) = status_map.get(parent) {
            match required_status {
                task::RequiredParentStatus::Unknown => {
                    return Err(anyhow::anyhow!(
                        "A parent dependency should never be in the state 'Unknown'"
                    ));
                }
                task::RequiredParentStatus::Any => {
                    if status_entry.0 != task_run::Status::Successful
                        && status_entry.0 != task_run::Status::Failed
                        && status_entry.0 != task_run::Status::Skipped
                    {
                        return Err(anyhow::anyhow!(
                            "parent '{parent}' is in incorrect state '{}'
                            for required 'any' dependency",
                            status_entry.0
                        ));
                    }
                }
                task::RequiredParentStatus::Success => {
                    if status_entry.0 != task_run::Status::Successful {
                        return Err(anyhow::anyhow!(
                            "parent '{parent}' is in incorrect state '{}';
                            task requires it to be in state '{}'",
                            status_entry.0,
                            task_run::Status::Successful.to_string()
                        ));
                    }
                }
                task::RequiredParentStatus::Failure => {
                    if status_entry.0 != task_run::Status::Failed {
                        return Err(anyhow::anyhow!(
                            "parent '{parent}' is in incorrect state '{}';
                            task requires it to be in state '{}'",
                            status_entry.0,
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

/// On every run Gofer injects some vars that are determined by the system.
/// These are usually meant to give the user some basic information that they can pull
/// into their program about the details of the run.
fn system_injected_vars(run: &run::Run, task: &task::Task) -> HashMap<String, Variable> {
    HashMap::from([
        (
            "GOFER_PIPELINE_ID".to_string(),
            Variable {
                key: "GOFER_PIPELINE_ID".to_string(),
                value: run.pipeline.clone(),
                owner: VariableOwner::System,
                sensitivity: VariableSensitivity::Public,
            },
        ),
        (
            "GOFER_RUN_ID".to_string(),
            Variable {
                key: "GOFER_RUN_ID".to_string(),
                value: run.id.to_string(),
                owner: VariableOwner::System,
                sensitivity: VariableSensitivity::Public,
            },
        ),
        (
            "GOFER_TASK_ID".to_string(),
            Variable {
                key: "GOFER_TASK_ID".to_string(),
                value: task.id.clone(),
                owner: VariableOwner::System,
                sensitivity: VariableSensitivity::Public,
            },
        ),
        (
            "GOFER_TASK_IMAGE".to_string(),
            Variable {
                key: "GOFER_TASK_IMAGE".to_string(),
                value: task.image.clone(),
                owner: VariableOwner::System,
                sensitivity: VariableSensitivity::Public,
            },
        ),
        (
            "GOFER_API_TOKEN".to_string(),
            Variable {
                key: "GOFER_API_TOKEN".to_string(),
                value: "".to_string(), //TODO(clintjedwards): token needed here.
                owner: VariableOwner::System,
                sensitivity: VariableSensitivity::Private,
            },
        ),
    ])
}

/// We need to combine the environment variables we get from multiple sources in order to pass them
/// finally to the task run. The order in which they are passed is very important as they can and should
/// overwrite each other, even though the intention of prefixing the environment variables is to prevent
/// the chance of overwriting. The order in which they are passed into the extend function
/// determines the priority in reverse order. Last in the stack will overwrite any conflicts from the others.
///
/// 1) We first pass in the Gofer system specific envvars as these are the most replaceable on the totem pole.
/// 2) We pass in the task specific envvars defined by the user in the pipeline config.
/// 3) Lastly we pass in the run specific defined envvars. These are usually provided by either a trigger
/// or the user when they attempt to start a new run manually. Since these are the most likely to be
/// edited adhoc they are treated as the most important.
fn combine_variables(run: &run::Run, task: &task::Task) -> Vec<Variable> {
    let system_injected_vars = system_injected_vars(run, task);

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

    let mut task_run_vars = system_injected_vars;
    task_run_vars.extend(task_vars);
    task_run_vars.extend(run_vars);

    // It is possible for the user to enter an empty key, but that would be an error when
    // attempting to pass it to the docker container.
    task_run_vars = task_run_vars
        .into_iter()
        .filter_map(|(key, value)| {
            if key.is_empty() {
                None
            } else {
                Some((key, value))
            }
        })
        .collect();

    task_run_vars.into_iter().map(|(_, value)| value).collect()
}

impl Api {
    /// Returns true if there are more runs in progress than the parallelism limit
    /// of a pipeline allows.
    /// If there was an error getting the current number of runs, we fail closed as the
    /// functionality of failing a parallelism_limit is usually retrying until it succeeds.
    pub async fn parallelism_limit_exceeded(
        &self,
        namespace_id: &str,
        pipeline_id: &str,
        limit: u64,
    ) -> bool {
        let mut limit = limit;

        if limit == 0 && self.conf.general.run_parallelism_limit == 0 {
            return false;
        }

        if limit > self.conf.general.run_parallelism_limit {
            limit = self.conf.general.run_parallelism_limit
        }

        let mut conn = match self.storage.conn().await {
            Ok(conn) => conn,
            Err(_) => return true,
        };

        let runs = match storage::runs::list(&mut conn, 0, 0, namespace_id, pipeline_id).await {
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

    /// Removes run level object_store objects once a run is past it's expiry threshold.
    pub async fn handle_run_object_expiry(self: Arc<Self>, namespace: String, pipeline: String) {
        let limit = self.conf.object_store.run_object_expiry;

        let mut conn = match self.storage.conn().await {
            Ok(conn) => conn,
            Err(e) => {
                error!("could not get runs for run expiry processing"; "error" => format!("{:?}", e));
                return;
            }
        };

        // We ask for the limit of runs plus one extra.
        let runs = match storage::runs::list(&mut conn, 0, limit + 1, &namespace, &pipeline).await {
            Ok(runs) => runs,
            Err(e) => {
                error!("could not get runs for run expiry processing"; "error" => format!("{:?}", e));
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

            let mut conn = match self
                .storage
                .conn()
                .await
                .map_err(|e| Status::internal(e.to_string()))
            {
                Ok(conn) => conn,
                Err(e) => {
                    error!("could not get run while performing run object expiry"; "error" => format!("{:?}", e));
                    continue;
                }
            };

            expired_run = match storage::runs::get(&mut conn, &namespace, &pipeline, expired_run.id)
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
                .object_store
                .delete_object(&fmt::run_object_key(
                    &namespace,
                    &pipeline,
                    expired_run.id,
                    object_key,
                ))
                .await
            {
                error!("could not delete run object for expiry processing"; "error" => format!("{:?}", e));
            }
        }

        store_info.is_expired = true;

        let mut conn = match self
            .storage
            .conn()
            .await
            .map_err(|e| Status::internal(e.to_string()))
        {
            Ok(conn) => conn,
            Err(e) => {
                error!("could not delete run object for expiry processing"; "error" => format!("{:?}", e));
                return;
            }
        };

        expired_run.store_info = Some(store_info.clone());
        if let Err(e) = storage::runs::update(
            &mut conn,
            &expired_run.namespace,
            &expired_run.pipeline,
            expired_run.id,
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

    pub async fn handle_run_log_expiry(self: Arc<Self>, namespace: String, pipeline: String) {
        let limit = self.conf.general.task_run_log_expiry;

        let mut conn = match self.storage.conn().await {
            Ok(conn) => conn,
            Err(e) => {
                error!("could not get runs for run log expiry processing"; "error" => format!("{:?}", e));
                return;
            }
        };

        // We ask for the limit of runs plus one extra.
        let runs = match storage::runs::list(&mut conn, 0, limit, &namespace, &pipeline).await {
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

            let mut conn = match self
                .storage
                .conn()
                .await
                .map_err(|e| Status::internal(e.to_string()))
            {
                Ok(conn) => conn,
                Err(e) => {
                    error!("could not get run while performing run log expiry"; "error" => format!("{:?}", e));
                    continue;
                }
            };

            expired_run = match storage::runs::get(&mut conn, &namespace, &pipeline, expired_run.id)
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
            task_runs = match self
                .storage
                .list_task_runs(0, 0, &namespace, &pipeline, expired_run.id)
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

        for task_run in &mut task_runs {
            if task_run.logs_expired || task_run.logs_removed {
                continue;
            }

            let log_file_path =
                fmt::task_run_log_path(&self.conf.general.task_run_logs_dir, task_run);

            if let Err(e) = tokio::fs::remove_file(&log_file_path).await {
                error!("io error while deleting log file";
                        "path" => log_file_path, "error" => format!("{:?}", e));
                continue;
            };

            task_run.logs_expired = true;
            task_run.logs_removed = true;

            if let Err(e) = self.storage.update_task_run(task_run).await {
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

    pub async fn handle_log_updates(
        self: Arc<Self>,
        container_name: String,
        task_run: task_run::TaskRun,
    ) {
        let mut log_stream = self.scheduler.get_logs(scheduler::GetLogsRequest {
            name: container_name,
        });

        let log_path = fmt::task_run_log_path(&self.conf.general.task_run_logs_dir, &task_run);

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

    /// Tracks state and log progress of a task_run. It automatically updates the provided task-run
    /// with the resulting state change(s). This function will block until the task-run has
    /// reached a terminal state.
    ///
    /// Does not save changed task state to storage.
    pub async fn monitor_task_run(
        self: Arc<Self>,
        container_name: String,
        task_run: &mut task_run::TaskRun,
    ) {
        tokio::spawn(
            self.clone()
                .handle_log_updates(container_name.clone(), task_run.clone()),
        );

        if let Err(e) = self
            .clone()
            .wait_task_run_finish(container_name, task_run)
            .await
        {
            error!("could not get state for container update";
                    "task_run" => format!("{:?}", task_run.clone()), "error" => format!("{:?}", e));
        }
    }

    pub async fn wait_task_run_finish(
        self: Arc<Self>,
        container_name: String,
        task_run: &mut task_run::TaskRun,
    ) -> Result<()> {
        loop {
            let resp = match self
                .scheduler
                .get_state(scheduler::GetStateRequest {
                    name: container_name.clone(),
                })
                .await
            {
                Ok(resp) => resp,
                Err(e) => {
                    task_run.set_finished_abnormal(
                        task_run::Status::Unknown,
                        task_run::Failure {
                            kind: task_run::FailureKind::SchedulerError,
                            description: format!(
                                "Could not query the scheduler for task run state; {}.",
                                e
                            ),
                        },
                        None,
                    );
                    return Err(anyhow::anyhow!(format!("{:?}", e)));
                }
            };

            match resp.state {
                scheduler::ContainerState::Unknown => {
                    task_run.set_finished_abnormal(
                        task_run::Status::Unknown,
                        task_run::Failure {
                            kind: task_run::FailureKind::SchedulerError,
                            description: "An unknown error has occurred on the scheduler level;
                                This should never happen."
                                .to_string(),
                        },
                        resp.exit_code,
                    );
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
                            task_run.set_finished();
                            return Ok(());
                        }

                        task_run.set_finished_abnormal(
                            task_run::Status::Failed,
                            task_run::Failure {
                                kind: task_run::FailureKind::AbnormalExit,
                                description: "Task run exited with abnormal exit code.".to_string(),
                            },
                            Some(exit_code),
                        );

                        return Ok(());
                    }

                    task_run.set_finished_abnormal(
                        task_run::Status::Unknown,
                        task_run::Failure {
                            kind: task_run::FailureKind::AbnormalExit,
                            description: "Task run exited without an exit code.".to_string(),
                        },
                        None,
                    );

                    return Ok(());
                }
            }
        }
    }

    /// Monitors all task run statuses and determines the final run status based on all
    /// finished task runs. It will block until all task runs have finished.
    pub async fn monitor_run_status(
        self: Arc<Self>,
        tasks_num: u64,
        mut run: run::Run,
        status_map: Arc<StatusMap>,
    ) {
        run.state = run::State::Running;

        let mut conn = match self
            .storage
            .conn()
            .await
            .map_err(|e| Status::internal(e.to_string()))
        {
            Ok(conn) => conn,
            Err(e) => {
                error!("could not not update run during run monitoring"; "error" => format!("{:?}", e));
                return;
            }
        };

        if let Err(e) = storage::runs::update(
            &mut conn,
            &run.namespace,
            &run.pipeline,
            run.id,
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

        // Make sure all are complete.
        'outer: loop {
            if status_map.len() as u64 != tasks_num {
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                continue;
            }

            for item in status_map.iter() {
                let (_, state) = item.value();
                if state != &task_run::State::Complete {
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                    continue 'outer;
                }
            }

            break;
        }

        let mut all_successful = true;

        for item in status_map.iter() {
            let (status, _) = item.value();
            match status {
                task_run::Status::Unknown | task_run::Status::Failed => {
                    run.set_finished_abnormal(
                        run::Status::Failed,
                        run::FailureInfo {
                            reason: run::FailureReason::AbnormalExit,
                            description: "One or more task runs failed during execution"
                                .to_string(),
                        },
                    );
                    all_successful = false;
                    break;
                }
                task_run::Status::Successful => continue,
                task_run::Status::Cancelled => {
                    run.set_finished_abnormal(
                        run::Status::Cancelled,
                        run::FailureInfo {
                            reason: run::FailureReason::AbnormalExit,
                            description: "One or more task runs were cancelled during execution"
                                .to_string(),
                        },
                    );
                    all_successful = false;
                    break;
                }
                task_run::Status::Skipped => continue,
            }
        }

        if all_successful {
            run.set_finished();
        }

        if let Err(e) = storage::runs::update(
            &mut conn,
            &run.namespace,
            &run.pipeline,
            run.id,
            storage::runs::UpdatableFields {
                state: Some(run.state.clone()),
                status: Some(run.status.clone()),
                ..Default::default()
            },
        )
        .await
        {
            error!("could not not update run during run monitoring"; "error" => format!("{:?}", e));
            return;
        }

        self.event_bus
            .publish(event::Kind::CompletedRun {
                namespace_id: run.namespace.clone(),
                pipeline_id: run.pipeline.clone(),
                run_id: run.id,
                status: run.status,
            })
            .await;
    }

    /// Takes in a mpa of mixed plaintext and raw secret/store strings and populates it with
    /// the fetched strings for each type.
    pub async fn interpolate_vars(
        self: Arc<Self>,
        namespace: String,
        pipeline: String,
        run: u64,
        variables: &mut Vec<Variable>,
    ) -> Result<()> {
        for variable in variables {
            if let Some(secret_key) =
                parse_interpolation_syntax(InterpolationKind::Secret, &variable.value)
            {
                let secret = match self
                    .secret_store
                    .get_secret(&fmt::secret_key(&namespace, &pipeline, &secret_key))
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
                    .object_store
                    .get_object(&fmt::pipeline_object_key(
                        &namespace,
                        &pipeline,
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
                    .object_store
                    .get_object(&fmt::run_object_key(&namespace, &pipeline, run, &run_key))
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

    pub async fn mark_task_run_complete(
        self: Arc<Self>,
        run: &run::Run,
        task_run: &task_run::TaskRun,
        status_map: &StatusMap,
    ) {
        if let Err(e) = self.storage.update_task_run(task_run).await {
            error!("could not update task run"; "error" => format!("{:?}", e));
            return;
        }

        status_map.insert(
            task_run.task.id.clone(),
            (task_run.status.clone(), task_run.state.clone()),
        );

        let namespace_id = run.namespace.clone();
        let pipeline_id = run.pipeline.clone();
        let run_id = run.id;
        let task_run_id = task_run.task.id.clone();
        let status = task_run.status.clone();

        tokio::spawn(async move {
            self.event_bus
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

    /// Launches a brand new task run as part of a larger run for a specific task.
    /// It blocks until the task run has completed.
    pub async fn launch_task_run(
        self: Arc<Self>,
        run: run::Run,
        task: task::Task,
        status_map: Arc<StatusMap>,
    ) {
        let mut new_task_run =
            task_run::TaskRun::new(&run.namespace, &run.pipeline, run.id, task.clone());

        // Alert the event bus that a new task run is being started.
        let evt_self_clone = self.clone();
        let namespace_id = run.namespace.clone();
        let pipeline_id = run.pipeline.clone();
        let run_id = run.id;
        let task_run_id = task.id.clone();

        tokio::spawn(async move {
            evt_self_clone
                .event_bus
                .publish(event::Kind::StartedTaskRun {
                    namespace_id,
                    pipeline_id,
                    run_id,
                    task_run_id,
                })
                .await;
        });

        // Update the task run state in the database/status map.
        new_task_run.state = task_run::State::Processing;
        if let Err(e) = self.storage.create_task_run(&new_task_run).await {
            error!("could not add task run to storage"; "error" => format!("{:?}", e));
            return;
        }

        status_map.insert(
            new_task_run.task.id.clone(),
            (new_task_run.status.clone(), new_task_run.state.clone()),
        );

        // Determine the task run's final variable set and pass them in.
        new_task_run.variables = combine_variables(&run, &task);

        if let Err(e) = self.storage.update_task_run(&new_task_run).await {
            error!("could not update task run"; "error" => format!("{:?}", e));
            return;
        }

        // Now we examine the validity of the task run to be started and wait for it's dependents to
        // finish running.

        new_task_run.state = task_run::State::Waiting;
        if let Err(e) = self.storage.update_task_run_state(&new_task_run).await {
            error!("could not update task run"; "error" => format!("{:?}", e));
        }

        // First we need to make sure all the parents of the current task are in a finished state.
        while !parent_tasks_finished(&status_map, &new_task_run.task.depends_on) {
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }

        new_task_run.state = task_run::State::Processing;
        if let Err(e) = self.storage.update_task_run_state(&new_task_run).await {
            error!("could not update task run"; "error" => format!("{:?}", e));
        }

        // Then check to make sure that the parents all finished in the required states. If not
        // we'll have to mark this task as skipped.
        if let Err(e) = task_dependencies_satisfied(&status_map, &new_task_run.task.depends_on) {
            new_task_run.set_finished_abnormal(
                task_run::Status::Skipped,
                task_run::Failure {
                    kind: task_run::FailureKind::FailedPrecondition,
                    description: format!("task could not be run due to unmet dependencies; {}", e),
                },
                None,
            );

            if let Err(e) = self.storage.update_task_run(&new_task_run).await {
                error!("could not update task run"; "error" => format!("{:?}", e));
                return;
            }

            status_map.insert(
                new_task_run.task.id.clone(),
                (new_task_run.status.clone(), new_task_run.state.clone()),
            );

            tokio::spawn(async move {
                self.event_bus
                    .publish(event::Kind::CompletedTaskRun {
                        namespace_id: run.namespace.clone(),
                        pipeline_id: run.pipeline.clone(),
                        run_id: run.id,
                        task_run_id: task.id.clone(),
                        status: new_task_run.status.clone(),
                    })
                    .await;
            });

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
        let mut env_vars = new_task_run.variables.clone();

        if let Err(e) = self
            .clone()
            .interpolate_vars(
                run.namespace.clone(),
                run.pipeline.clone(),
                run.id,
                &mut env_vars,
            )
            .await
        {
            new_task_run.set_finished_abnormal(
                    task_run::Status::Failed,
                    task_run::Failure {
                        kind: task_run::FailureKind::FailedPrecondition,
                        description: format!(
                            "task could not be run due to inability to retrieve interpolated variables; {}",
                            e
                        ),
                    },
                    None,
                );

            self.mark_task_run_complete(&run, &new_task_run, &status_map)
                .await;
            return;
        };

        let env_vars: HashMap<String, String> = env_vars
            .into_iter()
            .map(|variable| (variable.key, variable.value))
            .collect();

        let container_name = fmt::task_container_id(
            &run.namespace,
            &run.pipeline,
            &run.id.to_string(),
            &new_task_run.id,
        );

        if let Err(e) = self
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
            new_task_run.set_finished_abnormal(
                task_run::Status::Failed,
                task_run::Failure {
                    kind: task_run::FailureKind::SchedulerError,
                    description: format!(
                        "task could not be run due to inability to be scheduled; {}",
                        e
                    ),
                },
                None,
            );

            self.mark_task_run_complete(&run, &new_task_run, &status_map)
                .await;
            return;
        };

        new_task_run.state = task_run::State::Running;
        if let Err(e) = self.storage.update_task_run_state(&new_task_run).await {
            error!("could not update task run"; "error" => format!("{:?}", e));
            return;
        }

        status_map.insert(
            new_task_run.task.id.clone(),
            (new_task_run.status.clone(), new_task_run.state.clone()),
        );

        // Block until task-run status can be logged
        self.clone()
            .monitor_task_run(container_name, &mut new_task_run)
            .await;

        self.mark_task_run_complete(&run, &new_task_run, &status_map)
            .await;
    }

    /// Creates all child task_runs for a given run. After creating all task runs it then
    /// blocks and monitors the run until it is finished.
    pub async fn execute_task_tree(self: Arc<Self>, run: run::Run) {
        let mut conn = match self
            .storage
            .conn()
            .await
            .map_err(|e| Status::internal(e.to_string()))
        {
            Ok(conn) => conn,
            Err(e) => {
                error!("could not get pipeline in order to run task tree"; "error" => format!("{:?}", e));
                return;
            }
        };

        let pipeline = match storage::pipelines::get(&mut conn, &run.namespace, &run.pipeline).await
        {
            Ok(pipeline) => pipeline,
            Err(e) => {
                error!("could not get pipeline in order to run task tree"; "error" => format!("{:?}", e));
                return;
            }
        };

        // TODO(clintjedwards): create token here.

        // TODO(clintjedwards): include notifiers here.

        let status_map: Arc<dashmap::DashMap<String, (task_run::Status, task_run::State)>> =
            Arc::new(DashMap::default());

        for task in pipeline.tasks.values() {
            tokio::spawn(self.clone().launch_task_run(
                run.clone(),
                task.clone(),
                status_map.clone(),
            ));
        }

        self.monitor_run_status(pipeline.tasks.len().try_into().unwrap(), run, status_map)
            .await;
    }

    pub async fn start_run_handler(
        self: Arc<Self>,
        args: StartRunRequest,
    ) -> Result<Response<StartRunResponse>, Status> {
        validate::arg(
            "namespace_id",
            args.namespace_id.clone(),
            vec![validate::is_valid_identifier, validate::not_empty_str],
        )?;
        validate::arg(
            "pipeline_id",
            args.pipeline_id.clone(),
            vec![validate::is_valid_identifier],
        )?;

        let mut conn = self
            .storage
            .conn()
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        let mut tx = conn
            .begin()
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        // Make sure the pipeline is ready to take new runs.
        let pipeline = storage::pipelines::get(&mut tx, &args.namespace_id, &args.pipeline_id)
            .await
            .map_err(|e| match e {
                storage::StorageError::NotFound => Status::not_found(format!(
                    "pipeline with id '{}' does not exist",
                    &args.pipeline_id
                )),
                _ => Status::internal(e.to_string()),
            })?;

        if pipeline.state != pipeline::State::Active {
            return Err(Status::failed_precondition(
                "could not create run; pipeline is not active",
            ));
        }

        while self
            .parallelism_limit_exceeded(&pipeline.namespace, &pipeline.id, pipeline.parallelism)
            .await
        {
            debug!("parallelism limit exceeded; waiting for runs to end before launching new run"; "limit" => pipeline.parallelism);
        }

        // Create the new run and retrieve it's ID.
        let mut new_run = run::Run::new(
            &pipeline.namespace,
            &pipeline.id,
            run::TriggerInfo {
                name: "manual".to_string(),
                label: "via_api".to_string(),
            },
            variables_to_vec(
                args.variables,
                VariableOwner::User,
                VariableSensitivity::Public,
            ),
        );

        let id = storage::runs::insert(&mut tx, &new_run)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        new_run.id = id;

        tx.commit()
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        // Publish that the run has started.
        let event_self = self.clone();
        let event_namespace = new_run.namespace.clone();
        let event_pipeline = new_run.pipeline.clone();
        let event_run = new_run.id;
        tokio::spawn(async move {
            event_self
                .event_bus
                .publish(event::Kind::StartedRun {
                    namespace_id: event_namespace,
                    pipeline_id: event_pipeline,
                    run_id: event_run,
                })
                .await
        });

        // Launch per-run clean up jobs.
        tokio::spawn(
            self.clone()
                .handle_run_object_expiry(new_run.namespace.clone(), new_run.pipeline.clone()),
        );
        tokio::spawn(
            self.clone()
                .handle_run_log_expiry(new_run.namespace.clone(), new_run.pipeline.clone()),
        );

        // Finally, launch the thread that will launch all the task runs for a job.
        tokio::spawn(self.execute_task_tree(new_run.clone()));

        Ok(Response::new(StartRunResponse {
            run: Some(new_run.into()),
        }))
    }
}
