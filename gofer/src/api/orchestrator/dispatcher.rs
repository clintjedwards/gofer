use super::RunShepherd;
use crate::{
    api::{ApiState, epoch_milli, event_utils, pipelines, runs},
    storage,
};

use anyhow::{Context, Result};
use sqlx::SqliteConnection;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::{Arc, atomic};
use tracing::error;

/// The most runs a pipeline can have going at once: its own `parallelism`, capped by the server's per-pipeline limit.
/// 0 means unlimited, both for the inputs and the result.
fn pipeline_run_limit(pipeline_parallelism: u64, server_pipeline_limit: u64) -> u64 {
    match (pipeline_parallelism, server_pipeline_limit) {
        (0, server) => server,
        (pipeline, 0) => pipeline,
        (pipeline, server) => pipeline.min(server),
    }
}

/// Picks which queued runs can start right now. `queued` must be oldest first; runs are taken in that order, so the
/// queue is first come first served across all of Gofer. A pipeline that's at its limit is skipped instead of holding
/// up runs of other pipelines behind it. `global_limit` of 0 means unlimited.
fn select_runs_to_start(
    queued: &[storage::runs::QueuedRun],
    running: &HashMap<(String, String), u64>,
    server_pipeline_limit: u64,
    global_limit: u64,
) -> Vec<storage::runs::QueuedRun> {
    let mut running = running.clone();
    let mut total_running: u64 = running.values().sum();
    let mut selected = vec![];

    for run in queued {
        if global_limit != 0 && total_running >= global_limit {
            break;
        }

        let limit = pipeline_run_limit(
            run.parallelism.try_into().unwrap_or_default(),
            server_pipeline_limit,
        );

        let pipeline_running = running
            .entry((run.namespace_id.clone(), run.pipeline_id.clone()))
            .or_default();

        if limit != 0 && *pipeline_running >= limit {
            continue;
        }

        *pipeline_running += 1;
        total_running += 1;
        selected.push(run.clone());
    }

    selected
}

/// Finishes a run that never started, as long as it's still pending. Returns false if the dispatcher started it in the
/// meantime, in which case it has to be cancelled like any other running run.
pub async fn complete_queued_run(
    api_state: &ApiState,
    conn: &mut SqliteConnection,
    namespace_id: &str,
    pipeline_id: &str,
    run_id: u64,
    status: runs::Status,
    reason: runs::StatusReason,
) -> Result<bool> {
    let reason = serde_json::to_string(&reason).context("Could not serialize run status reason")?;

    let completed = storage::runs::complete_pending(
        conn,
        namespace_id,
        pipeline_id,
        run_id.try_into()?,
        &epoch_milli().to_string(),
        &status.to_string(),
        &reason,
    )
    .await
    .context("Could not update run in database")?;

    if completed {
        api_state
            .event_bus
            .clone()
            .publish(event_utils::Kind::CompletedRun {
                namespace_id: namespace_id.into(),
                pipeline_id: pipeline_id.into(),
                run_id,
                status,
            });
    }

    Ok(completed)
}

