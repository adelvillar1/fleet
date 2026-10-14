//! `fleet install`: the home layout, the config scaffold, the optional
//! heavy-lane venvs, and the doctor gate.
//!
//! The fork's `services/bootstrap.mjs` vocabulary, ported natively:
//! per-lane, idempotent (present ⇒ detected and named, never rebuilt),
//! degrade-without (a lane whose venv cannot be built — no network, a
//! failing pip, a missing python — is skipped with a named line and the
//! install still succeeds, the lane staying declared and dead exactly
//! as slice 4's dead-lane behavior already is). No Node and no
//! checkout dependency: the venvs are built by the machine's own `uv`
//! or `python3`, and the fork stays the read-only spec.
//!
//! The bare `fleet install` lays out the home, writes the config
//! scaffold only when absent, and names the lane step as available —
//! so a clean-machine install never blocks on a network. `--lanes`
//! runs the per-lane bootstrap. Either way the command ends on the
//! doctor's own checks: the install adds no new check and its exit
//! code is the doctor's.
//!
//! Every lane subprocess runs under
//! [`LANE_INSTALL_TIMEOUT_SECS`], and its lines stream to the operator
//! as they arrive — a multi-minute `uv pip install` of the torch stack
//! cannot look frozen, and a tool that has not finished within the
//! bound is killed: the lane is skipped naming the bound, never a hung
//! install. A lane's venv carries the installer's receipt
//! ([`LANE_RECEIPT`]) once its packages land, so an interrupted run's
//! half-built venv reads not-built and is retried rather than misread
//! as Present.

use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant, SystemTime};

use palette::{ACCENT, BOLD, DIM, OK, RESET, TEXT};

/// The ANSI palette, mirroring the other headless surfaces' modules so
/// the install reads as one surface with `fleet lanes` and `fleet
/// doctor`.
mod palette {
    pub const ACCENT: &str = "\u{1b}[36m"; // cyan
    pub const OK: &str = "\u{1b}[32m"; // green
    pub const DIM: &str = "\u{1b}[90m"; // dark gray
    pub const TEXT: &str = "\u{1b}[37m"; // gray
    pub const BOLD: &str = "\u{1b}[1m";
    pub const RESET: &str = "\u{1b}[0m";
}

/// The home layout's directories, in creation order. One per consumer
/// in the crate: `sessions` (the append-only JSONL logs), `memories`
/// (the durable-fact corpus), `skills` (the SKILL.md tree), `cron`
/// (the job store), `proposals` (the self-maintenance gate's pending
/// changes).
pub const LAYOUT_DIRS: &[&str] = &["sessions", "memories", "skills", "cron", "proposals"];

/// The config file's name inside the home: the roster's override
/// document, merged over the compiled-in defaults by
/// [`crate::route::load_roster`].
pub const CONFIG_FILE: &str = "config.json";

/// The scaffold's exact bytes: a minimal valid override document — no
/// pin, no provider overrides — so the compiled-in defaults stand as
/// the roster and this file is the seam an owner edits. Written only
/// when absent; an existing file is named and never rewritten.
pub const CONFIG_SCAFFOLD: &str = "{\n  \"pin\": null,\n  \"providers\": []\n}\n";

/// The credential environment name the install's note carries. The
/// resolution chain — process env, then `<home>/.env`, then the
/// DeepSeek Harness credential store's `refs` block (read-only) — is
/// named in the install's output and in the feature doc; no value ever
/// appears in either.
pub const CREDENTIAL_ENV: &str = "DEEPSEEK_API_KEY";

/// The bound every lane subprocess runs under. The torch stack the
/// sys1 lane installs legitimately takes minutes, so the bound is
/// generous — but forever is not a bound: a wedged download, a tool
/// that waits on a terminal, a child that never exits is killed at
/// the bound and the lane is skipped naming it, rather than hanging
/// the install for the operator's whole session.
pub const LANE_INSTALL_TIMEOUT_SECS: u64 = 900;

/// The same bound as a [`Duration`], the form the tool runner takes.
const LANE_INSTALL_TIMEOUT: Duration = Duration::from_secs(LANE_INSTALL_TIMEOUT_SECS);

/// The grace a tool gets to exit once its output pipes have closed:
/// the tools close their pipes as they finish, so anything past this
/// is a child that will not end — killed and named, never an
/// unbounded wait (the eval probe's own grace).
const TOOL_EXIT_GRACE: Duration = Duration::from_secs(5);

// --- the home layout --------------------------------------------------------

/// What one layout step found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutStep {
    /// The directory was absent; it was created.
    Created,
    /// The directory was already there; nothing changed.
    Present,
}

/// Create one layout directory under `home`, idempotently. A second
/// run reports [`LayoutStep::Present`] and touches nothing.
/// @return whether the directory was created or already present.
pub fn layout_step(home: &Path, name: &str) -> std::io::Result<LayoutStep> {
    let dir = home.join(name);
    if dir.is_dir() {
        return Ok(LayoutStep::Present);
    }
    fs::create_dir_all(&dir)?;
    Ok(LayoutStep::Created)
}

/// What the config step found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigStep {
    /// The file was absent; the scaffold was written.
    Written,
    /// The file was already there; it was named and left untouched.
    Present,
    /// A symlink sits at the config path — dangling or not. It is
    /// named and left exactly as it is: `exists()` follows the link,
    /// so a dangling one reads absent and the scaffold would be
    /// written *through* it into whatever the link points at.
    Symlink,
}

/// Write the config scaffold when absent; name and never rewrite an
/// existing file, whatever it holds — an owner's hand-edited config is
/// theirs, and the install is not the place it gets clobbered.
///
/// The absence test is [`fs::symlink_metadata`], which does not follow
/// the final component: a symlink at the config path is a named
/// [`ConfigStep::Symlink`] and is never written through, so a dangling
/// link is left dangling rather than becoming a scaffold silently
/// planted in the link's target.
/// @return the step's outcome and the config file's path.
pub fn config_step(home: &Path) -> std::io::Result<(ConfigStep, PathBuf)> {
    let path = home.join(CONFIG_FILE);
    match fs::symlink_metadata(&path) {
        Ok(metadata) => {
            let step = if metadata.file_type().is_symlink() {
                ConfigStep::Symlink
            } else {
                ConfigStep::Present
            };
            Ok((step, path))
        }
        // Absent — and only genuinely absent: a dangling symlink
        // resolves its metadata above and never reaches this arm.
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            fs::write(&path, CONFIG_SCAFFOLD)?;
            Ok((ConfigStep::Written, path))
        }
        Err(error) => Err(error),
    }
}

// --- the heavy lanes ---------------------------------------------------------

