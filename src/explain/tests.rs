//! The explanation beat's own tests: the discipline, graded. The pure
//! layer is tested directly; the end-to-end paths run against the
//! scripted MCP lane, so nothing here spends a paid call — and the
//! socket-guard test is the port of the sibling's proof that no
//! synchronous path reaches this module.

use super::*;

/// The card without its ANSI escapes, so a test reads the text an
/// operator reads rather than the bytes the terminal draws.
fn plain(card: &str) -> String {
    let mut out = String::with_capacity(card.len());
    let mut escape = false;
    for character in card.chars() {
        match character {
            '\u{1b}' => escape = true,
            c if escape => {
                if c == 'm' {
                    escape = false;
                }
            }
            c => out.push(c),
        }
    }
    out
}

/// A scratch directory, unique per call.
fn scratch(tag: &str) -> PathBuf {
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let serial = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "fleet-explain-{tag}-{}-{serial}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// A verdict row in the stream, the shape the gate's writer lands: a
/// refused proposal with per-step evidence and the judged artifact's
/// sha.
fn refused_proposal_row(root: &Path, proposal: &str, sha: &str) {
    let stream = calib::Stream::open(root);
    stream.record(Row::Propose(calib::ProposeRow {
        ts: calib::stamp(),
        proposal: proposal.to_string(),
        target: "source".to_string(),
        target_name: "src/lib.rs".to_string(),
        basis: "inferred".to_string(),
        verdict: calib::VERDICT_REFUSED.to_string(),
        reason: "test failed: the_ranker_orders_a_corpus".to_string(),
        gate: vec![
            calib::GateStepRow { step: "fmt".to_string(), ok: true, detail: "ok".to_string() },
            calib::GateStepRow {
                step: "test".to_string(),
                ok: false,
                detail: "error: test failed: the_ranker_orders_a_corpus".to_string(),
            },
        ],
        input_sha256: Some(sha.to_string()),
    }));
    // The last handle's drop drains the queue before this returns.
    drop(stream);
}

/// The proposal file the artifact resolves from: the change the gate
/// judged, filed in the home's proposals store.
fn proposal_file(home: &Path, name: &str, change: &str) {
    let text = crate::improve::render_proposal(
        name,
        crate::improve::Target::Source,
        "src/lib.rs",
        crate::improve::Basis::Inferred,
        "the ranking assertion failed",
        "the corpus's ranking assertion holds",
        "the ranker orders a corpus again",
        change,
        &[],
    );
    let dir = crate::improve::proposals_dir(home);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join(format!("{name}.md")), text).unwrap();
}

/// The change every end-to-end test explains: a patch whose content is
/// the judged artifact.
const CHANGE: &str = r#"{"op": "patch", "path": "src/lib.rs", "content": "pub fn one() -> u32 { 2 }\n"}"#;

/// The sha of [`CHANGE`]'s staged content, the join key the row carries.
fn change_sha() -> String {
    crate::improve::parse_change(&serde_json::from_str::<serde_json::Value>(CHANGE).unwrap())
        .expect("the change parses")
        .artifact_sha()
}

/// The prose the scripted lane answers with.
const ANSWER: &str = "The `test` step refused the change: the ranking assertion \
`the_ranker_orders_a_corpus` no longer holds because the patched comparator sorts on the \
score's absolute value rather than its rank. Restore the rank comparison, or update the \
assertion to the ordering the patch intends.";

