use super::permissioning::{Action, Resource};
use crate::{
    api::{epoch_milli, ApiState, PreflightOptions},
    http_error, object_store, storage,
};
use anyhow::{Context, Result};
use dropshot::{
    endpoint, HttpError, HttpResponseCreated, HttpResponseDeleted, HttpResponseOk, Path,
    RequestContext, TypedBody,
};
use futures::TryFutureExt;
use http::StatusCode;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

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

    let mut conn = match api_state.storage.read_conn() {
        Ok(conn) => conn,
        Err(e) => {
            return Err(http_error!(
                "Could not open connection to database",
                http::StatusCode::INTERNAL_SERVER_ERROR,
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
    ) {
        Ok(objects) => objects,
        Err(e) => {
            return Err(http_error!(
                "Could not get objects from database",
                http::StatusCode::INTERNAL_SERVER_ERROR,
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
                http::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id.clone(),
                Some(e.into())
            )
        })?;

        objects.push(object);
    }

    let resp = ListRunObjectsResponse { objects };
    Ok(HttpResponseOk(resp))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct GetRunObjectResponse {
    /// The requested object data.
    pub object: Vec<u8>,
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
) -> Result<HttpResponseOk<GetRunObjectResponse>, HttpError> {
    let api_state = rqctx.context();
    let path = path_params.into_inner();
    let _req_metadata = api_state
        .preflight_check(
            &rqctx.request,
            PreflightOptions {
                bypass_auth: false,
                admin_only: false,
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

    let object_value = api_state
        .object_store
        .get(&run_object_store_key(
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
                http::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id.clone(),
                Some(err.into())
            )
        })
        .await?;

    let resp = GetRunObjectResponse {
        object: object_value.0,
    };

    Ok(HttpResponseOk(resp))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PutRunObjectRequest {
    /// The name for the object you would like to store.
    pub key: String,

    /// The bytes for the object.
    pub content: Vec<u8>,

    /// Overwrite a value of a object if it already exists.
    pub force: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PutRunObjectResponse {
    /// Information about the object created.
    pub object: Object,
}

/// Insert a new object into the run object store.
#[endpoint(
    method = POST,
    path = "/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/runs/{run_id}/objects",
    tags = ["Objects"],
)]
pub async fn put_run_object(
    rqctx: RequestContext<Arc<ApiState>>,
    path_params: Path<RunObjectPathArgsRoot>,
    body: TypedBody<PutRunObjectRequest>,
) -> Result<HttpResponseCreated<PutRunObjectResponse>, HttpError> {
    let api_state = rqctx.context();
    let path = path_params.into_inner();
    let body = body.into_inner();
    let _req_metadata = api_state
        .preflight_check(
            &rqctx.request,
            PreflightOptions {
                bypass_auth: false,
                admin_only: false,
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

    let mut conn = match api_state.storage.write_conn() {
        Ok(conn) => conn,
        Err(e) => {
            return Err(http_error!(
                "Could not open connection to database",
                http::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id,
                Some(e.into())
            ));
        }
    };

    let new_object = Object::new(&body.key);

    let new_object_storage = match new_object.to_run_object_storage(
        &path.namespace_id,
        &path.pipeline_id,
        path.run_id,
    ) {
        Ok(object) => object,
        Err(e) => {
            return Err(http_error!(
                "Could not serialize object for database",
                http::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id.clone(),
                Some(e.into())
            ));
        }
    };

    if let Err(e) = storage::object_store_run_keys::insert(&mut conn, &new_object_storage) {
        match e {
            storage::StorageError::Exists => {
                return Err(HttpError::for_client_error(
                    None,
                    StatusCode::CONFLICT,
                    "object entry already exists".into(),
                ));
            }
            _ => {
                return Err(http_error!(
                    "Could not insert object into database",
                    http::StatusCode::INTERNAL_SERVER_ERROR,
                    rqctx.request_id.clone(),
                    Some(e.into())
                ));
            }
        }
    };

    if let Err(e) = api_state
        .object_store
        .put(
            &run_object_store_key(
                &path.namespace_id,
                &path.pipeline_id,
                path.run_id,
                &body.key,
            ),
            body.content,
            body.force,
        )
        .await
    {
        return Err(http_error!(
            "Could not insert object into store",
            http::StatusCode::INTERNAL_SERVER_ERROR,
            rqctx.request_id.clone(),
            Some(e.into())
        ));
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

    let mut conn = match api_state.storage.write_conn() {
        Ok(conn) => conn,
        Err(e) => {
            return Err(http_error!(
                "Could not open connection to database",
                http::StatusCode::INTERNAL_SERVER_ERROR,
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
    ) {
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
                    http::StatusCode::INTERNAL_SERVER_ERROR,
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
            http::StatusCode::INTERNAL_SERVER_ERROR,
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

    let mut conn = match api_state.storage.read_conn() {
        Ok(conn) => conn,
        Err(e) => {
            return Err(http_error!(
                "Could not open connection to database",
                http::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id,
                Some(e.into())
            ));
        }
    };

    let storage_objects = match storage::object_store_pipeline_keys::list(
        &mut conn,
        &path.namespace_id,
        &path.pipeline_id,
    ) {
        Ok(objects) => objects,
        Err(e) => {
            return Err(http_error!(
                "Could not get objects from database",
                http::StatusCode::INTERNAL_SERVER_ERROR,
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
                http::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id.clone(),
                Some(e.into())
            )
        })?;

        objects.push(object);
    }

    let resp = ListPipelineObjectsResponse { objects };
    Ok(HttpResponseOk(resp))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct GetPipelineObjectResponse {
    /// The requested object data.
    pub object: Vec<u8>,
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
) -> Result<HttpResponseOk<GetPipelineObjectResponse>, HttpError> {
    let api_state = rqctx.context();
    let path = path_params.into_inner();
    let _req_metadata = api_state
        .preflight_check(
            &rqctx.request,
            PreflightOptions {
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

    let object_value = api_state
        .object_store
        .get(&pipeline_object_store_key(
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
                http::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id.clone(),
                Some(err.into())
            )
        })
        .await?;

    let resp = GetPipelineObjectResponse {
        object: object_value.0,
    };

    Ok(HttpResponseOk(resp))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PutPipelineObjectRequest {
    /// The name for the object you would like to store.
    pub key: String,

    /// The bytes for the object.
    pub content: Vec<u8>,

    /// Overwrite a value of a object if it already exists.
    pub force: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PutPipelineObjectResponse {
    /// Information about the object created.
    pub object: Object,
}

/// Insert a new object into the pipeline object store.
#[endpoint(
    method = POST,
    path = "/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/objects",
    tags = ["Objects"],
)]
pub async fn put_pipeline_object(
    rqctx: RequestContext<Arc<ApiState>>,
    path_params: Path<PipelineObjectPathArgsRoot>,
    body: TypedBody<PutPipelineObjectRequest>,
) -> Result<HttpResponseCreated<PutPipelineObjectResponse>, HttpError> {
    let api_state = rqctx.context();
    let body = body.into_inner();
    let path = path_params.into_inner();
    let _req_metadata = api_state
        .preflight_check(
            &rqctx.request,
            PreflightOptions {
                bypass_auth: false,
                admin_only: false,
                resources: vec![
                    Resource::Namespaces(path.namespace_id.clone()),
                    Resource::Pipelines(path.pipeline_id.clone()),
                    Resource::Objects,
                ],
                action: Action::Write,
            },
        )
        .await?;

    let mut conn = match api_state.storage.write_conn() {
        Ok(conn) => conn,
        Err(e) => {
            return Err(http_error!(
                "Could not open connection to database",
                http::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id,
                Some(e.into())
            ));
        }
    };

    let new_object = Object::new(&body.key);

    let new_object_storage =
        match new_object.to_pipeline_object_storage(&path.namespace_id, &path.pipeline_id) {
            Ok(object) => object,
            Err(e) => {
                return Err(http_error!(
                    "Could not parse objects for database",
                    http::StatusCode::INTERNAL_SERVER_ERROR,
                    rqctx.request_id.clone(),
                    Some(e.into())
                ));
            }
        };

    if let Err(e) = storage::object_store_pipeline_keys::insert(&mut conn, &new_object_storage) {
        match e {
            storage::StorageError::Exists => {
                return Err(HttpError::for_client_error(
                    None,
                    StatusCode::CONFLICT,
                    "object entry already exists".into(),
                ));
            }
            _ => {
                return Err(http_error!(
                    "Could not insert object into database",
                    http::StatusCode::INTERNAL_SERVER_ERROR,
                    rqctx.request_id.clone(),
                    Some(e.into())
                ));
            }
        }
    };

    if let Err(e) = api_state
        .object_store
        .put(
            &pipeline_object_store_key(&path.namespace_id, &path.pipeline_id, &body.key),
            body.content,
            body.force,
        )
        .await
    {
        return Err(http_error!(
            "Could not insert object into store",
            http::StatusCode::INTERNAL_SERVER_ERROR,
            rqctx.request_id.clone(),
            Some(e.into())
        ));
    };

    let resp = PutPipelineObjectResponse { object: new_object };

    Ok(HttpResponseCreated(resp))
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
                resources: vec![
                    Resource::Namespaces(path.namespace_id.clone()),
                    Resource::Pipelines(path.pipeline_id.clone()),
                    Resource::Objects,
                ],
                action: Action::Delete,
            },
        )
        .await?;

    let mut conn = match api_state.storage.write_conn() {
        Ok(conn) => conn,
        Err(e) => {
            return Err(http_error!(
                "Could not open connection to database",
                http::StatusCode::INTERNAL_SERVER_ERROR,
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
    ) {
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
                    http::StatusCode::INTERNAL_SERVER_ERROR,
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
            http::StatusCode::INTERNAL_SERVER_ERROR,
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
                resources: vec![
                    Resource::Extensions(path.extension_id.clone()),
                    Resource::Objects,
                ],
                action: Action::Read,
            },
        )
        .await?;

    let mut conn = match api_state.storage.read_conn() {
        Ok(conn) => conn,
        Err(e) => {
            return Err(http_error!(
                "Could not open connection to database",
                http::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id,
                Some(e.into())
            ));
        }
    };

    let storage_objects =
        match storage::object_store_extension_keys::list(&mut conn, &path.extension_id) {
            Ok(objects) => objects,
            Err(e) => {
                return Err(http_error!(
                    "Could not get objects from database",
                    http::StatusCode::INTERNAL_SERVER_ERROR,
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
                http::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id.clone(),
                Some(e.into())
            )
        })?;

        objects.push(object);
    }

    let resp = ListExtensionObjectsResponse { objects };
    Ok(HttpResponseOk(resp))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct GetExtensionObjectResponse {
    /// The requested object data.
    pub object: Vec<u8>,
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
) -> Result<HttpResponseOk<GetExtensionObjectResponse>, HttpError> {
    let api_state = rqctx.context();
    let path = path_params.into_inner();
    let _req_metadata = api_state
        .preflight_check(
            &rqctx.request,
            PreflightOptions {
                bypass_auth: false,
                admin_only: false,
                resources: vec![
                    Resource::Extensions(path.extension_id.clone()),
                    Resource::Objects,
                ],
                action: Action::Read,
            },
        )
        .await?;

    let object_value = api_state
        .object_store
        .get(&extension_object_store_key(&path.extension_id, &path.key))
        .map_err(|err| {
            if err == object_store::ObjectStoreError::NotFound {
                return HttpError::for_bad_request(None, "Object not found".into());
            };

            http_error!(
                "Could not get object from store",
                http::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id.clone(),
                Some(err.into())
            )
        })
        .await?;

    let resp = GetExtensionObjectResponse {
        object: object_value.0,
    };

    Ok(HttpResponseOk(resp))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PutExtensionObjectRequest {
    /// The name for the object you would like to store.
    pub key: String,

    /// The bytes for the object.
    pub content: Vec<u8>,

    /// Overwrite a value of a object if it already exists.
    pub force: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PutExtensionObjectResponse {
    /// Information about the object created.
    pub object: Object,
}

/// Insert a new object into the extension object store.
#[endpoint(
    method = POST,
    path = "/api/extensions/{extension_id}/objects",
    tags = ["Objects"],
)]
pub async fn put_extension_object(
    rqctx: RequestContext<Arc<ApiState>>,
    path_params: Path<ExtensionObjectPathArgsRoot>,
    body: TypedBody<PutExtensionObjectRequest>,
) -> Result<HttpResponseCreated<PutExtensionObjectResponse>, HttpError> {
    let api_state = rqctx.context();
    let body = body.into_inner();
    let path = path_params.into_inner();
    let _req_metadata = api_state
        .preflight_check(
            &rqctx.request,
            PreflightOptions {
                bypass_auth: false,
                admin_only: false,
                resources: vec![
                    Resource::Extensions(path.extension_id.clone()),
                    Resource::Objects,
                ],
                action: Action::Write,
            },
        )
        .await?;

    let mut conn = match api_state.storage.write_conn() {
        Ok(conn) => conn,
        Err(e) => {
            return Err(http_error!(
                "Could not open connection to database",
                http::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id,
                Some(e.into())
            ));
        }
    };

    let new_object = Object::new(&body.key);

    let new_object_storage = match new_object.to_extension_object_storage(&path.extension_id) {
        Ok(object) => object,
        Err(e) => {
            return Err(http_error!(
                "Could not parse objects for database",
                http::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id.clone(),
                Some(e.into())
            ));
        }
    };

    if let Err(e) = storage::object_store_extension_keys::insert(&mut conn, &new_object_storage) {
        match e {
            storage::StorageError::Exists => {
                return Err(HttpError::for_client_error(
                    None,
                    StatusCode::CONFLICT,
                    "object entry already exists".into(),
                ));
            }
            _ => {
                return Err(http_error!(
                    "Could not insert object into database",
                    http::StatusCode::INTERNAL_SERVER_ERROR,
                    rqctx.request_id.clone(),
                    Some(e.into())
                ));
            }
        }
    };

    if let Err(e) = api_state
        .object_store
        .put(
            &extension_object_store_key(&path.extension_id, &body.key),
            body.content,
            body.force,
        )
        .await
    {
        return Err(http_error!(
            "Could not insert object into store",
            http::StatusCode::INTERNAL_SERVER_ERROR,
            rqctx.request_id.clone(),
            Some(e.into())
        ));
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
                resources: vec![
                    Resource::Extensions(path.extension_id.clone()),
                    Resource::Objects,
                ],
                action: Action::Delete,
            },
        )
        .await?;

    let mut conn = match api_state.storage.write_conn() {
        Ok(conn) => conn,
        Err(e) => {
            return Err(http_error!(
                "Could not open connection to database",
                http::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id,
                Some(e.into())
            ));
        }
    };

    if let Err(e) =
        storage::object_store_extension_keys::delete(&mut conn, &path.extension_id, &path.key)
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
                    http::StatusCode::INTERNAL_SERVER_ERROR,
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
            http::StatusCode::INTERNAL_SERVER_ERROR,
            rqctx.request_id.clone(),
            Some(e.into())
        ));
    };

    Ok(HttpResponseDeleted())
}
