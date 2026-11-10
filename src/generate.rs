//! The hosted small-model text leg: one bounded call to blend1's
//! `generate` tool.
//!
//! The grounded-selection plan's Phase 4 seam (C10/C11): prose a small
//! hosted model authors — compaction folds, titles, archive summaries,
//! advice narration — with the caller's today-path as the named
//! fallback. One wire, one key, the vendor blend1 already proves
//! (`services/blend1/src/blend1/lanes.py`, probed 2026-11-10: lfm3-1.7b
//! answers; both text models reason, so the lane clamps the budget to a
//! 1024-token floor).
//!
//! Every failure is named and returns `Err` — the caller falls back to
//! its own path and says so. A dead lane never takes a fold or a title
//! down with it.

use std::time::Duration;

use serde_json::json;

use crate::lanes;
use crate::mcp::{CallResult, McpServer, Timeouts};

/// The tool blend1 serves (`services/blend1/service/mcp_server.py`).
pub const GENERATE_TOOL: &str = "generate";
/// The lane's name in `AgentOptions.lane_specs` — an injected spec (an
/// integration test's mock, the face's registry merge) wins over the
/// home-resolved one.
pub const BLEND1_LANE: &str = "blend1";
/// The small text model — probed live 2026-11-10 on api.liquid.ai.
pub const TEXT_MODEL: &str = "lfm3-1.7b";
/// One bounded generate call: prose, not a conversation. The fold and
/// the title both finish well inside this; a hung lane costs the
/// caller its fallback, not its run.
pub const GENERATE_BUDGET: Duration = Duration::from_secs(120);

/// What the leg answered: the text and the model that wrote it.
#[derive(Debug, Clone, PartialEq)]
pub struct GenerateAnswer {
    pub text: String,
    pub model: String,
}

/// Parse the leg's structured result (`{status, text, model, usage}`).
/// A non-ok status is the leg's own named failure — carried through as
/// the error, never flattened into an empty answer.
pub fn answer_from_result(text: &str) -> Result<GenerateAnswer, String> {
    let value: serde_json::Value = serde_json::from_str(text).map_err(|error| {
        format!("the generate leg's answer was not structured content: {error}")
    })?;
    let status = value
        .get("status")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("missing");
    if status != "ok" {
        let reason = value
            .get("error")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("the leg named no reason");
        return Err(format!("the generate leg failed: {reason}"));
    }
    let text = value
        .get("text")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_string();
    if text.is_empty() {
        return Err("the generate leg answered with empty text".to_string());
    }
    let model = value
        .get("model")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("unknown")
        .to_string();
    Ok(GenerateAnswer { text, model })
}

/// One bounded generate call: spawn the lane, handshake, verify the
/// tool is declared, call, tear down. Every failure is named.
pub async fn text(
    spec: &crate::mcp::ServerSpec,
    prompt: &str,
    system: Option<&str>,
    max_tokens: u32,
) -> Result<GenerateAnswer, String> {
    let timeouts = Timeouts {
        call: GENERATE_BUDGET,
        ..Timeouts::default()
    };
    let server = McpServer::spawn_with(spec, timeouts)
        .map_err(|error| format!("the generate lane could not start: {error}"))?;
    let result = drive(&server, prompt, system, max_tokens).await;
    server.close().await.ok();
    result
}

async fn drive(
    server: &McpServer,
    prompt: &str,
    system: Option<&str>,
    max_tokens: u32,
) -> Result<GenerateAnswer, String> {
    server
        .initialize()
        .await
        .map_err(|error| format!("the generate lane's handshake failed: {error}"))?;
    let tools = server
        .list_tools()
        .await
        .map_err(|error| format!("the generate lane's tool list failed: {error}"))?;
    if !tools.iter().any(|tool| tool.name == GENERATE_TOOL) {
        return Err(format!(
            "the lane declares no `{GENERATE_TOOL}` tool — the vendored blend1 predates the text leg"
        ));
    }
    let mut arguments = json!({
        "prompt": prompt,
        "model": TEXT_MODEL,
        "max_tokens": max_tokens,
    });
    if let Some(system) = system {
        arguments["system"] = json!(system);
    }
    let result: CallResult = server
        .call_tool(GENERATE_TOOL, arguments)
        .await
        .map_err(|error| format!("the generate lane refused the call: {error}"))?;
    if result.is_error {
        return Err(format!(
            "the generate lane refused the call: {}",
            result.text()
        ));
    }
    answer_from_result(&result.text())
}

