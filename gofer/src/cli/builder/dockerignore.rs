//! Decides which files a `.dockerignore` leaves out of a build.
//!
//! People expect theirs to work the way it does with `docker build`, so this does what Docker's matcher does. The rules
//! are close to `.gitignore`'s but not the same. For example, `*.log` only matches files at the top.
//!
//! - <https://docs.docker.com/build/concepts/context/#dockerignore-files>
//! - <https://github.com/moby/patternmatcher>

use anyhow::{Context, Result, bail};
use regex::Regex;
use std::{iter::Peekable, str::Chars};

/// Characters that mean something other than themselves in a pattern.
const SPECIAL_CHARACTERS: [char; 5] = ['*', '?', '[', ']', '\\'];

#[derive(Debug)]
struct Rule {
    /// Starts with a `!`, so it brings back what an earlier rule left out.
    negated: bool,

    /// The pattern up to its first special character.
    prefix: String,

    regex: Regex,
}

impl Rule {
    fn parse(line: &str) -> Result<Option<Self>> {
        let (negated, pattern) = match line.strip_prefix('!') {
            Some(pattern) => (true, pattern.trim()),
            None => (false, line),
        };

        if pattern.is_empty() {
            bail!("nothing follows the '!'");
        }

        let pattern = clean(pattern);

        // What's left of patterns like `.` and `/`. They match nothing.
        if pattern.is_empty() {
            return Ok(None);
        }

        Ok(Some(Self {
            negated,
            prefix: pattern
                .chars()
                .take_while(|c| !SPECIAL_CHARACTERS.contains(c))
                .collect(),
            regex: Regex::new(&to_regex(&pattern)?)?,
        }))
    }
}

/// What the rules say about one path.
#[derive(Debug, Clone, Default)]
pub struct Verdict {
    pub ignored: bool,

    /// The path is a cache or inside of one; see [`DockerIgnore::check`].
    in_cache: bool,

    /// The rules that matched, by index. A rule that matches a folder matches everything in it too.
    matched: Vec<bool>,
}

#[derive(Debug, Default)]
pub struct DockerIgnore {
    rules: Vec<Rule>,
}

impl DockerIgnore {
    pub fn parse(content: &str) -> Result<Self> {
        let mut rules = vec![];

        // Some editors on Windows start files with a byte order mark.
        let content = content.trim_start_matches('\u{feff}');

        for (index, line) in content.lines().enumerate() {
            if line.starts_with('#') || line.trim().is_empty() {
                continue;
            }

            let rule = Rule::parse(line.trim())
                .with_context(|| format!("Could not understand line {}: '{line}'", index + 1))?;
            rules.extend(rule);
        }

        Ok(Self { rules })
    }

    /// The verdict on the folder that the rules apply to, which is never ignored itself.
    pub fn root(&self) -> Verdict {
        Verdict {
            matched: vec![false; self.rules.len()],
            ..Default::default()
        }
    }

    /// Checks a path, given the verdict on the folder it's in. When several rules match, the last one wins.
    ///
    /// A folder that's marked as a cache is ignored as though the first line of the file said so. The rest of the file
    /// still gets its say, so `!target` brings back a cache named `target`.
    pub fn check(&self, path: &str, is_cache: bool, folder: &Verdict) -> Verdict {
        let in_cache = is_cache || folder.in_cache;
        let mut ignored = in_cache;

        let matched = self
            .rules
            .iter()
            .zip(&folder.matched)
            .map(|(rule, folder_matched)| {
                // Only check rules that would change the outcome. This isn't only to save time: a rule that wasn't
                // checked for a folder doesn't apply to the files inside it either, and `docker build` works that way.
                let matched =
                    *folder_matched || (rule.negated == ignored && rule.regex.is_match(path));

                if matched {
                    ignored = !rule.negated;
                }

                matched
            })
            .collect();

        Verdict {
            ignored,
            in_cache,
            matched,
        }
    }

    /// Whether something inside of an ignored folder could still be brought back by a `!` rule.
    pub fn reaches_into(&self, folder: &str) -> bool {
        let folder = format!("{folder}/");

        self.rules.iter().any(|rule| {
            rule.negated && (rule.prefix.starts_with(&folder) || folder.starts_with(&rule.prefix))
        })
    }
}

