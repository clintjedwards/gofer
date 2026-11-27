//! Advice after a build, about things only the user can decide. Each tip comes from something Gofer saw in the build,
//! and says what to change.

use super::{
    Build, Step, Task,
    report::bytes,
    source::{Kind, Source},
};
use lazy_regex::regex;
use serde::Serialize;
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    path::{Path, PathBuf},
};

/// Files this big take seconds to hash and send on every build. 256MB.
const LARGE_SOURCE: u64 = 256 * 1024 * 1024;

/// An image this big, compressed, is slow to upload and to load into Docker. 256MB.
const LARGE_IMAGE: u64 = 256 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Tip {
    /// Stays the same for as long as the reason for the tip does, so it's only shown once.
    #[serde(skip)]
    pub key: String,

    pub message: String,
}

/// What a build showed that a tip could be about.
pub struct Seen<'a> {
    pub task: &'a Task<'a>,

    pub sources: &'a [Source],

    /// From [`super::dockerfile::Rendered`].
    pub uncached_rust: &'a [(String, String)],

    /// The Cargo version run.sh said was too old to cache with.
    pub old_cargo: Option<&'a str>,

    /// How big the image is, compressed.
    pub image_size: u64,
}

/// Every tip that applies to a build, the most useful first.
pub fn find(seen: &Seen) -> Vec<Tip> {
    let id = seen.task.id();
    let rust = seen.sources.iter().any(|s| s.has_file("Cargo.toml"));
    let go = seen.sources.iter().any(|s| s.has_file("go.mod"));
    let mut tips = vec![];
    let mut add = |key: String, message: String| tips.push(Tip { key, message });

    if let Some((command, earlier)) = seen.uncached_rust.first() {
        add(
            format!("uncached-rust:{earlier}:{command}"),
            format!(
                "'{}' in '{id}' builds without the Rust cache, because '{}' runs after the copy and could have changed \
                 the copied code. Move that step before the copy, or into the same run, to get the cache back.",
                first_line(command),
                first_line(earlier)
            ),
        );
    }

    if let (Some(version), Build::Steps(steps)) = (seen.old_cargo, seen.task.build) {
        add(
            format!("old-cargo:{}", steps.base),
            format!(
                "{} has Cargo {version}, which can't keep what it compiles between builds, so '{id}' compiles \
                 everything every time. Cargo 1.91 and newer can, like the one in rust:1.",
                steps.base
            ),
        );
    }

    for (name, source, ignore_file) in named_sources(seen) {
        if let Some((folder, size, total)) = large_folder(source) {
            add(
                format!("large:{name}:{}", folder.display()),
                format!(
                    "{} of the {} sent to build '{id}' is in {}. If the build doesn't need it, adding '{}' to {} \
                     makes every build start sooner.",
                    bytes(size),
                    bytes(total),
                    Path::new(name).join(&folder).display(),
                    folder.display(),
                    ignore_file.display()
                ),
            );
        }
    }

    if let Build::Steps(steps) = seen.task.build {
        let runs = steps
            .steps
            .iter()
            .any(|step| matches!(step, Step::Run { .. }));

        if (rust || go) && runs && steps.keep.is_none() && seen.image_size > LARGE_IMAGE {
            let language = if rust { "Rust" } else { "Go" };
            add(
                format!("large-image:{}", steps.base),
                format!(
                    "The image for '{id}' is {}, mostly the {language} compiler from {}. keep() can move just what \
                     was built onto a small base image, which makes uploads and starts much faster.",
                    bytes(seen.image_size),
                    steps.base
                ),
            );
        }
    }

    if let Some(dockerfile) = read_dockerfile(seen) {
        for (language, mounts) in missing_cache_mounts(&dockerfile) {
            add(
                format!("dockerfile-cache:{language}"),
                format!(
                    "The Dockerfile for '{id}' runs {language} without a cache, so every change starts over. \
                     Adding {mounts} to that RUN keeps what it can reuse."
                ),
            );
        }
    }

    if let Build::Steps(steps) = seen.task.build
        && rust
    {
        let debug = steps.steps.iter().find_map(|step| match step {
            Step::Run { command }
                if command.contains("cargo build")
                    && !regex!(r"--release|--profile|(^|\s)-r(\s|$)").is_match(command) =>
            {
                Some(command)
            }
            _ => None,
        });

        if let Some(command) = debug {
            add(
                format!("debug-build:{command}"),
                format!(
                    "'{id}' runs cargo build without --release, which makes a much slower program. Add --release, \
                     and use what's in target/release instead of target/debug."
                ),
            );
        }
    }

    for (name, source, _) in named_sources(seen) {
        if source.has_file("Cargo.toml") && !source.has_file("Cargo.lock") {
            add(
                format!("no-lock:{name}"),
                format!(
                    "{name} has no Cargo.lock, so each build of '{id}' can pick up newer versions of its \
                     dependencies. Commit the one Cargo makes, so every build uses the same ones."
                ),
            );
        }
    }

    tips
}

