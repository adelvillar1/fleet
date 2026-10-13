//! Memory: the git-tracked fact substrate, hybrid retrieval with a
//! probed vector path, Wave-D shape-routed recall, recall-hit feedback,
//! and the protected tier with budget-aware turn-context assembly.
//!
//! **The substrate.** One markdown file per durable fact under
//! `memories/`, frontmatter carrying `name`, `description`, `type`,
//! `date`, `basis`, `shape`, `recall-count`, `superseded-by`, and
//! `valid-until`, with the file's body as the fact's content. The fork's
//! `memories/` tree (234 facts in exactly this shape) is the spec, and
//! its `@fleet/dsh-brain` the design. Facts persist across sessions
//! because they are files; a bad one reverts cleanly because they are
//! git-tracked — no code path in this module rewrites history. The one
//! in-place rewrite is the recall-count bump, and it is bounded to that
//! one frontmatter line.
//!
//! **The hybrid ranker.** A fact's score is a weighted sum of four named
//! channels — vector similarity, keyword coverage, importance earned by
//! recall-count, and exponential temporal decay — with the weights named
//! constants below. The vector backend is **probed, not assumed**: the
//! loopback llama-server sem1 serves (`127.0.0.1:8901`) is discovered
//! with one real embedding round trip, and its absence degrades recall
//! to keyword-only — the correct degradation, named by `doctor` and by
//! this module's own surface, never silent. That is the fix for the live
//! system, where an absent embedding stack silently degraded recall over
//! 11k entries. `basis` deliberately does **not** enter the score: the
//! plan's rule is that importance is earned by use, not asserted at
//! write, and basis is provenance — the permission model part 2's
//! guardrail reads, not a rank.
//!
//! **Shape routing.** The incoming instruction classifies into one
//! structure shape from the fork's Wave D keyword taxonomy (rendering,
//! state-sync, deploy, build, change, deps, docs — keyword-backed, not
//! model-backed, because the model path proved unreliable), and a
//! same-shape fact takes a named boost. An unclassifiable instruction
//! answers from the surface alone — the documented fallback, not an
//! error.
//!
//! **The protected tier and assembly.** Facts whose `type` marks
//! identity or standing preferences (`user`, `feedback` in the fork's
//! taxonomy) are protected: the injection budget can never evict them.
//! Turn context is *assembled* — protected tier first, then shape-routed
//! recall, then recent working memory, within a named token budget —
//! rather than dumped whole. The observed failure this avoids: a turn
//! about one topic injected with unrelated facts.
//!
//! **Part 2's seams, named.** The write path (`brain_remember`-shaped),
//! the consolidation cycle, and the turn-path wiring that records recall
//! hits are part 2. This module leaves them: [`Memory::recall_hits`] is
//! the feedback entry point (it updates the file, re-ranks, and appends
//! the `recall_hit` calibration row), [`Memory::assemble`] takes the
//! working-memory tier as an input slice part 2 fills, and
//! `fleet memories --recall` is a dry run that writes nothing.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::{Duration, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::calib;
use crate::skills::wrap_detail;

// --- the substrate --------------------------------------------------------

/// The directory holding the fact tree, at the repo root.
pub const MEMORIES_DIR: &str = "memories";

/// The generated index under `memories/`, gitignored the way
/// `__pycache__` is: a cache beside the source it derives from. It maps
/// each fact to its stamp and its cached embedding vector.
pub const INDEX_FILE: &str = ".recall-index.json";

/// The types that mark identity or standing preferences — the fork's
/// taxonomy, where `user` is who the owner is and `feedback` is a
/// standing rule or preference he stated. Facts of these types are the
/// protected tier the injection budget can never evict.
pub const PROTECTED_TYPES: [&str; 2] = ["user", "feedback"];

/// How many ancestors above the working directory are searched for the
/// `memories/` directory. Covers any real clone depth.
const ANCESTOR_LIMIT: usize = 12;

/// Where a remembered fact came from: observed, or reasoned. Provenance
/// is the permission model part 2's guardrail reads — an `inferred` fact
/// is supersedable on the loop's own justification, a `confirmed-by` one
/// requires approval — and it is deliberately not a rank signal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Basis {
    ConfirmedBy,
    Inferred,
}

impl Basis {
    /// The word the frontmatter carries.
    pub fn label(self) -> &'static str {
        match self {
            Basis::ConfirmedBy => "confirmed-by",
            Basis::Inferred => "inferred",
        }
    }

    /// Parse the vocabulary, or `None` for anything else — a basis
    /// outside the two words is a malformed fact, not a default.
    fn parse(text: &str) -> Option<Basis> {
        match text.trim() {
            "confirmed-by" => Some(Basis::ConfirmedBy),
            "inferred" => Some(Basis::Inferred),
            _ => None,
        }
    }
}

/// One durable fact: its frontmatter, its body, and where it came from.
#[derive(Debug, Clone, PartialEq)]
pub struct Fact {
    pub name: String,
    /// The fact itself, as one sentence: what the index embeds and what
    /// an assembled turn carries. The body is the fact's full content.
    pub description: String,
    /// Free-form kind; the corpus uses `project`, `reference`,
    /// `feedback`, `user`.
    pub fact_type: String,
    /// ISO date, `YYYY-MM-DD`.
    pub date: String,
    pub basis: Basis,
    /// Structure tags from the fork's shape vocabulary.
    pub shape: Vec<String>,
    /// Times recall has surfaced this fact; the importance channel reads
    /// it and the recall-count bump writes it.
    pub recall_count: u64,
    /// Name of the fact this one is folded into; a value forgets this
    /// fact.
    pub superseded_by: String,
    /// ISO date after which the fact is stale; a past date forgets this
    /// fact.
    pub valid_until: String,
    /// The fact's full content: everything after the frontmatter fence.
    pub body: String,
    pub path: PathBuf,
    /// `secs:nanos:len` of the file at load, the index's freshness key.
    /// `None` when the metadata could not be read.
    pub stamp: Option<String>,
}

impl Fact {
    /// Whether this fact's type marks identity or a standing
    /// preference. Case-insensitive: a hand-written `User` protects the
    /// same as `user`.
    pub fn protected(&self) -> bool {
        let kind = self.fact_type.trim().to_ascii_lowercase();
        PROTECTED_TYPES.iter().any(|protected| *protected == kind)
    }

    /// Whether the fact's own markers retire it: a non-empty
    /// `superseded-by` folds it into another fact, and a `valid-until`
    /// before today expires it. A vector store does not decay, so the
    /// marker is the only thing that takes a fact out of the ranking —
    /// the fork's `isForgotten`, ported. An unreadable date is not an
    /// expiry.
    pub fn forgotten(&self, today: &str) -> bool {
        if !self.superseded_by.trim().is_empty() {
            return true;
        }
        matches!(day_age(&self.valid_until, parse_day(today)), Some(age) if age > 0)
    }

    /// Whether the fact's tags carry the structure shape.
    pub fn carries_shape(&self, shape: &str) -> bool {
        self.shape.iter().any(|tag| tag == shape)
    }

    /// The text the index embeds: the description with the shape tags
    /// riding along, the way the fork's importer indexes them.
    pub fn indexed_text(&self) -> String {
        if self.shape.is_empty() {
            self.description.clone()
        } else {
            format!("{} {}", self.description, self.shape.join(" "))
        }
    }
}

// --- the frontmatter subset ------------------------------------------------

/// One parsed fact document: its frontmatter fields, in order, and the
/// body after the closing fence.
///
/// This is a **sibling** of `skills`' parser, not a reuse: the two
/// subsets differ in three load-bearing ways. Memory accepts a bare
/// optional field (`superseded-by:` with no value — the fork's writer
/// emits exactly that for "unset"), accepts an indented continuation
/// line (the fork's writer folds a long plain scalar onto one), and
/// reads a bracketed `shape: [a, b]` list; the skills parser rejects
/// empty values and every indented line. Generalizing one parser to
/// both would cost the skills subset its deliberate rejections.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Document {
    fields: Vec<(String, String)>,
    body: String,
}

impl Document {
    fn get(&self, key: &str) -> Option<&str> {
        self.fields
            .iter()
            .find(|(name, _)| name == key)
            .map(|(_, value)| value.as_str())
    }

    fn required(&self, key: &str) -> Result<String, String> {
        self.get(key)
            .map(str::to_string)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| format!("frontmatter is missing a non-empty `{key}`"))
    }
}

/// Parse the frontmatter subset Fleet's facts accept: an opening `---`
/// fence, `key: value` lines with optional indented continuations, a
/// closing `---` fence, then the body.
///
/// Accepted, mirroring the skills parser where the subsets agree:
/// surrounding whitespace trimmed, keys and values optionally wrapped in
/// matching quotes, blank lines inside the fence, unknown keys (forward
/// compatibility), and a leading UTF-8 BOM. Accepted, where this subset
/// differs: a bare `key:` (an unset optional field), and an indented
/// line continuing the previous field's value with one joining space
/// (the fork's folded plain scalar). Rejected, each with the reason
/// named: no opening fence, no closing fence, a non-indented line that
/// is not a `key: value` pair (a `- item` list entry, a bare scalar), a
/// key whose quotes do not balance, a key that appears twice, an empty
/// key, and an indented line with no field to continue. The body is
/// everything after the closing fence with one leading newline taken
/// off.
fn parse_document(text: &str) -> Result<Document, String> {
    let (text, bom) = match text.strip_prefix('\u{feff}') {
        Some(rest) => (rest, true),
        None => (text, false),
    };
    parse_frontmatter(text).map_err(|reason| {
        if bom {
            format!("{reason} \u{2014} the file also opens with a UTF-8 BOM")
        } else {
            reason
        }
    })
}

