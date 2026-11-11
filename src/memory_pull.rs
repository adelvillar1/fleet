//! The cross-harness memory pull — the shared corpus's local feeder.
//!
//! Fleet imports a project's facts from other installed harnesses'
//! memory stores into its own substrate: durable, git-tracked, recallable
//! as ordinary facts. This is the inverse of the one-time zcode → fleet
//! push transplant, made ongoing — and it is an IMPORT, not a federated
//! recall: a fresh armada instance working an existing project has no
//! local foreign stores, so the memories must live in the corpus armada
//! syncs, not behind a live read (the portability ruling, the
//! cross-harness-memory-pull plan).
//!
//! One path, two triggers: a bootstrap imports all of a project's foreign
//! facts once, and a revision-cursor delta imports only what changed
//! since the last pull. The cursor is a generic revision — the store's
//! git HEAD where it is git-tracked, else a stable content hash — so a
//! non-git local store still syncs and armada's git commits plug in
//! unchanged. The [`Source`] abstraction is the seam: [`LocalHarness`]
//! reads an installed harness's store now (zcode first), and
//! [`RemoteInstance`] is armada's named stub, the same interface waiting
//! for the remote feeder.
//!
//! The own-home ruling holds at every read: knowledge only. The feeder
//! reads a foreign harness's memory files and never its credentials or
//! config, and nothing foreign is ever written — discovery mirrors
//! [`crate::mcp_registry::scan`]'s "reads them and never writes them".
//! Every import lands through the substrate's own [`Remember`] writer,
//! so the validation the tool path enforces is the validation the feeder
//! enforces; a foreign fact that does not read as a fleet fact is a
//! named skip, never garbage in the corpus.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use sha2::Digest;

use crate::calib;
use crate::memory::{load_facts, Fact, Memory, Remember, Supersede, MAX_NAME_CHARS};

// --- the source abstraction --------------------------------------------------

/// One fact as a source yields it: the fields the writer needs, already
/// in fleet's vocabulary. The source owns the mapping from its own
/// frontmatter shape; the pull owns the import. The name is the foreign
/// store's own — the pull slugifies it into a fleet fact name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceFact {
    pub name: String,
    pub description: String,
    /// The foreign type, mapped onto fleet's fact vocabulary by the
    /// source's reader.
    pub fact_type: String,
    pub body: String,
}

/// One read of a project's store: every fact it holds, the revision the
/// read stands at, and the named skips — files that exist but did not
/// read as facts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    /// The store's current revision: a git HEAD or a content hash. The
    /// cursor's generic shape — armada's commits ride the same field.
    pub revision: String,
    pub facts: Vec<SourceFact>,
    pub skips: Vec<String>,
}

/// A project's facts plus a revision cursor — the feeder's one
/// abstraction, so the pull is source-agnostic. `LocalHarness` reads an
/// installed harness's store now; `RemoteInstance` (armada) plugs in
/// later with the same interface.
pub trait Source {
    /// The source's own name — the provenance prefix (`zcode`).
    fn harness(&self) -> &str;
    /// The project this source reads.
    fn project(&self) -> &str;
    /// Read the project's facts and the store's current revision. An
    /// absent store is a named absence ([`PullError::Absent`]), never an
    /// empty success.
    fn read(&self) -> Result<Snapshot, PullError>;
}

/// Why a pull could not run. A typed error, the writer's own shape: the
/// CLI prints it and the feeder's caller names it, and both should say
/// the same thing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PullError {
    /// The foreign store is not installed for this project — a named
    /// absence, never a silent skip.
    Absent(String),
    /// The store exists but could not be read.
    Io(String),
    /// The source is a named stub: the feeder is not built yet.
    NotYetBuilt(String),
}

impl std::fmt::Display for PullError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PullError::Absent(reason) => write!(f, "{reason}"),
            PullError::Io(reason) => write!(f, "{reason}"),
            PullError::NotYetBuilt(reason) => write!(f, "{reason}"),
        }
    }
}

impl std::error::Error for PullError {}

// --- the local harness source ------------------------------------------------

/// The foreign memory stores under the USER's home, as `(harness,
/// user-home relative path)` pairs — the memory-side sibling of
/// [`crate::mcp_registry::FOREIGN_HARNESSES`]. The scan reads them and
/// never writes them: import is one-way, into the fleet corpus. Each
/// path is a store's project directory; the project's own folder is
/// resolved inside it.
pub const FOREIGN_MEMORY_STORES: &[(&str, &str)] = &[("zcode", ".zcode/cli/memories/projects")];

/// An installed harness's memory store, read locally — zcode first,
/// hermes next. The store is project-scoped: zcode keeps one folder per
/// project under its projects directory, named `<project>-<hash>`.
#[derive(Debug, Clone)]
pub struct LocalHarness {
    harness: String,
    project: String,
    /// The store's projects directory, resolved against the user home
    /// at construction so a test can script it in a temp dir.
    projects_dir: PathBuf,
    /// The project's own path, when the caller knows it — the exact
    /// slug's input, the disambiguator when two checkouts share a
    /// project name.
    project_path: Option<PathBuf>,
}

impl LocalHarness {
    /// The source for one harness and project under the user home.
    /// `user_home` is passed in so the discovery is testable without
    /// mutating the process environment every other test reads.
    pub fn new(harness: &str, project: &str, user_home: &Path) -> Result<LocalHarness, PullError> {
        let Some((_, relative)) = FOREIGN_MEMORY_STORES
            .iter()
            .find(|(name, _)| *name == harness)
        else {
            return Err(PullError::Absent(format!(
                "`{harness}` is not a known memory harness (known: {})",
                FOREIGN_MEMORY_STORES
                    .iter()
                    .map(|(name, _)| *name)
                    .collect::<Vec<_>>()
                    .join(", ")
            )));
        };
        Ok(LocalHarness {
            harness: harness.to_string(),
            project: project.to_string(),
            projects_dir: user_home.join(relative),
            project_path: None,
        })
    }

    /// The project's own path — the working directory or the repo root.
    /// zcode's slug hash is the path's SHA-256, so a known path resolves
    /// the exact store even when another checkout shares the name.
    pub fn with_project_path(mut self, path: &Path) -> LocalHarness {
        self.project_path = Some(path.to_path_buf());
        self
    }

