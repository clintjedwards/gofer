//! Extension images stored in Gofer.
//!
//! An extension's manifest names its image one of two ways: as an image in a registry, which Docker pulls, or as
//! `gofer://<name>:<tag>`, an image stored in Gofer, which Gofer loads into Docker the same way it does a task's. Admins
//! store them with `gofer extension image push`, which uploads them the same way `gofer up` uploads a task's image.
//! Gofer's own extensions are stored from its binary every time it starts; see `api::extensions::builtin`.
//!
//! A name and tag is fixed once stored, unless it's replaced on purpose with `force`. Upgrading an extension means
//! storing a new tag and pointing its manifest at it, so a reload shows the change and can still go back to the old one.

use super::{
    extensions::reconcile,
    image_blobs::{ListMissingImageBlobsRequest, ListMissingImageBlobsResponse},
    permissioning::{Action, Requirement},
};
use crate::{
    api::{ApiState, PreflightOptions, epoch_milli, image_retention, is_valid_identifier},
    http_error,
    images::{self, Digest, ImageError, Owner},
    storage::{self, StorageError},
};
use anyhow::{Result, anyhow, bail};
use dropshot::{
    ClientErrorStatusCode, HttpError, HttpResponseDeleted, HttpResponseOk,
    HttpResponseUpdatedNoContent, Path, RequestContext, StreamingBody, TypedBody, endpoint,
};
use lazy_regex::regex;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// What a manifest's `image` starts with when the image is stored in Gofer. `://` can't be part of a Docker image's
/// name, so this is never mistaken for an image in a registry.
pub const STORED_IMAGE_SCHEME: &str = "gofer://";

/// Asking about blobs holds the database's write connection, so this keeps the list to a few thousand digests.
const MISSING_REQUEST_BODY_MAX_BYTES: usize = 1024 * 1024; // 1MB

/// An image stored in Gofer, as a manifest names it: `gofer://<name>:<tag>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredImageRef {
    pub name: String,
    pub tag: String,
}

impl std::fmt::Display for StoredImageRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{STORED_IMAGE_SCHEME}{}:{}", self.name, self.tag)
    }
}

impl StoredImageRef {
    /// Reads a manifest's `image`. `None` means it's an image in a registry instead.
    pub fn parse(image: &str) -> Option<Result<Self>> {
        let rest = image.strip_prefix(STORED_IMAGE_SCHEME)?;
        Some(Self::from_name_and_tag(rest))
    }

    /// Reads `<name>:<tag>`, the way `gofer extension image push` takes it.
    pub fn from_name_and_tag(value: &str) -> Result<Self> {
        let Some((name, tag)) = value.split_once(':') else {
            bail!("'{value}' needs a tag, like '{value}:1.0.0'");
        };

        let image = Self {
            name: name.into(),
            tag: tag.into(),
        };
        image.check()?;
        Ok(image)
    }

    fn check(&self) -> Result<()> {
        is_valid_identifier(&self.name)
            .map_err(|e| anyhow!("'{}' is not a valid extension image name; {e}", self.name))?;

        // Blobs are uploaded at /api/extensions/images/blobs/{digest}, which an image with this name would collide with.
        if self.name == "blobs" {
            bail!("'blobs' can't be an extension image's name; pick another");
        }

        // The same rule Docker has for tags, so a version like 1.2.0-rc.1 fits.
        if !regex!("^[A-Za-z0-9_][A-Za-z0-9_.-]{0,127}$").is_match(&self.tag) {
            bail!(
                "'{}' is not a valid extension image tag; it can have letters, numbers, '_', '.' and '-', can't \
                 start with '.' or '-', and is at most 128 characters",
                self.tag
            );
        }

        Ok(())
    }
}

