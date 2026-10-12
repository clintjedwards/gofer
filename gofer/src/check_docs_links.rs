//! Checks every internal link in the built documentation book. Run via `make check-docs-links`.
//!
//! mdbook-linkcheck only looks at the markdown source, so it misses two kinds of breakage that only show up in the
//! built site: links to a section's `README.md` (mdbook renders the page as `index.html` but rewrites the link to
//! `README.html`, which doesn't exist) and `#anchor` links to headings that were renamed. This walks the built HTML
//! instead and checks that each link's page exists and that its anchor is an id on that page.

use regex::Regex;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};

const BOOK_DIR: &str = "docs/book/html";

/// Pages mdbook generates by stitching or wrapping the others; their links are already covered by the real pages.
const SKIPPED_PAGES: [&str; 3] = ["print.html", "toc.html", "404.html"];

fn main() {
    let book_dir = Path::new(BOOK_DIR);
    if !book_dir.exists() {
        eprintln!("{BOOK_DIR} doesn't exist; build the docs first with 'make build-docs'");
        std::process::exit(1);
    }

    let main_re = Regex::new(r"(?s)<main>(.*)</main>").unwrap();
    let href_re = Regex::new(r#"href="([^"]+)""#).unwrap();
    let id_re = Regex::new(r#"\bid="([^"]+)""#).unwrap();

    let mut pages = vec![];
    collect_html(book_dir, &mut pages);

    let mut ids: HashMap<PathBuf, HashSet<String>> = HashMap::new();
    let mut problems = BTreeSet::new();

    for page in &pages {
        let name = page.file_name().unwrap().to_string_lossy();
        if SKIPPED_PAGES.contains(&name.as_ref()) {
            continue;
        }

        let html = std::fs::read_to_string(page).expect("could not read page");
        // Only check the page body so the sidebar and theme chrome don't add noise.
        let body = main_re
            .captures(&html)
            .map_or(html.as_str(), |c| c.get(1).unwrap().as_str());

        for href in href_re.captures_iter(body).map(|c| c[1].to_string()) {
            if href.contains("://") || href.starts_with("//") || href.starts_with("mailto:") {
                continue;
            }

            let (path, anchor) = href.split_once('#').unwrap_or((&href, ""));
            let mut target = if path.is_empty() {
                page.clone()
            } else {
                normalize(&page.parent().unwrap().join(path))
            };
            if target.is_dir() {
                target = target.join("index.html");
            }

            let rel_page = page.strip_prefix(book_dir).unwrap().display();
            if !target.exists() {
                problems.insert(format!("{rel_page}: missing page -> {href}"));
                continue;
            }

            if anchor.is_empty() || target.extension().is_none_or(|ext| ext != "html") {
                continue;
            }

            let target_ids = ids.entry(target.clone()).or_insert_with(|| {
                let html = std::fs::read_to_string(&target).expect("could not read link target");
                id_re
                    .captures_iter(&html)
                    .map(|c| c[1].to_string())
                    .collect()
            });
            if !target_ids.contains(anchor) {
                problems.insert(format!("{rel_page}: missing anchor -> {href}"));
            }
        }
    }

    for problem in &problems {
        println!("{problem}");
    }

    if !problems.is_empty() {
        eprintln!("{} broken links in the docs", problems.len());
        std::process::exit(1);
    }
}

fn collect_html(dir: &Path, pages: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("could not read docs dir") {
        let path = entry.expect("could not read docs dir entry").path();
        if path.is_dir() {
            collect_html(&path, pages);
        } else if path.extension().is_some_and(|ext| ext == "html") {
            pages.push(path);
        }
    }
}

/// Resolves `.` and `..` without touching the filesystem, so a link to a missing page still gets a sensible path.
fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                out.pop();
            }
            other => out.push(other),
        }
    }
    out
}
