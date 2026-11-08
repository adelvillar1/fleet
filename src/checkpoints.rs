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
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// How many files one checkpoint captures. A tree past it captures its
/// first `MAX_FILES` in walk order and names the rest.
const MAX_FILES: usize = 2_000;

/// How many bytes one checkpoint captures in total.
const MAX_BYTES: u64 = 64 * 1024 * 1024;

/// The retention the hygiene pass enforces: each session keeps its last
/// twenty turns, and the store keeps its newest 5 GiB — the numbers the
/// 2026-11-06 manual sweep used when 72 G of per-turn snapshots filled
/// the drive and the gate died on ENOSPC.
pub const KEEP_TURNS: usize = 20;
pub const STORE_CAP_BYTES: u64 = 5 * 1024 * 1024 * 1024;

/// How often the store reclaims while a face is running. A turn's
/// capture is up to 64 MB, so a long session writes that much per turn
/// for as long as it is open; a quarter of an hour bounds what an open
/// face can hold without ever letting a session grow past its keep.
pub const RECLAIM_INTERVAL: Duration = Duration::from_secs(15 * 60);

/// The retention, run on a timer for the life of the process.
///
/// The policy was never the defect — [`prune`] is bounded, tested and
/// old. Its *trigger* was: the two places that called it were a manual
/// verb and a session's goal completing, and a face that runs for days
/// without a goal set never reaches either. So the store grew to 68 GB
/// across 72 sessions, two of them holding 54 GB between them, while
/// the retention that would have kept each at 20 turns never ran once.
/// A harness that mails back everything it ever wrote is not being
/// careful; it is squatting on the drive (KI-478).
///
/// Two rules the timer keeps, both learned from the compaction
/// operations that eat their own tail:
///
/// - **Never the current turn.** The newest checkpoint of every session
///   is what a face's rewind button points at, so the sweep keeps it
///   whatever the caps say — [`prune_live`] and not the plain cap.
/// - **Never mid-capture.** A capture writes a manifest beside its
///   files and the manifest is written last, so a turn directory
///   without one is a capture in flight and is skipped rather than
///   half-deleted under the run that is writing it.
///
/// The pass is quiet when it reclaims nothing, and names what it freed
/// when it does: the debris is the owner's disk, so the report is the
/// owner's.
pub fn spawn_reclaim(store: PathBuf) {
    std::thread::spawn(move || loop {
        let (removed, reclaimed) = prune_live(&store, KEEP_TURNS, STORE_CAP_BYTES);
        if removed > 0 {
            eprintln!(
                "fleet: checkpoints: {removed} turn(s) released, {} reclaimed \u{2014} \
                 each session keeps its last {KEEP_TURNS} turns under a {STORE_CAP_BYTES} byte cap",
                human_bytes(reclaimed),
            );
        }
        std::thread::sleep(RECLAIM_INTERVAL);
    });
}

/// A byte count in the unit that reads honestly. A pass that released
/// five 200 KB turns freed a megabyte, and reporting it as "0 MB" would
/// say it freed nothing when it freed all of it.
fn human_bytes(bytes: u64) -> String {
    const KIB: u64 = 1024;
    const MIB: u64 = 1024 * 1024;
    const GIB: u64 = 1024 * 1024 * 1024;
    if bytes >= GIB {
        format!("{} GiB", bytes / GIB)
    } else if bytes >= MIB {
        format!("{} MiB", bytes / MIB)
    } else if bytes >= KIB {
        format!("{} KiB", bytes / KIB)
    } else {
        format!("{bytes} bytes")
    }
}

