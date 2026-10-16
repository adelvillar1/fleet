//! The route-decision cache: one recorded judgment per task, keyed to
//! the judging lane that produced it.
//!
//! `select` judges every route from scratch — it spawns the sys1 MCP
//! server, initializes it, calls `sys1_classify`, and closes it. The
//! zcode-router-kit caches exactly this decision and its board reports
//! a 27:1 cached-to-fresh ratio, so the same judgment repeats
//! constantly in practice. This is fleet's own cache for it, local
//! state beside the roster it describes, on the recall-index's
//! discipline.
//!
//! Two rules carry over from that cache and one is new:
//!
//! - **Keyed to the producer, invalidated whole.** The key is the
//!   judging lane's launch identity — its program, arguments, and
//!   working directory. A cache written by another lane, or by the
//!   same lane before its service moved, is not this lane's cache and
//!   is discarded rather than served. The named limit: a lane whose
//!   model changes under an unchanged launch spec is the one case the
//!   key cannot see.
//! - **Never errors.** An absent or malformed file reads as empty and
//!   costs a recompute. The write is a unique temp file plus a rename,
//!   so a reader never sees a half-written cache and two processes
//!   cannot collide on one temp.
//! - **A hit is not a skipped event.** The caller still writes its
//!   decision row, marked `cached` — the log comes first, and a cache
//!   that suppressed its own evidence would break the calibration
//!   stream's completeness.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::mcp::ServerSpec;

/// `<FLEET_HOME>/route-cache.json`.
const CACHE_FILE: &str = "route-cache.json";

/// One recorded judgment: what the lane said about one task. The task
/// itself is never stored — its hash is the key, the same
/// never-the-text rule the decision row's `input_hash` runs on.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Verdict {
    /// The judged workload label.
    pub workload: String,
    /// The lane's own confidence in that label.
    pub confidence: f64,
    /// The lane's verdict (`pass`/`escalate`/`block`/`error`).
    pub verdict: String,
}

/// The loaded cache: the key it was written under, one entry per task
/// hash, and the file it reads and writes.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Cache {
    /// The file this cache reads and writes. Not serialized: the file
    /// is where the cache lives, not a fact about it.
    #[serde(skip)]
    path: PathBuf,
    /// The judging lane's launch identity the entries were computed
    /// under. Empty means the pre-key format, which no key matches.
    #[serde(default)]
    key: String,
    /// One recorded verdict per task hash.
    #[serde(default)]
    entries: HashMap<String, Verdict>,
}

