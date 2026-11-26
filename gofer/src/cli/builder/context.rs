//! Packs a build's files into the tar that gets copied into the builder.
//!
//! BuildKit normally gets files from the machine `buildctl` runs on, through a protocol of its own. Ours runs inside the
//! builder's container though, where it would only see the container's files. So we copy them in first, every time.
//! Sending only what changed would need that protocol, and 1GB takes about 8 seconds as it is.

use super::source::{Kind, Source};
use bytes::{Bytes, BytesMut};
use std::{
    fs::File,
    io::{self, Read, Write},
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};
use tokio::sync::mpsc::Sender;

/// What goes in a build's folder.
pub struct Contents {
    /// Files we made, like the Dockerfile, by where they go and what's in them.
    pub files: Vec<(String, Vec<u8>)>,

    /// The user's files, by the folder they go in.
    pub sources: Vec<(String, Source)>,
}

/// Writes the contents into a tar, all inside of `folder`.
///
/// Everything is owned by root, and only the executable bit is kept from a file's permissions, since that's all the
/// source hash covers. Otherwise two machines with the same hash could build different images, like when one has a
/// file only its owner can read and the image runs as another user.
pub fn write(out: impl Write, folder: &str, contents: &Contents) -> io::Result<()> {
    let mut tar = tar::Builder::new(out);
    let folder = Path::new(folder);

    let mut add_folder = |tar: &mut tar::Builder<_>, path: &Path, modified: u64| {
        let mut header = header(tar::EntryType::Directory, 0o755, modified);
        tar.append_data(&mut header, path, io::empty())
    };

    add_folder(&mut tar, folder, 0)?;

    for (path, content) in &contents.files {
        let path = folder.join(path);
        add_parents(&mut tar, folder, &path, &mut add_folder)?;

        let mut header = header(tar::EntryType::Regular, 0o644, 0);
        header.set_size(content.len() as u64);
        tar.append_data(&mut header, path, content.as_slice())?;
    }

    for (prefix, source) in &contents.sources {
        let prefix = folder.join(prefix);
        add_parents(&mut tar, folder, &prefix.join("x"), &mut add_folder)?;

        for entry in &source.entries {
            let path = prefix.join(&entry.name);
            let real_path = source.path_of(entry);
            let metadata = real_path
                .symlink_metadata()
                .map_err(|e| named(e, &real_path))?;

            // The real times, like `docker build` sends. Tools like make compare them, so with one fixed time every
            // file would look older than the last build. BuildKit leaves times out of its cache keys, so they don't
            // stop it from reusing a layer.
            let modified = metadata
                .modified()
                .ok()
                .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                .map_or(0, |time| time.as_secs());

            match &entry.kind {
                Kind::Folder => add_folder(&mut tar, &path, modified)?,

                Kind::File { executable } => {
                    let mode = if *executable { 0o755 } else { 0o644 };
                    let mut header = header(tar::EntryType::Regular, mode, modified);
                    let file = File::open(&real_path).map_err(|e| named(e, &real_path))?;
                    let size = file.metadata()?.len();
                    header.set_size(size);

                    let content = Exactly {
                        file: file.take(size),
                        left: size,
                        path: &real_path,
                    };
                    tar.append_data(&mut header, path, content)?;
                }

                Kind::Link { target } => {
                    let mut header = header(tar::EntryType::Symlink, 0o777, modified);
                    tar.append_link(&mut header, path, target)?;
                }
            }
        }
    }

    tar.into_inner()?.flush()
}

fn header(kind: tar::EntryType, mode: u32, modified: u64) -> tar::Header {
    let mut header = tar::Header::new_gnu();
    header.set_entry_type(kind);
    header.set_mode(mode);
    header.set_uid(0);
    header.set_gid(0);
    header.set_mtime(modified);
    header.set_size(0);
    header
}

