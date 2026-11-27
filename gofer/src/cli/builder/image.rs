//! Takes apart the image that BuildKit sends back and gets it ready to upload.
//!
//! BuildKit sends a tar in Docker's format. Under `blobs/sha256/` it has every blob of the image, named by its digest,
//! and `index.json` says which of them is the manifest. We only keep the blobs, which are what the server takes.

use super::direct::Registry;
use crate::images::Digest;
use anyhow::{Context, Result, bail};
use bytes::Bytes;
use futures::{StreamExt, stream::BoxStream};
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest as _, Sha256};
use std::{
    fs::File,
    io::{self, Read, Write},
    path::{Path, PathBuf},
    time::Duration,
};

/// The largest the index, manifest or config can be. They're read into memory and normally only a few KB.
pub const MAX_JSON_BYTES: u64 = 4 * 1024 * 1024;

/// A built image, as a folder with a file per blob.
pub struct Image {
    folder: tempfile::TempDir,

    pub manifest: Digest,

    /// Every blob the image is made of, with its size: its layers, config and manifest.
    pub blobs: Vec<(Digest, u64)>,

    /// For an image made without BuildKit, where its base's layers are. They're only downloaded if Gofer doesn't have
    /// them already, so they aren't in the folder.
    registry: Option<Registry>,
}

impl Image {
    pub fn blob_path(&self, digest: &Digest) -> PathBuf {
        blob_path(self.folder.path(), digest)
    }

    /// Whether the blob is on this machine, rather than still in the base's registry.
    pub fn has(&self, digest: &Digest) -> bool {
        self.blob_path(digest).exists()
    }

    /// A blob's contents, read from this machine or downloaded from the base's registry.
    pub async fn open(&self, digest: &Digest) -> Result<BoxStream<'static, io::Result<Bytes>>> {
        if self.has(digest) {
            let path = self.blob_path(digest);
            let file = tokio::fs::File::open(&path)
                .await
                .with_context(|| format!("Could not read '{}'", path.display()))?;
            return Ok(tokio_util::io::ReaderStream::new(file).boxed());
        }

        match &self.registry {
            Some(registry) => registry.blob(digest).await,
            None => bail!("The image has no blob {digest}"),
        }
    }

    /// How many bytes it takes up, compressed, which is how it's uploaded.
    pub fn size(&self) -> u64 {
        self.blobs.iter().map(|(_, size)| size).sum()
    }
}

/// Makes a folder to unpack an image into, which removes itself when it's dropped.
pub fn folder() -> Result<tempfile::TempDir> {
    // Not /tmp, which is often in memory, and images can be large.
    let parent: PathBuf = dirs::cache_dir()
        .context("Could not find a cache folder to unpack images in")?
        .join("gofer")
        .join("images");
    std::fs::create_dir_all(&parent)
        .with_context(|| format!("Could not create '{}'", parent.display()))?;

    // A CLI that was killed never got to remove its folder.
    let day_ago = std::time::SystemTime::now() - Duration::from_secs(24 * 60 * 60);
    for old in std::fs::read_dir(&parent)?.flatten() {
        let modified = old.metadata().and_then(|metadata| metadata.modified());
        if modified.is_ok_and(|modified| modified < day_ago) {
            _ = std::fs::remove_dir_all(old.path());
        }
    }

    tempfile::Builder::new()
        .prefix("build-")
        .tempdir_in(&parent)
        .with_context(|| format!("Could not create a folder in '{}'", parent.display()))
}

#[derive(Deserialize)]
struct Index {
    manifests: Vec<Descriptor>,
}

#[derive(Deserialize)]
struct Descriptor {
    digest: String,
}

fn blob_path(folder: &Path, digest: &Digest) -> PathBuf {
    folder.join(digest.hex())
}

/// Reads the tar BuildKit sent into `folder`, checking that every blob matches its digest, and returns the manifest's
/// digest. This reads as the tar arrives, so the whole image is never in memory.
pub fn unpack(tar: impl Read, folder: &Path) -> Result<Digest> {
    let mut index = None;

    for entry in tar::Archive::new(tar).entries()? {
        let mut entry = entry?;
        let path = entry.path()?.to_string_lossy().to_string();

        if path == "index.json" {
            index =
                Some(read_json::<Index>(&mut entry).context("Could not read the image's index")?);
        } else if let Some(hex) = path.strip_prefix("blobs/sha256/")
            && !hex.is_empty()
        {
            let digest: Digest = format!("sha256:{hex}").parse()?;
            save(&mut entry, folder, &digest)?;
        }
    }

    let index = index.context("The image BuildKit sent has no index.json")?;

    // An index can also list attestations or images for other platforms. We only ever ask for one image.
    let [manifest] = index.manifests.as_slice() else {
        bail!(
            "Expected one image from BuildKit but got {}",
            index.manifests.len()
        );
    };

    let manifest: Digest = manifest.digest.parse()?;
    if !blob_path(folder, &manifest).exists() {
        bail!("The image BuildKit sent is missing its manifest");
    }

    Ok(manifest)
}

