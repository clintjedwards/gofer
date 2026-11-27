use super::permissioning::{Action, NamespaceResource, Requirement};
use crate::{
    api::{ApiState, PreflightOptions, ensure_namespace_exists, epoch_milli, is_valid_identifier},
    http_error,
    images::{self, Digest, ImageError, Images},
    storage::{self, StorageError},
};
use dropshot::{
    HttpError, HttpResponseOk, HttpResponseUpdatedNoContent, Path, RequestContext, StreamingBody,
    TypedBody, endpoint,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sqlx::SqliteConnection;
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

/// Whether a blob the pipeline has can be told to the caller as stored. When it's the manifest of an image one of the
/// pipeline's tasks runs, that's only when every part of the image is stored too, and each part is marked as seen.
///
/// `gofer up` keeps a task's image from the last version by asking about its manifest alone. Say a layer's file was
/// removed from the store, by hand or by something that cleans up old files. The manifest is still here, so without
/// this `gofer up` would keep the image, and registering it would fail every time.
async fn counts_as_stored(
    images: &Images,
    conn: &mut SqliteConnection,
    namespace_id: &str,
    pipeline_id: &str,
    digest: &Digest,
    now: &str,
) -> anyhow::Result<bool> {
    let manifest = digest.to_string();
    if !storage::image_blobs::is_registered_image(conn, namespace_id, pipeline_id, &manifest)
        .await?
    {
        return Ok(true);
    }

    let Some(blobs) = images.blobs_of(digest).await? else {
        return Ok(false);
    };

    for blob in blobs.iter().filter(|blob| *blob != digest) {
        if !images.contains(blob).await? {
            return Ok(false);
        }

        let blob = blob.to_string();
        match storage::image_blobs::update_last_seen(conn, namespace_id, pipeline_id, &blob, now)
            .await
        {
            Ok(()) => {}
            Err(StorageError::NotFound) => return Ok(false),
            Err(e) => return Err(e.into()),
        }
    }

    Ok(true)
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
/// can only use blobs it uploaded itself, so blobs that only another pipeline has uploaded are returned too. The
/// manifest of an image the pipeline already runs is only left out when every part of that image is still stored, so
/// asking about a manifest is enough to tell whether its image can be used again.
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
        let parsed = parse_digest(&digest)?;
        let exists = api_state
            .images
            .contains(&parsed)
            .await
            .map_err(|e| image_error(e, &rqctx.request_id))?;

        if exists {
            stored.push(parsed);
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
            &digest.to_string(),
            &now,
        )
        .await
        {
            Ok(()) => {}
            Err(StorageError::NotFound) => {
                missing.push(digest.to_string());
                continue;
            }
            Err(e) => {
                return Err(http_error!(
                    "Could not update image blob in database",
                    hyper::StatusCode::INTERNAL_SERVER_ERROR,
                    rqctx.request_id.clone(),
                    Some(e.into())
                ));
            }
        }

        let counts = counts_as_stored(
            &api_state.images,
            &mut tx,
            &path.namespace_id,
            &path.pipeline_id,
            &digest,
            &now,
        )
        .await
        .map_err(|e| {
            http_error!(
                "Could not check the rest of an image",
                hyper::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id.clone(),
                Some(e.into())
            )
        })?;

        if !counts {
            missing.push(digest.to_string());
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{api::pipeline_configs::ConfigState, images::Config, storage::tests::TestHarness};
    use sha2::{Digest as _, Sha256};

    const NAMESPACE: &str = "default";
    const PIPELINE: &str = "pipeline";

    /// Stores a blob and says the pipeline uploaded it long ago.
    async fn upload(images: &Images, conn: &mut SqliteConnection, content: &[u8]) -> Digest {
        let digest = Digest::from_hasher(Sha256::new_with_prefix(content));
        let chunk: Result<_, std::io::Error> = Ok(bytes::Bytes::copy_from_slice(content));
        let blob = images
            .put(&digest, futures::stream::iter([chunk]))
            .await
            .unwrap();

        storage::image_blobs::upsert(
            conn,
            &storage::image_blobs::ImageBlob {
                namespace_id: NAMESPACE.into(),
                pipeline_id: PIPELINE.into(),
                digest: digest.to_string(),
                diff_id: blob.diff_id.to_string(),
                size: blob.size as i64,
                last_seen: "0".into(),
            },
        )
        .await
        .unwrap();

        digest
    }

    #[tokio::test]
    async fn test_a_manifest_only_counts_while_its_whole_image_is_stored() {
        let dir = tempfile::tempdir().unwrap();
        let images = Images::new(&Config {
            path: dir.path().to_string_lossy().into(),
            retention_versions: 10,
        })
        .await
        .unwrap();

        let db = TestHarness::new().await;
        let mut conn = db.write_conn().await.unwrap();
        storage::namespaces::insert(
            &mut conn,
            &storage::namespaces::Namespace {
                id: NAMESPACE.into(),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        storage::pipeline_metadata::insert(
            &mut conn,
            &storage::pipeline_metadata::PipelineMetadata {
                namespace_id: NAMESPACE.into(),
                pipeline_id: PIPELINE.into(),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        storage::pipeline_configs::insert(
            &mut conn,
            &storage::pipeline_configs::PipelineConfig {
                namespace_id: NAMESPACE.into(),
                pipeline_id: PIPELINE.into(),
                version: 1,
                state: ConfigState::Live.to_string(),
                ..Default::default()
            },
        )
        .await
        .unwrap();

        let layer = upload(&images, &mut conn, b"some layer").await;
        let config = upload(&images, &mut conn, b"some config").await;
        let manifest = serde_json::json!({
            "config": {"digest": config.to_string()},
            "layers": [{"digest": layer.to_string()}],
        });
        let manifest = upload(&images, &mut conn, manifest.to_string().as_bytes()).await;

        // No task runs it, so it's only a blob like any other.
        std::fs::remove_file(images.blob_path(&layer)).unwrap();
        assert!(counts(&images, &mut conn, &manifest, "1").await);
        upload(&images, &mut conn, b"some layer").await;

        storage::tasks::insert(
            &mut conn,
            &storage::tasks::Task {
                namespace_id: NAMESPACE.into(),
                pipeline_id: PIPELINE.into(),
                pipeline_config_version: 1,
                task_id: "task".into(),
                image_digest: manifest.to_string(),
                ..Default::default()
            },
        )
        .await
        .unwrap();

        assert!(counts(&images, &mut conn, &manifest, "2").await);
        let seen = storage::image_blobs::get(&mut conn, NAMESPACE, PIPELINE, &layer.to_string())
            .await
            .unwrap();
        assert_eq!(seen.last_seen, "2");

        std::fs::remove_file(images.blob_path(&layer)).unwrap();
        assert!(!counts(&images, &mut conn, &manifest, "3").await);
        upload(&images, &mut conn, b"some layer").await;

        storage::image_blobs::delete(&mut conn, NAMESPACE, PIPELINE, &config.to_string())
            .await
            .unwrap();
        assert!(!counts(&images, &mut conn, &manifest, "4").await);
    }

    async fn counts(
        images: &Images,
        conn: &mut SqliteConnection,
        manifest: &Digest,
        now: &str,
    ) -> bool {
        counts_as_stored(images, conn, NAMESPACE, PIPELINE, manifest, now)
            .await
            .unwrap()
    }
}
