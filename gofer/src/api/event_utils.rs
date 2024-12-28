use crate::{
    api::{epoch_milli, runs, task_executions},
    storage,
};
use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use strum::{Display, EnumDiscriminants, EnumIter, EnumString};
use tokio::sync::broadcast;
use tracing::{debug, error, info, trace};
use uuid::Uuid;

#[derive(
    Debug,
    PartialEq,
    Eq,
    EnumIter,
    EnumString,
    EnumDiscriminants,
    Display,
    Serialize,
    Deserialize,
    Clone,
    JsonSchema,
)]
#[strum_discriminants(derive(EnumString, Display, Hash))]
#[strum_discriminants(strum(serialize_all = "snake_case"))]
#[strum(serialize_all = "snake_case")]
#[strum(ascii_case_insensitive)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    /// The Any kind is a special event kind that denotes the caller wants to listen for any event.
    /// It should not be used as a normal event type(for example do not publish anything with it).
    /// It is internal only and not passed back on event streaming.
    Any,

    // Namespace events
    CreatedNamespace {
        namespace_id: String,
    },
    DeletedNamespace {
        namespace_id: String,
    },

    // Pipeline events
    DisabledPipeline {
        namespace_id: String,
        pipeline_id: String,
    },
    EnabledPipeline {
        namespace_id: String,
        pipeline_id: String,
    },
    CreatedPipeline {
        namespace_id: String,
        pipeline_id: String,
    },
    DeletedPipeline {
        namespace_id: String,
        pipeline_id: String,
    },

    // Deployment events
    StartedDeployment {
        namespace_id: String,
        pipeline_id: String,
        start_version: u64,
        end_version: u64,
    },
    CompletedDeployment {
        namespace_id: String,
        pipeline_id: String,
        start_version: u64,
        end_version: u64,
    },

    // Run events
    QueuedRun {
        namespace_id: String,
        pipeline_id: String,
        run_id: u64,
    },
    StartedRun {
        namespace_id: String,
        pipeline_id: String,
        run_id: u64,
    },
    CompletedRun {
        namespace_id: String,
        pipeline_id: String,
        run_id: u64,
        status: runs::Status,
    },
    StartedRunCancellation {
        namespace_id: String,
        pipeline_id: String,
        run_id: u64,
    },

    // Task execution events
    CreatedTaskExecution {
        namespace_id: String,
        pipeline_id: String,
        run_id: u64,
        task_execution_id: String,
    },
    StartedTaskExecution {
        namespace_id: String,
        pipeline_id: String,
        run_id: u64,
        task_execution_id: String,
    },
    CompletedTaskExecution {
        namespace_id: String,
        pipeline_id: String,
        run_id: u64,
        task_execution_id: String,
        status: task_executions::Status,
    },
    StartedTaskExecutionCancellation {
        namespace_id: String,
        pipeline_id: String,
        run_id: u64,
        task_execution_id: String,
        timeout: u64,
    },

    // Extension events
    InstalledExtension {
        id: String,
        image: String,
    },
    UninstalledExtension {
        id: String,
        image: String,
    },
    EnabledExtension {
        id: String,
        image: String,
    },
    DisabledExtension {
        id: String,
        image: String,
    },

    // Subscriptions
    PipelineExtensionSubscriptionRegistered {
        namespace_id: String,
        pipeline_id: String,
        extension_id: String,
        subscription_id: String,
    },
    PipelineExtensionSubscriptionUnregistered {
        namespace_id: String,
        pipeline_id: String,
        extension_id: String,
        subscription_id: String,
    },

    // Permissioning eventss
    CreatedRole {
        role_id: String,
    },
    DeletedRole {
        role_id: String,
    },
}

/// A single event
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
pub struct Event {
    /// Unique identifier for event.
    pub id: String,

    /// The type of event it is.
    pub kind: Kind,

    /// Time event was performed in epoch milliseconds.
    pub emitted: u64,
}

impl TryFrom<storage::events::Event> for Event {
    type Error = anyhow::Error;

    fn try_from(value: storage::events::Event) -> Result<Self> {
        let emitted = value.emitted.parse::<u64>().with_context(|| {
            format!(
                "Could not parse field 'emitted' from storage value '{}'",
                value.emitted
            )
        })?;

        let kind: Kind = serde_json::from_str(&value.kind).with_context(|| {
            format!(
                "Could not parse field 'kind' from storage value '{}'",
                value.kind
            )
        })?;

        Ok(Event {
            id: value.id,
            kind,
            emitted,
        })
    }
}

impl TryFrom<Event> for storage::events::Event {
    type Error = anyhow::Error;

