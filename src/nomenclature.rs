//! Nomenclature: the project's canonical identifier vocabulary, derived
//! from the tree, and the lexical lookup that answers "what does this
//! project already call this?" before something new is named.
//!
//! **Why lexical, and why the vector channel is not consulted.** A
//! semantic search returns the nearest term, and the nearest term to a
//! concept is usually a synonym — which is the drift this module exists
//! to catch. A sub-agent naming something in a fresh context invents a
//! near-synonym with high semantic density and no direction; the fix is
//! a lookup whose only direction channel is *the term is established in
//! this tree*. So the extractor reads the tree's identifiers and the
//! docs' vocabulary, the lookup matches tokens exactly (with light
//! singular/plural normalization and nothing else), and the answer is
//! one established term with its file:line sites — or a plain statement
//! that no established term exists. Never a ranked list of
//! near-synonyms.
//!
//! **What the vocabulary is.** Three sources, all read off the tree:
//!
//! - **identifiers** — every identifier-shaped word in the code files
//!   under `src/`, `services/`, and `tests/`, split on `snake`, `kebab`,
//!   and `camelCase` boundaries into tokens, with the 2-3 token
//!   compounds kept beside their parts (`recall_count` keeps
//!   `recall_count`, `recall`, `count`). Language keywords and
//!   sub-3-character tokens are dropped: `match` enters the vocabulary
//!   because the project uses it, not because Rust defines it.
//! - **doc vocabulary** — inline backtick spans and fenced code lines
//!   from the markdown docs (`CLAUDE.md`, `AGENTS.md`, `docs/**`), which
//!   is where the docs name things (`calib`, `judge`, `write-time`).
//! - **the file stem** — `src/calib.rs` establishes `calib` at its own
//!   first line, so a module's name is vocabulary whether or not the
//!   prose spells it out.
//!
//! **Establishment.** A term's establishment is how many *distinct
//! files* carry it, with the identifier channel weighted 1000x the doc
//! channel — the tree is the ground truth for what is established, the
//! docs are the tiebreaker — times a recency multiplier from the
//! carrying file's mtime (1.0 within 30 days, 0.75 within 180, 0.5
//! within 365, 0.25 beyond). The formula and its arithmetic are stated
//! at [`establishment`]; every term's identifier/doc file counts ride
//! the entry so the number is auditable rather than magical.
//!
//! **Bounded and cached, because the tree is large.** This repository
//! derives 188 sources / ~7.3k terms in about a second, which is far
//! too slow to repeat per tool call. The derivation runs once per
//! process per root ([`vocabulary`]), bounded by [`MAX_SOURCES`],
//! [`MAX_TERMS`], [`MAX_SITES_PER_TERM`], and [`DERIVE_BUDGET`]; the
//! budget degrades to a partial vocabulary (the walk is sorted, so the
//! partial is a prefix and therefore deterministic) rather than
//! stalling a turn. [`fresh`] is the invalidation rule: the vocabulary
//! names the newest mtime it saw, and a caller can compare it against
//! the tree's before trusting a long-lived cache.
//!
//! **The guard's two verdicts.** A novel identifier with a canonical
//! near-neighbour is *flagged* ([`check`] returns the candidate named,
//! so the model can rename); a case or separator variant of an
//! established term is *refused* ([`is_near_exact`], because that is
//! drift with no other explanation); a genuinely new concept is not
//! flagged at all. The flag is conservative by construction — a guard
//! that cries wolf gets ignored, and then it is worse than nothing.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

// --- the bounds ------------------------------------------------------------

/// How many source files one derivation may read. The fleet tree is
/// 188 files; the cap is headroom for a corpus five times this size and
/// keeps a pathological tree from stalling a turn.
pub const MAX_SOURCES: usize = 1_024;

/// How many distinct terms one derivation may carry.
pub const MAX_TERMS: usize = 20_000;

/// How many file:line sites one term's entry carries. A card shows a
/// handful of sites; the establishment number carries the weight, not
/// the site list.
pub const MAX_SITES_PER_TERM: usize = 12;

/// Wall-clock budget for one derivation. On exhaustion the walk stops
/// where it is: the walk is sorted, so a truncated vocabulary is a
/// deterministic prefix of the full one, and the caller can see it in
/// [`Vocabulary::truncated`].
pub const DERIVE_BUDGET: std::time::Duration = std::time::Duration::from_secs(5);

/// The weight the identifier channel carries over the doc channel in
/// [`establishment`]. The tree is the ground truth for what a project
/// calls something; the docs are the tiebreaker.
const IDENT_WEIGHT: u64 = 1_000;

/// The recency multipliers, in days since the carrying file's mtime:
/// full weight inside a month, three quarters inside six months, half
/// inside a year, a quarter beyond.
const RECENCY_STEPS: [(u64, f64); 4] = [(30, 1.0), (180, 0.75), (365, 0.5), (u64::MAX, 0.25)];

/// The directories the walk descends into, and the single files it
/// reads at the root. `docs/**` is walked whole — features, plans,
/// architecture, and recaps are all vocabulary sources, because the
/// docs are where the project's decisions are written down.
const WALK_DIRS: [&str; 6] = [
    "src",
    "src/bin",
    "src/tui",
    "src/explain",
    "src/judge",
    "services",
];
const WALK_FILES: [&str; 2] = ["CLAUDE.md", "AGENTS.md"];

/// The file extensions read as code. Anything else is skipped.
const CODE_EXTENSIONS: [&str; 4] = ["rs", "py", "toml", "sh"];

/// Directory names never descended into: build output, VCS metadata,
/// and the Python caches the vendored lane cores carry.
const SKIP_DIRS: [&str; 5] = ["target", ".git", "node_modules", "__pycache__", ".venv"];