/// What a lane's environment name addresses: `SEM1_ST_VENV` and
/// `SDM1_MODELS_VENV` are venv roots the core appends `bin/python` to,
/// while `GEN1_KOKORO_PYTHON` is the venv's python binary itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnvTarget {
    /// The venv root; the core appends `bin/python`.
    Root,
    /// The venv's python binary.
    Python,
}

/// One heavy lane: the vendored core it serves, the packages its venv
/// carries, and the wiring the core reads to find the venv. The
/// vocabulary is the fork's `services/bootstrap.mjs` ported to the
/// vendored cores; the venvs live at `<home>/venvs/<name>/`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Lane {
    /// The vendored core's name — also the venv directory's name.
    pub name: &'static str,
    /// The core's role, one word.
    pub role: &'static str,
    /// The packages the venv carries, the fork's package sets.
    pub packages: &'static [&'static str],
    /// The environment name the core reads to find its venv, when it
    /// reads one: sys1's is a config-file key, named in `note`.
    pub env_var: Option<&'static str>,
    /// Whether that name addresses the venv root or its python binary.
    pub env_target: EnvTarget,
    /// The step beyond the environment name, when the core needs one:
    /// sys1's `config.toml` key, gen1's machine-owner ONNX weights.
    /// `<venv>` is substituted with the lane's venv path.
    pub note: &'static str,
}

/// The five lanes, one per vendored core, in the fork's order.
///
/// The package sets are the fork's, with one fix at the vendoring
/// boundary: `gliner2` declares its local-model stack — torch,
/// transformers, numpy, peft, safetensors — under the `local` extra,
/// and `gliner2/models/base.py` imports torch at module load, so the
/// bare package the fork (and sys1's own setup note) names produces a
/// venv that cannot serve. The lane installs `gliner2[local]`.
///
/// blend1's own core is stdlib-only, so its venv carries the union of
/// its three siblings' package sets — the deps its lanes compose — and
/// the cores themselves are path-wired through the sibling
/// `<SVC>_PYTHONPATH` names its note names, never pip-installed.
pub const LANES: &[Lane] = &[
    Lane {
        name: "sys1",
        role: "judgment",
        packages: &["gliner2[local]"],
        env_var: None,
        env_target: EnvTarget::Root,
        note: "point ~/.config/sys1/config.toml at it: [providers] local_venv = \"<venv>\"",
    },
    Lane {
        name: "sdm1",
        role: "tabular",
        packages: &["structured-data-models"],
        env_var: Some("SDM1_MODELS_VENV"),
        env_target: EnvTarget::Root,
        note: "",
    },
    Lane {
        name: "sem1",
        role: "embeddings",
        packages: &["sentence-transformers"],
        env_var: Some("SEM1_ST_VENV"),
        env_target: EnvTarget::Root,
        note: "",
    },
    Lane {
        name: "gen1",
        role: "media",
        packages: &["kokoro-onnx"],
        env_var: Some("GEN1_KOKORO_PYTHON"),
        env_target: EnvTarget::Python,
        note: "the ONNX weights (~/.cache/gen1/tts/kokoro-v1.0.onnx and voices-v1.0.bin) are machine-owner downloads, not installed here",
    },
    Lane {
        name: "blend1",
        role: "grounded",
        // The union of the three siblings' sets: blend1's own core is
        // stdlib-only, and its venv exists to serve the sibling cores
        // its lanes compose — sys1's classifier stack, sdm1's tabular
        // stack, sem1's sentence-transformers — not blend1 itself.
        packages: &["gliner2[local]", "structured-data-models", "sentence-transformers"],
        env_var: Some("BLEND1_PYTHONPATH"),
        env_target: EnvTarget::Root,
        // The cores are path-wired, not installed here: the sibling
        // `<SVC>_PYTHONPATH` names point at the repo's vendored
        // `services/<svc>/src`, the house mechanism sem1's adapter
        // already reads.
        note: "the sibling cores are path-wired, not installed here: BLEND1_SYS1_PYTHONPATH / BLEND1_SEM1_PYTHONPATH / BLEND1_SDM1_PYTHONPATH / BLEND1_GEN1_PYTHONPATH point at the repo's vendored services/<svc>/src",
    },
];

/// One lane's rendered result: the lane and what its bootstrap found.
pub type LaneResult = (&'static Lane, LaneOutcome);

/// One lane's bootstrap outcome. Never a failure of the install: a
/// lane that cannot be built stays declared and dead — slice 4's
/// dead-lane behavior — with its reason named.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LaneOutcome {
    /// The venv was already there: detected and named, not rebuilt.
    Present,
    /// The venv was created and its packages installed.
    Installed,
    /// The venv could not be built; the reason is named.
    Skipped { reason: String },
}

/// The venvs root: `<home>/venvs`.
pub fn venvs_dir(home: &Path) -> PathBuf {
    home.join("venvs")
}

/// One lane's venv: `<home>/venvs/<lane>`.
pub fn venv_dir(home: &Path, lane: &Lane) -> PathBuf {
    venvs_dir(home).join(lane.name)
}

/// The venv's python — one half of the presence check.
fn venv_python(venv: &Path) -> PathBuf {
    venv.join("bin").join("python")
}

/// The receipt file's name inside a lane's venv: the installer's own
/// record that the venv's packages landed. It sits beside the venv's
/// python, so it travels with the venv.
pub const LANE_RECEIPT: &str = ".fleet-lane";

/// The receipt's path inside one lane's venv.
fn receipt_path(venv: &Path) -> PathBuf {
    venv.join(LANE_RECEIPT)
}

/// Whether the venv is a finished lane: its python exists **and** the
/// installer's receipt names the lane. The python alone is created by
/// `uv venv` before the packages are installed, so it is not evidence
/// of a finished lane — a Ctrl-C or an OOM-kill mid-install leaves it
/// in place, and without the receipt the next run would read Present
/// over a venv whose packages never landed.
fn venv_is_finished(venv: &Path, lane: &Lane) -> bool {
    venv_python(venv).exists() && receipt_names(venv, lane)
}

/// Whether the receipt beside the venv's python names this lane. A
/// receipt is written by the installer, so a venv without one is a
/// half-built venv — an interrupted prior run, a hand-made directory,
/// a venv built by a pre-receipt version of the installer — and is
/// treated as not-built: rebuilt or skipped, never misread as Present.
fn receipt_names(venv: &Path, lane: &Lane) -> bool {
    fs::read_to_string(receipt_path(venv))
        .map(|text| {
            text.lines()
                .any(|line| line.trim() == format!("lane {}", lane.name))
        })
        .unwrap_or(false)
}

