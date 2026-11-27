//! Builds the images that tasks run as.
//!
//! A task says which base image to start from, what to copy in, what to run and how to run the result, or it points at
//! a Dockerfile. `gofer up` turns that into an image and uploads it to the server, so a version of a pipeline always
//! runs exactly the image it was registered with. Getting from one to the other goes like this.
//!
//! A task that's only a base image and a command has nothing to build, so it skips all of this: [`direct`] makes its
//! image from the base's registry, without Docker.
//!
//! # 1. Find the files and hash them
//!
//! [`source`] walks every path the task copies. It leaves out what `.dockerignore` says to, matched the way Docker does
//! by [`dockerignore`], and for `copy` it also leaves out folders marked as caches, like Cargo's `target/`. Then it
//! hashes what's left: paths, contents and the executable bit, but not times or owners, so the same checkout gives the
//! same hash on every machine.
//!
//! The hash is how `gofer up` knows a task hasn't changed, so it can skip building it at all. BuildKit hashes files
//! too, but only once a build is already running.
//!
//! # 2. Write a Dockerfile
//!
//! [`dockerfile`] turns the steps into one. The base is `FROM`, `copy` is `COPY`, `run` is `RUN`, and `keep` is a
//! second stage that takes only the paths it names, leaving the compiler behind. Everything the user wrote goes in as
//! JSON, so a newline in a command can't turn into another instruction. How the image is run isn't in it; that comes
//! in step 6.
//!
//! Every `RUN` also gets cache mounts and goes through `run.sh`, which points Go and Cargo at caches that last between
//! builds. That's what makes a plain `cargo build --release` fast after the first time. Cargo decides what to rebuild
//! from file times, and BuildKit's cache can bring back old ones, so `run.sh` makes the copied files new first. It
//! explains why in more detail.
//!
//! A task with a Dockerfile of its own skips this step.
//!
//! # 3. Start the builder
//!
//! [`buildkit`] runs BuildKit in a privileged container on the user's Docker, `gofer-buildkit`, with its cache in a
//! volume. It stays running between builds, and gets replaced when its pinned version or its settings change. Docker
//! has a BuildKit of its own that would save running a container, but it hands back uncompressed layers on Docker's
//! older image store, and its cache is the user's own; the module has the details.
//!
//! # 4. Send the files in
//!
//! [`context`] writes the Dockerfile and the files into a tar while it's being uploaded into the container, so memory
//! stays flat however big they are. Every file is owned by root and keeps only its executable bit, to match the hash.
//!
//! BuildKit can't see the user's Docker logins from inside its container, so [`login`] finds the ones for the
//! registries this build downloads from, and those go in too.
//!
//! # 5. Build
//!
//! `buildctl`, BuildKit's own client, runs inside the container through `docker exec`, wrapped in `build.sh`. Docker
//! can't stop an exec, but when the CLI goes away the exec's stdin always closes, and `build.sh` stops the build and
//! removes its files when that happens. That covers Ctrl-C, a crash and `kill -9` alike.
//!
//! # 6. Take the image apart
//!
//! The image comes back on stdout as a tar. [`image`] unpacks it as it streams, checks each blob against its digest,
//! and keeps only the blobs, which are what the server stores. Then it writes the task's entrypoint and command into
//! the image's config, combining them with the base image's the same way Docker would.
//!
//! # 7. Explain what happened
//!
//! BuildKit's progress comes back on stderr. [`progress`] names each step after what the user wrote, for the status
//! line, writes everything to a log from [`logs`], and turns a failure into an error that says which step failed and
//! why. [`tips`] has advice for afterwards, and [`report`] prints all of it.
//!
//! [`build`] does all of this for one task. At the end it hashes the files again, and if they changed during the build
//! it refuses the image, which could hold some of the old files and some of the new.

mod buildkit;
mod context;
mod direct;
mod dockerfile;
mod dockerignore;
mod image;
mod login;
mod logs;
mod progress;
pub mod report;
mod source;
pub mod tips;

