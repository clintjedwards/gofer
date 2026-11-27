//! Stores the images that tasks run as.
//!
//! An image arrives as blobs (basically just hashed files): a manifest, the config and layers it names, all identified
//! by the sha256 digest of their contents. Blobs are only kept once their contents match that digest.

use crate::storage::{self, StorageError};
use bytes::Bytes;
use flate2::bufread::MultiGzDecoder;
use futures::{Stream, StreamExt, TryStreamExt};
use lazy_regex::regex;
use serde::{
    Deserialize,
    de::{DeserializeOwned, IgnoredAny},
};
use sha2::{Digest as _, Sha256};
use sqlx::SqliteConnection;
use std::{
    collections::HashMap,
    fmt::Display,
    io::{BufRead, Read},
    path::{Path, PathBuf},
    pin::Pin,
    str::FromStr,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufWriter};

/// The largest a blob can be, both as uploaded and once decompressed.
pub const MAX_BLOB_BYTES: usize = 50 * 1_000_000_000; // 50GB

/// The largest a manifest or an image's config can be. They're read into memory and normally only a few KB.
const MAX_JSON_BYTES: u64 = 4 * 1024 * 1024; // 4MB

/// The types of manifest that describe a single image. A manifest says which type it is in its `mediaType` field. The
/// first is from the OCI spec and the second is the Docker format it came from. Both specs also have a type for a list
/// of images, one per platform, which isn't something we can run.
///
/// - <https://github.com/opencontainers/image-spec/blob/main/media-types.md>
/// - <https://distribution.github.io/distribution/spec/manifest-v2-2/>
const MANIFEST_TYPES: [&str; 2] = [
    "application/vnd.oci.image.manifest.v1+json",
    "application/vnd.docker.distribution.manifest.v2+json",
];

/// The magic numbers of gzip and zstd, which are the bytes that every file of that type starts with. They're how we tell
/// what a layer is compressed with, if anything.
///
/// - <https://www.rfc-editor.org/rfc/rfc1952> section 2.3.1
/// - <https://www.rfc-editor.org/rfc/rfc8878> section 3.1.1, which writes it as 0xFD2FB528 since it's little-endian
const GZIP_MAGIC_NUMBER: [u8; 2] = [0x1f, 0x8b];
const ZSTD_MAGIC_NUMBER: [u8; 4] = [0x28, 0xb5, 0x2f, 0xfd];

/// Every image is built for this platform. Images record their own, so supporting others later won't need old data to
/// change.
const IMAGE_OS: &str = "linux";
const IMAGE_ARCHITECTURE: &str = "amd64";

#[derive(thiserror::Error, Debug, PartialEq, Eq)]
pub enum ImageError {
    /// What we were given isn't something we can accept.
    #[error("{0}")]
    Invalid(String),

    /// The store itself failed.
    #[error("unexpected image store error occurred; {0}")]
    Internal(String),
}

fn internal(e: impl Display) -> ImageError {
    ImageError::Internal(e.to_string())
}

/// A sha256 digest, like `sha256:2cf24dba...`. Can only be made from a well formed digest, which is what makes it safe
/// to use as part of a path.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Digest(String);

impl Digest {
    pub fn from_hasher(hasher: Sha256) -> Self {
        Self(format!("sha256:{}", hex::encode(hasher.finalize())))
    }

    pub fn hex(&self) -> &str {
        &self.0["sha256:".len()..]
    }

    /// What Docker calls a task's image once Gofer has loaded it, where this is the digest of its manifest. The name
    /// comes from the contents, so two versions that built the same image share it, and a name can never point at a
    /// different image.
    pub fn docker_name(&self) -> String {
        format!("gofer-image:{}", self.hex())
    }
}

impl FromStr for Digest {
    type Err = ImageError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if !regex!("^sha256:[0-9a-f]{64}$").is_match(value) {
            return Err(ImageError::Invalid(format!(
                "'{value}' is not a valid digest; must be 'sha256:' followed by 64 lowercase hex characters"
            )));
        }

        Ok(Self(value.into()))
    }
}

impl Display for Digest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// What we learned about a blob by storing it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Blob {
    pub size: u64,

    /// The digest of the blob's decompressed contents. Blobs that aren't compressed have their own digest here.
    pub diff_id: Digest,
}

#[derive(Deserialize, Default, Debug)]
#[serde(rename_all = "camelCase", default)]
struct ManifestType {
    media_type: String,
}

// Docker reads manifests with a parser that ignores case, so to Docker 'Layers' and 'layers' are the same field and
// the last one wins. Refusing fields we don't know is what stops a manifest from meaning one thing to us and another
// to Docker.
#[derive(Deserialize, Default, Debug)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
struct Manifest {
    #[allow(dead_code)]
    schema_version: u64,

    #[allow(dead_code)]
    media_type: String,

    config: Descriptor,

    layers: Vec<Descriptor>,

    #[allow(dead_code)]
    annotations: serde_json::Value,
}

#[derive(Deserialize, Default, Debug)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
struct Descriptor {
    #[allow(dead_code)]
    media_type: String,

    digest: String,

    size: u64,

    urls: Vec<String>,

    // A copy of the blob. Docker only uses it when it matches the digest, so there's nothing to check.
    #[allow(dead_code)]
    data: String,

