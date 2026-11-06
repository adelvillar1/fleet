//! The reload path: replace the serving binary, restart the agent, and
//! VERIFY the post-state before saying the face is up.
//!
//! KI-431 is the defect this module exists for. The deployment runbook
//! was a sequence of shell lines, and one of them — a `launchctl
//! bootout` that exists so the agent re-execs the newly installed
//! inode — had no counterpart that put the service back. When the
//! bootstrap did not happen, the face and both public relays stayed
//! down and the owner read it as "fleet is down". Nothing failed
//! loudly, because nothing checked: the runbook printed the commands
//! it ran, not the state it left.
//!
//! The rule this encodes is that a service outage is named by the step
//! that causes it, never assumed away. So the reload is not a sequence
//! of commands; it is a loop over one question — is the agent serving?
//! — asked of the machine's own records: `launchctl print` for the
//! service and its pid, and a TCP connect for the port. A step that
//! leaves the service absent from its domain re-bootstraps it by name,
//! and the run reports the state it proved rather than the commands it
//! typed.
//!
//! The restart itself stays thin. What is decided here is which verb
//! the observed state calls for, and whether the state afterwards is
//! one the caller may call running; the shell-outs are one line each.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use palette::{ACCENT, BOLD, DIM, FAIL, OK, RESET, TEXT};

/// The ANSI palette, mirroring `install`'s module so the two delivery
/// surfaces read as one.
mod palette {
    pub const ACCENT: &str = "\u{1b}[36m"; // cyan
    pub const OK: &str = "\u{1b}[32m"; // green
    pub const FAIL: &str = "\u{1b}[31m"; // red
    pub const DIM: &str = "\u{1b}[90m"; // dark gray
    pub const TEXT: &str = "\u{1b}[37m"; // gray
    pub const BOLD: &str = "\u{1b}[1m";
    pub const RESET: &str = "\u{1b}[0m";
}

/// The LaunchAgent label the face runs under. The plist's `Label` key
/// and every `launchctl` verb below name this one string, so the
/// domain path is derived rather than retyped.
pub const AGENT_LABEL: &str = "com.fleet.gui";

/// Where the installed binary's single home is. The LaunchAgent that
/// keeps the face up runs this path and never a worktree's `target/`
/// (KI-408): a session working elsewhere would otherwise serve a build
/// nobody else has.
pub const INSTALLED_BINARY: &str = ".fleet/bin/fleet";

/// The window a port verification gets. Loopback either accepts in
/// microseconds or is not there; the bound only stops a wedged stack
/// from hanging the report.
const PORT_PROBE: Duration = Duration::from_millis(750);

/// The grace the agent gets to bind after a restart. It reads the home,
/// opens the store, and starts the remote relays before it serves, so
/// the port is not there the instant the process appears.
const SETTLE: Duration = Duration::from_secs(6);

/// What the domain says about the service, read from the machine's own
/// record rather than remembered from an earlier step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceSnapshot {
    /// The service is registered in the user's launchd domain.
    pub present: bool,
    /// The pid `launchctl print` reported, when the service is running.
    pub pid: Option<u32>,
    /// The service's own `state` word (`running`, `spawn scheduled`, …).
    pub state: Option<String>,
}

/// Whether the agent is serving, which is the only question a reload's
/// own success is judged by.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PostState {
    /// A pid exists and the port answers: the reload is done.
    Serving,
    /// Registered, but no process — the domain holds a service that is
    /// not running.
    NoProcess,
    /// A process exists but nothing answers on the port, or no process
    /// at all: either way nothing is serving.
    NotServing,
}

/// The verb the observed state calls for. Separate from [`PostState`]
/// because "registered but dead" and "not registered" are repaired by
/// different `launchctl` verbs, and calling the wrong one leaves the
/// domain unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Repair {
    /// The service is registered: ask launchd to re-exec it.
    Restart,
    /// The domain lacks the service: load it from its plist by name.
    Bootstrap,
}

