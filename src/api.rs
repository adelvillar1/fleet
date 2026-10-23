//! The DeepSeek chat-completions client: typed requests, function-calling
//! tool declarations, and error mapping with actionable messages.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::sync::Arc;
use std::time::Duration;

use crate::route::{RouteDecision, Router};

/// Public DeepSeek API base.
pub const DEFAULT_BASE_URL: &str = "https://api.deepseek.com";

/// The reserve's model: what the roster's deepseek entry serves and the
/// failover tail every walk closes on. Not a default for anything else —
/// a run's model is the router's (`route::AUTO_MODEL` until it answers).
pub const DEEPSEEK_MODEL: &str = "deepseek-flash";

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
/// The two directions are hand-written rather than derived: the content
/// is a string on the ordinary message and a parts array on one carrying
/// images, and a derive would have forced a single shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    pub role: Role,
    pub content: Option<String>,
    pub tool_calls: Vec<ToolCall>,
    /// Present on `Role::Tool`: the call id this message answers.
    pub tool_call_id: Option<String>,
    /// The images this message carries, beside its text. Empty on the
    /// ordinary text message, and the wire format is then exactly what it
    /// has always been — a string. With images it is the parts array,
    /// which is the whole point of the custom serialization below.
    pub images: Vec<ImageRef>,
}

impl Message {
    /// A `system`/`user`/`assistant` text message.
    pub fn text(role: Role, content: impl Into<String>) -> Message {
        Message {
            role,
            content: Some(content.into()),
            tool_calls: Vec::new(),
            tool_call_id: None,
            images: Vec::new(),
        }
    }

    /// A text message carrying images. The text is the instruction the
    /// images answer, which is the shape every vision endpoint takes.
    pub fn text_with_images(
        role: Role,
        content: impl Into<String>,
        images: Vec<ImageRef>,
    ) -> Message {
        Message {
            role,
            content: Some(content.into()),
            tool_calls: Vec::new(),
            tool_call_id: None,
            images,
        }
    }

    /// A tool result answering `call_id`.
    pub fn tool_result(call_id: impl Into<String>, content: impl Into<String>) -> Message {
        Message {
            role: Role::Tool,
            content: Some(content.into()),
            tool_calls: Vec::new(),
            tool_call_id: Some(call_id.into()),
            images: Vec::new(),
        }
    }
}

/// One image a message carries: its media type and its raw bytes. The
/// bytes are base64-encoded on the wire, which is the only form every
/// OpenAI-compatible endpoint accepts — and every provider on the roster
/// is one, so there is one image format here rather than one per
/// provider.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageRef {
    /// The IANA media type: `image/png`, `image/jpeg`.
    pub media: String,
    /// The raw bytes, encoded on the way out.
    pub data: Vec<u8>,
}

impl ImageRef {
    /// One image from its media type and bytes.
    pub fn new(media: impl Into<String>, data: Vec<u8>) -> ImageRef {
        ImageRef {
            media: media.into(),
            data,
        }
    }

    /// One PNG.
    pub fn png(data: Vec<u8>) -> ImageRef {
        ImageRef::new("image/png", data)
    }

    /// The data URL this image rides as on the wire.
    fn data_url(&self) -> String {
        format!("data:{};base64,{}", self.media, base64(&self.data))
    }

    /// One image read back off a data URL, or `None` for a URL that is
    /// not one — an image that arrived by http reference rather than
    /// inline bytes is a different thing, and a fact the log should not
    /// pretend to hold.
    fn from_data_url(url: &str) -> Option<ImageRef> {
        let rest = url.strip_prefix("data:")?;
        let (head, encoded) = rest.split_once(";base64,")?;
        Some(ImageRef {
            media: head.to_string(),
            data: decode_base64(encoded)?,
        })
    }
}

