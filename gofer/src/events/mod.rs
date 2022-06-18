use crate::storage;
use std::collections::HashMap;
use std::sync::mpsc::{Receiver, Sender};

/// The event bus is a central handler for all things related to events with the application.
pub struct EventBus {
    storage: storage::Db,
    retention: u64,
    subscribers: HashMap<gofer_models::EventKind, HashMap<String, Subscription>>,
}

/// A representation of a new subscription to a certain topic.
pub struct Subscription {
    pub id: String,
    pub kind: gofer_models::EventKind,
    pub events: Receiver<gofer_models::Event>,
}
