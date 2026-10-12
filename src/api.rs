//! The DeepSeek chat-completions client: typed requests, function-calling
//! tool declarations, and error mapping with actionable messages.

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Public DeepSeek API base.
pub const DEFAULT_BASE_URL: &str = "https://api.deepseek.com";

/// Default chat model.
pub const DEFAULT_MODEL: &str = "deepseek-chat";

/// One tool call issued by the assistant. `arguments` stays the JSON text
/// the API issued; decoding is the agent's job so decode failures are
/// model-visible results rather than client errors.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    /// The API only defines `"function"` today; carried verbatim so a replayed
    /// log round-trips losslessly.
    #[serde(rename = "type")]
    pub kind: String,
    pub function: FunctionCall,
}

/// The function half of a tool call.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    /// JSON-encoded arguments object, exactly as issued.
    pub arguments: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
    /// A tool result answering one call.
    Tool,
}

/// One conversation message. This type is both the wire format and the
/// session-log payload, so a replayed log reproduces requests exactly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_calls: Vec<ToolCall>,
    /// Present on `Role::Tool`: the call id this message answers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

impl Message {
    /// A `system`/`user`/`assistant` text message.
    pub fn text(role: Role, content: impl Into<String>) -> Message {
        Message { role, content: Some(content.into()), tool_calls: Vec::new(), tool_call_id: None }
    }

    /// A tool result answering `call_id`.
    pub fn tool_result(call_id: impl Into<String>, content: impl Into<String>) -> Message {
        Message {
            role: Role::Tool,
            content: Some(content.into()),
            tool_calls: Vec::new(),
            tool_call_id: Some(call_id.into()),
        }
    }
}

/// A function-calling tool declaration sent with a request.
#[derive(Debug, Clone, Serialize)]
pub struct ToolSpec {
    r#type: String,
    function: FunctionSpec,
}

/// The function half of a tool declaration.
#[derive(Debug, Clone, Serialize)]
pub struct FunctionSpec {
    name: String,
    description: String,
    /// JSON Schema for the arguments object.
    parameters: serde_json::Value,
}

impl ToolSpec {
    /// A `"function"`-kind declaration.
    pub fn function(name: impl Into<String>, description: impl Into<String>, parameters: serde_json::Value) -> ToolSpec {
        ToolSpec {
            r#type: "function".to_string(),
            function: FunctionSpec { name: name.into(), description: description.into(), parameters },
        }
    }
}

/// One chat-completions request.
#[derive(Debug, Serialize)]
pub struct ChatRequest<'a> {
    pub model: &'a str,
    pub messages: &'a [Message],
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<ToolSpec>,
}

#[derive(Debug, Deserialize)]
pub struct ChatResponse {
    pub choices: Vec<Choice>,
    #[serde(default)]
    pub usage: Usage,
}

#[derive(Debug, Deserialize)]
pub struct Choice {
    pub message: Message,
    #[serde(default)]
    pub finish_reason: Option<String>,
}

/// Token accounting for one request. Absent fields read as zero.
#[derive(Debug, Clone, Copy, Default, Deserialize)]
pub struct Usage {
    #[serde(default)]
    pub prompt_tokens: u64,
    #[serde(default)]
    pub completion_tokens: u64,
    #[serde(default)]
    pub total_tokens: u64,
}

/// A non-2xx API answer.
#[derive(Debug)]
pub struct ApiError {
    pub status: u16,
    pub message: String,
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let hint = match self.status {
            401 => " — check DEEPSEEK_API_KEY",
            402 => " — the account needs a balance top-up",
            429 => " — slow down or retry later",
            _ => "",
        };
        write!(f, "API error {}: {}{}", self.status, self.message, hint)
    }
}

/// Everything that can go wrong between the agent and a parsed response.
#[derive(Debug)]
pub enum ClientError {
    Api(ApiError),
    /// The request never completed: DNS, connect, TLS, timeout, body read.
    Network(String),
    /// A 2xx answer whose body did not match the documented schema.
    Decode(String),
}

impl std::fmt::Display for ClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClientError::Api(error) => write!(f, "{error}"),
            ClientError::Network(message) => write!(f, "request failed: {message}"),
            ClientError::Decode(message) => write!(f, "unexpected API response: {message}"),
        }
    }
}