/// Removes the parts of a path that don't change where it leads, the same way as Go's `filepath.Clean`, along with the
/// leading slash. Patterns always start from the top, so `/foo` and `foo` mean the same thing.
fn clean(path: &str) -> String {
    let mut parts: Vec<&str> = vec![];

    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." if parts.last().is_some_and(|last| *last != "..") => {
                parts.pop();
            }
            ".." if path.starts_with('/') => {}
            part => parts.push(part),
        }
    }

    parts.join("/")
}

fn to_regex(pattern: &str) -> Result<String> {
    let mut regex = String::from("(?s)^");
    let mut chars = pattern.chars().peekable();

    // A leading `**` usually stands for any number of whole folders, so `**/foo` matches `a/foo` but not `a/xfoo`.
    // Without the slash, and with nothing else special in the pattern, Docker only compares the ends instead. That's
    // what makes `**.log` match `a/b.log`.
    if let Some(rest) = pattern.strip_prefix("**")
        && !rest.starts_with('/')
        && !rest.contains(SPECIAL_CHARACTERS)
    {
        return Ok(format!("{regex}.*{}$", regex::escape(rest)));
    }

    while let Some(c) = chars.next() {
        match c {
            '*' if chars.next_if_eq(&'*').is_some() => {
                chars.next_if_eq(&'/');

                // At the end it matches everything, and anywhere else it's any number of folders, including none.
                if chars.peek().is_none() {
                    regex.push_str(".*");
                } else {
                    regex.push_str("(.*/)?");
                }
            }
            '*' => regex.push_str("[^/]*"),
            '?' => regex.push_str("[^/]"),
            '[' => regex.push_str(&character_class(&mut chars)?),
            '\\' => {
                let escaped = chars.next().context("nothing follows the '\\'")?;
                regex.push_str(&regex::escape(&escaped.to_string()));
            }
            c => regex.push_str(&regex::escape(&c.to_string())),
        }
    }

    regex.push('$');
    Ok(regex)
}

/// Reads the rest of something like `[a-z]` or `[^abc]`, which is what Go's `filepath.Match` allows between brackets.
///
/// <https://pkg.go.dev/path/filepath#Match>
fn character_class(chars: &mut Peekable<Chars>) -> Result<String> {
    let mut class = String::from("[");

    if chars.next_if_eq(&'^').is_some() {
        class.push('^');
    }

    let mut ranges = 0;

    while ranges == 0 || chars.next_if_eq(&']').is_none() {
        let low = class_character(chars)?;
        class.push_str(&regex::escape(&low.to_string()));

        if chars.next_if_eq(&'-').is_some() {
            let high = class_character(chars)?;
            class.push('-');
            class.push_str(&regex::escape(&high.to_string()));
        }

        ranges += 1;
    }

    class.push(']');
    Ok(class)
}

