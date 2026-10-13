//! The agent loop: task in, model turns and approved tool calls out, until
//! the model answers without tools or the turn cap is reached.
//!
//! Every message is recorded to the session log as it happens, so the log
//! always contains exactly what the model saw.

use crate::api::{ChatRequest, Client, ClientError, Message, Role, StreamEvent, ToolCall, Usage};
use crate::lanes;
use crate::mcp::{CallResult, McpServer, ServerSpec, ToolInfo};
use crate::session::{Record, Session};
use crate::tools::{self, LaneTool, LaneTools, Tool, ToolOutcome};
use std::collections::HashMap;
use std::io::Write;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

/// System prompt sent with every request.
pub const SYSTEM_PROMPT: &str = "You are Fleet, a native Rust coding agent based on the DeepSeek Harness design. Work in the user's current directory. Use the provided tools to inspect and change files, run commands, and verify your work. Read before you write. When a task is done, reply with a short summary of the outcome; if part of it failed, say exactly what failed and what you tried.";

/// Turn cap when the caller does not choose one.
pub const DEFAULT_MAX_TURNS: usize = 25;

/// Options for one run.
#[derive(Debug, Clone)]
pub struct AgentOptions {
    pub model: String,
    pub max_turns: usize,
    /// Where live UI events go; `None` is the headless path, which keeps
    /// its progress-write behavior exactly as it was.
    pub ui_tx: Option<UnboundedSender<UiEvent>>,
    /// Where a run's lane children come from, keyed by lane name.
    ///
    /// `None` is the production path: the vendored registry
    /// (`crate::lanes::spec_for`), resolved lazily, one lane at a time,
    /// the first time one of its tools is called. A test injects a
    /// scripted fake's spec here so the loop's lane paths run without a
    /// real lane on the machine.
    pub lane_specs: Option<HashMap<String, ServerSpec>>,
}

impl Default for AgentOptions {
    fn default() -> AgentOptions {
        AgentOptions {
            model: crate::api::DEFAULT_MODEL.to_string(),
            max_turns: DEFAULT_MAX_TURNS,
            ui_tx: None,
            lane_specs: None,
        }
    }
}

/// What the loop reports to a live UI as it happens. These are transport,
/// not log content: the session log is written exactly as before, beside
/// these emissions, so model-visible stays model-visible either way.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UiEvent {
    /// One streamed content delta.
    Token(String),
    /// A turn started.
    Turn(u64),
    /// The model requested a tool call, before the approval gate.
    ToolCall { name: String, summary: String },
    /// A call finished: executed or denied.
    ToolResult { name: String, preview: String, approved: bool },
    /// A lane joined the run: its tools are in the model's tool set for
    /// the rest of the run, and its `lane_tools` record is in the log.
    LaneJoined { lane: String, tool_count: usize },
    /// The route this run takes: the active provider and how it was chosen
    /// (`judged`/`fallback`/`pinned`). Emitted once at the start of a
    /// routed run; a plain single-endpoint client emits nothing, so the
    /// unrouted and headless paths are unchanged.
    Routed { provider: String, how: String },
    /// The loop stopped orderly.
    Finished { stop_reason: String },
    /// The loop failed; the run is over.
    Failed(String),
}

/// Why the loop stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopReason {
    /// The model answered without requesting tools.
    Completed,
    /// The turn cap was reached first.
    MaxTurns,
}

/// What one run produced.
#[derive(Debug, Clone)]
pub struct RunReport {
    pub stop_reason: StopReason,
    pub turns: u64,
    pub usage: Usage,
    /// The final assistant text, when the stop produced one.
    pub final_message: Option<String>,
}

/// The loop's fatal failures. Tool failures are not fatal: they are recorded
/// and shown to the model.
#[derive(Debug)]
pub enum AgentError {
    Api(ClientError),
    Io(std::io::Error),
}

impl std::fmt::Display for AgentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentError::Api(error) => write!(f, "{error}"),
            AgentError::Io(error) => write!(f, "session log write failed: {error}"),
        }
    }
}

impl std::error::Error for AgentError {}

/// Decides whether one tool call may execute.
pub trait Approver {
    /// True when the call may run.
    fn approve(&mut self, call: &ToolCall) -> bool;
}

/// Approves everything (`--yes`).
#[derive(Debug, Default)]
pub struct ApproveAll;

impl Approver for ApproveAll {
    fn approve(&mut self, _call: &ToolCall) -> bool {
        true
    }
}

/// Denies everything; tests and non-interactive dry runs.
#[derive(Debug, Default)]
pub struct DenyAll;

impl Approver for DenyAll {
    fn approve(&mut self, _call: &ToolCall) -> bool {
        false
    }
}

/// Prompts on the console; `y`/`yes` allows one call, `a`/`always` latches
/// approval for the rest of the run, anything else denies. End of input
/// denies.
pub struct ConsoleApprover {
    input: std::io::Stdin,
    always: bool,
}

