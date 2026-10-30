//! The face's reads (the history instrument, C0): ONE backend read over
//! the LayerFS Store, served to the workspace-history destination.
//!
//! The whole view assembles from a single `backend.store.project.get_history`
//! answer — the branches (with their fork bases), their newest commits,
//! main's Layers, the Store's nine figures and a `version` for
//! incremental refresh — read through the bundle's backend script. The
//! CLI survives only where the backend cannot answer: the Files pane's
//! `fs` reads. Every source degrades by name: a missing backend script
//! is a refusal on the view, never an empty lie.

use std::path::{Path, PathBuf};

use super::layerfs_tools::Registration;
use serde_json::json;

/// One Commit — the fields the answer carries, parsed whole. The id is
/// FULL, never shortened: the face refs it (`fs read <ref>` takes a
/// Commit id) and clips it for display.
#[derive(Debug, Clone, PartialEq)]
pub struct CommitView {
    pub id: String,
    pub branch: String,
    pub parent: Option<String>,
    pub second_parent: Option<String>,
    /// The ref the branch was forked from — a Layer id for a branch
    /// forked from main.
    pub base: Option<String>,
    pub merged_from: Option<String>,
    /// The wire's `displaced` field, carried raw: its shape is the
    /// substrate's, and the face only names it when present.
    pub displaced: Option<serde_json::Value>,
    pub lww: Vec<String>,
    pub lww_truncated: bool,
    pub logical: u64,
    pub new_canonical: u64,
    pub new_stored: u64,
    pub new_metadata: u64,
    pub via: String,
    pub message: Option<String>,
    /// Wall-clock millis, as the wire reports it.
    pub time: i64,
    pub root: Option<String>,
}

/// One published Layer — main's own line, one immutable snapshot per
/// publish.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayerView {
    pub id: String,
    pub logical: u64,
    pub parent: Option<String>,
    pub root: Option<String>,
    /// The branch whose commit made this layer — `merge-<child>` for a
    /// parent's settlement. This is where the merge evidence lives: the
    /// live Store's main carries no merge COMMIT (its head stays null
    /// while the layers chain), so a branch's merged state is read off
    /// the layers that name it.
    pub source_branch: Option<String>,
    /// The Commit the layer was published from — the graft edge the
    /// graph draws from a branch's leaf to its band on main.
    pub source_commit: Option<String>,
    pub overwrote_count: u64,
}

/// One branch, as the answer names it: its fork base (where the graph
/// sprouts the lane) and its head (its newest commit, `None` for a
/// branch nobody has worked).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BranchView {
    pub name: String,
    pub base: Option<String>,
    pub forked_from: Option<String>,
    pub head: Option<String>,
    /// The join from a branch to the delegation that worked it: the
    /// branch's name carries the child's session id (folded), and the
    /// row carries the rest.
    pub child: Option<ChildLink>,
}

/// The join from a branch to a child session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChildLink {
    pub session: String,
    pub outcome: String,
}

/// The Init record — what the import carried and what it cost, recorded
/// once when Init sealed the Store. `None` for a Store built before
/// Init recorded it (named, never invented).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InitStats {
    pub entries: u64,
    pub files: u64,
    pub bytes: u64,
    pub canonical: u64,
    pub stored: u64,
    pub metadata: u64,
    pub millis: u64,
}

/// The Store's accounting — all nine figures the backend reports.
/// `dedupe` and `compression` arrive as numbers or strings depending on
/// the route that reported them, so the parse accepts both and the face
/// computes its ratios from the byte figures.
#[derive(Debug, Clone, PartialEq)]
pub struct StorageView {
    pub logical: u64,
    pub canonical: u64,
    pub stored: u64,
    pub metadata: u64,
    pub file: u64,
    pub free: u64,
    pub dedupe: Option<f64>,
    pub compression: Option<f64>,
    pub init: Option<InitStats>,
}

/// The whole view: the version token, the branches (with their
/// children), the commits, main's Layers, the figures, the branches
/// whose older commits the answer cut, and the note naming what could
/// not be read.
#[derive(Debug, Clone, PartialEq)]
pub struct HistoryView {
    pub version: String,
    pub branches: Vec<BranchView>,
    pub commits: Vec<CommitView>,
    pub layers: Vec<LayerView>,
    pub storage: Option<StorageView>,
    /// Branch names whose commit chains the answer cut (the `more`
    /// cursor names each cut branch): the graph draws their newest
    /// commit, and the truncation line says so.
    pub cut_branches: Vec<String>,
    pub more_branches: bool,
    pub more_layers: bool,
    pub note: Option<String>,
}

/// What one read answered: the full view, or the version token with
/// nothing moved since it (the 1 ms answer the face's refresh rides).
#[derive(Debug, Clone, PartialEq)]
pub enum HistoryRead {
    Full(Box<HistoryView>),
    Unchanged(String),
}

/// The history script's location: the home's own copy (install
/// provisions it), with the override for tests and operators.
fn history_script(home: &Path) -> Result<PathBuf, String> {
    if let Some(path) = std::env::var_os("FLEET_LAYERFS_HISTORY")
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
    {
        return Ok(path);
    }
    let script = home.join(crate::layerfs_tools::HISTORY_SCRIPT);
    if script.is_file() {
        Ok(script)
    } else {
        Err(format!(
            "the history script is not at {} — `fleet install` provisions it",
            script.display()
        ))
    }
}

/// The join from a branch name to a child session: `fleet-child-`
/// followed by the folded session id.
fn child_session_of(branch: &str) -> Option<&str> {
    branch.strip_prefix("fleet-child-")
}

/// Today's delegation rows' `session -> outcome` map — the join's
/// other half, read from the calibration stream's own day file.
fn delegation_outcomes() -> Vec<(String, String)> {
    delegation_rows()
        .into_iter()
        .map(|(session, _branch, outcome)| (session, outcome))
        .collect()
}

/// The day's delegation rows that stood on a workspace, as
/// `(session, branch, outcome)` — the join the face reads in both
/// directions: a branch folds to its child's session, and a session's
/// row names the branch its work stands on.
fn delegation_rows() -> Vec<(String, String, String)> {
    let Ok(home) = crate::home::fleet_home(
        std::env::var("FLEET_HOME").ok().as_deref(),
        std::env::var("HOME").ok().as_deref(),
    ) else {
        return Vec::new();
    };
    let root = crate::calib::resolve_root(&home, std::env::var("HOME").ok().as_deref());
    let day = crate::calib::today();
    // The OP-bounded reader, not the whole-stream tail: a busy day's
    // file is mixed-op, and the last 512 rows of it can hold no
    // delegation row at all — which is exactly how the live face lost
    // its child join on a day the router had been busy.
    crate::calib::read_op_day(&root, &day, crate::calib::OP_DELEGATION, 512)
        .into_iter()
        .filter(|row| row.op == crate::calib::OP_DELEGATION && !row.workspace_branch.is_empty())
        .map(|row| (row.session, row.workspace_branch, row.outcome))
        .collect()
}

/// The session → branch map the delegation panel's rows carry, read
/// from the day's stream. A branch a child worked is the click-through
/// into the workspace-history section, so the panel names it where the
/// owner already looks.
pub fn session_branches() -> std::collections::HashMap<String, String> {
    delegation_rows()
        .into_iter()
        .map(|(session, branch, _)| (session, branch))
        .collect()
}

/// A branch's state, read off the view itself: **merged** when main's
/// trunk carries the layer that names it (the parent's settlement
/// publishes with `merge-<branch>` as its source branch), **open**
/// when the branch holds a head nothing has merged, **idle** when it
/// was forked and never worked. The state is derived, never recorded:
/// one writer for the fact (the substrate), one reader for the face.
pub fn branch_state(view: &HistoryView, branch: &str) -> &'static str {
    let settled = format!("merge-{branch}");
    let merged = view.layers.iter().any(|layer| {
        layer
            .source_branch
            .as_deref()
            .is_some_and(|source| source == branch || source == settled)
    });
    if merged {
        return "merged";
    }
    // A head is the branch's newest commit: some is work unmerged.
    let worked = view
        .branches
        .iter()
        .find(|one| one.name == branch)
        .is_some_and(|one| one.head.is_some());
    if worked {
        "open"
    } else {
        "idle"
    }
}

/// The view, assembled from ONE backend call — the script's whole
/// `get_history` answer, parsed without dropping a field the graph
/// draws. A failed script is a named refusal, never an empty view.
pub fn history(
    registration: &Registration,
    home: &Path,
    since: Option<&str>,
) -> Result<HistoryRead, String> {
    // The composed environment, the same one the arm's spawns set:
    // the face runs in whatever env it was launched in (a clean one,
    // live), so the machine's facts are composed here rather than
    // assumed — home, endpoint, image, daemon, and the project's own
    // store path and volume.
    let machine = crate::layerfs_tools::read_machine();
    if let Ok(machine) = &machine {
        crate::layerfs_tools::apply_env(machine, Some(registration));
    } else {
        std::env::set_var("LAYERFS_STORE_PATH", &registration.store);
    }

    let args: Vec<&str> = since.into_iter().collect();
    let document = backend_answer(home, registration, &args)?;
    // The unchanged answer: the version token and nothing else — the
    // caller's view stands.
    if document.get("kind").and_then(|kind| kind.as_str()) == Some("unchanged") {
        return Ok(HistoryRead::Unchanged(
            document
                .get("version")
                .and_then(|version| version.as_str())
                .unwrap_or_default()
                .to_string(),
        ));
    }
    let version = document
        .get("version")
        .and_then(|version| version.as_str())
        .unwrap_or_default()
        .to_string();

    let branches = document
        .get("branches")
        .and_then(serde_json::Value::as_array)
        .map(|branches| {
            branches
                .iter()
                .map(|branch| BranchView {
                    name: str_at(branch, "name").unwrap_or_default(),
                    base: str_at(branch, "base"),
                    forked_from: str_at(branch, "forkedFrom"),
                    head: str_at(branch, "head"),
                    child: None,
                })
                .collect::<Vec<BranchView>>()
        })
        .unwrap_or_default();
    let commits = document
        .get("commits")
        .and_then(serde_json::Value::as_array)
        .map(|commits| {
            commits
                .iter()
                .map(parse_commit)
                .collect::<Vec<CommitView>>()
        })
        .unwrap_or_default();
    let layers = document
        .get("layers")
        .and_then(serde_json::Value::as_array)
        .map(|layers| {
            layers
                .iter()
                .map(|layer| LayerView {
                    id: str_at(layer, "id").unwrap_or_default(),
                    logical: u64_at(layer, "logical"),
                    parent: str_at(layer, "parent"),
                    root: str_at(layer, "root"),
                    source_branch: str_at(layer, "sourceBranch"),
                    source_commit: str_at(layer, "sourceCommit"),
                    overwrote_count: u64_at(layer, "overwroteCount"),
                })
                .collect::<Vec<LayerView>>()
        })
        .unwrap_or_default();
    let storage = document
        .get("store")
        .and_then(|store| store.get("stats"))
        .map(parse_storage);
    // The `more` cursor names each branch whose commit chain the answer
    // cut: the graph draws their newest commit and the truncation line
    // says so.
    let cut_branches = document
        .get("more")
        .and_then(serde_json::Value::as_object)
        .map(|more| more.keys().cloned().collect())
        .unwrap_or_default();

    // The child join: the day's delegation rows, folded against the
    // branch names the answer carried.
    let outcomes = delegation_outcomes();
    let mut branches = branches;
    for branch in &mut branches {
        if let Some(folded) = child_session_of(&branch.name) {
            branch.child = outcomes
                .iter()
                .find(|row| row.0.to_lowercase() == folded)
                .map(|row| ChildLink {
                    session: row.0.clone(),
                    outcome: row.1.clone(),
                });
        }
    }

    Ok(HistoryRead::Full(Box::new(HistoryView {
        version,
        branches,
        commits,
        layers,
        storage,
        cut_branches,
        more_branches: document
            .get("moreBranches")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false),
        more_layers: document
            .get("moreLayers")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false),
        note: None,
    })))
}