/// [`prune`] for a store a run is writing to: a turn still being
/// captured is not seen, and every session keeps the turn its rewind
/// points at.
///
/// The caps are the same ones; only the eviction order changes. The
/// per-session keep is `keep_turns`, and the store's byte cap is
/// measured over the turns that may really go — so a store holding
/// nothing but each session's newest turn reports nothing rather than
/// spending the pass on what it must not free.
pub fn prune_live(store: &Path, keep_turns: usize, max_bytes: u64) -> (usize, u64) {
    let mut removed = 0usize;
    let mut reclaimed = 0u64;
    // `(modified, dir, bytes, is_a_sessions_newest)`. The newest turn of
    // a session is sorted last and never evicted: it is the checkpoint a
    // face's rewind button points at, and the one the run that is live
    // just wrote.
    let mut survivors: Vec<(SystemTime, PathBuf, u64, bool)> = Vec::new();
    let Ok(sessions) = std::fs::read_dir(store) else {
        return (0, 0);
    };
    for sess in sessions.flatten() {
        let sess_path = sess.path();
        if !sess_path.is_dir() {
            continue;
        }
        // A turn counts only once its manifest landed: the capture
        // writes the manifest after the files, so a directory without
        // one is a capture still writing and is left alone.
        let mut ids: Vec<u64> = std::fs::read_dir(&sess_path)
            .into_iter()
            .flatten()
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| {
                let path = entry.path();
                if !path.is_dir() || !path.join("manifest.json").exists() {
                    return None;
                }
                entry
                    .file_name()
                    .to_str()
                    .and_then(|name| name.parse::<u64>().ok())
            })
            .collect();
        ids.sort_unstable();
        let Some(&newest) = ids.last() else {
            continue;
        };
        let keep_from = ids.len().saturating_sub(keep_turns);
        for (index, id) in ids.iter().enumerate() {
            let dir = sess_path.join(id.to_string());
            let is_newest = *id == newest;
            if index < keep_from && !is_newest {
                let size = dir_size(&dir);
                if std::fs::remove_dir_all(&dir).is_ok() {
                    removed += 1;
                    reclaimed += size;
                }
                continue;
            }
            let modified = std::fs::metadata(&dir)
                .and_then(|meta| meta.modified())
                .unwrap_or(SystemTime::UNIX_EPOCH);
            let size = dir_size(&dir);
            survivors.push((modified, dir, size, is_newest));
        }
    }
    // The store's cap: the oldest turns go first, and a session's newest
    // turn is never something the cap may spend.
    survivors.sort_by_key(|(modified, _, _, is_newest)| (*is_newest, *modified));
    let mut total: u64 = survivors
        .iter()
        .filter(|(_, _, _, is_newest)| !*is_newest)
        .map(|(_, _, size, _)| size)
        .sum();
    for (_, dir, size, is_newest) in &survivors {
        if total <= max_bytes {
            break;
        }
        if *is_newest {
            continue;
        }
        if std::fs::remove_dir_all(dir).is_ok() {
            removed += 1;
            reclaimed += size;
            total = total.saturating_sub(*size);
        }
    }
    (removed, reclaimed)
}

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
    out.sort_by_key(|one| std::cmp::Reverse(one.turn));
    out
}

