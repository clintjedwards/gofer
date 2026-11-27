//! Turns the steps of a build into a Dockerfile, and finds which images a Dockerfile downloads.

use super::{
    Step, Steps,
    source::{Kind, Source},
};
use anyhow::{Context, Result, bail};
use lazy_regex::regex;
use sha2::{Digest, Sha256};
use std::{
    collections::{HashMap, VecDeque},
    os::unix::ffi::OsStrExt,
};

/// The longest line BuildKit accepts in a Dockerfile.
///
/// <https://github.com/moby/buildkit/blob/v0.35.1/frontend/dockerfile/parser/parser.go#L590>
const MAX_LINE_BYTES: usize = 65535;

/// The variables BuildKit sets itself, which a `FROM` can use without an `ARG`. Tasks are built for linux/amd64. The
/// `BUILD` ones are really whatever the builder runs on, but here they only help find which registry an image is in,
/// and a registry's name doesn't change with the machine.
///
/// <https://github.com/moby/buildkit/blob/v0.35.1/frontend/dockerfile/dockerfile2llb/platform.go#L39-L57>
const PLATFORM_ARGS: [(&str, &str); 8] = [
    ("BUILDPLATFORM", "linux/amd64"),
    ("BUILDOS", "linux"),
    ("BUILDARCH", "amd64"),
    ("BUILDVARIANT", ""),
    ("TARGETPLATFORM", "linux/amd64"),
    ("TARGETOS", "linux"),
    ("TARGETARCH", "amd64"),
    ("TARGETVARIANT", ""),
];

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

/// Turns steps into a Dockerfile. The sources are the ones from [`super::sources`].
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

/// Finds every image a Dockerfile downloads: its bases, the images it copies or mounts files from, and the frontend a
/// `# syntax` line asks for. A build gets the user's logins for only these images' registries.
///
/// Gofer doesn't pass build arguments, so a variable in a `FROM` always has the default its `ARG` gives it, and this
/// works it out the way BuildKit does. BuildKit doesn't allow variables in the other places.
pub fn images(dockerfile: &str) -> Result<Vec<String>> {
    let mut lines = dockerfile.lines().enumerate().peekable();
    let mut images = vec![];
    let mut escape = '\\';

    // Parser directives can only be the very first lines.
    //
    // <https://docs.docker.com/reference/dockerfile/#parser-directives>
    while let Some(directive) = (lines.peek().copied())
        .and_then(|(_, line)| regex!(r"^\s*#\s*([a-zA-Z]+)\s*=\s*(\S+)\s*$").captures(line))
    {
        match directive[1].to_lowercase().as_str() {
            "syntax" => images.push(directive[2].to_string()),
            "escape" if &directive[2] == "`" => escape = '`',
            _ => {}
        }
        lines.next();
    }

    let instructions = instructions(lines, escape);

    // A name that's a stage's refers to that stage, wherever in the file it is, and so does a stage's number.
    let stages: Vec<String> = (instructions.iter())
        .filter_map(|(_, text)| match words(text).as_slice() {
            [from, .., keyword, name]
                if from.eq_ignore_ascii_case("FROM") && keyword.eq_ignore_ascii_case("AS") =>
            {
                Some(name.to_lowercase())
            }
            _ => None,
        })
        .collect();
    let is_image = |name: &&str| {
        !name.eq_ignore_ascii_case("scratch")
            && name.parse::<usize>().is_err()
            && !stages.contains(&name.to_lowercase())
    };

    let mut args: HashMap<String, String> = (PLATFORM_ARGS.iter())
        .map(|(name, value)| (name.to_string(), value.to_string()))
        .collect();
    let mut in_stage = false;

    for (number, text) in &instructions {
        let words = words(text);
        let Some((instruction, words)) = words.split_first() else {
            continue;
        };
        let cant_follow = || {
            format!(
                "Gofer can't tell which image line {number} uses, since it can't follow the variable in it. It \
                 needs to know which registry to use your login for. In a FROM, variables can be $NAME, ${{NAME}}, \
                 ${{NAME:-default}} or ${{NAME:+value}}."
            )
        };

        match instruction.to_uppercase().as_str() {
            // Only the ones before the first FROM can be used in a FROM.
            "ARG" if !in_stage => {
                for (name, value) in words.iter().filter_map(|arg| arg.split_once('=')) {
                    let value = expand(unquote(value), &args).with_context(cant_follow)?;
                    args.insert(name.to_string(), value);
                }
            }
            "FROM" => {
                in_stage = true;
                if let Some(image) = words.iter().find(|word| !word.starts_with("--")) {
                    let image = expand(image, &args).with_context(cant_follow)?;
                    images.extend(Some(image.as_str()).filter(is_image).map(String::from));
                }
            }
            "COPY" | "ADD" => {
                let from = (words.iter()).filter_map(|word| word.strip_prefix("--from="));
                images.extend(from.map(unquote).filter(is_image).map(String::from));
            }
            "RUN" => {
                let from = (words.iter())
                    .filter_map(|word| word.strip_prefix("--mount="))
                    .flat_map(|mount| unquote(mount).split(','))
                    .filter_map(|option| option.strip_prefix("from="));
                images.extend(from.filter(is_image).map(String::from));
            }
            _ => {}
        }
    }

    images.sort();
    images.dedup();
    Ok(images)
}