/// Finishes an image whose manifest and config are in `folder`: bakes in how it's run, and lists its blobs. Its layers
/// have to be in `folder` too, unless they're still in the base's `registry`.
pub fn finish(
    folder: tempfile::TempDir,
    manifest: Digest,
    entrypoint: Option<&[String]>,
    command: Option<&[String]>,
    registry: Option<Registry>,
) -> Result<Image> {
    let path = folder.path();
    let mut manifest_json: Value = read_blob(path, &manifest)?;
    let config: Digest = json_str(&manifest_json["config"]["digest"])?.parse()?;
    let mut config_json: Value = read_blob(path, &config)?;

    let platform = (
        json_str(&config_json["os"])?,
        json_str(&config_json["architecture"])?,
    );
    if platform != ("linux", "amd64") {
        bail!(
            "The image is for {}/{}, but tasks run on linux/amd64",
            platform.0,
            platform.1
        );
    }

    let mut image = Image {
        folder,
        manifest: manifest.clone(),
        blobs: vec![],
        registry,
    };

    if entrypoint.is_some() || command.is_some() {
        set_command(&mut config_json, entrypoint, command);
        let new_config = write_blob(image.folder.path(), &serde_json::to_vec(&config_json)?)?;

        manifest_json["config"]["digest"] = new_config.0.to_string().into();
        manifest_json["config"]["size"] = new_config.1.into();
        let new_manifest = write_blob(image.folder.path(), &serde_json::to_vec(&manifest_json)?)?;

        // Nothing uses the old ones anymore, unless the image already had this command.
        for (old, new) in [(&config, &new_config.0), (&manifest, &new_manifest.0)] {
            if old != new {
                std::fs::remove_file(image.blob_path(old))?;
            }
        }
        image.manifest = new_manifest.0;
    }

    let descriptors = manifest_json["layers"]
        .as_array()
        .context("The image's manifest has no layers")?
        .iter()
        .chain([&manifest_json["config"]]);

    for descriptor in descriptors {
        let digest: Digest = json_str(&descriptor["digest"])?.parse()?;
        let size = (descriptor["size"].as_u64())
            .with_context(|| format!("The image's manifest has no size for {digest}"))?;

        match std::fs::metadata(image.blob_path(&digest)) {
            Ok(file) if file.len() == size => {}
            Err(_) if image.registry.is_some() => {}
            _ => bail!("The image is missing {digest}, or it's the wrong size"),
        }

        image.blobs.push((digest, size));
    }

    let manifest_size = std::fs::metadata(image.blob_path(&image.manifest))?.len();
    image.blobs.push((image.manifest.clone(), manifest_size));
    Ok(image)
}

/// Bakes an entrypoint and command into an image's config. Docker would combine them with the image's own when starting
/// a container, so we do the same thing it would, and starting the new image gives the same result.
///
/// The image's command stays unless there's a new one, except when there's a new entrypoint. The old command was meant
/// for the old entrypoint, so then it goes too.
///
/// <https://github.com/moby/moby/blob/docker-v29.10.1/daemon/commit.go#L73-L81>
pub fn set_command(config: &mut Value, entrypoint: Option<&[String]>, command: Option<&[String]>) {
    if !config["config"].is_object() {
        config["config"] = serde_json::json!({});
    }
    let config = &mut config["config"];

    let new_entrypoint = entrypoint.is_some_and(|entrypoint| !entrypoint.is_empty());

    if let Some(entrypoint) = entrypoint {
        config["Entrypoint"] = entrypoint.into();
    }

    match command {
        Some(command) if !command.is_empty() => config["Cmd"] = command.into(),
        _ if new_entrypoint => config["Cmd"] = Value::Null,
        _ => {}
    }
}

fn json_str(value: &Value) -> Result<&str> {
    value
        .as_str()
        .with_context(|| format!("Expected text in the image's JSON, but got '{value}'"))
}

fn read_json<T: serde::de::DeserializeOwned>(reader: impl Read) -> Result<T> {
    let mut content = vec![];
    reader.take(MAX_JSON_BYTES + 1).read_to_end(&mut content)?;

    if content.len() as u64 > MAX_JSON_BYTES {
        bail!("it's larger than {MAX_JSON_BYTES} bytes");
    }

    Ok(serde_json::from_slice(&content)?)
}

