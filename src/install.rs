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

use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

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
}

/// Write the config scaffold when absent; name and never rewrite an
/// existing file, whatever it holds — an owner's hand-edited config is
/// theirs, and the install is not the place it gets clobbered.
/// @return the step's outcome and the config file's path.
pub fn config_step(home: &Path) -> std::io::Result<(ConfigStep, PathBuf)> {
    let path = home.join(CONFIG_FILE);
    if path.exists() {
        return Ok((ConfigStep::Present, path));
    }
    fs::write(&path, CONFIG_SCAFFOLD)?;
    Ok((ConfigStep::Written, path))
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
/// four vendored cores; the venvs live at `<home>/venvs/<name>/`.
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

/// The four lanes, one per vendored core, in the fork's order.
pub const LANES: &[Lane] = &[
    Lane {
        name: "sys1",
        role: "judgment",
        packages: &["gliner2"],
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

/// The venv's python — the presence check's whole test.
fn venv_python(venv: &Path) -> PathBuf {
    venv.join("bin").join("python")
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
}

impl ToolError {
    /// The named line the install prints.
    fn describe(&self) -> String {
        match self {
            ToolError::NotFound { program } => format!("{program} is not on PATH"),
            ToolError::Failed { detail } => detail.clone(),
        }
    }
}

/// Run one tool, with `path` as the child's PATH when given — the seam
/// a test drives a deterministic failure through — and the caller's
/// environment otherwise. `Ok(())` on a zero exit; otherwise the named
/// reason.
fn run_tool(program: &str, args: &[&str], path: Option<&OsStr>) -> Result<(), ToolError> {
    let mut command = Command::new(program);
    command.args(args);
    if let Some(path) = path {
        command.env("PATH", path);
    }
    let output = match command.output() {
        Ok(output) => output,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err(ToolError::NotFound { program: program.to_string() });
        }
        Err(error) => {
            return Err(ToolError::Failed { detail: format!("{program}: {error}") });
        }
    };
    if output.status.success() {
        return Ok(());
    }
    let code = output.status.code().unwrap_or(-1);
    let detail = last_line(&output.stderr)
        .or_else(|| last_line(&output.stdout))
        .unwrap_or_else(|| format!("{program} produced no output"));
    Err(ToolError::Failed { detail: format!("{program} exited {code}: {detail}") })
}

/// The last non-empty line of a tool's output, trimmed.
fn last_line(bytes: &[u8]) -> Option<String> {
    String::from_utf8_lossy(bytes)
        .lines()
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
/// when uv does not resolve on PATH. A tool that runs and fails is a
/// named error, not a silent second attempt — the failure is the
/// lane's, and it is named.
fn create_venv(venv: &Path, path: Option<&OsStr>) -> Result<VenvTool, ToolError> {
    let venv_arg = venv.to_string_lossy().into_owned();
    match run_tool("uv", &["venv", &venv_arg, "--python", UV_PYTHON], path) {
        Ok(()) => Ok(VenvTool::Uv),
        Err(ToolError::NotFound { .. }) => {
            run_tool("python3", &["-m", "venv", &venv_arg], path).map(|()| VenvTool::Python)
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
    path: Option<&OsStr>,
) -> Result<(), ToolError> {
    let python = venv_python(venv).to_string_lossy().into_owned();
    let (program, mut args): (&str, Vec<&str>) = match tool {
        VenvTool::Uv => ("uv", vec!["pip", "install", "--python", python.as_str()]),
        VenvTool::Python => (python.as_str(), vec!["-m", "pip", "install"]),
    };
    args.extend_from_slice(packages);
    run_tool(program, &args, path)
}

/// Bootstrap one lane under `home`: idempotent, and never a failure of
/// the install. `path` is the child's PATH when given — the seam a
/// test drives a deterministic failure through — and inherited
/// otherwise.
///
/// An existing venv is detected and named, not rebuilt. A venv that
/// cannot be built — no tool on PATH, a failing create, a failing
/// install — leaves the lane [`LaneOutcome::Skipped`] with its reason
/// named; the lane stays declared and dead, which is slice 4's
/// dead-lane behavior, and the install carries on.
pub fn bootstrap_lane(home: &Path, lane: &Lane, path: Option<&OsStr>) -> LaneOutcome {
    let venv = venv_dir(home, lane);
    // The presence check's whole test: the venv's python exists.
    if venv_python(&venv).exists() {
        return LaneOutcome::Present;
    }
    let tool = match create_venv(&venv, path) {
        Ok(tool) => tool,
        Err(failure) => return LaneOutcome::Skipped { reason: failure.describe() },
    };
    if let Err(failure) = install_packages(tool, &venv, lane.packages, path) {
        // The venv this run just created is incomplete. Remove it
        // best-effort so the lane's state stays honestly absent and a
        // later run retries, rather than reading "present" over a venv
        // whose packages never landed. Only this run's own artifact is
        // removed — an existing venv is never touched.
        let _ = fs::remove_dir_all(&venv);
        return LaneOutcome::Skipped {
            reason: format!(
                "the venv at {} was created but its packages did not install: {}",
                venv.display(),
                failure.describe()
            ),
        };
    }
    LaneOutcome::Installed
}

// --- the report --------------------------------------------------------------

/// Render the install's report: the header, the home, one line per
/// layout step, the config step, the lane step, the credential note,
/// and the doctor-gate separator — the fork's `+`/`=`/`!` vocabulary
/// in the house card-line style. `lanes` is `Some` when `--lanes` ran
/// the bootstrap and `None` when the bare install named the lane step
/// as available.
pub fn render(
    home: &Path,
    layout: &[(&'static str, LayoutStep)],
    config: (ConfigStep, &Path),
    lanes: Option<&[LaneResult]>,
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
    };
    out.push_str(&format!(
        "  {color}{glyph}{RESET} {TEXT}{:<12}{RESET} {color}{verb}{RESET} {DIM}{detail}{RESET}\n",
        CONFIG_FILE
    ));
    match lanes {
        Some(results) => {
            out.push_str(&format!(
                "  {DIM}{:<12}{RESET} {DIM}{} — one venv per core, idempotent, degrade-without{RESET}\n",
                "venvs",
                venvs_dir(home).display()
            ));
            for (lane, outcome) in results {
                out.push_str(&render_lane_line(lane, outcome, &venv_dir(home, lane)));
            }
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
    fn the_lane_table_is_the_four_vendored_cores() {
        let names: Vec<&str> = LANES.iter().map(|lane| lane.name).collect();
        assert_eq!(names, vec!["sys1", "sdm1", "sem1", "gen1"]);
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
        assert_eq!(sys1.packages, &["gliner2"]);
        assert_eq!(sys1.env_var, None);
        assert!(sys1.note.contains("local_venv"));
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
    }

    #[test]
    fn a_present_venv_is_detected_without_rebuilding() {
        // The presence check is the venv python's existence; the
        // bootstrap answers Present before any tool runs.
        let home = scratch("present");
        let sem1 = LANES.iter().find(|lane| lane.name == "sem1").unwrap();
        let venv = venv_dir(&home, sem1);
        fs::create_dir_all(venv.join("bin")).unwrap();
        fs::write(venv.join("bin").join("python"), "#!/bin/sh\n").unwrap();
        assert_eq!(bootstrap_lane(&home, sem1, None), LaneOutcome::Present);
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