/// Joins a Dockerfile's lines into instructions, each with the number of the line it starts on. Comments and what's
/// in heredocs are left out, since a line in either can look like an instruction.
///
/// <https://github.com/moby/buildkit/blob/v0.35.1/frontend/dockerfile/parser/parser.go#L305-L434>
fn instructions<'a>(
    lines: impl Iterator<Item = (usize, &'a str)>,
    escape: char,
) -> Vec<(usize, String)> {
    let mut instructions = vec![];
    let mut current: Option<(usize, String)> = None;

    // The words that end each heredoc, and whether tabs before them are left out.
    let mut heredocs: VecDeque<(String, bool)> = VecDeque::new();

    for (index, line) in lines {
        if let Some((end, strip_tabs)) = heredocs.front() {
            let line = if *strip_tabs {
                line.trim_start_matches('\t')
            } else {
                line
            };
            if line == end {
                heredocs.pop_front();
            }
            continue;
        }

        // These can even be in the middle of an instruction that goes over more than one line.
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let (number, mut text) = current.take().unwrap_or((index + 1, String::new()));
        if let Some(start) = line.trim_end().strip_suffix(escape) {
            text.push_str(start);
            current = Some((number, text));
            continue;
        }
        text.push_str(line);

        // A word like `<<EOF` starts a heredoc, which goes on until a line that's only `EOF`.
        //
        // <https://github.com/moby/buildkit/blob/v0.35.1/frontend/dockerfile/parser/parser.go#L503-L521>
        if regex!(r"(?i)^\s*(ONBUILD\s+)?(RUN|COPY|ADD)\s").is_match(&text) {
            for word in words(&text) {
                if let Some(heredoc) = regex!(r"^\d*<<(-?)\s*([^<]+)$").captures(word) {
                    let end = heredoc[2].replace(['"', '\''], "");
                    heredocs.push_back((end, &heredoc[1] == "-"));
                }
            }
        }

        instructions.push((number, text));
    }

    instructions.extend(current);
    instructions
}

/// Splits an instruction into words at the spaces that aren't in quotes. The quotes stay.
fn words(text: &str) -> Vec<&str> {
    let mut words = vec![];
    let mut start = None;
    let mut quote = None;

    for (index, c) in text.char_indices() {
        match quote {
            Some(open) if c == open => quote = None,
            Some(_) => {}
            None if c.is_whitespace() => {
                if let Some(start) = start.take() {
                    words.push(&text[start..index]);
                }
            }
            None => {
                start.get_or_insert(index);
                if c == '"' || c == '\'' {
                    quote = Some(c);
                }
            }
        }
    }

    words.extend(start.map(|start| &text[start..]));
    words
}

fn unquote(word: &str) -> &str {
    for quote in ['"', '\''] {
        if let Some(inside) = word
            .strip_prefix(quote)
            .and_then(|word| word.strip_suffix(quote))
        {
            return inside;
        }
    }

    word
}