    #[allow(dead_code)]
    annotations: serde_json::Value,
}

#[derive(Deserialize, Default, Debug)]
#[serde(default)]
struct ImageConfig {
    os: String,

    architecture: String,

    rootfs: RootFs,

    #[serde(flatten)]
    other: HashMap<String, IgnoredAny>,
}

impl ImageConfig {
    /// Finds a field that Docker would mistake for one of the fields above.
    ///
    /// Docker's JSON parser ignores case and ours doesn't. Given a config with both `rootfs` and `Rootfs`, we only see
    /// `rootfs`, but to Docker they're the same field and the last one wins. So someone could list layers they really
    /// uploaded in `rootfs`, which is what we check, and another pipeline's layers in `Rootfs`, which is what Docker
    /// would run.
    ///
    /// For manifests we refuse every field we don't know. Configs have too many legitimate fields to do that, so we
    /// only refuse the ones that could be mistaken for a field we check.
    fn lookalike_field(&self) -> Option<&String> {
        self.other.keys().find(|field| {
            // Docker's idea of case goes beyond A to Z. It reads 'ſ', an old form of 's', as an 's' too, so 'rootfſ'
            // also counts. That letter's uppercase is a plain 'S'.
            ["OS", "ARCHITECTURE", "ROOTFS"].contains(&field.to_uppercase().as_str())
        })
    }
}

#[derive(Deserialize, Default, Debug)]
#[serde(default, deny_unknown_fields)]
struct RootFs {
    #[allow(dead_code)]
    r#type: String,
    diff_ids: Vec<String>,
}

/// A file that's still being uploaded. Dropping it removes the file, so an upload that fails for any reason leaves
/// nothing behind. An upload that worked has been renamed by then, so there's nothing to remove.
struct Upload(PathBuf);

impl Drop for Upload {
    fn drop(&mut self) {
        _ = std::fs::remove_file(&self.0);
    }
}

#[derive(Deserialize, Default, Debug, Clone)]
pub struct Config {
    /// The path of the directory that should contain the image files.
    pub path: String,
}

#[derive(Debug, Clone)]
pub struct Images {
    path: PathBuf,
}

impl Images {
    pub async fn new(config: &Config) -> Result<Self, ImageError> {
        let images = Self {
            path: PathBuf::from(&config.path),
        };

        // Anything still in here was cut short by Gofer stopping.
        if let Err(e) = tokio::fs::remove_dir_all(images.uploads_path()).await
            && e.kind() != std::io::ErrorKind::NotFound
        {
            return Err(internal(e));
        }

        for path in [images.uploads_path(), images.blobs_path()] {
            tokio::fs::create_dir_all(&path).await.map_err(|e| {
                ImageError::Internal(format!(
                    "could not create directory '{}'; {e}",
                    path.display()
                ))
            })?;
        }

        Ok(images)
    }

    // Uploads live next to the blobs so that moving one into place is a rename and not a copy.
    fn uploads_path(&self) -> PathBuf {
        self.path.join("uploads")
    }

    fn blobs_path(&self) -> PathBuf {
        self.path.join("blobs").join("sha256")
    }

    fn blob_path(&self, digest: &Digest) -> PathBuf {
        self.blobs_path().join(digest.hex())
    }

    pub async fn contains(&self, digest: &Digest) -> Result<bool, ImageError> {
        tokio::fs::try_exists(self.blob_path(digest))
            .await
            .map_err(internal)
    }

    /// Stores a blob, as long as its contents match its digest.
    ///
    /// The stream's errors are only ever printed, so any type that can be printed will do. That keeps the API's
    /// `HttpError` out of here.
    pub async fn put<E: Display>(
        &self,
        digest: &Digest,
        content: impl Stream<Item = Result<Bytes, E>>,
    ) -> Result<Blob, ImageError> {
        let upload = Upload(
            self.uploads_path()
                .join(uuid::Uuid::now_v7().simple().to_string()),
        );

        let file = tokio::fs::File::create(&upload.0).await.map_err(internal)?;
        let mut writer = BufWriter::new(file);
        let mut hasher = Sha256::new();
        let mut size = 0;

        // Streams made from async blocks can hold references to themselves, so they can't move once they've started.
        tokio::pin!(content);

        while let Some(chunk) = content.next().await {
            let chunk = chunk
                .map_err(|e| ImageError::Invalid(format!("blob was not fully uploaded; {e}")))?;

            hasher.update(&chunk);
            size += chunk.len() as u64;
            writer.write_all(&chunk).await.map_err(internal)?;
        }

        let actual_digest = Digest::from_hasher(hasher);

        if &actual_digest != digest {
            return Err(ImageError::Invalid(format!(
                "blob does not match its digest; expected '{digest}' but its contents are '{actual_digest}'"
            )));
        }

        // The rename below means "this blob is whole", so its contents have to be on disk before it.
        writer.flush().await.map_err(internal)?;
        writer.get_ref().sync_all().await.map_err(internal)?;

        let upload_path = upload.0.clone();
        let diff_id = tokio::task::spawn_blocking(move || diff_id(&upload_path))
            .await
            .map_err(internal)?
            .map_err(|e| ImageError::Invalid(format!("blob could not be decompressed; {e}")))?;

        tokio::fs::rename(&upload.0, self.blob_path(digest))
            .await
            .map_err(internal)?;

        Ok(Blob { size, diff_id })
    }

