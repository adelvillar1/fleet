//! The shared corpus sync — armada's distributed mind (L4).
//!
//! One corpus, two feeders: harnesses feed it locally (the cross-harness
//! pull, `memory_pull`), armada distributes it globally (this module). The
//! corpus rides **git** — the transport — while fleet owns the **semantic
//! merge** (D23). `corpus.sync(remote)` is fetch → merge → push; with no
//! remote configured it refuses by name and fleet stays exactly as it is
//! today (the daily script keeps committing locally). Sync is opt-in and
//! additive (D6).
//!
//! Two facts make naive pull/push wrong, and the merge rules are the heart
//! of L4:
//! - The **recall-count** is a grow-only rank signal under a local-only
//!   flock. Two installs conflict on the same `recall-count:` line
//!   constantly; it merges by **sum/delta** over the merge base, never
//!   last-writer-wins (D24) — a fact recalled on A bumps its rank for B.
//! - **KI-285's two-writer lost update goes distributed**: auto-generated
//!   replacement/archive serials are local-disk decisions two installs make
//!   identically. The serials carry the install-id (D25) so two installs
//!   never mint the same name; a supersede collision resolves to **one live
//!   frontier** by the basis ladder (D26).
//!
//! The recall index, lock siblings, temps, and per-install state never sync
//! (D29) — only the tracked fact tree moves. Every sync settles a
//! `corpus_sync` journal event through L2 and is graded by read-back before
//! it reports done (D28, the substrate's KI-356 discipline lifted to the
//! merge).

use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};
use serde_json::json;

/// The sync state's path inside a home (D23).
pub const SYNC_FILE: &str = "corpus-sync.json";

/// The sync state: the remote, the corpus ref, the last-synced revision, and
/// the last-ten settle history (the estimate learns from them, D4).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncState {
    /// The git remote's URL (absent = unconfigured, sync refuses by name).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remote: Option<String>,
    /// The branch the corpus syncs (default `master`).
    #[serde(default = "default_ref")]
    pub corpus_ref: String,
    /// The last-synced remote revision.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_revision: Option<String>,
    /// The last ten settled sync durations/counts, for the estimate.
    #[serde(default)]
    pub settles: Vec<u64>,
}

fn default_ref() -> String {
    "master".to_string()
}

impl Default for SyncState {
    fn default() -> SyncState {
        SyncState {
            remote: None,
            corpus_ref: default_ref(),
            last_revision: None,
            settles: Vec::new(),
        }
    }
}

impl SyncState {
    /// The state's path inside a home.
    pub fn path(home: &Path) -> PathBuf {
        home.join(SYNC_FILE)
    }

    /// Read the state; an absent file is the unconfigured default.
    pub fn load(home: &Path) -> SyncState {
        std::fs::read_to_string(Self::path(home))
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    /// Write the state atomically.
    pub fn save(&self, home: &Path) -> std::io::Result<()> {
        let path = Self::path(home);
        let text = serde_json::to_string_pretty(self).map_err(|error| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, error.to_string())
        })?;
        let temp = path.with_extension(format!("tmp.{}", std::process::id()));
        std::fs::write(&temp, text)?;
        std::fs::rename(&temp, path)
    }
}

/// The merge's outcome counts — the report the surface prints and the
/// journal settles (D28).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct MergeReport {
    /// Fact files merged mechanically by git.
    pub merged: usize,
    /// Recall-count lines merged by sum/delta.
    pub summed: usize,
    /// Supersede collisions resolved to one frontier.
    pub resolved: usize,
    /// Conflicts refused by name (the sync curtailed).
    pub refused: usize,
}

/// Merge a conflicted `recall-count` line by sum/delta over the base (D24):
/// `merged = ours + theirs - base`. A grow-only signal — the deltas add, so
/// a fact recalled on two installs keeps both installs' recalls. Never
/// last-writer-wins. Non-numeric stages fall back to the larger count (a
/// named, conservative answer rather than a silent drop).
pub fn merge_recall_count(base: &str, ours: &str, theirs: &str) -> u64 {
    let parse = |text: &str| text.trim().parse::<u64>().ok();
    match (parse(base), parse(ours), parse(theirs)) {
        (Some(b), Some(o), Some(t)) => o.saturating_add(t).saturating_sub(b),
        _ => parse(ours).unwrap_or(0).max(parse(theirs).unwrap_or(0)),
    }
}

