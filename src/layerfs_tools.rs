//! The LayerFS driver: fleet's one door to the `layerfs` CLI — spawn,
//! parse the JSON answer, name the refusal. C1 of
//! docs/plans/2026-10-25-layerfs-delegated-workspaces.md.
//!
//! LayerFS (<https://github.com/Ephemeral-AI-Lab/layerfs>, MIT) is the
//! substrate the delegated-workspaces plan builds on: a delegated child
//! works in its own writable Workspace over one shared committed
//! history, so concurrent agents answer a named `Conflict` instead of
//! stepping on each other's code. Fleet never vendors it (hard rule 6):
//! the CLI is an external tool reached through `FLEET_LAYERFS_BIN` or
//! PATH, the way the lanes are `python3` — the CLI's own runtime is its
//! implementation detail, not a fleet dependency.
//!
//! Three decisions hold the slice:
//!
//! 1. **The opt-in is one env knob.** [`project_name`] reads
//!    `FLEET_LAYERFS_PROJECT`: the LayerFS project an opted-in run
//!    delegates into. Unset — the default — means the driver is never
//!    consulted and every run is byte-identical to the one fleet
//!    always had (the plan's C3).
//! 2. **Availability is its own once-per-process gate.** [`gate`]
//!    resolves the CLI, reads `--version`, and refuses any answer but
//!    the one wire version fleet drives. The CLI breaks its API
//!    between minors (1.1.0 broke 1.0.0 under a recorded waiver), so
//!    a mismatch is refused by name, never parsed hopefully.
//! 3. **Every call is one spawn, one attempt, typed custody.** No
//!    retry, no replay: a refusal carries the CLI's own code and
//!    message, a timeout carries its budget, and stdout that is not
//!    the promised JSON is a decode error with the bounded body
//!    beside it. Nothing here logs an environment value.

use std::ffi::OsString;
use std::fmt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

/// The LayerFS checkout fleet drives, named the way `services/` VENDOR.json
/// names a carry: repository at commit, and the wire that tree speaks.
/// This is the plan's C3 pin — the gate refuses a CLI whose `--version`
/// disagrees with [`SUPPORTED_WIRE`], and this line says which source
/// tree the supported wire came from. Update both together, in one
/// commit, when the checkout moves.
pub const CHECKOUT_PIN: &str =
    "github.com/Ephemeral-AI-Lab/layerfs@ad78914e36b258b360daf13aec164b5655814689 (wire 1.1.0)";

/// The two images the sealed bundle's build runs on, pinned by digest —
/// both OFFICIAL Docker Hub images, measured 2026-10-26 from the
/// provisioned machine's own recorded build (the builder's
/// `RepoDigests` names `rust@…`, the runtime's `node@…`). The install
/// pulls exactly these and verifies what it pulled against them; a
/// digest that does not match is a named failure, never a silent
/// accept.
pub const BUILD_IMAGE_PIN: &str =
    "rust@sha256:e51d0265072d2d9d5d320f6a44dde6b9ef13653b035098febd68cce8fa7c0bc4";
pub const RUNTIME_IMAGE_PIN: &str =
    "node@sha256:3d27e5c11e5786e309ec3e03f93ae536eb36e6e5eb3714d5eb3300a36157add0";

/// The one `layerfs` wire version fleet drives. A CLI that answers
/// anything else is refused by name: 1.1.0 broke the 1.0.0 SDK under
/// an explicit owner waiver, so "close enough" is a bug waiting to
/// misparse a refusal as a success.
pub const SUPPORTED_WIRE: &str = "1.1.0";

/// The knob's name, by which doctor and the settings footer address it.
pub const PROJECT_ENV: &str = "FLEET_LAYERFS_PROJECT";

/// The CLI override, by which a pinned checkout is addressed instead
/// of PATH resolution.
pub const CLI_ENV: &str = "FLEET_LAYERFS_BIN";

/// The control-call budget: fork, sandbox create, mount, commit,
/// unmount, close, merge. A mount binds a committed root without
/// copying it (the plan's C0 measured what this machine can do; the
/// live numbers land in the plan), so a healthy call is seconds —
/// this is the ceiling that names a hung Docker daemon, not a target.
const CONTROL_BUDGET: Duration = Duration::from_secs(120);

/// The `--version` budget: the smallest call there is, read once per
/// process through the gate.
const VERSION_BUDGET: Duration = Duration::from_secs(10);

/// How much of a CLI answer rides inside an error before it is cut:
/// enough to diagnose, bounded so a garbage answer cannot flood the
/// model's context through a refusal.
const BOUND: usize = 400;

/// The driver's typed failures. Every one names what was attempted and
/// what happened, in the house's named-refusal voice; none of them
/// retries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The CLI is not on this machine, or could not answer
    /// `--version`. The reason is the gate's, verbatim.
    Absent(String),
    /// The CLI answered a wire version fleet does not drive.
    Wire { answered: String },
    /// The CLI refused the call with its own code and message —
    /// `Conflict`, `NoSuchSandbox`, `NoSuchProject` and friends ride
    /// here as answers, not as malfunctions.
    Refused { code: String, message: String },
    /// The budget expired before the CLI answered.
    Timeout { call: String, budget: Duration },
    /// The CLI could not be spawned at all (a bad `FLEET_LAYERFS_BIN`).
    Spawn(String),
    /// The CLI answered, but stdout was not the promised JSON.
    Decode { call: String, body: String },
    /// The CLI exited nonzero with no refusal the driver recognizes —
    /// a wrong command line (exit 2) is a fleet bug and reads as one.
    Failed {
        call: String,
        exit: i32,
        stderr: String,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Absent(reason) => write!(f, "layerfs: {reason}"),
            Error::Wire { answered } => write!(
                f,
                "layerfs: the CLI answers wire {answered}, fleet drives {SUPPORTED_WIRE} — \
                 the versions must come from one pinned checkout"
            ),
            Error::Refused { code, message } => write!(f, "layerfs refused [{code}]: {message}"),
            Error::Timeout { call, budget } => write!(
                f,
                "layerfs: {call} did not answer within {}s — a hung Docker daemon is the \
                 usual cause; the call was made once and never retried",
                budget.as_secs()
            ),
            Error::Spawn(reason) => write!(f, "layerfs: the CLI could not be spawned: {reason}"),
            Error::Decode { call, body } => {
                write!(
                    f,
                    "layerfs: {call} answered no JSON the driver knows: {body}"
                )
            }
            Error::Failed { call, exit, stderr } => write!(
                f,
                "layerfs: {call} exited {exit} with no refusal the driver recognizes: {stderr}"
            ),
        }
    }
}

impl std::error::Error for Error {}

/// Cut a body to the error bound, naming the cut rather than hiding it.
fn bounded(text: &str) -> String {
    let trimmed = text.trim();
    let mut cut = String::new();
    for (index, ch) in trimmed.char_indices() {
        if index >= BOUND {
            cut.push_str(" \u{2026}");
            return cut;
        }
        cut.push(ch);
    }
    cut
}

/// The opt-in knob: `Ok(None)` unset (the default — the feature is off
/// and the driver is never consulted), `Ok(Some)` a valid LayerFS
/// project name, `Err` an invalid one — named, never silently off,
/// because a knob the owner set wrong must answer rather than vanish.
///
/// The grammar is the CLI's own (1–63 lowercase ASCII letters,
/// digits, `_`, `-`, `.`, alphanumeric at both ends).
///
/// Tests set [`set_project_for_test`] instead of the environment: the
/// knob is process-global as env, and one test's env window would
/// switch the arm on inside another test's delegation running on a
/// different thread (observed: a child pwd'd into a concurrent test's
/// mount). The override lives on the test's own thread — libtest runs
/// each test on its own, so it dies with the test — and `cfg(test)`
/// compiles it out of every production build entirely.
pub fn project_name() -> Result<Option<String>, String> {
    if let Some(name) = PROJECT_OVERRIDE.with(|slot| slot.borrow().clone()) {
        return validated(name);
    }
    let Some(raw) = std::env::var_os(PROJECT_ENV) else {
        return Ok(None);
    };
    validated(raw.to_string_lossy().trim().to_string())
}

/// One validation path for both sources: empty reads as unset, the
/// grammar decides the rest, and an invalid name is a named `Err`.
fn validated(name: String) -> Result<Option<String>, String> {
    if name.is_empty() {
        return Ok(None);
    }
    if name_fits(&name) {
        Ok(Some(name))
    } else {
        Err(format!(
            "{PROJECT_ENV} names `{name}`, which is not a LayerFS project name — 1 to 63 \
             lowercase letters, digits, `_`, `-` or `.`, starting and ending alphanumeric"
        ))
    }
}

/// The machine's LayerFS substrate facts, recorded by `fleet install`
/// under `<FLEET_HOME>/config.json`'s `layerfs` object: where the
/// LayerFS home lives, which sealed bundle carries the CLI, the
/// daemon and the worker, which runtime image the Sandboxes run. One
/// resolution per process. A machine that has none gets the named
/// absence with the provision command (the install phase's voice),
/// never a guess.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Machine {
    pub home: PathBuf,
    pub bundle: PathBuf,
    pub image: String,
}

/// One folder's registration — the name its LayerFS project answers
/// to and the Store that project installed. Written by registration
/// (the seamless plan's C2), read by the arm so a run launched in
/// that folder needs no fact of its own.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Registration {
    pub name: String,
    pub store: PathBuf,
    /// The git tree id the declared import staged — the fork base's
    /// provenance, the sync's diff anchor. `None` for a non-git
    /// folder's whole-tree import: no tree, no sync, named once.
    pub tree: Option<String>,
}

/// Where the LayerFS home lives: `<FLEET_HOME>/layerfs-home` — fleet's
/// own home, so the substrate's records live beside the machine's and
/// never in a repo's `tmp/`.
pub fn machine_home_dir() -> PathBuf {
    crate::home::fleet_home(
        std::env::var("FLEET_HOME").ok().as_deref(),
        std::env::var("HOME").ok().as_deref(),
    )
    .map(|home| home.join("layerfs-home"))
    .unwrap_or_else(|_| PathBuf::from("layerfs-home"))
}

/// The machine's facts, read once per process from `<FLEET_HOME>`'s
/// config. The named absence says what to run, the way a lane's
/// absence names its home.
pub fn machine() -> Result<&'static Machine, String> {
    static MACHINE: std::sync::OnceLock<Result<Machine, String>> = std::sync::OnceLock::new();
    MACHINE
        .get_or_init(read_machine)
        .as_ref()
        .map_err(|reason| reason.clone())
}

/// The machine's facts, read FRESH from the config — the process
/// cache answers whichever config the first caller saw, and a face
/// serving several folders cannot afford that. Tests and the view's
/// reads use this one.
pub fn read_machine() -> Result<Machine, String> {
    let home = crate::home::fleet_home(
        std::env::var("FLEET_HOME").ok().as_deref(),
        std::env::var("HOME").ok().as_deref(),
    )?;
    let config = std::fs::read_to_string(home.join("config.json")).map_err(|_| {
        "no <FLEET_HOME>/config.json — `fleet install` provisions the LayerFS substrate".to_string()
    })?;
    let document: serde_json::Value = serde_json::from_str(&config)
        .map_err(|error| format!("config.json is not JSON: {error}"))?;
    let layerfs = document.get("layerfs").ok_or_else(|| {
        "config.json carries no `layerfs` section — `fleet install` provisions the substrate"
            .to_string()
    })?;
    let text = |key: &str| {
        layerfs
            .get(key)
            .and_then(|value| value.as_str())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
            .ok_or_else(|| format!("the `layerfs.{key}` fact is missing — re-run `fleet install`"))
    };
    let home = PathBuf::from(text("home")?);
    let bundle = PathBuf::from(text("bundle")?);
    if !home.is_dir() {
        return Err(format!(
            "the recorded LayerFS home {} is gone — re-run `fleet install`",
            home.display()
        ));
    }
    if !bundle.is_dir() {
        return Err(format!(
            "the recorded bundle {} is gone — re-run `fleet install`",
            bundle.display()
        ));
    }
    Ok(Machine {
        home,
        bundle,
        image: text("image")?,
    })
}

/// Every folder the machine has registered, read fresh from the
/// config — the tests read this, since the process cache answers
/// whichever config the first caller saw.
pub fn read_registrations() -> Vec<(String, Registration)> {
    let Ok(machine_home) = crate::home::fleet_home(
        std::env::var("FLEET_HOME").ok().as_deref(),
        std::env::var("HOME").ok().as_deref(),
    ) else {
        return Vec::new();
    };
    let Ok(document) = serde_json::from_str::<serde_json::Value>(
        &std::fs::read_to_string(machine_home.join("config.json")).unwrap_or_default(),
    ) else {
        return Vec::new();
    };
    let Some(projects) = document
        .get("layerfs")
        .and_then(|layerfs| layerfs.get("projects"))
        .and_then(|projects| projects.as_object())
    else {
        return Vec::new();
    };
    projects
        .iter()
        .filter_map(|(folder, record)| {
            let name = record.get("name")?.as_str()?.to_string();
            let store = record.get("store")?.as_str().map(PathBuf::from)?;
            Some((
                folder.clone(),
                Registration {
                    name,
                    store,
                    tree: record
                        .get("tree")
                        .and_then(|tree| tree.as_str())
                        .map(str::to_string),
                },
            ))
        })
        .collect()
}

/// Every folder the machine has registered, cached once per process
/// — the arm's read; the config does not change under a running
/// fleet except through registration itself, which records before it
/// is consulted again.
pub fn registrations() -> &'static [(String, Registration)] {
    static REGISTRATIONS: std::sync::OnceLock<Vec<(String, Registration)>> =
        std::sync::OnceLock::new();
    REGISTRATIONS.get_or_init(read_registrations)
}

/// One folder's registration, matched on the canonical path (a
/// symlinked visit registers once, not twice).
pub fn registration(folder: &std::path::Path) -> Option<&'static Registration> {
    find_registration(registrations(), folder)
}

/// The lookup itself, over any map — the cached read's body, and the
/// tests' way to check a config they just wrote.
pub fn find_registration<'a>(
    map: &'a [(String, Registration)],
    folder: &std::path::Path,
) -> Option<&'a Registration> {
    let canonical = folder
        .canonicalize()
        .unwrap_or_else(|_| folder.to_path_buf());
    map.iter()
        .find(|(path, _)| Path::new(path) == canonical.as_path())
        .map(|(_, registration)| registration)
}

/// The composed environment the CLI needs — endpoint derived from
/// the home, daemon from the bundle, image from the record, store
/// path from the project's registration — set into the process once
/// so every spawn inherits: the seven manual exports retire. Values
/// are machine paths and a digest, never secrets.
pub fn apply_env(machine: &Machine, registration: Option<&Registration>) {
    let set = |name: &str, value: &std::path::Path| {
        std::env::set_var(name, value.as_os_str());
    };
    set("LAYERFS_HOME", &machine.home);
    set(
        "LAYERFS_WORKER_ENDPOINT",
        &machine.home.join("worker").join("endpoint.json"),
    );
    set("LAYERFS_IMAGE", Path::new(&machine.image));
    set("LAYERFS_DAEMON", &machine.bundle.join("layerfs-daemon"));
    if let Some(registration) = registration {
        set("LAYERFS_STORE_PATH", &registration.store);
        // The Store's Docker volume — the wire's `deploy.volume`, which
        // a Workspace's Sandbox refuses to start without. It is the
        // store path's own volume name, the same derivation the
        // doctor's volume check makes.
        set(
            "LAYERFS_VOLUME",
            Path::new(&store_volume(&registration.store)),
        );
    }
}

/// The launch folder's declared content hashes, the settle's record of
/// what the host looked like when the child forked: a path whose hash
/// moved since then was touched by the owner (or the parent) and is
/// never overwritten by an apply. `None` for a folder that is not a git
/// repository — the whole-tree walk has no cheap snapshot, and a folder
/// that cannot be snapshotted gets no home apply at all (its delta
/// stays in the Store, named in the result).
///
/// The hash is std's SipHash over the file's bytes and length: this is
/// change detection inside one delegation, not a cryptographic
/// boundary, and it is documented as such rather than borrowed
/// vocabulary.
pub fn content_hashes(folder: &Path) -> Option<std::collections::BTreeMap<String, String>> {
    let (declared, _skipped) = declared_files(folder)?;
    let mut hashes = std::collections::BTreeMap::new();
    for entry in &declared {
        let path = folder.join(&entry.path);
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        std::hash::Hasher::write(&mut hasher, &bytes);
        std::hash::Hasher::write_usize(&mut hasher, bytes.len());
        hashes.insert(
            entry.path.clone(),
            format!("{:016x}", std::hash::Hasher::finish(&hasher)),
        );
    }
    Some(hashes)
}

/// The host's hash for one path NOW — `None` when the file is absent,
/// which is how a Removed path reads.
fn host_hash(folder: &Path, path: &str) -> Option<String> {
    let bytes = std::fs::read(folder.join(path)).ok()?;
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    std::hash::Hasher::write(&mut hasher, &bytes);
    std::hash::Hasher::write_usize(&mut hasher, bytes.len());
    Some(format!("{:016x}", std::hash::Hasher::finish(&hasher)))
}

/// The tracked worktree's content tree, via a temp index: copy the
/// repo's index, `add -u` (updates tracked paths from the worktree,
/// drops deletions), `write-tree`. Milliseconds warm, and the id
/// changes iff tracked CONTENT changes — the sync's cheap anchor.
/// `None` when the folder is not a git repository.
pub fn worktree_tree(folder: &Path) -> Option<String> {
    let git_dir = folder.join(".git");
    let index_source = if git_dir.is_dir() {
        git_dir.join("index")
    } else {
        // A worktree link's gitdir lives in the .git file; the sync is
        // best-effort and its absence reads as "no tree".
        return None;
    };
    let temp = std::env::temp_dir().join(format!(
        "fleet-tree-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::copy(&index_source, &temp).ok()?;
    let run = |args: &[&str]| -> Result<String, String> {
        let output = Command::new("git")
            .arg("-C")
            .arg(folder)
            .args(args)
            .env("GIT_INDEX_FILE", &temp)
            .output()
            .map_err(|error| format!("git {args:?}: {error}"))?;
        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    };
    let refreshed = run(&["add", "-u"]);
    let tree = run(&["write-tree"]);
    let _ = std::fs::remove_file(&temp);
    match (refreshed, tree) {
        (_, Ok(tree)) if !tree.is_empty() => Some(tree),
        (Err(reason), _) | (_, Err(reason)) => {
            eprintln!("fleet: layerfs: the sync's tree read failed: {reason}");
            None
        }
        _ => None,
    }
}

/// The tracked delta between two trees: `(path, kind)` with the git
/// statuses that matter to the sync — A (added on the host), M
/// (modified), D (deleted).
pub fn tree_delta(folder: &Path, from_tree: &str) -> Option<Vec<(String, char)>> {
    let current = worktree_tree(folder)?;
    let output = Command::new("git")
        .arg("-C")
        .arg(folder)
        .args(["diff", "--name-status", from_tree, &current])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let mut delta = Vec::new();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let mut parts = line.split('\t');
        let Some(status) = parts.next() else {
            continue;
        };
        let Some(path) = parts.next() else {
            continue;
        };
        let kind = match status.chars().next() {
            Some('A') => 'A',
            Some('D') => 'D',
            _ => 'M',
        };
        delta.push((path.to_string(), kind));
    }
    Some(delta)
}

/// The spawn sync's decision, from the git delta and main's versions:
/// which delta paths to carry onto the fresh fork (the host's change is
/// newer — main still holds the recorded version), and which are
/// main-newer (another child's merged work the host hasn't received —
/// skipped here, reconciled home by the settle). One fs read per
/// candidate; the delta itself is the git diff.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncPlan {
    /// Paths to carry onto the child's branch, with the host's kind.
    pub to_child: Vec<(String, char)>,
    /// Paths main moved (another child's merge): left for the settle's
    /// reconciliation.
    pub main_newer: Vec<String>,
}

