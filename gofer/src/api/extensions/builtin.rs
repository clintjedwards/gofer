//! Stores the images of Gofer's own extensions, which are built into it, so their manifests can name them as
//! `gofer://<name>:<gofer version>` and starting them never pulls anything.
//!
//! Each image is the tar `docker save` made of it, which is an OCI image layout: `index.json` names the image's manifest,
//! and every blob is under `blobs/sha256/`. Storing one is the same as an admin pushing it, so it's checked the same way.

use crate::{
    api::{ApiState, epoch_milli},
    images::{Digest, Owner},
    storage::{self, StorageError},
};
use anyhow::{Context, Result, bail};
use bytes::Bytes;
use serde::Deserialize;
use std::collections::HashMap;
use tracing::{error, info};

include!(concat!(env!("OUT_DIR"), "/extension_images.rs"));

#[derive(Deserialize)]
struct Index {
    manifests: Vec<IndexEntry>,
}

#[derive(Deserialize)]
struct IndexEntry {
    digest: String,
}

/// Stores every built-in image this Gofer doesn't have yet, and drops the ones an earlier version stored. Run before the
/// extensions start. An image that can't be stored is logged, and the extension using it reports that it's missing.
pub async fn store(api_state: &ApiState) {
    let version = env!("CARGO_PKG_VERSION");

    for (name, tar) in BUILTIN_EXTENSION_IMAGES {
        if let Err(e) = store_one(api_state, name, version, tar).await {
            error!(
                name,
                error = format!("{e:#}"),
                "Could not store one of Gofer's own extension images"
            );
        }
    }

    let dropped = async {
        let mut conn = api_state.storage.write_conn().await?;
        storage::extension_images::delete_other_builtins(&mut conn, version).await?;
        anyhow::Ok(())
    }
    .await;

    if let Err(e) = dropped {
        error!(
            error = format!("{e:#}"),
            "Could not drop the extension images an earlier version of Gofer stored"
        );
    }
}

async fn store_one(
    api_state: &ApiState,
    name: &str,
    version: &str,
    tar: &'static [u8],
) -> Result<()> {
    let files = read_tar(tar)?;
    let index: Index =
        serde_json::from_slice(files.get("index.json").context("It has no index.json")?)
            .context("Its index.json is invalid")?;
    let [entry] = index.manifests.as_slice() else {
        bail!(
            "It should have exactly one image, but its index.json lists {}",
            index.manifests.len()
        );
    };
    let manifest: Digest = entry.digest.parse()?;

    {
        let mut conn = api_state.storage.read_conn().await?;
        match storage::extension_images::get(&mut conn, name, version).await {
            Ok(stored) if stored.digest == manifest.to_string() => return Ok(()),
            Ok(_) | Err(StorageError::NotFound) => {}
            Err(e) => return Err(e.into()),
        }
    }

    let mut conn = api_state.storage.write_conn().await?;
    let now = epoch_milli().to_string();

    for (path, content) in &files {
        let Some(hex) = path.strip_prefix("blobs/sha256/") else {
            continue;
        };
        let digest: Digest = format!("sha256:{hex}").parse()?;

        let chunk: std::io::Result<Bytes> = Ok(Bytes::from_static(content));
        let blob = (api_state
            .images
            .put(&digest, futures::stream::iter([chunk]))
            .await)
            .with_context(|| format!("Could not store blob '{digest}'"))?;

        storage::extension_images::upsert_blob(
            &mut conn,
            &storage::extension_images::ExtensionImageBlob {
                digest: digest.to_string(),
                diff_id: blob.diff_id.to_string(),
                size: blob.size.try_into()?,
                last_seen: now.clone(),
            },
        )
        .await?;
    }

    api_state
        .images
        .verify(&mut conn, Owner::Extensions, &manifest)
        .await?;

    storage::extension_images::upsert(
        &mut conn,
        &storage::extension_images::ExtensionImage {
            name: name.into(),
            tag: version.into(),
            digest: manifest.to_string(),
            builtin: true,
            created: now,
        },
    )
    .await?;

    info!(name, version, digest = %manifest, "Stored one of Gofer's own extension images");
    Ok(())
}

/// Every file in a tar, by path, as a slice of the tar itself so nothing is copied.
fn read_tar(tar: &'static [u8]) -> Result<HashMap<String, &'static [u8]>> {
    let mut archive = tar::Archive::new(tar);
    let mut files = HashMap::new();

    for entry in archive.entries().context("It isn't a valid tar")? {
        let entry = entry.context("It isn't a valid tar")?;
        if !entry.header().entry_type().is_file() {
            continue;
        }

        let path = entry.path()?.to_string_lossy().to_string();
        let start: usize = entry.raw_file_position().try_into()?;
        let end = start + usize::try_from(entry.size())?;
        files.insert(path, tar.get(start..end).context("It's cut short")?);
    }

    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_tar() {
        let mut builder = tar::Builder::new(vec![]);
        for (path, content) in [("index.json", "{}"), ("blobs/sha256/abc", "some layer")] {
            let mut header = tar::Header::new_ustar();
            header.set_size(content.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            builder
                .append_data(&mut header, path, content.as_bytes())
                .unwrap();
        }
        let tar: &'static [u8] = Box::leak(builder.into_inner().unwrap().into_boxed_slice());

        let files = read_tar(tar).unwrap();
        assert_eq!(files["index.json"], b"{}");
        assert_eq!(files["blobs/sha256/abc"], b"some layer");
    }
}
