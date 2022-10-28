use gofer_sdk::config::{CommonTask, Pipeline};

fn main() {
    Pipeline::new("common_task", "Common Task Pipeline")
        .description(concat!(
        "This pipeline shows off the common tasks feature of Gofer. ",
        "Common Tasks allow administrators to install tasks that can be shared amongst ",
        "all pipelines. This allows you to provide users with tasks that might require ",
        "variables and credentials that you might not want to manually include in every pipeline."
    ))
        .tasks(vec![Box::new(CommonTask::new("debug", "debug_task"))])
        .finish()
        .unwrap()
}
