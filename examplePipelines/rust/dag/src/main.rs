use gofer_sdk::config::{CustomTask, Pipeline, RequiredParentStatus};

fn main() {
    Pipeline::new("dag", "DAG Test Pipeline").
        description(concat!("This pipeline shows off how you might use Gofer's DAG(Directed Acyclic Graph) system to chain ",
        "together containers that depend on other container's end states. This is obviously very useful if you ",
        "want to perform certain trees of actions depending on what happens in earlier containers.")).
        tasks(vec![
            Box::new(
                CustomTask::new("first_task", "ghcr.io/clintjedwards/gofer/debug/wait:latest").
                    description("This task has no dependencies so it will run immediately").
                    variable("WAIT_DURATION", "20s")
            ),
            Box::new(
                CustomTask::new("depends_on_first", "ghcr.io/clintjedwards/gofer/debug/log:latest").
                description(concat!("This task depends on the first task to finish with a successful result. ",
                                    "This means that if the first task fails this task will not run")).
                depends_on("first_task", RequiredParentStatus::Success).
                variable("LOGS_HEADER", "This string is a stand in for something you might pass to your custom task")
            ),
            Box::new(
                CustomTask::new("depends_on_second", "docker.io/library/hello-world").
                description("This task depends on the second task, but will run after it's finished regardless of the result").
                depends_on("depends_on_first", RequiredParentStatus::Any)
            )
    ]).finish().unwrap();
}
