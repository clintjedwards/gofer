//! Gets every task in a config an image, for `gofer up`. A task that didn't change keeps the image the current version
//! has. The rest are built, and only the blobs Gofer doesn't have yet are uploaded.

use crate::{
    cli::{
        Cli, api_error_message,
        builder::{self, Builder, Built, Image, Source, report, tips},
    },
    conf::cli::OutputFormat,
    images::Digest,
};
use anyhow::{Context, Result, anyhow, bail};
use futures::StreamExt;
use gofer_sdk::api::types;
use polyfmt::{Format, print};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    path::Path,
    sync::Mutex,
    time::{Duration, Instant},
};

/// How many images are built at once. One build can keep every core busy on its own, so running many more than this at
/// once mostly makes each of them slower.
const PARALLEL_BUILDS: usize = 4;

/// A task's files, found and hashed before anything is built.
pub struct Prepared {
    pub build: gofer_sdk::config::Build,
    pub sources: Vec<Source>,
    pub source_hash: String,
}

/// Finds and hashes the files every task copies in. The hash is how `gofer up` tells whether a task changed.
pub fn prepare(
    config_folder: &Path,
    config: &types::Pipeline,
) -> Result<HashMap<String, Prepared>> {
    let mut prepared = HashMap::new();

    for task in &config.tasks {
        let could_not = || format!("Could not read the files for task '{}'", task.id);

        // The API's copy of this type and the SDK's are the same JSON, and the builder takes the SDK's.
        let build: gofer_sdk::config::Build =
            serde_json::from_value(serde_json::to_value(&task.build)?).with_context(could_not)?;
        let sources = builder::sources(&build, config_folder).with_context(could_not)?;
        let source_hash = builder::source_hash(&sources).with_context(could_not)?;

        prepared.insert(
            task.id.clone(),
            Prepared {
                build,
                sources,
                source_hash,
            },
        );
    }

    Ok(prepared)
}

/// The images that tasks can keep from the current version.
#[derive(Default)]
pub struct Reusable {
    /// Images from the current version for tasks that are built the same way, from the same files. Gofer still has
    /// all of them.
    pub images: HashMap<String, types::BuiltImage>,

    /// Tasks that could have kept their image, except Gofer doesn't have it anymore.
    pub gone: Vec<String>,
}

impl Cli {
    /// Works out which tasks can keep the image the current version has, so they aren't built again.
    pub async fn reusable_images(
        &self,
        namespace: &str,
        config: &types::Pipeline,
        prepared: &HashMap<String, Prepared>,
        current: Option<&types::Config>,
    ) -> Result<Reusable> {
        let mut reusable = Reusable::default();
        let Some(current) = current else {
            return Ok(reusable);
        };

        for task in &config.tasks {
            let Some(old) = current.tasks.get(&task.id) else {
                continue;
            };

            // The entrypoint and command are built into the image, so a change to either needs a new one.
            let same = serde_json::to_value(&old.build)? == serde_json::to_value(&task.build)?
                && old.entrypoint == task.entrypoint
                && old.command == task.command
                && old.image.source_hash == prepared[&task.id].source_hash;

            if same {
                reusable.images.insert(task.id.clone(), old.image.clone());
            }
        }

        if reusable.images.is_empty() {
            return Ok(reusable);
        }

        // An image's manifest goes with the rest of it, so asking about the manifest is enough.
        let manifests: BTreeSet<String> = (reusable.images.values())
            .map(|image| image.digest.clone())
            .collect();
        let missing = self
            .missing_blobs(namespace, &config.id, manifests.into_iter().collect())
            .await?;

        let gone = &mut reusable.gone;
        reusable.images.retain(|id, image| {
            let kept = !missing.contains(&image.digest);
            if !kept {
                gone.push(id.clone());
            }
            kept
        });
        gone.sort();

        Ok(reusable)
    }

