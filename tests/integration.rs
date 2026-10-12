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
use fleet::api::{Client, ClientError, ChatRequest, DEFAULT_BASE_URL, Message, Role};
use fleet::cli;
use fleet::session::{self, Record, Session};
use fleet::tools::{Tool, ALL_TOOLS, MAX_TOOL_OUTPUT};

// --- mock API server -------------------------------------------------------

/// One scripted response: an HTTP status line and its JSON document.
#[derive(Debug, Clone)]
struct ScriptedResponse {
    status: String,
    body: String,
}

/// A `200 OK` response carrying `body`.
fn ok(body: String) -> ScriptedResponse {
    ScriptedResponse { status: "200 OK".to_string(), body }
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
                serde_json::json!({ "status": response.status, "body": response.body })
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

/// One full agent run against `mock` on the given session.
fn run_on(
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
    agent.run(task)
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

#[test]
fn parse_surfaces_the_documented_commands() {
    assert_eq!(cli::parse(&args(&["sessions"])), Ok(cli::Command::Sessions));
    assert_eq!(cli::parse(&args(&["doctor"])), Ok(cli::Command::Doctor));
    assert_eq!(cli::parse(&args(&["version"])), Ok(cli::Command::Version));
    let error = cli::parse(&args(&["launch", "--profile", "web"])).unwrap_err();
    assert!(error.contains("launch"), "{error}");
}

// --- client ----------------------------------------------------------------

#[test]
fn client_posts_auth_and_tool_declarations() {
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

#[test]
fn client_maps_an_api_error_with_its_hint() {
    let mock = MockApi::start(vec![ScriptedResponse {
        status: "401 Unauthorized".to_string(),
        body: r#"{"error":{"message":"auth failed"}}"#.to_string(),
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

// --- the agent loop --------------------------------------------------------

#[test]
fn run_completes_a_two_turn_tool_loop() {
    let mock = MockApi::start(vec![
        ok(tool_call_response(serde_json::json!([tool_call(
            "call-1",
            "bash",
            r#"{"command":"echo hello"}"#
        )]))),
        ok(assistant_response("All done.")),
    ]);
    let home = temp_home("two-turn");
    let mut session = Session::create(&home, "test-model").unwrap();
    let report = run_on(&mock, &mut session, Box::new(ApproveAll), 5, "say hello").unwrap();

    assert_eq!(report.stop_reason, StopReason::Completed);
    assert_eq!(report.turns, 2);
    assert_eq!(report.final_message.as_deref(), Some("All done."));
    assert_eq!(report.usage.prompt_tokens, 20);
    assert_eq!(report.usage.completion_tokens, 10);

    // The second request sees the system prompt, the task, the assistant's
    // call, and the recorded tool output.
    let bodies = mock.bodies();
    assert_eq!(bodies.len(), 2);
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

#[test]
fn resume_seeds_the_next_request_from_the_log() {
    let first = MockApi::start(vec![
        ok(tool_call_response(serde_json::json!([tool_call(
            "call-1",
            "bash",
            r#"{"command":"echo one"}"#
        )]))),
        ok(assistant_response("First answer.")),
    ]);
    let home = temp_home("resume");
    let mut session = Session::create(&home, "test-model").unwrap();
    run_on(&first, &mut session, Box::new(ApproveAll), 5, "first task").unwrap();

    let second = MockApi::start(vec![ok(assistant_response("Second answer."))]);
    let mut resumed = Session::open(&session.path()).unwrap();
    let report = run_on(&second, &mut resumed, Box::new(ApproveAll), 5, "second task").unwrap();
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

#[test]
fn denied_call_is_recorded_and_shown_to_the_model() {
    let mock = MockApi::start(vec![
        ok(tool_call_response(serde_json::json!([tool_call(
            "call-1",
            "bash",
            r#"{"command":"echo no"}"#
        )]))),
        ok(assistant_response("Understood, standing down.")),
    ]);
    let home = temp_home("denied");
    let mut session = Session::create(&home, "test-model").unwrap();
    let report = run_on(&mock, &mut session, Box::new(DenyAll), 5, "go wild").unwrap();
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

#[test]
fn unknown_tool_and_bad_arguments_are_model_visible_errors() {
    let mock = MockApi::start(vec![
        ok(tool_call_response(serde_json::json!([
            tool_call("call-1", "sudo", "{}"),
            tool_call("call-2", "bash", "{not json"),
        ]))),
        ok(assistant_response("Recovering.")),
    ]);
    let home = temp_home("bad-tools");
    let mut session = Session::create(&home, "test-model").unwrap();
    run_on(&mock, &mut session, Box::new(ApproveAll), 5, "try things").unwrap();

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

#[test]
fn tool_failure_is_a_model_visible_error_result() {
    let mock = MockApi::start(vec![
        ok(tool_call_response(serde_json::json!([tool_call(
            "call-1",
            "read_file",
            r#"{"path":"/nonexistent/fleet-test-file"}"#
        )]))),
        ok(assistant_response("Noted.")),
    ]);
    let home = temp_home("tool-error");
    let mut session = Session::create(&home, "test-model").unwrap();
    run_on(&mock, &mut session, Box::new(ApproveAll), 5, "read the missing file").unwrap();

    let records = session::replay(&session.path()).unwrap();
    match &records[3] {
        Record::ToolResult { output, approved: true, .. } => {
            assert!(output.starts_with("error: /nonexistent/fleet-test-file"), "{output}");
        }
        other => panic!("expected a failed tool result, got {other:?}"),
    }

    std::fs::remove_dir_all(&home).ok();
}

#[test]
fn max_turns_stops_the_loop_and_marks_the_log() {
    let scripted = tool_call_response(serde_json::json!([tool_call("call-1", "bash", "{}")]));
    let mock = MockApi::start(vec![ok(scripted.clone()), ok(scripted)]);
    let home = temp_home("max-turns");
    let mut session = Session::create(&home, "test-model").unwrap();
    let report = run_on(&mock, &mut session, Box::new(ApproveAll), 2, "loop forever").unwrap();

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

#[test]
fn oversized_tool_output_is_truncated_in_the_log() {
    // write_file a larger-than-cap file, then read it back through the loop.
    let scratch = temp_home("truncation-scratch");
    let path = scratch.join("big.txt");
    let big = "x".repeat(MAX_TOOL_OUTPUT + 10_000);
    let mock = MockApi::start(vec![
        ok(tool_call_response(serde_json::json!([tool_call(
            "call-1",
            "write_file",
            &format!(r#"{{"path":"{}","content":"{}"}}"#, path.display(), big),
        )]))),
        ok(tool_call_response(serde_json::json!([tool_call(
            "call-2",
            "read_file",
            &format!(r#"{{"path":"{}"}}"#, path.display()),
        )]))),
        ok(assistant_response("Read it, partially.")),
    ]);
    let home = temp_home("truncation-run");
    let mut session = Session::create(&home, "test-model").unwrap();
    run_on(&mock, &mut session, Box::new(ApproveAll), 5, "round-trip a big file").unwrap();

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

#[test]
fn tools_execute_against_the_real_filesystem() {
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

#[test]
#[ignore = "real-API e2e; run with `cargo test --test integration -- --ignored` and DEEPSEEK_API_KEY set"]
fn live_api_completes_a_tool_task() {
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
        .unwrap();
    assert_eq!(report.stop_reason, StopReason::Completed);
    assert!(
        report.final_message.as_deref().unwrap_or_default().contains("fleet-live-ok"),
        "{report:?}"
    );
    std::fs::remove_dir_all(&home).ok();
}