/// The post-state, decided from the two facts a reload can actually
/// read. Pure, so the gate is a test rather than an experiment on the
/// owner's running face.
/// @return `Serving` only when both a pid and an answering port were
/// observed; anything else is named as the pair it was.
pub fn post_state(pid: Option<u32>, port_answers: bool) -> PostState {
    match (pid, port_answers) {
        (Some(_), true) => PostState::Serving,
        (None, _) => PostState::NoProcess,
        (Some(_), false) => PostState::NotServing,
    }
}

/// The verb that repairs `snapshot`'s world. Pure, for the same reason.
/// @return `Bootstrap` when the domain lacks the service, `Restart`
/// when it holds one that is not serving.
pub fn repair_for(snapshot: &ServiceSnapshot) -> Repair {
    if snapshot.present {
        Repair::Restart
    } else {
        Repair::Bootstrap
    }
}

/// The service's state, parsed out of `launchctl print`'s own output.
///
/// The parse is deliberately narrow: `pid =` and `state =` are the two
/// lines this module acts on, and the rest of the record (arguments,
/// environment, endpoints) is launchd's business. A key line is one
/// whose text before `=` is exactly the key after trimming, so the
/// nested `state = active` lines under an endpoint are not mistaken for
/// the service's own.
/// @return the snapshot, with `present` decided by the caller from
/// launchd's exit code — a print that failed produced no record.
pub fn parse_snapshot(record: &str, present: bool) -> ServiceSnapshot {
    let mut pid = None;
    let mut state = None;
    for line in record.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim();
        match key {
            "pid" => pid = value.parse::<u32>().ok(),
            "state" => {
                if state.is_none() {
                    state = Some(value.to_string());
                }
            }
            _ => {}
        }
    }
    ServiceSnapshot {
        present,
        pid,
        state,
    }
}

/// The plist that declares the agent, in the user's own LaunchAgents
/// folder. Derived from [`AGENT_LABEL`] so the two cannot drift.
pub fn plist_path(user_home: &Path, label: &str) -> PathBuf {
    user_home
        .join("Library/LaunchAgents")
        .join(format!("{label}.plist"))
}

/// The installed binary's path, in the user's own home.
pub fn installed_binary(user_home: &Path) -> PathBuf {
    user_home.join(INSTALLED_BINARY)
}

/// The label-qualified name launchd's domain verbs take.
///
/// `label` is a parameter rather than the constant because the reload
/// is a general verb, not a hard-wired one: the live proof exercises
/// these same verbs against a scratch agent, and a hard-wired label
/// could only ever be tested by restarting the owner's face.
pub fn domain_target(uid: u32, label: &str) -> String {
    format!("gui/{uid}/{label}")
}

/// The domain a bootstrap loads into.
pub fn domain(uid: u32) -> String {
    format!("gui/{uid}")
}

/// Read the service's state from launchd. `None` is not an error — a
/// domain that cannot be read at all is the same repair as a domain
/// with no service in it, and the caller verifies the outcome either
/// way.
pub fn read_snapshot(uid: u32, label: &str) -> ServiceSnapshot {
    let target = domain_target(uid, label);
    match Command::new("launchctl").args(["print", &target]).output() {
        Ok(output) if output.status.success() => {
            parse_snapshot(&String::from_utf8_lossy(&output.stdout), true)
        }
        _ => ServiceSnapshot {
            present: false,
            pid: None,
            state: None,
        },
    }
}

/// Whether the port answers. A TCP connect, not an HTTP read: the
/// question is whether something is bound and accepting, and a face
/// that answers a request slowly is still serving.
pub fn port_answers(host: &str, port: u16) -> bool {
    crate::net::listening(host, port, PORT_PROBE)
}

/// Run one `launchctl` verb, returning its own words on failure so the
/// report names what launchd said rather than what was attempted.
fn run_launchctl(args: &[&str]) -> Result<(), String> {
    match Command::new("launchctl").args(args).output() {
        Ok(output) if output.status.success() => Ok(()),
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            let text = format!("{}{}", stderr.trim(), stdout.trim());
            Err(if text.is_empty() {
                format!("launchctl {} exited {}", args.join(" "), output.status)
            } else {
                format!("launchctl {}: {}", args.join(" "), text.trim())
            })
        }
        Err(error) => Err(format!("launchctl {}: {error}", args.join(" "))),
    }
}

