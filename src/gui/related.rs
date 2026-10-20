//! Cross-kind related suggestions: one server-side index for the kinds
//! the recall caches do not already hold.
//!
//! The inspector's "related" block used to be a browser-side lexical
//! shortlist handed to `/api/rank`, which embedded each candidate per
//! request: skills and projects were never indexed at all, so a skill
//! could not surface from a fact and a project could not surface from a
//! session. This module owns the missing half — `<home>/.related-index.json`
//! for skills, projects, routes, and fit cells — and reads the two
//! caches that already exist for facts (`memories/.recall-index.json`,
//! through [`crate::memory::Memory::vector_of`]) and sessions
//! (`<sessions>/.recall-index.json`, through
//! [`crate::session_recall::SessionIndex::vectors_for`]). Nothing is
//! embedded twice and no cache is duplicated.
//!
//! ## The honest states
//!
//! Skills and projects are the cold half: a fresh home embeds 400-odd
//! skill descriptions on the first warm, which is tens of seconds. The
//! warm runs once, off the request path, in `spawn_blocking`; until it
//! lands a request answers from what is ready and says `index:"warming"`
//! so the browser can retry once. A machine with no embedding backend
//! answers lexically and says `scores:"lexical"`. Neither state is
//! silent: the card names what it is showing.
//!
//! ## The weak flag
//!
//! KI-058 measured unrelated pairs at 0.58-0.66 and related ones at
//! 0.77-0.93 on this machine's embedder. Raw cosine is returned and a
//! score under [`WEAK_BELOW`] is marked rather than filtered: a weak
//! relation is still evidence, and hiding it would make an empty list
//! mean two different things.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// The sidecar beside the home's other GUI-owned state. A cache, on the
/// recall index's discipline: an absent or malformed file reads as
/// empty and costs a recompute.
pub const INDEX_FILE: &str = ".related-index.json";

/// The schema version this reader writes and accepts. A file from a
/// newer writer is discarded whole rather than half-read.
const VERSION: u32 = 1;

/// How long a built corpus is reused. Facts and sessions are the parts
/// that cost a read; routes and cells are folded in on the same clock,
/// so a related request never replays every session log on the click.
const CORPUS_TTL: Duration = Duration::from_secs(60);

/// Below this cosine a pair is related but weak. Measured, not chosen:
/// KI-058's unrelated band is 0.58-0.66 and its related band 0.77-0.93,
/// so 0.72 is the gap.
pub const WEAK_BELOW: f64 = 0.72;

/// What a request gets when it does not ask.
pub const DEFAULT_LIMIT: usize = 3;

/// The most one card can use and still read as a suggestion list.
const MAX_LIMIT: usize = 8;

/// One cached vector with the digest of the text it was computed from,
/// plus the label the card renders. The label rides here so a warm
/// corpus needs no second read of the skill library or the projects
/// root — the cache is what the request reads.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Entry {
    digest: String,
    vector: Vec<f32>,
    name: String,
    summary: String,
    text: String,
}

/// The sidecar: one section per kind the recall caches do not own.
#[derive(Debug, Default, Serialize, Deserialize)]
struct Store {
    #[serde(default)]
    version: u32,
    #[serde(default)]
    embedder: String,
    #[serde(default)]
    dims: usize,
    #[serde(default)]
    skills: HashMap<String, Entry>,
    #[serde(default)]
    projects: HashMap<String, Entry>,
    #[serde(default)]
    routes: HashMap<String, Entry>,
    #[serde(default)]
    cells: HashMap<String, Entry>,
}

/// One thing a relation can point at, whichever cache its vector came
/// from. Built by [`RelatedIndex::corpus`] and cached.
#[derive(Debug, Clone)]
struct Candidate {
    kind: &'static str,
    id: String,
    name: String,
    summary: String,
    text: String,
    vector: Option<Vec<f32>>,
}

