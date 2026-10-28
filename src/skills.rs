//! Skills: SKILL.md loading with frontmatter, progressive disclosure
//! through the `use_skill` tool, the self-authoring drafter, and
//! usage-scan curation.
//!
//! The layout is the owner's own convention: one directory per skill in
//! the canonical library — `$FLEET_SKILLS_DIR`, else
//! `<user home>/.agents/skills`, the owner's cross-harness library, so a
//! skill is authored once and every harness loads the same source —
//! each directory holding a `SKILL.md` whose frontmatter carries at
//! least `name` and `description`, with the body after the fence. The
//! fork's `packages/skill` is the spec for the shape; the frontmatter
//! parse is hand-rolled on purpose — the subset is `key: value` lines
//! between `---` fences, plus the library's continuation blocks (a
//! `key:` whose indented or `- item` lines follow ride the key as one
//! opaque value), so a YAML dependency would buy nothing and cost a
//! crate. Fleet's own `skills/` under the fleet home holds only
//! proposal drafts.
//!
//! **Progressive disclosure.** Every run's system prompt carries the
//! available skills as a name+description list — the same seam the tool
//! declarations use — and a body enters context only when the loop
//! selects it, through the `use_skill` tool. The body rides the ordinary
//! `tool_result` path, so model-visible ⟺ logged falls out of the
//! existing seam with no new record type. Because the list is dynamic
//! (the files change between runs), each run records the skills it
//! declared as one session-log `skills` record at run start, exactly the
//! way a lane joining records `lane_tools`.
//!
//! **The self-improving half.** After a session, the drafter reads that
//! session's log plus the day's calibration rows, identifies a lesson —
//! a tool call that failed and was then corrected, a repeated multi-step
//! tool sequence, or a turn graded failed with a named class — and
//! drafts a SKILL.md creation or update into
//! `<FLEET_HOME>/skills/proposals/`. A proposal is never applied here:
//! applying one is slice 12's gate, whose apply path is what writes into
//! the canonical library.
//!
//! **Curation is usage-scan, not semantic recall.** Past session logs are
//! scanned for `use_skill` calls. The merge rule (descriptions near-
//! identical, token Jaccard) still drafts; the retirement rule is a
//! no-op against the canonical library — the scan reads fleet's sessions
//! only, and a skill no fleet session selected may be another harness's
//! daily driver.
//!
//! A skill whose frontmatter is missing or malformed is **skipped by
//! name** with one logged line naming it: a broken skill never blocks the
//! others, and never enters the model's view at all.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use crate::session::Record;

/// The directory under the fleet home holding proposal drafts. Live
/// skills live in the canonical library; this directory exists so the
/// drafts have a fleet-local staging ground the gate reads.
pub const SKILLS_DIR: &str = "skills";

/// The file every skill directory carries.
pub const SKILL_FILE: &str = "SKILL.md";

/// The directory under `skills/` holding drafted proposals.
pub const PROPOSALS_DIR: &str = "proposals";

/// How many sessions a skill may go unused across before curation
/// proposes retiring it. Configurable as `skills.retire_after_sessions`
/// in `<FLEET_HOME>/config.json`.
pub const DEFAULT_RETIRE_AFTER_SESSIONS: usize = 20;

/// The token-Jaccard similarity at which two skills' descriptions read as
/// near-identical and curation proposes a merge. Documented here and in
/// the feature doc; configurable as `skills.merge_similarity`.
pub const MERGE_SIMILARITY_THRESHOLD: f64 = 0.8;

/// How many lessons one session may draft. The evidence is finite and a
/// session that trips every detector should not flood the proposals
/// directory; the cap keeps the drafts reviewable.
pub const MAX_LESSONS_PER_SESSION: usize = 3;

// --- the loaded shapes -----------------------------------------------------

/// One live skill: its frontmatter, its body, and where it came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Skill {
    pub name: String,
    pub description: String,
    /// Everything after the frontmatter fence; what `use_skill` returns.
    pub body: String,
    pub path: PathBuf,
    /// The SKILL.md's mtime, for the surface's card. `None` when the
    /// metadata could not be read.
    pub modified: Option<SystemTime>,
}

/// One skill as the session log's `skills` record carries it: the two
/// facts the system prompt showed the model. The body is deliberately
/// absent — it enters context only when selected, and then it is already
/// in the log as that call's `tool_result`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillRef {
    pub name: String,
    pub description: String,
}

impl From<&Skill> for SkillRef {
    fn from(skill: &Skill) -> SkillRef {
        SkillRef {
            name: skill.name.clone(),
            description: skill.description.clone(),
        }
    }
}

/// What a proposal asks the gate to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProposalKind {
    /// A new skill.
    Create,
    /// A change to an existing skill.
    Update,
    /// Remove an unused skill.
    Retire,
    /// Fold one skill into another.
    Merge,
}

impl ProposalKind {
    /// The word the frontmatter and the surface carry.
    pub fn label(self) -> &'static str {
        match self {
            ProposalKind::Create => "create",
            ProposalKind::Update => "update",
            ProposalKind::Retire => "retire",
            ProposalKind::Merge => "merge",
        }
    }

    /// Every kind, in the order the surface groups them.
    pub const ALL: [ProposalKind; 4] = [
        ProposalKind::Create,
        ProposalKind::Update,
        ProposalKind::Retire,
        ProposalKind::Merge,
    ];
}

/// One drafted change awaiting the slice-12 gate. The file on disk is the
/// draft itself: frontmatter carrying `proposed: true` and the evidence,
/// then the proposed SKILL.md body. Nothing in this slice applies one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Proposal {
    pub name: String,
    pub description: String,
    pub kind: ProposalKind,
    /// The session id and the record or row that justifies the draft.
    pub evidence: String,
    pub body: String,
    pub path: PathBuf,
}

/// How a skill's usage was measured: the per-skill `use_skill` counts and
/// how many sessions the scan actually saw.
#[derive(Debug, Clone, Default)]
pub struct UsageScan {
    per_skill: HashMap<String, usize>,
    pub sessions: usize,
}

impl UsageScan {
    /// How many times the scan saw this skill selected.
    pub fn count(&self, name: &str) -> usize {
        self.per_skill.get(name).copied().unwrap_or(0)
    }

    /// How many of the scanned skills were never selected.
    pub fn unused(&self, skills: &[Skill]) -> usize {
        skills
            .iter()
            .filter(|skill| self.count(&skill.name) == 0)
            .count()
    }
}

// --- the frontmatter subset ------------------------------------------------

/// One parsed SKILL.md document: its frontmatter fields, in order, and
/// the body after the closing fence.
///
/// `pub(crate)`: slice 12's proposal files carry the same frontmatter
/// subset (KI-057's sibling-parser discipline stops here — the subsets
/// are identical, so the parser is shared rather than copied).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Document {
    pub(crate) fields: Vec<(String, String)>,
    pub(crate) body: String,
}

impl Document {
    pub(crate) fn get(&self, key: &str) -> Option<&str> {
        self.fields
            .iter()
            .find(|(name, _)| name == key)
            .map(|(_, value)| value.as_str())
    }

    pub(crate) fn required(&self, key: &str) -> Result<String, String> {
        self.get(key)
            .map(str::to_string)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| format!("frontmatter is missing a non-empty `{key}`"))
    }
}

/// Parse the frontmatter subset Fleet accepts: an opening `---` fence,
/// `key: value` lines, a closing `---` fence, then the body.
///
/// `pub(crate)`: slice 12's proposal files are the same subset, and one
/// parser serves both rather than a second copy that can drift.
///
/// The subset is deliberately small and the rejections are deliberate
/// too. Accepted: `key: value` pairs with surrounding whitespace
/// trimmed, keys and values optionally wrapped in matching single or
/// double quotes, blank lines inside the fence, and unknown keys
/// (forward compatibility — a future field must not invalidate today's
/// file). A leading UTF-8 BOM is stripped before anything else: editors
/// write them, and a BOM'd file is a valid file. Rejected, each with
/// the reason named: no opening fence, no closing fence, a fence line
/// that is not a `key: value` pair (a nested block's indent, a `- item`
/// list entry, a bare scalar), a key whose quotes do not balance, a key
/// that appears twice, an empty key, and an empty value. A BOM'd file
/// that still fails names the BOM in its reason, so the complaint is
/// not misdirected at the fence. The body is everything after the
/// closing fence with one leading newline taken off.
pub(crate) fn parse_document(text: &str) -> Result<Document, String> {
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
    // The key awaiting its block, and whether that block folds. YAML's
    // `>` opens a folded block — the continuation lines are one value
    // joined by spaces — and `|` a literal one. A writer using either is
    // writing ordinary YAML, and a parser that reads the indicator as the
    // value hands the model a description that begins `>`.
    let mut pending: Option<(String, bool)> = None;
    // The key whose block is currently being read, when that block folds.
    // The flag has to outlive the first continuation line: a folded block
    // is one value across all of its lines, not just its first.
    let mut folding: Option<String> = None;
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
        // The library's dialect: a `key:` whose block follows as indented
        // lines or `- item` entries. The block rides the key as one opaque
        // value — unknown keys are kept, and this is the kept form of a
        // nested one.
        if line.starts_with(' ') || line.starts_with('\t') || trimmed.starts_with('-') {
            let key = match pending.take() {
                Some((awaited, _)) => {
                    fields.push((awaited.clone(), String::new()));
                    awaited
                }
                None => match fields.last() {
                    Some((key, _)) => key.clone(),
                    None => {
                        return Err(format!(
                            "frontmatter line {} continues no key (a nested block or list entry must follow its `key:`)",
                            index + 1
                        ));
                    }
                },
            };
            let folded = folding.as_deref() == Some(key.as_str());
            if let Some((_, value)) = fields.iter_mut().find(|(seen, _)| *seen == key) {
                if !value.is_empty() {
                    value.push(if folded { ' ' } else { '\n' });
                }
                // A folded block is one value, so its continuation lines
                // lose the block's indentation on the way in — YAML
                // strips it, and a description that carries two leading
                // spaces reads as a mistake.
                let line = if folded { line.trim() } else { line.trim_end() };
                value.push_str(line);
            }
            continue;
        }
        let Some((key, value)) = trimmed.split_once(':') else {
            return Err(format!(
                "frontmatter line {} is not a `key: value` pair",
                index + 1
            ));
        };
        // Keys unquote the way values do: `"name": x` is `name: x`,
        // which is what a writer that quotes every key emits.
        folding = None;
        let raw = key.trim();
        let key = unquote(raw);
        if key.is_empty() {
            return Err(format!("frontmatter line {} has an empty key", index + 1));
        }
        // A key that still carries a quote after unquoting has
        // unbalanced ones — a typo. Name it here rather than letting it
        // read later as a missing required field with the key in plain
        // sight.
        if key.contains('"') || key.contains('\'') {
            return Err(format!(
                "frontmatter line {} has an unbalanced quoted key `{raw}`",
                index + 1
            ));
        }
        // A YAML block-scalar indicator is an empty value that opens a
        // block, exactly as a bare `key:` does — and it says whether that
        // block folds.
        let raw_value = unquote(value.trim());
        let folded = matches!(raw_value, ">" | ">-" | ">+");
        let literal = matches!(raw_value, "|" | "|-" | "|+");
        let value = if folded || literal {
            String::new()
        } else {
            raw_value.to_string()
        };
        // An empty value opens a continuation block, which the next
        // indented or `- item` line fills. A block that never comes is
        // the old empty-value error under its new, more precise name.
        if value.is_empty() {
            if let Some((awaited, _)) = &pending {
                return Err(format!(
                    "frontmatter key `{awaited}` has an empty value with no continuation block"
                ));
            }
            if fields.iter().any(|(seen, _)| seen == key) {
                return Err(format!("frontmatter key `{key}` appears twice"));
            }
            pending = Some((key.to_string(), folded));
            folding = folded.then(|| key.to_string());
            continue;
        }
        if let Some((awaited, _)) = &pending {
            return Err(format!(
                "frontmatter key `{awaited}` has an empty value with no continuation block"
            ));
        }
        if fields.iter().any(|(seen, _)| seen == key) {
            return Err(format!("frontmatter key `{key}` appears twice"));
        }
        fields.push((key.to_string(), value.to_string()));
    }
    if let Some((awaited, _)) = &pending {
        return Err(format!(
            "frontmatter key `{awaited}` has an empty value with no continuation block"
        ));
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

/// One skipped skill directory: its name and the loader's reason. A
/// broken skill is never silent — the *caller* decides where the report
/// goes (the CLI prints each line; the TUI renders one card), which is
/// what keeps bare stderr off a fixed-width pane (KI-216).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillSkip {
    pub name: String,
    pub reason: String,
}

// One stderr line naming the proposal that was skipped and why. The
// proposals tree is fleet-local and read only by CLI surfaces, where
// stderr is the right channel.
fn skip_line(name: &str, reason: &str) {
    eprintln!("fleet: skills: skipping `{name}` \u{2014} {reason}");
}

// --- loading ---------------------------------------------------------------

