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

    /// The declaration's name.
    pub fn name(&self) -> &str {
        &self.function.name
    }

    /// The declaration's description.
    pub fn description(&self) -> &str {
        &self.function.description
    }

    /// The declaration's argument schema — the JSON Schema the model
    /// reads, and the source of a call's defaults (`tool-surface C1`: a
    /// resolution record says which arguments the model gave and which a
    /// default filled).
    pub fn parameters(&self) -> &serde_json::Value {
        &self.function.parameters
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
    /// The model's output ceiling, sent only when the provider declared
    /// one (the provider-models plan's P7): a field fleet never sent
    /// before the settings carried it, and absent for every model whose
    /// listing named no ceiling.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u64>,
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

impl ChatResponse {
    /// Fold the usage's nested cached count into the flat field. The
    /// one call site per chat parse: serde captures OpenAI's details
    /// object on the way in, and this folds it before any reader
    /// looks.
    fn absorbed(mut self) -> ChatResponse {
        self.usage = self.usage.absorbed();
        self
    }
}

#[derive(Debug, Deserialize)]
pub struct Choice {
    pub message: Message,
    #[serde(default)]
    pub finish_reason: Option<String>,
}

/// Token accounting for one request. Absent fields read as zero.
///
/// The cache and reasoning fields are `Option`s, not zeros: a provider
/// that does not report them has said nothing, and a recorded row must
/// not pretend a zero it was never given (the usage-in-calibration
/// plan's C0). They serialize as omitted, so an old reader sees no
/// difference.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Usage {
    #[serde(default)]
    pub prompt_tokens: u64,
    #[serde(default)]
    pub completion_tokens: u64,
    #[serde(default)]
    pub total_tokens: u64,
    /// Tokens the provider served from its prompt cache. The
    /// provider's own accounting — a subset of `prompt_tokens` on
    /// every format confirmed so far, never an addition to it (the
    /// live leg of the usage plan confirms per provider; the reading
    /// lives here, at the definition, so no call site re-derives it).
    /// The wire names are the providers' own, read per format and
    /// never per model: the Anthropic name, DeepSeek's
    /// `prompt_cache_hit_tokens` (the served-from-cache half of its
    /// hit/miss split), and OpenAI's nested
    /// `prompt_tokens_details.cached_tokens` — the last two land here
    /// through the alias and the details capture below, so whichever
    /// provider the router picked, the breakdown reads the same. A
    /// provider that reports none of them leaves this `None`.
    #[serde(
        default,
        alias = "prompt_cache_hit_tokens",
        rename = "cache_read_input_tokens",
        skip_serializing_if = "Option::is_none"
    )]
    pub cache_read_tokens: Option<u64>,
    /// Tokens the provider wrote into its cache this request.
    #[serde(
        default,
        rename = "cache_creation_input_tokens",
        skip_serializing_if = "Option::is_none"
    )]
    pub cache_creation_tokens: Option<u64>,
    /// OpenAI-format providers nest their cached count inside a
    /// details object. Captured here, folded into
    /// [`Usage::cache_read_tokens`] by [`Usage::absorbed`] at the two
    /// chat parse sites, and never serialized — the wire shape is
    /// unchanged. Public only so an external test can name it in a
    /// struct literal; readers should use
    /// [`Usage::cache_read_tokens`] after [`Usage::absorbed`].
    #[serde(default, skip_serializing)]
    pub prompt_tokens_details: Option<PromptTokensDetails>,
    /// Tokens the model spent reasoning, when the provider reports
    /// them separately (DeepSeek's `reasoning_tokens` shape).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning_tokens: Option<u64>,
    #[serde(default)]
    pub finish_reason: Option<String>,
}

impl Usage {
    /// The tokens this request is billed on: prompt plus completion.
    /// The cache fields are the provider's breakdown of `prompt_tokens`
    /// (reads cached, writes created), not a second charge — the
    /// provider-by-provider confirmation rides the usage plan's live
    /// leg, and any provider whose accounting contradicts this gets
    /// its reading named here.
    pub fn billed_tokens(&self) -> u64 {
        self.prompt_tokens.saturating_add(self.completion_tokens)
    }

    /// Fold OpenAI's nested cached count into the flat field, leaving
    /// the details capture empty. A direct field (Anthropic's,
    /// DeepSeek's) wins when present: the details object is only the
    /// count's other home, never a second accounting.
    pub fn absorbed(mut self) -> Usage {
        if self.cache_read_tokens.is_none() {
            if let Some(details) = self.prompt_tokens_details.take() {
                self.cache_read_tokens = details.cached_tokens;
            }
        } else {
            self.prompt_tokens_details = None;
        }
        self
    }
}

/// OpenAI's `prompt_tokens_details` object: the home of
/// `cached_tokens` on the chat-completions format.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub struct PromptTokensDetails {
    #[serde(default)]
    pub cached_tokens: Option<u64>,
}