/// English function words plus Rust's own keywords and prelude names.
/// A term the language defines is not a term the project established —
/// which is why `match` (used) is vocabulary and `impl` (syntax) is not.
const STOPWORDS: [&str; 109] = [
    "the", "and", "for", "with", "that", "this", "from", "into", "onto", "over", "under", "than",
    "then", "when", "what", "which", "while", "will", "would", "should", "could", "also", "each",
    "every", "some", "such", "very", "only", "just", "like", "same", "other", "another", "both",
    "either", "neither", "because", "before", "after", "between", "through", "during", "about",
    "above", "below", "once", "here", "there", "where", "have", "has", "had", "been", "being",
    "does", "done", "did", "not", "but", "all", "any", "out", "off", "its", "their", "them",
    "they", "our", "your", "his", "her", "was", "were", "are", "can", "may", "must", "let", "impl",
    "trait", "enum", "struct", "const", "static", "mut", "ref", "dyn", "pub", "use", "mod", "fn",
    "self", "super", "crate", "async", "await", "loop", "move", "else", "break", "string",
    "option", "result", "vec", "box", "println", "eprintln", "format", "assert",
];

// --- the shapes ------------------------------------------------------------

/// One place a term appears: the file and the 1-indexed line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Site {
    /// The path relative to the root the vocabulary was derived from.
    pub file: String,
    pub line: usize,
    /// Which channel the site arrived through. Private: a caller reads
    /// [`Site::from_identifiers`], never the channel itself.
    kind: Kind,
}

impl Site {
    /// Whether this site is a place the tree's identifiers carry the
    /// term, rather than a doc naming it.
    pub fn from_identifiers(&self) -> bool {
        self.kind == Kind::Identifier
    }

    /// The sort rank: identifier sites first, then the doc channel's.
    fn rank(&self) -> u8 {
        match self.kind {
            Kind::Identifier => 0,
            Kind::Doc => 1,
        }
    }
}

/// One established term with its evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// The term, lowercased, parts joined by `_`.
    pub term: String,
    /// How many distinct files carry it through the identifier channel
    /// (code identifiers, doc code spans, file stems).
    pub ident_files: usize,
    /// How many distinct files carry it through the doc channel
    /// (backtick spans and fenced lines in the markdown docs).
    pub doc_files: usize,
    /// Its sites, bounded by [`MAX_SITES_PER_TERM`].
    pub sites: Vec<Site>,
    /// The term's newest carrying file's age, in days — the recency
    /// multiplier's input. `None` when no carrying file's stamp could
    /// be read, which takes the most conservative recency step.
    newest_days: Option<u64>,
}

impl Entry {
    /// Whether this term is established in the tree's identifiers
    /// rather than only in the docs' prose.
    pub fn is_identifier(&self) -> bool {
        self.ident_files > 0
    }

    /// The establishment score: the weighted file spread times the
    /// recency multiplier of the term's newest carrying file, rounded
    /// down to a whole number. See [`establishment`] for the formula.
    pub fn establishment(&self) -> u64 {
        establishment(self.ident_files, self.doc_files, self.newest_days())
    }

    /// The term's newest carrying file's age, in days. `None` when no
    /// carrying file's stamp could be read.
    pub fn newest_days(&self) -> Option<u64> {
        self.newest_days
    }

    /// The one-line form the surfaces render: `term (N files)`.
    pub fn label(&self) -> String {
        format!(
            "{} ({} files: {} identifier, {} doc)",
            self.term,
            self.ident_files + self.doc_files,
            self.ident_files,
            self.doc_files
        )
    }
}

/// The establishment formula, stated once so the number is auditable:
///
/// ```text
/// establishment = (1000 * identifier_files + doc_files) * recency(newest_days)
/// recency(d)    = 1.0  if d <= 30
///               = 0.75 if d <= 180
///               = 0.50 if d <= 365
///               = 0.25 otherwise
/// ```
///
/// The identifier channel is weighted 1000x so a term the code uses in
/// ten files (10_000) always outranks a term the docs mention in eight
/// (8) — the tree is the ground truth for what is established, the docs
/// are the tiebreaker. Recency decays it: a term last touched a year
/// ago carries half the weight of one touched this month, so a
/// vocabulary that moved on reads as having moved on. With no readable
/// mtime the multiplier is the most conservative step, 0.25, so an
/// unreadable file never inflates a term.
pub fn establishment(ident_files: usize, doc_files: usize, newest_days: Option<u64>) -> u64 {
    let spread = IDENT_WEIGHT * ident_files as u64 + doc_files as u64;
    let days = newest_days.unwrap_or(u64::MAX);
    let multiplier = RECENCY_STEPS
        .iter()
        .find(|(ceiling, _)| days <= *ceiling)
        .map(|(_, multiplier)| *multiplier)
        .unwrap_or(0.25);
    (spread as f64 * multiplier).floor() as u64
}

/// The derived vocabulary: the terms, the bounds' own report, and the
/// newest mtime the walk saw (the cache's invalidation key).
#[derive(Debug, Clone)]
pub struct Vocabulary {
    entries: Vec<Entry>,
    /// token -> indices into `entries`. A compound's parts index it, so
    /// a concept token finds both `recall_count` and `recall`.
    index: HashMap<String, Vec<usize>>,
    /// How many files the walk read.
    pub sources: usize,
    /// Whether any bound truncated the derivation.
    pub truncated: bool,
    /// The youngest age the walk saw, in days — the recency
    /// multiplier's input.
    pub newest_mtime_days: Option<u64>,
    /// The newest mtime the walk saw, in whole seconds — the freshness
    /// rule's input, at a granularity that sees a file written later in
    /// the same day.
    newest_mtime_secs: Option<u64>,
    /// How long the derivation took.
    pub elapsed: std::time::Duration,
}

impl Vocabulary {
    /// Every term, in the deterministic order the derivation produced
    /// (by establishment descending, then by term).
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    /// How many distinct terms the vocabulary carries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the vocabulary is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The entry for one exact term, if it is established. The index is
    /// keyed by the parts and by the whole term, so both `judge_route`
    /// and its part `judge` resolve.
    pub fn get(&self, term: &str) -> Option<&Entry> {
        let indices = self.index.get(term)?;
        indices
            .iter()
            .filter_map(|index| self.entries.get(*index))
            .find(|entry| entry.term == term)
    }

