//! The MCP stdio client: a hand-rolled, async, newline-delimited JSON-RPC
//! 2.0 speaker for Model Context Protocol servers (protocol version
//! 2024-11-05).
//!
//! The client spawns a server as a child process (`tokio::process`), writes
//! requests as one JSON object per line to its stdin, and runs a reader
//! task that drains stdout into an id-keyed map of pending waiters. There
//! are no Content-Length headers — MCP stdio framing is newline-delimited
//! JSON — and requests correlate by `id`: notifications carry none, and
//! unknown notifications (including the id-`null` stray frames sys1 and
//! gen1 emit in answer to `notifications/initialized`, KI-002) are ignored.
//!
//! stderr is drained too, for the child's whole life, into a bounded tail:
//! a pipe nobody reads is a pipe a chatty child fills and then blocks on,
//! and the tail is where a lane's startup failure says why (KI-043).
//!
//! Every wait carries a budget, so a silent server is a timeout and a dead
//! one an error — never a hang. Teardown is the graceful path first (close
//! stdin, wait, kill on expiry); `Drop` is the best-effort net under it.
//!
//! A child starts from an empty environment, not the caller's: `env_clear`
//! plus an explicitly built one (the base, the spec's own entries, the
//! lane's declared passthrough). Inheritance is an accident of
//! `tokio::process::Command`, not a decision — see KI-031.

use std::collections::HashMap;
use std::ffi::{OsStr, OsString};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use serde_json::{json, Value};
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStderr, ChildStdin, ChildStdout, Command};
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tokio::time::timeout;

/// The MCP protocol version this client speaks.
pub const PROTOCOL_VERSION: &str = "2024-11-05";

/// The initialize budget. Spawning a Python interpreter and importing a
/// stdlib-only package is sub-second on a healthy machine; ten seconds is
/// generous headroom for a loaded one.
pub const INITIALIZE_TIMEOUT: Duration = Duration::from_secs(10);

/// The tools/list budget: same shape of work as initialize.
pub const LIST_TIMEOUT: Duration = Duration::from_secs(10);

/// The tools/call budget. A call does the lane's real work — embedding a
/// batch, classifying a diff, generating an image — so it gets the long
/// budget the headless agent already gives its own tools.
pub const CALL_TIMEOUT: Duration = Duration::from_secs(60);

/// How long a closing server gets to exit after stdin closes before it is
/// killed. Every vendored lane exits 0 on EOF, so this only ever fires on
/// a wedged child.
pub const TEARDOWN_GRACE: Duration = Duration::from_secs(5);

/// The largest single stdout line the reader will assemble.
///
/// One MCP frame is one line, and the biggest a lane legitimately emits
/// is a batch of embedding vectors — kilobytes. 8 MiB is three orders of
/// magnitude over that and still bounded, so a lane that streams one
/// unterminated line (or a gigabyte of base64 with no newline in it)
/// fails the wait with a typed protocol error instead of growing the
/// reader's buffer until the process dies.
pub const MAX_LINE_BYTES: usize = 8 * 1024 * 1024;

/// How much of a child's stderr is kept, and why it is kept at all.
///
/// The pipe is the hazard, not the logging: nothing used to read a lane's
/// stderr, so a lane that wrote more than the OS pipe buffer (64 KiB on
/// macOS and Linux) blocked on its next write and stayed blocked until
/// the call budget expired — a wedged child that looked exactly like a
/// hung one. The drain below reads the pipe for the child's whole life,
/// which is what keeps a chatty lane answering.
///
/// What it keeps is the tail, not the head: a lane that dies during
/// startup prints its reason *last*, after whatever it printed on the way
/// in — a Python import traceback lands at the end. So the last 64 KiB
/// is where the reason is, and 64 KiB is a bound rather than a budget.
const STDERR_TAIL_BYTES: usize = 64 * 1024;

/// The bounded tail of one child's stderr.
#[derive(Debug, Default)]
struct StderrTail {
    bytes: Vec<u8>,
}

impl StderrTail {
    /// Keep the last `STDERR_TAIL_BYTES` of everything pushed, dropping
    /// from the front whatever no longer fits.
    fn push(&mut self, chunk: &[u8]) {
        self.bytes.extend_from_slice(chunk);
        let excess = self.bytes.len().saturating_sub(STDERR_TAIL_BYTES);
        if excess > 0 {
            self.bytes.drain(..excess);
        }
    }

    /// What is kept, as text. Lossy on purpose: a lane's stderr is a
    /// human's diagnostic, not a protocol frame.
    fn text(&self) -> String {
        String::from_utf8_lossy(&self.bytes).into_owned()
    }
}

/// The ambient names every MCP child gets, when the caller has them.
///
/// `PATH` first: the lanes launch as `python3`, resolved through the
/// child's PATH, so a lane is only findable if the caller's PATH carried
/// over. `HOME` because the Python cores resolve their config, cache and
/// store roots through `Path.home()`. `LANG` and `TMPDIR` because a lane
/// that shells out (sem1's st-worker venv) otherwise loses the caller's
/// locale and scratch directory. Everything else — every
/// credential-shaped name included — is left behind (KI-031).
const BASE_ENV: &[&str] = &["PATH", "HOME", "LANG", "TMPDIR"];

/// The child's whole environment: the minimal base, the lane's declared
/// passthrough copied from the caller only when present, and the spec's
/// own entries last so the lane's own declaration wins.
///
/// Pure over an injected parent environment, which is what makes the
/// policy testable without mutating the test process's environment.
pub fn child_env(
    spec: &ServerSpec,
    parent: &HashMap<OsString, OsString>,
) -> Vec<(OsString, OsString)> {
    let mut env: Vec<(OsString, OsString)> = Vec::new();
    for name in BASE_ENV {
        if let Some(value) = parent.get(OsStr::new(name)) {
            env.push((OsString::from(name), value.clone()));
        }
    }
    // The lane's declared passthrough: an ambient name the lane
    // legitimately needs, copied only when the caller actually has it.
    // Anything not named here — `DEEPSEEK_API_KEY` included — never
    // reaches the child.
    for name in &spec.passthrough {
        // The spec's own declaration wins, and it wins once: a name in the
        // child's environment twice is an ambiguity, not a precedence.
        if spec.env.iter().any(|(declared, _)| declared == name) {
            continue;
        }
        // A declared name ending in `_` is a NAMESPACE, not a name:
        // every ambient name under it crosses. KI-039 — sys1 reads its
        // whole `SYS1_` namespace (`SYS1_PROVIDERS__JEV_MODEL`,
        // `SYS1_CLASSIFY__LOG_DIR`, any section and key), and a
        // name-based allowlist can only carry the names someone
        // remembered to declare. A namespace declaration carries them
        // all, and the section/key shape is the lane's own.
        if let Some(prefix) = name.strip_suffix('_') {
            for (ambient, value) in parent {
                let Some(ambient) = ambient.to_str() else {
                    continue;
                };
                // Strictly under the namespace — the bare prefix is not
                // a name — and not already carried, by an exact entry
                // earlier in the list or the spec's own declaration.
                if !ambient.starts_with(prefix) || ambient.len() <= prefix.len() {
                    continue;
                }
                if env
                    .iter()
                    .any(|(carried, _)| carried == OsStr::new(ambient))
                {
                    continue;
                }
                if spec.env.iter().any(|(declared, _)| declared == ambient) {
                    continue;
                }
                env.push((OsString::from(ambient), value.clone()));
            }
            continue;
        }
        if let Some(value) = parent.get(OsStr::new(name)) {
            env.push((OsString::from(name), value.clone()));
        }
    }
    for (name, value) in &spec.env {
        env.push((OsString::from(name), OsString::from(value)));
    }
    env
}