/// The retention: every session keeps its last `keep_turns` turns, and
/// the store keeps its newest `max_bytes` overall, the oldest turns
/// going first. Returns `(turn directories removed, bytes reclaimed)`.
/// A directory already gone is not an error — the prune races nothing
/// that matters.
pub fn prune(store: &Path, keep_turns: usize, max_bytes: u64) -> (usize, u64) {
    let mut removed = 0usize;
    let mut reclaimed = 0u64;
    let mut survivors: Vec<(std::time::SystemTime, PathBuf, u64)> = Vec::new();
    let Ok(sessions) = std::fs::read_dir(store) else {
        return (0, 0);
    };
    for sess in sessions.flatten() {
        let sess_path = sess.path();
        if !sess_path.is_dir() {
            continue;
        }
        let mut ids: Vec<u64> = std::fs::read_dir(&sess_path)
            .into_iter()
            .flatten()
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| {
                entry
                    .file_name()
                    .to_str()
                    .and_then(|name| name.parse::<u64>().ok())
            })
            .collect();
        ids.sort_unstable();
        let keep_from = ids.len().saturating_sub(keep_turns);
        for (index, id) in ids.iter().enumerate() {
            let dir = sess_path.join(id.to_string());
            if index < keep_from {
                let size = dir_size(&dir);
                if std::fs::remove_dir_all(&dir).is_ok() {
                    removed += 1;
                    reclaimed += size;
                }
                continue;
            }
            let modified = std::fs::metadata(&dir)
                .and_then(|meta| meta.modified())
                .unwrap_or(std::time::UNIX_EPOCH);
            let size = dir_size(&dir);
            survivors.push((modified, dir, size));
        }
    }
    // The store's cap: the oldest surviving turns go first, so a long
    // session cannot hold the drive hostage a second time (the
    // 2026-11-06 incident: 72 G of per-turn snapshots, nothing pruning).
    survivors.sort_by_key(|(modified, _, _)| *modified);
    let mut total: u64 = survivors.iter().map(|(_, _, size)| size).sum();
    for (_, dir, size) in &survivors {
        if total <= max_bytes {
            break;
        }
        if std::fs::remove_dir_all(dir).is_ok() {
            removed += 1;
            reclaimed += size;
            total = total.saturating_sub(*size);
        }
    }
    (removed, reclaimed)
}

/// One directory's recursive size. The checkpoints' own caps bound a
/// turn to 64 MB, so the walk is cheap; a directory that vanished
/// mid-walk contributes nothing.
fn dir_size(dir: &Path) -> u64 {
    let mut total = 0u64;
    let Ok(read) = std::fs::read_dir(dir) else {
        return 0;
    };
    for entry in read.flatten() {
        let path = entry.path();
        if path.is_dir() {
            total += dir_size(&path);
        } else if let Ok(meta) = entry.metadata() {
            total += meta.len();
        }
    }
    total
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
    /// The unchanged lines around the change, up to three each way and
    /// **on each side**: the old side for an accept, the new side for a
    /// reject. They are the anchor a verdict splices against, because a
    /// neighbour's verdict moves every line number below it — a splice
    /// at `new_at` lands in the wrong place the moment anything above
    /// the hunk is put back. Each side's context stops short of a
    /// neighbouring hunk's lines, so the anchor survives whatever the
    /// owner decides about its neighbours, in whatever order.
    pub before_old: Vec<String>,
    pub after_old: Vec<String>,
    pub before_new: Vec<String>,
    pub after_new: Vec<String>,
}

/// How many context lines a hunk carries on each side.
const HUNK_CONTEXT: usize = 3;

/// The context for one run, on one side: up to [`HUNK_CONTEXT`] lines
/// before it and after it, trimmed to the matched lines between it and
/// its neighbours — a neighbour's changed lines are never part of this
/// hunk's anchor, because a verdict on the neighbour would move them.
fn side_context(
    lines: &[String],
    start: usize,
    end: usize,
    floor: usize,
    ceiling: usize,
) -> (Vec<String>, Vec<String>) {
    let before_count = (start - floor).min(HUNK_CONTEXT);
    let before = lines[start - before_count..start].to_vec();
    let after_count = (ceiling - end).min(HUNK_CONTEXT);
    let after = lines[end..end + after_count].to_vec();
    (before, after)
}

/// Which side of a hunk to find in the current file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Side {
    Old,
    New,
}

impl Hunk {
    fn side(&self, side: Side) -> &[String] {
        match side {
            Side::Old => &self.old,
            Side::New => &self.new,
        }
    }

    fn context(&self, side: Side) -> (&[String], &[String]) {
        match side {
            Side::Old => (&self.before_old, &self.after_old),
            Side::New => (&self.before_new, &self.after_new),
        }
    }
}

