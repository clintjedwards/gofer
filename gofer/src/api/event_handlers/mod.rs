use crate::api::{validate, Api};
use crate::storage;
use futures::Stream;
use gofer_proto::{GetEventRequest, GetEventResponse, ListEventsRequest, ListEventsResponse};
use std::sync::Arc;
use std::{path::Path, pin::Pin};
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
        &self,
        args: ListEventsRequest,
    ) -> Result<Response<ListEventsStream>, Status> {
        todo!()
    }
}
