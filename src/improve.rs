//! Self-maintenance under gates: the harness proposes and applies
//! changes to its own skills, memory, and config — and, as stage 2, to
//! its own source — with every change a proposal that survives only
//! the local gate.
//!
//! The design is the owner's plan-gate protocol, in the fork's
//! vocabulary: the fork's `packages/extensions` surface inspects the
//! runtime read-only and installs persistent plugins through Plugin
//! Manager in creator mode — **no model tool creates a dynamic
//! definition**. Fleet's port of that rule is structural here: the
//! `improve` tool *files* a proposal, and only the owner's
//! `fleet improve apply` runs one, through the same gate, with the
//! verdict in the calibration ledger. Nothing is applied silently.
//!
//! ## The proposal
//!
//! One markdown file per proposal under `<FLEET_HOME>/proposals/`, with
//! a frontmatter the skills parser already speaks (`name`, `target`,
//! `target-name`, `basis`, `created`, `status`) and a body carrying the
//! four things a reviewer needs: the **justification** (the session's
//! evidence, the failing test, or the graded outcome behind it), the
//! **acceptance criteria**, the **expected effect**, and the **change**
//! itself as a fenced JSON object. Slice 7's drafter files its own
//! skill drafts under `<FLEET_HOME>/skills/proposals/`; those files are
//! the skill-proposal feedstock and `fleet improve` lists them beside
//! the general store — the two locations are named honestly, and
//! neither is migrated into the other.
//!
//! ## The gate
//!
//! [`Gate::local`] runs, in the repository root, in order:
//! `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings`,
//! `cargo build --release`, `cargo test`. Exit codes are read directly
//! from each step's own status — never through a pipe — and a step
//! whose tool is absent is a **named** gate failure, never a silent
//! pass: on a machine without rustfmt (KI-007) the fmt step refuses
//! every proposal, which is the honest behavior.
//!
//! ## Apply, revert, and the provenance rule
//!
//! The honest reading of "applies only with the gate green" for a
//! source change is: **apply to the working tree, run the full gate,
//! and revert on any failure**. The net effect is that a source change
//! survives only if the gate is green — and the revert is structural,
//! because the pre-image is committed state (`git checkout HEAD --`).
//! The FLEET_HOME skills tree and `config.json` are not git-tracked, so
//! a home-side apply keeps the target's pre-image in a sidecar and
//! `fleet improve undo` restores it byte-for-byte. Memory changes are
//! not re-guarded here: they apply through slice 8's
//! [`memory::Memory::supersede`], whose provenance rule is the
//! permission model — an inferred fact supersedes on the recorded
//! justification, a confirmed-by one waits for approval.
//!
//! ## Destructive changes wait
//!
//! A proposal that retires a skill, forgets a fact, or removes a config
//! entry is destructive: it waits for explicit approval in the current
//! turn (`fleet improve apply <name> --approve`), its status is
//! `waiting`, nothing is applied, and the message names exactly what
//! will be destroyed. A non-destructive update — a skill's body
//! corrected, an inferred fact superseded, a config value changed, a
//! source patch — applies on a green gate alone.
//!
//! ## The ledger
//!
//! Every proposal that is applied, refused, or left waiting lands
//! exactly one `propose` row in the calibration stream, carrying the
//! proposal's name, target, basis, verdict, and the gate's per-step
//! evidence — the loop's self-modification is auditable rather than
//! asserted.

use std::fs;
use std::path::{Path, PathBuf};

use crate::calib::{self, Stream, VERDICT_APPLIED, VERDICT_REFUSED, VERDICT_WAITING};
use crate::eval;
use crate::memory::{self, Memory};
use crate::skills;

/// The directory under `<FLEET_HOME>` holding the general proposal
/// store, one markdown file per proposal.
pub const PROPOSALS_DIR: &str = "proposals";

/// The directory under `proposals/` holding the pre-images a home-side
/// apply keeps for its undo: a replaced `SKILL.md`, a moved-aside skill
/// directory, or a replaced `config.json`. Git-tracked targets keep
/// their pre-image in git instead and store nothing here.
pub const PREIMAGES_DIR: &str = "preimages";

/// The config file a `config` proposal merges into, under the home.
const CONFIG_FILE: &str = "config.json";

/// The memories directory a memory proposal addresses, under the repo
/// root the gate runs in. Slice 12 targets the repository's own
/// corpus — the same tree `git checkout` reverts — rather than
/// re-resolving a root from the working directory.
const MEMORIES_DIR: &str = "memories";

/// How much of a gate step's output a row or a stamp carries. A cargo
/// failure's last lines name the failure; the whole log is the
/// terminal's, not the ledger's.
pub const MAX_DETAIL_CHARS: usize = 240;

// --- the vocabulary --------------------------------------------------------

/// What a proposal addresses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    /// A skill under `<FLEET_HOME>/skills/`.
    Skill,
    /// A fact under the repo's `memories/`.
    Memory,
    /// A file in the repository's source tree.
    Source,
    /// A provider entry in `<FLEET_HOME>/config.json`.
    Config,
}

impl Target {
    /// The word the frontmatter, the tool, and the row carry.
    pub fn label(self) -> &'static str {
        match self {
            Target::Skill => "skill",
            Target::Memory => "memory",
            Target::Source => "source",
            Target::Config => "config",
        }
    }

    /// Parse the frontmatter's word. `None` names an unknown target.
    pub fn parse(text: &str) -> Option<Target> {
        match text {
            "skill" => Some(Target::Skill),
            "memory" => Some(Target::Memory),
            "source" => Some(Target::Source),
            "config" => Some(Target::Config),
            _ => None,
        }
    }

    /// Whether this target lives in the repository (git-tracked, so the
    /// revert is structural) or in the home (pre-image sidecar).
    pub fn in_repo(self) -> bool {
        matches!(self, Target::Memory | Target::Source)
    }
}

/// A proposal's provenance, in slice 8's vocabulary: `inferred` when
/// the session merely produced the change, `confirmed-by` when the
/// user stated or verified it. Required on every proposal, exactly as
/// `basis` is required on every memory write.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Basis {
    Inferred,
    ConfirmedBy,
}

impl Basis {
    /// The word the frontmatter and the row carry.
    pub fn label(self) -> &'static str {
        match self {
            Basis::Inferred => "inferred",
            Basis::ConfirmedBy => "confirmed-by",
        }
    }

    /// Parse the frontmatter's word. `None` names an unknown basis.
    pub fn parse(text: &str) -> Option<Basis> {
        match text {
            "inferred" => Some(Basis::Inferred),
            "confirmed-by" => Some(Basis::ConfirmedBy),
            _ => None,
        }
    }
}

/// Where a proposal stands with the gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// Filed and waiting for `fleet improve apply`.
    Pending,
    /// The gate was green and the change landed.
    Applied,
    /// The gate (or a validation before it) refused the change.
    Refused,
    /// Sound but waiting on the owner: destructive without approval,
    /// or a confirmed-by memory target.
    Waiting,
}

impl Status {
    /// The word the frontmatter and the surface carry.
    pub fn label(self) -> &'static str {
        match self {
            Status::Pending => "pending",
            Status::Applied => "applied",
            Status::Refused => "refused",
            Status::Waiting => "waiting",
        }
    }

    /// Parse the frontmatter's word. `None` names an unknown status.
    pub fn parse(text: &str) -> Option<Status> {
        match text {
            "pending" => Some(Status::Pending),
            "applied" => Some(Status::Applied),
            "refused" => Some(Status::Refused),
            "waiting" => Some(Status::Waiting),
            _ => None,
        }
    }
}

/// Which store a proposal's file lives in. The two are listed beside
/// each other and never merged: slice 7's drafts keep their home.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Store {
    /// `<FLEET_HOME>/proposals/` — the general store, where the
    /// `improve` tool files.
    General,
    /// `<FLEET_HOME>/skills/proposals/` — slice 7's drafts, the
    /// skill-proposal feedstock.
    SkillsDraft,
}

// --- the change ------------------------------------------------------------

/// What one apply does, parsed from the proposal's change block. The
/// `op` word is the fork's proposal shape: the change is a machine
/// checkable object, not prose a reviewer has to interpret.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Change {
    /// Write a skill's `SKILL.md` (a create, or an update that replaces
    /// the file). Non-destructive.
    WriteSkill { content: String },
    /// Retire a skill: its directory moves aside, out of the live tree,
    /// restorable by the undo. Destructive.
    RetireSkill { name: String },
    /// Slice 8's modify path, wired rather than re-guarded: provenance
    /// decides, and [`memory::Memory::supersede`] enforces it.
    Supersede(Box<memory::Supersede>),
    /// Slice 8's forget path: the fact is expired and the cycle
    /// archives it. Destructive.
    Forget { name: String, justification: String },
    /// Merge a provider entry into `<FLEET_HOME>/config.json`,
    /// validated against the roster's own `deny_unknown_fields`
    /// schema. Non-destructive.
    Config { entry: serde_json::Value },
    /// Remove a provider entry from the config. Destructive.
    ConfigRemove { id: String },
    /// Write a repo file's full new content. The proposal may carry a
    /// human-readable `diff` beside it for the reviewer; the content is
    /// what applies. Non-destructive — the gate is its guard.
    Patch { path: String, content: String },
    /// A change the gate refuses by name rather than guessing at: a
    /// slice-7 merge draft whose body does not name the pair, or any
    /// change object whose `op` is unknown.
    Unsupported { reason: String },
}

impl Change {
    /// The op word, the way the change block and the surface name it.
    pub fn op(&self) -> &'static str {
        match self {
            Change::WriteSkill { .. } => "write-skill",
            Change::RetireSkill { .. } => "retire-skill",
            Change::Supersede(_) => "supersede",
            Change::Forget { .. } => "forget",
            Change::Config { .. } => "config",
            Change::ConfigRemove { .. } => "config-remove",
            Change::Patch { .. } => "patch",
            Change::Unsupported { .. } => "unsupported",
        }
    }

    /// Whether applying this destroys something: a retired skill, a
    /// forgotten fact, or a removed config entry. A destructive change
    /// waits for explicit approval in the current turn.
    pub fn destructive(&self) -> bool {
        matches!(
            self,
            Change::RetireSkill { .. } | Change::Forget { .. } | Change::ConfigRemove { .. }
        )
    }

    /// Exactly what a destructive apply would destroy, for the approval
    /// prompt: the thing itself, named, not a category.
    pub fn destroys(&self) -> String {
        match self {
            Change::RetireSkill { name } => {
                format!("the skill `{name}` (its directory moves aside, restorable by `fleet improve undo`)")
            }
            Change::Forget { name, .. } => format!(
                "the fact `{name}` (it is expired and the cycle archives its file; the corpus is git-tracked)"
            ),
            Change::ConfigRemove { id } => {
                format!("the config entry `{id}` (it is removed from {CONFIG_FILE})")
            }
            _ => String::new(),
        }
    }

    /// The content this change stages, as one string: the artifact the
    /// gate judges. A write-shaped change carries the file's full new
    /// content; every other op carries the identity it acts on — the
    /// skill's name, the fact's name, the config entry's JSON, the
    /// provider id — so every change has one stable artifact and the
    /// verdict row can key an explanation to it (slice 17). The exact
    /// shape matters less than that it is deterministic: the same
    /// change always hashes the same way, so a row written today still
    /// joins tomorrow.
    pub fn artifact_text(&self) -> String {
        match self {
            Change::WriteSkill { content } => content.clone(),
            Change::Patch { path, content } => format!("{path}\n{content}"),
            Change::RetireSkill { name } => format!("retire-skill:{name}"),
            Change::Supersede(request) => format!(
                "supersede:{}:{}",
                request.name,
                request.replacement.clone().unwrap_or_default()
            ),
            Change::Forget { name, justification } => format!("forget:{name}:{justification}"),
            Change::Config { entry } => {
                format!("config:{}", serde_json::to_string(entry).unwrap_or_default())
            }
            Change::ConfigRemove { id } => format!("config-remove:{id}"),
            Change::Unsupported { reason } => format!("unsupported:{reason}"),
        }
    }

    /// The judged artifact's identity: the 16 hex characters of the
    /// sha256 over [`Change::artifact_text`], the devdec join shape.
    /// This is what the `propose` row carries, and what an explanation
    /// checks the artifact against before it spends an ask.
    pub fn artifact_sha(&self) -> String {
        crate::calib::hash16(&self.artifact_text())
    }

    /// The repo-relative paths this change touches, in the order a
    /// draft stamps them. A `patch` names its file; the memory ops name
    /// the fact file they rewrite. Everything else touches no repo
    /// file — a skill lives in the home, a config entry in the home's
    /// `config.json` — so it stamps nothing and guards nothing.
    pub fn target_paths(&self) -> Vec<String> {
        match self {
            Change::Patch { path, .. } => vec![path.clone()],
            Change::Supersede(request) => vec![format!("{MEMORIES_DIR}/{}.md", request.name)],
            Change::Forget { name, .. } => vec![format!("{MEMORIES_DIR}/{name}.md")],
            _ => Vec::new(),
        }
    }
}

/// Parse one change object. A missing or unknown `op`, or a field of
/// the wrong type, is a named refusal — the gate never guesses at what
/// a change means.
pub fn parse_change(value: &serde_json::Value) -> Result<Change, String> {
    let Some(object) = value.as_object() else {
        return Err("the change is not a JSON object".to_string());
    };
    let op = object
        .get("op")
        .and_then(|op| op.as_str())
        .ok_or_else(|| "the change object carries no `op` string".to_string())?;
    let string = |key: &str| -> Result<String, String> {
        object
            .get(key)
            .and_then(|value| value.as_str())
            .map(str::to_string)
            .ok_or_else(|| format!("the `{op}` change needs a string `{key}`"))
    };
    match op {
        "write-skill" => Ok(Change::WriteSkill { content: string("content")? }),
        "retire-skill" => Ok(Change::RetireSkill { name: string("name")? }),
        "supersede" => {
            let request = memory::Supersede::parse(value)
                .map_err(|error| format!("the supersede change is not valid: {error}"))?;
            Ok(Change::Supersede(Box::new(request)))
        }
        "forget" => Ok(Change::Forget { name: string("name")?, justification: string("justification")? }),
        "config" => {
            let entry = object
                .get("entry")
                .cloned()
                .ok_or_else(|| "the `config` change needs an `entry` object".to_string())?;
            Ok(Change::Config { entry })
        }
        "config-remove" => Ok(Change::ConfigRemove { id: string("id")? }),
        "patch" => Ok(Change::Patch { path: string("path")?, content: string("content")? }),
        other => Ok(Change::Unsupported {
            reason: format!("`{other}` is not a change op the gate applies"),
        }),
    }
}

// --- the proposal ----------------------------------------------------------

/// One proposal: its frontmatter, its body's four sections, and the
/// change the gate would apply.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Proposal {
    pub name: String,
    pub target: Target,
    /// The target's name or path, as the proposal names it.
    pub target_name: String,
    pub basis: Basis,
    pub created: String,
    pub status: Status,
    /// Why the change is justified: the session's evidence, the failing
    /// test, or the graded outcome behind it.
    pub justification: String,
    /// What must be true for the change to count as done.
    pub acceptance: String,
    /// What changes for the harness when this lands.
    pub effect: String,
    pub change: Change,
    pub store: Store,
    pub path: PathBuf,
    /// The commit a repo-side apply landed in, for the undo's revert.
    pub commit: Option<String>,
    /// The last gate result, stamped at verdict time: `fmt ok · clippy
    /// FAILED · …`. Empty when the gate has not run.
    pub gate: String,
    /// The target files' content shas as they were when the draft was
    /// filed — the drift guard (slice 17, PS-C). A proposal is written
    /// against the files as they stand; if a target changes before the
    /// apply, the change was reasoned about a different file, and the
    /// apply refuses by name rather than writing it anyway. Empty for a
    /// proposal filed before the field existed, which simply carries no
    /// guard — the guard is additive, never a reason to refuse an old
    /// proposal that still applies cleanly.
    pub target_shas: Vec<TargetSha>,
}

