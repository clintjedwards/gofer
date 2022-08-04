mod state_machine;

use crate::api::{validate, Api};
use crate::storage;
use anyhow::Result;
use gofer_models::{event, pipeline, run, task, task_run};
use gofer_models::{Variable, VariableOwner, VariableSensitivity};
use gofer_proto::{
    CancelAllRunsRequest, CancelAllRunsResponse, CancelRunRequest, CancelRunResponse,
    GetRunRequest, GetRunResponse, ListRunsRequest, ListRunsResponse, RetryRunRequest,
    RetryRunResponse, Run, StartRunRequest, StartRunResponse,
};
use slog_scope::debug;
use sqlx::Acquire;
use state_machine::RunStateMachine;
use std::{collections::HashMap, sync::Arc};
use strum::{Display, EnumString};
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

/// Converts a HashMap of variables(usually supplied by the user) into a Vec of type Variable that
/// can be used throughout Gofer.
pub fn variables_to_vec(
    map: HashMap<String, String>,
    owner: VariableOwner,
    sensitivity: VariableSensitivity,
) -> Vec<Variable> {
    let mut variables = vec![];

    for (key, value) in map {
        variables.push(Variable {
            key,
            value,
            owner: owner.clone(),
            sensitivity: sensitivity.clone(),
        })
    }

    variables
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
/// There are many places a task_run could potentially get env vars from:
/// 1) Right before the task_run starts, from Gofer itself.
/// 2) At the time of run inception, either by the user manually or the trigger.
/// 3) From the pipeline's configuration file.
///
/// The order in which the env vars are stacked are as such:
/// 1) We first pass in the Gofer system specific envvars as these are the most replaceable on the totem pole.
/// 2) We pass in the task specific envvars defined by the user in the pipeline config.
/// 3) Lastly we pass in the run specific defined envvars. These are usually provided by either a trigger
/// or the user when they attempt to start a new run manually. Since these are the most likely to be
/// edited adhoc they are treated as the most important.
pub fn combine_variables(run: &run::Run, task: &task::Task) -> Vec<Variable> {
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

    let mut task_run_vars = system_injected_vars; // Gofer provided env vars first.
    task_run_vars.extend(task_vars); // then we vars that come from the pipeline config.
    task_run_vars.extend(run_vars); // then finally vars that come from the user or the trigger.

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
    pub async fn cancel_run(
        &self,
        namespace_id: String,
        pipeline_id: String,
        run_id: u64,
    ) -> Result<()> {
        let mut conn = self
            .storage
            .conn()
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        let task_runs =
            storage::task_runs::list(&mut conn, 0, 0, &namespace_id, &pipeline_id, run_id).await?;

        let mut timeout = self.conf.general.task_run_stop_timeout;
        if timeout == 0 {
            timeout = 604800
        }

        let mut cancelled_task_runs: Vec<task_run::TaskRun> = vec![];

        for task_run in task_runs {
            if self
                .cancel_task_run(&namespace_id, &pipeline_id, run_id, &task_run.id, timeout)
                .await
                .is_ok()
            {
                cancelled_task_runs.push(task_run)
            }
        }

        // wait for the run to be marked complete due to the failures in task_runs.
        loop {
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            let run = storage::runs::get(&mut conn, &namespace_id, &pipeline_id, run_id).await?;

            if run.state != run::State::Complete {
                continue;
            }

            for task_run in cancelled_task_runs {
                let _ = storage::task_runs::update(
                    &mut conn,
                    &task_run,
                    storage::task_runs::UpdatableFields {
                        status: Some(task_run::Status::Cancelled),
                        ..Default::default()
                    },
                )
                .await;
            }

            break;
        }

        Ok(())
    }
}

impl Api {
    pub async fn get_run_handler(
        &self,
        args: GetRunRequest,
    ) -> Result<Response<GetRunResponse>, Status> {
        validate::arg(
            "namespace_id",
            args.namespace_id.clone(),
            vec![validate::is_valid_identifier, validate::not_empty_str],
        )?;
        validate::arg(
            "pipeline_id",
            args.pipeline_id.clone(),
            vec![validate::is_valid_identifier, validate::not_empty_str],
        )?;

        if args.id == 0 {
            return Err(Status::failed_precondition("must include target run id"));
        }

        let mut conn = self
            .storage
            .conn()
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        storage::runs::get(&mut conn, &args.namespace_id, &args.pipeline_id, args.id)
            .await
            .map(|run| {
                Response::new(GetRunResponse {
                    run: Some(run.into()),
                })
            })
            .map_err(|e| match e {
                storage::StorageError::NotFound => {
                    Status::not_found(format!("run with id '{}' does not exist", &args.id))
                }
                _ => Status::internal(e.to_string()),
            })
    }

    pub async fn list_runs_handler(
        &self,
        args: ListRunsRequest,
    ) -> Result<Response<ListRunsResponse>, Status> {
        validate::arg(
            "namespace_id",
            args.namespace_id.clone(),
            vec![validate::is_valid_identifier, validate::not_empty_str],
        )?;
        validate::arg(
            "pipeline_id",
            args.pipeline_id.clone(),
            vec![validate::is_valid_identifier, validate::not_empty_str],
        )?;

        let mut conn = self
            .storage
            .conn()
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        storage::runs::list(
            &mut conn,
            args.offset as u64,
            args.limit as u64,
            &args.namespace_id,
            &args.pipeline_id,
        )
        .await
        .map(|runs| {
            Response::new(ListRunsResponse {
                runs: runs.into_iter().map(Run::from).collect(),
            })
        })
        .map_err(|e| Status::internal(e.to_string()))
    }

    pub async fn retry_run_handler(
        self: Arc<Self>,
        args: RetryRunRequest,
    ) -> Result<Response<RetryRunResponse>, Status> {
        validate::arg(
            "namespace_id",
            args.namespace_id.clone(),
            vec![validate::is_valid_identifier, validate::not_empty_str],
        )?;
        validate::arg(
            "pipeline_id",
            args.pipeline_id.clone(),
            vec![validate::is_valid_identifier, validate::not_empty_str],
        )?;
        validate::arg("run_id", args.run_id, vec![validate::not_zero_num])?;

        let mut conn = self
            .storage
            .conn()
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        let run = storage::runs::get(
            &mut conn,
            &args.namespace_id,
            &args.pipeline_id,
            args.run_id,
        )
        .await
        .map_err(|e| match e {
            storage::StorageError::NotFound => {
                Status::not_found(format!("run with id '{}' does not exist", &args.run_id))
            }
            _ => Status::internal(e.to_string()),
        })?;

        let resp = self
            .start_run_handler(StartRunRequest {
                namespace_id: run.namespace,
                pipeline_id: run.pipeline,
                variables: run
                    .variables
                    .into_iter()
                    .map(|variable| (variable.key, variable.value))
                    .collect(),
            })
            .await?;

        Ok(Response::new(RetryRunResponse {
            run: Some(resp.into_inner().run.unwrap()),
        }))
    }
    pub async fn cancel_run_handler(
        self: Arc<Self>,
        args: CancelRunRequest,
    ) -> Result<Response<CancelRunResponse>, Status> {
        validate::arg(
            "namespace_id",
            args.namespace_id.clone(),
            vec![validate::is_valid_identifier, validate::not_empty_str],
        )?;
        validate::arg(
            "pipeline_id",
            args.pipeline_id.clone(),
            vec![validate::is_valid_identifier, validate::not_empty_str],
        )?;
        validate::arg("run_id", args.run_id, vec![validate::not_zero_num])?;

        tokio::spawn(async move {
            self.cancel_run(
                args.namespace_id.clone(),
                args.pipeline_id.clone(),
                args.run_id,
            )
            .await
        });

        Ok(Response::new(CancelRunResponse {}))
    }
    pub async fn cancel_all_runs_handler(
        self: Arc<Self>,
        args: CancelAllRunsRequest,
    ) -> Result<Response<CancelAllRunsResponse>, Status> {
        validate::arg(
            "namespace_id",
            args.namespace_id.clone(),
            vec![validate::is_valid_identifier, validate::not_empty_str],
        )?;
        validate::arg(
            "pipeline_id",
            args.pipeline_id.clone(),
            vec![validate::is_valid_identifier, validate::not_empty_str],
        )?;

        let mut conn = self
            .storage
            .conn()
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        let run = storage::runs::list(&mut conn, 0, 0, &args.namespace_id, &args.pipeline_id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        let runs_in_progress: Vec<run::Run> = run
            .into_iter()
            .filter(|run| run.state != run::State::Complete)
            .collect();

        for run in &runs_in_progress {
            let self_clone = self.clone();
            let namespace_id = args.namespace_id.clone();
            let pipeline_id = args.pipeline_id.clone();
            let run_id = run.id;

            tokio::spawn(async move {
                self_clone
                    .cancel_run(namespace_id, pipeline_id, run_id)
                    .await
            });
        }

        Ok(Response::new(CancelAllRunsResponse {
            runs: runs_in_progress.into_iter().map(|run| run.id).collect(),
        }))
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

        // Create a transaction to make sure there is no race condition between
        // pipeline state and the ability to create a new run.
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

        // Create the new run and retrieve it's ID.
        let mut new_run = run::Run::new(
            &pipeline.namespace,
            &pipeline.id,
            run::TriggerInfo {
                name: "manual".to_string(),
                label: "api".to_string(),
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

        let run_state_machine = RunStateMachine::new(self, pipeline.clone(), new_run.clone()).await;

        // Make sure the pipeline is ready for a new run.
        while run_state_machine
            .parallelism_limit_exceeded(&mut conn)
            .await
        {
            debug!("parallelism limit exceeded; waiting for runs to end before launching new run"; "run" => &new_run.id, "limit" => &pipeline.parallelism);
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }

        // Finally, launch the thread that will launch all the task runs for a job.
        tokio::spawn(run_state_machine.execute_task_tree());

        Ok(Response::new(StartRunResponse {
            run: Some(new_run.into()),
        }))
    }
}
