//! Follows a build as BuildKit reports it. The status line, the build's log and the explanation of a failed build all
//! come from here.

use super::{Build, Step, logs};
use base64::{Engine, engine::general_purpose::STANDARD};
use chrono::{DateTime, Utc};
use lazy_regex::regex;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, VecDeque},
    fmt,
    io::Write,
    path::PathBuf,
};

/// How many of a failed step's last lines are shown. The rest are in the log.
const SHOWN_LINES: usize = 20;

/// Longer lines are cut short when they're shown, since they're usually something like minified code.
const SHOWN_LINE_CHARS: usize = 500;

/// A run step that fails because its image has no `/bin/sh` only says that it exited with 1. What happened is in what
/// it printed, from runc.
const NO_SHELL: &str = r#"exec: "/bin/sh": stat /bin/sh: no such file or directory"#;

/// How a run step starts in the Dockerfile we make, before the user's command.
const RUN_PREFIX: &str = r#"["/bin/sh","/gofer-run/run.sh","#;

/// One update from `buildctl build --progress=rawjson`. It only has what changed, so a step shows up in many of them.
///
/// <https://github.com/moby/buildkit/blob/v0.35.1/client/graph.go>
#[derive(Deserialize)]
struct Update {
    #[serde(default)]
    vertexes: Vec<VertexUpdate>,

    #[serde(default)]
    logs: Vec<LogUpdate>,
}

/// BuildKit calls a step a vertex.
#[derive(Deserialize)]
struct VertexUpdate {
    digest: String,

    #[serde(default)]
    name: String,

    started: Option<String>,

    completed: Option<String>,

    #[serde(default)]
    cached: bool,

    #[serde(default)]
    error: String,
}

#[derive(Deserialize)]
struct LogUpdate {
    vertex: String,

    /// 1 is stdout and 2 is stderr.
    #[serde(default)]
    stream: u8,

    /// Base64, since it can be any bytes.
    #[serde(default)]
    data: String,

    timestamp: Option<String>,
}

#[derive(Default)]
struct Vertex {
    /// Steps are numbered in the log in the order they start, like `docker build` does.
    number: usize,

    name: String,

    /// What the user calls this step, or nothing for BuildKit's own bookkeeping.
    label: Option<String>,

    started: Option<DateTime<Utc>>,

    completed: bool,

    error: Option<String>,

    /// The last lines it printed, oldest first.
    lines: VecDeque<String>,

    /// What it printed after its last newline, on stdout and on stderr.
    partial: [Vec<u8>; 2],

    /// A step can start and finish more than once, like when it downloads layers. The log only says so the first time.
    logged_start: bool,
    logged_end: bool,
}

pub struct Progress<'a, W> {
    build: &'a Build,

    /// Everything BuildKit says goes here, as text. Writing it is best effort; a build shouldn't fail because its log
    /// couldn't be written.
    log: W,

    vertexes: HashMap<String, Vertex>,

    started: usize,

    /// The Cargo version that run.sh said was too old to cache with.
    old_cargo: Option<String>,
}

impl<'a, W: Write> Progress<'a, W> {
    pub fn new(build: &'a Build, log: W) -> Self {
        Self {
            build,
            log,
            vertexes: HashMap::new(),
            started: 0,
            old_cargo: None,
        }
    }

    /// Takes a line of BuildKit's progress. Anything that isn't progress goes into the log as it is.
    pub fn update(&mut self, line: &str) {
        let Ok(update) = serde_json::from_str::<Update>(line) else {
            self.write(line.trim_end());
            return;
        };

        for vertex in update.vertexes {
            self.update_vertex(vertex);
        }

        for log in update.logs {
            self.add_output(log);
        }
    }

