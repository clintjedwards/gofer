use gofer_sdk::config::{Pipeline, RequiredParentStatus, Task};

// Prints a few test results and exits with an error so the pipeline has a failure to debug.
const FAKE_TEST_RUN: &str = r#"sleep 3
echo "running unit tests..."
echo "ok    auth       0.12s"
echo "ok    billing    0.31s"
echo "ok    storage    0.08s"
echo "FAIL  api        0.44s"
echo "    handlers_test.go:42: expected status 200, got 500"
exit 1"#;

fn main() {
    Pipeline::new("dag", "DAG Test Pipeline")
        .description(concat!(
            "This pipeline shows off how you might use Gofer's DAG(Directed Acyclic Graph) system to chain ",
            "together containers that depend on other container's end states. This is obviously very useful if you ",
            "want to perform certain trees of actions depending on what happens in earlier containers."
        ))
        .tasks(vec![
            Task::new("first-task", "ghcr.io/clintjedwards/gofer/debug/wait:latest")
                .description("This task has no dependencies so it will run immediately")
                .variables(vec![("WAIT_DURATION", "20s")]),
            Task::new("depends-on-first", "ghcr.io/clintjedwards/gofer/debug/log:latest")
                .description(concat!(
                    "This task depends on the first task to finish with a successful result. This means ",
                    "that if the first task fails this task will not run"
                ))
                .depends_on("first-task", RequiredParentStatus::Success)
                .variables(vec![(
                    "LOGS_HEADER",
                    "This string is a stand in for something you might pass to your task",
                )]),
            Task::new("depends-on-second", "hello-world")
                .description(
                    "This task depends on the second task, but will run after it's finished regardless of the result",
                )
                .depends_on("depends-on-first", RequiredParentStatus::Any),
            Task::new("run-tests", "alpine:latest")
                .description("This task fails on purpose so you can see what a failure looks like in 'gofer run debug'")
                .command(vec!["sh".into(), "-c".into(), FAKE_TEST_RUN.into()]),
            Task::new("publish-release", "alpine:latest")
                .description("This task only runs if the tests pass, so it gets skipped when run-tests fails")
                .depends_on("run-tests", RequiredParentStatus::Success)
                .command(vec!["echo".into(), "publishing release".into()]),
        ])
        .finish()
        .unwrap();
}