/// The fence-and-fields parse itself, on text that carries no BOM.
fn parse_frontmatter(text: &str) -> Result<Document, String> {
    let lines: Vec<&str> = text.lines().collect();
    if lines.first().map(|line| line.trim_end()) != Some("---") {
        return Err("the file does not open with a `---` frontmatter fence".to_string());
    }
    let mut fields: Vec<(String, String)> = Vec::new();
    let mut closing: Option<usize> = None;
    for (index, line) in lines.iter().enumerate().skip(1) {
        if line.trim_end() == "---" {
            closing = Some(index);
            break;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        // An indented line continues the previous field's value: the
        // fork's writer folds a long plain scalar onto continuation
        // lines so the file stays readable, and a folded description
        // must read back as one sentence.
        if line.starts_with(' ') || line.starts_with('\t') {
            let Some((_, value)) = fields.last_mut() else {
                return Err(format!(
                    "frontmatter line {} is an indented continuation with no field to continue",
                    index + 1
                ));
            };
            value.push(' ');
            value.push_str(trimmed);
            continue;
        }
        // A list entry is YAML this parser does not speak: name it
        // rather than guessing.
        if trimmed.starts_with('-') {
            return Err(format!(
                "frontmatter line {} is not a `key: value` pair (nested YAML is not supported)",
                index + 1
            ));
        }
        let Some((key, value)) = trimmed.split_once(':') else {
            return Err(format!("frontmatter line {} is not a `key: value` pair", index + 1));
        };
        // Keys unquote the way values do: `"name": x` is `name: x`,
        // which is what a writer that quotes every key emits.
        let raw = key.trim();
        let key = unquote(raw);
        if key.is_empty() {
            return Err(format!("frontmatter line {} has an empty key", index + 1));
        }
        if key.contains('"') || key.contains('\'') {
            return Err(format!(
                "frontmatter line {} has an unbalanced quoted key `{raw}`",
                index + 1
            ));
        }
        let value = unquote(value.trim()).to_string();
        if fields.iter().any(|(seen, _)| seen == key) {
            return Err(format!("frontmatter key `{key}` appears twice"));
        }
        fields.push((key.to_string(), value));
    }
    let Some(closing) = closing else {
        return Err("the frontmatter fence never closes".to_string());
    };
    let body = lines[closing + 1..].join("\n");
    let body = body.strip_prefix('\n').unwrap_or(&body).to_string();
    Ok(Document { fields, body })
}

/// Strip one layer of matching surrounding quotes.
fn unquote(value: &str) -> &str {
    for quote in ['"', '\''] {
        if value.len() >= 2 && value.starts_with(quote) && value.ends_with(quote) {
            return &value[1..value.len() - 1];
        }
    }
    value
}

/// The `shape` field reads as a bracketed list — `[deploy, build]` — the
/// only form the fork's writer emits. A bare scalar reads as no tags,
/// the fork reader's own rule (its YAML parse yields a string, and its
/// `Array.isArray` check drops it); entries are trimmed, unquoted, and
/// lowercased, empties dropped. The three-tag cap is the writer's rule
/// (part 2); the reader reads what the file carries.
fn shape_tags(value: Option<&str>) -> Vec<String> {
    let Some(value) = value else { return Vec::new() };
    let Some(inner) = value.trim().strip_prefix('[').and_then(|v| v.strip_suffix(']')) else {
        return Vec::new();
    };
    inner
        .split(',')
        .map(|tag| unquote(tag.trim()).trim().to_ascii_lowercase())
        .filter(|tag| !tag.is_empty())
        .collect()
}

/// One log line naming the fact that was skipped and why. A broken fact
/// is never silent and never fatal — the skills precedent.
fn skip_line(name: &str, reason: &str) {
    eprintln!("fleet: memories: skipping `{name}` \u{2014} {reason}");
}

/// The memory rules on top of the shared parse: `name` and `description`
/// are required, the frontmatter's `name` must equal the file's stem
/// (one fact, one name — the name is the index and calibration key), a
/// `basis` outside the vocabulary is refused, a `recall-count` that is
/// not a non-negative integer is refused, and an absent `basis` defaults
/// to `inferred` — a fact the session merely produced is a guess until
/// someone confirms it.
fn fact(
    document: Document,
    stem: &str,
    path: PathBuf,
    stamp: Option<String>,
) -> Result<Fact, String> {
    let name = document.required("name")?;
    if name != stem {
        return Err(format!(
            "frontmatter name `{name}` does not match the file `{stem}.md`"
        ));
    }
    let description = document.required("description")?;
    let basis = match document.get("basis") {
        Some(text) => Basis::parse(text)
            .ok_or_else(|| format!("unknown basis `{text}` (expected confirmed-by or inferred)"))?,
        None => Basis::Inferred,
    };
    let recall_count = match document.get("recall-count") {
        Some(text) => text
            .trim()
            .parse::<u64>()
            .map_err(|_| format!("`recall-count: {text}` is not a non-negative integer"))?,
        None => 0,
    };
    Ok(Fact {
        name,
        description,
        fact_type: document.get("type").unwrap_or("").to_string(),
        date: document.get("date").unwrap_or("").to_string(),
        basis,
        shape: shape_tags(document.get("shape")),
        recall_count,
        superseded_by: document.get("superseded-by").unwrap_or("").to_string(),
        valid_until: document.get("valid-until").unwrap_or("").to_string(),
        body: document.body,
        path,
        stamp,
    })
}

// --- loading ---------------------------------------------------------------

/// Where `memories/` lives: the nearest ancestor of `cwd` that already
/// holds the directory, so a session opened in a subdirectory still
/// reads the clone's corpus; otherwise `cwd/memories`, where the first
/// write would create it (part 2). The fork's `resolveMemoryRoot`; the
/// configured override waits for a slice that needs it — Fleet's corpus
/// is the repo's own tree.
pub fn resolve_root(cwd: &Path) -> PathBuf {
    let mut dir = cwd.to_path_buf();
    for _ in 0..ANCESTOR_LIMIT {
        let candidate = dir.join(MEMORIES_DIR);
        if candidate.is_dir() {
            return candidate;
        }
        let Some(parent) = dir.parent() else { break };
        let parent = parent.to_path_buf();
        if parent == dir {
            break;
        }
        dir = parent;
    }
    cwd.join(MEMORIES_DIR)
}

/// `secs:nanos:len` of the file, the index's freshness key: a changed
/// fact is re-embedded, an untouched one keeps its cached vector.
fn file_stamp(path: &Path) -> Option<String> {
    let metadata = fs::metadata(path).ok()?;
    let modified = metadata.modified().ok()?;
    let duration = modified.duration_since(UNIX_EPOCH).ok()?;
    Some(format!("{}:{}:{}", duration.as_secs(), duration.subsec_nanos(), metadata.len()))
}

/// Load every fact under `root`, sorted by name. A missing directory is
/// no facts, not an error. A file that does not read as a fact is
/// skipped by name with one logged line; the others still load.
pub fn load_facts(root: &Path) -> Vec<Fact> {
    let Ok(entries) = fs::read_dir(root) else { return Vec::new() };
    let mut paths: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_file() && path.extension().and_then(|ext| ext.to_str()) == Some("md")
        })
        .collect();
    paths.sort();
    let mut facts: Vec<Fact> = Vec::new();
    for path in paths {
        // Owned, so the path itself stays free to move into the fact.
        let Some(stem) =
            path.file_stem().and_then(|stem| stem.to_str()).map(str::to_string)
        else {
            continue;
        };
        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(_) => {
                skip_line(&stem, "unreadable file");
                continue;
            }
        };
        let stamp = file_stamp(&path);
        match parse_document(&text).and_then(|document| fact(document, &stem, path.clone(), stamp))
        {
            Ok(fact) => facts.push(fact),
            Err(reason) => skip_line(&stem, &reason),
        }
    }
    facts.sort_by(|left, right| left.name.cmp(&right.name));
    facts
}

// --- the embedder ----------------------------------------------------------

/// The loopback embedding endpoint this machine's llama-server serves —
/// the same one sem1's default provider uses. Loopback by construction:
/// no new crate, no key, no internet.
pub const DEFAULT_EMBEDDING_ENDPOINT: &str = "http://127.0.0.1:8901/v1/embeddings";

/// The probe's round-trip budget. A live server answers a single embed
/// in milliseconds; a dead one refuses the connect instantly. The budget
/// is the ceiling for a black-holed port, not the expected cost.
const PROBE_TIMEOUT: Duration = Duration::from_secs(5);

/// A steady-state embed's budget, for the load-time index build.
const EMBED_TIMEOUT: Duration = Duration::from_secs(10);

/// The text the probe embeds. A real round trip, not a `/health` poll: a
/// health endpoint that answers ok proves nothing about the embeddings
/// route, and a route that cannot embed is exactly the silent
/// degradation this probe exists to name.
const PROBE_TEXT: &str = "fleet recall probe";

/// The vector backend, behind a trait so the keyword-only degradation is
/// a peer implementation rather than a stub: both serve recall, and
/// doctor names which one is live.
pub trait Embedder: Send + Sync {
    /// The embedding for `text`, or `None` when this backend cannot
    /// serve one — the keyword-only backend's honest answer.
    fn embed(&self, text: &str) -> Option<Vec<f32>>;

    /// The short name the surface's header carries.
    fn name(&self) -> &'static str;

    /// Whether this backend serves vectors at all. When it does not,
    /// cached vectors are ignored as well as new ones: keyword-only
    /// means keyword-only, so doctor's naming never contradicts what
    /// recall used.
    fn serves_vectors(&self) -> bool;

    /// The endpoint and dimensionality when this backend serves vectors
    /// — what the surface's header and doctor's named line report.
    /// `None` for the keyword-only degradation.
    fn vector_info(&self) -> Option<(String, usize)> {
        None
    }
}

/// The real local-model backend: a loopback HTTP call to the
/// llama-server embeddings route, with the crate's existing ureq client.
pub struct HttpEmbedder {
    endpoint: String,
    dims: usize,
    agent: ureq::Agent,
}

impl HttpEmbedder {
    /// One round trip against `endpoint`: a real embed of the probe
    /// text. `Some` when the answer carries a usable vector, with the
    /// dimensionality the probe found.
    pub fn probe(endpoint: &str) -> Option<HttpEmbedder> {
        let mut embedder = HttpEmbedder {
            endpoint: endpoint.to_string(),
            dims: 0,
            agent: ureq::AgentBuilder::new().timeout(PROBE_TIMEOUT).build(),
        };
        let dims = Embedder::embed(&embedder, PROBE_TEXT)?.len();
        embedder.dims = dims;
        // The probe proved the model answers within the probe's budget,
        // so the steady-state agent takes the more generous one: the
        // load-time index build embeds every fact, and a per-fact
        // timeout miss would silently cost that fact its vector.
        embedder.agent = ureq::AgentBuilder::new().timeout(EMBED_TIMEOUT).build();
        Some(embedder)
    }
}

impl Embedder for HttpEmbedder {
    fn embed(&self, text: &str) -> Option<Vec<f32>> {
        let body = serde_json::json!({ "input": text }).to_string();
        let response = self
            .agent
            .post(&self.endpoint)
            .set("Content-Type", "application/json")
            .send_string(&body)
            .ok()?;
        if response.status() != 200 {
            return None;
        }
        let text = response.into_string().ok()?;
        let value: serde_json::Value = serde_json::from_str(&text).ok()?;
        let embedding = value.get("data")?.get(0)?.get("embedding")?.as_array()?;
        let vector: Vec<f32> = embedding
            .iter()
            .map(|number| number.as_f64().map(|number| number as f32))
            .collect::<Option<Vec<_>>>()?;
        (!vector.is_empty()).then_some(vector)
    }

    fn name(&self) -> &'static str {
        "vector"
    }

    fn serves_vectors(&self) -> bool {
        true
    }

    fn vector_info(&self) -> Option<(String, usize)> {
        Some((self.endpoint.clone(), self.dims))
    }
}

/// The keyword-only degradation: the correct answer when no local model
/// is discoverable, not a stub. Hybrid ranking keeps its three other
/// channels and recall still answers.
pub struct KeywordOnly;

impl Embedder for KeywordOnly {
    fn embed(&self, _text: &str) -> Option<Vec<f32>> {
        None
    }

    fn name(&self) -> &'static str {
        "keyword-only"
    }

    fn serves_vectors(&self) -> bool {
        false
    }
}

/// What the vector-path probe found: the endpoint it probed, the
/// dimensionality when the path serves, and the embedder that carries
/// the answer into the index. The embedder is [`KeywordOnly`] whenever
/// `dims` is `None` — the named degradation, never a silent one.
pub struct Probe {
    pub endpoint: String,
    pub dims: Option<usize>,
    embedder: Box<dyn Embedder>,
}

impl Probe {
    /// The embedder that serves this machine.
    pub fn embedder(self) -> Box<dyn Embedder> {
        self.embedder
    }

    /// `(endpoint, dims)` when the vector path serves, for doctor's
    /// named line.
    pub fn vector(&self) -> Option<(&str, usize)> {
        self.dims.map(|dims| (self.endpoint.as_str(), dims))
    }
}

