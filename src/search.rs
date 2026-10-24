//! The search primitives: a capped `grep` and a mtime-ordered `glob`.
//!
//! Both are bounded on purpose. The measured lesson from SWE-agent is
//! that a search which pages through matches one at a time scores worse
//! than no search at all; a search that answers with a counted, capped
//! slice is what the model can act on. `rg` is preferred when it is on
//! the machine (it honours `.gitignore` and compiles patterns), and
//! `grep` is the fallback.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::SystemTime;

/// Files a `glob` returns at most.
pub const GLOB_CAP: usize = 100;
/// Paths a `grep` in files mode returns at most.
pub const GREP_FILES_CAP: usize = 100;
/// Lines a `grep` in content mode returns at most.
pub const GREP_CONTENT_CAP: usize = 50;
/// Count rows a `grep` in count mode returns at most.
pub const GREP_COUNT_CAP: usize = 100;
/// Entries the walker visits before it stops.
const WALK_CAP: usize = 50_000;
/// Depth the walker descends.
const WALK_DEPTH: usize = 12;
/// Directories never worth searching in a coding run.
const SKIP_DIRS: &[&str] = &[
    ".git",
    "target",
    "node_modules",
    "__pycache__",
    ".venv",
    "venv",
    ".mypy_cache",
    ".pytest_cache",
    "dist",
];

/// One finished search: the text, and whether a cap cut it.
pub struct SearchOutcome {
    pub text: String,
    pub truncated: bool,
}

/// The two providers content search has, in preference order: `rg`
/// (honors `.gitignore`, compiles patterns) and `grep` (always there).
/// The dispatch policy picks between them; a provider that is not
/// installed is unhealthy, and a healthy fallback is used instead.
pub fn grep_providers() -> Vec<crate::tool_router::Provider> {
    let rg = Command::new("rg")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false);
    vec![
        crate::tool_router::Provider {
            name: "rg".to_string(),
            warm: true,
            healthy: rg,
        },
        crate::tool_router::Provider {
            name: "grep".to_string(),
            warm: true,
            healthy: true,
        },
    ]
}

/// `grep`: literal or regex search, capped and counted.
pub fn run_grep(
    pattern: &str,
    path: &Path,
    base: &Path,
    glob: Option<&str>,
    case_sensitive: bool,
    mode: &str,
    max: Option<usize>,
) -> Result<SearchOutcome, String> {
    let target = display_path(path, base);
    let providers = grep_providers();
    let chosen = crate::tool_router::choose(&providers, false)
        .map(|(provider, reason)| (provider.name.clone(), reason))
        .unwrap_or_else(|| ("grep".to_string(), "no provider"));
    let mut command = match chosen.0.as_str() {
        "rg" => {
            let mut command = Command::new("rg");
            command.arg("--color=never").arg("--no-messages");
            match mode {
                "content" => {
                    command
                        .arg("--line-number")
                        .arg("--no-heading")
                        .arg(format!("--max-count={}", max.unwrap_or(10)));
                }
                "count" => {
                    command.arg("--count");
                }
                _ => {
                    command.arg("--files-with-matches");
                }
            }
            command.arg("--max-filesize=2M");
            if !case_sensitive {
                command.arg("-i");
            }
            if let Some(glob) = glob {
                command.arg("--glob").arg(glob);
            }
            command
        }
        _ => {
            let mut command = Command::new("grep");
            command.arg("-r").arg("-I");
            match mode {
                "content" => {
                    command.arg("-n");
                }
                "count" => {
                    command.arg("-c");
                }
                _ => {
                    command.arg("-l");
                }
            }
            command
                .arg("--exclude-dir=.git")
                .arg("--exclude-dir=target");
            if !case_sensitive {
                command.arg("-i");
            }
            if let Some(glob) = glob {
                command.arg(format!("--include={glob}"));
            }
            command
        }
    };
    command
        .arg("--")
        .arg(pattern)
        .arg(&target)
        .current_dir(base);
    let output = command
        .output()
        .map_err(|error| format!("could not run the search: {error}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    if output.status.code() == Some(2) {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "the search failed: {}",
            stderr.trim().lines().next().unwrap_or("unknown error")
        ));
    }
    let lines: Vec<&str> = stdout.lines().filter(|line| !line.is_empty()).collect();
    if lines.is_empty() {
        return Ok(SearchOutcome {
            text: format!("no matches for `{pattern}` in {target}\n"),
            truncated: false,
        });
    }
    let (cap, label) = match mode {
        "content" => (max.unwrap_or(GREP_CONTENT_CAP).max(1), "matches"),
        "count" => (GREP_COUNT_CAP, "files"),
        _ => (max.unwrap_or(GREP_FILES_CAP).max(1), "files"),
    };
    let shown = lines.len().min(cap);
    let mut text = String::new();
    for line in &lines[..shown] {
        let line = strip_prefix(line, base);
        text.push_str(line.trim_end());
        text.push('\n');
    }
    let files = if mode == "content" {
        lines
            .iter()
            .filter_map(|line| line.split(':').next())
            .collect::<std::collections::BTreeSet<_>>()
            .len()
    } else {
        lines.len()
    };
    let truncated = lines.len() > shown;
    text.push_str(&format!(
        "({} {label} in {} file(s){}",
        lines.len(),
        files,
        if truncated {
            format!("; showing {shown} — narrow the query")
        } else {
            String::new()
        }
    ));
    if chosen.0 != "rg" {
        text.push_str(&format!(
            "; rg unavailable — dispatch chose grep ({})",
            chosen.1
        ));
    }
    text.push_str(")\n");
    Ok(SearchOutcome { text, truncated })
}

