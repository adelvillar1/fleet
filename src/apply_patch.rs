//! A content-anchored patch, applied file by file.
//!
//! The shape is the one OpenAI's `apply_patch` takes: an envelope of
//! sections, each naming a file and carrying lines that begin with a
//! marker rather than a line number.
//!
//! ```text
//! *** Begin Patch
//! *** Update File: src/lib.rs
//! @@ fn main @@
//!  context line
//! -removed line
//! +added line
//! *** Add File: notes.md
//! +hello
//! *** Delete File: old.rs
//! *** End Patch
//! ```
//!
//! Why content instead of line numbers: a model that writes `@@ -12,7 +12,9 @@`
//! is guessing about a file it read twenty turns ago, and every upstream
//! measurement of that shape lands around 99% corruption. A context line
//! either exists in the file or it does not.
//!
//! Atomicity is per file, not per patch: a five-file patch where the
//! third file does not match leaves the first two written, because
//! rolling back a write the owner may already be reading is worse than
//! telling them exactly which files landed. What never happens is a
//! *file* half-written: every hunk for a file is matched against the
//! current text before any byte of that file is written.
//!
//! The applier is pure: it takes the current text of each file (or
//! nothing, for a file being added) and returns the new text. Reading and
//! writing are the caller's, so the same logic is testable without a
//! filesystem and the tools layer keeps its one place that resolves paths.

use std::collections::BTreeMap;

/// One file's section of a patch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Change {
    /// A file that must not exist yet.
    Add { path: String, lines: Vec<String> },
    /// A file that must exist, and the hunks to apply in order.
    Update { path: String, hunks: Vec<Hunk> },
    /// A file that must exist and is removed whole.
    Delete { path: String },
}

/// One `@@` block: the context it expects and the lines it changes.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Hunk {
    /// The text after `@@`, when the model wrote a section heading. It is
    /// a hint for a human reader, not an anchor — the context lines are
    /// the anchor.
    pub heading: Option<String>,
    /// The lines as written, markers included: ` c`, `-c`, `+c`.
    pub lines: Vec<HunkLine>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HunkLine {
    Context(String),
    Remove(String),
    Add(String),
}

/// A whole patch, in the order the sections appeared.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Patch {
    pub changes: Vec<Change>,
}

/// Why a patch could not be applied. Every variant names the file and the
/// line the model should look at, because a refusal it cannot act on is a
/// refusal it will retry verbatim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The envelope markers are missing or out of order.
    Envelope(String),
    /// A section named a file but carried no anchor at all.
    EmptySection(String),
    /// A hunk's context does not appear in the file. `nearest` is the
    /// closest real line, when the file has one close enough to name.
    NoMatch {
        path: String,
        hunk: usize,
        expected: String,
        nearest: Option<(usize, String)>,
    },
    /// An update or delete named a file that is not there; an add named
    /// one that is.
    Missing(String),
    Exists(String),
    /// An update would change nothing — the same failure as `edit` with
    /// identical arguments, which is always a misread of the file.
    NoChange(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Envelope(reason) => write!(formatter, "{reason}"),
            Error::EmptySection(path) => write!(
                formatter,
                "`{path}` has a section but no lines: give the context lines and the +/- lines it changes"
            ),
            Error::NoMatch {
                path,
                hunk,
                expected,
                nearest,
            } => {
                write!(
                    formatter,
                    "`{path}`: hunk {} did not match — no {expected} appears in the file",
                    hunk + 1
                )?;
                match nearest {
                    Some((line, text)) => write!(
                        formatter,
                        "\nthe nearest line is {}: {}",
                        line,
                        text.trim_end()
                    ),
                    None => Ok(()),
                }
            }
            Error::Missing(path) => write!(
                formatter,
                "`{path}` does not exist: an update or a delete needs the file's current text — read it first"
            ),
            Error::Exists(path) => write!(
                formatter,
                "`{path}` already exists: an add is for a new file — update it instead"
            ),
            Error::NoChange(path) => write!(
                formatter,
                "`{path}`: the patch changes nothing — the context matched but no line adds or removes"
            ),
        }
    }
}

