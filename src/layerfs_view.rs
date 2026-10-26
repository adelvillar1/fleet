//! The face's reads (C4): one view model over the LayerFS tree, served
//! to the workspace-history destination.
//!
//! Three sources, assembled once per refresh: the agent CLI for the
//! branches and their commits (`project list_branches`,
//! `branch list_commits`), the bundle's backend script for main's
//! Layers and the Store's figures (the CLI answers `NoSuchBranch` for
//! main's commits — the layer the CLI cannot see is the one the face
//! is built around), and fleet's OWN delegation rows for the join
//! from a `fleet-child-…` branch back to the child that worked it.
//! Every source degrades by name: a missing backend script is a note
//! on the view, never an empty lie.

use std::path::{Path, PathBuf};

use super::layerfs_tools::{Gate, Registration};

/// One Commit on a branch's line: the short id, the message, how it
/// was made (`fs`, mount, merge), when, and how many parents it
/// carries (a second parent is the merge the DAG draws).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitView {
    pub id: String,
    pub message: String,
    pub via: String,
    pub time: i64,
    pub parents: u8,
}

/// One published Layer — main's own line, one immutable snapshot per
/// publish.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayerView {
    pub id: String,
    pub logical: u64,
    pub parent: Option<String>,
}

/// One branch, its recent commits, and — when the name folds to a
/// delegated child — that child's session and outcome, joined from
/// fleet's own rows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BranchView {
    pub name: String,
    pub head: Option<String>,
    pub forked_from: Option<String>,
    pub commits: Vec<CommitView>,
    pub child: Option<ChildLink>,
}

/// The join from a branch to the delegation that worked it: the
/// branch's name carries the child's session id (folded), and the
/// row carries the rest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChildLink {
    pub session: String,
    pub outcome: String,
}

/// The Store's accounting, as the backend reports it.
#[derive(Debug, Clone, PartialEq)]
pub struct StorageView {
    pub logical: u64,
    pub canonical: u64,
    pub stored: u64,
    pub dedupe: f64,
    pub compression: f64,
}

/// The whole view: the branches (with their children), main's Layers,
/// the figures, and the note naming what could not be read.
#[derive(Debug, Clone, PartialEq)]
pub struct HistoryView {
    pub branches: Vec<BranchView>,
    pub layers: Vec<LayerView>,
    pub storage: Option<StorageView>,
    pub note: Option<String>,
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
    let script = home.join("history.mjs");
    if script.is_file() {
        Ok(script)
    } else {
        Err(format!(
            "the history script is not at {} — `fleet install` provisions it",
            script.display()
        ))
    }
}

/// Shorten an id for display: the first twelve bytes are enough to
/// tell two commits apart in a tree the owner reads at a glance.
fn short(id: &str) -> String {
    id.chars().take(12).collect()
}

/// The join from a branch name to a child session: `fleet-child-`
/// followed by the folded session id.
fn child_session_of(branch: &str) -> Option<&str> {
    branch.strip_prefix("fleet-child-")
}

/// Today's delegation rows' `session -> outcome` map — the join's
/// other half, read from the calibration stream's own day file.
fn delegation_outcomes() -> Vec<(String, String)> {
    let Ok(home) = crate::home::fleet_home(
        std::env::var("FLEET_HOME").ok().as_deref(),
        std::env::var("HOME").ok().as_deref(),
    ) else {
        return Vec::new();
    };
    let root = crate::calib::resolve_root(&home, std::env::var("HOME").ok().as_deref());
    let day = crate::calib::today();
    crate::calib::read_day(&root, &day, 512)
        .into_iter()
        .filter(|row| row.op == crate::calib::OP_DELEGATION && !row.workspace_branch.is_empty())
        .map(|row| (row.session, row.outcome))
        .collect()
}

