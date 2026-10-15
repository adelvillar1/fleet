//! Test-only fake MCP stdio server, run as a child process.
//!
//! Fleet's integration tests exercise the real MCP client; some client
//! paths cannot be produced deterministically by the real lane cores —
//! an unsolicited id-`null` stray frame, a child that dies mid-session, a
//! child that never answers, out-of-order replies. This binary provides
//! them, in the same cross-process style as `mock-server`.
//!
//! ```text
//! mock-mcp <script.jsonl>
//! ```
//!
//! The script is JSONL, one entry consumed per request line received.
//! Notifications (frames with no `id`) are never answered. Each entry is
//! an object with any of:
//!
//! - `reply`: the frame to write in answer to the request. Its `id` may
//!   be the string `"$echo"`, which is replaced by the request's id.
//! - `stray`: an unsolicited frame written before the reply (the shape
//!   sys1 and gen1 emit for `notifications/initialized`).
//! - `delay_ms`: pause before writing (spawned per request, so replies
//!   can land out of order).
//! - `hang`: answer nothing; the client must time out.
//! - `exit`: after handling, exit with this code (a crashed child).
//! - `huge_line_bytes`: answer with one line whose text block is this
//!   many bytes — the shape a lane streaming a runaway payload produces,
//!   for the client's per-line cap.
//! - `env`: answer with the server's own environment as the text block,
//!   so a test can prove what crossed the spawn boundary and what did
//!   not (KI-031).
//! - `on_eof`: a file to write when this server's stdin reaches EOF —
//!   the signature of a graceful teardown. A test that asks for one can
//!   prove the run closed the child it spawned instead of killing it.
//! - `stderr`: a line to write to this server's own stderr before
//!   answering — the reason a lane that dies during startup prints.
//! - `stderr_bytes`: this many bytes of padding to write to stderr
//!   before answering, the shape a lane that logs loudly produces. The
//!   client drains the pipe, so this must not wedge the call.
//!
//! A dry script exits 0.

use std::io::{BufRead, Write};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// One scripted entry. Every field is optional; an empty entry consumes
/// the request silently.
#[derive(serde::Deserialize)]
struct ScriptEntry {
    #[serde(default)]
    stray: Option<serde_json::Value>,
    #[serde(default)]
    reply: Option<serde_json::Value>,
    /// Write a reply frame whose text block is this many bytes, as one
    /// line. Past the client's cap the wait fails with a typed error
    /// instead of an unbounded read.
    #[serde(default)]
    huge_line_bytes: usize,
    /// Answer with this server's own environment, JSON-encoded.
    #[serde(default)]
    env: bool,
    #[serde(default)]
    delay_ms: u64,
    #[serde(default)]
    hang: bool,
    #[serde(default)]
    exit: Option<i32>,
    /// Write this file when stdin reaches EOF: the graceful teardown's
    /// signature, so a test can prove the run closed the child it
    /// spawned rather than killing it.
    #[serde(default)]
    on_eof: Option<String>,
    /// A line to write to this server's own stderr before answering: the
    /// shape of a lane that dies during startup saying why.
    #[serde(default)]
    stderr: Option<String>,
    /// This many bytes of padding to write to stderr before answering,
    /// in 8 KiB chunks: the shape of a lane that logs past the OS pipe
    /// buffer. The client drains stderr, so this must not wedge the call.
    #[serde(default)]
    stderr_bytes: usize,
    /// Answer only the requests whose raw line contains this substring.
    /// An entry with no `match` behaves exactly as before (consumed once,
    /// in script order). A matched entry is removed when it answers, so
    /// it is one-shot per child — each child serves exactly one ask. The
    /// judge spawns a fresh child per ask and every child reads the
    /// script from its head, so the ask's own CONTENT is the only thing
    /// that differs between two asks' children.
    #[serde(default)]
    r#match: Option<String>,
}

