use super::super::CliHarness;
use crate::cli::{humanize_absolute_duration, humanize_relative_duration, DEFAULT_NAMESPACE};
use colored::Colorize;
use comfy_table::{presets::ASCII_MARKDOWN, Cell, CellAlignment, Color, ContentArrangement};
use std::process;

impl CliHarness {
    pub async fn run_list(&self, pipeline_id: String) {
        let mut client = self.connect().await.unwrap_or_else(|e| {
            eprintln!("{} Command failed; {}", "x".red(), e);
            process::exit(1);
        });

        let request = tonic::Request::new(gofer_proto::ListRunsRequest {
            offset: 0,
            limit: 0,
            namespace_id: self
                .config
                .namespace
                .clone()
                .unwrap_or_else(|| DEFAULT_NAMESPACE.to_string()),
            pipeline_id: pipeline_id.to_string(),
        });
        let response = client
            .list_runs(request)
            .await
            .unwrap_or_else(|e| {
                eprintln!("{} Command failed; {}", "x".red(), e.message());
                process::exit(1);
            })
            .into_inner();
        if response.runs.is_empty() {
            println!("No runs found.");
            return;
        }

        let runs: Vec<gofer_models::run::Run> =
            response.runs.into_iter().map(|run| run.into()).collect();

        let mut table = comfy_table::Table::new();
        table
            .load_preset(ASCII_MARKDOWN)
            .set_content_arrangement(ContentArrangement::Dynamic)
            .set_header(vec![
                Cell::new("id")
                    .set_alignment(CellAlignment::Center)
                    .fg(Color::Blue),
                Cell::new("started")
                    .set_alignment(CellAlignment::Center)
                    .fg(Color::Blue),
                Cell::new("ended")
                    .set_alignment(CellAlignment::Center)
                    .fg(Color::Blue),
                Cell::new("duration")
                    .set_alignment(CellAlignment::Center)
                    .fg(Color::Blue),
                Cell::new("state")
                    .set_alignment(CellAlignment::Center)
                    .fg(Color::Blue),
                Cell::new("status")
                    .set_alignment(CellAlignment::Center)
                    .fg(Color::Blue),
                Cell::new("triggered by")
                    .set_alignment(CellAlignment::Center)
                    .fg(Color::Blue),
            ]);

        for run in runs {
            table.add_row(vec![
                Cell::new(run.id).fg(Color::Green),
                Cell::new(
                    humanize_relative_duration(run.started)
                        .unwrap_or_else(|| "Not yet".to_string()),
                ),
                Cell::new(
                    humanize_relative_duration(run.ended).unwrap_or_else(|| "Not yet".to_string()),
                ),
                Cell::new(humanize_absolute_duration(run.started, run.ended)),
                Cell::new(run.state.to_string()),
                Cell::new(run.status.to_string()),
                Cell::new(format!("{} ({})", run.trigger.label, run.trigger.name)),
            ]);
        }

        println!("{table}");
    }
}
