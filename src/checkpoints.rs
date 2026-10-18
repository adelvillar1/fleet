//! The trust trio's floor: per-turn file-state checkpoints.
//!
//! A checkpoint is the working directory's file state at one turn's
//! boundary, copied under the home's own `checkpoints/<session>/<turn>/`
//! with a manifest beside it. It exists so a run that went sideways can
//! be rewound to the turn before it did — the converged spec's
//! most-requested gap — and so the diff review has a before to measure
//! a run's after against.
//!
//! Three rules keep it honest:
//!
//! - **Bounded.** A tree is capped by file count and total bytes, and
//!   the version-control and build directories are skipped: a
//!   checkpoint of `.git` is a copy of the repository's whole history,
//!   which is not a snapshot of the working state.
//! - **Named.** Every read names what it left out — a tree over the cap
//!   captures what it reached and the manifest says how many files it
//!   did not.
//! - **Restore is exact.** A restore makes the tree match the manifest:
//!   entries are copied back and files the manifest does not carry are
//!   removed, so a rewind cannot leave a file the checkpoint never saw.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// How many files one checkpoint captures. A tree past it captures its
/// first `MAX_FILES` in walk order and names the rest.
const MAX_FILES: usize = 2_000;

/// How many bytes one checkpoint captures in total.
const MAX_BYTES: u64 = 64 * 1024 * 1024;

/// The directories a checkpoint never enters: version control's own
/// state, the build outputs, and the vendored dependencies. A snapshot
/// of the working state does not carry the repository's history.
const IGNORED: [&str; 6] = [
    ".git",
    "target",
    "node_modules",
    ".venv",
    "__pycache__",
    ".DS_Store",
];

/// One checkpoint's summary, as the manifest and the surface carry it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    pub turn: u64,
    /// The UTC stamp the capture ran at.
    pub at: String,
    pub files: usize,
    pub bytes: u64,
    /// The files the cap left out, named rather than hidden.
    pub unread: usize,
}

/// One captured file's manifest entry.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Entry {
    path: String,
    bytes: u64,
}

/// The manifest one checkpoint writes beside its files.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Manifest {
    turn: u64,
    at: String,
    files: usize,
    bytes: u64,
    unread: usize,
    entries: Vec<Entry>,
}

/// The store's root for one session: `<store>/<session>`.
pub fn session_dir(store: &Path, session: &str) -> PathBuf {
    store.join(session)
}

/// One checkpoint's directory: `<store>/<session>/<turn>`.
fn turn_dir(store: &Path, session: &str, turn: u64) -> PathBuf {
    session_dir(store, session).join(turn.to_string())
}

/// Capture the working tree at `root` as turn `turn`'s checkpoint. The
/// walk is depth-first in name order, so the cap's cut is deterministic
/// rather than whatever order the directory handed back.
pub fn capture(root: &Path, store: &Path, session: &str, turn: u64) -> std::io::Result<Snapshot> {
    let dir = turn_dir(store, session, turn);
    let files_dir = dir.join("files");
    std::fs::create_dir_all(&files_dir)?;
    let mut entries: Vec<Entry> = Vec::new();
    let mut bytes = 0u64;
    let mut unread = 0usize;
    walk(root, root, &mut |path, relative| {
        if entries.len() >= MAX_FILES || bytes >= MAX_BYTES {
            unread += 1;
            return Ok(());
        }
        let Ok(text) = std::fs::read(path) else {
            unread += 1;
            return Ok(());
        };
        bytes += text.len() as u64;
        let target = files_dir.join(&relative);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&target, &text)?;
        entries.push(Entry {
            path: relative,
            bytes: text.len() as u64,
        });
        Ok(())
    })?;
    let snapshot = Snapshot {
        turn,
        at: utc_stamp(),
        files: entries.len(),
        bytes,
        unread,
    };
    let manifest = Manifest {
        turn,
        at: snapshot.at.clone(),
        files: snapshot.files,
        bytes: snapshot.bytes,
        unread,
        entries,
    };
    std::fs::write(
        dir.join("manifest.json"),
        serde_json::to_string_pretty(&manifest_json(&manifest))
            .map_err(|error| std::io::Error::other(error.to_string()))?,
    )?;
    Ok(snapshot)
}