/// Base64, standard alphabet with padding. The one encoding an image
/// needs, and the crate contract is no new dependency for it.
fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let packed = (u32::from(chunk[0]) << 16)
            | (u32::from(chunk.get(1).copied().unwrap_or(0)) << 8)
            | u32::from(chunk.get(2).copied().unwrap_or(0));
        for at in 0..4 {
            if at <= chunk.len() {
                let index = ((packed >> (18 - at * 6)) & 0x3f) as usize;
                out.push(ALPHABET[index] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

/// `Message` serializes its content one of two ways, and which one is the
/// whole design: with no images it is the string it has always been, so a
/// text message's wire format and its log payload are unchanged; with
/// images it is the parts array every OpenAI-compatible endpoint reads.
/// Deriving both directions would have forced one shape or the other.
impl Serialize for Message {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(5))?;
        map.serialize_entry("role", &self.role)?;
        if self.images.is_empty() {
            map.serialize_entry("content", &self.content)?;
        } else {
            let mut parts: Vec<serde_json::Value> = Vec::new();
            if let Some(text) = &self.content {
                parts.push(serde_json::json!({"type": "text", "text": text}));
            }
            for image in &self.images {
                parts.push(serde_json::json!({
                    "type": "image_url",
                    "image_url": {"url": image.data_url()},
                }));
            }
            map.serialize_entry("content", &parts)?;
        }
        if !self.tool_calls.is_empty() {
            map.serialize_entry("tool_calls", &self.tool_calls)?;
        }
        if let Some(id) = &self.tool_call_id {
            map.serialize_entry("tool_call_id", id)?;
        }
        map.end()
    }
}

/// The read half, which accepts either shape: a string is text, an array
/// is parts, and an image part is read back into its media type and its
/// bytes so the log holds what was sent rather than a derived URL.
impl<'de> Deserialize<'de> for Message {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Content {
            Text(String),
            Parts(Vec<Part>),
        }
        #[derive(Deserialize)]
        struct Part {
            #[serde(rename = "type")]
            kind: String,
            text: Option<String>,
            image_url: Option<ImageUrl>,
        }
        #[derive(Deserialize)]
        struct ImageUrl {
            url: String,
        }
        #[derive(Deserialize)]
        struct Raw {
            role: Role,
            content: Option<Content>,
            #[serde(default)]
            tool_calls: Vec<ToolCall>,
            #[serde(default)]
            tool_call_id: Option<String>,
        }
        let raw = Raw::deserialize(deserializer)?;
        let (content, images) = match raw.content {
            None => (None, Vec::new()),
            Some(Content::Text(text)) => (Some(text), Vec::new()),
            Some(Content::Parts(parts)) => {
                let mut text: Option<String> = None;
                let mut images: Vec<ImageRef> = Vec::new();
                for part in parts {
                    match part.kind.as_str() {
                        "text" => text = part.text,
                        "image_url" => {
                            if let Some(url) = part.image_url {
                                if let Some(image) = ImageRef::from_data_url(&url.url) {
                                    images.push(image);
                                }
                            }
                        }
                        _ => {}
                    }
                }
                (text, images)
            }
        };
        Ok(Message {
            role: raw.role,
            content,
            tool_calls: raw.tool_calls,
            tool_call_id: raw.tool_call_id,
            images,
        })
    }
}

/// Base64's inverse, for reading a logged image back. `None` for input
/// that is not base64, so a malformed log names itself rather than
/// silently decoding to nothing.
fn decode_base64(text: &str) -> Option<Vec<u8>> {
    fn value(character: u8) -> Option<u8> {
        match character {
            b'A'..=b'Z' => Some(character - b'A'),
            b'a'..=b'z' => Some(character - b'a' + 26),
            b'0'..=b'9' => Some(character - b'0' + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    }
    let bytes: Vec<u8> = text
        .bytes()
        .filter(|byte| !byte.is_ascii_whitespace())
        .collect();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len() / 4 * 3);
    for chunk in bytes.chunks(4) {
        let mut packed = 0u32;
        for (at, byte) in chunk.iter().enumerate() {
            if *byte == b'=' {
                break;
            }
            packed |= u32::from(value(*byte)?) << (18 - at * 6);
        }
        out.push((packed >> 16) as u8);
        if chunk.len() > 2 && chunk[2] != b'=' {
            out.push((packed >> 8) as u8);
        }
        if chunk.len() > 3 && chunk[3] != b'=' {
            out.push(packed as u8);
        }
    }
    Some(out)
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
    pub fn function(
        name: impl Into<String>,
        description: impl Into<String>,
        parameters: serde_json::Value,
    ) -> ToolSpec {
        ToolSpec {
            r#type: "function".to_string(),
            function: FunctionSpec {
                name: name.into(),
                description: description.into(),
                parameters,
            },
        }
    }

    /// The declaration's description.
    pub fn description(&self) -> &str {
        &self.function.description
    }

    /// The declaration with its description replaced — the one seam a
    /// per-run declaration needs (the improve tool hands its lesson
    /// store back this way).
    pub fn with_description(mut self, description: impl Into<String>) -> ToolSpec {
        self.function.description = description.into();
        self
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

/// The streaming knobs carried beside a request. Absent on the
/// non-streaming path, where the API answers with one whole document.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct StreamOptions {
    /// Ask for the usage-only final chunk, so token accounting stays exact
    /// on the streamed path.
    pub include_usage: bool,
}

/// A streamed chat-completions request: the same shape plus `"stream": true`
/// and the usage option that keeps accounting exact.
#[derive(Debug, Serialize)]
pub struct StreamRequest<'a> {
    #[serde(flatten)]
    pub request: &'a ChatRequest<'a>,
    pub stream: bool,
    pub stream_options: StreamOptions,
}

/// One event off a streamed completion. Deltas are transport, not state:
/// `chat_stream` assembles them into the same `ChatResponse` `chat` returns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StreamEvent {
    /// One content delta: a token, a fragment of one, or a text chunk.
    Content(String),
    /// One delta of the model's reasoning, when the provider sends it
    /// (`reasoning_content`). It is display material: it is logged and
    /// shown, and never sent back in a later request — the conversation
    /// builder ignores it, which is what a reasoning model requires.
    Reasoning(String),
    /// Every tool call the answer requested, assembled and complete.
    ToolCalls(Vec<ToolCall>),
    /// The request's token accounting; providers send it in the final chunk,
    /// and any chunk carrying it is honored.
    Usage(Usage),
}

