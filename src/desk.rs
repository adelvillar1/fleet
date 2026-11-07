//! The desk: where knowledge work happens. One desk project is one JSON
//! document at `<home>/desk/<slug>/project.json` — canvas's law
//! (`src/canvas.rs`), restated for the loop the plan
//! (`docs/plans/2026-11-04-desk.md`) exists to close: a brief, the
//! sources it names with their provenance, the verified claim set, the
//! review verdicts, and the digest that makes the project knowledge.
//!
//! Every desk project binds to a workspace project and its folder — the
//! board's binding law, the key the corpus stores and the assembled
//! context read by. The document law lives here too: the
//! unsourced-assertion refusal, the check prose never had. A deliverable
//! may not assert what no source sentence carries; a near miss is named,
//! never swallowed.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// The directory under the fleet home holding every desk project.
pub const DESK_ROOT: &str = "desk";

/// The directory of digests, under the desk root — the K6 shape: one
/// subdirectory per project key, one file per deliverable, the bounded
/// account a run born in the project reads in assembled context.
pub const DIGESTS_DIR: &str = "digests";

/// The document law's match floor: an assertion matches a verified
/// claim when this share of the assertion's tokens the claim (and its
/// sentence) carry. A starting point named like the extraction floors —
/// the calibration stream is where it gets fit, never a prompt.
pub const ASSERTION_MATCH_FLOOR: f64 = 0.6;

/// The assembled-context section's bound: one line per deliverable,
/// newest first, this many — the design account's own shape
/// (`src/canvas.rs`, K6). An unbounded section would spend the budget
/// of exactly the project that uses the desk hardest.
pub const DESK_CONTEXT_MAX: usize = 8;

/// Where a source's text came from, and what it means for provenance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SourceKind {
    /// A local file, read through the documents routing — the path is
    /// the provenance.
    File,
    /// A memory fact — the fact's name is the provenance.
    Fact,
    /// A URL captured by the run's `web` tool into the desk project's
    /// sources directory. The capture is a run action under the
    /// approval gate; the extraction spine never does network itself.
    Url,
}

/// One source the brief names, with its provenance carried, not implied.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Source {
    #[serde(rename = "type")]
    pub kind: SourceKind,
    /// The local path (file), the fact's name (fact), or the captured
    /// file's path under the desk project (url).
    pub path: String,
    /// For a captured URL: the address it came from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// For a captured URL: when the capture happened (RFC 3339).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fetched_at: Option<String>,
    /// For a captured URL: the page's title at capture time.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

/// The verdict the extraction floors handed a claim: asserted at
/// p ≥ 0.85, vetoed at p ≤ 0.35, contested between (marked, never
/// dropped — the jobhound split, `src/extract.rs`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ClaimVerdict {
    Asserted,
    Vetoed,
    Contested,
}

/// What a verified segment IS — the somewhere-to-put-it labels
/// (jobhound's measured lesson: without them, background becomes
/// phantom claims).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ClaimKind {
    Claim,
    Support,
    Background,
    Boilerplate,
}

/// One verified claim: the text, where it came from, the sentence it
/// was read from, its confidence, its kind, and its verdict.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Claim {
    pub text: String,
    /// The source's path as the brief carries it.
    pub source: String,
    /// The sentence the claim was read from — the quote every review
    /// renders beside the claim.
    pub sentence: String,
    pub confidence: f64,
    pub kind: ClaimKind,
    pub verdict: ClaimVerdict,
}

/// The stages a desk project walks, in order. The rail shows every
/// stage's state; a refusal stops its stage by name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Stage {
    Claims,
    Outline,
    Render,
    Review,
    Knowledge,
}

pub const STAGES: [Stage; 5] = [
    Stage::Claims,
    Stage::Outline,
    Stage::Render,
    Stage::Review,
    Stage::Knowledge,
];

impl Stage {
    pub fn name(self) -> &'static str {
        match self {
            Stage::Claims => "claims",
            Stage::Outline => "outline",
            Stage::Render => "render",
            Stage::Review => "review",
            Stage::Knowledge => "knowledge",
        }
    }
}

/// One stage's state. A refusal carries its reason — the stage stops by
/// name, and the deliverable cannot end while one stands.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "lowercase")]
pub enum StageState {
    Pending,
    Running,
    Done,
    Refused { reason: String },
}

/// The document law's refusal: the deliverable asserted what no source
/// sentence carries. The near miss is named — the closest verified
/// sentence and where it lives — so the owner judges, and the
/// paraphrase limitation is stated at the definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnsourcedAssertion {
    /// The assertion the draft made.
    pub assertion: String,
    /// The closest verified sentence, verbatim, or empty when the claim
    /// set is empty (there was nothing to be near).
    pub near_miss: String,
    /// Where the near miss came from.
    pub near_miss_source: String,
    /// The near miss's overlap share — the number that fell under the
    /// floor, so the refusal is auditable.
    pub overlap: f64,
}

/// The coverage beat's verdict as the store carries it — the parsed
/// answer's durable shape.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CoverageRecord {
    pub score: f64,
    pub material_gap: bool,
    pub off_brief: bool,
    pub escalated: bool,
    pub reason: String,
}

/// The desk project: one JSON document, the whole loop's state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeskProject {
    pub slug: String,
    pub name: String,
    /// Who the deliverable is for.
    pub audience: String,
    /// The claim the brief asks the deliverable to make.
    pub claim: String,
    pub sources: Vec<Source>,
    /// The problem shape the brief routed under — memory's keyword
    /// rule, one vocabulary — and what selects the declared methodology
    /// skills. `None` when nothing routed: the undefined-shape case,
    /// and discovery falls to meaning.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shape: Option<String>,
    /// The bound workspace project folder — the board's binding law,
    /// an absolute path when one was named. Unbound desk work is
    /// allowed but second-class.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        alias = "projectPath"
    )]
    pub project: Option<String>,
    /// The stages' states, in walk order.
    pub stages: Vec<(Stage, StageState)>,
    /// The verified claim set the extraction spine produced.
    pub claims: Vec<Claim>,
    /// The document law's refusals. The deliverable cannot end while
    /// one stands.
    pub refusals: Vec<UnsourcedAssertion>,
    /// The computed figures the tabular leg answered — computed, never
    /// generated, and part of the number guard's givens.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub figures: Vec<Figure>,
    /// The coverage beat's verdict, once it ran. An escalated verdict is
    /// stored as it answered: unresolved, with its reason.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coverage: Option<CoverageRecord>,
    /// The outline the composing lane answered, before render target
    /// selection.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outline: Option<String>,
    /// The rendered draft — the document target's markdown.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub draft: Option<String>,
    /// The number guard's refusals: metric numbers the draft carried
    /// that no given answers. The end gate counts these beside the
    /// document law's refusals.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub number_refusals: Vec<UnsourcedNumber>,
    /// The digest's path under the home, once one was written.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub digest: Option<String>,
    pub created: String,
    pub updated: String,
}

impl DeskProject {
    /// The stage's current state, or pending when unmarked.
    pub fn stage(&self, stage: Stage) -> StageState {
        self.stages
            .iter()
            .find(|(s, _)| *s == stage)
            .map(|(_, state)| state.clone())
            .unwrap_or(StageState::Pending)
    }

    /// The evidence status the shelf rows carry: open refusals and the
    /// contested count, read off the store.
    pub fn evidence_status(&self) -> (usize, usize) {
        (
            self.refusals.len(),
            self.claims
                .iter()
                .filter(|c| c.verdict == ClaimVerdict::Contested)
                .count(),
        )
    }
}

/// The desk root under the home.
pub fn desk_root(home: &Path) -> PathBuf {
    home.join(DESK_ROOT)
}

/// One desk project's document path — the law the canvas restated:
/// `<home>/desk/<slug>/project.json`.
pub fn project_path(home: &Path, slug: &str) -> PathBuf {
    desk_root(home).join(slug).join("project.json")
}

/// The digest path for a bound project key — one file per deliverable.
pub fn digest_path(home: &Path, key: &str, slug: &str) -> PathBuf {
    desk_root(home)
        .join(DIGESTS_DIR)
        .join(key)
        .join(format!("{slug}.md"))
}

/// Whether a desk project exists.
pub fn exists(home: &Path, slug: &str) -> bool {
    project_path(home, slug).is_file()
}

/// Create one desk project, refusing an existing slug by name — the
/// writer's own collision rule; a modify path is a supersede, never an
/// overwrite.
pub fn create(
    home: &Path,
    slug: &str,
    name: &str,
    audience: &str,
    claim: &str,
    sources: Vec<Source>,
    project: Option<String>,
) -> Result<DeskProject, String> {
    if exists(home, slug) {
        return Err(format!(
            "a desk project named {slug} already exists; the modify path is a supersede, not a silent overwrite"
        ));
    }
    let now = crate::calib::stamp();
    let desk = DeskProject {
        slug: slug.to_string(),
        shape: crate::memory::classify_shape(&format!("{audience} {claim}")).map(str::to_string),
        name: name.to_string(),
        audience: audience.to_string(),
        claim: claim.to_string(),
        sources,
        project,
        stages: STAGES.map(|s| (s, StageState::Pending)).to_vec(),
        claims: Vec::new(),
        refusals: Vec::new(),
        figures: Vec::new(),
        coverage: None,
        outline: None,
        draft: None,
        number_refusals: Vec::new(),
        digest: None,
        created: now.clone(),
        updated: now,
    };
    save(home, &desk)?;
    Ok(desk)
}

