use super::*;
use crate::storage::Db;
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
async fn publish() {
    let harness = TestHarness::new().await;
    let event_bus = EventBus::new(harness.db.clone(), 5, 5000);

    let mut new_event = gofer_models::Event::new(gofer_models::EventKind::CreatedNamespace {
        namespace_id: "test_namespace".to_string(),
    });

    let new_event_id = event_bus.publish(&mut new_event).await.unwrap();

    assert_eq!(new_event_id, 1);

    let retrieved_event = harness.db.get_event(new_event_id).await.unwrap();

    assert_eq!(new_event, retrieved_event);
}

#[tokio::test]
/// Subscribe to one event kind.
async fn subscribe_one() {
    let harness = TestHarness::new().await;
    let event_bus = EventBus::new(harness.db.clone(), 5, 5000);

    let subscription = event_bus
        .subscribe(gofer_models::EventKind::CreatedNamespace {
            namespace_id: "".to_string(),
        })
        .await;

    let mut new_event_one = gofer_models::Event::new(gofer_models::EventKind::CreatedNamespace {
        namespace_id: "test_namespace".to_string(),
    });

    let mut new_event_two = gofer_models::Event::new(gofer_models::EventKind::CreatedNamespace {
        namespace_id: "test_namespace_1".to_string(),
    });

    event_bus.publish(&mut new_event_one).await.unwrap();
    event_bus.publish(&mut new_event_two).await.unwrap();

    let received_event_one = subscription.recv().unwrap();
    let received_event_two = subscription.recv().unwrap();

    assert_eq!(received_event_one, new_event_one);
    assert_eq!(received_event_two, new_event_two);
    assert_eq!(received_event_two.id, 2);
}

#[tokio::test]
/// Subscribe to the special any event kind.
async fn subscribe_any() {
    let harness = TestHarness::new().await;
    let event_bus = EventBus::new(harness.db.clone(), 5, 5000);

    let subscription = event_bus.subscribe(gofer_models::EventKind::Any).await;

    let mut new_event_one = gofer_models::Event::new(gofer_models::EventKind::CreatedNamespace {
        namespace_id: "test_namespace".to_string(),
    });

    let mut new_event_two = gofer_models::Event::new(gofer_models::EventKind::CreatedPipeline {
        namespace_id: "test_namespace".to_string(),
        pipeline_id: "test_pipeline".to_string(),
    });

    event_bus.publish(&mut new_event_one).await.unwrap();
    event_bus.publish(&mut new_event_two).await.unwrap();

    let received_event_one = subscription.recv().unwrap();
    let received_event_two = subscription.recv().unwrap();

    assert_eq!(received_event_one, new_event_one);
    assert_eq!(received_event_two, new_event_two);
    assert_eq!(received_event_two.id, 2);
}

#[tokio::test]
async fn correctly_prune_events() {
    let harness = TestHarness::new().await;
    let event_bus = EventBus::new(harness.db.clone(), 1, 5000);

    let mut new_event_one = gofer_models::Event::new(gofer_models::EventKind::CreatedNamespace {
        namespace_id: "test_namespace".to_string(),
    });
    event_bus.publish(&mut new_event_one).await.unwrap();

    let mut new_event_two = gofer_models::Event::new(gofer_models::EventKind::CreatedPipeline {
        namespace_id: "test_namespace".to_string(),
        pipeline_id: "test_pipeline".to_string(),
    });
    event_bus.publish(&mut new_event_two).await.unwrap();

    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // New event after sleep but before prune to make sure that prune only removes things older
    // than a second.
    let mut new_event_three = gofer_models::Event::new(gofer_models::EventKind::CreatedNamespace {
        namespace_id: "test_namespace".to_string(),
    });
    event_bus.publish(&mut new_event_three).await.unwrap();

    prune_events(&harness.db, 1).await.unwrap();

    let mut new_event_four = gofer_models::Event::new(gofer_models::EventKind::CreatedNamespace {
        namespace_id: "test_namespace".to_string(),
    });
    event_bus.publish(&mut new_event_four).await.unwrap();

    let events = harness.db.list_events(0, 0, false).await.unwrap();
    assert_eq!(events.len(), 2);

    let event = harness.db.get_event(1).await.unwrap_err();
    assert_eq!(event, StorageError::NotFound);

    let event = harness.db.get_event(3).await.unwrap();
    assert_eq!(new_event_three, event);
}
