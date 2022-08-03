use super::super::CliHarness;
use crate::cli::{
    epoch, humanize_absolute_duration, humanize_relative_duration, DEFAULT_NAMESPACE,
};
use colored::Colorize;
use futures::StreamExt;
use std::process;

#[derive(Debug, serde::Serialize)]
struct TaskData {
    name: String,
    depends_on: Vec<String>,
}

#[derive(Debug, serde::Serialize)]
struct RunData {
    id: String,
    started: String,
    ended: String,
    trigger_name: String,
    trigger_label: String,
    state: String,
    lasted: String,
    status: String,
}

#[derive(Debug, serde::Serialize)]
struct EventData {
    processed: String,
    details: String,
}

#[derive(Debug, serde::Serialize)]
struct TriggerData {
    label: String,
    name: String,
    state: String,
    events: Vec<EventData>,
}

#[derive(Debug, serde::Serialize)]
struct Data {
    namespace: String,
    pipeline_id: String,
    pipeline_name: String,
    created: String,
    description: String,
    health: String,
    last_run: String,
    state: String,
    store_keys: Vec<String>,
    recent_runs: Vec<RunData>,
    tasks: Vec<TaskData>,
    triggers: Vec<TriggerData>,
}

fn print_pipeline_template(data: Data) {
    const TEMPLATE: &str = r#"[{namespace}/{pipeline_id}] {pipeline_name} :: {state}

{description}
{{- if recent_runs}}

📦 Recent Runs
  {{- for run in recent_runs}}
  • #{run.id}: {run.started} by trigger {run.trigger_label} ({run.trigger_name}) :: {run.state} {run.lasted} :: {run.status}
  {{- endfor}}
{{- endif}}
{{- if tasks }}

🗒 Tasks:
  {{- for task in tasks}}
  • {task.name}
  {{- if task.depends_on -}}
    {{- for dependant in task.depends_on }}
      - {dependant}
    {{- endfor -}}
  {{- endif -}}
  {{- endfor -}}
{{- endif}}
{{- if store_keys}}

☁︎ Store keys: [{store_keys}]
{{- endif}}
{{- if triggers }}

🗘 Attached Triggers:
  {{- for trigger in triggers}}
  ⟳ [{trigger.state}] {trigger.label} ({trigger.name})
    {{- for event in trigger.events }}
    + {event.processed} | {event.details}
    {{- endfor}}
  {{- endfor}}
{{- endif}}

Created {created} | Last Run {last_run} | Health {health}"#;

    let mut template = tinytemplate::TinyTemplate::new();
    template.add_template("_", TEMPLATE).unwrap();
    println!("{}", template.render("_", &data).unwrap())
}

