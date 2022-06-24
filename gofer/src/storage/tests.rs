use std::collections::HashMap;

use super::*;
use gofer_models::{RunState, RunTriggerInfo, TaskRunState, TaskRunStatus};
use rand::prelude::*;

struct TestHarness {
    db: Db,
    storage_path: String,
}

impl TestHarness {
    async fn new() -> Self {
        let mut rng = rand::thread_rng();
        let append_num: u8 = rng.gen();
        let storage_path = format!("/tmp/gofer_tests_storage{}.db", append_num);

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

    let new_namespace = gofer_models::Namespace::new(
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
        gofer_models::Namespace::new("test_namespace", "Test Namespace", "Test Description");
    harness.db.create_namespace(&test_namespace).await.unwrap();

    let test_pipeline_config = gofer_sdk::config::Pipeline::new("test_pipeline", "Test Pipeline");
    let mut test_pipeline = gofer_models::Pipeline::new(&test_namespace.id, test_pipeline_config);

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
    let test_pipeline_full =
        gofer_models::Pipeline::new(&test_namespace.id, test_pipeline_full_config);

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
        gofer_models::Namespace::new("test_namespace", "Test Namespace", "Test Description");
    harness.db.create_namespace(&test_namespace).await.unwrap();

    let test_pipeline_config = gofer_sdk::config::Pipeline::new("test_pipeline", "Test Pipeline");
    let test_pipeline = gofer_models::Pipeline::new(&test_namespace.id, test_pipeline_config);

    harness.db.create_pipeline(&test_pipeline).await.unwrap();

    let mut test_run = gofer_models::Run::new(
        &test_namespace.id,
        &test_pipeline.id,
        RunTriggerInfo {
            name: "test_trigger".to_string(),
            label: "my_test_trigger".to_string(),
        },
        vec![],
    );
    // We list runs in descend order so we need to seed intentionally such that we get the correct order.
    test_run.started = 0;
    harness.db.create_run(&test_run).await.unwrap();

    let mut test_run_2 = gofer_models::Run::new(
        &test_namespace.id,
        &test_pipeline.id,
        RunTriggerInfo {
            name: "test_trigger".to_string(),
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
        gofer_models::Namespace::new("test_namespace", "Test Namespace", "Test Description");
    harness.db.create_namespace(&test_namespace).await.unwrap();

    let test_pipeline_config = gofer_sdk::config::Pipeline::new("test_pipeline", "Test Pipeline");
    let mut test_pipeline = gofer_models::Pipeline::new(&test_namespace.id, test_pipeline_config);

    let test_task = gofer_models::Task::new("test_task", "test_image");

    test_pipeline.tasks = HashMap::new();
    test_pipeline
        .tasks
        .insert("test_task".to_string(), test_task.clone());

    harness.db.create_pipeline(&test_pipeline).await.unwrap();

    let test_run = gofer_models::Run::new(
        &test_namespace.id,
        &test_pipeline.id,
        RunTriggerInfo {
            name: "test_trigger".to_string(),
            label: "my_test_trigger".to_string(),
        },
        vec![],
    );

    harness.db.create_run(&test_run).await.unwrap();

    let mut test_task_run = gofer_models::TaskRun::new(
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

#[tokio::test]
/// Basic CRUD can be accomplished for events.
async fn crud_events() {
    let harness = TestHarness::new().await;

    let mut test_event_one = gofer_models::Event::new(gofer_models::EventKind::CreatedNamespace {
        namespace_id: "test_namespace".to_string(),
    });
    let mut test_event_two = gofer_models::Event::new(gofer_models::EventKind::CreatedPipeline {
        namespace_id: "test_namespace".to_string(),
        pipeline_id: "test_pipeline".to_string(),
    });
    let id_one = harness.db.create_event(&test_event_one).await.unwrap();
    let id_two = harness.db.create_event(&test_event_two).await.unwrap();

    assert_eq!(id_one, 1);
    assert_eq!(id_two, 2);

    test_event_one.id = id_one;
    test_event_two.id = id_two;

    let events = harness.db.list_events(0, 0, true).await.unwrap();

    assert_eq!(events.len(), 2);
    assert_eq!(events[0], test_event_two);
    assert_eq!(events[1], test_event_one);

    let event = harness.db.get_event(2).await.unwrap();
    assert_eq!(event, test_event_two);

    harness.db.delete_event(1).await.unwrap();
    let event = harness.db.get_event(1).await.unwrap_err();

    assert_eq!(event, StorageError::NotFound);
}

#[tokio::test]
/// Basic CRUD can be accomplished for trigger_registrations.
async fn crud_trigger_registrations() {
    let harness = TestHarness::new().await;

    let test_trigger_registration = gofer_models::TriggerRegistration {
        name: "test_trigger".to_string(),
        image: "docker/test".to_string(),
        user: None,
        pass: None,
        variables: HashMap::new(),
        created: 0,
    };

    harness
        .db
        .create_trigger_registration(&test_trigger_registration)
        .await
        .unwrap();

    let triggers = harness.db.list_trigger_registrations(0, 0).await.unwrap();

    assert_eq!(triggers.len(), 1);
    assert_eq!(triggers[0], test_trigger_registration);

    let trigger = harness
        .db
        .get_trigger_registration("test_trigger")
        .await
        .unwrap();
    assert_eq!(trigger, test_trigger_registration);

    harness
        .db
        .delete_trigger_registration("test_trigger")
        .await
        .unwrap();
    let trigger = harness
        .db
        .get_trigger_registration("test_trigger")
        .await
        .unwrap_err();

    assert_eq!(trigger, StorageError::NotFound);
}

#[tokio::test]
/// Basic CRUD can be accomplished for notifier_registrations.
async fn crud_notifier_registrations() {
    let harness = TestHarness::new().await;

    let test_notifier_registration = gofer_models::NotifierRegistration {
        name: "test_notifier".to_string(),
        image: "docker/test".to_string(),
        user: None,
        pass: None,
        variables: HashMap::new(),
        created: 0,
    };

    harness
        .db
        .create_notifier_registration(&test_notifier_registration)
        .await
        .unwrap();

    let notifiers = harness.db.list_notifier_registrations(0, 0).await.unwrap();

    assert_eq!(notifiers.len(), 1);
    assert_eq!(notifiers[0], test_notifier_registration);

    let notifier = harness
        .db
        .get_notifier_registration("test_notifier")
        .await
        .unwrap();
    assert_eq!(notifier, test_notifier_registration);

    harness
        .db
        .delete_notifier_registration("test_notifier")
        .await
        .unwrap();
    let notifier = harness
        .db
        .get_notifier_registration("test_notifier")
        .await
        .unwrap_err();

    assert_eq!(notifier, StorageError::NotFound);
}
