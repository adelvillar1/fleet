//! Session recall: the session archive becomes rankable. One synthetic
//! fact per session log, ranked through the facts corpus's own
//! six-channel ranker, with a second embedder-keyed index under the
//! sessions directory.
//!
//! **Why a session is a better fit for the ranker than a fact is.**
//! The ranker's largest channel is the trajectory — the phrases the
//! resolution drove with — and on the facts corpus those phrases are
//! composed by hand at write time. A session log IS a trajectory: the
//! task, the calls that succeeded, the ones that failed and were
//! corrected, in order. So the derivation here is not a composition
//! but a reading: pair each tool call with its result, keep the ones
//! that succeeded, and the trajectory channel gets the record itself.
//! That is the plan's C2 — the phrase-writing piece's derivation,
//! applied to a finished session instead of a live one.
//!
//! **The ranker is reused, unchanged.** [`memory::Fact`] is the seam:
//! a session becomes one fact whose `phrases` are its successful
//! calls, whose `shape` is its outcome word, and whose `basis` is
//! `inferred` — a session is evidence this run produced, never
//! something the owner stated. The facts corpus reads provenance from
//! `basis`; the session archive reads it from the outcome grade
//! instead (the plan's C8), so every session rides the same
//! half-strength provenance and the grade rides the shape where the
//! surface can see it. [`memory::Memory::recall`] then ranks the
//! archive exactly as it ranks the corpus: same weights, same
//! channels, same floor.
//!
//! **The second index.** The cached vectors live at
//! `<sessions_dir>/.recall-index.json` — a second file on the recall
//! index's exact discipline, deliberately NOT shared with
//! `memories/.recall-index.json`: the two caches serve different
//! corpora, so a model swap invalidates one rather than both at once,
//! and neither pollutes the other. The index is `__pycache__`: it
//! never errors, an absent or malformed read costs a recompute, and a
//! cache keyed to another embedder is discarded whole rather than
//! serving vectors this embedder did not compute.
//!
//! **The bounds are named.** The task is bounded to the writer's own
//! description bound, the digest to [`DIGEST_CAP`], the phrases to the
//! writer's own caps. The vector channel reads a sample, never a
//! transcript — the channel degrades by design rather than flooding
//! the index with a long session's whole log.
//!
//! **What this module is not.** It ranks, it caches, and it closes its
//! own feedback loop; the surface that renders [`ScoredHit`] as cards
//! is the parent's. The loop is the archive's own because it cannot
//! be the corpus's: a synthetic fact's `path` is the session log, so
//! the corpus's own bump must never be pointed at it. Instead a
//! recalled session bumps [`RecallCounts`] —
//! `<sessions_dir>/.recall-counts.json`, a store on the recall
//! index's exact discipline — which seeds the synthetic fact's
//! `recall_count` and so feeds the history channel exactly as the
//! corpus's own counts feed its, and lands one `recall_hit` row
//! naming the session id through the stream's own path. The log stays
//! append-only and the synthetic fact stays read-only by
//! construction: the store is the only writer of a session's count,
//! and [`memory::Memory::recall_hits`] is still never routed a
//! synthetic fact.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use serde::{Deserialize, Serialize};

use crate::api::FunctionCall;
use crate::calib;
use crate::memory;
use crate::session::{self, Record};

// --- the synthetic facts -------------------------------------------------

/// The digest's bound: the task, then every tool call's name and its
/// compacted arguments, capped here. The vector channel reads a
/// sample, never a transcript — a long session's log is large, and an
/// unbounded digest would flood the index with text no channel reads
/// whole and no card can render. The cap is a named constant with the
/// same honesty as every other bound in the crate: the channel
/// degrades by design rather than by surprise.
pub const DIGEST_CAP: usize = 4000;

/// The fact type every synthetic session carries. A session is its own
/// kind in the corpus's taxonomy — not a `project`, and deliberately
/// not one of the protected types: the protected tier is the owner's
/// standing preferences, and a session is this run's evidence.
pub const SESSION_TYPE: &str = "session";

/// One synthetic [`memory::Fact`] per session log under
/// `sessions_dir(home)`, sorted by session id — the loader's own
/// order, so the archive ranks stably across loads.
///
/// A log that does not replay is skipped by id with one logged line: a
/// broken session is never silent and never fatal, the facts loader's
/// own rule. A missing directory is no sessions, not an error.
pub fn facts_from_sessions(home: &Path) -> Vec<memory::Fact> {
    facts_from_dir(&crate::home::sessions_dir(home))
}

/// The derivation over a sessions directory itself — the seam
/// [`SessionIndex::recall`] uses, because the index is keyed to the
/// directory it sits beside rather than to the home above it.
fn facts_from_dir(sessions_dir: &Path) -> Vec<memory::Fact> {
    let Ok(entries) = fs::read_dir(sessions_dir) else {
        return Vec::new();
    };
    let mut paths: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_file() && path.extension().and_then(|ext| ext.to_str()) == Some("jsonl")
        })
        .collect();
    // The loader's own order: by path, which for one directory and one
    // extension is by session id.
    paths.sort();
    // The archive's own feedback state, read once per load: a
    // session's earned recall count, which seeds the synthetic fact's
    // history channel. The store is `__pycache__` — an absent or
    // malformed file reads as empty and costs a recompute of nothing
    // at all, because the derivation is already in memory.
    let counts = RecallCounts::load(sessions_dir);
    let mut facts: Vec<memory::Fact> = Vec::new();
    for path in paths {
        let Some(id) = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .map(str::to_string)
        else {
            continue;
        };
        // `replay` fails loud by design — reading an unreadable format
        // silently would corrupt the derivation — so an unreadable log
        // is one logged line and no fact, never a half-read session.
        let records = match session::replay(&path) {
            Ok(records) if !records.is_empty() => records,
            Ok(_) => continue,
            Err(_) => {
                skip_line(&id, "unreadable log");
                continue;
            }
        };
        facts.push(fact_for(&id, &path, &records, counts.count_of(&id)));
    }
    facts
}

/// One session log as one fact. The field choices are the whole
/// design, so each carries its reason:
///
/// - `description` is the task and `body` the bounded digest — what
///   the session was and what it did;
/// - `phrases` are the successful calls — where it drove;
/// - `shape` is the outcome word — what became of the trajectory;
/// - `basis` is `Inferred` for every session, because a session is
///   evidence this run produced and never something the owner stated.
///   The facts corpus reads provenance from `basis`; the session
///   archive reads it from the outcome grade instead (the plan's C8),
///   so every session rides the same half-strength provenance and the
///   grade rides the shape where the surface can see it;
/// - `tier` is `Durable`: a log is append-only and never expires;
/// - `stamp` is the log's own `secs:nanos:len`, so the index can tell
///   a changed session from an untouched one;
/// - `date` is the `start` record's day, which is what the freshness
///   channel decays from — a session's age is its own;
/// - `recall_count` is seeded from the archive's own count store
///   ([`RecallCounts`]), so the history channel reads a recalled
///   session's earned count exactly as the facts corpus reads its own
///   — the same feedback loop, closed on state that is not the log.
fn fact_for(id: &str, path: &Path, records: &[Record], count: u64) -> memory::Fact {
    let task = task_text(records);
    memory::Fact {
        name: id.to_string(),
        description: task.clone(),
        fact_type: SESSION_TYPE.to_string(),
        date: session_date(records),
        basis: memory::Basis::Inferred,
        tier: memory::Tier::Durable,
        shape: vec![outcome_word(records).to_string()],
        phrases: derived_phrases(records),
        missing: Vec::new(),
        recall_count: count,
        superseded_by: String::new(),
        valid_until: String::new(),
        body: digest(records, &task),
        path: path.to_path_buf(),
        stamp: memory::file_stamp(path),
    }
}

/// The session's task: the first `user` record's content, bounded to
/// the description bound the facts writer enforces. A session with no
/// user message — an unattended job whose task rode a frame — has no
/// task text, and its description is empty rather than invented.
fn task_text(records: &[Record]) -> String {
    records
        .iter()
        .find_map(|record| match record {
            Record::User { content } => {
                Some(cut(&sanitized(content), memory::MAX_DESCRIPTION_CHARS))
            }
            _ => None,
        })
        .unwrap_or_default()
}

/// The session's day: the `start` record's `created` field, read at
/// its own ISO prefix. The freshness channel decays from the fact's
/// date, and a session's age is the day it ran — not the day its
/// index entry was written.
fn session_date(records: &[Record]) -> String {
    records
        .iter()
        .find_map(|record| match record {
            Record::Start { created, .. } => created.get(..10).map(str::to_string),
            _ => None,
        })
        .unwrap_or_default()
}

