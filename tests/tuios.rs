//! The tuios layer against the **real** daemon (the tuios armada plan).
//!
//! Every test here talks to the `tuios` the owner runs on this machine, over
//! its own control protocol, and **skips by name** where there is none: a
//! machine without tuios is not a failing fleet, and a test that passed
//! without measuring anything would be worse than a skip. The skip prints
//! the reason the layer itself would paint, so a green run that skipped
//! says so in its own words.
//!
//! What is pinned: the four read verbs answer the shapes the map parses
//! (`list-sessions`, `list-windows` per session, `list-agents`, and
//! `list-attention`); a dead daemon is a named refusal in one line and
//! never an empty grid; and the live specimen — Fleet's TUI in the
//! `fleet-tui` session — is discoverable by its live command line, which is
//! the plan's own evidence for the rule.

use std::path::PathBuf;

use fleet::tuios::{self, Failure};

/// The binary on this machine, or the named reason this test cannot run.
fn daemon() -> Option<PathBuf> {
    match tuios::binary() {
        Ok(program) => Some(program),
        Err(failure) => {
            eprintln!("tuios: skipped — {}", failure.refusal("this machine"));
            None
        }
    }
}

/// A call that must land, or the named skip. Only a missing daemon skips:
/// a daemon that answers with an error is a failure of this test, not of
/// the machine.
fn call_or_skip(
    program: &std::path::Path,
    verb: &str,
    params: Option<&str>,
) -> Option<serde_json::Value> {
    match tuios::call(program, "", verb, params) {
        Ok(answer) => Some(answer),
        Err(failure @ Failure::NotRunning(_)) | Err(failure @ Failure::NotInstalled(_)) => {
            eprintln!("tuios: skipped — {}", failure.refusal("this machine"));
            None
        }
        Err(failure) => panic!("{verb} did not land: {}", failure.refusal("this machine")),
    }
}

#[test]
fn the_four_read_verbs_answer_the_shapes_the_map_parses() {
    let Some(program) = daemon() else { return };
    let Some(sessions) = call_or_skip(&program, "list-sessions", None) else {
        return;
    };
    let rows = sessions
        .get("sessions")
        .and_then(|rows| rows.as_array())
        .expect("list-sessions answers a sessions array");
    let names: Vec<&str> = rows
        .iter()
        .filter_map(|row| row.get("name").and_then(|name| name.as_str()))
        .collect();
    if names.is_empty() {
        eprintln!(
            "tuios: skipped — the daemon holds no session on this machine, so there is \
             no window list to read; start one with `tuios new`"
        );
        return;
    }

    // list-windows answers for the session it is given, and carries the
    // AGENT column and the live command line the rule reads.
    let first = names[0];
    let Some(windows) = call_or_skip(
        &program,
        "list-windows",
        Some(&format!("{{\"session\":{}}}", serde_json::json!(first))),
    ) else {
        return;
    };
    let windows = windows
        .get("windows")
        .and_then(|windows| windows.as_array())
        .expect("list-windows answers a windows array");
    for window in windows {
        assert!(
            window
                .get("agent_state")
                .and_then(|state| state.as_str())
                .is_some(),
            "every window carries its AGENT state: {window}"
        );
        assert!(
            window.get("window_id").is_some() || window.get("id").is_some(),
            "every window carries an id: {window}"
        );
    }

    // The agents carry the same state words, for every session at once.
    let Some(agents) = call_or_skip(&program, "list-agents", Some("{\"all\":true}")) else {
        return;
    };
    let agents = agents
        .get("agents")
        .and_then(|agents| agents.as_array())
        .expect("list-agents answers an agents array");
    for agent in agents {
        assert!(
            agent
                .get("state")
                .and_then(|state| state.as_str())
                .is_some(),
            "every agent carries its state word: {agent}"
        );
    }

    // The Inbox's own total, which the section carries as a count.
    let Some(attention) = call_or_skip(&program, "list-attention", None) else {
        return;
    };
    assert!(
        attention
            .get("total")
            .and_then(|total| total.as_u64())
            .is_some(),
        "list-attention answers a total: {attention}"
    );
}

