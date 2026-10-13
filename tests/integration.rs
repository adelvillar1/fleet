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
    Agent, AgentError, AgentOptions, ApproveAll, Approver, DenyAll, RunReport, StopReason, UiEvent,
};
use fleet::api::{
    Client, ClientError, ChatRequest, DEFAULT_BASE_URL, Message, Role, StreamEvent, ToolCall, Usage,
};
use fleet::cli;
use fleet::lanes::{self, Lane, LaneHome, LaneState};
use fleet::mcp::{McpError, McpServer, ServerSpec, Timeouts};
use fleet::session::{self, Record, Session};
use fleet::tools::{Tool, ALL_TOOLS, MAX_TOOL_OUTPUT};

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
    ScriptedResponse { status: "200 OK".to_string(), body, sse: Vec::new(), delay_ms: 0 }
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
        let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_mock-server"))
            .arg(&script_path)
            .arg(&requests_path)
            .arg(&port_path)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("spawn the mock-server child");
        let port = wait_for_port(&port_path, &mut child);
        MockApi { child, port, scratch }
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
fn wait_for_port(port_path: &Path, child: &mut std::process::Child) -> u16 {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        if let Ok(text) = std::fs::read_to_string(port_path) {
            if let Ok(port) = text.trim().parse() {
                return port;
            }
        }
        if child.try_wait().unwrap().is_some() {
            panic!("the mock-server child exited before writing its port");
        }
        assert!(std::time::Instant::now() < deadline, "the mock-server port never arrived");
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
        Probe { writes: Vec::new(), snapshots: Vec::new(), buffer: String::new() }
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
async fn run_on(
    mock: &MockApi,
    session: &mut Session,
    approver: Box<dyn Approver>,
    max_turns: usize,
    task: &str,
) -> Result<RunReport, AgentError> {
    let client = Client::new(mock.url(), "test-key");
    let options = AgentOptions { model: "test-model".to_string(), max_turns, ui_tx: None };
    let mut progress = Vec::new();
    let mut agent = Agent::new(&client, options, session, approver, &mut progress).unwrap();
    agent.run(task).await
}

/// One full agent run whose progress sink records what it saw and when.
async fn run_on_probe(
    mock: &MockApi,
    session: &mut Session,
    probe: &mut Probe,
    task: &str,
) -> Result<RunReport, AgentError> {
    let client = Client::new(mock.url(), "test-key");
    let options =
        AgentOptions { model: "test-model".to_string(), max_turns: 5, ui_tx: None };
    let mut agent =
        Agent::new(&client, options, session, Box::new(ApproveAll), probe).unwrap();
    agent.run(task).await
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
    assert_eq!(cli::parse(&args(&["sessions"])), Ok(cli::Command::Sessions));
    assert_eq!(cli::parse(&args(&["doctor"])), Ok(cli::Command::Doctor));
    assert_eq!(cli::parse(&args(&["version"])), Ok(cli::Command::Version));
    let error = cli::parse(&args(&["launch", "--profile", "web"])).unwrap_err();
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
    assert!(recorded.head.starts_with("POST /chat/completions "), "{}", recorded.head);
    assert!(recorded.head.contains("Authorization: Bearer test-key"), "{}", recorded.head);
    let body: serde_json::Value = serde_json::from_str(&recorded.body).unwrap();
    assert_eq!(body["model"], "test-model");
    assert_eq!(body["tools"].as_array().unwrap().len(), 4);
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
    let mock = MockApi::start(vec![ok(tool_call_response(serde_json::json!([tool_call(
        "call-1",
        "bash",
        r#"{"command":"echo hi"}"#,
    )])))]);
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
    assert!(body.get("stream").is_none(), "the one-shot path does not ask for a stream");
    assert!(body.get("stream_options").is_none());
}

#[tokio::test]
async fn client_chat_stream_assembles_the_same_response_chat_returns() {
    let mock = MockApi::start(vec![tool_call_stream("call-1", "bash", r#"{"command":"echo hi"}"#)]);
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
    assert_eq!(assembled[0][0].function.arguments, r#"{"command":"echo hi"}"#);
    assert!(events.iter().all(|event| !matches!(event, StreamEvent::Content(_))));
    assert!(events.contains(&StreamEvent::Usage(Usage {
        prompt_tokens: 10,
        completion_tokens: 5,
        total_tokens: 15,
    })));
}

// --- the agent loop --------------------------------------------------------

#[tokio::test]
async fn run_completes_a_two_turn_tool_loop() {
    let mock = MockApi::start(vec![
        tool_call_stream("call-1", "bash", r#"{"command":"echo hello"}"#),
        assistant_stream("All done."),
    ]);
    let home = temp_home("two-turn");
    let mut session = Session::create(&home, "test-model").unwrap();
    let report = run_on(&mock, &mut session, Box::new(ApproveAll), 5, "say hello").await.unwrap();

    assert_eq!(report.stop_reason, StopReason::Completed);
    assert_eq!(report.turns, 2);
    assert_eq!(report.final_message.as_deref(), Some("All done."));
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
    assert_eq!(message_roles(&bodies[1]), vec!["system", "user", "assistant", "tool"]);
    assert_eq!(bodies[1]["messages"][3]["tool_call_id"], "call-1");
    assert!(bodies[1]["messages"][3]["content"].as_str().unwrap().contains("hello"));
    // KI-016: the assistant message the next request carries still names its
    // tool call — an assistant turn with no tool_calls followed by a tool
    // result is not a valid sequence for the API.
    let replayed_calls = &bodies[1]["messages"][2]["tool_calls"];
    assert_eq!(replayed_calls.as_array().map(Vec::len), Some(1), "{replayed_calls}");
    assert_eq!(replayed_calls[0]["id"], "call-1");

    // The log holds exactly what the model saw plus bookkeeping.
    let records = session::replay(&session.path()).unwrap();
    assert_eq!(records.len(), 6);
    assert!(matches!(&records[0], Record::Start { v: 1, model, .. } if model == "test-model"));
    assert!(matches!(&records[1], Record::User { content } if content == "say hello"));
    // KI-016: the assistant record carries the call the model made, so a
    // resumed session replays a valid sequence.
    assert!(
        matches!(&records[2], Record::Assistant { tool_calls, .. } if tool_calls.len() == 1 && tool_calls[0].id == "call-1"),
        "{:?}",
        records[2]
    );
    assert!(matches!(&records[3], Record::ToolResult { approved: true, .. }));
    assert!(matches!(
        &records[5],
        Record::End { stop_reason, turns: 2 } if stop_reason == "completed"
    ));

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
    let report = run_on_probe(&mock, &mut session, &mut probe, "stream it").await.unwrap();

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
    let records = session::replay(&session.path()).unwrap();
    let assistants: Vec<&Record> = records
        .iter()
        .filter(|record| matches!(record, Record::Assistant { .. }))
        .collect();
    assert_eq!(assistants.len(), 1);
    match assistants[0] {
        Record::Assistant { content, tool_calls } => {
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
    run_on(&first, &mut session, Box::new(ApproveAll), 5, "first task").await.unwrap();

    let second = MockApi::start(vec![assistant_stream("Second answer.")]);
    let mut resumed = Session::open(&session.path()).unwrap();
    let report = run_on(&second, &mut resumed, Box::new(ApproveAll), 5, "second task").await.unwrap();
    assert_eq!(report.stop_reason, StopReason::Completed);

    let bodies = second.bodies();
    let messages = bodies[0]["messages"].as_array().unwrap();
    let roles: Vec<&str> = messages.iter().map(|m| m["role"].as_str().unwrap()).collect();
    assert_eq!(roles, vec!["system", "user", "assistant", "tool", "assistant", "user"]);
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
    let report = run_on(&mock, &mut session, Box::new(DenyAll), 5, "go wild").await.unwrap();
    assert_eq!(report.stop_reason, StopReason::Completed);

    let records = session::replay(&session.path()).unwrap();
    match &records[3] {
        Record::ToolResult { output, approved: false, truncated: false, .. } => {
            assert_eq!(output, "denied: the user did not approve this call.");
        }
        other => panic!("expected a denied tool result, got {other:?}"),
    }

    let bodies = mock.bodies();
    assert_eq!(bodies[1]["messages"][3]["content"], "denied: the user did not approve this call.");

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
    run_on(&mock, &mut session, Box::new(ApproveAll), 5, "try things").await.unwrap();

    let records = session::replay(&session.path()).unwrap();
    let outputs: Vec<&str> = records
        .iter()
        .filter_map(|record| match record {
            Record::ToolResult { output, .. } => Some(output.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(outputs.len(), 2);
    assert_eq!(outputs[0], "error: unknown tool `sudo`");
    assert!(outputs[1].starts_with("error: arguments were not valid JSON"), "{}", outputs[1]);

    std::fs::remove_dir_all(&home).ok();
}

#[tokio::test]
async fn tool_failure_is_a_model_visible_error_result() {
    let mock = MockApi::start(vec![
        tool_call_stream("call-1", "read_file", r#"{"path":"/nonexistent/fleet-test-file"}"#),
        assistant_stream("Noted."),
    ]);
    let home = temp_home("tool-error");
    let mut session = Session::create(&home, "test-model").unwrap();
    run_on(&mock, &mut session, Box::new(ApproveAll), 5, "read the missing file").await.unwrap();

    let records = session::replay(&session.path()).unwrap();
    match &records[3] {
        Record::ToolResult { output, approved: true, .. } => {
            assert!(output.starts_with("error: /nonexistent/fleet-test-file"), "{output}");
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
    let report = run_on(&mock, &mut session, Box::new(ApproveAll), 2, "loop forever").await.unwrap();

    assert_eq!(report.stop_reason, StopReason::MaxTurns);
    assert_eq!(report.turns, 2);
    assert_eq!(report.final_message, None);
    assert_eq!(mock.bodies().len(), 2);

    let records = session::replay(&session.path()).unwrap();
    assert!(matches!(
        records.last(),
        Some(Record::End { stop_reason, turns: 2 }) if stop_reason == "max-turns"
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
    run_on(&mock, &mut session, Box::new(ApproveAll), 5, "round-trip a big file").await.unwrap();

    let records = session::replay(&session.path()).unwrap();
    match &records[5] {
        Record::ToolResult { output, truncated: true, .. } => {
            assert!(output.contains("[truncated"), "expected the truncation marker");
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
        .execute(&serde_json::json!({ "path": target.display().to_string(), "content": "from fleet" }))
        .unwrap();
    let read = Tool::ReadFile
        .execute(&serde_json::json!({ "path": target.display().to_string() }))
        .unwrap();
    assert_eq!(read.output, "from fleet");
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
async fn ui_events_arrive_in_order_and_beside_the_log() {
    let mock = MockApi::start(vec![
        stream(vec![
            tool_call_delta(0, "call-1", "bash", r#"{"command":"echo hi"}"#),
            finish_frame("tool_calls"),
            usage_frame(10, 5),
            "data: [DONE]".to_string(),
        ]),
        stream(vec![
            content_delta("All "),
            content_delta("done."),
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
        model: "test-model".to_string(),
        max_turns: 5,
        ui_tx: Some(ui_tx.clone()),
    };
    let client = Client::new(mock.url(), "test-key");
    let mut progress = Vec::new();
    let report = {
        let mut agent =
            Agent::new(&client, options, &mut session, Box::new(ApproveAll), &mut progress)
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
            UiEvent::ToolCall { name: "bash".to_string(), summary: "echo hi".to_string() },
            UiEvent::ToolResult {
                name: "bash".to_string(),
                preview: "exit 0".to_string(),
                approved: true,
            },
            UiEvent::Turn(2),
            UiEvent::Token("All ".to_string()),
            UiEvent::Token("done.".to_string()),
            UiEvent::Finished { stop_reason: "completed".to_string() },
        ]
    );

    // The headless contract is untouched: the log holds the whole turn, and
    // the progress sink saw the same deltas as raw text.
    let records = session::replay(&log_path).unwrap();
    assert_eq!(records.len(), 6);
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
    let options =
        AgentOptions { model: "test-model".to_string(), max_turns: 5, ui_tx: Some(ui_tx) };
    let client = Client::new(mock.url(), "test-key");
    let mut progress = Vec::new();
    let mut agent =
        Agent::new(&client, options, &mut session, Box::new(ApproveAll), &mut progress).unwrap();
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
        model: "test-model".to_string(),
        max_turns: 5,
        ui_tx: Some(ui_tx.clone()),
    };
    let client = Client::new(mock.url(), "test-key");
    let mut progress = Vec::new();
    {
        let mut agent =
            Agent::new(&client, options, &mut session, Box::new(DenyAll), &mut progress).unwrap();
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
        UiEvent::ToolResult { name, preview, approved } => {
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
        model: "test-model".to_string(),
        max_turns: 5,
        ui_tx: Some(ui_tx.clone()),
    };
    let client = Client::new(mock.url(), "test-key");
    let mut progress = Vec::new();
    let failed = {
        let mut agent =
            Agent::new(&client, options, &mut session, Box::new(ApproveAll), &mut progress)
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
    assert!(records.iter().all(|record| !matches!(record, Record::End { .. })));

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
        let script =
            entries.iter().map(|entry| format!("{entry}\n")).collect::<String>();
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
            args: vec![self.scratch.join("script.jsonl").to_string_lossy().into_owned()],
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
    *lanes::LANES.iter().find(|lane| lane.name == name).expect("a registry lane")
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
    assert_eq!(names, vec!["sem1_embed", "sem1_probe", "sem1_index", "sem1_search"]);
    assert_eq!(tools[0].input_schema["required"], serde_json::json!(["items"]));

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

    let call = server.call_tool("fake_tool", serde_json::json!({ "x": 1 })).await.expect("tools/call");
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

    let error = server.list_tools().await.expect_err("a dead child must fail the wait");
    assert!(matches!(error, McpError::Closed(_)), "got {error:?}");
    // The teardown of an already-dead child is still graceful.
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
    let error = server.list_tools().await.expect_err("a silent child must time out");
    let elapsed = started.elapsed();
    assert!(elapsed < std::time::Duration::from_secs(5), "the budget must fire: {elapsed:?}");
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
    for name in ["sys1", "sdm1", "sem1", "gen1"] {
        assert!(stdout.contains(name), "{stdout}");
    }
    // Cards, not a table.
    assert!(stdout.contains("┌─"), "{stdout}");
    assert_eq!(stdout.matches('┌').count(), 4, "{stdout}");
    assert!(stdout.contains("4 lanes · 4 ok · 0 not ok"), "{stdout}");
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
    assert!(stdout.contains("4 lanes · 0 ok · 4 not ok"), "{stdout}");
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
    // Only the lane whose home was overridden is affected; the other three
    // still boot from the bundled copy.
    assert!(stdout.contains("4 lanes · 3 ok · 1 not ok"), "{stdout}");
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
    assert!(!reported.contains_key("FLEET_UNRELATED_AMBIENT"), "{reported:?}");
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
        AmbientKey { name: name.to_string(), previous }
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

    let error = server.list_tools().await.expect_err("the budget must fire first");
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
    let first = first.await.unwrap().expect("the first spawned call").content[0].data.clone();
    let second = second.await.unwrap().expect("the second spawned call").content[0].data.clone();
    let mut answered = vec![first.as_str().unwrap_or_default(), second.as_str().unwrap_or_default()];
    answered.sort();
    assert_eq!(answered, vec!["first", "second"]);
}

// --- live API --------------------------------------------------------------

#[tokio::test]
#[ignore = "real-API e2e; run with `cargo test --test integration -- --ignored` and DEEPSEEK_API_KEY set"]
async fn live_api_completes_a_tool_task() {
    // The ambient-key test sets DEEPSEEK_API_KEY for its own duration, so
    // the read here serializes against it.
    let _ambient_lock = AMBIENT_LOCK.lock().unwrap();
    let Ok(key) = std::env::var("DEEPSEEK_API_KEY") else {
        eprintln!("DEEPSEEK_API_KEY is not set; skipping the live test");
        return;
    };
    let home = temp_home("live");
    let mut session = Session::create(&home, "deepseek-chat").unwrap();
    let client = Client::new(DEFAULT_BASE_URL, key);
    let mut progress = std::io::stderr().lock();
    let mut agent =
        Agent::new(&client, AgentOptions::default(), &mut session, Box::new(ApproveAll), &mut progress)
            .unwrap();
    let report = agent
        .run("Run `echo fleet-live-ok` with bash, then reply with exactly its output.")
        .await
        .unwrap();
    assert_eq!(report.stop_reason, StopReason::Completed);
    assert!(
        report.final_message.as_deref().unwrap_or_default().contains("fleet-live-ok"),
        "{report:?}"
    );
    std::fs::remove_dir_all(&home).ok();
}