    /// The project's store directory: the one folder under the projects
    /// directory whose slug names this project. zcode's slug is
    /// `<name>-<hash>` — the name lowercased, the hash the first 16 hex
    /// of the project path's SHA-256 — so the match strips the trailing
    /// hash and compares names, and a path whose exact slug is present
    /// wins over a same-named checkout elsewhere. An absent store is a
    /// named absence.
    fn store_dir(&self) -> Result<PathBuf, PullError> {
        let absent = || {
            PullError::Absent(format!(
                "no {} memory store for project `{}` under {}",
                self.harness,
                self.project,
                self.projects_dir.display()
            ))
        };
        // The exact slug, when the project's path is known: zcode
        // derives it from the path, so it names this checkout's store
        // outright.
        if let Some(path) = &self.project_path {
            let exact = self.projects_dir.join(exact_slug(&self.project, path));
            if exact.is_dir() {
                return Ok(exact);
            }
        }
        let entries = match fs::read_dir(&self.projects_dir) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Err(absent());
            }
            Err(error) => {
                return Err(PullError::Io(format!(
                    "{} is unreadable: {error}",
                    self.projects_dir.display()
                )));
            }
        };
        let wanted = self.project.to_ascii_lowercase();
        let mut matches: Vec<PathBuf> = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let Some(slug) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            if slug_name(slug) == wanted {
                matches.push(path);
            }
        }
        matches.sort();
        match matches.len() {
            0 => Err(absent()),
            // One match is the store; several are same-named checkouts
            // and the first by name stands — the provenance names the
            // revision, so the pull's record says which store fed it.
            _ => Ok(matches.remove(0)),
        }
    }
}

impl Source for LocalHarness {
    fn harness(&self) -> &str {
        &self.harness
    }

    fn project(&self) -> &str {
        &self.project
    }

    fn read(&self) -> Result<Snapshot, PullError> {
        let store = self.store_dir()?;
        let memory_dir = store.join("memory");
        if !memory_dir.is_dir() {
            return Err(PullError::Absent(format!(
                "the {} store for project `{}` holds no memory directory ({})",
                self.harness,
                self.project,
                memory_dir.display()
            )));
        }
        let mut facts = Vec::new();
        let mut skips = Vec::new();
        let entries = fs::read_dir(&memory_dir).map_err(|error| {
            PullError::Io(format!("{} is unreadable: {error}", memory_dir.display()))
        })?;
        let mut paths: Vec<PathBuf> = entries
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| {
                path.is_file() && path.extension().and_then(|ext| ext.to_str()) == Some("md")
            })
            .collect();
        paths.sort();
        for path in paths {
            let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) else {
                continue;
            };
            // The index is the store's table of contents, not a fact.
            if stem == "MEMORY" {
                continue;
            }
            let text = match fs::read_to_string(&path) {
                Ok(text) => text,
                Err(error) => {
                    skips.push(format!("`{stem}` is unreadable: {error}"));
                    continue;
                }
            };
            match read_zcode_fact(&text, stem) {
                Ok(fact) => facts.push(fact),
                Err(reason) => skips.push(format!("`{stem}` — {reason}")),
            }
        }
        facts.sort_by(|left, right| left.name.cmp(&right.name));
        Ok(Snapshot {
            revision: store_revision(&memory_dir),
            facts,
            skips,
        })
    }
}

/// The project name a zcode slug carries: the slug minus its trailing
/// `-<16 hex>` hash. A slug with no hash suffix reads as its own name.
fn slug_name(slug: &str) -> String {
    match slug.rsplit_once('-') {
        Some((name, hash)) if hash.len() == 16 && hash.chars().all(|c| c.is_ascii_hexdigit()) => {
            name.to_ascii_lowercase()
        }
        _ => slug.to_ascii_lowercase(),
    }
}

/// The exact slug zcode derives for a project: the lowercased name and
/// the first 16 hex of the SHA-256 of the project path — verified
/// against the live store (`fleet-e2ba136ae3eba04b` is the hash of
/// `/Users/alejandrodelvillar/Projects/fleet`). The name-prefix match
/// carries discovery when the path is unknown or the hash drifts.
fn exact_slug(project: &str, path: &Path) -> String {
    let digest = format!(
        "{:x}",
        sha2::Sha256::digest(path.as_os_str().as_encoded_bytes())
    );
    format!("{}-{}", project.to_ascii_lowercase(), &digest[..16])
}

/// The zcode type vocabulary mapped onto fleet's fact vocabulary — a
/// lookup, not a classification. The two vocabularies agree on the four
/// kinds; an unknown type rides the writer's default rather than
/// importing a word fleet's protected tier would not recognize.
fn map_zcode_type(foreign: &str) -> String {
    match foreign.trim().to_ascii_lowercase().as_str() {
        "user" => "user".to_string(),
        "feedback" => "feedback".to_string(),
        "project" => "project".to_string(),
        "reference" => "reference".to_string(),
        _ => crate::memory::DEFAULT_TYPE.to_string(),
    }
}

/// Read one zcode fact: the frontmatter's `name`, `description` (a
/// folded plain scalar, continuation lines joined), and the nested
/// `metadata.type`, then the body after the closing fence. The shape is
/// zcode's own — a `metadata:` block of indented sub-keys, which fleet's
/// frontmatter subset does not speak — so this reader is the foreign
/// side's mapper, and what it yields still rides the `Remember`
/// writer's validation before it lands.
fn read_zcode_fact(text: &str, stem: &str) -> Result<SourceFact, String> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let lines: Vec<&str> = text.lines().collect();
    if lines.first().map(|line| line.trim_end()) != Some("---") {
        return Err("no `---` frontmatter fence (the index or a stray file)".to_string());
    }
    let mut name: Option<String> = None;
    let mut description: Option<String> = None;
    let mut foreign_type: Option<String> = None;
    // The top-level field a continuation line extends, and whether the
    // cursor is inside the `metadata:` block.
    let mut current: Option<String> = None;
    let mut in_metadata = false;
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
        if line.starts_with(' ') || line.starts_with('\t') {
            if in_metadata {
                if let Some((key, value)) = trimmed.split_once(':') {
                    if key.trim() == "type" {
                        foreign_type = Some(unquote_foreign(value.trim()));
                    }
                }
                continue;
            }
            // A folded plain scalar: the continuation joins with one
            // space, fleet's own frontmatter rule.
            match current.as_deref() {
                Some("name") => {
                    if let Some(value) = &mut name {
                        value.push(' ');
                        value.push_str(trimmed);
                    }
                }
                Some("description") => {
                    if let Some(value) = &mut description {
                        value.push(' ');
                        value.push_str(trimmed);
                    }
                }
                _ => {}
            }
            continue;
        }
        let Some((key, value)) = trimmed.split_once(':') else {
            return Err(format!(
                "frontmatter line {} is not a `key: value` pair",
                index + 1
            ));
        };
        let key = key.trim();
        let value = unquote_foreign(value.trim());
        in_metadata = key == "metadata";
        current = Some(key.to_string());
        match key {
            "name" => name = Some(value),
            "description" => description = Some(value),
            _ => {}
        }
    }
    let Some(closing) = closing else {
        return Err("the frontmatter fence never closes".to_string());
    };
    let name = name
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| stem.to_string());
    let description = description
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            "the frontmatter carries no `description` — not a fact this feeder can import"
                .to_string()
        })?;
    let body = lines[closing + 1..].join("\n");
    let body = body.strip_prefix('\n').unwrap_or(&body).to_string();
    Ok(SourceFact {
        name,
        description,
        fact_type: map_zcode_type(foreign_type.as_deref().unwrap_or("")),
        body,
    })
}