/// Everything needed to spawn one MCP stdio server. Built by the lane
/// registry (`crate::lanes`); the client itself knows no lane names.
#[derive(Debug, Clone)]
pub struct ServerSpec {
    /// The program to run, resolved through PATH (e.g. `python3`).
    pub program: String,
    /// The program's arguments (e.g. `["-u", "<home>/service/mcp_server.py"]`).
    pub args: Vec<String>,
    /// Extra environment entries for the child (e.g. `SEM1_PYTHONPATH`).
    pub env: Vec<(String, String)>,
    /// Ambient names this server legitimately needs, copied from the
    /// caller's environment only when present. The child gets the base
    /// plus these plus `env` — never the caller's whole environment
    /// (KI-031).
    pub passthrough: Vec<String>,
    /// The child's working directory (the lane's home).
    pub cwd: Option<std::path::PathBuf>,
}

/// Per-operation wait budgets. The defaults are the constants above;
/// tests shorten them so a timeout path runs in milliseconds.
#[derive(Debug, Clone, Copy)]
pub struct Timeouts {
    pub initialize: Duration,
    pub list: Duration,
    pub call: Duration,
}

impl Default for Timeouts {
    fn default() -> Timeouts {
        Timeouts {
            initialize: INITIALIZE_TIMEOUT,
            list: LIST_TIMEOUT,
            call: CALL_TIMEOUT,
        }
    }
}

/// What one `initialize` answered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InitializeResult {
    /// The protocol version the server picked.
    pub protocol_version: String,
    /// The server's self-reported name.
    pub server_name: String,
    /// The server's self-reported version.
    pub server_version: String,
}

/// One tool as `tools/list` describes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolInfo {
    pub name: String,
    pub description: String,
    /// The tool's JSON Schema for its arguments, verbatim.
    pub input_schema: Value,
}

/// One content block of a `tools/call` result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentBlock {
    /// The block's type (usually `text`).
    pub content_type: String,
    /// The block's payload, kept as raw JSON so no shape is assumed.
    pub data: Value,
}

/// What one `tools/call` answered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallResult {
    pub content: Vec<ContentBlock>,
    /// The server's `isError` flag: the call ran, the tool refused.
    pub is_error: bool,
    /// The result's `structuredContent`, when the server sent one. The
    /// text blocks stay the model's primary reading; this is the typed
    /// answer beside them and the fallback when there is no text.
    pub structured: Option<Value>,
}

impl CallResult {
    /// The text one call answered with, in the shape the agent's tool
    /// results use: every text block joined by a newline, every non-text
    /// block noted by its type (`[image]`), so the model knows something
    /// came back that it cannot read rather than seeing nothing at all.
    /// A content-less result says so instead of reading as an empty
    /// success.
    pub fn text(&self) -> String {
        self.render(None)
    }

    /// The text, with image blocks written to `dir` and named by path.
    /// The model cannot read base64, and a lane that returns a picture
    /// has produced an artifact, not a `[image]` placeholder.
    pub fn text_with_artifacts(&self, dir: &Path) -> String {
        self.render(Some(dir))
    }

    fn render(&self, images: Option<&Path>) -> String {
        let mut parts: Vec<String> = Vec::with_capacity(self.content.len());
        for (index, block) in self.content.iter().enumerate() {
            if block.content_type == "text" {
                let text = block.data.as_str().unwrap_or_default();
                if !text.is_empty() {
                    parts.push(text.to_string());
                }
            } else if block.content_type == "image" {
                if let Some(dir) = images {
                    let mime = block
                        .data
                        .get("mimeType")
                        .and_then(Value::as_str)
                        .unwrap_or("image/png");
                    let encoded = block
                        .data
                        .get("data")
                        .and_then(Value::as_str)
                        .unwrap_or_default();
                    let extension = match mime {
                        "image/jpeg" => "jpg",
                        "image/gif" => "gif",
                        "image/webp" => "webp",
                        _ => "png",
                    };
                    if let Some(bytes) = decode_base64(encoded) {
                        if std::fs::create_dir_all(dir).is_ok() {
                            let path = dir.join(format!("image-{index}.{extension}"));
                            if std::fs::write(&path, bytes).is_ok() {
                                parts.push(format!("[image saved: {}]", path.display()));
                                continue;
                            }
                        }
                    }
                }
                parts.push("[image]".to_string());
            } else if !block.content_type.is_empty() {
                parts.push(format!("[{}]", block.content_type));
            }
        }
        if parts.is_empty() {
            if let Some(structured) = &self.structured {
                return serde_json::to_string(structured)
                    .unwrap_or_else(|_| "(the tool returned structured content)".to_string());
            }
            return "(the tool returned no content)".to_string();
        }
        parts.join("\n")
    }
}

/// Decode the standard base64 an MCP image block carries. `None` on a
/// malformed payload, which the caller reports as `[image]` rather than
/// writing a corrupt file.
fn decode_base64(input: &str) -> Option<Vec<u8>> {
    fn value(byte: u8) -> Option<u32> {
        match byte {
            b'A'..=b'Z' => Some((byte - b'A') as u32),
            b'a'..=b'z' => Some((byte - b'a') as u32 + 26),
            b'0'..=b'9' => Some((byte - b'0') as u32 + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    }
    let mut out = Vec::new();
    let mut buffer = 0u32;
    let mut bits = 0u32;
    for byte in input.bytes() {
        if byte == b'=' || byte.is_ascii_whitespace() {
            continue;
        }
        let value = value(byte)?;
        buffer = (buffer << 6) | value;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buffer >> bits) as u8);
        }
    }
    Some(out)
}

/// The client's failures. Typed values, never panics: a crashed child is
/// an error here, not a hang.
#[derive(Debug, Clone)]
pub enum McpError {
    /// The child could not be spawned (no `python3` on PATH, bad path).
    /// The error rides an `Arc` so the enum can derive `Clone` (KI-040):
    /// the reader builds one error per pending waiter when it drains
    /// them, and a cloneable error is one value shared rather than one
    /// built per waiter.
    Spawn(std::sync::Arc<std::io::Error>),
    /// The child's pipes failed (broken pipe on write, read error).
    Io(std::sync::Arc<std::io::Error>),
    /// The wait budget expired before the server answered.
    Timeout { method: String, budget: Duration },
    /// The child exited, or closed its pipes, before answering.
    Closed(String),
    /// The server answered with a JSON-RPC `error` object.
    Rpc { code: i64, message: String },
    /// The answer was not the shape the method promises.
    Protocol(String),
}

impl std::fmt::Display for McpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            McpError::Spawn(error) => write!(f, "could not spawn the MCP server: {error}"),
            McpError::Io(error) => write!(f, "the MCP server's pipes failed: {error}"),
            McpError::Timeout { method, budget } => {
                write!(f, "`{method}` timed out after {}s", budget.as_secs())
            }
            McpError::Closed(reason) => write!(f, "the MCP server closed: {reason}"),
            McpError::Rpc { code, message } => {
                write!(f, "the MCP server errored ({code}): {message}")
            }
            McpError::Protocol(detail) => write!(f, "malformed MCP reply: {detail}"),
        }
    }
}

impl std::error::Error for McpError {}

/// Lock a mutex, recovering from poisoning: a panic while the lock was
/// held must not turn every later call into another panic. The recovery
/// is loud — one line on stderr — because a poisoned guard means some
/// invariant the mutex protects was already broken, and silently
/// continuing on is how that stays invisible. The crate has no logger;
/// a CLI's stderr is its log.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|poison| {
        eprintln!("fleet: a mutex was poisoned by a panic; recovering the value it guards");
        PoisonError::into_inner(poison)
    })
}

