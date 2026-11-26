//! What `gofer up` prints about builds: a line for each task, with everything else in the task's log.
//!
//! ```text
//! ✓ cleanup  built in 1.2s
//! ⠋ report  cargo build --release  12s
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

/// Shows the step a task's build is on, next to the spinner.
pub fn step(task: &str, step: &str, running_for: Duration) {
    print!("{task}  {step}  {}s", running_for.as_secs(); vec![Format::Spinner]);
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
