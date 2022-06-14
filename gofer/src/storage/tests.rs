use std::collections::HashMap;

use super::*;
use crate::models::{self, RunState, RunTriggerInfo, TaskRunState, TaskRunStatus};
use rand::prelude::*;

struct TestHarness {
    db: Db,
    storage_path: String,
}

impl TestHarness {
    async fn new() -> Self {
        let mut rng = rand::thread_rng();
        let append_num: u8 = rng.gen();
        let storage_path = format!("/tmp/gofer_integration_test{}.db", append_num);

        let db = Db::new(&storage_path).await.unwrap();

        Self { db, storage_path }
    }
}

impl Drop for TestHarness {
    fn drop(&mut self) {
        std::fs::remove_file(&self.storage_path).unwrap();
        std::fs::remove_file(format!("{}{}", &self.storage_path, "-shm")).unwrap();
        std::fs::remove_file(format!("{}{}", &self.storage_path, "-wal")).unwrap();
    }
}

#[tokio::test]
/// Basic CRUD can be accomplished for namespaces.
async fn crud_namespaces() {
    let harness = TestHarness::new().await;

    let new_namespace = models::Namespace::new(
        "test_namespace",
        "Test Namespace",
        "a namespace example for integration testing",
    );

    harness.db.create_namespace(&new_namespace).await.unwrap();
    let namespaces = harness.db.list_namespaces(0, 0).await.unwrap();

    assert_eq!(namespaces.len(), 1);
    assert_eq!(namespaces[0], new_namespace);

    let namespace = harness.db.get_namespace(&new_namespace.id).await.unwrap();
    assert_eq!(namespace, new_namespace);

    let mut updated_namespace = new_namespace.clone();
    updated_namespace.name = "Test Namespace Updated".to_string();

    harness
        .db
        .update_namespace(&updated_namespace)
        .await
        .unwrap();

    let namespace = harness.db.get_namespace(&new_namespace.id).await.unwrap();
    assert_eq!(namespace, updated_namespace);

    harness
        .db
        .delete_namespace(&new_namespace.id)
        .await
        .unwrap();

    let namespace = harness
        .db
        .get_namespace(&new_namespace.id)
        .await
        .unwrap_err();

    assert_eq!(namespace, StorageError::NotFound);
}

#[tokio::test]
/// Basic CRUD can be accomplished for pipelines.
async fn crud_pipelines() {
    let harness = TestHarness::new().await;

    let test_namespace =
        models::Namespace::new("test_namespace", "Test Namespace", "Test Description");
    harness.db.create_namespace(&test_namespace).await.unwrap();

    let test_pipeline_config = gofer_sdk::config::Pipeline::new("test_pipeline", "Test Pipeline");
    let mut test_pipeline = models::Pipeline::new(&test_namespace.id, test_pipeline_config);

    harness.db.create_pipeline(&test_pipeline).await.unwrap();

    let test_pipeline_full_config =
        gofer_sdk::config::Pipeline::new("test_pipeline_full", "Test Pipeline")
            .description("a fully loaded pipeline config for testing")
            .parallelism(10)
            .tasks(vec![gofer_sdk::config::Task::new(
                "test_task",
                "test_image",
            )])
            .triggers(vec![gofer_sdk::config::PipelineTriggerConfig::new(
                "test_trigger",
                "test_trigger",
            )])
            .notifiers(vec![gofer_sdk::config::PipelineNotifierConfig::new(
                "test_notifier",
                "test_notifier",
            )]);
    let test_pipeline_full = models::Pipeline::new(&test_namespace.id, test_pipeline_full_config);

    harness
        .db
        .create_pipeline(&test_pipeline_full)
        .await
        .unwrap();

    let pipelines = harness
        .db
        .list_pipelines(0, 0, &test_namespace.id)
        .await
        .unwrap();

    assert_eq!(pipelines.len(), 2);
    assert_eq!(pipelines[0], test_pipeline);
    assert_eq!(pipelines[1], test_pipeline_full);

    let pipeline = harness
        .db
        .get_pipeline(&test_namespace.id, &test_pipeline.id)
        .await
        .unwrap();

    assert_eq!(pipeline, test_pipeline);

    test_pipeline.name = "Test Pipeline Updated".to_string();

    harness.db.update_pipeline(&test_pipeline).await.unwrap();

    let pipeline = harness
        .db
        .get_pipeline(&test_namespace.id, &test_pipeline.id)
        .await
        .unwrap();
    assert_eq!(pipeline, test_pipeline);

    harness
        .db
        .delete_pipeline(&test_namespace.id, &test_pipeline.id)
        .await
        .unwrap();

    let pipeline = harness
        .db
        .get_pipeline(&test_namespace.id, &test_pipeline.id)
        .await
        .unwrap_err();

    assert_eq!(pipeline, StorageError::NotFound);
}