/// One target file's identity at draft time: its repo-relative path and
/// the 16 hex characters of the sha256 over its content, the devdec
/// join shape. Carried in the proposal's frontmatter as
/// `target-shas: <path>=<sha>[,<path>=<sha>…]` — the frontmatter parser
/// is flat `key: value`, so the list rides one line rather than as
/// nested YAML it would refuse.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetSha {
    pub path: String,
    pub sha256: String,
}

impl TargetSha {
    /// Parse the `target-shas` frontmatter value. A pair that is not
    /// `path=sha`, or a sha that is not 16 hex characters, is dropped
    /// rather than guessed at: a guard that cannot read its own value
    /// guards nothing, and a malformed pair must not read as a match.
    pub fn parse_list(value: &str) -> Vec<TargetSha> {
        value
            .split(',')
            .filter_map(|pair| {
                let (path, sha) = pair.trim().split_once('=')?;
                let path = path.trim();
                let sha = sha.trim();
                let hex = sha.len() == 16 && sha.chars().all(|c| c.is_ascii_hexdigit());
                (!path.is_empty() && hex).then(|| TargetSha { path: path.to_string(), sha256: sha.to_string() })
            })
            .collect()
    }

    /// The frontmatter line's value for `shas`, or `None` when there is
    /// nothing to write — a proposal whose change touches no repo file
    /// carries no line at all.
    pub fn render_list(shas: &[TargetSha]) -> Option<String> {
        (!shas.is_empty()).then(|| {
            shas
                .iter()
                .map(|sha| format!("{}={}", sha.path, sha.sha256))
                .collect::<Vec<_>>()
                .join(",")
        })
    }
}

/// The 16 hex characters of the sha256 over `path`'s content under
/// `root`, or `None` when the file is absent or unreadable. A draft
/// stamps what exists; a guard then treats an absent file as the drift
/// it is.
pub fn file_sha(root: &Path, path: &str) -> Option<String> {
    let bytes = fs::read(root.join(path)).ok()?;
    Some(crate::calib::hash16(&String::from_utf8_lossy(&bytes)))
}

/// Stamp every repo-relative path `change` touches with its current
/// sha under `root`. A path that does not exist yet — a `patch` that
/// creates a file — is skipped: there is nothing to drift from.
pub fn stamp_target_shas(root: &Path, change: &Change) -> Vec<TargetSha> {
    change
        .target_paths()
        .iter()
        .filter_map(|path| {
            file_sha(root, path).map(|sha256| TargetSha { path: path.clone(), sha256 })
        })
        .collect()
}

/// The target shas in `proposal` that no longer match `root`: the
/// drift the apply refuses. A path the proposal stamped that has since
/// changed — or vanished — is named; a path the proposal never stamped
/// guards nothing.
pub fn drifted_targets(root: &Path, shas: &[TargetSha]) -> Vec<TargetSha> {
    shas.iter()
        .filter(|stamped| file_sha(root, &stamped.path).as_deref() != Some(stamped.sha256.as_str()))
        .cloned()
        .collect()
}

/// A bare proposal name: not empty, no path separator, not `.` or `..`.
/// The store joins the name into a path, so a name that is really a
/// path would address an arbitrary file.
pub fn bare_name(name: &str) -> Result<&str, String> {
    let invalid = name.is_empty()
        || name == "."
        || name == ".."
        || name.contains('/')
        || name.contains('\\')
        || name.contains('\0');
    if invalid {
        Err(format!("`{name}` is not a proposal name: a name carries no path separator and is not `.` or `..`"))
    } else {
        Ok(name)
    }
}

/// `<home>/proposals`.
pub fn proposals_dir(home: &Path) -> PathBuf {
    home.join(PROPOSALS_DIR)
}

/// `<home>/proposals/preimages`.
pub fn preimages_dir(home: &Path) -> PathBuf {
    proposals_dir(home).join(PREIMAGES_DIR)
}

/// One body section's text: everything between its `## <title>` header
/// and the next `## ` header, trimmed.
fn section(body: &str, title: &str) -> String {
    let header = format!("## {title}");
    let mut inside = false;
    let mut lines: Vec<&str> = Vec::new();
    for line in body.lines() {
        if line.starts_with("## ") {
            if inside {
                break;
            }
            inside = line.trim_end() == header;
            continue;
        }
        if inside {
            lines.push(line);
        }
    }
    lines.join("\n").trim().to_string()
}

/// The change block: the first fenced block after the `## Change`
/// header. `None` when the section or the fence is missing.
fn change_block(body: &str) -> Option<String> {
    let change = section(body, "Change");
    let mut fenced = false;
    let mut lines: Vec<&str> = Vec::new();
    for line in change.lines() {
        if line.trim_start().starts_with("```") {
            if fenced {
                return Some(lines.join("\n"));
            }
            fenced = true;
            continue;
        }
        if fenced {
            lines.push(line);
        }
    }
    None
}

/// Parse one general-store proposal file. The frontmatter is the
/// skills subset, so a proposal is diffable in the same shape a skill
/// is; the rejections are named rather than guessed at.
fn parse_proposal(text: &str, path: &Path, store: Store) -> Result<Proposal, String> {
    let document = skills::parse_document(text)?;
    let name = document.required("name")?;
    let stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or_default()
        .to_string();
    if store == Store::General && name != stem {
        return Err(format!("frontmatter name `{name}` does not match the file `{stem}.md`"));
    }
    let target = document
        .get("target")
        .and_then(Target::parse)
        .ok_or_else(|| "frontmatter `target` is missing or not skill, memory, source, or config".to_string())?;
    let target_name = document.required("target-name")?;
    let basis = document
        .get("basis")
        .and_then(Basis::parse)
        .ok_or_else(|| "frontmatter `basis` is missing or not inferred or confirmed-by".to_string())?;
    let created = document.get("created").unwrap_or("").to_string();
    let status = document
        .get("status")
        .and_then(Status::parse)
        .unwrap_or(Status::Pending);
    let commit = document.get("commit").map(str::to_string);
    let gate = document.get("gate").unwrap_or("").to_string();
    let target_shas = document
        .get("target-shas")
        .map(TargetSha::parse_list)
        .unwrap_or_default();
    let body = &document.body;
    let change_text = change_block(body)
        .ok_or_else(|| "the body carries no fenced change block under `## Change`".to_string())?;
    let change_json: serde_json::Value = serde_json::from_str(&change_text)
        .map_err(|error| format!("the change block is not valid JSON: {error}"))?;
    let change = parse_change(&change_json)?;
    Ok(Proposal {
        name,
        target,
        target_name,
        basis,
        created,
        status,
        justification: section(body, "Justification"),
        acceptance: section(body, "Acceptance criteria"),
        effect: section(body, "Expected effect"),
        change,
        store,
        path: path.to_path_buf(),
        commit,
        gate,
        target_shas,
    })
}

/// One slice-7 draft as the gate sees it: a read-only adaptation, never
/// a migration. The draft's frontmatter is slice 7's own (`name`,
/// `description`, `proposed: true`, `kind`, `evidence`); the proposal
/// view derives the change from its `kind`, and its basis is `inferred`
/// — a drafted lesson is the harness's own inference until someone
/// confirms it. The drafter's bodies carry evidence, not acceptance
/// criteria, so the view's acceptance and effect are empty and the
/// feature doc says so.
fn draft_proposal(path: &Path) -> Option<Proposal> {
    let text = fs::read_to_string(path).ok()?;
    let document = skills::parse_document(&text).ok()?;
    let name = document.required("name").ok()?;
    let kind = document.get("kind").unwrap_or("");
    let evidence = document.get("evidence").unwrap_or("").to_string();
    let status = document.get("status").and_then(Status::parse).unwrap_or(Status::Pending);
    let change = match kind {
        "create" | "update" => {
            // The draft's body is the proposed SKILL.md's body, and its
            // frontmatter carries the live skill's name and
            // description — the shape `render_proposal` wrote.
            let description = document.required("description").unwrap_or_default();
            let content =
                format!("---\nname: {name}\ndescription: {description}\n---\n\n{}\n", document.body.trim_end());
            Change::WriteSkill { content }
        }
        "retire" => Change::RetireSkill { name: name.clone() },
        "merge" => {
            // The drafter writes `# merge: <absorbed> into <survivor>`
            // as the body's first line; the survivor is the draft's
            // own name, so the absorbed name is what a merge apply
            // retires. A body that does not carry that line refuses by
            // name rather than guessing which half to absorb.
            let first = document.body.lines().next().unwrap_or_default();
            let absorbed = first
                .strip_prefix("# merge: ")
                .and_then(|rest| rest.split_once(" into "))
                .map(|(absorbed, _)| absorbed.trim().to_string());
            match absorbed {
                Some(absorbed) if !absorbed.is_empty() => {
                    Change::RetireSkill { name: absorbed }
                }
                _ => Change::Unsupported {
                    reason: "the merge draft's body does not name the pair as `# merge: <absorbed> into <survivor>`".to_string(),
                },
            }
        }
        other => Change::Unsupported {
            reason: format!("`{other}` is not a draft kind the gate applies"),
        },
    };
    Some(Proposal {
        name,
        target: Target::Skill,
        target_name: document
            .required("name")
            .unwrap_or_default(),
        basis: Basis::Inferred,
        created: calib::today(),
        status,
        justification: evidence,
        acceptance: String::new(),
        effect: String::new(),
        change,
        store: Store::SkillsDraft,
        path: path.to_path_buf(),
        commit: None,
        gate: String::new(),
        // A slice-7 draft is not filed against the repo's files — the
        // drafter writes a SKILL.md — so it stamps nothing and guards
        // nothing.
        target_shas: Vec::new(),
    })
}

/// Every proposal in both stores: the general store sorted by name,
/// then slice 7's drafts sorted by name. A malformed file in either
/// store is skipped by name with one logged line, the way the skills
/// loader skips a broken skill — one bad proposal never hides the
/// others.
pub fn load_all(home: &Path) -> Vec<Proposal> {
    let mut proposals: Vec<Proposal> = Vec::new();
    let Ok(entries) = fs::read_dir(proposals_dir(home)) else {
        return Vec::new();
    };
    let mut files: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("md"))
        .collect();
    files.sort();
    for path in files {
        let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()).map(str::to_string) else {
            continue;
        };
        match fs::read_to_string(&path) {
            Ok(text) => match parse_proposal(&text, &path, Store::General) {
                Ok(proposal) => proposals.push(proposal),
                Err(reason) => eprintln!("fleet: improve: skipping `{stem}` \u{2014} {reason}"),
            },
            Err(_) => eprintln!("fleet: improve: skipping `{stem}` \u{2014} no readable proposal file"),
        }
    }
    let Ok(entries) = fs::read_dir(skills::proposals_dir(home)) else {
        return proposals;
    };
    let mut drafts: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("md"))
        .collect();
    drafts.sort();
    for path in drafts {
        let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()).map(str::to_string) else {
            continue;
        };
        match draft_proposal(&path) {
            Some(proposal) => proposals.push(proposal),
            None => eprintln!("fleet: improve: skipping draft `{stem}` \u{2014} it does not read as a slice-7 draft"),
        }
    }
    proposals
}

/// One proposal by name, the general store first, then slice 7's
/// drafts — the order `fleet improve` lists them in.
pub fn load_one(home: &Path, name: &str) -> Option<Proposal> {
    let name = bare_name(name).ok()?;
    let path = proposals_dir(home).join(format!("{name}.md"));
    if let Ok(text) = fs::read_to_string(&path) {
        if let Ok(proposal) = parse_proposal(&text, &path, Store::General) {
            return Some(proposal);
        }
    }
    let draft = skills::proposals_dir(home).join(format!("{name}.md"));
    draft_proposal(&draft)
}

// --- the tool's file path --------------------------------------------------

/// File one proposal from the `improve` tool's arguments. This is the
/// only writer the tool has: the proposal lands in the general store
/// and the gate disposes of it, the way the fork's Creator installs
/// through Plugin Manager rather than through a model tool.
///
/// The result is the model-visible confirmation or the typed refusal,
/// in the same shape a built-in's output takes.
pub fn file_from_tool(home: &Path, repo: &Path, arguments: &serde_json::Value) -> String {
    let field = |key: &str| -> Result<String, String> {
        arguments
            .get(key)
            .and_then(|value| value.as_str())
            .map(str::to_string)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| format!("the `{key}` argument is required"))
    };
    let name = match field("name").and_then(|name| bare_name(&name).map(str::to_string)) {
        Ok(name) => name,
        Err(reason) => return format!("error: {reason}"),
    };
    let target = match field("target") {
        Ok(target) => match Target::parse(&target) {
            Some(target) => target,
            None => {
                return format!(
                    "error: `{target}` is not a target: skill, memory, source, or config"
                )
            }
        },
        Err(reason) => return format!("error: {reason}"),
    };
    let basis = match field("basis") {
        Ok(basis) => match Basis::parse(&basis) {
            Some(basis) => basis,
            None => {
                return format!("error: `{basis}` is not a basis: inferred or confirmed-by")
            }
        },
        Err(reason) => return format!("error: {reason}"),
    };
    let target_name = match field("target-name") {
        Ok(target_name) => target_name,
        Err(reason) => return format!("error: {reason}"),
    };
    let (justification, acceptance, effect, change) = match (
        field("justification"),
        field("acceptance"),
        field("effect"),
        field("change"),
    ) {
        (Ok(justification), Ok(acceptance), Ok(effect), Ok(change)) => {
            (justification, acceptance, effect, change)
        }
        (Err(reason), _, _, _) | (_, Err(reason), _, _) | (_, _, Err(reason), _) | (_, _, _, Err(reason)) => {
            return format!("error: {reason}")
        }
    };
    // The change is validated here, at file time, so a proposal the
    // gate could never apply is refused by the tool that filed it
    // rather than by the gate later.
    match serde_json::from_str::<serde_json::Value>(&change) {
        Err(error) => return format!("error: the change is not valid JSON: {error}"),
        Ok(value) => {
            if let Err(reason) = parse_change(&value) {
                return format!("error: the change is not valid: {reason}");
            }
        }
    }
    let path = proposals_dir(home).join(format!("{name}.md"));
    if path.exists() {
        return format!(
            "error: a proposal named `{name}` is already waiting in {}; the gate disposes of what is waiting",
            proposals_dir(home).display()
        );
    }
    // The change is validated above, so it parses here: the draft
    // stamps its targets' content shas as they stand right now, which
    // is what lets the apply refuse a target that drifted while the
    // proposal waited (slice 17, PS-C).
    let parsed = parse_change(&serde_json::from_str::<serde_json::Value>(&change).unwrap_or_default())
        .unwrap_or_else(|_| Change::Unsupported { reason: String::new() });
    let shas = stamp_target_shas(repo, &parsed);
    let text = render_proposal(
        &name,
        target,
        &target_name,
        basis,
        &justification,
        &acceptance,
        &effect,
        &change,
        &shas,
    );
    if let Err(error) = fs::create_dir_all(proposals_dir(home)).and_then(|()| fs::write(&path, &text)) {
        return format!("error: could not file the proposal: {error}");
    }
    format!(
        "filed proposal `{name}` ({} `{target_name}`, {}) at {} \u{2014} applying it is `fleet improve apply {name}`, through the gate",
        target.label(),
        basis.label(),
        path.display()
    )
}