/// Adds the folders between `folder` and `path`, since a tar only has the folders it lists.
fn add_parents<W: Write>(
    tar: &mut tar::Builder<W>,
    folder: &Path,
    path: &Path,
    add_folder: &mut impl FnMut(&mut tar::Builder<W>, &Path, u64) -> io::Result<()>,
) -> io::Result<()> {
    let parents: Vec<PathBuf> = path
        .ancestors()
        .skip(1)
        .take_while(|parent| *parent != folder)
        .map(Path::to_path_buf)
        .collect();

    // Duplicates don't hurt, they're the same folder twice.
    for parent in parents.iter().rev() {
        add_folder(tar, parent, 0)?;
    }

    Ok(())
}

fn named(e: io::Error, path: &Path) -> io::Error {
    io::Error::new(
        e.kind(),
        format!("could not read '{}'; {e}", path.display()),
    )
}

/// Reads exactly the number of bytes the tar header promised. A file that shrank while we read it would otherwise leave
/// the rest of the tar out of step with its headers.
struct Exactly<'a> {
    file: io::Take<File>,
    left: u64,
    path: &'a Path,
}

impl Read for Exactly<'_> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let read = self.file.read(buffer)?;

        if read == 0 && self.left > 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                format!(
                    "'{}' got shorter while it was being read",
                    self.path.display()
                ),
            ));
        }

        self.left -= read as u64;
        Ok(read)
    }
}

/// Sends what's written to it down a channel in large chunks, so a tar can be written on one thread and uploaded from
/// another.
pub struct ChannelWriter {
    sender: Sender<io::Result<Bytes>>,
    buffer: BytesMut,
}

const CHUNK_BYTES: usize = 256 * 1024;

impl ChannelWriter {
    pub fn new(sender: Sender<io::Result<Bytes>>) -> Self {
        Self {
            sender,
            buffer: BytesMut::with_capacity(CHUNK_BYTES),
        }
    }

    fn send(&mut self) -> io::Result<()> {
        if self.buffer.is_empty() {
            return Ok(());
        }

        let chunk = self.buffer.split().freeze();
        self.sender
            .blocking_send(Ok(chunk))
            .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "the upload stopped"))
    }
}

impl Write for ChannelWriter {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        self.buffer.extend_from_slice(data);

        if self.buffer.len() >= CHUNK_BYTES {
            self.send()?;
        }

        Ok(data.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.send()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use std::{fs, os::unix::ffi::OsStrExt, os::unix::fs::PermissionsExt};
    use tar::EntryType::{Directory, Regular, Symlink};

    /// Each entry's name, type and permissions.
    fn names(tar: &[u8]) -> Vec<(String, tar::EntryType, u32)> {
        let mut archive = tar::Archive::new(tar);
        archive
            .entries()
            .unwrap()
            .map(|entry| {
                let entry = entry.unwrap();
                let header = entry.header();
                let name = String::from_utf8_lossy(&entry.path_bytes()).to_string();
                (name, header.entry_type(), header.mode().unwrap())
            })
            .collect()
    }

    fn tar_of(contents: &Contents) -> Vec<u8> {
        let mut out = vec![];
        write(&mut out, "gofer-builds/1", contents).unwrap();
        out
    }

    #[test]
    fn test_everything_lands_in_the_folder() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("src")).unwrap();
        fs::write(dir.path().join("src/main.rs"), "fn main() {}").unwrap();
        fs::write(dir.path().join("run.sh"), "echo hi").unwrap();
        fs::set_permissions(dir.path().join("run.sh"), PermissionsExt::from_mode(0o700)).unwrap();
        fs::set_permissions(
            dir.path().join("src/main.rs"),
            PermissionsExt::from_mode(0o600),
        )
        .unwrap();
        std::os::unix::fs::symlink("src/main.rs", dir.path().join("link")).unwrap();

        let copy = Source::copy(dir.path()).unwrap();
        let file = Source::copy(&dir.path().join("run.sh")).unwrap();
        let tar = tar_of(&Contents {
            files: vec![
                ("Dockerfile".into(), b"FROM scratch".to_vec()),
                ("context/runs/0/run.sh".into(), b"true".to_vec()),
            ],
            sources: vec![
                ("context/copies/0/".into(), copy),
                ("context/copies/1/".into(), file),
            ],
        });