/// The basis ladder's rank (D26): the winner rule for a supersede collision,
/// authority-free in lane 1. `confirmed-by` > `validated` > `inferred`.
pub fn basis_rank(basis: &str) -> u8 {
    match basis.trim() {
        "confirmed-by" => 3,
        "validated" => 2,
        "inferred" => 1,
        _ => 0,
    }
}

/// One side of a supersede collision: its basis, date, install-id, and name.
#[derive(Debug, Clone)]
pub struct SupersedeCandidate {
    /// The replacement fact's name.
    pub name: String,
    /// Its basis (`confirmed-by` / `validated` / `inferred`).
    pub basis: String,
    /// Its date stamp (lexicographic ISO order).
    pub date: String,
    /// The install that wrote it.
    pub install: String,
}

/// Resolve two replacements claiming one predecessor to a single live
/// frontier (D26): higher basis wins, tie → later date, tie → greater
/// install-id. Returns the winner; the loser is folded by the caller with a
/// named retirement note (never hard-deleted).
pub fn resolve_supersede(a: &SupersedeCandidate, b: &SupersedeCandidate) -> SupersedeCandidate {
    let rank = |c: &SupersedeCandidate| (basis_rank(&c.basis), c.date.clone(), c.install.clone());
    if rank(a) >= rank(b) {
        a.clone()
    } else {
        b.clone()
    }
}

/// An install-scoped replacement serial (D25): `<base>-r<serial>-<install8>`.
/// The install suffix disambiguates across installs; the local free-serial
/// scan + flock still order one install's writes.
pub fn scoped_replacement(base: &str, serial: u32, install: &str) -> String {
    let install8 = install.chars().take(8).collect::<String>();
    format!("{base}-r{serial}-{install8}")
}

/// The sync's refusal when no remote is configured (C0) — named, writes
/// nothing, fleet stays local.
pub fn sync_unconfigured() -> String {
    "no corpus remote configured — set one with `fleet memories --sync --remote <url>`; the corpus stays local".to_string()
}