/// One proposal file's text: the frontmatter the skills parser reads,
/// then the four body sections with the change as a fenced JSON block.
pub(crate) fn render_proposal(
    name: &str,
    target: Target,
    target_name: &str,
    basis: Basis,
    justification: &str,
    acceptance: &str,
    effect: &str,
    change: &str,
    shas: &[TargetSha],
) -> String {
    let mut out = String::new();
    out.push_str("---\n");
    out.push_str(&format!("name: {name}\n"));
    out.push_str(&format!("target: {}\n", target.label()));
    out.push_str(&format!("target-name: {target_name}\n"));
    out.push_str(&format!("basis: {}\n", basis.label()));
    out.push_str(&format!("created: {}\n", calib::today()));
    out.push_str("status: pending\n");
    // The drift guard's line, written only when the change touches a
    // repo file that exists: a proposal whose targets stamped nothing
    // carries no line, and the apply guards nothing.
    if let Some(line) = TargetSha::render_list(shas) {
        out.push_str(&format!("target-shas: {line}\n"));
    }
    out.push_str("---\n\n");
    out.push_str("## Justification\n\n");
    out.push_str(justification.trim());
    out.push_str("\n\n## Acceptance criteria\n\n");
    out.push_str(acceptance.trim());
    out.push_str("\n\n## Expected effect\n\n");
    out.push_str(effect.trim());
    out.push_str("\n\n## Change\n\n```json\n");
    out.push_str(change.trim());
    out.push_str("\n```\n");
    out
}

// --- the gate --------------------------------------------------------------

/// One gate step: its label and the cargo arguments it runs. Owned, so
/// a test can supply its own steps and drive the runner without the
/// real toolchain (and without a `cargo test` inside a `cargo test`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    pub label: String,
    pub args: Vec<String>,
}

/// The local gate: the four steps, in order, run in the repository
/// root. `cargo` is resolved through `path` when it names one, which
/// is how a test points the runner at a stand-in — or at a PATH
/// without the tool, to prove the absent-tool refusal.
#[derive(Debug, Clone)]
pub struct Gate {
    pub root: PathBuf,
    pub cargo: String,
    pub path: Option<String>,
    pub steps: Vec<Step>,
}

impl Gate {
    /// The real gate: fmt, clippy, build, test — in that order, in
    /// `root`, on the ambient toolchain.
    pub fn local(root: impl Into<PathBuf>) -> Gate {
        Gate {
            root: root.into(),
            cargo: "cargo".to_string(),
            path: None,
            steps: vec![
                Step { label: "fmt".to_string(), args: args(&["fmt", "--all", "--check"]) },
                Step {
                    label: "clippy".to_string(),
                    args: args(&["clippy", "--all-targets", "--", "-D", "warnings"]),
                },
                Step { label: "build".to_string(), args: args(&["build", "--release"]) },
                Step { label: "test".to_string(), args: args(&["test"]) },
            ],
        }
    }

    /// Run every step and collect its result. The steps all run even
    /// past a failure: the row carries the whole gate's evidence, and a
    /// proposal refused by fmt is still worth knowing would also have
    /// failed test. Each step's exit code is read directly from its own
    /// status — never through a pipe — and a step whose program cannot
    /// be spawned at all is a named failure, not a skip.
    pub fn run(&self) -> Vec<GateStep> {
        self.steps.iter().map(|step| self.run_step(step)).collect()
    }

    fn run_step(&self, step: &Step) -> GateStep {
        let mut command = std::process::Command::new(&self.cargo);
        command.args(&step.args).current_dir(&self.root);
        if let Some(path) = &self.path {
            command.env("PATH", path);
        }
        match command.output() {
            Err(error) => GateStep {
                label: step.label.clone(),
                ok: false,
                detail: format!("could not run `{}`: {error}", self.cargo),
            },
            Ok(output) => GateStep {
                label: step.label.clone(),
                ok: output.status.success(),
                detail: if output.status.success() {
                    "ok".to_string()
                } else {
                    bounded_failure(&output)
                },
            },
        }
    }
}

/// A step list from static args.
fn args(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| value.to_string()).collect()
}

/// One step's result: its label, whether it passed, and the bounded
/// detail — "ok", or the failure's own last line, named.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateStep {
    pub label: String,
    pub ok: bool,
    pub detail: String,
}

impl GateStep {
    /// This step's evidence as a row carries it.
    fn row(&self) -> calib::GateStepRow {
        calib::GateStepRow {
            step: self.label.clone(),
            ok: self.ok,
            detail: self.detail.clone(),
        }
    }
}

/// The bounded detail of a failed step: the last line that names an
/// error, or the last line when none does, cut to
/// [`MAX_DETAIL_CHARS`] with a marker. Cargo puts its reason on an
/// `error:` line and its advice on `help:` lines after it — the reason
/// is the one that names the failure, so it wins over the last line.
fn bounded_failure(output: &std::process::Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let source = if stderr.trim().is_empty() { stdout.to_string() } else { stderr.to_string() };
    let last = source
        .lines()
        .rev()
        .find(|line| line.trim_start().starts_with("error"))
        .or_else(|| source.lines().last())
        .unwrap_or_default()
        .trim()
        .to_string();
    let mut bounded: String = last.chars().take(MAX_DETAIL_CHARS).collect();
    if last.chars().count() > MAX_DETAIL_CHARS {
        bounded.push('\u{2026}');
    }
    if bounded.is_empty() {
        bounded.push_str(&format!("exited {}", output.status));
    }
    bounded
}

/// The gate's steps as one bounded line, for the frontmatter stamp:
/// `fmt ok · clippy FAILED · build ok · test ok`.
fn gate_summary(steps: &[GateStep]) -> String {
    steps
        .iter()
        .map(|step| format!("{} {}", step.label, if step.ok { "ok" } else { "FAILED" }))
        .collect::<Vec<_>>()
        .join(" \u{b7} ")
}

/// The refusal's named failure: every failed step and the first one's
/// detail, so the refusal says both what failed and why.
fn named_failure(steps: &[GateStep]) -> String {
    let failed: Vec<&GateStep> = steps.iter().filter(|step| !step.ok).collect();
    let names = failed.iter().map(|step| step.label.as_str()).collect::<Vec<_>>().join(", ");
    let detail = failed.first().map(|step| step.detail.as_str()).unwrap_or("");
    if detail.is_empty() {
        format!("the gate refused the change: {names} failed")
    } else {
        format!("the gate refused the change: {names} failed \u{2014} {detail}")
    }
}

/// The eval-corpus gate step from one probe run: `ok` when the corpus is
/// green, and a failure whose detail names the regressing scenario (the
/// probe's `Err` reason) when it is not. The detail is bounded the way a
/// cargo step's is — the reason names the failure; the whole report is
/// `fleet eval`'s, not the ledger's.
fn eval_gate_step(probe: &eval::EvalProbe) -> GateStep {
    match probe() {
        Ok(()) => GateStep { label: "eval_corpus".to_string(), ok: true, detail: "ok".to_string() },
        Err(reason) => {
            let mut bounded: String = reason.chars().take(MAX_DETAIL_CHARS).collect();
            if reason.chars().count() > MAX_DETAIL_CHARS {
                bounded.push('\u{2026}');
            }
            GateStep { label: "eval_corpus".to_string(), ok: false, detail: bounded }
        }
    }
}

// --- the verdict -----------------------------------------------------------

/// What the gate did with one proposal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// The gate was green and the change landed.
    Applied { summary: String, commit: Option<String> },
    /// The gate, or a validation before it, refused the change; the
    /// reason names the failure.
    Refused { reason: String },
    /// The proposal is sound but waits on the owner; the reason names
    /// exactly what will be destroyed, or which fact is confirmed-by.
    Waiting { reason: String },
}

impl Verdict {
    /// The verdict's word, the way the row carries it.
    pub fn word(&self) -> &'static str {
        match self {
            Verdict::Applied { .. } => VERDICT_APPLIED,
            Verdict::Refused { .. } => VERDICT_REFUSED,
            Verdict::Waiting { .. } => VERDICT_WAITING,
        }
    }

    /// The verdict's status, for the frontmatter stamp.
    fn status(&self) -> Status {
        match self {
            Verdict::Applied { .. } => Status::Applied,
            Verdict::Refused { .. } => Status::Refused,
            Verdict::Waiting { .. } => Status::Waiting,
        }
    }

    /// The verdict's reason, bounded for the row. An apply's summary is
    /// its reason; a refusal or a wait names its own.
    fn reason(&self) -> String {
        let text = match self {
            Verdict::Applied { summary, .. } => summary.clone(),
            Verdict::Refused { reason } => reason.clone(),
            Verdict::Waiting { reason } => reason.clone(),
        };
        let mut bounded: String = text.chars().take(MAX_DETAIL_CHARS).collect();
        if text.chars().count() > MAX_DETAIL_CHARS {
            bounded.push('\u{2026}');
        }
        bounded
    }

    /// The one line the surface prints.
    pub fn line(&self) -> String {
        match self {
            Verdict::Applied { summary, .. } => format!("applied \u{2014} {summary}"),
            Verdict::Refused { reason } => format!("refused \u{2014} {reason}"),
            Verdict::Waiting { reason } => format!("waiting \u{2014} {reason}"),
        }
    }
}

// --- the apply -------------------------------------------------------------

/// Everything one apply needs beyond the proposal: the home, the repo
/// the gate runs in, the gate itself, whether the owner approved a
/// destructive change in this turn, and the stream the verdict lands
/// in.
pub struct Apply<'a> {
    pub home: &'a Path,
    pub repo: &'a Path,
    pub gate: &'a Gate,
    /// Approval in the current turn: `fleet improve apply --approve`.
    pub approved: bool,
    pub stream: Option<&'a Stream>,
    /// The eval-corpus gate probe, run as a gate step for a **source**
    /// target only: `Ok(())` when the corpus is green, `Err` naming the
    /// regressing scenario when it is not. `None` skips the corpus step
    /// — a test that is exercising something else, or a machine that has
    /// not wired one. Production always wires it (and the production
    /// probe fails closed rather than skipping), so a real source
    /// self-edit is gated on the corpus.
    pub eval: Option<eval::EvalProbe>,
}

/// A pre-gate outcome: the proposal waits on the owner, or is refused
/// before anything is written.
enum Hold {
    Waiting(String),
    Refused(String),
}

/// What one staged apply did to the tree, so the gate's verdict can
/// keep it or undo it.
struct Landed {
    /// Repo-relative paths a repo-side change touched. Empty for a
    /// home-side change, which reverts through its pre-image instead.
    repo_paths: Vec<String>,
    /// The home-side pre-image, kept for the undo (and consumed by a
    /// failed gate's revert).
    restore: Option<Restore>,
    /// What landed, for the apply's summary line.
    summary: String,
}

/// One home-side pre-image: where it is stashed, where it goes back,
/// and how.
struct Restore {
    stash: PathBuf,
    target: PathBuf,
    kind: RestoreKind,
}

enum RestoreKind {
    /// A file: the stash's bytes go back over the target, or the
    /// target is deleted when the apply was a create.
    File { create: bool },
    /// A directory the apply moved aside: the undo moves it back.
    Moved,
}

impl Restore {
    /// Put the pre-image back. The stash is consumed: after a restore
    /// there is nothing left to undo a second time.
    fn apply(&self) {
        match &self.kind {
            RestoreKind::File { create: true } => {
                let _ = fs::remove_file(&self.target);
                if let Some(parent) = self.target.parent() {
                    let _ = fs::remove_dir(parent);
                }
                let _ = fs::remove_file(&self.stash);
            }
            RestoreKind::File { create: false } => {
                if let Ok(bytes) = fs::read(&self.stash) {
                    let _ = fs::write(&self.target, bytes);
                }
                let _ = fs::remove_file(&self.stash);
            }
            RestoreKind::Moved => {
                if self.target.exists() {
                    let _ = fs::remove_dir_all(&self.target);
                }
                let _ = fs::rename(&self.stash, &self.target);
            }
        }
    }
}

impl Landed {
    /// Commit the repo-side change with explicit paths naming the
    /// proposal, and return the commit the undo reverts. A home-side
    /// change commits nothing — its pre-image is the undo.
    fn commit(&self, repo: &Path) -> Result<Option<String>, String> {
        if self.repo_paths.is_empty() {
            return Ok(None);
        }
        let paths: Vec<&str> = self.repo_paths.iter().map(String::as_str).collect();
        git_paths(repo, &["add", "--"], &paths, &[])?;
        git_paths(repo, &["commit", "-m", &format!("fleet improve: {}", self.summary), "--"], &paths, &[])?;
        git(repo, &["rev-parse", "HEAD"]).map(Some)
    }

    /// Undo the change: repo-side paths revert to committed state
    /// structurally, and a home-side pre-image goes back.
    fn revert(&self, repo: &Path) {
        for path in &self.repo_paths {
            revert_path(repo, path);
        }
        if let Some(restore) = &self.restore {
            restore.apply();
        }
    }
}

/// Apply one proposal through the gate. The order is the design's own:
/// load and validate, ask for approval when the change is destructive
/// (or the memory target is confirmed-by), apply to the tree, run the
/// full gate, and keep the change only when every step is green —
/// reverting and naming the failure otherwise. Every verdict lands
/// exactly one `propose` row and stamps the proposal's status.
pub fn apply(name: &str, ctx: &Apply<'_>) -> Verdict {
    let Some(proposal) = load_one(ctx.home, name) else {
        return Verdict::Refused {
            reason: format!(
                "no proposal named `{name}` under {} or {}",
                proposals_dir(ctx.home).display(),
                skills::proposals_dir(ctx.home).display()
            ),
        };
    };
    match proposal.status {
        Status::Pending => {}
        // A proposal left waiting waits on approval, so the approval in
        // this turn is exactly what unblocks it.
        Status::Waiting if ctx.approved => {}
        Status::Waiting => {
            return Verdict::Refused {
                reason: format!(
                    "`{name}` is already waiting for approval \u{2014} re-run `fleet improve apply {name} --approve` to confirm"
                ),
            }
        }
        // A proposal the gate has already disposed of is not re-run:
        // the ledger keeps its one row.
        other => {
            return Verdict::Refused {
                reason: format!("`{name}` is already {}", other.label()),
            }
        }
    }
    let landed = match stage(&proposal, ctx) {
        Ok(landed) => landed,
        Err(Hold::Waiting(reason)) => {
            return finish(&proposal, Verdict::Waiting { reason }, &[], ctx)
        }
        Err(Hold::Refused(reason)) => {
            return finish(&proposal, Verdict::Refused { reason }, &[], ctx)
        }
    };
    let mut steps = ctx.gate.run();
    let gate_green = steps.iter().all(|step| step.ok);
    // The eval-corpus gate step, for a source edit only and only when a
    // probe is wired: the four cargo steps have already rebuilt the
    // binary, so the corpus grades the edited code, and a regression
    // refuses the apply with the scenario named — the AC that closes the
    // loop with slice 12. A skill/memory/config target's gate is
    // unchanged: no corpus step, the same four steps. The signal is the
    // change itself — a `patch` is the one change that edits the repo's
    // source — so it holds whatever the frontmatter's `target` word says.
    let is_source_edit = matches!(proposal.change, Change::Patch { .. });
    let eval_regressed = if is_source_edit {
        ctx.eval.as_ref().map(|probe| {
            if gate_green {
                let step = eval_gate_step(probe);
                let regressed = !step.ok;
                steps.push(step);
                regressed
            } else {
                // The cargo gate refused first, so the corpus never
                // ran. The step is recorded not-run — `ok: false`, the
                // detail naming why — so the row says the corpus was
                // not reached rather than leaving the step absent and
                // an operator unable to tell "not reached" from
                // "passed". The refusal itself names the cargo steps.
                steps.push(GateStep {
                    label: "eval_corpus".to_string(),
                    ok: false,
                    detail: "not run \u{2014} the cargo gate refused first".to_string(),
                });
                false
            }
        })
        .unwrap_or(false)
    } else {
        false
    };
    let verdict = if gate_green && !eval_regressed {
        match landed.commit(ctx.repo) {
            Ok(commit) => Verdict::Applied { summary: landed.summary.clone(), commit },
            Err(reason) => {
                landed.revert(ctx.repo);
                Verdict::Refused { reason: format!("the change landed but its commit did not: {reason}") }
            }
        }
    } else {
        landed.revert(ctx.repo);
        Verdict::Refused { reason: named_failure(&steps) }
    };
    finish(&proposal, verdict, &steps, ctx)
}

