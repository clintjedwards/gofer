//! Each build writes everything BuildKit said to a log file of its own, so what's printed can stay short.

use anyhow::{Context, Result};
use std::{
    fs::{File, OpenOptions},
    io::ErrorKind,
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
    sync::OnceLock,
    time::SystemTime,
};

/// How many logs from earlier runs of the CLI are kept.
const KEPT: usize = 20;

/// When this CLI made its first log. Logs from after then are never removed, even past [`KEPT`]. A `gofer up` that
/// builds more tasks than that would otherwise remove the log of one that failed before it could say where it is.
static FIRST_LOG: OnceLock<SystemTime> = OnceLock::new();

/// Makes a new log for a build of `task`, and removes old ones. Only the user can read it, since builds can print
/// anything.
pub fn create(task: &str) -> Result<(PathBuf, File)> {
    let folder = dirs::cache_dir()
        .context("Could not find a cache folder to keep build logs in")?
        .join("gofer")
        .join("builds");
    std::fs::create_dir_all(&folder)
        .with_context(|| format!("Could not create '{}'", folder.display()))?;

    let since = *FIRST_LOG.get_or_init(SystemTime::now);
    remove_old(&folder, since);

    let name: String = task
        .chars()
        .map(|c| match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' => c,
            _ => '-',
        })
        .collect();
    let time = chrono::Local::now().format("%Y%m%dT%H%M%S");

    // Two builds of the same task can start in the same second.
    for attempt in 1.. {
        let file_name = match attempt {
            1 => format!("{name}-{time}.log"),
            _ => format!("{name}-{time}-{attempt}.log"),
        };
        let path = folder.join(file_name);

        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&path)
        {
            Ok(file) => return Ok((path, file)),
            Err(e) if e.kind() == ErrorKind::AlreadyExists => continue,
            Err(e) => {
                return Err(e).with_context(|| format!("Could not create '{}'", path.display()));
            }
        }
    }

    unreachable!()
}

/// Keeps the newest [`KEPT`] logs from before `since`. This is only tidying, so it doesn't stop a build when it can't.
fn remove_old(folder: &Path, since: SystemTime) {
    let Ok(entries) = std::fs::read_dir(folder) else {
        return;
    };

    let mut old: Vec<(SystemTime, PathBuf)> = entries
        .flatten()
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "log"))
        .filter_map(|entry| Some((entry.metadata().ok()?.modified().ok()?, entry.path())))
        .filter(|(modified, _)| *modified < since)
        .collect();

    // Newest first.
    old.sort_by_key(|(modified, _)| std::cmp::Reverse(*modified));

    for (_, path) in old.into_iter().skip(KEPT) {
        _ = std::fs::remove_file(path);
    }
}

/// Writes a path in the home folder as `~/...`, which is shorter and easier to read.
pub fn shorten(path: &Path) -> String {
    match dirs::home_dir().and_then(|home| path.strip_prefix(home).ok().map(Path::to_path_buf)) {
        Some(rest) => format!("~/{}", rest.display()),
        None => path.display().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_only_old_logs_are_removed() {
        let dir = tempfile::tempdir().unwrap();
        let start = SystemTime::now();

        let touch = |name: &str, age: u64| {
            let file = File::create(dir.path().join(name)).unwrap();
            file.set_modified(start - Duration::from_secs(age)).unwrap();
        };

        for age in 1..=25 {
            touch(&format!("old-{age:02}.log"), 100 + age);
        }
        touch("other.txt", 1000);
        for number in 1..=30 {
            touch(&format!("this-run-{number:02}.log"), 0);
        }

        remove_old(dir.path(), start - Duration::from_secs(10));

        let mut left: Vec<String> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect();
        left.sort();

        assert_eq!(
            left.iter()
                .filter(|name| name.starts_with("this-run"))
                .count(),
            30
        );
        assert!(left.contains(&"other.txt".to_string()));
        assert!(left.contains(&"old-20.log".to_string()));
        assert!(!left.contains(&"old-21.log".to_string()));
    }
}