    fn update_vertex(&mut self, update: VertexUpdate) {
        let build = self.build;
        let vertex = self.vertexes.entry(update.digest).or_default();

        if vertex.name.is_empty() && !update.name.is_empty() {
            vertex.label = label(&update.name, build);
            vertex.name = update.name;
        }

        let mut lines = vec![];

        if let Some(started) = update.started.as_deref().and_then(time) {
            vertex.started = Some(started);
            vertex.completed = false;

            if !vertex.logged_start {
                vertex.logged_start = true;
                self.started += 1;
                vertex.number = self.started;
                lines.push(vertex.label.clone().unwrap_or(vertex.name.clone()));
            }
        }

        if !update.error.is_empty() && vertex.error.is_none() {
            lines.extend(finish_lines(vertex));
            lines.push(format!("ERROR {}", update.error));
            vertex.error = Some(update.error);
            vertex.logged_end = true;
        }

        if let Some(completed) = update.completed.as_deref().and_then(time) {
            vertex.completed = true;

            if !vertex.logged_end {
                vertex.logged_end = true;
                lines.extend(finish_lines(vertex));

                match (update.cached, vertex.started) {
                    (true, _) => lines.push("CACHED".into()),
                    (false, Some(started)) => lines.push(format!(
                        "DONE {:.1}s",
                        (completed - started).as_seconds_f64()
                    )),
                    (false, None) => lines.push("DONE".into()),
                }
            }
        }

        let number = vertex.number;
        for line in lines {
            self.write(&format!("#{number} {line}"));
        }
    }

    fn add_output(&mut self, update: LogUpdate) {
        let Ok(data) = STANDARD.decode(&update.data) else {
            return;
        };

        let vertex = self.vertexes.entry(update.vertex).or_default();
        let since_start = match (vertex.started, update.timestamp.as_deref().and_then(time)) {
            (Some(started), Some(time)) => format!("{:.3} ", (time - started).as_seconds_f64()),
            _ => String::new(),
        };

        let partial = &mut vertex.partial[usize::from(update.stream == 2)];
        partial.extend_from_slice(&data);

        let mut lines = vec![];
        while let Some(end) = partial.iter().position(|byte| *byte == b'\n') {
            let line: Vec<u8> = partial.drain(..=end).collect();
            lines.push(String::from_utf8_lossy(&line[..end]).into_owned());
        }

        for line in &lines {
            keep_line(vertex, line);
        }
        let number = vertex.number;

        for line in lines {
            if let Some(version) = regex!(r"^gofer: Cargo (1\.\d+) can't keep").captures(&line) {
                self.old_cargo = Some(version[1].to_string());
            }

            self.write(&format!("#{number} {since_start}{line}"));
        }
    }

    fn write(&mut self, line: &str) {
        _ = writeln!(self.log, "{line}");
    }

    /// Adds a line of our own to the log.
    pub fn note(&mut self, line: &str) {
        self.write(line);
    }

    /// The step that's running now. When more than one is, it's the one that started last.
    pub fn step(&self) -> Option<&str> {
        self.vertexes
            .values()
            .filter(|vertex| !vertex.completed && vertex.error.is_none())
            .filter_map(|vertex| Some((vertex.started?, vertex.label.as_deref()?)))
            .max_by_key(|(started, _)| *started)
            .map(|(_, label)| label)
    }

    /// The Cargo version that was too old for the Rust cache, if a step had one.
    pub fn old_cargo(&self) -> Option<&str> {
        self.old_cargo.as_deref()
    }

