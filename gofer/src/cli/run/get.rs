use super::CliHarness;
use crate::cli::{humanize_absolute_duration, humanize_relative_duration, DEFAULT_NAMESPACE};
use colored::Colorize;
use std::process;

#[derive(Debug, serde::Serialize)]
struct Data {
    id: u64,
    namespace_id: String,
    pipeline_id: String,
    status: String,
    state: String,
    trigger_label: String,
    trigger_name: String,
    started: String,
    duration: String,
    task_runs: Vec<TaskRunData>,
    store_keys: Vec<StoreKeyData>,
}

#[derive(Debug, serde::Serialize)]
struct TaskRunData {
    id: String,
    started: String,
    duration: String,
    state: String,
    status: String,
}

#[derive(Debug, serde::Serialize)]
struct StoreKeyData {
    key: String,
    expired: bool,
}

fn print_run_template(data: Data) {
    const TEMPLATE: &str = r#"Run #{id} [{namespace_id}/{pipeline_id}] :: {status} | {state}

Triggered via {trigger_label} ({trigger_name}) {started} and ran for {duration}

🗒 Task Runs:
  {{- for task_run in task_runs}}
  • {task_run.id} :: Started {task_run.started} :: {task_run.duration} :: {task_run.status} | {task_run.state}
  {{- endfor -}}
{{- if store_keys}}

☁︎ Store keys: [{store_keys}]
{{- endif}}"#;

    let mut template = tinytemplate::TinyTemplate::new();
    template.add_template("_", TEMPLATE).unwrap();
    println!("{}", template.render("_", &data).unwrap())
}

impl CliHarness {
    pub async fn run_get(&self, pipeline_id: String, id: u64) {
        let mut client = self.connect().await.unwrap_or_else(|e| {
            eprintln!("{} Command failed; {}", "x".red(), e);
            process::exit(1);
        });

        let request = tonic::Request::new(gofer_proto::GetRunRequest {
            namespace_id: self
                .config
                .namespace
                .clone()
                .unwrap_or_else(|| DEFAULT_NAMESPACE.to_string()),
            pipeline_id: pipeline_id.to_string(),
            id,
        });
        let run = client
            .get_run(request)
            .await
            .unwrap_or_else(|e| {
                eprintln!("{} Command failed; {}", "x".red(), e.message());
                process::exit(1);
            })
            .into_inner()
            .run
            .unwrap_or_else(|| {
                eprintln!("{} Command failed; could not get run", "x".red());
                process::exit(1);
            });

        let run: gofer_models::run::Run = run.into();

        let request = tonic::Request::new(gofer_proto::ListTaskRunsRequest {
            namespace_id: self
                .config
                .namespace
                .clone()
                .unwrap_or_else(|| DEFAULT_NAMESPACE.to_string()),
            pipeline_id: pipeline_id.to_string(),
            run_id: id,
        });
        let task_runs = client
            .list_task_runs(request)
            .await
            .unwrap_or_else(|e| {
                eprintln!("{} Command failed; {}", "x".red(), e.message());
                process::exit(1);
            })
            .into_inner()
            .task_runs;

        let task_runs: Vec<TaskRunData> = task_runs
            .into_iter()
            .map(|task_run| {
                let task_run: gofer_models::task_run::TaskRun = task_run.into();
                TaskRunData {
                    id: task_run.id,
                    started: humanize_relative_duration(task_run.started)
                        .unwrap_or_else(|| "Not yet".to_string()),
                    duration: humanize_absolute_duration(task_run.started, task_run.ended),
                    state: task_run.state.to_string(),
                    status: task_run.status.to_string(),
                }
            })
            .collect();

        let template_data = Data {
            id: run.id,
            namespace_id: run.namespace,
            pipeline_id: run.pipeline,
            status: run.status.to_string(),
            state: run.state.to_string(),
            trigger_label: run.trigger.label,
            trigger_name: run.trigger.name,
            started: humanize_relative_duration(run.started)
                .unwrap_or_else(|| "Not yet".to_string()),
            duration: humanize_absolute_duration(run.started, run.ended),
            task_runs,
            store_keys: vec![],
        };

        print_run_template(template_data);
    }
}