/// Parse a patch. The whole patch is read before anything is applied, so a
/// malformed late section refuses the patch rather than half-applying it.
pub fn parse(text: &str) -> Result<Patch, Error> {
    let lines: Vec<&str> = text.lines().collect();
    let begin = lines
        .iter()
        .position(|line| line.trim() == "*** Begin Patch")
        .ok_or_else(|| {
            Error::Envelope("the patch needs a `*** Begin Patch` line first".to_string())
        })?;
    // The trailer is decoration: a model that stops writing after the
    // last section meant the same patch, and refusing it teaches nothing.
    let end = lines
        .iter()
        .rposition(|line| line.trim() == "*** End Patch")
        .unwrap_or(lines.len());
    if end <= begin {
        return Err(Error::Envelope(
            "`*** End Patch` comes before `*** Begin Patch`".to_string(),
        ));
    }
    let body = &lines[begin + 1..end];
    let mut changes = Vec::new();
    let mut index = 0;
    while index < body.len() {
        let line = body[index];
        let trimmed = line.trim();
        if trimmed.is_empty() {
            index += 1;
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("*** Update File:") {
            let path = rest.trim().to_string();
            let (hunks, next) = read_hunks(body, index + 1)?;
            if hunks.is_empty() {
                return Err(Error::EmptySection(path));
            }
            changes.push(Change::Update { path, hunks });
            index = next;
        } else if let Some(rest) = trimmed.strip_prefix("*** Add File:") {
            let path = rest.trim().to_string();
            let (lines, next) = read_added(body, index + 1)?;
            if lines.is_empty() {
                return Err(Error::EmptySection(path));
            }
            changes.push(Change::Add { path, lines });
            index = next;
        } else if let Some(rest) = trimmed.strip_prefix("*** Delete File:") {
            changes.push(Change::Delete {
                path: rest.trim().to_string(),
            });
            index += 1;
        } else if trimmed.starts_with("*** Move to:") {
            return Err(Error::Envelope(
                "`*** Move to:` is not supported: delete the old file and add the new one"
                    .to_string(),
            ));
        } else {
            return Err(Error::Envelope(format!(
                "unexpected line inside the patch: {:?} — a section starts with `*** Update File:`, `*** Add File:`, or `*** Delete File:`",
                trimmed
            )));
        }
    }
    if changes.is_empty() {
        return Err(Error::Envelope(
            "the patch names no file: add a `*** Update File:`, `*** Add File:`, or `*** Delete File:` section"
                .to_string(),
        ));
    }
    Ok(Patch { changes })
}

/// The hunks of one update section, and the index of the line after it.
fn read_hunks(body: &[&str], mut index: usize) -> Result<(Vec<Hunk>, usize), Error> {
    let mut hunks = Vec::new();
    let mut current: Option<Hunk> = None;
    while index < body.len() {
        let line = body[index];
        let trimmed = line.trim();
        if trimmed.starts_with("*** ") {
            break;
        }
        if let Some(heading) = trimmed.strip_prefix("@@") {
            if let Some(hunk) = current.take() {
                hunks.push(hunk);
            }
            let heading = heading.trim().trim_end_matches("@@").trim();
            current = Some(Hunk {
                heading: if heading.is_empty() {
                    None
                } else {
                    Some(heading.to_string())
                },
                lines: Vec::new(),
            });
            index += 1;
            continue;
        }
        // A leading space is the context marker; an empty line inside a
        // hunk is context for an empty line, which is how a model writes
        // a blank line's context.
        let hunk_line = match line.as_bytes().first() {
            Some(b' ') => HunkLine::Context(line[1..].to_string()),
            Some(b'+') => HunkLine::Add(line[1..].to_string()),
            Some(b'-') => HunkLine::Remove(line[1..].to_string()),
            None => HunkLine::Context(String::new()),
            // A bare line with no marker is refused by parse() as an
            // unexpected line; inside a hunk it is the same mistake.
            _ => {
                return Err(Error::Envelope(format!(
                    "line {:?} inside a hunk has no marker: prefix it with a space for context, `-` to remove, or `+` to add",
                    line
                )))
            }
        };
        match current.as_mut() {
            Some(hunk) => hunk.lines.push(hunk_line),
            None => {
                // Lines before the first `@@`: the model wrote the hunk
                // body without a header. That is fine — the header is
                // decoration.
                current = Some(Hunk {
                    heading: None,
                    lines: vec![hunk_line],
                });
            }
        }
        index += 1;
    }
    if let Some(hunk) = current.take() {
        hunks.push(hunk);
    }
    Ok((hunks, index))
}

/// The added lines of an add section, and the index after it.
fn read_added(body: &[&str], mut index: usize) -> Result<(Vec<String>, usize), Error> {
    let mut lines = Vec::new();
    while index < body.len() {
        let line = body[index];
        if line.trim().starts_with("*** ") {
            break;
        }
        // Tolerate the `+` marker, and file text that happens to start
        // with `+` written without one.
        let text = line.strip_prefix('+').unwrap_or(line).to_string();
        lines.push(text);
        index += 1;
    }
    Ok((lines, index))
}

/// Apply one update to the text it was written against.
///
/// Hunks apply in order and each one anchors on its own context, so a
/// hunk's match position is searched from where the previous hunk ended —
/// two hunks with identical context land on two different places, in the
/// order the model wrote them.
pub fn apply_update(path: &str, text: &str, hunks: &[Hunk]) -> Result<String, Error> {
    let mut source: Vec<String> = if text.is_empty() {
        Vec::new()
    } else {
        text.split_inclusive('\n').map(str::to_string).collect()
    };
    let mut cursor = 0;
    let mut changed = false;
    for (number, hunk) in hunks.iter().enumerate() {
        let pattern: Vec<&HunkLine> = hunk
            .lines
            .iter()
            .filter(|line| !matches!(line, HunkLine::Add(_)))
            .collect();
        if pattern.is_empty() {
            return Err(Error::EmptySection(path.to_string()));
        }
        let found = find(&source, &pattern, cursor).ok_or_else(|| Error::NoMatch {
            path: path.to_string(),
            hunk: number,
            expected: describe(&pattern),
            nearest: nearest(&source, &pattern),
        })?;
        let mut replacement: Vec<String> = Vec::new();
        let mut consumed = 0;
        for line in &hunk.lines {
            match line {
                HunkLine::Context(_) => {
                    replacement.push(source[found + consumed].clone());
                    consumed += 1;
                }
                HunkLine::Remove(_) => {
                    consumed += 1;
                    changed = true;
                }
                HunkLine::Add(text) => {
                    replacement.push(format!("{text}\n"));
                    changed = true;
                }
            }
        }
        let end = found + consumed;
        source.splice(found..end, replacement);
        cursor = found;
    }
    if !changed {
        return Err(Error::NoChange(path.to_string()));
    }
    Ok(source.concat())
}

/// The added lines of an add section as a file body: every line ends, and
/// a patch that ends without a trailing newline in the source still gets
/// one, because every editor the owner uses writes one.
pub fn add_body(lines: &[String]) -> String {
    let mut body = String::new();
    for line in lines {
        body.push_str(line);
        body.push('\n');
    }
    body
}

/// Find `pattern` in `source` at or after `from`, comparing text with the
/// line ending stripped so a CRLF file and an LF patch still match.
fn find(source: &[String], pattern: &[&HunkLine], from: usize) -> Option<usize> {
    if pattern.len() > source.len() {
        return None;
    }
    (from..=source.len() - pattern.len()).find(|start| {
        pattern.iter().enumerate().all(|(offset, line)| {
            text_of(line) == source[start + offset].trim_end_matches(['\r', '\n'])
        })
    })
}

/// The text a hunk line anchors on.
fn text_of(line: &HunkLine) -> &str {
    match line {
        HunkLine::Context(text) | HunkLine::Remove(text) => text,
        HunkLine::Add(text) => text,
    }
}

/// The nearest real line to a hunk's first anchor, by token overlap. It is
/// a courtesy, not a guarantee: the failing hunk is the one the model must
/// reread, and a nearest line is how it finds where.
fn nearest(source: &[String], pattern: &[&HunkLine]) -> Option<(usize, String)> {
    let wanted: Vec<&str> = text_of(pattern[0]).split_whitespace().collect();
    if wanted.is_empty() {
        return None;
    }
    let mut best: Option<(usize, usize, String)> = None;
    for (index, line) in source.iter().enumerate() {
        let score = wanted.iter().filter(|word| line.contains(*word)).count();
        if score == 0 {
            continue;
        }
        if best
            .as_ref()
            .is_none_or(|(_, best_score, _)| score > *best_score)
        {
            best = Some((index, score, line.clone()));
        }
    }
    best.map(|(index, _, line)| (index + 1, line))
}

/// The anchor a refusal names: the first context or removed line, with how
/// many lines the hunk expected.
fn describe(pattern: &[&HunkLine]) -> String {
    let first = pattern
        .iter()
        .find(|line| matches!(line, HunkLine::Context(_)))
        .or_else(|| pattern.first())
        .map(|line| text_of(line))
        .unwrap_or("");
    if first.is_empty() {
        format!("hunk of {} line(s)", pattern.len())
    } else {
        format!(
            "line {:?} (and {} line(s) after it)",
            first,
            pattern.len() - 1
        )
    }
}

/// Apply a whole patch against the current text of each file.
///
/// `existing` is the tree as the caller read it: a path mapped to its
/// text. An add's path must not be in the map; an update's or delete's
/// must be.
pub fn apply(patch: &Patch, existing: &BTreeMap<String, String>) -> Result<Vec<Outcome>, Error> {
    let mut written = Vec::new();
    for change in &patch.changes {
        match change {
            Change::Add { path, lines } => {
                if existing.contains_key(path) {
                    return Err(Error::Exists(path.clone()));
                }
                written.push(Outcome::Added {
                    path: path.clone(),
                    text: add_body(lines),
                });
            }
            Change::Update { path, hunks } => {
                let Some(current) = existing.get(path) else {
                    return Err(Error::Missing(path.clone()));
                };
                let text = apply_update(path, current, hunks)?;
                written.push(Outcome::Updated {
                    path: path.clone(),
                    text,
                });
            }
            Change::Delete { path } => {
                if !existing.contains_key(path) {
                    return Err(Error::Missing(path.clone()));
                }
                written.push(Outcome::Deleted { path: path.clone() });
            }
        }
    }
    Ok(written)
}

/// What one file's section did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Added { path: String, text: String },
    Updated { path: String, text: String },
    Deleted { path: String },
}