    /// Explains why the build failed, from the step that failed first. `None` if no step did, which means it was
    /// something like a Dockerfile that couldn't be read, and BuildKit's own message says that best.
    pub fn failure(&self, log: PathBuf) -> Option<Failure> {
        // When one step fails BuildKit stops the others, and those say they were canceled.
        let failed = self
            .vertexes
            .values()
            .filter(|vertex| vertex.error.is_some())
            .min_by_key(|vertex| {
                let error = vertex.error.as_deref().unwrap_or_default();
                (error.contains("canceled"), vertex.number)
            })?;

        let error = failed.error.as_deref().unwrap_or_default();
        let base = match self.build {
            Build::Steps(steps) => Some(steps.base.as_str()),
            Build::Dockerfile { .. } => None,
        };

        let mut output = vec![];
        let message = if error.contains("no space left on device") {
            "The builder ran out of disk space. `gofer builder prune` empties its cache, and a lower \
             builder_cache_max_gb keeps it smaller."
                .to_string()
        } else if let Some(image) = failed.name.strip_prefix("[internal] load metadata for ") {
            base_image_error(&short_image(image), error)
        } else if failed.lines.iter().any(|line| line.contains(NO_SHELL)) {
            let image = match base {
                Some(base) => format!("The base image, {base},"),
                None => "The image this step runs in".into(),
            };
            format!(
                "{image} has no /bin/sh, which run steps need. Use a base image that has a shell. If the image \
                 you run has to be small, keep() can move what you built onto one without a shell."
            )
        } else {
            output = failed.lines.iter().cloned().collect();
            let step = failed.label.as_deref().unwrap_or(&failed.name);

            let code = regex!(r"did not complete successfully: exit code: (\d+)$").captures(error);
            let missing = regex!(r#""([^"]+)": not found$"#).captures(error);

            match (code, missing) {
                (Some(code), _) => format!("Failed at: {step} (exit code {})", &code[1]),
                // A COPY in a Dockerfile, of something that isn't there.
                (None, Some(path)) => format!(
                    "Failed at: {step}; {} isn't in the build context, or its .dockerignore leaves it out",
                    &path[1]
                ),
                (None, None) => format!("Failed at: {step}; {error}"),
            }
        };

        Some(Failure {
            message,
            output,
            log,
        })
    }
}

/// Writes out what a step printed without a newline at the end, before saying it finished.
fn finish_lines(vertex: &mut Vertex) -> Vec<String> {
    let mut lines = vec![];

    for partial in &mut vertex.partial {
        if !partial.is_empty() {
            let line = String::from_utf8_lossy(partial).into_owned();
            partial.clear();
            lines.push(line);
        }
    }

    for line in &lines {
        keep_line(vertex, line);
    }

    lines
}

fn keep_line(vertex: &mut Vertex, line: &str) {
    // A progress bar redraws its line with carriage returns. A terminal only shows what came after the last one.
    let shown = line.rsplit('\r').next().unwrap_or_default();
    let mut kept: String = shown.chars().take(SHOWN_LINE_CHARS).collect();
    if kept.len() < shown.len() {
        kept.push_str("...");
    }

    vertex.lines.push_back(kept);
    if vertex.lines.len() > SHOWN_LINES {
        vertex.lines.pop_front();
    }
}

fn time(text: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(text)
        .ok()
        .map(|time| time.to_utc())
}

/// What a step is called in the output. BuildKit names a step after its line in the Dockerfile. For steps that came from
/// `copy` and `run` that's a line we made, so we put back what the user wrote. BuildKit's own bookkeeping gets no name,
/// since it's quick and means nothing to the user.
fn label(name: &str, build: &Build) -> Option<String> {
    if let Some(image) = name.strip_prefix("[internal] load metadata for ") {
        return Some(format!("look up {}", short_image(image)));
    }

    if name.starts_with("[internal]") {
        return None;
    }

    if name == "exporting to docker image format" {
        return Some("save the image".into());
    }

    // `[stage-0 2/3] RUN ...`, or `[2/3] RUN ...` when there's only one stage.
    let line = regex!(r"^\[[^\]]*\d+/\d+\] ").replace(name, "");

    let Build::Steps(steps) = build else {
        return Some(match line.strip_prefix("FROM ") {
            Some(image) => format!("FROM {}", short_image(image)),
            None => line.into_owned(),
        });
    };

    if let Some(image) = line.strip_prefix("FROM ") {
        return Some(format!("download {}", short_image(image)));
    }

    if let Some(start) = line.find(RUN_PREFIX)
        && let Ok([_, _, command]) = serde_json::from_str::<[String; 3]>(&line[start..])
    {
        let mut lines = command.trim().lines();
        let first = lines.next().unwrap_or_default().to_string();
        return Some(match lines.next() {
            Some(_) => format!("{first} ..."),
            None => first,
        });
    }

    if let Some(copy) = regex!(r"^COPY \[copies/(\d+)/").captures(&line) {
        let index: usize = copy[1].parse().ok()?;
        let (from, to) = steps
            .steps
            .iter()
            .filter_map(|step| match step {
                Step::Copy { from, to } => Some((from, to)),
                Step::Run { .. } => None,
            })
            .nth(index)?;
        return Some(format!("copy {from} to {to}"));
    }

    if line.starts_with("COPY --from=0 ")
        && let Some(keep) = &steps.keep
    {
        return Some(format!("move what's kept onto {}", keep.base));
    }

    Some(line.into_owned())
}

/// Turns `docker.io/library/rust:1@sha256:...` back into `rust:1`, which is how people write it.
fn short_image(image: &str) -> String {
    let image = image.split('@').next().unwrap_or(image);
    let image = image.strip_prefix("docker.io/").unwrap_or(image);
    image.strip_prefix("library/").unwrap_or(image).to_string()
}

/// BuildKit looks up each base image before it starts, so this is where a wrong name, a private image or no network
/// shows up.
fn base_image_error(image: &str, error: &str) -> String {
    const NO_LOGIN: [&str; 4] = [
        "pull access denied",
        "authorization failed",
        "unauthorized",
        "authentication required",
    ];
    const NO_NETWORK: [&str; 6] = [
        "dial tcp",
        "no such host",
        "i/o timeout",
        "connection refused",
        "network is unreachable",
        "TLS handshake timeout",
    ];

    if error.ends_with(": not found") {
        format!("The base image {image} doesn't exist. Check its name and tag.")
    } else if NO_LOGIN.iter().any(|text| error.contains(text)) {
        format!("The base image {image} doesn't exist, or it needs a login to download.")
    } else if NO_NETWORK.iter().any(|text| error.contains(text)) {
        let registry = match image.split_once('/') {
            Some((host, _)) if host.contains(['.', ':']) || host == "localhost" => host,
            _ => "Docker Hub",
        };
        format!(
            "Could not reach {registry} to look up the base image {image}. Builds look up their base images \
             every time, so they need the network even when everything is cached.\n{error}"
        )
    } else {
        format!("Could not look up the base image {image}; {error}")
    }
}

/// Why a build failed, and what it printed before it did.
#[derive(Debug, Serialize)]
pub struct Failure {
    pub message: String,