    /// Checks that an image is one Gofer can run and that the pipeline uploaded every part of it.
    ///
    /// A pipeline can only use blobs it uploaded itself. Otherwise anyone able to register a config could name a layer
    /// from someone else's image, run it, and read the files inside.
    pub async fn verify(
        &self,
        conn: &mut SqliteConnection,
        namespace_id: &str,
        pipeline_id: &str,
        digest: &Digest,
    ) -> Result<(), ImageError> {
        let owner = (namespace_id, pipeline_id);

        let blob = self.uploaded_blob(conn, owner, digest).await?;
        let manifest_type: ManifestType = self.read_json(digest, blob.size).await?;

        // Checked before parsing the rest so that other types get a clear error instead of one about their fields.
        if !MANIFEST_TYPES.contains(&manifest_type.media_type.as_str()) {
            return Err(ImageError::Invalid(format!(
                "image manifest '{digest}' has an unsupported type '{}'; it must be a single image",
                manifest_type.media_type
            )));
        }

        let manifest: Manifest = self.read_json(digest, blob.size).await?;
        let (config_digest, blob) = self.described_blob(conn, owner, &manifest.config).await?;
        let config: ImageConfig = self.read_json(&config_digest, blob.size).await?;

        if let Some(field) = config.lookalike_field() {
            return Err(ImageError::Invalid(format!(
                "image config '{config_digest}' has the field '{field}', which can be mistaken for another"
            )));
        }

        if config.os != IMAGE_OS || config.architecture != IMAGE_ARCHITECTURE {
            return Err(ImageError::Invalid(format!(
                "image is for '{}/{}'; it must be for '{IMAGE_OS}/{IMAGE_ARCHITECTURE}'",
                config.os, config.architecture
            )));
        }

        let mut diff_ids = vec![];

        for layer in &manifest.layers {
            let (_, blob) = self.described_blob(conn, owner, layer).await?;
            diff_ids.push(blob.diff_id.to_string());
        }

        // Docker finds layers by the diff ids in the config. When it already has one, say from another pipeline's
        // image, it uses that and never looks at the layer it was sent. So the ids have to be the ones we worked out
        // from the layers themselves.
        if config.rootfs.diff_ids != diff_ids {
            return Err(ImageError::Invalid(format!(
                "image config '{config_digest}' does not match the image's layers"
            )));
        }

        Ok(())
    }

    /// Reads what a registered image is made of, or `None` if any part of it is gone from the store.
    pub async fn stored_image(&self, manifest: &Digest) -> Result<Option<StoredImage>, ImageError> {
        let Some(manifest_size) = self.size(manifest).await? else {
            return Ok(None);
        };

        let manifest_type: ManifestType = self.read_json(manifest, manifest_size).await?;
        let contents: Manifest = self.read_json(manifest, manifest_size).await?;

        let config: Digest = contents.config.digest.parse()?;
        let Some(config_size) = self.size(&config).await? else {
            return Ok(None);
        };
        let config_contents: ImageConfig = self.read_json(&config, config_size).await?;

        let mut layers = vec![];
        for layer in &contents.layers {
            let layer: Digest = layer.digest.parse()?;
            if self.size(&layer).await?.is_none() {
                return Ok(None);
            }
            layers.push(layer);
        }

        Ok(Some(StoredImage {
            manifest: manifest.clone(),
            manifest_type: manifest_type.media_type,
            manifest_size,
            config,
            layers,
            diff_ids: config_contents.rootfs.diff_ids,
        }))
    }

    async fn size(&self, digest: &Digest) -> Result<Option<u64>, ImageError> {
        match tokio::fs::metadata(self.blob_path(digest)).await {
            Ok(metadata) => Ok(Some(metadata.len())),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(internal(e)),
        }
    }

    /// The image as a tar that `docker load` takes, read from the store as it's sent, so a large image is never in
    /// memory all at once. Docker knows it as [`Digest::docker_name`] afterwards.
    ///
    /// It has both Docker's own `manifest.json` and an OCI layout. Docker's older image store refuses a tar without the
    /// first. Its newer containerd store makes up a different manifest without the second, so the image wouldn't have
    /// the digest it was registered with.
    pub fn docker_archive(
        &self,
        image: &StoredImage,
    ) -> impl Stream<Item = std::io::Result<Bytes>> + Send + 'static {
        let name = image.manifest.docker_name();
        let blob_name = |digest: &Digest| format!("blobs/sha256/{}", digest.hex());

        let manifest_json = serde_json::json!([{
            "Config": blob_name(&image.config),
            "RepoTags": [name],
            "Layers": image.layers.iter().map(blob_name).collect::<Vec<_>>(),
        }]);

        let index_json = serde_json::json!({
            "schemaVersion": 2,
            "mediaType": "application/vnd.oci.image.index.v1+json",
            "manifests": [{
                "mediaType": image.manifest_type,
                "digest": image.manifest.to_string(),
                "size": image.manifest_size,
                "annotations": {
                    "io.containerd.image.name": format!("docker.io/library/{name}"),
                    "org.opencontainers.image.ref.name": image.manifest.hex(),
                },
            }],
        });

