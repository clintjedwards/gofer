//! Turns the steps of a build into a Dockerfile.

use super::{
    Step, Steps,
    source::{Kind, Source},
};
use anyhow::{Context, Result, bail};
use lazy_regex::regex;
use sha2::{Digest, Sha256};
use std::os::unix::ffi::OsStrExt;

/// The longest line BuildKit accepts in a Dockerfile.
///
/// <https://github.com/moby/buildkit/blob/v0.35.1/frontend/dockerfile/parser/parser.go#L590>
const MAX_LINE_BYTES: usize = 65535;

/// Every run step goes through this script, which sets up the compiler caches and then runs the user's command.
///
/// It's mounted into the step instead of pasted into its line. That keeps the user's command a single argument with no
/// quoting to get wrong, and keeps 40 lines of setup out of BuildKit's errors and logs. Being mounted, it never ends up
/// in the image, and neither does anything it sets.
const RUN_SCRIPT: &str = include_str!("run.sh");

/// A Dockerfile that expects to be built in a folder that looks like this. Each numbered folder is for one copy or run
/// step, in order.
///
/// ```text
/// copies/0/       The copied file, or what was in the copied folder.
/// runs/0/run.sh
/// runs/0/files    Where every file copied by earlier steps is in the image. Only for steps that get the Rust cache.
/// ```
#[derive(Debug)]
pub struct Rendered {
    pub dockerfile: String,

    /// The path and contents of everything that goes in `runs`.
    pub files: Vec<(String, Vec<u8>)>,

    /// Run steps that use Cargo but go without the Rust cache, each with the earlier step that's the reason.
    pub uncached_rust: Vec<(String, String)>,
}

/// Where the files of a copy go in the build's context. Copies can come from anywhere, like `./app` and `../shared`, and
/// two can have the same name, so each gets a numbered folder of its own.
pub fn copy_folder(index: usize) -> String {
    format!("copies/{index}/")
}

/// Turns steps into a Dockerfile. The sources are the ones from [`super::Build::sources`].
///
/// Everything the user wrote ends up in JSON, which is always one line, so a newline or a Dockerfile keyword in there
/// can't turn into another instruction. The task's name keeps its Rust cache apart from other tasks'.
///
/// How the image is run isn't part of it. A script can be longer than a Dockerfile line can, so [`super::image`] writes
/// the command into the finished image instead.
///
/// With `fresh_caches`, the compiler caches start out empty. The old ones stay for other builds to use.
pub fn render(
    task: &str,
    steps: &Steps,
    sources: &[Source],
    fresh_caches: bool,
) -> Result<Rendered> {
    let mut lines = vec![from_line(&steps.base)?];
    let mut files = vec![];

    let cache_suffix = match fresh_caches {
        true => format!("-{}", uuid::Uuid::now_v7().simple()),
        false => String::new(),
    };

    let mut copies = sources.iter().enumerate();
    let mut copied = false;
    let mut copied_rust = false;
    let mut copied_files: Vec<u8> = vec![];

    let mut runs: Vec<&str> = vec![];
    let mut ran_after_copy: Option<&str> = None;
    let mut uncached_rust = vec![];

    for step in &steps.steps {
        match step {
            Step::Copy { from, to } => {
                let (index, source) = copies
                    .next()
                    .with_context(|| format!("The files for copying '{from}' are missing"))?;

                // Docker reads a source like `a[1].py` as a wildcard. Using one on purpose to match the folder's only
                // file gets around that, and the file keeps its name.
                let wildcard = if source.is_folder { "" } else { "*" };
                let context_path = format!("{}{wildcard}", copy_folder(index));

                lines.push(format!("COPY {}", json(&[&context_path, &escape(to)])));

                copied = true;
                copied_rust |= source.has_file("Cargo.toml");
                list_files(source, to, &mut copied_files);
            }

            Step::Run { command } => {
                let folder = format!("runs/{}", runs.len());
                files.push((format!("{folder}/run.sh"), RUN_SCRIPT.into()));

                // The shared cache has Go's caches and Cargo's downloads, which every build can use at once: Go is made
                // for it, and run.sh shares Cargo's locks along with its downloads.
                //
                // Anyone can write to both caches, and run.sh gives each user a folder of their own. A cache owned by root
                // would break images that build as another user, which is how a Go build failed in testing.
                let mut line = format!(
                    "RUN --mount=type=bind,source={folder},target=/gofer-run \
                     --mount=type=cache,id=gofer-shared{cache_suffix},target=/gofer-cache/shared,mode=1777"
                );

                // run.sh makes copied files new so Cargo notices when they changed, and explains why it might not. That
                // can't be done for files an earlier step made out of the copied ones, like a crate that was generated
                // from a spec. Cargo could miss that such a file changed and ship an old build of it, so once there
                // might be any, we go without the Rust cache.
                match ran_after_copy {
                    None if copied_rust => {
                        files.push((format!("{folder}/files"), copied_files.clone()));

                        // It's locked because two builds using it at once would overwrite each other's files.
                        let id = rust_cache_id(task, &steps.base, &runs);
                        line.push_str(&format!(
                            " --mount=type=cache,id={id}{cache_suffix},target=/gofer-cache/rust,mode=1777,sharing=locked"
                        ));
                    }
                    Some(earlier) if copied_rust && command.contains("cargo") => {
                        uncached_rust.push((command.clone(), earlier.to_string()));
                    }
                    _ => {}
                }

                line.push_str(&format!(
                    " {}",
                    json(&["/bin/sh", "/gofer-run/run.sh", command])
                ));

                if line.len() > MAX_LINE_BYTES {
                    bail!(
                        "The run step starting with '{}' is too long for a Dockerfile; put it in a script and copy \
                         that in instead",
                        command.chars().take(40).collect::<String>()
                    );
                }

                lines.push(line);
                runs.push(command);
                if copied && ran_after_copy.is_none() {
                    ran_after_copy = Some(command);
                }
            }
        }
    }

    if let Some(keep) = &steps.keep {
        lines.push(from_line(&keep.base)?);

        for path in &keep.paths {
            if !path.starts_with('/') {
                bail!("Can't keep '{path}'; it has to be a full path, starting with '/'");
            }

            lines.push(format!(
                "COPY --from=0 {}",
                json(&[&escape(&literal(path)), &escape(path)])
            ));
        }
    }

    lines.push(String::new());

    Ok(Rendered {
        dockerfile: lines.join("\n"),
        files,
        uncached_rust,
    })
}