impl std::error::Error for ClientError {}

/// HTTPS client for one credential and base URL. Requests carry a generous
/// overall timeout because long completions are ordinary; connect failures
/// surface in seconds.
pub struct Client {
    base_url: String,
    api_key: String,
    agent: ureq::Agent,
}

impl Client {
    /// A client for `base_url` (trailing slash tolerated) and `api_key`.
    pub fn new(base_url: impl Into<String>, api_key: impl Into<String>) -> Client {
        let base_url = base_url.into().trim_end_matches('/').to_string();
        Client {
            base_url,
            api_key: api_key.into(),
            // The overall timeout bounds every phase including the connect;
            // ureq's `timeout_connect` path leaves reads broken on macOS
            // sandboxed environments, so it is deliberately not set.
            agent: ureq::AgentBuilder::new()
                .timeout(Duration::from_secs(600))
                .build(),
        }
    }

    /// The base URL this client talks to.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// One chat completion.
    pub fn chat(&self, request: &ChatRequest) -> Result<ChatResponse, ClientError> {
        let url = format!("{}/chat/completions", self.base_url);
        let response = self
            .agent
            .post(&url)
            .set("Authorization", &format!("Bearer {}", self.api_key))
            .send_json(request)
            .map_err(map_send_error)?;
        let status = response.status();
        let body = response
            .into_string()
            .map_err(|error| ClientError::Network(error.to_string()))?;
        if status != 200 {
            return Err(ClientError::Api(api_error(status, &body)));
        }
        serde_json::from_str(&body).map_err(|error| ClientError::Decode(error.to_string()))
    }
}

/// Split ureq's two failure shapes: an answered status carries its response
/// body, everything else is transport.
fn map_send_error(error: ureq::Error) -> ClientError {
    match error {
        ureq::Error::Status(status, response) => match response.into_string() {
            Ok(body) => ClientError::Api(api_error(status, &body)),
            Err(read) => ClientError::Network(read.to_string()),
        },
        ureq::Error::Transport(transport) => ClientError::Network(transport.to_string()),
    }
}

/// Build an `ApiError` from a raw body, extracting `{"error":{"message":…}}`
/// when present, falling back to a bounded excerpt.
fn api_error(status: u16, body: &str) -> ApiError {
    let message = serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|value| {
            value
                .get("error")
                .and_then(|error| error.get("message"))
                .and_then(|message| message.as_str())
                .map(str::to_string)
        })
        .unwrap_or_else(|| excerpt(body, 300));
    ApiError { status, message }
}

/// First `limit` bytes of `text` on a char boundary, with an elision marker
/// when anything was cut.
fn excerpt(text: &str, limit: usize) -> String {
    if text.len() <= limit {
        return text.to_string();
    }
    let mut end = limit;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}\u{2026}", &text[..end])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn excerpt_keeps_char_boundaries() {
        let text = "a\u{00e9}\u{4e2d}".repeat(200);
        let cut = excerpt(&text, 100);
        assert!(cut.len() <= 104);
        assert!(cut.ends_with('\u{2026}'));
    }

    #[test]
    fn api_error_reads_the_documented_error_message() {
        let error = api_error(401, r#"{"error":{"message":"auth failed"}}"#);
        assert_eq!(error.status, 401);
        assert_eq!(error.message, "auth failed");
    }

    #[test]
    fn api_error_falls_back_to_a_bounded_excerpt() {
        let error = api_error(500, "<html>not json at all, and long enough to matter</html>");
        assert_eq!(error.status, 500);
        assert!(error.message.contains("not json"));
    }

    #[test]
    fn message_serializes_only_present_fields() {
        let user = Message::text(Role::User, "hi");
        let line = serde_json::to_string(&user).unwrap();
        assert_eq!(line, r#"{"role":"user","content":"hi"}"#);
    }

    #[test]
    fn tool_result_serializes_with_its_call_id() {
        let result = Message::tool_result("call-1", "ok");
        let line = serde_json::to_string(&result).unwrap();
        assert!(line.contains(r#""role":"tool""#));
        assert!(line.contains(r#""tool_call_id":"call-1""#));
    }
}
