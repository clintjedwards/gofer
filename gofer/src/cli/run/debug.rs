use crate::cli::{Cli, colorize_status_text, duration, epoch_milli, rail};
use anyhow::{Context, Result};
use colored::{ColoredString, Colorize};
use gofer_sdk::api::types::{
    RequiredParentStatus, Run, RunState, TaskExecution, TaskExecutionState, TaskExecutionStatus,
};
use polyfmt::println;
use std::collections::HashMap;

/// How many columns wide the timeline bars are.
const TIMELINE_WIDTH: usize = 30;

/// The width of the labels ("Reason", "Image", ...) in the "What went wrong" section, so their values line up.
const LABEL_WIDTH: usize = 8;

impl Cli {
    pub async fn run_debug(
        &self,
        namespace_id: Option<String>,
        pipeline_id: &str,
        run_id: u64,
        lines: usize,
    ) -> Result<()> {
        let namespace = match namespace_id {
            Some(namespace) => namespace,
            None => self.conf.namespace.clone(),
        };

        let run = self
            .client
            .get_run(&namespace, pipeline_id, run_id)
            .await
            .context("Could not successfully retrieve run from Gofer api")?
            .into_inner()
            .run;

        let mut task_executions = self
            .client
            .list_task_executions(&namespace, pipeline_id, run_id, None)
            .await
            .context("Could not successfully retrieve task executions from Gofer api")?
            .into_inner()
            .task_executions;

        sort_by_run_order(&mut task_executions);

        let mut output = vec![super::run_title(&run)];

        output.push(format!(
            "  {} Initiated by {}",
            rail(),
            run.initiator.user.blue()
        ));
        output.push(format!(
            "  {} Started {} and {}",
            rail(),
            self.format_time(run.started)
                .unwrap_or_else(|| "not yet".to_string()),
            run_duration(&run)
        ));
        if let Some(reason) = &run.status_reason {
            output.push(format!(
                "  {} {}: {}: {}",
                rail(),
                "Failure".red(),
                reason.reason,
                reason.description
            ));
        }

        output.push(String::new());
        output.push("  $ Timeline:".to_string());
        output.extend(timeline(&run, &task_executions));

        output.push(String::new());
        output.push("  $ What went wrong:".to_string());
        output.extend(self.problems(&namespace, &task_executions, lines).await);

        let mut hints = vec![];

        if let Some(failed) = task_executions.iter().find(|task| is_failure(task)) {
            hints.push(format!(
                "* Use '{}' to see the full output of {}.",
                format!(
                    "gofer task logs {} {} {}",
                    pipeline_id, run_id, failed.task_id
                )
                .cyan(),
                failed.task_id.blue()
            ));
        }

        if let Some(running) = task_executions
            .iter()
            .find(|task| task.state == TaskExecutionState::Running)
        {
            hints.push(format!(
                "* Use '{}' to open a shell in {} while it's running.",
                format!(
                    "gofer task attach {} {} {}",
                    pipeline_id, run_id, running.task_id
                )
                .cyan(),
                running.task_id.blue()
            ));
        }

        if !hints.is_empty() {
            output.push(String::new());
            output.extend(hints);
        }

        println!("{}", output.join("\n"));
        Ok(())
    }

