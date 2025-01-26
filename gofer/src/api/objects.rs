use super::permissioning::{Action, Resource};
use crate::{
    api::{epoch_milli, ApiState, PreflightOptions},
    http_error, object_store, storage,
};
use anyhow::{Context, Result};
use bytes::Bytes;
use dropshot::{
    endpoint, Body, ClientErrorStatusCode, FreeformBody, HttpError, HttpResponseCreated,
    HttpResponseDeleted, HttpResponseOk, Path, Query, RequestContext, StreamingBody,
};
use futures::{Stream, StreamExt, TryFutureExt};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{pin::Pin, sync::Arc};

const LARGE_REQUEST_BODY_MAX_BYTES: usize = 50 * 1_000_000_000; // 50GB

pub fn pipeline_object_store_key(namespace_id: &str, pipeline_id: &str, key: &str) -> String {
    format!("{namespace_id}_{pipeline_id}_{key}")
}

pub fn run_object_store_key(
    namespace_id: &str,
    pipeline_id: &str,
    run_id: u64,
    key: &str,
) -> String {
    format!("{namespace_id}_{pipeline_id}_{run_id}_{key}")
}

pub fn extension_object_store_key(extension_id: &str, key: &str) -> String {
    format!("{extension_id}_{key}")
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PipelineObjectPathArgsRoot {
    /// The unique identifier for the target namespace.
    pub namespace_id: String,

    /// The unique identifier for the target pipeline.
    pub pipeline_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PipelineObjectPathArgs {
    /// The unique identifier for the target namespace.
    pub namespace_id: String,

    /// The unique identifier for the target pipeline.
    pub pipeline_id: String,

    /// The unique identifier for the target object.
    pub key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RunObjectPathArgsRoot {
    /// The unique identifier for the target namespace.
    pub namespace_id: String,

    /// The unique identifier for the target pipeline.
    pub pipeline_id: String,

    /// The unique identifier for the target run.
    pub run_id: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RunObjectPathArgs {
    /// The unique identifier for the target namespace.
    pub namespace_id: String,

    /// The unique identifier for the target pipeline.
    pub pipeline_id: String,

    /// The unique identifier for the target run.
    pub run_id: u64,

    /// The unique identifier for the target object.
    pub key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ExtensionObjectPathArgsRoot {
    /// The unique identifier for the target extension.
    pub extension_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ExtensionObjectPathArgs {
    /// The unique identifier for the target extension.
    pub extension_id: String,

    /// The unique identifier for the target object.
    pub key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
pub struct Object {
    /// The identifier for the object value.
    pub key: String,

    /// Time in epoch milliseconds that this object was registered.
    pub created: u64,
}

impl Object {
    pub fn new(key: &str) -> Self {
        Object {
            key: key.into(),
            created: epoch_milli(),
        }
    }

    fn to_pipeline_object_storage(
        &self,
        namespace_id: &str,
        pipeline_id: &str,
    ) -> Result<storage::object_store_pipeline_keys::ObjectStorePipelineKey> {
        Ok(
            storage::object_store_pipeline_keys::ObjectStorePipelineKey {
                namespace_id: namespace_id.into(),
                pipeline_id: pipeline_id.into(),
                key: self.key.clone(),
                created: self.created.to_string(),
            },
        )
    }

    fn to_run_object_storage(
        &self,
        namespace_id: &str,
        pipeline_id: &str,
        run_id: u64,
    ) -> Result<storage::object_store_run_keys::ObjectStoreRunKey> {
        let run_id_i64: i64 = run_id.try_into().with_context(|| {
            format!(
                "Could not parse field 'run_id' to storage value '{}'",
                run_id
            )
        })?;

        Ok(storage::object_store_run_keys::ObjectStoreRunKey {
            namespace_id: namespace_id.into(),
            pipeline_id: pipeline_id.into(),
            run_id: run_id_i64,
            key: self.key.clone(),
            created: self.created.to_string(),
        })
    }

    fn to_extension_object_storage(
        &self,
        extension_id: &str,
    ) -> Result<storage::object_store_extension_keys::ObjectStoreExtensionKey> {
        Ok(
            storage::object_store_extension_keys::ObjectStoreExtensionKey {
                extension_id: extension_id.into(),
                key: self.key.clone(),
                created: self.created.to_string(),
            },
        )
    }
}

impl TryFrom<storage::object_store_run_keys::ObjectStoreRunKey> for Object {
    type Error = anyhow::Error;

    fn try_from(value: storage::object_store_run_keys::ObjectStoreRunKey) -> Result<Self> {
        let created = value.created.parse::<u64>().with_context(|| {
            format!(
                "Could not parse field 'created' from storage value '{}'",
                value.created
            )
        })?;

        Ok(Object {
            key: value.key,
            created,
        })
    }
}

impl TryFrom<storage::object_store_pipeline_keys::ObjectStorePipelineKey> for Object {
    type Error = anyhow::Error;

    fn try_from(
        value: storage::object_store_pipeline_keys::ObjectStorePipelineKey,
    ) -> Result<Self> {
        let created = value.created.parse::<u64>().with_context(|| {
            format!(
                "Could not parse field 'created' from storage value '{}'",
                value.created
            )
        })?;

        Ok(Object {
            key: value.key,
            created,
        })
    }
}

impl TryFrom<storage::object_store_extension_keys::ObjectStoreExtensionKey> for Object {
    type Error = anyhow::Error;

    fn try_from(
        value: storage::object_store_extension_keys::ObjectStoreExtensionKey,
    ) -> Result<Self> {
        let created = value.created.parse::<u64>().with_context(|| {
            format!(
                "Could not parse field 'created' from storage value '{}'",
                value.created
            )
        })?;

        Ok(Object {
            key: value.key,
            created,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ListRunObjectsResponse {
    /// A list of all run objects.
    pub objects: Vec<Object>,
}

/// List all run objects.
#[endpoint(
    method = GET,
    path = "/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/runs/{run_id}/objects",
    tags = ["Objects"],
)]
pub async fn list_run_objects(
    rqctx: RequestContext<Arc<ApiState>>,
    path_params: Path<RunObjectPathArgsRoot>,
) -> Result<HttpResponseOk<ListRunObjectsResponse>, HttpError> {
    let api_state = rqctx.context();
    let path = path_params.into_inner();
    let _req_metadata = api_state
        .preflight_check(
            &rqctx.request,
            PreflightOptions {
                bypass_auth: false,
                admin_only: false,
                allow_anonymous: false,
                resources: vec![
                    Resource::Namespaces(path.namespace_id.clone()),
                    Resource::Pipelines(path.pipeline_id.clone()),
                    Resource::Runs,
                    Resource::Objects,
                ],
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

    let run_id_i64: i64 = match path.run_id.try_into() {
        Ok(id) => id,
        Err(e) => {
            return Err(HttpError::for_bad_request(
                None,
                format!("Could not serialize run id into a valid integer; {:#?}", e),
            ));
        }
    };

    let storage_objects = match storage::object_store_run_keys::list(
        &mut conn,
        &path.namespace_id,
        &path.pipeline_id,
        run_id_i64,
    )
    .await
    {
        Ok(objects) => objects,
        Err(e) => {
            return Err(http_error!(
                "Could not get objects from database",
                hyper::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id.clone(),
                Some(e.into())
            ));
        }
    };

    let mut objects: Vec<Object> = vec![];

    for storage_object in storage_objects {
        let object = Object::try_from(storage_object).map_err(|e| {
            http_error!(
                "Could not parse object from database",
                hyper::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id.clone(),
                Some(e.into())
            )
        })?;

        objects.push(object);
    }

    let resp = ListRunObjectsResponse { objects };
    Ok(HttpResponseOk(resp))
}

/// Get run object by key.
#[endpoint(
    method = GET,
    path = "/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/runs/{run_id}/objects/{key}",
    tags = ["Objects"],
)]
pub async fn get_run_object(
    rqctx: RequestContext<Arc<ApiState>>,
    path_params: Path<RunObjectPathArgs>,
) -> Result<HttpResponseOk<FreeformBody>, HttpError> {
    let api_state = rqctx.context();
    let path = path_params.into_inner();
    let _req_metadata = api_state
        .preflight_check(
            &rqctx.request,
            PreflightOptions {
                bypass_auth: false,
                admin_only: false,
                allow_anonymous: false,
                resources: vec![
                    Resource::Namespaces(path.namespace_id.clone()),
                    Resource::Pipelines(path.pipeline_id.clone()),
                    Resource::Runs,
                    Resource::Objects,
                ],
                action: Action::Read,
            },
        )
        .await?;

    let mut object_stream = api_state
        .object_store
        .get_stream(&run_object_store_key(
            &path.namespace_id,
            &path.pipeline_id,
            path.run_id,
            &path.key,
        ))
        .map_err(|err| {
            if err == object_store::ObjectStoreError::NotFound {
                return HttpError::for_bad_request(None, "Object not found".into());
            };

            http_error!(
                "Could not get object from store",
                hyper::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id.clone(),
                Some(err.into())
            )
        })
        .await?;

    // I'm not quite sure how to solve the problem that the stream itself is send only but the dropshot body
    // requires that it be sync + send.
    // To mitigate this I'm going to instead use an intermediate channel to use as the output end for our
    // stream.
    let (tx, rx) = tokio::sync::mpsc::channel(3);

    // The errors here are http_error but they never actually get back to the main thread, so maybe it's better
    // we remove that. For now we'll keep it since http_error allows us to log internally.
    tokio::spawn(async move {
        while let Some(chunk) = object_stream.next().await {
            let chunk = match chunk {
                Ok(chunk) => chunk,
                Err(e) => {
                    return Err(http_error!(
                        "Error reading object chunk",
                        hyper::StatusCode::INTERNAL_SERVER_ERROR,
                        rqctx.request_id.clone(),
                        Some(e.into())
                    ));
                }
            };

            if let Err(e) = tx.send(chunk).await {
                return Err(http_error!(
                    "Internal error sending chunks",
                    hyper::StatusCode::INTERNAL_SERVER_ERROR,
                    rqctx.request_id.clone(),
                    Some(e.into())
                ));
            };
        }

        Ok(())
    });

    // This incantation is also very confusing but all we're doing is massaging the stream type into a type that can
    // actually be used by dropshot. This requires meeting the requirements of `http_body::Body`.
    let stream_body_test = tokio_stream::wrappers::ReceiverStream::new(rx).map(|chunk| {
        Ok::<hyper::body::Frame<Bytes>, std::convert::Infallible>(hyper::body::Frame::data(chunk))
    });

    let stream_body = http_body_util::StreamBody::new(stream_body_test);

    Ok(HttpResponseOk(FreeformBody(Body::wrap(stream_body))))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PutRunObjectResponse {
    /// Information about the object created.
    pub object: Object,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PutRunObjectParams {
    /// Overwrite a value of a object if it already exists.
    pub force: bool,
}

/// Insert a new object into the run object store.
///
/// Overwrites can be performed by passing the `force` query param.
#[endpoint(
    method = POST,
    path = "/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/runs/{run_id}/objects/{key}",
    tags = ["Objects"],
    request_body_max_bytes = LARGE_REQUEST_BODY_MAX_BYTES,
)]
pub async fn put_run_object(
    rqctx: RequestContext<Arc<ApiState>>,
    path_params: Path<RunObjectPathArgs>,
    query_params: Query<PutRunObjectParams>,
    body: StreamingBody,
) -> Result<HttpResponseCreated<PutRunObjectResponse>, HttpError> {
    let api_state = rqctx.context();
    let path = path_params.into_inner();
    let query = query_params.into_inner();
    let _req_metadata = api_state
        .preflight_check(
            &rqctx.request,
            PreflightOptions {
                bypass_auth: false,
                admin_only: false,
                allow_anonymous: false,
                resources: vec![
                    Resource::Namespaces(path.namespace_id.clone()),
                    Resource::Pipelines(path.pipeline_id.clone()),
                    Resource::Runs,
                    Resource::Objects,
                ],
                action: Action::Write,
            },
        )
        .await?;

    let key = path.key;
    let force = query.force;
    let object_store_key =
        run_object_store_key(&path.namespace_id, &path.pipeline_id, path.run_id, &key);

    let object_exists = api_state
        .object_store
        .exists(&object_store_key)
        .await
        .map_err(|e| {
            http_error!(
                "Could not query object store for object",
                hyper::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id.clone(),
                Some(e.into())
            )
        })?;

    if object_exists && !force {
        return Err(http_error!(
            "Object found but 'force' query param not set",
            hyper::StatusCode::BAD_REQUEST,
            rqctx.request_id.clone(),
            None
        ));
    }

    // We create a channel here to use as a stream buffer.
    // We'll hold the tx side of this and then give the rx side to the put_stream function so that it can create
    // the file incrementally.
    let (tx, rx) = tokio::sync::mpsc::channel(3);

    let stream: Pin<Box<dyn Stream<Item = Bytes> + Send>> =
        Box::pin(tokio_stream::wrappers::ReceiverStream::new(rx));

    // Next we need to upload the file using the put_stream function for our object store. To do that in parallel
    // we spawn a new tokio task, which we hold the handle for to collect the error code afterwards.
    let api_state_clone = api_state.clone();
    let upload_handle = tokio::spawn(async move {
        api_state_clone
            .object_store
            .put_stream(&object_store_key, stream)
            .await
    });

    let body_stream = body.into_stream();
    tokio::pin!(body_stream);

    while let Some(chunk) = body_stream.next().await {
        let chunk = chunk.map_err(|e| {
            http_error!(
                "Internal error creating new object; Could not successfully send chunks to object store",
                hyper::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id.clone(),
                Some(e.into())
            )
        })?;

        if let Err(e) = tx.send(chunk).await {
            return Err(http_error!(
                "Internal error creating new object; Could not successfully send chunks to object store",
                hyper::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id.clone(),
                Some(e.into())
            ));
        }
    }

    // Manually drop the tx side to signal to the rx side that we're done sending and it can return.
    drop(tx);

    // Now that we've closed the tx end we can wait for the put_stream func to finish.
    let upload_result = upload_handle.await.map_err(|e| {
        http_error!(
            "Could not create upload thread for object store",
            hyper::StatusCode::INTERNAL_SERVER_ERROR,
            rqctx.request_id.clone(),
            Some(e.into())
        )
    })?;

    if let Err(e) = upload_result {
        return Err(http_error!(
            "Could not successfully upload object; Object store reported errors",
            hyper::StatusCode::INTERNAL_SERVER_ERROR,
            rqctx.request_id.clone(),
            Some(e.into())
        ));
    }

    // If all of this executed successfully then we can write to the main datastore that this was successful.
    let mut conn = match api_state.storage.write_conn().await {
        Ok(conn) => conn,
        Err(e) => {
            return Err(http_error!(
                "Could not open connection to database",
                hyper::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id.clone(),
                Some(e.into())
            ));
        }
    };

    let new_object = Object::new(&key);

    let new_object_storage = match new_object.to_run_object_storage(
        &path.namespace_id,
        &path.pipeline_id,
        path.run_id,
    ) {
        Ok(object) => object,
        Err(e) => {
            return Err(http_error!(
                "Could not serialize object for database",
                hyper::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id.clone(),
                Some(e.into())
            ));
        }
    };

    if let Err(e) = storage::object_store_run_keys::insert(&mut conn, &new_object_storage).await {
        match e {
            storage::StorageError::Exists => {
                if !force {
                    return Err(HttpError::for_client_error(
                        None,
                        ClientErrorStatusCode::CONFLICT,
                        "object entry already exists".into(),
                    ));
                }
            }
            _ => {
                return Err(http_error!(
                    "Could not insert object into database",
                    hyper::StatusCode::INTERNAL_SERVER_ERROR,
                    rqctx.request_id.clone(),
                    Some(e.into())
                ));
            }
        }
    };

    let resp = PutRunObjectResponse { object: new_object };

    Ok(HttpResponseCreated(resp))
}

/// Delete run object by key.
#[endpoint(
    method = DELETE,
    path = "/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/runs/{run_id}/objects/{key}",
    tags = ["Objects"],
)]
pub async fn delete_run_object(
    rqctx: RequestContext<Arc<ApiState>>,
    path_params: Path<RunObjectPathArgs>,
) -> Result<HttpResponseDeleted, HttpError> {
    let api_state = rqctx.context();
    let path = path_params.into_inner();
    let _req_metadata = api_state
        .preflight_check(
            &rqctx.request,
            PreflightOptions {
                bypass_auth: false,
                allow_anonymous: false,
                admin_only: false,
                resources: vec![
                    Resource::Namespaces(path.namespace_id.clone()),
                    Resource::Pipelines(path.pipeline_id.clone()),
                    Resource::Runs,
                    Resource::Objects,
                ],
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

    let run_id_i64: i64 = match path.run_id.try_into() {
        Ok(id) => id,
        Err(e) => {
            return Err(HttpError::for_bad_request(
                None,
                format!("Could not serialize run id into a valid integer; {:#?}", e),
            ));
        }
    };

    if let Err(e) = storage::object_store_run_keys::delete(
        &mut conn,
        &path.namespace_id,
        &path.pipeline_id,
        run_id_i64,
        &path.key,
    )
    .await
    {
        match e {
            storage::StorageError::NotFound => {
                return Err(HttpError::for_not_found(
                    None,
                    "object for key given does not exist".into(),
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

    if let Err(e) = api_state
        .object_store
        .delete(&run_object_store_key(
            &path.namespace_id,
            &path.pipeline_id,
            path.run_id,
            &path.key,
        ))
        .await
    {
        return Err(http_error!(
            "Could not delete object from store",
            hyper::StatusCode::INTERNAL_SERVER_ERROR,
            rqctx.request_id.clone(),
            Some(e.into())
        ));
    };

    Ok(HttpResponseDeleted())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ListPipelineObjectsResponse {
    /// A list of all pipeline objects.
    pub objects: Vec<Object>,
}

/// List all pipeline objects.
#[endpoint(
    method = GET,
    path = "/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/objects",
    tags = ["Objects"],
)]
pub async fn list_pipeline_objects(
    rqctx: RequestContext<Arc<ApiState>>,
    path_params: Path<PipelineObjectPathArgsRoot>,
) -> Result<HttpResponseOk<ListPipelineObjectsResponse>, HttpError> {
    let api_state = rqctx.context();
    let path = path_params.into_inner();
    let _req_metadata = api_state
        .preflight_check(
            &rqctx.request,
            PreflightOptions {
                bypass_auth: false,
                allow_anonymous: false,
                admin_only: false,
                resources: vec![
                    Resource::Namespaces(path.namespace_id.clone()),
                    Resource::Pipelines(path.pipeline_id.clone()),
                    Resource::Objects,
                ],
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

    let storage_objects = match storage::object_store_pipeline_keys::list(
        &mut conn,
        &path.namespace_id,
        &path.pipeline_id,
    )
    .await
    {
        Ok(objects) => objects,
        Err(e) => {
            return Err(http_error!(
                "Could not get objects from database",
                hyper::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id.clone(),
                Some(e.into())
            ));
        }
    };

    let mut objects: Vec<Object> = vec![];

    for storage_object in storage_objects {
        let object = Object::try_from(storage_object).map_err(|e| {
            http_error!(
                "Could not parse object from database",
                hyper::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id.clone(),
                Some(e.into())
            )
        })?;

        objects.push(object);
    }

    let resp = ListPipelineObjectsResponse { objects };
    Ok(HttpResponseOk(resp))
}

/// Get pipeline object by key.
#[endpoint(
    method = GET,
    path = "/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/objects/{key}",
    tags = ["Objects"],
)]
pub async fn get_pipeline_object(
    rqctx: RequestContext<Arc<ApiState>>,
    path_params: Path<PipelineObjectPathArgs>,
) -> Result<HttpResponseOk<FreeformBody>, HttpError> {
    let api_state = rqctx.context();
    let path = path_params.into_inner();
    let _req_metadata = api_state
        .preflight_check(
            &rqctx.request,
            PreflightOptions {
                allow_anonymous: false,
                bypass_auth: false,
                admin_only: false,
                resources: vec![
                    Resource::Namespaces(path.namespace_id.clone()),
                    Resource::Pipelines(path.pipeline_id.clone()),
                    Resource::Objects,
                ],
                action: Action::Read,
            },
        )
        .await?;

    let mut object_stream = api_state
        .object_store
        .get_stream(&pipeline_object_store_key(
            &path.namespace_id,
            &path.pipeline_id,
            &path.key,
        ))
        .map_err(|err| {
            if err == object_store::ObjectStoreError::NotFound {
                return HttpError::for_bad_request(None, "Object not found".into());
            };

            http_error!(
                "Could not get object from store",
                hyper::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id.clone(),
                Some(err.into())
            )
        })
        .await?;

    // I'm not quite sure how to solve the problem that the stream itself is send only but the dropshot body
    // requires that it be sync + send.
    // To mitigate this I'm going to instead use an intermediate channel to use as the output end for our
    // stream.
    let (tx, rx) = tokio::sync::mpsc::channel(3);

    // The errors here are http_error but they never actually get back to the main thread, so maybe it's better
    // we remove that. For now we'll keep it since http_error allows us to log internally.
    tokio::spawn(async move {
        while let Some(chunk) = object_stream.next().await {
            let chunk = match chunk {
                Ok(chunk) => chunk,
                Err(e) => {
                    return Err(http_error!(
                        "Error reading object chunk",
                        hyper::StatusCode::INTERNAL_SERVER_ERROR,
                        rqctx.request_id.clone(),
                        Some(e.into())
                    ));
                }
            };

            if let Err(e) = tx.send(chunk).await {
                return Err(http_error!(
                    "Internal error sending chunks",
                    hyper::StatusCode::INTERNAL_SERVER_ERROR,
                    rqctx.request_id.clone(),
                    Some(e.into())
                ));
            };
        }

        Ok(())
    });

    // This incantation is also very confusing but all we're doing is massaging the stream type into a type that can
    // actually be used by dropshot. This requires meeting the requirements of `http_body::Body`.
    let stream_body_test = tokio_stream::wrappers::ReceiverStream::new(rx).map(|chunk| {
        Ok::<hyper::body::Frame<Bytes>, std::convert::Infallible>(hyper::body::Frame::data(chunk))
    });

    let stream_body = http_body_util::StreamBody::new(stream_body_test);

    Ok(HttpResponseOk(FreeformBody(Body::wrap(stream_body))))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PutPipelineObjectResponse {
    /// Information about the object created.
    pub object: Object,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PutPipelineObjectParams {
    /// Overwrite a value of a object if it already exists.
    pub force: bool,
}

/// Insert a new object into the pipeline object store.
#[endpoint(
    method = POST,
    path = "/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/objects/{key}",
    tags = ["Objects"],
    request_body_max_bytes = LARGE_REQUEST_BODY_MAX_BYTES,
)]
pub async fn put_pipeline_object(
    rqctx: RequestContext<Arc<ApiState>>,
    path_params: Path<PipelineObjectPathArgs>,
    query_params: Query<PutPipelineObjectParams>,
    body: StreamingBody,
) -> Result<HttpResponseCreated<PutPipelineObjectResponse>, HttpError> {
    let api_state = rqctx.context();
    let path = path_params.into_inner();
    let query = query_params.into_inner();
    let _req_metadata = api_state
        .preflight_check(
            &rqctx.request,
            PreflightOptions {
                bypass_auth: false,
                admin_only: false,
                allow_anonymous: false,
                resources: vec![
                    Resource::Namespaces(path.namespace_id.clone()),
                    Resource::Pipelines(path.pipeline_id.clone()),
                    Resource::Objects,
                ],
                action: Action::Write,
            },
        )
        .await?;

    let key = path.key;
    let force = query.force;
    let object_store_key = pipeline_object_store_key(&path.namespace_id, &path.pipeline_id, &key);

    let object_exists = api_state
        .object_store
        .exists(&object_store_key)
        .await
        .map_err(|e| {
            http_error!(
                "Could not query object store for object",
                hyper::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id.clone(),
                Some(e.into())
            )
        })?;

    if object_exists && !force {
        return Err(http_error!(
            "Object found but 'force' query param not set",
            hyper::StatusCode::BAD_REQUEST,
            rqctx.request_id.clone(),
            None
        ));
    }

    // We create a channel here to use as a stream buffer.
    // We'll hold the tx side of this and then give the rx side to the put_stream function so that it can create
    // the file incrementally.
    let (tx, rx) = tokio::sync::mpsc::channel(3);

    let stream: Pin<Box<dyn Stream<Item = Bytes> + Send>> =
        Box::pin(tokio_stream::wrappers::ReceiverStream::new(rx));

    // Next we need to upload the file using the put_stream function for our object store. To do that in parallel
    // we spawn a new tokio task, which we hold the handle for to collect the error code afterwards.
    let api_state_clone = api_state.clone();
    let upload_handle = tokio::spawn(async move {
        api_state_clone
            .object_store
            .put_stream(&object_store_key, stream)
            .await
    });

    let body_stream = body.into_stream();
    tokio::pin!(body_stream);

    while let Some(chunk) = body_stream.next().await {
        let chunk = chunk.map_err(|e| {
            http_error!(
                "Internal error creating new object; Could not successfully send chunks to object store",
                hyper::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id.clone(),
                Some(e.into())
            )
        })?;

        if let Err(e) = tx.send(chunk).await {
            return Err(http_error!(
                "Internal error creating new object; Could not successfully send chunks to object store",
                hyper::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id.clone(),
                Some(e.into())
            ));
        }
    }

    // Manually drop the tx side to signal to the rx side that we're done sending and it can return.
    drop(tx);

    // Now that we've closed the tx end we can wait for the put_stream func to finish.
    let upload_result = upload_handle.await.map_err(|e| {
        http_error!(
            "Could not create upload thread for object store",
            hyper::StatusCode::INTERNAL_SERVER_ERROR,
            rqctx.request_id.clone(),
            Some(e.into())
        )
    })?;

    if let Err(e) = upload_result {
        return Err(http_error!(
            "Could not successfully upload object; Object store reported errors",
            hyper::StatusCode::INTERNAL_SERVER_ERROR,
            rqctx.request_id.clone(),
            Some(e.into())
        ));
    }

    // If all of this executed successfully then we can write to the main datastore that this was successful.
    let mut conn = match api_state.storage.write_conn().await {
        Ok(conn) => conn,
        Err(e) => {
            return Err(http_error!(
                "Could not open connection to database",
                hyper::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id.clone(),
                Some(e.into())
            ));
        }
    };

    let new_object = Object::new(&key);

    let new_object_storage =
        match new_object.to_pipeline_object_storage(&path.namespace_id, &path.pipeline_id) {
            Ok(object) => object,
            Err(e) => {
                return Err(http_error!(
                    "Could not serialize object for database",
                    hyper::StatusCode::INTERNAL_SERVER_ERROR,
                    rqctx.request_id.clone(),
                    Some(e.into())
                ));
            }
        };

    if let Err(e) =
        storage::object_store_pipeline_keys::insert(&mut conn, &new_object_storage).await
    {
        match e {
            storage::StorageError::Exists => {
                if !force {
                    return Err(HttpError::for_client_error(
                        None,
                        ClientErrorStatusCode::CONFLICT,
                        "object entry already exists".into(),
                    ));
                }
            }
            _ => {
                return Err(http_error!(
                    "Could not insert object into database",
                    hyper::StatusCode::INTERNAL_SERVER_ERROR,
                    rqctx.request_id.clone(),
                    Some(e.into())
                ));
            }
        }
    };

    let resp = PutPipelineObjectResponse { object: new_object };

    Ok(HttpResponseCreated(resp))

    // // TODO(): Implement pipeline object limits
    // let _ = api_state.config.object_store.pipeline_object_limit;
}

/// Delete pipeline object by key.
#[endpoint(
    method = DELETE,
    path = "/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/objects/{key}",
    tags = ["Objects"],
)]
pub async fn delete_pipeline_object(
    rqctx: RequestContext<Arc<ApiState>>,
    path_params: Path<PipelineObjectPathArgs>,
) -> Result<HttpResponseDeleted, HttpError> {
    let api_state = rqctx.context();
    let path = path_params.into_inner();
    let _req_metadata = api_state
        .preflight_check(
            &rqctx.request,
            PreflightOptions {
                bypass_auth: false,
                admin_only: false,
                allow_anonymous: false,
                resources: vec![
                    Resource::Namespaces(path.namespace_id.clone()),
                    Resource::Pipelines(path.pipeline_id.clone()),
                    Resource::Objects,
                ],
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

    if let Err(e) = storage::object_store_pipeline_keys::delete(
        &mut conn,
        &path.namespace_id,
        &path.pipeline_id,
        &path.key,
    )
    .await
    {
        match e {
            storage::StorageError::NotFound => {
                return Err(HttpError::for_not_found(
                    None,
                    "object for key given does not exist".into(),
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

    if let Err(e) = api_state
        .object_store
        .delete(&pipeline_object_store_key(
            &path.namespace_id,
            &path.pipeline_id,
            &path.key,
        ))
        .await
    {
        return Err(http_error!(
            "Could not delete object from store",
            hyper::StatusCode::INTERNAL_SERVER_ERROR,
            rqctx.request_id.clone(),
            Some(e.into())
        ));
    };

    Ok(HttpResponseDeleted())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ListExtensionObjectsResponse {
    /// A list of all extension objects.
    pub objects: Vec<Object>,
}

/// List all extension objects.
#[endpoint(
    method = GET,
    path = "/api/extensions/{extension_id}/objects",
    tags = ["Objects"],
)]
pub async fn list_extension_objects(
    rqctx: RequestContext<Arc<ApiState>>,
    path_params: Path<ExtensionObjectPathArgsRoot>,
) -> Result<HttpResponseOk<ListExtensionObjectsResponse>, HttpError> {
    let api_state = rqctx.context();
    let path = path_params.into_inner();
    let _req_metadata = api_state
        .preflight_check(
            &rqctx.request,
            PreflightOptions {
                bypass_auth: false,
                admin_only: false,
                allow_anonymous: false,
                resources: vec![
                    Resource::Extensions(path.extension_id.clone()),
                    Resource::Objects,
                ],
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

    let storage_objects =
        match storage::object_store_extension_keys::list(&mut conn, &path.extension_id).await {
            Ok(objects) => objects,
            Err(e) => {
                return Err(http_error!(
                    "Could not get objects from database",
                    hyper::StatusCode::INTERNAL_SERVER_ERROR,
                    rqctx.request_id.clone(),
                    Some(e.into())
                ));
            }
        };

    let mut objects: Vec<Object> = vec![];

    for storage_object in storage_objects {
        let object = Object::try_from(storage_object).map_err(|e| {
            http_error!(
                "Could not parse object from database",
                hyper::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id.clone(),
                Some(e.into())
            )
        })?;

        objects.push(object);
    }

    let resp = ListExtensionObjectsResponse { objects };
    Ok(HttpResponseOk(resp))
}

/// Get extension object by key.
#[endpoint(
    method = GET,
    path = "/api/extensions/{extension_id}/objects/{key}",
    tags = ["Objects"],
)]
pub async fn get_extension_object(
    rqctx: RequestContext<Arc<ApiState>>,
    path_params: Path<ExtensionObjectPathArgs>,
) -> Result<HttpResponseOk<FreeformBody>, HttpError> {
    let api_state = rqctx.context();
    let path = path_params.into_inner();
    let _req_metadata = api_state
        .preflight_check(
            &rqctx.request,
            PreflightOptions {
                bypass_auth: false,
                admin_only: false,
                allow_anonymous: false,
                resources: vec![
                    Resource::Extensions(path.extension_id.clone()),
                    Resource::Objects,
                ],
                action: Action::Read,
            },
        )
        .await?;

    let mut object_stream = api_state
        .object_store
        .get_stream(&extension_object_store_key(&path.extension_id, &path.key))
        .map_err(|err| {
            if err == object_store::ObjectStoreError::NotFound {
                return HttpError::for_bad_request(None, "Object not found".into());
            };

            http_error!(
                "Could not get object from store",
                hyper::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id.clone(),
                Some(err.into())
            )
        })
        .await?;

    // I'm not quite sure how to solve the problem that the stream itself is send only but the dropshot body
    // requires that it be sync + send.
    // To mitigate this I'm going to instead use an intermediate channel to use as the output end for our
    // stream.
    let (tx, rx) = tokio::sync::mpsc::channel(3);

    // The errors here are http_error but they never actually get back to the main thread, so maybe it's better
    // we remove that. For now we'll keep it since http_error allows us to log internally.
    tokio::spawn(async move {
        while let Some(chunk) = object_stream.next().await {
            let chunk = match chunk {
                Ok(chunk) => chunk,
                Err(e) => {
                    return Err(http_error!(
                        "Error reading object chunk",
                        hyper::StatusCode::INTERNAL_SERVER_ERROR,
                        rqctx.request_id.clone(),
                        Some(e.into())
                    ));
                }
            };

            if let Err(e) = tx.send(chunk).await {
                return Err(http_error!(
                    "Internal error sending chunks",
                    hyper::StatusCode::INTERNAL_SERVER_ERROR,
                    rqctx.request_id.clone(),
                    Some(e.into())
                ));
            };
        }

        Ok(())
    });

    // This incantation is also very confusing but all we're doing is massaging the stream type into a type that can
    // actually be used by dropshot. This requires meeting the requirements of `http_body::Body`.
    let stream_body_test = tokio_stream::wrappers::ReceiverStream::new(rx).map(|chunk| {
        Ok::<hyper::body::Frame<Bytes>, std::convert::Infallible>(hyper::body::Frame::data(chunk))
    });

    let stream_body = http_body_util::StreamBody::new(stream_body_test);

    Ok(HttpResponseOk(FreeformBody(Body::wrap(stream_body))))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PutExtensionObjectResponse {
    /// Information about the object created.
    pub object: Object,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PutExtensionObjectParams {
    /// Overwrite a value of a object if it already exists.
    pub force: bool,
}

/// Insert a new object into the extension object store.
#[endpoint(
    method = POST,
    path = "/api/extensions/{extension_id}/objects/{key}",
    tags = ["Objects"],
    request_body_max_bytes = LARGE_REQUEST_BODY_MAX_BYTES,
)]
pub async fn put_extension_object(
    rqctx: RequestContext<Arc<ApiState>>,
    path_params: Path<ExtensionObjectPathArgs>,
    query_params: Query<PutExtensionObjectParams>,
    body: StreamingBody,
) -> Result<HttpResponseCreated<PutExtensionObjectResponse>, HttpError> {
    let api_state = rqctx.context();
    let path = path_params.into_inner();
    let query = query_params.into_inner();
    let _req_metadata = api_state
        .preflight_check(
            &rqctx.request,
            PreflightOptions {
                bypass_auth: false,
                admin_only: false,
                allow_anonymous: false,
                resources: vec![
                    Resource::Extensions(path.extension_id.clone()),
                    Resource::Objects,
                ],
                action: Action::Write,
            },
        )
        .await?;

    let key = path.key;
    let force = query.force;
    let object_store_key = extension_object_store_key(&path.extension_id, &key);

    let object_exists = api_state
        .object_store
        .exists(&object_store_key)
        .await
        .map_err(|e| {
            http_error!(
                "Could not query object store for object",
                hyper::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id.clone(),
                Some(e.into())
            )
        })?;

    if object_exists && !force {
        return Err(http_error!(
            "Object found but 'force' query param not set",
            hyper::StatusCode::BAD_REQUEST,
            rqctx.request_id.clone(),
            None
        ));
    }

    // We create a channel here to use as a stream buffer.
    // We'll hold the tx side of this and then give the rx side to the put_stream function so that it can create
    // the file incrementally.
    let (tx, rx) = tokio::sync::mpsc::channel(3);

    let stream: Pin<Box<dyn Stream<Item = Bytes> + Send>> =
        Box::pin(tokio_stream::wrappers::ReceiverStream::new(rx));

    // Next we need to upload the file using the put_stream function for our object store. To do that in parallel
    // we spawn a new tokio task, which we hold the handle for to collect the error code afterwards.
    let api_state_clone = api_state.clone();
    let upload_handle = tokio::spawn(async move {
        api_state_clone
            .object_store
            .put_stream(&object_store_key, stream)
            .await
    });

    let body_stream = body.into_stream();
    tokio::pin!(body_stream);

    while let Some(chunk) = body_stream.next().await {
        let chunk = chunk.map_err(|e| {
            http_error!(
                "Internal error creating new object; Could not successfully send chunks to object store",
                hyper::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id.clone(),
                Some(e.into())
            )
        })?;

        if let Err(e) = tx.send(chunk).await {
            return Err(http_error!(
                "Internal error creating new object; Could not successfully send chunks to object store",
                hyper::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id.clone(),
                Some(e.into())
            ));
        }
    }

    // Manually drop the tx side to signal to the rx side that we're done sending and it can return.
    drop(tx);

    // Now that we've closed the tx end we can wait for the put_stream func to finish.
    let upload_result = upload_handle.await.map_err(|e| {
        http_error!(
            "Could not create upload thread for object store",
            hyper::StatusCode::INTERNAL_SERVER_ERROR,
            rqctx.request_id.clone(),
            Some(e.into())
        )
    })?;

    if let Err(e) = upload_result {
        return Err(http_error!(
            "Could not successfully upload object; Object store reported errors",
            hyper::StatusCode::INTERNAL_SERVER_ERROR,
            rqctx.request_id.clone(),
            Some(e.into())
        ));
    }

    // If all of this executed successfully then we can write to the main datastore that this was successful.
    let mut conn = match api_state.storage.write_conn().await {
        Ok(conn) => conn,
        Err(e) => {
            return Err(http_error!(
                "Could not open connection to database",
                hyper::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id.clone(),
                Some(e.into())
            ));
        }
    };

    let new_object = Object::new(&key);

    let new_object_storage = match new_object.to_extension_object_storage(&path.extension_id) {
        Ok(object) => object,
        Err(e) => {
            return Err(http_error!(
                "Could not parse objects for database",
                hyper::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id.clone(),
                Some(e.into())
            ));
        }
    };

    if let Err(e) =
        storage::object_store_extension_keys::insert(&mut conn, &new_object_storage).await
    {
        match e {
            storage::StorageError::Exists => {
                if !force {
                    return Err(HttpError::for_client_error(
                        None,
                        ClientErrorStatusCode::CONFLICT,
                        "object entry already exists".into(),
                    ));
                }
            }
            _ => {
                return Err(http_error!(
                    "Could not insert object into database",
                    hyper::StatusCode::INTERNAL_SERVER_ERROR,
                    rqctx.request_id.clone(),
                    Some(e.into())
                ));
            }
        }
    };

    let resp = PutExtensionObjectResponse { object: new_object };

    Ok(HttpResponseCreated(resp))
}

/// Delete extension object by key.
#[endpoint(
    method = DELETE,
    path = "/api/extensions/{extension_id}/objects/{key}",
    tags = ["Objects"],
)]
pub async fn delete_extension_object(
    rqctx: RequestContext<Arc<ApiState>>,
    path_params: Path<ExtensionObjectPathArgs>,
) -> Result<HttpResponseDeleted, HttpError> {
    let api_state = rqctx.context();
    let path = path_params.into_inner();
    let _req_metadata = api_state
        .preflight_check(
            &rqctx.request,
            PreflightOptions {
                bypass_auth: false,
                admin_only: false,
                allow_anonymous: false,
                resources: vec![
                    Resource::Extensions(path.extension_id.clone()),
                    Resource::Objects,
                ],
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

    if let Err(e) =
        storage::object_store_extension_keys::delete(&mut conn, &path.extension_id, &path.key).await
    {
        match e {
            storage::StorageError::NotFound => {
                return Err(HttpError::for_not_found(
                    None,
                    "object for key given does not exist".into(),
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

    if let Err(e) = api_state
        .object_store
        .delete(&extension_object_store_key(&path.extension_id, &path.key))
        .await
    {
        return Err(http_error!(
            "Could not delete object from store",
            hyper::StatusCode::INTERNAL_SERVER_ERROR,
            rqctx.request_id.clone(),
            Some(e.into())
        ));
    };

    Ok(HttpResponseDeleted())
}
