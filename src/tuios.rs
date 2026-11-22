//! tuios in the armada map: what the owner's multiplexer carries.
//!
//! The owner runs tuios (0.13.3) as their desktop shell, and Fleet's TUI
//! already runs inside it as a daemon-kept pane. This module is the
//! armada map's second tier: the per-instance section that names the
//! sessions, the windows by their AGENT state, the agents' own words, and
//! the panes that run Fleet — read through tuios's own JSON control
//! protocol (`tuios call <verb> [json]`, control protocol v1) and nothing
//! else.
//!
//! **Read-only over the protocol.** The map calls `list-sessions`,
//! `list-windows`, `list-agents` and `list-attention`. It never calls
//! `add-task`, `send-keys`, or any verb that changes the multiplexer:
//! Fleet watches the owner's shell, it does not drive it.
//!
//! **Same-document law.** Every word in the section is the daemon's own
//! answer for this call — the AGENT states pass through verbatim
//! (`working`, `needs_input`, `idle`, `done`, `errored`, `unknown`,
//! `limit_reached`, `none`), never mapped onto Fleet's vocabulary and
//! never cached. `agent-activity` is deliberately not read: tuios marks it
//! untrusted itself, and the map shows, it does not judge.
//!
//! **The refusal is named.** A machine whose daemon does not answer gets
//! `tuios is not running on ⟨sid⟩`, never an empty grid: an empty session
//! list is a measurement, an unreachable daemon is not.
//!
//! **Per-instance opt-in.** `<home>/tuios.json` names the sids whose
//! machine runs the daemon (and, when it is not the default socket, the
//! `--server` name). An instance the owner has not enabled carries no
//! section at all.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};

/// The env var that names the binary, for a machine whose tuios is not on
/// `PATH`. Same shape as kanzo's own.
pub const BIN_ENV: &str = "FLEET_TUIOS_BIN";

/// The exit status of a call whose daemon cannot be reached (`tuios call`
/// exits 3: no daemon, or no answer before the timeout).
pub const EXIT_UNREACHABLE: i32 = 3;

/// How long one verb call waits. The daemon answers in milliseconds when
/// it is there; a hung one names itself rather than stalling a board read.
pub const CALL_TIMEOUT: &str = "4s";

/// The opt-in file: `<home>/tuios.json`.
pub fn config_path(home: &Path) -> PathBuf {
    home.join("tuios.json")
}

/// One instance's opt-in: the daemon it carries, and the `--server` name
/// when it is not the default socket.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OptInInstance {
    #[serde(default)]
    pub enabled: bool,
    /// The named daemon (`tuios --server NAME`); empty for the default.
    #[serde(default)]
    pub server: String,
}

/// The per-instance opt-in: sid → its daemon. An absent file is an empty
/// opt-in, which is the ordinary install — no instance carries a tuios
/// section until the owner says which machine runs the daemon.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OptIn {
    #[serde(default)]
    pub instances: BTreeMap<String, OptInInstance>,
}