/// The pending waiters, shared between the server and its reader task:
/// request id to the one-shot that answers it.
type Pending = Arc<Mutex<HashMap<u64, oneshot::Sender<Result<Value, McpError>>>>>;

/// One MCP server child, with its reader task and pending waiters.
pub struct McpServer {
    child: Mutex<Option<Child>>,
    /// A tokio mutex, not a std one: the guard is held across the write's
    /// await, which is only sound — and only `Send` — here. A std guard
    /// across an await made the call future `!Send`, which slice 5's
    /// `tokio::spawn`ed concurrent calls cannot compile against.
    stdin: tokio::sync::Mutex<Option<ChildStdin>>,
    /// Waiters keyed by request id. The reader task owns the only other
    /// handle; an entry lives from the write until its reply, its timeout,
    /// or the child's death.
    pending: Pending,
    next_id: AtomicU64,
    timeouts: Timeouts,
    reader: JoinHandle<()>,
    /// The child's stderr, drained for its whole life into a bounded tail
    /// (KI-043). The drain task holds the other handle; the tail is what a
    /// handshake failure reports when the child said why on the way out.
    stderr: Arc<Mutex<StderrTail>>,
    /// The stderr drain. Aborted with the reader, on the same paths.
    stderr_drain: JoinHandle<()>,
}

impl McpServer {
    /// Spawn `spec` as a child and start its reader task. The child is not
    /// yet initialized — call [`McpServer::initialize`] next.
    pub fn spawn(spec: &ServerSpec) -> Result<McpServer, McpError> {
        Self::spawn_with(spec, Timeouts::default())
    }

    /// `spawn` with explicit budgets (tests shorten them).
    pub fn spawn_with(spec: &ServerSpec, timeouts: Timeouts) -> Result<McpServer, McpError> {
        let parent: HashMap<OsString, OsString> = std::env::vars_os().collect();
        let mut command = Command::new(&spec.program);
        command.args(&spec.args);
        // The child starts from an empty environment and is handed exactly
        // what `child_env` builds. Without `env_clear` the lane would see
        // the caller's whole environment — `DEEPSEEK_API_KEY` and every
        // other credential-shaped name on the machine included (KI-031).
        command.env_clear();
        for (name, value) in child_env(spec, &parent) {
            command.env(name, value);
        }
        if let Some(cwd) = &spec.cwd {
            command.current_dir(cwd);
        }
        // stdin and stdout are the protocol; stderr is drained for the
        // child's whole life into a bounded tail, so a lane that logs
        // past the pipe buffer cannot wedge itself on a full pipe and a
        // handshake failure can still report what the child said on its
        // way out (KI-043).
        command.stdin(std::process::Stdio::piped());
        command.stdout(std::process::Stdio::piped());
        command.stderr(std::process::Stdio::piped());
        let mut child = command
            .spawn()
            .map_err(|error| McpError::Spawn(std::sync::Arc::new(error)))?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| McpError::Protocol("the child's stdin was not piped".to_string()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| McpError::Protocol("the child's stdout was not piped".to_string()))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| McpError::Protocol("the child's stderr was not piped".to_string()))?;
        let pending: Pending = Arc::new(Mutex::new(HashMap::new()));
        let reader = tokio::spawn(read_frames(stdout, Arc::clone(&pending)));
        let stderr_tail: Arc<Mutex<StderrTail>> = Arc::new(Mutex::new(StderrTail::default()));
        let stderr_drain = tokio::spawn(drain_stderr(stderr, Arc::clone(&stderr_tail)));
        Ok(McpServer {
            child: Mutex::new(Some(child)),
            stdin: tokio::sync::Mutex::new(Some(stdin)),
            pending,
            next_id: AtomicU64::new(1),
            timeouts,
            reader,
            stderr: stderr_tail,
            stderr_drain,
        })
    }

    /// Perform the MCP handshake: `initialize`, then the
    /// `notifications/initialized` notification (never answered; sys1 and
    /// gen1 answer it with a `-32601` stray frame the reader ignores).
    ///
    /// A failure here carries the child's stderr tail when it wrote one:
    /// startup is where a lane says why it cannot start, and it says it on
    /// stderr, never on the protocol pipe (KI-043).
    pub async fn initialize(&self) -> Result<InitializeResult, McpError> {
        // One seam for the whole handshake: whichever step fails, the
        // child's stderr tail rides along when it wrote one.
        let handshake = async {
            let result = self
                .request(
                    "initialize",
                    json!({
                        "protocolVersion": PROTOCOL_VERSION,
                        "capabilities": {},
                        "clientInfo": { "name": "fleet", "version": env!("CARGO_PKG_VERSION") },
                    }),
                    self.timeouts.initialize,
                )
                .await?;
            let initialize = parse_initialize(result)?;
            self.notify("notifications/initialized").await?;
            Ok(initialize)
        };
        match handshake.await {
            Ok(initialize) => Ok(initialize),
            Err(error) => Err(annotated(error, &self.stderr_tail())),
        }
    }

    /// The server's tool roster. Part of the startup handshake, so a
    /// failure here carries the child's stderr tail too (KI-043).
    pub async fn list_tools(&self) -> Result<Vec<ToolInfo>, McpError> {
        let mut roster: Vec<ToolInfo> = Vec::new();
        let mut cursor: Option<String> = None;
        // Cursor pages, bounded: a server that keeps answering a cursor
        // cannot make the join unbounded.
        for _ in 0..10 {
            let params = match &cursor {
                Some(cursor) => json!({ "cursor": cursor }),
                None => json!({}),
            };
            let result = match self.request("tools/list", params, self.timeouts.list).await {
                Ok(result) => result,
                Err(error) => return Err(annotated(error, &self.stderr_tail())),
            };
            roster.extend(parse_tools(result.clone())?);
            cursor = result
                .get("nextCursor")
                .and_then(Value::as_str)
                .filter(|cursor| !cursor.is_empty())
                .map(str::to_string);
            if cursor.is_none() {
                break;
            }
        }
        Ok(roster)
    }

    /// The last `STDERR_TAIL_BYTES` the child wrote to stderr, as text.
    fn stderr_tail(&self) -> String {
        lock(&self.stderr).text()
    }

    /// Call one tool. A JSON-RPC error (bad arguments, unknown tool) comes
    /// back as [`McpError::Rpc`]; a tool-level refusal comes back as
    /// [`CallResult::is_error`].
    pub async fn call_tool(&self, name: &str, arguments: Value) -> Result<CallResult, McpError> {
        let result = self
            .request(
                "tools/call",
                json!({ "name": name, "arguments": arguments }),
                self.timeouts.call,
            )
            .await?;
        parse_call(result)
    }

    /// The graceful teardown: close stdin (every lane server exits 0 on
    /// EOF), wait up to `TEARDOWN_GRACE`, then kill a child that outstayed
    /// it. Consumes the server.
    pub async fn close(self) -> Result<(), McpError> {
        // Dropping the stdin handle is the EOF signal.
        drop(self.stdin.lock().await.take());
        let child = lock(&self.child).take();
        self.reader.abort();
        self.stderr_drain.abort();
        let Some(mut child) = child else {
            return Ok(());
        };
        match timeout(TEARDOWN_GRACE, child.wait()).await {
            Ok(Ok(_)) => Ok(()),
            Ok(Err(error)) => Err(McpError::Io(std::sync::Arc::new(error))),
            Err(_elapsed) => {
                // The grace expired: kill, and report a failure only when
                // even the kill goes wrong.
                child
                    .kill()
                    .await
                    .map_err(|error| McpError::Io(std::sync::Arc::new(error)))?;
                child
                    .wait()
                    .await
                    .map_err(|error| McpError::Io(std::sync::Arc::new(error)))?;
                Ok(())
            }
        }
    }

    /// Write one request and wait for its correlated reply. Registers the
    /// waiter *before* the frame is written, so a fast reply cannot arrive
    /// before there is a sender to receive it.
    async fn request(
        &self,
        method: &str,
        params: Value,
        budget: Duration,
    ) -> Result<Value, McpError> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = oneshot::channel();
        lock(&self.pending).insert(id, tx);
        let frame = json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params });
        if let Err(error) = self.write_frame(&frame).await {
            lock(&self.pending).remove(&id);
            return Err(error);
        }
        match timeout(budget, rx).await {
            Ok(Ok(outcome)) => outcome,
            // The reader drained the waiters: the child died mid-request.
            Ok(Err(_dropped)) => Err(McpError::Closed(format!(
                "no reply to `{method}` before the child closed its output"
            ))),
            Err(_elapsed) => {
                // A late reply now finds no waiter and is ignored. The
                // server is told the request is gone, best effort and
                // bounded: a wedged child must not extend the wait it
                // already blew.
                lock(&self.pending).remove(&id);
                let cancellation = json!({
                    "jsonrpc": "2.0",
                    "method": "notifications/cancelled",
                    "params": { "requestId": id, "reason": "timeout" },
                });
                let _ = timeout(Duration::from_millis(250), self.write_frame(&cancellation)).await;
                Err(McpError::Timeout {
                    method: method.to_string(),
                    budget,
                })
            }
        }
    }

    /// Write one notification: a frame with no id, never answered.
    async fn notify(&self, method: &str) -> Result<(), McpError> {
        self.write_frame(&json!({ "jsonrpc": "2.0", "method": method }))
            .await
    }

    /// Write one frame as a single newline-terminated line and flush.
    async fn write_frame(&self, frame: &Value) -> Result<(), McpError> {
        let mut line = serde_json::to_string(frame).expect("a frame is serializable JSON");
        line.push('\n');
        // The guard is held across the write's await on purpose: that is
        // what serializes two concurrent calls onto one stdin. A tokio
        // mutex makes it sound and keeps the future `Send`; a std guard
        // here did neither.
        let mut guard = self.stdin.lock().await;
        let Some(stdin) = guard.as_mut() else {
            return Err(McpError::Closed("the server's stdin is closed".to_string()));
        };
        stdin
            .write_all(line.as_bytes())
            .await
            .map_err(|error| McpError::Io(std::sync::Arc::new(error)))?;
        stdin
            .flush()
            .await
            .map_err(|error| McpError::Io(std::sync::Arc::new(error)))
    }
}