/// Picks which tips to show after a `gofer up`, at most `max`. `found` has every tip for each task that was built. A tip
/// isn't shown again while its reason stays the same, so Gofer remembers which were shown in `store`.
pub fn pick(store: &Path, found: &[(&str, Vec<Tip>)], max: usize) -> Vec<Tip> {
    // Losing this only means a tip could show up again, so it's not worth stopping for.
    let mut shown: BTreeMap<String, BTreeSet<String>> = std::fs::read(store)
        .ok()
        .and_then(|content| serde_json::from_slice(&content).ok())
        .unwrap_or_default();

    let mut picked = vec![];

    for (task, tips) in found {
        let before = shown.remove(*task).unwrap_or_default();
        let mut still_shown = BTreeSet::new();

        for tip in tips {
            if before.contains(&tip.key) {
                still_shown.insert(tip.key.clone());
            } else if picked.len() < max {
                picked.push(tip.clone());
                still_shown.insert(tip.key.clone());
            }
        }

        if !still_shown.is_empty() {
            shown.insert(task.to_string(), still_shown);
        }
    }

    if let Some(folder) = store.parent() {
        _ = std::fs::create_dir_all(folder);
    }
    if let Ok(content) = serde_json::to_vec(&shown) {
        _ = std::fs::write(store, content);
    }

    picked
}

/// Where [`pick`] remembers which tips were shown.
pub fn store() -> Option<PathBuf> {
    Some(dirs::cache_dir()?.join("gofer").join("tips.json"))
}

/// The folders the task's files come from, as the user wrote them, with the ignore file that would leave things out.
fn named_sources<'a>(seen: &'a Seen) -> Vec<(&'a str, &'a Source, PathBuf)> {
    match seen.task.build {
        Build::Steps(steps) => steps
            .steps
            .iter()
            .filter_map(|step| match step {
                Step::Copy { from, .. } => Some(from.as_str()),
                Step::Run { .. } => None,
            })
            .zip(seen.sources)
            .filter(|(_, source)| source.is_folder)
            .map(|(from, source)| (from, source, Path::new(from).join(".dockerignore")))
            .collect(),

        Build::Dockerfile { path, context } => {
            let (Some(dockerfile), Some(source)) = (seen.sources.first(), seen.sources.get(1))
            else {
                return vec![];
            };

            // Docker uses the Dockerfile's own ignore file instead, when it has one.
            let has_own = dockerfile.entries.first().is_some_and(|entry| {
                let mut own = dockerfile.path_of(entry).into_os_string();
                own.push(".dockerignore");
                Path::new(&own).exists()
            });
            let ignore_file = match has_own {
                true => PathBuf::from(format!("{path}.dockerignore")),
                false => Path::new(context).join(".dockerignore"),
            };

            vec![(context.as_str(), source, ignore_file)]
        }
    }
}