/// Probe `endpoint` for a live local embedding backend. Anything that
/// does not answer a usable vector — connection refused, non-200,
/// unreadable body, empty vector — is the keyword-only degradation.
pub fn probe(endpoint: &str) -> Probe {
    match HttpEmbedder::probe(endpoint) {
        Some(http) => {
            let (endpoint, dims) = (http.endpoint.clone(), http.dims);
            Probe { endpoint, dims: Some(dims), embedder: Box::new(http) }
        }
        None => Probe {
            endpoint: endpoint.to_string(),
            dims: None,
            embedder: Box::new(KeywordOnly),
        },
    }
}

// --- the index -------------------------------------------------------------

/// The generated cache under `memories/`: one entry per fact, keyed by
/// name, carrying the stamp the vector was computed at. A cache never
/// errors — an absent or malformed index reads as empty and costs a
/// recompute.
#[derive(Debug, Default, Serialize, Deserialize)]
struct Index {
    #[serde(default)]
    facts: HashMap<String, IndexEntry>,
}

/// One cached vector with the file stamp it was computed at.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct IndexEntry {
    stamp: String,
    vector: Vec<f32>,
}

/// `<root>/.recall-index.json`.
fn index_path(root: &Path) -> PathBuf {
    root.join(INDEX_FILE)
}

impl Index {
    fn read(root: &Path) -> Index {
        fs::read_to_string(index_path(root))
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    /// Write the cache through a unique temp file and a rename, so a
    /// reader never sees a half-written index and two fleet processes
    /// cannot collide on one temp. Best-effort and silent by design: a
    /// failed write costs the next load a recompute and nothing else —
    /// the index is `__pycache__`, and `__pycache__` never complains.
    fn write(root: &Path, entries: &HashMap<String, IndexEntry>) {
        let Ok(text) = serde_json::to_string(&Index { facts: entries.clone() }) else {
            return;
        };
        let path = index_path(root);
        let serial = INDEX_WRITE_SERIAL.fetch_add(1, Ordering::Relaxed);
        let mut temp = path.clone().into_os_string();
        temp.push(format!(".{}.{}.tmp", std::process::id(), serial));
        let temp = PathBuf::from(temp);
        if fs::write(&temp, text).is_ok() {
            let _ = fs::rename(&temp, &path);
        }
    }
}

/// A per-write serial for the index temp name, so two fleet processes —
/// or two writes in one process — never share a temp file.
static INDEX_WRITE_SERIAL: AtomicUsize = AtomicUsize::new(0);

// --- the ranker ------------------------------------------------------------

/// The vector channel's weight. The dominant channel when the vector
/// path serves; the keyword-only backend contributes nothing here, and
/// the other three channels carry the ranking.
pub const W_VECTOR: f64 = 0.55;

/// The keyword channel's weight: how much of the instruction the fact's
/// indexed text answers.
pub const W_KEYWORD: f64 = 0.25;

/// The importance channel's weight: recall-count, earned by use.
pub const W_IMPORTANCE: f64 = 0.10;

/// The temporal-decay channel's weight: exponential freshness.
pub const W_DECAY: f64 = 0.10;

/// Recalls at which the importance bonus reaches its ceiling — the
/// fork's `recallWindow`, ported.
pub const RECALL_CEILING: u64 = 8;

/// The decay half-life in days: a fact's freshness halves every 90
/// days. Exponential rather than the fork's linear window, so an old
/// fact decays smoothly instead of falling off a cliff.
pub const DECAY_HALFLIFE_DAYS: f64 = 90.0;

/// The shape-routing boost: a fact carrying the instruction's
/// classified shape takes this onto its score. Routing, not ranking —
/// it rides beside the four hybrid channels.
pub const SHAPE_BOOST: f64 = 0.15;

/// The floor a fact must clear to enter a turn for this instruction.
/// Below it the fact is noise: an unrelated fact's best case is its
/// decay plus a sliver of importance, and a turn about one topic must
/// not be injected with facts about another. The floor sits just above
/// the decay channel alone, so a fact enters when it matches something
/// — forty percent of the instruction's tokens, or a vector hit, or a
/// long recall history — and not merely for being fresh. The live
/// embedding model's cosines do not separate relevance on their own
/// (KI-058), so the floor is on the composite score; a heavily-recalled
/// fact that matches nothing still clears it, ranks last, and is the
/// budget's first drop.
pub const MIN_RECALL_SCORE: f64 = 0.20;

/// One fact's score: the total the ranking orders by, and the channels
/// that produced it — the "why" the surface prints.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Score {
    pub total: f64,
    pub vector: f64,
    pub keyword: f64,
    pub importance: f64,
    pub decay: f64,
    /// The shape-routing boost, `0.0` when unrouted or untagged.
    pub shape_boost: f64,
}

impl Score {
    /// Compose the four hybrid channels and the routing boost.
    fn compute(
        vector: f64,
        keyword: f64,
        recall_count: u64,
        date: &str,
        today_days: Option<i64>,
        shape_hit: bool,
    ) -> Score {
        let importance = recall_count.min(RECALL_CEILING) as f64 / RECALL_CEILING as f64;
        let decay = decay_factor(date, today_days);
        let shape_boost = if shape_hit { SHAPE_BOOST } else { 0.0 };
        let total = W_VECTOR * vector
            + W_KEYWORD * keyword
            + W_IMPORTANCE * importance
            + W_DECAY * decay
            + shape_boost;
        Score { total, vector, keyword, importance, decay, shape_boost }
    }
}

/// Cosine similarity, in full f64 even though the vectors are f32. A
/// length mismatch or a zero vector scores 0.0 — not similar, not an
/// error.
fn cosine(left: &[f32], right: &[f32]) -> f64 {
    if left.len() != right.len() || left.is_empty() {
        return 0.0;
    }
    let mut dot = 0.0;
    let mut left_norm = 0.0;
    let mut right_norm = 0.0;
    for (a, b) in left.iter().zip(right) {
        let (a, b) = (f64::from(*a), f64::from(*b));
        dot += a * b;
        left_norm += a * a;
        right_norm += b * b;
    }
    if left_norm == 0.0 || right_norm == 0.0 {
        return 0.0;
    }
    dot / (left_norm.sqrt() * right_norm.sqrt())
}

/// The normalized token set of `text`: lowercased, split on every
/// non-alphanumeric run, deduplicated — the skills module's tokenizer,
/// so the two surfaces measure words the same way.
fn tokens(text: &str) -> Vec<String> {
    let mut tokens: Vec<String> = Vec::new();
    for token in text
        .to_lowercase()
        .split(|character: char| !character.is_alphanumeric())
        .filter(|token| !token.is_empty())
    {
        if !tokens.iter().any(|seen| seen == token) {
            tokens.push(token.to_string());
        }
    }
    tokens
}

/// How much of the instruction the fact answers: the instruction's
/// tokens the fact's indexed text carries, over the instruction's
/// tokens. An instruction with no tokens covers nothing.
fn keyword_coverage(query: &[String], fact: &[String]) -> f64 {
    if query.is_empty() {
        return 0.0;
    }
    let covered = query.iter().filter(|token| fact.iter().any(|seen| seen == *token)).count();
    covered as f64 / query.len() as f64
}

/// Exponential temporal decay with a named half-life: a fact's freshness
/// halves every [`DECAY_HALFLIFE_DAYS`]. A fact with no readable date —
/// or a machine with no readable clock — contributes no decay term, the
/// fork reader's own rule for an unreadable date.
fn decay_factor(date: &str, today_days: Option<i64>) -> f64 {
    let Some(today) = today_days else { return 0.0 };
    let Some(then) = parse_day(date) else { return 0.0 };
    let age = (today - then).max(0) as f64;
    0.5_f64.powf(age / DECAY_HALFLIFE_DAYS)
}

// --- dates -----------------------------------------------------------------

/// Days since the Unix epoch for a `YYYY-MM-DD` day, read at UTC so an
/// age is a whole number of days regardless of the machine's zone.
/// Howard Hinnant's `days_from_civil`, hand-rolled because the crate
/// carries no date dependency and needs exactly this one primitive.
fn parse_day(day: &str) -> Option<i64> {
    let year: i64 = day.get(0..4)?.parse().ok()?;
    let month: i64 = day.get(5..7)?.parse().ok()?;
    let day_of_month: i64 = day.get(8..10)?.parse().ok()?;
    if !(1..=12).contains(&month) || !(1..=31).contains(&day_of_month) {
        return None;
    }
    let year = if month <= 2 { year - 1 } else { year };
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let year_of_era = year - era * 400;
    let day_of_year =
        (153 * (if month > 2 { month - 3 } else { month + 9 }) + 2) / 5 + day_of_month - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    Some(era * 146097 + day_of_era - 719468)
}

/// Whole days from `date` to `today_days`, or `None` when either is
/// unreadable.
fn day_age(date: &str, today_days: Option<i64>) -> Option<i64> {
    Some(today_days? - parse_day(date)?)
}

// --- shape routing ---------------------------------------------------------

/// The Wave D keyword taxonomy, ported verbatim from the fork's
/// `memories/shapes.json` (mirrored in `dsh-brain`'s
/// `DEFAULT_SHAPE_KEYWORDS`): structure shape → the lowercase terms that
/// name it. Keyword order decides — the same order the memory importer
/// tags in, so the shape this picks is the first structural tag the
/// importer would put on the same text.
pub const SHAPE_KEYWORDS: [(&str, &[&str]); 7] = [
    (
        "deploy",
        &[
            "deploy",
            "deployment",
            "railway",
            "vercel",
            "netlify",
            "rollout",
            "release",
            "production is live",
            "staging build",
        ],
    ),
    (
        "build",
        &[
            "build",
            "compile",
            "tsc",
            "bundle",
            "webpack",
            "vite",
            "esbuild",
            "build fails",
            "build error",
        ],
    ),
    (
        "deps",
        &[
            "dependency",
            "dependencies",
            "package.json",
            "pnpm",
            "npm",
            "version bump",
            "upgrade",
            "lockfile",
        ],
    ),
    (
        "docs",
        &[
            "documentation",
            "docs drift",
            "readme",
            "doc-sync",
            "translation pairing",
            "word budget",
        ],
    ),
    (
        "rendering",
        &[
            "render",
            "rendering",
            "canvas",
            "webgl",
            "shader",
            "layout",
            "css",
            "paint",
            "frame rate",
        ],
    ),
    (
        "state-sync",
        &[
            "state sync",
            "desync",
            "websocket",
            "ws ",
            "reconnect",
            "optimistic update",
            "cache invalidation",
            "stale state",
        ],
    ),
    (
        "change",
        &["refactor", "rename", "migration", "rewrite", "revert", "breaking change"],
    ),
];

/// The taxonomy in its own order, for the surface's naming.
pub const SHAPES: [&str; 7] = ["rendering", "state-sync", "deploy", "build", "change", "deps", "docs"];

/// Classify one instruction into a structure shape by its keywords, or
/// `None` when no shape's terms appear. Substring match on the
/// lowercased text, first shape in [`SHAPE_KEYWORDS`] order wins — the
/// fork's `classifyShape`, ported.
pub fn classify_shape(text: &str) -> Option<&'static str> {
    let haystack = text.to_lowercase();
    SHAPE_KEYWORDS
        .iter()
        .find(|(_, terms)| terms.iter().any(|term| haystack.contains(term)))
        .map(|(shape, _)| *shape)
}

// --- the memory ------------------------------------------------------------