/// Starts every queued run that has a free slot. Only the dispatcher calls this, one call at a time.
pub(super) async fn dispatch_queued_runs(api_state: &Arc<ApiState>) -> Result<()> {
    // Pausing run events holds queued runs too, so the switch can quiet Gofer down before maintenance.
    if api_state
        .ignore_pipeline_run_events
        .load(atomic::Ordering::SeqCst)
    {
        return Ok(());
    }

    let mut conn = api_state
        .storage
        .write_conn()
        .await
        .context("Could not open connection to database")?;

    let queued = storage::runs::list_queued(&mut conn)
        .await
        .context("Could not list queued runs")?;

    if queued.is_empty() {
        return Ok(());
    }

    let running: HashMap<(String, String), u64> = storage::runs::count_running(&mut conn)
        .await
        .context("Could not count running runs")?
        .into_iter()
        .map(|count| {
            (
                (count.namespace_id, count.pipeline_id),
                count.count.try_into().unwrap_or_default(),
            )
        })
        .collect();

    // A run waiting on a pipeline that has since been disabled would never start, so it's cancelled instead.
    let now = epoch_milli();
    let mut startable = vec![];

    for run in queued {
        let (pipeline_state, _) = pipelines::state_in_effect(
            pipelines::PipelineState::from_str(&run.pipeline_state).unwrap_or_default(),
            run.pipeline_disabled_until.parse().unwrap_or_default(),
            now,
        );

        if pipeline_state == pipelines::PipelineState::Active {
            startable.push(run);
            continue;
        }

        complete_queued_run(
            api_state,
            &mut conn,
            &run.namespace_id,
            &run.pipeline_id,
            run.run_id.try_into()?,
            runs::Status::Cancelled,
            runs::StatusReason {
                reason: runs::StatusReasonType::AdminCancelled,
                description: "Pipeline was disabled before the run started".into(),
            },
        )
        .await?;
    }

    let selected = select_runs_to_start(
        &startable,
        &running,
        api_state.config.api.pipeline_run_concurrency_limit,
        api_state.config.api.global_run_concurrency_limit,
    );

    for queued_run in selected {
        let run_id: u64 = queued_run.run_id.try_into()?;

        let stored_run = storage::runs::get(
            &mut conn,
            &queued_run.namespace_id,
            &queued_run.pipeline_id,
            queued_run.run_id,
        )
        .await
        .context("Could not get queued run from database")?;

        let run: runs::Run = stored_run.try_into()?;

        // A run whose pipeline can't be loaded would fail the same way every time, so it's failed instead of being
        // left in the queue.
        let mut shepherd = match RunShepherd::load(api_state.clone(), &mut conn, run).await {
            Ok(shepherd) => shepherd,
            Err(e) => {
                error!(namespace_id = queued_run.namespace_id, pipeline_id = queued_run.pipeline_id,
                    run_id, error = %e, "Could not load queued run");

                complete_queued_run(
                    api_state,
                    &mut conn,
                    &queued_run.namespace_id,
                    &queued_run.pipeline_id,
                    run_id,
                    runs::Status::Failed,
                    runs::StatusReason {
                        reason: runs::StatusReasonType::FailedPrecondition,
                        description: format!("Could not load the run's pipeline; {e}"),
                    },
                )
                .await?;

                continue;
            }
        };

        let started = epoch_milli();

        let claimed = storage::runs::claim(
            &mut conn,
            &queued_run.namespace_id,
            &queued_run.pipeline_id,
            queued_run.run_id,
            &started.to_string(),
        )
        .await
        .context("Could not claim queued run")?;

        // It was cancelled while it waited.
        if !claimed {
            continue;
        }

        shepherd.run.state = runs::State::Running;
        shepherd.run.started = started;

        tokio::spawn(shepherd.start_run());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn queued(pipeline_id: &str, run_id: i64, parallelism: i64) -> storage::runs::QueuedRun {
        storage::runs::QueuedRun {
            namespace_id: "default".into(),
            pipeline_id: pipeline_id.into(),
            run_id,
            parallelism,
            pipeline_state: "active".into(),
            pipeline_disabled_until: "0".into(),
        }
    }

    fn selected_ids(selected: &[storage::runs::QueuedRun]) -> Vec<(String, i64)> {
        selected
            .iter()
            .map(|run| (run.pipeline_id.clone(), run.run_id))
            .collect()
    }

    #[test]
    fn pipeline_run_limit_combines_pipeline_and_server_limits() {
        assert_eq!(pipeline_run_limit(0, 200), 200);
        assert_eq!(pipeline_run_limit(3, 0), 3);
        assert_eq!(pipeline_run_limit(3, 200), 3);
        assert_eq!(pipeline_run_limit(500, 200), 200);
        assert_eq!(pipeline_run_limit(0, 0), 0);
    }

    #[test]
    fn select_runs_takes_queue_in_order() {
        let queue = [queued("a", 1, 0), queued("b", 1, 0), queued("a", 2, 0)];

        let selected = select_runs_to_start(&queue, &HashMap::new(), 0, 2);

        assert_eq!(
            selected_ids(&selected),
            vec![("a".to_string(), 1), ("b".to_string(), 1)]
        );
    }

    #[test]
    fn select_runs_skips_pipeline_at_its_limit_without_blocking_others() {
        let queue = [queued("a", 2, 1), queued("a", 3, 1), queued("b", 1, 1)];
        let running = HashMap::from([(("default".to_string(), "a".to_string()), 1)]);

        let selected = select_runs_to_start(&queue, &running, 0, 0);

        assert_eq!(selected_ids(&selected), vec![("b".to_string(), 1)]);
    }

    #[test]
    fn select_runs_fills_free_slots_of_one_pipeline() {
        let queue = [queued("a", 1, 2), queued("a", 2, 2), queued("a", 3, 2)];

        let selected = select_runs_to_start(&queue, &HashMap::new(), 0, 0);

        assert_eq!(
            selected_ids(&selected),
            vec![("a".to_string(), 1), ("a".to_string(), 2)]
        );
    }

    #[test]
    fn select_runs_counts_running_runs_against_the_global_limit() {
        let queue = [queued("a", 1, 0), queued("b", 1, 0)];
        let running = HashMap::from([(("default".to_string(), "c".to_string()), 2)]);

        let selected = select_runs_to_start(&queue, &running, 0, 3);

        assert_eq!(selected_ids(&selected), vec![("a".to_string(), 1)]);
    }

    #[test]
    fn select_runs_uses_server_limit_when_pipeline_has_none() {
        let queue = [queued("a", 1, 0), queued("a", 2, 0)];

        let selected = select_runs_to_start(&queue, &HashMap::new(), 1, 0);

        assert_eq!(selected_ids(&selected), vec![("a".to_string(), 1)]);
    }

    #[test]
    fn select_runs_caps_pipeline_limit_at_server_limit() {
        let queue = [queued("a", 1, 5), queued("a", 2, 5)];

        let selected = select_runs_to_start(&queue, &HashMap::new(), 1, 0);

        assert_eq!(selected_ids(&selected), vec![("a".to_string(), 1)]);
    }
}
