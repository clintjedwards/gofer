use gofer_models::task_run;

pub fn task_run_log_path(log_dir: &str, task_run: &task_run::TaskRun) -> String {
    return format!(
        "{}/{}_{}_{}_{}",
        log_dir, &task_run.namespace, task_run.pipeline, task_run.run, task_run.id
    );
}

pub fn secret_key(namespace: &str, pipeline: &str, key: &str) -> String {
    return format!("{}_{}_{}", namespace, pipeline, key);
}

pub fn pipeline_object_key(namespace: &str, pipeline: &str, key: &str) -> String {
    return format!("{}_{}_{}", namespace, pipeline, key);
}

pub fn run_object_key(namespace: &str, pipeline: &str, run: u64, key: &str) -> String {
    return format!("{}_{}_{}_{}", namespace, pipeline, run, key);
}

pub fn task_container_id(namespace: &str, pipeline: &str, run: u64, task_run: &str) -> String {
    return format!("{}_{}_{}_{}", namespace, pipeline, run, task_run);
}

pub fn trigger_container_id(name: &str) -> String {
    return format!("trigger_{}", name);
}
