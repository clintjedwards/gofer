use gofer_sdk::config::{Pipeline, Task};

fn main() {
    Pipeline::new("run-tests", "Run Project Tests")
        .description("Runs all cargo tests for the overall Gofer workspace.")
        .tasks(vec![Task::new(
            "run-cargo-test",
            "ghcr.io/clintjedwards/gofer-repo:latest",
        )
        .description("Run cargo test command for workspace")
        .command(vec!["cargo".into(), "test".into()])])
        .finish()
        .unwrap();
}
