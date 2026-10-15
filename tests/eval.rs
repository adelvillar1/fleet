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
    // the run is deterministic: twelve scenarios, score 1.0, no
    // regression.
    assert!(report.green(), "the corpus regressed: {:?}", report.failed());
    assert_eq!(report.scenarios.len(), 12);
    assert_eq!(report.score(), 1.0);

    // The three fork families plus the three everyday shapes are all here.
    let families: Vec<&str> =
        report.scenarios.iter().map(|scenario| scenario.family.as_str()).collect();
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
    ] {
        assert!(families.contains(&family), "the corpus is missing {family}: {families:?}");
    }

    // One `eval_corpus` row landed and reads back with the per-scenario
    // verdicts and the aggregate score — the corpus's scores are queryable.
    let row = eval::last_row(&root).expect("the eval row landed");
    assert_eq!(row.op, calib::OP_EVAL_CORPUS);
    assert_eq!(row.passed, 12);
    assert_eq!(row.failed, 0);
    assert_eq!(row.score, 1.0);
    assert_eq!(row.scenarios.len(), 12);
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
    let broken =
        report.scenarios.iter().find(|scenario| scenario.scenario == "injected-break").unwrap();
    assert!(!broken.passed);
    assert_eq!(broken.failure.as_deref(), Some("the seam broke it"));
    // The real scenarios beside it still passed.
    assert_eq!(report.passed(), 12);
    assert_eq!(report.failed().len(), 1);
}