/// One SSE chunk's payload: the fields this crate reads. Everything else
/// (ids, logprobs, system fingerprints) is transport detail.
#[derive(Debug, Deserialize)]
struct Chunk {
    #[serde(default)]
    choices: Vec<ChunkChoice>,
    #[serde(default)]
    usage: Option<Usage>,
}

#[derive(Debug, Deserialize)]
struct ChunkChoice {
    #[serde(default)]
    delta: ChunkDelta,
    #[serde(default)]
    finish_reason: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct ChunkDelta {
    #[serde(default)]
    content: Option<String>,
    /// The model's thinking, on the providers that stream it beside the
    /// answer (DeepSeek's reasoner and its kin). Absent elsewhere.
    #[serde(default)]
    reasoning_content: Option<String>,
    #[serde(default)]
    tool_calls: Vec<ChunkToolCall>,
}

/// One tool-call delta, keyed by `index`: the fragments of a call arrive
/// across chunks and concatenate into the whole.
#[derive(Debug, Deserialize)]
struct ChunkToolCall {
    #[serde(default)]
    index: usize,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    function: Option<ChunkToolCallFunction>,
}

#[derive(Debug, Default, Deserialize)]
struct ChunkToolCallFunction {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    arguments: Option<String>,
}

/// A tool call under assembly: the first fragment names it, later ones
/// extend its arguments.
#[derive(Debug, Default)]
struct ToolCallDraft {
    id: String,
    name: String,
    arguments: String,
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
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Usage {
    #[serde(default)]
    pub prompt_tokens: u64,
    #[serde(default)]
    pub completion_tokens: u64,
    #[serde(default)]
    pub total_tokens: u64,
    #[serde(default)]
    pub finish_reason: Option<String>,
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
    /// When set, every request routes through the roster with failover
    /// ([`Router`]) instead of talking to `base_url` directly. `None` is a
    /// plain single-endpoint client — the shape every existing caller and
    /// test builds, unchanged.
    router: Option<Arc<Router>>,
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
            router: None,
        }
    }

    /// A client that routes every request through `router` — the roster
    /// with sys1-judged selection and failover. Its own base URL and key
    /// are unused: the router picks the candidate (endpoint, key, model)
    /// per request.
    pub fn routed(router: Arc<Router>) -> Client {
        Client {
            base_url: String::new(),
            api_key: String::new(),
            agent: ureq::AgentBuilder::new()
                .timeout(Duration::from_secs(600))
                .build(),
            router: Some(router),
        }
    }

    /// The router this client walks, or `None` for a plain
    /// single-endpoint client. A caller that needs a second route off the
    /// same roster — the vision workload's own chain — reads the resolved
    /// candidates from it rather than resolving the owner's config again.
    pub fn router(&self) -> Option<&Router> {
        self.router.as_deref()
    }