/// Write the lane's receipt once its packages have landed: the lane's
/// name and role, its package list, and the UTC minute it finished.
/// The file is the only thing separating a venv whose packages are in
/// from one an interrupted run left half-built, so a failure to write
/// it is a failure of the lane and is named by the caller.
fn write_receipt(venv: &Path, lane: &Lane, finished: &str) -> std::io::Result<()> {
    let text = format!(
        "lane {}\nrole {}\npackages {}\ninstalled {}\n",
        lane.name,
        lane.role,
        lane.packages.join(" "),
        finished
    );
    fs::write(receipt_path(venv), text)
}

/// The UTC minute, the resolution a receipt carries: a venv lands over
/// minutes, so the minute is the honest stamp. A clock that cannot be
/// read names itself rather than inventing a time.
fn utc_minute() -> String {
    crate::session::iso_utc_stamp(SystemTime::now())
        .map(|stamp| stamp.chars().take(16).collect())
        .unwrap_or_else(|_| "unknown".to_string())
}

/// The wiring line a lane's core reads once its venv exists: the
/// export for an environment-addressed core, or the config note for
/// sys1's config-file key. `<venv>` in the note is substituted with
/// the lane's venv path.
pub fn lane_wiring(lane: &Lane, venv: &Path) -> String {
    let mut parts: Vec<String> = Vec::new();
    if let Some(env_var) = lane.env_var {
        let value = match lane.env_target {
            EnvTarget::Root => venv.display().to_string(),
            EnvTarget::Python => venv_python(venv).display().to_string(),
        };
        parts.push(format!("export {env_var}={value}"));
    }
    if !lane.note.is_empty() {
        parts.push(lane.note.replace("<venv>", &venv.display().to_string()));
    }
    parts.join("; ")
}

/// Why a tool did not do its work.
#[derive(Debug, Clone, PartialEq, Eq)]
enum ToolError {
    /// The program did not resolve on the child's PATH.
    NotFound { program: String },
    /// The program ran and exited non-zero. The detail is its last
    /// output line — the fork's `run()` convention.
    Failed { detail: String },
    /// The bound expired: the child was killed, so the install cannot
    /// hang on a wedged tool. The bound is part of the name, so a
    /// test's short bound and production's long one both read truly.
    TimedOut { program: String, bound: Duration },
}

impl ToolError {
    /// The named line the install prints.
    fn describe(&self) -> String {
        match self {
            ToolError::NotFound { program } => format!("{program} is not on PATH"),
            ToolError::Failed { detail } => detail.clone(),
            ToolError::TimedOut { program, bound } => {
                format!("{program} did not finish within {bound:?} and was killed")
            }
        }
    }
}

/// One lane's subprocess run: the child's PATH seam, the lane's name
/// for the progress prefix, the sink the child's lines stream to, and
/// the bound every tool runs under. Built once per lane and handed to
/// each of its tools.
struct LaneRun<'a> {
    /// The child's PATH when given — the seam a test drives a
    /// deterministic outcome through — and the caller's environment
    /// otherwise.
    path: Option<&'a OsStr>,
    /// The lane's name, so a streamed line says which lane it is from.
    lane: &'a str,
    /// Where the child's lines go as they arrive.
    progress: &'a mut (dyn std::io::Write + Send),
    /// The bound every tool in this run is held to.
    bound: Duration,
}

/// One line from a lane subprocess, on its way to the operator. The
/// stream it came from is kept, so a failure's named last line still
/// prefers stderr — the fork's `run()` convention.
enum ToolLine {
    Out(String),
    Err(String),
}

/// One reader thread: every line the child writes on one pipe, sent
/// on as it arrives. A reader owns its pipe and nothing else, so the
/// parent stays free to enforce the bound and to do the writing — the
/// sink is never shared between threads.
fn pump(pipe: impl std::io::Read, tx: mpsc::Sender<ToolLine>, line: impl Fn(String) -> ToolLine) {
    use std::io::BufRead as _;
    for text in std::io::BufReader::new(pipe).lines() {
        match text {
            Ok(text) => {
                if tx.send(line(text)).is_err() {
                    return;
                }
            }
            // An unreadable pipe: the child's output ends here.
            Err(_) => return,
        }
    }
}

/// Run one tool under the run's bound, streaming its lines to the
/// operator as they arrive and capturing them for the failure's named
/// last line.
///
/// One reader thread per pipe drains the child's output — a chatty
/// child (a `uv pip install` of the torch stack writes to both pipes)
/// cannot deadlock the wait — and the parent waits with a deadline.
/// A tool that has not finished within the bound is killed and the
/// lane above is skipped naming it, so the install never hangs.
/// `Ok(())` on a zero exit; otherwise the named reason.
fn run_tool(program: &str, args: &[&str], run: &mut LaneRun<'_>) -> Result<(), ToolError> {
    let mut command = Command::new(program);
    command.args(args);
    // Null stdin: a tool that reads the terminal must not be able to
    // hold the install open, piped or not.
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(path) = run.path {
        command.env("PATH", path);
    }
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err(ToolError::NotFound { program: program.to_string() });
        }
        Err(error) => {
            return Err(ToolError::Failed { detail: format!("{program}: {error}") });
        }
    };
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let (tx, rx) = mpsc::channel::<ToolLine>();
    let out_tx = tx.clone();
    let err_tx = tx;
    // Detached readers rather than scoped ones: killing the child does
    // not kill its grandchildren, and a pipe one of them still holds
    // open keeps that reader alive until it exits. A scoped reader
    // would hold the install open for exactly as long, so the readers
    // are fire-and-forget and the bound above is the only thing that
    // bounds the install.
    if let Some(pipe) = stdout {
        thread::spawn(move || pump(pipe, out_tx, ToolLine::Out));
    }
    if let Some(pipe) = stderr {
        thread::spawn(move || pump(pipe, err_tx, ToolLine::Err));
    }
    let deadline = Instant::now() + run.bound;
    let mut out = String::new();
    let mut err = String::new();
    let status = loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        match rx.recv_timeout(remaining) {
            Ok(ToolLine::Out(text)) => {
                out.push_str(&text);
                out.push('\n');
                let _ = writeln!(run.progress, "  {} | {text}", run.lane);
            }
            Ok(ToolLine::Err(text)) => {
                err.push_str(&text);
                err.push('\n');
                let _ = writeln!(run.progress, "  {} | {text}", run.lane);
            }
            // Both pipes closed: the child's output is complete.
            Err(RecvTimeoutError::Disconnected) => {
                break Some(wait_for_exit(&mut child, program))
            }
            Err(RecvTimeoutError::Timeout) => {
                // The bound expired. Kill the child, which closes the
                // pipes it owns and ends the readers it does not share
                // with a grandchild.
                let _ = child.kill();
                let _ = child.wait();
                break None;
            }
        }
    };
    let status = match status {
        Some(Ok(status)) => status,
        Some(Err(failure)) => return Err(failure),
        None => return Err(ToolError::TimedOut { program: program.to_string(), bound: run.bound }),
    };
    if status.success() {
        return Ok(());
    }
    let code = status.code().unwrap_or(-1);
    let detail = last_line(&err)
        .or_else(|| last_line(&out))
        .unwrap_or_else(|| format!("{program} produced no output"));
    Err(ToolError::Failed { detail: format!("{program} exited {code}: {detail}") })
}

