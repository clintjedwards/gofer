use crate::{
    api::{
        fmt_pipeline_object_key, fmt_run_object_key, fmt_secret_key, fmt_task_container_id, Api,
    },
    object_store, scheduler, secret_store, storage,
};
use anyhow::Result;
use futures::Future;
use gofer_models::task::RequiredParentStatus;
use gofer_models::task_run::{Failure, FailureKind, State, Status};
use gofer_models::{Variable, VariableOwner, VariableSensitivity};
use slog_scope::{debug, error};
use std::collections::HashMap;
use std::sync::Arc;
use strum::{Display, EnumString};

/// Gofer allows users to enter special interpolation strings such that
/// special functionality is substituted when Gofer reads these strings
/// in a user's pipeline configuration.
#[derive(Debug, Display, EnumString)]
pub enum InterpolationKind {
    Unknown,
    /// secret{{<key>}}
    Secret,
    /// pipeline{{<key>}}
    Pipeline,
    /// run{{<key>}}
    Run,
}

type StatusMap = dashmap::DashMap<String, (Status, State)>;

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

        let runs = match self
            .storage
            .list_runs(0, 0, namespace_id, pipeline_id)
            .await
        {
            Ok(runs) => runs,
            Err(_) => return true,
        };

        let mut runs_in_progress = 0;
        for run in runs {
            if run.state != gofer_models::run::State::Complete {
                runs_in_progress += 1;
            }
        }

        if runs_in_progress >= limit {
            return true;
        }

        false
    }

    pub async fn launch_task_run(
        &self,
        namespace: String,
        pipeline: String,
        run: gofer_models::run::Run,
        task: gofer_models::task::Task,
        mut status_map: StatusMap,
    ) -> impl Future<Output = ()> {
        let event_bus = self.event_bus.clone();
        let storage = self.storage.clone();
        let object_store = self.object_store.clone();
        let secret_store = self.secret_store.clone();
        let scheduler = self.scheduler.clone();

        async move {
            let mut new_task_run =
                gofer_models::task_run::TaskRun::new(&namespace, &pipeline, run.id, task.clone());

            event_bus
                .publish(gofer_models::event::Kind::StartedTaskRun {
                    namespace_id: namespace.clone(),
                    pipeline_id: pipeline.clone(),
                    run_id: run.id,
                    task_run_id: task.id.clone(),
                })
                .await;

            new_task_run.state = gofer_models::task_run::State::Processing;
            new_task_run.status = gofer_models::task_run::Status::Unknown;

            if let Err(e) = storage.create_task_run(&new_task_run).await {
                error!("could not add task run to storage"; "error" => format!("{:?}", e));
                return;
            }

            status_map.insert(
                new_task_run.task.id.clone(),
                (new_task_run.status.clone(), new_task_run.state.clone()),
            );

            // These environment variables are present on every task run.
            let gofer_run_injected_vars = HashMap::from([
                (
                    "GOFER_PIPELINE_ID".to_string(),
                    gofer_models::Variable {
                        key: "GOFER_PIPELINE_ID".to_string(),
                        value: pipeline.clone(),
                        owner: gofer_models::VariableOwner::System,
                        sensitivity: gofer_models::VariableSensitivity::Public,
                    },
                ),
                (
                    "GOFER_RUN_ID".to_string(),
                    gofer_models::Variable {
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
            ]);

            let task_vars: HashMap<String, gofer_models::Variable> = task
                .variables
                .into_iter()
                .map(|variable| (variable.key.to_uppercase(), variable))
                .collect();

            let run_vars: HashMap<String, gofer_models::Variable> = run
                .variables
                .into_iter()
                .map(|variable| (variable.key.to_uppercase(), variable))
                .collect();

            // We need to combine the environment variables we get from multiple sources in order to pass them
            // finally to the task run. The order in which they are passed is very important as they can and should
            // overwrite each other, even though the intention of prefixing the environment variables is to prevent
            // the chance of overwriting. The order in which they are passed into the extend function
            // determines the priority in reverse order. Last in the stack will overwrite any conflicts from the others.
            //
            // 1) We first pass in the Gofer specific envvars as these are the most replaceable on the totem pole.
            // 2) We pass in the task specific envvars defined by the user in the pipeline config.
            // 3) Lastly we pass in the trigger's defined envvars, these are the most variable and most important since
            // they map back to the user's intent for a specific run.
            let mut task_run_vars = gofer_run_injected_vars;
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

            new_task_run.variables = task_run_vars.into_iter().map(|(_, value)| value).collect();

            if let Err(e) = storage.update_task_run(&new_task_run).await {
                error!("could not update task run"; "error" => format!("{:?}", e));
                return;
            }

            // First we need to make sure all the parents of the current task are in a finished state.
            while !parent_tasks_finished(&status_map, &new_task_run.task.depends_on) {
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            }

            // Then check to make sure that the parents all finished in the required states. If not
            // we'll have to mark this task as cancelled.
            if let Err(e) = task_dependencies_satisfied(&status_map, &new_task_run.task.depends_on)
            {
                new_task_run.set_finished_abnormal(
                    Status::Skipped,
                    Failure {
                        kind: FailureKind::FailedPrecondition,
                        description: format!(
                            "task could not be run due to unmet dependencies; {}",
                            e
                        ),
                    },
                    None,
                );

                if let Err(e) = storage.update_task_run(&new_task_run).await {
                    error!("could not update task run"; "error" => format!("{:?}", e));
                    return;
                }

                status_map.insert(
                    new_task_run.task.id.clone(),
                    (new_task_run.status.clone(), new_task_run.state.clone()),
                );

                event_bus
                    .publish(gofer_models::event::Kind::CompletedTaskRun {
                        namespace_id: namespace.clone(),
                        pipeline_id: pipeline.clone(),
                        run_id: run.id,
                        task_run_id: task.id.clone(),
                        status: new_task_run.status.clone(),
                    })
                    .await;

                return;
            }

            let mut env_vars = new_task_run.variables.clone();

            // After this point we're sure the task is in a state to be run. So we attempt to
            // contact the scheduler and start the container.

            // First we attempt to find any object/secret store variables and replace them
            // with the correct var.
            if let Err(e) = interpolate_vars(
                object_store,
                secret_store,
                namespace.clone(),
                pipeline.clone(),
                run.id,
                &mut env_vars,
            )
            .await
            {
                new_task_run.set_finished_abnormal(
                    Status::Skipped,
                    Failure {
                        kind: FailureKind::FailedPrecondition,
                        description: format!(
                            "task could not be run due to unmet dependencies; {}",
                            e
                        ),
                    },
                    None,
                );

                if let Err(e) = storage.update_task_run(&new_task_run).await {
                    error!("could not update task run"; "error" => format!("{:?}", e));
                    return;
                }

                status_map.insert(
                    new_task_run.task.id.clone(),
                    (new_task_run.status.clone(), new_task_run.state.clone()),
                );

                event_bus
                    .publish(gofer_models::event::Kind::CompletedTaskRun {
                        namespace_id: namespace.clone(),
                        pipeline_id: pipeline.clone(),
                        run_id: run.id,
                        task_run_id: task.id.clone(),
                        status: new_task_run.status.clone(),
                    })
                    .await;

                return;
            };

            let env_vars: HashMap<String, String> = env_vars
                .into_iter()
                .map(|variable| (variable.key, variable.value))
                .collect();

            // scheduler
            //     .start_container(scheduler::StartContainerRequest {
            //         name: fmt_task_container_id(
            //             &namespace,
            //             &pipeline,
            //             &run.id.to_string(),
            //             &new_task_run.id,
            //         ),
            //         image: new_task_run.task.image,
            //         variables: env_vars,
            //         registry_auth: {
            //             if let Some(mut auth) = new_task_run.task.registry_auth {
            //                 if let Some(secret) =
            //                     parse_interpolation_syntax(InterpolationKind::Secret, &auth.pass)
            //                 {
            //                     auth.pass = secret;
            //                 };

            //                 Some(auth.into())
            //             } else {
            //                 None
            //             }
            //         },
            //     })
            //     .await;

            todo!();
        }
    }

    /// Removes run level object_store objects once a run is past it's expiry threshold.
    pub async fn handle_run_object_expiry(
        &self,
        namespace_id: String,
        pipeline_id: String,
    ) -> impl Future<Output = ()> {
        let storage = self.storage.clone();
        let object_store = self.object_store.clone();
        let run_object_expiry = self.conf.object_store.run_object_expiry;
        async move {
            let limit = run_object_expiry;

            let runs = match storage.list_runs(0, 0, &namespace_id, &pipeline_id).await {
                Ok(runs) => runs,
                Err(e) => {
                    error!("could not get runs for run expiry processing"; "error" => format!("{:?}", e));
                    return;
                }
            };

            if runs.len() < limit.try_into().unwrap() {
                return;
            }

            for mut run in runs.into_iter().rev() {
                if run.state != gofer_models::run::State::Complete {
                    continue;
                };

                if run.store_info.is_none() {
                    break;
                };

                let mut store_info = run.store_info.unwrap();

                if store_info.is_expired {
                    break;
                };

                for object_key in &store_info.keys {
                    match object_store.delete_object(object_key).await {
                        Ok(_) => {}
                        Err(e) => {
                            error!("could not delete run object for expiry processing"; "error" => format!("{:?}", e))
                        }
                    }
                }

                store_info.is_expired = true;

                run.store_info = Some(store_info.clone());
                match storage.update_run(&run).await {
                    Ok(_) => {}
                    Err(e) => {
                        error!("could not not update run for expiry processing"; "error" => format!("{:?}", e))
                    }
                }

                debug!("old run objects removed";
                    "run_age_limit" => limit,
                    "run_id" => run.id,
                    "removed_objects" => format!("{:?}", store_info.keys),
                );
                return;
            }
        }
    }
}

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

pub async fn interpolate_vars(
    object_store: Arc<dyn object_store::Store + Sync + Send>,
    secret_store: Arc<dyn secret_store::Store + Sync + Send>,
    namespace: String,
    pipeline: String,
    run: u64,
    variables: &mut Vec<Variable>,
) -> Result<()> {
    for variable in variables {
        if let Some(secret_key) =
            parse_interpolation_syntax(InterpolationKind::Secret, &variable.value)
        {
            let secret = match secret_store
                .get_secret(&fmt_secret_key(&namespace, &pipeline, &secret_key))
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
            let pipeline = match object_store
                .get_object(&fmt_pipeline_object_key(
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

        if let Some(run_key) = parse_interpolation_syntax(InterpolationKind::Run, &variable.value) {
            let run = match object_store
                .get_object(&fmt_run_object_key(&namespace, &pipeline, run, &run_key))
                .await
            {
                Ok(run) => run,
                Err(e) => return Err(anyhow::anyhow!("could not get run '{}'; {}", run_key, e)),
            };

            variable.value = String::from_utf8_lossy(&run).to_string();
        };
    }

    Ok(())
}

pub async fn execute_task_tree(storage: storage::Db, run: gofer_models::run::Run) {
    let pipeline = match storage.get_pipeline(&run.namespace, &run.pipeline).await {
        Ok(pipeline) => pipeline,
        Err(e) => {
            error!("could not get pipeline in order to run task tree"; "error" => format!("{:?}", e));
            return;
        }
    };

    // TODO(clintjedwards): create token here.

    // TODO(clintjedwards): include notifiers here.

    for task in pipeline.tasks {}
}

fn parent_tasks_finished(
    status_map: &StatusMap,
    dependencies: &HashMap<String, gofer_models::task::RequiredParentStatus>,
) -> bool {
    for parent in dependencies.keys() {
        if let Some(status_entry) = status_map.get(parent) {
            if status_entry.1 != gofer_models::task_run::State::Complete {
                return false;
            }
        } else {
            return false;
        }
    }

    true
}

fn task_dependencies_satisfied(
    status_map: &StatusMap,
    dependencies: &HashMap<String, gofer_models::task::RequiredParentStatus>,
) -> anyhow::Result<()> {
    for (parent, required_status) in dependencies {
        if let Some(status_entry) = status_map.get(parent) {
            match required_status {
                gofer_models::task::RequiredParentStatus::Unknown => {
                    return Err(anyhow::anyhow!(
                        "A parent dependency should never be in the state 'Unknown'"
                    ));
                }
                gofer_models::task::RequiredParentStatus::Any => {
                    if status_entry.0 != Status::Successful
                        && status_entry.0 != Status::Failed
                        && status_entry.0 != Status::Skipped
                    {
                        return Err(anyhow::anyhow!(
                            "parent '{parent}' is in incorrect state '{}'
                            for required 'any' dependency",
                            status_entry.0
                        ));
                    }
                }
                RequiredParentStatus::Success => {
                    if status_entry.0 != Status::Successful {
                        return Err(anyhow::anyhow!(
                            "parent '{parent}' is in incorrect state '{}';
                            task requires it to be in state '{}'",
                            status_entry.0,
                            Status::Successful.to_string()
                        ));
                    }
                }
                RequiredParentStatus::Failure => {
                    if status_entry.0 != Status::Failed {
                        return Err(anyhow::anyhow!(
                            "parent '{parent}' is in incorrect state '{}';
                            task requires it to be in state '{}'",
                            status_entry.0,
                            Status::Failed.to_string()
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

pub async fn handle_run_log_expiry() {
    debug!("handle run log expiry not implemented");
}

pub fn map_to_variables(
    map: HashMap<String, String>,
    owner: gofer_models::VariableOwner,
    sensitivity: gofer_models::VariableSensitivity,
) -> Vec<gofer_models::Variable> {
    let mut variables = vec![];

    for (key, value) in map {
        variables.push(gofer_models::Variable {
            key,
            value,
            owner: owner.clone(),
            sensitivity: sensitivity.clone(),
        })
    }

    variables
}
