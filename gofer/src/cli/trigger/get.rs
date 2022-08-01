use super::super::CliHarness;
use serde::Serialize;
use std::process;

#[derive(Serialize)]
struct Data {
    name: String,
    state: String,
    image: String,
    started: String,
    url: String,
    documentation: String,
}

impl From<gofer_proto::Trigger> for Data {
    fn from(v: gofer_proto::Trigger) -> Self {
        Self {
            name: v.name,
            state: {
                let state = gofer_proto::trigger::TriggerState::from_i32(v.state).unwrap();
                gofer_models::trigger::State::from(state).to_string()
            },
            image: v.image,
            started: super::super::humanize_relative_duration(v.started)
                .unwrap_or_else(|| "Never".to_string()),
            url: v.url,
            documentation: v.documentation,
        }
    }
}

fn print_trigger_template(data: Data) {
    const TEMPLATE: &str = r#"Trigger '{name}' :: {state} :: {image}

Started {started}

Endpoint: {url}

{{- if documentation}}
Documentation: {documentation}
{{- else -}}
No Documentation found
{{- endif}}"#;

    let mut template = tinytemplate::TinyTemplate::new();
    template.add_template("_", TEMPLATE).unwrap();
    println!("{}", template.render("_", &data).unwrap())
}

impl CliHarness {
    pub async fn trigger_get(&self, name: &str) {
        let mut client = match self.connect().await {
            Ok(client) => client,
            Err(e) => {
                eprintln!("Command failed; {}", e);
                process::exit(1);
            }
        };

        let request = tonic::Request::new(gofer_proto::GetTriggerRequest {
            name: name.to_string(),
        });
        let response = match client.get_trigger(request).await {
            Ok(response) => response.into_inner(),
            Err(e) => {
                eprintln!("Command failed; {}", e.message());
                process::exit(1);
            }
        };

        let trigger = response.trigger.unwrap();
        print_trigger_template(trigger.into());
    }
}