impl ConsoleApprover {
    /// A fresh prompter that never allows until asked.
    pub fn new() -> ConsoleApprover {
        ConsoleApprover { input: std::io::stdin(), always: false }
    }
}

impl Default for ConsoleApprover {
    fn default() -> ConsoleApprover {
        ConsoleApprover::new()
    }
}

impl Approver for ConsoleApprover {
    fn approve(&mut self, call: &ToolCall) -> bool {
        if self.always {
            return true;
        }
        eprint!("fleet: allow {} `{}`? [y/N/a] ", call.function.name, call_summary(call));
        let _ = std::io::stderr().flush();
        let mut line = String::new();
        if self.input.read_line(&mut line).unwrap_or(0) == 0 {
            eprintln!();
            return false;
        }
        match line.trim().to_ascii_lowercase().as_str() {
            "y" | "yes" => true,
            "a" | "always" => {
                self.always = true;
                true
            }
            _ => false,
        }
    }
}

/// The keyboard's answer to one approval prompt. The `Always` latch is
/// run-scoped, exactly like `ConsoleApprover`'s.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Approval {
    /// Allow this call only.
    Once,
    /// Allow this call and every later one in the run.
    Always,
    /// Refuse this call.
    Denied,
}

/// One pending approval crossing to the UI thread: what to show, plus the
/// channel the decision returns on. The responder is a std channel on
/// purpose: the agent's `approve` blocks on it from inside its tokio
/// runtime, where tokio's own `blocking_recv` refuses to run.
#[derive(Debug)]
pub struct ApprovalRequest {
    /// The call awaiting a decision.
    pub call: ToolCall,
    /// One-line summary for the prompt, already bounded.
    pub summary: String,
    /// Where the decision goes; dropping it denies.
    pub responder: std::sync::mpsc::Sender<Approval>,
}

/// Approval gate that asks the UI thread and waits for the keyboard. The
/// semantics are `ConsoleApprover`'s exactly: `y`/`yes` approves one call,
/// `a`/`always` latches the rest of the run, anything else denies.
///
/// `approve` blocks until the UI answers, so it runs on the agent's
/// dedicated runtime thread while the UI thread stays free to render.
pub struct TuiApprover {
    requests: UnboundedSender<ApprovalRequest>,
    always: bool,
}

impl TuiApprover {
    /// An approver that sends its prompts to `requests`.
    pub fn new(requests: UnboundedSender<ApprovalRequest>) -> TuiApprover {
        TuiApprover { requests, always: false }
    }

    /// The approver and the receiving end the UI loop drains.
    pub fn channel() -> (TuiApprover, UnboundedReceiver<ApprovalRequest>) {
        let (requests, receiver) = tokio::sync::mpsc::unbounded_channel();
        (TuiApprover::new(requests), receiver)
    }
}

impl Approver for TuiApprover {
    fn approve(&mut self, call: &ToolCall) -> bool {
        if self.always {
            return true;
        }
        let (responder, decision) = std::sync::mpsc::channel();
        let request =
            ApprovalRequest { call: call.clone(), summary: call_summary(call), responder };
        // A UI that went away denies rather than hanging the run.
        if self.requests.send(request).is_err() {
            return false;
        }
        match decision.recv() {
            Ok(Approval::Always) => {
                self.always = true;
                true
            }
            Ok(Approval::Once) => true,
            // A dropped responder or an explicit denial.
            _ => false,
        }
    }
}

/// Runs tasks against the API with the built-in tools.
pub struct Agent<'a> {
    client: &'a Client,
    options: AgentOptions,
    session: &'a mut Session,
    messages: Vec<Message>,
    approver: Box<dyn Approver + 'a>,
    progress: &'a mut dyn Write,
    ui_tx: Option<UnboundedSender<UiEvent>>,
    usage: Usage,
    turns: u64,
    /// The lane children this run owns. A run that never calls a lane
    /// tool spawns nothing; a run that does closes every child it
    /// spawned when it ends.
    lanes: LaneRoster,
}