    /// Every entry carrying `token` — the term itself when it is
    /// established, plus the compounds that contain it. All of them, not
    /// the best one: a concept's tokens must be able to reach a
    /// compound (`judges` + `route` reaching `judge_route`) that a
    /// single token's best entry would hide behind.
    fn entries_for_token(&self, token: &str) -> Vec<&Entry> {
        let mut out: Vec<&Entry> = Vec::new();
        for variant in variants(token) {
            let Some(indices) = self.index.get(&variant) else {
                continue;
            };
            for index in indices {
                if let Some(entry) = self.entries.get(*index) {
                    if !out.iter().any(|seen| seen.term == entry.term) {
                        out.push(entry);
                    }
                }
            }
        }
        out
    }

    /// The entries a concept's tokens reach, deduplicated, best first.
    /// The ranking is two keys and both are stated here, because a
    /// lookup that cannot explain its answer is a lookup nobody
    /// trusts:
    ///
    /// 1. **compound coverage** — how many of the concept's distinct
    ///    tokens the entry's parts cover. A project's own multi-token
    ///    name for a concept outranks any single token's file count:
    ///    "the thing that judges a route" reaches `judge_route` (both
    ///    tokens) before `route` (one).
    /// 2. **establishment** — the weighted file spread times recency,
    ///    the same number the card shows. Identifier-channel entries
    ///    then the shorter term then alphabetical make the order total,
    ///    so the answer never depends on the walk.
    fn candidates(&self, tokens: &[String]) -> Vec<&Entry> {
        let mut seen: Vec<&Entry> = Vec::new();
        for token in tokens {
            for entry in self.entries_for_token(token) {
                if !seen.iter().any(|other| other.term == entry.term) {
                    seen.push(entry);
                }
            }
        }
        seen.sort_by(|left, right| {
            coverage(right, tokens)
                .cmp(&coverage(left, tokens))
                .then_with(|| establishment_key(right).cmp(&establishment_key(left)))
                .then_with(|| left.term.cmp(&right.term))
        });
        seen
    }
}

/// The establishment tuple a term ranks by: the weighted spread, then
/// the identifier channel, then the term's length as a last resort.
fn establishment_key(entry: &Entry) -> (u64, u64, usize) {
    (
        entry.establishment(),
        entry.ident_files as u64,
        usize::MAX - entry.term.len(),
    )
}

/// How many of the concept's distinct tokens an entry's parts cover,
/// with each token counted once. The compound-coverage key: a term the
/// project wrote as one identifier answers a concept better than a
/// single generic token with a longer file list.
fn coverage(entry: &Entry, tokens: &[String]) -> usize {
    let parts = entry.term.split('_').collect::<Vec<_>>();
    let mut covered = 0usize;
    for token in tokens {
        let reached = variants(token)
            .iter()
            .any(|variant| parts.iter().any(|part| *part == variant));
        if reached {
            covered += 1;
        }
    }
    covered
}

// --- the answer ------------------------------------------------------------

/// What the lookup answers. Either an established term with its
/// evidence, or the honest statement that none exists. The miss is a
/// first-class result: a concept with no established term must read as
/// "this project does not call this anything yet", never as an empty
/// list and never as a ranked list of near-synonyms.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Convention {
    /// The established term, lowercased with `_`-joined parts. `None`
    /// on an honest miss.
    pub term: Option<String>,
    /// The concept token that reached it (so the card can show why).
    pub token: String,
    /// Its sites, bounded by [`MAX_SITES_PER_TERM`].
    pub sites: Vec<Site>,
    /// Its establishment score.
    pub establishment: u64,
    /// Whether the term is established in the tree's identifiers
    /// rather than only in the docs.
    pub from_identifiers: bool,
    /// True when no established term exists for this concept.
    pub miss: bool,
}

impl Convention {
    /// The card the CLI and the TUI render: the concept's answer in
    /// plain text, with its sites and its establishment, or the honest
    /// miss stated plainly.
    pub fn card(&self) -> String {
        let mut out = String::new();
        match &self.term {
            Some(term) => {
                out.push_str(&format!("established term: {term}\n"));
                out.push_str(&format!(
                    "matched on: {} (establishment {}, {})\n",
                    self.token,
                    self.establishment,
                    if self.from_identifiers {
                        "identifiers"
                    } else {
                        "docs"
                    }
                ));
                out.push_str("sites:\n");
                for site in &self.sites {
                    out.push_str(&format!("  {}:{}\n", site.file, site.line));
                }
            }
            None => {
                out.push_str("no established term\n");
                out.push_str(&format!(
                    "concept token: {} reaches nothing in this tree's vocabulary\n",
                    self.token
                ));
            }
        }
        out
    }
}

// --- the derivation --------------------------------------------------------

/// The mutable accumulator one file's tokens land in.
struct Sink {
    per_term: HashMap<String, TermState>,
}

/// One term's in-flight accumulation.
#[derive(Default)]
struct TermState {
    ident_files: Vec<String>,
    doc_files: Vec<String>,
    sites: Vec<Site>,
    newest: Option<u64>,
}

impl Sink {
    fn new() -> Self {
        Sink {
            per_term: HashMap::new(),
        }
    }

    /// Record one term occurrence. The compound the identifier forms is
    /// recorded only through the identifier channel: a doc span naming
    /// `the_redacted_row` must not establish a compound the code never
    /// wrote, and a compound is the project's own multi-token name.
    fn push(&mut self, term: &str, site: Site, days: Option<u64>) {
        let state = self.per_term.entry(term.to_string()).or_default();
        let files = match site.kind {
            Kind::Identifier => &mut state.ident_files,
            Kind::Doc => &mut state.doc_files,
        };
        if !files.iter().any(|seen| seen == &site.file) {
            files.push(site.file.clone());
        }
        // Every site is collected; the bound and the ordering are
        // applied at fold time, where the identifier channel's sites
        // can be preferred over the doc channel's instead of losing to
        // the walk's alphabetical order.
        state.sites.push(site);
        if let Some(days) = days {
            // The newest carrying file is the youngest: with ages, the
            // newest stamp is the smallest number.
            state.newest = Some(state.newest.map_or(days, |seen: u64| seen.min(days)));
        }
    }

