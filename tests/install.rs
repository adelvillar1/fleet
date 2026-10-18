//! The install's acceptance, driven through the real binary: the
//! idempotent layout and the exact scaffold, the per-lane bootstrap's
//! three behaviors (present, installed, skipped-without), the venv
//! receipt that keeps an interrupted run's half-built venv from
//! reading present, the doctor gate's exit-code contract, and the
//! README quick start's sequence against a scratch home.
//!
//! No network beyond loopback: the doctor's API-reachability probe is
//! pointed at a local TCP listener the test holds open for the run's
//! life, and the credential is a placeholder the doctor only checks
//! for presence. No chat-completions request is ever made here — the
//! scripted mock API is not involved, and the live-API e2e stays where
//! it lives (`tests/integration.rs`, `#[ignore]`d). The lane-failure
//! drivers are unix-shaped (a controlled PATH), so those tests are
//! `#[cfg(unix)]`; the release binary and `fleet chat` halves of the
//! quick start are proven by hand against the live roster, because the
//! chat half is not deterministic and does not belong in a suite.
//!
//! The lane subprocess bound is not drivable through the CLI — its
//! production value is fifteen minutes, not a test's wait — so the
//! hung-tool and streaming cases are unit tests in `src/install.rs`,
//! which drive the bound-injecting seam directly.

use std::fs;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::Command;

use fleet::install::LANES;

/// The fleet binary under test.
const FLEET: &str = env!("CARGO_BIN_EXE_fleet");

/// The exact scaffold the install writes, mirrored here so the test
/// pins the on-disk bytes rather than the module's constant alone.
const SCAFFOLD: &str = "{\n  \"pin\": null,\n  \"providers\": []\n}\n";

/// A placeholder credential: the doctor's credentials check only tests
/// presence and masks what it finds, and no real key is involved
/// anywhere in this suite.
const PLACEHOLDER_KEY: &str = "sk-install-test-placeholder";