/// The view, assembled: branches and their commits from the CLI,
/// the Layers and figures from the backend script, the children from
/// the day's rows. Two rules the live run taught: **main's commits are
/// the backend's** (the CLI answers `NoSuchBranch` for main — the
/// layer the CLI cannot see is the one the face is built around), and
/// **one branch's failure is a note, never an empty view** — the
/// branches that did answer still serve.
pub fn history(
    gate: &Gate,
    registration: &Registration,
    home: &Path,
) -> Result<HistoryView, String> {
    // The store path is the project's own — the one fact the home
    // alone cannot give the CLI — composed before the reads so every
    // spawn in this view hits the same Store the driver's do.
    std::env::set_var("LAYERFS_STORE_PATH", &registration.store);
    let outcomes = delegation_outcomes();

    // The backend first: its answer carries main's commits beside the
    // Layers and the figures, and its absence is a note the branches
    // survive.
    let (layers, storage, note, trunk_commits) = match read_trunk(home, &registration.name) {
        Ok(trunk) => (trunk.layers, trunk.storage, None, trunk.commits),
        Err(reason) => (Vec::new(), None, Some(reason), Vec::new()),
    };

    let branches_value = gate
        .list_branches(&registration.name)
        .map_err(|error| error.to_string())?;
    let mut branches = Vec::new();
    let mut note = note;
    if let Some(items) = branches_value
        .get("items")
        .and_then(|items| items.as_array())
    {
        for item in items {
            let name = item
                .get("name")
                .and_then(|name| name.as_str())
                .unwrap_or_default()
                .to_string();
            let commits = if name == "main" {
                trunk_commits.clone()
            } else {
                match gate.list_commits(&registration.name, &name, 20) {
                    Ok(value) => parse_commits(&value),
                    Err(error) => {
                        note.get_or_insert_with(|| format!("branch `{name}`: {error}"));
                        Vec::new()
                    }
                }
            };
            let child = child_session_of(&name).and_then(|folded| {
                outcomes
                    .iter()
                    .find(|(session, _)| session.to_lowercase() == folded)
                    .map(|(session, outcome)| ChildLink {
                        session: session.clone(),
                        outcome: outcome.clone(),
                    })
            });
            branches.push(BranchView {
                name,
                head: item.get("head").and_then(|head| head.as_str()).map(short),
                forked_from: item
                    .get("forkedFrom")
                    .and_then(|from| from.as_str())
                    .map(short),
                commits,
                child,
            });
        }
    }
    Ok(HistoryView {
        branches,
        layers,
        storage,
        note,
    })
}

/// One commit row, from either the CLI's list_commits answer or the
/// backend's history answer — the same shape both sides carry.
fn parse_commit(commit: &serde_json::Value) -> CommitView {
    let parents = u8::from(commit.get("secondParent").is_some_and(|p| !p.is_null()))
        + u8::from(commit.get("parent").is_some_and(|p| !p.is_null()));
    CommitView {
        id: short(
            commit
                .get("id")
                .and_then(|id| id.as_str())
                .unwrap_or_default(),
        ),
        message: commit
            .get("message")
            .and_then(|message| message.as_str())
            .unwrap_or_default()
            .to_string(),
        via: commit
            .get("via")
            .and_then(|via| via.as_str())
            .unwrap_or_default()
            .to_string(),
        time: commit
            .get("time")
            .and_then(|time| time.as_i64())
            .unwrap_or(0),
        parents,
    }
}

/// A commits answer's items, either source's shape.
fn parse_commits(document: &serde_json::Value) -> Vec<CommitView> {
    document
        .get("items")
        .and_then(|items| items.as_array())
        .map(|items| items.iter().map(parse_commit).collect())
        .or_else(|| {
            document
                .as_array()
                .map(|items| items.iter().map(parse_commit).collect())
        })
        .unwrap_or_default()
}