/// Ask launchd to re-exec the registered service.
pub fn restart(uid: u32, label: &str) -> Result<(), String> {
    run_launchctl(&["kickstart", "-k", &domain_target(uid, label)])
}

/// Load the service into the domain from its plist, by name.
pub fn bootstrap(uid: u32, plist: &Path) -> Result<(), String> {
    let plist = plist.to_string_lossy().to_string();
    run_launchctl(&["bootstrap", &domain(uid), &plist])
}

/// Unload the service from the domain. Only the escalated repair needs
/// this, and it always bootstraps again in the same breath.
pub fn bootout(uid: u32, label: &str) -> Result<(), String> {
    run_launchctl(&["bootout", &domain_target(uid, label)])
}

/// Copy `source` over the installed binary through a NEW INODE.
///
/// The `mv` is not cosmetic. The running daemon has the installed file
/// mapped, so an in-place write over it leaves a binary whose every
/// later exec macOS kills with SIGKILL (exit 137) while the running
/// process keeps serving — the installed copy reads dead and only the
/// restart that needed it finds out (KI-418).
/// @return the installed path, or why the swap did not land.
pub fn install_binary(source: &Path, installed: &Path) -> Result<(), String> {
    let staged = installed.with_extension("new");
    if let Some(parent) = installed.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("{}: {error}", parent.display()))?;
    }
    std::fs::copy(source, &staged).map_err(|error| {
        format!(
            "could not copy {} to {}: {error}",
            source.display(),
            staged.display()
        )
    })?;
    // The executable bit is the one property a delivery cannot guess.
    // `fs::copy` carries the source's mode, so a source that was never
    // built as an executable installs a binary launchd cannot exec — and
    // the failure would surface at the restart, as the face staying
    // down, which is precisely the shape KI-431 exists to end. Refuse it
    // here, by name, where the cause is still visible.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(source)
            .map_err(|error| format!("{}: {error}", source.display()))?
            .permissions()
            .mode();
        if mode & 0o111 == 0 {
            let _ = std::fs::remove_file(&staged);
            return Err(format!(
                "{} is not executable (mode {mode:o}); install a built binary",
                source.display()
            ));
        }
        let _ = std::fs::set_permissions(&staged, std::fs::Permissions::from_mode(mode));
    }
    std::fs::rename(&staged, installed).map_err(|error| {
        format!(
            "could not move {} into place at {}: {error}",
            staged.display(),
            installed.display()
        )
    })
}

/// The report one reload is judged by: what the state was, what was
/// done about it, and what the state is now.
#[derive(Debug, Clone)]
pub struct ReloadReport {
    /// The agent this reload acted on. Carried rather than read off
    /// [`AGENT_LABEL`] at render time: the report names the service it
    /// verified, and a header that always said "com.fleet.gui" would be
    /// a lie the moment the verb reloaded anything else.
    pub label: String,
    pub before: ServiceSnapshot,
    pub after: ServiceSnapshot,
    pub port_after: bool,
    /// The verbs run, in order, so a reader sees the escalation when
    /// one was needed.
    pub steps: Vec<String>,
    /// Problems that did not stop the run, in the order they happened.
    pub notes: Vec<String>,
}

impl ReloadReport {
    /// The verdict the exit code follows.
    pub fn post_state(&self) -> PostState {
        post_state(self.after.pid, self.port_after)
    }

    /// True when the reload proved the face is serving.
    pub fn serving(&self) -> bool {
        self.post_state() == PostState::Serving
    }

    /// One state row: the label, the state word with its pid, and the
    /// record's shape when there is something to say about it. The
    /// third column is omitted rather than rendered empty, so no line
    /// ends on a stray colour reset.
    fn row(&self, label: &str, snapshot: &ServiceSnapshot) -> String {
        let detail = detail(snapshot);
        if detail.is_empty() {
            format!("  {}  {}\n", word(label, ACCENT), describe(snapshot))
        } else {
            format!(
                "  {}  {}  {DIM}{detail}{RESET}\n",
                word(label, ACCENT),
                describe(snapshot)
            )
        }
    }