/// Strip one layer of matching surrounding quotes — the foreign
/// writer's scalar form. No escape processing: a foreign description is
/// knowledge, read as written, and the `Remember` writer sanitizes it
/// on the way in.
fn unquote_foreign(value: &str) -> String {
    for quote in ['"', '\''] {
        if value.len() >= 2 && value.starts_with(quote) && value.ends_with(quote) {
            return value[1..value.len() - 1].to_string();
        }
    }
    value.to_string()
}

// --- the remote instance stub ------------------------------------------------

/// Armada's foreign-instance feeder — a NAMED STUB, not built yet. The
/// interface is the pull's own, so the remote feeder plugs in without
/// rework: a remote instance's corpus is git-tracked, its revision a
/// commit, and the import path is this module's unchanged. Until then
/// every read answers with the absence by name — never a silent no-op
/// (the armada direction note: the remote half arrives with armada).
#[derive(Debug, Clone)]
pub struct RemoteInstance {
    project: String,
}

impl RemoteInstance {
    pub fn new(project: &str) -> RemoteInstance {
        RemoteInstance {
            project: project.to_string(),
        }
    }
}

impl Source for RemoteInstance {
    fn harness(&self) -> &str {
        "armada"
    }

    fn project(&self) -> &str {
        &self.project
    }

    fn read(&self) -> Result<Snapshot, PullError> {
        Err(PullError::NotYetBuilt(format!(
            "the armada remote-instance feeder is not yet built — project `{}` has no remote source to pull from (the local harness feeder is `--source zcode`)",
            self.project
        )))
    }
}

// --- the revision cursor -----------------------------------------------------

/// The store's current revision: its git HEAD where the store is
/// git-tracked, else a stable content hash. The hash is sorted and
/// normalized — file names in order, CRLF folded to LF, trailing
/// whitespace trimmed — so an unchanged store reads unchanged and a
/// non-git store still syncs. Armada's git commits ride the same field.
pub fn store_revision(dir: &Path) -> String {
    if let Some(head) = git_head(dir) {
        return format!("git:{head}");
    }
    let mut hasher = sha2::Sha256::new();
    let mut paths: Vec<PathBuf> = match fs::read_dir(dir) {
        Ok(entries) => entries
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.is_file())
            .collect(),
        Err(_) => Vec::new(),
    };
    paths.sort();
    for path in paths {
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let Ok(bytes) = fs::read(&path) else {
            continue;
        };
        let text = String::from_utf8_lossy(&bytes);
        let normalized: String = text
            .replace("\r\n", "\n")
            .lines()
            .map(|line| line.trim_end())
            .collect::<Vec<_>>()
            .join("\n");
        hasher.update(name.as_bytes());
        hasher.update([0]);
        hasher.update(normalized.as_bytes());
        hasher.update([0]);
    }
    format!("content:{:x}", hasher.finalize())
}

/// The store's git HEAD, when the store is inside a git work tree.
/// `None` for a non-git store — the content hash's cue — and for a git
/// that cannot answer.
fn git_head(dir: &Path) -> Option<String> {
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(dir)
        .arg("rev-parse")
        .arg("HEAD")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let head = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (!head.is_empty()).then_some(head)
}

/// The cursor sidecar's file name, under the fleet home.
pub const CURSORS_FILE: &str = "memory-pull-cursors.json";

/// The last-pulled revision per project per source — the sidecar under
/// the fleet home. A missing or unreadable sidecar reads as no cursors:
/// the next pull is a bootstrap, which the content dedup makes safe.
pub fn read_cursors(home: &Path) -> BTreeMap<String, String> {
    let path = home.join(CURSORS_FILE);
    let Ok(text) = fs::read_to_string(&path) else {
        return BTreeMap::new();
    };
    serde_json::from_str::<BTreeMap<String, String>>(&text).unwrap_or_default()
}

/// Write the sidecar back, whole. The map is small — one row per
/// project per source — so a whole rewrite is the simple atomic shape:
/// a temp file and a rename, the corpus's own write rule.
pub fn write_cursors(home: &Path, cursors: &BTreeMap<String, String>) -> Result<(), PullError> {
    fs::create_dir_all(home)
        .map_err(|error| PullError::Io(format!("could not create {}: {error}", home.display())))?;
    let path = home.join(CURSORS_FILE);
    let text = serde_json::to_string_pretty(cursors)
        .map_err(|error| PullError::Io(format!("the cursors would not serialize: {error}")))?;
    let temp = path.with_extension("json.tmp");
    fs::write(&temp, format!("{text}\n"))
        .map_err(|error| PullError::Io(format!("could not write {}: {error}", temp.display())))?;
    fs::rename(&temp, &path)
        .map_err(|error| PullError::Io(format!("could not replace {}: {error}", path.display())))?;
    Ok(())
}

/// The sidecar key: one cursor per project per source, never global.
pub fn cursor_key(harness: &str, project: &str) -> String {
    format!("{harness}:{project}")
}

// --- the import --------------------------------------------------------------

/// What one pull did, fact by fact — the report the surface prints and
/// the tests assert. Every disposition is named: an import that skipped
/// a collision says whose name it was.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PullReport {
    pub harness: String,
    pub project: String,
    /// The revision the pull read.
    pub revision: String,
    /// The revision the cursor held before the pull, when it held one.
    pub previous: Option<String>,
    /// The revision was the cursor's own: nothing was imported and the
    /// cursor did not move.
    pub unchanged: bool,
    pub actions: Vec<ImportAction>,
    /// The source's named skips — foreign files that did not read as
    /// facts.
    pub skips: Vec<String>,
}

/// One fact's disposition in a pull.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportAction {
    /// A new fact landed in the substrate.
    Added { name: String },
    /// The source fact changed: the imported copy was superseded into
    /// its replacement.
    Updated { from: String, into: String },
    /// The source fact is byte-for-byte what the substrate holds.
    Unchanged { name: String },
    /// The source fact was deleted: the imported copy was archived,
    /// provenance preserved, never hard-deleted.
    Archived { name: String },
    /// The fact did not land, and the reason names why — a collision
    /// with a native fact, a name the writer refuses, a validation
    /// failure.
    Skipped { name: String, reason: String },
}

impl ImportAction {
    /// The fact's name, whichever disposition it carries.
    pub fn name(&self) -> &str {
        match self {
            ImportAction::Added { name }
            | ImportAction::Unchanged { name }
            | ImportAction::Archived { name }
            | ImportAction::Skipped { name, .. } => name,
            ImportAction::Updated { from, .. } => from,
        }
    }
}

/// Whether a fleet fact is this feeder's own imported copy: its
/// provenance names the harness and the project. The prefix is the
/// ownership rule — a native fact, or an import from another project,
/// is never this pull's to supersede or archive.
fn imported_by(fact: &Fact, harness: &str, project: &str) -> bool {
    fact.source.starts_with(&format!("{harness}:{project}:"))
}