/// A non-2xx API answer.
#[derive(Debug)]
pub struct ApiError {
    pub status: u16,
    pub message: String,
    /// The provider's own per-request diagnostic id, from a bounded
    /// allowlist of response headers — the id support channels ask for
    /// when a refusal is reported upstream. `None` when the answer
    /// carried none of them.
    pub request_id: Option<String>,
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let hint = match self.status {
            401 => " — check DEEPSEEK_API_KEY",
            402 => " — the account needs a balance top-up",
            429 => " — slow down or retry later",
            _ => "",
        };
        match &self.request_id {
            Some(id) => write!(
                f,
                "API error {}: {}{} — request id {id}",
                self.status, self.message, hint
            ),
            None => write!(f, "API error {}: {}{}", self.status, self.message, hint),
        }
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
    /// Extra headers every request carries beside `Authorization`, in the
    /// roster's resolved order — the provider-session-headers plan. The
    /// pairs arrive fully resolved (placeholders substituted by the
    /// router's build) and never carry a reserved name: both the settings
    /// writer and the router's build refuse one.
    headers: Vec<(String, String)>,
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
        Client::with_headers(base_url, api_key, Vec::new())
    }

    /// A client that carries extra headers beside `Authorization` on every
    /// request — one provider's resolved roster `headers`. The pairs are
    /// sent after `Authorization`, in the order given, on both the plain
    /// and the streamed path.
    pub fn with_headers(
        base_url: impl Into<String>,
        api_key: impl Into<String>,
        headers: Vec<(String, String)>,
    ) -> Client {
        // A pasted endpoint (`…/v1/chat/completions`) is normalized to the
        // base it names: the request paths are built as `{base}/…`, so an
        // unnormalized paste would ask `…/chat/completions/chat/completions`.
        let base_url: String = base_url.into();
        let base_url = crate::settings::normalize_base_url(&base_url);
        Client {
            base_url,
            api_key: api_key.into(),
            headers,
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
            headers: Vec::new(),
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
        let mut call = self
            .agent
            .post(&url)
            .set("Authorization", &format!("Bearer {}", self.api_key));
        for (name, value) in &self.headers {
            call = call.set(name, value);
        }
        let response = call.send_json(request).map_err(map_send_error)?;
        let status = response.status();
        if status != 200 {
            let request_id = diagnostic_id(&response);
            let body = response
                .into_string()
                .map_err(|error| ClientError::Network(error.to_string()))?;
            return Err(ClientError::Api(api_error(status, &body, request_id)));
        }
        let body = response
            .into_string()
            .map_err(|error| ClientError::Network(error.to_string()))?;
        serde_json::from_str(&body)
            .map(ChatResponse::absorbed)
            .map_err(|error| ClientError::Decode(error.to_string()))
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
        on_event: impl FnMut(StreamEvent),
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
        let headers = self.headers.clone();

        let (events, receiver) = tokio::sync::mpsc::unbounded_channel::<StreamEvent>();
        let reader = tokio::task::spawn_blocking(move || {
            let outcome = stream_response(&agent, &url, &key, &body, &headers, &events);
            // Drop the sender inside the blocking task, so a read failure or
            // a dropped receiver both end the pump instead of deadlocking.
            drop(events);
            outcome
        });

        pump_stream(receiver, reader, on_event).await
    }

    /// One streamed exchange over the Responses protocol — the
    /// dialect-only reading: `store: false`, no `previous_response_id`,
    /// the full history every turn. The typed events map onto the same
    /// [`StreamEvent`] vocabulary the chat path pumps, so the assembly
    /// below is shared byte for byte.
    pub(crate) async fn chat_stream_responses(
        &self,
        request: &ChatRequest<'_>,
        on_event: impl FnMut(StreamEvent),
    ) -> Result<ChatResponse, ClientError> {
        let url = format!("{}/responses", self.base_url);
        let key = self.api_key.clone();
        let body = serde_json::to_vec(&responses_request_value(request, true))
            .map_err(|error| ClientError::Decode(error.to_string()))?;
        let agent = self.agent.clone();
        let headers = self.headers.clone();

        let (events, receiver) = tokio::sync::mpsc::unbounded_channel::<StreamEvent>();
        let reader = tokio::task::spawn_blocking(move || {
            let outcome = stream_responses_response(&agent, &url, &key, &body, &headers, &events);
            drop(events);
            outcome
        });

        pump_stream(receiver, reader, on_event).await
    }
}

