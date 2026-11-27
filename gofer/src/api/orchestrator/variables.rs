use super::run_specific_api_key_id;
use crate::{
    api::{Variable, VariableSource, runs, tasks},
    conf,
};

use gofer_sdk::config::pipeline_secret;
use std::collections::HashMap;

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
                value: task.image.digest.clone(),
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
}
