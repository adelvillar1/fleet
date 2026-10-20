//! The delegation beat's own tests: the discipline, graded. The pure
//! layer is tested directly; the end-to-end paths run against the
//! scripted MCP lane, so nothing here spends a paid call — and the
//! socket-guard test is the port of the sibling's proof that no
//! synchronous path reaches this module, with `api.rs` and `mcp.rs` in
//! the scanned set the sibling's proof omitted.

use super::*;

use std::path::PathBuf;

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

/// The section as one run of characters with the decoration removed:
/// the ANSI escapes, the frame's own line prefix, the card's bullet, and
/// every space. The cards wrap their detail lines on a character count
/// and never drop a character, so what survives is exactly the text an
/// operator reads, in order, wherever the card happened to break — an
/// assertion over it reads the section rather than its line breaks.
fn prose(card: &str) -> String {
    plain(card)
        .lines()
        .map(|line| {
            line.trim_start()
                .trim_start_matches('\u{2502}')
                .trim_start()
                .trim_start_matches('\u{b7}')
        })
        .collect::<String>()
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect()
}

/// A scratch directory, unique per call.
fn scratch(tag: &str) -> PathBuf {
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let serial = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "fleet-delegate-{tag}-{}-{serial}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// The day the tests read and write: today (UTC), because the beat
/// stamps its rows with the real clock. A hardcoded day would silently
/// stop matching at a UTC midnight.
fn day() -> String {
    calib::today()
}

/// The sessions directory a fixture's logs live in, beside the stream's
/// own root: a child's log lands beside its parent's, which is what the
/// parent-chain join derives the parent's path from.
fn sessions(root: &Path) -> PathBuf {
    let dir = root.join("sessions");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// One delegation row in the stream, the shape the orchestrator's
/// writer lands: a `hard` brief on the routed provider that completed
/// in three turns, at depth 1 under a root session.
fn delegation_row(
    session: &str,
    provider: &str,
    brief_shape: &str,
    outcome: &str,
) -> DelegationRow {
    DelegationRow {
        ts: "2026-10-14T12:00:00Z".to_string(),
        session: session.to_string(),
        brief_shape: brief_shape.to_string(),
        scope: "bash,read_file,write_file,list_dir".to_string(),
        provider: provider.to_string(),
        outcome: outcome.to_string(),
        turns: 3,
        spend_usd: 0.0123,
        duration_ms: 812,
        log: String::new(),
        reason: None,
        parent_session: "parent-session".to_string(),
        root_session: "root-session".to_string(),
        depth: 1,
    }
}

/// Point one delegation row at the child log `session` names, under
/// `dir`.
fn with_log(mut row: DelegationRow, dir: &Path, session: &str) -> DelegationRow {
    row.log = dir
        .join(format!("{session}.jsonl"))
        .to_string_lossy()
        .into_owned();
    row
}

/// One judged row in the stream, as the beat's writer lands it.
fn judged_row(
    session: &str,
    provider: &str,
    brief_shape: &str,
    verdict: &str,
    confidence: f64,
) -> DelegationJudgedRow {
    DelegationJudgedRow {
        ts: calib::stamp(),
        session: session.to_string(),
        provider: provider.to_string(),
        brief_shape: brief_shape.to_string(),
        verdict: verdict.to_string(),
        confidence,
        reason: "the lane read the evidence".to_string(),
        disposition: None,
        cost_usd: Some(0.0002),
        latency_ms: Some(900),
        lane: DELEGATE_LANE.to_string(),
    }
}

/// One delegation row as the stream's own file carries it: the row
/// beside the op tag the reader joins on.
fn delegation_value(row: &DelegationRow) -> serde_json::Value {
    let mut value = serde_json::to_value(row).unwrap();
    value
        .as_object_mut()
        .unwrap()
        .insert("op".to_string(), serde_json::json!(OP_DELEGATION));
    value
}

/// One judged row as the stream's own file carries it.
fn judged_value(row: &DelegationJudgedRow) -> serde_json::Value {
    let mut value = serde_json::to_value(row).unwrap();
    value
        .as_object_mut()
        .unwrap()
        .insert("op".to_string(), serde_json::json!(OP_DELEGATION_JUDGED));
    value
}

/// A day's rows written straight to the stream's own file, so the
/// reader sees exactly what a writer would have landed.
fn write_day(root: &Path, day: &str, rows: &[serde_json::Value]) {
    let path = calib::day_path(root, day);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut text = String::new();
    for row in rows {
        text.push_str(&serde_json::to_string(row).unwrap());
        text.push('\n');
    }
    std::fs::write(&path, text).unwrap();
}

/// One session log's records written to `dir/<session>.jsonl`, the
/// crate's own layout, and the path the delegation row points at.
fn write_log(dir: &Path, session: &str, records: &[Record]) -> PathBuf {
    std::fs::create_dir_all(dir).unwrap();
    let path = dir.join(format!("{session}.jsonl"));
    let mut text = String::new();
    for record in records {
        text.push_str(&serde_json::to_string(record).unwrap());
        text.push('\n');
    }
    std::fs::write(&path, text).unwrap();
    path
}

/// One tool call as a session log records it.
fn call(id: &str, name: &str, arguments: &str) -> crate::api::ToolCall {
    crate::api::ToolCall {
        id: id.to_string(),
        kind: "function".to_string(),
        function: crate::api::FunctionCall {
            name: name.to_string(),
            arguments: arguments.to_string(),
        },
    }
}

/// A child's `start` record.
fn child_start(parent: &str) -> Record {
    Record::Start {
        v: 1,
        model: "step-5-preview".to_string(),
        created: "2026-10-14T12:00:00Z".to_string(),
        kind: Some(session::SUBAGENT_KIND.to_string()),
        parent: Some(parent.to_string()),
    }
}

/// A parent's `start` record.
fn parent_start() -> Record {
    Record::Start {
        v: 1,
        model: "step-5-preview".to_string(),
        created: "2026-10-14T11:59:00Z".to_string(),
        kind: None,
        parent: None,
    }
}

/// The child session log the fixture's delegation row points at: one
/// `write_file` on `src/delegate.rs`, one `bash`, and an orderly
/// `completed` ending at three turns — the claim the row makes.
fn child_log(dir: &Path, session: &str) -> PathBuf {
    write_log(
        dir,
        session,
        &[
            child_start("parent-session"),
            Record::Assistant {
                content: None,
                tool_calls: vec![call("c1", "write_file", r#"{"path":"src/delegate.rs"}"#)],
            },
            Record::ToolResult {
                call_id: "c1".to_string(),
                name: "write_file".to_string(),
                output: "wrote 12 bytes".to_string(),
                truncated: false,
                approved: true,
            },
            Record::Assistant {
                content: None,
                tool_calls: vec![call("c2", "bash", r#"{"command":"cargo build"}"#)],
            },
            Record::ToolResult {
                call_id: "c2".to_string(),
                name: "bash".to_string(),
                output: "ok".to_string(),
                truncated: false,
                approved: true,
            },
            Record::End {
                stop_reason: "completed".to_string(),
                turns: 3,
                served_model: None,
            },
        ],
    )
}

/// The parent session log beside the child's: the delegation call, its
/// result naming the child's session, and — when `continued` — a later
/// `write_file` on the very path the child wrote.
fn parent_log(dir: &Path, session: &str, child: &str, continued: bool) -> PathBuf {
    let mut records = vec![
        parent_start(),
        Record::Assistant {
            content: None,
            tool_calls: vec![call("p1", "subagent", r#"{"goal":"land the beat"}"#)],
        },
        Record::ToolResult {
            call_id: "p1".to_string(),
            name: "subagent".to_string(),
            output: format!("subagent completed · 3 turns · session {child} · log {child}.jsonl"),
            truncated: false,
            approved: true,
        },
    ];
    if continued {
        records.push(Record::Assistant {
            content: None,
            tool_calls: vec![call("p2", "write_file", r#"{"path":"src/delegate.rs"}"#)],
        });
        records.push(Record::ToolResult {
            call_id: "p2".to_string(),
            name: "write_file".to_string(),
            output: "wrote 30 bytes".to_string(),
            truncated: false,
            approved: true,
        });
    }
    write_log(dir, session, &records)
}

/// A scripted MCP lane: the `mock-mcp` child over a script whose one
/// `tools/call` answers with `reply`.
fn scripted_lane(reply: &str) -> Option<ServerSpec> {
    let bin = crate::eval::mock_mcp_binary()?;
    let scratch = scratch("lane-script");
    let entries = [
        serde_json::json!({ "reply": {
            "jsonrpc": "2.0", "id": "$echo",
            "result": {
                "protocolVersion": "2024-11-05",
                "capabilities": { "tools": { "listChanged": false } },
                "serverInfo": { "name": "sys1", "version": "0.1.0" },
            },
        } }),
        serde_json::json!({ "reply": {
            "jsonrpc": "2.0", "id": "$echo",
            "result": { "tools": [{ "name": "sys1_classify", "description": "one classify",
                                    "inputSchema": { "type": "object" } }] },
        } }),
        serde_json::json!({ "reply": {
            "jsonrpc": "2.0", "id": "$echo",
            "result": { "content": [{ "type": "text", "text": reply }], "isError": false },
        } }),
    ];
    let script = entries
        .iter()
        .map(|entry| format!("{entry}\n"))
        .collect::<String>();
    std::fs::write(scratch.join("script.jsonl"), script).unwrap();
    Some(ServerSpec {
        program: bin.to_string_lossy().into_owned(),
        args: vec![scratch.join("script.jsonl").to_string_lossy().into_owned()],
        env: Vec::new(),
        passthrough: Vec::new(),
        cwd: None,
    })
}

/// A `sys1_classify` answer carrying `verdict` as its judgment word,
/// with the confidence on the `workload` head the routing beat reads.
fn lane_reply(verdict: &str, confidence: f64) -> String {
    serde_json::json!({
        "task": "route",
        "verdict": verdict,
        "escalated": false,
        "provider": "glide",
        "providers_used": ["glide"],
        "answers": { "glide": {
            "workload": { "label": "hard", "confidence": confidence },
        } },
        "summary": [],
        "latency_ms": 900,
        "cost_usd": 0.0002,
    })
    .to_string()
}

/// One judge call over the scripted lane.
async fn judge_over(
    home: &Path,
    root: &Path,
    day: &str,
    dry_run: bool,
    reply: &str,
) -> JudgeOutcome {
    let lane = scripted_lane(reply).expect("the scripted lane");
    judge(JudgeRequest {
        root,
        home,
        day: day.to_string(),
        dry_run,
        lane: LaneSource::Scripted(lane),
    })
    .await
}

/// A judged row as the reader keeps it, for the calibration math's
/// tests. The session carries the index, so no two fixture rows
/// collide and dedupe into one.
fn read_judged(
    provider: &str,
    brief_shape: &str,
    verdict: &str,
    confidence: f64,
    disposition: &str,
) -> DelegationJudgedRow {
    let mut row = judged_row(
        &format!(
            "{provider}-{brief_shape}-{verdict}-{confidence}-{disposition}-{}",
            KEY_SERIAL.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ),
        provider,
        brief_shape,
        verdict,
        confidence,
    );
    row.confidence = confidence;
    row.disposition = (!disposition.is_empty()).then(|| disposition.to_string());
    row
}

/// The serial which keeps a fixture's judged rows distinct by key.
static KEY_SERIAL: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

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
            let previous = std::env::var_os(DELEGATE_BUDGET_ENV);
            std::env::set_var(DELEGATE_BUDGET_ENV, value);
            EnvVar { previous }
        }
    }
    impl Drop for EnvVar {
        fn drop(&mut self) {
            match &self.previous {
                Some(value) => std::env::set_var(DELEGATE_BUDGET_ENV, value),
                None => std::env::remove_var(DELEGATE_BUDGET_ENV),
            }
        }
    }

    let _guard = ENV_LOCK.lock().unwrap_or_else(|poison| poison.into_inner());
    std::env::remove_var(DELEGATE_BUDGET_ENV);
    assert_eq!(budget_usd_day(), DELEGATE_BUDGET_USD_DAY);
    // The cap's value is the stated one: 250 reserved asks at the
    // conservative per-ask reservation.
    assert_eq!(DELEGATE_BUDGET_USD_DAY, 0.05);
    assert!((DELEGATE_BUDGET_USD_DAY / DELEGATE_ESTIMATED_ASK_USD - 250.0).abs() < 1.0);
    // The owner's config lowers it.
    let _lower = EnvVar::set("0.005");
    assert_eq!(budget_usd_day(), 0.005);
    // And can never raise it: a value above the constant, a junk value,
    // and a negative one all read as the constant.
    let _higher = EnvVar::set("99.0");
    assert_eq!(budget_usd_day(), DELEGATE_BUDGET_USD_DAY);
    let _junk = EnvVar::set("not-a-number");
    assert_eq!(budget_usd_day(), DELEGATE_BUDGET_USD_DAY);
    let _negative = EnvVar::set("-1");
    assert_eq!(budget_usd_day(), DELEGATE_BUDGET_USD_DAY);
}

#[test]
fn the_lane_task_and_lane_are_the_ones_the_router_selects_with() {
    // The same lane, tool, and registered task the routing beat asks,
    // so the two beats cannot drift apart on which lane answers or which
    // question it answers.
    assert_eq!(DELEGATE_LANE, "sys1");
    assert_eq!(DELEGATE_TASK, "route");
    assert_eq!(DELEGATE_TOOL, "sys1_classify");
    // The thresholds are the sibling's own, reused rather than restated:
    // one place carries the fit rule's numbers.
    assert_eq!(FIT_MIN_ROWS, 8);
    assert!((FIT_MIN_SPAN - 0.30).abs() < 1e-9);
    assert!((WRONG_RATE_FLOOR - 0.50).abs() < 1e-9);
    assert!((INSUFFICIENT_RATE_FLOOR - 0.50).abs() < 1e-9);
}

#[test]
fn the_prompt_carries_the_outcome_evidence_and_never_the_summary_text() {
    let dir = sessions(&scratch("prompt"));
    let mut row = delegation_row("child-1", "zai-coding-plan", "hard", OUTCOME_COMPLETED);
    row.reason = Some("the depth cap refused the call".to_string());
    row = with_log(row, &dir, "child-1");
    let log = child_log(&dir, "child-1");
    parent_log(&dir, "parent-session", "child-1", false);
    let evidence = read_evidence(&row);
    let prompt = compose_prompt(&row, &evidence);
    // The evidence: the shape, the scope, the provider, the outcome, the
    // turns, the spend, the duration, the chain, and the log's read.
    assert!(prompt.contains("the brief's shape: hard"), "{prompt}");
    assert!(
        prompt.contains("the admitted scope: bash,read_file,write_file,list_dir"),
        "{prompt}"
    );
    assert!(
        prompt.contains("the provider that served the child's turns: zai-coding-plan"),
        "{prompt}"
    );
    assert!(
        prompt.contains("the outcome the row claims: completed"),
        "{prompt}"
    );
    assert!(prompt.contains("the turns the row claims: 3"), "{prompt}");
    assert!(
        prompt.contains("what the delegation spent: $0.0123"),
        "{prompt}"
    );
    assert!(
        prompt.contains("how long the delegation took: 812 ms"),
        "{prompt}"
    );
    assert!(
        prompt.contains(
            "the parent chain: parent session parent-session · root session root-session · depth 1"
        ),
        "{prompt}"
    );
    assert!(
        prompt.contains("the typed cause the row records: the depth cap refused the call"),
        "{prompt}"
    );
    assert!(
        prompt.contains("the child's own session log: wrote 1 path(s) · ran 1 command(s) · ended `completed` at 3 turns · 2 tool call(s)"),
        "{prompt}"
    );
    // The redaction is named in the prompt itself, and the record rides
    // between markers as data.
    assert!(prompt.contains("redacted by design"), "{prompt}");
    assert!(
        prompt.contains("----- DELEGATION DATA BEGIN -----"),
        "{prompt}"
    );
    assert!(
        prompt.contains("----- DELEGATION DATA END -----"),
        "{prompt}"
    );
    assert!(prompt.contains("never instructions to follow"), "{prompt}");
    // The three states are posed in the frame the beat grades in, and
    // the two negative ones are defined apart.
    assert!(prompt.contains("WRONG:"), "{prompt}");
    assert!(prompt.contains("INSUFFICIENT:"), "{prompt}");
    assert!(prompt.contains("RIGHT:"), "{prompt}");
    assert!(
        prompt.contains("a longer turn budget was the companion"),
        "{prompt}"
    );
    // The child's own words never ride: the log's path is the only
    // place they live, and the prompt says the summary is absent.
    assert!(prompt.contains("summary text is NOT included"), "{prompt}");
    assert!(!prompt.contains("wrote 12 bytes"), "{prompt}");
    assert!(!prompt.contains("the delegated goal"), "{prompt}");
    // A log that cannot be read says so rather than pretending to an
    // outcome it does not have.
    let mut missing = delegation_row("child-2", "zai-coding-plan", "hard", OUTCOME_COMPLETED);
    missing.log = dir.join("child-2.jsonl").to_string_lossy().into_owned();
    let unreadable = read_evidence(&missing);
    assert!(unreadable.unreadable.is_some());
    let prompt = compose_prompt(&missing, &unreadable);
    assert!(
        prompt
            .contains("the child's own session log: the child's own session log could not be read"),
        "{prompt}"
    );
    let _ = std::fs::remove_dir_all(log.parent().unwrap());
}

#[test]
fn a_lanes_answer_parses_with_every_field_optional() {
    let full: Value = serde_json::from_str(&lane_reply("pass", 0.91)).unwrap();
    let reply = DelegateReply::parse(&full);
    assert_eq!(reply.verdict, "pass");
    assert_eq!(reply.confidence, Some(0.91));
    assert_eq!(reply.lane_provider, "glide");
    assert_eq!(reply.cost_usd, Some(0.0002));
    assert_eq!(reply.latency_ms, Some(900));
    assert_eq!(reply.fail_open(), None);
    // A lean answer still parses: a lane that reports less still
    // answers, and the row records what it reported rather than a guess.
    let lean: Value = serde_json::from_str(r#"{"verdict": "block"}"#).unwrap();
    let reply = DelegateReply::parse(&lean);
    assert_eq!(reply.verdict, "block");
    assert_eq!(reply.confidence, None);
    // The answer's own confidence field is read where it carries one.
    let own: Value = serde_json::from_str(r#"{"verdict": "pass", "confidence": 0.42}"#).unwrap();
    assert_eq!(DelegateReply::parse(&own).confidence, Some(0.42));
    // A non-object answer is an empty reply, never a panic.
    assert_eq!(DelegateReply::parse(&Value::Null).verdict, "");
}

#[test]
fn a_lanes_fail_open_parses_its_status_and_names_its_reason() {
    // The vendored lane's fail-open shape: no provider answered, so the
    // answer carries `verdict: error` with the lane's own reason in the
    // telemetry and no verdict to grade. The parse carries all of it
    // through, and `fail_open()` names the lane's error — the signal the
    // beat refuses on rather than shipping the fallback as a judgment.
    let fallback = serde_json::json!({
        "task": "route",
        "verdict": "error",
        "provider": null,
        "providers_used": [],
        "answers": {},
        "telemetry": { "glide": { "error": "no provider succeeded", "error_kind": "provider" } },
        "summary": ["No provider succeeded — unclassified."],
        "latency_ms": 12,
        "reason": "no provider succeeded",
    })
    .to_string();
    let value: Value = serde_json::from_str(&fallback).unwrap();
    let reply = DelegateReply::parse(&value);
    assert_eq!(reply.verdict, "error");
    assert_eq!(
        reply.telemetry_error.as_deref(),
        Some("no provider succeeded")
    );
    let reason = reply.fail_open().expect("the fail-open is named");
    assert!(reason.contains("no provider succeeded"), "{reason}");

    // An answer that carries a verdict word is an answer, whatever its
    // confidence: a tentative judgment is a judgment.
    let low: Value = serde_json::from_str(&lane_reply("escalate", 0.31)).unwrap();
    assert_eq!(DelegateReply::parse(&low).fail_open(), None);
    // An answer that names no verdict at all is no answer: there is
    // nothing to grade.
    let mute: Value = serde_json::from_str(r#"{"latency_ms": 4}"#).unwrap();
    let reason = DelegateReply::parse(&mute)
        .fail_open()
        .expect("a verdict-less answer is no answer");
    assert!(reason.contains("no verdict"), "{reason}");
    // An error verdict with no reason of its own is still named, by its
    // status.
    let mute: Value = serde_json::from_str(r#"{"verdict": "error"}"#).unwrap();
    let reason = DelegateReply::parse(&mute)
        .fail_open()
        .expect("named by its status");
    assert!(reason.contains("error"), "{reason}");
}

// --- the grade: the three signals ---

/// A lane answer carrying `verdict` at `confidence`, parsed.
fn reply(verdict: &str, confidence: f64) -> DelegateReply {
    DelegateReply::parse(&serde_json::from_str::<Value>(&lane_reply(verdict, confidence)).unwrap())
}

#[test]
fn the_grade_is_wrong_when_the_childs_log_contradicts_the_claimed_outcome() {
    // The row claims the child completed its stated task; the child's own
    // session log ended `max-turns`. A claim the log contradicts is an
    // anti-vector whatever the lane said about it.
    let dir = sessions(&scratch("contradicted"));
    let mut row = delegation_row("child-1", "zai-coding-plan", "hard", OUTCOME_COMPLETED);
    row = with_log(row, &dir, "child-1");
    write_log(
        &dir,
        "child-1",
        &[
            child_start("parent-session"),
            Record::Assistant {
                content: None,
                tool_calls: vec![call("c1", "write_file", r#"{"path":"src/delegate.rs"}"#)],
            },
            Record::End {
                stop_reason: "max-turns".to_string(),
                turns: 3,
                served_model: None,
            },
        ],
    );
    let evidence = read_evidence(&row);
    // The lane said `pass`: the report is not the evidence, and the
    // facts gate.
    let judgment = grade(&row, &evidence, &reply("pass", 0.91));
    assert_eq!(judgment.verdict, calib::VERDICT_WRONG);
    assert!(
        judgment.reason.contains("contradicted"),
        "{}",
        judgment.reason
    );
    assert!(judgment.reason.contains("max-turns"), "{}", judgment.reason);
    assert!(
        judgment.reason.contains("drove away"),
        "{}",
        judgment.reason
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_grade_is_wrong_when_the_row_claims_more_turns_than_the_log_holds() {
    let dir = sessions(&scratch("turns"));
    let mut row = delegation_row("child-1", "zai-coding-plan", "hard", OUTCOME_COMPLETED);
    row.turns = 9;
    row = with_log(row, &dir, "child-1");
    write_log(
        &dir,
        "child-1",
        &[
            child_start("parent-session"),
            Record::Assistant {
                content: None,
                tool_calls: vec![call("c1", "bash", r#"{"command":"cargo build"}"#)],
            },
            Record::End {
                stop_reason: "completed".to_string(),
                turns: 3,
                served_model: None,
            },
        ],
    );
    let evidence = read_evidence(&row);
    assert_eq!(evidence.turns, 3);
    let judgment = grade(&row, &evidence, &reply("pass", 0.9));
    assert_eq!(judgment.verdict, calib::VERDICT_WRONG);
    assert!(
        judgment.reason.contains("claims 9 turns"),
        "{}",
        judgment.reason
    );
    assert!(
        judgment.reason.contains("log recorded 3"),
        "{}",
        judgment.reason
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_grade_is_wrong_when_the_claimed_artifact_or_command_is_absent_from_the_childs_log() {
    // The row claims the child completed its stated task, and the child's
    // own session log holds no tool call at all: the claimed artifact or
    // command is absent from its own log. The false-report case, and the
    // most serious kind.
    let dir = sessions(&scratch("absent"));
    let mut row = delegation_row("child-1", "zai-coding-plan", "hard", OUTCOME_COMPLETED);
    row = with_log(row, &dir, "child-1");
    write_log(
        &dir,
        "child-1",
        &[
            child_start("parent-session"),
            Record::End {
                stop_reason: "completed".to_string(),
                turns: 3,
                served_model: None,
            },
        ],
    );
    let evidence = read_evidence(&row);
    assert_eq!(evidence.calls, 0);
    let judgment = grade(&row, &evidence, &reply("pass", 0.88));
    assert_eq!(judgment.verdict, calib::VERDICT_WRONG);
    assert!(
        judgment.reason.contains("absent from its own log"),
        "{}",
        judgment.reason
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_grade_is_insufficient_when_the_child_stopped_at_max_turns_naming_the_companion() {
    // The child stopped at its turn cap with its direction right: the
    // minima was not reached for want of a companion, and the companion
    // is named — "insufficient" without the absent companion is not
    // actionable.
    let dir = sessions(&scratch("max-turns"));
    let mut row = delegation_row("child-1", "zai-coding-plan", "hard", OUTCOME_MAX_TURNS);
    row = with_log(row, &dir, "child-1");
    write_log(
        &dir,
        "child-1",
        &[
            child_start("parent-session"),
            Record::Assistant {
                content: None,
                tool_calls: vec![call("c1", "write_file", r#"{"path":"src/delegate.rs"}"#)],
            },
            Record::End {
                stop_reason: "max-turns".to_string(),
                turns: 12,
                served_model: None,
            },
        ],
    );
    let evidence = read_evidence(&row);
    let judgment = grade(&row, &evidence, &reply("pass", 0.9));
    assert_eq!(judgment.verdict, calib::VERDICT_INSUFFICIENT);
    assert!(
        judgment.reason.contains("a longer turn budget"),
        "{}",
        judgment.reason
    );
    assert!(
        judgment.reason.contains("want of a companion"),
        "{}",
        judgment.reason
    );
    assert!(
        judgment.reason.contains("direction was right"),
        "{}",
        judgment.reason
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_grade_is_insufficient_when_the_run_failed_and_names_what_the_step_needed() {
    let dir = sessions(&scratch("failed"));
    let mut row = delegation_row("child-1", "zai-coding-plan", "hard", OUTCOME_FAILED);
    row.reason = Some("the lane refused the ask: no key".to_string());
    row = with_log(row, &dir, "child-1");
    write_log(
        &dir,
        "child-1",
        &[
            child_start("parent-session"),
            Record::Assistant {
                content: None,
                tool_calls: vec![call("c1", "bash", r#"{"command":"cargo build"}"#)],
            },
            Record::End {
                stop_reason: "failed".to_string(),
                turns: 3,
                served_model: None,
            },
        ],
    );
    let evidence = read_evidence(&row);
    let judgment = grade(&row, &evidence, &reply("pass", 0.85));
    assert_eq!(judgment.verdict, calib::VERDICT_INSUFFICIENT);
    assert!(
        judgment
            .reason
            .contains("whatever the failed step needed to finish"),
        "{}",
        judgment.reason
    );
    // The row's own typed cause rides the reason, so the companion is
    // as specific as the record allows.
    assert!(
        judgment.reason.contains("the lane refused the ask: no key"),
        "{}",
        judgment.reason
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_grade_is_insufficient_when_the_parents_session_continues_the_same_paths() {
    // The child completed its stated task and the parent's own session
    // kept working on the path the child wrote: the brief was missing a
    // companion, and the companion is named by its path.
    let dir = sessions(&scratch("continued"));
    let mut row = delegation_row("child-1", "zai-coding-plan", "hard", OUTCOME_COMPLETED);
    row = with_log(row, &dir, "child-1");
    child_log(&dir, "child-1");
    parent_log(&dir, "parent-session", "child-1", true);
    let evidence = read_evidence(&row);
    assert_eq!(evidence.continued, vec!["src/delegate.rs".to_string()]);
    let judgment = grade(&row, &evidence, &reply("pass", 0.9));
    assert_eq!(judgment.verdict, calib::VERDICT_INSUFFICIENT);
    assert!(
        judgment.reason.contains("src/delegate.rs"),
        "{}",
        judgment.reason
    );
    assert!(
        judgment
            .reason
            .contains("the companion the brief never named"),
        "{}",
        judgment.reason
    );
    assert!(
        judgment.reason.contains("want of a companion"),
        "{}",
        judgment.reason
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_grade_is_right_when_the_task_completed_and_the_claim_verified_against_the_log() {
    let dir = sessions(&scratch("right"));
    let mut row = delegation_row("child-1", "zai-coding-plan", "hard", OUTCOME_COMPLETED);
    row = with_log(row, &dir, "child-1");
    child_log(&dir, "child-1");
    // The parent's session did NOT continue on the child's paths, so no
    // companion is named.
    parent_log(&dir, "parent-session", "child-1", false);
    let evidence = read_evidence(&row);
    let judgment = grade(&row, &evidence, &reply("pass", 0.91));
    assert_eq!(judgment.verdict, calib::VERDICT_RIGHT);
    assert!(
        judgment
            .reason
            .contains("claim verified against the child's own session log"),
        "{}",
        judgment.reason
    );
    assert!((judgment.confidence - 0.91).abs() < 1e-9);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_wrong_claim_outranks_a_missing_companion() {
    // Both signals at once: the row claims completion while the child's
    // log ended `max-turns`, AND the parent's session continued the work
    // on the child's path. The anti-vector is the actionable reading, so
    // the grade is wrong — an incomplete trajectory whose claim was also
    // false is a failure first.
    let dir = sessions(&scratch("outranks"));
    let mut row = delegation_row("child-1", "zai-coding-plan", "hard", OUTCOME_COMPLETED);
    row = with_log(row, &dir, "child-1");
    write_log(
        &dir,
        "child-1",
        &[
            child_start("parent-session"),
            Record::Assistant {
                content: None,
                tool_calls: vec![call("c1", "write_file", r#"{"path":"src/delegate.rs"}"#)],
            },
            Record::End {
                stop_reason: "max-turns".to_string(),
                turns: 3,
                served_model: None,
            },
        ],
    );
    parent_log(&dir, "parent-session", "child-1", true);
    let evidence = read_evidence(&row);
    assert!(!evidence.continued.is_empty(), "the continuation joined");
    let judgment = grade(&row, &evidence, &reply("pass", 0.9));
    assert_eq!(judgment.verdict, calib::VERDICT_WRONG);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_lanes_block_verdict_is_wrong_and_its_escalate_is_insufficient() {
    // The lane's own word is a report, and it can make a grade more
    // negative but never more positive. Clean evidence beside a `block`
    // is still a drive-away reading; beside an `escalate` it still names
    // an absent companion.
    let dir = sessions(&scratch("lane-word"));
    let mut row = delegation_row("child-1", "zai-coding-plan", "hard", OUTCOME_COMPLETED);
    row = with_log(row, &dir, "child-1");
    child_log(&dir, "child-1");
    parent_log(&dir, "parent-session", "child-1", false);
    let evidence = read_evidence(&row);

    let judgment = grade(&row, &evidence, &reply("block", 0.77));
    assert_eq!(judgment.verdict, calib::VERDICT_WRONG);
    assert!(judgment.reason.contains("`block`"), "{}", judgment.reason);

    let judgment = grade(&row, &evidence, &reply("escalate", 0.66));
    assert_eq!(judgment.verdict, calib::VERDICT_INSUFFICIENT);
    assert!(
        judgment.reason.contains("`escalate`"),
        "{}",
        judgment.reason
    );
    assert!(
        judgment.reason.contains("want of a companion"),
        "{}",
        judgment.reason
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn an_unreadable_child_log_is_absent_evidence_rather_than_a_guessed_signal() {
    // The log is gone: the signals are absent, not asserted. The grade
    // rests on the row's own claim beside the lane's word, and the
    // prompt says the log could not be read.
    let dir = sessions(&scratch("unreadable"));
    let mut row = delegation_row("child-1", "zai-coding-plan", "hard", OUTCOME_COMPLETED);
    row = with_log(row, &dir, "child-1");
    let evidence = read_evidence(&row);
    assert!(evidence.unreadable.is_some());
    assert!(evidence.wrote.is_empty());
    assert!(evidence.continued.is_empty());
    let judgment = grade(&row, &evidence, &reply("pass", 0.9));
    assert_eq!(judgment.verdict, calib::VERDICT_RIGHT);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_log_that_recorded_no_ending_is_absent_evidence_not_a_contradiction() {
    // The ruling the routing beat carries for `interrupted`, ported: a
    // log with no `end` record is a log that never recorded an ending —
    // ABSENT outcome evidence rather than a named absence. The row's
    // claim of completion stands on the work the log does hold, and an
    // owner's own stop cannot amber-stain a cell.
    let dir = sessions(&scratch("no-ending"));
    let mut row = delegation_row("child-1", "zai-coding-plan", "hard", OUTCOME_COMPLETED);
    row = with_log(row, &dir, "child-1");
    write_log(
        &dir,
        "child-1",
        &[
            child_start("parent-session"),
            Record::Assistant {
                content: None,
                tool_calls: vec![call("c1", "write_file", r#"{"path":"src/delegate.rs"}"#)],
            },
        ],
    );
    let evidence = read_evidence(&row);
    assert_eq!(evidence.ending, None);
    assert_eq!(evidence.turns, 1, "the assistant turns are the count");
    let judgment = grade(&row, &evidence, &reply("pass", 0.9));
    assert_eq!(judgment.verdict, calib::VERDICT_RIGHT);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_log_join_reads_the_childs_writes_and_the_parents_continuation() {
    let dir = sessions(&scratch("join"));
    let mut row = delegation_row("child-1", "zai-coding-plan", "hard", OUTCOME_COMPLETED);
    row = with_log(row, &dir, "child-1");
    child_log(&dir, "child-1");
    parent_log(&dir, "parent-session", "child-1", true);
    let evidence = read_evidence(&row);
    assert_eq!(evidence.wrote, vec!["src/delegate.rs".to_string()]);
    assert_eq!(evidence.commands, 1);
    assert_eq!(evidence.calls, 2);
    assert_eq!(evidence.ending.as_deref(), Some("completed"));
    assert_eq!(evidence.turns, 3);
    assert_eq!(evidence.continued, vec!["src/delegate.rs".to_string()]);

    // A parent log whose delegation result is not joinable — no tool
    // result names this child — leaves the continuation absent rather
    // than guessed.
    parent_log(&dir, "other-parent", "child-9", true);
    let mut orphan = delegation_row("child-1", "zai-coding-plan", "hard", OUTCOME_COMPLETED);
    orphan.parent_session = "other-parent".to_string();
    orphan = with_log(orphan, &dir, "child-1");
    let evidence = read_evidence(&orphan);
    assert!(evidence.continued.is_empty(), "{:?}", evidence.continued);
    // A row with no parent session at all reads the same way.
    let mut rootless = delegation_row("child-1", "zai-coding-plan", "hard", OUTCOME_COMPLETED);
    rootless.parent_session = String::new();
    rootless = with_log(rootless, &dir, "child-1");
    assert!(read_evidence(&rootless).continued.is_empty());
    let _ = std::fs::remove_dir_all(&dir);
}

// --- the row, the reader, and the round trip ---

#[test]
fn the_delegation_row_is_redacted_and_keyed_and_round_trips_through_the_reader() {
    // The KI-164/KI-171 class: a hand-written fixture row that the
    // reader drops silently is a false negative, so the row is written
    // through the stream and read back through the beat's own reader
    // before anything downstream is trusted. The fixture row OMITS the
    // provider, the depth, and the typed cause: every field defaults,
    // so none of them is dropped.
    let root = scratch("round-trip");
    let mut row = delegation_row("child-1", "zai-coding-plan", "hard", OUTCOME_COMPLETED);
    row.provider = String::new();
    row.depth = 0;
    row.reason = None;
    write_day(
        &root,
        &day(),
        &[
            delegation_value(&row),
            judged_value(&judged_row(
                "child-1",
                "zai-coding-plan",
                "hard",
                calib::VERDICT_WRONG,
                0.91,
            )),
        ],
    );
    let rows = read_day(&root, &day());
    assert_eq!(
        rows.delegations.len(),
        1,
        "the reader dropped the fixture row"
    );
    assert_eq!(rows.delegations[0].session, "child-1");
    assert_eq!(rows.delegations[0].brief_shape, "hard");
    assert_eq!(rows.delegations[0].provider, "");
    assert_eq!(rows.delegations[0].depth, 0);
    assert_eq!(rows.judged.len(), 1);
    assert_eq!(rows.judged[0].session, "child-1");
    assert_eq!(rows.judged[0].verdict, calib::VERDICT_WRONG);
    assert!((rows.judged[0].confidence - 0.91).abs() < 1e-9);
    assert_eq!(rows.judged[0].lane, DELEGATE_LANE);
    // The key is the child's own session id, so the two ops join on it.
    assert_eq!(delegation_key(&rows.delegations[0]), "child-1");
    assert_eq!(judged_keys(&rows.judged), vec!["child-1".to_string()]);
    // The judged row is redacted by shape: no prompt text and no answer
    // text rides in it, under any key a reader might look for.
    let line = serde_json::to_string(&judged_value(&judged_row(
        "child-1",
        "zai-coding-plan",
        "hard",
        calib::VERDICT_WRONG,
        0.91,
    )))
    .unwrap();
    for forbidden in [
        "prompt",
        "answer",
        "final_answer",
        "text",
        "summary",
        "artifact",
        "DELEGATION DATA",
    ] {
        assert!(
            !line.contains(&format!("\"{forbidden}\"")),
            "the row carries `{forbidden}`: {line}"
        );
    }
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn the_freshest_row_per_key_stands_so_a_disposition_is_a_later_row() {
    // The stream is append-only, so a disposition is a later row for the
    // same session and a reader takes the freshest one — the automated
    // verdict stays on the row it was written with.
    let root = scratch("freshest");
    let mut with_disposition = judged_row(
        "child-1",
        "zai-coding-plan",
        "hard",
        calib::VERDICT_WRONG,
        0.91,
    );
    with_disposition.disposition = Some(calib::DISPOSITION_DISAGREED.to_string());
    write_day(
        &root,
        &day(),
        &[
            judged_value(&judged_row(
                "child-1",
                "zai-coding-plan",
                "hard",
                calib::VERDICT_WRONG,
                0.91,
            )),
            judged_value(&with_disposition),
        ],
    );
    let rows = read_day(&root, &day());
    let judged = judged_rows(&rows.judged);
    assert_eq!(
        judged.len(),
        1,
        "the key is deduplicated to its freshest row"
    );
    assert_eq!(
        judged[0].disposition.as_deref(),
        Some(calib::DISPOSITION_DISAGREED)
    );
    // The verdict the lane wrote is unchanged: the disposition overrides
    // the math, never the record.
    assert_eq!(judged[0].verdict, calib::VERDICT_WRONG);
    let _ = std::fs::remove_dir_all(&root);
}

// --- the calibration math ---

#[test]
fn the_fit_rule_needs_rows_and_span_and_an_unfit_cell_renders_no_score() {
    // Too few rows: unfit, whatever the rate.
    let rows: Vec<DelegationJudgedRow> = (0..3)
        .map(|index| {
            read_judged(
                "zai-coding-plan",
                "hard",
                calib::VERDICT_WRONG,
                0.9 - index as f64 * 0.1,
                "",
            )
        })
        .collect();
    let cells = calibration(&rows);
    assert_eq!(cells.len(), 1);
    assert_eq!(cells[0].fit, FitState::Unfit);
    assert_eq!(cells[0].needs_rows(), FIT_MIN_ROWS - 3);
    // Enough rows but no span: the confidence weighting would be
    // vacuous, so the cell is still unfit and says what it needs.
    let flat: Vec<DelegationJudgedRow> = (0..FIT_MIN_ROWS)
        .map(|_| read_judged("zai-coding-plan", "hard", calib::VERDICT_WRONG, 0.9, ""))
        .collect();
    let cells = calibration(&flat);
    assert_eq!(cells[0].fit, FitState::Unfit);
    assert!((cells[0].needs_span() - FIT_MIN_SPAN).abs() < 1e-9);
    // Enough rows and enough span: fit.
    let fit: Vec<DelegationJudgedRow> = (0..FIT_MIN_ROWS)
        .map(|index| {
            read_judged(
                "zai-coding-plan",
                "hard",
                calib::VERDICT_WRONG,
                0.95 - index as f64 * 0.05,
                "",
            )
        })
        .collect();
    let cells = calibration(&fit);
    assert_eq!(cells[0].fit, FitState::Fit);
    assert_eq!(cells[0].needs_rows(), 0);
}

#[test]
fn the_calibration_is_keyed_by_provider_and_brief_shape_and_never_averaged() {
    // The owner's directive: a brief shape that drives toward done for
    // one kind of work and away from another is two vectors, not one.
    // One provider, two shapes, opposite verdicts — two cells, neither
    // hiding the other, and no per-provider average anywhere in the
    // read.
    let mut rows: Vec<DelegationJudgedRow> = (0..FIT_MIN_ROWS)
        .map(|index| {
            read_judged(
                "zai-coding-plan",
                "hard",
                calib::VERDICT_WRONG,
                0.95 - index as f64 * 0.05,
                "",
            )
        })
        .collect();
    rows.extend((0..FIT_MIN_ROWS).map(|index| {
        read_judged(
            "zai-coding-plan",
            "prose",
            calib::VERDICT_RIGHT,
            0.95 - index as f64 * 0.05,
            "",
        )
    }));
    let cells = calibration(&rows);
    assert_eq!(cells.len(), 2, "two shapes, two cells");
    let hard = cell(&cells, "zai-coding-plan", "hard").expect("the hard cell");
    let prose = cell(&cells, "zai-coding-plan", "prose").expect("the prose cell");
    assert_eq!(hard.judged, FIT_MIN_ROWS);
    assert_eq!(hard.wrong, FIT_MIN_ROWS);
    assert_eq!(prose.right, FIT_MIN_ROWS);
    assert_eq!(prose.wrong, 0);
    // The hard cell is fit and over the floor; the prose cell is fit and
    // nowhere near it. An averaged reading would have been 0.5 and would
    // have moved a suggestion on a shape that was half right.
    assert!(hard.informs_failover());
    assert!(!prose.informs_failover());
    // A shape the day holds no cell for reads as no cell at all.
    assert_eq!(cell(&cells, "zai-coding-plan", "quick"), None);
    // Each cell prices its own spend over its own right count.
    assert!(hard.cost_per_right.is_none(), "no right call to price");
    assert!(prose.cost_per_right.is_some());
}

#[test]
fn the_wrong_rate_is_confidence_weighted_and_the_floor_is_named() {
    // Eight rows spanning 0.35, six wrong and two right: the confident
    // wrongs dominate, so the weighted rate is above the plain one.
    let rows: Vec<DelegationJudgedRow> = (0..FIT_MIN_ROWS)
        .map(|index| {
            let (verdict, confidence) = if index < 6 {
                (calib::VERDICT_WRONG, 0.95 - index as f64 * 0.02)
            } else {
                (calib::VERDICT_RIGHT, 0.55)
            };
            read_judged("zai-coding-plan", "hard", verdict, confidence, "")
        })
        .collect();
    let cells = calibration(&rows);
    let cell = &cells[0];
    assert_eq!(cell.judged, 8);
    assert_eq!(cell.wrong, 6);
    assert_eq!(cell.right, 2);
    assert_eq!(cell.fit, FitState::Fit);
    // Weighted, not plain: 6/8 = 0.75 plain, and the wrongs are the
    // confident ones.
    assert!(
        cell.weighted_wrong_rate > 0.75,
        "{}",
        cell.weighted_wrong_rate
    );
    assert!(cell.informs_failover(), "{}", cell.weighted_wrong_rate);
    assert!((WRONG_RATE_FLOOR - 0.50).abs() < 1e-9);
    // The cost per correct call is the cell's spend over its right count.
    assert!(cell.cost_per_right.is_some());
}

#[test]
fn three_confident_wrongs_cross_the_floor_in_the_weighted_regime() {
    // The fit rule's arithmetic, pinned where it can fail — with the
    // CORRECTED reasoning (KI-184), not the false one.
    //
    // Read UNWEIGHTED, the floor's first crossing is four wrongs of
    // eight: three of eight is 0.375, below the 0.50 floor, and four is
    // exactly 0.50, the first crossing. The claim that used to justify
    // these thresholds — that three wrongs can never cross — was false,
    // and the true statement is the weighted regime's: three wrongs at
    // 0.95 against five rights at 0.55 weight to 2.85 of 5.60, which is
    // 0.509, over the floor, on a sample whose 0.40 span is fit. That is
    // deliberate rather than a loophole: the weighting is the point, and
    // FIT_MIN_SPAN is what keeps a flat high-confidence sample from
    // degenerating into an unweighted rate.
    let rows: Vec<DelegationJudgedRow> = (0..FIT_MIN_ROWS)
        .map(|index| {
            let (verdict, confidence) = if index < 3 {
                (calib::VERDICT_WRONG, 0.95)
            } else {
                (calib::VERDICT_RIGHT, 0.55)
            };
            read_judged("zai-coding-plan", "hard", verdict, confidence, "")
        })
        .collect();
    let cells = calibration(&rows);
    let cell = &cells[0];
    assert_eq!(cell.wrong, 3);
    assert_eq!(cell.right, 5);
    assert_eq!(cell.fit, FitState::Fit);
    assert!(
        (cell.confidence_span - 0.40).abs() < 1e-9,
        "{}",
        cell.confidence_span
    );
    assert!(
        (cell.weighted_wrong_rate - 2.85 / 5.60).abs() < 1e-9,
        "{}",
        cell.weighted_wrong_rate
    );
    assert!(cell.weighted_wrong_rate > WRONG_RATE_FLOOR);
    assert!(cell.informs_failover());
    // The same three wrongs unweighted do not cross, and four would:
    // the true unweighted crossing, pinned at compile time.
    const _: () = assert!((3.0 / 8.0) < WRONG_RATE_FLOOR);
    const _: () = assert!((4.0 / 8.0) >= WRONG_RATE_FLOOR);
}

#[test]
fn a_cell_below_the_floor_is_fit_and_does_not_inform_the_ride_back() {
    let rows: Vec<DelegationJudgedRow> = (0..FIT_MIN_ROWS)
        .map(|index| {
            let (verdict, confidence) = if index < 3 {
                (calib::VERDICT_WRONG, 0.95 - index as f64 * 0.05)
            } else {
                (calib::VERDICT_RIGHT, 0.6)
            };
            read_judged("zai-coding-plan", "hard", verdict, confidence, "")
        })
        .collect();
    let cells = calibration(&rows);
    assert_eq!(cells[0].fit, FitState::Fit);
    assert!(cells[0].weighted_wrong_rate < WRONG_RATE_FLOOR);
    assert!(!cells[0].informs_failover());
}

#[test]
fn an_insufficient_cell_keeps_its_rate_apart_and_never_informs_the_ride_back() {
    // The third vector in the math: eight rows spanning 0.35, all
    // insufficient — the direction was right every time and the minima
    // was not reached for want of a companion. The cell counts them
    // apart from both the rights and the wrongs, the insufficient-rate
    // is its own number, and it moves nothing: an incomplete solution is
    // not a failure.
    let rows: Vec<DelegationJudgedRow> = (0..FIT_MIN_ROWS)
        .map(|index| {
            read_judged(
                "zai-coding-plan",
                "hard",
                calib::VERDICT_INSUFFICIENT,
                0.95 - index as f64 * 0.05,
                "",
            )
        })
        .collect();
    let cells = calibration(&rows);
    let cell = &cells[0];
    assert_eq!(cell.judged, 8);
    assert_eq!(cell.right, 0);
    assert_eq!(cell.wrong, 0);
    assert_eq!(cell.insufficient, 8);
    assert_eq!(cell.fit, FitState::Fit);
    assert!(
        (cell.weighted_insufficient_rate - 1.0).abs() < 1e-9,
        "{}",
        cell.weighted_insufficient_rate
    );
    assert!(
        cell.weighted_wrong_rate.abs() < 1e-9,
        "{}",
        cell.weighted_wrong_rate
    );
    // The wrong-rate is the only thing that moves: a wholly insufficient
    // cell moves no candidate, however high its rate.
    assert!(!cell.informs_failover());
    assert!(cell.notes_companions());
}

#[test]
fn the_two_negative_vectors_are_never_folded_together() {
    // Four wrong and four insufficient on one fit cell: the wrong-rate
    // reads only the four wrongs, the insufficient-rate only the four
    // insufficients, and the ride-back follows the wrong-rate alone.
    // Folding them would move a suggestion on a shape that was
    // right-but-unaccompanied half the time.
    let rows: Vec<DelegationJudgedRow> = (0..FIT_MIN_ROWS)
        .map(|index| {
            let (verdict, confidence) = if index < 4 {
                (calib::VERDICT_WRONG, 0.95 - index as f64 * 0.05)
            } else {
                (
                    calib::VERDICT_INSUFFICIENT,
                    0.75 - (index - 4) as f64 * 0.05,
                )
            };
            read_judged("zai-coding-plan", "hard", verdict, confidence, "")
        })
        .collect();
    let cells = calibration(&rows);
    let cell = &cells[0];
    assert_eq!(cell.wrong, 4);
    assert_eq!(cell.insufficient, 4);
    assert_eq!(cell.right, 0);
    // The wrongs carry 0.95+0.90+0.85+0.80 = 3.5 of the 6.2 total
    // confidence weight; the insufficients the other 2.7.
    assert!(
        (cell.weighted_wrong_rate - 3.5 / 6.2).abs() < 1e-9,
        "{}",
        cell.weighted_wrong_rate
    );
    assert!(
        (cell.weighted_insufficient_rate - 2.7 / 6.2).abs() < 1e-9,
        "{}",
        cell.weighted_insufficient_rate
    );
    // Above the floor on the wrongs, below it on the insufficients: the
    // cell moves, and the move is the wrongs' doing alone.
    assert!(cell.informs_failover());
    assert!(!cell.notes_companions());
}

#[test]
fn a_disagreed_insufficient_row_is_the_ground_truth_that_the_companion_was_there() {
    // The owner's disagree on an insufficient verdict is exactly the
    // ground truth that says the companion WAS there: the minima was
    // reached, so the row stands on right — not on wrong, and not on the
    // automated insufficient. The beat is a signal; the owner is the
    // ground truth.
    let mut rows: Vec<DelegationJudgedRow> = (0..FIT_MIN_ROWS)
        .map(|index| {
            read_judged(
                "zai-coding-plan",
                "hard",
                calib::VERDICT_INSUFFICIENT,
                0.95 - index as f64 * 0.05,
                "",
            )
        })
        .collect();
    for row in rows.iter_mut().take(2) {
        row.disposition = Some(calib::DISPOSITION_DISAGREED.to_string());
    }
    let cells = calibration(&rows);
    let cell = &cells[0];
    assert_eq!(cell.insufficient, 6);
    assert_eq!(cell.right, 2);
    assert_eq!(cell.wrong, 0);
    assert_eq!(cell.overridden, 2);
    assert!(!cell.informs_failover());
}

#[test]
fn the_owners_disposition_overrides_the_automated_verdict_in_the_math() {
    // Eight confident wrongs: the cell informs the ride-back. The owner
    // disagrees with four of them, and the math follows the owner rather
    // than the lane.
    let mut rows: Vec<DelegationJudgedRow> = (0..FIT_MIN_ROWS)
        .map(|index| {
            read_judged(
                "zai-coding-plan",
                "hard",
                calib::VERDICT_WRONG,
                0.95 - index as f64 * 0.05,
                "",
            )
        })
        .collect();
    let automated = calibration(&rows);
    assert!(automated[0].informs_failover());
    for row in rows.iter_mut().take(4) {
        row.disposition = Some(calib::DISPOSITION_DISAGREED.to_string());
    }
    let overridden = calibration(&rows);
    assert_eq!(overridden[0].overridden, 4);
    // Four wrongs became rights: four wrong of eight, and the weighted
    // rate drops with them.
    assert_eq!(overridden[0].wrong, 4);
    assert_eq!(overridden[0].right, 4);
    assert!(!overridden[0].informs_failover());
    // A waived row is set aside entirely: neither right nor wrong, and
    // excluded from the rate.
    for row in rows.iter_mut().take(6) {
        row.disposition = Some(calib::DISPOSITION_WAIVED.to_string());
    }
    let waived = calibration(&rows);
    assert_eq!(waived[0].waived, 6);
    assert_eq!(waived[0].wrong, 2);
    assert_eq!(waived[0].right, 0);
    assert_eq!(
        waived[0].fit,
        FitState::Unfit,
        "six waived rows leave too little confidence span to fit"
    );
}

#[test]
fn an_unfit_cell_counts_its_insufficient_rows_toward_fit_and_sets_the_waived_aside() {
    // Six insufficient rows spanning 0.42: the cell is unfit on rows,
    // and it says it needs TWO more judged rows — not eight. The fit
    // rule counts every graded row, so `needs_rows` reads the graded
    // count; reading the right+wrong pair alone overstates the gap and
    // tells an operator to keep judging delegations the cell has already
    // graded.
    let rows: Vec<DelegationJudgedRow> = (0..6)
        .map(|index| {
            read_judged(
                "xiaomi-mimo",
                "hard",
                calib::VERDICT_INSUFFICIENT,
                0.95 - index as f64 * 0.07,
                "",
            )
        })
        .collect();
    let cells = calibration(&rows);
    let cell = &cells[0];
    assert_eq!(cell.insufficient, 6);
    assert_eq!(cell.right, 0);
    assert_eq!(cell.wrong, 0);
    assert_eq!(cell.fit, FitState::Unfit);
    assert_eq!(cell.needs_rows(), 2);

    // Six insufficient beside two waived is eight judged rows, and the
    // two counts now read apart: the day's holding is whole — eight
    // judged rows, `needs_rows` at zero — while the fit decision reads
    // the six graded verdicts and stays unfit. A waived row is a row
    // the owner set aside, so it counts toward what the day holds and
    // never toward the sample that earns a ride-back.
    let mut mixed = rows;
    mixed.extend([
        read_judged("xiaomi-mimo", "hard", calib::VERDICT_INSUFFICIENT, 0.95, ""),
        read_judged("xiaomi-mimo", "hard", calib::VERDICT_INSUFFICIENT, 0.88, ""),
    ]);
    for row in mixed.iter_mut().take(2) {
        row.disposition = Some(calib::DISPOSITION_WAIVED.to_string());
    }
    let cells = calibration(&mixed);
    let cell = &cells[0];
    assert_eq!(cell.judged, 8);
    assert_eq!(cell.insufficient, 6);
    assert_eq!(cell.waived, 2);
    // The display count keeps counting every graded row the day holds.
    assert_eq!(cell.needs_rows(), 0);
    // The fit decision excludes the waived, and the gap is named apart.
    assert_eq!(cell.graded_verdicts(), 6);
    assert_eq!(cell.needs_graded(), 2);
    assert_eq!(cell.fit, FitState::Unfit);
    // The waived pair still stays out of the rate, which is the whole of
    // the rate's own reading.
    assert!(
        (cell.weighted_insufficient_rate - 1.0).abs() < 1e-9,
        "{}",
        cell.weighted_insufficient_rate
    );
    // Unfit, so neither vector fires: no ride-back, and no companion
    // note either — the note is a fit cell's own read.
    assert!(!cell.informs_failover());
    assert!(!cell.notes_companions());
}

#[test]
fn a_cell_counts_its_zero_confidence_rows_and_names_the_reweighting_on_its_card() {
    // Four confident wrongs beside four rights that reported no
    // confidence at all: the rights weigh nothing, so the weighted
    // wrong-rate is 1.00 and the cell informs the ride-back — a shape
    // that was half right, moved. The reweighting is real (the lane's
    // confidence is the weight the whole math reads), and the cell NAMES
    // it on its card instead of leaving a reader to infer why the rate
    // is what it is.
    let mut rows: Vec<DelegationJudgedRow> = (0..4)
        .map(|index| {
            read_judged(
                "zai-coding-plan",
                "hard",
                calib::VERDICT_WRONG,
                0.95 - index as f64 * 0.05,
                "",
            )
        })
        .collect();
    rows.extend(
        (0..4).map(|_| read_judged("zai-coding-plan", "hard", calib::VERDICT_RIGHT, 0.0, "")),
    );
    let cells = calibration(&rows);
    let cell = &cells[0];
    assert_eq!(cell.right, 4);
    assert_eq!(cell.wrong, 4);
    assert_eq!(cell.zero_confidence, 4);
    assert_eq!(cell.fit, FitState::Fit);
    assert!(
        (cell.weighted_wrong_rate - 1.0).abs() < 1e-9,
        "{}",
        cell.weighted_wrong_rate
    );
    assert!(cell.informs_failover());

    // The count rides the card: the reweighting is visible, not inferred.
    let root = scratch("zero-confidence");
    let mut card_rows: Vec<serde_json::Value> = (0..4)
        .map(|index| {
            judged_value(&read_judged(
                "zai-coding-plan",
                "hard",
                calib::VERDICT_WRONG,
                0.95 - index as f64 * 0.05,
                "",
            ))
        })
        .collect();
    card_rows.extend([0.7, 0.6, 0.5].iter().map(|confidence| {
        judged_value(&read_judged(
            "zai-coding-plan",
            "hard",
            calib::VERDICT_RIGHT,
            *confidence,
            "",
        ))
    }));
    card_rows.push(judged_value(&read_judged(
        "zai-coding-plan",
        "hard",
        calib::VERDICT_RIGHT,
        0.0,
        "",
    )));
    write_day(&root, &day(), &card_rows);
    let rendered = render_calibration(&root, &day(), true);
    assert!(
        rendered.contains("1 of the graded rows reported no confidence"),
        "{rendered}"
    );
    for line in rendered.lines() {
        let width = plain(line).chars().count();
        assert!(width <= 72, "a line is {width} wide: {}", plain(line));
    }
    let _ = std::fs::remove_dir_all(&root);
}

// --- the batch-only proof ---

#[test]
fn no_synchronous_path_references_the_delegation_beat() {
    // The socket-guard test, in the sibling's shape: the synchronous
    // paths — the doctor's boot checks, the recall path, the tool loop,
    // the session log, the API client, the MCP client, the calibration
    // stream's writers and readers, the routing walk, the CLI parser,
    // and the TUI — never name this module. A beat that spends a lane
    // ask must be unreachable from a path that runs on every turn.
    //
    // `api.rs` and `mcp.rs` are in the scanned set on purpose: the
    // sibling's proof omitted them, and they are the two modules a beat
    // most plausibly reaches for from inside a turn.
    const MODULES: [(&str, &str); 27] = [
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
        ("skills.rs", include_str!("../skills.rs")),
        ("subagents.rs", include_str!("../subagents.rs")),
        ("api.rs", include_str!("../api.rs")),
        ("mcp.rs", include_str!("../mcp.rs")),
        ("lanes.rs", include_str!("../lanes.rs")),
        ("home.rs", include_str!("../home.rs")),
        ("install.rs", include_str!("../install.rs")),
        ("net.rs", include_str!("../net.rs")),
        ("cli.rs", include_str!("../cli.rs")),
        ("tui/mod.rs", include_str!("../tui/mod.rs")),
        ("tui/app.rs", include_str!("../tui/app.rs")),
        ("tui/ui/mod.rs", include_str!("../tui/ui/mod.rs")),
        ("tui/ui/run.rs", include_str!("../tui/ui/run.rs")),
        ("tui/events.rs", include_str!("../tui/events.rs")),
        ("main.rs", include_str!("../main.rs")),
        ("eval.rs", include_str!("../eval.rs")),
    ];
    // The beat's own legitimate in-crate callers, named rather than
    // inferred: the CLI dispatch in `main` and the graded scenario in
    // `eval`, plus the beat's own file and its tests. The guard above is
    // a source grep over a fixed module list, not a walk of the call
    // graph; what it proves is that no SYNCHRONOUS path names the beat,
    // and this names the ones that legitimately may.
    const DRIVERS: [&str; 4] = ["main.rs", "eval.rs", "delegate.rs", "delegate/tests.rs"];
    for (name, source) in MODULES {
        if DRIVERS.contains(&name) {
            continue;
        }
        // The control writer is not the beat: it writes the session's
        // control file and never spends a lane ask. The TUI's stop and
        // steer use it on the run path, so exactly that symbol is
        // allowed; every other name from the module stays refused.
        let source = source
            .lines()
            .filter(|line| !line.contains("delegate::write_control"))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            !source.contains("delegate::"),
            "{name} names the delegation beat"
        );
        assert!(
            !source.contains("crate::delegate"),
            "{name} names the delegation beat"
        );
        assert!(
            !source.contains("DELEGATE_BUDGET_ENV"),
            "{name} names the delegation beat"
        );
        assert!(
            !source.contains("DELEGATE_ESTIMATED_ASK_USD"),
            "{name} names the delegation beat"
        );
    }
    // The beat's two row shapes are constructed in exactly one module —
    // this one. Scan the reachable surface and every writer both:
    // nothing outside `delegate` and its tests builds one, so a second
    // writer of the beat's own rows cannot appear unnoticed.
    const BUILDERS: [(&str, &str); 26] = [
        ("calib.rs", include_str!("../calib.rs")),
        ("delegate.rs", include_str!("../delegate.rs")),
        ("delegate/tests.rs", include_str!("tests.rs")),
        ("main.rs", include_str!("../main.rs")),
        ("eval.rs", include_str!("../eval.rs")),
        ("doctor.rs", include_str!("../doctor.rs")),
        ("memory.rs", include_str!("../memory.rs")),
        ("agent.rs", include_str!("../agent.rs")),
        ("tools.rs", include_str!("../tools.rs")),
        ("session.rs", include_str!("../session.rs")),
        ("route.rs", include_str!("../route.rs")),
        ("compaction.rs", include_str!("../compaction.rs")),
        ("cron.rs", include_str!("../cron.rs")),
        ("improve.rs", include_str!("../improve.rs")),
        ("boards.rs", include_str!("../boards.rs")),
        ("skills.rs", include_str!("../skills.rs")),
        ("subagents.rs", include_str!("../subagents.rs")),
        ("api.rs", include_str!("../api.rs")),
        ("mcp.rs", include_str!("../mcp.rs")),
        ("lanes.rs", include_str!("../lanes.rs")),
        ("home.rs", include_str!("../home.rs")),
        ("install.rs", include_str!("../install.rs")),
        ("net.rs", include_str!("../net.rs")),
        ("cli.rs", include_str!("../cli.rs")),
        ("tui/mod.rs", include_str!("../tui/mod.rs")),
        ("tui/app.rs", include_str!("../tui/app.rs")),
    ];
    for (name, source) in BUILDERS {
        let allowed = name == "delegate.rs" || name == "delegate/tests.rs";
        assert!(
            allowed || !source.contains("DelegationJudgedRow {"),
            "{name} constructs a judged row — only delegate may"
        );
        assert!(
            allowed || !source.contains("DelegationRow {"),
            "{name} constructs a delegation row — only delegate may"
        );
    }
}

// --- the beat, end to end over the scripted lane ---

#[tokio::test]
async fn a_days_delegations_are_judged_beside_themselves_and_the_rows_land() {
    let home = scratch("judged-home");
    let root = scratch("judged-calib");
    let dir = sessions(&root);
    let mut row = delegation_row("child-1", "zai-coding-plan", "hard", OUTCOME_COMPLETED);
    row = with_log(row, &dir, "child-1");
    write_day(&root, &day(), &[delegation_value(&row)]);
    child_log(&dir, "child-1");
    parent_log(&dir, "parent-session", "child-1", false);
    let outcome = judge_over(&home, &root, &day(), false, &lane_reply("pass", 0.91)).await;

    assert!(outcome.refusal.is_none(), "{:?}", outcome.refusal);
    assert_eq!(outcome.judged.len(), 1, "{:?}", outcome.refusals);
    let judged = &outcome.judged[0];
    assert_eq!(judged.provider, "zai-coding-plan");
    assert_eq!(judged.verdict, calib::VERDICT_RIGHT);
    // The shape rides the row, so the calibration is keyed by
    // (provider × brief shape) rather than averaged per provider.
    assert_eq!(judged.brief_shape, "hard");
    assert!((judged.confidence - 0.91).abs() < 1e-9);
    assert_eq!(judged.lane, DELEGATE_LANE);
    // The key is the child's own session id, so the judgment joins the
    // delegation it graded.
    assert_eq!(judged.session, "child-1");
    // The row landed in the stream and reads back through the beat's own
    // reader, beside the delegation row it graded.
    let rows = read_day(&root, &day());
    assert_eq!(rows.delegations.len(), 1);
    assert_eq!(rows.judged.len(), 1);
    assert_eq!(rows.judged[0].session, "child-1");
    // The day's spend is recorded, and the calibration reads the row.
    assert!((day_spend(&root, &home, &day()) - 0.0002).abs() < 1e-9);
    let cells = calibration(&rows.judged);
    assert_eq!(cells.len(), 1);
    assert_eq!(cells[0].provider, "zai-coding-plan");
    assert_eq!(cells[0].brief_shape, "hard");
    assert_eq!(cells[0].judged, 1);
    // The card is a card, never a dump, and it names the key the owner
    // addresses.
    let text = plain(&outcome.card);
    assert!(text.contains("judged"), "{text}");
    assert!(text.contains("zai-coding-plan"), "{text}");
    assert!(text.contains("child-1"), "{text}");
    assert!(
        outcome.card.contains('\u{250c}') && outcome.card.contains('\u{2514}'),
        "{}",
        outcome.card
    );
    for line in outcome.card.lines() {
        let width = plain(line).chars().count();
        assert!(
            width <= CARD_WIDTH,
            "a card line is {width} wide: {}",
            plain(line)
        );
    }
    let _ = std::fs::remove_dir_all(&home);
    let _ = std::fs::remove_dir_all(&root);
}

#[tokio::test]
async fn a_second_batch_spends_nothing_on_a_delegation_it_already_judged() {
    let home = scratch("idempotent-home");
    let root = scratch("idempotent-calib");
    let dir = sessions(&root);
    let mut row = delegation_row("child-1", "zai-coding-plan", "hard", OUTCOME_COMPLETED);
    row = with_log(row, &dir, "child-1");
    write_day(&root, &day(), &[delegation_value(&row)]);
    child_log(&dir, "child-1");
    parent_log(&dir, "parent-session", "child-1", false);
    let first = judge_over(&home, &root, &day(), false, &lane_reply("pass", 0.91)).await;
    assert_eq!(first.judged.len(), 1);
    let second = judge_over(&home, &root, &day(), false, &lane_reply("pass", 0.91)).await;
    // No ask, no row: the batch is idempotent, and it says so by name.
    assert!(second.judged.is_empty());
    assert_eq!(second.refusals.len(), 1);
    assert!(
        second.refusals[0].1.contains("already judged"),
        "{}",
        second.refusals[0].1
    );
    assert_eq!(read_day(&root, &day()).judged.len(), 1);
    // The day's spend did not move.
    assert!((day_spend(&root, &home, &day()) - 0.0002).abs() < 1e-9);
    // An all-already-judged batch answered by confirming, so a
    // twice-daily cron is not a false alarm.
    assert!(second.batch_answered());
    let _ = std::fs::remove_dir_all(&home);
    let _ = std::fs::remove_dir_all(&root);
}

#[tokio::test]
async fn a_dry_run_spends_nothing_and_shows_what_would_be_asked() {
    let home = scratch("dry-home");
    let root = scratch("dry-calib");
    let dir = sessions(&root);
    let mut row = delegation_row("child-1", "zai-coding-plan", "hard", OUTCOME_COMPLETED);
    row = with_log(row, &dir, "child-1");
    write_day(&root, &day(), &[delegation_value(&row)]);
    child_log(&dir, "child-1");
    parent_log(&dir, "parent-session", "child-1", false);
    let outcome = judge_over(&home, &root, &day(), true, &lane_reply("pass", 0.91)).await;

    assert!(outcome.refusal.is_none(), "{:?}", outcome.refusal);
    // No ask, no row, no spend.
    assert!(outcome.judged.is_empty());
    assert!(!outcome.dry_run_prompts.is_empty());
    assert_eq!(
        read_day(&root, &day()).judged.len(),
        0,
        "only the delegation row is there"
    );
    assert!(day_spend(&root, &home, &day()).abs() < 1e-9);
    // The card shows what would be asked, keyed.
    let (key, prompt) = &outcome.dry_run_prompts[0];
    assert_eq!(key, "child-1");
    assert!(prompt.contains("DELEGATION DATA BEGIN"), "{prompt}");
    let text = plain(&outcome.card);
    assert!(text.contains("dry run"), "{text}");
    assert!(text.contains("no ask, no row, no spend"), "{text}");
    assert!(text.contains("budget"), "{text}");
    for line in outcome.card.lines() {
        let width = plain(line).chars().count();
        assert!(
            width <= CARD_WIDTH,
            "a card line is {width} wide: {}",
            plain(line)
        );
    }
    let _ = std::fs::remove_dir_all(&home);
    let _ = std::fs::remove_dir_all(&root);
}

#[tokio::test]
async fn a_day_with_no_delegation_rows_is_a_named_refusal() {
    let home = scratch("empty-home");
    let root = scratch("empty-calib");
    let outcome = judge_over(&home, &root, &day(), false, &lane_reply("pass", 0.91)).await;
    let refusal = outcome.refusal.clone().expect("a named refusal");
    assert!(refusal.contains("no `delegation` rows"), "{refusal}");
    assert!(outcome.judged.is_empty());
    assert!(
        plain(&outcome.card).contains("no `delegation` rows"),
        "{}",
        outcome.card
    );
    // An empty day is NO ANSWER, so the command's exit code says so.
    assert!(!outcome.batch_answered());
    let _ = std::fs::remove_dir_all(&home);
    let _ = std::fs::remove_dir_all(&root);
}

#[tokio::test]
async fn a_lane_that_cannot_answer_fails_open_naming_its_reason() {
    // A spec whose program does not exist is a named refusal, the
    // recorded delegation stands, and no judgment is fabricated.
    // Unconfigured, down, and past its bound all land here.
    let home = scratch("open-home");
    let root = scratch("open-calib");
    let dir = sessions(&root);
    let mut row = delegation_row("child-1", "zai-coding-plan", "hard", OUTCOME_COMPLETED);
    row = with_log(row, &dir, "child-1");
    write_day(&root, &day(), &[delegation_value(&row)]);
    child_log(&dir, "child-1");
    let lane = LaneSource::Scripted(ServerSpec {
        program: "/nonexistent/fleet-lane".to_string(),
        args: Vec::new(),
        env: Vec::new(),
        passthrough: Vec::new(),
        cwd: None,
    });
    let outcome = judge(JudgeRequest {
        root: &root,
        home: &home,
        day: day(),
        dry_run: false,
        lane,
    })
    .await;
    assert!(outcome.judged.is_empty());
    assert_eq!(outcome.refusals.len(), 1);
    assert!(
        outcome.refusals[0].1.contains("could not start"),
        "{}",
        outcome.refusals[0].1
    );
    // No row, and the stream still holds exactly the delegation row.
    assert_eq!(read_day(&root, &day()).judged.len(), 0);
    assert!(day_spend(&root, &home, &day()).abs() < 1e-9);
    assert!(
        plain(&outcome.card).contains("the recorded delegations stand"),
        "{}",
        outcome.card
    );
    let _ = std::fs::remove_dir_all(&home);
    let _ = std::fs::remove_dir_all(&root);
}

#[tokio::test]
async fn a_lane_that_fails_open_is_refused_by_name_not_shipped_as_a_judgment() {
    // The lane's own fail-open: `verdict: error`, no verdict to grade,
    // the lane's reason in the telemetry. The beat refuses by name
    // carrying the lane's error, treats the fallback as no answer — no
    // row — and the recorded delegation stands. This is the
    // consuming-lanes rule's scripted leg: a consumer verified only
    // against a healthy lane passes every test and ships the lane's
    // error as its product.
    let home = scratch("failopen-home");
    let root = scratch("failopen-calib");
    let dir = sessions(&root);
    let mut row = delegation_row("child-1", "zai-coding-plan", "hard", OUTCOME_COMPLETED);
    row = with_log(row, &dir, "child-1");
    write_day(&root, &day(), &[delegation_value(&row)]);
    child_log(&dir, "child-1");
    let fallback = serde_json::json!({
        "task": "route",
        "verdict": "error",
        "provider": null,
        "providers_used": [],
        "answers": {},
        "telemetry": { "glide": { "error": "FASTINO_API_KEY is not set", "error_kind": "provider" } },
        "summary": ["No provider succeeded — unclassified."],
        "latency_ms": 12,
        "reason": "FASTINO_API_KEY is not set",
    })
    .to_string();
    let outcome = judge_over(&home, &root, &day(), false, &fallback).await;
    assert!(outcome.judged.is_empty());
    assert_eq!(outcome.refusals.len(), 1);
    let reason = &outcome.refusals[0].1;
    assert!(reason.contains("failed open"), "{reason}");
    assert!(reason.contains("FASTINO_API_KEY"), "{reason}");
    // No row, and the stream still holds exactly the delegation row.
    assert_eq!(read_day(&root, &day()).judged.len(), 0);
    assert!(day_spend(&root, &home, &day()).abs() < 1e-9);
    assert!(
        plain(&outcome.card).contains("the recorded delegations stand"),
        "{}",
        outcome.card
    );
    let _ = std::fs::remove_dir_all(&home);
    let _ = std::fs::remove_dir_all(&root);
}

#[tokio::test]
async fn a_lanes_answer_with_no_confidence_is_refused_rather_than_graded_on_a_guessed_weight() {
    // The confidence is the weight the whole cell math reads. A reply
    // that reports a verdict but no confidence used to default to 0.0,
    // and a zero-confidence row weighs nothing: four confident wrongs
    // beside four zero-confidence rights read a 1.00 weighted
    // wrong-rate and move a suggestion on a shape that was half right.
    // The two guesses available are both wrong (0.0 hides the row, a
    // "neutral" 1.0 invents a confidence the lane never claimed), so the
    // answer is refused by name — no row, no influence — exactly as a
    // fail-open is.
    let home = scratch("noconf-home");
    let root = scratch("noconf-calib");
    let dir = sessions(&root);
    let mut row = delegation_row("child-1", "zai-coding-plan", "hard", OUTCOME_COMPLETED);
    row = with_log(row, &dir, "child-1");
    write_day(&root, &day(), &[delegation_value(&row)]);
    child_log(&dir, "child-1");
    let reply = serde_json::json!({
        "task": "route",
        "verdict": "pass",
        "provider": "glide",
        "providers_used": ["glide"],
        "answers": { "glide": { "workload": { "label": "hard" } } },
        "latency_ms": 900,
        "cost_usd": 0.0002,
    })
    .to_string();
    let outcome = judge_over(&home, &root, &day(), false, &reply).await;

    assert!(outcome.judged.is_empty());
    assert_eq!(outcome.refusals.len(), 1);
    let reason = &outcome.refusals[0].1;
    assert!(reason.contains("no confidence"), "{reason}");
    // No row, and the stream still holds exactly the delegation row.
    assert_eq!(read_day(&root, &day()).judged.len(), 0);
    // The batch did not answer, so the command's exit code says so.
    assert!(!outcome.batch_answered());
    assert!(
        plain(&outcome.card).contains("the recorded delegations stand"),
        "{}",
        outcome.card
    );
    let _ = std::fs::remove_dir_all(&home);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_batch_that_judged_nothing_did_not_answer_and_the_command_says_so() {
    // The exit-code half of the consuming-lanes rule: a leg that could
    // not answer is NO ANSWER — a named refusal, no row, the recorded
    // delegation standing, and a NONZERO EXIT, so whatever ran the batch
    // (a cron, an owner) learns the judgments did not land. A silent
    // success would ship the lane's failure as the batch's result. The
    // card names every refusal; the exit code is the machine-readable
    // half of the same refusal.
    let refused = JudgeOutcome {
        card: String::new(),
        judged: Vec::new(),
        refusals: vec![(
            "child-1".to_string(),
            "the lane failed open and its fallback is not a judgment: FASTINO_API_KEY is not set"
                .to_string(),
        )],
        refusal: None,
        dry_run_prompts: Vec::new(),
        spent: 0.0,
    };
    assert!(!refused.batch_answered());

    // A dry run answered by rehearsing: no ask, no row, no spend, and
    // the exit code says the rehearsal happened.
    let rehearsed = JudgeOutcome {
        card: String::new(),
        judged: Vec::new(),
        refusals: Vec::new(),
        refusal: None,
        dry_run_prompts: vec![("child-1".to_string(), "DELEGATION DATA BEGIN".to_string())],
        spent: 0.0,
    };
    assert!(rehearsed.batch_answered());

    // A batch whose every delegation was already judged answered by
    // confirming: an idempotent second run spends nothing and has
    // nothing new to report, which is not a failure.
    let idempotent = JudgeOutcome {
        card: String::new(),
        judged: Vec::new(),
        refusals: vec![(
            "child-1".to_string(),
            "already judged in this day's stream — the batch is idempotent and spends nothing on a \
             delegation it has already graded"
                .to_string(),
        )],
        refusal: None,
        dry_run_prompts: Vec::new(),
        spent: 0.0,
    };
    assert!(idempotent.batch_answered());

    // The batch-level refusals did not answer: an empty day, and a day
    // whose budget refused the first ask.
    let empty = JudgeOutcome {
        card: String::new(),
        judged: Vec::new(),
        refusals: Vec::new(),
        refusal: Some("no `delegation` rows for 2026-10-15 in the stream".to_string()),
        dry_run_prompts: Vec::new(),
        spent: 0.0,
    };
    assert!(!empty.batch_answered());
    let over_budget = JudgeOutcome {
        card: String::new(),
        judged: Vec::new(),
        refusals: vec![(
            "child-1".to_string(),
            "over the $0.05 daily budget".to_string(),
        )],
        refusal: Some("the day's delegate spend is $0.0500".to_string()),
        dry_run_prompts: Vec::new(),
        spent: 0.05,
    };
    assert!(!over_budget.batch_answered());
}

#[tokio::test]
async fn an_over_budget_day_is_refused_naming_the_spend_on_a_mixed_day() {
    // The budget's arithmetic on a mixed day, pinned: three delegation
    // rows, none judged, and a scripted ask that costs $0.049 each.
    //   row 1: 0.0000 + 0.0002 reserved = 0.0002, at or under 0.05 → asked
    //   row 2: 0.0490 + 0.0002 reserved = 0.0492, at or under 0.05 → asked
    //   row 3: 0.0980 + 0.0002 reserved = 0.0982, over 0.05 → refused by name
    // The cap holds to within one ask's real cost, and the reservation
    // is the only number available before the ask: the day's total can
    // cross the cap by at most one ask's real cost.
    let home = scratch("budget-home");
    let root = scratch("budget-calib");
    let dir = sessions(&root);
    let mut rows: Vec<serde_json::Value> = Vec::new();
    for index in 0..3 {
        let session = format!("child-{index}");
        let mut row = delegation_row(&session, "zai-coding-plan", "hard", OUTCOME_COMPLETED);
        row = with_log(row, &dir, &session);
        rows.push(delegation_value(&row));
        child_log(&dir, &session);
        parent_log(&dir, "parent-session", &session, false);
    }
    write_day(&root, &day(), &rows);
    let reply = serde_json::json!({
        "task": "route",
        "verdict": "pass",
        "provider": "glide",
        "providers_used": ["glide"],
        "answers": { "glide": { "workload": { "label": "hard", "confidence": 0.91 } } },
        "latency_ms": 900,
        "cost_usd": 0.049,
    })
    .to_string();
    let outcome = judge_over(&home, &root, &day(), false, &reply).await;

    // Two judged, one refused, and the refusal names both numbers.
    assert_eq!(outcome.judged.len(), 2, "{:?}", outcome.refusals);
    assert_eq!(outcome.refusals.len(), 1, "{:?}", outcome.refusals);
    let reason = &outcome.refusals[0].1;
    assert!(reason.contains("daily budget"), "{reason}");
    assert!(reason.contains("$0.0980"), "{reason}");
    assert!(
        reason.contains(&format!("${DELEGATE_BUDGET_USD_DAY:.2}")),
        "{reason}"
    );
    // The batch-level refusal is set, so the command's exit code says
    // the day's remaining delegations stand unjudged.
    assert!(outcome.refusal.is_some());
    // The day's spend is the two asks that landed.
    assert!((day_spend(&root, &home, &day()) - 0.098).abs() < 1e-9);
    // A dry run still rehearses: it spends nothing, so the budget never
    // refuses it.
    let dry = judge_over(&home, &root, &day(), true, &reply).await;
    assert!(dry.refusal.is_none(), "{:?}", dry.refusal);
    assert!(dry.judged.is_empty());
    assert_eq!(dry.dry_run_prompts.len(), 1);
    assert!(dry.batch_answered());
    let _ = std::fs::remove_dir_all(&home);
    let _ = std::fs::remove_dir_all(&root);
}

#[tokio::test]
async fn a_row_the_writer_could_not_land_records_its_spend_to_the_sidecar_and_refuses_by_name() {
    // The sidecar's failed branch, asserted rather than commented: the
    // ask spent, the row could not reach the stream, and the spend still
    // lands on the home-side ledger — a cap that only moves on success
    // lets a broken writer spend uncapped. The refusal names the
    // sidecar's own path, and the day's spend reads the cost back for
    // the day.
    let home = scratch("sidecar-home");
    let root = scratch("sidecar-calib");
    let dir = sessions(&root);
    let mut row = delegation_row("child-1", "zai-coding-plan", "hard", OUTCOME_COMPLETED);
    row = with_log(row, &dir, "child-1");
    write_day(&root, &day(), &[delegation_value(&row)]);
    child_log(&dir, "child-1");
    // The day's own file loses its write bit: the reader still sees the
    // delegation row — the ask still runs — and every append the writer
    // attempts fails.
    let day_file = calib::day_path(&root, &day());
    let mut permissions = std::fs::metadata(&day_file).unwrap().permissions();
    std::os::unix::fs::PermissionsExt::set_mode(&mut permissions, 0o444);
    std::fs::set_permissions(&day_file, permissions).unwrap();
    let outcome = judge_over(&home, &root, &day(), false, &lane_reply("pass", 0.91)).await;
    // The write bit comes back, so the scratch directory can be removed.
    let mut permissions = std::fs::metadata(&day_file).unwrap().permissions();
    std::os::unix::fs::PermissionsExt::set_mode(&mut permissions, 0o644);
    std::fs::set_permissions(&day_file, permissions).unwrap();

    assert!(outcome.judged.is_empty(), "{:?}", outcome.refusals);
    assert_eq!(outcome.refusals.len(), 1);
    let reason = &outcome.refusals[0].1;
    assert!(reason.contains("could not be recorded"), "{reason}");
    assert!(
        reason.contains(
            &calib::spend_sidecar(&home, SPEND_LEDGER)
                .display()
                .to_string()
        ),
        "{reason}"
    );
    // No judged row landed — the stream still holds exactly the
    // delegation row — and the spend is on the home-side ledger for the
    // day, so the day's cap still moves on money the lane already spent.
    assert_eq!(read_day(&root, &day()).judged.len(), 0);
    assert!((calib::sidecar_spend(&home, SPEND_LEDGER, &day()) - 0.0002).abs() < 1e-9);
    assert!((day_spend(&root, &home, &day()) - 0.0002).abs() < 1e-9);
    // The batch did not answer, so the command's exit code says so.
    assert!(!outcome.batch_answered());
    assert!(
        plain(&outcome.card).contains("the recorded delegations stand"),
        "{}",
        outcome.card
    );
    let _ = std::fs::remove_dir_all(&home);
    let _ = std::fs::remove_dir_all(&root);
}

// --- the human seam ---

#[test]
fn a_disposition_lands_as_a_later_row_and_overrides_the_math() {
    let root = scratch("seam-calib");
    write_day(
        &root,
        &day(),
        &[judged_value(&judged_row(
            "child-1",
            "zai-coding-plan",
            "hard",
            calib::VERDICT_WRONG,
            0.91,
        ))],
    );
    let row = disposition(&root, &day(), "child-1", calib::DISPOSITION_DISAGREED)
        .expect("the disposition landed");
    // The verdict the lane wrote rides through unchanged; the
    // disposition is the later row's own claim.
    assert_eq!(row.verdict, calib::VERDICT_WRONG);
    assert_eq!(
        row.disposition.as_deref(),
        Some(calib::DISPOSITION_DISAGREED)
    );
    assert_eq!(row.session, "child-1");
    assert_eq!(row.provider, "zai-coding-plan");
    assert_eq!(row.brief_shape, "hard");
    // The stream holds both rows, and the reader takes the freshest.
    let rows = read_day(&root, &day());
    assert_eq!(rows.judged.len(), 2);
    let freshest = judged_rows(&rows.judged);
    assert_eq!(freshest.len(), 1);
    assert_eq!(
        freshest[0].disposition.as_deref(),
        Some(calib::DISPOSITION_DISAGREED)
    );
    // The math follows the owner: a disagreed wrong is a right.
    let cells = calibration(&rows.judged);
    assert_eq!(cells[0].right, 1);
    assert_eq!(cells[0].wrong, 0);
    assert_eq!(cells[0].overridden, 1);
    assert_eq!(cells[0].brief_shape, "hard");
    // The card names the disposition and says what it overrides.
    let text = plain(&disposition_card(&row, &day()));
    assert!(text.contains("disagreed"), "{text}");
    assert!(text.contains("ground truth"), "{text}");
    assert!(text.contains("hard"), "{text}");
    for line in disposition_card(&row, &day()).lines() {
        let width = plain(line).chars().count();
        assert!(
            width <= CARD_WIDTH,
            "a card line is {width} wide: {}",
            plain(line)
        );
    }
    // `day_keys` lets a surface address one: the key, the provider, and
    // the verdict that stands.
    assert_eq!(
        day_keys(&root, &day()),
        vec![(
            "child-1".to_string(),
            "zai-coding-plan".to_string(),
            calib::VERDICT_WRONG.to_string()
        )]
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_disposition_whose_key_names_no_judged_row_is_a_named_refusal() {
    let root = scratch("seam-miss");
    let error = disposition(&root, &day(), "child-404", calib::DISPOSITION_AGREED)
        .expect_err("a named refusal");
    assert!(error.contains("child-404"), "{error}");
    assert!(error.contains("delegation_judged"), "{error}");
    let text = plain(&disposition_refusal_card(&day(), &error));
    assert!(text.contains("refused"), "{text}");
    assert!(text.contains("child-404"), "{text}");
    let _ = std::fs::remove_dir_all(&root);
}

// --- the conservative ride-back ---

#[test]
fn the_ride_back_moves_only_a_fit_cells_wrong_rate_and_names_the_shape() {
    // The conservative ride-back, in the sibling's shape: a fit cell
    // whose confidence-weighted WRONG-rate crosses the floor moves the
    // suggestion among the remaining candidates, the insufficient-rate
    // never moves anything, and both are shape-scoped — a shape-Y
    // record never answers a shape-X question.
    let root = scratch("ride-back");
    let day = day();
    let mut rows: Vec<serde_json::Value> = (0..FIT_MIN_ROWS)
        .map(|index| {
            judged_value(&read_judged(
                "zai-coding-plan",
                "hard",
                calib::VERDICT_WRONG,
                0.95 - index as f64 * 0.05,
                "",
            ))
        })
        .collect();
    rows.extend((0..FIT_MIN_ROWS).map(|index| {
        judged_value(&read_judged(
            "zai-coding-plan",
            "prose",
            calib::VERDICT_INSUFFICIENT,
            0.95 - index as f64 * 0.05,
            "",
        ))
    }));
    write_day(&root, &day, &rows);

    // The hard cell's wrong-rate crosses the floor, so a suggestion for
    // `hard` moves among the remaining candidates.
    let skips = delegation_skips(&root, &day, &["zai-coding-plan"], "hard");
    assert_eq!(skips.len(), 1, "{skips:?}");
    assert_eq!(skips[0].brief_shape, "hard");
    assert!(skips[0].informs_failover());
    // The insufficient-rate never moves anything: a suggestion for
    // `prose` skips no one, however high the insufficient-rate runs.
    assert!(delegation_skips(&root, &day, &["zai-coding-plan"], "prose").is_empty());
    // The notes are the third vector's own read, and they are the
    // insufficient cell's alone.
    let notes = delegation_notes(&root, &day, &["zai-coding-plan"], "prose");
    assert_eq!(notes.len(), 1, "{notes:?}");
    assert!(notes[0].notes_companions());
    assert!(!notes[0].informs_failover());
    assert!(delegation_notes(&root, &day, &["zai-coding-plan"], "hard").is_empty());
    // A candidate the day holds no cell for is never moved, and a shape
    // the day holds no cell for reads as no cell at all.
    assert!(delegation_skips(&root, &day, &["xiaomi-mimo"], "hard").is_empty());
    assert!(delegation_skips(&root, &day, &["zai-coding-plan"], "quick").is_empty());
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_cell_whose_rows_are_mostly_waived_never_earns_its_ride_back() {
    // Seven wrong rows at full spread beside two the owner waived: nine
    // judged rows on the day, and only seven of them carry a verdict.
    // The fit decision reads the seven, so the cell is unfit however
    // wrong it looks and however much the day holds — the conservative
    // direction, because a ride-back moves work and a waived row is a
    // row nobody stands behind.
    let root = scratch("mostly-waived");
    let day = day();
    let mut rows: Vec<serde_json::Value> = (0..7)
        .map(|index| {
            judged_value(&read_judged(
                "zai-coding-plan",
                "hard",
                calib::VERDICT_WRONG,
                0.95 - index as f64 * 0.06,
                "",
            ))
        })
        .collect();
    rows.extend([
        judged_value(&read_judged(
            "zai-coding-plan",
            "hard",
            calib::VERDICT_WRONG,
            0.90,
            calib::DISPOSITION_WAIVED,
        )),
        judged_value(&read_judged(
            "zai-coding-plan",
            "hard",
            calib::VERDICT_WRONG,
            0.90,
            calib::DISPOSITION_WAIVED,
        )),
    ]);
    write_day(&root, &day, &rows);
    let cells = calibration(&read_day(&root, &day).judged);
    let cell = &cells[0];
    // What the day holds: nine judged rows, two of them set aside.
    assert_eq!(cell.judged, 9);
    assert_eq!(cell.waived, 2);
    assert_eq!(cell.wrong, 7);
    // What the fit reads: the seven graded verdicts, and the gap named
    // apart from the day's whole holding.
    assert_eq!(cell.graded_verdicts(), 7);
    assert_eq!(cell.needs_rows(), 0);
    assert_eq!(cell.needs_graded(), 1);
    assert_eq!(cell.fit, FitState::Unfit);
    // Every graded row is wrong and the rate says so — and it still
    // moves nothing, because the fit requirement outranks the number.
    assert!(
        (cell.weighted_wrong_rate - 1.0).abs() < 1e-9,
        "{}",
        cell.weighted_wrong_rate
    );
    assert!(!cell.informs_failover());
    // The card names the split rather than leaving the two counts to
    // read as a contradiction of each other.
    let rendered = render_calibration(&root, &day, true);
    assert!(
        rendered.contains("fit counts 7 graded verdicts of 9 judged \u{b7} 2 waived set aside"),
        "{rendered}"
    );
    assert!(rendered.contains("1 more graded verdict"), "{rendered}");
    assert!(!rendered.contains("more judged row"), "{rendered}");
    assert!(
        rendered.contains("no score is drawn from this sample"),
        "{rendered}"
    );

    // The same day with the owner's waiver lifted: nine graded verdicts,
    // the same wrong-rate, and the cell earns its ride-back. The waived
    // pair was the only thing holding it back.
    let mut standing = rows;
    for row in standing.iter_mut().skip(7) {
        row.as_object_mut()
            .unwrap()
            .remove("disposition")
            .expect("the waived pair's disposition");
    }
    write_day(&root, &day, &standing);
    let cells = calibration(&read_day(&root, &day).judged);
    let cell = &cells[0];
    assert_eq!(cell.judged, 9);
    assert_eq!(cell.waived, 0);
    assert_eq!(cell.graded_verdicts(), 9);
    assert_eq!(cell.fit, FitState::Fit);
    assert!(cell.informs_failover());
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn the_beat_renders_what_the_calibration_would_move_beside_the_graded_rows() {
    // The cells' named consumer: `fleet delegate` renders what the
    // calibration would move for the day's shapes — which cell, for
    // which shape, with the clause that names it. Driven through the
    // real render path over a scripted day's rows.
    let root = scratch("ride-back-render");
    let day = day();
    let mut rows: Vec<serde_json::Value> = (0..FIT_MIN_ROWS)
        .map(|index| {
            judged_value(&read_judged(
                "zai-coding-plan",
                "hard",
                calib::VERDICT_WRONG,
                0.95 - index as f64 * 0.05,
                "",
            ))
        })
        .collect();
    rows.extend((0..FIT_MIN_ROWS).map(|index| {
        judged_value(&read_judged(
            "zai-coding-plan",
            "prose",
            calib::VERDICT_INSUFFICIENT,
            0.95 - index as f64 * 0.05,
            "",
        ))
    }));
    // A third shape that is fit and right: it earns nothing, and the
    // section must not name it.
    rows.extend((0..FIT_MIN_ROWS).map(|index| {
        judged_value(&read_judged(
            "zai-coding-plan",
            "triage",
            calib::VERDICT_RIGHT,
            0.95 - index as f64 * 0.05,
            "",
        ))
    }));
    write_day(&root, &day, &rows);

    let rendered = render_ride_back_day(&root, &day, &["zai-coding-plan"], true);
    // The card wraps its clauses on a character count, so the assertions
    // read the section through `prose`: the wrap splits a line and never
    // drops a character, so the words survive it exactly and the
    // assertion reads what an operator reads.
    let text = prose(&rendered);
    // The shape that earned the move, named with the router's own
    // clause vocabulary: shape, weighted wrong-rate, judged count,
    // confidence span.
    assert!(
        text.contains(&prose(
            "would move the suggestion past zai-coding-plan for shape `hard`"
        )),
        "{text}"
    );
    assert!(text.contains(&prose("judged wrong at 1.00")), "{text}");
    assert!(
        text.contains(&prose(
            "a fit cell of 8 graded verdicts from 8 judged rows (span 0.35)"
        )),
        "{text}"
    );
    // The third vector rides beside it and moves no one.
    assert!(text.contains(&prose("would name")), "{text}");
    assert!(
        text.contains(&prose("names zai-coding-plan for shape `prose`")),
        "{text}"
    );
    assert!(text.contains(&prose("1.00 insufficient")), "{text}");
    assert!(
        text.contains(&prose("the direction is right and nothing is moved")),
        "{text}"
    );
    // And nothing for the shape that earned nothing.
    assert!(!text.contains(&prose("for shape `triage`")), "{text}");
    // A card in the house shape, never a plain-text table, and every
    // line inside the house width.
    assert!(rendered.contains('\u{250c}'), "{rendered}");
    assert!(!rendered.contains("| id |"), "{rendered}");
    for line in rendered.lines() {
        let width = plain(line).chars().count();
        assert!(
            width <= CARD_WIDTH,
            "a line is {width} wide: {}",
            plain(line)
        );
    }

    // The gate off says the ride-back is off rather than rendering an
    // influence that will not happen.
    let gated = plain(&render_ride_back_day(
        &root,
        &day,
        &["zai-coding-plan"],
        false,
    ));
    assert!(
        gated.contains("gate delegation.informs_failover off"),
        "{gated}"
    );
    assert!(!gated.contains("would move the suggestion past"), "{gated}");

    // A day whose cells earn nothing says so rather than rendering an
    // empty section: one fit-and-right shape, and nothing to move.
    let empty = scratch("ride-back-empty");
    write_day(
        &empty,
        &day,
        &[judged_value(&read_judged(
            "zai-coding-plan",
            "triage",
            calib::VERDICT_RIGHT,
            0.90,
            "",
        ))],
    );
    let nothing = plain(&render_ride_back_day(
        &empty,
        &day,
        &["zai-coding-plan"],
        true,
    ));
    assert!(
        nothing.contains("the calibration would move nothing"),
        "{nothing}"
    );
    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(&empty);
}

// --- the surface's shape ---

#[test]
fn the_calibration_surface_cards_the_cells_and_names_the_fit_state_and_the_floor() {
    let root = scratch("surface");
    let day = day();
    // One provider, two shapes: a fit-and-wrong `hard` cell and a
    // fit-and-right `prose` cell, plus an unfit `quick` cell and a
    // fit-and-insufficient `prose` cell on a second provider.
    let mut rows: Vec<serde_json::Value> = (0..FIT_MIN_ROWS)
        .map(|index| {
            judged_value(&read_judged(
                "zai-coding-plan",
                "hard",
                calib::VERDICT_WRONG,
                0.95 - index as f64 * 0.05,
                "",
            ))
        })
        .collect();
    rows.extend((0..FIT_MIN_ROWS).map(|index| {
        judged_value(&read_judged(
            "zai-coding-plan",
            "prose",
            calib::VERDICT_RIGHT,
            0.95 - index as f64 * 0.05,
            "",
        ))
    }));
    rows.push(judged_value(&read_judged(
        "xiaomi-mimo",
        "quick",
        calib::VERDICT_RIGHT,
        0.8,
        "",
    )));
    rows.extend((0..FIT_MIN_ROWS).map(|index| {
        judged_value(&read_judged(
            "xiaomi-mimo",
            "prose",
            calib::VERDICT_INSUFFICIENT,
            0.95 - index as f64 * 0.05,
            "",
        ))
    }));
    write_day(&root, &day, &rows);
    let rendered = render_calibration(&root, &day, true);
    // One card per cell, never a plain-text table and never a single
    // averaged per-provider score.
    assert_eq!(rendered.matches('\u{250c}').count(), 4, "{rendered}");
    assert!(!rendered.contains("| id |"), "{rendered}");
    // The cells are named by provider AND brief shape.
    assert!(
        rendered.contains("zai-coding-plan \u{b7} hard"),
        "{rendered}"
    );
    assert!(
        rendered.contains("zai-coding-plan \u{b7} prose"),
        "{rendered}"
    );
    assert!(rendered.contains("xiaomi-mimo \u{b7} quick"), "{rendered}");
    // The fit cell's score, and the unfit one's needs — and never a
    // score for the unfit one.
    assert!(
        rendered.contains("confidence-weighted wrong-rate"),
        "{rendered}"
    );
    assert!(rendered.contains("unfit"), "{rendered}");
    assert!(rendered.contains("more judged row"), "{rendered}");
    assert!(
        rendered.contains("no score is drawn from this sample"),
        "{rendered}"
    );
    // The ride-back is shape-scoped on the card, not a global claim.
    assert!(
        rendered.contains("informs the ride-back for shape `hard` only"),
        "{rendered}"
    );
    // The third state renders as its own named number and its own rate,
    // never folded into the wrong-rate, and the companion note names
    // what the minima needs without moving anyone.
    assert!(
        rendered.contains("0 right \u{b7} 0 wrong \u{b7} 8 insufficient"),
        "{rendered}"
    );
    assert!(
        rendered.contains("confidence-weighted insufficient-rate"),
        "{rendered}"
    );
    assert!(
        rendered.contains("the minima needs companions"),
        "{rendered}"
    );
    assert!(rendered.contains("nothing is moved"), "{rendered}");
    // The fit rule's thresholds and the ride-back's scope are named.
    assert!(
        rendered.contains(&format!("floor {:.2}", WRONG_RATE_FLOOR)),
        "{rendered}"
    );
    assert!(
        rendered.contains(&format!("{} rows", FIT_MIN_ROWS)),
        "{rendered}"
    );
    assert!(
        rendered.contains("gate delegation.informs_failover on"),
        "{rendered}"
    );
    assert!(
        rendered.contains("the declared chain's order and its head are untouched"),
        "{rendered}"
    );
    assert!(
        rendered.contains("the ride-back moves only a suggestion among the remaining candidates")
            && rendered.contains("and only for the brief shape's own cell"),
        "{rendered}"
    );
    // The gate reads off when the owner turned it off.
    let gated = render_calibration(&root, &day, false);
    assert!(
        gated.contains("gate delegation.informs_failover off"),
        "{gated}"
    );
    // Every drawn line fits the house width, so nothing clips at 80
    // columns and the cards read the same at 120.
    for line in rendered.lines() {
        let width = plain(line).chars().count();
        assert!(width <= 72, "a line is {width} wide: {}", plain(line));
    }
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_day_with_no_judged_rows_names_the_command_that_makes_them() {
    let root = scratch("surface-empty");
    let day = day();
    let rendered = render_calibration(&root, &day, true);
    assert!(
        rendered.contains(&format!("no judged delegations for {day}")),
        "{rendered}"
    );
    assert!(rendered.contains("fleet delegate"), "{rendered}");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn an_insufficient_row_renders_as_insufficient_and_never_as_right() {
    // The iteration cap's residue, and the round's top finding on the
    // sibling: a card whose per-row line mapped anything that was not
    // `wrong` to a green `right` printed `right <provider>` in green for
    // every insufficient row — directly above the same card's
    // `0 right · 0 wrong · 8 insufficient` tally. The card contradicted
    // itself and misreported the loop's own output. The line reads the
    // one verdict mapping all three states carry.
    let row = DelegationJudgedRow {
        ts: calib::stamp(),
        session: "child-1".to_string(),
        provider: "xiaomi-mimo".to_string(),
        brief_shape: "hard".to_string(),
        verdict: calib::VERDICT_INSUFFICIENT.to_string(),
        confidence: 0.88,
        reason: "the child stopped at max-turns: the direction was right and the minima was not \
                 reached for want of a companion (a longer turn budget)"
            .to_string(),
        disposition: None,
        cost_usd: Some(0.0002),
        latency_ms: Some(900),
        lane: DELEGATE_LANE.to_string(),
    };
    let card = batch_card(
        "2026-10-14",
        &[row],
        &[],
        None,
        &[],
        0.0002,
        DELEGATE_BUDGET_USD_DAY,
    );
    let text = plain(&card);
    // The word, beside the provider and the shape it was judged for.
    assert!(
        text.contains("insufficient xiaomi-mimo for shape hard"),
        "{text}"
    );
    // Never the green word, on the row's line or anywhere in the card.
    assert!(!text.contains("right xiaomi-mimo"), "{text}");
    // The card's own tally still says the same thing, so the two halves
    // of the card agree.
    assert!(
        text.contains("0 right \u{b7} 0 wrong \u{b7} 1 insufficient"),
        "{text}"
    );
    // Amber — the third state's own colour, the same escape the
    // disposition card paints it with.
    assert!(
        card.contains(&format!(
            "{}{}insufficient{}",
            palette::BOLD,
            palette::WARN,
            palette::RESET
        )),
        "{}",
        card
    );
    for line in card.lines() {
        let width = plain(line).chars().count();
        assert!(
            width <= CARD_WIDTH,
            "a card line is {width} wide: {}",
            plain(line)
        );
    }
}

#[test]
fn a_card_with_unbounded_fields_stays_inside_the_house_width() {
    // A provider id, a session id, and a reason the operator never
    // bounded are cut at the house width, not run past it — and the
    // LONGEST verdict word the vocabulary carries is the case that
    // proves it, because the row line's room for its two ids shrinks as
    // the word grows.
    let mut row = DelegationJudgedRow {
        ts: calib::stamp(),
        session: "a-session-id-that-no-one-ever-bounded-".repeat(3),
        provider: "a-very-long-provider-id-that-no-one-ever-bounded-".repeat(3),
        brief_shape: "a-brief-shape-nobody-bounded-either".repeat(2),
        verdict: calib::VERDICT_WRONG.to_string(),
        confidence: 0.91,
        reason: "a reason the lane reported that is far too long to fit on one line ".repeat(6),
        disposition: None,
        cost_usd: Some(0.0002),
        latency_ms: Some(900),
        lane: DELEGATE_LANE.to_string(),
    };
    let card = batch_card(
        "2026-10-14",
        &[row.clone()],
        &[],
        None,
        &[],
        0.0002,
        DELEGATE_BUDGET_USD_DAY,
    );
    for line in card.lines() {
        let width = plain(line).chars().count();
        assert!(
            width <= CARD_WIDTH,
            "a card line is {width} wide: {}",
            plain(line)
        );
    }
    // The same two unbounded ids under the longest word: the ids' tails
    // are what a long line costs, never the width.
    row.verdict = calib::VERDICT_INSUFFICIENT.to_string();
    let card = batch_card(
        "2026-10-14",
        &[row.clone()],
        &[],
        None,
        &[],
        0.0002,
        DELEGATE_BUDGET_USD_DAY,
    );
    for line in card.lines() {
        let width = plain(line).chars().count();
        assert!(
            width <= CARD_WIDTH,
            "a card line is {width} wide: {}",
            plain(line)
        );
    }
    // And the disposition card draws the same line, so it holds there
    // too.
    row.disposition = Some(calib::DISPOSITION_DISAGREED.to_string());
    for line in disposition_card(&row, "2026-10-14").lines() {
        let width = plain(line).chars().count();
        assert!(
            width <= CARD_WIDTH,
            "a card line is {width} wide: {}",
            plain(line)
        );
    }
    // The refusal card carries an unbounded reason too.
    for line in disposition_refusal_card(
        "2026-10-14",
        &"a refusal reason nobody bounded that runs on and on and on ".repeat(6),
    )
    .lines()
    {
        let width = plain(line).chars().count();
        assert!(
            width <= CARD_WIDTH,
            "a card line is {width} wide: {}",
            plain(line)
        );
    }
}