/// The loaded corpus: the facts, the probed embedder, and the cached
/// vectors keyed by fact name.
pub struct Memory {
    root: PathBuf,
    embedder: Box<dyn Embedder>,
    facts: Mutex<Vec<Fact>>,
    vectors: HashMap<String, Vec<f32>>,
}

/// The mutex, recovered from poison: the only critical sections are
/// reads and a bounded count update, neither of which can panic, so a
/// poisoned lock means some unrelated panic tripped the flag and the
/// corpus is still sound — the calibration writer's own rule.
fn lock_facts(facts: &Mutex<Vec<Fact>>) -> std::sync::MutexGuard<'_, Vec<Fact>> {
    facts.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

impl Memory {
    /// Load the corpus under `root` with the vector path probed: the
    /// real local-model backend when it answers, keyword-only otherwise.
    pub fn load(root: &Path) -> Memory {
        Memory::load_with(root, probe(DEFAULT_EMBEDDING_ENDPOINT).embedder())
    }

    /// Load with an explicit embedder — the seam the tests and a future
    /// configured endpoint use. The index is built here: a fact whose
    /// stamp matches its cached entry keeps its vector, every other
    /// fact is embedded once, and the cache is rewritten only when
    /// something changed.
    pub fn load_with(root: &Path, embedder: Box<dyn Embedder>) -> Memory {
        let facts = load_facts(root);
        let mut vectors: HashMap<String, Vec<f32>> = HashMap::new();
        if embedder.serves_vectors() {
            let index = Index::read(root);
            let mut fresh: HashMap<String, IndexEntry> = HashMap::new();
            let mut changed = false;
            for fact in &facts {
                let cached = fact.stamp.as_ref().and_then(|stamp| {
                    index
                        .facts
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
                        None => continue,
                    },
                };
                if let Some(stamp) = &fact.stamp {
                    fresh.insert(
                        fact.name.clone(),
                        IndexEntry { stamp: stamp.clone(), vector: vector.clone() },
                    );
                }
                vectors.insert(fact.name.clone(), vector);
            }
            if changed || fresh.len() != index.facts.len() {
                Index::write(root, &fresh);
            }
        }
        Memory { root: root.to_path_buf(), embedder, facts: Mutex::new(facts), vectors }
    }

    /// The corpus root, `memories/`.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The backend's short name: `vector` or `keyword-only`.
    pub fn backend(&self) -> &'static str {
        self.embedder.name()
    }

    /// The backend as the surface's header names it: the short name, and
    /// the endpoint and dimensionality when the vector path serves.
    /// Never silent — a keyword-only corpus says so where an operator
    /// looks.
    pub fn backend_label(&self) -> String {
        match self.embedder.vector_info() {
            Some((_, dims)) => format!("vector ({dims}-dim)"),
            None => "keyword-only".to_string(),
        }
    }

    /// Every loaded fact, protected and retired included, sorted by
    /// name — the surface's view.
    pub fn facts(&self) -> Vec<Fact> {
        lock_facts(&self.facts).clone()
    }

    /// The protected tier: every non-forgotten protected fact, sorted by
    /// name. These enter a turn unconditionally — the budget never
    /// evicts them.
    pub fn protected(&self) -> Vec<ScoredFact> {
        let today = calib::today();
        let today_days = parse_day(&today);
        let facts = lock_facts(&self.facts);
        let mut hits: Vec<ScoredFact> = facts
            .iter()
            .filter(|fact| fact.protected() && !fact.forgotten(&today))
            .map(|fact| ScoredFact {
                fact: fact.clone(),
                // The protected tier is admitted unconditionally, so
                // its score is the fact's own standing — importance and
                // decay, with no query to match against.
                score: Score::compute(0.0, 0.0, fact.recall_count, &fact.date, today_days, false),
            })
            .collect();
        hits.sort_by(|left, right| left.fact.name.cmp(&right.fact.name));
        hits
    }

    /// Ranked recall for one instruction: the hybrid score with the
    /// shape-routing boost, protected facts excluded (they are assembled
    /// through [`Memory::protected`] — no fact enters a turn twice),
    /// forgotten facts excluded, and everything below
    /// [`MIN_RECALL_SCORE`] dropped as noise for this instruction.
    /// Strongest first, name as the deterministic tie-break.
    pub fn recall(&self, instruction: &str) -> Vec<ScoredFact> {
        let shape = classify_shape(instruction);
        let query_vector = self.embedder.embed(instruction);
        let query_tokens = tokens(instruction);
        let today = calib::today();
        let today_days = parse_day(&today);
        let facts = lock_facts(&self.facts);
        let mut hits: Vec<ScoredFact> = facts
            .iter()
            .filter(|fact| !fact.protected() && !fact.forgotten(&today))
            .map(|fact| ScoredFact {
                fact: fact.clone(),
                score: self.score(fact, &query_vector, &query_tokens, shape, today_days),
            })
            .filter(|hit| hit.score.total >= MIN_RECALL_SCORE)
            .collect();
        hits.sort_by(|left, right| {
            right
                .score
                .total
                .total_cmp(&left.score.total)
                .then_with(|| left.fact.name.cmp(&right.fact.name))
        });
        hits
    }

    /// The feedback entry point — part 2's turn path calls this where
    /// part 1 has no caller. Every hit updates the fact's `recall-count`
    /// in its file (a bounded, frontmatter-preserving rewrite), re-ranks
    /// on the next recall, and appends one `recall_hit` calibration row
    /// naming the fact, the shape, and the score. The rewrite is
    /// best-effort: a file that cannot be rewritten logs one line and
    /// the hit is still recorded — the audit trail must not depend on
    /// the corpus being writable.
    pub fn recall_hits(&self, instruction: &str, stream: Option<&calib::Stream>) -> Vec<ScoredFact> {
        let hits = self.recall(instruction);
        let shape = classify_shape(instruction).unwrap_or_default().to_string();
        for hit in &hits {
            self.bump_recall_count(&hit.fact.name);
            if let Some(stream) = stream {
                stream.record(calib::Row::RecallHit(calib::RecallHitRow {
                    ts: calib::stamp(),
                    fact: hit.fact.name.clone(),
                    shape: shape.clone(),
                    score: hit.score.total,
                }));
            }
        }
        hits
    }

    /// One fact's score against a query. The vector channel needs both
    /// vectors; the keyword channel needs the tokens; the rest is the
    /// fact's own.
    fn score(
        &self,
        fact: &Fact,
        query_vector: &Option<Vec<f32>>,
        query_tokens: &[String],
        shape: Option<&str>,
        today_days: Option<i64>,
    ) -> Score {
        let vector = match (query_vector, self.vectors.get(&fact.name)) {
            (Some(query), Some(fact_vector)) => cosine(query, fact_vector).max(0.0),
            _ => 0.0,
        };
        let keyword = keyword_coverage(query_tokens, &tokens(&fact.indexed_text()));
        let shape_hit = matches!(shape, Some(shape) if fact.carries_shape(shape));
        Score::compute(vector, keyword, fact.recall_count, &fact.date, today_days, shape_hit)
    }

    /// Add one to the fact's `recall-count`: in memory first (so the
    /// next recall in this process re-ranks), then in its file. Returns
    /// whether the file rewrite landed.
    fn bump_recall_count(&self, name: &str) -> bool {
        let (path, count) = {
            let mut facts = lock_facts(&self.facts);
            let Some(fact) = facts.iter_mut().find(|fact| fact.name == name) else {
                return false;
            };
            fact.recall_count += 1;
            (fact.path.clone(), fact.recall_count)
        };
        rewrite_recall_count(&path, count)
    }
}

/// One fact ranked for one instruction: the fact and the score that
/// ordered it.
#[derive(Debug, Clone, PartialEq)]
pub struct ScoredFact {
    pub fact: Fact,
    pub score: Score,
}

/// The bounded, frontmatter-preserving recall-count rewrite: the one
/// line the rewrite owns is re-rendered, and every other byte — the
/// body above all — is preserved. The write lands through a unique temp
/// file and a rename, so a reader never sees a half-written fact.
fn rewrite_recall_count(path: &Path, count: u64) -> bool {
    let Ok(text) = fs::read_to_string(path) else {
        eprintln!(
            "fleet: memories: could not re-read {} to bump its recall-count",
            path.display()
        );
        return false;
    };
    let lines: Vec<&str> = text.lines().collect();
    let mut closing: Option<usize> = None;
    for (index, line) in lines.iter().enumerate().skip(1) {
        if line.trim_end() == "---" {
            closing = Some(index);
            break;
        }
    }
    let Some(closing) = closing else {
        eprintln!(
            "fleet: memories: {} lost its frontmatter fence; the recall-count was not bumped",
            path.display()
        );
        return false;
    };
    let mut out_lines: Vec<String> = lines.iter().map(|line| line.to_string()).collect();
    let mut written = false;
    for line in out_lines.iter_mut().take(closing).skip(1) {
        let key = line.split_once(':').map(|(key, _)| key.trim()).unwrap_or_default();
        if key == "recall-count" {
            *line = format!("recall-count: {count}");
            written = true;
            break;
        }
    }
    if !written {
        // A fact the sleep cycle never touched carries no recall-count
        // line; insert one as the last frontmatter line rather than
        // appending anywhere else.
        out_lines.insert(closing, format!("recall-count: {count}"));
    }
    let mut out = out_lines.join("\n");
    if text.ends_with('\n') {
        out.push('\n');
    }
    let serial = REWRITE_SERIAL.fetch_add(1, Ordering::Relaxed);
    let mut temp = path.to_path_buf().into_os_string();
    temp.push(format!(".{}.{}.tmp", std::process::id(), serial));
    let temp = PathBuf::from(temp);
    match fs::write(&temp, &out) {
        Ok(()) => {
            if fs::rename(&temp, path).is_ok() {
                true
            } else {
                let _ = fs::remove_file(&temp);
                eprintln!(
                    "fleet: memories: could not replace {} with its bumped recall-count",
                    path.display()
                );
                false
            }
        }
        Err(error) => {
            eprintln!("fleet: memories: could not write {}: {error}", temp.display());
            false
        }
    }
}

/// A per-write serial for the rewrite's temp name, so two bumps in one
/// process never share a temp file.
static REWRITE_SERIAL: AtomicUsize = AtomicUsize::new(0);

// --- assembly --------------------------------------------------------------

/// The turn-context budget in tokens, estimated at one token per four
/// characters — the fork's own estimate and budget, ported. It bounds
/// what enters a turn; it can never evict the protected tier.
pub const ASSEMBLY_TOKEN_BUDGET: usize = 512;

/// One turn's assembled context: the protected tier (never evicted),
/// the shape-routed recall that fit the budget, the working memory that
/// fit after it, and what the budget dropped — named, so the surface
/// can be honest about it.
#[derive(Debug, Clone, PartialEq)]
pub struct Assembly {
    /// The classified shape, or `None` for the surface-alone fallback.
    pub shape: Option<&'static str>,
    /// Every fact above the floor, strongest first — the full ranking,
    /// before the budget applies.
    pub ranked: Vec<ScoredFact>,
    /// The protected tier, name-sorted.
    pub protected: Vec<ScoredFact>,
    /// The ranked facts that entered, in rank order.
    pub recalled: Vec<ScoredFact>,
    /// The working memory that entered, in the order given.
    pub working: Vec<Fact>,
    /// Names the budget dropped, ranked facts first.
    pub dropped: Vec<String>,
    /// Tokens the assembly uses, against [`Assembly::budget`].
    pub used: usize,
    /// The budget this assembly was built within.
    pub budget: usize,
}

