#[cfg(test)]
mod tests;

use crate::storage::{self, StorageError};
use crossbeam::{channel, sync::ShardedLock};
use gofer_models::EventKind;
use slog_scope::{debug, error, info};
use std::collections::HashMap;
use std::mem;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use strum::IntoEnumIterator;

#[derive(Debug, thiserror::Error)]
pub enum EventError {
    #[error("could not find event '{0}'")]
    NotFound(u64),

    #[error("could not persist event to storage; {0}")]
    StorageError(String),

    #[error("could not send or receive event on channel; {0}")]
    ChannelError(String),
}

/// A mapping of each event type to the sender and receiver channels for that type.
type EventChannelMap = ShardedLock<
    HashMap<
        mem::Discriminant<EventKind>,
        (
            channel::Sender<gofer_models::Event>,
            channel::Receiver<gofer_models::Event>,
        ),
    >,
>;

/// The event bus is a central handler for all things related to events with the application.
/// It allows the caller to listen to and emit events.
/// This is useful as it provides an internal interface for functions to listen for events.
/// But it's even more powerful when you think of the outside applications that can be written on top.
pub struct EventBus {
    storage: storage::Db,
    event_channel_map: EventChannelMap,
}

impl EventBus {
    pub fn new(storage: storage::Db, retention: u64, prune_interval: u64) -> Self {
        let mut subscriber_map = HashMap::new();
        for event in gofer_models::EventKind::iter() {
            let (sender, receiver) = crossbeam::channel::unbounded::<gofer_models::Event>();
            subscriber_map.insert(mem::discriminant(&event), (sender, receiver));
        }

        let event_bus = Self {
            storage: storage.clone(),
            event_channel_map: ShardedLock::new(subscriber_map),
        };

        tokio::spawn(async move {
            loop {
                match prune_events(&storage, retention).await {
                    Ok(_) => (),
                    Err(e) => {
                        error!("encountered an error during attempt to prune old events";  "error" => e.to_string())
                    }
                };

                tokio::time::sleep(tokio::time::Duration::from_secs(prune_interval)).await;
            }
        });

        event_bus
    }

    /// Returns a channel receiver end which can be used to listen to events.
    /// Unfortunately, the API for this function requires that the EventKind struct fields
    /// are populated (you can use blank fields) even though they are thrown away.
    /// Passing fields to the EventKind you wish to subscribe to DOES NOT filter which
    /// events you receive back.
    /// For example specifying the namespace_id for a CreateNamespace event will get still
    /// get you a subscription to all namespaces.
    pub async fn subscribe(
        &self,
        kind: gofer_models::EventKind,
    ) -> channel::Receiver<gofer_models::Event> {
        let event_channel_map = self.event_channel_map.read().unwrap();
        let (_, read_channel) = &event_channel_map[&mem::discriminant(&kind)];

        read_channel.clone()
    }

    /// Allows caller to emit a new event to the eventbus. Mutates event to have the proper
    /// id and returns the id generated.
    pub async fn publish(&self, event: &mut gofer_models::Event) -> Result<u64, EventError> {
        let id = self
            .storage
            .create_event(event)
            .await
            .map_err(|e| EventError::StorageError(e.to_string()))?;

        event.id = id;

        let event_channel_map = self.event_channel_map.read().unwrap();
        let (event_send_channel, _) = &event_channel_map[&mem::discriminant(&event.kind)];
        let (any_send_channel, _) =
            &event_channel_map[&mem::discriminant(&gofer_models::EventKind::Any)];

        event_send_channel
            .send(event.clone())
            .map_err(|e| EventError::StorageError(e.to_string()))?;
        any_send_channel
            .send(event.clone())
            .map_err(|e| EventError::StorageError(e.to_string()))?;

        Ok(id)
    }
}

async fn prune_events(storage: &storage::Db, retention: u64) -> Result<(), StorageError> {
    let mut offset = 0;
    let mut total_pruned = 0;

    loop {
        let events = storage.list_events(offset, 50, false).await?;

        for event in &events {
            if is_past_cut_date(event, retention) {
                debug!("removed event past retention period";
                        "emitted" => event.emitted,
                        "retention" => retention,
                        "current_time" => format!("{}",epoch()));

                total_pruned += 1;

                storage.delete_event(event.id).await?;
            }
        }

        if events.len() != 50 {
            if total_pruned > 0 {
                info!("pruned old events"; "retention" => retention, "total_pruned" => format!("{}", total_pruned));
            }

            return Ok(());
        }

        offset += events.len() as u64;
    }
}

fn is_past_cut_date(event: &gofer_models::Event, limit: u64) -> bool {
    let now = epoch();
    let limit = Duration::from_secs(limit).as_millis();
    let expiry_time = (now as u128) - limit;

    (event.emitted as u128) < expiry_time
}

fn epoch() -> u64 {
    let current_epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();

    u64::try_from(current_epoch).unwrap()
}