/// One commit row, from the answer's own shape — every field the graph
/// and the accounting draw, parsed.
fn parse_commit(commit: &serde_json::Value) -> CommitView {
    CommitView {
        id: str_at(commit, "id").unwrap_or_default(),
        branch: str_at(commit, "branch").unwrap_or_default(),
        parent: str_at(commit, "parent"),
        second_parent: str_at(commit, "secondParent"),
        base: str_at(commit, "base"),
        merged_from: str_at(commit, "mergedFrom"),
        displaced: commit
            .get("displaced")
            .filter(|value| !value.is_null())
            .cloned(),
        lww: commit
            .get("lww")
            .and_then(serde_json::Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| match item {
                        // The wire's entries are objects
                        // (`{path, kept, deleted, rule}`); the bare
                        // strings are the older shape, kept for a
                        // reader of either.
                        serde_json::Value::String(text) => Some(text.clone()),
                        serde_json::Value::Object(entry) => Some(lww_line(entry)),
                        _ => None,
                    })
                    .collect()
            })
            .unwrap_or_default(),
        lww_truncated: commit
            .get("lwwTruncated")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false),
        logical: u64_at(commit, "logical"),
        new_canonical: u64_at(commit, "newCanonical"),
        new_stored: u64_at(commit, "newStored"),
        new_metadata: u64_at(commit, "newMetadata"),
        via: str_at(commit, "via").unwrap_or_default(),
        message: str_at(commit, "message"),
        time: commit
            .get("time")
            .and_then(|time| time.as_i64())
            .unwrap_or(0),
        root: str_at(commit, "root"),
    }
}

/// The Store's nine figures. `dedupe` and `compression` arrive as
/// numbers on one route and strings on another (measured live), so the
/// parse accepts both and hands the face numbers it can divide.
fn parse_storage(stats: &serde_json::Value) -> StorageView {
    let number = |key: &str| -> Option<f64> {
        stats.get(key).and_then(|value| match value {
            serde_json::Value::Number(number) => number.as_f64(),
            serde_json::Value::String(text) => text.parse::<f64>().ok(),
            _ => None,
        })
    };
    StorageView {
        logical: u64_at(stats, "logical"),
        canonical: u64_at(stats, "canonical"),
        stored: u64_at(stats, "stored"),
        metadata: u64_at(stats, "metadata"),
        file: u64_at(stats, "file"),
        free: u64_at(stats, "free"),
        dedupe: number("dedupe"),
        compression: number("compression"),
        init: stats
            .get("init")
            .filter(|init| !init.is_null())
            .map(|init| InitStats {
                entries: u64_at(init, "entries"),
                files: u64_at(init, "files"),
                bytes: u64_at(init, "bytes"),
                canonical: u64_at(init, "canonical"),
                stored: u64_at(init, "stored"),
                metadata: u64_at(init, "metadata"),
                millis: u64_at(init, "millis"),
            }),
    }
}

fn str_at(value: &serde_json::Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(serde_json::Value::as_str)
        .filter(|text| !text.is_empty())
        .map(str::to_string)
}

fn u64_at(value: &serde_json::Value, key: &str) -> u64 {
    value
        .get(key)
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0)
}

/// A script's stderr, bounded the way the driver's answers are.
fn bounded_stderr(stderr: &[u8]) -> String {
    let text = String::from_utf8_lossy(stderr);
    let mut cut = String::new();
    for (index, ch) in text.trim().char_indices() {
        if index >= 400 {
            cut.push_str(" \u{2026}");
            return cut;
        }
        cut.push(ch);
    }
    cut
}

/// The launch folder's registration, resolved fresh from the config
/// (the process cache answers whichever config the first caller saw,
/// which a face that serves several folders cannot afford). `None` for
/// a folder no delegation has registered yet.
fn registered_folder(cwd: &Path) -> Option<(String, Registration)> {
    let canonical = cwd.canonicalize().unwrap_or_else(|_| cwd.to_path_buf());
    crate::layerfs_tools::read_registrations()
        .into_iter()
        .find(|(folder, _)| Path::new(folder) == canonical.as_path())
}

/// The truncation, in words: what the answer carried and what it cut.
/// The face prints it where the tree would be — the cut is a named
/// fact, never a silent one.
fn truncation_word(view: &HistoryView) -> Option<String> {
    let branches = view.branches.len();
    let mut said = Vec::new();
    if view.more_branches {
        said.push(format!(
            "{branches} branches shown, more remain in the Store"
        ));
    }
    if view.more_layers {
        said.push(format!("{} layers shown, more remain", view.layers.len()));
    }
    if !view.cut_branches.is_empty() {
        said.push(format!(
            "older commits cut on {}",
            view.cut_branches.join(", ")
        ));
    }
    if said.is_empty() {
        None
    } else {
        Some(said.join(" \u{00b7} "))
    }
}

/// The section's document: the folder's own view, with each branch's
/// state derived and the substrate's absences named rather than drawn
/// as an empty tree. The GUI's launch folder is the project — the face
/// reads the machine, it never registers anything (that is the arm's
/// one-time job at a delegation's first spawn). A `since` version the
/// substrate still holds current answers `unchanged` and the caller's
/// view stands.
pub fn workspace_history(cwd: &Path, since: Option<&str>) -> serde_json::Value {
    workspace_history_for(cwd, since)
}

/// The section's document for one folder, made explicit — the rail's
/// per-project read. The launch-folder path delegates here.
pub fn workspace_history_for(folder: &Path, since: Option<&str>) -> serde_json::Value {
    let Ok(home) = crate::home::fleet_home(
        std::env::var("FLEET_HOME").ok().as_deref(),
        std::env::var("HOME").ok().as_deref(),
    ) else {
        return json!({ "type": "workspace-history", "refused": ["no fleet home is known"] });
    };
    let Some((folder, registration)) = registered_folder(folder) else {
        return json!({
            "type": "workspace-history",
            "registered": false,
            "folder": folder.display().to_string(),
            "branches": [],
            "layers": [],
            "refused": [],
            "note": "this folder has no LayerFS project — the first delegation here registers one",
        });
    };
    project_document(&folder, &registration, &home, since)
}

/// One project's section document, by its registered folder's path —
/// the rail's per-project read for a face that serves every registered
/// folder. The path is the project's identity the way the control
/// tower and the second brain resolve projects: the registration map's
/// own key, canonicalized the same way the arm's lookup is. A path no
/// registration carries is refused by name.
pub fn workspace_history_for_registered(folder: &str, since: Option<&str>) -> serde_json::Value {
    let Ok(home) = crate::home::fleet_home(
        std::env::var("FLEET_HOME").ok().as_deref(),
        std::env::var("HOME").ok().as_deref(),
    ) else {
        return json!({ "type": "workspace-history", "refused": ["no fleet home is known"] });
    };
    let canonical = std::path::Path::new(folder)
        .canonicalize()
        .unwrap_or_else(|_| std::path::PathBuf::from(folder));
    let Some((registered, registration)) = crate::layerfs_tools::read_registrations()
        .into_iter()
        .find(|(path, _)| std::path::Path::new(path) == canonical.as_path())
    else {
        return json!({
            "type": "workspace-history",
            "registered": false,
            "folder": folder,
            "branches": [],
            "layers": [],
            "refused": [],
            "note": format!("no registered project at `{folder}`"),
        });
    };
    project_document(&registered, &registration, &home, since)
}

/// The read for one resolved registration, rendered either way it can
/// answer — the full view, the `unchanged` version stamp, or the named
/// refusal. One body, three callers.
fn project_document(
    folder: &str,
    registration: &Registration,
    home: &Path,
    since: Option<&str>,
) -> serde_json::Value {
    match history(registration, home, since) {
        Ok(HistoryRead::Unchanged(version)) => json!({
            "type": "workspace-history",
            "unchanged": true,
            "version": version,
        }),
        Ok(HistoryRead::Full(view)) => {
            let branches: Vec<serde_json::Value> = view
                .branches
                .iter()
                .map(|branch| {
                    json!({
                        "name": branch.name,
                        "base": branch.base,
                        "forked_from": branch.forked_from,
                        "head": branch.head,
                        "state": branch_state(&view, &branch.name),
                        "child": branch.child.as_ref().map(|child| json!({
                            "session": child.session,
                            "outcome": child.outcome,
                        })),
                    })
                })
                .collect();
            let commits: Vec<serde_json::Value> = view
                .commits
                .iter()
                .map(|commit| {
                    json!({
                        "id": commit.id,
                        "branch": commit.branch,
                        "parent": commit.parent,
                        "second_parent": commit.second_parent,
                        "base": commit.base,
                        "merged_from": commit.merged_from,
                        "displaced": commit.displaced,
                        "lww": commit.lww,
                        "lww_truncated": commit.lww_truncated,
                        "logical": commit.logical,
                        "new_canonical": commit.new_canonical,
                        "new_stored": commit.new_stored,
                        "new_metadata": commit.new_metadata,
                        "via": commit.via,
                        "message": commit.message,
                        "time": commit.time,
                        "root": commit.root,
                    })
                })
                .collect();
            json!({
                "type": "workspace-history",
                "registered": true,
                "folder": folder,
                "project": registration.name,
                "version": view.version,
                "branches": branches,
                "commits": commits,
                "layers": view.layers.iter().map(|layer| json!({
                    "id": layer.id,
                    "logical": layer.logical,
                    "parent": layer.parent,
                    "root": layer.root,
                    "source_branch": layer.source_branch,
                    "source_commit": layer.source_commit,
                    "overwrote_count": layer.overwrote_count,
                })).collect::<Vec<_>>(),
                "storage": view.storage.as_ref().map(|storage| json!({
                    "logical": storage.logical,
                    "canonical": storage.canonical,
                    "stored": storage.stored,
                    "metadata": storage.metadata,
                    "file": storage.file,
                    "free": storage.free,
                    "dedupe": storage.dedupe,
                    "compression": storage.compression,
                    "init": storage.init.as_ref().map(|init| json!({
                        "entries": init.entries,
                        "files": init.files,
                        "bytes": init.bytes,
                        "canonical": init.canonical,
                        "stored": init.stored,
                        "metadata": init.metadata,
                        "millis": init.millis,
                    })),
                })),
                "cut_branches": view.cut_branches,
                "truncated": truncation_word(&view),
                "note": view.note,
                "refused": [],
            })
        }
        Err(reason) => json!({
            "type": "workspace-history",
            "registered": true,
            "project": registration.name,
            "branches": [],
            "layers": [],
            "refused": [reason],
        }),
    }
}

/// The activity window the owner set, in days: a project whose newest
/// commit is older than this is absent from the rail — the rule is the
/// owner's own words, fixed, not a knob.
pub const ACTIVITY_WINDOW_DAYS: i64 = 42;

/// The rail document: every registered folder read once, named by its
/// registration, ordered newest-activity first. A project whose newest
/// commit falls inside [`ACTIVITY_WINDOW_DAYS`] carries its full view;
/// one outside is absent — the owner's filter is the rule. A project
/// whose read died names the substrate's own refusal in
/// `refused_projects`, because a broken read is not inactivity and an
/// unexplained absence is the one shape this surface never draws. A
/// project with no commits at all has no activity to measure and is
/// absent with the rest.
pub fn workspace_projects() -> serde_json::Value {
    let Ok(home) = crate::home::fleet_home(
        std::env::var("FLEET_HOME").ok().as_deref(),
        std::env::var("HOME").ok().as_deref(),
    ) else {
        return json!({ "type": "workspace-history-rail", "refused": ["no fleet home is known"] });
    };
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|since| since.as_millis() as i64)
        .unwrap_or(0);
    let window_ms = ACTIVITY_WINDOW_DAYS * 24 * 60 * 60 * 1000;
    let mut projects: Vec<(i64, serde_json::Value)> = Vec::new();
    let mut refused_projects: Vec<serde_json::Value> = Vec::new();
    for (folder, registration) in crate::layerfs_tools::read_registrations() {
        match history(&registration, &home, None) {
            Ok(HistoryRead::Full(view)) => {
                let newest = view.commits.iter().map(|commit| commit.time).max();
                // The activity measure is two-sided: the newest
                // commit, and — for a store that has never committed —
                // the creation the substrate itself recorded in the
                // project's runtime record. The import IS the store's
                // first activity (the fleet store restarted empty when
                // its volume was re-created, and the project's origin
                // is exactly that event); the record's `created` keeps
                // a freshly-registered project visible until real work
                // or the window retires it. A project neither dated
                // falls back to its commits alone — absent.
                let birth = crate::layerfs_tools::runtime_created(&registration.name);
                let active = match (newest, birth) {
                    (Some(time), Some(at)) => now_ms - time.max(at) <= window_ms,
                    (Some(time), None) => now_ms - time <= window_ms,
                    (None, Some(at)) => now_ms - at <= window_ms,
                    (None, None) => false,
                };
                if !active {
                    continue;
                }
                let document = project_document(&folder, &registration, &home, None);
                let summary = json!({
                    "name": registration.name,
                    "folder": folder,
                    "active": true,
                    "newest": newest,
                    "branches": document["branches"].as_array().map(Vec::len).unwrap_or(0),
                    "commits": document["commits"].as_array().map(Vec::len).unwrap_or(0),
                    "layers": document["layers"].as_array().map(Vec::len).unwrap_or(0),
                    "stored": document["storage"]["stored"],
                    "truncated": document["truncated"],
                    "view": document,
                });
                projects.push((newest.unwrap_or(0), summary));
            }
            // A version stamp never reaches the rail: the rail reads
            // whole, once per window.
            Ok(HistoryRead::Unchanged(_)) => {}
            Err(reason) => refused_projects.push(json!({
                "name": registration.name,
                "folder": folder,
                "refused": [reason],
            })),
        }
    }
    projects.sort_by(|left, right| right.0.cmp(&left.0));
    json!({
        "type": "workspace-history-rail",
        "window_days": ACTIVITY_WINDOW_DAYS,
        "projects": projects.into_iter().map(|(_, summary)| summary).collect::<Vec<_>>(),
        "refused_projects": refused_projects,
        "refused": [],
    })
}

