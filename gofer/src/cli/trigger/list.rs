use super::super::CliHarness;
use comfy_table::{presets::ASCII_MARKDOWN, Cell, CellAlignment, Color, ContentArrangement};
use std::process;

impl CliHarness {
    pub async fn trigger_list(&self) {
        let mut client = match self.connect().await {
            Ok(client) => client,
            Err(e) => {
                eprintln!("Command failed; {:?}", e);
                process::exit(1);
            }
        };

        let request = tonic::Request::new(gofer_proto::ListTriggersRequest {});
        let response = match client.list_triggers(request).await {
            Ok(response) => response.into_inner(),
            Err(e) => {
                eprintln!("Command failed; {:?}", e);
                process::exit(1);
            }
        };

        let mut table = comfy_table::Table::new();
        table
            .load_preset(ASCII_MARKDOWN)
            .set_content_arrangement(ContentArrangement::Dynamic)
            .set_header(vec![
                Cell::new("name")
                    .set_alignment(CellAlignment::Center)
                    .fg(Color::Blue),
                Cell::new("image")
                    .set_alignment(CellAlignment::Center)
                    .fg(Color::Blue),
                Cell::new("url")
                    .set_alignment(CellAlignment::Center)
                    .fg(Color::Blue),
                Cell::new("state")
                    .set_alignment(CellAlignment::Center)
                    .fg(Color::Blue),
                Cell::new("documentation")
                    .set_alignment(CellAlignment::Center)
                    .fg(Color::Blue),
            ]);

        for trigger in response.triggers {
            table.add_row(vec![
                Cell::new(trigger.name).fg(Color::Green),
                Cell::new(trigger.image),
                Cell::new(trigger.url),
                Cell::new({
                    let state =
                        gofer_proto::trigger::TriggerState::from_i32(trigger.state).unwrap();
                    gofer_models::trigger::State::from(state).to_string()
                }),
                Cell::new(trigger.documentation),
            ]);
        }

        println!("{table}");
    }
}