/// Puts the values of variables into a word, the way BuildKit does. One that isn't set is empty. `None` when it uses a
/// variable in a way this doesn't follow, like `${NAME#prefix}`.
///
/// <https://github.com/moby/buildkit/blob/v0.35.1/frontend/dockerfile/shell/lex.go#L353-L500>
fn expand(word: &str, args: &HashMap<String, String>) -> Option<String> {
    let name_length = |text: &str| {
        text.find(|c: char| !c.is_ascii_alphanumeric() && c != '_')
            .unwrap_or(text.len())
    };

    let mut expanded = String::new();
    let mut rest = word;

    while let Some(dollar) = rest.find('$') {
        expanded.push_str(&rest[..dollar]);
        rest = &rest[dollar + 1..];

        let Some(braced) = rest.strip_prefix('{') else {
            let (name, after) = rest.split_at(name_length(rest));
            match name {
                "" => expanded.push('$'),
                name => expanded.push_str(args.get(name).map_or("", String::as_str)),
            }
            rest = after;
            continue;
        };

        // The `}` that ends this one, after any that are inside it.
        let mut depth = 0;
        let end = braced.find(|c| {
            match c {
                '{' => depth += 1,
                '}' if depth == 0 => return true,
                '}' => depth -= 1,
                _ => {}
            }
            false
        })?;
        let inside = &braced[..end];
        rest = &braced[end + 1..];

        let (name, modifier) = inside.split_at(name_length(inside));
        let value = args.get(name);

        // With a colon, empty counts the same as not set.
        let (empty_is_unset, modifier) = match modifier.strip_prefix(':') {
            Some(modifier) => (true, modifier),
            None => (false, modifier),
        };
        let set = value.filter(|value| !empty_is_unset || !value.is_empty());

        let mut modifier = modifier.chars();
        let operator = modifier.next();
        let word = modifier.as_str();

        expanded.push_str(&match (operator, set) {
            (None, _) if name.is_empty() || empty_is_unset => return None,
            (None, _) => value.cloned().unwrap_or_default(),
            (Some('-'), Some(value)) => value.clone(),
            (Some('-'), None) => expand(word, args)?,
            (Some('+'), Some(_)) => expand(word, args)?,
            (Some('+'), None) => String::new(),
            // When it isn't set, BuildKit stops with the message in `word`, which says it better than we could.
            (Some('?'), _) => value.cloned().unwrap_or_default(),
            _ => return None,
        });
    }

    expanded.push_str(rest);
    Some(expanded)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::builder::Build;
    use gofer_sdk::config::Keep;
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
        let sources = super::super::sources(&Build::Steps(steps.clone()), dir.path())?;
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
        let sources = super::super::sources(&Build::Steps(steps.clone()), dir.path()).unwrap();
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

    #[test]
    fn test_every_image_a_dockerfile_downloads_is_found() {
        let dockerfile = r#"# syntax=docker/dockerfile:1
ARG GO_VERSION=1.25
ARG REGISTRY=ghcr.io/acme
ARG RUNTIME=${REGISTRY}/runtime
from --platform=$BUILDPLATFORM golang:${GO_VERSION} AS Build
COPY --from=ghcr.io/astral-sh/uv:0.9 /uv /bin/
RUN --mount=type=cache,target=/root/.cache \
    --mount=type=bind,from=quay.io/acme/tools:1,source=/,target=/tools go build
FROM build AS test
FROM ${RUNTIME}:${TARGETARCH}
COPY --from=Build /app /app
COPY --from=0 /a /a
RUN --mount=type=bind,from=test,target=/test true
FROM scratch
"#;

        assert_eq!(
            images(dockerfile).unwrap(),
            vec![
                "docker/dockerfile:1",
                "ghcr.io/acme/runtime:amd64",
                "ghcr.io/astral-sh/uv:0.9",
                "golang:1.25",
                "quay.io/acme/tools:1",
            ]
        );
    }

    #[test]
    fn test_only_instructions_can_name_images() {
        let dockerfile = "FROM alpine:3.22
RUN <<EOF
FROM in-a-heredoc
EOF
RUN cat <<-\"END\" > /a && cat <<'MORE' > /b
\tFROM in-a-heredoc-with-tabs
\tEND
FROM in-a-second-heredoc
MORE
# FROM in-a-comment
FROM \\
  # A comment can be in the middle of an instruction.

  debian:13
RUN echo \"not <<a heredoc\" 'nor <<this'
FROM python:3
";

        assert_eq!(
            images(dockerfile).unwrap(),
            vec!["alpine:3.22", "debian:13", "python:3"]
        );
    }

    #[test]
    fn test_from_uses_only_args_from_before_the_first_stage() {
        let dockerfile = "ARG VERSION=1
FROM alpine:${VERSION}
ARG VERSION=2
FROM debian:${VERSION}
";
        assert_eq!(images(dockerfile).unwrap(), vec!["alpine:1", "debian:1"]);

        // The escape directive changes what continues a line.
        let dockerfile = "# escape=`
FROM `
  mcr.microsoft.com/windows/servercore:ltsc2022
";
        assert_eq!(
            images(dockerfile).unwrap(),
            vec!["mcr.microsoft.com/windows/servercore:ltsc2022"]
        );
    }

    #[rstest]
    #[case("$A", Some("a"))]
    #[case("${A}-x", Some("a-x"))]
    #[case("x${UNSET}y", Some("xy"))]
    #[case("${EMPTY:-default}", Some("default"))]
    #[case("${EMPTY-default}", Some(""))]
    #[case("${UNSET-default}", Some("default"))]
    #[case("${A:+other}", Some("other"))]
    #[case("${EMPTY:+other}", Some(""))]
    #[case("${EMPTY+other}", Some("other"))]
    #[case("${UNSET:-${A}b}", Some("ab"))]
    #[case("${A:?must be set}", Some("a"))]
    #[case("cost$", Some("cost$"))]
    #[case("$-x", Some("$-x"))]
    #[case("${A#a}", None)]
    #[case("${A%a}", None)]
    #[case("${A/a/b}", None)]
    #[case("${A:}", None)]
    #[case("${}", None)]
    #[case("${A", None)]
    fn test_variables_are_expanded_like_buildkit_does(
        #[case] word: &str,
        #[case] expected: Option<&str>,
    ) {
        let args = HashMap::from([("A".into(), "a".into()), ("EMPTY".into(), String::new())]);
        assert_eq!(expand(word, &args).as_deref(), expected);
    }

    #[test]
    fn test_a_variable_that_cant_be_followed_says_where() {
        let dockerfile = "ARG TAG=v1\n\nFROM alpine:${TAG#v}\n";
        let error = images(dockerfile).unwrap_err().to_string();
        assert!(error.contains("line 3"), "{error}");
    }
}