/// The kinds the endpoint accepts. An unknown kind is a 400 at the
/// boundary rather than an empty list that could mean "no relation".
pub const KINDS: [&str; 6] = ["fact", "skill", "session", "project", "route", "cell"];

/// The related index: the sidecar, the warm flag, and the built corpus.
pub struct RelatedIndex {
    home: PathBuf,
    memories: PathBuf,
    store: Arc<Mutex<Store>>,
    warming: Arc<AtomicBool>,
    corpus: Arc<Mutex<Option<(Instant, Vec<Candidate>)>>>,
}

impl RelatedIndex {
    /// Read the sidecar and hold it. Nothing is embedded here: the
    /// warm is the bridge's second step, so a construction on the
    /// server's own thread never waits on the embedding endpoint.
    pub fn new(home: PathBuf, memories: PathBuf) -> RelatedIndex {
        let store = read_store(&home);
        RelatedIndex {
            home,
            memories,
            store: Arc::new(Mutex::new(store)),
            warming: Arc::new(AtomicBool::new(false)),
            corpus: Arc::new(Mutex::new(None)),
        }
    }

    /// Warm the cold kinds once, off the request path. Called by the
    /// bridge after construction; a second call while the first is
    /// running is a no-op.
    pub fn warm(&self) {
        if self.warming.swap(true, Ordering::SeqCst) {
            return;
        }
        let home = self.home.clone();
        let memories = self.memories.clone();
        let store = Arc::clone(&self.store);
        let warming = Arc::clone(&self.warming);
        let corpus = Arc::clone(&self.corpus);
        tokio::task::spawn_blocking(move || {
            build_warm(&home, &memories, &store);
            // Build the corpus here, behind the request path: the first
            // click after the warm is the read, not the archive replay.
            let built = build_corpus(
                &home,
                &memories,
                &store,
                crate::memory::probed_embedder().as_ref(),
            );
            *lock(&corpus) = Some((Instant::now(), built));
            warming.store(false, Ordering::SeqCst);
        });
    }

    /// The endpoint's work: rank every other candidate against the
    /// selected object, by raw cosine when the embedder serves vectors
    /// and by word overlap when it does not.
    pub fn related(&self, kind: &str, id: &str, limit: usize) -> Value {
        let limit = limit.clamp(1, MAX_LIMIT);
        let embedder = crate::memory::probed_embedder();
        if !embedder.serves_vectors() {
            let candidates = self.corpus(embedder.as_ref());
            return payload(
                kind,
                id,
                &candidates,
                limit,
                "lexical",
                "warm",
                Some("keyword-only"),
            );
        }
        let warming = self.warming.load(Ordering::SeqCst);
        let candidates = self.corpus(embedder.as_ref());
        let index = if warming { "warming" } else { "warm" };
        // What decides the scoring mode is the SELECTED object's
        // vector, not the corpus's: a skill added after the warm has
        // none, and comparing it by cosine to nothing would answer an
        // empty list where a lexical pass still has something to say.
        let scores = if candidates
            .iter()
            .find(|item| item.kind == kind && item.id == id)
            .and_then(|item| item.vector.as_ref())
            .is_some()
        {
            "cosine"
        } else {
            "lexical"
        };
        payload(
            kind,
            id,
            &candidates,
            limit,
            scores,
            index,
            if scores == "lexical" {
                Some(if warming { "warming" } else { "keyword-only" })
            } else {
                None
            },
        )
    }

    /// The current corpus, built on demand and held for [`CORPUS_TTL`].
    /// A build is the only read of the sessions directory and the
    /// skill library on this path.
    fn corpus(&self, embedder: &dyn crate::memory::Embedder) -> Vec<Candidate> {
        {
            let cache = lock(&self.corpus);
            if let Some((at, candidates)) = cache.as_ref() {
                if at.elapsed() < CORPUS_TTL {
                    return candidates.clone();
                }
            }
        }
        let candidates = build_corpus(&self.home, &self.memories, &self.store, embedder);
        *lock(&self.corpus) = Some((Instant::now(), candidates.clone()));
        candidates
    }
}