fn class_character(chars: &mut Peekable<Chars>) -> Result<char> {
    match chars.next().context("a '[' is never closed")? {
        c @ ('-' | ']') => bail!("'{c}' needs a '\\' in front of it when it's between brackets"),
        '\\' => chars.next().context("nothing follows the '\\'"),
        c => Ok(c),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    /// Checks a path the way walking down to it would, one folder at a time.
    fn ignored(content: &str, path: &str, cache: Option<&str>) -> bool {
        let ignore = DockerIgnore::parse(content).unwrap();
        let mut verdict = ignore.root();

        for (index, _) in path.match_indices('/').chain([(path.len(), "")]) {
            let path = &path[..index];
            verdict = ignore.check(path, Some(path) == cache, &verdict);
        }

        verdict.ignored
    }

    #[rstest]
    // Patterns start from the top, with or without a slash, and cover everything inside a folder.
    #[case("foo", "foo", true)]
    #[case("/foo/", "foo", true)]
    #[case("./a/../foo", "foo", true)]
    #[case("foo", "foo/bar/baz", true)]
    #[case("foo", "a/foo", false)]
    #[case("foo", "foobar", false)]
    #[case("a.b", "axb", false)]
    // Wildcards stay inside their folder.
    #[case("*.log", "a.log", true)]
    #[case("*.log", "a/b.log", false)]
    #[case("*/temp*", "a/temporary", true)]
    #[case("*/temp*", "a/b/temporary", false)]
    #[case("temp?", "tempa", true)]
    #[case("temp?", "temp", false)]
    #[case("temp?", "temp/", false)]
    // Double wildcards don't.
    #[case("**/foo", "foo", true)]
    #[case("**/foo", "a/b/foo", true)]
    #[case("**/foo", "a/xfoo", false)]
    #[case("**/*.log", "a/b.log", true)]
    #[case("**.log", "a/b.log", true)]
    #[case("foo/**", "foo/bar/baz", true)]
    #[case("foo/**", "foo", false)]
    #[case("a/**/b", "a/b", true)]
    #[case("a/**/b", "a/x/y/b", true)]
    #[case("**", "anything", true)]
    // Character classes and escapes.
    #[case("[a-c]at", "bat", true)]
    #[case("[^a-c]at", "bat", false)]
    #[case("[a&&b]", "&", true)]
    #[case("[*]", "*", true)]
    #[case("[*]", "x", false)]
    #[case("\\*", "*", true)]
    #[case("\\*", "x", false)]
    #[case("\\!foo", "!foo", true)]
    // Lines that aren't patterns, and whitespace around ones that are.
    #[case("# foo", "# foo", false)]
    #[case("\u{feff}foo", "foo", true)]
    #[case("\n  foo \t\r\n", "foo", true)]
    #[case(".", "foo", false)]
    #[case("/", "foo", false)]
    // The last rule that matches wins.
    #[case("*.md\n!README.md", "README.md", false)]
    #[case("*.md\n! README.md", "README.md", false)]
    #[case("*.md\n!README*.md\nREADME-secret.md", "README-secret.md", true)]
    #[case("*.md\nREADME-secret.md\n!README*.md", "README-secret.md", false)]
    #[case("foo\n!foo/bar", "foo/bar/baz", false)]
    #[case("foo\n!foo/bar", "foo/other", true)]
    #[case("*\n!src", "src/main.rs", false)]
    #[case("*\n!src", "docs/main.rs", true)]
    // Nothing had ignored `dir` when its rule came up, so it wasn't checked and doesn't cover what's inside.
    #[case("**/*.x\n!dir", "dir/f.x", true)]
    fn test_rules(#[case] content: &str, #[case] path: &str, #[case] expected: bool) {
        assert_eq!(ignored(content, path, None), expected);
    }

    #[rstest]
    #[case("", "target", true)]
    #[case("", "target/debug/app", true)]
    #[case("", "src/main.rs", false)]
    // The user's own rules can bring back all of a cache or a part of it.
    #[case("!target", "target/debug/app", false)]
    #[case("!target/debug/app", "target/debug/app", false)]
    #[case("!target/debug/app", "target/debug/other", true)]
    #[case("!target\ntarget/debug", "target/debug/app", true)]
    fn test_caches(#[case] content: &str, #[case] path: &str, #[case] expected: bool) {
        assert_eq!(ignored(content, path, Some("target")), expected);
    }

    #[rstest]
    #[case("!target/debug/app", "target", true)]
    #[case("!target/debug/app", "target/debug", true)]
    #[case("!target/debug/app", "target/release", false)]
    #[case("!target/*/app", "target/release", true)]
    #[case("!tar*", "target", true)]
    #[case("!**/app", "target", true)]
    #[case("!README.md", "target", false)]
    #[case("!target-docs", "target", false)]
    #[case("target/debug/app", "target", false)]
    fn test_reaches_into(#[case] content: &str, #[case] folder: &str, #[case] expected: bool) {
        let ignore = DockerIgnore::parse(content).unwrap();

        assert_eq!(ignore.reaches_into(folder), expected);
    }

    #[rstest]
    #[case("!")]
    #[case("foo\\")]
    #[case("[")]
    #[case("[]")]
    #[case("[a")]
    #[case("[a-]")]
    #[case("[-a]")]
    #[case("[z-a]")]
    fn test_bad_patterns_are_refused(#[case] pattern: &str) {
        let error = DockerIgnore::parse(&format!("fine\n{pattern}")).unwrap_err();

        assert!(error.to_string().contains("line 2"), "{error}");
    }
}