/// How long a prompt line's trigger may run before it is clipped at a
/// word boundary. The trigger is the match signal; the body arrives on
/// the call, and a prompt is not the place for the whole description.
const TRIGGER_CLIP: usize = 160;

/// The trigger a prompt line carries: the description with its leading
/// frame stripped and its first word lowered, so the line reads as a
/// match a model can make rather than as a library catalogue entry.
/// A description with no frame is lowered as it stands.
pub fn trigger(description: &str) -> String {
    let text = description.trim();
    let lowered = text.to_ascii_lowercase();
    let stripped = ["use when ", "use this ", "use for "]
        .iter()
        .find_map(|frame| lowered.strip_prefix(frame).map(|_| &text[frame.len()..]))
        .unwrap_or(text)
        .trim();
    let mut chars = stripped.chars();
    let trigger = match chars.next() {
        Some(first) => first.to_lowercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    };
    clip_words(&trigger, TRIGGER_CLIP)
}

/// The longest prefix of `text` that ends on a word boundary at or
/// before `limit` characters, with an ellipsis when it cut something.
/// A word longer than the limit cuts mid-word rather than losing the
/// line entirely.
fn clip_words(text: &str, limit: usize) -> String {
    if text.chars().count() <= limit {
        return text.to_string();
    }
    let mut boundary = None;
    for (seen, (index, ch)) in text.char_indices().enumerate() {
        if seen > limit {
            break;
        }
        if ch == ' ' {
            boundary = Some(index);
        }
    }
    let cut = boundary.unwrap_or_else(|| {
        text.char_indices()
            .nth(limit)
            .map(|(index, _)| index)
            .unwrap_or(text.len())
    });
    let mut clipped = text[..cut].trim_end().to_string();
    clipped.push('\u{2026}');
    clipped
}

/// `<home>/skills-retired.json`: the names the owner retired from the
/// prompt on the skills page.
const RETIRED_FILE: &str = "skills-retired.json";

/// The retired names, in the file's own order. An absent or malformed
/// file is an empty list — the same tolerance the recall index runs on.
pub fn retired(home: &Path) -> Vec<String> {
    std::fs::read_to_string(home.join(RETIRED_FILE))
        .ok()
        .and_then(|text| serde_json::from_str::<Vec<String>>(&text).ok())
        .unwrap_or_default()
}

/// The library as a run declares it: the loaded skills with the
/// fleet-local retired filtered out. The library is cross-harness and
/// is never written by this — a retired skill stays in it,
/// byte-identical, and leaves only what a run declares (KI-215).
pub fn load_declared(canonical: &Path, home: &Path) -> Vec<Skill> {
    let retired = retired(home);
    if retired.is_empty() {
        return load(canonical);
    }
    load(canonical)
        .into_iter()
        .filter(|skill| !retired.iter().any(|name| name == &skill.name))
        .collect()
}

/// The skills an unattended run declares: the same canonical library the
/// interactive paths load — `$FLEET_SKILLS_DIR` or the user home's
/// `.agents/skills` — falling back to the fleet home's own `skills/`
/// directory when the library is unresolvable (an isolated test home, a
/// container). A scheduled run and a delegation ride the declared list a
/// foreground run sees, instead of reading FLEET_HOME itself as a
/// library and finding none (KI-294). Both environment inputs are
/// parameters (the `fleet_home` pattern): the caller's boundary reads
/// its own environment.
pub fn load_unattended(env_dir: Option<&str>, user_home: Option<&str>, home: &Path) -> Vec<Skill> {
    if let Some(dir) = resolve_skills_dir(env_dir, user_home) {
        return load_declared(&dir, home);
    }
    let fallback = home.join(SKILLS_DIR);
    if fallback.is_dir() {
        load_declared(&fallback, home)
    } else {
        Vec::new()
    }
}

/// The canonical skills library: `$FLEET_SKILLS_DIR` when set and
/// non-empty, else `<user home>/.agents/skills` — the owner's
/// cross-harness library, so a skill is authored once and every harness
/// loads the same source. Both inputs are parameters (the `fleet_home`
/// pattern: the caller's boundary decides its own environment
/// precedence). `None` when neither is known, and the caller names it.
pub fn resolve_skills_dir(env_value: Option<&str>, user_home: Option<&str>) -> Option<PathBuf> {
    if let Some(dir) = env_value.filter(|value| !value.is_empty()) {
        return Some(PathBuf::from(dir));
    }
    user_home.map(|home| Path::new(home).join(".agents").join("skills"))
}

/// `<home>/skills/proposals` — the fleet home's own draft staging. Live
/// skills live in the canonical library; a proposal is fleet's working
/// state and never enters it unapplied.
pub fn proposals_dir(home: &Path) -> PathBuf {
    home.join(SKILLS_DIR).join(PROPOSALS_DIR)
}

/// Load every live skill under the canonical library directory, sorted
/// by name, printing one stderr line per skip — the headless and CLI
/// surfaces' shape. A surface that renders (the TUI) wants
/// [`load_reported`] instead and decides where the report goes.
pub fn load(canonical: &Path) -> Vec<Skill> {
    let (skills, skips) = load_reported(canonical);
    for skip in &skips {
        eprintln!(
            "fleet: skills: skipping `{}` \u{2014} {}",
            skip.name, skip.reason
        );
    }
    skills
}

/// Load every live skill under the canonical library directory, sorted
/// by name, with the skips **reported, not printed**: a missing
/// directory is no skills and no skips, not an error. A directory
/// without a readable `SKILL.md`, or with a malformed one, comes back
/// as a [`SkillSkip`]; the others still load. A live skill whose
/// frontmatter carries `proposed: true` is skipped — that is a proposal
/// that somehow reached the live tree, and the loader refuses to
/// promote it silently.
pub fn load_reported(canonical: &Path) -> (Vec<Skill>, Vec<SkillSkip>) {
    let dir = canonical;
    let Ok(entries) = fs::read_dir(dir) else {
        return (Vec::new(), Vec::new());
    };
    let mut directories: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();
    directories.sort();
    let mut skills: Vec<Skill> = Vec::new();
    let mut skips: Vec<SkillSkip> = Vec::new();
    for path in directories {
        // Owned, so the path itself stays free to move into the skill.
        let Some(name) = path
            .file_name()
            .and_then(|name| name.to_str())
            .map(str::to_string)
        else {
            continue;
        };
        let file = path.join(SKILL_FILE);
        let text = match fs::read_to_string(&file) {
            Ok(text) => text,
            Err(_) => {
                skips.push(SkillSkip {
                    name,
                    reason: format!("no readable {SKILL_FILE}"),
                });
                continue;
            }
        };
        let modified = fs::metadata(&file).and_then(|meta| meta.modified()).ok();
        match parse_document(&text).and_then(|document| live_skill(document, &name, file, modified))
        {
            Ok(skill) => skills.push(skill),
            Err(reason) => skips.push(SkillSkip { name, reason }),
        }
    }
    skills.sort_by(|left, right| left.name.cmp(&right.name));
    (skills, skips)
}

/// The live-skill rules on top of the shared parse: `name` and
/// `description` are required, the frontmatter's `name` must equal the
/// directory's (one skill, one name), and `proposed: true` is refused.
fn live_skill(
    document: Document,
    directory: &str,
    path: PathBuf,
    modified: Option<SystemTime>,
) -> Result<Skill, String> {
    let name = document.required("name")?;
    if name != directory {
        return Err(format!(
            "frontmatter name `{name}` does not match the directory `{directory}`"
        ));
    }
    let description = document.required("description")?;
    if document.get("proposed") == Some("true") {
        return Err(
            "carries `proposed: true` \u{2014} that is a proposal, not a live skill".to_string(),
        );
    }
    Ok(Skill {
        name,
        description,
        body: document.body,
        path,
        modified,
    })
}

/// Load every proposal under `<home>/skills/proposals`, sorted by name.
/// Same discipline as [`load`]: a malformed draft is skipped by name
/// with one logged line, and the others still list.
pub fn load_proposals(home: &Path) -> Vec<Proposal> {
    let dir = proposals_dir(home);
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut files: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("md"))
        .collect();
    files.sort();
    let mut proposals: Vec<Proposal> = Vec::new();
    for path in files {
        // Owned, so the path itself stays free to move into the proposal.
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
                skip_line(&stem, "no readable proposal file");
                continue;
            }
        };
        match parse_document(&text).and_then(|document| proposal(document, &stem, path)) {
            Ok(proposal) => proposals.push(proposal),
            Err(reason) => skip_line(&stem, &reason),
        }
    }
    proposals.sort_by(|left, right| left.name.cmp(&right.name));
    proposals
}

/// The proposal rules on top of the shared parse: `name`, `description`,
/// `proposed: true`, a known `kind`, and the `evidence` line are all
/// required, and the frontmatter's `name` must equal the file's stem.
fn proposal(document: Document, stem: &str, path: PathBuf) -> Result<Proposal, String> {
    let name = document.required("name")?;
    if name != stem {
        return Err(format!(
            "frontmatter name `{name}` does not match the file `{stem}.md`"
        ));
    }
    let description = document.required("description")?;
    if document.get("proposed") != Some("true") {
        return Err("frontmatter is missing `proposed: true`".to_string());
    }
    let kind = match document.get("kind") {
        Some("create") => ProposalKind::Create,
        Some("update") => ProposalKind::Update,
        Some("retire") => ProposalKind::Retire,
        Some("merge") => ProposalKind::Merge,
        Some(other) => {
            return Err(format!("unknown proposal kind `{other}`"));
        }
        None => return Err("frontmatter is missing a `kind`".to_string()),
    };
    let evidence = document.required("evidence")?;
    Ok(Proposal {
        name,
        description,
        kind,
        evidence,
        body: document.body,
        path,
    })
}

// --- the use_skill tool ----------------------------------------------------

/// The skill name one `use_skill` call asks for, for the approval prompt
/// and the progress line. `None` when the arguments do not carry one.
pub fn requested_skill(arguments: &str) -> Option<String> {
    let parsed: serde_json::Value = serde_json::from_str(arguments).ok()?;
    parsed
        .get("name")
        .and_then(|name| name.as_str())
        .map(str::to_string)
        .filter(|name| !name.is_empty())
}

/// The `use_skill` tool's result for one call: the selected skill's body,
/// or a typed error the model can read and react to. Never a panic, and
/// never an empty answer.
pub fn use_skill_result(skills: &[Skill], arguments: &serde_json::Value) -> String {
    let Some(name) = arguments
        .get("name")
        .and_then(|name| name.as_str())
        .filter(|name| !name.is_empty())
    else {
        return "error: the `name` argument is required and must be a string".to_string();
    };
    match skills.iter().find(|skill| skill.name == name) {
        Some(skill) => skill.body.clone(),
        None => {
            // A name the run never declared may be a retired skill: the
            // named result tells the model the rule and the reverse,
            // not a bare "unknown" that reads as a typo.
            let retired = RETIRED_DISCLOSE
                .lock()
                .map(|disclosed| disclosed.iter().any(|held| held == name))
                .unwrap_or(false);
            if retired {
                format!(
                    "disabled: the skill `{name}` is retired fleet-locally \u{2014} no use over the \
                     measured window. The owner's `fleet skills` card enables it; nothing loaded."
                )
            } else {
                format!("error: no skill named `{name}`")
            }
        }
    }
}

/// The retired names a running process has already read, set once at
/// run start beside the skills list. `use_skill` consults it so a
/// disabled skill's refusal is the rule, not a typo.
pub static RETIRED_DISCLOSE: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());

/// Load the retired names once, at run start, so `use_skill` can name
/// the rule instead of guessing.
pub fn note_retired(home: &Path) {
    if let Ok(mut disclosed) = RETIRED_DISCLOSE.lock() {
        *disclosed = retired(home);
    }
}

// --- the usage scan --------------------------------------------------------

/// Scan the last `sessions` session logs for `use_skill` calls, counting
/// selections per skill. The count comes from the assistant's call
/// arguments joined to that call's `tool_result`: a result is the body,
/// which names no skill, and a call the user denied loaded nothing — so
/// a denied selection is not a use.
///
/// This is the pre-slice-8 measure of use — a log scan, not recall.
pub fn scan_usage(home: &Path, sessions: usize) -> UsageScan {
    let mut scan = UsageScan::default();
    let Ok(summaries) = crate::session::list(home) else {
        return scan;
    };
    // `list` is oldest first; the window is the most recent `sessions`.
    // KI-056: `list` reads each log's head and tail through
    // `session::summarize` rather than materializing every record, so
    // the summaries no longer pay a full replay of every log in the
    // home — the window's replay below is the only one.
    for summary in summaries.iter().rev().take(sessions) {
        scan.sessions += 1;
        let Ok(records) = crate::session::replay(&summary.path) else {
            continue;
        };
        // Which calls were approved, by id: the join that makes the
        // count "the body entered context" rather than "the model
        // asked".
        let mut approved: HashMap<String, bool> = HashMap::new();
        for record in &records {
            if let Record::ToolResult {
                call_id,
                approved: ok,
                ..
            } = record
            {
                approved.insert(call_id.clone(), *ok);
            }
        }
        for record in &records {
            let Record::Assistant { tool_calls, .. } = record else {
                continue;
            };
            for call in tool_calls {
                if call.function.name != crate::tools::USE_SKILL_TOOL {
                    continue;
                }
                if !approved.get(&call.id).copied().unwrap_or(false) {
                    continue;
                }
                if let Some(name) = requested_skill(&call.function.arguments) {
                    *scan.per_skill.entry(name).or_insert(0) += 1;
                }
            }
        }
    }
    scan
}