impl Memory {
    /// Assemble one turn's context within [`ASSEMBLY_TOKEN_BUDGET`]:
    /// protected tier first, then shape-routed recall, then the caller's
    /// working memory. `working` is part 2's seam — the write path
    /// fills the session-scoped tier; part 1's callers pass none.
    pub fn assemble(&self, instruction: &str, working: &[Fact]) -> Assembly {
        self.assemble_within(instruction, working, ASSEMBLY_TOKEN_BUDGET)
    }

    /// Assemble within an explicit budget — the seam the budget-pressure
    /// test uses; production calls [`Memory::assemble`].
    ///
    /// The protected tier is added first and is never dropped, even when
    /// it alone exceeds the budget — that is the tier's whole point, the
    /// fix for the live system's mis-tiered cap where the most
    /// load-bearing facts contended with task facts in the smallest
    /// space. The strongest recalled fact always enters (the fork's
    /// `renderBlock` rule: a budget too small for it still carries the
    /// strongest match rather than an empty block); the rest enter while
    /// they fit, and the ones that do not are named in `dropped`.
    /// Working memory is strictly budget-bounded.
    pub fn assemble_within(
        &self,
        instruction: &str,
        working: &[Fact],
        budget: usize,
    ) -> Assembly {
        let shape = classify_shape(instruction);
        let protected = self.protected();
        let ranked = self.recall(instruction);
        let today = calib::today();

        let mut used = 0;
        for hit in &protected {
            used += estimate_tokens(protected_line(hit).chars().count());
        }
        let mut recalled: Vec<ScoredFact> = Vec::new();
        let mut dropped: Vec<String> = Vec::new();
        for (index, hit) in ranked.iter().enumerate() {
            let cost = estimate_tokens(recalled_line(hit).chars().count());
            if index == 0 || used + cost <= budget {
                used += cost;
                recalled.push(hit.clone());
            } else {
                dropped.push(hit.fact.name.clone());
            }
        }
        let mut kept_working: Vec<Fact> = Vec::new();
        for fact in working {
            if fact.forgotten(&today) {
                continue;
            }
            let cost = estimate_tokens(working_line(fact).chars().count());
            if used + cost <= budget {
                used += cost;
                kept_working.push(fact.clone());
            } else {
                dropped.push(fact.name.clone());
            }
        }
        Assembly {
            shape,
            ranked,
            protected,
            recalled,
            working: kept_working,
            dropped,
            used,
            budget,
        }
    }
}

/// One token per four characters, rounded up — the fork's estimate.
fn estimate_tokens(chars: usize) -> usize {
    (chars + 3) / 4
}

/// One protected fact as the assembled context carries it: its type and
/// its basis, no score — a protected fact did not compete for its place.
fn protected_line(hit: &ScoredFact) -> String {
    format!(
        "- [{} {}] {}",
        hit.fact.fact_type,
        hit.fact.basis.label(),
        hit.fact.description
    )
}

/// One recalled fact as the assembled context carries it: its first
/// shape tag, the score it ranked at, and its basis.
fn recalled_line(hit: &ScoredFact) -> String {
    let tag = hit.fact.shape.first().map(String::as_str).unwrap_or("\u{b7}");
    format!(
        "- [{tag} {:.2} {}] {}",
        hit.score.total,
        hit.fact.basis.label(),
        hit.fact.description
    )
}

/// One working-memory fact as the assembled context carries it.
fn working_line(fact: &Fact) -> String {
    format!("- [working {}] {}", fact.basis.label(), fact.description)
}

/// Render an assembly as the block that enters a turn's context: the
/// protected tier first, then the recall section — named for its shape,
/// or named as the surface-alone fallback when nothing classified —
/// then the working tier. Sections with nothing in them render nothing.
pub fn render_assembly(assembly: &Assembly) -> String {
    let mut out = String::new();
    if !assembly.protected.is_empty() {
        out.push_str(
            "Standing context (protected \u{2014} identity and standing preferences; the budget never evicts these):\n",
        );
        for hit in &assembly.protected {
            out.push_str(&protected_line(hit));
            out.push('\n');
        }
    }
    if !assembly.recalled.is_empty() {
        match assembly.shape {
            Some(shape) => out.push_str(&format!(
                "Relevant memory (shape-routed recall for `{shape}` \u{2014} propose only, verify before relying):\n"
            )),
            None => out.push_str(
                "Relevant memory (surface recall \u{2014} no shape matched, the documented fallback \u{2014} propose only, verify before relying):\n",
            ),
        }
        for hit in &assembly.recalled {
            out.push_str(&recalled_line(hit));
            out.push('\n');
        }
    }
    if !assembly.working.is_empty() {
        out.push_str("Working memory (session-scoped):\n");
        for fact in &assembly.working {
            out.push_str(&working_line(fact));
            out.push('\n');
        }
    }
    out
}

// --- the surface -----------------------------------------------------------

mod palette {
    pub const ACCENT: &str = "\u{1b}[36m";
    pub const DIM: &str = "\u{1b}[90m";
    pub const TEXT: &str = "\u{1b}[37m";
    pub const BOLD: &str = "\u{1b}[1m";
    pub const RESET: &str = "\u{1b}[0m";
}

use palette::{ACCENT, BOLD, DIM, RESET, TEXT};

/// The card width, matching `skills`, `calib`, `lanes`, and `providers`.
const CARD_WIDTH: usize = 72;

/// The English plural for a count, the way the card family carries it.
fn plural(count: usize) -> &'static str {
    if count == 1 {
        ""
    } else {
        "s"
    }
}

/// Render the corpus as cards: one per live fact with its name, type,
/// shape, basis, recall-count, and valid-until, in the house card style.
/// Retired facts are counted in the footer rather than carded — a
/// superseded or expired fact is not part of the working corpus.
pub fn render(memory: &Memory, shape: Option<&str>) -> String {
    let facts = memory.facts();
    let today = calib::today();
    let live: Vec<&Fact> = facts.iter().filter(|fact| !fact.forgotten(&today)).collect();
    let retired = facts.len() - live.len();
    let protected = live.iter().filter(|fact| fact.protected()).count();
    let shown: Vec<&Fact> = match shape {
        Some(shape) => live.iter().copied().filter(|fact| fact.carries_shape(shape)).collect(),
        None => live.clone(),
    };
    let mut out = String::new();
    out.push_str(&format!(
        "{BOLD}fleet memories{RESET} {DIM}\u{2014} {} fact{} from {}/ \u{b7} backend: {}{RESET}\n\n",
        shown.len(),
        plural(shown.len()),
        MEMORIES_DIR,
        memory.backend_label(),
    ));
    if let Some(shape) = shape {
        out.push_str(&format!("{DIM}filtered by shape: {shape}{RESET}\n"));
    }
    if shown.is_empty() {
        if let Some(shape) = shape {
            out.push_str(&format!(
                "{DIM}no facts carry shape `{shape}` \u{b7} the structure shapes are {}{RESET}\n",
                SHAPES.join(", ")
            ));
        } else {
            out.push_str(&format!(
                "{DIM}no facts yet \u{b7} a fact is {}/<name>.md with a frontmatter block (see docs/features/memory.md){RESET}\n",
                MEMORIES_DIR
            ));
        }
        return out;
    }
    for fact in &shown {
        out.push_str(&render_card(fact));
    }
    out.push_str(&format!(
        "\n{DIM}{} fact{} \u{b7} {} protected \u{b7} {} retired (superseded or expired){RESET}\n",
        live.len(),
        plural(live.len()),
        protected,
        retired,
    ));
    out.push_str(&format!("{DIM}{}{RESET}\n", memory.root().display()));
    out
}

/// One fact's card.
fn render_card(fact: &Fact) -> String {
    let header_right = format!("{} \u{b7} {}", fact.fact_type, fact.basis.label());
    let header = format!(
        "{DIM}\u{250c}\u{2500}{RESET} {BOLD}{ACCENT}{name}{RESET} {DIM}{dashes} {TEXT}{right}{RESET} {DIM}\u{2510}{RESET}",
        name = fact.name,
        dashes = dashes_for(&format!(" {} ", fact.name), &header_right),
        right = header_right,
    );
    let mut lines: Vec<String> = Vec::new();
    for chunk in wrap_detail(&fact.description, CARD_WIDTH - 4) {
        lines.push(format!("{DIM}\u{2502}{RESET} {TEXT}{chunk}{RESET}"));
    }
    let shape = if fact.shape.is_empty() {
        "\u{2014}".to_string()
    } else {
        fact.shape.join(", ")
    };
    let valid_until = if fact.valid_until.trim().is_empty() {
        "\u{2014}".to_string()
    } else {
        fact.valid_until.clone()
    };
    lines.push(format!(
        "{DIM}\u{2502}{RESET} {DIM}shape: {shape} \u{b7} recalls: {} \u{b7} valid-until: {valid_until}{RESET}",
        fact.recall_count
    ));
    lines.push(format!("{DIM}\u{2502}{RESET} {DIM}{}{RESET}", fact.path.display()));
    let bottom = format!("{DIM}\u{2514}{}\u{2518}{RESET}", "\u{2500}".repeat(CARD_WIDTH - 2));
    let mut card = format!("{header}\n");
    for line in lines {
        card.push_str(&line);
        card.push('\n');
    }
    card.push_str(&bottom);
    card.push('\n');
    card
}

/// Render the ranked assembly for one instruction: every fact above the
/// floor with its score and the channels that produced it, then the
/// block that would enter a turn's context, then what the budget
/// dropped. A dry run — printing changes no recall-count and records no
/// row; the turn path (part 2) records the hits.
pub fn render_recall(memory: &Memory, instruction: &str) -> String {
    let assembly = memory.assemble(instruction, &[]);
    let mut out = String::new();
    out.push_str(&format!(
        "{BOLD}fleet memories --recall{RESET} {DIM}\u{2014} \"{instruction}\"{RESET}\n"
    ));
    let shape_line = match assembly.shape {
        Some(shape) => format!("shape: {shape}"),
        None => "shape: none \u{2014} surface recall (the documented fallback)".to_string(),
    };
    out.push_str(&format!(
        "{DIM}{shape_line} \u{b7} backend: {} \u{b7} {} fact{} ranked \u{b7} {} fit the {}-token budget{RESET}\n\n",
        memory.backend_label(),
        assembly.ranked.len(),
        plural(assembly.ranked.len()),
        assembly.recalled.len(),
        assembly.budget,
    ));
    let live = memory
        .facts()
        .iter()
        .filter(|fact| !fact.forgotten(&calib::today()))
        .count();
    if live == 0 {
        out.push_str(&format!(
            "{DIM}no facts to recall from \u{b7} {}/ holds no live facts{RESET}\n",
            MEMORIES_DIR
        ));
        return out;
    }
    if assembly.ranked.is_empty() {
        out.push_str(&format!(
            "{DIM}no fact cleared the {MIN_RECALL_SCORE} floor for this instruction \u{b7} the turn answers from the surface alone{RESET}\n"
        ));
        return out;
    }
    for hit in &assembly.ranked {
        out.push_str(&render_recall_card(hit));
    }
    let block = render_assembly(&assembly);
    if block.is_empty() {
        out.push_str(&format!(
            "{DIM}nothing would enter a turn's context for this instruction{RESET}\n"
        ));
    } else {
        out.push_str(&format!(
            "{DIM}what would enter a turn's context ({} of {} tokens):{RESET}\n{block}",
            assembly.used, assembly.budget
        ));
    }
    if !assembly.dropped.is_empty() {
        out.push_str(&format!(
            "{DIM}dropped under budget: {}{RESET}\n",
            assembly.dropped.join(", ")
        ));
    }
    out.push_str(&format!(
        "{DIM}a dry run \u{b7} no recall-count changed and no row recorded; the turn path records hits{RESET}\n"
    ));
    out
}