    /// Details on each task that didn't succeed, failures first since those are usually the root cause.
    async fn problems(
        &self,
        namespace: &str,
        task_executions: &[TaskExecution],
        lines: usize,
    ) -> Vec<String> {
        let by_id: HashMap<&str, &TaskExecution> = task_executions
            .iter()
            .map(|task| (task.task_id.as_str(), task))
            .collect();

        let mut problems: Vec<&TaskExecution> = task_executions
            .iter()
            .filter(|task| is_problem(task))
            .collect();
        problems.sort_by_key(|task| problem_rank(task));

        if problems.is_empty() {
            let message = if task_executions
                .iter()
                .all(|task| task.state == TaskExecutionState::Complete)
            {
                "Nothing went wrong".green()
            } else {
                "Nothing has gone wrong yet".green()
            };
            return vec![format!("  {} {}", rail(), message)];
        }

        let mut output = vec![];

        for (i, task) in problems.into_iter().enumerate() {
            if i > 0 {
                output.push(format!("  {}", rail()));
            }

            match task.status {
                TaskExecutionStatus::Skipped => {
                    output.push(format!(
                        "  {} {} {} was skipped",
                        rail(),
                        "↷".dimmed(),
                        task.task_id.blue()
                    ));

                    let mut reasons = unmet_dependencies(task, &by_id);
                    if reasons.is_empty()
                        && let Some(reason) = &task.status_reason
                    {
                        reasons.push(reason.description.clone());
                    }
                    output.extend(labeled("Reason", &reasons));
                }
                TaskExecutionStatus::Cancelled => {
                    output.push(format!(
                        "  {} {} {} was cancelled",
                        rail(),
                        "⊘".dimmed(),
                        task.task_id.blue()
                    ));

                    if let Some(reason) = &task.status_reason {
                        output.extend(labeled("Reason", std::slice::from_ref(&reason.description)));
                    }
                }
                _ => {
                    let headline = match (&task.status, task.exit_code) {
                        (TaskExecutionStatus::Failed, Some(code)) => {
                            format!("failed with exit code {code}")
                        }
                        (TaskExecutionStatus::Failed, None) => "failed".to_string(),
                        _ => "ended in an unknown state".to_string(),
                    };

                    output.push(format!(
                        "  {} {} {} {}",
                        rail(),
                        "✗".red(),
                        task.task_id.blue(),
                        headline
                    ));

                    if let Some(reason) = &task.status_reason {
                        output.extend(labeled(
                            "Reason",
                            &[format!("{}: {}", reason.reason, reason.description)],
                        ));
                    }

                    output.extend(labeled("Image", std::slice::from_ref(&task.task.image)));
                    if !task.image_digest.is_empty() {
                        output.extend(labeled("Digest", std::slice::from_ref(&task.image_digest)));
                    }

                    let log_lines = if lines == 0 {
                        vec![]
                    } else if task.started == 0 {
                        vec![
                            "(task never started, so there is no output)"
                                .dimmed()
                                .to_string(),
                        ]
                    } else if task.logs_removed || task.logs_expired {
                        vec!["(logs have been removed)".dimmed().to_string()]
                    } else {
                        match self
                            .log_tail(
                                namespace,
                                &task.pipeline_id,
                                task.run_id,
                                &task.task_id,
                                lines,
                            )
                            .await
                        {
                            Ok(log_lines) if log_lines.is_empty() => {
                                vec!["(no output)".dimmed().to_string()]
                            }
                            Ok(log_lines) => log_lines,
                            Err(e) => {
                                vec![format!("(could not read logs; {e})").dimmed().to_string()]
                            }
                        }
                    };
                    output.extend(labeled("Output", &log_lines));
                }
            }
        }

        output
    }
}

/// "ran for 2 mins" once the run is done, "has been running for 2 mins" while it's still going.
pub(super) fn run_duration(run: &Run) -> String {
    let elapsed = duration(run.started as i64, run.ended as i64);
    if run.state == RunState::Complete {
        format!("ran for {elapsed}")
    } else {
        format!("has been running for {elapsed}")
    }
}

/// Failures first since those are usually the root cause, then cancellations, then skips.
pub(super) fn problem_rank(task: &TaskExecution) -> u8 {
    match task.status {
        TaskExecutionStatus::Failed | TaskExecutionStatus::Unknown => 0,
        TaskExecutionStatus::Cancelled => 1,
        _ => 2,
    }
}

/// Sorts tasks in the order they ran. Tasks that never started go last, ordered by how deep they sit in the
/// dependency chain so a skipped task always comes after the skipped task it was waiting on.
pub(super) fn sort_by_run_order(task_executions: &mut [TaskExecution]) {
    let deps: HashMap<&str, &HashMap<String, RequiredParentStatus>> = task_executions
        .iter()
        .map(|task| (task.task_id.as_str(), &task.task.depends_on))
        .collect();

    let depths: HashMap<String, usize> = task_executions
        .iter()
        .map(|task| {
            (
                task.task_id.clone(),
                dependency_depth(&task.task_id, &deps, deps.len()),
            )
        })
        .collect();

    task_executions.sort_by(|a, b| {
        (a.started == 0, a.started, depths[&a.task_id], &a.task_id).cmp(&(
            b.started == 0,
            b.started,
            depths[&b.task_id],
            &b.task_id,
        ))
    });
}

