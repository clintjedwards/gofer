use gofer_sdk::config::{pipeline_secret, Pipeline, Task};
use std::collections::HashMap;

fn main() {
    Pipeline::new("secrets", "Secrets Pipeline")
        .description(concat!(
            "This pipeline displays how one might use Gofer's object/kv store to pass container ",
            "results to other containers."
        ))
        .tasks(vec![Task::new(
            "simple-task",
            "ghcr.io/clintjedwards/gofer/debug/log:latest",
        )
        .description("This task has no dependencies so it will run immediately")
        .variables(HashMap::from([
            ("SOME_VARIABLE", "something here"),
            ("LOGS_HEADER", &pipeline_secret("logs_header")),
            (
                "ALTERNATE_LOGS_HEADER",
                "pipeline_secret{{alternate_logs_header}}",
            ),
        ]))])
        .finish()
        .unwrap();
}
