//! The lane-kind candidate's client boundary.
//!
//! A lane candidate (blend1) is not a chat-completions gateway: it
//! answers a one-shot grounding `ask` over MCP, and its answer is a
//! structured result rather than a chat completion. This module is the
//! one place that mapping lives — the chat request in, the lane's
//! answer out in the chat shape — so the router's walk treats a lane
//! exactly like any other candidate and the lane's own facts (its
//! path, whether it escalated, its score) ride the route row's reason
//! instead of being flattened away. The owner's ruling on this
//! adapter: it SURFACES blend1's differences, never hides them.
//!
//! The seam is the judge's and the delegate's, walked the same way:
//! resolve the spec, spawn the child, handshake, list its tools, call
//! `ask`, tear the child down. One ask per call — a lane candidate's
//! request spawns its lane, which is the self-contained harness's
//! shape (no long-running service the owner must start), and every
//! failure is named so the walk can fail over on it.

use serde_json::{json, Value};

use crate::api::{ChatResponse, Choice, Message, Role, Usage};
use crate::mcp::{CallResult, McpServer, ServerSpec, Timeouts};

/// The lane tool this boundary calls. blend1's MCP surface declares
/// `ask`; the judge's and the delegate's tools belong to their own
/// boundaries, not this one.
pub const ASK_TOOL: &str = "ask";

/// The boundary's own budget for one composed ask. The MCP client's
/// default call timeout (60s) is sized for the judge's fast classify
/// asks; a composed grounding ask walks the lane's legs — route,
/// semantic retrieval over the corpus, tabular, generate, validate,
/// score — and the owner's ruling on this lane is explicit that a slow
/// answer beats a missed one ("if a problem takes 15/45 seconds, that's
/// irrelevant"). Five minutes is the named budget; an ask that outlasts
/// it is a named refusal and the walk advances.
pub const ASK_BUDGET: std::time::Duration = std::time::Duration::from_secs(300);

/// One lane answer, mapped into the chat shape.
#[derive(Debug, Clone, PartialEq)]
pub struct LaneAnswer {
    /// The grounded answer — the chat shape's content.
    pub text: String,
    /// The named clause the route row carries: the lane's path, whether
    /// it escalated, its score, its own latency. Bounded by the lane's
    /// own result shape (it strips its telemetry before answering).
    pub note: String,
}

