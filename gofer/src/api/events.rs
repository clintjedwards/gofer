use super::permissioning::{Action, Resource};
use crate::{
    api::{
        event_utils::{Event, EventListener},
        format_duration, listen_for_terminate_signal, websocket_error, ApiState, PreflightOptions,
    },
    http_error, storage,
};
use anyhow::Result;
use dropshot::{
    channel, endpoint, HttpError, HttpResponseDeleted, HttpResponseOk, Path, Query, RequestContext,
    WebsocketChannelResult, WebsocketConnection,
};
use futures::{SinkExt, StreamExt};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, error};
use tungstenite::{
    protocol::{frame::coding::CloseCode, CloseFrame, Role},
    Message,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct EventPathArgs {
    /// The unique identifier for the target event.
    pub event_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct EventQueryArgs {
    /// If set to true Gofer first exhausts events that have already passed before it starts to stream
    /// new events.
    pub history: Option<bool>,

    /// Reverses the order of events by the time they were emitted. By default Gofer lists events in ascending order;
    /// setting reverse to true causes events to be in descending order.
    pub reverse: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ListEventsResponse {
    /// A list of all events.
    pub events: Vec<Event>,
}

/// List all events.
#[channel(
    protocol = WEBSOCKETS,
    path = "/api/events",
    tags = ["Events"],
)]
pub async fn stream_events(
    rqctx: RequestContext<Arc<ApiState>>,
    query_params: Query<EventQueryArgs>,
    conn: WebsocketConnection,
) -> WebsocketChannelResult {
    let api_state = rqctx.context();
    let query = query_params.into_inner();
    let _req_metadata = api_state
        .preflight_check(
            &rqctx.request,
            PreflightOptions {
                bypass_auth: false,
                admin_only: false,
                allow_anonymous: false,
                resources: vec![Resource::Events],
                action: Action::Read,
            },
        )
        .await?;

    let start_time = std::time::Instant::now();

    let mut ws =
        tokio_tungstenite::WebSocketStream::from_raw_socket(conn.into_inner(), Role::Server, None)
            .await;

    let reverse = query.reverse.unwrap_or_default();
    let history = query.history.unwrap_or_default();

    // The close function for ws is limited to 123 bytes (-2 bytes for the code).
    if !history && reverse {
        let _ = ws
            .close(Some(CloseFrame {
                code: CloseCode::Unsupported,
                reason: "Cannot use params 'history' = false with 'reverse' = true; reverse does not allow \
                streaming causing nothing to be shown".into(),
            }))
            .await;

        return Ok(());
    }

    let mut conn = match api_state.storage.read_conn().await {
        Ok(conn) => conn,
        Err(e) => {
            return Err(websocket_error(
                "Could not open connection to database",
                CloseCode::Error,
                rqctx.request_id.clone(),
                ws,
                Some(e.to_string()),
            )
            .await
            .into());
        }
    };

    // We need to launch two async functions to:
    // * Push events to the user.
    // * Listen for the user closing the connection.
    // * Listen for shutdown signal from main process.
    //
    // The JoinSet below allows us to launch all of the functions and then
    // wait for one of them to return. Since all need to be running
    // or they are all basically useless, we wait for any one of them to finish
    // and then we simply abort the others and then close the stream.

    let mut set: tokio::task::JoinSet<std::result::Result<(), String>> =
        tokio::task::JoinSet::new();

    let (client_write, mut client_read) = ws.split();
    let client_writer = Arc::new(Mutex::new(client_write));
    let client_writer_handle = client_writer.clone();

    let mut event_stream = api_state.event_bus.subscribe_live();

    // Listen for a terminal signal from the main process.
    set.spawn(async move {
        listen_for_terminate_signal().await;
        Err("Server is shutting down".into())
    });

    // Launch thread to collect messages from event sources and push them to the user.
    set.spawn(async move {
        if history {
            let limit = 20;
            let mut offset = 0;

            loop {
                let storage_events =
                    match storage::events::list(&mut conn, offset, limit, reverse).await {
                        Ok(events) => events,
                        Err(err) => {
                            error!(error = %err,"Could not get events from database");
                            return Err("Could not get events from database".into());
                        }
                    };

                // If there are no more events then we can move on to streaming current events.
                if storage_events.is_empty() {
                    break;
                }

                for event in storage_events {
                    let event: Event = match event.try_into() {
                        Ok(event) => event,
                        Err(err) => {
                            error!(error = %err,"Could not parse event object from database");
                            return Err("Could not parse event object from database".into());
                        }
                    };

                    let event_str = match serde_json::to_string(&event) {
                        Ok(event_str) => event_str,
                        Err(err) => {
                            error!(error = %err,"Could not serialize event for sending");
                            return Err("Could not serialize event for sending".into());
                        }
                    };

                    let mut locked_writer = client_writer_handle.lock().await;

                    if let Err(err) = locked_writer.send(Message::text(event_str)).await {
                        error!(error = %err,"Could not send event");
                        return Err("Could not send event".into());
                    }
                }

                offset += limit;
            }
        };

        // It's impossible to stream current events in reverse.
        if !reverse {
            loop {
                match event_stream.next().await {
                    Ok(event) => {
                        let event_str = match serde_json::to_string(&event) {
                            Ok(event_str) => event_str,
                            Err(err) => {
                                error!(error = %err,"Could not serialize event from event string");
                                return Err("Could not serialize event from event string".into());
                            }
                        };

                        let mut locked_writer = client_writer_handle.lock().await;

                        let _ = locked_writer.send(Message::Text(event_str)).await;
                    }
                    Err(e) => {
                        return Err(format!("Server closed connectionx; {:#?}", e));
                    }
                }
            }
        };

        Ok(())
    });

    set.spawn(async move {
        loop {
            if let Some(output) = client_read.next().await {
                match output {
                    Ok(message) => match message {
                        tungstenite::protocol::Message::Close(_) => {
                            break;
                        }
                        _ => {
                            continue;
                        }
                    },
                    Err(_) => {
                        break;
                    }
                }
            }
        }

        Ok(())
    });

    // The first one to finish will return here. We can unwrap the option safely because it only returns a None if there
    // was nothing in the set.
    let result = set.join_next().await.unwrap()?;
    if let Err(err) = result {
        let mut locked_writer = client_writer.lock().await;

        let close_message = Message::Close(Some(tungstenite::protocol::CloseFrame {
            code: tungstenite::protocol::frame::coding::CloseCode::Error,
            reason: err.clone().into(),
        }));

        let _ = locked_writer.send(close_message).await;
        let _ = locked_writer.close().await;
        return Err(err.into());
    }

    set.shutdown().await; // When one finishes we no longer have use for the others, make sure they all shutdown.

    let mut locked_writer = client_writer.lock().await;

    let close_message = Message::Close(Some(tungstenite::protocol::CloseFrame {
        code: tungstenite::protocol::frame::coding::CloseCode::Normal,
        reason: "out of events".into(),
    }));

    let _ = locked_writer.send(close_message).await;
    let _ = locked_writer.close().await;

    debug!(
        duration = format_duration(start_time.elapsed()),
        request_id = rqctx.request_id.clone(),
        "Finished stream_events",
    );
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct GetEventResponse {
    /// The target event.
    pub event: Event,
}

/// Get api event by id.
#[endpoint(
    method = GET,
    path = "/api/events/{event_id}",
    tags = ["Events"],
)]
pub async fn get_event(
    rqctx: RequestContext<Arc<ApiState>>,
    path_params: Path<EventPathArgs>,
) -> Result<HttpResponseOk<GetEventResponse>, HttpError> {
    let api_state = rqctx.context();
    let path = path_params.into_inner();
    let _req_metadata = api_state
        .preflight_check(
            &rqctx.request,
            PreflightOptions {
                bypass_auth: false,
                admin_only: false,
                allow_anonymous: false,
                resources: vec![Resource::Events],
                action: Action::Read,
            },
        )
        .await?;

    let mut conn = match api_state.storage.read_conn().await {
        Ok(conn) => conn,
        Err(e) => {
            return Err(http_error!(
                "Could not open connection to database",
                hyper::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id,
                Some(e.into())
            ));
        }
    };

    let storage_event = match storage::events::get(&mut conn, &path.event_id).await {
        Ok(event) => event,
        Err(e) => match e {
            storage::StorageError::NotFound => {
                return Err(HttpError::for_not_found(None, String::new()));
            }
            _ => {
                return Err(http_error!(
                    "Could not get objects from database",
                    hyper::StatusCode::INTERNAL_SERVER_ERROR,
                    rqctx.request_id.clone(),
                    Some(e.into())
                ));
            }
        },
    };

    let event = Event::try_from(storage_event).map_err(|e| {
        http_error!(
            "Could not parse object from database",
            hyper::StatusCode::INTERNAL_SERVER_ERROR,
            rqctx.request_id.clone(),
            Some(e.into())
        )
    })?;

    let resp = GetEventResponse { event };
    Ok(HttpResponseOk(resp))
}