/// Restore the tree at `root` to turn `turn`'s checkpoint: the entries
/// copied back, and every file the manifest does not carry removed —
/// a file created after the checkpoint is exactly what a rewind undoes.
/// Returns `(entries restored, stray files removed)`.
pub fn restore(
    root: &Path,
    store: &Path,
    session: &str,
    turn: u64,
) -> std::io::Result<(usize, usize)> {
    let dir = turn_dir(store, session, turn);
    let manifest = read_manifest(&dir)?
        .ok_or_else(|| std::io::Error::other(format!("no checkpoint at turn {turn}")))?;
    let files_dir = dir.join("files");
    let mut kept: BTreeMap<String, ()> = BTreeMap::new();
    for entry in &manifest.entries {
        let from = files_dir.join(&entry.path);
        let to = root.join(&entry.path);
        if let Some(parent) = to.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(&from, &to)?;
        kept.insert(entry.path.clone(), ());
    }
    // The removal pass: every file under the root that the manifest does
    // not carry goes. The walk's own ignores apply, so a `.git` the
    // checkpoint never captured is never removed by a restore.
    let mut removed = 0usize;
    let mut strays: Vec<PathBuf> = Vec::new();
    walk(root, root, &mut |path, relative| {
        if !kept.contains_key(&relative) {
            strays.push(path.to_path_buf());
        }
        Ok(())
    })?;
    for stray in strays {
        if std::fs::remove_file(&stray).is_ok() {
            removed += 1;
        }
    }
    Ok((manifest.entries.len(), removed))
}

/// The checkpoints one session's store holds, newest first.
pub fn list(store: &Path, session: &str) -> Vec<Snapshot> {
    let dir = session_dir(store, session);
    let Ok(read) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut out: Vec<Snapshot> = read
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().is_dir())
        .filter_map(|entry| read_manifest(&entry.path()).ok().flatten())
        .map(|manifest| Snapshot {
            turn: manifest.turn,
            at: manifest.at,
            files: manifest.files,
            bytes: manifest.bytes,
            unread: manifest.unread,
        })
        .collect();
    out.sort_by(|left, right| right.turn.cmp(&left.turn));
    out
}

/// The manifest as the JSON file carries it.
fn manifest_json(manifest: &Manifest) -> serde_json::Value {
    serde_json::json!({
        "turn": manifest.turn,
        "at": manifest.at,
        "files": manifest.files,
        "bytes": manifest.bytes,
        "unread": manifest.unread,
        "entries": manifest.entries.iter().map(|entry| serde_json::json!({
            "path": entry.path,
            "bytes": entry.bytes,
        })).collect::<Vec<_>>(),
    })
}