/// A FROM can't be JSON, so this only allows the characters that can be in an image's name. That leaves out spaces and
/// newlines, which could turn into more than a name, and `$`, which Docker would take as a variable.
///
/// <https://github.com/distribution/reference/blob/main/reference.go>
fn from_line(base: &str) -> Result<String> {
    if !regex!("^[a-zA-Z0-9][a-zA-Z0-9._:/@-]*$").is_match(base) {
        bail!("'{base}' isn't a valid image");
    }

    Ok(format!("FROM {base}"))
}

fn json(args: &[&str]) -> String {
    serde_json::json!(args).to_string()
}

/// Docker expands variables and takes quotes out of a COPY's paths, even in JSON. Without this, copying to `/$HOME`
/// would go wherever that variable says.
///
/// <https://docs.docker.com/reference/dockerfile/#environment-replacement>
fn escape(path: &str) -> String {
    let mut escaped = String::new();

    for c in path.chars() {
        if ['\\', '$', '\'', '"'].contains(&c) {
            escaped.push('\\');
        }
        escaped.push(c);
    }

    escaped
}

/// Docker reads the path to copy from as a pattern if it has any wildcards. Putting a `\` in front of one doesn't work,
/// but a character class with only the wildcard in it does.
fn literal(path: &str) -> String {
    const WILDCARDS: [char; 3] = ['*', '?', '['];

    if !path.contains(WILDCARDS) {
        return path.into();
    }

    let mut literal = String::new();

    for c in path.chars() {
        match c {
            c if WILDCARDS.contains(&c) => literal.push_str(&format!("[{c}]")),
            '\\' => literal.push_str("\\\\"),
            c => literal.push(c),
        }
    }

    literal
}