/// Write one desk project, atomically: a unique temp beside it, then a
/// rename, so a reader never sees half a document. A bound project's
/// digest is regenerated here — the one funnel every write path
/// crosses, so no path can move the document without refreshing its
/// knowledge account.
pub fn save(home: &Path, desk: &DeskProject) -> Result<(), String> {
    let path = project_path(home, &desk.slug);
    let dir = path
        .parent()
        .ok_or_else(|| "the project path has no directory".to_string())?;
    std::fs::create_dir_all(dir)
        .map_err(|error| format!("could not make {}: {error}", dir.display()))?;
    let text = serde_json::to_string_pretty(desk)
        .map_err(|error| format!("the desk project does not serialize: {error}"))?;
    crate::settings::write_atomic(&path, &text)?;
    refresh_digest(home, desk)
}

/// Read one desk project.
pub fn load(home: &Path, slug: &str) -> Result<DeskProject, String> {
    let path = project_path(home, slug);
    let text = std::fs::read_to_string(&path)
        .map_err(|error| format!("could not read {slug}: {error}"))?;
    serde_json::from_str(&text).map_err(|error| format!("{slug} does not parse: {error}"))
}

/// Delete one desk project and its digest (the clearing op knows the
/// binding; callers that move a binding remove the old digest first).
pub fn delete_project(home: &Path, slug: &str) -> Result<(), String> {
    let desk = load(home, slug)?;
    let dir = desk_root(home).join(slug);
    match std::fs::remove_dir_all(&dir) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("could not remove {slug}: {error}")),
    }?;
    if let Some(key) = desk.project.as_deref().and_then(crate::canvas::project_key) {
        match std::fs::remove_file(digest_path(home, &key, slug)) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(format!("could not remove {slug}'s digest: {error}")),
        }
    } else {
        Ok(())
    }
}

/// Every desk project under the home, newest first.
pub fn projects(home: &Path) -> Vec<DeskProject> {
    let mut all: Vec<DeskProject> = list_slugs(home)
        .into_iter()
        .filter_map(|slug| load(home, &slug).ok())
        .collect();
    all.sort_by(|a, b| b.updated.cmp(&a.updated));
    all
}

/// One shelf row's state, by slug: the project itself, or the slug and
/// the reason it could not be read. A corrupt project is a named row on
/// the shelf, never a silent absence — `projects()` folds it away with
/// `.ok()` (KI-437), so the face reads the shelf through here and prints
/// the failure beside the deliverable that should be there.
pub fn project_rows(home: &Path) -> Vec<Result<DeskProject, (String, String)>> {
    let mut rows: Vec<Result<DeskProject, (String, String)>> = list_slugs(home)
        .into_iter()
        .map(|slug| load(home, &slug).map_err(|error| (slug, error)))
        .collect();
    // Newest first for the rows that can date themselves; a row that
    // cannot keeps the slug order the listing walked, and sorts after
    // the readable ones so a broken project never pushes real work down.
    rows.sort_by(|a, b| match (a, b) {
        (Ok(left), Ok(right)) => right.updated.cmp(&left.updated),
        (Ok(_), Err(_)) => std::cmp::Ordering::Less,
        (Err(_), Ok(_)) => std::cmp::Ordering::Greater,
        (Err(left), Err(right)) => left.0.cmp(&right.0),
    });
    rows
}

/// The slugs on disk, sorted — the listing walks the store, never a
/// side file.
pub fn list_slugs(home: &Path) -> Vec<String> {
    let mut slugs: Vec<String> = std::fs::read_dir(desk_root(home))
        .into_iter()
        .flatten()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().is_dir())
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| project_path(home, name).is_file())
        .collect();
    slugs.sort();
    slugs
}

// --- the document law --------------------------------------------------------
//
// The check prose never had: an assertion must stand on a verified
// claim. The claim set is the spine's output; the law matches the
// assertion against what the sources actually carried, and a refusal
// names its nearest miss. The paraphrase limitation is stated here:
// the match is lexical containment — a claim reworded past the floor
// refuses, and the near miss is what the owner reads to judge.

/// One assertion's tokens, lowercased, punctuation stripped — the same
/// normalization the lexical fallback ranks with.
fn tokens(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|t| t.len() > 1)
        .map(|t| t.to_lowercase())
        .collect()
}

/// The share of `assertion`'s tokens `candidate` carries.
fn overlap_share(assertion: &[String], candidate: &[String]) -> f64 {
    if assertion.is_empty() {
        return 0.0;
    }
    let hits = assertion.iter().filter(|t| candidate.contains(t)).count();
    hits as f64 / assertion.len() as f64
}

/// Check one assertion against the verified claim set: `Ok` with the
/// claim it stands on, or `Err` with the refusal naming the near miss.
pub fn check_assertion(assertion: &str, claims: &[Claim]) -> Result<Claim, UnsourcedAssertion> {
    let assertion_tokens = tokens(assertion);
    let mut best: Option<(f64, &Claim)> = None;
    for claim in claims {
        if claim.verdict == ClaimVerdict::Vetoed {
            continue;
        }
        let mut candidate = tokens(&claim.text);
        candidate.extend(tokens(&claim.sentence));
        let share = overlap_share(&assertion_tokens, &candidate);
        if share > best.as_ref().map_or(0.0, |(s, _)| *s) {
            best = Some((share, claim));
        }
    }
    match best {
        Some((share, claim)) if share >= ASSERTION_MATCH_FLOOR => Ok(claim.clone()),
        Some((share, claim)) => Err(UnsourcedAssertion {
            assertion: assertion.to_string(),
            near_miss: claim.sentence.clone(),
            near_miss_source: claim.source.clone(),
            overlap: share,
        }),
        None => Err(UnsourcedAssertion {
            assertion: assertion.to_string(),
            near_miss: String::new(),
            near_miss_source: String::new(),
            overlap: 0.0,
        }),
    }
}

/// Check a whole draft: every assertion's outcome, in order.
pub fn check_draft(
    assertions: &[String],
    claims: &[Claim],
) -> Vec<Result<Claim, UnsourcedAssertion>> {
    assertions
        .iter()
        .map(|assertion| check_assertion(assertion, claims))
        .collect()
}

// --- the digest --------------------------------------------------------------
//
// A bound desk project is project knowledge (the owner's 2026-11-04
// ruling): the deliverable keeps a deterministic markdown account of
// itself at a project-keyed path — the unit the assembled-context read
// serves and the corpus-refresh globs embed.

/// The digest's markdown: deterministic, bounded, one screen.
pub fn digest_markdown(desk: &DeskProject) -> String {
    let mut out = String::new();
    out.push_str(&format!("# {}\n\n", desk.name));
    out.push_str(&format!("deliverable: {}\n", desk.slug));
    out.push_str(&format!(
        "project: {}\n",
        desk.project.as_deref().unwrap_or("\u{2014}")
    ));
    out.push_str(&format!("updated: {}\n", desk.updated));
    out.push_str(&format!("claim: {}\n", desk.claim));
    let (refusals, contested) = desk.evidence_status();
    let asserted = desk
        .claims
        .iter()
        .filter(|c| c.verdict == ClaimVerdict::Asserted)
        .count();
    out.push_str(&format!(
        "claims: {} asserted, {contested} contested, {refusals} open refusal(s)\n",
        asserted
    ));
    out.push_str("\n## Stages\n\n");
    for (stage, state) in &desk.stages {
        let mark = match state {
            StageState::Pending => "\u{2014}",
            StageState::Running => "running",
            StageState::Done => "done",
            StageState::Refused { .. } => "refused",
        };
        out.push_str(&format!("- {} \u{b7} {mark}\n", stage.name()));
    }
    out
}

/// Regenerate a bound project's digest — the save funnel calls this, so
/// no write path can move the document without refreshing its knowledge
/// account. Unbound desk work keeps no digest (nothing keys it).
fn refresh_digest(home: &Path, desk: &DeskProject) -> Result<(), String> {
    let Some(key) = desk.project.as_deref().and_then(crate::canvas::project_key) else {
        return Ok(());
    };
    let path = digest_path(home, &key, &desk.slug);
    let dir = path
        .parent()
        .ok_or_else(|| "the digest path has no directory".to_string())?;
    std::fs::create_dir_all(dir)
        .map_err(|error| format!("could not make {}: {error}", dir.display()))?;
    crate::settings::write_atomic(&path, &digest_markdown(desk))
}

/// Remove one digest at the key the caller names — the clearing op and
/// the delete path know the old binding; `save` never guesses.
pub fn remove_digest(home: &Path, key: &str, slug: &str) -> Result<(), String> {
    match std::fs::remove_file(digest_path(home, key, slug)) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("could not remove the digest: {error}")),
    }
}