/// The response, in the shape the browser reduces: the selected kind
/// and id, the scoring mode, the index's state, and the rows.
fn payload(
    kind: &str,
    id: &str,
    candidates: &[Candidate],
    limit: usize,
    scores: &str,
    index: &str,
    degraded: Option<&str>,
) -> Value {
    let mut scored: Vec<(f64, &Candidate)> = Vec::new();
    if scores == "cosine" {
        let selected = candidates
            .iter()
            .find(|item| item.kind == kind && item.id == id)
            .and_then(|item| item.vector.clone());
        if let Some(selected) = selected {
            for item in candidates {
                if item.kind == kind && item.id == id {
                    continue;
                }
                if let Some(vector) = &item.vector {
                    scored.push((crate::memory::cosine(&selected, vector), item));
                }
            }
        }
    } else {
        let query = candidates
            .iter()
            .find(|item| item.kind == kind && item.id == id)
            .map(|item| words(&item.text))
            .unwrap_or_default();
        for item in candidates {
            if item.kind == kind && item.id == id {
                continue;
            }
            let overlap = lexical(&query, &item.text);
            if overlap > 0.0 {
                scored.push((overlap, item));
            }
        }
    }
    scored.sort_by(|left, right| {
        right
            .0
            .partial_cmp(&left.0)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let related: Vec<Value> = scored
        .into_iter()
        .take(limit)
        .map(|(score, item)| {
            json!({
                "kind": item.kind,
                "id": item.id,
                "name": item.name,
                "summary": item.summary,
                "score": score,
                "weak": scores == "cosine" && score < WEAK_BELOW,
            })
        })
        .collect();
    let mut document = json!({
        "type": "related",
        "kind": kind,
        "id": id,
        "scores": scores,
        "index": index,
        // An empty list is not "everything here is weak": it is nothing
        // found, and the card says that separately.
        "weak": !related.is_empty()
            && related.iter().all(|row| row["weak"].as_bool() == Some(true)),
        "related": related,
    });
    if let Some(degraded) = degraded {
        document["degraded"] = json!(degraded);
    }
    document
}

/// Build the whole corpus once: the two caches' facts and sessions,
/// and the sidecar's cold kinds.
fn build_corpus(
    home: &Path,
    memories: &Path,
    store: &Mutex<Store>,
    embedder: &dyn crate::memory::Embedder,
) -> Vec<Candidate> {
    let mut candidates: Vec<Candidate> = Vec::new();

    // Facts: the recall cache is already warm, so this is a read of the
    // corpus and the index, not an embed. A fact with no vector this
    // load is left out of the ranking rather than scored zero.
    let memory = crate::memory::Memory::load_with(
        memories,
        embedder.box_clone(),
    );
    let today = crate::calib::today();
    for fact in memory.facts() {
        // A superseded or expired fact is not a relation: the panel
        // shows it under its own filter, and a suggestion is a live
        // object.
        if fact.forgotten(&today) {
            continue;
        }
        candidates.push(Candidate {
            kind: "fact",
            id: fact.name.clone(),
            name: fact.name.clone(),
            summary: fact.description.clone(),
            text: fact.indexed_text(),
            vector: memory.vector_of(&fact.name),
        });
    }

    // Sessions: the archive's own index, through the one body its
    // recall already runs. The facts are synthesized from the logs;
    // the vectors are cached by stamp.
    let sessions_dir = crate::home::sessions_dir(home);
    let session_facts = crate::session_recall::facts_from_sessions(home);
    // Cache-only, deliberately: embedding a cold archive is one call per
    // session, and the background warm does it. A click must not wait a
    // minute for a model swap to finish.
    let index = crate::session_recall::SessionIndex::load(&sessions_dir);
    let vectors = index.cached_vectors(&session_facts);
    for fact in session_facts {
        candidates.push(Candidate {
            kind: "session",
            id: fact.name.clone(),
            name: fact.name.clone(),
            summary: fact.description.clone(),
            text: fact.indexed_text(),
            vector: vectors.get(&fact.name).cloned(),
        });
    }

    // The day's routes and cells change without the process restarting,
    // so they are refreshed here rather than in the startup warm. Their
    // sources are one small ledger read and one calibration pass; a
    // changed digest re-embeds that row and nothing else.
    if let Some((name, dims)) = embedder.vector_info() {
        let routes = route_sources(home);
        let cells = cell_sources(home);
        let (routes_existing, cells_existing, reset) = {
            let mut store = lock(store);
            let reset = store.version != VERSION || store.embedder != name || store.dims != dims;
            if reset {
                *store = Store {
                    version: VERSION,
                    embedder: name.clone(),
                    dims,
                    ..Store::default()
                };
            }
            (store.routes.clone(), store.cells.clone(), reset)
        };
        let (routes_fresh, routes_changed) =
            refresh_section(&routes_existing, &routes, embedder);
        let (cells_fresh, cells_changed) = refresh_section(&cells_existing, &cells, embedder);
        if reset || routes_changed || cells_changed {
            let snapshot = {
                let mut store = lock(store);
                store.routes = routes_fresh;
                store.cells = cells_fresh;
                store.clone_for_write()
            };
            write_store(home, &snapshot);
        }
    }

    // The sidecar's kinds. The vectors are already there; the labels
    // rode along with them so this path reads no skill file and no git
    // repository.
    let store = lock(store);
    push_store(&mut candidates, "skill", &store.skills);
    push_store(&mut candidates, "project", &store.projects);
    push_store(&mut candidates, "route", &store.routes);
    push_store(&mut candidates, "cell", &store.cells);
    candidates
}

fn push_store(candidates: &mut Vec<Candidate>, kind: &'static str, section: &HashMap<String, Entry>) {
    for (id, entry) in section {
        candidates.push(Candidate {
            kind,
            id: id.clone(),
            name: entry.name.clone(),
            summary: entry.summary.clone(),
            text: entry.text.clone(),
            vector: Some(entry.vector.clone()),
        });
    }
}

/// The warm pass: the cold kinds only. Facts and sessions are already
/// cached by their own indices, so this is exactly the work that has
/// never been done: every skill description and every project's name,
/// branch, and subject.
fn build_warm(home: &Path, _memories: &Path, store: &Mutex<Store>) {
    let embedder = crate::memory::probed_embedder();
    if !embedder.serves_vectors() {
        return;
    }
    let Some(key) = embedder.vector_info() else {
        return;
    };
    // A sidecar written by another embedder is not this one's: discard
    // it whole, exactly as the two recall indices do.
    {
        let mut store = lock(store);
        if store.version != VERSION || store.embedder != key.0 || store.dims != key.1 {
            *store = Store {
                version: VERSION,
                embedder: key.0.clone(),
                dims: key.1,
                ..Store::default()
            };
        }
    }

    let skills = skill_entries(home);
    let projects = project_entries(home);
    // The embedding happens with no lock held: the whole point of the
    // warm is that a request can read what is already there while this
    // runs, and a lock across 400 embeds is a request path that waits.
    let (skills_existing, projects_existing) = {
        let store = lock(store);
        (store.skills.clone(), store.projects.clone())
    };
    let (skills_fresh, skills_changed) =
        refresh_section(&skills_existing, &skills, embedder.as_ref());
    let (projects_fresh, projects_changed) =
        refresh_section(&projects_existing, &projects, embedder.as_ref());
    let changed = {
        let mut store = lock(store);
        store.skills = skills_fresh;
        store.projects = projects_fresh;
        skills_changed || projects_changed
    };
    if changed {
        let snapshot = lock(store).clone_for_write();
        write_store(home, &snapshot);
    }

    // The archive's cold vectors too. They are not this sidecar's to
    // store — the sessions index owns them — but nothing else warms
    // them on a model swap, and the related corpus reads only what that
    // cache already holds.
    let sessions_dir = crate::home::sessions_dir(home);
    let session_facts = crate::session_recall::facts_from_sessions(home);
    let mut index = crate::session_recall::SessionIndex::load(&sessions_dir);
    index.vectors_for(&sessions_dir, embedder.as_ref(), &session_facts);
}

/// One kind's source rows: id, name, summary, text.
type Source = (String, String, String, String);

/// Update one sidecar section against its sources, embedding only the
/// rows whose digest changed. A failed embed drops the stale entry: an
/// old vector under a new digest would be served as this text's.
fn refresh_section(
    section: &HashMap<String, Entry>,
    sources: &[Source],
    embedder: &dyn crate::memory::Embedder,
) -> (HashMap<String, Entry>, bool) {
    let mut fresh: HashMap<String, Entry> = HashMap::new();
    let mut changed = false;
    for (id, name, summary, text) in sources {
        let digest = crate::calib::hash16(text);
        let cached = section
            .get(id)
            .filter(|entry| entry.digest == digest)
            .cloned();
        let entry = match cached {
            Some(entry) => Entry {
                name: name.clone(),
                summary: summary.clone(),
                text: text.clone(),
                ..entry
            },
            None => match embedder.embed(text) {
                Some(vector) => {
                    changed = true;
                    Entry {
                        digest,
                        vector,
                        name: name.clone(),
                        summary: summary.clone(),
                        text: text.clone(),
                    }
                }
                None => {
                    changed = true;
                    continue;
                }
            },
        };
        fresh.insert(id.clone(), entry);
    }
    if fresh.len() != section.len() {
        changed = true;
    }
    (fresh, changed)
}

/// Every loaded skill as a source row. The library read is the skills
/// panel's own; a missing canonical directory is no skills.
fn skill_entries(home: &Path) -> Vec<Source> {
    let override_dir = std::env::var("FLEET_SKILLS_DIR").ok();
    let user_home = std::env::var("HOME").ok();
    let canonical =
        crate::skills::resolve_skills_dir(override_dir.as_deref(), user_home.as_deref());
    let Some(snapshot) = canonical.map(|dir| crate::boards::read_skills(Some(&dir), home)) else {
        return Vec::new();
    };
    snapshot
        .skills
        .iter()
        .map(|skill| {
            let text = format!("{}\n{}", skill.name, skill.description);
            (
                skill.name.clone(),
                skill.name.clone(),
                skill.description.clone(),
                text,
            )
        })
        .collect()
}

/// Every project as a source row: name, branch, and the head subject —
/// the git facts without the paths or the counts, which say where a
/// project is, not what it is about.
fn project_entries(home: &Path) -> Vec<Source> {
    let passphrase = std::env::var("FLEET_VAULT_PASSPHRASE").ok();
    let dotenv = crate::home::read_dotenv_with_vault(home, passphrase.as_deref());
    let sources = crate::boards::resolve_sources(home, &dotenv);
    let Ok(snapshot) = crate::boards::read_tower(&sources.tower_root, &sources.bounds) else {
        return Vec::new();
    };
    snapshot
        .projects
        .iter()
        .map(|project| {
            let branch = project.branch.clone().unwrap_or_default();
            let subject = project.subject.clone().unwrap_or_default();
            let text = format!("{}\n{}\n{}", project.name, branch, subject);
            let summary = if subject.is_empty() {
                branch.clone()
            } else {
                subject.clone()
            };
            (project.name.clone(), project.name.clone(), summary, text)
        })
        .collect()
}

/// The route rows and fit cells also ride the sidecar, but they change
/// with the day and cost one small read, so they are refreshed inside
/// the corpus build rather than the startup warm. The caller passes
/// the rows the panels already read.
pub fn route_sources(home: &Path) -> Vec<Source> {
    let passphrase = std::env::var("FLEET_VAULT_PASSPHRASE").ok();
    let dotenv = crate::home::read_dotenv_with_vault(home, passphrase.as_deref());
    let sources = crate::boards::resolve_sources(home, &dotenv);
    let snapshot = crate::boards::read_router(&sources.router, &crate::calib::today());
    snapshot
        .recent
        .iter()
        .map(|row| {
            let text = format!(
                "{} {} {} {} {}",
                row.provider,
                row.model,
                row.workload.clone().unwrap_or_default(),
                row.outcome,
                row.reason.clone().unwrap_or_default()
            );
            let summary = format!(
                "{}{} \u{b7} {}",
                row.model,
                row
                    .workload
                    .as_ref()
                    .map(|workload| format!(" \u{b7} {workload}"))
                    .unwrap_or_default(),
                row.outcome
            );
            (row.at.clone(), row.model.clone(), summary, text)
        })
        .collect()
}

/// The fit cells for today, as source rows keyed by provider and
/// workload — the card's own identity.
pub fn cell_sources(home: &Path) -> Vec<Source> {
    let user_home = std::env::var("HOME").ok();
    let root = crate::calib::resolve_root(home, user_home.as_deref());
    let rows = crate::calib::read_day(&root, &crate::calib::today(), crate::calib::READ_CAP);
    crate::calib::calibration(&rows)
        .iter()
        .map(|cell| {
            let workload = cell.workload.clone();
            let id = format!("{}::{}", cell.provider, workload);
            let text = format!(
                "{} {} {} wrong {} insufficient {} judged {}",
                cell.provider,
                workload,
                cell.fit.label(),
                cell.weighted_wrong_rate,
                cell.weighted_insufficient_rate,
                cell.judged
            );
            let summary = format!("{} \u{b7} {}", cell.provider, cell.fit.label());
            (id, cell.provider.clone(), summary, text)
        })
        .collect()
}

/// Read the sidecar. Never errors: an absent, malformed, or foreign
/// version reads as empty and costs the warm a recompute.
fn read_store(home: &Path) -> Store {
    let Ok(text) = std::fs::read_to_string(home.join(INDEX_FILE)) else {
        return Store::default();
    };
    let Ok(store) = serde_json::from_str::<Store>(&text) else {
        return Store::default();
    };
    if store.version != VERSION {
        return Store::default();
    }
    store
}

impl Store {
    /// The clone a write takes, so the lock is not held across the file.
    fn clone_for_write(&self) -> Store {
        Store {
            version: self.version,
            embedder: self.embedder.clone(),
            dims: self.dims,
            skills: self.skills.clone(),
            projects: self.projects.clone(),
            routes: self.routes.clone(),
            cells: self.cells.clone(),
        }
    }
}

/// Write the sidecar through a unique temp and a rename, the recall
/// index's discipline: a reader never sees a half-written file and a
/// failed write costs the next warm a recompute, never an error.
fn write_store(home: &Path, store: &Store) {
    let Ok(text) = serde_json::to_string(store) else {
        return;
    };
    let path = home.join(INDEX_FILE);
    static SERIAL: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let serial = SERIAL.fetch_add(1, Ordering::Relaxed);
    let mut temp = path.clone().into_os_string();
    temp.push(format!(".{}.{}.tmp", std::process::id(), serial));
    let temp = PathBuf::from(temp);
    if std::fs::write(&temp, text).is_ok() {
        let _ = std::fs::rename(&temp, &path);
    }
}

/// The words a lexical fallback compares: lowercased, alphanumeric
/// runs of three or more, deduplicated. The same shape the browser's
/// old shortlist used, moved to the one place both states read.
fn words(text: &str) -> std::collections::HashSet<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|word| word.len() >= 3)
        .map(|word| word.to_lowercase())
        .collect()
}

