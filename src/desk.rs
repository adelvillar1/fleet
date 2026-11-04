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
        (self.refusals.len(), self.claims.iter().filter(|c| c.verdict == ClaimVerdict::Contested).count())
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
    desk_root(home).join(DIGESTS_DIR).join(key).join(format!("{slug}.md"))
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
        name: name.to_string(),
        audience: audience.to_string(),
        claim: claim.to_string(),
        sources,
        project,
        stages: STAGES.map(|s| (s, StageState::Pending)).to_vec(),
        claims: Vec::new(),
        refusals: Vec::new(),
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
pub fn check_draft(assertions: &[String], claims: &[Claim]) -> Vec<Result<Claim, UnsourcedAssertion>> {
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
    let asserted = desk.claims.iter().filter(|c| c.verdict == ClaimVerdict::Asserted).count();
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
            digest: None,
            created: "2026-11-04T12:00:00Z".to_string(),
            updated: "2026-11-04T12:00:00Z".to_string(),
        }
    }

    #[test]
    fn a_desk_project_round_trips_through_the_store() {
        let home = temp_home("round-trip");
        let desk = sample();
        save(&home, &desk).unwrap();
        let read = load(&home, "landing-copy").unwrap();
        assert_eq!(read, desk);
        assert!(exists(&home, "landing-copy"));
    }

    #[test]
    fn a_second_project_with_a_lived_slug_is_refused_by_name() {
        let home = temp_home("collision");
        create(&home, "landing-copy", "one", "a", "c", vec![], None).unwrap();
        let error = create(&home, "landing-copy", "two", "a", "c", vec![], None)
            .unwrap_err();
        assert!(error.contains("already exists"), "{error}");
        assert!(error.contains("supersede"), "{error}");
    }

    #[test]
    fn the_binding_keys_the_digest_by_the_folders_final_segment() {
        let home = temp_home("binding");
        save(&home, &sample()).unwrap();
        let path = digest_path(&home, "landing", "landing-copy");
        assert!(path.is_file(), "the digest lands at {}", path.display());
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("# The landing page's copy"));
        assert!(text.contains("claims: 1 asserted"));
    }

    #[test]
    fn an_unbound_project_keeps_no_digest() {
        let home = temp_home("unbound");
        let mut desk = sample();
        desk.project = None;
        save(&home, &desk).unwrap();
        assert!(!desk_root(&home).join(DIGESTS_DIR).exists());
    }

    #[test]
    fn the_document_law_admits_a_sourced_assertion() {
        let desk = sample();
        let claim = check_assertion("the fold earns its keep on every screen", &desk.claims).unwrap();
        assert_eq!(claim.source, "sources/brief-source.txt");
    }

    #[test]
    fn the_document_law_refuses_and_names_the_near_miss() {
        let desk = sample();
        let error = check_assertion("the footer converts at four percent", &desk.claims)
            .unwrap_err();
        assert_eq!(error.near_miss, "The fold earns its keep on every screen.");
        assert!(error.near_miss_source.contains("brief-source"));
        assert!(error.overlap < ASSERTION_MATCH_FLOOR);
    }

    #[test]
    fn a_vetoed_claim_never_carries_an_assertion() {
        let mut desk = sample();
        desk.claims[0].verdict = ClaimVerdict::Vetoed;
        assert!(check_assertion("the fold earns its keep", &desk.claims).is_err());
    }

    #[test]
    fn the_context_section_is_bounded_and_one_line_per_deliverable() {
        let home = temp_home("context");
        for n in 0..(DESK_CONTEXT_MAX + 2) {
            let mut desk = sample();
            desk.slug = format!("deliverable-{n}");
            desk.name = format!("Deliverable {n}");
            save(&home, &desk).unwrap();
        }
        let section = desk_context_section(&home, "landing").unwrap();
        assert!(section.starts_with("## Desk"));
        assert_eq!(section.lines().count() - 2, DESK_CONTEXT_MAX);
        // Newest first, so the bound cuts the oldest two.
        assert!(section.contains("Deliverable 9"));
        assert!(!section.contains("Deliverable 0"));
    }

    // --- the claim pass ------------------------------------------------------

    use std::io::{Read, Write};
    use std::sync::{Arc, Mutex};

    /// A stub systemone endpoint that routes by marker: the first body
    /// whose marker appears in the request wins, else the default. The
    /// desk's claim pass makes two shapes of call — the batched
    /// narrowing (its state carries `[SEGMENT`) and the per-segment
    /// kind choice — and the stub answers each in its own voice.
    fn stub_by_marker(routes: Vec<(&'static str, String)>, default: String) -> (u16, Arc<Mutex<Vec<String>>>) {
        let routes: Arc<Vec<(&'static str, String)>> = Arc::new(routes);
        let default = Arc::new(default);
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let seen: Arc<Mutex<Vec<String>>> = Arc::default();
        let seen_writer = Arc::clone(&seen);
        std::thread::spawn(move || {
            for mut stream in listener.incoming().flatten() {
                let mut request: Vec<u8> = Vec::new();
                let mut chunk = [0u8; 8192];
                loop {
                    let read = stream.read(&mut chunk).unwrap_or(0);
                    if read == 0 {
                        break;
                    }
                    request.extend_from_slice(&chunk[..read]);
                    if request.windows(4).any(|w| w == b"\r\n\r\n") {
                        let head_end = request.windows(4).position(|w| w == b"\r\n\r\n").unwrap() + 4;
                        let head = String::from_utf8_lossy(&request[..head_end]).to_ascii_lowercase();
                        let declared: usize = head
                            .lines()
                            .find_map(|line| line.strip_prefix("content-length:"))
                            .and_then(|value| value.trim().parse().ok())
                            .unwrap_or(0);
                        if request.len() >= head_end + declared {
                            break;
                        }
                    }
                }
                let request = String::from_utf8_lossy(&request).into_owned();
                if let Some(body_at) = request.find("\r\n\r\n") {
                    seen_writer.lock().unwrap().push(request[body_at + 4..].to_string());
                }
                let body = routes
                    .iter()
                    .find(|(marker, _)| request.contains(marker))
                    .map(|(_, body)| body.clone())
                    .unwrap_or_else(|| (*default).clone());
                let reply = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = stream.write_all(reply.as_bytes());
            }
        });
        (port, seen)
    }

    fn kind_body(p_claim: f64) -> String {
        serde_json::json!({
            "answers": {
                "kind": {
                    "label": if p_claim >= 0.5 { "claim" } else { "background" },
                    "probabilities": {
                        "claim": p_claim,
                        "support": (1.0 - p_claim) * 0.3,
                        "background": (1.0 - p_claim) * 0.5,
                        "boilerplate": (1.0 - p_claim) * 0.2,
                    }
                }
            }
        })
        .to_string()
    }

    /// The first verifier, glide: always present in a resolved config,
    /// state_object false — the plain-string state.
    fn glide_at(port: u16) -> Verifier {
        Verifier {
            name: "glide".to_string(),
            url: format!("http://127.0.0.1:{port}/v1/systemone"),
            model: "fastino/glide".to_string(),
            key: "test-key".to_string(),
            state_object: false,
        }
    }

    fn client_at(port: u16) -> ExtractClient {
        ExtractClient::new(crate::extract::ExtractConfig {
            base_url: format!("http://127.0.0.1:{port}"),
            entity_model: "fastino/gliner2.5-multi-v1".to_string(),
            token: "test-key".to_string(),
            verifiers: vec![glide_at(port)],
        })
    }

    #[test]
    fn the_claim_pass_asserts_vetoes_and_labels_by_the_floors() {
        let text = "The fold earns its keep on every screen the owner opens. \
The footer carries the usual legal furniture and nothing more. \
Independent labs measured the load at ninety milliseconds.";
        // Three sentences, three segments; each gets its own kind call.
        let (port, seen) = stub_by_marker(
            vec![
                ("The fold earns its keep", kind_body(0.91)),
                ("legal furniture", kind_body(0.10)),
                ("ninety milliseconds", kind_body(0.60)),
            ],
            kind_body(0.0),
        );
        let client = client_at(port);
        let verifier = glide_at(port);
        let claims = claims_from_source(&client, &verifier, "sources/brief.txt", text).unwrap();
        assert_eq!(claims.len(), 3, "{claims:?}");
        assert_eq!(claims[0].verdict, ClaimVerdict::Asserted);
        assert_eq!(claims[0].kind, ClaimKind::Claim);
        assert_eq!(claims[1].verdict, ClaimVerdict::Vetoed);
        assert_eq!(claims[2].verdict, ClaimVerdict::Contested);
        assert!(claims.iter().all(|c| c.source == "sources/brief.txt"));
        // No batched call ran: a short source carries every segment.
        assert!(seen.lock().unwrap().iter().all(|body| !body.contains("[SEGMENT")));
    }

    #[test]
    fn a_long_source_narrows_in_chunks_before_it_verifies() {
        // 30 sentence-segments: over the carrying cap, so the
        // narrowing rides first — 3 chunks of <= 14.
        let mut text = String::new();
        for i in 0..30 {
            text.push_str(&format!(
                "Sentence number {i} states a measured fact about the system. "
            ));
        }
        let batched = serde_json::json!({
            "answers": {
                "seg-0": {"noul": 0.9},
                "seg-1": {"noul": 0.8},
            }
        })
        .to_string();
        let (port, seen) = stub_by_marker(vec![("[SEGMENT", batched)], kind_body(0.95));
        let client = client_at(port);
        let verifier = glide_at(port);
        let claims = claims_from_source(&client, &verifier, "sources/long.txt", &text).unwrap();
        let bodies = seen.lock().unwrap();
        let batched_calls = bodies.iter().filter(|body| body.contains("[SEGMENT")).count();
        assert_eq!(batched_calls, 3, "30 segments narrow in 3 chunks of <= 14");
        // Only the narrowed survivors cost a kind call: two per chunk
        // (seg-0 and seg-1 clear the batched floor), so six.
        let kind_calls = bodies.iter().filter(|body| !body.contains("[SEGMENT")).count();
        assert_eq!(kind_calls, 6, "{kind_calls}");
        assert_eq!(claims.len(), 6);
        assert!(claims.iter().all(|c| c.verdict == ClaimVerdict::Asserted));
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
}

// --- the claim set -----------------------------------------------------------
//
// The spine's shape, applied to a source: segment (pure Rust), narrow
// (one batched call per chunk when the source is long), verify
// (per carrying segment), reconcile (the measured floors). Every claim
// carries its source and its sentence; a declined answer reads as a
// veto at 0.0 — a named absence, never a fabricated claim.

use crate::extract::{ExtractClient, ExtractError, Segment, Verifier};

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
fn choice_probabilities(
    answers: &serde_json::Value,
    id: &str,
) -> Option<Vec<(String, f64)>> {
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
fn narrow_claims(
    client: &ExtractClient,
    verifier: &Verifier,
    segments: &[Segment],
) -> Result<Vec<usize>, ExtractError> {
    let mut kept: Vec<(usize, f64)> = Vec::new();
    for (chunk_at, chunk) in segments.chunks(CLAIM_BATCH_CHUNK).enumerate() {
        let state = chunk
            .iter()
            .enumerate()
            .map(|(i, segment)| format!("[SEGMENT {}] {}", chunk_at * CLAIM_BATCH_CHUNK + i, segment.text))
            .collect::<Vec<_>>()
            .join("\n\n");
        let mut questions = serde_json::Map::new();
        for i in 0..chunk.len() {
            questions.insert(
                format!("seg-{i}"),
                serde_json::json!({
                    "type": "noul",
                    "instructions": "Does this segment state a claim a deliverable could stand on, with its evidence in the segment's own words?",
                    "criteria": {
                        "true": "Yes, the segment states such a claim.",
                        "false": "No, it is support, background, or boilerplate.",
                    },
                    "id": format!("seg-{i}"),
                }),
            );
        }
        let answers = client.answers(verifier, &state, serde_json::Value::Object(questions))?;
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
pub fn claims_from_source(
    client: &ExtractClient,
    verifier: &Verifier,
    source: &str,
    text: &str,
) -> Result<Vec<Claim>, ExtractError> {
    let segments = crate::extract::segments(text);
    let carrying: Vec<usize> = if segments.len() > crate::extract::MAX_CARRYING_SEGMENTS {
        narrow_claims(client, verifier, &segments)?
    } else {
        (0..segments.len()).collect()
    };
    let mut claims = Vec::new();
    for at in carrying {
        let segment = &segments[at];
        let mut questions = serde_json::Map::new();
        let mut head = serde_json::json!({
            "type": "choice",
            "instructions": "What is this text? One label.",
            "criteria": {
                "claim": "A statement a deliverable could stand on, with its evidence in the text.",
                "support": "Evidence for a claim stated elsewhere, not itself the claim.",
                "background": "Context that carries no claim.",
                "boilerplate": "Furniture: company blurb, footers, disclaimers.",
            },
        });
        if verifier.state_object {
            head["id"] = serde_json::Value::String("kind".to_string());
        }
        questions.insert("kind".to_string(), head);
        let answers = client.answers(
            verifier,
            &segment.text,
            serde_json::Value::Object(questions),
        )?;
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