/// K6's shape, applied to knowledge work: the bounded desk account a
/// run born in a project carries in its assembled context — one line
/// per deliverable from the project's digests, newest first, absent
/// when the project has none. The digest is the knowledge; this is its
/// read.
pub fn desk_context_section(home: &Path, key: &str) -> Option<String> {
    let dir = desk_root(home).join(DIGESTS_DIR).join(key);
    let mut entries: Vec<(std::time::SystemTime, PathBuf)> = std::fs::read_dir(&dir)
        .ok()?
        .flatten()
        .filter(|entry| {
            entry
                .path()
                .extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| ext == "md")
        })
        .filter_map(|entry| {
            let stamp = entry.metadata().ok()?.modified().ok()?;
            Some((stamp, entry.path()))
        })
        .collect();
    entries.sort_by_key(|(stamp, _)| std::cmp::Reverse(*stamp));
    let mut lines: Vec<String> = Vec::new();
    for (_, path) in entries.iter().take(DESK_CONTEXT_MAX) {
        let text = std::fs::read_to_string(path).ok()?;
        let name = text.lines().find_map(|line| line.strip_prefix("# "))?;
        let claims = text
            .lines()
            .find_map(|line| line.strip_prefix("claims: "))?
            .to_string();
        lines.push(format!("- {name} \u{b7} {claims}"));
    }
    if lines.is_empty() {
        return None;
    }
    Some(format!("## Desk\n\n{}", lines.join("\n")))
}

// --- the corpus leg ----------------------------------------------------------
//
// The digest is the knowledge; the corpus is where it embeds. A bound
// desk project seeds its digest into the project's blend1 corpus by the
// lane's own `seed` verb — idempotent by key, the next brief recalls it
// through `search`. Unbound desk work names the skip: nothing keys it.

/// The blend1 verb the corpus leg calls.
pub const BLEND1_SEED_TOOL: &str = "seed";

/// Seed one bound desk project's digest into its project corpus. The
/// key is the desk path and the deliverable — the provenance the
/// search answers carry. An unbound project is a named skip: the corpus
/// keys by project, and unbound desk work has none.
pub async fn seed_corpus(
    source: &crate::lanes::LaneSource,
    desk: &DeskProject,
) -> Result<(), String> {
    let Some(key) = desk.project.as_deref().and_then(crate::canvas::project_key) else {
        return Err(format!(
            "{} is unbound — a desk project seeds its corpus only when a project keys it",
            desk.slug
        ));
    };
    let spec = crate::judge::lane_spec(source, "blend1")?;
    let server = crate::mcp::McpServer::spawn(&spec)
        .map_err(|error| format!("the composed lane could not start: {error}"))?;
    let result = async {
        server
            .initialize()
            .await
            .map_err(|error| format!("the composed lane's handshake failed: {error}"))?;
        let tools = server
            .list_tools()
            .await
            .map_err(|error| format!("the composed lane's tool list failed: {error}"))?;
        if !tools
            .iter()
            .any(|declared| declared.name == BLEND1_SEED_TOOL)
        {
            return Err(format!(
                "lane `blend1` declares no `{BLEND1_SEED_TOOL}` tool — the corpus leg cannot seed"
            ));
        }
        let arguments = serde_json::json!({
            "project": key,
            "items": [{
                "key": format!("desk/{}/{}", desk.slug, desk.name),
                "text": digest_markdown(desk),
            }],
        });
        let call: crate::mcp::CallResult = server
            .call_tool(BLEND1_SEED_TOOL, arguments)
            .await
            .map_err(|error| format!("the composed lane refused the seed: {error}"))?;
        if call.is_error {
            return Err(format!(
                "the composed lane refused the seed: {}",
                call.text()
            ));
        }
        Ok(())
    };
    let outcome = result.await;
    server.close().await.ok();
    outcome
}

// --- the number leg ------------------------------------------------------

// --- the claim set -----------------------------------------------------------
//
// The spine's shape, applied to a source: segment (pure Rust), narrow
// (one batched call per chunk when the source is long), verify
// (per carrying segment), reconcile (the measured floors). Every claim
// carries its source and its sentence; a declined answer reads as a
// veto at 0.0 — a named absence, never a fabricated claim.

use crate::extract::{ExtractError, ExtractSession, Segment};

/// The batched narrowing's floor. A probability over many segments in
/// one state rides lower than the per-segment margin's — the jobhound
/// split (0.70 for the batched classifier, 0.85 for the per-text one).
pub const CLAIM_BATCH_FLOOR: f64 = 0.70;

/// Segments per batched narrowing call — the measured fan-out cap:
/// glide times out on larger multi-head requests, and sys1's own
/// plansurface chunks at 14 for exactly this reason.
pub const CLAIM_BATCH_CHUNK: usize = 14;

/// The choice head's labels, in asking order — the
/// somewhere-to-put-it labels (jobhound's measured lesson: without
/// them, background becomes phantom claims).
pub const CLAIM_KINDS: [&str; 4] = ["claim", "support", "background", "boilerplate"];

/// One choice head's probabilities, by label. A missing or malformed
/// answer is `None` — the named absence, and the caller reads it as a
/// decline rather than inventing a number.
fn choice_probabilities(answers: &serde_json::Value, id: &str) -> Option<Vec<(String, f64)>> {
    let probabilities = answers.get(id)?.get("probabilities")?.as_object()?;
    let mut pairs: Vec<(String, f64)> = probabilities
        .iter()
        .map(|(label, p)| (label.clone(), p.as_f64().unwrap_or(0.0)))
        .collect();
    pairs.sort_by(|a, b| b.1.total_cmp(&a.1));
    Some(pairs)
}

fn probability_of(pairs: &[(String, f64)], label: &str) -> f64 {
    pairs
        .iter()
        .find(|(name, _)| name == label)
        .map_or(0.0, |(_, p)| *p)
}