/// Classify the delta. The recorded tree is what was imported; main's
/// version per candidate decides the side:
///   main == recorded — main is stale here, the host's change is
///     newer: carry it onto the child;
///   main != recorded — main moved: skip, the settle reconciles.
pub fn plan_sync(
    gate: &Gate,
    project: &str,
    folder: &Path,
    recorded_tree: &str,
) -> Option<SyncPlan> {
    let delta = tree_delta(folder, recorded_tree)?;
    let mut plan = SyncPlan {
        to_child: Vec::new(),
        main_newer: Vec::new(),
    };
    for (path, kind) in delta {
        // main's version, read through the branch view's root.
        let main_version = gate
            .fs_read(project, "main", &format!("/{path}"), None)
            .ok()
            .and_then(|document| {
                document
                    .get("text")
                    .and_then(|text| text.as_str())
                    .map(str::to_string)
            });
        let recorded_version = recorded_content(folder, recorded_tree, &path);
        let main_hash = main_version.as_deref().map(hash_text);
        let recorded_hash = recorded_version.as_deref().map(hash_text);
        if main_hash != recorded_hash {
            plan.main_newer.push(path);
        } else {
            plan.to_child.push((path, kind));
        }
    }
    Some(plan)
}

/// The recorded tree's content for one path — git's own object store,
/// `None` when the tree lacks the path (an Added-on-the-host candidate).
/// The read runs in the FOLDER's repository: the tree object lives in
/// its own .git, and a cat-file from another cwd names nothing.
fn recorded_content(folder: &Path, tree: &str, path: &str) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(folder)
        .args(["cat-file", "blob", &format!("{tree}:{path}")])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).to_string())
}

/// The Store's Docker volume, derived from the store path a/// The Store's Docker volume, derived from the store path a
/// registration records: `/var/lib/docker/volumes/<name>/_data/global/
/// store.sqlite` names the volume every Sandbox mounts. Empty when the
/// path is not that shape — the wire refuses by name, which is the
/// honest answer for a store path that is not a Docker volume's.
pub fn store_volume(store: &Path) -> String {
    store
        .to_string_lossy()
        .split("/volumes/")
        .nth(1)
        .and_then(|rest| rest.split('/').next())
        .unwrap_or_default()
        .to_string()
}

/// The wire's own name rule — `HistoryName` in layerfs-history: 1 to
/// 63 ASCII bytes, lowercase letters, digits, `.`, `_`, `-`,
/// alphanumeric at both ends. Branch names and project names share
/// it; the live run taught that assuming otherwise costs a real
/// delegation (`fleet/<session>` with its `/` and uppercase stamp was
/// refused by the wire while the stub accepted it).
fn name_fits(name: &str) -> bool {
    let bytes = name.as_bytes();
    let body = |byte: u8| {
        byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'_' | b'-')
    };
    let ends = |byte: u8| byte.is_ascii_lowercase() || byte.is_ascii_digit();
    !bytes.is_empty()
        && bytes.len() <= 63
        && ends(bytes[0])
        && ends(bytes[bytes.len() - 1])
        && bytes.iter().all(|&byte| body(byte))
}

/// The rule, as the driver's named refusal: checked before a spawn so
/// an illegal name is an answer here, never a wasted round trip.
pub fn history_name_ok(name: &str) -> Result<(), String> {
    if name_fits(name) {
        Ok(())
    } else {
        Err(format!(
            "`{name}` is not a valid LayerFS name — 1 to 63 ASCII bytes, lowercase letters, \
             digits, `.`, `_` or `-`, starting and ending alphanumeric (the wire's own rule)"
        ))
    }
}

/// Fold any raw text into the wire's grammar within `budget` bytes:
/// lowercase, illegal runs collapse to one `-`, trimmed at both ends,
/// truncated at the budget with a trailing `-` re-trimmed so the
/// result always ends alphanumeric.
pub fn fold_history_name(raw: &str, budget: usize) -> String {
    let mut folded = String::with_capacity(raw.len());
    let mut last_dash = false;
    for raw_ch in raw.chars() {
        // Lowercase first — a session stamp's `T`/`Z` become readable
        // letters, not dashes — then map anything outside the grammar.
        let ch = raw_ch.to_ascii_lowercase();
        let keep = ch.is_ascii_lowercase() || ch.is_ascii_digit() || matches!(ch, '.' | '_');
        let mapped = if keep { ch } else { '-' };
        if mapped == '-' {
            if !last_dash && !folded.is_empty() {
                folded.push('-');
            }
            last_dash = true;
        } else {
            folded.push(mapped);
            last_dash = false;
        }
    }
    let mut cut: String = folded.chars().take(budget).collect();
    while cut
        .chars()
        .last()
        .is_some_and(|ch| !ch.is_ascii_lowercase() && !ch.is_ascii_digit())
    {
        cut.pop();
    }
    // A budget-0 or all-illegal input can cut to nothing; the prefix
    // callers always prepend keeps the final name non-empty.
    cut
}

/// The child's Branch from its session id: session stamps carry
/// uppercase (`20261025T183442Z-…`), which the wire refuses — folded
/// here so every arm fork is valid by construction, bounded to 63
/// with the `fleet-child-` prefix.
pub fn child_branch_name(session: &str) -> String {
    const PREFIX: &str = "fleet-child-";
    let folded = fold_history_name(session, 63 - PREFIX.len());
    format!("{PREFIX}{folded}")
}

/// The integration Branch a settled child merges through, folded to
/// the same rule: `merge-` plus the child's branch, bounded to 63.
pub fn merge_branch_name(child: &str) -> String {
    const PREFIX: &str = "merge-";
    let folded = fold_history_name(child, 63 - PREFIX.len());
    format!("{PREFIX}{folded}")
}

/// Arm the knob for THIS thread only. Test support: every test gets
/// its own thread (libtest) and its own default, so nothing resets —
/// and production never calls this, which is why the read sits in
/// front of the environment as a plain thread-local with a `None`
/// default: the production path is byte-identical to an env-only
/// read.
pub fn set_project_for_test(name: &str) {
    PROJECT_OVERRIDE.with(|slot| *slot.borrow_mut() = Some(name.to_string()));
}

/// The test thread's override, when one is armed — the arm's
/// resolution reads it first, before any machine fact. Production
/// never arms it, so the thread-local reads `None` and the
/// resolution falls through to the machine's facts.
pub fn test_project() -> Option<String> {
    PROJECT_OVERRIDE.with(|slot| slot.borrow().clone())
}

/// The test thread's folder, when one is set — the arm's resolution
/// reads it before the process's cwd, so a test can register a folder
/// without moving the process-global working directory a concurrent
/// test would read. Production never sets it, so the resolution
/// falls through to the cwd exactly as before.
pub fn set_folder_for_test(folder: &Path) {
    FOLDER_OVERRIDE.with(|slot| *slot.borrow_mut() = Some(folder.to_path_buf()));
}

/// The test thread's folder, when one is set.
pub fn test_folder() -> Option<PathBuf> {
    FOLDER_OVERRIDE.with(|slot| slot.borrow().clone())
}

thread_local! {
    static FOLDER_OVERRIDE: std::cell::RefCell<Option<PathBuf>> =
        const { std::cell::RefCell::new(None) };
}

thread_local! {
    static PROJECT_OVERRIDE: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) };
}

/// What one `workspace mount` answered: the sandbox it lives in, the
/// mount path inside that Sandbox, and the CLI's own `ws` handle
/// (`<sandbox>:<path>`) that commit and unmount take.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mount {
    pub sandbox: String,
    pub path: PathBuf,
    pub ws: String,
}

impl std::fmt::Display for RegisterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RegisterError::Absent(reason) => write!(f, "{reason}"),
            RegisterError::OverBound { .. } => {
                write!(f, "the folder is past the registration bound")
            }
            RegisterError::Failed(reason) => write!(f, "{reason}"),
        }
    }
}

/// C2's bound, set by C0's measurement: a real repo (1,236 entries /
/// 18.7 MB, `.git` included) registered in 2.5 s — and the runtime's
/// import has no excludes (`scanSource` walks every entry), so a
/// build tree is swallowed whole. These two numbers give real repos
/// headroom and refuse the build-tree class; a folder past either is
/// the named fallback, never a blind import.
pub const REGISTRATION_MAX_BYTES: u64 = 2 * 1024 * 1024 * 1024;
pub const REGISTRATION_MAX_ENTRIES: u64 = 100_000;

/// What a registration attempt answers. The named fallback is a
/// first-class outcome: the delegation runs the way it does without
/// the substrate, with the reason and the next step riding one
/// owner-facing notice — never a silent import, never a failed run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistrationOutcome {
    /// Registered (or already registered): the folder's project, and
    /// the declared paths the import could not carry (gitlinks — a
    /// submodule's contents live in their own checkout), for the one
    /// notice that names them.
    Registered {
        registration: Registration,
        skipped: Vec<String>,
    },
    /// Past the bound: the shape that crossed it, for the notice.
    OverBound { entries: u64, bytes: u64 },
}

/// Why a registration could not be attempted at all — every one
/// names what to do next.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegisterError {
    /// The machine has no recorded facts, or the register script is
    /// not installed: `fleet install` provisions the substrate.
    Absent(String),
    /// The folder is past the bound: the shape that crossed it rides
    /// the notice — the named fallback, never a blind import.
    OverBound { entries: u64, bytes: u64 },
    /// The registration ran and refused, or answered something the
    /// driver does not know.
    Failed(String),
}

/// The folder's shape — entries and regular-file bytes — with the
/// early exit the bound exists for: a walk that crosses either bound
/// stops and answers `OverBound` rather than measuring the whole
/// tree. Links are not followed, mirroring the runtime's own scan.
fn folder_shape_with(
    folder: &Path,
    max_bytes: u64,
    max_entries: u64,
) -> Result<(u64, u64), RegisterError> {
    let mut entries: u64 = 0;
    let mut bytes: u64 = 0;
    let mut stack = vec![folder.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let reader = std::fs::read_dir(&dir)
            .map_err(|error| RegisterError::Failed(format!("{}: {error}", dir.display())))?;
        for entry in reader {
            let entry = entry.map_err(|error| RegisterError::Failed(error.to_string()))?;
            entries += 1;
            if entries > max_entries {
                return Err(RegisterError::OverBound { entries, bytes });
            }
            let file_type = entry
                .file_type()
                .map_err(|error| RegisterError::Failed(error.to_string()))?;
            if file_type.is_dir() {
                stack.push(entry.path());
            } else if file_type.is_file() {
                bytes += entry
                    .metadata()
                    .map_err(|error| RegisterError::Failed(error.to_string()))?
                    .len();
                if bytes > max_bytes {
                    return Err(RegisterError::OverBound { entries, bytes });
                }
            }
        }
    }
    Ok((entries, bytes))
}

/// The bound's pre-check at its production values.
fn folder_shape(folder: &Path) -> Result<(u64, u64), RegisterError> {
    folder_shape_with(folder, REGISTRATION_MAX_BYTES, REGISTRATION_MAX_ENTRIES)
}

/// The project name a folder registers under: its basename folded
/// into the wire's grammar, de-duplicated against the names already
/// registered (two folders called `api` are `api` and `api-2`).
fn project_name_for(folder: &Path, taken: &[String]) -> String {
    let raw = folder
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "project".to_string());
    let folded = fold_history_name(&raw.to_lowercase(), 63);
    let base = if folded.is_empty() {
        "project".to_string()
    } else {
        folded
    };
    if !taken.iter().any(|name| name == &base) {
        return base;
    }
    for suffix in 2..=999u32 {
        let candidate = format!("{base}-{suffix}");
        if !taken.iter().any(|name| name == &candidate) {
            return candidate;
        }
    }
    format!("{base}-{}", std::process::id())
}

/// The three scripts the driver spawns, by the name this repository
/// ships them under in `scripts/` — the install copies them into the
/// home by the SAME name and the driver resolves them by it. The live
/// run taught why this is one constant rather than three literals: the
/// install copied `layerfs-register.mjs` while the driver looked for
/// `register.mjs`, so every clean-environment registration failed by
/// name while the tests (which wrote their own fixture's short name)
/// stayed green.
pub const REGISTER_SCRIPT: &str = "layerfs-register.mjs";
pub const HISTORY_SCRIPT: &str = "layerfs-history.mjs";
pub const WORKER_SCRIPT: &str = "layerfs-worker.mjs";

/// The register script's location: the home's own copy (install puts
/// it there), with the override for tests and operators.
fn register_script(machine: &Machine) -> Result<PathBuf, RegisterError> {
    if let Some(path) = std::env::var_os("FLEET_LAYERFS_REGISTER")
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
    {
        return Ok(path);
    }
    let script = machine.home.join(REGISTER_SCRIPT);
    if script.is_file() {
        Ok(script)
    } else {
        Err(RegisterError::Absent(format!(
            "the register script is not at {} — `fleet install` provisions the substrate",
            script.display()
        )))
    }
}

/// Register one folder: the bound's pre-check, the name, the
/// runtime's Init through the register script, and the record —
/// one attempt, and the config write under the corpus's own
/// per-path lock so two runs registering at once cannot interleave.
/// One path git declares, with how it must be carried: a regular file
/// copied as bytes, a symlink recreated as the link it is (the Store's
/// view carries links, not their targets), a gitlink SKIPPED (a
/// submodule's contents live in their own checkout — the staged tree
/// names the gap rather than importing a half-world).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Declared {
    pub path: String,
    pub kind: DeclaredKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeclaredKind {
    File,
    Symlink,
}

/// What the folder's own rules say to carry: `git ls-files --stage`
/// parsed NUL-safely (paths ride the tab, never a shell split). `None`
/// for a folder that is not a git repository — the whole-tree walk
/// stays that folder's answer, with the bound as its guard.
pub fn declared_files(folder: &Path) -> Option<(Vec<Declared>, Vec<String>)> {
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(folder)
        .args(["ls-files", "--stage", "-z"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let mut declared = Vec::new();
    let mut skipped = Vec::new();
    for record in output.stdout.split(|byte| *byte == 0) {
        if record.is_empty() {
            continue;
        }
        let text = String::from_utf8_lossy(record);
        let Some((meta, path)) = text.split_once('\t') else {
            continue;
        };
        let mode = meta.split(' ').next().unwrap_or_default();
        let path = path.to_string();
        match mode {
            "120000" => declared.push(Declared {
                path,
                kind: DeclaredKind::Symlink,
            }),
            "160000" => skipped.push(path),
            _ => declared.push(Declared {
                path,
                kind: DeclaredKind::File,
            }),
        }
    }
    Some((declared, skipped))
}

/// The declared set's shape: entries counted, bytes statted — the
/// bound applies to what will actually be imported, so an ignored
/// build tree never counts against the repo that declares around it.
fn declared_shape(declared: &[Declared], folder: &Path) -> Result<(u64, u64), RegisterError> {
    let mut bytes: u64 = 0;
    for entry in declared {
        let meta = folder
            .join(&entry.path)
            .symlink_metadata()
            .map_err(|error| RegisterError::Failed(format!("{}: {error}", entry.path)))?;
        bytes += meta.len();
    }
    Ok((declared.len() as u64, bytes))
}

/// Stage the declared paths into a scratch dir under the LayerFS home:
/// the Init imports the scratch, so the Store carries exactly what the
/// repo declares. The caller removes the dir on EVERY path — success,
/// refusal, or failure — with [`remove_staging`].
fn stage_declared(
    machine: &Machine,
    folder: &Path,
    declared: &[Declared],
    name: &str,
) -> Result<std::path::PathBuf, RegisterError> {
    let staging = machine
        .home
        .join("staging")
        .join(format!("{name}-{}", std::process::id()));
    std::fs::create_dir_all(&staging)
        .map_err(|error| RegisterError::Failed(format!("the staging dir: {error}")))?;
    for entry in declared {
        let from = folder.join(&entry.path);
        let to = staging.join(&entry.path);
        if let Some(parent) = to.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| RegisterError::Failed(format!("{}: {error}", entry.path)))?;
        }
        let staged = match entry.kind {
            DeclaredKind::File => std::fs::copy(&from, &to).map(|_| ()),
            DeclaredKind::Symlink => {
                let target = std::fs::read_link(&from)
                    .map_err(|error| RegisterError::Failed(format!("{}: {error}", entry.path)))?;
                #[cfg(unix)]
                {
                    std::os::unix::fs::symlink(target, &to)
                }
                #[cfg(not(unix))]
                {
                    std::fs::copy(&from, &to).map(|_| ())
                }
            }
        };
        staged.map_err(|error| RegisterError::Failed(format!("{}: {error}", entry.path)))?;
    }
    Ok(staging)
}

/// The staging dir's removal, on every path — the scratch outlives no
/// outcome.
fn remove_staging(staging: &std::path::Path) {
    let _ = std::fs::remove_dir_all(staging);
}