impl Outcome {
    pub fn path(&self) -> &str {
        match self {
            Outcome::Added { path, .. }
            | Outcome::Updated { path, .. }
            | Outcome::Deleted { path } => path,
        }
    }

    /// The line a report shows.
    pub fn summary(&self) -> String {
        match self {
            Outcome::Added { path, text } => {
                format!("added {path} ({} line(s))", text.lines().count())
            }
            Outcome::Updated { path, .. } => format!("updated {path}"),
            Outcome::Deleted { path } => format!("deleted {path}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn patch_of(text: &str) -> Patch {
        parse(text).expect("a parseable patch")
    }

    fn tree(entries: &[(&str, &str)]) -> BTreeMap<String, String> {
        entries
            .iter()
            .map(|(path, text)| (path.to_string(), text.to_string()))
            .collect()
    }

    #[test]
    fn a_multi_file_patch_updates_adds_and_deletes() {
        let patch = patch_of(
            &[
                "*** Begin Patch",
                "*** Update File: src/a.rs",
                "@@",
                " fn one() {",
                "-    old();",
                "+    new();",
                " }",
                "*** Add File: docs/b.md",
                "+# B",
            ]
            .join("\n"),
        );
        let existing = tree(&[("src/a.rs", "fn one() {\n    old();\n}\n")]);
        let outcomes = apply(&patch, &existing).unwrap();
        assert_eq!(outcomes.len(), 2);
        assert_eq!(outcomes[0].path(), "src/a.rs");
        assert_eq!(
            outcomes[0],
            Outcome::Updated {
                path: "src/a.rs".to_string(),
                text: "fn one() {\n    new();\n}\n".to_string()
            }
        );
        assert_eq!(
            outcomes[1],
            Outcome::Added {
                path: "docs/b.md".to_string(),
                text: "# B\n".to_string()
            }
        );
    }

    #[test]
    fn a_delete_removes_the_file() {
        let patch = patch_of("*** Begin Patch\n*** Delete File: gone.rs");
        let outcomes = apply(&patch, &tree(&[("gone.rs", "x\n")])).unwrap();
        assert_eq!(
            outcomes,
            vec![Outcome::Deleted {
                path: "gone.rs".to_string()
            }]
        );
    }

    #[test]
    fn a_hunk_that_does_not_match_names_itself_and_the_nearest_line() {
        let patch = patch_of(
            &[
                "*** Begin Patch",
                "*** Update File: a.rs",
                "@@",
                " fn one() {",
                "-    gone();",
                "+    new();",
                " }",
            ]
            .join("\n"),
        );
        let existing = tree(&[("a.rs", "fn one() {\n    present();\n}\n")]);
        let error = apply(&patch, &existing).unwrap_err();
        let text = error.to_string();
        assert!(text.contains("hunk 1 did not match"), "{text}");
        assert!(text.contains("nearest line is 1: fn one() {"), "{text}");
    }

    #[test]
    fn two_hunks_with_the_same_context_apply_in_order() {
        let patch = patch_of(
            &[
                "*** Begin Patch",
                "*** Update File: a.rs",
                "@@",
                " x",
                "-a",
                "+A",
                "@@",
                " x",
                "-b",
                "+B",
            ]
            .join("\n"),
        );
        let existing = tree(&[("a.rs", "x\na\nx\nb\n")]);
        let outcomes = apply(&patch, &existing).unwrap();
        assert_eq!(
            outcomes[0],
            Outcome::Updated {
                path: "a.rs".to_string(),
                text: "x\nA\nx\nB\n".to_string()
            }
        );
    }

    #[test]
    fn a_patch_that_changes_nothing_is_refused() {
        let patch = patch_of(&["*** Begin Patch", "*** Update File: a.rs", "@@", " x"].join("\n"));
        let error = apply(&patch, &tree(&[("a.rs", "x\n")])).unwrap_err();
        assert_eq!(error, Error::NoChange("a.rs".to_string()));
    }

    #[test]
    fn add_to_an_existing_file_and_update_a_missing_one_are_named() {
        let add = patch_of(&["*** Begin Patch", "*** Add File: a.rs", "+x"].join("\n"));
        assert_eq!(
            apply(&add, &tree(&[("a.rs", "old\n")])).unwrap_err(),
            Error::Exists("a.rs".to_string())
        );
        let update = patch_of(&["*** Begin Patch", "*** Update File: b.rs", "@@", "-x"].join("\n"));
        assert_eq!(
            apply(&update, &tree(&[("a.rs", "old\n")])).unwrap_err(),
            Error::Missing("b.rs".to_string())
        );
    }

    #[test]
    fn a_missing_envelope_or_a_markerless_line_is_refused_by_name() {
        assert!(parse("no envelope").is_err());
        let error = parse("*** Begin Patch\n*** Update File: a\n??\n*** End Patch").unwrap_err();
        assert!(error.to_string().contains("has no marker"), "{error}");
        let straight =
            parse(&["*** Begin Patch", "*** Update File: a", "+new"].join("\n")).unwrap();
        assert_eq!(straight.changes.len(), 1);
    }
}