/// The live feed's tail state: per-file read offsets and the partial
/// trailing line each file held back, so a poll answers only what moved
/// and never a torn line.
#[derive(Default)]
pub struct TraceTail {
    offsets: std::collections::HashMap<PathBuf, (u64, String)>,
}

/// The workspace-calls read: the day's substrate activity, tailed from
/// `<layerfs-home>/traces/*.jsonl` — one JSON line per agent and
/// backend call, the observe contract's own format. The actor is the
/// trace file's name (one per agent; the arm names each child's file
/// for its session). Filters: the launch folder's project, and `since`
/// — an ISO start time; the comparison is lexicographic on the ISO
/// form the substrate writes, which orders correctly for one format.
/// Newest last, capped at 200 lines with `more: true` when older lines
/// were dropped. A machine without traces names the absence — "traces
/// are off" — never "no activity".
pub fn workspace_calls(cwd: &Path, since: Option<&str>, tail: &mut TraceTail) -> serde_json::Value {
    let machine = match crate::layerfs_tools::read_machine() {
        Ok(machine) => machine,
        Err(reason) => {
            return json!({
                "type": "workspace-calls",
                "refused": [format!("the substrate is not provisioned: {reason}")],
            })
        }
    };
    let project = registered_folder(cwd).map(|(name, _)| name);
    let traces = machine.home.join("traces");
    let Ok(entries) = std::fs::read_dir(&traces) else {
        return json!({
            "type": "workspace-calls",
            "calls": [],
            "more": false,
            "note": "traces are off — no LayerFS process on this machine has LAYERFS_TRACE set, so there is nothing live to show",
        });
    };
    let mut files: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "jsonl")
        })
        .collect();
    files.sort();
    let mut calls: Vec<serde_json::Value> = Vec::new();
    for path in files {
        let Ok(meta) = std::fs::metadata(&path) else {
            continue;
        };
        let size = meta.len();
        let entry = tail.offsets.entry(path.clone()).or_default();
        let mut offset = entry.0;
        let mut rest = entry.1.clone();
        // A file that shrank was truncated or replaced: start over.
        if size < offset {
            offset = 0;
            rest.clear();
        }
        // A file rewritten under us (grew, but its content moved): the
        // held rest must still match the bytes just before the offset —
        // a mismatch is a wholesale change, and the only honest answer
        // is a full re-read.
        if offset > 0 && !rest.is_empty() {
            use std::io::{Read, Seek, SeekFrom};
            let mut probe = vec![0u8; rest.len()];
            let rewritten = std::fs::File::open(&path).is_ok_and(|mut file| {
                file.seek(SeekFrom::Start(offset - rest.len() as u64))
                    .is_ok()
                    && file.read_exact(&mut probe).is_ok()
                    && String::from_utf8_lossy(&probe) != rest
            });
            if rewritten {
                offset = 0;
                rest.clear();
            }
        }
        let mut fresh = Vec::new();
        {
            use std::io::{Read, Seek, SeekFrom};
            let Ok(mut file) = std::fs::File::open(&path) else {
                continue;
            };
            if offset > 0 && file.seek(SeekFrom::Start(offset)).is_err() {
                continue;
            }
            if file.read_to_end(&mut fresh).is_err() {
                continue;
            }
        }
        offset += fresh.len() as u64;
        let actor = path
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
            .unwrap_or_default();
        let text = rest + &String::from_utf8_lossy(&fresh);
        let mut lines = text.split('\n').collect::<Vec<_>>();
        // The trailing fragment is held until its newline arrives —
        // a torn line is never answered.
        rest = lines.pop().unwrap_or_default().to_string();
        entry.0 = offset;
        entry.1 = rest;
        for line in lines {
            if line.trim().is_empty() {
                continue;
            }
            let Ok(event) = serde_json::from_str::<serde_json::Value>(line) else {
                continue;
            };
            if event
                .get("layerfs_trace")
                .and_then(serde_json::Value::as_i64)
                != Some(1)
            {
                continue;
            }
            if let Some(line_project) = event.get("project").and_then(serde_json::Value::as_str) {
                if let Some(watched) = &project {
                    if line_project != watched {
                        continue;
                    }
                }
            }
            let start = event
                .get("start")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_string();
            if let Some(since) = since {
                if !since.is_empty() && start.as_str() < since {
                    continue;
                }
            }
            calls.push(json!({
                "actor": actor,
                "method": event.get("method").cloned().unwrap_or(json!(null)),
                "args": event.get("args").cloned().unwrap_or(json!(null)),
                "start": start,
                "ms": event.get("ms").cloned().unwrap_or(json!(null)),
                "ok": event.get("ok").cloned().unwrap_or(json!(null)),
                "kind": event.get("kind").cloned().unwrap_or(json!(null)),
                "commit": event.get("commit").cloned().unwrap_or(json!(null)),
                "sizes": event.get("sizes").cloned().unwrap_or(json!(null)),
            }));
        }
    }
    // The feed is a timeline, whatever order the files were read in:
    // sort by the ISO start, keep the newest 200, name the drop.
    fn start_of(call: &serde_json::Value) -> &str {
        call.get("start")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
    }
    calls.sort_by(|a, b| start_of(a).cmp(start_of(b)));
    let total = calls.len();
    let more = total > 200;
    calls.reverse();
    calls.truncate(200);
    calls.reverse();
    json!({
        "type": "workspace-calls",
        "calls": calls,
        "more": more,
        "note": if more {
            json!(format!("{total} calls in the window; the oldest {} dropped", total - 200))
        } else {
            json!(null)
        },
        "refused": [],
    })
}

/// One answer from the backend script: the machine's facts composed,
/// the home's script run with the given extra arguments, and its one
/// JSON line parsed. A failed script is a named reason, never an
/// empty answer.
fn backend_answer(
    home: &Path,
    registration: &Registration,
    args: &[&str],
) -> Result<serde_json::Value, String> {
    let machine = crate::layerfs_tools::read_machine();
    if let Ok(machine) = &machine {
        crate::layerfs_tools::apply_env(machine, Some(registration));
    } else {
        std::env::set_var("LAYERFS_STORE_PATH", &registration.store);
    }
    // The scripts live in the LAYERFS home, which is not the fleet
    // home the caller handed us — the machine's own record says where,
    // and a machine without facts falls back to the caller's.
    let layerfs_home = machine
        .as_ref()
        .map(|machine| machine.home.clone())
        .unwrap_or_else(|_| home.to_path_buf());
    // A worker the home records but nothing answers is named before the
    // script runs: the node stack's first line blames the script, and
    // the cause is the worker — a Docker reset takes the container and
    // leaves the endpoint file pointing at its dead port.
    if let Some(gap) = worker_gap(&layerfs_home) {
        return Err(gap);
    }
    let script = history_script(&layerfs_home)?;
    // The bundle is the machine's own record, never the process env: a
    // face launched in a clean environment has no LAYERFS_* facts, and
    // the env knob stays the operator's override on top of it.
    let bundle = std::env::var("FLEET_LAYERFS_BUNDLE")
        .ok()
        .filter(|bundle| !bundle.is_empty())
        .map(std::path::PathBuf::from)
        .or_else(|| machine.as_ref().ok().map(|machine| machine.bundle.clone()))
        .unwrap_or_default();
    let mut command = std::process::Command::new("node");
    command.args([
        script.to_string_lossy().as_ref(),
        &registration.name,
        bundle.to_string_lossy().as_ref(),
    ]);
    command.args(args);
    let output = command
        .output()
        .map_err(|error| format!("the history script could not run: {error}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // A recognized substrate failure is named first — the panel is
        // told what to do — with the script's own words kept after it
        // as evidence, bounded like every other read's.
        return Err(match named_refusal(&stderr) {
            Some(cause) => format!(
                "{cause} \u{2014} the script said: {}",
                bounded_stderr(&output.stderr)
            ),
            None => format!("the script failed: {}", bounded_stderr(&output.stderr)),
        });
    }
    let document: serde_json::Value = serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("the script's answer was not JSON: {error}"))?;
    if let Some(error) = document.get("error").and_then(|error| error.as_str()) {
        return Err(error.to_string());
    }
    Ok(document)
}

/// One last-writer-wins entry as the hover card reads it. The substrate
/// emits objects (`{path, kept, deleted, rule}`) — the old parse took
/// strings only, so every real entry was dropped and the field looked
/// parsed while always empty.
fn lww_line(entry: &serde_json::Map<String, serde_json::Value>) -> String {
    let text = |key: &str| {
        entry
            .get(key)
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
    };
    let mut said = if text("path").is_empty() {
        "(an unnamed path)".to_string()
    } else {
        text("path").to_string()
    };
    if !text("rule").is_empty() {
        said.push_str(" \u{00b7} ");
        said.push_str(text("rule"));
    }
    if !text("deleted").is_empty() {
        said.push_str(" \u{2014} dropped ");
        said.push_str(text("deleted"));
    }
    if said.chars().count() > 120 {
        said = said.chars().take(117).collect::<String>() + "\u{2026}";
    }
    said
}

/// The worker the reads ride, when the home records one and nothing
/// answers on its port. A home that never provisioned a worker has no
/// endpoint file and is not told to start one — the script's own
/// failure is its answer there.
fn worker_gap(layerfs_home: &Path) -> Option<String> {
    let text = std::fs::read_to_string(layerfs_home.join("worker").join("endpoint.json")).ok()?;
    let port = serde_json::from_str::<serde_json::Value>(&text)
        .ok()?
        .get("port")?
        .as_u64()?;
    let port = u16::try_from(port).ok()?;
    let address = std::net::SocketAddr::from(([127, 0, 0, 1], port));
    match std::net::TcpStream::connect_timeout(&address, std::time::Duration::from_millis(300)) {
        Ok(_) => None,
        Err(_) => Some(format!(
            "the LayerFS worker is not running (nothing answers on {address}) \u{2014} start it with `fleet install`"
        )),
    }
}

/// A substrate failure translated to its named cause — the two a Docker
/// reset produces (the worker gone, then the Store volume gone) named
/// in the panel's language instead of an uncaught node stack. `None`
/// for anything unrecognized: the caller prints it verbatim rather than
/// guessing.
fn named_refusal(stderr: &str) -> Option<String> {
    if stderr.contains("the LayerFS worker did not answer") {
        return Some(
            "the LayerFS worker is not running \u{2014} start it with `fleet install`".to_string(),
        );
    }
    if stderr.contains("cannot be examined") && stderr.contains("ENOENT") {
        return Some(
            "the project's LayerFS Store volume is missing (a Docker reset removes it) \u{2014} re-register the folder; its history restarts empty"
                .to_string(),
        );
    }
    None
}