/// When a source is large and most of it is in one folder, that's often something the build doesn't need, like
/// `node_modules` or a `target` folder. Returns the deepest folder with at least half of it, its size and the total.
fn large_folder(source: &Source) -> Option<(PathBuf, u64, u64)> {
    let mut total = 0;
    let mut folders: HashMap<&Path, u64> = HashMap::new();

    for entry in &source.entries {
        if !matches!(entry.kind, Kind::File { .. }) {
            continue;
        }

        let Ok(metadata) = std::fs::symlink_metadata(source.path_of(entry)) else {
            continue;
        };
        total += metadata.len();

        for folder in entry.name.ancestors().skip(1) {
            if !folder.as_os_str().is_empty() {
                *folders.entry(folder).or_default() += metadata.len();
            }
        }
    }

    if total < LARGE_SOURCE {
        return None;
    }

    // A folder has at least as much as each folder inside it, so these are a chain from the top down.
    let (folder, size) = folders
        .into_iter()
        .filter(|(_, size)| *size * 2 >= total)
        .max_by_key(|(folder, _)| folder.components().count())?;

    Some((folder.to_path_buf(), size, total))
}

fn read_dockerfile(seen: &Seen) -> Option<String> {
    let Build::Dockerfile { .. } = seen.task.build else {
        return None;
    };
    let source = seen.sources.first()?;
    std::fs::read_to_string(source.path_of(source.entries.first()?)).ok()
}

/// The languages a Dockerfile compiles without cache mounts, with the mounts to add. These are the paths in the
/// official images, as root.
fn missing_cache_mounts(dockerfile: &str) -> Vec<(&'static str, &'static str)> {
    // A line ending in `\` goes on to the next.
    let dockerfile = dockerfile.replace("\\\r\n", " ").replace("\\\n", " ");
    let mut missing = vec![];

    for line in dockerfile.lines() {
        let line = line.trim();
        let is_run = line
            .get(..4)
            .is_some_and(|start| start.eq_ignore_ascii_case("RUN "));
        if !is_run || line.contains("--mount=type=cache") {
            continue;
        }

        // Cargo needs its lock shared to share its downloads, which it keeps outside of them. `locked` makes builds
        // that use them take turns instead.
        if regex!(r"cargo (build|install)").is_match(line) {
            missing.push((
                "Cargo",
                "--mount=type=cache,target=/usr/local/cargo/registry,sharing=locked",
            ));
        }

        if regex!(r"go (build|install|test)").is_match(line) {
            missing.push((
                "Go",
                "--mount=type=cache,target=/root/.cache/go-build --mount=type=cache,target=/go/pkg/mod",
            ));
        }
    }

    missing.sort();
    missing.dedup();
    missing
}