    /// Builds images for these tasks, showing how each is going, and returns them with the tasks they're for. Tasks that
    /// are built exactly the same way, from the same files, share one build. Builds that work are kept even when
    /// another fails, so every failure shows up at once.
    pub async fn build_images(
        &self,
        namespace: &str,
        config: &types::Pipeline,
        tasks: &[&types::Task2],
        prepared: &HashMap<String, Prepared>,
        rebuild: bool,
    ) -> Result<Vec<(Vec<String>, Built)>> {
        let mut groups: BTreeMap<String, Vec<&types::Task2>> = BTreeMap::new();
        for task in tasks {
            let key = serde_json::to_string(&(
                &task.build,
                &task.entrypoint,
                &task.command,
                &prepared[&task.id].source_hash,
            ))?;
            groups.entry(key).or_default().push(task);
        }

        // A task that's only a base image is made without Docker, so a pipeline of those doesn't need it at all.
        let needs_docker: Vec<&str> = (tasks.iter())
            .filter(|task| builder::needs_docker(&prepared[&task.id].build))
            .map(|task| task.id.as_str())
            .collect();
        let builder = match needs_docker.is_empty() {
            true => None,
            false => Some(
                Builder::new(self.conf.builder_cache_max_gb).with_context(|| {
                    format!(
                        "Building the images for {} needs Docker",
                        needs_docker.join(", ")
                    )
                })?,
            ),
        };

        if let Some(builder) = &builder
            && builder.emulated().await.unwrap_or(false)
        {
            report::emulated();
        }

        // The step each running build is on, and since when, for the status line.
        let running: Mutex<BTreeMap<String, (String, Instant)>> = Mutex::new(BTreeMap::new());

        let builds = futures::stream::iter(groups.into_values())
            .map(|tasks| {
                let (builder, running) = (builder.as_ref(), &running);
                async move {
                    let first = tasks[0];
                    let files = &prepared[&first.id];
                    let name = format!("{namespace}/{}/{}", config.id, first.id);
                    let task = builder::Task {
                        name: &name,
                        build: &files.build,
                        entrypoint: first.entrypoint.as_deref(),
                        command: first.command.as_deref(),
                    };

                    let started = Instant::now();
                    let set_step = |step: &str| {
                        (running.lock().unwrap())
                            .insert(first.id.clone(), (step.into(), Instant::now()));
                    };
                    set_step("starting");

                    let result = builder::build(
                        builder,
                        &task,
                        &files.sources,
                        &files.source_hash,
                        rebuild,
                        set_step,
                    )
                    .await;
                    running.lock().unwrap().remove(&first.id);

                    let ids: Vec<String> = tasks.iter().map(|task| task.id.clone()).collect();
                    (ids, started.elapsed(), result)
                }
            })
            .buffer_unordered(PARALLEL_BUILDS);
        tokio::pin!(builds);

        let mut ticker = tokio::time::interval(Duration::from_secs(1));
        let mut built = vec![];
        let mut failed = 0;

        loop {
            tokio::select! {
                next = builds.next() => {
                    let Some((ids, took, result)) = next else {
                        break;
                    };

                    match result {
                        Ok(image) => {
                            report::built(&ids.join(", "), took);
                            built.push((ids, image));
                        }
                        Err(e) => {
                            report::failed(&ids.join(", "), &e);
                            failed += 1;
                        }
                    }
                }

                _ = ticker.tick() => {
                    let running = running.lock().unwrap();
                    let now = Instant::now();
                    let steps: Vec<(&str, &str, Duration)> = (running.iter())
                        .map(|(id, (step, since))| (id.as_str(), step.as_str(), now - *since))
                        .collect();
                    report::building(&steps);
                }
            }
        }

        if failed > 0 {
            bail!("{failed} of the images couldn't be built, so nothing was registered");
        }

        // Tips are only for people, and they're only remembered as shown when they really were.
        if self.conf.build_tips > 0
            && matches!(
                self.conf.output_format,
                OutputFormat::Spinner | OutputFormat::Plain
            )
            && let Some(store) = tips::store()
        {
            let names: Vec<String> = (built.iter())
                .map(|(ids, _)| format!("{namespace}/{}/{}", config.id, ids[0]))
                .collect();
            let found: Vec<(&str, Vec<tips::Tip>)> = (names.iter())
                .zip(&built)
                .map(|(name, (_, built))| (name.as_str(), built.tips.clone()))
                .collect();
            report::tips(&tips::pick(&store, &found, self.conf.build_tips));
        }

        Ok(built)
    }

    /// Uploads the parts of these images that Gofer doesn't have yet. Images share layers with each other, and with the
    /// pipeline's earlier versions, so this is often only the few that changed.
    pub async fn upload_images(
        &self,
        namespace: &str,
        pipeline_id: &str,
        built: &[(Vec<String>, Built)],
    ) -> Result<()> {
        // Images on the same base share its layers. One that has them on this machine saves downloading them.
        let mut blobs: BTreeMap<String, (&Image, &Digest, u64)> = BTreeMap::new();
        for (_, built) in built {
            for (digest, size) in &built.image.blobs {
                let blob = blobs
                    .entry(digest.to_string())
                    .or_insert((&built.image, digest, *size));
                if built.image.has(digest) {
                    *blob = (&built.image, digest, *size);
                }
            }
        }

        let missing = self
            .missing_blobs(namespace, pipeline_id, blobs.keys().cloned().collect())
            .await?;
        let total: u64 = missing.iter().map(|digest| blobs[digest].2).sum();
        let mut sent = 0;

        for (number, digest) in missing.iter().enumerate() {
            print!(
                "Uploading images: part {} of {}, {} of {}",
                number + 1,
                missing.len(),
                report::bytes(sent),
                report::bytes(total);
                vec![Format::Spinner]
            );

            // A base layer that isn't on this machine goes straight from its registry into the upload.
            let (image, blob, size) = blobs[digest];
            let body = reqwest::Body::wrap_stream(image.open(blob).await?);

            self.client
                .put_image_blob(namespace, pipeline_id, digest, body)
                .await
                .map_err(|e| {
                    anyhow!(
                        "Could not upload image part '{digest}'; {}",
                        api_error_message(e)
                    )
                })?;

            sent += size;
        }

        Ok(())
    }

    async fn missing_blobs(
        &self,
        namespace: &str,
        pipeline_id: &str,
        digests: Vec<String>,
    ) -> Result<BTreeSet<String>> {
        let response = self
            .client
            .list_missing_image_blobs(
                namespace,
                pipeline_id,
                &types::ListMissingImageBlobsRequest { digests },
            )
            .await
            .map_err(|e| {
                anyhow!(
                    "Could not ask Gofer which images it has; {}",
                    api_error_message(e)
                )
            })?;

        Ok(response.into_inner().digests.into_iter().collect())
    }
}