// --- similarity ------------------------------------------------------------

/// The normalized token set of `text`: lowercased, split on every
/// non-alphanumeric run, deduplicated.
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

/// Token Jaccard over two descriptions: the shared tokens over the union.
/// An empty token set on either side scores 0 — two descriptions with no
/// words in common with anything are not similar, they are absent.
pub fn description_similarity(left: &str, right: &str) -> f64 {
    TokenSet::of(left).similarity(&TokenSet::of(right))
}

/// One description's normalized token set, sorted and deduplicated.
///
/// The seam a surface that scores many pairs against one library needs:
/// a library of four hundred skills is eighty thousand pairs, and a scan
/// that re-tokenizes — or re-scans — per pair does not land inside a
/// refresh interval. Each description is tokenized once into one of
/// these, and the pairs are scored by a merge over sorted sets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenSet {
    sorted: Vec<String>,
}

impl TokenSet {
    /// The token set of one description.
    pub fn of(text: &str) -> TokenSet {
        let mut sorted = tokens(text);
        sorted.sort();
        TokenSet { sorted }
    }

    /// How many distinct tokens this set carries.
    pub fn len(&self) -> usize {
        self.sorted.len()
    }

    /// Whether this set carries no tokens at all.
    pub fn is_empty(&self) -> bool {
        self.sorted.is_empty()
    }

    /// The Jaccard against another set: the shared tokens over the union,
    /// read by a merge over the two sorted sets. An empty set on either
    /// side scores 0.
    pub fn similarity(&self, other: &TokenSet) -> f64 {
        if self.sorted.is_empty() || other.sorted.is_empty() {
            return 0.0;
        }
        let mut left = self.sorted.iter().peekable();
        let mut right = other.sorted.iter().peekable();
        let mut shared = 0usize;
        while let (Some(a), Some(b)) = (left.peek(), right.peek()) {
            match a.cmp(b) {
                std::cmp::Ordering::Equal => {
                    shared += 1;
                    left.next();
                    right.next();
                }
                std::cmp::Ordering::Less => {
                    left.next();
                }
                std::cmp::Ordering::Greater => {
                    right.next();
                }
            }
        }
        let union = self.sorted.len() + other.sorted.len() - shared;
        shared as f64 / union as f64
    }
}

// --- the drafter -----------------------------------------------------------

/// One lesson a session's evidence justifies, before it is filed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lesson {
    pub kind: ProposalKind,
    pub name: String,
    pub description: String,
    pub evidence: String,
    pub body: String,
}

impl Lesson {
    /// This lesson as an update to an existing skill: the kind flips, the
    /// description stays the live skill's (the proposal replaces the
    /// file, so its frontmatter must stay the skill's), and the body is
    /// the live body plus the drafted addition — a diff a reviewer can
    /// read.
    fn into_update(self, existing: &Skill) -> Lesson {
        let addition = format!(
            "\n\n## Proposed addition (drafted {})\n\n{}\n\nEvidence: {}\n",
            crate::calib::today(),
            self.description,
            self.evidence,
        );
        Lesson {
            kind: ProposalKind::Update,
            description: existing.description.clone(),
            evidence: self.evidence,
            body: format!("{}{}", existing.body.trim_end(), addition),
            name: self.name,
        }
    }
}

/// The lessons `records` and the day's calibration `rows` justify, at most
/// [`MAX_LESSONS_PER_SESSION`], in a fixed order: a failure that was
/// corrected, a repeated multi-step tool sequence, a turn graded failed
/// with a named class. `session_id` rides in every evidence line.
///
/// Deterministic by construction: the same log and the same rows draft
/// the same lessons, which is what makes the drafter testable without a
/// model. The bodies are structured drafts carrying their evidence, not
/// model-written prose.
pub fn lessons(
    records: &[Record],
    rows: &[crate::calib::ReadRow],
    session_id: &str,
) -> Vec<Lesson> {
    let mut found: Vec<Lesson> = Vec::new();
    if let Some(lesson) = failure_correction(records, session_id) {
        found.push(lesson);
    }
    if let Some(lesson) = repeated_sequence(records, session_id) {
        found.push(lesson);
    }
    if let Some(lesson) = failed_turn(records, rows, session_id) {
        found.push(lesson);
    }
    found.truncate(MAX_LESSONS_PER_SESSION);
    found
}

/// One executed tool call, resolved against the assistant turn that
/// requested it.
struct Executed {
    name: String,
    /// The arguments the requesting turn sent, for the evidence line.
    arguments: String,
    output: String,
    /// Whether the call was approved. A denied call produced no result
    /// of its own — its output is the denial — so it is evidence of
    /// nothing the tool did.
    approved: bool,
    /// The tool_result record's index in the log, for the evidence line.
    record: usize,
}

/// Every executed call in the log, in order, with the tool each result
/// belongs to. The call id is the join; the result record's own `name`
/// is the fallback.
fn executed_calls(records: &[Record]) -> Vec<Executed> {
    let mut requested: HashMap<String, (String, String)> = HashMap::new();
    let mut executed: Vec<Executed> = Vec::new();
    for (index, record) in records.iter().enumerate() {
        match record {
            Record::Assistant { tool_calls, .. } => {
                for call in tool_calls {
                    requested.insert(
                        call.id.clone(),
                        (call.function.name.clone(), call.function.arguments.clone()),
                    );
                }
            }
            Record::ToolResult {
                call_id,
                name,
                output,
                approved,
                ..
            } => {
                let (resolved, arguments) = requested
                    .get(call_id)
                    .cloned()
                    .unwrap_or_else(|| (name.clone(), String::new()));
                executed.push(Executed {
                    name: resolved,
                    arguments,
                    output: output.clone(),
                    approved: *approved,
                    record: index,
                });
            }
            _ => {}
        }
    }
    executed
}

/// The call ids whose result was approved, by id. A call with no result
/// record — a log that ends mid-turn — counts as not approved.
fn approved_calls(records: &[Record]) -> HashMap<String, bool> {
    let mut approved: HashMap<String, bool> = HashMap::new();
    for record in records {
        if let Record::ToolResult {
            call_id,
            approved: ok,
            ..
        } = record
        {
            approved.insert(call_id.clone(), *ok);
        }
    }
    approved
}

/// How wide a compacted argument summary or output line may run in an
/// evidence line. Long enough for a real command, short enough that one
/// evidence line stays one line.
const COMPACT_WIDTH: usize = 80;

/// One call as an evidence line names it: the tool, then a compacted
/// argument summary in parentheses. Self-containment is the point — the
/// owner judges the pair at the gate, not the log behind it.
fn call_label(call: &Executed) -> String {
    let arguments = compact_arguments(&call.arguments);
    if arguments.is_empty() {
        format!("`{}`", call.name)
    } else {
        format!("`{}` ({arguments})", call.name)
    }
}

/// A compacted one-line summary of one call's arguments: a JSON
/// object's `key: value` pairs joined, string values unquoted, bounded
/// to [`COMPACT_WIDTH`]. Anything that is not an object — `{}`, a bare
/// string, invalid JSON — is used raw and bounded the same way.
fn compact_arguments(arguments: &str) -> String {
    let rendered = match serde_json::from_str::<serde_json::Value>(arguments) {
        Ok(serde_json::Value::Object(fields)) => fields
            .iter()
            .map(|(key, value)| match value {
                serde_json::Value::String(text) => format!("{key}: {text}"),
                other => format!("{key}: {other}"),
            })
            .collect::<Vec<_>>()
            .join(", "),
        _ => arguments.trim().to_string(),
    };
    compact(&rendered, COMPACT_WIDTH)
}

/// `text` on one bounded line: the first line, cut to `width`
/// characters with an ellipsis when longer. An evidence line is one
/// line by construction — a multi-line or unbounded output would flood
/// the proposal's frontmatter.
fn compact(text: &str, width: usize) -> String {
    let first = text.lines().next().unwrap_or_default();
    let mut characters = first.chars();
    let head: String = characters.by_ref().take(width).collect();
    if characters.next().is_some() {
        format!("{head}\u{2026}")
    } else {
        head
    }
}

/// The failure→correction lesson: one tool that failed with a typed
/// `error:` result and was later called again successfully. The first
/// such pair in the log is the lesson; the correction is the evidence
/// that the failure was understood, not just survived.
///
/// A failure with no correction is not the lesson and does not end the
/// search — the next failure in the log may be the one that was
/// understood. Only approved calls count on either side: a denied retry
/// produced no result of its own, so it corrects nothing.
///
/// The evidence line is self-contained: the tool name, a compacted
/// argument summary on both sides, and both record numbers, so the
/// owner judges the pair at the gate without replaying the log.
fn failure_correction(records: &[Record], session_id: &str) -> Option<Lesson> {
    let calls = executed_calls(records);
    for (index, failed) in calls.iter().enumerate() {
        if !failed.approved || !failed.output.starts_with("error:") {
            continue;
        }
        let Some(corrected) = calls[index + 1..].iter().find(|call| {
            call.approved && call.name == failed.name && !call.output.starts_with("error:")
        }) else {
            continue;
        };
        let name = format!("{}-recovery", slug(&failed.name));
        let description = format!(
            "Recover a failed `{}` call: read the typed error, correct one thing, and rerun the same tool.",
            failed.name
        );
        let evidence = format!(
            "session {session_id}: {failed_call} failed at record {failed_record} with `{failure}`; the approved rerun {corrected_call} succeeded at record {corrected_record}",
            failed_call = call_label(failed),
            failed_record = failed.record + 1,
            failure = compact(&failed.output, COMPACT_WIDTH),
            corrected_call = call_label(corrected),
            corrected_record = corrected.record + 1,
        );
        let body = format!(
            "# {name}\n\n{description}\n\n## When to reach for it\n\nA `{tool}` call has just come back with a typed `error:` result.\n\n## The lesson\n\nRead the error before retrying. The failure text names what was wrong, so the corrected call is a fix rather than a repeat: change one thing, then run the same tool again.\n\n## Evidence\n\n{evidence}\n",
            name = name,
            description = description,
            tool = failed.name,
            evidence = evidence,
        );
        return Some(Lesson {
            kind: ProposalKind::Create,
            name,
            description,
            evidence,
            body,
        });
    }
    None
}

/// The repeated-sequence lesson: one assistant turn that requested two or
/// more tools, whose exact ordered sequence appeared in another turn of
/// the same session. A recurring multi-step procedure is a skill the
/// session kept re-deriving.
///
/// A turn counts only when every call it requested was approved: a
/// denied call means the procedure never ran, and a sequence of denials
/// is not a procedure the session kept re-deriving.
fn repeated_sequence(records: &[Record], session_id: &str) -> Option<Lesson> {
    let approved = approved_calls(records);
    let mut seen: Vec<(Vec<String>, Vec<usize>)> = Vec::new();
    for (index, record) in records.iter().enumerate() {
        let Record::Assistant { tool_calls, .. } = record else {
            continue;
        };
        if tool_calls.len() < 2 {
            continue;
        }
        if !tool_calls
            .iter()
            .all(|call| approved.get(&call.id).copied().unwrap_or(false))
        {
            continue;
        }
        let names: Vec<String> = tool_calls
            .iter()
            .map(|call| call.function.name.clone())
            .collect();
        match seen.iter_mut().find(|(sequence, _)| *sequence == names) {
            Some((_, turns)) => turns.push(index),
            None => seen.push((names, vec![index])),
        }
    }
    let (names, turns) = seen.into_iter().find(|(_, turns)| turns.len() >= 2)?;
    let name = format!("procedure-{}", slug(&names.join("-")));
    let arrow = names.join(" \u{2192} ");
    let description = format!(
        "The recurring {} step procedure `{}` as one skill instead of {} separate decisions.",
        names.len(),
        arrow,
        turns.len()
    );
    let records_at = turns.iter().map(|index| index + 1).collect::<Vec<_>>();
    let evidence = format!(
        "session {session_id}: the sequence `{arrow}` ran {} times (records {})",
        turns.len(),
        records_at
            .iter()
            .map(|index| index.to_string())
            .collect::<Vec<_>>()
            .join(", ")
    );
    let body = format!(
        "# {name}\n\n{description}\n\n## When to reach for it\n\nThe task needs these steps in this order: {arrow}.\n\n## The lesson\n\nThis sequence recurred within one session. Treat it as one procedure with one intent, not as a series of independent calls: decide the sequence once, then run it.\n\n## Evidence\n\n{evidence}\n",
        name = name,
        description = description,
        arrow = arrow,
        evidence = evidence,
    );
    Some(Lesson {
        kind: ProposalKind::Create,
        name,
        description,
        evidence,
        body,
    })
}