fn first_line(command: &str) -> String {
    let mut lines = command.trim().lines();
    let first = lines.next().unwrap_or_default();
    match lines.next() {
        Some(_) => format!("{first} ..."),
        None => first.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::builder::Steps;
    use gofer_sdk::config::Keep;
    use pretty_assertions::assert_eq;
    use std::fs;

    fn task<'a>(build: &'a Build) -> Task<'a> {
        Task {
            name: "default/nightly/report",
            build,
            entrypoint: None,
            command: None,
        }
    }

    fn steps(steps: Vec<Step>, keep: Option<Keep>) -> Build {
        Build::Steps(Steps {
            base: "rust:1".into(),
            steps,
            keep,
        })
    }

    fn copy(from: &str) -> Step {
        Step::Copy {
            from: from.into(),
            to: "/src".into(),
        }
    }

    fn run(command: &str) -> Step {
        Step::Run {
            command: command.into(),
        }
    }

    fn keys(tips: &[Tip]) -> Vec<&str> {
        tips.iter()
            .map(|tip| tip.key.split(':').next().unwrap())
            .collect()
    }

    fn found(dir: &Path, build: &Build, old_cargo: Option<&str>, image_size: u64) -> Vec<Tip> {
        let sources = super::super::sources(build, dir).unwrap();
        let uncached = match build {
            Build::Steps(steps) => {
                super::super::dockerfile::render("t", steps, &sources, false)
                    .unwrap()
                    .uncached_rust
            }
            Build::Dockerfile { .. } => vec![],
        };
        find(&Seen {
            task: &task(build),
            sources: &sources,
            uncached_rust: &uncached,
            old_cargo,
            image_size,
        })
    }

    fn project(files: &[&str]) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        for file in files {
            let path = dir.path().join(file);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, "").unwrap();
        }
        dir
    }

    #[test]
    fn test_a_tidy_rust_task_gets_no_tips() {
        let dir = project(&["app/Cargo.toml", "app/Cargo.lock", "app/src/main.rs"]);
        let build = steps(
            vec![copy("app"), run("cd /src && cargo build --release")],
            Some(Keep {
                paths: vec!["/src/target/release/app".into()],
                base: "debian:13-slim".into(),
            }),
        );

        assert_eq!(found(dir.path(), &build, None, 10_000_000), vec![]);
    }

    #[test]
    fn test_an_untidy_rust_task_gets_them_all() {
        let dir = project(&["app/Cargo.toml", "app/src/main.rs"]);
        let build = steps(
            vec![copy("app"), run("./generate.sh"), run("cargo build")],
            None,
        );

        assert_eq!(
            keys(&found(dir.path(), &build, Some("1.85"), 900_000_000)),
            vec![
                "uncached-rust",
                "old-cargo",
                "large-image",
                "debug-build",
                "no-lock"
            ]
        );
    }

    #[test]
    fn test_dockerfiles_without_cache_mounts() {
        assert_eq!(
            missing_cache_mounts(
                "FROM golang:1\nRUN --mount=type=cache,target=/go/pkg/mod go build ./...\nrun go test \\\n  ./...\n\
                 RUN cargo install --path . && cargo build"
            )
            .into_iter()
            .map(|(language, _)| language)
            .collect::<Vec<_>>(),
            vec!["Cargo", "Go"]
        );
        assert_eq!(
            missing_cache_mounts("RUN --mount=type=cache,target=/root/.cache/go-build go build"),
            vec![]
        );
    }

    #[test]
    fn test_large_folders_are_named() {
        let dir = tempfile::tempdir().unwrap();
        let file = |name: &str, size: u64| {
            let path = dir.path().join(name);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::File::create(path).unwrap().set_len(size).unwrap();
        };
        file("Dockerfile", 10);
        file("web/node_modules/a/big", 100 * 1024 * 1024);
        file("web/node_modules/b/big", 100 * 1024 * 1024);
        file("web/node_modules/c/big", 100 * 1024 * 1024);
        file("web/src/app.js", 1024 * 1024);

        let build = Build::Dockerfile {
            path: "Dockerfile".into(),
            context: ".".into(),
        };
        let tips = found(dir.path(), &build, None, 0);
        assert_eq!(keys(&tips), vec!["large"]);
        assert_eq!(
            tips[0].message,
            "315MB of the 316MB sent to build 'report' is in ./web/node_modules. If the build doesn't need it, \
             adding 'web/node_modules' to ./.dockerignore makes every build start sooner."
        );

        // The Dockerfile's own ignore file is the one that counts, when it has one.
        fs::write(dir.path().join("Dockerfile.dockerignore"), "").unwrap();
        let tips = found(dir.path(), &build, None, 0);
        assert!(
            tips[0]
                .message
                .ends_with("to Dockerfile.dockerignore makes every build start sooner.")
        );

        // When no folder holds most of it, there's nothing to suggest.
        file("big", 400 * 1024 * 1024);
        assert_eq!(found(dir.path(), &build, None, 0), vec![]);
    }

    #[test]
    fn test_tips_are_shown_once_while_their_reason_lasts() {
        let dir = tempfile::tempdir().unwrap();
        let store = dir.path().join("tips.json");
        let tip = |key: &str| Tip {
            key: key.into(),
            message: key.into(),
        };

        let picked = pick(&store, &[("a", vec![tip("1"), tip("2"), tip("3")])], 2);
        assert_eq!(picked, vec![tip("1"), tip("2")]);

        // The third didn't fit before.
        let picked = pick(&store, &[("a", vec![tip("1"), tip("2"), tip("3")])], 2);
        assert_eq!(picked, vec![tip("3")]);

        // Once its reason goes away, a tip can come back.
        pick(&store, &[("a", vec![tip("2"), tip("3")])], 2);
        let picked = pick(&store, &[("a", vec![tip("1"), tip("2")])], 2);
        assert_eq!(picked, vec![tip("1")]);

        // Each task has its own.
        let picked = pick(&store, &[("b", vec![tip("1")])], 2);
        assert_eq!(picked, vec![tip("1")]);
    }
}
