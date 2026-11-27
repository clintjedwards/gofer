//! Cleans up after a run once it's over. Its folder goes, along with its tasks' containers, which Docker won't remove
//! the folder without, and so does the API token its tasks were given.
//!
//! A run does this itself when it finishes. A sweep when Gofer starts and every hour after catches what that missed,
//! like a run that finished while Gofer was stopping, or one that was cancelled before it ever started.

use super::{RunShepherd, run_specific_api_key_id};
use crate::{
    api::{ApiState, epoch_milli, runs, secrets},
    storage::{self, StorageError},
};
use anyhow::{Context, Result};
use std::{sync::Arc, time::Duration};
use tracing::{error, warn};

/// How long a run waits for its tasks' logs to be written before it removes their containers anyway.
const LOG_STREAM_WAIT: Duration = Duration::from_secs(60);

/// How often the sweep runs.
const SWEEP_INTERVAL: Duration = Duration::from_secs(60 * 60);

/// How long the sweep leaves a run alone after it ends, since the run is still cleaning up after itself.
const SWEEP_GRACE: Duration = Duration::from_secs(10 * 60);

impl RunShepherd {
    /// Cleans up after the run once it's finished.
    pub(super) async fn clean_up(self) {
        let namespace_id = &self.pipeline.metadata.namespace_id;
        let pipeline_id = &self.pipeline.metadata.pipeline_id;
        let run_id = self.run.run_id;

        // Removing a task's container ends its log stream, so whatever hadn't been written yet would be lost.
        self.log_streams.close();
        if tokio::time::timeout(LOG_STREAM_WAIT, self.log_streams.wait())
            .await
            .is_err()
        {
            warn!(
                namespace_id,
                pipeline_id,
                run_id,
                "A task's logs were still being written when the run's containers were removed"
            );
        }

        if let Err(e) = self
            .api_state
            .containers
            .remove_run_folder(&self.run_folder())
            .await
        {
            error!(namespace_id, pipeline_id, run_id, error = %e, "Could not remove the run's folder");
        }

        if let Some(token_id) = &self.run.token_id
            && let Err(e) =
                delete_run_token(&self.api_state, namespace_id, pipeline_id, run_id, token_id).await
        {
            error!(
                namespace_id,
                pipeline_id,
                run_id,
                error = format!("{e:#}"),
                "Could not delete the run's API token"
            );
        }
    }
}

/// Deletes the API token a run's tasks were given (see `inject_api_token`), and the pipeline secret it was handed to
/// them through.
///
/// The token goes last. While it's still there the sweep tries again, so a cleanup that was cut short gets finished,
/// and the parts that are already gone are no trouble.
async fn delete_run_token(
    api_state: &ApiState,
    namespace_id: &str,
    pipeline_id: &str,
    run_id: u64,
    token_id: &str,
) -> Result<()> {
    let key_id = run_specific_api_key_id(run_id);

    api_state
        .secret_store
        .delete(&secrets::pipeline_secret_store_key(
            namespace_id,
            pipeline_id,
            &key_id,
        ))
        .await
        .context("Could not remove the token from the secret store")?;

    let mut conn = api_state
        .storage
        .write_conn()
        .await
        .context("Could not establish connection to database")?;

    storage::secret_store_pipeline_keys::delete(&mut conn, namespace_id, pipeline_id, &key_id)
        .await
        .context("Could not remove the token from the pipeline's secrets")?;

    storage::tokens::delete(&mut conn, token_id)
        .await
        .context("Could not remove the token")?;

    Ok(())
}

/// Cleans up after every run that's over, when Gofer starts and every hour after, for as long as Gofer runs.
pub async fn sweep_finished_runs(api_state: Arc<ApiState>) {
    let mut interval = tokio::time::interval(SWEEP_INTERVAL);

    loop {
        interval.tick().await;

        if let Err(e) = sweep(&api_state).await {
            error!(
                error = format!("{e:#}"),
                "Could not clean up after finished runs"
            );
        }
    }
}

async fn sweep(api_state: &ApiState) -> Result<()> {
    let ended_before = epoch_milli().saturating_sub(SWEEP_GRACE.as_millis() as u64);

    let folders = (api_state.containers.list_run_folders().await)
        .context("Could not list the run folders in Docker")?;

    let mut finished = vec![];
    let tokens = {
        let mut conn = api_state
            .storage
            .read_conn()
            .await
            .context("Could not establish connection to database")?;

        for folder in folders {
            let run = storage::runs::get(
                &mut conn,
                &folder.namespace_id,
                &folder.pipeline_id,
                folder.run_id.try_into()?,
            )
            .await;

            if is_over(run, ended_before)? {
                finished.push(folder);
            }
        }

        storage::runs::list_finished_with_tokens(&mut conn, ended_before)
            .await
            .context("Could not list finished runs that still have tokens")?
    };

    // One that can't be cleaned up shouldn't stop the rest. It'll be tried again next time.
    for folder in finished {
        if let Err(e) = api_state.containers.remove_run_folder(&folder).await {
            error!(namespace_id = folder.namespace_id, pipeline_id = folder.pipeline_id,
                run_id = folder.run_id, error = %e, "Could not remove a finished run's folder");
        }
    }

    for run in tokens {
        let result = delete_run_token(
            api_state,
            &run.namespace_id,
            &run.pipeline_id,
            run.run_id.try_into()?,
            &run.token_id,
        )
        .await;

        if let Err(e) = result {
            error!(
                namespace_id = run.namespace_id,
                pipeline_id = run.pipeline_id,
                run_id = run.run_id,
                error = format!("{e:#}"),
                "Could not delete a finished run's API token"
            );
        }
    }

    Ok(())
}

/// Whether the sweep should clean up after a run with a folder: one that ended before `ended_before`, or one that's
/// gone, because its pipeline was deleted.
fn is_over(run: Result<storage::runs::Run, StorageError>, ended_before: u64) -> Result<bool> {
    match run {
        Ok(run) => Ok(run.state == runs::State::Complete.to_string()
            && run
                .ended
                .parse::<u64>()
                .is_ok_and(|ended| ended < ended_before)),
        Err(StorageError::NotFound) => Ok(true),
        Err(e) => Err(e).context("Could not get a run"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(state: runs::State, ended: &str) -> Result<storage::runs::Run, StorageError> {
        Ok(storage::runs::Run {
            state: state.to_string(),
            ended: ended.into(),
            ..Default::default()
        })
    }

    #[test]
    fn test_only_runs_that_ended_a_while_ago_are_swept() {
        assert!(is_over(run(runs::State::Complete, "100"), 200).unwrap());
        assert!(is_over(Err(StorageError::NotFound), 200).unwrap());

        // It ended a moment ago, so it's still cleaning up after itself.
        assert!(!is_over(run(runs::State::Complete, "300"), 200).unwrap());
        assert!(!is_over(run(runs::State::Running, "0"), 200).unwrap());
        assert!(!is_over(run(runs::State::Pending, "0"), 200).unwrap());

        assert!(is_over(Err(StorageError::Connection("broken".into())), 200).is_err());
    }
}