    /// Record every term one identifier-shaped word contributes: the
    /// compound first (identifier channel only), then the parts.
    fn push_word(&mut self, word: &str, site: Site, days: Option<u64>) {
        let parts = split_identifier(word);
        let kept: Vec<String> = parts
            .iter()
            .filter(|part| is_token(part))
            .cloned()
            .collect();
        if site.kind == Kind::Identifier && (2..=3).contains(&kept.len()) {
            self.push(&kept.join("_"), site.clone(), days);
        }
        for part in kept {
            self.push(&part.to_lowercase(), site.clone(), days);
        }
    }

    /// Fold into the derivation's final entries, ranked and bounded.
    fn finish(self) -> Vec<Entry> {
        let mut entries: Vec<Entry> = self
            .per_term
            .into_iter()
            .map(|(term, mut state)| {
                // The sites a card shows are the identifier channel's
                // first — the tree's own code is the better evidence —
                // then by file and line, and deduplicated by (file,
                // line) so a term repeated on one line is one site. The
                // bound lands after the ordering, so the bounded list
                // is the best evidence rather than the walk's first
                // dozen.
                state.sites.sort_by(|left, right| {
                    left.rank()
                        .cmp(&right.rank())
                        .then_with(|| left.file.cmp(&right.file))
                        .then_with(|| left.line.cmp(&right.line))
                });
                state
                    .sites
                    .dedup_by(|left, right| left.file == right.file && left.line == right.line);
                state.sites.truncate(MAX_SITES_PER_TERM);
                Entry {
                    term,
                    ident_files: state.ident_files.len(),
                    doc_files: state.doc_files.len(),
                    sites: state.sites,
                    newest_days: state.newest,
                }
            })
            .collect();
        entries.sort_by(|left, right| {
            establishment_key(right)
                .cmp(&establishment_key(left))
                .then_with(|| left.term.cmp(&right.term))
        });
        entries.truncate(MAX_TERMS);
        entries
    }
}

/// Which channel a token arrived through.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Identifier,
    Doc,
}

/// Derive the vocabulary from the tree at `root`. The walk is sorted
/// and the fold is a sort, so the same tree derives the same terms in
/// the same order every time; the one clock read is the recency
/// multiplier's, and it is what makes a term the tree stopped touching
/// read as stale. A missing or unreadable root is an empty
/// vocabulary, never an error: the lookup's honest miss covers it.
pub fn derive(root: &Path) -> Vocabulary {
    let started = Instant::now();
    let mut sink = Sink::new();
    let mut sources = 0usize;
    let mut truncated = false;
    let mut newest_days: Option<u64> = None;
    let mut newest_secs: Option<u64> = None;

    let mut paths = walk(root);
    for path in paths.by_ref() {
        if sources >= MAX_SOURCES
            || sink.per_term.len() >= MAX_TERMS
            || started.elapsed() > DERIVE_BUDGET
        {
            truncated = true;
            break;
        }
        sources += 1;
        let days = file_days(&path);
        if let Some(days) = days {
            newest_days = Some(newest_days.map_or(days, |seen: u64| seen.min(days)));
        }
        if let Some(secs) = file_secs(&path) {
            newest_secs = Some(newest_secs.map_or(secs, |seen: u64| seen.max(secs)));
        }
        extract(root, &path, days, &mut sink);
    }
    // A walk that still had files left when the bounds stopped it is a
    // truncated vocabulary: the walk is sorted, so what was read is a
    // deterministic prefix of the whole.
    if !truncated && paths.next().is_some() {
        truncated = true;
    }

    let entries = sink.finish();
    let mut index: HashMap<String, Vec<usize>> = HashMap::new();
    for (position, entry) in entries.iter().enumerate() {
        // The whole term first, so `get("judge_route")` resolves without
        // scanning, then each part, so a concept token reaches every
        // compound carrying it.
        index.entry(entry.term.clone()).or_default().push(position);
        for part in entry.term.split('_') {
            if part != entry.term {
                index.entry(part.to_string()).or_default().push(position);
            }
        }
    }
    Vocabulary {
        entries,
        index,
        sources,
        truncated,
        newest_mtime_days: newest_days,
        newest_mtime_secs: newest_secs,
        elapsed: started.elapsed(),
    }
}

/// The sorted file list the derivation reads: the walk dirs' code
/// files, the docs tree, and the root's markdown files. Test
/// scaffolding is excluded by name — `tests.rs`, `tests/`, and a
/// `test_`-prefixed file — because a corpus's own fixtures are not the
/// project's vocabulary, and this module's own tests name concepts
/// (`frobnicator`, `quantum`) precisely so they are absent.
fn walk(root: &Path) -> std::vec::IntoIter<PathBuf> {
    let mut paths: Vec<PathBuf> = Vec::new();
    for name in WALK_DIRS {
        collect(root.join(name), &mut paths);
    }
    collect(root.join("docs"), &mut paths);
    for name in WALK_FILES {
        let path = root.join(name);
        if path.is_file() {
            paths.push(path);
        }
    }
    paths.sort();
    paths.dedup();
    paths.retain(|path| !is_test_scaffolding(root, path));
    paths.into_iter()
}

/// Whether a file is test scaffolding: the crate's `tests/` directory,
/// any `tests.rs`, or a `test_`-prefixed file. The shipped binary's own
/// mock servers are NOT scaffolding — they are binaries the project
/// ships and names.
fn is_test_scaffolding(root: &Path, path: &Path) -> bool {
    let relative = path.strip_prefix(root).unwrap_or(path);
    let parts: Vec<&str> = relative
        .components()
        .filter_map(|component| component.as_os_str().to_str())
        .collect();
    if parts.contains(&"tests") {
        return true;
    }
    let name = parts.last().copied().unwrap_or_default();
    name == "tests.rs" || name.starts_with("test_") || name.ends_with("_test.rs")
}

/// One directory, recursively, in sorted order, skipping the build and
/// VCS directories by name.
fn collect(dir: PathBuf, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(&dir) else {
        return;
    };
    let mut children: Vec<PathBuf> = entries.flatten().map(|entry| entry.path()).collect();
    children.sort();
    for child in children {
        if child.is_dir() {
            let skip = child
                .file_name()
                .and_then(|name| name.to_str())
                .map(|name| SKIP_DIRS.contains(&name))
                .unwrap_or(false);
            if !skip {
                collect(child, out);
            }
        } else if child.is_file() {
            out.push(child);
        }
    }
}