impl<'a> Agent<'a> {
    /// An agent over `session`; an existing log's records seed the
    /// conversation so resumed runs continue in context.
    pub fn new(
        client: &'a Client,
        options: AgentOptions,
        session: &'a mut Session,
        approver: Box<dyn Approver + 'a>,
        progress: &'a mut dyn Write,
    ) -> Result<Agent<'a>, AgentError> {
        let records = crate::session::replay(session.path()).map_err(AgentError::Io)?;
        let ui_tx = options.ui_tx.clone();
        // An injected spec map wins for the lanes it names; every other
        // lane resolves through the vendored registry at call time, with
        // its named reason when it cannot launch.
        let lanes = LaneRoster::new(options.lane_specs.clone().unwrap_or_default());
        Ok(Agent {
            client,
            options,
            session,
            messages: crate::session::messages(&records),
            approver,
            progress,
            ui_tx,
            usage: Usage::default(),
            turns: 0,
            lanes,
        })
    }

    /// Token accounting accumulated across the run's requests.
    pub fn usage(&self) -> Usage {
        self.usage
    }

    /// Run `task` to a stop. The session log receives every message and the
    /// `end` record; an API error leaves the log without `end`, which is the
    /// documented interrupted-log shape.
    ///
    /// The exchange is streamed: content deltas are written to `progress`
    /// as raw text with no formatting, so a headless run shows tokens live
    /// and a TUI receives the same events. The assembled assistant message
    /// is logged once, exactly as before — deltas are transport, not log
    /// content.
    ///
    /// A `ui_tx` in the options receives every event beside the progress
    /// writes; without one the run is byte-for-byte the headless run it
    /// has always been.
    ///
    /// The run owns its lane children: every lane it spawned closes on
    /// the way out, on every exit path — orderly stop, turn cap, or API
    /// failure alike. A teardown that goes wrong never fails a run that
    /// is already over.
    pub async fn run(&mut self, task: &str) -> Result<RunReport, AgentError> {
        let outcome = self.run_task(task).await;
        self.lanes.close_all().await;
        match outcome {
            Ok(report) => Ok(report),
            Err(error) => {
                self.emit(UiEvent::Failed(error.to_string()));
                Err(error)
            }
        }
    }

    async fn run_task(&mut self, task: &str) -> Result<RunReport, AgentError> {
        // A routed run announces its route once, before the first turn: the
        // headless surface prints the same fact, and the TUI's status bar
        // reads it. A plain client emits nothing here.
        if let Some(decision) = self.client.route_decision() {
            self.emit(UiEvent::Routed {
                provider: decision.provider.clone(),
                how: decision.how.label().to_string(),
            });
        }
        self.push_message(Message::text(Role::User, task))?;
        while self.turns < self.options.max_turns as u64 {
            self.turns += 1;
            self.emit(UiEvent::Turn(self.turns));
            let _ = writeln!(self.progress, "\u{2014} turn {}", self.turns);
            let _ = self.progress.flush();

            // The system prompt is a compiled-in constant, like the tool
            // declarations: model-visible but not session-specific, so it is
            // reconstructed per request rather than logged.
            let mut request_messages = Vec::with_capacity(self.messages.len() + 1);
            request_messages.push(Message::text(Role::System, SYSTEM_PROMPT));
            request_messages.extend(self.messages.iter().cloned());
            // The declarations are dynamic now: the four built-ins, plus
            // every lane that has joined this run so far, in join order.
            // A lane's tools enter the set on its first call and stay for
            // the rest of the run — and because they are dynamic, each
            // join is recorded in the log as `lane_tools`, so the request
            // the model saw stays reconstructable.
            let request = ChatRequest {
                model: &self.options.model,
                messages: &request_messages,
                tools: self.lanes.declarations(),
            };
            let client = self.client;
            let progress = &mut *self.progress;
            let ui_tx = self.ui_tx.clone();
            let response = client
                .chat_stream(&request, |event| {
                    if let StreamEvent::Content(delta) = event {
                        let _ = progress.write_all(delta.as_bytes());
                        let _ = progress.flush();
                        if let Some(tx) = &ui_tx {
                            let _ = tx.send(UiEvent::Token(delta.clone()));
                        }
                    }
                })
                .await
                .map_err(AgentError::Api)?;
            self.usage.prompt_tokens += response.usage.prompt_tokens;
            self.usage.completion_tokens += response.usage.completion_tokens;
            let choice = response.choices.into_iter().next().ok_or_else(|| {
                AgentError::Api(ClientError::Decode("response had no choices".to_string()))
            })?;

            let message = choice.message;
            for call in &message.tool_calls {
                let _ = writeln!(self.progress, "\u{2192} {}", describe_call(call));
                self.emit(UiEvent::ToolCall {
                    name: call.function.name.clone(),
                    summary: call_summary(call),
                });
            }
            let _ = self.progress.flush();

            // The assistant turn is logged and kept in the live conversation
            // with its tool calls intact: a record without them breaks resume
            // (the replay emits an assistant message with no calls followed by
            // a tool result — not a valid sequence), and the next request in
            // this same run would carry the same invalid sequence. KI-016.
            if message.tool_calls.is_empty() {
                let content = message.content.clone().unwrap_or_default();
                self.push_message(message)?;
                let turns = self.turns;
                self.record(&Record::End {
                    stop_reason: "completed".to_string(),
                    turns,
                })?;
                self.emit(UiEvent::Finished { stop_reason: "completed".to_string() });
                return Ok(RunReport {
                    stop_reason: StopReason::Completed,
                    turns,
                    usage: self.usage,
                    final_message: Some(content),
                });
            }

            let tool_calls = message.tool_calls.clone();
            self.push_message(message)?;
            for call in tool_calls {
                self.execute_call(&call).await?;
            }
        }

        let turns = self.turns;
        self.record(&Record::End { stop_reason: "max-turns".to_string(), turns })?;
        self.emit(UiEvent::Finished { stop_reason: "max-turns".to_string() });
        Ok(RunReport {
            stop_reason: StopReason::MaxTurns,
            turns,
            usage: self.usage,
            final_message: None,
        })
    }

    /// Execute one call behind the approval gate and record what the model
    /// will see. Unknown tools and malformed arguments are model-visible
    /// error results, so the model can correct itself.
    ///
    /// A lane call goes through the same gate, the same result record, and
    /// the same conversation message a built-in does. It differs only in
    /// where the work happens: the lane's own child process, spawned on
    /// this call if the run has not met the lane yet.
    async fn execute_call(&mut self, call: &ToolCall) -> Result<(), AgentError> {
        let approved = self.approver.approve(call);
        let tool = Tool::from_name(&call.function.name);
        let (output, truncated) = if !approved {
            ("denied: the user did not approve this call.".to_string(), false)
        } else if let Some((lane, lane_tool)) = tools::lane_of(&call.function.name) {
            self.call_lane(call, lane, lane_tool).await?
        } else {
            match tool {
                None => (
                    format!("error: unknown tool `{}`", call.function.name),
                    false,
                ),
                Some(tool) => match serde_json::from_str(&call.function.arguments) {
                    Err(error) => (format!("error: arguments were not valid JSON: {error}"), false),
                    Ok(arguments) => {
                        // KI-017: tool execution is blocking work (a `bash`
                        // child can run for minutes), so it leaves the async
                        // context on the blocking pool instead of stalling
                        // the runtime's worker thread.
                        match tokio::task::spawn_blocking(move || tool.execute(&arguments)).await {
                            Err(error) => (format!("error: the tool task failed: {error}"), false),
                            Ok(Ok(outcome)) => (outcome.output, outcome.truncated),
                            Ok(Err(message)) => (format!("error: {message}"), false),
                        }
                    }
                },
            }
        };
        let _ = writeln!(
            self.progress,
            "\u{2190} {}",
            if approved { preview(&output, 80) } else { "denied".to_string() },
        );
        let _ = self.progress.flush();
        self.emit(UiEvent::ToolResult {
            name: call.function.name.clone(),
            preview: preview(&output, 80),
            approved,
        });
        self.record(&Record::ToolResult {
            call_id: call.id.clone(),
            name: call.function.name.clone(),
            output: output.clone(),
            truncated,
            approved,
        })?;
        self.messages.push(Message::tool_result(call.id.clone(), output));
        Ok(())
    }

    /// Execute one lane tool call behind the approval gate: the lane's
    /// child spawns on this call when the run has not met the lane yet,
    /// and the answer maps onto the same `(output, truncated)` pair the
    /// built-ins produce.
    ///
    /// A lane that just joined is visible twice over: one progress line
    /// and one UI event for the user, one `lane_tools` record for the
    /// log — the record is what keeps the model's now-dynamic tool set
    /// reconstructable.
    async fn call_lane(
        &mut self,
        call: &ToolCall,
        lane: &str,
        lane_tool: &str,
    ) -> Result<(String, bool), AgentError> {
        let arguments = parse_arguments(&call.function.arguments);
        let outcome = self.lanes.call(lane, lane_tool, &arguments).await;
        if let Some(tools) = outcome.joined {
            let count = tools.len();
            let _ = writeln!(
                self.progress,
                "\u{2192} lane {lane} joined \u{2014} {count} tool{}",
                if count == 1 { "" } else { "s" },
            );
            let _ = self.progress.flush();
            self.emit(UiEvent::LaneJoined { lane: lane.to_string(), tool_count: count });
            self.record(&Record::LaneTools { lane: lane.to_string(), tools })?;
        }
        // The `truncated` flag is the lane's, not a guess: it rode out of
        // the same `bound` the built-ins go through (KI-041), so the log
        // says whether the model read a whole answer or a cut one.
        Ok((outcome.text, outcome.truncated))
    }

    /// Send one event to the UI, when a UI is listening. A closed channel
    /// is not a failure: the log and the run report are the record.
    fn emit(&self, event: UiEvent) {
        if let Some(tx) = &self.ui_tx {
            let _ = tx.send(event);
        }
    }

    fn push_message(&mut self, message: Message) -> Result<(), AgentError> {
        let record = match message.role {
            Role::User => Record::User {
                content: message.content.clone().unwrap_or_default(),
            },
            Role::Assistant => Record::Assistant {
                content: message.content.clone(),
                tool_calls: message.tool_calls.clone(),
            },
            Role::Tool => Record::ToolResult {
                call_id: message.tool_call_id.clone().unwrap_or_default(),
                name: String::new(),
                output: message.content.clone().unwrap_or_default(),
                truncated: false,
                approved: true,
            },
            Role::System => return Ok(()),
        };
        self.record(&record)?;
        self.messages.push(message);
        Ok(())
    }

    fn record(&mut self, record: &Record) -> Result<(), AgentError> {
        self.session.record(record).map_err(AgentError::Io)
    }
}

