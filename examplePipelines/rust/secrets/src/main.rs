use gofer_sdk::config::{Pipeline, Task, global_secret, pipeline_secret};
use std::collections::HashMap;

fn main() {
    Pipeline::new("secrets", "Secrets Pipeline")
        .description(concat!(
            "This pipeline shows how to pass pipeline and global secrets to a task. Gofer swaps in the real ",
            "values when the task starts, so they never show up in the pipeline config."
        ))
        .tasks(vec![
            Task::new("simple-task", "ghcr.io/clintjedwards/gofer/debug/log:latest")
                .description("This task has no dependencies so it will run immediately")
                .variables(HashMap::from([
                    ("SOME_VARIABLE", "something here".to_string()),
                    ("ALTERNATE_LOGS_HEADER", pipeline_secret("alternate_logs_header")),
                    ("SOME_GLOBAL_SECRET_I_NEED", global_secret("test_secret_global")),
                ])),
        ])
        .finish()
        .unwrap();
}