/// One ranked fact's card: the name with its score, then the channels
/// that produced the score — the "why" — then the fact.
fn render_recall_card(hit: &ScoredFact) -> String {
    let header_right = format!("{:.2}", hit.score.total);
    let header = format!(
        "{DIM}\u{250c}\u{2500}{RESET} {BOLD}{ACCENT}{name}{RESET} {DIM}{dashes} {TEXT}{right}{RESET} {DIM}\u{2510}{RESET}",
        name = hit.fact.name,
        dashes = dashes_for(&format!(" {} ", hit.fact.name), &header_right),
        right = header_right,
    );
    let mut lines: Vec<String> = Vec::new();
    // The channels render as two fixed lines rather than one wrapped
    // one: the wrap point would fall mid-phrase ("shape" ending a line,
    // "+0.15" opening the next), and the "why" must read as one fact.
    lines.push(format!(
        "{DIM}\u{2502}{RESET} {DIM}vector {:.2} \u{b7} keyword {:.2} \u{b7} importance {:.2}{RESET}",
        hit.score.vector, hit.score.keyword, hit.score.importance
    ));
    let mut standing = format!("decay {:.2}", hit.score.decay);
    if hit.score.shape_boost > 0.0 {
        standing.push_str(&format!(" \u{b7} shape +{:.2}", hit.score.shape_boost));
    }
    lines.push(format!("{DIM}\u{2502}{RESET} {DIM}{standing}{RESET}"));
    for chunk in wrap_detail(&hit.fact.description, CARD_WIDTH - 4) {
        lines.push(format!("{DIM}\u{2502}{RESET} {TEXT}{chunk}{RESET}"));
    }
    let bottom = format!("{DIM}\u{2514}{}\u{2518}{RESET}", "\u{2500}".repeat(CARD_WIDTH - 2));
    let mut card = format!("{header}\n");
    for line in lines {
        card.push_str(&line);
        card.push('\n');
    }
    card.push_str(&bottom);
    card.push('\n');
    card
}

