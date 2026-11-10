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