/// `glob`: walk, match, order by mtime, cap.
pub fn run_glob(pattern: &str, path: &Path, base: &Path) -> Result<SearchOutcome, String> {
    let mut found: Vec<(SystemTime, PathBuf)> = Vec::new();
    let mut visited = 0usize;
    let mut cut = false;
    walk(path, 0, &mut visited, &mut cut, &mut |entry, meta| {
        let relative = entry.strip_prefix(base).unwrap_or(entry);
        let text = relative.to_string_lossy().replace('\\', "/");
        let matches = if pattern.contains('/') {
            glob_match(pattern, &text)
        } else {
            entry
                .file_name()
                .map(|name| glob_match(pattern, &name.to_string_lossy()))
                .unwrap_or(false)
        };
        if matches {
            let mtime = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
            found.push((mtime, relative.to_path_buf()));
        }
    });
    found.sort_by(|left, right| right.0.cmp(&left.0));
    let total = found.len();
    let shown = total.min(GLOB_CAP);
    let mut text = String::new();
    for (_, path) in &found[..shown] {
        text.push_str(&path.to_string_lossy().replace('\\', "/"));
        text.push('\n');
    }
    if shown == 0 {
        text.push_str(&format!("no files matched `{pattern}`\n"));
    }
    if total > shown || cut {
        text.push_str(&format!(
            "({total} file(s) matched{}; showing {shown}, newest first)\n",
            if cut { ", walk capped" } else { "" }
        ));
    }
    Ok(SearchOutcome {
        text,
        truncated: total > shown || cut,
    })
}

/// Every file under `root` the walker reaches, relative to it, with the
/// same skips and caps a search uses. Sorted by path so two calls over an
/// unchanged tree return the same order — a map that reshuffles itself
/// between reads is a map the model cannot cite.
pub fn tree_files(root: &Path) -> (Vec<PathBuf>, bool) {
    let mut found: Vec<PathBuf> = Vec::new();
    let mut visited = 0usize;
    let mut cut = false;
    walk(root, 0, &mut visited, &mut cut, &mut |entry, _| {
        found.push(entry.strip_prefix(root).unwrap_or(entry).to_path_buf());
    });
    found.sort();
    (found, cut)
}

/// The entries in `dir` whose names are closest to `name`, for a
/// not-found error that points at what exists.
pub fn nearest_entries(dir: &Path, name: &str) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut names: Vec<(usize, String)> = entries
        .filter_map(Result::ok)
        .map(|entry| {
            let text = entry.file_name().to_string_lossy().into_owned();
            (distance(&text, name), text)
        })
        .collect();
    names.sort();
    names.into_iter().take(3).map(|(_, name)| name).collect()
}

fn walk(
    dir: &Path,
    depth: usize,
    visited: &mut usize,
    cut: &mut bool,
    on_file: &mut dyn FnMut(&Path, &std::fs::Metadata),
) {
    if depth > WALK_DEPTH || *visited >= WALK_CAP {
        *cut = true;
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        *visited += 1;
        if *visited >= WALK_CAP {
            *cut = true;
            return;
        }
        let path = entry.path();
        let Ok(meta) = entry.metadata() else { continue };
        if meta.is_dir() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if SKIP_DIRS.contains(&name.as_ref()) {
                continue;
            }
            walk(&path, depth + 1, visited, cut, on_file);
        } else if meta.is_file() {
            on_file(&path, &meta);
        }
    }
}

/// Match a `/`-separated glob against a `/`-separated path.
pub fn glob_match(pattern: &str, text: &str) -> bool {
    let pattern: Vec<&str> = pattern.split('/').collect();
    let text: Vec<&str> = text.split('/').collect();
    match_segments(&pattern, &text)
}

fn match_segments(pattern: &[&str], text: &[&str]) -> bool {
    match pattern.split_first() {
        None => text.is_empty(),
        Some((&"**", rest)) => {
            for index in 0..=text.len() {
                if match_segments(rest, &text[index..]) {
                    return true;
                }
            }
            false
        }
        Some((segment, rest)) => match text.split_first() {
            Some((first, tail)) => star_match(segment, first) && match_segments(rest, tail),
            None => false,
        },
    }
}