pub fn register(machine: &Machine, folder: &Path) -> Result<RegistrationOutcome, RegisterError> {
    let script = register_script(machine)?;
    let taken: Vec<String> = read_registrations()
        .iter()
        .map(|(_, registration)| registration.name.clone())
        .collect();
    // The declared set decides the shape: a git repo's bound counts
    // what git tracks, so an ignored build tree never refuses the repo
    // that declares around it. Nothing tracked is the named fallback —
    // there is nothing to isolate a child inside.
    let declared = declared_files(folder);
    let (_entries, _bytes, skipped, staging) = match &declared {
        Some((files, skipped)) if files.is_empty() => {
            let _ = skipped;
            return Err(RegisterError::Failed(
                "the repository declares no files to carry (git tracks nothing here) —                  the folder runs without LayerFS workspaces"
                    .to_string(),
            ));
        }
        Some((files, skipped)) => {
            let (entries, bytes) = declared_shape(files, folder)?;
            if entries > REGISTRATION_MAX_ENTRIES || bytes > REGISTRATION_MAX_BYTES {
                return Err(RegisterError::OverBound { entries, bytes });
            }
            let staging = stage_declared(machine, folder, files, "pending")?;
            (entries, bytes, skipped.clone(), Some(staging))
        }
        None => {
            let (entries, bytes) = folder_shape(folder)?;
            (entries, bytes, Vec::new(), None)
        }
    };
    let name = project_name_for(folder, &taken);
    // The staging dir's name carries the project once it is known;
    // rename so a failed run's scratch is still identifiable.
    let staging = match staging {
        Some(dir) => {
            let named = machine
                .home
                .join("staging")
                .join(format!("{name}-{}", std::process::id()));
            let _ = std::fs::rename(&dir, &named);
            Some(named)
        }
        None => None,
    };
    let source = staging.clone().unwrap_or_else(|| folder.to_path_buf());
    let outcome = register_inner(machine, &script, &name, &source, folder);
    if let Some(dir) = &staging {
        remove_staging(dir);
    }
    let registration = outcome?;
    Ok(RegistrationOutcome::Registered {
        registration,
        skipped,
    })
}

/// The script's one attempt, from the source the caller chose — the
/// staging dir for a declared import, the folder itself for the
/// whole-tree walk.
fn register_inner(
    machine: &Machine,
    script: &Path,
    name: &str,
    source: &Path,
    folder: &Path,
) -> Result<Registration, RegisterError> {
    let output = std::process::Command::new("node")
        .args([
            script.to_string_lossy().as_ref(),
            name,
            source.to_string_lossy().as_ref(),
            machine.home.to_string_lossy().as_ref(),
            machine.bundle.to_string_lossy().as_ref(),
            &machine.image,
        ])
        .output()
        .map_err(|error| {
            RegisterError::Failed(format!("the register script could not run: {error}"))
        })?;
    if !output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(RegisterError::Failed(format!(
            "registration failed ({}): {}",
            stdout.trim(),
            stderr.trim()
        )));
    }
    let document: serde_json::Value =
        serde_json::from_str(std::str::from_utf8(&output.stdout).map_err(|error| {
            RegisterError::Failed(format!("the register answer was not UTF-8: {error}"))
        })?)
        .map_err(|error| {
            RegisterError::Failed(format!("the register answer was not JSON: {error}"))
        })?;
    let error = document
        .get("error")
        .and_then(|error| error.as_str())
        .map(str::to_string);
    if let Some(error) = error {
        return Err(RegisterError::Failed(error));
    }
    let volume = document
        .get("volume")
        .and_then(|volume| volume.as_str())
        .map(str::to_string)
        .ok_or_else(|| RegisterError::Failed("the register answer named no volume".to_string()))?;
    let store = PathBuf::from(
        document
            .get("store")
            .and_then(|store| store.as_str())
            .ok_or_else(|| {
                RegisterError::Failed("the register answer named no store".to_string())
            })?,
    );
    // The imported tree, recorded with the registration: the fork
    // base's provenance and the spawn sync's diff anchor. A non-git
    // folder's whole-tree import has none (no tree, no sync).
    let tree = worktree_tree(folder);
    let registration = Registration {
        name: name.to_string(),
        store,
        tree,
    };
    record_registration(folder, &registration, &volume)?;
    Ok(registration)
}

/// Write the folder's registration into `<FLEET_HOME>/config.json`
/// under the per-path lock — the same exclusive lock the write path
/// uses, so two registrations cannot interleave a read-modify-write.
fn record_registration(
    folder: &Path,
    registration: &Registration,
    volume: &str,
) -> Result<(), RegisterError> {
    let home = crate::home::fleet_home(
        std::env::var("FLEET_HOME").ok().as_deref(),
        std::env::var("HOME").ok().as_deref(),
    )
    .map_err(RegisterError::Absent)?;
    let config = home.join("config.json");
    let _guard = crate::filelock::FileLock::take(&crate::filelock::FileLock::beside(&config));
    let mut document: serde_json::Value = std::fs::read_to_string(&config)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_else(|| serde_json::json!({}));
    let canonical = folder
        .canonicalize()
        .unwrap_or_else(|_| folder.to_path_buf());
    let root = document
        .as_object_mut()
        .ok_or_else(|| RegisterError::Failed("config.json is not an object".to_string()))?;
    let layerfs = root
        .entry("layerfs")
        .or_insert_with(|| serde_json::json!({}));
    let layerfs = layerfs.as_object_mut().ok_or_else(|| {
        RegisterError::Failed("config.json's `layerfs` is not an object".to_string())
    })?;
    let projects = layerfs
        .entry("projects")
        .or_insert_with(|| serde_json::json!({}));
    let projects = projects.as_object_mut().ok_or_else(|| {
        RegisterError::Failed("config.json's `layerfs.projects` is not an object".to_string())
    })?;
    projects.insert(
        canonical.to_string_lossy().into_owned(),
        serde_json::json!({
            "name": registration.name,
            "volume": volume,
            "store": registration.store.to_string_lossy(),
            "tree": registration.tree,
        }),
    );
    std::fs::write(
        &config,
        serde_json::to_string_pretty(&document).unwrap_or_default(),
    )
    .map_err(|error| RegisterError::Failed(format!("config.json could not be written: {error}")))?;
    Ok(())
}

/// Record the machine's facts — the home, the bundle, the runtime
/// image — into `<FLEET_HOME>/config.json` under the same per-path
/// lock and the same merge `record_registration` uses: the `layerfs`
/// section beside whatever the file already carries, so the scaffold
/// the install wrote a moment earlier and an owner's hand-edits both
/// survive. One writer for the section, whatever writes it.
pub fn record_machine(home: &Path, bundle: &Path, image: &str) -> Result<(), String> {
    let fleet_home = crate::home::fleet_home(
        std::env::var("FLEET_HOME").ok().as_deref(),
        std::env::var("HOME").ok().as_deref(),
    )?;
    let config = fleet_home.join("config.json");
    let _guard = crate::filelock::FileLock::take(&crate::filelock::FileLock::beside(&config));
    let mut document: serde_json::Value = std::fs::read_to_string(&config)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_else(|| serde_json::json!({}));
    let root = document
        .as_object_mut()
        .ok_or_else(|| "config.json is not an object".to_string())?;
    let layerfs = root
        .entry("layerfs")
        .or_insert_with(|| serde_json::json!({}));
    let layerfs = layerfs
        .as_object_mut()
        .ok_or_else(|| "config.json's `layerfs` is not an object".to_string())?;
    layerfs.insert(
        "home".to_string(),
        serde_json::json!(home.display().to_string()),
    );
    layerfs.insert(
        "bundle".to_string(),
        serde_json::json!(bundle.display().to_string()),
    );
    layerfs.insert("image".to_string(), serde_json::json!(image));
    std::fs::write(
        &config,
        serde_json::to_string_pretty(&document).unwrap_or_default(),
    )
    .map_err(|error| format!("config.json could not be written: {error}"))
}

/// Advance the recorded tree to the host's current one — after an
/// apply-home or a sync, main and the host agree on the touched paths,
/// and the next spawn's delta diffs against what was actually
/// reconciled. Fresh reads, the corpus lock, the explicit path.
pub fn record_tree(folder: &Path) -> Result<(), String> {
    let Some(tree) = worktree_tree(folder) else {
        return Ok(()); // a non-git folder has no tree to record
    };
    let fleet_home = crate::home::fleet_home(
        std::env::var("FLEET_HOME").ok().as_deref(),
        std::env::var("HOME").ok().as_deref(),
    )?;
    let config = fleet_home.join("config.json");
    let _guard = crate::filelock::FileLock::take(&crate::filelock::FileLock::beside(&config));
    let canonical = folder
        .canonicalize()
        .unwrap_or_else(|_| folder.to_path_buf());
    let mut document: serde_json::Value = std::fs::read_to_string(&config)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_else(|| serde_json::json!({}));
    let root = document
        .as_object_mut()
        .ok_or_else(|| "config.json is not an object".to_string())?;
    if let Some(record) = root
        .get_mut("layerfs")
        .and_then(|layerfs| layerfs.get_mut("projects"))
        .and_then(|projects| projects.as_object_mut())
        .and_then(|projects| projects.get_mut(canonical.to_string_lossy().as_ref()))
    {
        record["tree"] = serde_json::json!(tree);
    }
    std::fs::write(
        &config,
        serde_json::to_string_pretty(&document).unwrap_or_default(),
    )
    .map_err(|error| format!("config.json could not be written: {error}"))
}

/// The context a [`FileAccess::LayerFs`] run works in — which
/// project's which branch, in which sandbox. The child's whole
/// filesystem identity in one value; the arm carries it, the dispatch
/// hands it to the driver, and nothing else about the run changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceHandle {
    pub project: String,
    pub branch: String,
    pub sandbox: String,
    /// The launch folder's content hashes at fork time — the settle's
    /// record of the host's state, `None` for a folder that cannot be
    /// snapshotted (a non-git folder: no home apply, the delta named in
    /// the result instead).
    pub digests: Option<std::collections::BTreeMap<String, String>>,
    /// The launch folder the child's branch forked from — the apply's
    /// host side. `None` for the probe's direct sessions, which have
    /// no host to apply to.
    pub folder: Option<PathBuf>,
    /// The spawn sync's plan, executed after the fork: the host's newer
    /// paths carried onto the fresh branch, so the fork base is the
    /// host AS IT STANDS. `None` for the probe's sessions and for
    /// folders with no recorded tree.
    pub sync: Option<SyncPlan>,
}

/// What one command answered: the exit code and streams the model
/// reads, plus the cadence's own steps with their shape (mount,
/// exec, commit, unmount) — the record the tests and the plan pin and
/// the arm renders as ordinary bash output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandOutcome {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    /// One line per cadence step: `mount`, `exec`, `commit kind`,
    /// `unmount` — timings in the steps the probe measures, shapes
    /// here.
    pub steps: Vec<String>,
}

/// The view path rule, mirroring `FileAccess::Folder`: a relative
/// path resolves under the branch root, an absolute path IS the
/// view's path, and `..` that would pop past the root is refused by
/// name — an escape cannot be spelled. Measured contract detail: a
/// command's absolute paths resolve against the CONTAINER root
/// (never the workspace), so this rule governs the file tools, and
/// bash keeps the ordinary terminal contract — cwd is the project
/// root, shell commands use paths relative to it.
pub fn view_path(raw: &str) -> Result<String, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("an empty path is not a file".to_string());
    }
    let mut clean: Vec<&str> = Vec::new();
    for part in trimmed.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                if clean.pop().is_none() {
                    return Err(format!(
                        "`{raw}` leaves the workspace view: `..` would pass the root"
                    ));
                }
            }
            other => clean.push(other),
        }
    }
    Ok(format!("/{}", clean.join("/")))
}

/// What one `branch merge` answered. A same-file collision is an
/// outcome here, not an error: the parent reads it as the first-class
/// fact it is (the plan's C2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MergeOutcome {
    /// The merge landed; the value is the CLI's own answer.
    Merged(serde_json::Value),
    /// Two commits changed the same file and no `.lww` rule covers it.
    /// The paths are the CLI's conflict list, bounded; the child's
    /// branch is kept so the parent can resolve there.
    Conflict { paths: Vec<String> },
}

/// One resolved CLI: the executable to spawn. Constructed by
/// [`Gate::resolve`] (fresh, for doctor and tests) or read once per
/// process through [`gate`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Gate {
    cli: OsString,
    /// The arguments that precede every call's own: empty for a
    /// resolved executable, the bundle's CLI script for the
    /// `node <bundle>/…/layerfs.js` production route.
    prefix: Vec<String>,
}

impl Gate {
    /// Resolve the CLI and read its wire version — fresh, uncached.
    /// [`gate`] memoizes this once per process; doctor and the tests
    /// call it directly so a stub can be the CLI under test.
    pub fn resolve() -> Result<Gate, String> {
        // Two resolutions, one order: the `FLEET_LAYERFS_BIN` override
        // (tests, and an operator pointing at a different checkout —
        // it carries its own environment), else the machine's recorded
        // facts, whose sealed bundle runs the CLI under `node` — the
        // `/tmp` checkout retires here.
        let (cli, prefix) = match std::env::var_os(CLI_ENV).filter(|path| !path.is_empty()) {
            Some(path) => (path, Vec::new()),
            None => {
                let machine = machine()?;
                let script = machine
                    .bundle
                    .join("api/agent/cli/src/layerfs.js")
                    .to_string_lossy()
                    .into_owned();
                (OsString::from("node"), vec![script])
            }
        };
        let answered = run_version(&cli, &prefix)?;
        if answered == SUPPORTED_WIRE {
            Ok(Gate { cli, prefix })
        } else {
            Err(Error::Wire { answered }.to_string())
        }
    }