/// The batched narrowing: which of a long source's segments carry a
/// claim worth a call of their own. One call per chunk of
/// `CLAIM_BATCH_CHUNK` segments — the state renders the chunk numbered,
/// one noul head per segment — and a head under the batched floor drops
/// its segment by name.
async fn narrow_claims(
    session: &ExtractSession,
    segments: &[Segment],
) -> Result<Vec<usize>, ExtractError> {
    let mut kept: Vec<(usize, f64)> = Vec::new();
    for (chunk_at, chunk) in segments.chunks(CLAIM_BATCH_CHUNK).enumerate() {
        let state = chunk
            .iter()
            .enumerate()
            .map(|(i, segment)| {
                format!(
                    "[SEGMENT {}] {}",
                    chunk_at * CLAIM_BATCH_CHUNK + i,
                    segment.text
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n");
        let questions: Vec<serde_json::Value> = (0..chunk.len())
            .map(|i| {
                serde_json::json!({
                    "id": format!("seg-{i}"),
                    "kind": "noul",
                    "task": "Does this segment state a claim a deliverable could stand on, with its evidence in the segment's own words?",
                    "descriptions": {
                        "true": "Yes, the segment states such a claim.",
                        "false": "No, it is support, background, or boilerplate.",
                    },
                })
            })
            .collect();
        let (answers, provider, _chain) = session
            .answers(&state, serde_json::Value::Array(questions), None, "narrow")
            .await?;
        let answers = crate::extract::provider_map(&answers, &provider);
        for i in 0..chunk.len() {
            let p = answers
                .get(format!("seg-{i}").as_str())
                .and_then(crate::extract::p_true)
                .unwrap_or(0.0);
            if p >= CLAIM_BATCH_FLOOR {
                kept.push((chunk_at * CLAIM_BATCH_CHUNK + i, p));
            }
        }
    }
    kept.sort_by(|a, b| b.1.total_cmp(&a.1));
    kept.truncate(crate::extract::MAX_CARRYING_SEGMENTS);
    kept.sort_by_key(|(at, _)| *at);
    Ok(kept.into_iter().map(|(at, _)| at).collect())
}

/// The claim pass over one source's text. The floors are the
/// extraction leg's own (`ACCEPT_FLOOR` / `VETO_FLOOR`); the kind is
/// the choice head's argmax, so background and boilerplate land
/// somewhere honest instead of passing as claims.
pub async fn claims_from_source(
    session: &ExtractSession,
    source: &str,
    text: &str,
) -> Result<Vec<Claim>, ExtractError> {
    let segments = crate::extract::segments(text);
    let carrying: Vec<usize> = if segments.len() > crate::extract::MAX_CARRYING_SEGMENTS {
        narrow_claims(session, &segments).await?
    } else {
        (0..segments.len()).collect()
    };
    let mut claims = Vec::new();
    for at in carrying {
        let segment = &segments[at];
        // The lane's choice head: one label per carrying segment, so
        // background and boilerplate land somewhere honest.
        let head = serde_json::json!([{
            "id": "kind",
            "kind": "choice",
            "task": "What is this text? One label.",
            "labels": ["claim", "support", "background", "boilerplate"],
            "descriptions": {
                "claim": "A statement a deliverable could stand on, with its evidence in the text.",
                "support": "Evidence for a claim stated elsewhere, not itself the claim.",
                "background": "Context that carries no claim.",
                "boilerplate": "Furniture: company blurb, footers, disclaimers.",
            },
        }]);
        let (answers, provider, _chain) =
            session.answers(&segment.text, head, None, "claims").await?;
        let answers = crate::extract::provider_map(&answers, &provider);
        let pairs = choice_probabilities(&answers, "kind").unwrap_or_default();
        let (kind_label, _) = pairs
            .first()
            .cloned()
            .unwrap_or_else(|| ("background".to_string(), 0.0));
        let kind = match kind_label.as_str() {
            "support" => ClaimKind::Support,
            "boilerplate" => ClaimKind::Boilerplate,
            "claim" => ClaimKind::Claim,
            _ => ClaimKind::Background,
        };
        let p_claim = probability_of(&pairs, "claim");
        let verdict = if p_claim >= crate::extract::ACCEPT_FLOOR {
            ClaimVerdict::Asserted
        } else if p_claim <= crate::extract::VETO_FLOOR {
            ClaimVerdict::Vetoed
        } else {
            ClaimVerdict::Contested
        };
        claims.push(Claim {
            text: segment.text.clone(),
            source: source.to_string(),
            sentence: segment.text.clone(),
            confidence: p_claim,
            kind,
            verdict,
        });
    }
    Ok(claims)
}

// --- the coverage beat -------------------------------------------------------
//
// One sys1 judgment over the brief and the claim set: does the evidence
// carry the claim? The vendored `desk_coverage` task carries the heads
// (an anchored coverage score, the material-gap noul, the off-brief
// noul); the ask rides the judge beat's own lane session — one lane
// source, one session, not a second loop — and the spend rides the
// same home-side ledger pattern under the desk's own name.

/// The vendored task the coverage beat asks.
pub const DESK_COVERAGE_TASK: &str = "desk_coverage";

/// The daily cap on the coverage beat's spend, in USD — the judge
/// beat's own shape: a floor under the owner's wallet, not a target.
pub const DESK_COVERAGE_BUDGET_USD_DAY: f64 = 0.05;

/// The per-ask spend a budget check reserves before it asks — the
/// judge beat's five-times rationale, mirrored.
pub const DESK_COVERAGE_ESTIMATED_ASK_USD: f64 = 0.0002;

/// The config seam that can lower the daily budget, never loosen it.
pub const DESK_COVERAGE_BUDGET_ENV: &str = "FLEET_DESK_BUDGET_USD_DAY";

/// The desk's own home-side spend ledger — its own name, so it never
/// shares a line with the judge beat's ledger.
pub const DESK_SPEND_LEDGER: &str = ".fleet-desk-spend.jsonl";

/// The state text's bound inside the ask — the judge beat's evidence
/// bound, mirrored: the claim set is built from bounded fields, so
/// this is a ceiling rather than a truncation point.
pub const COVERAGE_STATE_CHARS: usize = 4_000;

/// The effective daily budget: the constant, lowered by the config
/// seam when the owner set one. A seam value that does not parse, is
/// not finite, or is above the constant is ignored.
pub fn desk_budget_usd_day() -> f64 {
    match std::env::var(DESK_COVERAGE_BUDGET_ENV) {
        Ok(value) => match value.trim().parse::<f64>() {
            Ok(lower)
                if lower.is_finite() && (0.0..DESK_COVERAGE_BUDGET_USD_DAY).contains(&lower) =>
            {
                lower
            }
            _ => DESK_COVERAGE_BUDGET_USD_DAY,
        },
        Err(_) => DESK_COVERAGE_BUDGET_USD_DAY,
    }
}

/// The coverage beat's answer, parsed or named. An escalated verdict
/// means the lane would not assert: the coverage reads as UNRESOLVED —
/// the gap question stays open by name, and no number pretends.
#[derive(Debug, Clone, PartialEq)]
pub struct Coverage {
    /// The anchored 1–5 coverage score the task's legend carries.
    pub score: f64,
    /// A material sub-question of the brief's claim the claim set does
    /// not answer.
    pub material_gap: bool,
    /// Claims the brief did not ask for.
    pub off_brief: bool,
    /// The lane escalated: the judgment did not assert.
    pub escalated: bool,
    /// The provider that answered, as the answer carries it.
    pub provider: Option<String>,
    /// The lane's own reason, for the review surface.
    pub reason: String,
}

/// Parse one `sys1_classify` answer into the beat's coverage. The
/// provider key varies with the lane's routing, so the heads are read
/// under the answer's first provider.
pub fn parse_coverage(answer: &serde_json::Value) -> Result<Coverage, String> {
    let verdict = answer
        .get("verdict")
        .and_then(|v| v.as_str())
        .unwrap_or("error");
    let escalated = verdict != "pass";
    let reason = answer
        .get("reason")
        .and_then(|r| r.as_str())
        .unwrap_or("")
        .to_string();
    let answers = answer
        .get("answers")
        .and_then(|a| a.as_object())
        .ok_or_else(|| "the lane's answer carries no answers map".to_string())?;
    let (provider, heads) = answers
        .iter()
        .next()
        .ok_or_else(|| "the lane's answer carries no provider's heads".to_string())?;
    let heads = heads
        .as_object()
        .ok_or_else(|| "the lane's provider answer is not a head map".to_string())?;
    let score = heads
        .get("coverage")
        .and_then(|h| h.get("score"))
        .and_then(|s| s.as_f64());
    let gap = heads.get("material_gap").and_then(crate::extract::p_true);
    let off = heads.get("off_brief").and_then(crate::extract::p_true);
    if escalated {
        return Ok(Coverage {
            score: score.unwrap_or(0.0),
            material_gap: gap.is_none_or(|p| p >= 0.5),
            off_brief: off.is_none_or(|p| p >= 0.5),
            escalated: true,
            provider: Some(provider.clone()),
            reason,
        });
    }
    let score = score.ok_or_else(|| "the passing answer carries no coverage score".to_string())?;
    Ok(Coverage {
        score,
        material_gap: gap.is_none_or(|p| p >= 0.5),
        off_brief: off.is_none_or(|p| p >= 0.5),
        escalated: false,
        provider: Some(provider.clone()),
        reason,
    })
}

/// The ask's state text: the brief, then the claim set's own account —
/// each carried claim with its confidence and its source, the floors'
/// verdicts included. Bounded at the ceiling; the brief rides first
/// because the judgment grades the claim set AGAINST it.
pub fn coverage_state(desk: &DeskProject) -> String {
    let asserted = desk
        .claims
        .iter()
        .filter(|c| c.verdict == ClaimVerdict::Asserted)
        .count();
    let contested = desk
        .claims
        .iter()
        .filter(|c| c.verdict == ClaimVerdict::Contested)
        .count();
    let vetoed = desk
        .claims
        .iter()
        .filter(|c| c.verdict == ClaimVerdict::Vetoed)
        .count();
    let mut out = String::new();
    out.push_str("BRIEF\n");
    out.push_str(&format!("audience: {}\n", desk.audience));
    out.push_str(&format!("claim: {}\n", desk.claim));
    out.push_str(&format!(
        "\nCLAIM SET\n{asserted} asserted, {contested} contested, {vetoed} vetoed\n"
    ));
    for claim in &desk.claims {
        if claim.verdict == ClaimVerdict::Vetoed {
            continue;
        }
        out.push_str(&format!(
            "- {} (p {:.2}, {}, {})\n",
            claim.text,
            claim.confidence,
            claim.verdict == ClaimVerdict::Asserted,
            claim.source
        ));
    }
    crate::extract::take_chars(&out, COVERAGE_STATE_CHARS).to_string()
}

/// The coverage beat: budget check by name, one bounded ask on the
/// judge beat's lane session, the answer parsed or named, the spend
/// recorded. A refused budget or a failed ask is a named absence — the
/// gap question stays open, and the deliverable's outline renders it as
/// one.
pub async fn coverage_ask(
    source: &crate::lanes::LaneSource,
    home: &Path,
    desk: &DeskProject,
) -> Result<Coverage, String> {
    let today = crate::calib::stamp();
    let today = today.get(..10).unwrap_or(&today).to_string();
    let spent = crate::calib::sidecar_spend(home, DESK_SPEND_LEDGER, &today);
    if spent + DESK_COVERAGE_ESTIMATED_ASK_USD > desk_budget_usd_day() {
        return Err(format!(
            "the desk's coverage beat is over its daily budget: {:.4} spent of {:.2} — the cap is a floor under the wallet, and the ask is refused by name",
            spent,
            desk_budget_usd_day()
        ));
    }
    let mut session = crate::judge::LaneSession::open(
        source,
        "sys1",
        crate::judge::JUDGE_TOOL,
        DESK_COVERAGE_TASK,
    )
    .await?;
    let answer = session.ask(&coverage_state(desk)).await;
    let result = match answer {
        Ok(answer) => parse_coverage(&answer),
        Err(error) => Err(error),
    };
    session.close().await;
    if result.is_ok() {
        crate::calib::record_spend(
            home,
            DESK_SPEND_LEDGER,
            Some(DESK_COVERAGE_ESTIMATED_ASK_USD),
        );
    }
    result
}

// --- the number leg ----------------------------------------------------------
//
// Figures are computed, never generated: a question a table can answer
// goes to sdm1, and every metric-shaped number in the drafted prose
// must appear canonically in the givens — the composed lane's
// deterministic number check, applied at the desk. An invented number
// refuses by name.

/// One computed figure: the question a table answered, the answer, and
/// what computed it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Figure {
    /// The target the table was scored against.
    pub target: String,
    /// The task type the table was scored under.
    pub task_type: String,
    /// The lane's structured answer, verbatim.
    pub answer: serde_json::Value,
    /// What computed it — the lane's name, so a figure never reads as
    /// a model's opinion.
    pub computed_by: String,
}

/// One figure request: the table (labeled rows plus query rows), the
/// target, and the task type.
#[derive(Debug, Clone)]
pub struct FigureRequest {
    pub table: Vec<serde_json::Value>,
    pub target: String,
    pub task_type: String,
}

/// The sdm1 tool the figure leg calls.
pub const SDM1_TOOL: &str = "sdm1_classify";

/// Parse one `sdm1_classify` answer's text block into a figure. The
/// lane answers in-band (`verdict` carried, never a raise); a verdict
/// of `error` is a named absence, not a number.
pub fn parse_figure(text: &str, request: &FigureRequest) -> Result<Figure, String> {
    let answer: serde_json::Value = serde_json::from_str(text).map_err(|error| {
        format!("the tabular lane's answer was not structured content: {error}")
    })?;
    if answer.get("verdict").and_then(|v| v.as_str()) == Some("error") {
        let reason = answer
            .get("summary")
            .and_then(|s| s.as_str())
            .unwrap_or("the tabular lane answered error without a reason");
        return Err(format!("the tabular lane refused the figure: {reason}"));
    }
    Ok(Figure {
        target: request.target.clone(),
        task_type: request.task_type.clone(),
        answer,
        computed_by: "sdm1".to_string(),
    })
}

/// The figure leg: one `sdm1_classify` call on the lane the caller
/// names — the judge beats' lane-source seam, resolved the same way.
/// Open once, ask once, close; every failure is a named absence.
pub async fn figure_ask(
    source: &crate::lanes::LaneSource,
    request: &FigureRequest,
) -> Result<Figure, String> {
    let spec = crate::judge::lane_spec(source, "sdm1")?;
    let server = crate::mcp::McpServer::spawn(&spec)
        .map_err(|error| format!("the tabular lane could not start: {error}"))?;
    let result = async {
        server
            .initialize()
            .await
            .map_err(|error| format!("the tabular lane's handshake failed: {error}"))?;
        let tools = server
            .list_tools()
            .await
            .map_err(|error| format!("the tabular lane's tool list failed: {error}"))?;
        if !tools.iter().any(|declared| declared.name == SDM1_TOOL) {
            return Err(format!(
                "lane `sdm1` declares no `{SDM1_TOOL}` tool — it cannot answer the figure leg's ask"
            ));
        }
        let arguments = serde_json::json!({
            "table": request.table,
            "target": request.target,
            "task_type": request.task_type,
        });
        let call: crate::mcp::CallResult = server
            .call_tool(SDM1_TOOL, arguments)
            .await
            .map_err(|error| format!("the tabular lane refused the figure: {error}"))?;
        if call.is_error {
            return Err(format!(
                "the tabular lane refused the figure: {}",
                call.text()
            ));
        }
        parse_figure(call.text().as_str(), request)
    };
    let outcome = result.await;
    server.close().await.ok();
    outcome
}

/// One metric-shaped number the draft carried that no given answers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnsourcedNumber {
    /// The number as the draft wrote it.
    pub raw: String,
    /// Its canonical value — what was matched, and failed.
    pub canonical: f64,
}

/// Canonicalize one metric token: separators and currency stripped, a
/// percent read as its probability (92.4% == 0.924), the value as f64.
fn canonical_number(raw: &str) -> Option<f64> {
    let mut token = raw.trim();
    let percent = token.ends_with('%');
    if percent {
        token = &token[..token.len() - 1];
    }
    let token = token.trim_start_matches('$').replace(',', "");
    if token.is_empty() {
        return None;
    }
    let value = token.parse::<f64>().ok()?;
    Some(if percent { value / 100.0 } else { value })
}

/// Every metric-shaped number in a draft, in order: a number token with
/// optional currency and an optional percent — the shapes a deliverable
/// asserts with. Plain digits inside identifiers do not count.
pub fn metric_numbers(draft: &str) -> Vec<(String, f64)> {
    let mut found = Vec::new();
    let bytes = draft.as_bytes();
    let mut at = 0;
    while at < bytes.len() {
        let rest = &draft[at..];
        let mut end = 0;
        let mut seen_digit = false;
        for (i, ch) in rest.char_indices() {
            if ch.is_ascii_digit() {
                seen_digit = true;
                end = i + ch.len_utf8();
            } else if ch == ',' || ch == '.' || ch == '$' {
                // Only while a number is forming: a second dot ends it.
                if ch == '.' && rest[i + 1..].starts_with(|c: char| !c.is_ascii_digit()) {
                    break;
                }
                if seen_digit || ch == '$' {
                    end = i + ch.len_utf8();
                } else {
                    break;
                }
            } else if ch == '%' {
                end = i + ch.len_utf8();
                break;
            } else {
                break;
            }
        }
        if seen_digit && end > 0 {
            let raw = &rest[..end];
            if let Some(value) = canonical_number(raw) {
                found.push((raw.to_string(), value));
            }
            at += end;
        } else {
            // No number formed here: step one CHARACTER, not one byte —
            // a byte step lands inside a multi-byte glyph (an em dash in
            // a real draft) and the slice above panics.
            at += rest.chars().next().map_or(1, char::len_utf8);
        }
    }
    found
}

/// The guard: every metric-shaped number in the draft must appear
/// canonically in the givens — the claim set's sentences and the
/// computed figures both feed it. Returns the refusals; an empty vec is
/// a draft whose numbers all stand.
pub fn check_numbers(draft: &str, givens: &[f64]) -> Vec<UnsourcedNumber> {
    // The outline's list ordinals are structure, not assertions: a
    // line-leading `N.` marker leaves the draft before the scan, while
    // every other number in the line is a claim the givens must carry.
    let scan = draft
        .lines()
        .map(|line| {
            let trimmed = line.trim_start();
            let marker = trimmed
                .split_once('.')
                .filter(|(head, tail)| {
                    !head.is_empty()
                        && head.chars().all(|c| c.is_ascii_digit())
                        && tail.starts_with(' ')
                })
                .map_or(0, |(head, _)| head.len() + 2);
            let prefix = line.len() - trimmed.len();
            format!("{}{}", &line[..prefix], &trimmed[marker..])
        })
        .collect::<Vec<_>>()
        .join("\n");
    metric_numbers(&scan)
        .into_iter()
        .filter(|(_, value)| {
            !givens
                .iter()
                .any(|given| (*given - value).abs() <= 1e-6 * value.abs().max(1.0))
        })
        .map(|(raw, canonical)| UnsourcedNumber { raw, canonical })
        .collect()
}

/// The givens one desk project carries: every claim's sentence's and
/// text's metric numbers, the claims' own confidences (the floors
/// computed them — the draft's p annotations are bookkeeping, not
/// assertions), the coverage score, and every computed figure's answer
/// numbers, canonicalized. The outline's numbers are NOT givens — that
/// is the point: a stat the composing lane invented refuses here.
pub fn number_givens(desk: &DeskProject) -> Vec<f64> {
    let mut givens = Vec::new();
    for claim in &desk.claims {
        for (_, value) in metric_numbers(&claim.sentence) {
            givens.push(value);
        }
        for (_, value) in metric_numbers(&claim.text) {
            givens.push(value);
        }
        // The render prints the confidence at two decimals ({:.2}) —
        // the display the reader sees. The guard grades what the
        // reader sees, so the rounded value is a given beside the
        // exact one (the live walk refused its own "p 0.81" because
        // the stored confidence was 0.8087…; the round-trip through
        // the display is part of the evidence).
        givens.push(claim.confidence);
        givens.push((claim.confidence * 100.0).round() / 100.0);
    }
    if let Some(coverage) = &desk.coverage {
        givens.push(coverage.score);
        givens.push((coverage.score * 100.0).round() / 100.0);
    }
    for figure in &desk.figures {
        for (_, value) in metric_numbers(&figure.answer.to_string()) {
            givens.push(value);
        }
    }
    givens
}

// --- the CLI card ------------------------------------------------------------

/// The card width the crate's CLI surfaces draw at.
const CARD_WIDTH: usize = 72;

/// One desk project's card: the name, the binding, the stage marks, the
/// shape, the evidence status — the `fleet desk` listing.
pub fn card(desk: &DeskProject) -> String {
    let line = "\u{2500}".repeat(CARD_WIDTH);
    let mut out = String::new();
    out.push_str(&format!("{}\n", desk.name));
    out.push_str(&format!("{line}\n"));
    out.push_str(&format!("deliverable  {}\n", desk.slug));
    out.push_str(&format!(
        "project      {}\n",
        desk.project.as_deref().unwrap_or("\u{2014} unbound")
    ));
    out.push_str(&format!(
        "shape        {}\n",
        desk.shape.as_deref().unwrap_or("\u{2014} unrouted")
    ));
    out.push_str(&format!("claim        {}\n", desk.claim));
    for (stage, state) in &desk.stages {
        let mark = match state {
            StageState::Pending => "\u{2014}",
            StageState::Running => "running",
            StageState::Done => "done",
            StageState::Refused { reason } => {
                // A short refusal rides its reason; a long one names the
                // state here and the review view carries the words.
                if reason.chars().count() <= CARD_WIDTH - 14 {
                    reason.as_str()
                } else {
                    "refused (the review view names it)"
                }
            }
        };
        out.push_str(&format!("  {:9} {}\n", stage.name(), mark));
    }
    let (refusals, contested) = desk.evidence_status();
    let asserted = desk
        .claims
        .iter()
        .filter(|c| c.verdict == ClaimVerdict::Asserted)
        .count();
    out.push_str(&format!(
        "evidence     {asserted} asserted, {contested} contested, {refusals} open refusal(s), {} figure(s)\n",
        desk.figures.len()
    ));
    out.push_str(&format!("updated      {}\n", desk.updated));
    out
}

/// The `fleet desk` listing: every project's card, newest first, or the
/// named absence when the desk is empty.
pub fn cards(home: &Path) -> String {
    let projects = projects(home);
    if projects.is_empty() {
        return "the desk holds no deliverables yet \u{2014} a brief starts one\n".to_string();
    }
    projects.iter().map(card).collect()
}

// --- the stage runner --------------------------------------------------------
//
// The loop's orchestration: claims → coverage → outline → render →
// review → knowledge, walking the store's own stages and marking each
// transition as it lands — a stage that fails stops by name, and the
// deliverable cannot end while a refusal stands.

/// The blend1 verb the outline stage asks.
pub const BLEND1_ASK_TOOL: &str = "ask";

/// Mark one stage and save — the one funnel every transition crosses,
/// so the store on disk never disagrees with the walk.
fn mark_stage(
    home: &Path,
    desk: &mut DeskProject,
    stage: Stage,
    state: StageState,
) -> Result<(), String> {
    match desk.stages.iter_mut().find(|(s, _)| *s == stage) {
        Some(entry) => entry.1 = state,
        // A store that predates this stage (a hand-written project, an
        // older document) gains the entry in walk order. A silent no-op
        // here once made a whole walk report nothing: the stage ran, the
        // state had nowhere to land, and the store read `pending`.
        None => {
            desk.stages.push((stage, state));
            desk.stages.sort_by_key(|(s, _)| *s);
        }
    }
    desk.updated = crate::calib::stamp();
    save(home, desk)
}

/// The outline stage's ask: the brief, the coverage verdict, and the
/// asserted claims ride one composed ask — the answer is the outline,
/// where a named gap renders as a gap because the prompt says so.
pub async fn outline_ask(
    source: &crate::lanes::LaneSource,
    desk: &DeskProject,
    coverage: Option<&CoverageRecord>,
) -> Result<String, String> {
    let spec = crate::judge::lane_spec(source, "blend1")?;
    let server = crate::mcp::McpServer::spawn(&spec)
        .map_err(|error| format!("the composed lane could not start: {error}"))?;
    let result = async {
        server
            .initialize()
            .await
            .map_err(|error| format!("the composed lane's handshake failed: {error}"))?;
        let tools = server
            .list_tools()
            .await
            .map_err(|error| format!("the composed lane's tool list failed: {error}"))?;
        if !tools
            .iter()
            .any(|declared| declared.name == BLEND1_ASK_TOOL)
        {
            return Err(format!(
                "lane `blend1` declares no `{BLEND1_ASK_TOOL}` tool — the outline stage cannot ask"
            ));
        }
        let asserted: Vec<&Claim> = desk
            .claims
            .iter()
            .filter(|c| c.verdict == ClaimVerdict::Asserted)
            .collect();
        let mut prompt = format!(
            "Compose the outline for a deliverable.\nAudience: {}\nThe claim it must make: {}\n",
            desk.audience, desk.claim
        );
        if let Some(coverage) = coverage {
            if coverage.escalated {
                prompt.push_str(&format!(
                    "The coverage judgment did not assert ({}). Name the gaps as gaps.\n",
                    coverage.reason
                ));
            } else {
                prompt.push_str(&format!(
                    "Coverage scored {} of 5. A material gap: {}. Off-brief claims: {}.\n",
                    coverage.score, coverage.material_gap, coverage.off_brief
                ));
            }
        }
        prompt.push_str(&format!(
            "The claim set carries {} asserted claims. Write a numbered outline; \
             a sub-question the claims do not answer renders as `GAP:` followed by the \
             question, never as an answer.",
            asserted.len()
        ));
        let arguments = serde_json::json!({ "prompt": prompt, "retrieve": false });
        let call: crate::mcp::CallResult = server
            .call_tool(BLEND1_ASK_TOOL, arguments)
            .await
            .map_err(|error| format!("the composed lane refused the outline: {error}"))?;
        if call.is_error {
            return Err(format!(
                "the composed lane refused the outline: {}",
                call.text()
            ));
        }
        let answer: serde_json::Value =
            serde_json::from_str(call.text().as_str()).map_err(|error| {
                format!("the composed lane's answer was not structured content: {error}")
            })?;
        answer
            .get("final_answer")
            .and_then(|a| a.as_str())
            .map(str::to_string)
            .ok_or_else(|| "the composed lane's answer carries no final_answer".to_string())
    };
    let outcome = result.await;
    server.close().await.ok();
    outcome
}

/// The document target's render: deterministic markdown over the
/// outline and the claims — the deliverable's evidence-first body,
/// every claim carrying its quote. The run's model polishes prose; the
/// numbers and the quotes come from here, where they are honest.
pub fn render_document(desk: &DeskProject) -> String {
    let mut out = String::new();
    out.push_str(&format!("# {}\n\n", desk.name));
    out.push_str(&format!("*For {}.*\n\n", desk.audience));
    if let Some(outline) = &desk.outline {
        out.push_str(&format!("{outline}\n\n"));
    }
    out.push_str("## The claims\n\n");
    for claim in &desk.claims {
        match claim.verdict {
            ClaimVerdict::Asserted => out.push_str(&format!(
                "- {} _(p {:.2}, {})_\n  > \"{}\"\n",
                claim.text, claim.confidence, claim.source, claim.sentence
            )),
            ClaimVerdict::Contested => out.push_str(&format!(
                "- {} _(contested, p {:.2}, {})_\n",
                claim.text, claim.confidence, claim.source
            )),
            ClaimVerdict::Vetoed => {}
        }
    }
    if let Some(coverage) = &desk.coverage {
        if coverage.material_gap {
            out.push_str(&format!(
                "\n> GAP: the sources do not answer every sub-question the claim implies (coverage {}). The gap is named, not filled.\n",
                if coverage.escalated { "unresolved".to_string() } else { format!("{:.0}/5", coverage.score) }
            ));
        }
    }
    out
}

/// What stands between the deliverable and its end: the document law's
/// refusals and the number guard's, named — `None` when nothing does.
pub fn end_stands(desk: &DeskProject) -> Option<String> {
    let mut stands = Vec::new();
    for refusal in &desk.refusals {
        stands.push(format!("unsourced assertion: \"{}\"", refusal.assertion));
    }
    for refusal in &desk.number_refusals {
        stands.push(format!("unsourced number: {}", refusal.raw));
    }
    if stands.is_empty() {
        None
    } else {
        Some(stands.join("; "))
    }
}

/// End a deliverable: write its knowledge and stamp the record. The end
/// gate is the law — a refusal that stands refuses the end by name, so a
/// deliverable that cannot end says why rather than accepting the press.
///
/// This is the distil step the plan's C10 names and the face had no verb
/// for (KI-435): the digest is written to its project-keyed path (the
/// unit the assembled-context read serves and the corpus refresh embeds),
/// the knowledge stage is marked done, and the document records where its
/// knowledge landed. Unbound desk work still ends — it keeps no digest
/// because nothing keys one, and the record says so by staying `None`.
pub fn end_desk(home: &Path, slug: &str) -> Result<DeskProject, String> {
    let mut desk = load(home, slug)?;
    if let Some(stands) = end_stands(&desk) {
        return Err(format!("the deliverable cannot end while {stands}"));
    }
    // The digest's path is derived, never guessed: `refresh_digest` wrote
    // it under the project's own key when the binding was made.
    let digest = desk
        .project
        .as_deref()
        .and_then(crate::canvas::project_key)
        .map(|key| digest_path(home, &key, &desk.slug))
        .filter(|path| path.is_file())
        .map(|path| path.to_string_lossy().to_string());
    desk.digest = digest;
    mark_stage(home, &mut desk, Stage::Knowledge, StageState::Done)?;
    Ok(desk)
}

/// One source's text for the claim pass: files read from disk, URL
/// captures read from the desk project's sources directory.
///
/// A relative path is the desk project's own — `create` names sources
/// the way the brief words them (`sources/brief.pdf`), and resolving
/// them against the process CWD made a correct brief read as zero
/// claims in silence. An unreadable file is a named refusal, never a
/// skip: a brief whose source vanished must stop its stage by name.
/// Only a fact is a by-design absence — its text rides the assembled
/// context, not the claim pass.
fn source_text(home: &Path, desk: &DeskProject, source: &Source) -> Result<Option<String>, String> {
    let path = match source.kind {
        SourceKind::Url => desk_root(home).join(&source.path),
        SourceKind::File => {
            let given = Path::new(&source.path);
            if given.is_absolute() {
                given.to_path_buf()
            } else {
                desk_root(home).join(&desk.slug).join(given)
            }
        }
        SourceKind::Fact => return Ok(None),
    };
    std::fs::read_to_string(&path)
        .map(Some)
        .map_err(|error| format!("the source {} could not be read: {error}", path.display()))
}

/// The walk. `read_claims` says whether the claims stage should read (a
/// resumed store with its claim set stands, and passes `false`). Every
/// stage marks the store as it lands; a stage that fails marks refused
/// and returns the error. The claim pass is a sys1 lane client: the
/// vendor is the lane's, and the leg's judgment rides its answers.
pub async fn run_stages(
    source: &crate::lanes::LaneSource,
    home: &Path,
    slug: &str,
    read_claims: bool,
) -> Result<DeskProject, String> {
    let mut desk = load(home, slug)?;

    // Claims.
    if desk.claims.is_empty() {
        mark_stage(home, &mut desk, Stage::Claims, StageState::Running)?;
        if !read_claims {
            let error =
                "the claims stage was not given the lane and the store holds no claims".to_string();
            mark_stage(
                home,
                &mut desk,
                Stage::Claims,
                StageState::Refused {
                    reason: error.clone(),
                },
            )?;
            return Err(error);
        };
        let session = match crate::extract::ExtractSession::open(source).await {
            Ok(session) => session,
            Err(error) => {
                let reason = format!("the claims stage could not reach sys1: {error}");
                mark_stage(
                    home,
                    &mut desk,
                    Stage::Claims,
                    StageState::Refused {
                        reason: reason.clone(),
                    },
                )?;
                return Err(reason);
            }
        };
        let mut claims = Vec::new();
        for source_desc in &desk.sources {
            let text = match source_text(home, &desk, source_desc) {
                Ok(Some(text)) => text,
                Ok(None) => continue,
                Err(reason) => {
                    mark_stage(
                        home,
                        &mut desk,
                        Stage::Claims,
                        StageState::Refused {
                            reason: reason.clone(),
                        },
                    )?;
                    return Err(reason);
                }
            };
            match claims_from_source(&session, &source_desc.path, &text).await {
                Ok(mut found) => claims.append(&mut found),
                Err(error) => {
                    let reason = format!("the claim pass failed on {}: {error}", source_desc.path);
                    mark_stage(
                        home,
                        &mut desk,
                        Stage::Claims,
                        StageState::Refused {
                            reason: reason.clone(),
                        },
                    )?;
                    return Err(reason);
                }
            }
        }
        desk.claims = claims;
        mark_stage(home, &mut desk, Stage::Claims, StageState::Done)?;
    } else if !matches!(desk.stage(Stage::Claims), StageState::Done) {
        // The resume path: the store's claim set stands, and the stage
        // says so.
        mark_stage(home, &mut desk, Stage::Claims, StageState::Done)?;
    }

    // Coverage.
    if desk.coverage.is_none() {
        mark_stage(home, &mut desk, Stage::Outline, StageState::Running)?;
        match coverage_ask(source, home, &desk).await {
            Ok(coverage) => {
                desk.coverage = Some(CoverageRecord {
                    score: coverage.score,
                    material_gap: coverage.material_gap,
                    off_brief: coverage.off_brief,
                    escalated: coverage.escalated,
                    reason: coverage.reason,
                });
            }
            Err(error) => {
                // The beat's named absence rides the outline: the gap
                // question stays open.
                desk.coverage = Some(CoverageRecord {
                    score: 0.0,
                    material_gap: true,
                    off_brief: false,
                    escalated: true,
                    reason: error.clone(),
                });
            }
        }
        save(home, &desk)?;
    }

    // Outline.
    if desk.outline.is_none() {
        mark_stage(home, &mut desk, Stage::Outline, StageState::Running)?;
        match outline_ask(source, &desk, desk.coverage.as_ref()).await {
            Ok(outline) => desk.outline = Some(outline),
            Err(error) => {
                mark_stage(
                    home,
                    &mut desk,
                    Stage::Outline,
                    StageState::Refused {
                        reason: error.clone(),
                    },
                )?;
                return Err(error);
            }
        }
        mark_stage(home, &mut desk, Stage::Outline, StageState::Done)?;
    }

    // Render.
    if desk.draft.is_none() {
        mark_stage(home, &mut desk, Stage::Render, StageState::Running)?;
        desk.draft = Some(render_document(&desk));
        mark_stage(home, &mut desk, Stage::Render, StageState::Done)?;
    }

    // Review: the number guard over the draft; the document law's
    // refusals are already the store's own.
    if desk.refusals.is_empty() && desk.number_refusals.is_empty() {
        mark_stage(home, &mut desk, Stage::Review, StageState::Running)?;
        let draft = desk.draft.clone().unwrap_or_default();
        desk.number_refusals = check_numbers(&draft, &number_givens(&desk));
        let state = if desk.number_refusals.is_empty() {
            StageState::Done
        } else {
            StageState::Refused {
                reason: format!(
                    "{} unsourced number(s) — the review names them",
                    desk.number_refusals.len()
                ),
            }
        };
        mark_stage(home, &mut desk, Stage::Review, state)?;
    }

    // Knowledge: the digest rides every save; the corpus seed lands
    // here, and a bound project's seed failure refuses the stage by
    // name.
    mark_stage(home, &mut desk, Stage::Knowledge, StageState::Running)?;
    match seed_corpus(source, &desk).await {
        Ok(()) => mark_stage(home, &mut desk, Stage::Knowledge, StageState::Done)?,
        Err(error) => {
            mark_stage(
                home,
                &mut desk,
                Stage::Knowledge,
                StageState::Refused {
                    reason: error.clone(),
                },
            )?;
            return Err(error);
        }
    }

    Ok(desk)
}

// --- the face's JSON ---------------------------------------------------------

/// The slug a brief's name becomes: lowercase words joined by hyphens —
/// the canvas slug rule's reading (`src/canvas.rs`), punctuation dropped.
pub fn slug_for(name: &str) -> String {
    let slug: Vec<&str> = name
        .split(|c: char| !c.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect();
    let joined = slug.join("-");
    let joined = joined.as_str().trim_matches('-').to_string();
    if joined.is_empty() {
        "deliverable".to_string()
    } else {
        joined.chars().take(64).collect()
    }
}

/// One desk project as the face reads it: the whole store document —
/// the brief, the shape, the stages with their marks, the claims with
/// their sentences and verdicts, the coverage record, the outline, the
/// draft, every refusal named. The states are the store's, never
/// derived twice.
pub fn project_json(desk: &DeskProject) -> serde_json::Value {
    serde_json::json!({
        "slug": desk.slug,
        "name": desk.name,
        "audience": desk.audience,
        "claim": desk.claim,
        "shape": desk.shape,
        "project": desk.project,
        "sources": desk.sources.iter().map(|source| serde_json::json!({
            "type": match source.kind {
                SourceKind::File => "file",
                SourceKind::Fact => "fact",
                SourceKind::Url => "url",
            },
            "path": source.path,
            "url": source.url,
            "fetched_at": source.fetched_at,
            "title": source.title,
        })).collect::<Vec<_>>(),
        "stages": desk.stages.iter().map(|(stage, state)| {
            let (mark, reason) = match state {
                StageState::Pending => ("pending", None),
                StageState::Running => ("running", None),
                StageState::Done => ("done", None),
                StageState::Refused { reason } => ("refused", Some(reason.as_str())),
            };
            serde_json::json!({ "stage": stage.name(), "state": mark, "reason": reason })
        }).collect::<Vec<_>>(),
        "claims": desk.claims.iter().map(|claim| serde_json::json!({
            "text": claim.text,
            "source": claim.source,
            "sentence": claim.sentence,
            "confidence": claim.confidence,
            "kind": match claim.kind {
                ClaimKind::Claim => "claim",
                ClaimKind::Support => "support",
                ClaimKind::Background => "background",
                ClaimKind::Boilerplate => "boilerplate",
            },
            "verdict": match claim.verdict {
                ClaimVerdict::Asserted => "asserted",
                ClaimVerdict::Contested => "contested",
                ClaimVerdict::Vetoed => "vetoed",
            },
        })).collect::<Vec<_>>(),
        "coverage": desk.coverage.as_ref().map(|coverage| serde_json::json!({
            "score": coverage.score,
            "material_gap": coverage.material_gap,
            "off_brief": coverage.off_brief,
            "escalated": coverage.escalated,
            "reason": coverage.reason,
        })),
        "outline": desk.outline,
        "draft": desk.draft,
        "refusals": desk.refusals.iter().map(|refusal| serde_json::json!({
            "assertion": refusal.assertion,
            "near_miss": refusal.near_miss,
            "near_miss_source": refusal.near_miss_source,
            "overlap": refusal.overlap,
        })).collect::<Vec<_>>(),
        "number_refusals": desk.number_refusals.iter().map(|refusal| serde_json::json!({
            "raw": refusal.raw,
            "canonical": refusal.canonical,
        })).collect::<Vec<_>>(),
        "figures": desk.figures.len(),
        "digest": desk.digest,
        "stands": end_stands(desk),
        "created": desk.created,
        "updated": desk.updated,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_home(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("fleet-desk-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn sample() -> DeskProject {
        DeskProject {
            slug: "landing-copy".to_string(),
            shape: None,
            name: "The landing page's copy".to_string(),
            audience: "the beta cohort".to_string(),
            claim: "the fold earns its keep".to_string(),
            sources: vec![Source {
                kind: SourceKind::Url,
                path: "sources/brief-source.txt".to_string(),
                url: Some("https://example.com/brief".to_string()),
                fetched_at: Some("2026-11-04T12:00:00Z".to_string()),
                title: Some("The brief".to_string()),
            }],
            project: Some("/Users/someone/Projects/landing".to_string()),
            stages: STAGES.map(|s| (s, StageState::Pending)).to_vec(),
            claims: vec![Claim {
                text: "the fold earns its keep".to_string(),
                source: "sources/brief-source.txt".to_string(),
                sentence: "The fold earns its keep on every screen.".to_string(),
                confidence: 0.91,
                kind: ClaimKind::Claim,
                verdict: ClaimVerdict::Asserted,
            }],
            refusals: Vec::new(),
            figures: Vec::new(),
            coverage: None,
            outline: None,
            draft: None,
            number_refusals: Vec::new(),
            digest: None,
            created: "2026-11-04T12:00:00Z".to_string(),
            updated: "2026-11-04T12:00:00Z".to_string(),
        }
    }

    #[test]
    fn metric_numbers_canonicalize_their_shapes() {
        let numbers = metric_numbers("92.4% of $4,180.00 across 12 runs, at 0.924 and 90ms");
        let values: Vec<f64> = numbers.iter().map(|(_, v)| *v).collect();
        assert_eq!(
            values,
            vec![0.924, 4180.0, 12.0, 0.924, 90.0],
            "{numbers:?}"
        );
    }

    #[test]
    fn the_number_scan_walks_characters_not_bytes() {
        // A real draft carries multi-byte glyphs (em dashes, curly
        // quotes); a byte step through one used to panic the scan.
        let numbers = metric_numbers("the total — as agreed — was $4,180.00");
        assert_eq!(numbers, vec![("$4,180.00".to_string(), 4180.0)]);
        assert!(metric_numbers("— — —").is_empty());
    }

    #[test]
    fn a_relative_source_resolves_under_the_desk_project() {
        let home = temp_home("source-relative");
        let mut desk = sample();
        desk.slug = "brief".to_string();
        desk.sources = vec![Source {
            kind: SourceKind::File,
            path: "sources/brief-source.txt".to_string(),
            url: None,
            fetched_at: None,
            title: None,
        }];
        let dir = desk_root(&home).join("brief").join("sources");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("brief-source.txt"), "The fold earns its keep.").unwrap();
        let text = source_text(&home, &desk, &desk.sources[0]).unwrap();
        assert_eq!(text.as_deref(), Some("The fold earns its keep."));
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn the_emitted_stages_carry_keys_not_tuple_arrays() {
        // KI-444: `stages` is `Vec<(Stage, StageState)>`, and an audit that
        // assumed a tuple serialized as a two-element array would write a
        // reader finding nothing. `project_json` builds the objects
        // explicitly; this pins the shape a consumer actually receives, so
        // the next reader is written against the emitted keys rather than
        // against the Rust type's apparent arity.
        let mut desk = sample();
        desk.stages = vec![
            (Stage::Claims, StageState::Done),
            (Stage::Outline, StageState::Running),
        ];
        let emitted = project_json(&desk);
        let stages = emitted
            .get("stages")
            .and_then(serde_json::Value::as_array)
            .unwrap_or_else(|| panic!("stages must be an array: {emitted}"));
        assert_eq!(stages.len(), 2, "{emitted}");
        for stage in stages {
            let object = stage
                .as_object()
                .unwrap_or_else(|| panic!("a stage must be an object, not a tuple array: {stage}"));
            assert!(
                object.contains_key("stage"),
                "the stage's own name: {stage}"
            );
            assert!(object.contains_key("state"), "the state key: {stage}");
            assert!(object.contains_key("reason"), "the reason key: {stage}");
        }
        assert_eq!(stages[0]["stage"], "claims");
        assert_eq!(stages[0]["state"], "done");
        assert_eq!(stages[1]["state"], "running");
    }

    #[test]
    fn a_missing_source_is_a_named_refusal_not_a_silent_zero() {
        // The silent skip this replaces made a correct brief read as
        // zero claims: the stage said done, the claim set was empty.
        let home = temp_home("source-missing");
        let desk = sample();
        let error = source_text(&home, &desk, &desk.sources[0]).unwrap_err();
        assert!(error.contains("could not be read"), "{error}");
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn marking_a_stage_the_store_lacks_lands_it_in_walk_order() {
        let home = temp_home("stage-insert");
        let mut desk = sample();
        desk.stages = Vec::new();
        save(&home, &desk).unwrap();
        mark_stage(&home, &mut desk, Stage::Render, StageState::Running).unwrap();
        let stored = load(&home, "landing-copy").unwrap();
        assert_eq!(
            stored.stages,
            vec![(Stage::Render, StageState::Running)],
            "a stage the store lacks must land, not vanish"
        );
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn the_number_guard_admits_a_sourced_figure_and_refuses_an_invented_one() {
        let givens = vec![0.924, 4180.0];
        assert!(
            check_numbers("the load read 92.4% and the total was $4,180.00", &givens).is_empty()
        );
        let refusals = check_numbers("the footer converts at 4.0%", &givens);
        assert_eq!(refusals.len(), 1);
        assert_eq!(refusals[0].raw, "4.0%");
        assert!((refusals[0].canonical - 0.04).abs() < 1e-9);
    }

    #[test]
    fn the_givens_read_claims_and_figures_alike() {
        let mut desk = sample();
        let figure = Figure {
            target: "churned".to_string(),
            task_type: "classification".to_string(),
            answer: serde_json::json!({
                "verdict": "pass",
                "predictions": [{"churned": 1, "probabilities": {"1": 0.842}}]
            }),
            computed_by: "sdm1".to_string(),
        };
        desk.figures.push(figure);
        let givens = number_givens(&desk);
        assert!(
            givens.iter().any(|v| (v - 0.842).abs() < 1e-9),
            "{givens:?}"
        );
    }

    #[test]
    fn a_figure_answered_in_error_is_a_named_absence() {
        let request = FigureRequest {
            table: vec![serde_json::json!({"x": 1, "y": null})],
            target: "y".to_string(),
            task_type: "classification".to_string(),
        };
        let error = parse_figure(
            r#"{"verdict": "error", "summary": "no labeled context rows"}"#,
            &request,
        )
        .unwrap_err();
        assert!(error.contains("no labeled context rows"), "{error}");
    }

    #[tokio::test]
    async fn an_unbound_desk_project_names_its_seed_skip() {
        // The seed is async, but the unbound refusal fires before any
        // lane: no spawn stands between the call and the refusal.
        let mut desk = sample();
        desk.project = None;
        let error = seed_corpus(
            &crate::lanes::LaneSource::Scripted(crate::mcp::ServerSpec {
                program: "true".to_string(),
                args: vec![],
                env: Vec::new(),
                passthrough: Vec::new(),
                cwd: None,
            }),
            &desk,
        )
        .await
        .unwrap_err();
        assert!(error.contains("unbound"), "{error}");
    }

    #[test]
    fn deleting_a_project_removes_its_digest() {
        let home = temp_home("delete");
        save(&home, &sample()).unwrap();
        assert!(digest_path(&home, "landing", "landing-copy").is_file());
        delete_project(&home, "landing-copy").unwrap();
        assert!(!exists(&home, "landing-copy"));
        assert!(!digest_path(&home, "landing", "landing-copy").exists());
    }
    #[test]
    fn the_document_render_carries_quotes_marks_and_the_named_gap() {
        let mut desk = sample();
        desk.outline = Some("1. The fold\n2. What the labs measured".to_string());
        desk.coverage = Some(CoverageRecord {
            score: 3.0,
            material_gap: true,
            off_brief: false,
            escalated: false,
            reason: String::new(),
        });
        let draft = render_document(&desk);
        assert!(draft.starts_with("# The landing page's copy"));
        assert!(draft.contains("1. The fold"));
        assert!(draft.contains("> \"The fold earns its keep on every screen.\""));
        assert!(draft.contains("GAP:"));
        assert!(draft.contains("3/5"));
        // A contested claim ships marked, never as an assertion.
        desk.claims.push(Claim {
            text: "a contested reading".to_string(),
            source: "sources/x.txt".to_string(),
            sentence: "s".to_string(),
            confidence: 0.6,
            kind: ClaimKind::Claim,
            verdict: ClaimVerdict::Contested,
        });
        let draft = render_document(&desk);
        assert!(draft.contains("_(contested, p 0.60"));
    }

    #[test]
    fn the_end_gate_names_what_stands_or_says_nothing() {
        let mut desk = sample();
        assert_eq!(end_stands(&desk), None);
        desk.refusals.push(UnsourcedAssertion {
            assertion: "the footer converts at four percent".to_string(),
            near_miss: String::new(),
            near_miss_source: String::new(),
            overlap: 0.17,
        });
        desk.number_refusals.push(UnsourcedNumber {
            raw: "4.0%".to_string(),
            canonical: 0.04,
        });
        let stands = end_stands(&desk).unwrap();
        assert!(stands.contains("the footer converts at four percent"));
        assert!(stands.contains("4.0%"));
    }
    #[test]
    fn the_live_proofs_outline_zeros_refuse() {
        // And the outline's invented "0 asserted claims" pair refuses —
        // the honest guard, the live walk's own finding.
        let givens = vec![0.81, 0.18, 0.01, 3.5576];
        let draft = "3. Claim set: 0 asserted claims (the claim set carries 0 asserted claims)";
        let refusals = check_numbers(draft, &givens);
        assert_eq!(refusals.len(), 2, "{refusals:?}");
    }
    #[test]
    fn the_display_rounds_and_the_guard_grades_the_display() {
        // The live walk (2026-11-04) refused its own "p 0.81": the
        // stored confidence was 0.8087…, the render prints two
        // decimals. The guard grades the display, so the rounded
        // value admits the rendered annotation.
        let mut desk = sample();
        desk.claims[0].confidence = 0.808_732_387_819_014_7;
        let givens = number_givens(&desk);
        let refusals = check_numbers("_(contested, p 0.81, sources/x)_", &givens);
        assert!(refusals.is_empty(), "{refusals:?}");
    }
}
