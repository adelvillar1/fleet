//! File primitives shared by the read and edit tools: a bounded window
//! with line numbers, an atomic write, and a content-anchored edit that
//! refuses an ambiguous target.
//!
//! The edit is content-anchored on purpose. Line-number anchors fail
//! silently when a file shifts by one line; a passage anchor either
//! matches exactly once or names why it did not, which is the failure
//! mode the model can recover from.

use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// Lines a read returns when the caller names no limit.
pub const READ_DEFAULT_LINES: usize = 2_000;

/// The largest window a single read may ask for.
pub const READ_MAX_LINES: usize = 10_000;

/// Characters kept from one line before it is cut.
pub const LINE_CHAR_CAP: usize = 2_000;

/// Bytes inspected for a NUL byte before a file is called binary.
const SNIFF_BYTES: usize = 8 * 1024;

/// One file's bounded window.
#[derive(Debug)]
pub struct Window {
    /// The rendered lines, each prefixed with its number.
    pub text: String,
    /// The file's total line count, counted without keeping the lines.
    pub total_lines: usize,
    /// The first line included (1-based).
    pub start: usize,
    /// The last line included, 0 when the window is empty.
    pub end: usize,
    /// Whether a line was cut at the character cap.
    pub cut: bool,
}

/// Read `offset..offset+limit` with line numbers, counting the whole
/// file so the caller can say how much was left.
///
/// The window is kept; the file is streamed, never held whole. A file
/// with a NUL in its first eight kilobytes is refused by name rather
/// than handed to a model as mojibake.
pub fn read_window(path: &Path, offset: usize, limit: usize) -> Result<Window, String> {
    let file = File::open(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let mut reader = BufReader::new(file);
    let mut head = [0u8; SNIFF_BYTES];
    let sniffed = std::io::Read::read(&mut reader, &mut head)
        .map_err(|error| format!("{}: {error}", path.display()))?;
    if head[..sniffed].contains(&0) {
        return Err(format!(
            "{}: binary file (a NUL byte in the first {} bytes); use bash to inspect it",
            path.display(),
            SNIFF_BYTES
        ));
    }
    // Re-open: the sniff consumed bytes the line reader needs.
    let file = File::open(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let mut reader = BufReader::new(file);
    let offset = offset.max(1);
    let limit = limit.clamp(1, READ_MAX_LINES);
    let mut raw = Vec::new();
    let mut line_no = 0usize;
    let mut text = String::new();
    let mut start = 0usize;
    let mut end = 0usize;
    let mut cut = false;
    loop {
        raw.clear();
        let read = reader
            .read_until(b'\n', &mut raw)
            .map_err(|error| format!("{}: {error}", path.display()))?;
        if read == 0 {
            break;
        }
        line_no += 1;
        if line_no < offset || line_no >= offset.saturating_add(limit) {
            continue;
        }
        let line = std::str::from_utf8(&raw).map_err(|_| {
            format!(
                "{}: not valid UTF-8 at line {line_no}; use bash to inspect it",
                path.display()
            )
        })?;
        let line = line.trim_end_matches(['\n', '\r']);
        if start == 0 {
            start = line_no;
        }
        end = line_no;
        let mut body: String = line.chars().take(LINE_CHAR_CAP).collect();
        if line.chars().count() > LINE_CHAR_CAP {
            body.push('\u{2026}');
            cut = true;
        }
        text.push_str(&format!("{line_no:>6}\t{body}\n"));
    }
    Ok(Window {
        text,
        total_lines: line_no,
        start,
        end,
        cut,
    })
}

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Write `content` through a sibling temp file and a rename, so a
/// reader never sees a half-written file and a crash leaves the old one
/// intact.
pub fn atomic_write(path: &Path, content: &str) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("{}: has no parent directory", path.display()))?;
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "file".to_string());
    let mut last: Option<String> = None;
    for _ in 0..3 {
        let n = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let temp: PathBuf = parent.join(format!(".{name}.fleet-tmp-{}-{n}", std::process::id()));
        match OpenOptions::new().create_new(true).write(true).open(&temp) {
            Ok(mut file) => {
                let result = (|| -> std::io::Result<()> {
                    file.write_all(content.as_bytes())?;
                    file.sync_all()?;
                    Ok(())
                })();
                drop(file);
                if let Err(error) = result {
                    let _ = fs::remove_file(&temp);
                    return Err(format!("{}: {error}", path.display()));
                }
                return fs::rename(&temp, path).map_err(|error| {
                    let _ = fs::remove_file(&temp);
                    format!("{}: {error}", path.display())
                });
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                last = Some(error.to_string());
            }
            Err(error) => return Err(format!("{}: {error}", path.display())),
        }
    }
    Err(format!(
        "{}: could not create a temporary file{}",
        path.display(),
        last.map(|reason| format!(": {reason}")).unwrap_or_default()
    ))
}

