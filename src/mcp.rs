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
//! Every wait carries a budget, so a silent server is a timeout and a dead
//! one an error — never a hang. Teardown is the graceful path first (close
//! stdin, wait, kill on expiry); `Drop` is the best-effort net under it.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::oneshot;
use tokio::time::timeout;
use tokio::task::JoinHandle;

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
        Timeouts { initialize: INITIALIZE_TIMEOUT, list: LIST_TIMEOUT, call: CALL_TIMEOUT }
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
}

/// The client's failures. Typed values, never panics: a crashed child is
/// an error here, not a hang.
#[derive(Debug)]
pub enum McpError {
    /// The child could not be spawned (no `python3` on PATH, bad path).
    Spawn(std::io::Error),
    /// The child's pipes failed (broken pipe on write, read error).
    Io(std::io::Error),
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
            McpError::Rpc { code, message } => write!(f, "the MCP server errored ({code}): {message}"),
            McpError::Protocol(detail) => write!(f, "malformed MCP reply: {detail}"),
        }
    }
}

impl std::error::Error for McpError {}

/// Lock a mutex, recovering from poisoning: a panic while the lock was
/// held must not turn every later call into another panic.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// One MCP server child, with its reader task and pending waiters.
pub struct McpServer {
    child: Mutex<Option<Child>>,
    stdin: Mutex<Option<ChildStdin>>,
    /// Waiters keyed by request id. The reader task owns the only other
    /// handle; an entry lives from the write until its reply, its timeout,
    /// or the child's death.
    pending: Arc<Mutex<HashMap<u64, oneshot::Sender<Result<Value, McpError>>>>>,
    next_id: AtomicU64,
    timeouts: Timeouts,
    reader: JoinHandle<()>,
}

impl McpServer {
    /// Spawn `spec` as a child and start its reader task. The child is not
    /// yet initialized — call [`McpServer::initialize`] next.
    pub fn spawn(spec: &ServerSpec) -> Result<McpServer, McpError> {
        Self::spawn_with(spec, Timeouts::default())
    }

    /// `spawn` with explicit budgets (tests shorten them).
    pub fn spawn_with(spec: &ServerSpec, timeouts: Timeouts) -> Result<McpServer, McpError> {
        let mut command = Command::new(&spec.program);
        command.args(&spec.args);
        for (name, value) in &spec.env {
            command.env(name, value);
        }
        if let Some(cwd) = &spec.cwd {
            command.current_dir(cwd);
        }
        // stdin/stdout are the protocol; stderr is captured for the error
        // message when a child dies during startup.
        command.stdin(std::process::Stdio::piped());
        command.stdout(std::process::Stdio::piped());
        command.stderr(std::process::Stdio::piped());
        let mut child = command.spawn().map_err(McpError::Spawn)?;
        let stdin = child.stdin.take().ok_or_else(|| {
            McpError::Protocol("the child's stdin was not piped".to_string())
        })?;
        let stdout = child.stdout.take().ok_or_else(|| {
            McpError::Protocol("the child's stdout was not piped".to_string())
        })?;
        let pending: Arc<Mutex<HashMap<u64, oneshot::Sender<Result<Value, McpError>>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let reader = tokio::spawn(read_frames(stdout, Arc::clone(&pending)));
        Ok(McpServer {
            child: Mutex::new(Some(child)),
            stdin: Mutex::new(Some(stdin)),
            pending,
            next_id: AtomicU64::new(1),
            timeouts,
            reader,
        })
    }

    /// Perform the MCP handshake: `initialize`, then the
    /// `notifications/initialized` notification (never answered; sys1 and
    /// gen1 answer it with a `-32601` stray frame the reader ignores).
    pub async fn initialize(&self) -> Result<InitializeResult, McpError> {
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
    }

    /// The server's tool roster.
    pub async fn list_tools(&self) -> Result<Vec<ToolInfo>, McpError> {
        let result = self.request("tools/list", json!({}), self.timeouts.list).await?;
        parse_tools(result)
    }

    /// Call one tool. A JSON-RPC error (bad arguments, unknown tool) comes
    /// back as [`McpError::Rpc`]; a tool-level refusal comes back as
    /// [`CallResult::is_error`].
    pub async fn call_tool(
        &self,
        name: &str,
        arguments: Value,
    ) -> Result<CallResult, McpError> {
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
        drop(lock(&self.stdin).take());
        let child = lock(&self.child).take();
        self.reader.abort();
        let Some(mut child) = child else { return Ok(()) };
        match timeout(TEARDOWN_GRACE, child.wait()).await {
            Ok(Ok(_)) => Ok(()),
            Ok(Err(error)) => Err(McpError::Io(error)),
            Err(_elapsed) => {
                // The grace expired: kill, and report a failure only when
                // even the kill goes wrong.
                child.kill().await.map_err(McpError::Io)?;
                child.wait().await.map_err(McpError::Io)?;
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
                // A late reply now finds no waiter and is ignored.
                lock(&self.pending).remove(&id);
                Err(McpError::Timeout { method: method.to_string(), budget })
            }
        }
    }

    /// Write one notification: a frame with no id, never answered.
    async fn notify(&self, method: &str) -> Result<(), McpError> {
        self.write_frame(&json!({ "jsonrpc": "2.0", "method": method })).await
    }

    /// Write one frame as a single newline-terminated line and flush.
    async fn write_frame(&self, frame: &Value) -> Result<(), McpError> {
        let mut line = serde_json::to_string(frame).expect("a frame is serializable JSON");
        line.push('\n');
        let mut guard = lock(&self.stdin);
        let Some(stdin) = guard.as_mut() else {
            return Err(McpError::Closed("the server's stdin is closed".to_string()));
        };
        stdin.write_all(line.as_bytes()).await.map_err(McpError::Io)?;
        stdin.flush().await.map_err(McpError::Io)
    }
}