/// The failed-turn lesson: a `turn` row graded `failed` with a named
/// class, joined to this session by the task's hash. The row is the
/// audit spine's own grade of the run; the join is what makes the
/// evidence this session's rather than the day's.
fn failed_turn(
    records: &[Record],
    rows: &[crate::calib::ReadRow],
    session_id: &str,
) -> Option<Lesson> {
    let task_hashes: Vec<String> = records
        .iter()
        .filter_map(|record| match record {
            Record::User { content } => Some(crate::calib::hash16(content)),
            _ => None,
        })
        .collect();
    let row = rows.iter().find(|row| {
        row.op == crate::calib::OP_TURN
            && row.outcome == "failed"
            && !row.reason.is_empty()
            && task_hashes.iter().any(|hash| hash == &row.input_hash)
    })?;
    let name = format!("turn-failure-{}", slug(&row.reason));
    let description = format!(
        "A run ending `failed` with the `{}` class: what the class means and the first thing to check.",
        row.reason
    );
    let evidence = format!(
        "session {session_id}; calibration turn row {} graded failed/{} (task hash {})",
        row.ts, row.reason, row.input_hash
    );
    let body = format!(
        "# {name}\n\n{description}\n\n## When to reach for it\n\nA run is ending `failed` and the calibration row names the `{reason}` class.\n\n## The lesson\n\nThe class is the router's own failure classification, so it names the layer that broke. Check that layer first, and record what the check found rather than retrying blind.\n\n## Evidence\n\n{evidence}\n",
        name = name,
        description = description,
        reason = row.reason,
        evidence = evidence,
    );
    Some(Lesson {
        kind: ProposalKind::Create,
        name,
        description,
        evidence,
        body,
    })
}

/// A filesystem-safe name: lowercased alphanumerics, every other run as
/// one dash, bounded so a long tool sequence cannot name an absurd file.
fn slug(text: &str) -> String {
    let mut out = String::new();
    for character in text.chars() {
        if character.is_ascii_alphanumeric() {
            out.push(character.to_ascii_lowercase());
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    let slug: String = out.trim_matches('-').chars().take(48).collect();
    slug.trim_matches('-').to_string()
}

/// Render one proposal's file text: the frontmatter the loader reads
/// back, then the body.
fn render_proposal(lesson: &Lesson) -> String {
    format!(
        "---\nname: {name}\ndescription: {description}\nproposed: true\nkind: {kind}\nevidence: {evidence}\n---\n\n{body}",
        name = lesson.name,
        description = lesson.description,
        kind = lesson.kind.label(),
        evidence = lesson.evidence,
        body = lesson.body,
    )
}

/// File one lesson as a proposal, unless a proposal with that name is
/// already waiting: a lesson stays as first drafted, and the gate
/// disposes of it. Returns the path written, or `None` when nothing was.
///
/// The write lands in `<home>/skills/proposals/` and nowhere else — this
/// is the only writer in the slice, and the live tree is never its
/// target.
fn file_proposal(home: &Path, lesson: &Lesson, source: &str) -> Option<PathBuf> {
    let dir = proposals_dir(home);
    let path = dir.join(format!("{}.md", lesson.name));
    if path.exists() {
        // KI-053: a lesson already drafted is not skipped — the new
        // session's evidence accumulates onto the waiting draft, named
        // with its source, so a second session's evidence for the same
        // lesson reaches the gate that disposes of it. The first
        // draft's frontmatter stands; the later evidence rides a
        // section a reviewer reads beside it.
        let existing = fs::read_to_string(&path).ok()?;
        let appended = format!(
            "{existing}\n\n## Further evidence\n\n{evidence} (from {source})\n",
            evidence = lesson.evidence,
        );
        fs::write(&path, appended).ok()?;
        return Some(path);
    }
    fs::create_dir_all(&dir).ok()?;
    fs::write(&path, render_proposal(lesson)).ok()?;
    Some(path)
}

/// Draft the lessons one session's evidence justifies and file them as
/// proposals. Best-effort throughout: an unreadable log, an absent
/// stream, or an unwritable proposals directory produce no drafts and no
/// failure — drafting is the harness noticing, never the run's outcome.
///
/// Returns the paths filed. A lesson whose name matches a live skill is
/// filed as an update to that skill rather than a create, so the draft
/// diffs against the file it would replace.
pub fn draft_session(
    canonical: &Path,
    home: &Path,
    session_path: &Path,
    user_home: Option<&str>,
) -> Vec<PathBuf> {
    let Ok(records) = crate::session::replay(session_path) else {
        return Vec::new();
    };
    if records.is_empty() {
        return Vec::new();
    }
    let session_id = session_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or_default()
        .to_string();
    let live = load(canonical);
    let root = crate::calib::resolve_root(home, user_home);
    let rows = crate::calib::read_day(&root, &crate::calib::today(), crate::calib::READ_CAP);
    let mut filed: Vec<PathBuf> = Vec::new();
    for lesson in lessons(&records, &rows, &session_id) {
        let lesson = match live.iter().find(|skill| skill.name == lesson.name) {
            Some(existing) => lesson.into_update(existing),
            None => lesson,
        };
        if let Some(path) = file_proposal(home, &lesson, &session_id) {
            filed.push(path);
        }
    }
    filed
}

// --- curation --------------------------------------------------------------

/// Run the usage-scan curation and file what it finds: merge proposals
/// for pairs of skills whose descriptions are at least
/// `merge_similarity` similar. Returns the paths filed.
///
/// The retirement rule is a **no-op against the canonical library** and
/// one logged line says so whenever the window is long enough to have
/// judged: the usage scan reads fleet's own session logs only, and a
/// skill no fleet session selected may be another harness's daily
/// driver — "unused by fleet" is not unused across harnesses (KI-215).
/// A hand-filed retirement proposal still goes through the gate; what
/// stopped is the automatic drafting. A skill that is the absorbed half
/// of a merge pair was never separately proposed, and the merge rule's
/// evidence is the descriptions themselves, so it composes with the
/// shared library fine.
pub fn curate(
    canonical: &Path,
    home: &Path,
    retire_after: usize,
    merge_similarity: f64,
) -> Vec<PathBuf> {
    let live = load(canonical);
    let usage = scan_usage(home, retire_after);
    let mut lessons: Vec<Lesson> = Vec::new();
    for (index, left) in live.iter().enumerate() {
        for right in &live[index + 1..] {
            let similarity = description_similarity(&left.description, &right.description);
            if similarity >= merge_similarity {
                let lesson =
                    merge_lesson(left, right, similarity, merge_similarity, usage.sessions);
                lessons.push(lesson);
            }
        }
    }
    if usage.sessions >= retire_after {
        eprintln!(
            "fleet: skills: retirement curation skipped \u{2014} the usage scan reads fleet's sessions only and the canonical library is cross-harness (KI-215)"
        );
    }
    let mut filed: Vec<PathBuf> = Vec::new();
    for lesson in &lessons {
        // The curation's source is the usage scan, not one session.
        if let Some(path) = file_proposal(home, lesson, "a later usage scan") {
            filed.push(path);
        }
    }
    filed
}

/// The merge lesson for one near-identical pair. The survivor is the
/// alphabetically first name; the other is named in the evidence as the
/// one to absorb.
fn merge_lesson(
    left: &Skill,
    right: &Skill,
    similarity: f64,
    threshold: f64,
    sessions: usize,
) -> Lesson {
    let (survivor, absorbed) = if left.name <= right.name {
        (left, right)
    } else {
        (right, left)
    };
    let evidence = format!(
        "usage scan over {sessions} sessions: `{}` and `{}` descriptions score {similarity:.2} token Jaccard (threshold {threshold:.2})",
        survivor.name, absorbed.name
    );
    let description = format!(
        "Merge `{}` into `{}`: the two descriptions are {similarity:.2} similar, so they read as one skill.",
        absorbed.name, survivor.name
    );
    let body = format!(
        "# merge: {absorbed} into {survivor}\n\n{description}\n\n## Proposed merge\n\n`{absorbed}`: {absorbed_description}\n\n`{survivor}`: {survivor_description}\n\nThe descriptions share most of their tokens. Two skills a reader cannot tell apart are one skill with a stale name.\n\n## Evidence\n\n{evidence}\n\n## What applying this means\n\nSlice 12's gate decides. Until it does, both live files stay on disk untouched and this draft is the record of what was proposed and why.\n",
        survivor = survivor.name,
        absorbed = absorbed.name,
        description = description,
        absorbed_description = absorbed.description,
        survivor_description = survivor.description,
        evidence = evidence,
    );
    Lesson {
        kind: ProposalKind::Merge,
        // KI-054: the draft is named by the PAIR, not the survivor — a
        // retire draft for the survivor and a merge draft for it used to
        // target one file, and first-filed won. The pair is the merge's
        // own identity; the gate reads the body for which half absorbs.
        name: format!("merge-{}-{}", absorbed.name, survivor.name),
        description,
        evidence,
        body,
    }
}

// --- the surface -----------------------------------------------------------

mod palette {
    pub const ACCENT: &str = "\u{1b}[36m";
    pub const OK: &str = "\u{1b}[32m";
    pub const WARN: &str = "\u{1b}[33m";
    pub const DIM: &str = "\u{1b}[90m";
    pub const TEXT: &str = "\u{1b}[37m";
    pub const BOLD: &str = "\u{1b}[1m";
    pub const RESET: &str = "\u{1b}[0m";
}

use palette::{ACCENT, BOLD, DIM, RESET, TEXT};

/// The card width, matching `lanes`, `providers`, and `calib`.
const CARD_WIDTH: usize = 72;

/// Render the live skills as cards: one per skill with its description,
/// its usage count from the scan, and the file's mtime. Never a
/// plain-text table.
pub fn render(skills: &[Skill], usage: &UsageScan) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "{BOLD}fleet skills{RESET} {DIM}\u{2014} {} loaded from {}{RESET}\n\n",
        skills.len(),
        SKILLS_DIR,
    ));
    if skills.is_empty() {
        out.push_str(&format!(
            "{DIM}no skills yet \u{b7} a skill is {SKILLS_DIR}/<name>/{SKILL_FILE}{RESET}\n"
        ));
        return out;
    }
    for skill in skills {
        out.push_str(&render_card(skill, usage));
    }
    if usage.sessions == 0 {
        out.push_str(&format!(
            "\n{DIM}{} skills \u{b7} usage unscanned (no sessions yet){RESET}\n",
            skills.len(),
        ));
        return out;
    }
    out.push_str(&format!(
        "\n{DIM}{} skills \u{b7} {} used \u{b7} {} unused in the last {} session{}{RESET}\n",
        skills.len(),
        skills.len() - usage.unused(skills),
        usage.unused(skills),
        usage.sessions,
        if usage.sessions == 1 { "" } else { "s" },
    ));
    out
}

/// One skill's card.
fn render_card(skill: &Skill, usage: &UsageScan) -> String {
    let uses = usage.count(&skill.name);
    let uses_color = if uses == 0 {
        palette::WARN
    } else {
        palette::OK
    };
    let header_right = format!("{uses} use{}", if uses == 1 { "" } else { "s" });
    let header = format!(
        "{DIM}\u{250c}\u{2500}{RESET} {BOLD}{ACCENT}{name}{RESET} {DIM}{dashes} {uses_color}{right}{RESET} {DIM}\u{2510}{RESET}",
        name = skill.name,
        dashes = dashes_for(&format!(" {} ", skill.name), &header_right),
        right = header_right,
    );
    let mut lines: Vec<String> = Vec::new();
    for chunk in wrap_detail(&skill.description, CARD_WIDTH - 4) {
        lines.push(format!("{DIM}\u{2502}{RESET} {TEXT}{chunk}{RESET}"));
    }
    // The file, as the last two path components: `<name>/SKILL.md` names
    // it without the machine's home prefix filling the card.
    let relative: String = skill
        .path
        .components()
        .rev()
        .take(2)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/");
    lines.push(format!("{DIM}\u{2502}{RESET} {DIM}{relative}{RESET}"));
    if let Some(modified) = skill.modified {
        let stamp = crate::session::iso_utc_stamp(modified).unwrap_or_default();
        lines.push(format!("{DIM}\u{2502}{RESET} {DIM}modified {stamp}{RESET}"));
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

/// Render the proposals awaiting the gate as cards: one per draft with
/// its kind and its evidence line.
pub fn render_proposals(proposals: &[Proposal]) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "{BOLD}fleet skills --proposed{RESET} {DIM}\u{2014} {} draft{} awaiting the slice-12 gate{RESET}\n\n",
        proposals.len(),
        if proposals.len() == 1 { "" } else { "s" },
    ));
    if proposals.is_empty() {
        out.push_str(&format!(
            "{DIM}no drafts yet \u{b7} a session's lessons are drafted after it ends{RESET}\n"
        ));
        return out;
    }
    for proposal in proposals {
        out.push_str(&render_proposal_card(proposal));
    }
    out.push_str(&format!(
        "\n{DIM}applying a draft is slice 12's gate \u{b7} `fleet skills --show <name>` prints one{RESET}\n"
    ));
    out
}