    /// The whole report as the operator reads it: the state found, each
    /// verb and what it did, and the state left.
    pub fn render(&self, host: &str, port: u16) -> String {
        let mut out = String::new();
        out.push_str(&format!(
            "{BOLD}reload{RESET} {DIM}{}{RESET}\n\n",
            self.label
        ));
        out.push_str(&self.row("before", &self.before));
        for step in &self.steps {
            out.push_str(&format!("  {}  {}\n", word("step  ", DIM), step));
        }
        for note in &self.notes {
            out.push_str(&format!("  {}  {}\n", word("note  ", DIM), note));
        }
        out.push_str(&self.row("after ", &self.after));
        out.push('\n');
        match self.post_state() {
            PostState::Serving => out.push_str(&format!(
                "  {}  the face is serving on {host}:{port}\n",
                word("ok    ", OK),
            )),
            PostState::NoProcess => out.push_str(&format!(
                "  {}  no process in the domain after the reload; the face is DOWN\n",
                word("FAIL  ", FAIL),
            )),
            PostState::NotServing => out.push_str(&format!(
                "  {}  a process is up but {host}:{port} does not answer; the face is DOWN\n",
                word("FAIL  ", FAIL),
            )),
        }
        out
    }
}

/// One left-hand label, padded so the report's columns line up.
fn word(text: &str, color: &str) -> String {
    format!("{color}{text:<7}{RESET}")
}

/// The state word plus pid, as the report's second column.
fn describe(snapshot: &ServiceSnapshot) -> String {
    let state = snapshot.state.as_deref().unwrap_or("absent");
    match snapshot.pid {
        Some(pid) => format!("{TEXT}{state}{RESET} {DIM}pid {pid}{RESET}"),
        None => format!("{TEXT}{state}{RESET}"),
    }
}

/// The third column: the record's own shape, so a reader can tell
/// "registered, not running" from "never registered".
fn detail(snapshot: &ServiceSnapshot) -> String {
    if snapshot.present {
        String::new()
    } else {
        "not in the domain".to_string()
    }
}

/// The whole reload: verify, repair, verify again, and — when the first
/// repair did not take — escalate to the bootout/bootstrap pair the
/// runbook always meant to run together.
///
/// `install` is the binary to put in place first, if the caller had one
/// to deliver. Its absence is not a failure: a reload with nothing to
/// install is the plain restart.
/// @return the report, whose `serving()` is the only success signal.
pub fn reload(
    uid: u32,
    user_home: &Path,
    host: &str,
    port: u16,
    install: Option<&Path>,
) -> ReloadReport {
    reload_labeled(uid, AGENT_LABEL, user_home, host, port, install)
}

