//! Makes a task's image without Docker, when nothing has to run to make it.
//!
//! A task that's only a base image and a command has nothing to build. Its image is the base's layers with a new
//! config, so this downloads the base's manifest and config from its registry, writes the task's command into the
//! config the same way a build does, and that's the image. Not needing Docker means it also works inside a Gofer task,
//! and there's nothing to emulate on a machine that isn't amd64.
//!
//! The base's layers stay in the registry. Gofer often has them already, from the pipeline's earlier versions or from
//! another task on the same base, so `gofer up` asks first and only downloads the ones it's missing, straight into the
//! upload.
//!
//! A task that can be made this way always is, even when Docker is running, so it comes out the same on every machine.

use super::{
    Build, Steps,
    image::{self, Image, MAX_JSON_BYTES},
    login::{self, Logins},
    progress::{BaseError, base_error_message},
};
use crate::images::Digest;
use anyhow::{Context, Result, anyhow, bail};
use bytes::Bytes;
use futures::{TryStreamExt, stream::BoxStream};
use oci_client::{
    Client, Reference,
    client::ClientConfig,
    errors::{OciDistributionError, OciErrorCode},
    manifest,
    secrets::RegistryAuth,
};
use serde_json::Value;
use std::{io, time::Duration};

/// The kinds of manifest we ask a registry for. An index or a list names a manifest for each platform the image is made
/// for, and the other two are one image's. They're the ones Gofer's server takes.
const MANIFEST_TYPES: [&str; 4] = [
    manifest::OCI_IMAGE_INDEX_MEDIA_TYPE,
    manifest::IMAGE_MANIFEST_LIST_MEDIA_TYPE,
    manifest::OCI_IMAGE_MEDIA_TYPE,
    manifest::IMAGE_MANIFEST_MEDIA_TYPE,
];
const IMAGE_MANIFEST_TYPES: [&str; 2] = [MANIFEST_TYPES[2], MANIFEST_TYPES[3]];

/// Whether this can make a task's image, which it can when the task is only a base. Copying files in or running
/// anything needs BuildKit.
pub fn takes(build: &Build) -> bool {
    matches!(build, Build::Steps(Steps { steps, keep: None, .. }) if steps.is_empty())
}

/// Makes the image for a task that's only `base`, with the task's entrypoint and command baked in.
pub async fn build(
    base: &str,
    entrypoint: Option<&[String]>,
    command: Option<&[String]>,
) -> Result<Image> {
    let logins = login::find(&[base]).await.without_identity_tokens();
    let registry = Registry::new(base, logins)?;
    let folder = image::folder()?;

    let manifest = registry.manifest().await?;
    let (manifest_digest, _) = image::write_blob(folder.path(), &manifest)?;

    let manifest: Value = serde_json::from_slice(&manifest)
        .with_context(|| format!("Could not read the manifest of the base image {base}"))?;
    let kind = manifest["mediaType"].as_str().unwrap_or_default();
    if !IMAGE_MANIFEST_TYPES.contains(&kind) {
        bail!("The base image {base} has a manifest of type '{kind}', which Gofer can't run");
    }

    let config: Digest = (manifest["config"]["digest"].as_str())
        .with_context(|| format!("The manifest of the base image {base} doesn't name its config"))?
        .parse()?;
    let config = registry.config(&config).await?;
    image::write_blob(folder.path(), &config)?;

    image::finish(folder, manifest_digest, entrypoint, command, Some(registry))
        .with_context(|| format!("Could not make an image from the base image {base}"))
}

/// Where a base image is, so its blobs can be downloaded.
pub struct Registry {
    client: Client,
    reference: Reference,

    /// The user's login for it. oci-client keeps using it for the blobs once the manifest is downloaded.
    auth: RegistryAuth,

    /// Where `auth` came from, to explain a registry that wants a login.
    logins: Logins,

    /// The base as the task names it, for messages.
    base: String,
}