/// A scripted MCP lane: the `mock-mcp` child over a script whose one
/// `tools/call` answers with `reply`. `None` when the binary cannot be
/// resolved beside the executable — which the calling test names
/// rather than guessing at.
fn scripted_lane(reply: &str) -> Option<ServerSpec> {
    let bin = crate::eval::mock_mcp_binary()?;
    let scratch = scratch("lane-script");
    let entries = vec![
        serde_json::json!({ "reply": {
            "jsonrpc": "2.0", "id": "$echo",
            "result": {
                "protocolVersion": "2024-11-05",
                "capabilities": { "tools": { "listChanged": false } },
                "serverInfo": { "name": "blend1", "version": "0.1.0" },
            },
        } }),
        serde_json::json!({ "reply": {
            "jsonrpc": "2.0", "id": "$echo",
            "result": { "tools": [{ "name": "ask", "description": "one blended ask",
                                    "inputSchema": { "type": "object" } }] },
        } }),
        serde_json::json!({ "reply": {
            "jsonrpc": "2.0", "id": "$echo",
            "result": { "content": [{ "type": "text", "text": reply }], "isError": false },
        } }),
    ];
    let script = entries.iter().map(|entry| format!("{entry}\n")).collect::<String>();
    std::fs::write(scratch.join("script.jsonl"), script).unwrap();
    Some(ServerSpec {
        program: bin.to_string_lossy().into_owned(),
        args: vec![scratch.join("script.jsonl").to_string_lossy().into_owned()],
        env: Vec::new(),
        passthrough: Vec::new(),
        cwd: None,
    })
}

/// A BlendResult-shaped lane answer carrying `answer` as its text.
fn lane_reply(answer: &str, cost: f64) -> String {
    serde_json::json!({
        "prompt_sha256": "9f2c1d4b6a8e0357",
        "path": "synthesized",
        "route": { "status": "ok", "provider": "liquid", "needs_generation": 0.978,
                   "answer_shape": "synthesized", "latency_ms": 900 },
        "generated": { "status": "ok", "model": "lfm2.5-24b-a2b", "text": answer,
                       "cost_usd": cost },
        "validation": { "status": "ok", "provider": "glide", "passed": true,
                        "outcome": "passed", "number_check": "ok",
                        "answers_the_question": true, "contradicts_facts": false },
        "final_answer": answer,
        "total_latency_ms": 15_400,
    })
    .to_string()
}

/// One explain call over the scripted lane.
async fn explain_over(
    home: &Path,
    root: &Path,
    selector: Selector,
    dry_run: bool,
) -> ExplainOutcome {
    let lane = scripted_lane(&lane_reply(ANSWER, 0.0018)).expect("the scripted lane");
    explain(ExplainRequest { root, home, selector, dry_run, lane: LaneSource::Scripted(lane) }).await
}

// --- the discipline's pure layer ---

#[test]
fn the_budget_is_a_named_constant_the_config_can_only_lower() {
    // The env-mutating test serializes on this: `cargo test` runs a
    // module's tests on threads, and a `set_var` in one must not be
    // visible to another mid-flight.
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    struct EnvVar {
        previous: Option<std::ffi::OsString>,
    }
    impl EnvVar {
        fn set(value: &str) -> EnvVar {
            let previous = std::env::var_os(EXPLAIN_BUDGET_ENV);
            std::env::set_var(EXPLAIN_BUDGET_ENV, value);
            EnvVar { previous }
        }
    }
    impl Drop for EnvVar {
        fn drop(&mut self) {
            match &self.previous {
                Some(value) => std::env::set_var(EXPLAIN_BUDGET_ENV, value),
                None => std::env::remove_var(EXPLAIN_BUDGET_ENV),
            }
        }
    }

    let _guard = ENV_LOCK.lock().unwrap_or_else(|poison| poison.into_inner());
    std::env::remove_var(EXPLAIN_BUDGET_ENV);
    assert_eq!(budget_usd_day(), EXPLAIN_BUDGET_USD_DAY);
    // The cap's value is the stated one: roughly 275 synthesized asks
    // at the sibling's measured per-ask spend.
    assert_eq!(EXPLAIN_BUDGET_USD_DAY, 0.50);
    assert!((EXPLAIN_BUDGET_USD_DAY / EXPLAIN_ESTIMATED_ASK_USD - 277.7).abs() < 1.0);
    // The owner's config lowers it.
    let _lower = EnvVar::set("0.05");
    assert_eq!(budget_usd_day(), 0.05);
    // And can never raise it: a value above the constant, a junk value,
    // and a negative one all read as the constant.
    let _higher = EnvVar::set("99.0");
    assert_eq!(budget_usd_day(), EXPLAIN_BUDGET_USD_DAY);
    let _junk = EnvVar::set("not-a-number");
    assert_eq!(budget_usd_day(), EXPLAIN_BUDGET_USD_DAY);
    let _negative = EnvVar::set("-1");
    assert_eq!(budget_usd_day(), EXPLAIN_BUDGET_USD_DAY);
}