/// Find the hunk's block between its recorded context lines. `Ok(None)`
/// when the block is not there, the index of the block's first line when
/// it is there exactly once, and an error when the context is ambiguous —
/// two places a splice could land is a file this function will not guess
/// about.
fn locate(
    current: &[String],
    hunk: &Hunk,
    side: Side,
    path: &str,
) -> std::io::Result<Option<usize>> {
    let block = hunk.side(side);
    let (before, after) = hunk.context(side);
    let mut pattern: Vec<&String> = before.iter().collect();
    pattern.extend(block.iter());
    pattern.extend(after.iter());
    if pattern.is_empty() {
        return Ok(None);
    }
    let mut found: Option<usize> = None;
    if current.len() >= pattern.len() {
        for start in 0..=current.len() - pattern.len() {
            let holds = pattern
                .iter()
                .enumerate()
                .all(|(offset, line)| &current[start + offset] == *line);
            if holds {
                if found.is_some() {
                    return Err(std::io::Error::other(format!(
                        "the hunk's context appears more than once in `{path}` — the file cannot say which place the review meant"
                    )));
                }
                found = Some(start + before.len());
            }
        }
    }
    Ok(found)
}

/// Write `current` back with the file's own trailing-newline shape kept.
fn write_lines(path: &Path, current: &[String], had_newline: bool) -> std::io::Result<()> {
    let mut written = current.join("\n");
    if had_newline || current.is_empty() {
        written.push('\n');
    }
    std::fs::write(path, written)
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
    // The walk emits the changed runs between the matched lines. Each
    // run keeps its span on both sides, so the context a verdict anchors
    // on can be trimmed to the *matched* lines around it — a neighbour's
    // changed lines never enter this hunk's anchor, because a verdict on
    // the neighbour would move them.
    struct Run {
        old: Vec<String>,
        new: Vec<String>,
        start_old: usize,
        start_new: usize,
        end_old: usize,
        end_new: usize,
    }
    let mut runs: Vec<Run> = Vec::new();
    let mut run_old: Vec<String> = Vec::new();
    let mut run_new: Vec<String> = Vec::new();
    let mut run_start: Option<(usize, usize)> = None;
    let begin = |i: usize, j: usize, start: &mut Option<(usize, usize)>| {
        if start.is_none() {
            *start = Some((i, j));
        }
    };
    let (mut i, mut j) = (0usize, 0usize);
    while i < rows && j < cols {
        if old_lines[i] == new_lines[j] {
            if let Some((start_old, start_new)) = run_start.take() {
                runs.push(Run {
                    old: std::mem::take(&mut run_old),
                    new: std::mem::take(&mut run_new),
                    start_old,
                    start_new,
                    end_old: i,
                    end_new: j,
                });
            }
            i += 1;
            j += 1;
        } else if table[(i + 1) * (cols + 1) + j] >= table[i * (cols + 1) + j + 1] {
            begin(i, j, &mut run_start);
            run_old.push(old_lines[i].clone());
            i += 1;
        } else {
            begin(i, j, &mut run_start);
            run_new.push(new_lines[j].clone());
            j += 1;
        }
    }
    while i < rows {
        begin(i, j, &mut run_start);
        run_old.push(old_lines[i].clone());
        i += 1;
    }
    while j < cols {
        begin(i, j, &mut run_start);
        run_new.push(new_lines[j].clone());
        j += 1;
    }
    if let Some((start_old, start_new)) = run_start.take() {
        runs.push(Run {
            old: run_old,
            new: run_new,
            start_old,
            start_new,
            end_old: rows,
            end_new: cols,
        });
    }
    let mut hunks: Vec<Hunk> = Vec::with_capacity(runs.len());
    for (index, run) in runs.iter().enumerate() {
        let prev = index.checked_sub(1).map(|at| &runs[at]);
        let next = runs.get(index + 1);
        let floor_old = prev.map(|run| run.end_old).unwrap_or(0);
        let floor_new = prev.map(|run| run.end_new).unwrap_or(0);
        let ceiling_old = next.map(|run| run.start_old).unwrap_or(rows);
        let ceiling_new = next.map(|run| run.start_new).unwrap_or(cols);
        let (before_old, after_old) = side_context(
            &old_lines,
            run.start_old,
            run.end_old,
            floor_old,
            ceiling_old,
        );
        let (before_new, after_new) = side_context(
            &new_lines,
            run.start_new,
            run.end_new,
            floor_new,
            ceiling_new,
        );
        hunks.push(Hunk {
            old: run.old.clone(),
            new: run.new.clone(),
            new_at: run.start_new,
            before_old,
            after_old,
            before_new,
            after_new,
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
    // Anchored on the hunk's own context, not on `new_at`: a neighbour's
    // reject moves every line after it, and a blind splice at a stale
    // coordinate corrupts the file silently (the cheap fix this
    // replaces). The block is looked up where it was and refused when it
    // is not there or is there twice.
    let Some(at) = locate(&current, hunk, Side::New, &file.path)? else {
        // Already rejected: the checkpoint's lines are where the run's
        // were. A second press is a no-op, not a second splice.
        if locate(&current, hunk, Side::Old, &file.path)?.is_some() {
            return Ok(());
        }
        return Err(std::io::Error::other(format!(
            "`{}` no longer carries hunk at line {} — the file changed since the review",
            file.path,
            hunk.new_at + 1
        )));
    };
    current.splice(at..at + hunk.new.len(), hunk.old.iter().cloned());
    write_lines(&path, &current, text.ends_with('\n'))
}

/// Accept one hunk: put the run's lines back where a reject left the
/// checkpoint's — the inverse of [`reject_hunk`] over the same
/// coordinate, so a hunk rejected by mistake is accepted again without
/// touching its neighbours (gui-revamp C13).
pub fn accept_hunk(root: &Path, file: &str, hunk: &Hunk) -> std::io::Result<()> {
    let path = root.join(file);
    let text = String::from_utf8_lossy(&std::fs::read(&path)?).into_owned();
    let mut current: Vec<String> = text.lines().map(str::to_string).collect();
    // Already accepted: the run's lines are where the checkpoint's were.
    if locate(&current, hunk, Side::New, file)?.is_some() {
        return Ok(());
    }
    let Some(at) = locate(&current, hunk, Side::Old, file)? else {
        return Err(std::io::Error::other(format!(
            "`{file}` carries neither side of the reviewed hunk — the file changed since the review"
        )));
    };
    current.splice(at..at + hunk.old.len(), hunk.new.iter().cloned());
    write_lines(&path, &current, text.ends_with('\n'))
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
    fn prune_keeps_each_sessions_last_turns_and_the_stores_cap() {
        let (tree, store) = scratch("prune");
        std::fs::write(tree.join("a.txt"), "one\n").unwrap();
        for turn in 1..=25u64 {
            capture(&tree, &store, "s-1", turn).unwrap();
        }
        for turn in 1..=10u64 {
            capture(&tree, &store, "s-2", turn).unwrap();
        }
        let (removed, _) = prune(&store, 20, u64::MAX);
        // s-1 keeps 6..=25 (twenty of twenty-five); s-2 keeps everything,
        // because ten turns is inside the per-session keep.
        assert_eq!(removed, 5, "the oldest five of s-1 go");
        assert_eq!(list(&store, "s-1").len(), 20);
        assert_eq!(list(&store, "s-2").len(), 10);
        assert!(!store.join("s-1/5").exists());
        assert!(store.join("s-1/6").exists());
        // The store's cap: a total the survivors bust drops the oldest
        // turns first, across sessions — the 72 G incident's cure.
        let (removed2, _) = prune(&store, 20, 1);
        assert_eq!(removed2, 30, "a one-byte cap keeps nothing");
        assert_eq!(list(&store, "s-1").len(), 0);
        assert_eq!(list(&store, "s-2").len(), 0);
        let _ = std::fs::remove_dir_all(tree.parent().unwrap());
    }

    /// The timer's retention must never take the checkpoint a rewind
    /// points at: a face whose store was trimmed past its newest turn
    /// would offer a rewind to a turn that no longer exists. The plain
    /// `prune` may empty a store; the live one may not, at any cap.
    #[test]
    fn live_prune_keeps_every_sessions_newest_turn_at_any_cap() {
        let (tree, store) = scratch("prune-live");
        std::fs::write(tree.join("a.txt"), "one\n").unwrap();
        for turn in 1..=25u64 {
            capture(&tree, &store, "s-1", turn).unwrap();
        }
        for turn in 1..=3u64 {
            capture(&tree, &store, "s-2", turn).unwrap();
        }
        // The per-session keep still holds: 25 turns trim to 20.
        let (removed, _) = prune_live(&store, 20, u64::MAX);
        assert_eq!(removed, 5, "the oldest five of s-1 go");
        assert_eq!(list(&store, "s-1").len(), 20);
        // A one-byte cap would have `prune` empty both sessions. The live
        // pass keeps each session's newest turn and nothing else, which
        // is the invariant the rewind button rests on.
        let (removed2, _) = prune_live(&store, 20, 0);
        // Twenty on s-1 plus three on s-2 survive the keep; one each stays.
        assert_eq!(removed2, 21, "every turn but the two newest went");
        let kept = list(&store, "s-1");
        assert_eq!(kept.len(), 1, "one turn survives on s-1");
        assert_eq!(kept[0].turn, 25, "and it is the newest");
        assert_eq!(list(&store, "s-2").len(), 1);
        // A capture still writing has no manifest yet: it is not a turn
        // the pass may see, let alone delete.
        let inflight = store.join("s-1").join("26");
        std::fs::create_dir_all(inflight.join("files")).unwrap();
        std::fs::write(inflight.join("files/partial.bin"), vec![0u8; 4096]).unwrap();
        let (removed3, _) = prune_live(&store, 20, 0);
        assert_eq!(removed3, 0, "an in-flight capture is invisible to the pass");
        assert!(inflight.exists(), "and it is still there for its run");
        let _ = std::fs::remove_dir_all(tree.parent().unwrap());
    }

    /// A file with two changed regions, four lines each, so a hunk has
    /// real context on both sides and the second hunk sits below the
    /// first (the ordering the anchored verbs are for).
    fn two_region_file(tree: &Path, store: &Path) -> FileDiff {
        let before = "one\ntwo\nthree\nfour\nfive\nsix\nseven\neight\nnine\nten\n";
        let after = "ONE\ntwo\nthree\nfour\nfive\nsix\nseven\neight\nNINE\nten\n";
        std::fs::write(tree.join("f.txt"), before).unwrap();
        capture(tree, store, "s-1", 1).unwrap();
        std::fs::write(tree.join("f.txt"), after).unwrap();
        let files = diff(tree, store, "s-1", 1).unwrap();
        files.into_iter().find(|file| file.path == "f.txt").unwrap()
    }

    #[test]
    fn a_rejected_hunk_is_accepted_again_by_its_context() {
        let (tree, store) = scratch("accept");
        let file = two_region_file(&tree, &store);
        assert_eq!(file.hunks.len(), 2, "{file:?}");
        reject_hunk(&tree, &file, &file.hunks[0]).unwrap();
        assert_eq!(
            std::fs::read_to_string(tree.join("f.txt")).unwrap(),
            "one\ntwo\nthree\nfour\nfive\nsix\nseven\neight\nNINE\nten\n"
        );
        accept_hunk(&tree, &file.path, &file.hunks[0]).unwrap();
        assert_eq!(
            std::fs::read_to_string(tree.join("f.txt")).unwrap(),
            "ONE\ntwo\nthree\nfour\nfive\nsix\nseven\neight\nNINE\nten\n",
            "the accept puts the run's lines back"
        );
        // A second press is a no-op, not a second splice.
        accept_hunk(&tree, &file.path, &file.hunks[0]).unwrap();
        assert_eq!(
            std::fs::read_to_string(tree.join("f.txt")).unwrap(),
            "ONE\ntwo\nthree\nfour\nfive\nsix\nseven\neight\nNINE\nten\n"
        );
        let _ = std::fs::remove_dir_all(tree.parent().unwrap());
    }

    #[test]
    fn a_neighbours_reject_does_not_move_a_later_verdict() {
        // The blind-splice defect: rejecting the first hunk moves every
        // line below it, so a second verdict addressed by line number
        // lands in the wrong place. Anchored on context, the order of
        // verdicts does not matter.
        let (tree, store) = scratch("order");
        let file = two_region_file(&tree, &store);
        reject_hunk(&tree, &file, &file.hunks[0]).unwrap();
        reject_hunk(&tree, &file, &file.hunks[1]).unwrap();
        assert_eq!(
            std::fs::read_to_string(tree.join("f.txt")).unwrap(),
            "one\ntwo\nthree\nfour\nfive\nsix\nseven\neight\nnine\nten\n",
            "both regions are back"
        );
        accept_hunk(&tree, &file.path, &file.hunks[1]).unwrap();
        assert_eq!(
            std::fs::read_to_string(tree.join("f.txt")).unwrap(),
            "one\ntwo\nthree\nfour\nfive\nsix\nseven\neight\nNINE\nten\n",
            "the second region is the run's again, the first is not"
        );
        let _ = std::fs::remove_dir_all(tree.parent().unwrap());
    }

    #[test]
    fn a_file_that_moved_under_the_review_is_refused_by_name() {
        let (tree, store) = scratch("moved");
        let file = two_region_file(&tree, &store);
        reject_hunk(&tree, &file, &file.hunks[0]).unwrap();
        // Someone edits a line the hunk's own context names ("two" is
        // the line just after the change): the block and its context
        // are no longer in the file, so the verb refuses rather than
        // splicing at a coordinate it can no longer trust.
        std::fs::write(
            tree.join("f.txt"),
            "one\nTWO\nthree\nfour\nfive\nsix\nseven\neight\nNINE\nten\n",
        )
        .unwrap();
        let refused =
            accept_hunk(&tree, &file.path, &file.hunks[0]).expect_err("the moved file refuses");
        assert!(
            refused.to_string().contains("f.txt"),
            "the refusal names the file: {refused}"
        );
        let _ = std::fs::remove_dir_all(tree.parent().unwrap());
    }

    #[test]
    fn an_ambiguous_context_is_refused_rather_than_guessed() {
        // The same block and its context twice in one file: two places a
        // splice could land is a file the verb will not guess about.
        let (tree, store) = scratch("ambiguous");
        let pattern = "alpha\nbeta\ngamma\ndelta\n";
        std::fs::write(tree.join("f.txt"), format!("{pattern}{pattern}")).unwrap();
        capture(&tree, &store, "s-1", 1).unwrap();
        std::fs::write(
            tree.join("f.txt"),
            format!("{pattern}{pattern}").replace("beta", "BETA"),
        )
        .unwrap();
        let files = diff(&tree, &store, "s-1", 1).unwrap();
        let file = files.iter().find(|file| file.path == "f.txt").unwrap();
        // Two identical regions change identically: the hunk's own lines
        // appear twice, and the verb says so.
        let twin = Hunk {
            before_old: Vec::new(),
            after_old: Vec::new(),
            before_new: Vec::new(),
            after_new: Vec::new(),
            old: vec!["beta".to_string()],
            new: vec!["BETA".to_string()],
            new_at: 1,
        };
        let refused = reject_hunk(&tree, file, &twin).expect_err("the twin is ambiguous");
        assert!(
            refused.to_string().contains("more than once"),
            "the refusal names the ambiguity: {refused}"
        );
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