/// One proposal's card.
fn render_proposal_card(proposal: &Proposal) -> String {
    let header_right = format!("{} \u{b7} proposed", proposal.kind.label());
    let header = format!(
        "{DIM}\u{250c}\u{2500}{RESET} {BOLD}{ACCENT}{name}{RESET} {DIM}{dashes} {TEXT}{right}{RESET} {DIM}\u{2510}{RESET}",
        name = proposal.name,
        dashes = dashes_for(&format!(" {} ", proposal.name), &header_right),
        right = header_right,
    );
    let mut lines: Vec<String> = Vec::new();
    for chunk in wrap_detail(&proposal.description, CARD_WIDTH - 4) {
        lines.push(format!("{DIM}\u{2502}{RESET} {TEXT}{chunk}{RESET}"));
    }
    for chunk in wrap_detail(&format!("evidence: {}", proposal.evidence), CARD_WIDTH - 4) {
        lines.push(format!("{DIM}\u{2502}{RESET} {DIM}{chunk}{RESET}"));
    }
    lines.push(format!(
        "{DIM}\u{2502}{RESET} {DIM}{}{RESET}",
        proposal.path.display()
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

/// Render what a curation run filed.
pub fn render_curation(filed: &[PathBuf]) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "{BOLD}fleet skills --curate{RESET} {DIM}\u{2014} {} proposal{} filed{RESET}\n\n",
        filed.len(),
        if filed.len() == 1 { "" } else { "s" },
    ));
    for path in filed {
        out.push_str(&format!("{DIM}  {}{RESET}\n", path.display()));
    }
    out
}

// --- the collapse detector -------------------------------------------------

/// The line-Jaccard a pair of skill bodies must reach before the
/// surface reports it as a likely duplicate. The measure is Harness²'s
/// collapse detector, ported as a visibility instrument: report-only,
/// nothing is merged or deleted here.
pub const DUPLICATE_SIMILARITY: f64 = 0.6;

/// One reported near-duplicate pair: two skill names and the
/// line-Jaccard similarity of their bodies.
#[derive(Debug, Clone, PartialEq)]
pub struct DuplicatePair {
    pub a: String,
    pub b: String,
    pub similarity: f64,
}

/// Pairwise line-Jaccard over the skills' bodies, reporting every pair
/// at or above `threshold`, most similar first. Empty bodies are
/// skipped: two skills that carry no prose share nothing worth naming.
/// The comparison is deterministic — same corpus, same pairs — so the
/// surface and its test agree without a model in the loop.
pub fn near_duplicates(skills: &[Skill], threshold: f64) -> Vec<DuplicatePair> {
    let bodies: Vec<Option<std::collections::HashSet<String>>> = skills
        .iter()
        .map(|skill| {
            let lines: std::collections::HashSet<String> = skill
                .body
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .map(str::to_string)
                .collect();
            (!lines.is_empty()).then_some(lines)
        })
        .collect();
    let mut pairs: Vec<DuplicatePair> = Vec::new();
    for (i, left) in skills.iter().enumerate() {
        let Some(left_lines) = &bodies[i] else {
            continue;
        };
        for (j, right) in skills.iter().enumerate().skip(i + 1) {
            let Some(right_lines) = &bodies[j] else {
                continue;
            };
            let union = left_lines.union(right_lines).count();
            if union == 0 {
                continue;
            }
            let shared = left_lines.intersection(right_lines).count();
            let similarity = shared as f64 / union as f64;
            if similarity >= threshold {
                pairs.push(DuplicatePair {
                    a: left.name.clone(),
                    b: right.name.clone(),
                    similarity,
                });
            }
        }
    }
    pairs.sort_by(|left, right| {
        right
            .similarity
            .partial_cmp(&left.similarity)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.a.cmp(&right.a))
            .then_with(|| left.b.cmp(&right.b))
    });
    pairs
}

/// Render the collapse report: the threshold, every reported pair with
/// its similarity, and the report-only footer. Never a plain-text table.
pub fn render_duplicates(skills: usize, pairs: &[DuplicatePair]) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "{BOLD}fleet skills --duplicates{RESET} {DIM}\u{2014} {skills} skill{} scanned \u{b7} threshold {:.2}{RESET}\n\n",
        if skills == 1 { "" } else { "s" },
        DUPLICATE_SIMILARITY,
    ));
    if pairs.is_empty() {
        out.push_str(&format!(
            "{DIM}no pair reaches the threshold \u{b7} nothing is merged or deleted here{RESET}\n"
        ));
        return out;
    }
    for pair in pairs {
        out.push_str(&format!(
            "{DIM}\u{250c}\u{2500}{RESET} {BOLD}{ACCENT}{a}{RESET} {DIM}\u{b7} {b} \u{b7} {similarity:.2}{RESET} {DIM}\u{2510}{RESET}\n",
            a = pair.a,
            b = pair.b,
            similarity = pair.similarity,
        ));
    }
    out.push_str(&format!(
        "\n{DIM}{} near-duplicate pair{} \u{b7} report-only: the curation proposals stay the only writer{RESET}\n",
        pairs.len(),
        if pairs.len() == 1 { "" } else { "s" },
    ));
    out
}

/// Why one proposal could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProposalReadError {
    /// The name is not a bare proposal name: it carries a path
    /// separator or a parent component, so it addresses a file rather
    /// than naming a draft.
    NotAName(String),
    /// No proposal carries that name.
    Missing(String),
}

impl std::fmt::Display for ProposalReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProposalReadError::NotAName(name) => write!(
                f,
                "`{name}` is not a proposal name: a name carries no path separator and is not `.` or `..`"
            ),
            ProposalReadError::Missing(name) => {
                write!(f, "no proposal named `{name}` under {SKILLS_DIR}/{PROPOSALS_DIR}")
            }
        }
    }
}

impl std::error::Error for ProposalReadError {}

/// A bare proposal name: not empty, no path separator, not `.` or `..`.
/// `--show` joins the name into a path, so a name that is really a path
/// would read an arbitrary file and print it as a draft.
pub fn proposal_name(name: &str) -> Result<&str, ProposalReadError> {
    let invalid = name.is_empty()
        || name == "."
        || name == ".."
        || name.contains('/')
        || name.contains('\\')
        || name.contains('\0');
    if invalid {
        Err(ProposalReadError::NotAName(name.to_string()))
    } else {
        Ok(name)
    }
}

/// One proposal's draft, verbatim — the diff-able content `--show`
/// prints. The name must be a bare proposal name; a name carrying a path
/// separator or a parent component is refused rather than joined into a
/// path.
pub fn read_proposal(home: &Path, name: &str) -> Result<String, ProposalReadError> {
    let name = proposal_name(name)?;
    fs::read_to_string(proposals_dir(home).join(format!("{name}.md")))
        .map_err(|_| ProposalReadError::Missing(name.to_string()))
}

/// The dash run that pads a card header so its right edge aligns.
fn dashes_for(left: &str, right: &str) -> String {
    let used = left.chars().count() + right.chars().count() + 2;
    "\u{2500}".repeat(CARD_WIDTH.saturating_sub(used + 4).max(1))
}