/// The session's outcome, mapped onto the sign vocabulary the ranker
/// already carries — [`memory::Sign`]'s own three words, so no new
/// vocabulary is invented: a session that stopped `completed` read
/// `resolved`, `max-turns` drove the right direction and stalled so it
/// reads `incomplete`, and everything else — `failed`, an owner's
/// stop, a log with no `end` at all, which is the interrupted shape —
/// reads `diverged`.
///
/// The word rides the fact's `shape` rather than a field of its own
/// because the shape is where the ranker already keeps a vector's
/// kind, and the surface reads it back from there. Note what this is
/// NOT: `Fact::sign` derives its marker from the failure axis and the
/// two outcome tags, and `diverged` is neither — so the session
/// surface reads the outcome word off the shape (through
/// [`ScoredHit::outcome`]) rather than asking `Sign::of`, which would
/// call a diverged session a resolution. The archive's marker is its
/// own; the ranker's is the corpus's.
fn outcome_word(records: &[Record]) -> &'static str {
    match records.last() {
        Some(Record::End { stop_reason, .. }) => match stop_reason.as_str() {
            "completed" => memory::Sign::Resolution.word(),
            "max-turns" => memory::Sign::Incomplete.word(),
            // `failed`, `stopped by the owner`, and any word a later
            // format adds: the conservative answer, because a
            // trajectory that did not land drove somewhere else.
            _ => memory::Sign::Divergence.word(),
        },
        // No `end` record: interrupted, the documented shape. Any
        // other last record is a log still in flight or cut short —
        // the same answer.
        _ => memory::Sign::Divergence.word(),
    }
}

/// The session's phrases: the trajectory's own vocabulary, read off
/// the record. This is the derivation the `remember` tool's
/// phrase-filler runs over a live conversation, applied to a finished
/// session instead — the plan's C2, the phrase-writing piece's rules
/// matched exactly:
///
/// - pair each tool call with its result by call id, and keep only the
///   calls that SUCCEEDED — a result whose text starts with `error:`
///   or `denied` drove somewhere else, so it contributes nothing;
/// - a `bash` call contributes its `command`, a file call its `path`,
///   and anything else contributes its own name, because the
///   mechanism is the vocabulary for a lane call or a delegation;
/// - most recent first, deduplicated, bounded by the writer's own
///   caps — a direction sample, never a transcript.
///
/// The one difference from the live derivation is the window: a
/// session has none. The agent's derivation windows to the calls after
/// the last user message, because one conversation can hold several
/// tasks and a fact about one must not carry another's phrases. A
/// session log is one trajectory from its task to its outcome, so the
/// whole log is the window.
fn derived_phrases(records: &[Record]) -> Vec<String> {
    // The results, by call id: the join that makes "succeeded" a fact
    // rather than a guess. A call with no result yet — the turn still
    // in flight when the log ends — has not succeeded.
    let mut results: HashMap<&str, &str> = HashMap::new();
    for record in records {
        if let Record::ToolResult {
            call_id, output, ..
        } = record
        {
            results.insert(call_id.as_str(), output.as_str());
        }
    }
    let mut phrases: Vec<String> = Vec::new();
    for record in records.iter().rev() {
        let Record::Assistant { tool_calls, .. } = record else {
            continue;
        };
        for call in tool_calls.iter().rev() {
            let Some(output) = results.get(call.id.as_str()).copied() else {
                continue;
            };
            if output.starts_with("error:") || output.starts_with("denied") {
                continue;
            }
            if let Some(phrase) = call_phrase(&call.function) {
                if !phrases.contains(&phrase) {
                    phrases.push(phrase);
                }
            }
        }
    }
    phrases.truncate(memory::MAX_PHRASES);
    phrases
}

/// One successful call's phrase: the concrete vocabulary a later
/// session would actually search with. `None` for a call whose
/// arguments name nothing, because a phrase that says only the tool's
/// name twice is noise. The agent's own `call_phrase`, matched rule
/// for rule so the two derivations cannot drift.
fn call_phrase(function: &FunctionCall) -> Option<String> {
    let key = match function.name.as_str() {
        "bash" => "command",
        "write_file" | "read_file" | "list_dir" => "path",
        _ => return Some(bounded_phrase(&function.name)),
    };
    let parsed: serde_json::Value = serde_json::from_str(&function.arguments).ok()?;
    let value = parsed.get(key).and_then(|value| value.as_str())?;
    Some(bounded_phrase(value))
}

/// One derived phrase, bounded to the writer's own per-item bound so
/// a long command cannot flood the fact's phrases — plus the
/// control-character strip every other writer in the crate applies
/// (KI-060), because the phrase rides a card and a card is a
/// fixed-width surface.
fn bounded_phrase(text: &str) -> String {
    cut(&sanitized(text), memory::MAX_PHRASE_CHARS)
}

/// The session's body: a bounded digest — the task, then each tool
/// call's name and its compacted arguments, capped at [`DIGEST_CAP`].
///
/// The digest is what the session DID, which is not the same as what
/// the phrases record: every call rides it, the failed ones included,
/// because the record of an attempt is evidence even when the attempt
/// drove nowhere. What it is bounded for is the surface: a card
/// renders a body, a card is a fixed-width surface, and a long
/// session's log is large — so the digest is a sample with a marked
/// cut, never a transcript.
fn digest(records: &[Record], task: &str) -> String {
    let mut lines: Vec<String> = Vec::new();
    if !task.is_empty() {
        lines.push(format!("task: {task}"));
    }
    for record in records {
        if let Record::Assistant { tool_calls, .. } = record {
            for call in tool_calls {
                lines.push(format!(
                    "{} {}",
                    sanitized(&call.function.name),
                    compact_arguments(&call.function.arguments)
                ));
            }
        }
    }
    cut(&lines.join("\n"), DIGEST_CAP)
}

/// The call's arguments as one compact line: re-serialized through
/// JSON so the digest carries one line per call, sanitized like every
/// other line. Arguments that do not parse ride verbatim — a
/// malformed call is still evidence of what the model tried.
fn compact_arguments(arguments: &str) -> String {
    let compact = match serde_json::from_str::<serde_json::Value>(arguments) {
        Ok(value) => serde_json::to_string(&value).unwrap_or_else(|_| arguments.to_string()),
        Err(_) => arguments.to_string(),
    };
    sanitized(&compact)
}

/// One line of derived text, sanitized the way the writer sanitizes:
/// control characters removed — not replaced, so a stripped escape
/// sequence leaves its payload visible and inert — then trimmed.
/// KI-060's mechanism, mirrored because this module writes no files
/// and still owes the same bound: the phrases and the digest ride a
/// card, and a card is a fixed-width surface.
fn sanitized(text: &str) -> String {
    text.chars()
        .filter(|character| !character.is_control())
        .collect::<String>()
        .trim()
        .to_string()
}

/// One bounded text: cut at `bound` with a marked ellipsis, the
/// writer's own bound style. A bounded sample says it was cut; a
/// silently truncated one reads as whole.
fn cut(text: &str, bound: usize) -> String {
    if text.chars().count() > bound {
        format!("{}\u{2026}", text.chars().take(bound).collect::<String>())
    } else {
        text.to_string()
    }
}

/// One log line naming the session that was skipped and why. A broken
/// session is never silent and never fatal — the facts loader's rule.
fn skip_line(id: &str, reason: &str) {
    eprintln!("fleet: sessions: skipping `{id}` \u{2014} {reason}");
}

// --- the index ------------------------------------------------------------

/// The generated cache under the sessions directory: one entry per
/// session, keyed by id, carrying the stamp the vector was computed
/// at — plus the embedder identity and dimensionality the vectors
/// were computed **at**.
///
/// A SECOND index, deliberately not shared with
/// `memories/.recall-index.json`: the two caches serve different
/// corpora, so a model swap invalidates one of them rather than both
/// at once, and neither pollutes the other. The recall index's exact
/// discipline, mirrored rather than reused, because the two files
/// answer to different roots and a shared reader would couple them: a
/// cache never errors — an absent or malformed index reads as empty
/// and costs a recompute — and an index keyed to another embedder is
/// not this embedder's cache, so it is discarded whole rather than
/// serving vectors this embedder did not compute.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct SessionIndex {
    /// The embedder that computed the cached vectors — its endpoint or
    /// model name, from `Embedder::vector_info`.
    #[serde(default)]
    embedder: String,
    /// The vector length that embedder served.
    #[serde(default)]
    dims: usize,
    /// One entry per session id.
    #[serde(default)]
    sessions: HashMap<String, SessionEntry>,
}

/// One cached vector with the file stamp it was computed at.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SessionEntry {
    stamp: String,
    vector: Vec<f32>,
}

/// `<sessions_dir>/.recall-index.json` — the recall index's own file
/// name under the sessions directory, which is the whole point: the
/// same name in a different root is a second cache, not a shared one.
fn index_path(sessions_dir: &Path) -> PathBuf {
    sessions_dir.join(memory::INDEX_FILE)
}