/// Parse a call's arguments for display only.
fn parse_arguments(arguments: &str) -> serde_json::Value {
    serde_json::from_str(arguments).unwrap_or(serde_json::Value::Null)
}

/// One-line summary of what a call does, for prompts and UI events.
fn call_summary(call: &ToolCall) -> String {
    if let Some((lane, tool)) = tools::lane_of(&call.function.name) {
        return lane_summary(lane, tool, &parse_arguments(&call.function.arguments));
    }
    let tool = Tool::from_name(&call.function.name);
    tool.map(|tool| tool.summarize(&parse_arguments(&call.function.arguments), 72))
        .unwrap_or_else(|| call.function.name.clone())
}

/// One line naming the lane and the call, for an approval prompt and a
/// UI event: the lane's role in the house pattern, the lane's own tool
/// name, and the arguments as one compact line — the facts a user needs
/// to answer "may this run?".
fn lane_summary(lane: &str, tool: &str, arguments: &serde_json::Value) -> String {
    let role = lanes::LANES
        .iter()
        .find(|candidate| candidate.name == lane)
        .map(|candidate| candidate.role)
        .unwrap_or("lane");
    format!("{role} \u{b7} {tool} {}", compact_arguments(arguments, 60))
}

/// `arguments` as one compact JSON line, cut at `limit` characters. An
/// absent or non-object argument set reads as `{}`.
fn compact_arguments(arguments: &serde_json::Value, limit: usize) -> String {
    let text = serde_json::to_string(arguments).unwrap_or_else(|_| "{}".to_string());
    let mut compact: String = text.chars().take(limit).collect();
    if text.chars().count() > limit {
        compact.push('\u{2026}');
    }
    compact
}