/// The provenance an import carries: `<harness>:<project>:<revision>`,
/// bounded the way the writer bounds `source` — the pointer is
/// provenance, not prose.
fn provenance(harness: &str, project: &str, revision: &str) -> String {
    let pointer = format!("{harness}:{project}:{revision}");
    let cleaned: String = pointer.chars().filter(|c| !c.is_control()).collect();
    if cleaned.chars().count() > crate::memory::MAX_SOURCE_CHARS {
        let cut: String = cleaned
            .chars()
            .take(crate::memory::MAX_SOURCE_CHARS)
            .collect();
        format!("{cut}\u{2026}")
    } else {
        cleaned
    }
}

/// The foreign name as a fleet fact name: lowercase letters, digits,
/// and hyphens, bounded, no leading or trailing hyphen — the writer's
/// own grammar, applied before the writer sees it so a foreign name
/// that cannot be a key is a named skip rather than a refusal deep in
/// the write.
fn fleet_name(foreign: &str) -> String {
    let mut out = String::new();
    let mut pending_hyphen = false;
    for character in foreign.chars() {
        let lower = character.to_ascii_lowercase();
        if lower.is_ascii_lowercase() || lower.is_ascii_digit() {
            if pending_hyphen && !out.is_empty() {
                out.push('-');
            }
            pending_hyphen = false;
            out.push(lower);
        } else if !out.is_empty() {
            pending_hyphen = true;
        }
    }
    // The bound is the writer's own: cut at the bound, then drop the
    // hyphen the cut may have left trailing — a name that overshoots
    // the bound by the joining hyphen is a refusal, not a key.
    if out.chars().count() > MAX_NAME_CHARS {
        out = out.chars().take(MAX_NAME_CHARS).collect();
    }
    while out.ends_with('-') {
        out.pop();
    }
    out
}