/// The pump every streamed exchange shares: drain the channel into the
/// assembled answer while the caller sees each event live, wait for the
/// blocking reader, and demand a clean close. The chat-completions and
/// Responses mappers differ only in what they emit into the channel;
/// this assembly is byte-for-byte shared.
async fn pump_stream(
    mut receiver: tokio::sync::mpsc::UnboundedReceiver<StreamEvent>,
    reader: tokio::task::JoinHandle<Result<bool, ClientError>>,
    mut on_event: impl FnMut(StreamEvent),
) -> Result<ChatResponse, ClientError> {
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

/// The Responses-dialect body for one chat request: the system frames
/// become `instructions`, the history becomes typed input items, and
/// the tool results become `function_call_output` items referencing
/// their call ids. `store: false` and no `previous_response_id`,
/// always — the conversation's state lives in Fleet's session log,
/// never on the server (hard rule 5; the session-parity-remainders
/// plan's spine).
fn responses_request_value(request: &ChatRequest<'_>, stream: bool) -> serde_json::Value {
    let mut instructions: Vec<&str> = Vec::new();
    let mut input: Vec<serde_json::Value> = Vec::new();
    for message in request.messages {
        match message.role {
            Role::System => {
                if let Some(text) = &message.content {
                    if !text.is_empty() {
                        instructions.push(text);
                    }
                }
            }
            Role::User => {
                let mut parts: Vec<serde_json::Value> = Vec::new();
                if let Some(text) = &message.content {
                    if !text.is_empty() {
                        parts.push(serde_json::json!({ "type": "input_text", "text": text }));
                    }
                }
                for image in &message.images {
                    parts.push(serde_json::json!({
                        "type": "input_image",
                        "image_url": image.data_url(),
                    }));
                }
                input.push(serde_json::json!({ "role": "user", "content": parts }));
            }
            Role::Assistant => {
                if let Some(text) = &message.content {
                    if !text.is_empty() {
                        input.push(serde_json::json!({
                            "role": "assistant",
                            "content": [{ "type": "output_text", "text": text }],
                        }));
                    }
                }
                for call in &message.tool_calls {
                    input.push(serde_json::json!({
                        "type": "function_call",
                        "call_id": call.id,
                        "name": call.function.name,
                        "arguments": call.function.arguments,
                    }));
                }
            }
            Role::Tool => {
                input.push(serde_json::json!({
                    "type": "function_call_output",
                    "call_id": message.tool_call_id,
                    "output": message.content,
                }));
            }
        }
    }
    let tools: Vec<serde_json::Value> = request
        .tools
        .iter()
        .map(|spec| {
            serde_json::json!({
                "type": "function",
                "name": spec.function.name,
                "description": spec.function.description,
                "parameters": spec.function.parameters,
                "strict": false,
            })
        })
        .collect();
    let mut body = serde_json::json!({
        "model": request.model,
        "input": input,
        "store": false,
        "stream": stream,
    });
    if !instructions.is_empty() {
        body["instructions"] = serde_json::Value::String(instructions.join("\n\n"));
    }
    if !tools.is_empty() {
        body["tools"] = serde_json::Value::Array(tools);
    }
    body
}

/// The Responses document → the assembled `ChatResponse`: `output`
/// items become the choice's content and tool calls, and the protocol's
/// own usage accounting rides across. The finish reason follows the
/// shape the chat path assembles: calls present means `tool_calls`.
fn responses_response_value(document: &serde_json::Value) -> Result<ChatResponse, ClientError> {
    let mut content = String::new();
    let mut calls: Vec<ToolCall> = Vec::new();
    let mut finish = "stop";
    if let Some(items) = document.get("output").and_then(|value| value.as_array()) {
        for item in items {
            match item.get("type").and_then(|kind| kind.as_str()) {
                Some("message") => {
                    if let Some(parts) = item.get("content").and_then(|value| value.as_array()) {
                        for part in parts {
                            if part.get("type").and_then(|kind| kind.as_str())
                                == Some("output_text")
                            {
                                if let Some(text) = part.get("text").and_then(|t| t.as_str()) {
                                    content.push_str(text);
                                }
                            }
                        }
                    }
                }
                Some("function_call") => {
                    finish = "tool_calls";
                    calls.push(ToolCall {
                        id: item
                            .get("call_id")
                            .and_then(|value| value.as_str())
                            .unwrap_or_default()
                            .to_string(),
                        kind: "function".to_string(),
                        function: FunctionCall {
                            name: item
                                .get("name")
                                .and_then(|value| value.as_str())
                                .unwrap_or_default()
                                .to_string(),
                            arguments: item
                                .get("arguments")
                                .and_then(|value| value.as_str())
                                .unwrap_or_default()
                                .to_string(),
                        },
                    });
                }
                _ => {}
            }
        }
    }
    let usage_json = document.get("usage").cloned().unwrap_or_default();
    let token = |name: &str| {
        usage_json
            .get(name)
            .and_then(|value| value.as_u64())
            .unwrap_or(0)
    };
    let usage = Usage {
        prompt_tokens: token("input_tokens"),
        completion_tokens: token("output_tokens"),
        total_tokens: token("total_tokens"),
        finish_reason: Some(finish.to_string()),

        cache_read_tokens: None,
        cache_creation_tokens: None,
        reasoning_tokens: None,
        prompt_tokens_details: None,
    };
    Ok(ChatResponse {
        choices: vec![Choice {
            message: Message {
                role: Role::Assistant,
                content: (!content.is_empty()).then_some(content),
                tool_calls: calls,
                tool_call_id: None,
                images: Vec::new(),
            },
            finish_reason: Some(finish.to_string()),
        }],
        usage,
    })
}

/// The blocking half of a streamed Responses exchange: send the request,
/// then hand the body reader to the typed-event parser. Returns whether
/// the stream closed cleanly.
fn stream_responses_response(
    agent: &ureq::Agent,
    url: &str,
    api_key: &str,
    body: &[u8],
    headers: &[(String, String)],
    events: &tokio::sync::mpsc::UnboundedSender<StreamEvent>,
) -> Result<bool, ClientError> {
    let mut call = agent
        .post(url)
        .set("Authorization", &format!("Bearer {api_key}"))
        .set("Content-Type", "application/json");
    for (name, value) in headers {
        call = call.set(name, value);
    }
    let response = call.send_bytes(body).map_err(map_send_error)?;
    let status = response.status();
    if status != 200 {
        let request_id = diagnostic_id(&response);
        let text = response
            .into_string()
            .map_err(|error| ClientError::Network(error.to_string()))?;
        return Err(ClientError::Api(api_error(status, &text, request_id)));
    }
    parse_responses_sse(std::io::BufReader::new(response.into_reader()), events)
}

/// Read a Responses stream line by line, mapping the typed SSE events
/// onto the shared [`StreamEvent`] vocabulary. Text and reasoning
/// deltas stream live; the tool calls ride the terminal
/// `response.completed` event's own output — the authoritative whole
/// call, so the argument-delta fragments are consumed without effect
/// and can never double-assemble. `response.completed` closes; a
/// `response.failed` or `error` event is the typed failure.
fn parse_responses_sse(
    reader: impl std::io::BufRead,
    events: &tokio::sync::mpsc::UnboundedSender<StreamEvent>,
) -> Result<bool, ClientError> {
    let mut closed = false;
    for line in reader.lines() {
        let line = line.map_err(|error| ClientError::Network(error.to_string()))?;
        let Some(payload) = sse_data(&line) else {
            continue;
        };
        if payload == "[DONE]" {
            closed = true;
            break;
        }
        let event: serde_json::Value = serde_json::from_str(payload)
            .map_err(|error| ClientError::Decode(format!("bad SSE event: {error}")))?;
        match event.get("type").and_then(|kind| kind.as_str()) {
            Some("response.output_text.delta") => {
                if let Some(delta) = event.get("delta").and_then(|value| value.as_str()) {
                    if !delta.is_empty()
                        && events
                            .send(StreamEvent::Content(delta.to_string()))
                            .is_err()
                    {
                        return Ok(true);
                    }
                }
            }
            Some("response.reasoning_summary_text.delta") => {
                if let Some(delta) = event.get("delta").and_then(|value| value.as_str()) {
                    if !delta.is_empty()
                        && events
                            .send(StreamEvent::Reasoning(delta.to_string()))
                            .is_err()
                    {
                        return Ok(true);
                    }
                }
            }
            // Argument fragments stream live but carry nothing the
            // terminal event does not carry whole; consumed here so an
            // unknown-type fallthrough never mistakes them.
            Some("response.function_call_arguments.delta") => {}
            Some("response.completed") => {
                let document = event.get("response").cloned().unwrap_or(event.clone());
                let assembled = responses_response_value(&document)?;
                if let Some(choice) = assembled.choices.first() {
                    if !choice.message.tool_calls.is_empty()
                        && events
                            .send(StreamEvent::ToolCalls(choice.message.tool_calls.clone()))
                            .is_err()
                    {
                        return Ok(true);
                    }
                }
                if events.send(StreamEvent::Usage(assembled.usage)).is_err() {
                    return Ok(true);
                }
                closed = true;
                break;
            }
            Some("response.failed") | Some("error") => {
                let message = event
                    .get("response")
                    .and_then(|response| response.get("error"))
                    .and_then(|error| error.get("message"))
                    .and_then(|message| message.as_str())
                    .or_else(|| event.get("message").and_then(|m| m.as_str()))
                    .unwrap_or("the stream answered with a failure event")
                    .to_string();
                return Err(ClientError::Decode(message));
            }
            _ => {}
        }
    }
    Ok(closed)
}

/// The blocking half of a streamed exchange: send the request, then hand
/// ureq's body reader to the SSE parser. Returns whether the stream closed
/// cleanly.
fn stream_response(
    agent: &ureq::Agent,
    url: &str,
    api_key: &str,
    body: &[u8],
    headers: &[(String, String)],
    events: &tokio::sync::mpsc::UnboundedSender<StreamEvent>,
) -> Result<bool, ClientError> {
    let mut call = agent
        .post(url)
        .set("Authorization", &format!("Bearer {api_key}"))
        .set("Content-Type", "application/json");
    for (name, value) in headers {
        call = call.set(name, value);
    }
    let response = call.send_bytes(body).map_err(map_send_error)?;
    let status = response.status();
    if status != 200 {
        // An error answer is one document, not a stream.
        let request_id = diagnostic_id(&response);
        let text = response
            .into_string()
            .map_err(|error| ClientError::Network(error.to_string()))?;
        return Err(ClientError::Api(api_error(status, &text, request_id)));
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
            if events.send(StreamEvent::Usage(counted.absorbed())).is_err() {
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
        ureq::Error::Status(status, response) => {
            let request_id = diagnostic_id(&response);
            match response.into_string() {
                Ok(body) => ClientError::Api(api_error(status, &body, request_id)),
                Err(read) => ClientError::Network(read.to_string()),
            }
        }
        ureq::Error::Transport(transport) => ClientError::Network(transport.to_string()),
    }
}

/// The response-header allowlist a refusal's diagnostic id is read from
/// — the ids providers and gateways ask to see when a request is
/// reported upstream (Anthropic's `request-id`, LiteLLM's call id,
/// OpenCode's log id, Cloudflare's ray). First present wins, capped.
fn diagnostic_id(response: &ureq::Response) -> Option<String> {
    const ALLOWLIST: [&str; 5] = [
        "x-request-id",
        "request-id",
        "x-opencode-log-id",
        "x-litellm-call-id",
        "cf-ray",
    ];
    ALLOWLIST
        .iter()
        .find_map(|name| {
            response
                .header(name)
                .filter(|value| !value.trim().is_empty())
        })
        .map(|value| {
            let mut capped = value.trim().to_string();
            capped.truncate(128);
            capped
        })
}

/// Build an `ApiError` from a raw body, extracting `{"error":{"message":…}}`
/// when present, falling back to a bounded excerpt.
fn api_error(status: u16, body: &str, request_id: Option<String>) -> ApiError {
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
    ApiError {
        status,
        message,
        request_id,
    }
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
    use std::io::{Read, Write};
    use std::sync::Mutex;

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
        let error = api_error(401, r#"{"error":{"message":"auth failed"}}"#, None);
        assert_eq!(error.status, 401);
        assert_eq!(error.message, "auth failed");
        assert!(error.request_id.is_none());
    }

    #[test]
    fn api_error_falls_back_to_a_bounded_excerpt() {
        let error = api_error(
            500,
            "<html>not json at all, and long enough to matter</html>",
            None,
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
            max_tokens: None,
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
        // The provider-models plan's P7: the ceiling rides only when it
        // is declared — the field is absent from an undeclared model's
        // wire, the shape every provider has always received.
        assert!(body.get("max_tokens").is_none(), "{body}");
        let declared = ChatRequest {
            model: "test-model",
            messages: &[Message::text(Role::User, "hi")],
            tools: Vec::new(),
            max_tokens: Some(8_192),
        };
        let declared_wire = serde_json::to_string(&StreamRequest {
            request: &declared,
            stream: false,
            stream_options: StreamOptions {
                include_usage: false,
            },
        })
        .unwrap();
        let declared_body: serde_json::Value = serde_json::from_str(&declared_wire).unwrap();
        assert_eq!(declared_body["max_tokens"], 8_192, "{declared_wire}");
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

                    cache_read_tokens: None,
                    cache_creation_tokens: None,
                    reasoning_tokens: None,
                    prompt_tokens_details: None,
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

            cache_read_tokens: None,
            cache_creation_tokens: None,
            reasoning_tokens: None,
            prompt_tokens_details: None,
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

    /// A stub endpoint that records every request's head — the lines up to
    /// the blank line — and answers `body` with 200. The heads are what
    /// the header pins assert on.
    fn stub_head_capture(body: &'static str) -> (u16, Arc<Mutex<Vec<String>>>) {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let seen: Arc<Mutex<Vec<String>>> = Arc::default();
        let writer = Arc::clone(&seen);
        std::thread::spawn(move || {
            for mut stream in listener.incoming().flatten() {
                let mut request: Vec<u8> = Vec::new();
                let mut chunk = [0u8; 4096];
                let head_end = loop {
                    let read = stream.read(&mut chunk).unwrap_or(0);
                    if read == 0 {
                        break request.len();
                    }
                    request.extend_from_slice(&chunk[..read]);
                    if let Some(at) = request.windows(4).position(|w| w == b"\r\n\r\n") {
                        break at + 4;
                    }
                };
                // Drain the declared body so the client's write completes
                // before the connection closes.
                let head =
                    String::from_utf8_lossy(&request[..head_end.min(request.len())]).to_string();
                let declared: usize = head
                    .to_ascii_lowercase()
                    .lines()
                    .find_map(|line| line.strip_prefix("content-length:"))
                    .and_then(|value| value.trim().parse().ok())
                    .unwrap_or(0);
                while request.len() < head_end + declared {
                    let read = stream.read(&mut chunk).unwrap_or(0);
                    if read == 0 {
                        break;
                    }
                    request.extend_from_slice(&chunk[..read]);
                }
                writer.lock().unwrap().push(head);
                let reply = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = stream.write_all(reply.as_bytes());
            }
        });
        (port, seen)
    }

    fn header_lines(head: &str) -> Vec<String> {
        head.lines()
            .skip(1) // the request line
            .take_while(|line| !line.is_empty())
            .map(|line| line.trim().to_ascii_lowercase())
            .collect()
    }

    /// A stub endpoint that answers every request with `status` and the
    /// given error body, optionally carrying an `x-request-id` response
    /// header — the refusal the request-id pins assert against. A 200
    /// answers a parseable chat response with the same header present.
    fn stub_refusal(status: u16, request_id: Option<&str>) -> u16 {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let id_line = request_id
            .map(|id| format!("x-request-id: {id}\r\n"))
            .unwrap_or_default();
        let body = r#"{"error":{"message":"MissingSessionID","code":"MissingSessionID"}}"#;
        let ok_body = r#"{"choices":[{"message":{"role":"assistant","content":"ok"},"finish_reason":"stop"}],"usage":{"prompt_tokens":1,"completion_tokens":1,"total_tokens":2}}"#;
        let served = if status == 200 { ok_body } else { body };
        std::thread::spawn(move || {
            for mut stream in listener.incoming().flatten() {
                read_and_drain_request(&mut stream);
                let reply = format!(
                    "HTTP/1.1 {status} E\r\nContent-Type: application/json\r\n{id_line}Content-Length: {}\r\nConnection: close\r\n\r\n{}",
                    served.len(),
                    served
                );
                let _ = stream.write_all(reply.as_bytes());
            }
        });
        port
    }

    /// Read one request to its declared end so the client's write
    /// completes before the connection closes. Returns the head.
    fn read_and_drain_request(stream: &mut std::net::TcpStream) -> String {
        let mut request: Vec<u8> = Vec::new();
        let mut chunk = [0u8; 4096];
        let head_end = loop {
            let read = stream.read(&mut chunk).unwrap_or(0);
            if read == 0 {
                break request.len();
            }
            request.extend_from_slice(&chunk[..read]);
            if let Some(at) = request.windows(4).position(|w| w == b"\r\n\r\n") {
                break at + 4;
            }
        };
        let declared: usize = String::from_utf8_lossy(&request[..head_end.min(request.len())])
            .to_ascii_lowercase()
            .lines()
            .find_map(|line| line.strip_prefix("content-length:"))
            .and_then(|value| value.trim().parse().ok())
            .unwrap_or(0);
        while request.len() < head_end + declared {
            let read = stream.read(&mut chunk).unwrap_or(0);
            if read == 0 {
                break;
            }
            request.extend_from_slice(&chunk[..read]);
        }
        String::from_utf8_lossy(&request[..head_end.min(request.len())]).to_string()
    }

    /// A stub that answers per path — the mixed gateway: everything but
    /// `/responses` gets the chat body with a refusal status, `/responses`
    /// gets the SSE answer. Records each request line.
    fn stub_by_path(
        chat_body: &'static str,
        responses_body: String,
    ) -> (u16, Arc<Mutex<Vec<String>>>) {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let seen: Arc<Mutex<Vec<String>>> = Arc::default();
        let writer = Arc::clone(&seen);
        std::thread::spawn(move || {
            for mut stream in listener.incoming().flatten() {
                let head = read_and_drain_request(&mut stream);
                let path = head
                    .lines()
                    .next()
                    .and_then(|line| line.split_whitespace().nth(1))
                    .unwrap_or_default()
                    .to_string();
                writer.lock().unwrap().push(path.clone());
                let (status, body) = if path.ends_with("/responses") {
                    ("HTTP/1.1 200 OK".to_string(), responses_body.clone())
                } else {
                    ("HTTP/1.1 500 E".to_string(), chat_body.to_string())
                };
                let reply = format!(
                    "{status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = stream.write_all(reply.as_bytes());
            }
        });
        (port, seen)
    }

    #[test]
    fn a_configured_header_rides_both_request_paths() {
        let json_answer = r#"{"choices":[{"message":{"role":"assistant","content":"ok"},"finish_reason":"stop"}],"usage":{"prompt_tokens":1,"completion_tokens":1,"total_tokens":2}}"#;
        let sse_answer = concat!(
            "data: {\"choices\":[{\"delta\":{\"content\":\"ok\"}}]}\n\n",
            "data: {\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\"}]}\n\n",
            "data: [DONE]\n\n",
        );
        let (plain_port, plain_seen) = stub_head_capture(json_answer);
        let (stream_port, stream_seen) = stub_head_capture(sse_answer);
        let headers = vec![
            ("x-opencode-session".to_string(), "conv-7".to_string()),
            ("x-gateway-route".to_string(), "go".to_string()),
        ];
        let messages = vec![Message::text(Role::User, "hi")];
        let request = ChatRequest {
            model: "m",
            messages: &messages,
            tools: Vec::new(),
            max_tokens: None,
        };

        // The plain path: both configured headers ride, after
        // `Authorization`, and the key still resolves from the client.
        Client::with_headers(
            format!("http://127.0.0.1:{plain_port}"),
            "sekret",
            headers.clone(),
        )
        .chat(&request)
        .expect("the plain path answers against the stub");
        let heads = plain_seen.lock().unwrap();
        assert_eq!(heads.len(), 1, "one request so far: {:?}", *heads);
        let plain = header_lines(&heads[0]);
        let authorization = plain
            .iter()
            .position(|line| line.starts_with("authorization:"))
            .expect("authorization is set");
        let session = plain
            .iter()
            .position(|line| line.starts_with("x-opencode-session:"))
            .expect("the session header rides");
        let route = plain
            .iter()
            .position(|line| line.starts_with("x-gateway-route:"))
            .expect("the second header rides");
        assert!(authorization < session && session < route, "{plain:?}");
        assert!(
            plain
                .iter()
                .any(|line| line.starts_with("x-opencode-session: conv-7")),
            "{plain:?}"
        );

        // The streamed path: the same headers cross into the blocking
        // task's request, and the stream's Content-Type is still set.
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("a runtime");
        runtime.block_on(async {
            Client::with_headers(format!("http://127.0.0.1:{stream_port}"), "sekret", headers)
                .chat_stream(&request, |_| {})
                .await
                .expect("the streamed path answers against the stub");
        });
        let heads = stream_seen.lock().unwrap();
        assert_eq!(heads.len(), 1, "the streamed request arrived: {:?}", *heads);
        let streamed = header_lines(&heads[0]);
        assert!(
            streamed
                .iter()
                .any(|line| line.starts_with("content-type: application/json")),
            "{streamed:?}"
        );
        assert!(
            streamed
                .iter()
                .any(|line| line.starts_with("x-opencode-session: conv-7")),
            "{streamed:?}"
        );
        assert!(
            streamed
                .iter()
                .any(|line| line.starts_with("x-gateway-route: go")),
            "{streamed:?}"
        );
        assert!(
            streamed
                .iter()
                .any(|line| line.starts_with("authorization: bearer sekret")),
            "{streamed:?}"
        );
    }

    #[test]
    fn a_plain_client_sends_no_extra_headers() {
        // The default shape stays byte-identical: `Client::new` carries no
        // headers, so every existing caller and roster sends exactly what
        // it sent before.
        let body = r#"{"choices":[{"message":{"role":"assistant","content":"ok"},"finish_reason":"stop"}],"usage":{"prompt_tokens":1,"completion_tokens":1,"total_tokens":2}}"#;
        let (port, seen) = stub_head_capture(body);
        let client = Client::new(format!("http://127.0.0.1:{port}"), "k");
        let messages = vec![Message::text(Role::User, "hi")];
        let request = ChatRequest {
            model: "m",
            messages: &messages,
            tools: Vec::new(),
            max_tokens: None,
        };
        client.chat(&request).expect("answers");
        let heads = seen.lock().unwrap();
        assert_eq!(heads.len(), 1);
        let plain = header_lines(&heads[0]);
        assert!(
            plain
                .iter()
                .any(|line| line.starts_with("authorization: bearer k")),
            "{plain:?}"
        );
        assert!(
            !plain.iter().any(|line| line.starts_with("x-")),
            "no configured header rides: {plain:?}"
        );
    }

    #[test]
    fn a_refusal_carries_the_providers_request_id() {
        let messages = vec![Message::text(Role::User, "hi")];
        let request = ChatRequest {
            model: "m",
            messages: &messages,
            tools: Vec::new(),
            max_tokens: None,
        };

        // The plain path: the 400's `x-request-id` rides the error's own
        // words, the id a support channel asks to see.
        let port = stub_refusal(400, Some("req-opencode-123"));
        let error = Client::new(format!("http://127.0.0.1:{port}"), "k")
            .chat(&request)
            .expect_err("the refusal");
        let text = error.to_string();
        assert!(
            text.contains("400") && text.contains("request id req-opencode-123"),
            "{text}"
        );

        // The streamed path keeps the same capture.
        let stream_port = stub_refusal(400, Some("req-stream-9"));
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("a runtime");
        let stream_error = runtime.block_on(async {
            Client::new(format!("http://127.0.0.1:{stream_port}"), "k")
                .chat_stream(&request, |_| {})
                .await
                .expect_err("the streamed refusal")
        });
        assert!(
            stream_error.to_string().contains("request id req-stream-9"),
            "{stream_error}"
        );

        // A 200 carrying the allowlist header parses fine: the capture
        // is an error-path concern, never a success-path one.
        let ok_port = stub_refusal(200, Some("req-ok"));
        let answer = Client::new(format!("http://127.0.0.1:{ok_port}"), "k")
            .chat(&request)
            .expect("the 200 parses");
        assert_eq!(answer.choices[0].message.content.as_deref(), Some("ok"));
    }

    #[test]
    fn usage_reads_the_providers_cache_fields_and_their_absence() {
        // An answer carrying the providers' own wire names records
        // them; one without records nothing, not zeros.
        let with: Usage = serde_json::from_str(
            r#"{"prompt_tokens": 100, "completion_tokens": 20, "total_tokens": 120,
                "cache_read_input_tokens": 80, "cache_creation_input_tokens": 5}"#,
        )
        .expect("parses");
        assert_eq!(with.cache_read_tokens, Some(80));
        assert_eq!(with.cache_creation_tokens, Some(5));
        assert_eq!(with.reasoning_tokens, None);
        assert_eq!(with.billed_tokens(), 120);
        let without: Usage = serde_json::from_str(
            r#"{"prompt_tokens": 100, "completion_tokens": 20, "total_tokens": 120}"#,
        )
        .expect("parses");
        assert_eq!(without.cache_read_tokens, None);
        assert_eq!(without.cache_creation_tokens, None);
        // The absence serializes as omitted, under both names — an old
        // reader sees no difference.
        let json = serde_json::to_value(&without).expect("serializes");
        assert!(json.get("cache_read_input_tokens").is_none(), "{json}");
        assert!(json.get("cache_read_tokens").is_none(), "{json}");
    }

    #[test]
    fn the_cache_breakdown_reads_whatever_provider_served() {
        // DeepSeek's chat-completions answer names the served-from-cache
        // half directly; the alias reads it into the same generic field
        // the Anthropic name lands in. Not a DeepSeek default — a
        // per-format name, read only when that provider serves.
        let deepseek: Usage = serde_json::from_str(
            r#"{"prompt_tokens": 100, "completion_tokens": 20,
                "prompt_cache_hit_tokens": 90, "prompt_cache_miss_tokens": 10}"#,
        )
        .expect("parses");
        assert_eq!(deepseek.cache_read_tokens, Some(90));
        assert_eq!(deepseek.cache_creation_tokens, None);
        // OpenAI nests the count in a details object; the capture holds
        // it on the way in and the fold moves it into the flat field.
        let openai: Usage = serde_json::from_str::<Usage>(
            r#"{"prompt_tokens": 100, "completion_tokens": 20,
                "prompt_tokens_details": {"cached_tokens": 77}}"#,
        )
        .expect("parses")
        .absorbed();
        assert_eq!(openai.cache_read_tokens, Some(77));
        assert!(openai.prompt_tokens_details.is_none());
        // A direct field wins over the details object — one accounting,
        // never two.
        let both: Usage = serde_json::from_str::<Usage>(
            r#"{"prompt_tokens": 100, "completion_tokens": 20,
                "cache_read_input_tokens": 80,
                "prompt_tokens_details": {"cached_tokens": 77}}"#,
        )
        .expect("parses")
        .absorbed();
        assert_eq!(both.cache_read_tokens, Some(80));
        // A provider that reports neither leaves the field absent.
        let none: Usage = serde_json::from_str::<Usage>(
            r#"{"prompt_tokens": 100, "completion_tokens": 20,
                "prompt_tokens_details": {"cached_tokens": null}}"#,
        )
        .expect("parses")
        .absorbed();
        assert_eq!(none.cache_read_tokens, None);
        // And the capture never serializes — the wire shape is
        // unchanged for every reader downstream.
        let json = serde_json::to_value(&openai).expect("serializes");
        assert!(json.get("prompt_tokens_details").is_none(), "{json}");
    }

    #[test]
    fn an_answer_without_the_header_names_nothing() {
        let port = stub_refusal(400, None);
        let messages = vec![Message::text(Role::User, "hi")];
        let request = ChatRequest {
            model: "m",
            messages: &messages,
            tools: Vec::new(),
            max_tokens: None,
        };
        let error = Client::new(format!("http://127.0.0.1:{port}"), "k")
            .chat(&request)
            .expect_err("the refusal");
        assert!(!error.to_string().contains("request id"), "{error:?}");
    }

    /// A history with every message kind, for the request-mapper pins.
    fn full_history() -> (Vec<Message>, Vec<ToolSpec>) {
        let messages = vec![
            Message::text(Role::System, "be brief"),
            Message::text(Role::User, "list the files"),
            Message {
                role: Role::Assistant,
                content: None,
                tool_calls: vec![ToolCall {
                    id: "call-1".to_string(),
                    kind: "function".to_string(),
                    function: FunctionCall {
                        name: "bash".to_string(),
                        arguments: "{}".to_string(),
                    },
                }],
                tool_call_id: None,
                images: Vec::new(),
            },
            Message {
                role: Role::Tool,
                content: Some("a.txt".to_string()),
                tool_calls: Vec::new(),
                tool_call_id: Some("call-1".to_string()),
                images: Vec::new(),
            },
        ];
        let tools = vec![ToolSpec {
            r#type: "function".to_string(),
            function: FunctionSpec {
                name: "bash".to_string(),
                description: "run a command".to_string(),
                parameters: serde_json::json!({"type": "object"}),
            },
        }];
        (messages, tools)
    }

    #[test]
    fn the_responses_request_maps_the_history_statelessly() {
        let (messages, tools) = full_history();
        let request = ChatRequest {
            model: "gpt-6-luna",
            messages: &messages,
            tools,
            max_tokens: None,
        };
        let body = responses_request_value(&request, true);
        // The statelessness is a contract, not a preference: the whole
        // history rides, nothing is stored, nothing is chained.
        assert_eq!(body["store"], false, "{body}");
        assert_eq!(body["stream"], true, "{body}");
        assert!(body.get("previous_response_id").is_none(), "{body}");
        // The system frame became instructions; the user message became
        // a typed input item.
        assert_eq!(body["instructions"], "be brief", "{body}");
        assert_eq!(body["input"][0]["role"], "user", "{body}");
        assert_eq!(body["input"][0]["content"][0]["type"], "input_text");
        // The assistant's call and the tool's answer map to the
        // protocol's own pair, joined by the call id.
        assert_eq!(body["input"][1]["type"], "function_call", "{body}");
        assert_eq!(body["input"][1]["call_id"], "call-1");
        assert_eq!(body["input"][2]["type"], "function_call_output", "{body}");
        assert_eq!(body["input"][2]["call_id"], "call-1");
        assert_eq!(body["input"][2]["output"], "a.txt");
        // The tool declaration is the flat function shape, non-strict.
        assert_eq!(body["tools"][0]["type"], "function", "{body}");
        assert_eq!(body["tools"][0]["name"], "bash");
        assert_eq!(body["tools"][0]["strict"], false, "{body}");
    }

    #[test]
    fn the_responses_answer_maps_back_to_the_chat_shape() {
        let document = serde_json::json!({
            "output": [
                {"type": "message",
                 "content": [{"type": "output_text", "text": "here is the listing"}]},
                {"type": "function_call",
                 "call_id": "call-9", "name": "bash", "arguments": "{\"cmd\":\"ls\"}"}
            ],
            "usage": {"input_tokens": 5, "output_tokens": 7, "total_tokens": 12}
        });
        let answer = responses_response_value(&document).expect("maps");
        let choice = &answer.choices[0];
        assert_eq!(
            choice.message.content.as_deref(),
            Some("here is the listing")
        );
        assert_eq!(choice.message.tool_calls[0].id, "call-9");
        assert_eq!(choice.message.tool_calls[0].function.name, "bash");
        assert_eq!(choice.finish_reason.as_deref(), Some("tool_calls"));
        assert_eq!(answer.usage.prompt_tokens, 5);
        assert_eq!(answer.usage.completion_tokens, 7);
        assert_eq!(answer.usage.total_tokens, 12);
    }

    #[test]
    fn the_responses_stream_maps_the_typed_events() {
        let stream = concat!(
            "data: {\"type\":\"response.output_text.delta\",\"delta\":\"hel\"}\n\n",
            "data: {\"type\":\"response.output_text.delta\",\"delta\":\"lo\"}\n\n",
            "data: {\"type\":\"response.reasoning_summary_text.delta\",\"delta\":\"thinking\"}\n\n",
            "data: {\"type\":\"response.function_call_arguments.delta\",\"item_id\":\"i1\",\"delta\":\"{\\\"\"}\n\n",
            "data: {\"type\":\"response.completed\",\"response\":{\"output\":[{\"type\":\"function_call\",\"call_id\":\"call-1\",\"name\":\"bash\",\"arguments\":\"{\\\"cmd\\\":\\\"ls\\\"}\"}],\"usage\":{\"input_tokens\":5,\"output_tokens\":7,\"total_tokens\":12}}}\n\n",
        );
        let (events, mut receiver) = tokio::sync::mpsc::unbounded_channel::<StreamEvent>();
        let closed =
            parse_responses_sse(std::io::Cursor::new(stream.as_bytes()), &events).expect("parses");
        assert!(closed, "the completed event closes");
        drop(events);
        let mut seen = Vec::new();
        while let Ok(event) = receiver.try_recv() {
            seen.push(event);
        }
        // The text deltas streamed live, in order.
        let content: Vec<&str> = seen
            .iter()
            .filter_map(|event| match event {
                StreamEvent::Content(delta) => Some(delta.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(content, vec!["hel", "lo"], "{seen:?}");
        // The reasoning delta rode its own event.
        assert!(matches!(seen[2], StreamEvent::Reasoning(_)), "{seen:?}");
        // The argument-delta fragment was consumed without effect — the
        // terminal event's own output is the authoritative whole call.
        let calls = seen
            .iter()
            .find_map(|event| match event {
                StreamEvent::ToolCalls(calls) => Some(calls.clone()),
                _ => None,
            })
            .expect("the calls are emitted");
        assert_eq!(calls.len(), 1, "{seen:?}");
        assert_eq!(calls[0].id, "call-1");
        assert_eq!(calls[0].function.name, "bash");
        assert_eq!(calls[0].function.arguments, "{\"cmd\":\"ls\"}");
        // The usage rode the completion with its finish reason.
        let usage = seen
            .iter()
            .find_map(|event| match event {
                StreamEvent::Usage(counted) => Some(counted.clone()),
                _ => None,
            })
            .expect("the usage is emitted");
        assert_eq!(usage.total_tokens, 12);
        assert_eq!(usage.finish_reason.as_deref(), Some("tool_calls"));
    }

    #[tokio::test]
    async fn a_responses_marked_model_rides_the_responses_path() {
        // The mixed gateway, end to end: one candidate whose model is
        // marked `responses`; the walk serves it over /responses and
        // never touches the chat path.
        let sse = concat!(
            "data: {\"type\":\"response.output_text.delta\",\"delta\":\"listing\"}\n\n",
            "data: {\"type\":\"response.completed\",\"response\":{\"output\":[{\"type\":\"message\",\"content\":[{\"type\":\"output_text\",\"text\":\"listing\"}]}],\"usage\":{\"input_tokens\":3,\"output_tokens\":4,\"total_tokens\":7}}}\n\n",
        );
        let (port, seen) = stub_by_path(
            r#"{"error":{"message":"chat path refused"}}"#,
            sse.to_string(),
        );
        let mut candidate = crate::route::default_roster()
            .into_iter()
            .find(|c| c.id == "zai-coding-plan")
            .unwrap();
        candidate.base_url = format!("http://127.0.0.1:{port}");
        candidate.key_required = false;
        candidate.model_protocols.insert(
            "GLM-5.3-Flash".to_string(),
            crate::route::Protocol::Responses,
        );
        let resolved = crate::route::resolve(
            std::slice::from_ref(&candidate),
            &std::collections::HashMap::new(),
        );
        let decision = RouteDecision {
            provider: "zai-coding-plan".to_string(),
            model: "GLM-5.3-Flash".to_string(),
            how: crate::route::How::Judged,
            workload: Some("hard".to_string()),
            verdict: None,
            execution: None,
            reason: "test".to_string(),
            judged_from: crate::route::JudgedFrom::Fresh,
            start_index: 0,
            order: vec![("zai-coding-plan".to_string(), "GLM-5.3-Flash".to_string())],
        };
        let router = Router::new(resolved, decision);
        let message = Message::text(Role::User, "hi");
        let request = ChatRequest {
            model: "GLM-5.3-Flash",
            messages: std::slice::from_ref(&message),
            tools: Vec::new(),
            max_tokens: None,
        };
        let answer = router
            .chat_stream(&request, &mut |_| {})
            .await
            .expect("the responses-marked model serves");
        assert_eq!(
            answer.choices[0].message.content.as_deref(),
            Some("listing")
        );
        let paths = seen.lock().unwrap();
        assert!(
            paths.iter().all(|path| path.ends_with("/responses")),
            "every request rode the responses path: {paths:?}"
        );
    }
}