/// Wait for the child to exit once its output pipes have closed, with
/// a short grace: the tools close their pipes as they finish, so a
/// child that will not end past the grace is killed and named rather
/// than hanging the install.
fn wait_for_exit(
    child: &mut std::process::Child,
    program: &str,
) -> Result<ExitStatus, ToolError> {
    let deadline = Instant::now() + TOOL_EXIT_GRACE;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Ok(status),
            Ok(None) if Instant::now() < deadline => {
                thread::sleep(Duration::from_millis(25));
            }
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(ToolError::Failed {
                    detail: format!("{program} did not exit after its output closed"),
                });
            }
            Err(error) => {
                // A child that cannot be waited on is a named failure,
                // not a hang: its pipes are already closed.
                return Err(ToolError::Failed {
                    detail: format!("{program} could not be waited on: {error}"),
                });
            }
        }
    }
}

/// The last non-empty line of a tool's output, trimmed.
fn last_line(text: &str) -> Option<String> {
    text.lines()
        .rev()
        .find(|line| !line.trim().is_empty())
        .map(|line| line.trim().to_string())
}

/// Which tool built a venv, so the package install uses the same one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VenvTool {
    /// `uv venv` — the fork's tool, the fast path.
    Uv,
    /// `python3 -m venv` — the fallback when uv is absent.
    Python,
}

/// The Python version `uv` is asked for: the fork's `3.12`.
const UV_PYTHON: &str = "3.12";

/// Create the venv: `uv` first (the fork's vocabulary), `python3 -m venv`
/// when uv does not resolve on PATH. A tool that runs and fails — or
/// that runs past the bound — is a named error, not a silent second
/// attempt: the failure is the lane's, and it is named.
fn create_venv(venv: &Path, run: &mut LaneRun<'_>) -> Result<VenvTool, ToolError> {
    let venv_arg = venv.to_string_lossy().into_owned();
    match run_tool("uv", &["venv", &venv_arg, "--python", UV_PYTHON], run) {
        Ok(()) => Ok(VenvTool::Uv),
        // Only an absent uv falls back: a uv that ran and failed, or
        // one that was killed at the bound, is the lane's own answer.
        Err(ToolError::NotFound { .. }) => {
            run_tool("python3", &["-m", "venv", &venv_arg], run).map(|()| VenvTool::Python)
        }
        Err(failure) => Err(failure),
    }
}

/// Install the lane's packages into the venv with the tool that built
/// it: `uv pip install --python <venv python>` or the venv's own
/// `<venv python> -m pip install`.
fn install_packages(
    tool: VenvTool,
    venv: &Path,
    packages: &[&str],
    run: &mut LaneRun<'_>,
) -> Result<(), ToolError> {
    let python = venv_python(venv).to_string_lossy().into_owned();
    let (program, mut args): (&str, Vec<&str>) = match tool {
        VenvTool::Uv => ("uv", vec!["pip", "install", "--python", python.as_str()]),
        VenvTool::Python => (python.as_str(), vec!["-m", "pip", "install"]),
    };
    args.extend_from_slice(packages);
    run_tool(program, &args, run)
}

/// Bootstrap one lane under `home`: idempotent, and never a failure of
/// the install.
///
/// A finished venv — its python **and** its receipt — is detected and
/// named, not rebuilt. A venv that cannot be built — no tool on PATH,
/// a failing create, a failing install, a tool killed at the bound —
/// leaves the lane [`LaneOutcome::Skipped`] with its reason named; the
/// lane stays declared and dead, which is slice 4's dead-lane
/// behavior, and the install carries on. `path` is the child's PATH
/// when given — the seam a test drives a deterministic outcome
/// through — and inherited otherwise. The child's lines stream to
/// `progress` as they arrive, so a multi-minute install cannot look
/// frozen.
pub fn bootstrap_lane(
    home: &Path,
    lane: &Lane,
    path: Option<&OsStr>,
    progress: &mut (dyn std::io::Write + Send),
) -> LaneOutcome {
    bootstrap_lane_bounded(home, lane, path, progress, LANE_INSTALL_TIMEOUT)
}

/// Bootstrap one lane under an explicit subprocess bound — the seam a
/// test drives a hung tool through, at 300ms rather than
/// [`LANE_INSTALL_TIMEOUT_SECS`]. [`bootstrap_lane`] is this with the
/// production bound.
fn bootstrap_lane_bounded(
    home: &Path,
    lane: &Lane,
    path: Option<&OsStr>,
    progress: &mut (dyn std::io::Write + Send),
    bound: Duration,
) -> LaneOutcome {
    let venv = venv_dir(home, lane);
    // The presence check's whole test: the venv's python exists AND
    // the installer's receipt names the lane. A venv with one and not
    // the other is a half-built venv — an interrupted prior run, a
    // hand-made directory, a pre-receipt build — and is rebuilt or
    // skipped below, never misread as Present.
    if venv_is_finished(&venv, lane) {
        return LaneOutcome::Present;
    }
    // Whether this run is the venv's author. The removal below takes
    // only the artifact this run created: a directory that was already
    // there — an interrupted prior run's half-built venv, a user's own
    // junk — is retried over or skipped, never removed. Without the
    // receipt it can never read as Present either way.
    let created_here = !venv.exists();
    let mut run = LaneRun { path, lane: lane.name, progress, bound };
    let tool = match create_venv(&venv, &mut run) {
        Ok(tool) => tool,
        Err(failure) => {
            if created_here {
                let _ = fs::remove_dir_all(&venv);
            }
            return LaneOutcome::Skipped { reason: failure.describe() };
        }
    };
    if let Err(failure) = install_packages(tool, &venv, lane.packages, &mut run) {
        // The venv this run created is incomplete. Remove it
        // best-effort so the lane's state stays honestly absent and a
        // later run retries, rather than reading "present" over a venv
        // whose packages never landed. Only this run's own artifact is
        // removed — a directory that was already there is left for the
        // receipt guard to keep reading not-built.
        if created_here {
            let _ = fs::remove_dir_all(&venv);
        }
        let reason = match &failure {
            // The bound expired and the child was killed: name the
            // bound and the kill, not the step that was mid-flight.
            ToolError::TimedOut { .. } => failure.describe(),
            _ => format!(
                "the venv at {} was created but its packages did not install: {}",
                venv.display(),
                failure.describe()
            ),
        };
        return LaneOutcome::Skipped { reason };
    }
    // The packages landed: the receipt is what makes the venv read as
    // finished on every later run. A receipt that cannot be written is
    // a failure of the lane, named — never a silent Present-over-nothing.
    if let Err(error) = write_receipt(&venv, lane, &utc_minute()) {
        return LaneOutcome::Skipped {
            reason: format!(
                "the packages installed but the receipt at {} could not be written: {error}",
                receipt_path(&venv).display()
            ),
        };
    }
    LaneOutcome::Installed
}