/// Wrap `text` into `width`-character chunks. Prose breaks at the last
/// space that fits, so a description never splits mid-word; a single
/// word longer than the width — a path, a command — hard-splits rather
/// than overflowing the card, the way `lanes` wraps its details.
///
/// `pub(crate)`: the card family shares one word-aware wrapper (KI-055's
/// named remedy), and slice 8's memory surface is the next card author.
pub(crate) fn wrap_detail(text: &str, width: usize) -> Vec<String> {
    let width = width.max(16);
    let mut chunks: Vec<String> = Vec::new();
    let mut current = String::new();
    for word in text.split_inclusive(' ') {
        // A word that cannot fit on a line of its own hard-splits.
        if word.chars().count() > width {
            if !current.is_empty() {
                chunks.push(std::mem::take(&mut current));
            }
            let mut long = String::new();
            for character in word.chars() {
                long.push(character);
                if long.chars().count() >= width {
                    chunks.push(std::mem::take(&mut long));
                }
            }
            current = long;
            continue;
        }
        if current.chars().count() + word.chars().count() > width {
            chunks.push(std::mem::take(&mut current));
        }
        current.push_str(word);
    }
    if !current.is_empty() {
        chunks.push(current);
    }
    // An empty detail renders one empty line, not none: the card's body
    // keeps its line count whatever the detail held.
    if chunks.is_empty() {
        chunks.push(String::new());
    }
    chunks
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_retirement_leaves_the_library_byte_identical() {
        // C2: the owner retires from the skills page, fleet-locally —
        // and the library is never written (KI-215's shape). The file
        // is hashed before and after, and the declared list moves.
        let home = scratch("retire-library");
        let canonical = canon(&home);
        let path = write_skill(&home, "alpha", &good_skill("alpha", "use when x", "b"));
        let before = std::fs::read(&path).expect("the library file");

        crate::settings::retire_skill(&home, "alpha").expect("the retire lands");
        assert_eq!(retired(&home), vec!["alpha".to_string()]);
        assert_eq!(
            std::fs::read(&path).expect("the library file after"),
            before,
            "the library file is byte-identical"
        );
        assert!(
            load_declared(&canonical, &home).is_empty(),
            "the retired skill leaves the declared list"
        );

        // Returning it is the same click, and the skill is declared again.
        crate::settings::unretire_skill(&home, "alpha").expect("the return lands");
        assert_eq!(
            skill_names(&load_declared(&canonical, &home)),
            vec!["alpha"]
        );
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn an_unattended_run_declares_the_same_library_a_foreground_run_does() {
        // KI-294: the scheduled and delegated sites read FLEET_HOME as
        // the library and declared nothing. The unattended loader
        // prefers the canonical library and falls back to the fleet
        // home's own skills/ when the library is unresolvable, so an
        // isolated home still declares its skills.
        let home = scratch("unattended-declares");
        let library = home.join("library");
        std::fs::create_dir_all(library.join("alpha")).unwrap();
        std::fs::write(
            library.join("alpha").join(SKILL_FILE),
            good_skill("alpha", "use when x", "b"),
        )
        .unwrap();
        std::fs::create_dir_all(home.join(SKILLS_DIR).join("beta")).unwrap();
        std::fs::write(
            home.join(SKILLS_DIR).join("beta").join(SKILL_FILE),
            good_skill("beta", "use when y", "b"),
        )
        .unwrap();

        // The canonical library wins when it resolves, and the fleet
        // home's own skills/ is not read beside it.
        assert_eq!(
            skill_names(&load_unattended(
                Some(library.to_str().unwrap()),
                None,
                &home
            )),
            vec!["alpha"]
        );

        // Library unresolvable: the fleet home's own skills/ serves.
        assert_eq!(
            skill_names(&load_unattended(None, None, &home)),
            vec!["beta"]
        );
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn the_trigger_strips_its_frame_and_lowers_its_first_word() {
        // The three frames the library's descriptions lead with.
        assert_eq!(
            trigger("Use when needing to encrypt credentials at rest in Node"),
            "needing to encrypt credentials at rest in Node"
        );
        assert_eq!(
            trigger("Use this to diagnose why a generated asset looks wrong"),
            "to diagnose why a generated asset looks wrong"
        );
        assert_eq!(
            trigger("Use for triaging a stale accuracy page"),
            "triaging a stale accuracy page"
        );
        // No frame: the description is lowered as it stands.
        assert_eq!(
            trigger("Systematic triage of the accuracy page when it shows stale data"),
            "systematic triage of the accuracy page when it shows stale data"
        );
        // An empty description triggers nothing rather than panicking.
        assert_eq!(trigger("   "), "");
    }

    #[test]
    fn a_long_trigger_is_clipped_at_a_word_boundary() {
        let long = format!("Use when {}", "word ".repeat(80));
        let clipped = trigger(&long);
        // The clip plus its ellipsis, and the cut is on a boundary: no
        // half a word, no double space where the cut landed.
        assert!(clipped.chars().count() <= TRIGGER_CLIP + 1, "{clipped}");
        assert!(clipped.ends_with('\u{2026}'), "{clipped}");
        assert!(!clipped.contains("  "), "{clipped}");
        // A description inside the clip is kept whole.
        let short = "Use when a task needs credentials encrypted at rest";
        assert_eq!(trigger(short), "a task needs credentials encrypted at rest");
    }

    use super::*;

    /// A scratch home, unique per call.
    fn scratch(tag: &str) -> PathBuf {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let serial = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "fleet-skills-{tag}-{}-{serial}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// The tests' canonical library: a `canonical` directory under the
    /// scratch home, distinct from the fleet home that holds proposals.
    fn canon(home: &Path) -> PathBuf {
        home.join("canonical")
    }

    /// Write one live skill into the canonical library.
    fn write_skill(home: &Path, name: &str, text: &str) -> PathBuf {
        let dir = canon(home).join(name);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(SKILL_FILE);
        std::fs::write(&path, text).unwrap();
        path
    }

    fn good_skill(name: &str, description: &str, body: &str) -> String {
        format!("---\nname: {name}\ndescription: {description}\n---\n\n{body}\n")
    }

    fn skill_names(skills: &[Skill]) -> Vec<&str> {
        skills.iter().map(|skill| skill.name.as_str()).collect()
    }

    // --- the frontmatter subset ---

    #[test]
    fn the_parser_accepts_the_subset_and_returns_the_body() {
        let document = parse_document(
            "---\nname: demo\ndescription: 'a quoted: description'\nextra: ignored\n---\n\n# Body\n\nline two\n",
        )
        .unwrap();
        assert_eq!(document.get("name"), Some("demo"));
        // Quotes come off, and a colon inside a value is not a second key.
        assert_eq!(document.get("description"), Some("a quoted: description"));
        // An unknown key is kept, not rejected: forward compatibility.
        assert_eq!(document.get("extra"), Some("ignored"));
        // The body is everything after the fence, one leading newline
        // taken off; line endings normalize and a trailing newline does
        // not survive the split, which is fine for markdown.
        assert_eq!(document.body, "# Body\n\nline two");
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
        // A YAML block-scalar indicator opens a folded block: the
        // continuation rides the key as one value, the indicator itself
        // dropped and the block's indent stripped. This assertion used to
        // read `">\n  folded text"` — the parser handing the model a
        // description that began with the indicator, which is what the
        // shipped library did to every skill written the ordinary way.
        let document =
            parse_document("---\nname: demo\ndescription: >\n  folded text\n---\nbody\n")
                .expect("the continuation parses");
        assert_eq!(document.get("description"), Some("folded text"));
        // A list entry with no key before it.
        assert!(parse_document("---\n- item\n---\nbody\n")
            .unwrap_err()
            .contains("continues no key"));
        // A bare scalar line.
        assert!(parse_document("---\nbare\n---\nbody\n")
            .unwrap_err()
            .contains("key: value"));
        // A missing required key is the loader's rule, not the parser's:
        // the document parses, and `required` names what is absent.
        let document = parse_document("---\ndescription: only\n---\nbody\n").unwrap();
        assert!(document.required("name").unwrap_err().contains("name"));
        assert_eq!(document.required("description").unwrap(), "only");
        // An empty value.
        assert!(
            parse_document("---\nname: \"\"\ndescription: d\n---\nbody\n")
                .unwrap_err()
                .contains("empty value")
        );
        // A duplicated key.
        assert!(
            parse_document("---\nname: a\nname: b\ndescription: d\n---\nbody\n")
                .unwrap_err()
                .contains("twice")
        );
    }

    #[test]
    fn a_utf8_bom_is_stripped_and_a_bomd_failure_names_the_bom() {
        // Editors do write BOMs; a BOM'd valid file is a valid file.
        let document =
            parse_document("\u{feff}---\nname: demo\ndescription: d\n---\n\nbody\n").unwrap();
        assert_eq!(document.get("name"), Some("demo"));
        assert_eq!(document.body, "body");
        // And through the loader, not just the parser.
        let home = scratch("bom");
        write_skill(
            &home,
            "bommed",
            "\u{feff}---\nname: bommed\ndescription: d\n---\n\nbody\n",
        );
        assert_eq!(skill_names(&load(&canon(&home))), vec!["bommed"]);
        // A BOM'd file that still fails names the BOM, so the reason is
        // not misdirected at the fence alone.
        let reason = parse_document("\u{feff}name: demo\n").unwrap_err();
        assert!(reason.contains("does not open"), "{reason}");
        assert!(reason.contains("UTF-8 BOM"), "{reason}");
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn quoted_keys_unquote_and_unbalanced_ones_are_named() {
        // A writer that quotes every key is not a writer this parser
        // rejects: the key unquotes the way the value does.
        let document =
            parse_document("---\n\"name\": demo\n'description': d\n---\nbody\n").unwrap();
        assert_eq!(document.get("name"), Some("demo"));
        assert_eq!(document.get("description"), Some("d"));
        // The duplicate check runs on the unquoted key, so a quoted key
        // and a bare one still collide.
        assert!(
            parse_document("---\n\"name\": a\nname: b\ndescription: d\n---\nbody\n")
                .unwrap_err()
                .contains("twice")
        );
        // A quoted-but-empty key is still an empty key.
        assert!(parse_document("---\n\"\": a\ndescription: d\n---\nbody\n")
            .unwrap_err()
            .contains("empty key"));
        // An unbalanced quote is a typo, and it is named as the key it
        // is rather than read later as a missing required field.
        let reason = parse_document("---\n\"name: demo\ndescription: d\n---\nbody\n").unwrap_err();
        assert!(reason.contains("unbalanced quoted key"), "{reason}");
        assert!(reason.contains("\"name"), "{reason}");
    }

    // --- loading ---

    #[test]
    fn a_malformed_skill_is_skipped_by_name_and_the_others_load() {
        let home = scratch("skip");
        write_skill(
            &home,
            "good-one",
            &good_skill("good-one", "the first", "body one"),
        );
        write_skill(
            &home,
            "good-two",
            &good_skill("good-two", "the second", "body two"),
        );
        // Three ways to be malformed: no frontmatter, a missing
        // description, and a frontmatter name that is not the directory.
        write_skill(&home, "broken-fence", "# no fence here\n");
        write_skill(
            &home,
            "broken-fields",
            "---\nname: broken-fields\n---\nbody\n",
        );
        write_skill(
            &home,
            "broken-name",
            &good_skill("some-other-name", "d", "body"),
        );
        // The proposals directory is not a skill.
        std::fs::create_dir_all(proposals_dir(&home)).unwrap();

        let skills = load(&canon(&home));
        assert_eq!(skill_names(&skills), vec!["good-one", "good-two"]);
        assert_eq!(skills[0].body, "body one");
        // The loaded skills carry where they came from, for the card.
        assert!(skills[0].path.ends_with("good-one/SKILL.md"));
        assert!(skills[0].modified.is_some());
        let _ = std::fs::remove_dir_all(&home);
    }

    /// The reporting loader returns the skips with their reasons instead
    /// of printing them — a rendering surface decides where the report
    /// goes (KI-216).
    #[test]
    fn load_reported_returns_the_skips_with_their_reasons() {
        let home = scratch("reported");
        write_skill(&home, "good-one", &good_skill("good-one", "d", "body"));
        write_skill(&home, "broken-name", &good_skill("other", "d", "body"));
        write_skill(&home, "broken-fence", "# no fence here\n");
        std::fs::create_dir_all(canon(&home).join("empty-dir")).unwrap();

        let (skills, skips) = load_reported(&canon(&home));
        assert_eq!(skill_names(&skills), vec!["good-one"]);
        let named: Vec<(&str, &str)> = skips
            .iter()
            .map(|skip| (skip.name.as_str(), skip.reason.as_str()))
            .collect();
        assert_eq!(
            named,
            vec![
                (
                    "broken-fence",
                    "the file does not open with a `---` frontmatter fence"
                ),
                (
                    "broken-name",
                    "frontmatter name `other` does not match the directory `broken-name`"
                ),
                ("empty-dir", "no readable SKILL.md"),
            ]
        );
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn a_live_skill_carrying_proposed_true_is_refused() {
        // A proposal that reached the live tree is not promoted silently.
        let home = scratch("proposed-live");
        write_skill(
            &home,
            "misfiled",
            "---\nname: misfiled\ndescription: d\nproposed: true\n---\nbody\n",
        );
        assert!(load(&canon(&home)).is_empty());
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn a_missing_skills_directory_is_no_skills_not_an_error() {
        let home = scratch("absent");
        assert!(load(&canon(&home)).is_empty());
        assert!(load_proposals(&home).is_empty());
        let _ = std::fs::remove_dir_all(&home);
    }

    /// The library's nested-metadata dialect — the `project-warmup`
    /// shape: a `key:` whose indented block follows rides the key as one
    /// opaque value, and the fields the loader requires still parse.
    #[test]
    fn a_nested_metadata_block_loads_as_one_opaque_value() {
        let text = "---\nname: demo\ndescription: a library skill\nversion: 3.5.0\nmetadata:\n  hermes:\n    tags: [methodology]\n    related_skills: [other]\n---\n\nbody\n";
        let document = parse_document(text).expect("the dialect parses");
        assert_eq!(document.required("name").unwrap(), "demo");
        assert_eq!(document.required("description").unwrap(), "a library skill");
        assert_eq!(document.get("version"), Some("3.5.0"));
        let metadata = document.get("metadata").expect("the block rides the key");
        assert!(metadata.contains("hermes:"), "{metadata}");
        assert!(metadata.contains("tags: [methodology]"), "{metadata}");
    }

    /// The library's list dialect — the `rss-feeds` shape: column-0
    /// `- item` entries under an empty-valued key.
    #[test]
    fn a_list_block_loads_as_one_opaque_value() {
        let text = "---\nname: demo\ndescription: reads feeds\nplatforms:\n- linux\n- macos\n---\n\nbody\n";
        let document = parse_document(text).expect("the dialect parses");
        assert_eq!(document.required("name").unwrap(), "demo");
        assert_eq!(document.get("platforms"), Some("- linux\n- macos"));
    }

    /// A continuation with no key before it is still malformed, by name.
    #[test]
    fn a_leading_continuation_line_is_rejected_by_name() {
        let error = parse_document("---\n- linux\nname: demo\n---\nbody\n")
            .expect_err("no key to continue");
        assert!(
            error.contains("continues no key"),
            "the reason names the shape: {error}"
        );
    }

    /// An empty value whose block never comes is still malformed, by
    /// name — the old empty-value error under its precise new one.
    #[test]
    fn an_empty_value_with_no_block_is_rejected_by_name() {
        let error = parse_document("---\nname: demo\ndescription: d\nmetadata:\n---\nbody\n")
            .expect_err("no block follows");
        assert!(
            error.contains("`metadata` has an empty value with no continuation block"),
            "the reason names the key: {error}"
        );
    }

    /// The resolution seam: the override wins, an empty one does not,
    /// the user home is the default, and neither known is `None`.
    #[test]
    fn the_canonical_directory_resolves_override_then_user_home() {
        assert_eq!(
            resolve_skills_dir(Some("/tmp/lib"), Some("/usr/home")),
            Some(PathBuf::from("/tmp/lib"))
        );
        assert_eq!(
            resolve_skills_dir(Some(""), Some("/usr/home")),
            Some(PathBuf::from("/usr/home/.agents/skills"))
        );
        assert_eq!(
            resolve_skills_dir(None, Some("/usr/home")),
            Some(PathBuf::from("/usr/home/.agents/skills"))
        );
        assert_eq!(resolve_skills_dir(None, None), None);
    }

    /// The proposals directory stays under the fleet home even when the
    /// canonical library resolves elsewhere — drafts are fleet's own
    /// working state.
    #[test]
    fn proposals_stay_under_the_fleet_home() {
        let home = scratch("proposals-home");
        assert_eq!(proposals_dir(&home), home.join("skills").join("proposals"));
    }

    #[test]
    fn proposals_load_with_their_kind_and_evidence() {
        let home = scratch("proposals");
        let dir = proposals_dir(&home);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("bash-recovery.md"),
            "---\nname: bash-recovery\ndescription: d\nproposed: true\nkind: create\nevidence: session x\n---\n\n# body\n",
        )
        .unwrap();
        // A malformed draft is skipped by name, the other still lists.
        std::fs::write(dir.join("broken.md"), "---\nname: broken\n---\nbody\n").unwrap();
        let proposals = load_proposals(&home);
        assert_eq!(proposals.len(), 1);
        assert_eq!(proposals[0].kind, ProposalKind::Create);
        assert_eq!(proposals[0].evidence, "session x");
        assert_eq!(proposals[0].body, "# body");
        let _ = std::fs::remove_dir_all(&home);
    }

    // --- the use_skill tool ---

    #[test]
    fn use_skill_returns_the_body_or_a_typed_error() {
        let skills = vec![Skill {
            name: "demo".to_string(),
            description: "d".to_string(),
            body: "the body".to_string(),
            path: PathBuf::from("/x/SKILL.md"),
            modified: None,
        }];
        let found = use_skill_result(&skills, &serde_json::json!({ "name": "demo" }));
        assert_eq!(found, "the body");
        let missing = use_skill_result(&skills, &serde_json::json!({ "name": "nope" }));
        assert_eq!(missing, "error: no skill named `nope`");
        let unnamed = use_skill_result(&skills, &serde_json::json!({}));
        assert!(
            unnamed.starts_with("error: the `name` argument"),
            "{unnamed}"
        );
        // The name a call asks for is what the approval prompt shows.
        assert_eq!(
            requested_skill(r#"{"name":"demo"}"#).as_deref(),
            Some("demo")
        );
        assert_eq!(requested_skill("{not json"), None);
        assert_eq!(requested_skill(r#"{"name":""}"#), None);
    }

    // --- similarity ---

    #[test]
    fn near_identical_descriptions_score_high_and_different_ones_do_not() {
        // Six shared tokens over a seven-token union: the added `now` is
        // the only difference.
        let similarity = description_similarity(
            "probe the embeddings worker for reachability",
            "probe the embeddings worker for reachability now",
        );
        assert!((similarity - 6.0 / 7.0).abs() < 1e-9, "{similarity}");
        assert!(similarity >= MERGE_SIMILARITY_THRESHOLD, "{similarity}");
        assert!(description_similarity("write release notes", "probe the embeddings worker") < 0.2);
        // Two empty token sets are not similar; they are absent.
        assert_eq!(description_similarity("", "anything"), 0.0);
        assert_eq!(description_similarity("---", "!!!"), 0.0);
        // Identical text is a perfect score.
        assert_eq!(
            description_similarity("one two three", "one two three"),
            1.0
        );
    }

    // --- the drafter ---

    fn tool_call(id: &str, name: &str, arguments: &str) -> crate::api::ToolCall {
        crate::api::ToolCall {
            id: id.to_string(),
            kind: "function".to_string(),
            function: crate::api::FunctionCall {
                name: name.to_string(),
                arguments: arguments.to_string(),
            },
        }
    }

    fn assistant(calls: Vec<crate::api::ToolCall>) -> Record {
        Record::Assistant {
            content: None,
            tool_calls: calls,
            persona: None,
        }
    }

    fn result(call_id: &str, name: &str, output: &str) -> Record {
        Record::ToolResult {
            call_id: call_id.to_string(),
            name: name.to_string(),
            output: output.to_string(),
            truncated: false,
            approved: true,
        }
    }

    /// A call the user denied: the denial text the model received, with
    /// the `approved` flag the log carries.
    fn denied(call_id: &str, name: &str) -> Record {
        Record::ToolResult {
            call_id: call_id.to_string(),
            name: name.to_string(),
            output: "denied: the user did not approve this call.".to_string(),
            truncated: false,
            approved: false,
        }
    }

    #[test]
    fn a_failure_that_was_corrected_drafts_a_create_with_the_records_cited() {
        let records = vec![
            Record::Start {
                personas: None,
                v: 1,
                model: "m".into(),
                created: "c".into(),
                kind: None,
                parent: None,
                workspace: None,
            },
            Record::User {
                content: "run it".into(),
            },
            assistant(vec![tool_call("c1", "bash", "{}")]),
            result("c1", "bash", "error: exit 127"),
            assistant(vec![tool_call("c2", "bash", "{}")]),
            result("c2", "bash", "exit 0"),
            assistant(vec![]),
        ];
        let found = lessons(&records, &[], "session-1");
        assert_eq!(found.len(), 1, "{found:?}");
        let lesson = &found[0];
        assert_eq!(lesson.kind, ProposalKind::Create);
        assert_eq!(lesson.name, "bash-recovery");
        // The evidence names the session and both records.
        assert!(lesson.evidence.contains("session-1"), "{}", lesson.evidence);
        assert!(lesson.evidence.contains("record 4"), "{}", lesson.evidence);
        assert!(lesson.evidence.contains("record 6"), "{}", lesson.evidence);
        let filed = render_proposal(lesson);
        assert!(filed.contains("proposed: true"), "{filed}");
        assert!(filed.contains("kind: create"), "{filed}");
        assert!(filed.starts_with("---\n"), "{filed}");
    }

    #[test]
    fn an_uncorrected_failure_does_not_end_the_search() {
        // The first failure is never corrected; the second one is. The
        // lesson is the second pair, not nothing.
        let records = vec![
            Record::User {
                content: "t".into(),
            },
            assistant(vec![tool_call("c1", "bash", "{}")]),
            result("c1", "bash", "error: exit 127"),
            assistant(vec![tool_call("c2", "read_file", "{}")]),
            result("c2", "read_file", "error: not valid UTF-8"),
            assistant(vec![tool_call("c3", "read_file", "{}")]),
            result("c3", "read_file", "the text"),
        ];
        let found = lessons(&records, &[], "session-4");
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].name, "read-file-recovery");
        assert!(
            found[0].evidence.contains("record 5"),
            "{}",
            found[0].evidence
        );
        assert!(
            found[0].evidence.contains("record 7"),
            "{}",
            found[0].evidence
        );
    }

    #[test]
    fn a_denied_retry_is_not_a_correction() {
        // A denied retry's output is the denial, not a result: it
        // corrects nothing, and the failure it follows stays
        // uncorrected.
        let records = vec![
            Record::User {
                content: "t".into(),
            },
            assistant(vec![tool_call("c1", "bash", "{}")]),
            result("c1", "bash", "error: exit 127"),
            assistant(vec![tool_call("c2", "bash", "{}")]),
            denied("c2", "bash"),
        ];
        assert!(lessons(&records, &[], "session-5").is_empty());
        // The denial neither corrects nor ends the search: an approved
        // success after it is the correction.
        let records = vec![
            Record::User {
                content: "t".into(),
            },
            assistant(vec![tool_call("c1", "bash", "{}")]),
            result("c1", "bash", "error: exit 127"),
            assistant(vec![tool_call("c2", "bash", "{}")]),
            denied("c2", "bash"),
            assistant(vec![tool_call("c3", "bash", "{}")]),
            result("c3", "bash", "exit 0"),
        ];
        let found = lessons(&records, &[], "session-5");
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].name, "bash-recovery");
        assert!(
            found[0].evidence.contains("record 7"),
            "{}",
            found[0].evidence
        );
    }

    #[test]
    fn two_denied_sequences_are_not_a_repeated_procedure() {
        // Two identical turns the user denied twice are not a procedure
        // the session kept re-deriving: nothing ran.
        let records = vec![
            Record::User {
                content: "t".into(),
            },
            assistant(vec![
                tool_call("c1", "list_dir", "{}"),
                tool_call("c2", "read_file", "{}"),
            ]),
            denied("c1", "list_dir"),
            denied("c2", "read_file"),
            assistant(vec![
                tool_call("c3", "list_dir", "{}"),
                tool_call("c4", "read_file", "{}"),
            ]),
            denied("c3", "list_dir"),
            denied("c4", "read_file"),
        ];
        assert!(lessons(&records, &[], "session-6").is_empty());
        // A turn that only half ran does not count either: the sequence
        // that executed is not the sequence that was requested.
        let records = vec![
            Record::User {
                content: "t".into(),
            },
            assistant(vec![
                tool_call("c1", "list_dir", "{}"),
                tool_call("c2", "read_file", "{}"),
            ]),
            result("c1", "list_dir", "ok"),
            denied("c2", "read_file"),
            assistant(vec![
                tool_call("c3", "list_dir", "{}"),
                tool_call("c4", "read_file", "{}"),
            ]),
            result("c3", "list_dir", "ok"),
            result("c4", "read_file", "ok"),
        ];
        assert!(lessons(&records, &[], "session-6").is_empty());
    }

    #[test]
    fn the_evidence_names_the_tool_and_a_compacted_argument_summary() {
        let records = vec![
            Record::User {
                content: "t".into(),
            },
            assistant(vec![tool_call("c1", "bash", r#"{"command":"ls"}"#)]),
            result("c1", "bash", "error: exit 127"),
            assistant(vec![tool_call("c2", "bash", r#"{"command":"ls -la"}"#)]),
            result("c2", "bash", "exit 0"),
        ];
        let found = lessons(&records, &[], "session-7");
        assert_eq!(found.len(), 1, "{found:?}");
        let evidence = &found[0].evidence;
        // Self-contained: the tool, both argument summaries, and both
        // records, so the pair is judgeable without replaying the log.
        assert!(evidence.contains("`bash`"), "{evidence}");
        assert!(evidence.contains("command: ls -la"), "{evidence}");
        assert!(evidence.contains("command: ls)"), "{evidence}");
        assert!(evidence.contains("record 3"), "{evidence}");
        assert!(evidence.contains("record 5"), "{evidence}");
        assert!(evidence.contains("error: exit 127"), "{evidence}");
        // The evidence rides the proposal's frontmatter as one line.
        let filed = render_proposal(&found[0]);
        assert!(filed.contains("evidence: session session-7:"), "{filed}");
    }

    #[test]
    fn a_long_argument_summary_stays_one_bounded_line() {
        let long = "x".repeat(200);
        let records = vec![
            Record::User {
                content: "t".into(),
            },
            assistant(vec![tool_call(
                "c1",
                "bash",
                &format!(r#"{{"command":"{long}"}}"#),
            )]),
            result("c1", "bash", "error: exit 127"),
            assistant(vec![tool_call("c2", "bash", r#"{"command":"ls"}"#)]),
            result("c2", "bash", "exit 0"),
        ];
        let found = lessons(&records, &[], "session-8");
        let evidence = &found[0].evidence;
        assert!(!evidence.contains('\n'), "{evidence}");
        assert!(evidence.contains('\u{2026}'), "{evidence}");
        assert!(evidence.contains("command: x"), "{evidence}");
    }

    #[test]
    fn compact_arguments_summarizes_an_object_and_falls_back_to_raw() {
        assert_eq!(
            compact_arguments(r#"{"command":"ls -la"}"#),
            "command: ls -la"
        );
        assert_eq!(compact_arguments(r#"{"a":"1","b":2}"#), "a: 1, b: 2");
        // An empty object, a bare string, and invalid JSON are used raw.
        assert_eq!(compact_arguments("{}"), "");
        assert_eq!(compact_arguments("not json"), "not json");
        // A multi-line output keeps only its first line.
        assert_eq!(compact("a\nb", 80), "a");
    }

    #[test]
    fn a_repeated_multi_step_sequence_drafts_a_procedure() {
        let records = vec![
            Record::User {
                content: "t".into(),
            },
            assistant(vec![
                tool_call("c1", "list_dir", "{}"),
                tool_call("c2", "read_file", "{}"),
            ]),
            result("c1", "list_dir", "ok"),
            result("c2", "read_file", "ok"),
            assistant(vec![
                tool_call("c3", "list_dir", "{}"),
                tool_call("c4", "read_file", "{}"),
            ]),
            result("c3", "list_dir", "ok"),
            result("c4", "read_file", "ok"),
        ];
        let found = lessons(&records, &[], "session-2");
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].name, "procedure-list-dir-read-file");
        assert_eq!(found[0].kind, ProposalKind::Create);
        assert!(
            found[0].evidence.contains("ran 2 times"),
            "{}",
            found[0].evidence
        );
    }

    #[test]
    fn a_failed_turn_row_joined_by_the_task_hash_drafts_a_lesson() {
        let records = vec![Record::User {
            content: "refactor the crate".into(),
        }];
        let hash = crate::calib::hash16("refactor the crate");
        let rows = vec![crate::calib::ReadRow {
            ts: "2026-10-13T12:00:00Z".to_string(),
            op: crate::calib::OP_TURN.to_string(),
            input_hash: hash.clone(),
            outcome: "failed".to_string(),
            stop_reason: "failed".to_string(),
            reason: "server".to_string(),
            ..Default::default()
        }];
        let found = lessons(&records, &rows, "session-3");
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].name, "turn-failure-server");
        assert!(found[0].evidence.contains(&hash), "{}", found[0].evidence);
        // A row for some other session's task is not this session's
        // evidence: the join is on the hash.
        let other = vec![crate::calib::ReadRow {
            input_hash: "ffffffffffffffff".to_string(),
            ..rows[0].clone()
        }];
        assert!(lessons(&records, &other, "session-3").is_empty());
    }

    #[test]
    fn an_update_lesson_keeps_the_live_body_and_appends_the_lesson() {
        let home = scratch("update");
        write_skill(
            &home,
            "bash-recovery",
            &good_skill("bash-recovery", "live description", "# live body"),
        );
        let live = load(&canon(&home));
        let records = vec![
            Record::User {
                content: "t".into(),
            },
            assistant(vec![tool_call("c1", "bash", "{}")]),
            result("c1", "bash", "error: exit 127"),
            assistant(vec![tool_call("c2", "bash", "{}")]),
            result("c2", "bash", "exit 0"),
        ];
        let filed = draft_session(&canon(&home), &home, &session_log(&home, &records), None);
        assert_eq!(filed.len(), 1, "{filed:?}");
        let text = std::fs::read_to_string(&filed[0]).unwrap();
        assert!(text.contains("kind: update"), "{text}");
        // The description stays the live skill's, and the live body rides
        // along so the draft diffs against the file it would replace.
        assert!(text.contains("description: live description"), "{text}");
        assert!(text.contains("# live body"), "{text}");
        assert!(text.contains("## Proposed addition"), "{text}");
        // The live tree is untouched: one skill, unchanged on disk.
        let after = load(&canon(&home));
        assert_eq!(after, live);
        let _ = std::fs::remove_dir_all(&home);
    }

    /// Write `records` as a session log under `home` and return its path.
    fn session_log(home: &Path, records: &[Record]) -> PathBuf {
        let mut session = crate::session::Session::create(home, "m").unwrap();
        for record in records {
            session.record(record).unwrap();
        }
        session.path().to_path_buf()
    }

    #[test]
    fn drafting_files_into_proposals_and_never_into_the_live_tree() {
        let home = scratch("draft");
        // One live skill that is NOT the drafted name, to prove the
        // drafter does not touch it.
        write_skill(&home, "unrelated", &good_skill("unrelated", "d", "body"));
        let before = std::fs::read_to_string(canon(&home).join("unrelated/SKILL.md")).unwrap();
        let records = vec![
            Record::User {
                content: "t".into(),
            },
            assistant(vec![tool_call("c1", "read_file", "{}")]),
            result("c1", "read_file", "error: not valid UTF-8"),
            assistant(vec![tool_call("c2", "read_file", "{}")]),
            result("c2", "read_file", "the text"),
        ];
        let filed = draft_session(&canon(&home), &home, &session_log(&home, &records), None);
        assert_eq!(filed.len(), 1, "{filed:?}");
        assert!(filed[0].starts_with(proposals_dir(&home)), "{filed:?}");
        assert_eq!(filed[0].file_name().unwrap(), "read-file-recovery.md");
        // The live tree holds exactly the one skill it held before.
        let mut entries: Vec<String> = std::fs::read_dir(canon(&home))
            .unwrap()
            .flatten()
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect();
        entries.sort();
        assert_eq!(entries, vec!["unrelated".to_string()]);
        assert_eq!(
            before,
            std::fs::read_to_string(canon(&home).join("unrelated/SKILL.md")).unwrap()
        );
        // KI-053: a second session's evidence for the same lesson
        // accumulates onto the waiting draft rather than being skipped,
        // and the appended line names the session it came from.
        let second_log = session_log(&home, &records);
        let second_id = second_log
            .file_stem()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        let second = draft_session(&canon(&home), &home, &second_log, None);
        assert_eq!(second.len(), 1, "{second:?}");
        let draft = std::fs::read_to_string(filed[0].clone()).unwrap();
        assert!(draft.contains("## Further evidence"), "{draft}");
        assert!(
            draft.contains(&format!("(from {second_id})")),
            "the appended line names its session: {draft}"
        );
        // The first draft's frontmatter stands; the live tree is untouched.
        assert!(
            draft.starts_with("---\nname: read-file-recovery\n"),
            "{draft}"
        );
        assert_eq!(entries, vec!["unrelated".to_string()]);
        let _ = std::fs::remove_dir_all(&home);
    }

    // --- curation ---

    #[test]
    fn curation_merges_the_twins_and_retires_nothing_from_the_canonical_library() {
        let home = scratch("curate");
        write_skill(
            &home,
            "alpha",
            &good_skill("alpha", "probe the embeddings worker for reachability", "a"),
        );
        write_skill(
            &home,
            "beta",
            &good_skill(
                "beta",
                "probe the embeddings worker for reachability now",
                "b",
            ),
        );
        write_skill(
            &home,
            "gamma",
            &good_skill("gamma", "write the weekly release notes", "g"),
        );
        // Two sessions of history — the whole window — and one `use_skill`
        // call for alpha.
        let used = session_log(
            &home,
            &[
                Record::User {
                    content: "t".into(),
                },
                assistant(vec![tool_call(
                    "c1",
                    crate::tools::USE_SKILL_TOOL,
                    r#"{"name":"alpha"}"#,
                )]),
                result("c1", "use_skill", "a"),
            ],
        );
        let _unused = session_log(
            &home,
            &[Record::User {
                content: "t2".into(),
            }],
        );
        let _ = used;

        let filed = curate(&canon(&home), &home, 2, MERGE_SIMILARITY_THRESHOLD);
        let names: Vec<String> = filed
            .iter()
            .map(|path| path.file_stem().unwrap().to_string_lossy().into_owned())
            .collect();
        // KI-054: the merge draft is named by the PAIR, so a retire
        // draft for the survivor and a merge draft for it no longer
        // target one file.
        assert!(names.contains(&"merge-beta-alpha".to_string()), "{names:?}");
        let proposals = load_proposals(&home);
        let merge = proposals
            .iter()
            .find(|p| p.name == "merge-beta-alpha")
            .expect("the merge draft");
        assert_eq!(merge.kind, ProposalKind::Merge);
        assert!(merge.evidence.contains("0.86"), "{}", merge.evidence);
        assert!(merge.evidence.contains("`beta`"), "{}", merge.evidence);
        // The retirement rule is a no-op against the canonical library:
        // the usage scan reads fleet's sessions only, so "unused by
        // fleet" is not unused across harnesses (KI-215). gamma was never
        // selected and nothing is filed against it.
        assert!(!names.contains(&"gamma".to_string()), "{names:?}");
        assert!(!names.contains(&"beta".to_string()), "{names:?}");
        // The live tree is untouched.
        assert_eq!(
            skill_names(&load(&canon(&home))),
            vec!["alpha", "beta", "gamma"]
        );
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn curation_refuses_to_judge_on_a_window_shorter_than_the_rule() {
        let home = scratch("curate-short");
        write_skill(
            &home,
            "lonely",
            &good_skill("lonely", "never used at all", "b"),
        );
        let _one = session_log(
            &home,
            &[Record::User {
                content: "t".into(),
            }],
        );
        // One session of history against a window of twenty: nothing is
        // proposed, because "unused across the last 20" is not knowable.
        assert!(curate(
            &canon(&home),
            &home,
            DEFAULT_RETIRE_AFTER_SESSIONS,
            MERGE_SIMILARITY_THRESHOLD
        )
        .is_empty());
        assert!(load_proposals(&home).is_empty());
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn the_usage_scan_counts_selections_per_skill_over_the_window() {
        let home = scratch("usage");
        write_skill(&home, "alpha", &good_skill("alpha", "d", "a"));
        for _ in 0..3 {
            let _ = session_log(
                &home,
                &[
                    Record::User {
                        content: "t".into(),
                    },
                    assistant(vec![tool_call(
                        "c1",
                        crate::tools::USE_SKILL_TOOL,
                        r#"{"name":"alpha"}"#,
                    )]),
                    result("c1", "use_skill", "a"),
                ],
            );
        }
        let scan = scan_usage(&home, 2);
        assert_eq!(scan.sessions, 2);
        assert_eq!(scan.count("alpha"), 2);
        // The window is the most recent sessions, not the oldest.
        let wide = scan_usage(&home, 20);
        assert_eq!(wide.count("alpha"), 3);
        // A denied selection is not a use: the body never entered
        // context, so the call does not count.
        let denied = session_log(
            &home,
            &[
                Record::User {
                    content: "t".into(),
                },
                assistant(vec![tool_call(
                    "c9",
                    crate::tools::USE_SKILL_TOOL,
                    r#"{"name":"alpha"}"#,
                )]),
                Record::ToolResult {
                    call_id: "c9".to_string(),
                    name: "use_skill".to_string(),
                    output: "denied: the user did not approve this call.".to_string(),
                    truncated: false,
                    approved: false,
                },
            ],
        );
        let _ = denied;
        let after = scan_usage(&home, 20);
        assert_eq!(after.count("alpha"), 3, "a denied selection must not count");
        let _ = std::fs::remove_dir_all(&home);
    }

    // --- the surface ---

    #[test]
    fn a_folded_description_reads_as_one_line_not_the_indicator() {
        // A writer using YAML's `>` is writing ordinary YAML. A parser
        // that reads the indicator as the value hands the model a
        // description that begins `>` and carries newlines — which is
        // what the shipped library did until this was fixed.
        let folded = "---\nname: a-skill\ndescription: >\n  The first line of it\n  and the second.\n---\n\nThe body.\n";
        let document = parse_document(folded).expect("the folded block parses");
        assert_eq!(
            document.get("description"),
            Some("The first line of it and the second.")
        );
        // A literal block keeps its line breaks rather than folding them,
        // which is the reason the two indicators differ. The parser keeps
        // the lines as written rather than stripping the block's common
        // indent — a deviation from full YAML that predates this change
        // and that the nested-block dialect depends on, so it is named
        // here rather than quietly tightened.
        let literal = "---\nname: a-skill\ndescription: |\n  The first line\n  and the second.\n---\n\nThe body.\n";
        let document = parse_document(literal).expect("the literal block parses");
        assert_eq!(
            document.get("description"),
            Some("  The first line\n  and the second.")
        );
        // A plain scalar is unchanged, and a chomping indicator (`>-`)
        // is the same fold.
        let plain = parse_document("---\nname: a\ndescription: one line\n---\n\nb").unwrap();
        assert_eq!(plain.get("description"), Some("one line"));
        let chomped =
            parse_document("---\nname: a\ndescription: >-\n  folded\n  too\n---\n\nb").unwrap();
        assert_eq!(chomped.get("description"), Some("folded too"));
    }

    #[test]
    fn the_surface_is_cards_and_never_a_table() {
        let home = scratch("render");
        write_skill(
            &home,
            "alpha",
            &good_skill("alpha", "probe the embeddings worker", "a"),
        );
        let skills = load(&canon(&home));
        let usage = scan_usage(&home, 20);
        let rendered = render(&skills, &usage);
        assert!(rendered.contains("fleet skills"), "{rendered}");
        assert!(rendered.contains("alpha"), "{rendered}");
        assert!(rendered.contains("0 uses"), "{rendered}");
        assert!(rendered.contains("\u{250c}\u{2500}"), "{rendered}");
        // An empty tree says so rather than rendering nothing.
        let empty = scratch("render-empty");
        assert!(render(&load(&empty), &usage).contains("no skills yet"));
        let _ = std::fs::remove_dir_all(&home);
        let _ = std::fs::remove_dir_all(&empty);
    }

    #[test]
    fn the_proposed_surface_lists_the_kind_and_the_evidence() {
        let home = scratch("render-proposed");
        std::fs::create_dir_all(proposals_dir(&home)).unwrap();
        std::fs::write(
            proposals_dir(&home).join("gamma.md"),
            "---\nname: gamma\ndescription: retire gamma\nproposed: true\nkind: retire\nevidence: unused across the last 20 sessions\n---\n\n# retire: gamma\n",
        )
        .unwrap();
        let rendered = render_proposals(&load_proposals(&home));
        assert!(
            rendered.contains("awaiting the slice-12 gate"),
            "{rendered}"
        );
        assert!(rendered.contains("retire"), "{rendered}");
        assert!(
            rendered.contains("unused across the last 20 sessions"),
            "{rendered}"
        );
        // The draft itself is readable back, for `--show`.
        assert!(read_proposal(&home, "gamma")
            .unwrap()
            .contains("# retire: gamma"));
        assert_eq!(
            read_proposal(&home, "nope"),
            Err(ProposalReadError::Missing("nope".to_string()))
        );
        // A name that is really a path is refused, not joined: the file
        // outside the proposals directory stays unread.
        std::fs::write(home.join("outside.md"), "not a draft").unwrap();
        assert_eq!(
            read_proposal(&home, "../outside"),
            Err(ProposalReadError::NotAName("../outside".to_string()))
        );
        assert_eq!(
            read_proposal(&home, "nested/gamma"),
            Err(ProposalReadError::NotAName("nested/gamma".to_string()))
        );
        assert_eq!(
            read_proposal(&home, ".."),
            Err(ProposalReadError::NotAName("..".to_string()))
        );
        assert!(proposal_name("bash-recovery").is_ok());
        let _ = std::fs::remove_dir_all(&home);
    }

    /// One skill for the detector's tests: a name and a body, no disk.
    fn skill(name: &str, body: &str) -> Skill {
        Skill {
            name: name.to_string(),
            description: "a test skill".to_string(),
            body: body.to_string(),
            path: PathBuf::from(format!("/tmp/{name}/SKILL.md")),
            modified: None,
        }
    }

    #[test]
    fn the_collapse_detector_reports_a_near_duplicate_pair_and_stays_silent_otherwise() {
        let shared = "one line\ntwo line\nthree line\nfour line\nfive line\n\
                      six line\nseven line\neight line\nnine line\nten line";
        let near = format!("{shared}\nan eleventh differing line");
        let distinct = "an entirely different body\nwith none of the shared lines";
        let skills = vec![
            skill("alpha", shared),
            skill("beta", &near),
            skill("gamma", distinct),
        ];
        let pairs = near_duplicates(&skills, DUPLICATE_SIMILARITY);
        assert_eq!(pairs.len(), 1, "{pairs:?}");
        assert_eq!(pairs[0].a, "alpha");
        assert_eq!(pairs[0].b, "beta");
        assert!(pairs[0].similarity >= DUPLICATE_SIMILARITY);
        // Bodyless skills share nothing worth naming, and a library of
        // distinct bodies reports nothing.
        let distinct_only = vec![
            skill("one", distinct),
            skill("two", "another body\nwith other lines"),
            skill("empty", ""),
        ];
        assert!(near_duplicates(&distinct_only, DUPLICATE_SIMILARITY).is_empty());
        // Report-only: the render names the pair and the threshold, and
        // says it changes nothing.
        let rendered = render_duplicates(skills.len(), &pairs);
        assert!(
            rendered.contains("alpha") && rendered.contains("beta"),
            "{rendered}"
        );
        assert!(rendered.contains("report-only"), "{rendered}");
        let quiet = render_duplicates(distinct_only.len(), &[]);
        assert!(quiet.contains("no pair reaches the threshold"), "{quiet}");
    }
}