/// The result of one anchored edit.
#[derive(Debug)]
pub struct EditOutcome {
    pub replaced: usize,
    pub first_line: usize,
    pub diff: String,
}

/// Replace one exact passage. Zero matches and an ambiguous match are
/// both errors that name what to do next; a unique match is written
/// atomically and returns a bounded diff.
pub fn edit(path: &Path, old: &str, new: &str, replace_all: bool) -> Result<EditOutcome, String> {
    if old.is_empty() {
        return Err("`old_string` must not be empty".to_string());
    }
    let original = fs::read_to_string(path)
        .map_err(|error| format!("{}: {error}; use read_file first", path.display()))?;
    let replaced = original.matches(old).count();
    if replaced == 0 {
        return Err(not_found(path, &original, old));
    }
    let (first_line, lines) = occurrence_lines(&original, old);
    if replaced > 1 && !replace_all {
        let shown: Vec<String> = lines.iter().take(8).map(usize::to_string).collect();
        return Err(format!(
            "`old_string` appears {replaced} times in {} (lines {}); include more surrounding \
             context to make it unique, or pass `replace_all: true`",
            path.display(),
            shown.join(", ")
        ));
    }
    let updated = if replace_all {
        original.replace(old, new)
    } else {
        original.replacen(old, new, 1)
    };
    atomic_write(path, &updated)?;
    Ok(EditOutcome {
        replaced,
        first_line,
        diff: diff_hunk(&original, &updated, first_line),
    })
}

/// The line numbers an exact passage starts on.
fn occurrence_lines(content: &str, needle: &str) -> (usize, Vec<usize>) {
    let mut lines = Vec::new();
    let mut line = 1usize;
    let mut last = 0usize;
    for (at, _) in content.match_indices(needle) {
        line += content[last..at].matches('\n').count();
        lines.push(line);
        last = at;
    }
    (lines.first().copied().unwrap_or(1), lines)
}

/// A zero-match error that points at the closest line rather than
/// leaving the model to re-read the whole file.
fn not_found(path: &Path, content: &str, needle: &str) -> String {
    let probe = needle
        .split_whitespace()
        .filter(|word| word.chars().count() >= 4)
        .max_by_key(|word| word.chars().count())
        .unwrap_or(needle)
        .trim_matches(|c: char| !c.is_alphanumeric() && c != '_');
    let mut best: Option<(usize, &str)> = None;
    if !probe.is_empty() {
        for (index, line) in content.lines().enumerate() {
            if line.contains(probe) {
                best = Some((index + 1, line));
                break;
            }
        }
    }
    match best {
        Some((line_no, line)) => {
            let body: String = line.trim().chars().take(160).collect();
            format!(
                "`old_string` was not found in {}; the closest line is {line_no}: `{body}` — \
                 the file may have changed since you read it. Re-read it and anchor on what is \
                 there now.",
                path.display()
            )
        }
        None => format!(
            "`old_string` was not found in {}; re-read the file and anchor on text it contains",
            path.display()
        ),
    }
}

