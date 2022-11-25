use gofer_sdk::config::{CustomTask, Extension, Pipeline};

fn main() {
    Pipeline::new("extension", "Extension Pipeline").
        description(concat!("This pipeline shows off the various features of a simple Gofer pipeline. ",
        "Extensions, Tasks, and dependency graphs are all tools that can be wielded to create as complicated ",
        "pipelines as need be.")).
        extensions(vec![
            Extension::new("interval", "every_one_minute").setting("every", "1m")
        ]).
        tasks(vec![
            Box::new(
                CustomTask::new("simple_task", "ubuntu:latest").
                    description("This task simply prints our hello-world message and exits!").
                    command(vec!["echo".to_string(), "Hello from Gofer!".to_string()])
            )
        ]).finish().unwrap();
}