#[test]
fn a_dead_daemon_is_one_named_refusal_and_never_an_empty_grid() {
    let Some(program) = daemon() else { return };
    // A named server with no socket behind it is the plan's dead-socket
    // red-proof, measured on the real binary rather than faked: the CLI
    // exits 3 with a transport error. A server name is capped at 14
    // characters by the CLI itself, so the name is short on purpose.
    let dead = "fleet-dead";
    let failure = tuios::call(&program, dead, "list-sessions", None)
        .expect_err("a server with no daemon must not answer");
    match &failure {
        Failure::NotRunning(detail) => {
            assert!(
                detail.contains("not running"),
                "the daemon's own words name the transport: {detail}"
            );
        }
        other => panic!("a dead socket is NotRunning, not {other:?}"),
    }
    let words = failure.refusal("sid-dead");
    assert!(
        words.starts_with("tuios is not running on sid-dead"),
        "the refusal names the instance: {words}"
    );
    assert!(
        !words.contains('\n'),
        "a refusal is one line, because a row is one line: {words:?}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn the_armada_document_carries_the_section_only_where_the_owner_enabled_it() {
    let Some(_program) = daemon() else { return };
    // A home with one registered instance and no opt-in: the ordinary
    // install. The document carries the key as null (the card names the
    // absence), and nothing is spawned for a machine the owner did not
    // enable.
    let home = std::env::temp_dir().join(format!("fleet-tuios-doc-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).expect("the scratch home");
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|span| span.as_secs())
        .unwrap_or_default();
    fleet::armada::roster::register(&home, "i-1", "sid-live", "0.8.24", 2_000_000, now)
        .expect("the registration");

    let (bridge, _sources) = fleet::gui::bridge::Bridge::detached(
        home.clone(),
        "deepseek-flash".to_string(),
        Some("tuios-document".to_string()),
    );
    let before = bridge.armada().await;
    assert_eq!(
        before["instances"][0]["tuios"],
        serde_json::Value::Null,
        "an instance the owner has not enabled carries no section"
    );

    // Enabled: the same document now carries the daemon's own answer.
    fleet::tuios::OptIn::enable(&home, "sid-live", "").expect("the opt-in");
    let after = bridge.armada().await;
    let section = &after["instances"][0]["tuios"];
    if section.is_null() {
        eprintln!("tuios: skipped — the daemon did not answer this read");
        let _ = std::fs::remove_dir_all(&home);
        return;
    }
    assert!(
        section.get("refusal").is_none(),
        "the live daemon answered, so the section is not a refusal: {section}"
    );
    let sessions = section["sessions"].as_array().expect("a sessions array");
    assert!(
        !sessions.is_empty(),
        "the section carries the daemon's sessions: {section}"
    );
    assert!(
        sessions.iter().any(|session| session["name"] == "fleet-tui"
            && session["fleet_panes"]
                .as_array()
                .is_some_and(|panes| !panes.is_empty())),
        "the document names Fleet's TUI where the daemon reports it: {section}"
    );
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn the_live_specimen_is_discoverable_by_its_live_command_line() {
    let Some(program) = daemon() else { return };
    let Some(sessions) = call_or_skip(&program, "list-sessions", None) else {
        return;
    };
    let names: Vec<String> = sessions
        .get("sessions")
        .and_then(|rows| rows.as_array())
        .map(|rows| {
            rows.iter()
                .filter_map(|row| row.get("name").and_then(|name| name.as_str()))
                .map(|name| name.to_string())
                .collect()
        })
        .unwrap_or_default();

    // The section as the map reads it: the same call path the board takes.
    let section = tuios::section_with("this-machine", "", &program);
    assert_eq!(
        section.refusal, None,
        "the section read the live daemon: {:?}",
        section.refusal
    );
    assert_eq!(
        section.sessions.len(),
        names.len(),
        "the section carries every session the daemon listed"
    );

    // Every pane the section calls a fleet pane must really have the fleet
    // binary in front of it — the rule, measured on the live daemon rather
    // than on a fixture.
    let mut found = 0;
    for session in &section.sessions {
        for pane in &session.fleet_panes {
            found += 1;
            let program = pane.command.split_whitespace().next().unwrap_or("");
            let base = std::path::Path::new(program)
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_else(|| program.to_string());
            assert_eq!(
                base, "fleet",
                "a fleet pane's live command is the fleet binary: {pane:?}"
            );
            assert!(
                !pane.window.is_empty(),
                "the pane is named by the daemon's own display name: {pane:?}"
            );
        }
    }
    // The evidence, printed so a green run says what it measured.
    let panes: Vec<(String, String, String)> = section
        .sessions
        .iter()
        .flat_map(|session| {
            session.fleet_panes.iter().map(move |pane| {
                (
                    session.name.clone(),
                    pane.window.clone(),
                    pane.command.clone(),
                )
            })
        })
        .collect();
    eprintln!(
        "tuios: {} session(s) on the live daemon ({names:?}); {} fleet pane(s): {panes:?}",
        section.sessions.len(),
        panes.len()
    );
    if names.iter().any(|name| name == "fleet-tui") {
        assert!(
            found > 0,
            "the fleet-tui session exists, so the map must find Fleet's TUI in it; \
             sessions: {:?}",
            section
                .sessions
                .iter()
                .map(|session| (&session.name, &session.fleet_panes))
                .collect::<Vec<_>>()
        );
    } else {
        eprintln!(
            "tuios: skipped the live specimen — the daemon holds no `fleet-tui` \
             session on this machine (sessions: {names:?}); start one with \
             `tuios attach fleet-tui`"
        );
    }
}
