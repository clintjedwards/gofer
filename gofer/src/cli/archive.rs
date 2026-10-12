//! Moving directories in and out of the object store as gzipped tarballs.
//!
//! Handing a directory from one task to the next is common enough that it's worth doing the tar work for users instead
//! of making every task image carry its own tar commands.

use anyhow::{Context, Result};
use bytes::Bytes;
use flate2::{Compression, read::GzDecoder, write::GzEncoder};
use futures::{Stream, StreamExt};
use std::io::Write;
use std::path::PathBuf;
use tokio_util::io::{ReaderStream, StreamReader, SyncIoBridge};

/// Streams `dir` as a gzipped tarball so it can be uploaded without writing the archive to disk first.
///
/// Paths in the archive are relative to `dir`, so extracting it recreates the directory's contents rather than the
/// directory itself. Symlinks are stored as symlinks, since following them could pull in files from outside `dir`.
pub fn tar_directory(dir: PathBuf) -> reqwest::Body {
    let (reader, writer) = tokio::io::duplex(64 * 1024);

    let archiver = tokio::task::spawn_blocking(move || -> std::io::Result<()> {
        let encoder = GzEncoder::new(SyncIoBridge::new(writer), Compression::default());
        let mut builder = tar::Builder::new(encoder);
        builder.follow_symlinks(false);
        builder.append_dir_all(".", &dir)?;
        builder.into_inner()?.finish()?.flush()
    });

    // If archiving fails partway through, the reader just sees an early EOF and we'd upload a truncated object that
    // looks fine. Ending the stream with the archiver's error instead aborts the request before the server records
    // the object.
    let archiver_result = futures::stream::once(async move {
        let result: Option<std::io::Result<Bytes>> = match archiver.await {
            Ok(Ok(())) => None,
            Ok(Err(e)) => Some(Err(e)),
            Err(e) => Some(Err(std::io::Error::other(e))),
        };
        result
    })
    .filter_map(|result| async move { result });

    reqwest::Body::wrap_stream(ReaderStream::new(reader).chain(archiver_result))
}

/// Unpacks a gzipped tarball from `stream` into `dest`, creating `dest` if it doesn't exist.
///
/// Entries that would land outside of `dest` (through `..` or absolute paths) are skipped by the tar crate.
pub async fn untar_into<S>(stream: S, dest: PathBuf) -> Result<()>
where
    S: Stream<Item = reqwest::Result<Bytes>> + Send + 'static,
{
    let reader = StreamReader::new(Box::pin(
        stream.map(|chunk| chunk.map_err(std::io::Error::other)),
    ));

    tokio::task::spawn_blocking(move || -> Result<()> {
        std::fs::create_dir_all(&dest)
            .with_context(|| format!("Could not create directory '{}'", dest.display()))?;

        let mut archive = tar::Archive::new(GzDecoder::new(SyncIoBridge::new(reader)));
        archive.unpack(&dest).context(
            "Could not extract object; --extract only works on objects uploaded from a directory",
        )?;

        Ok(())
    })
    .await
    .context("Could not start extraction thread")?
}

#[cfg(test)]
mod tests {
    use super::*;
    use http_body_util::BodyDataStream;
    use std::os::unix::fs::PermissionsExt;

    #[tokio::test]
    async fn directory_round_trip() {
        let src = tempfile::tempdir().unwrap();
        std::fs::create_dir(src.path().join("bin")).unwrap();
        std::fs::write(src.path().join("bin/tool"), "#!/bin/sh\n").unwrap();
        std::fs::set_permissions(
            src.path().join("bin/tool"),
            std::fs::Permissions::from_mode(0o755),
        )
        .unwrap();
        std::fs::write(src.path().join("readme"), "hello").unwrap();
        std::os::unix::fs::symlink("bin/tool", src.path().join("link")).unwrap();

        let dest = tempfile::tempdir().unwrap();
        let out = dest.path().join("out");

        let body = tar_directory(src.path().to_path_buf());
        untar_into(BodyDataStream::new(body), out.clone())
            .await
            .unwrap();

        assert_eq!(
            std::fs::read_to_string(out.join("readme")).unwrap(),
            "hello"
        );

        // Build outputs often include executables, so the mode has to survive the trip.
        let mode = std::fs::metadata(out.join("bin/tool"))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o755);

        assert_eq!(
            std::fs::read_link(out.join("link")).unwrap(),
            PathBuf::from("bin/tool")
        );
    }

    #[tokio::test]
    async fn archive_failure_errors_the_stream() {
        let dir = tempfile::tempdir().unwrap();
        let mut stream = BodyDataStream::new(tar_directory(dir.path().join("missing")));

        let mut saw_error = false;
        while let Some(chunk) = stream.next().await {
            if chunk.is_err() {
                saw_error = true;
                break;
            }
        }

        assert!(
            saw_error,
            "a failed archive should error the upload, not end it cleanly"
        );
    }
}