// --- the report --------------------------------------------------------------

/// Render the install's report: [`render_head`] then [`render_tail`],
/// the one call a test — or any caller that is not streaming — makes.
/// `lanes` is `Some` when `--lanes` ran the bootstrap and `None` when
/// the bare install named the lane step as available.
pub fn render(
    home: &Path,
    layout: &[(&'static str, LayoutStep)],
    config: (ConfigStep, &Path),
    lanes: Option<&[LaneResult]>,
) -> String {
    let mut out = render_head(home, layout, config, lanes.is_some());
    out.push_str(&render_tail(home, lanes));
    out
}

/// The report's opening: the header, the home, one line per layout
/// step, the config step, and — when `--lanes` is running — the venvs
/// header the lanes' streamed output lands under.
///
/// It is its own function so the command can print it *before* the
/// lane bootstrap: the operator sees the install begin, and each
/// tool's own lines arrive under the venvs header as they come,
/// rather than the whole report waiting behind a multi-minute
/// `uv pip install`.
pub fn render_head(
    home: &Path,
    layout: &[(&'static str, LayoutStep)],
    config: (ConfigStep, &Path),
    lanes: bool,
) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "{BOLD}{ACCENT}fleet install{RESET} {DIM}— the home layout, the config scaffold, and the doctor gate{RESET}\n\n"
    ));
    out.push_str(&format!(
        "  {TEXT}{:<12}{RESET} {DIM}{}{RESET}\n",
        "home",
        home.display()
    ));
    for (name, step) in layout {
        let (glyph, color, verb) = match step {
            LayoutStep::Created => ("+", OK, "created"),
            LayoutStep::Present => ("=", DIM, "present"),
        };
        out.push_str(&format!(
            "  {color}{glyph}{RESET} {TEXT}{name:<12}{RESET} {color}{verb}{RESET} {DIM}{}{RESET}\n",
            home.join(name).display()
        ));
    }
    let (config_step, config_path) = config;
    let (glyph, color, verb, detail) = match config_step {
        ConfigStep::Written => (
            "+",
            OK,
            "written",
            format!(
                "{} — the roster's override seam: {{\"pin\": null, \"providers\": []}}",
                config_path.display()
            ),
        ),
        ConfigStep::Present => (
            "=",
            DIM,
            "present",
            format!("{} — left untouched", config_path.display()),
        ),
        ConfigStep::Symlink => (
            "!",
            TEXT,
            "symlink",
            format!(
                "{} — left exactly as it is, never written through",
                config_path.display()
            ),
        ),
    };
    out.push_str(&format!(
        "  {color}{glyph}{RESET} {TEXT}{:<12}{RESET} {color}{verb}{RESET} {DIM}{detail}{RESET}\n",
        CONFIG_FILE
    ));
    if lanes {
        out.push_str(&format!(
            "  {DIM}{:<12}{RESET} {DIM}{} — one venv per core, idempotent, degrade-without{RESET}\n",
            "venvs",
            venvs_dir(home).display()
        ));
    }
    out
}

/// The report's close: the lane section — one line per lane and the
/// summary that names the skips — or the bare install's note that the
/// lane step is available, then the credential note and the
/// doctor-gate separator. Printed once the lanes are done, so every
/// line it names is a finished answer.
pub fn render_tail(home: &Path, lanes: Option<&[LaneResult]>) -> String {
    let mut out = String::new();
    match lanes {
        Some(results) => {
            for (lane, outcome) in results {
                out.push_str(&render_lane_line(lane, outcome, &venv_dir(home, lane)));
            }
            out.push_str(&render_lane_summary(results));
        }
        None => out.push_str(&format!(
            "  {DIM}!{RESET} {TEXT}{:<12}{RESET} {DIM}optional heavy lanes — `fleet install --lanes` builds {}/<lane>/; a lane that cannot build is skipped by name{RESET}\n",
            "lanes",
            venvs_dir(home).display()
        )),
    }
    out.push_str(&render_credentials(home));
    out.push_str(&format!(
        "\n{DIM}── doctor ── the install's exit code is the doctor's ──{RESET}\n"
    ));
    out
}

/// The lane section's summary line: how many lanes are ready and how
/// many were skipped, so a degraded `--lanes` run is visible in the
/// report itself and not only in the skipped lane's own line. It is a
/// report line, not a gate: a skipped lane still leaves the install
/// green, because the lane is declared and dead, not broken.
fn render_lane_summary(results: &[LaneResult]) -> String {
    let total = results.len();
    let installed = results
        .iter()
        .filter(|(_, outcome)| matches!(outcome, LaneOutcome::Installed))
        .count();
    let present = results
        .iter()
        .filter(|(_, outcome)| matches!(outcome, LaneOutcome::Present))
        .count();
    let skipped = total - installed - present;
    let (glyph, color) = if skipped == 0 { ("=", DIM) } else { ("!", TEXT) };
    format!(
        "  {color}{glyph}{RESET} {TEXT}{:<12}{RESET} {color}{installed} of {total} lanes installed, {present} present, {skipped} skipped{RESET}\n",
        "lanes"
    )
}

/// One lane's line under the venvs header: the outcome, the venv's
/// path, and — for a built or already-present venv — the wiring the
/// core reads.
fn render_lane_line(lane: &Lane, outcome: &LaneOutcome, venv: &Path) -> String {
    let (glyph, color, verb, detail) = match outcome {
        LaneOutcome::Present => (
            "=",
            DIM,
            "present",
            format!("{} — {}", venv.display(), lane_wiring(lane, venv)),
        ),
        LaneOutcome::Installed => (
            "+",
            OK,
            "installed",
            format!("{} — {}", venv.display(), lane_wiring(lane, venv)),
        ),
        LaneOutcome::Skipped { reason } => {
            ("!", TEXT, "skipped", format!("{} — {}", venv.display(), reason))
        }
    };
    format!(
        "  {color}{glyph}{RESET} {TEXT}{:<12}{RESET} {color}{verb}{RESET} {DIM}{detail}{RESET}\n",
        lane.name
    )
}

/// The credential chain note: the environment name, the `<home>/.env`
/// fallback, and the Harness credential store's `refs` block, named
/// with its resolved path. Names only — no value is ever read or
/// printed here, and the doctor's credentials check below is the place
/// that reports whether one resolved.
fn render_credentials(home: &Path) -> String {
    let harness = crate::home::harness_credentials_path(
        std::env::var("DSH_HOME").ok().as_deref(),
        std::env::var("HOME").ok().as_deref(),
    );
    let named = harness
        .as_deref()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "~/.dsh/.credentials.yaml".to_string());
    format!(
        "  {DIM}!{RESET} {TEXT}{:<12}{RESET} {DIM}{CREDENTIAL_ENV}: process env → {}/.env → the harness refs ({named}, read-only){RESET}\n",
        "credentials",
        home.display()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A scratch directory, unique per call.
    fn scratch(tag: &str) -> PathBuf {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let serial = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "fleet-install-{tag}-{}-{serial}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// Strip the palette's SGR sequences, so an assertion names a whole
    /// line instead of a fragment that happens not to straddle a color
    /// boundary (the eval gate's own stripping rule).
    fn strip_ansi(text: &str) -> String {
        let mut out = String::with_capacity(text.len());
        let mut chars = text.chars();
        while let Some(ch) = chars.next() {
            if ch == '\u{1b}' {
                for next in chars.by_ref() {
                    if next.is_ascii_alphabetic() {
                        break;
                    }
                }
            } else {
                out.push(ch);
            }
        }
        out
    }

    /// A controlled PATH holding a fake `uv`: it materializes the venv
    /// layout `uv venv` is asked for, then answers the package step
    /// the way `hang` says. Not hanging, it prints one line on each
    /// pipe and exits zero — the streaming driver. Hanging, it prints
    /// and `exec`s a sleep, so the process the installer holds is the
    /// one that never finishes — the hung-tool driver.
    /// `#[cfg(unix)]` because the stand-in is a shell script, and
    /// `/bin/mkdir` is absolute because a bare `mkdir` would not
    /// resolve on the controlled PATH.
    #[cfg(unix)]
    fn fake_uv(dir: &Path, hang: bool) {
        use std::os::unix::fs::PermissionsExt;

        fs::create_dir_all(dir).unwrap();
        let package_step = if hang {
            "echo 'fake uv: wedged' >&2; exec /bin/sleep 30"
        } else {
            "echo 'fake uv: resolving one package'; echo 'fake uv: installed one package' >&2"
        };
        let script = format!(
            "#!/bin/sh\n\
             if [ \"$1\" = \"venv\" ]; then /bin/mkdir -p \"$2/bin\" && : > \"$2/bin/python\"; exit 0; fi\n\
             if [ \"$1\" = \"pip\" ]; then {package_step}; fi\n\
             exit 0\n"
        );
        let uv = dir.join("uv");
        fs::write(&uv, script).unwrap();
        fs::set_permissions(&uv, fs::Permissions::from_mode(0o755)).unwrap();
    }

    #[test]
    fn the_layout_is_the_five_directories() {
        assert_eq!(
            LAYOUT_DIRS,
            &["sessions", "memories", "skills", "cron", "proposals"]
        );
    }

    #[test]
    fn the_scaffold_is_the_minimal_override_document() {
        // The exact bytes a fresh home carries.
        assert_eq!(CONFIG_SCAFFOLD, "{\n  \"pin\": null,\n  \"providers\": []\n}\n");
        // And they parse as the roster's override seam: no pin, no
        // overrides — the compiled-in defaults stand.
        let file: serde_json::Value = serde_json::from_str(CONFIG_SCAFFOLD).unwrap();
        assert_eq!(file["pin"], serde_json::Value::Null);
        assert_eq!(file["providers"], serde_json::json!([]));
    }

    #[test]
    fn the_scaffold_loads_as_the_default_roster() {
        // The scaffold beside the real loader: the roster it produces
        // is the compiled-in one, so a fresh home routes identically
        // to a home with no config file at all.
        let home = scratch("scaffold-roster");
        fs::write(home.join(CONFIG_FILE), CONFIG_SCAFFOLD).unwrap();
        let roster = crate::route::load_roster(&home).unwrap();
        assert_eq!(roster.pin, None);
        assert!(
            roster.candidates.len() >= 6,
            "the compiled-in roster stands beside the scaffold"
        );
        let _ = fs::remove_dir_all(&home);
    }

    #[test]
    fn layout_steps_are_idempotent_and_name_themselves() {
        let home = scratch("layout");
        assert_eq!(layout_step(&home, "sessions").unwrap(), LayoutStep::Created);
        assert!(home.join("sessions").is_dir());
        // A second run changes nothing and names what exists.
        assert_eq!(layout_step(&home, "sessions").unwrap(), LayoutStep::Present);
        let _ = fs::remove_dir_all(&home);
    }

    #[test]
    fn the_config_scaffold_is_written_once_and_never_rewritten() {
        let home = scratch("config");
        let (step, path) = config_step(&home).unwrap();
        assert_eq!(step, ConfigStep::Written);
        assert_eq!(fs::read_to_string(&path).unwrap(), CONFIG_SCAFFOLD);
        // An existing file — whatever it holds — is named and left
        // byte-for-byte untouched, even when it is not valid JSON.
        fs::write(&path, "{ not json ").unwrap();
        let (again, _) = config_step(&home).unwrap();
        assert_eq!(again, ConfigStep::Present);
        assert_eq!(fs::read_to_string(&path).unwrap(), "{ not json ");
        let _ = fs::remove_dir_all(&home);
    }

    #[test]
    fn the_lane_table_is_the_five_vendored_cores() {
        let names: Vec<&str> = LANES.iter().map(|lane| lane.name).collect();
        assert_eq!(names, vec!["sys1", "sdm1", "sem1", "gen1", "blend1"]);
        // The fork's vocabulary, ported: each lane names the packages
        // its venv carries and the environment name its core reads.
        let sem1 = LANES.iter().find(|lane| lane.name == "sem1").unwrap();
        assert_eq!(sem1.packages, &["sentence-transformers"]);
        assert_eq!(sem1.env_var, Some("SEM1_ST_VENV"));
        assert_eq!(sem1.env_target, EnvTarget::Root);
        let gen1 = LANES.iter().find(|lane| lane.name == "gen1").unwrap();
        // GEN1_KOKORO_PYTHON addresses the binary, not the root.
        assert_eq!(gen1.env_var, Some("GEN1_KOKORO_PYTHON"));
        assert_eq!(gen1.env_target, EnvTarget::Python);
        // sys1 reads a config-file key, so it carries no environment
        // name and names the config step instead.
        let sys1 = LANES.iter().find(|lane| lane.name == "sys1").unwrap();
        // The one fix at the vendoring boundary: gliner2's local-model
        // stack (torch and the rest) is the `local` extra, and the
        // classifier imports torch at module load.
        assert_eq!(sys1.packages, &["gliner2[local]"]);
        assert_eq!(sys1.env_var, None);
        assert!(sys1.note.contains("local_venv"));
        // blend1's own core is stdlib-only: its venv carries the union
        // of the three siblings' package sets, and the cores themselves
        // are path-wired through the sibling `<SVC>_PYTHONPATH` names
        // rather than pip-installed here.
        let blend1 = LANES.iter().find(|lane| lane.name == "blend1").unwrap();
        assert_eq!(blend1.role, "grounded");
        assert_eq!(
            blend1.packages,
            &["gliner2[local]", "structured-data-models", "sentence-transformers"]
        );
        assert_eq!(blend1.env_var, Some("BLEND1_PYTHONPATH"));
        assert_eq!(blend1.env_target, EnvTarget::Root);
        assert!(blend1.note.contains("BLEND1_SYS1_PYTHONPATH"), "{}", blend1.note);
        assert!(blend1.note.contains("services/<svc>/src"), "{}", blend1.note);
    }

    #[test]
    fn the_venvs_live_under_the_home_one_directory_per_lane() {
        let home = Path::new("/home/x/.fleet");
        assert_eq!(venvs_dir(home), PathBuf::from("/home/x/.fleet/venvs"));
        let sem1 = LANES.iter().find(|lane| lane.name == "sem1").unwrap();
        assert_eq!(
            venv_dir(home, sem1),
            PathBuf::from("/home/x/.fleet/venvs/sem1")
        );
        assert_eq!(
            venv_python(&venv_dir(home, sem1)),
            PathBuf::from("/home/x/.fleet/venvs/sem1/bin/python")
        );
    }

    #[test]
    fn lane_wiring_names_the_export_or_the_config_key() {
        let venv = Path::new("/home/x/.fleet/venvs/sem1");
        let sem1 = LANES.iter().find(|lane| lane.name == "sem1").unwrap();
        assert_eq!(
            lane_wiring(sem1, venv),
            "export SEM1_ST_VENV=/home/x/.fleet/venvs/sem1"
        );
        let gen1 = LANES.iter().find(|lane| lane.name == "gen1").unwrap();
        // The binary-addressed lane exports the python itself.
        assert_eq!(
            lane_wiring(gen1, venv),
            "export GEN1_KOKORO_PYTHON=/home/x/.fleet/venvs/sem1/bin/python; the ONNX weights (~/.cache/gen1/tts/kokoro-v1.0.onnx and voices-v1.0.bin) are machine-owner downloads, not installed here"
        );
        let sys1 = LANES.iter().find(|lane| lane.name == "sys1").unwrap();
        // sys1's config-file key carries the real venv path.
        assert_eq!(
            lane_wiring(sys1, venv),
            "point ~/.config/sys1/config.toml at it: [providers] local_venv = \"/home/x/.fleet/venvs/sem1\""
        );
        let blend1 = LANES.iter().find(|lane| lane.name == "blend1").unwrap();
        // blend1's venv export rides beside its sibling path-wiring
        // note: the cores resolve through the <SVC>_PYTHONPATH names,
        // not through a pip install.
        let blend1_wiring = lane_wiring(blend1, venv);
        assert!(
            blend1_wiring.starts_with("export BLEND1_PYTHONPATH=/home/x/.fleet/venvs/sem1; "),
            "{blend1_wiring}"
        );
        assert!(blend1_wiring.contains("BLEND1_SEM1_PYTHONPATH"), "{blend1_wiring}");
        assert!(blend1_wiring.contains("services/<svc>/src"), "{blend1_wiring}");
    }

    #[test]
    fn a_present_venv_is_detected_without_rebuilding() {
        // The presence check is the venv python's existence together
        // with the installer's receipt; the bootstrap answers Present
        // before any tool runs.
        let home = scratch("present");
        let sem1 = LANES.iter().find(|lane| lane.name == "sem1").unwrap();
        let venv = venv_dir(&home, sem1);
        fs::create_dir_all(venv.join("bin")).unwrap();
        fs::write(venv.join("bin").join("python"), "#!/bin/sh\n").unwrap();
        write_receipt(&venv, sem1, "2026-10-14T18:04").unwrap();
        assert_eq!(bootstrap_lane(&home, sem1, None, &mut Vec::new()), LaneOutcome::Present);
        let _ = fs::remove_dir_all(&home);
    }

    #[test]
    fn the_receipt_names_the_lane_its_packages_and_the_utc_minute() {
        let home = scratch("receipt");
        let sem1 = LANES.iter().find(|lane| lane.name == "sem1").unwrap();
        let venv = venv_dir(&home, sem1);
        fs::create_dir_all(venv.join("bin")).unwrap();
        fs::write(venv.join("bin").join("python"), "#!/bin/sh\n").unwrap();
        // The exact bytes: the lane's name and role, its package list,
        // and the UTC minute it landed.
        write_receipt(&venv, sem1, "2026-10-14T18:04").unwrap();
        assert_eq!(
            fs::read_to_string(receipt_path(&venv)).unwrap(),
            "lane sem1\nrole embeddings\npackages sentence-transformers\ninstalled 2026-10-14T18:04\n"
        );
        // And the presence check reads it back.
        assert!(venv_is_finished(&venv, sem1));
        // A receipt that does not name this lane does not vouch for
        // this venv: the python alone is not a finished lane, and
        // neither is a receipt belonging to a different lane or one
        // that names no lane at all.
        fs::write(receipt_path(&venv), "lane sdm1\n").unwrap();
        assert!(!venv_is_finished(&venv, sem1));
        fs::write(receipt_path(&venv), "packages sentence-transformers\n").unwrap();
        assert!(!venv_is_finished(&venv, sem1));
        let _ = fs::remove_dir_all(&home);
    }

    #[test]
    fn a_partial_venv_without_the_receipt_is_not_present() {
        // An interrupted prior run: `uv venv` has created bin/python
        // and the packages never landed. The presence check must not
        // read it as Present, or every later run would name a dead
        // venv as installed. The PATH carries no venv tool, so the
        // lane is skipped — never Present, and never rebuilt silently
        // by the machine's own uv.
        let home = scratch("partial");
        let sem1 = LANES.iter().find(|lane| lane.name == "sem1").unwrap();
        let venv = venv_dir(&home, sem1);
        fs::create_dir_all(venv.join("bin")).unwrap();
        fs::write(venv.join("bin").join("python"), "#!/bin/sh\n").unwrap();
        let bare = scratch("bare-path");
        let outcome = bootstrap_lane(&home, sem1, Some(bare.as_os_str()), &mut Vec::new());
        assert!(
            matches!(outcome, LaneOutcome::Skipped { .. }),
            "a venv without the receipt is not-built: {outcome:?}"
        );
        assert!(!venv_is_finished(&venv, sem1));
        let _ = fs::remove_dir_all(&home);
        let _ = fs::remove_dir_all(&bare);
    }

    #[cfg(unix)]
    #[test]
    fn the_progress_sink_receives_the_tools_lines_as_they_arrive() {
        // The streaming half of the bound: a tool that prints is seen
        // by the operator while it runs, not after every lane has
        // finished. The sink is a Vec here — the shape a test drives.
        let home = scratch("stream");
        let sem1 = LANES.iter().find(|lane| lane.name == "sem1").unwrap();
        let tools = scratch("stream-tools");
        fake_uv(&tools, false);
        let mut sink: Vec<u8> = Vec::new();
        let outcome = bootstrap_lane(&home, sem1, Some(tools.as_os_str()), &mut sink);
        assert_eq!(outcome, LaneOutcome::Installed);
        let streamed = String::from_utf8(sink).unwrap();
        // Both pipes' lines arrive, each named under its lane.
        assert!(
            streamed.contains("sem1 | fake uv: resolving one package"),
            "{streamed}"
        );
        assert!(
            streamed.contains("sem1 | fake uv: installed one package"),
            "{streamed}"
        );
        let _ = fs::remove_dir_all(&home);
        let _ = fs::remove_dir_all(&tools);
    }

    #[cfg(unix)]
    #[test]
    fn a_hung_tool_is_killed_at_the_bound_and_the_lane_is_skipped() {
        // A tool that never finishes: killed at the bound, the
        // half-built venv removed, and the lane skipped naming the
        // bound. The bound here is 300ms rather than
        // LANE_INSTALL_TIMEOUT_SECS, which is the seam this test
        // drives — production's 900s is not a test's wait.
        let home = scratch("hung");
        let sem1 = LANES.iter().find(|lane| lane.name == "sem1").unwrap();
        let tools = scratch("hung-tools");
        fake_uv(&tools, true);
        let venv = venv_dir(&home, sem1);
        let mut sink: Vec<u8> = Vec::new();
        let started = Instant::now();
        let outcome = bootstrap_lane_bounded(
            &home,
            sem1,
            Some(tools.as_os_str()),
            &mut sink,
            Duration::from_millis(300),
        );
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "the bound bounds the wait: {:?}",
            started.elapsed()
        );
        let LaneOutcome::Skipped { reason } = outcome else {
            panic!("a hung tool is a skipped lane: {outcome:?}");
        };
        assert!(reason.contains("did not finish within 300ms"), "{reason}");
        assert!(reason.contains("was killed"), "{reason}");
        // The half-built venv is gone, so a later run retries rather
        // than reading present over a venv whose packages never landed.
        assert!(!venv.exists(), "the half-built venv was removed");
        let _ = fs::remove_dir_all(&home);
        let _ = fs::remove_dir_all(&tools);
    }

    #[test]
    fn the_config_scaffold_is_never_written_through_a_dangling_symlink() {
        // `exists()` follows the link, so a dangling one reads absent —
        // and `fs::write` would write the scaffold *through* it into
        // whatever the link points at. The step names the symlink and
        // leaves it exactly as it is.
        let home = scratch("dangling");
        fs::create_dir_all(&home).unwrap();
        let target = home.join("elsewhere").join("config.json");
        std::os::unix::fs::symlink(&target, home.join(CONFIG_FILE)).unwrap();
        let (step, path) = config_step(&home).unwrap();
        assert_eq!(step, ConfigStep::Symlink);
        assert_eq!(path, home.join(CONFIG_FILE));
        assert!(!target.exists(), "nothing was written through the link");
        assert!(home.join(CONFIG_FILE).read_link().is_ok(), "the link stands");
        let _ = fs::remove_dir_all(&home);
    }

    #[test]
    fn the_report_names_the_skips_in_its_summary_line() {
        // The degradation is visible in the report itself, not only in
        // the skipped lane's own line — and it is a report line, not a
        // gate: a skipped lane leaves the install green.
        let home = scratch("summary");
        let layout = vec![("sessions", LayoutStep::Created)];
        let sys1 = LANES.iter().find(|lane| lane.name == "sys1").unwrap();
        let sem1 = LANES.iter().find(|lane| lane.name == "sem1").unwrap();
        let gen1 = LANES.iter().find(|lane| lane.name == "gen1").unwrap();
        let results = vec![
            (sys1, LaneOutcome::Installed),
            (sem1, LaneOutcome::Present),
            (gen1, LaneOutcome::Skipped { reason: "no network".to_string() }),
        ];
        let report = strip_ansi(&render(
            &home,
            &layout,
            (ConfigStep::Written, &home.join(CONFIG_FILE)),
            Some(&results),
        ));
        assert!(
            report.contains("! lanes        1 of 3 lanes installed, 1 present, 1 skipped"),
            "{report}"
        );
        let _ = fs::remove_dir_all(&home);
    }

    #[test]
    fn the_report_names_every_step_and_the_gate() {
        let home = scratch("render");
        let layout = vec![
            ("sessions", LayoutStep::Created),
            ("memories", LayoutStep::Present),
        ];
        let report = strip_ansi(&render(
            &home,
            &layout,
            (ConfigStep::Written, &home.join(CONFIG_FILE)),
            None,
        ));
        assert!(report.contains("fleet install"), "{report}");
        // Each step's outcome is named, the bare install names the
        // lane step as available, and the doctor gate is announced.
        assert!(report.contains("+ sessions"), "{report}");
        assert!(report.contains("created"), "{report}");
        assert!(report.contains("= memories"), "{report}");
        assert!(report.contains("present"), "{report}");
        assert!(report.contains("+ config.json"), "{report}");
        assert!(report.contains("the roster's override seam"), "{report}");
        assert!(report.contains("! lanes"), "{report}");
        assert!(report.contains("fleet install --lanes"), "{report}");
        assert!(report.contains("! credentials"), "{report}");
        assert!(report.contains("DEEPSEEK_API_KEY"), "{report}");
        assert!(report.contains("── doctor ──"), "{report}");
        let _ = fs::remove_dir_all(&home);
    }
}