impl OptIn {
    pub fn load(home: &Path) -> OptIn {
        std::fs::read_to_string(config_path(home))
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    /// Write the opt-in atomically (a temp beside it, then a rename), so a
    /// reader never sees half a file.
    pub fn save(home: &Path, opt_in: &OptIn) -> Result<(), String> {
        let path = config_path(home);
        let text =
            serde_json::to_string_pretty(opt_in).map_err(|error| format!("the opt-in: {error}"))?;
        let tmp = path.with_extension(format!("tmp-{}", std::process::id()));
        std::fs::write(&tmp, text).map_err(|error| format!("the opt-in write: {error}"))?;
        std::fs::rename(&tmp, &path).map_err(|error| format!("the opt-in rename: {error}"))?;
        Ok(())
    }

    /// Enable the section for one instance, naming the daemon it runs.
    pub fn enable(home: &Path, sid: &str, server: &str) -> Result<(), String> {
        if sid.trim().is_empty() {
            return Err("a tuios section names the instance it reads for".to_string());
        }
        let mut opt_in = OptIn::load(home);
        opt_in.instances.insert(
            sid.to_string(),
            OptInInstance {
                enabled: true,
                server: server.to_string(),
            },
        );
        OptIn::save(home, &opt_in)
    }

    /// Disable the section for one instance (the row returns to its four
    /// cells; nothing is left half-rendered).
    pub fn disable(home: &Path, sid: &str) -> Result<(), String> {
        let mut opt_in = OptIn::load(home);
        opt_in.instances.remove(sid);
        OptIn::save(home, &opt_in)
    }

    /// The daemon this instance carries, when the owner enabled it.
    pub fn enabled_for(&self, sid: &str) -> Option<&OptInInstance> {
        self.instances.get(sid).filter(|entry| entry.enabled)
    }
}

/// One pane in a session that runs Fleet: the daemon's own reading of the
/// live process, so the map can say "Fleet's TUI is here" without guessing
/// from the pane's text (the TUI's frame says "fleet" everywhere, which is
/// not a discriminator; the command line is).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FleetPane {
    pub window: String,
    pub command: String,
    pub cwd: String,
}

/// One tuios session as the section carries it: its name, its window
/// count, its windows counted by the daemon's AGENT state words, and the
/// panes that run Fleet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Session {
    pub name: String,
    pub windows: usize,
    /// The daemon's AGENT state words, verbatim, each with its count.
    pub agents: BTreeMap<String, u64>,
    pub fleet_panes: Vec<FleetPane>,
    /// A verb that did not answer for this session, named. Absent when
    /// the read landed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub probe_error: Option<String>,
}

/// One agent pane as the daemon lists it: the name, the session, and its
/// state word verbatim.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Agent {
    pub session: String,
    pub name: String,
    pub state: String,
    pub foreground: String,
}

/// The tuios section of one instance's map row. `refusal` set means the
/// daemon did not answer and `sessions` is empty on purpose — the card
/// paints the refusal, never a grid.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Section {
    pub sid: String,
    /// The `--server` name, empty for the default socket.
    pub server: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refusal: Option<String>,
    pub sessions: Vec<Session>,
    pub agents: Vec<Agent>,
    /// The daemon's own count of open attention items across every
    /// session and machine it knows (list-attention's `total`).
    pub attention_open: u64,
}

impl Section {
    fn refused(sid: &str, server: &str, words: String) -> Section {
        Section {
            sid: sid.to_string(),
            server: server.to_string(),
            refusal: Some(words),
            sessions: Vec::new(),
            agents: Vec::new(),
            attention_open: 0,
        }
    }
}

/// A call that did not land, in the three shapes the map must tell apart:
/// no binary, no daemon, and a daemon that answered with an error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Failure {
    /// No `tuios` on PATH (and no `FLEET_TUIOS_BIN` naming one).
    NotInstalled(String),
    /// Exit 3: the daemon is not running, or did not answer in time.
    NotRunning(String),
    /// A verb error: the daemon answered `{error:{code,message}}` and
    /// exited 1.
    Refused { code: String, message: String },
    /// The answer was not the JSON the verb promised.
    Unreadable(String),
}

impl Failure {
    /// The words the card paints. The refusal names the instance, because
    /// "tuios is not running" alone leaves the owner asking which machine.
    pub fn refusal(&self, sid: &str) -> String {
        match self {
            Failure::NotInstalled(detail) => format!("tuios is not installed on {sid} ({detail})"),
            Failure::NotRunning(detail) => {
                let detail = detail.trim();
                if detail.is_empty() {
                    format!("tuios is not running on {sid}")
                } else {
                    format!("tuios is not running on {sid} — {detail}")
                }
            }
            Failure::Refused { code, message } => {
                format!("the tuios daemon on {sid} refused: {code} — {message}")
            }
            Failure::Unreadable(detail) => {
                format!("the tuios answer on {sid} could not be read: {detail}")
            }
        }
    }
}