/// The instrument's file reads: one route, four modes — text (a
/// windowed read; a binary is named, never rendered), blame (every
/// line with the Commit that last changed it), revisions (the file's
/// revisions, newest first), and a node's diff (the changed set
/// between two refs, or one changed file's hunks). Caps are named when
/// they bite: 400 lines / 256 KB of text per window, 2,000 blame
/// lines, 50 revisions per page.
pub fn workspace_file(
    cwd: &Path,
    ref_: &str,
    path: &str,
    mode: &str,
    from: Option<&str>,
    to: Option<&str>,
    page: u32,
) -> serde_json::Value {
    let Some((_, registration)) = registered_folder(cwd) else {
        return json!({
            "type": "workspace-file",
            "refused": ["this folder has no LayerFS project"],
        });
    };
    let Ok(home) = crate::home::fleet_home(
        std::env::var("FLEET_HOME").ok().as_deref(),
        std::env::var("HOME").ok().as_deref(),
    ) else {
        return json!({ "type": "workspace-file", "refused": ["no fleet home is known"] });
    };
    let gate = match crate::layerfs_tools::gate() {
        Ok(gate) => gate,
        Err(reason) => return json!({ "type": "workspace-file", "refused": [reason.clone()] }),
    };
    let refused = |reason: String| {
        json!({
            "type": "workspace-file",
            "project": registration.name,
            "ref": ref_,
            "path": path,
            "mode": mode,
            "refused": [reason],
        })
    };
    // The entry's kind and size, read where the content is read: a
    // folder is not a file, and a binary is named, never rendered.
    // Revisions and the diff stand on the ref itself, not an entry.
    let entry = |path: &str| -> Result<(u64, bool), String> {
        gate.fs_stat(&registration.name, ref_, path)
            .map_err(|error| error.to_string())
            .map(|document| {
                (
                    document
                        .get("size")
                        .and_then(serde_json::Value::as_u64)
                        .unwrap_or(0),
                    document
                        .get("type")
                        .and_then(serde_json::Value::as_str)
                        .is_some_and(|kind| kind == "file"),
                )
            })
    };
    match mode {
        "blame" => match entry(path) {
            Err(reason) => refused(reason),
            Ok((_, false)) => refused(format!("{path} is not a file on {ref_}")),
            Ok((size, true)) => match gate.fs_blame(&registration.name, ref_, path, 0) {
                Ok(document) => {
                    let lines = document
                        .get("lines")
                        .and_then(serde_json::Value::as_array)
                        .cloned()
                        .unwrap_or_default();
                    let total = lines.len();
                    let cut = total > 2_000;
                    let kept: Vec<serde_json::Value> = lines.into_iter().take(2_000).collect();
                    json!({
                        "type": "workspace-file",
                        "mode": "blame",
                        "ref": ref_,
                        "path": path,
                        "size": size,
                        "lines": kept,
                        "note": if cut {
                            json!(format!("showing the first 2,000 of {total} lines"))
                        } else {
                            json!(null)
                        },
                        "refused": [],
                    })
                }
                Err(error) => refused(error.to_string()),
            },
        },
        "revisions" => {
            let offset = page.to_string();
            let size_text = "50".to_string();
            match backend_answer(
                &home,
                &registration,
                &["--revisions", ref_, path, &size_text, &offset],
            ) {
                Ok(document) => json!({
                    "type": "workspace-file",
                    "mode": "revisions",
                    "ref": ref_,
                    "path": path,
                    "items": document.get("items").cloned().unwrap_or(json!([])),
                    "has_more": document.get("hasMore").and_then(serde_json::Value::as_bool).unwrap_or(false),
                    "refused": [],
                }),
                Err(reason) => refused(reason),
            }
        }
        "diff" => {
            let Some(from) = from.filter(|from| !from.is_empty()) else {
                return refused("a diff needs the ref it stands on".to_string());
            };
            let Some(to) = to.filter(|to| !to.is_empty()) else {
                return refused("a diff needs the ref it compares".to_string());
            };
            // The wire diffs Commits. A Layer is resolved through the
            // backend's own comparable mapping; a Layer with no Commit
            // of the same files (the import's own, or a publish that
            // merged) is the named NotComparable refusal.
            let resolve = |which: &str, id: &str| -> Result<String, String> {
                backend_answer(&home, &registration, &["--comparable", id]).and_then(|answer| {
                    match answer.get("code").and_then(serde_json::Value::as_str) {
                        Some(code) => Err(format!(
                            "{which} {id} cannot ride a diff: [{code}] {}",
                            answer
                                .get("message")
                                .and_then(serde_json::Value::as_str)
                                .unwrap_or_default()
                        )),
                        None => answer
                            .get("commit")
                            .and_then(serde_json::Value::as_str)
                            .map(str::to_string)
                            .ok_or_else(|| format!("{which} {id} resolved to nothing")),
                    }
                })
            };
            let left = match resolve("the left ref", from) {
                Ok(commit) => commit,
                Err(reason) => return refused(reason),
            };
            let right = match resolve("the right ref", to) {
                Ok(commit) => commit,
                Err(reason) => return refused(reason),
            };
            // One changed file's hunks, when the pane names a path;
            // the changed set otherwise.
            let result = if path.is_empty() {
                gate.diff_commits(&registration.name, &left, &right, true)
                    .map(|document| {
                        json!({
                            "type": "workspace-file",
                            "mode": "diff",
                            "ref": ref_,
                            "path": path,
                            "from": from,
                            "to": to,
                            "from_commit": left,
                            "to_commit": right,
                            "items": document.get("items").cloned().unwrap_or(json!([])),
                            "hunks": false,
                            "refused": [],
                        })
                    })
            } else {
                gate.fs_diff_files(&registration.name, &left, path, &right, path)
                    .map(|document| {
                        json!({
                            "type": "workspace-file",
                            "mode": "diff",
                            "ref": ref_,
                            "path": path,
                            "from": from,
                            "to": to,
                            "from_commit": left,
                            "to_commit": right,
                            "hunks": true,
                            "changes": document.get("changes").cloned()
                                .or_else(|| document.get("hunks").cloned())
                                .unwrap_or(json!([])),
                            "left": document.get("left").cloned(),
                            "right": document.get("right").cloned(),
                            "refused": [],
                        })
                    })
            };
            match result {
                Ok(document) => document,
                Err(error) => refused(error.to_string()),
            }
        }
        _ => match entry(path) {
            Err(reason) => refused(reason),
            Ok((_, false)) => refused(format!("{path} is not a file on {ref_}")),
            Ok((size, true)) => {
                // The text mode: one 400-line window at a time, cut at
                // 256 KB, with the binary named rather than rendered.
                let start = page * 400 + 1;
                let end = start + 399;
                match gate.fs_read_window(&registration.name, ref_, path, start, end) {
                    Ok(document) => {
                        let text = document
                            .get("text")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or_default()
                            .to_string();
                        if text.contains('\u{0}') {
                            return json!({
                                "type": "workspace-file",
                                "mode": "text",
                                "ref": ref_,
                                "path": path,
                                "size": size,
                                "binary": true,
                                "note": format!("binary, {size} bytes"),
                                "refused": [],
                            });
                        }
                        let mut text = text;
                        let mut cut = None;
                        if text.len() > 256 * 1024 {
                            let mut at = 256 * 1024;
                            while !text.is_char_boundary(at) {
                                at -= 1;
                            }
                            cut = Some(format!("cut at 256 KB; {} bytes held", at));
                            text.truncate(at);
                        }
                        json!({
                            "type": "workspace-file",
                            "mode": "text",
                            "ref": ref_,
                            "path": path,
                            "size": size,
                            "text": text,
                            "start_line": document.get("startLine").and_then(serde_json::Value::as_u64).unwrap_or(1),
                            "end_line": document.get("endLine").and_then(serde_json::Value::as_u64).unwrap_or(0),
                            "next_line": document.get("nextLine").and_then(serde_json::Value::as_u64),
                            "note": cut.map(|said| json!(said)).unwrap_or(json!(null)),
                            "refused": [],
                        })
                    }
                    Err(error) => refused(error.to_string()),
                }
            }
        },
    }
}

