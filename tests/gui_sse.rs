//! The face streams a *real* run over SSE (gui-revamp C1): a scripted
//! mock chat-completions server (the `mock-server` binary, cross-process
//! for the same ureq reason the integration tests give), a home whose
//! roster names that server, the real agent thread behind the bridge,
//! and the server's own `/events` stream read to the run's end. What the
//! assertions collect is exactly what the browser would render: the
//! session event, a turn, the streamed tokens of the answer, and the
//! run's finish.

use std::path::PathBuf;
use std::process::{Child, Command, Stdio};

/// The mock-server child, its port file, and its request log — the same
/// shape the integration tests' harness carries, kept local because the
/// gui crate's tests cannot see that file's helpers.
struct Mock {
    child: Child,
    port: u16,
    requests: PathBuf,
    scratch: PathBuf,
}

impl Drop for Mock {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_dir_all(&self.scratch);
    }
}

fn mock(stream: Vec<String>) -> Mock {
    let scratch = std::env::temp_dir().join(format!(
        "fleet-gui-sse-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir_all(&scratch).expect("the scratch");
    let script_path = scratch.join("script.jsonl");
    std::fs::write(
        &script_path,
        stream
            .into_iter()
            .map(|line| format!("{line}\n"))
            .collect::<String>(),
    )
    .unwrap();
    let requests_path = scratch.join("requests.jsonl");
    let port_path = scratch.join("port");
    let stderr_path = scratch.join("mock-stderr.log");
    let stderr_file = std::fs::File::create(&stderr_path).unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_mock-server"))
        .arg(&script_path)
        .arg(&requests_path)
        .arg(&port_path)
        .stdout(Stdio::null())
        .stderr(Stdio::from(stderr_file))
        .spawn()
        .expect("spawn the mock-server child");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    let port = loop {
        if let Ok(text) = std::fs::read_to_string(&port_path) {
            if let Ok(port) = text.trim().parse() {
                break port;
            }
        }
        if child.try_wait().unwrap().is_some() {
            let said = std::fs::read_to_string(&stderr_path).unwrap_or_default();
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
    };
    Mock {
        child,
        port,
        requests: requests_path,
        scratch,
    }
}

fn sse_line(payload: serde_json::Value) -> String {
    serde_json::json!({
        "status": "200 OK",
        "sse": [format!("data: {payload}"), "data: [DONE]"],
    })
    .to_string()
}

fn assistant_delta(field: &str, value: serde_json::Value) -> String {
    sse_line(serde_json::json!({
        "id": "1",
        "choices": [{ "delta": { field: value } }],
        "model": "m",
    }))
}

fn usage_line() -> String {
    sse_line(serde_json::json!({
        "id": "1",
        "choices": [{ "delta": {} }],
        "usage": { "prompt_tokens": 3, "completion_tokens": 4 },
        "model": "m",
    }))
}

/// The streamed run, end to end, through the face's own server.
#[tokio::test(flavor = "multi_thread")]
async fn a_real_run_streams_over_sse_tokens_turn_and_end() {
    let answer = "the streamed answer";
    // One streamed answer, then the closing turn so the run ends
    // finished rather than awaiting another tool call.
    let m = mock(vec![
        assistant_delta("content", serde_json::json!(answer)),
        usage_line(),
        assistant_delta("content", serde_json::json!("done")),
    ]);
    let home = std::env::temp_dir().join(format!(
        "fleet-gui-sse-home-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).expect("the home");
    // The roster: one candidate, the mock, the plan's default workload
    // walking it. The key env name is never set — a keyless candidate is
    // still dialable when the mock does not check the header.
    std::fs::write(
        home.join("config.json"),
        serde_json::json!({
            "providers": [{
                "id": "mock",
                "base_url": format!("http://127.0.0.1:{}/v1", m.port),
                "key_env": "FLEET_TEST_SSE_KEY_ABSENT",
                "key_required": false,
                "models": ["m"]
            }]
        })
        .to_string(),
    )
    .expect("the roster");
    std::fs::write(
        home.join("routing.json"),
        serde_json::json!({
            "workloads": [
                { "workload": "quick", "candidates": ["mock/m"] }
            ]
        })
        .to_string(),
    )
    .expect("the routing");

    // The face: the real bridge, the real agent thread, the real server.
    let (mut bridge, sources) =
        fleet::gui::bridge::Bridge::detached(home.clone(), "m".to_string(), None);
    let door = bridge.connect();
    // The handle is dropped on purpose: the thread ends when the bridge
    // does, and the mock's own Drop tears the child down after.
    fleet::gui::bridge::spawn_agent_for_tests(fleet::gui::bridge::AgentInputs {
        home: home.clone(),
        model: "m".to_string(),
        session_id: None,
        ui_tx: sources.ui,
        approval_tx: sources.seat,
        command_rx: door,
        base_url: format!("http://127.0.0.1:{}", m.port),
        memories_root: home.clone(),
        state: std::sync::Arc::clone(&bridge.state),
    });
    let (mut server, port) =
        fleet::gui::server::Server::bind("127.0.0.1", 0, std::sync::Arc::new(bridge))
            .await
            .expect("the server binds");
    server.heartbeat = std::time::Duration::from_millis(20);
    let server_task = tokio::spawn(async move { server.serve().await });

    // The run goes through the same route the composer posts to.
    let accepted = tokio::net::TcpStream::connect(("127.0.0.1", port))
        .await
        .expect("the server answers");
    // The pin is what keeps this run on the mock: the machine's real
    // sys1 judgment is live, and a `standard_code` verdict would walk the
    // compiled chain. A pin routes every shape through its provider —
    // the same lever the composer's own picker pulls.
    let body = r#"{"task":"say the streamed answer","pin":"mock"}"#;
    let head = format!(
        "POST /api/run HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}\r\n",
        body.len(),
        body
    );
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let mut conn = accepted;
    conn.write_all(head.as_bytes())
        .await
        .expect("the run posts");

    // Read the event stream beside it: one connection, held open, exactly
    // what the browser holds.
    let mut stream = tokio::net::TcpStream::connect(("127.0.0.1", port))
        .await
        .expect("the stream connects");
    stream
        .write_all(b"GET /events HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n")
        .await
        .expect("the stream asks");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
    let mut read = String::new();
    loop {
        assert!(
            std::time::Instant::now() < deadline,
            "the run never finished; the stream so far:\n{read}"
        );
        let mut chunk = [0u8; 8192];
        match tokio::time::timeout(std::time::Duration::from_secs(2), stream.read(&mut chunk)).await
        {
            Ok(Ok(0)) => break,
            Ok(Ok(n)) => read.push_str(&String::from_utf8_lossy(&chunk[..n])),
            Ok(Err(error)) => panic!("the stream broke: {error}"),
            Err(_) => continue,
        }
        if read.contains("\"type\":\"finished\"") || read.contains("\"type\":\"failed\"") {
            break;
        }
        // A failed run names its reason on the same stream; keep the
        // tail so the panic below can show it.
        if let Some(at) = read.rfind("\"type\":\"failed\"") {
            let tail = &read[at..];
            std::fs::write(m.scratch.join("sse-failed.txt"), tail).ok();
        }
    }

    // What the browser would render, all of it over the one stream.
    assert!(read.contains("Content-Type: text/event-stream"), "{read}");
    assert!(
        read.contains("\"type\":\"session\""),
        "the session lands: {read}"
    );
    assert!(read.contains("\"type\":\"turn\""), "a turn lands: {read}");
    assert!(read.contains(answer), "the streamed token lands: {read}");
    assert!(
        read.contains("\"type\":\"finished\""),
        "the run finishes on the stream; the failure, if any: {}\nSTREAM:\n{read}",
        std::fs::read_to_string(m.scratch.join("sse-failed.txt")).unwrap_or_default()
    );

    // And the mock served exactly one chat call: the run's request.
    let logged = std::fs::read_to_string(&m.requests).unwrap_or_default();
    assert_eq!(
        logged.lines().count(),
        1,
        "one model request served the run: {logged}"
    );

    server_task.abort();
    let _ = std::fs::remove_dir_all(&home);
}