/// The changed region with a little context, capped so a large edit
/// cannot flood the result.
fn diff_hunk(before: &str, after: &str, first_line: usize) -> String {
    let old: Vec<&str> = before.lines().collect();
    let new: Vec<&str> = after.lines().collect();
    let mut prefix = 0usize;
    while prefix < old.len() && prefix < new.len() && old[prefix] == new[prefix] {
        prefix += 1;
    }
    let mut suffix = 0usize;
    while suffix < old.len().saturating_sub(prefix)
        && suffix < new.len().saturating_sub(prefix)
        && old[old.len() - 1 - suffix] == new[new.len() - 1 - suffix]
    {
        suffix += 1;
    }
    let context = 2usize;
    let mut out = String::new();
    let mut emitted = 0usize;
    let start = prefix.saturating_sub(context);
    for (index, line) in old.iter().enumerate().take(prefix).skip(start) {
        out.push_str(&format!("{:>6}  {}\n", index + 1, line));
        emitted += 1;
    }
    for line in &old[prefix..old.len() - suffix] {
        if emitted >= 200 {
            out.push_str("\u{2026}[diff cut]\n");
            return out;
        }
        out.push_str(&format!("{:>6} -{}\n", first_line + emitted, line));
        emitted += 1;
    }
    for line in &new[prefix..new.len() - suffix] {
        if emitted >= 200 {
            out.push_str("\u{2026}[diff cut]\n");
            return out;
        }
        out.push_str(&format!("{:>6} +{}\n", first_line + emitted, line));
        emitted += 1;
    }
    let tail_start = old.len() - suffix;
    for (index, line) in old
        .iter()
        .enumerate()
        .take(old.len().min(tail_start + context))
        .skip(tail_start)
    {
        out.push_str(&format!("{:>6}  {}\n", index + 1, line));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("fleet-fileops-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn a_window_keeps_line_numbers_and_counts_the_file() {
        let dir = scratch("window");
        let path = dir.join("big.txt");
        let body: String = (1..=50).map(|n| format!("line {n}\n")).collect();
        fs::write(&path, body).unwrap();
        let window = read_window(&path, 20, 5).unwrap();
        assert_eq!(window.total_lines, 50);
        assert_eq!((window.start, window.end), (20, 24));
        assert!(window.text.contains("    20\tline 20"));
        assert!(window.text.contains("    24\tline 24"));
        assert!(!window.text.contains("line 25"));
        assert_eq!(window.text.lines().count(), 5);
    }

    #[test]
    fn a_window_past_the_end_is_empty_and_named() {
        let dir = scratch("past");
        let path = dir.join("small.txt");
        fs::write(&path, "one\ntwo\n").unwrap();
        let window = read_window(&path, 9, 4).unwrap();
        assert_eq!(window.total_lines, 2);
        assert_eq!(window.end, 0);
        assert!(window.text.is_empty());
    }

    #[test]
    fn a_binary_file_is_refused_by_name() {
        let dir = scratch("binary");
        let path = dir.join("blob.bin");
        fs::write(&path, [0u8, 1, 2, 3]).unwrap();
        let error = read_window(&path, 1, 10).unwrap_err();
        assert!(error.contains("binary file"), "{error}");
    }

    #[test]
    fn an_edit_replaces_one_passage_and_reports_its_line() {
        let dir = scratch("edit");
        let path = dir.join("code.rs");
        fs::write(
            &path,
            "fn main() {\n    let x = 1;\n    println!(\"{x}\");\n}\n",
        )
        .unwrap();
        let outcome = edit(&path, "let x = 1;", "let x = 2;", false).unwrap();
        assert_eq!(outcome.replaced, 1);
        assert_eq!(outcome.first_line, 2);
        assert!(outcome.diff.contains("-    let x = 1;"), "{}", outcome.diff);
        assert!(outcome.diff.contains("+    let x = 2;"), "{}", outcome.diff);
        assert!(fs::read_to_string(&path).unwrap().contains("let x = 2;"));
    }

    #[test]
    fn an_ambiguous_edit_names_the_lines_and_does_not_write() {
        let dir = scratch("ambiguous");
        let path = dir.join("code.rs");
        fs::write(&path, "a\nsame\nb\nsame\n").unwrap();
        let error = edit(&path, "same", "other", false).unwrap_err();
        assert!(error.contains("appears 2 times"), "{error}");
        assert!(error.contains("lines 2, 4"), "{error}");
        assert_eq!(fs::read_to_string(&path).unwrap(), "a\nsame\nb\nsame\n");
    }

    #[test]
    fn a_missing_passage_names_the_closest_line() {
        let dir = scratch("missing");
        let path = dir.join("code.rs");
        fs::write(&path, "alpha\nfn configure_widgets() {}\nomega\n").unwrap();
        let error = edit(&path, "fn configure_widget() {}", "x", false).unwrap_err();
        assert!(error.contains("closest line is 2"), "{error}");
        assert!(error.contains("configure_widgets"), "{error}");
    }

    #[test]
    fn an_atomic_write_leaves_no_temp_behind() {
        let dir = scratch("atomic");
        let path = dir.join("out.txt");
        atomic_write(&path, "first").unwrap();
        atomic_write(&path, "second").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "second");
        let strays: Vec<_> = fs::read_dir(&dir)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name().to_string_lossy().contains("fleet-tmp"))
            .collect();
        assert!(strays.is_empty(), "temporary files left: {strays:?}");
    }
}