/// The Files pane's one page of a branch's tree at a path — the CLI's
/// own listing, shaped for the browser. Every absence (an unregistered
/// folder, a CLI that cannot answer, a path the branch does not hold)
/// is a named reason, never an empty list that reads as "nothing here".
pub fn workspace_files(cwd: &Path, branch: &str, path: &str, page: u32) -> serde_json::Value {
    let Some((_, registration)) = registered_folder(cwd) else {
        return json!({
            "type": "workspace-files",
            "refused": ["this folder has no LayerFS project"],
        });
    };
    let gate = match crate::layerfs_tools::gate() {
        Ok(gate) => gate,
        Err(reason) => return json!({ "type": "workspace-files", "refused": [reason.clone()] }),
    };
    match gate.fs_list_page(&registration.name, branch, path, 200, page) {
        Ok(document) => json!({
            "type": "workspace-files",
            "project": registration.name,
            "branch": branch,
            "path": path,
            "page": page,
            "items": document.get("items").cloned().unwrap_or(json!([])),
            "has_more": document.get("hasMore").and_then(serde_json::Value::as_bool).unwrap_or(false),
        }),
        Err(error) => json!({
            "type": "workspace-files",
            "project": registration.name,
            "branch": branch,
            "path": path,
            "refused": [error.to_string()],
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct EnvGuard {
        name: &'static str,
        previous: Option<std::ffi::OsString>,
    }
    impl EnvGuard {
        fn set(name: &'static str, value: &std::path::Path) -> EnvGuard {
            let previous = std::env::var_os(name);
            std::env::set_var(name, value);
            EnvGuard { name, previous }
        }
    }
    impl Drop for EnvGuard {
        fn drop(&mut self) {
            match &self.previous {
                Some(value) => std::env::set_var(self.name, value),
                None => std::env::remove_var(self.name),
            }
        }
    }

    /// A stub CLI that answers `--version` (the gate's check) and
    /// FAILS every real call: the one-read view must never invoke it,
    /// so a fan-out that sneaks back reddens here.
    fn stub_cli_refuses_calls(dir: &std::path::Path) -> std::path::PathBuf {
        std::fs::create_dir_all(dir).unwrap();
        let script = dir.join("layerfs");
        std::fs::write(
            &script,
            r#"#!/bin/sh
case "$*" in
  *--version*) echo "1.1.0" ;;
  *) echo "layerfs: the one-read view must not call the CLI" >&2; exit 3 ;;
esac
"#,
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        script
    }

    /// The live Store's own answer shape, as `get_history` reported it
    /// on the fleet project on 2026-10-26: `kind`/`version`/`store`,
    /// 65 branches (main + one worked child + 63 idle), the one
    /// worked commit with all nineteen fields, two Layers whose merge
    /// evidence is `sourceBranch`, and the `more` cursor map empty.
    /// `dedupe`/`compression` ride as STRINGS on this route (numbers on
    /// `get_storage`) — the parse accepts both.
    fn live_shape_answer() -> String {
        let mut branches = vec![serde_json::json!({
            "name": "main",
            "base": "326898f652a0826295892a03aec7c6519c48403b098c540c1052a891cab441a5b0",
            "head": null,
        })];
        for index in 0..64 {
            let worked = index == 0;
            branches.push(serde_json::json!({
                "name": format!("fleet-child-20261026t104335z-{index:06}"),
                "forkedFrom": "32919c395f934e2d0c7366dcdd2d93ea961538ee63e2ff54767da80b8f980a07eb",
                "base": "32919c395f934e2d0c7366dcdd2d93ea961538ee63e2ff54767da80b8f980a07eb",
                "head": if worked { serde_json::json!("1289afb03d218314f09a4dd61b70e004590c39fe225b93c3cb4519cf6b297ebd41") } else { serde_json::Value::Null },
            }));
        }
        serde_json::json!({
            "kind": "ok",
            "version": "1b70c3e210460f57",
            "store": { "stats": {
                "logical": 46549294, "canonical": 15562812, "stored": 7388596,
                "metadata": 500678, "file": 7860224, "free": 0,
                "dedupe": "2.991059", "compression": "2.106329",
                "init": { "entries": 510, "files": 448, "bytes": 15516426,
                          "canonical": 15552692, "stored": 7387760,
                          "metadata": 447475, "millis": 525 },
            }},
            "project": "fleet",
            "branches": branches,
            "moreBranches": false,
            "commits": [{
                "id": "1289afb03d218314f09a4dd61b70e004590c39fe225b93c3cb4519cf6b297ebd41",
                "branch": "fleet-child-20261026t104335z-000000",
                "parent": null, "secondParent": null,
                "base": "32919c395f934e2d0c7366dcdd2d93ea961538ee63e2ff54767da80b8f980a07eb",
                "root": "ace414acc7c6bfda555494d5e2f3c09966e28b99c3607961d070c5c7d095763b",
                "via": "fs", "message": "fleet: write_file", "mergedFrom": null,
                "displaced": null, "lww": [], "lwwTruncated": false,
                "logical": 15516434, "newCanonical": 10120, "newStored": 836,
                "newMetadata": 791, "time": 1793011431135_i64,
            }],
            "more": {},
            "layers": [
                { "id": "326898f652a0826295892a03aec7c6519c48403b098c540c1052a891cab441a5b0",
                  "logical": 15516434,
                  "parent": "32919c395f934e2d0c7366dcdd2d93ea961538ee63e2ff54767da80b8f980a07eb",
                  "root": "ace414acc7c6bfda555494d5e2f3c09966e28b99c3607961d070c5c7d095763b",
                  "sourceBranch": null, "sourceCommit": null, "overwroteCount": 0 },
                { "id": "32919c395f934e2d0c7366dcdd2d93ea961538ee63e2ff54767da80b8f980a07eb",
                  "logical": 15516426, "parent": null,
                  "root": "ace414acc7c6bfda555494d5e2f3c09966e28b99c3607961d070c5c7d095763b",
                  "sourceBranch": null, "sourceCommit": null, "overwroteCount": 0 },
            ],
            "moreLayers": false,
            "workspaces": [],
        })
        .to_string()
    }

    /// A stub history script: runs the given JS body (a full statement
    /// or expression — a bare JSON literal is wrapped for the caller),
    /// and appends its argv to a count file so a test can count the
    /// reads. The script is ESM: its imports are real imports.
    fn stub_history(dir: &std::path::Path, body: &str) -> (std::path::PathBuf, std::path::PathBuf) {
        std::fs::create_dir_all(dir).unwrap();
        let script = dir.join("history.mjs");
        let count = dir.join("calls.log");
        let trimmed = body.trim_start();
        let printed = if trimmed.starts_with('{') {
            format!("console.log(JSON.stringify({body}));")
        } else {
            body.to_string()
        };
        std::fs::write(
            &script,
            format!(
                "import {{ appendFileSync }} from \"node:fs\";
appendFileSync({count:?}, JSON.stringify(process.argv.slice(2)) + \"\\n\");
{printed}
",
                count = count,
                printed = printed
            ),
        )
        .unwrap();
        (script, count)
    }

    #[test]
    fn a_substrate_failure_is_named_in_the_panels_language() {
        // The two failures a Docker reset produces, verbatim from the
        // node SDK's uncaught throw, are recognized by name.
        let worker = "LayerFSError: Failed: the LayerFS worker did not answer (connect ECONNREFUSED 127.0.0.1:54277); the call was not sent\n    at outcome (file:///bundle/client.js:40)";
        let named = named_refusal(worker).expect("the worker failure is recognized");
        assert!(named.contains("worker is not running"), "{named}");
        assert!(named.contains("fleet install"), "{named}");
        let store = "LayerFSError: Unsupported: the Store's directory /var/lib/docker/volumes/v/_data/global cannot be examined: ENOENT: No such file or directory";
        let named = named_refusal(store).expect("the missing store is recognized");
        assert!(named.contains("Store volume is missing"), "{named}");
        assert!(named.contains("restarts empty"), "{named}");
        // Anything unrecognized keeps its own words.
        assert!(named_refusal("TypeError: something else broke").is_none());
    }

    #[test]
    fn a_recorded_worker_that_answers_nothing_is_named_before_the_script() {
        let root = std::env::temp_dir().join(format!(
            "fleet-layerfs-worker-gap-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let layerfs_home = root.join("layerfs-home");
        std::fs::create_dir_all(layerfs_home.join("worker")).unwrap();
        let endpoint = layerfs_home.join("worker").join("endpoint.json");
        // No endpoint file: no worker was ever recorded, so nothing is named.
        assert!(worker_gap(&layerfs_home).is_none());
        // A live port behind the record: no gap either.
        let live = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let live_port = live.local_addr().unwrap().port();
        std::fs::write(
            &endpoint,
            serde_json::json!({ "port": live_port }).to_string(),
        )
        .unwrap();
        assert!(worker_gap(&layerfs_home).is_none());
        drop(live);
        // A closed port behind the record: named, with the address it
        // was expected to answer on.
        let closed = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let closed_port = closed.local_addr().unwrap().port();
        drop(closed);
        std::fs::write(
            &endpoint,
            serde_json::json!({ "port": closed_port }).to_string(),
        )
        .unwrap();
        let gap = worker_gap(&layerfs_home).expect("a dead worker is a gap");
        assert!(gap.contains("worker is not running"), "{gap}");
        assert!(gap.contains(&closed_port.to_string()), "{gap}");
        assert!(gap.contains("fleet install"), "{gap}");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_dead_worker_is_named_before_the_history_script_runs() {
        let _lock = crate::TEST_ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let root = std::env::temp_dir().join(format!(
            "fleet-layerfs-dead-worker-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let home = view_home(&root);
        // The machine's facts, so the read resolves the recorded home:
        // a config carrying the layerfs home, bundle, and image.
        let layerfs_home = root.join("layerfs-home");
        let bundle = root.join("bundle");
        std::fs::create_dir_all(layerfs_home.join("worker")).unwrap();
        std::fs::create_dir_all(&bundle).unwrap();
        let mut config: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(home.join("config.json")).unwrap())
                .unwrap();
        config["layerfs"]["home"] = serde_json::json!(layerfs_home.display().to_string());
        config["layerfs"]["bundle"] = serde_json::json!(bundle.display().to_string());
        config["layerfs"]["image"] = serde_json::json!("sha256:test");
        std::fs::write(
            home.join("config.json"),
            serde_json::to_string(&config).unwrap(),
        )
        .unwrap();
        // A recorded worker behind a closed port: the Docker reset's shape.
        let closed = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let closed_port = closed.local_addr().unwrap().port();
        drop(closed);
        std::fs::write(
            layerfs_home.join("worker").join("endpoint.json"),
            serde_json::json!({ "port": closed_port }).to_string(),
        )
        .unwrap();

        let _home = EnvGuard::set("FLEET_HOME", &home);
        let (script, count) = stub_history(&root.join("history"), &live_shape_answer());
        let _history = EnvGuard::set("FLEET_LAYERFS_HISTORY", &script);
        let err = history(&registration(), &home, None).expect_err("the dead worker refuses");
        assert!(err.contains("worker is not running"), "{err}");
        assert!(err.contains(&closed_port.to_string()), "{err}");
        // The script never ran: the count file stays absent or empty.
        let calls = std::fs::read_to_string(&count).unwrap_or_default();
        assert!(calls.trim().is_empty(), "{calls}");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_last_writer_wins_entry_reads_as_its_object() {
        // The substrate's shape: `{path, kept, deleted, rule}` — the old
        // parse kept only strings, so every entry vanished.
        let entry = serde_json::json!({
            "path": "docs/plans/x.md",
            "kept": "main",
            "deleted": "fleet-child-20261026t104335z-000000",
            "rule": "newer wins"
        });
        let line = lww_line(entry.as_object().expect("an object"));
        assert!(line.contains("docs/plans/x.md"), "{line}");
        assert!(line.contains("newer wins"), "{line}");
        assert!(line.contains("dropped fleet-child"), "{line}");
        // A bare string still reads as itself (the older shape).
        let legacy: Vec<serde_json::Value> = vec![serde_json::json!("plain entry")];
        let parsed: Vec<String> = legacy
            .iter()
            .filter_map(|item| match item {
                serde_json::Value::String(text) => Some(text.clone()),
                serde_json::Value::Object(map) => Some(lww_line(map)),
                _ => None,
            })
            .collect();
        assert_eq!(parsed, vec!["plain entry".to_string()]);
    }

    fn view_home(root: &std::path::Path) -> std::path::PathBuf {
        let home = root.join("fleet-home");
        let calib_root = root.join("calib");
        std::fs::create_dir_all(&home).unwrap();
        std::fs::create_dir_all(&calib_root).unwrap();
        // The home points the stream at the fixture root.
        std::fs::write(
            home.join("config.json"),
            serde_json::json!({
                "calib": { "log_root": calib_root.display().to_string() },
                "layerfs": { "projects": {
                    "/nowhere": { "name": "fleet", "store": "/volumes/v/_data/global/store.sqlite" },
                } },
            })
            .to_string(),
        )
        .unwrap();
        // One delegation row: the child's session and its branch.
        let day = crate::calib::today();
        // The stream's day file is nested (year/month/day.jsonl) —
        // built through the stream's own path helper.
        std::fs::create_dir_all(crate::calib::day_path(&calib_root, &day).parent().unwrap())
            .unwrap();
        std::fs::write(
            crate::calib::day_path(&calib_root, &day),
            serde_json::json!({
                "op": "delegation",
                "session": "20261026T104335Z-000000",
                "outcome": "completed",
                "workspace_branch": "fleet-child-20261026t104335z-000000",
            })
            .to_string()
                + "\n",
        )
        .unwrap();
        home
    }

    fn registration() -> Registration {
        Registration {
            name: "fleet".to_string(),
            store: PathBuf::from("/volumes/v/_data/global/store.sqlite"),
            tree: None,
        }
    }

    /// C0's spine: the whole view assembles from ONE backend read — the
    /// stub script is called exactly once with the project and bundle,
    /// the CLI stub is never invoked for a real call, and every field
    /// the graph draws survives the parse. The live answer's own shape
    /// (65 branches, 2 layers, 1 commit) is the fixture.
    #[test]
    fn the_history_read_is_one_backend_call_parsed_whole() {
        let _lock = crate::TEST_ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let root = std::env::temp_dir().join(format!(
            "fleet-layerfs-view-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&root);
        let home = view_home(&root);
        let _home = EnvGuard::set("FLEET_HOME", &home);
        let cli = stub_cli_refuses_calls(&root.join("stub"));
        let _cli = EnvGuard::set("FLEET_LAYERFS_BIN", &cli);
        let (script, count) = stub_history(&root.join("history"), &live_shape_answer());
        let _history = EnvGuard::set("FLEET_LAYERFS_HISTORY", &script);

        let view = history(&registration(), &home, None).expect("the view assembles");
        let HistoryRead::Full(view) = view else {
            panic!("the first read is the full view");
        };
        // ONE read: the script ran once, and it ran with the project
        // and the bundle.
        let calls = std::fs::read_to_string(&count).unwrap();
        assert_eq!(calls.lines().count(), 1, "{calls}");
        assert!(calls.contains("\"fleet\""), "{calls}");
        // The whole answer parsed: 65 branches (main + 64 children),
        // 2 layers, 1 commit — no field left null by omission.
        assert_eq!(view.branches.len(), 65, "{}", calls.lines().count());
        assert_eq!(view.branches[0].name, "main");
        assert!(view.branches[0].head.is_none());
        let child = view
            .branches
            .iter()
            .find(|branch| branch.name == "fleet-child-20261026t104335z-000000")
            .expect("the worked branch");
        assert_eq!(
            child.head.as_deref(),
            Some("1289afb03d218314f09a4dd61b70e004590c39fe225b93c3cb4519cf6b297ebd41")
        );
        assert_eq!(
            child.base.as_deref(),
            Some("32919c395f934e2d0c7366dcdd2d93ea961538ee63e2ff54767da80b8f980a07eb")
        );
        // The child join: the day's row folds onto the branch.
        let link = child.child.as_ref().expect("the branch folds to a child");
        assert_eq!(link.session, "20261026T104335Z-000000");
        assert_eq!(link.outcome, "completed");
        assert_eq!(view.commits.len(), 1);
        let commit = &view.commits[0];
        assert_eq!(commit.branch, "fleet-child-20261026t104335z-000000");
        assert_eq!(commit.via, "fs");
        assert_eq!(commit.message.as_deref(), Some("fleet: write_file"));
        assert_eq!(commit.new_stored, 836);
        assert_eq!(commit.new_metadata, 791);
        assert!(commit.lww.is_empty());
        assert!(!commit.lww_truncated);
        assert!(commit.displaced.is_none());
        assert_eq!(commit.time, 1_793_011_431_135_i64);
        assert_eq!(view.layers.len(), 2);
        assert_eq!(
            view.layers[0].parent.as_deref(),
            Some("32919c395f934e2d0c7366dcdd2d93ea961538ee63e2ff54767da80b8f980a07eb")
        );
        assert!(view.layers[0].source_commit.is_none());
        assert_eq!(view.layers[0].overwrote_count, 0);
        // The nine figures, from the answer's own store.stats — the
        // string-carried ratios parsed to numbers.
        let storage = view.storage.as_ref().expect("the figures");
        assert_eq!(storage.logical, 46_549_294);
        assert_eq!(storage.canonical, 15_562_812);
        assert_eq!(storage.stored, 7_388_596);
        assert_eq!(storage.metadata, 500_678);
        assert_eq!(storage.file, 7_860_224);
        assert_eq!(storage.free, 0);
        assert_eq!(storage.dedupe, Some(2.991059));
        assert_eq!(storage.compression, Some(2.106329));
        let init = storage.init.as_ref().expect("the Init record");
        assert_eq!(init.files, 448);
        assert_eq!(init.millis, 525);
        // The states, derived: merged needs a layer naming the branch,
        // so this store's one worked branch is open; the rest idle.
        assert_eq!(
            branch_state(&view, "fleet-child-20261026t104335z-000000"),
            "open"
        );
        assert_eq!(
            branch_state(&view, "fleet-child-20261026t104335z-000001"),
            "idle"
        );
        assert_eq!(branch_state(&view, "main"), "idle");
        assert!(view.note.is_none());
        assert!(view.cut_branches.is_empty());
        assert!(!view.more_branches);

        // A second read carrying the version the answer named: the
        // version rides the call (the unchanged answer itself is pinned
        // by `the_since_version_rides_the_call`).
        let read = history(&registration(), &home, Some("1b70c3e210460f57"))
            .expect("the since read answers");
        assert!(matches!(read, HistoryRead::Full(_)));
        let calls = std::fs::read_to_string(&count).unwrap();
        assert_eq!(calls.lines().count(), 2, "{calls}");
        assert!(calls.lines().nth(1).unwrap().contains("1b70c3e210460f57"));
        let _ = std::fs::remove_dir_all(&root);
    }

    /// The `since` version rides the script's third argument, and the
    /// unchanged answer carries only the version token.
    #[test]
    fn the_since_version_rides_the_call() {
        let _lock = crate::TEST_ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let root = std::env::temp_dir().join(format!(
            "fleet-layerfs-since-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&root);
        let home = view_home(&root);
        let _home = EnvGuard::set("FLEET_HOME", &home);
        let (script, count) = stub_history(
            &root.join("history"),
            r#"console.log(JSON.stringify({ kind: "unchanged", version: "same-token" }));"#,
        );
        let _history = EnvGuard::set("FLEET_LAYERFS_HISTORY", &script);
        let read = history(&registration(), &home, Some("same-token")).expect("the read");
        match read {
            HistoryRead::Unchanged(version) => assert_eq!(version, "same-token"),
            HistoryRead::Full(_) => panic!("unchanged"),
        }
        let calls = std::fs::read_to_string(&count).unwrap();
        assert!(calls.contains("same-token"), "{calls}");
        let _ = std::fs::remove_dir_all(&root);
    }

    /// The cut is named, never silent: moreBranches, moreLayers and the
    /// per-branch commit cut each say so in the truncation word.
    #[test]
    fn the_cut_is_named_in_words() {
        let _lock = crate::TEST_ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let root = std::env::temp_dir().join(format!(
            "fleet-layerfs-cut-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&root);
        let home = view_home(&root);
        let _home = EnvGuard::set("FLEET_HOME", &home);
        let body = r#"console.log(JSON.stringify({
  kind: "ok", version: "v2", store: { stats: {} }, project: "fleet",
  branches: [], moreBranches: true,
  commits: [], more: { "fleet-child-x": "cursor" },
  layers: [], moreLayers: true, workspaces: [],
}));"#;
        let (script, _count) = stub_history(&root.join("history"), body);
        let _history = EnvGuard::set("FLEET_LAYERFS_HISTORY", &script);
        let view = history(&registration(), &home, None).expect("the view");
        let HistoryRead::Full(view) = view else {
            panic!("full");
        };
        assert!(view.more_branches);
        assert!(view.more_layers);
        assert_eq!(view.cut_branches, vec!["fleet-child-x".to_string()]);
        let said = truncation_word(&view).expect("the cut is named");
        assert!(said.contains("more remain in the Store"), "{said}");
        assert!(said.contains("layers shown, more remain"), "{said}");
        assert!(
            said.contains("older commits cut on fleet-child-x"),
            "{said}"
        );
        // A whole answer says nothing.
        let (script, _) = stub_history(
            &root.join("history2"),
            r#"console.log(JSON.stringify({
  kind: "ok", version: "v3", store: { stats: {} }, project: "fleet",
  branches: [], moreBranches: false, commits: [], more: {},
  layers: [], moreLayers: false, workspaces: [],
}));"#,
        );
        let _history = EnvGuard::set("FLEET_LAYERFS_HISTORY", &script);
        let view = history(&registration(), &home, None).expect("the view");
        let HistoryRead::Full(view) = view else {
            panic!("full");
        };
        assert!(truncation_word(&view).is_none());
        let _ = std::fs::remove_dir_all(&root);
    }

    /// A dead read is named: the script's failure is the view's
    /// refusal, with nothing drawn as an empty tree.
    #[test]
    fn a_dead_read_is_named() {
        let _lock = crate::TEST_ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let root = std::env::temp_dir().join(format!(
            "fleet-layerfs-dead-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&root);
        let home = view_home(&root);
        let _home = EnvGuard::set("FLEET_HOME", &home);
        let (script, _) = stub_history(
            &root.join("history"),
            r#"console.error("worker down"); process.exit(1);"#,
        );
        let _history = EnvGuard::set("FLEET_LAYERFS_HISTORY", &script);
        let error = history(&registration(), &home, None).expect_err("the failure is named");
        assert!(error.contains("script failed"), "{error}");
        assert!(error.contains("worker down"), "{error}");
        let _ = std::fs::remove_dir_all(&root);
    }

    /// The merge state reads the layer that names the branch: a layer
    /// whose `sourceBranch` is `merge-<child>` (the parent's settlement
    /// form) marks the child merged; a head nobody merged is open.
    #[test]
    fn the_merge_state_reads_the_layer_that_names_the_branch() {
        let _lock = crate::TEST_ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let root = std::env::temp_dir().join(format!(
            "fleet-layerfs-state-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&root);
        let home = view_home(&root);
        let _home = EnvGuard::set("FLEET_HOME", &home);
        let body = r#"console.log(JSON.stringify({
  kind: "ok", version: "v4", store: { stats: {} }, project: "fleet",
  branches: [
    { name: "main", base: "l2", head: null },
    { name: "fleet-child-merged", base: "l1", head: "c1" },
    { name: "fleet-child-open", base: "l1", head: "c2" },
    { name: "fleet-child-idle", base: "l1", head: null },
  ],
  commits: [
    { id: "c1", branch: "fleet-child-merged", via: "fs", time: 1 },
    { id: "c2", branch: "fleet-child-open", via: "fs", time: 2 },
  ],
  more: {},
  layers: [
    { id: "l2", logical: 5, parent: "l1", sourceBranch: "merge-fleet-child-merged", sourceCommit: "c1", overwroteCount: 0 },
    { id: "l1", logical: 5, parent: null, sourceBranch: null, sourceCommit: null, overwroteCount: 0 },
  ],
  moreLayers: false, workspaces: [],
}));"#;
        let (script, _) = stub_history(&root.join("history"), body);
        let _history = EnvGuard::set("FLEET_LAYERFS_HISTORY", &script);
        let view = history(&registration(), &home, None).expect("the view");
        let HistoryRead::Full(view) = view else {
            panic!("full");
        };
        assert_eq!(branch_state(&view, "fleet-child-merged"), "merged");
        assert_eq!(branch_state(&view, "fleet-child-open"), "open");
        assert_eq!(branch_state(&view, "fleet-child-idle"), "idle");
        let _ = std::fs::remove_dir_all(&root);
    }

    /// C0's live proof: the one-read view against the provisioned wire.
    /// Needs the environment (see scripts/layerfs-probe.sh):
    ///   FLEET_LAYERFS_HISTORY=… (or an installed home) \
    ///   cargo test --lib layerfs_live_history -- --ignored --nocapture
    #[test]
    #[ignore = "needs the provisioned LayerFS environment (KI-313)"]
    fn layerfs_live_history_against_the_real_wire() {
        let home = std::env::var("FLEET_HOME")
            .ok()
            .map(std::path::PathBuf::from)
            .expect("FLEET_HOME");
        let registration = crate::layerfs_tools::registrations()
            .first()
            .expect("a registered project")
            .1
            .clone();
        // The first read warms the worker and node; the BOUND is the
        // warm read's (a cold worker's 0.7 s is startup, not the read).
        let _ = history(&registration, &home, None).expect("the warm-up read");
        let started = std::time::Instant::now();
        let view = history(&registration, &home, None).expect("the live view");
        let elapsed = started.elapsed();
        let HistoryRead::Full(view) = view else {
            panic!("the live read is full");
        };
        println!(
            "LIVE READ: {elapsed:?} — {} branch(es), {} commit(s), {} layer(s), version {}",
            view.branches.len(),
            view.commits.len(),
            view.layers.len(),
            view.version
        );
        if let Some(storage) = &view.storage {
            println!(
                "  figures: logical {} canonical {} stored {} metadata {} file {} free {} dedupe {:?} compression {:?}",
                storage.logical, storage.canonical, storage.stored, storage.metadata,
                storage.file, storage.free, storage.dedupe, storage.compression
            );
        }
        assert!(!view.branches.is_empty(), "the live branches serve");
        assert!(!view.layers.is_empty(), "main's trunk serves");
        assert!(view.storage.is_some(), "the figures serve");
        // The bound the plan names for the one read.
        assert!(
            elapsed < std::time::Duration::from_millis(250),
            "the one read answers under 250 ms, took {elapsed:?}"
        );
    }

    /// A scratch machine whose traces dir the test fills: the config's
    /// `layerfs` facts point at real dirs (the reader refuses a home or
    /// a bundle that is gone), so the tail has a world to read.
    fn tail_home(root: &std::path::Path) -> (std::path::PathBuf, std::path::PathBuf) {
        let fleet_home = root.join("fleet-home");
        let layerfs_home = root.join("layerfs-home");
        let bundle = root.join("bundle");
        std::fs::create_dir_all(&fleet_home).unwrap();
        std::fs::create_dir_all(layerfs_home.join("traces")).unwrap();
        std::fs::create_dir_all(&bundle).unwrap();
        std::fs::write(
            fleet_home.join("config.json"),
            serde_json::json!({
                "layerfs": {
                    "home": layerfs_home.display().to_string(),
                    "bundle": bundle.display().to_string(),
                    "image": "image",
                }
            })
            .to_string(),
        )
        .unwrap();
        (fleet_home, layerfs_home)
    }

    fn trace_line(method: &str, start: &str, project: &str) -> String {
        serde_json::json!({
            "layerfs_trace": 1,
            "id": "c5c6a04c-1",
            "method": method,
            "args": { "path": "/notes.txt" },
            "start": start,
            "ms": 9.265,
            "ok": true,
            "kind": "Committed",
            "commit": "12c20c75063c",
            "sizes": { "logical": 124, "newStored": 1508 },
            "project": project,
        })
        .to_string()
    }

    /// C2's tail: complete lines answered once, the torn trailing line
    /// held until its newline, a second poll answering only what moved.
    #[test]
    fn the_tail_answers_complete_lines_once() {
        let _lock = crate::TEST_ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let root = std::env::temp_dir().join(format!(
            "fleet-tail-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&root);
        let (fleet_home, layerfs_home) = tail_home(&root);
        let _home = EnvGuard::set("FLEET_HOME", &fleet_home);
        let traces = layerfs_home.join("traces");
        // Two agents: one file with two complete lines, one whose last
        // line is torn mid-write.
        std::fs::write(
            traces.join("20261026T100000Z-aaaaaa.jsonl"),
            trace_line("fs.write", "2026-10-26T10:00:00.000Z", "fleet")
                + "\n"
                + &trace_line("fs.read", "2026-10-26T10:00:01.000Z", "fleet")
                + "\n",
        )
        .unwrap();
        let complete = trace_line("fs.write", "2026-10-26T10:00:03.000Z", "fleet");
        let torn = &complete[..complete.len() - 12]; // cut mid-JSON, no newline yet
        std::fs::write(
            traces.join("20261026T100001Z-bbbbbb.jsonl"),
            trace_line("workspace.mount", "2026-10-26T10:00:02.000Z", "fleet") + "\n" + torn,
        )
        .unwrap();

        let mut tail = TraceTail::default();
        let doc = workspace_calls(Path::new("/nowhere"), None, &mut tail);
        let calls = doc
            .get("calls")
            .and_then(serde_json::Value::as_array)
            .unwrap();
        assert_eq!(calls.len(), 3, "{doc}");
        // Newest last — the timeline sorts across files; the torn line
        // is not among them.
        assert_eq!(
            calls[2].get("method").and_then(|m| m.as_str()),
            Some("workspace.mount")
        );
        assert!(calls[2]
            .get("start")
            .and_then(|s| s.as_str())
            .unwrap()
            .ends_with(".000Z"));
        assert_eq!(
            calls[0].get("actor").and_then(|a| a.as_str()),
            Some("20261026T100000Z-aaaaaa")
        );
        assert!(
            doc.get("note")
                .and_then(serde_json::Value::as_str)
                .is_none(),
            "{doc}"
        );

        // A second poll: only what moved — nothing, until the torn line
        // finishes. The partial fragment is answered as a whole line
        // once its newline lands.
        let doc = workspace_calls(Path::new("/nowhere"), None, &mut tail);
        assert!(
            doc.get("calls")
                .and_then(serde_json::Value::as_array)
                .unwrap()
                .is_empty(),
            "{doc}"
        );
        // The torn line completes the way the substrate writes: an
        // append of the missing tail and its newline.
        {
            use std::io::Write;
            let mut file = std::fs::OpenOptions::new()
                .append(true)
                .open(traces.join("20261026T100001Z-bbbbbb.jsonl"))
                .unwrap();
            file.write_all(&complete.as_bytes()[complete.len() - 12..])
                .unwrap();
            file.write_all(b"\n").unwrap();
        }
        let doc = workspace_calls(Path::new("/nowhere"), None, &mut tail);
        let calls = doc
            .get("calls")
            .and_then(serde_json::Value::as_array)
            .unwrap();
        assert_eq!(calls.len(), 1, "{doc}");
        assert_eq!(
            calls[0].get("method").and_then(|m| m.as_str()),
            Some("fs.write")
        );
        assert_eq!(
            calls[0].get("start").and_then(|s| s.as_str()),
            Some("2026-10-26T10:00:03.000Z")
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// A file that shrank was truncated or replaced: the tail starts
    /// over rather than answering from a stale offset.
    #[test]
    fn the_tail_resets_when_a_file_shrinks() {
        let _lock = crate::TEST_ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let root = std::env::temp_dir().join(format!(
            "fleet-tail-shrink-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&root);
        let (fleet_home, layerfs_home) = tail_home(&root);
        let _home = EnvGuard::set("FLEET_HOME", &fleet_home);
        let traces = layerfs_home.join("traces");
        let file = traces.join("20261026T100000Z-cccccc.jsonl");
        std::fs::write(
            &file,
            trace_line("fs.write", "2026-10-26T10:00:00.000Z", "fleet")
                + "\n"
                + &trace_line("fs.rm", "2026-10-26T10:00:01.000Z", "fleet")
                + "\n",
        )
        .unwrap();
        let mut tail = TraceTail::default();
        let doc = workspace_calls(Path::new("/nowhere"), None, &mut tail);
        assert_eq!(
            doc.get("calls")
                .and_then(serde_json::Value::as_array)
                .unwrap()
                .len(),
            2,
            "{doc}"
        );
        // The file replaced by a shorter one: the tail re-reads whole.
        std::fs::write(
            &file,
            trace_line("workspace.mount", "2026-10-26T10:00:02.000Z", "fleet") + "\n",
        )
        .unwrap();
        let doc = workspace_calls(Path::new("/nowhere"), None, &mut tail);
        let calls = doc
            .get("calls")
            .and_then(serde_json::Value::as_array)
            .unwrap();
        assert_eq!(calls.len(), 1, "{doc}");
        assert_eq!(
            calls[0].get("method").and_then(|m| m.as_str()),
            Some("workspace.mount")
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// The filters: another project's lines never answer, and `since`
    /// keeps everything at or after the named start.
    #[test]
    fn the_tail_filters_by_project_and_since() {
        let _lock = crate::TEST_ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let root = std::env::temp_dir().join(format!(
            "fleet-tail-filter-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&root);
        let (fleet_home, layerfs_home) = tail_home(&root);
        // The launch folder is a registered project, so the project
        // filter has a name to hold the lines to.
        let cwd = std::env::temp_dir().join(format!(
            "fleet-tail-folder-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&cwd).unwrap();
        let config: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(fleet_home.join("config.json")).unwrap())
                .unwrap();
        let mut with_projects = config.clone();
        with_projects["layerfs"]["projects"] = serde_json::json!({
            cwd.display().to_string(): {
                "name": "fleet",
                "store": "/volumes/v/_data/global/store.sqlite",
            }
        });
        std::fs::write(fleet_home.join("config.json"), with_projects.to_string()).unwrap();
        let _home = EnvGuard::set("FLEET_HOME", &fleet_home);
        let traces = layerfs_home.join("traces");
        std::fs::write(
            traces.join("20261026T100000Z-dddddd.jsonl"),
            trace_line("fs.write", "2026-10-26T10:00:00.000Z", "other")
                + "\n"
                + &trace_line("fs.write", "2026-10-26T10:00:01.000Z", "fleet")
                + "\n"
                + &trace_line("fs.read", "2026-10-26T10:00:02.000Z", "fleet")
                + "\n",
        )
        .unwrap();
        let mut tail = TraceTail::default();
        let doc = workspace_calls(&cwd, Some("2026-10-26T10:00:02.000Z"), &mut tail);
        let calls = doc
            .get("calls")
            .and_then(serde_json::Value::as_array)
            .unwrap();
        assert_eq!(calls.len(), 1, "{doc}");
        assert_eq!(
            calls[0].get("method").and_then(|m| m.as_str()),
            Some("fs.read")
        );
        // A refused call rides the feed as refused, never as work.
        std::fs::write(
            traces.join("20261026T100000Z-dddddd.jsonl"),
            trace_line("fs.write", "2026-10-26T10:00:00.000Z", "other")
                + "\n"
                + &trace_line("fs.write", "2026-10-26T10:00:01.000Z", "fleet")
                + "\n"
                + &trace_line("fs.read", "2026-10-26T10:00:02.000Z", "fleet")
                + "\n"
                + &trace_line("fs.write", "2026-10-26T10:00:03.000Z", "fleet")
                    .replace("\"ok\":true", "\"ok\":false")
                + "\n",
        )
        .unwrap();
        tail.offsets.clear();
        let doc = workspace_calls(&cwd, None, &mut tail);
        let calls = doc
            .get("calls")
            .and_then(serde_json::Value::as_array)
            .unwrap();
        let refused = calls
            .iter()
            .find(|call| call.get("ok").and_then(serde_json::Value::as_bool) == Some(false))
            .expect("the refused call rides the feed");
        assert_eq!(
            refused.get("ok").and_then(serde_json::Value::as_bool),
            Some(false)
        );
        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&cwd);
    }

    /// The cap: past 200 lines the newest 200 answer and the drop is
    /// named.
    #[test]
    fn the_tail_caps_at_200_and_names_the_drop() {
        let _lock = crate::TEST_ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let root = std::env::temp_dir().join(format!(
            "fleet-tail-cap-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&root);
        let (fleet_home, layerfs_home) = tail_home(&root);
        let _home = EnvGuard::set("FLEET_HOME", &fleet_home);
        let traces = layerfs_home.join("traces");
        let mut body = String::new();
        for index in 0..250 {
            body.push_str(&trace_line(
                "fs.write",
                &format!("2026-10-26T10:{:02}:{:02}.000Z", index / 60, index % 60),
                "fleet",
            ));
            body.push('\n');
        }
        std::fs::write(traces.join("20261026T100000Z-eeeeee.jsonl"), body).unwrap();
        let mut tail = TraceTail::default();
        let doc = workspace_calls(Path::new("/nowhere"), None, &mut tail);
        assert_eq!(
            doc.get("calls")
                .and_then(serde_json::Value::as_array)
                .unwrap()
                .len(),
            200,
            "{doc}"
        );
        assert_eq!(
            doc.get("more").and_then(serde_json::Value::as_bool),
            Some(true)
        );
        let note = doc.get("note").and_then(serde_json::Value::as_str).unwrap();
        assert!(note.contains("250 calls"), "{note}");
        let _ = std::fs::remove_dir_all(&root);
    }

    /// A stub CLI for the instrument: fs stat, fs read (a window),
    /// fs blame, and the diff verbs — the answer shapes measured live
    /// on the wire.
    fn stub_cli_instrument(dir: &std::path::Path) -> std::path::PathBuf {
        std::fs::create_dir_all(dir).unwrap();
        let script = dir.join("layerfs");
        std::fs::write(
            &script,
            r#"#!/bin/sh
case "$*" in
  *--version*) echo "1.1.0" ;;
  *"fs stat"*"/notes.txt"*|*"fs stat"*big.txt*|*"fs stat"*many.txt*)
    printf '%s\n' '{"kind":"ok","type":"file","size":8542}' ;;
  *"fs stat"*binary.bin*)
    printf '%s\n' '{"kind":"ok","type":"file","size":11}' ;;
  *"fs stat"*"/missing.txt"*)
    printf '%s\n' '{"code":"NoSuchPath","message":"/missing.txt does not exist on that ref"}' >&2; exit 3 ;;
  *"fs stat"*)
    printf '%s\n' '{"kind":"ok","type":"directory","size":0}' ;;
  *"fs read"*"/notes.txt"*)
    printf '%s\n' '{"kind":"ok","path":"/notes.txt","text":"line one\nline two\n","startLine":1,"endLine":2,"nextLine":3,"truncated":false,"size":8542}' ;;
  *"fs read"*binary.bin*)
    printf '%s\n' '{"kind":"ok","path":"/binary.bin","text":"a\u0000b","startLine":1,"endLine":1,"nextLine":null,"truncated":false,"size":11}' ;;
  *"fs read"*big.txt*)
    python3 -c 'import json; print(json.dumps({"kind":"ok","path":"/big.txt","text":"a"*300000,"startLine":1,"endLine":1,"nextLine":None,"truncated":False,"size":300000}))' ;;
  *"fs blame"*many.txt*)
    python3 -c 'import json; print(json.dumps({"kind":"ok","path":"/many.txt","lines":[{"n":i,"text":"line %d" % i,"commit":"12e968660147668df46ef623a1a28858b74736a2758ddaa5e51b19607d6210","message":"fleet: write_file"} for i in range(1,2002)]}))' ;;
  *"fs blame"*)
    printf '%s\n' '{"kind":"ok","path":"/notes.txt","lines":[{"n":1,"text":"line one","commit":"12e968660147668df46ef623a1a28858b74736a2758ddaa5e51b19607d62104d","message":"fleet: write_file"},{"n":2,"text":"line two","commit":"12e968660147668df46ef623a1a28858b74736a2758ddaa5e51b19607d62104d","message":"fleet: write_file"}]}' ;;
  *"branch diff_commits"*)
    printf '%s\n' '{"kind":"ok","from":{"commit":"aaaa"},"to":{"commit":"bbbb"},"items":[{"path":"/live-proof.txt","kind":"Modified","type":"file"}]}' ;;
  *"fs diff_files"*)
    printf '%s\n' '{"kind":"Text","left":{"path":"/notes.txt"},"right":{"path":"/notes.txt"},"hunks":[{"op":"-","text":"old"},{"op":"+","text":"new"}]}' ;;
  *) echo "layerfs: unknown" >&2; exit 2 ;;
esac
"#,
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        script
    }

    /// C3's four modes: text names its window and its cut, a binary is
    /// named rather than rendered, blame answers the lines with their
    /// commits, revisions ride the backend, and a diff resolves its
    /// layers through the comparable mapping — with a NotComparable
    /// layer refused by name.
    #[test]
    fn the_files_instrument_reads_four_modes_with_named_caps() {
        let _lock = crate::TEST_ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let root = std::env::temp_dir().join(format!(
            "fleet-files-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&root);
        let home = view_home(&root);
        let _home = EnvGuard::set("FLEET_HOME", &home);
        let cli = stub_cli_instrument(&root.join("stub"));
        let _cli = EnvGuard::set("FLEET_LAYERFS_BIN", &cli);
        // The script answers the comparable mapping (a Layer id to its
        // Commit) and the revisions page; everything else is CLI.
        let (script, _count) = stub_history(
            &root.join("history"),
            r#""use strict";
if (process.argv[4] === "--comparable") {
  const ref = process.argv[5] || "";
  if (ref === "initlayer000000000000000000000000000000000000000000000000000000") {
    console.log(JSON.stringify({ code: "NotComparable", message: "LayerFS compares Commits, and this Layer has no Commit with the same files" }));
  } else {
    console.log(JSON.stringify({ commit: "aaaa0000000000000000000000000000000000000000000000000000000000", root: "ace414", exact: true }));
  }
} else if (process.argv[4] === "--revisions") {
  console.log(JSON.stringify({ items: [
    { commit: "aaaa", message: "fleet: write_file", change: "Added", size: 12 },
  ], hasMore: false }));
} else {
  console.log(JSON.stringify({ kind: "ok", version: "v9", store: { stats: {} }, project: "fleet",
    branches: [], moreBranches: false, commits: [], more: {}, layers: [], moreLayers: false, workspaces: [] }));
}"#,
        );
        let _history = EnvGuard::set("FLEET_LAYERFS_HISTORY", &script);

        let cwd = Path::new("/nowhere");
        // Text: the window's own lines and the next-window cursor.
        let doc = workspace_file(cwd, "main", "/notes.txt", "text", None, None, 0);
        assert_eq!(doc["mode"], "text", "{doc}");
        assert_eq!(doc["text"], "line one\nline two\n");
        assert_eq!(doc["start_line"], 1);
        assert_eq!(doc["end_line"], 2);
        assert_eq!(doc["next_line"], 3);
        assert_eq!(doc["size"], 8542);
        // A folder is named, never read as an empty file.
        let doc = workspace_file(cwd, "main", "/", "text", None, None, 0);
        let refused = doc["refused"].as_array().expect("the refusal");
        assert!(
            refused[0].as_str().unwrap().contains("is not a file"),
            "{doc}"
        );
        // A binary is named, never rendered.
        let doc = workspace_file(cwd, "main", "/binary.bin", "text", None, None, 0);
        assert_eq!(doc["binary"], true, "{doc}");
        assert!(
            doc["note"].as_str().unwrap().contains("binary, 11 bytes"),
            "{doc}"
        );
        // A path not in the tree is refused by the wire's own name.
        let doc = workspace_file(cwd, "main", "/missing.txt", "text", None, None, 0);
        let refused = doc["refused"].as_array().expect("the refusal");
        assert!(refused[0].as_str().unwrap().contains("NoSuchPath"), "{doc}");
        // The 256 KB text cap reports that it cut.
        let doc = workspace_file(cwd, "main", "/big.txt", "text", None, None, 0);
        let note = doc["note"].as_str().expect("the cut is named");
        assert!(note.contains("cut at 256 KB"), "{note}");
        assert_eq!(doc["text"].as_str().unwrap().len(), 256 * 1024, "{doc}");
        // The 2,000-line blame cap reports that it cut.
        let doc = workspace_file(cwd, "main", "/many.txt", "blame", None, None, 0);
        let lines = doc["lines"].as_array().expect("the lines");
        assert_eq!(lines.len(), 2_000, "{} lines", lines.len());
        let note = doc["note"].as_str().expect("the cut is named");
        assert!(note.contains("first 2,000 of 2001 lines"), "{note}");
        // Blame: every line with its commit.
        let doc = workspace_file(cwd, "main", "/notes.txt", "blame", None, None, 0);
        let lines = doc["lines"].as_array().expect("the lines");
        assert_eq!(lines.len(), 2, "{doc}");
        assert_eq!(lines[0]["commit"].as_str().unwrap().len(), 64, "{doc}");
        assert_eq!(lines[1]["text"], "line two");
        // Revisions: the backend's page, newest first.
        let doc = workspace_file(cwd, "main", "/notes.txt", "revisions", None, None, 0);
        let items = doc["items"].as_array().expect("the revisions");
        assert_eq!(items[0]["change"], "Added", "{doc}");
        assert_eq!(doc["has_more"], false, "{doc}");
        // Diff: two layers resolved through the comparable mapping,
        // then the changed set — one CLI call after the two resolves.
        let doc = workspace_file(
            cwd,
            "main",
            "",
            "diff",
            Some("aa11layer"),
            Some("bb22layer"),
            0,
        );
        assert_eq!(doc["mode"], "diff", "{doc}");
        assert_eq!(
            doc["from_commit"],
            "aaaa0000000000000000000000000000000000000000000000000000000000"
        );
        assert_eq!(doc["hunks"], false, "{doc}");
        let items = doc["items"].as_array().expect("the changed set");
        assert_eq!(items[0]["path"], "/live-proof.txt", "{doc}");
        // One changed file's hunks, when the pane names the path.
        let doc = workspace_file(
            cwd,
            "main",
            "/notes.txt",
            "diff",
            Some("aa11layer"),
            Some("bb22layer"),
            0,
        );
        assert_eq!(doc["hunks"], true, "{doc}");
        let hunks = doc["changes"].as_array().expect("the hunks");
        assert_eq!(hunks[0]["op"], "-", "{doc}");
        // A layer with no Commit of the same files — the import's own —
        // is the named NotComparable refusal.
        let doc = workspace_file(
            cwd,
            "main",
            "",
            "diff",
            Some("initlayer000000000000000000000000000000000000000000000000000000"),
            Some("bb22layer"),
            0,
        );
        let refused = doc["refused"].as_array().expect("the refusal");
        assert!(
            refused[0].as_str().unwrap().contains("NotComparable"),
            "{doc}"
        );
        // A diff with no ends is refused by name.
        let doc = workspace_file(cwd, "main", "", "diff", None, None, 0);
        let refused = doc["refused"].as_array().expect("the refusal");
        assert!(refused[0].as_str().unwrap().contains("stands on"), "{doc}");
        let _ = std::fs::remove_dir_all(&root);
    }

    /// The named absence: no traces dir is "traces are off", never
    /// "no activity".
    #[test]
    fn an_empty_traces_dir_names_the_absence() {
        let _lock = crate::TEST_ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let root = std::env::temp_dir().join(format!(
            "fleet-tail-empty-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&root);
        let (fleet_home, layerfs_home) = tail_home(&root);
        std::fs::remove_dir_all(layerfs_home.join("traces")).unwrap();
        let _home = EnvGuard::set("FLEET_HOME", &fleet_home);
        let mut tail = TraceTail::default();
        let doc = workspace_calls(Path::new("/nowhere"), None, &mut tail);
        let note = doc
            .get("note")
            .and_then(serde_json::Value::as_str)
            .expect("named");
        assert!(note.contains("traces are off"), "{note}");
        assert!(doc
            .get("calls")
            .and_then(serde_json::Value::as_array)
            .unwrap()
            .is_empty());
        let _ = std::fs::remove_dir_all(&root);
    }

    /// H1: the rail document filters to the owner's six-week window —
    /// a project whose newest commit sits inside it carries its full
    /// view, one outside is absent, and a read that dies names the
    /// substrate's own refusal rather than an unexplained absence. The
    /// stub keys its answers off the store path the composed
    /// environment carries, so one config holds all four cases.
    #[test]
    fn the_rail_document_filters_to_the_six_week_window() {
        let _lock = crate::TEST_ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let root = std::env::temp_dir().join(format!(
            "fleet-layerfs-rail-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&root);
        let fleet_home = view_home(&root);
        let _home = EnvGuard::set("FLEET_HOME", &fleet_home);
        // Three registrations: two active, one stale, one whose read
        // dies. The config carries all four; the stub tells them apart
        // by the store path the composed environment sets.
        let document = serde_json::json!({
            "calib": { "log_root": root.join("calib").display().to_string() },
            "layerfs": { "projects": {
                "/proj/active-a": { "name": "active-a", "store": "/stores/active-a.sqlite" },
                "/proj/active-b": { "name": "active-b", "store": "/stores/active-b.sqlite" },
                "/proj/stale": { "name": "stale", "store": "/stores/stale.sqlite" },
                "/proj/dead": { "name": "dead", "store": "/stores/dead.sqlite" },
                "/proj/imported": {
                    "name": "imported",
                    "store": root.join("stores/imported.sqlite").display().to_string(),
                },
            } },
        });
        // The imported store's activity is the substrate's own runtime
        // record: created now, zero commits — the import is the origin.
        let runtime = fleet_home.join("layerfs-home/.runtime/projects");
        std::fs::create_dir_all(&runtime).unwrap();
        std::fs::write(
            runtime.join("imported.json"),
            serde_json::json!({
                "name": "imported",
                "created": std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis() as i64)
                    .unwrap_or(0),
            })
            .to_string(),
        )
        .unwrap();
        std::fs::write(fleet_home.join("config.json"), document.to_string()).unwrap();
        let cli = stub_cli_refuses_calls(&root.join("stub"));
        let _cli = EnvGuard::set("FLEET_LAYERFS_BIN", &cli);
        // The stub: the live answer for the active stores, a commit
        // fifty days old for the stale one, and a refusal for the dead
        // one — keyed on LAYERFS_STORE_PATH, which backend_answer sets
        // from the registration before the spawn.
        let script = root.join("history");
        std::fs::create_dir_all(&script).unwrap();
        let script = script.join("history.mjs");
        std::fs::write(
            &script,
            r#"import { writeFileSync } from "node:fs";
const store = process.env.LAYERFS_STORE_PATH || "";
const now = Date.now();
const commit = (time) => ({
  branches: [{ name: "main", commits: [] }, { name: "child", commits: ["c1"] }],
  commits: [{ id: "c1", branch: "child", time, via: "fs", lww: [] }],
  layers: [{ id: "l1" }],
  storage: { logical: 10, canonical: 8, stored: 6, metadata: 2, file: 4096, free: 100 },
  version: "v1",
});
if (store.endsWith("stale.sqlite")) {
  console.log(JSON.stringify(commit(now - 50 * 24 * 3600 * 1000)));
} else if (store.endsWith("dead.sqlite")) {
  console.error("the store volume is gone");
  process.exit(1);
} else if (store.endsWith("imported.sqlite")) {
  console.log(JSON.stringify({
    branches: [{ name: "main", commits: [] }],
    commits: [],
    layers: [{ id: "l1" }],
    storage: { logical: 10, canonical: 8, stored: 6, metadata: 2, file: 4096, free: 100 },
    version: "v1",
  }));
} else {
  console.log(JSON.stringify(commit(now - 3600 * 1000)));
}
"#,
        )
        .unwrap();
        let _history = EnvGuard::set("FLEET_LAYERFS_HISTORY", &script);

        let rail = workspace_projects();
        assert_eq!(rail["type"], "workspace-history-rail");
        assert_eq!(rail["window_days"], ACTIVITY_WINDOW_DAYS);
        let projects = rail["projects"].as_array().expect("the projects");
        let names: Vec<&str> = projects.iter().filter_map(|p| p["name"].as_str()).collect();
        // The stale project is absent, the two active ones carry their
        // views, newest first — the two stubs answer milliseconds
        // apart, so the pair is asserted as a set and the order by the
        // sort's own rule.
        assert_eq!(names.len(), 3, "{rail}");
        assert!(
            names.contains(&"active-a") && names.contains(&"active-b"),
            "{rail}"
        );
        // The imported store: no commits, a fresh birthtime — active.
        assert!(names.contains(&"imported"), "{rail}");
        assert!(!names.contains(&"stale"), "{rail}");
        assert!(projects[0]["newest"].as_i64().unwrap() >= projects[1]["newest"].as_i64().unwrap());
        for project in projects {
            assert_eq!(project["active"], true);
            if project["name"] == "imported" {
                // The imported store's own shape: no commits, the
                // import's origin.
                assert!(project["view"]["commits"].as_array().unwrap().is_empty());
            } else {
                assert!(project["view"]["branches"].as_array().unwrap().len() == 2);
                assert!(project["newest"].as_i64().unwrap() > 0);
            }
        }
        // The dead read names the substrate's own refusal — a broken
        // read is not inactivity, and an unexplained absence is the one
        // shape the surface never draws.
        let refused = rail["refused_projects"].as_array().expect("the refusals");
        assert_eq!(refused.len(), 1, "{rail}");
        assert_eq!(refused[0]["name"], "dead");
        assert!(
            refused[0]["refused"][0]
                .as_str()
                .unwrap_or_default()
                .contains("store volume"),
            "{refused:?}"
        );
        // By path: the rail section's own read answers the project's
        // full view, and an unknown path is refused by name — the
        // control tower's and the second brain's resolution.
        let one = workspace_history_for_registered("/proj/active-a", None);
        assert_eq!(one["project"], "active-a");
        assert_eq!(one["branches"].as_array().unwrap().len(), 2);
        let unknown = workspace_history_for_registered("/proj/nowhere", None);
        assert!(
            unknown["note"]
                .as_str()
                .unwrap_or_default()
                .contains("no registered project"),
            "{unknown}"
        );
        let _ = std::fs::remove_dir_all(&root);
    }
}