/// Find the binary: `FLEET_TUIOS_BIN` when set (absolute, as kanzo's own
/// rule), else `tuios` on `PATH`.
pub fn binary() -> Result<PathBuf, Failure> {
    if let Some(named) = std::env::var_os(BIN_ENV) {
        let named = PathBuf::from(named);
        if !named.is_absolute() {
            return Err(Failure::NotInstalled(format!(
                "{BIN_ENV}={} is a relative path; an absolute one is required",
                named.display()
            )));
        }
        return if named.is_file() {
            Ok(named)
        } else {
            Err(Failure::NotInstalled(format!(
                "{BIN_ENV}={} is not a file",
                named.display()
            )))
        };
    }
    if let Some(path) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path) {
            let candidate = dir.join("tuios");
            if candidate.is_file() {
                return Ok(candidate);
            }
        }
    }
    if let Some(home) = std::env::var_os("HOME") {
        for sub in [".local/bin", ".cargo/bin", "go/bin"] {
            let candidate = PathBuf::from(&home).join(sub).join("tuios");
            if candidate.is_file() {
                return Ok(candidate);
            }
        }
    }
    Err(Failure::NotInstalled(
        "no `tuios` on PATH; install it or name one with FLEET_TUIOS_BIN".to_string(),
    ))
}

/// One verb call: `tuios call <verb> [params] [--server NAME] --timeout
/// 4s --json`. Read-only by construction — the verb is the caller's and
/// this module only ever names the four list verbs.
pub fn call(
    program: &Path,
    server: &str,
    verb: &str,
    params: Option<&str>,
) -> Result<serde_json::Value, Failure> {
    let mut command = Command::new(program);
    command.arg("call").arg(verb);
    if let Some(params) = params {
        command.arg(params);
    }
    if !server.trim().is_empty() {
        command.arg("--server").arg(server);
    }
    command.arg("--timeout").arg(CALL_TIMEOUT).arg("--json");
    let output = command
        .output()
        .map_err(|error| Failure::NotInstalled(format!("the call could not run: {error}")))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if output.status.code() == Some(EXIT_UNREACHABLE) {
        // With --json the transport failure is JSON too; the message is
        // the daemon's own, trimmed to its first line so a row stays a row.
        let detail = serde_json::from_str::<serde_json::Value>(stdout.trim())
            .ok()
            .and_then(|value| {
                value
                    .get("error")
                    .and_then(|error| error.get("message"))
                    .and_then(|message| message.as_str())
                    // The daemon's message can run to several lines (the
                    // cause, the fix); a row is one line, so the first one
                    // is the refusal and the rest is a paragraph nobody
                    // reads in a cell.
                    .map(|message| message.lines().next().unwrap_or("").to_string())
            })
            .or_else(|| {
                let text = if stdout.trim().is_empty() {
                    stderr.trim()
                } else {
                    stdout.trim()
                };
                (!text.is_empty()).then(|| text.lines().next().unwrap_or("").to_string())
            })
            .unwrap_or_default();
        return Err(Failure::NotRunning(detail));
    }
    let text = stdout.trim();
    let parsed: serde_json::Value = match serde_json::from_str(text) {
        Ok(parsed) => parsed,
        Err(error) => {
            return Err(Failure::Unreadable(format!(
                "{verb} answered no JSON ({error}); stderr: {}",
                stderr.trim()
            )))
        }
    };
    if let Some(error) = parsed.get("error") {
        let code = error
            .get("code")
            .and_then(|code| code.as_str())
            .unwrap_or("unknown")
            .to_string();
        let message = error
            .get("message")
            .and_then(|message| message.as_str())
            .unwrap_or("no message")
            .to_string();
        return Err(Failure::Refused { code, message });
    }
    if !output.status.success() {
        return Err(Failure::Unreadable(format!(
            "{verb} exited {} without an error object",
            output
                .status
                .code()
                .map(|code| code.to_string())
                .unwrap_or_else(|| "a signal".to_string())
        )));
    }
    Ok(parsed)
}