/// The digest of the image a name and tag point to, or `None` if nothing is stored under it.
pub async fn resolve(api_state: &ApiState, image: &StoredImageRef) -> Result<Option<Digest>> {
    let mut conn = api_state.storage.read_conn().await?;

    match storage::extension_images::get(&mut conn, &image.name, &image.tag).await {
        Ok(stored) => Ok(Some(stored.digest.parse()?)),
        Err(StorageError::NotFound) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ExtensionImage {
    pub name: String,
    pub tag: String,

    /// The digest of the image's manifest.
    pub digest: String,

    /// Whether it's one of Gofer's own extensions, stored from its binary when it starts. These can't be replaced or
    /// deleted.
    pub builtin: bool,

    /// Time it was stored in epoch milliseconds.
    pub created: u64,

    /// The extensions whose manifest names this image.
    pub used_by: Vec<String>,
}

/// The extensions whose registration names this image, whatever state they're in. One that's been removed from the
/// config still has its registration until it's purged, and adding it back would need the image.
fn used_by(api_state: &ApiState, image: &StoredImageRef) -> Vec<String> {
    let name = image.to_string();
    let mut used: Vec<String> = (api_state.extensions.iter())
        .filter(|extension| extension.registration.image == name)
        .map(|extension| extension.key().clone())
        .collect();
    used.sort();
    used
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ExtensionImagePathArgs {
    /// The image's name, as in `gofer://<name>:<tag>`.
    pub name: String,

    /// The image's tag, as in `gofer://<name>:<tag>`.
    pub tag: String,
}

impl ExtensionImagePathArgs {
    fn image(&self) -> Result<StoredImageRef, HttpError> {
        let image = StoredImageRef {
            name: self.name.clone(),
            tag: self.tag.clone(),
        };
        image
            .check()
            .map_err(|e| HttpError::for_bad_request(None, format!("{e:#}")))?;
        Ok(image)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ExtensionImageBlobPathArgs {
    /// The sha256 digest of the blob's contents. Ex: `sha256:2cf24dba5fb0...`
    pub digest: String,
}

fn admin_only(action: Action) -> PreflightOptions {
    PreflightOptions {
        bypass_auth: false,
        admin_only: true,
        allow_anonymous: false,
        requires: Requirement::Extension {
            extension: None,
            resource: None,
        },
        action,
    }
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

fn database_error(e: impl Into<anyhow::Error>, request_id: &str) -> HttpError {
    let e: anyhow::Error = e.into();
    http_error!(
        "Could not reach the database",
        hyper::StatusCode::INTERNAL_SERVER_ERROR,
        request_id.to_string(),
        Some(e.into())
    )
}

/// List which extension image blobs need to be uploaded.
///
/// Given the digests of the blobs that make up an extension image, returns the ones Gofer doesn't have yet. Extension
/// images can only use blobs uploaded for extension images, so blobs only a pipeline has uploaded are returned too.
#[endpoint(
    method = POST,
    path = "/api/extensions/images/missing",
    tags = ["Extensions"],
    request_body_max_bytes = MISSING_REQUEST_BODY_MAX_BYTES,
)]
pub async fn list_missing_extension_image_blobs(
    rqctx: RequestContext<Arc<ApiState>>,
    body: TypedBody<ListMissingImageBlobsRequest>,
) -> Result<HttpResponseOk<ListMissingImageBlobsResponse>, HttpError> {
    let api_state = rqctx.context();
    let body = body.into_inner();
    let _req_metadata = api_state
        .preflight_check(&rqctx, admin_only(Action::Write))
        .await?;

    let mut missing = vec![];
    let mut tx =
        (api_state.storage.open_tx().await).map_err(|e| database_error(e, &rqctx.request_id))?;
    let now = epoch_milli().to_string();

    for digest in body.digests {
        let parsed = parse_digest(&digest)?;
        let stored = (api_state.images.contains(&parsed).await)
            .map_err(|e| image_error(e, &rqctx.request_id))?;
        if !stored {
            missing.push(digest);
            continue;
        }

        // Telling the caller we have a blob means they won't upload it, so we mark it as seen to keep it around until
        // the image that needs it is stored.
        match storage::extension_images::update_blob_last_seen(&mut tx, &digest, &now).await {
            Ok(()) => {}
            Err(StorageError::NotFound) => missing.push(digest),
            Err(e) => return Err(database_error(e, &rqctx.request_id)),
        }
    }

    (tx.commit().await).map_err(|e| database_error(e, &rqctx.request_id))?;

    Ok(HttpResponseOk(ListMissingImageBlobsResponse {
        digests: missing,
    }))
}

/// Upload an extension image blob.
///
/// Uploads one part of an extension image: its manifest, its config, or one of its layers. The contents must match the
/// digest in the path. Uploading the same blob again is fine.
#[endpoint(
    method = PUT,
    path = "/api/extensions/images/blobs/{digest}",
    tags = ["Extensions"],
    request_body_max_bytes = images::MAX_BLOB_BYTES,
)]
pub async fn put_extension_image_blob(
    rqctx: RequestContext<Arc<ApiState>>,
    path_params: Path<ExtensionImageBlobPathArgs>,
    body: StreamingBody,
) -> Result<HttpResponseUpdatedNoContent, HttpError> {
    let api_state = rqctx.context();
    let path = path_params.into_inner();
    let _req_metadata = api_state
        .preflight_check(&rqctx, admin_only(Action::Write))
        .await?;

    let digest = parse_digest(&path.digest)?;
    let blob = (api_state.images.put(&digest, body.into_stream()).await)
        .map_err(|e| image_error(e, &rqctx.request_id))?;

    let mut conn =
        (api_state.storage.write_conn().await).map_err(|e| database_error(e, &rqctx.request_id))?;

    storage::extension_images::upsert_blob(
        &mut conn,
        &storage::extension_images::ExtensionImageBlob {
            digest: digest.to_string(),
            diff_id: blob.diff_id.to_string(),
            size: (blob.size.try_into())
                .map_err(|e: std::num::TryFromIntError| database_error(e, &rqctx.request_id))?,
            last_seen: epoch_milli().to_string(),
        },
    )
    .await
    .map_err(|e| database_error(e, &rqctx.request_id))?;

    Ok(HttpResponseUpdatedNoContent())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PutExtensionImageRequest {
    /// The digest of the image's manifest. Every part of the image has to be uploaded first.
    pub digest: String,

    /// Replace the image this name and tag point to, if there is one. Without it, a name and tag can only be stored
    /// once.
    #[serde(default)]
    pub force: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PutExtensionImageResponse {
    pub image: ExtensionImage,
}

/// Store an extension image.
///
/// Stores an image whose parts were uploaded, so manifests can name it as `gofer://<name>:<tag>`. The image has to be
/// one Gofer can run. A name and tag that's already stored is refused unless `force` is set, and the names of Gofer's own
/// extensions are kept for them.
#[endpoint(
    method = PUT,
    path = "/api/extensions/images/{name}/{tag}",
    tags = ["Extensions"],
)]
pub async fn put_extension_image(
    rqctx: RequestContext<Arc<ApiState>>,
    path_params: Path<ExtensionImagePathArgs>,
    body: TypedBody<PutExtensionImageRequest>,
) -> Result<HttpResponseOk<PutExtensionImageResponse>, HttpError> {
    let api_state = rqctx.context();
    let image = path_params.into_inner().image()?;
    let body = body.into_inner();
    let _req_metadata = api_state
        .preflight_check(&rqctx, admin_only(Action::Write))
        .await?;

    if reconcile::is_provided_extension(&image.name) {
        return Err(HttpError::for_bad_request(
            None,
            format!(
                "'{}' is the name of one of Gofer's own extensions, whose images come with Gofer; pick another name",
                image.name
            ),
        ));
    }

    let digest = parse_digest(&body.digest)?;
    let mut tx =
        (api_state.storage.open_tx().await).map_err(|e| database_error(e, &rqctx.request_id))?;

    (api_state
        .images
        .verify(&mut tx, Owner::Extensions, &digest)
        .await)
        .map_err(|e| image_error(e, &rqctx.request_id))?;

    let stored = storage::extension_images::ExtensionImage {
        name: image.name.clone(),
        tag: image.tag.clone(),
        digest: digest.to_string(),
        builtin: false,
        created: epoch_milli().to_string(),
    };

    match storage::extension_images::get(&mut tx, &image.name, &image.tag).await {
        // Storing the same image again changes nothing, so it isn't refused.
        Ok(existing) if existing.digest == stored.digest => {}
        Ok(_) if !body.force => {
            return Err(HttpError::for_client_error(
                None,
                ClientErrorStatusCode::CONFLICT,
                format!(
                    "'{image}' is already stored with a different image; store it under a new tag, or replace it with \
                     force (--force with the CLI)"
                ),
            ));
        }
        Ok(_) | Err(StorageError::NotFound) => {
            (storage::extension_images::upsert(&mut tx, &stored).await)
                .map_err(|e| database_error(e, &rqctx.request_id))?;
        }
        Err(e) => return Err(database_error(e, &rqctx.request_id)),
    }

    (tx.commit().await).map_err(|e| database_error(e, &rqctx.request_id))?;

    Ok(HttpResponseOk(PutExtensionImageResponse {
        image: ExtensionImage {
            name: stored.name,
            tag: stored.tag,
            digest: stored.digest,
            builtin: false,
            created: stored.created.parse().unwrap_or_default(),
            used_by: used_by(api_state, &image),
        },
    }))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ListExtensionImagesResponse {
    pub images: Vec<ExtensionImage>,
}

/// List the extension images stored in Gofer.
#[endpoint(
    method = GET,
    path = "/api/extensions/images",
    tags = ["Extensions"],
)]
pub async fn list_extension_images(
    rqctx: RequestContext<Arc<ApiState>>,
) -> Result<HttpResponseOk<ListExtensionImagesResponse>, HttpError> {
    let api_state = rqctx.context();
    let _req_metadata = api_state
        .preflight_check(&rqctx, admin_only(Action::Read))
        .await?;

    let mut conn =
        (api_state.storage.read_conn().await).map_err(|e| database_error(e, &rqctx.request_id))?;
    let stored = (storage::extension_images::list(&mut conn).await)
        .map_err(|e| database_error(e, &rqctx.request_id))?;

    let images = (stored.into_iter())
        .map(|stored| {
            let image = StoredImageRef {
                name: stored.name.clone(),
                tag: stored.tag.clone(),
            };
            ExtensionImage {
                used_by: used_by(api_state, &image),
                name: stored.name,
                tag: stored.tag,
                digest: stored.digest,
                builtin: stored.builtin,
                created: stored.created.parse().unwrap_or_default(),
            }
        })
        .collect();

    Ok(HttpResponseOk(ListExtensionImagesResponse { images }))
}

/// Delete an extension image.
///
/// Refused while any extension's manifest names it, including extensions removed from the config but not yet purged,
/// and for Gofer's own extensions. Its blobs are removed once nothing else uses them.
#[endpoint(
    method = DELETE,
    path = "/api/extensions/images/{name}/{tag}",
    tags = ["Extensions"],
)]
pub async fn delete_extension_image(
    rqctx: RequestContext<Arc<ApiState>>,
    path_params: Path<ExtensionImagePathArgs>,
) -> Result<HttpResponseDeleted, HttpError> {
    let api_state = rqctx.context();
    let image = path_params.into_inner().image()?;
    let _req_metadata = api_state
        .preflight_check(&rqctx, admin_only(Action::Delete))
        .await?;

    let mut tx =
        (api_state.storage.open_tx().await).map_err(|e| database_error(e, &rqctx.request_id))?;

    match storage::extension_images::get(&mut tx, &image.name, &image.tag).await {
        Ok(stored) if stored.builtin => {
            return Err(HttpError::for_bad_request(
                None,
                format!("'{image}' is one of Gofer's own extension images, which come with Gofer"),
            ));
        }
        Ok(_) => {}
        Err(StorageError::NotFound) => {
            // for_not_found keeps its message to itself, and this one is worth showing.
            return Err(HttpError::for_client_error(
                None,
                ClientErrorStatusCode::NOT_FOUND,
                format!("'{image}' isn't stored"),
            ));
        }
        Err(e) => return Err(database_error(e, &rqctx.request_id)),
    }

    let used = used_by(api_state, &image);
    if !used.is_empty() {
        return Err(HttpError::for_client_error(
            None,
            ClientErrorStatusCode::CONFLICT,
            format!(
                "'{image}' is used by {}; point their manifests at another image and reload, or purge them, first",
                used.join(", ")
            ),
        ));
    }

    (storage::extension_images::delete(&mut tx, &image.name, &image.tag).await)
        .map_err(|e| database_error(e, &rqctx.request_id))?;
    (tx.commit().await).map_err(|e| database_error(e, &rqctx.request_id))?;

    tokio::spawn(image_retention::clean_extension_images(api_state.clone()));

    Ok(HttpResponseDeleted())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_stored_images() {
        assert_eq!(
            StoredImageRef::parse("gofer://slack:1.2.0-rc.1")
                .unwrap()
                .unwrap(),
            StoredImageRef {
                name: "slack".into(),
                tag: "1.2.0-rc.1".into()
            }
        );

        // Anything else is an image in a registry.
        assert!(StoredImageRef::parse("ghcr.io/example/slack:1.2.0").is_none());

        for bad in [
            "gofer://slack",
            "gofer://slack:",
            "gofer://sl:1.0",
            "gofer://slack_bot:1.0",
            "gofer://slack:.hidden",
            "gofer://slack:1.0:extra",
            "gofer://ghcr.io/slack:1.0",
            "gofer://blobs:1.0",
        ] {
            assert!(StoredImageRef::parse(bad).unwrap().is_err(), "{bad}");
        }
    }
}
