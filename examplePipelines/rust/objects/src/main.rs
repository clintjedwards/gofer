use gofer_sdk::config::{pipeline_object, Pipeline, Task};
use std::collections::HashMap;

fn main() {
    Pipeline::new("objects", "Objects Pipeline").
        description("This pipeline displays how one might use Gofer's object/kv store to pass container results to others containers.").
        tasks(vec![
                Task::new("simple-task", "ghcr.io/clintjedwards/gofer/debug/log:latest").
                    description("This task has no dependencies so it will run immediately").
                    variables(HashMap::from([
                        ("SOME_VARIABLE".to_string(), "something here".to_string()),
                        ("LOGS_HEADER".to_string(), pipeline_object("logs_header")),
                        ("ALTERNATE_LOGS_HEADER".to_string(), "pipeline_object{{alternate_logs_header}}".to_string())
                        ])
                    )
        ]).finish().unwrap();
}
