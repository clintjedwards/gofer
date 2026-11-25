use super::permissioning::{Action, NamespaceResource, Requirement};
use crate::{
    api::{ApiState, PreflightOptions, ensure_namespace_exists, epoch_milli, is_valid_identifier},
    http_error,
    images::{self, Digest, ImageError},
    storage::{self, StorageError},
};
use dropshot::{
    HttpError, HttpResponseOk, HttpResponseUpdatedNoContent, Path, RequestContext, StreamingBody,
    TypedBody, endpoint,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Asking about blobs holds the database's write connection, so this keeps the list to a few thousand digests.
const MISSING_REQUEST_BODY_MAX_BYTES: usize = 1024 * 1024; // 1MB

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ImageBlobPathArgsRoot {
    /// The unique identifier for the target namespace.
    pub namespace_id: String,

    /// The unique identifier for the target pipeline.
    pub pipeline_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ImageBlobPathArgs {
    /// The unique identifier for the target namespace.
    pub namespace_id: String,

    /// The unique identifier for the target pipeline.
    pub pipeline_id: String,

    /// The sha256 digest of the blob's contents. Ex: `sha256:2cf24dba5fb0...`
    pub digest: String,
}

/// Uploads come before the pipeline's config, so for a new pipeline its id hasn't been checked by anything yet.
fn check_pipeline_id(pipeline_id: &str) -> Result<(), HttpError> {
    is_valid_identifier(pipeline_id).map_err(|e| {
        HttpError::for_bad_request(
            None,
            format!("'{pipeline_id}' is not a valid identifier; {e}"),
        )
    })
}

fn parse_digest(digest: &str) -> Result<Digest, HttpError> {
    digest
        .parse()
        .map_err(|e: ImageError| HttpError::for_bad_request(None, e.to_string()))
}

fn image_error(e: ImageError, request_id: &str) -> HttpError {
    match e {
        ImageError::Invalid(message) => HttpError::for_bad_request(None, message),
        ImageError::Internal(_) => http_error!(
            "Could not reach image store",
            hyper::StatusCode::INTERNAL_SERVER_ERROR,
            request_id.to_string(),
            Some(e.into())
        ),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ListMissingImageBlobsRequest {
    /// The digests of every blob the images are made of.
    pub digests: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ListMissingImageBlobsResponse {
    /// The digests of the blobs that still need to be uploaded.
    pub digests: Vec<String>,
}

/// List which image blobs need to be uploaded.
///
/// Given the digests of the blobs that make up a pipeline's images, returns the ones Gofer doesn't have yet. A pipeline
/// can only use blobs it uploaded itself, so blobs that only another pipeline has uploaded are returned too.
#[endpoint(
    method = POST,
    path = "/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/image-blobs/missing",
    tags = ["Configs"],
    request_body_max_bytes = MISSING_REQUEST_BODY_MAX_BYTES,
)]
pub async fn list_missing_image_blobs(
    rqctx: RequestContext<Arc<ApiState>>,
    path_params: Path<ImageBlobPathArgsRoot>,
    body: TypedBody<ListMissingImageBlobsRequest>,
) -> Result<HttpResponseOk<ListMissingImageBlobsResponse>, HttpError> {
    let api_state = rqctx.context();
    let path = path_params.into_inner();
    let body = body.into_inner();
    let _req_metadata = api_state
        .preflight_check(
            &rqctx,
            PreflightOptions {
                bypass_auth: false,
                admin_only: false,
                allow_anonymous: false,
                requires: Requirement::pipeline(
                    &path.namespace_id,
                    &path.pipeline_id,
                    NamespaceResource::Configs,
                ),
                action: Action::Write,
            },
        )
        .await?;

    check_pipeline_id(&path.pipeline_id)?;

    let mut missing = vec![];
    let mut stored = vec![];

    for digest in body.digests {
        let exists = api_state
            .images
            .contains(&parse_digest(&digest)?)
            .await
            .map_err(|e| image_error(e, &rqctx.request_id))?;

        if exists {
            stored.push(digest);
        } else {
            missing.push(digest);
        }
    }

    let mut tx = api_state.storage.open_tx().await.map_err(|e| {
        http_error!(
            "Could not open connection to database",
            hyper::StatusCode::INTERNAL_SERVER_ERROR,
            rqctx.request_id.clone(),
            Some(e.into())
        )
    })?;

    ensure_namespace_exists(&mut tx, &rqctx.request_id, &path.namespace_id).await?;

    let now = epoch_milli().to_string();

    // Telling the caller we have a blob means they won't upload it, so we mark it as seen to keep it around until the
    // config that needs it is registered.
    for digest in stored {
        match storage::image_blobs::update_last_seen(
            &mut tx,
            &path.namespace_id,
            &path.pipeline_id,
            &digest,
            &now,
        )
        .await
        {
            Ok(()) => {}
            Err(StorageError::NotFound) => missing.push(digest),
            Err(e) => {
                return Err(http_error!(
                    "Could not update image blob in database",
                    hyper::StatusCode::INTERNAL_SERVER_ERROR,
                    rqctx.request_id.clone(),
                    Some(e.into())
                ));
            }
        }
    }

    if let Err(e) = tx.commit().await {
        return Err(http_error!(
            "Could not close database transaction",
            hyper::StatusCode::INTERNAL_SERVER_ERROR,
            rqctx.request_id.clone(),
            Some(e.into())
        ));
    };

    Ok(HttpResponseOk(ListMissingImageBlobsResponse {
        digests: missing,
    }))
}

/// Upload an image blob.
///
/// Uploads one part of an image: its manifest, its config, or one of its layers. The contents must match the digest in
/// the path. Uploading the same blob again is fine.
#[endpoint(
    method = PUT,
    path = "/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/image-blobs/{digest}",
    tags = ["Configs"],
    request_body_max_bytes = images::MAX_BLOB_BYTES,
)]
pub async fn put_image_blob(
    rqctx: RequestContext<Arc<ApiState>>,
    path_params: Path<ImageBlobPathArgs>,
    body: StreamingBody,
) -> Result<HttpResponseUpdatedNoContent, HttpError> {
    let api_state = rqctx.context();
    let path = path_params.into_inner();
    let _req_metadata = api_state
        .preflight_check(
            &rqctx,
            PreflightOptions {
                bypass_auth: false,
                admin_only: false,
                allow_anonymous: false,
                requires: Requirement::pipeline(
                    &path.namespace_id,
                    &path.pipeline_id,
                    NamespaceResource::Configs,
                ),
                action: Action::Write,
            },
        )
        .await?;

    check_pipeline_id(&path.pipeline_id)?;
    let digest = parse_digest(&path.digest)?;

    // Check the namespace exists before uploading, otherwise the blob lands in the store and only fails at the
    // database insert, leaving it orphaned.
    {
        let mut conn = api_state.storage.read_conn().await.map_err(|e| {
            http_error!(
                "Could not open connection to database",
                hyper::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id.clone(),
                Some(e.into())
            )
        })?;

        ensure_namespace_exists(&mut conn, &rqctx.request_id, &path.namespace_id).await?;
    }

    let blob = api_state
        .images
        .put(&digest, body.into_stream())
        .await
        .map_err(|e| image_error(e, &rqctx.request_id))?;

    let size = blob
        .size
        .try_into()
        .map_err(|e: std::num::TryFromIntError| {
            http_error!(
                "Could not parse blob size into storage type",
                hyper::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id.clone(),
                Some(e.into())
            )
        })?;

    let mut conn = api_state.storage.write_conn().await.map_err(|e| {
        http_error!(
            "Could not open connection to database",
            hyper::StatusCode::INTERNAL_SERVER_ERROR,
            rqctx.request_id.clone(),
            Some(e.into())
        )
    })?;

    let image_blob = storage::image_blobs::ImageBlob {
        namespace_id: path.namespace_id,
        pipeline_id: path.pipeline_id,
        digest: digest.to_string(),
        diff_id: blob.diff_id.to_string(),
        size,
        last_seen: epoch_milli().to_string(),
    };

    storage::image_blobs::upsert(&mut conn, &image_blob)
        .await
        .map_err(|e| {
            http_error!(
                "Could not insert image blob into database",
                hyper::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id.clone(),
                Some(e.into())
            )
        })?;

    Ok(HttpResponseUpdatedNoContent())
}