use crate::cli::Cli;
use anyhow::{Context, Result, bail};
pub use buildkit::Builder;
use buildkit::NoDocker;
use clap::{Args, Subcommand};
use context::Contents;
pub use image::Image;
use login::Logins;
use polyfmt::{Format, print, success};
use progress::{Failure, Progress};
pub use source::{Source, hash as source_hash};
use std::{io::Write, path::Path, time::Instant};
use tips::Tip;

// Tasks say how their images are built in the SDK's own types, the same ones the server stores.
pub use gofer_sdk::config::{Build, Step, Steps};

/// Linux won't start a program with an argument longer than this, counting the NUL at its end. It's 32 pages, and a
/// page is 4KB on amd64.
///
/// <https://github.com/torvalds/linux/blob/v6.12/include/uapi/linux/binfmts.h>
const MAX_ARGUMENT_BYTES: usize = 32 * 4096;

#[derive(Debug, Args, Clone)]
pub struct BuilderSubcommands {
    #[clap(subcommand)]
    pub command: BuilderCommands,
}

#[derive(Debug, Subcommand, Clone)]
pub enum BuilderCommands {
    /// Empty the builder's cache.
    ///
    /// The cache is what makes builds after the first one fast. It never grows past `builder_cache_max_gb`, so this
    /// is only needed to get the space back right away.
    Prune,

    /// Remove the builder and its cache. The next build starts a new one.
    Remove,
}

impl Cli {
    pub async fn handle_builder_subcommands(&self, command: BuilderSubcommands) -> Result<()> {
        let builder = Builder::new(self.conf.builder_cache_max_gb)?;

        match command.command {
            BuilderCommands::Prune => {
                print!("Emptying the builder's cache"; vec![Format::Spinner]);
                let freed = builder.prune().await?;
                success!("Emptied the builder's cache, which freed {freed}");
            }
            BuilderCommands::Remove => {
                print!("Removing the builder"; vec![Format::Spinner]);
                if builder.remove().await? {
                    success!("Removed the builder and its cache");
                } else {
                    success!("There was no builder to remove");
                }
            }
        }

        Ok(())
    }
}

/// A task whose image is being built.
pub struct Task<'a> {
    /// Like `namespace/pipeline/task`. Each task gets compiler caches of its own.
    pub name: &'a str,

    pub build: &'a Build,

    pub entrypoint: Option<&'a [String]>,

    pub command: Option<&'a [String]>,
}

impl Task<'_> {
    /// The task's own id, without its namespace and pipeline.
    pub fn id(&self) -> &str {
        self.name.rsplit('/').next().unwrap_or(self.name)
    }
}

/// A task's finished image.
pub struct Built {
    pub image: Image,

    /// Everything the user could improve. [`tips::pick`] decides which ones are worth saying.
    pub tips: Vec<Tip>,
}

/// Goes next to the Dockerfile, where BuildKit looks before it reads the `.dockerignore` in the context. What that file
/// says to leave out is already gone, and this stops BuildKit from doing it again in a slightly different way.
const IGNORE_FILE: &str = "Dockerfile.dockerignore";
const IGNORE_FILE_CONTENT: &str = "# Gofer has already left out what .dockerignore says to.\n";

/// Whether building this image needs Docker, which is for anything more than a base and a command.
pub fn needs_docker(build: &Build) -> bool {
    !direct::takes(build)
}