/// `tool: summary` for a progress line.
fn describe_call(call: &ToolCall) -> String {
    if tools::lane_of(&call.function.name).is_some() {
        return format!("{}: {}", call.function.name, call_summary(call));
    }
    let tool = Tool::from_name(&call.function.name);
    let arguments = parse_arguments(&call.function.arguments);
    match tool {
        Some(tool) => format!("{}: {}", tool.name(), tool.summarize(&arguments, 72)),
        None => format!("{}: (unknown tool)", call.function.name),
    }
}

/// First `limit` characters of `text`, first line only, for progress output.
pub(crate) fn preview(text: &str, limit: usize) -> String {
    let first_line = text.lines().next().unwrap_or("");
    let mut preview: String = first_line.chars().take(limit).collect();
    if first_line.chars().count() > limit {
        preview.push('\u{2026}');
    }
    preview
}

/// The lane children one run owns.
///
/// A lane spawns on the first call to one of its tools — a run that never
/// touches a lane starts no child, so the lazy path costs nothing — and
/// every child it spawns closes when the run ends. A lane that cannot
/// spawn, cannot complete its handshake, or dies under a call is dead for
/// the rest of the run: later calls fail fast with the reason, and
/// nothing respawns. No lane failure is ever fatal to the run; each one
/// is a typed failure result the model can read and react to.
struct LaneRoster {
    /// Launch specs injected for this run, keyed by lane name. Empty on
    /// the production path, where a lane resolves through the vendored
    /// registry the first time it is called.
    specs: HashMap<String, ServerSpec>,
    /// One entry per lane this run has met.
    children: HashMap<String, LaneChild>,
    /// Lane names in join order, so the declaration order is stable and
    /// the log reads in the order the run met the lanes.
    order: Vec<String>,
}

/// One lane's child process for the length of a run.
enum LaneChild {
    /// Spawned, initialized, and listed: its tools are in the model's
    /// tool set until the run ends, dead child or not.
    Live { server: McpServer, tools: Vec<LaneTool> },
    /// Failed or gone: the tools it brought stay declared — the model saw
    /// them, and the log records them — but every call fails fast.
    Dead { reason: String, tools: Vec<LaneTool> },
}