impl Drop for McpServer {
    fn drop(&mut self) {
        // The best-effort net when `close` was not awaited: signal EOF,
        // then kill a child still running. A synchronous wait would block
        // the caller's thread (and cannot run inside a runtime), so the
        // graceful path is `close`'s job.
        drop(lock(&self.stdin).take());
        self.reader.abort();
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
    Ok(InitializeResult { protocol_version, server_name, server_version })
}

/// Parse a `tools/list` result into the roster.
fn parse_tools(result: Value) -> Result<Vec<ToolInfo>, McpError> {
    let tools = result
        .get("tools")
        .and_then(Value::as_array)
        .ok_or_else(|| McpError::Protocol("tools/list answered without a tools array".to_string()))?;
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
    let content = result
        .get("content")
        .and_then(Value::as_array)
        .map(|blocks| {
            blocks
                .iter()
                .map(|block| ContentBlock {
                    content_type: block
                        .get("type")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    data: block.get("text").cloned().unwrap_or(Value::Null),
                })
                .collect()
        })
        .unwrap_or_default();
    Ok(CallResult {
        content,
        is_error: result.get("isError").and_then(Value::as_bool).unwrap_or(false),
    })
}

/// The reader task: drain the child's stdout line by line, routing each
/// reply to its waiter and ignoring everything unsolicited. When the
/// output ends — the child exited or closed the pipe — every waiter is
/// failed with [`McpError::Closed`] rather than left hanging.
async fn read_frames(
    stdout: ChildStdout,
    pending: Arc<Mutex<HashMap<u64, oneshot::Sender<Result<Value, McpError>>>>>,
) {
    let mut lines = BufReader::new(stdout).lines();
    loop {
        match lines.next_line().await {
            Ok(Some(line)) => route_frame(&line, &pending),
            // EOF or a read error: either way no further replies exist.
            Ok(None) | Err(_) => break,
        }
    }
    for (_, waiter) in lock(&pending).drain() {
        let _ = waiter.send(Err(McpError::Closed(
            "the child closed its output before replying".to_string(),
        )));
    }
}

/// Route one stdout line: a reply for a pending id wakes its waiter; a
/// notification, an id-`null` stray (KI-002), an unknown id, or a line
/// that is not JSON at all is ignored. Pure — the seam the unit tests
/// drive.
fn route_frame(line: &str, pending: &Mutex<HashMap<u64, oneshot::Sender<Result<Value, McpError>>>>) {
    let Ok(frame) = serde_json::from_str::<Value>(line) else { return };
    // A frame with no numeric id is not a reply to anything this client
    // sent: a server-initiated notification, or sys1/gen1's id-null
    // answer to `notifications/initialized`.
    let Some(id) = frame.get("id").and_then(Value::as_u64) else { return };
    let Some(waiter) = lock(pending).remove(&id) else { return };
    let outcome = match frame.get("error") {
        Some(error) => Err(McpError::Rpc {
            code: error.get("code").and_then(Value::as_i64).unwrap_or_default(),
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

    #[test]
    fn a_request_frame_is_one_newline_terminated_json_line_with_its_id() {
        // The framing is the contract: newline-delimited JSON-RPC 2.0, no
        // Content-Length headers anywhere. (serde_json orders object keys
        // alphabetically, so the assertion is on the parsed frame.)
        let frame = json!({ "jsonrpc": "2.0", "id": 7, "method": "tools/list", "params": {} });
        let mut line = serde_json::to_string(&frame).unwrap();
        assert_eq!(line.matches('\n').count(), 0, "the frame is one line: {line}");
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
        route_frame(r#"{"jsonrpc":"2.0","id":1,"result":{"tools":[]}}"#, &pending);
        let outcome = rx.try_recv().expect("the waiter woke").expect("an ok outcome");
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
        route_frame(r#"{"jsonrpc":"2.0","method":"notifications/cancelled"}"#, &pending);
        route_frame(
            r#"{"jsonrpc":"2.0","id":null,"result":{"error":{"code":-32601,"message":"Method not found: notifications/initialized"}}}"#,
            &pending,
        );
        route_frame(r#"{"jsonrpc":"2.0","id":99,"result":{"tools":[]}}"#, &pending);
        route_frame("not json at all", &pending);
        assert!(rx.try_recv().is_err(), "the waiter must not have woken");
        assert_eq!(lock(&pending).len(), 1, "the pending waiter stays pending");
        // The real reply still lands afterwards.
        route_frame(r#"{"jsonrpc":"2.0","id":4,"result":{"ok":true}}"#, &pending);
        let outcome = rx.try_recv().expect("the waiter woke").expect("an ok outcome");
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
        let error = rx.try_recv().expect("the waiter woke").expect_err("an error outcome");
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
        let outcome = rx.try_recv().expect("the waiter woke").expect("an ok outcome");
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
    }

    #[tokio::test]
    async fn a_missing_program_is_a_spawn_error_not_a_panic() {
        let spec = ServerSpec {
            program: "fleet-no-such-mcp-program".to_string(),
            args: Vec::new(),
            env: Vec::new(),
            cwd: None,
        };
        let Err(error) = McpServer::spawn(&spec) else {
            panic!("the spawn must fail for a missing program")
        };
        assert!(matches!(error, McpError::Spawn(_)), "got {error:?}");
        assert!(error.to_string().contains("could not spawn"), "{error}");
    }
}