/// Builds a task's image. The sources have to be the ones from [`sources`], and `source_hash` their hash. With
/// `no_cache`, nothing is reused from earlier builds. `on_step` hears about each step as it starts, by what the user
/// calls it. `builder` is only used for an image that [`needs_docker`].
///
/// When the build itself fails, the error is a [`Failure`], which says why and where the log is.
pub async fn build(
    builder: Option<&Builder>,
    task: &Task<'_>,
    sources: &[Source],
    source_hash: &str,
    no_cache: bool,
    mut on_step: impl FnMut(&str),
) -> Result<Built> {
    // Better to find out before building than when the task can't start.
    for argument in task
        .entrypoint
        .iter()
        .chain(&task.command)
        .copied()
        .flatten()
    {
        if argument.len() >= MAX_ARGUMENT_BYTES {
            bail!(
                "The command for '{}' has an argument of {}KB, and Linux won't start a program with one of 128KB or \
                 more; put it in a file and copy that in instead",
                task.id(),
                argument.len() / 1024
            );
        }
    }

    if let Build::Steps(steps) = task.build
        && direct::takes(task.build)
    {
        // The same name BuildKit's step for this gets.
        on_step(&format!("look up {}", steps.base));
        let image = direct::build(&steps.base, task.entrypoint, task.command).await?;

        let tips = tips::find(&tips::Seen {
            task,
            sources,
            uncached_rust: &[],
            old_cargo: None,
            image_size: image.size(),
        });
        return Ok(Built { image, tips });
    }

    let mut files = vec![(IGNORE_FILE.to_string(), IGNORE_FILE_CONTENT.into())];
    let mut made_dockerfile = None;
    let mut uncached_rust = vec![];

    // Every image the build downloads, so it can have the user's logins for their registries.
    let mut images = vec![];

    let mut contents = match task.build {
        Build::Steps(steps) => {
            images.push(steps.base.clone());
            images.extend(steps.keep.as_ref().map(|keep| keep.base.clone()));

            let rendered = dockerfile::render(task.name, steps, sources, no_cache)?;
            made_dockerfile = Some(rendered.dockerfile.clone());
            uncached_rust = rendered.uncached_rust;
            files.push(("Dockerfile".into(), rendered.dockerfile.into_bytes()));
            files.extend(
                (rendered.files.into_iter())
                    .map(|(path, content)| (format!("context/{path}"), content)),
            );

            let copies = sources.iter().enumerate().map(|(index, source)| {
                (
                    format!("context/{}", dockerfile::copy_folder(index)),
                    source.clone(),
                )
            });

            Contents {
                files,
                private_files: vec![],
                sources: copies.collect(),
            }
        }

        Build::Dockerfile { path, .. } => {
            let [dockerfile, context] = sources else {
                bail!(
                    "Expected the Dockerfile and its context, but got {} sources",
                    sources.len()
                );
            };

            let content = std::fs::read(dockerfile.path_of(&dockerfile.entries[0]))
                .with_context(|| format!("Could not read '{path}'"))?;
            images = dockerfile::images(&String::from_utf8_lossy(&content))
                .with_context(|| format!("Could not read the Dockerfile '{path}'"))?;
            files.push(("Dockerfile".into(), content));

            Contents {
                files,
                private_files: vec![],
                sources: vec![("context/".into(), context.clone())],
            }
        }
    };

    let Some(builder) = builder else {
        bail!(
            "Building '{}' needs Docker, because {}",
            task.id(),
            needs_docker_because(task.build)
        );
    };
    if let Err(e) = builder.start().await {
        return match e.downcast_ref::<NoDocker>() {
            Some(_) => Err(e.context(format!(
                "Building '{}' needs Docker, because {}",
                task.id(),
                needs_docker_because(task.build)
            ))),
            None => Err(e),
        };
    }

    // BuildKit can't see the user's logins from inside its container, so the ones for this build's registries go in
    // with its files, and build.sh points it at them. BuildKit only reads the Dockerfile and its ignore file from this
    // folder, so the logins can't end up in the image.
    let images: Vec<&str> = images.iter().map(String::as_str).collect();
    let logins = login::find(&images).await;
    (contents.private_files).push(("docker/config.json".into(), logins.docker_config()));

    let (log_path, mut log) = logs::create(task.name)?;
    _ = writeln!(
        log,
        "Building {} on {}\n",
        task.name,
        chrono::Local::now().to_rfc2822()
    );
    if let Some(dockerfile) = made_dockerfile {
        let indented: Vec<String> = dockerfile
            .lines()
            .map(|line| format!("    {line}"))
            .collect();
        _ = writeln!(
            log,
            "The Dockerfile made from its steps:\n\n{}\n",
            indented.join("\n")
        );
    }

    let started = Instant::now();
    let mut progress = Progress::new(task.build, &mut log);
    let mut step = String::new();

    let built = builder
        .build(contents, no_cache, task.entrypoint, task.command, |line| {
            progress.update(line);
            if let Some(now) = progress.step()
                && now != step
            {
                now.clone_into(&mut step);
                on_step(now);
            }
        })
        .await;

    let image = match built {
        Ok(image) => image,
        Err(e) => {
            let error = explain(e, &progress, &log_path, &logins);
            let message = match error.downcast_ref::<Failure>() {
                Some(failure) => failure.message.clone(),
                None => format!("{error:#}"),
            };
            progress.note(&format!(
                "\nFailed after {:.1}s: {message}",
                started.elapsed().as_secs_f64()
            ));
            return Err(error);
        }
    };

    // The hash is how a later `gofer up` knows whether to build again. If files changed while they were being copied
    // in, the image could have some of the new ones and some of the old, and the hash would describe neither.
    if source::hash(sources)? != source_hash {
        progress.note("\nFiles changed while the image was being built, so it isn't used");
        bail!(
            "Files for '{}' changed while its image was being built; build it again so the image matches them",
            task.id()
        );
    }

    progress.note(&format!(
        "\nBuilt in {:.1}s: {}",
        started.elapsed().as_secs_f64(),
        image.manifest
    ));

    let tips = tips::find(&tips::Seen {
        task,
        sources,
        uncached_rust: &uncached_rust,
        old_cargo: progress.old_cargo(),
        image_size: image.size(),
    });

    Ok(Built { image, tips })
}