/// What one lane call produced.
struct LaneOutcome {
    /// The text the model sees, in exactly the shape a built-in's output
    /// takes — a refusal or a transport failure carries the `error:`
    /// marker the built-ins use — and bounded by the same
    /// `MAX_TOOL_OUTPUT` cap a built-in's output is (KI-041).
    text: String,
    /// Whether the cap cut that text. The real flag, not a guess: it
    /// rides into the session log's `tool_result` record beside the
    /// built-ins' own, so the log says what the model actually read.
    truncated: bool,
    /// The roster that joined on this call, when one did. The caller
    /// records it: the log is what keeps the dynamic declarations
    /// reconstructable.
    joined: Option<Vec<LaneTool>>,
}

impl LaneRoster {
    /// A roster over `specs`; an empty map is the production path.
    fn new(specs: HashMap<String, ServerSpec>) -> LaneRoster {
        LaneRoster { specs, children: HashMap::new(), order: Vec::new() }
    }

    /// Every declaration the run's requests carry: the four built-ins,
    /// then each joined lane's tools under the lane namespace, in join
    /// order.
    fn declarations(&self) -> Vec<crate::api::ToolSpec> {
        tools::declarations(&self.lane_tools())
    }

    /// Each joined lane's tools as the model sees them, in join order. A
    /// lane that died after joining keeps its tools here: the model saw
    /// them declared, so withdrawing them would make the log's
    /// `lane_tools` record stop matching the requests that followed.
    fn lane_tools(&self) -> Vec<LaneTools> {
        self.order
            .iter()
            .filter_map(|lane| match self.children.get(lane) {
                Some(LaneChild::Live { tools, .. }) | Some(LaneChild::Dead { tools, .. }) => {
                    Some(LaneTools { lane: lane.clone(), tools: tools.clone() })
                }
                None => None,
            })
            .collect()
    }

    /// The launch spec for `lane`: an injected one when the run carries
    /// it, else the vendored registry's — which names the reason when a
    /// lane cannot launch.
    fn spec_for(&self, lane: &str) -> Result<ServerSpec, String> {
        match self.specs.get(lane) {
            Some(spec) => Ok(spec.clone()),
            None => lanes::spec_for(lane),
        }
    }

    /// Call one lane tool, with its answer bounded. Spawns and handshakes
    /// the lane first when this run has not met it yet.
    ///
    /// The cap lives here, at the one seam every lane answer crosses, so
    /// a success, a refusal, and a typed failure are all bounded the same
    /// way — and so the `truncated` flag the log records is the flag
    /// `tools::bound` actually set (KI-041). Before this, a lane's text
    /// rode into the log and the model's context at whatever length the
    /// lane emitted, capped only by the transport's 8 MiB line limit.
    async fn call(
        &mut self,
        lane: &str,
        tool: &str,
        arguments: &serde_json::Value,
    ) -> LaneOutcome {
        let outcome = self.call_unbounded(lane, tool, arguments).await;
        let bounded = tools::bound(ToolOutcome { output: outcome.text, truncated: false });
        LaneOutcome {
            text: bounded.output,
            truncated: bounded.truncated,
            joined: outcome.joined,
        }
    }

    /// The lane call itself, before the output cap. Everything the lane
    /// path does — the lazy join, the dead-lane fail-fast, the call and
    /// its typed failures — and nothing about how much text came back.
    async fn call_unbounded(
        &mut self,
        lane: &str,
        tool: &str,
        arguments: &serde_json::Value,
    ) -> LaneOutcome {
        let mut joined: Option<Vec<LaneTool>> = None;
        if !self.children.contains_key(lane) {
            match self.join(lane).await {
                Ok((server, listed)) => {
                    // The roster moves into the namespace once, at the
                    // join: the declarations, the log record, and the
                    // call below all read the same names.
                    let tools = tools::namespaced(lane, &listed);
                    joined = Some(tools.clone());
                    self.order.push(lane.to_string());
                    self.children.insert(lane.to_string(), LaneChild::Live { server, tools });
                }
                // A lane that cannot start is dead for the rest of the
                // run: the reason is recorded once and reused, so a
                // broken lane costs one spawn attempt, not one per call.
                // It brought no tools, so it declares none.
                Err(reason) => {
                    let text = format!("error: lane `{lane}` could not start: {reason}");
                    self.children.insert(
                        lane.to_string(),
                        LaneChild::Dead { reason, tools: Vec::new() },
                    );
                    return LaneOutcome { text, truncated: false, joined: None };
                }
            }
        }
        // The lane's child exists now: live, or dead from an earlier call.
        let Some(child) = self.children.get_mut(lane) else {
            // Unreachable: the branch above inserted this lane's child,
            // and nothing removes one while a run is in flight.
            return LaneOutcome {
                text: format!("error: lane `{lane}` has no child to call"),
                truncated: false,
                joined,
            };
        };
        match child {
            // Dead: fail fast. No respawn, and the run carries on.
            LaneChild::Dead { reason, .. } => LaneOutcome {
                text: format!(
                    "error: lane `{lane}` is unavailable for the rest of this run: {reason}"
                ),
                truncated: false,
                joined,
            },
            LaneChild::Live { server, tools } => {
                // The model called `mcp__<lane>__<short>`; the declaration
                // it answered is the one that name maps to, which is the
                // lane's own tool name for the call itself.
                let wanted = tools::lane_tool_name(lane, tool);
                let Some(listed) = tools.iter().find(|listed| listed.name == wanted) else {
                    // The lane is healthy and simply does not carry this
                    // tool: a model-visible correction, not a dead lane.
                    return LaneOutcome {
                        text: format!("error: lane `{lane}` has no tool `{tool}`"),
                        truncated: false,
                        joined,
                    };
                };
                let lane_tool = listed.lane_tool.clone();
                match server.call_tool(&lane_tool, arguments.clone()).await {
                    Ok(result) => LaneOutcome {
                        text: lane_call_output(&result),
                        truncated: false,
                        joined,
                    },
                    // The child died under the call. The lane is dead for
                    // the rest of the run; the run itself carries on with
                    // the typed failure in its log and its conversation.
                    // The tools it brought stay declared.
                    Err(error) => {
                        let reason = error.to_string();
                        *child = LaneChild::Dead {
                            reason: reason.clone(),
                            tools: std::mem::take(tools),
                        };
                        LaneOutcome { text: format!("error: {reason}"), truncated: false, joined }
                    }
                }
            }
        }
    }