/// Adds where each of the source's files ends up in the image, each followed by a NUL. run.sh makes exactly these new.
/// Making the destination folder new instead would be simpler, but a copy to `/` would then touch the whole image.
fn list_files(source: &Source, to: &str, list: &mut Vec<u8>) {
    let mut add = |path: &str, name: &[u8]| {
        // Something like `-x` would look like a flag.
        if !path.starts_with('/') {
            list.extend_from_slice(b"./");
        }
        list.extend_from_slice(path.as_bytes());
        list.extend_from_slice(name);
        list.push(0);
    };

    let is_folder = to.ends_with('/');
    let folder = if is_folder {
        to.to_string()
    } else {
        format!("{to}/")
    };

    // A file that's copied to `/tmp` becomes `/tmp/file` if the image has a folder there, and `/tmp` if it doesn't. We
    // don't know which, so list both.
    if !source.is_folder && !is_folder {
        add(to, b"");
    }

    for entry in &source.entries {
        if matches!(entry.kind, Kind::File { .. }) {
            add(&folder, entry.name.as_os_str().as_bytes());
        }
    }
}

/// Names a run step's Rust cache after everything that decides what's in the image by then, except the copied files.
///
/// Say an earlier step clones a crate, and the user changes which version it clones and then changes it back. Just like
/// with copied files, Cargo wouldn't notice the second change. Those files aren't made new though, so a different step
/// has to mean a different cache.
fn rust_cache_id(task: &str, base: &str, earlier_runs: &[&str]) -> String {
    let mut hasher = Sha256::new();

    for part in [task, base].iter().chain(earlier_runs) {
        hasher.update((part.len() as u64).to_le_bytes());
        hasher.update(part);
    }

    format!("gofer-rust-{}", hex::encode(hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::builder::{Build, Keep};
    use pretty_assertions::assert_eq;
    use rstest::rstest;
    use std::fs;
    use tempfile::TempDir;

    fn copy(from: &str, to: &str) -> Step {
        Step::Copy {
            from: from.into(),
            to: to.into(),
        }
    }

    fn run(command: &str) -> Step {
        Step::Run {
            command: command.into(),
        }
    }

    fn steps(steps: Vec<Step>) -> Steps {
        Steps {
            base: "rust:1-trixie".into(),
            steps,
            keep: None,
        }
    }

    /// Has a Rust project, a Go project and a script.
    fn project() -> TempDir {
        let dir = tempfile::tempdir().unwrap();
        for file in [
            "rust/Cargo.toml",
            "rust/src/main.rs",
            "go/go.mod",
            "script.py",
        ] {
            let path = dir.path().join(file);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, "").unwrap();
        }
        dir
    }

    fn try_render(steps: &Steps) -> Result<Rendered> {
        let dir = project();
        let sources = Build::Steps(steps.clone()).sources(dir.path())?;
        render("default/nightly/report", steps, &sources, false)
    }

    fn rendered(steps: &Steps) -> Rendered {
        try_render(steps).unwrap()
    }

    /// The Rust cache that each run step gets.
    fn rust_caches(rendered: &Rendered) -> Vec<Option<&str>> {
        let id = regex!("id=(gofer-rust-[0-9a-f]+),target=/gofer-cache/rust,");

        rendered
            .dockerfile
            .lines()
            .filter(|line| line.starts_with("RUN "))
            .map(|line| Some(id.captures(line)?.get(1)?.as_str()))
            .collect()
    }

    fn file<'a>(rendered: &'a Rendered, path: &str) -> Option<&'a [u8]> {
        let (_, content) = rendered.files.iter().find(|(name, _)| name == path)?;
        Some(content)
    }

    #[test]
    fn test_steps_become_a_dockerfile() {
        let mut steps = steps(vec![
            copy("./rust", "/src"),
            copy("script.py", "/script.py"),
            run("cd /src && cargo build --release"),
        ]);
        steps.keep = Some(Keep {
            paths: vec!["/src/target/release/report".into(), "/script.py".into()],
            base: "debian:13-slim".into(),
        });

        let rendered = rendered(&steps);
        let cache = rust_caches(&rendered)[0].unwrap();

        assert_eq!(
            rendered.dockerfile.lines().collect::<Vec<_>>(),
            vec![
                "FROM rust:1-trixie",
                r#"COPY ["copies/0/","/src"]"#,
                r#"COPY ["copies/1/*","/script.py"]"#,
                &format!(
                    "RUN --mount=type=bind,source=runs/0,target=/gofer-run \
                     --mount=type=cache,id=gofer-shared,target=/gofer-cache/shared,mode=1777 \
                     --mount=type=cache,id={cache},target=/gofer-cache/rust,mode=1777,sharing=locked {}",
                    r#"["/bin/sh","/gofer-run/run.sh","cd /src && cargo build --release"]"#
                ),
                "FROM debian:13-slim",
                r#"COPY --from=0 ["/src/target/release/report","/src/target/release/report"]"#,
                r#"COPY --from=0 ["/script.py","/script.py"]"#,
            ]
        );
        assert_eq!(
            file(&rendered, "runs/0/run.sh"),
            Some(RUN_SCRIPT.as_bytes())
        );
        assert_eq!(
            file(&rendered, "runs/0/files"),
            Some(
                b"/src/Cargo.toml\0/src/src/main.rs\0/script.py\0/script.py/script.py\0".as_slice()
            )
        );
    }

    #[rstest]
    #[case("echo one\nRUN echo two")]
    #[case("cat <<EOF\nFROM evil\nEOF")]
    #[case("echo \"it's\" \\\n  $HOME # comment")]
    #[case("\r\n]")]
    fn test_user_text_stays_in_its_instruction(#[case] text: &str) {
        let mut steps = steps(vec![copy("rust", text), run(text)]);
        steps.keep = Some(Keep {
            paths: vec![format!("/{text}")],
            base: "debian:13-slim".into(),
        });
        let rendered = rendered(&steps);

        let instructions: Vec<&str> = rendered
            .dockerfile
            .split('\n')
            .map(|line| line.split(' ').next().unwrap())
            .collect();
        assert_eq!(
            instructions,
            vec!["FROM", "COPY", "RUN", "FROM", "COPY", ""]
        );

        // A run step's command also comes out the other side unchanged.
        let run = rendered.dockerfile.lines().nth(2).unwrap();
        let args: Vec<String> = serde_json::from_str(&run[run.find('[').unwrap()..]).unwrap();
        assert_eq!(args, vec!["/bin/sh", "/gofer-run/run.sh", text]);
    }

    #[rstest]
    #[case("/a b", r#"/a b"#)]
    #[case("/$HOME", r#"/\\$HOME"#)]
    #[case("/it's", r#"/it\\'s"#)]
    #[case("/a\"b", r#"/a\\\"b"#)]
    #[case("/a\\b", r#"/a\\\\b"#)]
    #[case("/a*[b]?", r#"/a*[b]?"#)]
    fn test_copies_land_where_they_say(#[case] to: &str, #[case] expected: &str) {
        let rendered = rendered(&steps(vec![copy("go", to)]));

        assert_eq!(
            rendered.dockerfile.lines().nth(1).unwrap(),
            format!(r#"COPY ["copies/0/","{expected}"]"#)
        );
    }

    #[rstest]
    #[case("/a b", r#""/a b","/a b""#)]
    #[case("/$HOME", r#""/\\$HOME","/\\$HOME""#)]
    #[case("/a\\b", r#""/a\\\\b","/a\\\\b""#)]
    #[case("/a*[b]?", r#""/a[*][[]b][?]","/a*[b]?""#)]
    #[case("/a\\b*", r#""/a\\\\\\\\b[*]","/a\\\\b*""#)]
    fn test_kept_paths_are_not_patterns(#[case] path: &str, #[case] expected: &str) {
        let mut steps = steps(vec![]);
        steps.keep = Some(Keep {
            paths: vec![path.into()],
            base: "alpine:3.22".into(),
        });

        assert_eq!(
            rendered(&steps).dockerfile.lines().nth(2).unwrap(),
            format!("COPY --from=0 [{expected}]")
        );
    }

    #[rstest]
    #[case("/src", "go", &["/src/go.mod"])]
    #[case("/src/", "go", &["/src/go.mod"])]
    #[case("/", "go", &["/go.mod"])]
    #[case(".", "go", &["././go.mod"])]
    #[case("-x", "go", &["./-x/go.mod"])]
    #[case("/a b\n", "go", &["/a b\n/go.mod"])]
    #[case("/tmp", "script.py", &["/tmp", "/tmp/script.py"])]
    #[case("/tmp/", "script.py", &["/tmp/script.py"])]
    #[case("-x", "script.py", &["./-x", "./-x/script.py"])]
    fn test_list_files(#[case] to: &str, #[case] from: &str, #[case] expected: &[&str]) {
        let dir = project();
        let source = Source::copy(&dir.path().join(from)).unwrap();
        let mut list = vec![];
        list_files(&source, to, &mut list);

        let expected: String = expected.iter().map(|path| format!("{path}\0")).collect();
        assert_eq!(String::from_utf8(list).unwrap(), expected);
    }

    #[test]
    fn test_only_rust_tasks_get_the_rust_cache() {
        assert_eq!(
            rust_caches(&rendered(&steps(vec![copy("go", "/src"), run("go build")]))),
            vec![None]
        );

        // This step comes before the code does.
        let rendered = rendered(&steps(vec![
            run("apt-get install -y protobuf-compiler"),
            copy("rust", "/src"),
            run("cargo build"),
        ]));
        let caches = rust_caches(&rendered);
        assert!(caches[0].is_none() && caches[1].is_some(), "{caches:?}");
        assert!(file(&rendered, "runs/0/files").is_none());
        assert!(file(&rendered, "runs/1/files").is_some());
    }

    #[test]
    fn test_no_rust_cache_after_a_step_that_could_have_made_code() {
        let rendered = rendered(&steps(vec![
            copy("rust", "/src"),
            run("./generate.sh"),
            run("cargo build"),
        ]));

        let caches = rust_caches(&rendered);
        assert!(caches[0].is_some() && caches[1].is_none(), "{caches:?}");
        assert!(file(&rendered, "runs/1/files").is_none());

        // So the user can be told how to get the cache back.
        assert_eq!(
            rendered.uncached_rust,
            vec![("cargo build".to_string(), "./generate.sh".to_string())]
        );
    }

    #[test]
    fn test_rust_caches_are_kept_apart() {
        let cache = |task: &str, base: &str, earlier: &str| {
            rust_cache_id(task, base, &earlier.split_whitespace().collect::<Vec<_>>())
        };
        let mut ids = vec![
            cache("a", "rust:1", ""),
            cache("b", "rust:1", ""),
            cache("a", "rust:2", ""),
            cache("a", "rust:1", "one"),
            cache("a", "rust:1", "one two"),
            cache("a", "rust:1", "onetwo"),
        ];
        assert_eq!(ids[0], cache("a", "rust:1", ""));

        let total = ids.len();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), total);
    }

    #[test]
    fn test_fresh_caches_are_new_every_time() {
        let dir = project();
        let steps = steps(vec![copy("rust", "/src"), run("cargo build")]);
        let sources = Build::Steps(steps.clone()).sources(dir.path()).unwrap();
        let caches = |fresh| {
            let rendered = render("default/nightly/report", &steps, &sources, fresh).unwrap();
            let ids = regex!(r"id=([a-z0-9-]+),");
            let line = rendered.dockerfile.lines().nth(2).unwrap().to_string();
            let ids: Vec<String> = ids
                .captures_iter(&line)
                .map(|id| id[1].to_string())
                .collect();
            ids
        };

        let usual = caches(false);
        assert_eq!(usual.len(), 2);
        assert_eq!(caches(false), usual);

        let (fresh, fresher) = (caches(true), caches(true));
        for index in 0..2 {
            assert!(fresh[index].starts_with(&usual[index]));
            assert_ne!(fresh[index], usual[index]);
            assert_ne!(fresh[index], fresher[index]);
        }
    }

    #[rstest]
    #[case("rust:1-trixie", true)]
    #[case("localhost:5000/team/rust@sha256:abc", true)]
    #[case("scratch", true)]
    #[case("", false)]
    #[case("rust AS build", false)]
    #[case("rust\nRUN echo hi", false)]
    #[case("$BASE", false)]
    #[case("--platform=linux/arm64", false)]
    #[case("al\"pine\"", false)]
    fn test_base_images(#[case] base: &str, #[case] valid: bool) {
        let mut as_base = steps(vec![]);
        as_base.base = base.into();
        assert_eq!(try_render(&as_base).is_ok(), valid);

        let mut as_keep = steps(vec![]);
        as_keep.keep = Some(Keep {
            paths: vec![],
            base: base.into(),
        });
        assert_eq!(try_render(&as_keep).is_ok(), valid);
    }

    #[test]
    fn test_long_steps_and_relative_keeps_are_refused() {
        let error = try_render(&steps(vec![run(&format!("echo {}", "x".repeat(66_000)))]))
            .unwrap_err()
            .to_string();
        assert!(error.contains("echo xxx") && error.len() < 200, "{error}");

        let mut steps = steps(vec![]);
        steps.keep = Some(Keep {
            paths: vec!["target/release/report".into()],
            base: "alpine".into(),
        });
        let error = try_render(&steps).unwrap_err().to_string();
        assert!(error.contains("target/release/report"), "{error}");
    }
}
