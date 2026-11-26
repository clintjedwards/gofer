//! Finds the files that go into a task's image, and hashes them so we can tell when they've changed.
//!
//! BuildKit can do both of these itself, but only once a build is running. `gofer up` needs to know before that, so a
//! task whose files and config didn't change is never built at all. The image's digest can't tell us either, since
//! building the same files twice gives a different one: files that run steps make get new times.
//!
//! The same walk decides what gets sent to the build, so the hash and the image can't disagree about what's in it.

use super::dockerignore::{DockerIgnore, Verdict};
use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};
use std::{
    fs::File,
    io::{ErrorKind, Read},
    os::unix::{ffi::OsStrExt, fs::PermissionsExt},
    path::{Path, PathBuf},
};

/// Programs mark a folder that only holds a cache by putting a file in it with this name, which starts with this
/// signature. Cargo does it to `target/`, for example.
///
/// <https://bford.info/cachedir/>
const CACHE_TAG_NAME: &str = "CACHEDIR.TAG";
const CACHE_TAG_SIGNATURE: &[u8] = b"Signature: 8a477f597d28d172789f06886806bc55";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kind {
    Folder,
    File { executable: bool },
    Link { target: PathBuf },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// Where it is inside of the copied folder. A file that's copied by itself just has its own name here.
    pub name: PathBuf,

    pub kind: Kind,
}

/// A file or folder that a task copies into its image, and everything in it that gets copied.
#[derive(Debug, Clone)]
pub struct Source {
    path: PathBuf,

    pub is_folder: bool,

    /// Folders come before the things inside them, and the order is the same on every machine.
    pub entries: Vec<Entry>,
}

impl Source {
    /// What a `copy` step puts in the image. The folder's `.dockerignore` decides what to leave out of it, and folders
    /// marked as caches are left out too. Otherwise copying a Rust project would also copy its `target/`, which can be
    /// many gigabytes and changes with every local build.
    pub fn copy(path: &Path) -> Result<Self> {
        Self::read(path, None, true)
    }

    /// A Dockerfile's build context. The `.dockerignore` decides what's left out, just like with `docker build`, which
    /// uses the one at the top of the context unless the Dockerfile has its own.
    pub fn context(path: &Path, ignore_file: Option<&Path>) -> Result<Self> {
        Self::read(path, ignore_file, false)
    }

    fn read(path: &Path, ignore_file: Option<&Path>, skip_caches: bool) -> Result<Self> {
        // Links inside of a folder are copied as links, since following them could lead anywhere. This one was named by
        // the user though, so what they want is whatever it leads to.
        let real_path = path
            .canonicalize()
            .with_context(|| format!("Could not find '{}'", path.display()))?;

        let metadata = real_path.metadata()?;

        if metadata.is_dir() {
            let default_ignore_file = real_path.join(".dockerignore");
            let ignore = read_ignore_file(ignore_file.unwrap_or(&default_ignore_file))?;

            let mut source = Self {
                path: real_path,
                is_folder: true,
                entries: vec![],
            };
            source.add_folder(Path::new(""), &ignore, &ignore.root(), skip_caches)?;
            return Ok(source);
        }

        if !metadata.is_file() {
            bail!("'{}' isn't a file or a folder", path.display());
        }

        let name = path
            .file_name()
            .with_context(|| format!("'{}' doesn't have a name", path.display()))?;

        Ok(Self {
            path: real_path,
            is_folder: false,
            entries: vec![Entry {
                name: name.into(),
                kind: Kind::File {
                    executable: is_executable(&metadata),
                },
            }],
        })
    }