    /// One call's full argv: the prefix (the bundle's script) then the
    /// call's own arguments.
    fn argv<'a>(&'a self, args: &'a [&'a str]) -> Vec<&'a str> {
        self.prefix
            .iter()
            .map(String::as_str)
            .chain(args.iter().copied())
            .collect()
    }

    /// A Gate over any executable, for the surfaces' tests: nothing
    /// resolves, nothing is spawned — the caller names the CLI.
    #[cfg(test)]
    pub fn for_test(cli: OsString) -> Gate {
        Gate {
            cli,
            prefix: Vec::new(),
        }
    }

    fn call(&self, call: &str, args: &[&str]) -> Result<serde_json::Value, Error> {
        let body = spawn(&self.cli, call, &self.argv(args), CONTROL_BUDGET)?;
        match serde_json::from_str::<serde_json::Value>(body.trim()) {
            Ok(value) => Ok(value),
            Err(_) => Err(Error::Decode {
                call: call.to_string(),
                body: bounded(&body),
            }),
        }
    }

    /// `branch fork <branch>` — the child's own line, forked from the
    /// project's main.
    pub fn fork(&self, project: &str, branch: &str) -> Result<serde_json::Value, Error> {
        self.call(
            "branch fork",
            &["--project", project, "branch", "fork", branch],
        )
    }

    /// `sandbox create` — the only call that starts one; the lowest
    /// free number answers.
    pub fn sandbox_create(&self, project: &str) -> Result<String, Error> {
        let answer = self.call(
            "sandbox create",
            &["--project", project, "sandbox", "create"],
        )?;
        answer
            .get("sandbox")
            .and_then(|sandbox| sandbox.as_str())
            .map(str::to_string)
            .ok_or_else(|| Error::Decode {
                call: "sandbox create".to_string(),
                body: bounded(&answer.to_string()),
            })
    }

    /// `workspace mount <sandbox> <branch>` — binds the committed tree
    /// without copying it; writes go to the Workspace's own overlay.
    pub fn mount(&self, project: &str, sandbox: &str, branch: &str) -> Result<Mount, Error> {
        let answer = self.call(
            "workspace mount",
            &["--project", project, "workspace", "mount", sandbox, branch],
        )?;
        let ws = answer
            .get("ws")
            .and_then(|ws| ws.as_str())
            .map(str::to_string);
        let path = answer
            .get("path")
            .and_then(|path| path.as_str())
            .map(PathBuf::from);
        match (ws, path, answer.get("sandbox").and_then(|s| s.as_str())) {
            (Some(ws), Some(path), Some(_)) => Ok(Mount {
                sandbox: sandbox.to_string(),
                path,
                ws,
            }),
            _ => Err(Error::Decode {
                call: "workspace mount".to_string(),
                body: bounded(&answer.to_string()),
            }),
        }
    }

    /// `workspace commit <ws>` — captures the overlay and publishes a
    /// Commit on the branch, one compare-and-swap on its head.
    pub fn commit(
        &self,
        project: &str,
        ws: &str,
        message: &str,
    ) -> Result<serde_json::Value, Error> {
        self.call(
            "workspace commit",
            &[
                "--project",
                project,
                "workspace",
                "commit",
                ws,
                "-m",
                message,
            ],
        )
    }

    /// `workspace unmount <ws> [--discard]` — ends the Workspace.
    /// Clean End refuses uncommitted state; `discard` abandons it.
    /// Neither ever commits implicitly, which is why the arm commits
    /// first and only then unmounts.
    pub fn unmount(
        &self,
        project: &str,
        ws: &str,
        discard: bool,
    ) -> Result<serde_json::Value, Error> {
        let mut args = vec!["--project", project, "workspace", "unmount", ws];
        if discard {
            args.push("--discard");
        }
        self.call("workspace unmount", &args)
    }

    /// `sandbox close <sandbox>` — stops and removes one container.
    pub fn sandbox_close(&self, project: &str, sandbox: &str) -> Result<serde_json::Value, Error> {
        self.call(
            "sandbox close",
            &["--project", project, "sandbox", "close", sandbox],
        )
    }

    /// `branch merge <a> <b> <name>` — merges two branches into a new
    /// one. A same-file collision answers [`MergeOutcome::Conflict`],
    /// carried as the answer it is.
    pub fn merge(
        &self,
        project: &str,
        a: &str,
        b: &str,
        name: &str,
        message: &str,
    ) -> Result<MergeOutcome, Error> {
        let args = [
            "--project",
            project,
            "branch",
            "merge",
            a,
            b,
            name,
            "-m",
            message,
        ];
        // The merge reads its own exit: a `Conflict` refusal is an
        // answer here, so it must reach this method's hands rather
        // than being mapped to an error inside the shared spawn.
        let raw = spawn_with(
            &self.cli,
            "branch merge",
            &self.argv(&args),
            CONTROL_BUDGET,
            None,
        )?;
        if raw.success {
            return match serde_json::from_str::<serde_json::Value>(raw.stdout.trim()) {
                Ok(value) => Ok(MergeOutcome::Merged(value)),
                Err(_) => Err(Error::Decode {
                    call: "branch merge".to_string(),
                    body: bounded(&raw.stdout),
                }),
            };
        }
        match refusal_of(&raw.stderr) {
            Some((code, _)) if code == "Conflict" => Ok(MergeOutcome::Conflict {
                paths: conflict_paths(&raw.stderr),
            }),
            Some((code, message)) => Err(Error::Refused { code, message }),
            None => Err(Error::Failed {
                call: "branch merge".to_string(),
                exit: raw.exit,
                stderr: bounded(&raw.stderr),
            }),
        }
    }

    /// `branch publish <branch>` — the branch's head becomes main's
    /// new Layer, guarded to move main forward.
    pub fn publish(&self, project: &str, branch: &str) -> Result<serde_json::Value, Error> {
        self.call(
            "branch publish",
            &["--project", project, "branch", "publish", branch],
        )
    }

    // The fs verbs speak the CLI's own grammar over a branch — the
    // SDK path that needs no mount: it reads and writes the branch
    // head directly (measured: a fresh view sees every commit).

    /// `fs read <ref> <path>` — bounded window, text only (a binary
    /// file answers the CLI's `Binary` refusal, carried as one).
    pub fn fs_read(
        &self,
        project: &str,
        branch: &str,
        path: &str,
        lines: Option<(u32, u32)>,
    ) -> Result<serde_json::Value, Error> {
        let (start, end) = match lines {
            Some((start, end)) => (start.to_string(), end.to_string()),
            None => (String::new(), String::new()),
        };
        let mut args = vec!["--project", project, "fs", "read", branch, path];
        if lines.is_some() {
            args.extend(["--start-line", start.as_str(), "--end-line", end.as_str()]);
        }
        self.call("fs read", &args)
    }

    /// `fs list <ref> <path>` — one directory page.
    pub fn fs_list(
        &self,
        project: &str,
        branch: &str,
        path: &str,
    ) -> Result<serde_json::Value, Error> {
        self.call(
            "fs list",
            &["--project", project, "fs", "list", branch, path],
        )
    }

    /// `project list_branches` — every branch with its base, head and
    /// fork point: the face's branch list.
    /// `branch diff_branches <from> <to>` — the changed paths between
    /// two Branches (the wire diffs Branches or `main`; a fork base is
    /// a Layer when main has never committed, so the settle's changed
    /// set diffs the child against main and reads the spawn hashes
    /// for the base).
    pub fn diff_branches(
        &self,
        project: &str,
        from: &str,
        to: &str,
    ) -> Result<serde_json::Value, Error> {
        self.call(
            "branch diff_branches",
            &["--project", project, "branch", "diff_branches", from, to],
        )
    }

    /// `fs rm <branch> <path>` — a Deleted delta path carried onto the
    /// child's branch.
    pub fn fs_rm(
        &self,
        project: &str,
        branch: &str,
        path: &str,
    ) -> Result<serde_json::Value, Error> {
        self.call("fs rm", &["--project", project, "fs", "rm", branch, path])
    }

    pub fn list_branches(&self, project: &str) -> Result<serde_json::Value, Error> {
        self.call(
            "project list_branches",
            &["--project", project, "project", "list_branches"],
        )
    }

    /// `branch list_commits <branch>` — one branch's recent commits,
    /// newest first, with their parents.
    pub fn list_commits(
        &self,
        project: &str,
        branch: &str,
        page_size: u32,
    ) -> Result<serde_json::Value, Error> {
        let size = page_size.to_string();
        self.call(
            "branch list_commits",
            &[
                "--project",
                project,
                "branch",
                "list_commits",
                branch,
                "--page-size",
                &size,
            ],
        )
    }

    /// `fs list` with an explicit page — the depth walk needs every
    /// page of a big directory, not the CLI's default fifty.
    pub fn fs_list_page(
        &self,
        project: &str,
        branch: &str,
        path: &str,
        page_size: u32,
        page_pos: u32,
    ) -> Result<serde_json::Value, Error> {
        let size = page_size.to_string();
        let pos = page_pos.to_string();
        self.call(
            "fs list",
            &[
                "--project",
                project,
                "fs",
                "list",
                branch,
                path,
                "--page-size",
                &size,
                "--page-pos",
                &pos,
            ],
        )
    }

    /// `fs read <ref> <path> [--start-line --end-line]` — a file's
    /// text, a window of lines at a time. The instrument's text mode.
    pub fn fs_read_window(
        &self,
        project: &str,
        ref_: &str,
        path: &str,
        start_line: u32,
        end_line: u32,
    ) -> Result<serde_json::Value, Error> {
        let start = start_line.to_string();
        let end = end_line.to_string();
        self.call(
            "fs read",
            &[
                "--project",
                project,
                "fs",
                "read",
                ref_,
                path,
                "--start-line",
                &start,
                "--end-line",
                &end,
            ],
        )
    }

    /// `fs stat <ref> <path>` — one entry's type and size, read before
    /// the text: a binary is named, never rendered.
    pub fn fs_stat(
        &self,
        project: &str,
        ref_: &str,
        path: &str,
    ) -> Result<serde_json::Value, Error> {
        self.call("fs stat", &["--project", project, "fs", "stat", ref_, path])
    }

    /// `fs blame <ref> <path>` — every line with the Commit that last
    /// changed it. The instrument's blame mode.
    pub fn fs_blame(
        &self,
        project: &str,
        ref_: &str,
        path: &str,
        from_line: u32,
    ) -> Result<serde_json::Value, Error> {
        let from = from_line.to_string();
        let mut args = vec!["--project", project, "fs", "blame", ref_, path];
        if from_line > 0 {
            args.extend(["--from-line", &from]);
        }
        self.call("fs blame", &args)
    }

    /// `branch diff_commits <from> <to>` — two saved Commits' changed
    /// set (the wire takes Commits: the face resolves a Layer through
    /// the backend before it gets here). `--no-hunks` lists paths only.
    pub fn diff_commits(
        &self,
        project: &str,
        from: &str,
        to: &str,
        paths_only: bool,
    ) -> Result<serde_json::Value, Error> {
        let mut args = vec!["--project", project, "branch", "diff_commits", from, to];
        if paths_only {
            args.push("--no-hunks");
        }
        self.call("branch diff_commits", &args)
    }

    /// `fs diff_files <left-ref> <left-path> <right-ref> <right-path>`
    /// — one changed file's line changes between two refs.
    pub fn fs_diff_files(
        &self,
        project: &str,
        left_ref: &str,
        left_path: &str,
        right_ref: &str,
        right_path: &str,
    ) -> Result<serde_json::Value, Error> {
        self.call(
            "fs diff_files",
            &[
                "--project",
                project,
                "fs",
                "diff_files",
                left_ref,
                left_path,
                right_ref,
                right_path,
            ],
        )
    }

    /// `fs write <branch> <path>` with the body on stdin — exactly
    /// the bytes the caller sends, text or binary alike (the CLI's
    /// own bytes route).
    pub fn fs_write(
        &self,
        project: &str,
        branch: &str,
        path: &str,
        content: &[u8],
        message: &str,
    ) -> Result<serde_json::Value, Error> {
        let raw = spawn_with(
            &self.cli,
            "fs write",
            &self.argv(&[
                "--project",
                project,
                "fs",
                "write",
                branch,
                path,
                "-m",
                message,
            ]),
            CONTROL_BUDGET,
            Some(content),
        )?;
        let value: serde_json::Value =
            serde_json::from_str(raw.stdout.trim()).map_err(|_| Error::Decode {
                call: "fs write".to_string(),
                body: bounded(&raw.stdout),
            })?;
        // The wire answers a refusal as JSON with a `code` — parsing it
        // as success would report a write that never landed (the live
        // seam's failure: "wrote 8 bytes" with no Commit on the
        // branch).
        if let Some(code) = value.get("code").and_then(|code| code.as_str()) {
            let message = value
                .get("message")
                .and_then(|message| message.as_str())
                .unwrap_or_default();
            return Err(Error::Refused {
                code: code.to_string(),
                message: message.to_string(),
            });
        }
        Ok(value)
    }

    /// `fs edit <branch> <path> --old --new` — exactly-one-match,
    /// fleet's anchored-edit contract in the CLI itself.
    pub fn fs_edit(
        &self,
        project: &str,
        branch: &str,
        path: &str,
        old: &str,
        new: &str,
        message: &str,
    ) -> Result<serde_json::Value, Error> {
        self.call(
            "fs edit",
            &[
                "--project",
                project,
                "fs",
                "edit",
                branch,
                path,
                "-m",
                message,
                "--old",
                old,
                "--new",
                new,
            ],
        )
    }

    /// `fs grep <ref> <pattern> [<path>]` — chunked scan across
    /// boundaries, the CLI's own.
    pub fn fs_grep(
        &self,
        project: &str,
        branch: &str,
        pattern: &str,
        path: Option<&str>,
        page_size: Option<u32>,
    ) -> Result<serde_json::Value, Error> {
        let size = page_size.map(|size| size.to_string());
        let mut args = vec!["--project", project, "fs", "grep", branch, pattern];
        if let Some(path) = path {
            args.push(path);
        }
        if let Some(size) = size.as_deref() {
            args.push("--page-size");
            args.push(size);
        }
        self.call("fs grep", &args)
    }

    /// `fs glob <ref> <pattern>`.
    pub fn fs_glob(
        &self,
        project: &str,
        branch: &str,
        pattern: &str,
        page_size: Option<u32>,
    ) -> Result<serde_json::Value, Error> {
        let size = page_size.map(|size| size.to_string());
        let mut args = vec!["--project", project, "fs", "glob", branch, pattern];
        if let Some(size) = size.as_deref() {
            args.push("--page-size");
            args.push(size);
        }
        self.call("fs glob", &args)
    }

    /// `workspace exec <ws> <command>` — the command runs IN the
    /// sandbox at the workspace root; a nonzero exit is an answer
    /// (the CLI exits 0 carrying `exitCode`), and a refusal is one
    /// of the driver's own.
    pub fn exec(
        &self,
        project: &str,
        ws: &str,
        command: &str,
        budget: Duration,
    ) -> Result<serde_json::Value, Error> {
        let raw = spawn_with(
            &self.cli,
            "workspace exec",
            &self.argv(&["--project", project, "workspace", "exec", ws, command]),
            budget,
            None,
        )?;
        match serde_json::from_str::<serde_json::Value>(raw.stdout.trim()) {
            Ok(value) => Ok(value),
            Err(_) => Err(Error::Decode {
                call: "workspace exec".to_string(),
                body: bounded(&raw.stdout),
            }),
        }
    }

    /// One bash call's whole cadence — the shape dsh's own doctrine
    /// prescribes and this machine's wire confirmed (a fresh view
    /// sees the head; a held view pins; command writes publish only
    /// through Commit): **mount -> exec -> commit -> unmount**.
    ///
    /// The commit runs whatever the command's exit was: every step
    /// restorable is the point of the substrate, and the parent's
    /// merge gate — not this cadence — decides what reaches main. An
    /// unchanged-state commit (the CLI creates none) reads as success
    /// by its own law. A failed commit discards the overlay (its
    /// changes exist nowhere yet) and returns the typed error.
    pub fn run_command(
        &self,
        ctx: &WorkspaceHandle,
        command: &str,
        budget: Duration,
    ) -> Result<CommandOutcome, Error> {
        let mut steps: Vec<String> = Vec::new();
        let mount = self.mount(&ctx.project, &ctx.sandbox, &ctx.branch)?;
        steps.push("mount".to_string());
        let answered = match self.exec(&ctx.project, &mount.ws, command, budget) {
            Ok(value) => value,
            Err(error) => {
                let _ = self.unmount(&ctx.project, &mount.ws, true);
                return Err(error);
            }
        };
        steps.push("exec".to_string());
        let exit_code = answered
            .get("exitCode")
            .and_then(|code| code.as_i64())
            .map(|code| code as i32)
            .ok_or_else(|| Error::Decode {
                call: "workspace exec".to_string(),
                body: bounded(&answered.to_string()),
            })?;
        let stdout = answered
            .get("stdout")
            .and_then(|text| text.as_str())
            .unwrap_or_default()
            .to_string();
        let stderr = answered
            .get("stderr")
            .and_then(|text| text.as_str())
            .unwrap_or_default()
            .to_string();
        // Checkpoint, whatever the exit said.
        let preview: String = command
            .chars()
            .map(|ch| if ch.is_whitespace() { ' ' } else { ch })
            .take(120)
            .collect();
        let message = format!("fleet: bash {preview}");
        if let Err(error) = self.commit(&ctx.project, &mount.ws, &message) {
            let _ = self.unmount(&ctx.project, &mount.ws, true);
            return Err(error);
        }
        steps.push("commit".to_string());
        self.unmount(&ctx.project, &mount.ws, false)?;
        steps.push("unmount".to_string());
        Ok(CommandOutcome {
            exit_code,
            stdout,
            stderr,
            steps,
        })
    }
}

/// The once-per-process gate: the same answer for every caller in one
/// process, the way `sys1_extracts()` gates its own keys. A machine
/// that resolves the CLI once keeps resolving it; a machine that
/// cannot names the same absence to every caller, and no caller
/// retries per call.
/// The gate, resolved FRESH per call: the ambient environment decides
/// (the operator's `FLEET_LAYERFS_BIN`, else the sealed bundle), and no
/// process cache can hand one caller's CLI to another's world — the
/// live registration taught this, when a real delegation in the repo
/// claimed the once-per-process cache and every stub test beside it
/// inherited the real CLI. The cost is one `--version` per open, which
/// is nothing beside the spawns it gates.
pub fn gate() -> Result<Gate, String> {
    Gate::resolve()
}

/// Read the CLI's wire version: the last `x.y.z` token of its first
/// `--version` line. A CLI that answers nothing version-shaped is an
/// absence, named.
fn run_version(cli: &OsString, prefix: &[String]) -> Result<String, String> {
    let mut argv: Vec<&str> = prefix.iter().map(String::as_str).collect();
    argv.push("--version");
    let body = spawn(cli, "--version", &argv, VERSION_BUDGET).map_err(|error| match error {
        Error::Spawn(reason) => format!("the CLI could not be spawned: {reason}"),
        Error::Timeout { .. } => format!(
            "the CLI did not answer --version within {}s",
            VERSION_BUDGET.as_secs()
        ),
        other => other.to_string(),
    })?;
    body.lines()
        .next()
        .unwrap_or_default()
        .split_whitespace()
        .rev()
        .find(|token| {
            let parts: Vec<&str> = token.split('.').collect();
            parts.len() == 3
                && parts
                    .iter()
                    .all(|part| !part.is_empty() && part.chars().all(|c| c.is_ascii_digit()))
        })
        .map(str::to_string)
        .ok_or_else(|| {
            format!(
                "the CLI answered no version the driver knows: {}",
                bounded(&body)
            )
        })
}

/// The refusal hiding in a nonzero exit's stderr: the CLI prints one
/// `{"code","message","detail"}` object there on a refusal (exit 125)
/// and a usage line on a wrong command line (exit 2).
fn refusal_of(stderr: &str) -> Option<(String, String)> {
    let value: serde_json::Value = serde_json::from_str(stderr.trim()).ok()?;
    let code = value.get("code")?.as_str()?.to_string();
    let message = value
        .get("message")
        .and_then(|message| message.as_str())
        .unwrap_or_default()
        .to_string();
    Some((code, bounded(&message)))
}

/// The conflict list inside a `Conflict` refusal's detail: the paths
/// both sides changed, bounded. Shape-tolerant on purpose — the CLI's
/// detail is diagnostics, and a path the driver cannot find is still
/// carried by the refusal's message.
fn conflict_paths(stderr: &str) -> Vec<String> {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(stderr.trim()) else {
        return Vec::new();
    };
    let mut paths = Vec::new();
    if let Some(conflicts) = value
        .get("detail")
        .and_then(|detail| detail.get("conflicts"))
    {
        let items = conflicts.as_array().map(Vec::as_slice).unwrap_or(&[]);
        for item in items {
            if paths.len() >= 8 {
                break;
            }
            if let Some(path) = item
                .as_str()
                .or_else(|| item.get("path").and_then(|p| p.as_str()))
            {
                if !paths.iter().any(|seen| seen == path) {
                    paths.push(path.to_string());
                }
            }
        }
    }
    paths
}

/// What one raw spawn answered, before any exit mapping: the streams
/// and the exit code, cut to bounds.
struct Raw {
    success: bool,
    exit: i32,
    stdout: String,
    stderr: String,
}

/// One spawn, one attempt, no mapping — optionally with a body on
/// stdin (the `fs write` route: the CLI reads its input before it
/// opens anything, so the write is one shot).
fn spawn_with(
    cli: &OsString,
    call: &str,
    args: &[&str],
    budget: Duration,
    stdin: Option<&[u8]>,
) -> Result<Raw, Error> {
    let mut child = Command::new(cli)
        .args(args)
        .stdin(if stdin.is_some() {
            std::process::Stdio::piped()
        } else {
            std::process::Stdio::null()
        })
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|error| Error::Spawn(format!("{call}: {error}")))?;
    if let Some(bytes) = stdin {
        use std::io::Write as _;
        let mut pipe = child
            .stdin
            .take()
            .ok_or_else(|| Error::Spawn(format!("{call}: no stdin pipe")))?;
        pipe.write_all(bytes)
            .map_err(|error| Error::Spawn(format!("{call}: stdin: {error}")))?;
        drop(pipe);
    }
    // The pipes are drained WHILE the child runs: an `fs read` window or
    // a big `fs list` answer exceeds the OS pipe buffer (64 KiB), and a
    // child blocked writing to a pipe nobody reads never exits — the
    // wait loop would burn the whole budget on a call that answered in
    // milliseconds. The readers are the reason it cannot.
    fn drain<R: std::io::Read + Send + 'static>(mut pipe: R) -> std::thread::JoinHandle<Vec<u8>> {
        std::thread::spawn(move || {
            let mut buf = Vec::new();
            let _ = pipe.read_to_end(&mut buf);
            buf
        })
    }
    let out = child.stdout.take().map(drain);
    let err = child.stderr.take().map(drain);
    let deadline = std::time::Instant::now() + budget;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {
                if std::time::Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(Error::Timeout {
                        call: call.to_string(),
                        budget,
                    });
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(error) => return Err(Error::Spawn(format!("{call}: {error}"))),
        }
    };
    let stdout = out
        .map(|handle| handle.join().unwrap_or_default())
        .unwrap_or_default();
    let stderr = err
        .map(|handle| handle.join().unwrap_or_default())
        .unwrap_or_default();
    Ok(Raw {
        success: status.success(),
        exit: status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&stdout).to_string(),
        stderr: String::from_utf8_lossy(&stderr).to_string(),
    })
}

