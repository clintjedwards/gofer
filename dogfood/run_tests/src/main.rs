use gofer_sdk::config::{Pipeline, Task};

fn main() {
    Pipeline::new("run-tests", "Run Project Tests")
        .description("Runs all cargo tests for the overall Gofer workspace.")
        .tasks(vec![Task::new(
            "run-cargo-test",
            "ghcr.io/clintjedwards/gofer/tools:rust",
        )
        .description("Run cargo test command for workspace")
        .always_pull_newest_image(true)
        .command(vec!["cargo".into(), "test".into()])])
        .finish()
        .unwrap();
}
