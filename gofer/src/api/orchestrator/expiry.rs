use super::RunShepherd;
use crate::{
    api::{runs, task_executions},
    storage,
};

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::str::FromStr;
use tracing::{debug, error};

impl RunShepherd {
    /// Removes task execution logs once a run is past the log retention threshold.
    pub(super) async fn handle_run_log_expiry(self) {
        if let Err(e) = self.expire_run_logs().await {
            error!(namespace_id = &self.pipeline.metadata.namespace_id,
                pipeline_id = &self.pipeline.metadata.pipeline_id,
                run_id = self.run.run_id,
                error = %format!("{e:#}"), "Could not process run log expiry");
        }
    }

    /// Finds the run that just fell out of a retention window of `limit` runs and waits for it to finish, so cleanup
    /// never pulls things out from under a run that's still going. Returns None if the pipeline doesn't have enough
    /// runs for one to have expired.
    async fn wait_for_expired_run(&self, limit: u64) -> Result<Option<runs::Run>> {
        let mut conn = self
            .api_state
            .storage
            .read_conn()
            .await
            .context("Could not establish connection to database")?;

        let runs = storage::runs::list_newest_unskipped(
            &mut conn,
            &self.pipeline.metadata.namespace_id,
            &self.pipeline.metadata.pipeline_id,
            limit as i64 + 1,
        )
        .await
        .context("Could not retrieve runs")?;

        // Don't hold the connection while we wait.
        drop(conn);

        // if there aren't enough runs to reach the limit there is nothing to remove
        if limit > runs.len() as u64 {
            return Ok(None);
        }

        let Some(run) = runs.last() else {
            return Ok(None);
        };

        let mut expired_run: runs::Run = run
            .to_owned()
            .try_into()
            .context("Could not serialize run")?;

        let run_id = i64::try_from(expired_run.run_id).context("Could not serialize run id")?;

        while expired_run.state != runs::State::Complete {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

            let mut conn = self
                .api_state
                .storage
                .read_conn()
                .await
                .context("Could not establish connection to database")?;

            expired_run = storage::runs::get(
                &mut conn,
                &self.pipeline.metadata.namespace_id,
                &self.pipeline.metadata.pipeline_id,
                run_id,
            )
            .await
            .context("Could not get updated run state")?
            .try_into()
            .context("Could not serialize updated run")?;
        }

        Ok(Some(expired_run))
    }

    async fn expire_run_logs(&self) -> Result<()> {
        let limit = self.api_state.config.api.task_execution_log_retention_runs;

        let Some(expired_run) = self.wait_for_expired_run(limit).await? else {
            return Ok(());
        };

        self.remove_run_logs(&expired_run).await.with_context(|| {
            format!(
                "Could not remove logs for expired run {}",
                expired_run.run_id
            )
        })
    }

    async fn remove_run_logs(&self, expired_run: &runs::Run) -> Result<()> {
        let namespace_id = &self.pipeline.metadata.namespace_id;
        let pipeline_id = &self.pipeline.metadata.pipeline_id;
        let expired_run_id =
            i64::try_from(expired_run.run_id).context("Could not serialize run id")?;

        // If the task executions are in progress we wait for them to be finished also.
        // <task_id> => <logs_removed>
        let mut chopping_block_ids = HashMap::new();

        loop {
            let mut conn = self
                .api_state
                .storage
                .read_conn()
                .await
                .context("Could not establish connection to database")?;

            let task_executions_raw = storage::task_executions::list(
                &mut conn,
                namespace_id,
                pipeline_id,
                expired_run_id,
            )
            .await
            .context("Could not get task executions")?;

            // Don't hold database connection across loop.
            drop(conn);

            for execution in task_executions_raw.iter() {
                let execution_state = match task_executions::State::from_str(&execution.state) {
                    Ok(state) => state,
                    Err(e) => {
                        error!(namespace_id, pipeline_id, run_id = expired_run.run_id,
                            error = %e, storage_state = execution.state,
                            "Could not parse state while attempting to process run log expiry");
                        continue;
                    }
                };

                // If the task execution is complete we put it on the chopping block.
                if execution_state == task_executions::State::Complete {
                    chopping_block_ids.insert(execution.task_id.clone(), execution.logs_removed);
                }
            }

            if chopping_block_ids.len() == task_executions_raw.len() {
                break;
            }

            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }

        let mut removed_files = vec![];

        for (id, logs_removed) in chopping_block_ids {
            if logs_removed {
                continue;
            }

            let log_path = task_executions::task_execution_log_path(
                &self.api_state.config.api.task_execution_logs_dir,
                namespace_id,
                pipeline_id,
                expired_run.run_id,
                &id,
            );

            if let Err(e) = tokio::fs::remove_file(log_path.clone()).await {
                debug!(namespace_id, pipeline_id, run_id = expired_run.run_id,
                    error = %e, path = ?log_path, "Could not remove task execution log file");
            }

            removed_files.push(log_path.to_string_lossy().to_string());

            let mut conn = self
                .api_state
                .storage
                .write_conn()
                .await
                .context("Could not establish connection to database")?;

            // One task execution failing to update shouldn't keep the others' logs around.
            if let Err(e) = storage::task_executions::update(
                &mut conn,
                namespace_id,
                pipeline_id,
                expired_run_id,
                &id,
                storage::task_executions::UpdatableFields {
                    logs_expired: Some(true),
                    logs_removed: Some(true),
                    ..Default::default()
                },
            )
            .await
            {
                error!(namespace_id, pipeline_id, run_id = expired_run.run_id,
                    error = %e, task_id = id, "Could not update task execution while attempting to process run log expiry");
            };
        }

        debug!(namespace_id, pipeline_id, run_id = expired_run.run_id,
            removed_files = ?removed_files, "Removed task execution log files");

        Ok(())
    }
}