fn main() {
    let mut args = std::env::args().skip(1);
    let script_path = args.next().expect("usage: mock-mcp <script.jsonl>");
    let script = std::fs::read_to_string(&script_path).expect("read the script");
    let mut entries: Vec<ScriptEntry> = script
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).expect("a scripted entry"))
        .collect();

    let stdout = Arc::new(Mutex::new(std::io::stdout()));
    let stdin = std::io::stdin();
    // The last entry that named a marker owns it: the script decides when
    // the marker becomes armed, not the request order.
    let mut marker: Option<String> = None;
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let Ok(request) = serde_json::from_str::<serde_json::Value>(&line) else { continue };
        // A notification carries no id and is never answered.
        let Some(request_id) = request.get("id").cloned() else { continue };
        let Some(entry) = pop_entry(&mut entries, &line) else {
            // A dry script ends the server: the client sees a closed
            // child, which is the honest answer to an unscripted request.
            std::process::exit(0);
        };
        if entry.on_eof.is_some() {
            marker = entry.on_eof.clone();
        }
        let stdout = Arc::clone(&stdout);
        // A hanging entry never answers, and the server keeps reading so
        // the client's timeout — not a dead pipe — ends the wait.
        let hangs = entry.hang;
        std::thread::spawn(move || {
            if hangs {
                return;
            }
            // Whatever this entry says on stderr comes first: a lane's
            // reason precedes its answer, and a lane that logs a lot
            // writes it in chunks the way a real logger does.
            if let Some(line) = &entry.stderr {
                let _ = writeln!(std::io::stderr(), "{line}");
                let _ = std::io::stderr().flush();
            }
            if entry.stderr_bytes > 0 {
                write_stderr_padding(entry.stderr_bytes);
            }
            if entry.delay_ms > 0 {
                std::thread::sleep(Duration::from_millis(entry.delay_ms));
            }
            if let Some(stray) = &entry.stray {
                write_frame(&stdout, stray);
            }
            if let Some(reply) = &entry.reply {
                let reply = with_echoed_id(reply, &request_id);
                write_frame(&stdout, &reply);
            }
            if entry.huge_line_bytes > 0 {
                let reply = serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": request_id,
                    "result": { "content": [{ "type": "text", "text": "x".repeat(entry.huge_line_bytes) }] },
                });
                write_frame(&stdout, &reply);
            }
            if entry.env {
                let reply = serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": request_id,
                    "result": { "content": [{ "type": "text", "text": own_environment() }] },
                });
                write_frame(&stdout, &reply);
            }
            if let Some(code) = entry.exit {
                // Flush whatever this entry wrote before the process dies.
                if let Ok(mut guard) = stdout.lock() {
                    let _ = guard.flush();
                }
                std::process::exit(code);
            }
        });
    }
    // stdin reached EOF: the graceful teardown's signature. A scripted
    // marker lands here, and only here — a killed child never writes one,
    // which is exactly what makes the marker worth asserting on.
    if let Some(path) = marker {
        let _ = std::fs::write(path, "closed\n");
    }
}

/// Write `bytes` of padding to this server's stderr, in 8 KiB chunks.
///
/// Chunked on purpose: one giant `write_all` is not what a logging lane
/// does, and the interesting property under test is that the *pipe* fills
/// up while the client keeps reading it.
fn write_stderr_padding(bytes: usize) {
    let chunk = vec![b'e'; 8 * 1024];
    let mut written = 0;
    while written < bytes {
        let take = (bytes - written).min(chunk.len());
        if std::io::stderr().write_all(&chunk[..take]).is_err() {
            break;
        }
        written += take;
    }
    let _ = std::io::stderr().flush();
}

/// This server's own environment as one JSON object: names only where a
/// value would be a secret is not the point — a test needs the names to
/// prove which crossed the spawn boundary, and the values are the test's
/// own fixtures.
fn own_environment() -> String {
    let map: std::collections::BTreeMap<String, String> = std::env::vars_os()
        .map(|(name, value)| {
            (
                name.to_string_lossy().into_owned(),
                value.to_string_lossy().into_owned(),
            )
        })
        .collect();
    serde_json::to_string(&map).expect("the environment is serializable JSON")
}

/// Pop the entry that answers this request, or `None` when the script
/// is dry. An entry carrying a `match` answers only the requests whose
/// raw line contains that substring and is consumed when it does; an
/// entry without one is consumed by the next request in script order,
/// exactly as before. Every existing script has no `match`, so the
/// position is always 0 and the behaviour is byte-identical.
fn pop_entry(entries: &mut Vec<ScriptEntry>, line: &str) -> Option<ScriptEntry> {
    if entries.is_empty() {
        return None;
    }
    let position = entries
        .iter()
        .position(|entry| entry.r#match.as_deref().is_none_or(|needle| line.contains(needle)))?;
    Some(entries.remove(position))
}

/// A reply frame whose `id` is the string `"$echo"` takes the request's
/// id, so one script entry can serve whichever in-flight request it
/// answers.
fn with_echoed_id(reply: &serde_json::Value, request_id: &serde_json::Value) -> serde_json::Value {
    let mut reply = reply.clone();
    if reply.get("id").and_then(serde_json::Value::as_str) == Some("$echo") {
        reply["id"] = request_id.clone();
    }
    reply
}

/// Write one frame as a newline-terminated line, serialized against the
/// other writers.
fn write_frame(stdout: &Mutex<std::io::Stdout>, frame: &serde_json::Value) {
    let line = serde_json::to_string(frame).expect("a frame is serializable JSON");
    let mut guard = stdout.lock().expect("the stdout lock");
    let _ = writeln!(guard, "{line}");
    let _ = guard.flush();
}