/// A scratch root, unique per call.
fn scratch(tag: &str) -> PathBuf {
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let serial = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "fleet-install-it-{tag}-{}-{serial}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// A loopback listener for the doctor's reachability probe, held for
/// the life of the value. Its port rides to the child as
/// `DEEPSEEK_BASE_URL`, so the api check is deterministic and no
/// external host is contacted. The doctor only completes a TCP
/// connect here; nothing is ever served or requested.
struct Probe(TcpListener);

impl Probe {
    fn listen() -> Probe {
        Probe(TcpListener::bind("127.0.0.1:0").expect("a loopback port"))
    }

    fn port(&self) -> u16 {
        self.0.local_addr().expect("a bound port").port()
    }
}

/// Strip the palette's SGR sequences from a command's output, so an
/// assertion names a whole line (`+ sessions`) instead of a fragment
/// that happens not to straddle a color boundary — the eval gate's own
/// stripping rule.
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

/// One `fleet install` run against a scratch home: the doctor's API
/// probe pointed at a live loopback port (or a dead one when `probe`
/// is `None`), a placeholder credential, and optionally a controlled
/// PATH — the seam the lane-failure tests drive. Returns
/// `(stdout, stderr, exit code)`, ANSI-stripped.
fn install(
    home: &Path,
    args: &[&str],
    probe: Option<&Probe>,
    path: Option<&Path>,
) -> (String, String, i32) {
    let mut command = Command::new(FLEET);
    command.arg("install").args(args);
    command.env("FLEET_HOME", home);
    command.env("DEEPSEEK_API_KEY", PLACEHOLDER_KEY);
    let port = probe.map(|probe| probe.port()).unwrap_or(1);
    command.env("DEEPSEEK_BASE_URL", format!("http://127.0.0.1:{port}"));
    if let Some(path) = path {
        command.env("PATH", path);
    }
    let output = command.output().expect("the install runs");
    (
        strip_ansi(&String::from_utf8_lossy(&output.stdout)),
        strip_ansi(&String::from_utf8_lossy(&output.stderr)),
        output.status.code().unwrap_or(-1),
    )
}

#[test]
fn install_creates_the_layout_and_the_scaffold_once() {
    let root = scratch("layout");
    let home = root.join("home");
    let probe = Probe::listen();

    // The first run creates everything and ends green on the gate.
    let (stdout, stderr, code) = install(&home, &[], Some(&probe), None);
    assert_eq!(code, 0, "the doctor gate is green: {stdout}{stderr}");
    for dir in ["sessions", "memories", "skills", "cron", "proposals"] {
        assert!(home.join(dir).is_dir(), "{dir} was created");
        assert!(
            stdout.contains(&format!("+ {dir}")),
            "{dir} is named created: {stdout}"
        );
    }
    assert_eq!(
        fs::read_to_string(home.join("config.json")).unwrap(),
        SCAFFOLD
    );
    assert!(stdout.contains("+ config.json"), "{stdout}");
    // The gate's checks rode along with the install's own report.
    assert!(stdout.contains("── doctor ──"), "{stdout}");
    assert!(stdout.contains("credentials"), "{stdout}");

    // A second run changes nothing and names what exists: every step
    // reads present, and the scaffold's bytes are untouched.
    let (again, stderr, code) = install(&home, &[], Some(&probe), None);
    assert_eq!(code, 0, "the second run is still green: {again}{stderr}");
    for dir in ["sessions", "memories", "skills", "cron", "proposals"] {
        assert!(
            again.contains(&format!("= {dir}")),
            "{dir} is named present: {again}"
        );
        assert!(
            !again.contains(&format!("+ {dir}")),
            "{dir} is not recreated: {again}"
        );
    }
    assert_eq!(
        fs::read_to_string(home.join("config.json")).unwrap(),
        SCAFFOLD
    );
    assert!(again.contains("left untouched"), "{again}");
}

#[test]
fn an_existing_config_is_named_and_never_rewritten() {
    let root = scratch("config");
    let home = root.join("home");
    fs::create_dir_all(&home).unwrap();
    // A hand-edited but VALID config is the owner's: the install names
    // it and leaves the bytes be. (A config too broken to parse is the
    // roster check's failure, and KI-134 made that verdict reach the
    // gate — asserted separately below.)
    fs::write(home.join("config.json"), "{ \"pin\": \"stepfun\" }").unwrap();
    let probe = Probe::listen();

    let (stdout, stderr, code) = install(&home, &[], Some(&probe), None);
    assert_eq!(
        code, 0,
        "an existing config does not fail the install: {stdout}{stderr}"
    );
    assert_eq!(
        fs::read_to_string(home.join("config.json")).unwrap(),
        "{ \"pin\": \"stepfun\" }"
    );
    assert!(stdout.contains("left untouched"), "{stdout}");
}

#[test]
fn an_unparseable_config_fails_the_gate_by_name() {
    // KI-134: a config too broken to load used to print a FAIL line and
    // exit 0. The roster check's verdict now reaches the gate, so the
    // install fails with it — while still leaving the owner's bytes
    // untouched, which is the other half of the contract.
    let root = scratch("config-broken");
    let home = root.join("home");
    fs::create_dir_all(&home).unwrap();
    fs::write(home.join("config.json"), "{ hand edited ").unwrap();
    let probe = Probe::listen();

    let (stdout, _, code) = install(&home, &[], Some(&probe), None);
    assert_ne!(code, 0, "a broken roster is a red gate: {stdout}");
    assert!(
        stdout.contains("the roster did not load"),
        "the failure is named: {stdout}"
    );
    assert_eq!(
        fs::read_to_string(home.join("config.json")).unwrap(),
        "{ hand edited "
    );
    assert!(stdout.contains("left untouched"), "{stdout}");
}

#[test]
fn the_installs_exit_code_is_the_doctors() {
    let root = scratch("gate");
    // Green: a live loopback endpoint and a placeholder credential.
    let probe = Probe::listen();
    let (stdout, stderr, code) = install(&root.join("home-green"), &[], Some(&probe), None);
    assert_eq!(
        code, 0,
        "a green doctor is a green install: {stdout}{stderr}"
    );

    // Red: the api check fails against a dead port, and the install
    // fails with it — the gate is the doctor's own check, named.
    let (stdout, _, code) = install(&root.join("home-red"), &[], None, None);
    assert_ne!(code, 0, "a red doctor is a red install: {stdout}");
    assert!(
        stdout.contains("FAIL"),
        "the failing check is named: {stdout}"
    );
}

#[test]
fn the_quick_start_sequence_runs_against_a_scratch_home() {
    // The README's literal sequence — install, then doctor on the home
    // the install left — against a scratch FLEET_HOME. The release
    // binary and the `fleet chat` half are proven by hand against the
    // live roster; the chat half is not deterministic and stays out of
    // the suite, so this is the sequence's deterministic core.
    let root = scratch("quickstart");
    let home = root.join("home");
    let probe = Probe::listen();

    let (stdout, stderr, code) = install(&home, &[], Some(&probe), None);
    assert_eq!(code, 0, "install is green: {stdout}{stderr}");

    // `fleet doctor` on the installed home: the quick start's second
    // command, green on the home the install left behind.
    let output = Command::new(FLEET)
        .arg("doctor")
        .env("FLEET_HOME", &home)
        .env("DEEPSEEK_API_KEY", PLACEHOLDER_KEY)
        .env(
            "DEEPSEEK_BASE_URL",
            format!("http://127.0.0.1:{}", probe.port()),
        )
        .output()
        .expect("the doctor runs");
    let stdout = strip_ansi(&String::from_utf8_lossy(&output.stdout));
    assert_eq!(output.status.code(), Some(0), "doctor is green: {stdout}");
    assert!(stdout.contains("credentials"), "{stdout}");
    assert!(
        stdout.contains(&home.display().to_string()),
        "the doctor names the installed home: {stdout}"
    );
}

/// A controlled PATH holding `sh` (the doctor's shell probe) and a
/// fake `uv` that records its argv and materializes the venv layout
/// `uv venv` is asked for. `fail_pip` makes the package step exit
/// non-zero — the deterministic failing-lane driver. The fixture is
/// hermetic on purpose: `/bin/mkdir` is absolute because a bare
/// `mkdir` would not resolve on the controlled PATH, and the fake
/// would silently build nothing while still exiting zero.
#[cfg(unix)]
fn fake_tool_dir(dir: &Path, log: &Path, fail_pip: bool) {
    use std::os::unix::fs::PermissionsExt;

    fs::create_dir_all(dir).unwrap();
    std::os::unix::fs::symlink("/bin/sh", dir.join("sh")).unwrap();
    let script = format!(
        "#!/bin/sh\n\
         echo \"$@\" >> '{log}'\n\
         if [ \"$1\" = \"venv\" ]; then /bin/mkdir -p \"$2/bin\" && : > \"$2/bin/python\"; fi\n\
         if [ \"$1\" = \"pip\" ] && [ '{fail_pip}' = \"true\" ]; then echo 'fake uv: no network' >&2; exit 1; fi\n\
         exit 0\n",
        log = log.display(),
        fail_pip = fail_pip
    );
    let uv = dir.join("uv");
    fs::write(&uv, script).unwrap();
    fs::set_permissions(&uv, fs::Permissions::from_mode(0o755)).unwrap();
}

#[cfg(unix)]
#[test]
fn install_lanes_bootstraps_each_lane_idempotently() {
    let root = scratch("lanes");
    let home = root.join("home");
    let tools = root.join("tools");
    let log = root.join("uv-argv.log");
    fake_tool_dir(&tools, &log, false);
    let probe = Probe::listen();

    // The first --lanes run installs all five lanes and the install
    // still ends green on the doctor gate.
    let (stdout, stderr, code) = install(&home, &["--lanes"], Some(&probe), Some(&tools));
    assert_eq!(
        code, 0,
        "a lane that builds does not fail the install: {stdout}{stderr}"
    );
    for lane in ["sys1", "sdm1", "sem1", "gen1", "blend1"] {
        assert!(
            stdout.contains(&format!("+ {lane}")),
            "{lane} is named installed: {stdout}"
        );
        let venv = home.join("venvs").join(lane);
        assert!(
            venv.join("bin").join("python").is_file(),
            "{lane} has its venv"
        );
    }
    // The wiring is named: the environment exports and sys1's config key.
    assert!(stdout.contains("export SEM1_ST_VENV="), "{stdout}");
    assert!(stdout.contains("export SDM1_MODELS_VENV="), "{stdout}");
    assert!(stdout.contains("export GEN1_KOKORO_PYTHON="), "{stdout}");
    assert!(stdout.contains("local_venv"), "{stdout}");
    // blend1's wiring rides beside them: no export at all — the launch
    // spec sets every BLEND1_*_PYTHONPATH at spawn, and BLEND1_PYTHONPATH
    // locates the core's src, not a venv root — and the note naming the
    // cores path-wired rather than pip-installed.
    assert!(
        stdout.contains("the path wiring is automatic at spawn"),
        "{stdout}"
    );
    assert!(!stdout.contains("export BLEND1_PYTHONPATH="), "{stdout}");
    assert!(stdout.contains("BLEND1_SYS1_PYTHONPATH"), "{stdout}");
    assert!(stdout.contains("BLEND1_SEM1_PYTHONPATH"), "{stdout}");
    assert!(stdout.contains("BLEND1_SDM1_PYTHONPATH"), "{stdout}");
    assert!(stdout.contains("BLEND1_GEN1_PYTHONPATH"), "{stdout}");
    // The fork's command shapes: `uv venv <path> --python 3.12`, then
    // `uv pip install --python <venv python> <packages>`.
    let argv = fs::read_to_string(&log).unwrap();
    assert_eq!(
        argv.lines()
            .filter(|line| line.starts_with("venv "))
            .count(),
        LANES.len()
    );
    assert_eq!(
        argv.lines()
            .filter(|line| line.starts_with("pip install "))
            .count(),
        LANES.len()
    );
    for package in [
        "gliner2[local]",
        "structured-data-models",
        "sentence-transformers",
        "kokoro-onnx",
    ] {
        assert!(argv.contains(package), "{package} was installed: {argv}");
    }
    // blend1's own install carries the union of the three siblings'
    // package sets — its core is stdlib-only, so the venv exists to
    // serve the cores its lanes compose.
    let blend1_install = argv
        .lines()
        .find(|line| {
            line.contains("pip install ")
                && line.contains("sentence-transformers")
                && line.contains("structured-data-models")
        })
        .unwrap_or_else(|| panic!("blend1's union install is named: {argv}"));
    assert!(
        blend1_install.contains("gliner2[local]"),
        "{blend1_install}"
    );

    // A second run detects every venv and names it, rebuilding none:
    // the argv log is unchanged, which is what "not rebuilt" means.
    let (again, stderr, code) = install(&home, &["--lanes"], Some(&probe), Some(&tools));
    assert_eq!(code, 0, "the second --lanes run is green: {again}{stderr}");
    for lane in ["sys1", "sdm1", "sem1", "gen1", "blend1"] {
        assert!(
            again.contains(&format!("= {lane}")),
            "{lane} is named present: {again}"
        );
    }
    assert_eq!(fs::read_to_string(&log).unwrap(), argv, "no tool ran twice");
    // The summary names the run's shape: nothing skipped, so the
    // report itself carries the green rather than leaving it to the
    // per-lane lines.
    assert!(
        again.contains(&format!(
            "= lanes        0 of {} lanes installed, {} present, 0 skipped",
            LANES.len(),
            LANES.len()
        )),
        "{again}"
    );
}

#[cfg(unix)]
#[test]
fn a_lane_installed_this_run_carries_its_receipt_and_a_second_run_reads_present() {
    let root = scratch("receipt");
    let home = root.join("home");
    let tools = root.join("tools");
    let log = root.join("uv-argv.log");
    fake_tool_dir(&tools, &log, false);
    let probe = Probe::listen();

    let (stdout, stderr, code) = install(&home, &["--lanes"], Some(&probe), Some(&tools));
    assert_eq!(code, 0, "the first run is green: {stdout}{stderr}");
    // The receipt beside the venv's python: the lane's package list and
    // the UTC minute it landed.
    let venv = home.join("venvs").join("sem1");
    let receipt = fs::read_to_string(venv.join(".fleet-lane")).unwrap();
    assert!(receipt.contains("lane sem1"), "{receipt}");
    assert!(
        receipt.contains("packages sentence-transformers"),
        "{receipt}"
    );
    let installed = receipt
        .lines()
        .find(|line| line.starts_with("installed "))
        .expect("the receipt names its minute");
    assert_eq!(
        installed.trim_start_matches("installed ").len(),
        16,
        "{installed}"
    );
    // blend1's receipt carries the union of its three siblings' sets:
    // its own core is stdlib-only, so the package line names the deps
    // its lanes compose.
    let blend1_receipt =
        fs::read_to_string(home.join("venvs").join("blend1").join(".fleet-lane")).unwrap();
    assert!(blend1_receipt.contains("lane blend1"), "{blend1_receipt}");
    assert!(blend1_receipt.contains("role grounded"), "{blend1_receipt}");
    assert!(
        blend1_receipt
            .contains("packages gliner2[local] structured-data-models sentence-transformers"),
        "{blend1_receipt}"
    );
    assert!(
        stdout.contains(&format!(
            "= lanes        {} of {} lanes installed, 0 present, 0 skipped",
            LANES.len(),
            LANES.len()
        )),
        "{stdout}"
    );

    // The receipt is what the second run reads Present off.
    let argv = fs::read_to_string(&log).unwrap();
    let (again, stderr, code) = install(&home, &["--lanes"], Some(&probe), Some(&tools));
    assert_eq!(code, 0, "the second run is green: {again}{stderr}");
    assert!(again.contains("= sem1"), "{again}");
    assert_eq!(fs::read_to_string(&log).unwrap(), argv, "no tool ran twice");
}

#[cfg(unix)]
#[test]
fn a_partial_venv_without_the_receipt_is_rebuilt_never_read_present() {
    let root = scratch("partial");
    let home = root.join("home");
    let tools = root.join("tools");
    let log = root.join("uv-argv.log");
    fake_tool_dir(&tools, &log, false);
    let probe = Probe::listen();
    // An interrupted prior run: `uv venv` created the venv's python and
    // the packages never landed, so no receipt was written. The
    // presence check is the python *and* the receipt, so this venv
    // reads not-built.
    let venv = home.join("venvs").join("sem1");
    fs::create_dir_all(venv.join("bin")).unwrap();
    fs::write(venv.join("bin").join("python"), "#!/bin/sh\n").unwrap();

    let (stdout, stderr, code) = install(&home, &["--lanes"], Some(&probe), Some(&tools));
    assert_eq!(
        code, 0,
        "the rebuild does not fail the install: {stdout}{stderr}"
    );
    assert!(
        stdout.contains("+ sem1"),
        "{lane} is rebuilt: {stdout}",
        lane = "sem1"
    );
    assert!(!stdout.contains("= sem1"), "never read present: {stdout}");
    assert!(venv.join(".fleet-lane").is_file(), "the receipt landed");
    let argv = fs::read_to_string(&log).unwrap();
    assert_eq!(
        argv.lines()
            .filter(|line| line.starts_with("venv "))
            .count(),
        LANES.len(),
        "every lane was rebuilt: {argv}"
    );

    // With the receipt in place the next run reads Present, and the
    // venv is not rebuilt again.
    let (again, stderr, code) = install(&home, &["--lanes"], Some(&probe), Some(&tools));
    assert_eq!(
        code, 0,
        "the run after the rebuild is green: {again}{stderr}"
    );
    assert!(again.contains("= sem1"), "{again}");
    assert_eq!(fs::read_to_string(&log).unwrap(), argv, "no tool ran twice");
}

#[test]
fn a_file_where_a_directory_belongs_is_a_named_failure() {
    let root = scratch("wrong-kind");
    let home = root.join("home");
    fs::create_dir_all(&home).unwrap();
    // A regular file where the sessions directory belongs: the install
    // names the failure and exits 1 rather than overwriting the file
    // or pretending the step succeeded.
    fs::write(home.join("sessions"), "not a directory\n").unwrap();
    let probe = Probe::listen();

    let (stdout, stderr, code) = install(&home, &[], Some(&probe), None);
    assert_eq!(
        code, 1,
        "a wrong-kind path is a named failure: {stdout}{stderr}"
    );
    assert!(stderr.contains("sessions"), "the path is named: {stderr}");
    assert_eq!(
        fs::read_to_string(home.join("sessions")).unwrap(),
        "not a directory\n",
        "the file is left exactly as it was"
    );
}

#[cfg(unix)]
#[test]
fn a_dangling_config_symlink_is_named_and_nothing_is_written_through_it() {
    let root = scratch("dangling");
    let home = root.join("home");
    fs::create_dir_all(&home).unwrap();
    // `exists()` follows the link, so a dangling one reads absent and
    // the scaffold would be written through it into the target.
    let target = root.join("elsewhere").join("config.json");
    std::os::unix::fs::symlink(&target, home.join("config.json")).unwrap();
    let probe = Probe::listen();

    let (stdout, stderr, code) = install(&home, &[], Some(&probe), None);
    assert_eq!(
        code, 0,
        "a named symlink does not fail the install: {stdout}{stderr}"
    );
    assert!(stdout.contains("symlink"), "the symlink is named: {stdout}");
    assert!(stdout.contains("never written through"), "{stdout}");
    assert!(!target.exists(), "nothing was written through the link");
    assert!(
        home.join("config.json").read_link().is_ok(),
        "the link stands exactly as it was"
    );
}

#[cfg(unix)]
#[test]
fn install_lanes_degrades_without_a_venv_tool() {
    let root = scratch("no-tool");
    let home = root.join("home");
    // A controlled PATH carrying only `sh` (the doctor's shell probe):
    // no uv and no python3 — the deterministic missing-tool driver.
    let tools = root.join("tools");
    fs::create_dir_all(&tools).unwrap();
    std::os::unix::fs::symlink("/bin/sh", tools.join("sh")).unwrap();
    let probe = Probe::listen();

    let (stdout, stderr, code) = install(&home, &["--lanes"], Some(&probe), Some(&tools));
    // The install still succeeds: a lane that cannot build is skipped
    // with a named line, never a hard failure.
    assert_eq!(
        code, 0,
        "degrade-without keeps the install green: {stdout}{stderr}"
    );
    for lane in ["sys1", "sdm1", "sem1", "gen1", "blend1"] {
        assert!(
            stdout.contains(&format!("! {lane}")),
            "{lane} is named skipped: {stdout}"
        );
    }
    assert!(
        stdout.contains("not on PATH"),
        "the reason names the missing tool: {stdout}"
    );
    // The degradation is visible in the report's own summary line, not
    // only in each lane's line — and it is a report line, not a gate.
    assert!(
        stdout.contains(&format!(
            "! lanes        0 of {} lanes installed, 0 present, {} skipped",
            LANES.len(),
            LANES.len()
        )),
        "{stdout}"
    );
    // No venv was left behind: the lane stays declared and dead, which
    // is slice 4's dead-lane behavior.
    assert!(!home.join("venvs").join("sem1").exists());
}

#[cfg(unix)]
#[test]
fn a_lane_whose_packages_fail_is_skipped_and_leaves_nothing_behind() {
    let root = scratch("pip-fails");
    let home = root.join("home");
    let tools = root.join("tools");
    let log = root.join("uv-argv.log");
    // The venv is created, the package install exits non-zero: the
    // deterministic failing-pip driver.
    fake_tool_dir(&tools, &log, true);
    let probe = Probe::listen();

    let (stdout, stderr, code) = install(&home, &["--lanes"], Some(&probe), Some(&tools));
    assert_eq!(
        code, 0,
        "a failing lane is skipped, not fatal: {stdout}{stderr}"
    );
    for lane in ["sys1", "sdm1", "sem1", "gen1", "blend1"] {
        assert!(
            stdout.contains(&format!("! {lane}")),
            "{lane} is named skipped: {stdout}"
        );
    }
    assert!(
        stdout.contains("did not install"),
        "the reason names the failing step: {stdout}"
    );
    // And the summary line carries the degradation too.
    assert!(
        stdout.contains(&format!(
            "! lanes        0 of {} lanes installed, 0 present, {} skipped",
            LANES.len(),
            LANES.len()
        )),
        "{stdout}"
    );
    // The half-built venv was removed, so the lane's state stays
    // honestly absent and a later run retries rather than reading
    // "present" over a venv whose packages never landed.
    assert!(!home.join("venvs").join("sem1").exists());
}

/// KI-128: a stdout pipe closed by the reader is the ordinary case for a
/// CLI, not a crash. Rust ignores SIGPIPE by default, which turns the
/// closed pipe into an `io::Error` every `println!` unwraps — so
/// `fleet install | head -12` used to kill the process with "failed
/// printing to stdout: Broken pipe" and exit 101 after the report had
/// already been read. The restore of the default disposition is one
/// line at the top of `main`; this is the guard that keeps it there.
#[cfg(unix)]
#[test]
fn a_closed_stdout_pipe_is_a_quiet_exit_not_a_panic() {
    use std::io::{BufRead, BufReader};
    // The install's report is the longest output in the crate, so it is
    // the surface most likely to still be writing when the reader goes.
    let home = scratch("pipe");
    fs::create_dir_all(&home).unwrap();
    let mut child = Command::new(FLEET)
        .arg("install")
        .env("FLEET_HOME", &home)
        .env("DEEPSEEK_API_KEY", PLACEHOLDER_KEY)
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("the install spawns");
    // Read one line, then drop the reader: the pipe is closed with the
    // child still holding it open for writing, which is exactly the
    // shape `| head` produces.
    {
        let stdout = child.stdout.take().expect("a piped stdout");
        let mut lines = BufReader::new(stdout).lines();
        let _ = lines.next();
    }
    let status = child.wait().expect("the install ends");
    // The conventional posture: the process dies of the signal the
    // closed pipe delivers (or exits 0 if it finished first), and never
    // with the panic's 101.
    assert_ne!(
        status.code(),
        Some(101),
        "a closed stdout pipe must not panic the process"
    );
    assert!(
        status.code().is_none_or(|code| code <= 1),
        "the exit is the signal or a success, not an error: {status}"
    );
    let _ = fs::remove_dir_all(&home);
}