#[test]
fn a_shared_sha_prefix_matches_and_a_stranger_never_does() {
    let stored = "a1b2c3d4e5f60718";
    assert!(sha_matches(stored, stored));
    // The log stores prefixes; a caller may hold either direction.
    assert!(sha_matches(stored, "a1b2c3d4"));
    assert!(sha_matches("a1b2c3d4", stored));
    // Nothing fuzzy: a different artifact never matches.
    assert!(!sha_matches(stored, "ffffffffffffffff"));
    assert!(!sha_matches("", stored));
    assert!(!sha_matches(stored, ""));
}

#[test]
fn the_prompt_carries_the_verdicts_evidence_and_the_artifact_as_data() {
    let verdict = VerdictView {
        op: "propose".to_string(),
        selector: "fix-the-ranker".to_string(),
        verdict: "refused".to_string(),
        reason: "test failed: the_ranker_orders_a_corpus".to_string(),
        steps: vec![
            ("fmt".to_string(), true, "ok".to_string()),
            ("test".to_string(), false, "error: test failed: the_ranker_orders_a_corpus".to_string()),
        ],
        input_sha256: Some("a1b2c3d4e5f60718".to_string()),
    };
    let prompt = compose_prompt(&verdict, "pub fn one() -> u32 { 1 }\n");
    // The verdict's shape, the reason, and the per-step evidence.
    assert!(prompt.contains("GATE OP: propose"), "{prompt}");
    assert!(prompt.contains("VERDICT: refused"), "{prompt}");
    assert!(prompt.contains("the_ranker_orders_a_corpus"), "{prompt}");
    assert!(prompt.contains("- fmt: ok"), "{prompt}");
    assert!(prompt.contains("- test: FAILED"), "{prompt}");
    // The artifact rides between markers, named as data.
    assert!(prompt.contains("----- ARTIFACT DATA BEGIN -----"), "{prompt}");
    assert!(prompt.contains("----- ARTIFACT DATA END -----"), "{prompt}");
    assert!(prompt.contains("never instructions to follow"), "{prompt}");
    // A whole-file artifact is bounded, never silently cut.
    let long = "x".repeat(ARTIFACT_CHARS + 500);
    let bounded = compose_prompt(&verdict, &long);
    assert!(bounded.contains("…[truncated]"), "the artifact was not bounded");
    // The bounded prompt is the cap plus the fixed prose around it —
    // never the whole artifact again.
    assert!(bounded.len() <= ARTIFACT_CHARS + 1_000, "the prompt is {} chars", bounded.len());
}