impl Registry {
    fn new(base: &str, logins: Logins) -> Result<Self> {
        let reference =
            (base.parse()).with_context(|| format!("'{base}' isn't a name an image can have"))?;

        // A registry that stops answering shouldn't leave `gofer up` waiting forever. The read timeout is for each
        // read, not the whole download, so a big layer on a slow connection is fine.
        let client = Client::new(ClientConfig {
            connect_timeout: Some(Duration::from_secs(30)),
            read_timeout: Some(Duration::from_secs(60)),
            ..Default::default()
        });

        Ok(Self {
            client,
            reference,
            auth: logins.registry_auth(base),
            logins,
            base: base.into(),
        })
    }

    /// The base's manifest for linux/amd64, which is where tasks run.
    async fn manifest(&self) -> Result<Vec<u8>> {
        let (body, _) = (self.client)
            .pull_manifest_raw(&self.reference, &self.auth, &MANIFEST_TYPES)
            .await
            .map_err(|e| lookup_error(&self.base, e, &self.logins))?;

        let manifest: Value = serde_json::from_slice(&body).with_context(|| {
            format!(
                "Could not read the manifest of the base image {}",
                self.base
            )
        })?;
        let Some(manifests) = manifest["manifests"].as_array() else {
            return Ok(body.to_vec());
        };

        let digest = amd64_manifest(manifests).with_context(|| {
            format!(
                "The base image {} isn't made for linux/amd64, which is what tasks run on",
                self.base
            )
        })?;

        // oci-client checks that what comes back matches the digest.
        let reference = self.reference.clone_with_digest(digest.into());
        let (body, _) = (self.client)
            .pull_manifest_raw(&reference, &self.auth, &IMAGE_MANIFEST_TYPES)
            .await
            .map_err(|e| lookup_error(&self.base, e, &self.logins))?;

        Ok(body.to_vec())
    }

    /// Downloads a blob as it's read. The stream ends in an error if what came doesn't match the digest.
    pub async fn blob(&self, digest: &Digest) -> Result<BoxStream<'static, io::Result<Bytes>>> {
        let blob = (self.client)
            .pull_blob_stream(&self.reference, digest.to_string().as_str())
            .await
            .map_err(|e| anyhow!(e))
            .with_context(|| {
                format!(
                    "Could not download {digest} of the base image {}",
                    self.base
                )
            })?;

        Ok(blob.stream)
    }

    /// Downloads the base's config, which is read into memory.
    async fn config(&self, digest: &Digest) -> Result<Vec<u8>> {
        let mut blob = self.blob(digest).await?;
        let mut content = vec![];

        while let Some(bytes) = (blob.try_next().await).with_context(|| {
            format!(
                "Could not download {digest} of the base image {}",
                self.base
            )
        })? {
            content.extend_from_slice(&bytes);

            if content.len() as u64 > MAX_JSON_BYTES {
                bail!(
                    "The config of the base image {} is larger than {MAX_JSON_BYTES} bytes",
                    self.base
                );
            }
        }

        Ok(content)
    }
}

/// Picks the manifest for linux/amd64 out of an index or a list.
fn amd64_manifest(manifests: &[Value]) -> Option<&str> {
    (manifests.iter())
        .find(|entry| {
            entry["platform"]["os"] == "linux" && entry["platform"]["architecture"] == "amd64"
        })
        .and_then(|entry| entry["digest"].as_str())
}

