#[cfg(test)]
mod tests;

use crate::storage::{self, StorageError};
use crossbeam::channel;
use dashmap::DashMap;
use gofer_models::event::{Event, Kind, KindDiscriminant};
use nanoid::nanoid;
use slog_scope::{debug, error, info};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Debug, thiserror::Error)]
pub enum EventError {
    #[error("unforeseen error occurred; '{0}'")]
    Unknown(String),

    #[error("could not find event '{0}'")]
    NotFound(u64),

    #[error("could not persist event to storage; {0}")]
    StorageError(String),

    #[error("could not send or receive event on channel; {0}")]
    ChannelError(String),
}

pub struct Subscription<'a> {
    id: String,
    kind: KindDiscriminant,
    event_bus: &'a EventBus,
    pub receiver: channel::Receiver<Event>,
}

impl Drop for Subscription<'_> {
    fn drop(&mut self) {
        if let Some(subscription_map) = self.event_bus.event_channel_map.get_mut(&self.kind) {
            let subscription_map = subscription_map.value();
            let send_channel = subscription_map.remove(&self.id);

            if let Some((_, send_channel)) = send_channel {
                drop(send_channel);
            }
        }
    }
}

/// A mapping of each event kind to the subscription id and sender end of the channel.
/// When publishing events we need just a lookup by event kind, but when removing
/// an event channel we need to be able to lookup by event kind and subscription id.
type EventChannelMap = DashMap<KindDiscriminant, DashMap<String, channel::Sender<Event>>>;

/// The event bus is a central handler for all things related to events with the application.
/// It allows the caller to listen to and emit events.
/// This is useful as it provides an internal interface for functions to listen for events.
/// But it's even more powerful when you think of the outside applications that can be written on top.
#[derive(Debug)]
pub struct EventBus {
    storage: storage::Db,
    event_channel_map: EventChannelMap,
}

impl EventBus {
    pub fn new(storage: storage::Db, retention: u64, prune_interval: u64) -> Self {
        let event_bus = Self {
            storage: storage.clone(),
            event_channel_map: DashMap::new(),
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
    /// The subscription return type automatically drops it's subscription upon drop/loss of scope.
    pub async fn subscribe(&self, kind: KindDiscriminant) -> Result<Subscription<'_>, EventError> {
        let subscription_map = self
            .event_channel_map
            .entry(kind)
            .or_insert_with(DashMap::new);

        let (sender, receiver) = channel::unbounded::<Event>();
        let new_subscription = Subscription {
            id: nanoid!(10),
            kind,
            event_bus: self,
            receiver,
        };

        subscription_map.insert(new_subscription.id.clone(), sender);

        Ok(new_subscription)
    }

    /// Allows caller to emit a new event to the eventbus. Returns the resulting
    /// event once it has been successfully published.
    pub async fn publish(&self, kind: Kind) -> Option<Event> {
        let mut new_event = Event::new(kind.clone());

        let mut conn = match self.storage.conn().await {
            Ok(conn) => conn,
            Err(e) => {
                error!("could not publish event"; "event" => new_event.kind.to_string(), "error" => e.to_string());
                return None;
            }
        };

        let id = match storage::events::insert(&mut conn, &new_event).await {
            Ok(id) => id,
            Err(e) => {
                error!("could not publish event"; "event" => new_event.kind.to_string(), "error" => e.to_string());
                return None;
            }
        };

        debug!("New event"; "kind" => format!("{:?}", &kind));

        new_event.id = id;

        if let Some(specific_event_subs) = self.event_channel_map.get(&KindDiscriminant::from(kind))
        {
            for item in specific_event_subs.iter() {
                let send_channel = item.value();
                match send_channel.send(new_event.clone()) {
                    Ok(v) => v,
                    Err(e) => {
                        error!("could not publish event"; "event" => new_event.kind.to_string(), "error" => e.to_string());
                        break;
                    }
                };
            }
        }

        if let Some(any_event_subs) = self.event_channel_map.get(&KindDiscriminant::Any) {
            for item in any_event_subs.iter() {
                let send_channel = item.value();
                match send_channel.send(new_event.clone()) {
                    Ok(v) => v,
                    Err(e) => {
                        error!("could not publish event"; "event" => new_event.kind.to_string(), "error" => e.to_string());
                        break;
                    }
                };
            }
        }

        Some(new_event)
    }
}

async fn prune_events(storage: &storage::Db, retention: u64) -> Result<(), StorageError> {
    let mut offset = 0;
    let mut total_pruned = 0;

    let mut conn = match storage.conn().await {
        Ok(conn) => conn,
        Err(e) => {
            error!("could not prune events; connection error");
            return Err(e);
        }
    };

    loop {
        let events = storage::events::list(&mut conn, offset, 50, false).await?;

        for event in &events {
            if is_past_cut_date(event, retention) {
                debug!("removed event past retention period";
                        "emitted" => event.emitted,
                        "retention" => retention,
                        "current_time" => format!("{}",epoch()));

                total_pruned += 1;

                storage::events::delete(&mut conn, event.id).await?;
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

fn is_past_cut_date(event: &Event, limit: u64) -> bool {
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