    /// The last lines the failed step printed, oldest first.
    pub output: Vec<String>,

    /// Where everything BuildKit said is.
    pub log: PathBuf,
}

impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}", self.message)?;
        for line in &self.output {
            writeln!(f, "    {line}")?;
        }
        write!(f, "Full log: {}", logs::shorten(&self.log))
    }
}

impl std::error::Error for Failure {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::builder::{Keep, Steps};
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    const RUN: &str = r#"[stage-0 3/3] RUN --mount=type=bind,source=runs/0,target=/gofer-run --mount=type=cache,id=gofer-shared,target=/gofer-cache/shared,mode=1777 ["/bin/sh","/gofer-run/run.sh","cargo build --release\ncp target/release/app /app"]"#;

    fn steps() -> Build {
        Build::Steps(Steps {
            base: "rust:1".into(),
            steps: vec![
                Step::Copy {
                    from: "./app".into(),
                    to: "/src".into(),
                },
                Step::Run {
                    command: "cargo build --release\ncp target/release/app /app".into(),
                },
            ],
            keep: Some(Keep {
                paths: vec!["/app".into()],
                base: "debian:13-slim".into(),
            }),
        })
    }

    fn dockerfile() -> Build {
        Build::Dockerfile {
            path: "Dockerfile".into(),
            context: ".".into(),
        }
    }

    /// The shape of what BuildKit prints, taken from real builds.
    fn vertex(digest: &str, name: &str, state: &str, error: &str) -> String {
        let mut vertex = serde_json::json!({"digest": digest, "name": name});
        if state.contains("started") {
            vertex["started"] = "2026-11-26T02:23:17.000000000Z".into();
        }
        if state.contains("completed") {
            vertex["completed"] = "2026-11-26T02:23:18.500000000Z".into();
        }
        if state.contains("cached") {
            vertex["cached"] = true.into();
        }
        if !error.is_empty() {
            vertex["error"] = error.into();
        }
        serde_json::json!({ "vertexes": [vertex] }).to_string()
    }