/// Says what went wrong in the user's terms, where we can tell.
fn explain(
    e: anyhow::Error,
    progress: &Progress<impl Write>,
    log: &Path,
    logins: &Logins,
) -> anyhow::Error {
    if let Some(failed) = e.downcast_ref::<buildkit::Failed>() {
        // Something like a Dockerfile that can't be read doesn't fail a step, and BuildKit's message says it best.
        return match progress.failure(log.to_path_buf(), logins) {
            Some(failure) => failure.into(),
            None => Failure {
                message: failed.to_string(),
                output: vec![],
                log: log.to_path_buf(),
            }
            .into(),
        };
    }

    // Either on this machine, while unpacking the image, or in Docker, while copying the files in.
    if format!("{e:#}")
        .to_lowercase()
        .contains("no space left on device")
    {
        return e.context(
            "Ran out of disk space. `gofer builder prune` empties the builder's cache, which is often what takes \
             up the most",
        );
    }

    e
}

/// For when Docker isn't there, so the user knows why this task needs it.
fn needs_docker_because(build: &Build) -> &'static str {
    match build {
        Build::Dockerfile { .. } => "it's built from a Dockerfile",
        Build::Steps(steps)
            if (steps.steps.iter()).any(|step| matches!(step, Step::Run { .. })) =>
        {
            "it has run steps"
        }
        Build::Steps(steps) if steps.keep.is_some() => "it uses keep",
        Build::Steps(_) => "it copies files into its base image",
    }
}