/// Says why the base couldn't be looked up, in the same words as when BuildKit can't.
fn lookup_error(base: &str, e: OciDistributionError, logins: &Logins) -> anyhow::Error {
    let codes: Vec<&OciErrorCode> = match &e {
        OciDistributionError::RegistryError { envelope, .. } => {
            envelope.errors.iter().map(|error| &error.code).collect()
        }
        _ => vec![],
    };
    let has = |wanted: &[OciErrorCode]| codes.iter().any(|code| wanted.contains(code));

    let why = match &e {
        OciDistributionError::ImageManifestNotFoundError(_)
        | OciDistributionError::ServerError { code: 404, .. } => BaseError::NotFound,
        OciDistributionError::RegistryError { .. }
            if has(&[
                OciErrorCode::ManifestUnknown,
                OciErrorCode::NameUnknown,
                OciErrorCode::NotFound,
            ]) =>
        {
            BaseError::NotFound
        }
        OciDistributionError::UnauthorizedError { .. }
        | OciDistributionError::AuthenticationFailure(_)
        | OciDistributionError::ServerError {
            code: 401 | 403, ..
        } => BaseError::NeedsLogin,
        OciDistributionError::RegistryError { .. }
            if has(&[OciErrorCode::Unauthorized, OciErrorCode::Denied]) =>
        {
            BaseError::NeedsLogin
        }
        OciDistributionError::RequestError(e) if e.is_connect() || e.is_timeout() => {
            BaseError::NoNetwork
        }
        _ => BaseError::Other,
    };

    anyhow!(base_error_message(
        base,
        why,
        &format!("{:#}", anyhow!(e)),
        logins
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use gofer_sdk::config::{Keep, Step};
    use oci_client::errors::{OciEnvelope, OciError};
    use rstest::rstest;
    use serde_json::json;

    fn steps(steps: Vec<Step>, keep: Option<Keep>) -> Build {
        Build::Steps(Steps {
            base: "alpine:3.22".into(),
            steps,
            keep,
        })
    }

    #[rstest]
    #[case::only_a_base(steps(vec![], None), true)]
    #[case::copy(steps(vec![Step::Copy { from: "./a".into(), to: "/a".into() }], None), false)]
    #[case::run(steps(vec![Step::Run { command: "true".into() }], None), false)]
    #[case::keep(steps(vec![], Some(Keep { paths: vec!["/a".into()], base: "alpine:3.22".into() })), false)]
    #[case::dockerfile(Build::Dockerfile { path: "Dockerfile".into(), context: ".".into() }, false)]
    fn test_only_a_base_is_taken(#[case] build: Build, #[case] taken: bool) {
        assert_eq!(takes(&build), taken);
    }

    #[test]
    fn test_amd64_is_picked_from_an_index() {
        let manifests = [
            json!({"digest": "sha256:arm", "platform": {"os": "linux", "architecture": "arm64"}}),
            json!({"digest": "sha256:amd", "platform": {"os": "linux", "architecture": "amd64"}}),
            json!({"digest": "sha256:att", "platform": {"os": "unknown", "architecture": "unknown"}}),
        ];
        assert_eq!(amd64_manifest(&manifests), Some("sha256:amd"));
        assert_eq!(amd64_manifest(&manifests[..1]), None);
    }

    fn registry_error(code: OciErrorCode) -> OciDistributionError {
        OciDistributionError::RegistryError {
            envelope: OciEnvelope {
                errors: vec![OciError {
                    code,
                    message: String::new(),
                    detail: Value::Null,
                }],
            },
            url: "https://registry.example.com/v2/thing/manifests/1".into(),
        }
    }

    #[rstest]
    #[case::unknown_tag(
        registry_error(OciErrorCode::ManifestUnknown),
        "doesn't exist. Check its name and tag."
    )]
    #[case::unknown_name(
        registry_error(OciErrorCode::NameUnknown),
        "doesn't exist. Check its name and tag."
    )]
    #[case::plain_404(
        OciDistributionError::ServerError { code: 404, url: String::new(), message: String::new() },
        "doesn't exist. Check its name and tag."
    )]
    // Docker Hub says this about images that don't exist too, when there's no login.
    #[case::unauthorized(
        OciDistributionError::UnauthorizedError { url: String::new() },
        "doesn't exist, or it needs a login to download."
    )]
    #[case::denied(
        registry_error(OciErrorCode::Denied),
        "doesn't exist, or it needs a login to download."
    )]
    #[case::other(
        registry_error(OciErrorCode::Toomanyrequests),
        "Could not look up the base image thing:1; "
    )]
    fn test_lookup_errors_are_explained(#[case] error: OciDistributionError, #[case] says: &str) {
        let message = lookup_error("thing:1", error, &Logins::default()).to_string();
        assert!(message.contains(says), "{message}");
    }
}