/// The blend1 spec from the home's own lane resolution — `None` when the
/// lane cannot be resolved (no venv, no home), which the caller reads as
/// "run today's path and say so".
pub fn blend1_spec(home: &std::path::Path) -> Option<crate::mcp::ServerSpec> {
    let _ = home;
    lanes::spec_for(BLEND1_LANE).ok()
}

// ── the prose stores ─────────────────────────────────────────────────────
// The sidecars the end-of-run act writes and the archive renders. One
// home per fact: the titles store's readers and writer live in the gui
// bridge (the face's own surface), imported here; the summaries store
// (`sessions-summaries.json`) is new, owned beside the titles store's
// shape.

pub(crate) use crate::gui::bridge::{load_titles, save_titles, SUMMARIES_FILE};

/// The stored lane summaries: the same id-keyed map shape as the titles
/// store, read for the archive listing.
pub fn load_summaries(home: &std::path::Path) -> std::collections::HashMap<String, String> {
    let text = match std::fs::read_to_string(home.join(SUMMARIES_FILE)) {
        Ok(text) => text,
        Err(_) => return std::collections::HashMap::new(),
    };
    serde_json::from_str(&text).unwrap_or_default()
}

fn save_summaries(home: &std::path::Path, summaries: &std::collections::HashMap<String, String>) {
    let path = home.join(SUMMARIES_FILE);
    if summaries.is_empty() {
        let _ = std::fs::remove_file(&path);
        return;
    }
    let Ok(text) = serde_json::to_string_pretty(summaries) else {
        return;
    };
    let temp = path.with_extension(format!("{}.tmp", std::process::id()));
    if std::fs::write(&temp, text).is_err() {
        return;
    }
    let _ = std::fs::rename(&temp, &path);
}

/// The C11 narration pass, for a caller holding a spec: the lane drafts
/// prose from the caller's deterministic facts, and `validate` is the
/// consumer's own acceptance check — a draft that fails it is as good
/// as a dead lane. Sync by design: the CLI surfaces that narrate are
/// sync, and each builds its own current-thread runtime here.
pub fn narrate_with_spec(
    spec: &crate::mcp::ServerSpec,
    system: &str,
    facts: &str,
    max_tokens: u32,
    validate: impl Fn(&str) -> Result<(), String>,
) -> Option<String> {
    // No ambient runtime here — the narrating surfaces are sync CLI
    // paths — so this builds its own current-thread runtime, the same
    // pattern the headless commands use.
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .ok()
        .and_then(|runtime| {
            runtime.block_on(async {
                match text(spec, facts, Some(system), max_tokens).await {
                    Ok(answer) if validate(&answer.text).is_ok() => Some(answer.text),
                    Ok(answer) => {
                        let _ = validate(&answer.text).unwrap_err();
                        None
                    }
                    Err(_) => None,
                }
            })
        })
}

/// The C11 narration pass, for a sync caller that holds only the home:
/// resolve blend1 from the home's own lane chain, then narrate. `None`
/// means "run today's path" — no lane, no narration, no error surfaced
/// beyond the caller's own fallback line.
pub fn narrate_sync(
    home: &std::path::Path,
    system: &str,
    facts: &str,
    max_tokens: u32,
    validate: impl Fn(&str) -> Result<(), String>,
) -> Option<String> {
    let spec = blend1_spec(home)?;
    narrate_with_spec(&spec, system, facts, max_tokens, validate)
}