/// The trunk: main's Layers and commits and the Store's figures, read
/// through the home's history script. A missing or failing script is
/// a named reason, never an empty lie.
fn read_trunk(home: &Path, project: &str) -> Result<Trunk, String> {
    let script = history_script(home)?;
    let bundle = std::env::var("FLEET_LAYERFS_BUNDLE").unwrap_or_default();
    let output = std::process::Command::new("node")
        .args([script.to_string_lossy().as_ref(), project, &bundle])
        .output()
        .map_err(|error| format!("the history script could not run: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "the history script failed: {}",
            bounded_stderr(&output.stderr)
        ));
    }
    let document: serde_json::Value = serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("the history answer was not JSON: {error}"))?;
    if let Some(error) = document.get("error").and_then(|error| error.as_str()) {
        return Err(error.to_string());
    }
    let layers = document
        .get("layers")
        .and_then(|layers| layers.as_array())
        .map(|layers| {
            layers
                .iter()
                .map(|layer| LayerView {
                    id: short(
                        layer
                            .get("id")
                            .and_then(|id| id.as_str())
                            .unwrap_or_default(),
                    ),
                    logical: layer
                        .get("logical")
                        .and_then(|logical| logical.as_u64())
                        .unwrap_or(0),
                    parent: layer
                        .get("parent")
                        .and_then(|parent| parent.as_str())
                        .map(short),
                })
                .collect()
        })
        .unwrap_or_default();
    let storage = document.get("storage").map(|storage| StorageView {
        logical: storage.get("logical").and_then(|v| v.as_u64()).unwrap_or(0),
        canonical: storage
            .get("canonical")
            .and_then(|v| v.as_u64())
            .unwrap_or(0),
        stored: storage.get("stored").and_then(|v| v.as_u64()).unwrap_or(0),
        dedupe: storage
            .get("dedupe")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0),
        compression: storage
            .get("compression")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0),
    });
    Ok(Trunk {
        layers,
        commits: document
            .get("commits")
            .map(parse_commits)
            .unwrap_or_default(),
        storage,
    })
}

/// The backend's answer, parsed: the Layers, main's commits, and the
/// figures.
struct Trunk {
    layers: Vec<LayerView>,
    commits: Vec<CommitView>,
    storage: Option<StorageView>,
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

    fn stub_cli(dir: &std::path::Path) -> std::path::PathBuf {
        std::fs::create_dir_all(dir).unwrap();
        let script = dir.join("layerfs");
        std::fs::write(
            &script,
            r#"#!/bin/sh
case "$*" in
  *--version*) echo "1.1.0" ;;
  *"project list_branches"*)
    echo '{"items":[{"name":"main","base":"aa11","head":"bb22","forkedFrom":null},{"name":"fleet-child-20261026t010000z-abc123","base":"aa11","head":"cc33","forkedFrom":"aa11"}],"hasMore":false}' ;;
  *"branch list_commits main"*)
    echo '{"items":[{"id":"1244ee6601","parent":null,"secondParent":null,"via":"fs","message":"fleet: write_file","time":1792975007476}],"hasMore":false}' ;;
  *"branch list_commits"*)
    echo '{"items":[{"id":"12e968660147668df46ef623a1a28858b74736a2758ddaa5e51b19607d62104d4c","parent":"1244ee6601","secondParent":"aa11","via":"merge","message":"fleet: merge subagent fleet-child-x","time":1792975010000}],"hasMore":false}' ;;
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

    fn stub_history(dir: &std::path::Path, ok: bool) -> std::path::PathBuf {
        std::fs::create_dir_all(dir).unwrap();
        let script = dir.join("history.mjs");
        let body = if ok {
            r#"console.log(JSON.stringify({
  layers: [{ id: "323d354b0ca1efdc4cdea06cd168c12386b6eec9a3dee373768d1802bc41bb280b", logical: 18777528, parent: null }],
  commits: [{ id: "1244ee6601", parent: null, secondParent: null, via: "fs", message: "fleet: write_file", time: 1792975007476 }],
  workspaces: [],
  storage: { logical: 18777528, canonical: 14955859, stored: 11229684, dedupe: 1.25553, compression: 1.331815 },
}));"#
        } else {
            r#"console.error("worker down"); process.exit(1);"#
        };
        std::fs::write(
            &script,
            format!(
                "{body}
"
            ),
        )
        .unwrap();
        script
    }

    /// C4's view: the branches from the CLI, main's commits and the
    /// trunk's Layers and figures from the backend (the CLI answers
    /// NoSuchBranch for main — the trunk's commits are the backend's),
    /// the child branch's commits from the CLI, and the child joined
    /// from the day's delegation row — and a failed backend named in
    /// the note with the branches still served.
    #[test]
    fn the_view_joins_the_child_and_names_a_dead_backend() {
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
        let home = root.join("fleet-home");
        let calib_root = root.join("calib");
        std::fs::create_dir_all(&home).unwrap();
        std::fs::create_dir_all(&calib_root).unwrap();
        // The home points the stream at the fixture root.
        std::fs::write(
            home.join("config.json"),
            serde_json::json!({ "calib": { "log_root": calib_root.display().to_string() } })
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
                "session": "20261026T010000Z-abc123",
                "outcome": "completed",
                "workspace_branch": "fleet-child-20261026t010000z-abc123",
            })
            .to_string()
                + "