    fn output(digest: &str, stream: u8, text: &str) -> String {
        serde_json::json!({"logs": [{
            "vertex": digest,
            "stream": stream,
            "data": STANDARD.encode(text),
            "timestamp": "2026-11-26T02:23:17.250000000Z",
        }]})
        .to_string()
    }

    fn feed<'a>(build: &'a Build, lines: &[String]) -> Progress<'a, Vec<u8>> {
        let mut progress = Progress::new(build, vec![]);
        for line in lines {
            progress.update(line);
        }
        progress
    }

    fn log_of(progress: Progress<'_, Vec<u8>>) -> String {
        String::from_utf8(progress.log).unwrap()
    }

    #[rstest]
    #[case::run_step(RUN, Some("cargo build --release ..."))]
    #[case::copy_step("[stage-0 2/3] COPY [copies/0/,/src]", Some("copy ./app to /src"))]
    #[case::keep(
        "[stage-1 2/2] COPY --from=0 [/app,/app]",
        Some("move what's kept onto debian:13-slim")
    )]
    #[case::base(
        "[stage-0 1/3] FROM docker.io/library/rust:1@sha256:8d0e82abfdf9a38071e07cabc5afba4ddbe077631a80e4e742a7",
        Some("download rust:1")
    )]
    #[case::look_up(
        "[internal] load metadata for ghcr.io/org/tool:2",
        Some("look up ghcr.io/org/tool:2")
    )]
    #[case::bookkeeping("[internal] load build context", None)]
    #[case::export("exporting to docker image format", Some("save the image"))]
    fn test_steps_are_called_what_the_user_wrote(
        #[case] name: &str,
        #[case] expected: Option<&str>,
    ) {
        assert_eq!(label(name, &steps()).as_deref(), expected);
    }

    #[test]
    fn test_dockerfile_steps_keep_their_lines() {
        assert_eq!(
            label("[2/3] RUN go build ./...", &dockerfile()).as_deref(),
            Some("RUN go build ./...")
        );
        assert_eq!(
            label(
                "[1/3] FROM docker.io/library/golang:1.25@sha256:abc",
                &dockerfile()
            )
            .as_deref(),
            Some("FROM golang:1.25")
        );
    }

    #[test]
    fn test_log_reads_like_docker_build() {
        let build = steps();
        let progress = feed(
            &build,
            &[
                vertex("sha256:a", "[internal] load build context", "started", ""),
                vertex(
                    "sha256:a",
                    "[internal] load build context",
                    "started completed",
                    "",
                ),
                vertex(
                    "sha256:b",
                    "[stage-0 2/3] COPY [copies/0/,/src]",
                    "started completed cached",
                    "",
                ),
                vertex("sha256:c", RUN, "started", ""),
                output("sha256:c", 2, "   Compiling app v0.1.0\n    Finish"),
                output("sha256:c", 2, "ed `release`"),
                vertex("sha256:c", RUN, "started completed", ""),
                "error: something BuildKit said".into(),
            ],
        );

        assert_eq!(
            log_of(progress),
            "#1 [internal] load build context\n\
             #1 DONE 1.5s\n\
             #2 copy ./app to /src\n\
             #2 CACHED\n\
             #3 cargo build --release ...\n\
             #3 0.250    Compiling app v0.1.0\n\
             #3     Finished `release`\n\
             #3 DONE 1.5s\n\
             error: something BuildKit said\n"
        );
    }

    #[test]
    fn test_step_is_the_one_running() {
        let build = steps();
        let mut progress = feed(
            &build,
            &[vertex(
                "sha256:a",
                "[internal] load build context",
                "started",
                "",
            )],
        );
        assert_eq!(progress.step(), None);

        progress.update(&vertex("sha256:c", RUN, "started", ""));
        assert_eq!(progress.step(), Some("cargo build --release ..."));

        progress.update(&vertex("sha256:c", RUN, "started completed", ""));
        assert_eq!(progress.step(), None);
    }

    #[test]
    fn test_failed_step_shows_its_last_lines() {
        let build = steps();
        let mut lines = vec![vertex("sha256:c", RUN, "started", "")];
        for number in 1..=25 {
            lines.push(output("sha256:c", 1, &format!("line {number}\n")));
        }
        lines.push(vertex(
            "sha256:c",
            RUN,
            "started completed",
            r#"process "/bin/sh /gofer-run/run.sh cargo build --release" did not complete successfully: exit code: 101"#,
        ));
        // Another step that BuildKit stopped because this one failed.
        lines.push(vertex(
            "sha256:d",
            "[stage-1 1/2] FROM docker.io/library/debian:13-slim",
            "started completed",
            "context canceled",
        ));

        let failure = feed(&build, &lines)
            .failure("/logs/build.log".into())
            .unwrap();
        assert_eq!(
            failure.message,
            "Failed at: cargo build --release ... (exit code 101)"
        );
        assert_eq!(failure.output.first().map(String::as_str), Some("line 6"));
        assert_eq!(failure.output.last().map(String::as_str), Some("line 25"));
        assert_eq!(failure.output.len(), SHOWN_LINES);
    }

    #[rstest]
    #[case::no_shell(
        RUN,
        "runc run failed: unable to start container process: error during container init: exec: \"/bin/sh\": stat /bin/sh: no such file or directory\n",
        "process \"/bin/sh /gofer-run/run.sh echo hi\" did not complete successfully: exit code: 1",
        "The base image, rust:1, has no /bin/sh"
    )]
    #[case::not_found(
        "[internal] load metadata for docker.io/library/alpine:doesnotexist-1",
        "",
        "docker.io/library/alpine:doesnotexist-1: not found",
        "The base image alpine:doesnotexist-1 doesn't exist. Check its name and tag."
    )]
    #[case::private(
        "[internal] load metadata for docker.io/someone/private:1",
        "",
        "pull access denied, repository does not exist or may require authorization: server message: insufficient_scope: authorization failed",
        "The base image someone/private:1 doesn't exist, or it needs a login to download."
    )]
    #[case::no_network(
        "[internal] load metadata for registry.example.invalid/thing:1",
        "",
        "failed to do request: Head \"https://registry.example.invalid/v2/thing/manifests/1\": dial tcp: lookup registry.example.invalid on 10.0.1.3:53: no such host",
        "Could not reach registry.example.invalid to look up the base image registry.example.invalid/thing:1."
    )]
    #[case::no_network_hub(
        "[internal] load metadata for docker.io/library/alpine:3",
        "",
        "failed to do request: Head \"https://registry-1.docker.io/v2/library/alpine/manifests/3\": dial tcp: i/o timeout",
        "Could not reach Docker Hub to look up the base image alpine:3."
    )]
    #[case::missing_copy(
        "[2/2] COPY target/release/app /app",
        "",
        "failed to calculate checksum of ref e4sw70wjuyll::w1aic0ar3ci0: \"/target/release/app\": not found",
        "Failed at: COPY target/release/app /app; /target/release/app isn't in the build context"
    )]
    #[case::out_of_disk(
        RUN,
        "",
        "failed to copy: write /var/lib/buildkit/runc-overlayfs/content/ingest/abc/data: no space left on device",
        "The builder ran out of disk space."
    )]
    fn test_failures_are_explained(
        #[case] name: &str,
        #[case] printed: &str,
        #[case] error: &str,
        #[case] expected: &str,
    ) {
        let build = steps();
        let progress = feed(
            &build,
            &[
                vertex("sha256:a", name, "started", ""),
                output("sha256:a", 2, printed),
                vertex("sha256:a", name, "started completed", error),
            ],
        );

        let failure = progress.failure("/logs/build.log".into()).unwrap();
        assert!(failure.message.starts_with(expected), "{}", failure.message);
    }

    #[test]
    fn test_run_sh_says_when_cargo_is_too_old() {
        let build = steps();
        let line = "gofer: Cargo 1.85 can't keep what it compiles between builds";
        assert!(
            include_str!("run.sh").contains("gofer: Cargo 1.$version can't keep"),
            "run.sh has to print what this looks for"
        );

        let progress = feed(
            &build,
            &[
                vertex("sha256:c", RUN, "started", ""),
                output("sha256:c", 2, &format!("{line}\n")),
            ],
        );
        assert_eq!(progress.old_cargo(), Some("1.85"));
    }
}