/// The same reload against a named agent. `reload` is this with the
/// face's own label, and the parameter exists so the repair path can be
/// proved live against a scratch agent instead of by taking the
/// owner's face down to test it.
pub fn reload_labeled(
    uid: u32,
    label: &str,
    user_home: &Path,
    host: &str,
    port: u16,
    install: Option<&Path>,
) -> ReloadReport {
    let mut steps = Vec::new();
    let mut notes = Vec::new();

    if let Some(source) = install {
        let installed = installed_binary(user_home);
        match self::install_binary(source, &installed) {
            Ok(()) => steps.push(format!(
                "installed {} at {}",
                source.display(),
                installed.display()
            )),
            Err(error) => notes.push(error),
        }
    }

    let before = read_snapshot(uid, label);
    let repair = repair_for(&before);
    match repair {
        Repair::Restart => match restart(uid, label) {
            Ok(()) => steps.push("restart: asked launchd to re-exec the service".to_string()),
            Err(error) => notes.push(error),
        },
        Repair::Bootstrap => {
            let plist = plist_path(user_home, label);
            match bootstrap(uid, &plist) {
                Ok(()) => steps.push(format!("bootstrap: loaded {}", plist.display())),
                Err(error) => notes.push(error),
            }
        }
    }

    std::thread::sleep(SETTLE);
    let mut after = read_snapshot(uid, label);

    // The escalation. The first verb repaired the world it read; if the
    // service is now absent, or a process exists with nothing bound,
    // the pair is run together so the domain cannot be left holding a
    // service that is not there.
    if post_state(after.pid, port_answers(host, port)) != PostState::Serving {
        if after.present || !before.present {
            if let Err(error) = bootout(uid, label) {
                notes.push(error);
            } else {
                steps.push("bootout: unloaded the service for a clean load".to_string());
            }
            let plist = plist_path(user_home, label);
            match bootstrap(uid, &plist) {
                Ok(()) => steps.push(format!("bootstrap: reloaded {}", plist.display())),
                Err(error) => notes.push(error),
            }
            std::thread::sleep(SETTLE);
            after = read_snapshot(uid, label);
        }
    }

    let port_after = port_answers(host, port);
    ReloadReport {
        label: label.to_string(),
        before,
        after,
        port_after,
        steps,
        notes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pid_and_an_answering_port_is_the_only_serving_state() {
        assert_eq!(post_state(Some(94983), true), PostState::Serving);
        assert_eq!(post_state(None, true), PostState::NoProcess);
        assert_eq!(post_state(Some(94983), false), PostState::NotServing);
        assert_eq!(post_state(None, false), PostState::NoProcess);
    }

    #[test]
    fn a_domain_with_no_service_is_bootstrapped_and_one_with_a_service_is_restarted() {
        let absent = ServiceSnapshot {
            present: false,
            pid: None,
            state: None,
        };
        assert_eq!(repair_for(&absent), Repair::Bootstrap);

        // Registered but dead: the domain holds the service, so the
        // repair is a re-exec, not a second bootstrap.
        let dead = ServiceSnapshot {
            present: true,
            pid: None,
            state: Some("spawn scheduled".to_string()),
        };
        assert_eq!(repair_for(&dead), Repair::Restart);
    }

    #[test]
    fn the_snapshot_parse_reads_the_service_s_own_pid_and_state() {
        let record = "\
gui/501/com.fleet.gui = {
\tactive count = 1
\tpath = /Users/x/Library/LaunchAgents/com.fleet.gui.plist
\tstate = running

\tpid = 94983

\tendpoint = {
\t\tstate = active
\t}
}
";
        let snapshot = parse_snapshot(record, true);
        assert!(snapshot.present);
        assert_eq!(snapshot.pid, Some(94983));
        // The first `state =` is the service's; the nested one under
        // `endpoint` belongs to an inherited environment entry.
        assert_eq!(snapshot.state.as_deref(), Some("running"));
    }

    #[test]
    fn a_scheduled_service_reports_no_pid_rather_than_a_zero() {
        let record = "\tstate = spawn scheduled\n\tlast exit code = 137\n";
        let snapshot = parse_snapshot(record, true);
        assert_eq!(snapshot.pid, None);
        assert_eq!(snapshot.state.as_deref(), Some("spawn scheduled"));
    }

    #[test]
    fn a_failed_print_reads_absent_rather_than_a_service_with_no_pid() {
        // launchd answers a missing label with a non-zero exit and no
        // record; `read_snapshot` collapses that to absent, and the
        // snapshot must not claim the service was there.
        let snapshot = parse_snapshot("", false);
        assert!(!snapshot.present);
        assert_eq!(snapshot.pid, None);
        assert_eq!(repair_for(&snapshot), Repair::Bootstrap);
    }

    #[test]
    fn the_report_says_the_face_is_down_when_the_port_never_answered() {
        let report = ReloadReport {
            label: "com.fleet.gui".to_string(),
            before: ServiceSnapshot {
                present: false,
                pid: None,
                state: None,
            },
            after: ServiceSnapshot {
                present: true,
                pid: Some(4242),
                state: Some("running".to_string()),
            },
            port_after: false,
            steps: vec!["bootstrap: loaded the plist".to_string()],
            notes: Vec::new(),
        };
        assert!(!report.serving());
        assert_eq!(report.post_state(), PostState::NotServing);
        let rendered = report.render("127.0.0.1", 3080);
        // The old behavior printed the commands it ran and called that
        // success; the report must state the verdict instead.
        assert!(rendered.contains("the face is DOWN"), "{rendered}");
        assert!(rendered.contains("127.0.0.1:3080"), "{rendered}");
    }

    #[test]
    fn the_report_says_the_face_is_up_only_when_both_facts_were_proved() {
        let report = ReloadReport {
            label: "com.fleet.gui".to_string(),
            before: ServiceSnapshot {
                present: true,
                pid: Some(1),
                state: Some("running".to_string()),
            },
            after: ServiceSnapshot {
                present: true,
                pid: Some(4242),
                state: Some("running".to_string()),
            },
            port_after: true,
            steps: Vec::new(),
            notes: Vec::new(),
        };
        assert!(report.serving());
        assert!(report.render("127.0.0.1", 3080).contains("is serving"));
    }

    #[test]
    fn the_report_names_the_agent_it_acted_on_and_leaves_no_empty_column() {
        // The header is read off the report, not off the constant: a
        // verb that reloaded some other agent must say so, or the
        // operator reads a verified service that was never touched.
        let report = ReloadReport {
            label: "com.fleet.somebody-else".to_string(),
            before: ServiceSnapshot {
                present: true,
                pid: Some(1),
                state: Some("running".to_string()),
            },
            after: ServiceSnapshot {
                present: true,
                pid: Some(2),
                state: Some("running".to_string()),
            },
            port_after: true,
            steps: Vec::new(),
            notes: Vec::new(),
        };
        let rendered = report.render("127.0.0.1", 3080);
        assert!(rendered.contains("com.fleet.somebody-else"), "{rendered}");
        // A state row with nothing to add ends at the pid; the live run
        // showed a dangling reset sequence there.
        for line in rendered.lines() {
            assert!(
                !line.trim_end().ends_with("\u{1b}[0m\u{1b}[0m"),
                "row ended on a doubled reset: {line:?}"
            );
            assert!(
                !line.trim_end().ends_with("  \u{1b}[0m"),
                "row ended on an empty detail column: {line:?}"
            );
        }
    }

    #[test]
    fn the_agent_label_derives_the_plist_and_the_domain_target() {
        let home = Path::new("/Users/x");
        assert_eq!(
            plist_path(home, AGENT_LABEL),
            PathBuf::from("/Users/x/Library/LaunchAgents/com.fleet.gui.plist")
        );
        assert_eq!(
            installed_binary(home),
            PathBuf::from("/Users/x/.fleet/bin/fleet")
        );
        assert_eq!(domain_target(501, AGENT_LABEL), "gui/501/com.fleet.gui");
        assert_eq!(domain(501), "gui/501");
    }

    /// Build a scratch source that is executable, the way a real build
    /// directory's binary is. A fixture written with `fs::write` lands
    /// 0o644, so a test that skipped this would assert against a source
    /// no delivery ever installs.
    fn a_built_binary(root: &Path, name: &str, bytes: &[u8]) -> PathBuf {
        let path = root.join(name);
        std::fs::write(&path, bytes).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        path
    }

    #[test]
    fn installing_a_binary_leaves_it_executable_and_does_not_write_in_place() {
        let root = std::env::temp_dir().join(format!("fleet-reload-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let source = a_built_binary(&root, "source", b"first");
        let installed = root.join("home/.fleet/bin/fleet");
        install_binary(&source, &installed).unwrap();
        assert_eq!(std::fs::read(&installed).unwrap(), b"first");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&installed).unwrap().permissions().mode();
            assert!(mode & 0o111 != 0, "installed binary lost its exec bit");
        }
        // A second install replaces the bytes...
        a_built_binary(&root, "source", b"second");
        install_binary(&source, &installed).unwrap();
        assert_eq!(std::fs::read(&installed).unwrap(), b"second");
        // ...and the staging file is gone, so nothing is left beside it.
        assert!(!installed.with_extension("new").exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_source_that_was_never_executable_is_refused_by_name() {
        let root = std::env::temp_dir().join(format!("fleet-reload-x-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let source = root.join("stale");
        std::fs::write(&source, b"not a binary").unwrap();
        let installed = root.join("home/.fleet/bin/fleet");
        let error = install_binary(&source, &installed).unwrap_err();
        assert!(error.contains("not executable"), "{error}");
        assert!(error.contains("stale"), "{error}");
        // The refusal leaves nothing installed and no staging file: the
        // serving path is untouched rather than half-delivered.
        assert!(!installed.exists());
        assert!(!installed.with_extension("new").exists());
        let _ = std::fs::remove_dir_all(&root);
    }
}