/// One file's tokens into the sink. Markdown is doc vocabulary; code
/// extensions are identifier vocabulary; anything else is skipped. The
/// site's file is the path relative to the root, so two `tests.rs` in
/// different directories are two files and not one.
fn extract(root: &Path, path: &Path, days: Option<u64>, sink: &mut Sink) {
    let Ok(text) = fs::read_to_string(path) else {
        return;
    };
    let relative = path
        .strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .to_string();
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
        .to_string();
    let stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or_default()
        .to_string();

    if extension == "md" {
        extract_doc(&relative, &text, days, sink);
    } else if CODE_EXTENSIONS.contains(&extension.as_str()) {
        extract_code(&relative, &text, days, sink);
    }

    // The file stem establishes its own name at line 1 — `src/calib.rs`
    // makes `calib` vocabulary whether or not the prose spells it out.
    if !stem.is_empty() && stem != "mod" {
        sink.push_word(
            &stem,
            Site {
                file: relative.clone(),
                line: 1,
                kind: Kind::Identifier,
            },
            days,
        );
    }
}

/// A code file's identifier tokens. Rust comments are stripped first —
/// a comment's prose is not an identifier — and `#` comments are
/// stripped from the shell and TOML files the vendored lanes carry.
fn extract_code(file: &str, text: &str, days: Option<u64>, sink: &mut Sink) {
    let stripped = if file.ends_with(".rs") {
        strip_rust_comments(text)
    } else {
        text.lines()
            .map(|line| match line.find('#') {
                Some(at) => &line[..at],
                None => line,
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    for (number, line) in stripped.lines().enumerate() {
        for word in words(line) {
            sink.push_word(
                &word,
                Site {
                    file: file.to_string(),
                    line: number + 1,
                    kind: Kind::Identifier,
                },
                days,
            );
        }
    }
}

/// A markdown file's doc vocabulary: the inline backtick spans, the
/// path-like runs in the prose, and the fenced code lines. Everything
/// else is prose and contributes nothing — the docs name things in
/// code, and that is what the vocabulary takes. Every channel here is
/// the doc channel: a fenced Rust block inside a feature doc is
/// documentation quoting the code, not the code itself, and counting it
/// as identifier evidence would let the docs outvote the tree.
fn extract_doc(file: &str, text: &str, days: Option<u64>, sink: &mut Sink) {
    let mut fenced = false;
    for (number, line) in text.lines().enumerate() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            fenced = !fenced;
            continue;
        }
        if fenced {
            for word in words(line) {
                sink.push_word(
                    &word,
                    Site {
                        file: file.to_string(),
                        line: number + 1,
                        kind: Kind::Doc,
                    },
                    days,
                );
            }
            continue;
        }
        for span in backtick_spans(line) {
            for word in words(&span) {
                sink.push_word(
                    &word,
                    Site {
                        file: file.to_string(),
                        line: number + 1,
                        kind: Kind::Doc,
                    },
                    days,
                );
            }
        }
        // The prose's own path-like runs, with the markdown link targets
        // removed first: a link's destination is navigation, not
        // vocabulary, and `[the recap](../recaps/2026-10-14-shape.md)`
        // would otherwise establish `shape` from a file name.
        for word in path_words(&strip_link_targets(line)) {
            sink.push_word(
                &word,
                Site {
                    file: file.to_string(),
                    line: number + 1,
                    kind: Kind::Doc,
                },
                days,
            );
        }
    }
}

// --- the shared channel rule -----------------------------------------------

/// The identifier-shaped candidates one file's text carries, in order
/// of first appearance, read through the same channel rules the
/// derivation itself applies.
///
/// This is the rule the write-time guard reads by, and it is stated
/// here beside the derivation for one reason: the guard polices
/// identifiers, so it must read exactly what the vocabulary reads and
/// nothing more. Two channels, and each is the derivation's own:
///
/// - a **markdown** file (`.md`) contributes its inline backtick spans,
///   its path-like runs, and its fenced code lines, and its prose
///   contributes nothing;
/// - **every other file** contributes every identifier-shaped word
///   outside its comments — a comment's prose is not an identifier.
///
/// A word survives when at least one of its split parts is a
/// vocabulary token ([`is_token`]), which is what drops `let`, `done`,
/// and every sub-three-character fragment — the same filter the
/// derivation's own [`Sink::push_word`] applies, so a candidate the
/// guard considers is a candidate the vocabulary could have
/// established.
///
/// Markdown is the one doc channel, and it is keyed on the extension
/// the derivation itself keys it on. **Everything else is code**,
/// including a path with no extension at all: the guard's contract is to
/// police identifiers, and an identifier is an identifier whatever file
/// it lands in, so a file that is not a doc has its identifiers read
/// like code's. The residual is stated rather than hidden — prose in an
/// extensionless file is read as code, so a prose word that happens to
/// be a case variant of an established term can be refused there. That
/// is the narrower failure, and KI-219 is the wider one: the refusal
/// that actually blocked work was markdown prose, which the `.md`
/// extension now excludes.
pub fn channel_tokens(path: &str, content: &str) -> Vec<String> {
    let extension = Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default();
    if extension != "md" {
        // The derivation keys the Rust branch on the path's suffix
        // rather than the extension, and this mirrors it exactly.
        let stripped = if path.ends_with(".rs") {
            strip_rust_comments(content)
        } else {
            content
                .lines()
                .map(|line| match line.find('#') {
                    Some(at) => &line[..at],
                    None => line,
                })
                .collect::<Vec<_>>()
                .join("\n")
        };
        return kept(stripped.lines().flat_map(words).collect());
    }
    let mut out: Vec<String> = Vec::new();
    let mut fenced = false;
    for line in content.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            fenced = !fenced;
            continue;
        }
        if fenced {
            out.extend(kept(words(line)));
            continue;
        }
        for span in backtick_spans(line) {
            out.extend(kept(words(&span)));
        }
        for word in path_words(&strip_link_targets(line)) {
            out.extend(kept(vec![word]));
        }
    }
    out
}