    /// The routing decision this client was built with, or `None` for a
    /// plain single-endpoint client.
    pub fn route_decision(&self) -> Option<RouteDecision> {
        self.router.as_ref().map(|router| router.decision())
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

    /// One streamed chat completion. Every delta reaches `on_event` as it
    /// arrives; the returned `ChatResponse` is the same assembled shape
    /// `chat` produces, so callers keep one code path downstream.
    ///
    /// The blocking socket work runs on a spawned blocking task and crosses
    /// to this async function over an unbounded channel: no `futures` Stream
    /// trait, and the reader stops as soon as the receiver goes away.
    pub async fn chat_stream(
        &self,
        request: &ChatRequest<'_>,
        mut on_event: impl FnMut(StreamEvent),
    ) -> Result<ChatResponse, ClientError> {
        // A routed client walks the roster with failover; a plain one takes
        // the single-endpoint path below, byte-for-byte as before.
        if let Some(router) = &self.router {
            return router.chat_stream(request, &mut on_event).await;
        }
        self.chat_stream_plain(request, on_event).await
    }

    /// The single-endpoint streamed exchange, no routing. Split out so the
    /// router calls it per candidate without the compiler seeing an
    /// infinitely-sized recursive `chat_stream` future: the routed branch
    /// and the plain branch would otherwise form one mutual async cycle.
    pub(crate) async fn chat_stream_plain(
        &self,
        request: &ChatRequest<'_>,
        mut on_event: impl FnMut(StreamEvent),
    ) -> Result<ChatResponse, ClientError> {
        let url = format!("{}/chat/completions", self.base_url);
        let key = self.api_key.clone();
        let body = serde_json::to_vec(&StreamRequest {
            request,
            stream: true,
            stream_options: StreamOptions {
                include_usage: true,
            },
        })
        .map_err(|error| ClientError::Decode(error.to_string()))?;
        let agent = self.agent.clone();

        let (events, mut receiver) = tokio::sync::mpsc::unbounded_channel::<StreamEvent>();
        let reader = tokio::task::spawn_blocking(move || {
            let outcome = stream_response(&agent, &url, &key, &body, &events);
            // Drop the sender inside the blocking task, so a read failure or
            // a dropped receiver both end the pump instead of deadlocking.
            drop(events);
            outcome
        });

        let mut content = String::new();
        let mut tool_calls: Vec<ToolCall> = Vec::new();
        let mut usage = Usage::default();
        while let Some(event) = receiver.recv().await {
            match &event {
                StreamEvent::Content(delta) => content.push_str(delta),
                // Reasoning is shown and logged by the caller; it is not
                // part of the answer the conversation carries.
                StreamEvent::Reasoning(_) => {}
                StreamEvent::ToolCalls(calls) => tool_calls = calls.clone(),
                StreamEvent::Usage(counted) => {
                    if counted.finish_reason.is_some() {
                        usage.finish_reason = counted.finish_reason.clone();
                    }
                    usage = counted.clone();
                }
            }
            on_event(event);
        }

        // A pump that ended without a closing event failed; its error is the
        // run's error. A stream that ended without one is a decode failure,
        // exactly as on the non-streaming path.
        let closed = reader
            .await
            .map_err(|error| ClientError::Network(error.to_string()))??;
        if !closed {
            return Err(ClientError::Decode(
                "the stream ended without a completion".to_string(),
            ));
        }
        let usage = Usage {
            total_tokens: if usage.total_tokens == 0 {
                usage.prompt_tokens.saturating_add(usage.completion_tokens)
            } else {
                usage.total_tokens
            },
            // KI-019: the streamed path's finish_reason arrived as `None`
            // whenever the provider carried it on the choice rather than
            // inside the usage object, so the assembled response disagreed
            // with the non-streaming one. The API's own value wins when it
            // is there; otherwise the message says which it was — calls
            // assembled means `tool_calls`, and a clean close with none
            // means `stop`. A provider that sends nothing else and a
            // truncated stream are both decode failures before this point.
            finish_reason: usage.finish_reason.or_else(|| {
                Some(if tool_calls.is_empty() {
                    "stop".to_string()
                } else {
                    "tool_calls".to_string()
                })
            }),
            ..usage
        };
        Ok(ChatResponse {
            choices: vec![Choice {
                message: Message {
                    role: Role::Assistant,
                    content: (!content.is_empty()).then_some(content),
                    tool_calls,
                    tool_call_id: None,
                    images: Vec::new(),
                },
                finish_reason: usage.finish_reason.clone(),
            }],
            usage,
        })
    }
}

/// The blocking half of a streamed exchange: send the request, then hand
/// ureq's body reader to the SSE parser. Returns whether the stream closed
/// cleanly.
fn stream_response(
    agent: &ureq::Agent,
    url: &str,
    api_key: &str,
    body: &[u8],
    events: &tokio::sync::mpsc::UnboundedSender<StreamEvent>,
) -> Result<bool, ClientError> {
    let response = agent
        .post(url)
        .set("Authorization", &format!("Bearer {api_key}"))
        .set("Content-Type", "application/json")
        .send_bytes(body)
        .map_err(map_send_error)?;
    let status = response.status();
    if status != 200 {
        // An error answer is one document, not a stream.
        let text = response
            .into_string()
            .map_err(|error| ClientError::Network(error.to_string()))?;
        return Err(ClientError::Api(api_error(status, &text)));
    }
    parse_sse(std::io::BufReader::new(response.into_reader()), events)
}

/// Read an SSE body line by line, emitting one event per delta and the
/// assembled tool calls at the finish. Blank lines, comments, and non-`data`
/// fields are skipped; `data: [DONE]` closes the stream. Returns whether it
/// closed cleanly, so a truncated body is a decode failure rather than a
/// silently empty answer.
fn parse_sse(
    reader: impl std::io::BufRead,
    events: &tokio::sync::mpsc::UnboundedSender<StreamEvent>,
) -> Result<bool, ClientError> {
    let mut drafts: Vec<ToolCallDraft> = Vec::new();
    let mut closed = false;
    for line in reader.lines() {
        let line = line.map_err(|error| ClientError::Network(error.to_string()))?;
        let Some(payload) = sse_data(&line) else {
            continue;
        };
        if payload == "[DONE]" {
            // A stream can close without a finish_reason; the calls it
            // assembled are still the model's, and dropping them would lose
            // them silently (KI-020).
            if !drafts.is_empty() {
                let calls: Vec<ToolCall> = drafts.iter().map(ToolCall::from).collect();
                if events.send(StreamEvent::ToolCalls(calls)).is_err() {
                    return Ok(true);
                }
            }
            closed = true;
            break;
        }
        let chunk: Chunk = serde_json::from_str(payload)
            .map_err(|error| ClientError::Decode(format!("bad SSE chunk: {error}")))?;
        // The usage-only final chunk (empty choices) carries the exact
        // accounting; a usage-bearing content chunk would carry it too.
        if let Some(counted) = chunk.usage {
            // A dropped receiver means the caller stopped caring; reading on
            // would only block on a channel nobody reads.
            if events.send(StreamEvent::Usage(counted)).is_err() {
                return Ok(true);
            }
        }
        for choice in &chunk.choices {
            if let Some(reasoning) = &choice.delta.reasoning_content {
                if !reasoning.is_empty()
                    && events
                        .send(StreamEvent::Reasoning(reasoning.clone()))
                        .is_err()
                {
                    return Ok(true);
                }
            }
            if let Some(delta) = &choice.delta.content {
                if !delta.is_empty() && events.send(StreamEvent::Content(delta.clone())).is_err() {
                    return Ok(true);
                }
            }
            for call in &choice.delta.tool_calls {
                // Indices arrive in order from a call's first fragment on;
                // that first fragment names it, later ones extend it.
                // Conforming streams number a call's fragments 0,1,2… in
                // order; a gap would silently merge fragments into the wrong
                // call, so it is a decode failure rather than a wrong tool
                // call (KI-020).
                if call.index > drafts.len() {
                    return Err(ClientError::Decode(format!(
                        "tool-call fragment index {} arrives before index {}",
                        call.index,
                        drafts.len()
                    )));
                }
                let draft = if call.index < drafts.len() {
                    &mut drafts[call.index]
                } else {
                    drafts.push(ToolCallDraft::default());
                    drafts.last_mut().expect("just pushed a draft")
                };
                if let Some(id) = &call.id {
                    draft.id.push_str(id);
                }
                if let Some(function) = &call.function {
                    if let Some(name) = &function.name {
                        draft.name.push_str(name);
                    }
                    if let Some(arguments) = &function.arguments {
                        draft.arguments.push_str(arguments);
                    }
                }
            }
            if choice.finish_reason.is_some() {
                // The calls are complete here; emit them whole. Clearing the
                // drafts keeps a later [DONE] from emitting them twice.
                if !drafts.is_empty() {
                    let calls: Vec<ToolCall> = drafts.iter().map(ToolCall::from).collect();
                    if events.send(StreamEvent::ToolCalls(calls)).is_err() {
                        return Ok(true);
                    }
                    drafts.clear();
                }
                closed = true;
            }
        }
    }
    Ok(closed)
}

/// The `data:` payload of one SSE line, or `None` for blank lines,
/// heartbeats/comments, and any other field this protocol does not use.
fn sse_data(line: &str) -> Option<&str> {
    let line = line.strip_suffix('\r').unwrap_or(line);
    if line.is_empty() || line.starts_with(':') {
        return None;
    }
    let payload = line.strip_prefix("data:")?;
    Some(payload.strip_prefix(' ').unwrap_or(payload))
}

impl From<&ToolCallDraft> for ToolCall {
    fn from(draft: &ToolCallDraft) -> ToolCall {
        ToolCall {
            id: draft.id.clone(),
            kind: "function".to_string(),
            function: FunctionCall {
                name: draft.name.clone(),
                arguments: draft.arguments.clone(),
            },
        }
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
    fn a_text_message_serializes_as_the_string_it_always_was() {
        // The whole design: a message with no images rides the wire
        // exactly as it did before, so nothing downstream and no stored
        // log changes shape.
        let message = Message::text(Role::User, "hello");
        let json = serde_json::to_value(&message).expect("serialises");
        assert_eq!(json["role"], "user");
        assert_eq!(json["content"], "hello");
        assert!(json.get("images").is_none(), "{json}");
        assert!(json.get("tool_calls").is_none(), "{json}");
        // And it reads back.
        let back: Message = serde_json::from_value(json).expect("deserialises");
        assert_eq!(back, message);
    }

    #[test]
    fn an_image_message_rides_the_parts_array_every_endpoint_reads() {
        let message = Message::text_with_images(
            Role::User,
            "what is in this diagram?",
            vec![ImageRef::png(vec![
                0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a,
            ])],
        );
        let json = serde_json::to_value(&message).expect("serialises");
        let parts = json["content"].as_array().expect("the parts array");
        assert_eq!(parts.len(), 2, "{json}");
        assert_eq!(parts[0]["type"], "text");
        assert_eq!(parts[0]["text"], "what is in this diagram?");
        assert_eq!(parts[1]["type"], "image_url");
        let url = parts[1]["image_url"]["url"].as_str().expect("the data url");
        assert!(url.starts_with("data:image/png;base64,"), "{url}");
        // The PNG's own eight-byte signature, base64-encoded.
        assert!(url.ends_with("iVBORw0KGgo="), "{url}");
    }

    #[test]
    fn an_image_message_round_trips_through_the_log() {
        // The log holds what was sent, not a derived URL: read back, the
        // bytes are the bytes.
        let original = Message::text_with_images(
            Role::User,
            "look",
            vec![ImageRef::new("image/jpeg", vec![1, 2, 3, 4, 5])],
        );
        let text = serde_json::to_string(&original).expect("serialises");
        let back: Message = serde_json::from_str(&text).expect("reads back");
        assert_eq!(back, original);
        assert_eq!(back.images[0].media, "image/jpeg");
        assert_eq!(back.images[0].data, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn base64_round_trips_and_pads() {
        // The RFC's own test vectors.
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foob"), "Zm9vYg==");
        assert_eq!(base64(b"fooba"), "Zm9vYmE=");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
        // And the inverse agrees, including the padded cases.
        for input in [
            b"" as &[u8],
            b"f",
            b"fo",
            b"foo",
            b"foob",
            b"fooba",
            b"foobar",
        ] {
            assert_eq!(
                decode_base64(&base64(input)).as_deref(),
                Some(input),
                "{input:?}"
            );
        }
        // Input that is not base64 names itself rather than decoding to
        // nothing.
        assert_eq!(decode_base64("!!!!"), None);
    }

    #[test]
    fn an_image_that_arrived_by_url_is_not_pretended_to_be_inline() {
        // A part carrying an http reference is not inline bytes, so the
        // read half drops it rather than storing a URL as though it were
        // data.
        let json = serde_json::json!({
            "role": "user",
            "content": [{
                "type": "image_url",
                "image_url": {"url": "https://example.com/a.png"}
            }]
        });
        let back: Message = serde_json::from_value(json).expect("reads back");
        assert!(back.images.is_empty(), "{back:?}");
        assert_eq!(back.content, None);
    }

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
        let error = api_error(
            500,
            "<html>not json at all, and long enough to matter</html>",
        );
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

    #[test]
    fn stream_request_carries_the_stream_and_usage_flags() {
        let request = ChatRequest {
            model: "test-model",
            messages: &[Message::text(Role::User, "hi")],
            tools: Vec::new(),
        };
        let line = serde_json::to_string(&StreamRequest {
            request: &request,
            stream: true,
            stream_options: StreamOptions {
                include_usage: true,
            },
        })
        .unwrap();
        let body: serde_json::Value = serde_json::from_str(&line).unwrap();
        assert_eq!(body["stream"], true);
        assert_eq!(body["stream_options"]["include_usage"], true);
        assert_eq!(body["model"], "test-model");
        assert_eq!(body["messages"][0]["role"], "user");
    }

    /// Drive the parser over a body, collecting the events it emitted.
    fn parse(body: &str) -> (Vec<StreamEvent>, bool) {
        let (events, mut receiver) = tokio::sync::mpsc::unbounded_channel::<StreamEvent>();
        // The parser is synchronous; drain the channel without a runtime by
        // reading what a blocking send already delivered.
        let closed = parse_sse(std::io::Cursor::new(body.as_bytes()), &events).unwrap();
        drop(events);
        let mut collected = Vec::new();
        while let Ok(event) = receiver.try_recv() {
            collected.push(event);
        }
        (collected, closed)
    }

    #[test]
    fn sse_data_reads_payloads_and_skips_the_rest() {
        assert_eq!(sse_data("data: {\"a\":1}"), Some("{\"a\":1}"));
        assert_eq!(sse_data("data:{\"a\":1}"), Some("{\"a\":1}"));
        assert_eq!(sse_data("data: [DONE]"), Some("[DONE]"));
        assert_eq!(sse_data(""), None);
        assert_eq!(sse_data(": heartbeat"), None);
        assert_eq!(sse_data("event: message"), None);
        assert_eq!(sse_data("id: 7"), None);
    }

    #[test]
    fn parse_sse_emits_content_deltas_and_the_final_usage() {
        let body = concat!(
            ": keep-alive\n",
            "\n",
            "data: {\"choices\":[{\"delta\":{\"content\":\"Hel\"}}]}\n",
            "\n",
            "data: {\"choices\":[{\"delta\":{\"content\":\"lo\"},\"finish_reason\":\"stop\"}]}\n",
            "\n",
            "data: {\"choices\":[],\"usage\":{\"prompt_tokens\":10,\"completion_tokens\":2,\"total_tokens\":12}}\n",
            "\n",
            "data: [DONE]\n",
            "\n",
        );
        let (events, closed) = parse(body);
        assert!(closed);
        assert_eq!(
            events,
            vec![
                StreamEvent::Content("Hel".to_string()),
                StreamEvent::Content("lo".to_string()),
                StreamEvent::Usage(Usage {
                    prompt_tokens: 10,
                    completion_tokens: 2,
                    total_tokens: 12,
                    finish_reason: None,
                }),
            ]
        );
    }

    #[test]
    fn parse_sse_emits_reasoning_beside_the_answer() {
        let body = concat!(
            "data: {\"choices\":[{\"delta\":{\"reasoning_content\":\"the ledger asks\"}}]}\n\n",
            "data: {\"choices\":[{\"delta\":{\"reasoning_content\":\" for the count\"}}]}\n\n",
            "data: {\"choices\":[{\"delta\":{\"content\":\"It is 3.\"}}]}\n\n",
            "data: [DONE]\n\n",
        );
        let (events, closed) = parse(body);
        assert!(closed);
        assert_eq!(
            events,
            vec![
                StreamEvent::Reasoning("the ledger asks".to_string()),
                StreamEvent::Reasoning(" for the count".to_string()),
                StreamEvent::Content("It is 3.".to_string()),
            ]
        );
        // The assembled answer carries only the content: reasoning is
        // display material and never rides back to the provider.
        let answer: String = events
            .iter()
            .filter_map(|event| match event {
                StreamEvent::Content(text) => Some(text.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(answer, "It is 3.");
    }

    #[test]
    fn parse_sse_assembles_tool_call_deltas_keyed_by_index() {
        let body = concat!(
            "data: {\"choices\":[{\"delta\":{\"tool_calls\":[{\"index\":0,\"id\":\"call-1\",",
            "\"type\":\"function\",\"function\":{\"name\":\"ba\",\"arguments\":\"{\\\"comm\"}}]}}]}\n",
            "\n",
            "data: {\"choices\":[{\"delta\":{\"tool_calls\":[{\"index\":0,",
            "\"function\":{\"name\":\"sh\",\"arguments\":\"and\\\":1}\"}}]}}]}\n",
            "\n",
            "data: {\"choices\":[{\"delta\":{},\"finish_reason\":\"tool_calls\"}]}\n",
            "\n",
            "data: {\"choices\":[],\"usage\":{\"prompt_tokens\":3,\"completion_tokens\":4,\"total_tokens\":7}}\n",
            "\n",
            "data: [DONE]\n",
            "\n",
        );
        let (events, closed) = parse(body);
        assert!(closed);
        let calls = events
            .iter()
            .find_map(|event| match event {
                StreamEvent::ToolCalls(calls) => Some(calls.clone()),
                _ => None,
            })
            .expect("one assembled tool-call event");
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].id, "call-1");
        assert_eq!(calls[0].kind, "function");
        assert_eq!(calls[0].function.name, "bash");
        assert_eq!(calls[0].function.arguments, r#"{"command":1}"#);
        assert!(events.contains(&StreamEvent::Usage(Usage {
            prompt_tokens: 3,
            completion_tokens: 4,
            total_tokens: 7,
            finish_reason: None,
        })));
    }

    #[test]
    fn parse_sse_reports_a_truncated_body_as_unclosed() {
        let body = "data: {\"choices\":[{\"delta\":{\"content\":\"half\"}}]}\n\n";
        let (events, closed) = parse(body);
        assert!(!closed);
        assert_eq!(events, vec![StreamEvent::Content("half".to_string())]);
    }

    #[test]
    fn parse_sse_rejects_a_malformed_chunk() {
        let body = "data: {not json}\n\n";
        let (events, mut receiver) = tokio::sync::mpsc::unbounded_channel::<StreamEvent>();
        let error = parse_sse(std::io::Cursor::new(body.as_bytes()), &events).unwrap_err();
        assert!(matches!(error, ClientError::Decode(_)), "{error:?}");
        drop(events);
        assert!(receiver.try_recv().is_err());
    }

    #[test]
    fn parse_sse_emits_assembled_calls_when_the_stream_closes_without_a_finish_reason() {
        // A [DONE] with no finish_reason still delivers the calls the model
        // asked for: dropping them would lose them silently (KI-020).
        let body = concat!(
            "data: {\"choices\":[{\"delta\":{\"tool_calls\":[{\"index\":0,\"id\":\"call-1\",",
            "\"function\":{\"name\":\"bash\",\"arguments\":\"{}\"}}]}}]}\n",
            "\n",
            "data: [DONE]\n",
            "\n",
        );
        let (events, closed) = parse(body);
        assert!(closed);
        let calls = events
            .iter()
            .find_map(|event| match event {
                StreamEvent::ToolCalls(calls) => Some(calls.clone()),
                _ => None,
            })
            .expect("the assembled call is emitted at close");
        assert_eq!(calls[0].id, "call-1");
        assert_eq!(calls[0].function.name, "bash");
    }

    #[test]
    fn parse_sse_rejects_a_gapped_tool_call_index() {
        // A fragment whose index skips a slot would merge into the wrong
        // call; that is a decode failure, not a silently wrong tool call.
        let body = concat!(
            "data: {\"choices\":[{\"delta\":{\"tool_calls\":[{\"index\":0,\"id\":\"c0\",",
            "\"function\":{\"name\":\"bash\",\"arguments\":\"{}\"}}]}}]}\n",
            "\n",
            "data: {\"choices\":[{\"delta\":{\"tool_calls\":[{\"index\":2,",
            "\"function\":{\"name\":\"bash\",\"arguments\":\"{}\"}}]}}]}\n",
            "\n",
        );
        let (events, mut receiver) = tokio::sync::mpsc::unbounded_channel::<StreamEvent>();
        let error = parse_sse(std::io::Cursor::new(body.as_bytes()), &events).unwrap_err();
        assert!(matches!(error, ClientError::Decode(_)), "{error:?}");
        drop(events);
        assert!(receiver.try_recv().is_err());
    }
}