/// Finds the files that go into a task's image. For steps that's one source per copy, in order. For a Dockerfile it's
/// the Dockerfile and then its context.
pub fn sources(build: &Build, config_folder: &Path) -> Result<Vec<Source>> {
    match build {
        Build::Steps(steps) => steps
            .steps
            .iter()
            .filter_map(|step| match step {
                Step::Copy { from, .. } => Some(
                    Source::copy(&config_folder.join(from))
                        .with_context(|| format!("Could not copy '{from}'")),
                ),
                Step::Run { .. } => None,
            })
            .collect(),

        Build::Dockerfile { path, context } => {
            // Docker lets a Dockerfile have an ignore file of its own, which replaces the one in the context.
            let ignore_file = config_folder.join(format!("{path}.dockerignore"));
            let ignore_file = ignore_file.exists().then_some(ignore_file.as_path());

            Ok(vec![
                Source::copy(&config_folder.join(path))
                    .with_context(|| format!("Could not read the Dockerfile '{path}'"))?,
                Source::context(&config_folder.join(context), ignore_file)
                    .with_context(|| format!("Could not read the build context '{context}'"))?,
            ])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use std::fs;

    fn names(source: &Source) -> Vec<&str> {
        source
            .entries
            .iter()
            .map(|entry| entry.name.to_str().unwrap())
            .collect()
    }

    #[test]
    fn test_steps_have_a_source_per_copy() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("pipeline/src")).unwrap();
        fs::write(dir.path().join("pipeline/src/main.py"), "").unwrap();
        fs::write(dir.path().join("shared.py"), "").unwrap();

        let copy = |from: &str| Step::Copy {
            from: from.into(),
            to: "/app".into(),
        };
        let build = Build::Steps(Steps {
            base: "python:3".into(),
            steps: vec![
                copy("src"),
                Step::Run {
                    command: "true".into(),
                },
                copy("../shared.py"),
            ],
            keep: None,
        });

        let sources = super::sources(&build, &dir.path().join("pipeline")).unwrap();
        assert_eq!(sources.len(), 2);
        assert_eq!(names(&sources[0]), vec!["main.py"]);
        assert_eq!(names(&sources[1]), vec!["shared.py"]);

        // The path is the first thing someone needs to fix this.
        let build = Build::Steps(Steps {
            base: "python:3".into(),
            steps: vec![copy("./missing")],
            keep: None,
        });
        let error = super::sources(&build, dir.path()).unwrap_err().to_string();
        assert!(error.contains("./missing"), "{error}");
    }

    #[test]
    fn test_dockerfiles_can_have_their_own_ignore_file() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("Dockerfile"), "FROM scratch").unwrap();
        fs::write(dir.path().join(".dockerignore"), "Dockerfile*\n.docker*").unwrap();
        fs::write(dir.path().join("a"), "").unwrap();
        let build = Build::Dockerfile {
            path: "./Dockerfile".into(),
            context: ".".into(),
        };

        // Ignoring the Dockerfile only keeps it out of the context.
        let sources = super::sources(&build, dir.path()).unwrap();
        assert_eq!(names(&sources[0]), vec!["Dockerfile"]);
        assert_eq!(names(&sources[1]), vec!["a"]);

        fs::write(dir.path().join("Dockerfile.dockerignore"), "*\n!Dockerfile").unwrap();
        let sources = super::sources(&build, dir.path()).unwrap();
        assert_eq!(names(&sources[1]), vec!["Dockerfile"]);
    }

    #[test]
    fn test_only_copies_leave_out_caches() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("target")).unwrap();
        fs::write(
            dir.path().join("target/CACHEDIR.TAG"),
            "Signature: 8a477f597d28d172789f06886806bc55",
        )
        .unwrap();
        fs::write(dir.path().join("Dockerfile"), "FROM scratch").unwrap();

        let copy = Build::Steps(Steps {
            base: "rust:1".into(),
            steps: vec![Step::Copy {
                from: ".".into(),
                to: "/src".into(),
            }],
            keep: None,
        });
        assert_eq!(
            names(&super::sources(&copy, dir.path()).unwrap()[0]),
            vec!["Dockerfile"]
        );

        // A Dockerfile can only use what it was given, so `COPY target/...` has to work like it does with Docker.
        let dockerfile = Build::Dockerfile {
            path: "Dockerfile".into(),
            context: ".".into(),
        };
        assert_eq!(
            names(&super::sources(&dockerfile, dir.path()).unwrap()[1]),
            vec!["Dockerfile", "target", "target/CACHEDIR.TAG"]
        );
    }
}