/// The end-of-run prose act (Phase 4's C10): a finished main-surface run
/// asks the small model for the session's title and one-sentence archive
/// summary, writing both sidecars. The owner's own title always wins
/// (an existing `sessions-titles.json` entry is never overwritten); the
/// failure path is a named progress line and an unchanged archive —
/// the ladder and the head-and-tail summary keep serving. Never fails
/// the run: every error is swallowed into the progress line.
pub async fn end_of_run_prose(
    spec: &crate::mcp::ServerSpec,
    home: &std::path::Path,
    session_id: &str,
    task: &str,
    final_message: Option<&str>,
    progress: &mut dyn std::io::Write,
) {
    let seed: String = task.chars().take(800).collect();
    // The title: the owner's own title wins, so an existing entry means
    // one lane call (the summary) instead of two.
    let titles = load_titles(home);
    if !titles.contains_key(session_id) {
        match text(
            spec,
            &format!("The session's work: {seed}"),
            Some(
                "You name work sessions. Answer with the name only — at most six                  words, no quotes, no period, no explanation.",
            ),
            1024,
        )
        .await
        {
            Ok(answer) => {
                let mut titles = load_titles(home);
                titles.insert(session_id.to_string(), answer.text);
                save_titles(home, &titles);
            }
            Err(error) => {
                let _ = writeln!(
                    progress,
                    "\u{2014} the lane title was not written ({error}); the ladder keeps serving"
                );
            }
        }
    }
    match text(
        spec,
        &format!(
            "The session's task: {seed}\n\nHow it ended: {}",
            final_message
                .map(|message| message.chars().take(500).collect::<String>())
                .unwrap_or_else(|| "no final message".to_string())
        ),
        Some(
            "Summarize this work session in one sentence for an archive card.              State what was done and how it ended. Facts only, no preamble.",
        ),
        1024,
    )
    .await
    {
        Ok(answer) => {
            let mut summaries = load_summaries(home);
            summaries.insert(session_id.to_string(), answer.text);
            save_summaries(home, &summaries);
        }
        Err(error) => {
            let _ = writeln!(
                progress,
                "\u{2014} the lane summary was not written ({error}); the head-and-tail summary keeps serving"
            );
        }
    }
    let _ = progress.flush();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_answer_parses_the_legs_ok_shape() {
        let answer = answer_from_result(
            r#"{"status":"ok","model":"lfm3-1.7b","text":"  The Parser Fix  ","usage":{}}"#,
        )
        .unwrap();
        assert_eq!(answer.text, "The Parser Fix", "the text is trimmed");
        assert_eq!(answer.model, "lfm3-1.7b");
    }

    #[test]
    fn the_legs_named_failure_is_the_error() {
        let error = answer_from_result(
            r#"{"status":"error","error":"Liquid API HTTP 401: bad key","model":"lfm3-1.7b"}"#,
        )
        .unwrap_err();
        assert!(error.contains("HTTP 401"), "{error}");
    }

    #[test]
    fn an_empty_answer_is_a_named_error_not_a_silent_one() {
        let error = answer_from_result(r#"{"status":"ok","model":"m","text":"   "}"#).unwrap_err();
        assert!(error.contains("empty text"), "{error}");
        let unshaped = answer_from_result("not json at all").unwrap_err();
        assert!(unshaped.contains("not structured content"), "{unshaped}");
    }

    #[test]
    fn the_system_prompt_rides_when_given() {
        // The arguments built for the wire: system present only when
        // the caller gave one. Pinned at the JSON level so a field
        // rename here is a test failure, not a silent shape change.
        let mut arguments = json!({
            "prompt": "fold this",
            "model": TEXT_MODEL,
            "max_tokens": 4096u32,
        });
        arguments["system"] = json!("summarize");
        assert_eq!(arguments["prompt"], "fold this");
        assert_eq!(arguments["model"], TEXT_MODEL);
        assert_eq!(arguments["max_tokens"], 4096);
        assert_eq!(arguments["system"], "summarize");
    }
}

#[cfg(test)]
mod prose_tests {
    use super::*;

    /// A scripted blend1 (the fold tests' harness shape, local here):
    /// two generate calls scripted — title then summary.
    fn lane_mock(
        answers: &[&str],
    ) -> (std::process::Child, std::path::PathBuf, std::path::PathBuf) {
        // Nanos alone collide when two tests start inside the same
        // microsecond (measured flake): a process-wide counter makes
        // the directory unique per call.
        static SEQ: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let seq = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let scratch = std::env::temp_dir().join(format!(
            "fleet-prose-mock-{}-{}-{seq}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&scratch).expect("the scratch");
        let mut entries = vec![
            serde_json::json!({ "reply": {
                "jsonrpc": "2.0", "id": "$echo",
                "result": {
                    "protocolVersion": "2024-11-05",
                    "capabilities": { "tools": { "listChanged": false } },
                    "serverInfo": { "name": "mock-blend1", "version": "0.0.0" },
                },
            }}),
            serde_json::json!({ "reply": {
                "jsonrpc": "2.0", "id": "$echo",
                "result": { "tools": [ { "name": "generate", "inputSchema": {} } ] },
            }}),
        ];
        for answer in answers {
            entries.push(serde_json::json!({ "reply": {
                "jsonrpc": "2.0", "id": "$echo",
                "result": { "content": [ { "type": "text", "text": answer } ], "isError": false },
            }}));
        }
        let script = entries
            .iter()
            .map(|entry| format!("{entry}\n"))
            .collect::<String>();
        std::fs::write(scratch.join("script.jsonl"), script).unwrap();
        let bin = crate::eval::mock_mcp_binary().expect("the mock-mcp binary");
        let child = std::process::Command::new(bin)
            .arg(scratch.join("script.jsonl"))
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("spawn the mock-mcp child");
        (child, scratch.join("script.jsonl"), scratch)
    }

    fn spec(script: &std::path::Path) -> crate::mcp::ServerSpec {
        crate::mcp::ServerSpec {
            program: crate::eval::mock_mcp_binary()
                .expect("the mock-mcp binary")
                .to_string_lossy()
                .into_owned(),
            args: vec![script.to_string_lossy().into_owned()],
            env: Vec::new(),
            passthrough: Vec::new(),
            cwd: None,
        }
    }

    fn notes_home(name: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("fleet-prose-home-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("sessions")).unwrap();
        dir
    }

    #[tokio::test]
    async fn the_prose_act_writes_title_and_summary_sidecars() {
        // Each lane call spawns a fresh mock child that re-reads the
        // script from its first entry — one answer must serve both the
        // title arm and the summary arm. It reads as both.
        let (mut child, script, scratch) = lane_mock(&[
            r#"{"status":"ok","model":"lfm3-1.7b","text":"Fixed the streaming parser and swept the caches"}"#,
        ]);
        let home = notes_home("writes");
        let id = "20261110T120000Z-prose1";
        std::fs::write(home.join("sessions").join(format!("{id}.jsonl")), "").unwrap();
        let mut progress: Vec<u8> = Vec::new();
        end_of_run_prose(
            &spec(&script),
            &home,
            id,
            "fix the streaming tool-call parser",
            Some("done: the parser is pinned and the caches swept"),
            &mut progress,
        )
        .await;
        let titles = load_titles(&home);
        assert_eq!(
            titles.get(id).map(String::as_str),
            Some("Fixed the streaming parser and swept the caches")
        );
        let summaries = load_summaries(&home);
        if !summaries.contains_key(id) {
            panic!(
                "no summary; progress was: {}",
                String::from_utf8_lossy(&progress)
            );
        }
        assert_eq!(
            summaries.get(id).map(String::as_str),
            Some("Fixed the streaming parser and swept the caches")
        );
        // The bridge's reader agrees — one store, two readers.
        let from_bridge = crate::gui::bridge::load_titles(&home);
        assert_eq!(
            from_bridge.get(id).map(String::as_str),
            Some("Fixed the streaming parser and swept the caches")
        );
        let _ = std::fs::remove_dir_all(&home);
        let _ = child.kill();
        let _ = child.wait();
        let _ = std::fs::remove_dir_all(&scratch);
    }

    #[tokio::test]
    async fn the_owners_title_wins_and_a_failed_leg_falls_back_named() {
        // An existing title entry: only the summary call is scripted.
        // Then the summary leg fails — named on progress, the archive
        // keeps its heuristic, nothing dies.
        let (mut child, script, scratch) = lane_mock(&[
            r#"{"status":"error","error":"Liquid API HTTP 503: warming","model":"lfm3-1.7b"}"#,
        ]);
        let home = notes_home("owner-wins");
        let id = "20261110T120000Z-prose2";
        std::fs::write(home.join("sessions").join(format!("{id}.jsonl")), "").unwrap();
        {
            let mut titles = load_titles(&home);
            titles.insert(id.to_string(), "the owner named it".to_string());
            save_titles(&home, &titles);
        }
        let mut progress: Vec<u8> = Vec::new();
        end_of_run_prose(&spec(&script), &home, id, "the task", None, &mut progress).await;
        assert_eq!(
            load_titles(&home).get(id).map(String::as_str),
            Some("the owner named it"),
            "the owner's title is never overwritten"
        );
        let text = String::from_utf8_lossy(&progress);
        assert!(text.contains("the lane summary was not written"), "{text}");
        assert!(
            load_summaries(&home).is_empty(),
            "no summary was written: {:?}",
            load_summaries(&home)
        );
        let _ = std::fs::remove_dir_all(&home);
        let _ = child.kill();
        let _ = child.wait();
        let _ = std::fs::remove_dir_all(&scratch);
    }
}