/// The dash run that pads a card header so its right edge aligns.
fn dashes_for(left: &str, right: &str) -> String {
    let used = left.chars().count() + right.chars().count() + 2;
    "\u{2500}".repeat(CARD_WIDTH.saturating_sub(used + 4).max(1))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    /// A scratch corpus root, unique per call.
    fn scratch(tag: &str) -> PathBuf {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let serial = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "fleet-memory-{tag}-{}-{serial}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// The `YYYY-MM-DD` string `days` before today — the tests' own
    /// clock arithmetic, so an expiry or a decay assertion never depends
    /// on a hard-coded date.
    fn days_ago(days: i64) -> String {
        let value = parse_day(&calib::today()).unwrap() - days;
        let z = value + 719468;
        let era = if z >= 0 { z } else { z - 146096 } / 146097;
        let day_of_era = z - era * 146097;
        let year_of_era =
            (day_of_era - day_of_era / 1460 + day_of_era / 36524 - day_of_era / 146096) / 365;
        let year = year_of_era + era * 400;
        let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
        let mp = (5 * day_of_year + 2) / 153;
        let day = day_of_year - (153 * mp + 2) / 5 + 1;
        let month = if mp < 10 { mp + 3 } else { mp - 9 };
        let year = if month <= 2 { year + 1 } else { year };
        format!("{year:04}-{month:02}-{day:02}")
    }

    /// Write one fact file into `root`.
    fn write_fact(root: &Path, name: &str, text: &str) -> PathBuf {
        fs::create_dir_all(root).unwrap();
        let path = root.join(format!("{name}.md"));
        fs::write(&path, text).unwrap();
        path
    }

    /// A well-formed fact in the writer's field order.
    fn good_fact(name: &str, description: &str, fact_type: &str, shape: &str) -> String {
        format!(
            "---\nname: {name}\ndescription: \"{description}\"\ntype: {fact_type}\ndate: {today}\nbasis: confirmed-by\nshape: [{shape}]\nrecall-count: 0\nsuperseded-by:\nvalid-until:\n---\n\nThe full content of {name}.\n",
            today = calib::today()
        )
    }

    fn fact_names(facts: &[Fact]) -> Vec<&str> {
        facts.iter().map(|fact| fact.name.as_str()).collect()
    }

    /// A deterministic embedder over a fixed scripted vocabulary: each
    /// text embeds as the indicator of which scripted terms it carries,
    /// so a cosine is a scripted-token overlap and every ordering below
    /// is exact. It shares its call counter with the test, which is how
    /// the index-cache test proves a rebuild embeds only what changed.
    struct ScriptedEmbedder {
        calls: Arc<AtomicUsize>,
    }

    impl ScriptedEmbedder {
        /// The shared counter and the boxed embedder that increments it.
        fn new() -> (Arc<AtomicUsize>, Box<ScriptedEmbedder>) {
            let calls = Arc::new(AtomicUsize::new(0));
            (Arc::clone(&calls), Box::new(ScriptedEmbedder { calls }))
        }
    }

    /// The scripted vocabulary, in dimension order.
    const SCRIPTED_TERMS: [&str; 5] = ["deploy", "railway", "render", "shader", "walnut"];

    impl Embedder for ScriptedEmbedder {
        fn embed(&self, text: &str) -> Option<Vec<f32>> {
            self.calls.fetch_add(1, Ordering::Relaxed);
            let lower = text.to_lowercase();
            Some(
                SCRIPTED_TERMS
                    .iter()
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
    }

    // --- the frontmatter subset ---

    #[test]
    fn the_parser_accepts_the_fork_corpus_shapes() {
        // A folded plain scalar, a bare optional field, a bracketed
        // shape list, an unknown key — every shape the fork's own tree
        // carries.
        let document = parse_document(
            "---\nname: demo\ndescription: For any deploy operation, read CLAUDE.local.md directly for\n  credentials instead of grep-extracting them\ntype: feedback\ndate: 2026-10-01\nbasis: confirmed-by\nshape: [business_analysis, deploy]\nsuperseded-by:\nextra: ignored\n---\n\nThe body.\n",
        )
        .unwrap();
        assert_eq!(document.get("name"), Some("demo"));
        // The folded scalar reads back as one sentence.
        assert_eq!(
            document.get("description"),
            Some("For any deploy operation, read CLAUDE.local.md directly for credentials instead of grep-extracting them")
        );
        // A bare optional field is present and empty, not absent.
        assert_eq!(document.get("superseded-by"), Some(""));
        assert_eq!(shape_tags(document.get("shape")), vec!["business_analysis", "deploy"]);
        // Unknown keys are kept, not rejected: forward compatibility.
        assert_eq!(document.get("extra"), Some("ignored"));
        assert_eq!(document.body, "The body.");
        // A quoted description with colons inside splits at the key's
        // colon only.
        let quoted = parse_document(
            "---\nname: q\ndescription: \"dnd-vtt deploy (confirmed 2026-08-14): per-branch auto-deploy is REAL\"\n---\nbody\n",
        )
        .unwrap();
        assert_eq!(
            quoted.get("description"),
            Some("dnd-vtt deploy (confirmed 2026-08-14): per-branch auto-deploy is REAL")
        );
        // A bare shape scalar reads as no tags, the fork reader's rule.
        assert!(shape_tags(Some("deploy")).is_empty());
    }

    #[test]
    fn the_parser_rejects_what_it_does_not_speak() {
        // No opening fence.
        assert!(parse_document("name: demo\n").unwrap_err().contains("does not open"));
        // No closing fence.
        assert!(parse_document("---\nname: demo\n").unwrap_err().contains("never closes"));
        // A list entry.
        assert!(parse_document("---\n- item\n---\nbody\n").unwrap_err().contains("key: value"));
        // A bare scalar line.
        assert!(parse_document("---\nbare\n---\nbody\n").unwrap_err().contains("key: value"));
        // An indented line with no field to continue.
        assert!(parse_document("---\n  orphan\n---\nbody\n")
            .unwrap_err()
            .contains("no field to continue"));
        // A duplicated key.
        assert!(parse_document("---\nname: a\nname: b\ndescription: d\n---\nbody\n")
            .unwrap_err()
            .contains("twice"));
        // An unbalanced quoted key.
        assert!(parse_document("---\n\"name: demo\ndescription: d\n---\nbody\n")
            .unwrap_err()
            .contains("unbalanced quoted key"));
        // A missing required key is the loader's rule, not the parser's.
        let document = parse_document("---\ntype: project\n---\nbody\n").unwrap();
        assert!(document.required("name").unwrap_err().contains("name"));
        assert!(document.required("description").unwrap_err().contains("description"));
    }

    #[test]
    fn a_utf8_bom_is_stripped_and_a_bomd_failure_names_the_bom() {
        let document =
            parse_document("\u{feff}---\nname: demo\ndescription: d\n---\n\nbody\n").unwrap();
        assert_eq!(document.get("name"), Some("demo"));
        assert_eq!(document.body, "body");
        let reason = parse_document("\u{feff}name: demo\n").unwrap_err();
        assert!(reason.contains("does not open"), "{reason}");
        assert!(reason.contains("UTF-8 BOM"), "{reason}");
    }

    // --- loading ---

    #[test]
    fn facts_load_with_a_malformed_one_skipped_by_name() {
        let root = scratch("skip");
        write_fact(&root, "good-one", &good_fact("good-one", "the first fact", "project", "deploy"));
        write_fact(&root, "good-two", &good_fact("good-two", "the second fact", "reference", ""));
        // The bare minimum: no basis, no recall-count, no type — the
        // loader's defaults apply rather than a rejection.
        write_fact(&root, "minimal", "---\nname: minimal\ndescription: \"the bare minimum\"\n---\n\nbody\n");
        // Five ways to be malformed: no frontmatter, a missing
        // description, a name that is not the file stem, a basis outside
        // the vocabulary, and a recall-count that is not an integer.
        write_fact(&root, "broken-fence", "# no fence here\n");
        write_fact(&root, "broken-fields", "---\nname: broken-fields\n---\nbody\n");
        write_fact(&root, "broken-name", &good_fact("some-other-name", "d", "project", ""));
        write_fact(
            &root,
            "broken-basis",
            "---\nname: broken-basis\ndescription: d\nbasis: guessed\n---\nbody\n",
        );
        write_fact(
            &root,
            "broken-count",
            "---\nname: broken-count\ndescription: d\nrecall-count: many\n---\nbody\n",
        );
        // The generated index is not a fact.
        fs::write(index_path(&root), "{\"facts\":{}}").unwrap();

        let facts = load_facts(&root);
        assert_eq!(fact_names(&facts), vec!["good-one", "good-two", "minimal"]);
        let minimal = facts.iter().find(|fact| fact.name == "minimal").unwrap();
        assert_eq!(minimal.basis, Basis::Inferred);
        assert_eq!(minimal.recall_count, 0);
        assert_eq!(minimal.body, "body");
        assert!(facts[0].path.ends_with("good-one.md"));
        assert!(facts[0].stamp.is_some());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_missing_directory_is_no_facts_not_an_error() {
        let root = scratch("absent").join("nowhere");
        assert!(load_facts(&root).is_empty());
    }

    #[test]
    fn protected_types_are_the_identity_and_preference_taxonomy() {
        let root = scratch("protected");
        write_fact(&root, "identity", &good_fact("identity", "who the owner is", "user", ""));
        write_fact(&root, "rule", &good_fact("rule", "a standing rule", "feedback", ""));
        write_fact(&root, "task", &good_fact("task", "project state", "project", ""));
        let facts = load_facts(&root);
        let protected: Vec<&str> = facts
            .iter()
            .filter(|fact| fact.protected())
            .map(|fact| fact.name.as_str())
            .collect();
        assert_eq!(protected, vec!["identity", "rule"]);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_forgotten_fact_stays_out_of_recall() {
        let root = scratch("forgotten");
        // Superseded: folded into another fact.
        write_fact(
            &root,
            "folded",
            &format!(
                "---\nname: folded\ndescription: \"a deploy fact that is failing\"\ntype: project\ndate: {}\nbasis: confirmed-by\nsuperseded-by: newer\n---\n\nbody\n",
                calib::today()
            ),
        );
        // Expired: a valid-until in the past.
        write_fact(
            &root,
            "expired",
            &format!(
                "---\nname: expired\ndescription: \"a deploy fact that is failing\"\ntype: project\ndate: {}\nbasis: confirmed-by\nvalid-until: {}\n---\n\nbody\n",
                calib::today(),
                days_ago(1)
            ),
        );
        // Live: a valid-until in the future.
        write_fact(
            &root,
            "live",
            &format!(
                "---\nname: live\ndescription: \"a deploy fact that is failing\"\ntype: project\ndate: {}\nbasis: confirmed-by\nvalid-until: 2099-01-01\n---\n\nbody\n",
                calib::today()
            ),
        );
        let memory = Memory::load_with(&root, Box::new(KeywordOnly));
        let names: Vec<String> = memory
            .recall("the deploy is failing")
            .into_iter()
            .map(|hit| hit.fact.name)
            .collect();
        assert_eq!(names, vec!["live".to_string()]);
        let _ = fs::remove_dir_all(&root);
    }

    // --- the hybrid ranker ---

    /// The order-test corpus: all dated today, no recalls, no shapes.
    /// `alpha` and `zeta` share the query's scripted vector exactly;
    /// `eta` overlaps it partially but covers more query tokens; `junk`
    /// shares one query token and no vector term.
    fn order_corpus(root: &Path) {
        write_fact(root, "alpha", &good_fact("alpha", "railway deploy per branch", "project", ""));
        write_fact(root, "zeta", &good_fact("zeta", "deploy railway", "project", ""));
        write_fact(root, "eta", &good_fact("eta", "the railway is failing", "project", ""));
        write_fact(root, "junk", &good_fact("junk", "the walnut table texture", "project", ""));
    }

    #[test]
    fn the_hybrid_ranker_orders_a_scripted_corpus_deterministically() {
        let root = scratch("order");
        order_corpus(&root);

        // Vector-off: the keyword channel decides. `eta` covers four of
        // the five query tokens against `alpha`/`zeta`'s two; `junk`
        // covers one and lands under the floor.
        let keyword_only = Memory::load_with(&root, Box::new(KeywordOnly));
        let off: Vec<String> = keyword_only
            .recall("the railway deploy is failing")
            .into_iter()
            .map(|hit| hit.fact.name)
            .collect();
        assert_eq!(off, vec!["eta".to_string(), "alpha".to_string(), "zeta".to_string()]);
        // The same corpus ranks the same way twice: deterministic.
        let again: Vec<String> = keyword_only
            .recall("the railway deploy is failing")
            .into_iter()
            .map(|hit| hit.fact.name)
            .collect();
        assert_eq!(off, again);
        // The floor dropped the junk: an unrelated fact never enters.
        assert!(!off.contains(&"junk".to_string()));

        // Vector-on: the same corpus, the same query, a different order.
        // `alpha` and `zeta` match the query's vector exactly (cosine
        // 1.0) and overtake `eta`'s partial match.
        let (calls, embedder) = ScriptedEmbedder::new();
        let vector_on = Memory::load_with(&root, embedder);
        // The load-time index build embedded every fact exactly once;
        // the recalls below each embed the query beside it.
        assert_eq!(calls.load(Ordering::Relaxed), 4);
        let on: Vec<String> = vector_on
            .recall("the railway deploy is failing")
            .into_iter()
            .map(|hit| hit.fact.name)
            .collect();
        assert_eq!(on, vec!["alpha".to_string(), "zeta".to_string(), "eta".to_string()]);
        // The vector channel is what moved the order.
        let hits = vector_on.recall("the railway deploy is failing");
        assert!((hits[0].score.vector - 1.0).abs() < 1e-9, "{:?}", hits[0].score);
        let eta = hits.iter().find(|hit| hit.fact.name == "eta").expect("eta ranks");
        assert!((eta.score.vector - 1.0 / 2.0_f64.sqrt()).abs() < 1e-9, "{:?}", eta.score);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn the_named_weights_compose_the_score_and_the_floor_drops_noise() {
        let today = parse_day(&calib::today());
        // A fact dated today with the ceiling's recalls: importance
        // saturates, decay is fresh.
        let score = Score::compute(0.8, 0.5, RECALL_CEILING, &calib::today(), today, false);
        let expected =
            W_VECTOR * 0.8 + W_KEYWORD * 0.5 + W_IMPORTANCE * 1.0 + W_DECAY * 1.0;
        assert!((score.total - expected).abs() < 1e-12, "{score:?}");
        // The shape boost rides beside the four channels, not inside them.
        let boosted = Score::compute(0.8, 0.5, RECALL_CEILING, &calib::today(), today, true);
        assert!((boosted.total - expected - SHAPE_BOOST).abs() < 1e-12, "{boosted:?}");
        // Importance saturates: eight recalls and eighty score the same.
        let saturated = Score::compute(0.0, 0.0, 80, &calib::today(), today, false);
        let at_ceiling = Score::compute(0.0, 0.0, RECALL_CEILING, &calib::today(), today, false);
        assert_eq!(saturated.importance, at_ceiling.importance);
        // A fact with no readable date contributes no decay term.
        assert_eq!(Score::compute(0.0, 0.0, 0, "not-a-date", today, false).decay, 0.0);
        // Decay halves at the half-life.
        let decayed = Score::compute(0.0, 0.0, 0, &days_ago(90), today, false);
        assert!((decayed.decay - 0.5).abs() < 1e-9, "{decayed:?}");
        // The floor: a fresh fact with no coverage and no vector clears
        // nothing, and one with full coverage does.
        assert!(Score::compute(0.0, 0.0, 0, &calib::today(), today, false).total < MIN_RECALL_SCORE);
        assert!(Score::compute(0.0, 1.0, 0, &calib::today(), today, false).total >= MIN_RECALL_SCORE);
    }

    // --- shape routing ---

    #[test]
    fn shape_routing_classifies_each_shape_and_falls_back() {
        // One instruction per shape, each naming its shape's keywords.
        let cases = [
            ("push the staging branch and watch the railway deploy", "deploy"),
            ("the webpack bundle fails to compile", "build"),
            ("bump the lockfile after the pnpm upgrade", "deps"),
            ("the readme drifted from the documentation", "docs"),
            ("the shader layout repaints every frame", "rendering"),
            ("the websocket desyncs on reconnect", "state-sync"),
            ("rewrite the migration and revert the rename", "change"),
        ];
        for (instruction, expected) in cases {
            assert_eq!(classify_shape(instruction), Some(expected), "{instruction}");
        }
        // Keyword order decides when two shapes' terms both appear: the
        // earlier shape in the taxonomy wins.
        assert_eq!(classify_shape("the build release"), Some("deploy"));
        // An unclassifiable instruction is the documented fallback, not
        // an error: no shape, no boost, the surface alone answers.
        assert_eq!(classify_shape("tell me about the walnut table"), None);
    }

    #[test]
    fn an_unclassified_instruction_answers_from_the_surface_alone() {
        let root = scratch("fallback");
        let twin = "the railway deploy is a lesson";
        write_fact(&root, "tagged", &good_fact("tagged", twin, "project", "deploy"));
        write_fact(&root, "untagged", &good_fact("untagged", twin, "project", ""));
        let memory = Memory::load_with(&root, Box::new(KeywordOnly));
        // Unclassified: no boost, so the tagged and untagged twins rank
        // equally and the name breaks the tie.
        let hits = memory.recall("a lesson");
        let tagged = hits.iter().find(|hit| hit.fact.name == "tagged").expect("tagged ranks");
        let untagged = hits.iter().find(|hit| hit.fact.name == "untagged").expect("untagged ranks");
        assert_eq!(tagged.score.shape_boost, 0.0);
        assert_eq!(tagged.score.total, untagged.score.total);
        // Classified: the same-shape fact takes the boost and outranks
        // its twin.
        let hits = memory.recall("the railway deploy is failing");
        let names: Vec<&str> = hits.iter().map(|hit| hit.fact.name.as_str()).collect();
        assert_eq!(names, vec!["tagged", "untagged"]);
        assert!(hits[0].score.shape_boost > 0.0);
        let _ = fs::remove_dir_all(&root);
    }

    // --- feedback ---

    #[test]
    fn a_recall_hit_updates_recall_count_and_reranks() {
        let root = scratch("hit");
        let path = write_fact(
            &root,
            "hit-me",
            &format!(
                "---\nname: hit-me\ndescription: \"a deploy lesson: the railway build triggers per branch\"\ntype: project\ndate: {}\nbasis: confirmed-by\nshape: [deploy]\nrecall-count: 2\nsuperseded-by:\nvalid-until:\n---\n\nThe full body, with a second line.\n",
                days_ago(0)
            ),
        );
        let before = fs::read_to_string(&path).unwrap();
        let memory = Memory::load_with(&root, Box::new(KeywordOnly));

        let hits = memory.recall_hits("the railway deploy is failing", None);
        assert_eq!(hits.len(), 1);
        let first_score = hits[0].score.total;

        // The rewrite is bounded: the one line it owns changed, and
        // every other byte — the body above all — is preserved.
        let after = fs::read_to_string(&path).unwrap();
        assert!(after.contains("recall-count: 3"), "{after}");
        assert!(after.contains("The full body, with a second line."), "{after}");
        assert!(after.contains("shape: [deploy]"), "{after}");
        assert_eq!(
            after
                .lines()
                .filter(|line| !line.starts_with("recall-count"))
                .collect::<Vec<_>>(),
            before
                .lines()
                .filter(|line| !line.starts_with("recall-count"))
                .collect::<Vec<_>>()
        );

        // The hit re-ranks: the fact's importance channel rose, so the
        // same instruction scores it higher than before.
        let second = memory.recall("the railway deploy is failing");
        assert!(
            second[0].score.total > first_score,
            "{:?} vs {first_score}",
            second[0].score
        );
        // A second hit counts again.
        memory.recall_hits("the railway deploy is failing", None);
        assert!(fs::read_to_string(&path).unwrap().contains("recall-count: 4"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_recall_hit_with_no_count_line_inserts_one_and_preserves_the_body() {
        let root = scratch("insert");
        let path = write_fact(
            &root,
            "no-count",
            "---\nname: no-count\ndescription: \"a deploy lesson about railway\"\ntype: project\ndate: 2026-10-13\nbasis: inferred\nshape: [deploy]\n---\n\nBody one.\n\nBody two.\n",
        );
        let memory = Memory::load_with(&root, Box::new(KeywordOnly));
        memory.recall_hits("the railway deploy is failing", None);
        let after = fs::read_to_string(&path).unwrap();
        assert!(after.contains("recall-count: 1"), "{after}");
        assert!(after.ends_with("Body one.\n\nBody two.\n"), "{after}");
        assert!(after.contains("basis: inferred"), "{after}");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn the_recall_hit_row_carries_the_fact_the_shape_and_the_score() {
        let root = scratch("calib-row");
        write_fact(
            &root,
            "hit-me",
            &good_fact("hit-me", "a deploy lesson about railway", "project", "deploy"),
        );
        let memory = Memory::load_with(&root, Box::new(KeywordOnly));
        let stream_root = scratch("calib-stream");
        let stream = calib::Stream::open(&stream_root);
        let hits = memory.recall_hits("the railway deploy is failing", Some(&stream));
        assert_eq!(hits.len(), 1);
        drop(stream);
        let rows = calib::read_day(&stream_root, &calib::today(), 64);
        let row = rows
            .iter()
            .find(|row| row.op == calib::OP_RECALL_HIT)
            .expect("the recall_hit row landed in the stream");
        assert_eq!(row.fact, "hit-me");
        assert_eq!(row.shape, "deploy");
        assert!((row.score - hits[0].score.total).abs() < 1e-12, "{row:?}");
        // An unclassified instruction records an empty shape, not an
        // invented one.
        let stream = calib::Stream::open(&stream_root);
        memory.recall_hits("a lesson", Some(&stream));
        drop(stream);
        let rows = calib::read_day(&stream_root, &calib::today(), 64);
        let unclassified = rows
            .iter()
            .filter(|row| row.op == calib::OP_RECALL_HIT)
            .last()
            .expect("the second recall_hit row");
        assert_eq!(unclassified.shape, "");
        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&stream_root);
    }

    // --- the index ---

    #[test]
    fn the_index_caches_vectors_and_rebuilds_only_what_changed() {
        let root = scratch("index");
        order_corpus(&root);

        // First load embeds every fact and writes the cache.
        let (calls, embedder) = ScriptedEmbedder::new();
        let memory = Memory::load_with(&root, embedder);
        assert_eq!(calls.load(Ordering::Relaxed), 4);
        assert!(index_path(&root).exists());
        assert_eq!(memory.backend(), "scripted");
        drop(memory);

        // Second load: every stamp matches, nothing is embedded.
        let (calls, embedder) = ScriptedEmbedder::new();
        let _ = Memory::load_with(&root, embedder);
        assert_eq!(calls.load(Ordering::Relaxed), 0);

        // One changed fact: only it is re-embedded.
        let path = root.join("alpha.md");
        let text = fs::read_to_string(&path).unwrap();
        fs::write(&path, format!("{text}\nAn appended line changes the size.\n")).unwrap();
        let (calls, embedder) = ScriptedEmbedder::new();
        let _ = Memory::load_with(&root, embedder);
        assert_eq!(calls.load(Ordering::Relaxed), 1);

        // The keyword-only backend ignores the cache entirely: a stale
        // vector must not serve recall on a machine whose server is
        // down, or doctor's naming would contradict what recall used.
        let keyword_only = Memory::load_with(&root, Box::new(KeywordOnly));
        assert_eq!(keyword_only.backend(), "keyword-only");
        let _ = fs::remove_dir_all(&root);
    }

    // --- assembly ---

    /// The pressure corpus: two protected facts, three ranked facts
    /// above the floor, and one below it.
    fn pressure_corpus(root: &Path) {
        write_fact(
            &root,
            "identity",
            &good_fact("identity", "who the owner is and how he works, in a standing fact", "user", ""),
        );
        write_fact(
            &root,
            "rule",
            &good_fact("rule", "a standing rule the owner stated about documentation", "feedback", "docs"),
        );
        write_fact(
            &root,
            "top",
            &good_fact("top", "the railway deploy is failing per branch", "project", "deploy"),
        );
        write_fact(
            &root,
            "middle",
            &good_fact("middle", "the railway deploy waits for the webhook", "project", "deploy"),
        );
        write_fact(
            &root,
            "bottom",
            &good_fact("bottom", "the railway deploy logs its outcome", "project", "deploy"),
        );
        write_fact(&root, "junk", &good_fact("junk", "the walnut table texture", "project", ""));
    }

    #[test]
    fn the_budget_assembly_survives_pressure() {
        let root = scratch("pressure");
        pressure_corpus(&root);
        let memory = Memory::load_with(&root, Box::new(KeywordOnly));
        let assembly = memory.assemble_within("the railway deploy is failing", &[], 48);

        // The protected tier survives: both protected facts are in the
        // block whatever the budget says.
        let block = render_assembly(&assembly);
        assert!(block.contains("[user confirmed-by]"), "{block}");
        assert!(block.contains("[feedback confirmed-by]"), "{block}");
        assert_eq!(assembly.protected.len(), 2);

        // The junk never ranked, so it was never a budget candidate.
        assert!(!assembly.ranked.iter().any(|hit| hit.fact.name == "junk"));

        // The strongest recall always enters; the rest are bounded by
        // the budget and the ones that did not fit are named.
        assert_eq!(assembly.recalled[0].fact.name, "top");
        assert!(assembly.dropped.contains(&"bottom".to_string()), "{:?}", assembly.dropped);
        assert!(block.contains("the railway deploy is failing per branch"), "{block}");
        assert!(!block.contains("waits for the webhook"), "{block}");
        assert!(!block.contains("logs its outcome"), "{block}");

        // The production budget is the named constant, and a generous
        // one keeps everything.
        let full = memory.assemble("the railway deploy is failing", &[]);
        assert_eq!(full.budget, ASSEMBLY_TOKEN_BUDGET);
        assert_eq!(full.recalled.len(), 3);
        assert!(full.dropped.is_empty());
        assert!(full.used <= full.budget);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn the_assembly_orders_protected_then_recall_then_working() {
        let root = scratch("order-tiers");
        pressure_corpus(&root);
        let memory = Memory::load_with(&root, Box::new(KeywordOnly));
        // The working tier is part 2's seam: a caller passes it, the
        // assembly places it last and bounds it by the budget.
        let working: Vec<Fact> = load_facts(&root)
            .into_iter()
            .find(|fact| fact.name == "middle")
            .map(|fact| Fact { name: "working-note".to_string(), ..fact })
            .into_iter()
            .collect();
        let assembly = memory.assemble_within("the railway deploy is failing", &working, 512);
        let block = render_assembly(&assembly);
        let protected_at = block.find("[user confirmed-by]").expect("the protected tier renders");
        let recalled_at = block.find("[deploy").expect("the recall tier renders");
        let working_at = block.find("[working confirmed-by]").expect("the working tier renders");
        assert!(protected_at < recalled_at, "{block}");
        assert!(recalled_at < working_at, "{block}");
        // A forgotten working fact is dropped before the budget sees it.
        let expired = Fact {
            name: "expired-note".to_string(),
            valid_until: days_ago(1),
            ..working[0].clone()
        };
        let assembly = memory.assemble_within("the railway deploy is failing", &[expired], 512);
        assert!(assembly.working.is_empty());
        let _ = fs::remove_dir_all(&root);
    }

    // --- the surface ---

    #[test]
    fn the_surface_is_cards_and_never_a_table() {
        let root = scratch("render");
        pressure_corpus(&root);
        let memory = Memory::load_with(&root, Box::new(KeywordOnly));
        let rendered = render(&memory, None);
        assert!(rendered.contains("fleet memories"), "{rendered}");
        // One card per live fact, the retired footer, and the root path.
        assert_eq!(rendered.matches('\u{250c}').count(), 6, "{rendered}");
        assert!(rendered.contains("6 facts \u{b7} 2 protected \u{b7} 0 retired"), "{rendered}");
        assert!(rendered.contains(&root.display().to_string()), "{rendered}");
        // Each card carries name, type, basis, shape, recall-count, and
        // valid-until.
        assert!(rendered.contains("project \u{b7} confirmed-by"), "{rendered}");
        assert!(
            rendered.contains("shape: deploy \u{b7} recalls: 0 \u{b7} valid-until: \u{2014}"),
            "{rendered}"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn the_shape_filter_narrows_the_cards_and_names_the_taxonomy_when_empty() {
        let root = scratch("shape-filter");
        pressure_corpus(&root);
        let memory = Memory::load_with(&root, Box::new(KeywordOnly));
        let filtered = render(&memory, Some("deploy"));
        assert!(filtered.contains("filtered by shape: deploy"), "{filtered}");
        assert_eq!(filtered.matches('\u{250c}').count(), 3, "{filtered}");
        // An unknown shape names the taxonomy rather than showing
        // nothing silently.
        let empty = render(&memory, Some("telemetry"));
        assert!(empty.contains("no facts carry shape `telemetry`"), "{empty}");
        assert!(
            empty.contains("rendering, state-sync, deploy, build, change, deps, docs"),
            "{empty}"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn the_recall_surface_prints_the_ranking_the_block_and_the_dry_run() {
        let root = scratch("render-recall");
        pressure_corpus(&root);
        let memory = Memory::load_with(&root, Box::new(KeywordOnly));
        let before: Vec<String> = load_facts(&root)
            .iter()
            .map(|fact| fs::read_to_string(&fact.path).unwrap())
            .collect();
        let rendered = render_recall(&memory, "the railway deploy is failing");
        // The shape, the backend, and the counts are named.
        assert!(rendered.contains("shape: deploy"), "{rendered}");
        assert!(rendered.contains("backend: keyword-only"), "{rendered}");
        assert!(rendered.contains("3 facts ranked"), "{rendered}");
        // Each ranked fact's card carries its score and the channels
        // that produced it — the "why".
        assert!(rendered.contains("vector 0.00 \u{b7} keyword"), "{rendered}");
        assert!(rendered.contains("decay 1.00 \u{b7} shape +0.15"), "{rendered}");
        // The assembled block renders, and the junk is absent from it.
        assert!(rendered.contains("Standing context (protected"), "{rendered}");
        assert!(rendered.contains("shape-routed recall for `deploy`"), "{rendered}");
        assert!(!rendered.contains("walnut"), "{rendered}");
        // The dry run is named, and it is true: no file changed.
        assert!(rendered.contains("a dry run"), "{rendered}");
        let after: Vec<String> = load_facts(&root)
            .iter()
            .map(|fact| fs::read_to_string(&fact.path).unwrap())
            .collect();
        assert_eq!(before, after);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn the_recall_surface_names_the_unclassified_fallback() {
        let root = scratch("render-fallback");
        pressure_corpus(&root);
        let memory = Memory::load_with(&root, Box::new(KeywordOnly));
        let rendered = render_recall(&memory, "tell me about the walnut table");
        assert!(
            rendered.contains("shape: none \u{2014} surface recall (the documented fallback)"),
            "{rendered}"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn an_empty_corpus_says_so_on_both_surfaces() {
        let root = scratch("empty");
        let memory = Memory::load_with(&root, Box::new(KeywordOnly));
        let rendered = render(&memory, None);
        assert!(rendered.contains("no facts yet"), "{rendered}");
        let recall = render_recall(&memory, "the railway deploy is failing");
        assert!(recall.contains("no facts to recall from"), "{recall}");
        let _ = fs::remove_dir_all(&root);
    }
}
