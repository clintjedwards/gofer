use super::*;
use gofer_models::*;
use pretty_assertions::assert_eq;
use rand::prelude::*;
use std::{collections::HashMap, ops::Deref};

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

impl Deref for TestHarness {
    type Target = Db;

    fn deref(&self) -> &Self::Target {
        &self.db
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
    let mut conn = harness.conn().await.unwrap();

    let new_namespace = namespace::Namespace::new(
        "test_namespace",
        "Test Namespace",
        "a namespace example for integration testing",
    );

    namespaces::insert(&mut conn, &new_namespace).await.unwrap();
    let namespaces = namespaces::list(&mut conn, 0, 0).await.unwrap();

    assert_eq!(namespaces.len(), 1);
    assert_eq!(namespaces[0], new_namespace);

    let namespace = namespaces::get(&mut conn, &new_namespace.id).await.unwrap();
    assert_eq!(namespace, new_namespace);

    let mut updated_namespace = new_namespace.clone();
    updated_namespace.name = "Test Namespace Updated".to_string();

    namespaces::update(
        &mut conn,
        &new_namespace.id,
        namespaces::UpdatableFields {
            name: Some(updated_namespace.name.clone()),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    let namespace = namespaces::get(&mut conn, &new_namespace.id).await.unwrap();
    assert_eq!(namespace, updated_namespace);

    namespaces::delete(&mut conn, &new_namespace.id)
        .await
        .unwrap();

    let namespace = namespaces::get(&mut conn, &new_namespace.id)
        .await
        .unwrap_err();

    assert_eq!(namespace, StorageError::NotFound);
}

#[tokio::test]
/// Basic CRUD can be accomplished for pipelines.
async fn crud_pipelines() {
    let harness = TestHarness::new().await;
    let mut conn = harness.conn().await.unwrap();

    let test_namespace =
        namespace::Namespace::new("test_namespace", "Test Namespace", "Test Description");
    namespaces::insert(&mut conn, &test_namespace)
        .await
        .unwrap();

    let test_pipeline_config = gofer_sdk::config::Pipeline::new("test_pipeline", "Test Pipeline");
    let mut test_pipeline = pipeline::Pipeline::new(&test_namespace.id, test_pipeline_config);

    pipelines::insert(&mut conn, &test_pipeline).await.unwrap();

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
            .common_tasks(vec![gofer_sdk::config::PipelineCommonTaskConfig::new(
                "test_common_task",
                "test_common_task",
            )]);
    let test_pipeline_full = pipeline::Pipeline::new(&test_namespace.id, test_pipeline_full_config);

    pipelines::insert(&mut conn, &test_pipeline_full)
        .await
        .unwrap();

    let pipelines = pipelines::list(&mut conn, 0, 0, &test_namespace.id)
        .await
        .unwrap();

    assert_eq!(pipelines.len(), 2);
    assert_eq!(pipelines[0], test_pipeline);
    assert_eq!(pipelines[1], test_pipeline_full);

    let pipeline = pipelines::get(&mut conn, &test_namespace.id, &test_pipeline.id)
        .await
        .unwrap();

    assert_eq!(pipeline, test_pipeline);

    test_pipeline.name = "Test Pipeline Updated".to_string();

    pipelines::update(
        &mut conn,
        &test_pipeline.namespace,
        &test_pipeline.id,
        pipelines::UpdatableFields {
            name: Some(test_pipeline.name.clone()),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    let pipeline = pipelines::get(&mut conn, &test_namespace.id, &test_pipeline.id)
        .await
        .unwrap();
    assert_eq!(pipeline, test_pipeline);

    pipelines::delete(&mut conn, &test_namespace.id, &test_pipeline.id)
        .await
        .unwrap();

    let pipeline = pipelines::get(&mut conn, &test_namespace.id, &test_pipeline.id)
        .await
        .unwrap_err();

    assert_eq!(pipeline, StorageError::NotFound);
}

#[tokio::test]
/// Basic CRUD can be accomplished for runs.
async fn crud_runs() {
    let harness = TestHarness::new().await;
    let mut conn = harness.conn().await.unwrap();

    let test_namespace =
        namespace::Namespace::new("test_namespace", "Test Namespace", "Test Description");
    namespaces::insert(&mut conn, &test_namespace)
        .await
        .unwrap();

    let test_pipeline_config = gofer_sdk::config::Pipeline::new("test_pipeline", "Test Pipeline");
    let test_pipeline = pipeline::Pipeline::new(&test_namespace.id, test_pipeline_config);

    pipelines::insert(&mut conn, &test_pipeline).await.unwrap();

    let mut test_run = run::Run::new(
        &test_namespace.id,
        &test_pipeline.id,
        run::TriggerInfo {
            name: "test_trigger".to_string(),
            label: "my_test_trigger".to_string(),
        },
        vec![],
    );
    // We list runs in descend order so we need to seed intentionally such that we get the correct order.
    test_run.started = 0;
    runs::insert(&mut conn, &test_run).await.unwrap();

    let mut test_run_2 = run::Run::new(
        &test_namespace.id,
        &test_pipeline.id,
        run::TriggerInfo {
            name: "test_trigger".to_string(),
            label: "my_test_trigger".to_string(),
        },
        vec![],
    );
    runs::insert(&mut conn, &test_run_2).await.unwrap();

    let runs = runs::list(&mut conn, 0, 0, &test_namespace.id, &test_pipeline.id)
        .await
        .unwrap();

    test_run.id = 1; // Because we auto-assign run id
    test_run_2.id = 2;

    assert_eq!(runs.len(), 2);
    assert_eq!(runs[0], test_run_2);
    assert_eq!(runs[1], test_run);

    let run = runs::get(
        &mut conn,
        &test_namespace.id,
        &test_pipeline.id,
        test_run.id,
    )
    .await
    .unwrap();

    assert_eq!(run, test_run);

    test_run.state = run::State::Complete;

    runs::update(
        &mut conn,
        &run,
        runs::UpdatableFields {
            state: Some(run::State::Complete),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    let run = runs::get(
        &mut conn,
        &test_namespace.id,
        &test_pipeline.id,
        test_run.id,
    )
    .await
    .unwrap();
    assert_eq!(run, test_run);

    runs::delete(
        &mut conn,
        &test_namespace.id,
        &test_pipeline.id,
        test_run.id,
    )
    .await
    .unwrap();

    let run = runs::get(
        &mut conn,
        &test_namespace.id,
        &test_pipeline.id,
        test_run.id,
    )
    .await
    .unwrap_err();

    assert_eq!(run, StorageError::NotFound);
}

#[tokio::test]
/// Basic CRUD can be accomplished for task runs.
async fn crud_task_runs() {
    let harness = TestHarness::new().await;
    let mut conn = harness.conn().await.unwrap();

    let test_namespace =
        namespace::Namespace::new("test_namespace", "Test Namespace", "Test Description");
    namespaces::insert(&mut conn, &test_namespace)
        .await
        .unwrap();

    let test_pipeline_config = gofer_sdk::config::Pipeline::new("test_pipeline", "Test Pipeline");
    let mut test_pipeline = pipeline::Pipeline::new(&test_namespace.id, test_pipeline_config);

    let test_task = task::Task::new("test_task", "test_image");

    test_pipeline.tasks = HashMap::new();
    test_pipeline
        .tasks
        .insert("test_task".to_string(), test_task.clone());

    pipelines::insert(&mut conn, &test_pipeline).await.unwrap();

    let test_run = run::Run::new(
        &test_namespace.id,
        &test_pipeline.id,
        run::TriggerInfo {
            name: "test_trigger".to_string(),
            label: "my_test_trigger".to_string(),
        },
        vec![],
    );

    runs::insert(&mut conn, &test_run).await.unwrap();

    let mut test_task_run = task_run::TaskRun::new(
        &test_namespace.id,
        &test_pipeline.id,
        test_run.id,
        test_task,
    );

    task_runs::insert(&mut conn, &test_task_run).await.unwrap();

    let task_runs = task_runs::list(
        &mut conn,
        0,
        0,
        &test_namespace.id,
        &test_pipeline.id,
        test_run.id,
    )
    .await
    .unwrap();

    assert_eq!(task_runs.len(), 1);
    assert_eq!(task_runs[0], test_task_run);

    let task_run = task_runs::get(
        &mut conn,
        &test_namespace.id,
        &test_pipeline.id,
        test_run.id,
        &test_task_run.id,
    )
    .await
    .unwrap();

    assert_eq!(task_run, test_task_run);

    test_task_run.state = task_run::State::Complete;
    task_runs::update(
        &mut conn,
        &task_run,
        task_runs::UpdatableFields {
            state: Some(task_run::State::Complete),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    let task_run = task_runs::get(
        &mut conn,
        &test_namespace.id,
        &test_pipeline.id,
        test_run.id,
        &test_task_run.id,
    )
    .await
    .unwrap();

    assert_eq!(task_run, test_task_run);

    task_runs::delete(
        &mut conn,
        &test_namespace.id,
        &test_pipeline.id,
        test_run.id,
        &test_task_run.id,
    )
    .await
    .unwrap();

    let task_run = task_runs::get(
        &mut conn,
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
    let mut conn = harness.conn().await.unwrap();

    let mut test_event_one = event::Event::new(event::Kind::CreatedNamespace {
        namespace_id: "test_namespace".to_string(),
    });
    let mut test_event_two = event::Event::new(event::Kind::CreatedPipeline {
        namespace_id: "test_namespace".to_string(),
        pipeline_id: "test_pipeline".to_string(),
    });
    let id_one = events::insert(&mut conn, &test_event_one).await.unwrap();
    let id_two = events::insert(&mut conn, &test_event_two).await.unwrap();

    assert_eq!(id_one, 1);
    assert_eq!(id_two, 2);

    test_event_one.id = id_one;
    test_event_two.id = id_two;

    let events = events::list(&mut conn, 0, 0, true).await.unwrap();

    assert_eq!(events.len(), 2);
    assert_eq!(events[0], test_event_two);
    assert_eq!(events[1], test_event_one);

    let event = events::get(&mut conn, 2).await.unwrap();
    assert_eq!(event, test_event_two);

    events::delete(&mut conn, 1).await.unwrap();
    let event = events::get(&mut conn, 1).await.unwrap_err();

    assert_eq!(event, StorageError::NotFound);
}

#[tokio::test]
/// Basic CRUD can be accomplished for trigger_registrations.
async fn crud_trigger_registrations() {
    let harness = TestHarness::new().await;
    let mut conn = harness.conn().await.unwrap();

    let test_trigger_registration = trigger::Registration {
        name: "test_trigger".to_string(),
        image: "docker/test".to_string(),
        user: None,
        pass: None,
        variables: HashMap::new(),
        created: 0,
        status: trigger::Status::Enabled,
    };

    trigger_registrations::insert(&mut conn, &test_trigger_registration)
        .await
        .unwrap();

    let triggers = trigger_registrations::list(&mut conn, 0, 0).await.unwrap();

    assert_eq!(triggers.len(), 1);
    assert_eq!(triggers[0], test_trigger_registration);

    let trigger = trigger_registrations::get(&mut conn, "test_trigger")
        .await
        .unwrap();
    assert_eq!(trigger, test_trigger_registration);

    trigger_registrations::delete(&mut conn, "test_trigger")
        .await
        .unwrap();
    let trigger = trigger_registrations::get(&mut conn, "test_trigger")
        .await
        .unwrap_err();

    assert_eq!(trigger, StorageError::NotFound);
}

#[tokio::test]
/// Basic CRUD can be accomplished for common_task_registrations.
async fn crud_common_task_registrations() {
    let harness = TestHarness::new().await;
    let mut conn = harness.conn().await.unwrap();

    let test_common_task_registration = common_task::Registration {
        name: "test_common_task".to_string(),
        image: "docker/test".to_string(),
        user: None,
        pass: None,
        variables: HashMap::new(),
        created: 0,
        status: common_task::Status::Enabled,
    };

    common_task_registrations::insert(&mut conn, &test_common_task_registration)
        .await
        .unwrap();

    let common_tasks = common_task_registrations::list(&mut conn, 0, 0)
        .await
        .unwrap();

    assert_eq!(common_tasks.len(), 1);
    assert_eq!(common_tasks[0], test_common_task_registration);

    let common_task = common_task_registrations::get(&mut conn, "test_common_task")
        .await
        .unwrap();
    assert_eq!(common_task, test_common_task_registration);

    common_task_registrations::delete(&mut conn, "test_common_task")
        .await
        .unwrap();
    let common_task = common_task_registrations::get(&mut conn, "test_common_task")
        .await
        .unwrap_err();

    assert_eq!(common_task, StorageError::NotFound);
}