/// One spawn, one attempt: run the CLI with no body and map its exit
/// onto typed custody — the refusal dialect (exit 125 carries a JSON
/// refusal, exit 2 a usage line the driver reads as its own bug) on
/// the way out.
fn spawn(cli: &OsString, call: &str, args: &[&str], budget: Duration) -> Result<String, Error> {
    let raw = spawn_with(cli, call, args, budget, None)?;
    if raw.success {
        return Ok(raw.stdout);
    }
    if let Some((code, message)) = refusal_of(&raw.stderr) {
        return Err(Error::Refused { code, message });
    }
    Err(Error::Failed {
        call: call.to_string(),
        exit: raw.exit,
        stderr: bounded(&raw.stderr),
    })
}

/// One child's Workspace, opened at spawn and settled at completion:
/// fork a Branch, start a Sandbox, mount the Workspace — and on the
/// way out commit-or-discard, unmount, close the Sandbox, and (when
/// it committed) merge the Branch back the way the plan's C2 spells:
/// the harness acts for the **parent** at the parent's own boundary;
/// the child never sees a layerfs call and structurally cannot merge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceSession {
    pub project: String,
    pub branch: String,
    pub sandbox: String,
    pub mount: Mount,
}

/// What one settled Workspace reports to the parent. Every failure is
/// carried as a fact beside the result — a cleanup that did not land
/// is named, never swallowed and never able to fail the delegation
/// itself (the audit-trail law, applied to custody).
/// The settle's decision about the launch folder: what may come home
/// and what is held. The apply itself is the ARM's act (it rides the
/// parent's approval gate); this is the decision the settle reaches
/// from the wire and the spawn-time hashes.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HomeApply {
    /// Paths the child changed and the host did not: safe to apply.
    pub applied: Vec<String>,
    /// Paths the host moved since the fork: a named conflict, never
    /// overwritten.
    pub held: Vec<String>,
    /// Paths main moved (another child's merge) and the host never
    /// touched: brought home alongside the child's work, so the host
    /// converges to main for everything the owner left alone.
    pub reconciled: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorkspaceReport {
    /// The child's branch — kept whether or not the merge landed, so
    /// a conflict can be resolved there.
    pub branch: String,
    /// The CLI's own Workspace handle (`<sandbox>:<path>`), the join
    /// from a calibration row back to the mount. Empty when the CLI
    /// vanished before settling.
    pub ws: String,
    /// The Sandbox the Workspace lived in.
    pub sandbox: String,
    /// Whether the overlay was captured as a Commit before unmount.
    pub committed: bool,
    /// Why the commit did not happen, when it did not.
    pub commit_note: Option<String>,
    /// The merge-back into main, after a commit. `None` when there
    /// was nothing to merge.
    pub merge: Option<MergeOutcome>,
    /// The settle's home apply, when one was decided: the paths safe to
    /// bring home (the child changed them, the host did not) and the
    /// paths held (the host moved since the fork — a named conflict,
    /// the file untouched). `None` when nothing was applied — no
    /// snapshot, a refused merge, a child that changed nothing.
    pub home: Option<HomeApply>,
    /// Cleanup facts: discards, unmount or close failures, publish
    /// failures — named, bounded, never fatal.
    pub notes: Vec<String>,
}

impl WorkspaceSession {
    /// Fork, Sandbox, mount — in that order, and each partial is
    /// cleaned on the way out of a failed open: a Sandbox that started
    /// gets closed, a Branch that forked is named in the error (it is
    /// harmless and an owner can remove it; losing the name would be
    /// the actual sin).
    /// Fork a branch and start its Sandbox WITHOUT mounting — the
    /// seam plan's shape: the mount belongs to each bash call's
    /// cadence, not to the child's whole life, because a held view
    /// pins its base (measured) while a fresh mount sees the head.
    pub fn open_handle(
        project: &str,
        branch: &str,
        digests: Option<std::collections::BTreeMap<String, String>>,
        folder: Option<PathBuf>,
        sync: Option<SyncPlan>,
    ) -> Result<WorkspaceHandle, Error> {
        // The wire's name rule, checked before anything spawns: an
        // illegal branch is refused here, named, with no round trip —
        // the live run taught that the stub would have accepted it.
        history_name_ok(branch).map_err(|reason| Error::Refused {
            code: "WorkspaceOpen".to_string(),
            message: format!("the child's branch would be refused by the wire: {reason}"),
        })?;
        let gate = gate().map_err(|reason| Error::Absent(reason.clone()))?;
        gate.fork(project, branch)?;
        let sandbox = match gate.sandbox_create(project) {
            Ok(sandbox) => sandbox,
            Err(error) => {
                return Err(Error::Refused {
                    code: "WorkspaceOpen".to_string(),
                    message: format!(
                        "branch `{branch}` was forked but the Sandbox could not start ({error});                          the branch is kept and named here rather than deleted behind your back"
                    ),
                })
            }
        };
        let handle = WorkspaceHandle {
            project: project.to_string(),
            branch: branch.to_string(),
            sandbox,
            digests,
            folder,
            sync,
        };
        // The sync rides the fork: the host's newer paths are written
        // onto the fresh branch BEFORE the child's first turn, so the
        // fork base is the host as it stands. A carried path's failure
        // is named in the sync note and the child still runs — the
        // settle's digest check sees whatever landed.
        if let Some(plan) = &handle.sync {
            for (path, kind) in &plan.to_child {
                let view = format!("/{path}");
                match kind {
                    'D' => {
                        if let Err(error) = gate.fs_rm(&handle.project, &handle.branch, &view) {
                            eprintln!("fleet: layerfs: sync: {path}: {error}");
                        }
                    }
                    _ => {
                        let Some(host_bytes) = handle
                            .folder
                            .as_ref()
                            .and_then(|folder| std::fs::read(folder.join(path)).ok())
                        else {
                            eprintln!("fleet: layerfs: sync: {path}: the host file is gone");
                            continue;
                        };
                        if let Err(error) = gate.fs_write(
                            &handle.project,
                            &handle.branch,
                            &view,
                            &host_bytes,
                            "fleet: sync from host",
                        ) {
                            eprintln!("fleet: layerfs: sync: {path}: {error}");
                        }
                    }
                }
            }
        }
        Ok(handle)
    }

    /// The mounted lifecycle over a fresh handle — the C0/C2 shape
    /// the probe and the live roundtrip drive directly. The arm uses
    /// `open_handle` + per-command mounts instead; both share the
    /// fork half above.
    pub fn open(project: &str, branch: &str) -> Result<WorkspaceSession, Error> {
        let handle = Self::open_handle(project, branch, None, None, None)?;
        let gate = gate().map_err(|reason| Error::Absent(reason.clone()))?;
        match gate.mount(project, &handle.sandbox, branch) {
            Ok(mount) => Ok(WorkspaceSession {
                project: handle.project,
                branch: handle.branch,
                sandbox: handle.sandbox,
                mount,
            }),
            Err(error) => {
                let _ = gate.sandbox_close(project, &handle.sandbox);
                Err(Error::Refused {
                    code: "WorkspaceOpen".to_string(),
                    message: format!(
                        "the Sandbox started but the mount failed ({error}); it was closed and                          branch `{branch}` is kept"
                    ),
                })
            }
        }
    }

    /// The path the child's tools work at — the seam the plan names:
    /// `FileAccess::Folder` already confines a run to one folder, and
    /// this folder is the mount.
    pub fn path(&self) -> &std::path::Path {
        &self.mount.path
    }

    /// Settle at completion, in the order custody requires: capture
    /// first (a Clean End refuses uncommitted state, by the CLI's own
    /// law), then unmount, then close the Sandbox, then — only when a
    /// Commit exists — merge the Branch back into main and publish.
    /// `commit` is the policy's answer: true when the child stopped
    /// clean enough to keep its work.
    pub fn settle(self, commit: bool) -> WorkspaceReport {
        let gate = match gate() {
            Ok(gate) => gate,
            Err(reason) => {
                return WorkspaceReport {
                    branch: self.branch,
                    ws: self.mount.ws,
                    sandbox: self.sandbox,
                    ..WorkspaceReport::default()
                }
                .with_note(format!("the CLI vanished before settling: {reason}"))
            }
        };
        let mut report = WorkspaceReport {
            branch: self.branch.clone(),
            ws: self.mount.ws.clone(),
            sandbox: self.sandbox.clone(),
            ..WorkspaceReport::default()
        };
        let mut merge_branch: Option<String> = None;
        if commit {
            let message = format!("fleet: subagent workspace {}", self.branch);
            match gate.commit(&self.project, &self.mount.ws, &message) {
                Ok(_) => {
                    report.committed = true;
                    merge_branch = Some(merge_branch_name(&self.branch));
                }
                Err(error) => {
                    report.commit_note = Some(error.to_string());
                    // The overlay still holds the work; the discard
                    // below loses it, so the note says exactly that.
                    report.notes.push(
                        "the Commit did not land, so the Workspace's changes are discarded with \
                         its overlay — the child's session log still holds what it did"
                            .to_string(),
                    );
                }
            }
        } else {
            report
                .notes
                .push("discarded: the child stopped without a keepable result".to_string());
        }
        if let Err(error) =
            gate.unmount(&self.project, &self.mount.ws, !(commit && report.committed))
        {
            report.notes.push(format!("unmount: {error}"));
        }
        if let Err(error) = gate.sandbox_close(&self.project, &self.sandbox) {
            report.notes.push(format!("sandbox close: {error}"));
        }
        if merge_branch.is_some() {
            merge_back(&gate, &self.project, &self.branch, &mut report);
        }
        report
    }
}

/// The parent's half of the settlement — merge the child's branch
/// into `main` through a fresh integration branch, publish when it
/// lands, and carry a `Conflict` as the answer it is — shared by the
/// mounted `settle` and the seam plan's `close_handle`.
fn merge_back(gate: &Gate, project: &str, branch: &str, report: &mut WorkspaceReport) {
    let integration = merge_branch_name(branch);
    let message = format!("fleet: merge subagent {branch}");
    match gate.merge(project, "main", branch, &integration, &message) {
        Ok(MergeOutcome::Merged(_)) => {
            // One retry on the lagging-worker race: a merge that just
            // landed can be read before the worker's saved Commit is
            // visible to the publish (KI-321's own remedy note). A
            // second refusal is the real answer, surfaced as today.
            let mut outcome = gate.publish(project, &integration);
            if let Err(Error::Refused { code, .. }) = &outcome {
                if code == "NoCommit" {
                    std::thread::sleep(std::time::Duration::from_millis(400));
                    outcome = gate.publish(project, &integration);
                }
            }
            match outcome {
                Ok(_) => report.merge = Some(MergeOutcome::Merged(serde_json::Value::Null)),
                Err(error) => {
                    report.notes.push(format!(
                        "the merge landed on `{integration}` but the publish to main failed: {error}                      — the integration branch is kept"
                    ));
                    report.merge = Some(MergeOutcome::Merged(serde_json::Value::Null));
                }
            }
        }
        Ok(conflict @ MergeOutcome::Conflict { .. }) => {
            report.notes.push(format!(
                "main moved while the child worked — the merge was refused as a Conflict and                  `{branch}` is kept for resolution; nothing was overwritten"
            ));
            report.merge = Some(conflict);
        }
        Err(error) => report.notes.push(format!("merge: {error}")),
    }
}

impl WorkspaceHandle {
    /// Close an opted-in child's settlement WITHOUT a mounted
    /// Workspace to end — the seam plan's cadence committed every
    /// step as it happened, so there is no overlay left to capture:
    /// close the Sandbox, and when the child stopped keepable, merge
    /// its branch back the parent's way. A child that stopped
    /// without a keepable result keeps its branch unmerged —
    /// isolation was always the protection, and the parent's merge
    /// gate is still the only door to `main`.
    pub fn close(self, keep: bool) -> WorkspaceReport {
        let mut report = WorkspaceReport {
            branch: self.branch.clone(),
            ws: String::new(),
            sandbox: self.sandbox.clone(),
            // The cadence's claim: every step that changed anything
            // is already on the branch. (A read-only child has
            // nothing to commit, and the claim is vacuous for it.)
            committed: true,
            ..WorkspaceReport::default()
        };
        let gate = match gate() {
            Ok(gate) => gate,
            Err(reason) => {
                report
                    .notes
                    .push(format!("the CLI vanished before settling: {reason}"));
                return report;
            }
        };
        if let Err(error) = gate.sandbox_close(&self.project, &self.sandbox) {
            report.notes.push(format!("sandbox close: {error}"));
        }
        if !keep {
            report.notes.push(
                "not merged: the child stopped without a keepable result — its branch is kept"
                    .to_string(),
            );
            return report;
        }
        // The home apply's DECISION, before the merge: the changed set
        // is the child against main, and the spawn-time hashes decide
        // which of those paths the CHILD moved (a path where the
        // child's version equals the fork-time version moved on main,
        // not here — it is not this child's to bring home) and whether
        // the host moved since the fork (a named conflict, never
        // overwritten).
        let home = plan_home(&gate, &self, &mut report);
        merge_back(&gate, &self.project, &self.branch, &mut report);
        // Only a landed merge applies home: a Conflict keeps the branch
        // for resolution, and applying its work to the host would walk
        // around the very conflict the merge named.
        match (&home, &report.merge) {
            (Some(home), Some(MergeOutcome::Merged(_))) => report.home = Some(home.clone()),
            (Some(_), _) => report.notes.push(
                "the merge did not land, so nothing was applied home — the branch is kept for \
                 resolution"
                    .to_string(),
            ),
            (None, _) => {}
        }
        report
    }
}

/// The home apply's decision, from the wire and the spawn hashes. The
/// changed set is `diff_branches main <child>` — the wire diffs
/// Branches, and the child's fork base is a Layer when main has never
/// committed, so there is no base branch to name; the spawn hashes
/// carry the base instead. Each candidate is read once from the
/// child's branch and classified:
///   the child's version equals the fork-time hash — main moved, not
///     the child: skipped, not this child's change;
///   the host equals the fork-time hash — untouched since the fork:
///     applied;
///   the host equals the child's version — already home: no-op;
///   anything else — the host moved: HELD, named, untouched.
fn plan_home(
    gate: &Gate,
    handle: &WorkspaceHandle,
    report: &mut WorkspaceReport,
) -> Option<HomeApply> {
    let digests = handle.digests.as_ref()?;
    let Ok(document) = gate.diff_branches(&handle.project, "main", &handle.branch) else {
        // The diff is one call; a refusal names itself and the delta
        // stays in the Store.
        report.notes.push(
            "the home apply could not read the child's changes — the delta stays on the \
                   branch, readable in the workspace-history section"
                .to_string(),
        );
        return None;
    };
    let items = document.get("items").and_then(|items| items.as_array())?;
    let mut home = HomeApply::default();
    for item in items {
        let Some(path) = item.get("path").and_then(|path| path.as_str()) else {
            continue;
        };
        // The branch view's paths are root-absolute (`/live-proof.txt`);
        // the host's and the snapshot's keys are relative
        // (`live-proof.txt`).
        let view = format!("/{}", path.trim_start_matches('/'));
        let path = path.trim_start_matches('/').to_string();
        let child_version = gate
            .fs_read(&handle.project, &handle.branch, &view, None)
            .ok();
        let child_hash = child_version
            .as_ref()
            .and_then(|document| document.get("text").and_then(|text| text.as_str()))
            .map(hash_text);
        let spawn = digests.get(&path).cloned();
        // The child did not move this path (its version is the
        // fork-time version, or it never existed there): main may have.
        // The host untouched + main's version different from the fork
        // time = main's work the host never received: reconciled home.
        if child_hash == spawn {
            if let Some(main_version) = gate
                .fs_read(&handle.project, "main", &view, None)
                .ok()
                .and_then(|document| {
                    document
                        .get("text")
                        .and_then(|text| text.as_str())
                        .map(str::to_string)
                })
            {
                let main_hash = hash_text(&main_version);
                if Some(&main_hash) != spawn.as_ref() && host_hash_for(handle, &path) == spawn {
                    home.reconciled.push(path);
                }
            }
            continue;
        }
        let now = host_hash_for(handle, &path);
        if now == child_hash {
            continue;
        }
        if now == spawn {
            home.applied.push(path);
        } else {
            home.held.push(path);
        }
    }
    if home.applied.is_empty() && home.held.is_empty() {
        return None;
    }
    Some(home)
}

/// The host's hash for one path, through the folder the handle carries
/// — `None` when the file is absent, which is how a Removed path reads.
fn host_hash_for(handle: &WorkspaceHandle, path: &str) -> Option<String> {
    let folder = handle.folder.as_ref()?;
    host_hash(folder, path)
}

/// One text's hash, the same function the spawn snapshot uses — the
/// two sides of a comparison must be one implementation.
fn hash_text(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    std::hash::Hasher::write(&mut hasher, bytes);
    std::hash::Hasher::write_usize(&mut hasher, bytes.len());
    format!("{:016x}", std::hash::Hasher::finish(&hasher))
}

impl WorkspaceReport {
    fn with_note(mut self, note: String) -> Self {
        self.notes.push(note);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as _;

    struct EnvGuard {
        name: &'static str,
        previous: Option<std::ffi::OsString>,
    }

    impl EnvGuard {
        fn set(name: &'static str, value: impl AsRef<std::ffi::OsStr>) -> EnvGuard {
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

    /// A stub `layerfs`: a POSIX sh script that logs its argv and
    /// answers from the case table in `body`. The double pattern the
    /// plan's verification names — no Docker, no Store, no daemon.
    fn stub(name: &str, body: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "fleet-layerfs-stub-{}-{}-{name}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let script = dir.join("layerfs");
        let mut file = std::fs::File::create(&script).unwrap();
        writeln!(file, "#!/bin/sh").unwrap();
        writeln!(
            file,
            "printf '%s\\n' \"$*\" >> \"{}\"",
            dir.join("argv.log").display()
        )
        .unwrap();
        write!(file, "{body}").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        script
    }

    fn log_lines(script: &std::path::Path) -> Vec<String> {
        let dir = script.parent().unwrap();
        std::fs::read_to_string(dir.join("argv.log"))
            .unwrap_or_default()
            .lines()
            .map(str::to_string)
            .collect()
    }

    const HAPPY: &str = r#"case "$*" in
  *--version*) echo "1.1.0" ;;
  *"branch fork"*) echo '{"name":"fleet/abc"}' ;;
  *"sandbox create"*) echo '{"sandbox":"3","image":"sha256:x"}' ;;
  *"workspace mount"*) echo '{"sandbox":"3","path":"/workspaces/1","image":"sha256:x","ws":"3:/workspaces/1"}' ;;
  *"workspace commit"*) echo '{"kind":"Committed","commit":{"id":"c-1"}}' ;;
  *"workspace unmount"*) echo '{}' ;;
  *"sandbox close"*) echo '{}' ;;
  *"branch merge"*) echo '{"merge":"m-1"}' ;;
  *"branch publish"*) echo '{"layer":"l-1"}' ;;
  *"workspace exec"*)
    if [ "$FLEET_STUB_MODE" = "execfail" ]; then
      printf '%s\n' '{"kind":"exit:3","exitCode":3,"stdout":"partial\n","stderr":"boom\n","message":null,"truncated":false}'
    elif [ "$FLEET_STUB_MODE" = "execsleep" ]; then
      sleep 5
      printf '%s\n' '{"kind":"exit:0","exitCode":0,"stdout":"","stderr":"","message":null,"truncated":false}'
    else
      printf '%s\n' '{"kind":"exit:0","exitCode":0,"stdout":"ran\n","stderr":"","message":null,"truncated":false}'
    fi ;;
  *"fs read"*) printf '%s\n' '{"kind":"ok","text":"hello\n","startLine":1,"endLine":1,"truncated":false}' ;;
  *"fs list"*) echo '{"items":[],"hasMore":false}' ;;
  *"fs write"*)
    if [ -n "$FLEET_STUB_STDIN" ]; then cat > "$FLEET_STUB_STDIN"; else cat > /dev/null; fi
    echo '{"kind":"ok"}' ;;
  *"fs edit"*) echo '{"kind":"ok"}' ;;
  *"fs grep"*|*"fs glob"*) echo '{"items":[],"hasMore":false}' ;;
  *) echo "layerfs: unknown" >&2; exit 2 ;;