/// The words carrying at least one vocabulary token, in order.
fn kept(words: Vec<String>) -> Vec<String> {
    words
        .into_iter()
        .filter(|word| split_identifier(word).iter().any(|part| is_token(part)))
        .collect()
}

/// Markdown link and image targets removed, the link text kept. A
/// doc's `[label](path)` becomes `[label]`, so the path never reaches
/// the vocabulary as a doc token — but the backtick pass, which runs
/// first on the raw line, still takes a backticked path verbatim.
fn strip_link_targets(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut rest = line;
    while let Some(open) = rest.find("](") {
        out.push_str(&rest[..open + 1]);
        let after = &rest[open + 2..];
        let close = after.find(')').unwrap_or(after.len());
        rest = &after[close..];
    }
    out.push_str(rest);
    out
}

/// Rust comments stripped, string literals left alone: `//` and `/* */`
/// to end of comment, never crossing a newline a string opened.
fn strip_rust_comments(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while !rest.is_empty() {
        let slash = rest.find('/');
        let (plain, tail) = match slash {
            Some(at) => (&rest[..at], &rest[at..]),
            None => (rest, ""),
        };
        out.push_str(plain);
        if let Some(after) = tail.strip_prefix("//") {
            let end = after.find('\n').unwrap_or(after.len());
            rest = &after[end..];
        } else if let Some(after) = tail.strip_prefix("/*") {
            let end = after.find("*/").map(|at| at + 2).unwrap_or(after.len());
            rest = &after[end..];
        } else if tail.is_empty() {
            break;
        } else {
            // A lone slash: one character, and the scan continues.
            let mut chars = tail.chars();
            let first = chars.next().unwrap_or('/');
            out.push(first);
            rest = chars.as_str();
        }
    }
    out
}

// --- the tokenizer ---------------------------------------------------------

/// The identifier-shaped words on one line, in order. A word starts
/// with a letter or `_` and continues alphanumeric or `_`; a `-`
/// between two word characters separates two words, so `write-time`
/// yields `write` and `time`, while `--flag` yields `flag`.
fn words(line: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut current = String::new();
    for character in line.chars() {
        if character.is_alphanumeric() || character == '_' {
            current.push(character);
        } else if !current.is_empty() {
            out.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        out.push(current);
    }
    out
}

/// The path-like runs on one prose line: a maximal run of non-space
/// characters containing a `/`, with backtick spans and common
/// punctuation trimmed. `src/calib.rs` yields `src/calib.rs`;
/// `` `fleet calib` `` was already taken by the backtick pass.
fn path_words(line: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut in_ticks = false;
    for raw in line.split('`') {
        in_ticks = !in_ticks;
        if in_ticks {
            continue;
        }
        for run in raw.split_whitespace() {
            let trimmed = run.trim_matches(|c: char| {
                c.is_whitespace()
                    || matches!(
                        c,
                        '(' | ')' | '[' | ']' | ',' | ';' | ':' | '.' | '"' | '\''
                    )
            });
            if trimmed.contains('/') && trimmed.len() > 1 {
                out.push(trimmed.to_string());
            }
        }
    }
    out
}

/// The inline backtick spans on one prose line, in order. A line with
/// an odd number of backticks yields the pairs it has; the trailing
/// one opens nothing.
fn backtick_spans(line: &str) -> Vec<String> {
    let mut spans: Vec<String> = Vec::new();
    let mut rest = line;
    while let Some(open) = rest.find('`') {
        rest = &rest[open + 1..];
        let Some(close) = rest.find('`') else { break };
        spans.push(rest[..close].to_string());
        rest = &rest[close + 1..];
    }
    spans
}

/// An identifier split on `_`, `-`, and camelCase boundaries, in order.
/// `recall_count` -> recall, count; `recallCount` -> recall, count;
/// `HTTPServer` -> http, server.
///
/// Public because the write-time guard's lookup key is this split, and
/// the one place it is stated is here: a guard that carried its own
/// copy would be one refactor away from looking up a spelling the
/// vocabulary never stored.
pub fn split_identifier(identifier: &str) -> Vec<String> {
    let mut parts: Vec<String> = Vec::new();
    for segment in identifier.split(['_', '-']) {
        if segment.is_empty() {
            continue;
        }
        let chars: Vec<char> = segment.chars().collect();
        let mut current = String::new();
        for (index, character) in chars.iter().enumerate() {
            let previous = if index == 0 {
                None
            } else {
                Some(chars[index - 1])
            };
            let next = chars.get(index + 1).copied();
            let boundary = match (previous, character.is_uppercase()) {
                (Some(previous), true) => {
                    previous.is_lowercase()
                        || previous.is_ascii_digit()
                        || (next.map(|n| n.is_lowercase()).unwrap_or(false)
                            && previous.is_uppercase())
                }
                _ => false,
            };
            if boundary && !current.is_empty() {
                parts.push(std::mem::take(&mut current));
            }
            current.push(*character);
        }
        if !current.is_empty() {
            parts.push(current);
        }
    }
    parts
}

/// Whether a split part is a vocabulary token: at least three
/// characters, and neither an English function word nor a language
/// keyword the project happens to type.
fn is_token(part: &str) -> bool {
    let lowered = part.to_lowercase();
    lowered.chars().count() >= 3
        && !lowered.chars().all(|c| c.is_ascii_digit())
        && !STOPWORDS.contains(&lowered.as_str())
}

/// The forms a concept token is looked up under: itself, its singular
/// when it ends in a plural `s`, and its plural. Light on purpose — a
/// stemmer that folded `judgement` to `judge` would also fold
/// `direction` to `direct`, and a lookup that guesses is the drift
/// wearing a different hat.
fn variants(token: &str) -> Vec<String> {
    let mut out = vec![token.to_string()];
    if token.len() > 4 && token.ends_with('s') && !token.ends_with("ss") {
        out.push(token[..token.len() - 1].to_string());
    } else if token.len() > 4 && token.ends_with("ies") {
        out.push(format!("{}y", &token[..token.len() - 3]));
    } else {
        out.push(format!("{token}s"));
    }
    out
}

/// A file's age in days — now minus its mtime — the recency
/// multiplier's input. A file stamped in the future (clock skew)
/// reads as zero days old rather than wrapping.
fn file_days(path: &Path) -> Option<u64> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs();
    Some(now.saturating_sub(file_secs(path)?) / 86_400)
}