/// Delete api event by id.
///
/// This route is only accessible by admin tokens.
#[endpoint(
    method = DELETE,
    path = "/api/events/{event_id}",
    tags = ["Events"],
)]
pub async fn delete_event(
    rqctx: RequestContext<Arc<ApiState>>,
    path_params: Path<EventPathArgs>,
) -> Result<HttpResponseDeleted, HttpError> {
    let api_state = rqctx.context();
    let path = path_params.into_inner();
    let _req_metadata = api_state
        .preflight_check(
            &rqctx.request,
            PreflightOptions {
                bypass_auth: false,
                admin_only: true,
                allow_anonymous: false,
                resources: vec![Resource::Events],
                action: Action::Delete,
            },
        )
        .await?;

    let mut conn = match api_state.storage.write_conn().await {
        Ok(conn) => conn,
        Err(e) => {
            return Err(http_error!(
                "Could not open connection to database",
                hyper::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id,
                Some(e.into())
            ));
        }
    };

    if let Err(e) = storage::events::delete(&mut conn, &path.event_id).await {
        match e {
            storage::StorageError::NotFound => {
                return Err(HttpError::for_not_found(
                    None,
                    "event for id given does not exist".into(),
                ));
            }
            _ => {
                return Err(http_error!(
                    "Could not delete object from database",
                    hyper::StatusCode::INTERNAL_SERVER_ERROR,
                    rqctx.request_id.clone(),
                    Some(e.into())
                ));
            }
        }
    };

    Ok(HttpResponseDeleted())
}