esac
"#;

    #[test]
    fn the_driver_composes_the_clis_own_grammar_and_parses_the_answer() {
        let _lock = crate::TEST_ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let script = stub("grammar", HAPPY);
        let _env = EnvGuard::set(CLI_ENV, &script);
        let gate = Gate::resolve().expect("the stub answers 1.1.0");
        let mount = gate
            .mount("shop", "3", "fleet/abc")
            .expect("the mount answer parses");
        assert_eq!(mount.ws, "3:/workspaces/1");
        assert_eq!(mount.path, PathBuf::from("/workspaces/1"));
        let lines = log_lines(&script);
        assert!(
            lines
                .iter()
                .any(|line| line == "--project shop workspace mount 3 fleet/abc"),
            "the mount call is the CLI's grammar, by name: {lines:?}"
        );
    }

    #[test]
    fn a_conflict_is_an_answer_not_an_error() {
        let _lock = crate::TEST_ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let script = stub(
            "conflict",
            r#"case "$*" in
  *--version*) echo "1.1.0" ;;
  *"branch merge"*)
    echo '{"code":"Conflict","message":"both changed src/lib.rs","detail":{"conflicts":[{"path":"src/lib.rs"},{"path":"src tools.rs"}]}}' >&2
    exit 125 ;;
esac
"#,
        );
        let _env = EnvGuard::set(CLI_ENV, &script);
        let gate = Gate::resolve().expect("the stub answers 1.1.0");
        let outcome = gate
            .merge("shop", "main", "fleet/abc", "fleet-merge-fleet/abc", "m")
            .expect("a conflict is an answer");
        match outcome {
            MergeOutcome::Conflict { paths } => {
                assert_eq!(paths, vec!["src/lib.rs", "src tools.rs"]);
            }
            other => panic!("expected the conflict, got {other:?}"),
        }
    }

    #[test]
    fn a_refusal_carries_its_own_code_and_message() {
        let _lock = crate::TEST_ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let script = stub(
            "refusal",
            r#"case "$*" in
  *--version*) echo "1.1.0" ;;
  *"sandbox create"*)
    echo '{"code":"NoSuchProject","message":"no project named shop"}' >&2
    exit 125 ;;
esac
"#,
        );
        let _env = EnvGuard::set(CLI_ENV, &script);
        let gate = Gate::resolve().expect("the stub answers 1.1.0");
        let error = gate.sandbox_create("shop").expect_err("the refusal");
        assert!(
            matches!(&error, Error::Refused { code, message }
                if code == "NoSuchProject" && message.contains("shop")),
            "{error}"
        );
    }

    #[test]
    fn a_wire_that_is_not_ours_is_refused_by_name() {
        let _lock = crate::TEST_ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let script = stub("wire", "echo 'layerfs 1.0.0'\n");
        let _env = EnvGuard::set(CLI_ENV, &script);
        let error = Gate::resolve().expect_err("1.0.0 is not driven");
        assert!(
            error.contains("1.0.0") && error.contains(SUPPORTED_WIRE),
            "{error}"
        );
    }

    #[test]
    fn stdout_that_is_not_json_is_a_decode_with_the_body_bounded() {
        let _lock = crate::TEST_ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let script = stub(
            "decode",
            r#"case "$*" in
  *--version*) echo "1.1.0" ;;
  *"workspace commit"*) echo "certainly not json" ;;
esac
"#,
        );
        let _env = EnvGuard::set(CLI_ENV, &script);
        let gate = Gate::resolve().expect("the stub answers 1.1.0");
        let error = gate
            .commit("shop", "3:/workspaces/1", "m")
            .expect_err("decode");
        assert!(matches!(error, Error::Decode { .. }), "{error}");
    }

    #[test]
    fn a_budget_that_expires_is_one_attempt_named() {
        let _lock = crate::TEST_ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let script = stub(
            "timeout",
            r#"case "$*" in
  *--version*) echo "1.1.0" ;;
  *"sandbox create"*) sleep 5 ;;
esac
"#,
        );
        let _env = EnvGuard::set(CLI_ENV, &script);
        let _gate = Gate::resolve().expect("the stub answers 1.1.0");
        let started = std::time::Instant::now();
        // The public method carries the control budget; the timeout's
        // shape is pinned here through the private spawn's own knob.
        let error = spawn(
            &std::ffi::OsString::from(script_as_string(&script)),
            "sandbox create",
            &["sandbox", "create"],
            Duration::from_millis(200),
        )
        .expect_err("the budget expires");
        assert!(matches!(error, Error::Timeout { .. }), "{error}");
        assert!(
            started.elapsed() < Duration::from_secs(2),
            "one attempt, bounded"
        );
    }

    fn script_as_string(script: &std::path::Path) -> String {
        script.to_string_lossy().to_string()
    }

    #[test]
    fn a_missing_cli_is_the_named_absence() {
        let _lock = crate::TEST_ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let script = stub("absent", "exit 127\n");
        let _env = EnvGuard::set(CLI_ENV, &script);
        // exit 127 with no refusal: the driver reads the CLI's own
        // failure, never a fallback.
        let error = Gate::resolve().expect_err("no version answers");
        assert!(
            error.contains("127") || error.contains("version"),
            "{error}"
        );
    }

    /// C1: the machine's facts, the folder map, and the composed
    /// environment — read from `<FLEET_HOME>/config.json`'s `layerfs`
    /// section, with every missing fact named by what provisions it.
    #[test]
    fn the_machine_facts_compose_the_clis_environment() {
        let _lock = crate::TEST_ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let home = std::env::temp_dir().join(format!(
            "fleet-layerfs-machine-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).unwrap();
        let layerfs_home = home.join("layerfs-home");
        std::fs::create_dir_all(&layerfs_home).unwrap();
        let bundle = home.join("bundle");
        std::fs::create_dir_all(&bundle).unwrap();
        let folder = home.join("repo");
        std::fs::create_dir_all(&folder).unwrap();
        // The registration map is keyed by the CANONICAL path — macOS's
        // /var is a symlink to /private/var, and a registration written
        // against the unresolved path would never match its own folder.
        let folder = folder.canonicalize().unwrap();
        let store = PathBuf::from("/var/lib/docker/volumes/v1/_data/global/store.sqlite");
        std::fs::write(
            home.join("config.json"),
            serde_json::json!({
                "layerfs": {
                    "home": layerfs_home.display().to_string(),
                    "bundle": bundle.display().to_string(),
                    "image": "sha256:abc",
                    "projects": {
                        folder.display().to_string(): {
                            "name": "shop",
                            "store": store.display().to_string(),
                        }
                    }
                }
            })
            .to_string(),
        )
        .unwrap();

        let _home_env = EnvGuard::set("FLEET_HOME", &home);
        let facts = read_machine().expect("the facts parse");
        assert_eq!(facts.home, layerfs_home);
        assert_eq!(facts.bundle, bundle);
        assert_eq!(facts.image, "sha256:abc");

        // The folder's registration resolves; an unregistered folder
        // is None, never a guess.
        let map = read_registrations();
        let reg = find_registration(&map, &folder).expect("the registered folder");
        assert_eq!(reg.name, "shop");
        assert_eq!(reg.store, store);
        assert!(find_registration(&map, &home.join("elsewhere")).is_none());

        // The composition: endpoint derived from the home, daemon from
        // the bundle, image from the record, store path from the
        // project's registration.
        apply_env(&facts, Some(reg));
        let read = |name: &str| std::env::var_os(name).map(|v| v.to_string_lossy().into_owned());
        assert_eq!(
            read("LAYERFS_HOME").as_deref(),
            Some(layerfs_home.to_string_lossy().as_ref())
        );
        assert!(read("LAYERFS_WORKER_ENDPOINT")
            .as_deref()
            .is_some_and(|v| v.ends_with("worker/endpoint.json")));
        assert_eq!(
            read("LAYERFS_DAEMON").as_deref(),
            Some(bundle.join("layerfs-daemon").to_string_lossy().as_ref())
        );
        assert_eq!(read("LAYERFS_IMAGE").as_deref(), Some("sha256:abc"));
        assert_eq!(
            read("LAYERFS_STORE_PATH").as_deref(),
            Some(store.to_string_lossy().as_ref())
        );
        for name in [
            "LAYERFS_HOME",
            "LAYERFS_WORKER_ENDPOINT",
            "LAYERFS_DAEMON",
            "LAYERFS_IMAGE",
            "LAYERFS_STORE_PATH",
        ] {
            std::env::remove_var(name);
        }

        // The named absence: no `layerfs` section says what to run —
        // read through the uncached reader, since the process-level
        // cache already holds this home's facts.
        std::fs::write(home.join("config.json"), "{}").unwrap();
        let Err(reason) = read_machine() else {
            panic!("no section is named");
        };
        assert!(reason.contains("fleet install"), "{reason}");
        std::fs::remove_dir_all(&home).ok();
    }

    /// C2's bound, named from C0's measurement: a folder past either
    /// limit answers the typed over-bound — and the walk exits early,
    /// so a 32 GB build tree costs the walk of its first directory
    /// rather than the whole thing.
    #[test]
    fn the_registration_bound_refuses_a_build_tree_early() {
        let root = std::env::temp_dir().join(format!(
            "fleet-layerfs-bound-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("README.md"), "small\n").unwrap();
        std::fs::write(root.join("src/lib.rs"), "fn main() {}\n").unwrap();
        let shape = folder_shape_with(&root, 1_000, 100).expect("under the bound");
        // The root itself is not an entry; every entry under it is,
        // and only regular files count bytes.
        assert_eq!(shape, (3, 19), "entries and bytes counted");

        // Over on bytes: one big file answers the over-bound without
        // walking further.
        std::fs::write(root.join("big.bin"), vec![0u8; 2_000]).unwrap();
        let over = folder_shape_with(&root, 1_000, 100).expect_err("past the bytes bound");
        assert!(matches!(over, RegisterError::OverBound { .. }), "{over:?}");

        // Over on entries: many small files answer the over-bound.
        let many = std::env::temp_dir().join(format!(
            "fleet-layerfs-bound-many-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&many);
        std::fs::create_dir_all(&many).unwrap();
        for index in 0..12 {
            std::fs::write(many.join(format!("f{index}.txt")), "x").unwrap();
        }
        let over = folder_shape_with(&many, 1_000_000, 10).expect_err("past the entries bound");
        assert!(matches!(over, RegisterError::OverBound { .. }), "{over:?}");
        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&many);
    }

    /// C2's naming: the basename folded into the wire's grammar, and
    /// two folders of one name answered `api` and `api-2`.
    #[test]
    fn the_registration_name_folds_and_dedupes() {
        // The fold lowercases and keeps the grammar's legal `_`.
        assert_eq!(project_name_for(Path::new("/srv/My_Repo"), &[]), "my_repo");
        assert_eq!(
            project_name_for(Path::new("/srv/My_Repo"), &["my_repo".to_string()]),
            "my_repo-2"
        );
        assert_eq!(
            project_name_for(
                Path::new("/srv/My_Repo"),
                &["my_repo".to_string(), "my_repo-2".to_string()]
            ),
            "my_repo-3"
        );
        // A hostile basename still lands inside the grammar.
        let folded = project_name_for(Path::new("/srv/../weird name!"), &[]);
        assert!(history_name_ok(&folded).is_ok(), "{folded}");
    }

    /// C2's record: the folder's registration lands in the config's
    /// `layerfs.projects` under the canonical path, and the resolver
    /// reads it back — the write that makes the second delegation in
    /// the folder cost nothing.
    #[test]
    fn a_registration_lands_in_the_config_and_reads_back() {
        let _lock = crate::TEST_ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let home = std::env::temp_dir().join(format!(
            "fleet-layerfs-reg-home-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).unwrap();
        let folder = home.join("repo");
        std::fs::create_dir_all(&folder).unwrap();
        let folder = folder.canonicalize().unwrap();
        let _home_env = EnvGuard::set("FLEET_HOME", &home);
        record_registration(
            &folder,
            &Registration {
                name: "repo".to_string(),
                store: PathBuf::from("/volumes/v/_data/global/store.sqlite"),
                tree: None,
            },
            "layerfs-repo-store-x",
        )
        .expect("the record lands");
        let document: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(home.join("config.json")).unwrap())
                .unwrap();
        let record = &document["layerfs"]["projects"][folder.to_string_lossy().as_ref()];
        assert_eq!(record["name"], "repo");
        assert_eq!(record["volume"], "layerfs-repo-store-x");
        assert_eq!(record["store"], "/volumes/v/_data/global/store.sqlite");
        let _ = std::fs::remove_dir_all(&home);
    }

    /// C2's registration, end to end at the driver's layer: a
    /// constructed machine (no config, no process cache), a stub
    /// register script in the home, a real folder — the bound's
    /// pre-check, the name, the script's argv, the config's record.
    #[test]
    fn register_runs_the_script_and_records_the_folder() {
        let _lock = crate::TEST_ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let home = std::env::temp_dir().join(format!(
            "fleet-layerfs-reg-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&home);
        let layerfs_home = home.join("layerfs-home");
        let bundle = home.join("bundle");
        std::fs::create_dir_all(&layerfs_home).unwrap();
        std::fs::create_dir_all(&bundle).unwrap();
        let folder = home.join("My Project");
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::write(folder.join("README.md"), "real\n").unwrap();
        let argv_log = home.join("register-argv.log");
        std::fs::write(
            layerfs_home.join(REGISTER_SCRIPT),
            r#"import { appendFileSync } from "node:fs";
const name = process.argv[2];
appendFileSync(process.env.FLEET_STUB_REGISTER_LOG, process.argv.slice(2).join(" ") + "\n");
console.log(JSON.stringify({
  name,
  volume: `layerfs-${name}-store-x`,
  store: `/var/lib/docker/volumes/layerfs-${name}-store-x/_data/global/store.sqlite`,
}));
"#,
        )
        .unwrap();

        let _home_env = EnvGuard::set("FLEET_HOME", &home);
        let _log = EnvGuard::set("FLEET_STUB_REGISTER_LOG", &argv_log);
        let machine = Machine {
            home: layerfs_home,
            bundle,
            image: "sha256:x".to_string(),
        };
        let outcome = register(&machine, &folder).expect("the registration runs");
        let RegistrationOutcome::Registered {
            registration,
            skipped,
        } = outcome
        else {
            panic!("expected a registration");
        };
        // A plain folder is not a git repository: the whole-tree walk
        // is its answer, so nothing was skipped.
        assert!(skipped.is_empty(), "{skipped:?}");
        assert_eq!(registration.name, "my-project");
        assert_eq!(
            registration.store,
            PathBuf::from(
                "/var/lib/docker/volumes/layerfs-my-project-store-x/_data/global/store.sqlite"
            )
        );
        // The script's argv: the derived name, the source folder, and
        // the machine's facts.
        let argv = std::fs::read_to_string(&argv_log).expect("the script ran");
        assert!(argv.contains("my-project"), "{argv}");
        assert!(argv.contains(folder.to_string_lossy().as_ref()), "{argv}");
        assert!(
            argv.contains(&machine.home.to_string_lossy().to_string()),
            "{argv}"
        );
        assert!(
            argv.contains(&machine.bundle.to_string_lossy().to_string()),
            "{argv}"
        );
        assert!(argv.contains("sha256:x"), "{argv}");
        // The config recorded it, keyed canonically.
        let document: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(home.join("config.json")).unwrap())
                .unwrap();
        let canonical = folder.canonicalize().unwrap();
        assert_eq!(
            document["layerfs"]["projects"][canonical.to_string_lossy().as_ref()]["name"],
            "my-project"
        );
        let _ = std::fs::remove_dir_all(&home);
    }

    /// A fixture git repository: one tracked file, one ignored file,
    /// one untracked file, one tracked symlink, one gitlink. The shape
    /// the declared import has to read exactly right.
    fn declared_fixture(root: &Path) -> std::path::PathBuf {
        let folder = root.join("Declared Repo");
        std::fs::create_dir_all(folder.join("sub")).unwrap();
        std::fs::write(folder.join("README.md"), "declared\n").unwrap();
        std::fs::write(folder.join(".gitignore"), "target/\nignored.txt\n").unwrap();
        std::fs::create_dir_all(folder.join("target")).unwrap();
        std::fs::write(
            folder.join("target").join("build-artifact.bin"),
            vec![b'x'; 64 * 1024],
        )
        .unwrap();
        std::fs::write(folder.join("ignored.txt"), "ignored\n").unwrap();
        std::fs::write(folder.join("link-target.txt"), "linked\n").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink("../link-target.txt", folder.join("sub").join("link")).unwrap();
        let git = |args: &[&str]| {
            let status = std::process::Command::new("git")
                .arg("-C")
                .arg(&folder)
                .args(args)
                .env("GIT_AUTHOR_NAME", "t")
                .env("GIT_AUTHOR_EMAIL", "t@t")
                .env("GIT_COMMITTER_NAME", "t")
                .env("GIT_COMMITTER_EMAIL", "t@t")
                .status()
                .expect("git runs");
            assert!(status.success(), "git {args:?} failed");
        };
        git(&["init", "-q"]);
        git(&["add", "-A"]);
        // Untracked, honestly: written after the add, never staged.
        std::fs::write(folder.join("untracked.txt"), "untracked\n").unwrap();
        // A submodule's gitlink, without a real submodule: the entry
        // the import must SKIP and name.
        git(&[
            "update-index",
            "--add",
            "--cacheinfo",
            "160000,0123456789abcdef0123456789abcdef01234567,vendor/thing",
        ]);
        git(&["commit", "-qm", "the declared tree"]);
        folder
    }

    /// C0's pin: a git repository's registration stages exactly what
    /// git declares — the tracked file and the symlink carried, the
    /// ignored and untracked files absent, the gitlink named as the
    /// gap — and the staging dir outlives no outcome. The record is
    /// keyed on the real folder, never on the scratch.
    #[test]
    fn the_register_stages_what_git_declares() {
        let _lock = crate::TEST_ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let home = std::env::temp_dir().join(format!(
            "fleet-layerfs-declared-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&home);
        let layerfs_home = home.join("layerfs-home");
        let bundle = home.join("bundle");
        std::fs::create_dir_all(&layerfs_home).unwrap();
        std::fs::create_dir_all(&bundle).unwrap();
        let folder = declared_fixture(&home);
        let argv_log = home.join("register-argv.log");
        // The stub logs its argv AND what its source dir actually
        // holds — the Init would import exactly that tree.
        std::fs::write(
            layerfs_home.join(REGISTER_SCRIPT),
            r#"import { appendFileSync, readdirSync } from "node:fs";
const name = process.argv[2];
const source = process.argv[3];
const NL = String.fromCharCode(10);
appendFileSync(process.env.FLEET_STUB_REGISTER_LOG, "ARGV " + process.argv.slice(2).join(" ") + NL);
const walk = (dir, prefix) => readdirSync(dir, { withFileTypes: true }).flatMap((e) => {
  const path = prefix + e.name;
  if (e.name === ".git") return [];
  return e.isDirectory() ? walk(dir + "/" + e.name, path + "/") : [path];
});
appendFileSync(process.env.FLEET_STUB_REGISTER_LOG, "SOURCE " + walk(source, "").sort().join(",") + NL);
console.log(JSON.stringify({
  name,
  volume: `layerfs-${name}-store-x`,
  store: `/var/lib/docker/volumes/layerfs-${name}-store-x/_data/global/store.sqlite`,
}));
"#,
        )
        .unwrap();

        let _home_env = EnvGuard::set("FLEET_HOME", &home);
        let _log = EnvGuard::set("FLEET_STUB_REGISTER_LOG", &argv_log);
        let machine = Machine {
            home: layerfs_home.clone(),
            bundle,
            image: "sha256:x".to_string(),
        };
        let outcome = register(&machine, &folder).expect("the registration runs");
        let RegistrationOutcome::Registered {
            registration,
            skipped,
        } = outcome
        else {
            panic!("expected a registration");
        };
        assert_eq!(registration.name, "declared-repo");
        // The gitlink is the named gap; the ignored and untracked
        // files are not named because they are not declared at all.
        assert_eq!(skipped, vec!["vendor/thing".to_string()], "{skipped:?}");
        let log = std::fs::read_to_string(&argv_log).expect("the script ran");
        // The Init's source was the staging dir, not the folder.
        assert!(
            !log.contains(&folder.to_string_lossy().to_string()),
            "{log}"
        );
        // What the Init would import: the declared files, carried.
        let source_line = log
            .lines()
            .find(|line| line.starts_with("SOURCE "))
            .expect("the source listing");
        assert!(source_line.contains("README.md"), "{source_line}");
        assert!(source_line.contains("sub/link"), "{source_line}");
        assert!(!source_line.contains("ignored.txt"), "{source_line}");
        assert!(!source_line.contains("untracked.txt"), "{source_line}");
        assert!(!source_line.contains("build-artifact"), "{source_line}");
        assert!(!source_line.contains("vendor"), "{source_line}");
        // The staging dir outlives no outcome.
        let staging_root = layerfs_home.join("staging");
        let left = std::fs::read_dir(&staging_root)
            .map(|entries| entries.count())
            .unwrap_or(0);
        assert_eq!(left, 0, "the staging scratch is empty: {log}");
        // The record is keyed on the real folder, canonically.
        let document: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(home.join("config.json")).unwrap())
                .unwrap();
        let canonical = folder.canonicalize().unwrap();
        assert_eq!(
            document["layerfs"]["projects"][canonical.to_string_lossy().as_ref()]["name"],
            "declared-repo"
        );
        let _ = std::fs::remove_dir_all(&home);
    }

    /// The bound counts the DECLARED set: a repository sitting on a
    /// build tree larger than the bound registers anyway, because the
    /// import will never touch what the repo declares around. The
    /// whole-tree walk keeps its own bound (the previous test's
    /// plain-folder shape), and nothing tracked is the named fallback.
    #[test]
    fn the_bound_counts_what_git_declares_and_nothing_else() {
        let _lock = crate::TEST_ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let root = std::env::temp_dir().join(format!(
            "fleet-layerfs-declbound-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&root);
        let folder = declared_fixture(&root);
        // The declared parse: four carried paths (README, .gitignore,
        // link-target, sub/link), the gitlink skipped, the ignored and
        // untracked absent.
        let (declared, skipped) = declared_files(&folder).expect("a git repository");
        assert_eq!(skipped, vec!["vendor/thing".to_string()]);
        let names: Vec<&str> = declared.iter().map(|entry| entry.path.as_str()).collect();
        assert!(names.contains(&"README.md"), "{names:?}");
        assert!(names.contains(&".gitignore"), "{names:?}");
        assert!(names.contains(&"sub/link"), "{names:?}");
        assert!(!names.contains(&"target/build-artifact.bin"), "{names:?}");
        assert!(!names.contains(&"untracked.txt"), "{names:?}");
        // The declared shape: the carried files' bytes, nothing else —
        // and the symlink's own metadata (its link length), never its
        // target's.
        let (entries, bytes) = declared_shape(&declared, &folder).expect("the declared shape");
        assert_eq!(entries, 4, "{names:?}");
        let tracked_only = folder.join("README.md");
        let readme_bytes = std::fs::metadata(&tracked_only).unwrap().len();
        assert!(bytes > readme_bytes, "{bytes}");
        assert!(
            bytes
                < std::fs::metadata(folder.join("target").join("build-artifact.bin"))
                    .unwrap()
                    .len(),
            "the ignored build artifact never counts: {bytes}"
        );
        // Nothing tracked: the named fallback, word for word.
        let empty = root.join("Empty Repo");
        std::fs::create_dir_all(&empty).unwrap();
        let git = std::process::Command::new("git")
            .arg("-C")
            .arg(&empty)
            .args(["init", "-q"])
            .status()
            .expect("git runs");
        assert!(git.success());
        let machine = Machine {
            home: root.join("layerfs-home"),
            bundle: root.join("bundle"),
            image: "sha256:x".to_string(),
        };
        std::fs::create_dir_all(machine.home.clone()).unwrap();
        std::fs::write(machine.home.join(REGISTER_SCRIPT), "console.log('{}');").unwrap();
        let error = register(&machine, &empty).expect_err("nothing declared");
        match error {
            RegisterError::Failed(reason) => {
                assert!(reason.contains("declares no files"), "{reason}");
            }
            other => panic!("expected the named fallback, got {other:?}"),
        }
        let _ = std::fs::remove_dir_all(&root);
    }

    /// C2's pin: the settle's home decision, classified from the wire
    /// and the spawn hashes. Three paths in one diff — the child's own
    /// change (host untouched: applied), the owner's edit (host moved:
    /// HELD, never overwritten), and main's own movement (the child's
    /// version equals the fork-time version: skipped, not this child's
    /// to bring home).
    #[test]
    fn the_settle_classifies_the_home_apply_from_the_spawn_hashes() {
        let _lock = crate::TEST_ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let root = std::env::temp_dir().join(format!(
            "fleet-layerfs-home-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&root);
        // A git repository: the snapshot is the DECLARED set (a 2 GiB
        // whole-tree hash per delegation is the wrong cost), so the
        // fixture is a real one.
        let folder = root.join("repo");
        std::fs::create_dir_all(&folder).unwrap();
        // Three tracked files: one the child will change, one the owner
        // will touch, one main will move (the child never does).
        std::fs::write(folder.join("childs.txt"), "base\n").unwrap();
        std::fs::write(folder.join("owners.txt"), "base\n").unwrap();
        std::fs::write(folder.join("main-moved.txt"), "base\n").unwrap();
        let git = |args: &[&str]| {
            let status = std::process::Command::new("git")
                .arg("-C")
                .arg(&folder)
                .args(args)
                .env("GIT_AUTHOR_NAME", "t")
                .env("GIT_AUTHOR_EMAIL", "t@t")
                .env("GIT_COMMITTER_NAME", "t")
                .env("GIT_COMMITTER_EMAIL", "t@t")
                .status()
                .expect("git runs");
            assert!(status.success(), "git {args:?}");
        };
        git(&["init", "-q"]);
        git(&["add", "-A"]);
        git(&["commit", "-qm", "three files"]);
        let digests = content_hashes(&folder).expect("the declared set hashes");
        assert_eq!(digests.len(), 3, "{digests:?}");

        // The stub: the diff names three paths; fs read answers the
        // child's version per path — the child changed childs.txt,
        // main moved main-moved.txt (the child's version is the base),
        // and owners.txt is the child's version too.
        let script = stub(
            "home",
            r#"case "$*" in
  *--version*) echo "1.1.0" ;;
  *"branch fork"*) echo '{"name":"ok"}' ;;
  *"sandbox create"*) echo '{"sandbox":"1","image":"sha256:x"}' ;;
  *"branch diff_branches"*)
    echo '{"items":[
      {"path":"/childs.txt","kind":"Modified"},
      {"path":"/owners.txt","kind":"Modified"},
      {"path":"/main-moved.txt","kind":"Modified"}
    ],"hasMore":false}' ;;
  *"fs read"*)
    case "$*" in
      *childs.txt*) printf '%s\n' '{"kind":"ok","text":"the child wrote this\n"}' ;;
      *owners.txt*) printf '%s\n' '{"kind":"ok","text":"the child wrote this too\n"}' ;;
      *main-moved.txt*) printf '%s\n' '{"kind":"ok","text":"base\n"}' ;;
      *) printf '%s\n' '{"kind":"ok","text":""}' ;;
    esac ;;
  *"sandbox close"*) echo '{}' ;;
  *"branch merge"*) echo '{"merge":"m-1"}' ;;
  *"branch publish"*) echo '{}' ;;
  *) echo '{}' ;;