/// The manifest as the file carries it, entries included — the restore
/// needs the entries, so the reader returns the whole manifest and the
/// listing maps it down to the summary.
fn read_manifest(dir: &Path) -> std::io::Result<Option<Manifest>> {
    let text = match std::fs::read_to_string(dir.join("manifest.json")) {
        Ok(text) => text,
        // An absent manifest is an absent checkpoint, which the caller
        // names — not an error the reader invents.
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    let value: serde_json::Value =
        serde_json::from_str(&text).map_err(|error| std::io::Error::other(error.to_string()))?;
    let entries = value
        .get("entries")
        .and_then(|entries| entries.as_array())
        .map(|entries| {
            entries
                .iter()
                .filter_map(|entry| {
                    Some(Entry {
                        path: entry.get("path")?.as_str()?.to_string(),
                        bytes: entry.get("bytes")?.as_u64()?,
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    Ok(Some(Manifest {
        turn: value
            .get("turn")
            .and_then(|turn| turn.as_u64())
            .unwrap_or(0),
        at: value
            .get("at")
            .and_then(|at| at.as_str())
            .unwrap_or_default()
            .to_string(),
        files: value.get("files").and_then(|f| f.as_u64()).unwrap_or(0) as usize,
        bytes: value.get("bytes").and_then(|b| b.as_u64()).unwrap_or(0),
        unread: value.get("unread").and_then(|u| u.as_u64()).unwrap_or(0) as usize,
        entries,
    }))
}

/// The walk both passes share: every file under `root`, its path
/// relative to the root, in name order, with the ignored directories
/// skipped. A read failure inside `visit` stops the walk with its
/// error — a checkpoint that silently skipped a file it could not copy
/// would be a snapshot that lies about what it holds.
fn walk(
    root: &Path,
    dir: &Path,
    visit: &mut dyn FnMut(&Path, String) -> std::io::Result<()>,
) -> std::io::Result<()> {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .collect();
    entries.sort();
    for path in entries {
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        if IGNORED.contains(&name.as_str()) {
            continue;
        }
        if path.is_dir() {
            walk(root, &path, visit)?;
            continue;
        }
        let relative = path
            .strip_prefix(root)
            .map_err(|error| std::io::Error::other(error.to_string()))?
            .to_string_lossy()
            .into_owned();
        visit(&path, relative)?;
    }
    Ok(())
}

/// The UTC stamp, seconds resolution, the store's own time vocabulary.
fn utc_stamp() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|span| span.as_secs())
        .unwrap_or(0);
    let days = secs / 86_400;
    let clock = secs % 86_400;
    let (year, month, day) = civil_from_days(days as i64);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        clock / 3_600,
        (clock % 3_600) / 60,
        clock % 60
    )
}

/// Days since the epoch to a civil date, the standard algorithm.
fn civil_from_days(days: i64) -> (i64, u64, u64) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u64;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u64;
    (year, month, day)
}

// --- the diff ---------------------------------------------------------------

/// How many lines of one file the line-diff will align. A file past it
/// is carried whole-file: the panel says the file is too large to diff
/// and offers the whole-file reject, which is honest where a partial
/// alignment would be a guess.
const MAX_DIFF_LINES: usize = 4_000;

/// One file's change against a checkpoint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileDiff {
    pub path: String,
    pub kind: DiffKind,
    /// The changed regions, oldest-side against newest-side. Empty for
    /// an added or removed file, which is one whole-file hunk the panel
    /// renders as such.
    pub hunks: Vec<Hunk>,
    /// True when the file was too large to align line by line.
    pub too_large: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffKind {
    Added,
    Removed,
    Modified,
}

/// One changed region: the lines the checkpoint held (`old`) against
/// the lines the tree holds now (`new`), with `new_at` the 0-based line
/// in the current file where `new` starts — the coordinate a reject
/// needs to apply the reverse.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hunk {
    pub old: Vec<String>,
    pub new: Vec<String>,
    pub new_at: usize,
}

/// The tree at `root` against turn `turn`'s checkpoint: added, removed,
/// and modified files, each modified file aligned line by line.
pub fn diff(root: &Path, store: &Path, session: &str, turn: u64) -> std::io::Result<Vec<FileDiff>> {
    let dir = turn_dir(store, session, turn);
    let Some(manifest) = read_manifest(&dir)? else {
        return Err(std::io::Error::other(format!(
            "no checkpoint at turn {turn}"
        )));
    };
    let files_dir = dir.join("files");
    let mut out: Vec<FileDiff> = Vec::new();
    let mut seen: BTreeMap<String, ()> = BTreeMap::new();
    for entry in &manifest.entries {
        seen.insert(entry.path.clone(), ());
        let before = std::fs::read(files_dir.join(&entry.path)).unwrap_or_default();
        let after = std::fs::read(root.join(&entry.path)).unwrap_or_default();
        if after.is_empty() && !root.join(&entry.path).exists() {
            out.push(FileDiff {
                path: entry.path.clone(),
                kind: DiffKind::Removed,
                hunks: Vec::new(),
                too_large: false,
            });
            continue;
        }
        if before == after {
            continue;
        }
        let (hunks, too_large) = align(&before, &after);
        out.push(FileDiff {
            path: entry.path.clone(),
            kind: DiffKind::Modified,
            hunks,
            too_large,
        });
    }
    // The files the tree carries that the checkpoint never saw.
    let mut added: Vec<String> = Vec::new();
    walk(root, root, &mut |_path, relative| {
        if !seen.contains_key(&relative) {
            added.push(relative);
        }
        Ok(())
    })?;
    for path in added {
        out.push(FileDiff {
            path,
            kind: DiffKind::Added,
            hunks: Vec::new(),
            too_large: false,
        });
    }
    out.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(out)
}

/// Align two versions of one file into hunks: the classic longest-
/// common-subsequence walk over lines, then the changed runs between
/// the kept lines. `too_large` is set instead of a partial alignment
/// when either side is past [`MAX_DIFF_LINES`].
fn align(before: &[u8], after: &[u8]) -> (Vec<Hunk>, bool) {
    let old_lines: Vec<String> = lines(before);
    let new_lines: Vec<String> = lines(after);
    if old_lines.len() > MAX_DIFF_LINES || new_lines.len() > MAX_DIFF_LINES {
        return (Vec::new(), true);
    }
    // The LCS table, rows the old side and columns the new side.
    let rows = old_lines.len();
    let cols = new_lines.len();
    let mut table = vec![0u32; (rows + 1) * (cols + 1)];
    for i in (0..rows).rev() {
        for j in (0..cols).rev() {
            table[i * (cols + 1) + j] = if old_lines[i] == new_lines[j] {
                table[(i + 1) * (cols + 1) + j + 1] + 1
            } else {
                table[(i + 1) * (cols + 1) + j].max(table[i * (cols + 1) + j + 1])
            };
        }
    }
    // The walk emits the changed runs between the matched lines.
    let mut hunks: Vec<Hunk> = Vec::new();
    let mut run_old: Vec<String> = Vec::new();
    let mut run_new: Vec<String> = Vec::new();
    let mut run_at = 0usize;
    let (mut i, mut j) = (0usize, 0usize);
    while i < rows && j < cols {
        if old_lines[i] == new_lines[j] {
            if !run_old.is_empty() || !run_new.is_empty() {
                hunks.push(Hunk {
                    old: std::mem::take(&mut run_old),
                    new: std::mem::take(&mut run_new),
                    new_at: run_at,
                });
            }
            i += 1;
            j += 1;
        } else if table[(i + 1) * (cols + 1) + j] >= table[i * (cols + 1) + j + 1] {
            run_old.push(old_lines[i].clone());
            i += 1;
        } else {
            if run_new.is_empty() {
                run_at = j;
            }
            run_new.push(new_lines[j].clone());
            j += 1;
        }
    }
    while i < rows {
        run_old.push(old_lines[i].clone());
        i += 1;
    }
    while j < cols {
        if run_new.is_empty() {
            run_at = j;
        }
        run_new.push(new_lines[j].clone());
        j += 1;
    }
    if !run_old.is_empty() || !run_new.is_empty() {
        hunks.push(Hunk {
            old: run_old,
            new: run_new,
            new_at: run_at,
        });
    }
    (hunks, false)
}

fn lines(bytes: &[u8]) -> Vec<String> {
    String::from_utf8_lossy(bytes)
        .lines()
        .map(str::to_string)
        .collect()
}

/// Reject one hunk: put the checkpoint's lines back where the run's
/// lines went. The hunk carries its own coordinate in the current file,
/// so a reject is the reverse of exactly what the hunk changed and
/// nothing else — a second reject of a neighbour hunk still applies.
pub fn reject_hunk(root: &Path, file: &FileDiff, hunk: &Hunk) -> std::io::Result<()> {
    let path = root.join(&file.path);
    let text = String::from_utf8_lossy(&std::fs::read(&path)?).into_owned();
    let mut current: Vec<String> = text.lines().map(str::to_string).collect();
    let end = (hunk.new_at + hunk.new.len()).min(current.len());
    if end < hunk.new_at {
        return Err(std::io::Error::other("the hunk is past the file’s end"));
    }
    current.splice(hunk.new_at..end, hunk.old.iter().cloned());
    let mut written = current.join("\n");
    if text.ends_with('\n') || current.is_empty() {
        written.push('\n');
    }
    std::fs::write(&path, written)
}

/// Reject a whole file: the checkpoint's bytes, or removal when the
/// checkpoint never held the file. The path is contained by construction
/// — every diff entry is a manifest path or a walk-relative path.
pub fn reject_file(
    root: &Path,
    store: &Path,
    session: &str,
    turn: u64,
    path: &str,
) -> std::io::Result<()> {
    let stored = turn_dir(store, session, turn).join("files").join(path);
    let target = root.join(path);
    if stored.exists() {
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(&stored, &target)?;
        Ok(())
    } else if target.exists() {
        std::fs::remove_file(&target)
    } else {
        // Neither side holds the file: a reject of a path the diff does
        // not carry names itself rather than reporting a restore of
        // nothing.
        Err(std::io::Error::other(format!(
            "`{path}` carries no change against this checkpoint"
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(tag: &str) -> (PathBuf, PathBuf) {
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let serial = NEXT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let root =
            std::env::temp_dir().join(format!("fleet-ckpt-{tag}-{}-{serial}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let tree = root.join("tree");
        let store = root.join("store");
        std::fs::create_dir_all(&tree).unwrap();
        std::fs::create_dir_all(&store).unwrap();
        (tree, store)
    }

    #[test]
    fn a_capture_copies_the_tree_and_names_its_bounds() {
        let (tree, store) = scratch("capture");
        std::fs::write(tree.join("a.txt"), "one\n").unwrap();
        std::fs::create_dir_all(tree.join("nested")).unwrap();
        std::fs::write(tree.join("nested/b.txt"), "two\n").unwrap();
        // A directory the checkpoint never enters.
        std::fs::create_dir_all(tree.join(".git")).unwrap();
        std::fs::write(tree.join(".git/HEAD"), "ref\n").unwrap();

        let snapshot = capture(&tree, &store, "s-1", 3).unwrap();
        assert_eq!(snapshot.turn, 3);
        assert_eq!(snapshot.files, 2, "{snapshot:?}");
        assert_eq!(snapshot.bytes, 8, "{snapshot:?}");
        assert_eq!(snapshot.unread, 0);
        assert!(store.join("s-1/3/files/a.txt").exists());
        assert!(store.join("s-1/3/files/nested/b.txt").exists());
        assert!(!store.join("s-1/3/files/.git").exists());
        let listed = list(&store, "s-1");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0], snapshot);
        let _ = std::fs::remove_dir_all(tree.parent().unwrap());
    }

    #[test]
    fn a_restore_makes_the_tree_match_the_checkpoint() {
        let (tree, store) = scratch("restore");
        std::fs::write(tree.join("kept.txt"), "before\n").unwrap();
        std::fs::write(tree.join("changed.txt"), "before\n").unwrap();
        capture(&tree, &store, "s-1", 1).unwrap();
        // The turn's writes: one file changed, one created.
        std::fs::write(tree.join("changed.txt"), "after\n").unwrap();
        std::fs::write(tree.join("created.txt"), "new\n").unwrap();

        restore(&tree, &store, "s-1", 1).unwrap();
        assert_eq!(
            std::fs::read_to_string(tree.join("kept.txt")).unwrap(),
            "before\n"
        );
        assert_eq!(
            std::fs::read_to_string(tree.join("changed.txt")).unwrap(),
            "before\n",
            "a changed file is restored"
        );
        assert!(
            !tree.join("created.txt").exists(),
            "a file the checkpoint never saw is removed"
        );
        let _ = std::fs::remove_dir_all(tree.parent().unwrap());
    }

    #[test]
    fn a_restore_of_an_absent_checkpoint_is_a_named_refusal() {
        let (tree, store) = scratch("absent");
        let error = restore(&tree, &store, "s-1", 9).unwrap_err();
        assert!(
            error.to_string().contains("no checkpoint at turn 9"),
            "{error}"
        );
        let _ = std::fs::remove_dir_all(tree.parent().unwrap());
    }
}