    fn try_from(value: Event) -> Result<Self> {
        let kind = serde_json::to_string(&value.kind).with_context(|| {
            format!(
                "Could not parse field 'kind' to storage value '{:#?}'",
                value.kind
            )
        })?;

        Ok(Self {
            id: value.id,
            kind,
            details: "test".into(),
            emitted: value.emitted.to_string(),
        })
    }
}

impl Event {
    pub fn new(kind: Kind) -> Self {
        Self {
            id: Uuid::now_v7().to_string(),
            kind,
            emitted: epoch_milli(),
        }
    }
}

#[async_trait]
pub trait EventListener: Send {
    async fn next(&mut self) -> Result<Event>;
}

pub struct HistoricalEventListener {
    /// Handle to the database pool so that we can populate events from the database.
    storage: storage::Db,

    /// The last ID that was given to the user. We keep this so that we can understand where to filter or query from.
    last_id_read: Option<String>,

    /// The event stream populated by the database.
    stream: std::collections::VecDeque<Event>,
}

impl HistoricalEventListener {
    async fn repopulate_queue(&mut self) -> Result<()> {
        if self.last_id_read.is_none() {
            bail!("When attempting to repopulate queue the last_id_read was empty; this should not happen.");
        }

        let mut conn = match self.storage.read_conn().await {
            Ok(conn) => conn,
            Err(err) => {
                bail!(
                    "Could not establish connection to database in order to read events: {:#?}",
                    err
                );
            }
        };

        let events = storage::events::list_by_id(
            &mut conn,
            self.last_id_read.as_ref().unwrap(),
            50,
        )
        .await
        .context(
            "Could not list historical events while attempting to populate events from database.",
        )?;

        for storage_event in events {
            let event: Event = storage_event.try_into()?;
            self.stream.push_back(event);
        }

        Ok(())
    }

    pub async fn next(&mut self) -> Option<Event> {
        if let Some(event) = self.stream.pop_front() {
            return Some(event);
        }

        if let Err(err) = self.repopulate_queue().await {
            error!(
                error = %err,
                "Failed to repopulate queue",
            );

            return None;
        }

        self.stream.pop_front()
    }
}

#[async_trait]
impl EventListener for HistoricalEventListener {
    async fn next(&mut self) -> Result<Event> {
        self.next()
            .await
            .ok_or(anyhow::anyhow!("Could not receive event"))
    }
}

#[async_trait]
impl EventListener for broadcast::Receiver<Event> {
    async fn next(&mut self) -> Result<Event> {
        self.recv()
            .await
            .map_err(|e| anyhow::anyhow!("Could not receive event; {:#?}", e))
    }
}

/// The event bus is a central handler for all things related to events with the application.
/// It allows a subscriber to listen to events and a sender to emit events.
/// This is useful as it provides an internal interface for functions to listen for events.
/// But it's even more powerful when you think of the outside applications that can be written on top.
#[derive(Debug, Clone)]
pub struct EventBus {
    storage: storage::Db,
    broadcast_channel: broadcast::Sender<Event>,
}

impl EventBus {
    pub fn new(storage: storage::Db, retention: u64, prune_interval: u64) -> Self {
        let (tx, _) = broadcast::channel(100);

        let event_bus = Self {
            storage: storage.clone(),
            broadcast_channel: tx,
        };

        tokio::spawn(async move {
            loop {
                match prune_events(&storage, retention).await {
                    Ok(_) => (),
                    Err(e) => {
                        error!(error = %e, "encountered an error during attempt to prune old events")
                    }
                };

                tokio::time::sleep(tokio::time::Duration::from_secs(prune_interval)).await;
            }
        });

        event_bus
    }

    /// Returns a channel receiver end which can be used to list to live events as they are emitted.
    /// Since this is done purely in memory it is faster and less DB heavy than [`subscribe_historical`].
    pub fn subscribe_live(&self) -> impl EventListener {
        self.broadcast_channel.subscribe()
    }