/// How many levels of dependencies sit above a task. `limit` keeps a malformed cycle from looping forever.
fn dependency_depth(
    task_id: &str,
    deps: &HashMap<&str, &HashMap<String, RequiredParentStatus>>,
    limit: usize,
) -> usize {
    if limit == 0 {
        return 0;
    }

    deps.get(task_id)
        .into_iter()
        .flat_map(|parents| parents.keys())
        .map(|parent| 1 + dependency_depth(parent, deps, limit - 1))
        .max()
        .unwrap_or(0)
}

fn is_failure(task: &TaskExecution) -> bool {
    task.state == TaskExecutionState::Complete
        && matches!(
            task.status,
            TaskExecutionStatus::Failed | TaskExecutionStatus::Unknown
        )
}

fn is_problem(task: &TaskExecution) -> bool {
    is_failure(task)
        || matches!(
            task.status,
            TaskExecutionStatus::Cancelled | TaskExecutionStatus::Skipped
        )
}

/// A task's status in a word or two, colored the same way as the rest of the CLI.
pub(super) fn status_text(task: &TaskExecution) -> ColoredString {
    if task.state != TaskExecutionState::Complete {
        return task.state.to_string().to_lowercase().yellow();
    }

    match (&task.status, task.exit_code) {
        (TaskExecutionStatus::Failed, Some(code)) => format!("failed (exit {code})").red(),
        (TaskExecutionStatus::Skipped, _) => "skipped".dimmed(),
        (status, _) => colorize_status_text(status).normal(),
    }
}

/// Renders a value under a label, with any extra lines lined up under the first one:
///
///   │   Output  first line
///   │           second line
fn labeled(label: &str, values: &[String]) -> Vec<String> {
    values
        .iter()
        .enumerate()
        .map(|(i, value)| {
            let label = if i == 0 { label } else { "" };
            format!(
                "  {}   {}{}",
                rail(),
                format!("{label:<LABEL_WIDTH$}").dimmed(),
                value
            )
        })
        .collect()
}

/// One line per task showing when it waited and ran relative to the rest of the run. The columns are padded before
/// they're colored so the bars, arrows, and statuses line up.
fn timeline(run: &Run, task_executions: &[TaskExecution]) -> Vec<String> {
    if task_executions.is_empty() {
        return vec![format!("  {} None", rail())];
    }

    let now = epoch_milli();
    let run_start = if run.started != 0 {
        run.started
    } else {
        task_executions
            .iter()
            .map(|task| task.created)
            .min()
            .unwrap_or(now)
    };
    let run_end = if run.ended != 0 { run.ended } else { now };
    let span = run_end.saturating_sub(run_start).max(1);

    let column = |time: u64| -> usize {
        let offset = time.saturating_sub(run_start) as u128;
        (offset * TIMELINE_WIDTH as u128 / span as u128).min(TIMELINE_WIDTH as u128) as usize
    };

    let times: Vec<Option<(String, String)>> = task_executions
        .iter()
        .map(|task| {
            if task.started == 0 {
                return None;
            }

            let end = if task.ended != 0 {
                offset(task.ended.saturating_sub(run_start))
            } else {
                "now".to_string()
            };

            Some((offset(task.started.saturating_sub(run_start)), end))
        })
        .collect();

    let task_width = task_executions
        .iter()
        .map(|task| task.task_id.chars().count())
        .max()
        .unwrap_or(0);
    let start_width = times
        .iter()
        .flatten()
        .map(|(start, _)| start.len())
        .max()
        .unwrap_or(0);
    let end_width = times
        .iter()
        .flatten()
        .map(|(_, end)| end.len())
        .max()
        .unwrap_or(0);

    const NOT_STARTED: &str = "not started";
    let times_width = (start_width + 3 + end_width).max(NOT_STARTED.len());

    let mut output = vec![];

    for (task, times) in task_executions.iter().zip(times) {
        let bar = match times {
            None => " ".repeat(TIMELINE_WIDTH),
            Some(_) => {
                let run_from = column(task.started).min(TIMELINE_WIDTH - 1);
                let wait_from = column(task.created.max(run_start)).min(run_from);
                let end = if task.ended != 0 { task.ended } else { now };
                let run_to = column(end).clamp(run_from + 1, TIMELINE_WIDTH);

                let running = "█".repeat(run_to - run_from);
                let running = match (&task.state, &task.status) {
                    (TaskExecutionState::Complete, TaskExecutionStatus::Successful) => {
                        running.green()
                    }
                    (TaskExecutionState::Complete, TaskExecutionStatus::Failed) => running.red(),
                    (TaskExecutionState::Complete, _) => running.dimmed(),
                    _ => running.yellow(),
                };

                format!(
                    "{}{}{}{}",
                    " ".repeat(wait_from),
                    "░".repeat(run_from - wait_from).dimmed(),
                    running,
                    " ".repeat(TIMELINE_WIDTH - run_to)
                )
            }
        };

        let times = match times {
            Some((start, end)) => {
                format!("{start:>start_width$} → {end:<end_width$}")
            }
            None => NOT_STARTED.to_string(),
        };

        output.push(format!(
            "  {} {}  {}  {}  {}",
            rail(),
            format!("{:<task_width$}", task.task_id).blue(),
            bar,
            format!("{times:<times_width$}").dimmed(),
            status_text(task)
        ));
    }

    output.push(format!(
        "  {} {}  {}",
        rail(),
        " ".repeat(task_width),
        format!("░ waiting  {} running", "█").dimmed()
    ));

    output
}