/// Record the verdict's row, stamp the proposal's frontmatter, and
/// hand the verdict back.
fn finish(proposal: &Proposal, verdict: Verdict, steps: &[GateStep], ctx: &Apply<'_>) -> Verdict {
    if let Some(stream) = ctx.stream {
        stream.record(calib::Row::Propose(calib::ProposeRow {
            ts: calib::stamp(),
            proposal: proposal.name.clone(),
            target: proposal.target.label().to_string(),
            target_name: proposal.target_name.clone(),
            basis: proposal.basis.label().to_string(),
            verdict: verdict.word().to_string(),
            reason: verdict.reason(),
            gate: steps.iter().map(GateStep::row).collect(),
            // The judged artifact's identity, so the verdict is
            // explainable: the explanation beat joins on exactly this
            // sha (slice 17).
            input_sha256: Some(proposal.change.artifact_sha()),
        }));
    }
    let commit = match &verdict {
        Verdict::Applied { commit, .. } => commit.as_deref(),
        _ => None,
    };
    let _ = stamp(&proposal.path, verdict.status(), commit, &gate_summary(steps));
    verdict
}

/// Validate the change against its target and apply it to the tree.
/// Every refusal here happens before anything is written, so a
/// proposal the gate could never apply leaves the tree untouched.
fn stage(proposal: &Proposal, ctx: &Apply<'_>) -> Result<Landed, Hold> {
    // A destructive change waits for explicit approval in the current
    // turn, and the wait names exactly what will be destroyed.
    if proposal.change.destructive() && !ctx.approved {
        return Err(Hold::Waiting(format!(
            "`{}` is destructive and would destroy {}; re-run `fleet improve apply {} --approve` to confirm",
            proposal.name,
            proposal.change.destroys(),
            proposal.name
        )));
    }
    // PS-C's drift guard, beside the dirty-tree guard: the draft was
    // filed against the targets as they stood, and a target that
    // changed while the proposal waited is a different file than the
    // one the change was reasoned about. A clean tree is not the same
    // as the same file — the sha is what tells the two apart — so the
    // apply refuses by name rather than writing a change reasoned
    // about a file that no longer exists in that shape.
    let drifted = drifted_targets(&ctx.repo, &proposal.target_shas);
    if !drifted.is_empty() {
        let named = drifted
            .iter()
            .map(|stamped| {
                let now = file_sha(&ctx.repo, &stamped.path)
                    .unwrap_or_else(|| "absent".to_string());
                format!("`{}` (drafted at {}, now {now})", stamped.path, stamped.sha256)
            })
            .collect::<Vec<_>>()
            .join("; ");
        return Err(Hold::Refused(format!(
            "the proposal's target drifted since the draft: {named} — re-file the proposal against the current file rather than applying a change reasoned about a different one"
        )));
    }
    match &proposal.change {
        Change::Unsupported { reason } => {
            Err(Hold::Refused(format!("the change cannot be applied: {reason}")))
        }
        Change::WriteSkill { content } => stage_write_skill(proposal, content, ctx),
        Change::RetireSkill { name } => stage_retire_skill(proposal, name, ctx),
        Change::Supersede(request) => stage_memory(proposal, request, ctx),
        Change::Forget { name, justification } => stage_memory(
            proposal,
            &memory::Supersede {
                name: name.clone(),
                justification: justification.clone(),
                replacement: None,
                description: String::new(),
                body: String::new(),
                fact_type: None,
                shape: None,
                missing: None,
                valid_until: None,
            },
            ctx,
        ),
        Change::Config { entry } => stage_config(proposal, entry, ctx),
        Change::ConfigRemove { id } => stage_config_remove(proposal, id, ctx),
        Change::Patch { path, content } => stage_patch(proposal, path, content, ctx),
    }
}

/// Stage a skill write: validate the content as a live skill, keep the
/// target's pre-image, and write the file.
fn stage_write_skill(proposal: &Proposal, content: &str, ctx: &Apply<'_>) -> Result<Landed, Hold> {
    let name = &proposal.target_name;
    bare_name(name).map_err(Hold::Refused)?;
    let document = skills::parse_document(content)
        .map_err(|reason| Hold::Refused(format!("the proposed SKILL.md does not parse: {reason}")))?;
    let front = document
        .required("name")
        .map_err(|reason| Hold::Refused(format!("the proposed SKILL.md does not parse: {reason}")))?;
    if front != *name {
        return Err(Hold::Refused(format!(
            "the proposed SKILL.md names `{front}`, but the proposal targets `{name}`"
        )));
    }
    if document.get("proposed") == Some("true") {
        return Err(Hold::Refused(
            "the proposed SKILL.md carries `proposed: true` \u{2014} that is a draft, not a live skill".to_string(),
        ));
    }
    let target = skills::skills_dir(ctx.home).join(name).join(skills::SKILL_FILE);
    let pre = fs::read_to_string(&target).ok();
    let stash = preimages_dir(ctx.home).join(format!("{}.md", proposal.name));
    if let Some(text) = &pre {
        stash_bytes(&stash, text.as_bytes())?;
    }
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| Hold::Refused(format!("could not create {}: {error}", parent.display())))?;
    }
    fs::write(&target, content)
        .map_err(|error| Hold::Refused(format!("could not write {}: {error}", target.display())))?;
    Ok(Landed {
        repo_paths: Vec::new(),
        restore: Some(Restore {
            stash,
            target,
            kind: RestoreKind::File { create: pre.is_none() },
        }),
        summary: format!(
            "wrote the skill `{name}` (proposal `{}`)",
            proposal.name
        ),
    })
}

/// Stage a skill retirement: the directory moves aside, out of the
/// live tree and restorable by the undo. Moving rather than deleting
/// is what makes the destruction reversible.
fn stage_retire_skill(proposal: &Proposal, name: &str, ctx: &Apply<'_>) -> Result<Landed, Hold> {
    bare_name(name).map_err(Hold::Refused)?;
    let directory = skills::skills_dir(ctx.home).join(name);
    if !directory.is_dir() {
        return Err(Hold::Refused(format!(
            "no skill named `{name}` under {}",
            skills::skills_dir(ctx.home).display()
        )));
    }
    let stash = preimages_dir(ctx.home).join(format!("{}.retired", proposal.name));
    fs::create_dir_all(preimages_dir(ctx.home))
        .map_err(|error| Hold::Refused(format!("could not create the pre-image store: {error}")))?;
    if stash.exists() {
        fs::remove_dir_all(&stash)
            .map_err(|error| Hold::Refused(format!("could not replace the old pre-image: {error}")))?;
    }
    fs::rename(&directory, &stash).map_err(|error| {
        Hold::Refused(format!("could not move the skill aside: {error}"))
    })?;
    Ok(Landed {
        repo_paths: Vec::new(),
        restore: Some(Restore { stash, target: directory, kind: RestoreKind::Moved }),
        summary: format!(
            "retired the skill `{name}` (proposal `{}`) \u{2014} moved aside, restorable by `fleet improve undo`",
            proposal.name
        ),
    })
}

/// Stage a memory change through slice 8's own path. The provenance
/// rule is wired, not reimplemented: the basis is read from the corpus
/// to decide waiting-versus-apply, and [`memory::Memory::supersede`]
/// enforces the same rule on the write itself. The stream is
/// deliberately not handed to `supersede`: a gate revert after a
/// `consolidate` row would claim an action the files no longer show,
/// so this path's single record is the `propose` verdict row, with the
/// justification in its reason.
fn stage_memory(
    proposal: &Proposal,
    request: &memory::Supersede,
    ctx: &Apply<'_>,
) -> Result<Landed, Hold> {
    let root = ctx.repo.join(MEMORIES_DIR);
    let facts = memory::load_facts(&root);
    let Some(fact) = facts.iter().find(|fact| fact.name == request.name) else {
        return Err(Hold::Refused(format!(
            "no fact named `{}` in {}",
            request.name,
            root.display()
        )));
    };
    if fact.basis == memory::Basis::ConfirmedBy && !ctx.approved {
        return Err(Hold::Waiting(format!(
            "`{}` is confirmed-by \u{2014} a load-bearing fact \u{2014} so the proposal waits for approval: re-run `fleet improve apply {} --approve`",
            fact.name, proposal.name
        )));
    }
    // The same dirty-tree guard the patch path carries: the supersede
    // rewrites the fact files, and a red gate's revert restores HEAD
    // over them.
    let dirty = dirty_paths(&ctx.repo, &[&format!("{MEMORIES_DIR}/{}.md", fact.name)]);
    if !dirty.is_empty() {
        return Err(Hold::Refused(format!(
            "`{MEMORIES_DIR}/{}.md` carries uncommitted work ({}) — commit or stash it first; the apply's revert would restore HEAD over it",
            fact.name,
            dirty.join(", ")
        )));
    }
    let memory = Memory::load_with(&root, Box::new(memory::KeywordOnly));
    match memory.supersede(request, ctx.approved, None) {
        Err(memory::WriteError::NeedsApproval(reason)) => Err(Hold::Waiting(reason)),
        Err(error) => Err(Hold::Refused(format!("the memory change did not land: {error}"))),
        Ok(superseded) => {
            let (paths, summary) = match &superseded {
                memory::Superseded::By { from, into } => (
                    vec![format!("{MEMORIES_DIR}/{from}.md"), format!("{MEMORIES_DIR}/{into}.md")],
                    format!("superseded the fact `{from}` by `{into}` (proposal `{}`)", proposal.name),
                ),
                memory::Superseded::Forgotten { fact } => (
                    vec![format!("{MEMORIES_DIR}/{fact}.md")],
                    format!(
                        "forgot the fact `{fact}` (proposal `{}`) \u{2014} expired, and the cycle archives the file",
                        proposal.name
                    ),
                ),
            };
            Ok(Landed { repo_paths: paths, restore: None, summary })
        }
    }
}

/// Stage a config change: the entry validates against the roster's own
/// `deny_unknown_fields` schema, then merges into `config.json` by id.
/// The file lives in the home, not the repo, so its pre-image is a
/// sidecar and its undo is a restore.
fn stage_config(proposal: &Proposal, entry: &serde_json::Value, ctx: &Apply<'_>) -> Result<Landed, Hold> {
    crate::route::validate_provider_entry(entry).map_err(|reason| {
        Hold::Refused(format!("the config change is not a valid provider entry: {reason}"))
    })?;
    let path = ctx.home.join(CONFIG_FILE);
    let pre = fs::read_to_string(&path).ok();
    let mut document: serde_json::Value = match &pre {
        Some(text) => serde_json::from_str(text)
            .map_err(|error| Hold::Refused(format!("the existing {CONFIG_FILE} does not parse: {error}")))?,
        None => serde_json::json!({}),
    };
    let id = entry.get("id").and_then(|id| id.as_str()).unwrap_or_default().to_string();
    let providers = document
        .as_object_mut()
        .ok_or_else(|| Hold::Refused(format!("the existing {CONFIG_FILE} is not an object")))?
        .entry("providers")
        .or_insert_with(|| serde_json::json!([]));
    let array = providers
        .as_array_mut()
        .ok_or_else(|| Hold::Refused(format!("the existing {CONFIG_FILE} has a `providers` that is not a list")))?;
    match array.iter_mut().find(|existing| existing.get("id").and_then(|id| id.as_str()) == Some(id.as_str())) {
        Some(existing) => *existing = entry.clone(),
        None => array.push(entry.clone()),
    }
    write_atomic(&path, &serde_json::to_string_pretty(&document).unwrap_or_default())
        .map_err(|reason| Hold::Refused(reason))?;
    let stash = preimages_dir(ctx.home).join(format!("{}.config.json", proposal.name));
    if let Some(text) = &pre {
        stash_bytes(&stash, text.as_bytes())?;
    }
    Ok(Landed {
        repo_paths: Vec::new(),
        restore: Some(Restore { stash, target: path, kind: RestoreKind::File { create: pre.is_none() } }),
        summary: format!("merged the config entry `{id}` (proposal `{}`)", proposal.name),
    })
}

/// Stage a config removal: the entry leaves `config.json`, its
/// pre-image is a sidecar, and the undo restores it.
fn stage_config_remove(proposal: &Proposal, id: &str, ctx: &Apply<'_>) -> Result<Landed, Hold> {
    let path = ctx.home.join(CONFIG_FILE);
    let Some(pre) = fs::read_to_string(&path).ok() else {
        return Err(Hold::Refused(format!("no {CONFIG_FILE} under {}", ctx.home.display())));
    };
    let mut document: serde_json::Value = serde_json::from_str(&pre)
        .map_err(|error| Hold::Refused(format!("the existing {CONFIG_FILE} does not parse: {error}")))?;
    let array = document
        .get_mut("providers")
        .and_then(|providers| providers.as_array_mut())
        .ok_or_else(|| Hold::Refused(format!("the existing {CONFIG_FILE} has no `providers` list")))?;
    let before = array.len();
    array.retain(|existing| existing.get("id").and_then(|id| id.as_str()) != Some(id));
    if array.len() == before {
        return Err(Hold::Refused(format!("no config entry named `{id}` in {}", path.display())));
    }
    write_atomic(&path, &serde_json::to_string_pretty(&document).unwrap_or_default())
        .map_err(|reason| Hold::Refused(reason))?;
    let stash = preimages_dir(ctx.home).join(format!("{}.config.json", proposal.name));
    stash_bytes(&stash, pre.as_bytes())?;
    Ok(Landed {
        repo_paths: Vec::new(),
        restore: Some(Restore {
            stash,
            target: path.clone(),
            kind: RestoreKind::File { create: false },
        }),
        summary: format!("removed the config entry `{id}` (proposal `{}`)", proposal.name),
    })
}