fn read_blob<T: serde::de::DeserializeOwned>(folder: &Path, digest: &Digest) -> Result<T> {
    read_json(File::open(blob_path(folder, digest))?)
        .with_context(|| format!("Could not read {digest}"))
}

/// Saves a blob and returns its digest and size.
pub fn write_blob(folder: &Path, content: &[u8]) -> Result<(Digest, u64)> {
    let digest = Digest::from_hasher(Sha256::new_with_prefix(content));
    std::fs::write(blob_path(folder, &digest), content)?;
    Ok((digest, content.len() as u64))
}

/// Saves a blob as it's read, then keeps it only if it matches its digest.
fn save(mut reader: impl Read, folder: &Path, digest: &Digest) -> Result<()> {
    let partial = folder.join(format!("{}.partial", digest.hex()));
    let mut file = File::create(&partial)?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0; 64 * 1024];

    loop {
        let read = reader.read(&mut buffer)?;

        if read == 0 {
            break;
        }

        hasher.update(&buffer[..read]);
        file.write_all(&buffer[..read])?;
    }

    let actual = Digest::from_hasher(hasher);
    if actual != *digest {
        std::fs::remove_file(&partial)?;
        bail!(
            "A blob BuildKit sent doesn't match its digest; expected {digest} but it was {actual}"
        );
    }

    std::fs::rename(partial, blob_path(folder, digest))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use serde_json::json;

    fn digest(content: &[u8]) -> Digest {
        Digest::from_hasher(Sha256::new_with_prefix(content))
    }

    /// An image like BuildKit makes, with one layer.
    fn built_image(platform: (&str, &str)) -> (Vec<u8>, Digest) {
        let layer = b"pretend this is a tarball".to_vec();
        let config = serde_json::to_vec(&json!({
            "os": platform.0,
            "architecture": platform.1,
            "config": {"Entrypoint": ["/old"], "Cmd": ["old"], "Env": ["PATH=/bin"]},
            "rootfs": {"type": "layers", "diff_ids": [digest(&layer).to_string()]},
        }))
        .unwrap();
        let manifest = serde_json::to_vec(&json!({
            "schemaVersion": 2,
            "mediaType": "application/vnd.docker.distribution.manifest.v2+json",
            "config": {"digest": digest(&config).to_string(), "size": config.len()},
            "layers": [{"digest": digest(&layer).to_string(), "size": layer.len()}],
        }))
        .unwrap();
        let index =
            serde_json::to_vec(&json!({"manifests": [{"digest": digest(&manifest).to_string()}]}))
                .unwrap();

        let tar = tar_of(&[
            (format!("blobs/sha256/{}", digest(&layer).hex()), layer),
            (format!("blobs/sha256/{}", digest(&config).hex()), config),
            (
                format!("blobs/sha256/{}", digest(&manifest).hex()),
                manifest.clone(),
            ),
            ("index.json".into(), index),
            ("manifest.json".into(), b"[]".to_vec()),
        ]);
        (tar, digest(&manifest))
    }

    fn tar_of(files: &[(String, Vec<u8>)]) -> Vec<u8> {
        let mut tar = tar::Builder::new(vec![]);
        for (path, content) in files {
            let mut header = tar::Header::new_gnu();
            header.set_size(content.len() as u64);
            header.set_mode(0o644);
            tar.append_data(&mut header, path, content.as_slice())
                .unwrap();
        }
        tar.into_inner().unwrap()
    }

    fn files_in(folder: &Path) -> usize {
        std::fs::read_dir(folder).unwrap().count()
    }

    #[test]
    fn test_unpacks_and_lists_every_blob() {
        let (tar, manifest) = built_image(("linux", "amd64"));
        let folder = tempfile::tempdir().unwrap();

        assert_eq!(unpack(tar.as_slice(), folder.path()).unwrap(), manifest);

        let image = finish(folder, manifest.clone(), None, None, None).unwrap();
        assert_eq!(image.manifest, manifest);
        assert_eq!(image.blobs.len(), 3);
        for (blob, size) in &image.blobs {
            let content = std::fs::read(image.blob_path(blob)).unwrap();
            assert_eq!(digest(&content), *blob);
            assert_eq!(content.len() as u64, *size);
        }
    }

    #[test]
    fn test_command_is_baked_in() {
        let (tar, manifest) = built_image(("linux", "amd64"));
        let folder = tempfile::tempdir().unwrap();
        unpack(tar.as_slice(), folder.path()).unwrap();

        let command = ["python".to_string(), "/cleanup.py".to_string()];
        let image = finish(folder, manifest.clone(), None, Some(&command), None).unwrap();
        assert_ne!(image.manifest, manifest);

        // The old config and manifest are gone, and the new ones point at each other.
        assert_eq!(files_in(image.folder.path()), 3);
        let manifest: Value = read_blob(image.folder.path(), &image.manifest).unwrap();
        let config: Digest = manifest["config"]["digest"]
            .as_str()
            .unwrap()
            .parse()
            .unwrap();
        let config: Value = read_blob(image.folder.path(), &config).unwrap();
        assert_eq!(
            config["config"],
            json!({"Entrypoint": ["/old"], "Cmd": ["python", "/cleanup.py"], "Env": ["PATH=/bin"]})
        );

        // Setting what's already there changes nothing, and loses nothing.
        let (tar, manifest) = built_image(("linux", "amd64"));
        let folder = tempfile::tempdir().unwrap();
        unpack(tar.as_slice(), folder.path()).unwrap();
        let old = ["old".to_string()];
        let image = finish(folder, manifest.clone(), None, Some(&old), None).unwrap();
        for (blob, _) in &image.blobs {
            assert!(image.blob_path(blob).exists());
        }
    }

    /// An entrypoint, a command, and the config that should come of them.
    type Case<'a> = (Option<&'a [&'a str]>, Option<&'a [&'a str]>, Value);

    #[test]
    fn test_set_command_works_like_docker() {
        let cases: [Case; 6] = [
            (None, None, json!({"Entrypoint": ["/old"], "Cmd": ["old"]})),
            (
                None,
                Some(&["new"]),
                json!({"Entrypoint": ["/old"], "Cmd": ["new"]}),
            ),
            // A new entrypoint doesn't want the old entrypoint's command.
            (
                Some(&["/new"]),
                None,
                json!({"Entrypoint": ["/new"], "Cmd": null}),
            ),
            (
                Some(&["/new"]),
                Some(&["new"]),
                json!({"Entrypoint": ["/new"], "Cmd": ["new"]}),
            ),
            // Empty is the same as not set, except that an empty entrypoint still replaces the old one.
            (
                None,
                Some(&[]),
                json!({"Entrypoint": ["/old"], "Cmd": ["old"]}),
            ),
            (Some(&[]), None, json!({"Entrypoint": [], "Cmd": ["old"]})),
        ];

        let strings = |args: &[&str]| args.iter().map(|arg| arg.to_string()).collect::<Vec<_>>();

        for (entrypoint, command, expected) in cases {
            let mut config = json!({"config": {"Entrypoint": ["/old"], "Cmd": ["old"]}});
            let entrypoint = entrypoint.map(strings);
            let command = command.map(strings);
            set_command(&mut config, entrypoint.as_deref(), command.as_deref());
            assert_eq!(config["config"], expected, "{entrypoint:?} {command:?}");
        }

        // Images from `scratch` can have no config at all.
        let mut config = json!({"os": "linux"});
        set_command(&mut config, None, Some(&strings(&["/app"])));
        assert_eq!(config["config"], json!({"Cmd": ["/app"]}));
    }

    #[test]
    fn test_blobs_that_do_not_match_are_refused() {
        let (tar, _) = built_image(("linux", "amd64"));
        let mut tar = tar;
        let at = tar
            .windows(9)
            .position(|bytes| bytes == b"pretend t")
            .unwrap();
        tar[at] = b'P';

        let folder = tempfile::tempdir().unwrap();
        let error = unpack(tar.as_slice(), folder.path())
            .unwrap_err()
            .to_string();
        assert!(error.contains("doesn't match its digest"), "{error}");
        assert_eq!(files_in(folder.path()), 0);
    }

    #[test]
    fn test_other_platforms_are_refused() {
        let (tar, manifest) = built_image(("linux", "arm64"));
        let folder = tempfile::tempdir().unwrap();
        unpack(tar.as_slice(), folder.path()).unwrap();

        let error = finish(folder, manifest, None, None, None)
            .err()
            .unwrap()
            .to_string();
        assert!(error.contains("linux/arm64"), "{error}");
    }

    #[test]
    fn test_only_one_image_is_accepted() {
        let index = |count| {
            let manifests: Vec<Value> = (0..count)
                .map(|_| json!({"digest": digest(b"x").to_string()}))
                .collect();
            serde_json::to_vec(&json!({ "manifests": manifests })).unwrap()
        };

        for count in [0, 2] {
            let tar = tar_of(&[("index.json".into(), index(count))]);
            let folder = tempfile::tempdir().unwrap();
            assert!(unpack(tar.as_slice(), folder.path()).is_err());
        }

        // An index that names a manifest it didn't send.
        let tar = tar_of(&[("index.json".into(), index(1))]);
        let folder = tempfile::tempdir().unwrap();
        assert!(unpack(tar.as_slice(), folder.path()).is_err());
    }
}