/// A short offset from the start of the run, like "48s", "1m52s", or "1h05m".
fn offset(millis: u64) -> String {
    let secs = millis / 1000;
    match secs {
        0..60 => format!("{secs}s"),
        60..3600 => format!("{}m{:02}s", secs / 60, secs % 60),
        _ => format!("{}h{:02}m", secs / 3600, (secs % 3600) / 60),
    }
}

/// Explains which of a skipped task's dependencies weren't met. When the parent was itself skipped, we follow the
/// chain up to the task that actually caused it so the user doesn't have to.
fn unmet_dependencies(task: &TaskExecution, by_id: &HashMap<&str, &TaskExecution>) -> Vec<String> {
    let mut parents: Vec<_> = task.task.depends_on.iter().collect();
    parents.sort_by_key(|(parent_id, _)| parent_id.as_str());

    let mut reasons = vec![];

    for (parent_id, required) in parents {
        let Some(parent) = by_id.get(parent_id.as_str()) else {
            continue;
        };

        if dependency_met(parent, required) {
            continue;
        }

        let needs = match required {
            RequiredParentStatus::Any => "to finish",
            RequiredParentStatus::Success => "to succeed",
            RequiredParentStatus::Failure => "to fail",
            RequiredParentStatus::Unknown => continue,
        };

        let mut reason = format!(
            "needs {} {needs}, but it {}",
            parent_id.blue(),
            outcome(parent)
        );

        if parent.status == TaskExecutionStatus::Skipped
            && let Some(root) = root_cause(parent, by_id, by_id.len())
        {
            reason.push_str(&format!(
                " because {} {}",
                root.task_id.blue(),
                outcome(root)
            ));
        }

        reasons.push(reason);
    }

    reasons
}

fn dependency_met(parent: &TaskExecution, required: &RequiredParentStatus) -> bool {
    match required {
        RequiredParentStatus::Any => parent.state == TaskExecutionState::Complete,
        RequiredParentStatus::Success => parent.status == TaskExecutionStatus::Successful,
        RequiredParentStatus::Failure => parent.status == TaskExecutionStatus::Failed,
        RequiredParentStatus::Unknown => true,
    }
}

/// Walks up from a skipped task to the first unmet dependency that wasn't itself skipped. `depth` keeps a
/// malformed dependency cycle from looping forever.
fn root_cause<'a>(
    task: &TaskExecution,
    by_id: &HashMap<&str, &'a TaskExecution>,
    depth: usize,
) -> Option<&'a TaskExecution> {
    if depth == 0 {
        return None;
    }

    for (parent_id, required) in &task.task.depends_on {
        let Some(parent) = by_id.get(parent_id.as_str()) else {
            continue;
        };

        if dependency_met(parent, required) {
            continue;
        }

        if parent.status == TaskExecutionStatus::Skipped {
            return root_cause(parent, by_id, depth - 1);
        }

        return Some(parent);
    }

    None
}

fn outcome(task: &TaskExecution) -> &'static str {
    match task.status {
        TaskExecutionStatus::Successful => "succeeded",
        TaskExecutionStatus::Failed => "failed",
        TaskExecutionStatus::Cancelled => "was cancelled",
        TaskExecutionStatus::Skipped => "was skipped",
        TaskExecutionStatus::Unknown => "ended in an unknown state",
    }
}