impl Drop for McpServer {
    fn drop(&mut self) {
        // The best-effort net when `close` was not awaited: signal EOF,
        // then kill a child still running. A synchronous wait would block
        // the caller's thread (and cannot run inside a runtime), so the
        // graceful path is `close`'s job.
        self.reader.abort();
        self.stderr_drain.abort();
        // The reader is stopped, so a lock it held is free; one another
        // task still holds is not worth waiting for in a destructor. The
        // child is killed either way, which closes the pipe from the
        // other end, and the field drops with the struct.
        if let Ok(mut guard) = self.stdin.try_lock() {
            guard.take();
        }
        if let Some(child) = lock(&self.child).as_mut() {
            let _ = child.start_kill();
        }
    }
}

/// Parse an `initialize` result. A server that omits `serverInfo` still
/// initializes; the name and version read `unknown`.
fn parse_initialize(result: Value) -> Result<InitializeResult, McpError> {
    let protocol_version = result
        .get("protocolVersion")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            McpError::Protocol("initialize answered without a protocolVersion".to_string())
        })?
        .to_string();
    let server = result.get("serverInfo");
    let server_name = server
        .and_then(|info| info.get("name"))
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_string();
    let server_version = server
        .and_then(|info| info.get("version"))
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_string();
    Ok(InitializeResult {
        protocol_version,
        server_name,
        server_version,
    })
}

/// Parse a `tools/list` result into the roster.
fn parse_tools(result: Value) -> Result<Vec<ToolInfo>, McpError> {
    let tools = result
        .get("tools")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            McpError::Protocol("tools/list answered without a tools array".to_string())
        })?;
    let mut roster = Vec::with_capacity(tools.len());
    for tool in tools {
        roster.push(ToolInfo {
            name: tool
                .get("name")
                .and_then(Value::as_str)
                .ok_or_else(|| McpError::Protocol("a tool without a name".to_string()))?
                .to_string(),
            description: tool
                .get("description")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            input_schema: tool.get("inputSchema").cloned().unwrap_or(Value::Null),
        });
    }
    Ok(roster)
}

/// Parse a `tools/call` result. The content is kept as raw JSON per
/// block, so no tool's payload shape is assumed here.
fn parse_call(result: Value) -> Result<CallResult, McpError> {
    // A lane that answers a failed call with a JSON-RPC error nested
    // under `result` — the vendored sys1 lane's shape (KI-191): the
    // lane's own words ARE the refusal. Reading it as a content-less
    // success drops the lane's error, and the caller then names a
    // parse failure ("not the structured content it promises") over
    // what the lane actually said. The message rides as the call's
    // text with `is_error` set, so every caller's existing refusal
    // path names the lane's own words.
    if let Some(error) = result.get("error") {
        let message = error
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("the lane returned an error carrying no message");
        return Ok(CallResult {
            content: vec![ContentBlock {
                content_type: "text".to_string(),
                data: Value::String(message.to_string()),
            }],
            is_error: true,
            structured: None,
        });
    }
    let content = result
        .get("content")
        .and_then(Value::as_array)
        .map(|blocks| {
            blocks
                .iter()
                .map(|block| {
                    let content_type = block
                        .get("type")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string();
                    // A text block carries its string; every other block
                    // keeps its whole shape, so an image's mime type and
                    // base64 payload are still there to write out.
                    let data = if content_type == "text" {
                        block.get("text").cloned().unwrap_or(Value::Null)
                    } else {
                        block.clone()
                    };
                    ContentBlock { content_type, data }
                })
                .collect()
        })
        .unwrap_or_default();
    Ok(CallResult {
        content,
        is_error: result
            .get("isError")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        structured: result
            .get("structuredContent")
            .cloned()
            .filter(|value| !value.is_null()),
    })
}

/// The stderr drain: read the child's stderr to the end of its life,
/// keeping only the bounded tail (KI-043).
///
/// Detached on purpose — nothing waits on it — because the point is that
/// the pipe always has a reader. A child that logs a gigabyte of warnings
/// is drained, not wedged: the reads are fixed-size, the tail is capped,
/// and the task ends by itself when the child's stderr closes (its exit,
/// its death, or the teardown's kill).
async fn drain_stderr(stderr: ChildStderr, tail: Arc<Mutex<StderrTail>>) {
    let mut stderr = stderr;
    let mut chunk = vec![0u8; 8 * 1024];
    loop {
        match stderr.read(&mut chunk).await {
            // EOF: the child's stderr is gone, which is the end of the drain.
            Ok(0) => break,
            Ok(read) => lock(&tail).push(&chunk[..read]),
            // A pipe error is EOF's shape here: the child is gone.
            Err(_) => break,
        }
    }
}

