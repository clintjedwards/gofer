use gofer_sdk::config::{Pipeline, Task};

fn main() {
    Pipeline::new("dag", "DAG Test Pipeline").
        description(concat!("This pipeline shows off how you might use Gofer's DAG(Directed Acyclic Graph) system to chain ",
        "together containers that depend on other container's end states. This is obviously very useful if you ",
        "want to perform certain trees of actions depending on what happens in earlier containers.")).
        tasks(vec![
                Task::new("first-task", "ghcr.io/clintjedwards/gofer/debug/wait:latest").
                    description("This task has no dependencies so it will run immediately").
                    variable("WAIT_DURATION", "20s").always_pull_newest_image(true),
    ]).finish().unwrap();
}
