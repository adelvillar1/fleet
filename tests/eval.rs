//! The eval corpus end to end: the fixed, graded suite runs headless
//! against the mock, every scenario passes on the current code, one
//! `eval_corpus` row lands and is queryable, and a deliberately broken
//! scenario injected through the seam fails with its name. These are the
//! ACs the corpus is the contract for (plan ACs 3 and the improve-gate
//! half of 4); the improve-gate refusal itself is driven through the
//! stand-in seam in `src/improve.rs`'s own tests.
//!
//! No network beyond loopback; the mock server runs as a child process
//! (`CARGO_BIN_EXE_mock-server`), and every scenario drives a plain
//! single-endpoint client, so no run can reach the live API.

use std::path::Path;

use fleet::calib;
use fleet::eval::{self, EvalReport, Scenario, ScenarioFamily};

/// A scratch root, unique per call.
fn scratch(tag: &str) -> std::path::PathBuf {
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let serial = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "fleet-eval-it-{tag}-{}-{}-{serial}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn mock_bin() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_BIN_EXE_mock-server"))
}

#[tokio::test]
async fn the_corpus_runs_green_and_lands_a_queryable_row() {
    let root = scratch("corpus-green");
    let report = eval::run(&root, &mock_bin()).await;

    // Every scenario in the fixed corpus passes on the current code, and
    // the run is deterministic: twenty-one scenarios, score 1.0, no
    // regression.
    assert!(
        report.green(),
        "the corpus regressed: {:?}",
        report.failed()
    );
    assert_eq!(report.scenarios.len(), 21);
    assert_eq!(report.score(), 1.0);

    // The three fork families plus the three everyday shapes are all here.
    let families: Vec<&str> = report
        .scenarios
        .iter()
        .map(|scenario| scenario.family.as_str())
        .collect();
    for family in [
        "session-corpus",
        "agent-continuation",
        "conversation-fold",
        "tool-loop",
        "subagent",
        "memory",
        "grounded-lane",
        "explain",
        "calibration",
        "delegation",
        "nomenclature",
        "improve",
    ] {
        assert!(
            families.contains(&family),
            "the corpus is missing {family}: {families:?}"
        );
    }

    // One `eval_corpus` row landed and reads back with the per-scenario
    // verdicts and the aggregate score — the corpus's scores are queryable.
    let row = eval::last_row(&root).expect("the eval row landed");
    assert_eq!(row.op, calib::OP_EVAL_CORPUS);
    assert_eq!(row.passed, 21);
    assert_eq!(row.failed, 0);
    assert_eq!(row.score, 1.0);
    assert_eq!(row.scenarios.len(), 21);
    assert!(row.scenarios.iter().all(|scenario| scenario.passed));

    // The judged artifact's identity rides the row: the corpus's own
    // fixed definition, hashed — the key an explanation joins on.
    assert_eq!(row.input_sha256, eval::corpus_identity());

    std::fs::remove_dir_all(&root).ok();
}

#[tokio::test]
async fn a_broken_scenario_injected_through_the_seam_fails_with_its_name() {
    // The seam: a scenario that always fails, pushed beside the real
    // corpus. Running the whole thing grades the broken one failed and
    // names it, while the real scenarios still pass — which is exactly
    // what the improve gate reads a regression from.
    let mut scenarios = eval::corpus();
    scenarios.push(Scenario::always_fails(
        "injected-break",
        ScenarioFamily::ToolLoop,
        "the seam broke it",
    ));
    let mut outcomes = Vec::new();
    for scenario in &scenarios {
        outcomes.push(scenario.run(Path::new(&mock_bin())).await);
    }
    let report = EvalReport::from(outcomes);

    assert!(!report.green());
    assert_eq!(report.regressions(), vec!["injected-break".to_string()]);
    let broken = report
        .scenarios
        .iter()
        .find(|scenario| scenario.scenario == "injected-break")
        .unwrap();
    assert!(!broken.passed);
    assert_eq!(broken.failure.as_deref(), Some("the seam broke it"));
    // The real scenarios beside it still passed.
    assert_eq!(report.passed(), 21);
    assert_eq!(report.failed().len(), 1);
}

/// KI-477's pin: a `mock-server` exits on its stdin pipe's EOF, so the
/// pipe closing *is* its parent's death.
///
/// The harnesses reap their child from `Drop`, which covers every exit
/// they control. It cannot cover the ones they do not: a test binary the
/// runner kills, or the grandchild of a `fleet eval` whose own bound
/// expired (`src/eval.rs`'s `run_eval_subprocess` kills only its direct
/// child). On macOS a killed parent's children are reparented to launchd
/// and live on, each holding a listening socket — measured, 80 of them,
/// the oldest at 8h57m.
///
/// The mechanism the mock carries instead: every harness gives it a
/// stdin **pipe**, and the mock exits when that pipe reaches EOF. The
/// kernel closes the pipe when the last holder dies, however it dies, so
/// the child goes with its parent. This test drives the half that is
/// ours to drive — bind, serve, then close the write end and require the
/// child gone — because a fix for a leak that only manifests under a kill
/// is unproven by a suite that never kills.
#[test]
fn a_mock_server_exits_when_the_parent_holding_its_stdin_dies() {
    use std::process::{Command, Stdio};

    let root = scratch("eof-orphan");
    let script = root.join("script.jsonl");
    let port_path = root.join("port");
    std::fs::write(&script, "{\"status\":\"200 OK\",\"body\":\"{}\"}\n").unwrap();

    // Exactly the harness's spawn shape: stdin is a pipe this process
    // holds, and the port file is the readiness signal.
    let mut child = Command::new(mock_bin())
        .arg(&script)
        .arg(root.join("requests.jsonl"))
        .arg(&port_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("the mock spawns");

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        if let Ok(text) = std::fs::read_to_string(&port_path) {
            if let Ok(port) = text.trim().parse::<u16>() {
                assert!(port > 0, "the mock bound a real port");
                break;
            }
        }
        if child
            .try_wait()
            .map(|status| status.is_some())
            .unwrap_or(true)
        {
            panic!("the mock exited before serving");
        }
        assert!(
            std::time::Instant::now() < deadline,
            "the mock never wrote its port"
        );
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    // It is serving, not merely bound.
    assert!(
        child.try_wait().unwrap().is_none(),
        "the mock is alive once it has a port"
    );

    // The parent lets go — the shape a SIGKILL produces, since the kernel
    // closes the same write end either way.
    drop(child.stdin.take());
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if std::time::Instant::now() < deadline => {
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            Ok(None) => {
                panic!("the mock outlived the close of its stdin pipe — it leaked, which is KI-477")
            }
            Err(error) => panic!("the mock could not be waited on: {error}"),
        }
    }
    let _ = std::fs::remove_dir_all(&root);
}