/// A handshake failure with the child's stderr tail attached, when it
/// wrote one.
///
/// Startup is the moment a lane's stderr carries its reason: a missing
/// venv, an unreadable config, a traceback on import — all of it lands on
/// stderr, none of it on the protocol pipe, so without this the failure
/// reads "the child closed its output" and nothing more. An empty tail
/// leaves the error exactly as it was, which is why a silent child's
/// messages are unchanged.
///
/// A timeout keeps its own shape: the budget and the step that blew it
/// are the fact, and the variant carries no message to append to.
fn annotated(error: McpError, tail: &str) -> McpError {
    if tail.is_empty() {
        return error;
    }
    let suffix = format!("; its stderr said: {tail}");
    match error {
        McpError::Spawn(error) => {
            McpError::Spawn(std::sync::Arc::new(annotated_io(&error, &suffix)))
        }
        McpError::Io(error) => McpError::Io(std::sync::Arc::new(annotated_io(&error, &suffix))),
        McpError::Timeout { .. } => error,
        McpError::Closed(reason) => McpError::Closed(format!("{reason}{suffix}")),
        McpError::Rpc { code, message } => McpError::Rpc {
            code,
            message: format!("{message}{suffix}"),
        },
        McpError::Protocol(detail) => McpError::Protocol(format!("{detail}{suffix}")),
    }
}

/// The same `io::Error` with a longer message. The kind survives: a
/// caller matches on it, and annotating the text must not cost that.
fn annotated_io(error: &std::io::Error, suffix: &str) -> std::io::Error {
    std::io::Error::new(error.kind(), format!("{error}{suffix}"))
}

/// Why the reader stopped draining the child's stdout. A waiter is never
/// left hanging, and it is told which of these happened rather than a
/// single "closed" that would misdescribe two of them.
#[derive(Debug, Clone, Copy)]
enum OutputEnd {
    /// The output ended: EOF, or a pipe error, which is the same shape to
    /// a waiter.
    Closed,
    /// The child sent a line that is not valid UTF-8.
    NotUtf8,
    /// The child sent one line longer than `MAX_LINE_BYTES`.
    TooLong,
}

impl OutputEnd {
    /// What every pending waiter is failed with.
    fn error(self) -> McpError {
        match self {
            OutputEnd::Closed => {
                McpError::Closed("the child closed its output before replying".to_string())
            }
            OutputEnd::NotUtf8 => {
                McpError::Protocol("the MCP server sent a line that is not valid UTF-8".to_string())
            }
            OutputEnd::TooLong => McpError::Protocol(format!(
                "the MCP server sent one line longer than the {} MiB cap",
                MAX_LINE_BYTES / (1024 * 1024)
            )),
        }
    }
}

/// One bounded read from the child's stdout.
enum LineRead {
    /// A complete line landed in `buffer`, its newline included.
    Frame,
    /// The output ended.
    Eof,
    /// A pipe error: the output is gone, which is `Eof`'s shape.
    Error,
    /// One line passed `MAX_LINE_BYTES`: `buffer` holds the first
    /// `MAX_LINE_BYTES` bytes and the rest is abandoned.
    TooLong,
}

/// Read one newline-terminated line into `buffer`, refusing to grow past
/// `MAX_LINE_BYTES`. `buffer` is cleared first: one call, one line, no
/// residue from the previous one. `fill_buf`/`consume` rather than
/// `lines()`: the std and tokio line readers have no bound, and the bound
/// is the point.
async fn read_bounded_line<R>(reader: &mut R, buffer: &mut Vec<u8>) -> LineRead
where
    R: AsyncBufRead + Unpin,
{
    buffer.clear();
    loop {
        let available = match reader.fill_buf().await {
            Ok(available) => available,
            Err(_) => return LineRead::Error,
        };
        if available.is_empty() {
            return LineRead::Eof;
        }
        match available.iter().position(|byte| *byte == b'\n') {
            Some(at) => {
                buffer.extend_from_slice(&available[..=at]);
                let consumed = at + 1;
                reader.consume(consumed);
                return LineRead::Frame;
            }
            None => {
                buffer.extend_from_slice(available);
                let length = available.len();
                reader.consume(length);
                if buffer.len() > MAX_LINE_BYTES {
                    return LineRead::TooLong;
                }
            }
        }
    }
}

/// The reader task: drain the child's stdout line by line, routing each
/// reply to its waiter and ignoring everything unsolicited. When the
/// output ends — the child exited, closed the pipe, spoke a line that is
/// not valid UTF-8, or blew the line cap — every waiter is failed with
/// the matching typed error rather than left hanging.
async fn read_frames(stdout: ChildStdout, pending: Pending) {
    let mut reader = BufReader::new(stdout);
    let mut line: Vec<u8> = Vec::new();
    let ended = loop {
        match read_bounded_line(&mut reader, &mut line).await {
            LineRead::Frame => match String::from_utf8(std::mem::take(&mut line)) {
                Ok(text) => route_frame(&text, &pending),
                Err(_invalid) => break OutputEnd::NotUtf8,
            },
            LineRead::Eof => break OutputEnd::Closed,
            LineRead::Error => break OutputEnd::Closed,
            LineRead::TooLong => break OutputEnd::TooLong,
        }
    };
    for (_, waiter) in lock(&pending).drain() {
        let _ = waiter.send(Err(ended.error()));
    }
}