        let mut entries = vec![
            ArchiveEntry::Text("manifest.json".into(), manifest_json.to_string()),
            ArchiveEntry::Text("index.json".into(), index_json.to_string()),
            ArchiveEntry::Text(
                "oci-layout".into(),
                r#"{"imageLayoutVersion":"1.0.0"}"#.into(),
            ),
        ];

        // An image can list the same layer twice, but the tar only needs it once.
        let mut blobs: Vec<&Digest> = vec![&image.manifest, &image.config];
        for layer in &image.layers {
            if !blobs.contains(&layer) {
                blobs.push(layer);
            }
        }
        for digest in blobs {
            entries.push(ArchiveEntry::Blob(
                blob_name(digest),
                self.blob_path(digest),
            ));
        }

        futures::stream::iter(entries)
            .flat_map(ArchiveEntry::into_tar)
            // A tar ends with two empty blocks.
            .chain(futures::stream::once(async {
                Ok(Bytes::from_static(&[0; 1024]))
            }))
    }

    /// Returns a blob by its digest, provided the pipeline uploaded it and it's still in the store.
    async fn uploaded_blob(
        &self,
        conn: &mut SqliteConnection,
        (namespace_id, pipeline_id): (&str, &str),
        digest: &Digest,
    ) -> Result<Blob, ImageError> {
        let blob =
            match storage::image_blobs::get(conn, namespace_id, pipeline_id, &digest.to_string())
                .await
            {
                Ok(blob) if self.contains(digest).await? => blob,
                Ok(_) | Err(StorageError::NotFound) => {
                    return Err(ImageError::Invalid(format!(
                        "image blob '{digest}' has not been uploaded for pipeline '{pipeline_id}'"
                    )));
                }
                Err(e) => return Err(internal(e)),
            };

        Ok(Blob {
            size: blob.size.try_into().map_err(internal)?,
            diff_id: blob.diff_id.parse().map_err(internal)?,
        })
    }

    /// Returns the blob that part of a manifest describes.
    async fn described_blob(
        &self,
        conn: &mut SqliteConnection,
        owner: (&str, &str),
        descriptor: &Descriptor,
    ) -> Result<(Digest, Blob), ImageError> {
        let digest: Digest = descriptor.digest.parse()?;

        if !descriptor.urls.is_empty() {
            return Err(ImageError::Invalid(format!(
                "image blob '{digest}' is stored somewhere else; every part of an image must be uploaded"
            )));
        }

        let blob = self.uploaded_blob(conn, owner, &digest).await?;

        if blob.size != descriptor.size {
            return Err(ImageError::Invalid(format!(
                "image blob '{digest}' is {} bytes but the image says it should be {}",
                blob.size, descriptor.size
            )));
        }

        Ok((digest, blob))
    }

    async fn read_json<T: DeserializeOwned>(
        &self,
        digest: &Digest,
        size: u64,
    ) -> Result<T, ImageError> {
        if size > MAX_JSON_BYTES {
            return Err(ImageError::Invalid(format!(
                "image blob '{digest}' is too large; it must be under {MAX_JSON_BYTES} bytes"
            )));
        }

        let content = tokio::fs::read(self.blob_path(digest))
            .await
            .map_err(internal)?;

        serde_json::from_slice(&content).map_err(|e| {
            ImageError::Invalid(format!("image blob '{digest}' could not be parsed; {e}"))
        })
    }
}

/// What a registered image is made of, read from its manifest and config.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredImage {
    pub manifest: Digest,

    /// The manifest's own media type, which `docker load` has to be told.
    manifest_type: String,

    manifest_size: u64,

    pub config: Digest,

    pub layers: Vec<Digest>,

    /// The digest of each layer's decompressed contents, in order, as the config lists them. Registration checked them
    /// against the layers themselves, so they're what Docker's copy of the image has to have too.
    pub diff_ids: Vec<String>,
}

/// One file in the tar that [`Images::docker_archive`] makes.
enum ArchiveEntry {
    Text(String, String),

    /// A blob from the store, which is only read while it's being sent.
    Blob(String, PathBuf),
}

impl ArchiveEntry {
    fn into_tar(self) -> Pin<Box<dyn Stream<Item = std::io::Result<Bytes>> + Send>> {
        match self {
            ArchiveEntry::Text(name, text) => {
                let header = tar_header(&name, text.len() as u64);
                let padding = tar_padding(text.len() as u64);
                Box::pin(futures::stream::iter([
                    header,
                    Ok(Bytes::from(text)),
                    Ok(padding),
                ]))
            }

            ArchiveEntry::Blob(name, path) => Box::pin(
                futures::stream::once(async move {
                    let file = tokio::fs::File::open(&path).await?;
                    let size = file.metadata().await?.len();
                    let contents = tokio_util::io::ReaderStream::new(file.take(size));

                    Ok::<_, std::io::Error>(
                        futures::stream::once(async move { tar_header(&name, size) })
                            .chain(contents)
                            .chain(futures::stream::once(async move { Ok(tar_padding(size)) })),
                    )
                })
                .try_flatten(),
            ),
        }
    }
}

