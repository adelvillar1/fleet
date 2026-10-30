//! Memory: the git-tracked fact substrate, hybrid retrieval with a
//! probed vector path, shape-first recall (the shape partitions, the
//! words corroborate), the failure axis with its three signs, the
//! resolution's phrases, recall-hit feedback, and the protected tier
//! with budget-aware turn-context assembly.
//!
//! **The substrate.** One markdown file per durable fact under
//! `memories/`, frontmatter carrying `name`, `description`, `type`,
//! `date`, `basis`, `shape`, `phrases`, `recall-count`,
//! `superseded-by`, and `valid-until`, with the file's body as the
//! fact's content. The fork's
//! `memories/` tree (234 facts in exactly this shape) is the spec, and
//! its `@fleet/dsh-brain` the design. Facts persist across sessions
//! because they are files; a bad one reverts cleanly because they are
//! git-tracked — no code path in this module rewrites history. The one
//! in-place rewrite is the recall-count bump, and it is bounded to that
//! one frontmatter line.
//!
//! **The hybrid ranker.** A fact's score is a weighted sum of named
//! channels — no single one dominating — with the weights named
//! constants below. The vector backend is **probed, not assumed**: the
//! loopback llama-server sem1 serves (`127.0.0.1:8901`, or wherever
//! `FLEET_EMBEDDING_ENDPOINT` points) is discovered with one real
//! embedding round trip, and its absence degrades recall
//! to keyword-only — the correct degradation, named by `doctor` and by
//! this module's own surface, never silent. That is the fix for the live
//! system, where an absent embedding stack silently degraded recall over
//! 11k entries.
//!
//! **Shape-first recall.** The owner's directive, stated twice and
//! sharpened both times: *problem shape must drive semantic search* —
//! "a keyword or phrase in the wrong context is meaningless, while a
//! problem shape tied to the word phrases that were recorded during the
//! resolution are key information." Recall runs in two movements. The
//! instruction classifies into a shape — a structure shape from the
//! fork's Wave D keyword taxonomy, or a failure shape from Fleet's own
//! failure record — and that shape **partitions** the pool: the facts
//! carrying it are the primary pool, every other fact is spillover.
//! Within the primary pool the facts rank as multidimensional vectors:
//! the topical channels (vector similarity, keyword coverage), the
//! trajectory channel (the phrases the resolution recorded — where the
//! fact drove), the history channel (recall-count, earned by use), the
//! freshness channel (exponential decay), and the provenance channel
//! (basis). The shape is **structural**: it selects the pool and
//! carries no additive term of its own — the old `SHAPE_BOOST`, a 0.15
//! tiebreaker under 0.80 of word weight, is gone, because it let a
//! homonym across shapes outrank the right shape's own answer. An
//! unclassifiable instruction degrades to the word-ranked pool as the
//! primary pool: the classification's failure must never starve recall,
//! it only fails to reorder it.
//!
//! **Direction outranks proximity.** The semantic channels (vector,
//! keyword) measure how NEAR a fact sits to the query; the direction
//! channels (trajectory, history, provenance) measure which way its
//! vector points. The owner's ruling governs the weighting: a fact with
//! maximum semantic density and no direction must not outrank a fact
//! pointing at the minima. So the proximity pair is 0.30 of the
//! composite and the direction channels are 0.60, with margin — a fact
//! whose whole case is the neighborhood tops out at 0.45, and a
//! direction-strong fact with weak words scores 0.545 and wins.
//!
//! **The spillover.** Off-shape facts enter the assembly only through
//! one of three named routes — the primary pool is empty, its best
//! score is below [`SPILLOVER_FLOOR`], or the budget has room after the
//! pool is in — and every off-shape fact that enters is MARKED
//! `off-shape` in its line, so the model knows its context is weaker.
//! Never silently mixed into the shape's answers.
//!
//! **The three signs.** A fact is a vector with a sign. A **resolution**
//! drives toward the minima — the approaches that fit the shape, the
//! prevention half of the shelf. A **failure** is an anti-vector
//! driving the opposite way — what kind of thing broke, the avoidance
//! half. The sign is derived, not stored: a fact carrying any
//! failure-shape tag is an anti-vector, every other fact is a
//! resolution. An anti-vector that matches the current trajectory is
//! highly relevant and surfaces as prominently as a resolution — it is
//! not a penalty and not a lower rank — and the assembly line names the
//! sign (`resolved:` / `diverged:`) so the model never reads an
//! anti-vector as a candidate solution.
//!
//! **The third sign.** An **incomplete** solution drove the right
//! direction and never reached the minima, because the companions it
//! needed were absent. It is the vector most mistaken for done: it is
//! not wrong (its direction was correct, so it is never skipped or
//! penalized like a failure) and it is not finished. It ranks exactly
//! as a resolution — same pool, same weights, no penalty — and it is
//! surfaced with its missing companions named, on the `incomplete:`
//! marker, so a later session can finish the trajectory instead of
//! re-deriving it or reading it as settled. The `missing` frontmatter
//! field is where those companions ride; it is deliberately absent
//! from the indexed text, because an absent companion is not the
//! fact's direction.
//!
//! **The phrases.** The write path records the phrases that resolved
//! the problem — the vocabulary that worked — and, for a failure, the
//! phrases of the trajectory that was tried and diverged. They ride the
//! fact's indexed text beside the description and the shape tags (what
//! the vector channel embeds) and they are the trajectory channel's own
//! text: the fact's direction, scored on its own channel rather than
//! riding inside the topical words.
//!
//! **The protected tier and assembly.** Facts whose `type` marks
//! identity or standing preferences (`user`, `feedback` in the fork's
//! taxonomy) are protected: the injection budget can never evict them.
//! Turn context is *assembled* — protected tier first, then shape-first
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
//!
//! **Part 2, landed.** [`Remember`] is the write path: a
//! `brain_remember`-shaped structured request with `basis` required on
//! every write and `valid-until` required for the session-scoped tier,
//! rendered in the fork's field order through an atomic temp-and-rename
//! write. Three tiers ride the `tier` frontmatter field — `working`
//! (session-scoped, expires on its `valid-until`), `durable`
//! (basis-recorded), `episodic` (consolidated summaries, written by the
//! cycle only). [`Memory::supersede`] is the modify/forget path and
//! doubles as the permission model: a `confirmed-by` fact is refused
//! without approval, an `inferred` one is supersedable on a recorded
//! justification. [`consolidate`] is the scheduled cycle — compress old
//! working memory into an episodic summary, supersede stale inferred
//! facts, forget expired ones by archiving them — with every action
//! logged as a `consolidate` calibration row; [`spawn_consolidator`]
//! runs it on its timer in the long-lived processes, and
//! `fleet memories --consolidate` runs one pass on demand. The turn path
//! calls [`Memory::assemble`] with [`Memory::working`] and records the
//! hits the assembly surfaced through [`Memory::record_hits`] — after
//! the first successful model response, so a hit means the model saw
//! the fact (KI-068).

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

/// The fork's documented synonyms for the two protected types, mapped
/// onto them. KI-059: the fork documents `type` as free-form and its
/// own corpus writes `preference` and `identity` where Fleet's two
/// protected types would not match, so a fact the fork meant to
/// protect rides the ordinary tiers here. A documented synonym set is
/// the loader's mapping; a writer-side enum waits for the owner.
pub const PROTECTED_SYNONYMS: [(&str, &str); 4] = [
    ("identity", "user"),
    ("owner", "user"),
    ("preference", "feedback"),
    ("standing-preference", "feedback"),
];

/// How many ancestors above the working directory are searched for the
/// `memories/` directory. Covers any real clone depth.
const ANCESTOR_LIMIT: usize = 12;

/// Where a remembered fact came from: observed, or reasoned. Provenance
/// is the permission model part 2's guardrail reads — an `inferred` fact
/// is supersedable on the loop's own justification, a `confirmed-by` one
/// requires approval — and since slice 19 it is also a rank signal
/// (`W_PROVENANCE`): the magnitude of a fact's evidence used to be
/// invisible to the ranking, and now it is one of the named channels.
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

/// How long a fact lives: the three speeds of the semantic layer.
///
/// - `Working` is session-scoped: it expires on its `valid-until`, the
///   consolidation cycle compresses it into an episodic summary, and it
///   enters a turn through the working tier rather than the ranking.
/// - `Durable` is basis-recorded and does not expire; it is the tier the
///   corpus's own facts are, and the default when the frontmatter
///   carries no `tier` at all.
/// - `Episodic` is a consolidated summary, written by the cycle and
///   never by the write path; it is durable-shaped and recallable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tier {
    Working,
    Durable,
    Episodic,
}

impl Tier {
    /// The word the frontmatter carries.
    pub fn label(self) -> &'static str {
        match self {
            Tier::Working => "working",
            Tier::Durable => "durable",
            Tier::Episodic => "episodic",
        }
    }

    /// Parse the vocabulary, or `None` for anything else — a tier
    /// outside the three words is a malformed fact, the basis rule.
    fn parse(text: &str) -> Option<Tier> {
        match text.trim() {
            "working" => Some(Tier::Working),
            "durable" => Some(Tier::Durable),
            "episodic" => Some(Tier::Episodic),
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
    /// How long this fact lives: the three speeds. Absent from the
    /// frontmatter reads as [`Tier::Durable`], so the corpus's own
    /// pre-tier facts load unchanged.
    pub tier: Tier,
    /// Structure tags from the fork's shape vocabulary, and — since
    /// slice 19 — the failure shapes from Fleet's own failure record. A
    /// fact may carry both axes; either match puts it in a recall's
    /// primary pool.
    pub shape: Vec<String>,
    /// The phrases the resolution recorded: the vocabulary that drove
    /// toward the answer, and — for a failure — the vocabulary of the
    /// trajectory that was tried and diverged. The fact's direction:
    /// the trajectory channel's own text, and part of what the vector
    /// channel embeds.
    pub phrases: Vec<String>,
    /// The companions the trajectory needed and did not have: the
    /// absent pieces that kept an otherwise-correct direction from
    /// reaching the minima. Slice 19's third vector. Deliberately
    /// EXCLUDED from [`Fact::indexed_text`] — an absent companion is
    /// not the fact's direction, and embedding it would score a fact
    /// for what it does not have.
    pub missing: Vec<String>,
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
        let canonical = PROTECTED_SYNONYMS
            .iter()
            .find(|(synonym, _)| *synonym == kind)
            .map(|(_, protected)| *protected)
            .unwrap_or(kind.as_str());
        PROTECTED_TYPES.contains(&canonical)
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

    /// The fact's sign, derived from its tags — one vocabulary, so no
    /// field can contradict the tags. The failure shape says WHAT KIND
    /// of thing broke; the outcome tags say WHAT BECAME of the
    /// trajectory. Three signs, and the precedence is the order that
    /// keeps an anti-vector from being read as a candidate:
    ///
    /// - any failure tag ⇒ [`Sign::Divergence`] — unless an outcome tag
    ///   overrides it, because a failure shape is the problem's shape,
    ///   not a claim about how the trajectory ended;
    /// - `incomplete` ⇒ [`Sign::Incomplete`] — the direction was right
    ///   and the companions were absent, so it drove partway and
    ///   stalled. It wins over a failure tag: a stalled trajectory is
    ///   not an anti-vector;
    /// - `resolved` ⇒ [`Sign::Resolution`] — the failure shape's fix
    ///   landed, so the fact is a candidate even inside a
    ///   failure-shape pool;
    /// - nothing else ⇒ [`Sign::Resolution`].
    pub fn sign(&self) -> Sign {
        let carries = |tag: &str| self.shape.iter().any(|seen| seen == tag);
        if self.shape.iter().any(|tag| is_failure_shape(tag)) {
            if carries(INCOMPLETE_TAG) {
                Sign::Incomplete
            } else if carries(RESOLVED_TAG) {
                Sign::Resolution
            } else {
                Sign::Divergence
            }
        } else if carries(INCOMPLETE_TAG) {
            Sign::Incomplete
        } else {
            Sign::Resolution
        }
    }

    /// The fact's topical text: the description with the shape tags
    /// riding along, the way the fork's importer indexes them. This is
    /// what the topical channels score — the neighborhood the fact
    /// talks about.
    pub fn topic_text(&self) -> String {
        if self.shape.is_empty() {
            self.description.clone()
        } else {
            format!("{} {}", self.description, self.shape.join(" "))
        }
    }

    /// The text the index embeds: the topic text with the resolution's
    /// phrases riding along — what the fact is about AND where it
    /// drove. The trajectory channel scores the phrases on their own;
    /// the topical channels score the topic text, so the two
    /// dimensions cannot double-count each other. The **missing**
    /// companions are deliberately NOT here: an absent companion is not
    /// the fact's direction, and embedding it would score a fact for
    /// what it does not have.
    pub fn indexed_text(&self) -> String {
        let topic = self.topic_text();
        if self.phrases.is_empty() {
            topic
        } else {
            format!("{} {}", topic, self.phrases.join(" "))
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
///
/// The bracketed-list reader serves three fields since slice 19:
/// `shape` (lowercased tags, either axis), `phrases` (the
/// resolution's recorded vocabulary, case-preserving), and `missing`
/// (the third vector's absent companions, case-preserving).
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
            return Err(format!(
                "frontmatter line {} is not a `key: value` pair",
                index + 1
            ));
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
    let Some(value) = value else {
        return Vec::new();
    };
    let Some(inner) = value
        .trim()
        .strip_prefix('[')
        .and_then(|v| v.strip_suffix(']'))
    else {
        return Vec::new();
    };
    inner
        .split(',')
        .map(|tag| unquote(tag.trim()).trim().to_ascii_lowercase())
        .filter(|tag| !tag.is_empty())
        .collect()
}

/// The `phrases` field reads as a bracketed list too —
/// `[read CLAUDE.local.md directly, never grep-extract credentials]` —
/// Fleet's own field, written by the write path since slice 19. Unlike
/// the shape tags the phrases are **case-preserving**: they are the
/// vocabulary that worked, quoted as it was used, and lowercasing them
/// would corrupt the direction they record. Each is trimmed and
/// unquoted, empties dropped, the count capped at [`MAX_PHRASES`], and
/// each bounded at [`MAX_PHRASE_CHARS`] with a marked cut so a
/// hand-written file cannot inflate the index. A bare scalar reads as
/// no phrases, the shape field's own rule.
fn phrase_list(value: Option<&str>) -> Vec<String> {
    let Some(value) = value else {
        return Vec::new();
    };
    let Some(inner) = value
        .trim()
        .strip_prefix('[')
        .and_then(|v| v.strip_suffix(']'))
    else {
        return Vec::new();
    };
    inner
        .split(',')
        .map(|phrase| sanitize_line(unquote(phrase.trim()), MAX_PHRASE_CHARS))
        .filter(|phrase| !phrase.is_empty())
        .take(MAX_PHRASES)
        .collect()
}

/// The `missing` field reads as a bracketed list too — the third
/// vector's companions, `[the torch stack, the CUDA driver]` — Fleet's
/// own field, written by the write path since slice 19. Like the
/// phrases it is **case-preserving**: a companion is a name as it was
/// written, and lowercasing it would corrupt the very thing a later
/// session needs to go find. Each is trimmed and unquoted, empties
/// dropped, the count capped at [`MAX_PHRASES`], and each bounded at
/// [`MAX_PHRASE_CHARS`] with a marked cut. A bare scalar reads as no
/// missing companions, the shape field's own rule.
fn missing_list(value: Option<&str>) -> Vec<String> {
    let Some(value) = value else {
        return Vec::new();
    };
    let Some(inner) = value
        .trim()
        .strip_prefix('[')
        .and_then(|v| v.strip_suffix(']'))
    else {
        return Vec::new();
    };
    inner
        .split(',')
        .map(|entry| sanitize_line(unquote(entry.trim()), MAX_MISSING_CHARS))
        .filter(|entry| !entry.is_empty())
        .take(MAX_MISSING)
        .collect()
}

/// One log line naming the fact that was skipped and why. A broken fact
/// is never silent and never fatal — the skills precedent.
fn skip_line(name: &str, reason: &str) {
    eprintln!("fleet: memories: skipping `{name}` \u{2014} {reason}");
}

/// The memory rules on top of the shared parse: `name` and `description`
/// are required, the frontmatter's `name` must equal the file's stem
/// (one fact, one name — the name is the index and calibration key)
/// while `stem_is_the_name`, a `basis` outside the vocabulary is
/// refused, a `tier` outside the three speeds is refused, a
/// `recall-count` that is not a non-negative integer is refused, and an
/// absent `basis` defaults to `inferred` — a fact the session merely
/// produced is a guess until someone confirms it. An absent `tier`
/// defaults to `durable`: the corpus's own facts were written before
/// the tier field existed.
///
/// `stem_is_the_name` is what every live-corpus read enforces and the
/// archive's read relaxes: an archived collision copy is named
/// `<name>.<serial>.md` on disk while its frontmatter still carries the
/// fact's own name.
fn fact(
    document: Document,
    stem: &str,
    path: PathBuf,
    stamp: Option<String>,
    stem_is_the_name: bool,
) -> Result<Fact, String> {
    let name = document.required("name")?;
    if stem_is_the_name && name != stem {
        return Err(format!(
            "frontmatter name `{name}` does not match the file `{stem}.md`"
        ));
    }
    // KI-061: the writer's bound, applied at load. The corpus is
    // hand-editable by design, so a description edited by hand rides
    // whatever it was given — the loader applies the same sanitize and
    // cut the writer applies, with the cut marked, rather than trusting
    // every writer to have been the harness's own.
    let description = sanitize_line(&document.required("description")?, MAX_DESCRIPTION_CHARS);
    let basis = match document.get("basis") {
        Some(text) => Basis::parse(text)
            .ok_or_else(|| format!("unknown basis `{text}` (expected confirmed-by or inferred)"))?,
        None => Basis::Inferred,
    };
    let tier = match document.get("tier") {
        Some(text) => Tier::parse(text).ok_or_else(|| {
            format!("unknown tier `{text}` (expected working, durable, or episodic)")
        })?,
        None => Tier::Durable,
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
        tier,
        shape: shape_tags(document.get("shape")),
        phrases: phrase_list(document.get("phrases")),
        missing: missing_list(document.get("missing")),
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
/// Whether a corpus directory holds at least one fact file. The
/// scaffold's emptiness is what distinguishes "the home has a corpus"
/// from "the home has the directory a corpus would live in".
fn holds_facts(dir: &std::path::Path) -> bool {
    let Ok(read) = fs::read_dir(dir) else {
        return false;
    };
    read.flatten()
        .any(|entry| entry.path().extension().and_then(|ext| ext.to_str()) == Some("md"))
}

pub fn resolve_root(cwd: &Path) -> PathBuf {
    let home = crate::home::fleet_home(
        std::env::var("FLEET_HOME").ok().as_deref(),
        std::env::var("HOME").ok().as_deref(),
    )
    .ok();
    resolve_root_from(cwd, home.as_deref())
}

/// The resolution behind its own seam, so the home-first rule is
/// testable without mutating the process environment every other test
/// in the binary reads.
pub fn resolve_root_from(cwd: &Path, home: Option<&Path>) -> PathBuf {
    // KI-087 (and KI-074's root cause): the fleet home's own corpus is
    // the one this process's runs belong to, so it is checked BEFORE
    // the working directory's ancestor walk. A run started outside any
    // intended corpus — a scheduled job from a scratch directory, a
    // smoke from an empty dir — silently adopted a foreign corpus the
    // walk found further up the tree.
    if let Some(home) = home {
        let own = home.join(MEMORIES_DIR);
        // It must HOLD facts, not merely exist: an empty `memories/`
        // under the home is the install's scaffold, and a scaffold
        // preferred over a real corpus found by the walk silently
        // shadows it — the failure this rule itself introduced before
        // the check was added.
        if own.is_dir() && holds_facts(&own) {
            return own;
        }
    }
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
/// fact is re-embedded, an untouched one keeps its cached vector. The
/// session archive's index reads the same stamp off its own logs, so
/// the two caches agree on what "changed" means — which is why it is
/// public rather than a second, drifting copy of the format.
pub fn file_stamp(path: &Path) -> Option<String> {
    let metadata = fs::metadata(path).ok()?;
    let modified = metadata.modified().ok()?;
    let duration = modified.duration_since(UNIX_EPOCH).ok()?;
    Some(format!(
        "{}:{}:{}",
        duration.as_secs(),
        duration.subsec_nanos(),
        metadata.len()
    ))
}

/// Load every fact under `root`, sorted by name. A missing directory is
/// no facts, not an error. A file that does not read as a fact is
/// skipped by name with one logged line; the others still load.
pub fn load_facts(root: &Path) -> Vec<Fact> {
    load_facts_matching(root, true)
}

/// The archive's read of the same shape: every `.md` under the archive
/// directory, with the stem-equals-name rule relaxed. A collision
/// copy's file name carries its serial (`<name>.2.md`) while its
/// frontmatter still names the fact, so the archive's surface cards it
/// by the name the frontmatter carries. A file that does not read as a
/// fact is skipped by name exactly as the live corpus's is.
fn load_facts_archived(root: &Path) -> Vec<Fact> {
    load_facts_matching(root, false)
}

/// The read itself: `*.md` at one level under `root`, sorted by name,
/// with `stem_is_the_name` deciding whether the frontmatter's `name`
/// must equal the file's stem — every live-corpus read enforces it,
/// the archive's read does not.
fn load_facts_matching(root: &Path, stem_is_the_name: bool) -> Vec<Fact> {
    let Ok(entries) = fs::read_dir(root) else {
        return Vec::new();
    };
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
        let Some(stem) = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .map(str::to_string)
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
        match parse_document(&text)
            .and_then(|document| fact(document, &stem, path.clone(), stamp, stem_is_the_name))
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
/// no new crate, no key, no internet. `FLEET_EMBEDDING_ENDPOINT`
/// overrides it for a machine whose server listens elsewhere.
pub const DEFAULT_EMBEDDING_ENDPOINT: &str = "http://127.0.0.1:8901/v1/embeddings";

/// The model the embeddings request names. A llama-server built as a
/// router needs the field: without it the request waits for a default
/// the server does not have and the probe times out, which degrades
/// every recall to keyword-only on a machine whose endpoint is healthy
/// (KI-296). Overridable with `FLEET_EMBEDDING_MODEL`.
pub const DEFAULT_EMBEDDING_MODEL: &str = "embeddinggemma";

/// The env var that overrides the embedding endpoint — a second model
/// on another port, or a llama-server bound elsewhere. Resolved the
/// house way: env → the default constant, an empty value counting as
/// unset.
pub const EMBEDDING_ENDPOINT_VAR: &str = "FLEET_EMBEDDING_ENDPOINT";

/// The endpoint the probe uses: `FLEET_EMBEDDING_ENDPOINT` when it
/// names one, the loopback default otherwise.
///
/// The env var wins and `<home>/fleet.env` is the fallback: the
/// settings writer's env-var writes are a real configuration surface,
/// so the runtime reads what it writes — the G8 defect's close, which
/// had a GUI write land in a file nothing read.
pub fn embedding_endpoint() -> String {
    endpoint_from(|name| std::env::var(name).ok().or_else(|| fleet_env_value(name)))
}

/// The name's value in the fleet home's own `fleet.env`, when the home
/// resolves and the file carries it. Pure behind its own seam so the
/// fallback is testable without mutating the process environment.
pub fn fleet_env_value(name: &str) -> Option<String> {
    let home = crate::home::fleet_home(
        std::env::var("FLEET_HOME").ok().as_deref(),
        std::env::var("HOME").ok().as_deref(),
    )
    .ok()?;
    crate::home::read_fleet_env(&home)
        .get(name)
        .filter(|value| !value.is_empty())
        .cloned()
}

/// The resolution itself, behind the lookup seam so the test proves the
/// override without mutating a process-global environment other tests
/// read — the same reason `calib::resolve_root` takes its home as a
/// parameter.
fn endpoint_from(lookup: impl Fn(&str) -> Option<String>) -> String {
    lookup(EMBEDDING_ENDPOINT_VAR)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_EMBEDDING_ENDPOINT.to_string())
}

/// The probe's round-trip budget. A live server answers a single embed
/// in milliseconds; a dead one refuses the connect instantly. The budget
/// is the ceiling for a black-holed port, not the expected cost.
const PROBE_TIMEOUT: Duration = Duration::from_secs(5);

/// The pause between the probe's tries. The machine's endpoint flaps —
/// a request hangs past every budget, an identical one answers in
/// 37 ms between them, and a restart window refuses outright — and a
/// probe that reads one failure as "no backend" degrades a healthy
/// machine for a whole process (KI-296). The embed's own retry is
/// immediate, which absorbs a dead pooled connection but lands right
/// back in a hang or a refused restart; the pause is what spans it. A
/// genuinely-dead backend refuses in milliseconds, so the pauses are
/// the whole extra cost; a black-holed port is bounded by the probe
/// budget per try, paid once per load.
const PROBE_RETRY_PAUSE: Duration = Duration::from_millis(750);

/// How many paused retries follow the probe's first try — three tries
/// in all. One was not enough under a loaded suite: a try can fail
/// before it ever reaches the backend, and the flap's own length is
/// nobody's to promise.
const PROBE_RETRIES: usize = 2;

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
    /// One clone of this backend behind its own box, so a cached
    /// embedder can hand a copy to each load. KI-064.
    fn box_clone(&self) -> Box<dyn Embedder>;

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
/// `Clone` so a cached probe answer can hand a copy to each load
/// (KI-064).
#[derive(Clone)]
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
        // Paused retries (KI-296): the flap a live endpoint answers
        // through outlasts an immediate retry, and a restart window
        // refuses for about as long as the pause.
        let mut dims = Embedder::embed(&embedder, PROBE_TEXT);
        for _ in 0..PROBE_RETRIES {
            if dims.is_some() {
                break;
            }
            std::thread::sleep(PROBE_RETRY_PAUSE);
            dims = Embedder::embed(&embedder, PROBE_TEXT);
        }
        let dims = dims?.len();
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
    fn box_clone(&self) -> Box<dyn Embedder> {
        Box::new((*self).clone())
    }

    fn embed(&self, text: &str) -> Option<Vec<f32>> {
        let body = serde_json::json!({
            "model": std::env::var("FLEET_EMBEDDING_MODEL")
                .ok()
                .filter(|name| !name.trim().is_empty())
                .unwrap_or_else(|| DEFAULT_EMBEDDING_MODEL.to_string()),
            "input": text,
        })
        .to_string();
        // The embed server closes an idle keep-alive connection after
        // five seconds (`Keep-Alive: timeout=5`), and a pooled connection
        // that died while idle fails the request in milliseconds — one
        // silent `None` that turns a whole ranking into zeros and a
        // corpus load into keyword-only. The retry absorbs exactly that
        // race: the failed attempt burned the dead connection, and the
        // second rides a fresh one. A backend that is genuinely down
        // pays one extra failed attempt per embed and answers the same.
        let attempt = || {
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
        };
        attempt().or_else(attempt)
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
    fn box_clone(&self) -> Box<dyn Embedder> {
        Box::new(KeywordOnly)
    }

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
            Probe {
                endpoint,
                dims: Some(dims),
                embedder: Box::new(http),
            }
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
/// name, carrying the digest of the text the vector was computed from —
/// plus the embedder identity and dimensionality the vectors were
/// computed **at**. A cache never errors — an absent or malformed index
/// reads as empty and costs a recompute — and an index keyed to another
/// embedder is not this embedder's cache: a model swap leaves
/// stale-dimension vectors that cosine's length guard scores 0.0,
/// which would silently kill the vector channel while `doctor` names
/// it live. The key is what makes that impossible.
#[derive(Debug, Default, Serialize, Deserialize)]
struct Index {
    /// The embedder that computed the cached vectors — its endpoint or
    /// model name, from `Embedder::vector_info`.
    #[serde(default)]
    embedder: String,
    /// The vector length that embedder served.
    #[serde(default)]
    dims: usize,
    #[serde(default)]
    facts: HashMap<String, IndexEntry>,
}

/// One cached vector with the digest of the text it was computed from.
///
/// The digest, not the file's stamp (KI-228): the recall-count bump
/// rewrites a fact's file in place, which moves its mtime and so its
/// `secs:nanos:len` stamp, and a stamp-keyed entry therefore re-embeds a
/// fact whose *content* never changed. The corpus's three most-recalled
/// facts had been re-embedded on essentially every load. The digest is
/// over `Fact::indexed_text()` — the description, the shape tags, and
/// the phrases, which is exactly what the embedder saw — so a bump
/// changes nothing and an edit to any of them re-embeds that one fact.
/// `recall-count` is deliberately outside the digest for the same
/// reason: it is a rank signal, not part of the fact's direction.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct IndexEntry {
    digest: String,
    vector: Vec<f32>,
}

/// The digest an index entry is keyed by: the first 16 hex characters of
/// a SHA-256 over the text the vector was computed from, the same hash
/// the route-decision cache joins on.
fn indexed_digest(fact: &Fact) -> String {
    calib::hash16(&fact.indexed_text())
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
    fn write(root: &Path, key: Option<&(String, usize)>, entries: &HashMap<String, IndexEntry>) {
        let index = Index {
            embedder: key.map(|(name, _)| name.clone()).unwrap_or_default(),
            dims: key.map(|(_, dims)| *dims).unwrap_or_default(),
            facts: entries.clone(),
        };
        let Ok(text) = serde_json::to_string(&index) else {
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

/// The topical channels' weights, and the topical dimension they sum
/// to. The vector channel is the cosine between the instruction's
/// embedding and the fact's indexed text; the keyword channel is how
/// much of the instruction the fact's topic text answers. Together
/// they are the **neighborhood** the fact talks about — 0.30 of the
/// composite, well under half, so no topical fact can carry a ranking
/// on its own. That is the slice-19 reweighting, twice over: the pair
/// used to be 0.80 (vector 0.55 + keyword 0.25) and could, which is
/// exactly how a homonym across shapes outranked the right shape's own
/// answer; and the pair's first reweighting (0.20 each, 0.40 together)
/// still let a maximum-proximity fact with NO direction evidence score
/// 0.55 against a direction-strong fact's 0.49 — proximity won, which
/// is the owner's ruling inverted. The topical pair is 0.15 each now,
/// and the direction channels carry 0.60 against their 0.30.
///
/// The two sub-channels carry equal weight: the embedded direction and
/// the token overlap are two readings of the same dimension, and
/// neither is the other's tiebreaker.
pub const W_VECTOR: f64 = 0.15;

/// The keyword channel's weight: how much of the instruction the fact's
/// topic text answers.
pub const W_KEYWORD: f64 = 0.15;

/// The trajectory channel's weight: how much of the instruction's
/// trajectory the fact's recorded phrases answer. This is the fact's
/// DIRECTION — the vocabulary the resolution drove with — and it is
/// the largest single channel, because "a problem shape tied to the
/// word phrases that were recorded during the resolution" is the
/// owner's key information. A fact whose phrases echo the
/// instruction's trajectory is driving toward this minima; a fact that
/// merely talks about the neighborhood is not.
pub const W_TRAJECTORY: f64 = 0.30;

/// The history channel's weight: recall-count, earned by use. Most of
/// the fact's MAGNITUDE — the evidence it has actually driven — beside
/// the provenance channel.
pub const W_HISTORY: f64 = 0.25;

/// The freshness channel's weight: exponential temporal decay. The
/// smallest channel on purpose: a fresh fact is not a better vector.
pub const W_FRESHNESS: f64 = 0.10;

/// The provenance channel's weight: the fact's basis. The smaller half
/// of the magnitude — a confirmed-by vector has driven somewhere the
/// owner verified, an inferred one is a proposal at half strength.
/// Provenance entered the score at slice 19; before that it was read
/// only by the permission model, and the magnitude of a fact's
/// evidence was invisible to the ranking.
pub const W_PROVENANCE: f64 = 0.05;

/// What a `confirmed-by` fact contributes to the provenance channel,
/// against an `inferred` one's half. A confirmed vector outweighs an
/// inferred one without erasing it: the multidimensional context is
/// what ranks, and an inferred fact that drove is still a vector.
pub const PROVENANCE_CONFIRMED: f64 = 1.0;

/// What an `inferred` fact contributes to the provenance channel.
pub const PROVENANCE_INFERRED: f64 = 0.5;

/// Recalls at which the history channel reaches its ceiling — the
/// fork's `recallWindow`, ported, and then re-derived against the corpus
/// that outgrew it.
///
/// The fork's window was 8, and at 8 the channel fell off a cliff: 16 of
/// this corpus's 24 facts sit at or above it (the median recall-count is
/// 56), so the channel contributed a constant 0.250 to two-thirds of the
/// facts and discriminated only among the remaining third — a saturated
/// channel is a dead channel, and it handed every never-recalled fact a
/// 0.25 deficit the word channels, measured at 0.04–0.15 in practice,
/// could not overcome.
///
/// 128 spans the observed range instead of clamping it, and the curve is
/// logarithmic rather than linear, so the channel keeps discriminating
/// across the whole span rather than re-saturating partway up.
pub const RECALL_CEILING: u64 = 128;

/// The history channel's curve: recall-count over the ceiling, on a
/// logarithmic scale.
///
/// Linear-to-the-ceiling was the fork's own shape and it is what
/// saturated: `min(count, 8) / 8` maps 8, 27, 56 and 103 all to 1.0. The
/// logarithm keeps 0 at 0.0 and the ceiling at 1.0 — so the endpoints,
/// the weight, and the direction-outranks-proximity ruling are all
/// unchanged — while spreading the middle: 1 recall is 0.145, 8 is 0.452,
/// 27 is 0.686, 56 is 0.832, and 103 is 0.955.
///
/// Monotonic in count, which is what keeps the calibration stream's
/// meaning intact: a `recall_hit` row's score is still the score the
/// fact entered the turn at, and more recalls still means more history.
pub fn history_term(recall_count: u64) -> f64 {
    let capped = recall_count.min(RECALL_CEILING);
    ((1 + capped) as f64).ln() / ((1 + RECALL_CEILING) as f64).ln()
}

/// The decay half-life in days: a fact's freshness halves every 90
/// days. Exponential rather than the fork's linear window, so an old
/// fact decays smoothly instead of falling off a cliff.
pub const DECAY_HALFLIFE_DAYS: f64 = 90.0;

/// The floor a fact must clear to enter a turn for this instruction.
/// Below it the fact is noise: a fact that matches nothing at all is
/// worth its standing alone — freshness (0.10) plus provenance (0.05) —
/// 0.15, and a turn about one topic must not be injected with facts
/// about another. The floor's meaning is exact under the named weights,
/// and it reads the owner's ruling in miniature: a quarter-strength
/// match in a DIRECTION channel beside standing clears it (trajectory
/// 0.225, history 0.2125), while the topical pair needs a third of its
/// weight to reach it (0.15 × 1/3 = 0.05, exactly at the floor) — a
/// fact enters when it points somewhere, not merely when it sits near.
/// A heavily-recalled fact that matches nothing still clears it, ranks
/// last, and is the budget's first drop — the same accepted behavior as
/// before the reweighting.
///
/// The floor is NOT a guard against a near-miss: the live embedding
/// model's cosines do not separate relevance on their own (KI-058
/// measured 0.6049 for an unrelated pair), and under these weights that
/// cosine alone puts an unrelated same-domain fact at 0.15 × 0.6049 +
/// 0.10 + 0.05 = 0.24 — over the floor. The floor excludes standing and
/// wordless noise; the shape partition and the spillover's routes are
/// what keep a near-miss from being read as the shape's answer.
pub const MIN_RECALL_SCORE: f64 = 0.20;

/// The floor the primary pool's best score must clear before the
/// shape's own answer stands alone. Below it the off-shape facts are
/// admitted as candidates through the spillover's second route; at or
/// above it they enter only on the budget's room.
///
/// Sized against the composite's 1.00 and against
/// [`MIN_RECALL_SCORE`] (0.20), and re-derived when the weights moved:
/// 0.50 is the point at which the shape's own answer carries DIRECTION.
/// The most a fact with no direction evidence can score is full topical
/// coverage (vector 0.15 + keyword 0.15) with freshness and provenance
/// (0.15) — 0.45 — so the neighborhood alone can no longer stand alone
/// as the shape's answer, which is the owner's ruling applied to the
/// pool. A same-shape fact clears it with a 0.2 trajectory match on top
/// of full topical coverage (0.51), or two recalls' history (0.5125);
/// the direction-strong fact — trajectory 0.8, four recalls'
/// history, confirmed-by, vector 0.2 — scores 0.545 and clears it with
/// 0.045 to spare. A pool whose best is below it has matched the shape
/// without corroborating it in a second dimension — the fact that
/// merely talks about the neighborhood — and the marked off-shape facts
/// are worth their weaker status beside it.
pub const SPILLOVER_FLOOR: f64 = 0.50;

/// One fact's score: the total the ranking orders by, and the channels
/// that produced it — the "why" the surface prints.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Score {
    pub total: f64,
    /// The topical cosine sub-channel: the embedded direction.
    pub vector: f64,
    /// The topical token-coverage sub-channel.
    pub keyword: f64,
    /// The trajectory channel: the recorded phrases' match.
    pub trajectory: f64,
    /// The history channel: recall-count over the ceiling.
    pub history: f64,
    /// The freshness channel: exponential decay.
    pub freshness: f64,
    /// The provenance channel: the basis.
    pub provenance: f64,
}

impl Score {
    /// Compose the named channels. The shape is deliberately absent:
    /// it partitions the pool (see [`Memory::partition`]) rather than
    /// adding a term, so no constant here can make a wrong-shape fact
    /// outrank the right shape's own answer.
    fn compute(
        vector: f64,
        keyword: f64,
        trajectory: f64,
        recall_count: u64,
        basis: Basis,
        date: &str,
        today_days: Option<i64>,
    ) -> Score {
        let history = history_term(recall_count);
        let freshness = decay_factor(date, today_days);
        let provenance = match basis {
            Basis::ConfirmedBy => PROVENANCE_CONFIRMED,
            Basis::Inferred => PROVENANCE_INFERRED,
        };
        let total = W_VECTOR * vector
            + W_KEYWORD * keyword
            + W_TRAJECTORY * trajectory
            + W_HISTORY * history
            + W_FRESHNESS * freshness
            + W_PROVENANCE * provenance;
        Score {
            total,
            vector,
            keyword,
            trajectory,
            history,
            freshness,
            provenance,
        }
    }

    /// Every channel zero: the score of a fact that was never ranked.
    /// The protected tier is admitted unconditionally — its members did
    /// not compete for their place — so they carry this rather than an
    /// invented standing the ranker never computed.
    fn unranked() -> Score {
        Score {
            total: 0.0,
            vector: 0.0,
            keyword: 0.0,
            trajectory: 0.0,
            history: 0.0,
            freshness: 0.0,
            provenance: 0.0,
        }
    }
}

/// Cosine similarity, in full f64 even though the vectors are f32. A
/// length mismatch or a zero vector scores 0.0 — not similar, not an
/// error.
/// The Matryoshka slice: the vector's leading 128 dimensions. Truncation
/// is the property the model was trained for — the slice carries most of
/// the neighborhood for coarse ranking at a quarter of the multiply cost,
/// and the full-width re-rank restores the fine order (KI-252 move 2).
pub(crate) const SLICE_DIMS: usize = 128;

/// How many slice-stage survivors re-rank full-width (KI-252 move 2).
/// Named so the measurement gate can argue it: large enough that the
/// true neighborhood survives the coarse order on a healthy corpus,
/// small enough that the full-width pass touches a fraction of it.
pub(crate) const PREFILTER_K: usize = 24;

pub(crate) fn slice(v: &[f32]) -> &[f32] {
    &v[..v.len().min(SLICE_DIMS)]
}

/// The two-stage prefilter: which facts' vector term gets a full-width
/// cosine. Stage one scores every fact's slice against the query's slice
/// and keeps the top-K; every fact outside the set reads 0.0 for its
/// vector term, exactly the shape a partial embed leaves.
pub(crate) fn vector_prefilter(
    query_vector: &Option<Vec<f32>>,
    vectors: &HashMap<String, Vec<f32>>,
    k: usize,
) -> Option<std::collections::HashSet<String>> {
    let query = query_vector.as_ref()?;
    if query.len() < SLICE_DIMS {
        return None; // a short (already-sliced or partial) query: rank full-width
    }
    let query_slice = slice(query);
    let mut scored: Vec<(String, f64)> = vectors
        .iter()
        .filter(|(_, vector)| vector.len() >= SLICE_DIMS)
        .map(|(name, vector)| (name.clone(), cosine(query_slice, slice(vector))))
        .collect();
    scored.sort_by(|left, right| {
        right
            .1
            .partial_cmp(&left.1)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    Some(scored.into_iter().take(k).map(|(name, _)| name).collect())
}

pub(crate) fn cosine(left: &[f32], right: &[f32]) -> f64 {
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
pub(crate) fn tokens(text: &str) -> Vec<String> {
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

/// The English inflection suffixes the stemmer strips, longest first so
/// the longest match wins. Deliberately short: this is not Porter, and
/// it does not need to be. The corpus is code-shaped prose and the gap
/// it closes is the documented one — "the trajectory channel measures
/// literal token overlap with no stemming" — measured at 6 of 99 query
/// tokens (`facts`→`fact`, `fixed`→`fix`, `derived`→`deriv`,
/// `resolved`→`resolv`, `served`→`serv`).
///
/// The `-ies`/`-ied` rules map to `y` rather than stripping to nothing,
/// because `carries`→`carry` is the match and `carri` is not a word any
/// corpus writes.
const SUFFIXES: [(&str, &str); 15] = [
    ("ations", "ate"),
    ("ation", "ate"),
    ("ments", "ment"),
    ("ment", "ment"),
    ("nesses", "ness"),
    ("ness", "ness"),
    ("ings", ""),
    ("ing", ""),
    ("ies", "y"),
    ("ied", "y"),
    ("ers", "er"),
    ("ed", ""),
    ("es", ""),
    ("s", ""),
    ("ly", ""),
];

/// The shortest stem worth returning: a suffix is only stripped when
/// what remains is at least this long, so `is` and `as` do not collapse
/// to the empty string and `es` does not eat a two-letter word whole.
const MIN_STEM: usize = 3;

/// The light stem the lexical channels compare on: lowercase, one
/// inflection suffix stripped, longest match first.
///
/// Both sides of every comparison are stemmed, so the channel stays
/// symmetric — a fact's `phrases` and the instruction's tokens are
/// reduced the same way, and neither gains an advantage from the rule.
/// This is the lexical channels' own business: the skills module shares
/// [`tokens`] and keeps its own behaviour, because a guard that polices
/// `judgment` against a vocabulary carrying `judgement` must not start
/// accepting both.
pub(crate) fn stem(word: &str) -> String {
    for (suffix, replacement) in SUFFIXES {
        if let Some(head) = word.strip_suffix(suffix) {
            if head.len() + replacement.len() >= MIN_STEM {
                return format!("{head}{replacement}");
            }
        }
    }
    word.to_string()
}

/// The stemmed token set of `text`: [`tokens`], each stemmed, order
/// preserved. This is what the keyword and trajectory channels compare.
pub(crate) fn stems(text: &str) -> Vec<String> {
    tokens(text).iter().map(|token| stem(token)).collect()
}

/// How much of the instruction the fact answers: the instruction's
/// stems the fact's topic text carries, over the instruction's stems. An
/// instruction with no tokens covers nothing.
fn keyword_coverage(query: &[String], fact: &[String]) -> f64 {
    if query.is_empty() {
        return 0.0;
    }
    let covered = query
        .iter()
        .filter(|token| fact.iter().any(|seen| seen == *token))
        .count();
    covered as f64 / query.len() as f64
}

/// How much of the instruction's trajectory the fact's recorded
/// phrases answer: the instruction's stems the phrases carry, over the
/// instruction's stems. A fact with no recorded phrases has no
/// direction, so it scores 0.0 — the same shape as the keyword
/// channel, measured over the trajectory's own text so the two
/// dimensions cannot double-count each other.
fn trajectory_coverage(query: &[String], phrases: &[String]) -> f64 {
    if query.is_empty() || phrases.is_empty() {
        return 0.0;
    }
    let carried: Vec<String> = phrases.iter().flat_map(|phrase| stems(phrase)).collect();
    let covered = query
        .iter()
        .filter(|token| carried.iter().any(|seen| seen == *token))
        .count();
    covered as f64 / query.len() as f64
}

/// Exponential temporal decay with a named half-life: a fact's freshness
/// halves every [`DECAY_HALFLIFE_DAYS`]. A fact with no readable date —
/// or a machine with no readable clock — contributes no decay term, the
/// fork reader's own rule for an unreadable date.
fn decay_factor(date: &str, today_days: Option<i64>) -> f64 {
    let Some(today) = today_days else { return 0.0 };
    let Some(then) = parse_day(date) else {
        return 0.0;
    };
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
        &[
            "refactor",
            "rename",
            "migration",
            "rewrite",
            "revert",
            "breaking change",
        ],
    ),
];

/// The taxonomy in its own order, for the surface's naming.
pub const SHAPES: [&str; 7] = [
    "rendering",
    "state-sync",
    "deploy",
    "build",
    "change",
    "deps",
    "docs",
];

/// The **failure shapes**: the avoidance half of the shelf, named for
/// Fleet's own failure record in `docs/known-issues.md`. Each names a
/// class of thing that broke, with the ledger rows it is drawn from:
///
/// - `fail-open` — a dependency was unavailable and the path proceeded
///   on a degraded answer instead of refusing (KI-044: route selection's
///   `sys1_classify` fails open with verdict `error` and every run falls
///   back to the declared HEAD; KI-110: the improve-gate's subprocess
///   probe failed open and could grade the wrong binary).
/// - `drops-the-row` — a writer wrote a record the reader silently
///   discarded, so the system behaved as if it never existed (KI-164: a
///   JSON `null` made `parse_row` drop the whole line; KI-171: a
///   hand-written row missing `family` dropped silently).
/// - `bound-exceeded` — an unbounded or wrongly-bounded read or write
///   overran its budget, undercounted, or rode control characters into
///   the surface (KI-060: the description unbounded; KI-167: the
///   reader's `READ_CAP=512` undercounts a heavy day; KI-120/KI-122:
///   the tower scan and the second-brain read unbounded; KI-035: no cap
///   on a lane's stdout line).
/// - `claim-outran-evidence` — a doc, a recap, a plan's assumption, or
///   a test asserted more than the code or the run proved (KI-160: the
///   slice-16 plan's "fleet's suite covers the lane" overclaimed;
///   KI-159: two doc claims read stronger than the code; KI-009: a
///   recap claimed the wrong CI state; KI-133: the feature doc's
///   network wording overclaimed).
/// - `fixed-width-clip` — a fixed-width surface was handed a line
///   built by concatenation rather than bounded to the pane's own
///   width, and it clipped or hard-wrapped the line mid-token
///   (KI-078: the approval modal cut a long call summary mid-word and
///   the subagent card's state token was pushed off the line; KI-085:
///   the compaction card's first line cut mid-token at 52 columns;
///   KI-096: the cron fire card cut at 50; KI-115: a widened pane cut
///   the compaction card again; KI-179: three card lines grew past
///   `CARD_WIDTH` at once. KI-055's one shared word-aware wrapper is
///   the named remedy and is still open — this shape is what the
///   wrapper is for).
/// - `foreign-root` — a path was resolved from the working directory
///   alone, so a run started outside any intended root silently
///   adopted a foreign one (KI-074: `memory::resolve_root`'s ancestor
///   walk adopted a corpus found further up the tree; KI-087: the
///   scheduled run reproduced it verbatim; KI-099: the improve gate
///   gates against the wrong tree; KI-106: `doctor --diagnose`'s
///   default ledger path. One root cause, four surfaces).
///
/// A fact tagged with a failure shape is an anti-vector: it records
/// what drove AWAY from the minima, and recall serves it through the
/// same partition as a resolution. A fact may carry both axes —
/// `[build, bound-exceeded]` is a structure shape and a failure shape,
/// and either match puts it in the pool. The failure shape is the
/// PROBLEM's shape, not a claim about how the trajectory ended: the
/// outcome tags (`incomplete`, `resolved`) carry that, and
/// [`Fact::sign`] reads them first.
pub const FAILURE_SHAPES: [&str; 6] = [
    "fail-open",
    "drops-the-row",
    "bound-exceeded",
    "claim-outran-evidence",
    "fixed-width-clip",
    "foreign-root",
];

/// The **outcome tags**: what became of a trajectory, beside the shape
/// that says what kind of problem it was. They are the third sign's
/// vocabulary and the failure-pool's disambiguation, and they are the
/// reason a failure shape is not by itself a claim that the trajectory
/// diverged:
///
/// - `incomplete` — the direction was right and the companions it
///   needed were absent, so it drove partway and stalled. The vector
///   most mistaken for done: not wrong, not finished. It ranks exactly
///   as a resolution, and its `missing` companions ride its line.
/// - `resolved` — the trajectory reached the minima. Only load-bearing
///   beside a failure shape, where it says the fix landed and the fact
///   is a candidate rather than an anti-vector.
///
/// An outcome tag is not a problem shape: a fact carries one beside a
/// structure or failure shape, never instead of one.
pub const OUTCOME_SHAPES: [&str; 2] = ["incomplete", "resolved"];

/// The `incomplete` outcome tag — the third sign's own word, and the
/// marker's source.
pub const INCOMPLETE_TAG: &str = "incomplete";

/// The `resolved` outcome tag — the failure-shape fix that landed.
pub const RESOLVED_TAG: &str = "resolved";

/// The failure axis's keyword taxonomy, Fleet's own (the fork's
/// `shapes.json` carries only the structure shapes). Consulted after
/// the structure table, so the fork's order rule still decides when
/// both axes' terms appear — an instruction that names a structure
/// shape and a failure shape routes under the structure shape.
pub const FAILURE_SHAPE_KEYWORDS: [(&str, &[&str]); 6] = [
    (
        "fail-open",
        &[
            "fail open",
            "fails open",
            "failed open",
            "failing open",
            "fail-open",
            "fails-open",
            "degraded answer",
        ],
    ),
    (
        "drops-the-row",
        &[
            "drops the row",
            "dropped the row",
            "drops the record",
            "dropped the record",
            "row vanished",
            "row was dropped",
            "silently dropped",
            "reader dropped",
            "row drop",
            "dropped row",
        ],
    ),
    (
        "bound-exceeded",
        &[
            "unbounded",
            "no cap",
            "past the bound",
            "bound exceeded",
            "overran",
            "undercount",
        ],
    ),
    (
        "claim-outran-evidence",
        &[
            "outran",
            "overclaimed",
            "claims more than",
            "asserts more than",
            "reads stronger than",
        ],
    ),
    (
        "fixed-width-clip",
        &[
            "fixed width",
            "fixed-width",
            "mid-token",
            "mid-word",
            "clipped the",
            "cut mid",
            "wraps mid",
            "wrapped mid",
            "overflows the pane",
            "past the card width",
        ],
    ),
    (
        "foreign-root",
        &[
            "foreign root",
            "adopted a foreign",
            "adopts a foreign",
            "wrong tree",
            "wrong repo",
            "cwd walk",
            "walks up",
            "working-directory walk",
        ],
    ),
];

/// The outcome axis's keyword taxonomy, Fleet's own like the failure
/// table's. Consulted LAST, after both problem-shape tables, because an
/// outcome is not a problem shape: an instruction that names a shape
/// and an outcome routes under the shape, and an instruction that names
/// only an outcome ("the torch stack was missing") routes to the third
/// sign — the "have I stalled here before?" question.
pub const OUTCOME_SHAPE_KEYWORDS: [(&str, &[&str]); 2] = [
    (
        INCOMPLETE_TAG,
        &[
            "incomplete",
            "was missing",
            "is missing",
            "were missing",
            "missing the",
            "stalled",
            "never completed",
            "partway",
        ],
    ),
    (
        RESOLVED_TAG,
        &[
            "resolved",
            "what resolved",
            "was fixed",
            "is fixed",
            "fixed it",
            "the fix landed",
        ],
    ),
];

/// Whether `tag` is one of the failure shapes — the axis that gives a
/// fact its sign.
pub fn is_failure_shape(tag: &str) -> bool {
    FAILURE_SHAPES.contains(&tag)
}

/// Whether `tag` is one of the outcome tags — what became of the
/// trajectory, the third sign's vocabulary.
pub fn is_outcome_shape(tag: &str) -> bool {
    OUTCOME_SHAPES.contains(&tag)
}

/// The axis a shape name belongs to, for the surfaces' naming.
pub fn shape_axis(shape: &str) -> &'static str {
    if is_failure_shape(shape) {
        "failure"
    } else if is_outcome_shape(shape) {
        "outcome"
    } else {
        "structure"
    }
}

/// A fact's sign in the recall ranking.
///
/// A resolution is a vector TOWARD the minima: the approaches that fit
/// the shape, the prevention half of the shelf. A divergence is an
/// anti-vector in the OPPOSITE direction: what kind of thing broke, the
/// avoidance half. An incomplete is a vector whose direction was right
/// and whose companions were absent: it drove partway and stalled, and
/// it is the one most mistaken for done. "A failure drives the vector
/// in an opposite direction for a solution trajectory" — so an
/// anti-vector that matches the current trajectory is highly relevant
/// and surfaces as prominently as a resolution. It is not a penalty and
/// not a lower rank; what keeps it from being read as a candidate
/// solution is the marker every line and card carries. The third sign
/// is not a penalty either: an incomplete vector ranks exactly as a
/// resolution, and what distinguishes it is the marker and the missing
/// companions named beside it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sign {
    /// Drove toward the minima: a resolution.
    Resolution,
    /// Drove away from it: a failure, an anti-vector.
    Divergence,
    /// Drove the right direction and stalled: an incomplete solution,
    /// whose missing companions are named.
    Incomplete,
}

impl Sign {
    /// The sign a fact's tags give it — the tag is the sign, so no
    /// field can contradict the tags.
    pub fn of(fact: &Fact) -> Sign {
        fact.sign()
    }

    /// The word the surface and the assembly line carry.
    pub fn marker(self) -> &'static str {
        match self {
            Sign::Resolution => "resolved:",
            Sign::Divergence => "diverged:",
            Sign::Incomplete => "incomplete:",
        }
    }

    /// The bare word, for a card's header.
    pub fn word(self) -> &'static str {
        match self {
            Sign::Resolution => "resolved",
            Sign::Divergence => "diverged",
            Sign::Incomplete => "incomplete",
        }
    }
}

/// Classify one instruction into a shape by its keywords, or `None`
/// when no shape's terms appear. The structure table is consulted
/// first — the fork's `classifyShape`, ported, substring match on the
/// lowercased text with the taxonomy's order deciding — then Fleet's
/// failure axis, then the outcome axis, so Fleet's own failure record
/// is routable without disturbing the fork's order rule. Keyword-backed,
/// not model-backed, because the model path proved unreliable.
pub fn classify_shape(text: &str) -> Option<&'static str> {
    let haystack = text.to_lowercase();
    SHAPE_KEYWORDS
        .iter()
        .find(|(_, terms)| terms.iter().any(|term| haystack.contains(term)))
        .or_else(|| {
            FAILURE_SHAPE_KEYWORDS
                .iter()
                .find(|(_, terms)| terms.iter().any(|term| haystack.contains(term)))
        })
        .or_else(|| {
            OUTCOME_SHAPE_KEYWORDS
                .iter()
                .find(|(_, terms)| terms.iter().any(|term| haystack.contains(term)))
        })
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
    /// The adjacency map over the frontmatter's own edges, derived at
    /// load. See [`crate::recall_graph`].
    graph: crate::recall_graph::Graph,
    /// KI-252 move 2, default off: rank the vector channel's fan-out on
    /// the 128-dimension Matryoshka slice and re-rank the top slice hits
    /// full-width. The measurement gate decides the default; the setting
    /// exists so the single pass stays the contract until it does.
    pub two_stage: std::sync::atomic::AtomicBool,
    /// The load-time index build's per-embed wall clocks, in call order
    /// (KI-249): `EMBED_TIMEOUT` bounds one of these calls, and the
    /// bound's size is the measurement's to set. Empty unless this
    /// load embedded something; the caller holding a calibration stream
    /// records one row per entry.
    pub build_embeds_ms: Vec<u128>,
}

impl Memory {
    /// How many facts the corpus loaded — the denominator of an
    /// embed-timing measurement (KI-249: milliseconds per load over this
    /// many facts is the tail the embed bounds should come from).
    pub fn fact_count(&self) -> usize {
        lock_facts(&self.facts).len()
    }
}

/// The mutex, recovered from poison: the only critical sections are
/// reads and a bounded count update, neither of which can panic, so a
/// poisoned lock means some unrelated panic tripped the flag and the
/// corpus is still sound — the calibration writer's own rule.
fn lock_facts(facts: &Mutex<Vec<Fact>>) -> std::sync::MutexGuard<'_, Vec<Fact>> {
    facts
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// The corpus's `Debug`: the root, the backend, and the fact count —
/// what a log line or a test failure wants to see, and never the
/// vectors. Hand-written because the embedder is a trait object with
/// no `Debug` of its own, and a derived `Debug` would demand one.
impl std::fmt::Debug for Memory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Memory")
            .field("root", &self.root)
            .field("backend", &self.embedder.name())
            .field("facts", &lock_facts(&self.facts).len())
            .finish()
    }
}

/// The embedder the corpus loads with, from the probe's answer cached
/// per process. KI-064: every load ran one real round trip, so a chat
/// session paid it per message and a black-holed port would cost the
/// probe's five-second budget every task. The endpoint's liveness
/// rarely changes within a process's life, so the answer is taken once
/// and the rest of the session reads it.
pub fn probed_embedder() -> Box<dyn Embedder> {
    static PROBED: std::sync::OnceLock<Box<dyn Embedder>> = std::sync::OnceLock::new();
    PROBED
        .get_or_init(|| probe(&embedding_endpoint()).embedder())
        .box_clone()
}

/// The suite's one live-embedder gate — KI-340's class, widened
/// 2026-10-29: the machine has one embedding backend, and parallel
/// test threads queued behind it until the ten-second embed timeout
/// outran them (the deck tool called a busy backend "absent", the rank
/// route's handler outlived its client's budget and the reply came
/// back empty). A lib test that embeds against the live endpoint holds
/// this lock, so the queue the backend sees is the cron driver's
/// alone. The lock is tokio's own: the gated server tests hold it
/// across the request's await, which a std guard may not — clippy's
/// rule, and the reason there are two ways in (the async tests await
/// it, the plain `#[test]`s take it blocking). Tests in another binary
/// need no gate of their own: cargo runs test binaries one at a time.
#[cfg(test)]
pub(crate) static EMBED_GATE: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// The gate for an async test — held across the request the test
/// makes, dropped when the guard drops.
#[cfg(test)]
pub(crate) async fn embed_gate() -> tokio::sync::MutexGuard<'static, ()> {
    EMBED_GATE.lock().await
}

/// The gate for a plain `#[test]`, which has no runtime to await on.
/// `blocking_lock` panics inside an async context, and a plain test
/// is never one.
#[cfg(test)]
pub(crate) fn embed_gate_blocking() -> tokio::sync::MutexGuard<'static, ()> {
    EMBED_GATE.blocking_lock()
}

impl Memory {
    /// Load the corpus under `root` with the vector path probed: the
    /// real local-model backend when it answers, keyword-only otherwise.
    /// The endpoint is `FLEET_EMBEDDING_ENDPOINT` when set, the loopback
    /// default otherwise.
    pub fn load(root: &Path) -> Memory {
        Memory::load_with(root, probed_embedder())
    }

    /// Load with an explicit embedder — the seam the tests and a future
    /// configured endpoint use. The index is built here: a fact whose
    /// stamp matches its cached entry keeps its vector, every other
    /// fact is embedded once, and the cache is rewritten only when
    /// something changed. The cache is keyed to the embedder: one
    /// written by another model — or by the pre-key format — is
    /// invalidated whole and rebuilt, so a model swap can never serve
    /// stale-dimension vectors that cosine would score 0.0.
    pub fn load_with(root: &Path, embedder: Box<dyn Embedder>) -> Memory {
        let facts = load_facts(root);
        let mut vectors: HashMap<String, Vec<f32>> = HashMap::new();
        // KI-249: the load-time index build's per-embed wall clocks,
        // collected across the block below and carried on the Memory for
        // the caller's stream to record.
        let mut build_embeds_ms: Vec<u128> = Vec::new();
        if embedder.serves_vectors() {
            let key = embedder.vector_info();
            let index = Index::read(root);
            let index = match &key {
                Some(key) if index.keyed_for(key) => index,
                _ => Index::default(),
            };
            let mut fresh: HashMap<String, IndexEntry> = HashMap::new();
            let mut changed = false;
            for fact in &facts {
                // KI-228: the entry is keyed by the digest of the text
                // the vector was computed from, not by the file's stamp.
                // A recall-count bump rewrites the file in place and so
                // moves its mtime, and a stamp-keyed entry re-embeds a
                // fact whose content never changed — which is how the
                // corpus's three most-recalled facts came to be
                // re-embedded on essentially every load.
                let digest = indexed_digest(fact);
                let cached = index
                    .facts
                    .get(&fact.name)
                    .filter(|entry| entry.digest == digest)
                    .map(|entry| entry.vector.clone());
                let vector = match cached {
                    Some(vector) => vector,
                    None => {
                        let started = std::time::Instant::now();
                        let embedded = embedder.embed(&fact.indexed_text());
                        build_embeds_ms.push(started.elapsed().as_millis());
                        match embedded {
                            Some(vector) => {
                                changed = true;
                                vector
                            }
                            None => {
                                // Never silent: the fact keeps no vector
                                // this load and one line says so, by
                                // name \u{2014} the keyword channels
                                // still rank it.
                                eprintln!(
                                    "fleet: memories: `{}` \u{2014} the embed failed, so it carries no vector this load",
                                    fact.name
                                );
                                continue;
                            }
                        }
                    }
                };
                fresh.insert(
                    fact.name.clone(),
                    IndexEntry {
                        digest,
                        vector: vector.clone(),
                    },
                );
                vectors.insert(fact.name.clone(), vector);
            }
            // KI-231: rewrite only for a reason that is not a retry. A
            // fact whose embed failed is absent from `fresh`, and the
            // old `fresh.len() != index.facts.len()` condition read
            // that as a change and rewrote the pruned map — so a fact
            // whose text changed while the embedding server was down
            // was re-attempted, re-failed, and re-written on every
            // load, forever. A failed embed costs a retry next load and
            // no write; an entry for a fact that no longer exists is
            // the one thing that must still be pruned. A fact that is
            // still here is never an orphan, however its embed went —
            // its old entry keeps its slot and can never be served,
            // because the entry is keyed by digest.
            let orphaned = index
                .facts
                .keys()
                .any(|name| !facts.iter().any(|fact| &fact.name == name));
            if changed || orphaned {
                Index::write(root, key.as_ref(), &fresh);
            }
        }
        let graph = crate::recall_graph::Graph::build(&facts);
        Memory {
            root: root.to_path_buf(),
            embedder,
            facts: Mutex::new(facts),
            vectors,
            graph,
            two_stage: std::sync::atomic::AtomicBool::new(false),
            build_embeds_ms,
        }
    }

    /// Build a Memory over facts that did not come from a corpus root —
    /// the session archive's synthetic ones — with the vectors already
    /// computed for them and the embedder that computed them.
    ///
    /// [`Memory::load_with`] is the corpus root's path: it reads the
    /// facts off disk and embeds what its index does not already hold.
    /// The session archive derives its facts in memory from JSONL logs
    /// and caches their vectors in the sessions directory's own index,
    /// so it arrives with the corpus already in hand and needs the
    /// same Memory without the disk read. The vectors ride in rather
    /// than being recomputed because the archive's index has already
    /// done the work, and the embedder rides in because the vector
    /// channel needs the instruction's own embedding from the same
    /// model that computed the facts' — a query embedded by any other
    /// model would cosine against stale-dimension vectors and score
    /// 0.0, the exact failure the index's embedder key exists to
    /// prevent.
    ///
    /// Read-only by construction: the recall-count bump writes to a
    /// fact's `path`, and a synthetic fact's path is the session log
    /// it was derived from, so a synthetic Memory must never be routed
    /// through the hit-recording path — [`Memory::recall_hits`] or
    /// [`Memory::record_hits`] — the session archive's own
    /// feedback loop is a later piece of its plan. `root` is empty
    /// because synthetic facts have no corpus root; the caller already
    /// holds the sessions directory they came from.
    pub fn from_facts(
        facts: Vec<Fact>,
        vectors: HashMap<String, Vec<f32>>,
        embedder: Box<dyn Embedder>,
    ) -> Memory {
        let graph = crate::recall_graph::Graph::build(&facts);
        Memory {
            root: PathBuf::new(),
            embedder,
            facts: Mutex::new(facts),
            vectors,
            graph,
            two_stage: std::sync::atomic::AtomicBool::new(false),
            build_embeds_ms: Vec::new(),
        }
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

    /// The vector this corpus holds for one fact, by name — `None` when
    /// the fact is unknown or the embedder served no vector for it this
    /// load. The related index reads the recall cache's work rather
    /// than re-embedding the corpus: this is the one accessor that
    /// turns the cache inside [`Memory::load_with`] into a read.
    pub fn vector_of(&self, name: &str) -> Option<Vec<f32>> {
        self.vectors.get(name).cloned()
    }

    /// The protected tier: every non-forgotten protected fact, sorted by
    /// name. These enter a turn unconditionally — the budget never
    /// evicts them. Their score is [`Score::unranked`]: the tier did not
    /// compete, so no standing is computed for it and none is rendered.
    pub fn protected(&self) -> Vec<ScoredFact> {
        let today = calib::today();
        let facts = lock_facts(&self.facts);
        let mut hits: Vec<ScoredFact> = facts
            .iter()
            .filter(|fact| fact.protected() && !fact.forgotten(&today))
            .map(|fact| ScoredFact {
                fact: fact.clone(),
                score: Score::unranked(),
                off_shape: false,
                via: None,
            })
            .collect();
        hits.sort_by(|left, right| left.fact.name.cmp(&right.fact.name));
        hits
    }

    /// One instruction's shape-partitioned ranking: the shape's own
    /// pool and the off-shape spillover, each strongest first with the
    /// name as the deterministic tie-break.
    ///
    /// The instruction classifies into a shape — or into nothing, in
    /// which case every fact is primary: the word-ranked pool IS the
    /// primary pool, the documented degradation. The classification's
    /// failure must never starve recall; it only fails to reorder it.
    /// A fact carrying the classified shape is primary — a fact with
    /// several tags is in the pool when any of them matches, so a fact
    /// carrying both a structure shape and a failure shape answers
    /// either axis — and every other fact above the floor is spillover,
    /// marked as such.
    pub fn partition(&self, instruction: &str) -> Partition {
        let shape = classify_shape(instruction);
        let query_vector = self.embedder.embed(instruction);
        // The two-stage prefilter, when the setting is on: the slice
        // stage's top-K is the only set whose vector term survives.
        let prefilter = if self.two_stage.load(std::sync::atomic::Ordering::Relaxed) {
            vector_prefilter(&query_vector, &self.vectors, PREFILTER_K)
        } else {
            None
        };
        // Stemmed, like the fact side: both halves of every lexical
        // comparison are reduced the same way, so the channel stays
        // symmetric and neither side gains from the rule (R3).
        let query_tokens = stems(instruction);
        let today = calib::today();
        let today_days = parse_day(&today);
        let facts = lock_facts(&self.facts);
        let mut primary: Vec<ScoredFact> = Vec::new();
        let mut spillover: Vec<ScoredFact> = Vec::new();
        for fact in facts.iter().filter(|fact| {
            !fact.protected() && fact.tier != Tier::Working && !fact.forgotten(&today)
        }) {
            let score = self.score(
                fact,
                &query_vector,
                &query_tokens,
                today_days,
                prefilter.as_ref(),
            );
            if score.total < MIN_RECALL_SCORE {
                continue;
            }
            let in_pool = match shape {
                Some(shape) => fact.carries_shape(shape),
                None => true,
            };
            let hit = ScoredFact {
                fact: fact.clone(),
                score,
                off_shape: !in_pool,
                via: None,
            };
            if in_pool {
                primary.push(hit);
            } else {
                spillover.push(hit);
            }
        }
        by_rank(&mut primary);
        by_rank(&mut spillover);
        let query = Query {
            vector: &query_vector,
            tokens: &query_tokens,
            today_days,
            prefilter: prefilter.as_ref(),
        };
        let graph = self.graph_hits(&facts, &primary, &spillover, &query, &today);
        Partition {
            shape,
            primary,
            spillover,
            graph,
        }
    }

    /// One bounded hop over the frontmatter's own edges. See
    /// [`crate::recall_graph`] for why this exists, what it deliberately
    /// does not walk, and why a retired fact is a source rather than a
    /// hit.
    ///
    /// A neighbour enters on its source's authority rather than its own
    /// score — that is the whole point, since a fact that cleared the
    /// floor on its own would already be in a pool — so the bar is the
    /// source's own total against [`MIN_RECALL_SCORE`], the same floor
    /// every other fact faces, and the neighbour's score rides along
    /// unchanged and honest. Every neighbour is marked with the edge that
    /// reached it, so the assembly never mixes one silently into the
    /// shape's answer.
    fn graph_hits(
        &self,
        facts: &[Fact],
        primary: &[ScoredFact],
        spillover: &[ScoredFact],
        query: &Query<'_>,
        today: &str,
    ) -> Vec<ScoredFact> {
        use crate::recall_graph::{GRAPH_HOPS, GRAPH_SOURCES};
        let mut out: Vec<ScoredFact> = Vec::new();
        // Only the hop count gates this. The retired-source path is
        // independent of the pools — a corpus whose only match is a
        // fact the cycle folded away has an EMPTY primary pool, and that
        // is precisely the case where the replacement must still surface.
        if GRAPH_HOPS == 0 {
            return out;
        }
        let mut expand = |edge_owner: &str| {
            for neighbour in self.graph.neighbours(edge_owner) {
                // Already in a pool, or already pulled in by an earlier
                // source: no fact enters a turn twice.
                let taken = primary.iter().any(|hit| hit.fact.name == neighbour.name)
                    || spillover.iter().any(|hit| hit.fact.name == neighbour.name)
                    || out.iter().any(|hit| hit.fact.name == neighbour.name);
                if taken {
                    continue;
                }
                let Some(fact) = facts.iter().find(|fact| fact.name == neighbour.name) else {
                    continue;
                };
                // The same exclusions the pools run on: a protected fact
                // enters through the protected tier and a working one
                // through the working tier. A retired fact is not a hit —
                // it is out of recall by design, and it only ever vouches.
                if fact.protected() || fact.tier == Tier::Working || fact.forgotten(today) {
                    continue;
                }
                let score = self.score(
                    fact,
                    query.vector,
                    query.tokens,
                    query.today_days,
                    query.prefilter,
                );
                out.push(ScoredFact {
                    fact: fact.clone(),
                    score,
                    off_shape: false,
                    via: Some(neighbour.edge.label()),
                });
            }
        };
        // The primary pool's head first: the shape's own strongest
        // answers are the sources that earned the authority.
        for source in primary.iter().take(GRAPH_SOURCES) {
            expand(&source.fact.name);
        }
        // Then the retired facts that vouch. A fact the corpus folded
        // away never enters a pool, so without this its replacement is
        // unreachable for any instruction worded like the old fact.
        for name in self.graph.sources() {
            let Some(fact) = facts.iter().find(|fact| &fact.name == name) else {
                continue;
            };
            if !fact.forgotten(today) {
                continue;
            }
            let score = self.score(
                fact,
                query.vector,
                query.tokens,
                query.today_days,
                query.prefilter,
            );
            if score.total < MIN_RECALL_SCORE {
                continue;
            }
            expand(name);
        }
        out
    }

    /// Ranked recall for one instruction: the shape-partitioned
    /// ranking, flattened — the primary pool first, then the spillover,
    /// each strongest first with the name as the deterministic
    /// tie-break. Protected facts excluded (they are assembled
    /// through [`Memory::protected`] — no fact enters a turn twice),
    /// working-tier facts excluded (they are assembled through
    /// [`Memory::working`], the same rule one tier over), forgotten
    /// facts excluded, and everything below [`MIN_RECALL_SCORE`]
    /// dropped as noise for this instruction. [`Memory::partition`] is
    /// the same ranking with the two pools kept apart, which is what
    /// the assembly and the surface read.
    pub fn recall(&self, instruction: &str) -> Vec<ScoredFact> {
        let partition = self.partition(instruction);
        let mut hits = partition.primary;
        hits.extend(partition.spillover);
        // The graph's neighbours ride last: they did not compete, so
        // they never displace a fact that did.
        hits.extend(partition.graph);
        hits
    }

    /// The working tier: every non-forgotten working fact that is not
    /// already protected (a protected fact enters through the protected
    /// tier — no fact enters a turn twice), most recent first. This is
    /// the slice the turn path hands [`Memory::assemble`], which places
    /// it after the protected tier and ranked recall and bounds it by
    /// the budget like everything else.
    pub fn working(&self) -> Vec<Fact> {
        let today = calib::today();
        let mut facts: Vec<Fact> = lock_facts(&self.facts)
            .iter()
            .filter(|fact| {
                fact.tier == Tier::Working && !fact.protected() && !fact.forgotten(&today)
            })
            .cloned()
            .collect();
        // Most recent first: recent working memory is the point of the
        // tier, and the name is the deterministic tie-break.
        facts.sort_by(|left, right| {
            right
                .date
                .cmp(&left.date)
                .then_with(|| left.name.cmp(&right.name))
        });
        facts
    }

    /// Rank an instruction and record what it surfaced, in one call:
    /// every hit updates the fact's `recall-count` in its file (a
    /// bounded, frontmatter-preserving rewrite), re-ranks on the next
    /// recall, and appends one `recall_hit` calibration row naming the
    /// fact, the shape, and the score. This is the direct caller's
    /// entry point, where ranking and recording are one step; the
    /// agent loop instead records the assembly's own hits through
    /// [`Memory::record_hits`], after the model's first answer, so a
    /// hit there means the model saw the fact (KI-068). The rewrite is
    /// best-effort: a file that cannot be rewritten logs one line and
    /// the hit is still recorded — the audit trail must not depend on
    /// the corpus being writable.
    pub fn recall_hits(
        &self,
        instruction: &str,
        stream: Option<&calib::Stream>,
    ) -> Vec<ScoredFact> {
        let hits = self.recall(instruction);
        self.record_hits(&hits, classify_shape(instruction), stream);
        hits
    }

    /// Record the hits an assembly actually surfaced — the agent loop's
    /// call, with the assembly's `recalled` slice, made after the first
    /// successful model response rather than at assembly time (KI-068):
    /// the bump and the row mean the model saw the fact, and a run
    /// whose call fails before any response records nothing. One
    /// ranking per turn, and the row's score is the score the fact
    /// entered the turn at, which is what makes the stream's numbers
    /// mean the same thing the assembled block does. A fact the budget
    /// dropped never reached the model, so it is not a hit.
    pub fn record_hits(
        &self,
        hits: &[ScoredFact],
        shape: Option<&str>,
        stream: Option<&calib::Stream>,
    ) {
        let shape = shape.unwrap_or_default().to_string();
        for hit in hits {
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
    }

    /// One fact's score against a query. The vector channel needs both
    /// vectors; the keyword channel needs the topic text's tokens; the
    /// trajectory channel needs the recorded phrases; the rest is the
    /// fact's own. The shape is deliberately not an argument: it
    /// partitioned the pool upstream, so no term here can reorder
    /// across it.
    fn score(
        &self,
        fact: &Fact,
        query_vector: &Option<Vec<f32>>,
        query_tokens: &[String],
        today_days: Option<i64>,
        prefilter: Option<&std::collections::HashSet<String>>,
    ) -> Score {
        // KI-252 move 2, when the prefilter is on: a fact the 128-dim
        // stage ranked out reads 0.0 for its vector term — the same
        // shape a partial embed leaves — and the top-K members keep the
        // full-width cosine the final order is ranked on.
        let sliced_out = matches!(prefilter, Some(members) if !members.contains(&fact.name));
        let vector = if sliced_out {
            0.0
        } else {
            match (query_vector, self.vectors.get(&fact.name)) {
                (Some(query), Some(fact_vector)) => cosine(query, fact_vector).max(0.0),
                _ => 0.0,
            }
        };
        let keyword = keyword_coverage(query_tokens, &stems(&fact.topic_text()));
        let trajectory = trajectory_coverage(query_tokens, &fact.phrases);
        Score::compute(
            vector,
            keyword,
            trajectory,
            fact.recall_count,
            fact.basis,
            &fact.date,
            today_days,
        )
    }

    /// Add one to the fact's `recall-count`: in memory first (so the
    /// next recall in this process re-ranks), then in its file. Returns
    /// whether the file rewrite landed — best-effort by design: the
    /// count is a rank signal, not an audit record, and a hit the model
    /// saw is not un-seen by a failed rewrite (the row still lands).
    fn bump_recall_count(&self, name: &str) -> bool {
        let (path, count) = {
            let mut facts = lock_facts(&self.facts);
            let Some(fact) = facts.iter_mut().find(|fact| fact.name == name) else {
                return false;
            };
            fact.recall_count += 1;
            (fact.path.clone(), fact.recall_count)
        };
        rewrite_field(&path, "recall-count", &count.to_string()).is_ok()
    }
}

/// One instruction's recall, partitioned by the classified shape: the
/// shape's own pool first, the off-shape spillover second, each
/// strongest first. [`Memory::recall`] is this, flattened.
#[derive(Debug, Clone, PartialEq)]
pub struct Partition {
    /// The classified shape, or `None` for the surface-alone fallback —
    /// in which case every fact above the floor is primary.
    pub shape: Option<&'static str>,
    /// The facts carrying the shape, strongest first.
    pub primary: Vec<ScoredFact>,
    /// The facts outside the shape, strongest first, each marked
    /// `off_shape`.
    pub spillover: Vec<ScoredFact>,
    /// The facts one bounded graph hop reached, each marked with the
    /// edge that reached it. Ordered by the source that vouched for
    /// them, then by name — they did not compete, so they do not rank.
    pub graph: Vec<ScoredFact>,
}

impl Partition {
    /// The primary pool's best score, or `None` when the pool is empty
    /// — the value [`SPILLOVER_FLOOR`] is read against.
    pub fn best(&self) -> Option<f64> {
        self.primary.first().map(|hit| hit.score.total)
    }
}

/// Strongest first, trajectory as the second key, name as the last —
/// the one ordering every pool in this module ranks by.
///
/// The trajectory term is the second key because the name alone decided
/// exact ties arbitrarily: `own-home-credentials-ruling` and
/// `router-enforcement-ruling` scored identically (kw 0.060, traj 0.120,
/// hist 0.250, fresh 0.098, prov 0.050) and the alphabet picked the
/// wrong one. Within a tie, the fact that drove is the better answer —
/// the same direction-outranks-proximity ruling, applied to the
/// tie-break. The name stays last so the ordering is still fully
/// deterministic, which is what the surface's reproducibility needs.
fn by_rank(hits: &mut [ScoredFact]) {
    hits.sort_by(|left, right| {
        right
            .score
            .total
            .total_cmp(&left.score.total)
            .then_with(|| right.score.trajectory.total_cmp(&left.score.trajectory))
            .then_with(|| left.fact.name.cmp(&right.fact.name))
    });
}

/// One instruction's ranking context, derived once per recall and shared
/// by the pools and the graph hop so neither re-derives it — and so the
/// hop takes the query as one value rather than as four loose
/// arguments.
struct Query<'a> {
    /// The instruction's own embedding, or `None` when the backend
    /// serves no vectors.
    vector: &'a Option<Vec<f32>>,
    /// The instruction's stems, the same reduction the fact side takes.
    tokens: &'a [String],
    /// Days since the epoch for today, or `None` when the clock is
    /// unreadable.
    today_days: Option<i64>,
    /// The two-stage prefilter's surviving names (KI-252 move 2), when
    /// the setting is on; `None` ranks full-width.
    prefilter: Option<&'a std::collections::HashSet<String>>,
}

/// One fact ranked for one instruction: the fact, the score that
/// ordered it, and how it entered.
#[derive(Debug, Clone, PartialEq)]
pub struct ScoredFact {
    pub fact: Fact,
    pub score: Score,
    /// True when the fact entered through the spillover rather than the
    /// shape's own pool — the weaker context the assembly marks
    /// `off-shape` in its line.
    pub off_shape: bool,
    /// The graph edge that reached this fact, when it entered on another
    /// fact's authority rather than its own score — the assembly marks
    /// the line with it, so a graph-sourced fact is never silently mixed
    /// into the shape's own answer.
    pub via: Option<&'static str>,
}

/// The bounded, frontmatter-preserving field rewrite: the one line the
/// rewrite owns is re-rendered, and every other byte — the body above
/// all — is preserved. The write lands through a unique temp file and a
/// rename, so a reader never sees a half-written fact.
///
/// One rewriter for every in-place edit the module makes — the
/// recall-count bump, the `superseded-by` stamp, the `valid-until`
/// stamp — because a second rewrite path would drift from this one's
/// bound and its atomicity. A line the fact does not carry is inserted
/// Take the lock on one fact file's writes, released on drop. KI-062:
/// the lock is what makes a fact's read-modify-write atomic across the
/// consolidation cycle and a live run's recall-count bump. A sibling
/// `.lock` beside the fact carries it; the lock BLOCKS, so a caller must
/// not hold one across a call that rewrites the same file.
///
/// The mechanism is [`crate::filelock::FileLock`], shared with the
/// session archive's count store rather than copied: two `unsafe
/// extern "C"` blocks declaring `flock` in two modules is two places for
/// the same primitive to drift.
fn lock_fact(path: &Path) -> crate::filelock::FileLock {
    crate::filelock::FileLock::take(&crate::filelock::FileLock::beside(path))
}

/// as the last frontmatter line rather than appended anywhere else.
///
/// The failure is a typed error, not a swallowed `bool`: the
/// supersession path records its action only once its stamp has
/// actually landed, so the audit trail cannot claim a retirement the
/// file never took. The best-effort callers (the recall-count bump,
/// the cycle's stamps) keep their own logging and their own
/// consequence for a failed rewrite.
fn rewrite_field(path: &Path, key: &str, value: &str) -> Result<(), WriteError> {
    // KI-062: the whole read-modify-write runs under a per-fact lock.
    // The consolidation cycle's bounded rewrites and a concurrent run's
    // recall-count bump both re-read, edit one line, and rename over the
    // file; without the lock one of the two updates is lost — the
    // count is a rank signal rather than an audit record, so the
    // honest first version left it, and this is the lock that closes
    // it. The lock file is a sibling beside the fact, never inside the
    // corpus a walk would read.
    let _guard = lock_fact(path);
    let Ok(text) = fs::read_to_string(path) else {
        let reason = format!(
            "could not re-read {} to rewrite its `{key}`",
            path.display()
        );
        eprintln!("fleet: memories: {reason}");
        return Err(WriteError::Io(reason));
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
        let reason = format!(
            "{} lost its frontmatter fence; `{key}` was not rewritten",
            path.display()
        );
        eprintln!("fleet: memories: {reason}");
        return Err(WriteError::Io(reason));
    };
    let mut out_lines: Vec<String> = lines.iter().map(|line| line.to_string()).collect();
    let rendered = format!("{key}: {value}");
    let mut written = false;
    for line in out_lines.iter_mut().take(closing).skip(1) {
        let field = line
            .split_once(':')
            .map(|(field, _)| field.trim())
            .unwrap_or_default();
        if field == key {
            *line = rendered.clone();
            written = true;
            break;
        }
    }
    if !written {
        out_lines.insert(closing, rendered);
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
                Ok(())
            } else {
                let _ = fs::remove_file(&temp);
                let reason = format!("could not replace {} with its new `{key}`", path.display());
                eprintln!("fleet: memories: {reason}");
                Err(WriteError::Io(reason))
            }
        }
        Err(error) => {
            let _ = fs::remove_file(&temp);
            let reason = format!("could not write {}: {error}", temp.display());
            eprintln!("fleet: memories: {reason}");
            Err(WriteError::Io(reason))
        }
    }
}

/// Append one note to a fact's body, preserving everything already
/// there. The forget path's record of why a fact was retired: the file
/// is the record, and a retirement with no note is a fact that looks
/// like it expired on its own. The failure is typed like
/// [`rewrite_field`]'s, for the same reason: the forget path records
/// its action only once both the stamp and the note have landed.
fn append_note(path: &Path, note: &str) -> Result<(), WriteError> {
    let Ok(mut text) = fs::read_to_string(path) else {
        let reason = format!(
            "could not re-read {} to append its retirement note",
            path.display()
        );
        eprintln!("fleet: memories: {reason}");
        return Err(WriteError::Io(reason));
    };
    if !text.ends_with('\n') && !text.is_empty() {
        text.push('\n');
    }
    text.push_str(note);
    if !text.ends_with('\n') {
        text.push('\n');
    }
    let serial = REWRITE_SERIAL.fetch_add(1, Ordering::Relaxed);
    let mut temp = path.to_path_buf().into_os_string();
    temp.push(format!(".{}.{}.tmp", std::process::id(), serial));
    let temp = PathBuf::from(temp);
    match fs::write(&temp, &text) {
        Ok(()) => {
            if fs::rename(&temp, path).is_ok() {
                Ok(())
            } else {
                let _ = fs::remove_file(&temp);
                let reason = format!(
                    "could not replace {} with its retirement note",
                    path.display()
                );
                eprintln!("fleet: memories: {reason}");
                Err(WriteError::Io(reason))
            }
        }
        Err(error) => {
            let _ = fs::remove_file(&temp);
            let reason = format!("could not write {}: {error}", temp.display());
            eprintln!("fleet: memories: {reason}");
            Err(WriteError::Io(reason))
        }
    }
}

/// A per-write serial for the rewrite's temp name, so two rewrites in
/// one process never share a temp file.
static REWRITE_SERIAL: AtomicUsize = AtomicUsize::new(0);

// --- assembly --------------------------------------------------------------

/// The turn-context budget in tokens, estimated at one token per four
/// characters — the fork's own estimate, ported. It bounds what enters
/// a turn; it can never evict the protected tier.
///
/// 2048 against DeepSeek's 128k context: the fork's 512 was tuned for
/// a corpus whose protected tier was a handful of short lines, and on
/// the seed corpus it starved recall — the seven protected facts cost
/// ~450 tokens on their own, so at 512 the second ranked fact (~85
/// tokens) did not fit and ranked recall got exactly one fact. The
/// floor (`MIN_RECALL_SCORE`) already does the coarse noise control;
/// the budget is the fine one, and 2048 bounds a turn to ~25-30 facts
/// while letting the protected tier and ranked recall coexist.
pub const ASSEMBLY_TOKEN_BUDGET: usize = 2048;

/// One turn's assembled context: the protected tier (never evicted),
/// the shape-first recall that fit the budget — its primary pool and
/// its marked spillover — the working memory that fit after it, and
/// what the budget dropped — named, so the surface can be honest
/// about it.
#[derive(Debug, Clone, PartialEq)]
pub struct Assembly {
    /// The classified shape, or `None` for the surface-alone fallback.
    pub shape: Option<&'static str>,
    /// Every fact above the floor, the primary pool first then the
    /// spillover, each strongest first — the full ranking, before the
    /// budget applies.
    pub ranked: Vec<ScoredFact>,
    /// The protected tier, name-sorted.
    pub protected: Vec<ScoredFact>,
    /// The ranked facts that entered, in rank order: the primary pool
    /// first, then the spillover that a route admitted. This is what
    /// the turn path records hits for and what the block renders.
    pub recalled: Vec<ScoredFact>,
    /// The primary-pool facts that entered, in rank order.
    pub primary: Vec<ScoredFact>,
    /// The off-shape facts that entered, in rank order — the marked
    /// half of `recalled`.
    pub spillover: Vec<ScoredFact>,
    /// The graph's neighbours that entered, in the order the sources
    /// vouched for them — each marked with the edge that reached it.
    /// Not part of `recalled`: a graph-sourced fact never records a
    /// recall hit, because it did not rank.
    pub graph: Vec<ScoredFact>,
    /// Which of the spillover's three routes admitted them, when any
    /// entered — the named reason the surface prints.
    pub spillover_route: Option<String>,
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
    /// protected tier first, then shape-first recall, then the caller's
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
    ///
    /// **The spillover's three routes** (slice 19). The primary pool
    /// always leads — the shape's own answer is what the turn is about.
    /// The off-shape facts enter only through one of three named
    /// routes: (a) the primary pool is empty, so they are the answer;
    /// (b) the pool's best is below [`SPILLOVER_FLOOR`], so the shape
    /// matched without corroborating itself and the strongest off-shape
    /// fact enters even under budget pressure, beside the weak pool;
    /// (c) the budget has room after the pool is in. Every off-shape
    /// fact that enters is marked `off-shape` in its line. Never
    /// silently mixed.
    pub fn assemble_within(&self, instruction: &str, working: &[Fact], budget: usize) -> Assembly {
        let partition = self.partition(instruction);
        let shape = partition.shape;
        let protected = self.protected();
        let ranked = self.ranked_of(&partition);
        let today = calib::today();

        // The three routes, decided once and named for the surface.
        let spillover_route = if partition.spillover.is_empty() {
            None
        } else if partition.primary.is_empty() {
            Some(spillover_route_empty(shape))
        } else if partition.best().unwrap_or(0.0) < SPILLOVER_FLOOR {
            Some(spillover_route_weak(shape, partition.best().unwrap_or(0.0)))
        } else {
            Some(spillover_route_room(shape))
        };

        let mut used = 0;
        for hit in &protected {
            used += estimate_tokens(protected_line(hit).chars().count());
        }
        let mut recalled: Vec<ScoredFact> = Vec::new();
        let mut primary: Vec<ScoredFact> = Vec::new();
        let mut spillover: Vec<ScoredFact> = Vec::new();
        let mut dropped: Vec<String> = Vec::new();
        // The primary pool first: the shape's own answer leads whatever
        // the words say, and its strongest fact always enters.
        for (index, hit) in partition.primary.iter().enumerate() {
            let cost = estimate_tokens(recalled_line(hit).chars().count());
            if index == 0 || used + cost <= budget {
                used += cost;
                recalled.push(hit.clone());
                primary.push(hit.clone());
            } else {
                dropped.push(hit.fact.name.clone());
            }
        }
        // Then the spillover, by the route that admitted it. Routes (a)
        // and (b) put the off-shape facts in the running from the start,
        // and their strongest fact still enters under budget pressure —
        // the fork's `renderBlock` rule applied to the marked half,
        // because a weak shape answer plus one marked weaker fact beats
        // a weak shape answer alone. Route (c) admits them only while
        // the budget has room after the pool is in.
        let weak_pool =
            partition.primary.is_empty() || partition.best().unwrap_or(0.0) < SPILLOVER_FLOOR;
        for (index, hit) in partition.spillover.iter().enumerate() {
            let cost = estimate_tokens(recalled_line(hit).chars().count());
            let guaranteed = weak_pool && index == 0;
            if guaranteed || used + cost <= budget {
                used += cost;
                recalled.push(hit.clone());
                spillover.push(hit.clone());
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
        // The graph's neighbours last, and strictly budget-bounded like
        // everything that did not compete for its place. Their strongest
        // enters under pressure — the fork's `renderBlock` rule applied
        // here too, because a companion the trajectory needed is worth a
        // line even in a tight turn — and the rest enter while they fit.
        let mut kept_graph: Vec<ScoredFact> = Vec::new();
        for (index, hit) in partition.graph.iter().enumerate() {
            let cost = estimate_tokens(recalled_line(hit).chars().count());
            if index == 0 || used + cost <= budget {
                used += cost;
                kept_graph.push(hit.clone());
            } else {
                dropped.push(hit.fact.name.clone());
            }
        }
        Assembly {
            shape,
            ranked,
            protected,
            recalled,
            primary,
            spillover,
            graph: kept_graph,
            spillover_route,
            working: kept_working,
            dropped,
            used,
            budget,
        }
    }

    /// The partition flattened: primary pool first, then the spillover,
    /// each strongest first — the full ranking the surface cards. The
    /// graph's neighbours are not part of the ranking: they did not
    /// compete, so the surface cards them apart.
    fn ranked_of(&self, partition: &Partition) -> Vec<ScoredFact> {
        let mut ranked = partition.primary.clone();
        ranked.extend(partition.spillover.iter().cloned());
        ranked
    }
}

/// Route (a)'s named reason: the shape's own pool is empty, so the
/// off-shape facts are the answer.
fn spillover_route_empty(shape: Option<&str>) -> String {
    match shape {
        Some(shape) => {
            format!("no fact carries `{shape}`, so the off-shape facts are the answer")
        }
        None => "the word-ranked pool is the primary pool".to_string(),
    }
}

/// Route (b)'s named reason: the pool's best is below the floor, so
/// the shape matched without corroborating itself.
fn spillover_route_weak(shape: Option<&str>, best: f64) -> String {
    match shape {
        Some(shape) => format!(
            "the `{shape}` pool's best ({best:.2}) is below the SPILLOVER_FLOOR ({SPILLOVER_FLOOR:.2})"
        ),
        None => "the word-ranked pool is the primary pool".to_string(),
    }
}

/// Route (c)'s named reason: the budget had room after the pool was in.
fn spillover_route_room(shape: Option<&str>) -> String {
    match shape {
        Some(shape) => format!("the budget had room after the `{shape}` pool was in"),
        None => "the word-ranked pool is the primary pool".to_string(),
    }
}

/// One token per four characters, rounded up — the fork's estimate.
fn estimate_tokens(chars: usize) -> usize {
    chars.div_ceil(4)
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

/// One recalled fact as the assembled context carries it: its sign, the
/// shape tag the line names, the score it ranked at, its basis — and,
/// for an off-shape fact, the `off-shape` marker. The sign comes first
/// so the model can never read an anti-vector as a candidate solution: a
/// `diverged:` line is what drove AWAY from this problem, not a
/// proposal to adopt. An incomplete vector's missing companions ride
/// the same line, because the whole point of the third sign is that the
/// absent pieces are named rather than lost.
fn recalled_line(hit: &ScoredFact) -> String {
    let tag = line_tag(&hit.fact);
    let marker = if hit.off_shape { " off-shape" } else { "" };
    let mut line = format!(
        "- {} [{tag} {:.2} {}{}] {}",
        hit.fact.sign().marker(),
        hit.score.total,
        hit.fact.basis.label(),
        marker,
        hit.fact.description
    );
    if let Some(missing) = missing_note(&hit.fact) {
        line.push_str(&format!(" \u{b7} missing: {missing}"));
    }
    // The edge that reached a graph-sourced fact rides the same line, so
    // a fact that entered on another fact's authority is never read as
    // one that ranked.
    if let Some(edge) = hit.via {
        line.push_str(&format!(" \u{b7} via: {edge}"));
    }
    line
}

/// One fact's missing companions as one line, or `None` when it carries
/// none. Only an incomplete vector has any, so this is also the sign's
/// own honesty check: a `missing:` note on a fact that reads `resolved:`
/// would be a contradiction the tags forbid.
fn missing_note(fact: &Fact) -> Option<String> {
    (!fact.missing.is_empty()).then(|| fact.missing.join(", "))
}

/// The tag one assembled line carries: the fact's first shape tag, or —
/// for an anti-vector — its failure-shape tag, because the class of
/// thing that broke is what the line has to name. A fact with no tags
/// rides the middle dot.
fn line_tag(fact: &Fact) -> &str {
    fact.shape
        .iter()
        .find(|tag| is_failure_shape(tag))
        .or_else(|| fact.shape.first())
        .map(String::as_str)
        .unwrap_or("\u{b7}")
}

/// One working-memory fact as the assembled context carries it.
fn working_line(fact: &Fact) -> String {
    format!("- [working {}] {}", fact.basis.label(), fact.description)
}

/// The legend the recall section carries when it holds anything other
/// than a resolution: the three signs, named, so a `diverged:` line is
/// never read as a candidate and an `incomplete:` one is never read as
/// done.
const SIGN_LEGEND: &str =
    "resolved: drove toward the answer \u{b7} diverged: drove away \u{2014} an anti-vector to avoid, not a candidate \u{b7} incomplete: drove partway and stalled \u{2014} the direction was right, the companions were missing\n";

/// Render an assembly as the block that enters a turn's context: the
/// protected tier first, then the recall section — named for its shape,
/// its axis, or named as the surface-alone fallback when nothing
/// classified — then the working tier. Sections with nothing in them
/// render nothing.
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
            Some(shape) if is_failure_shape(shape) => out.push_str(&format!(
                "Relevant memory (failure-shape recall for `{shape}` \u{2014} what broke before, what resolved it, and what stalled \u{2014} propose only, verify before relying):\n"
            )),
            Some(shape) if is_outcome_shape(shape) => out.push_str(&format!(
                "Relevant memory ({shape} recall \u{2014} the trajectories that ended this way \u{2014} propose only, verify before relying):\n"
            )),
            Some(shape) => out.push_str(&format!(
                "Relevant memory (shape-routed recall for `{shape}` \u{2014} propose only, verify before relying):\n"
            )),
            None => out.push_str(
                "Relevant memory (surface recall \u{2014} no shape matched, the documented fallback \u{2014} propose only, verify before relying):\n",
            ),
        }
        if assembly
            .recalled
            .iter()
            .any(|hit| hit.fact.sign() != Sign::Resolution)
        {
            out.push_str(SIGN_LEGEND);
        }
        for hit in &assembly.recalled {
            out.push_str(&recalled_line(hit));
            out.push('\n');
        }
    }
    if !assembly.graph.is_empty() {
        out.push_str(
            "Adjacent memory (reached through an edge the matched facts declare — the current truth a superseded fact points at, or a companion a trajectory was missing — propose only, verify before relying):\n",
        );
        for hit in &assembly.graph {
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

// --- the write path --------------------------------------------------------

/// The description's bound: the one-sentence fact an assembled turn
/// carries and a card renders. KI-060's fix — the fork's writer emits
/// one sanitized sentence, and a description with neither bound rides
/// control characters into the turn context and the surface and
/// inflates the assembly's token estimate without limit. The cut is
/// marked, so a truncated description says so.
pub const MAX_DESCRIPTION_CHARS: usize = 512;

/// The body's bound. A fact's full content is the file's, not the
/// context's, but an unbounded body is an unbounded write: past this
/// the write is refused rather than silently cut, because a fact that
/// lost its tail would read as whole.
pub const MAX_BODY_BYTES: usize = 64 * 1024;

/// The name's bound. The name is the file stem and the key every
/// surface and every calibration row addresses, so it is short by
/// construction.
pub const MAX_NAME_CHARS: usize = 64;

/// The type's bound. The type rides the assembled line and the card,
/// so it is bounded like the description.
pub const MAX_TYPE_CHARS: usize = 32;

/// How many shape tags the writer emits — part 1's reader reads
/// whatever the file carries; the cap is the writer's rule. The cap is
/// across BOTH axes: a fact may carry structure shapes and failure
/// shapes, and the total is what the frontmatter line and the
/// partition's matching stay readable at.
pub const MAX_SHAPE_TAGS: usize = 3;

/// How many resolution phrases the writer emits. The phrases are the
/// fact's direction — the vocabulary that drove — and a direction is a
/// handful of phrases, not a transcript.
pub const MAX_PHRASES: usize = 6;

/// One phrase's bound: it rides the indexed text and the trajectory
/// channel, so it is bounded like the description.
pub const MAX_PHRASE_CHARS: usize = 160;

/// How many missing companions the writer emits. The third vector's
/// absent pieces ride the same frontmatter line and the same surfaces
/// the phrases do, so they take the phrases' bound — a direction is a
/// handful of phrases and a stall is a handful of companions, not a
/// transcript of everything that was absent.
pub const MAX_MISSING: usize = 6;

/// One missing companion's bound: it rides the assembly line and the
/// card, so it is bounded like a phrase.
pub const MAX_MISSING_CHARS: usize = 160;

/// The justification's bound: it rides the calibration row and the
/// replacement fact's body, so it is bounded like the description.
pub const MAX_JUSTIFICATION_CHARS: usize = 512;

/// The type a write defaults to: the corpus's common kind, and a
/// deliberately unprotected one. A fact that marks identity or a
/// standing preference says so — the protected tier is never a
/// default.
pub const DEFAULT_TYPE: &str = "project";

/// Why a write was refused. A typed error, not a string: the tool
/// relays it to the model verbatim and the CLI prints it, and both
/// should name the same rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WriteError {
    /// The name is unusable as a file stem.
    Name(String),
    /// The description is missing or unusable.
    Description(String),
    /// The type is unusable.
    Type(String),
    /// A shape tag is outside the taxonomy, or there are too many.
    Shape(String),
    /// Too many phrases, or a phrase list that is not strings.
    Phrases(String),
    /// Too many missing companions, or a missing list that is not
    /// strings.
    Missing(String),
    /// The basis is missing or outside the vocabulary.
    Basis(String),
    /// The tier is outside the three speeds — `episodic` included,
    /// which only the consolidation cycle writes.
    Tier(String),
    /// `valid-until` is missing where the tier requires it, or is not
    /// a real day.
    ValidUntil(String),
    /// The body is past its bound.
    Body(String),
    /// The justification is missing.
    Justification(String),
    /// A fact of this name already exists — `remember` never
    /// overwrites; the modify path is `supersede`.
    Exists(String),
    /// No fact of this name exists.
    UnknownFact(String),
    /// The fact is already retired, so there is nothing to modify.
    Retired(String),
    /// A `confirmed-by` fact cannot be modified or forgotten without
    /// approval — the permission model, enforced at the module seam so
    /// a programmatic caller cannot bypass it either.
    NeedsApproval(String),
    /// A lifecycle stamp or a retirement note failed to land: the
    /// replacement fact is on disk but the old one is not retired, or
    /// the forget's expiry landed and its note did not. The audit
    /// cannot claim an action that did not happen, so the supersession
    /// fails with what landed and what did not, both named.
    Stamp(String),
    /// The write itself failed.
    Io(String),
}

impl std::fmt::Display for WriteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WriteError::Name(reason) => write!(f, "{reason}"),
            WriteError::Description(reason) => write!(f, "{reason}"),
            WriteError::Type(reason) => write!(f, "{reason}"),
            WriteError::Shape(reason) => write!(f, "{reason}"),
            WriteError::Phrases(reason) => write!(f, "{reason}"),
            WriteError::Missing(reason) => write!(f, "{reason}"),
            WriteError::Basis(reason) => write!(f, "{reason}"),
            WriteError::Tier(reason) => write!(f, "{reason}"),
            WriteError::ValidUntil(reason) => write!(f, "{reason}"),
            WriteError::Body(reason) => write!(f, "{reason}"),
            WriteError::Justification(reason) => write!(f, "{reason}"),
            WriteError::Exists(reason) => write!(f, "{reason}"),
            WriteError::UnknownFact(reason) => write!(f, "{reason}"),
            WriteError::Retired(reason) => write!(f, "{reason}"),
            WriteError::NeedsApproval(reason) => write!(f, "{reason}"),
            WriteError::Stamp(reason) => write!(f, "{reason}"),
            WriteError::Io(reason) => write!(f, "{reason}"),
        }
    }
}

impl std::error::Error for WriteError {}

/// Sanitize one line of a write: control characters removed — not
/// replaced, so a stripped escape sequence leaves its payload visible
/// and inert — then trimmed, then bounded with a marked cut. KI-060's
/// mechanism, in the writer where the fork's writer owns the
/// rendering.
fn sanitize_line(text: &str, bound: usize) -> String {
    let cleaned: String = text
        .chars()
        .filter(|character| !character.is_control())
        .collect();
    let cleaned = cleaned.trim();
    if cleaned.chars().count() > bound {
        let cut: String = cleaned.chars().take(bound).collect();
        format!("{cut}\u{2026}")
    } else {
        cleaned.to_string()
    }
}

/// Whether `name` is usable as a fact's file stem: lowercase letters,
/// digits, and hyphens, bounded, with no leading or trailing hyphen.
/// The rule is what keeps a name a key — no path separator, no `.` or
/// `..`, no dotfile, no space — so a write can never address a file
/// outside the corpus.
fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.chars().count() <= MAX_NAME_CHARS
        && name.chars().all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-'
        })
        && !name.starts_with('-')
        && !name.ends_with('-')
}

/// One structured write: the `brain_remember` shape the fork's writer
/// speaks, mechanism over prose. `basis` is required on every write and
/// `valid-until` on every working-tier one, because those two rules are
/// what make the corpus's provenance and lifecycle mean something.
/// `phrases` is the slice-19 addition: the vocabulary the resolution
/// drove with — and, for a failure, the vocabulary of the trajectory
/// that was tried and diverged. `missing` is the third vector's
/// addition: the companions an incomplete trajectory needed and did not
/// have, which is what keeps "the bootstrap was correct but the torch
/// stack was missing" from being recorded as a resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Remember {
    pub name: String,
    pub description: String,
    pub fact_type: String,
    pub shape: Vec<String>,
    /// The resolution's phrases: the trajectory's own vocabulary. Empty
    /// for a fact with no recorded direction.
    pub phrases: Vec<String>,
    /// The companions an incomplete trajectory was missing: the absent
    /// pieces that kept a correct direction from reaching the minima.
    /// Empty for a trajectory that either completed or was never
    /// recorded as stalled.
    pub missing: Vec<String>,
    pub basis: Basis,
    pub tier: Tier,
    /// ISO date after which the fact is stale; empty for a durable
    /// fact, which is the writer's default and means no expiry.
    pub valid_until: String,
    pub body: String,
}

impl Remember {
    /// Parse and validate the tool's (or the CLI's) arguments object.
    /// The keys are the frontmatter's own names — `name`, `description`,
    /// `type`, `shape`, `basis`, `tier`, `valid-until`, `body` — so one
    /// validator serves the model-visible tool and the CLI form.
    pub fn parse(arguments: &serde_json::Value) -> Result<Remember, WriteError> {
        let name = string_argument(arguments, "name")?
            .map(|name| name.to_ascii_lowercase())
            .unwrap_or_default();
        if !valid_name(&name) {
            return Err(WriteError::Name(format!(
                "`{name}` is not a fact name: lowercase letters, digits, and hyphens, up to {MAX_NAME_CHARS} characters, no leading or trailing hyphen"
            )));
        }
        let description = sanitize_line(
            &string_argument(arguments, "description")?.unwrap_or_default(),
            MAX_DESCRIPTION_CHARS,
        );
        if description.is_empty() {
            return Err(WriteError::Description(
                "the `description` argument is required: the fact as one sentence".to_string(),
            ));
        }
        let fact_type = match string_argument(arguments, "type")? {
            Some(text) => sanitize_line(&text, MAX_TYPE_CHARS),
            None => DEFAULT_TYPE.to_string(),
        };
        if fact_type.is_empty() {
            return Err(WriteError::Type(
                "the `type` argument is empty; omit it for the default".to_string(),
            ));
        }
        let shape = parse_shape(&string_argument(arguments, "shape")?)?;
        let phrases = parse_phrases(&string_array_argument(arguments, "phrases")?)?;
        let missing = parse_missing(&string_array_argument(arguments, "missing")?)?;
        let basis = match string_argument(arguments, "basis")? {
            Some(text) => Basis::parse(&text.to_ascii_lowercase()).ok_or_else(|| {
                WriteError::Basis(format!(
                    "`{text}` is not a basis: `basis` is required on every write and is confirmed-by or inferred"
                ))
            })?,
            None => {
                return Err(WriteError::Basis(
                    "the `basis` argument is required on every write: confirmed-by when the user stated or verified the fact, inferred when the session merely produced it".to_string(),
                ));
            }
        };
        let tier = match string_argument(arguments, "tier")? {
            Some(text) => {
                let parsed = Tier::parse(&text.to_ascii_lowercase()).ok_or_else(|| {
                    WriteError::Tier(format!(
                        "`{text}` is not a tier: `tier` is working or durable — episodic summaries are written by the consolidation cycle, never by a write"
                    ))
                })?;
                // Episodic is the cycle's tier: a hand-written summary
                // would put an unaudited fact in the recallable store
                // with no cycle pass behind it.
                if parsed == Tier::Episodic {
                    return Err(WriteError::Tier(
                        "`episodic` facts are written by the consolidation cycle, never by a write"
                            .to_string(),
                    ));
                }
                parsed
            }
            None => Tier::Durable,
        };
        let valid_until = match string_argument(arguments, "valid-until")? {
            Some(text) => {
                if !crate::cli::valid_calendar_day(&text) {
                    return Err(WriteError::ValidUntil(format!(
                        "`{text}` is not a day; use valid-until YYYY-MM-DD"
                    )));
                }
                text
            }
            None => String::new(),
        };
        if tier == Tier::Working && valid_until.is_empty() {
            return Err(WriteError::ValidUntil(
                "a working-tier fact is session-scoped, so `valid-until` is required: the day it stops being true".to_string(),
            ));
        }
        let body = string_argument(arguments, "body")?.unwrap_or_default();
        if body.len() > MAX_BODY_BYTES {
            return Err(WriteError::Body(format!(
                "the `body` argument is {} bytes, past the {MAX_BODY_BYTES}-byte bound",
                body.len()
            )));
        }
        // R4b: the third sign is only real if the companions are named.
        // `missing` was optional and unpopulated — 15 corpus facts carry
        // the field and all 15 carry it empty — so an `incomplete` fact
        // rendered `incomplete:` with nothing to finish the trajectory
        // with, and the graph had no edge to walk. The tag now has to
        // earn its marker: a fact that says it stalled must say what it
        // was missing.
        if shape.iter().any(|tag| tag == INCOMPLETE_TAG) && missing.is_empty() {
            return Err(WriteError::Missing(format!(
                "an `{INCOMPLETE_TAG}` fact must name what it was missing: `missing` is the third sign's whole content, and the companions are the fact's reason to exist"
            )));
        }
        Ok(Remember {
            name,
            description,
            fact_type,
            shape,
            phrases,
            missing,
            basis,
            tier,
            valid_until,
            body,
        })
    }
}

/// One in-place edit of a fact, handed from a surface.
///
/// Every field rides [`rewrite_field`], the module's one in-place
/// rewriter: the named frontmatter line is re-rendered, every other byte
/// — the body above all — survives, and the write lands through a temp
/// file and a rename. Nothing here writes a fact file whole, because a
/// second write path would drift from this one's bound and its
/// atomicity, and a surface that wrote around the rewriter would leave
/// the corpus with two truths about how a fact changed.
///
/// The name is not editable: it is the fact's identity and its file's
/// own stem, so renaming one is a supersession, which records why.
/// Neither are the tier and the basis, which are the lifecycle's inputs
/// rather than a surface's.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FactEdit {
    /// The fact being edited.
    pub name: String,
    pub description: Option<String>,
    pub fact_type: Option<String>,
    pub shape: Option<Vec<String>>,
    pub phrases: Option<Vec<String>>,
    pub missing: Option<Vec<String>>,
    pub valid_until: Option<String>,
    /// The replacement body: everything after the frontmatter fence.
    pub body: Option<String>,
}

impl FactEdit {
    /// Parse and validate the arguments object — the same key vocabulary
    /// [`Remember::parse`] reads, and the same bounds, because an edit
    /// that landed a field the writer would refuse would be a fact the
    /// corpus's own rules do not accept.
    pub fn parse(arguments: &serde_json::Value) -> Result<FactEdit, WriteError> {
        let name = string_argument(arguments, "name")?
            .map(|name| name.to_ascii_lowercase())
            .unwrap_or_default();
        if !valid_name(&name) {
            return Err(WriteError::Name(format!(
                "`{name}` is not a fact name: lowercase letters, digits, and hyphens, up to {MAX_NAME_CHARS} characters, no leading or trailing hyphen"
            )));
        }
        let description = string_argument(arguments, "description")?
            .map(|text| sanitize_line(&text, MAX_DESCRIPTION_CHARS));
        if description.as_deref() == Some("") {
            return Err(WriteError::Description(
                "the `description` argument is empty; the fact as one sentence is required"
                    .to_string(),
            ));
        }
        let fact_type =
            string_argument(arguments, "type")?.map(|text| sanitize_line(&text, MAX_TYPE_CHARS));
        if fact_type.as_deref() == Some("") {
            return Err(WriteError::Type(
                "the `type` argument is empty; omit it to leave the type alone".to_string(),
            ));
        }
        let valid_until = string_argument(arguments, "valid-until")?;
        if let Some(day) = &valid_until {
            if !day.is_empty() && !crate::cli::valid_calendar_day(day) {
                return Err(WriteError::ValidUntil(format!(
                    "`{day}` is not a day; use valid-until YYYY-MM-DD"
                )));
            }
        }
        let body = string_argument(arguments, "body")?;
        if let Some(body) = &body {
            if body.len() > MAX_BODY_BYTES {
                return Err(WriteError::Body(format!(
                    "the `body` argument is {} bytes, past the {MAX_BODY_BYTES}-byte bound",
                    body.len()
                )));
            }
        }
        Ok(FactEdit {
            name,
            description,
            fact_type,
            // The shape rides the same scalar the writer reads, parsed
            // here so an absent argument and an empty list stay two
            // different statements: leave the tags alone, or clear them.
            shape: match string_argument(arguments, "shape")? {
                Some(text) => Some(parse_shape(&Some(text))?),
                None => None,
            },
            phrases: match string_array_argument(arguments, "phrases")? {
                Some(list) => Some(parse_phrases(&Some(list))?),
                None => None,
            },
            missing: match string_array_argument(arguments, "missing")? {
                Some(list) => Some(parse_missing(&Some(list))?),
                None => None,
            },
            valid_until,
            body,
        })
    }
}

/// Apply one in-place edit through the module's own rewriter, and hand
/// the fact back as it now stands on disk.
///
/// An unknown name is a typed refusal, and so is a fact a supersession
/// already folded away: editing a retired fact would rewrite a record
/// the corpus keeps for its trail. The tier and the basis are left
/// alone by design — they are the lifecycle's inputs.
pub fn edit(root: &Path, request: &FactEdit) -> Result<Fact, WriteError> {
    let Some(fact) = load_facts(root)
        .into_iter()
        .find(|fact| fact.name == request.name)
    else {
        return Err(WriteError::UnknownFact(format!(
            "no fact named `{}` under {}",
            request.name,
            root.display()
        )));
    };
    if !fact.superseded_by.trim().is_empty() {
        return Err(WriteError::Retired(format!(
            "`{}` was folded into `{}`; a retired fact is a record, not a draft",
            fact.name, fact.superseded_by
        )));
    }
    let path = fact.path.clone();
    if let Some(description) = &request.description {
        rewrite_field(&path, "description", &format!("\"{description}\""))?;
    }
    if let Some(fact_type) = &request.fact_type {
        rewrite_field(&path, "type", fact_type)?;
    }
    if let Some(shape) = &request.shape {
        rewrite_field(&path, "shape", &render_list(shape))?;
    }
    if let Some(phrases) = &request.phrases {
        rewrite_field(&path, "phrases", &render_list(phrases))?;
    }
    if let Some(missing) = &request.missing {
        rewrite_field(&path, "missing", &render_list(missing))?;
    }
    if let Some(valid_until) = &request.valid_until {
        rewrite_field(&path, "valid-until", valid_until)?;
    }
    if let Some(body) = &request.body {
        rewrite_body(&path, body)?;
    }
    // Read the fact back as it now stands, so a caller sees what the
    // rewrite landed rather than what it asked for.
    load_facts(root)
        .into_iter()
        .find(|fact| fact.name == request.name)
        .ok_or_else(|| {
            WriteError::UnknownFact(format!(
                "`{}` did not read back after its edit",
                request.name
            ))
        })
}

/// One tag list as the frontmatter renders it: `[]` when empty, the
/// bracket form otherwise.
fn render_list(entries: &[String]) -> String {
    if entries.is_empty() {
        return "[]".to_string();
    }
    format!("[{}]", entries.join(", "))
}

/// Replace everything after the frontmatter fence, preserving the fence
/// and every byte above it. The body's counterpart to
/// [`rewrite_field`], with the same bound and the same atomicity: the
/// write lands through a unique temp file and a rename, so a reader
/// never sees a half-written fact.
fn rewrite_body(path: &Path, body: &str) -> Result<(), WriteError> {
    let Ok(text) = fs::read_to_string(path) else {
        let reason = format!("could not re-read {} to rewrite its body", path.display());
        eprintln!("fleet: memories: {reason}");
        return Err(WriteError::Io(reason));
    };
    let Some(closing) = text.find("\n---\n") else {
        let reason = format!(
            "{} lost its frontmatter fence; its body was not rewritten",
            path.display()
        );
        eprintln!("fleet: memories: {reason}");
        return Err(WriteError::Io(reason));
    };
    let mut out = String::from(&text[..closing + 5]);
    out.push('\n');
    out.push_str(body);
    if !body.is_empty() && !body.ends_with('\n') {
        out.push('\n');
    }
    let serial = REWRITE_SERIAL.fetch_add(1, Ordering::Relaxed);
    let mut temp = path.to_path_buf().into_os_string();
    temp.push(format!(".{}.{}.tmp", std::process::id(), serial));
    let temp = PathBuf::from(temp);
    match fs::write(&temp, &out) {
        Ok(()) => {
            if fs::rename(&temp, path).is_ok() {
                Ok(())
            } else {
                let _ = fs::remove_file(&temp);
                let reason = format!("could not replace {} with its new body", path.display());
                eprintln!("fleet: memories: {reason}");
                Err(WriteError::Io(reason))
            }
        }
        Err(error) => {
            let _ = fs::remove_file(&temp);
            let reason = format!("could not write {}: {error}", temp.display());
            eprintln!("fleet: memories: {reason}");
            Err(WriteError::Io(reason))
        }
    }
}

/// One string argument read off the arguments object: `None` when the
/// key is absent or null — the caller decides whether that is required
/// — and a typed error when the key is present but is not a string, so
/// a number where a sentence belongs is never a silent default.
fn string_argument(arguments: &serde_json::Value, key: &str) -> Result<Option<String>, WriteError> {
    match arguments.get(key) {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(serde_json::Value::String(text)) => Ok(Some(text.clone())),
        Some(_) => Err(WriteError::Name(format!(
            "the `{key}` argument must be a string"
        ))),
    }
}

/// One array-of-strings argument read off the arguments object: `None`
/// when the key is absent or null, and a typed error when the key is
/// present but is not an array of strings. The phrases ride an array
/// rather than a comma-separated scalar because a phrase legitimately
/// contains commas ("read CLAUDE.local.md directly, never grep").
fn string_array_argument(
    arguments: &serde_json::Value,
    key: &str,
) -> Result<Option<Vec<String>>, WriteError> {
    match arguments.get(key) {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(serde_json::Value::Array(items)) => items
            .iter()
            .map(|item| match item {
                serde_json::Value::String(text) => Ok(text.clone()),
                _ => Err(WriteError::Name(format!(
                    "every `{key}` entry must be a string"
                ))),
            })
            .collect::<Result<Vec<String>, WriteError>>()
            .map(Some),
        Some(_) => Err(WriteError::Name(format!(
            "the `{key}` argument must be an array of strings"
        ))),
    }
}

/// Parse the comma-separated shape list: each tag lowercased and
/// checked against the three axes' vocabulary — the fork's structure
/// shapes, Fleet's failure shapes, and the outcome tags — duplicates
/// dropped, the count capped across all three. A tag outside the
/// taxonomy is refused with the taxonomy named — a typo'd tag is dead
/// weight in the partition, a typo'd failure tag would silently make
/// an anti-vector read as a resolution, and a typo'd outcome tag would
/// silently make a stalled trajectory read as finished.
fn parse_shape(value: &Option<String>) -> Result<Vec<String>, WriteError> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    let mut tags: Vec<String> = Vec::new();
    for tag in value.split(',') {
        let tag = unquote(tag.trim()).trim().to_ascii_lowercase();
        if tag.is_empty() {
            continue;
        }
        if !SHAPES.contains(&tag.as_str()) && !is_failure_shape(&tag) && !is_outcome_shape(&tag) {
            return Err(WriteError::Shape(format!(
                "`{tag}` is not a shape; the structure shapes are {}, the failure shapes are {}, and the outcome tags are {}",
                SHAPES.join(", "),
                FAILURE_SHAPES.join(", "),
                OUTCOME_SHAPES.join(", ")
            )));
        }
        if !tags.iter().any(|seen| seen == &tag) {
            tags.push(tag);
        }
    }
    if tags.len() > MAX_SHAPE_TAGS {
        return Err(WriteError::Shape(format!(
            "{} shape tags is past the {MAX_SHAPE_TAGS}-tag bound",
            tags.len()
        )));
    }
    Ok(tags)
}

/// Parse the missing companions: each sanitized and bounded, empties
/// dropped, duplicates dropped, the count capped — the phrases' own
/// rule, because a companion is a name as it was written and a stall is
/// a handful of names, not an inventory of everything absent.
fn parse_missing(value: &Option<Vec<String>>) -> Result<Vec<String>, WriteError> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    let mut missing: Vec<String> = Vec::new();
    for entry in value {
        let entry = sanitize_line(entry, MAX_MISSING_CHARS);
        if entry.is_empty() || missing.iter().any(|seen| seen == &entry) {
            continue;
        }
        missing.push(entry);
    }
    if missing.len() > MAX_MISSING {
        return Err(WriteError::Missing(format!(
            "{} missing companions is past the {MAX_MISSING}-entry bound",
            missing.len()
        )));
    }
    Ok(missing)
}

/// Parse the resolution's phrases: each sanitized and bounded, empties
/// dropped, duplicates dropped, the count capped. A phrase is the
/// vocabulary that drove, so it is kept as written — control characters
/// stripped and the length bounded the way the description is, and
/// nothing else.
fn parse_phrases(value: &Option<Vec<String>>) -> Result<Vec<String>, WriteError> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    let mut phrases: Vec<String> = Vec::new();
    for phrase in value {
        let phrase = sanitize_line(phrase, MAX_PHRASE_CHARS);
        if phrase.is_empty() || phrases.iter().any(|seen| seen == &phrase) {
            continue;
        }
        phrases.push(phrase);
    }
    if phrases.len() > MAX_PHRASES {
        return Err(WriteError::Phrases(format!(
            "{} phrases is past the {MAX_PHRASES}-phrase bound",
            phrases.len()
        )));
    }
    Ok(phrases)
}

/// Render one fact in the fork's field order: `name`, `description`,
/// `type`, `date`, `basis`, `tier`, `shape`, `phrases`, `missing`,
/// `recall-count`, `superseded-by`, `valid-until` — the order part 1's
/// reader reads back, with `tier` beside `basis` and the slice-19
/// `phrases` and `missing` fields beside their sibling `shape`. An unset
/// optional field renders bare, the fork writer's own "unset" rendering,
/// which the parser accepts.
fn render_document(request: &Remember, today: &str) -> String {
    let mut out = String::from("---\n");
    out.push_str(&format!("name: {}\n", request.name));
    out.push_str(&format!("description: \"{}\"\n", request.description));
    out.push_str(&format!("type: {}\n", request.fact_type));
    out.push_str(&format!("date: {today}\n"));
    out.push_str(&format!("basis: {}\n", request.basis.label()));
    out.push_str(&format!("tier: {}\n", request.tier.label()));
    if request.shape.is_empty() {
        out.push_str("shape: []\n");
    } else {
        out.push_str(&format!("shape: [{}]\n", request.shape.join(", ")));
    }
    if request.phrases.is_empty() {
        out.push_str("phrases: []\n");
    } else {
        out.push_str(&format!("phrases: [{}]\n", request.phrases.join(", ")));
    }
    if request.missing.is_empty() {
        out.push_str("missing: []\n");
    } else {
        out.push_str(&format!("missing: [{}]\n", request.missing.join(", ")));
    }
    out.push_str("recall-count: 0\n");
    out.push_str("superseded-by:\n");
    if request.valid_until.is_empty() {
        out.push_str("valid-until:\n");
    } else {
        out.push_str(&format!("valid-until: {}\n", request.valid_until));
    }
    out.push_str("---\n\n");
    out.push_str(&request.body);
    if !request.body.is_empty() && !request.body.ends_with('\n') {
        out.push('\n');
    }
    out
}

/// Write one fact document under `root` through a unique temp file and
/// a rename, so a reader never sees a half-written fact. With
/// `refuse_existing`, a name that is already taken is a typed error —
/// the modify path is `supersede`, never a silent overwrite.
fn write_document(
    root: &Path,
    request: &Remember,
    today: &str,
    refuse_existing: bool,
) -> Result<PathBuf, WriteError> {
    fs::create_dir_all(root)
        .map_err(|error| WriteError::Io(format!("could not create {}: {error}", root.display())))?;
    let path = root.join(format!("{}.md", request.name));
    if refuse_existing && path.exists() {
        return Err(WriteError::Exists(format!(
            "`{}` already exists; the modify path is `supersede`, which records why",
            request.name
        )));
    }
    let text = render_document(request, today);
    let serial = DOCUMENT_WRITE_SERIAL.fetch_add(1, Ordering::Relaxed);
    let mut temp = path.clone().into_os_string();
    temp.push(format!(".{}.{}.tmp", std::process::id(), serial));
    let temp = PathBuf::from(temp);
    match fs::write(&temp, &text) {
        Ok(()) => {
            if fs::rename(&temp, &path).is_ok() {
                Ok(path)
            } else {
                let _ = fs::remove_file(&temp);
                Err(WriteError::Io(format!(
                    "could not replace {}",
                    path.display()
                )))
            }
        }
        Err(error) => {
            let _ = fs::remove_file(&temp);
            Err(WriteError::Io(format!(
                "could not write {}: {error}",
                temp.display()
            )))
        }
    }
}

/// A per-write serial for the document temp name, so two writes in one
/// process never share a temp file.
static DOCUMENT_WRITE_SERIAL: AtomicUsize = AtomicUsize::new(0);

impl Memory {
    /// Write one fact into the corpus and return it. The write is the
    /// whole record: one file per fact, git-tracked, in the frontmatter
    /// shape part 1's parser reads back — which is what makes a bad
    /// fact a revert. A name that already exists is refused rather
    /// than overwritten: the modify path is [`Memory::supersede`], and
    /// a silent overwrite would erase a confirmed-by fact's provenance.
    pub fn remember(&self, request: &Remember) -> Result<Fact, WriteError> {
        let today = calib::today();
        let path = write_document(&self.root, request, &today, true)?;
        let stamp = file_stamp(&path);
        Ok(Fact {
            name: request.name.clone(),
            description: request.description.clone(),
            fact_type: request.fact_type.clone(),
            date: today,
            basis: request.basis,
            tier: request.tier,
            shape: request.shape.clone(),
            phrases: request.phrases.clone(),
            missing: request.missing.clone(),
            recall_count: 0,
            superseded_by: String::new(),
            valid_until: request.valid_until.clone(),
            body: request.body.clone(),
            path,
            stamp,
        })
    }
}

/// One structured supersession: the modify/forget path. The
/// justification is required either way — it is the proposal's whole
/// provenance, and it is recorded in the calibration row and in the
/// replacement's body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Supersede {
    /// The fact being retired.
    pub name: String,
    pub justification: String,
    /// The replacement fact's name, for the modify path. Omit it to
    /// forget: the fact is expired, and the cycle archives it.
    pub replacement: Option<String>,
    /// The replacement's one-sentence fact; required with a
    /// replacement.
    pub description: String,
    /// The replacement's full content; defaults to the supersession
    /// note.
    pub body: String,
    /// The replacement's type and shape; both default to the
    /// superseded fact's own.
    pub fact_type: Option<String>,
    pub shape: Option<String>,
    /// The replacement's missing companions — the third vector's absent
    /// pieces. **Inherit-or-clear**, the phrases' precedent with one
    /// addition: omitted, the replacement inherits the superseded
    /// fact's own companions, because a modify that changes the fact's
    /// sentence does not change what the trajectory was missing; an
    /// explicit empty list CLEARS them, because the companions arrived.
    /// There is no way to say "unknown" — an absent argument is an
    /// inheritance, and a list is a statement.
    pub missing: Option<Vec<String>>,
    /// The replacement's `valid-until`; defaults to the superseded
    /// fact's own, and is required when that is a working fact with
    /// none.
    pub valid_until: Option<String>,
}

impl Supersede {
    /// Parse and validate the arguments object, the same key vocabulary
    /// [`Remember::parse`] reads.
    pub fn parse(arguments: &serde_json::Value) -> Result<Supersede, WriteError> {
        let name = string_argument(arguments, "name")?
            .map(|name| name.to_ascii_lowercase())
            .unwrap_or_default();
        if !valid_name(&name) {
            return Err(WriteError::Name(format!(
                "`{name}` is not a fact name: lowercase letters, digits, and hyphens, up to {MAX_NAME_CHARS} characters, no leading or trailing hyphen"
            )));
        }
        let justification = sanitize_line(
            &string_argument(arguments, "justification")?.unwrap_or_default(),
            MAX_JUSTIFICATION_CHARS,
        );
        if justification.is_empty() {
            return Err(WriteError::Justification(
                "the `justification` argument is required: why this fact is being retired, recorded in the calibration row and the replacement's body".to_string(),
            ));
        }
        let replacement = match string_argument(arguments, "replacement")? {
            Some(text) => {
                let text = text.to_ascii_lowercase();
                if !valid_name(&text) {
                    return Err(WriteError::Name(format!(
                        "`{text}` is not a replacement fact name: lowercase letters, digits, and hyphens, up to {MAX_NAME_CHARS} characters"
                    )));
                }
                Some(text)
            }
            None => None,
        };
        let description = sanitize_line(
            &string_argument(arguments, "description")?.unwrap_or_default(),
            MAX_DESCRIPTION_CHARS,
        );
        if replacement.is_some() && description.is_empty() {
            return Err(WriteError::Description(
                "the `description` argument is required with a replacement: the new fact as one sentence".to_string(),
            ));
        }
        let fact_type =
            string_argument(arguments, "type")?.map(|text| sanitize_line(&text, MAX_TYPE_CHARS));
        let shape = string_argument(arguments, "shape")?;
        // Absent is not the same as empty: an absent `missing`
        // argument inherits the superseded fact's companions, and an
        // explicit empty list clears them. The distinction has to
        // survive the parse, so the argument's presence rides the
        // `Option`.
        let missing = string_array_argument(arguments, "missing")?
            .map(|list| parse_missing(&Some(list)))
            .transpose()?;
        let valid_until = match string_argument(arguments, "valid-until")? {
            Some(text) => {
                if !crate::cli::valid_calendar_day(&text) {
                    return Err(WriteError::ValidUntil(format!(
                        "`{text}` is not a day; use valid-until YYYY-MM-DD"
                    )));
                }
                Some(text)
            }
            None => None,
        };
        let body = string_argument(arguments, "body")?.unwrap_or_default();
        if body.len() > MAX_BODY_BYTES {
            return Err(WriteError::Body(format!(
                "the `body` argument is {} bytes, past the {MAX_BODY_BYTES}-byte bound",
                body.len()
            )));
        }
        Ok(Supersede {
            name,
            justification,
            replacement,
            description,
            body,
            fact_type,
            shape,
            missing,
            valid_until,
        })
    }
}

/// What one supersession did, so the caller can say so and the
/// calibration row can name it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Superseded {
    /// The fact was folded into a replacement; the file stays, retired
    /// from recall.
    By { from: String, into: String },
    /// The fact was forgotten: expired, with the retirement noted in
    /// its body, and archived by the cycle's next pass.
    Forgotten { fact: String },
}

impl Memory {
    /// The modify/forget path, and the permission model part 2 lands
    /// for slice 12: a `confirmed-by` fact is refused without
    /// approval, an `inferred` one is supersedable on the recorded
    /// justification. The rule lives here, at the module seam, so a
    /// programmatic caller gets the same refusal an interactive one
    /// does — provenance is the guardrail's input, not a prompt string.
    ///
    /// The file is never deleted: a supersession stamps `superseded-by`
    /// and a forget stamps `valid-until`, both reversible, both
    /// git-tracked.
    ///
    /// A stamp that fails to land fails the whole supersession with
    /// [`WriteError::Stamp`] and records no calibration row: the audit
    /// trail cannot claim an action the files did not take. The modify
    /// path writes the replacement first and stamps after, so its
    /// error names the replacement that landed beside the stamp that
    /// did not; the forget path lands both its writes before the row.
    pub fn supersede(
        &self,
        request: &Supersede,
        approved: bool,
        stream: Option<&calib::Stream>,
    ) -> Result<Superseded, WriteError> {
        let today = calib::today();
        let facts = load_facts(&self.root);
        let Some(fact) = facts.iter().find(|fact| fact.name == request.name).cloned() else {
            return Err(WriteError::UnknownFact(format!(
                "no fact named `{}` in {}",
                request.name,
                self.root.display()
            )));
        };
        if !fact.superseded_by.trim().is_empty() {
            return Err(WriteError::Retired(format!(
                "`{}` is already superseded by `{}`",
                fact.name, fact.superseded_by
            )));
        }
        if fact.forgotten(&today) {
            return Err(WriteError::Retired(format!(
                "`{}` is already expired (valid-until {})",
                fact.name, fact.valid_until
            )));
        }
        // The permission model: provenance decides what a write may do
        // without an approval in hand.
        if fact.basis == Basis::ConfirmedBy && !approved {
            return Err(WriteError::NeedsApproval(format!(
                "`{}` is confirmed-by \u{2014} a load-bearing fact \u{2014} so it cannot be modified or forgotten without approval",
                fact.name
            )));
        }
        match &request.replacement {
            Some(replacement) => {
                let valid_until = match &request.valid_until {
                    Some(day) => day.clone(),
                    None => fact.valid_until.clone(),
                };
                if fact.tier == Tier::Working && valid_until.is_empty() {
                    return Err(WriteError::ValidUntil(
                        "the superseded fact is working-tier, so the replacement needs a `valid-until`".to_string(),
                    ));
                }
                let body = if request.body.is_empty() {
                    format!(
                        "Supersedes `{}` \u{2014} {}\n",
                        fact.name, request.justification
                    )
                } else {
                    request.body.clone()
                };
                let replacement_request = Remember {
                    name: replacement.clone(),
                    description: request.description.clone(),
                    fact_type: request
                        .fact_type
                        .clone()
                        .unwrap_or_else(|| fact.fact_type.clone()),
                    shape: match &request.shape {
                        Some(text) => parse_shape(&Some(text.clone()))?,
                        None => fact.shape.clone(),
                    },
                    // A modify keeps the direction the superseded fact
                    // recorded: the vocabulary that worked does not
                    // change because the fact's sentence did. A
                    // supersession that needs a new direction writes a
                    // new fact rather than editing this one's.
                    phrases: fact.phrases.clone(),
                    // The companions follow the same inherit-or-clear
                    // rule with one addition: an omitted argument
                    // inherits them (a reworded fact does not unmiss
                    // what was missing), and an explicit empty list
                    // clears them, because the companions arrived.
                    missing: match &request.missing {
                        Some(list) => list.clone(),
                        None => fact.missing.clone(),
                    },
                    // A supersession is a proposal: the replacement is
                    // inferred until someone confirms it.
                    basis: Basis::Inferred,
                    tier: fact.tier,
                    valid_until,
                    body,
                };
                write_document(&self.root, &replacement_request, &today, true)?;
                // The replacement is on disk; the old fact must now be
                // stamped retired or both stay live — and a row here
                // would claim a retirement the file never took. The
                // error names what landed and what did not.
                if let Err(error) = rewrite_field(&fact.path, "superseded-by", replacement) {
                    return Err(WriteError::Stamp(format!(
                        "the replacement `{replacement}` was written, but stamping `{}` superseded-by it did not land ({error}), so both facts are still live",
                        fact.name
                    )));
                }
                if let Some(stream) = stream {
                    stream.record(calib::Row::Consolidate(calib::ConsolidateRow {
                        ts: calib::stamp(),
                        action: calib::ACTION_SUPERSEDE.to_string(),
                        fact: fact.name.clone(),
                        reason: request.justification.clone(),
                    }));
                }
                Ok(Superseded::By {
                    from: fact.name,
                    into: replacement.clone(),
                })
            }
            None => {
                // Forget: the fact's validity ends the day before the
                // retirement, so the loader retires it now and the
                // cycle's next pass archives the file. The note in the
                // body is the human record of why. Both writes land
                // before the row is recorded — a row for a retirement
                // that did not happen is the bug this ordering
                // prevents.
                let last_valid_day = shift_day(&today, -1).unwrap_or_else(|| today.clone());
                rewrite_field(&fact.path, "valid-until", &last_valid_day).map_err(|error| {
                    WriteError::Stamp(format!(
                        "`{}` was not expired, so nothing was recorded: {error}",
                        fact.name
                    ))
                })?;
                append_note(
                    &fact.path,
                    &format!("Retired {today}: {}\n", request.justification),
                )
                .map_err(|error| {
                    WriteError::Stamp(format!(
                        "`{}` was expired (valid-until {last_valid_day}) but its retirement note did not land, so the retirement was not recorded: {error}",
                        fact.name
                    ))
                })?;
                if let Some(stream) = stream {
                    stream.record(calib::Row::Consolidate(calib::ConsolidateRow {
                        ts: calib::stamp(),
                        action: calib::ACTION_FORGET.to_string(),
                        fact: fact.name.clone(),
                        reason: request.justification.clone(),
                    }));
                }
                Ok(Superseded::Forgotten { fact: fact.name })
            }
        }
    }
}

/// Shift a `YYYY-MM-DD` day by `delta` days, or `None` when the day is
/// unreadable. The calendar arithmetic is the session log's own — one
/// implementation in the crate, not two.
fn shift_day(day: &str, delta: i64) -> Option<String> {
    let days = parse_day(day)? + delta;
    let (year, month, day_of_month) = crate::session::civil_from_days(days);
    Some(format!("{year:04}-{month:02}-{day_of_month:02}"))
}

/// One line naming the fact a `remember` call would write, for the
/// approval prompt and the progress line: the fact and its basis, the
/// two facts a user needs to answer "may this run?".
pub fn remember_summary(arguments: &str) -> String {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(arguments) else {
        return "remember".to_string();
    };
    let name = value
        .get("name")
        .and_then(|value| value.as_str())
        .unwrap_or_default();
    if name.is_empty() {
        return "remember".to_string();
    }
    let basis = value
        .get("basis")
        .and_then(|value| value.as_str())
        .unwrap_or("inferred");
    let tier = value
        .get("tier")
        .and_then(|value| value.as_str())
        .unwrap_or("durable");
    format!("remember {name} ({basis} {tier})")
}

/// One line naming the fact a `supersede` call would retire, its basis
/// when the corpus carries one, and the justification: the facts a user
/// needs to answer "may this run?" — and, for a confirmed-by target,
/// the prompt the provenance guard asks again with.
pub fn supersede_summary(arguments: &str, basis: Option<Basis>) -> String {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(arguments) else {
        return "supersede".to_string();
    };
    let name = value
        .get("name")
        .and_then(|value| value.as_str())
        .unwrap_or_default();
    if name.is_empty() {
        return "supersede".to_string();
    }
    let justification = sanitize_line(
        value
            .get("justification")
            .and_then(|value| value.as_str())
            .unwrap_or_default(),
        MAX_JUSTIFICATION_CHARS,
    );
    // KI-067: what the fact currently says, when the caller carried it
    // — the confirmed-by gate's synthetic call does, so the approver
    // decides with the target's own description in front of them.
    let target = sanitize_line(
        value
            .get("target_description")
            .and_then(|value| value.as_str())
            .unwrap_or_default(),
        MAX_JUSTIFICATION_CHARS,
    );
    let basis = basis.map(Basis::label);
    let mut summary = match basis {
        Some(basis) => format!("supersede {name} ({basis})"),
        None => format!("supersede {name}"),
    };
    if !target.is_empty() {
        summary.push_str(&format!(" \u{2014} replaces: {target}"));
    }
    if !justification.is_empty() {
        summary.push_str(&format!(" \u{2014} {justification}"));
    }
    summary
}

// --- the consolidation cycle ------------------------------------------------

/// How often the scheduled cycle runs. Hourly: the live system's
/// episodic store went dormant because its consolidation was manual,
/// and an hourly pass is often enough to keep working memory from
/// outliving its session without being so frequent that it churns the
/// corpus.
pub const CONSOLIDATE_INTERVAL: Duration = Duration::from_secs(3_600);

/// Working memory older than this is compressed even when its
/// `valid-until` has not passed: a session-scoped fact nobody renewed
/// is a fact the session is done with.
pub const WORKING_MAX_AGE_DAYS: i64 = 7;

/// A durable inferred fact that recall has never surfaced and that is
/// older than this is stale, and the cycle supersedes it. Confirmed-by
/// facts are never stale in this sense: provenance decides, not age.
pub const STALE_AFTER_DAYS: i64 = 180;

/// Where the cycle's forget action moves an expired fact: a directory
/// beside the corpus, not a delete. The file stays — reversible by a
/// move back, and diffable in git — it simply leaves the loader's
/// read, which takes `memories/*.md` and never a subdirectory.
pub const ARCHIVE_DIR: &str = ".archive";

/// One thing the cycle did to one fact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CycleAction {
    /// `compress`, `supersede`, or `forget` — [`calib::ACTIONS`].
    pub action: &'static str,
    pub fact: String,
    /// Why: the named bound the cycle acted on, or the proposal's
    /// justification.
    pub reason: String,
}

/// What one consolidation pass did — the shape the surface renders and
/// the test asserts.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Consolidation {
    /// Every action, in the order the pass took them.
    pub actions: Vec<CycleAction>,
    /// Confirmed-by facts the pass left alone: the provenance rule,
    /// named rather than silently skipped.
    pub untouched: Vec<String>,
    /// Incomplete facts the pass left alone: the third-vector rule,
    /// named rather than silently skipped. An incomplete durable
    /// inferred fact in the stale shape is NOT folded — the summary's
    /// bullets carry a fact's name, date, and description, never its
    /// missing companions, and the companions are the fact's whole
    /// point. Folding one would keep the sentence and lose the absent
    /// pieces a later session needs.
    pub kept: Vec<String>,
    /// The episodic summary this pass wrote, when it wrote one.
    pub summary: Option<String>,
}

impl Consolidation {
    /// Whether the pass changed nothing.
    pub fn is_empty(&self) -> bool {
        self.actions.is_empty()
    }
}

/// Whether `fact`'s date is more than `days` before `today_days`. An
/// unreadable date is not aged out: the cycle cannot prove a fact is
/// old, so it leaves it alone.
fn aged_out(fact: &Fact, today_days: Option<i64>, days: i64) -> bool {
    matches!(day_age(&fact.date, today_days), Some(age) if age > days)
}

// --- the cycle's four sets, shared by the pass and the preview --------------

/// The pass's compress set: working facts past their `valid-until` or
/// older than [`WORKING_MAX_AGE_DAYS`], not already folded. Shared by
/// [`consolidate`] and [`preview`], so the surface's queue and the
/// cycle's actions can never disagree about who is in it.
fn compress_candidates(facts: &[Fact], today: &str, today_days: Option<i64>) -> Vec<Fact> {
    facts
        .iter()
        .filter(|fact| {
            fact.tier == Tier::Working
                && fact.superseded_by.trim().is_empty()
                && (fact.forgotten(today) || aged_out(fact, today_days, WORKING_MAX_AGE_DAYS))
        })
        .cloned()
        .collect()
}

/// The pass's supersede set: durable, inferred, never recalled, past
/// [`STALE_AFTER_DAYS`], not already retired, and not incomplete — the
/// third-vector rule: a summary bullet cannot carry the missing
/// companions, and they are the fact's whole point.
fn supersede_candidates(facts: &[Fact], today: &str, today_days: Option<i64>) -> Vec<Fact> {
    facts
        .iter()
        .filter(|fact| {
            fact.tier == Tier::Durable
                && fact.basis == Basis::Inferred
                && fact.superseded_by.trim().is_empty()
                && !fact.forgotten(today)
                && fact.recall_count == 0
                && fact.sign() != Sign::Incomplete
                && aged_out(fact, today_days, STALE_AFTER_DAYS)
        })
        .cloned()
        .collect()
}

/// The provenance rule, from the cycle's side: confirmed-by facts in the
/// supersede set's shape, named and left exactly as they are.
fn untouched_candidates(facts: &[Fact], today: &str, today_days: Option<i64>) -> Vec<Fact> {
    facts
        .iter()
        .filter(|fact| {
            fact.tier == Tier::Durable
                && fact.basis == Basis::ConfirmedBy
                && fact.superseded_by.trim().is_empty()
                && !fact.forgotten(today)
                && fact.recall_count == 0
                && aged_out(fact, today_days, STALE_AFTER_DAYS)
        })
        .cloned()
        .collect()
}

/// The third-vector rule, from the cycle's side: incomplete facts in the
/// supersede set's shape, named and left exactly as they are.
fn kept_candidates(facts: &[Fact], today: &str, today_days: Option<i64>) -> Vec<Fact> {
    facts
        .iter()
        .filter(|fact| {
            fact.tier == Tier::Durable
                && fact.basis == Basis::Inferred
                && fact.sign() == Sign::Incomplete
                && fact.superseded_by.trim().is_empty()
                && !fact.forgotten(today)
                && fact.recall_count == 0
                && aged_out(fact, today_days, STALE_AFTER_DAYS)
        })
        .cloned()
        .collect()
}

fn names_of(facts: &[Fact]) -> Vec<String> {
    facts.iter().map(|fact| fact.name.clone()).collect()
}

/// What the cycle WOULD do, computed and written nowhere — the face's
/// needs-you section. The same four filters [`consolidate`] acts on,
/// through the same helpers, so the queue and the pass cannot drift.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CyclePreview {
    /// Working facts the pass would compress.
    pub compress: Vec<String>,
    /// Stale never-recalled inferred facts the pass would fold.
    pub supersede: Vec<String>,
    /// The confirmed-by facts in the same shape the pass would leave —
    /// named, because they are the owner's to judge.
    pub untouched: Vec<String>,
    /// The incomplete facts the pass would keep — the missing companions
    /// cannot ride a summary bullet.
    pub kept: Vec<String>,
}

impl CyclePreview {
    /// Whether the cycle has nothing queued.
    pub fn is_empty(&self) -> bool {
        self.compress.is_empty()
            && self.supersede.is_empty()
            && self.untouched.is_empty()
            && self.kept.is_empty()
    }

    /// How many facts the queue names in all.
    pub fn total(&self) -> usize {
        self.compress.len() + self.supersede.len() + self.untouched.len() + self.kept.len()
    }
}

/// The cycle's queue, computed and written nowhere: `fleet memories
/// --consolidate` acts on it, the face's needs-you section shows it.
pub fn preview(root: &Path, today: &str) -> CyclePreview {
    let facts = load_facts(root);
    let today_days = parse_day(today);
    CyclePreview {
        compress: names_of(&compress_candidates(&facts, today, today_days)),
        supersede: names_of(&supersede_candidates(&facts, today, today_days)),
        untouched: names_of(&untouched_candidates(&facts, today, today_days)),
        kept: names_of(&kept_candidates(&facts, today, today_days)),
    }
}

/// The section the fact reads under on the face's rail: the type word
/// with the documented synonyms mapped onto the protected types (KI-059's
/// mapping — a `preference` sits with feedback, where the protection
/// already put it), the cycle's episodic word beside the four canonical
/// kinds, and `everything-else` for the free-form types the substrate
/// deliberately permits. The declared words are the categories; nothing
/// is inferred.
pub fn section_of(fact: &Fact) -> &'static str {
    let kind = fact.fact_type.trim().to_ascii_lowercase();
    let canonical = PROTECTED_SYNONYMS
        .iter()
        .find(|(synonym, _)| *synonym == kind)
        .map(|(_, protected)| *protected)
        .unwrap_or(kind.as_str());
    match canonical {
        "user" => "user",
        "feedback" => "feedback",
        "project" => "project",
        "reference" => "reference",
        "episodic" => "episodic",
        _ => "everything-else",
    }
}

/// The archive's facts: what the cycle forgot, read with the
/// stem-equals-name rule relaxed (a collision copy carries a serial on
/// disk while its frontmatter still names the fact).
pub fn load_archive(root: &Path) -> Vec<Fact> {
    load_facts_archived(&root.join(ARCHIVE_DIR))
}

/// One consolidation pass over the corpus under `root`, against the
/// day `today` names — the seam a test injects a fixed day through, so
/// the cycle is deterministic rather than sleep-dependent.
///
/// Three actions. A fact that is past its `valid-until` gets two of
/// them, in order: **compress** folds its content into the pass's
/// summary, then **forget** archives the expired original — the
/// content lives on in the recallable summary, and the file leaves the
/// working tree. A fact that is old but not expired is compressed
/// only, and its file stays as the record of what was folded.
///
/// - **compress** — a working fact that is past its `valid-until` or
///   older than [`WORKING_MAX_AGE_DAYS`] is folded into the pass's
///   episodic summary and stamped `superseded-by` it. The file stays;
///   the summary carries its content.
/// - **supersede** — a durable `inferred` fact recall has never
///   surfaced, older than [`STALE_AFTER_DAYS`], is folded into the
///   same summary and stamped. A `confirmed-by` fact in that shape is
///   left alone and named in [`Consolidation::untouched`] — the
///   permission model, applied to the cycle itself — and an
///   **incomplete** one is left alone and named in
///   [`Consolidation::kept`], because a summary bullet cannot carry
///   the missing companions and the companions are the fact's whole
///   point.
/// - **forget** — a fact whose `valid-until` has passed and that no
///   summary carries is moved to [`ARCHIVE_DIR`]. Nothing is deleted.
///
/// Every action appends one `consolidate` calibration row.
pub fn consolidate(root: &Path, today: &str, stream: Option<&calib::Stream>) -> Consolidation {
    let facts = load_facts(root);
    let today_days = parse_day(today);
    let mut out = Consolidation::default();

    let compress = compress_candidates(&facts, today, today_days);
    let supersede = supersede_candidates(&facts, today, today_days);
    // The provenance rule, from the cycle's side: a confirmed-by fact
    // in the stale shape is named and left exactly as it is.
    out.untouched = names_of(&untouched_candidates(&facts, today, today_days));
    // The third-vector rule, from the cycle's side: an incomplete
    // durable inferred fact in the stale shape is named and left exactly
    // as it is, so the surface can say why rather than skip it silently.
    out.kept = names_of(&kept_candidates(&facts, today, today_days));

    let summary = if compress.is_empty() && supersede.is_empty() {
        None
    } else {
        match write_summary(root, today, &compress, &supersede) {
            Ok(name) => Some(name),
            Err(error) => {
                // The fold did not land: nothing is stamped, nothing is
                // recorded, and nothing is archived — every source fact
                // stays live and rankable for the next pass. Named, not
                // silent: a fold the audit trail claims but the disk
                // contradicts is the one lie this cycle cannot afford
                // (KI-212).
                eprintln!(
                    "fleet: the consolidation summary could not be written ({error}); \
                     nothing was folded and every source fact stays live"
                );
                None
            }
        }
    };
    // The forget half only follows a landed fold. `None` here means one
    // of two things: there was nothing to fold (the pass still forgets,
    // exactly as before), or the fold failed (nothing is archived — the
    // expired facts are retried whole next pass).
    let fold_landed = summary.is_some() || (compress.is_empty() && supersede.is_empty());

    for fact in &compress {
        // The action exists only when its fold did: a failed summary
        // leaves the fact unstamped, unrecorded, and unarchived — the
        // pass reports nothing it did not do (KI-212).
        if let Some(summary) = &summary {
            // Best-effort, and logged by the rewriter itself: a stamp
            // the cycle could not land leaves the fact live and names
            // the reason on stderr, and the cycle's pass is advisory
            // rather than a gated action.
            let _ = rewrite_field(&fact.path, "superseded-by", summary);
            let reason = if fact.forgotten(today) {
                format!("working memory past its valid-until {}", fact.valid_until)
            } else {
                format!("working memory older than {WORKING_MAX_AGE_DAYS} days")
            };
            record_cycle(stream, calib::ACTION_COMPRESS, &fact.name, &reason);
            out.actions.push(CycleAction {
                action: calib::ACTION_COMPRESS,
                fact: fact.name.clone(),
                reason,
            });
        }
    }
    for fact in &supersede {
        if let Some(summary) = &summary {
            let _ = rewrite_field(&fact.path, "superseded-by", summary);
            let reason = format!("inferred, never recalled, older than {STALE_AFTER_DAYS} days");
            record_cycle(stream, calib::ACTION_SUPERSEDE, &fact.name, &reason);
            out.actions.push(CycleAction {
                action: calib::ACTION_SUPERSEDE,
                fact: fact.name.clone(),
                reason,
            });
        }
    }

    if fold_landed {
        for fact in &facts {
            let expired = matches!(day_age(&fact.valid_until, today_days), Some(age) if age > 0);
            if !expired || !fact.superseded_by.trim().is_empty() {
                continue;
            }
            if archive_fact(root, fact) {
                let reason = format!("expired {}", fact.valid_until);
                record_cycle(stream, calib::ACTION_FORGET, &fact.name, &reason);
                out.actions.push(CycleAction {
                    action: calib::ACTION_FORGET,
                    fact: fact.name.clone(),
                    reason,
                });
            }
        }
    }

    out.summary = summary;
    out
}

/// Append one `consolidate` row, best-effort like every row.
fn record_cycle(stream: Option<&calib::Stream>, action: &str, fact: &str, reason: &str) {
    if let Some(stream) = stream {
        stream.record(calib::Row::Consolidate(calib::ConsolidateRow {
            ts: calib::stamp(),
            action: action.to_string(),
            fact: fact.to_string(),
            reason: reason.to_string(),
        }));
    }
}

/// Write the pass's episodic summary: one fact naming everything the
/// pass folded, so a compression preserves the content it removes from
/// the working tier. The name is the day, numbered past any earlier
/// pass the same day already wrote — `consolidation-2026-10-13`,
/// then `-2`, so two passes never collide on one file. A write that
/// fails is the caller's to name: the fold it was to carry did not
/// happen (KI-212).
fn write_summary(
    root: &Path,
    today: &str,
    compressed: &[Fact],
    superseded: &[Fact],
) -> Result<String, WriteError> {
    let folded = compressed.len() + superseded.len();
    let mut body = format!("What the consolidation cycle folded on {today}.\n");
    if !compressed.is_empty() {
        body.push_str("\n## Compressed working memory\n\n");
        for fact in compressed {
            body.push_str(&format!(
                "- `{}` ({}): {}\n",
                fact.name, fact.date, fact.description
            ));
        }
    }
    if !superseded.is_empty() {
        body.push_str("\n## Superseded stale facts\n\n");
        for fact in superseded {
            body.push_str(&format!(
                "- `{}` ({}): {}\n",
                fact.name, fact.date, fact.description
            ));
        }
    }
    let base = format!("consolidation-{today}");
    let mut name = base.clone();
    let mut serial = 2;
    while root.join(format!("{name}.md")).exists() {
        name = format!("{base}-{serial}");
        serial += 1;
    }
    let request = Remember {
        name: name.clone(),
        description: sanitize_line(
            &format!(
                "the consolidation cycle's {today} pass \u{2014} {folded} fact{} folded",
                if folded == 1 { "" } else { "s" }
            ),
            MAX_DESCRIPTION_CHARS,
        ),
        fact_type: "episodic".to_string(),
        shape: Vec::new(),
        phrases: Vec::new(),
        missing: Vec::new(),
        basis: Basis::Inferred,
        tier: Tier::Episodic,
        valid_until: String::new(),
        body,
    };
    // The name is free by construction, so the existence check is off.
    write_document(root, &request, today, false)?;
    Ok(name)
}

/// The archive's collision shape: the fact's own name for the first
/// copy, then a serial — `.archive/<name>.md`, `.archive/<name>.2.md`,
/// `.archive/<name>.3.md`. A fact name is re-usable (a fact expires,
/// the name is written again, it expires again), and the second copy
/// must not clobber the first: the archive is the record of what the
/// cycle forgot, and git is a backstop only for what was committed.
/// The serial rather than the day: two copies retired the same day
/// would collide on a day-stamp, and each copy's own `valid-until` and
/// body already date it.
///
/// The name is claimed with `create_new`, so the collision check and
/// the move are one step rather than a check that can drift from the
/// rename that follows it.
fn archive_target(archive: &Path, fact: &Fact) -> Result<PathBuf, std::io::Error> {
    let mut serial = 1;
    loop {
        let candidate = if serial == 1 {
            archive.join(format!("{}.md", fact.name))
        } else {
            archive.join(format!("{}.{serial}.md", fact.name))
        };
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate)
        {
            Ok(_) => return Ok(candidate),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => serial += 1,
            Err(error) => return Err(error),
        }
    }
}

/// Move one expired fact into [`ARCHIVE_DIR`]. A rename, not a delete:
/// the file stays, reversible, and out of the loader's read. A name the
/// archive already holds takes the next serial (see
/// [`archive_target`]), so a re-used fact name never overwrites the
/// copy already there.
fn archive_fact(root: &Path, fact: &Fact) -> bool {
    let archive = root.join(ARCHIVE_DIR);
    if fs::create_dir_all(&archive).is_err() {
        eprintln!(
            "fleet: memories: could not create {}; `{}` was not archived",
            archive.display(),
            fact.name
        );
        return false;
    }
    let target = match archive_target(&archive, fact) {
        Ok(target) => target,
        Err(error) => {
            eprintln!(
                "fleet: memories: could not claim an archive name for `{}`: {error}",
                fact.name
            );
            return false;
        }
    };
    match fs::rename(&fact.path, &target) {
        Ok(()) => true,
        Err(error) => {
            // The claim is dropped rather than left behind as an empty
            // file the archive surface would skip by name.
            let _ = fs::remove_file(&target);
            eprintln!(
                "fleet: memories: could not archive {}: {error}",
                fact.path.display()
            );
            false
        }
    }
}

/// Spawn the scheduled consolidation thread: one pass at once, then one
/// every [`CONSOLIDATE_INTERVAL`], for the life of the process. This is
/// the timer the live system never had — the thread is detached, so it
/// costs a long-lived process nothing to end, and a pass that finds
/// nothing writes nothing.
///
/// The stream is the thread's own: the cycle must run even when
/// routing never produced one, and two writer threads in one process
/// are safe by the stream's own construction (one `O_APPEND` write per
/// row).
pub fn spawn_consolidator(root: PathBuf, stream: calib::Stream) {
    std::thread::spawn(move || loop {
        let today = calib::today();
        let result = consolidate(&root, &today, Some(&stream));
        if !result.is_empty() {
            let tally = calib::ACTIONS
                .iter()
                .filter_map(|action| {
                    let count = result
                        .actions
                        .iter()
                        .filter(|one| one.action == *action)
                        .count();
                    (count > 0).then(|| format!("{count} {action}"))
                })
                .collect::<Vec<_>>()
                .join(", ");
            eprintln!(
                "fleet: memories: consolidated {} fact{} ({tally}) \u{2014} see `fleet calib`",
                result.actions.len(),
                if result.actions.len() == 1 { "" } else { "s" },
            );
        }
        // KI-063: the pass's rows are flushed before the sleep. The
        // stream drains when its last handle drops, and this thread's
        // handle never drops — so a process that exits mid-pass used to
        // lose the rows still queued. A bounded wait per pass: the cycle
        // is hourly, and a wedged disk costs the wait, not the cycle.
        stream.flush();
        std::thread::sleep(CONSOLIDATE_INTERVAL);
    });
}

// --- the surface -----------------------------------------------------------

mod palette {
    pub const ACCENT: &str = "\u{1b}[36m";
    pub const DIM: &str = "\u{1b}[90m";
    pub const TEXT: &str = "\u{1b}[37m";
    pub const BOLD: &str = "\u{1b}[1m";
    /// The divergence's color: an anti-vector is a caution, not a
    /// candidate — the surface renders it in a different color so the
    /// signs read apart at a glance.
    pub const WARN: &str = "\u{1b}[33m";
    pub const RESET: &str = "\u{1b}[0m";
}

use palette::{ACCENT, BOLD, DIM, RESET, TEXT, WARN};

/// The card width, matching `skills`, `calib`, `lanes`, and `providers`.
const CARD_WIDTH: usize = 72;

/// The recall surface's line bound. Its status line and its dividers
/// are concatenations that grow with the corpus — a backend label, a
/// fact count, a route reason, a three-tag standing line — and an
/// unbounded concatenation on a fixed-width surface is the KI-078 class
/// (KI-085, KI-096, KI-115, KI-179 are the same failure on four other
/// surfaces, and this slice's own recall surface was its fifth). 110
/// sits under the 120-column terminal the owner works in and over the
/// 80-column minimum, so neither the header nor a three-tag card line
/// runs past the edge; both wrap through [`wrap_detail`], KI-055's
/// shared word-aware wrapper, at word boundaries rather than mid-token.
const SURFACE_WIDTH: usize = 110;

/// The English plural for a count, the way the card family carries it.
fn plural(count: usize) -> &'static str {
    if count == 1 {
        ""
    } else {
        "s"
    }
}

/// One dim status or divider line, bounded to `width` and wrapped at
/// word boundaries. Each chunk carries its own palette codes, so the
/// wrap is invisible to the reader: the line reads as one sentence that
/// happens to continue, and never as a mid-token cut.
fn dim_line(text: &str, width: usize) -> Vec<String> {
    wrap_detail(text, width)
        .into_iter()
        .map(|chunk| format!("{DIM}{chunk}{RESET}"))
        .collect()
}

/// Render the corpus as cards: one per live fact with its name, type,
/// shape, basis, recall-count, and valid-until, in the house card style.
/// Retired facts are counted in the footer rather than carded — a
/// superseded or expired fact is not part of the working corpus.
pub fn render(memory: &Memory, shape: Option<&str>) -> String {
    let facts = memory.facts();
    let today = calib::today();
    let live: Vec<&Fact> = facts
        .iter()
        .filter(|fact| !fact.forgotten(&today))
        .collect();
    let retired = facts.len() - live.len();
    let protected = live.iter().filter(|fact| fact.protected()).count();
    let shown: Vec<&Fact> = match shape {
        Some(shape) => live
            .iter()
            .copied()
            .filter(|fact| fact.carries_shape(shape))
            .collect(),
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
            for line in dim_line(
                &format!(
                    "no facts carry shape `{shape}` \u{b7} the structure shapes are {}, the failure shapes are {}, and the outcome tags are {}",
                    SHAPES.join(", "),
                    FAILURE_SHAPES.join(", "),
                    OUTCOME_SHAPES.join(", ")
                ),
                SURFACE_WIDTH,
            ) {
                out.push_str(&line);
                out.push('\n');
            }
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
    // The recorded phrases — the fact's direction — ride the card when
    // the fact carries them, so the surface shows what drove, not only
    // what the fact is about.
    if !fact.phrases.is_empty() {
        let phrases = fact.phrases.join(" \u{b7} ");
        for chunk in wrap_detail(&format!("drove: {phrases}"), CARD_WIDTH - 4) {
            lines.push(format!("{DIM}\u{2502}{RESET} {DIM}{chunk}{RESET}"));
        }
    }
    // The missing companions ride the card too, beside the direction:
    // an incomplete vector's whole point is the named absent pieces.
    if !fact.missing.is_empty() {
        let missing = fact.missing.join(" \u{b7} ");
        for chunk in wrap_detail(&format!("missing: {missing}"), CARD_WIDTH - 4) {
            lines.push(format!("{DIM}\u{2502}{RESET} {DIM}{chunk}{RESET}"));
        }
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
    // The footer is a concatenation that grows with the tags, so it is
    // bounded to the card's own width and wrapped rather than run past
    // the border (the KI-078 class, on this surface).
    let footer = format!(
        "shape: {shape} \u{b7} recalls: {} \u{b7} valid-until: {valid_until}{}",
        fact.recall_count,
        tier_suffix(fact),
    );
    for chunk in wrap_detail(&footer, CARD_WIDTH - 4) {
        lines.push(format!("{DIM}\u{2502}{RESET} {DIM}{chunk}{RESET}"));
    }
    lines.push(format!(
        "{DIM}\u{2502}{RESET} {DIM}{}{RESET}",
        fact.path.display()
    ));
    let bottom = format!(
        "{DIM}\u{2514}{}\u{2518}{RESET}",
        "\u{2500}".repeat(CARD_WIDTH - 2)
    );
    let mut card = format!("{header}\n");
    for line in lines {
        card.push_str(&line);
        card.push('\n');
    }
    card.push_str(&bottom);
    card.push('\n');
    card
}

/// Render the ranked assembly for one instruction: the partition — the
/// shape's own pool, then the off-shape spillover marked and named for
/// the route that admitted it — each fact's card naming the evidence
/// that ranked it (the channels, the trajectory, the hit history, the
/// basis, the sign), then the block that would enter a turn's context,
/// then what the budget dropped. A dry run — printing changes no
/// recall-count and records no row; the turn path (part 2) records the
/// hits.
pub fn render_recall(memory: &Memory, instruction: &str) -> String {
    render_recall_at(memory, instruction, SURFACE_WIDTH)
}

/// The recall surface at an explicit line bound — the seam a test
/// narrows the surface through, the way `Memory::assemble_within` is
/// the budget's seam. Production calls [`render_recall`].
fn render_recall_at(memory: &Memory, instruction: &str, width: usize) -> String {
    let assembly = memory.assemble(instruction, &[]);
    let mut out = String::new();
    out.push_str(&format!(
        "{BOLD}fleet memories --recall{RESET} {DIM}\u{2014} \"{instruction}\"{RESET}\n"
    ));
    let shape_line = match assembly.shape {
        Some(shape) => format!("shape: {shape} ({}-shape)", shape_axis(shape)),
        None => "shape: none \u{2014} surface recall (the documented fallback)".to_string(),
    };
    let primary = assembly.ranked.iter().filter(|hit| !hit.off_shape).count();
    let off = assembly.ranked.len() - primary;
    for line in dim_line(
        &format!(
            "{shape_line} \u{b7} backend: {} \u{b7} {} fact{} ranked \u{b7} {primary} in the pool \u{b7} {off} off-shape \u{b7} {} fit the {}-token budget",
            memory.backend_label(),
            assembly.ranked.len(),
            plural(assembly.ranked.len()),
            assembly.recalled.len(),
            assembly.budget,
        ),
        width,
    ) {
        out.push_str(&line);
        out.push('\n');
    }
    out.push('\n');
    let live = memory
        .facts()
        .iter()
        .filter(|fact| !fact.forgotten(&calib::today()))
        .count();
    if live == 0 {
        for line in dim_line(
            &format!(
                "no facts to recall from \u{b7} {}/ holds no live facts",
                MEMORIES_DIR
            ),
            width,
        ) {
            out.push_str(&line);
            out.push('\n');
        }
        return out;
    }
    if assembly.ranked.is_empty() {
        for line in dim_line(
            &format!(
                "no fact cleared the {MIN_RECALL_SCORE} floor for this instruction \u{b7} the turn answers from the surface alone"
            ),
            width,
        ) {
            out.push_str(&line);
            out.push('\n');
        }
        return out;
    }
    // The partition, sectioned: the shape's own pool first — whatever
    // the words say, the shape's answer leads — then the spillover,
    // marked and named for the route that admitted it.
    let pool_name = match assembly.shape {
        Some(shape) => format!("the `{shape}` pool"),
        None => "the word-ranked pool (no shape matched)".to_string(),
    };
    for line in dim_line(
        &format!(
            "\u{b7} {pool_name} \u{b7} {primary} fact{} \u{b7} resolved: drove toward the minima \u{b7} diverged: drove away \u{b7} incomplete: drove partway and stalled",
            plural(primary)
        ),
        width,
    ) {
        out.push_str(&line);
        out.push('\n');
    }
    for hit in assembly.ranked.iter().filter(|hit| !hit.off_shape) {
        out.push_str(&render_recall_card(hit, width));
    }
    if !assembly.spillover.is_empty() {
        let route = assembly
            .spillover_route
            .clone()
            .unwrap_or_else(|| "an off-shape route admitted them".to_string());
        for line in dim_line(
            &format!(
                "\u{b7} off-shape \u{b7} {} fact{} outside the pool \u{b7} admitted because {route}",
                assembly.spillover.len(),
                plural(assembly.spillover.len()),
            ),
            width,
        ) {
            out.push_str(&line);
            out.push('\n');
        }
        for hit in &assembly.spillover {
            out.push_str(&render_recall_card(hit, width));
        }
    }
    let block = render_assembly(&assembly);
    if block.is_empty() {
        for line in dim_line(
            "nothing would enter a turn's context for this instruction",
            width,
        ) {
            out.push_str(&line);
            out.push('\n');
        }
    } else {
        for line in dim_line(
            &format!(
                "what would enter a turn's context ({} of {} tokens):",
                assembly.used, assembly.budget
            ),
            width,
        ) {
            out.push_str(&line);
            out.push('\n');
        }
        out.push_str(&block);
    }
    if !assembly.dropped.is_empty() {
        for line in dim_line(
            &format!("dropped under budget: {}", assembly.dropped.join(", ")),
            width,
        ) {
            out.push_str(&line);
            out.push('\n');
        }
    }
    for line in dim_line(
        "a dry run \u{b7} no recall-count changed and no row recorded; the turn path records hits",
        width,
    ) {
        out.push_str(&line);
        out.push('\n');
    }
    out
}

/// One ranked fact's card: the name with its score and its sign, then
/// the channels that produced the score — the "why", each dimension
/// named so the operator sees what the fact's evidence IS — then the
/// fact. An anti-vector's sign renders in the divergence's color and an
/// incomplete one beside it, and an off-shape fact says so. The line
/// bound is [`SURFACE_WIDTH`] from [`render_recall`] and whatever a
/// test narrows it to through [`render_recall_at`].
fn render_recall_card(hit: &ScoredFact, width: usize) -> String {
    let sign = hit.fact.sign();
    // The three signs are visually distinct: a divergence renders in
    // the caution color, an incomplete one in the accent, a resolution
    // in the plain text color.
    let score_color = match sign {
        Sign::Resolution => TEXT,
        Sign::Divergence => WARN,
        Sign::Incomplete => ACCENT,
    };
    let off = if hit.off_shape { " off-shape" } else { "" };
    let header_right = format!("{:.2} {}{off}", hit.score.total, sign.marker());
    let header = format!(
        "{DIM}\u{250c}\u{2500}{RESET} {BOLD}{ACCENT}{name}{RESET} {DIM}{dashes} {score_color}{right}{RESET} {DIM}\u{2510}{RESET}",
        name = hit.fact.name,
        dashes = dashes_for(&format!(" {} ", hit.fact.name), &header_right),
        right = header_right,
    );
    let mut lines: Vec<String> = Vec::new();
    // The channels render as fixed lines rather than one wrapped one:
    // the wrap point would fall mid-phrase and the "why" must read as
    // one fact. The topical pair, then the trajectory — the fact's
    // direction — then the magnitude and the pool it came from.
    lines.push(format!(
        "{DIM}\u{2502}{RESET} {DIM}vector {:.2} \u{b7} keyword {:.2} \u{b7} trajectory {:.2}{RESET}",
        hit.score.vector, hit.score.keyword, hit.score.trajectory
    ));
    let shape = if hit.fact.shape.is_empty() {
        "\u{2014}".to_string()
    } else {
        hit.fact.shape.join(", ")
    };
    let standing = format!(
        "shape: {shape} \u{b7} history {} \u{b7} freshness {:.2} \u{b7} provenance {}",
        hit.fact.recall_count,
        hit.score.freshness,
        hit.fact.basis.label()
    );
    let standing = if sign == Sign::Divergence {
        format!("{WARN}{standing}{RESET}")
    } else {
        standing
    };
    // The standing line is a concatenation that grows with the tags, so
    // it is bounded to the card's own width and wrapped rather than run
    // past the border (the KI-078 class, on this surface). Two columns
    // of the width are the card's own `│ ` prefix.
    for chunk in wrap_detail(&standing, (width - 2).min(CARD_WIDTH - 4)) {
        lines.push(format!("{DIM}\u{2502}{RESET} {DIM}{chunk}{RESET}"));
    }
    for chunk in wrap_detail(&hit.fact.description, CARD_WIDTH - 4) {
        lines.push(format!("{DIM}\u{2502}{RESET} {TEXT}{chunk}{RESET}"));
    }
    // The missing companions ride the recall card too, beside the
    // direction: a stalled trajectory's whole point is the named absent
    // pieces, and the operator reads them where the fact is ranked.
    if let Some(missing) = missing_note(&hit.fact) {
        for chunk in wrap_detail(&format!("missing: {missing}"), CARD_WIDTH - 4) {
            lines.push(format!("{DIM}\u{2502}{RESET} {DIM}{chunk}{RESET}"));
        }
    }
    let bottom = format!(
        "{DIM}\u{2514}{}\u{2518}{RESET}",
        "\u{2500}".repeat(CARD_WIDTH - 2)
    );
    let mut card = format!("{header}\n");
    for line in lines {
        card.push_str(&line);
        card.push('\n');
    }
    card.push_str(&bottom);
    card.push('\n');
    card
}

/// The card's tier suffix: named only when it is not `durable`, so a
/// durable card reads exactly as it did before the tier field existed
/// and a working or episodic one says so.
fn tier_suffix(fact: &Fact) -> String {
    if fact.tier == Tier::Durable {
        String::new()
    } else {
        format!(" \u{b7} tier: {}", fact.tier.label())
    }
}

/// One fact as a card — the `fleet remember` confirmation's body, so a
/// write shows the fact it wrote in the house style.
pub fn render_one(fact: &Fact) -> String {
    render_card(fact)
}

/// Render one consolidation pass as cards: one per action that ran,
/// with every fact it touched and the reason it carried, then the
/// confirmed-by facts it left alone and the summary it folded into.
/// An empty pass says so rather than rendering nothing.
pub fn render_consolidation(result: &Consolidation) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "{BOLD}fleet memories --consolidate{RESET} {DIM}\u{2014} the consolidation cycle{RESET}\n\n"
    ));
    if result.is_empty() {
        out.push_str(&format!(
            "{DIM}nothing to consolidate \u{b7} every fact is within its bounds{RESET}\n"
        ));
        if !result.untouched.is_empty() {
            out.push_str(&untouched_line(&result.untouched));
        }
        if !result.kept.is_empty() {
            out.push_str(&kept_line(&result.kept));
        }
        return out;
    }
    for action in calib::ACTIONS {
        let taken: Vec<&CycleAction> = result
            .actions
            .iter()
            .filter(|one| one.action == action)
            .collect();
        if taken.is_empty() {
            continue;
        }
        let header_right = format!("{} fact{}", taken.len(), plural(taken.len()));
        let header = format!(
            "{DIM}\u{250c}\u{2500}{RESET} {BOLD}{ACCENT}{action}{RESET} {DIM}{dashes} {TEXT}{right}{RESET} {DIM}\u{2510}{RESET}",
            dashes = dashes_for(&format!(" {action} "), &header_right),
            right = header_right,
        );
        let mut lines: Vec<String> = Vec::new();
        for one in &taken {
            let text = format!("{} \u{2014} {}", one.fact, one.reason);
            for chunk in wrap_detail(&text, CARD_WIDTH - 4) {
                lines.push(format!("{DIM}\u{2502}{RESET} {TEXT}{chunk}{RESET}"));
            }
        }
        let bottom = format!(
            "{DIM}\u{2514}{}\u{2518}{RESET}",
            "\u{2500}".repeat(CARD_WIDTH - 2)
        );
        let mut card = format!("{header}\n");
        for line in lines {
            card.push_str(&line);
            card.push('\n');
        }
        card.push_str(&bottom);
        card.push('\n');
        out.push_str(&card);
    }
    if !result.untouched.is_empty() {
        out.push_str(&untouched_line(&result.untouched));
    }
    if !result.kept.is_empty() {
        out.push_str(&kept_line(&result.kept));
    }
    if let Some(summary) = &result.summary {
        out.push_str(&format!("{DIM}folded into: {summary}{RESET}\n"));
    }
    out
}

/// The one line naming the confirmed-by facts the cycle left alone:
/// the provenance rule, visible rather than silent.
fn untouched_line(untouched: &[String]) -> String {
    format!(
        "{DIM}left alone: {} \u{b7} confirmed-by, so the cycle never retires one without approval{RESET}\n",
        untouched.join(", ")
    )
}

/// The one line naming the incomplete facts the cycle left alone: the
/// third-vector rule, visible rather than silent.
fn kept_line(kept: &[String]) -> String {
    format!(
        "{DIM}left alone: {} \u{b7} incomplete, so the cycle never folds one into a summary \u{2014} its missing companions cannot ride a bullet{RESET}\n",
        kept.join(", ")
    )
}

/// Render the archive as cards: the facts the cycle's forget action
/// moved out of the working tree. Nothing here is deleted — a move
/// back restores a fact, and git remembers the move. The read is the
/// archive's own: a collision copy's file name carries its serial
/// suffix, so the card names the fact its frontmatter carries.
pub fn render_archive(root: &Path) -> String {
    let facts = load_facts_archived(&root.join(ARCHIVE_DIR));
    let mut out = String::new();
    out.push_str(&format!(
        "{BOLD}fleet memories --archive{RESET} {DIM}\u{2014} {} forgotten fact{} from {}/{}/{RESET}\n\n",
        facts.len(),
        plural(facts.len()),
        MEMORIES_DIR,
        ARCHIVE_DIR,
    ));
    if facts.is_empty() {
        out.push_str(&format!(
            "{DIM}the archive is empty \u{b7} the cycle moves an expired fact here, and a move back restores it{RESET}\n"
        ));
        return out;
    }
    for fact in &facts {
        out.push_str(&render_card(fact));
    }
    out
}

/// The dash run that pads a card header so its right edge aligns.
fn dashes_for(left: &str, right: &str) -> String {
    let used = left.chars().count() + right.chars().count() + 2;
    "\u{2500}".repeat(CARD_WIDTH.saturating_sub(used + 4).max(1))
}

/// The G8 defect's close, pinned: a settings write to the env-var
/// counterpart lands in `<home>/fleet.env`, and the endpoint
/// resolver reads it back through the same fallback the runtime
/// walks. The write had been landing in a file nothing read — this
/// is the round-trip that makes the settings surface real, and it
/// runs with no process environment involved.
#[test]
fn an_env_write_is_readable_by_the_runtime() {
    let home = std::env::temp_dir().join(format!("fleet-env-rt-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).expect("a temp home");
    crate::settings::set(
        &home,
        "FLEET_EMBEDDING_ENDPOINT",
        "http://127.0.0.1:9999/v1/embeddings",
    )
    .expect("the write");
    let read = crate::home::read_fleet_env(&home);
    assert_eq!(
        read.get(EMBEDDING_ENDPOINT_VAR).map(String::as_str),
        Some("http://127.0.0.1:9999/v1/embeddings"),
        "the runtime reads what the writer wrote"
    );
    // The round-trip "runs with no process environment involved" (the
    // comment above is the premise): a machine that exports the var —
    // this one live-serves the embedder with it set — would otherwise
    // answer from its ambient value and the test would pin the env's
    // precedence instead of the write. The lookup shadows only the var
    // under test, keeping the resolver's env-first shape intact for
    // every other name.
    let resolved = endpoint_from(|name| {
        if name == EMBEDDING_ENDPOINT_VAR {
            read.get(name).cloned()
        } else {
            std::env::var(name).ok().or_else(|| read.get(name).cloned())
        }
    });
    assert_eq!(resolved, "http://127.0.0.1:9999/v1/embeddings");
    let _ = std::fs::remove_dir_all(&home);
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as _;
    use std::os::unix::fs::PermissionsExt;
    use std::sync::Arc;

    #[test]
    fn the_probe_retries_after_a_pause_so_a_flap_is_not_a_missing_backend() {
        // KI-296: the machine's endpoint flaps — a request hangs past
        // every budget, an identical one answers in 37 ms between them,
        // and a restart window refuses outright. A probe that reads one
        // failure as "no backend" degrades a healthy machine for a
        // whole process. The double refuses the first four connections
        // — the embed's own immediate retry and the probe's first
        // paused retry among them — and answers within the second,
        // which is also the margin a loaded suite needs: a try can fail
        // before it ever reaches the backend, and the probe's own
        // retry count is the bound the double is sized against.
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        std::thread::spawn(move || {
            for (index, stream) in listener.incoming().enumerate() {
                let mut stream = match stream {
                    Ok(stream) => stream,
                    Err(_) => break,
                };
                if index < 4 {
                    drop(stream);
                    continue;
                }
                // Read the request before answering: dropping a stream
                // that still holds unread request bytes sends RST, and
                // an RST discards what is already sitting in the
                // client's receive queue — under a loaded suite the
                // reply was lost to exactly that, and every try failed.
                let mut seen = 0;
                while seen < 10 {
                    let mut scratch = [0u8; 4096];
                    match std::io::Read::read(&mut stream, &mut scratch) {
                        Ok(0) => break,
                        Ok(read) => {
                            seen += 1;
                            if scratch[..read].windows(4).any(|w| w == b"\r\n\r\n") {
                                break;
                            }
                        }
                        Err(_) => break,
                    }
                }
                let body = r#"{"data":[{"embedding":[0.5,0.25,0.125]}]}"#;
                let _ = stream.write_all(
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\
                         Content-Length: {}\r\nConnection: close\r\n\r\n{}",
                        body.len(),
                        body
                    )
                    .as_bytes(),
                );
            }
        });
        let embedder = HttpEmbedder::probe(&format!("http://127.0.0.1:{port}/v1/embeddings"));
        let embedder = embedder.expect("the probe read the flap as no backend");
        assert_eq!(embedder.dims, 3, "the dimensionality the probe found");
    }

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

    /// Make one fact file immutable so its rewrite cannot land: the
    /// read still works and the file still loads, so a supersession
    /// reaches its stamp and fails exactly at the rewrite's rename —
    /// the deterministic seam the stamp-failure tests need (a
    /// read-only file would not do: the rewrite is a temp write plus a
    /// rename, and a rename replaces a read-only file without
    /// consulting its mode). macOS's `chflags uchg`. The helper fails
    /// the test when the flag could not be set, so a machine without
    /// `chflags` fails loudly rather than passing a supersession that
    /// actually landed.
    fn set_immutable(path: &Path, immutable: bool) {
        let flag = if immutable { "uchg" } else { "nouchg" };
        let status = std::process::Command::new("chflags")
            .arg(flag)
            .arg(path)
            .status()
            .expect("`chflags` is available on this machine");
        assert!(
            status.success(),
            "`chflags {flag}` failed on {}",
            path.display()
        );
    }

    /// A well-formed fact in the writer's field order.
    fn good_fact(name: &str, description: &str, fact_type: &str, shape: &str) -> String {
        format!(
            "---\nname: {name}\ndescription: \"{description}\"\ntype: {fact_type}\ndate: {today}\nbasis: confirmed-by\nshape: [{shape}]\nrecall-count: 0\nsuperseded-by:\nvalid-until:\n---\n\nThe full content of {name}.\n",
            today = calib::today()
        )
    }

    /// A well-formed fact with a recall history — the history channel's
    /// input, the magnitude the ranking reads.
    fn recalled_fact(name: &str, description: &str, shape: &str, recalls: u64) -> String {
        format!(
            "---\nname: {name}\ndescription: \"{description}\"\ntype: project\ndate: {today}\nbasis: confirmed-by\nshape: [{shape}]\nrecall-count: {recalls}\nsuperseded-by:\nvalid-until:\n---\n\nThe full content of {name}.\n",
            today = calib::today()
        )
    }

    /// A fact carrying the resolution's phrases — the trajectory
    /// channel's own text — and its recall history, in the writer's
    /// field order.
    fn trajectory_fact(
        name: &str,
        description: &str,
        shape: &str,
        phrases: &[&str],
        recalls: u64,
    ) -> String {
        format!(
            "---\nname: {name}\ndescription: \"{description}\"\ntype: project\ndate: {today}\nbasis: confirmed-by\nshape: [{shape}]\nphrases: [{}]\nrecall-count: {recalls}\nsuperseded-by:\nvalid-until:\n---\n\nThe full content of {name}.\n",
            phrases.join(", "),
            today = calib::today()
        )
    }

    /// A fact carrying the third vector's own vocabulary: the outcome
    /// tag, the recorded phrases, and the missing companions — in the
    /// writer's field order.
    fn incomplete_fact(
        name: &str,
        description: &str,
        shape: &str,
        phrases: &[&str],
        missing: &[&str],
        recalls: u64,
    ) -> String {
        format!(
            "---\nname: {name}\ndescription: \"{description}\"\ntype: project\ndate: {today}\nbasis: confirmed-by\nshape: [{shape}]\nphrases: [{}]\nmissing: [{}]\nrecall-count: {recalls}\nsuperseded-by:\nvalid-until:\n---\n\nThe full content of {name}.\n",
            phrases.join(", "),
            missing.join(", "),
            today = calib::today()
        )
    }

    fn fact_names(facts: &[Fact]) -> Vec<&str> {
        facts.iter().map(|fact| fact.name.as_str()).collect()
    }

    fn hit_names(hits: &[ScoredFact]) -> Vec<&str> {
        hits.iter().map(|hit| hit.fact.name.as_str()).collect()
    }

    #[test]
    fn the_two_stage_prefilter_keeps_the_neighborhood_and_ranks_the_rest_out() {
        // KI-252 move 2, unit-level: with the setting on, the queried
        // fact keeps its full-width score (it survives the slice stage)
        // and the shape of a slice-ranked-out fact is the vector term at
        // 0.0 — the same shape a partial embed leaves, by construction.
        let (_calls, embedder) = ScriptedEmbedder::new();
        let root = scratch("two-stage");
        write_fact(
            &root,
            "deploy-note",
            &recalled_fact(
                "deploy-note",
                "How the deploy pipeline works on railway",
                "How the deploy pipeline works",
                0,
            ),
        );
        write_fact(
            &root,
            "walnut-note",
            &recalled_fact(
                "walnut-note",
                "A walnut unrelated to deploys or renders",
                "A walnut note",
                0,
            ),
        );
        let memory = Memory::load_with(&root, embedder);
        memory
            .two_stage
            .store(true, std::sync::atomic::Ordering::Relaxed);

        // Directly: the prefilter over three synthetic vectors keeps the
        // top-K by slice and only the slice.
        // Real variance in the slice: `near` mirrors the query's first
        // 128 dims, `far` inverts them — constant vectors are all
        // parallel under cosine and would prove nothing.
        let mut query_vec = vec![0.0; SLICE_DIMS + 4];
        for (index, value) in query_vec.iter_mut().enumerate().take(SLICE_DIMS) {
            *value = if index % 2 == 0 { 1.0 } else { -1.0 };
        }
        let query = Some(query_vec);
        let mut vectors: HashMap<String, Vec<f32>> = HashMap::new();
        vectors.insert("near".to_string(), query.clone().unwrap());
        vectors.insert(
            "far".to_string(),
            query
                .clone()
                .unwrap()
                .iter()
                .take(SLICE_DIMS)
                .map(|value| -value)
                .chain(vec![0.0; 4])
                .collect(),
        );
        let prefilter = vector_prefilter(&query, &vectors, 1);
        assert!(
            prefilter
                .as_ref()
                .is_some_and(|members| members.contains("near") && !members.contains("far")),
            "{prefilter:?}"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// The measurement harness (KI-252 move 2's gate, C1 of its plan):
    /// over the real corpus, recall@10 single-pass versus two-stage and
    /// both wall clocks. `#[ignore]`d because it grades the machine's
    /// live corpus and embedder — run it with `cargo test
    /// --lib the_two_stage_measurement -- --ignored --nocapture` and let
    /// the printed gate decide `memory.vectors.two_stage`'s default.
    #[test]
    #[ignore = "grades the live corpus and embedder; run with --ignored"]
    fn the_two_stage_measurement() {
        let root = crate::memory::resolve_root(&std::env::current_dir().unwrap_or_default());
        let memory = Memory::load(&root);
        let facts = memory.fact_count();
        let instructions = [
            "how does the router choose a provider",
            "the gate's four commands",
            "what is the calibration stream",
            "blend1's corpus refresh spawn route",
            "the plan-as-contract discipline",
            "how recall ranks facts",
            "the tui's session surface",
            "credentials resolve from where",
            "the deck tool's layouts",
            "what the lifecycle sweep disables",
        ];
        let measure = |two_stage: bool| {
            memory
                .two_stage
                .store(two_stage, std::sync::atomic::Ordering::Relaxed);
            let started = std::time::Instant::now();
            let mut runs: Vec<Vec<String>> = Vec::new();
            for instruction in &instructions {
                let hits = memory.recall(instruction);
                runs.push(
                    hits.iter()
                        .take(10)
                        .map(|hit| hit.fact.name.clone())
                        .collect(),
                );
            }
            (started.elapsed().as_millis(), runs)
        };
        let (plain_ms, plain_runs) = measure(false);
        let (two_ms, two_runs) = measure(true);
        let mut overlaps = Vec::new();
        for (index, (plain, two)) in plain_runs.iter().zip(two_runs.iter()).enumerate() {
            let shared = plain.iter().filter(|name| two.contains(name)).count();
            overlaps.push(shared);
            println!(
                "q{index:02}: overlap@10 {shared}/10 (single {plain_ms}ms total, two-stage {two_ms}ms total)"
            );
        }
        let mean_overlap: f64 =
            overlaps.iter().map(|o| *o as f64).sum::<f64>() / overlaps.len() as f64 / 10.0;
        println!(
            "the gate: mean recall@10 agreement {mean_overlap:.3} (≥0.98 passes) over {facts} facts; single {plain_ms}ms vs two-stage {two_ms}ms for {} instructions",
            instructions.len()
        );
    }

    /// A deterministic embedder over a fixed scripted vocabulary: each
    /// text embeds as the indicator of which scripted terms it carries,
    /// so a cosine is a scripted-token overlap and every ordering below
    /// is exact. It shares its call counter with the test, which is how
    /// the index-cache test proves a rebuild embeds only what changed,
    /// and it serves a settable dimensionality, which is how the
    /// index-key test swaps models.
    struct ScriptedEmbedder {
        calls: Arc<AtomicUsize>,
        dims: usize,
    }

    impl ScriptedEmbedder {
        /// The shared counter and the boxed embedder that increments it,
        /// serving the whole scripted vocabulary.
        fn new() -> (Arc<AtomicUsize>, Box<ScriptedEmbedder>) {
            Self::at_dims(SCRIPTED_TERMS.len())
        }

        /// The same embedder at an explicit dimensionality — the model
        /// swap. A dimension past the vocabulary's length truncates it,
        /// so the vectors stay exact indicators over fewer terms.
        fn at_dims(dims: usize) -> (Arc<AtomicUsize>, Box<ScriptedEmbedder>) {
            let calls = Arc::new(AtomicUsize::new(0));
            (
                Arc::clone(&calls),
                Box::new(ScriptedEmbedder { calls, dims }),
            )
        }
    }

    /// The scripted vocabulary, in dimension order.
    const SCRIPTED_TERMS: [&str; 5] = ["deploy", "railway", "render", "shader", "walnut"];

    /// An embedder that fails for one text and serves the scripted
    /// vocabulary for the rest — the embedding server that is down for
    /// one request, or a fact whose embed times out. It writes the same
    /// index key the scripted embedder does, so a load with one reads
    /// the cache the other wrote.
    struct PartialEmbedder {
        calls: Arc<AtomicUsize>,
        fails_on: String,
    }

    impl Embedder for PartialEmbedder {
        fn box_clone(&self) -> Box<dyn Embedder> {
            Box::new(PartialEmbedder {
                calls: Arc::clone(&self.calls),
                fails_on: self.fails_on.clone(),
            })
        }

        fn embed(&self, text: &str) -> Option<Vec<f32>> {
            self.calls.fetch_add(1, Ordering::Relaxed);
            if text.to_lowercase().contains(&self.fails_on) {
                return None;
            }
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

        fn vector_info(&self) -> Option<(String, usize)> {
            Some(("scripted".to_string(), SCRIPTED_TERMS.len()))
        }
    }

    impl Embedder for ScriptedEmbedder {
        fn box_clone(&self) -> Box<dyn Embedder> {
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

    // --- the frontmatter subset ---

    /// KI-067: the confirmed-by gate's summary carries the target's own
    /// description beside the justification, so the approver decides
    /// with what the fact currently says in front of them.
    /// An empty `memories/` under the home is the install's scaffold,
    /// not a corpus: it must not shadow a real corpus the walk finds.
    /// The regression this pins was introduced by the home-first rule
    /// itself before the holds-facts check was added.
    #[test]
    fn an_empty_home_scaffold_does_not_shadow_the_walks_corpus() {
        let home = scratch("scaffold-shadow");
        let foreign = scratch("scaffold-foreign");
        fs::create_dir_all(home.join(MEMORIES_DIR)).unwrap();
        fs::create_dir_all(foreign.join("nested").join(MEMORIES_DIR)).unwrap();
        std::fs::write(
            foreign.join("nested").join(MEMORIES_DIR).join("real.md"),
            "---\nname: real\n---\n",
        )
        .unwrap();
        // The home's directory exists and holds nothing: the walk's
        // corpus wins.
        let resolved = resolve_root_from(&foreign.join("nested"), Some(&home));
        assert_eq!(
            resolved,
            foreign.join("nested").join(MEMORIES_DIR),
            "{resolved:?}"
        );
        let _ = std::fs::remove_dir_all(&home);
        let _ = std::fs::remove_dir_all(&foreign);
    }

    #[test]
    fn a_supersede_summary_carries_the_targets_current_description() {
        let with_target = supersede_summary(
            &serde_json::json!({
                "name": "a-fact",
                "justification": "the measurement moved",
                "target_description": "the old sentence the corpus carries",
            })
            .to_string(),
            Some(Basis::ConfirmedBy),
        );
        assert!(
            with_target.contains("replaces: the old sentence the corpus carries"),
            "{with_target}"
        );
        assert!(
            with_target.contains("the measurement moved"),
            "{with_target}"
        );
        assert!(with_target.contains("confirmed-by"), "{with_target}");
        // Without a target description — the ordinary supersede — the
        // summary is exactly what it was.
        let plain = supersede_summary(
            &serde_json::json!({"name": "a-fact", "justification": "why"}).to_string(),
            None,
        );
        assert!(!plain.contains("replaces:"), "{plain}");
        assert!(plain.contains("why"), "{plain}");
    }

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
        assert_eq!(
            shape_tags(document.get("shape")),
            vec!["business_analysis", "deploy"]
        );
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
        // The phrases field reads as a bracketed list too, but
        // case-preserving: they are the vocabulary that worked, quoted
        // as it was used.
        let phrased = parse_document(
            "---\nname: demo\ndescription: d\nphrases: [Read CLAUDE.local.md directly, never grep-extract credentials]\n---\nbody\n",
        )
        .unwrap();
        assert_eq!(
            phrase_list(phrased.get("phrases")),
            vec![
                "Read CLAUDE.local.md directly".to_string(),
                "never grep-extract credentials".to_string()
            ]
        );
        // A bare scalar reads as no phrases, the shape field's own rule.
        assert!(phrase_list(Some("read the docs")).is_empty());
        assert!(phrase_list(None).is_empty());
    }

    #[test]
    fn the_parser_rejects_what_it_does_not_speak() {
        // No opening fence.
        assert!(parse_document("name: demo\n")
            .unwrap_err()
            .contains("does not open"));
        // No closing fence.
        assert!(parse_document("---\nname: demo\n")
            .unwrap_err()
            .contains("never closes"));
        // A list entry.
        assert!(parse_document("---\n- item\n---\nbody\n")
            .unwrap_err()
            .contains("key: value"));
        // A bare scalar line.
        assert!(parse_document("---\nbare\n---\nbody\n")
            .unwrap_err()
            .contains("key: value"));
        // An indented line with no field to continue.
        assert!(parse_document("---\n  orphan\n---\nbody\n")
            .unwrap_err()
            .contains("no field to continue"));
        // A duplicated key.
        assert!(
            parse_document("---\nname: a\nname: b\ndescription: d\n---\nbody\n")
                .unwrap_err()
                .contains("twice")
        );
        // An unbalanced quoted key.
        assert!(
            parse_document("---\n\"name: demo\ndescription: d\n---\nbody\n")
                .unwrap_err()
                .contains("unbalanced quoted key")
        );
        // A missing required key is the loader's rule, not the parser's.
        let document = parse_document("---\ntype: project\n---\nbody\n").unwrap();
        assert!(document.required("name").unwrap_err().contains("name"));
        assert!(document
            .required("description")
            .unwrap_err()
            .contains("description"));
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
        write_fact(
            &root,
            "good-one",
            &good_fact("good-one", "the first fact", "project", "deploy"),
        );
        write_fact(
            &root,
            "good-two",
            &good_fact("good-two", "the second fact", "reference", ""),
        );
        // The bare minimum: no basis, no recall-count, no type — the
        // loader's defaults apply rather than a rejection.
        write_fact(
            &root,
            "minimal",
            "---\nname: minimal\ndescription: \"the bare minimum\"\n---\n\nbody\n",
        );
        // Five ways to be malformed: no frontmatter, a missing
        // description, a name that is not the file stem, a basis outside
        // the vocabulary, and a recall-count that is not an integer.
        write_fact(&root, "broken-fence", "# no fence here\n");
        write_fact(
            &root,
            "broken-fields",
            "---\nname: broken-fields\n---\nbody\n",
        );
        write_fact(
            &root,
            "broken-name",
            &good_fact("some-other-name", "d", "project", ""),
        );
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
        write_fact(
            &root,
            "identity",
            &good_fact("identity", "who the owner is", "user", ""),
        );
        write_fact(
            &root,
            "rule",
            &good_fact("rule", "a standing rule", "feedback", ""),
        );
        write_fact(
            &root,
            "task",
            &good_fact("task", "project state", "project", ""),
        );
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
        write_fact(
            root,
            "alpha",
            &good_fact("alpha", "railway deploy per branch", "project", ""),
        );
        write_fact(
            root,
            "zeta",
            &good_fact("zeta", "deploy railway", "project", ""),
        );
        write_fact(
            root,
            "eta",
            &good_fact("eta", "railway is failing", "project", ""),
        );
        write_fact(
            root,
            "junk",
            &good_fact("junk", "the walnut table texture", "project", ""),
        );
    }

    #[test]
    fn the_hybrid_ranker_orders_a_scripted_corpus_deterministically() {
        let root = scratch("order");
        order_corpus(&root);

        // Vector-off: the keyword sub-channel decides. `eta` covers
        // three of the five query tokens against `alpha`/`zeta`'s two;
        // `junk` covers one — a stopword — and lands under the floor.
        let keyword_only = Memory::load_with(&root, Box::new(KeywordOnly));
        let off: Vec<String> = keyword_only
            .recall("the railway deploy is failing")
            .into_iter()
            .map(|hit| hit.fact.name)
            .collect();
        assert_eq!(
            off,
            vec!["eta".to_string(), "alpha".to_string(), "zeta".to_string()]
        );
        // The same corpus ranks the same way twice: deterministic.
        let again: Vec<String> = keyword_only
            .recall("the railway deploy is failing")
            .into_iter()
            .map(|hit| hit.fact.name)
            .collect();
        assert_eq!(off, again);
        // The floor dropped the junk: an unrelated fact never enters.
        // Its whole score is freshness (0.10) plus provenance (0.05)
        // plus one stopword's overlap (0.15 × 0.2 = 0.03) — 0.18, under
        // the floor.
        assert!(!off.contains(&"junk".to_string()));

        // Vector-on: the same corpus, the same query, a different order.
        // `alpha` and `zeta` match the query's vector exactly (cosine
        // 1.0) and overtake `eta`'s partial match.
        let (calls, embedder) = ScriptedEmbedder::new();
        let vector_on = Memory::load_with(&root, embedder);
        // The load-time index build embedded every fact exactly once;
        // the recalls below each embed the query beside it.
        assert_eq!(calls.load(Ordering::Relaxed), 4);
        // KI-249: the build's per-embed timings carry the same count, so
        // the caller's stream records one row per embed the bound
        // (`EMBED_TIMEOUT`) is about. A cached reload adds none.
        assert_eq!(
            vector_on.build_embeds_ms.len(),
            4,
            "{:?}",
            vector_on.build_embeds_ms
        );
        let reloaded = Memory::load_with(&root, Box::new(KeywordOnly));
        assert!(reloaded.build_embeds_ms.is_empty());
        let on: Vec<String> = vector_on
            .recall("the railway deploy is failing")
            .into_iter()
            .map(|hit| hit.fact.name)
            .collect();
        assert_eq!(
            on,
            vec!["alpha".to_string(), "zeta".to_string(), "eta".to_string()]
        );
        // The vector sub-channel is what moved the order, and the two
        // topical sub-channels are the only thing that separates these
        // three — no trajectory, no history, one basis.
        let hits = vector_on.recall("the railway deploy is failing");
        assert!(
            (hits[0].score.vector - 1.0).abs() < 1e-9,
            "{:?}",
            hits[0].score
        );
        let eta = hits
            .iter()
            .find(|hit| hit.fact.name == "eta")
            .expect("eta ranks");
        assert!(
            (eta.score.vector - 1.0 / 2.0_f64.sqrt()).abs() < 1e-9,
            "{:?}",
            eta.score
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn the_named_weights_compose_the_score_and_no_channel_dominates() {
        let today = parse_day(&calib::today());
        // A fact dated today with the ceiling's recalls: history
        // saturates, freshness is fresh.
        let score = Score::compute(
            0.8,
            0.5,
            0.5,
            RECALL_CEILING,
            Basis::ConfirmedBy,
            &calib::today(),
            today,
        );
        let expected = W_VECTOR * 0.8
            + W_KEYWORD * 0.5
            + W_TRAJECTORY * 0.5
            + W_HISTORY * 1.0
            + W_FRESHNESS * 1.0
            + W_PROVENANCE * PROVENANCE_CONFIRMED;
        assert!((score.total - expected).abs() < 1e-12, "{score:?}");
        // No single channel dominates: the weights sum to one and the
        // largest single one is a minority of the composite, so a fact
        // that is strong in one dimension and empty in the rest cannot
        // outrank a fact that is moderate in several. That is the
        // owner's rule — a per-dimension minimum is meaningless in a
        // multidimensional context.
        let weights = [
            W_VECTOR,
            W_KEYWORD,
            W_TRAJECTORY,
            W_HISTORY,
            W_FRESHNESS,
            W_PROVENANCE,
        ];
        let sum: f64 = weights.iter().sum();
        assert!((sum - 1.0).abs() < 1e-12, "{weights:?}");
        assert!(
            weights.iter().all(|weight| *weight < 0.5),
            "no channel may carry half the composite: {weights:?}"
        );
        // The topical pair — the neighborhood — is less than the rest
        // of the evidence put together, so talking about the
        // neighborhood cannot outrank driving toward the minima.
        let topical = W_VECTOR + W_KEYWORD;
        assert!(topical < 1.0 - topical, "the topical pair is {topical}");
        // History saturates: past the ceiling, more recalls score the
        // same. The ceiling is 128 and the curve is logarithmic, so the
        // saturation is a clamp at the ceiling rather than a cliff
        // partway up — which is what kept 16 of 24 corpus facts (median
        // recall-count 56) pinned at the same 0.250 term.
        let saturated = Score::compute(0.0, 0.0, 0.0, 200, Basis::Inferred, &calib::today(), today);
        let at_ceiling = Score::compute(
            0.0,
            0.0,
            0.0,
            RECALL_CEILING,
            Basis::Inferred,
            &calib::today(),
            today,
        );
        assert_eq!(saturated.history, at_ceiling.history);
        // And it still discriminates across the observed range instead
        // of clamping two-thirds of the corpus to the same term.
        let spread = history_term(0) < history_term(1)
            && history_term(1) < history_term(8)
            && history_term(8) < history_term(56)
            && history_term(56) < history_term(RECALL_CEILING);
        assert!(spread, "the curve must separate the observed range");
        // A confirmed-by vector outweighs an inferred one, and neither
        // is erased: provenance is magnitude, not a gate.
        let confirmed =
            Score::compute(0.0, 0.0, 0.0, 0, Basis::ConfirmedBy, &calib::today(), today);
        let inferred = Score::compute(0.0, 0.0, 0.0, 0, Basis::Inferred, &calib::today(), today);
        assert!(confirmed.provenance > inferred.provenance);
        assert!(inferred.provenance > 0.0);
        // A fact with no readable date contributes no freshness term.
        assert_eq!(
            Score::compute(0.0, 0.0, 0.0, 0, Basis::Inferred, "not-a-date", today).freshness,
            0.0
        );
        // Freshness halves at the half-life.
        let decayed = Score::compute(0.0, 0.0, 0.0, 0, Basis::Inferred, &days_ago(90), today);
        assert!((decayed.freshness - 0.5).abs() < 1e-9, "{decayed:?}");
        // The floor's arithmetic, exact. Standing alone — freshness
        // plus provenance, no match at all — does not clear it.
        let standing = W_FRESHNESS * 1.0 + W_PROVENANCE * PROVENANCE_CONFIRMED;
        assert!(standing < MIN_RECALL_SCORE, "{standing}");
        // A quarter-strength match in a DIRECTION channel beside
        // standing clears it: the owner's ruling reads in miniature,
        // because direction outranks proximity.
        assert!(
            (W_TRAJECTORY * 0.25 + standing - 0.225).abs() < 1e-12,
            "{}",
            W_TRAJECTORY * 0.25 + standing
        );
        assert!(
            (W_HISTORY * 0.25 + standing - 0.2125).abs() < 1e-12,
            "{}",
            W_HISTORY * 0.25 + standing
        );
        assert!(W_TRAJECTORY * 0.25 + standing > MIN_RECALL_SCORE);
        assert!(W_HISTORY * 0.25 + standing > MIN_RECALL_SCORE);
        // The topical pair needs a third of its weight to reach the
        // floor, not a quarter: a fact that merely sits near is not
        // enough on its own.
        let topical_third = W_VECTOR * (1.0 / 3.0) + standing;
        assert!(
            (topical_third - MIN_RECALL_SCORE).abs() < 1e-12,
            "{topical_third}"
        );
        assert!(W_KEYWORD * 0.25 + standing < MIN_RECALL_SCORE);

        // THE OWNER'S RULING, in arithmetic. The most a fact with NO
        // direction evidence can score is full topical coverage plus
        // standing — maximum proximity, zero direction.
        let proximity_max = W_VECTOR * 1.0
            + W_KEYWORD * 1.0
            + W_FRESHNESS * 1.0
            + W_PROVENANCE * PROVENANCE_CONFIRMED;
        assert!((proximity_max - 0.45).abs() < 1e-12, "{proximity_max}");
        // The direction-strong fact: a 0.8 trajectory match, four
        // recalls' history, confirmed-by, and a weak 0.2 cosine.
        let direction_strong = W_VECTOR * 0.2
            + W_TRAJECTORY * 0.8
            + W_HISTORY * history_term(4)
            + W_FRESHNESS * 1.0
            + W_PROVENANCE * PROVENANCE_CONFIRMED;
        assert!(
            (direction_strong - 0.5028).abs() < 1e-3,
            "{direction_strong}"
        );
        // Direction outranks proximity WITH MARGIN, not by a hair.
        assert!(
            direction_strong > proximity_max,
            "{direction_strong} must beat {proximity_max}"
        );
        // The margin is 0.053. It was 0.095 under the linear curve,
        // where four recalls already scored history 0.5 — and that
        // narrower margin is the honest consequence of R1: the
        // direction-strong fact no longer buys a quarter of its score
        // from being recalled four times. What is left is still
        // direction's own 0.24 of trajectory against the proximity
        // pair's 0.30, so the ruling holds; it just holds by the
        // weights' arithmetic rather than by a history subsidy.
        assert!(
            (direction_strong - proximity_max - 0.0528).abs() < 1e-3,
            "the margin is {}",
            direction_strong - proximity_max
        );
        // And the direction channels outweigh the proximity pair as
        // groups, which is what makes the margin structural rather
        // than a coincidence of these numbers.
        let proximity = W_VECTOR + W_KEYWORD;
        let direction = W_TRAJECTORY + W_HISTORY + W_PROVENANCE;
        assert!((proximity - 0.30).abs() < 1e-12, "{proximity}");
        assert!((direction - 0.60).abs() < 1e-12, "{direction}");
        assert!(direction > proximity, "{direction} vs {proximity}");

        // Full topical coverage alone, with nothing else, is under the
        // spillover floor: the neighborhood is not an answer. Under the
        // re-derived floor that holds for BOTH topical channels at once
        // — the maximum a directionless fact can reach is 0.45, and the
        // floor is 0.50, so a directionless fact can no longer stand
        // alone as the shape's own answer.
        let topical_only =
            W_KEYWORD * 1.0 + W_FRESHNESS * 1.0 + W_PROVENANCE * PROVENANCE_CONFIRMED;
        assert!(topical_only < SPILLOVER_FLOOR, "{topical_only}");
        assert!(proximity_max < SPILLOVER_FLOOR, "{proximity_max}");
        assert!((SPILLOVER_FLOOR - 0.50).abs() < 1e-12, "{SPILLOVER_FLOOR}");
        // A same-shape fact clears the floor with a 0.2 trajectory
        // match on top of full topical coverage, or two recalls.
        let corroborated = proximity_max + W_TRAJECTORY * 0.2;
        assert!(corroborated > SPILLOVER_FLOOR, "{corroborated}");
        assert!((corroborated - 0.51).abs() < 1e-12, "{corroborated}");
        assert!(proximity_max + W_HISTORY * history_term(2) > SPILLOVER_FLOOR);
        // The direction-strong fact clears it with room to spare.
        assert!(
            direction_strong - SPILLOVER_FLOOR > 0.0,
            "{direction_strong}"
        );
    }

    // --- shape routing ---

    #[test]
    fn shape_routing_classifies_each_shape_and_falls_back() {
        // One instruction per shape, each naming its shape's keywords.
        let cases = [
            (
                "push the staging branch and watch the railway deploy",
                "deploy",
            ),
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
        // an error: no shape, no partition, the surface alone answers.
        assert_eq!(classify_shape("tell me about the walnut table"), None);
        assert_eq!(classify_shape("a lesson"), None);

        // The failure axis classifies too — the avoidance half of the
        // shelf is routable. One instruction per failure shape, each
        // naming its class from Fleet's own failure record.
        let failures = [
            ("the lane failed open when sys1 was down", "fail-open"),
            ("the reader silently dropped the row", "drops-the-row"),
            ("the tower scan read was unbounded", "bound-exceeded"),
            (
                "the feature doc claims more than the code proved",
                "claim-outran-evidence",
            ),
            (
                "the card line clipped the summary mid-token",
                "fixed-width-clip",
            ),
            ("the run adopted a foreign root", "foreign-root"),
        ];
        for (instruction, expected) in failures {
            assert_eq!(classify_shape(instruction), Some(expected), "{instruction}");
        }
        // The structure table is consulted first: an instruction naming
        // both axes' terms routes under the structure shape, the fork's
        // order rule undisturbed.
        assert_eq!(
            classify_shape("the unbounded webpack bundle"),
            Some("build")
        );
        // An instruction with no shape's terms at all is still the
        // fallback, not an error.
        assert_eq!(
            classify_shape("the railway deploy is failing"),
            Some("deploy")
        );
        assert_eq!(classify_shape("the walnut table"), None);
        // The outcome axis classifies last, and only when nothing else
        // did: "the torch stack was missing" is the third sign's own
        // question, and a question that names a failure shape's terms
        // still routes to that shape — "what resolved the row drop"
        // asks about the drops-the-row class, not about outcomes.
        assert_eq!(
            classify_shape("the torch stack was missing"),
            Some("incomplete")
        );
        assert_eq!(
            classify_shape("the trajectory stalled partway"),
            Some("incomplete")
        );
        assert_eq!(
            classify_shape("what resolved the row drop"),
            Some("drops-the-row")
        );
        // And a question with no problem-shape term routes to the
        // outcome, which is the only way the third sign is reachable.
        assert_eq!(classify_shape("the trajectory was fixed"), Some("resolved"));
    }

    #[test]
    fn the_failure_shapes_name_fleets_own_failure_record() {
        // The taxonomy is the ledger's classes, named: six shapes, each
        // with the rows it is drawn from in docs/known-issues.md.
        assert_eq!(
            FAILURE_SHAPES,
            [
                "fail-open",
                "drops-the-row",
                "bound-exceeded",
                "claim-outran-evidence",
                "fixed-width-clip",
                "foreign-root",
            ]
        );
        for shape in FAILURE_SHAPES {
            assert!(is_failure_shape(shape), "{shape}");
            // Every failure shape has its own keyword entry, so it
            // classifies rather than sitting in the vocabulary unrouted.
            assert!(
                FAILURE_SHAPE_KEYWORDS
                    .iter()
                    .any(|(name, _)| *name == shape),
                "{shape} has no keywords"
            );
        }
        // The two axes are disjoint: a structure shape is never a
        // failure shape, so the sign derived from a tag is unambiguous.
        for shape in SHAPES {
            assert!(!is_failure_shape(shape), "{shape}");
            assert_eq!(shape_axis(shape), "structure");
        }
        assert_eq!(shape_axis("fail-open"), "failure");
        // The ledger's own sources, named against the rows: the
        // fixed-width clipping class (five recurrences — KI-078,
        // KI-085, KI-096, KI-115, KI-179 — with KI-055's general fix
        // still open) and the cwd-walk foreign-root class (four
        // surfaces, one root cause — KI-074, KI-087, KI-099, KI-106).
        assert_eq!(shape_axis("fixed-width-clip"), "failure");
        assert_eq!(shape_axis("foreign-root"), "failure");
        let clip = FAILURE_SHAPE_KEYWORDS
            .iter()
            .find(|(name, _)| *name == "fixed-width-clip")
            .expect("the clipping class has keywords");
        for term in [
            "fixed width",
            "mid-token",
            "clipped the",
            "overflows the pane",
        ] {
            assert!(clip.1.contains(&term), "{term} is not routable");
        }
        let root = FAILURE_SHAPE_KEYWORDS
            .iter()
            .find(|(name, _)| *name == "foreign-root")
            .expect("the foreign-root class has keywords");
        for term in ["foreign root", "wrong tree", "cwd walk", "walks up"] {
            assert!(root.1.contains(&term), "{term} is not routable");
        }
    }

    #[test]
    fn the_outcome_tags_are_a_third_axis_and_the_sign_reads_them_first() {
        // The outcome tags are not problem shapes: neither is a
        // structure shape, neither is a failure shape, and neither
        // ranks alone.
        for tag in OUTCOME_SHAPES {
            assert!(is_outcome_shape(tag), "{tag}");
            assert!(!is_failure_shape(tag), "{tag}");
            assert!(!SHAPES.contains(&tag), "{tag}");
            assert_eq!(shape_axis(tag), "outcome");
            // Every outcome tag classifies, so "the torch stack was
            // missing" routes somewhere rather than falling back.
            assert!(
                OUTCOME_SHAPE_KEYWORDS.iter().any(|(name, _)| *name == tag),
                "{tag} has no keywords"
            );
        }
        // The write path accepts them inside the three-tag cap. The
        // `incomplete` tag carries its companions, which R4b now requires
        // of it — a third sign with nothing named to finish the
        // trajectory is not a sign.
        let request = Remember::parse(&serde_json::json!({
            "name": "the-bootstrap-stalled",
            "description": "the bootstrap was correct but the torch stack was missing",
            "shape": "build, incomplete",
            "basis": "inferred",
            "missing": ["the-torch-stack"],
        }))
        .unwrap();
        assert_eq!(
            request.shape,
            vec!["build".to_string(), "incomplete".to_string()]
        );
        assert_eq!(request.missing, vec!["the-torch-stack".to_string()]);
        // A tag outside all three axes is refused with all three named.
        let error = Remember::parse(&serde_json::json!({
            "name": "typo",
            "description": "a fact with a typo'd outcome tag",
            "basis": "inferred",
            "shape": "stalled-out",
        }))
        .unwrap_err();
        assert!(
            matches!(&error, WriteError::Shape(reason) if reason.contains("incomplete, resolved")),
            "{error}"
        );
    }

    #[test]
    fn an_unclassified_instruction_answers_from_the_surface_alone() {
        let root = scratch("fallback");
        let twin = "the railway deploy is a lesson";
        write_fact(
            &root,
            "tagged",
            &good_fact("tagged", twin, "project", "deploy"),
        );
        write_fact(
            &root,
            "untagged",
            &good_fact("untagged", twin, "project", ""),
        );
        let memory = Memory::load_with(&root, Box::new(KeywordOnly));
        // Unclassified: no partition — the word-ranked pool IS the
        // primary pool — so the tagged and untagged twins rank equally
        // and the name breaks the tie. Nothing is off-shape.
        let hits = memory.recall("a lesson");
        let names: Vec<&str> = hits.iter().map(|hit| hit.fact.name.as_str()).collect();
        assert_eq!(names, vec!["tagged", "untagged"]);
        assert!(hits.iter().all(|hit| !hit.off_shape), "{hits:?}");
        let tagged = &hits[0];
        let untagged = &hits[1];
        assert_eq!(tagged.score.total, untagged.score.total);
        // Classified: the partition puts the same-shape twin in the
        // primary pool and the other in the spillover, whatever the
        // words say — the shape term is structural, so there is no
        // boost to assert and none to rely on.
        let hits = memory.recall("the railway deploy is failing");
        let names: Vec<&str> = hits.iter().map(|hit| hit.fact.name.as_str()).collect();
        assert_eq!(names, vec!["tagged", "untagged"]);
        assert!(!hits[0].off_shape, "the same-shape twin is primary");
        assert!(hits[1].off_shape, "the untagged twin is spillover");
        // The twins' word evidence is identical, so only the partition
        // separates them.
        assert_eq!(hits[0].score.total, hits[1].score.total);
        // The degradation never starves recall: both twins rank.
        let partition = memory.partition("the railway deploy is failing");
        assert_eq!(partition.shape, Some("deploy"));
        assert_eq!(hit_names(&partition.primary), vec!["tagged"]);
        assert_eq!(hit_names(&partition.spillover), vec!["untagged"]);
        let _ = fs::remove_dir_all(&root);
    }

    // --- shape-first recall: the partition, the vectors, the signs ----

    #[test]
    fn the_shape_partitions_the_pool_and_the_words_cannot_cross_it() {
        let root = scratch("partition");
        // The proof corpus: a same-shape fact with WEAK word evidence
        // beside a wrong-shape fact with STRONG word evidence and a
        // long recall history. Keyword-only, so every number below is
        // exact and hand-checkable.
        write_fact(
            &root,
            "same-shape",
            &good_fact(
                "same-shape",
                "the branch pipeline is green",
                "project",
                "deploy",
            ),
        );
        write_fact(
            &root,
            "wrong-shape",
            &recalled_fact(
                "wrong-shape",
                "the railway deploy is failing",
                "rendering",
                RECALL_CEILING,
            ),
        );
        let memory = Memory::load_with(&root, Box::new(KeywordOnly));
        let hits = memory.recall("the railway deploy is failing");
        // The same-shape fact ranks first whatever the words say: the
        // shape selects the pool, and no term can cross it.
        assert_eq!(hit_names(&hits), vec!["same-shape", "wrong-shape"]);
        assert!(!hits[0].off_shape, "the same-shape fact is primary");
        assert!(hits[1].off_shape, "the wrong-shape fact is spillover");
        // The word evidence, which used to decide, is on the wrong-shape
        // side — that is what makes this a proof.
        let same = &hits[0].score;
        let wrong = &hits[1].score;
        assert!((same.keyword - 0.6).abs() < 1e-12, "{same:?}");
        assert!((same.total - 0.24).abs() < 1e-12, "{same:?}");
        assert!((wrong.keyword - 1.0).abs() < 1e-12, "{wrong:?}");
        assert!((wrong.history - 1.0).abs() < 1e-12, "{wrong:?}");
        assert!((wrong.total - 0.55).abs() < 1e-12, "{wrong:?}");
        assert!(
            wrong.total > same.total,
            "the words still prefer the wrong shape"
        );
        // THIS TEST FAILS UNDER THE OLD WEIGHTING. The old score was
        // vector 0.55 + keyword 0.25 + importance 0.10 + decay 0.10 plus
        // SHAPE_BOOST 0.15, and by hand:
        //   same-shape:  0.55*0.0 + 0.25*0.6 + 0.10*0.0 + 0.10*1.0 + 0.15 = 0.40
        //   wrong-shape: 0.55*0.0 + 0.25*1.0 + 0.10*1.0 + 0.10*1.0 + 0.00 = 0.45
        // — the wrong-shape fact ranked FIRST, which is exactly the
        // failure the owner named ("a keyword or phrase in the wrong
        // context is meaningless"). The partition is what fixes it, and
        // this ordering is the proof.
    }

    #[test]
    fn within_one_shape_the_vector_that_drove_outranks_the_one_that_merely_talks() {
        let root = scratch("multidimensional");
        // Inside ONE shape: fact A has strong topical overlap and no
        // trajectory evidence — it talks about the neighborhood. Fact B
        // has weak topical overlap but strong trajectory evidence (its
        // recorded phrases echo the instruction's trajectory) and a
        // recall history — it drove toward this minima. A word-dominant
        // ranking picks A; the multidimensional one picks B.
        write_fact(
            &root,
            "topical-only",
            &good_fact(
                "topical-only",
                "the railway deploy is failing per branch on the canvas",
                "project",
                "deploy",
            ),
        );
        write_fact(
            &root,
            "drove-here",
            &trajectory_fact(
                "drove-here",
                "a lesson about branches",
                "deploy",
                &["check the railway deploy logs before failing again"],
                4,
            ),
        );
        let memory = Memory::load_with(&root, Box::new(KeywordOnly));
        let hits = memory.recall("the railway deploy is failing");
        assert_eq!(hit_names(&hits), vec!["drove-here", "topical-only"]);
        let drove = &hits[0].score;
        let talked = &hits[1].score;
        // B's evidence is multidimensional: a 0.8 trajectory match and
        // four recalls of history, over weak topical overlap.
        assert!((drove.keyword - 0.2).abs() < 1e-12, "{drove:?}");
        assert!((drove.trajectory - 0.8).abs() < 1e-12, "{drove:?}");
        assert!((drove.history - history_term(4)).abs() < 1e-12, "{drove:?}");
        assert!((drove.total - 0.5028).abs() < 1e-3, "{drove:?}");
        // A's evidence is topical and nothing else: full coverage, no
        // direction, no history.
        assert!((talked.keyword - 1.0).abs() < 1e-12, "{talked:?}");
        assert!((talked.trajectory - 0.0).abs() < 1e-12, "{talked:?}");
        assert!((talked.total - 0.30).abs() < 1e-12, "{talked:?}");
        // The rebalanced weights keep the ordering and hold the gap:
        // under the first reweighting (0.20 each, 0.40 together) this
        // same corpus put A at 0.35 and B at 0.49 — a 0.14 margin that
        // shrank to nothing the moment the vector channel served a
        // maximum-proximity fact. The direction channels carry 0.60
        // against the proximity pair's 0.30, and the margin is 0.203.
        assert!(drove.total - talked.total > 0.15, "{drove:?} vs {talked:?}");
        // THIS TEST FAILS UNDER THE OLD WEIGHTING, where the topical
        // pair was 0.80 and there was no trajectory channel at all:
        //   topical-only: 0.25*1.0 + 0.10*0.0 + 0.10*1.0 + 0.15 = 0.50
        //   drove-here:   0.25*0.2 + 0.10*0.5 + 0.10*1.0 + 0.15 = 0.35
        // — A ranked first. The old weighting had no way to see that B
        // drove toward this problem and A merely talks about it.
    }

    #[test]
    fn direction_outranks_proximity_with_the_vector_channel_live_for_both() {
        let root = scratch("direction-over-proximity");
        // THE OWNER'S RULING, pinned where it can actually fail. The
        // slice-19 proof ran KeywordOnly, so the vector channel was
        // dead for BOTH facts and the case never ran: a fact with
        // maximum proximity and zero direction could not be shown to
        // lose. Here the scripted embedder serves both, the channel is
        // live, and the proximity-max fact still loses.
        //
        // The instruction is deliberately unclassifiable — no shape's
        // terms appear — so the word-ranked pool IS the primary pool
        // and nothing but the score separates these two facts.
        write_fact(
            &root,
            "proximity-max",
            &good_fact(
                "proximity-max",
                "the walnut table is wobbly",
                "project",
                "deps",
            ),
        );
        // The direction-strong fact: worded differently (its
        // description shares NO token with the instruction, so its
        // keyword channel reads 0.00), carrying the resolution's own
        // phrases, four recalls behind it, confirmed-by, dated today.
        write_fact(
            &root,
            "drove-here",
            &trajectory_fact(
                "drove-here",
                "a lesson about furniture",
                "deps",
                &["shim the walnut table legs until it is level"],
                4,
            ),
        );
        let (calls, embedder) = ScriptedEmbedder::new();
        let memory = Memory::load_with(&root, embedder);
        // The vector channel is live: both facts were embedded at load.
        assert_eq!(calls.load(Ordering::Relaxed), 2);
        let hits = memory.recall("the walnut table is wobbly");
        // And the instruction was embedded beside them, so the channel
        // served the query too — not merely the facts.
        assert_eq!(calls.load(Ordering::Relaxed), 3);
        assert_eq!(hit_names(&hits), vec!["drove-here", "proximity-max"]);
        let drove = &hits[0].score;
        let near = &hits[1].score;
        // The winner's whole case is direction: it matched ZERO of the
        // instruction's tokens, and its phrases carried the direction
        // into the embedding as well as into the trajectory channel.
        assert!((drove.keyword - 0.0).abs() < 1e-12, "{drove:?}");
        assert!((drove.trajectory - 0.8).abs() < 1e-12, "{drove:?}");
        assert!((drove.history - history_term(4)).abs() < 1e-12, "{drove:?}");
        assert!((drove.vector - 1.0).abs() < 1e-12, "{drove:?}");
        assert!((drove.total - 0.6228).abs() < 1e-3, "{drove:?}");
        // The loser's whole case is proximity: every word matched, and
        // it has no direction at all.
        assert!((near.keyword - 1.0).abs() < 1e-12, "{near:?}");
        assert!((near.trajectory - 0.0).abs() < 1e-12, "{near:?}");
        assert!((near.vector - 1.0).abs() < 1e-12, "{near:?}");
        assert!((near.total - 0.45).abs() < 1e-12, "{near:?}");
        // Maximum proximity loses to direction by 0.173, and the
        // arithmetic is the weights' own, hand-checked:
        //   proximity-max: 0.15*1.0 + 0.15*1.0 + 0.10 + 0.05 = 0.45
        //   drove-here:    0.15*1.0 + 0.00    + 0.30*0.8
        //                  + 0.25*ln(5)/ln(129) + 0.10 + 0.05 = 0.6228
        //
        // The margin was 0.215 under the linear curve, where four
        // recalls already scored history 0.5. The logarithmic curve
        // reads the same four recalls as 0.331, so the margin narrows —
        // and it narrows in the direction the ruling wants: history is
        // magnitude, not direction, and a fact that never drove should
        // not be able to buy its way past one that did.
        assert!(drove.total > near.total + 0.15, "{drove:?} vs {near:?}");
        // The phrases are the resolution's own vocabulary — "shim the
        // walnut table legs until it is level" — not the question's
        // words restated: the question says "wobbly", the resolution
        // says how it was levelled. That is the "worded differently"
        // case, and the trajectory channel still reads 0.80.
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn the_two_signs_play_opposite_roles_and_the_assembly_names_which_is_which() {
        let root = scratch("signs");
        // Inside one shape: a resolution whose phrases match the
        // instruction's trajectory, a failure whose phrases match the
        // SAME trajectory, and a failure whose phrases do not.
        write_fact(
            &root,
            "what-worked",
            &trajectory_fact(
                "what-worked",
                "the bundle build is slow",
                "build",
                &[
                    "pin the webpack cache before the bundle build",
                    "compile with the pinned lockfile",
                ],
                4,
            ),
        );
        write_fact(
            &root,
            "what-diverged",
            &trajectory_fact(
                "what-diverged",
                "the bundle output grew past its bound",
                "build, bound-exceeded",
                &["the webpack bundle was left unbounded and failed to compile"],
                0,
            ),
        );
        write_fact(
            &root,
            "unrelated-divergence",
            &trajectory_fact(
                "unrelated-divergence",
                "a lane judged a workload",
                "build, fail-open",
                &["a lane fell back once"],
                0,
            ),
        );
        let memory = Memory::load_with(&root, Box::new(KeywordOnly));
        let hits = memory.recall("the webpack bundle fails to compile");
        // The divergence now leads, and this is the R1 curve's own
        // consequence rather than a regression. Both facts carry the
        // same shape and the same words; the divergence's phrases match
        // the instruction's trajectory perfectly (1.0 against 0.667),
        // and under the linear curve the resolution's four recalls
        // scored history 0.5 — enough to outrank a better direction
        // match. The logarithmic curve reads those same four recalls as
        // 0.331, so direction decides, which is the ruling.
        //
        // The order is safe because the sign is on the line and the
        // legend names it: the model reads `diverged:` as an
        // anti-vector to avoid, not a candidate to adopt. That is
        // exactly what the three signs exist for, and it is why an
        // anti-vector may rank as prominently as a resolution.
        assert_eq!(hit_names(&hits), vec!["what-diverged", "what-worked"]);
        assert_eq!(hits[0].fact.sign(), Sign::Divergence);
        assert_eq!(hits[1].fact.sign(), Sign::Resolution);
        assert!(
            (hits[0].score.total - 0.5).abs() < 1e-3,
            "{:?}",
            hits[0].score
        );
        assert!(
            (hits[1].score.total - 0.4828).abs() < 1e-3,
            "{:?}",
            hits[1].score
        );
        // The failure whose phrases do NOT match does not surface merely
        // for sharing the shape: its whole score is freshness plus
        // provenance (0.15), under the floor — it is in neither pool.
        let partition = memory.partition("the webpack bundle fails to compile");
        assert!(
            partition
                .primary
                .iter()
                .all(|hit| hit.fact.name != "unrelated-divergence"),
            "{:?}",
            hit_names(&partition.primary)
        );
        assert!(
            partition
                .spillover
                .iter()
                .all(|hit| hit.fact.name != "unrelated-divergence"),
            "{:?}",
            hit_names(&partition.spillover)
        );
        // The sign is derived from the tags, not stored: a failure-shape
        // tag is what makes a fact an anti-vector. It is hits[0] now —
        // the divergence leads, and it is the fact carrying both tags.
        assert_eq!(
            hits[0].fact.shape,
            vec!["build".to_string(), "bound-exceeded".to_string()]
        );
        // The assembly names which is which, and the model can never
        // read the anti-vector as a candidate.
        let assembly = memory.assemble("the webpack bundle fails to compile", &[]);
        let block = render_assembly(&assembly);
        assert!(!block.contains("failure-shape recall"), "{block}");
        assert!(block.contains("shape-routed recall for `build`"), "{block}");
        assert!(block.contains(SIGN_LEGEND.trim_end()), "{block}");
        assert!(block.contains("- resolved: [build"), "{block}");
        assert!(block.contains("- diverged: [bound-exceeded"), "{block}");
        assert!(!block.contains("a lane fell back once"), "{block}");
        // The divergence surfaces as prominently as the resolution: it
        // is in the same pool, ranked by the same evidence, and only the
        // marker distinguishes it.
        assert_eq!(assembly.primary.len(), 2);
        // THIS TEST FAILS UNDER THE OLD WEIGHTING, where neither the
        // sign nor the trajectory channel existed: the old score ranked
        // all three facts by topical overlap alone (all three carry
        // `build` and take the same 0.15 boost), the unrelated
        // divergence surfaced on freshness plus the boost (0.25 ≥ the
        // 0.20 floor), and nothing marked an anti-vector from a
        // resolution — the model would have read all three as
        // candidates.
    }

    #[test]
    fn the_third_sign_ranks_exactly_as_a_resolution_and_names_the_missing_companions() {
        let root = scratch("third-sign");
        // The third vector's whole design in one corpus: two facts with
        // IDENTICAL evidence — same description, same phrases, same
        // recall history, same basis, same day — differing only in the
        // outcome tag and the missing companions. An incomplete vector
        // is not a penalty: it ranks exactly as the resolution beside
        // it, and what distinguishes it is the marker and the named
        // companions.
        let description = "the bundle build is slow";
        let phrases = ["pin the webpack cache before the bundle build"];
        write_fact(
            &root,
            "drove-here",
            &trajectory_fact("drove-here", description, "build", &phrases, 4),
        );
        write_fact(
            &root,
            "stalled-here",
            &incomplete_fact(
                "stalled-here",
                description,
                "build, incomplete",
                &phrases,
                &["the torch stack", "the CUDA driver"],
                4,
            ),
        );
        let memory = Memory::load_with(&root, Box::new(KeywordOnly));
        let hits = memory.recall("the webpack bundle fails to compile");
        // Same pool, same rank evidence, name as the tie-break.
        assert_eq!(hit_names(&hits), vec!["drove-here", "stalled-here"]);
        assert!(hits.iter().all(|hit| !hit.off_shape), "{hits:?}");
        assert_eq!(hits[0].fact.sign(), Sign::Resolution);
        assert_eq!(hits[1].fact.sign(), Sign::Incomplete);
        // NO PENALTY, in exact arithmetic: the two totals are equal, and
        // every channel is equal. The outcome tag rides the tags (and
        // so the topic text), which is why neither `build` nor
        // `incomplete` — neither of which the instruction names — moves
        // the keyword channel.
        assert_eq!(hits[0].score.total, hits[1].score.total);
        assert_eq!(hits[0].score.keyword, hits[1].score.keyword);
        assert_eq!(hits[0].score.trajectory, hits[1].score.trajectory);
        assert_eq!(hits[0].score.history, hits[1].score.history);
        assert!(
            (hits[0].score.total - 0.4328).abs() < 1e-3,
            "{:?}",
            hits[0].score
        );
        // The missing companions are NOT the fact's direction: they ride
        // no indexed text, so they cannot score.
        assert!(
            !hits[1].fact.indexed_text().contains("torch stack"),
            "{:?}",
            hits[1].fact.indexed_text()
        );
        // The marker and the companions ride the assembly line, and the
        // legend names all three signs.
        let block = render_assembly(&memory.assemble("the webpack bundle fails to compile", &[]));
        assert!(block.contains("- incomplete: [build"), "{block}");
        assert!(
            block.contains("missing: the torch stack, the CUDA driver"),
            "{block}"
        );
        assert!(block.contains("- resolved: [build"), "{block}");
        assert!(block.contains(SIGN_LEGEND.trim_end()), "{block}");
        assert!(
            block.contains("incomplete: drove partway and stalled"),
            "{block}"
        );
        // The recall surface cards the companions where the fact ranks.
        let rendered = render_recall(&memory, "the webpack bundle fails to compile");
        assert!(rendered.contains("incomplete:"), "{rendered}");
        assert!(rendered.contains("missing: the torch stack"), "{rendered}");
        // The corpus surface cards them too.
        let cards = render(&memory, None);
        assert!(cards.contains("missing: the torch stack"), "{cards}");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_failure_shape_resolution_is_a_candidate_not_an_anti_vector() {
        let root = scratch("failure-pool-signs");
        // THE FAILURE-POOL'S SIGN PROBLEM. A fact about how a
        // drops-the-row bug was FIXED is tagged with the failure shape
        // because that is the problem's shape — and under the two-sign
        // rule it rendered `diverged:`, warning the model off a correct
        // trajectory while the failure-shape header promised "what broke
        // before, and what resolved it" with the resolved half unable
        // to occur. The outcome tags fix it: the shape says what kind of
        // thing broke, the tag says what became of the trajectory.
        write_fact(
            &root,
            "what-broke",
            &trajectory_fact(
                "what-broke",
                "the calibration reader cannot read a null",
                "drops-the-row",
                &["parse_row drops the whole line"],
                0,
            ),
        );
        write_fact(
            &root,
            "what-fixed-it",
            &trajectory_fact(
                "what-fixed-it",
                "the reader now takes a null into a numeric field",
                "drops-the-row, resolved",
                &["serde default for Option", "parse_row keeps the row"],
                0,
            ),
        );
        write_fact(
            &root,
            "what-stalled",
            &incomplete_fact(
                "what-stalled",
                "the reader keeps the row but loses its timestamp",
                "drops-the-row, incomplete",
                &["serde default for Option"],
                &["the timezone-aware parser"],
                0,
            ),
        );
        let memory = Memory::load_with(&root, Box::new(KeywordOnly));
        let partition = memory.partition("the row vanished from the stream");
        // One failure shape, one pool, three signs — the avoidance half
        // and the prevention half serve the same shape.
        assert_eq!(partition.shape, Some("drops-the-row"));
        assert_eq!(
            hit_names(&partition.primary),
            vec!["what-broke", "what-fixed-it", "what-stalled"]
        );
        let signs: Vec<Sign> = partition
            .primary
            .iter()
            .map(|hit| hit.fact.sign())
            .collect();
        assert_eq!(
            signs,
            vec![Sign::Divergence, Sign::Resolution, Sign::Incomplete]
        );
        // The assembly names each one, and the failure-shape header's
        // promise is now true: what broke, what resolved it, and what
        // stalled.
        let block = render_assembly(&memory.assemble("the row vanished from the stream", &[]));
        assert!(block.contains("- diverged: [drops-the-row"), "{block}");
        assert!(block.contains("- resolved: [drops-the-row"), "{block}");
        assert!(block.contains("- incomplete: [drops-the-row"), "{block}");
        assert!(
            block.contains("missing: the timezone-aware parser"),
            "{block}"
        );
        assert!(
            block.contains("what broke before, what resolved it, and what stalled"),
            "{block}"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn the_incomplete_vector_round_trips_and_the_block_does_not_read_as_done() {
        let root = scratch("incomplete-round-trip");
        let memory = Memory::load_with(&root, Box::new(KeywordOnly));
        // The owner's own case, most mistaken for done: "the bootstrap
        // was correct but the torch stack was missing". Written with its
        // missing companions, recalled by the shape, and read back.
        let request = Remember::parse(&serde_json::json!({
            "name": "bootstrap-without-the-torch-stack",
            "description": "the bootstrap was correct but the torch stack was missing",
            "shape": "build, incomplete",
            "basis": "inferred",
            "phrases": ["the bootstrap ran and the model loaded"],
            "missing": ["the torch stack", "the CUDA driver"],
        }))
        .unwrap();
        let written = memory.remember(&request).unwrap();
        assert_eq!(written.sign(), Sign::Incomplete);
        assert_eq!(
            written.missing,
            vec!["the torch stack".to_string(), "the CUDA driver".to_string()]
        );
        // The document renders the companions beside the phrases, and
        // the loader reads them back exactly.
        let text = fs::read_to_string(&written.path).unwrap();
        assert!(
            text.contains("missing: [the torch stack, the CUDA driver]\n"),
            "{text}"
        );
        let loaded = load_facts(&root);
        assert_eq!(loaded[0].missing, request.missing);

        // A later recall for the shape names the companions and does NOT
        // read the fact as done.
        let reloaded = Memory::load_with(&root, Box::new(KeywordOnly));
        let hits = reloaded.recall("the webpack bundle build is incomplete");
        assert_eq!(hit_names(&hits), vec!["bootstrap-without-the-torch-stack"]);
        assert_eq!(hits[0].fact.sign(), Sign::Incomplete);
        let block =
            render_assembly(&reloaded.assemble("the webpack bundle build is incomplete", &[]));
        assert!(block.contains("- incomplete: [build"), "{block}");
        assert!(block.contains("the torch stack"), "{block}");
        assert!(block.contains("the CUDA driver"), "{block}");
        // No line reads this fact as finished, and no line reads it as
        // an anti-vector either. (The legend names all three signs as
        // vocabulary, so the assertion is on the fact's own line, not
        // on the block as a whole.)
        assert!(!block.contains("- resolved: [build"), "{block}");
        assert!(!block.contains("- diverged: [build"), "{block}");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_failure_shape_recalls_through_the_same_partition() {
        let root = scratch("failure-axis");
        // The avoidance half: two facts tagged with a failure shape,
        // recalled when the instruction matches that failure shape,
        // through the same partition as a resolution.
        write_fact(
            &root,
            "what-broke",
            &trajectory_fact(
                "what-broke",
                "the calibration reader cannot read a null",
                "drops-the-row",
                &[
                    "parse_row drops the whole line",
                    "the row vanished from read_day",
                ],
                0,
            ),
        );
        write_fact(
            &root,
            "what-it-cost",
            &trajectory_fact(
                "what-it-cost",
                "the reader takes a null into a numeric field",
                "drops-the-row",
                &["serde default for Option", "parse_row keeps the row"],
                0,
            ),
        );
        // A fact carrying BOTH axes: it answers either one.
        write_fact(
            &root,
            "both-axes",
            &good_fact(
                "both-axes",
                "the shader layout repaints and the tower scan read was unbounded",
                "project",
                "rendering, bound-exceeded",
            ),
        );
        let memory = Memory::load_with(&root, Box::new(KeywordOnly));

        let partition = memory.partition("the row vanished from the stream");
        assert_eq!(partition.shape, Some("drops-the-row"));
        assert_eq!(
            hit_names(&partition.primary),
            vec!["what-broke", "what-it-cost"]
        );
        assert!(partition.spillover.is_empty());
        // Every fact in a failure-shape pool is an anti-vector: the
        // avoidance half is not a candidate, and its own phrases may
        // still carry what resolved it.
        assert!(partition
            .primary
            .iter()
            .all(|hit| hit.fact.sign() == Sign::Divergence));

        // Either axis matches: the both-axes fact is primary under the
        // structure shape AND under the failure shape.
        let rendering = memory.partition("the shader layout repaints every frame");
        assert_eq!(rendering.shape, Some("rendering"));
        assert!(rendering
            .primary
            .iter()
            .any(|hit| hit.fact.name == "both-axes"));
        let failure = memory.partition("the tower scan read was unbounded");
        assert_eq!(failure.shape, Some("bound-exceeded"));
        assert!(failure
            .primary
            .iter()
            .any(|hit| hit.fact.name == "both-axes"));
        assert_eq!(shape_axis("bound-exceeded"), "failure");
        assert_eq!(shape_axis("rendering"), "structure");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn the_spillover_enters_only_through_its_three_named_routes() {
        // Route (a): the primary pool is empty, so the off-shape facts
        // are the answer — and the strongest of them enters even at a
        // budget that fits nothing else.
        let root_a = scratch("spillover-empty");
        write_fact(
            &root_a,
            "off-one",
            &good_fact(
                "off-one",
                "the railway deploy is failing per branch",
                "project",
                "rendering",
            ),
        );
        let memory = Memory::load_with(&root_a, Box::new(KeywordOnly));
        let assembly = memory.assemble_within("the railway deploy is failing", &[], 1);
        assert_eq!(hit_names(&assembly.recalled), vec!["off-one"]);
        assert!(assembly.recalled[0].off_shape);
        assert_eq!(assembly.spillover.len(), 1);
        assert!(
            assembly
                .spillover_route
                .as_deref()
                .unwrap_or_default()
                .contains("no fact carries `deploy`"),
            "{:?}",
            assembly.spillover_route
        );
        assert!(render_assembly(&assembly).contains("off-shape"));

        // Route (b): the pool's best is below SPILLOVER_FLOOR, so the
        // strongest off-shape fact enters beside the weak pool — again
        // at a budget that fits nothing else.
        let root_b = scratch("spillover-weak");
        write_fact(
            &root_b,
            "same-shape",
            &good_fact(
                "same-shape",
                "the branch pipeline is green",
                "project",
                "deploy",
            ),
        );
        write_fact(
            &root_b,
            "wrong-shape",
            &recalled_fact(
                "wrong-shape",
                "the railway deploy is failing",
                "rendering",
                RECALL_CEILING,
            ),
        );
        let memory = Memory::load_with(&root_b, Box::new(KeywordOnly));
        let assembly = memory.assemble_within("the railway deploy is failing", &[], 1);
        assert_eq!(
            hit_names(&assembly.recalled),
            vec!["same-shape", "wrong-shape"]
        );
        assert!(!assembly.recalled[0].off_shape);
        assert!(
            assembly.recalled[1].off_shape,
            "the weak pool's neighbour is marked"
        );
        assert!(
            assembly
                .spillover_route
                .as_deref()
                .unwrap_or_default()
                .contains("below the SPILLOVER_FLOOR"),
            "{:?}",
            assembly.spillover_route
        );

        // Route (c) and the floor's other side: a pool whose best clears
        // the floor stands alone, so the off-shape fact enters only on
        // the budget's room — and is dropped, named, when there is none.
        let root_c = scratch("spillover-strong");
        write_fact(
            &root_c,
            "strong",
            &trajectory_fact(
                "strong",
                "the railway deploy is failing per branch",
                "deploy",
                &["read the railway deploy logs before failing again"],
                RECALL_CEILING,
            ),
        );
        write_fact(
            &root_c,
            "off-shape-fact",
            &good_fact(
                "off-shape-fact",
                "the railway deploy is failing",
                "project",
                "rendering",
            ),
        );
        let memory = Memory::load_with(&root_c, Box::new(KeywordOnly));
        let partition = memory.partition("the railway deploy is failing");
        assert!(
            partition.best().expect("the pool has a best") >= SPILLOVER_FLOOR,
            "{:?}",
            partition.best()
        );
        let room = memory.assemble("the railway deploy is failing", &[]);
        assert_eq!(hit_names(&room.recalled), vec!["strong", "off-shape-fact"]);
        assert!(room.recalled[1].off_shape);
        assert!(
            room.spillover_route
                .as_deref()
                .unwrap_or_default()
                .contains("the budget had room"),
            "{:?}",
            room.spillover_route
        );
        let tight = memory.assemble_within("the railway deploy is failing", &[], 1);
        assert_eq!(hit_names(&tight.recalled), vec!["strong"]);
        assert!(tight.spillover.is_empty(), "no route admitted them");
        assert!(
            tight.dropped.contains(&"off-shape-fact".to_string()),
            "{:?}",
            tight.dropped
        );
        assert!(!render_assembly(&tight).contains("off-shape"));
        let _ = fs::remove_dir_all(&root_a);
        let _ = fs::remove_dir_all(&root_b);
        let _ = fs::remove_dir_all(&root_c);
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
        assert!(
            after.contains("The full body, with a second line."),
            "{after}"
        );
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
        assert!(fs::read_to_string(&path)
            .unwrap()
            .contains("recall-count: 4"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_recall_hit_with_no_count_line_inserts_one_and_preserves_the_body() {
        let root = scratch("insert");
        let path = write_fact(
            &root,
            "no-count",
            "---\nname: no-count\ndescription: \"a deploy lesson about the railway deploy failing\"\ntype: project\ndate: 2026-10-13\nbasis: inferred\nshape: [deploy]\n---\n\nBody one.\n\nBody two.\n",
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
            &good_fact(
                "hit-me",
                "a deploy lesson about railway",
                "project",
                "deploy",
            ),
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
            .next_back()
            .expect("the second recall_hit row");
        assert_eq!(unclassified.shape, "");
        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&stream_root);
    }

    // --- the embedder ---

    #[test]
    fn the_env_override_changes_the_resolved_endpoint() {
        // The house resolution: env → the default constant, an empty or
        // whitespace value counting as unset. The lookup seam keeps the
        // test off the process-global environment, so it cannot race a
        // sibling test.
        assert_eq!(
            endpoint_from(|_| Some("http://127.0.0.1:9002/v1/embeddings".to_string())),
            "http://127.0.0.1:9002/v1/embeddings"
        );
        assert_eq!(endpoint_from(|_| None), DEFAULT_EMBEDDING_ENDPOINT);
        assert_eq!(
            endpoint_from(|_| Some(String::new())),
            DEFAULT_EMBEDDING_ENDPOINT
        );
        assert_eq!(
            endpoint_from(|_| Some("   ".to_string())),
            DEFAULT_EMBEDDING_ENDPOINT
        );
        // The name the environment carries is the one documented.
        assert_eq!(EMBEDDING_ENDPOINT_VAR, "FLEET_EMBEDDING_ENDPOINT");
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

        // Second load: every digest matches, nothing is embedded.
        let (calls, embedder) = ScriptedEmbedder::new();
        let _ = Memory::load_with(&root, embedder);
        assert_eq!(calls.load(Ordering::Relaxed), 0);

        // One changed fact: only it is re-embedded. The change is to the
        // indexed text — the description — which is what the digest
        // covers.
        let path = root.join("alpha.md");
        let text = fs::read_to_string(&path).unwrap();
        let edited = text.replace(
            "description: \"railway deploy per branch\"",
            "description: \"railway deploy per branch, now with a rollback\"",
        );
        assert_ne!(
            edited, text,
            "the fixture's description line is what this edits"
        );
        fs::write(&path, edited).unwrap();
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

    /// KI-228's own test, and the reason the entry is keyed by a digest
    /// rather than the file's stamp: a recall-count bump rewrites the
    /// file in place, which moves its mtime and so its
    /// `secs:nanos:len` stamp. A stamp-keyed entry re-embeds a fact
    /// whose content never changed — and the corpus's most-recalled
    /// facts are bumped on essentially every recall, so they were
    /// re-embedded on essentially every load.
    #[test]
    fn a_recall_count_bump_re_embeds_nothing() {
        let root = scratch("index-bump");
        order_corpus(&root);

        let (calls, embedder) = ScriptedEmbedder::new();
        let memory = Memory::load_with(&root, embedder);
        let first = calls.load(Ordering::Relaxed);
        assert!(first > 0, "the first load embeds the corpus");
        let before = fs::read_to_string(index_path(&root)).unwrap();
        drop(memory);

        // One recall, which bumps one fact's recall-count in place.
        let (calls, embedder) = ScriptedEmbedder::new();
        let bumped = Memory::load_with(&root, embedder);
        assert_eq!(
            calls.load(Ordering::Relaxed),
            0,
            "the load before the bump is fully cached"
        );
        let hits = bumped.recall("the railway deploy is failing");
        assert!(!hits.is_empty(), "the fixture answers this instruction");
        bumped.record_hits(&hits, None, None);
        drop(bumped);

        // The next load embeds nothing and does not even rewrite the
        // index: the bump moved the file's mtime but not its indexed
        // text, and the digest is over the indexed text.
        let (calls, embedder) = ScriptedEmbedder::new();
        let _ = Memory::load_with(&root, embedder);
        assert_eq!(
            calls.load(Ordering::Relaxed),
            0,
            "a recall-count bump must not invalidate a fact's cached vector"
        );
        assert_eq!(
            fs::read_to_string(index_path(&root)).unwrap(),
            before,
            "and it must not even rewrite the index"
        );
        let _ = fs::remove_dir_all(&root);
    }

    /// KI-231: a fact whose embed fails is absent from the rebuilt map,
    /// and the old write condition (`fresh.len() != index.facts.len()`)
    /// read that absence as a change and rewrote the pruned index — so a
    /// fact whose text changed while the embedding server was down was
    /// re-attempted, re-failed, and re-written on every load, forever.
    /// The rewrite now needs a new vector or a genuinely orphaned entry.
    #[test]
    fn a_failed_embed_rewrites_nothing() {
        let root = scratch("index-failed-embed");
        order_corpus(&root);

        let (calls, embedder) = ScriptedEmbedder::new();
        let _ = Memory::load_with(&root, embedder);
        assert_eq!(calls.load(Ordering::Relaxed), 4);
        let before = fs::read_to_string(index_path(&root)).unwrap();

        // One fact's indexed text changes, and the embedding server is
        // down for exactly that fact.
        let path = root.join("alpha.md");
        let text = fs::read_to_string(&path).unwrap();
        let edited = text.replace(
            "description: \"railway deploy per branch\"",
            "description: \"railway deploy per branch, blackholed\"",
        );
        assert_ne!(
            edited, text,
            "the fixture's description line is what this edits"
        );
        fs::write(&path, edited).unwrap();

        for pass in 0..2 {
            let calls = Arc::new(AtomicUsize::new(0));
            let failing = Box::new(PartialEmbedder {
                calls: Arc::clone(&calls),
                fails_on: "blackholed".to_string(),
            });
            let _ = Memory::load_with(&root, failing);
            assert_eq!(
                calls.load(Ordering::Relaxed),
                1,
                "pass {pass}: the changed fact is still attempted — the retry is not the bug"
            );
            assert_eq!(
                fs::read_to_string(index_path(&root)).unwrap(),
                before,
                "pass {pass}: a failed embed must not rewrite the index"
            );
        }

        // The server comes back: the changed fact embeds and the index
        // lands with it.
        let (calls, embedder) = ScriptedEmbedder::new();
        let _ = Memory::load_with(&root, embedder);
        assert_eq!(
            calls.load(Ordering::Relaxed),
            1,
            "only the changed fact re-embeds"
        );
        assert_ne!(
            fs::read_to_string(index_path(&root)).unwrap(),
            before,
            "and the new vector lands"
        );

        // A deleted fact's entry is still pruned — the one rewrite the
        // failed-embed rule must not have swallowed.
        fs::remove_file(root.join("zeta.md")).unwrap();
        let (calls, embedder) = ScriptedEmbedder::new();
        let _ = Memory::load_with(&root, embedder);
        assert_eq!(calls.load(Ordering::Relaxed), 0, "nothing to re-embed");
        let after = fs::read_to_string(index_path(&root)).unwrap();
        assert_ne!(after, before, "the deleted fact's entry is pruned");
        assert!(!after.contains("\"zeta\""), "and only that one");

        let _ = fs::remove_dir_all(&root);
    }

    // --- the recall graph ----------------------------------------------------

    /// A fact with `superseded-by` and `missing` set, so a fixture can
    /// declare the edges the frontmatter carries.
    fn edge_fact(name: &str, description: &str, superseded_by: &str, missing: &[&str]) -> String {
        format!(
            "---\nname: {name}\ndescription: \"{description}\"\ntype: project\ndate: {today}\nbasis: confirmed-by\nshape: [deploy]\nphrases: []\nmissing: [{}]\nrecall-count: 0\nsuperseded-by: {superseded_by}\nvalid-until:\n---\n\nThe body of {name}.\n",
            missing.join(", "),
            today = calib::today()
        )
    }

    /// R4a: a fact reachable only through a `superseded-by` edge is
    /// surfaced, and marked with the edge that reached it.
    ///
    /// The neighbour's own description shares no stem with the
    /// instruction and it carries no phrases, so on its own it scores
    /// standing alone (0.15) — under the floor, in no pool. The edge is
    /// the only thing that reaches it.
    #[test]
    fn a_superseded_fact_surfaces_the_fact_that_replaced_it() {
        let root = scratch("graph-supersede");
        // The old fact matches the instruction on its own words; the
        // fact that replaced it says something entirely different.
        write_fact(
            &root,
            "old-rule",
            &edge_fact("old-rule", "the railway deploy rule", "new-rule", &[]),
        );
        write_fact(
            &root,
            "new-rule",
            &edge_fact("new-rule", "walnut table texture", "", &[]),
        );
        let memory = Memory::load_with(&root, Box::new(KeywordOnly));
        let hits = memory.recall("the railway deploy rule");
        // The retired source is out of recall by design, so the only hit
        // is the fact that replaced it.
        assert_eq!(
            hit_names(&hits),
            vec!["new-rule"],
            "the fact that replaced the match must surface, and it alone"
        );
        assert_eq!(hits[0].via, Some("superseded-by"));
        assert!(
            !hits[0].off_shape,
            "a neighbour is not off-shape: it did not compete at all"
        );
        // And the assembled line names the edge, so it is never read as
        // a fact that ranked.
        let assembly = memory.assemble("the railway deploy rule", &[]);
        let block = render_assembly(&assembly);
        assert!(block.contains("via: superseded-by"), "{block}");
        assert!(block.contains("Adjacent memory"), "{block}");
        let _ = fs::remove_dir_all(&root);
    }

    /// R4a: a fact reachable only through a `missing` edge is surfaced —
    /// the companion an incomplete trajectory needed, which nothing
    /// fetched before because `missing` is deliberately excluded from
    /// the indexed text.
    #[test]
    fn a_missing_companion_is_fetched_not_just_named() {
        let root = scratch("graph-missing");
        write_fact(
            &root,
            "stalled",
            &edge_fact(
                "stalled",
                "the railway deploy stalled halfway",
                "",
                &["the-rollback-flag"],
            ),
        );
        write_fact(
            &root,
            "the-rollback-flag",
            &edge_fact("the-rollback-flag", "walnut table texture", "", &[]),
        );
        let memory = Memory::load_with(&root, Box::new(KeywordOnly));
        let hits = memory.recall("the railway deploy stalled halfway");
        let reached = hits
            .iter()
            .find(|hit| hit.fact.name == "the-rollback-flag")
            .expect("the named companion must be fetched");
        assert_eq!(reached.via, Some("missing"));
        // The stalled fact still leads: it matched, and a neighbour never
        // displaces a fact that competed.
        assert_eq!(hits[0].fact.name, "stalled");
        let _ = fs::remove_dir_all(&root);
    }

    /// R4a: the hop is bounded. A source that scored too little pulls in
    /// nothing, and a fact already in a pool is never pulled twice.
    #[test]
    fn a_weak_source_pulls_in_nothing_and_no_fact_enters_twice() {
        let root = scratch("graph-bound");
        // A retired source whose own vocabulary shares nothing with the
        // instruction: it scores standing alone, under the floor, so its
        // authority admits nobody.
        write_fact(
            &root,
            "weak-source",
            &edge_fact("weak-source", "walnut table texture", "target", &[]),
        );
        write_fact(
            &root,
            "target",
            &edge_fact("target", "shader compile failed", "", &[]),
        );
        let memory = Memory::load_with(&root, Box::new(KeywordOnly));
        let hits = memory.recall("the railway rollout is failing");
        assert!(
            hits.iter().all(|hit| hit.fact.name != "target"),
            "a source below the floor must pull in nothing: {:?}",
            hit_names(&hits)
        );
        assert!(
            hits.is_empty(),
            "and neither fact clears the floor on its own: {:?}",
            hit_names(&hits)
        );

        // A fact already ranked in a pool is not duplicated by an edge
        // pointing at it.
        let both = scratch("graph-dup");
        write_fact(
            &both,
            "source",
            &edge_fact(
                "source",
                "the railway rollout is failing",
                "already-ranked",
                &[],
            ),
        );
        write_fact(
            &both,
            "already-ranked",
            &edge_fact("already-ranked", "the railway rollout is failing", "", &[]),
        );
        let memory = Memory::load_with(&both, Box::new(KeywordOnly));
        let hits = memory.recall("the railway rollout is failing");
        assert_eq!(
            hits.iter()
                .filter(|hit| hit.fact.name == "already-ranked")
                .count(),
            1,
            "a fact already in a pool is not pulled in a second time"
        );
        assert!(
            hits.iter()
                .filter(|hit| hit.fact.name == "already-ranked")
                .all(|hit| hit.via.is_none()),
            "and it enters as the fact that ranked, not as a neighbour"
        );
        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&both);
    }

    /// R4a: a dangling edge name is dropped silently, because the corpus
    /// is hand-editable and a `missing` name that resolves to nothing is
    /// not an error the recall has to answer for.
    #[test]
    fn a_dangling_edge_name_is_dropped_silently() {
        let root = scratch("graph-dangling");
        // Live, so it is a hit in its own right; the dangling `missing`
        // names are simply not edges to anything.
        write_fact(
            &root,
            "points-at-nothing",
            &edge_fact(
                "points-at-nothing",
                "the railway rollout is failing",
                "",
                &["an-absent-companion"],
            ),
        );
        let memory = Memory::load_with(&root, Box::new(KeywordOnly));
        let hits = memory.recall("the railway rollout is failing");
        assert_eq!(
            hit_names(&hits),
            vec!["points-at-nothing"],
            "an edge to a fact the corpus does not hold is not an error"
        );
        assert!(
            hits.iter().all(|hit| hit.via.is_none()),
            "and nothing was pulled in"
        );
        let _ = fs::remove_dir_all(&root);
    }

    /// A fact with only the fields the graph reads set, for the edge
    /// extraction's own tests.
    fn graph_fact(name: &str, superseded_by: &str, missing: &[&str]) -> Fact {
        Fact {
            name: name.to_string(),
            description: format!("the {name} rule"),
            fact_type: "project".to_string(),
            date: calib::today(),
            basis: Basis::Inferred,
            tier: Tier::Durable,
            shape: vec!["deploy".to_string()],
            phrases: Vec::new(),
            missing: missing.iter().map(|m| m.to_string()).collect(),
            recall_count: 0,
            superseded_by: superseded_by.to_string(),
            valid_until: String::new(),
            body: String::new(),
            path: std::path::PathBuf::from(format!("{name}.md")),
            stamp: None,
        }
    }

    /// R4a: the edges the substrate declares, and the ones it deliberately
    /// does not. The mechanism's bounds (`GRAPH_HOPS`, `GRAPH_SOURCES`) are
    /// named constants rather than assertions here — the seam they zero is
    /// documented at `crate::recall_graph`, and a test asserting a constant
    /// asserts nothing.
    #[test]
    fn the_graph_reads_the_edges_the_substrate_declares() {
        // A retired fact vouches for its replacement without entering
        // recall itself, and a shared shape tag is deliberately not an
        // edge — the partition already reads it.
        let old = graph_fact("old", "new", &[]);
        assert!(
            old.forgotten(&calib::today()),
            "a stamped fact is retired, so it is never a hit"
        );
        let graph = crate::recall_graph::Graph::build(&[old, graph_fact("new", "", &[])]);
        assert_eq!(
            graph.neighbours("old"),
            &[crate::recall_graph::Neighbour {
                name: "new".into(),
                edge: crate::recall_graph::Edge::SupersededBy,
            }],
            "the retired fact points at its replacement"
        );
        assert!(
            graph.neighbours("new").is_empty(),
            "and the replacement points nowhere — the shape tag is not an edge"
        );
        // A dangling name is dropped rather than erroring.
        let dangling = crate::recall_graph::Graph::build(&[graph_fact("a", "gone", &["absent"])]);
        assert!(
            dangling.neighbours("a").is_empty(),
            "an edge to a fact the corpus does not hold is dropped"
        );
    }

    #[test]
    fn a_dimension_mismatch_invalidates_the_index_and_rebuilds() {
        let root = scratch("dims");
        order_corpus(&root);

        // The index is written at the scripted vocabulary's five
        // dimensions, and every cached vector is five long.
        let (calls, embedder) = ScriptedEmbedder::new();
        let memory = Memory::load_with(&root, embedder);
        assert_eq!(calls.load(Ordering::Relaxed), 4);
        assert!(memory.vectors.values().all(|vector| vector.len() == 5));
        drop(memory);

        // A model swap — the same embedder name serving three
        // dimensions — invalidates the whole cache: every fact is
        // re-embedded at the new length. Without the key this is the
        // silent failure: cosine's length guard scores the stale
        // vectors 0.0 and the vector channel dies while doctor names
        // it live.
        let (calls, embedder) = ScriptedEmbedder::at_dims(3);
        let memory = Memory::load_with(&root, embedder);
        assert_eq!(calls.load(Ordering::Relaxed), 4);
        assert!(memory.vectors.values().all(|vector| vector.len() == 3));
        drop(memory);

        // The rewritten index is keyed to the new model, so the next
        // load with the same one embeds nothing.
        let (calls, embedder) = ScriptedEmbedder::at_dims(3);
        let _ = Memory::load_with(&root, embedder);
        assert_eq!(calls.load(Ordering::Relaxed), 0);

        // And the file says so: the stale-dimension vectors are gone,
        // not merely shadowed in memory.
        let text = fs::read_to_string(index_path(&root)).unwrap();
        assert!(text.contains("\"dims\":3"), "{text}");
        assert!(text.contains("\"embedder\":\"scripted\""), "{text}");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_legacy_index_without_the_key_is_rebuilt_not_served() {
        let root = scratch("legacy-index");
        write_fact(
            &root,
            "alpha",
            &good_fact("alpha", "railway deploy per branch", "project", "deploy"),
        );
        // The pre-key format: a stamp-matching entry with no embedder
        // and no dims — an index written before the key existed. Its
        // stamp says fresh; its absent key says it is not this
        // embedder's cache, and the key wins.
        let stamp = load_facts(&root)[0]
            .stamp
            .clone()
            .expect("the fact carries a stamp");
        fs::write(
            index_path(&root),
            format!(
                "{{\"facts\":{{\"alpha\":{{\"stamp\":\"{stamp}\",\"vector\":[9.0,9.0,9.0,9.0,9.0]}}}}}}"
            ),
        )
        .unwrap();

        let (calls, embedder) = ScriptedEmbedder::new();
        let memory = Memory::load_with(&root, embedder);
        assert_eq!(calls.load(Ordering::Relaxed), 1);
        assert_eq!(memory.vectors["alpha"], vec![1.0, 1.0, 0.0, 0.0, 0.0]);
        let _ = fs::remove_dir_all(&root);
    }

    // --- assembly ---

    /// The pressure corpus: two protected facts, three ranked facts
    /// above the floor, and one below it.
    fn pressure_corpus(root: &Path) {
        write_fact(
            root,
            "identity",
            &good_fact(
                "identity",
                "who the owner is and how he works, in a standing fact",
                "user",
                "",
            ),
        );
        write_fact(
            root,
            "rule",
            &good_fact(
                "rule",
                "a standing rule the owner stated about documentation",
                "feedback",
                "docs",
            ),
        );
        write_fact(
            root,
            "top",
            &good_fact(
                "top",
                "the railway deploy is failing per branch",
                "project",
                "deploy",
            ),
        );
        write_fact(
            root,
            "middle",
            &good_fact(
                "middle",
                "the railway deploy waits for the webhook",
                "project",
                "deploy",
            ),
        );
        write_fact(
            root,
            "bottom",
            &good_fact(
                "bottom",
                "the railway deploy logs its outcome",
                "project",
                "deploy",
            ),
        );
        write_fact(
            root,
            "junk",
            &good_fact("junk", "the walnut table texture", "project", ""),
        );
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
        assert!(
            assembly.dropped.contains(&"bottom".to_string()),
            "{:?}",
            assembly.dropped
        );
        assert!(
            block.contains("the railway deploy is failing per branch"),
            "{block}"
        );
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

    /// The seed-corpus shape: seven protected facts carrying the seed
    /// tree's own descriptions, plus four ranked facts above the
    /// floor. The seven protected lines cost 449 tokens on their own
    /// (the real tree measures 453), so at the fork's 512 the tier
    /// crowded the ranking out: on the real tree the second ranked
    /// fact is ~85 tokens and did not fit, which is the parent's
    /// observed "exactly one fact". The shipped constant has to carry
    /// the tier and the ranking together.
    fn seed_shaped_corpus(root: &Path) {
        let protected: [(&str, &str, &str); 7] = [
            ("p-handle", "user", "The owner's GitHub handle is adelvillar1 and he runs ZCode as his daily coding agent on his own macOS machine; his LLM access is prepaid subscription plans routed through Fleet's own router, with DeepSeek held as a working account in reserve"),
            ("p-plans", "user", "The owner prefers spending the other plans' quota first and holds DeepSeek in reserve even though it has balance (owner clarification 2026-10-13); Fleet's own router selects the route (owner ruling 2026-10-16)"),
            ("p-english", "feedback", "Fleet's documentation is English-only \u{2014} no Chinese or other non-English docs anywhere in the repo \u{2014} a deliberate departure from the fork's bilingual convention (owner directive 2026-10-13)"),
            ("p-logging", "feedback", "Every error, warning, defect, gap, and improvement encountered in Fleet or any sibling repo lands in docs/known-issues.md with its evidence the moment it is seen; logging is not fixing and not scope, and 'not my code' is a reason to log, not to skip"),
            ("p-gate", "feedback", "Fleet's gate is local and permanent: cargo build --all-targets then cargo test, with the exit code read on the cargo invocation itself and never through a status-losing pipe; CI is disarmed by owner decision because the GitHub Actions budget is exhausted"),
            ("p-ux", "feedback", "Every Fleet slice ships its user-facing surface with the feature at the 'genuinely visual, never plain-text tables' bar \u{2014} cards, status dots, real layout \u{2014} as a per-slice deliverable, never a polish finale"),
            ("p-successor", "feedback", "Fleet is the self-contained Rust successor to the DeepSeek Harness fork's fleet profile: the fork stays the spec and the integration lab, the two never both ship the same feature, and every absorption is a native port rather than a Node dependency"),
        ];
        for (name, fact_type, description) in protected {
            write_fact(root, name, &good_fact(name, description, fact_type, ""));
        }
        write_fact(
            root,
            "top",
            &good_fact(
                "top",
                "the railway deploy is failing per branch",
                "project",
                "deploy",
            ),
        );
        write_fact(
            root,
            "bottom",
            &good_fact(
                "bottom",
                "the railway deploy logs its outcome",
                "project",
                "deploy",
            ),
        );
        write_fact(
            root,
            "lower",
            &good_fact(
                "lower",
                "the railway deploy ships quietly",
                "project",
                "deploy",
            ),
        );
        write_fact(
            root,
            "middle",
            &good_fact(
                "middle",
                "the railway deploy waits for the webhook",
                "project",
                "deploy",
            ),
        );
    }

    #[test]
    fn the_shipped_budget_carries_the_protected_tier_and_recall_together() {
        let root = scratch("seed-shaped");
        seed_shaped_corpus(&root);
        let memory = Memory::load_with(&root, Box::new(KeywordOnly));

        // The starvation the raise fixes, on the same corpus: at the
        // fork's 512 the seven protected lines left a ranked fact
        // behind. The budget arithmetic is exact integer math on fixed
        // strings, so this is deterministic, not flaky.
        let squeezed = memory.assemble_within("the railway deploy is failing", &[], 512);
        assert_eq!(squeezed.protected.len(), 7);
        assert_eq!(squeezed.recalled.len(), 3, "{:?}", squeezed.dropped);
        assert_eq!(squeezed.dropped, vec!["middle".to_string()]);

        // The shipped constant, not a test-local one: the tier and all
        // four ranked facts coexist, and nothing is dropped.
        let assembly = memory.assemble("the railway deploy is failing", &[]);
        assert_eq!(assembly.budget, ASSEMBLY_TOKEN_BUDGET);
        assert_eq!(assembly.protected.len(), 7);
        assert_eq!(assembly.recalled.len(), 4, "{:?}", assembly.dropped);
        assert!(assembly.dropped.is_empty(), "{:?}", assembly.dropped);
        assert!(assembly.used <= assembly.budget);
        // The block renders the tier first and the recall section
        // after it, with every line present.
        let block = render_assembly(&assembly);
        assert!(block.contains("Standing context (protected"), "{block}");
        for line in [
            "the railway deploy is failing per branch",
            "the railway deploy logs its outcome",
            "the railway deploy ships quietly",
            "waits for the webhook",
        ] {
            assert!(block.contains(line), "{block}");
        }
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn protected_facts_never_enter_the_ranked_section() {
        let root = scratch("protected-excluded");
        // The instruction's best match is protected: its keyword
        // coverage is the highest in the corpus, so if the ranking
        // admitted protected facts it would top the ranked section.
        write_fact(
            &root,
            "identity",
            &good_fact(
                "identity",
                "the railway deploy is the owner's standing rule about deploys",
                "user",
                "deploy",
            ),
        );
        write_fact(
            &root,
            "top",
            &good_fact(
                "top",
                "the railway deploy is failing per branch",
                "project",
                "deploy",
            ),
        );
        write_fact(
            &root,
            "bottom",
            &good_fact(
                "bottom",
                "the railway deploy logs its outcome",
                "project",
                "deploy",
            ),
        );
        let memory = Memory::load_with(&root, Box::new(KeywordOnly));

        // Recall excludes it: no fact enters a turn twice.
        let ranked: Vec<String> = memory
            .recall("the railway deploy is failing")
            .into_iter()
            .map(|hit| hit.fact.name)
            .collect();
        assert_eq!(ranked, vec!["top".to_string(), "bottom".to_string()]);

        // The assembly shows it in the protected block and the ranked
        // section carries the remainder — the documented behavior,
        // pinned.
        let assembly = memory.assemble("the railway deploy is failing", &[]);
        let protected: Vec<&str> = assembly
            .protected
            .iter()
            .map(|hit| hit.fact.name.as_str())
            .collect();
        assert_eq!(protected, vec!["identity"]);
        let recalled: Vec<&str> = assembly
            .recalled
            .iter()
            .map(|hit| hit.fact.name.as_str())
            .collect();
        assert_eq!(recalled, vec!["top", "bottom"]);
        let block = render_assembly(&assembly);
        let protected_at = block
            .find("the owner's standing rule")
            .expect("the protected fact renders in its tier");
        let ranked_at = block
            .find("the railway deploy is failing per branch")
            .expect("the ranked section renders");
        assert!(protected_at < ranked_at, "{block}");
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
            .map(|fact| Fact {
                name: "working-note".to_string(),
                ..fact
            })
            .into_iter()
            .collect();
        let assembly = memory.assemble_within("the railway deploy is failing", &working, 512);
        let block = render_assembly(&assembly);
        let protected_at = block
            .find("[user confirmed-by]")
            .expect("the protected tier renders");
        let recalled_at = block.find("[deploy").expect("the recall tier renders");
        let working_at = block
            .find("[working confirmed-by]")
            .expect("the working tier renders");
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
        assert!(
            rendered.contains("6 facts \u{b7} 2 protected \u{b7} 0 retired"),
            "{rendered}"
        );
        assert!(rendered.contains(&root.display().to_string()), "{rendered}");
        // Each card carries name, type, basis, shape, recall-count, and
        // valid-until.
        assert!(
            rendered.contains("project \u{b7} confirmed-by"),
            "{rendered}"
        );
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
        // nothing silently. The line is long enough to wrap, so the
        // assertion is on the axes' own vocabulary rather than on one
        // unbroken string: every shape in all three taxonomies is
        // named somewhere on the surface.
        let empty = render(&memory, Some("telemetry"));
        assert!(
            empty.contains("no facts carry shape `telemetry`"),
            "{empty}"
        );
        for shape in SHAPES
            .iter()
            .chain(FAILURE_SHAPES.iter())
            .chain(OUTCOME_SHAPES.iter())
        {
            assert!(empty.contains(*shape), "{shape} is not named: {empty}");
        }
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn the_recall_surface_prints_the_partition_the_ranking_and_the_dry_run() {
        let root = scratch("render-recall");
        pressure_corpus(&root);
        let memory = Memory::load_with(&root, Box::new(KeywordOnly));
        let before: Vec<String> = load_facts(&root)
            .iter()
            .map(|fact| fs::read_to_string(&fact.path).unwrap())
            .collect();
        let rendered = render_recall(&memory, "the railway deploy is failing");
        // The shape and its axis, the backend, and the counts are
        // named — including how many facts are in the pool and how many
        // are off-shape.
        assert!(
            rendered.contains("shape: deploy (structure-shape)"),
            "{rendered}"
        );
        assert!(rendered.contains("backend: keyword-only"), "{rendered}");
        assert!(rendered.contains("3 facts ranked"), "{rendered}");
        assert!(rendered.contains("3 in the pool"), "{rendered}");
        assert!(rendered.contains("0 off-shape"), "{rendered}");
        // Each ranked fact's card carries its channels — the "why",
        // each dimension named — and its magnitude: the shape, the hit
        // history, the freshness, the basis, and the sign.
        assert!(
            rendered.contains("vector 0.00 \u{b7} keyword 0.60 \u{b7} trajectory 0.00"),
            "{rendered}"
        );
        assert!(
            rendered.contains("shape: deploy \u{b7} history 0 \u{b7} freshness 1.00 \u{b7} provenance confirmed-by"),
            "{rendered}"
        );
        assert!(rendered.contains("0.30 resolved:"), "{rendered}");
        // The assembled block renders, and the junk is absent from it.
        assert!(
            rendered.contains("Standing context (protected"),
            "{rendered}"
        );
        assert!(
            rendered.contains("shape-routed recall for `deploy`"),
            "{rendered}"
        );
        assert!(
            rendered.contains("- resolved: [deploy 0.30 confirmed-by]"),
            "{rendered}"
        );
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
    fn the_recall_surface_marks_the_spillover_and_names_the_route() {
        let root = scratch("render-spillover");
        // The M1 corpus: a same-shape fact with weak word evidence and
        // a wrong-shape fact with strong word evidence and a long
        // recall history — the words alone would rank the wrong-shape
        // fact first.
        write_fact(
            &root,
            "same-shape",
            &good_fact(
                "same-shape",
                "the branch pipeline is green",
                "project",
                "deploy",
            ),
        );
        write_fact(
            &root,
            "wrong-shape",
            &recalled_fact(
                "wrong-shape",
                "the railway deploy is failing",
                "rendering",
                RECALL_CEILING,
            ),
        );
        let memory = Memory::load_with(&root, Box::new(KeywordOnly));
        let rendered = render_recall(&memory, "the railway deploy is failing");
        // The partition is sectioned: the shape's pool, then the
        // off-shape facts with the route that admitted it named.
        assert!(
            rendered.contains("\u{b7} the `deploy` pool \u{b7} 1 fact"),
            "{rendered}"
        );
        assert!(
            rendered.contains("\u{b7} off-shape \u{b7} 1 fact outside the pool"),
            "{rendered}"
        );
        // The route reason is long enough to wrap, and it wraps at a
        // word boundary rather than mid-token: the first line carries
        // the reason's head and the floor's name rides the
        // continuation. Both halves are asserted, because a wrap that
        // split the number from its label would be the KI-078 class.
        assert!(
            rendered.contains("admitted because the `deploy` pool's best (0.24) is below the"),
            "{rendered}"
        );
        assert!(rendered.contains("SPILLOVER_FLOOR (0.50)"), "{rendered}");
        // The off-shape card says so on its header, and the assembly
        // line carries the marker.
        assert!(rendered.contains("0.55 resolved: off-shape"), "{rendered}");
        assert!(
            rendered.contains(
                "- resolved: [rendering 0.55 confirmed-by off-shape] the railway deploy is failing"
            ),
            "{rendered}"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn the_recall_surface_renders_the_two_signs_distinctly() {
        let root = scratch("render-signs");
        write_fact(
            &root,
            "what-worked",
            &trajectory_fact(
                "what-worked",
                "the bundle build is slow",
                "build",
                &["pin the webpack cache before the bundle build"],
                0,
            ),
        );
        write_fact(
            &root,
            "what-diverged",
            &trajectory_fact(
                "what-diverged",
                "the bundle output grew past its bound",
                "build, bound-exceeded",
                &["the webpack bundle was left unbounded and failed to compile"],
                0,
            ),
        );
        let memory = Memory::load_with(&root, Box::new(KeywordOnly));
        let rendered = render_recall(&memory, "the webpack bundle fails to compile");
        // The resolution is a candidate; the anti-vector is beside it,
        // marked, in the divergence's color — never a candidate.
        assert!(rendered.contains("resolved:"), "{rendered}");
        assert!(rendered.contains("diverged:"), "{rendered}");
        assert!(
            rendered.contains(WARN),
            "the divergence renders in the caution color"
        );
        // The block carries the legend and both markers, and the
        // anti-vector's line names the class of thing that broke.
        assert!(rendered.contains(SIGN_LEGEND.trim_end()), "{rendered}");
        assert!(rendered.contains("- resolved: [build"), "{rendered}");
        assert!(
            rendered.contains("- diverged: [bound-exceeded"),
            "{rendered}"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn the_recall_surface_lines_are_bounded_at_80_and_120() {
        let root = scratch("surface-width");
        // The KI-078 class on this slice's own surface: a status line
        // built by concatenation and a card's standing line built from
        // every tag the fact carries. Both used to run past the
        // terminal — the header at 136 visible characters, the standing
        // line hard-wrapping mid-word at 80. They wrap now, at a word
        // boundary, through the shared wrapper.
        write_fact(
            &root,
            "three-tags",
            &recalled_fact(
                "three-tags",
                "the railway deploy is failing per branch",
                "deploy, bound-exceeded, claim-outran-evidence",
                RECALL_CEILING,
            ),
        );
        write_fact(
            &root,
            "off-shape-fact",
            &recalled_fact(
                "off-shape-fact",
                "the railway deploy is failing",
                "rendering",
                RECALL_CEILING,
            ),
        );
        let memory = Memory::load_with(&root, Box::new(KeywordOnly));
        for width in [80, 120] {
            let rendered = render_recall_at(&memory, "the railway deploy is failing", width);
            // The block that would enter a turn's context is not a
            // terminal surface — it is a prompt fragment, and its
            // section headers and fact sentences are not bounded to a
            // column count. Everything the surface itself draws is:
            // the title, the status line, the dividers, every card, and
            // the dry-run footer.
            let (surface, tail) = match rendered.split_once("what would enter a turn's context") {
                Some((head, rest)) => (head.to_string(), rest.to_string()),
                None => (rendered.clone(), String::new()),
            };
            let mut lines: Vec<&str> = surface.lines().collect();
            if let Some(last) = tail.lines().last() {
                lines.push(last);
            }
            for line in lines {
                assert!(
                    visible_width(line) <= width,
                    "at {width} columns a line is {} wide: {line:?}",
                    visible_width(line)
                );
            }
            // The wrap is at a word boundary, so no line ends mid-token:
            // the description reads whole on its own line.
            assert!(
                rendered.contains("the railway deploy is failing per branch"),
                "{width}: {rendered}"
            );
        }
        // The shipped bound is the named constant, and the surface at
        // that bound still carries everything it names.
        let shipped = render_recall(&memory, "the railway deploy is failing");
        assert!(
            shipped.contains("shape: deploy (structure-shape)"),
            "{shipped}"
        );
        assert!(shipped.contains("backend: keyword-only"), "{shipped}");
        assert!(shipped.contains("2 facts ranked"), "{shipped}");
        assert!(shipped.contains("1 in the pool"), "{shipped}");
        assert!(shipped.contains("1 off-shape"), "{shipped}");
        // The three-tag standing line wrapped rather than ran past the
        // card, and every tag it carries is still named.
        assert!(
            shipped.contains("shape: deploy, bound-exceeded, claim-outran-evidence"),
            "{shipped}"
        );
        let _ = fs::remove_dir_all(&root);
    }

    /// One rendered line's visible width: the palette codes are not
    /// columns, and a width test that counts them tests nothing.
    fn visible_width(line: &str) -> usize {
        let mut width = 0;
        let mut escape = false;
        for character in line.chars() {
            if escape {
                if character.is_ascii_alphabetic() {
                    escape = false;
                }
                continue;
            }
            if character == '\u{1b}' {
                escape = true;
                continue;
            }
            width += 1;
        }
        width
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

    // --- the write path ---

    /// One working-tier fact in the writer's field order.
    fn working_fact(name: &str, description: &str, date: &str, valid_until: &str) -> String {
        format!(
            "---\nname: {name}\ndescription: \"{description}\"\ntype: project\ndate: {date}\nbasis: inferred\ntier: working\nshape: []\nrecall-count: 0\nsuperseded-by:\nvalid-until: {valid_until}\n---\n\nThe note's body.\n"
        )
    }

    /// B1: the preview is the cycle's own filters — the sets it names
    /// before the pass are exactly the actions the pass takes, on the
    /// same fixture and the same day. The four shapes cover the four
    /// sets: an expired working fact (compress), a stale never-recalled
    /// inferred fact (supersede), the same shape confirmed-by
    /// (untouched), and the same shape incomplete (kept).
    #[test]
    fn the_preview_names_what_the_cycle_would_do() {
        let root = scratch("preview-agrees");
        let old = days_ago(400);
        write_fact(
            &root,
            "stale-working",
            &working_fact("stale-working", "a stale note", &old, &days_ago(2)),
        );
        let durable = |name: &str, description: &str, basis: &str, shape: &str, missing: &str| {
            format!(
                "---\nname: {name}\ndescription: \"{description}\"\ntype: project\ndate: {old}\nbasis: {basis}\ntier: durable\nshape: [{shape}]\nmissing: [{missing}]\nrecall-count: 0\nsuperseded-by:\nvalid-until:\n---\n\nbody\n"
            )
        };
        write_fact(
            &root,
            "stale-inferred",
            &durable("stale-inferred", "a stale guess", "inferred", "", ""),
        );
        write_fact(
            &root,
            "stale-confirmed",
            &durable(
                "stale-confirmed",
                "a stale verified fact",
                "confirmed-by",
                "",
                "",
            ),
        );
        write_fact(
            &root,
            "stalled",
            &durable(
                "stalled",
                "a stalled trajectory",
                "inferred",
                "build, incomplete",
                "the companion",
            ),
        );
        let today = calib::today();
        let before = preview(&root, &today);
        assert_eq!(before.compress, vec!["stale-working".to_string()]);
        assert_eq!(before.supersede, vec!["stale-inferred".to_string()]);
        assert_eq!(before.untouched, vec!["stale-confirmed".to_string()]);
        assert_eq!(before.kept, vec!["stalled".to_string()]);
        // The pass acts on exactly those sets and nothing else.
        let result = consolidate(&root, &today, None);
        let acted: Vec<String> = result.actions.iter().map(|a| a.fact.clone()).collect();
        assert!(acted.contains(&"stale-working".to_string()), "{acted:?}");
        assert!(acted.contains(&"stale-inferred".to_string()), "{acted:?}");
        assert!(!acted.contains(&"stale-confirmed".to_string()));
        assert!(!acted.contains(&"stalled".to_string()));
        assert_eq!(result.untouched, before.untouched);
        assert_eq!(result.kept, before.kept);
        let _ = fs::remove_dir_all(&root);
    }

    /// R4b: the third sign is only real if the companions are named. The
    /// tag was accepted with an empty `missing` — 15 corpus facts carry
    /// the field and all 15 carry it empty — so an `incomplete` fact
    /// rendered `incomplete:` with nothing to finish the trajectory
    /// with, and the graph had no edge to walk.
    #[test]
    fn an_incomplete_fact_must_name_what_it_was_missing() {
        // The tag with no companions is refused, with the reason named.
        let error = Remember::parse(&serde_json::json!({
            "name": "stalled-halfway",
            "description": "the trajectory drove partway and stalled",
            "basis": "inferred",
            "shape": "build, incomplete",
            "phrases": ["the rollback flag was never wired"],
        }))
        .unwrap_err();
        assert!(
            matches!(&error, WriteError::Missing(reason) if reason.contains("third sign")),
            "{error}"
        );
        // The same fact with the companions named is accepted, and the
        // written document carries them where the loader reads them back.
        let root = scratch("incomplete-missing");
        let memory = Memory::load_with(&root, Box::new(KeywordOnly));
        let request = Remember::parse(&serde_json::json!({
            "name": "stalled-halfway",
            "description": "the trajectory drove partway and stalled",
            "basis": "inferred",
            "shape": "build, incomplete",
            "phrases": ["the rollback flag was never wired"],
            "missing": ["the-rollback-flag"],
        }))
        .unwrap();
        assert_eq!(request.missing, vec!["the-rollback-flag".to_string()]);
        let written = memory.remember(&request).unwrap();
        assert_eq!(written.sign(), Sign::Incomplete);
        let text = fs::read_to_string(&written.path).unwrap();
        assert!(text.contains("missing: [the-rollback-flag]\n"), "{text}");
        // A fact with no `incomplete` tag is untouched by the rule: an
        // empty `missing` is fine when the fact is not claiming to have
        // stalled.
        let plain = Remember::parse(&serde_json::json!({
            "name": "a-plain-fact",
            "description": "a fact that finished",
            "basis": "inferred",
            "shape": "build",
        }))
        .unwrap();
        assert!(plain.missing.is_empty());
        let _ = fs::remove_dir_all(&root);
    }

    // --- the measurement over this repository's own corpus -----------------
    //
    // R4c: the recall fixes are measured here, against the tree this
    // module ships in, so the numbers are the gate's own rather than a
    // script's. The assertions are structural, not exact: the corpus is
    // git-tracked and grows, and a test pinned to today's fact count
    // would fail the moment a fact is added.

    /// The twelve instructions the deep dive measured, each naming the
    /// fact in this corpus that answers it.
    const MEASURED: [(&str, &str); 12] = [
        (
            "where does fleet load its skills from",
            "canonical-skills-library-is-agents-skills",
        ),
        (
            "does the CLI remember command write phrases",
            "cli-remember-speaks-the-writers-whole-shape",
        ),
        (
            "what endpoint serves the embedding vectors",
            "vector-path-live-on-loopback-8901",
        ),
        (
            "what cosine did the unrelated fact pairs measure",
            "embeddinggemma-cosines-confirm-ki-058",
        ),
        (
            "which workload does the orchestrator walk",
            "orchestrator-walks-the-hard-chain",
        ),
        (
            "where do credentials resolve from",
            "own-home-credentials-ruling",
        ),
        (
            "what is the harness voice",
            "the-harness-voice-is-best-of-both",
        ),
        (
            "why is a slow lane answer better than a missed one",
            "lane-budget-a-slow-answer-beats-a-missed-one",
        ),
        (
            "where does the harness orientation doc live",
            "harness-orientation-lives-in-docs",
        ),
        (
            "is delegation whole at slice 20",
            "slice-20-delegation-is-whole",
        ),
        (
            "what is the substrate one file per fact",
            "fleet-memories-substrate-is-one-markdown-file-per-fact",
        ),
        (
            "which router enforces the ruling",
            "router-enforcement-ruling",
        ),
    ];

    /// R1 and R2 measured over this repository's own corpus, on the
    /// keyword-only machine so the vector channel is zero and the
    /// lexical channels carry the whole ranking.
    ///
    /// Before: the history channel clamped at a ceiling of 8, 16 of 24
    /// facts sat at or above it, and the correct fact ranked first in 7
    /// of 12 — with two of the five failures decided by the alphabet on
    /// an exact tie. After: the ceiling is 128 on a logarithmic curve,
    /// the tie-break is the trajectory term, and the same set measures
    /// 8 of 12. The assertion is the floor, not the exact number, so the
    /// corpus can grow without reddening the gate.
    #[test]
    fn the_measured_set_holds_the_floor_the_reweighting_bought() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(MEMORIES_DIR);
        let memory = Memory::load_with(&root, Box::new(KeywordOnly));
        let facts = memory.facts();
        assert!(
            facts.len() >= 12,
            "this corpus is the measurement's subject"
        );
        let mut first = 0;
        let mut top_three = 0;
        for (instruction, answer) in MEASURED {
            let hits = memory.recall(instruction);
            let rank = hits
                .iter()
                .position(|hit| hit.fact.name == answer)
                .map(|position| position + 1);
            match rank {
                Some(1) => first += 1,
                Some(2..=3) => top_three += 1,
                _ => {}
            }
        }
        assert!(
            first >= 8,
            "the correct fact ranked first in {first} of {} — R1 and R2 measured 8 before this assertion",
            MEASURED.len()
        );
        assert!(
            first + top_three >= 10,
            "and in the top three in {total} of {len}",
            total = first + top_three,
            len = MEASURED.len()
        );
    }

    /// R1: the history channel discriminates across the range this
    /// corpus actually occupies, instead of clamping two-thirds of it to
    /// the same term. The measured distribution is the evidence: the
    /// ceiling is 128, the median recall-count is far above the old 8,
    /// and no two of the measured counts share a history term.
    #[test]
    fn the_history_channel_separates_the_corpus_that_saturated_it() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(MEMORIES_DIR);
        let facts = load_facts(&root);
        let counts: Vec<u64> = facts.iter().map(|fact| fact.recall_count).collect();
        // The premise this test was written on — "more than half the
        // corpus sits above the old ceiling of 8" — was a measurement of
        // the live corpus at design time, and the corpus grows a batch
        // of low-count facts with every session, so the fraction decays
        // by construction (28 of 66 on 2026-10-26). The property it stood
        // for is the channel's, not the corpus's, so it is pinned here on
        // the motivating SHAPE instead of the live counts: a spread with
        // more than half above the old ceiling must separate, with no
        // two counts sharing a term.
        let motivating: Vec<u64> = vec![0, 1, 2, 4, 8, 12, 20, 33, 51, 80, 128];
        let saturated = motivating.iter().filter(|count| **count >= 8).count();
        assert!(
            saturated * 2 > motivating.len(),
            "the motivating shape: {saturated} of {} counts sit above the old ceiling of 8",
            motivating.len()
        );
        let mut motivating_sorted = motivating.clone();
        motivating_sorted.sort_unstable();
        motivating_sorted.dedup();
        let motivating_terms: Vec<f64> = motivating_sorted
            .iter()
            .copied()
            .filter(|count| *count < RECALL_CEILING)
            .map(history_term)
            .collect();
        for pair in motivating_terms.windows(2) {
            assert!(
                pair[0] < pair[1],
                "the channel separates the motivating corpus: {pair:?}"
            );
        }
        // The curve is strictly increasing below the ceiling, so no two
        // distinct counts there collapse to one term. At and above the
        // ceiling the term is exactly 1.0 by design: a saturated channel
        // is the clamp's own statement, not a defect. The corpus has
        // since grown a fact past 128, which is why the assertion is
        // scoped to the range where the curve discriminates.
        let mut sorted = counts.clone();
        sorted.sort_unstable();
        sorted.dedup();
        let below: Vec<u64> = sorted
            .iter()
            .copied()
            .filter(|count| *count < RECALL_CEILING)
            .collect();
        let terms: Vec<f64> = below.iter().map(|count| history_term(*count)).collect();
        for pair in terms.windows(2) {
            assert!(
                pair[0] < pair[1],
                "the history curve must be strictly increasing below the ceiling: {pair:?}"
            );
        }
        for count in sorted.iter().filter(|count| **count >= RECALL_CEILING) {
            assert_eq!(
                history_term(*count),
                1.0,
                "a count at or past the ceiling is the channel's saturation"
            );
        }
    }

    #[test]
    fn the_write_path_accepts_the_failure_shapes_and_records_the_phrases() {
        let root = scratch("write-signs");
        let memory = Memory::load_with(&root, Box::new(KeywordOnly));
        // A tag outside BOTH taxonomies is refused with both named.
        let error = Remember::parse(&serde_json::json!({
            "name": "typo-tag",
            "description": "a fact with a typo'd shape",
            "basis": "inferred",
            "shape": "telemetry",
        }))
        .unwrap_err();
        assert!(
            matches!(&error, WriteError::Shape(reason) if reason.contains("rendering, state-sync") && reason.contains("fail-open, drops-the-row")),
            "{error}"
        );
        // A failure-shape tag beside a structure tag is accepted, the
        // cap is across both axes, and the written fact's sign follows
        // the tag.
        let request = Remember::parse(&serde_json::json!({
            "name": "the-reader-dropped-a-row",
            "description": "the calibration reader dropped a row it had written",
            "shape": "build, bound-exceeded",
            "basis": "inferred",
            "phrases": ["parse_row drops the whole line"],
        }))
        .unwrap();
        assert_eq!(
            request.shape,
            vec!["build".to_string(), "bound-exceeded".to_string()]
        );
        assert_eq!(
            request.phrases,
            vec!["parse_row drops the whole line".to_string()]
        );
        let written = memory.remember(&request).unwrap();
        assert_eq!(written.sign(), Sign::Divergence);
        // The document renders the phrases beside their sibling shape,
        // and the loader reads them back.
        let text = fs::read_to_string(&written.path).unwrap();
        assert!(text.contains("shape: [build, bound-exceeded]\n"), "{text}");
        assert!(
            text.contains("phrases: [parse_row drops the whole line]\n"),
            "{text}"
        );
        let loaded = load_facts(&root);
        assert_eq!(loaded[0].phrases, request.phrases);
        // A modify keeps the direction the superseded fact recorded.
        let modify = Supersede::parse(&serde_json::json!({
            "name": "the-reader-dropped-a-row",
            "justification": "the reader now takes a null",
            "replacement": "the-reader-keeps-the-row",
            "description": "the calibration reader keeps every row it writes",
        }))
        .unwrap();
        memory.supersede(&modify, false, None).unwrap();
        let replacement = load_facts(&root)
            .into_iter()
            .find(|fact| fact.name == "the-reader-keeps-the-row")
            .expect("the replacement was written");
        assert_eq!(replacement.phrases, request.phrases);

        // The phrase list is bounded, sanitized, and typed.
        let too_many = Remember::parse(&serde_json::json!({
            "name": "too-many-phrases",
            "description": "a fact with too many phrases",
            "basis": "inferred",
            "phrases": (0..MAX_PHRASES + 1).map(|index| format!("phrase {index}")).collect::<Vec<_>>(),
        }))
        .unwrap_err();
        assert!(matches!(too_many, WriteError::Phrases(_)), "{too_many}");
        let not_strings = Remember::parse(&serde_json::json!({
            "name": "bad-phrases",
            "description": "a fact with a non-string phrase",
            "basis": "inferred",
            "phrases": ["fine", 7],
        }))
        .unwrap_err();
        assert!(
            not_strings.to_string().contains("must be a string"),
            "{not_strings}"
        );
        let nasty = Remember::parse(&serde_json::json!({
            "name": "nasty-phrases",
            "description": "a fact with a nasty phrase",
            "basis": "inferred",
            "phrases": ["tab\there and an escape \u{1b}[31m"],
        }))
        .unwrap();
        assert_eq!(
            nasty.phrases,
            vec!["tabhere and an escape [31m".to_string()]
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn the_phrases_round_trip_through_the_write_path_and_a_later_recall() {
        let root = scratch("phrases-round-trip");
        let memory = Memory::load_with(&root, Box::new(KeywordOnly));
        // The write path records the phrases that resolved the problem —
        // the vocabulary that worked — and they ride the fact's indexed
        // text beside the description and the shape tags.
        let request = Remember::parse(&serde_json::json!({
            "name": "deploy-credentials-rule",
            "description": "the deploy credential rule",
            "shape": "deploy",
            "basis": "confirmed-by",
            "phrases": ["read CLAUDE.local.md directly", "never grep-extract credentials"],
        }))
        .unwrap();
        let written = memory.remember(&request).unwrap();
        assert_eq!(
            written.indexed_text(),
            "the deploy credential rule deploy read CLAUDE.local.md directly never grep-extract credentials"
        );
        // The topical text is the description and the tags; the phrases
        // are the trajectory's own field, so the two dimensions cannot
        // double-count each other.
        assert_eq!(written.topic_text(), "the deploy credential rule deploy");
        assert!(!written.topic_text().contains("CLAUDE.local.md"));

        // The control twin: the same fact with no phrases recorded.
        let twin = Remember::parse(&serde_json::json!({
            "name": "deploy-credentials-twin",
            "description": "the deploy credential rule",
            "shape": "deploy",
            "basis": "confirmed-by",
        }))
        .unwrap();
        memory.remember(&twin).unwrap();

        // A later recall for the shape, in a DIFFERENT wording: the
        // phrases are what match, and the twin — same description, same
        // shape, no direction — never clears the floor.
        let reloaded = Memory::load_with(&root, Box::new(KeywordOnly));
        let hits = reloaded.recall("read CLAUDE.local.md for the railway credentials");
        assert_eq!(hit_names(&hits), vec!["deploy-credentials-rule"]);
        let hit = &hits[0];
        assert!(
            (hit.score.trajectory - 0.625).abs() < 1e-12,
            "{:?}",
            hit.score
        );
        // 0.25, not the 0.125 the unstemmed tokenizer read: the
        // instruction's `credentials` and the phrase's `credential`
        // are one stem now (R3).
        assert!((hit.score.keyword - 0.25).abs() < 1e-12, "{:?}", hit.score);
        assert!(!hit.off_shape, "the fact carries the instruction's shape");
        // The phrases round-trip through the file: the loader reads back
        // exactly what the write path recorded.
        let loaded = load_facts(&root);
        let fact = loaded
            .iter()
            .find(|fact| fact.name == "deploy-credentials-rule")
            .expect("the fact is on disk");
        assert_eq!(fact.phrases, request.phrases);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_written_fact_round_trips_through_the_loader() {
        let root = scratch("write-round-trip");
        let memory = Memory::load_with(&root, Box::new(KeywordOnly));
        let request = Remember::parse(&serde_json::json!({
            "name": "The-Gate-Is-Local",
            "description": "Fleet's gate is local and its exit code is read, never piped",
            "type": "feedback",
            "shape": "build, change",
            "basis": "confirmed-by",
            "tier": "durable",
            "body": "The full content, with a second line.",
        }))
        .unwrap();
        let written = memory.remember(&request).unwrap();

        // The file loads, and every field survives the round trip —
        // the written document is the record the parser reads back.
        let loaded = load_facts(&root);
        assert_eq!(loaded.len(), 1);
        let fact = &loaded[0];
        assert_eq!(fact.name, "the-gate-is-local");
        assert_eq!(fact.description, request.description);
        assert_eq!(fact.fact_type, "feedback");
        assert_eq!(fact.date, calib::today());
        assert_eq!(fact.basis, Basis::ConfirmedBy);
        assert_eq!(fact.tier, Tier::Durable);
        assert_eq!(fact.shape, vec!["build".to_string(), "change".to_string()]);
        assert_eq!(fact.recall_count, 0);
        assert!(fact.superseded_by.is_empty());
        assert!(fact.valid_until.is_empty());
        assert_eq!(fact.body, "The full content, with a second line.");
        assert_eq!(fact.path, root.join("the-gate-is-local.md"));
        // The returned fact is the file's fact, stamp included.
        assert_eq!(&written, fact);
        // The document renders in the fork's field order, tier beside
        // basis, with the unset optionals bare.
        let text = fs::read_to_string(&fact.path).unwrap();
        assert!(text.starts_with(
            "---\nname: the-gate-is-local\ndescription: \"Fleet's gate is local and its exit code is read, never piped\"\ntype: feedback\ndate: "
        ), "{text}");
        assert!(
            text.contains("basis: confirmed-by\ntier: durable\n"),
            "{text}"
        );
        assert!(text.contains("shape: [build, change]\n"), "{text}");
        assert!(text.contains("superseded-by:\nvalid-until:\n"), "{text}");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_write_without_a_basis_is_a_typed_error() {
        let root = scratch("write-basis");
        let error = Remember::parse(&serde_json::json!({
            "name": "no-basis",
            "description": "a fact with no provenance",
        }))
        .unwrap_err();
        assert!(
            matches!(&error, WriteError::Basis(reason) if reason.contains("required on every write")),
            "{error}"
        );
        // A basis outside the vocabulary is the same refusal, not a
        // silent default.
        let error = Remember::parse(&serde_json::json!({
            "name": "bad-basis",
            "description": "a fact with a made-up basis",
            "basis": "guessed",
        }))
        .unwrap_err();
        assert!(matches!(&error, WriteError::Basis(_)), "{error}");
        // Nothing was written by either refusal.
        assert!(load_facts(&root).is_empty());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_working_write_without_a_valid_until_is_a_typed_error() {
        let root = scratch("write-valid-until");
        let memory = Memory::load_with(&root, Box::new(KeywordOnly));
        let error = Remember::parse(&serde_json::json!({
            "name": "no-expiry",
            "description": "a session-scoped fact with no end",
            "basis": "inferred",
            "tier": "working",
        }))
        .unwrap_err();
        assert!(
            matches!(&error, WriteError::ValidUntil(reason) if reason.contains("required")),
            "{error}"
        );
        // The durable tier's default is no expiry, and it says so by
        // rendering the field bare.
        let request = Remember::parse(&serde_json::json!({
            "name": "durable-default",
            "description": "a durable fact with no expiry",
            "basis": "inferred",
        }))
        .unwrap();
        assert_eq!(request.tier, Tier::Durable);
        assert!(request.valid_until.is_empty());
        memory.remember(&request).unwrap();
        assert!(fs::read_to_string(root.join("durable-default.md"))
            .unwrap()
            .contains("valid-until:\n"));
        // A day that is not real is refused, not written as one.
        let error = Remember::parse(&serde_json::json!({
            "name": "bad-day",
            "description": "a fact with an impossible expiry",
            "basis": "inferred",
            "tier": "working",
            "valid-until": "2026-02-30",
        }))
        .unwrap_err();
        assert!(matches!(&error, WriteError::ValidUntil(_)), "{error}");
        // Episodic is the cycle's tier, never a write's.
        let error = Remember::parse(&serde_json::json!({
            "name": "by-hand",
            "description": "a hand-written episodic summary",
            "basis": "inferred",
            "tier": "episodic",
        }))
        .unwrap_err();
        assert!(
            matches!(&error, WriteError::Tier(reason) if reason.contains("consolidation cycle")),
            "{error}"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn the_writer_refuses_a_bad_name_and_a_taken_one() {
        let root = scratch("write-names");
        let memory = Memory::load_with(&root, Box::new(KeywordOnly));
        // Five unusable names: a path, a parent, a dotfile, a space,
        // and a leading hyphen — none of them may address a file
        // outside the corpus.
        for name in ["../escape", "..", ".hidden", "has space", "-lead"] {
            let error = Remember::parse(&serde_json::json!({
                "name": name,
                "description": "a fact with an unusable name",
                "basis": "inferred",
            }))
            .unwrap_err();
            assert!(matches!(&error, WriteError::Name(_)), "{name}: {error}");
        }
        // A name is lowercased, not refused for its case: `UPPER`
        // writes `upper`.
        let upper = Remember::parse(&serde_json::json!({
            "name": "UPPER",
            "description": "a fact with a shouty name",
            "basis": "inferred",
        }))
        .unwrap();
        assert_eq!(upper.name, "upper");
        // A name that is already taken is refused rather than
        // overwritten: the modify path is `supersede`.
        let request = Remember::parse(&serde_json::json!({
            "name": "taken",
            "description": "the first fact under this name",
            "basis": "inferred",
        }))
        .unwrap();
        memory.remember(&request).unwrap();
        let again = Remember::parse(&serde_json::json!({
            "name": "taken",
            "description": "a second fact under the same name",
            "basis": "confirmed-by",
        }))
        .unwrap();
        let error = memory.remember(&again).unwrap_err();
        assert!(
            matches!(&error, WriteError::Exists(reason) if reason.contains("supersede")),
            "{error}"
        );
        // The first fact is untouched.
        let facts = load_facts(&root);
        assert_eq!(facts.len(), 1);
        assert_eq!(facts[0].description, "the first fact under this name");
        // A shape outside the taxonomy is refused with the taxonomy
        // named, and four tags are past the writer's cap.
        let error = Remember::parse(&serde_json::json!({
            "name": "bad-shape",
            "description": "a fact with a typo'd shape",
            "basis": "inferred",
            "shape": "telemetry",
        }))
        .unwrap_err();
        assert!(
            matches!(&error, WriteError::Shape(reason) if reason.contains("rendering, state-sync")),
            "{error}"
        );
        let error = Remember::parse(&serde_json::json!({
            "name": "too-many-shapes",
            "description": "a fact with too many tags",
            "basis": "inferred",
            "shape": "deploy, build, change, docs",
        }))
        .unwrap_err();
        assert!(matches!(&error, WriteError::Shape(_)), "{error}");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn the_writer_bounds_and_sanitizes_the_description() {
        // KI-060: a description carrying newlines, tabs, an escape
        // sequence, and two thousand characters. The writer strips the
        // control characters — so no injected line can spoof a context
        // block and no escape reaches the surface — and bounds the
        // length with a marked cut.
        let root = scratch("sanitize");
        let memory = Memory::load_with(&root, Box::new(KeywordOnly));
        let nasty = format!(
            "line one\nline two\twith an escape \u{1b}[31m and a bell \u{7} {}",
            "x".repeat(2_000)
        );
        let request = Remember::parse(&serde_json::json!({
            "name": "nasty",
            "description": nasty,
            // Confirmed-by so the fact clears MIN_RECALL_SCORE on its
            // word evidence alone: the assertions below are about the
            // block's sanitization, not about the ranking.
            "basis": "confirmed-by",
        }))
        .unwrap();
        let written = memory.remember(&request).unwrap();
        assert!(
            !written
                .description
                .chars()
                .any(|character| character.is_control()),
            "{:?}",
            written.description
        );
        assert!(
            written.description.chars().count() <= MAX_DESCRIPTION_CHARS + 1,
            "{}",
            written.description.chars().count()
        );
        assert!(
            written.description.ends_with('\u{2026}'),
            "{}",
            written.description
        );
        // It still round-trips: the sanitized description is what the
        // loader reads back.
        let loaded = load_facts(&root);
        assert_eq!(loaded[0].description, written.description);
        // And the assembled block — which carries no palette codes of
        // its own — carries none of the description's either. The
        // corpus is re-read, because a write lands on disk.
        let reloaded = Memory::load_with(&root, Box::new(KeywordOnly));
        let assembly = reloaded.assemble("line one", &[]);
        let block = render_assembly(&assembly);
        assert!(!block.is_empty(), "the fact assembled");
        // The escape character is gone; its payload stays as visible,
        // inert text — a stripped sequence cannot repaint a terminal.
        assert!(!block.contains('\u{1b}'), "{block:?}");
        assert!(block.contains("[31m"), "{block:?}");
        // A working fact is bounded the same way.
        let request = Remember::parse(&serde_json::json!({
            "name": "nasty-working",
            "description": "tab\there",
            "basis": "inferred",
            "tier": "working",
            "valid-until": "2099-01-01",
        }))
        .unwrap();
        assert_eq!(memory.remember(&request).unwrap().description, "tabhere");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn the_writer_refuses_a_body_past_its_byte_bound() {
        // KI-060's body rule is a refusal, not a cut: the body is the
        // fact's full content, and silently truncating it would change
        // what the fact says.
        let root = scratch("body-bound");
        let _memory = Memory::load_with(&root, Box::new(KeywordOnly));
        let error = Remember::parse(&serde_json::json!({
            "name": "huge",
            "description": "a fact with an enormous body",
            "basis": "inferred",
            "body": "x".repeat(MAX_BODY_BYTES + 1),
        }))
        .unwrap_err();
        assert!(matches!(error, WriteError::Body(_)), "{error}");
        assert!(
            error.to_string().contains(&MAX_BODY_BYTES.to_string()),
            "{error}"
        );
        // Nothing landed: a refused write leaves no file behind.
        assert!(load_facts(&root).is_empty());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn the_supersede_path_inherits_or_clears_the_missing_companions() {
        let root = scratch("supersede-missing");
        let memory = Memory::load_with(&root, Box::new(KeywordOnly));
        let request = Remember::parse(&serde_json::json!({
            "name": "the-bootstrap-stalled",
            "description": "the bootstrap was correct but the torch stack was missing",
            "shape": "build, incomplete",
            "basis": "inferred",
            "phrases": ["the bootstrap ran and the model loaded"],
            "missing": ["the torch stack", "the CUDA driver"],
        }))
        .unwrap();
        memory.remember(&request).unwrap();

        // Inherit: a modify that rewords the fact does not unmiss what
        // the trajectory was missing.
        let modify = Supersede::parse(&serde_json::json!({
            "name": "the-bootstrap-stalled",
            "justification": "the sentence needed the companions named",
            "replacement": "the-bootstrap-stalled-v2",
            "description": "the bootstrap was correct and the torch stack was still missing",
        }))
        .unwrap();
        assert!(
            modify.missing.is_none(),
            "an absent argument is an inheritance"
        );
        memory.supersede(&modify, false, None).unwrap();
        let inherited = load_facts(&root)
            .into_iter()
            .find(|fact| fact.name == "the-bootstrap-stalled-v2")
            .expect("the replacement was written");
        assert_eq!(inherited.missing, request.missing);
        assert_eq!(inherited.sign(), Sign::Incomplete);

        // Clear: the companions arrived, so an explicit empty list says
        // so — and the replacement stops being an incomplete vector.
        let completed = Supersede::parse(&serde_json::json!({
            "name": "the-bootstrap-stalled-v2",
            "justification": "the torch stack landed",
            "replacement": "the-bootstrap-complete",
            "description": "the bootstrap runs end to end with the torch stack",
            "shape": "build",
            "missing": [],
        }))
        .unwrap();
        memory.supersede(&completed, false, None).unwrap();
        let done = load_facts(&root)
            .into_iter()
            .find(|fact| fact.name == "the-bootstrap-complete")
            .expect("the replacement was written");
        assert!(done.missing.is_empty(), "{:?}", done.missing);
        assert_eq!(done.sign(), Sign::Resolution);

        // Replace: a list is a statement, and it rides the new fact.
        let restalled = Supersede::parse(&serde_json::json!({
            "name": "the-bootstrap-complete",
            "justification": "the ROCm stack is now the absent companion",
            "replacement": "the-bootstrap-stalled-again",
            "description": "the bootstrap was correct but the ROCm stack was missing",
            "shape": "build, incomplete",
            "missing": ["the ROCm stack"],
        }))
        .unwrap();
        memory.supersede(&restalled, false, None).unwrap();
        let again = load_facts(&root)
            .into_iter()
            .find(|fact| fact.name == "the-bootstrap-stalled-again")
            .expect("the replacement was written");
        assert_eq!(again.missing, vec!["the ROCm stack".to_string()]);
        assert_eq!(again.sign(), Sign::Incomplete);

        // The list is bounded and typed like the phrases'.
        let too_many = Remember::parse(&serde_json::json!({
            "name": "too-many-missing",
            "description": "a fact with too many companions",
            "basis": "inferred",
            "missing": (0..MAX_MISSING + 1).map(|index| format!("companion {index}")).collect::<Vec<_>>(),
        }))
        .unwrap_err();
        assert!(matches!(too_many, WriteError::Missing(_)), "{too_many}");
        let not_strings = Remember::parse(&serde_json::json!({
            "name": "bad-missing",
            "description": "a fact with a non-string companion",
            "basis": "inferred",
            "missing": ["fine", 7],
        }))
        .unwrap_err();
        assert!(
            not_strings.to_string().contains("must be a string"),
            "{not_strings}"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn the_writer_bounds_the_type_and_the_justification() {
        // The description's bound was pinned first; the same
        // sanitize_line rule covers the type at write time and the
        // justification at supersede time — control characters
        // stripped, length bounded with a marked cut.
        let root = scratch("type-justification");
        let memory = Memory::load_with(&root, Box::new(KeywordOnly));
        let request = Remember::parse(&serde_json::json!({
            "name": "typed",
            "description": "a fact whose type is long and carries a newline",
            "basis": "inferred",
            "type": format!("{}\n{}", "t".repeat(MAX_TYPE_CHARS + 10), "control"),
        }))
        .unwrap();
        let written = memory.remember(&request).unwrap();
        assert!(
            !written
                .fact_type
                .chars()
                .any(|character| character.is_control()),
            "{:?}",
            written.fact_type
        );
        assert!(
            written.fact_type.chars().count() <= MAX_TYPE_CHARS + 1,
            "{}",
            written.fact_type.chars().count()
        );
        assert!(
            written.fact_type.ends_with('\u{2026}'),
            "{}",
            written.fact_type
        );

        // The justification rides supersede, bounded the same way where
        // the calibration row and the retirement note read it.
        let stream_root = scratch("type-justification-stream");
        let stream = calib::Stream::open(&stream_root);
        let request = Supersede::parse(&serde_json::json!({
            "name": "typed",
            "justification": format!("{}\n{}", "j".repeat(MAX_JUSTIFICATION_CHARS + 10), "tail"),
        }))
        .unwrap();
        memory.supersede(&request, false, Some(&stream)).unwrap();
        let retired = fs::read_to_string(root.join("typed.md")).unwrap();
        let note = retired
            .lines()
            .find(|line| line.contains("Retired"))
            .expect("the retirement note carries the justification");
        assert!(
            !note.chars().any(|character| character.is_control()),
            "{note}"
        );
        assert!(note.ends_with('\u{2026}'), "{note}");
        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&stream_root);
    }

    #[test]
    fn a_working_fact_past_its_valid_until_is_retired_by_the_loader() {
        let root = scratch("working-expiry");
        write_fact(
            &root,
            "expired-note",
            &working_fact(
                "expired-note",
                "a working note whose time has passed",
                &days_ago(3),
                &days_ago(1),
            ),
        );
        write_fact(
            &root,
            "live-note",
            &working_fact(
                "live-note",
                "a working note still in its window",
                &days_ago(0),
                &days_ago(-1),
            ),
        );
        let memory = Memory::load_with(&root, Box::new(KeywordOnly));
        // The loader loads both — a retired fact is still a file — but
        // only the live one is working memory.
        assert_eq!(load_facts(&root).len(), 2);
        let working = memory.working();
        assert_eq!(fact_names(&working), vec!["live-note"]);
        // The expired one is out of recall and out of the working tier.
        let recalled: Vec<String> = memory
            .recall("a working note whose time has passed")
            .into_iter()
            .map(|hit| hit.fact.name)
            .collect();
        assert!(
            !recalled.contains(&"expired-note".to_string()),
            "{recalled:?}"
        );
        let assembly = memory.assemble("a working note whose time has passed", &working);
        assert!(assembly
            .working
            .iter()
            .all(|fact| fact.name != "expired-note"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn the_working_tier_enters_the_assembly_and_stays_out_of_the_ranking() {
        let root = scratch("working-tier");
        write_fact(
            &root,
            "durable-one",
            &good_fact(
                "durable-one",
                "the railway deploy is failing per branch",
                "project",
                "deploy",
            ),
        );
        write_fact(
            &root,
            "working-note",
            &working_fact(
                "working-note",
                "the railway deploy needs a re-check",
                &days_ago(0),
                &days_ago(-1),
            ),
        );
        let memory = Memory::load_with(&root, Box::new(KeywordOnly));

        let working = memory.working();
        assert_eq!(fact_names(&working), vec!["working-note"]);
        // The ranking excludes it: a working fact enters through the
        // working tier, so no fact enters a turn twice.
        let ranked: Vec<String> = memory
            .recall("the railway deploy is failing")
            .into_iter()
            .map(|hit| hit.fact.name)
            .collect();
        assert_eq!(ranked, vec!["durable-one".to_string()]);

        let assembly = memory.assemble("the railway deploy is failing", &working);
        let block = render_assembly(&assembly);
        assert!(
            block.contains("Working memory (session-scoped):"),
            "{block}"
        );
        assert!(
            block.contains("the railway deploy needs a re-check"),
            "{block}"
        );
        // The tier renders last, after the protected block and the
        // ranked section.
        let ranked_at = block.find("[deploy ").expect("the ranked section renders");
        let working_at = block.find("[working ").expect("the working tier renders");
        assert!(ranked_at < working_at, "{block}");
        let _ = fs::remove_dir_all(&root);
    }

    // --- the consolidation cycle ---

    /// The cycle corpus: a working fact past its valid-until, a working
    /// fact older than the age, a stale inferred fact, a stale
    /// confirmed-by fact that must survive, an expired durable fact,
    /// and a fresh one.
    fn cycle_corpus(root: &Path) {
        write_fact(
            root,
            "expired-note",
            &working_fact(
                "expired-note",
                "a working note whose time has passed",
                &days_ago(10),
                &days_ago(1),
            ),
        );
        write_fact(
            root,
            "old-note",
            &working_fact(
                "old-note",
                "a working note nobody renewed",
                &days_ago(30),
                &days_ago(-30),
            ),
        );
        write_fact(
            root,
            "stale-inferred",
            &format!(
                "---\nname: stale-inferred\ndescription: \"an inferred fact nobody ever recalled\"\ntype: project\ndate: {}\nbasis: inferred\ntier: durable\nshape: []\nrecall-count: 0\nsuperseded-by:\nvalid-until:\n---\n\nThe body.\n",
                days_ago(200)
            ),
        );
        write_fact(
            root,
            "stale-confirmed",
            &format!(
                "---\nname: stale-confirmed\ndescription: \"a confirmed-by fact nobody ever recalled\"\ntype: project\ndate: {}\nbasis: confirmed-by\ntier: durable\nshape: []\nrecall-count: 0\nsuperseded-by:\nvalid-until:\n---\n\nThe body.\n",
                days_ago(200)
            ),
        );
        write_fact(
            root,
            "expired-durable",
            &format!(
                "---\nname: expired-durable\ndescription: \"a durable fact whose time has passed\"\ntype: project\ndate: {}\nbasis: confirmed-by\ntier: durable\nshape: []\nrecall-count: 0\nsuperseded-by:\nvalid-until: {}\n---\n\nThe body.\n",
                days_ago(60),
                days_ago(1)
            ),
        );
        write_fact(
            root,
            "fresh",
            &good_fact("fresh", "a durable fact written today", "project", "deploy"),
        );
    }

    #[test]
    fn a_fold_that_cannot_land_stamps_records_and_archives_nothing() {
        let root = scratch("cycle-failed-fold");
        write_fact(
            &root,
            "expired-note",
            &working_fact(
                "expired-note",
                "a working note whose time has passed",
                &days_ago(10),
                &days_ago(1),
            ),
        );
        // The corpus goes read-only: the summary's write cannot land,
        // and with it neither the stamps, the rows, nor the archive
        // move (KI-212).
        let mut sealed = fs::metadata(&root).unwrap().permissions();
        sealed.set_mode(0o555);
        fs::set_permissions(&root, sealed).unwrap();
        let result = consolidate(&root, &calib::today(), None);
        let mut open = fs::metadata(&root).unwrap().permissions();
        open.set_mode(0o755);
        fs::set_permissions(&root, open).unwrap();

        // The pass names nothing it did not do: no summary, no actions,
        // and the source fact is exactly as it was — live, rankable,
        // unstamped.
        assert!(result.summary.is_none());
        assert!(result.actions.is_empty());
        let fact_text = fs::read_to_string(root.join("expired-note.md")).unwrap();
        assert!(fact_text.contains("The note's body."), "{fact_text}");
        assert!(
            fact_text.contains("superseded-by:\n"),
            "unstamped: {fact_text}"
        );

        // The next pass, with the disk writable again, folds it whole.
        let retried = consolidate(&root, &calib::today(), None);
        assert!(retried.summary.is_some());
        assert!(
            retried
                .actions
                .iter()
                .any(|action| action.action == calib::ACTION_COMPRESS
                    && action.fact == "expired-note")
        );
    }

    #[test]
    fn the_cycle_compresses_supersedes_and_forgets_with_their_rows() {
        let root = scratch("cycle");
        cycle_corpus(&root);
        let stream_root = scratch("cycle-stream");
        let stream = calib::Stream::open(&stream_root);

        let result = consolidate(&root, &calib::today(), Some(&stream));
        drop(stream);

        // The summary: one episodic fact naming everything folded, and
        // the stamp on every folded fact's file.
        let summary = result.summary.clone().expect("the pass wrote a summary");
        assert!(summary.starts_with("consolidation-"), "{summary}");
        let summary_text = fs::read_to_string(root.join(format!("{summary}.md"))).unwrap();
        assert!(summary_text.contains("tier: episodic"), "{summary_text}");
        assert!(summary_text.contains("expired-note"), "{summary_text}");
        assert!(summary_text.contains("old-note"), "{summary_text}");
        assert!(summary_text.contains("stale-inferred"), "{summary_text}");

        // Compress: the expired working note is folded and then
        // archived — its content lives in the summary, the file leaves
        // the working tree. The old-but-unexpired one is folded and
        // stays as the record.
        assert!(!root.join("expired-note.md").exists());
        assert!(root.join(".archive").join("expired-note.md").exists());
        let old = fs::read_to_string(root.join("old-note.md")).unwrap();
        assert!(old.contains(&format!("superseded-by: {summary}")), "{old}");

        // Supersede: the stale inferred fact is folded and stamped.
        let stale = fs::read_to_string(root.join("stale-inferred.md")).unwrap();
        assert!(
            stale.contains(&format!("superseded-by: {summary}")),
            "{stale}"
        );

        // Forget: the expired durable fact is archived, not deleted.
        assert!(!root.join("expired-durable.md").exists());
        assert!(root.join(".archive").join("expired-durable.md").exists());

        // The fresh fact is untouched, and the confirmed-by stale fact
        // survives — named in `untouched`, not silently skipped.
        assert!(root.join("fresh.md").exists());
        assert!(root.join("stale-confirmed.md").exists());
        let confirmed = fs::read_to_string(root.join("stale-confirmed.md")).unwrap();
        assert!(
            !confirmed.contains("superseded-by: consolidation"),
            "{confirmed}"
        );
        assert_eq!(result.untouched, vec!["stale-confirmed".to_string()]);

        // Every action landed as a `consolidate` row with its reason.
        let yesterday = days_ago(1);
        let rows = calib::read_day(&stream_root, &calib::today(), 64);
        let actions: Vec<(&str, &str, &str)> = rows
            .iter()
            .map(|row| (row.action.as_str(), row.fact.as_str(), row.reason.as_str()))
            .collect();
        assert!(
            actions.contains(&(
                "compress",
                "expired-note",
                &format!("working memory past its valid-until {yesterday}")
            )),
            "{actions:?}"
        );
        assert!(
            actions.contains(&("compress", "old-note", "working memory older than 7 days")),
            "{actions:?}"
        );
        assert!(
            actions.contains(&(
                "supersede",
                "stale-inferred",
                "inferred, never recalled, older than 180 days"
            )),
            "{actions:?}"
        );
        assert!(
            actions.contains(&("forget", "expired-durable", &format!("expired {yesterday}"))),
            "{actions:?}"
        );
        assert_eq!(rows.len(), 5, "{actions:?}");

        // The folded facts are retired from recall by their stamp.
        let memory = Memory::load_with(&root, Box::new(KeywordOnly));
        let recalled: Vec<String> = memory
            .recall("an inferred fact nobody ever recalled")
            .into_iter()
            .map(|hit| hit.fact.name)
            .collect();
        assert!(
            !recalled.contains(&"stale-inferred".to_string()),
            "{recalled:?}"
        );
        assert!(
            !recalled.contains(&"expired-note".to_string()),
            "{recalled:?}"
        );
        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&stream_root);
    }

    #[test]
    fn the_cycle_keeps_an_incomplete_fact_rather_than_folding_it() {
        let root = scratch("cycle-incomplete");
        // The open decision, pinned: does the consolidation cycle fold
        // an incomplete durable inferred fact at STALE_AFTER_DAYS?
        // It does NOT. The summary's bullets carry a fact's name, date,
        // and description — never its missing companions — so folding
        // one would keep the sentence and lose the absent pieces, which
        // are the fact's whole point. It is named and left alone, the
        // way a confirmed-by fact is.
        write_fact(
            &root,
            "stalled-inferred",
            &format!(
                "---\nname: stalled-inferred\ndescription: \"an inferred trajectory nobody ever finished\"\ntype: project\ndate: {}\nbasis: inferred\ntier: durable\nshape: [build, incomplete]\nphrases: [the bootstrap ran]\nmissing: [the torch stack]\nrecall-count: 0\nsuperseded-by:\nvalid-until:\n---\n\nThe body.\n",
                days_ago(200)
            ),
        );
        // The control: the same fact without the outcome tag is folded,
        // so the rule is the sign's and not an accident of the corpus.
        write_fact(
            &root,
            "stale-inferred",
            &format!(
                "---\nname: stale-inferred\ndescription: \"an inferred fact nobody ever recalled\"\ntype: project\ndate: {}\nbasis: inferred\ntier: durable\nshape: [build]\nrecall-count: 0\nsuperseded-by:\nvalid-until:\n---\n\nThe body.\n",
                days_ago(200)
            ),
        );
        let result = consolidate(&root, &calib::today(), None);
        // The control folded; the incomplete one did not.
        let folded: Vec<&str> = result
            .actions
            .iter()
            .filter(|action| action.action == calib::ACTION_SUPERSEDE)
            .map(|action| action.fact.as_str())
            .collect();
        assert_eq!(folded, vec!["stale-inferred"], "{result:?}");
        assert_eq!(result.kept, vec!["stalled-inferred".to_string()]);
        // The file is untouched: no stamp, still live, still recallable
        // with its companions.
        let kept = fs::read_to_string(root.join("stalled-inferred.md")).unwrap();
        assert!(!kept.contains("superseded-by: consolidation"), "{kept}");
        assert!(kept.contains("missing: [the torch stack]"), "{kept}");
        let memory = Memory::load_with(&root, Box::new(KeywordOnly));
        let hits = memory.recall("the bootstrap trajectory nobody ever finished");
        assert_eq!(hit_names(&hits), vec!["stalled-inferred"]);
        assert_eq!(hits[0].fact.missing, vec!["the torch stack".to_string()]);
        // The surface names what it kept and why.
        let rendered = render_consolidation(&result);
        assert!(
            rendered.contains("left alone: stalled-inferred"),
            "{rendered}"
        );
        assert!(
            rendered.contains("incomplete, so the cycle never folds one"),
            "{rendered}"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_second_pass_the_same_day_takes_a_numbered_summary() {
        let root = scratch("cycle-twice");
        write_fact(
            &root,
            "old-note",
            &working_fact(
                "old-note",
                "a working note nobody renewed",
                &days_ago(30),
                &days_ago(-30),
            ),
        );
        let first = consolidate(&root, &calib::today(), None);
        let first_summary = first
            .summary
            .clone()
            .expect("the first pass wrote a summary");
        // The pass stamped its fact, so a second pass finds nothing
        // left to fold — and writes no second summary.
        let second = consolidate(&root, &calib::today(), None);
        assert!(second.is_empty(), "{second:?}");
        assert_eq!(first_summary, format!("consolidation-{}", calib::today()));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn an_empty_corpus_consolidates_to_nothing() {
        let root = scratch("cycle-empty");
        let result = consolidate(&root, &calib::today(), None);
        assert!(result.is_empty());
        assert!(result.summary.is_none());
        assert!(!root.join(".archive").exists());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn an_expired_working_fact_is_compressed_and_archived_in_the_same_pass() {
        let root = scratch("cycle-order");
        write_fact(
            &root,
            "expired-note",
            &working_fact(
                "expired-note",
                "a working note whose time has passed",
                &days_ago(10),
                &days_ago(1),
            ),
        );

        let result = consolidate(&root, &calib::today(), None);
        let summary = result.summary.clone().expect("the pass wrote a summary");

        // The ordering this pins: compress stamps the fact, and the
        // forget loop — which reads the pass's pre-pass snapshot, where
        // the stamp is not yet in memory — archives the very file the
        // stamp just landed on. The archived copy therefore carries
        // `superseded-by`, and both actions name the fact.
        let archived = fs::read_to_string(root.join(".archive").join("expired-note.md")).unwrap();
        assert!(
            archived.contains(&format!("superseded-by: {summary}")),
            "{archived}"
        );
        assert!(!root.join("expired-note.md").exists());
        let facts: Vec<&str> = result
            .actions
            .iter()
            .filter(|action| action.fact == "expired-note")
            .map(|action| action.action)
            .collect();
        assert_eq!(
            facts,
            vec![calib::ACTION_COMPRESS, calib::ACTION_FORGET],
            "{result:?}"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_re_used_fact_name_archives_both_copies_and_the_loader_ignores_the_archive() {
        let root = scratch("archive-collision");
        // First life of the name: it expires, and the cycle archives it.
        write_fact(
            &root,
            "reused",
            &working_fact(
                "reused",
                "the first fact of this name",
                &days_ago(10),
                &days_ago(1),
            ),
        );
        let first = consolidate(&root, &calib::today(), None);
        assert!(
            first
                .actions
                .iter()
                .any(|action| action.action == calib::ACTION_FORGET && action.fact == "reused"),
            "{first:?}"
        );
        assert!(root.join(".archive").join("reused.md").exists());

        // Second life: the name is written again, expires again, and is
        // archived beside the first copy rather than over it.
        write_fact(
            &root,
            "reused",
            &working_fact(
                "reused",
                "the second fact of this name",
                &days_ago(10),
                &days_ago(1),
            ),
        );
        let second = consolidate(&root, &calib::today(), None);
        assert!(
            second
                .actions
                .iter()
                .any(|action| action.action == calib::ACTION_FORGET && action.fact == "reused"),
            "{second:?}"
        );

        // Both versions survive under distinct names, each with its own
        // content — the archive is the record, and git is not the only
        // backstop.
        assert!(root.join(".archive").join("reused.md").exists());
        assert!(root.join(".archive").join("reused.2.md").exists());
        let first_copy = fs::read_to_string(root.join(".archive").join("reused.md")).unwrap();
        let second_copy = fs::read_to_string(root.join(".archive").join("reused.2.md")).unwrap();
        assert!(
            first_copy.contains("the first fact of this name"),
            "{first_copy}"
        );
        assert!(
            second_copy.contains("the second fact of this name"),
            "{second_copy}"
        );

        // The loader reads `memories/*.md` and never a subdirectory:
        // neither archived copy is a live fact, and the archive surface
        // cards both by the name their frontmatter carries.
        let live = load_facts(&root);
        assert!(live.iter().all(|fact| fact.name != "reused"), "{live:?}");
        let rendered = render_archive(&root);
        assert!(rendered.contains("2 forgotten facts"), "{rendered}");
        assert!(rendered.matches("reused").count() >= 2, "{rendered}");
        let _ = fs::remove_dir_all(&root);
    }

    // --- provenance as the permission model ---

    /// A corpus with one confirmed-by fact and one inferred fact, both
    /// live.
    fn provenance_corpus(root: &Path) {
        write_fact(
            root,
            "load-bearing",
            &good_fact(
                "load-bearing",
                "the owner's standing rule about the gate",
                "feedback",
                "",
            ),
        );
        write_fact(
            root,
            "a-guess",
            &format!(
                "---\nname: a-guess\ndescription: \"an inferred guess about the deploy\"\ntype: project\ndate: {}\nbasis: inferred\ntier: durable\nshape: [deploy]\nrecall-count: 0\nsuperseded-by:\nvalid-until:\n---\n\nThe body.\n",
                calib::today()
            ),
        );
    }

    #[test]
    fn a_confirmed_by_supersede_is_refused_without_approval() {
        let root = scratch("provenance-refused");
        provenance_corpus(&root);
        let memory = Memory::load_with(&root, Box::new(KeywordOnly));
        let request = Supersede::parse(&serde_json::json!({
            "name": "load-bearing",
            "justification": "the owner changed his mind",
            "replacement": "load-bearing-v2",
            "description": "the owner's new standing rule about the gate",
        }))
        .unwrap();

        let error = memory.supersede(&request, false, None).unwrap_err();
        assert!(
            matches!(&error, WriteError::NeedsApproval(reason) if reason.contains("confirmed-by") && reason.contains("approval")),
            "{error}"
        );
        // Nothing moved: no replacement, no stamp, the file is intact.
        assert!(!root.join("load-bearing-v2.md").exists());
        let text = fs::read_to_string(root.join("load-bearing.md")).unwrap();
        assert!(!text.contains("superseded-by: load-bearing-v2"), "{text}");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_confirmed_by_supersede_lands_with_approval_and_an_inferred_one_on_its_justification() {
        let root = scratch("provenance-allowed");
        provenance_corpus(&root);
        let stream_root = scratch("provenance-stream");
        let stream = calib::Stream::open(&stream_root);
        let memory = Memory::load_with(&root, Box::new(KeywordOnly));

        // Confirmed-by, with approval: the modify path writes the
        // replacement and stamps the old fact into it.
        let request = Supersede::parse(&serde_json::json!({
            "name": "load-bearing",
            "justification": "the owner restated the rule on 2026-10-13",
            "replacement": "load-bearing-v2",
            "description": "the owner's new standing rule about the gate",
        }))
        .unwrap();
        let outcome = memory.supersede(&request, true, Some(&stream)).unwrap();
        assert_eq!(
            outcome,
            Superseded::By {
                from: "load-bearing".to_string(),
                into: "load-bearing-v2".to_string()
            }
        );
        let replacement = fs::read_to_string(root.join("load-bearing-v2.md")).unwrap();
        // The justification is recorded in the new fact's body, and the
        // replacement is inferred — a supersession is a proposal.
        assert!(
            replacement.contains(
                "Supersedes `load-bearing` \u{2014} the owner restated the rule on 2026-10-13"
            ),
            "{replacement}"
        );
        assert!(replacement.contains("basis: inferred"), "{replacement}");
        let old = fs::read_to_string(root.join("load-bearing.md")).unwrap();
        assert!(old.contains("superseded-by: load-bearing-v2"), "{old}");
        // The superseded fact left the protected tier and the
        // replacement took its place — no fact enters a turn twice.
        let memory = Memory::load_with(&root, Box::new(KeywordOnly));
        let protected: Vec<String> = memory
            .protected()
            .into_iter()
            .map(|hit| hit.fact.name)
            .collect();
        assert_eq!(protected, vec!["load-bearing-v2".to_string()]);

        // Inferred, with its justification: no approval needed, and the
        // justification rides the row and the replacement's body.
        let request = Supersede::parse(&serde_json::json!({
            "name": "a-guess",
            "justification": "the deploy moved to per-branch builds",
            "replacement": "a-better-guess",
            "description": "an inferred fact about the per-branch deploy",
        }))
        .unwrap();
        memory.supersede(&request, false, Some(&stream)).unwrap();
        assert!(root.join("a-better-guess.md").exists());

        // An inferred supersession without its justification is a typed
        // error — the justification is the permission.
        let error = Supersede::parse(&serde_json::json!({
            "name": "a-guess",
            "justification": "   ",
        }))
        .unwrap_err();
        assert!(matches!(&error, WriteError::Justification(_)), "{error}");

        drop(stream);
        let rows = calib::read_day(&stream_root, &calib::today(), 64);
        let supersede_rows: Vec<&calib::ReadRow> = rows
            .iter()
            .filter(|row| row.action == calib::ACTION_SUPERSEDE)
            .collect();
        assert_eq!(supersede_rows.len(), 2, "{rows:?}");
        assert_eq!(supersede_rows[0].fact, "load-bearing");
        assert_eq!(
            supersede_rows[0].reason,
            "the owner restated the rule on 2026-10-13"
        );
        assert_eq!(supersede_rows[1].fact, "a-guess");
        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&stream_root);
    }

    #[test]
    fn the_supersede_forget_path_expires_the_fact_and_notes_why() {
        let root = scratch("provenance-forget");
        provenance_corpus(&root);
        let stream_root = scratch("forget-stream");
        let stream = calib::Stream::open(&stream_root);
        let memory = Memory::load_with(&root, Box::new(KeywordOnly));

        let request = Supersede::parse(&serde_json::json!({
            "name": "a-guess",
            "justification": "the guess turned out to be wrong",
        }))
        .unwrap();
        let outcome = memory.supersede(&request, false, Some(&stream)).unwrap();
        assert_eq!(
            outcome,
            Superseded::Forgotten {
                fact: "a-guess".to_string()
            }
        );
        drop(stream);

        // The file stays — nothing is deleted — with its validity ended
        // the day before the retirement and the reason in its body.
        let text = fs::read_to_string(root.join("a-guess.md")).unwrap();
        let yesterday = days_ago(1);
        assert!(
            text.contains(&format!("valid-until: {yesterday}")),
            "{text}"
        );
        assert!(
            text.contains(&format!(
                "Retired {}: the guess turned out to be wrong",
                calib::today()
            )),
            "{text}"
        );
        // The loader retires it now.
        let memory = Memory::load_with(&root, Box::new(KeywordOnly));
        assert!(memory
            .recall("an inferred guess about the deploy")
            .is_empty());
        // And the row names the forget.
        let rows = calib::read_day(&stream_root, &calib::today(), 64);
        let row = rows.last().expect("the forget row");
        assert_eq!(row.action, calib::ACTION_FORGET);
        assert_eq!(row.fact, "a-guess");
        assert_eq!(row.reason, "the guess turned out to be wrong");
        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&stream_root);
    }

    #[test]
    fn a_modify_whose_stamp_cannot_land_names_what_landed_and_records_no_row() {
        let root = scratch("provenance-stamp-modify");
        provenance_corpus(&root);
        let stream_root = scratch("stamp-modify-stream");
        let stream = calib::Stream::open(&stream_root);
        let memory = Memory::load_with(&root, Box::new(KeywordOnly));
        let request = Supersede::parse(&serde_json::json!({
            "name": "a-guess",
            "justification": "the deploy moved to per-branch builds",
            "replacement": "a-better-guess",
            "description": "an inferred fact about the per-branch deploy",
        }))
        .unwrap();

        // The stamp cannot land: the fact is immutable, so the
        // rewrite's rename is refused while every read still works.
        let fact_path = root.join("a-guess.md");
        set_immutable(&fact_path, true);
        let outcome = memory.supersede(&request, false, Some(&stream));
        set_immutable(&fact_path, false);

        let error = outcome.unwrap_err();
        assert!(
            matches!(&error, WriteError::Stamp(reason) if reason.contains("a-better-guess") && reason.contains("a-guess")),
            "{error}"
        );
        // What landed and what did not, on disk as in the message: the
        // replacement exists, the old fact is not retired.
        assert!(root.join("a-better-guess.md").exists());
        let old = fs::read_to_string(&fact_path).unwrap();
        assert!(!old.contains("superseded-by: a-better-guess"), "{old}");
        // No row: the audit cannot claim a retirement that did not
        // land.
        drop(stream);
        let rows = calib::read_day(&stream_root, &calib::today(), 64);
        assert!(rows.is_empty(), "{rows:?}");
        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&stream_root);
    }

    #[test]
    fn a_forget_whose_stamp_cannot_land_records_no_row() {
        let root = scratch("provenance-stamp-forget");
        provenance_corpus(&root);
        let stream_root = scratch("stamp-forget-stream");
        let stream = calib::Stream::open(&stream_root);
        let memory = Memory::load_with(&root, Box::new(KeywordOnly));
        let request = Supersede::parse(&serde_json::json!({
            "name": "a-guess",
            "justification": "the guess turned out to be wrong",
        }))
        .unwrap();

        let fact_path = root.join("a-guess.md");
        set_immutable(&fact_path, true);
        let outcome = memory.supersede(&request, false, Some(&stream));
        set_immutable(&fact_path, false);

        // The forget's first write is its expiry stamp, so nothing
        // landed at all: no expiry, no note, no row.
        let error = outcome.unwrap_err();
        assert!(
            matches!(&error, WriteError::Stamp(reason) if reason.contains("a-guess")),
            "{error}"
        );
        let text = fs::read_to_string(&fact_path).unwrap();
        let yesterday = days_ago(1);
        assert!(
            !text.contains(&format!("valid-until: {yesterday}")),
            "{text}"
        );
        assert!(!text.contains("Retired "), "{text}");
        drop(stream);
        let rows = calib::read_day(&stream_root, &calib::today(), 64);
        assert!(rows.is_empty(), "{rows:?}");
        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&stream_root);
    }

    #[test]
    fn a_supersede_of_an_unknown_or_retired_fact_is_a_typed_error() {
        let root = scratch("provenance-unknown");
        provenance_corpus(&root);
        let memory = Memory::load_with(&root, Box::new(KeywordOnly));
        let request = Supersede::parse(&serde_json::json!({
            "name": "no-such-fact",
            "justification": "it was never there",
        }))
        .unwrap();
        let error = memory.supersede(&request, true, None).unwrap_err();
        assert!(matches!(&error, WriteError::UnknownFact(_)), "{error}");
        let _ = fs::remove_dir_all(&root);
    }

    // --- the surfaces ---

    #[test]
    fn the_consolidation_surface_is_cards_and_names_an_empty_pass() {
        let rendered = render_consolidation(&Consolidation::default());
        assert!(
            rendered.contains("fleet memories --consolidate"),
            "{rendered}"
        );
        assert!(rendered.contains("nothing to consolidate"), "{rendered}");

        let result = Consolidation {
            actions: vec![
                CycleAction {
                    action: calib::ACTION_COMPRESS,
                    fact: "old-note".to_string(),
                    reason: "working memory older than 7 days".to_string(),
                },
                CycleAction {
                    action: calib::ACTION_FORGET,
                    fact: "expired-durable".to_string(),
                    reason: "expired 2026-10-12".to_string(),
                },
            ],
            untouched: vec!["stale-confirmed".to_string()],
            kept: vec!["stalled-inferred".to_string()],
            summary: Some("consolidation-2026-10-13".to_string()),
        };
        let rendered = render_consolidation(&result);
        assert_eq!(rendered.matches('\u{250c}').count(), 2, "{rendered}");
        assert!(
            rendered.contains("old-note \u{2014} working memory older than 7 days"),
            "{rendered}"
        );
        assert!(
            rendered.contains("expired-durable \u{2014} expired 2026-10-12"),
            "{rendered}"
        );
        assert!(
            rendered.contains("left alone: stale-confirmed"),
            "{rendered}"
        );
        // The third-vector rule names what it kept and why.
        assert!(
            rendered.contains("left alone: stalled-inferred"),
            "{rendered}"
        );
        assert!(
            rendered.contains("incomplete, so the cycle never folds one"),
            "{rendered}"
        );
        assert!(
            rendered.contains("folded into: consolidation-2026-10-13"),
            "{rendered}"
        );
    }

    #[test]
    fn the_archive_surface_lists_what_the_cycle_forgot() {
        let root = scratch("archive-surface");
        let empty = render_archive(&root);
        assert!(empty.contains("the archive is empty"), "{empty}");
        cycle_corpus(&root);
        consolidate(&root, &calib::today(), None);
        let rendered = render_archive(&root);
        assert!(rendered.contains("fleet memories --archive"), "{rendered}");
        assert!(rendered.contains("2 forgotten facts"), "{rendered}");
        assert!(rendered.contains("expired-note"), "{rendered}");
        assert!(rendered.contains("expired-durable"), "{rendered}");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn the_card_names_a_non_durable_tier_and_leaves_a_durable_one_alone() {
        let root = scratch("card-tier");
        write_fact(
            &root,
            "durable-one",
            &good_fact("durable-one", "a durable fact", "project", "deploy"),
        );
        write_fact(
            &root,
            "working-one",
            &working_fact("working-one", "a working fact", &days_ago(0), &days_ago(-1)),
        );
        let memory = Memory::load_with(&root, Box::new(KeywordOnly));
        let rendered = render(&memory, None);
        // The durable card reads exactly as it did before the tier
        // field existed.
        assert!(
            rendered.contains("shape: deploy \u{b7} recalls: 0 \u{b7} valid-until: \u{2014}"),
            "{rendered}"
        );
        // The working one names its tier.
        assert!(rendered.contains("tier: working"), "{rendered}");
        let _ = fs::remove_dir_all(&root);
    }

    // --- the in-place edit ---

    /// One fact file in the writer's own field order.
    fn a_fact(name: &str, description: &str) -> String {
        format!(
            "---\nname: {name}\ndescription: \"{description}\"\ntype: project\ndate: {}\nbasis: inferred\ntier: durable\nshape: []\nphrases: []\nmissing: []\nrecall-count: 0\nsuperseded-by:\nvalid-until:\n---\n\nThe body.\n",
            crate::calib::today()
        )
    }

    /// KI-059: the fork's documented synonyms for the two protected
    /// types map onto them — a hand-written `preference` protects the
    /// same as `feedback`, because the fork documents `type` as
    /// free-form and its own corpus writes the synonym.
    #[test]
    fn a_documented_synonym_protects_the_same_as_its_type() {
        let root = scratch("protected-synonym");
        write_fact(
            &root,
            "a-pref",
            &a_fact("a-pref", "a standing preference").replace("type: project", "type: preference"),
        );
        let facts = load_facts(&root);
        let fact = facts
            .iter()
            .find(|fact| fact.name == "a-pref")
            .expect("loaded");
        assert!(fact.protected(), "{fact:?}");
        let _ = fs::remove_dir_all(&root);
    }

    /// KI-061: the writer's bound, applied at load. The corpus is
    /// hand-editable by design, so a description edited by hand rides
    /// whatever it was given — the loader applies the same sanitize and
    /// marked cut rather than trusting every writer to be the harness.
    #[test]
    fn a_hand_edited_description_is_bounded_at_load() {
        let root = scratch("description-bound");
        let long = "x".repeat(MAX_DESCRIPTION_CHARS + 40);
        write_fact(&root, "a-long", &a_fact("a-long", &long));
        let facts = load_facts(&root);
        let fact = facts
            .iter()
            .find(|fact| fact.name == "a-long")
            .expect("loaded");
        assert!(
            fact.description.ends_with('\u{2026}'),
            "the cut is marked: {}",
            fact.description.len()
        );
        assert!(
            fact.description.chars().count() <= MAX_DESCRIPTION_CHARS + 1,
            "{}",
            fact.description.chars().count()
        );
        let _ = fs::remove_dir_all(&root);
    }

    /// KI-087: the fleet home's own corpus is checked BEFORE the working
    /// directory's ancestor walk, so a run started outside any intended
    /// corpus cannot adopt a foreign one the walk found further up.
    #[test]
    fn the_homes_own_corpus_wins_over_a_walked_up_foreign_one() {
        let home = scratch("resolve-home");
        let foreign = scratch("resolve-foreign");
        fs::create_dir_all(home.join(MEMORIES_DIR)).unwrap();
        std::fs::write(
            home.join(MEMORIES_DIR).join("a-fact.md"),
            "---\nname: a-fact\n---\n",
        )
        .unwrap();
        fs::create_dir_all(foreign.join("nested").join(MEMORIES_DIR)).unwrap();
        // From inside the foreign tree, the walk would find the foreign
        // corpus; the home's own wins.
        let resolved = resolve_root_from(&foreign.join("nested"), Some(&home));
        assert_eq!(resolved, home.join(MEMORIES_DIR), "{resolved:?}");
        // With no home, the walk stands as it was.
        let walked = resolve_root_from(&foreign.join("nested"), None);
        assert_eq!(
            walked,
            foreign.join("nested").join(MEMORIES_DIR),
            "{walked:?}"
        );
        let _ = fs::remove_dir_all(&home);
        let _ = fs::remove_dir_all(&foreign);
    }

    #[test]
    fn an_edit_rewrites_the_named_fields_and_preserves_the_body() {
        let root = scratch("edit-fields");
        write_fact(&root, "a-fact", &a_fact("a-fact", "the old sentence"));
        let request = FactEdit {
            name: "a-fact".to_string(),
            description: Some("the new sentence".to_string()),
            fact_type: Some("reference".to_string()),
            shape: Some(vec!["docs".to_string()]),
            valid_until: Some("2099-01-01".to_string()),
            ..Default::default()
        };

        let edited = edit(&root, &request).expect("the edit lands");

        assert_eq!(edited.description, "the new sentence");
        assert_eq!(edited.fact_type, "reference");
        assert_eq!(edited.shape, vec!["docs".to_string()]);
        assert_eq!(edited.valid_until, "2099-01-01");
        // The body above all survives: the rewrite owns one line and
        // every other byte is preserved.
        assert_eq!(edited.body, "The body.");
        // The name, the tier, and the basis are not a surface's to move.
        assert_eq!(edited.name, "a-fact");
        assert_eq!(edited.tier, Tier::Durable);
        assert_eq!(edited.basis, Basis::Inferred);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn an_edit_of_the_body_replaces_everything_after_the_fence() {
        let root = scratch("edit-body");
        write_fact(&root, "a-fact", &a_fact("a-fact", "the sentence"));
        let request = FactEdit {
            name: "a-fact".to_string(),
            body: Some("A whole new body.\n".to_string()),
            ..Default::default()
        };

        let edited = edit(&root, &request).expect("the edit lands");

        // The body reads back as the loader normalizes it: one trailing
        // newline is the file's own, not the fact's.
        assert_eq!(edited.body, "A whole new body.");
        assert_eq!(edited.description, "the sentence");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn an_edit_that_asks_for_nothing_lands_no_write() {
        let root = scratch("edit-empty");
        let path = write_fact(&root, "a-fact", &a_fact("a-fact", "the sentence"));
        let before = fs::read_to_string(&path).unwrap();

        let edited = edit(
            &root,
            &FactEdit {
                name: "a-fact".to_string(),
                ..Default::default()
            },
        )
        .expect("an edit with no fields is a no-op, not an error");

        assert_eq!(fs::read_to_string(&path).unwrap(), before);
        assert_eq!(edited.description, "the sentence");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn an_edit_of_an_unknown_or_retired_fact_is_a_typed_error() {
        let root = scratch("edit-refused");
        write_fact(
            &root,
            "a-folded",
            &a_fact("a-folded", "the folded sentence"),
        );
        let folded = root.join("a-folded.md");
        let mut text = fs::read_to_string(&folded).unwrap();
        text = text.replace("superseded-by:\n", "superseded-by: a-fact\n");
        fs::write(&folded, text).unwrap();

        let unknown = edit(
            &root,
            &FactEdit {
                name: "no-such-fact".to_string(),
                ..Default::default()
            },
        );
        assert!(
            matches!(unknown, Err(WriteError::UnknownFact(_))),
            "{unknown:?}"
        );
        let retired = edit(
            &root,
            &FactEdit {
                name: "a-folded".to_string(),
                description: Some("rewriting a record".to_string()),
                ..Default::default()
            },
        );
        assert!(
            matches!(retired, Err(WriteError::Retired(_))),
            "{retired:?}"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn an_edit_parses_the_writers_own_key_vocabulary_and_bounds() {
        let arguments = serde_json::json!({
            "name": "a-fact",
            "description": "the new sentence",
            "shape": "docs",
            "phrases": ["read the log directly"],
            "missing": ["the log"],
            "valid-until": "2099-01-01",
        });
        let request = FactEdit::parse(&arguments).expect("the arguments parse");
        assert_eq!(request.name, "a-fact");
        assert_eq!(request.shape, Some(vec!["docs".to_string()]));
        assert_eq!(
            request.phrases,
            Some(vec!["read the log directly".to_string()])
        );

        // An absent field is not a cleared one.
        let absent = FactEdit::parse(&serde_json::json!({"name": "a-fact"})).unwrap();
        assert_eq!(absent.description, None);
        assert_eq!(absent.shape, None);

        // An empty description is refused rather than landed.
        let empty = FactEdit::parse(&serde_json::json!({"name": "a-fact", "description": "  "}));
        assert!(
            matches!(empty, Err(WriteError::Description(_))),
            "{empty:?}"
        );
        // A day that is not a day is refused.
        let day = FactEdit::parse(&serde_json::json!({"name": "a-fact", "valid-until": "soon"}));
        assert!(matches!(day, Err(WriteError::ValidUntil(_))), "{day:?}");
    }
}