esac
"#,
        );
        let _env = EnvGuard::set(CLI_ENV, &script);
        let handle = WorkspaceHandle {
            project: "shop".to_string(),
            branch: "fleet-child-20261026t010000z-abc123".to_string(),
            sandbox: "1".to_string(),
            digests: Some(digests),
            folder: Some(folder.clone()),
            sync: None,
        };
        // The owner touched their file while the child worked.
        std::fs::write(folder.join("owners.txt"), "the owner's own edit\n").unwrap();
        let report = handle.close(true);
        let home = report.home.clone().expect("the settle decided an apply");
        assert_eq!(home.applied, vec!["childs.txt".to_string()], "{home:?}");
        assert_eq!(home.held, vec!["owners.txt".to_string()], "{home:?}");
        // main-moved.txt is in neither list: the child's version equals
        // the fork-time version, so main moved it, not the child.
        assert!(!home.applied.iter().any(|p| p == "main-moved.txt"));
        assert!(!home.held.iter().any(|p| p == "main-moved.txt"));
        // The merge landed, so the plan stands.
        assert!(
            matches!(report.merge, Some(MergeOutcome::Merged(_))),
            "{report:?}"
        );
        // And nothing was written by the settle itself — the apply is
        // the arm's act, gated.
        assert_eq!(
            std::fs::read_to_string(folder.join("childs.txt")).unwrap(),
            "base\n",
            "the settle decides; the arm applies"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// The other half: a merge that does not land applies nothing home
    /// — the branch is kept for resolution, and walking its work into
    /// the host would route around the very conflict the merge named.
    #[test]
    fn a_conflicted_merge_applies_nothing_home() {
        let _lock = crate::TEST_ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let root = std::env::temp_dir().join(format!(
            "fleet-layerfs-homeconf-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&root);
        let folder = root.join("repo");
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::write(folder.join("childs.txt"), "base\n").unwrap();
        let git = |args: &[&str]| {
            let status = std::process::Command::new("git")
                .arg("-C")
                .arg(&folder)
                .args(args)
                .env("GIT_AUTHOR_NAME", "t")
                .env("GIT_AUTHOR_EMAIL", "t@t")
                .env("GIT_COMMITTER_NAME", "t")
                .env("GIT_COMMITTER_EMAIL", "t@t")
                .status()
                .expect("git runs");
            assert!(status.success(), "git {args:?}");
        };
        git(&["init", "-q"]);
        git(&["add", "-A"]);
        git(&["commit", "-qm", "one file"]);
        let digests = content_hashes(&folder).expect("the snapshot");
        let script = stub(
            "homeconf",
            r#"case "$*" in
  *--version*) echo "1.1.0" ;;
  *"branch fork"*) echo '{"name":"ok"}' ;;
  *"sandbox create"*) echo '{"sandbox":"1","image":"sha256:x"}' ;;
  *"branch diff_branches"*)
    echo '{"items":[{"path":"/childs.txt","kind":"Modified"}],"hasMore":false}' ;;
  *"fs read"*) printf '%s\n' '{"kind":"ok","text":"the child wrote this\n"}' ;;
  *"sandbox close"*) echo '{}' ;;
  *"branch merge"*)
    echo '{"code":"Conflict","message":"both changed childs.txt","detail":{"conflicts":[{"path":"childs.txt"}]}}' >&2
    exit 125 ;;
  *) echo '{}' ;;
esac
"#,
        );
        let _env = EnvGuard::set(CLI_ENV, &script);
        let handle = WorkspaceHandle {
            project: "shop".to_string(),
            branch: "fleet-child-20261026t010000z-abc123".to_string(),
            sandbox: "1".to_string(),
            digests: Some(digests),
            folder: Some(folder.clone()),
            sync: None,
        };
        let report = handle.close(true);
        assert!(report.home.is_none(), "{report:?}");
        assert!(
            report
                .notes
                .iter()
                .any(|note| note.contains("nothing was applied home")),
            "{:?}",
            report.notes
        );
        assert_eq!(
            std::fs::read_to_string(folder.join("childs.txt")).unwrap(),
            "base\n"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// C1's pin: the spawn sync carries the host's newer paths onto the
    /// fresh fork, and leaves main-newer paths for the settle. The
    /// fixture: a git repo registered with tree T0; the host changes
    /// one file, adds another, deletes a third; main (the stub) reads
    /// back the recorded content for the carried paths (main is stale
    /// there) and a DIFFERENT version for the main-newer path.
    #[test]
    fn the_spawn_sync_carries_the_hosts_newer_paths() {
        let _lock = crate::TEST_ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let root = std::env::temp_dir().join(format!(
            "fleet-layerfs-sync-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&root);
        // The git fixture: three tracked files (modified, added, deleted
        // will touch the first two and drop the third).
        let folder = root.join("repo");
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::write(folder.join("modified.txt"), "base\n").unwrap();
        std::fs::write(folder.join("added.txt"), "placeholder\n").unwrap();
        std::fs::write(folder.join("deleted.txt"), "doomed\n").unwrap();
        std::fs::write(folder.join("main-newer.txt"), "recorded\n").unwrap();
        let git = |args: &[&str]| {
            let status = std::process::Command::new("git")
                .arg("-C")
                .arg(&folder)
                .args(args)
                .env("GIT_AUTHOR_NAME", "t")
                .env("GIT_AUTHOR_EMAIL", "t@t")
                .env("GIT_COMMITTER_NAME", "t")
                .env("GIT_COMMITTER_EMAIL", "t@t")
                .status()
                .expect("git runs");
            assert!(status.success(), "git {args:?}");
        };
        git(&["init", "-q"]);
        git(&["add", "-A"]);
        git(&["commit", "-qm", "the recorded tree"]);
        let recorded = worktree_tree(&folder).expect("the recorded tree");
        // The host moves on: modify one, add one, delete one. The
        // main-newer file stays as recorded (so its delta is none) —
        // its main-newer-ness is exercised in the classification below
        // by a stub that answers a DIFFERENT main version for it.
        std::fs::write(folder.join("modified.txt"), "the host's edit\n").unwrap();
        std::fs::write(folder.join("added.txt"), "the host added this\n").unwrap();
        std::fs::remove_file(folder.join("deleted.txt")).unwrap();

        // The stub: main reads back the RECORDED content for the
        // carried paths (main is stale there) and a changed version for
        // the main-newer path.
        let script = stub(
            "sync",
            r#"case "$*" in
  *--version*) echo "1.1.0" ;;
  *"branch fork"*) echo '{"name":"ok"}' ;;
  *"sandbox create"*) echo '{"sandbox":"1","image":"sha256:x"}' ;;
  *"fs read"*)
    case "$*" in
      *main-newer.txt*) printf '%s\n' '{"kind":"ok","text":"main moved this\n"}' ;;
      *modified.txt*) printf '%s\n' '{"kind":"ok","text":"base\n"}' ;;
      *added.txt*) printf '%s\n' '{"kind":"ok","text":"placeholder\n"}' ;;
      *deleted.txt*) printf '%s\n' '{"kind":"ok","text":"doomed\n"}' ;;
      *) printf '%s\n' '{"kind":"ok","text":""}' ;;
    esac ;;
  *"fs write"*) printf '%s
' '{"kind":"Committed"}' ;;
  *"fs rm"*) printf '%s
' '{"kind":"ok"}' ;;
  *) echo '{}' ;;