    /// Returns a channel receiver end which can be used to listen to events as they are commited to the database.
    /// You can provide an event ID to start from and the receiver returned will return events starting from that id.
    ///
    /// Omitting the ID starts the subscription from the oldest event.
    pub async fn subscribe_historical(
        &self,
        start_from: Option<String>,
    ) -> Result<impl EventListener> {
        let mut stream = std::collections::VecDeque::new();
        let mut last_id_read = None;

        if let Some(id) = start_from {
            last_id_read = Some(id.clone());

            let mut conn = match self.storage.read_conn().await {
                Ok(conn) => conn,
                Err(err) => {
                    bail!(
                        "Could not establish connection to database in order to read events; {:#?}",
                        err
                    );
                }
            };

            let events = storage::events::list_by_id(&mut conn, &id, 50)
                .await
                .context(
                    "Could not list historical events\
                 while attempting to populate events from database.",
                )?;

            for storage_event in events {
                let event: Event = storage_event.try_into()?;
                stream.push_back(event);
            }
        } else {
            let mut conn = match self.storage.read_conn().await {
                Ok(conn) => conn,
                Err(err) => {
                    bail!(
                        "Could not establish connection to database in order to read events; {:#?}",
                        err
                    );
                }
            };

            let events = storage::events::list(&mut conn, 0, 0, false)
                .await
                .context(
                    "Could not list historical events\
                 while attempting to populate events from database.",
                )?;

            for storage_event in events {
                let event: Event = storage_event.try_into()?;
                stream.push_back(event);
            }
        }

        Ok(HistoricalEventListener {
            storage: self.storage.clone(),
            last_id_read,
            stream,
        })
    }

    /// Allows caller to emit a new event to the eventbus. Returns the resulting
    /// event once it has been successfully published.
    #[allow(dead_code)]
    pub async fn try_publish(&self, kind: Kind) -> Result<Event> {
        let new_event = Event::new(kind.clone());

        let mut conn = self.storage.write_conn().await.with_context(|| {
            format!(
                "could not publish event for kind '{}'; Database error;",
                new_event.kind,
            )
        })?;

        let new_event_storage: storage::events::Event =
            new_event.clone().try_into().with_context(|| {
                format!(
                    "could not publish event for kind '{}'; could not serialize event into storage",
                    &kind.to_string()
                )
            })?;

        storage::events::insert(&mut conn, &new_event_storage)
            .await
            .with_context(|| {
                format!(
                    "could not publish event for kind '{}'; Database insert error",
                    &kind.to_string()
                )
            })?;

        debug!(kind = %kind, emitted = new_event.emitted, "new event");
        self.broadcast_channel
            .send(new_event.clone())
            .with_context(|| {
                format!(
                    "could not publish event for kind '{}'; Database error;",
                    &kind.to_string(),
                )
            })?;

        Ok(new_event)
    }

    /// Allows caller to emit a new event to the eventbus.
    pub fn publish(self, kind: Kind) -> Event {
        let new_event = Event::new(kind.clone());
        let new_event_clone = new_event.clone();
        tokio::spawn(async move {
            let mut conn = match self.storage.write_conn().await {
                Ok(conn) => conn,
                Err(err) => {
                    error!(error = %err, kind = %new_event.kind,  "Could not publish event; Database error;");
                    return;
                }
            };

            let new_event_storage: storage::events::Event = match new_event.clone().try_into() {
                Ok(event) => event,
                Err(err) => {
                    error!(error = %err, kind = %new_event.kind,  "Could not publish event; Serialization error;");
                    return;
                }
            };

            match storage::events::insert(&mut conn, &new_event_storage).await {
                Ok(_) => {}
                Err(err) => {
                    error!(error = %err, kind = %new_event.kind,  "Could not publish event; Database insert error");
                    return;
                }
            };

            trace!(id = new_event.id, kind = %kind, emitted = new_event.emitted, "new event");
            match self.broadcast_channel.send(new_event.clone()) {
                Ok(_) => {}
                Err(err) => {
                    trace!(
                        error = %err,
                        "No receivers available to receive published message; This is not an actual error unless events\
                        are being missed by known subscribed listeners.",
                    );
                }
            };
        });

        new_event_clone
    }
}

async fn prune_events(storage: &storage::Db, retention: u64) -> Result<(), storage::StorageError> {
    let mut offset = 0;
    let mut total_pruned = 0;

    let mut conn = match storage.write_conn().await {
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
                debug!(
                    emitted = event.emitted,
                    retention = retention,
                    current_time = epoch_milli(),
                    "removed event past retention period"
                );

                total_pruned += 1;

                storage::events::delete(&mut conn, &event.id).await?;
            }
        }

        if events.len() != 50 {
            if total_pruned > 0 {
                info!(
                    retention = retention,
                    total_pruned = total_pruned,
                    "pruned old events"
                );
            }

            return Ok(());
        }

        offset += events.len() as i64;
    }
}

fn is_past_cut_date(event: &storage::events::Event, limit: u64) -> bool {
    let now = epoch_milli();
    let limit = Duration::from_secs(limit).as_millis() as u64;
    let expiry_time = now - limit;

    let emitted = match event.emitted.parse::<u64>() {
        Ok(emitted) => emitted,
        Err(_) => return false,
    };

    emitted < expiry_time
}
