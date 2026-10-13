//! End-to-end tests for the native agent: the parser's public surface, the
//! HTTP client against a local mock API, and full tool-loop runs that assert
//! both the run report and the session log the model saw. No network beyond
//! loopback; the mock API runs as a child process.
//!
//! The live-API test is `#[ignore]`d like the repo's `test:e2e`: run it with
//! `cargo test --test integration -- --ignored` and `DEEPSEEK_API_KEY` set.

use std::path::{Path, PathBuf};

use fleet::agent::{
    Agent, AgentError, AgentOptions, ApproveAll, Approver, DenyAll, RunReport, StopReason,
};
use fleet::api::{
    Client, ClientError, ChatRequest, DEFAULT_BASE_URL, Message, Role, StreamEvent, ToolCall, Usage,
};
use fleet::cli;
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
    let dir = std::env::temp_dir().join(format!(
        "fleet-integration-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .subsec_nanos()
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
    let options = AgentOptions { model: "test-model".to_string(), max_turns };
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
    let options = AgentOptions { model: "test-model".to_string(), max_turns: 5 };
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

    // The log holds exactly what the model saw plus bookkeeping.
    let records = session::replay(&session.path()).unwrap();
    assert_eq!(records.len(), 6);
    assert!(matches!(&records[0], Record::Start { v: 1, model, .. } if model == "test-model"));
    assert!(matches!(&records[1], Record::User { content } if content == "say hello"));
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

// --- live API --------------------------------------------------------------

#[tokio::test]
#[ignore = "real-API e2e; run with `cargo test --test integration -- --ignored` and DEEPSEEK_API_KEY set"]
async fn live_api_completes_a_tool_task() {
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