/// A file's mtime in whole seconds since the epoch.
fn file_secs(path: &Path) -> Option<u64> {
    let modified = fs::metadata(path).ok()?.modified().ok()?;
    let duration = modified.duration_since(UNIX_EPOCH).ok()?;
    Some(duration.as_secs())
}

// --- the lookup ------------------------------------------------------------

/// What does this project already call `concept`? Tokenizes the
/// concept, matches the tokens against the vocabulary exactly (with
/// the light plural normalization [`variants`] applies), and returns
/// the best-established term with its file:line sites. When nothing
/// matches, the answer is an honest miss — never an empty list, never
/// a ranked list of near-synonyms.
pub fn lookup(root: &Path, concept: &str) -> Convention {
    let vocabulary = vocabulary(root);
    lookup_in(&vocabulary, concept)
}

/// The lookup against an already-derived vocabulary — the shape the
/// cached path and the tests both use.
pub fn lookup_in(vocabulary: &Vocabulary, concept: &str) -> Convention {
    let tokens = concept_tokens(concept);
    let Some(best) = vocabulary.candidates(&tokens).into_iter().next() else {
        return Convention {
            term: None,
            token: tokens.first().cloned().unwrap_or_default(),
            sites: Vec::new(),
            establishment: 0,
            from_identifiers: false,
            miss: true,
        };
    };
    let token = tokens
        .iter()
        .find(|token| {
            variants(token).iter().any(|variant| {
                best.term.split('_').any(|part| part == variant) || *variant == best.term
            })
        })
        .cloned()
        .unwrap_or_else(|| tokens[0].clone());
    Convention {
        term: Some(best.term.clone()),
        token,
        sites: best.sites.clone(),
        establishment: best.establishment(),
        from_identifiers: best.is_identifier(),
        miss: false,
    }
}

/// A concept's vocabulary tokens: lowercased, split on every
/// non-alphanumeric run, with the function words and the sub-3-character
/// fragments dropped. "the thing that judges a route" yields thing,
/// judges, route.
fn concept_tokens(concept: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for token in concept
        .to_lowercase()
        .split(|character: char| !character.is_alphanumeric())
        .filter(|token| !token.is_empty())
    {
        if token.chars().count() >= 3 && !STOPWORDS.contains(&token) {
            out.push(token.to_string());
        }
    }
    out
}

// --- the guard -------------------------------------------------------------

/// Whether `candidate` differs from `established` only by case or
/// separator. True for `judgeRoute`/`judge_route`,
/// `Judge-Route`/`judge_route`, `judge_route`/`JudgeRoute`; false for
/// `judgement`/`judge` (a different word, however near) and for
/// `route_judge`/`judge_route` (the same words in a different order —
/// that is [`check`]'s flag case, not the refusal). This is the case
/// the write-time guard refuses rather than flags, because a case or
/// separator variant of an established term is drift with no other
/// explanation.
pub fn is_near_exact(candidate: &str, established: &str) -> bool {
    let left = split_identifier(candidate);
    let right = split_identifier(established);
    if left.is_empty() || right.is_empty() || left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right.iter())
        .all(|(left, right)| left.to_lowercase() == right.to_lowercase())
}

/// Given a new identifier, the canonical near-neighbour when one
/// exists — what the write-time guard flags, with the candidate named
/// — or `None` for a genuinely new concept. The near-neighbour is
/// established on two relations and nothing weaker:
///
/// - **an anagram** — the same tokens in a different order
///   (`route_judge` beside `judge_route`);
/// - **a near-synonym inside one token** — `judgement` beside
///   `judgment`, `calibs` beside `calib` (up to two edits for a single
///   token, one inside a compound).
///
/// Deliberately NOT a relation: containing an established token
/// (`judge_route` beside `judge`), which is ordinary qualification and
/// the reason the guard does not cry wolf on every new compound.
/// `None` also covers the near-exact case — that one is the caller's
/// refusal ([`is_near_exact`]), not its flag.
pub fn check(root: &Path, candidate: &str) -> Option<Convention> {
    let vocabulary = vocabulary(root);
    check_in(&vocabulary, candidate)
}

/// The established term `candidate` is a case or separator variant of,
/// when one exists — the state the write-time guard refuses, answered
/// here for the lookup that runs before the write.
///
/// The scan is the module's own because the vocabulary's entries are
/// its private shape: a caller that could walk them directly would be
/// one refactor away from reimplementing the establishment ordering.
/// The first match in establishment order wins, so the answer names the
/// term the project leans on hardest.
pub fn near_exact_term<'a>(vocabulary: &'a Vocabulary, candidate: &str) -> Option<&'a Entry> {
    let mut found: Vec<&Entry> = vocabulary
        .entries
        .iter()
        .filter(|entry| is_near_exact(candidate, &entry.term))
        .collect();
    found.sort_by(|left, right| {
        establishment_key(right)
            .cmp(&establishment_key(left))
            .then_with(|| left.term.cmp(&right.term))
    });
    found.into_iter().next()
}

/// The candidate check against an already-derived vocabulary.
pub fn check_in(vocabulary: &Vocabulary, candidate: &str) -> Option<Convention> {
    let parts = split_identifier(candidate);
    if parts.is_empty() {
        return None;
    }
    let normalized = parts
        .iter()
        .map(|part| part.to_lowercase())
        .collect::<Vec<_>>();

    // The candidate itself, established: nothing to flag. Exact match
    // only — a plural variant of an established term (`judges` beside
    // `judge`) is exactly the drift the flag exists to name, so the
    // variant expansion stops at the lookup and starts here.
    let compound = normalized.join("_");
    if vocabulary.get(&compound).is_some() {
        return None;
    }

    let mut neighbours: Vec<&Entry> = Vec::new();
    for entry in &vocabulary.entries {
        if entry.term == compound {
            continue;
        }
        let near = is_near_neighbour(&normalized, entry);
        if std::env::var("NOMEN_DEBUG").is_ok() {
            eprintln!("SCAN {:?} vs {:?} -> {}", normalized, entry.term, near);
        }
        if near {
            neighbours.push(entry);
        }
    }
    neighbours.sort_by(|left, right| {
        establishment_key(right)
            .cmp(&establishment_key(left))
            .then_with(|| left.term.cmp(&right.term))
    });
    let best = neighbours.first()?;
    Some(Convention {
        term: Some(best.term.clone()),
        token: compound,
        sites: best.sites.clone(),
        establishment: best.establishment(),
        from_identifiers: best.is_identifier(),
        miss: false,
    })
}