    /// Spawn one lane and complete its handshake: spawn, `initialize`,
    /// `tools/list`. A failure at any step tears the child down and names
    /// the step that failed, which the caller records as the lane's
    /// death.
    async fn join(&self, lane: &str) -> Result<(McpServer, Vec<ToolInfo>), String> {
        let spec = self.spec_for(lane)?;
        let server = McpServer::spawn(&spec)
            .map_err(|error| format!("lane `{lane}` could not start: {error}"))?;
        if let Err(error) = server.initialize().await {
            let _ = server.close().await;
            return Err(format!("lane `{lane}` could not initialize: {error}"));
        }
        match server.list_tools().await {
            Ok(tools) => Ok((server, tools)),
            Err(error) => {
                let _ = server.close().await;
                Err(format!("lane `{lane}` could not list its tools: {error}"))
            }
        }
    }

    /// Close every child this run spawned, on every exit path. Best
    /// effort by design: a wedged child is killed, and a teardown failure
    /// never fails a run that is already over.
    async fn close_all(&mut self) {
        for (_, child) in self.children.drain() {
            if let LaneChild::Live { server, .. } = child {
                let _ = server.close().await;
            }
        }
        self.order.clear();
    }
}

/// What the model sees for one lane call: the server's text, and the
/// `error:` marker a refusal carries — the same failure vocabulary the
/// built-ins use, so the model reads one shape either way.
fn lane_call_output(result: &CallResult) -> String {
    let text = result.text();
    if result.is_error {
        format!("error: {text}")
    } else {
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::FunctionCall;

    fn call(id: &str, name: &str, arguments: &str) -> ToolCall {
        ToolCall {
            id: id.to_string(),
            kind: "function".to_string(),
            function: FunctionCall { name: name.to_string(), arguments: arguments.to_string() },
        }
    }

    #[test]
    fn preview_keeps_one_line_and_a_bound() {
        assert_eq!(preview("a\nb", 10), "a");
        assert_eq!(preview("x".repeat(30).as_str(), 10), format!("{}{}", "x".repeat(10), '\u{2026}'));
    }

    #[test]
    fn describe_call_handles_unknown_tools() {
        let text = describe_call(&call("1", "launch_missiles", "{}"));
        assert!(text.contains("unknown tool"));
    }

    #[test]
    fn tui_approver_round_trips_a_decision_and_latches_always() {
        // A fake UI loop: receive the request, answer it, and assert what
        // crossed the channel. No terminal, no agent run.
        let (approver, mut requests) = TuiApprover::channel();
        let ui = std::thread::spawn(move || {
            let first = requests.blocking_recv().unwrap();
            assert_eq!(first.call.function.name, "bash");
            assert_eq!(first.summary, "ls -la");
            first.responder.send(Approval::Once).unwrap();
            let second = requests.blocking_recv().unwrap();
            second.responder.send(Approval::Always).unwrap();
            // The latch means no third prompt ever arrives.
            assert!(requests.try_recv().is_err());
        });
        let mut approver = approver;
        assert!(approver.approve(&call("c1", "bash", r#"{"command":"ls -la"}"#)));
        // The second prompt is answered `always`, which latches the rest.
        assert!(approver.approve(&call("c2", "bash", r#"{"command":"pwd"}"#)));
        ui.join().unwrap();
        // Latched: no prompt crosses the channel for the third call.
        assert!(approver.approve(&call("c3", "bash", r#"{"command":"whoami"}"#)));
    }

    #[test]
    fn tui_approver_denies_when_the_ui_gone_or_says_no() {
        let (approver, requests) = TuiApprover::channel();
        drop(requests);
        let mut approver = approver;
        assert!(!approver.approve(&call("c1", "bash", "{}")));

        let (approver, mut requests) = TuiApprover::channel();
        let ui = std::thread::spawn(move || {
            let request = requests.blocking_recv().unwrap();
            request.responder.send(Approval::Denied).unwrap();
        });
        let mut approver = approver;
        assert!(!approver.approve(&call("c2", "bash", "{}")));
        ui.join().unwrap();
    }

    #[test]
    fn a_lane_calls_summary_names_the_lane_the_role_and_the_arguments() {
        // The approval prompt is the same gate a built-in goes through,
        // so a lane call owes the user the same facts: which lane, what
        // kind of worker it is, and what the call asks for.
        let probe = call("c1", "mcp__sem1__probe", r#"{"provider":"llama-server"}"#);
        assert_eq!(call_summary(&probe), "embeddings \u{b7} probe {\"provider\":\"llama-server\"}");
        // A long argument set is cut, not wrapped into the prompt.
        let long = call("c2", "mcp__sem1__embed", &format!(r#"{{"items":"{}"}}"#, "x".repeat(200)));
        let summary = call_summary(&long);
        assert!(summary.chars().count() <= 80, "{summary}");
        assert!(summary.ends_with('\u{2026}'), "{summary}");
        // A built-in's summary is untouched.
        assert_eq!(call_summary(&call("c3", "bash", r#"{"command":"ls -la"}"#)), "ls -la");
        // And the progress line names the model-visible tool name.
        assert!(describe_call(&probe).starts_with("mcp__sem1__probe: embeddings"));
    }

    #[test]
    fn a_refused_lane_call_is_a_failure_that_keeps_its_text() {
        // `isError` means the tool ran and refused: the model reads the
        // refusal, so the text survives and the marker matches the one
        // the built-ins use for their own failures.
        let refused = crate::mcp::CallResult {
            content: vec![crate::mcp::ContentBlock {
                content_type: "text".to_string(),
                data: serde_json::json!("`items` must be a string"),
            }],
            is_error: true,
        };
        assert_eq!(lane_call_output(&refused), "error: `items` must be a string");
        // A success carries no marker.
        let answered = crate::mcp::CallResult {
            content: vec![crate::mcp::ContentBlock {
                content_type: "text".to_string(),
                data: serde_json::json!("{\"dims\": 768}"),
            }],
            is_error: false,
        };
        assert_eq!(lane_call_output(&answered), "{\"dims\": 768}");
    }

    /// A roster whose one lane cannot even spawn: no such program.
    fn unspawnable_roster() -> LaneRoster {
        let mut specs: HashMap<String, ServerSpec> = HashMap::new();
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
        LaneRoster::new(specs)
    }

    #[tokio::test]
    async fn a_lane_that_cannot_start_dies_once_and_fails_fast_after() {
        let mut roster = unspawnable_roster();
        // The first call tries the spawn and names the reason.
        let first = roster.call("sem1", "probe", &serde_json::json!({})).await;
        assert!(first.joined.is_none(), "a lane that cannot start never joins");
        assert!(first.text.starts_with("error: lane `sem1` could not start:"), "{}", first.text);
        // The second call does not try again: the lane is dead for the
        // rest of the run, and it says so.
        let second = roster.call("sem1", "probe", &serde_json::json!({})).await;
        assert!(
            second.text.starts_with("error: lane `sem1` is unavailable for the rest of this run:"),
            "{}",
            second.text
        );
        // A dead lane contributes no declarations: the model never saw a
        // tool that cannot run.
        assert_eq!(roster.declarations().len(), 4);
        assert!(roster.lane_tools().is_empty());
        // And an unknown lane is a named reason, not a panic — and not a
        // spawn: no registry lane carries this name, so nothing launches.
        let unknown = roster.call("nosuchlane", "classify", &serde_json::json!({})).await;
        assert!(unknown.text.contains("no lane named `nosuchlane`"), "{}", unknown.text);
        assert!(unknown.joined.is_none());
    }

    #[tokio::test]
    async fn a_lane_that_never_joins_leaves_the_declarations_at_the_builtins() {
        // The lazy path's cost when no lane is called: nothing.
        let roster = LaneRoster::new(HashMap::new());
        assert_eq!(roster.declarations().len(), 4);
        let names: Vec<String> = roster
            .declarations()
            .iter()
            .map(|spec| serde_json::to_value(spec).unwrap()["function"]["name"]
                .as_str()
                .unwrap()
                .to_string())
            .collect();
        assert_eq!(names, vec!["bash", "read_file", "write_file", "list_dir"]);
    }
}