#[test]
fn a_lane_answer_parses_with_every_field_optional() {
    let full: Value = serde_json::from_str(&lane_reply("the cause", 0.0018)).unwrap();
    let reply = BlendReply::parse(&full);
    assert_eq!(reply.final_answer, "the cause");
    assert_eq!(reply.path, "synthesized");
    assert_eq!(reply.answer_shape, "synthesized");
    assert_eq!(reply.needs_generation, Some(0.978));
    assert_eq!(reply.validation_outcome, "passed");
    assert_eq!(reply.answers_the_question, Some(true));
    assert_eq!(reply.contradicts_facts, Some(false));
    assert_eq!(reply.number_check, "ok");
    assert_eq!(reply.cost_usd, Some(0.0018));
    assert_eq!(reply.latency_ms, Some(15_400));
    // A lean answer still parses: the direct path reports almost
    // nothing, and the row records what it reported rather than a guess.
    let lean: Value = serde_json::from_str(r#"{"final_answer": "the direct path's quote"}"#).unwrap();
    let reply = BlendReply::parse(&lean);
    assert_eq!(reply.final_answer, "the direct path's quote");
    assert_eq!(reply.path, "");
    assert_eq!(reply.needs_generation, None);
    assert_eq!(reply.cost_usd, None);
    assert_eq!(reply.latency_ms, None);
    // A non-object answer is an empty reply, never a panic.
    assert_eq!(BlendReply::parse(&Value::Null).final_answer, "");
}

// --- the batch-only proof ---

#[test]
fn no_synchronous_path_references_the_explanation_beat() {
    // The socket-guard test, in the sibling's shape: the synchronous
    // paths — the doctor's boot checks, the recall path, the tool loop,
    // the session log, the calibration stream's writers and readers —
    // never name this module. A beat that spends a lane ask must be
    // unreachable from a path that runs on every turn.
    for (name, source) in [
        ("doctor.rs", include_str!("../doctor.rs")),
        ("memory.rs", include_str!("../memory.rs")),
        ("agent.rs", include_str!("../agent.rs")),
        ("tools.rs", include_str!("../tools.rs")),
        ("session.rs", include_str!("../session.rs")),
        ("calib.rs", include_str!("../calib.rs")),
        ("route.rs", include_str!("../route.rs")),
        ("compaction.rs", include_str!("../compaction.rs")),
        ("cron.rs", include_str!("../cron.rs")),
        ("improve.rs", include_str!("../improve.rs")),
        ("boards.rs", include_str!("../boards.rs")),
    ] {
        assert!(!source.contains("explain::"), "{name} names the explanation beat");
        assert!(!source.contains("crate::explain"), "{name} names the explanation beat");
        assert!(!source.contains("EXPLAIN_LANE"), "{name} names the explanation beat");
    }
    // The beat's own row is the only new op, and no verdict reads it:
    // the ops that grade a run are the ones that were there before.
    assert!(calib::OPS.contains(&calib::OP_EXPLAIN));
    assert_eq!(calib::OP_EXPLAIN, "explain");
}

#[test]
fn an_explain_row_changes_no_verdict() {
    // The functional half of eval-only: an `explain` row in the stream
    // is invisible to every verdict reader. A day that carries one
    // demotes no provider, and the verdict rows beside it are exactly
    // as they were written.
    let root = scratch("eval-only");
    refused_proposal_row(&root, "fix-the-ranker", &change_sha());
    let stream = calib::Stream::open(&root);
    stream.record(Row::Explain(ExplainRow {
        ts: calib::stamp(),
        eval_only: true,
        explained_op: "propose".to_string(),
        selector: "fix-the-ranker".to_string(),
        input_sha256: change_sha(),
        explanation_sha256: "5a6b7c8d9e0f1123".to_string(),
        explanation_chars: 300,
        route: ExplainRouteRow { needs_generation: None, path: "direct".to_string(), answer_shape: String::new() },
        validation: ExplainValidationRow {
            outcome: "unreported".to_string(),
            answers_the_question: None,
            contradicts_facts: None,
            number_check: String::new(),
        },
        cost_usd: None,
        latency_ms: None,
        lane: EXPLAIN_LANE.to_string(),
    }));
    drop(stream);
    let day = calib::today();
    let rows = calib::read_day(&root, &day, 64);
    // The verdict rows are untouched, and the demotion reader — which
    // grades a day's route rows — sees nothing to demote.
    let propose: Vec<&ReadRow> = rows.iter().filter(|row| row.op == calib::OP_PROPOSE).collect();
    assert_eq!(propose.len(), 1);
    assert_eq!(propose[0].verdict, calib::VERDICT_REFUSED);
    assert_eq!(propose[0].reason, "test failed: the_ranker_orders_a_corpus");
    assert_eq!(propose[0].gate.len(), 2);
    assert!(!propose[0].gate[1].ok);
    assert!(calib::demoted_providers(&rows, calib::DEMOTION_THRESHOLD).is_empty());
    // The explanation row is the only one of its op, and it is marked.
    let explained: Vec<&ReadRow> = rows.iter().filter(|row| row.op == calib::OP_EXPLAIN).collect();
    assert_eq!(explained.len(), 1);
    assert!(explained[0].eval_only);
    let _ = std::fs::remove_dir_all(&root);
}

// --- the beat, end to end over the scripted lane ---

#[tokio::test]
async fn a_refused_verdict_is_explained_beside_itself_with_the_text_recoverable() {
    let home = scratch("explained-home");
    let root = scratch("explained-calib");
    let sha = change_sha();
    proposal_file(&home, "fix-the-ranker", CHANGE);
    refused_proposal_row(&root, "fix-the-ranker", &sha);
    let before = recorded_verdict(&root, &Selector::Proposal("fix-the-ranker".to_string()));
    let outcome = explain_over(&home, &root, Selector::Proposal("fix-the-ranker".to_string()), false).await;

    assert!(outcome.refusal.is_none(), "{:?}", outcome.refusal);
    let row = outcome.row.clone().expect("the row landed");
    // The verdict stands exactly as recorded.
    assert_eq!(
        recorded_verdict(&root, &Selector::Proposal("fix-the-ranker".to_string())),
        before
    );
    // The row is redacted and keyed.
    assert!(row.eval_only);
    assert_eq!(row.input_sha256, sha);
    assert_eq!(row.explained_op, "propose");
    assert_eq!(row.selector, "fix-the-ranker");
    assert_eq!(row.explanation_chars, ANSWER.chars().count());
    let text = serde_json::to_string(&row).unwrap();
    for key in ["prompt", "answer", "final_answer", "text", "artifact"] {
        assert!(!text.contains(&format!("\"{key}\"")), "the row carries `{key}`: {text}");
    }
    assert!(!text.contains(ANSWER), "the row carries the answer text: {text}");
    // The text is recoverable by its sha, at the documented path.
    let path = explanation_path(&home, &sha);
    assert_eq!(path, explanations_dir(&home).join(format!("{sha}.md")));
    let recovered = std::fs::read_to_string(&path).unwrap();
    assert_eq!(recovered, ANSWER);
    assert_eq!(calib::hash16(&recovered), row.explanation_sha256);
    // The ask's telemetry rode the row, and the day's spend is recorded.
    assert_eq!(row.route.path, "synthesized");
    assert_eq!(row.validation.outcome, "passed");
    assert_eq!(row.cost_usd, Some(0.0018));
    assert_eq!(explain_rows(&root).len(), 1);
    assert!((day_spend(&root, &calib::today()) - 0.0018).abs() < 1e-9);
    // The card carries the verdict, the prose, the cost, and the
    // eval-only line — and it is a card, not a dump.
    let text = plain(&outcome.card);
    assert!(text.contains("verdict refused"), "{text}");
    assert!(text.contains("test FAILED"), "{text}");
    assert!(text.contains("the_ranker_orders_a_corpus"), "{text}");
    assert!(text.contains("$0.0018"), "{text}");
    assert!(text.contains("eval-only"), "{text}");
    assert!(text.contains("recoverable by sha"), "{text}");
    assert!(outcome.card.contains('\u{250c}') && outcome.card.contains('\u{2514}'), "{}", outcome.card);
    // Every drawn line fits the house width, so nothing clips at 80
    // columns: the escapes are the terminal's, not the layout's.
    for line in outcome.card.lines() {
        let width = plain(line).chars().count();
        assert!(width <= CARD_WIDTH, "a card line is {width} wide: {}", plain(line));
    }
    // The headless card and the TUI surface agree: the TUI reads the
    // file the row indexes and re-renders it, so the card it opens is
    // the same card this call printed.
    let latest = latest_card(&root, &home).expect("the latest explanation reads back");
    let latest_text = plain(&latest);
    assert!(latest_text.contains("verdict refused"), "{latest_text}");
    // The prose rides wrapped, so the check names its opening rather
    // than the whole paragraph; the equality below is the real proof.
    assert!(latest_text.contains("The `test` step refused the change"), "{latest_text}");
    assert!(latest_text.contains("$0.0018"), "{latest_text}");
    assert_eq!(
        plain(&latest).lines().map(str::to_string).collect::<Vec<_>>(),
        plain(&outcome.card).lines().map(str::to_string).collect::<Vec<_>>(),
        "the headless card and the TUI surface disagree"
    );
    let _ = std::fs::remove_dir_all(&home);
    let _ = std::fs::remove_dir_all(&root);
}

#[tokio::test]
async fn a_dry_run_spends_nothing_and_shows_what_would_be_asked() {
    let home = scratch("dry-home");
    let root = scratch("dry-calib");
    let sha = change_sha();
    proposal_file(&home, "fix-the-ranker", CHANGE);
    refused_proposal_row(&root, "fix-the-ranker", &sha);
    let outcome = explain_over(&home, &root, Selector::Proposal("fix-the-ranker".to_string()), true).await;

    assert!(outcome.refusal.is_none(), "{:?}", outcome.refusal);
    // No ask, no row, no file.
    assert!(outcome.row.is_none());
    assert!(outcome.explanation_path.is_none());
    assert!(explain_rows(&root).is_empty());
    assert!(!explanation_path(&home, &sha).exists());
    assert!((day_spend(&root, &calib::today())).abs() < 1e-9);
    // The card shows what would be asked.
    let prompt = outcome.dry_run_prompt.expect("the prompt is rehearsed");
    assert!(prompt.contains("GATE OP: propose"));
    assert!(prompt.contains("ARTIFACT DATA BEGIN"));
    assert!(outcome.card.contains("dry run"), "{}", outcome.card);
    assert!(outcome.card.contains("no ask, no row, no file"), "{}", outcome.card);
    assert!(outcome.card.contains("budget"), "{}", outcome.card);
    let _ = std::fs::remove_dir_all(&home);
    let _ = std::fs::remove_dir_all(&root);
}

#[tokio::test]
async fn a_drifted_artifact_is_refused_by_name() {
    // The proposal file's change is not the change the gate judged: the
    // row's sha and the artifact's sha disagree, and the beat refuses
    // rather than explaining prose against the wrong input.
    let home = scratch("drift-home");
    let root = scratch("drift-calib");
    proposal_file(&home, "fix-the-ranker", CHANGE);
    refused_proposal_row(&root, "fix-the-ranker", "ffffffffffffffff");
    let outcome = explain_over(&home, &root, Selector::Proposal("fix-the-ranker".to_string()), false).await;

    let refusal = outcome.refusal.clone().expect("a named refusal");
    assert!(refusal.contains("refusing to explain a drifted artifact"), "{refusal}");
    assert!(refusal.contains(&change_sha()), "{refusal}");
    assert!(refusal.contains("ffffffffffffffff"), "{refusal}");
    assert!(outcome.row.is_none());
    assert!(!explanations_dir(&home).join(format!("{}.md", change_sha())).exists());
    assert!(explain_rows(&root).is_empty());
    // The verdict stands, and the card says so.
    assert!(plain(&outcome.card).contains("the verdict stands exactly as recorded"), "{}", outcome.card);
    let _ = std::fs::remove_dir_all(&home);
    let _ = std::fs::remove_dir_all(&root);
}

#[tokio::test]
async fn an_unknown_selector_is_a_named_refusal() {
    let home = scratch("unknown-home");
    let root = scratch("unknown-calib");
    // No rows at all, and a proposal that names nothing the stream holds.
    let outcome = explain_over(&home, &root, Selector::Proposal("never-filed".to_string()), false).await;
    let refusal = outcome.refusal.clone().expect("a named refusal");
    assert!(refusal.contains("never-filed"), "{refusal}");
    assert!(outcome.row.is_none());
    assert!(plain(&outcome.card).contains("refused"), "{}", outcome.card);
    let _ = std::fs::remove_dir_all(&home);
    let _ = std::fs::remove_dir_all(&root);
}

#[tokio::test]
async fn a_lane_that_cannot_answer_fails_open_naming_its_reason() {
    // The lane's own fail-open: a spec whose program does not exist is a
    // named refusal, the verdict stands, and no explanation is
    // fabricated. Unconfigured, down, and past its bound all land here.
    let home = scratch("open-home");
    let root = scratch("open-calib");
    let sha = change_sha();
    proposal_file(&home, "fix-the-ranker", CHANGE);
    refused_proposal_row(&root, "fix-the-ranker", &sha);
    let lane = LaneSource::Scripted(ServerSpec {
        program: "/nonexistent/fleet-lane".to_string(),
        args: Vec::new(),
        env: Vec::new(),
        passthrough: Vec::new(),
        cwd: None,
    });
    let outcome = explain(ExplainRequest {
        root: &root,
        home: &home,
        selector: Selector::Proposal("fix-the-ranker".to_string()),
        dry_run: false,
        lane,
    })
    .await;
    let refusal = outcome.refusal.clone().expect("a named refusal");
    assert!(refusal.contains("could not start"), "{refusal}");
    assert!(outcome.row.is_none());
    assert!(!explanations_dir(&home).join(format!("{sha}.md")).exists());
    assert!(explain_rows(&root).is_empty());
    assert!(outcome.card.contains("never a fabricated explanation") || outcome.card.contains("the verdict stands"), "{}", outcome.card);
    let _ = std::fs::remove_dir_all(&home);
    let _ = std::fs::remove_dir_all(&root);
}

#[tokio::test]
async fn an_over_budget_day_is_refused_naming_the_spend() {
    // The cap, reached: the day's recorded spend plus one reserved ask
    // is over the budget, and the refusal names both numbers.
    let home = scratch("budget-home");
    let root = scratch("budget-calib");
    let sha = change_sha();
    proposal_file(&home, "fix-the-ranker", CHANGE);
    refused_proposal_row(&root, "fix-the-ranker", &sha);
    let stream = calib::Stream::open(&root);
    stream.record(Row::Explain(ExplainRow {
        ts: calib::stamp(),
        eval_only: true,
        explained_op: "eval_corpus".to_string(),
        selector: "conversation-fold".to_string(),
        input_sha256: "0f1e2d3c4b5a6978".to_string(),
        explanation_sha256: "5a6b7c8d9e0f1123".to_string(),
        explanation_chars: 400,
        route: ExplainRouteRow { needs_generation: None, path: "synthesized".to_string(), answer_shape: String::new() },
        validation: ExplainValidationRow {
            outcome: "passed".to_string(),
            answers_the_question: None,
            contradicts_facts: None,
            number_check: String::new(),
        },
        cost_usd: Some(EXPLAIN_BUDGET_USD_DAY),
        latency_ms: Some(1_000),
        lane: EXPLAIN_LANE.to_string(),
    }));
    drop(stream);
    let outcome = explain_over(&home, &root, Selector::Proposal("fix-the-ranker".to_string()), false).await;

    let refusal = outcome.refusal.clone().expect("a named refusal");
    assert!(refusal.contains("daily budget"), "{refusal}");
    assert!(refusal.contains(&format!("${EXPLAIN_BUDGET_USD_DAY:.2}")), "{refusal}");
    assert!(outcome.row.is_none());
    // A dry run still rehearses: it spends nothing, so the budget never
    // refuses it.
    let dry = explain_over(&home, &root, Selector::Proposal("fix-the-ranker".to_string()), true).await;
    assert!(dry.refusal.is_none(), "{:?}", dry.refusal);
    assert!(dry.row.is_none());
    let _ = std::fs::remove_dir_all(&home);
    let _ = std::fs::remove_dir_all(&root);
}

