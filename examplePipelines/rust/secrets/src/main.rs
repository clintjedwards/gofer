use gofer_sdk::config::{pipeline_secret, CustomTask, Pipeline};
use std::collections::HashMap;

fn main() {
    Pipeline::new("secrets", "Secrets Pipeline")
        .description(concat!(
            "This pipeline displays how one might use Gofer's object/kv store to pass container ",
            "results to other containers."
        ))
        .tasks(vec![Box::new(
            CustomTask::new(
                "simple_task",
                "ghcr.io/clintjedwards/gofer/debug/log:latest",
            )
            .description("This task has no dependencies so it will run immediately")
            .variables(HashMap::from([
                ("SOME_VARIABLE".to_string(), "something here".to_string()),
                ("LOGS_HEADER".to_string(), pipeline_secret("logs_header")),
                (
                    "ALTERNATE_LOGS_HEADER".to_string(),
                    "pipeline_secret{{alternate_logs_header}}".to_string(),
                ),
            ])),
        )])
        .finish()
        .unwrap();
}