/// Stage a source patch: the path must be repo-relative and inside the
/// repository, the content is the file's whole new text, and the
/// pre-image is committed state — the revert is `git checkout HEAD`.
fn stage_patch(proposal: &Proposal, path: &str, content: &str, ctx: &Apply<'_>) -> Result<Landed, Hold> {
    let relative = Path::new(path);
    let unsafe_path = path.is_empty()
        || relative.is_absolute()
        || path.starts_with('/')
        || relative.components().any(|component| {
            matches!(component, std::path::Component::ParentDir | std::path::Component::Prefix(_))
        });
    if unsafe_path {
        return Err(Hold::Refused(format!(
            "`{path}` is not a repo-relative path inside the repository"
        )));
    }
    // The memory substrate's changes go through `supersede`, whose
    // provenance rule is the permission model: a confirmed-by fact
    // cannot be rewritten by a path the guards above never consulted.
    // A patch aimed at `memories/` is that bypass in one line of JSON,
    // so it is refused here — the proposal's `supersede` op is the only
    // door, and it asks the fact's basis before it writes.
    if relative.components().next().and_then(|component| component.as_os_str().to_str())
        == Some(MEMORIES_DIR)
    {
        return Err(Hold::Refused(format!(
            "`{path}` is under `{MEMORIES_DIR}/`: a fact changes through the `supersede` op, whose provenance rule is the permission model — a patch would bypass it"
        )));
    }
    let dirty = dirty_paths(&ctx.repo, &[path]);
    if !dirty.is_empty() {
        return Err(Hold::Refused(format!(
            "`{path}` carries uncommitted work ({}) — the apply overwrites the target before the gate runs, and a red gate's revert restores HEAD over it, so the work would be silently destroyed; commit or stash it first",
            dirty.join(", ")
        )));
    }
    let target = ctx.repo.join(relative);
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| Hold::Refused(format!("could not create {}: {error}", parent.display())))?;
    }
    fs::write(&target, content)
        .map_err(|error| Hold::Refused(format!("could not write {}: {error}", target.display())))?;
    Ok(Landed {
        repo_paths: vec![path.to_string()],
        restore: None,
        summary: format!("patched {} (proposal `{}`)", target.display(), proposal.name),
    })
}

/// The paths among `paths` carrying uncommitted work in `repo`. The
/// apply overwrites a target before the gate runs and a red gate's
/// revert restores HEAD, so pre-existing uncommitted work in a target
/// would be silently destroyed — the apply refuses instead. An
/// unreadable or absent git answers "nothing dirty": the gate is the
/// authority on what lands, not this guard.
fn dirty_paths(repo: &Path, paths: &[&str]) -> Vec<String> {
    let output = std::process::Command::new("git")
        .args(["status", "--porcelain", "--"])
        .args(paths)
        .current_dir(repo)
        .output();
    let Ok(output) = output else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| line.get(3..))
        .map(|path| path.trim().to_string())
        .filter(|path| !path.is_empty())
        .collect()
}

/// Write `bytes` to the pre-image store, creating it when missing.
fn stash_bytes(stash: &Path, bytes: &[u8]) -> Result<(), Hold> {
    if let Some(parent) = stash.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| Hold::Refused(format!("could not create the pre-image store: {error}")))?;
    }
    fs::write(stash, bytes)
        .map_err(|error| Hold::Refused(format!("could not keep the pre-image: {error}")))
}

/// Write `text` to `path` through a temp file and a rename, so a
/// reader never sees a half-written config.
fn write_atomic(path: &Path, text: &str) -> Result<(), String> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)
        .map_err(|error| format!("could not create {}: {error}", parent.display()))?;
    let mut temp = path.as_os_str().to_os_string();
    temp.push(format!(".{}.tmp", std::process::id()));
    let temp = PathBuf::from(temp);
    fs::write(&temp, text).map_err(|error| format!("could not write {}: {error}", temp.display()))?;
    fs::rename(&temp, path).map_err(|error| {
        let _ = fs::remove_file(&temp);
        format!("could not replace {}: {error}", path.display())
    })
}

// --- the undo --------------------------------------------------------------

/// Revert one applied change. A repo-side apply reverts structurally —
/// `git revert` of the commit the apply landed in — and a home-side
/// apply restores its pre-image byte-for-byte (or deletes what a
/// create added). The proposal's status returns to `pending`: the
/// change is not in effect, and the ledger keeps the record that it
/// once was.
pub fn undo(name: &str, home: &Path, repo: &Path) -> Result<String, String> {
    let name = bare_name(name)?;
    let Some(proposal) = load_one(home, name) else {
        return Err(format!(
            "no proposal named `{name}` under {} or {}",
            proposals_dir(home).display(),
            skills::proposals_dir(home).display()
        ));
    };
    if proposal.status != Status::Applied {
        return Err(format!(
            "`{name}` is not applied (status {}), so there is nothing to undo",
            proposal.status.label()
        ));
    }
    let summary = match &proposal.commit {
        Some(commit) => {
            git(repo, &["revert", "--no-edit", commit])?;
            format!("reverted commit {commit}")
        }
        None => {
            let Some(restore) = undo_restore(home, &proposal) else {
                return Err(format!(
                    "`{name}` is a repo-side change with no recorded commit, so there is nothing structural to revert"
                ));
            };
            restore.apply();
            format!("restored the pre-image of `{}`", proposal.target_name)
        }
    };
    stamp(&proposal.path, Status::Pending, None, "")?;
    Ok(format!("undid `{name}` \u{2014} {summary}"))
}

/// The pre-image an applied home-side proposal keeps for its undo.
/// `None` for a repo-side target, whose undo is the commit's revert.
fn undo_restore(home: &Path, proposal: &Proposal) -> Option<Restore> {
    let store = preimages_dir(home);
    match proposal.target {
        Target::Skill => {
            let skill = skills::skills_dir(home).join(&proposal.target_name);
            let moved = store.join(format!("{}.retired", proposal.name));
            if moved.exists() {
                return Some(Restore { stash: moved, target: skill, kind: RestoreKind::Moved });
            }
            let file = store.join(format!("{}.md", proposal.name));
            let target = skill.join(skills::SKILL_FILE);
            if file.exists() {
                Some(Restore { stash: file, target, kind: RestoreKind::File { create: false } })
            } else {
                // A create kept no pre-image: the undo deletes what
                // the apply added.
                Some(Restore {
                    stash: PathBuf::new(),
                    target,
                    kind: RestoreKind::File { create: true },
                })
            }
        }
        Target::Config => {
            let file = store.join(format!("{}.config.json", proposal.name));
            let target = home.join(CONFIG_FILE);
            if file.exists() {
                Some(Restore { stash: file, target, kind: RestoreKind::File { create: false } })
            } else {
                Some(Restore {
                    stash: PathBuf::new(),
                    target,
                    kind: RestoreKind::File { create: true },
                })
            }
        }
        Target::Source | Target::Memory => None,
    }
}

// --- the frontmatter stamp -------------------------------------------------

/// Rewrite one proposal file's verdict fields in place: `status`, the
/// apply's `commit` when it has one, and the last `gate` line when the
/// gate ran. Every other line is preserved, so the file stays the
/// diff-able record it was filed as.
fn stamp(path: &Path, status: Status, commit: Option<&str>, gate: &str) -> Result<(), String> {
    let text = fs::read_to_string(path)
        .map_err(|error| format!("could not read {}: {error}", path.display()))?;
    let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
    // The frontmatter is the fence pair; a file without one is not a
    // proposal this function can stamp.
    let Some(open) = lines.iter().position(|line| line.trim_end() == "---") else {
        return Err(format!("{} has no frontmatter fence", path.display()));
    };
    let Some(close) = lines.iter().skip(open + 1).position(|line| line.trim_end() == "---") else {
        return Err(format!("{} has no closing frontmatter fence", path.display()));
    };
    let close = open + 1 + close;
    let kept: Vec<String> = lines[open + 1..close]
        .iter()
        .filter(|line| {
            let key = line.split(':').next().unwrap_or_default().trim();
            !matches!(key, "status" | "commit" | "gate")
        })
        .cloned()
        .collect();
    let mut stamped: Vec<String> = Vec::new();
    stamped.push(format!("status: {}", status.label()));
    if let Some(commit) = commit {
        stamped.push(format!("commit: {commit}"));
    }
    if !gate.is_empty() {
        stamped.push(format!("gate: {gate}"));
    }
    stamped.extend(kept);
    lines.splice(open + 1..close, stamped);
    let mut out = lines.join("\n");
    out.push('\n');
    write_atomic(path, &out)
}

// --- git -------------------------------------------------------------------

/// One git command in `repo`. The exit code is read directly, and a
/// failure carries git's own stderr, named.
fn git(repo: &Path, args: &[&str]) -> Result<String, String> {
    git_paths(repo, args, &[], &[])
}