/// Whether an established term is a near-neighbour of a candidate's
/// token sequence. Two relations and nothing weaker:
///
/// - **an anagram** — the same tokens in a different order
///   (`route_judge` beside `route_judged`, `verdict_judge` beside
///   `judge_verdict`);
/// - **one token one edit away** — the rest of the tokens exactly in
///   place (`judge_route` beside `route_judged`, where `judge` and
///   `judged` differ by one character).
///
/// A single-token candidate may sit up to two edits away, because a
/// one-word near-synonym (`judgement` for `judgment`) is exactly the
/// drift the guard exists to name. Containing an established token
/// (`judge_route` beside `judge`) is deliberately NOT a relation: that
/// is ordinary qualification, and a guard that flags it cries wolf.
fn is_near_neighbour(candidate: &[String], entry: &Entry) -> bool {
    let established = entry.term.split('_').collect::<Vec<_>>();
    if std::env::var("NOMEN_DEBUG").is_ok() {
        eprintln!(
            "  NEAR? cand={:?}({}) est={:?}({})",
            candidate,
            candidate.len(),
            established,
            established.len()
        );
    }
    if established.len() != candidate.len() || candidate.is_empty() {
        return false;
    }
    if established.len() == 1 {
        // A two-edit near-synonym needs a four-character shared prefix
        // (`judgement`/`judgment` share `judg`); a one-edit slip needs
        // three (`calibs`/`calib`). Without the prefix the distance
        // alone matches `calibs` to `calls`, and a guard that flags
        // unrelated words gets ignored.
        let distance = edit_distance(&candidate[0], established[0]);
        let prefix = shared_prefix(&candidate[0], established[0]);
        return (distance <= 1 && prefix >= 3) || (distance == 2 && prefix >= 4);
    }
    // An anagram: the same multiset of tokens in a different order.
    let mut sorted = candidate.to_vec();
    let mut established_sorted = established
        .iter()
        .map(|part| part.to_string())
        .collect::<Vec<_>>();
    sorted.sort();
    established_sorted.sort();
    if sorted == established_sorted {
        // Same tokens, and not in the established order: the drift.
        return candidate != established;
    }
    // One token one edit away, the rest exactly in place.
    let differing = established
        .iter()
        .zip(candidate.iter())
        .filter(|(left, right)| left != right)
        .count();
    if differing != 1 {
        return false;
    }
    established
        .iter()
        .zip(candidate.iter())
        .find(|(left, right)| left != right)
        .map(|(left, right)| edit_distance(right, left) <= 1)
        .unwrap_or(false)
}

/// The number of leading characters two words share. The near-synonym
/// relation is prefix-guarded: `judgement`/`judgment` share `judg` and
/// are one relation, while `calibs`/`calls` share only `cal` and are
/// two different words that happen to sit two edits apart.
fn shared_prefix(left: &str, right: &str) -> usize {
    left.chars()
        .zip(right.chars())
        .take_while(|(left, right)| left == right)
        .count()
}

/// Levenshtein distance with the two-row table and an early exit once a
/// row's best is above 2: the answer cannot come back under it.
fn edit_distance(left: &str, right: &str) -> usize {
    let left: Vec<char> = left.chars().collect();
    let right: Vec<char> = right.chars().collect();
    let (left, right) = if left.len() <= right.len() {
        (left, right)
    } else {
        (right, left)
    };
    if right.len() - left.len() > 2 {
        return 3;
    }
    let mut previous: Vec<usize> = (0..=left.len()).collect();
    let mut current = vec![0usize; left.len() + 1];
    for row in 1..=right.len() {
        current[0] = row;
        for column in 1..=left.len() {
            let cost = if left[column - 1] == right[row - 1] {
                0
            } else {
                1
            };
            current[column] = (previous[column] + 1)
                .min(current[column - 1] + 1)
                .min(previous[column - 1] + cost);
        }
        std::mem::swap(&mut previous, &mut current);
    }
    previous[left.len()]
}

// --- the cache -------------------------------------------------------------

/// The process's derived vocabularies, one per root, with the newest
/// mtime each was derived from.
static CACHE: OnceLock<Mutex<HashMap<PathBuf, Cached>>> = OnceLock::new();

/// One cached derivation and the tree state it was derived from.
struct Cached {
    vocabulary: Vocabulary,
}

/// The vocabulary for `root`, derived once per process per root. The
/// derivation is the expensive part (about a second on this tree), and
/// every tool call and every write-time guard check asks the same
/// question, so the answer is cached: the same root returns the same
/// vocabulary for the process's life. [`fresh`] is the invalidation
/// rule for a caller that outlives a tree change.
pub fn vocabulary(root: &Path) -> Vocabulary {
    let key = root.to_path_buf();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let mut guard = cache
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    guard
        .entry(key)
        .or_insert_with(|| Cached {
            vocabulary: derive(root),
        })
        .vocabulary
        .clone()
}

/// Whether the cached vocabulary for `root` still describes the tree:
/// true when no file the walk would read is newer than the newest
/// mtime the derivation saw. A caller holding a long-lived cache
/// compares this before trusting it; a stale vocabulary is not wrong
/// so much as silent, which is the failure mode the guard cannot
/// afford.
pub fn fresh(root: &Path) -> bool {
    let key = root.to_path_buf();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let guard = cache
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let Some(cached) = guard.get(&key) else {
        return false;
    };
    let Some(newest) = cached.vocabulary.newest_mtime_secs else {
        return true;
    };
    let mut paths = walk(root);
    paths.all(|path| file_secs(&path).is_none_or(|secs| secs <= newest))
}

#[cfg(test)]
mod tests;