impl SessionIndex {
    /// Read the cache. Never errors: an absent or malformed file reads
    /// as empty and costs a recompute — the index is `__pycache__`,
    /// and `__pycache__` never complains.
    pub fn load(sessions_dir: &Path) -> SessionIndex {
        fs::read_to_string(index_path(sessions_dir))
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    /// Whether the cached vectors came from the embedder loading now.
    /// A different endpoint, a different dimensionality, or the pre-key
    /// format (which reads as an empty key) all say no — and a no
    /// invalidates the whole cache rather than serving vectors this
    /// embedder did not compute.
    fn keyed_for(&self, key: &(String, usize)) -> bool {
        self.embedder == key.0 && self.dims == key.1
    }

    /// Write the cache through a unique temp file and a rename, so a
    /// reader never sees a half-written index and two fleet processes
    /// cannot collide on one temp. Best-effort and silent by design: a
    /// failed write costs the next load a recompute and nothing else —
    /// the index is `__pycache__`, and `__pycache__` never complains.
    fn write(
        sessions_dir: &Path,
        key: Option<&(String, usize)>,
        entries: &HashMap<String, SessionEntry>,
    ) {
        let index = SessionIndex {
            embedder: key.map(|(name, _)| name.clone()).unwrap_or_default(),
            dims: key.map(|(_, dims)| *dims).unwrap_or_default(),
            sessions: entries.clone(),
        };
        let Ok(text) = serde_json::to_string(&index) else {
            return;
        };
        let path = index_path(sessions_dir);
        let serial = SESSION_INDEX_WRITE_SERIAL.fetch_add(1, Ordering::Relaxed);
        let mut temp = path.clone().into_os_string();
        temp.push(format!(".{}.{}.tmp", std::process::id(), serial));
        let temp = PathBuf::from(temp);
        if fs::write(&temp, text).is_ok() {
            let _ = fs::rename(&temp, &path);
        }
    }

    /// Rank the archive for one instruction: derive the facts, reuse
    /// the cached vectors whose stamps still match, embed only what
    /// changed, flush the index only when something changed, and run
    /// the corpus's own ranker over the result — the recall index's
    /// load-time build, applied to the session archive.
    ///
    /// The embedder rides in by value, the way [`memory::Memory::load_with`]
    /// takes it: `Memory` owns its backend (`Box<dyn Embedder>`), so a
    /// `Memory` built over facts that did not come from a corpus root
    /// needs an owned one, and a borrow per call cannot feed it
    /// without either a second boxing method on the `Embedder` trait
    /// or a proxy the crate has no way to build. A surface reads the
    /// backend's label off `vector_info` before handing the box over,
    /// and a keyword-only backend costs no index write at all — the
    /// cache is left exactly as it was, the same rule the recall
    /// index runs on.
    pub fn recall(
        &mut self,
        sessions_dir: &Path,
        embedder: Box<dyn memory::Embedder>,
        instruction: &str,
    ) -> Vec<ScoredHit> {
        let facts = facts_from_dir(sessions_dir);
        let vectors = self.vectors_for(sessions_dir, embedder.as_ref(), &facts);
        // The ranker itself, unchanged: the corpus's own recall over
        // the synthetic facts and their vectors.
        memory::Memory::from_facts(facts, vectors, embedder)
            .recall(instruction)
            .iter()
            .map(hit_of)
            .collect()
    }

    /// The vectors the cache already holds for `facts`, keyed by id —
    /// no embed, no write. The related index's read: a request must not
    /// pay the archive's cold embed (one call per session, which on a
    /// model swap is a minute), so it ranks the sessions the cache can
    /// answer and leaves the rest to the background warm.
    pub fn cached_vectors(&self, facts: &[memory::Fact]) -> HashMap<String, Vec<f32>> {
        let mut vectors = HashMap::new();
        for fact in facts {
            let Some(stamp) = &fact.stamp else {
                continue;
            };
            if let Some(entry) = self
                .sessions
                .get(&fact.name)
                .filter(|entry| &entry.stamp == stamp)
            {
                vectors.insert(fact.name.clone(), entry.vector.clone());
            }
        }
        vectors
    }

    /// The vectors this cache holds for `facts`, embedding only what
    /// changed and flushing the index only when something did — the
    /// body [`SessionIndex::recall`] has always run, split out so the
    /// related index can read the archive's vectors without running a
    /// recall it does not want. A keyword-only embedder answers with
    /// an empty map and writes nothing, exactly as the recall does.
    pub fn vectors_for(
        &mut self,
        sessions_dir: &Path,
        embedder: &dyn memory::Embedder,
        facts: &[memory::Fact],
    ) -> HashMap<String, Vec<f32>> {
        let mut vectors: HashMap<String, Vec<f32>> = HashMap::new();
        if !embedder.serves_vectors() {
            return vectors;
        }
        let key = embedder.vector_info();
        // A cache keyed to another embedder is not this embedder's
        // cache: discard it whole, the way the recall index does.
        if !matches!(&key, Some(key) if self.keyed_for(key)) {
            self.embedder.clear();
            self.dims = 0;
            self.sessions.clear();
        }
        let mut fresh: HashMap<String, SessionEntry> = HashMap::new();
        let mut changed = false;
        for fact in facts {
            let cached = fact.stamp.as_ref().and_then(|stamp| {
                self.sessions
                    .get(&fact.name)
                    .filter(|entry| &entry.stamp == stamp)
                    .map(|entry| entry.vector.clone())
            });
            let vector = match cached {
                Some(vector) => vector,
                None => match embedder.embed(&fact.indexed_text()) {
                    Some(vector) => {
                        changed = true;
                        vector
                    }
                    None => {
                        // Never silent: the session keeps no vector
                        // this load and one line says so, by id —
                        // the keyword channels still rank it.
                        eprintln!(
                            "fleet: sessions: `{}` \u{2014} the embed failed, so it carries no vector this load",
                            fact.name
                        );
                        continue;
                    }
                },
            };
            if let Some(stamp) = &fact.stamp {
                fresh.insert(
                    fact.name.clone(),
                    SessionEntry {
                        stamp: stamp.clone(),
                        vector: vector.clone(),
                    },
                );
            }
            vectors.insert(fact.name.clone(), vector);
        }
        // Flush only when something changed — a new embed, a dropped
        // session, or a discarded cache — so an unchanged archive costs
        // no write.
        if changed || fresh.len() != self.sessions.len() {
            SessionIndex::write(sessions_dir, key.as_ref(), &fresh);
        }
        self.sessions = fresh;
        if let Some((name, dims)) = &key {
            self.embedder = name.clone();
            self.dims = *dims;
        }
        vectors
    }
}

/// A per-write serial for the index temp name, so two fleet processes
/// — or two writes in one process — never share a temp file.
static SESSION_INDEX_WRITE_SERIAL: AtomicUsize = AtomicUsize::new(0);

// --- the count store -------------------------------------------------------

/// The count store's file name: `<sessions_dir>/.recall-counts.json`,
/// beside the logs it counts and beside the vector index — its own
/// name, so it shares a line with neither.
const COUNTS_FILE: &str = ".recall-counts.json";

/// How many recalls each session has earned, keyed by session id —
/// the archive's own feedback state, on the recall index's exact
/// discipline.
///
/// Why the count cannot live in the log: the log is append-only, and
/// a bounded frontmatter-preserving rewrite is exactly what the
/// corpus's own bump performs on a fact's file. A synthetic fact's
/// `path` IS the session log, so routing a session through the
/// corpus's hit-recording path would rewrite an append-only log —
/// which is why the archive's loop was deferred in the first place.
/// The store is the resolution: the count lives in its own file, the
/// log is never opened for writing, and the synthetic fact stays
/// read-only by construction.
///
/// The discipline mirrors [`SessionIndex`] rather than reusing it:
/// a cache never errors — an absent or malformed store reads as empty
/// and costs a session its earned history until the next recall,
/// which is the honest failure for state that only re-ranks — and a
/// failed write is one logged line, never an error the surface has
/// to answer for. The recall itself is never blocked, delayed, or
/// failed by its own bookkeeping, the same contract the calibration
/// stream runs on.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct RecallCounts {
    /// One entry per session id: the recalls it has earned and the
    /// day of the last one.
    #[serde(default)]
    sessions: HashMap<String, RecallCount>,
}

/// One session's earned recalls and the day of the last.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct RecallCount {
    /// The recalls this session has earned.
    count: u64,
    /// The day of the last one, `YYYY-MM-DD`. The history channel's
    /// own arithmetic reads the count; the day rides along so the
    /// store says when the archive last reached for this session.
    #[serde(default)]
    last: String,
}

/// `<sessions_dir>/.recall-counts.json` — the store's own file, under
/// the sessions directory beside the logs it counts.
fn counts_path(sessions_dir: &Path) -> PathBuf {
    sessions_dir.join(COUNTS_FILE)
}