/// The content digest that decides "changed": the mapped fact's own
/// fields, normalized the way the writer normalizes them, so a foreign
/// description past the writer's bound compares equal to the truncated
/// copy the substrate holds. The name is the key, not the content.
fn content_digest(description: &str, fact_type: &str, body: &str) -> String {
    let description = crate::memory::sanitize_description(description);
    let mut hasher = sha2::Sha256::new();
    hasher.update(description.as_bytes());
    hasher.update([0]);
    hasher.update(fact_type.as_bytes());
    hasher.update([0]);
    hasher.update(body.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// The replacement name for an updated import: the fact's own name with
/// a revision serial, the first free one — `-r2`, `-r3`, … — so the
/// supersession chain reads in order and a re-used name never collides.
/// The base is shortened to leave room for the suffix, so an update to a
/// fact whose name already sits at the bound still lands a valid key
/// rather than overshooting it.
fn replacement_name(root: &Path, name: &str) -> String {
    // Room for `-r` and a serial of up to four digits.
    let budget = MAX_NAME_CHARS.saturating_sub(6);
    let mut base: String = name.chars().take(budget).collect();
    while base.ends_with('-') {
        base.pop();
    }
    let mut serial = 2;
    loop {
        let candidate = format!("{base}-r{serial}");
        if !root.join(format!("{candidate}.md")).exists() {
            return candidate;
        }
        serial += 1;
    }
}

/// One pull: bootstrap or delta over one source, through the substrate's
/// own writer. The cursor decides which: a revision the sidecar already
/// holds imports nothing; a new revision (or no cursor at all) imports
/// the changed facts — added, superseded, archived — and advances the
/// cursor. The dedup is the substrate's own supersede semantics: a
/// collision with a native fact is named and skipped, never clobbered;
/// a collision with this feeder's own imported copy is the update path.
pub fn pull(
    source: &dyn Source,
    memory: &Memory,
    home: &Path,
    stream: Option<&calib::Stream>,
) -> Result<PullReport, PullError> {
    let snapshot = source.read()?;
    let key = cursor_key(source.harness(), source.project());
    let mut cursors = read_cursors(home);
    let previous = cursors.get(&key).cloned();
    let mut report = PullReport {
        harness: source.harness().to_string(),
        project: source.project().to_string(),
        revision: snapshot.revision.clone(),
        previous: previous.clone(),
        unchanged: false,
        actions: Vec::new(),
        skips: snapshot.skips.clone(),
    };
    if previous.as_deref() == Some(snapshot.revision.as_str()) {
        report.unchanged = true;
        return Ok(report);
    }

    let today = calib::today();
    let pointer = provenance(source.harness(), source.project(), &snapshot.revision);
    let existing = load_facts(memory.root());
    // The names this revision holds, so the delete half knows what went
    // missing at the source.
    let mut present: Vec<String> = Vec::new();
    for fact in &snapshot.facts {
        let name = fleet_name(&fact.name);
        if name.is_empty() {
            report.actions.push(ImportAction::Skipped {
                name: fact.name.clone(),
                reason: "the foreign name carries no letters or digits to make a fact name"
                    .to_string(),
            });
            continue;
        }
        // Two foreign facts can slug to one fleet name; the first lands
        // and the second is a named collision, never a silent overwrite
        // of the copy this same pull just wrote.
        if present.iter().any(|seen| seen == &name) {
            report.actions.push(ImportAction::Skipped {
                name,
                reason: "another foreign fact already maps to this name in the same pull"
                    .to_string(),
            });
            continue;
        }
        present.push(name.clone());
        let request = match Remember::parse(&serde_json::json!({
            "name": name,
            "description": fact.description,
            "type": fact.fact_type,
            "basis": "inferred",
            "source": pointer,
            "body": fact.body,
        })) {
            Ok(request) => request,
            Err(error) => {
                report.actions.push(ImportAction::Skipped {
                    name,
                    reason: format!("the writer refused the foreign fact: {error}"),
                });
                continue;
            }
        };
        let Some(held) = existing.iter().find(|held| held.name == name) else {
            match memory.remember(&request) {
                Ok(_) => report.actions.push(ImportAction::Added { name }),
                Err(error) => report.actions.push(ImportAction::Skipped {
                    name,
                    reason: format!("the write did not land: {error}"),
                }),
            }
            continue;
        };
        // A retired name is dead on disk until the cycle archives it;
        // the import does not resurrect it.
        if held.forgotten(&today) {
            report.actions.push(ImportAction::Skipped {
                name,
                reason: "the name is held by a retired fact".to_string(),
            });
            continue;
        }
        if !imported_by(held, source.harness(), source.project()) {
            // The dedup's load-bearing rule: a native fact — above all
            // an owner-stated one — is never clobbered by an import.
            report.actions.push(ImportAction::Skipped {
                name,
                reason: format!(
                    "a native fact holds the name (basis {}); the import does not clobber it",
                    held.basis.label()
                ),
            });
            continue;
        }
        let digest = content_digest(&request.description, &request.fact_type, &request.body);
        let held_digest = content_digest(&held.description, &held.fact_type, &held.body);
        if digest == held_digest {
            report.actions.push(ImportAction::Unchanged { name });
            continue;
        }
        // The update path: the source fact supersedes the imported
        // copy, the substrate's own modify semantics — the old file
        // stays, retired, provenance intact.
        let replacement = replacement_name(memory.root(), &name);
        let supersede = Supersede {
            name: name.clone(),
            justification: format!(
                "the {} source changed the fact at revision {}",
                source.harness(),
                snapshot.revision
            ),
            replacement: Some(replacement.clone()),
            description: request.description.clone(),
            body: request.body.clone(),
            fact_type: Some(request.fact_type.clone()),
            shape: None,
            missing: None,
            phrases: None,
            premises: None,
            valid_until: None,
        };
        match memory.supersede(&supersede, false, stream) {
            Ok(_) => {
                // The substrate's supersession carries the old evidence
                // pointer; the import's replacement stands at the NEW
                // revision, so the provenance is stamped through the
                // module's own in-place rewriter. A stamp that does not
                // land is named beside the supersession that did.
                let replacement_path = memory.root().join(format!("{replacement}.md"));
                match crate::memory::rewrite_field(&replacement_path, "source", &pointer) {
                    Ok(()) => report.actions.push(ImportAction::Updated {
                        from: name,
                        into: replacement,
                    }),
                    Err(error) => report.actions.push(ImportAction::Skipped {
                        name,
                        reason: format!(
                            "the supersession into `{replacement}` landed, but its provenance stamp did not: {error}"
                        ),
                    }),
                }
            }
            Err(error) => report.actions.push(ImportAction::Skipped {
                name,
                reason: format!("the supersession did not land: {error}"),
            }),
        }
    }

    // The delete half: an imported copy whose fact the source no longer
    // holds is archived — moved to the archive, provenance preserved,
    // never hard-deleted. Only this feeder's own live copies for this
    // project: a native fact and a retired one are never the pull's to
    // move.
    for held in &existing {
        if !imported_by(held, source.harness(), source.project()) || held.forgotten(&today) {
            continue;
        }
        if present.iter().any(|name| name == &held.name) {
            continue;
        }
        if archive_imported(memory.root(), held) {
            report.actions.push(ImportAction::Archived {
                name: held.name.clone(),
            });
        } else {
            report.actions.push(ImportAction::Skipped {
                name: held.name.clone(),
                reason: "the source deleted the fact, but the archive move did not land"
                    .to_string(),
            });
        }
    }

    cursors.insert(key, snapshot.revision.clone());
    write_cursors(home, &cursors)?;
    Ok(report)
}

/// Move one deleted-at-source import into the archive, through the
/// cycle's own archive path — a rename, not a delete, with the
/// collision serial the archive's re-used names ride.
fn archive_imported(root: &Path, fact: &Fact) -> bool {
    crate::memory::archive_fact(root, fact)
}

// --- the surface -------------------------------------------------------------

/// The pull's report as the surface prints it: the source and revision,
/// then one line per fact's disposition, then the named skips. An
/// unchanged revision says so in one line.
pub fn render(report: &PullReport) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "memory pull — {}:{} at {}\n",
        report.harness, report.project, report.revision
    ));
    if report.unchanged {
        out.push_str("  unchanged since the last pull — nothing imported\n");
        return out;
    }
    if let Some(previous) = &report.previous {
        out.push_str(&format!("  delta since {previous}\n"));
    } else {
        out.push_str("  bootstrap — no cursor held\n");
    }
    for action in &report.actions {
        match action {
            ImportAction::Added { name } => {
                out.push_str(&format!("  added      {name}\n"));
            }
            ImportAction::Updated { from, into } => {
                out.push_str(&format!("  updated    {from} -> {into}\n"));
            }
            ImportAction::Unchanged { name } => {
                out.push_str(&format!("  unchanged  {name}\n"));
            }
            ImportAction::Archived { name } => {
                out.push_str(&format!("  archived   {name} (deleted at the source)\n"));
            }
            ImportAction::Skipped { name, reason } => {
                out.push_str(&format!("  skipped    {name} — {reason}\n"));
            }
        }
    }
    for skip in &report.skips {
        out.push_str(&format!("  unreadable {skip}\n"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::{load_archive, Basis, KeywordOnly};

    /// A scratch directory, unique per call — the corpus tests' own
    /// shape.
    fn scratch(tag: &str) -> PathBuf {
        static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let serial = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "fleet-memory-pull-{tag}-{}-{serial}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// One scripted zcode fact, the store's own frontmatter shape: a
    /// folded description, the nested metadata block, the body after
    /// the fence.
    fn zcode_fact(name: &str, description: &str, kind: &str, body: &str) -> String {
        format!(
            "---\nname: {name}\ndescription: {description}\nmetadata:\n  node_type: memory\n  type: {kind}\n  originSessionId: sess_scripted\n---\n\n{body}\n"
        )
    }

    /// Script one project's store under a fake user home: the projects
    /// directory, the slug folder, the memory files. Returns the slug
    /// directory.
    fn script_store(user_home: &Path, slug: &str, files: &[(&str, &str)]) -> PathBuf {
        let memory = user_home
            .join(".zcode/cli/memories/projects")
            .join(slug)
            .join("memory");
        fs::create_dir_all(&memory).unwrap();
        for (name, text) in files {
            fs::write(memory.join(name), text).unwrap();
        }
        memory.parent().unwrap().to_path_buf()
    }

    /// A keyword-only corpus over a scratch root — the feeder writes
    /// through the real writer; the vectors are not the pull's test.
    fn corpus(root: &Path) -> Memory {
        Memory::load_with(root, Box::new(KeywordOnly))
    }

    fn action_names(report: &PullReport) -> Vec<(String, String)> {
        report
            .actions
            .iter()
            .map(|action| {
                let kind = match action {
                    ImportAction::Added { .. } => "added",
                    ImportAction::Updated { .. } => "updated",
                    ImportAction::Unchanged { .. } => "unchanged",
                    ImportAction::Archived { .. } => "archived",
                    ImportAction::Skipped { .. } => "skipped",
                };
                (kind.to_string(), action.name().to_string())
            })
            .collect()
    }

    /// C0 — discovery names the installed store and the project's facts,
    /// an absent store is a named absence, and nothing foreign is
    /// written: the store's bytes are the same after the read.
    #[test]
    fn discovery_names_the_store_and_the_absence() {
        let home = scratch("discovery");
        let user_home = home.join("user");
        let store = script_store(
            &user_home,
            "fleet-e2ba136ae3eba04b",
            &[
                (
                    "[feedback] a-ruling.md",
                    &zcode_fact(
                        "a-ruling",
                        "the owner ruled it so",
                        "feedback",
                        "The ruling.",
                    ),
                ),
                (
                    "a-project-fact.md",
                    &zcode_fact(
                        "a-project-fact",
                        "the project ships on friday",
                        "project",
                        "Body.",
                    ),
                ),
                ("MEMORY.md", "- [index](a-ruling.md) — not a fact\n"),
            ],
        );
        let before = fs::read_to_string(store.join("memory/MEMORY.md")).unwrap();

        let source = LocalHarness::new("zcode", "fleet", &user_home).unwrap();
        let snapshot = source.read().unwrap();
        assert_eq!(snapshot.facts.len(), 2, "the index is not a fact");
        assert_eq!(snapshot.facts[0].name, "a-project-fact");
        assert_eq!(snapshot.facts[1].name, "a-ruling");
        assert_eq!(snapshot.facts[1].fact_type, "feedback");
        assert!(snapshot.skips.is_empty());
        assert!(snapshot.revision.starts_with("content:"), "a non-git store");

        // The read wrote nothing foreign.
        assert_eq!(
            fs::read_to_string(store.join("memory/MEMORY.md")).unwrap(),
            before
        );

        // An absent project is a named absence, never an empty success.
        let absent = LocalHarness::new("zcode", "no-such-project", &user_home).unwrap();
        let error = absent.read().unwrap_err();
        assert!(matches!(error, PullError::Absent(_)), "{error}");
        assert!(error.to_string().contains("no-such-project"), "{error}");

        // An unknown harness is named too.
        let unknown = LocalHarness::new("hermes", "fleet", &user_home).unwrap_err();
        assert!(unknown.to_string().contains("hermes"), "{unknown}");
        let _ = fs::remove_dir_all(&home);
    }

    /// C0 — a foreign file that does not read as a fact is a named skip,
    /// and the facts beside it still read.
    #[test]
    fn discovery_names_the_files_it_cannot_read() {
        let home = scratch("discovery-skips");
        let user_home = home.join("user");
        script_store(
            &user_home,
            "fleet-e2ba136ae3eba04b",
            &[
                (
                    "good.md",
                    &zcode_fact("good", "a readable fact", "project", "Body."),
                ),
                ("stray.md", "no frontmatter here, just prose\n"),
                (
                    "no-description.md",
                    "---\nname: no-description\nmetadata:\n  type: project\n---\n\nBody.\n",
                ),
            ],
        );
        let source = LocalHarness::new("zcode", "fleet", &user_home).unwrap();
        let snapshot = source.read().unwrap();
        assert_eq!(snapshot.facts.len(), 1);
        assert_eq!(snapshot.skips.len(), 2, "{:?}", snapshot.skips);
        assert!(snapshot.skips.iter().any(|skip| skip.contains("stray")));
        assert!(snapshot
            .skips
            .iter()
            .any(|skip| skip.contains("no-description")));
        let _ = fs::remove_dir_all(&home);
    }

    /// C1 — the bootstrap imports every foreign fact through the writer:
    /// provenance-tagged, basis inferred, type-mapped, body carried —
    /// and a collision with an owner-stated native fact is named and
    /// skipped, never clobbered.
    #[test]
    fn bootstrap_imports_provenance_tagged_and_deduped() {
        let home = scratch("bootstrap");
        let user_home = home.join("user");
        let root = home.join("memories");
        script_store(
            &user_home,
            "fleet-e2ba136ae3eba04b",
            &[
                (
                    "[user] the-owner.md",
                    &zcode_fact("the-owner", "the owner is alejandro", "user", "Identity."),
                ),
                (
                    "a-project-fact.md",
                    &zcode_fact(
                        "a-project-fact",
                        "the project ships on friday",
                        "project",
                        "The full body.",
                    ),
                ),
                (
                    "a-reference.md",
                    &zcode_fact("a-reference", "the api lives at /v1", "reference", "Ref."),
                ),
                (
                    "an-unknown-type.md",
                    &zcode_fact(
                        "an-unknown-type",
                        "a fact of a foreign kind",
                        "exotic",
                        "Body.",
                    ),
                ),
            ],
        );
        // A native owner-stated fact holding one of the foreign names.
        let memory = corpus(&root);
        memory
            .remember(
                &Remember::parse(&serde_json::json!({
                    "name": "a-project-fact",
                    "description": "the owner's own words about the project",
                    "basis": "confirmed-by",
                }))
                .unwrap(),
            )
            .unwrap();

        let source = LocalHarness::new("zcode", "fleet", &user_home).unwrap();
        let report = pull(&source, &memory, &home, None).unwrap();
        assert!(!report.unchanged);
        assert!(report.previous.is_none());

        let facts = load_facts(&root);
        // The three free names landed; the collision did not.
        let landed = facts.iter().find(|fact| fact.name == "the-owner").unwrap();
        assert_eq!(landed.basis, Basis::Inferred);
        assert_eq!(landed.fact_type, "user");
        assert!(
            landed.source.starts_with("zcode:fleet:"),
            "{}",
            landed.source
        );
        assert!(landed.source.contains(&report.revision));
        assert_eq!(landed.body, "Identity.");
        let reference = facts
            .iter()
            .find(|fact| fact.name == "a-reference")
            .unwrap();
        assert_eq!(reference.fact_type, "reference");
        // The unknown type rode the writer's default, not garbage.
        let unknown = facts
            .iter()
            .find(|fact| fact.name == "an-unknown-type")
            .unwrap();
        assert_eq!(unknown.fact_type, crate::memory::DEFAULT_TYPE);
        // The native fact stands exactly as the owner stated it.
        let native = facts
            .iter()
            .find(|fact| fact.name == "a-project-fact")
            .unwrap();
        assert_eq!(native.basis, Basis::ConfirmedBy);
        assert_eq!(
            native.description,
            "the owner's own words about the project"
        );
        assert!(native.source.is_empty());
        // The collision is named in the report.
        let collision = report
            .actions
            .iter()
            .find(|action| action.name() == "a-project-fact")
            .unwrap();
        assert!(
            matches!(collision, ImportAction::Skipped { reason, .. } if reason.contains("native")),
            "{collision:?}"
        );
        // The cursor advanced to the store's revision.
        let cursors = read_cursors(&home);
        assert_eq!(
            cursors.get(&cursor_key("zcode", "fleet")),
            Some(&report.revision)
        );
        let _ = fs::remove_dir_all(&home);
    }

    /// C2 — the two-revision delta: an unchanged revision imports
    /// nothing; a new revision imports only the changed facts and
    /// advances the cursor.
    #[test]
    fn the_cursor_delta_imports_only_what_changed() {
        let home = scratch("delta");
        let user_home = home.join("user");
        let root = home.join("memories");
        let store = script_store(
            &user_home,
            "fleet-e2ba136ae3eba04b",
            &[
                (
                    "stable.md",
                    &zcode_fact("stable", "a fact that never changes", "project", "Stable."),
                ),
                (
                    "changing.md",
                    &zcode_fact("changing", "the first revision", "project", "First."),
                ),
            ],
        );
        let memory = corpus(&root);
        let source = LocalHarness::new("zcode", "fleet", &user_home).unwrap();

        // Bootstrap.
        let first = pull(&source, &memory, &home, None).unwrap();
        assert_eq!(first.actions.len(), 2);
        assert!(first
            .actions
            .iter()
            .all(|action| matches!(action, ImportAction::Added { .. })));

        // The same revision imports nothing and does not move the cursor.
        let again = pull(&source, &memory, &home, None).unwrap();
        assert!(again.unchanged);
        assert!(again.actions.is_empty());
        assert_eq!(again.revision, first.revision);

        // A new revision: one fact changes, one stays, one arrives.
        fs::write(
            store.join("memory/changing.md"),
            zcode_fact("changing", "the second revision", "project", "Second."),
        )
        .unwrap();
        fs::write(
            store.join("memory/newcomer.md"),
            zcode_fact(
                "newcomer",
                "a fact the second revision adds",
                "project",
                "New.",
            ),
        )
        .unwrap();
        let second = pull(&source, &memory, &home, None).unwrap();
        assert!(!second.unchanged);
        assert_ne!(second.revision, first.revision);
        assert_eq!(second.previous.as_deref(), Some(first.revision.as_str()));
        let names = action_names(&second);
        assert!(names.contains(&("unchanged".to_string(), "stable".to_string())));
        assert!(names.contains(&("added".to_string(), "newcomer".to_string())));
        assert!(names.contains(&("updated".to_string(), "changing".to_string())));
        assert_eq!(second.actions.len(), 3, "{names:?}");
        // The cursor advanced.
        let cursors = read_cursors(&home);
        assert_eq!(
            cursors.get(&cursor_key("zcode", "fleet")),
            Some(&second.revision)
        );
        // And the next read of the same revision is unchanged again.
        let third = pull(&source, &memory, &home, None).unwrap();
        assert!(third.unchanged);
        let _ = fs::remove_dir_all(&home);
    }

    /// C3 — the import semantics: an updated source fact supersedes the
    /// imported copy (the old file stays, retired), and a deleted source
    /// fact archives the imported copy — provenance preserved, never a
    /// hard delete.
    #[test]
    fn an_update_supersedes_and_a_delete_archives() {
        let home = scratch("semantics");
        let user_home = home.join("user");
        let root = home.join("memories");
        let store = script_store(
            &user_home,
            "fleet-e2ba136ae3eba04b",
            &[
                (
                    "updated.md",
                    &zcode_fact("updated", "the first words", "project", "First body."),
                ),
                (
                    "deleted.md",
                    &zcode_fact(
                        "deleted",
                        "a fact the source will drop",
                        "project",
                        "Doomed.",
                    ),
                ),
            ],
        );
        let memory = corpus(&root);
        let source = LocalHarness::new("zcode", "fleet", &user_home).unwrap();
        pull(&source, &memory, &home, None).unwrap();

        // Revision two: one fact reworded, one removed.
        fs::write(
            store.join("memory/updated.md"),
            zcode_fact("updated", "the second words", "project", "Second body."),
        )
        .unwrap();
        fs::remove_file(store.join("memory/deleted.md")).unwrap();
        let report = pull(&source, &memory, &home, None).unwrap();
        let names = action_names(&report);
        assert!(names.contains(&("updated".to_string(), "updated".to_string())));
        assert!(names.contains(&("archived".to_string(), "deleted".to_string())));

        // The supersession: the old copy stays on disk, retired into the
        // replacement, and the replacement carries the new content and
        // the fresh provenance.
        let facts = load_facts(&root);
        let old = facts.iter().find(|fact| fact.name == "updated").unwrap();
        assert_eq!(old.superseded_by, "updated-r2");
        let new = facts.iter().find(|fact| fact.name == "updated-r2").unwrap();
        assert_eq!(new.description, "the second words");
        assert_eq!(new.body, "Second body.");
        assert!(new.source.starts_with("zcode:fleet:"));
        assert!(new.source.contains(&report.revision));

        // The delete: the copy moved to the archive, whole, provenance
        // intact — nothing hard-deleted.
        assert!(!root.join("deleted.md").exists());
        let archived = load_archive(&root);
        let archived = archived
            .iter()
            .find(|fact| fact.name == "deleted")
            .expect("the deleted fact is archived, not gone");
        assert_eq!(archived.description, "a fact the source will drop");
        assert!(archived.source.starts_with("zcode:fleet:"));
        let _ = fs::remove_dir_all(&home);
    }

    /// C4 — the pull is source-agnostic: a scripted source drives the
    /// same path, and the RemoteInstance stub names its absence through
    /// the same interface.
    #[test]
    fn the_pull_rides_the_source_abstraction() {
        /// A scripted source: facts and a revision in hand, no disk.
        struct Scripted {
            revision: String,
            facts: Vec<SourceFact>,
        }
        impl Source for Scripted {
            fn harness(&self) -> &str {
                "scripted"
            }
            fn project(&self) -> &str {
                "demo"
            }
            fn read(&self) -> Result<Snapshot, PullError> {
                Ok(Snapshot {
                    revision: self.revision.clone(),
                    facts: self.facts.clone(),
                    skips: Vec::new(),
                })
            }
        }

        let home = scratch("abstraction");
        let root = home.join("memories");
        let memory = corpus(&root);
        let scripted = Scripted {
            revision: "git:abc123".to_string(),
            facts: vec![SourceFact {
                name: "scripted-fact".to_string(),
                description: "a fact from a scripted source".to_string(),
                fact_type: "project".to_string(),
                body: "Scripted body.".to_string(),
            }],
        };
        let report = pull(&scripted, &memory, &home, None).unwrap();
        assert_eq!(report.harness, "scripted");
        assert_eq!(report.actions.len(), 1);
        let facts = load_facts(&root);
        assert_eq!(facts.len(), 1);
        assert!(facts[0].source.starts_with("scripted:demo:git:abc123"));
        // The cursor is the generic revision, whatever the source.
        let cursors = read_cursors(&home);
        assert_eq!(
            cursors.get(&cursor_key("scripted", "demo")),
            Some(&"git:abc123".to_string())
        );

        // The armada stub: the same interface, a named absence.
        let remote = RemoteInstance::new("demo");
        assert_eq!(remote.harness(), "armada");
        let error = remote.read().unwrap_err();
        assert!(matches!(error, PullError::NotYetBuilt(_)), "{error}");
        assert!(error.to_string().contains("not yet built"), "{error}");
        let error = pull(&remote, &memory, &home, None).unwrap_err();
        assert!(error.to_string().contains("armada"), "{error}");
        let _ = fs::remove_dir_all(&home);
    }

    /// The revision cursor's stability: an unchanged non-git store reads
    /// the same revision twice, a changed one reads a different one, and
    /// the hash ignores trailing whitespace and line-ending noise.
    #[test]
    fn the_content_revision_is_stable() {
        let home = scratch("revision");
        let dir = home.join("store");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("b.md"), "second\n").unwrap();
        fs::write(dir.join("a.md"), "first\n").unwrap();
        let first = store_revision(&dir);
        assert!(first.starts_with("content:"));
        assert_eq!(
            store_revision(&dir),
            first,
            "an unchanged store reads unchanged"
        );
        // Normalization: CRLF and trailing whitespace are not a change.
        fs::write(dir.join("a.md"), "first  \r\n").unwrap();
        assert_eq!(store_revision(&dir), first);
        // A real change is a new revision.
        fs::write(dir.join("a.md"), "changed\n").unwrap();
        assert_ne!(store_revision(&dir), first);
        let _ = fs::remove_dir_all(&home);
    }

    /// The name mapper: a foreign name becomes a fleet key, and a name
    /// with nothing to map is empty — the pull's named skip.
    #[test]
    fn foreign_names_map_to_fleet_keys() {
        assert_eq!(fleet_name("A Bot Name"), "a-bot-name");
        assert_eq!(fleet_name("[feedback] a-ruling"), "feedback-a-ruling");
        assert_eq!(fleet_name("dots.and_underscores"), "dots-and-underscores");
        assert_eq!(fleet_name("---"), "");
        assert_eq!(fleet_name("  "), "");
        let long = "x".repeat(MAX_NAME_CHARS + 20);
        assert!(fleet_name(&long).chars().count() <= MAX_NAME_CHARS);
        // The joining hyphen at the bound must not overshoot it: 63
        // letters, a separator, then more — the cut stays within 64 and
        // drops any trailing hyphen.
        let boundary = format!("{} tail", "a".repeat(MAX_NAME_CHARS - 1));
        let mapped = fleet_name(&boundary);
        assert!(mapped.chars().count() <= MAX_NAME_CHARS, "{mapped}");
        assert!(!mapped.ends_with('-'), "{mapped}");
    }

    /// Two foreign facts that slug to one fleet name: the first lands,
    /// the second is a named collision — never a silent overwrite of the
    /// copy the same pull just wrote.
    #[test]
    fn two_foreign_facts_mapping_to_one_name_collide_by_name() {
        struct Scripted;
        impl Source for Scripted {
            fn harness(&self) -> &str {
                "scripted"
            }
            fn project(&self) -> &str {
                "demo"
            }
            fn read(&self) -> Result<Snapshot, PullError> {
                Ok(Snapshot {
                    revision: "content:one".to_string(),
                    facts: vec![
                        SourceFact {
                            name: "same name".to_string(),
                            description: "the first fact".to_string(),
                            fact_type: "project".to_string(),
                            body: "First.".to_string(),
                        },
                        SourceFact {
                            name: "same-name".to_string(),
                            description: "the second fact".to_string(),
                            fact_type: "project".to_string(),
                            body: "Second.".to_string(),
                        },
                    ],
                    skips: Vec::new(),
                })
            }
        }
        let home = scratch("batch-collision");
        let root = home.join("memories");
        let memory = corpus(&root);
        let report = pull(&Scripted, &memory, &home, None).unwrap();
        let names = action_names(&report);
        assert!(names.contains(&("added".to_string(), "same-name".to_string())));
        let collision = report
            .actions
            .iter()
            .filter(|action| action.name() == "same-name")
            .count();
        assert_eq!(collision, 2, "both dispositions named: {names:?}");
        assert!(report.actions.iter().any(|action| matches!(
            action,
            ImportAction::Skipped { reason, .. } if reason.contains("same pull")
        )));
        // Exactly one file landed — the second did not clobber it.
        assert_eq!(load_facts(&root).len(), 1);
        let _ = fs::remove_dir_all(&home);
    }

    /// An update to a fact whose name sits at the bound still lands a
    /// valid replacement key — the serial suffix never overshoots the
    /// name bound.
    #[test]
    fn an_update_to_a_bound_length_name_still_lands() {
        struct Scripted {
            revision: String,
            description: String,
        }
        impl Source for Scripted {
            fn harness(&self) -> &str {
                "scripted"
            }
            fn project(&self) -> &str {
                "demo"
            }
            fn read(&self) -> Result<Snapshot, PullError> {
                Ok(Snapshot {
                    revision: self.revision.clone(),
                    facts: vec![SourceFact {
                        name: "a".repeat(MAX_NAME_CHARS),
                        description: self.description.clone(),
                        fact_type: "project".to_string(),
                        body: "Body.".to_string(),
                    }],
                    skips: Vec::new(),
                })
            }
        }
        let home = scratch("bound-update");
        let root = home.join("memories");
        let memory = corpus(&root);
        let long = "a".repeat(MAX_NAME_CHARS);
        pull(
            &Scripted {
                revision: "content:one".to_string(),
                description: "the first words".to_string(),
            },
            &memory,
            &home,
            None,
        )
        .unwrap();
        let report = pull(
            &Scripted {
                revision: "content:two".to_string(),
                description: "the second words".to_string(),
            },
            &memory,
            &home,
            None,
        )
        .unwrap();
        let updated = report
            .actions
            .iter()
            .find_map(|action| match action {
                ImportAction::Updated { into, .. } => Some(into.clone()),
                _ => None,
            })
            .expect("the update landed");
        assert!(updated.chars().count() <= MAX_NAME_CHARS, "{updated}");
        let facts = load_facts(&root);
        let replacement = facts.iter().find(|fact| fact.name == updated).unwrap();
        assert_eq!(replacement.description, "the second words");
        let old = facts.iter().find(|fact| fact.name == long).unwrap();
        assert_eq!(old.superseded_by, updated);
        let _ = fs::remove_dir_all(&home);
    }

    /// A git-tracked store reads its HEAD as the revision — the cursor's
    /// other basis, the one armada's commits ride.
    #[test]
    fn a_git_store_reads_its_head() {
        let home = scratch("git-revision");
        let dir = home.join("store");
        fs::create_dir_all(&dir).unwrap();
        let git = |args: &[&str]| {
            std::process::Command::new("git")
                .arg("-C")
                .arg(&dir)
                .args(args)
                .output()
                .unwrap()
        };
        if !git(&["init", "-q"]).status.success() {
            // A machine without git degrades to the content hash; the
            // test names the skip rather than failing the gate.
            eprintln!("git is unavailable; the git-revision test skips");
            let _ = fs::remove_dir_all(&home);
            return;
        }
        fs::write(dir.join("fact.md"), "one\n").unwrap();
        git(&["add", "."]);
        git(&[
            "-c",
            "user.name=test",
            "-c",
            "user.email=test@example.com",
            "commit",
            "-qm",
            "one",
        ]);
        let revision = store_revision(&dir);
        assert!(revision.starts_with("git:"), "{revision}");
        let head = String::from_utf8_lossy(&git(&["rev-parse", "HEAD"]).stdout)
            .trim()
            .to_string();
        assert_eq!(revision, format!("git:{head}"));
        let _ = fs::remove_dir_all(&home);
    }
}