impl Cache {
    /// Read `<home>/route-cache.json`. An absent or malformed file is
    /// an empty cache and costs a recompute — the same tolerance the
    /// recall-index runs on.
    pub fn load(home: &Path) -> Cache {
        let path = home.join(CACHE_FILE);
        let mut cache: Cache = fs::read_to_string(&path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default();
        cache.path = path;
        cache
    }

    /// The launch identity of the lane that would judge now: its
    /// program, its arguments, and the working directory its service
    /// resolves to. A lane that moved — a re-vendored service, a
    /// different snapshot — is a different producer and must not
    /// inherit the old lane's verdicts.
    pub fn key_for(spec: &ServerSpec) -> String {
        format!(
            "{} {} [{}]",
            spec.program,
            spec.args.join(" "),
            spec.cwd
                .as_ref()
                .map(|dir| dir.display().to_string())
                .unwrap_or_default()
        )
    }

    /// Whether the entries were computed by the lane loading now. A
    /// different key — or the pre-key format's empty key — says no,
    /// and the caller discards the whole cache rather than serving
    /// verdicts this lane never produced.
    pub fn keyed_for(&self, key: &str) -> bool {
        !key.is_empty() && self.key == key
    }

    /// The recorded verdict for `task_hash`, when this cache is the
    /// loading lane's.
    pub fn get(&self, task_hash: &str) -> Option<&Verdict> {
        self.entries.get(task_hash)
    }

    /// Record one verdict under `task_hash`. The write is not
    /// immediate: `flush` persists, so a run that judges once and
    /// writes the file once does not rewrite it per call.
    pub fn put(&mut self, task_hash: String, verdict: Verdict) {
        self.entries.insert(task_hash, verdict);
    }

    /// Persist under `key`, through a unique temp file and a rename.
    /// Best-effort and silent by design: a failed write costs the next
    /// run a recompute and nothing else — the cache is
    /// `__pycache__`, and `__pycache__` never complains.
    pub fn flush(&mut self, key: &str) {
        self.key = key.to_string();
        let text = match serde_json::to_string_pretty(self) {
            Ok(text) => text,
            Err(_) => return,
        };
        // The temp name carries this process's id, so two fleet
        // processes writing the same cache cannot collide on one
        // temp file.
        let temp = self
            .path
            .with_extension(format!("{}.tmp", std::process::id()));
        if fs::write(&temp, text).is_err() {
            return;
        }
        let _ = fs::rename(&temp, &self.path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp::ServerSpec;

    fn spec(program: &str, cwd: &str) -> ServerSpec {
        ServerSpec {
            program: program.to_string(),
            args: vec!["-u".to_string(), "service/mcp_server.py".to_string()],
            env: Vec::new(),
            passthrough: Vec::new(),
            cwd: Some(PathBuf::from(cwd)),
        }
    }

    fn verdict() -> Verdict {
        Verdict {
            workload: "hard".to_string(),
            confidence: 0.91,
            verdict: "pass".to_string(),
        }
    }

    #[test]
    fn a_cache_round_trips_through_its_file() {
        let home = scratch("route-cache-round-trip");
        let mut cache = Cache::load(&home);
        assert!(cache.get("abc123").is_none(), "a fresh cache is empty");
        cache.put("abc123".to_string(), verdict());
        cache.flush(&Cache::key_for(&spec("python3", "/lane")));
        // A second load reads what the first wrote.
        let reloaded = Cache::load(&home);
        assert!(reloaded.keyed_for(&Cache::key_for(&spec("python3", "/lane"))));
        let recorded = reloaded.get("abc123").expect("the recorded verdict");
        assert_eq!(recorded.workload, "hard");
        assert_eq!(recorded.confidence, 0.91);
        assert_eq!(recorded.verdict, "pass");
        let _ = std::fs::remove_dir_all(&home);
    }

    /// The key is the invalidation discipline: a cache written by
    /// another lane — or by this lane before its service moved — is
    /// not this lane's cache, and the caller discards it whole rather
    /// than serving verdicts this lane never produced.
    #[test]
    fn a_cache_keyed_to_another_lane_is_discarded_whole() {
        let home = scratch("route-cache-key");
        let mut cache = Cache::load(&home);
        cache.put("abc123".to_string(), verdict());
        cache.flush(&Cache::key_for(&spec("python3", "/lane-a")));
        let reloaded = Cache::load(&home);
        assert!(
            !reloaded.keyed_for(&Cache::key_for(&spec("python3", "/lane-b"))),
            "a different lane's key does not match"
        );
        assert!(
            !reloaded.keyed_for(&Cache::key_for(&spec("python3", "/lane-a/moved"))),
            "a moved service is a different producer"
        );
        assert!(
            reloaded.keyed_for(&Cache::key_for(&spec("python3", "/lane-a"))),
            "the same launch identity still matches"
        );
        let _ = std::fs::remove_dir_all(&home);
    }

    /// A cache never errors: absent or malformed reads as empty and
    /// costs a recompute — the recall-index's tolerance.
    #[test]
    fn an_absent_or_malformed_file_reads_as_empty() {
        let home = scratch("route-cache-absent");
        assert!(Cache::load(&home).get("anything").is_none());
        std::fs::write(home.join(CACHE_FILE), "{ not json at all").unwrap();
        let cache = Cache::load(&home);
        assert!(cache.get("anything").is_none(), "malformed reads as empty");
        assert!(!cache.keyed_for("python3 /lane"), "and carries no key");
        let _ = std::fs::remove_dir_all(&home);
    }

    /// A scratch directory under the session's TMPDIR, named for the
    /// test so parallel runs do not collide.
    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("fleet-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }
}