/// One git command with a pathspec in the middle.
fn git_paths(repo: &Path, before: &[&str], paths: &[&str], after: &[&str]) -> Result<String, String> {
    let mut args: Vec<&str> = before.to_vec();
    args.extend(paths.iter().copied());
    args.extend_from_slice(after);
    let output = std::process::Command::new("git")
        .args(&args)
        .current_dir(repo)
        .output()
        .map_err(|error| format!("could not run git: {error}"))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(format!(
            "git {}: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

/// Whether `path` is in the repo's committed state.
fn tracked_in_head(repo: &Path, path: &str) -> bool {
    let reference = format!("HEAD:{path}");
    git(repo, &["cat-file", "-e", &reference]).is_ok()
}

/// Revert one repo path to committed state: a tracked path checks out
/// from HEAD, and a path the apply created (not in HEAD) is deleted.
fn revert_path(repo: &Path, path: &str) {
    if tracked_in_head(repo, path) {
        let _ = git_paths(repo, &["checkout", "HEAD", "--"], &[path], &[]);
    } else {
        let _ = fs::remove_file(repo.join(path));
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

/// The card width, matching `lanes`, `providers`, `skills`, and `calib`.
const CARD_WIDTH: usize = 72;

/// Render the proposals as cards, in two named sections: the general
/// store, then slice 7's drafts. Every card carries the target, the
/// basis, the status, and the last gate result — the four facts a
/// reviewer needs before running one. Never a plain-text table.
pub fn render(proposals: &[Proposal]) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "{BOLD}fleet improve{RESET} {DIM}\u{2014} {} proposal{}{RESET}\n\n",
        proposals.len(),
        if proposals.len() == 1 { "" } else { "s" },
    ));
    let general: Vec<&Proposal> =
        proposals.iter().filter(|proposal| proposal.store == Store::General).collect();
    let drafts: Vec<&Proposal> =
        proposals.iter().filter(|proposal| proposal.store == Store::SkillsDraft).collect();
    if general.is_empty() && drafts.is_empty() {
        out.push_str(&format!(
            "{DIM}no proposals yet \u{b7} the `improve` tool files one, and slice 7's drafter files skill drafts after a session{RESET}\n"
        ));
        return out;
    }
    if !general.is_empty() {
        out.push_str(&format!(
            "{DIM}proposals \u{b7} {PROPOSALS_DIR}/{RESET}\n\n"
        ));
        for proposal in &general {
            out.push_str(&render_card(proposal));
        }
    }
    if !drafts.is_empty() {
        out.push_str(&format!(
            "{DIM}skill drafts \u{b7} slice 7's {}/{PROPOSALS_DIR}/{RESET}\n\n",
            skills::SKILLS_DIR
        ));
        for proposal in &drafts {
            out.push_str(&render_card(proposal));
        }
    }
    out.push_str(&format!(
        "\n{DIM}applying is `fleet improve apply <name>` \u{b7} a destructive change waits for --approve \u{b7} reverting is `fleet improve undo <name>`{RESET}\n"
    ));
    out
}

/// One proposal's card: the name and its status in the header, the
/// target, the basis, and the last gate result beneath.
fn render_card(proposal: &Proposal) -> String {
    let status_color = match proposal.status {
        Status::Applied => palette::OK,
        Status::Refused => palette::WARN,
        Status::Waiting => palette::WARN,
        Status::Pending => TEXT,
    };
    // The status is the one colored fact on the card.
    let header_right =
        format!("{BOLD}{status_color}{}{RESET}", proposal.status.label());
    let header = format!(
        "{DIM}\u{250c}\u{2500}{RESET} {BOLD}{ACCENT}{name}{RESET} {DIM}{dashes} {right}{RESET} {DIM}\u{2510}{RESET}",
        name = proposal.name,
        dashes = dashes_for(&format!(" {} ", proposal.name), proposal.status.label()),
        right = header_right,
    );
    let mut lines: Vec<String> = Vec::new();
    lines.push(format!(
        "{DIM}\u{2502}{RESET} {TEXT}{} `{}`{RESET}",
        proposal.target.label(),
        proposal.target_name
    ));
    lines.push(format!(
        "{DIM}\u{2502}{RESET} {DIM}basis {} \u{b7} filed {}{RESET}",
        proposal.basis.label(),
        if proposal.created.is_empty() { "unknown" } else { &proposal.created }
    ));
    let gate = if proposal.gate.is_empty() {
        "gate: not yet run".to_string()
    } else {
        format!("gate: {}", proposal.gate)
    };
    lines.push(format!("{DIM}\u{2502}{RESET} {DIM}{gate}{RESET}"));
    lines.push(format!("{DIM}\u{2502}{RESET} {DIM}{}{RESET}", proposal.path.display()));
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

/// The dash run that pads a card header so its right edge aligns — the
/// skills card family's own.
fn dashes_for(left: &str, right: &str) -> String {
    let used = left.chars().count() + right.chars().count() + 2;
    "\u{2500}".repeat(CARD_WIDTH.saturating_sub(used + 4).max(1))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A scratch home, unique per call.
    fn scratch(tag: &str) -> PathBuf {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let serial = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "fleet-improve-{tag}-{}-{serial}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// A scratch repo-shaped tree: a git repo with one committed file,
    /// so a source patch has a pre-image to revert to.
    fn scratch_repo(tag: &str) -> PathBuf {
        let dir = scratch(tag);
        git(&dir, &["init", "-q"]).unwrap();
        git(&dir, &["config", "user.name", "fleet-test"]).unwrap();
        git(&dir, &["config", "user.email", "fleet-test@localhost"]).unwrap();
        fs::create_dir_all(dir.join("src")).unwrap();
        fs::write(dir.join("src/lib.rs"), "pub fn one() -> u32 { 1 }\n").unwrap();
        git_paths(&dir, &["add", "--"], &["src/lib.rs"], &[]).unwrap();
        git(&dir, &["commit", "-q", "-m", "pre-image"]).unwrap();
        dir
    }

    /// A stand-in `cargo` on a scratch PATH: it appends the args it was
    /// called with to `log` (baked into the script — no process-wide
    /// environment, which parallel tests would race on), prints
    /// `stderr` when it has any, and exits as `exit_code` says. The log
    /// is how a test proves the gate ran, and in what order, without
    /// the real toolchain — and without a `cargo test` inside a `cargo
    /// test`. Returns the PATH directory holding the stand-in.
    fn stand_in(dir: &Path, exit_code: i32, stderr: &str, log: &Path) -> String {
        let bin = dir.join("bin");
        fs::create_dir_all(&bin).unwrap();
        let script = bin.join("cargo");
        let mut text = format!("#!/bin/sh\necho \"$@\" >> '{}'\n", log.display());
        if !stderr.is_empty() {
            text.push_str(&format!("echo '{stderr}' >&2\n"));
        }
        text.push_str(&format!("exit {exit_code}\n"));
        fs::write(&script, text).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
        }
        bin.to_string_lossy().into_owned()
    }

    /// A gate over the stand-in, with the four real steps.
    fn fake_gate(root: &Path, bin: &str) -> Gate {
        let mut gate = Gate::local(root);
        gate.path = Some(bin.to_string());
        gate
    }

    /// Wait for the stream's writer to finish what is queued: a row
    /// lands asynchronously, and a test that reads the file needs the
    /// drain first (the calib tests' own `settle`).
    fn settle(stream: &calib::Stream) {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        while stream.pending() > 0 && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    }

    fn skill_proposal(home: &Path, name: &str, target_name: &str, body: &str) -> PathBuf {
        let text = render_proposal(
            name,
            Target::Skill,
            target_name,
            Basis::Inferred,
            "the session corrected a bash failure",
            "the skill loads and its body carries the correction",
            "the correction is in context next time",
            body,
            &[],
        );
        let path = proposals_dir(home).join(format!("{name}.md"));
        fs::create_dir_all(proposals_dir(home)).unwrap();
        fs::write(&path, text).unwrap();
        path
    }

    // --- the proposal shape ---

    #[test]
    fn a_proposal_round_trips_through_its_file() {
        let home = scratch("round-trip");
        let change = r#"{"op": "write-skill", "content": "---\nname: demo\ndescription: a demo skill\n---\n\nbody\n"}"#;
        let path = skill_proposal(&home, "demo-update", "demo", change);
        let proposal = load_one(&home, "demo-update").expect("the proposal loads");
        assert_eq!(proposal.name, "demo-update");
        assert_eq!(proposal.target, Target::Skill);
        assert_eq!(proposal.target_name, "demo");
        assert_eq!(proposal.basis, Basis::Inferred);
        assert_eq!(proposal.status, Status::Pending);
        assert_eq!(proposal.justification, "the session corrected a bash failure");
        assert_eq!(proposal.acceptance, "the skill loads and its body carries the correction");
        assert_eq!(proposal.effect, "the correction is in context next time");
        assert_eq!(proposal.store, Store::General);
        assert_eq!(
            proposal.change,
            Change::WriteSkill {
                content: "---\nname: demo\ndescription: a demo skill\n---\n\nbody\n".to_string()
            }
        );
        assert_eq!(proposal.path, path);
    }

    #[test]
    fn a_proposal_without_a_change_block_is_refused_by_name() {
        let home = scratch("no-change");
        let text = "---\nname: broken\ntarget: skill\ntarget-name: demo\nbasis: inferred\ncreated: 2026-10-14\nstatus: pending\n---\n\n## Justification\n\nsomething\n";
        fs::create_dir_all(proposals_dir(&home)).unwrap();
        fs::write(proposals_dir(&home).join("broken.md"), text).unwrap();
        assert!(load_one(&home, "broken").is_none());
    }

    #[test]
    fn the_change_ops_parse_and_unknown_ones_are_named() {
        let value: serde_json::Value =
            serde_json::from_str(r#"{"op": "write-skill", "content": "x"}"#).unwrap();
        assert_eq!(parse_change(&value).unwrap(), Change::WriteSkill { content: "x".to_string() });
        let value: serde_json::Value = serde_json::from_str(
            r#"{"op": "supersede", "name": "a", "justification": "because", "replacement": "b", "description": "the new fact"}"#,
        )
        .unwrap();
        let change = parse_change(&value).unwrap();
        assert_eq!(change.op(), "supersede");
        assert!(!change.destructive());
        let value: serde_json::Value = serde_json::from_str(r#"{"op": "retire-skill", "name": "a"}"#).unwrap();
        assert!(parse_change(&value).unwrap().destructive());
        let value: serde_json::Value = serde_json::from_str(r#"{"op": "forget", "name": "a", "justification": "because"}"#).unwrap();
        assert!(parse_change(&value).unwrap().destructive());
        let value: serde_json::Value = serde_json::from_str(r#"{"op": "config-remove", "id": "zai"}"#).unwrap();
        assert!(parse_change(&value).unwrap().destructive());
        let value: serde_json::Value =
            serde_json::from_str(r#"{"op": "patch", "path": "src/a.rs", "content": "x"}"#).unwrap();
        assert!(!parse_change(&value).unwrap().destructive());
        let value: serde_json::Value = serde_json::from_str(r#"{"op": "nonsense"}"#).unwrap();
        match parse_change(&value).unwrap() {
            Change::Unsupported { reason } => assert!(reason.contains("nonsense")),
            other => panic!("expected unsupported, got {other:?}"),
        }
    }

    #[test]
    fn a_destructive_change_names_exactly_what_it_destroys() {
        let change = Change::RetireSkill { name: "bash-recovery".to_string() };
        let destroys = change.destroys();
        assert!(destroys.contains("bash-recovery"), "{destroys}");
        let change = Change::ConfigRemove { id: "zai-coding-plan".to_string() };
        assert!(change.destroys().contains("zai-coding-plan"));
    }

    // --- the gate ---

    #[test]
    fn the_local_gate_is_the_four_steps_in_order() {
        let gate = Gate::local("/tmp/repo");
        let labels: Vec<&str> = gate.steps.iter().map(|step| step.label.as_str()).collect();
        assert_eq!(labels, vec!["fmt", "clippy", "build", "test"]);
        assert_eq!(gate.steps[0].args, args(&["fmt", "--all", "--check"]));
        assert_eq!(gate.steps[1].args, args(&["clippy", "--all-targets", "--", "-D", "warnings"]));
        assert_eq!(gate.steps[2].args, args(&["build", "--release"]));
        assert_eq!(gate.steps[3].args, args(&["test"]));
    }

    #[test]
    fn a_green_gate_runs_every_step_and_passes() {
        let scratch = scratch("gate-green");
        let log = scratch.join("gate.log");
        let bin = stand_in(&scratch, 0, "", &log);
        let gate = fake_gate(&scratch, &bin);
        let steps = gate.run();
        assert!(steps.iter().all(|step| step.ok), "{steps:?}");
        assert_eq!(steps.len(), 4);
        let ran = fs::read_to_string(&log).unwrap();
        assert!(ran.contains("fmt --all --check"), "{ran}");
        assert!(ran.contains("clippy --all-targets -- -D warnings"), "{ran}");
        assert!(ran.contains("build --release"), "{ran}");
        assert!(ran.contains("test"), "{ran}");
    }

    #[test]
    fn a_failing_gate_names_the_failure() {
        let scratch = scratch("gate-red");
        let log = scratch.join("gate.log");
        let bin = stand_in(&scratch, 1, "error: test failed: the_ranker_orders_a_corpus", &log);
        let gate = fake_gate(&scratch, &bin);
        let steps = gate.run();
        assert!(steps.iter().all(|step| !step.ok), "{steps:?}");
        assert_eq!(steps[3].label, "test");
        assert!(
            steps[3].detail.contains("the_ranker_orders_a_corpus"),
            "{:?}",
            steps[3].detail
        );
        let reason = named_failure(&steps);
        assert!(reason.contains("test"), "{reason}");
    }

    #[test]
    fn a_gate_step_whose_tool_is_absent_is_a_named_refusal_not_a_pass() {
        // The runner resolves its program through the configured PATH;
        // a PATH without the tool refuses the step by name. This is
        // the KI-007 shape: rustfmt absent means the fmt step fails,
        // never silently passes.
        let scratch = scratch("gate-absent");
        let empty = scratch.join("empty-path");
        fs::create_dir_all(&empty).unwrap();
        let mut gate = Gate::local(&scratch);
        gate.path = Some(empty.to_string_lossy().into_owned());
        let steps = gate.run();
        assert_eq!(steps.len(), 4);
        for step in &steps {
            assert!(!step.ok, "{step:?} should have failed");
            assert!(step.detail.contains("could not run"), "{step:?}");
        }
        assert!(named_failure(&steps).contains("could not run"));
    }

    // --- the apply ---

    #[test]
    fn a_skill_proposal_applies_through_a_green_gate_and_lands_the_row() {
        let home = scratch("apply-skill");
        let repo = scratch_repo("apply-skill-repo");
        let tools = scratch("apply-tools");
        let log = tools.join("gate.log");
        let bin = stand_in(&tools, 0, "", &log);
        let gate = fake_gate(&repo, &bin);
        // A live skill to update, and the proposal's new body.
        let skill_dir = skills::skills_dir(&home).join("demo");
        fs::create_dir_all(&skill_dir).unwrap();
        let live = "---\nname: demo\ndescription: the old body\n---\n\nold body\n";
        fs::write(skill_dir.join(skills::SKILL_FILE), live).unwrap();
        let change = r#"{"op": "write-skill", "content": "---\nname: demo\ndescription: the old body\n---\n\nnew body\n"}"#;
        skill_proposal(&home, "demo-update", "demo", change);
        let stream = calib::Stream::open(scratch("apply-skill-calib"));
        let ctx = Apply { home: &home, repo: &repo, gate: &gate, approved: false, stream: Some(&stream), eval: None };
        let verdict = apply("demo-update", &ctx);
        let Verdict::Applied { .. } = &verdict else { panic!("expected applied, got {verdict:?}") };
        // The skill changed on disk.
        let written = fs::read_to_string(skill_dir.join(skills::SKILL_FILE)).unwrap();
        assert!(written.contains("new body"), "{written}");
        // The gate ran.
        let ran = fs::read_to_string(&log).unwrap();
        assert!(ran.contains("build --release"), "{ran}");
        // The row landed, with the verdict and the gate's per-step evidence.
        settle(&stream);
        let root = stream.root().to_path_buf();
        let rows = calib::read_day(&root, &calib::today(), 64);
        let row = rows.iter().find(|row| row.op == calib::OP_PROPOSE).expect("the propose row");
        assert_eq!(row.proposal, "demo-update");
        assert_eq!(row.target, "skill");
        assert_eq!(row.target_name, "demo");
        assert_eq!(row.basis, "inferred");
        assert_eq!(row.verdict, calib::VERDICT_APPLIED);
        assert_eq!(row.gate.len(), 4);
        assert!(row.gate.iter().all(|step| step.ok));
        // The proposal's status is stamped.
        let stamped = load_one(&home, "demo-update").unwrap();
        assert_eq!(stamped.status, Status::Applied);
        assert!(stamped.gate.contains("fmt ok"), "{}", stamped.gate);
        // Exactly one row for the one verdict.
        assert_eq!(rows.iter().filter(|row| row.op == calib::OP_PROPOSE).count(), 1);
    }

    #[test]
    fn a_patch_under_memories_is_refused_because_it_would_bypass_the_provenance_rule() {
        // The Critical the quality review found: a patch op whose path
        // is under `memories/` passed every guard, was classified
        // non-destructive, never consulted the fact's basis, and landed
        // on a green gate — a model-filed proposal could rewrite a
        // confirmed-by fact with no human in the loop. The patch op now
        // refuses the path; `supersede` is the only door.
        let home = scratch("apply-memories-bypass");
        let repo = scratch_repo("apply-memories-bypass-repo");
        let tools = scratch("apply-tools");
        let log = tools.join("gate.log");
        let bin = stand_in(&tools, 0, "", &log);
        let gate = fake_gate(&repo, &bin);
        let change = r#"{"op": "patch", "path": "memories/load-bearing.md", "content": "rewritten\n"}"#;
        skill_proposal(&home, "bypass-the-fact", "memories/load-bearing.md", change);
        let stream = calib::Stream::open(scratch("apply-memories-bypass-calib"));
        let ctx = Apply { home: &home, repo: &repo, gate: &gate, approved: false, stream: Some(&stream), eval: None };
        let verdict = apply("bypass-the-fact", &ctx);
        let Verdict::Refused { reason } = &verdict else { panic!("expected refused, got {verdict:?}") };
        assert!(reason.contains("memories/"), "{reason}");
        assert!(reason.contains("supersede"), "{reason}");
        assert!(reason.contains("provenance"), "{reason}");
        // Nothing was written, and the gate never ran.
        assert!(!repo.join("memories/load-bearing.md").exists());
        assert!(!log.exists(), "the gate ran: {}", fs::read_to_string(&log).unwrap_or_default());
        settle(&stream);
        let root = stream.root().to_path_buf();
        let rows = calib::read_day(&root, &calib::today(), 64);
        let row = rows.iter().find(|row| row.op == calib::OP_PROPOSE).expect("the propose row");
        assert_eq!(row.verdict, calib::VERDICT_REFUSED);
        assert!(row.gate.is_empty(), "the gate never ran: {:?}", row.gate);
    }

    #[test]
    fn a_patch_into_a_dirty_target_is_refused_rather_than_destroying_the_work() {
        // The apply overwrites the target before the gate runs, and a
        // red gate's revert restores HEAD — so pre-existing uncommitted
        // work in the target would be silently destroyed. The apply
        // refuses instead, and the work survives.
        let home = scratch("apply-dirty-target");
        let repo = scratch_repo("apply-dirty-target-repo");
        fs::write(repo.join("src/lib.rs"), "pub fn one() -> u32 { 99 }\n").unwrap();
        let tools = scratch("apply-tools");
        let log = tools.join("gate.log");
        let bin = stand_in(&tools, 1, "error: test failed", &log);
        let gate = fake_gate(&repo, &bin);
        let change = r#"{"op": "patch", "path": "src/lib.rs", "content": "pub fn one() -> u32 { 2 }\n"}"#;
        skill_proposal(&home, "overwrite-the-work", "src/lib.rs", change);
        let stream = calib::Stream::open(scratch("apply-dirty-target-calib"));
        let ctx = Apply { home: &home, repo: &repo, gate: &gate, approved: false, stream: Some(&stream), eval: None };
        let verdict = apply("overwrite-the-work", &ctx);
        let Verdict::Refused { reason } = &verdict else { panic!("expected refused, got {verdict:?}") };
        assert!(reason.contains("uncommitted work"), "{reason}");
        // The operator's work is exactly as they left it.
        assert_eq!(fs::read_to_string(repo.join("src/lib.rs")).unwrap(), "pub fn one() -> u32 { 99 }\n");
        // The gate never ran.
        assert!(!log.exists(), "the gate ran: {}", fs::read_to_string(&log).unwrap_or_default());
    }

    #[test]
    fn a_patch_whose_target_drifted_since_the_draft_is_refused_by_name() {
        // PS-C's guard, beside the dirty-tree one. The draft was filed
        // against the file as it stood; the file then changed and was
        // committed — a clean tree, so the dirty guard has nothing to
        // say — and the change was reasoned about a different file. The
        // apply refuses by name, naming both shas, and the target is
        // left exactly as it now stands.
        let home = scratch("apply-drift");
        let repo = scratch_repo("apply-drift-repo");
        let change = r#"{"op": "patch", "path": "src/lib.rs", "content": "pub fn one() -> u32 { 2 }\n"}"#;
        let stamped = file_sha(&repo, "src/lib.rs").expect("the target exists at draft time");
        let text = render_proposal(
            "drifted-target",
            Target::Source,
            "src/lib.rs",
            Basis::Inferred,
            "the session's evidence",
            "the gate is green",
            "the function returns 2",
            change,
            &[TargetSha { path: "src/lib.rs".to_string(), sha256: stamped.clone() }],
        );
        fs::create_dir_all(proposals_dir(&home)).unwrap();
        fs::write(proposals_dir(&home).join("drifted-target.md"), text).unwrap();
        // The target drifts, and the drift is committed: the tree is
        // clean, so only the sha can see it.
        fs::write(repo.join("src/lib.rs"), "pub fn one() -> u32 { 42 }\n").unwrap();
        git(&repo, &["add", "-A"]).unwrap();
        git(&repo, &["-c", "user.email=a@b.c", "-c", "user.name=t", "commit", "-m", "drift"]).unwrap();
        let status = git(&repo, &["status", "--porcelain"]).unwrap();
        assert!(status.trim().is_empty(), "the tree is not clean: {status}");
        let tools = scratch("apply-tools");
        let log = tools.join("gate.log");
        let bin = stand_in(&tools, 0, "", &log);
        let gate = fake_gate(&repo, &bin);
        let stream = calib::Stream::open(scratch("apply-drift-calib"));
        let ctx = Apply { home: &home, repo: &repo, gate: &gate, approved: false, stream: Some(&stream), eval: None };
        let verdict = apply("drifted-target", &ctx);
        let Verdict::Refused { reason } = &verdict else { panic!("expected refused, got {verdict:?}") };
        assert!(reason.contains("drifted since the draft"), "{reason}");
        assert!(reason.contains(&stamped), "{reason}");
        // The target is exactly as it now stands, and the gate never ran.
        assert_eq!(fs::read_to_string(repo.join("src/lib.rs")).unwrap(), "pub fn one() -> u32 { 42 }\n");
        assert!(!log.exists(), "the gate ran: {}", fs::read_to_string(&log).unwrap_or_default());
    }

    #[test]
    fn a_proposal_whose_targets_still_match_applies_through_the_drift_guard() {
        // The guard is additive, not a new way to refuse: a proposal
        // whose stamped shas still match walks straight past it.
        let home = scratch("apply-no-drift");
        let repo = scratch_repo("apply-no-drift-repo");
        let change = r#"{"op": "patch", "path": "src/lib.rs", "content": "pub fn one() -> u32 { 2 }\n"}"#;
        let stamped = file_sha(&repo, "src/lib.rs").expect("the target exists at draft time");
        let text = render_proposal(
            "undrifted-target",
            Target::Source,
            "src/lib.rs",
            Basis::Inferred,
            "the session's evidence",
            "the gate is green",
            "the function returns 2",
            change,
            &[TargetSha { path: "src/lib.rs".to_string(), sha256: stamped.clone() }],
        );
        fs::create_dir_all(proposals_dir(&home)).unwrap();
        fs::write(proposals_dir(&home).join("undrifted-target.md"), text).unwrap();
        let tools = scratch("apply-tools");
        let log = tools.join("gate.log");
        let bin = stand_in(&tools, 0, "", &log);
        let gate = fake_gate(&repo, &bin);
        let stream = calib::Stream::open(scratch("apply-no-drift-calib"));
        let ctx = Apply { home: &home, repo: &repo, gate: &gate, approved: false, stream: Some(&stream), eval: None };
        let verdict = apply("undrifted-target", &ctx);
        assert!(matches!(verdict, Verdict::Applied { .. }), "{verdict:?}");
        // The gate ran, and the row carries the judged artifact's sha.
        assert!(log.exists(), "the gate never ran");
        settle(&stream);
        let root = stream.root().to_path_buf();
        let rows = calib::read_day(&root, &calib::today(), 64);
        let row = rows.iter().find(|row| row.op == calib::OP_PROPOSE).expect("the propose row");
        assert_eq!(row.verdict, calib::VERDICT_APPLIED);
        let expected = parse_change(&serde_json::from_str::<serde_json::Value>(change).unwrap())
            .expect("the change parses")
            .artifact_sha();
        assert_eq!(row.input_sha256, expected);
    }

    #[test]
    fn a_source_proposal_that_fails_the_gate_is_refused_with_the_failure_named_and_reverted() {
        let home = scratch("apply-source-red");
        let repo = scratch_repo("apply-source-red-repo");
        let pre_image = fs::read_to_string(repo.join("src/lib.rs")).unwrap();
        let tools = scratch("apply-tools");
        let log = tools.join("gate.log");
        let bin = stand_in(&tools, 1, "error: test failed: the_ranker_orders_a_corpus", &log);
        let gate = fake_gate(&repo, &bin);
        let change = r#"{"op": "patch", "path": "src/lib.rs", "content": "pub fn one() -> u32 { 2 }\n"}"#;
        skill_proposal(&home, "break-the-ranker", "src/lib.rs", change);
        let stream = calib::Stream::open(scratch("apply-source-red-calib"));
        let ctx = Apply { home: &home, repo: &repo, gate: &gate, approved: false, stream: Some(&stream), eval: None };
        let verdict = apply("break-the-ranker", &ctx);
        let Verdict::Refused { reason } = &verdict else { panic!("expected refused, got {verdict:?}") };
        assert!(reason.contains("the_ranker_orders_a_corpus"), "{reason}");
        // The tree is back to its pre-image.
        assert_eq!(fs::read_to_string(repo.join("src/lib.rs")).unwrap(), pre_image);
        let status = git(&repo, &["status", "--porcelain"]).unwrap();
        assert!(status.trim().is_empty(), "{status}");
        // The row landed, refused, naming the test.
        settle(&stream);
        let root = stream.root().to_path_buf();
        let rows = calib::read_day(&root, &calib::today(), 64);
        let row = rows.iter().find(|row| row.op == calib::OP_PROPOSE).expect("the propose row");
        assert_eq!(row.verdict, calib::VERDICT_REFUSED);
        assert!(row.gate.iter().any(|step| !step.ok && step.detail.contains("the_ranker")));
        let stamped = load_one(&home, "break-the-ranker").unwrap();
        assert_eq!(stamped.status, Status::Refused);
        assert!(stamped.commit.is_none());
    }

    #[test]
    fn a_source_proposal_that_passes_applies_and_commits() {
        let home = scratch("apply-source-green");
        let repo = scratch_repo("apply-source-green-repo");
        let tools = scratch("apply-tools");
        let log = tools.join("gate.log");
        let bin = stand_in(&tools, 0, "", &log);
        let gate = fake_gate(&repo, &bin);
        let change = r#"{"op": "patch", "path": "src/lib.rs", "content": "pub fn one() -> u32 { 2 }\n"}"#;
        skill_proposal(&home, "fix-the-ranker", "src/lib.rs", change);
        let stream = calib::Stream::open(scratch("apply-source-green-calib"));
        let ctx = Apply { home: &home, repo: &repo, gate: &gate, approved: false, stream: Some(&stream), eval: None };
        let verdict = apply("fix-the-ranker", &ctx);
        let Verdict::Applied { commit, .. } = &verdict else { panic!("expected applied, got {verdict:?}") };
        let commit = commit.clone().expect("a repo-side apply commits");
        // The change is on disk and committed.
        assert!(fs::read_to_string(repo.join("src/lib.rs")).unwrap().contains("{ 2 }"));
        let status = git(&repo, &["status", "--porcelain"]).unwrap();
        assert!(status.trim().is_empty(), "{status}");
        let subject = git(&repo, &["log", "-1", "--format=%s"]).unwrap();
        assert!(subject.contains("fix-the-ranker"), "{subject}");
        // The stamped proposal carries the commit for the undo.
        let stamped = load_one(&home, "fix-the-ranker").unwrap();
        assert_eq!(stamped.commit.as_deref(), Some(commit.as_str()));
        // The undo reverts the commit structurally.
        let reverted = undo("fix-the-ranker", &home, &repo).expect("the undo");
        assert!(reverted.contains("reverted commit"), "{reverted}");
        assert!(fs::read_to_string(repo.join("src/lib.rs")).unwrap().contains("{ 1 }"));
        let status = git(&repo, &["status", "--porcelain"]).unwrap();
        assert!(status.trim().is_empty(), "{status}");
        assert_eq!(load_one(&home, "fix-the-ranker").unwrap().status, Status::Pending);
    }

    #[test]
    fn a_source_proposal_that_regresses_the_corpus_is_refused_with_the_scenario_named() {
        // The slice-13 hook that closes the loop with slice 12: a source
        // self-edit whose corpus run regresses is refused at the
        // `eval_corpus` gate step, naming the scenario. Driven through the
        // stand-in seam — a green stand-in cargo gate plus an eval probe
        // that reports a regression — so the refusal is deterministic.
        let home = scratch("apply-eval-regress");
        let repo = scratch_repo("apply-eval-regress-repo");
        let pre_image = fs::read_to_string(repo.join("src/lib.rs")).unwrap();
        let tools = scratch("apply-tools");
        let log = tools.join("gate.log");
        let bin = stand_in(&tools, 0, "", &log);
        let gate = fake_gate(&repo, &bin);
        let probe: eval::EvalProbe = std::sync::Arc::new(|| {
            Err("the eval corpus regressed: conversation-fold".to_string())
        });
        let change = r#"{"op": "patch", "path": "src/lib.rs", "content": "pub fn one() -> u32 { 2 }\n"}"#;
        skill_proposal(&home, "break-the-fold", "src/lib.rs", change);
        let stream = calib::Stream::open(scratch("apply-eval-regress-calib"));
        let ctx = Apply {
            home: &home,
            repo: &repo,
            gate: &gate,
            approved: false,
            stream: Some(&stream),
            eval: Some(probe),
        };
        let verdict = apply("break-the-fold", &ctx);
        let Verdict::Refused { reason } = &verdict else { panic!("expected refused, got {verdict:?}") };
        assert!(reason.contains("conversation-fold"), "{reason}");
        assert!(reason.contains("eval_corpus"), "{reason}");
        // The tree is back to its pre-image and nothing committed.
        assert_eq!(fs::read_to_string(repo.join("src/lib.rs")).unwrap(), pre_image);
        let status = git(&repo, &["status", "--porcelain"]).unwrap();
        assert!(status.trim().is_empty(), "{status}");
        // The row landed, refused, carrying the eval step's evidence.
        settle(&stream);
        let root = stream.root().to_path_buf();
        let rows = calib::read_day(&root, &calib::today(), 64);
        let row = rows.iter().find(|row| row.op == calib::OP_PROPOSE).expect("the propose row");
        assert_eq!(row.verdict, calib::VERDICT_REFUSED);
        let eval_step =
            row.gate.iter().find(|step| step.step == "eval_corpus").expect("the eval step");
        assert!(!eval_step.ok);
        assert!(eval_step.detail.contains("conversation-fold"), "{:?}", eval_step.detail);
    }

    #[test]
    fn a_red_cargo_gate_records_the_eval_step_as_not_run_rather_than_leaving_it_absent() {
        // The named finding: when the cargo gate refused, the row carried
        // no eval step at all, so an operator reading the row could not
        // tell whether the corpus would have passed. The step is now
        // recorded not-run — `ok: false`, the reason named — so the row
        // says the corpus was not reached, not that it passed.
        let home = scratch("apply-eval-not-run");
        let repo = scratch_repo("apply-eval-not-run-repo");
        let pre_image = fs::read_to_string(repo.join("src/lib.rs")).unwrap();
        let tools = scratch("apply-tools");
        let log = tools.join("gate.log");
        let bin = stand_in(&tools, 1, "error: test failed: the_ranker_orders_a_corpus", &log);
        let gate = fake_gate(&repo, &bin);
        // A wired probe that records whether it ran: under a red cargo
        // gate it must not run at all.
        let ran = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let seen = std::sync::Arc::clone(&ran);
        let probe: eval::EvalProbe = std::sync::Arc::new(move || {
            seen.store(true, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        });
        let change = r#"{"op": "patch", "path": "src/lib.rs", "content": "pub fn one() -> u32 { 2 }\n"}"#;
        skill_proposal(&home, "red-gate", "src/lib.rs", change);
        let stream = calib::Stream::open(scratch("apply-eval-not-run-calib"));
        let ctx = Apply {
            home: &home,
            repo: &repo,
            gate: &gate,
            approved: false,
            stream: Some(&stream),
            eval: Some(probe),
        };
        let verdict = apply("red-gate", &ctx);
        let Verdict::Refused { reason } = &verdict else { panic!("expected refused, got {verdict:?}") };
        assert!(reason.contains("the gate refused the change"), "{reason}");
        assert_eq!(fs::read_to_string(repo.join("src/lib.rs")).unwrap(), pre_image);
        assert!(
            !ran.load(std::sync::atomic::Ordering::SeqCst),
            "the corpus probe ran under a red cargo gate"
        );
        settle(&stream);
        let root = stream.root().to_path_buf();
        let rows = calib::read_day(&root, &calib::today(), 64);
        let row = rows.iter().find(|row| row.op == calib::OP_PROPOSE).expect("the propose row");
        // Five steps: the four cargo steps plus the eval step recorded
        // not-run, with the reason named.
        assert_eq!(row.gate.len(), 5, "{:?}", row.gate);
        let eval_step =
            row.gate.iter().find(|step| step.step == "eval_corpus").expect("the eval step");
        assert!(!eval_step.ok, "{eval_step:?}");
        assert!(eval_step.detail.contains("not run"), "{eval_step:?}");
        assert!(eval_step.detail.contains("the cargo gate refused first"), "{eval_step:?}");
    }

    #[test]
    fn a_source_proposal_applies_when_the_corpus_is_green() {
        // The green path: a green cargo gate plus a green corpus probe lets
        // the source edit apply and commit, with the eval step recorded ok.
        let home = scratch("apply-eval-green");
        let repo = scratch_repo("apply-eval-green-repo");
        let tools = scratch("apply-tools");
        let log = tools.join("gate.log");
        let bin = stand_in(&tools, 0, "", &log);
        let gate = fake_gate(&repo, &bin);
        let probe: eval::EvalProbe = std::sync::Arc::new(|| Ok(()));
        let change = r#"{"op": "patch", "path": "src/lib.rs", "content": "pub fn one() -> u32 { 2 }\n"}"#;
        skill_proposal(&home, "keep-the-fold", "src/lib.rs", change);
        let stream = calib::Stream::open(scratch("apply-eval-green-calib"));
        let ctx = Apply {
            home: &home,
            repo: &repo,
            gate: &gate,
            approved: false,
            stream: Some(&stream),
            eval: Some(probe),
        };
        let verdict = apply("keep-the-fold", &ctx);
        assert!(matches!(verdict, Verdict::Applied { .. }), "{verdict:?}");
        assert!(fs::read_to_string(repo.join("src/lib.rs")).unwrap().contains("{ 2 }"));
        settle(&stream);
        let root = stream.root().to_path_buf();
        let rows = calib::read_day(&root, &calib::today(), 64);
        let row = rows.iter().find(|row| row.op == calib::OP_PROPOSE).expect("the propose row");
        assert_eq!(row.verdict, calib::VERDICT_APPLIED);
        assert!(row.gate.iter().any(|step| step.step == "eval_corpus" && step.ok));
    }

    #[test]
    fn a_destructive_proposal_waits_and_names_what_will_be_destroyed() {
        let home = scratch("apply-destructive");
        let repo = scratch_repo("apply-destructive-repo");
        let tools = scratch("apply-tools");
        let log = tools.join("gate.log");
        let bin = stand_in(&tools, 0, "", &log);
        let gate = fake_gate(&repo, &bin);
        // A live skill, and a proposal that retires it.
        let skill_dir = skills::skills_dir(&home).join("demo");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(
            skill_dir.join(skills::SKILL_FILE),
            "---\nname: demo\ndescription: a demo\n---\n\nbody\n",
        )
        .unwrap();
        let change = r#"{"op": "retire-skill", "name": "demo"}"#;
        skill_proposal(&home, "retire-demo", "demo", change);
        let stream = calib::Stream::open(scratch("apply-destructive-calib"));
        let ctx = Apply { home: &home, repo: &repo, gate: &gate, approved: false, stream: Some(&stream), eval: None };
        let verdict = apply("retire-demo", &ctx);
        let Verdict::Waiting { reason } = &verdict else { panic!("expected waiting, got {verdict:?}") };
        assert!(reason.contains("demo"), "{reason}");
        assert!(reason.contains("--approve"), "{reason}");
        // Nothing applied: the skill is still live.
        assert!(skill_dir.join(skills::SKILL_FILE).exists());
        // The gate never ran.
        assert!(!log.exists());
        // The row landed, waiting, with no gate evidence.
        settle(&stream);
        let root = stream.root().to_path_buf();
        let rows = calib::read_day(&root, &calib::today(), 64);
        let row = rows.iter().find(|row| row.op == calib::OP_PROPOSE).expect("the propose row");
        assert_eq!(row.verdict, calib::VERDICT_WAITING);
        assert!(row.gate.is_empty());
        // The status is waiting, so a second apply does not double-record.
        let second = apply("retire-demo", &ctx);
        let Verdict::Refused { reason } = &second else { panic!("expected refused, got {second:?}") };
        assert!(reason.contains("already waiting"), "{reason}");
        // With approval in the current turn, the retirement lands.
        let ctx = Apply { approved: true, ..ctx };
        let verdict = apply("retire-demo", &ctx);
        assert!(matches!(verdict, Verdict::Applied { .. }), "{verdict:?}");
        assert!(!skill_dir.exists(), "the skill left the live tree");
    }

    #[test]
    fn a_skill_apply_reverts_to_its_pre_image_byte_for_byte() {
        let home = scratch("undo-skill");
        let repo = scratch_repo("undo-skill-repo");
        let tools = scratch("apply-tools");
        let log = tools.join("gate.log");
        let bin = stand_in(&tools, 0, "", &log);
        let gate = fake_gate(&repo, &bin);
        let skill_dir = skills::skills_dir(&home).join("demo");
        fs::create_dir_all(&skill_dir).unwrap();
        let live = "---\nname: demo\ndescription: the old body\n---\n\nold body\n";
        let live_path = skill_dir.join(skills::SKILL_FILE);
        fs::write(&live_path, live).unwrap();
        let change = r#"{"op": "write-skill", "content": "---\nname: demo\ndescription: the old body\n---\n\nnew body\n"}"#;
        skill_proposal(&home, "demo-update", "demo", change);
        let stream = calib::Stream::open(scratch("undo-skill-calib"));
        let ctx = Apply { home: &home, repo: &repo, gate: &gate, approved: false, stream: Some(&stream), eval: None };
        assert!(matches!(apply("demo-update", &ctx), Verdict::Applied { .. }));
        assert!(fs::read_to_string(&live_path).unwrap().contains("new body"));
        let reverted = undo("demo-update", &home, &repo).expect("the undo");
        assert!(reverted.contains("restored the pre-image"), "{reverted}");
        assert_eq!(fs::read_to_string(&live_path).unwrap(), live);
    }

    #[test]
    fn a_created_skill_undoes_by_deleting_what_was_added() {
        let home = scratch("undo-create");
        let repo = scratch_repo("undo-create-repo");
        let tools = scratch("apply-tools");
        let log = tools.join("gate.log");
        let bin = stand_in(&tools, 0, "", &log);
        let gate = fake_gate(&repo, &bin);
        let change = r#"{"op": "write-skill", "content": "---\nname: fresh\ndescription: a new skill\n---\n\nbody\n"}"#;
        skill_proposal(&home, "create-fresh", "fresh", change);
        let stream = calib::Stream::open(scratch("undo-create-calib"));
        let ctx = Apply { home: &home, repo: &repo, gate: &gate, approved: false, stream: Some(&stream), eval: None };
        assert!(matches!(apply("create-fresh", &ctx), Verdict::Applied { .. }));
        let created = skills::skills_dir(&home).join("fresh").join(skills::SKILL_FILE);
        assert!(created.exists());
        undo("create-fresh", &home, &repo).expect("the undo");
        assert!(!created.exists(), "the created file is gone");
        assert!(!skills::skills_dir(&home).join("fresh").exists(), "the empty directory is gone too");
    }

    #[test]
    fn a_retired_skill_undoes_by_moving_its_directory_back() {
        let home = scratch("undo-retire");
        let repo = scratch_repo("undo-retire-repo");
        let tools = scratch("apply-tools");
        let log = tools.join("gate.log");
        let bin = stand_in(&tools, 0, "", &log);
        let gate = fake_gate(&repo, &bin);
        let skill_dir = skills::skills_dir(&home).join("demo");
        fs::create_dir_all(&skill_dir).unwrap();
        let live = "---\nname: demo\ndescription: a demo\n---\n\nbody\n";
        fs::write(skill_dir.join(skills::SKILL_FILE), live).unwrap();
        let change = r#"{"op": "retire-skill", "name": "demo"}"#;
        skill_proposal(&home, "retire-demo", "demo", change);
        let stream = calib::Stream::open(scratch("undo-retire-calib"));
        let ctx = Apply {
            home: &home,
            repo: &repo,
            gate: &gate,
            approved: true,
            stream: Some(&stream),
            eval: None,
        };
        assert!(matches!(apply("retire-demo", &ctx), Verdict::Applied { .. }));
        assert!(!skill_dir.exists());
        undo("retire-demo", &home, &repo).expect("the undo");
        assert_eq!(fs::read_to_string(skill_dir.join(skills::SKILL_FILE)).unwrap(), live);
    }

    // --- the provenance rule ---

    /// A scratch corpus with one fact of the given basis.
    fn scratch_corpus(tag: &str, name: &str, basis: &str) -> PathBuf {
        let repo = scratch_repo(tag);
        let memories = repo.join("memories");
        fs::create_dir_all(&memories).unwrap();
        fs::write(
            memories.join(format!("{name}.md")),
            format!(
                "---\nname: {name}\ndescription: a fact about {name}\ntype: project\nbasis: {basis}\n---\n\nthe body\n"
            ),
        )
        .unwrap();
        git_paths(&repo, &["add", "--"], &[&format!("memories/{name}.md")], &[]).unwrap();
        git(&repo, &["commit", "-q", "-m", "the corpus"]).unwrap();
        repo
    }

    fn memory_proposal(home: &Path, name: &str, fact: &str, change: &str) -> PathBuf {
        let path = proposals_dir(home).join(format!("{name}.md"));
        fs::create_dir_all(proposals_dir(home)).unwrap();
        fs::write(
            &path,
            render_proposal(
                name,
                Target::Memory,
                fact,
                Basis::Inferred,
                "recall ranked a stale fact",
                "recall ranks the replacement",
                "the corpus stops carrying the stale fact",
                change,
                &[],
            ),
        )
        .unwrap();
        path
    }

    #[test]
    fn an_inferred_fact_supersedes_on_the_recorded_justification() {
        let home = scratch("provenance-inferred");
        let repo = scratch_corpus("provenance-inferred-repo", "old-fact", "inferred");
        let tools = scratch("apply-tools");
        let log = tools.join("gate.log");
        let bin = stand_in(&tools, 0, "", &log);
        let gate = fake_gate(&repo, &bin);
        let change = r#"{"op": "supersede", "name": "old-fact", "justification": "the port moved to 8902", "replacement": "new-fact", "description": "the endpoint is 8902"}"#;
        memory_proposal(&home, "fix-the-fact", "old-fact", change);
        let stream = calib::Stream::open(scratch("provenance-inferred-calib"));
        let ctx = Apply { home: &home, repo: &repo, gate: &gate, approved: false, stream: Some(&stream), eval: None };
        let verdict = apply("fix-the-fact", &ctx);
        assert!(matches!(verdict, Verdict::Applied { .. }), "{verdict:?}");
        // The supersession landed through slice 8's own path.
        let superseded = fs::read_to_string(repo.join("memories/old-fact.md")).unwrap();
        assert!(superseded.contains("superseded-by: new-fact"), "{superseded}");
        assert!(repo.join("memories/new-fact.md").exists());
        // The justification rode the row.
        settle(&stream);
        let root = stream.root().to_path_buf();
        let rows = calib::read_day(&root, &calib::today(), 64);
        let row = rows.iter().find(|row| row.op == calib::OP_PROPOSE).expect("the propose row");
        assert_eq!(row.target, "memory");
        assert_eq!(row.verdict, calib::VERDICT_APPLIED);
        assert!(row.reason.contains("superseded"), "{}", row.reason);
    }

    #[test]
    fn a_confirmed_by_fact_waits_for_approval() {
        let home = scratch("provenance-confirmed");
        let repo = scratch_corpus("provenance-confirmed-repo", "load-bearing", "confirmed-by");
        let tools = scratch("apply-tools");
        let log = tools.join("gate.log");
        let bin = stand_in(&tools, 0, "", &log);
        let gate = fake_gate(&repo, &bin);
        let change = r#"{"op": "supersede", "name": "load-bearing", "justification": "it changed", "replacement": "load-bearing-2", "description": "the new rule"}"#;
        memory_proposal(&home, "touch-the-rule", "load-bearing", change);
        let stream = calib::Stream::open(scratch("provenance-confirmed-calib"));
        let ctx = Apply { home: &home, repo: &repo, gate: &gate, approved: false, stream: Some(&stream), eval: None };
        let verdict = apply("touch-the-rule", &ctx);
        let Verdict::Waiting { reason } = &verdict else { panic!("expected waiting, got {verdict:?}") };
        assert!(reason.contains("confirmed-by"), "{reason}");
        assert!(reason.contains("load-bearing"), "{reason}");
        // Nothing landed: the fact is untouched.
        let fact = fs::read_to_string(repo.join("memories/load-bearing.md")).unwrap();
        assert!(!fact.contains("superseded-by"), "{fact}");
        assert!(!repo.join("memories/load-bearing-2.md").exists());
    }

    // --- the tool's file path ---

    #[test]
    fn the_tool_files_a_proposal_and_refuses_an_existing_name() {
        let home = scratch("tool-file");
        let arguments = serde_json::json!({
            "name": "session-lesson",
            "target": "skill",
            "target-name": "bash-recovery",
            "basis": "inferred",
            "justification": "a failed bash call was corrected in this session",
            "acceptance": "the skill's body carries the correction",
            "effect": "the next session recovers without the failure",
            "change": r#"{"op": "write-skill", "content": "---\nname: bash-recovery\ndescription: recover from a failed call\n---\n\nthe correction\n"}"#,
        });
        let repo = scratch("tool-file-repo");
        let result = file_from_tool(&home, &repo, &arguments);
        assert!(result.contains("filed proposal `session-lesson`"), "{result}");
        assert!(result.contains("fleet improve apply session-lesson"), "{result}");
        let proposal = load_one(&home, "session-lesson").expect("the filed proposal loads");
        assert_eq!(proposal.target, Target::Skill);
        assert_eq!(proposal.target_name, "bash-recovery");
        assert_eq!(proposal.status, Status::Pending);
        // A second filing of the same name is refused by name.
        let again = file_from_tool(&home, &repo, &arguments);
        assert!(again.contains("already waiting"), "{again}");
    }

    #[test]
    fn the_tool_refuses_a_change_it_could_never_apply() {
        let home = scratch("tool-bad-change");
        let arguments = serde_json::json!({
            "name": "bad",
            "target": "source",
            "target-name": "src/x.rs",
            "basis": "inferred",
            "justification": "because",
            "acceptance": "criteria",
            "effect": "effect",
            "change": "not json at all",
        });
        let repo = scratch("tool-file-repo");
        let result = file_from_tool(&home, &repo, &arguments);
        assert!(result.contains("not valid JSON"), "{result}");
        assert!(!proposals_dir(&home).join("bad.md").exists());
    }

    // --- the surface ---

    #[test]
    fn the_listing_names_both_stores_with_target_basis_and_status() {
        let home = scratch("listing");
        skill_proposal(&home, "general-one", "demo", r#"{"op": "write-skill", "content": "x"}"#);
        // A slice-7 draft in its own store.
        let drafts = skills::proposals_dir(&home);
        fs::create_dir_all(&drafts).unwrap();
        fs::write(
            drafts.join("drafted.md"),
            "---\nname: drafted\ndescription: a drafted skill\nproposed: true\nkind: create\nevidence: session 42\n---\n\nthe drafted body\n",
        )
        .unwrap();
        let proposals = load_all(&home);
        assert_eq!(proposals.len(), 2);
        let rendered = render(&proposals);
        assert!(rendered.contains("general-one"), "{rendered}");
        assert!(rendered.contains("drafted"), "{rendered}");
        assert!(rendered.contains("skill `demo`"), "{rendered}");
        assert!(rendered.contains("basis inferred"), "{rendered}");
        assert!(rendered.contains("gate: not yet run"), "{rendered}");
        // The two stores are named honestly, not merged.
        assert!(rendered.contains(&format!("{PROPOSALS_DIR}/")), "{rendered}");
        assert!(rendered.contains(&format!("{}/{PROPOSALS_DIR}/", skills::SKILLS_DIR)), "{rendered}");
    }

    #[test]
    fn a_slice_seven_draft_adapts_into_the_gates_view() {
        let home = scratch("draft-view");
        let drafts = skills::proposals_dir(&home);
        fs::create_dir_all(&drafts).unwrap();
        fs::write(
            drafts.join("drafted.md"),
            "---\nname: drafted\ndescription: a drafted skill\nproposed: true\nkind: update\nevidence: session 42\n---\n\nthe drafted body\n",
        )
        .unwrap();
        let proposal = load_one(&home, "drafted").expect("the draft loads");
        assert_eq!(proposal.store, Store::SkillsDraft);
        assert_eq!(proposal.target, Target::Skill);
        assert_eq!(proposal.target_name, "drafted");
        assert_eq!(proposal.basis, Basis::Inferred);
        assert_eq!(proposal.justification, "session 42");
        match &proposal.change {
            Change::WriteSkill { content } => {
                assert!(content.starts_with("---\nname: drafted\n"), "{content}");
                assert!(!content.contains("proposed: true"), "{content}");
                assert!(content.contains("the drafted body"), "{content}");
            }
            other => panic!("expected a write-skill change, got {other:?}"),
        }
    }

    #[test]
    fn a_merge_draft_names_the_absorbed_half_and_a_broken_one_refuses() {
        let home = scratch("draft-merge");
        let drafts = skills::proposals_dir(&home);
        fs::create_dir_all(&drafts).unwrap();
        fs::write(
            drafts.join("survivor.md"),
            "---\nname: survivor\ndescription: the survivor\nproposed: true\nkind: merge\nevidence: 0.90 similar\n---\n\n# merge: absorbed into survivor\n\nthe body\n",
        )
        .unwrap();
        let proposal = load_one(&home, "survivor").expect("the draft loads");
        assert_eq!(proposal.change, Change::RetireSkill { name: "absorbed".to_string() });
        fs::write(
            drafts.join("broken-merge.md"),
            "---\nname: broken-merge\ndescription: a broken draft\nproposed: true\nkind: merge\nevidence: 0.90 similar\n---\n\nno header line\n",
        )
        .unwrap();
        let broken = load_one(&home, "broken-merge").expect("the draft loads");
        assert!(matches!(broken.change, Change::Unsupported { .. }));
    }

    // --- the stamp ---

    #[test]
    fn the_stamp_rewrites_the_verdict_fields_and_keeps_the_rest() {
        let home = scratch("stamp");
        let path = skill_proposal(&home, "stamped", "demo", r#"{"op": "retire-skill", "name": "demo"}"#);
        let before = fs::read_to_string(&path).unwrap();
        assert!(before.contains("status: pending"));
        stamp(&path, Status::Applied, Some("abc123"), "fmt ok · test ok").unwrap();
        let after = fs::read_to_string(&path).unwrap();
        assert!(after.contains("status: applied"), "{after}");
        assert!(after.contains("commit: abc123"), "{after}");
        assert!(after.contains("gate: fmt ok · test ok"), "{after}");
        // The body survived whole.
        assert!(after.contains("## Justification"), "{after}");
        assert!(after.contains("the session corrected a bash failure"), "{after}");
        // A second stamp replaces, never duplicates.
        stamp(&path, Status::Pending, None, "").unwrap();
        let again = fs::read_to_string(&path).unwrap();
        assert_eq!(again.matches("status:").count(), 1, "{again}");
        assert!(!again.contains("commit:"), "{again}");
        assert!(!again.contains("gate:"), "{again}");
        let reloaded = load_one(&home, "stamped").unwrap();
        assert_eq!(reloaded.status, Status::Pending);
    }
}