fn tar_header(name: &str, size: u64) -> std::io::Result<Bytes> {
    let mut header = tar::Header::new_ustar();
    header.set_path(name)?;
    header.set_size(size);
    header.set_mode(0o644);
    header.set_mtime(0);
    header.set_entry_type(tar::EntryType::Regular);
    header.set_cksum();
    Ok(Bytes::copy_from_slice(header.as_bytes()))
}

/// A tar stores each file in 512 byte blocks, so the last one is filled out with zeros.
fn tar_padding(size: u64) -> Bytes {
    Bytes::from(vec![0; ((512 - size % 512) % 512) as usize])
}

/// Works out the digest of a file's decompressed contents.
fn diff_id(path: &Path) -> std::io::Result<Digest> {
    let mut file = std::io::BufReader::with_capacity(64 * 1024, std::fs::File::open(path)?);

    // Read the magic number at the start of the file to find out how it's compressed, if it is. This only looks at it,
    // so the decoder still gets the whole file.
    let start = file.fill_buf()?;

    let reader: Box<dyn Read> = if start.starts_with(&GZIP_MAGIC_NUMBER) {
        Box::new(MultiGzDecoder::new(file))
    } else if start.starts_with(&ZSTD_MAGIC_NUMBER) {
        Box::new(zstd::Decoder::with_buffer(file)?)
    } else {
        Box::new(file)
    };

    // A small file can decompress into an enormous one, so stop once it's larger than anything we'd accept.
    let mut reader = reader.take(MAX_BLOB_BYTES as u64 + 1);
    let mut hasher = Sha256::new();
    let mut buffer = vec![0; 64 * 1024];
    let mut size = 0;

    loop {
        let read = reader.read(&mut buffer)?;

        if read == 0 {
            break;
        }

        hasher.update(&buffer[..read]);
        size += read;
    }

    if size > MAX_BLOB_BYTES {
        return Err(std::io::Error::other(format!(
            "it's larger than {MAX_BLOB_BYTES} bytes"
        )));
    }

    Ok(Digest::from_hasher(hasher))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{image_blobs::ImageBlob, tests::TestHarness};
    use flate2::{Compression, write::GzEncoder};
    use futures::stream;
    use pretty_assertions::assert_eq;
    use rstest::rstest;
    use serde_json::json;
    use sqlx::{Sqlite, pool::PoolConnection};
    use std::io::Write;

    const NAMESPACE: &str = "some-namespace";
    const PIPELINE: &str = "some-pipeline";

    struct Harness {
        images: Images,
        conn: PoolConnection<Sqlite>,
        dir: tempfile::TempDir,
        // Dropped last so the database file outlives its connection.
        _db: TestHarness,
    }

    fn digest(content: &[u8]) -> Digest {
        Digest::from_hasher(Sha256::new_with_prefix(content))
    }

    fn gzip(content: &[u8]) -> Vec<u8> {
        let mut encoder = GzEncoder::new(vec![], Compression::default());
        encoder.write_all(content).unwrap();
        encoder.finish().unwrap()
    }

    /// The message of an error that's down to what we were given.
    fn invalid_message<T: std::fmt::Debug>(result: Result<T, ImageError>) -> String {
        match result {
            Err(ImageError::Invalid(message)) => message,
            other => panic!("expected it to be invalid; got {other:?}"),
        }
    }

    fn chunks(content: &[u8]) -> impl Stream<Item = Result<Bytes, std::io::Error>> {
        stream::iter(
            content
                .chunks(3)
                .map(|chunk| Ok(Bytes::copy_from_slice(chunk)))
                .collect::<Vec<_>>(),
        )
    }

    impl Harness {
        async fn new() -> Self {
            let dir = tempfile::tempdir().unwrap();
            let images = Images::new(&Config {
                path: dir.path().to_string_lossy().into(),
            })
            .await
            .unwrap();

            let db = TestHarness::new().await;
            let mut conn = db.write_conn().await.unwrap();

            storage::namespaces::insert(
                &mut conn,
                &storage::namespaces::Namespace {
                    id: NAMESPACE.into(),
                    name: "some_name".into(),
                    description: "some_description".into(),
                    created: "0".into(),
                    modified: "0".into(),
                },
            )
            .await
            .unwrap();

            Self {
                images,
                conn,
                dir,
                _db: db,
            }
        }

        /// Uploads a blob the way the API does.
        async fn upload(&mut self, pipeline_id: &str, content: &[u8]) -> Digest {
            let digest = digest(content);
            let blob = self.images.put(&digest, chunks(content)).await.unwrap();

            storage::image_blobs::upsert(
                &mut self.conn,
                &ImageBlob {
                    namespace_id: NAMESPACE.into(),
                    pipeline_id: pipeline_id.into(),
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

        async fn verify(&mut self, digest: &Digest) -> Result<(), ImageError> {
            self.images
                .verify(&mut self.conn, NAMESPACE, PIPELINE, digest)
                .await
        }

        /// Uploads an image for a pipeline, with a chance to change its config and manifest first.
        async fn upload_image(
            &mut self,
            pipeline_id: &str,
            change_config: impl FnOnce(&mut serde_json::Value),
            change_manifest: impl FnOnce(&mut serde_json::Value),
        ) -> Digest {
            let layer = gzip(b"some layer");

            let mut config = json!({
                "os": "linux",
                "architecture": "amd64",
                "os.version": "1",
                "created": "2026-11-24T00:00:00Z",
                "history": [],
                "config": { "Cmd": ["true"] },
                "rootfs": { "type": "layers", "diff_ids": [digest(b"some layer").to_string()] },
            });
            change_config(&mut config);
            let config = config.to_string().into_bytes();

            let mut manifest = json!({
                "schemaVersion": 2,
                "mediaType": "application/vnd.oci.image.manifest.v1+json",
                "config": {
                    "mediaType": "application/vnd.oci.image.config.v1+json",
                    "digest": digest(&config).to_string(),
                    "size": config.len(),
                },
                "layers": [{
                    "mediaType": "application/vnd.oci.image.layer.v1.tar+gzip",
                    "digest": digest(&layer).to_string(),
                    "size": layer.len(),
                }],
            });
            change_manifest(&mut manifest);

            self.upload(pipeline_id, &layer).await;
            self.upload(pipeline_id, &config).await;
            self.upload(pipeline_id, manifest.to_string().as_bytes())
                .await
        }

        /// Every file in the image store.
        fn files(&self) -> Vec<PathBuf> {
            ["uploads", "blobs/sha256"]
                .iter()
                .flat_map(|dir| std::fs::read_dir(self.dir.path().join(dir)).unwrap())
                .map(|entry| entry.unwrap().path())
                .collect()
        }
    }

    #[rstest]
    #[case::empty("")]
    #[case::no_algorithm("2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824")]
    #[case::other_algorithm(
        "sha512:2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
    )]
    #[case::uppercase("sha256:2CF24DBA5FB0A30E26E83B2AC5B9E29E1B161E5C1FA7425E73043362938B9824")]
    #[case::short("sha256:2cf24dba")]
    #[case::trailing_newline(
        "sha256:2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824\n"
    )]
    #[case::path("sha256:../../../../../../../../../../../../../../../../../../../../../../etc")]
    fn invalid_digests(#[case] value: &str) {
        assert!(matches!(
            value.parse::<Digest>(),
            Err(ImageError::Invalid(_))
        ));
    }

    #[rstest]
    #[case::plain(b"some content".to_vec())]
    #[case::gzip(gzip(b"some content"))]
    #[case::gzip_in_parts([gzip(b"some "), gzip(b"content")].concat())]
    #[case::zstd(zstd::encode_all(&b"some content"[..], 0).unwrap())]
    #[tokio::test]
    async fn put_stores_blob(#[case] content: Vec<u8>) {
        let harness = Harness::new().await;
        let digest_of_content = digest(&content);

        let blob = harness
            .images
            .put(&digest_of_content, chunks(&content))
            .await
            .unwrap();

        assert_eq!(blob.size, content.len() as u64);
        assert_eq!(blob.diff_id, digest(b"some content"));
        assert!(harness.images.contains(&digest_of_content).await.unwrap());
        assert_eq!(
            harness.files(),
            vec![harness.images.blob_path(&digest_of_content)]
        );
        assert_eq!(std::fs::read(&harness.files()[0]).unwrap(), content);
    }

    #[tokio::test]
    async fn put_twice() {
        let harness = Harness::new().await;
        let digest = digest(b"some content");

        for _ in 0..2 {
            harness
                .images
                .put(&digest, chunks(b"some content"))
                .await
                .unwrap();
        }

        assert_eq!(harness.files().len(), 1);
    }

    #[tokio::test]
    async fn put_with_wrong_digest_leaves_nothing() {
        let harness = Harness::new().await;

        let result = harness
            .images
            .put(&digest(b"some content"), chunks(b"other content"))
            .await;

        let message = invalid_message(result);
        assert!(message.contains("does not match its digest"), "{message}");
        assert_eq!(harness.files(), Vec::<PathBuf>::new());
    }

    #[tokio::test]
    async fn put_that_is_cut_off_leaves_nothing() {
        let harness = Harness::new().await;

        let content = chunks(b"some content").chain(stream::iter([Err(std::io::Error::other(
            "connection closed",
        ))]));
        let result = harness.images.put(&digest(b"some content"), content).await;

        let message = invalid_message(result);
        assert!(
            message.contains("was not fully uploaded; connection closed"),
            "{message}"
        );
        assert_eq!(harness.files(), Vec::<PathBuf>::new());
    }

    #[tokio::test]
    async fn put_that_can_not_be_decompressed_leaves_nothing() {
        let harness = Harness::new().await;
        let mut content = gzip(b"some content");
        content.truncate(content.len() - 4);

        let result = harness
            .images
            .put(&digest(&content), chunks(&content))
            .await;

        let message = invalid_message(result);
        assert!(message.contains("could not be decompressed"), "{message}");
        assert_eq!(harness.files(), Vec::<PathBuf>::new());
    }

    #[tokio::test]
    async fn unfinished_uploads_are_removed_on_start() {
        let harness = Harness::new().await;
        let digest = digest(b"some content");
        harness
            .images
            .put(&digest, chunks(b"some content"))
            .await
            .unwrap();
        std::fs::write(harness.images.uploads_path().join("unfinished"), "some").unwrap();

        let images = Images::new(&Config {
            path: harness.dir.path().to_string_lossy().into(),
        })
        .await
        .unwrap();

        assert_eq!(harness.files(), vec![images.blob_path(&digest)]);
    }

    #[tokio::test]
    async fn verify_image() {
        let mut harness = Harness::new().await;
        let digest = harness.upload_image(PIPELINE, |_| {}, |_| {}).await;

        assert_eq!(harness.verify(&digest).await, Ok(()));
    }

    #[rstest]
    #[case::wrong_architecture(|config: &mut serde_json::Value| config["architecture"] = json!("arm64"), "image is for 'linux/arm64'")]
    #[case::wrong_os(|config: &mut serde_json::Value| config["os"] = json!("windows"), "image is for 'windows/amd64'")]
    #[case::no_platform(|config: &mut serde_json::Value| *config = json!({ "rootfs": config["rootfs"] }), "image is for '/'")]
    // What an image that's after another pipeline's layer looks like.
    #[case::other_diff_id(|config: &mut serde_json::Value| {
        config["rootfs"]["diff_ids"] = json!([digest(b"other layer").to_string()])
    }, "does not match the image's layers")]
    #[case::extra_diff_id(|config: &mut serde_json::Value| {
        config["rootfs"]["diff_ids"].as_array_mut().unwrap().push(json!(digest(b"other layer").to_string()))
    }, "does not match the image's layers")]
    #[case::field_with_other_case(|config: &mut serde_json::Value| {
        config["Rootfs"] = json!({ "diff_ids": [digest(b"other layer").to_string()] })
    }, "has the field 'Rootfs'")]
    #[case::field_with_lookalike_letter(|config: &mut serde_json::Value| {
        config["rootfſ"] = json!({ "diff_ids": [digest(b"other layer").to_string()] })
    }, "has the field 'rootfſ'")]
    #[case::nested_field_with_other_case(|config: &mut serde_json::Value| {
        config["rootfs"]["Diff_ids"] = json!([digest(b"other layer").to_string()])
    }, "unknown field `Diff_ids`")]
    #[case::no_diff_ids(|config: &mut serde_json::Value| config["rootfs"] = json!({}), "does not match the image's layers")]
    #[tokio::test]
    async fn verify_rejects_config(
        #[case] change: fn(&mut serde_json::Value),
        #[case] expected: &str,
    ) {
        let mut harness = Harness::new().await;
        let digest = harness.upload_image(PIPELINE, change, |_| {}).await;

        let message = invalid_message(harness.verify(&digest).await);
        assert!(message.contains(expected), "{message}");
    }

    #[rstest]
    #[case::index(|manifest: &mut serde_json::Value| {
        *manifest = json!({
            "schemaVersion": 2,
            "mediaType": "application/vnd.oci.image.index.v1+json",
            "manifests": [],
        })
    }, "unsupported type 'application/vnd.oci.image.index.v1+json'")]
    #[case::index_type(|manifest: &mut serde_json::Value| {
        manifest["mediaType"] = json!("application/vnd.oci.image.index.v1+json")
    }, "unsupported type 'application/vnd.oci.image.index.v1+json'")]
    #[case::no_type(|manifest: &mut serde_json::Value| {
        manifest.as_object_mut().unwrap().remove("mediaType");
    }, "unsupported type ''")]
    #[case::not_an_object(|manifest: &mut serde_json::Value| *manifest = json!([]), "unsupported type ''")]
    #[case::no_config(|manifest: &mut serde_json::Value| {
        manifest.as_object_mut().unwrap().remove("config");
    }, "'' is not a valid digest")]
    #[case::external_url(|manifest: &mut serde_json::Value| {
        manifest["layers"][0]["urls"] = json!(["https://example.com/layer"])
    }, "is stored somewhere else")]
    #[case::wrong_size(|manifest: &mut serde_json::Value| manifest["layers"][0]["size"] = json!(1), "but the image says it should be 1")]
    #[case::missing_layer(|manifest: &mut serde_json::Value| {
        manifest["layers"][0]["digest"] = json!(digest(b"never uploaded").to_string())
    }, "has not been uploaded for pipeline 'some-pipeline'")]
    #[case::bad_digest(|manifest: &mut serde_json::Value| {
        manifest["layers"][0]["digest"] = json!("sha256:../../secret")
    }, "'sha256:../../secret' is not a valid digest")]
    #[case::field_with_other_case(|manifest: &mut serde_json::Value| {
        manifest["Layers"] = manifest["layers"].clone()
    }, "unknown field `Layers`")]
    #[case::also_an_index(|manifest: &mut serde_json::Value| manifest["manifests"] = json!([]), "unknown field `manifests`")]
    #[case::unknown_field(|manifest: &mut serde_json::Value| manifest["layers"][0]["other"] = json!("x"), "unknown field `other`")]
    #[tokio::test]
    async fn verify_rejects_manifest(
        #[case] change: fn(&mut serde_json::Value),
        #[case] expected: &str,
    ) {
        let mut harness = Harness::new().await;
        let digest = harness.upload_image(PIPELINE, |_| {}, change).await;

        let message = invalid_message(harness.verify(&digest).await);
        assert!(message.contains(expected), "{message}");
    }

    // Docker and serde_json don't agree on which one of a repeated field to use.
    #[rstest]
    #[case::config(serde_json::from_str::<ImageConfig>(r#"{"rootfs":{},"rootfs":{}}"#).err())]
    #[case::manifest(serde_json::from_str::<Manifest>(r#"{"layers":[],"layers":[]}"#).err())]
    fn repeated_fields_are_refused(#[case] error: Option<serde_json::Error>) {
        let message = error.unwrap().to_string();
        assert!(message.contains("duplicate field"), "{message}");
    }

    // Some official images do this.
    #[tokio::test]
    async fn verify_image_with_embedded_config() {
        let mut harness = Harness::new().await;
        let digest = harness
            .upload_image(
                PIPELINE,
                |_| {},
                |manifest| manifest["config"]["data"] = json!("e30="),
            )
            .await;

        assert_eq!(harness.verify(&digest).await, Ok(()));
    }

    #[tokio::test]
    async fn verify_rejects_other_pipelines_image() {
        let mut harness = Harness::new().await;
        let digest = harness.upload_image("other-pipeline", |_| {}, |_| {}).await;

        let message = invalid_message(harness.verify(&digest).await);
        assert!(
            message.contains("has not been uploaded for pipeline 'some-pipeline'"),
            "{message}"
        );
    }

    #[tokio::test]
    async fn verify_rejects_other_pipelines_layer() {
        let mut harness = Harness::new().await;
        let layer = gzip(b"other layer");
        harness.upload("other-pipeline", &layer).await;

        let digest = harness
            .upload_image(
                PIPELINE,
                |config| config["rootfs"]["diff_ids"] = json!([digest(b"other layer").to_string()]),
                |manifest| {
                    manifest["layers"][0]["digest"] = json!(digest(&layer).to_string());
                    manifest["layers"][0]["size"] = json!(layer.len());
                },
            )
            .await;

        let message = invalid_message(harness.verify(&digest).await);
        assert!(
            message.contains("has not been uploaded for pipeline 'some-pipeline'"),
            "{message}"
        );
    }

    #[tokio::test]
    async fn docker_archive_has_what_docker_load_needs() {
        let mut harness = Harness::new().await;
        let manifest = harness.upload_image(PIPELINE, |_| {}, |_| {}).await;
        let image = harness
            .images
            .stored_image(&manifest)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(image.diff_ids, vec![digest(b"some layer").to_string()]);

        let tar: Vec<u8> = harness
            .images
            .docker_archive(&image)
            .try_collect::<Vec<Bytes>>()
            .await
            .unwrap()
            .concat();

        let mut files = HashMap::new();
        for entry in tar::Archive::new(tar.as_slice()).entries().unwrap() {
            let mut entry = entry.unwrap();
            let mut content = vec![];
            entry.read_to_end(&mut content).unwrap();
            files.insert(entry.path().unwrap().display().to_string(), content);
        }

        let blob = |digest: &Digest| format!("blobs/sha256/{}", digest.hex());
        let mut names: Vec<&str> = files.keys().map(String::as_str).collect();
        names.sort();
        let mut expected = vec![
            "index.json".to_string(),
            "manifest.json".to_string(),
            "oci-layout".to_string(),
            blob(&manifest),
            blob(&image.config),
            blob(&image.layers[0]),
        ];
        expected.sort();
        assert_eq!(names, expected);

        // Blobs come out exactly as they were uploaded.
        for digest in [&manifest, &image.config, &image.layers[0]] {
            assert_eq!(
                files[&blob(digest)],
                std::fs::read(harness.images.blob_path(digest)).unwrap()
            );
        }

        let docker_manifest: serde_json::Value =
            serde_json::from_slice(&files["manifest.json"]).unwrap();
        assert_eq!(
            docker_manifest[0]["RepoTags"],
            json!([manifest.docker_name()])
        );

        let index: serde_json::Value = serde_json::from_slice(&files["index.json"]).unwrap();
        assert_eq!(index["manifests"][0]["digest"], json!(manifest.to_string()));
    }

    #[tokio::test]
    async fn stored_image_is_none_once_a_part_is_gone() {
        let mut harness = Harness::new().await;
        let manifest = harness.upload_image(PIPELINE, |_| {}, |_| {}).await;
        std::fs::remove_file(harness.images.blob_path(&digest(&gzip(b"some layer")))).unwrap();

        assert_eq!(harness.images.stored_image(&manifest).await.unwrap(), None);
    }

    #[tokio::test]
    async fn verify_rejects_blob_missing_from_disk() {
        let mut harness = Harness::new().await;
        let digest = harness.upload_image(PIPELINE, |_| {}, |_| {}).await;
        std::fs::remove_file(
            harness
                .images
                .blob_path(&super::tests::digest(&gzip(b"some layer"))),
        )
        .unwrap();

        let message = invalid_message(harness.verify(&digest).await);
        assert!(
            message.contains("has not been uploaded for pipeline 'some-pipeline'"),
            "{message}"
        );
    }

    #[tokio::test]
    async fn verify_rejects_large_manifest() {
        let mut harness = Harness::new().await;
        let digest = harness
            .upload_image(
                PIPELINE,
                |_| {},
                |manifest| {
                    manifest["annotations"] = json!({ "a": "a".repeat(MAX_JSON_BYTES as usize) })
                },
            )
            .await;

        let message = invalid_message(harness.verify(&digest).await);
        assert!(message.contains("is too large"), "{message}");
    }
}
