//! Keeps the image store from growing forever.
//!
//! Each pipeline keeps the images of its newest `retention_versions` versions, and of any version that's live, being
//! deployed, or has runs that haven't finished, so nothing that can still run loses its image. The rest go in two
//! steps. First the pipeline's rows in `image_blobs` for them are dropped. Pipelines share blob files, so a file only
//! goes once no pipeline has a row for it, and the same goes for an image's copy in Docker.
//!
//! A blob nothing uses is still kept for a day after Gofer last saw it. `gofer up` asks which blobs Gofer has, uploads
//! the rest and then registers, so what Gofer said it has has to stay until then. Being asked about counts as being
//! seen, and so does being uploaded.
//!
//! This runs for a pipeline after it registers a version, and for every pipeline when Gofer starts and every hour after.
//! The hourly one catches what registering doesn't: runs that finished, uploads that were never registered, and deleted
//! pipelines.
//!
//! What a pipeline keeps is worked out in the same write transaction that drops the rest. Gofer has one write
//! connection, so a deploy, which checks its version's images in a transaction of its own, never lands in between.

use crate::{
    api::ApiState,
    images::{Digest, Images},
    storage::{self, image_blobs},
};
use anyhow::{Context, Result};
use std::{
    collections::HashSet,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tracing::{error, info, warn};

/// How long a blob nothing uses is kept after Gofer last saw it.
const GRACE: Duration = Duration::from_secs(24 * 60 * 60);

/// How often every pipeline is cleaned up.
const INTERVAL: Duration = Duration::from_secs(60 * 60);

/// Cleans up every pipeline now, and then every hour for as long as Gofer runs.
pub async fn run(api_state: Arc<ApiState>) {
    let mut interval = tokio::time::interval(INTERVAL);

    loop {
        interval.tick().await;

        if let Err(e) = clean_all(&api_state).await {
            error!(error = format!("{e:#}"), "Could not clean up images");
        }
    }
}

/// Cleans up after one pipeline, like when it's registered a new version. Nothing waits for this, so problems are only
/// logged.
pub async fn clean_pipeline(api_state: Arc<ApiState>, namespace_id: String, pipeline_id: String) {
    let now = SystemTime::now();

    let result = async {
        let dropped = drop_unkept(
            &api_state.storage,
            &api_state.images,
            &namespace_id,
            &pipeline_id,
            now,
        )
        .await?;
        remove_unused(&api_state.storage, &api_state.images, &dropped, now).await?;
        remove_docker_images(&api_state).await
    }
    .await;

    if let Err(e) = result {
        error!(
            namespace_id,
            pipeline_id,
            error = format!("{e:#}"),
            "Could not clean up a pipeline's images"
        );
    }
}

async fn clean_all(api_state: &ApiState) -> Result<()> {
    let now = SystemTime::now();

    let pipelines = {
        let mut conn = api_state.storage.read_conn().await?;
        image_blobs::list_pipelines(&mut conn).await?
    };

    // One pipeline that can't be cleaned up shouldn't stop the rest. Its rows stay, so its files do too.
    for (namespace_id, pipeline_id) in pipelines {
        if let Err(e) = drop_unkept(
            &api_state.storage,
            &api_state.images,
            &namespace_id,
            &pipeline_id,
            now,
        )
        .await
        {
            error!(
                namespace_id,
                pipeline_id,
                error = format!("{e:#}"),
                "Could not clean up a pipeline's images"
            );
        }
    }

    let stored = api_state.images.list().await?;
    remove_unused(&api_state.storage, &api_state.images, &stored, now).await?;
    remove_docker_images(api_state).await
}

/// Drops a pipeline's rows for blobs that none of the versions it keeps use and that Gofer hasn't seen for a day, and
/// returns which blobs they were.
async fn drop_unkept(
    storage: &storage::Db,
    images: &Images,
    namespace_id: &str,
    pipeline_id: &str,
    now: SystemTime,
) -> Result<Vec<Digest>> {
    let mut tx = storage.open_tx().await?;

    let manifests = image_blobs::list_kept_images(
        &mut tx,
        namespace_id,
        pipeline_id,
        images.retention_versions.try_into()?,
    )
    .await?;

    let mut kept = HashSet::new();
    for manifest in manifests {
        let manifest: Digest = manifest.parse()?;

        // An image whose manifest is gone can't run anyway, so its other blobs needn't stay for it.
        match images.blobs_of(&manifest).await? {
            Some(blobs) => kept.extend(blobs),
            None => {
                kept.insert(manifest);
            }
        }
    }

    let cutoff = epoch_milli(now - GRACE)?;
    let mut dropped = vec![];

    for digest in image_blobs::list_seen_before(&mut tx, namespace_id, pipeline_id, cutoff).await? {
        let digest: Digest = digest.parse()?;
        if kept.contains(&digest) {
            continue;
        }

        image_blobs::delete(&mut tx, namespace_id, pipeline_id, &digest.to_string()).await?;
        dropped.push(digest);
    }

    tx.commit().await?;

    if !dropped.is_empty() {
        info!(
            namespace_id,
            pipeline_id,
            blobs = dropped.len(),
            "Dropped the image blobs a pipeline no longer keeps"
        );
    }

    Ok(dropped)
}

/// Deletes the files of these blobs that no pipeline has anymore, unless they were uploaded in the last day.
async fn remove_unused(
    storage: &storage::Db,
    images: &Images,
    digests: &[Digest],
    now: SystemTime,
) -> Result<()> {
    let mut conn = storage.read_conn().await?;
    let mut removed = 0;

    for digest in digests {
        if image_blobs::is_used(&mut conn, &digest.to_string()).await? {
            continue;
        }

        if images.remove_unless_newer(digest, now - GRACE).await? {
            removed += 1;
        }
    }

    if removed > 0 {
        info!(removed, "Removed image blobs that no pipeline keeps");
    }

    Ok(())
}

/// Removes the task images from Docker whose manifest no pipeline has anymore. A version that can still run keeps its
/// blobs, so this never removes one a task is about to start from.
async fn remove_docker_images(api_state: &ApiState) -> Result<()> {
    let names = (api_state.containers.list_task_images().await)
        .context("Could not list the task images in Docker")?;
    let mut conn = api_state.storage.read_conn().await?;
    let mut removed = 0;

    for name in names {
        let Some(manifest) = Digest::from_docker_name(&name) else {
            continue;
        };
        if image_blobs::is_used(&mut conn, &manifest.to_string()).await? {
            continue;
        }

        // Docker won't remove one a running container uses. It'll go next time.
        match api_state.containers.remove_image(&name).await {
            Ok(()) => removed += 1,
            Err(e) => warn!(image = name, error = %e, "Could not remove a task image from Docker"),
        }
    }

    if removed > 0 {
        info!(
            removed,
            "Removed task images from Docker that no pipeline keeps"
        );
    }

    Ok(())
}

fn epoch_milli(time: SystemTime) -> Result<u64> {
    Ok(time.duration_since(UNIX_EPOCH)?.as_millis().try_into()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        api::{deployments, pipeline_configs::ConfigState, runs},
        images::Config,
        storage::tests::TestHarness,
    };
    use sha2::{Digest as _, Sha256};

    const NAMESPACE: &str = "default";
    const PIPELINE: &str = "pipeline";

    struct Harness {
        db: TestHarness,
        images: Images,
        _dir: tempfile::TempDir,
    }

    fn digest(content: &[u8]) -> Digest {
        Digest::from_hasher(Sha256::new_with_prefix(content))
    }

    impl Harness {
        async fn new(retention_versions: u64) -> Self {
            let dir = tempfile::tempdir().unwrap();
            let images = Images::new(&Config {
                path: dir.path().to_string_lossy().into(),
                retention_versions,
            })
            .await
            .unwrap();

            let db = TestHarness::new().await;
            let mut conn = db.write_conn().await.unwrap();
            storage::namespaces::insert(
                &mut conn,
                &storage::namespaces::Namespace {
                    id: NAMESPACE.into(),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
            storage::pipeline_metadata::insert(
                &mut conn,
                &storage::pipeline_metadata::PipelineMetadata {
                    namespace_id: NAMESPACE.into(),
                    pipeline_id: PIPELINE.into(),
                    ..Default::default()
                },
            )
            .await
            .unwrap();

            Self {
                db,
                images,
                _dir: dir,
            }
        }

        /// Stores a blob and says `pipeline` uploaded it just now.
        async fn upload(&self, pipeline: &str, content: &[u8]) -> Digest {
            let digest = digest(content);
            let chunk: Result<_, std::io::Error> = Ok(bytes::Bytes::copy_from_slice(content));
            let blob = (self.images)
                .put(&digest, futures::stream::iter([chunk]))
                .await
                .unwrap();

            let mut conn = self.db.write_conn().await.unwrap();
            image_blobs::upsert(
                &mut conn,
                &image_blobs::ImageBlob {
                    namespace_id: NAMESPACE.into(),
                    pipeline_id: pipeline.into(),
                    digest: digest.to_string(),
                    diff_id: blob.diff_id.to_string(),
                    size: blob.size as i64,
                    last_seen: epoch_milli(SystemTime::now()).unwrap().to_string(),
                },
            )
            .await
            .unwrap();

            digest
        }

        /// Registers a version whose one task has an image of its own on top of a layer every version shares.
        async fn register(&self, version: i64, state: ConfigState) -> Vec<Digest> {
            let layer = self.upload(PIPELINE, b"shared layer").await;
            let config = self
                .upload(PIPELINE, format!("config {version}").as_bytes())
                .await;
            let manifest = serde_json::to_vec(&serde_json::json!({
                "config": {"digest": config.to_string()},
                "layers": [{"digest": layer.to_string()}],
            }))
            .unwrap();
            let manifest = self.upload(PIPELINE, &manifest).await;

            let mut conn = self.db.write_conn().await.unwrap();
            storage::pipeline_configs::insert(
                &mut conn,
                &storage::pipeline_configs::PipelineConfig {
                    namespace_id: NAMESPACE.into(),
                    pipeline_id: PIPELINE.into(),
                    version,
                    state: state.to_string(),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
            storage::tasks::insert(
                &mut conn,
                &storage::tasks::Task {
                    namespace_id: NAMESPACE.into(),
                    pipeline_id: PIPELINE.into(),
                    pipeline_config_version: version,
                    task_id: "task".into(),
                    image_digest: manifest.to_string(),
                    ..Default::default()
                },
            )
            .await
            .unwrap();

            vec![manifest, config]
        }

        async fn clean(&self, pipeline: &str, now: SystemTime) {
            let dropped = drop_unkept(&self.db, &self.images, NAMESPACE, pipeline, now)
                .await
                .unwrap();
            remove_unused(&self.db, &self.images, &dropped, now)
                .await
                .unwrap();
        }

        async fn stored(&self, digest: &Digest) -> bool {
            self.images.contains(digest).await.unwrap()
        }
    }

    fn two_days_from_now() -> SystemTime {
        SystemTime::now() + 2 * GRACE
    }

    #[tokio::test]
    async fn test_only_versions_that_can_run_keep_their_images() {
        let harness = Harness::new(3).await;

        let live = harness.register(1, ConfigState::Live).await;
        let queued = harness.register(2, ConfigState::Deprecated).await;
        let deploying = harness.register(3, ConfigState::Unreleased).await;
        let old = harness.register(4, ConfigState::Deprecated).await;
        let mut newest = vec![];
        for version in 5..=7 {
            newest.extend(harness.register(version, ConfigState::Unreleased).await);
        }

        let mut conn = harness.db.write_conn().await.unwrap();
        storage::runs::insert(
            &mut conn,
            &storage::runs::Run {
                namespace_id: NAMESPACE.into(),
                pipeline_id: PIPELINE.into(),
                pipeline_config_version: 2,
                run_id: 1,
                state: runs::State::Pending.to_string(),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        storage::deployments::insert(
            &mut conn,
            &storage::deployments::Deployment {
                namespace_id: NAMESPACE.into(),
                pipeline_id: PIPELINE.into(),
                deployment_id: 1,
                start_version: 1,
                end_version: 3,
                state: deployments::State::Running.to_string(),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        drop(conn);

        harness.clean(PIPELINE, two_days_from_now()).await;

        for blob in [live, queued, deploying, newest].concat() {
            assert!(harness.stored(&blob).await, "{blob} should be kept");
        }
        for blob in &old {
            assert!(!harness.stored(blob).await, "{blob} should be removed");
        }
        assert!(harness.stored(&digest(b"shared layer")).await);
    }

    #[tokio::test]
    async fn test_blobs_are_kept_for_a_day() {
        let harness = Harness::new(1).await;
        let old = harness.register(1, ConfigState::Deprecated).await;
        harness.register(2, ConfigState::Live).await;

        harness.clean(PIPELINE, SystemTime::now()).await;

        for blob in &old {
            assert!(harness.stored(blob).await);
        }
    }

    #[tokio::test]
    async fn test_blobs_another_pipeline_has_are_kept() {
        let harness = Harness::new(1).await;
        let old = harness.register(1, ConfigState::Deprecated).await;
        harness.register(2, ConfigState::Live).await;
        harness.upload("other", b"config 1").await;

        harness.clean(PIPELINE, two_days_from_now()).await;

        assert!(!harness.stored(&old[0]).await);
        assert!(harness.stored(&old[1]).await);

        // The row is still dropped, so this pipeline would have to upload it again to use it.
        let mut conn = harness.db.read_conn().await.unwrap();
        assert!(
            image_blobs::get(&mut conn, NAMESPACE, PIPELINE, &old[1].to_string())
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn test_a_blob_uploaded_since_is_kept() {
        let harness = Harness::new(1).await;
        let old = harness.register(1, ConfigState::Deprecated).await;
        harness.register(2, ConfigState::Live).await;

        // Like an upload landing just before cleanup, while no pipeline has it written down yet.
        let file = std::fs::File::options()
            .write(true)
            .open(harness.images.blob_path(&old[0]))
            .unwrap();
        file.set_modified(two_days_from_now()).unwrap();

        harness.clean(PIPELINE, two_days_from_now()).await;

        assert!(harness.stored(&old[0]).await);
        assert!(!harness.stored(&old[1]).await);
    }
}
