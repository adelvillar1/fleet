//! The dream pass's schedule seam (the memory-dreaming plan's C5): the
//! lane-owning validator and the task word, one module beside the pass
//! itself. [`crate::memory`] runs the pass lane-free — the judge is
//! injected — so the per-turn synchronous paths the beats' socket guards
//! scan never reach a lane ask; this module is reached only from the
//! scheduled thread, the CLI verb, and the face's runner, the seams that
//! already own scheduled lane work.

use crate::judge::{LaneSession, JUDGE_LANE, JUDGE_TOOL};
use crate::lanes::LaneSource;

/// The dream pass's validate judge (the memory-dreaming plan's C3):
/// one sys1 lane session over the facts the pass queues, the route
/// cache for repeat pairs, and the transcript read. The lane opens
/// lazily — a pass with nothing to judge spawns nothing — and a lane
/// that fails its open stays failed for the pass, its facts named
/// skips. Built by the CLI, the scheduled thread, and the face; the
/// pass itself stays lane-free so the fixture test can inject verdicts.
pub struct DreamValidator {
    home: std::path::PathBuf,
    session: Option<LaneSession>,
    cache: crate::route_cache::Cache,
    cache_key: String,
}

impl DreamValidator {
    /// Build the judge over `home`: the route cache loaded and keyed to
    /// the sys1 spec the same way the curate judgment keys its
    /// verdicts. A lane with no spec is not an error here — the first
    /// fact judges as a named skip.
    pub fn new(home: &std::path::Path) -> DreamValidator {
        let mut cache = crate::route_cache::Cache::load(home);
        let cache_key = crate::lanes::spec_for(JUDGE_LANE)
            .map(|spec| {
                let key = crate::route_cache::Cache::key_for(&spec);
                cache.rekey(&key);
                key
            })
            .unwrap_or_default();
        DreamValidator {
            home: home.to_path_buf(),
            session: None,
            cache,
            cache_key,
        }
    }

    /// Judge one fact: its `source` session read, the pair cached or
    /// asked, the verdict parsed. `None` is the named skip — no lane,
    /// no key, a dead lane, an unparseable reply — and the pass writes
    /// the row either way.
    pub async fn judge(
        &mut self,
        fact: &crate::memory::Fact,
    ) -> Option<crate::route_cache::ValidateVerdict> {
        // The evidence: the session the fact names, replayed and
        // bounded. A source that does not resolve is the honest
        // `unverifiable`, not a crash — the graded answer the pass
        // names.
        let records = crate::session::replay(&crate::session::session_path(
            &self.home,
            fact.source.trim(),
        ))
        .ok()?;
        let transcript = crate::session_recall::transcript_text(&records);
        let pair = format!(
            "fact: {}\ndescription: {}\n\nthe source session's transcript:\n{}",
            fact.name, fact.description, transcript
        );
        let hash = crate::calib::hash16(&pair);
        if let Some(verdict) = self.cache.get_validate(&hash) {
            return Some(verdict.clone());
        }
        // The lane opens lazily and stays failed for the pass.
        if self.session.is_none() {
            let source = LaneSource::Vendored;
            let opened = LaneSession::open(&source, JUDGE_LANE, JUDGE_TOOL, VALIDATE_TASK).await;
            match opened {
                Ok(session) => self.session = Some(session),
                Err(reason) => {
                    eprintln!("fleet: the validate lane could not start: {reason}");
                    return None;
                }
            }
        }
        let session = self.session.as_mut().expect("the lane opened above");
        let answer = session.ask(&pair).await.map_err(|error| {
            eprintln!("fleet: the validate ask did not cross: {error}");
        });
        let answer = answer.ok()?;
        let parsed = crate::memory::parse_validate(&answer.to_string());
        if parsed.is_none() {
            eprintln!(
                "fleet: the validate answer did not parse as a verdict: {}",
                answer
            );
        }
        let verdict = parsed?;
        self.cache.put_validate(hash, verdict.clone());
        self.cache.flush(&self.cache_key);
        Some(verdict)
    }

    /// Close the lane on the pass's one exit path.
    pub async fn close(self) {
        if let Some(session) = self.session {
            session.close().await;
        }
    }
}

/// The registered sys1 task the validate judgment asks with: the
/// memory-dreaming plan's task_spec, `validity` head naming
/// `supported` / `contradicted` / `unverifiable` with a confidence.
pub const VALIDATE_TASK: &str = "validate";
