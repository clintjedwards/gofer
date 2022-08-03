use crate::api::{validate, Api};
use crate::storage;
use futures::Stream;
use gofer_models::event::KindDiscriminant;
use gofer_proto::{GetEventRequest, GetEventResponse, ListEventsRequest, ListEventsResponse};
use slog_scope::error;
use std::sync::Arc;
use std::{path::Path, pin::Pin};
use tokio::sync::mpsc::{self, channel, Sender};
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Response, Status};

type ListEventsStream = Pin<Box<dyn Stream<Item = Result<ListEventsResponse, Status>> + Send>>;

impl Api {
    pub async fn get_event_handler(
        &self,
        args: GetEventRequest,
    ) -> Result<Response<GetEventResponse>, Status> {
        validate::arg("id", args.id, vec![validate::not_zero_num])?;

        let mut conn = self
            .storage
            .conn()
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        let event = storage::events::get(&mut conn, args.id)
            .await
            .map_err(|e| match e {
                storage::StorageError::NotFound => {
                    Status::not_found(format!("event with id '{}' does not exist", args.id))
                }
                _ => Status::internal(e.to_string()),
            })?;

        Ok(Response::new(GetEventResponse {
            event: Some(event.into()),
        }))
    }

    pub async fn list_events_handler(
        self: Arc<Self>,
        args: ListEventsRequest,
    ) -> Result<Response<ListEventsStream>, Status> {
        // We create a channel we will eventually turn into
        // a stream we can use and pass back to the client.
        let (tx, rx) = mpsc::channel(128);
        let output_stream = ReceiverStream::new(rx);

        tokio::spawn(async move { self.stream_events(args.reverse, args.follow, tx).await });

        Ok(Response::new(Box::pin(output_stream)))
    }

    async fn stream_events(
        &self,
        reverse: bool,
        follow: bool,
        input: Sender<Result<ListEventsResponse, Status>>,
    ) {
        let subscription = match self.event_bus.subscribe(KindDiscriminant::Any).await {
            Ok(sub) => sub,
            Err(e) => {
                error!(
                    "could not stream events to client; could not get subscription; {:?}",
                    e
                );
                return;
            }
        };

        let mut conn = match self.storage.conn().await {
            Ok(conn) => conn,
            Err(e) => {
                error!(
                    "could not stream events to client; could not get events from db; {:?}",
                    e
                );
                return;
            }
        };

        let mut offset = 0;
        let mut last_event_id = 0;

        loop {
            let events = match storage::events::list(&mut conn, offset, 10, reverse).await {
                Ok(evt) => evt,
                Err(e) => {
                    error!(
                        "could not stream events to client; could not get events from db; {:?}",
                        e
                    );
                    return;
                }
            };

            if events.is_empty() {
                break;
            }

            for event in events {
                if let Err(e) = input
                    .send(Result::<ListEventsResponse, Status>::Ok(
                        ListEventsResponse {
                            event: Some(event.clone().into()),
                        },
                    ))
                    .await
                {
                    error!("could not stream event to client; error in send; {:?}", e);
                    return;
                }
                last_event_id = event.id;
            }

            offset += 10
        }

        // If the user wants the events in reverse order there is no need to wait for incoming
        // events, so once we finish the historical events we just exit.
        if reverse || !follow {
            return;
        }

        #[allow(clippy::for_loops_over_fallibles)]
        for event in subscription.receiver.recv() {
            // Because we get a subscription before we start iterating over the events in the database
            // we need to make sure that we don't print repeat events. To easily do this we just skip
            // over all events which are less than the last event we sent.
            if event.id < last_event_id {
                continue;
            }

            if let Err(e) = input
                .send(Result::<ListEventsResponse, Status>::Ok(
                    ListEventsResponse {
                        event: Some(event.clone().into()),
                    },
                ))
                .await
            {
                error!("could not stream event to client; error in send; {:?}", e);
                return;
            }
        }

        //TODO(clintjedwards): We need to enable this to be cancelled by both the client
        // and the server shutdown. Currently it doesn't not and hangs the server.
        dbg!("we exited because the client exited here");
    }
}
