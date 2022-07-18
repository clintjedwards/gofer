use super::super::CliHarness;
use crate::cli::{humanize_duration, DEFAULT_NAMESPACE};
use comfy_table::{presets::ASCII_MARKDOWN, Cell, CellAlignment, Color, ContentArrangement};
use std::process;

impl CliHarness {
    pub async fn pipeline_list(&self) {
        let mut client = self.connect().await.unwrap_or_else(|e| {
            eprintln!("Command failed; {}", e);
            process::exit(1);
        });

        let request = tonic::Request::new(gofer_proto::ListPipelinesRequest {
            namespace_id: self
                .config
                .namespace
                .clone()
                .unwrap_or_else(|| DEFAULT_NAMESPACE.to_string()),
            offset: 0,
            limit: 0,
        });
        let response = client
            .list_pipelines(request)
            .await
            .unwrap_or_else(|e| {
                eprintln!("Command failed; {}", e);
                process::exit(1);
            })
            .into_inner();

        let mut table = comfy_table::Table::new();
        table
            .load_preset(ASCII_MARKDOWN)
            .set_content_arrangement(ContentArrangement::Dynamic)
            .set_header(vec![
                Cell::new("id")
                    .set_alignment(CellAlignment::Center)
                    .fg(Color::Blue),
                Cell::new("name")
                    .set_alignment(CellAlignment::Center)
                    .fg(Color::Blue),
                Cell::new("description")
                    .set_alignment(CellAlignment::Center)
                    .fg(Color::Blue),
                Cell::new("last run")
                    .set_alignment(CellAlignment::Center)
                    .fg(Color::Blue),
                Cell::new("state")
                    .set_alignment(CellAlignment::Center)
                    .fg(Color::Blue),
                Cell::new("created")
                    .set_alignment(CellAlignment::Center)
                    .fg(Color::Blue),
            ]);

        for pipeline in response.pipelines {
            table.add_row(vec![
                Cell::new(pipeline.id).fg(Color::Green),
                Cell::new(pipeline.name),
                Cell::new(pipeline.description),
                Cell::new(humanize_duration(pipeline.last_run_time as i64)),
                Cell::new({
                    let state =
                        gofer_proto::pipeline::PipelineState::from_i32(pipeline.state).unwrap();
                    gofer_models::pipeline::PipelineState::from(state).to_string()
                }),
                Cell::new(humanize_duration(pipeline.created as i64)),
            ]);
        }

        println!("{table}",);
    }
}