impl CliHarness {
    pub async fn pipeline_get(&self, id: &str) {
        let mut client = self.connect().await.unwrap_or_else(|e| {
            eprintln!("{} Command failed; {}", "x".red(), e);
            process::exit(1);
        });

        let request = tonic::Request::new(gofer_proto::GetPipelineRequest {
            namespace_id: self
                .config
                .namespace
                .clone()
                .unwrap_or_else(|| DEFAULT_NAMESPACE.to_string()),
            id: id.to_string(),
        });
        let response = client
            .get_pipeline(request)
            .await
            .unwrap_or_else(|e| {
                eprintln!("{} Command failed; {}", "x".red(), e.message());
                process::exit(1);
            })
            .into_inner();

        let pipeline: gofer_models::pipeline::Pipeline = response.pipeline.unwrap().into();

        let request = tonic::Request::new(gofer_proto::ListRunsRequest {
            namespace_id: self
                .config
                .namespace
                .clone()
                .unwrap_or_else(|| DEFAULT_NAMESPACE.to_string()),
            pipeline_id: pipeline.id.clone(),
            offset: 0,
            limit: 5,
        });
        let response = client
            .list_runs(request)
            .await
            .unwrap_or_else(|e| {
                eprintln!("{} Command failed; {}", "x".red(), e.message());
                process::exit(1);
            })
            .into_inner();

        let last_five_runs: Vec<gofer_models::run::Run> =
            response.runs.into_iter().map(|r| r.into()).collect();
        let mut recent_runs = vec![];

        for run in &last_five_runs {
            recent_runs.push(RunData {
                id: run.id.to_string(),
                started: humanize_relative_duration(run.started)
                    .unwrap_or_else(|| "Not yet".to_string()),
                ended: humanize_relative_duration(run.ended)
                    .unwrap_or_else(|| "Not yet".to_string()),
                trigger_name: run.trigger.clone().name,
                trigger_label: run.trigger.clone().label,
                state: run.state.to_string(),
                status: run.status.to_string(),
                lasted: humanize_absolute_duration(run.started, run.ended),
            })
        }

        let mut tasks = vec![];
        for task in pipeline.tasks.values() {
            tasks.push(TaskData {
                name: task.id.clone(),
                depends_on: task
                    .depends_on
                    .clone()
                    .into_iter()
                    .map(|(key, _)| key)
                    .collect(),
            });
        }

        let mut triggers = vec![];
        for trigger_settings in pipeline.triggers.values() {
            let request = tonic::Request::new(gofer_proto::GetTriggerRequest {
                name: trigger_settings.clone().name,
            });
            let response = client
                .get_trigger(request)
                .await
                .unwrap_or_else(|e| {
                    eprintln!("{} Command failed; {}", "x".red(), e.message());
                    process::exit(1);
                })
                .into_inner();

            if response.trigger.is_none() {
                continue;
            }

            let trigger: gofer_models::trigger::Trigger = response.trigger.unwrap().into();

            let request = tonic::Request::new(gofer_proto::ListEventsRequest {
                reverse: true,
                follow: false,
            });
            let response = client
                .list_events(request)
                .await
                .unwrap_or_else(|e| {
                    eprintln!("{} Command failed; {}", "x".red(), e.message());
                    process::exit(1);
                })
                .into_inner();

            let mut response = response.take(100);
            let mut events = vec![];

            while let Some(resp) = response.next().await {
                //TODO(we need to be able to translate events back into objects):
                let event = match resp {
                    Ok(resp) => resp.event.unwrap(),
                    Err(e) => {
                        eprintln!(
                            "{} Could not collect all events; {}",
                            "x".red(),
                            e.message()
                        );
                        break;
                    }
                };

                events.push(EventData {
                    processed: event.emitted.to_string(),
                    details: event.details,
                })
            }

            triggers.push(TriggerData {
                label: trigger_settings.label.clone(),
                name: trigger_settings.name.clone(),
                state: trigger.state.to_string(),
                events: vec![], //TODO(clintjedwards):
            });
        }

        let template_data = Data {
            namespace: pipeline.namespace,
            pipeline_id: pipeline.id,
            pipeline_name: pipeline.name,
            created: humanize_relative_duration(pipeline.created)
                .unwrap_or_else(|| "Unknown".to_string()),
            description: pipeline.description,
            last_run: {
                match last_five_runs.get(0) {
                    Some(run) => humanize_relative_duration(run.started)
                        .unwrap_or_else(|| "Never".to_string()),
                    None => "Never".to_string(),
                }
            },
            state: pipeline.state.to_string(),
            health: health(&last_five_runs, true),
            store_keys: pipeline.store_keys,
            recent_runs,
            tasks,
            triggers,
        };

        print_pipeline_template(template_data);
    }
}

fn health(runs: &Vec<gofer_models::run::Run>, emoji: bool) -> String {
    let mut failed = 0;
    let mut passed = 0;

    for run in runs {
        match run.status {
            gofer_models::run::Status::Failed => failed += 1,
            gofer_models::run::Status::Successful => passed += 1,
            _ => {}
        };
    }

    if failed > 0 && passed == 0 {
        if emoji {
            return format!("☔︎ {}", "Poor").red().to_string();
        }
        return "Poor".red().to_string();
    }

    if failed > 0 && passed > 0 {
        if emoji {
            return format!("☁︎ {}", "Unstable").yellow().to_string();
        }
        return "Unstable".yellow().to_string();
    }

    if emoji {
        return format!("☀︎ {}", "Good").green().to_string();
    }

    "Good".green().to_string()
}