/// **The rule: a fleet pane is matched by the live command line.** A pane
/// runs Fleet when the program in front of it — the daemon's
/// `foreground_cmd`, or the first word of `running_cmdline` (which
/// `list-windows` documents as present "while a command runs") — is the
/// fleet binary by basename: `fleet`, `./target/debug/fleet`, or any path
/// ending in `/fleet`.
///
/// Three things are deliberately *not* evidence:
///
/// - `last_cmdline` is history: a shell that ran `fleet tui` an hour ago
///   and sits at its prompt now is a shell, and the map must say so.
/// - `cwd` is a directory: every pane in the repo has the repo's path,
///   including the ones running `vim`.
/// - the pane's text is the TUI's own frame, which says "fleet" on every
///   row — a capture would match any pane showing the docs.
fn names_fleet(window: &serde_json::Value) -> Option<FleetPane> {
    let program = |text: &str| -> String {
        let first = text.split_whitespace().next().unwrap_or("");
        Path::new(first)
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| first.to_string())
    };
    let foreground = window
        .get("foreground_cmd")
        .and_then(|value| value.as_str())
        .unwrap_or("");
    let running = window
        .get("running_cmdline")
        .and_then(|value| value.as_str())
        .unwrap_or("");
    let command = if !foreground.is_empty() && program(foreground) == "fleet" {
        foreground
    } else if !running.is_empty() && program(running) == "fleet" {
        running
    } else {
        return None;
    };
    let name = window
        .get("display_name")
        .or_else(|| window.get("title"))
        .and_then(|value| value.as_str())
        .or_else(|| window.get("window_id").and_then(|value| value.as_str()))
        .unwrap_or("a pane")
        .to_string();
    Some(FleetPane {
        window: name,
        command: command.to_string(),
        cwd: window
            .get("cwd")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .to_string(),
    })
}

/// The section for one instance, when the owner enabled it. `None` is the
/// ordinary install: no opt-in, no section, and the row keeps its four
/// cells.
pub fn section(home: &Path, sid: &str) -> Option<Section> {
    let opt_in = OptIn::load(home);
    let entry = opt_in.enabled_for(sid)?;
    let server = entry.server.clone();
    Some(match binary() {
        Ok(program) => section_with(sid, &server, &program),
        Err(failure) => Section::refused(sid, &server, failure.refusal(sid)),
    })
}

