//! Test-only mock DeepSeek chat-completions server, run as a child process.
//!
//! Fleet's integration tests exercise the real HTTP client, and ureq's reads
//! misbehave on this machine when the client and an in-process loopback
//! server share one process, so the mock runs cross-process: the test spawns
//! this binary with a script and file paths for the port and the request
//! log.
//!
//! ```text
//! mock-server <responses.jsonl> <requests.jsonl> <port-file>
//! ```
//!
//! The script is JSONL, one response per line. A plain line is
//! `{"status":"200 OK","body":…}` and is answered as one document. A
//! streaming line is `{"status":"200 OK","sse":["data: {…}","data: [DONE]"]}`
//! and is answered as a chunkless SSE body: `Connection: close`, no
//! `Content-Length`, so the client reads to EOF exactly as it does against
//! the real API. Every served request is appended to the requests log as
//! `{"head":…,"body":…}` so the test can assert on what arrived.

use std::collections::VecDeque;
use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::path::Path;
use std::time::Duration;

fn main() {
    let mut args = std::env::args().skip(1);
    let script_path = args
        .next()
        .expect("usage: mock-server <responses> <requests> <port-file>");
    let requests_path = args
        .next()
        .expect("usage: mock-server <responses> <requests> <port-file>");
    let port_path = args
        .next()
        .expect("usage: mock-server <responses> <requests> <port-file>");

    let script = std::fs::read_to_string(&script_path).expect("read the response script");
    let mut responses: VecDeque<ScriptedResponse> = script
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| serde_json::from_str(line).expect("a scripted line"))
        .collect();

    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind the mock server");
    let port = listener.local_addr().unwrap().port();
    std::fs::write(&port_path, port.to_string()).expect("write the port file");
    eprintln!("[mock-server] listening on {port}");

    for stream in listener.incoming() {
        let Ok(mut stream) = stream else { break };
        let Some((head, body)) = read_request_and_body(&mut stream) else {
            // KI-018: a connection that carries no request — a bare TCP
            // probe, a health check, a scanner — is closed and the loop
            // continues. `break` here let one empty connection take the
            // mock down mid-suite, failing whichever tests were still to
            // spawn against it.
            continue;
        };
        record_request(Path::new(&requests_path), &head, &body);
        match responses.pop_front() {
            Some(scripted) => write_scripted(&mut stream, &scripted),
            None => write_response(
                &mut stream,
                "500 Internal Server Error",
                r#"{"error":{"message":"mock exhausted"}}"#,
            ),
        }
    }
}

/// One scripted response line: either a whole document or a list of raw SSE
/// frames.
#[derive(serde::Deserialize)]
struct ScriptedResponse {
    status: String,
    #[serde(default)]
    body: String,
    /// Raw `data: …` frame strings, written as a close-delimited body.
    #[serde(default)]
    sse: Vec<String>,
    /// Pause between SSE frames, simulating a model that types.
    #[serde(default)]
    delay_ms: u64,
}

/// Answer one connection from its scripted response.
fn write_scripted(stream: &mut TcpStream, scripted: &ScriptedResponse) {
    if scripted.sse.is_empty() {
        write_response(stream, &scripted.status, &scripted.body);
        return;
    }
    let head = format!(
        "HTTP/1.1 {}\r\nContent-Type: text/event-stream\r\nCache-Control: no-cache\r\nConnection: close\r\n\r\n",
        scripted.status
    );
    let _ = stream.write_all(head.as_bytes());
    for frame in &scripted.sse {
        if scripted.delay_ms > 0 {
            std::thread::sleep(Duration::from_millis(scripted.delay_ms));
        }
        let _ = stream.write_all(frame.as_bytes());
        let _ = stream.write_all(b"\n\n");
        let _ = stream.flush();
    }
    let _ = stream.shutdown(Shutdown::Write);
}

/// Read the head plus its `Content-Length`-bounded body.
fn read_request_and_body(stream: &mut std::net::TcpStream) -> Option<(String, String)> {
    let (buffer, marker) = read_until(stream, b"\r\n\r\n")?;
    let head = String::from_utf8_lossy(&buffer[..marker]).into_owned();
    let length: usize = head
        .lines()
        .find(|line| line.to_ascii_lowercase().starts_with("content-length:"))
        .and_then(|line| line.split(':').nth(1))
        .and_then(|value| value.trim().parse().ok())
        .unwrap_or(0);
    // Bytes past the marker were read together with the head; they open the body.
    let mut body = buffer[marker + 4..].to_vec();
    let mut chunk = [0u8; 8192];
    while body.len() < length {
        let read = stream.read(&mut chunk).ok()?;
        if read == 0 {
            break;
        }
        body.extend_from_slice(&chunk[..read]);
    }
    body.truncate(length);
    Some((head, String::from_utf8_lossy(&body).into_owned()))
}

/// Read until `marker` appears, returning every byte read and the marker's
/// start offset; bytes after the marker belong to the body.
fn read_until(stream: &mut std::net::TcpStream, marker: &[u8]) -> Option<(Vec<u8>, usize)> {
    let mut buffer = Vec::new();
    let mut chunk = [0u8; 8192];
    loop {
        if let Some(position) = find(&buffer, marker) {
            return Some((buffer, position));
        }
        let read = stream.read(&mut chunk).ok()?;
        if read == 0 {
            return None;
        }
        buffer.extend_from_slice(&chunk[..read]);
    }
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if haystack.len() < needle.len() {
        return None;
    }
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

/// Append one request as a JSON line so the test can assert on what arrived.
fn record_request(requests_path: &Path, head: &str, body: &str) {
    let line = serde_json::json!({ "head": head, "body": body });
    let Ok(mut log) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(requests_path)
    else {
        return;
    };
    let _ = writeln!(log, "{line}");
    let _ = log.flush();
}

fn write_response(stream: &mut std::net::TcpStream, status: &str, body: &str) {
    let head = format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let _ = stream.write_all(head.as_bytes());
    let _ = stream.write_all(body.as_bytes());
    let _ = stream.flush();
}
