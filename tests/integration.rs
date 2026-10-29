//! End-to-end tests for the native agent: the parser's public surface, the
//! HTTP client against a local mock API, and full tool-loop runs that assert
//! both the run report and the session log the model saw. No network beyond
//! loopback; the mock API runs as a child process.
//!
//! The MCP section drives the real vendored sem1 lane through the client
//! end to end (skipped with a loud note when `python3` is absent), and
//! covers the paths the real lanes cannot produce deterministically —
//! stray id-`null` frames, a crashed child, a silent child, out-of-order
//! replies — against a scripted fake MCP server in the mock-server style.
//!
//! The live-API test is `#[ignore]`d like the repo's `test:e2e`: run it with
//! `cargo test --test integration -- --ignored` and `DEEPSEEK_API_KEY` set.

use std::path::{Path, PathBuf};

use fleet::agent::{
    Agent, AgentError, AgentOptions, ApproveAll, Approver, DenyAll, FileAccess, RunReport,
    StopReason, UiEvent,
};
use fleet::api::{
    ChatRequest, Client, ClientError, Message, Role, StreamEvent, ToolCall, Usage, DEFAULT_BASE_URL,
};
use fleet::cli;
use fleet::lanes::{self, Lane, LaneHome, LaneState};
use fleet::mcp::{McpError, McpServer, ServerSpec, Timeouts};
use fleet::route::{Candidate, How, Resolved, RouteDecision, Router};
use fleet::session::{self, Record, Session};
use fleet::tools::{Tool, ALL_TOOLS, MAX_TOOL_OUTPUT};
use std::sync::Arc;

// --- mock API server -------------------------------------------------------

/// One scripted response: an HTTP status line and its JSON document, or a
/// list of raw SSE frames served as a close-delimited body.
#[derive(Debug, Clone)]
struct ScriptedResponse {
    status: String,
    body: String,
    /// Raw `data: …` frame strings; present means "answer as a stream".
    sse: Vec<String>,
    /// Pause between SSE frames, simulating a model that types.
    delay_ms: u64,
}

/// A `200 OK` response carrying `body` as one document.
fn ok(body: String) -> ScriptedResponse {
    ScriptedResponse {
        status: "200 OK".to_string(),
        body,
        sse: Vec::new(),
        delay_ms: 0,
    }
}

/// A `200 OK` streamed response carrying `frames` as an SSE body.
fn stream(frames: Vec<String>) -> ScriptedResponse {
    ScriptedResponse {
        status: "200 OK".to_string(),
        body: String::new(),
        sse: frames,
        delay_ms: 0,
    }
}

/// One recorded HTTP request: the head (request line plus headers) and the
/// body, as logged by the mock-server child.
#[derive(serde::Deserialize)]
struct RecordedRequest {
    #[allow(dead_code)]
    head: String,
    body: String,
}

/// A local chat-completions stand-in, served by the `mock-server` binary in a
/// child process. ureq's reads misbehave here when the client and an
/// in-process loopback server share one process, so the mock runs
/// cross-process: each connection gets the next scripted response with
/// `Connection: close`, and requests are logged to a file the parent reads.
/// When the script runs dry the server answers 500 `mock exhausted`, which
/// fails the requesting test loudly.
struct MockApi {
    child: std::process::Child,
    port: u16,
    scratch: PathBuf,
}

impl MockApi {
    fn start(responses: Vec<ScriptedResponse>) -> MockApi {
        let scratch = temp_home("mock");
        let script_path = scratch.join("responses.jsonl");
        let script = responses
            .iter()
            .map(|response| {
                serde_json::json!({
                    "status": response.status,
                    "body": response.body,
                    "sse": response.sse,
                    "delay_ms": response.delay_ms,
                })
            })
            .map(|line| format!("{line}\n"))
            .collect::<String>();
        std::fs::write(&script_path, script).unwrap();
        let requests_path = scratch.join("requests.jsonl");
        let port_path = scratch.join("port");
        // KI-027: the child's stderr rides a file, not null. A mock that
        // exits under load — the flake this suite carries — leaves its
        // reason where the panic can read it, instead of a child that
        // died silently and a test that names nothing.
        let stderr_path = scratch.join("mock-stderr.log");
        let stderr_file = std::fs::File::create(&stderr_path).unwrap();
        let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_mock-server"))
            .arg(&script_path)
            .arg(&requests_path)
            .arg(&port_path)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::from(stderr_file))
            .spawn()
            .expect("spawn the mock-server child");
        let port = wait_for_port(&port_path, &mut child, &stderr_path);
        MockApi {
            child,
            port,
            scratch,
        }
    }

    fn url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    /// Recorded request bodies, parsed.
    fn bodies(&self) -> Vec<serde_json::Value> {
        let text = std::fs::read_to_string(self.scratch.join("requests.jsonl")).unwrap_or_default();
        text.lines()
            .filter(|line| !line.is_empty())
            .map(|line| {
                let recorded: RecordedRequest =
                    serde_json::from_str(line).expect("a recorded request line");
                serde_json::from_str(&recorded.body).expect("a recorded request body")
            })
            .collect()
    }
}

impl Drop for MockApi {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        std::fs::remove_dir_all(&self.scratch).ok();
    }
}

/// Poll for the child's port file; a child that exits first fails the test.
fn wait_for_port(port_path: &Path, child: &mut std::process::Child, stderr_path: &Path) -> u16 {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        if let Ok(text) = std::fs::read_to_string(port_path) {
            if let Ok(port) = text.trim().parse() {
                return port;
            }
        }
        if child.try_wait().unwrap().is_some() {
            // KI-027: the child's own stderr rides the panic. A dead mock
            // is a fact about the mock, and the reason it died is the one
            // thing the test could not see before.
            let said = std::fs::read_to_string(stderr_path).unwrap_or_default();
            panic!(
                "the mock-server child exited before writing its port; its stderr: {}",
                said.trim()
            );
        }
        assert!(
            std::time::Instant::now() < deadline,
            "the mock-server port never arrived"
        );
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
}

// --- helpers ---------------------------------------------------------------

fn temp_home(tag: &str) -> PathBuf {
    // Unique per call, three ways over: the process id, the clock, and an
    // atomic counter. The clock alone is not enough — two calls in the
    // same nanosecond produce the same `subsec_nanos`, and a collided
    // scratch directory means one test's scripted fixture is another
    // test's, which reads as a protocol error from the wrong server.
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let serial = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "fleet-integration-{tag}-{}-{}-{serial}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn args(list: &[&str]) -> Vec<String> {
    list.iter().map(|item| item.to_string()).collect()
}

/// A plain assistant text response body.
fn assistant_response(content: &str) -> String {
    chat_response(serde_json::json!({ "role": "assistant", "content": content }))
}

/// An assistant response body requesting tool calls.
fn tool_call_response(calls: serde_json::Value) -> String {
    chat_response(serde_json::json!({
        "role": "assistant",
        "content": null,
        "tool_calls": calls,
    }))
}

fn chat_response(message: serde_json::Value) -> String {
    serde_json::json!({
        "choices": [{ "message": message, "finish_reason": "stop" }],
        "usage": { "prompt_tokens": 10, "completion_tokens": 5, "total_tokens": 15 },
    })
    .to_string()
}

fn tool_call(id: &str, name: &str, arguments: &str) -> serde_json::Value {
    serde_json::json!({
        "id": id,
        "type": "function",
        "function": { "name": name, "arguments": arguments },
    })
}

// --- streamed responses ----------------------------------------------------

/// One raw SSE `data:` frame carrying `chunk`.
fn frame(chunk: serde_json::Value) -> String {
    format!("data: {chunk}")
}

/// A content delta frame.
fn content_delta(text: &str) -> String {
    frame(serde_json::json!({ "choices": [{ "delta": { "content": text } }] }))
}

/// A reasoning delta frame: the model's thinking, on the providers that
/// stream it beside the answer.
fn reasoning_delta(text: &str) -> String {
    frame(serde_json::json!({ "choices": [{ "delta": { "reasoning_content": text } }] }))
}

/// A tool-call delta frame: the fragment at `index` of one call.
fn tool_call_delta(index: usize, id: &str, name: &str, arguments: &str) -> String {
    frame(serde_json::json!({
        "choices": [{ "delta": { "tool_calls": [{ "index": index, "id": id,
            "type": "function", "function": { "name": name, "arguments": arguments } }] } }],
    }))
}

/// The frame that closes a streamed answer.
fn finish_frame(reason: &str) -> String {
    frame(serde_json::json!({ "choices": [{ "delta": {}, "finish_reason": reason }] }))
}

/// The usage-only final chunk `include_usage` asks for.
fn usage_frame(prompt: u64, completion: u64) -> String {
    frame(serde_json::json!({
        "choices": [],
        "usage": {
            "prompt_tokens": prompt,
            "completion_tokens": completion,
            "total_tokens": prompt + completion,
        },
    }))
}

/// A complete streamed text answer with the same accounting the
/// non-streaming fixtures use.
fn assistant_stream(content: &str) -> ScriptedResponse {
    stream(vec![
        content_delta(content),
        finish_frame("stop"),
        usage_frame(10, 5),
        "data: [DONE]".to_string(),
    ])
}

/// The final line a tool-running script ends with: a report, not an
/// acknowledgment. The harness's report repair owns the silent-stop
/// cases, and a fixture that ends "done" after a tool call is a run
/// with no answer as far as the loop is concerned.
const TOOL_REPORT: &str = "The run's commands ran and returned their output; the work is complete and nothing needed from you.";

/// A complete streamed answer requesting one tool call, delivered in
/// fragments the way the real API splits a call.
fn tool_call_stream(id: &str, name: &str, arguments: &str) -> ScriptedResponse {
    let (head, tail) = arguments.split_at(arguments.len() / 2);
    stream(vec![
        tool_call_delta(0, id, name, head),
        tool_call_delta(0, "", "", tail),
        finish_frame("tool_calls"),
        usage_frame(10, 5),
        "data: [DONE]".to_string(),
    ])
}

/// One tool-calling turn that reports `prompt` prompt tokens: the shape a
/// provider sends when the conversation is genuinely large, which is the
/// only way a compaction can be driven from the API's own count.
fn tool_call_stream_counting(
    id: &str,
    name: &str,
    arguments: &str,
    prompt: u64,
) -> ScriptedResponse {
    let (head, tail) = arguments.split_at(arguments.len() / 2);
    stream(vec![
        tool_call_delta(0, id, name, head),
        tool_call_delta(0, "", "", tail),
        finish_frame("tool_calls"),
        usage_frame(prompt, 5),
        "data: [DONE]".to_string(),
    ])
}

/// One tool-calling turn that reports no usage at all — a provider that
/// sends nothing, which is the case the chars/4 estimate exists for.
fn tool_call_stream_uncounted(id: &str, name: &str, arguments: &str) -> ScriptedResponse {
    let (head, tail) = arguments.split_at(arguments.len() / 2);
    stream(vec![
        tool_call_delta(0, id, name, head),
        tool_call_delta(0, "", "", tail),
        finish_frame("tool_calls"),
        "data: [DONE]".to_string(),
    ])
}

/// A progress sink that remembers what had already been delivered at every
/// write, so a test can observe the stream mid-run instead of only its end.
struct Probe {
    /// Every write, in arrival order.
    writes: Vec<String>,
    /// The whole buffer as it stood after each write.
    snapshots: Vec<String>,
    buffer: String,
}

impl Probe {
    fn new() -> Probe {
        Probe {
            writes: Vec::new(),
            snapshots: Vec::new(),
            buffer: String::new(),
        }
    }

    /// Everything written, concatenated.
    fn text(&self) -> String {
        self.buffer.clone()
    }
}

impl std::io::Write for Probe {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let text = String::from_utf8_lossy(buf).into_owned();
        self.buffer.push_str(&text);
        self.writes.push(text);
        self.snapshots.push(self.buffer.clone());
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// One full agent run against `mock` on the given session.
///
/// The run's world is pinned: a scratch FLEET_HOME with no config means
/// the arm's resolve finds no machine facts, so a plain delegation never
/// opts into a LayerFS workspace — whatever the launching repository
/// has registered in the real home. (This repository itself became
/// registered mid-plan — the motivating case — and a plain delegation
/// test's premise must not ride on the world it runs in.)
async fn run_on(
    mock: &MockApi,
    session: &mut Session,
    approver: Box<dyn Approver>,
    max_turns: usize,
    task: &str,
) -> Result<RunReport, AgentError> {
    let scratch = temp_home("run-on-fleet-home");
    let _fh = {
        let _lock = AMBIENT_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        AmbientKey::set("FLEET_HOME", scratch.to_str().unwrap())
    };
    let client = Client::new(mock.url(), "test-key");
    let options = AgentOptions {
        goal: None,
        model: "test-model".to_string(),
        identity: None,
        max_turns,
        ui_tx: None,
        lane_specs: None,
        calib: None,
        skills: Vec::new(),
        role_overlay: None,
        memory: None,
        scope: None,
        improve_home: None,
        route_home: None,
        gui_prompt: None,
        compact_now: false,
        checkpoints: None,
        file_access: FileAccess::Unrestricted,
        plan_mode: false,
    };
    let mut progress = Vec::new();
    let mut agent = Agent::new(&client, options, session, approver, &mut progress).unwrap();
    agent.run(task).await
}

/// One full agent run whose progress sink records what it saw and when.
/// `run_on` with the fold forced after the tool batch.
async fn run_on_compacting(
    mock: &MockApi,
    session: &mut Session,
    approver: Box<dyn Approver>,
    task: &str,
) -> Result<RunReport, AgentError> {
    let client = Client::new(mock.url(), "test-key");
    let options = AgentOptions {
        goal: None,
        model: "test-model".to_string(),
        identity: None,
        max_turns: 5,
        ui_tx: None,
        lane_specs: None,
        calib: None,
        skills: Vec::new(),
        role_overlay: None,
        memory: None,
        scope: None,
        improve_home: None,
        route_home: None,
        gui_prompt: None,
        compact_now: true,
        checkpoints: None,
        file_access: FileAccess::Unrestricted,
        plan_mode: false,
    };
    let mut progress = Vec::new();
    let mut agent = Agent::new(&client, options, session, approver, &mut progress).unwrap();
    agent.run(task).await
}

async fn run_on_probe(
    mock: &MockApi,
    session: &mut Session,
    probe: &mut Probe,
    task: &str,
) -> Result<RunReport, AgentError> {
    let client = Client::new(mock.url(), "test-key");
    let options = AgentOptions {
        goal: None,
        model: "test-model".to_string(),
        identity: None,
        max_turns: 5,
        ui_tx: None,
        lane_specs: None,
        calib: None,
        skills: Vec::new(),
        role_overlay: None,
        memory: None,
        scope: None,
        improve_home: None,
        route_home: None,
        gui_prompt: None,
        compact_now: false,
        checkpoints: None,
        file_access: FileAccess::Unrestricted,
        plan_mode: false,
    };
    let mut agent = Agent::new(&client, options, session, Box::new(ApproveAll), probe).unwrap();
    agent.run(task).await
}

/// One full agent run whose lane children come from `lane_specs` rather
/// than the vendored registry. `None` is the production path: a lane
/// resolves through `lanes::spec_for` on its first call.
async fn run_with_lanes(
    mock: &MockApi,
    session: &mut Session,
    approver: Box<dyn Approver>,
    lane_specs: Option<std::collections::HashMap<String, ServerSpec>>,
    progress: &mut dyn std::io::Write,
    task: &str,
) -> Result<RunReport, AgentError> {
    let client = Client::new(mock.url(), "test-key");
    let options = AgentOptions {
        goal: None,
        model: "test-model".to_string(),
        identity: None,
        max_turns: 5,
        ui_tx: None,
        lane_specs,
        calib: None,
        skills: Vec::new(),
        role_overlay: None,
        memory: None,
        scope: None,
        improve_home: None,
        route_home: None,
        gui_prompt: None,
        compact_now: false,
        checkpoints: None,
        file_access: FileAccess::Unrestricted,
        plan_mode: false,
    };
    let mut agent = Agent::new(&client, options, session, approver, progress).unwrap();
    agent.run(task).await
}

/// The scripted fake standing in for the lane a run addresses by name.
fn lane_specs(fake: &FakeMcp) -> std::collections::HashMap<String, ServerSpec> {
    let mut specs = std::collections::HashMap::new();
    specs.insert("sem1".to_string(), fake.spec());
    specs
}

/// A scripted `tools/list` reply carrying `tools`.
fn tools_list_reply(tools: serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": "$echo",
        "result": { "tools": tools },
    })
}

/// A scripted `tools/call` reply carrying one text block.
fn call_reply(text: &str) -> serde_json::Value {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": "$echo",
        "result": { "content": [{ "type": "text", "text": text }], "isError": false },
    })
}

/// The model-visible names in one request body's tool declarations.
/// The first tool result in a replay, wherever the resolution and
/// bookkeeping records sit around it.
fn first_result(records: &[Record]) -> &Record {
    records
        .iter()
        .find(|record| matches!(record, Record::ToolResult { .. }))
        .expect("a tool result")
}

fn declared_tools(body: &serde_json::Value) -> Vec<String> {
    body["tools"]
        .as_array()
        .expect("a tools array")
        .iter()
        .map(|tool| {
            tool["function"]["name"]
                .as_str()
                .unwrap_or_default()
                .to_string()
        })
        .collect()
}

fn message_roles(body: &serde_json::Value) -> Vec<&str> {
    body["messages"]
        .as_array()
        .unwrap()
        .iter()
        .map(|message| message["role"].as_str().unwrap())
        .collect()
}

// --- parser ----------------------------------------------------------------

#[tokio::test]
async fn parse_surfaces_the_documented_commands() {
    assert_eq!(
        cli::parse(&args(&["sessions"])),
        Ok(cli::Command::SessionsView {
            recall: None,
            view: None,
            probes: Vec::new(),
        })
    );
    assert_eq!(
        cli::parse(&args(&["doctor"])),
        Ok(cli::Command::Doctor(cli::DoctorOptions {
            diagnose: None,
            ledger: None,
            open: false
        }))
    );
    assert_eq!(cli::parse(&args(&["version"])), Ok(cli::Command::Version));
    let error = cli::parse(&args(&["launch", "--profile", "web", "computer_use"])).unwrap_err();
    assert!(error.contains("launch"), "{error}");
}

// --- client ----------------------------------------------------------------

#[tokio::test]
async fn client_posts_auth_and_tool_declarations() {
    let mock = MockApi::start(vec![ok(assistant_response("hi"))]);
    let client = Client::new(mock.url(), "test-key");
    let request = ChatRequest {
        model: "test-model",
        messages: &[Message::text(Role::User, "hello")],
        tools: ALL_TOOLS.iter().map(|tool| tool.spec()).collect(),
    };
    let response = client.chat(&request).unwrap();
    assert_eq!(response.choices.len(), 1);
    assert_eq!(response.usage.prompt_tokens, 10);

    let text = std::fs::read_to_string(mock.scratch.join("requests.jsonl")).unwrap();
    let recorded: RecordedRequest = serde_json::from_str(text.lines().next().unwrap()).unwrap();
    assert!(
        recorded.head.starts_with("POST /chat/completions "),
        "{}",
        recorded.head
    );
    assert!(
        recorded.head.contains("Authorization: Bearer test-key"),
        "{}",
        recorded.head
    );
    let body: serde_json::Value = serde_json::from_str(&recorded.body).unwrap();
    assert_eq!(body["model"], "test-model");
    assert_eq!(body["tools"].as_array().unwrap().len(), 11);
    assert_eq!(body["messages"][0]["role"], "user");
}

#[tokio::test]
async fn client_maps_an_api_error_with_its_hint() {
    let mock = MockApi::start(vec![ScriptedResponse {
        status: "401 Unauthorized".to_string(),
        body: r#"{"error":{"message":"auth failed"}}"#.to_string(),
        sse: Vec::new(),
        delay_ms: 0,
    }]);
    let client = Client::new(mock.url(), "test-key");
    let request = ChatRequest {
        model: "test-model",
        messages: &[Message::text(Role::User, "hello")],
        tools: Vec::new(),
    };
    let error = match client.chat(&request) {
        Err(ClientError::Api(error)) => error,
        other => panic!("expected an API error, got {other:?}"),
    };
    assert_eq!(error.status, 401);
    assert_eq!(error.message, "auth failed");
    assert!(error.to_string().contains("DEEPSEEK_API_KEY"));
}

#[tokio::test]
async fn client_chat_still_answers_a_whole_document() {
    // The agent streams, but `chat` stays the one-shot path and must keep
    // assembling tool calls from a plain document.
    let mock = MockApi::start(vec![ok(tool_call_response(serde_json::json!([
        tool_call("call-1", "bash", r#"{"command":"echo hi"}"#,)
    ])))]);
    let client = Client::new(mock.url(), "test-key");
    let request = ChatRequest {
        model: "test-model",
        messages: &[Message::text(Role::User, "hello")],
        tools: Vec::new(),
    };
    let response = client.chat(&request).unwrap();
    let calls = &response.choices[0].message.tool_calls;
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].id, "call-1");
    assert_eq!(calls[0].function.name, "bash");
    assert_eq!(calls[0].function.arguments, r#"{"command":"echo hi"}"#);
    assert_eq!(response.usage.total_tokens, 15);

    let body = mock.bodies().remove(0);
    assert!(
        body.get("stream").is_none(),
        "the one-shot path does not ask for a stream"
    );
    assert!(body.get("stream_options").is_none());
}

#[tokio::test]
async fn client_chat_stream_assembles_the_same_response_chat_returns() {
    let mock = MockApi::start(vec![tool_call_stream(
        "call-1",
        "bash",
        r#"{"command":"echo hi"}"#,
    )]);
    let client = Client::new(mock.url(), "test-key");
    let request = ChatRequest {
        model: "test-model",
        messages: &[Message::text(Role::User, "hello")],
        tools: Vec::new(),
    };
    let mut events = Vec::new();
    let streamed = client
        .chat_stream(&request, |event| events.push(event))
        .await
        .unwrap();
    let calls = &streamed.choices[0].message.tool_calls;
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].id, "call-1");
    assert_eq!(calls[0].function.name, "bash");
    assert_eq!(calls[0].function.arguments, r#"{"command":"echo hi"}"#);
    assert_eq!(streamed.usage.prompt_tokens, 10);
    assert_eq!(streamed.usage.completion_tokens, 5);
    assert_eq!(streamed.usage.total_tokens, 15);
    // KI-019: the streamed response's finish_reason matches the
    // non-streaming one's for the same shape. The provider carried the
    // reason on the choice, not in the usage object, and the streamed
    // path used to assemble `None`.
    assert_eq!(
        streamed.choices[0].finish_reason.as_deref(),
        Some("tool_calls"),
        "the assembled calls are the reason the stream closed"
    );

    let body = mock.bodies().remove(0);
    assert_eq!(body["stream"], true);
    assert_eq!(body["stream_options"]["include_usage"], true);

    // The tool call arrives once, assembled; usage arrives once, at the end.
    let assembled: Vec<&[ToolCall]> = events
        .iter()
        .filter_map(|event| match event {
            StreamEvent::ToolCalls(calls) => Some(calls.as_slice()),
            _ => None,
        })
        .collect();
    assert_eq!(assembled.len(), 1);
    assert_eq!(
        assembled[0][0].function.arguments,
        r#"{"command":"echo hi"}"#
    );
    assert!(events
        .iter()
        .all(|event| !matches!(event, StreamEvent::Content(_))));
    assert!(events.contains(&StreamEvent::Usage(Usage {
        prompt_tokens: 10,
        completion_tokens: 5,
        total_tokens: 15,
        finish_reason: None,
    })));
}

#[tokio::test]
async fn client_chat_stream_derives_stop_when_the_stream_closes_with_calls() {
    // KI-019: the text shape. A stream that closes cleanly with no calls
    // is `stop`, the reason the non-streaming fixture returns for the
    // same answer, so a consumer reading one path is not told a
    // different story by the other.
    let mock = MockApi::start(vec![assistant_stream("All done.")]);
    let client = Client::new(mock.url(), "test-key");
    let request = ChatRequest {
        model: "test-model",
        messages: &[Message::text(Role::User, "hello")],
        tools: Vec::new(),
    };
    let streamed = client.chat_stream(&request, |_| {}).await.unwrap();
    assert_eq!(streamed.choices[0].finish_reason.as_deref(), Some("stop"));
}

// --- the agent loop --------------------------------------------------------

#[tokio::test]
async fn run_completes_a_two_turn_tool_loop() {
    let mock = MockApi::start(vec![
        tool_call_stream("call-1", "bash", r#"{"command":"echo hello"}"#),
        assistant_stream(TOOL_REPORT),
    ]);
    let home = temp_home("two-turn");
    let mut session = Session::create(&home, "test-model").unwrap();
    let report = run_on(&mock, &mut session, Box::new(ApproveAll), 5, "say hello")
        .await
        .unwrap();

    assert_eq!(report.stop_reason, StopReason::Completed);
    assert_eq!(report.turns, 2);
    assert_eq!(report.final_message.as_deref(), Some(TOOL_REPORT));
    assert_eq!(report.usage.prompt_tokens, 20);
    assert_eq!(report.usage.completion_tokens, 10);

    // Both requests asked for a stream with usage, the accounting contract.
    let bodies = mock.bodies();
    assert_eq!(bodies.len(), 2);
    for body in &bodies {
        assert_eq!(body["stream"], true);
        assert_eq!(body["stream_options"]["include_usage"], true);
    }

    // The second request sees the system prompt, the task, the assistant's
    // call, and the recorded tool output.
    assert_eq!(message_roles(&bodies[0]), vec!["system", "user"]);
    assert_eq!(
        message_roles(&bodies[1]),
        vec!["system", "user", "assistant", "tool"]
    );
    assert_eq!(bodies[1]["messages"][3]["tool_call_id"], "call-1");
    assert!(bodies[1]["messages"][3]["content"]
        .as_str()
        .unwrap()
        .contains("hello"));
    // KI-016: the assistant message the next request carries still names its
    // tool call — an assistant turn with no tool_calls followed by a tool
    // result is not a valid sequence for the API.
    let replayed_calls = &bodies[1]["messages"][2]["tool_calls"];
    assert_eq!(
        replayed_calls.as_array().map(Vec::len),
        Some(1),
        "{replayed_calls}"
    );
    assert_eq!(replayed_calls[0]["id"], "call-1");

    // The log holds exactly what the model saw plus bookkeeping.
    let records = session::replay(session.path()).unwrap();
    assert_eq!(records.len(), 8);
    assert!(matches!(&records[0], Record::Start { v: 1, model, .. } if model == "test-model"));
    assert!(matches!(&records[1], Record::Declarations { .. }));
    assert!(matches!(&records[2], Record::User { content } if content == "say hello"));
    // KI-016: the assistant record carries the call the model made, so a
    // resumed session replays a valid sequence.
    assert!(
        matches!(&records[3], Record::Assistant { tool_calls, .. } if tool_calls.len() == 1 && tool_calls[0].id == "call-1"),
        "{:?}",
        records[3]
    );
    assert!(matches!(
        first_result(&records),
        Record::ToolResult { approved: true, .. }
    ));
    assert!(matches!(
        records.last(),
        Some(Record::End { stop_reason, turns: 2, .. }) if stop_reason == "handed back"
    ));

    std::fs::remove_dir_all(&home).ok();
}

#[tokio::test]
async fn every_call_writes_one_resolution_and_the_methods_are_named() {
    // One alias (`view` -> read_file), one alias (`search` -> grep), one
    // name nothing answers. The ledger is the log's answer to "what did
    // the model ask for, and how did it get an answer".
    let home = temp_home("resolution-ledger");
    let note = home.join("note.txt");
    std::fs::write(&note, "hello\n").unwrap();
    let stream = stream(vec![
        tool_call_delta(
            0,
            "c1",
            "view",
            &format!(r#"{{"path":"{}"}}"#, note.display()),
        ),
        tool_call_delta(
            1,
            "c2",
            "search",
            &format!(r#"{{"pattern":"hello","path":"{}"}}"#, home.display()),
        ),
        tool_call_delta(2, "c3", "ghost", "{}"),
        finish_frame("tool_calls"),
        usage_frame(10, 5),
        "data: [DONE]".to_string(),
    ]);
    let mock = MockApi::start(vec![stream, assistant_stream(TOOL_REPORT)]);
    let mut session = Session::create(&home, "test-model").unwrap();
    run_on(&mock, &mut session, Box::new(ApproveAll), 5, "look around")
        .await
        .unwrap();
    let records = session::replay(session.path()).unwrap();
    let resolutions: Vec<(String, String, String)> = records
        .iter()
        .filter_map(|record| match record {
            Record::ToolResolution {
                requested,
                resolved,
                method,
                ..
            } => Some((requested.clone(), resolved.clone(), method.clone())),
            _ => None,
        })
        .collect();
    assert_eq!(resolutions.len(), 3, "{resolutions:?}");
    assert_eq!(
        resolutions[0],
        (
            "view".to_string(),
            "read_file".to_string(),
            "alias".to_string()
        )
    );
    assert_eq!(
        resolutions[1],
        (
            "search".to_string(),
            "grep".to_string(),
            "alias".to_string()
        )
    );
    assert_eq!(resolutions[2].0, "ghost");
    assert_eq!(resolutions[2].2, "refused");
    assert!(resolutions[2].1.is_empty());
    std::fs::remove_dir_all(&home).ok();
}

#[tokio::test]
async fn a_turn_past_the_cap_is_refused_by_name() {
    let home = temp_home("call-cap");
    let note = home.join("note.txt");
    std::fs::write(&note, "hello\n").unwrap();
    let mut frames = Vec::new();
    for index in 0..=fleet::agent::MAX_CALLS_PER_TURN {
        frames.push(tool_call_delta(
            index,
            &format!("c{index}"),
            "read_file",
            &format!(r#"{{"path":"{}"}}"#, note.display()),
        ));
    }
    frames.push(finish_frame("tool_calls"));
    frames.push(usage_frame(10, 5));
    frames.push("data: [DONE]".to_string());
    let mock = MockApi::start(vec![stream(frames), assistant_stream(TOOL_REPORT)]);
    let mut session = Session::create(&home, "test-model").unwrap();
    run_on(&mock, &mut session, Box::new(ApproveAll), 5, "read it")
        .await
        .unwrap();
    let records = session::replay(session.path()).unwrap();
    let refused = records
        .iter()
        .find(|record| matches!(record, Record::ToolResult { call_id, .. } if call_id == "c40"))
        .expect("the 41st result");
    match refused {
        Record::ToolResult { output, .. } => assert!(
            output.starts_with("error: this turn carried 41 calls"),
            "{output}"
        ),
        other => panic!("expected a refusal, got {other:?}"),
    }
    std::fs::remove_dir_all(&home).ok();
}

#[tokio::test]
async fn an_edit_without_a_prior_read_is_refused_and_a_read_unlocks_it() {
    let home = temp_home("edit-without-read");
    let file = home.join("code.rs");
    std::fs::write(&file, "fn main() {}\n").unwrap();
    let edit = format!(
        r#"{{"path":"{}","old_string":"fn main() {{}}","new_string":"fn main() {{ let _ = 1; }}"}}"#,
        file.display()
    );
    let read = format!(r#"{{"path":"{}"}}"#, file.display());
    let mock = MockApi::start(vec![
        tool_call_stream("c1", "edit", &edit),
        tool_call_stream("c2", "read_file", &read),
        assistant_stream(TOOL_REPORT),
    ]);
    let mut session = Session::create(&home, "test-model").unwrap();
    run_on(&mock, &mut session, Box::new(ApproveAll), 5, "edit it")
        .await
        .unwrap();
    let records = session::replay(session.path()).unwrap();
    let results: Vec<&Record> = records
        .iter()
        .filter(|record| matches!(record, Record::ToolResult { .. }))
        .collect();
    assert_eq!(results.len(), 2, "{results:?}");
    match results[0] {
        Record::ToolResult { output, .. } => assert!(
            output.starts_with("error: read") && output.contains("before editing"),
            "{output}"
        ),
        other => panic!("expected the refusal, got {other:?}"),
    }
    match results[1] {
        Record::ToolResult { output, .. } => assert!(
            output.contains("fn main() {}"),
            "the read answers with the file: {output}"
        ),
        other => panic!("expected the read, got {other:?}"),
    }
    std::fs::remove_dir_all(&home).ok();
}

/// A stop written while a command runs lands during the call: the child
/// is killed, the result says who stopped it, and the run ends.
#[tokio::test]
async fn a_stop_during_bash_ends_the_call_within_a_second() {
    let home = temp_home("stop-bash");
    let mock = MockApi::start(vec![
        tool_call_stream("c1", "bash", r#"{"command":"sleep 10"}"#),
        assistant_stream(TOOL_REPORT),
    ]);
    let mut session = Session::create(&home, "test-model").unwrap();
    let control = session.path().with_extension("control");
    let writer = tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(700)).await;
        std::fs::write(&control, r#"{"stop": true}"#).unwrap();
    });
    let started = std::time::Instant::now();
    let error = run_on(&mock, &mut session, Box::new(ApproveAll), 5, "sleep")
        .await
        .unwrap_err();
    let elapsed = started.elapsed();
    writer.await.unwrap();
    assert_eq!(error.to_string(), "stopped by the owner");
    assert!(elapsed < std::time::Duration::from_secs(5), "{elapsed:?}");
    let records = session::replay(session.path()).unwrap();
    let output = records
        .iter()
        .find_map(|record| match record {
            Record::ToolResult { output, .. } => Some(output),
            _ => None,
        })
        .expect("the bash result");
    assert!(output.contains("stopped by the owner"), "{output}");
    std::fs::remove_dir_all(&home).ok();
}

#[tokio::test]
async fn a_forced_fold_carries_the_todo_list() {
    let home = temp_home("fold-todo");
    // Four earlier turns are seeded so a fifth leaves something before
    // the verbatim tail to fold; the live turn is the todo call.
    let mut session = Session::create(&home, "test-model").unwrap();
    session
        .record(&Record::User {
            content: "start".to_string(),
        })
        .unwrap();
    for index in 0..4 {
        session
            .record(&Record::Assistant {
                content: None,
                tool_calls: vec![serde_json::from_value(tool_call(
                    &format!("s{index}"),
                    "bash",
                    r#"{"command":"true"}"#,
                ))
                .unwrap()],
                persona: None,
            })
            .unwrap();
        session
            .record(&Record::ToolResult {
                call_id: format!("s{index}"),
                name: "bash".to_string(),
                output: "exit 0\n(no output)\n".to_string(),
                truncated: false,
                approved: true,
            })
            .unwrap();
    }
    let mock = MockApi::start(vec![
        tool_call_stream(
            "c1",
            "todo",
            r#"{"items":[{"step":"read","status":"completed"},{"step":"edit","status":"in_progress"}]}"#,
        ),
        assistant_stream("The work so far."),
        assistant_stream(TOOL_REPORT),
    ]);
    run_on_compacting(&mock, &mut session, Box::new(ApproveAll), "do the work")
        .await
        .unwrap();
    let records = session::replay(session.path()).unwrap();
    let summary = records
        .iter()
        .find_map(|record| match record {
            Record::Compaction { summary, .. } => Some(summary),
            _ => None,
        })
        .expect("the fold happened");
    assert!(
        summary.contains("## Task list (carried across the fold)"),
        "{summary}"
    );
    assert!(summary.contains("[x] read"), "{summary}");
    assert!(summary.contains("[~] edit"), "{summary}");
    std::fs::remove_dir_all(&home).ok();
}

#[tokio::test]
async fn tool_search_asks_sys1_and_ranks_with_sem1_end_to_end() {
    // sys1 declares one classify tool and answers the capability head;
    // sem1 declares two tools and returns vectors that put the sys1
    // tool first.
    let sys1 = FakeMcp::start(vec![
        serde_json::json!({ "reply": initialize_reply() }),
        serde_json::json!({ "reply": tools_list_reply(serde_json::json!([
            { "name": "sys1_classify", "description": "classify text", "inputSchema": { "type": "object" } }
        ])) }),
        serde_json::json!({ "reply": call_reply(
            r#"{"verdict":"ok","answers":{"jev":{"capability":{"label":"search","confidence":0.9}}}}"#
        ) }),
    ]);
    let sem1 = FakeMcp::start(vec![
        serde_json::json!({ "reply": initialize_reply() }),
        serde_json::json!({ "reply": tools_list_reply(serde_json::json!([
            { "name": "sem1_embed", "description": "embed text", "inputSchema": { "type": "object" } },
            { "name": "sem1_search", "description": "search a store", "inputSchema": { "type": "object" } }
        ])) }),
        serde_json::json!({ "reply": call_reply(
            r#"{"vectors":[[1.0,0.0],[0.1,1.0],[0.1,1.0],[0.9,0.1]],"dims":2,"model":"fake","provider":"fake"}"#
        ) }),
    ]);
    let mock = MockApi::start(vec![
        tool_call_stream("c1", "tool_search", r#"{"query":"classify this text"}"#),
        assistant_stream(TOOL_REPORT),
    ]);
    let home = temp_home("tool-search");
    let mut session = Session::create(&home, "test-model").unwrap();
    let mut specs = std::collections::HashMap::new();
    specs.insert("sys1".to_string(), sys1.spec());
    specs.insert("sem1".to_string(), sem1.spec());
    let mut progress = Vec::new();
    run_with_lanes(
        &mock,
        &mut session,
        Box::new(ApproveAll),
        Some(specs),
        &mut progress,
        "find a tool",
    )
    .await
    .unwrap();
    let records = session::replay(session.path()).unwrap();
    let output = records
        .iter()
        .find_map(|record| match record {
            Record::ToolResult { name, output, .. } if name == "tool_search" => Some(output),
            _ => None,
        })
        .expect("the search result");
    assert!(
        output.contains("sys1: search (0.90)"),
        "{output}\n{records:?}"
    );
    assert!(
        output.contains("activated for the next request: mcp__sys1__classify"),
        "{output}"
    );
    // The resolution added no model request: the mock saw the turn that
    // asked for the search and the final report, and nothing between
    // them. Selection is sys1's class, sem1's vectors, and a rule.
    assert_eq!(
        mock.bodies().len(),
        2,
        "tool_search made a model request of its own"
    );
    std::fs::remove_dir_all(&home).ok();
}

/// The log's Declarations record is the first request's own
/// declaration list: the denominator of every usage rate cannot drift
/// from what the wire carried.
#[tokio::test]
async fn the_declarations_record_matches_the_first_request() {
    let home = temp_home("declarations-record");
    let mock = MockApi::start(vec![assistant_stream(TOOL_REPORT)]);
    let mut session = Session::create(&home, "test-model").unwrap();
    run_on(&mock, &mut session, Box::new(ApproveAll), 5, "say done")
        .await
        .unwrap();
    let records = session::replay(session.path()).unwrap();
    let declared = records
        .iter()
        .find_map(|record| match record {
            Record::Declarations { names, tier, .. } => Some((names.clone(), tier.clone())),
            _ => None,
        })
        .expect("the record exists");
    let wire = declared_tools(&mock.bodies()[0]);
    assert_eq!(declared.0, wire, "log and wire must agree");
    assert_eq!(declared.1, "core");
    std::fs::remove_dir_all(&home).ok();
}

/// A disabled tool is not declared, is excluded from tool_search, and
/// a call to it answers "disabled" with the enable command. Enabling
/// puts it back on the next run.
#[tokio::test]
async fn a_disabled_tool_is_undeclared_and_its_call_names_the_rule() {
    let home = temp_home("disabled-tool");
    // Disable deck through the same writer the CLI rides. The run
    // carries this home, the way the GUI and TUI runs do — a run with
    // no home reads the ambient fleet home instead.
    fleet::settings::set_tool_disabled(&home, "deck", true).unwrap();
    let mock = MockApi::start(vec![
        tool_call_stream("c1", "deck", r#"{"deck":{}}"#),
        assistant_stream(TOOL_REPORT),
    ]);
    let mut session = Session::create(&home, "test-model").unwrap();
    let client = Client::new(mock.url(), "test-key");
    let options = AgentOptions {
        goal: None,
        model: "test-model".to_string(),
        identity: None,
        max_turns: 5,
        ui_tx: None,
        lane_specs: None,
        calib: None,
        skills: Vec::new(),
        role_overlay: None,
        memory: None,
        scope: None,
        improve_home: Some(home.clone()),
        route_home: None,
        gui_prompt: None,
        compact_now: false,
        checkpoints: None,
        file_access: FileAccess::Unrestricted,
        plan_mode: false,
    };
    let mut progress = Vec::new();
    let log_path = session.path().to_path_buf();
    {
        let mut agent = Agent::new(
            &client,
            options,
            &mut session,
            Box::new(ApproveAll),
            &mut progress,
        )
        .unwrap();
        agent.run("make a deck").await.unwrap();
    }
    let records = session::replay(&log_path).unwrap();
    // Not declared.
    let declared = records
        .iter()
        .find_map(|record| match record {
            Record::Declarations { names, .. } => Some(names.clone()),
            _ => None,
        })
        .expect("the record");
    assert!(!declared.contains(&"deck".to_string()));
    // The call answers with the rule and the reverse, not "unknown".
    let output = records
        .iter()
        .find_map(|record| match record {
            Record::ToolResult { output, .. } => Some(output),
            _ => None,
        })
        .expect("the result");
    assert!(output.starts_with("disabled:"), "{output}");
    assert!(output.contains("fleet tools enable deck"), "{output}");
    // Enable restores it: the next run declares it again.
    fleet::settings::set_tool_disabled(&home, "deck", false).unwrap();
    let mock2 = MockApi::start(vec![assistant_stream(TOOL_REPORT)]);
    let mut session2 = Session::create(&home, "test-model").unwrap();
    run_on(&mock2, &mut session2, Box::new(ApproveAll), 5, "again")
        .await
        .unwrap();
    let declared2 = session::replay(session2.path())
        .unwrap()
        .iter()
        .find_map(|record| match record {
            Record::Declarations { names, .. } => Some(names.clone()),
            _ => None,
        })
        .expect("the record");
    assert!(declared2.contains(&"deck".to_string()));
    std::fs::remove_dir_all(&home).ok();
}

/// The sweep's dry run changes nothing and names its skip reasons; the
/// applied run disables a gate candidate, records the calibration row,
/// and never touches the protected set.
#[tokio::test]
async fn the_lifecycle_sweep_disables_only_what_the_gate_names() {
    let home = temp_home("lifecycle-sweep");
    // Twelve aged sessions declaring deck and never calling it: a
    // candidate. bash is declared too, and protected.
    for index in 0..12 {
        let mut session = Session::create(&home, "test-model").unwrap();
        session
            .record(&Record::Declarations {
                names: vec!["bash".to_string(), "deck".to_string()],
                tier: "core".to_string(),
            })
            .unwrap();
        age_session(&home, session, &format!("s{index}"));
    }
    // Dry run: nothing is written.
    let output = sweep_output(&home, false);
    assert!(output.contains("deck"), "{output}");
    assert!(fleet::settings::tools_disabled(&home).is_empty());
    // Apply: deck is disabled, bash is skipped as protected.
    let output = sweep_output(&home, true);
    assert!(output.contains("disable  deck"), "{output}");
    assert_eq!(
        fleet::settings::tools_disabled(&home),
        vec!["deck".to_string()]
    );
    assert!(output.contains("protected"), "{output}");
    // The calibration row exists: the sweep's own stream wrote it
    // beside the config write, so the audit trail is on disk.
    let user_home = std::env::var("HOME").ok();
    let root = fleet::calib::resolve_root(&home, user_home.as_deref());
    fn read_jsonl(dir: &Path, out: &mut String) {
        for entry in std::fs::read_dir(dir).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                read_jsonl(&path, out);
            } else if path.extension().and_then(|e| e.to_str()) == Some("jsonl") {
                out.push_str(&std::fs::read_to_string(&path).unwrap_or_default());
            }
        }
    }
    let mut rows = String::new();
    read_jsonl(&root, &mut rows);
    assert!(
        rows.contains("\"op\":\"lifecycle\"") && rows.contains("\"name\":\"deck\""),
        "{rows}"
    );
    std::fs::remove_dir_all(&home).ok();
}

fn age_session(home: &Path, session: Session, stamp: &str) {
    let dir = fleet::home::sessions_dir(home);
    std::fs::create_dir_all(&dir).unwrap();
    let target = dir.join(format!("20261001T0000{stamp}Z-test.jsonl"));
    let text = std::fs::read_to_string(session.path()).unwrap();
    let aged = text.replacen(r#""created":""#, r#""created":"2026-10-01"#, 1);
    std::fs::write(&target, aged).unwrap();
    std::fs::remove_file(session.path()).unwrap();
}

fn sweep_output(home: &Path, apply: bool) -> String {
    let thresholds = fleet::settings::lifecycle_thresholds(home);
    let scan = fleet::tool_advisor::scan(home, thresholds.window);
    let metrics = fleet::tool_advisor::lifecycle_metrics(&scan);
    let skill_metrics = fleet::tool_advisor::scan_skills(home, thresholds.window)
        .into_iter()
        .map(|skill| fleet::lifecycle::SkillMetrics {
            name: skill.name,
            declared_runs: skill.declared_runs,
            uses: skill.uses,
            first_seen: skill.first_seen,
        })
        .collect::<Vec<_>>();
    let stream = fleet::calib::Stream::open(fleet::calib::resolve_root(
        home,
        std::env::var("HOME").ok().as_deref(),
    ));
    let plan = fleet::lifecycle::sweep(
        home,
        &thresholds,
        &metrics,
        &skill_metrics,
        apply,
        Some(&stream),
    );
    let mut text = String::new();
    for (kind, result) in &plan {
        for candidate in &result.applied {
            text.push_str(&format!(
                "disable  {} \u{2014} {}\n",
                candidate.name, candidate.reason
            ));
        }
        for (name, reason) in &result.skipped {
            text.push_str(&format!("skip {kind} {name} \u{2014} {reason}\n"));
        }
    }
    text
}

/// A folder-scoped run over `tools`, so the write tools take their real
/// path resolution and the patch lands in a tree the test owns.
async fn run_in_folder(
    mock: &MockApi,
    session: &mut Session,
    folder: &std::path::Path,
    max_turns: usize,
) -> Result<RunReport, AgentError> {
    let client = Client::new(mock.url(), "test-key");
    let options = AgentOptions {
        goal: None,
        model: "test-model".to_string(),
        identity: None,
        max_turns,
        ui_tx: None,
        lane_specs: None,
        calib: None,
        skills: Vec::new(),
        role_overlay: None,
        memory: None,
        scope: None,
        improve_home: None,
        route_home: None,
        gui_prompt: None,
        compact_now: false,
        checkpoints: None,
        file_access: FileAccess::Folder(folder.to_path_buf()),
        plan_mode: false,
    };
    let mut progress = Vec::new();
    let mut agent = Agent::new(
        &client,
        options,
        session,
        Box::new(ApproveAll),
        &mut progress,
    )
    .unwrap();
    agent.run("patch the tree").await
}

/// `apply_patch` writes every file a patch names, and refuses a file whose
/// hunk does not match without touching that file.
#[tokio::test]
async fn apply_patch_writes_every_file_and_refuses_a_stale_hunk_without_writing() {
    let folder = std::env::temp_dir().join(format!("fleet-patch-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(folder.join("src")).unwrap();
    std::fs::write(folder.join("src/one.rs"), "fn one() {\n    old();\n}\n").unwrap();
    std::fs::write(folder.join("src/two.rs"), "fn two() {\n    keep();\n}\n").unwrap();

    let good = [
        "*** Begin Patch",
        "*** Update File: src/one.rs",
        "@@",
        " fn one() {",
        "-    old();",
        "+    new();",
        " }",
        "*** Add File: src/three.rs",
        "+pub fn three() {}",
        "*** End Patch",
    ]
    .join("\n");
    let arguments = serde_json::json!({ "patch": good }).to_string();
    let mock = MockApi::start(vec![
        tool_call_stream("c1", "apply_patch", &arguments),
        assistant_stream(TOOL_REPORT),
    ]);
    let home = temp_home("patch-good");
    let mut session = Session::create(&home, "test-model").unwrap();
    run_in_folder(&mock, &mut session, &folder, 4)
        .await
        .unwrap();
    let (output, _) = tool_result(&session::replay(session.path()).unwrap(), "apply_patch");
    assert!(output.contains("updated src/one.rs"), "{output}");
    assert!(output.contains("added src/three.rs"), "{output}");
    assert_eq!(
        std::fs::read_to_string(folder.join("src/one.rs")).unwrap(),
        "fn one() {\n    new();\n}\n"
    );
    assert_eq!(
        std::fs::read_to_string(folder.join("src/three.rs")).unwrap(),
        "pub fn three() {}\n"
    );

    // A stale hunk: refused by name, and the file it named is untouched.
    let stale = [
        "*** Begin Patch",
        "*** Update File: src/two.rs",
        "@@",
        " fn two() {",
        "-    gone();",
        "+    changed();",
        " }",
        "*** End Patch",
    ]
    .join("\n");
    let arguments = serde_json::json!({ "patch": stale }).to_string();
    let mock = MockApi::start(vec![
        tool_call_stream("c1", "apply_patch", &arguments),
        assistant_stream(TOOL_REPORT),
    ]);
    let home2 = temp_home("patch-stale");
    let mut session2 = Session::create(&home2, "test-model").unwrap();
    run_in_folder(&mock, &mut session2, &folder, 4)
        .await
        .unwrap();
    let (output, _) = tool_result(&session2_replay(&session2), "apply_patch");
    assert!(output.contains("did not match"), "{output}");
    assert!(output.contains("nearest line"), "{output}");
    assert_eq!(
        std::fs::read_to_string(folder.join("src/two.rs")).unwrap(),
        "fn two() {\n    keep();\n}\n"
    );
    std::fs::remove_dir_all(&home).ok();
    std::fs::remove_dir_all(&home2).ok();
    std::fs::remove_dir_all(&folder).ok();
}

fn session2_replay(session: &Session) -> Vec<Record> {
    session::replay(session.path()).unwrap()
}

/// `repo_map` is deterministic through the tool path, stays inside its
/// budget, and says it is a heuristic.
#[tokio::test]
async fn repo_map_is_deterministic_budgeted_and_names_its_heuristic() {
    let folder = std::env::temp_dir().join(format!("fleet-repomap-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(folder.join("src")).unwrap();
    for index in 0..12 {
        std::fs::write(
            folder.join(format!("src/file{index}.rs")),
            format!("pub fn shared_name() {{}}\npub fn only_{index}() {{}}\n"),
        )
        .unwrap();
    }
    let arguments = serde_json::json!({ "budget_tokens": 900 }).to_string();
    let mock = MockApi::start(vec![
        tool_call_stream("c1", "repo_map", &arguments),
        tool_call_stream("c2", "repo_map", &arguments),
        assistant_stream(TOOL_REPORT),
    ]);
    let home = temp_home("repomap");
    let mut session = Session::create(&home, "test-model").unwrap();
    run_in_folder(&mock, &mut session, &folder, 4)
        .await
        .unwrap();
    let outputs: Vec<String> = session::replay(session.path())
        .unwrap()
        .iter()
        .filter_map(|record| match record {
            Record::ToolResult { name, output, .. } if name == "repo_map" => Some(output.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(outputs.len(), 2, "{outputs:?}");
    assert_eq!(outputs[0], outputs[1], "two reads, one map");
    assert!(
        outputs[0].starts_with("Repo map (heuristic"),
        "{}",
        outputs[0]
    );
    assert!(outputs[0].contains("shared_name"), "{}", outputs[0]);
    assert!(
        outputs[0].len() <= 900 * 4 + 300,
        "{} bytes",
        outputs[0].len()
    );
    std::fs::remove_dir_all(&home).ok();
    std::fs::remove_dir_all(&folder).ok();
}

/// A DOCX with one paragraph, written the way the format is written: a
/// stored (uncompressed) zip holding `word/document.xml`.
///
/// Hand-built rather than shelling out to `zip`: the fixture must not
/// depend on a binary the gate's machine may not have, and a stored
/// entry is a header, the bytes, and a directory. The CRC is the only
/// arithmetic.
fn write_docx(path: &std::path::Path, text: &str) {
    let body = format!(
        r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>{text}</w:t></w:r></w:p></w:body></w:document>"#
    );
    let name = b"word/document.xml";
    let data = body.as_bytes();
    let crc = crc32(data);
    let mut out: Vec<u8> = Vec::new();
    let u16le = |out: &mut Vec<u8>, value: u16| out.extend_from_slice(&value.to_le_bytes());
    let u32le = |out: &mut Vec<u8>, value: u32| out.extend_from_slice(&value.to_le_bytes());
    // Local file header.
    u32le(&mut out, 0x0403_4b50);
    u16le(&mut out, 20);
    u16le(&mut out, 0);
    u16le(&mut out, 0);
    u16le(&mut out, 0);
    u16le(&mut out, 0);
    u32le(&mut out, crc);
    u32le(&mut out, data.len() as u32);
    u32le(&mut out, data.len() as u32);
    u16le(&mut out, name.len() as u16);
    u16le(&mut out, 0);
    out.extend_from_slice(name);
    out.extend_from_slice(data);
    // Central directory.
    let directory_at = out.len() as u32;
    u32le(&mut out, 0x0201_4b50);
    u16le(&mut out, 20);
    u16le(&mut out, 20);
    u16le(&mut out, 0);
    u16le(&mut out, 0);
    u16le(&mut out, 0);
    u16le(&mut out, 0);
    u32le(&mut out, crc);
    u32le(&mut out, data.len() as u32);
    u32le(&mut out, data.len() as u32);
    u16le(&mut out, name.len() as u16);
    u16le(&mut out, 0);
    u16le(&mut out, 0);
    u16le(&mut out, 0);
    u16le(&mut out, 0);
    u32le(&mut out, 0);
    u32le(&mut out, 0);
    out.extend_from_slice(name);
    let directory_size = out.len() as u32 - directory_at;
    // End of central directory.
    u32le(&mut out, 0x0605_4b50);
    u16le(&mut out, 0);
    u16le(&mut out, 0);
    u16le(&mut out, 1);
    u16le(&mut out, 1);
    u32le(&mut out, directory_size);
    u32le(&mut out, directory_at);
    u16le(&mut out, 0);
    std::fs::write(path, out).unwrap();
}

/// The zip's own CRC-32 (polynomial 0xEDB88320).
fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for byte in data {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

/// C19: the search refinements behave as declared, and a provider that
/// cannot honor one says so rather than dropping it.
#[tokio::test]
async fn grep_context_and_type_and_multiline_are_honored_or_named() {
    let folder = std::env::temp_dir().join(format!("fleet-grep-opts-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(
        folder.join("one.rs"),
        "fn outer() {\n    let needle = 1;\n}\n",
    )
    .unwrap();
    std::fs::write(folder.join("two.py"), "needle = 2\n").unwrap();

    let mock = MockApi::start(vec![
        // context: the match plus the lines around it.
        tool_call_stream(
            "c1",
            "grep",
            r#"{"pattern":"needle","output_mode":"content","context":1}"#,
        ),
        // type: only the Python file.
        tool_call_stream(
            "c2",
            "grep",
            r#"{"pattern":"needle","output_mode":"files","type":"py"}"#,
        ),
        // multiline: a pattern that spans two lines.
        tool_call_stream(
            "c3",
            "grep",
            r#"{"pattern":"fn outer.*\n    let","output_mode":"files","multiline":true}"#,
        ),
        assistant_stream(TOOL_REPORT),
    ]);
    let home = temp_home("grep-opts");
    let mut session = Session::create(&home, "test-model").unwrap();
    run_in_folder(&mock, &mut session, &folder, 6)
        .await
        .unwrap();
    let outputs: Vec<String> = session::replay(session.path())
        .unwrap()
        .iter()
        .filter_map(|record| match record {
            Record::ToolResult { name, output, .. } if name == "grep" => Some(output.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(outputs.len(), 3, "{outputs:?}");
    // context: the surrounding lines arrive with the match.
    assert!(outputs[0].contains("fn outer() {"), "{}", outputs[0]);
    assert!(outputs[0].contains("needle"), "{}", outputs[0]);
    // type: the Python file, and not the Rust one.
    assert!(outputs[1].contains("two.py"), "{}", outputs[1]);
    assert!(!outputs[1].contains("one.rs"), "{}", outputs[1]);
    // multiline: the two-line pattern matched.
    assert!(outputs[2].contains("one.rs"), "{}", outputs[2]);
    std::fs::remove_dir_all(&home).ok();
    std::fs::remove_dir_all(&folder).ok();
}

/// C20: `file_change` moves, copies, deletes and creates; a path under a
/// reserved segment is refused by name; `move` and `delete` are in the
/// destructive class and ask in every mode.
#[tokio::test]
async fn file_change_does_its_four_ops_and_refuses_a_reserved_path() {
    let folder = std::env::temp_dir().join(format!("fleet-change-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(folder.join(".git")).unwrap();
    std::fs::write(folder.join("a.txt"), "alpha\n").unwrap();
    std::fs::write(folder.join(".git/config"), "[core]\n").unwrap();

    let mock = MockApi::start(vec![
        tool_call_stream("c1", "file_change", r#"{"op":"make_dir","path":"sub"}"#),
        tool_call_stream(
            "c2",
            "file_change",
            r#"{"op":"copy","path":"a.txt","to":"sub/b.txt"}"#,
        ),
        tool_call_stream(
            "c3",
            "file_change",
            r#"{"op":"move","path":"a.txt","to":"sub/a.txt"}"#,
        ),
        tool_call_stream("c4", "file_change", r#"{"op":"delete","path":"sub/b.txt"}"#),
        tool_call_stream(
            "c5",
            "file_change",
            r#"{"op":"delete","path":".git/config"}"#,
        ),
        assistant_stream(TOOL_REPORT),
    ]);
    let home = temp_home("file-change");
    let mut session = Session::create(&home, "test-model").unwrap();
    run_in_folder(&mock, &mut session, &folder, 8)
        .await
        .unwrap();
    let outputs: Vec<String> = session::replay(session.path())
        .unwrap()
        .iter()
        .filter_map(|record| match record {
            Record::ToolResult { name, output, .. } if name == "file_change" => {
                Some(output.clone())
            }
            _ => None,
        })
        .collect();
    assert_eq!(outputs.len(), 5, "{outputs:?}");
    assert!(outputs[0].contains("created sub"), "{}", outputs[0]);
    assert!(
        outputs[1].contains("copied a.txt -> sub/b.txt"),
        "{outputs:?}"
    );
    assert!(
        outputs[2].contains("moved a.txt -> sub/a.txt"),
        "{}",
        outputs[2]
    );
    assert!(outputs[3].contains("deleted sub/b.txt"), "{}", outputs[3]);
    assert!(outputs[4].contains("inside `.git`"), "{}", outputs[4]);
    // The tree is what the ops said it would be.
    assert!(folder.join("sub/a.txt").is_file());
    assert!(!folder.join("a.txt").exists());
    assert!(!folder.join("sub/b.txt").exists());
    assert!(
        folder.join(".git/config").is_file(),
        "the refusal left it alone"
    );
    std::fs::remove_dir_all(&home).ok();
    std::fs::remove_dir_all(&folder).ok();
}

/// C20's classification half: the two subtractions are destructive and
/// the two additions are not, whatever the mode.
#[test]
fn file_change_classes_move_and_delete_as_destructive() {
    use fleet::approvals::Gate;
    let call = |arguments: &str| fleet::api::ToolCall {
        id: "c1".to_string(),
        kind: "function".to_string(),
        function: fleet::api::FunctionCall {
            name: "file_change".to_string(),
            arguments: arguments.to_string(),
        },
    };
    assert!(Gate::is_destructive(&call(
        r#"{"op":"move","path":"a","to":"b"}"#
    )));
    assert!(Gate::is_destructive(&call(r#"{"op":"delete","path":"a"}"#)));
    assert!(!Gate::is_destructive(&call(
        r#"{"op":"copy","path":"a","to":"b"}"#
    )));
    assert!(!Gate::is_destructive(&call(
        r#"{"op":"make_dir","path":"a"}"#
    )));
}

/// C17: `recall` reads the harness's own holdings, ranked, and makes no
/// model request at all.
#[tokio::test]
async fn recall_returns_ranked_facts_and_skills_without_a_model_call() {
    let root = memory_corpus(
        "recall",
        &[(
            "deploy",
            fact_file(
                "deploy",
                "the railway deploy is failing per branch",
                "inferred",
                "project",
                "deploy",
            ),
        )],
    );
    let memory = std::sync::Arc::new(fleet::memory::Memory::load_with(
        &root,
        Box::new(fleet::memory::KeywordOnly),
    ));
    let mock = MockApi::start(vec![
        tool_call_stream(
            "c1",
            "recall",
            r#"{"instruction":"the railway deploy is failing per branch"}"#,
        ),
        assistant_stream(TOOL_REPORT),
    ]);
    let home = temp_home("recall-tool");
    let mut session = Session::create(&home, "test-model").unwrap();
    run_with_memory(
        &mock,
        &mut session,
        Box::new(ApproveAll),
        Some(memory),
        None,
        "what do I know",
    )
    .await
    .unwrap();
    let (output, _) = tool_result(&session::replay(session.path()).unwrap(), "recall");
    assert!(output.contains("facts ("), "{output}");
    assert!(output.contains("deploy"), "{output}");
    // The numbers and the sources are on the line, not implied.
    assert!(output.contains("[0."), "a score on each fact: {output}");
    assert!(output.contains("inferred"), "the basis: {output}");
    assert!(output.contains("deploy"), "the kind: {output}");
    // One request per turn and no more: the tool itself called no model.
    assert_eq!(mock.bodies().len(), 2, "the run's own two requests");
    std::fs::remove_dir_all(&home).ok();
    std::fs::remove_dir_all(&root).ok();
}

/// C17: a query with no match says so, rather than returning an empty
/// string the model reads as a failure.
#[tokio::test]
async fn recall_with_nothing_matching_says_so_by_name() {
    let mock = MockApi::start(vec![
        tool_call_stream(
            "c1",
            "recall",
            r#"{"instruction":"who owns the billing service"}"#,
        ),
        assistant_stream(TOOL_REPORT),
    ]);
    let home = temp_home("recall-empty");
    let mut session = Session::create(&home, "test-model").unwrap();
    run_on(&mock, &mut session, Box::new(ApproveAll), 4, "ask")
        .await
        .unwrap();
    let (output, _) = tool_result(&session::replay(session.path()).unwrap(), "recall");
    assert!(
        output.contains("nothing in this harness's memory"),
        "{output}"
    );
    // The channel it did not search is named rather than left out.
    assert!(output.contains("session index is not warmed"), "{output}");
    std::fs::remove_dir_all(&home).ok();
}

/// C18: resources and prompts are spoken over the wire, with cursors
/// followed; a server without the capability is named.
#[tokio::test]
async fn mcp_resources_and_prompts_are_listed_and_read() {
    let fake = FakeMcp::start(vec![
        serde_json::json!({ "reply": initialize_reply() }),
        serde_json::json!({ "reply": tools_list_reply(serde_json::json!([])) }),
        // Page one of resources, with a cursor; page two ends it.
        serde_json::json!({ "reply": {
            "jsonrpc": "2.0", "id": "$echo",
            "result": {
                "resources": [{"name": "notes", "uri": "file:///notes.md", "mimeType": "text/markdown"}],
                "nextCursor": "page-2"
            }
        } }),
        serde_json::json!({ "reply": {
            "jsonrpc": "2.0", "id": "$echo",
            "result": {
                "resources": [{"name": "guide", "uri": "file:///guide.md", "mimeType": "text/markdown"}]
            }
        } }),
        serde_json::json!({ "reply": {
            "jsonrpc": "2.0", "id": "$echo",
            "result": {"contents": [{"uri": "file:///notes.md", "mimeType": "text/markdown", "text": "# Notes\nthe notes body"}]}
        } }),
        serde_json::json!({ "reply": {
            "jsonrpc": "2.0", "id": "$echo",
            "error": {"code": -32601, "message": "Method not found"}
        } }),
    ]);
    let mock = MockApi::start(vec![
        tool_call_stream("c1", "mcp_resource", r#"{"server":"sem1","action":"list"}"#),
        tool_call_stream(
            "c2",
            "mcp_resource",
            r#"{"server":"sem1","action":"read","uri":"file:///notes.md"}"#,
        ),
        tool_call_stream("c3", "mcp_prompt", r#"{"server":"sem1","action":"list"}"#),
        assistant_stream(TOOL_REPORT),
    ]);
    let home = temp_home("mcp-meta");
    let mut session = Session::create(&home, "test-model").unwrap();
    let client = Client::new(mock.url(), "test-key");
    let options = AgentOptions {
        goal: None,
        model: "test-model".to_string(),
        identity: None,
        max_turns: 6,
        ui_tx: None,
        lane_specs: Some(lane_specs(&fake)),
        calib: None,
        skills: Vec::new(),
        role_overlay: None,
        memory: None,
        scope: None,
        improve_home: None,
        route_home: None,
        gui_prompt: None,
        compact_now: false,
        checkpoints: None,
        file_access: FileAccess::Unrestricted,
        plan_mode: false,
    };
    let mut progress = Vec::new();
    {
        let mut agent = Agent::new(
            &client,
            options,
            &mut session,
            Box::new(ApproveAll),
            &mut progress,
        )
        .unwrap();
        agent.run("read its resources").await.unwrap();
    }

    let records = session::replay(session.path()).unwrap();
    let results: Vec<(String, String)> = records
        .iter()
        .filter_map(|record| match record {
            Record::ToolResult { name, output, .. } => Some((name.clone(), output.clone())),
            _ => None,
        })
        .collect();
    let listed = &results[0].1;
    assert!(listed.contains("- notes"), "{listed}");
    // The cursor was followed: page two's entry is there too.
    assert!(listed.contains("- guide"), "{listed}");
    let read = &results[1].1;
    assert!(read.contains("the notes body"), "{read}");
    assert!(read.contains("(text/markdown)"), "{read}");
    // A capability the server does not implement is named, not echoed.
    let prompts = &results[2].1;
    assert!(prompts.contains("does not implement prompts"), "{prompts}");
    // Asking for its resources joined the lane, with its tools declared.
    assert!(
        records
            .iter()
            .any(|record| matches!(record, Record::LaneTools { .. })),
        "{records:?}"
    );
    std::fs::remove_dir_all(&home).ok();
}

/// C14: a document is read locally and the parser is named; a format the
/// local parsers do not handle is refused by name, never shipped.
#[tokio::test]
async fn read_document_names_its_parser_and_refuses_what_it_cannot_read() {
    let folder = std::env::temp_dir().join(format!("fleet-doc-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(folder.join("rows.csv"), "name,qty\nwidget,3\n").unwrap();
    // A real DOCX: a zip with the one part the extractor reads. Built
    // here rather than checked in, so the fixture is the format's own
    // shape and not a file that could drift.
    write_docx(&folder.join("brief.docx"), "the quarterly brief");
    std::fs::write(folder.join("photo.png"), "not really a png").unwrap();
    std::fs::write(folder.join("archive.zip"), "not really a zip").unwrap();

    let mock = MockApi::start(vec![
        tool_call_stream("c1", "read_document", r#"{"path":"rows.csv"}"#),
        tool_call_stream("c2", "read_document", r#"{"path":"photo.png"}"#),
        tool_call_stream("c3", "read_document", r#"{"path":"brief.docx"}"#),
        tool_call_stream("c4", "read_document", r#"{"path":"archive.zip"}"#),
        assistant_stream(TOOL_REPORT),
    ]);
    let home = temp_home("read-document");
    let mut session = Session::create(&home, "test-model").unwrap();
    run_in_folder(&mock, &mut session, &folder, 4)
        .await
        .unwrap();
    let outputs: Vec<String> = session::replay(session.path())
        .unwrap()
        .iter()
        .filter_map(|record| match record {
            Record::ToolResult { name, output, .. } if name == "read_document" => {
                Some(output.clone())
            }
            _ => None,
        })
        .collect();
    assert_eq!(outputs.len(), 4, "{outputs:?}");
    assert!(outputs[0].starts_with("read by local:"), "{}", outputs[0]);
    assert!(outputs[0].contains("widget"), "{}", outputs[0]);
    // No egress, and the refusal points at the owner's own surface.
    assert!(
        outputs[1].contains("never sends a document off this machine"),
        "{}",
        outputs[1]
    );
    // A real document format reads as markdown, with its own parser.
    assert!(
        outputs[2].starts_with("read by local:docx"),
        "{}",
        outputs[2]
    );
    assert!(outputs[2].contains("the quarterly brief"), "{}", outputs[2]);
    // An extension no parser claims is refused by name, too.
    assert!(
        outputs[3].contains("no parser handles this extension"),
        "{}",
        outputs[3]
    );
    std::fs::remove_dir_all(&home).ok();
    std::fs::remove_dir_all(&folder).ok();
}

/// C16: handing a file over emits one artifact event and returns the
/// path and size; a path outside the run's folder is refused.
#[tokio::test]
async fn send_file_emits_one_artifact_and_refuses_a_path_outside_the_folder() {
    let folder = std::env::temp_dir().join(format!("fleet-send-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(folder.join("report.md"), "# Report\n").unwrap();

    let mock = MockApi::start(vec![
        tool_call_stream(
            "c1",
            "send_file",
            r#"{"path":"report.md","note":"the quarter's numbers"}"#,
        ),
        tool_call_stream("c2", "send_file", r#"{"path":"../../etc/hosts"}"#),
        assistant_stream(TOOL_REPORT),
    ]);
    let home = temp_home("send-file");
    let mut session = Session::create(&home, "test-model").unwrap();
    let (ui_tx, mut ui_rx) = tokio::sync::mpsc::unbounded_channel::<UiEvent>();
    {
        let client = Client::new(mock.url(), "test-key");
        let options = AgentOptions {
            goal: None,
            model: "test-model".to_string(),
            identity: None,
            max_turns: 4,
            ui_tx: Some(ui_tx.clone()),
            lane_specs: None,
            calib: None,
            skills: Vec::new(),
            role_overlay: None,
            memory: None,
            scope: None,
            improve_home: None,
            route_home: None,
            gui_prompt: None,
            compact_now: false,
            checkpoints: None,
            file_access: FileAccess::Folder(folder.clone()),
            plan_mode: false,
        };
        let mut progress = Vec::new();
        let mut agent = Agent::new(
            &client,
            options,
            &mut session,
            Box::new(ApproveAll),
            &mut progress,
        )
        .unwrap();
        agent.run("hand it over").await.unwrap();
    }
    drop(ui_tx);
    let mut artifacts = Vec::new();
    while let Some(event) = ui_rx.recv().await {
        if let UiEvent::Artifact { path, size, note } = event {
            artifacts.push((path, size, note));
        }
    }
    assert_eq!(artifacts.len(), 1, "{artifacts:?}");
    assert_eq!(artifacts[0].0, "report.md");
    assert_eq!(artifacts[0].1, 9);
    assert_eq!(artifacts[0].2.as_deref(), Some("the quarter's numbers"));

    let outputs: Vec<String> = session::replay(session.path())
        .unwrap()
        .iter()
        .filter_map(|record| match record {
            Record::ToolResult { name, output, .. } if name == "send_file" => Some(output.clone()),
            _ => None,
        })
        .collect();
    assert!(
        outputs[0].contains("handed `report.md` to the owner (9 bytes)"),
        "{}",
        outputs[0]
    );
    assert!(outputs[1].contains("error:"), "{}", outputs[1]);
    std::fs::remove_dir_all(&home).ok();
    std::fs::remove_dir_all(&folder).ok();
}

/// C15: a URL is read through the guard. A private address is refused at
/// the first hop, and the refusal is the guard's sentence, not a fetch
/// failure.
#[tokio::test]
async fn web_with_a_url_reads_through_the_guard_and_refuses_a_private_hop() {
    let mock = MockApi::start(vec![
        tool_call_stream("c1", "web", r#"{"url":"http://127.0.0.1:9/"}"#),
        assistant_stream(TOOL_REPORT),
    ]);
    let home = temp_home("web-url");
    let mut session = Session::create(&home, "test-model").unwrap();
    run_on(&mock, &mut session, Box::new(ApproveAll), 4, "read it")
        .await
        .unwrap();
    let (output, _) = tool_result(&session::replay(session.path()).unwrap(), "web");
    assert!(output.starts_with("error:"), "{output}");
    assert!(
        output.contains("loopback") || output.contains("private"),
        "the guard's own sentence, not a transport error: {output}"
    );
    std::fs::remove_dir_all(&home).ok();
}

/// The `process` tool controls what `bash run_in_background` started:
/// incremental output, a bounded wait, and a kill. The registry dies
/// with the run, so an unknown id is a named error.
#[tokio::test]
async fn process_output_wait_and_kill_control_a_background_child() {
    let home = temp_home("process-tool");
    let mock = MockApi::start(vec![
        tool_call_stream(
            "c1",
            "bash",
            r#"{"command":"printf line-one; printf line-two >> /dev/stdout","run_in_background":true}"#,
        ),
        tool_call_stream("c2", "process", r#"{"id":"p0","action":"output"}"#),
        tool_call_stream(
            "c3",
            "process",
            r#"{"id":"p0","action":"wait","timeout_ms":5000}"#,
        ),
        tool_call_stream("c4", "process", r#"{"id":"p0","action":"kill"}"#),
        assistant_stream(TOOL_REPORT),
    ]);
    let mut session = Session::create(&home, "test-model").unwrap();
    run_on(
        &mock,
        &mut session,
        Box::new(ApproveAll),
        5,
        "start and watch",
    )
    .await
    .unwrap();
    let records = session::replay(session.path()).unwrap();
    let outputs: Vec<&String> = records
        .iter()
        .filter_map(|record| match record {
            Record::ToolResult { name, output, .. } if name == "process" => Some(output),
            _ => None,
        })
        .collect();
    assert_eq!(outputs.len(), 3, "{records:?}");
    // The id was named in the bash result and the registry answered.
    assert!(
        !outputs[0].contains("no background process"),
        "{}",
        outputs[0]
    );
    assert!(
        outputs[1].contains("finished") || outputs[1].contains("still running"),
        "{}",
        outputs[1]
    );
    // A second run knows nothing of this registry: the ids are per run.
    let mock2 = MockApi::start(vec![
        tool_call_stream("c1", "process", r#"{"id":"p0","action":"output"}"#),
        assistant_stream(TOOL_REPORT),
    ]);
    let mut session2 = Session::create(&home, "test-model").unwrap();
    run_on(&mock2, &mut session2, Box::new(ApproveAll), 5, "again")
        .await
        .unwrap();
    let output2 = session::replay(session2.path())
        .unwrap()
        .iter()
        .find_map(|record| match record {
            Record::ToolResult { output, .. } => Some(output.clone()),
            _ => None,
        })
        .unwrap();
    assert!(output2.contains("no background process `p0`"), "{output2}");
    std::fs::remove_dir_all(&home).ok();
}

/// Validation: a call without an id, an unknown action, and a wild
/// timeout are refused by name without touching the registry.
#[tokio::test]
async fn process_calls_are_validated_before_the_registry() {
    let home = temp_home("process-validate");
    let mock = MockApi::start(vec![
        tool_call_stream("c1", "process", r#"{"action":"output"}"#),
        tool_call_stream("c2", "process", r#"{"id":"p0","action":"restart"}"#),
        assistant_stream(TOOL_REPORT),
    ]);
    let mut session = Session::create(&home, "test-model").unwrap();
    run_on(&mock, &mut session, Box::new(ApproveAll), 5, "try it")
        .await
        .unwrap();
    let outputs: Vec<String> = session::replay(session.path())
        .unwrap()
        .iter()
        .filter_map(|record| match record {
            Record::ToolResult { output, .. } => Some(output.clone()),
            _ => None,
        })
        .collect();
    assert!(
        outputs[0].contains("field `id` is required"),
        "{}",
        outputs[0]
    );
    assert!(
        outputs[1].contains("field `action` must be one of"),
        "{}",
        outputs[1]
    );
    std::fs::remove_dir_all(&home).ok();
}

/// The capability tools are declared on every ordinary run, translate
/// into their lane's call, and a bad shape never spawns the lane.
#[tokio::test]
async fn capability_tools_declare_translate_and_validate() {
    let home = temp_home("capabilities");
    // Declared from the first request.
    let mock = MockApi::start(vec![assistant_stream(TOOL_REPORT)]);
    let mut session = Session::create(&home, "test-model").unwrap();
    run_on(&mock, &mut session, Box::new(ApproveAll), 5, "hello")
        .await
        .unwrap();
    let declared = declared_tools(&mock.bodies()[0]);
    for name in ["measure", "imagine", "browse"] {
        assert!(declared.contains(&name.to_string()), "{declared:?}");
    }
    // A bad shape is refused by name without a lane spawn.
    let mock = MockApi::start(vec![
        tool_call_stream("c1", "browse", r#"{"action":"open"}"#),
        tool_call_stream("c2", "measure", r#"{"target":"x"}"#),
        assistant_stream(TOOL_REPORT),
    ]);
    let mut session = Session::create(&home, "test-model").unwrap();
    run_on(&mock, &mut session, Box::new(ApproveAll), 5, "try it")
        .await
        .unwrap();
    let records = session::replay(session.path()).unwrap();
    let outputs: Vec<&String> = records
        .iter()
        .filter_map(|record| match record {
            Record::ToolResult { output, .. } => Some(output),
            _ => None,
        })
        .collect();
    assert!(
        outputs[0].contains("field `url` is required"),
        "{}",
        outputs[0]
    );
    assert!(
        outputs[1].contains("field `table` expects an array"),
        "{}",
        outputs[1]
    );
    std::fs::remove_dir_all(&home).ok();
}

/// The imagine tool writes the lane's base64 answer as files and names
/// them, never carrying the bytes back to the model.
#[tokio::test]
async fn imagine_writes_files_instead_of_base64() {
    let home = temp_home("imagine-files");
    let png: &[u8] = &[
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90,
        0x77, 0x53, 0xDE,
    ];
    let encoded = fleet::mcp::encode_base64(png);
    let fake = FakeMcp::start(vec![
        serde_json::json!({ "reply": initialize_reply() }),
        serde_json::json!({ "reply": tools_list_reply(serde_json::json!([
            { "name": "gen1_imagine", "description": "render", "inputSchema": { "type": "object" } }
        ])) }),
        serde_json::json!({ "reply": call_reply(
            &serde_json::json!({
                "provider": "qwen",
                "model": "qwen-image",
                "images": [format!("data:image/png;base64,{encoded}")],
            })
            .to_string()
        ) }),
    ]);
    let mock = MockApi::start(vec![
        tool_call_stream("c1", "imagine", r#"{"prompt":"a graphite icon"}"#),
        assistant_stream(TOOL_REPORT),
    ]);
    let mut session = Session::create(&home, "test-model").unwrap();
    let mut specs = std::collections::HashMap::new();
    specs.insert("gen1".to_string(), fake.spec());
    let mut progress = Vec::new();
    std::env::set_current_dir(&home).unwrap();
    run_with_lanes(
        &mock,
        &mut session,
        Box::new(ApproveAll),
        Some(specs),
        &mut progress,
        "render it",
    )
    .await
    .unwrap();
    let output = session::replay(session.path())
        .unwrap()
        .iter()
        .find_map(|record| match record {
            Record::ToolResult { name, output, .. } if name == "imagine" => Some(output.clone()),
            _ => None,
        })
        .expect("the imagine result");
    assert!(output.contains("provider qwen"), "{output}");
    assert!(!output.contains("base64"), "{output}");
    assert!(home
        .join("generated")
        .read_dir()
        .map(|mut d| d.next().is_some())
        .unwrap_or(false));
    let restore = std::env::current_dir()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let _ = std::env::set_current_dir(restore);
    std::fs::remove_dir_all(&home).ok();
}

/// A name that merely starts with `supersede` is not supersede: the
/// dispatch matches the exact tool name, so a lookalike is an unknown.
#[tokio::test]
async fn a_name_that_only_starts_with_supersede_is_not_supersede() {
    let home = temp_home("supersede-exact");
    let mock = MockApi::start(vec![
        tool_call_stream("c1", "supersede_extra", "{}"),
        assistant_stream(TOOL_REPORT),
    ]);
    let mut session = Session::create(&home, "test-model").unwrap();
    run_on(&mock, &mut session, Box::new(ApproveAll), 5, "try it")
        .await
        .unwrap();
    let records = session::replay(session.path()).unwrap();
    let output = records
        .iter()
        .find_map(|record| match record {
            Record::ToolResult { output, .. } => Some(output),
            _ => None,
        })
        .expect("the result");
    assert!(
        output.starts_with("error: unknown tool `supersede_extra`"),
        "{output}"
    );
    std::fs::remove_dir_all(&home).ok();
}

#[tokio::test]
async fn streamed_tokens_reach_progress_before_the_run_ends() {
    // Three deltas with a pause between them, so the run is observably
    // mid-stream while the first one lands.
    let mut frames = vec![
        content_delta("Wri"),
        content_delta("ting "),
        content_delta("tokens."),
    ];
    frames.push(finish_frame("stop"));
    frames.push(usage_frame(10, 5));
    frames.push("data: [DONE]".to_string());
    let mock = MockApi::start(vec![ScriptedResponse {
        status: "200 OK".to_string(),
        body: String::new(),
        sse: frames,
        delay_ms: 25,
    }]);
    let home = temp_home("progressive");
    let mut session = Session::create(&home, "test-model").unwrap();
    let mut probe = Probe::new();
    let report = run_on_probe(&mock, &mut session, &mut probe, "stream it")
        .await
        .unwrap();

    assert_eq!(report.stop_reason, StopReason::Completed);
    assert_eq!(report.final_message.as_deref(), Some("Writing tokens."));

    // The deltas arrived as separate writes, in order.
    let deltas: Vec<&str> = probe
        .writes
        .iter()
        .map(|write| write.as_str())
        .filter(|write| matches!(*write, "Wri" | "ting " | "tokens."))
        .collect();
    assert_eq!(deltas, vec!["Wri", "ting ", "tokens."]);

    // And they arrived during the turn, not as one blob at the end: at the
    // first delta the run had not yet reached turn 2 or the final answer.
    let first = probe
        .snapshots
        .iter()
        .find(|snapshot| snapshot.contains("Wri"))
        .expect("the first delta was written to progress");
    assert!(first.contains("\u{2014} turn 1"), "{first}");
    assert!(!first.contains("\u{2014} turn 2"), "{first}");
    assert!(!first.contains("tokens."), "{first}");
    assert!(!first.contains("Writing tokens."), "{first}");

    // The whole streamed text is what progress ends up holding, in order.
    assert!(probe.text().contains("Writing tokens."));

    // The assembled answer is logged once, as one assistant record: deltas
    // are transport, not log content.
    let records = session::replay(session.path()).unwrap();
    let assistants: Vec<&Record> = records
        .iter()
        .filter(|record| matches!(record, Record::Assistant { .. }))
        .collect();
    assert_eq!(assistants.len(), 1);
    match assistants[0] {
        Record::Assistant {
            content,
            tool_calls,
            ..
        } => {
            assert_eq!(content.as_deref(), Some("Writing tokens."));
            assert!(tool_calls.is_empty());
        }
        other => panic!("expected an assistant record, got {other:?}"),
    }

    std::fs::remove_dir_all(&home).ok();
}

#[tokio::test]
async fn resume_seeds_the_next_request_from_the_log() {
    let first = MockApi::start(vec![
        tool_call_stream("call-1", "bash", r#"{"command":"echo one"}"#),
        assistant_stream("First answer."),
    ]);
    let home = temp_home("resume");
    let mut session = Session::create(&home, "test-model").unwrap();
    run_on(&first, &mut session, Box::new(ApproveAll), 5, "first task")
        .await
        .unwrap();

    let second = MockApi::start(vec![assistant_stream("Second answer.")]);
    let mut resumed = Session::open(session.path()).unwrap();
    let report = run_on(
        &second,
        &mut resumed,
        Box::new(ApproveAll),
        5,
        "second task",
    )
    .await
    .unwrap();
    assert_eq!(report.stop_reason, StopReason::Completed);

    let bodies = second.bodies();
    let messages = bodies[0]["messages"].as_array().unwrap();
    let roles: Vec<&str> = messages
        .iter()
        .map(|m| m["role"].as_str().unwrap())
        .collect();
    assert_eq!(
        roles,
        vec!["system", "user", "assistant", "tool", "assistant", "user"]
    );
    assert_eq!(messages[1]["content"], "first task");
    assert_eq!(messages[4]["content"], "First answer.");
    assert_eq!(messages[5]["content"], "second task");

    std::fs::remove_dir_all(&home).ok();
}

#[tokio::test]
async fn denied_call_is_recorded_and_shown_to_the_model() {
    let mock = MockApi::start(vec![
        tool_call_stream("call-1", "bash", r#"{"command":"echo no"}"#),
        assistant_stream("Understood, standing down."),
    ]);
    let home = temp_home("denied");
    let mut session = Session::create(&home, "test-model").unwrap();
    let report = run_on(&mock, &mut session, Box::new(DenyAll), 5, "go wild")
        .await
        .unwrap();
    assert_eq!(report.stop_reason, StopReason::Completed);

    let records = session::replay(session.path()).unwrap();
    match first_result(&records) {
        Record::ToolResult {
            output,
            approved: false,
            truncated: false,
            ..
        } => {
            assert_eq!(output, "denied: the user did not approve this call.");
        }
        other => panic!("expected a denied tool result, got {other:?}"),
    }

    let bodies = mock.bodies();
    assert_eq!(
        bodies[1]["messages"][3]["content"],
        "denied: the user did not approve this call."
    );

    std::fs::remove_dir_all(&home).ok();
}

#[tokio::test]
async fn unknown_tool_and_bad_arguments_are_model_visible_errors() {
    let mock = MockApi::start(vec![
        stream(vec![
            tool_call_delta(0, "call-1", "sudo", "{}"),
            tool_call_delta(1, "call-2", "bash", "{not json"),
            finish_frame("tool_calls"),
            usage_frame(10, 5),
            "data: [DONE]".to_string(),
        ]),
        assistant_stream("Recovering."),
    ]);
    let home = temp_home("bad-tools");
    let mut session = Session::create(&home, "test-model").unwrap();
    run_on(&mock, &mut session, Box::new(ApproveAll), 5, "try things")
        .await
        .unwrap();

    let records = session::replay(session.path()).unwrap();
    let outputs: Vec<&str> = records
        .iter()
        .filter_map(|record| match record {
            Record::ToolResult { output, .. } => Some(output.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(outputs.len(), 2);
    assert!(
        outputs[0].starts_with("error: unknown tool `sudo`"),
        "{}",
        outputs[0]
    );
    assert!(
        outputs[1].starts_with("error[validation]: arguments were not valid JSON"),
        "{}",
        outputs[1]
    );

    std::fs::remove_dir_all(&home).ok();
}

#[tokio::test]
async fn tool_failure_is_a_model_visible_error_result() {
    let mock = MockApi::start(vec![
        tool_call_stream(
            "call-1",
            "read_file",
            r#"{"path":"/nonexistent/fleet-test-file"}"#,
        ),
        assistant_stream("Noted."),
    ]);
    let home = temp_home("tool-error");
    let mut session = Session::create(&home, "test-model").unwrap();
    run_on(
        &mock,
        &mut session,
        Box::new(ApproveAll),
        5,
        "read the missing file",
    )
    .await
    .unwrap();

    let records = session::replay(session.path()).unwrap();
    match first_result(&records) {
        Record::ToolResult {
            output,
            approved: true,
            ..
        } => {
            assert!(
                output.starts_with("error: /nonexistent/fleet-test-file"),
                "{output}"
            );
        }
        other => panic!("expected a failed tool result, got {other:?}"),
    }

    std::fs::remove_dir_all(&home).ok();
}

#[tokio::test]
async fn max_turns_stops_the_loop_and_marks_the_log() {
    let scripted = tool_call_stream("call-1", "bash", "{}");
    let mock = MockApi::start(vec![scripted.clone(), scripted]);
    let home = temp_home("max-turns");
    let mut session = Session::create(&home, "test-model").unwrap();
    let report = run_on(&mock, &mut session, Box::new(ApproveAll), 2, "loop forever")
        .await
        .unwrap();

    assert_eq!(report.stop_reason, StopReason::MaxTurns);
    assert_eq!(report.turns, 2);
    assert_eq!(report.final_message, None);
    assert_eq!(mock.bodies().len(), 2);

    let records = session::replay(session.path()).unwrap();
    assert!(matches!(
        records.last(),
        Some(Record::End { stop_reason, turns: 2, .. }) if stop_reason == "max-turns"
    ));

    std::fs::remove_dir_all(&home).ok();
}

#[tokio::test]
async fn oversized_tool_output_is_truncated_in_the_log() {
    // write_file a larger-than-cap file, then read it back through the loop.
    let scratch = temp_home("truncation-scratch");
    let path = scratch.join("big.txt");
    let big = "x".repeat(MAX_TOOL_OUTPUT + 10_000);
    let mock = MockApi::start(vec![
        tool_call_stream(
            "call-1",
            "write_file",
            &format!(r#"{{"path":"{}","content":"{}"}}"#, path.display(), big),
        ),
        tool_call_stream(
            "call-2",
            "read_file",
            &format!(r#"{{"path":"{}"}}"#, path.display()),
        ),
        assistant_stream("Read it, partially."),
    ]);
    let home = temp_home("truncation-run");
    let mut session = Session::create(&home, "test-model").unwrap();
    run_on(
        &mock,
        &mut session,
        Box::new(ApproveAll),
        5,
        "round-trip a big file",
    )
    .await
    .unwrap();

    let records = session::replay(session.path()).unwrap();
    match records
        .iter()
        .find(|record| {
            matches!(
                record,
                Record::ToolResult {
                    truncated: true,
                    ..
                }
            )
        })
        .expect("a truncated result")
    {
        Record::ToolResult {
            output,
            truncated: true,
            ..
        } => {
            assert!(output.contains('\u{2026}'), "expected the line-cut marker");
            assert!(output.len() < MAX_TOOL_OUTPUT + 100);
        }
        other => panic!("expected a truncated tool result, got {other:?}"),
    }

    std::fs::remove_dir_all(&home).ok();
    std::fs::remove_dir_all(&scratch).ok();
}

#[tokio::test]
async fn tools_execute_against_the_real_filesystem() {
    let home = temp_home("tools");
    let target = home.join("note.txt");
    Tool::WriteFile
        .execute(
            &serde_json::json!({ "path": target.display().to_string(), "content": "from fleet" }),
        )
        .unwrap();
    let read = Tool::ReadFile
        .execute(&serde_json::json!({ "path": target.display().to_string() }))
        .unwrap();
    assert_eq!(
        read.output,
        format!("{} (1 lines)\n     1\tfrom fleet\n", target.display())
    );
    let listed = Tool::ListDir
        .execute(&serde_json::json!({ "path": home.display().to_string() }))
        .unwrap();
    assert!(listed.output.contains("note.txt"));
    std::fs::remove_dir_all(&home).ok();
}

// --- the TUI's event seam ---------------------------------------------------

/// One run with a `ui_tx` attached: the events a TUI renders, in the order
/// the loop emits them, beside the unchanged log and report.
#[tokio::test]
async fn a_runs_thinking_is_logged_once_and_heard_before_the_answer() {
    let mock = MockApi::start(vec![stream(vec![
        reasoning_delta("count the rows"),
        reasoning_delta(", then answer"),
        content_delta("done"),
        finish_frame("stop"),
        "data: [DONE]".to_string(),
    ])]);
    let home = temp_home("reasoning-run");
    let mut session = Session::create(&home, "test-model").unwrap();
    let log_path = session.path().to_path_buf();
    let (ui_tx, mut ui_rx) = tokio::sync::mpsc::unbounded_channel::<UiEvent>();
    let options = AgentOptions {
        goal: None,
        model: "test-model".to_string(),
        identity: None,
        max_turns: 3,
        ui_tx: Some(ui_tx.clone()),
        lane_specs: None,
        calib: None,
        skills: Vec::new(),
        role_overlay: None,
        memory: None,
        scope: None,
        improve_home: None,
        route_home: None,
        gui_prompt: None,
        compact_now: false,
        checkpoints: None,
        file_access: FileAccess::Unrestricted,
        plan_mode: false,
    };
    let client = Client::new(mock.url(), "test-key");
    let mut progress = Vec::new();
    {
        let mut agent = Agent::new(
            &client,
            options,
            &mut session,
            Box::new(ApproveAll),
            &mut progress,
        )
        .unwrap();
        agent.run("count them").await.unwrap();
    }
    drop(ui_tx);

    let mut events = Vec::new();
    while let Some(event) = ui_rx.recv().await {
        events.push(event);
    }
    let heard: Vec<(String, u64)> = events
        .iter()
        .filter_map(|event| match event {
            UiEvent::Reasoning { text, seconds } => Some((text.clone(), *seconds)),
            _ => None,
        })
        .collect();
    assert_eq!(heard, vec![("count the rows, then answer".to_string(), 0)]);
    // One block per response, and it lands before the answer it preceded.
    let reasoning_at = events
        .iter()
        .position(|event| matches!(event, UiEvent::Reasoning { .. }))
        .expect("a reasoning event");
    let token_at = events
        .iter()
        .position(|event| matches!(event, UiEvent::Token(_)))
        .expect("a token event");
    assert!(reasoning_at < token_at, "{events:?}");

    // The log keeps the thinking; the conversation never sends it back.
    let records = fleet::session::replay(&log_path).unwrap();
    let kept: Vec<(String, u64)> = records
        .iter()
        .filter_map(|record| match record {
            Record::Reasoning { text, seconds } => Some((text.clone(), *seconds)),
            _ => None,
        })
        .collect();
    assert_eq!(kept, vec![("count the rows, then answer".to_string(), 0)]);
    let sent = fleet::session::conversation(&records);
    assert!(
        sent.messages
            .iter()
            .all(|message| message.content.as_deref() != Some("count the rows, then answer")),
        "{:?}",
        sent.messages
    );
}

#[tokio::test]
async fn ui_events_arrive_in_order_and_beside_the_log() {
    let mock = MockApi::start(vec![
        stream(vec![
            tool_call_delta(0, "call-1", "bash", r#"{"command":"echo hi"}"#),
            finish_frame("tool_calls"),
            usage_frame(10, 5),
            "data: [DONE]".to_string(),
        ]),
        stream(vec![
            content_delta("The command ran and printed hi; "),
            content_delta("nothing needed from you."),
            finish_frame("stop"),
            usage_frame(10, 5),
            "data: [DONE]".to_string(),
        ]),
    ]);
    let home = temp_home("ui-events");
    let mut session = Session::create(&home, "test-model").unwrap();
    let log_path = session.path().to_path_buf();
    let (ui_tx, mut ui_rx) = tokio::sync::mpsc::unbounded_channel::<UiEvent>();
    let options = AgentOptions {
        goal: None,
        model: "test-model".to_string(),
        identity: None,
        max_turns: 5,
        ui_tx: Some(ui_tx.clone()),
        lane_specs: None,
        calib: None,
        skills: Vec::new(),
        role_overlay: None,
        memory: None,
        scope: None,
        improve_home: None,
        route_home: None,
        gui_prompt: None,
        compact_now: false,
        checkpoints: None,
        file_access: FileAccess::Unrestricted,
        plan_mode: false,
    };
    let client = Client::new(mock.url(), "test-key");
    let mut progress = Vec::new();
    let report = {
        let mut agent = Agent::new(
            &client,
            options,
            &mut session,
            Box::new(ApproveAll),
            &mut progress,
        )
        .unwrap();
        agent.run("say hello").await.unwrap()
    };
    drop(ui_tx);
    assert_eq!(report.stop_reason, StopReason::Completed);

    let mut events = Vec::new();
    while let Some(event) = ui_rx.recv().await {
        events.push(event);
    }
    assert_eq!(
        events,
        vec![
            UiEvent::Turn(1),
            // The turn's own reading of the measure the compaction check
            // reads, so a live surface and the compactor see one number.
            UiEvent::Usage {
                prompt_tokens: 10,
                completion_tokens: 5,
            },
            UiEvent::ToolCall {
                name: "bash".to_string(),
                summary: "echo hi".to_string(),
                arguments: Some("{\"command\":\"echo hi\"}".to_string()),
            },
            UiEvent::ToolResult {
                name: "bash".to_string(),
                preview: "exit 0".to_string(),
                approved: true,
                resolution: Some("exact \u{b7} declared".to_string()),
            },
            UiEvent::Turn(2),
            UiEvent::Token("The command ran and printed hi; ".to_string()),
            UiEvent::Token("nothing needed from you.".to_string()),
            UiEvent::Usage {
                prompt_tokens: 10,
                completion_tokens: 5,
            },
            UiEvent::Finished {
                stop_reason: "handed back".to_string()
            },
        ]
    );

    // The headless contract is untouched: the log holds the whole turn, and
    // the progress sink saw the same deltas as raw text.
    let records = session::replay(&log_path).unwrap();
    assert_eq!(records.len(), 8);
    let progress = String::from_utf8(progress).unwrap();
    assert!(progress.contains("bash: echo hi"), "{progress}");
    assert!(progress.contains("exit 0"), "{progress}");

    std::fs::remove_dir_all(&home).ok();
}

/// A closed UI channel does not break the run: emissions are best-effort,
/// the log and the report are the record.
#[tokio::test]
async fn a_closed_ui_channel_does_not_break_the_run() {
    let mock = MockApi::start(vec![assistant_stream("Just text.")]);
    let home = temp_home("ui-headless");
    let mut session = Session::create(&home, "test-model").unwrap();
    let (ui_tx, ui_rx) = tokio::sync::mpsc::unbounded_channel::<UiEvent>();
    let options = AgentOptions {
        goal: None,
        model: "test-model".to_string(),
        identity: None,
        max_turns: 5,
        ui_tx: Some(ui_tx),
        lane_specs: None,
        calib: None,
        skills: Vec::new(),
        role_overlay: None,
        memory: None,
        scope: None,
        improve_home: None,
        route_home: None,
        gui_prompt: None,
        compact_now: false,
        checkpoints: None,
        file_access: FileAccess::Unrestricted,
        plan_mode: false,
    };
    let client = Client::new(mock.url(), "test-key");
    let mut progress = Vec::new();
    let mut agent = Agent::new(
        &client,
        options,
        &mut session,
        Box::new(ApproveAll),
        &mut progress,
    )
    .unwrap();
    // The channel is closed before the run, so nothing can be delivered.
    drop(ui_rx);
    let report = agent.run("hello").await.unwrap();
    assert_eq!(report.final_message.as_deref(), Some("Just text."));
    assert!(report.stop_reason == StopReason::Completed);

    std::fs::remove_dir_all(&home).ok();
}

/// A denied call still reports its result event, with `approved: false`,
/// and the denial still reaches the model through the log.
#[tokio::test]
async fn a_denied_call_reports_its_result_event() {
    let mock = MockApi::start(vec![
        tool_call_stream("call-1", "bash", r#"{"command":"echo no"}"#),
        assistant_stream("Standing down."),
    ]);
    let home = temp_home("ui-denied");
    let mut session = Session::create(&home, "test-model").unwrap();
    let (ui_tx, mut ui_rx) = tokio::sync::mpsc::unbounded_channel::<UiEvent>();
    let options = AgentOptions {
        goal: None,
        model: "test-model".to_string(),
        identity: None,
        max_turns: 5,
        ui_tx: Some(ui_tx.clone()),
        lane_specs: None,
        calib: None,
        skills: Vec::new(),
        role_overlay: None,
        memory: None,
        scope: None,
        improve_home: None,
        route_home: None,
        gui_prompt: None,
        compact_now: false,
        checkpoints: None,
        file_access: FileAccess::Unrestricted,
        plan_mode: false,
    };
    let client = Client::new(mock.url(), "test-key");
    let mut progress = Vec::new();
    {
        let mut agent = Agent::new(
            &client,
            options,
            &mut session,
            Box::new(DenyAll),
            &mut progress,
        )
        .unwrap();
        agent.run("go wild").await.unwrap();
    }
    // The agent's own sender clone is gone with it, so the channel closes.
    drop(ui_tx);

    let mut events = Vec::new();
    while let Some(event) = ui_rx.recv().await {
        events.push(event);
    }
    let result = events
        .iter()
        .find(|event| matches!(event, UiEvent::ToolResult { .. }))
        .expect("a tool result event");
    match result {
        UiEvent::ToolResult {
            name,
            preview,
            approved,
            ..
        } => {
            assert_eq!(name, "bash");
            assert!(!approved);
            assert!(preview.contains("denied"), "{preview}");
        }
        other => panic!("expected a tool result event, got {other:?}"),
    }

    std::fs::remove_dir_all(&home).ok();
}

/// A failing run reports its failure as an event, and the log keeps the
/// interrupted shape.
#[tokio::test]
async fn a_failed_run_reports_a_failure_event() {
    let mock = MockApi::start(vec![ScriptedResponse {
        status: "500 Internal Server Error".to_string(),
        body: r#"{"error":{"message":"boom"}}"#.to_string(),
        sse: Vec::new(),
        delay_ms: 0,
    }]);
    let home = temp_home("ui-failed");
    let mut session = Session::create(&home, "test-model").unwrap();
    let log_path = session.path().to_path_buf();
    let (ui_tx, mut ui_rx) = tokio::sync::mpsc::unbounded_channel::<UiEvent>();
    let options = AgentOptions {
        goal: None,
        model: "test-model".to_string(),
        identity: None,
        max_turns: 5,
        ui_tx: Some(ui_tx.clone()),
        lane_specs: None,
        calib: None,
        skills: Vec::new(),
        role_overlay: None,
        memory: None,
        scope: None,
        improve_home: None,
        route_home: None,
        gui_prompt: None,
        compact_now: false,
        checkpoints: None,
        file_access: FileAccess::Unrestricted,
        plan_mode: false,
    };
    let client = Client::new(mock.url(), "test-key");
    let mut progress = Vec::new();
    let failed = {
        let mut agent = Agent::new(
            &client,
            options,
            &mut session,
            Box::new(ApproveAll),
            &mut progress,
        )
        .unwrap();
        agent.run("break things").await.is_err()
    };
    assert!(failed);
    drop(ui_tx);

    let mut events = Vec::new();
    while let Some(event) = ui_rx.recv().await {
        events.push(event);
    }
    match events.last() {
        Some(UiEvent::Failed(message)) => assert!(message.contains("boom"), "{message}"),
        other => panic!("expected a failure event, got {other:?}"),
    }
    let records = session::replay(&log_path).unwrap();
    assert!(records
        .iter()
        .all(|record| !matches!(record, Record::End { .. })));

    std::fs::remove_dir_all(&home).ok();
}

// --- MCP client and the vendored lanes --------------------------------------

/// A scripted fake MCP stdio server in a child process (the `mock-mcp`
/// binary): the paths the real lanes cannot produce deterministically.
struct FakeMcp {
    child: std::process::Child,
    scratch: PathBuf,
}

impl FakeMcp {
    /// Start the fake over `entries` (one script entry per request).
    fn start(entries: Vec<serde_json::Value>) -> FakeMcp {
        let scratch = temp_home("mock-mcp");
        let script_path = scratch.join("script.jsonl");
        let script = entries
            .iter()
            .map(|entry| format!("{entry}\n"))
            .collect::<String>();
        std::fs::write(&script_path, script).unwrap();
        let child = std::process::Command::new(env!("CARGO_BIN_EXE_mock-mcp"))
            .arg(&script_path)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("spawn the mock-mcp child");
        FakeMcp { child, scratch }
    }

    /// The client-side launch spec for the fake.
    fn spec(&self) -> ServerSpec {
        ServerSpec {
            program: env!("CARGO_BIN_EXE_mock-mcp").to_string(),
            args: vec![self
                .scratch
                .join("script.jsonl")
                .to_string_lossy()
                .into_owned()],
            env: Vec::new(),
            passthrough: Vec::new(),
            cwd: None,
        }
    }
}

impl Drop for FakeMcp {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        std::fs::remove_dir_all(&self.scratch).ok();
    }
}

/// Test budgets: generous for the happy paths, so a real failure is never
/// masked by a tight clock on a loaded machine.
fn budgets(seconds: u64) -> Timeouts {
    Timeouts {
        initialize: std::time::Duration::from_secs(seconds),
        list: std::time::Duration::from_secs(seconds),
        call: std::time::Duration::from_secs(seconds),
    }
}

/// The initialize result every scripted reply below reuses.
fn initialize_reply() -> serde_json::Value {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "result": {
            "protocolVersion": "2024-11-05",
            "capabilities": { "tools": { "listChanged": false } },
            "serverInfo": { "name": "fake", "version": "9.9.9" },
        },
    })
}

/// The id-`null` `-32601` frame sys1 and gen1 emit in answer to
/// `notifications/initialized` (KI-002).
fn stray_initialized_error() -> serde_json::Value {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": null,
        "result": { "error": { "code": -32601, "message": "Method not found: notifications/initialized" } },
    })
}

/// Is `python3` on PATH? The real-lane tests skip with a loud note when
/// it is not, so a Python-less machine stays green without silently
/// passing.
fn python3_available() -> bool {
    std::process::Command::new("python3")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

/// One registry lane by name.
fn lane(name: &str) -> Lane {
    *lanes::LANES
        .iter()
        .find(|lane| lane.name == name)
        .expect("a registry lane")
}

/// sem1's resolved home; the vendored copy must always answer.
fn sem1_home() -> PathBuf {
    match lanes::resolve_home(&lane("sem1")) {
        LaneHome::Resolved { home, .. } => home,
        LaneHome::Unavailable { reason } => {
            panic!("sem1 must resolve from the vendored copy: {reason}")
        }
    }
}

/// The real vendored sem1 lane, driven end to end: initialize (with the
/// `notifications/initialized` notification the servers deviate on),
/// tools/list, a tools/call error path, and clean teardown.
#[tokio::test]
async fn the_real_sem1_lane_round_trips_through_the_client() {
    if !python3_available() {
        eprintln!("python3 is not on PATH; skipping the real-lane round trip");
        return;
    }
    let spec = lanes::launch_spec(&lane("sem1"), &sem1_home());
    let server = McpServer::spawn(&spec).expect("spawn the sem1 lane");

    let initialize = server.initialize().await.expect("initialize");
    assert_eq!(initialize.protocol_version, "2024-11-05");
    assert_eq!(initialize.server_name, "sem1");
    assert_eq!(initialize.server_version, "0.1.0");

    let tools = server.list_tools().await.expect("tools/list");
    let names: Vec<&str> = tools.iter().map(|tool| tool.name.as_str()).collect();
    assert_eq!(
        names,
        vec!["sem1_embed", "sem1_probe", "sem1_index", "sem1_search"]
    );
    assert_eq!(
        tools[0].input_schema["required"],
        serde_json::json!(["items"])
    );

    // A tools/call the server refuses comes back as a typed JSON-RPC
    // error, not a hang and not a panic.
    let error = server
        .call_tool("sem1_embed", serde_json::json!({ "items": 42 }))
        .await
        .expect_err("invalid items must be refused");
    match error {
        McpError::Rpc { code, message } => {
            assert_eq!(code, -32602);
            assert!(message.contains("`items`"), "{message}");
        }
        other => panic!("expected an Rpc error, got {other:?}"),
    }

    // The success path: sem1_probe reports reachability without raising,
    // whether or not this machine runs an embedding endpoint.
    let probe = server
        .call_tool("sem1_probe", serde_json::json!({}))
        .await
        .expect("sem1_probe answers");
    let report: serde_json::Value =
        serde_json::from_str(probe.content[0].data.as_str().expect("a text block"))
            .expect("the probe report is JSON");
    assert_eq!(report["provider"], "llama-server");
    assert!(report.get("reachable").is_some(), "{report}");

    // Teardown: stdin closes, the child exits 0, nothing lingers.
    server.close().await.expect("clean teardown");
}

/// The registry's probe wrapper over the real lane: the health row the
/// headless command and the TUI both render.
#[tokio::test]
async fn the_lane_probe_reports_the_real_sem1_as_ok() {
    if !python3_available() {
        eprintln!("python3 is not on PATH; skipping the real-lane probe");
        return;
    }
    let health = lanes::probe_lane(&lane("sem1")).await;
    assert_eq!(health.state, LaneState::Ok, "{}", health.detail);
    assert_eq!(health.name, "sem1");
    assert_eq!(health.role, "embeddings");
    assert_eq!(health.server_version.as_deref(), Some("0.1.0"));
    assert_eq!(health.tools.len(), 4);
    assert!(health.detail.contains("python3 -u"), "{}", health.detail);
}

/// The client round-trips a scripted fake and ignores the id-`null`
/// stray frame sys1 and gen1 emit (KI-002).
#[tokio::test]
async fn the_client_round_trips_a_fake_server_and_ignores_strays() {
    let fake = FakeMcp::start(vec![
        serde_json::json!({ "reply": initialize_reply() }),
        serde_json::json!({
            "stray": stray_initialized_error(),
            "reply": {
                "jsonrpc": "2.0",
                "id": 2,
                "result": { "tools": [{ "name": "fake_tool", "description": "a fake", "inputSchema": { "type": "object" } }] },
            },
        }),
        serde_json::json!({
            "reply": {
                "jsonrpc": "2.0",
                "id": 3,
                "result": { "content": [{ "type": "text", "text": "called" }], "isError": false },
            },
        }),
    ]);
    let server = McpServer::spawn_with(&fake.spec(), budgets(5)).expect("spawn the fake");

    let initialize = server.initialize().await.expect("initialize");
    assert_eq!(initialize.server_name, "fake");
    assert_eq!(initialize.server_version, "9.9.9");

    let tools = server.list_tools().await.expect("tools/list");
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].name, "fake_tool");

    let call = server
        .call_tool("fake_tool", serde_json::json!({ "x": 1 }))
        .await
        .expect("tools/call");
    assert_eq!(call.content.len(), 1);
    assert_eq!(call.content[0].data, serde_json::json!("called"));
    assert!(!call.is_error);

    server.close().await.expect("clean teardown");
}

/// A child that dies mid-session fails the pending wait — an error, never
/// a hang.
#[tokio::test]
async fn a_dead_child_is_an_error_not_a_hang() {
    let fake = FakeMcp::start(vec![
        serde_json::json!({ "reply": initialize_reply() }),
        serde_json::json!({ "exit": 1 }),
    ]);
    let server = McpServer::spawn_with(&fake.spec(), budgets(5)).expect("spawn the fake");
    server.initialize().await.expect("initialize");

    let error = server
        .list_tools()
        .await
        .expect_err("a dead child must fail the wait");
    assert!(matches!(error, McpError::Closed(_)), "got {error:?}");
    // The teardown of an already-dead child is still graceful.
    server.close().await.expect("teardown after death");
}

/// A lane that logs loudly on stderr does not wedge its own call: the
/// pipe is drained for the child's whole life, so the writes land
/// somewhere instead of blocking the child on a full buffer (KI-043).
#[tokio::test]
async fn a_chatty_stderr_child_does_not_wedge_the_call() {
    // 256 KiB, four times the OS pipe buffer: past it, a child with no
    // reader blocks on its next write and never answers, and the call
    // fails on its budget instead of on the work.
    let chatty = 256 * 1024;
    let fake = FakeMcp::start(vec![
        serde_json::json!({ "stderr_bytes": chatty, "reply": initialize_reply() }),
        serde_json::json!({ "reply": tools_list_reply(serde_json::json!([
            { "name": "embed", "description": "an embed", "inputSchema": { "type": "object" } },
        ])) }),
        serde_json::json!({ "reply": call_reply("embedded") }),
    ]);
    let server = McpServer::spawn_with(&fake.spec(), budgets(5)).expect("spawn the fake");

    let started = std::time::Instant::now();
    server
        .initialize()
        .await
        .expect("initialize despite the chatter");
    server
        .list_tools()
        .await
        .expect("tools/list despite the chatter");
    let call = server
        .call_tool("embed", serde_json::json!({}))
        .await
        .expect("tools/call");
    assert_eq!(call.text(), "embedded");
    // The budget is 5s and the drain is the only reason this is not the
    // time it takes to hit it.
    assert!(
        started.elapsed() < std::time::Duration::from_secs(5),
        "the stderr pipe must not wedge the call: {:?}",
        started.elapsed()
    );

    server.close().await.expect("clean teardown");
}

/// A child that dies during startup leaves its reason on stderr, and the
/// handshake failure carries it — the drain's other half (KI-043).
#[tokio::test]
async fn a_child_that_dies_during_startup_reports_its_stderr() {
    let fake = FakeMcp::start(vec![
        serde_json::json!({ "reply": initialize_reply() }),
        // The child says why on stderr, then dies a moment later. The
        // moment is what makes the drain's read deterministic rather than
        // a race against the failure being built.
        serde_json::json!({
            "stderr": "ModuleNotFoundError: no module named 'glinter'",
            "exit": 1,
            "delay_ms": 100,
        }),
    ]);
    let server = McpServer::spawn_with(&fake.spec(), budgets(5)).expect("spawn the fake");
    server.initialize().await.expect("initialize");

    let error = server
        .list_tools()
        .await
        .expect_err("the child died during startup");
    assert!(matches!(error, McpError::Closed(_)), "got {error:?}");
    assert!(
        error.to_string().contains("no module named 'glinter'"),
        "the child's reason must reach the caller: {error}"
    );

    server.close().await.expect("teardown after death");
}

/// A child that never answers times out on its budget.
#[tokio::test]
async fn a_silent_child_times_out_on_the_budget() {
    let fake = FakeMcp::start(vec![
        serde_json::json!({ "reply": initialize_reply() }),
        serde_json::json!({ "hang": true }),
    ]);
    let server = McpServer::spawn_with(
        &fake.spec(),
        Timeouts {
            initialize: std::time::Duration::from_secs(5),
            list: std::time::Duration::from_millis(300),
            call: std::time::Duration::from_secs(5),
        },
    )
    .expect("spawn the fake");
    server.initialize().await.expect("initialize");

    let started = std::time::Instant::now();
    let error = server
        .list_tools()
        .await
        .expect_err("a silent child must time out");
    let elapsed = started.elapsed();
    assert!(
        elapsed < std::time::Duration::from_secs(5),
        "the budget must fire: {elapsed:?}"
    );
    match error {
        McpError::Timeout { method, budget } => {
            assert_eq!(method, "tools/list");
            assert_eq!(budget, std::time::Duration::from_millis(300));
        }
        other => panic!("expected a timeout, got {other:?}"),
    }
    server.close().await.expect("teardown after timeout");
}

/// Two requests in flight correlate even when the replies land out of
/// order: the id-keyed pending map, not arrival order, routes them.
#[tokio::test]
async fn concurrent_requests_correlate_when_replies_land_out_of_order() {
    let fake = FakeMcp::start(vec![
        serde_json::json!({ "reply": initialize_reply() }),
        // The first-arriving request's reply is delayed, so the second
        // request's reply lands first.
        serde_json::json!({
            "reply": { "jsonrpc": "2.0", "id": "$echo", "result": { "tools": [{ "name": "slow" }] } },
            "delay_ms": 400,
        }),
        serde_json::json!({
            "reply": { "jsonrpc": "2.0", "id": "$echo", "result": { "tools": [{ "name": "fast" }] } },
        }),
    ]);
    let server = McpServer::spawn_with(&fake.spec(), budgets(5)).expect("spawn the fake");
    server.initialize().await.expect("initialize");

    let started = std::time::Instant::now();
    let (first, second) = tokio::join!(server.list_tools(), server.list_tools());
    let first = first.expect("the delayed request completed");
    let second = second.expect("the immediate request completed");
    assert!(
        started.elapsed() >= std::time::Duration::from_millis(300),
        "the delayed reply must have been awaited, not dropped"
    );
    let mut names = vec![first[0].name.clone(), second[0].name.clone()];
    names.sort();
    assert_eq!(names, vec!["fast".to_string(), "slow".to_string()]);

    server.close().await.expect("clean teardown");
}

/// `fleet lanes` end to end against the real vendored lanes: cards, one
/// per lane, every one ok.
/// KI-018: a connection that carries no request — a bare TCP probe,
/// a health check, a scanner — is closed and the mock keeps
/// serving. The loop used to `break` on it, so one empty connection
/// took the mock down mid-suite and failed whichever tests were
/// still to spawn against it.
#[test]
fn a_connection_with_no_request_does_not_take_the_mock_down() {
    let mock = MockApi::start(vec![ok(chat_response(serde_json::json!({
        "role": "assistant",
        "content": "hi"
    })))]);
    // A bare connection: connect, say nothing, drop.
    {
        let _probe = std::net::TcpStream::connect(("127.0.0.1", mock.port)).unwrap();
    }
    // The mock still answers a real request.
    let client = Client::new(mock.url(), "test-key");
    let reply = client.chat(&ChatRequest {
        model: "test-model",
        messages: &[Message::text(Role::User, "hello")],
        tools: Vec::new(),
    });
    assert!(reply.is_ok(), "{reply:?}");
}

/// KI-069: `fleet remember` end to end — the flags, the arguments
/// object, the writer, and the file on disk — driven through the real
/// binary against a scratch corpus, the way the owner runs it.
#[test]
fn the_remember_command_writes_the_fact_end_to_end() {
    let home = temp_home("remember-e2e");
    // The corpus root the run adopts: FLEET_HOME's own `memories/`, which
    // is where `resolve_root` looks first.
    let corpus = home.join("memories");
    std::fs::create_dir_all(&corpus).unwrap();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_fleet"))
        .args([
            "remember",
            "--name",
            "an-end-to-end-fact",
            "--description",
            "the CLI writes the file the writer would write",
            "--basis",
            "inferred",
            "--shape",
            "docs,deps",
        ])
        .env("FLEET_HOME", &home)
        .current_dir(&home)
        .output()
        .expect("run the fleet binary");
    assert!(output.status.success(), "{output:?}");
    let written = std::fs::read_to_string(corpus.join("an-end-to-end-fact.md"))
        .expect("the fact file on disk");
    assert!(written.contains("name: an-end-to-end-fact"), "{written}");
    assert!(
        written.contains("description: \"the CLI writes the file the writer would write\""),
        "{written}"
    );
    assert!(written.contains("basis: inferred"), "{written}");
    assert!(written.contains("docs"), "{written}");
    assert!(written.contains("deps"), "{written}");
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn the_lanes_command_reports_every_real_lane() {
    if !python3_available() {
        eprintln!("python3 is not on PATH; skipping the real-lanes command");
        return;
    }
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_fleet"))
        .arg("lanes")
        .output()
        .expect("run the fleet binary");
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    for name in ["sys1", "sdm1", "sem1", "gen1", "browser", "blend1"] {
        assert!(stdout.contains(name), "{stdout}");
    }
    // Cards, not a table.
    assert!(stdout.contains("┌─"), "{stdout}");
    assert_eq!(stdout.matches('┌').count(), 6, "{stdout}");
    assert!(stdout.contains("6 lanes"), "{stdout}");
}

/// A wrong services root degrades loudly: every lane unavailable, one
/// named reason per card, exit 1.
#[test]
fn the_lanes_command_reports_unavailable_when_the_root_is_wrong() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_fleet"))
        .arg("lanes")
        .env("FLEET_SERVICES_ROOT", "/definitely/not/a/services/root")
        .output()
        .expect("run the fleet binary");
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("unavailable"), "{stdout}");
    assert!(stdout.contains("6 lanes"), "{stdout}");
    assert!(stdout.contains("┌─"), "{stdout}");
}

/// A per-lane home override shadows the bundled copy and names itself as
/// the source.
#[test]
fn a_lane_home_override_shadows_the_bundled_copy() {
    if !python3_available() {
        eprintln!("python3 is not on PATH; skipping the lane override");
        return;
    }
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_fleet"))
        .arg("lanes")
        .env("FLEET_LANE_SEM1_HOME", sem1_home())
        .output()
        .expect("run the fleet binary");
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("home: FLEET_LANE_SEM1_HOME"), "{stdout}");
}

/// A relative lane home is refused with a named reason instead of
/// resolving against whatever directory the caller stood in.
#[test]
fn a_relative_lane_home_fails_with_a_named_reason() {
    if !python3_available() {
        eprintln!("python3 is not on PATH; skipping the relative-home check");
        return;
    }
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_fleet"))
        .arg("lanes")
        .env("FLEET_LANE_SEM1_HOME", "services/sem1")
        .env_remove("FLEET_SERVICES_ROOT")
        .output()
        .expect("run the fleet binary");
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("relative"), "{stdout}");
    assert!(stdout.contains("FLEET_LANE_SEM1_HOME"), "{stdout}");
    // Only the lane whose home was overridden is affected; the other four
    // still boot from the bundled copy.
    assert!(stdout.contains("6 lanes"), "{stdout}");
    assert!(stdout.contains("○ unavailable"), "{stdout}");
}

/// A relative services root is a typed error, not a silent fallback to the
/// bundled copy.
#[test]
fn a_relative_services_root_fails_with_a_named_reason() {
    if !python3_available() {
        eprintln!("python3 is not on PATH; skipping the relative-root check");
        return;
    }
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_fleet"))
        .arg("lanes")
        .env("FLEET_SERVICES_ROOT", "services")
        .output()
        .expect("run the fleet binary");
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("FLEET_SERVICES_ROOT=services"), "{stdout}");
    assert!(stdout.contains("relative"), "{stdout}");
}

/// The child process the client spawns does not see the caller's
/// credential-shaped environment (KI-031). The fake server answers a call
/// with its own environment, so the assertion is on what actually crossed
/// the spawn boundary rather than on the list the client builds.
#[tokio::test]
// The ambient lock is a std mutex by design: it serializes the process's
// environment against the other tests for the whole awaited body, because
// the thing guarded (env) is process-global and not async.
#[allow(clippy::await_holding_lock)]
async fn a_spawned_child_does_not_inherit_the_ambient_environment() {
    let fake = FakeMcp::start(vec![
        serde_json::json!({ "reply": initialize_reply() }),
        serde_json::json!({ "env": true }),
    ]);
    let server = McpServer::spawn_with(&fake.spec(), budgets(5)).expect("spawn the fake");
    server.initialize().await.expect("initialize");

    // The DeepSeek key is in this process's environment while the call is
    // in flight, which is exactly the shape the fix is about.
    let _ambient_lock = AMBIENT_LOCK.lock().unwrap();
    let _ambient = AmbientKey::set("DEEPSEEK_API_KEY", "sk-ambient-must-not-cross");
    let _unrelated = AmbientKey::set("FLEET_UNRELATED_AMBIENT", "also-must-not-cross");

    let call = server
        .call_tool("fake_tool", serde_json::json!({}))
        .await
        .expect("the env-echo call answers");
    let reported: std::collections::BTreeMap<String, String> =
        serde_json::from_str(call.content[0].data.as_str().expect("a text block"))
            .expect("the echoed environment is a JSON object");

    assert!(!reported.contains_key("DEEPSEEK_API_KEY"), "{reported:?}");
    assert!(
        !reported.contains_key("FLEET_UNRELATED_AMBIENT"),
        "{reported:?}"
    );
    // The base crossed: the lanes launch as `python3`, resolved through the
    // child's PATH, so PATH must be there.
    assert!(reported.contains_key("PATH"), "{reported:?}");
    assert!(reported.contains_key("HOME"), "{reported:?}");

    server.close().await.expect("clean teardown");
}

/// Set one ambient variable for the duration of a test and restore it —
/// including its absence — on the way out. The env-mutating tests hold
/// `AMBIENT_LOCK` so a `set_var` is never visible mid-flight to another.
static AMBIENT_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

struct AmbientKey {
    name: String,
    previous: Option<std::ffi::OsString>,
}

impl AmbientKey {
    fn set(name: &str, value: &str) -> AmbientKey {
        let previous = std::env::var_os(name);
        std::env::set_var(name, value);
        AmbientKey {
            name: name.to_string(),
            previous,
        }
    }
}

impl Drop for AmbientKey {
    fn drop(&mut self) {
        match &self.previous {
            Some(value) => std::env::set_var(&self.name, value),
            None => std::env::remove_var(&self.name),
        }
    }
}

/// A single stdout line past the cap fails the wait with a typed protocol
/// error instead of growing the reader's buffer without bound.
#[tokio::test]
async fn a_line_past_the_cap_is_a_typed_error_not_an_unbounded_read() {
    let fake = FakeMcp::start(vec![
        serde_json::json!({ "reply": initialize_reply() }),
        // One line about 9 MiB long: past the 8 MiB cap, still far short
        // of anything a lane legitimately emits.
        serde_json::json!({ "huge_line_bytes": 9 * 1024 * 1024 }),
    ]);
    let server = McpServer::spawn_with(&fake.spec(), budgets(20)).expect("spawn the fake");
    server.initialize().await.expect("initialize");

    let started = std::time::Instant::now();
    let error = server
        .call_tool("fake_tool", serde_json::json!({}))
        .await
        .expect_err("a line past the cap must fail the wait");
    assert!(
        started.elapsed() < std::time::Duration::from_secs(10),
        "the cap must fire, not the budget: {:?}",
        started.elapsed()
    );
    match error {
        McpError::Protocol(detail) => {
            assert!(detail.contains("longer than the 8 MiB cap"), "{detail}")
        }
        other => panic!("expected a protocol error, got {other:?}"),
    }

    // The teardown is still graceful: the child exits on stdin EOF even
    // though its writer is parked on a full pipe.
    server.close().await.expect("teardown after the cap");
}

/// A reply that lands after its budget expired finds no waiter and is
/// ignored, so the next request still correlates.
#[tokio::test]
async fn a_late_reply_after_the_timeout_is_dropped_cleanly() {
    let fake = FakeMcp::start(vec![
        serde_json::json!({ "reply": initialize_reply() }),
        // 400 ms late against a 200 ms budget.
        serde_json::json!({
            "reply": { "jsonrpc": "2.0", "id": "$echo", "result": { "tools": [{ "name": "late" }] } },
            "delay_ms": 400,
        }),
        serde_json::json!({
            "reply": { "jsonrpc": "2.0", "id": "$echo", "result": { "tools": [{ "name": "next" }] } },
        }),
    ]);
    let server = McpServer::spawn_with(
        &fake.spec(),
        Timeouts {
            initialize: std::time::Duration::from_secs(5),
            list: std::time::Duration::from_millis(200),
            call: std::time::Duration::from_secs(5),
        },
    )
    .expect("spawn the fake");
    server.initialize().await.expect("initialize");

    let error = server
        .list_tools()
        .await
        .expect_err("the budget must fire first");
    assert!(matches!(error, McpError::Timeout { .. }), "got {error:?}");

    // The late reply arrives with no waiter left for its id: it is
    // ignored, and the request after it still correlates by id.
    let tools = server.list_tools().await.expect("the next request answers");
    assert_eq!(tools[0].name, "next");

    server.close().await.expect("clean teardown");
}

/// Teardown during a call: the child goes away while the call is waiting,
/// and the call ends with a typed error rather than a hang. `close` cannot
/// express this from the test's side — it consumes the server while a call
/// borrows it — so the child's own exit is the shape.
#[tokio::test]
async fn a_child_that_goes_away_during_a_call_fails_the_call_typed() {
    let fake = FakeMcp::start(vec![
        serde_json::json!({ "reply": initialize_reply() }),
        // The child exits instead of answering the call.
        serde_json::json!({ "exit": 0 }),
    ]);
    let server = McpServer::spawn_with(&fake.spec(), budgets(5)).expect("spawn the fake");
    server.initialize().await.expect("initialize");

    let error = server
        .call_tool("fake_tool", serde_json::json!({}))
        .await
        .expect_err("a call into a departing child must fail");
    assert!(matches!(error, McpError::Closed(_)), "got {error:?}");

    // And the teardown afterwards is still graceful.
    server.close().await.expect("teardown after the child left");
}

/// Slice 5 drives concurrent calls on one server through `tokio::spawn`,
/// which needs the server and its call futures to be `Send`. Holding a
/// `std::sync::MutexGuard` across the stdin write's await made them
/// `!Send`; the field is a `tokio::sync::Mutex` now. The spawn below is the
/// compile-time proof, and the round trip is the runtime one.
#[tokio::test]
async fn two_spawned_calls_share_one_server() {
    let fake = FakeMcp::start(vec![
        serde_json::json!({ "reply": initialize_reply() }),
        serde_json::json!({
            "reply": { "jsonrpc": "2.0", "id": "$echo",
                       "result": { "content": [{ "type": "text", "text": "first" }], "isError": false } },
        }),
        serde_json::json!({
            "reply": { "jsonrpc": "2.0", "id": "$echo",
                       "result": { "content": [{ "type": "text", "text": "second" }], "isError": false } },
        }),
    ]);
    let server = McpServer::spawn_with(&fake.spec(), budgets(5)).expect("spawn the fake");
    server.initialize().await.expect("initialize");
    let server = std::sync::Arc::new(server);

    let first = {
        let server = std::sync::Arc::clone(&server);
        tokio::spawn(async move { server.call_tool("fake_tool", serde_json::json!({})).await })
    };
    let second = {
        let server = std::sync::Arc::clone(&server);
        tokio::spawn(async move { server.call_tool("fake_tool", serde_json::json!({})).await })
    };
    let first = first
        .await
        .unwrap()
        .expect("the first spawned call")
        .content[0]
        .data
        .clone();
    let second = second
        .await
        .unwrap()
        .expect("the second spawned call")
        .content[0]
        .data
        .clone();
    let mut answered = vec![
        first.as_str().unwrap_or_default(),
        second.as_str().unwrap_or_default(),
    ];
    answered.sort();
    assert_eq!(answered, vec!["first", "second"]);
}

// --- the agent loop's lane tools -------------------------------------------

/// The headline of the slice-4 remainder: a full two-turn run against the
/// scripted mock API whose turn 1 calls `mcp__sem1__probe`. The agent
/// spawns the REAL sem1 lane on that call, the call succeeds, and turn 2
/// answers — with the lane's tools in the model's tool set from then on.
#[tokio::test]
async fn a_full_run_calls_the_real_sem1_lane_through_the_mock_api() {
    if !python3_available() {
        eprintln!("python3 is not on PATH; skipping the real-lane agent run");
        return;
    }
    let mock = MockApi::start(vec![
        tool_call_stream("call-1", "mcp__sem1__probe", "{}"),
        assistant_stream("The embeddings worker answers; llama-server is not running here."),
    ]);
    let home = temp_home("lane-run");
    let mut session = Session::create(&home, "test-model").unwrap();
    let mut progress = Vec::new();
    // No injected specs: the production path, resolved through the
    // vendored registry the first time the lane's tool is called.
    let report = run_with_lanes(
        &mock,
        &mut session,
        Box::new(ApproveAll),
        None,
        &mut progress,
        "probe the embeddings worker",
    )
    .await
    .unwrap();

    assert_eq!(report.stop_reason, StopReason::Completed);
    assert_eq!(report.turns, 2);
    assert_eq!(
        report.final_message.as_deref(),
        Some("The embeddings worker answers; llama-server is not running here.")
    );

    // Lazy by construction: turn 1's request was built before the run had
    // met the lane, so it declared only the four built-ins and the
    // delegation tool. Turn 2's request carries the lane's four tools
    // under the namespace too.
    let bodies = mock.bodies();
    assert_eq!(bodies.len(), 2);
    assert_eq!(
        declared_tools(&bodies[0]),
        vec![
            "bash",
            "read_file",
            "write_file",
            "list_dir",
            "edit",
            "apply_patch",
            "grep",
            "glob",
            "repo_map",
            "file_change",
            "todo",
            "measure",
            "imagine",
            "browse",
            "recall",
            "mcp_resource",
            "mcp_prompt",
            "subagent",
            "convention",
            "plan",
            "deck",
            "extract",
            "vision",
            "web",
            "read_document",
            "send_file",
            "computer_use",
            // The background handle: a command started
            // in the background is controllable only if the
            // control tool is declared beside it.
            "process"
        ]
    );
    let second = declared_tools(&bodies[1]);
    assert_eq!(second.len(), 32, "{second:?}");
    assert_eq!(
        &second[..4],
        &["bash", "read_file", "write_file", "list_dir"]
    );
    // By name, not position: the base declarations move as the surface
    // grows, and the lane's tools land after them.
    for name in [
        "mcp__sem1__embed",
        "mcp__sem1__probe",
        "mcp__sem1__index",
        "mcp__sem1__search",
    ] {
        assert!(second.contains(&name.to_string()), "{second:?}");
    }
    assert_eq!(second.last().unwrap(), "mcp__sem1__search");
    // The native tools ride before the joined lane's, and the lane's
    // tools are the deferred tail of the declaration list.
    assert_eq!(
        &second[11..28],
        &[
            "measure",
            "imagine",
            "browse",
            "recall",
            "mcp_resource",
            "mcp_prompt",
            "subagent",
            "convention",
            "plan",
            "deck",
            "extract",
            "vision",
            "web",
            "read_document",
            "send_file",
            "computer_use",
            // The background handle: a command started
            // in the background is controllable only if the
            // control tool is declared beside it.
            "process"
        ],
        "{second:?}"
    );
    // The tool result the model read rode back in the same request.
    let answer = bodies[1]["messages"][3]["content"]
        .as_str()
        .expect("the tool result");
    assert!(answer.contains("llama-server"), "{answer}");
    assert!(answer.contains("reachable"), "{answer}");

    // The log holds the whole turn, in order: the assistant's call, the
    // lane that joined for it, the result, and the answer.
    let records = session::replay(session.path()).unwrap();
    assert_eq!(records.len(), 9, "{records:?}");
    assert!(
        matches!(&records[2], Record::User { content } if content == "probe the embeddings worker")
    );
    assert!(
        matches!(&records[3], Record::Assistant { tool_calls, .. }
            if tool_calls.len() == 1 && tool_calls[0].function.name == "mcp__sem1__probe"),
        "{:?}",
        records[3]
    );
    // The reconstructability record: the lane, and every tool it brought
    // with the description and schema the model was shown.
    match &records[4] {
        Record::LaneTools { lane, tools } => {
            assert_eq!(lane, "sem1");
            let names: Vec<&str> = tools.iter().map(|tool| tool.name.as_str()).collect();
            assert_eq!(
                names,
                vec![
                    "mcp__sem1__embed",
                    "mcp__sem1__probe",
                    "mcp__sem1__index",
                    "mcp__sem1__search"
                ]
            );
            // The lane's own names ride along, so the log names both ends
            // of the mapping the model's call went through.
            let own: Vec<&str> = tools.iter().map(|tool| tool.lane_tool.as_str()).collect();
            assert_eq!(
                own,
                vec!["sem1_embed", "sem1_probe", "sem1_index", "sem1_search"]
            );
            for tool in tools {
                assert!(
                    !tool.description.is_empty(),
                    "{} has no description",
                    tool.name
                );
                assert_eq!(tool.input_schema["type"], "object", "{}", tool.name);
            }
        }
        other => panic!("expected a lane_tools record, got {other:?}"),
    }
    match first_result(&records) {
        Record::ToolResult {
            name,
            output,
            approved: true,
            ..
        } => {
            assert_eq!(name, "mcp__sem1__probe");
            assert!(output.contains("llama-server"), "{output}");
        }
        other => panic!("expected a lane tool result, got {other:?}"),
    }
    assert!(matches!(
        records.last(),
        Some(Record::End { stop_reason, turns: 2, .. }) if stop_reason == "handed back"
    ));

    // And the headless surface said the lane joined.
    let progress = String::from_utf8(progress).unwrap();
    assert!(
        progress.contains("lane sem1 joined \u{2014} 4 tools"),
        "{progress}"
    );
    assert!(progress.contains("mcp__sem1__probe"), "{progress}");

    std::fs::remove_dir_all(&home).ok();
}

/// A lane whose child dies mid-run degrades instead of killing the run:
/// the first call records the typed failure, every later call to that
/// lane fails fast with no respawn, and the run answers anyway.
#[tokio::test]
async fn a_dead_lane_child_degrades_the_run_without_killing_it() {
    let fake = FakeMcp::start(vec![
        serde_json::json!({ "reply": initialize_reply() }),
        serde_json::json!({ "reply": tools_list_reply(serde_json::json!([
            { "name": "probe", "description": "a probe", "inputSchema": { "type": "object" } },
        ])) }),
        // The child exits instead of answering the call: a dead child.
        serde_json::json!({ "exit": 1 }),
    ]);
    let mock = MockApi::start(vec![
        tool_call_stream("call-1", "mcp__sem1__probe", "{}"),
        tool_call_stream("call-2", "mcp__sem1__probe", "{}"),
        assistant_stream("The embeddings worker went away mid-run; that is recorded."),
    ]);
    let home = temp_home("dead-lane");
    let mut session = Session::create(&home, "test-model").unwrap();
    let mut progress = Vec::new();
    let report = run_with_lanes(
        &mock,
        &mut session,
        Box::new(ApproveAll),
        Some(lane_specs(&fake)),
        &mut progress,
        "probe it twice",
    )
    .await
    .unwrap();

    // The run survived its lane's death and answered on its third turn.
    assert_eq!(report.stop_reason, StopReason::Completed);
    assert_eq!(report.turns, 3);
    assert!(report.final_message.is_some());

    // Two failures, two different shapes: the child's death, then the
    // fail-fast that proves nothing respawned.
    let records = session::replay(session.path()).unwrap();
    let outputs: Vec<String> = records
        .iter()
        .filter_map(|record| match record {
            Record::ToolResult { output, .. } => Some(output.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(outputs.len(), 2);
    assert!(
        outputs[0].starts_with("error: the MCP server closed:"),
        "the dead child is a typed failure: {}",
        outputs[0]
    );
    assert!(
        outputs[1].starts_with("error: lane `sem1` is gone for the rest of this run"),
        "the second call fails fast, it does not respawn: {}",
        outputs[1]
    );
    // The lane joined exactly once: no second `lane_tools` record, so no
    // second spawn and no second handshake.
    let joins = records
        .iter()
        .filter(|record| matches!(record, Record::LaneTools { .. }))
        .count();
    assert_eq!(joins, 1, "{records:?}");

    // The lane's tools stayed in the model's tool set after it died: the
    // model can still see them, and calling one is a typed failure.
    let bodies = mock.bodies();
    assert_eq!(bodies.len(), 3);
    assert_eq!(declared_tools(&bodies[1]).len(), 29);
    assert_eq!(declared_tools(&bodies[2]).len(), 29);
    assert!(declared_tools(&bodies[1]).contains(&"mcp__sem1__probe".to_string()));

    std::fs::remove_dir_all(&home).ok();
}

/// A lane joining a run is visible on both surfaces: the headless
/// progress line and the UI's event stream, beside the unchanged log.
/// The run also closes the child it spawned — the fake's EOF marker is
/// the graceful teardown's signature.
#[tokio::test]
async fn a_lane_joining_is_visible_and_its_child_is_closed() {
    let marker_dir = temp_home("lane-marker");
    let marker = marker_dir.join("closed.marker");
    let fake = FakeMcp::start(vec![
        serde_json::json!({ "reply": initialize_reply(), "on_eof": marker.display().to_string() }),
        serde_json::json!({ "reply": tools_list_reply(serde_json::json!([
            { "name": "probe", "description": "a probe", "inputSchema": { "type": "object" } },
            { "name": "embed", "description": "an embed", "inputSchema": { "type": "object" } },
        ])) }),
        serde_json::json!({ "reply": call_reply("{\"provider\":\"fake\",\"reachable\":true}") }),
    ]);
    let mock = MockApi::start(vec![
        tool_call_stream("call-1", "mcp__sem1__probe", "{}"),
        assistant_stream("Probed."),
    ]);
    let home = temp_home("lane-visible");
    let mut session = Session::create(&home, "test-model").unwrap();
    let (ui_tx, mut ui_rx) = tokio::sync::mpsc::unbounded_channel::<UiEvent>();
    let progress_text = {
        let mut progress = Vec::new();
        let client = Client::new(mock.url(), "test-key");
        let options = AgentOptions {
            goal: None,
            model: "test-model".to_string(),
            identity: None,
            max_turns: 5,
            ui_tx: Some(ui_tx.clone()),
            lane_specs: Some(lane_specs(&fake)),
            calib: None,
            skills: Vec::new(),
            role_overlay: None,
            memory: None,
            scope: None,
            improve_home: None,
            route_home: None,
            gui_prompt: None,
            compact_now: false,
            checkpoints: None,
            file_access: FileAccess::Unrestricted,
            plan_mode: false,
        };
        let report = {
            let mut agent = Agent::new(
                &client,
                options,
                &mut session,
                Box::new(ApproveAll),
                &mut progress,
            )
            .unwrap();
            agent.run("probe it").await.unwrap()
        };
        assert_eq!(report.stop_reason, StopReason::Completed);
        String::from_utf8(progress).unwrap()
    };
    // The agent's own sender clone is gone with it, so the channel closes.
    drop(ui_tx);

    let mut events = Vec::new();
    while let Some(event) = ui_rx.recv().await {
        events.push(event);
    }
    // The lane joins before the first request now (the registry plan's
    // D2: a spec in the map pre-joins, and the user servers are seeded
    // into the map), so its arrival is the run's first tool fact —
    // before turn 1's usage and call.
    assert_eq!(
        events,
        vec![
            UiEvent::LaneJoined {
                lane: "sem1".to_string(),
                tool_count: 2
            },
            UiEvent::Turn(1),
            UiEvent::Usage {
                prompt_tokens: 10,
                completion_tokens: 5,
            },
            UiEvent::ToolCall {
                name: "mcp__sem1__probe".to_string(),
                summary: "embeddings \u{b7} probe {}".to_string(),
                arguments: Some("{}".to_string()),
            },
            UiEvent::ToolResult {
                name: "mcp__sem1__probe".to_string(),
                // The preview keeps one line up to 80 characters: this
                // answer is shorter, so it arrives whole.
                preview: "{\"provider\":\"fake\",\"reachable\":true}".to_string(),
                approved: true,
                resolution: Some("exact \u{b7} lane tool".to_string()),
            },
            UiEvent::Turn(2),
            UiEvent::Token("Probed.".to_string()),
            UiEvent::Usage {
                prompt_tokens: 10,
                completion_tokens: 5,
            },
            UiEvent::Finished {
                stop_reason: "handed back".to_string()
            },
        ]
    );
    // The headless surface said the same thing.
    assert!(
        progress_text.contains("server sem1 joined \u{2014} 2 tools"),
        "{progress_text}"
    );
    // And the run closed the child it spawned rather than killing it.
    assert!(
        marker.exists(),
        "the run must close the lane child it spawned"
    );

    std::fs::remove_dir_all(&home).ok();
    std::fs::remove_dir_all(&marker_dir).ok();
}

/// A lane answer past the tool-output cap is bounded the way every
/// built-in's output is bounded, and the cut is marked the same way
/// (KI-041): a sem1_embed batch is exactly the payload the cap exists
/// for, and neither the model's context nor the session log is the place
/// for it whole.
#[tokio::test]
async fn a_lane_answer_past_the_cap_is_bounded_and_marked() {
    // 256 KiB: four times the 64 KiB tool-output cap, and far under the
    // client's 8 MiB per-line cap, so the frame arrives whole and the
    // tool cap is what cuts it.
    let fake = FakeMcp::start(vec![
        serde_json::json!({ "reply": initialize_reply() }),
        serde_json::json!({ "reply": tools_list_reply(serde_json::json!([
            { "name": "embed", "description": "an embed", "inputSchema": { "type": "object" } },
        ])) }),
        serde_json::json!({ "huge_line_bytes": 4 * MAX_TOOL_OUTPUT }),
    ]);
    let mock = MockApi::start(vec![
        tool_call_stream("call-1", "mcp__sem1__embed", r#"{"items":["a"]}"#),
        assistant_stream("The embeddings came back; the answer was cut at the cap."),
    ]);
    let home = temp_home("lane-bounded");
    let mut session = Session::create(&home, "test-model").unwrap();
    let mut progress = Vec::new();
    let report = run_with_lanes(
        &mock,
        &mut session,
        Box::new(ApproveAll),
        Some(lane_specs(&fake)),
        &mut progress,
        "embed the batch",
    )
    .await
    .unwrap();
    assert_eq!(report.stop_reason, StopReason::Completed);
    assert_eq!(report.turns, 2);

    let records = session::replay(session.path()).unwrap();
    let output = match first_result(&records) {
        Record::ToolResult {
            name,
            output,
            truncated: true,
            approved: true,
            ..
        } => {
            assert_eq!(name, "mcp__sem1__embed");
            // The same bound and the same marker a built-in's cut carries.
            assert!(
                output.len() <= MAX_TOOL_OUTPUT + 64,
                "the lane's answer must be capped at MAX_TOOL_OUTPUT: {}",
                output.len()
            );
            assert!(
                output.contains("\u{2026}[truncated"),
                "the cut must be marked: {output}"
            );
            // The head of the lane's answer survived; the tail did not.
            assert!(output.starts_with('x'), "{output}");
            assert!(!output.ends_with('x'), "the tail must be gone: {output}");
            output.clone()
        }
        other => panic!("expected a bounded lane tool result, got {other:?}"),
    };
    // The model read the bounded text, not the lane's whole answer: the
    // bytes the log recorded rode back in the next request.
    let bodies = mock.bodies();
    let seen = bodies[1]["messages"][3]["content"]
        .as_str()
        .expect("the tool result the model read");
    assert_eq!(seen, output.as_str());

    std::fs::remove_dir_all(&home).ok();
}

/// A lane tool's refusal (`isError`) is recorded as a failure with its
/// text: the model reads why the tool refused, and the run carries on.
#[tokio::test]
async fn a_lane_tool_refusal_is_recorded_as_a_failure_with_its_text() {
    let fake = FakeMcp::start(vec![
        serde_json::json!({ "reply": initialize_reply() }),
        serde_json::json!({ "reply": tools_list_reply(serde_json::json!([
            { "name": "embed", "description": "an embed", "inputSchema": { "type": "object" } },
        ])) }),
        serde_json::json!({
            "reply": {
                "jsonrpc": "2.0",
                "id": "$echo",
                "result": {
                    "content": [{ "type": "text", "text": "`items` must be a string" }],
                    "isError": true,
                },
            },
        }),
    ]);
    let mock = MockApi::start(vec![
        tool_call_stream("call-1", "mcp__sem1__embed", r#"{"items":42}"#),
        assistant_stream("The lane refused; I will fix the arguments."),
    ]);
    let home = temp_home("lane-refusal");
    let mut session = Session::create(&home, "test-model").unwrap();
    let mut progress = Vec::new();
    let report = run_with_lanes(
        &mock,
        &mut session,
        Box::new(ApproveAll),
        Some(lane_specs(&fake)),
        &mut progress,
        "embed the number 42",
    )
    .await
    .unwrap();
    assert_eq!(report.stop_reason, StopReason::Completed);
    assert_eq!(report.turns, 2);

    let records = session::replay(session.path()).unwrap();
    assert!(
        records
            .iter()
            .any(|r| matches!(r, Record::LaneTools { lane, .. } if lane == "sem1")),
        "the pre-join is in the log"
    );
    match first_result(&records) {
        Record::ToolResult {
            name,
            output,
            approved: true,
            ..
        } => {
            assert_eq!(name, "mcp__sem1__embed");
            assert_eq!(output, "error: `items` must be a string");
        }
        other => panic!("expected a refused lane tool result, got {other:?}"),
    }
    // The refusal reached the model in the next request.
    let bodies = mock.bodies();
    assert_eq!(
        bodies[1]["messages"][3]["content"],
        "error: `items` must be a string"
    );

    std::fs::remove_dir_all(&home).ok();
}

/// A `tools/list` that pages is followed to its last cursor: a server
/// that splits its roster is joined whole.
#[tokio::test]
async fn a_paged_tools_list_is_followed_to_the_last_cursor() {
    let fake = FakeMcp::start(vec![
        serde_json::json!({ "reply": initialize_reply() }),
        serde_json::json!({ "reply": {
            "jsonrpc": "2.0", "id": "$echo",
            "result": {
                "tools": [{ "name": "one", "description": "first", "inputSchema": { "type": "object" } }],
                "nextCursor": "page-2"
            }
        }}),
        serde_json::json!({ "reply": {
            "jsonrpc": "2.0", "id": "$echo",
            "result": {
                "tools": [{ "name": "two", "description": "second", "inputSchema": { "type": "object" } }]
            }
        }}),
        serde_json::json!({ "reply": call_reply("ok") }),
    ]);
    let mock = MockApi::start(vec![
        tool_call_stream("call-1", "mcp__sem1__one", "{}"),
        assistant_stream(TOOL_REPORT),
    ]);
    let home = temp_home("paged-tools");
    let mut session = Session::create(&home, "test-model").unwrap();
    let mut progress = Vec::new();
    run_with_lanes(
        &mock,
        &mut session,
        Box::new(ApproveAll),
        Some(lane_specs(&fake)),
        &mut progress,
        "call one",
    )
    .await
    .unwrap();
    let records = session::replay(session.path()).unwrap();
    let tools = records
        .iter()
        .find_map(|record| match record {
            Record::LaneTools { tools, .. } => Some(tools),
            _ => None,
        })
        .expect("the lane joined");
    assert_eq!(tools.len(), 2, "{tools:?}");
    let names: Vec<&str> = tools.iter().map(|tool| tool.lane_tool.as_str()).collect();
    assert!(
        names.contains(&"one") && names.contains(&"two"),
        "{names:?}"
    );
    std::fs::remove_dir_all(&home).ok();
}

/// A denied lane call never spawns the lane: the gate runs first, so a
/// refusal costs no child and leaves no `lane_tools` record behind.
#[tokio::test]
async fn a_denied_lane_call_never_spawns_the_lane() {
    let fake = FakeMcp::start(vec![serde_json::json!({ "reply": initialize_reply() })]);
    let mock = MockApi::start(vec![
        tool_call_stream("call-1", "mcp__sem1__probe", "{}"),
        assistant_stream("Standing down."),
    ]);
    let home = temp_home("lane-denied");
    let mut session = Session::create(&home, "test-model").unwrap();
    let mut progress = Vec::new();
    let report = run_with_lanes(
        &mock,
        &mut session,
        Box::new(DenyAll),
        Some(lane_specs(&fake)),
        &mut progress,
        "probe it",
    )
    .await
    .unwrap();
    assert_eq!(report.stop_reason, StopReason::Completed);

    let records = session::replay(session.path()).unwrap();
    assert!(
        records
            .iter()
            .all(|record| !matches!(record, Record::LaneTools { .. })),
        "a denied call must not join a lane: {records:?}"
    );
    match first_result(&records) {
        Record::ToolResult {
            name,
            output,
            approved: false,
            ..
        } => {
            assert_eq!(name, "mcp__sem1__probe");
            assert_eq!(output, "denied: the user did not approve this call.");
        }
        other => panic!("expected a denied lane tool result, got {other:?}"),
    }
    // No lane joined, so the second request still declares only the
    // built-ins and the delegation tool.
    let bodies = mock.bodies();
    assert_eq!(declared_tools(&bodies[1]).len(), 28);

    std::fs::remove_dir_all(&home).ok();
}

/// A lane this run cannot launch fails that call with its named reason,
/// and the run answers anyway — a missing lane is never fatal.
#[tokio::test]
async fn a_lane_that_cannot_launch_fails_its_call_and_the_run_answers() {
    // An injected spec whose program does not exist: the shape a machine
    // without `python3` produces, without depending on one.
    let mut specs = std::collections::HashMap::new();
    specs.insert(
        "sem1".to_string(),
        ServerSpec {
            program: "fleet-no-such-lane-program".to_string(),
            args: Vec::new(),
            env: Vec::new(),
            passthrough: Vec::new(),
            cwd: None,
        },
    );
    let mock = MockApi::start(vec![
        tool_call_stream("call-1", "mcp__sem1__probe", "{}"),
        tool_call_stream("call-2", "mcp__sem1__probe", "{}"),
        assistant_stream("The lane never started; the reason is in the log."),
    ]);
    let home = temp_home("lane-unlaunchable");
    let mut session = Session::create(&home, "test-model").unwrap();
    let mut progress = Vec::new();
    let report = run_with_lanes(
        &mock,
        &mut session,
        Box::new(ApproveAll),
        Some(specs),
        &mut progress,
        "probe it twice",
    )
    .await
    .unwrap();

    assert_eq!(report.stop_reason, StopReason::Completed);
    assert_eq!(report.turns, 3);
    let records = session::replay(session.path()).unwrap();
    let outputs: Vec<String> = records
        .iter()
        .filter_map(|record| match record {
            Record::ToolResult { output, .. } => Some(output.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(outputs.len(), 2);
    // The lane dies at the pre-join now: the run's first request carries
    // no tools from it, and both calls fail fast — the start failure is
    // named once, on the progress log, not per call.
    assert!(
        outputs[0].starts_with("error: lane `sem1` is gone for the rest of this run"),
        "the pre-join failure fails the call fast: {}",
        outputs[0]
    );
    assert!(
        outputs[1].starts_with("error: lane `sem1` is gone for the rest of this run"),
        "the second call fails fast: {}",
        outputs[1]
    );
    // A lane that never started never joined, so nothing was declared
    // beyond the built-ins and the delegation tool.
    assert!(records
        .iter()
        .all(|record| !matches!(record, Record::LaneTools { .. })));
    assert_eq!(declared_tools(&mock.bodies()[1]).len(), 28);

    std::fs::remove_dir_all(&home).ok();
}

/// A server that fails its run-start join is named in the session, not
/// only in the progress log, and it declares nothing to the model.
#[tokio::test]
async fn a_server_that_fails_to_pre_join_is_recorded_in_the_session() {
    let mock = MockApi::start(vec![assistant_stream("Nothing to do.")]);
    let home = temp_home("pre-join-failure");
    let mut session = Session::create(&home, "test-model").unwrap();
    let mut specs = std::collections::HashMap::new();
    specs.insert(
        "broken".to_string(),
        ServerSpec {
            program: "/no/such/fleet-program".to_string(),
            args: Vec::new(),
            env: Vec::new(),
            passthrough: Vec::new(),
            cwd: None,
        },
    );
    let mut progress = Vec::new();
    run_with_lanes(
        &mock,
        &mut session,
        Box::new(ApproveAll),
        Some(specs),
        &mut progress,
        "say done",
    )
    .await
    .unwrap();
    let records = session::replay(session.path()).unwrap();
    assert!(
        records.iter().any(|record| matches!(
            record,
            Record::LaneFailed { lane, reason } if lane == "broken" && !reason.is_empty()
        )),
        "{records:?}"
    );
    // A failed server declares nothing.
    assert!(!declared_tools(&mock.bodies()[0])
        .iter()
        .any(|name| name.starts_with("mcp__broken__")));
    std::fs::remove_dir_all(&home).ok();
}

// --- multi-provider routing (slice 5) --------------------------------------

/// One roster entry with its key state, for the routing tests.
fn resolved(id: &str, base_url: String, key_required: bool, key: Option<&str>) -> Resolved {
    Resolved {
        candidate: Candidate {
            id: id.to_string(),
            base_url,
            key_env: format!("{}_KEY", id.to_uppercase().replace('-', "_")),
            models: vec![format!("{id}-model")],
            billing: fleet::route::Billing::Plan,
            kind: fleet::route::CandidateKind::Http,
            enabled: true,
            key_required,
            disabled_reason: None,
        },
        key: key.map(str::to_string),
    }
}

/// The headline of slice 5's failover: a full run where the HEAD candidate
/// answers 402 and the second serves — the answer comes from candidate two,
/// and the decision row names both attempts.
#[tokio::test]
async fn a_failed_head_fails_over_to_the_next_candidate() {
    let head = MockApi::start(vec![ScriptedResponse {
        status: "402 Payment Required".to_string(),
        body: r#"{"error":{"message":"insufficient balance"}}"#.to_string(),
        sse: Vec::new(),
        delay_ms: 0,
    }]);
    let second = MockApi::start(vec![assistant_stream("Answered by candidate two.")]);
    let home = temp_home("failover");
    let mut session = Session::create(&home, "test-model").unwrap();

    let resolved = vec![
        resolved("cand-head", head.url(), false, None),
        resolved("cand-two", second.url(), true, Some("sk-fake-test")),
    ];
    let decision = RouteDecision {
        provider: "cand-head".to_string(),
        model: "cand-head-model".to_string(),
        how: How::Fallback,
        workload: None,
        verdict: None,
        execution: None,
        reason: "test head".to_string(),
        judged_from: fleet::route::JudgedFrom::Fresh,
        start_index: 0,
        // The walk is the order, each entry carrying the model its own
        // candidate serves — the failover steps to the second entry,
        // not to the roster's next candidate.
        order: vec![
            ("cand-head".to_string(), "cand-head-model".to_string()),
            ("cand-two".to_string(), "cand-two-model".to_string()),
        ],
    };
    let router = Arc::new(Router::new(resolved, decision));
    let client = Client::routed(router.clone());
    let options = AgentOptions {
        goal: None,
        model: "test-model".to_string(),
        identity: None,
        max_turns: 5,
        ui_tx: None,
        lane_specs: None,
        calib: None,
        skills: Vec::new(),
        role_overlay: None,
        memory: None,
        scope: None,
        improve_home: None,
        route_home: None,
        gui_prompt: None,
        compact_now: false,
        checkpoints: None,
        file_access: FileAccess::Unrestricted,
        plan_mode: false,
    };
    let mut progress = Vec::new();
    let report = {
        let mut agent = Agent::new(
            &client,
            options,
            &mut session,
            Box::new(ApproveAll),
            &mut progress,
        )
        .unwrap();
        agent.run("do the thing").await.unwrap()
    };

    // The answer came from the second candidate.
    assert_eq!(report.stop_reason, StopReason::Completed);
    assert_eq!(
        report.final_message.as_deref(),
        Some("Answered by candidate two.")
    );

    // The head was tried exactly once and failed; the second served. The
    // walk never retried the head.
    assert_eq!(head.bodies().len(), 1, "the head was tried exactly once");
    assert_eq!(second.bodies().len(), 1, "the second served the run");

    // The decision row names both attempts, in order, and the final choice.
    let row = router.row("do the thing", 0);
    assert_eq!(row.provider, "cand-two");
    assert_eq!(row.model, "cand-two-model");
    let attempts: Vec<(&str, &str, Option<&str>)> = row
        .attempts
        .iter()
        .map(|attempt| (attempt.provider.as_str(), attempt.outcome, attempt.class))
        .collect();
    assert_eq!(
        attempts,
        vec![
            ("cand-head", "failed", Some("quota")),
            ("cand-two", "served", None)
        ]
    );
    assert_eq!(
        row.providers_used,
        vec!["cand-head".to_string(), "cand-two".to_string()]
    );

    // The session log is unchanged in vocabulary: no routing record, just
    // the ordinary turn and its orderly end.
    let records = session::replay(session.path()).unwrap();
    assert!(records.iter().any(
        |record| matches!(record, Record::End { stop_reason, .. } if stop_reason == "handed back")
    ));

    std::fs::remove_dir_all(&home).ok();
}

/// sys1 judges the route through the injected-spec seam: the fake lane
/// answers `sys1_classify` with a `hard` workload, which routes to the
/// table's first `hard` candidate — a judged route, not the head. The
/// table is declared here rather than borrowed from the compiled-in
/// default so the seam's proof — the lane's verdict routes to whatever
/// the table names first — holds whatever the owner's default prefers.
#[tokio::test]
async fn sys1_judges_the_route_through_the_injected_spec() {
    let fake = FakeMcp::start(vec![
        serde_json::json!({ "reply": initialize_reply() }),
        serde_json::json!({ "reply": {
            "jsonrpc": "2.0", "id": "$echo",
            "result": { "content": [{ "type": "text",
                "text": r#"{"verdict":"pass","answers":{"glide":{"workload":{"label":"hard","confidence":0.91}}}}"# }],
                "isError": false }
        }}),
    ]);
    let mut specs = std::collections::HashMap::new();
    specs.insert("sys1".to_string(), fake.spec());

    let preferences = vec![fleet::route::Preference {
        workload: "hard",
        candidates: vec![("zai-coding-plan", "GLM-5.3-Flash"), ("stand-in", "auto")],
    }];
    let resolved = vec![
        resolved(
            "stand-in",
            "http://127.0.0.1:8400/v1".to_string(),
            false,
            None,
        ),
        resolved(
            "zai-coding-plan",
            "https://api.z.ai".to_string(),
            true,
            Some("sk-fake-test"),
        ),
    ];
    let (decision, _latency) = fleet::route::select(
        &resolved,
        None,
        "refactor the crate",
        &specs,
        &preferences,
        fleet::route::Gates::default(),
        None,
    )
    .await;
    assert_eq!(decision.how, How::Judged);
    assert_eq!(decision.provider, "zai-coding-plan");
    assert_eq!(decision.workload.as_deref(), Some("hard"));
    assert_eq!(decision.verdict.as_deref(), Some("pass"));
    // The walk starts at the selection — the order's head, position
    // zero — never at the candidate's roster position.
    assert_eq!(decision.start_index, 0);
}

/// The orchestrator's ruling: the walked chain leaves the judged
/// workload. A task judged `quick` is still planned by `hard`'s head —
/// the row records the judged label, the order is `hard`'s own, and
/// the reason names both.
#[tokio::test]
async fn the_orchestrator_walks_hard_whatever_the_task_was_judged() {
    let fake = FakeMcp::start(vec![
        serde_json::json!({ "reply": initialize_reply() }),
        serde_json::json!({ "reply": {
            "jsonrpc": "2.0", "id": "$echo",
            "result": { "content": [{ "type": "text",
                "text": r#"{"verdict":"pass","answers":{"glide":{"workload":{"label":"quick","confidence":0.91}}}}"# }],
                "isError": false }
        }}),
    ]);
    let mut specs = std::collections::HashMap::new();
    specs.insert("sys1".to_string(), fake.spec());

    let preferences = vec![
        fleet::route::Preference {
            workload: "quick",
            candidates: vec![
                ("stand-in", "weak-model"),
                ("zai-coding-plan", "GLM-5.3-Flash"),
            ],
        },
        fleet::route::Preference {
            workload: "hard",
            candidates: vec![("zai-coding-plan", "GLM-5.3-Flash"), ("stand-in", "auto")],
        },
    ];
    let resolved = vec![
        resolved(
            "stand-in",
            "http://127.0.0.1:8400/v1".to_string(),
            false,
            None,
        ),
        resolved(
            "zai-coding-plan",
            "https://api.z.ai".to_string(),
            true,
            Some("sk-fake-test"),
        ),
    ];
    let (decision, _latency) = fleet::route::select(
        &resolved,
        None,
        "a one-line question",
        &specs,
        &preferences,
        fleet::route::Gates::default(),
        None,
    )
    .await;
    // The row still records what sys1 judged.
    assert_eq!(decision.how, How::Judged);
    assert_eq!(decision.workload.as_deref(), Some("quick"));
    // But the walk is the orchestrator's: `hard`'s own chain.
    assert_eq!(decision.provider, "zai-coding-plan");
    assert_eq!(
        decision.order,
        vec![
            ("zai-coding-plan".to_string(), "GLM-5.3-Flash".to_string()),
            ("stand-in".to_string(), "auto".to_string()),
        ]
    );
    assert!(
        decision.reason.contains("sys1 judged `quick`"),
        "{}",
        decision.reason
    );
    assert!(
        decision
            .reason
            .contains("the orchestrator walks `hard`'s chain"),
        "{}",
        decision.reason
    );
}

/// The 2026-10-21 narrowing: the orchestrator's chain is the *leader's*.
/// A child's route — [`fleet::route::Walk::Judged`], which is what
/// `prepare_child` walks — follows the judged workload, so a brief sys1
/// judges `quick` heads `quick`'s own chain while the same verdict
/// plans the leader on `hard`'s. Both orders close on the reserve.
#[tokio::test]
async fn a_child_walks_the_judged_workload_while_the_leader_keeps_hard() {
    let quick_verdict = || {
        serde_json::json!({ "reply": {
            "jsonrpc": "2.0", "id": "$echo",
            "result": { "content": [{ "type": "text",
                "text": r#"{"verdict":"pass","answers":{"glide":{"workload":{"label":"quick","confidence":0.93}}}}"# }],
                "isError": false }
        }})
    };
    let preferences = vec![
        fleet::route::Preference {
            workload: "quick",
            candidates: vec![
                ("stand-in", "weak-model"),
                ("zai-coding-plan", "GLM-5.3-Flash"),
            ],
        },
        fleet::route::Preference {
            workload: "hard",
            candidates: vec![("zai-coding-plan", "GLM-5.3-Flash"), ("stand-in", "auto")],
        },
    ];
    let resolved = vec![
        resolved(
            "stand-in",
            "http://127.0.0.1:8400/v1".to_string(),
            false,
            None,
        ),
        resolved(
            "zai-coding-plan",
            "https://api.z.ai".to_string(),
            true,
            Some("sk-fake-test"),
        ),
        resolved(
            "deepseek",
            "https://api.deepseek.com".to_string(),
            true,
            Some("sk-fake-reserve"),
        ),
    ];

    let leader_judge = FakeMcp::start(vec![
        serde_json::json!({ "reply": initialize_reply() }),
        quick_verdict(),
    ]);
    let mut specs = std::collections::HashMap::new();
    specs.insert("sys1".to_string(), leader_judge.spec());
    let (leader, _) = fleet::route::select_walk(
        &resolved,
        None,
        "a one-line question",
        &specs,
        &preferences,
        fleet::route::Gates::default(),
        None,
        fleet::route::Walk::Leader,
    )
    .await;

    let child_judge = FakeMcp::start(vec![
        serde_json::json!({ "reply": initialize_reply() }),
        quick_verdict(),
    ]);
    let mut specs = std::collections::HashMap::new();
    specs.insert("sys1".to_string(), child_judge.spec());
    let (child, _) = fleet::route::select_walk(
        &resolved,
        None,
        "a one-line question",
        &specs,
        &preferences,
        fleet::route::Gates::default(),
        None,
        fleet::route::Walk::Judged,
    )
    .await;

    // Same verdict on both rows.
    assert_eq!(leader.workload.as_deref(), Some("quick"));
    assert_eq!(child.workload.as_deref(), Some("quick"));
    // The leader plans on `hard`'s head; the child works on `quick`'s.
    assert_eq!(leader.provider, "zai-coding-plan");
    assert_eq!(child.provider, "stand-in");
    assert_eq!(child.model, "weak-model");
    assert!(
        leader.reason.contains("the orchestrator walks"),
        "{}",
        leader.reason
    );
    assert!(
        !child.reason.contains("the orchestrator walks"),
        "{}",
        child.reason
    );
    // Every walk the router builds closes on the reserve, which the
    // resolved roster carries this time — with the reserve entry's own
    // default model.
    let tail = ("deepseek".to_string(), "deepseek-model".to_string());
    assert_eq!(leader.order.last(), Some(&tail), "{:?}", leader.order);
    assert_eq!(child.order.last(), Some(&tail), "{:?}", child.order);
    assert_eq!(
        child.order.iter().filter(|(p, _)| p == "deepseek").count(),
        1
    );
}

/// The owner's override of `hard` governs the orchestrator's walk: the
/// merged table's `hard` list is what the walk consumes.
#[tokio::test]
async fn the_orchestrators_walk_follows_a_hard_override() {
    let fake = FakeMcp::start(vec![
        serde_json::json!({ "reply": initialize_reply() }),
        serde_json::json!({ "reply": {
            "jsonrpc": "2.0", "id": "$echo",
            "result": { "content": [{ "type": "text",
                "text": r#"{"verdict":"pass","answers":{"glide":{"workload":{"label":"hard","confidence":0.91}}}}"# }],
                "isError": false }
        }}),
    ]);
    let mut specs = std::collections::HashMap::new();
    specs.insert("sys1".to_string(), fake.spec());

    // The merged table as an override of `hard` would leave it: the
    // owner's list replaces the compiled-in one.
    let preferences = vec![fleet::route::Preference {
        workload: "hard",
        candidates: vec![("stand-in", "overridden-model")],
    }];
    let resolved = vec![resolved(
        "stand-in",
        "http://127.0.0.1:8400/v1".to_string(),
        false,
        None,
    )];
    let (decision, _latency) = fleet::route::select(
        &resolved,
        None,
        "refactor the crate",
        &specs,
        &preferences,
        fleet::route::Gates::default(),
        None,
    )
    .await;
    assert_eq!(decision.how, How::Judged);
    assert_eq!(decision.provider, "stand-in");
    assert_eq!(decision.model, "overridden-model");
}

/// The route-decision cache: a recorded judgment serves without
/// spawning the lane again, and the decision says the verdict came
/// from the record. The no-spawn proof is the mock's own `on_eof`
/// marker — a child that runs reaches EOF and writes it, and a cache
/// hit spawns nothing, so no marker appears. The cache is reloaded
/// from its file between the two runs, so the round trip through disk
/// is what the hit reads.
#[tokio::test]
async fn a_recorded_judgment_serves_without_spawning_the_lane() {
    let home = temp_home("route-decision-cache");
    let marker = home.join("spawned");
    let script = home.join("script.jsonl");
    // The judge's live path is initialize → call (no tools/list), so
    // the script carries exactly those two entries (KI-201's order
    // rule). The `on_eof` marker is the spawn's own signature.
    std::fs::write(
        &script,
        format!(
            "{}\n{}\n",
            serde_json::json!({ "reply": initialize_reply(), "on_eof": marker.display().to_string() }),
            serde_json::json!({ "reply": call_reply(
                r#"{"verdict":"pass","answers":{"glide":{"workload":{"label":"hard","confidence":0.91}}}}"#
            ) }),
        ),
    )
    .unwrap();
    let spec = fleet::mcp::ServerSpec {
        program: env!("CARGO_BIN_EXE_mock-mcp").to_string(),
        args: vec![script.to_string_lossy().into_owned()],
        env: Vec::new(),
        passthrough: Vec::new(),
        cwd: None,
    };
    let mut specs = std::collections::HashMap::new();
    specs.insert("sys1".to_string(), spec);
    let preferences = vec![fleet::route::Preference {
        workload: "hard",
        candidates: vec![("zai-coding-plan", "GLM-5.3-Flash")],
    }];
    let resolved = vec![resolved(
        "zai-coding-plan",
        "https://api.z.ai".to_string(),
        true,
        Some("sk-fake-test"),
    )];

    // The first run: a miss — judged live, written through to the file.
    let mut cache = fleet::route_cache::Cache::load(&home);
    let (first, _) = fleet::route::select(
        &resolved,
        None,
        "refactor the crate",
        &specs,
        &preferences,
        fleet::route::Gates::default(),
        Some(&mut cache),
    )
    .await;
    assert_eq!(first.how, How::Judged);
    assert_eq!(first.judged_from, fleet::route::JudgedFrom::Fresh);
    assert!(
        marker.exists(),
        "the miss spawned the lane: the child reached EOF and wrote its marker"
    );

    // The second run reads the cache back from its file: a hit, with
    // nothing spawned.
    std::fs::remove_file(&marker).unwrap();
    let mut cache = fleet::route_cache::Cache::load(&home);
    let (second, _) = fleet::route::select(
        &resolved,
        None,
        "refactor the crate",
        &specs,
        &preferences,
        fleet::route::Gates::default(),
        Some(&mut cache),
    )
    .await;
    assert_eq!(second.judged_from, fleet::route::JudgedFrom::Cached);
    assert_eq!(second.workload.as_deref(), Some("hard"));
    assert_eq!(second.verdict.as_deref(), Some("pass"));
    assert_eq!(second.provider, "zai-coding-plan");
    assert!(
        !marker.exists(),
        "the hit spawned nothing: no child, so no marker"
    );
}

/// The composed workload's candidate is a lane, not a gateway: the full
/// production path — `prepare` judges the workload through the injected
/// sys1 seam, the composed judgment targets blend1, and the walk drives
/// the blend1 lane through the MCP client with the ask payload mapped
/// at the boundary. The answer comes back in the chat shape, and the
/// lane's own facts (path, escalation, score) ride the route row's
/// reason rather than being flattened away.
#[tokio::test]
async fn a_composed_judgment_answers_through_the_blend1_lane() {
    // The judge seam: sys1 answers the route judgment `composed`. The
    // judge's live path is initialize → call (no tools/list), so the
    // script carries exactly those two entries — an extra one would be
    // served to the call (KI-201's order rule).
    let judge = FakeMcp::start(vec![
        serde_json::json!({ "reply": initialize_reply() }),
        serde_json::json!({ "reply": call_reply(
            r#"{"verdict":"pass","answers":{"glide":{"workload":{"label":"composed","confidence":0.88}}}}"#
        ) }),
    ]);
    // The answer seam: blend1 declares `ask` and answers with its own
    // structured result — the shape `BlendResult.as_dict()` carries.
    // The lane boundary DOES list tools before calling, so this script
    // carries the three entries its three requests consume in order.
    let answer = FakeMcp::start(vec![
        serde_json::json!({ "reply": initialize_reply() }),
        serde_json::json!({ "reply": tools_list_reply(serde_json::json!([
            { "name": "ask", "description": "the grounding ask", "inputSchema": { "type": "object" } }
        ])) }),
        serde_json::json!({ "reply": call_reply(
            r#"{"path":"synthesized","final_answer":"the grounded answer","escalated":false,
                "score":{"correct":true,"confidence":0.8},"total_latency_ms":812}"#
        ) }),
    ]);
    let mut specs = std::collections::HashMap::new();
    specs.insert("sys1".to_string(), judge.spec());
    specs.insert("blend1".to_string(), answer.spec());

    // The task is judged `composed`, but the orchestrator walks `hard`
    // (the owner's ruling), so the owner's override aims `hard` at the
    // lane: the row records the judgment, the walk follows the ruling.
    let home = temp_home("composed-lane");
    std::fs::write(
        home.join("routing.json"),
        serde_json::json!({
            "workloads": [
                { "workload": "hard", "candidates": ["blend1/blend1"] }
            ]
        })
        .to_string(),
    )
    .unwrap();
    let prepared = fleet::route::prepare(
        &home,
        &std::collections::HashMap::new(),
        "compose the answer",
        &specs,
    )
    .await
    .unwrap();
    assert_eq!(prepared.decision.how, How::Judged);
    assert_eq!(prepared.decision.provider, "blend1");
    assert_eq!(prepared.decision.workload.as_deref(), Some("composed"));

    // The run: the walk drives the lane, and the agent sees the lane's
    // answer as an ordinary assistant message. The session is created
    // with the routed model, the production order — the route is
    // prepared first, so the log's start record names what serves.
    let mut session = Session::create(&home, &prepared.decision.model).unwrap();
    let client = Client::routed(prepared.router.clone());
    let options = AgentOptions {
        goal: None,
        model: prepared.decision.model.clone(),
        identity: None,
        max_turns: 3,
        ui_tx: None,
        lane_specs: None,
        calib: Some(prepared.calib.clone()),
        skills: Vec::new(),
        role_overlay: None,
        memory: None,
        scope: None,
        improve_home: None,
        route_home: None,
        gui_prompt: None,
        compact_now: false,
        checkpoints: None,
        file_access: FileAccess::Unrestricted,
        plan_mode: false,
    };
    let report = {
        let mut progress = Vec::new();
        let mut agent = Agent::new(
            &client,
            options,
            &mut session,
            Box::new(ApproveAll),
            &mut progress,
        )
        .unwrap();
        agent.run("compose the answer").await.unwrap()
    };
    assert_eq!(report.stop_reason, StopReason::Completed);
    assert_eq!(report.final_message.as_deref(), Some("the grounded answer"));

    // The route row names the lane's own facts — the adapter surfaces
    // blend1's differences, never flattens them.
    let row = prepared.router.row("compose the answer", 0);
    assert_eq!(row.provider, "blend1");
    assert_eq!(row.model, "blend1");
    assert!(
        row.reason
            .contains("blend1 answered through path `synthesized`"),
        "{}",
        row.reason
    );
    assert!(row.reason.contains("escalated: false"), "{}", row.reason);
    assert!(row.reason.contains("\"correct\":true"), "{}", row.reason);
    assert!(row.reason.contains("lane latency 812ms"), "{}", row.reason);
    // The session log is an ordinary session: the routed model in the
    // start record, the lane's answer as the turn's content.
    let records = session::replay(session.path()).unwrap();
    assert!(records.iter().any(|record| matches!(
        record,
        Record::Start { model, .. } if model == "blend1"
    )));
    std::fs::remove_dir_all(&home).ok();
}

/// The lane that fails open is a named refusal that advances the walk:
/// blend1 refuses the ask, its own words ride the route row, the
/// attempt's class names the lane's failure, and the workload's next
/// candidate serves — the consuming-lanes rule inside the walk.
#[tokio::test]
async fn a_lane_that_refuses_advances_the_walk_by_name() {
    let stand_in = MockApi::start(vec![assistant_stream("Answered by the stand-in.")]);
    let home = temp_home("composed-lane-refusal");
    // A declared table: the ORCHESTRATOR walks `hard`, and the owner's
    // override makes that blend1, then the stand-in. The task is still
    // judged `composed` — the row records the judgment, the walk follows
    // the ruling. The stand-in's endpoint is the mock, so the failover
    // leg lands somewhere real.
    std::fs::write(
        home.join("config.json"),
        serde_json::json!({
            "providers": [
                { "id": "stand-in", "base_url": stand_in.url(), "key_env": "STAND_IN_API_KEY",
                  "models": ["auto"], "key_required": false }
            ]
        })
        .to_string(),
    )
    .unwrap();
    std::fs::write(
        home.join("routing.json"),
        serde_json::json!({
            "workloads": [
                { "workload": "hard", "candidates": ["blend1/blend1", "stand-in/auto"] }
            ]
        })
        .to_string(),
    )
    .unwrap();

    let judge = FakeMcp::start(vec![
        serde_json::json!({ "reply": initialize_reply() }),
        serde_json::json!({ "reply": call_reply(
            r#"{"verdict":"pass","answers":{"glide":{"workload":{"label":"composed","confidence":0.88}}}}"#
        ) }),
    ]);
    // The lane that fails open: the call runs, the tool refuses, and
    // the refusal's own words are the answer.
    let refusing = FakeMcp::start(vec![
        serde_json::json!({ "reply": initialize_reply() }),
        serde_json::json!({ "reply": tools_list_reply(serde_json::json!([
            { "name": "ask", "description": "the grounding ask", "inputSchema": { "type": "object" } }
        ])) }),
        serde_json::json!({ "reply": {
            "jsonrpc": "2.0", "id": "$echo",
            "result": { "content": [{ "type": "text",
                "text": "the corpus for this project is not indexed" }], "isError": true }
        } }),
    ]);
    let mut specs = std::collections::HashMap::new();
    specs.insert("sys1".to_string(), judge.spec());
    specs.insert("blend1".to_string(), refusing.spec());

    let prepared = fleet::route::prepare(
        &home,
        &std::collections::HashMap::new(),
        "compose the answer",
        &specs,
    )
    .await
    .unwrap();
    assert_eq!(prepared.decision.provider, "blend1");

    let mut session = Session::create(&home, "test-model").unwrap();
    let client = Client::routed(prepared.router.clone());
    let options = AgentOptions {
        goal: None,
        model: "test-model".to_string(),
        identity: None,
        max_turns: 3,
        ui_tx: None,
        lane_specs: None,
        calib: Some(prepared.calib.clone()),
        skills: Vec::new(),
        role_overlay: None,
        memory: None,
        scope: None,
        improve_home: None,
        route_home: None,
        gui_prompt: None,
        compact_now: false,
        checkpoints: None,
        file_access: FileAccess::Unrestricted,
        plan_mode: false,
    };
    let report = {
        let mut progress = Vec::new();
        let mut agent = Agent::new(
            &client,
            options,
            &mut session,
            Box::new(ApproveAll),
            &mut progress,
        )
        .unwrap();
        agent.run("compose the answer").await.unwrap()
    };
    // The stand-in served: the lane's refusal advanced the walk.
    assert_eq!(
        report.final_message.as_deref(),
        Some("Answered by the stand-in.")
    );
    let row = prepared.router.row("compose the answer", 0);
    assert_eq!(row.provider, "stand-in");
    let attempts: Vec<(&str, &str, Option<&str>)> = row
        .attempts
        .iter()
        .map(|attempt| (attempt.provider.as_str(), attempt.outcome, attempt.class))
        .collect();
    assert_eq!(
        attempts,
        vec![
            ("blend1", "failed", Some("lane")),
            ("stand-in", "served", None)
        ]
    );
    // The lane's own words ride the row: the refusal is named, not a
    // bare class the reader must interpret.
    assert!(
        row.reason
            .contains("the corpus for this project is not indexed"),
        "{}",
        row.reason
    );
    std::fs::remove_dir_all(&home).ok();
}

/// The owner's control, from the surface's end: a steer lands beside
/// the session's own log in the shape the turn loop's reader consumes
/// (one JSON object naming `steer` or `stop`), a stop lands its own
/// shape, and a control naming a session with no log is a named
/// refusal — the control rides beside a log that exists.
#[tokio::test]
async fn an_owners_control_lands_beside_the_log_it_steers() {
    let home = temp_home("delegate-control");
    let sessions = fleet::home::sessions_dir(&home);
    std::fs::create_dir_all(&sessions).unwrap();
    // One live session: its log is what a control rides beside.
    std::fs::write(
        sessions.join("sess-live.jsonl"),
        "{\"type\":\"start\",\"v\":1,\"model\":\"test-model\",\"created\":\"2026-10-14T12:00:00Z\"}\n",
    )
    .unwrap();

    let steer_path =
        fleet::delegate::write_control(&home, "sess-live", Some("focus on the parser"))
            .expect("the steer lands");
    assert_eq!(steer_path, sessions.join("sess-live.control"));
    let written: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&steer_path).unwrap()).unwrap();
    // The reader's contract: `steer` names the text, and the file is
    // consumed on read — the surface writes exactly that shape.
    assert_eq!(written["steer"], "focus on the parser");
    assert!(written.get("stop").is_none());

    let stop_path =
        fleet::delegate::write_control(&home, "sess-live", None).expect("the stop lands");
    let written: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&stop_path).unwrap()).unwrap();
    assert_eq!(written["stop"], true);

    // A control naming a session whose log does not exist refuses by
    // name — never a file nothing reads.
    let error = fleet::delegate::write_control(&home, "sess-absent", Some("text")).unwrap_err();
    assert!(error.contains("no session named `sess-absent`"), "{error}");
    assert!(error.contains("beside a log that exists"), "{error}");

    // The card is a card: framed, naming the session and the kind, and
    // the path it rides — bounded, the card's own discipline. (The
    // width discipline is pinned in delegate's own tests beside the
    // disposition card's.)
    let card = fleet::delegate::control_card("sess-live", Some("focus on the parser"), &steer_path);
    assert!(
        card.contains('\u{250c}') && card.contains('\u{2518}'),
        "{card}"
    );
    assert!(card.contains("steer"), "{card}");
    assert!(card.contains("session sess-live"), "{card}");
    assert!(card.contains("focus on the parser"), "{card}");
    assert!(card.contains("rides"), "{card}");
    assert!(card.contains("one-shot"), "{card}");
    // A stop's card names the stop and no preview.
    let stop_card = fleet::delegate::control_card("sess-live", None, &stop_path);
    assert!(stop_card.contains("stop"), "{stop_card}");
    assert!(!stop_card.contains("focus on the parser"), "{stop_card}");
    std::fs::remove_dir_all(&home).ok();
}

/// A role is the delegation's specialization, and it shows up where
/// the model will read it: the child's own request carries the role's
/// overlay in its system prompt, delimited, and its declarations are
/// the role's admitted set — the tools outside it are not merely
/// uncallable in the child, they are absent from what the child saw.
#[tokio::test]
async fn a_named_role_rides_the_childs_prompt_and_narrows_its_tools() {
    let home = temp_home("role-overlay");
    std::fs::create_dir_all(fleet::roles::roles_dir(&home)).unwrap();
    std::fs::write(
        fleet::roles::roles_dir(&home).join("auditor.md"),
        "---\ndescription: reads a diff and names what it proves\ntools: bash,read_file\n---\n\nYou are auditing a change. Read the diff before you judge it.\n",
    )
    .unwrap();
    let mock = MockApi::start(vec![
        tool_call_stream(
            "call-1",
            "subagent",
            &serde_json::json!({
                "goal": "audit the change",
                "role": "auditor",
            })
            .to_string(),
        ),
        assistant_stream("the audit is complete"),
        assistant_stream("the child audited it"),
    ]);
    let mut session = Session::create(&home, "test-model").unwrap();
    let client = Client::new(mock.url(), "test-key");
    let options = AgentOptions {
        goal: None,
        model: "test-model".to_string(),
        identity: None,
        max_turns: 5,
        ui_tx: None,
        lane_specs: None,
        calib: None,
        skills: Vec::new(),
        role_overlay: None,
        memory: None,
        scope: None,
        improve_home: Some(home.clone()),
        route_home: None,
        gui_prompt: None,
        compact_now: false,
        checkpoints: None,
        file_access: FileAccess::Unrestricted,
        plan_mode: false,
    };
    let report = {
        let mut progress = Vec::new();
        let mut agent = Agent::new(
            &client,
            options,
            &mut session,
            Box::new(ApproveAll),
            &mut progress,
        )
        .unwrap();
        agent.run("delegate the audit").await.unwrap()
    };
    assert_eq!(report.stop_reason, StopReason::Completed);

    // The child's own request: its system prompt carries the role's
    // overlay, delimited so the child knows what is standing
    // instruction and what is the harness prompt.
    let bodies = mock.bodies();
    assert_eq!(bodies.len(), 3, "{bodies:?}");
    let child_system = bodies[1]["messages"][0]["content"].as_str().unwrap();
    assert!(
        child_system.contains("ROLE OVERLAY BEGIN"),
        "{child_system}"
    );
    assert!(child_system.contains("role `auditor`"), "{child_system}");
    assert!(
        child_system.contains("Read the diff before you judge it."),
        "{child_system}"
    );
    // And its declarations are the role's admitted set — the harness
    // prompt rode along, but write_file was never declared to it.
    let declared = declared_tools(&bodies[1]);
    assert_eq!(declared, vec!["bash", "read_file"], "{declared:?}");
    // The parent's own prompt is untouched: a role is a delegation's
    // specialization, never the owner's session's.
    let parent_system = bodies[0]["messages"][0]["content"].as_str().unwrap();
    assert!(!parent_system.contains("ROLE OVERLAY"), "{parent_system}");
    std::fs::remove_dir_all(&home).ok();
}

/// A role that names no role under the home is a named refusal, never
/// a silent default: the child never runs, the tool result says why,
/// and the parent's run continues without it.
#[tokio::test]
async fn an_unknown_role_is_refused_by_name_and_the_child_never_runs() {
    let home = temp_home("role-unknown");
    std::fs::create_dir_all(fleet::roles::roles_dir(&home)).unwrap();
    let mock = MockApi::start(vec![
        tool_call_stream(
            "call-1",
            "subagent",
            &serde_json::json!({
                "goal": "audit the change",
                "role": "no-such-role",
            })
            .to_string(),
        ),
        assistant_stream("the role did not resolve, so I will do it myself"),
    ]);
    let mut session = Session::create(&home, "test-model").unwrap();
    let client = Client::new(mock.url(), "test-key");
    let options = AgentOptions {
        goal: None,
        model: "test-model".to_string(),
        identity: None,
        max_turns: 5,
        ui_tx: None,
        lane_specs: None,
        calib: None,
        skills: Vec::new(),
        role_overlay: None,
        memory: None,
        scope: None,
        improve_home: Some(home.clone()),
        route_home: None,
        gui_prompt: None,
        compact_now: false,
        checkpoints: None,
        file_access: FileAccess::Unrestricted,
        plan_mode: false,
    };
    let report = {
        let mut progress = Vec::new();
        let mut agent = Agent::new(
            &client,
            options,
            &mut session,
            Box::new(ApproveAll),
            &mut progress,
        )
        .unwrap();
        agent.run("delegate the audit").await.unwrap()
    };
    assert_eq!(report.stop_reason, StopReason::Completed);
    // Only the parent's two requests: the child never ran.
    assert_eq!(mock.bodies().len(), 2);
    // The refusal is the tool result the model read, and it names the
    // role and where roles live.
    let (output, _) = tool_result(&session::replay(session.path()).unwrap(), "subagent");
    assert!(output.contains("no role named `no-such-role`"), "{output}");
    assert!(output.contains("fleet roles"), "{output}");
    std::fs::remove_dir_all(&home).ok();
}

/// A real-lane probe of `sys1_classify`'s schema: if the vendored sys1 lane
/// boots on this machine, confirm the call shape the route judgment relies
/// on. If it does not boot (no python3, an import failure), log it and do
/// not block the slice.
#[tokio::test]
async fn the_real_sys1_lane_exposes_the_classify_schema() {
    if !python3_available() {
        eprintln!("python3 is not on PATH; skipping the real sys1 schema probe");
        return;
    }
    let Ok(spec) = lanes::spec_for("sys1") else {
        eprintln!(
            "the sys1 lane did not resolve from the vendored copy; skipping the schema probe"
        );
        return;
    };
    let server = match McpServer::spawn(&spec) {
        Ok(server) => server,
        Err(error) => {
            eprintln!("the sys1 lane could not spawn ({error}); skipping the schema probe");
            return;
        }
    };
    let schema = async {
        server.initialize().await.ok()?;
        let tools = server.list_tools().await.ok()?;
        tools
            .iter()
            .find(|tool| tool.name == "sys1_classify")
            .map(|tool| tool.input_schema.clone())
    }
    .await;
    let _ = server.close().await;
    match schema {
        Some(schema) => {
            // The route call sends {"task": "route", "text": …}; the schema
            // must make `text` required and carry a `task` default. The
            // required check tolerates ADDED fields — a re-vendored sys1
            // that gains a required field must not break this canary; the
            // default stays exact.
            let required = schema["required"].as_array().cloned().unwrap_or_default();
            assert!(
                required.iter().any(|field| field == "text"),
                "sys1_classify must require `text`: {required:?}"
            );
            assert_eq!(schema["properties"]["task"]["default"], "change");
        }
        None => eprintln!(
            "the sys1 lane booted but did not list sys1_classify; skipping the schema assertion"
        ),
    }
}

// --- the calibration stream (slice 6) -------------------------------------

/// One day of the calibration stream, read back through the crate's own
/// reader — the same path `fleet calib` and the demotion take.
fn calib_rows(root: &Path, day: &str) -> Vec<fleet::calib::ReadRow> {
    fleet::calib::read_day(root, day, 512)
}

/// The rows of one op, in arrival order.
fn rows_of<'a>(rows: &'a [fleet::calib::ReadRow], op: &str) -> Vec<&'a fleet::calib::ReadRow> {
    rows.iter().filter(|row| row.op == op).collect()
}

/// A completed run with one approved tool call records both rows: the
/// screening verdict (named after the decision) and the turn ending.
#[tokio::test]
async fn a_completed_run_records_its_screenings_and_its_turn_row() {
    let mock = MockApi::start(vec![
        tool_call_stream("call-1", "list_dir", "{\"path\":\".\"}"),
        assistant_stream("Listed the directory."),
    ]);
    let home = temp_home("calib-turn");
    let calib_root = home.join("calib");
    let stream = fleet::calib::Stream::open(&calib_root);
    let mut session = Session::create(&home, "test-model").unwrap();
    let client = Client::new(mock.url(), "test-key");
    let options = AgentOptions {
        goal: None,
        model: "test-model".to_string(),
        identity: None,
        max_turns: 5,
        ui_tx: None,
        lane_specs: None,
        calib: Some(stream.clone()),
        skills: Vec::new(),
        role_overlay: None,
        memory: None,
        scope: None,
        improve_home: None,
        route_home: None,
        gui_prompt: None,
        compact_now: false,
        checkpoints: None,
        file_access: FileAccess::Unrestricted,
        plan_mode: false,
    };
    let report = {
        let mut progress = Vec::new();
        let mut agent = Agent::new(
            &client,
            options,
            &mut session,
            Box::new(ApproveAll),
            &mut progress,
        )
        .unwrap();
        agent.run("list the directory").await.unwrap()
    };
    assert_eq!(report.stop_reason, StopReason::Completed);
    drop(stream);

    let day = fleet::calib::today();
    let rows = calib_rows(&calib_root, &day);
    // One screening row per approval decision: `--yes` is a run-scoped
    // always, so the verdict is named `always`, and the tool is named.
    let screens = rows_of(&rows, "tool_screen");
    assert_eq!(screens.len(), 1, "{screens:?}");
    assert_eq!(screens[0].tool, "list_dir");
    assert_eq!(screens[0].verdict, "always");
    // One turn row: an orderly stop, graded ok, with the ending named the
    // way the session log names it.
    let turns = rows_of(&rows, "turn");
    assert_eq!(turns.len(), 1, "{turns:?}");
    assert_eq!(turns[0].outcome, "ok");
    assert_eq!(turns[0].stop_reason, "completed");
    assert_eq!(turns[0].turns, report.turns);

    // The task text is nowhere in the stream — only its hash and length.
    let text = std::fs::read_to_string(fleet::calib::day_path(&calib_root, &day)).unwrap();
    assert!(!text.contains("list the directory"), "{text}");
    std::fs::remove_dir_all(&home).ok();
}

/// A run that dies on the API records its turn row too, with the ending
/// named and the router's failure class carried — the stream grades
/// failures, it does not only record successes.
#[tokio::test]
async fn a_failed_run_records_a_turn_row_with_the_failure_named() {
    let mock = MockApi::start(vec![ScriptedResponse {
        status: "500 Internal Server Error".to_string(),
        body: r#"{"error":{"message":"boom"}}"#.to_string(),
        sse: Vec::new(),
        delay_ms: 0,
    }]);
    let home = temp_home("calib-failed");
    let calib_root = home.join("calib");
    let stream = fleet::calib::Stream::open(&calib_root);
    let mut session = Session::create(&home, "test-model").unwrap();
    let client = Client::new(mock.url(), "test-key");
    let options = AgentOptions {
        goal: None,
        model: "test-model".to_string(),
        identity: None,
        max_turns: 5,
        ui_tx: None,
        lane_specs: None,
        calib: Some(stream.clone()),
        skills: Vec::new(),
        role_overlay: None,
        memory: None,
        scope: None,
        improve_home: None,
        route_home: None,
        gui_prompt: None,
        compact_now: false,
        checkpoints: None,
        file_access: FileAccess::Unrestricted,
        plan_mode: false,
    };
    let outcome = {
        let mut progress = Vec::new();
        let mut agent = Agent::new(
            &client,
            options,
            &mut session,
            Box::new(ApproveAll),
            &mut progress,
        )
        .unwrap();
        agent.run("do the thing").await
    };
    assert!(outcome.is_err());
    drop(stream);

    let day = fleet::calib::today();
    let rows = calib_rows(&calib_root, &day);
    let turns = rows_of(&rows, "turn");
    assert_eq!(turns.len(), 1, "{turns:?}");
    assert_eq!(turns[0].outcome, "failed");
    assert_eq!(turns[0].stop_reason, "failed");
    // The failure class is the router's, so the row joins to the route
    // row's own attempt classes.
    let text = std::fs::read_to_string(fleet::calib::day_path(&calib_root, &day)).unwrap();
    assert!(text.contains("\"reason\":\"server\""), "{text}");
    std::fs::remove_dir_all(&home).ok();
}

/// The turn cap is a partial ending: the run is graded ok, and the ending
/// is named `max-turns` exactly as the session log names it.
#[tokio::test]
async fn a_turn_cap_records_max_turns_as_a_partial_ending() {
    let mock = MockApi::start(vec![tool_call_stream(
        "call-1",
        "list_dir",
        "{\"path\":\".\"}",
    )]);
    let home = temp_home("calib-cap");
    let calib_root = home.join("calib");
    let stream = fleet::calib::Stream::open(&calib_root);
    let mut session = Session::create(&home, "test-model").unwrap();
    let client = Client::new(mock.url(), "test-key");
    let options = AgentOptions {
        goal: None,
        model: "test-model".to_string(),
        identity: None,
        max_turns: 1,
        ui_tx: None,
        lane_specs: None,
        calib: Some(stream.clone()),
        skills: Vec::new(),
        role_overlay: None,
        memory: None,
        scope: None,
        improve_home: None,
        route_home: None,
        gui_prompt: None,
        compact_now: false,
        checkpoints: None,
        file_access: FileAccess::Unrestricted,
        plan_mode: false,
    };
    let report = {
        let mut progress = Vec::new();
        let mut agent = Agent::new(
            &client,
            options,
            &mut session,
            Box::new(ApproveAll),
            &mut progress,
        )
        .unwrap();
        agent.run("keep going").await.unwrap()
    };
    assert_eq!(report.stop_reason, StopReason::MaxTurns);
    drop(stream);

    let day = fleet::calib::today();
    let rows = calib_rows(&calib_root, &day);
    let turns = rows_of(&rows, "turn");
    assert_eq!(turns.len(), 1, "{turns:?}");
    assert_eq!(turns[0].outcome, "ok");
    assert_eq!(turns[0].stop_reason, "max-turns");
    assert_eq!(turns[0].turns, 1);
    std::fs::remove_dir_all(&home).ok();
}

/// A run that never touches the stream records nothing: the calibration
/// path is opt-in per surface, and the silent path stays silent.
#[tokio::test]
async fn a_run_without_a_stream_records_nothing() {
    let mock = MockApi::start(vec![assistant_stream("Done.")]);
    let home = temp_home("calib-silent");
    let calib_root = home.join("calib");
    std::fs::create_dir_all(&calib_root).unwrap();
    let mut session = Session::create(&home, "test-model").unwrap();
    let client = Client::new(mock.url(), "test-key");
    let mut progress = Vec::new();
    let mut agent = Agent::new(
        &client,
        AgentOptions {
            goal: None,
            model: "test-model".to_string(),
            identity: None,
            max_turns: 5,
            ui_tx: None,
            lane_specs: None,
            calib: None,
            skills: Vec::new(),
            role_overlay: None,
            memory: None,
            scope: None,
            improve_home: None,
            route_home: None,
            gui_prompt: None,
            compact_now: false,
            checkpoints: None,
            file_access: FileAccess::Unrestricted,
            plan_mode: false,
        },
        &mut session,
        Box::new(ApproveAll),
        &mut progress,
    )
    .unwrap();
    agent.run("say done").await.unwrap();
    assert!(calib_rows(&calib_root, &fleet::calib::today()).is_empty());
    std::fs::remove_dir_all(&home).ok();
}

/// The stream's first reader: a candidate with three consecutive failed
/// attempts in today's rows is demoted to the END of this session's
/// failover walk, while sys1's judgment still chooses the provider.
#[tokio::test]
async fn a_graded_failure_moves_the_failed_provider_down_its_chain() {
    // Four mocks, one per roster candidate that stays enabled.
    let stand_in = MockApi::start(vec![assistant_stream("Answered by the stand-in.")]);
    let zai = MockApi::start(vec![ScriptedResponse {
        status: "500 Internal Server Error".to_string(),
        body: r#"{"error":{"message":"zai is down"}}"#.to_string(),
        sse: Vec::new(),
        delay_ms: 0,
    }]);
    let mimo = MockApi::start(vec![ScriptedResponse {
        status: "500 Internal Server Error".to_string(),
        body: r#"{"error":{"message":"mimo is down"}}"#.to_string(),
        sse: Vec::new(),
        delay_ms: 0,
    }]);
    let stepfun = MockApi::start(vec![assistant_stream("Answered by stepfun.")]);

    let home = temp_home("calib-demote");
    let calib_root = home.join("calib");
    // The stream's root is the config override, which is also the path the
    // demotion reader takes.
    std::fs::write(
        home.join("config.json"),
        serde_json::json!({
            "providers": [
                { "id": "stand-in", "base_url": stand_in.url(), "key_env": "STAND_IN_API_KEY",
                  "models": ["auto"], "key_required": false },
                { "id": "zai-coding-plan", "base_url": zai.url(), "key_required": false },
                { "id": "xiaomi-mimo", "base_url": mimo.url(), "key_required": false },
                { "id": "stepfun", "base_url": stepfun.url(), "key_required": false },
                { "id": "token-plan", "enabled": false },
                { "id": "deepseek", "enabled": false }
            ],
            "calib": { "log_root": calib_root.to_string_lossy() }
        })
        .to_string(),
    )
    .unwrap();
    // The plugin-shaped routing table this scenario declares for
    // itself: `hard` walks zai, then the stand-in, then mimo, then stepfun.
    // Declared rather than borrowed from the compiled-in default so
    // the scenario proves the mechanics — the judgment's selection is
    // respected, the demotion reorders the walk — whatever the owner's
    // default table prefers. The stand-in sits mid-list on purpose: the
    // demotion must move it out of the walk's way, and without the
    // demotion the walk would reach it (its mock answers) before
    // stepfun.
    std::fs::write(
        home.join("routing.json"),
        serde_json::json!({
            "workloads": [
                { "workload": "hard", "candidates": [
                    "zai-coding-plan/GLM-5.3-Flash",
                    "stand-in/auto",
                    "xiaomi-mimo/mimo-v2.6-flash",
                    "stepfun/step-5-preview"
                ] }
            ]
        })
        .to_string(),
    )
    .unwrap();

    // Three graded failures against the stand-in, in the stream's own
    // vocabulary: three route rows whose walk failed at the stand-in.
    let day = fleet::calib::today();
    let path = fleet::calib::day_path(&calib_root, &day);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut scripted = String::new();
    for _ in 0..3 {
        scripted.push_str(
            &serde_json::json!({
                "ts": "2026-10-13T12:00:00Z",
                "op": "route",
                "provider": "stand-in",
                "how": "fallback",
                "reason": "sys1 gave no usable verdict",
                "model": "auto",
                "providers_used": ["stand-in"],
                "attempts": [{ "provider": "stand-in", "outcome": "failed", "class": "auth" }],
                "input_chars": 9,
                "input_hash": "0000000000000000",
                "latency_ms": 1
            })
            .to_string(),
        );
        scripted.push('\n');
    }
    std::fs::write(&path, scripted).unwrap();

    // sys1 judges `hard`, whose declared table names zai first: the
    // selection is the judgment's, and the demotion must not touch it.
    let fake = FakeMcp::start(vec![
        serde_json::json!({ "reply": initialize_reply() }),
        serde_json::json!({ "reply": {
            "jsonrpc": "2.0", "id": "$echo",
            "result": { "content": [{ "type": "text",
                "text": r#"{"verdict":"pass","answers":{"glide":{"workload":{"label":"hard","confidence":0.91}}}}"# }],
                "isError": false }
        }}),
    ]);
    let mut specs = std::collections::HashMap::new();
    specs.insert("sys1".to_string(), fake.spec());

    let dotenv = std::collections::HashMap::new();
    let prepared = fleet::route::prepare(&home, &dotenv, "refactor the crate", &specs)
        .await
        .unwrap();
    assert_eq!(prepared.decision.provider, "zai-coding-plan");
    assert_eq!(prepared.decision.how, How::Judged);
    // The demotion is auditable: it rides in the decision row's reason.
    assert!(
        prepared.decision.reason.contains("demoted stand-in"),
        "{}",
        prepared.decision.reason
    );

    // The walk: zai fails, mimo fails, stepfun serves — and the demoted
    // stand-in is never tried, because it now sits at the end of the chain.
    let mut session = Session::create(&home, "test-model").unwrap();
    let client = Client::routed(prepared.router.clone());
    let options = AgentOptions {
        goal: None,
        model: "test-model".to_string(),
        identity: None,
        max_turns: 5,
        ui_tx: None,
        lane_specs: None,
        calib: Some(prepared.calib.clone()),
        skills: Vec::new(),
        role_overlay: None,
        memory: None,
        scope: None,
        improve_home: None,
        route_home: None,
        gui_prompt: None,
        compact_now: false,
        checkpoints: None,
        file_access: FileAccess::Unrestricted,
        plan_mode: false,
    };
    let report = {
        let mut progress = Vec::new();
        let mut agent = Agent::new(
            &client,
            options,
            &mut session,
            Box::new(ApproveAll),
            &mut progress,
        )
        .unwrap();
        agent.run("refactor the crate").await.unwrap()
    };
    assert_eq!(report.stop_reason, StopReason::Completed);
    assert_eq!(
        report.final_message.as_deref(),
        Some("Answered by stepfun.")
    );
    assert_eq!(
        stand_in.bodies().len(),
        0,
        "the demoted candidate was never tried"
    );
    assert_eq!(zai.bodies().len(), 1);
    assert_eq!(mimo.bodies().len(), 1);
    assert_eq!(stepfun.bodies().len(), 1);

    let row = prepared.router.row("refactor the crate", 0);
    let attempts: Vec<(&str, &str)> = row
        .attempts
        .iter()
        .map(|attempt| (attempt.provider.as_str(), attempt.outcome))
        .collect();
    assert_eq!(
        attempts,
        vec![
            ("zai-coding-plan", "failed"),
            ("xiaomi-mimo", "failed"),
            ("stepfun", "served")
        ]
    );

    // The session's own route row lands in the stream beside the graded
    // ones that reordered it.
    prepared.record_route("refactor the crate");
    drop(prepared);
    let rows = calib_rows(&calib_root, &day);
    assert_eq!(
        rows_of(&rows, "route").len(),
        4,
        "three graded rows plus this session's"
    );
    std::fs::remove_dir_all(&home).ok();
}

// --- skills (slice 7) ------------------------------------------------------

/// The canonical skills library under a scratch home: the path the
/// tests hand to `FLEET_SKILLS_DIR` and to `skills::load`.
fn skills_canon(home: &Path) -> PathBuf {
    home.join("canonical")
}

/// A scratch FLEET_HOME carrying one live skill.
fn skills_home(tag: &str, name: &str, description: &str, body: &str) -> PathBuf {
    let home = temp_home(tag);
    let dir = skills_canon(&home).join(name);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("SKILL.md"),
        format!("---\nname: {name}\ndescription: {description}\n---\n\n{body}\n"),
    )
    .unwrap();
    home
}

/// Write one live skill into an existing home.
fn write_skill(home: &Path, name: &str, description: &str) {
    let dir = skills_canon(home).join(name);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("SKILL.md"),
        format!("---\nname: {name}\ndescription: {description}\n---\n\n# {name}\n"),
    )
    .unwrap();
}

/// One full agent run that declares `skills`.
async fn run_with_skills(
    mock: &MockApi,
    session: &mut Session,
    skills: Vec<fleet::skills::Skill>,
    task: &str,
) -> Result<RunReport, AgentError> {
    let client = Client::new(mock.url(), "test-key");
    let options = AgentOptions {
        goal: None,
        model: "test-model".to_string(),
        identity: None,
        max_turns: 5,
        ui_tx: None,
        lane_specs: None,
        calib: None,
        skills,
        role_overlay: None,
        memory: None,
        scope: None,
        improve_home: None,
        route_home: None,
        gui_prompt: None,
        compact_now: false,
        checkpoints: None,
        file_access: FileAccess::Unrestricted,
        plan_mode: false,
    };
    let mut progress = Vec::new();
    let mut agent = Agent::new(
        &client,
        options,
        session,
        Box::new(ApproveAll),
        &mut progress,
    )
    .unwrap();
    agent.run(task).await
}

/// One assistant record requesting the tool calls `calls` describes.
fn assistant_calls(calls: Vec<(&str, &str, &str)>) -> Record {
    Record::Assistant {
        content: None,
        tool_calls: calls
            .into_iter()
            .map(|(id, name, arguments)| {
                serde_json::from_value::<ToolCall>(tool_call(id, name, arguments)).unwrap()
            })
            .collect(),
        persona: None,
    }
}

/// The headline of the slice: a full run whose turn 1 selects a skill.
/// The description rides the system prompt, the body rides back as the
/// call's ordinary tool result, and the `skills` record is what keeps
/// that prompt reconstructable from the log.
#[tokio::test]
async fn a_selected_skills_body_rides_the_tool_result_path() {
    let home = skills_home(
        "skill-run",
        "demo-skill",
        "Recover a failed bash call",
        "# demo\n\nread the error, fix one thing, rerun",
    );
    let mock = MockApi::start(vec![
        tool_call_stream("call-1", "use_skill", r#"{"name":"demo-skill"}"#),
        assistant_stream("The skill is loaded; I will read the error first."),
    ]);
    let mut session = Session::create(&home, "test-model").unwrap();
    let report = run_with_skills(
        &mock,
        &mut session,
        fleet::skills::load(&skills_canon(&home)),
        "recover from it",
    )
    .await
    .unwrap();
    assert_eq!(report.stop_reason, StopReason::Completed);

    let bodies = mock.bodies();
    assert_eq!(bodies.len(), 2);
    // The first request's system prompt carries the name and the
    // description and nothing of the body: description first, body on
    // demand.
    let system = bodies[0]["messages"][0]["content"]
        .as_str()
        .expect("the system prompt");
    assert!(system.contains("demo-skill"), "{system}");
    assert!(
        system.contains("- `demo-skill` \u{2014} use when recover a failed bash call"),
        "{system}"
    );
    assert!(
        !system.contains("read the error, fix one thing"),
        "{system}"
    );
    // The declarations carry `use_skill` beside the four built-ins, the
    // delegation tool beside that, and the convention lookup after it.
    let declared = declared_tools(&bodies[0]);
    assert_eq!(declared.len(), 29, "{declared:?}");
    assert!(declared.contains(&"use_skill".to_string()), "{declared:?}");
    assert!(declared.contains(&"subagent".to_string()), "{declared:?}");
    assert!(declared.contains(&"convention".to_string()), "{declared:?}");
    // The body entered context as the call's ordinary tool result — the
    // same seam, the same message the model reads.
    let seen = bodies[1]["messages"][3]["content"]
        .as_str()
        .expect("the tool result");
    assert_eq!(seen, "# demo\n\nread the error, fix one thing, rerun");

    // One `skills` record, written once at run start, and it replays to
    // no message: the prompt the model saw stays reconstructable.
    let records = session::replay(session.path()).unwrap();
    let declared_records: Vec<&Record> = records
        .iter()
        .filter(|record| matches!(record, Record::Skills { .. }))
        .collect();
    assert_eq!(declared_records.len(), 1, "{records:?}");
    match declared_records[0] {
        Record::Skills { skills } => {
            assert_eq!(skills.len(), 1);
            assert_eq!(skills[0].name, "demo-skill");
            assert_eq!(skills[0].description, "Recover a failed bash call");
        }
        other => panic!("expected a skills record, got {other:?}"),
    }
    // The body is in the log exactly once — as the tool result — and the
    // messages the log rebuilds are the four the model saw.
    assert_eq!(session::messages(&records).len(), 4, "{records:?}");
    match first_result(&records) {
        Record::ToolResult {
            name,
            output,
            approved: true,
            ..
        } => {
            assert_eq!(name, "use_skill");
            assert_eq!(output, "# demo\n\nread the error, fix one thing, rerun");
        }
        other => panic!("expected the use_skill tool result, got {other:?}"),
    }
    std::fs::remove_dir_all(&home).ok();
}

/// An unknown skill is a typed error result the model can read and react
/// to, never a panic and never a silent empty answer.
#[tokio::test]
async fn an_unknown_skill_is_a_typed_error_result() {
    let home = skills_home("skill-unknown", "demo-skill", "d", "body");
    let mock = MockApi::start(vec![
        tool_call_stream("call-1", "use_skill", r#"{"name":"no-such-skill"}"#),
        assistant_stream("That skill does not exist; carrying on."),
    ]);
    let mut session = Session::create(&home, "test-model").unwrap();
    let report = run_with_skills(
        &mock,
        &mut session,
        fleet::skills::load(&skills_canon(&home)),
        "try it",
    )
    .await
    .unwrap();
    assert_eq!(report.stop_reason, StopReason::Completed);

    let records = session::replay(session.path()).unwrap();
    match first_result(&records) {
        Record::ToolResult { name, output, .. } => {
            assert_eq!(name, "use_skill");
            assert_eq!(output, "error: no skill named `no-such-skill`");
        }
        other => panic!("expected a typed error result, got {other:?}"),
    }
    // The model read the typed error in the next request.
    let bodies = mock.bodies();
    let seen = bodies[1]["messages"][3]["content"]
        .as_str()
        .expect("the tool result");
    assert_eq!(seen, "error: no skill named `no-such-skill`");
    std::fs::remove_dir_all(&home).ok();
}

/// The self-authoring half: a session whose `bash` call failed and was
/// then corrected drafts a creation proposal with the records cited —
/// and the live tree is never the drafter's target.
#[tokio::test]
async fn a_sessions_failure_then_correction_drafts_a_proposal() {
    let home = skills_home(
        "draft",
        "unrelated",
        "a skill that is not the drafted one",
        "body",
    );
    let mut session = Session::create(&home, "test-model").unwrap();
    session
        .record(&Record::User {
            content: "run it".to_string(),
        })
        .unwrap();
    session
        .record(&assistant_calls(vec![(
            "call-1",
            "bash",
            r#"{"command":"ls"}"#,
        )]))
        .unwrap();
    session
        .record(&Record::ToolResult {
            call_id: "call-1".to_string(),
            name: "bash".to_string(),
            output: "error: exit 127".to_string(),
            truncated: false,
            approved: true,
        })
        .unwrap();
    session
        .record(&assistant_calls(vec![(
            "call-2",
            "bash",
            r#"{"command":"ls -la"}"#,
        )]))
        .unwrap();
    session
        .record(&Record::ToolResult {
            call_id: "call-2".to_string(),
            name: "bash".to_string(),
            output: "exit 0".to_string(),
            truncated: false,
            approved: true,
        })
        .unwrap();
    session
        .record(&Record::Assistant {
            content: Some("done".to_string()),
            tool_calls: vec![],
            persona: None,
        })
        .unwrap();

    let filed = fleet::skills::draft_session(&skills_canon(&home), &home, session.path(), None);
    assert_eq!(filed.len(), 1, "{filed:?}");
    let draft = std::fs::read_to_string(&filed[0]).unwrap();
    // The frontmatter carries the proposal flag, the kind, and the
    // evidence — the session id and both records.
    assert!(draft.starts_with("---\n"), "{draft}");
    assert!(draft.contains("name: bash-recovery"), "{draft}");
    assert!(draft.contains("proposed: true"), "{draft}");
    assert!(draft.contains("kind: create"), "{draft}");
    assert!(
        draft.contains(&format!("session {}", session.id())),
        "{draft}"
    );
    assert!(draft.contains("record 4"), "{draft}");
    assert!(draft.contains("record 6"), "{draft}");
    // The draft lives under proposals/ and the live tree is untouched:
    // the one skill it held, and no `bash-recovery` directory beside it.
    assert!(
        filed[0].starts_with(fleet::skills::proposals_dir(&home)),
        "{filed:?}"
    );
    let live = fleet::skills::load(&skills_canon(&home));
    assert_eq!(live.len(), 1, "{live:?}");
    assert_eq!(live[0].name, "unrelated");
    assert!(!skills_canon(&home).join("bash-recovery").exists());
    // The proposal loads back through the loader, ready for the gate.
    let proposals = fleet::skills::load_proposals(&home);
    assert_eq!(proposals.len(), 1);
    assert_eq!(proposals[0].kind, fleet::skills::ProposalKind::Create);
    assert_eq!(proposals[0].name, "bash-recovery");
    std::fs::remove_dir_all(&home).ok();
}

/// The usage-scan curation, end to end: the window from config, a used
/// skill, an unused one, and a near-identical pair — one retire draft,
/// one merge draft, and a live tree nothing touched.
#[tokio::test]
async fn the_usage_scan_curation_merges_and_retires_nothing_from_the_canonical_library() {
    let home = temp_home("curate");
    write_skill(
        &home,
        "alpha",
        "probe the embeddings worker for reachability",
    );
    write_skill(
        &home,
        "beta",
        "probe the embeddings worker for reachability now",
    );
    write_skill(&home, "gamma", "write the weekly release notes");
    write_skill(&home, "delta", "run the release checklist");
    // The window is two sessions and the merge threshold is the default.
    std::fs::write(
        home.join("config.json"),
        r#"{"skills":{"retire_after_sessions":2,"merge_similarity":0.8}}"#,
    )
    .unwrap();
    let config = fleet::route::skills_config(&home);
    assert_eq!(config.retire_after_sessions, 2);

    // Two sessions of history — the whole window. One selects alpha and
    // delta; gamma is never selected and beta only exists as alpha's twin.
    let mut used = Session::create(&home, "test-model").unwrap();
    used.record(&Record::User {
        content: "t".to_string(),
    })
    .unwrap();
    used.record(&assistant_calls(vec![
        ("call-1", "use_skill", r#"{"name":"alpha"}"#),
        ("call-2", "use_skill", r#"{"name":"delta"}"#),
    ]))
    .unwrap();
    used.record(&Record::ToolResult {
        call_id: "call-1".to_string(),
        name: "use_skill".to_string(),
        output: "# alpha".to_string(),
        truncated: false,
        approved: true,
    })
    .unwrap();
    used.record(&Record::ToolResult {
        call_id: "call-2".to_string(),
        name: "use_skill".to_string(),
        output: "# delta".to_string(),
        truncated: false,
        approved: true,
    })
    .unwrap();
    let mut idle = Session::create(&home, "test-model").unwrap();
    idle.record(&Record::User {
        content: "t2".to_string(),
    })
    .unwrap();

    let filed = fleet::skills::curate(
        &skills_canon(&home),
        &home,
        config.retire_after_sessions,
        config.merge_similarity,
    );
    let names: Vec<String> = filed
        .iter()
        .map(|path| path.file_stem().unwrap().to_string_lossy().into_owned())
        .collect();
    // KI-054: the merge draft is named by the pair, not the survivor.
    assert!(names.contains(&"merge-beta-alpha".to_string()), "{names:?}");
    // The retirement rule is a no-op against the canonical library (the
    // usage scan reads fleet's sessions only, KI-215): gamma was never
    // selected and nothing is filed against it. beta is the absorbed
    // half of the merge draft; delta was used.
    assert!(!names.contains(&"gamma".to_string()), "{names:?}");
    assert!(!names.contains(&"beta".to_string()), "{names:?}");
    assert!(!names.contains(&"delta".to_string()), "{names:?}");

    let proposals = fleet::skills::load_proposals(&home);
    let merge = proposals
        .iter()
        .find(|proposal| proposal.name == "merge-beta-alpha")
        .expect("the merge");
    assert_eq!(merge.kind, fleet::skills::ProposalKind::Merge);
    assert!(merge.evidence.contains("0.86"), "{}", merge.evidence);
    assert!(merge.evidence.contains("`beta`"), "{}", merge.evidence);

    // The live tree is exactly what it was.
    let live = fleet::skills::load(&skills_canon(&home));
    let live_names: Vec<&str> = live.iter().map(|skill| skill.name.as_str()).collect();
    assert_eq!(live_names, vec!["alpha", "beta", "delta", "gamma"]);
    std::fs::remove_dir_all(&home).ok();
}

/// `fleet skills` end to end against the real binary: the live skills as
/// cards, the drafts awaiting the gate, and one draft printed verbatim.
#[test]
fn the_skills_command_lists_cards_proposals_and_a_draft() {
    let home = skills_home(
        "skills-cmd",
        "demo-skill",
        "Recover a failed bash call",
        "# demo",
    );
    let fleet = env!("CARGO_BIN_EXE_fleet");
    let run = |args: &[&str]| {
        std::process::Command::new(fleet)
            .args(args)
            .env("FLEET_HOME", &home)
            .env("FLEET_SKILLS_DIR", skills_canon(&home))
            .output()
            .expect("run the fleet binary")
    };

    let listed = run(&["skills"]);
    assert!(listed.status.success(), "{listed:?}");
    let stdout = String::from_utf8_lossy(&listed.stdout);
    assert!(stdout.contains("fleet skills"), "{stdout}");
    assert!(stdout.contains("demo-skill"), "{stdout}");
    assert!(stdout.contains("Recover a failed bash call"), "{stdout}");
    // Cards, not a table.
    assert!(stdout.contains("\u{250c}\u{2500}"), "{stdout}");
    assert!(!stdout.contains("| id |"), "{stdout}");

    // No drafts yet says so rather than rendering nothing.
    let empty_proposed = run(&["skills", "--proposed"]);
    assert!(empty_proposed.status.success());
    assert!(String::from_utf8_lossy(&empty_proposed.stdout).contains("no drafts yet"));

    // A `--show` naming a draft that does not exist is a named failure.
    let missing = run(&["skills", "--show", "nope"]);
    assert_eq!(missing.status.code(), Some(1), "{missing:?}");

    // File one draft through the crate's own drafter, then read it back
    // through both surfaces.
    let mut session = Session::create(&home, "test-model").unwrap();
    session
        .record(&Record::User {
            content: "run it".to_string(),
        })
        .unwrap();
    session
        .record(&assistant_calls(vec![(
            "call-1",
            "bash",
            r#"{"command":"ls"}"#,
        )]))
        .unwrap();
    session
        .record(&Record::ToolResult {
            call_id: "call-1".to_string(),
            name: "bash".to_string(),
            output: "error: exit 127".to_string(),
            truncated: false,
            approved: true,
        })
        .unwrap();
    session
        .record(&assistant_calls(vec![(
            "call-2",
            "bash",
            r#"{"command":"ls -la"}"#,
        )]))
        .unwrap();
    session
        .record(&Record::ToolResult {
            call_id: "call-2".to_string(),
            name: "bash".to_string(),
            output: "exit 0".to_string(),
            truncated: false,
            approved: true,
        })
        .unwrap();
    let filed = fleet::skills::draft_session(&skills_canon(&home), &home, session.path(), None);
    assert_eq!(filed.len(), 1, "{filed:?}");

    let shown = run(&["skills", "--show", "bash-recovery"]);
    assert!(shown.status.success(), "{shown:?}");
    let shown_out = String::from_utf8_lossy(&shown.stdout);
    assert!(shown_out.contains("proposed: true"), "{shown_out}");
    assert!(shown_out.contains("# bash-recovery"), "{shown_out}");

    let proposed = run(&["skills", "--proposed"]);
    assert!(proposed.status.success());
    let proposed_out = String::from_utf8_lossy(&proposed.stdout);
    assert!(proposed_out.contains("bash-recovery"), "{proposed_out}");
    assert!(proposed_out.contains("create"), "{proposed_out}");
    assert!(
        proposed_out.contains("awaiting the slice-12 gate"),
        "{proposed_out}"
    );
    // The gate boundary is named on the surface: nothing here applies.
    assert!(proposed_out.contains("slice 12"), "{proposed_out}");

    // A library with no skills directory says so and still exits 0.
    // The seam is redirected explicitly: a bare FLEET_HOME would resolve
    // to the machine's real `~/.agents/skills`, and a test never reads
    // the owner's library.
    let bare = temp_home("skills-bare");
    let listed_bare = std::process::Command::new(fleet)
        .arg("skills")
        .env("FLEET_HOME", &bare)
        .env("FLEET_SKILLS_DIR", bare.join("absent-library"))
        .output()
        .expect("run the fleet binary");
    assert!(listed_bare.status.success());
    assert!(String::from_utf8_lossy(&listed_bare.stdout).contains("no skills yet"));

    std::fs::remove_dir_all(&home).ok();
    std::fs::remove_dir_all(&bare).ok();
}

// --- memory (slice 8) ------------------------------------------------------

/// A scratch corpus under a fresh working directory: two protected
/// facts, three ranked facts above the floor, one below it, and one
/// malformed file the loader must skip by name. Returns the directory
/// the binary runs from — `fleet memories` resolves the nearest
/// `memories/` at or above the working directory.
///
/// The returned root is a FLEET_HOME: every child the tests spawn is
/// given it as `FLEET_HOME`, so the resolution's home-first rule lands
/// on this corpus and nothing else. Without that, the rule prefers the
/// ambient `~/.fleet/memories` the moment it holds any `.md` — a
/// consolidator's summary, a `remember` write, the owner's own fleet
/// process — and the test silently reads that corpus instead of its
/// fixture (KI-227: three of these tests read 0 facts that way under
/// the full suite's parallel load, and passed in isolation).
fn memories_corpus(tag: &str) -> PathBuf {
    let root = temp_home(tag);
    let memories = root.join("memories");
    std::fs::create_dir_all(&memories).unwrap();
    let fact = |name: &str, description: &str, fact_type: &str, shape: &str| {
        std::fs::write(
            memories.join(format!("{name}.md")),
            format!(
                "---\nname: {name}\ndescription: \"{description}\"\ntype: {fact_type}\ndate: 2026-10-13\nbasis: confirmed-by\nshape: [{shape}]\nrecall-count: 0\nsuperseded-by:\nvalid-until:\n---\n\nThe full content of {name}.\n"
            ),
        )
        .unwrap();
    };
    fact(
        "identity",
        "who the owner is and how he works, in a standing fact",
        "user",
        "",
    );
    fact(
        "rule",
        "a standing rule the owner stated about documentation",
        "feedback",
        "docs",
    );
    fact(
        "top",
        "the railway deploy is failing per branch",
        "project",
        "deploy",
    );
    fact(
        "middle",
        "the railway deploy waits for the webhook",
        "project",
        "deploy",
    );
    fact(
        "bottom",
        "the railway deploy logs its outcome",
        "project",
        "deploy",
    );
    fact("junk", "the walnut table texture", "project", "");
    std::fs::write(memories.join("broken.md"), "# no fence here\n").unwrap();
    root
}

/// `fleet memories` end to end against the real binary: the facts as
/// cards, the protected and retired tallies, and the backend the probe
/// found — named either way, never silent.
/// KI-227: the resolution's home-first rule prefers the fleet home's
/// own corpus the moment it holds any `.md`, so a test that reads its
/// fixture through the ambient home silently reads whatever else is
/// there — a consolidator's summary, a `remember` write, the owner's
/// own fleet process. The fixture pins FLEET_HOME to its own home, so
/// a competing ambient corpus cannot shadow it: this test writes one
/// into a scratch `~/.fleet` and the fixture still reads its own.
#[test]
fn a_competing_home_corpus_does_not_shadow_the_fixture() {
    let root = memories_corpus("memories-shadow");
    // The ambient home, with a corpus of its own that would win the
    // home-first rule if the child did not carry FLEET_HOME: the home
    // is this directory, so its corpus is `<home>/.fleet/memories`.
    let ambient = temp_home("memories-ambient");
    let corpus = ambient.join(".fleet").join("memories");
    std::fs::create_dir_all(&corpus).unwrap();
    std::fs::write(
        corpus.join("an-ambient-fact.md"),
        "---\nname: an-ambient-fact\ndescription: \"the ambient corpus\"\ntype: project\ndate: 2026-10-13\nbasis: inferred\nshape: []\nrecall-count: 0\nsuperseded-by:\nvalid-until:\n---\n\nThe ambient corpus's fact.\n",
    )
    .unwrap();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_fleet"))
        .env("FLEET_HOME", &root)
        .env("HOME", &ambient)
        .arg("memories")
        .current_dir(&root)
        .output()
        .expect("run the fleet binary");
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("6 facts \u{b7} 2 protected \u{b7} 0 retired"),
        "the fixture's own corpus is read, not the ambient one: {stdout}"
    );
    assert!(!stdout.contains("an-ambient-fact"), "{stdout}");
    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(&ambient);
}

#[test]
fn the_memories_command_cards_the_facts_and_names_the_backend() {
    let root = memories_corpus("memories-cmd");
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_fleet"))
        .env("FLEET_HOME", &root)
        .arg("memories")
        .current_dir(&root)
        .output()
        .expect("run the fleet binary");
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Cards, never a table: one per live fact, the malformed one skipped.
    assert!(stdout.contains("fleet memories"), "{stdout}");
    assert_eq!(stdout.matches('\u{250c}').count(), 6, "{stdout}");
    assert!(
        stdout.contains("6 facts \u{b7} 2 protected \u{b7} 0 retired"),
        "{stdout}"
    );
    // Every card field the surface promises: type, basis, shape,
    // recall-count, valid-until.
    assert!(stdout.contains("project \u{b7} confirmed-by"), "{stdout}");
    assert!(
        stdout.contains("shape: deploy \u{b7} recalls: 0 \u{b7} valid-until: \u{2014}"),
        "{stdout}"
    );
    // The backend is named where an operator looks — which of the two
    // paths serves is the machine's, the naming is the contract.
    assert!(
        stdout.contains("backend: vector") || stdout.contains("backend: keyword-only"),
        "{stdout}"
    );
    // The malformed fact is skipped by name, on stderr, once.
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("skipping `broken`"), "{stderr}");
    // The generated index is gitignored-style cache, not a fact.
    assert!(root.join("memories/.recall-index.json").exists() || stdout.contains("keyword-only"));
    std::fs::remove_dir_all(&root).ok();
}

/// `fleet memories --recall` end to end: the ranked assembly for one
/// instruction, the protected tier in the block, and the dry run that
/// changes nothing on disk.
#[test]
fn the_memories_recall_flag_prints_the_ranked_assembly() {
    let root = memories_corpus("memories-recall");
    let before: Vec<u8> = {
        let mut files: Vec<PathBuf> = std::fs::read_dir(root.join("memories"))
            .unwrap()
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("md"))
            .collect();
        files.sort();
        files
            .iter()
            .flat_map(|path| std::fs::read(path).unwrap())
            .collect()
    };
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_fleet"))
        .env("FLEET_HOME", &root)
        .args(["memories", "--recall", "the railway deploy is failing"])
        .current_dir(&root)
        .output()
        .expect("run the fleet binary");
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    // The shape the instruction routed under, and the fallback named
    // when nothing classifies.
    assert!(stdout.contains("shape: deploy"), "{stdout}");
    // The protected tier enters whatever the budget says.
    assert!(stdout.contains("Standing context (protected"), "{stdout}");
    assert!(stdout.contains("[user confirmed-by]"), "{stdout}");
    assert!(stdout.contains("[feedback confirmed-by]"), "{stdout}");
    // The recall section names its shape, and the dry run is honest.
    assert!(
        stdout.contains("shape-routed recall for `deploy`"),
        "{stdout}"
    );
    assert!(stdout.contains("a dry run"), "{stdout}");
    // Nothing on disk changed: printing is not a recall hit.
    let after: Vec<u8> = {
        let mut files: Vec<PathBuf> = std::fs::read_dir(root.join("memories"))
            .unwrap()
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("md"))
            .collect();
        files.sort();
        files
            .iter()
            .flat_map(|path| std::fs::read(path).unwrap())
            .collect()
    };
    assert_eq!(before, after);
    std::fs::remove_dir_all(&root).ok();
}

/// An unclassifiable instruction answers from the surface alone — the
/// documented fallback, named on the surface, not an error.
#[test]
fn the_memories_recall_flag_names_the_unclassified_fallback() {
    let root = memories_corpus("memories-fallback");
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_fleet"))
        .env("FLEET_HOME", &root)
        .args(["memories", "--recall", "tell me about the walnut table"])
        .current_dir(&root)
        .output()
        .expect("run the fleet binary");
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("shape: none \u{2014} surface recall (the documented fallback)"),
        "{stdout}"
    );
    std::fs::remove_dir_all(&root).ok();
}

/// `fleet memories --shape` filters the cards by structure shape, and a
/// shape nothing carries names the taxonomy rather than showing an
/// empty screen.
#[test]
fn the_memories_shape_flag_filters_the_cards() {
    let root = memories_corpus("memories-shape");
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_fleet"))
        .env("FLEET_HOME", &root)
        .args(["memories", "--shape", "deploy"])
        .current_dir(&root)
        .output()
        .expect("run the fleet binary");
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("filtered by shape: deploy"), "{stdout}");
    assert_eq!(stdout.matches('\u{250c}').count(), 3, "{stdout}");
    let empty = std::process::Command::new(env!("CARGO_BIN_EXE_fleet"))
        .env("FLEET_HOME", &root)
        .args(["memories", "--shape", "telemetry"])
        .current_dir(&root)
        .output()
        .expect("run the fleet binary");
    let empty_out = String::from_utf8_lossy(&empty.stdout);
    assert!(
        empty_out.contains("no facts carry shape `telemetry`"),
        "{empty_out}"
    );
    // The line is long enough to wrap, so the assertion is on the axes'
    // own vocabulary rather than on one unbroken string: every shape in
    // all three taxonomies is named somewhere on the surface.
    for shape in [
        "rendering",
        "state-sync",
        "deploy",
        "build",
        "change",
        "deps",
        "docs",
        "fail-open",
        "drops-the-row",
        "bound-exceeded",
        "claim-outran-evidence",
        "fixed-width-clip",
        "foreign-root",
        "incomplete",
        "resolved",
    ] {
        assert!(
            empty_out.contains(shape),
            "{shape} is not named: {empty_out}"
        );
    }
    std::fs::remove_dir_all(&root).ok();
}

/// `fleet doctor`'s recall line names which retrieval path is live —
/// the vector path with the endpoint and dimensionality when the local
/// model answers, the one named degradation line when it does not.
/// Either way the line is there: never silent.
#[test]
fn the_doctor_recall_line_names_the_live_path() {
    let root = memories_corpus("memories-doctor");
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_fleet"))
        .env("FLEET_HOME", &root)
        .arg("doctor")
        .current_dir(&root)
        .output()
        .expect("run the fleet binary");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let recall = stdout
        .lines()
        .find(|line| line.starts_with("recall"))
        .unwrap_or_else(|| panic!("doctor names no recall path: {stdout}"));
    assert!(
        recall.contains("vector path live") || recall.contains("embeddings unavailable"),
        "{recall}"
    );
    // The degradation, when it is the live answer, is the one named
    // line — not a silent absence.
    if recall.contains("embeddings unavailable") {
        assert!(recall.contains("recall is keyword-only"), "{recall}");
    }
    std::fs::remove_dir_all(&root).ok();
}

// --- live API --------------------------------------------------------------

#[tokio::test]
#[ignore = "real-API e2e; run with `cargo test --test integration -- --ignored` and DEEPSEEK_API_KEY set"]
// The ambient lock is a std mutex by design: it serializes the process's
// environment against the other tests for the whole awaited body.
#[allow(clippy::await_holding_lock)]
async fn live_api_completes_a_tool_task() {
    // The ambient-key test sets DEEPSEEK_API_KEY for its own duration, so
    // the read here serializes against it.
    let _ambient_lock = AMBIENT_LOCK.lock().unwrap();
    let Ok(key) = std::env::var("DEEPSEEK_API_KEY") else {
        eprintln!("DEEPSEEK_API_KEY is not set; skipping the live test");
        return;
    };
    let home = temp_home("live");
    let mut session = Session::create(&home, "deepseek-flash").unwrap();
    let client = Client::new(DEFAULT_BASE_URL, key);
    let mut progress = std::io::stderr().lock();
    let mut agent = Agent::new(
        &client,
        AgentOptions::default(),
        &mut session,
        Box::new(ApproveAll),
        &mut progress,
    )
    .unwrap();
    let report = agent
        .run("Run `echo fleet-live-ok` with bash, then reply with exactly its output.")
        .await
        .unwrap();
    assert_eq!(report.stop_reason, StopReason::Completed);
    assert!(
        report
            .final_message
            .as_deref()
            .unwrap_or_default()
            .contains("fleet-live-ok"),
        "{report:?}"
    );
    std::fs::remove_dir_all(&home).ok();
}

// --- memory: the write path, the turn wiring (slice 8, part 2) --------------

/// A scratch corpus root with `facts` written into it, unique per call.
fn memory_corpus(tag: &str, facts: &[(&str, String)]) -> PathBuf {
    let dir = temp_home(&format!("memory-{tag}"));
    for (name, text) in facts {
        std::fs::write(dir.join(format!("{name}.md")), text).unwrap();
    }
    dir
}

/// One durable fact in the writer's field order.
fn fact_file(name: &str, description: &str, basis: &str, fact_type: &str, shape: &str) -> String {
    format!(
        "---\nname: {name}\ndescription: \"{description}\"\ntype: {fact_type}\ndate: {today}\nbasis: {basis}\ntier: durable\nshape: [{shape}]\nrecall-count: 0\nsuperseded-by:\nvalid-until:\n---\n\nThe body of {name}.\n",
        today = fleet::calib::today()
    )
}

/// One full agent run with an optional corpus wired in.
async fn run_with_memory(
    mock: &MockApi,
    session: &mut Session,
    approver: Box<dyn Approver>,
    memory: Option<std::sync::Arc<fleet::memory::Memory>>,
    calib: Option<fleet::calib::Stream>,
    task: &str,
) -> Result<RunReport, AgentError> {
    let client = Client::new(mock.url(), "test-key");
    let options = AgentOptions {
        goal: None,
        model: "test-model".to_string(),
        identity: None,
        max_turns: 5,
        ui_tx: None,
        lane_specs: None,
        calib,
        skills: Vec::new(),
        role_overlay: None,
        memory,
        scope: None,
        improve_home: None,
        route_home: None,
        gui_prompt: None,
        compact_now: false,
        checkpoints: None,
        file_access: FileAccess::Unrestricted,
        plan_mode: false,
    };
    let mut progress = Vec::new();
    let mut agent = Agent::new(&client, options, session, approver, &mut progress).unwrap();
    agent.run(task).await
}

#[tokio::test]
async fn a_runs_request_carries_the_assembled_memory_block() {
    let mock = MockApi::start(vec![assistant_stream("Done.")]);
    let home = temp_home("memory-turn");
    let root = memory_corpus(
        "turn",
        &[
            (
                "identity",
                fact_file(
                    "identity",
                    "the owner's standing rule about the gate",
                    "confirmed-by",
                    "feedback",
                    "",
                ),
            ),
            (
                "top",
                fact_file(
                    "top",
                    "the railway deploy is failing per branch",
                    "inferred",
                    "project",
                    "deploy",
                ),
            ),
        ],
    );
    let calib_root = temp_home("memory-turn-calib");
    let stream = fleet::calib::Stream::open(&calib_root);
    let memory = std::sync::Arc::new(fleet::memory::Memory::load_with(
        &root,
        Box::new(fleet::memory::KeywordOnly),
    ));
    let mut session = Session::create(&home, "test-model").unwrap();
    run_with_memory(
        &mock,
        &mut session,
        Box::new(ApproveAll),
        Some(memory),
        Some(stream.clone()),
        "the railway deploy is failing",
    )
    .await
    .unwrap();

    // The request the model saw carries the assembled block: the
    // protected tier, then the shape-routed recall, in the frame that
    // says how to read it.
    let bodies = mock.bodies();
    let system = bodies[0]["messages"][0]["content"].as_str().unwrap();
    assert!(system.starts_with(fleet::agent::SYSTEM_PROMPT), "{system}");
    assert!(system.contains("## Memory"), "{system}");
    assert!(system.contains("Standing context (protected"), "{system}");
    assert!(
        system.contains("shape-routed recall for `deploy`"),
        "{system}"
    );
    assert!(
        system.contains("the railway deploy is failing per branch"),
        "{system}"
    );
    // The write path rides beside the closed four, and the delegation
    // tool beside that.
    assert_eq!(
        declared_tools(&bodies[0]),
        vec![
            "bash",
            "read_file",
            "write_file",
            "list_dir",
            "edit",
            "apply_patch",
            "grep",
            "glob",
            "repo_map",
            "file_change",
            "todo",
            "measure",
            "imagine",
            "browse",
            "recall",
            "mcp_resource",
            "mcp_prompt",
            "remember",
            "supersede",
            "subagent",
            "convention",
            "plan",
            "deck",
            "extract",
            "vision",
            "web",
            "read_document",
            "send_file",
            "computer_use",
            // The background handle: a command started
            // in the background is controllable only if the
            // control tool is declared beside it.
            "process"
        ]
    );

    // The `memory` record keeps the prompt reconstructable (hard rule
    // 5): the shape and every fact name, in block order.
    let records = session::replay(session.path()).unwrap();
    match records
        .iter()
        .find(|record| matches!(record, Record::Memory { .. }))
    {
        Some(Record::Memory { shape, facts }) => {
            assert_eq!(shape, "deploy");
            assert_eq!(facts, &vec!["identity".to_string(), "top".to_string()]);
        }
        other => panic!("expected a memory record, got {other:?}"),
    }

    // The recall hits: one row for the fact that entered, and none for
    // the protected tier, which did not compete for its place.
    drop(stream);
    let rows = fleet::calib::read_day(&calib_root, &fleet::calib::today(), 64);
    let hits: Vec<&str> = rows
        .iter()
        .filter(|row| row.op == fleet::calib::OP_RECALL_HIT)
        .map(|row| row.fact.as_str())
        .collect();
    assert_eq!(hits, vec!["top"]);
}

#[tokio::test]
async fn a_degenerate_stream_is_still_one_answer_and_the_hits_fire_once() {
    // The degenerate stream: a 200 that closes with `[DONE]` and no
    // chunks. The client assembles one empty choice from it, so the run
    // completes — this IS a successful first response, and the hits
    // fire for it exactly once, after it (KI-068's ordering, pinned
    // from the degenerate side). A run whose first call dies before any
    // response bumps nothing; that is the failed-call unit test in
    // agent.rs.
    let mock = MockApi::start(vec![stream(vec!["data: [DONE]".to_string()])]);
    let home = temp_home("memory-empty-answer");
    let root = memory_corpus(
        "empty-answer",
        &[
            (
                "identity",
                fact_file(
                    "identity",
                    "the owner's standing rule about the gate",
                    "confirmed-by",
                    "feedback",
                    "",
                ),
            ),
            (
                "top",
                fact_file(
                    "top",
                    "the railway deploy is failing per branch",
                    "inferred",
                    "project",
                    "deploy",
                ),
            ),
        ],
    );
    let calib_root = temp_home("memory-empty-answer-calib");
    let stream = fleet::calib::Stream::open(&calib_root);
    let memory = std::sync::Arc::new(fleet::memory::Memory::load_with(
        &root,
        Box::new(fleet::memory::KeywordOnly),
    ));
    let mut session = Session::create(&home, "test-model").unwrap();
    let report = run_with_memory(
        &mock,
        &mut session,
        Box::new(ApproveAll),
        Some(memory),
        Some(stream.clone()),
        "the railway deploy is failing",
    )
    .await
    .unwrap();
    assert_eq!(report.stop_reason, fleet::agent::StopReason::Completed);

    // The answer was empty, and the answer was real: the surfaced
    // fact's count moved exactly one step, and the stream carries the
    // one `recall_hit` row — never two, whatever the stream looked
    // like.
    let top = std::fs::read_to_string(root.join("top.md")).unwrap();
    assert!(top.contains("recall-count: 1"), "{top}");
    drop(stream);
    let rows = fleet::calib::read_day(&calib_root, &fleet::calib::today(), 64);
    let hits = rows
        .iter()
        .filter(|row| row.op == fleet::calib::OP_RECALL_HIT)
        .count();
    assert_eq!(hits, 1, "{rows:?}");
}

#[tokio::test]
async fn a_memory_less_run_is_byte_for_byte_unchanged() {
    let mock = MockApi::start(vec![assistant_stream("Done.")]);
    let home = temp_home("memory-less");
    let mut session = Session::create(&home, "test-model").unwrap();
    run_with_memory(
        &mock,
        &mut session,
        Box::new(ApproveAll),
        None,
        None,
        "say done",
    )
    .await
    .unwrap();

    // No memory wired: the system prompt is the compiled-in constant,
    // byte for byte, and the declarations are the closed four plus the
    // delegation tool.
    let bodies = mock.bodies();
    assert_eq!(
        bodies[0]["messages"][0]["content"],
        fleet::agent::default_prompt()
    );
    assert_eq!(
        declared_tools(&bodies[0]),
        vec![
            "bash",
            "read_file",
            "write_file",
            "list_dir",
            "edit",
            "apply_patch",
            "grep",
            "glob",
            "repo_map",
            "file_change",
            "todo",
            "measure",
            "imagine",
            "browse",
            "recall",
            "mcp_resource",
            "mcp_prompt",
            "subagent",
            "convention",
            "plan",
            "deck",
            "extract",
            "vision",
            "web",
            "read_document",
            "send_file",
            "computer_use",
            // The background handle: a command started
            // in the background is controllable only if the
            // control tool is declared beside it.
            "process"
        ]
    );
    let records = session::replay(session.path()).unwrap();
    assert!(
        !records
            .iter()
            .any(|record| matches!(record, Record::Memory { .. })),
        "{records:?}"
    );

    // An empty corpus is the same run with a place to write: no block,
    // no record, the same constant prompt — the write path and the
    // delegation tool declared.
    let mock = MockApi::start(vec![assistant_stream("Done.")]);
    let root = memory_corpus("empty-turn", &[]);
    let memory = std::sync::Arc::new(fleet::memory::Memory::load_with(
        &root,
        Box::new(fleet::memory::KeywordOnly),
    ));
    let mut session = Session::create(&home, "test-model").unwrap();
    run_with_memory(
        &mock,
        &mut session,
        Box::new(ApproveAll),
        Some(memory),
        None,
        "say done",
    )
    .await
    .unwrap();
    let bodies = mock.bodies();
    assert_eq!(
        bodies[0]["messages"][0]["content"],
        fleet::agent::default_prompt()
    );
    assert_eq!(
        declared_tools(&bodies[0]),
        vec![
            "bash",
            "read_file",
            "write_file",
            "list_dir",
            "edit",
            "apply_patch",
            "grep",
            "glob",
            "repo_map",
            "file_change",
            "todo",
            "measure",
            "imagine",
            "browse",
            "recall",
            "mcp_resource",
            "mcp_prompt",
            "remember",
            "supersede",
            "subagent",
            "convention",
            "plan",
            "deck",
            "extract",
            "vision",
            "web",
            "read_document",
            "send_file",
            "computer_use",
            // The background handle: a command started
            // in the background is controllable only if the
            // control tool is declared beside it.
            "process"
        ]
    );
    let records = session::replay(session.path()).unwrap();
    assert!(
        !records
            .iter()
            .any(|record| matches!(record, Record::Memory { .. })),
        "{records:?}"
    );
}

#[tokio::test]
async fn the_remember_tool_writes_a_fact_inside_a_run() {
    let mock = MockApi::start(vec![
        tool_call_stream(
            "call-1",
            "remember",
            r#"{"name":"a-new-fact","description":"a fact the model learned","basis":"inferred","tier":"working","valid-until":"2099-01-01","body":"The full content."}"#,
        ),
        // A write without its basis is a model-visible error, not a
        // file — the required rule, enforced where the model meets it.
        tool_call_stream(
            "call-2",
            "remember",
            r#"{"name":"no-basis","description":"a fact with no provenance"}"#,
        ),
        assistant_stream("Remembered."),
    ]);
    let home = temp_home("remember-run");
    let root = memory_corpus("remember-run", &[]);
    let memory = std::sync::Arc::new(fleet::memory::Memory::load_with(
        &root,
        Box::new(fleet::memory::KeywordOnly),
    ));
    let mut session = Session::create(&home, "test-model").unwrap();
    run_with_memory(
        &mock,
        &mut session,
        Box::new(ApproveAll),
        Some(memory),
        None,
        "remember this",
    )
    .await
    .unwrap();

    // The fact is on disk, in the shape the loader reads back.
    let loaded = fleet::memory::load_facts(&root);
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].name, "a-new-fact");
    assert_eq!(loaded[0].description, "a fact the model learned");
    assert_eq!(loaded[0].basis, fleet::memory::Basis::Inferred);
    assert_eq!(loaded[0].tier, fleet::memory::Tier::Working);
    assert_eq!(loaded[0].valid_until, "2099-01-01");
    assert_eq!(loaded[0].body, "The full content.");

    // Both results rode the log: the confirmation and the refusal.
    let records = session::replay(session.path()).unwrap();
    let results: Vec<(String, String)> = records
        .iter()
        .filter_map(|record| match record {
            Record::ToolResult { name, output, .. } if name == "remember" => {
                Some((name.clone(), output.clone()))
            }
            _ => None,
        })
        .collect();
    assert_eq!(results.len(), 2, "{results:?}");
    assert!(
        results[0].1.contains("remembered `a-new-fact`"),
        "{results:?}"
    );
    assert!(results[1].1.starts_with("error:"), "{results:?}");
    assert!(results[1].1.contains("basis"), "{results:?}");
}

#[tokio::test]
async fn a_confirmed_by_supersede_lands_with_approval_and_is_refused_without_it() {
    let arguments = r#"{"name":"load-bearing","justification":"the owner restated the rule","replacement":"load-bearing-v2","description":"the owner's new standing rule about the gate"}"#;

    // Approved (`--yes`): the supersession lands, and its justification
    // rides the calibration row — slice 12's guardrail input.
    let mock = MockApi::start(vec![
        tool_call_stream("call-1", "supersede", arguments),
        assistant_stream("Superseded."),
    ]);
    let home = temp_home("supersede-approved");
    let root = memory_corpus(
        "supersede-approved",
        &[(
            "load-bearing",
            fact_file(
                "load-bearing",
                "the owner's standing rule about the gate",
                "confirmed-by",
                "feedback",
                "",
            ),
        )],
    );
    let calib_root = temp_home("supersede-calib");
    let stream = fleet::calib::Stream::open(&calib_root);
    let memory = std::sync::Arc::new(fleet::memory::Memory::load_with(
        &root,
        Box::new(fleet::memory::KeywordOnly),
    ));
    let mut session = Session::create(&home, "test-model").unwrap();
    run_with_memory(
        &mock,
        &mut session,
        Box::new(ApproveAll),
        Some(memory),
        Some(stream.clone()),
        "restate the rule",
    )
    .await
    .unwrap();

    assert!(root.join("load-bearing-v2.md").exists());
    let replacement = std::fs::read_to_string(root.join("load-bearing-v2.md")).unwrap();
    assert!(
        replacement.contains("Supersedes `load-bearing` \u{2014} the owner restated the rule"),
        "{replacement}"
    );
    let old = std::fs::read_to_string(root.join("load-bearing.md")).unwrap();
    assert!(old.contains("superseded-by: load-bearing-v2"), "{old}");
    drop(stream);
    let rows = fleet::calib::read_day(&calib_root, &fleet::calib::today(), 64);
    let row = rows
        .iter()
        .find(|row| row.op == fleet::calib::OP_CONSOLIDATE)
        .expect("the consolidate row");
    assert_eq!(row.action, "supersede");
    assert_eq!(row.fact, "load-bearing");
    assert_eq!(row.reason, "the owner restated the rule");

    // Denied: nothing moves, and the model reads the denial.
    let mock = MockApi::start(vec![
        tool_call_stream("call-1", "supersede", arguments),
        assistant_stream("Denied."),
    ]);
    let home = temp_home("supersede-denied");
    let root = memory_corpus(
        "supersede-denied",
        &[(
            "load-bearing",
            fact_file(
                "load-bearing",
                "the owner's standing rule about the gate",
                "confirmed-by",
                "feedback",
                "",
            ),
        )],
    );
    let memory = std::sync::Arc::new(fleet::memory::Memory::load_with(
        &root,
        Box::new(fleet::memory::KeywordOnly),
    ));
    let mut session = Session::create(&home, "test-model").unwrap();
    run_with_memory(
        &mock,
        &mut session,
        Box::new(DenyAll),
        Some(memory),
        None,
        "restate the rule",
    )
    .await
    .unwrap();

    assert!(!root.join("load-bearing-v2.md").exists());
    let old = std::fs::read_to_string(root.join("load-bearing.md")).unwrap();
    // The file still carries its bare (unset) `superseded-by:` line —
    // what must be absent is the stamp naming the replacement.
    assert!(!old.contains("superseded-by: load-bearing-v2"), "{old}");
    let records = session::replay(session.path()).unwrap();
    let denial = records
        .iter()
        .find_map(|record| match record {
            Record::ToolResult {
                name,
                output,
                approved,
                ..
            } if name == "supersede" => Some((output.clone(), *approved)),
            _ => None,
        })
        .expect("the supersede tool_result");
    assert!(!denial.1, "the call was not approved");
    assert!(denial.0.contains("denied"), "{denial:?}");
}

// --- self-maintenance (slice 12) -------------------------------------------

/// One full agent run whose `improve` tool has a home to file into.
async fn run_with_improve(
    mock: &MockApi,
    session: &mut Session,
    approver: Box<dyn Approver>,
    home: &Path,
    task: &str,
) -> Result<RunReport, AgentError> {
    let client = Client::new(mock.url(), "test-key");
    let options = AgentOptions {
        goal: None,
        model: "test-model".to_string(),
        identity: None,
        max_turns: 5,
        ui_tx: None,
        lane_specs: None,
        calib: None,
        skills: Vec::new(),
        role_overlay: None,
        memory: None,
        scope: None,
        improve_home: Some(home.to_path_buf()),
        route_home: None,
        gui_prompt: None,
        compact_now: false,
        checkpoints: None,
        file_access: FileAccess::Unrestricted,
        plan_mode: false,
    };
    let mut progress = Vec::new();
    let mut agent = Agent::new(&client, options, session, approver, &mut progress).unwrap();
    agent.run(task).await
}

#[tokio::test]
async fn the_improve_tool_files_a_proposal_inside_a_run_and_never_applies_one() {
    // A session's justified skill update: the model calls `improve`,
    // the proposal lands in the general store, and the tool_result the
    // model reads names it — filing is all the tool ever does, and
    // applying is the owner's `fleet improve apply`.
    let change = serde_json::json!({
        "op": "write-skill",
        "content": "---\nname: bash-recovery\ndescription: recover from a failed call\n---\n\nthe correction\n"
    })
    .to_string();
    let arguments = serde_json::json!({
        "name": "bash-recovery-lesson",
        "target": "skill",
        "target-name": "bash-recovery",
        "basis": "inferred",
        "justification": "a failed bash call was corrected in this session",
        "acceptance": "the skill's body carries the correction",
        "effect": "the next session recovers without the failure",
        "change": change,
    })
    .to_string();
    let mock = MockApi::start(vec![
        tool_call_stream("call-1", "improve", &arguments),
        assistant_stream("Filed the proposal."),
    ]);
    let home = temp_home("improve-run");
    let mut session = Session::create(&home, "test-model").unwrap();
    run_with_improve(
        &mock,
        &mut session,
        Box::new(ApproveAll),
        &home,
        "draft the session's lesson",
    )
    .await
    .unwrap();

    // The declaration rides the request beside the delegation tool,
    // and only when the run carries a home.
    let bodies = mock.bodies();
    assert_eq!(
        declared_tools(&bodies[0]),
        vec![
            "bash",
            "read_file",
            "write_file",
            "list_dir",
            "edit",
            "apply_patch",
            "grep",
            "glob",
            "repo_map",
            "file_change",
            "todo",
            "measure",
            "imagine",
            "browse",
            "recall",
            "mcp_resource",
            "mcp_prompt",
            "subagent",
            "improve",
            "convention",
            "plan",
            "deck",
            "extract",
            "vision",
            "web",
            "read_document",
            "send_file",
            "computer_use",
            // The background handle: a command started
            // in the background is controllable only if the
            // control tool is declared beside it.
            "process"
        ]
    );

    // The proposal is on disk, in the shape the gate reads: frontmatter
    // with the target and the basis, the four body sections, and the
    // change as a fenced JSON block.
    let filed =
        std::fs::read_to_string(home.join("proposals").join("bash-recovery-lesson.md")).unwrap();
    assert!(filed.contains("name: bash-recovery-lesson"), "{filed}");
    assert!(filed.contains("target: skill"), "{filed}");
    assert!(filed.contains("target-name: bash-recovery"), "{filed}");
    assert!(filed.contains("basis: inferred"), "{filed}");
    assert!(filed.contains("status: pending"), "{filed}");
    assert!(filed.contains("## Justification"), "{filed}");
    assert!(filed.contains("## Acceptance criteria"), "{filed}");
    assert!(filed.contains("## Expected effect"), "{filed}");
    assert!(filed.contains("## Change"), "{filed}");
    assert!(filed.contains("\"op\":\"write-skill\""), "{filed}");

    // The tool_result the model read names the filing and where it
    // landed, and the live skills tree is untouched — nothing applied.
    let records = session::replay(session.path()).unwrap();
    let (output, approved) = tool_result(&records, "improve");
    assert!(approved);
    assert!(
        output.contains("filed proposal `bash-recovery-lesson`"),
        "{output}"
    );
    assert!(
        output.contains("fleet improve apply bash-recovery-lesson"),
        "{output}"
    );
    assert!(!home.join("skills").join("bash-recovery").exists());
}

#[tokio::test]
async fn a_run_without_a_home_does_not_declare_the_improve_tool() {
    // The tool exists only where a session can propose: a memory-less,
    // home-less run is the run this crate has always had, byte for
    // byte, and the declaration is absent from what the model saw.
    let mock = MockApi::start(vec![assistant_stream("Done.")]);
    let home = temp_home("improve-less");
    let mut session = Session::create(&home, "test-model").unwrap();
    run_on(&mock, &mut session, Box::new(ApproveAll), 5, "say done")
        .await
        .unwrap();
    let bodies = mock.bodies();
    assert_eq!(
        declared_tools(&bodies[0]),
        vec![
            "bash",
            "read_file",
            "write_file",
            "list_dir",
            "edit",
            "apply_patch",
            "grep",
            "glob",
            "repo_map",
            "file_change",
            "todo",
            "measure",
            "imagine",
            "browse",
            "recall",
            "mcp_resource",
            "mcp_prompt",
            "subagent",
            "convention",
            "plan",
            "deck",
            "extract",
            "vision",
            "web",
            "read_document",
            "send_file",
            "computer_use",
            // The background handle: a command started
            // in the background is controllable only if the
            // control tool is declared beside it.
            "process"
        ]
    );
}

// --- subagents (slice 9) ---------------------------------------------------

/// One `subagent` call's arguments as the model issues them.
fn delegation_arguments(goal: &str) -> String {
    serde_json::json!({ "goal": goal }).to_string()
}

/// A streamed answer carrying text AND one tool call: the shape a model
/// uses when it narrates before it acts, which is what makes a partial
/// result partial.
fn narrated_tool_call_stream(
    id: &str,
    name: &str,
    arguments: &str,
    narration: &str,
) -> ScriptedResponse {
    let (head, tail) = arguments.split_at(arguments.len() / 2);
    stream(vec![
        content_delta(narration),
        tool_call_delta(0, id, name, head),
        tool_call_delta(0, "", "", tail),
        finish_frame("tool_calls"),
        usage_frame(10, 5),
        "data: [DONE]".to_string(),
    ])
}

/// The child session's summary, from a home that holds exactly one.
fn child_summary(home: &Path) -> session::SessionSummary {
    let summaries = session::list(home).unwrap();
    let mut children: Vec<session::SessionSummary> = summaries
        .into_iter()
        .filter(|summary| summary.kind.as_deref() == Some("subagent"))
        .collect();
    assert_eq!(children.len(), 1, "exactly one child log: {children:?}");
    children.remove(0)
}

/// One tool_result record from `records`, by tool name.
fn tool_result(records: &[Record], name: &str) -> (String, bool) {
    records
        .iter()
        .find_map(|record| match record {
            Record::ToolResult {
                name: recorded,
                output,
                approved,
                ..
            } if recorded == name => Some((output.clone(), *approved)),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no {name} tool_result in {records:?}"))
}

/// An approver that records every call it was asked about and refuses a
/// subagent's own calls, so a test can watch a child's call cross the
/// parent's gate and be refused there.
struct GateSpy {
    seen: Arc<std::sync::Mutex<Vec<ToolCall>>>,
}

impl Approver for GateSpy {
    fn approve(&mut self, call: &ToolCall) -> bool {
        self.seen.lock().unwrap().push(call.clone());
        // The parent's own delegation goes through; the child's calls do
        // not. The labelled name is what tells them apart.
        !call.function.name.starts_with("subagent \u{2192}")
    }
}

/// The headline of slice 9: a delegated task runs in its own session log,
/// on the parent's own model, and the parent's log carries the call and
/// its result.
#[tokio::test]
async fn a_delegated_task_runs_in_its_own_session_on_the_parents_model() {
    let mock = MockApi::start(vec![
        tool_call_stream(
            "call-1",
            "subagent",
            &delegation_arguments("count the files in /tmp"),
        ),
        tool_call_stream("child-1", "use_skill", r#"{"name":"demo-skill"}"#),
        assistant_stream("there are three files"),
        assistant_stream("the child counted three files"),
    ]);
    let home = temp_home("subagent-own-session");
    // The arm resolves the launch folder's registration, so the test
    // pins its own absence: a scratch FLEET_HOME with no config means
    // no machine facts, no workspace, the plain prompt — whatever the
    // launching repository has registered in the real home. (This
    // repository itself became registered mid-plan — the motivating
    // case — and this test's premise must not ride on the world it
    // runs in.)
    let scratch = temp_home("subagent-own-session-fleet-home");
    let mut session = Session::create(&home, "test-model").unwrap();
    // The lock guards the SET (atomic against the other ambient
    // windows); the restoration guard outlives the await, the lock
    // guard does not.
    let _fh = {
        let _lock = AMBIENT_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        AmbientKey::set("FLEET_HOME", scratch.to_str().unwrap())
    };
    let report = run_on(
        &mock,
        &mut session,
        Box::new(ApproveAll),
        5,
        "delegate the count",
    )
    .await
    .unwrap();
    assert_eq!(report.stop_reason, StopReason::Completed);

    // Two logs exist afterward: the parent's and the child's.
    let summaries = session::list(&home).unwrap();
    assert_eq!(summaries.len(), 2, "{summaries:?}");
    let child = child_summary(&home);
    assert_eq!(child.model, "test-model");
    assert!(child.ended, "the child stopped orderly");

    // The child's own start record carries its kind and its parent — the
    // marker that survives a replay and the picker listing alike.
    let child_records = session::replay(&child.path).unwrap();
    match &child_records[0] {
        Record::Start {
            kind,
            parent,
            model,
            ..
        } => {
            assert_eq!(kind.as_deref(), Some("subagent"));
            assert_eq!(parent.as_deref(), Some(session.id()));
            assert_eq!(model, "test-model");
        }
        other => panic!("expected a start record, got {other:?}"),
    }
    // The child's conversation is the goal and its answer, nothing else:
    // no trace of the parent's task or its turns.
    let messages = session::messages(&child_records);
    assert_eq!(messages.len(), 4, "{messages:?}");
    assert_eq!(messages[0].role, Role::User);
    assert_eq!(
        messages[0].content.as_deref(),
        Some("count the files in /tmp")
    );
    assert_eq!(messages[1].role, Role::Assistant);
    assert_eq!(messages[1].tool_calls[0].function.name, "use_skill");
    assert_eq!(messages[2].role, Role::Tool);
    assert_eq!(
        messages[2].content.as_deref(),
        Some("error: no skill named `demo-skill`")
    );
    assert_eq!(
        messages[3].content.as_deref(),
        Some("there are three files")
    );
    assert!(matches!(
        child_records.last(),
        Some(Record::End { stop_reason, .. }) if stop_reason == "completed"
    ));

    // Both sessions' requests carried the same model, and the child's
    // carried the same compiled-in system prompt: the same loop, the same
    // prompt, a different conversation.
    let bodies = mock.bodies();
    assert_eq!(bodies.len(), 4, "{bodies:?}");
    for body in &bodies {
        assert_eq!(body["model"], "test-model", "{body}");
    }
    assert_eq!(
        bodies[1]["messages"][0]["content"],
        fleet::agent::default_prompt()
    );
    // The child's first user message is the goal, whole.
    assert_eq!(
        bodies[1]["messages"][1]["content"],
        "count the files in /tmp"
    );
    // The child's tool set is its admitted scope. `use_skill` is admitted
    // and undeclared here, exactly as on a skill-less parent's run: the
    // scope decides what is callable, the run's skills decide what is
    // declared. A call to an admitted-but-undeclared tool is the typed
    // unknown the skills path returns, not a scope refusal.
    assert_eq!(
        declared_tools(&bodies[1]),
        vec![
            "bash",
            "read_file",
            "write_file",
            "list_dir",
            "edit",
            "apply_patch",
            "grep",
            "glob",
            "repo_map",
            "file_change",
            "todo"
        ]
    );
    let (skill_result, _) = tool_result(&child_records, "use_skill");
    assert!(skill_result.starts_with("error:"), "{skill_result}");
    assert!(!skill_result.contains("admitted scope"), "{skill_result}");
    // The parent's request declared `subagent` beside its own four.
    assert!(declared_tools(&bodies[0]).contains(&"subagent".to_string()));

    // The parent's session log carries the call and its result: the
    // outcome, the child's session, its log path, and its words.
    let parent_records = session::replay(session.path()).unwrap();
    let (output, approved) = tool_result(&parent_records, "subagent");
    assert!(approved, "the delegation was approved");
    assert!(output.starts_with("subagent completed"), "{output}");
    assert!(output.contains(child.id.as_str()), "{output}");
    assert!(
        output.contains(&child.path.display().to_string()),
        "{output}"
    );
    assert!(output.contains("there are three files"), "{output}");
    // The assistant turn that requested the call is logged with its call
    // intact, so the replay stays a valid sequence.
    assert!(parent_records.iter().any(|record| matches!(
        record,
        Record::Assistant { tool_calls, .. } if tool_calls.iter().any(|call| call.function.name == "subagent")
    )));

    std::fs::remove_dir_all(&home).ok();
}

/// C8: a child's built-in tools work in the parent chat's folder. The
/// child prints its own working directory, and the path in its log is
/// the folder, not the process's launch directory.
#[tokio::test]
async fn a_subagents_builtins_work_in_the_parents_folder() {
    let folder = temp_home("subagent-workdir");
    let mock = MockApi::start(vec![
        tool_call_stream(
            "call-1",
            "subagent",
            &delegation_arguments("print the working directory"),
        ),
        tool_call_stream("child-1", "bash", r#"{"command":"pwd -P"}"#),
        assistant_stream("printed it"),
        assistant_stream("the child printed the folder"),
    ]);
    let home = temp_home("subagent-workdir-home");
    // The default (non-isolated) path, pinned against the world: a
    // scratch FLEET_HOME means no machine facts, so the arm never opts
    // this child into a workspace — whatever the launching repository
    // has registered in the real home.
    let scratch = temp_home("subagent-workdir-fleet-home");
    let mut session = Session::create(&home, "test-model").unwrap();
    let client = Client::new(mock.url(), "test-key");
    let _fh = {
        let _lock = AMBIENT_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        AmbientKey::set("FLEET_HOME", scratch.to_str().unwrap())
    };
    {
        let _held = &_fh;
        let options = AgentOptions {
            goal: None,
            model: "test-model".to_string(),
            identity: None,
            max_turns: 6,
            ui_tx: None,
            lane_specs: None,
            calib: None,
            skills: Vec::new(),
            role_overlay: None,
            memory: None,
            scope: None,
            improve_home: None,
            route_home: None,
            gui_prompt: None,
            compact_now: false,
            checkpoints: None,
            file_access: FileAccess::Folder(folder.clone()),
            plan_mode: false,
        };
        let mut progress = Vec::new();
        let mut agent = Agent::new(
            &client,
            options,
            &mut session,
            Box::new(ApproveAll),
            &mut progress,
        )
        .unwrap();
        agent.run("show the folder").await.unwrap();
    }

    let child = child_summary(&home);
    let records = session::replay(&child.path).unwrap();
    let (output, _) = tool_result(&records, "bash");
    let canonical = folder.canonicalize().unwrap();
    assert!(
        output.contains(&canonical.display().to_string()),
        "the child's pwd is the parent's folder: {output}"
    );
    assert!(
        !output.contains(&std::env::current_dir().unwrap().display().to_string()),
        "the child did not work in the process's directory: {output}"
    );

    std::fs::remove_dir_all(&home).ok();
    std::fs::remove_dir_all(&folder).ok();
}

/// C9: a chat with no working folder cannot touch the filesystem. The
/// model's write is refused with the rule stated, and the file is not
/// created — the refusal is the tool's answer, not a prompt line.
#[tokio::test]
async fn a_chat_with_no_folder_cannot_touch_the_filesystem() {
    let target = std::env::temp_dir().join(format!(
        "fleet-no-folder-{}-{}.txt",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    let _ = std::fs::remove_file(&target);
    let arguments = serde_json::json!({
        "path": target.display().to_string(),
        "content": "the chat was never given a folder"
    })
    .to_string();
    let mock = MockApi::start(vec![
        tool_call_stream("call-1", "write_file", &arguments),
        assistant_stream("I need a working folder before I can write."),
    ]);
    let home = temp_home("no-folder-refusal");
    let mut session = Session::create(&home, "test-model").unwrap();
    let client = Client::new(mock.url(), "test-key");
    let options = AgentOptions {
        goal: None,
        model: "test-model".to_string(),
        identity: None,
        max_turns: 4,
        ui_tx: None,
        lane_specs: None,
        calib: None,
        skills: Vec::new(),
        role_overlay: None,
        memory: None,
        scope: None,
        improve_home: None,
        route_home: None,
        gui_prompt: None,
        compact_now: false,
        checkpoints: None,
        file_access: FileAccess::None,
        plan_mode: false,
    };
    let mut progress = Vec::new();
    {
        // The agent borrows the session for the run; the replay reads
        // the log after the borrow ends.
        let mut agent = Agent::new(
            &client,
            options,
            &mut session,
            Box::new(ApproveAll),
            &mut progress,
        )
        .unwrap();
        agent.run("write a file").await.unwrap();
    }

    let records = session::replay(session.path()).unwrap();
    let (output, _) = tool_result(&records, "write_file");
    assert!(output.contains("no working folder"), "{output}");
    assert!(output.contains("Ask the owner"), "{output}");
    assert!(!target.exists(), "nothing was written: {output}");

    std::fs::remove_dir_all(&home).ok();
}

// --- the report repair -----------------------------------------------------

/// The report the repaired run is expected to end with.
const REPAIR_REPORT: &str = "The command printed hi (exit 0). Nothing needed from you.";

/// The system prompt of one recorded request.
fn request_system(mock: &MockApi, index: usize) -> String {
    mock.bodies()[index]["messages"][0]["content"]
        .as_str()
        .unwrap_or_default()
        .to_string()
}

/// The end record's stop reason, when the log carries one.
fn end_reason(records: &[Record]) -> Option<String> {
    records.iter().find_map(|record| match record {
        Record::End { stop_reason, .. } => Some(stop_reason.clone()),
        _ => None,
    })
}

/// C1/C5: a run that executed a tool and stopped with no message is
/// asked once, with the conversation unchanged, and the report it
/// returns is the run's answer.
#[tokio::test]
async fn a_blank_stop_is_asked_once_for_its_report() {
    let mock = MockApi::start(vec![
        tool_call_stream("call-1", "bash", r#"{"command":"echo hi"}"#),
        assistant_stream(""),
        assistant_stream(REPAIR_REPORT),
    ]);
    let home = temp_home("repair-blank");
    let mut session = Session::create(&home, "test-model").unwrap();
    let report = run_on(
        &mock,
        &mut session,
        Box::new(ApproveAll),
        6,
        "run the check",
    )
    .await
    .unwrap();

    assert_eq!(report.final_message.as_deref(), Some(REPAIR_REPORT));
    let bodies = mock.bodies();
    assert_eq!(
        bodies.len(),
        3,
        "the tool turn, the silent turn, the repair"
    );
    let system = request_system(&mock, 2);
    assert!(
        system.contains("You stopped without a report"),
        "the repair ask rides the request: {system}"
    );
    assert!(system.starts_with(fleet::agent::SYSTEM_PROMPT), "{system}");
    assert_eq!(
        bodies[2]["messages"].as_array().map(Vec::len),
        bodies[1]["messages"].as_array().map(Vec::len),
        "the silent turn was not pushed: the conversation is unchanged"
    );

    let records = session::replay(session.path()).unwrap();
    assert!(
        !records.iter().any(|record| matches!(
            record,
            Record::Assistant { content, tool_calls, .. }
                if tool_calls.is_empty()
                    && content.as_deref().map(str::trim).unwrap_or("").is_empty()
        )),
        "no empty assistant turn is in the log"
    );
    assert_eq!(end_reason(&records).as_deref(), Some("handed back"));
    std::fs::remove_dir_all(&home).ok();
}

/// C2: a bare acknowledgment after tool work is the same failure.
#[tokio::test]
async fn a_bare_acknowledgment_is_asked_once_for_its_report() {
    let mock = MockApi::start(vec![
        tool_call_stream("call-1", "bash", r#"{"command":"echo hi"}"#),
        assistant_stream("Done."),
        assistant_stream(REPAIR_REPORT),
    ]);
    let home = temp_home("repair-ack");
    let mut session = Session::create(&home, "test-model").unwrap();
    let report = run_on(
        &mock,
        &mut session,
        Box::new(ApproveAll),
        6,
        "run the check",
    )
    .await
    .unwrap();

    assert_eq!(report.final_message.as_deref(), Some(REPAIR_REPORT));
    assert_eq!(mock.bodies().len(), 3);
    assert!(request_system(&mock, 2).contains("You stopped without a report"));
    std::fs::remove_dir_all(&home).ok();
}

/// C2/C3: an informative answer and a conversational answer are not
/// repaired — the trigger is a missing report, not a short message.
#[tokio::test]
async fn an_informative_or_conversational_stop_is_not_repaired() {
    let mock = MockApi::start(vec![
        tool_call_stream("call-1", "bash", r#"{"command":"echo hi"}"#),
        assistant_stream("The command printed hi and exited 0; nothing needed from you."),
    ]);
    let home = temp_home("repair-no");
    let mut session = Session::create(&home, "test-model").unwrap();
    let report = run_on(&mock, &mut session, Box::new(ApproveAll), 6, "run it")
        .await
        .unwrap();
    assert_eq!(mock.bodies().len(), 2, "no second ask");
    assert_eq!(
        report.final_message.as_deref(),
        Some("The command printed hi and exited 0; nothing needed from you.")
    );
    for body in mock.bodies() {
        assert!(
            !body["messages"][0]["content"]
                .as_str()
                .unwrap_or_default()
                .contains("You stopped without a report"),
            "{body}"
        );
    }

    // A conversational "yes" ran no tools, so it is an answer, not a
    // missing report.
    let mock = MockApi::start(vec![assistant_stream("Yes.")]);
    let home = temp_home("repair-chat");
    let mut session = Session::create(&home, "test-model").unwrap();
    let report = run_on(&mock, &mut session, Box::new(ApproveAll), 4, "is it ready?")
        .await
        .unwrap();
    assert_eq!(report.final_message.as_deref(), Some("Yes."));
    assert_eq!(mock.bodies().len(), 1, "a chat answer is not repaired");

    std::fs::remove_dir_all(&home).ok();
}

/// C4: the repair is bounded. A second silent stop ends the run, and
/// the log and the finished event say the report is what is missing.
#[tokio::test]
async fn a_second_silent_stop_ends_the_run_with_no_report() {
    let mock = MockApi::start(vec![
        tool_call_stream("call-1", "bash", r#"{"command":"echo hi"}"#),
        assistant_stream(""),
        assistant_stream("ok"),
    ]);
    let home = temp_home("repair-bound");
    let mut session = Session::create(&home, "test-model").unwrap();
    let report = run_on(
        &mock,
        &mut session,
        Box::new(ApproveAll),
        6,
        "run the check",
    )
    .await
    .unwrap();

    assert_eq!(
        report.final_message.as_deref(),
        Some("ok"),
        "the second silent stop is what the run ended on"
    );
    assert_eq!(
        mock.bodies().len(),
        3,
        "one ask, never a chain of them: {}",
        mock.bodies().len()
    );
    let records = session::replay(session.path()).unwrap();
    assert_eq!(
        end_reason(&records).as_deref(),
        Some("handed back without a report")
    );
    std::fs::remove_dir_all(&home).ok();
}

/// A routed parent's child runs on the routed candidate's model: the
/// never-downgrade rule in its strongest form, because the child
/// inherits the roster's choice rather than the nominal one.
#[tokio::test]
async fn a_child_runs_on_the_routed_model_the_parent_runs_on() {
    let mock = MockApi::start(vec![
        tool_call_stream(
            "call-1",
            "subagent",
            &delegation_arguments("answer briefly"),
        ),
        assistant_stream("answered on the routed model"),
        assistant_stream("the child answered"),
    ]);
    let home = temp_home("subagent-routed");
    let mut session = Session::create(&home, "test-model").unwrap();
    let resolved = vec![resolved("cand-one", mock.url(), false, None)];
    let decision = RouteDecision {
        provider: "cand-one".to_string(),
        model: "cand-one-model".to_string(),
        how: How::Pinned,
        workload: None,
        verdict: None,
        execution: None,
        reason: "pinned for the test".to_string(),
        judged_from: fleet::route::JudgedFrom::Fresh,
        start_index: 0,
        // The walk's entry names the candidate's own model — that is
        // what every request on this route carries.
        order: vec![("cand-one".to_string(), "cand-one-model".to_string())],
    };
    let client = Client::routed(Arc::new(Router::new(resolved, decision)));
    let options = AgentOptions {
        goal: None,
        model: "test-model".to_string(),
        identity: None,
        max_turns: 5,
        ui_tx: None,
        lane_specs: None,
        calib: None,
        skills: Vec::new(),
        role_overlay: None,
        memory: None,
        scope: None,
        improve_home: None,
        route_home: None,
        gui_prompt: None,
        compact_now: false,
        checkpoints: None,
        file_access: FileAccess::Unrestricted,
        plan_mode: false,
    };
    let mut progress = Vec::new();
    let mut agent = Agent::new(
        &client,
        options,
        &mut session,
        Box::new(ApproveAll),
        &mut progress,
    )
    .unwrap();
    agent.run("delegate it").await.unwrap();

    // Every request — the parent's two and the child's one — carries the
    // roster candidate's model, not the nominal one the options name.
    let bodies = mock.bodies();
    assert_eq!(bodies.len(), 3, "{bodies:?}");
    for body in &bodies {
        assert_eq!(body["model"], "cand-one-model", "{body}");
    }
    // The child's log names the same nominal model the parent's does: a
    // `start` record carries the run's model, and the route decides what
    // serves — for both sessions alike.
    let child = child_summary(&home);
    assert_eq!(child.model, "test-model");
    let parent = session::list(&home)
        .unwrap()
        .into_iter()
        .find(|summary| summary.kind.is_none())
        .expect("the parent's log");
    assert_eq!(parent.model, "test-model");

    std::fs::remove_dir_all(&home).ok();
}

/// The scope, from both ends: the declarations exclude a tool outside it,
/// and the loop refuses a call that arrives anyway — by name.
#[tokio::test]
async fn a_tool_outside_the_childs_scope_is_refused_by_name() {
    let mock = MockApi::start(vec![
        tool_call_stream(
            "call-1",
            "subagent",
            &serde_json::json!({
                "goal": "write the note",
                "tools": ["bash"],
            })
            .to_string(),
        ),
        tool_call_stream(
            "child-1",
            "write_file",
            r#"{"path":"/tmp/nope.txt","content":"x"}"#,
        ),
        assistant_stream("I could not write the file"),
        assistant_stream("the child could not write it"),
    ]);
    let home = temp_home("subagent-scope");
    let mut session = Session::create(&home, "test-model").unwrap();
    run_on(
        &mock,
        &mut session,
        Box::new(ApproveAll),
        5,
        "delegate the write",
    )
    .await
    .unwrap();

    let bodies = mock.bodies();
    // The child's second request declared only what its scope admits.
    assert_eq!(
        declared_tools(&bodies[2]),
        vec!["bash"],
        "{:?}",
        declared_tools(&bodies[2])
    );
    // The refusal is a typed, model-visible result that names the tool and
    // the scope it fell outside.
    let child = child_summary(&home);
    let (output, approved) = tool_result(&session::replay(&child.path).unwrap(), "write_file");
    assert!(
        approved,
        "the call crossed the gate before the harness refused it"
    );
    assert!(
        output.contains("outside this subagent's admitted scope"),
        "{output}"
    );
    assert!(output.contains("`write_file`"), "{output}");
    assert!(output.contains("bash"), "{output}");

    std::fs::remove_dir_all(&home).ok();
}

/// The depth cap: a subagent reaching for `subagent` is refused by name,
/// with the cap named — and the tool was never declared to it.
#[tokio::test]
async fn a_subagent_cannot_spawn_a_subagent() {
    let mock = MockApi::start(vec![
        tool_call_stream(
            "call-1",
            "subagent",
            &delegation_arguments("delegate onward"),
        ),
        tool_call_stream(
            "child-1",
            "subagent",
            &delegation_arguments("delegate again"),
        ),
        assistant_stream("I cannot delegate further"),
        assistant_stream("the child could not delegate"),
    ]);
    let home = temp_home("subagent-depth");
    let mut session = Session::create(&home, "test-model").unwrap();
    run_on(
        &mock,
        &mut session,
        Box::new(ApproveAll),
        5,
        "delegate the delegation",
    )
    .await
    .unwrap();

    // The delegation tool was never in the child's declarations: the one
    // request the child made is the second one recorded.
    let bodies = mock.bodies();
    assert_eq!(bodies.len(), 4, "{bodies:?}");
    assert!(
        !declared_tools(&bodies[1]).contains(&"subagent".to_string()),
        "{:?}",
        declared_tools(&bodies[1])
    );
    let child = child_summary(&home);
    let (output, _) = tool_result(&session::replay(&child.path).unwrap(), "subagent");
    assert!(
        output.contains("a subagent cannot spawn subagents"),
        "{output}"
    );
    assert!(output.contains("depth cap 2"), "{output}");

    std::fs::remove_dir_all(&home).ok();
}

/// The turn cap: a child that keeps asking for tool calls stops at its
/// cap with outcome `max-turns`, its partial log intact, and the parent
/// still gets the partial result.
#[tokio::test]
async fn a_child_stops_at_its_turn_cap_with_its_partial_log_intact() {
    let mock = MockApi::start(vec![
        tool_call_stream(
            "call-1",
            "subagent",
            &serde_json::json!({
                "goal": "check every file",
                "max-turns": 2,
            })
            .to_string(),
        ),
        narrated_tool_call_stream(
            "child-1",
            "bash",
            r#"{"command":"echo one"}"#,
            "checked one of five",
        ),
        narrated_tool_call_stream(
            "child-2",
            "bash",
            r#"{"command":"echo two"}"#,
            "checked two of five",
        ),
        assistant_stream("the child ran out of turns"),
    ]);
    let home = temp_home("subagent-cap");
    let mut session = Session::create(&home, "test-model").unwrap();
    let report = run_on(
        &mock,
        &mut session,
        Box::new(ApproveAll),
        5,
        "delegate the check",
    )
    .await
    .unwrap();
    assert_eq!(
        report.stop_reason,
        StopReason::Completed,
        "the parent carries on"
    );

    let child = child_summary(&home);
    let child_records = session::replay(&child.path).unwrap();
    // The child's own log ends at its cap, with both of its turns in it.
    assert!(
        matches!(
            child_records.last(),
            Some(Record::End { stop_reason, turns, .. }) if stop_reason == "max-turns" && *turns == 2
        ),
        "{child_records:?}"
    );
    assert_eq!(
        child_records
            .iter()
            .filter(|record| matches!(record, Record::ToolResult { .. }))
            .count(),
        2
    );

    // The parent's result names the outcome and carries the child's last
    // words — the partial result, not silence.
    let (output, _) = tool_result(&session::replay(session.path()).unwrap(), "subagent");
    assert!(output.starts_with("subagent max-turns"), "{output}");
    assert!(output.contains("2 turns"), "{output}");
    assert!(output.contains("checked two of five"), "{output}");
    assert!(output.contains(child.id.as_str()), "{output}");

    std::fs::remove_dir_all(&home).ok();
}

/// A failure returns partial results, not silence: an API error mid-child
/// yields outcome `failed` with the reason and the log path, and the
/// parent's log carries the call and that result.
#[tokio::test]
async fn a_failed_child_returns_its_partial_result_and_its_reason() {
    let mock = MockApi::start(vec![
        tool_call_stream(
            "call-1",
            "subagent",
            &delegation_arguments("check every file"),
        ),
        narrated_tool_call_stream(
            "child-1",
            "bash",
            r#"{"command":"echo one"}"#,
            "checked one of five",
        ),
        ScriptedResponse {
            status: "500 Internal Server Error".to_string(),
            body: r#"{"error":{"message":"the child's model went away"}}"#.to_string(),
            sse: Vec::new(),
            delay_ms: 0,
        },
        assistant_stream("the child failed partway"),
    ]);
    let home = temp_home("subagent-failure");
    let mut session = Session::create(&home, "test-model").unwrap();
    let report = run_on(
        &mock,
        &mut session,
        Box::new(ApproveAll),
        5,
        "delegate the check",
    )
    .await
    .unwrap();
    assert_eq!(
        report.stop_reason,
        StopReason::Completed,
        "the parent survives its child's failure"
    );

    let child = child_summary(&home);
    let child_records = session::replay(&child.path).unwrap();
    // The interrupted-log shape: the child's turns are there, its `end` is
    // not — and its partial narration survived.
    assert!(
        !matches!(child_records.last(), Some(Record::End { .. })),
        "{child_records:?}"
    );
    assert!(child_records.iter().any(|record| matches!(
        record,
        Record::Assistant { content: Some(text), .. }
        if text.contains("checked one of five")
    )));

    // The parent's result is the typed failure: the reason, the log path,
    // and the child's last words.
    let (output, approved) = tool_result(&session::replay(session.path()).unwrap(), "subagent");
    assert!(approved, "{output}");
    assert!(
        output.starts_with("subagent failed (API error 500: the child's model went away)"),
        "{output}"
    );
    // One turn completed: the log's count, not the loop's counter, which
    // would name the turn the child died on.
    assert!(output.contains("1 turn"), "{output}");
    assert!(
        output.contains(&child.path.display().to_string()),
        "{output}"
    );
    assert!(output.contains("checked one of five"), "{output}");

    std::fs::remove_dir_all(&home).ok();
}

/// The same approver: a child's call crosses the parent's gate, labelled
/// for whose call it is, and the gate's refusal reaches the child's log as
/// an ordinary denial.
#[tokio::test]
async fn a_childs_call_crosses_the_parents_approver_labelled() {
    let mock = MockApi::start(vec![
        tool_call_stream(
            "call-1",
            "subagent",
            &delegation_arguments("run echo from-the-child"),
        ),
        tool_call_stream("child-1", "bash", r#"{"command":"echo from-the-child"}"#),
        assistant_stream("my command was denied"),
        assistant_stream("the child's command was denied"),
    ]);
    let home = temp_home("subagent-approver");
    let mut session = Session::create(&home, "test-model").unwrap();
    let spy = Arc::new(std::sync::Mutex::new(Vec::new()));
    let approver = Box::new(GateSpy {
        seen: Arc::clone(&spy),
    });
    run_on(&mock, &mut session, approver, 5, "delegate the echo")
        .await
        .unwrap();

    let seen = spy.lock().unwrap();
    // Two calls crossed the one gate: the parent's own delegation, then
    // the child's bash — named for the subagent it came from.
    assert_eq!(seen.len(), 2, "{seen:?}");
    assert_eq!(seen[0].function.name, "subagent");
    assert_eq!(seen[1].function.name, "subagent \u{2192} bash");
    // The forwarded call carries the goal beside the child's own
    // arguments, so the prompt can name both.
    let forwarded: serde_json::Value = serde_json::from_str(&seen[1].function.arguments).unwrap();
    assert_eq!(forwarded["goal"], "run echo from-the-child");
    assert_eq!(forwarded["tool"], "bash");
    assert_eq!(forwarded["arguments"]["command"], "echo from-the-child");

    // The denial reached the child's log as an ordinary denied result, and
    // the command never ran.
    let child = child_summary(&home);
    let (output, approved) = tool_result(&session::replay(&child.path).unwrap(), "bash");
    assert!(!approved, "{output}");
    assert!(output.contains("denied"), "{output}");

    std::fs::remove_dir_all(&home).ok();
}

/// A malformed delegation whose arguments are wrong is a typed refusal the parent's
/// model can read and react to — no child session, no silence.
#[tokio::test]
async fn a_malformed_delegation_is_a_typed_refusal() {
    let mock = MockApi::start(vec![
        tool_call_stream("call-1", "subagent", r#"{"tools":["bash"]}"#),
        assistant_stream("the goal was missing"),
    ]);
    let home = temp_home("subagent-refusal");
    let mut session = Session::create(&home, "test-model").unwrap();
    run_on(
        &mock,
        &mut session,
        Box::new(ApproveAll),
        5,
        "delegate nothing",
    )
    .await
    .unwrap();

    let (output, _) = tool_result(&session::replay(session.path()).unwrap(), "subagent");
    assert!(output.starts_with("error:"), "{output}");
    assert!(output.contains("goal"), "{output}");
    // No child was created: the refusal happened before any session.
    assert!(session::list(&home)
        .unwrap()
        .iter()
        .all(|summary| summary.kind.is_none()));

    std::fs::remove_dir_all(&home).ok();
}

/// The child inherits the parent's declared skills: `use_skill` is in its
/// admitted scope, so a child that declares no skills would carry a tool
/// whose every call fails. The child's prompt and its `skills` record are
/// the parent's own, reconstructed in the child's log (hard rule 5).
#[tokio::test]
async fn a_child_inherits_the_parents_declared_skills() {
    let home = skills_home(
        "subagent-skills",
        "demo-skill",
        "Recover a failed bash call",
        "# demo\n\nread the error, fix one thing, rerun",
    );
    let mock = MockApi::start(vec![
        tool_call_stream(
            "call-1",
            "subagent",
            &delegation_arguments("recover from it"),
        ),
        tool_call_stream("child-1", "use_skill", r#"{"name":"demo-skill"}"#),
        assistant_stream("The skill is loaded; I will read the error first."),
        assistant_stream("the child loaded the skill"),
    ]);
    let mut session = Session::create(&home, "test-model").unwrap();
    run_with_skills(
        &mock,
        &mut session,
        fleet::skills::load(&skills_canon(&home)),
        "delegate the recovery",
    )
    .await
    .unwrap();

    let bodies = mock.bodies();
    assert_eq!(bodies.len(), 4, "{bodies:?}");
    // The child's system prompt carries the same name+description list the
    // parent's does, and still nothing of the body.
    let child_prompt = bodies[1]["messages"][0]["content"].as_str().unwrap();
    assert!(
        child_prompt.starts_with(fleet::agent::SYSTEM_PROMPT),
        "{child_prompt}"
    );
    assert!(child_prompt.contains("demo-skill"), "{child_prompt}");
    assert!(
        child_prompt.contains("- `demo-skill` \u{2014} use when recover a failed bash call"),
        "{child_prompt}"
    );
    assert!(!child_prompt.contains("fix one thing"), "{child_prompt}");
    // `use_skill` is declared to the child, and the body it fetches rides
    // the child's own tool_result path.
    assert!(declared_tools(&bodies[1]).contains(&"use_skill".to_string()));
    let child = child_summary(&home);
    let (body, approved) = tool_result(&session::replay(&child.path).unwrap(), "use_skill");
    assert!(approved, "{body}");
    assert_eq!(body, "# demo\n\nread the error, fix one thing, rerun");
    // The child's own log records the skills it declared, so its prompt
    // stays reconstructable from its log alone.
    let child_records = session::replay(&child.path).unwrap();
    assert!(
        child_records.iter().any(|record| matches!(
            record,
            Record::Skills { skills } if skills.iter().any(|skill| skill.name == "demo-skill")
        )),
        "{child_records:?}"
    );

    std::fs::remove_dir_all(&home).ok();
}

// --- compaction (slice 10) -------------------------------------------------

/// The messages one recorded request carried, as the wire type.
fn request_messages(mock: &MockApi, index: usize) -> Vec<Message> {
    mock.bodies()
        .remove(index)
        .get("messages")
        .and_then(|messages| messages.as_array())
        .expect("the request carried messages")
        .iter()
        .map(|message| serde_json::from_value(message.clone()).expect("one wire message"))
        .collect()
}

/// One full agent run whose progress sink the caller owns, so a test can
/// read the headless progress lines a compaction writes.
async fn run_capturing(
    mock: &MockApi,
    session: &mut Session,
    approver: Box<dyn Approver>,
    max_turns: usize,
    task: &str,
    progress: &mut Vec<u8>,
) -> Result<RunReport, AgentError> {
    let client = Client::new(mock.url(), "test-key");
    let options = AgentOptions {
        goal: None,
        model: "test-model".to_string(),
        identity: None,
        max_turns,
        ui_tx: None,
        lane_specs: None,
        calib: None,
        skills: Vec::new(),
        role_overlay: None,
        memory: None,
        scope: None,
        improve_home: None,
        route_home: None,
        gui_prompt: None,
        compact_now: false,
        checkpoints: None,
        file_access: FileAccess::Unrestricted,
        plan_mode: false,
    };
    let mut agent = Agent::new(&client, options, session, approver, progress).unwrap();
    agent.run(task).await
}

/// The records as they stood when the run's last request was built:
/// everything before the final answer's own record. A request is built
/// before the answer it provokes is recorded, so this is the slice of the
/// log a request's message list must equal.
fn before_final_answer(records: &[Record]) -> &[Record] {
    let cut = records
        .iter()
        .rposition(|record| {
            matches!(
                record,
                Record::Assistant { content, tool_calls, .. } if tool_calls.is_empty() && content.is_some()
            )
        })
        .expect("the final answer's record");
    &records[..cut]
}

/// The messages one compaction request replaced, read back off the log:
/// the records before the fold, split by the same tail function the agent
/// applied when it compacted. That the two splits agree is the whole
/// claim — the log's preceding records are the compaction request's
/// messages.
fn replaced_by(records: &[Record], last_turn: u64) -> Vec<Message> {
    let fold = records
        .iter()
        .position(|record| matches!(record, Record::Compaction { last_turn: named, .. } if *named == last_turn))
        .expect("the compaction record");
    let seeded = session::conversation(&records[..fold]);
    let cut = fleet::compaction::tail_start(&seeded.messages);
    assert!(cut > 0, "a fold replaces something");
    seeded.messages[..cut].to_vec()
}

/// A tool-calling turn that reports `prompt` prompt tokens — the shape a
/// provider sends when the conversation is genuinely large, which is the
/// only way a fold can be driven from the API's own count.
fn counting_turn(index: u32, prompt: u64) -> ScriptedResponse {
    tool_call_stream_counting(
        &format!("call-{index}"),
        "bash",
        &format!(r#"{{"command":"echo {index}"}}"#),
        prompt,
    )
}

/// Five ordinary tool-calling turns, then a sixth whose API-reported
/// prompt count crosses the budget, then the compaction request, then the
/// answer: the script a run that outgrows the window walks. Six turns is
/// the first point a fold has anything to replace — the tail is four, so
/// the fold lands between turn 6 and the next request and replaces
/// turns 1 through 2.
fn crossing_script(summary: &str) -> Vec<ScriptedResponse> {
    let mut script: Vec<ScriptedResponse> = (1..=5)
        .map(|index| {
            tool_call_stream(
                &format!("call-{index}"),
                "bash",
                &format!(r#"{{"command":"echo {index}"}}"#),
            )
        })
        .collect();
    script.push(counting_turn(6, 120_000));
    // The compaction request: its answer is the summary.
    script.push(assistant_stream(summary));
    // The turn after the fold.
    script.push(assistant_stream(TOOL_REPORT));
    script
}

#[tokio::test]
async fn a_conversation_past_the_budget_compacts_and_the_next_request_carries_the_summary() {
    // The acceptance shape end to end: the API's own count crosses the
    // budget, the fold happens, and the request that follows carries the
    // framed summary plus the verbatim tail.
    let home = temp_home("compaction-e2e");
    let mut session = Session::create(&home, "test-model").unwrap();
    let mock = MockApi::start(crossing_script("- the summary of the early turns"));
    let mut progress: Vec<u8> = Vec::new();
    let report = run_capturing(
        &mock,
        &mut session,
        Box::new(ApproveAll),
        20,
        "the task",
        &mut progress,
    )
    .await
    .unwrap();
    assert_eq!(report.stop_reason, StopReason::Completed);
    let written = String::from_utf8(std::mem::take(&mut progress)).unwrap();

    // The headless progress line.
    assert!(
        written.contains("compacted: 2 turns → 1 summary"),
        "{written}"
    );
    assert!(written.contains("~120000 → ~"), "{written}");

    // Eight requests: six turns, the compaction, and the final answer.
    let bodies = mock.bodies();
    assert_eq!(bodies.len(), 8, "{} requests", bodies.len());

    // The compaction request itself: the system prompt, the messages being
    // replaced, and the compaction instruction — and no tools, because a
    // compaction request has nothing to execute.
    let compaction_request = request_messages(&mock, 6);
    assert_eq!(
        compaction_request.len(),
        1 + 5 + 1,
        "{compaction_request:?}"
    );
    assert_eq!(compaction_request[0].role, Role::System);
    assert_eq!(compaction_request[1].role, Role::User);
    assert_eq!(compaction_request[1].content.as_deref(), Some("the task"));
    assert_eq!(
        compaction_request[compaction_request.len() - 1]
            .content
            .as_deref(),
        Some(fleet::compaction::COMPACTION_INSTRUCTION)
    );
    assert!(
        bodies[6].get("tools").is_none(),
        "a compaction request carries no tools"
    );

    // The next request carries the summary and the verbatim tail.
    let after = request_messages(&mock, 7);
    assert_eq!(after.len(), 1 + 1 + 4 * 2, "{after:?}");
    assert_eq!(after[0].role, Role::System);
    assert_eq!(
        after[1].content.as_deref(),
        Some(
            "Summary of turns 1 through 2, compacted to fit the window:\n\n- the summary of the early turns"
        )
    );
    // The tail opens on an assistant message, so the sequence stays valid
    // for the API (KI-016's failure in a new shape), and it holds the last
    // four turns' calls and results verbatim.
    assert_eq!(after[2].role, Role::Assistant);
    assert_eq!(after[2].tool_calls[0].id, "call-3");
    assert_eq!(after[3].role, Role::Tool);
    assert_eq!(after[3].tool_call_id.as_deref(), Some("call-3"));
    assert_eq!(
        after[after.len() - 1].content.as_deref(),
        Some("exit 0\n--- stdout ---\n6\n")
    );

    // The log reconstructs that request byte for byte: replay's message
    // list is exactly what was sent after the system prompt.
    let records = session::replay(session.path()).unwrap();
    let replayed = session::messages(before_final_answer(&records));
    assert_eq!(replayed, after[1..].to_vec(), "replay equals what was sent");

    // The compaction record, with its range and its counts.
    match records
        .iter()
        .find(|record| matches!(record, Record::Compaction { .. }))
    {
        Some(Record::Compaction {
            summary,
            first_turn,
            last_turn,
            tokens_before,
            tokens_after,
        }) => {
            assert_eq!(summary, "- the summary of the early turns");
            assert_eq!(*first_turn, 1);
            assert_eq!(*last_turn, 2);
            assert_eq!(*tokens_before, 120_000);
            assert!(*tokens_after > 0 && *tokens_after < *tokens_before);
        }
        other => panic!("expected a compaction record, got {other:?}"),
    }
    assert!(records
        .iter()
        .any(|record| matches!(record, Record::End { .. })));
    // And the session lists as compacted.
    assert!(session::list(&home).unwrap()[0].compacted);
    std::fs::remove_dir_all(&home).ok();
}

#[tokio::test]
async fn the_measure_is_the_apis_own_count_when_it_reports_one() {
    // A conversation whose chars/4 estimate is far past the budget while
    // the API reports a prompt that fits: no fold. The two paths are
    // separated by scripting the usage, not by fudging the messages.
    let home = temp_home("compaction-api-measure");
    let mut session = Session::create(&home, "test-model").unwrap();
    let big = "x".repeat(400_000);
    let mock = MockApi::start(vec![
        tool_call_stream("call-1", "bash", &format!(r#"{{"command":"echo {big}"}}"#)),
        assistant_stream(TOOL_REPORT),
    ]);
    let mut progress: Vec<u8> = Vec::new();
    let report = run_capturing(
        &mock,
        &mut session,
        Box::new(ApproveAll),
        5,
        "the task",
        &mut progress,
    )
    .await
    .unwrap();
    assert_eq!(report.stop_reason, StopReason::Completed);
    // Two requests only: no compaction request was made.
    assert_eq!(mock.bodies().len(), 2);
    let written = String::from_utf8(std::mem::take(&mut progress)).unwrap();
    assert!(!written.contains("compacted"), "{written}");
    assert!(!session::replay(session.path())
        .unwrap()
        .iter()
        .any(|record| matches!(record, Record::Compaction { .. })));
    std::fs::remove_dir_all(&home).ok();
}

#[tokio::test]
async fn the_measure_falls_back_to_the_estimate_when_usage_is_absent() {
    // The same shape with the usage frames removed: the estimate is what
    // crosses, and the fold happens. Five uncounted turns carry enough
    // characters for the estimate — the messages plus the assembled
    // system prompt, KI-081's correction — to pass the budget on its
    // own, and five assistant turns is the first point a fold has
    // something to replace: the tail is four turns and the fold replaces
    // turn 1. The payload is sized on both sides — turns 1-4 stay under
    // the floor so the crossing lands at turn 5 with a foldable range,
    // and the tail plus the prompt still fits after the fold. A
    // conversation past either bound is KI-080's named failure, not a
    // fold, which is what a larger payload here correctly hits.
    let home = temp_home("compaction-estimate-measure");
    let mut session = Session::create(&home, "test-model").unwrap();
    let big = "x".repeat(48_000);
    let mut script: Vec<ScriptedResponse> = (1..=5)
        .map(|index| {
            tool_call_stream_uncounted(
                &format!("call-{index}"),
                "bash",
                &format!(r#"{{"command":"echo {big}"}}"#),
            )
        })
        .collect();
    // The compaction request, itself uncounted, then the final answer.
    script.push(stream(vec![
        content_delta("- the estimated summary"),
        finish_frame("stop"),
        "data: [DONE]".to_string(),
    ]));
    script.push(assistant_stream(TOOL_REPORT));
    let mock = MockApi::start(script);
    let mut progress: Vec<u8> = Vec::new();
    let report = run_capturing(
        &mock,
        &mut session,
        Box::new(ApproveAll),
        20,
        "the task",
        &mut progress,
    )
    .await
    .unwrap();
    assert_eq!(report.stop_reason, StopReason::Completed);
    let written = String::from_utf8(std::mem::take(&mut progress)).unwrap();
    assert!(
        written.contains("compacted: 1 turn → 1 summary"),
        "{written}"
    );
    // Seven requests: five turns, the compaction, and the final answer.
    assert_eq!(mock.bodies().len(), 7);
    // The record's `tokens_before` is the estimate, which is what the
    // absence of usage forces — and it is at least the budget, because
    // that is what made it fire.
    match session::replay(session.path())
        .unwrap()
        .iter()
        .find(|record| matches!(record, Record::Compaction { .. }))
    {
        Some(Record::Compaction { tokens_before, .. }) => {
            assert!(*tokens_before >= fleet::compaction::COMPACTION_TOKEN_BUDGET);
        }
        other => panic!("expected a compaction record, got {other:?}"),
    }
    // And the request after the fold is the compacted shape.
    let after = request_messages(&mock, 6);
    assert_eq!(after.len(), 1 + 1 + 4 * 2, "{after:?}");
    let records = session::replay(session.path()).unwrap();
    assert_eq!(
        session::messages(before_final_answer(&records)),
        after[1..].to_vec(),
        "replay equals what was sent"
    );
    std::fs::remove_dir_all(&home).ok();
}

#[tokio::test]
async fn a_failed_compaction_request_lets_the_run_continue_un_compacted() {
    // The fold is an auxiliary call: its failure must not kill a healthy
    // run. The conversation keeps its context, the failure is named on
    // one line, the latch holds so a failing fold is not retried every
    // turn, and the run finishes.
    let home = temp_home("compaction-fold-failure");
    let mut session = Session::create(&home, "test-model").unwrap();
    let mut script = crossing_script("- the summary");
    // The compaction request — the seventh response — fails.
    script[6] = ScriptedResponse {
        status: "500 Internal Server Error".to_string(),
        body: r#"{"error":{"message":"the fold failed"}}"#.to_string(),
        sse: Vec::new(),
        delay_ms: 0,
    };
    let mock = MockApi::start(script);
    let mut progress: Vec<u8> = Vec::new();
    let report = run_capturing(
        &mock,
        &mut session,
        Box::new(ApproveAll),
        20,
        "the task",
        &mut progress,
    )
    .await
    .unwrap();
    assert_eq!(report.stop_reason, StopReason::Completed);

    // No compaction happened, and the log records none that did not.
    let records = session::replay(session.path()).unwrap();
    assert_eq!(
        records
            .iter()
            .filter(|record| matches!(record, Record::Compaction { .. }))
            .count(),
        0,
        "{records:?}"
    );
    let written = String::from_utf8(std::mem::take(&mut progress)).unwrap();
    assert!(written.contains("compaction failed"), "{written}");
    assert!(written.contains("un-compacted"), "{written}");
    // The failed fold was attempted exactly once: the run's requests are
    // the six turns, the failed fold, and the final answer — no retry.
    assert_eq!(mock.bodies().len(), 8, "{}", mock.bodies().len());
    // And the final request is the whole conversation, un-folded: no
    // summary frame rides in it.
    let after = request_messages(&mock, 7);
    assert!(
        !after.iter().any(|message| message
            .content
            .as_deref()
            .is_some_and(|content| content.contains("Summary of turns"))),
        "{after:?}"
    );
}

#[tokio::test]
async fn one_compaction_per_crossing_and_the_tool_loop_survives_it() {
    // The turns after a fold do not fold again, and the calls after it
    // still execute and log.
    let home = temp_home("compaction-once");
    let mut session = Session::create(&home, "test-model").unwrap();
    let mut script = crossing_script("- the summary");
    // Two more tool-calling turns after the fold, still reporting a count
    // past the budget: neither of them folds again.
    script.pop();
    script.push(counting_turn(7, 130_000));
    script.push(counting_turn(8, 140_000));
    script.push(assistant_stream(TOOL_REPORT));
    let mock = MockApi::start(script);
    let mut progress: Vec<u8> = Vec::new();
    let report = run_capturing(
        &mock,
        &mut session,
        Box::new(ApproveAll),
        20,
        "the task",
        &mut progress,
    )
    .await
    .unwrap();
    assert_eq!(report.stop_reason, StopReason::Completed);

    let records = session::replay(session.path()).unwrap();
    assert_eq!(
        records
            .iter()
            .filter(|record| matches!(record, Record::Compaction { .. }))
            .count(),
        1,
        "{records:?}"
    );
    let written = String::from_utf8(std::mem::take(&mut progress)).unwrap();
    assert_eq!(written.matches("compacted:").count(), 1, "{written}");
    // The tool loop's integrity across the fold: every call the model made
    // executed and logged, before and after.
    assert_eq!(
        records
            .iter()
            .filter(|record| matches!(record, Record::ToolResult { .. }))
            .count(),
        8
    );
    // The final request carries the summary, the tail's four turns, and
    // the two post-fold turns — the fold is not repeated.
    let bodies = mock.bodies();
    assert_eq!(bodies.len(), 10);
    let after = request_messages(&mock, 9);
    assert_eq!(after.len(), 1 + 1 + 4 * 2 + 2 * 2, "{after:?}");
    assert_eq!(
        after[after.len() - 1].content.as_deref(),
        Some("exit 0\n--- stdout ---\n8\n")
    );
    assert_eq!(
        session::messages(before_final_answer(&records)),
        after[1..].to_vec(),
        "replay equals what was sent"
    );
    std::fs::remove_dir_all(&home).ok();
}

#[tokio::test]
async fn a_compacted_session_resumes_from_disk_and_continues_coherently() {
    // The resumed run's first request carries the summary the earlier run
    // logged, and its turns continue the log's own numbering.
    let home = temp_home("compaction-resume");
    let mut session = Session::create(&home, "test-model").unwrap();
    let id = session.id().to_string();
    let mock = MockApi::start(crossing_script("- the summary of the early turns"));
    let mut progress: Vec<u8> = Vec::new();
    let report = run_capturing(
        &mock,
        &mut session,
        Box::new(ApproveAll),
        20,
        "the task",
        &mut progress,
    )
    .await
    .unwrap();
    assert_eq!(report.stop_reason, StopReason::Completed);
    assert!(String::from_utf8(std::mem::take(&mut progress))
        .unwrap()
        .contains("compacted:"));
    drop(mock);

    // Resume from disk and ask for more.
    let path = session::session_path(&home, &id);
    let mut resumed = Session::open(&path).unwrap();
    let mock = MockApi::start(vec![assistant_stream("carried on coherently")]);
    let mut progress: Vec<u8> = Vec::new();
    let report = run_capturing(
        &mock,
        &mut resumed,
        Box::new(ApproveAll),
        20,
        "keep going",
        &mut progress,
    )
    .await
    .unwrap();
    assert_eq!(report.stop_reason, StopReason::Completed);
    assert_eq!(
        report.final_message.as_deref(),
        Some("carried on coherently")
    );
    // The resumed run is well under the budget already, so it folds
    // nothing: one compaction in the log's life, not one per resume.
    let written = String::from_utf8(std::mem::take(&mut progress)).unwrap();
    assert!(!written.contains("compacted"), "{written}");

    // The first request of the resumed run carries the summary, the tail,
    // the earlier run's own final answer, and the new task after it —
    // which is what "continues coherently" means: nothing the earlier run
    // said is lost, and nothing it folded is repeated.
    let first = request_messages(&mock, 0);
    assert_eq!(first.len(), 1 + 1 + 4 * 2 + 2, "{first:?}");
    assert_eq!(
        first[1].content.as_deref(),
        Some(
            "Summary of turns 1 through 2, compacted to fit the window:\n\n- the summary of the early turns"
        )
    );
    assert_eq!(first[first.len() - 2].content.as_deref(), Some(TOOL_REPORT));
    assert_eq!(
        first[first.len() - 1].content.as_deref(),
        Some("keep going")
    );

    // The log still holds exactly one compaction, and its turn numbering
    // continues from the log's own last answer: the fold's turn, the
    // answer after it, and the resumed run's own turn.
    let records = session::replay(&path).unwrap();
    assert_eq!(
        records
            .iter()
            .filter(|record| matches!(record, Record::Compaction { .. }))
            .count(),
        1,
        "{records:?}"
    );
    let conversation = session::conversation(&records);
    assert_eq!(conversation.next_turn, 9);
    std::fs::remove_dir_all(&home).ok();
}

#[tokio::test]
async fn an_empty_summary_skips_the_fold_rather_than_dropping_the_conversation() {
    // A compaction request that answers with nothing is a fold that would
    // replace the conversation with nothing. It is skipped: the run keeps
    // its context, no record is written, and the next request is the one
    // it would have been anyway.
    let home = temp_home("compaction-empty-summary");
    let mut session = Session::create(&home, "test-model").unwrap();
    let mut script = crossing_script("");
    script.pop();
    script.push(assistant_stream(TOOL_REPORT));
    let mock = MockApi::start(script);
    let mut progress: Vec<u8> = Vec::new();
    let report = run_capturing(
        &mock,
        &mut session,
        Box::new(ApproveAll),
        20,
        "the task",
        &mut progress,
    )
    .await
    .unwrap();
    assert_eq!(report.stop_reason, StopReason::Completed);
    let written = String::from_utf8(std::mem::take(&mut progress)).unwrap();
    assert!(written.contains("compaction skipped"), "{written}");
    assert!(!written.contains("compacted:"), "{written}");
    let records = session::replay(session.path()).unwrap();
    assert!(
        !records
            .iter()
            .any(|record| matches!(record, Record::Compaction { .. })),
        "{records:?}"
    );
    // The final request still carries the whole conversation.
    let after = request_messages(&mock, 7);
    assert_eq!(after.len(), 1 + 1 + 6 * 2, "{after:?}");
    std::fs::remove_dir_all(&home).ok();
}

#[tokio::test]
async fn a_run_under_the_budget_logs_no_compaction_record() {
    // The ordinary run, byte for byte: the record is additive, so a log
    // without one is the log it has always been.
    let home = temp_home("compaction-ordinary");
    let mut session = Session::create(&home, "test-model").unwrap();
    let mock = MockApi::start(vec![
        tool_call_stream("call-1", "bash", r#"{"command":"echo hello"}"#),
        assistant_stream(TOOL_REPORT),
    ]);
    let mut progress = Vec::new();
    let report = run_on(&mock, &mut session, Box::new(ApproveAll), 5, "say hello")
        .await
        .unwrap();
    assert_eq!(report.stop_reason, StopReason::Completed);
    let written = String::from_utf8(std::mem::take(&mut progress)).unwrap();
    assert!(!written.contains("compacted"), "{written}");
    assert!(!written.contains("compaction skipped"), "{written}");
    // Two requests, the log's own shape, and no compaction marker.
    assert_eq!(mock.bodies().len(), 2);
    let records = session::replay(session.path()).unwrap();
    assert_eq!(records.len(), 8, "{records:?}");
    assert!(!session::list(&home).unwrap()[0].compacted);
    // Every message the second request carried is reconstructable from the
    // log, and the log holds the final answer after it.
    let second = request_messages(&mock, 1);
    let replayed = session::messages(&records);
    assert!(replayed.starts_with(&second[1..]), "{replayed:?}");
    std::fs::remove_dir_all(&home).ok();
}

#[tokio::test]
async fn the_compaction_request_is_reconstructable_from_the_log() {
    // Hard rule 5, applied to the compaction request itself: the messages
    // it carried are the records before the compaction record, the system
    // prompt is the compiled-in constant, and the instruction is the
    // compiled-in constant beside it.
    let home = temp_home("compaction-reconstructable");
    let mut session = Session::create(&home, "test-model").unwrap();
    let mock = MockApi::start(crossing_script("- the summary"));
    let mut progress: Vec<u8> = Vec::new();
    let report = run_capturing(
        &mock,
        &mut session,
        Box::new(ApproveAll),
        20,
        "the task",
        &mut progress,
    )
    .await
    .unwrap();
    assert_eq!(report.stop_reason, StopReason::Completed);
    assert!(String::from_utf8(std::mem::take(&mut progress))
        .unwrap()
        .contains("compacted:"));
    let records = session::replay(session.path()).unwrap();
    let fold = records
        .iter()
        .position(|record| matches!(record, Record::Compaction { .. }))
        .expect("the compaction record");
    let Record::Compaction {
        first_turn,
        last_turn,
        ..
    } = &records[fold]
    else {
        unreachable!()
    };
    // The records before the fold, split by the same tail function the
    // agent applied, are exactly the messages the request replaced.
    let replaced = replaced_by(&records, *last_turn);
    let wanted = request_messages(&mock, 6);
    assert_eq!(replaced.len() + 2, wanted.len(), "{replaced:?}");
    assert_eq!(
        replaced,
        wanted[1..wanted.len() - 1].to_vec(),
        "the log's preceding records are the compaction request's messages"
    );
    assert_eq!(
        wanted[0].content.as_deref(),
        Some(fleet::agent::default_prompt().as_str()),
        "the fold carries the run's own prompt, the reporting contract included"
    );
    assert_eq!(
        wanted[wanted.len() - 1].content.as_deref(),
        Some(fleet::compaction::COMPACTION_INSTRUCTION)
    );
    // And the range the record names is the range the log holds: five
    // messages spanning turns 1 and 2.
    assert_eq!(*first_turn, 1);
    assert_eq!(*last_turn, 2);
    assert_eq!(replaced.len(), 5);
    std::fs::remove_dir_all(&home).ok();
}

// --- scheduled runs (slice 11) ----------------------------------------------

/// One scheduled run's scheduler: the store under a scratch home, the
/// runner the pass fires through, and the router every fire's client is
/// built from. The runner is the production shape with the route
/// preflight taken out — `cron::execute` over a mock-routed client — so a
/// whole scheduler pass is driven deterministically against the scripted
/// API, with no sys1 spawn and no clock.
struct CronHarness {
    home: PathBuf,
    router: Arc<Router>,
    /// The mock outlives the harness's own scope: it is dropped with the
    /// harness, which kills the child and removes its scratch.
    mock: MockApi,
}

impl CronHarness {
    fn start(home_tag: &str, responses: Vec<ScriptedResponse>) -> CronHarness {
        let home = temp_home(home_tag);
        let mock = MockApi::start(responses);
        let resolved = vec![resolved("cand-mock", mock.url(), false, None)];
        let decision = RouteDecision {
            provider: "cand-mock".to_string(),
            model: "test-model".to_string(),
            how: How::Pinned,
            workload: None,
            verdict: None,
            execution: None,
            reason: "the mock is the only candidate".to_string(),
            judged_from: fleet::route::JudgedFrom::None,
            start_index: 0,
            order: vec![("cand-mock".to_string(), "test-model".to_string())],
        };
        CronHarness {
            home,
            router: Arc::new(Router::new(resolved, decision)),
            mock,
        }
    }

    /// A runner that executes a fire the way production does, minus the
    /// route preflight.
    fn runner(&self) -> fleet::cron::Runner {
        let home = self.home.clone();
        let router = self.router.clone();
        // The corpus root is the scratch home's own, deliberately empty:
        // the integration tests run with the crate root as their working
        // directory, where the repo's own git-tracked `memories/` corpus
        // resolves — and a scheduled run's recall hits would otherwise
        // write into it, dirtying the tree the gate is meant to keep
        // clean. `execute` takes the root as an input for exactly this.
        let memory_root = home.join("memories");
        std::fs::create_dir_all(&memory_root).unwrap();
        Box::new(move |job, minute, catch_up| {
            let home = home.clone();
            let router = router.clone();
            let memory_root = memory_root.clone();
            Box::pin(async move {
                let client = Client::routed(router);
                fleet::cron::execute(&home, &client, None, &job, minute, catch_up, &memory_root)
                    .await
                    .map_err(|error| error.to_string())
            })
        })
    }

    /// Write one job into the store directly, the way `fleet cron add`
    /// leaves it.
    fn seed_job(&self, name: &str, schedule: &str, last_run: Option<&str>, auto_approve: bool) {
        let mut store = fleet::cron::Store::open(&self.home).unwrap();
        store
            .add(name, schedule, "summarize the day", auto_approve)
            .unwrap();
        if let Some(stamp) = last_run {
            let minute = fleet::cron::Minute::parse(stamp).unwrap();
            store
                .record_run(name, minute, fleet::cron::Outcome::Completed)
                .unwrap();
        }
    }

    /// The session logs this home's scheduled runs wrote, oldest first.
    /// A home whose runs never created the sessions directory reads as
    /// no sessions, not as an error.
    fn scheduled_sessions(&self) -> Vec<PathBuf> {
        let Ok(entries) = std::fs::read_dir(fleet::home::sessions_dir(&self.home)) else {
            return Vec::new();
        };
        let mut paths: Vec<PathBuf> = entries
            .map(|entry| entry.unwrap().path())
            .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("jsonl"))
            .collect();
        paths.sort();
        paths
    }

    fn store(&self) -> fleet::cron::Store {
        fleet::cron::Store::open(&self.home).unwrap()
    }

    /// The request bodies the mock recorded: one per scheduled run's
    /// turn, so a pass that ran nothing reached no provider.
    fn requests(&self) -> Vec<serde_json::Value> {
        self.mock.bodies()
    }
}

#[tokio::test]
async fn a_scheduled_task_fires_when_its_minute_matches_and_only_once() {
    let harness = CronHarness::start("cron-fires", vec![assistant_stream("the digest")]);
    harness.seed_job("every-minute", "* * * * *", None, true);
    let mut scheduler = fleet::cron::Scheduler::new(harness.home.clone(), harness.runner());
    let now = fleet::cron::Minute::parse("2026-10-14T12:00Z").unwrap();

    let lines = scheduler.tick(now).await;
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(lines[0].contains("job `every-minute`"), "{}", lines[0]);
    assert!(lines[0].contains("2026-10-14T12:00Z"), "{}", lines[0]);
    assert!(lines[0].contains("completed"), "{}", lines[0]);

    // The run happened in its own ordinary session log, and the store
    // recorded the minute and the outcome.
    let logs = harness.scheduled_sessions();
    assert_eq!(logs.len(), 1, "{logs:?}");
    let records = session::replay(&logs[0]).unwrap();
    let start = records
        .iter()
        .find_map(|record| match record {
            Record::Start { kind, .. } => Some(kind.clone()),
            _ => None,
        })
        .expect("the start record");
    assert_eq!(
        start.as_deref(),
        Some(fleet::session::SCHEDULED_KIND),
        "a scheduled run's log is an ordinary log marked kind: scheduled"
    );
    let task = records
        .iter()
        .find_map(|record| match record {
            Record::User { content } => Some(content.clone()),
            _ => None,
        })
        .expect("the framed task");
    assert!(task.starts_with("[scheduled run"), "{task}");
    assert!(
        task.contains("job `every-minute` at 2026-10-14T12:00Z"),
        "{task}"
    );
    assert!(task.ends_with("summarize the day"), "{task}");
    // The log is an ordinary session: `fleet sessions` lists it, marked
    // as a scheduled run — the retained output is reachable from the
    // surface that lists sessions, not only from the job's card.
    let listed = session::list(&harness.home).unwrap();
    assert_eq!(listed.len(), 1, "{listed:?}");
    assert_eq!(
        listed[0].kind.as_deref(),
        Some(fleet::session::SCHEDULED_KIND)
    );
    let job = harness.store().job("every-minute").unwrap().clone();
    assert_eq!(job.last_run.as_deref(), Some("2026-10-14T12:00Z"));
    assert_eq!(job.last_outcome, Some(fleet::cron::Outcome::Completed));

    // The latch: the same minute does not fire twice, and no second
    // session was written.
    let again = scheduler.tick(now).await;
    assert!(again.is_empty(), "{again:?}");
    assert_eq!(harness.scheduled_sessions().len(), 1);

    // A job added a moment ago is picked up by the next tick: the store
    // is re-read every time.
    harness.seed_job("second-job", "* * * * *", None, true);
    let next = fleet::cron::Minute::parse("2026-10-14T12:01Z").unwrap();
    let fired = scheduler.tick(next).await;
    assert_eq!(fired.len(), 2, "{fired:?}");
    assert_eq!(harness.scheduled_sessions().len(), 3);
    std::fs::remove_dir_all(&harness.home).ok();
}

#[tokio::test]
async fn a_missed_window_catches_up_once_at_startup_and_the_catch_up_is_named() {
    let harness = CronHarness::start("cron-catch-up", vec![assistant_stream("the digest")]);
    // The job last ran at 11:00 and its schedule is hourly at :00, so the
    // 12:00 window was missed while the process was down.
    harness.seed_job("hourly", "0 * * * *", Some("2026-10-14T11:00Z"), true);
    let mut scheduler = fleet::cron::Scheduler::new(harness.home.clone(), harness.runner());

    let lines = scheduler
        .start(fleet::cron::Minute::parse("2026-10-14T12:30Z").unwrap())
        .await;
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(
        lines[0].contains("catch-up for 2026-10-14T12:00Z"),
        "{}",
        lines[0]
    );

    // Exactly one run, and its own log names the window it served.
    let logs = harness.scheduled_sessions();
    assert_eq!(logs.len(), 1, "{logs:?}");
    let records = session::replay(&logs[0]).unwrap();
    let task = records
        .iter()
        .find_map(|record| match record {
            Record::User { content } => Some(content.clone()),
            _ => None,
        })
        .expect("the framed task");
    assert!(task.contains("catch-up for job `hourly`"), "{task}");
    assert!(
        task.contains("the 2026-10-14T12:00Z window was missed while fleet was down"),
        "{task}"
    );
    let job = harness.store().job("hourly").unwrap().clone();
    assert_eq!(job.last_run.as_deref(), Some("2026-10-14T12:00Z"));

    // A second startup pass serves nothing: the window is recorded.
    assert!(scheduler
        .start(fleet::cron::Minute::parse("2026-10-14T12:31Z").unwrap())
        .await
        .is_empty());
    assert_eq!(harness.scheduled_sessions().len(), 1);
    std::fs::remove_dir_all(&harness.home).ok();
}

#[tokio::test]
async fn a_job_that_has_never_run_does_not_catch_up() {
    let harness = CronHarness::start("cron-never-run", vec![assistant_stream("the digest")]);
    // No last_run at all: the startup pass serves nothing, and the job's
    // first fire is its first matching minute going forward.
    harness.seed_job("hourly", "0 * * * *", None, true);
    let mut scheduler = fleet::cron::Scheduler::new(harness.home.clone(), harness.runner());

    assert!(scheduler
        .start(fleet::cron::Minute::parse("2026-10-14T12:30Z").unwrap())
        .await
        .is_empty());
    assert!(harness.scheduled_sessions().is_empty());
    // Nothing reached the provider either: a startup pass that serves
    // nothing spends nothing.
    assert!(harness.requests().is_empty());
    // The tick is the path that starts it, at the matching minute.
    let fired = scheduler
        .tick(fleet::cron::Minute::parse("2026-10-14T13:00Z").unwrap())
        .await;
    assert_eq!(fired.len(), 1, "{fired:?}");
    assert!(!fired[0].contains("catch-up"), "{}", fired[0]);
    assert_eq!(harness.scheduled_sessions().len(), 1);
    assert_eq!(harness.requests().len(), 1);
    std::fs::remove_dir_all(&harness.home).ok();
}

#[tokio::test]
async fn a_job_that_is_not_declared_unattended_skips_its_tool_call_and_names_it() {
    // The model reaches for a tool; the job is not declared
    // auto-approve; nobody is there to approve it.
    let harness = CronHarness::start(
        "cron-needs-approval",
        vec![
            tool_call_stream("call-1", "bash", "{\"command\":\"ls\"}"),
            assistant_stream("done without the tool"),
        ],
    );
    harness.seed_job("careful", "* * * * *", None, false);
    let mut scheduler = fleet::cron::Scheduler::new(harness.home.clone(), harness.runner());

    let lines = scheduler
        .tick(fleet::cron::Minute::parse("2026-10-14T12:00Z").unwrap())
        .await;
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(lines[0].contains("skipped `bash`"), "{}", lines[0]);
    assert!(
        lines[0].contains("no one is here to approve it"),
        "{}",
        lines[0]
    );

    // The call never ran: the log carries the denial, and the outcome
    // records the skip.
    let logs = harness.scheduled_sessions();
    assert_eq!(logs.len(), 1, "{logs:?}");
    let records = session::replay(&logs[0]).unwrap();
    let denied = records
        .iter()
        .find_map(|record| match record {
            Record::ToolResult { name, approved, .. } if name == "bash" => Some(*approved),
            _ => None,
        })
        .expect("the denied call's result");
    assert!(!denied, "the unattended gate denied the call");
    // The denial the model reads names the job's policy, not a user who
    // is guaranteed absent: this gate denies precisely because nobody
    // is there (KI-088). It rides the ordinary tool_result path into
    // the log, so model-visible stays model-visible.
    let denial = records
        .iter()
        .find_map(|record| match record {
            Record::ToolResult { name, output, .. } if name == "bash" => Some(output.clone()),
            _ => None,
        })
        .expect("the denied call's output");
    assert!(denial.contains("not declared unattended"), "{denial}");
    assert!(denial.contains("auto_approve"), "{denial}");
    assert!(!denial.contains("the user did not approve"), "{denial}");
    let job = harness.store().job("careful").unwrap().clone();
    assert_eq!(
        job.last_outcome,
        Some(fleet::cron::Outcome::SkippedNeedsApproval)
    );
    assert_eq!(job.last_run.as_deref(), Some("2026-10-14T12:00Z"));
    std::fs::remove_dir_all(&harness.home).ok();
}

#[tokio::test]
async fn a_job_declared_unattended_lets_its_calls_run() {
    let harness = CronHarness::start(
        "cron-auto-approve",
        vec![
            tool_call_stream("call-1", "list_dir", "{\"path\":\".\"}"),
            assistant_stream("listed"),
        ],
    );
    harness.seed_job("trusted", "* * * * *", None, true);
    let mut scheduler = fleet::cron::Scheduler::new(harness.home.clone(), harness.runner());

    let lines = scheduler
        .tick(fleet::cron::Minute::parse("2026-10-14T12:00Z").unwrap())
        .await;
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(lines[0].contains("completed"), "{}", lines[0]);
    let logs = harness.scheduled_sessions();
    assert_eq!(logs.len(), 1, "{logs:?}");
    let records = session::replay(&logs[0]).unwrap();
    let approved = records
        .iter()
        .find_map(|record| match record {
            Record::ToolResult { name, approved, .. } if name == "list_dir" => Some(*approved),
            _ => None,
        })
        .expect("the approved call's result");
    assert!(approved, "the declared-unattended gate let the call run");
    let job = harness.store().job("trusted").unwrap().clone();
    assert_eq!(job.last_outcome, Some(fleet::cron::Outcome::Completed));
    std::fs::remove_dir_all(&harness.home).ok();
}

// --- the MCP registry's run wiring ---------------------------------------

/// The registry's enabled servers are declared to the run from the FIRST
/// request, not from the first call (the registry plan's D2 / C2): the
/// pre-join happens before the turn loop, its `lane_tools` record is in
/// the log with the mapping, and the call itself answers bounded through
/// the same seam the lanes cross.
#[tokio::test]
async fn a_registry_server_declares_from_the_first_request() {
    let fake = FakeMcp::start(vec![
        serde_json::json!({ "reply": initialize_reply() }),
        serde_json::json!({ "reply": tools_list_reply(serde_json::json!([
            { "name": "embed", "description": "an embed", "inputSchema": { "type": "object" } },
        ])) }),
        serde_json::json!({ "reply": call_reply("embedded.") }),
    ]);
    // The registry names the server; its env name resolves through the
    // home's own .env, the chain the lanes use — never the process env,
    // which the test cannot set for a parallel suite.
    let home = temp_home("registry-run");
    let entry = serde_json::json!({
        "program": env!("CARGO_BIN_EXE_mock-mcp"),
        "args": [fake.scratch.join("script.jsonl").to_string_lossy().into_owned()],
        "env_names": ["FLEET_TEST_REGISTRY_KEY"],
        "enabled": true,
    });
    std::fs::write(
        home.join("mcp.json"),
        serde_json::to_string(&serde_json::json!({ "servers": { "regtime": entry } })).unwrap(),
    )
    .unwrap();
    std::fs::write(
        home.join(".env"),
        "FLEET_TEST_REGISTRY_KEY=from-home-dotenv\n",
    )
    .unwrap();

    let mock = MockApi::start(vec![
        tool_call_stream("call-1", "mcp__regtime__embed", r#"{"items":"x"}"#),
        assistant_stream("The registry server answered."),
    ]);
    let mut session = Session::create(&home, "test-model").unwrap();
    let mut progress = Vec::new();
    let client = Client::new(mock.url(), "test-key");
    let mut options = AgentOptions {
        goal: None,
        model: "test-model".to_string(),
        identity: None,
        max_turns: 5,
        ui_tx: None,
        lane_specs: None,
        calib: None,
        skills: Vec::new(),
        role_overlay: None,
        memory: None,
        scope: None,
        improve_home: None,
        route_home: None,
        gui_prompt: None,
        compact_now: false,
        checkpoints: None,
        file_access: FileAccess::Unrestricted,
        plan_mode: false,
    };
    let dotenv: std::collections::BTreeMap<String, String> =
        fleet::home::read_dotenv_with_process_vault(&home)
            .into_iter()
            .collect();
    options
        .seed_user_servers(&home, &dotenv)
        .expect("the registry seeds");
    {
        let mut agent = Agent::new(
            &client,
            options,
            &mut session,
            Box::new(ApproveAll),
            &mut progress,
        )
        .unwrap();
        agent.run("embed x").await.unwrap();
    }

    // The FIRST request carried the server's tool already — the pre-join
    // beat the turn loop.
    let bodies = mock.bodies();
    assert!(
        declared_tools(&bodies[0]).contains(&"mcp__regtime__embed".to_string()),
        "the first request declares the registry server: {:?}",
        declared_tools(&bodies[0])
    );
    // The answer crossed, bounded.
    let records = session::replay(session.path()).unwrap();
    let result = records.iter().find_map(|record| match record {
        Record::ToolResult { name, output, .. } if name == "mcp__regtime__embed" => {
            Some(output.clone())
        }
        _ => None,
    });
    assert_eq!(result.as_deref(), Some("embedded."));
    // The log carries the mapping, both ends of it.
    match records
        .iter()
        .find(|r| matches!(r, Record::LaneTools { .. }))
    {
        Some(Record::LaneTools { lane, tools }) => {
            assert_eq!(lane, "regtime");
            assert_eq!(tools.len(), 1);
            assert_eq!(tools[0].name, "mcp__regtime__embed");
            assert_eq!(tools[0].lane_tool, "embed");
        }
        other => panic!("no lane_tools record for the registry server: {other:?}"),
    }
    // The resolved env value crossed the spawn boundary (the lane's own
    // KI-031 guarantee, now for a user server): the scripted server's
    // env tool would name it. The registry's spec resolved it from the
    // home's .env, not the process.
    let stderr = String::from_utf8(progress).unwrap();
    assert!(
        stderr.contains("server regtime joined"),
        "the pre-join says so: {stderr}"
    );

    std::fs::remove_dir_all(&home).ok();
}

#[tokio::test]
async fn a_run_completes_over_a_registry_whose_server_is_unkeyed() {
    // C3 (the 2026-10-26 plan): the incident's own shape, pinned end
    // to end. One enabled registry server whose env resolves to
    // nothing: the run seeds, completes, and answers — and the
    // server's tools are simply not declared. Before the plan, this
    // registry refused the run before its memory load, which is how
    // one unkeyed imported server stopped every headless run on
    // 2026-10-25.
    let home = temp_home("registry-unkeyed-run");
    let fake = FakeMcp::start(vec![]);
    let entry = serde_json::json!({
        "program": env!("CARGO_BIN_EXE_mock-mcp"),
        "args": [fake.scratch.join("script.jsonl").to_string_lossy().into_owned()],
        "env_names": ["FLEET_TEST_REGISTRY_UNKEYED"],
        "enabled": true,
    });
    std::fs::write(
        home.join("mcp.json"),
        serde_json::to_string(&serde_json::json!({ "servers": { "regtime": entry } })).unwrap(),
    )
    .unwrap();
    // No `.env` line and no process env: the name resolves to
    // nothing, which is the whole point.

    let mock = MockApi::start(vec![assistant_stream("The run answered anyway.")]);
    let mut session = Session::create(&home, "test-model").unwrap();
    let mut progress = Vec::new();
    let client = Client::new(mock.url(), "test-key");
    let mut options = AgentOptions {
        goal: None,
        model: "test-model".to_string(),
        identity: None,
        max_turns: 5,
        ui_tx: None,
        lane_specs: None,
        calib: None,
        skills: Vec::new(),
        role_overlay: None,
        memory: None,
        scope: None,
        improve_home: None,
        route_home: None,
        gui_prompt: None,
        compact_now: false,
        checkpoints: None,
        file_access: FileAccess::Unrestricted,
        plan_mode: false,
    };
    let dotenv: std::collections::BTreeMap<String, String> =
        fleet::home::read_dotenv_with_process_vault(&home)
            .into_iter()
            .collect();
    // The seeding tolerates the unkeyed server and names it.
    let absent = options
        .seed_user_servers(&home, &dotenv)
        .expect("an unkeyed server is an absence, not a failed run");
    assert_eq!(absent.len(), 1, "{absent:?}");
    assert_eq!(absent[0].0, "regtime");
    {
        let mut agent = Agent::new(
            &client,
            options,
            &mut session,
            Box::new(ApproveAll),
            &mut progress,
        )
        .unwrap();
        agent.run("embed x").await.unwrap();
    }
    // The run's own answer landed, and the unkeyed server's tool was
    // never declared — its absence cost exactly its tools.
    let bodies = mock.bodies();
    assert!(
        !declared_tools(&bodies[0]).contains(&"mcp__regtime__embed".to_string()),
        "the unkeyed server's tools are not declared: {:?}",
        declared_tools(&bodies[0])
    );
    let report = std::fs::read_to_string(session.path()).unwrap();
    assert!(
        report.contains("The run answered anyway."),
        "the run completed: {report}"
    );
}

/// A server whose tools/list carries two tools of one short name
/// declares two DISTINCT names — the `unique_name` rule (C5) — so no two
/// declarations ever share a name, and both calls reach their tool.
#[tokio::test]
async fn colliding_tool_names_declare_distinct_names() {
    let fake = FakeMcp::start(vec![
        serde_json::json!({ "reply": initialize_reply() }),
        serde_json::json!({ "reply": tools_list_reply(serde_json::json!([
            { "name": "probe", "description": "first", "inputSchema": { "type": "object" } },
            { "name": "probe", "description": "second", "inputSchema": { "type": "object" } },
        ])) }),
        serde_json::json!({ "reply": call_reply("first.") }),
        serde_json::json!({ "reply": call_reply("second.") }),
    ]);
    let mut specs = std::collections::HashMap::new();
    specs.insert("alpha".to_string(), fake.spec());
    let mock = MockApi::start(vec![
        tool_call_stream("call-1", "mcp__alpha__probe", "{}"),
        tool_call_stream("call-2", "mcp__alpha__probe_2", "{}"),
        assistant_stream("Both answered."),
    ]);
    let home = temp_home("registry-collision");
    let mut session = Session::create(&home, "test-model").unwrap();
    let mut progress = Vec::new();
    run_with_lanes(
        &mock,
        &mut session,
        Box::new(ApproveAll),
        Some(specs),
        &mut progress,
        "probe twice",
    )
    .await
    .unwrap();
    let bodies = mock.bodies();
    let declared = declared_tools(&bodies[0]);
    assert!(
        declared.contains(&"mcp__alpha__probe".to_string())
            && declared.contains(&"mcp__alpha__probe_2".to_string()),
        "both tools declared, distinct: {declared:?}"
    );
    let records = session::replay(session.path()).unwrap();
    let outputs: Vec<String> = records
        .iter()
        .filter_map(|record| match record {
            Record::ToolResult { output, .. } => Some(output.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(outputs, vec!["first.", "second."]);
    std::fs::remove_dir_all(&home).ok();
}

/// A disabled server stays in the registry and out of the run: its name
/// is in `fleet mcp`, and its tools are in no request (C7).
#[tokio::test]
async fn a_disabled_server_is_absent_from_the_declarations() {
    let fake = FakeMcp::start(vec![serde_json::json!({ "reply": initialize_reply() })]);
    let home = temp_home("registry-disabled");
    let entry = serde_json::json!({
        "program": env!("CARGO_BIN_EXE_mock-mcp"),
        "args": [fake.scratch.join("script.jsonl").to_string_lossy().into_owned()],
        "env_names": [],
        "enabled": false,
    });
    std::fs::write(
        home.join("mcp.json"),
        serde_json::to_string(&serde_json::json!({ "servers": { "regtime": entry } })).unwrap(),
    )
    .unwrap();
    // The registry still lists it.
    let servers = fleet::mcp_registry::read_servers(&home).unwrap();
    assert_eq!(servers.len(), 1);
    assert!(!servers[0].1.enabled);

    let mock = MockApi::start(vec![assistant_stream("Nothing to call.")]);
    let mut session = Session::create(&home, "test-model").unwrap();
    let mut progress = Vec::new();
    let client = Client::new(mock.url(), "test-key");
    let mut options = AgentOptions {
        goal: None,
        model: "test-model".to_string(),
        identity: None,
        max_turns: 5,
        ui_tx: None,
        lane_specs: None,
        calib: None,
        skills: Vec::new(),
        role_overlay: None,
        memory: None,
        scope: None,
        improve_home: None,
        route_home: None,
        gui_prompt: None,
        compact_now: false,
        checkpoints: None,
        file_access: FileAccess::Unrestricted,
        plan_mode: false,
    };
    let dotenv: std::collections::BTreeMap<String, String> =
        fleet::home::read_dotenv_with_process_vault(&home)
            .into_iter()
            .collect();
    options
        .seed_user_servers(&home, &dotenv)
        .expect("the registry seeds");
    {
        let mut agent = Agent::new(
            &client,
            options,
            &mut session,
            Box::new(ApproveAll),
            &mut progress,
        )
        .unwrap();
        agent.run("anything").await.unwrap();
    }
    let bodies = mock.bodies();
    assert!(
        !declared_tools(&bodies[0])
            .iter()
            .any(|name| name.starts_with("mcp__regtime__")),
        "a disabled server declares nothing: {:?}",
        declared_tools(&bodies[0])
    );
    std::fs::remove_dir_all(&home).ok();
}

/// The health probe reports the server's self-reported name and version
/// and its tool count, writes the outcome into the registry's
/// `last_probe`, and a server that cannot start probes as failed with
/// the reason named (C6).
#[tokio::test]
async fn a_probe_reports_the_server_and_writes_its_outcome() {
    let fake = FakeMcp::start(vec![
        serde_json::json!({ "reply": initialize_reply() }),
        serde_json::json!({ "reply": tools_list_reply(serde_json::json!([
            { "name": "embed", "description": "an embed", "inputSchema": { "type": "object" } },
        ])) }),
    ]);
    let home = temp_home("registry-probe");
    fleet::settings::set_server(
        &home,
        "regtime",
        &serde_json::json!({
            "program": env!("CARGO_BIN_EXE_mock-mcp"),
            "args": [fake.scratch.join("script.jsonl").to_string_lossy().into_owned()],
            "env_names": [],
            "enabled": true,
        }),
    )
    .unwrap();
    let probe =
        fleet::mcp_registry::probe_server(&home, "regtime", &std::collections::BTreeMap::new())
            .await
            .expect("probes");
    assert!(probe.ok, "{probe:?}");
    assert_eq!(probe.server_name, "fake");
    assert_eq!(probe.server_version, "9.9.9");
    assert_eq!(probe.tools, 1);
    fleet::settings::record_server_probe(&home, "regtime", probe).expect("records");
    let (_, entry) = &fleet::mcp_registry::read_servers(&home).expect("reads")[0];
    let probe = entry.last_probe.as_ref().expect("the outcome rode");
    assert!(probe.ok);
    assert_eq!(probe.tools, 1);
    // A server that cannot start probes as failed, named.
    fleet::settings::set_server(
        &home,
        "broken",
        &serde_json::json!({
            "program": "/fleet/no-such-binary-here",
            "env_names": [],
            "enabled": true,
        }),
    )
    .unwrap();
    let probe =
        fleet::mcp_registry::probe_server(&home, "broken", &std::collections::BTreeMap::new())
            .await
            .expect("probes");
    assert!(!probe.ok);
    assert!(probe.reason.contains("could not start"), "{}", probe.reason);
    fleet::settings::record_server_probe(&home, "broken", probe).expect("records");
    let servers = fleet::mcp_registry::read_servers(&home).expect("reads");
    let broken = &servers.iter().find(|(name, _)| name == "broken").unwrap().1;
    assert!(!broken.last_probe.as_ref().expect("recorded").ok);
    // The cards carry the outcomes: one ok, one failed, never a table.
    let out = fleet::mcp_registry::render(&servers);
    assert!(out.contains("probed ok"), "{out}");
    assert!(out.contains("last probe failed"), "{out}");
    assert!(out.contains("fake v9.9.9"), "{out}");
    std::fs::remove_dir_all(&home).ok();
}

/// C10 (the context inspector): the reconstruction IS the request. A
/// scripted two-run conversation is captured body-for-body; the
/// reconstructor's document — built from the log alone, with the same
/// identity, goal, gui line, and file access the run carried — must be
/// byte-equal to the last captured request's messages and tools arrays,
/// and its system prompt equal to the first message's content.
#[tokio::test]
async fn what_the_model_saw_is_the_request_itself() {
    let mock = MockApi::start(vec![
        assistant_stream("first answer"),
        assistant_stream("second answer"),
    ]);
    let home = temp_home("context-reconstruct");
    let mut session = Session::create(&home, "test-model").unwrap();
    let client = Client::new(mock.url(), "test-key");
    let gui_line = fleet::gui::prompt_line("http://127.0.0.1:3080");
    let options = AgentOptions {
        goal: Some("ship the controls this week".to_string()),
        model: "test-model".to_string(),
        identity: None,
        max_turns: 5,
        ui_tx: None,
        lane_specs: None,
        calib: None,
        skills: Vec::new(),
        role_overlay: None,
        memory: None,
        scope: None,
        improve_home: None,
        route_home: None,
        gui_prompt: Some(gui_line.clone()),
        compact_now: false,
        checkpoints: None,
        file_access: FileAccess::Unrestricted,
        plan_mode: false,
    };
    {
        let mut progress = Vec::new();
        let mut agent = Agent::new(
            &client,
            options,
            &mut session,
            Box::new(ApproveAll),
            &mut progress,
        )
        .unwrap();
        agent.run("first ask").await.unwrap();
        // A second run in the same session: the conversation grows.
        agent.run("second ask").await.unwrap();
    }
    let bodies = mock.bodies();
    assert!(bodies.len() >= 2, "{}", bodies.len());
    let last = &bodies[bodies.len() - 1];
    let records = session::replay(session.path()).unwrap();
    let document = fleet::agent::Agent::reconstruct_document(
        &records,
        None,
        Some(gui_line.as_str()),
        &FileAccess::Unrestricted,
    );
    // The system prompt is the first message's content, byte for byte.
    assert_eq!(
        document["system"].as_str().unwrap(),
        last["messages"][0]["content"].as_str().unwrap(),
        "the system prompt is the request's own"
    );
    // The messages array is byte-equal.
    assert_eq!(
        serde_json::to_value(&document["messages"]).unwrap(),
        serde_json::to_value(&last["messages"]).unwrap(),
        "the messages are the request's own"
    );
    // The tools array is byte-equal.
    assert_eq!(
        serde_json::to_value(&document["tools"]).unwrap(),
        serde_json::to_value(&last["tools"]).unwrap(),
        "the tools are the request's own"
    );
    // The goal the run carried is in the reconstruction — the record is
    // what made it reconstructable.
    assert!(
        document["system"]
            .as_str()
            .unwrap()
            .contains("ship the controls this week"),
        "the goal record rode the prompt"
    );
    std::fs::remove_dir_all(&home).ok();
}

/// C11 (the re-run): a recorded bash call re-runs through the executor
/// with its bound and its result never lands in the session log; the
/// write class is refused by name.
#[tokio::test]
async fn a_recorded_call_re_runs_without_touching_the_log() {
    let home = temp_home("replay-tool");
    let session = Session::create(&home, "test-model").unwrap();
    let (bridge, _sources) =
        fleet::gui::bridge::Bridge::detached(home.clone(), "test-model".to_string(), None);
    let id = session
        .path()
        .file_stem()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    // The ask-mode first press stands the seat the run path would stand.
    let seat = bridge
        .replay_tool(&id, "bash", r#"{"command":"echo re-run"}"#)
        .await
        .expect("the ask class stands a seat, not a refusal");
    assert_eq!(seat["seat"].as_bool(), Some(true), "{seat}");
    assert_eq!(seat["logged"].as_bool(), Some(false));
    // Under a mode that approves bash without asking, the same press
    // executes through the executor.
    let _ = bridge.run_with(
        &id,
        "ignored",
        fleet::approvals::Mode::YesGuarded,
        None,
        Some("/tmp"),
    );
    let replayed = bridge
        .replay_tool(&id, "bash", r#"{"command":"echo re-run"}"#)
        .await
        .expect("re-runs");
    let said = replayed["output"].as_str().unwrap_or_default().to_string();
    assert!(said.contains("re-run"), "{said}");
    assert_eq!(replayed["logged"].as_bool(), Some(false));
    // The log gains no record for it.
    let records = session::replay(session.path()).unwrap();
    assert!(
        !records.iter().any(|record| matches!(
            record,
            Record::ToolResult { output, .. } if output.contains("re-run")
        )),
        "the re-run never entered the model's context"
    );
    // The write class is refused by name, with the correction path named.
    let refused = bridge
        .replay_tool(
            session
                .path()
                .file_stem()
                .unwrap()
                .to_string_lossy()
                .as_ref(),
            "write_file",
            r#"{"path":"x","content":"y"}"#,
        )
        .await
        .expect_err("refused");
    assert!(refused.contains("no checkpoint"), "{refused}");
    assert!(refused.contains("edit-and-resend"), "{refused}");
    std::fs::remove_dir_all(&home).ok();
}

// --- The delegated-workspaces arm -------------------------------------
// docs/plans/2026-10-25-layerfs-delegated-workspaces.md, C2/C3.
//
// The driver under test is the real one; the CLI is a stub whose mode
// and log ride the environment. The gate caches the ONE stub path per
// process, so the stub is written once and never deleted here — an
// earlier draft deleted it at cleanup and the next test's spawn hit a
// missing file, then panicked while holding AMBIENT_LOCK and poisoned
// the lock for every env test behind it. Two rules follow: the stub
// outlives the tests, and assertions run AFTER the lock scope closes
// so a failing test cannot poison a shared mutex.

/// The shared stub body: one script, env-driven behavior.
const LAYERFS_STUB: &str = r#"#!/bin/sh
printf '%s\n' "$*" >> "$FLEET_STUB_LOG"
case "$*" in
  *--version*) echo "1.1.0" ;;
  *"workspace mount"*)
    mkdir -p "$FLEET_TEST_MOUNT"
    printf '{"sandbox":"1","path":"%s","image":"sha256:x","ws":"1:%s"}\n' "$FLEET_TEST_MOUNT" "$FLEET_TEST_MOUNT" ;;
  *"branch merge"*)
    if [ "$FLEET_STUB_MODE" = "conflict" ]; then
      echo '{"code":"Conflict","message":"both changed src/lib.rs","detail":{"conflicts":[{"path":"src/lib.rs"}]}}' >&2
      exit 125
    fi
    echo '{"merge":"m-1"}' ;;
  *"sandbox create"*) echo '{"sandbox":"1","image":"sha256:x"}' ;;
  *"branch fork"*) echo '{"name":"ok"}' ;;
  *"workspace commit"*) echo '{"commit":"c-1"}' ;;
  *"workspace unmount"*|*"sandbox close"*|*"branch publish"*) echo '{}' ;;
  *"workspace exec"*) printf '%s\n' '{"kind":"exit:0","exitCode":0,"stdout":"ran\n","stderr":"","message":null,"truncated":false}' ;;
  *"fs read"*) printf '%s\n' '{"kind":"ok","text":"hello\n","startLine":1,"endLine":1,"truncated":false}' ;;
  *"fs list"*) echo '{"items":[],"hasMore":false}' ;;
  *"fs write"*)
    if [ -n "$FLEET_STUB_STDIN" ]; then cat > "$FLEET_STUB_STDIN"; else cat > /dev/null; fi
    echo '{"kind":"ok"}' ;;
  *"fs edit"*) echo '{"kind":"ok"}' ;;
  *"fs grep"*|*"fs glob"*) echo '{"items":[],"hasMore":false}' ;;
  *"branch diff_branches"*) printf '%s\n' '{"items":[{"path":"/home.txt","kind":"Modified"}],"hasMore":false}' ;;
  *) echo '{"code":"Usage","message":"unknown command"}' >&2; exit 2 ;;
esac
"#;

/// The process's one stub: written once (the gate's cached path), kept
/// for the process's life, its per-test argv log addressed by env.
fn shared_layerfs_stub() -> &'static std::path::Path {
    static STUB: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();
    STUB.get_or_init(|| {
        let dir = std::env::temp_dir().join(format!(
            "fleet-layerfs-it-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let script = dir.join("layerfs");
        std::fs::write(&script, LAYERFS_STUB).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        script
    })
    .as_path()
}

/// The seam plan's C1 pin — the coherence sequence a child's tools
/// walk over the stub: an `fs write` (no mount needed), then one
/// bash call's cadence (mount -> exec -> commit -> unmount), then an
/// `fs read` that sees the branch the commands checkpointed. The
/// child's answers render exactly as the host tools render (the
/// bash `exit N` shape, the numbered read), the parent reads its
/// branch and merge back, and the cadence's argv order is the plan's
/// claim made visible.
#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn a_layerfs_child_walks_the_coherence_sequence() {
    let mock = MockApi::start(vec![
        tool_call_stream(
            "call-1",
            "subagent",
            &delegation_arguments("write, run, and read in your workspace"),
        ),
        tool_call_stream(
            "child-w",
            "write_file",
            r#"{"path":"/notes.txt","content":"from the child\n"}"#,
        ),
        tool_call_stream("child-b", "bash", r#"{"command":"cat notes.txt"}"#),
        tool_call_stream("child-r", "read_file", r#"{"path":"/notes.txt"}"#),
        assistant_stream("the child is done"),
        assistant_stream("the parent wraps up"),
    ]);
    let home = temp_home("layerfs-seam-home");
    let mut session = Session::create(&home, "test-model").unwrap();
    let client = Client::new(mock.url(), "test-key");
    let options = AgentOptions {
        goal: None,
        model: "test-model".to_string(),
        identity: None,
        max_turns: 9,
        ui_tx: None,
        lane_specs: None,
        calib: None,
        skills: Vec::new(),
        role_overlay: None,
        memory: None,
        scope: None,
        improve_home: None,
        route_home: None,
        gui_prompt: None,
        compact_now: false,
        checkpoints: None,
        file_access: FileAccess::Unrestricted,
        plan_mode: false,
    };
    let argv =
        std::env::temp_dir().join(format!("fleet-layerfs-it-argv-seam-{}", std::process::id()));
    let _ = std::fs::remove_file(&argv);
    {
        let _lock = AMBIENT_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let _bin = AmbientKey::set("FLEET_LAYERFS_BIN", shared_layerfs_stub().to_str().unwrap());
        fleet::layerfs_tools::set_project_for_test("shop");
        let _mode = AmbientKey::set("FLEET_STUB_MODE", "happy");
        let _log = AmbientKey::set("FLEET_STUB_LOG", argv.to_str().unwrap());
        let mount = std::env::temp_dir().join(format!(
            "fleet-layerfs-it-mount-seam-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&mount);
        let _mount_path = AmbientKey::set("FLEET_TEST_MOUNT", mount.to_str().unwrap());
        let mut progress = Vec::new();
        {
            let mut agent = Agent::new(
                &client,
                options,
                &mut session,
                Box::new(ApproveAll),
                &mut progress,
            )
            .unwrap();
            agent.run("work in your workspace").await.unwrap();
        }
        let _ = std::fs::remove_dir_all(&mount);
    }

    // The child's three answers, rendered the way the host tools
    // render them.
    let child = child_summary(&home);
    let records = session::replay(&child.path).unwrap();
    let (write_out, _) = tool_result(&records, "write_file");
    assert!(
        write_out.contains("wrote 15 bytes to /notes.txt"),
        "the host's write line: {write_out}"
    );
    let (bash_out, _) = tool_result(&records, "bash");
    assert!(
        bash_out.contains("exit 0") && bash_out.contains("--- stdout ---\nran"),
        "the host's bash shape over the cadence: {bash_out}"
    );
    let (read_out, _) = tool_result(&records, "read_file");
    assert!(
        read_out.contains("notes.txt") && read_out.contains("hello"),
        "the host's numbered read over the branch: {read_out}"
    );

    // The parent reads its branch and the merge back.
    let parent_records = session::replay(session.path()).unwrap();
    let (read_back, _) = tool_result(&parent_records, "subagent");
    assert!(
        read_back.contains("workspace: branch fleet-child-"),
        "the branch rides the result: {read_back}"
    );
    assert!(read_back.contains("merged to main"), "{read_back}");

    // The coherence sequence, in argv order: the file write lands on
    // the branch directly, ONE bash call walks the whole cadence
    // around its exec, and the read comes after the command's
    // checkpoint — the sequence the wire findings demanded.
    let calls = std::fs::read_to_string(&argv).expect("the stub was called");
    let at = |needle: &str| {
        calls
            .find(needle)
            .unwrap_or_else(|| panic!("`{needle}` in the argv: {calls}"))
    };
    let write = at("fs write");
    let mount = at("workspace mount");
    let exec = at("workspace exec");
    let commit = at("workspace commit");
    let unmount = at("workspace unmount");
    let read = at("fs read");
    assert!(
        write < mount && mount < exec && exec < commit && commit < unmount && unmount < read,
        "fs write < mount < exec < commit < unmount < fs read: {calls}"
    );
    // The settle's own reads rode along after the cadence: the home
    // apply's diff (C2) and the merge-back's publish, in that order.
    let diff = at("branch diff_branches");
    let publish = at("branch publish");
    assert!(
        unmount < diff && diff < publish,
        "the settle diffs the child against main, then publishes: {calls}"
    );
    assert!(
        calls.contains("--project shop branch fork fleet-child-"),
        "the fork: {calls}"
    );
    assert!(
        !calls[..write].contains("workspace mount"),
        "no mount at spawn — only the bash call mounts: {calls}"
    );
    assert!(
        calls.contains("branch merge main fleet-child-"),
        "the parent's merge: {calls}"
    );
    assert!(
        calls.contains("branch publish merge-fleet-child-"),
        "{calls}"
    );

    std::fs::remove_dir_all(&home).ok();
    std::fs::remove_file(&argv).ok();
}

/// C3's byte-identity, at the arm: with no project named, the driver
/// is never consulted — the stub records no argv at all — and the
/// parent's result carries no workspace block. The default run is the
/// run fleet always had.
// The lock serializes the process's env against the other env
// tests for the whole awaited run — the same deliberate hold the
// ambient test documents.
#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn without_the_knob_the_delegation_never_touches_the_driver() {
    let mock = MockApi::start(vec![
        tool_call_stream(
            "call-1",
            "subagent",
            &delegation_arguments("tidy the workspace"),
        ),
        assistant_stream("the child is done"),
        assistant_stream("the parent wraps up"),
    ]);
    let home = temp_home("layerfs-off-home");
    let mut session = Session::create(&home, "test-model").unwrap();
    let client = Client::new(mock.url(), "test-key");
    let options = AgentOptions {
        goal: None,
        model: "test-model".to_string(),
        identity: None,
        max_turns: 8,
        ui_tx: None,
        lane_specs: None,
        calib: None,
        skills: Vec::new(),
        role_overlay: None,
        memory: None,
        scope: None,
        improve_home: None,
        route_home: None,
        gui_prompt: None,
        compact_now: false,
        checkpoints: None,
        file_access: FileAccess::Unrestricted,
        plan_mode: false,
    };
    let argv =
        std::env::temp_dir().join(format!("fleet-layerfs-it-argv-off-{}", std::process::id()));
    let _ = std::fs::remove_file(&argv);
    {
        let _lock = AMBIENT_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let _bin = AmbientKey::set("FLEET_LAYERFS_BIN", shared_layerfs_stub().to_str().unwrap());
        // The knob is never set on this thread and never set in the
        // environment: off is the default the C3 test asserts.
        let _log = AmbientKey::set("FLEET_STUB_LOG", argv.to_str().unwrap());
        let mut progress = Vec::new();
        {
            let mut agent = Agent::new(
                &client,
                options,
                &mut session,
                Box::new(ApproveAll),
                &mut progress,
            )
            .unwrap();
            agent.run("delegate the tidying").await.unwrap();
        }
    }

    assert!(
        !argv.exists(),
        "the driver was never consulted — no argv log exists at {argv:?}"
    );
    let parent_records = session::replay(session.path()).unwrap();
    let (read_back, _) = tool_result(&parent_records, "subagent");
    assert!(
        !read_back.contains("workspace:"),
        "no workspace block in the default run: {read_back}"
    );
    assert!(read_back.contains("subagent completed"), "{read_back}");

    std::fs::remove_dir_all(&home).ok();
    std::fs::remove_file(&argv).ok();
}

/// C2's conflict leg: two agents changed the same file, the merge
/// refused, and the PARENT reads the Conflict — named with its path,
/// the child's branch kept for resolution, the delegation itself
/// still the success it was. Nothing is overwritten silently.
// The lock serializes the process's env against the other env
// tests for the whole awaited run — the same deliberate hold the
// ambient test documents.
#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn a_same_file_collision_is_a_conflict_the_parent_reads() {
    let mock = MockApi::start(vec![
        tool_call_stream(
            "call-1",
            "subagent",
            &delegation_arguments("tidy the workspace"),
        ),
        assistant_stream("the child is done"),
        assistant_stream("the parent wraps up"),
    ]);
    let home = temp_home("layerfs-conflict-home");
    let mount = temp_home("layerfs-conflict-mount");
    let mut session = Session::create(&home, "test-model").unwrap();
    let client = Client::new(mock.url(), "test-key");
    let options = AgentOptions {
        goal: None,
        model: "test-model".to_string(),
        identity: None,
        max_turns: 8,
        ui_tx: None,
        lane_specs: None,
        calib: None,
        skills: Vec::new(),
        role_overlay: None,
        memory: None,
        scope: None,
        improve_home: None,
        route_home: None,
        gui_prompt: None,
        compact_now: false,
        checkpoints: None,
        file_access: FileAccess::Unrestricted,
        plan_mode: false,
    };
    let argv = std::env::temp_dir().join(format!(
        "fleet-layerfs-it-argv-conflict-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&argv);
    {
        let _lock = AMBIENT_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let _bin = AmbientKey::set("FLEET_LAYERFS_BIN", shared_layerfs_stub().to_str().unwrap());
        // The knob is per-test-thread (thread-local), never env: an
        // env window would switch the arm on inside another test's
        // delegation running concurrently.
        fleet::layerfs_tools::set_project_for_test("shop");
        let _mode = AmbientKey::set("FLEET_STUB_MODE", "conflict");
        let _log = AmbientKey::set("FLEET_STUB_LOG", argv.to_str().unwrap());
        let _mount_path = AmbientKey::set("FLEET_TEST_MOUNT", mount.to_str().unwrap());
        let mut progress = Vec::new();
        {
            let mut agent = Agent::new(
                &client,
                options,
                &mut session,
                Box::new(ApproveAll),
                &mut progress,
            )
            .unwrap();
            agent.run("delegate the tidying").await.unwrap();
        }
    }

    let parent_records = session::replay(session.path()).unwrap();
    let (read_back, _) = tool_result(&parent_records, "subagent");
    assert!(
        read_back.contains("CONFLICT with main on 1 path: src/lib.rs"),
        "the collision is named with its path: {read_back}"
    );
    assert!(
        read_back.contains("is kept for resolution"),
        "the child's branch is kept, not silently dropped: {read_back}"
    );
    // The child succeeded; the collision is a fact about main, not a
    // failure of the delegation.
    assert!(
        read_back.starts_with("subagent completed"),
        "the outcome stays the child's: {read_back}"
    );
    let calls = std::fs::read_to_string(&argv).expect("the stub was called");
    assert!(
        calls.contains("--project shop branch merge main fleet-child-"),
        "the merge was attempted the parent's way: {calls}"
    );
    // A refused merge publishes nothing.
    assert!(
        !calls.contains("branch publish"),
        "no publish follows a conflict: {calls}"
    );

    std::fs::remove_dir_all(&home).ok();
    std::fs::remove_dir_all(&mount).ok();
    std::fs::remove_file(&argv).ok();
}

/// C2's live run — the driver's whole LayerFS leg against the real
/// wire, plus the boundary the live run itself discovered: the mount
/// lives in the SANDBOX's namespace, so a host process at that path
/// meets ENOENT (named as KI-314 — the child-execution seam is the
/// next increment; dsh's pattern routes file ops through `layerfs
/// fs.*` and command runs through `workspace exec`).
///
/// Needs the provisioned environment (KI-313's recipe: the sealed
/// bundle, the worker, an initialized project — `scripts/layerfs-probe.sh`
/// walks and names every rung):
///   LAYERFS_HOME=… LAYERFS_WORKER_ENDPOINT=… LAYERFS_IMAGE=… \
///   LAYERFS_DAEMON=… LAYERFS_STORE_PATH=… FLEET_LAYERFS_BIN=… \
///   cargo test --test integration a_layerfs_live -- --ignored --nocapture
#[tokio::test]
#[ignore = "needs the provisioned LayerFS environment (KI-313); see scripts/layerfs-probe.sh"]
async fn a_layerfs_live_driver_roundtrip_against_the_real_wire() {
    // Prerequisites named, never assumed.
    let endpoint = std::env::var("LAYERFS_WORKER_ENDPOINT")
        .expect("LAYERFS_WORKER_ENDPOINT — provision first (KI-313's recipe)");
    assert!(
        std::path::Path::new(&endpoint).exists(),
        "the worker endpoint exists at {endpoint}"
    );
    let bin = std::env::var("FLEET_LAYERFS_BIN").expect("FLEET_LAYERFS_BIN");
    let project =
        std::env::var("LAYERFS_PROJECT_FOR_TEST").unwrap_or_else(|_| "fleet-probe".to_string());
    let branch = format!("fleet-live-{}", std::process::id());

    // Open: fork, sandbox, mount — the real CLI through the driver's gate.
    let open_started = std::time::Instant::now();
    let session = fleet::layerfs_tools::WorkspaceSession::open(&project, &branch)
        .expect("a live workspace opens against the provisioned wire");
    let open_ms = open_started.elapsed().as_millis();
    let ws = session.mount.ws.clone();

    // The boundary: the host cannot see the mount; the sandbox can.
    assert!(
        !session.path().exists(),
        "the mount is sandbox-local — the host meets ENOENT at {:?}",
        session.path()
    );
    let exec = std::process::Command::new(&bin)
        .args([
            "--project",
            &project,
            "workspace",
            "exec",
            &ws,
            "printf 'live-roundtrip\\n' > live-marker.txt && cat live-marker.txt README.md",
        ])
        .output()
        .expect("the exec call spawns");
    assert!(
        exec.status.success(),
        "the in-sandbox write runs: {}",
        String::from_utf8_lossy(&exec.stderr)
    );
    let exec_stdout = String::from_utf8_lossy(&exec.stdout);
    assert!(
        exec_stdout.contains("live-roundtrip") && exec_stdout.contains("layerfs probe"),
        "the in-sandbox command wrote inside the mount and read the project tree: {exec_stdout}"
    );

    // Settle the arm's way: Commit, unmount, Sandbox close, merge into
    // main, publish — against the real store.
    let settle_started = std::time::Instant::now();
    let report = session.settle(true);
    let settle_ms = settle_started.elapsed().as_millis();
    assert!(report.committed, "the live Commit landed: {report:?}");
    assert!(
        matches!(
            report.merge,
            Some(fleet::layerfs_tools::MergeOutcome::Merged(_))
        ),
        "the merge into main landed: {report:?}"
    );
    assert!(
        report.notes.is_empty(),
        "no cleanup needed a note: {:?}",
        report.notes
    );

    // Read back from MAIN after publish: the child's file is history.
    let read = std::process::Command::new(&bin)
        .args([
            "--project",
            &project,
            "fs",
            "read",
            "main",
            "/live-marker.txt",
        ])
        .output()
        .expect("the read-back spawns");
    assert!(
        read.status.success(),
        "main carries the published file: {}",
        String::from_utf8_lossy(&read.stderr)
    );
    let body = String::from_utf8_lossy(&read.stdout);
    assert!(
        body.contains("live-roundtrip"),
        "read back from main: {body}"
    );

    println!(
        "LIVE ROUNDTRIP: open={open_ms}ms settle(incl. commit+unmount+close+merge+publish)={settle_ms}ms branch={branch} ws={ws}"
    );

    std::process::Command::new(&bin)
        .args(["--project", &project, "sandbox", "close", &report.sandbox])
        .output()
        .ok();
}

/// The seam plan's C2 — the live run. Part A asserts the three
/// findings the plan reconciled before writing code (a fresh mount
/// sees the branch head; a held view pins; command writes publish
/// only through Commit) and times one whole cadence call. Part B
/// walks the CHILD-shaped workflow against the real wire: a
/// delegation whose child writes a file, runs a command that reads
/// it, and reads it back — the coherence the stub only models.
///
/// Needs the provisioned environment (KI-313's recipe — see
/// scripts/layerfs-probe.sh):
///   LAYERFS_HOME=… LAYERFS_WORKER_ENDPOINT=… LAYERFS_IMAGE=… \
///   LAYERFS_DAEMON=… LAYERFS_STORE_PATH=… FLEET_LAYERFS_BIN=… \
///   cargo test --test integration a_layerfs_live_coherence -- --ignored --nocapture
#[tokio::test]
#[ignore = "needs the provisioned LayerFS environment (KI-313); see scripts/layerfs-probe.sh"]
async fn a_layerfs_live_coherence_findings_and_the_command_cadence() {
    let endpoint = std::env::var("LAYERFS_WORKER_ENDPOINT")
        .expect("LAYERFS_WORKER_ENDPOINT — provision first (KI-313's recipe)");
    assert!(
        std::path::Path::new(&endpoint).exists(),
        "the worker endpoint exists at {endpoint}"
    );
    std::env::var("FLEET_LAYERFS_BIN").expect("FLEET_LAYERFS_BIN");
    let project =
        std::env::var("LAYERFS_PROJECT_FOR_TEST").unwrap_or_else(|_| "fleet-probe".to_string());
    let branch = format!("fleet-seam-{}", std::process::id());
    let gate = fleet::layerfs_tools::gate()
        .as_ref()
        .expect("the real CLI resolves")
        .clone();

    // --- Part A: the three findings, on the wire ---
    // Fresh mount sees the current head.
    gate.fork(&project, &branch)
        .expect("fork the coherence branch");
    gate.fs_write(
        &project,
        &branch,
        "/via-sdk.txt",
        b"content-v1\n",
        "live: finding 1",
    )
    .expect("sdk write");
    let sandbox = gate.sandbox_create(&project).expect("sandbox");
    let mount = gate
        .mount(&project, &sandbox, &branch)
        .expect("mount after the sdk write");
    let out = gate
        .exec(
            &project,
            &mount.ws,
            "cat via-sdk.txt",
            std::time::Duration::from_secs(30),
        )
        .await_exec_json("fresh mount");
    assert_eq!(
        out.get("stdout").and_then(|v| v.as_str()),
        Some("content-v1\n"),
        "FINDING 1: a fresh mount sees the branch's current head"
    );

    // A held view pins: an SDK write after the mount stays invisible.
    gate.fs_write(
        &project,
        &branch,
        "/via-sdk.txt",
        b"content-v2\n",
        "live: finding 2",
    )
    .expect("sdk write while mounted");
    let pinned = gate
        .exec(
            &project,
            &mount.ws,
            "cat via-sdk.txt",
            std::time::Duration::from_secs(30),
        )
        .await_exec_json("held view");
    assert_eq!(
        pinned.get("stdout").and_then(|v| v.as_str()),
        Some("content-v1\n"),
        "FINDING 2: the held view pins its base — the cadence must re-mount per command"
    );

    // Command writes publish only through Commit.
    gate.exec(
        &project,
        &mount.ws,
        "printf 'cmd-file\n' > cmd.txt && cat cmd.txt",
        std::time::Duration::from_secs(30),
    )
    .await_exec_json("command write");
    let before = gate.fs_read(&project, &branch, "/cmd.txt", None);
    match before {
        Err(fleet::layerfs_tools::Error::Refused { code, .. }) => {
            assert_eq!(
                code, "NoSuchPath",
                "FINDING 3a: uncommitted writes are invisible to reads"
            )
        }
        other => panic!("FINDING 3a: expected NoSuchPath before commit, got {other:?}"),
    }
    gate.commit(&project, &mount.ws, "live: finding 3")
        .expect("the checkpoint");
    let after = gate
        .fs_read(&project, &branch, "/cmd.txt", None)
        .expect("FINDING 3b: after commit the read returns the command's file");
    assert!(
        after
            .get("text")
            .and_then(|t| t.as_str())
            .is_some_and(|t| t.contains("cmd-file")),
        "FINDING 3b: {after}"
    );
    gate.unmount(&project, &mount.ws, false).expect("unmount");
    gate.sandbox_close(&project, &sandbox).expect("close");

    // One whole cadence, timed: mount -> exec -> commit -> unmount.
    let ctx = fleet::layerfs_tools::WorkspaceHandle {
        project: project.clone(),
        branch: branch.clone(),
        sandbox: gate.sandbox_create(&project).expect("sandbox for cadence"),
        digests: None,
        folder: None,
        sync: None,
    };
    let started = std::time::Instant::now();
    let outcome = gate
        .run_command(
            &ctx,
            "echo cadence-probe",
            std::time::Duration::from_secs(30),
        )
        .expect("the live cadence completes");
    let cadence_ms = started.elapsed().as_millis();
    assert_eq!(outcome.exit_code, 0);
    assert_eq!(outcome.stdout, "cadence-probe\n");
    assert_eq!(outcome.steps, vec!["mount", "exec", "commit", "unmount"]);
    ctx.close(false);
    println!(
        "CADENCE (mount+exec+commit+unmount, trivial command): {cadence_ms}ms  branch={branch}"
    );

    // --- Part B: the child-shaped workflow on the real wire ---
    let mock = MockApi::start(vec![
        tool_call_stream(
            "call-1",
            "subagent",
            &delegation_arguments("write, run, and read in your workspace"),
        ),
        tool_call_stream(
            "child-w",
            "write_file",
            r#"{"path":"/live-note.txt","content":"the child was here\n"}"#,
        ),
        tool_call_stream("child-b", "bash", r#"{"command":"cat live-note.txt"}"#),
        tool_call_stream("child-r", "read_file", r#"{"path":"/live-note.txt"}"#),
        assistant_stream("the child is done"),
        assistant_stream("the parent wraps up"),
    ]);
    let home = temp_home("layerfs-live-seam-home");
    let mut session = Session::create(&home, "test-model").unwrap();
    let client = Client::new(mock.url(), "test-key");
    let options = AgentOptions {
        goal: None,
        model: "test-model".to_string(),
        identity: None,
        max_turns: 9,
        ui_tx: None,
        lane_specs: None,
        calib: None,
        skills: Vec::new(),
        role_overlay: None,
        memory: None,
        scope: None,
        improve_home: None,
        route_home: None,
        gui_prompt: None,
        compact_now: false,
        checkpoints: None,
        file_access: FileAccess::Unrestricted,
        plan_mode: false,
    };
    fleet::layerfs_tools::set_project_for_test(&project);
    let mut progress = Vec::new();
    {
        let mut agent = Agent::new(
            &client,
            options,
            &mut session,
            Box::new(ApproveAll),
            &mut progress,
        )
        .unwrap();
        agent.run("work in your workspace").await.unwrap();
    }

    let child = child_summary(&home);
    let records = session::replay(&child.path).unwrap();
    let (write_out, _) = tool_result(&records, "write_file");
    assert!(
        write_out.contains("wrote 19 bytes to /live-note.txt"),
        "the child's real write: {write_out}"
    );
    let (bash_out, _) = tool_result(&records, "bash");
    assert!(
        bash_out.contains("exit 0") && bash_out.contains("the child was here"),
        "the child's command read ITS OWN file off the branch through a fresh mount: {bash_out}"
    );
    let (read_out, _) = tool_result(&records, "read_file");
    assert!(
        read_out.contains("the child was here"),
        "read_file sees what the command checkpointed: {read_out}"
    );
    let parent_records = session::replay(session.path()).unwrap();
    let (read_back, _) = tool_result(&parent_records, "subagent");
    assert!(
        read_back.contains("merged to main"),
        "the parent merged the child's branch: {read_back}"
    );

    // The child's file is on main after the merge+publish: prove it
    // through the wire, not through the render.
    let published = gate
        .fs_read(&project, "main", "/live-note.txt", None)
        .expect("main carries the child's file after the parent's publish");
    assert!(
        published
            .get("text")
            .and_then(|t| t.as_str())
            .is_some_and(|t| t.contains("the child was here")),
        "published to main: {published}"
    );

    println!(
        "LIVE COHERENCE: findings 1-3 asserted, cadence {cadence_ms}ms, child workflow on branch of {project} merged and published to main"
    );
    std::fs::remove_dir_all(&home).ok();
}

/// The one-call JSON shape helper for the live part A: parse the
/// driver's answer or panic with it (a live test may not go quiet).
trait LiveJson {
    fn await_exec_json(self, what: &str) -> serde_json::Value;
}
impl LiveJson for Result<serde_json::Value, fleet::layerfs_tools::Error> {
    fn await_exec_json(self, what: &str) -> serde_json::Value {
        self.unwrap_or_else(|error| panic!("{what}: {error}"))
    }
}

/// C2's integration proof: a delegation in a registered git folder
/// settles its merge home — the child's change applied through the
/// parent's own write path (one grouped approval, ApproveAll here), the
/// host file carrying the merged child's version, and the report naming
/// what was applied. The conflict half is pinned at the decision's own
/// level (`the_settle_classifies_the_home_apply_from_the_spawn_hashes`),
/// where the classification lives.
#[tokio::test]
async fn a_settled_delegation_applies_the_merge_home() {
    let mock = MockApi::start(vec![
        tool_call_stream(
            "call-1",
            "subagent",
            &delegation_arguments("change home.txt"),
        ),
        tool_call_stream(
            "child-w",
            "write_file",
            r#"{"path":"/home.txt","content":"the child wrote this\n"}"#,
        ),
        assistant_stream("the child is done"),
        assistant_stream("the parent wraps up"),
    ]);
    let home = temp_home("layerfs-apply-home");
    let folder = temp_home("layerfs-apply-repo");
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(folder.join("home.txt"), "base\n").unwrap();
    let git = |args: &[&str]| {
        let status = std::process::Command::new("git")
            .arg("-C")
            .arg(&folder)
            .args(args)
            .env("GIT_AUTHOR_NAME", "t")
            .env("GIT_AUTHOR_EMAIL", "t@t")
            .env("GIT_COMMITTER_NAME", "t")
            .env("GIT_COMMITTER_EMAIL", "t@t")
            .status()
            .expect("git runs");
        assert!(status.success(), "git {args:?}");
    };
    git(&["init", "-q"]);
    git(&["add", "-A"]);
    git(&["commit", "-qm", "the base"]);
    let mut session = Session::create(&home, "test-model").unwrap();
    let client = Client::new(mock.url(), "test-key");
    let options = AgentOptions {
        goal: None,
        model: "test-model".to_string(),
        identity: None,
        max_turns: 8,
        ui_tx: None,
        lane_specs: None,
        calib: None,
        skills: Vec::new(),
        role_overlay: None,
        memory: None,
        scope: None,
        improve_home: None,
        route_home: None,
        gui_prompt: None,
        compact_now: false,
        checkpoints: None,
        file_access: FileAccess::Folder(folder.clone()),
        plan_mode: false,
    };
    let argv = std::env::temp_dir().join(format!(
        "fleet-layerfs-it-argv-applyhome-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&argv);
    // The SET is atomic (the lock); the restoration guard outlives the
    // await, the lock guard does not — the sibling layerfs tests set
    // their own stub mode in their own windows.
    let _bin = {
        let _lock = AMBIENT_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        AmbientKey::set("FLEET_LAYERFS_BIN", shared_layerfs_stub().to_str().unwrap())
    };
    fleet::layerfs_tools::set_project_for_test("shop");
    let _mode = {
        let _lock = AMBIENT_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        AmbientKey::set("FLEET_STUB_MODE", "happy")
    };
    let _log = {
        let _lock = AMBIENT_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        AmbientKey::set("FLEET_STUB_LOG", argv.to_str().unwrap())
    };
    let mount = std::env::temp_dir().join(format!(
        "fleet-layerfs-it-mount-applyhome-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&mount);
    let _mount_path = {
        let _lock = AMBIENT_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        AmbientKey::set("FLEET_TEST_MOUNT", mount.to_str().unwrap())
    };
    let mut progress = Vec::new();
    {
        let mut agent = Agent::new(
            &client,
            options,
            &mut session,
            Box::new(ApproveAll),
            &mut progress,
        )
        .unwrap();
        agent.run("delegate the change").await.unwrap();
    }

    // The host file carries the merged child's version — the apply rode
    // the parent's own write path. The version is the stub's `fs read`
    // answer (the shared stub does not track content), which is exactly
    // what a real branch read would return.
    assert_eq!(
        std::fs::read_to_string(folder.join("home.txt")).unwrap(),
        "hello\n",
        "the settled merge landed home"
    );
    // The report names what was applied, in the parent's own result.
    let parent_records = session::replay(session.path()).unwrap();
    let (output, _) = tool_result(&parent_records, "subagent");
    assert!(
        output.contains("applied home") && output.contains("home.txt"),
        "the apply is named in the result: {output}"
    );
    let _ = std::fs::remove_dir_all(&home);
    let _ = std::fs::remove_dir_all(&folder);
}

// --- the guided onboarding ------------------------------------------------

/// A scratch home with nothing configured: the onboarding's own
/// starting point. The lanes' homes are absent, no key resolves from
/// the process (the caller's own dotenv is the only source), and the
/// corpus is empty.
fn onboard_home(name: &str) -> std::path::PathBuf {
    let home = temp_home(name);
    std::fs::create_dir_all(&home).unwrap();
    home
}

/// The onboarding's sequence over a scratch home: the state derives
/// from the machine, a decision survives a restart without any state of
/// its own, and the completion probe reads back what the sequence wrote.
#[test]
fn the_onboarding_resumes_at_the_first_incomplete_step() {
    use fleet::onboard;
    let home = onboard_home("onboard-resume");
    let dotenv = std::collections::HashMap::new();

    // First launch: the home step is the first incomplete one.
    let first = onboard::state(&home, &dotenv);
    assert_eq!(first.first_incomplete(), Some(onboard::STEP_HOME));

    // Do the home step's own work, then read again — the restart.
    for name in ["sessions", "memories", "skills"] {
        fleet::install::layout_step(&home, name).unwrap();
    }
    fleet::install::config_step(&home).unwrap();
    let second = onboard::state(&home, &dotenv);
    let home_step = second.step(onboard::STEP_HOME).expect("the home step");
    assert!(home_step.done, "the home step's proof must hold");
    assert_ne!(
        second.first_incomplete(),
        Some(onboard::STEP_HOME),
        "a completed step must not be where the sequence resumes"
    );

    // Decline the lanes with the consequence the narration names, then
    // read a third time: the skip survives, and the sequence moves past
    // it without any onboarding state on disk.
    let facts = onboard::Facts::read(&home, &dotenv);
    let consequence = onboard::narration(onboard::STEP_LANES, &facts).skip;
    onboard::skip(&home, onboard::STEP_LANES, &consequence).unwrap();
    let third = onboard::state(&home, &dotenv);
    assert!(
        third.step(onboard::STEP_LANES).expect("lanes").skipped,
        "the skip must be read back from the stream"
    );
    let decided = onboard::decisions(&home);
    assert_eq!(decided.skipped, vec![onboard::STEP_LANES]);
    let _ = std::fs::remove_dir_all(&home);
}

/// The completion probe: four real reads, and the recalled fact is the
/// one the sequence itself wrote.
#[test]
fn the_onboarding_seeds_the_fact_its_completion_reads() {
    use fleet::onboard;
    let home = onboard_home("onboard-completion");
    let dotenv = std::collections::HashMap::new();
    // Before: no fact, no project, nothing routed — not green.
    let before = onboard::completion(&home, &dotenv);
    assert!(!before.recalled_fact, "an empty corpus recalls nothing");
    assert!(!before.green());
    // The sequence remembers itself; the probe reads it back.
    let facts = onboard::Facts::read(&home, &dotenv);
    let name = onboard::remember_onboarding(&home, &facts).expect("the fact lands");
    assert!(name.starts_with("onboarding-"), "{name}");
    let after = onboard::completion(&home, &dotenv);
    assert!(
        after.recalled_fact,
        "the probe must read the fact the sequence wrote ({name})"
    );
    let _ = std::fs::remove_dir_all(&home);
}

/// The GUI's document: the door is up exactly when the machine cannot
/// answer, and the narration's default path carries no mechanism word.
#[test]
fn the_onboarding_document_is_the_door_when_it_cannot_answer() {
    use fleet::onboard;
    let home = onboard_home("onboard-document");
    let dotenv = std::collections::HashMap::new();
    let state = onboard::state(&home, &dotenv);
    let facts = onboard::Facts::read(&home, &dotenv);
    let document = state.to_json(&facts);
    assert_eq!(document["type"], "onboard");
    assert_eq!(
        document["ready"],
        serde_json::json!(!facts.keyed.is_empty())
    );
    // Every step carries its narration, so the face paints one read.
    let steps = document["steps"].as_array().expect("steps");
    assert_eq!(steps.len(), onboard::STEPS.len());
    for step in steps {
        assert!(
            step["narration"]["title"]
                .as_str()
                .unwrap_or_default()
                .len()
                > 4
        );
    }
    // The door's own copy: the benefit before the mechanism, and no
    // mechanism word on the default path.
    let door = &document["door"];
    let shown = format!(
        "{} {} {}",
        door["title"].as_str().unwrap_or_default(),
        door["lede"].as_str().unwrap_or_default(),
        door["action"].as_str().unwrap_or_default()
    );
    for banned in ["_API_KEY", "endpoint", "roster", "vault", "tier", "port"] {
        assert!(
            !shown.contains(banned),
            "the door names `{banned}`: {shown}"
        );
    }
    assert!(
        door["mechanism"].as_str().unwrap_or_default().len() > 20,
        "the mechanism must exist — it is where the words live"
    );
    let _ = std::fs::remove_dir_all(&home);
}
