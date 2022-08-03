use super::super::CliHarness;
use crate::cli::{humanize_relative_duration, DEFAULT_NAMESPACE};
use colored::Colorize;
use comfy_table::{presets::ASCII_MARKDOWN, Cell, CellAlignment, Color, ContentArrangement};
use std::process;

impl CliHarness {
    pub async fn pipeline_list(&self) {
        let mut client = self.connect().await.unwrap_or_else(|e| {
            eprintln!("{} Command failed; {}", "x".red(), e);
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
                eprintln!("{} Command failed; {}", "x".red(), e);
                process::exit(1);
            })
            .into_inner();

        if response.pipelines.is_empty() {
            println!("No pipelines found.");
            return;
        }

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

        for mut pipeline in response.pipelines {
            let request = tonic::Request::new(gofer_proto::ListRunsRequest {
                namespace_id: self
                    .config
                    .namespace
                    .clone()
                    .unwrap_or_else(|| DEFAULT_NAMESPACE.to_string()),
                pipeline_id: pipeline.id.clone(),
                offset: 0,
                limit: 1,
            });

            let response = client
                .list_runs(request)
                .await
                .unwrap_or_else(|e| {
                    eprintln!("{} Command failed; {}", "x".red(), e);
                    process::exit(1);
                })
                .into_inner();

            let last_run_time = match response.runs.get(0) {
                Some(last_run) => last_run.started,
                None => 0,
            };

            table.add_row(vec![
                Cell::new(pipeline.id).fg(Color::Green),
                Cell::new(pipeline.name),
                Cell::new({
                    pipeline.description.truncate(60);
                    pipeline.description
                }),
                Cell::new(
                    humanize_relative_duration(last_run_time)
                        .unwrap_or_else(|| "Never".to_string()),
                ),
                Cell::new({
                    let state =
                        gofer_proto::pipeline::PipelineState::from_i32(pipeline.state).unwrap();
                    gofer_models::pipeline::State::from(state).to_string()
                }),
                Cell::new(
                    humanize_relative_duration(pipeline.created)
                        .unwrap_or_else(|| "Unknown".to_string()),
                ),
            ]);
        }

        println!("{table}",);
    }
}