    fn add_folder(
        &mut self,
        folder: &Path,
        ignore: &DockerIgnore,
        folder_verdict: &Verdict,
        skip_caches: bool,
    ) -> Result<()> {
        let full_path = self.path.join(folder);
        let could_not_read = || format!("Could not read '{}'", full_path.display());

        let mut children = std::fs::read_dir(&full_path)
            .with_context(could_not_read)?
            .collect::<std::io::Result<Vec<_>>>()
            .with_context(could_not_read)?;
        children.sort_by_key(|child| child.file_name());

        for child in children {
            let name = folder.join(child.file_name());
            let metadata = child.metadata().with_context(could_not_read)?;

            // Rules are text, so a name that isn't valid text has to be matched as close as we can get to it.
            let name_text = name.to_string_lossy();
            let is_cache = skip_caches && metadata.is_dir() && is_cache(&child.path());
            let verdict = ignore.check(&name_text, is_cache, folder_verdict);

            if metadata.is_dir() {
                self.entries.push(Entry {
                    name: name.clone(),
                    kind: Kind::Folder,
                });
                let entries_before = self.entries.len();

                if !verdict.ignored || ignore.reaches_into(&name_text) {
                    self.add_folder(&name, ignore, &verdict, skip_caches)?;
                }

                // An ignored folder is only there to hold something that wasn't ignored.
                if verdict.ignored && self.entries.len() == entries_before {
                    self.entries.pop();
                }

                continue;
            }

            if verdict.ignored {
                continue;
            }

            let kind = if metadata.is_file() {
                Kind::File {
                    executable: is_executable(&metadata),
                }
            } else if metadata.is_symlink() {
                Kind::Link {
                    target: std::fs::read_link(child.path()).with_context(could_not_read)?,
                }
            } else {
                // Sockets, pipes and devices don't have contents to copy.
                continue;
            };

            self.entries.push(Entry { name, kind });
        }

        Ok(())
    }

    /// Where an entry is on this machine.
    pub fn path_of(&self, entry: &Entry) -> PathBuf {
        if self.is_folder {
            self.path.join(&entry.name)
        } else {
            self.path.clone()
        }
    }

    /// Whether there's a file with this name anywhere in it.
    pub fn has_file(&self, file_name: &str) -> bool {
        self.entries.iter().any(|entry| {
            matches!(entry.kind, Kind::File { .. })
                && entry.name.file_name().is_some_and(|name| name == file_name)
        })
    }
}

/// One hash over all the files that go into an image. File times and owners aren't part of it because they differ
/// between machines that have the same files. Neither are the steps the files are used in, since those are part of the
/// config.
pub fn hash(sources: &[Source]) -> Result<String> {
    let mut hasher = Sha256::new();

    // Everything has its length in front, or else a different split of the same bytes would have the same hash, like
    // a file named `ab` that holds `c` and a file named `a` that holds `bc`.
    let mut add = |bytes: &[u8]| {
        hasher.update((bytes.len() as u64).to_le_bytes());
        hasher.update(bytes);
    };

    for source in sources {
        add(if source.is_folder { b"folder" } else { b"file" });
        add(&(source.entries.len() as u64).to_le_bytes());

        for entry in &source.entries {
            add(entry.name.as_os_str().as_bytes());

            match &entry.kind {
                Kind::Folder => add(b"folder"),
                Kind::File { executable } => {
                    add(if *executable { b"executable" } else { b"file" });

                    let path = source.path_of(entry);
                    add(&hash_file(&path)
                        .with_context(|| format!("Could not read '{}'", path.display()))?);
                }
                Kind::Link { target } => {
                    add(b"link");
                    add(target.as_os_str().as_bytes());
                }
            }
        }
    }

    Ok(hex::encode(hasher.finalize()))
}

fn hash_file(path: &Path) -> std::io::Result<[u8; 32]> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0; 64 * 1024];

    loop {
        let read = file.read(&mut buffer)?;

        if read == 0 {
            return Ok(hasher.finalize().into());
        }

        hasher.update(&buffer[..read]);
    }
}

fn read_ignore_file(path: &Path) -> Result<DockerIgnore> {
    let content = match std::fs::read(path) {
        Ok(content) => content,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(DockerIgnore::default()),
        Err(e) => return Err(e).context(format!("Could not read '{}'", path.display())),
    };

    DockerIgnore::parse(&String::from_utf8_lossy(&content))
        .with_context(|| format!("Could not use '{}'", path.display()))
}

/// Git only tracks whether a file is executable, so that's the only permission that's the same everywhere.
fn is_executable(metadata: &std::fs::Metadata) -> bool {
    metadata.permissions().mode() & 0o111 != 0
}