/// `*` any run of characters, `?` one character, everything else exact.
fn star_match(pattern: &str, text: &str) -> bool {
    let pattern: Vec<char> = pattern.chars().collect();
    let text: Vec<char> = text.chars().collect();
    let (mut p, mut t) = (0usize, 0usize);
    let (mut star, mut mark) = (None, 0usize);
    while t < text.len() {
        if p < pattern.len() && (pattern[p] == '?' || pattern[p] == text[t]) {
            p += 1;
            t += 1;
        } else if p < pattern.len() && pattern[p] == '*' {
            star = Some(p);
            mark = t;
            p += 1;
        } else if let Some(at) = star {
            p = at + 1;
            mark += 1;
            t = mark;
        } else {
            return false;
        }
    }
    while p < pattern.len() && pattern[p] == '*' {
        p += 1;
    }
    p == pattern.len()
}

fn display_path(path: &Path, base: &Path) -> String {
    path.strip_prefix(base)
        .map(|rest| {
            let text = rest.to_string_lossy().replace('\\', "/");
            if text.is_empty() {
                ".".to_string()
            } else {
                text
            }
        })
        .unwrap_or_else(|_| path.to_string_lossy().into_owned())
}

fn strip_prefix(line: &str, base: &Path) -> String {
    let prefix = format!("{}/", base.to_string_lossy());
    line.strip_prefix(&prefix).unwrap_or(line).to_string()
}

/// A small edit distance, enough to rank filenames.
pub(crate) fn distance(left: &str, right: &str) -> usize {
    let left: Vec<char> = left.chars().collect();
    let right: Vec<char> = right.chars().collect();
    let mut previous: Vec<usize> = (0..=right.len()).collect();
    let mut current = vec![0usize; right.len() + 1];
    for (i, a) in left.iter().enumerate() {
        current[0] = i + 1;
        for (j, b) in right.iter().enumerate() {
            let cost = if a == b { 0 } else { 1 };
            current[j + 1] = (previous[j + 1] + 1)
                .min(current[j] + 1)
                .min(previous[j] + cost);
        }
        std::mem::swap(&mut previous, &mut current);
    }
    previous[right.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grep_has_two_providers_and_grep_is_always_healthy() {
        let providers = grep_providers();
        assert_eq!(providers.len(), 2);
        assert_eq!(providers[0].name, "rg");
        assert_eq!(providers[1].name, "grep");
        assert!(providers[1].healthy);
    }

    #[test]
    fn globs_match_segments_stars_and_double_stars() {
        assert!(glob_match("*.rs", "main.rs"));
        assert!(!glob_match("*.rs", "main.rss"));
        assert!(glob_match("src/*.rs", "src/main.rs"));
        assert!(!glob_match("src/*.rs", "src/deep/main.rs"));
        assert!(glob_match("src/**/*.rs", "src/deep/main.rs"));
        assert!(glob_match("**/*.rs", "main.rs"));
        assert!(glob_match("src/**", "src/a/b/c.rs"));
        assert!(glob_match("a?c", "abc"));
        assert!(!glob_match("a?c", "ac"));
    }

    #[test]
    fn a_glob_lists_newest_first_and_skips_heavy_dirs() {
        let dir = std::env::temp_dir().join(format!("fleet-search-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::create_dir_all(dir.join("target/debug")).unwrap();
        std::fs::write(dir.join("src/main.rs"), "fn main() {}\n").unwrap();
        std::fs::write(dir.join("src/lib.rs"), "pub fn one() {}\n").unwrap();
        std::fs::write(dir.join("target/debug/junk.rs"), "junk\n").unwrap();
        let outcome = run_glob("**/*.rs", &dir, &dir).unwrap();
        assert!(outcome.text.contains("src/main.rs"), "{}", outcome.text);
        assert!(outcome.text.contains("src/lib.rs"), "{}", outcome.text);
        assert!(!outcome.text.contains("junk.rs"), "{}", outcome.text);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_grep_counts_and_caps() {
        let dir = std::env::temp_dir().join(format!("fleet-grep-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("one.txt"), "alpha\nneedle\n").unwrap();
        std::fs::write(dir.join("two.txt"), "needle\nneedle\n").unwrap();
        let outcome = run_grep("needle", &dir, &dir, None, true, "files", None).unwrap();
        assert!(outcome.text.contains("one.txt"), "{}", outcome.text);
        assert!(outcome.text.contains("two.txt"), "{}", outcome.text);
        assert!(outcome.text.contains("2 files"), "{}", outcome.text);
        let none = run_grep("absent-token", &dir, &dir, None, true, "files", None).unwrap();
        assert!(none.text.contains("no matches"), "{}", none.text);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn nearest_entries_ranks_the_real_names() {
        let dir = std::env::temp_dir().join(format!("fleet-near-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("Cargo.toml"), "").unwrap();
        std::fs::write(dir.join("Cargo.lock"), "").unwrap();
        std::fs::write(dir.join("README.md"), "").unwrap();
        let names = nearest_entries(&dir, "Cargo.tml");
        assert_eq!(names.first().map(String::as_str), Some("Cargo.toml"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