/// The section read with a named binary: the test seam, and what
/// [`section`] uses once it has found the daemon.
pub fn section_with(sid: &str, server: &str, program: &Path) -> Section {
    // The daemon's session list is the first read: if it does not answer,
    // the whole section is the refusal — an unreachable daemon must never
    // render as a session list of none.
    let sessions_answer = match call(program, server, "list-sessions", None) {
        Ok(answer) => answer,
        Err(failure) => return Section::refused(sid, server, failure.refusal(sid)),
    };
    let mut sessions = Vec::new();
    let mut names = Vec::new();
    if let Some(rows) = sessions_answer
        .get("sessions")
        .and_then(|rows| rows.as_array())
    {
        for row in rows {
            let Some(name) = row.get("name").and_then(|name| name.as_str()) else {
                continue;
            };
            names.push(name.to_string());
            let declared = row
                .get("window_count")
                .and_then(|count| count.as_u64())
                .unwrap_or(0) as usize;
            // The AGENT column is list-windows' own: one call per session,
            // because list-windows answers for the session it is given (a
            // call with no session answers for the most recent one, which
            // would attribute one session's windows to another).
            match call(
                program,
                server,
                "list-windows",
                Some(&format!("{{\"session\":{}}}", serde_json::json!(name))),
            ) {
                Ok(windows_answer) => {
                    let windows = windows_answer
                        .get("windows")
                        .and_then(|windows| windows.as_array())
                        .cloned()
                        .unwrap_or_default();
                    let mut states: BTreeMap<String, u64> = BTreeMap::new();
                    let mut fleet_panes = Vec::new();
                    for window in &windows {
                        let state = window
                            .get("agent_state")
                            .and_then(|state| state.as_str())
                            .unwrap_or("unknown")
                            .to_string();
                        *states.entry(state).or_insert(0) += 1;
                        if let Some(pane) = names_fleet(window) {
                            fleet_panes.push(pane);
                        }
                    }
                    sessions.push(Session {
                        name: name.to_string(),
                        windows: windows.len(),
                        agents: states,
                        fleet_panes,
                        probe_error: None,
                    });
                }
                Err(failure) => sessions.push(Session {
                    name: name.to_string(),
                    windows: declared,
                    agents: BTreeMap::new(),
                    fleet_panes: Vec::new(),
                    probe_error: Some(failure.refusal(sid)),
                }),
            }
        }
    }
    // The agents: one call for every session the daemon holds (`all`).
    let agents = match call(program, server, "list-agents", Some("{\"all\":true}")) {
        Ok(answer) => answer
            .get("agents")
            .and_then(|agents| agents.as_array())
            .map(|agents| {
                agents
                    .iter()
                    .filter_map(|agent| {
                        Some(Agent {
                            session: agent.get("session")?.as_str()?.to_string(),
                            name: agent.get("name")?.as_str()?.to_string(),
                            state: agent.get("state")?.as_str()?.to_string(),
                            foreground: agent
                                .get("foreground")
                                .and_then(|value| value.as_str())
                                .unwrap_or("")
                                .to_string(),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default(),
        // A daemon that answers list-sessions but not list-agents is a
        // named probe failure, not a silent empty list.
        Err(failure) => {
            for session in &mut sessions {
                if session.probe_error.is_none() {
                    session.probe_error = Some(failure.refusal(sid));
                }
            }
            Vec::new()
        }
    };
    let attention_open = call(program, server, "list-attention", None)
        .ok()
        .and_then(|answer| answer.get("total").and_then(|total| total.as_u64()))
        .unwrap_or(0);
    Section {
        sid: sid.to_string(),
        server: server.to_string(),
        refusal: None,
        sessions,
        agents,
        attention_open,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Scratch(PathBuf);

    impl Scratch {
        fn new(name: &str) -> Scratch {
            let path =
                std::env::temp_dir().join(format!("fleet-tuios-{name}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir_all(&path).expect("the scratch directory");
            Scratch(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// A fake `tuios` that answers each verb from a file the test wrote,
    /// so the parse is proved against the measured shapes without a
    /// daemon. `$2` is the verb; a verb with no file answers exit 3, the
    /// daemon-not-running shape.
    fn fake_daemon(dir: &Path, answers: &[(&str, &str)]) -> PathBuf {
        let mut script = String::from("#!/bin/sh\nverb=\"$2\"\n");
        for (verb, body) in answers {
            script.push_str(&format!(
                "if [ \"$verb\" = \"{verb}\" ]; then\ncat <<'EOF'\n{body}\nEOF\nexit 0\nfi\n"
            ));
        }
        script.push_str("echo '{\"error\":{\"code\":\"transport\",\"message\":\"The TUIOS daemon is not running\"}}'\nexit 3\n");
        write_fake(dir, script)
    }

    /// A fake `tuios` whose `list-windows` answers per session: `$3` is the
    /// params JSON, and a rule matches only the call that names its
    /// session. The other verbs answer from `answers`. This is the shape
    /// the real daemon has — `list-windows` answers for the session it is
    /// given — and a fake that answered the same windows for every session
    /// is what let a fleet pane be "found" in a session holding a shell.
    fn fake_daemon_per_session(
        dir: &Path,
        answers: &[(&str, &str)],
        windows: &[(&str, &str)],
    ) -> PathBuf {
        let mut script = String::from("#!/bin/sh\nverb=\"$2\"\nparams=\"$3\"\n");
        script.push_str("if [ \"$verb\" = \"list-windows\" ]; then\ncase \"$params\" in\n");
        for (session, body) in windows {
            script.push_str(&format!(
                "  *'\"{session}\"'*) cat <<'EOF'\n{body}\nEOF\nexit 0;;\n"
            ));
        }
        script.push_str("esac\nfi\n");
        for (verb, body) in answers {
            script.push_str(&format!(
                "if [ \"$verb\" = \"{verb}\" ]; then\ncat <<'EOF'\n{body}\nEOF\nexit 0\nfi\n"
            ));
        }
        script.push_str("echo '{\"error\":{\"code\":\"transport\",\"message\":\"The TUIOS daemon is not running\"}}'\nexit 3\n");
        write_fake(dir, script)
    }

    fn write_fake(dir: &Path, script: String) -> PathBuf {
        let path = dir.join("tuios");
        std::fs::write(&path, script).expect("the fake binary");
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))
            .expect("the fake binary's mode");
        path
    }

    const SESSIONS: &str = r#"{"sessions":[{"name":"fleet-tui","window_count":1,"windows":[{"foreground_cmd":"fleet"}]},{"name":"work","window_count":2}],"type":"session_list"}"#;

    const WINDOWS_FLEET: &str = r#"{"total":1,"type":"window_list","windows":[{"agent_state":"none","cwd":"/Users/owner/Projects/fleet","display_name":"Terminal cb1fe908","foreground_cmd":"fleet","last_cmdline":"fleet tui","running_cmdline":"./target/debug/fleet","window_id":"cb1fe908"}]}"#;

    // The adversarial shell: its cwd is the repo and its history names
    // fleet, but the process in front of it is zsh at a prompt. Under the
    // live-command-line rule this is a shell, and the map says so.
    const WINDOWS_WORK: &str = r#"{"total":2,"type":"window_list","windows":[{"agent_state":"working","cwd":"/tmp","display_name":"api","foreground_cmd":"claude","running_cmdline":"claude --resume","window_id":"w1"},{"agent_state":"needs_input","cwd":"/Users/owner/Projects/fleet","display_name":"web","foreground_cmd":"-zsh","last_cmdline":"fleet tui","window_id":"w2"}]}"#;

    #[test]
    fn the_opt_in_is_per_instance_and_off_by_default() {
        let scratch = Scratch::new("optin");
        let home = scratch.path();
        // No file: no instance carries a section.
        assert!(section(home, "sid-a").is_none(), "no opt-in, no section");
        OptIn::enable(home, "sid-a", "").expect("the opt-in write");
        let opt_in = OptIn::load(home);
        assert!(opt_in.enabled_for("sid-a").is_some(), "the enabled sid");
        assert!(
            opt_in.enabled_for("sid-b").is_none(),
            "one instance's opt-in never enables another's"
        );
        OptIn::disable(home, "sid-a").expect("the opt-out write");
        assert!(
            OptIn::load(home).enabled_for("sid-a").is_none(),
            "the opt-out removes the row's section"
        );
        assert!(
            OptIn::enable(home, "  ", "").is_err(),
            "a section names the instance it reads for"
        );
    }

    #[test]
    fn a_daemon_that_is_not_running_is_named_and_never_an_empty_grid() {
        let scratch = Scratch::new("refusal");
        let home = scratch.path();
        OptIn::enable(home, "sid-a", "").expect("the opt-in write");
        let program = fake_daemon(scratch.path(), &[]);
        let section = section_with("sid-a", "", &program);
        assert_eq!(
            section.refusal.as_deref(),
            Some("tuios is not running on sid-a — The TUIOS daemon is not running"),
            "the refusal names the instance and the daemon's own words"
        );
        assert!(
            section.sessions.is_empty(),
            "an unreachable daemon renders no sessions, not a grid of none"
        );
        assert_eq!(section.agents.len(), 0);
    }

    #[test]
    fn a_missing_binary_is_named_as_not_installed_not_as_not_running() {
        let scratch = Scratch::new("missing");
        let home = scratch.path();
        OptIn::enable(home, "sid-a", "").expect("the opt-in write");
        let nowhere = scratch.path().join("no-such-tuios");
        let failure = Failure::NotInstalled("no `tuios` on PATH".to_string());
        let words = failure.refusal("sid-a");
        assert!(
            words.starts_with("tuios is not installed on sid-a"),
            "a machine with no binary says so: {words}"
        );
        assert!(
            call(&nowhere, "", "list-sessions", None).is_err(),
            "a binary that is not there is a failure, never an empty answer"
        );
    }

    #[test]
    fn the_agent_words_pass_through_verbatim() {
        let scratch = Scratch::new("words");
        let home = scratch.path();
        OptIn::enable(home, "sid-a", "").expect("the opt-in write");
        let program = fake_daemon(
            scratch.path(),
            &[
                ("list-sessions", SESSIONS),
                ("list-windows", WINDOWS_WORK),
                (
                    "list-agents",
                    r#"{"agents":[{"session":"work","name":"api","state":"working","foreground":"claude"}],"total":1}"#,
                ),
                ("list-attention", r#"{"counts":{"approval":0},"total":2}"#),
            ],
        );
        // The fake answers list-windows for every session, so the fleet
        // session reads the work session's windows here; the words are
        // what this test pins.
        let section = section_with("sid-a", "", &program);
        assert_eq!(section.refusal, None);
        assert_eq!(section.sessions.len(), 2);
        let work = &section.sessions[1];
        assert_eq!(work.name, "work");
        assert_eq!(
            work.agents.keys().cloned().collect::<Vec<_>>(),
            vec!["needs_input".to_string(), "working".to_string()],
            "the daemon's own state words, never mapped onto Fleet's"
        );
        assert_eq!(work.agents["working"], 1);
        assert_eq!(section.agents[0].state, "working");
        assert_eq!(section.attention_open, 2, "the daemon's own count");
    }

    #[test]
    fn a_fleet_pane_is_matched_by_the_live_command_line() {
        let scratch = Scratch::new("fleetpane");
        // The daemon answers per session, as the real one does: the fleet
        // session holds Fleet's TUI, the work session holds an agent and a
        // shell whose history names fleet. A fake that answered the same
        // windows for both sessions is the fixture bug this test caught.
        let program = fake_daemon_per_session(
            scratch.path(),
            &[
                ("list-sessions", SESSIONS),
                ("list-agents", r#"{"agents":[]}"#),
            ],
            &[("fleet-tui", WINDOWS_FLEET), ("work", WINDOWS_WORK)],
        );
        let section = section_with("sid-a", "", &program);
        let fleet = &section.sessions[0];
        assert_eq!(fleet.name, "fleet-tui");
        assert_eq!(fleet.fleet_panes.len(), 1, "the fleet pane is found");
        assert_eq!(fleet.fleet_panes[0].command, "fleet");
        assert_eq!(fleet.fleet_panes[0].cwd, "/Users/owner/Projects/fleet");
        assert_eq!(
            fleet.fleet_panes[0].window, "Terminal cb1fe908",
            "the pane is named by the daemon's own display name"
        );
        // The shell that merely ran `fleet tui` once is not a fleet pane:
        // its live process is zsh, whatever its history and cwd say.
        let work = &section.sessions[1];
        assert!(
            work.fleet_panes.is_empty(),
            "a shell whose history names fleet is not a fleet pane: {:?}",
            work.fleet_panes
        );
        assert_eq!(work.windows, 2, "both work windows were counted");
    }

    /// THE RULE, pinned as a table: the live command line is the only
    /// evidence. The three forms the plan names are fleet; a shell's
    /// history, its cwd, a look-alike name and a non-fleet command that
    /// merely mentions fleet are not.
    #[test]
    fn the_rule_is_the_live_command_line_and_nothing_else() {
        let pane = |window: serde_json::Value| names_fleet(&window).map(|pane| pane.command);
        for command in ["fleet", "./target/debug/fleet", "/opt/homebrew/bin/fleet"] {
            assert_eq!(
                pane(serde_json::json!({ "foreground_cmd": command })),
                Some(command.to_string()),
                "{command} is the fleet binary"
            );
        }
        // The running command line is the same evidence when the
        // foreground program is unnamed.
        assert_eq!(
            pane(serde_json::json!({ "running_cmdline": "./target/debug/fleet tui" })),
            Some("./target/debug/fleet tui".to_string()),
            "a running fleet command line is fleet"
        );
        // Not evidence: history, cwd, a look-alike, and a command that only
        // mentions fleet in its arguments.
        assert_eq!(
            pane(serde_json::json!({
                "foreground_cmd": "-zsh",
                "last_cmdline": "fleet tui",
                "cwd": "/Users/owner/Projects/fleet"
            })),
            None,
            "a shell whose history names fleet is a shell"
        );
        assert_eq!(
            pane(serde_json::json!({ "foreground_cmd": "fleet-tui" })),
            None,
            "a look-alike name is not the fleet binary"
        );
        assert_eq!(
            pane(serde_json::json!({ "running_cmdline": "grep fleet notes.txt" })),
            None,
            "a command that names fleet in its arguments does not run fleet"
        );
        assert_eq!(
            pane(serde_json::json!({ "cwd": "/Users/owner/Projects/fleet" })),
            None,
            "a directory is not a process"
        );
    }

    /// The document the card reads: the section serializes with the keys
    /// `app.js` names (`sessions[].agents`, `sessions[].fleet_panes`,
    /// `attention_open`), so a rename here without one there would paint an
    /// empty section instead of failing a build.
    #[test]
    fn the_section_serializes_as_the_card_reads_it() {
        let scratch = Scratch::new("document");
        let program = fake_daemon(
            scratch.path(),
            &[
                ("list-sessions", SESSIONS),
                ("list-windows", WINDOWS_FLEET),
                (
                    "list-agents",
                    r#"{"agents":[{"session":"fleet-tui","name":"Terminal cb1fe908","state":"none","foreground":"fleet"}]}"#,
                ),
                ("list-attention", r#"{"total":2}"#),
            ],
        );
        let section = section_with("sid-a", "", &program);
        let doc = serde_json::to_value(&section).expect("the section is a document");
        assert_eq!(doc["sid"], "sid-a");
        assert_eq!(doc["attention_open"], 2);
        assert!(
            doc.get("refusal").is_none(),
            "a section that landed carries no refusal key: {doc}"
        );
        let session = &doc["sessions"][0];
        assert_eq!(session["name"], "fleet-tui");
        assert_eq!(session["windows"], 1);
        assert_eq!(session["agents"]["none"], 1, "the AGENT words, verbatim");
        assert_eq!(session["fleet_panes"][0]["command"], "fleet");
        assert_eq!(
            session["fleet_panes"][0]["window"], "Terminal cb1fe908",
            "the pane is named by the daemon's display name"
        );
        assert_eq!(doc["agents"][0]["state"], "none");
    }

    #[test]
    fn a_verb_that_does_not_answer_is_a_named_probe_failure() {
        let scratch = Scratch::new("probe");
        let program = fake_daemon(
            scratch.path(),
            &[
                ("list-sessions", SESSIONS),
                (
                    "list-windows",
                    r#"{"error":{"code":"session_not_found","message":"session work not found"}}"#,
                ),
                ("list-agents", r#"{"agents":[]}"#),
            ],
        );
        // list-windows exits 0 with an error object only for the fake; the
        // real CLI exits 1. Either way the row names it.
        let section = section_with("sid-a", "", &program);
        assert_eq!(section.refusal, None, "the session list itself landed");
        let work = &section.sessions[1];
        let error = work.probe_error.as_deref().unwrap_or("");
        assert!(
            error.contains("session_not_found"),
            "the row names the verb's failure: {error}"
        );
        assert_eq!(work.windows, 2, "the daemon's declared window count stays");
    }
}