",
        )
        .unwrap();

        let stub = stub_cli(&root.join("stub"));
        let _cli = EnvGuard::set("FLEET_LAYERFS_BIN", &stub);
        let _home = EnvGuard::set("FLEET_HOME", &home);
        let history_ok = stub_history(&root.join("history-ok"), true);
        let _history = EnvGuard::set("FLEET_LAYERFS_HISTORY", &history_ok);
        let gate = Gate::resolve().expect("the stub answers 1.1.0");
        let registration = Registration {
            name: "shop".to_string(),
            store: PathBuf::from("/volumes/v/_data/global/store.sqlite"),
        };

        let view = history(&gate, &registration, &home).expect("the view assembles");
        assert_eq!(view.branches.len(), 2);
        let main = &view.branches[0];
        assert_eq!(main.name, "main");
        assert_eq!(main.commits.len(), 1);
        // main's first Commit is the root: no parents.
        assert_eq!(main.commits[0].parents, 0);
        let child = &view.branches[1];
        // The child's merge-back Commit carries both parents — the
        // edge the DAG draws.
        assert_eq!(child.commits[0].parents, 2);
        let link = child.child.as_ref().expect("the branch folds to a child");
        assert_eq!(link.session, "20261026T010000Z-abc123");
        assert_eq!(link.outcome, "completed");
        assert_eq!(view.layers.len(), 1);
        assert_eq!(view.layers[0].logical, 18777528);
        let storage = view.storage.as_ref().expect("the figures");
        assert_eq!(storage.stored, 11229684);
        assert!(view.note.is_none(), "{:?}", view.note);

        // A dead history script: the note names it, the branches serve.
        let history_dead = stub_history(&root.join("history-dead"), false);
        std::env::set_var("FLEET_LAYERFS_HISTORY", &history_dead);
        let view = history(&gate, &registration, &home).expect("the view still assembles");
        assert_eq!(view.branches.len(), 2, "the branches serve regardless");
        assert!(view.layers.is_empty());
        let note = view.note.expect("the failure is named");
        assert!(note.contains("history script failed"), "{note}");
        std::env::remove_var("FLEET_LAYERFS_HISTORY");

        let _ = std::fs::remove_dir_all(&root);
    }

    /// C4's live proof: the view against the provisioned wire — the
    /// kit project's branches, main's Layers, the figures, and the
    /// history script's real answer. Needs the environment (see
    /// scripts/layerfs-probe.sh):
    ///   FLEET_LAYERFS_BIN=… LAYERFS_*=… FLEET_LAYERFS_HISTORY=… \
    ///   cargo test --lib layerfs_live_history -- --ignored --nocapture
    #[test]
    #[ignore = "needs the provisioned LayerFS environment (KI-313)"]
    fn layerfs_live_history_against_the_real_wire() {
        let home = std::env::var("FLEET_HOME")
            .ok()
            .map(std::path::PathBuf::from)
            .expect("FLEET_HOME");
        let _machine = crate::layerfs_tools::machine().expect("the machine's facts");
        let registration = crate::layerfs_tools::registrations()
            .first()
            .expect("a registered project")
            .1
            .clone();
        let gate = crate::layerfs_tools::gate()
            .as_ref()
            .expect("the CLI")
            .clone();
        let view = history(&gate, &registration, &home).expect("the live view");
        println!(
            "LIVE VIEW: {} branch(es), {} layer(s), storage {}",
            view.branches.len(),
            view.layers.len(),
            view.storage
                .as_ref()
                .map(|s| format!("{} stored of {} logical", s.stored, s.logical))
                .unwrap_or_else(|| "none".to_string())
        );
        for branch in &view.branches {
            println!(
                "  branch {} head={:?} commits={} child={:?}",
                branch.name,
                branch.head,
                branch.commits.len(),
                branch.child.as_ref().map(|c| c.session.clone())
            );
        }
        assert!(!view.branches.is_empty(), "the live branches serve");
        assert!(!view.layers.is_empty(), "main's trunk serves");
        assert!(view.storage.is_some(), "the figures serve");
    }
}