/// Route one stdout line: a reply for a pending id wakes its waiter; a
/// notification, an id-`null` stray (KI-002), an unknown id, or a line
/// that is not JSON at all is ignored. Pure — the seam the unit tests
/// drive.
fn route_frame(
    line: &str,
    pending: &Mutex<HashMap<u64, oneshot::Sender<Result<Value, McpError>>>>,
) {
    let Ok(frame) = serde_json::from_str::<Value>(line) else {
        return;
    };
    // A frame with no numeric id is not a reply to anything this client
    // sent: a server-initiated notification, or sys1/gen1's id-null
    // answer to `notifications/initialized`.
    let Some(id) = frame.get("id").and_then(Value::as_u64) else {
        return;
    };
    let Some(waiter) = lock(pending).remove(&id) else {
        return;
    };
    let outcome = match frame.get("error") {
        Some(error) => Err(McpError::Rpc {
            code: error
                .get("code")
                .and_then(Value::as_i64)
                .unwrap_or_default(),
            message: error
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("(no message)")
                .to_string(),
        }),
        None => Ok(frame.get("result").cloned().unwrap_or(Value::Null)),
    };
    let _ = waiter.send(outcome);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pending() -> Mutex<HashMap<u64, oneshot::Sender<Result<Value, McpError>>>> {
        Mutex::new(HashMap::new())
    }

    /// KI-040: the error is cloneable, so the reader builds one value
    /// per pending waiter when it drains them rather than one error per
    /// waiter — the row's own remedy.
    #[test]
    fn the_error_is_cloneable_for_the_drain() {
        let error = McpError::Io(std::sync::Arc::new(std::io::Error::other("broken pipe")));
        let cloned = error.clone();
        assert_eq!(error.to_string(), cloned.to_string());
        let timeout = McpError::Timeout {
            method: "tools/call".to_string(),
            budget: Duration::from_secs(60),
        };
        assert_eq!(timeout.clone().to_string(), timeout.to_string());
    }

    #[test]
    fn a_request_frame_is_one_newline_terminated_json_line_with_its_id() {
        // The framing is the contract: newline-delimited JSON-RPC 2.0, no
        // Content-Length headers anywhere. (serde_json orders object keys
        // alphabetically, so the assertion is on the parsed frame.)
        let frame = json!({ "jsonrpc": "2.0", "id": 7, "method": "tools/list", "params": {} });
        let mut line = serde_json::to_string(&frame).unwrap();
        assert_eq!(
            line.matches('\n').count(),
            0,
            "the frame is one line: {line}"
        );
        assert!(!line.contains("Content-Length"));
        assert_eq!(
            serde_json::from_str::<Value>(&line).unwrap(),
            json!({ "jsonrpc": "2.0", "id": 7, "method": "tools/list", "params": {} })
        );
        // The write path appends exactly one newline.
        line.push('\n');
        assert!(line.ends_with("}\n"), "{line}");
    }

    #[test]
    fn a_notification_frame_carries_no_id() {
        let frame = json!({ "jsonrpc": "2.0", "method": "notifications/initialized" });
        assert!(frame.get("id").is_none());
    }

    #[test]
    fn a_reply_wakes_the_waiter_its_id_names() {
        let pending = pending();
        let (tx, mut rx) = oneshot::channel();
        lock(&pending).insert(1, tx);
        route_frame(
            r#"{"jsonrpc":"2.0","id":1,"result":{"tools":[]}}"#,
            &pending,
        );
        let outcome = rx
            .try_recv()
            .expect("the waiter woke")
            .expect("an ok outcome");
        assert_eq!(outcome, json!({ "tools": [] }));
        // The waiter is consumed, not left pending.
        assert!(lock(&pending).is_empty());
    }

    #[test]
    fn notifications_and_id_less_strays_are_ignored() {
        // sys1 and gen1 answer `notifications/initialized` with an id-null
        // -32601 frame (KI-002); a server may also emit notifications of
        // its own. None of them may disturb a pending waiter.
        let pending = pending();
        let (tx, mut rx) = oneshot::channel();
        lock(&pending).insert(4, tx);
        route_frame(
            r#"{"jsonrpc":"2.0","method":"notifications/cancelled"}"#,
            &pending,
        );
        route_frame(
            r#"{"jsonrpc":"2.0","id":null,"result":{"error":{"code":-32601,"message":"Method not found: notifications/initialized"}}}"#,
            &pending,
        );
        route_frame(
            r#"{"jsonrpc":"2.0","id":99,"result":{"tools":[]}}"#,
            &pending,
        );
        route_frame("not json at all", &pending);
        assert!(rx.try_recv().is_err(), "the waiter must not have woken");
        assert_eq!(lock(&pending).len(), 1, "the pending waiter stays pending");
        // The real reply still lands afterwards.
        route_frame(r#"{"jsonrpc":"2.0","id":4,"result":{"ok":true}}"#, &pending);
        let outcome = rx
            .try_recv()
            .expect("the waiter woke")
            .expect("an ok outcome");
        assert_eq!(outcome, json!({ "ok": true }));
    }

    #[test]
    fn an_error_frame_wakes_the_waiter_with_a_typed_rpc_error() {
        let pending = pending();
        let (tx, mut rx) = oneshot::channel();
        lock(&pending).insert(2, tx);
        route_frame(
            r#"{"jsonrpc":"2.0","id":2,"error":{"code":-32602,"message":"`items` must be a string"}}"#,
            &pending,
        );
        let error = rx
            .try_recv()
            .expect("the waiter woke")
            .expect_err("an error outcome");
        match error {
            McpError::Rpc { code, message } => {
                assert_eq!(code, -32602);
                assert!(message.contains("`items`"));
            }
            other => panic!("expected an Rpc error, got {other:?}"),
        }
    }

    #[test]
    fn a_result_less_reply_is_a_null_result_not_a_hang() {
        let pending = pending();
        let (tx, mut rx) = oneshot::channel();
        lock(&pending).insert(3, tx);
        route_frame(r#"{"jsonrpc":"2.0","id":3}"#, &pending);
        let outcome = rx
            .try_recv()
            .expect("the waiter woke")
            .expect("an ok outcome");
        assert_eq!(outcome, Value::Null);
    }

    #[test]
    fn initialize_results_parse_with_the_server_named() {
        let result = parse_initialize(json!({
            "protocolVersion": "2024-11-05",
            "capabilities": { "tools": { "listChanged": false } },
            "serverInfo": { "name": "sem1", "version": "0.1.0" },
        }))
        .unwrap();
        assert_eq!(result.protocol_version, PROTOCOL_VERSION);
        assert_eq!(result.server_name, "sem1");
        assert_eq!(result.server_version, "0.1.0");
        // A server with no serverInfo still initializes.
        let bare = parse_initialize(json!({ "protocolVersion": "2024-11-05" })).unwrap();
        assert_eq!(bare.server_name, "unknown");
        // No protocol version is a protocol error, not a panic.
        let error = parse_initialize(json!({})).unwrap_err();
        assert!(matches!(error, McpError::Protocol(_)), "got {error:?}");
    }

    #[test]
    fn tool_rosters_parse_with_schemas_verbatim() {
        let schema = json!({ "type": "object", "properties": { "items": { "type": "string" } } });
        let tools = parse_tools(json!({
            "tools": [{ "name": "sem1_embed", "description": "embed", "inputSchema": schema }]
        }))
        .unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "sem1_embed");
        assert_eq!(tools[0].input_schema, schema);
        // A missing tools array is a protocol error.
        let error = parse_tools(json!({})).unwrap_err();
        assert!(matches!(error, McpError::Protocol(_)), "got {error:?}");
    }

    #[test]
    fn call_results_parse_content_and_the_is_error_flag() {
        let result = parse_call(json!({
            "content": [{ "type": "text", "text": "{\"dims\": 768}" }],
            "isError": false,
        }))
        .unwrap();
        assert_eq!(result.content.len(), 1);
        assert_eq!(result.content[0].content_type, "text");
        assert!(!result.is_error);
        // An empty result parses as no content, not an error.
        let empty = parse_call(json!({})).unwrap();
        assert!(empty.content.is_empty());
        // A JSON-RPC error nested under `result` is the lane's own
        // refusal, carried in the lane's words with is_error set —
        // never a content-less success that drops what the lane said
        // (KI-191: the beat refused nine rows naming a parse failure
        // over the lane's own "can only join an iterable").
        let nested = parse_call(json!({
            "error": { "code": -32603, "message": "can only join an iterable" },
        }))
        .unwrap();
        assert!(nested.is_error);
        assert_eq!(nested.text(), "can only join an iterable");
        // An error with no message still refuses, with a named stand-in.
        let mute = parse_call(json!({ "error": { "code": -32603 } })).unwrap();
        assert!(mute.is_error);
        assert!(mute.text().contains("no message"), "got {:?}", mute.text());
    }

    /// A parsed call result over `blocks`, for the mapping tests.
    fn call_result(blocks: serde_json::Value, is_error: bool) -> CallResult {
        parse_call(json!({ "content": blocks, "isError": is_error })).unwrap()
    }

    #[test]
    fn a_call_results_text_joins_the_text_blocks_and_notes_the_rest() {
        // The agent's tool results carry one text blob: text blocks join,
        // and a block the model cannot read is named by its type rather
        // than dropped, so the model knows it is there.
        let result = call_result(
            json!([
                { "type": "text", "text": "{\"dims\": 768}" },
                { "type": "text", "text": "model: bge-small" },
                { "type": "image", "text": "ignored-for-a-non-text-block" },
            ]),
            false,
        );
        assert_eq!(result.text(), "{\"dims\": 768}\nmodel: bge-small\n[image]");

        // A typed result with no text blocks is still an answer: the
        // structured content is the fallback reading.
        let structured = parse_call(json!({
            "content": [],
            "structuredContent": { "dims": 768, "model": "bge-small" }
        }))
        .unwrap();
        assert_eq!(structured.text(), "{\"dims\":768,\"model\":\"bge-small\"}");
        // When both arrive, the text is what the model already reads.
        let both = parse_call(json!({
            "content": [{ "type": "text", "text": "ok" }],
            "structuredContent": { "dims": 768 }
        }))
        .unwrap();
        assert_eq!(both.text(), "ok");
    }

    #[test]
    fn an_image_block_is_written_to_a_file_and_named() {
        let dir = std::env::temp_dir().join(format!("fleet-mcp-image-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let result = parse_call(json!({
            "content": [{
                "type": "image",
                "mimeType": "image/png",
                "data": "iVBORw0KGgo="
            }]
        }))
        .unwrap();
        let text = result.text_with_artifacts(&dir);
        assert!(text.contains("[image saved:"), "{text}");
        let path = text
            .split("[image saved: ")
            .nth(1)
            .and_then(|rest| rest.split(']').next())
            .expect("the path is named");
        let bytes = std::fs::read(path).unwrap();
        assert_eq!(bytes, vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_content_less_result_says_so_rather_than_reading_as_an_empty_success() {
        let empty = call_result(json!([]), false);
        assert_eq!(empty.text(), "(the tool returned no content)");
        // An isError result keeps its text: a refusal is the model's to
        // read, and the agent records it as the failure it is.
        let refused = call_result(
            json!([{ "type": "text", "text": "`items` must be a string" }]),
            true,
        );
        assert!(refused.is_error);
        assert_eq!(refused.text(), "`items` must be a string");
    }

    #[tokio::test]
    async fn a_missing_program_is_a_spawn_error_not_a_panic() {
        let spec = ServerSpec {
            program: "fleet-no-such-mcp-program".to_string(),
            args: Vec::new(),
            env: Vec::new(),
            passthrough: Vec::new(),
            cwd: None,
        };
        let Err(error) = McpServer::spawn(&spec) else {
            panic!("the spawn must fail for a missing program")
        };
        assert!(matches!(error, McpError::Spawn(_)), "got {error:?}");
        assert!(error.to_string().contains("could not spawn"), "{error}");
    }

    /// A parent environment for `child_env`: the caller's, plus the names
    /// a test wants present. Nothing here mutates the test process.
    fn parent_env(entries: &[(&str, &str)]) -> HashMap<OsString, OsString> {
        let mut parent: HashMap<OsString, OsString> = std::env::vars_os().collect();
        for (name, value) in entries {
            parent.insert(OsString::from(name), OsString::from(value));
        }
        parent
    }

    fn names(env: &[(OsString, OsString)]) -> Vec<String> {
        env.iter()
            .map(|(name, _)| name.to_string_lossy().into_owned())
            .collect()
    }

    fn lookup<'a>(env: &'a [(OsString, OsString)], name: &str) -> Option<&'a OsStr> {
        env.iter()
            .find(|(key, _)| key == OsStr::new(name))
            .map(|(_, value)| value.as_os_str())
    }

    #[test]
    fn a_child_gets_the_base_and_the_lanes_allowlist_and_nothing_else() {
        // KI-031: the lane child must not inherit the ambient environment.
        // A sem1-shaped spec whose caller holds the DeepSeek key (which no
        // lane has any use for), a sem1 allowlist entry, and the base.
        let spec = ServerSpec {
            program: "python3".to_string(),
            args: vec!["-u".to_string(), "/lane/service/mcp_server.py".to_string()],
            env: vec![("SEM1_PYTHONPATH".to_string(), "/lane/src".to_string())],
            passthrough: vec!["SEM1_ST_VENV".to_string()],
            cwd: None,
        };
        let parent = parent_env(&[
            ("DEEPSEEK_API_KEY", "sk-do-not-cross-into-a-lane"),
            ("TABPFN_API_KEY", "tabpfn-do-not-cross-either"),
            ("SEM1_ST_VENV", "/opt/venvs/st"),
            ("PATH", "/opt/homebrew/bin:/usr/bin"),
        ]);

        let env = child_env(&spec, &parent);

        // The credential-shaped ambient names are gone.
        assert!(
            !names(&env).contains(&"DEEPSEEK_API_KEY".to_string()),
            "{:?}",
            names(&env)
        );
        assert!(
            !names(&env).contains(&"TABPFN_API_KEY".to_string()),
            "{:?}",
            names(&env)
        );
        // The base is there, PATH first, and the caller's value survived.
        assert_eq!(env[0].0.as_os_str(), OsStr::new("PATH"));
        assert_eq!(
            lookup(&env, "PATH"),
            Some(OsStr::new("/opt/homebrew/bin:/usr/bin"))
        );
        assert!(
            names(&env).contains(&"HOME".to_string()),
            "{:?}",
            names(&env)
        );
        // The lane's allowlist entry crossed; its own declaration did too.
        assert_eq!(
            lookup(&env, "SEM1_ST_VENV"),
            Some(OsStr::new("/opt/venvs/st")),
            "{:?}",
            names(&env)
        );
        assert_eq!(
            lookup(&env, "SEM1_PYTHONPATH"),
            Some(OsStr::new("/lane/src"))
        );
    }

    #[test]
    fn an_absent_allowlist_entry_is_absent_not_empty() {
        // A name the caller does not have must not appear at all: an empty
        // value in a child's environment is a different thing from an
        // unset one, and several lanes branch on presence.
        let spec = ServerSpec {
            program: "python3".to_string(),
            args: Vec::new(),
            env: Vec::new(),
            passthrough: vec!["SEM1_ST_VENV".to_string()],
            cwd: None,
        };
        let parent = parent_env(&[]);
        let env = child_env(&spec, &parent);
        assert!(
            !names(&env).contains(&"SEM1_ST_VENV".to_string()),
            "{:?}",
            names(&env)
        );
        // Every name that did cross is one the caller actually has, so the
        // child's environment is a subset of the base plus the allowlist.
        for (name, _) in &env {
            assert!(
                BASE_ENV.contains(&name.to_string_lossy().as_ref()),
                "{name:?} is not a base name"
            );
        }
    }

    #[tokio::test]
    async fn a_line_at_the_cap_is_a_frame_and_one_past_it_is_too_long() {
        // The bound is the point: `lines()` has none, so a lane streaming
        // an unterminated line could grow the reader without limit.
        let mut buffer: Vec<u8> = Vec::new();
        let short = b"{\"id\":1}\nrest".to_vec();
        let at_cap = {
            let mut line = b"x".repeat(MAX_LINE_BYTES);
            line.push(b'\n');
            line
        };
        let past_cap = b"y".repeat(MAX_LINE_BYTES + 4096);
        let empty: Vec<u8> = Vec::new();

        // A short line arrives whole, newline included.
        let mut reader = BufReader::new(&short[..]);
        assert!(matches!(
            read_bounded_line(&mut reader, &mut buffer).await,
            LineRead::Frame
        ));
        assert_eq!(&buffer, b"{\"id\":1}\n");

        // A line of exactly the cap, newline included, is still a frame.
        let mut reader = BufReader::new(&at_cap[..]);
        assert!(matches!(
            read_bounded_line(&mut reader, &mut buffer).await,
            LineRead::Frame
        ));
        assert_eq!(buffer.len(), MAX_LINE_BYTES + 1);

        // One byte past it is a typed refusal, and the buffer stopped
        // growing at the cap rather than at the line's real length.
        let mut reader = BufReader::new(&past_cap[..]);
        assert!(matches!(
            read_bounded_line(&mut reader, &mut buffer).await,
            LineRead::TooLong
        ));
        assert!(buffer.len() > MAX_LINE_BYTES, "{}", buffer.len());
        assert!(
            buffer.len() <= MAX_LINE_BYTES + 8 * 1024,
            "the buffer must stop at the cap, not at the line: {}",
            buffer.len()
        );

        // EOF with nothing buffered is an Eof, not a frame.
        let mut reader = BufReader::new(&empty[..]);
        assert!(matches!(
            read_bounded_line(&mut reader, &mut buffer).await,
            LineRead::Eof
        ));
    }

    #[test]
    fn each_output_end_names_its_own_reason() {
        // A non-UTF-8 line is not "the child closed its output", and a
        // line past the cap is neither: the waiter is told which happened.
        assert!(matches!(OutputEnd::Closed.error(), McpError::Closed(_)));
        match OutputEnd::NotUtf8.error() {
            McpError::Protocol(detail) => {
                assert!(detail.contains("not valid UTF-8"), "{detail}")
            }
            other => panic!("expected a protocol error, got {other:?}"),
        }
        match OutputEnd::TooLong.error() {
            McpError::Protocol(detail) => {
                assert!(detail.contains("longer than the 8 MiB cap"), "{detail}")
            }
            other => panic!("expected a protocol error, got {other:?}"),
        }
    }

    #[test]
    fn the_specs_own_entries_win_over_the_passthrough() {
        let spec = ServerSpec {
            program: "python3".to_string(),
            args: Vec::new(),
            env: vec![("SEM1_ST_VENV".to_string(), "/lane/declared".to_string())],
            passthrough: vec!["SEM1_ST_VENV".to_string()],
            cwd: None,
        };
        let parent = parent_env(&[("SEM1_ST_VENV", "/ambient")]);
        let env = child_env(&spec, &parent);
        assert_eq!(
            lookup(&env, "SEM1_ST_VENV"),
            Some(OsStr::new("/lane/declared"))
        );
    }

    /// KI-039: a declared name ending in `_` is a namespace — every
    /// ambient name strictly under it crosses, once. An exact entry
    /// earlier in the list is not duplicated, and a name the spec
    /// declares itself never crosses.
    #[test]
    fn a_namespace_declaration_carries_every_name_under_it_once() {
        let spec = ServerSpec {
            program: "python3".to_string(),
            args: Vec::new(),
            env: vec![(
                "SYS1_CLASSIFY__LOG_DIR".to_string(),
                "/lane/own".to_string(),
            )],
            passthrough: vec!["SYS1_ENV_FILE".to_string(), "SYS1_".to_string()],
            cwd: None,
        };
        let parent = parent_env(&[
            ("SYS1_ENV_FILE", "/ambient/env"),
            ("SYS1_PROVIDERS__JEV_MODEL", "jevgemma"),
            ("SYS1_CLASSIFY__LOG_DIR", "/ambient/logs"),
            ("SYS2_OTHER", "not-this-namespace"),
            ("DEEPSEEK_API_KEY", "sk-never"),
        ]);
        let env = child_env(&spec, &parent);
        assert_eq!(
            lookup(&env, "SYS1_ENV_FILE"),
            Some(OsStr::new("/ambient/env"))
        );
        assert_eq!(
            lookup(&env, "SYS1_PROVIDERS__JEV_MODEL"),
            Some(OsStr::new("jevgemma"))
        );
        // The spec's own declaration wins, and the namespace does not
        // duplicate the exact entry.
        assert_eq!(
            lookup(&env, "SYS1_CLASSIFY__LOG_DIR"),
            Some(OsStr::new("/lane/own"))
        );
        let carried: Vec<&OsStr> = env.iter().map(|(name, _)| name.as_os_str()).collect();
        assert_eq!(
            carried
                .iter()
                .filter(|name| **name == OsStr::new("SYS1_ENV_FILE"))
                .count(),
            1,
            "the exact entry is not duplicated by the namespace: {carried:?}"
        );
        assert_eq!(lookup(&env, "SYS2_OTHER"), None);
        assert_eq!(lookup(&env, "DEEPSEEK_API_KEY"), None);
    }

    #[test]
    fn the_stderr_tail_keeps_the_last_bytes_not_the_first() {
        // A lane that fails mid-startup prints its reason last, after
        // whatever it printed on the way in, so the tail is the half
        // worth keeping — and the half that stays bounded.
        let mut tail = StderrTail::default();
        let head = vec![b'h'; STDERR_TAIL_BYTES / 2];
        let middle = vec![b'm'; STDERR_TAIL_BYTES];
        tail.push(&head);
        tail.push(&middle);
        let reason = b"ModuleNotFoundError: no module named 'glinter'\n";
        tail.push(reason);

        let text = tail.text();
        assert_eq!(
            text.len(),
            STDERR_TAIL_BYTES,
            "the tail is a bound, not a budget"
        );
        // The reason survived whole, at the end.
        assert!(
            text.ends_with("ModuleNotFoundError: no module named 'glinter'\n"),
            "{text}"
        );
        // The head is gone: the oldest bytes are what a cap drops.
        assert!(
            !text.starts_with('h'),
            "the head must have been dropped: {text}"
        );
    }

    #[test]
    fn a_small_stderr_tail_is_kept_whole() {
        let mut tail = StderrTail::default();
        tail.push(b"warning: falling back to cpu\n");
        assert_eq!(tail.text(), "warning: falling back to cpu\n");
        // Nothing written is an empty tail, which is what leaves an error
        // message unannotated.
        assert!(StderrTail::default().text().is_empty());
    }

    #[test]
    fn an_empty_stderr_tail_leaves_the_failure_exactly_as_it_was() {
        // A silent child's messages must not change: the annotation is
        // additive, and only when there is something to add.
        let error = McpError::Closed("the child closed its output before replying".to_string());
        let unchanged = annotated(error, "");
        match unchanged {
            McpError::Closed(reason) => {
                assert_eq!(reason, "the child closed its output before replying")
            }
            other => panic!("expected a Closed error, got {other:?}"),
        }
    }

    #[test]
    fn a_stderr_tail_rides_along_on_a_handshake_failure() {
        // The point of draining at all: the child's reason reaches the
        // caller instead of a bare "closed".
        let error = McpError::Closed("the child closed its output before replying".to_string());
        match annotated(error, "ModuleNotFoundError: no module named 'glinter'") {
            McpError::Closed(reason) => {
                assert!(
                    reason.contains("the child closed its output before replying"),
                    "{reason}"
                );
                assert!(
                    reason.ends_with(
                        "; its stderr said: ModuleNotFoundError: no module named 'glinter'"
                    ),
                    "{reason}"
                );
            }
            other => panic!("expected a Closed error, got {other:?}"),
        }
        // The other message-carrying shapes take the tail too, keeping
        // the fields a caller matches on.
        match annotated(
            McpError::Protocol("malformed MCP reply: bad json".to_string()),
            "traceback",
        ) {
            McpError::Protocol(detail) => {
                assert!(
                    detail.starts_with("malformed MCP reply: bad json"),
                    "{detail}"
                );
                assert!(detail.ends_with("; its stderr said: traceback"), "{detail}");
            }
            other => panic!("expected a Protocol error, got {other:?}"),
        }
        match annotated(
            McpError::Rpc {
                code: -32602,
                message: "`items` must be a string".to_string(),
            },
            "traceback",
        ) {
            McpError::Rpc { code, message } => {
                assert_eq!(code, -32602);
                assert!(message.starts_with("`items` must be a string"), "{message}");
            }
            other => panic!("expected an Rpc error, got {other:?}"),
        }
        // A timeout keeps its own shape: the budget and the step that
        // blew it are the fact, and the variant has no message field.
        let timeout = McpError::Timeout {
            method: "initialize".to_string(),
            budget: std::time::Duration::from_secs(10),
        };
        assert!(matches!(
            annotated(timeout, "traceback"),
            McpError::Timeout { .. }
        ));
        // An io::Error keeps its kind, so a caller matching on it still
        // matches after the annotation.
        let broken = std::io::Error::new(std::io::ErrorKind::BrokenPipe, "the pipe broke");
        let annotated_io_error = annotated(McpError::Io(std::sync::Arc::new(broken)), "traceback");
        match annotated_io_error {
            McpError::Io(error) => {
                assert_eq!(error.kind(), std::io::ErrorKind::BrokenPipe);
                assert!(error.to_string().contains("the pipe broke"), "{error}");
                assert!(
                    error.to_string().ends_with("; its stderr said: traceback"),
                    "{error}"
                );
            }
            other => panic!("expected an Io error, got {other:?}"),
        }
    }
}