fn is_cache(folder: &Path) -> bool {
    let tag = folder.join(CACHE_TAG_NAME);

    // The spec says links don't count.
    if !tag.symlink_metadata().is_ok_and(|tag| tag.is_file()) {
        return false;
    }

    let mut start = [0; CACHE_TAG_SIGNATURE.len()];

    File::open(tag)
        .and_then(|mut tag| tag.read_exact(&mut start))
        .is_ok_and(|()| start == CACHE_TAG_SIGNATURE)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use std::{
        ffi::OsStr,
        fs,
        os::unix::{fs::symlink, net::UnixListener},
        time::{Duration, SystemTime},
    };
    use tempfile::TempDir;

    fn write(dir: &TempDir, name: &str, content: &str) {
        let path = dir.path().join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    /// A small project with one of everything.
    fn project() -> TempDir {
        let dir = tempfile::tempdir().unwrap();
        write(&dir, "Cargo.toml", "[package]");
        write(&dir, "src/main.rs", "fn main() {}");
        write(&dir, "run.sh", "echo hi");
        fs::set_permissions(dir.path().join("run.sh"), PermissionsExt::from_mode(0o755)).unwrap();
        symlink("src/main.rs", dir.path().join("link")).unwrap();
        fs::create_dir(dir.path().join("empty")).unwrap();
        dir
    }

    fn read(dir: &TempDir) -> Source {
        Source::copy(dir.path()).unwrap()
    }

    fn names(source: &Source) -> Vec<&str> {
        source
            .entries
            .iter()
            .map(|entry| entry.name.to_str().unwrap())
            .collect()
    }

    fn hash_of(dir: &TempDir) -> String {
        hash(&[read(dir)]).unwrap()
    }

    #[test]
    fn test_finds_everything_in_a_folder() {
        let dir = project();
        let file = |executable| Kind::File { executable };
        let entry = |name: &str, kind| Entry {
            name: name.into(),
            kind,
        };

        assert_eq!(
            read(&dir).entries,
            vec![
                entry("Cargo.toml", file(false)),
                entry("empty", Kind::Folder),
                entry(
                    "link",
                    Kind::Link {
                        target: "src/main.rs".into()
                    }
                ),
                entry("run.sh", file(true)),
                entry("src", Kind::Folder),
                entry("src/main.rs", file(false)),
            ]
        );
    }

    #[test]
    fn test_links_are_only_followed_when_named_by_the_user() {
        let dir = project();
        symlink("/", dir.path().join("everything")).unwrap();
        symlink(".", dir.path().join("loop")).unwrap();
        assert_eq!(read(&dir).entries.len(), 8);

        let source = Source::copy(&dir.path().join("link")).unwrap();
        assert_eq!(names(&source), vec!["link"]);
        assert_eq!(source.entries[0].kind, Kind::File { executable: false });
        assert_eq!(
            fs::read_to_string(source.path_of(&source.entries[0])).unwrap(),
            "fn main() {}"
        );

        let source = Source::copy(&dir.path().join("loop")).unwrap();
        assert_eq!(source.entries, read(&dir).entries);
    }

    #[test]
    fn test_things_without_contents_are_skipped() {
        let dir = project();
        let before = names(&read(&dir)).len();
        let _socket = UnixListener::bind(dir.path().join("socket")).unwrap();

        assert_eq!(names(&read(&dir)).len(), before);
        assert!(Source::copy(&dir.path().join("socket")).is_err());
        assert!(Source::copy(&dir.path().join("missing")).is_err());
    }

    #[test]
    fn test_dockerignore_is_honored() {
        let dir = project();
        write(
            &dir,
            ".dockerignore",
            "src\n!src/lib.rs\nempty\n.dockerignore",
        );
        write(&dir, "src/lib.rs", "");
        assert_eq!(
            names(&read(&dir)),
            vec!["Cargo.toml", "link", "run.sh", "src", "src/lib.rs"]
        );

        // Another file can be used instead, and then the folder's own is like any other file.
        write(&dir, "other", "*\n!.dockerignore");
        let source = Source::context(dir.path(), Some(&dir.path().join("other"))).unwrap();
        assert_eq!(names(&source), vec![".dockerignore"]);

        write(&dir, ".dockerignore", "[");
        let error = format!("{:#}", Source::copy(dir.path()).unwrap_err());
        assert!(error.contains(".dockerignore") && error.contains("line 1"));
    }

    #[test]
    fn test_caches_are_skipped() {
        let dir = project();
        let before = hash_of(&dir);
        let tag = std::str::from_utf8(CACHE_TAG_SIGNATURE).unwrap();

        write(&dir, "target/CACHEDIR.TAG", &format!("{tag}\n# By cargo."));
        write(&dir, "target/debug/app", "");
        write(&dir, "src/nested/CACHEDIR.TAG", tag);
        assert_eq!(hash_of(&dir), before);

        // Copying the cache itself is asking for what's in it.
        let source = Source::copy(&dir.path().join("target")).unwrap();
        assert_eq!(names(&source), vec!["CACHEDIR.TAG", "debug", "debug/app"]);

        write(&dir, ".dockerignore", "!target/debug/app");
        let source = read(&dir);
        let in_target: Vec<&str> = names(&source)
            .into_iter()
            .filter(|name| name.starts_with("target"))
            .collect();
        assert_eq!(
            in_target,
            vec!["target", "target/debug", "target/debug/app"]
        );
    }

    #[test]
    fn test_only_valid_cache_tags_count() {
        let dir = project();
        write(&dir, "short/CACHEDIR.TAG", "Signature: 8a477f597d28d172");
        write(&dir, "wrong/CACHEDIR.TAG", &"x".repeat(43));
        write(
            &dir,
            "tag",
            std::str::from_utf8(CACHE_TAG_SIGNATURE).unwrap(),
        );
        fs::create_dir(dir.path().join("linked")).unwrap();
        symlink("../tag", dir.path().join("linked/CACHEDIR.TAG")).unwrap();
        let source = read(&dir);

        for folder in ["short", "wrong", "linked"] {
            let tag = format!("{folder}/CACHEDIR.TAG");
            assert!(names(&source).contains(&tag.as_str()), "{folder}");
        }
    }

    #[test]
    fn test_hash_is_the_same_for_the_same_files() {
        let dir = project();
        let before = hash_of(&dir);

        let file = File::open(dir.path().join("Cargo.toml")).unwrap();
        file.set_modified(SystemTime::now() - Duration::from_secs(1000))
            .unwrap();
        // Checkouts differ in this depending on the user's umask.
        file.set_permissions(PermissionsExt::from_mode(0o600))
            .unwrap();
        assert_eq!(hash_of(&dir), before);

        // Nor does it matter where the folder is.
        assert_eq!(hash_of(&project()), before);
    }

    #[test]
    fn test_hash_changes_with_the_files() {
        let mut hashes = vec![hash_of(&project())];

        let changes: [fn(&Path); 8] = [
            |dir| fs::write(dir.join("Cargo.toml"), "[workspace]").unwrap(),
            |dir| fs::rename(dir.join("Cargo.toml"), dir.join("Cargo.tom")).unwrap(),
            |dir| fs::remove_file(dir.join("Cargo.toml")).unwrap(),
            |dir| fs::write(dir.join("new"), "").unwrap(),
            |dir| fs::remove_dir(dir.join("empty")).unwrap(),
            |dir| {
                fs::set_permissions(dir.join("Cargo.toml"), PermissionsExt::from_mode(0o755))
                    .unwrap()
            },
            |dir| {
                fs::remove_file(dir.join("link")).unwrap();
                symlink("src", dir.join("link")).unwrap();
            },
            |dir| {
                fs::remove_file(dir.join("link")).unwrap();
                fs::write(dir.join("link"), "src/main.rs").unwrap();
            },
        ];

        for change in changes {
            let dir = project();
            change(dir.path());
            hashes.push(hash_of(&dir));
        }

        // The same file, copied on its own.
        let dir = tempfile::tempdir().unwrap();
        write(&dir, "a", "bc");
        let file = Source::copy(&dir.path().join("a")).unwrap();
        hashes.push(hash(&[read(&dir)]).unwrap());
        hashes.push(hash(std::slice::from_ref(&file)).unwrap());
        hashes.push(hash(&[file, read(&dir)]).unwrap());

        // The same bytes, split differently between the name and the contents.
        let dir = tempfile::tempdir().unwrap();
        write(&dir, "ab", "c");
        hashes.push(hash_of(&dir));

        let total = hashes.len();
        hashes.sort();
        hashes.dedup();
        assert_eq!(hashes.len(), total);
    }

    #[test]
    fn test_names_that_are_not_text_are_kept() {
        let dir = tempfile::tempdir().unwrap();
        let name = OsStr::from_bytes(b"caf\xe9");
        fs::write(dir.path().join(name), "").unwrap();

        assert_eq!(read(&dir).entries[0].name, name);
        hash_of(&dir);
    }

    #[test]
    fn test_unreadable_files_are_named() {
        let dir = project();
        let source = read(&dir);
        fs::remove_file(dir.path().join("src/main.rs")).unwrap();

        let error = format!("{:#}", hash(&[source]).unwrap_err());
        assert!(error.contains("src/main.rs"), "{error}");
    }

    #[test]
    fn test_has_file() {
        let dir = project();
        write(&dir, "deep/down/go.mod", "");
        let source = read(&dir);

        assert!(source.has_file("Cargo.toml"));
        assert!(source.has_file("go.mod"));
        assert!(!source.has_file("down"));
        assert!(!source.has_file("argo.toml"));
    }
}