impl RecallCounts {
    /// Read the store. Never errors: an absent or malformed file reads
    /// as empty, the recall index's own rule — the count is state that
    /// only re-ranks, so a lost store costs a session its history
    /// until the next recall and nothing else.
    pub fn load(sessions_dir: &Path) -> RecallCounts {
        fs::read_to_string(counts_path(sessions_dir))
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    /// The count one session has earned; zero for a session the store
    /// has never seen.
    pub fn count_of(&self, session_id: &str) -> u64 {
        self.sessions
            .get(session_id)
            .map(|entry| entry.count)
            .unwrap_or(0)
    }

    /// Bump one session's count and flush the store. Best-effort by
    /// design: the write goes through a unique temp file and a rename
    /// — the index's own discipline, so a reader never sees a
    /// half-written store and two fleet processes cannot collide on
    /// one temp — and a failed write is one logged line.
    ///
    /// The whole read-modify-write runs under an exclusive lock on a
    /// sibling `.lock` beside the store (KI-235). The temp-plus-rename
    /// discipline prevents a torn file but not a lost update: two
    /// concurrent recalls each read the old count and each write back
    /// one increment, and the last rename wins. The lock is what makes
    /// the increment atomic across processes.
    pub fn bump(sessions_dir: &Path, session_id: &str) {
        let _guard = crate::filelock::FileLock::take(&crate::filelock::FileLock::beside(
            &counts_path(sessions_dir),
        ));
        let mut counts = Self::load(sessions_dir);
        let entry = counts.sessions.entry(session_id.to_string()).or_default();
        entry.count += 1;
        entry.last = calib::today();
        counts.write(sessions_dir);
    }

    /// Write the store through a unique temp file and a rename. A
    /// failed write costs the next load the previous store — the
    /// increment is lost, the history is not — and one line names it.
    fn write(&self, sessions_dir: &Path) {
        let Ok(text) = serde_json::to_string(self) else {
            return;
        };
        let path = counts_path(sessions_dir);
        let serial = COUNTS_WRITE_SERIAL.fetch_add(1, Ordering::Relaxed);
        let mut temp = path.clone().into_os_string();
        temp.push(format!(".{}.{}.tmp", std::process::id(), serial));
        let temp = PathBuf::from(temp);
        if fs::write(&temp, text).is_ok() {
            let _ = fs::rename(&temp, &path);
        } else {
            eprintln!(
                "fleet: sessions: the recall-count store could not be written \u{2014} the recall is unaffected"
            );
        }
    }
}

/// A per-write serial for the store's temp name, so two fleet
/// processes — or two recalls in one process — never share a temp
/// file. The index's own serial, mirrored for the second store.
static COUNTS_WRITE_SERIAL: AtomicUsize = AtomicUsize::new(0);

/// One recalled session's feedback: bump the count store beside the
/// archive and land one `recall_hit` row naming the session id, the
/// shape it surfaced under, and the score it ranked at — through the
/// stream's own record path, the same row the facts corpus lands.
///
/// This is the archive's own loop, and it closes the one the corpus
/// runs without touching what makes a session synthetic: the store is
/// the only writer of a session's count, the log is never opened for
/// writing, and [`memory::Memory::recall_hits`] is still never routed
/// a synthetic fact — a synthetic fact's `path` is the session log,
/// and the corpus's bump would rewrite it. The next derivation reads
/// the count back through [`RecallCounts`] and seeds the synthetic
/// fact's `recall_count` from it, so the history channel reads a
/// recalled session's earned count exactly as the corpus's own.
///
/// The row's `fact` field names a session id here rather than a
/// memories stem — the one field's meaning widened rather than a new
/// op invented, so the shared devdec vocabulary does not grow for one
/// beat's feedback — and its `shape` carries the session's outcome
/// word. Never errors: a failed store write is one logged line, and
/// the row rides the stream's own never-blocks contract.
pub fn record_hit(
    sessions_dir: &Path,
    session_id: &str,
    shape: &str,
    score: f64,
    stream: Option<&calib::Stream>,
) {
    RecallCounts::bump(sessions_dir, session_id);
    if let Some(stream) = stream {
        stream.record(calib::Row::RecallHit(calib::RecallHitRow {
            ts: calib::stamp(),
            fact: session_id.to_string(),
            shape: shape.to_string(),
            score,
        }));
    }
}

// --- the hit --------------------------------------------------------------

/// What one ranked session carries for the surface: flat and public,
/// because the parent renders it. The session id, the composite
/// score, the per-channel scores the card renders, the derived
/// phrases, the outcome word, and the task text — everything a card
/// shows, and nothing it has to go back to the ranker for.
#[derive(Debug, Clone, PartialEq)]
pub struct ScoredHit {
    /// The session id — the log's file stem.
    pub id: String,
    /// The composite the ranking ordered by.
    pub score: f64,
    /// The vector channel: the cosine between the instruction's
    /// embedding and this session's indexed text.
    pub vector: f64,
    /// The keyword channel: how much of the instruction the task text
    /// answers.
    pub keyword: f64,
    /// The trajectory channel: how much of the instruction's
    /// trajectory the successful calls answer — the largest channel,
    /// and the one that reads the session's own record.
    pub trajectory: f64,
    /// The history channel: recall-count over the ceiling.
    pub history: f64,
    /// The freshness channel: exponential decay from the session's own
    /// day.
    pub freshness: f64,
    /// The provenance channel: the basis, half-strength for every
    /// session — the archive reads provenance from the outcome grade
    /// instead, which rides [`ScoredHit::outcome`].
    pub provenance: f64,
    /// The derived phrases: the successful calls, most recent first.
    pub phrases: Vec<String>,
    /// The outcome word — `resolved`, `incomplete`, or `diverged` —
    /// the sign vocabulary's own word, read off the session's ending.
    pub outcome: String,
    /// The task text: the first user message, bounded.
    pub task: String,
}

/// One ranked fact as one hit: the score's channels flattened onto the
/// hit, and the outcome word read back off the shape it rode in on.
fn hit_of(scored: &memory::ScoredFact) -> ScoredHit {
    ScoredHit {
        id: scored.fact.name.clone(),
        score: scored.score.total,
        vector: scored.score.vector,
        keyword: scored.score.keyword,
        trajectory: scored.score.trajectory,
        history: scored.score.history,
        freshness: scored.score.freshness,
        provenance: scored.score.provenance,
        phrases: scored.fact.phrases.clone(),
        outcome: scored.fact.shape.first().cloned().unwrap_or_default(),
        task: scored.fact.description.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    /// A scratch directory, unique per call — a sessions directory for
    /// the index's tests, a home for the derivation's.
    fn scratch(tag: &str) -> PathBuf {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let serial = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "fleet-session-recall-{tag}-{}-{serial}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// One tool call in the shape the log carries.
    fn call(id: &str, name: &str, arguments: &str) -> crate::api::ToolCall {
        crate::api::ToolCall {
            id: id.to_string(),
            kind: "function".to_string(),
            function: FunctionCall {
                name: name.to_string(),
                arguments: arguments.to_string(),
            },
        }
    }

    /// The log's own first line.
    fn start() -> Record {
        Record::Start {
            v: 1,
            model: "m".to_string(),
            created: "2026-10-15T00:00:00Z".to_string(),
            kind: None,
            parent: None,
            workspace: None,
        }
    }

    /// One assistant call and its result, as the agent writes them.
    fn exchange(id: &str, name: &str, arguments: &str, output: &str) -> Vec<Record> {
        vec![
            Record::Assistant {
                content: None,
                tool_calls: vec![call(id, name, arguments)],
            },
            Record::ToolResult {
                call_id: id.to_string(),
                name: name.to_string(),
                output: output.to_string(),
                truncated: false,
                approved: true,
            },
        ]
    }

    /// Write one session log into `dir` under `id`, the way the log
    /// writer lays it down: one JSON record per line.
    fn write_session(dir: &Path, id: &str, records: &[Record]) -> PathBuf {
        fs::create_dir_all(dir).unwrap();
        let path = dir.join(format!("{id}.jsonl"));
        let mut text = String::new();
        for record in records {
            text.push_str(&serde_json::to_string(record).unwrap());
            text.push('\n');
        }
        fs::write(&path, text).unwrap();
        path
    }

    /// A deterministic embedder over a fixed scripted vocabulary: each
    /// text embeds as the indicator of which scripted terms it carries,
    /// so a cosine is a scripted-token overlap. It shares its call
    /// counter with the test — which is how the index tests prove a
    /// rebuild embeds only what changed — and serves a settable
    /// dimensionality, which is how the model-swap test changes the
    /// index key. The recall index's own test embedder, mirrored.
    struct ScriptedEmbedder {
        calls: Arc<AtomicUsize>,
        dims: usize,
    }

    /// The scripted vocabulary, in dimension order.
    const SCRIPTED_TERMS: [&str; 6] = ["cargo", "test", "session", "recall", "walnut", "render"];

    impl ScriptedEmbedder {
        /// The shared counter every spawned embedder increments.
        fn counter() -> Arc<AtomicUsize> {
            Arc::new(AtomicUsize::new(0))
        }

        /// One embedder sharing `calls`, at `dims` dimensions. A fresh
        /// box per recall, because `SessionIndex::recall` takes the
        /// embedder by value the way `Memory::load_with` does.
        fn spawn(calls: &Arc<AtomicUsize>, dims: usize) -> Box<ScriptedEmbedder> {
            Box::new(ScriptedEmbedder {
                calls: Arc::clone(calls),
                dims,
            })
        }
    }

    impl memory::Embedder for ScriptedEmbedder {
        fn box_clone(&self) -> Box<dyn memory::Embedder> {
            Box::new(ScriptedEmbedder {
                calls: Arc::clone(&self.calls),
                dims: self.dims,
            })
        }

        fn embed(&self, text: &str) -> Option<Vec<f32>> {
            self.calls.fetch_add(1, Ordering::Relaxed);
            let lower = text.to_lowercase();
            Some(
                SCRIPTED_TERMS
                    .iter()
                    .take(self.dims)
                    .map(|term| if lower.contains(term) { 1.0 } else { 0.0 })
                    .collect(),
            )
        }

        fn name(&self) -> &'static str {
            "scripted"
        }

        fn serves_vectors(&self) -> bool {
            true
        }

        /// The index key this embedder writes: its name and the vector
        /// length it serves — the pair a model swap changes.
        fn vector_info(&self) -> Option<(String, usize)> {
            Some(("scripted".to_string(), self.dims))
        }
    }

    #[test]
    fn facts_from_sessions_derives_the_task_the_bounded_digest_and_the_successful_phrases() {
        let home = scratch("derive");
        let dir = crate::home::ensure_sessions_dir(&home).unwrap();
        let mut records = vec![
            start(),
            Record::User {
                content: "wire the session recall surface".to_string(),
            },
        ];
        // The successful call and the failed one: the same tool, the
        // same shape, opposite outcomes.
        records.extend(exchange(
            "c1",
            "bash",
            "{\"command\":\"cargo test --lib session_recall\"}",
            "test result: ok. 6 passed; 0 failed",
        ));
        records.extend(exchange(
            "c2",
            "bash",
            "{\"command\":\"rm -rf target\"}",
            "error: refused",
        ));
        records.push(Record::End {
            stop_reason: "completed".to_string(),
            turns: 2,
            served_model: None,
        });
        write_session(&dir, "20261015T000000Z-aaa", &records);

        let facts = facts_from_sessions(&home);
        assert_eq!(facts.len(), 1, "{facts:?}");
        let fact = &facts[0];
        assert_eq!(fact.name, "20261015T000000Z-aaa");
        // The task is the first user message, and the rest of the
        // fact's identity rides the design: a session is evidence this
        // run produced, durable, its own type, stamped by its log.
        assert_eq!(fact.description, "wire the session recall surface");
        assert_eq!(fact.basis, memory::Basis::Inferred);
        assert_eq!(fact.tier, memory::Tier::Durable);
        assert_eq!(fact.fact_type, "session");
        assert_eq!(fact.date, "2026-10-15");
        assert!(fact.stamp.is_some());
        assert!(fact.valid_until.is_empty());
        assert!(fact.missing.is_empty());
        // The phrases are the direction: the successful call's own
        // command, and the failed one nowhere in sight — it drove
        // somewhere else.
        assert_eq!(
            fact.phrases,
            vec!["cargo test --lib session_recall".to_string()]
        );
        // The digest is the record: the task, then every call's name
        // and its compacted arguments, the failed one included.
        assert!(
            fact.body
                .starts_with("task: wire the session recall surface"),
            "{}",
            fact.body
        );
        assert!(
            fact.body
                .contains("bash {\"command\":\"cargo test --lib session_recall\"}"),
            "{}",
            fact.body
        );
        assert!(
            fact.body.contains("bash {\"command\":\"rm -rf target\"}"),
            "{}",
            fact.body
        );

        // The digest is bounded: a long session's log is large, and
        // the cap is a named constant with a marked cut. The task is
        // bounded the same way, to the writer's own description bound.
        let mut long: Vec<Record> = vec![
            start(),
            Record::User {
                content: "x".repeat(2000),
            },
        ];
        for index in 0..200 {
            long.extend(exchange(
                &format!("c{index}"),
                "bash",
                "{\"command\":\"echo aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\"}",
                "ok",
            ));
        }
        long.push(Record::End {
            stop_reason: "completed".to_string(),
            turns: 200,
            served_model: None,
        });
        write_session(&dir, "20261015T000001Z-bbb", &long);
        let facts = facts_from_sessions(&home);
        let capped = facts
            .iter()
            .find(|fact| fact.name == "20261015T000001Z-bbb")
            .unwrap();
        assert!(
            capped.body.chars().count() <= DIGEST_CAP + 1,
            "{} chars",
            capped.body.chars().count()
        );
        assert!(capped.body.ends_with('\u{2026}'), "{}", capped.body);
        assert!(
            capped.description.chars().count() <= memory::MAX_DESCRIPTION_CHARS + 1,
            "{} chars",
            capped.description.chars().count()
        );

        // A log that does not replay is skipped by id, and a non-log
        // file is not a session at all — the others still load.
        fs::write(dir.join("bad.jsonl"), "not json\n").unwrap();
        fs::write(dir.join("notes.txt"), "not a log").unwrap();
        assert_eq!(
            facts_from_sessions(&home).len(),
            2,
            "the two readable sessions"
        );
    }

    #[test]
    fn the_outcome_maps_onto_the_sign_vocabulary_the_ranker_carries() {
        let home = scratch("outcome");
        let dir = crate::home::ensure_sessions_dir(&home).unwrap();
        // The log's own ending vocabulary, one session each:
        // completed is a resolution, max-turns drove the right
        // direction and stalled, and everything else — failed, an
        // owner's stop, and the interrupted shape of a log with no
        // `end` at all — diverged.
        let endings: [(Option<&str>, &str); 5] = [
            (Some("completed"), "resolved"),
            (Some("max-turns"), "incomplete"),
            (Some("failed"), "diverged"),
            (Some("stopped by the owner"), "diverged"),
            (None, "diverged"),
        ];
        for (index, (stop, _)) in endings.iter().enumerate() {
            let mut records = vec![
                start(),
                Record::User {
                    content: "a task".to_string(),
                },
            ];
            if let Some(stop) = stop {
                records.push(Record::End {
                    stop_reason: stop.to_string(),
                    turns: 1,
                    served_model: None,
                });
            }
            write_session(&dir, &format!("s{index}"), &records);
        }
        let facts = facts_from_sessions(&home);
        let shapes: Vec<Vec<String>> = facts.iter().map(|fact| fact.shape.clone()).collect();
        let expected: Vec<Vec<String>> = endings
            .iter()
            .map(|(_, word)| vec![word.to_string()])
            .collect();
        assert_eq!(shapes, expected);
        // And the words are the ranker's own three: no new sign
        // vocabulary was invented for the archive.
        let vocabulary = [
            memory::Sign::Resolution.word(),
            memory::Sign::Incomplete.word(),
            memory::Sign::Divergence.word(),
        ];
        for fact in &facts {
            let word = fact.shape.first().map(String::as_str).unwrap_or_default();
            assert!(
                vocabulary.contains(&word),
                "`{word}` is not the sign vocabulary"
            );
        }
    }

    #[test]
    fn the_index_round_trips_and_a_stamp_change_re_embeds_only_that_session() {
        let dir = scratch("index");
        let mut one = vec![
            start(),
            Record::User {
                content: "first".to_string(),
            },
        ];
        one.extend(exchange("c1", "bash", "{\"command\":\"cargo test\"}", "ok"));
        one.push(Record::End {
            stop_reason: "completed".to_string(),
            turns: 1,
            served_model: None,
        });
        write_session(&dir, "s1", &one);
        let mut two = vec![
            start(),
            Record::User {
                content: "second".to_string(),
            },
        ];
        two.extend(exchange(
            "c2",
            "list_dir",
            "{\"path\":\"/tmp\"}",
            "notes.txt",
        ));
        two.push(Record::End {
            stop_reason: "completed".to_string(),
            turns: 1,
            served_model: None,
        });
        write_session(&dir, "s2", &two);

        let calls = ScriptedEmbedder::counter();
        // The first recall embeds both sessions — and the instruction
        // itself, which the ranker embeds through the same backend.
        let mut index = SessionIndex::load(&dir);
        let _ = index.recall(&dir, ScriptedEmbedder::spawn(&calls, 4), "anything");
        assert_eq!(
            calls.load(Ordering::Relaxed),
            3,
            "two sessions and the instruction"
        );
        assert!(index_path(&dir).exists(), "the cache landed on disk");

        // A second load reads the cache through its file: nothing
        // re-embeds but the instruction.
        let mut reloaded = SessionIndex::load(&dir);
        assert!(reloaded.keyed_for(&("scripted".to_string(), 4)));
        let _ = reloaded.recall(&dir, ScriptedEmbedder::spawn(&calls, 4), "anything");
        assert_eq!(
            calls.load(Ordering::Relaxed),
            4,
            "both sessions came from the cache"
        );

        // Change one session's log: only it re-embeds. The stamp is the
        // log's own secs:nanos:len, so a rewritten file is a changed
        // session and an untouched one keeps its vector.
        let mut longer = one.clone();
        longer.push(Record::User {
            content: "and one more turn".to_string(),
        });
        write_session(&dir, "s1", &longer);
        let mut third = SessionIndex::load(&dir);
        let _ = third.recall(&dir, ScriptedEmbedder::spawn(&calls, 4), "anything");
        assert_eq!(
            calls.load(Ordering::Relaxed),
            6,
            "the changed session re-embedded, the untouched one did not, and the instruction always embeds"
        );
    }

    #[test]
    fn an_index_keyed_to_another_embedder_is_discarded_whole() {
        let dir = scratch("keyed");
        let mut records = vec![
            start(),
            Record::User {
                content: "one".to_string(),
            },
        ];
        records.extend(exchange("c1", "bash", "{\"command\":\"cargo test\"}", "ok"));
        records.push(Record::End {
            stop_reason: "completed".to_string(),
            turns: 1,
            served_model: None,
        });
        write_session(&dir, "s1", &records);

        let calls = ScriptedEmbedder::counter();
        let mut first = SessionIndex::load(&dir);
        let _ = first.recall(&dir, ScriptedEmbedder::spawn(&calls, 4), "anything");
        let after_first = calls.load(Ordering::Relaxed);

        // The model swap: the same archive under an embedder of another
        // dimensionality. The cache is not this embedder's, so it goes
        // whole — stale-dimension vectors would cosine 0.0 and silently
        // kill the vector channel while the surface named it live.
        let mut second = SessionIndex::load(&dir);
        let _ = second.recall(&dir, ScriptedEmbedder::spawn(&calls, 5), "anything");
        assert_eq!(
            calls.load(Ordering::Relaxed) - after_first,
            2,
            "the session re-embedded, and the instruction with it"
        );
        // And the rewritten cache is keyed to the embedder that wrote it.
        assert_eq!(second.embedder, "scripted");
        assert_eq!(second.dims, 5);
        assert_eq!(second.sessions.len(), 1);
    }

    #[test]
    fn a_malformed_index_reads_as_empty_and_never_panics() {
        let dir = scratch("malformed");
        let mut records = vec![
            start(),
            Record::User {
                content: "one".to_string(),
            },
        ];
        records.extend(exchange("c1", "bash", "{\"command\":\"cargo test\"}", "ok"));
        records.push(Record::End {
            stop_reason: "completed".to_string(),
            turns: 1,
            served_model: None,
        });
        write_session(&dir, "s1", &records);
        // The cache a crashed writer could leave: half a file, not a
        // file. It reads as empty and costs a recompute.
        fs::write(index_path(&dir), "{\"embedder\": \"scripted\", ").unwrap();

        let calls = ScriptedEmbedder::counter();
        let mut index = SessionIndex::load(&dir);
        assert!(index.sessions.is_empty(), "a malformed index read as empty");
        let _ = index.recall(&dir, ScriptedEmbedder::spawn(&calls, 4), "anything");
        // The recompute happened, and the rebuilt cache is on disk.
        assert_eq!(
            calls.load(Ordering::Relaxed),
            2,
            "the session embedded from scratch"
        );
        assert_eq!(index.sessions.len(), 1);
        assert_eq!(
            SessionIndex::load(&dir).sessions.len(),
            1,
            "the rebuilt cache round-trips"
        );
    }

    #[test]
    fn recall_ranks_the_session_whose_phrases_match_the_instruction() {
        let dir = scratch("recall");
        // Session A walked the trajectory the instruction asks about.
        let mut a = vec![
            start(),
            Record::User {
                content: "prove the session archive ranks".to_string(),
            },
        ];
        a.extend(exchange(
            "c1",
            "bash",
            "{\"command\":\"cargo test --lib session_recall\"}",
            "test result: ok. 6 passed; 0 failed",
        ));
        a.push(Record::End {
            stop_reason: "completed".to_string(),
            turns: 1,
            served_model: None,
        });
        write_session(&dir, "a-session", &a);
        // Session B walked somewhere else entirely.
        let mut b = vec![
            start(),
            Record::User {
                content: "water the plants".to_string(),
            },
        ];
        b.extend(exchange(
            "c9",
            "list_dir",
            "{\"path\":\"/tmp\"}",
            "notes.txt",
        ));
        b.push(Record::End {
            stop_reason: "completed".to_string(),
            turns: 1,
            served_model: None,
        });
        write_session(&dir, "b-session", &b);

        let mut index = SessionIndex::load(&dir);
        // Keyword-only on purpose: the proof is that the DIRECTION
        // channel reads the session's own record, with no vector path
        // to carry it.
        let hits = index.recall(
            &dir,
            Box::new(memory::KeywordOnly),
            "cargo test --lib session_recall",
        );
        assert_eq!(hits.len(), 1, "{hits:?}");
        let hit = &hits[0];
        assert_eq!(hit.id, "a-session");
        assert!(
            hit.trajectory > 0.0,
            "the direction channel read the record: {hit:?}"
        );
        assert_eq!(
            hit.phrases,
            vec!["cargo test --lib session_recall".to_string()]
        );
        assert_eq!(hit.outcome, "resolved");
        assert_eq!(hit.task, "prove the session archive ranks");
        assert!(hit.score > 0.0);
        // B is below the floor: no direction, no words, standing alone.
        assert!(!hits.iter().any(|hit| hit.id == "b-session"));
    }

    // --- the count store ---

    /// One completed session log under `dir`: a task, one successful
    /// call, and a `completed` ending — the smallest log the
    /// derivation ranks.
    fn one_session(dir: &Path, id: &str, task: &str) -> PathBuf {
        let mut records = vec![
            start(),
            Record::User {
                content: task.to_string(),
            },
        ];
        records.extend(exchange("c1", "bash", "{\"command\":\"cargo test\"}", "ok"));
        records.push(Record::End {
            stop_reason: "completed".to_string(),
            turns: 1,
            served_model: None,
        });
        write_session(dir, id, &records)
    }

    #[test]
    fn a_recalled_session_bumps_the_store_lands_one_row_and_never_touches_the_log() {
        let dir = scratch("counts");
        let log = one_session(&dir, "20261015T000000Z-aaa", "wire the recall counts");
        let other = one_session(&dir, "20261015T000001Z-bbb", "another session");
        // The logs' bytes before: the append-only property is proven
        // by comparing them after, byte for byte.
        let before: Vec<Vec<u8>> = [&log, &other]
            .iter()
            .map(|path| fs::read(path).unwrap())
            .collect();
        assert!(
            !counts_path(&dir).exists(),
            "no store before the first recall"
        );

        let stream_root = scratch("counts-stream");
        let stream = calib::Stream::open(&stream_root);
        record_hit(&dir, "20261015T000000Z-aaa", "resolved", 0.5, Some(&stream));
        record_hit(&dir, "20261015T000000Z-aaa", "resolved", 0.5, Some(&stream));
        drop(stream);

        // The store: 0 -> 1 -> 2, keyed by session id, and the other
        // session untouched.
        let counts = RecallCounts::load(&dir);
        assert_eq!(counts.count_of("20261015T000000Z-aaa"), 2);
        assert_eq!(
            counts.count_of("20261015T000001Z-bbb"),
            0,
            "the other session was not recalled"
        );
        let text = fs::read_to_string(counts_path(&dir)).unwrap();
        assert!(text.contains("\"count\":2"), "{text}");

        // The logs: byte-identical. The store is the only writer of a
        // session's count; an append-only log is never rewritten.
        for (path, bytes) in [&log, &other].iter().zip(&before) {
            assert_eq!(
                &fs::read(path).unwrap(),
                bytes,
                "{} changed",
                path.display()
            );
        }

        // The row: one recall_hit per hit, naming the session id, its
        // shape, and its score — through the stream's own path.
        let rows = calib::read_day(&stream_root, &calib::today(), 64);
        let hits: Vec<_> = rows
            .iter()
            .filter(|row| row.op == calib::OP_RECALL_HIT)
            .collect();
        assert_eq!(hits.len(), 2, "{hits:?}");
        assert_eq!(hits[0].fact, "20261015T000000Z-aaa");
        assert_eq!(hits[0].shape, "resolved");
        assert!((hits[0].score - 0.5).abs() < 1e-12, "{hits:?}");
        let _ = fs::remove_dir_all(&dir);
        let _ = fs::remove_dir_all(&stream_root);
    }

    #[test]
    fn the_store_seeds_the_synthetic_facts_count_and_the_history_channel_reads_it() {
        let dir = scratch("seed");
        one_session(
            &dir,
            "20261015T000000Z-aaa",
            "prove the store seeds the count",
        );
        RecallCounts::bump(&dir, "20261015T000000Z-aaa");
        RecallCounts::bump(&dir, "20261015T000000Z-aaa");

        // The next derivation seeds the synthetic fact's recall_count
        // from the store — the corpus's own feedback loop, closed on
        // state that is not the log.
        let facts = facts_from_dir(&dir);
        assert_eq!(facts[0].recall_count, 2, "the count came from the store");

        // And the ranker reads it: the history channel is the count
        // over the ceiling, the same arithmetic the corpus's facts
        // ride, so a recalled session re-ranks on its own history.
        let mut index = SessionIndex::load(&dir);
        let hits = index.recall(
            &dir,
            Box::new(memory::KeywordOnly),
            "prove the store seeds the count",
        );
        let hit = hits
            .iter()
            .find(|hit| hit.id == "20261015T000000Z-aaa")
            .expect("the session ranked");
        assert!(
            hit.history > 0.0,
            "the history channel read the earned count: {hit:?}"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_malformed_store_reads_as_empty_and_a_failed_write_never_errors() {
        let dir = scratch("malformed-counts");
        one_session(&dir, "20261015T000000Z-aaa", "one session");
        // The store a crashed writer could leave: half a file, not a
        // file. It reads as empty and costs the session its earned
        // count until the next recall — never a panic, never an error.
        fs::write(counts_path(&dir), "{\"sessions\": {\"2026").unwrap();
        let counts = RecallCounts::load(&dir);
        assert_eq!(
            counts.count_of("20261015T000000Z-aaa"),
            0,
            "a malformed store read as empty"
        );
        assert_eq!(
            facts_from_dir(&dir)[0].recall_count,
            0,
            "the derivation read it as empty too"
        );

        // A failed write is one logged line and never an error: a
        // sessions directory that does not exist refuses the temp
        // file, and the recall it books for is unaffected.
        let absent = dir.join("no-such-dir");
        record_hit(&absent, "20261015T000000Z-aaa", "resolved", 0.5, None);
        assert!(
            !counts_path(&absent).exists(),
            "the failed write left no store"
        );
        // The readable store beside it still works.
        RecallCounts::bump(&dir, "20261015T000000Z-aaa");
        assert_eq!(RecallCounts::load(&dir).count_of("20261015T000000Z-aaa"), 1);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_guard_holds_the_store_is_the_only_writer_of_a_sessions_count() {
        let dir = scratch("guard");
        let log = one_session(&dir, "20261015T000000Z-aaa", "the read-only synthetic fact");
        let bytes = fs::read(&log).unwrap();

        // The synthetic fact's path IS the session log — the reason
        // the corpus's own bump must never be routed at it.
        let facts = facts_from_dir(&dir);
        assert_eq!(facts[0].path, log);

        // The archive's own loop: the recall bumps the store and the
        // row rides the stream, and the log the fact points at is
        // untouched — byte for byte, with no recall-count line
        // written into it the way the corpus's bump writes a fact's
        // file.
        record_hit(&dir, "20261015T000000Z-aaa", "resolved", 0.4, None);
        assert_eq!(fs::read(&log).unwrap(), bytes, "the log was rewritten");
        let text = String::from_utf8(fs::read(&log).unwrap()).unwrap();
        assert!(
            !text.contains("recall-count"),
            "the log carries a count line: {text}"
        );
        assert_eq!(RecallCounts::load(&dir).count_of("20261015T000000Z-aaa"), 1);
        let _ = fs::remove_dir_all(&dir);
    }

    /// KI-235: the temp-plus-rename discipline prevents a torn file but
    /// not a lost update. Two concurrent recalls each read the old count
    /// and each write back one increment, and the last rename wins — so
    /// without the lock around the whole read-modify-write, one of the
    /// two increments is silently dropped. Two threads racing the same
    /// store is the in-process face of the same race.
    #[test]
    fn two_concurrent_bumps_both_land() {
        let dir = scratch("counts-race");
        let id = "20261015T000000Z-aaa";
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
        let mut handles = Vec::new();
        for _ in 0..2 {
            let dir = dir.clone();
            let barrier = std::sync::Arc::clone(&barrier);
            handles.push(std::thread::spawn(move || {
                barrier.wait();
                RecallCounts::bump(&dir, id);
            }));
        }
        for handle in handles {
            handle.join().expect("a bump thread panicked");
        }
        assert_eq!(
            RecallCounts::load(&dir).count_of(id),
            2,
            "both increments must land — a lost update is the race the lock closes"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    /// One summary row for the archive card tests.
    fn row(id: &str, workspace: Option<&str>) -> crate::session::SessionSummary {
        crate::session::SessionSummary {
            id: id.to_string(),
            model: "deepseek-flash".to_string(),
            created: "2026-10-15T00:00:00Z".to_string(),
            ended: true,
            kind: None,
            workspace: workspace.map(str::to_string),
            workspace_inferred: false,
            channel: None,
            compacted: false,
            served_model: None,
            path: std::path::PathBuf::from(format!("/tmp/{id}.jsonl")),
        }
    }

    #[test]
    fn the_archive_names_the_project_a_session_was_born_in() {
        let home = std::env::var("HOME").unwrap_or_default();
        let project = format!("{home}/projects/fleet");
        let rendered = render_list(&[
            row("20261015T000000Z-aaa", Some(&project)),
            row("20261015T000001Z-bbb", None),
        ]);
        // The home prefix folds to `~`: every row would otherwise start
        // with the same six characters.
        assert!(rendered.contains("~/projects/fleet"), "{rendered}");
        // A log without the field simply carries no project line — the
        // grouping is absent, the row is not.
        assert!(rendered.contains("20261015T000001Z-bbb"), "{rendered}");
        let after = rendered.split("20261015T000001Z-bbb").nth(1).unwrap();
        let row_text = after.split('\u{250c}').next().unwrap();
        assert!(!row_text.contains("projects/fleet"), "{row_text}");
    }

    #[test]
    fn a_project_outside_the_home_is_shown_whole() {
        let rendered = render_list(&[row("20261015T000000Z-aaa", Some("/srv/elsewhere/app"))]);
        assert!(rendered.contains("/srv/elsewhere/app"), "{rendered}");
    }
}

// --- the surfaces ----------------------------------------------------------

/// The card width, matching `lanes` and `calib`.
const CARD_WIDTH: usize = 72;

mod palette {
    pub const ACCENT: &str = "\u{1b}[36m";
    pub const OK: &str = "\u{1b}[32m";
    pub const WARN: &str = "\u{1b}[33m";
    pub const ERROR: &str = "\u{1b}[31m";
    pub const DIM: &str = "\u{1b}[90m";
    pub const TEXT: &str = "\u{1b}[37m";
    pub const BOLD: &str = "\u{1b}[1m";
    pub const RESET: &str = "\u{1b}[0m";
}

use palette::{ACCENT, BOLD, DIM, ERROR, OK, RESET, TEXT, WARN};

/// The outcome word's color: a resolved session is evidence, a
/// diverged one is an anti-vector, and an incomplete one is a
/// trajectory that stalled — three states, three colors, never one
/// undifferentiated "session" tone.
fn outcome_color(outcome: &str) -> &'static str {
    match outcome {
        "resolved" => OK,
        "diverged" => ERROR,
        _ => WARN,
    }
}

/// `fleet sessions --recall`: the ranked archive as cards, never a
/// table. Each card names the session, its outcome, the six channels
/// the composite was built from, and the derived phrases — the
/// successful calls that drove toward the answer, which is the
/// direction a later session reads.
pub fn render_hits(hits: &[ScoredHit], instruction: &str) -> String {
    let mut out = format!(
        "{BOLD}fleet sessions --recall{RESET} {DIM}\u{2014} the archive ranked for \u{201c}{instruction}\u{201d}{RESET}\n\n"
    );
    if hits.is_empty() {
        out.push_str(&format!(
            "{DIM}no session in the archive answers that \u{b7} the corpus grows with every run{RESET}\n"
        ));
        return out;
    }
    for hit in hits {
        let title = &hit.id;
        let header_right = format!("{} {}", outcome_color(&hit.outcome), hit.outcome);
        let header = format!(
            "{DIM}\u{250c}\u{2500}{RESET} {BOLD}{ACCENT}{title}{RESET} {DIM}{dashes} {header_right} {RESET}{DIM}\u{2510}{RESET}",
            dashes = dashes_for(&format!(" {title} "), &hit.outcome),
        );
        out.push_str(&header);
        out.push('\n');
        out.push_str(&format!(
            "{DIM}\u{2502}{RESET} {TEXT}score {score:.2} \u{b7} vector {vector:.2} \u{b7} keyword {keyword:.2} \u{b7} trajectory {trajectory:.2}{RESET}\n",
            score = hit.score,
            vector = hit.vector,
            keyword = hit.keyword,
            trajectory = hit.trajectory,
        ));
        out.push_str(&format!(
            "{DIM}\u{2502}{RESET} {DIM}history {history:.2} \u{b7} freshness {freshness:.2} \u{b7} provenance {provenance:.2}{RESET}\n",
            history = hit.history,
            freshness = hit.freshness,
            provenance = hit.provenance,
        ));
        let task: String = hit.task.chars().take(CARD_WIDTH - 12).collect();
        out.push_str(&format!("{DIM}\u{2502}{RESET} {TEXT}{task}{RESET}\n"));
        if !hit.phrases.is_empty() {
            out.push_str(&format!(
                "{DIM}\u{2502}{RESET} {DIM}drove: {}{RESET}\n",
                hit.phrases.join(" \u{b7} ")
            ));
        }
        out.push_str(&format!(
            "{DIM}\u{2514}{}\u{2518}{RESET}\n",
            "\u{2500}".repeat(CARD_WIDTH - 2)
        ));
    }
    out.push_str(&format!(
        "\n{DIM}{} session{} ranked \u{b7} the trajectory channel reads the record itself{RESET}\n",
        hits.len(),
        if hits.len() == 1 { "" } else { "s" }
    ));
    out
}

/// `fleet sessions`: the archive as cards, replacing the plain-text
/// table the surface used to print. A session is a piece of work with
/// an outcome, not a row. Newest first: the reader wants the work they
/// just did, and `session::list` hands them over oldest first.
pub fn render_list(summaries: &[crate::session::SessionSummary]) -> String {
    let mut out =
        format!("{BOLD}fleet sessions{RESET} {DIM}\u{2014} the archive, newest first{RESET}\n\n");
    if summaries.is_empty() {
        out.push_str(&format!(
            "{DIM}no sessions yet \u{b7} run `fleet run \"<task>\"` to start one{RESET}\n"
        ));
        return out;
    }
    for summary in summaries.iter().rev() {
        let kind = summary.kind.as_deref().unwrap_or("session");
        let markers = if summary.compacted {
            format!("{kind} \u{b7} compacted")
        } else {
            kind.to_string()
        };
        let state_color = if summary.ended { OK } else { WARN };
        let state = if summary.ended { "ended" } else { "open" };
        let header = format!(
            "{DIM}\u{250c}\u{2500}{RESET} {BOLD}{ACCENT}{id}{RESET} {DIM}{dashes} {state_color}{state}{RESET} {DIM}\u{2510}{RESET}",
            id = summary.id,
            dashes = dashes_for(&format!(" {} ", summary.id), state),
        );
        out.push_str(&header);
        out.push('\n');
        out.push_str(&format!(
            "{DIM}\u{2502}{RESET} {TEXT}{created} \u{b7} {model} \u{b7} {markers}{RESET}\n",
            created = summary.created,
            model = summary.model,
            markers = markers,
        ));
        // The project, when the log records one: the archive groups by
        // it, and a reader scanning for "the fleet work" should not have
        // to open a session to learn where it ran. The home prefix folds
        // to `~`, because every row would otherwise start with the same
        // six characters.
        if let Some(workspace) = &summary.workspace {
            out.push_str(&format!(
                "{DIM}\u{2502}{RESET} {DIM}{workspace}{RESET}\n",
                workspace = shorten_home(workspace),
            ));
        }
        out.push_str(&format!(
            "{DIM}\u{2514}{}\u{2518}{RESET}\n",
            "\u{2500}".repeat(CARD_WIDTH - 2)
        ));
    }
    out.push_str(&format!(
        "\n{DIM}{} session{} \u{b7} `fleet sessions <id>` opens one \u{b7} `--recall \"<what>\"` ranks them{RESET}\n",
        summaries.len(),
        if summaries.len() == 1 { "" } else { "s" }
    ));
    out
}

/// The dash run that pads a card header so its right edge aligns.
fn dashes_for(left: &str, right: &str) -> String {
    let used = left.chars().count() + right.chars().count() + 2;
    "\u{2500}".repeat(CARD_WIDTH.saturating_sub(used + 4).max(1))
}

/// A workspace path with the home prefix folded to `~`. A path outside
/// the home is shown whole: the reader's own projects all live under it,
/// and a path that is not theirs is information, not noise.
fn shorten_home(workspace: &str) -> String {
    let home = std::env::var("HOME").ok().filter(|home| !home.is_empty());
    match home {
        Some(home) if workspace.starts_with(&format!("{home}/")) => {
            format!("~{}", &workspace[home.len()..])
        }
        _ => workspace.to_string(),
    }
}

/// `fleet sessions <id>`: one session's log as cards — the task, the
/// turns, the calls, and the outcome. The audit view shows the record,
/// not a summary of it: every tool call is named with its outcome, and
/// a failure is named rather than smoothed over, because a session
/// whose failures are hidden is a session nobody can learn from.
///
/// The budget state rides the header: the measured tokens against the
/// budget the automatic fold fires at, so the owner can see how close a
/// session is before it folds rather than finding out afterwards.
pub fn render_detail(id: &str, created: &str, records: &[crate::session::Record]) -> String {
    use crate::session::Record;
    let conversation = crate::session::conversation(records);
    let measured = crate::compaction::estimate_tokens(&conversation.messages);
    let budget = crate::compaction::COMPACTION_TOKEN_BUDGET;
    let pct = (measured as f64 / budget as f64 * 100.0).min(999.0);
    let mut out = format!(
        "{BOLD}fleet sessions{RESET} {DIM}\u{2014} {id} \u{b7} {created}{RESET}\n\n\
         {DIM}context ~{measured} tokens \u{b7} the fold fires at {budget} \u{b7} {pct:.0}% of it{RESET}\n\n"
    );
    let mut turns = 0usize;
    let mut calls = 0usize;
    let mut failed = 0usize;
    let mut last_outcome = String::new();
    for record in records {
        match record {
            Record::Start {
                model,
                kind,
                parent,
                ..
            } => {
                let mut line = format!("model {model}");
                if let Some(kind) = kind {
                    line.push_str(&format!(" \u{b7} {kind}"));
                }
                if let Some(parent) = parent {
                    line.push_str(&format!(" \u{b7} child of {parent}"));
                }
                out.push_str(&format!("{DIM}start \u{b7} {line}{RESET}\n"));
            }
            Record::User { content } => {
                turns += 1;
                let task: String = content.chars().take(CARD_WIDTH - 10).collect();
                out.push_str(&format!(
                    "\n{DIM}\u{250c}\u{2500}{RESET} {BOLD}{ACCENT}turn {turns}{RESET} {DIM}{dashes} the ask{RESET} {DIM}\u{2510}{RESET}\n",
                    dashes = dashes_for(&format!(" turn {turns} "), "the ask"),
                ));
                out.push_str(&format!("{DIM}\u{2502}{RESET} {TEXT}{task}{RESET}\n"));
            }
            Record::Assistant {
                content,
                tool_calls,
            } => {
                if let Some(content) = content {
                    let text: String = content.chars().take(CARD_WIDTH - 6).collect();
                    out.push_str(&format!("{DIM}\u{2502}{RESET} {TEXT}{text}{RESET}\n"));
                }
                for call in tool_calls {
                    calls += 1;
                    let args: String = call.function.arguments.chars().take(40).collect();
                    out.push_str(&format!(
                        "{DIM}\u{2502}{RESET} {DIM}\u{2192} {name} {args}{RESET}\n",
                        name = call.function.name,
                    ));
                }
            }
            Record::ToolResult {
                name,
                output,
                approved,
                ..
            } => {
                let failed_call = output.starts_with("error:") || output.starts_with("denied");
                if failed_call {
                    failed += 1;
                }
                let color = if failed_call {
                    ERROR
                } else if *approved {
                    OK
                } else {
                    WARN
                };
                let mark = if failed_call {
                    "failed"
                } else if *approved {
                    "approved"
                } else {
                    "unapproved"
                };
                let text: String = output.chars().take(CARD_WIDTH - 14).collect();
                out.push_str(&format!(
                    "{DIM}\u{2502}{RESET}   {color}{mark}{RESET} {DIM}{name} \u{b7} {text}{RESET}\n",
                ));
            }
            Record::End { stop_reason, .. } => {
                last_outcome = stop_reason.clone();
            }
            _ => {}
        }
    }
    if !last_outcome.is_empty() {
        let color = match last_outcome.as_str() {
            "completed" => OK,
            "max-turns" => WARN,
            _ => ERROR,
        };
        out.push_str(&format!(
            "\n{DIM}\u{2514}{}\u{2518}{RESET} {color}{stop}{RESET} {DIM}\u{b7} {turns} turn{turn_plural} \u{b7} {calls} call{call_plural} \u{b7} {failed} failed{RESET}\n",
            "\u{2500}".repeat(CARD_WIDTH - 2),
            stop = last_outcome,
            turn_plural = if turns == 1 { "" } else { "s" },
            call_plural = if calls == 1 { "" } else { "s" },
        ));
    }
    out
}