esac
"#,
        );
        let _env = EnvGuard::set(CLI_ENV, &script);
        let gate = Gate::resolve().expect("the stub answers 1.1.0");

        // The plan: the host's three delta paths classify — modified
        // and added carried (main reads the recorded content), deleted
        // carried (main lacks it), main-newer classified separately
        // below via a direct plan on a repo whose recorded tree lacks
        // the file's main version.
        let plan = plan_sync(&gate, "shop", &folder, &recorded).expect("the plan");
        let names: Vec<&str> = plan
            .to_child
            .iter()
            .map(|(path, _)| path.as_str())
            .collect();
        assert!(names.contains(&"modified.txt"), "{names:?}");
        assert!(names.contains(&"added.txt"), "{names:?}");
        assert!(names.contains(&"deleted.txt"), "{names:?}");
        assert!(
            !names.contains(&"main-newer.txt"),
            "main's version is unchanged on the host: {names:?}"
        );

        // The main-newer classification: a path the host DID change
        // (so it is a delta candidate) whose main version differs from
        // the recorded content — main moved, not the host.
        std::fs::write(folder.join("main-newer.txt"), "the host moved this too\n").unwrap();
        let plan = plan_sync(&gate, "shop", &folder, &recorded).expect("the plan");
        let names: Vec<&str> = plan
            .to_child
            .iter()
            .map(|(path, _)| path.as_str())
            .collect();
        assert!(
            !names.contains(&"main-newer.txt"),
            "main's version differs from the recorded one: {names:?}"
        );
        assert!(
            plan.main_newer.contains(&"main-newer.txt".to_string()),
            "{:?}",
            plan.main_newer
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// The sync rides the fork: open_handle carries the plan's paths
    /// onto the fresh branch before returning — the argv log names
    /// fs write for the host's content and fs rm for the deletion.
    #[test]
    fn the_fork_executes_the_sync_before_the_child_runs() {
        let _lock = crate::TEST_ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let root = std::env::temp_dir().join(format!(
            "fleet-layerfs-syncfork-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&root);
        let folder = root.join("repo");
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::write(folder.join("changed.txt"), "the host's newer bytes\n").unwrap();
        std::fs::write(folder.join("gone.txt"), "doomed\n").unwrap();
        let git = |args: &[&str]| {
            let status = std::process::Command::new("git")
                .arg("-C")
                .arg(&folder)
                .args(args)
                .env("GIT_AUTHOR_NAME", "t")
                .env("GIT_AUTHOR_EMAIL", "t@t")
                .env("GIT_COMMITTER_NAME", "t")
                .env("GIT_COMMITTER_EMAIL", "t@t")
                .status()
                .expect("git runs");
            assert!(status.success(), "git {args:?}");
        };
        git(&["init", "-q"]);
        git(&["add", "-A"]);
        git(&["commit", "-qm", "the recorded tree"]);
        let _recorded = worktree_tree(&folder).expect("the recorded tree");
        std::fs::write(
            folder.join("changed.txt"),
            "the host's newer bytes, edited\n",
        )
        .unwrap();
        std::fs::remove_file(folder.join("gone.txt")).unwrap();

        let script = stub(
            "syncfork",
            r#"case "$*" in
  *--version*) echo "1.1.0" ;;
  *"branch fork"*) echo '{"name":"ok"}' ;;
  *"sandbox create"*) echo '{"sandbox":"1","image":"sha256:x"}' ;;
  *"fs write"*)
    if [ -n "$FLEET_STUB_STDIN" ]; then cat > "$FLEET_STUB_STDIN"; else cat > /dev/null; fi
    printf '%s
' '{"kind":"Committed"}' ;;
  *"fs rm"*) printf '%s
' '{"kind":"ok"}' ;;
  *) echo '{}' ;;
esac
"#,
        );
        let _env = EnvGuard::set(CLI_ENV, &script);
        let plan = SyncPlan {
            to_child: vec![
                ("changed.txt".to_string(), 'M'),
                ("gone.txt".to_string(), 'D'),
            ],
            main_newer: Vec::new(),
        };
        let handle = WorkspaceHandle {
            project: "shop".to_string(),
            branch: "fleet-child-20261026t010000z-abc123".to_string(),
            sandbox: "1".to_string(),
            digests: None,
            folder: Some(folder.clone()),
            sync: Some(plan),
        };
        let gate = Gate::resolve().expect("the stub answers 1.1.0");
        // Drive the sync the way open_handle does: the plan's paths hit
        // the wire through the gate the handle carries.
        if let Some(plan) = &handle.sync {
            for (path, kind) in &plan.to_child {
                let view = format!("/{path}");
                match kind {
                    'D' => {
                        gate.fs_rm(&handle.project, &handle.branch, &view).unwrap();
                    }
                    _ => {
                        let bytes =
                            std::fs::read(handle.folder.as_ref().unwrap().join(path)).unwrap();
                        gate.fs_write(
                            &handle.project,
                            &handle.branch,
                            &view,
                            &bytes,
                            "fleet: sync from host",
                        )
                        .unwrap();
                    }
                }
            }
        }
        let argv = log_lines(&script).join("\n");
        assert!(
            argv.contains("fs write") && argv.contains("changed.txt"),
            "the host's newer content was carried: {argv}"
        );
        assert!(
            argv.contains("fs rm") && argv.contains("gone.txt"),
            "the deletion was carried: {argv}"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// C3's repair, pinned: the machine's facts MERGE into the config —
    /// the scaffold the install wrote a moment earlier and the
    /// registrations C2 recorded both survive beside the new section.
    /// The install's first pass replaced the whole file, which
    /// destroyed the scaffold and the owner's hand-edits in one write;
    /// the live run caught it as eleven red install tests.
    #[test]
    fn the_machine_facts_merge_into_the_config_beside_what_is_there() {
        let _lock = crate::TEST_ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let home = std::env::temp_dir().join(format!(
            "fleet-layerfs-machine-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&home);
        let layerfs_home = home.join("layerfs-home");
        let bundle = home.join("bundle");
        std::fs::create_dir_all(&layerfs_home).unwrap();
        std::fs::create_dir_all(&bundle).unwrap();
        let _home_env = EnvGuard::set("FLEET_HOME", &home);

        // The world as the install leaves it: the scaffold it just
        // wrote, and one registration C2 recorded.
        std::fs::write(
            home.join("config.json"),
            "{\n  \"pin\": null,\n  \"providers\": [],\n  \"layerfs\": {\n    \"projects\": {\n      \
             \"/repo\": { \"name\": \"repo\", \"volume\": \"v1\", \"store\": \"/volumes/v1/store\" }\n    \
             }\n  }\n}\n",
        )
        .unwrap();

        record_machine(&layerfs_home, &bundle, "sha256:runtime").expect("the facts record");

        let document: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(home.join("config.json")).unwrap())
                .unwrap();
        assert_eq!(document["pin"], serde_json::json!(null));
        assert_eq!(document["providers"], serde_json::json!([]));
        assert_eq!(
            document["layerfs"]["home"],
            layerfs_home.display().to_string()
        );
        assert_eq!(document["layerfs"]["bundle"], bundle.display().to_string());
        assert_eq!(document["layerfs"]["image"], "sha256:runtime");
        // The registration beside it is untouched.
        assert_eq!(document["layerfs"]["projects"]["/repo"]["name"], "repo");
        let _ = std::fs::remove_dir_all(&home);
    }

    /// C3's pin: the checkout identity names a 40-hex commit of the
    /// repository and the wire that tree speaks — moved together or
    /// not at all.
    #[test]
    fn the_checkout_pin_names_the_commit_and_the_wire() {
        let commit = CHECKOUT_PIN
            .split('@')
            .nth(1)
            .and_then(|rest| rest.split(' ').next())
            .expect("repo@commit");
        assert_eq!(commit.len(), 40, "{commit}");
        assert!(
            commit.chars().all(|ch| ch.is_ascii_hexdigit()),
            "a full lowercase hex commit: {commit}"
        );
        assert!(CHECKOUT_PIN.contains(SUPPORTED_WIRE), "{CHECKOUT_PIN}");
    }

    /// The three script names are the names this repository ships: the
    /// install copies `scripts/<name>` into the home by the constant,
    /// and the driver resolves the home's copy by the same one. The
    /// live run taught the cost of a mismatch — the install copied
    /// `layerfs-register.mjs`, the driver looked for `register.mjs`,
    /// and every clean-environment registration failed by name while
    /// the tests (each writing its own fixture's short name) stayed
    /// green.
    #[test]
    fn the_script_names_are_the_names_the_repository_ships() {
        for name in [REGISTER_SCRIPT, HISTORY_SCRIPT, WORKER_SCRIPT] {
            let shipped = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("scripts")
                .join(name);
            assert!(
                shipped.is_file(),
                "the driver spawns {name}, which scripts/ does not ship"
            );
        }
    }

    /// The live run's lesson, pinned: session stamps carry uppercase
    /// and `fleet/` a slash — both refused by the wire — so the arm's
    /// branch is folded into the wire's own grammar by construction.
    #[test]
    fn child_branch_name_folds_a_real_session_id_into_the_wires_rule() {
        let session = "20261025T183442Z-b340b2c9";
        let branch = child_branch_name(session);
        assert!(branch.starts_with("fleet-child-"), "{branch}");
        assert!(
            history_name_ok(&branch).is_ok(),
            "a real session id folds to a valid branch: {branch}"
        );
        assert_eq!(branch, "fleet-child-20261025t183442z-b340b2c9");
        let merge = merge_branch_name(&branch);
        assert!(history_name_ok(&merge).is_ok(), "{merge}");
        assert_eq!(merge, "merge-fleet-child-20261025t183442z-b340b2c9");
        // Long and hostile input still bounds to 63 and ends alnum.
        let long = child_branch_name(&"x/".repeat(80));
        assert!(long.len() <= 63, "{}", long.len());
        assert!(history_name_ok(&long).is_ok(), "{long}");
    }

    /// The wire's refusal raised HERE, before a spawn spends anything:
    /// an illegal branch names itself and the stub records no fork.
    #[test]
    fn open_refuses_an_illegal_branch_before_it_spawns() {
        let _lock = crate::TEST_ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let script = stub("illegal-branch", HAPPY);
        let _env = EnvGuard::set(CLI_ENV, &script);
        let error = WorkspaceSession::open("shop", "fleet/bad")
            .expect_err("the slash is not the wire's grammar");
        match &error {
            Error::Refused { code, message } => {
                assert_eq!(code, "WorkspaceOpen");
                assert!(message.contains("not a valid LayerFS name"), "{message}");
            }
            other => panic!("the named refusal, got {other:?}"),
        }
        let lines = log_lines(&script);
        assert!(
            !lines.iter().any(|line| line.contains("branch fork")),
            "nothing spawned: {lines:?}"
        );
    }

    /// C0's cadence pin: one bash call walks mount -> exec ->
    /// commit -> unmount in that order, and the answer parses into
    /// the outcome the arm renders.
    #[test]
    fn run_command_walks_the_cadence_in_order() {
        let _lock = crate::TEST_ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let script = stub("cadence", HAPPY);
        let _env = EnvGuard::set(CLI_ENV, &script);
        let gate = Gate::resolve().expect("the stub answers 1.1.0");
        let ctx = WorkspaceHandle {
            project: "shop".to_string(),
            branch: child_branch_name("20261026T010000Z-abc123"),
            sandbox: "2".to_string(),
            digests: None,
            folder: None,
            sync: None,
        };
        let outcome = gate
            .run_command(&ctx, "cargo test --lib", Duration::from_secs(5))
            .expect("the cadence completes");
        assert_eq!(outcome.exit_code, 0);
        assert_eq!(outcome.stdout, "ran\n");
        assert_eq!(outcome.steps, vec!["mount", "exec", "commit", "unmount"]);
        let lines = log_lines(&script);
        let at = |needle: &str| {
            lines
                .iter()
                .position(|line| line.contains(needle))
                .unwrap_or_else(|| panic!("`{needle}` in the argv: {lines:?}"))
        };
        let (m, e, c, u) = (
            at("workspace mount"),
            at("workspace exec"),
            at("workspace commit"),
            at("workspace unmount"),
        );
        assert!(
            m < e && e < c && c < u,
            "mount < exec < commit < unmount: {lines:?}"
        );
        assert!(
            lines[c].contains("fleet: bash cargo test --lib"),
            "the checkpoint names the command: {}",
            lines[c]
        );
        assert!(
            !lines[u].contains("--discard"),
            "a clean unmount: {}",
            lines[u]
        );
    }

    /// Every step restorable: a command that exits 3 still
    /// checkpoints (the parent's merge gate decides what reaches
    /// main), and the exit rides the answer.
    #[test]
    fn a_failing_command_still_checkpoints() {
        let _lock = crate::TEST_ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let script = stub("execfail", HAPPY);
        let _env = EnvGuard::set(CLI_ENV, &script);
        let _mode = EnvGuard::set("FLEET_STUB_MODE", "execfail");
        let gate = Gate::resolve().expect("the stub answers 1.1.0");
        let ctx = WorkspaceHandle {
            project: "shop".to_string(),
            branch: "fleet-child-a".to_string(),
            sandbox: "1".to_string(),
            digests: None,
            folder: None,
            sync: None,
        };
        let outcome = gate
            .run_command(&ctx, "false", Duration::from_secs(5))
            .expect("a nonzero exit is an answer, not an error");
        assert_eq!(outcome.exit_code, 3);
        assert_eq!(outcome.stderr, "boom\n");
        let lines = log_lines(&script);
        assert!(
            lines.iter().any(|line| line.contains("workspace commit")),
            "the checkpoint ran anyway: {lines:?}"
        );
    }

    /// A hung command meets its budget once: the call is killed at
    /// the deadline, the overlay is discarded (its half-state exists
    /// nowhere else), and the typed timeout is what returns.
    #[test]
    fn a_hung_command_is_discarded_at_its_budget() {
        let _lock = crate::TEST_ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let script = stub("execsleep", HAPPY);
        let _env = EnvGuard::set(CLI_ENV, &script);
        let _mode = EnvGuard::set("FLEET_STUB_MODE", "execsleep");
        let gate = Gate::resolve().expect("the stub answers 1.1.0");
        let ctx = WorkspaceHandle {
            project: "shop".to_string(),
            branch: "fleet-child-b".to_string(),
            sandbox: "1".to_string(),
            digests: None,
            folder: None,
            sync: None,
        };
        let started = std::time::Instant::now();
        let error = gate
            .run_command(&ctx, "sleep-forever", Duration::from_millis(300))
            .expect_err("the budget expires");
        assert!(matches!(error, Error::Timeout { .. }), "{error}");
        assert!(
            started.elapsed() < Duration::from_secs(3),
            "one attempt, bounded"
        );
        let lines = log_lines(&script);
        let unmount = lines
            .iter()
            .position(|line| line.contains("workspace unmount"))
            .expect("the cleanup unmount ran");
        assert!(
            lines[unmount].contains("--discard"),
            "the half-state is discarded, never committed: {}",
            lines[unmount]
        );
        assert!(
            !lines.iter().any(|line| line.contains("workspace commit")),
            "no commit after a killed exec: {lines:?}"
        );
    }

    /// The fs verbs speak the CLI's grammar, and `fs write` carries
    /// its body on stdin exactly.
    #[test]
    fn the_fs_verbs_speak_the_clis_grammar_and_write_rides_stdin() {
        let _lock = crate::TEST_ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let script = stub("fs-verbs", HAPPY);
        let _env = EnvGuard::set(CLI_ENV, &script);
        let stdin_cap = script.parent().unwrap().join("stdin.bin");
        let _stdin_env = EnvGuard::set("FLEET_STUB_STDIN", stdin_cap.to_str().unwrap());
        let gate = Gate::resolve().expect("the stub answers 1.1.0");

        gate.fs_read("shop", "fleet-child-a", "/src/main.rs", Some((1, 40)))
            .expect("read");
        gate.fs_list("shop", "fleet-child-a", "/src").expect("list");
        gate.fs_write(
            "shop",
            "fleet-child-a",
            "/src/new.rs",
            b"fn main() {}\n\x00binary",
            "fleet: write_file",
        )
        .expect("write");
        gate.fs_edit("shop", "fleet-child-a", "/a.txt", "old", "new", "m")
            .expect("edit");
        gate.fs_grep("shop", "fleet-child-a", "needle", Some("/src"), None)
            .expect("grep");
        gate.fs_glob("shop", "fleet-child-a", "**/*.rs", None)
            .expect("glob");

        let body = std::fs::read(&stdin_cap).expect("the stdin capture");
        assert_eq!(body, b"fn main() {}\n\x00binary", "bytes travel exactly");
        let lines = log_lines(&script);
        assert!(
            lines.iter().any(|line| {
                line
                == "--project shop fs read fleet-child-a /src/main.rs --start-line 1 --end-line 40"
            }),
            "read grammar: {lines:?}"
        );
        assert!(
            lines.iter().any(|line| line
                == "--project shop fs edit fleet-child-a /a.txt -m m --old old --new new"),
            "edit grammar (exactly-one-match is the CLI's): {lines:?}"
        );
        assert!(
            lines
                .iter()
                .any(|line| line == "--project shop fs grep fleet-child-a needle /src"),
            "grep grammar: {lines:?}"
        );
        assert!(
            lines
                .iter()
                .any(|line| line == "--project shop fs glob fleet-child-a **/*.rs"),
            "glob grammar: {lines:?}"
        );
    }

    /// The view path rule: re-rooting, normalization, and the
    /// escape that cannot be spelled.
    #[test]
    fn view_path_re_roots_and_refuses_escapes() {
        assert_eq!(view_path("src/main.rs").unwrap(), "/src/main.rs");
        assert_eq!(view_path("./a//b/").unwrap(), "/a/b");
        assert_eq!(view_path("/abs/x.txt").unwrap(), "/abs/x.txt");
        assert_eq!(view_path("a/../b").unwrap(), "/b");
        assert_eq!(view_path("/").unwrap(), "/");
        let escape = view_path("../etc/passwd").expect_err("pops past the root");
        assert!(escape.contains("leaves the workspace view"), "{escape}");
        let deep = view_path("/a/../../../b").expect_err("absolute escape too");
        assert!(deep.contains("leaves the workspace view"), "{deep}");
        assert!(view_path("   ").is_err(), "an empty path is named");
    }

    #[test]
    fn the_knob_reads_unset_valid_and_invalid_by_name() {
        let _lock = crate::TEST_ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let _env = EnvGuard::set(PROJECT_ENV, std::path::Path::new(""));
        assert_eq!(project_name().unwrap(), None, "empty is unset");
        std::env::set_var(PROJECT_ENV, "shop-floor-2");
        assert_eq!(project_name().unwrap(), Some("shop-floor-2".to_string()));
        std::env::set_var(PROJECT_ENV, "Shop");
        let error = project_name().expect_err("uppercase is not a project name");
        assert!(
            error.contains(PROJECT_ENV) && error.contains("Shop"),
            "{error}"
        );
        std::env::set_var(PROJECT_ENV, "-leading");
        assert!(project_name().is_err(), "a leading dash is refused");
        let long = "a".repeat(64);
        std::env::set_var(PROJECT_ENV, &long);
        assert!(project_name().is_err(), "64 characters is past the grammar");
        std::env::remove_var(PROJECT_ENV);
    }
}