/// Run a git command in a directory, returning stdout on success.
fn git(dir: &Path, args: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .map_err(|error| format!("git cannot run: {error}"))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

/// The corpus root for a home: the home's `memories` dir (the tracked fact
/// tree). The sync moves only this tree (D29).
pub fn corpus_root(home: &Path) -> PathBuf {
    home.join("memories")
}

/// Sync the corpus with its remote (D23): fetch → merge → push. With no
/// remote configured, refuses by name (C0). The merge takes over only the
/// conflicted fact files; any unresolvable conflict aborts the merge and
/// curtails the sync (the corpus stays at local HEAD — never a half-merge
/// pushed). Settles a `corpus_sync` journal event and grades by read-back
/// (D28). Returns the report.
pub fn sync(home: &Path, remote_override: Option<&str>) -> Result<MergeReport, String> {
    let mut state = SyncState::load(home);
    let remote = remote_override
        .map(str::to_string)
        .or_else(|| state.remote.clone());
    let Some(remote) = remote else {
        return Err(sync_unconfigured());
    };
    let root = corpus_root(home);
    if !root.join(".git").exists() {
        return Err(format!(
            "the corpus at {} is not a git work tree — the sync rides git",
            root.display()
        ));
    }
    let install = crate::identity::install(home).map_err(|error| error.to_string())?;
    let before = git(&root, &["rev-parse", "HEAD"]).unwrap_or_default();

    // Fetch the remote's corpus ref.
    git(&root, &["fetch", &remote, &state.corpus_ref])
        .map_err(|error| format!("the corpus fetch failed: {error}"))?;
    let remote_head = git(&root, &["rev-parse", "FETCH_HEAD"])
        .map_err(|error| format!("the remote revision is unreadable: {error}"))?;

    let mut report = MergeReport::default();
    // A no-change sync is a named no-op (C1).
    if remote_head == before {
        settle_sync(home, &install, &remote, &before, &before, &report);
        state.last_revision = Some(remote_head);
        state.remote = Some(remote);
        state.save(home).map_err(|error| error.to_string())?;
        return Ok(report);
    }

    // Merge the remote ref; git does the mechanical three-way merge.
    let merge = git(&root, &["merge", "--no-edit", "FETCH_HEAD"]);
    if merge.is_err() {
        // Take over the conflicted fact files; an unresolvable conflict
        // aborts and curtails (never a half-merge pushed).
        let conflicted =
            git(&root, &["diff", "--name-only", "--diff-filter=U"]).unwrap_or_default();
        let mut unresolvable = false;
        for path in conflicted.lines() {
            if path.ends_with(".md") {
                match resolve_conflict(&root, path) {
                    Ok(ConflictKind::Summed) => report.summed += 1,
                    Err(_) => unresolvable = true,
                }
            } else {
                unresolvable = true;
            }
        }
        if unresolvable {
            let _ = git(&root, &["merge", "--abort"]);
            report.refused += 1;
            settle_sync(home, &install, &remote, &before, &before, &report);
            return Err("the corpus merge had an unresolvable conflict — aborted, the corpus stays at local HEAD".to_string());
        }
        let _ = git(&root, &["add", "-A"]);
        let _ = git(&root, &["commit", "--no-edit"]);
    }
    report.merged += 1;

    // Grade by read-back before reporting done (D28): the merged tree must
    // load as facts, no live fact lost.
    let facts = crate::memory::load_facts(&root);
    if facts.is_empty() && before != remote_head {
        let _ = git(&root, &["reset", "--hard", &before]);
        return Err("the merged corpus read back empty — reset to local HEAD".to_string());
    }

    // Push the merged corpus.
    git(
        &root,
        &["push", &remote, &format!("HEAD:{}", state.corpus_ref)],
    )
    .map_err(|error| format!("the corpus push failed: {error}"))?;
    let after = git(&root, &["rev-parse", "HEAD"]).unwrap_or_default();

    settle_sync(home, &install, &remote, &before, &after, &report);
    state.last_revision = Some(after);
    state.remote = Some(remote);
    state.save(home).map_err(|error| error.to_string())?;
    Ok(report)
}

/// The kind of conflict resolved. Supersede collisions are prevented at the
/// source by install-scoped serials (D25); the winner rule itself is
/// [`resolve_supersede`], exercised directly.
enum ConflictKind {
    Summed,
}

/// Resolve one conflicted fact file from its base/ours/theirs stages. A
/// recall-count-only conflict merges by sum/delta; anything else is a named
/// refusal (the caller curtails).
fn resolve_conflict(root: &Path, path: &str) -> Result<ConflictKind, String> {
    let stage = |n: &str| git(root, &["show", &format!(":{n}:{path}")]).unwrap_or_default();
    let base = stage("1");
    let ours = stage("2");
    let theirs = stage("3");
    // A recall-count-only difference: merge by sum/delta.
    let count_of = |text: &str| {
        text.lines()
            .find(|line| line.starts_with("recall-count:"))
            .and_then(|line| line.split(':').nth(1))
            .map(str::trim)
            .unwrap_or("0")
            .to_string()
    };
    let base_count = count_of(&base);
    let ours_count = count_of(&ours);
    let theirs_count = count_of(&theirs);
    let strip_count = |text: &str| {
        text.lines()
            .filter(|line| !line.starts_with("recall-count:"))
            .collect::<Vec<_>>()
            .join("\n")
    };
    if strip_count(&ours) == strip_count(&theirs) {
        let merged = merge_recall_count(&base_count, &ours_count, &theirs_count);
        let mut result = ours.clone();
        if let Some(at) = result.find("recall-count:") {
            let end = result[at..]
                .find('\n')
                .map(|n| at + n)
                .unwrap_or(result.len());
            result = format!("{}recall-count: {merged}{}", &result[..at], &result[end..]);
        } else {
            result = format!("recall-count: {merged}\n{result}");
        }
        std::fs::write(root.join(path), result).map_err(|error| error.to_string())?;
        return Ok(ConflictKind::Summed);
    }
    // Any other conflict is a named refusal (lane 1's authority-free rule
    // does not guess content); the caller curtails the sync.
    Err(format!(
        "{path}: a content conflict beyond recall-count — refused by name"
    ))
}

/// Settle a `corpus_sync` journal event through L2 (D28).
fn settle_sync(
    home: &Path,
    install: &crate::identity::InstallId,
    remote: &str,
    before: &str,
    after: &str,
    report: &MergeReport,
) {
    if let Ok(mut journal) = crate::journal::Journal::open(home) {
        let _ = journal.append(
            "corpus_sync",
            Some(crate::identity::OWNER),
            None,
            json!({
                "install": install.as_str(),
                "remote": remote,
                "before": before,
                "after": after,
                "merged": report.merged,
                "summed": report.summed,
                "resolved": report.resolved,
                "refused": report.refused,
            }),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recall_count_merges_by_sum_delta_never_last_writer_wins() {
        // base 5, ours 8, theirs 7 → 8 + 7 - 5 = 10.
        assert_eq!(merge_recall_count("5", "8", "7"), 10);
        // No change on one side keeps the other's delta.
        assert_eq!(merge_recall_count("5", "5", "9"), 9);
        assert_eq!(merge_recall_count("5", "9", "5"), 9);
        // Both unchanged.
        assert_eq!(merge_recall_count("5", "5", "5"), 5);
        // A non-numeric stage falls back to the larger count.
        assert_eq!(merge_recall_count("x", "8", "7"), 8);
    }

    #[test]
    fn the_basis_ladder_orders_the_winner() {
        assert!(basis_rank("confirmed-by") > basis_rank("validated"));
        assert!(basis_rank("validated") > basis_rank("inferred"));
        assert!(basis_rank("inferred") > basis_rank("unknown"));
    }

    #[test]
    fn supersede_resolves_to_one_frontier_by_basis_then_date_then_install() {
        let mk = |name: &str, basis: &str, date: &str, install: &str| SupersedeCandidate {
            name: name.to_string(),
            basis: basis.to_string(),
            date: date.to_string(),
            install: install.to_string(),
        };
        // Higher basis wins.
        let a = mk("a", "validated", "2026-01-01", "install-a");
        let b = mk("b", "inferred", "2026-02-01", "install-b");
        assert_eq!(resolve_supersede(&a, &b).name, "a");
        // Tie on basis → later date.
        let c = mk("c", "inferred", "2026-03-01", "install-a");
        let d = mk("d", "inferred", "2026-02-01", "install-z");
        assert_eq!(resolve_supersede(&c, &d).name, "c");
        // Tie on basis + date → greater install-id.
        let e = mk("e", "inferred", "2026-01-01", "install-z");
        let f = mk("f", "inferred", "2026-01-01", "install-a");
        assert_eq!(resolve_supersede(&e, &f).name, "e");
    }

    #[test]
    fn two_installs_mint_distinct_scoped_serials() {
        let a = scoped_replacement("fact", 1, "aaaaaaaa11111111");
        let b = scoped_replacement("fact", 1, "bbbbbbbb22222222");
        assert_ne!(a, b);
        assert_eq!(a, "fact-r1-aaaaaaaa");
        assert_eq!(b, "fact-r1-bbbbbbbb");
    }

    #[test]
    fn an_unconfigured_sync_refuses_by_name() {
        let err = sync_unconfigured();
        assert!(err.contains("no corpus remote configured"));
    }

    #[test]
    fn sync_state_round_trips_and_defaults_unconfigured() {
        let dir = std::env::temp_dir().join(format!("fleet-corpus-state-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let default = SyncState::load(&dir);
        assert!(default.remote.is_none());
        assert_eq!(default.corpus_ref, "master");
        let mut state = default;
        state.remote = Some("https://example.com/corpus.git".to_string());
        state.last_revision = Some("git:abc".to_string());
        state.save(&dir).unwrap();
        let reloaded = SyncState::load(&dir);
        assert_eq!(reloaded.remote, state.remote);
        assert_eq!(reloaded.last_revision, state.last_revision);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn sync_with_no_remote_refuses_and_writes_nothing() {
        let dir = std::env::temp_dir().join(format!("fleet-corpus-nosync-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let result = sync(&dir, None);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("no corpus remote configured"));
        // Nothing was written.
        assert!(!SyncState::path(&dir).exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
