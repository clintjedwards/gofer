use super::super::CliHarness;
use crate::cli::DEFAULT_NAMESPACE;
use serde::Serialize;
use std::process;

#[derive(Serialize)]
struct Data {
    created: String,
    description: String,
    health: String,
    id: String,
    last_run: String,
    name: String,
    state: String,
    store_keys: Vec<String>,
}

fn _print_pipeline_template(data: Data) {
    const TEMPLATE: &str = r#"[{id}] {name} :: {state}

{description}
{{- if recent_runs}}
📦 Recent Runs
  {{- for run in recent_runs}}
  • {run.id} :: {run.started} by trigger {run.trigger_name} ({run.trigger_kind}) :: {run.state_prefix} {run.lasted} :: {run.state}
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
{{- endfor}}
{{- if triggers }}
🗘 Attached Triggers:
  {{- for trigger in triggers}}
  ⟳ [{trigger.state}] {trigger.label} ({trigger.kind})
    {{- for event in trigger.events }}
    + {event.processed} | {event.details}
    {{- endfor}}
  {{- endfor}}
{{- endif}}
{{- if notifiers }}
🕪 Attached Notifiers:
  {{- for notifier in notifiers range}}
  🕩 {notifier.label} ({notifier.kind})
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
            eprintln!("Command failed; {}", e);
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
                eprintln!("Command failed; {}", e);
                process::exit(1);
            })
            .into_inner();

        let pipeline = response.pipeline.unwrap();
        println!("{:?}", pipeline);
    }
}
