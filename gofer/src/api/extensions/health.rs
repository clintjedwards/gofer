//! Keeps an eye on running extensions.
//!
//! Gofer checks each running extension's health endpoint on an interval. One that stops answering is marked degraded,
//! restarted if it keeps failing, and marked failed if restarting doesn't help. From there it's on an admin to fix it
//! and bring it back with `gofer extension reload`.
//!
//! Calls Gofer makes to an extension for other reasons, like handing it a subscription or an external event, count
//! too. That way an extension Gofer rarely talks to still gets noticed by the poller, and one Gofer talks to often
//! gets noticed right away.

use super::{Extension, State, new_extension_client, reconcile, stop_extension};
use crate::api::ApiState;
use std::{sync::Arc, time::Duration};
use tracing::{error, info, warn};

/// How long a single health check can take before it counts as a miss.
const HEALTH_CHECK_TIMEOUT: Duration = Duration::from_secs(10);

/// Checks every running extension on the configured interval. Runs for as long as Gofer does.
pub async fn poll(api_state: Arc<ApiState>) {
    let interval = Duration::from_secs(
        api_state
            .config
            .extensions
            .health_check_interval_seconds
            .max(1),
    );

    loop {
        tokio::time::sleep(interval).await;

        let up: Vec<Extension> = api_state
            .extensions
            .iter()
            .filter(|extension| extension.is_up())
            .map(|extension| extension.value().clone())
            .collect();

        // Each check gets its own task so one slow extension can't hold up the rest.
        for extension in up {
            let api_state = api_state.clone();
            tokio::spawn(async move {
                let result = check(&api_state, &extension).await;
                record(&api_state, &extension, result).await;
            });
        }
    }
}

/// Call when a request to an extension fails. If the extension couldn't be reached at all, as opposed to answering
/// with an error, it counts the same as a missed health check.
pub fn record_failed_call<E>(
    api_state: &Arc<ApiState>,
    extension: &Extension,
    error: &gofer_sdk::extension::api::Error<E>,
) {
    let gofer_sdk::extension::api::Error::CommunicationError(error) = error else {
        return;
    };

    let api_state = api_state.clone();
    let extension = extension.clone();
    let error = error.to_string();
    tokio::spawn(async move { record(&api_state, &extension, Err(error)).await });
}

async fn check(api_state: &ApiState, extension: &Extension) -> Result<(), String> {
    let client = new_extension_client(
        &extension.url,
        &extension.secret,
        api_state.config.extensions.verify_certs,
    )
    .map_err(|e| format!("{e:#}"))?;

    match tokio::time::timeout(HEALTH_CHECK_TIMEOUT, client.health()).await {
        Ok(Ok(_)) => Ok(()),
        Ok(Err(e)) => Err(e.to_string()),
        Err(_) => Err(format!(
            "no answer within {} seconds",
            HEALTH_CHECK_TIMEOUT.as_secs()
        )),
    }
}

/// Updates an extension's health with the result of one check. `checked` is the entry the check ran against; if
/// the extension has been restarted, reloaded, or stopped since, the result is about a container that's gone and is
/// dropped.
async fn record(api_state: &Arc<ApiState>, checked: &Extension, result: Result<(), String>) {
    // Held through any restart so a reload can't start the extension at the same time.
    let _guard = api_state.extension_lock.lock().await;

    let extension_id = &checked.registration.extension_id;
    let Some(mut current) = api_state
        .extensions
        .get(extension_id)
        .map(|extension| extension.value().clone())
    else {
        return;
    };

    if !current.is_up() || current.started != checked.started {
        return;
    }

    let error = match result {
        Ok(()) => {
            if current.state == State::Degraded {
                info!(
                    extension_id = extension_id,
                    "Extension is answering health checks again"
                );
            }

            current.state = State::Running;
            current.state_reason = String::new();
            current.missed_health_checks = 0;
            current.health_restarts = 0;
            api_state.set_extension(current);
            return;
        }
        Err(error) => error,
    };

    current.missed_health_checks += 1;
    warn!(
        extension_id = extension_id,
        missed = current.missed_health_checks,
        error = error,
        "Extension missed a health check"
    );

    let config = &api_state.config.extensions;
    if current.missed_health_checks < config.health_check_failures_before_action.max(1) {
        current.state = State::Degraded;
        current.state_reason = format!("Not answering health checks. Last error: {error}");
        api_state.set_extension(current);
        return;
    }

    let can_restart = config.health_check_auto_restart
        && current.health_restarts < config.health_check_max_restarts;

    if can_restart && let Some(prepared) = current.applied.clone() {
        warn!(
            extension_id = extension_id,
            restart = current.health_restarts + 1,
            "Extension keeps failing health checks; restarting it"
        );

        stop_extension(api_state, extension_id).await;
        let mut restarted =
            reconcile::run_prepared(api_state, *prepared, current.previous.clone()).await;
        restarted.health_restarts = current.health_restarts + 1;
        api_state.set_extension(restarted);
        return;
    }

    let reason = if current.health_restarts > 0 {
        format!(
            "Stopped answering health checks and restarting didn't help; fix it and run 'gofer extension reload'. \
            Last error: {error}"
        )
    } else {
        format!(
            "Stopped answering health checks; fix it and run 'gofer extension reload'. Last error: {error}"
        )
    };

    error!(
        extension_id = extension_id,
        reason = reason,
        "Marking extension failed"
    );

    // It might be hung rather than gone, so make sure the container is stopped before giving up on it.
    stop_extension(api_state, extension_id).await;
    current.state = State::Failed;
    current.state_reason = reason;
    api_state.set_extension(current);
}