/// Map the chat request's messages into the lane's ask prompt: the
/// whole conversation, role-named, because the lane is stateless and
/// one-shot — every turn's ask must carry the work so far, exactly as
/// the chat request carries it for a gateway.
pub fn ask_prompt(messages: &[Message]) -> String {
    messages
        .iter()
        .map(|message| {
            let content = message.content.as_deref().unwrap_or("");
            format!("{}: {content}", role_label(message.role))
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// The role's name in the prompt. The lane reads prose; the role label
/// is what tells it which lines are the task and which are answers.
fn role_label(role: Role) -> &'static str {
    match role {
        Role::System => "system",
        Role::User => "user",
        Role::Assistant => "assistant",
        Role::Tool => "tool",
    }
}

/// Map the lane's `ask` result into the chat shape. The result is the
/// lane's own JSON (`BlendResult.as_dict()`); a result that is not
/// that shape — unparseable, no `final_answer`, an empty one — is a
/// named refusal, never a silent empty answer, so the walk advances
/// instead of shipping nothing.
pub fn answer_from_result(text: &str) -> Result<LaneAnswer, String> {
    let value: Value = serde_json::from_str(text).map_err(|error| {
        format!("the lane's answer was not the structured content it promises: {error}")
    })?;
    let final_answer = value
        .get("final_answer")
        .and_then(Value::as_str)
        .ok_or_else(|| "the lane's answer carried no `final_answer`".to_string())?;
    if final_answer.trim().is_empty() {
        return Err("the lane's answer carried an empty `final_answer`".to_string());
    }
    Ok(LaneAnswer {
        text: final_answer.to_string(),
        note: note_from_result(&value),
    })
}

/// The named clause the route row carries. The lane's path (how it
/// answered — direct, synthesized, or decomposed), whether any leg
/// failed open (`escalated`), its score as the lane's own compact
/// JSON, and its own latency — every difference the adapter surfaces
/// rather than flattens.
fn note_from_result(value: &Value) -> String {
    let path = value
        .get("path")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let escalated = value
        .get("escalated")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let latency = value
        .get("total_latency_ms")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let score = value
        .get("score")
        .map(|score| score.to_string())
        .unwrap_or_else(|| "{}".to_string());
    format!(
        "; blend1 answered through path `{path}` (escalated: {escalated}, \
         score {score}, lane latency {latency}ms)"
    )
}

/// One bounded ask to the lane: spawn the child, handshake, list its
/// tools, call `ask` with the mapped prompt, tear the child down.
/// Every failure is named — an unspawnable child, a handshake that
/// times out, a lane that declares no `ask`, a call the lane refused,
/// an answer that is not the structured content it promises — and the
/// walk treats the name as a failover and advances to the workload's
/// next candidate.
pub async fn ask(spec: &ServerSpec, prompt: &str) -> Result<LaneAnswer, String> {
    let timeouts = Timeouts {
        call: ASK_BUDGET,
        ..Timeouts::default()
    };
    let server = McpServer::spawn_with(spec, timeouts)
        .map_err(|error| format!("the lane could not start: {error}"))?;
    server
        .initialize()
        .await
        .map_err(|error| format!("the lane's handshake failed: {error}"))?;
    let tools = server
        .list_tools()
        .await
        .map_err(|error| format!("the lane's tool list failed: {error}"))?;
    if !tools.iter().any(|tool| tool.name == ASK_TOOL) {
        server.close().await.ok();
        return Err(format!(
            "the lane declares no `{ASK_TOOL}` tool — it cannot answer a composed ask"
        ));
    }
    let result: CallResult = server
        .call_tool(ASK_TOOL, json!({ "prompt": prompt }))
        .await
        .map_err(|error| format!("the lane refused the ask: {error}"))?;
    server.close().await.ok();
    if result.is_error {
        return Err(format!("the lane refused the ask: {}", result.text()));
    }
    answer_from_result(&result.text())
}

/// The chat shape one lane answer maps into: the answer's text as the
/// assistant message, one `stop` finish, and no token accounting —
/// the lane reports none, and inventing one would put fake numbers in
/// the stream. The lane's own latency rides the route row's reason.
pub fn chat_response(answer: &LaneAnswer) -> ChatResponse {
    ChatResponse {
        choices: vec![Choice {
            message: Message::text(Role::Assistant, answer.text.clone()),
            finish_reason: Some("stop".to_string()),
        }],
        usage: Usage::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::Message;

    fn user(text: &str) -> Message {
        Message::text(Role::User, text)
    }

    #[test]
    fn the_prompt_carries_the_whole_conversation_role_named() {
        let prompt = ask_prompt(&[
            Message::text(Role::System, "you are fleet"),
            user("refactor the parser"),
            Message::text(Role::Assistant, "reading it now"),
            Message::tool_result("call-1", "the parser is 400 lines"),
        ]);
        assert!(prompt.contains("system: you are fleet"), "{prompt}");
        assert!(prompt.contains("user: refactor the parser"), "{prompt}");
        assert!(prompt.contains("assistant: reading it now"), "{prompt}");
        assert!(prompt.contains("tool: the parser is 400 lines"), "{prompt}");
    }

    #[test]
    fn a_blend_result_maps_to_the_chat_shape_and_names_its_facts() {
        let answer = answer_from_result(
            r#"{"path":"synthesized","final_answer":"the grounded answer","escalated":true,
                "score":{"correct":true,"confidence":0.8},"total_latency_ms":1234}"#,
        )
        .expect("the lane's result maps");
        assert_eq!(answer.text, "the grounded answer");
        // The adapter surfaces the lane's differences: path, escalation,
        // score, latency — all named in the clause.
        assert!(
            answer.note.contains("path `synthesized`"),
            "{}",
            answer.note
        );
        assert!(answer.note.contains("escalated: true"), "{}", answer.note);
        assert!(answer.note.contains("\"correct\":true"), "{}", answer.note);
        assert!(answer.note.contains("latency 1234ms"), "{}", answer.note);
        // The chat shape carries the text and no invented accounting.
        let response = chat_response(&answer);
        assert_eq!(response.choices.len(), 1);
        assert_eq!(
            response.choices[0].message.content.as_deref(),
            Some("the grounded answer")
        );
        assert_eq!(response.usage.total_tokens, 0);
    }

    #[test]
    fn a_result_that_is_not_the_promised_shape_is_refused_by_name() {
        // Not JSON at all.
        let error = answer_from_result("the lane said something prose").unwrap_err();
        assert!(error.contains("not the structured content"), "{error}");
        // JSON without the answer.
        let error = answer_from_result(r#"{"path":"direct"}"#).unwrap_err();
        assert!(error.contains("no `final_answer`"), "{error}");
        // An empty answer ships nothing, so it is a refusal, not a
        // silent empty completion.
        let error = answer_from_result(r#"{"final_answer":"  "}"#).unwrap_err();
        assert!(error.contains("empty `final_answer`"), "{error}");
    }
}