        let mut names = names(&tar);
        names.dedup();
        assert_eq!(
            names,
            vec![
                ("gofer-builds/1".into(), Directory, 0o755),
                ("gofer-builds/1/Dockerfile".into(), Regular, 0o644),
                ("gofer-builds/1/context".into(), Directory, 0o755),
                ("gofer-builds/1/context/runs".into(), Directory, 0o755),
                ("gofer-builds/1/context/runs/0".into(), Directory, 0o755),
                (
                    "gofer-builds/1/context/runs/0/run.sh".into(),
                    Regular,
                    0o644
                ),
                ("gofer-builds/1/context".into(), Directory, 0o755),
                ("gofer-builds/1/context/copies".into(), Directory, 0o755),
                ("gofer-builds/1/context/copies/0".into(), Directory, 0o755),
                (
                    "gofer-builds/1/context/copies/0/link".into(),
                    Symlink,
                    0o777
                ),
                (
                    "gofer-builds/1/context/copies/0/run.sh".into(),
                    Regular,
                    0o755
                ),
                (
                    "gofer-builds/1/context/copies/0/src".into(),
                    Directory,
                    0o755
                ),
                (
                    "gofer-builds/1/context/copies/0/src/main.rs".into(),
                    Regular,
                    0o644
                ),
                ("gofer-builds/1/context".into(), Directory, 0o755),
                ("gofer-builds/1/context/copies".into(), Directory, 0o755),
                ("gofer-builds/1/context/copies/1".into(), Directory, 0o755),
                (
                    "gofer-builds/1/context/copies/1/run.sh".into(),
                    Regular,
                    0o755
                ),
            ]
        );

        let mut archive = tar::Archive::new(tar.as_slice());
        let mut main = archive
            .entries()
            .unwrap()
            .map(Result::unwrap)
            .find(|entry| entry.path().unwrap().ends_with("src/main.rs"))
            .unwrap();
        assert_eq!(main.header().uid().unwrap(), 0);
        let mut content = String::new();
        main.read_to_string(&mut content).unwrap();
        assert_eq!(content, "fn main() {}");
    }

    #[test]
    fn test_long_names_and_names_that_are_not_text() {
        let dir = tempfile::tempdir().unwrap();
        let long = "a".repeat(200);
        let odd = std::ffi::OsStr::from_bytes(b"caf\xe9");
        fs::create_dir(dir.path().join(&long)).unwrap();
        fs::write(dir.path().join(&long).join(odd), "").unwrap();
        std::os::unix::fs::symlink("b".repeat(300), dir.path().join("link")).unwrap();

        let copy = Source::copy(dir.path()).unwrap();
        let tar = tar_of(&Contents {
            files: vec![],
            sources: vec![("context/".into(), copy)],
        });

        let mut archive = tar::Archive::new(tar.as_slice());
        let entries: Vec<(Vec<u8>, Option<Vec<u8>>)> = archive
            .entries()
            .unwrap()
            .map(|entry| {
                let entry = entry.unwrap();
                let target = entry.link_name_bytes().map(|target| target.to_vec());
                (entry.path_bytes().to_vec(), target)
            })
            .collect();

        let file = format!("gofer-builds/1/context/{long}/caf");
        let mut file = file.into_bytes();
        file.push(0xe9);
        assert!(entries.iter().any(|(name, _)| *name == file));
        assert!(
            entries
                .iter()
                .any(|(_, target)| target.as_deref() == Some("b".repeat(300).as_bytes()))
        );
    }

    #[test]
    fn test_files_that_change_while_read_are_noticed() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("a"), "one").unwrap();
        let copy = Source::copy(dir.path()).unwrap();
        fs::remove_file(dir.path().join("a")).unwrap();

        let mut out = vec![];
        let contents = Contents {
            files: vec![],
            sources: vec![("context/".into(), copy)],
        };
        let error = write(&mut out, "build", &contents).unwrap_err().to_string();
        assert!(error.contains("/a'"), "{error}");
    }
}
