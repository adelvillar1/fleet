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
use std::path::PathBuf;
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

/// The context a [`FileAccess::LayerFs`] run works in — which
/// project's which branch, in which sandbox. The child's whole
/// filesystem identity in one value; the arm carries it, the dispatch
/// hands it to the driver, and nothing else about the run changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceHandle {
    pub project: String,
    pub branch: String,
    pub sandbox: String,
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
}

impl Gate {
    /// Resolve the CLI and read its wire version — fresh, uncached.
    /// [`gate`] memoizes this once per process; doctor and the tests
    /// call it directly so a stub can be the CLI under test.
    pub fn resolve() -> Result<Gate, String> {
        let cli = match std::env::var_os(CLI_ENV) {
            Some(path) if !path.is_empty() => path,
            _ => OsString::from("layerfs"),
        };
        let answered = run_version(&cli)?;
        if answered == SUPPORTED_WIRE {
            Ok(Gate { cli })
        } else {
            Err(Error::Wire { answered }.to_string())
        }
    }

    /// A Gate over any executable, for the surfaces' tests: nothing
    /// resolves, nothing is spawned — the caller names the CLI.
    #[cfg(test)]
    pub fn for_test(cli: OsString) -> Gate {
        Gate { cli }
    }

    fn call(&self, call: &str, args: &[&str]) -> Result<serde_json::Value, Error> {
        let body = spawn(&self.cli, call, args, CONTROL_BUDGET)?;
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
        let raw = spawn_with(&self.cli, "branch merge", &args, CONTROL_BUDGET, None)?;
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
            &[
                "--project",
                project,
                "fs",
                "write",
                branch,
                path,
                "-m",
                message,
            ],
            CONTROL_BUDGET,
            Some(content),
        )?;
        match serde_json::from_str::<serde_json::Value>(raw.stdout.trim()) {
            Ok(value) => Ok(value),
            Err(_) => Err(Error::Decode {
                call: "fs write".to_string(),
                body: bounded(&raw.stdout),
            }),
        }
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
            &["--project", project, "workspace", "exec", ws, command],
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
pub fn gate() -> &'static Result<Gate, String> {
    static GATE: std::sync::OnceLock<Result<Gate, String>> = std::sync::OnceLock::new();
    GATE.get_or_init(Gate::resolve)
}

/// Read the CLI's wire version: the last `x.y.z` token of its first
/// `--version` line. A CLI that answers nothing version-shaped is an
/// absence, named.
fn run_version(cli: &OsString) -> Result<String, String> {
    let body =
        spawn(cli, "--version", &["--version"], VERSION_BUDGET).map_err(|error| match error {
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
    let deadline = std::time::Instant::now() + budget;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let output = child
                    .wait_with_output()
                    .map_err(|error| Error::Spawn(format!("{call}: {error}")))?;
                return Ok(Raw {
                    success: status.success(),
                    exit: status.code().unwrap_or(-1),
                    stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                    stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                });
            }
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
    }
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
    pub fn open_handle(project: &str, branch: &str) -> Result<WorkspaceHandle, Error> {
        // The wire's name rule, checked before anything spawns: an
        // illegal branch is refused here, named, with no round trip —
        // the live run taught that the stub would have accepted it.
        history_name_ok(branch).map_err(|reason| Error::Refused {
            code: "WorkspaceOpen".to_string(),
            message: format!("the child's branch would be refused by the wire: {reason}"),
        })?;
        let gate = match gate() {
            Ok(gate) => gate.clone(),
            Err(reason) => return Err(Error::Absent(reason.clone())),
        };
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
        Ok(WorkspaceHandle {
            project: project.to_string(),
            branch: branch.to_string(),
            sandbox,
        })
    }

    /// The mounted lifecycle over a fresh handle — the C0/C2 shape
    /// the probe and the live roundtrip drive directly. The arm uses
    /// `open_handle` + per-command mounts instead; both share the
    /// fork half above.
    pub fn open(project: &str, branch: &str) -> Result<WorkspaceSession, Error> {
        let handle = Self::open_handle(project, branch)?;
        let gate = match gate() {
            Ok(gate) => gate.clone(),
            Err(reason) => return Err(Error::Absent(reason.clone())),
        };
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
            Ok(gate) => gate.clone(),
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
        Ok(MergeOutcome::Merged(_)) => match gate.publish(project, &integration) {
            Ok(_) => report.merge = Some(MergeOutcome::Merged(serde_json::Value::Null)),
            Err(error) => {
                report.notes.push(format!(
                    "the merge landed on `{integration}` but the publish to main failed: {error}                      — the integration branch is kept"
                ));
                report.merge = Some(MergeOutcome::Merged(serde_json::Value::Null));
            }
        },
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
            Ok(gate) => gate.clone(),
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
        merge_back(&gate, &self.project, &self.branch, &mut report);
        report
    }
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

    /// Env the tests mutate is process-global; one lock keeps a stub
    /// path from being read mid-swap by another test thread.
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

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
        let _lock = ENV_LOCK
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
        let _lock = ENV_LOCK
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
        let _lock = ENV_LOCK
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
        let _lock = ENV_LOCK
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
        let _lock = ENV_LOCK
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
        let _lock = ENV_LOCK
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
        let _lock = ENV_LOCK
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
        let _lock = ENV_LOCK
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
        let _lock = ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let script = stub("cadence", HAPPY);
        let _env = EnvGuard::set(CLI_ENV, &script);
        let gate = Gate::resolve().expect("the stub answers 1.1.0");
        let ctx = WorkspaceHandle {
            project: "shop".to_string(),
            branch: child_branch_name("20261026T010000Z-abc123"),
            sandbox: "2".to_string(),
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
        let _lock = ENV_LOCK
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
        let _lock = ENV_LOCK
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
        let _lock = ENV_LOCK
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
        let _lock = ENV_LOCK
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