#[tokio::test]
/// Basic CRUD can be accomplished for runs.
async fn crud_runs() {
    let harness = TestHarness::new().await;

    let test_namespace =
        models::Namespace::new("test_namespace", "Test Namespace", "Test Description");
    harness.db.create_namespace(&test_namespace).await.unwrap();

    let test_pipeline_config = gofer_sdk::config::Pipeline::new("test_pipeline", "Test Pipeline");
    let test_pipeline = models::Pipeline::new(&test_namespace.id, test_pipeline_config);

    harness.db.create_pipeline(&test_pipeline).await.unwrap();

    let mut test_run = models::Run::new(
        &test_namespace.id,
        &test_pipeline.id,
        RunTriggerInfo {
            kind: "test_trigger".to_string(),
            label: "my_test_trigger".to_string(),
        },
        vec![],
    );
    // We list runs in descend order so we need to seed intentionally such that we get the correct order.
    test_run.started = 0;
    harness.db.create_run(&test_run).await.unwrap();

    let mut test_run_2 = models::Run::new(
        &test_namespace.id,
        &test_pipeline.id,
        RunTriggerInfo {
            kind: "test_trigger".to_string(),
            label: "my_test_trigger".to_string(),
        },
        vec![],
    );
    harness.db.create_run(&test_run_2).await.unwrap();

    let runs = harness
        .db
        .list_runs(0, 0, &test_namespace.id, &test_pipeline.id)
        .await
        .unwrap();

    test_run.id = 1; // Because we auto-assign run id
    test_run_2.id = 2;

    assert_eq!(runs.len(), 2);
    assert_eq!(runs[0], test_run_2);
    assert_eq!(runs[1], test_run);

    let run = harness
        .db
        .get_run(&test_namespace.id, &test_pipeline.id, test_run.id)
        .await
        .unwrap();

    assert_eq!(run, test_run);

    let runs = harness
        .db
        .batch_get_runs(
            &test_namespace.id,
            &test_pipeline.id,
            &vec![test_run.id, test_run_2.id],
        )
        .await
        .unwrap();

    assert_eq!(runs.len(), 2);
    assert_eq!(runs[0], test_run);
    assert_eq!(runs[1], test_run_2);

    test_run.state = RunState::Complete;

    harness.db.update_run(&test_run).await.unwrap();

    let run = harness
        .db
        .get_run(&test_namespace.id, &test_pipeline.id, test_run.id)
        .await
        .unwrap();
    assert_eq!(run, test_run);

    harness
        .db
        .delete_run(&test_namespace.id, &test_pipeline.id, test_run.id)
        .await
        .unwrap();

    let run = harness
        .db
        .get_run(&test_namespace.id, &test_pipeline.id, test_run.id)
        .await
        .unwrap_err();

    assert_eq!(run, StorageError::NotFound);
}

#[tokio::test]
/// Basic CRUD can be accomplished for task runs.
async fn crud_task_runs() {
    let harness = TestHarness::new().await;

    let test_namespace =
        models::Namespace::new("test_namespace", "Test Namespace", "Test Description");
    harness.db.create_namespace(&test_namespace).await.unwrap();

    let test_pipeline_config = gofer_sdk::config::Pipeline::new("test_pipeline", "Test Pipeline");
    let mut test_pipeline = models::Pipeline::new(&test_namespace.id, test_pipeline_config);

    let test_task = models::Task::new("test_task", "test_image");

    test_pipeline.tasks = HashMap::new();
    test_pipeline
        .tasks
        .insert("test_task".to_string(), test_task.clone());

    harness.db.create_pipeline(&test_pipeline).await.unwrap();

    let test_run = models::Run::new(
        &test_namespace.id,
        &test_pipeline.id,
        RunTriggerInfo {
            kind: "test_trigger".to_string(),
            label: "my_test_trigger".to_string(),
        },
        vec![],
    );

    harness.db.create_run(&test_run).await.unwrap();

    let mut test_task_run = models::TaskRun::new(
        &test_namespace.id,
        &test_pipeline.id,
        test_run.id,
        test_task,
    );

    harness.db.create_task_run(&test_task_run).await.unwrap();

    let task_runs = harness
        .db
        .list_task_runs(0, 0, &test_namespace.id, &test_pipeline.id, test_run.id)
        .await
        .unwrap();

    assert_eq!(task_runs.len(), 1);
    assert_eq!(task_runs[0], test_task_run);

    let task_run = harness
        .db
        .get_task_run(
            &test_namespace.id,
            &test_pipeline.id,
            test_run.id,
            &test_task_run.id,
        )
        .await
        .unwrap();

    assert_eq!(task_run, test_task_run);

    test_task_run.state = TaskRunState::Complete;
    harness.db.update_task_run(&test_task_run).await.unwrap();

    let task_run = harness
        .db
        .get_task_run(
            &test_namespace.id,
            &test_pipeline.id,
            test_run.id,
            &test_task_run.id,
        )
        .await
        .unwrap();

    assert_eq!(task_run, test_task_run);

    harness
        .db
        .update_task_run_state(
            &test_task_run.namespace,
            &test_task_run.pipeline,
            test_task_run.run,
            &test_task_run.id,
            TaskRunState::Processing,
        )
        .await
        .unwrap();

    let task_run = harness
        .db
        .get_task_run(
            &test_namespace.id,
            &test_pipeline.id,
            test_run.id,
            &test_task_run.id,
        )
        .await
        .unwrap();

    assert_eq!(task_run.state, TaskRunState::Processing);

    harness
        .db
        .update_task_run_status(
            &test_task_run.namespace,
            &test_task_run.pipeline,
            test_task_run.run,
            &test_task_run.id,
            TaskRunStatus::Failed,
        )
        .await
        .unwrap();

    let task_run = harness
        .db
        .get_task_run(
            &test_namespace.id,
            &test_pipeline.id,
            test_run.id,
            &test_task_run.id,
        )
        .await
        .unwrap();

    assert_eq!(task_run.status, TaskRunStatus::Failed);

    harness
        .db
        .delete_task_run(
            &test_namespace.id,
            &test_pipeline.id,
            test_run.id,
            &test_task_run.id,
        )
        .await
        .unwrap();

    let task_run = harness
        .db
        .get_task_run(
            &test_namespace.id,
            &test_pipeline.id,
            test_run.id,
            &test_task_run.id,
        )
        .await
        .unwrap_err();

    assert_eq!(task_run, StorageError::NotFound);
}
