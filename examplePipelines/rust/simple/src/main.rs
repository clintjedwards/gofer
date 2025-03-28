use gofer_sdk::config::{Pipeline, Task};

fn main() {
    Pipeline::new("simple", "Simple Pipeline")
    .description("This pipeline shows off a very simple Gofer pipeline that simply pulls in a container and runs \
a command. Veterans of CI/CD tooling should be familiar with this pattern.

Shown below, tasks are the building blocks of a pipeline. They represent individual containers and can be \
configured to depend on one or multiple other tasks.

In the task here, we simply call the very familiar Ubuntu container and run some commands of our own.

While this is the simplest example of Gofer, the vision is to move away from writing our logic code in long bash \
scripts within these task definitions.

Ideally, these tasks are custom containers built with the purpose of being run within Gofer for a particular \
workflow. Allowing you to keep the logic code closer to the actual object that uses it and keeping the Gofer \
pipeline configurations from becoming a mess.

    ")
        .tasks(vec![
                Task::new("simple-task", "ubuntu:latest").
                    description("This task simply prints our hello-world message and exits!").
                    command(vec!["echo".to_string(), "Hello from Gofer!".to_string()])
        ])
        .finish().unwrap();
}
