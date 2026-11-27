//! What `gofer up` prints about builds: a line for each task, with everything else in the task's log.
//!
//! ```text
//! ✓ cleanup  built in 1.2s
//! ⠋ report  cargo build --release  12s  ·  scan  RUN go build ./...  3s
//! x scan  Failed at: go build ./... (exit code 1)
//!       <its last 20 lines>
//!     Full log: ~/.cache/gofer/builds/default-nightly-scan-20261124T120000.log
//! ```

use super::{logs, progress::Failure, tips::Tip};
use polyfmt::{Format, get_global_formatter, print, println, success};
use serde::Serialize;
use std::{fmt, time::Duration};

/// The formats that are read by people. Tips and notes are left out of the others.
fn for_people() -> Vec<Format> {
    vec![Format::Spinner, Format::Plain, Format::Tree]
}

/// Shows the step each running build is on, next to the spinner, like `report  cargo build --release  12s`. With more
/// than one, they share the line.
pub fn building(running: &[(&str, &str, Duration)]) {
    let line: Vec<String> = (running.iter())
        .map(|(task, step, running_for)| format!("{task}  {step}  {}s", running_for.as_secs()))
        .collect();

    if !line.is_empty() {
        print!("{}", line.join("  ·  "); vec![Format::Spinner]);
    }
}

pub fn built(task: &str, took: Duration) {
    success!("{task}  built in {:.1}s", took.as_secs_f64());
}

/// Says why a task's build failed. When it was a step that failed, that includes its last lines and where the log is.
pub fn failed(task: &str, error: &anyhow::Error) {
    let failed = match error.downcast_ref::<Failure>() {
        Some(failure) => Failed {
            task,
            error: failure.message.clone(),
            output: failure.output.clone(),
            log: Some(logs::shorten(&failure.log)),
        },
        None => Failed {
            task,
            error: format!("{error:#}"),
            output: vec![],
            log: None,
        },
    };

    get_global_formatter().lock().unwrap().error(&failed);
}

pub fn tips(tips: &[Tip]) {
    for tip in tips {
        println!("Tip: {}", tip.message; for_people());
    }
}

/// For when Docker isn't on amd64, like on an ARM Mac. Images are always for amd64, so building them is emulated.
pub fn emulated() {
    println!("Building for linux/amd64. This uses emulation, so compiling can be slow."; for_people());
}

/// A size the way people say it, like `310MB` or `1.2GB`.
pub fn bytes(size: u64) -> String {
    const MB: f64 = 1000.0 * 1000.0;

    match size as f64 / MB {
        mb if mb >= 1000.0 => format!("{:.1}GB", mb / 1000.0),
        mb => format!("{mb:.0}MB"),
    }
}

/// A failed build, as one message for people and as fields for JSON.
#[derive(Serialize)]
struct Failed<'a> {
    task: &'a str,
    error: String,
    output: Vec<String>,
    log: Option<String>,
}

impl fmt::Display for Failed<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}  {}", self.task, self.error)?;
        for line in &self.output {
            write!(f, "\n    {line}")?;
        }
        if let Some(log) = &self.log {
            write!(f, "\nFull log: {log}")?;
        }
        Ok(())
    }
}