/// How much of `query` the candidate answers, as a 0-1 share.
fn lexical(query: &std::collections::HashSet<String>, text: &str) -> f64 {
    if query.is_empty() {
        return 0.0;
    }
    let seen = words(text);
    let hits = query.iter().filter(|word| seen.contains(*word)).count();
    hits as f64 / query.len() as f64
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poison| poison.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_home() -> PathBuf {
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let serial = NEXT.fetch_add(1, Ordering::SeqCst);
        let home =
            std::env::temp_dir().join(format!("fleet-related-{}-{serial}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).expect("a temp home");
        home
    }

    #[test]
    fn the_payload_orders_by_score_and_names_weak_rows() {
        let candidates = vec![
            Candidate {
                kind: "fact",
                id: "a".into(),
                name: "a".into(),
                summary: "the selected fact".into(),
                text: "selected".into(),
                vector: Some(vec![1.0, 0.0]),
            },
            Candidate {
                kind: "skill",
                id: "close".into(),
                name: "close".into(),
                summary: "close".into(),
                text: "close".into(),
                vector: Some(vec![0.9, 0.1]),
            },
            Candidate {
                kind: "project",
                id: "far".into(),
                name: "far".into(),
                summary: "far".into(),
                text: "far".into(),
                vector: Some(vec![0.0, 1.0]),
            },
        ];
        let document = payload("fact", "a", &candidates, 3, "cosine", "warm", None);
        assert_eq!(document["related"][0]["id"], "close");
        assert_eq!(document["related"][0]["weak"], false);
        assert_eq!(document["related"][1]["id"], "far");
        assert_eq!(document["related"][1]["weak"], true);
        assert_eq!(document["weak"], false);
        // The selected row is never its own relation.
        assert!(document["related"]
            .as_array()
            .expect("rows")
            .iter()
            .all(|row| row["id"] != "a"));
    }

    #[test]
    fn a_lexical_fallback_orders_by_shared_words() {
        let candidates = vec![
            Candidate {
                kind: "fact",
                id: "a".into(),
                name: "a".into(),
                summary: "".into(),
                text: "the router walks the hard chain".into(),
                vector: None,
            },
            Candidate {
                kind: "skill",
                id: "router".into(),
                name: "router".into(),
                summary: "".into(),
                text: "router chain debugging".into(),
                vector: None,
            },
            Candidate {
                kind: "project".into(),
                id: "p".into(),
                name: "p".into(),
                summary: "".into(),
                text: "a different project".into(),
                vector: None,
            },
        ];
        let document = payload("fact", "a", &candidates, 3, "lexical", "warm", Some("keyword-only"));
        assert_eq!(document["scores"], "lexical");
        assert_eq!(document["degraded"], "keyword-only");
        assert_eq!(document["related"][0]["id"], "router");
    }

    #[test]
    fn a_foreign_or_malformed_sidecar_reads_as_empty() {
        let home = temp_home();
        std::fs::write(home.join(INDEX_FILE), "not json").expect("the bad file");
        assert!(read_store(&home).skills.is_empty());
        std::fs::write(
            home.join(INDEX_FILE),
            json!({"version": VERSION + 1, "skills": {"x": {}}}).to_string(),
        )
        .expect("the future file");
        assert_eq!(read_store(&home).version, 0);
    }

    #[test]
    fn a_write_lands_atomically_and_reads_back() {
        let home = temp_home();
        let mut store = Store {
            version: VERSION,
            embedder: "test".into(),
            dims: 2,
            ..Store::default()
        };
        store.skills.insert(
            "one".into(),
            Entry {
                digest: "d".into(),
                vector: vec![0.5, 0.5],
                name: "one".into(),
                summary: "first".into(),
                text: "one first".into(),
            },
        );
        write_store(&home, &store);
        let read = read_store(&home);
        assert_eq!(read.version, VERSION);
        assert_eq!(read.skills["one"].vector, vec![0.5, 0.5]);
        // No temp file is left beside it.
        let temps = std::fs::read_dir(&home)
            .expect("the home")
            .flatten()
            .filter(|entry| entry.file_name().to_string_lossy().ends_with(".tmp"))
            .count();
        assert_eq!(temps, 0);
    }
}
