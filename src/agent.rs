//! The agent loop: task in, model turns and approved tool calls out, until
//! the model answers without tools or the turn cap is reached.
//!
//! Every message is recorded to the session log as it happens, so the log
//! always contains exactly what the model saw.
//!
//! Three disciplines hold this loop's shape, and each is structural
//! rather than advisory:
//!
//! - **A turn's independent calls run together, bounded.** The batch is
//!   planned into waves no larger than [`DELEGATION_CONCURRENCY`]; a call
//!   that mutates shared state takes its writer's slot and runs serially
//!   in the model's own order — never reordered, never silently
//!   ([`plan_batch`]). The gate is crossed one call at a time, so the
//!   owner decides every call in the batch, never the model and never
//!   two at once.
//! - **A running session can be steered and stopped** through a control
//!   file beside its own log, read before each model call. A steer rides
//!   the next turn as a message labelled as the owner's; a stop lands
//!   the partial result with the typed reason ([`Control`]).
//! - **A write is checked against the project's vocabulary** before it
//!   lands: a case or separator variant of an established term is
//!   refused, a novel name with a canonical near-neighbour is flagged
//!   with the candidate named, and a genuinely new concept is written
//!   untouched ([`write_guard`]).

use crate::api::{ChatRequest, Client, ClientError, Message, Role, StreamEvent, ToolCall, Usage};
use crate::compaction;
use crate::lanes;
use crate::mcp::{CallResult, McpServer, ServerSpec, ToolInfo};
use crate::memory::{self, Memory};
use crate::session::{Conversation, Record, Session};
use crate::tools::{self, LaneTool, LaneTools, Tool, ToolOutcome};
use std::collections::{HashMap, VecDeque};
use std::io::Write;
use std::pin::Pin;
use std::sync::Arc;
use std::task::Poll;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

/// System prompt sent with every request.
pub const SYSTEM_PROMPT: &str = "You are Fleet, a native Rust coding agent based on the DeepSeek Harness design. Work in the user's current directory. Use the provided tools to inspect and change files, run commands, and verify your work. Read before you write. When a task is done, reply with a short summary of the outcome; if part of it failed, say exactly what failed and what you tried.";

/// Turn cap when the caller does not choose one.
pub const DEFAULT_MAX_TURNS: usize = 25;

/// The soft cap on how many skills one run declares in its system
/// prompt. Past it, one line is logged naming the count — a warning
/// only: every skill stays declared and callable, and nothing is hidden.
/// The list grows the prompt on every request of every run, so an
/// unbounded list is an unbounded per-turn cost; curation retiring
/// unused skills is the lever that brings it back down.
pub const SKILLS_PROMPT_SOFT_CAP: usize = 50;

/// How many of one turn's independent tool calls may run at once.
///
/// Three, sized for a single owner's machine and the embedder's single
/// slot. The bound is a ceiling and never a floor: the one config seam
/// ([`concurrency_bound`]'s `FLEET_CONCURRENCY` read) can only lower it,
/// because an unbounded fan-out turns one bad brief into N bad briefs on
/// one clock. A heavier machine is a change to this constant, not a
/// config value.
pub const DELEGATION_CONCURRENCY: usize = 3;

/// The one config seam over [`DELEGATION_CONCURRENCY`]: an environment
/// variable naming a smaller positive number. It lives here, beside the
/// constant it lowers, so there is exactly one place the bound can move.
const FLEET_CONCURRENCY: &str = "FLEET_CONCURRENCY";

/// The bound this process executes batches under: the named constant,
/// lowered by the config seam when it names a smaller positive number.
///
/// Read once per batch rather than cached, because a run is one process
/// and a batch is one turn: the cost is one environment lookup per turn,
/// and a long-lived surface that wants the bound to move mid-run gets
/// it without a restart.
fn concurrency_bound() -> usize {
    clamp_concurrency(DELEGATION_CONCURRENCY, std::env::var(FLEET_CONCURRENCY).ok().as_deref())
}

/// The clamp, pure so it is testable without touching the environment:
/// `requested` when it names a positive number at or under `ceiling`,
/// else the ceiling itself.
///
/// A malformed or absent request leaves the constant standing — a bound
/// that silently becomes one because of a typo is a bound nobody asked
/// for — and a request above the ceiling is refused rather than honored,
/// which is what "config can only lower" means.
fn clamp_concurrency(ceiling: usize, requested: Option<&str>) -> usize {
    requested
        .and_then(|requested| requested.trim().parse::<usize>().ok())
        .filter(|requested| *requested >= 1)
        .map_or(ceiling, |requested| requested.min(ceiling))
}

/// The one logged line when a run declares more than
/// [`SKILLS_PROMPT_SOFT_CAP`] skills; `None` at or under the cap.
fn skills_prompt_growth_line(declared: usize) -> Option<String> {
    (declared > SKILLS_PROMPT_SOFT_CAP).then(|| {
        format!(
            "fleet: {declared} skills declared; the list is growing the prompt \u{2014} curation retires unused ones"
        )
    })
}

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
    /// The calibration stream this run's rows are recorded to. `None` is
    /// the silent path — a test that does not care about the stream, or a
    /// surface that has none.
    pub calib: Option<crate::calib::Stream>,
    /// The skills this run declares. The list rides the system prompt as
    /// name+description pairs and `use_skill` fetches a body on demand,
    /// so an empty list is the run this crate has always had: the same
    /// prompt, the same four declarations, the same log. A non-empty
    /// list is recorded once per run as the `skills` record, which is
    /// what keeps the prompt reconstructable from the log.
    pub skills: Vec<crate::skills::Skill>,
    /// The memory this run carries. `None` is the run this crate has
    /// always had: the same prompt, the same declarations, the same
    /// log. `Some` declares the write path (`remember`, `supersede`)
    /// beside `use_skill`, assembles the turn's context — the
    /// protected tier, shape-routed recall, and recent working memory
    /// — into the system prompt, records one `memory` record per run
    /// so the prompt stays reconstructable from the log (hard rule 5),
    /// and records the recall hits the assembly surfaced. An assembly
    /// with nothing in it adds no block and no record: a memory-less
    /// turn is byte-for-byte the run it has always been.
    pub memory: Option<Arc<Memory>>,
    /// The tool set this run is admitted to. `None` is the run this
    /// crate has always had: every declaration the run earns, and every
    /// call it makes, goes through. `Some` is a subagent's own run — a
    /// scope that decides both which declarations its requests carry and
    /// which calls the loop refuses by name.
    pub scope: Option<crate::subagents::Scope>,
    /// The home this run's `improve` tool files proposals into. `None`
    /// is the run that cannot propose: the tool is not declared, the
    /// way a memory-less run does not declare `remember`. `Some` is
    /// every surface that knows its home — `run`, `chat`, the TUI. The
    /// tool only *files*; applying is the owner's `fleet improve
    /// apply`, through the same gate, so a session can propose a change
    /// to the harness but never apply one.
    pub improve_home: Option<std::path::PathBuf>,
}

impl Default for AgentOptions {
    fn default() -> AgentOptions {
        AgentOptions {
            model: crate::api::DEFAULT_MODEL.to_string(),
            max_turns: DEFAULT_MAX_TURNS,
            ui_tx: None,
            lane_specs: None,
            calib: None,
            skills: Vec::new(),
            memory: None,
            scope: None,
            improve_home: None,
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
    /// The conversation outgrew the window and was compacted: `turns`
    /// turns became one summary, and the context went from
    /// `tokens_before` to `tokens_after`. Emitted once per crossing,
    /// beside the `compaction` record the log carries; a run that never
    /// crosses the budget emits nothing, so the ordinary run is unchanged.
    Compacted { turns: usize, tokens_before: u64, tokens_after: u64 },
    /// What this run's assembled memory carried: the shape the
    /// instruction routed under (empty when unclassified — the
    /// documented fallback) and how many facts entered the turn's
    /// context. Emitted once per run, beside the `memory` record; a
    /// run whose assembly was empty emits nothing, so the memory-less
    /// run is unchanged.
    Memory { shape: String, facts: usize },
    /// Where a delegated child run stands, reported to the parent's
    /// live surface while the parent's turn waits for it.
    ///
    /// The child's own turn, token, and finish events deliberately do
    /// NOT ride here: they belong to the child's conversation, and
    /// landing them in the parent's would move its turn counter and
    /// clear its working state mid-run. What crosses is the fact a
    /// surface needs — a child exists, it is working, it stopped — so
    /// the `subagent` call reads as one tool card that goes from
    /// pending to done with the outcome on it.
    Subagent { goal: String, session: String, stage: SubagentStage },
    /// A scheduled job fired. The scheduler thread — which the TUI
    /// spawns beside the agent's own, because a fire is an ordinary
    /// headless run in its own session log rather than a turn in the
    /// session this interface is driving — reports each finished fire
    /// here, so the outcome lands as a card instead of a stderr line
    /// scratched over the alternate screen (KI-090).
    CronFire {
        /// The job's name, the store's key.
        job: String,
        /// The minute the fire ran for, stamped.
        minute: String,
        /// Whether this fire served a window missed while fleet was
        /// down (the catch-up rule) rather than its own minute.
        catch_up: bool,
        /// How it ended: `completed`, `failed`,
        /// `skipped-needs-approval`, or `panicked`.
        outcome: String,
        /// The one-line reason, the same text the headless line
        /// carries beside the job and the minute.
        detail: String,
        /// The session log this run wrote, when it got as far as one:
        /// the retained output, reachable from the card.
        session: Option<String>,
    },
    /// The loop stopped orderly.
    Finished { stop_reason: String },
    /// The loop failed; the run is over.
    Failed(String),
}

/// Where a delegated child run stands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubagentStage {
    /// The child's session exists and its run is starting.
    Spawned,
    /// The child is working: `calls` of its tool calls have crossed the
    /// parent's approval gate so far, the last one named `tool`.
    Working { calls: u64, tool: String },
    /// The child stopped, at `turns`, with `outcome` named in the session
    /// log's own ending vocabulary (`completed`/`max-turns`/`failed`).
    Done { outcome: String, turns: u64 },
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
    /// The owner stopped this run through its control file. The typed
    /// reason is the sentence itself: a stop lands the partial result
    /// with this reason and the run's own log path, never silence, so
    /// the one thing the error must not do is be vague.
    Stopped,
}

impl std::fmt::Display for AgentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentError::Api(error) => write!(f, "{error}"),
            AgentError::Io(error) => write!(f, "session log write failed: {error}"),
            AgentError::Stopped => write!(f, "stopped by the owner"),
        }
    }
}

impl std::error::Error for AgentError {}

/// What a run's control file says, read once and consumed.
///
/// The channel is a file rather than a signal (design decision 6): a
/// signal cannot carry a steer's text and cannot be logged. The file is
/// the run's own — `<sessions_dir>/<session_id>.control`, beside the log
/// it steers — and reading an absent one costs a single failed open, so
/// the turn loop checks it before every model call without a watcher, a
/// thread, or a poll.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Control {
    /// Text the owner wants this run to take into its next turn. It
    /// rides as a user-role message labelled as the owner's, never as
    /// the model's own words.
    Steer(String),
    /// The owner wants this run to end now. The partial result lands
    /// with the typed reason and the run's own log path.
    Stop,
}

/// Decides whether one tool call may execute.
pub trait Approver {
    /// True when the call may run.
    fn approve(&mut self, call: &ToolCall) -> bool;

    /// The model-visible text a denial carries, or `None` for the
    /// ordinary interactive one — "the user did not approve this
    /// call." An approver that denies for a reason of its own, where
    /// no user exists to name, supplies its own sentence here: the
    /// unattended job gate denies precisely because nobody is there,
    /// so naming a user who is guaranteed absent is the one signal
    /// the model gets and it has to be true (KI-088).
    fn denial_reason(&self) -> Option<String> {
        None
    }

    /// Whether this approver's approvals come from a run-scoped latch
    /// rather than a per-call decision: `--yes`, or the console/TUI `a`
    /// key. The calibration row reads it to name the third verdict
    /// (`always`) — recording happens after the decision, never inside it.
    fn latched(&self) -> bool {
        false
    }
}

/// Approves everything (`--yes`).
#[derive(Debug, Default)]
pub struct ApproveAll;

impl Approver for ApproveAll {
    fn approve(&mut self, _call: &ToolCall) -> bool {
        true
    }

    fn latched(&self) -> bool {
        // `--yes` is a run-scoped always: no per-call decision happens.
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

    fn latched(&self) -> bool {
        self.always
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

    fn latched(&self) -> bool {
        self.always
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
    /// The skills this run declared, in load order. Their
    /// name+description pairs ride the system prompt; a body enters
    /// context only when the loop selects it through `use_skill`.
    skills: Vec<crate::skills::Skill>,
    /// The memory this run carries, when it carries one. The corpus
    /// lives behind an `Arc` because the agent borrows it for the run
    /// while the options that built it stay owned.
    memory: Option<Arc<Memory>>,
    /// The tool set this run is admitted to, for a subagent's own run.
    /// `None` is the ordinary run: every declaration the run earns, and
    /// every call it makes, goes through.
    scope: Option<crate::subagents::Scope>,
    /// The home this run's `improve` tool files proposals into, when it
    /// has one. The tool only files; applying is the owner's gate.
    improve_home: Option<std::path::PathBuf>,
    /// The calibration stream this run records to, when it has one.
    calib: Option<crate::calib::Stream>,
    /// One turn number per message in `messages`: the turn of the request
    /// that first carried it. A compaction record names a turn range, so
    /// the range has to be a fact about the conversation rather than a
    /// guess, and a resumed run has to continue the log's own numbering.
    message_turns: Vec<u64>,
    /// The turn the next request will be. Seeded from the log on a resume
    /// so a compacted session's later turns number from where its earlier
    /// ones stopped.
    next_turn: u64,
    /// True from a compaction (or a failed or skipped one) until a
    /// measurement comes in under the budget. The latch is what makes
    /// one compaction per crossing rather than one per turn, and it is
    /// also what keeps a tail that is itself over the window — or a
    /// fold that keeps failing — from being retried every turn.
    compaction_latch: bool,
    /// The API-reported `prompt_tokens` of the last response, or `None`
    /// when it reported none — in which case the window check falls back
    /// to the chars/4 estimate of the assembled messages.
    last_prompt_tokens: Option<u64>,
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
        let skills = options.skills.clone();
        let memory = options.memory.clone();
        let scope = options.scope.clone();
        let improve_home = options.improve_home.clone();
        // An injected spec map wins for the lanes it names; every other
        // lane resolves through the vendored registry at call time, with
        // its named reason when it cannot launch.
        let lanes = LaneRoster::new(options.lane_specs.clone().unwrap_or_default());
        let calib = options.calib.clone();
        // A resumed run seeds its conversation from the log, and the log's
        // own turn numbering with it: the compacted shape is materialized
        // here, so a compacted session resumes as the compacted
        // conversation it is rather than as the raw records.
        let Conversation { messages, turns, next_turn } =
            crate::session::conversation(&records);
        Ok(Agent {
            client,
            options,
            session,
            messages,
            approver,
            progress,
            ui_tx,
            usage: Usage::default(),
            turns: 0,
            lanes,
            skills,
            memory,
            scope,
            calib,
            message_turns: turns,
            next_turn,
            compaction_latch: false,
            last_prompt_tokens: None,
            improve_home,
        })
    }

    /// Token accounting accumulated across the run's requests.
    pub fn usage(&self) -> Usage {
        self.usage
    }

    /// The system prompt this run's requests carry: the compiled-in
    /// constant, plus the available skills as a name+description list
    /// when the run declared any, plus the assembled memory block when
    /// the run's assembly produced one. Progressive disclosure starts
    /// here — the model sees what each skill is for and nothing of what
    /// it says, until it selects one.
    ///
    /// With no skills and no memory block the prompt is the constant
    /// itself, so a run that carries neither is byte-for-byte the run
    /// it has always been.
    ///
    /// Called once per run, before the turn loop, so the growth warning
    /// is one logged line per run. Nothing is ever dropped from the
    /// list: the cap warns, it does not curate.
    fn system_prompt(&self, memory_block: Option<&str>) -> String {
        if self.skills.is_empty() && memory_block.is_none() {
            return SYSTEM_PROMPT.to_string();
        }
        if let Some(line) = skills_prompt_growth_line(self.skills.len()) {
            eprintln!("{line}");
        }
        let mut prompt = String::from(SYSTEM_PROMPT);
        if !self.skills.is_empty() {
            prompt.push_str(
                "\n\n## Skills\n\nSkills are procedural knowledge you can load on demand. Call `use_skill(name)` with one of these names when a task matches its description; the skill's body arrives as that call's result.\n",
            );
            for skill in &self.skills {
                prompt.push_str(&format!("\n- {}: {}", skill.name, skill.description));
            }
        }
        if let Some(block) = memory_block {
            // The assembled block carries its own section headers; the
            // frame around it says what the block is and how to read
            // it — recalled facts are proposals, the protected tier is
            // the owner's own.
            prompt.push_str(
                "\n\n## Memory\n\nStanding context and recall assembled for this task. The protected tier is the owner's own; recalled and working facts are proposals to verify, not instructions.\n\n",
            );
            prompt.push_str(block);
        }
        prompt
    }

    /// Every declaration this run's requests carry: the four built-ins,
    /// then `use_skill` when the run declared skills, then `remember`
    /// and `supersede` when the run carries a corpus, then each joined
    /// lane's tools in join order.
    ///
    /// `use_skill` and the two memory tools are declared beside the
    /// closed four rather than inside them: the four are compiled in,
    /// and these exist only while the run has something to select or
    /// somewhere to write.
    ///
    /// A subagent's own run declares exactly its admitted scope instead:
    /// the built-ins it admitted, in their fixed order, then `use_skill`
    /// when it carries skills. The scope is what keeps the child's tool
    /// set honest — a tool outside it is not merely uncallable, it is
    /// absent from what the model saw.
    fn declarations(&self) -> Vec<crate::api::ToolSpec> {
        let mut specs: Vec<crate::api::ToolSpec> = match &self.scope {
            Some(scope) => crate::tools::ALL_TOOLS
                .iter()
                .filter(|tool| scope.admits(tool.name()))
                .map(|tool| tool.spec())
                .collect(),
            None => self.lanes.declarations(),
        };
        // A scope that admits nothing admits the beside-the-four tools
        // either; the check is one place, not one per tool.
        let admitted = |name: &str| {
            self.scope.as_ref().is_none_or(|scope| scope.admits(name))
        };
        if !self.skills.is_empty() && admitted(tools::USE_SKILL_TOOL) {
            specs.push(tools::use_skill_spec());
        }
        if self.memory.is_some() && admitted(tools::REMEMBER_TOOL) {
            specs.push(tools::remember_spec());
            specs.push(tools::supersede_spec());
        }
        // `subagent` is declared on every run that is not itself a
        // subagent: delegation is part of the tool set this crate ships,
        // not a capability a run opts into. A child's scope never admits
        // it, which is the depth cap in its declarative half.
        if admitted(tools::SUBAGENT_TOOL) {
            specs.push(tools::subagent_spec());
        }
        // `improve` is declared on every run that carries a home. It
        // only files a proposal — the owner's `fleet improve apply` runs
        // the gate — so a session can propose a change to the harness
        // but never apply one, the way the fork's Creator installs
        // through Plugin Manager rather than through a model tool.
        if self.improve_home.is_some() && admitted(tools::IMPROVE_TOOL) {
            specs.push(tools::improve_spec());
        }
        specs
    }

    /// The typed refusal for a call outside this run's admitted scope,
    /// or `None` for an unscoped run or an admitted name.
    ///
    /// The scope check sits after the approval gate rather than before
    /// it, which is the order every other call takes: the owner sees a
    /// subagent reach for something it was not given, and the harness
    /// refuses it. A gate-first order also means the screening row the
    /// stream carries is the gate's own verdict, not a second kind of
    /// verdict invented here.
    fn scope_refusal(&self, name: &str) -> Option<String> {
        let scope = self.scope.as_ref()?;
        if scope.admits(name) {
            return None;
        }
        // The one refusal with a reason of its own: a subagent reaching
        // for delegation is the depth cap, named.
        if name == tools::SUBAGENT_TOOL {
            return Some(format!(
                "error: `{}` is outside this subagent's admitted scope: a subagent cannot spawn subagents (depth cap {})",
                tools::SUBAGENT_TOOL,
                crate::subagents::SUBAGENT_MAX_DEPTH
            ));
        }
        Some(format!(
            "error: `{name}` is outside this subagent's admitted scope ({}); ask for it in the task, or do it yourself",
            scope.names().join(", ")
        ))
    }

    /// Assemble this run's memory: the protected tier, shape-routed
    /// recall, and the recent working memory, as the block the system
    /// prompt carries. Three things ride along with it — the recall
    /// hits the assembly earned (one `recall_hit` row per fact that
    /// entered, at the score it entered at), the `memory` record that
    /// keeps the prompt reconstructable from the log (hard rule 5, the
    /// `skills` record's precedent), and one progress line and UI event
    /// naming the shape and the fact count.
    ///
    /// `None` when nothing assembled: no protected facts, no recall
    /// above the floor, no working memory. A memory-less turn is then
    /// byte-for-byte the run it has always been — the same prompt, no
    /// record, no rows.
    fn prepare_memory(&mut self, task: &str) -> Result<Option<String>, AgentError> {
        let Some(memory) = self.memory.clone() else { return Ok(None) };
        let assembly = memory.assemble(task, &memory.working());
        let block = memory::render_assembly(&assembly);
        if block.is_empty() {
            return Ok(None);
        }
        memory.record_hits(&assembly.recalled, assembly.shape, self.calib.as_ref());
        let shape = assembly.shape.unwrap_or_default().to_string();
        let facts: Vec<String> = assembly
            .protected
            .iter()
            .map(|hit| hit.fact.name.clone())
            .chain(assembly.recalled.iter().map(|hit| hit.fact.name.clone()))
            .chain(assembly.working.iter().map(|fact| fact.name.clone()))
            .collect();
        self.record(&Record::Memory { shape: shape.clone(), facts: facts.clone() })?;
        let named = if shape.is_empty() {
            "no shape (surface recall)".to_string()
        } else {
            format!("shape {shape}")
        };
        let _ = writeln!(
            self.progress,
            "\u{2014} memory: {named} \u{b7} {} fact{}",
            facts.len(),
            if facts.len() == 1 { "" } else { "s" },
        );
        let _ = self.progress.flush();
        self.emit(UiEvent::Memory { shape, facts: facts.len() });
        Ok(Some(block))
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
        // One turn row per run, on every exit path — the outcome's grade
        // and the ending named. Best-effort by construction.
        self.record_turn(task, &outcome);
        match outcome {
            Ok(report) => Ok(report),
            Err(error) => {
                self.emit(UiEvent::Failed(error.to_string()));
                Err(error)
            }
        }
    }

    /// The stop reason's name in the calibration row: the session log's
    /// own ending vocabulary, carried verbatim.
    fn turn_stop(reason: StopReason) -> crate::calib::TurnStop {
        match reason {
            StopReason::Completed => crate::calib::TurnStop::Completed,
            StopReason::MaxTurns => crate::calib::TurnStop::MaxTurns,
        }
    }

    /// Record the run's ending. An API failure that reached the loop's own
    /// end is `failed`, with the router's failure class; an I/O failure is
    /// `interrupted`, because it is what leaves the session log without
    /// its `end` record — the documented interrupted-log shape.
    fn record_turn(&self, task: &str, outcome: &Result<RunReport, AgentError>) {
        let Some(stream) = &self.calib else { return };
        let (stop, reason) = match outcome {
            Ok(report) => (Self::turn_stop(report.stop_reason), None),
            Err(AgentError::Api(error)) => (
                crate::calib::TurnStop::Failed,
                Some(crate::route::failure_class(error).to_string()),
            ),
            Err(AgentError::Io(_)) => {
                (crate::calib::TurnStop::Interrupted, Some("io".to_string()))
            }
            // An owner's stop is `interrupted` in the calibration's own
            // vocabulary — the class that means "stopped from outside",
            // which is absent outcome evidence rather than a named
            // absence, so a stopped run never reads as a stall that was
            // missing a companion — and the typed reason rides the row.
            Err(AgentError::Stopped) => (
                crate::calib::TurnStop::Interrupted,
                Some("stopped by the owner".to_string()),
            ),
        };
        let (turns, usage) = match outcome {
            Ok(report) => (report.turns, report.usage),
            Err(_) => (self.turns, self.usage),
        };
        stream.record(crate::calib::Row::Turn(crate::calib::TurnRow {
            ts: crate::calib::stamp(),
            outcome: stop.outcome().to_string(),
            stop_reason: stop.label().to_string(),
            turns,
            usage,
            input_chars: task.chars().count(),
            input_hash: crate::calib::hash16(task),
            reason,
        }));
    }

    /// Record the screening verdict for one tool call — after the
    /// decision, never inside it.
    fn record_screen(&self, call: &ToolCall, approved: bool) {
        let Some(stream) = &self.calib else { return };
        let verdict = if !approved {
            crate::calib::ScreenVerdict::Denied
        } else if self.approver.latched() {
            crate::calib::ScreenVerdict::Always
        } else {
            crate::calib::ScreenVerdict::Approved
        };
        let (lane, _) = tools::lane_of(&call.function.name).unzip();
        stream.record(crate::calib::Row::ToolScreen(crate::calib::ToolScreenRow {
            ts: crate::calib::stamp(),
            tool: call.function.name.clone(),
            verdict: verdict.label().to_string(),
            lane: lane.map(str::to_string),
            input_chars: call.function.arguments.chars().count(),
            input_hash: crate::calib::hash16(&call.function.arguments),
        }));
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
        // The skills this run declared, recorded once, at run start. The
        // list is dynamic — the files change between runs — and the
        // system prompt carries it, so the log is what makes the prompt
        // the model saw reconstructable (hard rule 5). The lane_tools
        // precedent, applied to the skills seam. A run with no skills
        // records nothing and its prompt is the compiled-in constant, so
        // the skill-less run is byte-for-byte the run it has always been.
        if !self.skills.is_empty() {
            let declared: Vec<crate::skills::SkillRef> =
                self.skills.iter().map(crate::skills::SkillRef::from).collect();
            self.record(&Record::Skills { skills: declared })?;
        }
        // The assembled memory block, recorded the same way: the block
        // is dynamic — it depends on the corpus and the instruction —
        // so the `memory` record is what keeps the prompt the model saw
        // reconstructable (hard rule 5). An empty assembly adds
        // nothing, and the prompt stays the compiled-in constant.
        let memory_block = self.prepare_memory(task)?;
        let system_prompt = self.system_prompt(memory_block.as_deref());
        self.push_message(Message::text(Role::User, task))?;
        while self.turns < self.options.max_turns as u64 {
            self.turns += 1;
            self.emit(UiEvent::Turn(self.turns));
            let _ = writeln!(self.progress, "\u{2014} turn {}", self.turns);
            let _ = self.progress.flush();

            // The control channel, read before every model call. An
            // absent file is one failed open and nothing else; a steer
            // rides this turn's request as a message labelled as the
            // owner's; a stop ends the run here, before another token
            // is spent and with the partial result still in hand.
            match self.read_control() {
                Some(Control::Steer(text)) => self.apply_steer(&text)?,
                Some(Control::Stop) => return self.stop_by_owner(),
                None => {}
            }

            // The system prompt is the compiled-in constant plus this
            // run's skills list, which the `skills` record above carries:
            // model-visible and reconstructable, like the tool
            // declarations.
            let mut request_messages = Vec::with_capacity(self.messages.len() + 1);
            request_messages.push(Message::text(Role::System, &system_prompt));
            request_messages.extend(self.messages.iter().cloned());
            // The declarations are dynamic now: the four built-ins, the
            // `use_skill` tool when this run carries skills, plus every
            // lane that has joined this run so far, in join order. A
            // lane's tools enter the set on its first call and stay for
            // the rest of the run — and because they are dynamic, each
            // join is recorded in the log as `lane_tools`, so the request
            // the model saw stays reconstructable.
            let request = ChatRequest {
                model: &self.options.model,
                messages: &request_messages,
                tools: self.declarations(),
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
            // The API's own count of this request's prompt, when it
            // reported one. A provider that reports nothing leaves this
            // `None`, and the window check falls back to the estimate.
            self.last_prompt_tokens =
                (response.usage.prompt_tokens > 0).then_some(response.usage.prompt_tokens);
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
            // The batch: the gate one call at a time, the approved calls
            // under the plan, every result recorded in the batch's own
            // order. The recursion this loop carries — a delegation runs
            // a child agent that reaches this same line — is pinned by
            // the one `Box::pin` on the serial lane inside
            // `execute_calls`, the same indirection `api::chat_stream`
            // needed when the router called back into it.
            self.execute_calls(&tool_calls).await?;
            // Between turns: the window check. It runs after the last
            // response landed and before the next request is built, so
            // the message list it measures is the one the next request
            // would carry — and a compaction it performs is already in
            // place by the time that request is assembled.
            self.maybe_compact(&system_prompt).await?;
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

    /// The window check, run between turns: after the last response landed
    /// and before the next request is built.
    ///
    /// The measure is the API-reported `prompt_tokens` of the last
    /// response when it carried one — the streamed usage does, so this is
    /// the ordinary case — and the chars/4 estimate of the assembled
    /// messages when it did not. A conversation under
    /// [`compaction::COMPACTION_TOKEN_BUDGET`] costs one comparison and
    /// nothing else, so the run that never approaches the window is
    /// byte-for-byte the run it has always been.
    ///
    /// One compaction per crossing: the latch holds from a compaction
    /// (or a failed or skipped one) until a measurement comes back under
    /// the budget, so a conversation whose tail is itself over the
    /// window is not folded every turn.
    async fn maybe_compact(&mut self, system_prompt: &str) -> Result<(), AgentError> {
        let measured = self
            .last_prompt_tokens
            .unwrap_or_else(|| compaction::estimate_tokens(&self.messages));
        if measured < compaction::COMPACTION_TOKEN_BUDGET {
            self.compaction_latch = false;
            return Ok(());
        }
        // Over the budget. Nothing to replace means nothing to compact:
        // the whole conversation is the verbatim tail already.
        let tail_start = compaction::tail_start(&self.messages);
        if self.compaction_latch || tail_start == 0 {
            return Ok(());
        }
        self.compact(system_prompt, tail_start, measured).await
    }

    /// Summarize-and-continue: one request through this run's own client
    /// — the same routed path every other request takes, no separate
    /// provider and no separate model — carrying the messages being
    /// replaced plus the compaction instruction. The model's answer is
    /// the summary.
    ///
    /// The request is model-visible, so it is logged like any other
    /// request (hard rule 5): the `compaction` record names the range it
    /// replaces, and those records are in the log before it, so the
    /// request itself is reconstructable — system prompt (the compiled-in
    /// constant), the replaced messages (the preceding records), the
    /// instruction ([`compaction::COMPACTION_INSTRUCTION`]).
    ///
    /// The summary rides as a user-role message framed by
    /// [`compaction::summary_frame`], the most recent
    /// [`compaction::COMPACTION_TAIL_TURNS`] turns stay verbatim after
    /// it, and the record carries the counts either side of the fold.
    async fn compact(
        &mut self,
        system_prompt: &str,
        tail_start: usize,
        tokens_before: u64,
    ) -> Result<(), AgentError> {
        // The range the summary replaces: from the first message's turn to
        // the last assistant answer before the tail. A tool result is
        // numbered for the request that carried it, so the last *assistant*
        // is the honest end of the range.
        let first_turn = self.message_turns.first().copied().unwrap_or(1);
        let last_turn = self.message_turns[..tail_start]
            .iter()
            .zip(&self.messages[..tail_start])
            .filter(|(_, message)| message.role == Role::Assistant)
            .map(|(turn, _)| *turn)
            .last()
            .unwrap_or(first_turn);
        let turns = compaction::turns_in_range(first_turn, last_turn);

        let mut request_messages = Vec::with_capacity(tail_start + 2);
        request_messages.push(Message::text(Role::System, system_prompt));
        request_messages.extend(self.messages[..tail_start].iter().cloned());
        request_messages.push(Message::text(Role::User, compaction::COMPACTION_INSTRUCTION));
        // No tools: a compaction request has nothing to execute, and a
        // declaration on it would invite a call with no context to run in.
        let request = ChatRequest {
            model: &self.options.model,
            messages: &request_messages,
            tools: Vec::new(),
        };
        // The deltas are transport, not content: the summary arrives whole
        // in the response, and streaming it to the progress sink would
        // put a second model's prose in the middle of the run's output.
        let client = self.client;
        let response = match client.chat_stream(&request, |_| {}).await {
            Ok(response) => response,
            Err(error) => {
                // The fold is an auxiliary call: its failure must not
                // kill a healthy run. The conversation keeps its
                // context, the failure is named on one line, and the
                // latch holds so a failing fold is not retried every
                // turn. The run then either finishes inside the
                // window's headroom or dies at the API's own length
                // refusal, which names the real limit — a more honest
                // death than an auxiliary call's transport error.
                let _ = writeln!(
                    self.progress,
                    "\u{2014} compaction failed: {error}; the run continues un-compacted"
                );
                let _ = self.progress.flush();
                self.compaction_latch = true;
                return Ok(());
            }
        };
        self.usage.prompt_tokens += response.usage.prompt_tokens;
        self.usage.completion_tokens += response.usage.completion_tokens;
        let summary = response
            .choices
            .into_iter()
            .next()
            .and_then(|choice| choice.message.content)
            .unwrap_or_default();
        // An empty summary would replace the conversation with nothing.
        // The fold is skipped rather than performed: the run keeps its
        // context and its next request, and the log records no compaction
        // that did not happen.
        if summary.trim().is_empty() {
            let _ = writeln!(
                self.progress,
                "\u{2014} compaction skipped: the model returned no summary"
            );
            let _ = self.progress.flush();
            // The latch holds: a model that returns no summary
            // deterministically would otherwise be asked once per turn.
            self.compaction_latch = true;
            return Ok(());
        }

        let mut compacted = Vec::with_capacity(self.messages.len() - tail_start + 1);
        compacted.push(Message::text(
            Role::User,
            compaction::summary_frame(first_turn, last_turn, &summary),
        ));
        compacted.extend_from_slice(&self.messages[tail_start..]);
        let tokens_after = compaction::estimate_tokens(&compacted);
        self.messages = compacted;
        self.message_turns =
            std::iter::once(first_turn).chain(self.message_turns[tail_start..].iter().copied())
                .collect();

        self.record(&Record::Compaction {
            summary: summary.clone(),
            first_turn,
            last_turn,
            tokens_before,
            tokens_after,
        })?;
        let _ = writeln!(
            self.progress,
            "\u{2014} compacted: {turns} turn{} \u{2192} 1 summary (~{tokens_before} \u{2192} ~{tokens_after} tokens)",
            if turns == 1 { "" } else { "s" },
        );
        let _ = self.progress.flush();
        self.emit(UiEvent::Compacted { turns, tokens_before, tokens_after });
        // The latch holds until a measurement comes back under the
        // budget, which is what keeps one compaction per crossing.
        self.compaction_latch = true;
        Ok(())
    }

    /// Where this run's control file lives: its own log's path with the
    /// extension swapped, so a child's control file is
    /// `<sessions_dir>/<child_session_id>.control` — the file the owner
    /// writes, beside the log it steers.
    fn control_path(&self) -> std::path::PathBuf {
        self.session.path().with_extension("control")
    }

    /// Read the control file, consuming it. `None` when there is nothing
    /// to read, and an absent file is one failed open — the whole cost
    /// of checking before every model call.
    ///
    /// Both shapes are one-shot: consumed on read, so a steer rides
    /// exactly one turn and a stop is not read twice. A file that is not
    /// a control file — unreadable JSON, or JSON naming neither a steer
    /// nor a stop — is named on one progress line and left in place: a
    /// malformed file is evidence of something, and silently eating it
    /// would hide that (hard rule 10).
    fn read_control(&mut self) -> Option<Control> {
        let path = self.control_path();
        let text = std::fs::read_to_string(&path).ok()?;
        let control = match serde_json::from_str::<serde_json::Value>(&text) {
            Err(error) => {
                let _ = writeln!(
                    self.progress,
                    "\u{2014} control: {} is not valid JSON ({error}); ignored",
                    path.display()
                );
                let _ = self.progress.flush();
                return None;
            }
            Ok(value) => parse_control(&value),
        };
        match control {
            None => {
                let _ = writeln!(
                    self.progress,
                    "\u{2014} control: {} names no steer and no stop; ignored",
                    path.display()
                );
                let _ = self.progress.flush();
                None
            }
            Some(control) => {
                // Consumed on read. A removal that fails is not a
                // failure: the control already rode, and a second read
                // of the same text would be a duplicate, not a
                // corruption.
                let _ = std::fs::remove_file(&path);
                Some(control)
            }
        }
    }

    /// Land a steer as the next turn's first message: a user-role
    /// message framed as the owner's, recorded through `push_message`
    /// like every other model-visible message (hard rule 5), so a
    /// steered run's trajectory is reconstructable from its log alone.
    fn apply_steer(&mut self, text: &str) -> Result<(), AgentError> {
        let bounded = steer_text(text);
        let _ = writeln!(self.progress, "\u{2014} steer: {}", preview(&bounded, 72));
        let _ = self.progress.flush();
        self.push_message(Message::text(Role::User, &steer_frame(&bounded)))
    }

    /// Land an owner's stop: the progress line, the log's own ending
    /// naming who stopped it, and the typed error the caller turns into
    /// the partial result. The run's log path and its last words are
    /// what the delegation path replays, so a stop is never silence.
    fn stop_by_owner(&mut self) -> Result<RunReport, AgentError> {
        let turns = self.turns;
        let _ = writeln!(self.progress, "\u{2014} stopped by the owner (control file)");
        let _ = self.progress.flush();
        self.record(&Record::End { stop_reason: "stopped by the owner".to_string(), turns })?;
        Err(AgentError::Stopped)
    }

    /// Execute one turn's batch of tool calls.
    ///
    /// Three phases, in this order and for these reasons:
    ///
    /// 1. **The gate, one call at a time, in the model's order.** A
    ///    concurrent batch's calls still reach the owner through the
    ///    parent's approver serially — the owner decides, never the
    ///    model, and never two prompts at once. A denial, a scope
    ///    refusal, a refused provenance gate, or an argument error
    ///    settles here: the call will not run and its result is known.
    /// 2. **The plan, then the waves.** [`plan_batch`] walks the batch
    ///    in the model's own order and places each call in a wave: a
    ///    call that mutates shared state lands after the last wave that
    ///    carried its writer, so two writes to one path run serially in
    ///    the model's order rather than being silently reordered; every
    ///    wave holds at most [`DELEGATION_CONCURRENCY`] calls, so the
    ///    fan-out is bounded whatever the batch's size. Within a wave,
    ///    the calls that need the loop's own borrow for their whole
    ///    execution run first and alone, and the rest run together
    ///    under the bound.
    /// 3. **Every result recorded in the batch's own order.** The log
    ///    and the next request read exactly as the serial loop wrote
    ///    them; what changed is that the work overlapped, not what the
    ///    model sees.
    ///
    /// Unknown tools and malformed arguments are model-visible error
    /// results, so the model can correct itself — the rule the
    /// single-call path always had.
    async fn execute_calls(&mut self, calls: &[ToolCall]) -> Result<(), AgentError> {
        // Phase 1: the gate.
        let mut prepared: Vec<Option<Prepared>> = Vec::with_capacity(calls.len());
        for call in calls {
            prepared.push(Some(self.prepare_call(call)?));
        }
        // Phase 2: the plan, then the waves.
        let plan = plan_batch(&prepared, concurrency_bound());
        if let Some(line) = plan.line(calls) {
            let _ = writeln!(self.progress, "{line}");
            let _ = self.progress.flush();
        }
        let bound = plan.bound;
        // The plan's split: each wave's concurrent work and serial
        // calls, taken out of the gates so the settled results stay in
        // place for the finalize.
        let mut concurrent: Vec<Vec<(usize, Work)>> = vec![Vec::new(); plan.waves];
        let mut serial: Vec<Vec<(usize, ToolCall)>> = vec![Vec::new(); plan.waves];
        for (index, gate) in prepared.iter_mut().enumerate() {
            // A settled call keeps its result where it is: it will not
            // run, so the plan has nothing to schedule for it.
            if matches!(gate, Some(Prepared::Settled { .. })) {
                continue;
            }
            let Some(gate) = gate.take() else { continue };
            match gate {
                Prepared::Settled { .. } => unreachable!("settled calls were skipped above"),
                Prepared::Ready(work) => concurrent[plan.steps[index].wave].push((index, work)),
                Prepared::Serial { call } => serial[plan.steps[index].wave].push((index, call)),
            }
        }
        let mut outcomes: Vec<Option<(String, bool)>> = calls.iter().map(|_| None).collect();
        for wave in 0..plan.waves {
            // The serial lane first: a delegation borrows the parent's
            // gate and progress sink for its whole run, and a lane call
            // borrows the run's roster, so neither can overlap another
            // call. One `Box::pin` in the cycle, and only because the
            // cycle now exists: this path → the delegation path → a
            // child's own loop → back here. A recursive `async fn`
            // needs one indirection to have a finite future size, the
            // same shape `api::chat_stream` needed when the router
            // called back into it.
            for (index, call) in &serial[wave] {
                let outcome = Box::pin(self.run_serial(call)).await?;
                outcomes[*index] = Some(outcome);
            }
            // The concurrent lane: the calls whose work is owned, run
            // together under the bound. The wave's work was pushed in
            // the batch's own order and `run_bounded` answers in that
            // order, so each result lands on the call that produced it.
            let wave_work = std::mem::take(&mut concurrent[wave]);
            let indices: Vec<usize> = wave_work.iter().map(|(index, _)| *index).collect();
            let outputs = run_bounded(
                wave_work.into_iter().map(|(_, work)| run_work(work)).collect(),
                bound,
            )
            .await;
            for (index, output) in indices.into_iter().zip(outputs) {
                outcomes[index] = Some(output);
            }
        }
        // Phase 3: every result, in the batch's own order.
        for (index, call) in calls.iter().enumerate() {
            let (approved, output, truncated) = match &prepared[index] {
                Some(Prepared::Settled { approved, output, truncated }) => {
                    (*approved, output.clone(), *truncated)
                }
                // Every call the plan scheduled landed in its wave; a
                // hole here would leave the model without a result for
                // a call it made, which is the one thing the log cannot
                // do.
                _ => {
                    let (output, truncated) = outcomes[index]
                        .clone()
                        .expect("every call the plan scheduled landed in its wave");
                    (true, output, truncated)
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
            // The same turn rule `push_message` applies: a tool result
            // belongs to the turn whose request carried it, and it does
            // not advance the turn.
            self.message_turns.push(self.next_turn);
            self.messages.push(Message::tool_result(call.id.clone(), output));
        }
        Ok(())
    }

    /// Execute one call behind the approval gate and record what the
    /// model will see — the single-call form of [`Agent::execute_calls`],
    /// kept because a test drives one call at a time the way the loop
    /// does. The loop itself always goes through the batch.
    #[cfg(test)]
    async fn execute_call(&mut self, call: &ToolCall) -> Result<(), AgentError> {
        self.execute_calls(std::slice::from_ref(call)).await
    }

    /// The gate half of one call: the approval, the screening row, and
    /// what the call turned into — a settled result, owned work, or a
    /// call that needs the loop's own borrow.
    ///
    /// The branch order is the one the single-call path always had, and
    /// it is load-bearing: the denial first (the owner said no), then
    /// the scope refusal (a child reaching outside what it was given),
    /// then each tool in its own order. A lane call goes through the
    /// same gate and the same result record a built-in does; it differs
    /// only in where the work happens — the lane's own child process,
    /// spawned on this call if the run has not met the lane yet.
    fn prepare_call(&mut self, call: &ToolCall) -> Result<Prepared, AgentError> {
        let approved = self.approver.approve(call);
        // The screening verdict, recorded after the decision: the gate's
        // own answer, one row per call, never inside the decision itself.
        self.record_screen(call, approved);
        if !approved {
            // The denial the model reads. The ordinary gate names the
            // user who was asked and said no; an approver that denies
            // where no user exists — the unattended job gate — names
            // its own policy instead (KI-088). Either way the text
            // rides the ordinary `tool_result` path into the log, so
            // model-visible stays model-visible (hard rule 5).
            let output = self
                .approver
                .denial_reason()
                .unwrap_or_else(|| "denied: the user did not approve this call.".to_string());
            return Ok(Prepared::Settled { approved: false, output, truncated: false });
        }
        if let Some(refusal) = self.scope_refusal(&call.function.name) {
            // A subagent reaching outside its admitted scope. The
            // declarations already excluded the tool; this is the
            // refusal that meets a call that arrived anyway, and it
            // names the tool and the scope so the model can correct
            // itself rather than retry.
            return Ok(Prepared::Settled { approved: true, output: refusal, truncated: false });
        }
        let name = call.function.name.as_str();
        if name == tools::USE_SKILL_TOOL {
            // A skill's body is model-visible content, so it crosses the
            // same approval gate every other tool crosses — approving a
            // body into context is the same decision as approving any
            // other text the model will read — and it rides the same
            // bounded tool_result path: the log records exactly the body
            // the model received, with the same truncation marker a
            // built-in's cut carries.
            return Ok(match serde_json::from_str(&call.function.arguments) {
                Err(error) => settled(format!("error: arguments were not valid JSON: {error}")),
                Ok(arguments) => Prepared::Ready(Work::UseSkill {
                    skills: self.skills.clone(),
                    arguments,
                }),
            });
        }
        if name == tools::REMEMBER_TOOL {
            // A durable write is a side effect, so it crosses the same
            // approval gate every other tool crosses — the summary
            // names the fact and its basis — and rides the same bounded
            // tool_result path. The write itself is one fact file, the
            // record the loader reads back.
            return Ok(match serde_json::from_str(&call.function.arguments) {
                Err(error) => settled(format!("error: arguments were not valid JSON: {error}")),
                Ok(arguments) => match self.memory.clone() {
                    None => settled("error: memory is not wired in this run".to_string()),
                    Some(memory) => Prepared::Ready(Work::Remember { memory, arguments }),
                },
            });
        }
        if name.starts_with(tools::SUPERSEDE_TOOL) {
            return self.prepare_supersede(call);
        }
        if name == tools::SUBAGENT_TOOL {
            return Ok(Prepared::Serial { call: call.clone() });
        }
        if name == tools::IMPROVE_TOOL {
            // A self-maintenance proposal is filed, never applied: the
            // tool writes one markdown file into the home's proposals
            // store and the owner's `fleet improve apply` runs the same
            // gate. The call crosses the ordinary approval gate like
            // every other tool — the summary names the proposal — and
            // the confirmation rides the same bounded tool_result path,
            // so model-visible stays model-visible (hard rule 5).
            return Ok(match self.improve_home.clone() {
                None => settled(
                    "error: no home is wired in this run, so there is nowhere to file a proposal"
                        .to_string(),
                ),
                Some(home) => match serde_json::from_str(&call.function.arguments) {
                    Err(error) => settled(format!("error: arguments were not valid JSON: {error}")),
                    Ok(arguments) => Prepared::Ready(Work::Improve {
                        home,
                        // The draft stamps its targets against the
                        // working tree this process runs in — the same
                        // repo `fleet improve apply` resolves — so the
                        // apply can refuse a target that drifted while
                        // the proposal waited.
                        repo: std::env::current_dir().unwrap_or_default(),
                        arguments,
                    }),
                },
            });
        }
        if tools::lane_of(name).is_some() {
            return Ok(Prepared::Serial { call: call.clone() });
        }
        match Tool::from_name(name) {
            None => Ok(settled(format!("error: unknown tool `{name}`"))),
            Some(tool) => match serde_json::from_str(&call.function.arguments) {
                Err(error) => Ok(settled(format!("error: arguments were not valid JSON: {error}"))),
                Ok(arguments) => Ok(Prepared::Ready(Work::Builtin {
                    tool,
                    arguments,
                    // The project root the write-time guard reads the
                    // vocabulary from: the working directory this run
                    // works in.
                    root: std::env::current_dir().unwrap_or_default(),
                })),
            },
        }
    }

    /// The supersede call's gate half: parse, the provenance gate, and
    /// the owned work that lands the write.
    ///
    /// Split from the write itself so a batch's mutating calls can be
    /// approved one at a time through the owner's gate — including the
    /// second, provenance confirmation a confirmed-by fact needs — and
    /// then executed without borrowing the loop.
    fn prepare_supersede(&mut self, call: &ToolCall) -> Result<Prepared, AgentError> {
        let Some(memory) = self.memory.clone() else {
            return Ok(settled("error: memory is not wired in this run".to_string()));
        };
        let arguments = match serde_json::from_str::<serde_json::Value>(&call.function.arguments) {
            Err(error) => {
                return Ok(settled(format!("error: arguments were not valid JSON: {error}")))
            }
            Ok(arguments) => arguments,
        };
        let request = match memory::Supersede::parse(&arguments) {
            Err(error) => return Ok(settled(format!("error: {error}"))),
            Ok(request) => request,
        };
        let mut approved = true;
        if let Some(fact) =
            memory::load_facts(memory.root()).into_iter().find(|fact| fact.name == request.name)
        {
            if fact.basis == memory::Basis::ConfirmedBy {
                let _ = writeln!(
                    self.progress,
                    "\u{2192} supersede `{}` is confirmed-by \u{2014} asking for explicit approval",
                    fact.name
                );
                let _ = self.progress.flush();
                // The prompt names the fact and its confirmed-by basis:
                // the synthetic call's name carries the basis, and its
                // summary carries the fact and the justification.
                let provenance = ToolCall {
                    id: format!("{}-provenance", call.id),
                    kind: "function".to_string(),
                    function: crate::api::FunctionCall {
                        name: format!("{} (confirmed-by)", tools::SUPERSEDE_TOOL),
                        arguments: call.function.arguments.clone(),
                    },
                };
                approved = self.approver.approve(&provenance);
                if !approved {
                    return Ok(Prepared::Settled {
                        approved: true,
                        output: "denied: the fact is confirmed-by and its modification was not approved."
                            .to_string(),
                        truncated: false,
                    });
                }
            }
        }
        Ok(Prepared::Ready(Work::Supersede {
            memory,
            calib: self.calib.clone(),
            request,
            approved,
        }))
    }

    /// Run one call that needs the loop's own borrow for its whole
    /// execution: a delegation, which borrows the parent's gate and
    /// progress sink, and a lane call, which borrows the run's roster.
    async fn run_serial(&mut self, call: &ToolCall) -> Result<(String, bool), AgentError> {
        if call.function.name == tools::SUBAGENT_TOOL {
            return self.run_subagent(call).await;
        }
        let (lane, lane_tool) = tools::lane_of(&call.function.name)
            .expect("a serial call is a delegation or a lane call");
        self.call_lane(call, lane, lane_tool).await
    }

    /// Run one delegated task in its own child session, on this run's
    /// own client and this run's own model, and return what it produced.
    ///
    /// The parent's turn waits: a delegation is synchronous from the
    /// parent's side. What the child gets is this run's resolved client
    /// (so its requests cross the same roster with the same failover),
    /// this run's model and active route decision (so "never downgrades"
    /// is a property of the construction, not a rule the model follows),
    /// this run's approver (so the owner sees a subagent's `bash` call
    /// before it runs), this run's progress sink, this run's declared
    /// skills, and this run's calibration stream. What it does not get
    /// is this run's conversation, this run's memory, or this run's
    /// lanes: the goal is the child's whole context.
    ///
    /// The child's log lives in this log's own directory and names this
    /// session as its parent, so `fleet sessions`, the picker, and
    /// replay treat it as an ordinary session with a marker.
    ///
    /// The result rides the same bounded `tool_result` path every other
    /// tool's output rides, which is what puts the call and its outcome
    /// in the parent's log (hard rule 5).
    async fn run_subagent(&mut self, call: &ToolCall) -> Result<(String, bool), AgentError> {
        let request = match crate::subagents::SubagentRequest::parse(&parse_arguments(
            &call.function.arguments,
        )) {
            Err(error) => return Ok((format!("error: {error}"), false)),
            Ok(request) => request,
        };
        // Every borrow the child needs is taken from a disjoint field, so
        // the parent's approver and progress sink can be lent to it while
        // the parent's own session, client, and options stay in hand.
        let client = self.client;
        let route = client.route_decision();
        let model = self.options.model.clone();
        let calib = self.options.calib.clone();
        let skills = self.skills.clone();
        let ui_tx = self.ui_tx.clone();
        let Some(sessions_dir) = self.session.path().parent().map(std::path::Path::to_path_buf)
        else {
            return Ok((
                "error: the session log has no directory to create a child session in".to_string(),
                false,
            ));
        };
        let parent_session = self.session.id().to_string();
        // An ordinary parent's child sits at depth 1; a subagent's own
        // child would sit at 2, which the cap refuses — and which the
        // scope has already made unreachable by excluding the tool.
        let depth = self.scope.as_ref().map_or(0, |scope| scope.depth) + 1;
        let result = crate::subagents::run(crate::subagents::Delegation {
            client,
            route,
            model,
            sessions_dir,
            parent_session,
            goal: request.goal,
            tools: request.tools,
            max_turns: request.max_turns,
            approver: &mut *self.approver,
            progress: &mut *self.progress,
            ui_tx,
            calib,
            skills,
            depth,
        })
        .await;
        let outcome = crate::subagents::bound_result(&result);
        Ok((outcome.output, outcome.truncated))
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
        let role = message.role;
        let record = match role {
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
        // The turn rule, the same one `session::conversation` applies when
        // it replays a log: a user message and the assistant answer that
        // follows it share a turn, a tool result belongs to the turn whose
        // request carried it, and an assistant answer advances the turn.
        self.message_turns.push(self.next_turn);
        if role == Role::Assistant {
            self.next_turn += 1;
        }
        self.messages.push(message);
        Ok(())
    }

    fn record(&mut self, record: &Record) -> Result<(), AgentError> {
        self.session.record(record).map_err(AgentError::Io)
    }
}

/// What the gate decided for one call.
#[derive(Debug, Clone)]
enum Prepared {
    /// The call will not run: its result is already known — a denial, a
    /// scope refusal, a refused provenance gate, an argument error, or
    /// an unknown tool. `approved` is the gate's own answer, which is
    /// what the log records.
    Settled { approved: bool, output: String, truncated: bool },
    /// The call runs on the concurrent lane: its work is owned, so it
    /// borrows nothing from the loop and can overlap its siblings.
    Ready(Work),
    /// The call needs the loop's own `&mut self` for its whole
    /// execution, so it never overlaps another call.
    Serial { call: ToolCall },
}

/// A settled result for a call that will not run. `approved` is true:
/// the call crossed the gate and was refused for a reason of its own,
/// which is what distinguishes it from a denial.
fn settled(output: String) -> Prepared {
    Prepared::Settled { approved: true, output, truncated: false }
}

/// One concurrent call's work: everything the call needs, owned.
///
/// Ownership is the whole point. None of these borrows the loop, so a
/// batch's calls can be in flight together without the borrow checker
/// serializing them — the only serialization left is the one the
/// one-writer discipline asks for.
#[derive(Debug, Clone)]
enum Work {
    /// A built-in tool's execution, against `root` for the write-time
    /// guard.
    Builtin { tool: Tool, arguments: serde_json::Value, root: std::path::PathBuf },
    /// A skill's body, fetched from this run's declared list.
    UseSkill { skills: Vec<crate::skills::Skill>, arguments: serde_json::Value },
    /// One fact written into this run's corpus.
    Remember { memory: Arc<Memory>, arguments: serde_json::Value },
    /// One proposal filed into the home's store.
    Improve {
        home: std::path::PathBuf,
        repo: std::path::PathBuf,
        arguments: serde_json::Value,
    },
    /// One supersession, already past its provenance gate.
    Supersede {
        memory: Arc<Memory>,
        calib: Option<crate::calib::Stream>,
        request: memory::Supersede,
        approved: bool,
    },
}

/// Run one concurrent call's work to the `(output, truncated)` pair
/// every tool's result takes.
async fn run_work(work: Work) -> (String, bool) {
    match work {
        Work::Builtin { tool, arguments, root } => {
            if tool == Tool::WriteFile {
                // The write-time guard runs before the write, and it
                // runs on the blocking pool: reading the vocabulary and
                // matching names against it is CPU work, the same reason
                // a tool's own execution leaves the async context
                // (KI-017).
                let guard = {
                    let root = root.clone();
                    let arguments = arguments.clone();
                    match tokio::task::spawn_blocking(move || write_guard(&root, &arguments)).await
                    {
                        Err(error) => {
                            return (format!("error: the guard task failed: {error}"), false)
                        }
                        Ok(guard) => guard,
                    }
                };
                match guard {
                    // A near-exact variant of an established term is
                    // drift with no other explanation: the write does
                    // not happen, and the refusal names both names.
                    WriteGuard::Refused { reason } => return (reason, false),
                    // A novel name with a canonical near-neighbour: the
                    // write happens, and the flag rides the result the
                    // model already reads.
                    WriteGuard::Flagged { note } => {
                        let (output, truncated) = run_builtin(tool, arguments).await;
                        return (format!("{output}\n{note}"), truncated);
                    }
                    WriteGuard::Clean => {}
                }
            }
            run_builtin(tool, arguments).await
        }
        Work::UseSkill { skills, arguments } => {
            let outcome = tools::bound(ToolOutcome {
                output: crate::skills::use_skill_result(&skills, &arguments),
                truncated: false,
            });
            (outcome.output, outcome.truncated)
        }
        Work::Remember { memory, arguments } => {
            let outcome = tools::bound(ToolOutcome {
                output: land_remember(&memory, &arguments),
                truncated: false,
            });
            (outcome.output, outcome.truncated)
        }
        Work::Improve { home, repo, arguments } => {
            let outcome = tools::bound(ToolOutcome {
                output: crate::improve::file_from_tool(&home, &repo, &arguments),
                truncated: false,
            });
            (outcome.output, outcome.truncated)
        }
        Work::Supersede { memory, calib, request, approved } => {
            land_supersede(&memory, &request, approved, calib.as_ref())
        }
    }
}

/// One built-in tool's execution, on the blocking pool.
async fn run_builtin(tool: Tool, arguments: serde_json::Value) -> (String, bool) {
    // KI-017: tool execution is blocking work (a `bash` child can run
    // for minutes), so it leaves the async context on the blocking pool
    // instead of stalling the runtime's worker thread.
    match tokio::task::spawn_blocking(move || tool.execute(&arguments)).await {
        Err(error) => (format!("error: the tool task failed: {error}"), false),
        Ok(Ok(outcome)) => (outcome.output, outcome.truncated),
        Ok(Err(message)) => (format!("error: {message}"), false),
    }
}

/// Land one `remember` call against its corpus: the model-visible
/// confirmation or the typed refusal, in the same shape a built-in's
/// output takes — a failure is a value the model can read and react to.
fn land_remember(memory: &Memory, arguments: &serde_json::Value) -> String {
    match memory::Remember::parse(arguments) {
        Err(error) => format!("error: {error}"),
        Ok(request) => match memory.remember(&request) {
            Err(error) => format!("error: {error}"),
            Ok(fact) => format!(
                "remembered `{}` \u{2014} {}, {} \u{b7} {}",
                fact.name,
                fact.tier.label(),
                fact.basis.label(),
                fact.path.display()
            ),
        },
    }
}

/// Land one approved supersession and render what the model reads. The
/// provenance gate is the caller's ([`Agent::prepare_supersede`]); this
/// is the write and its sentence.
fn land_supersede(
    memory: &Memory,
    request: &memory::Supersede,
    approved: bool,
    calib: Option<&crate::calib::Stream>,
) -> (String, bool) {
    match memory.supersede(request, approved, calib) {
        Err(error) => (format!("error: {error}"), false),
        Ok(memory::Superseded::By { from, into }) => (
            format!(
                "superseded `{from}` by `{into}` \u{2014} `{from}` stays on disk, retired from recall"
            ),
            false,
        ),
        Ok(memory::Superseded::Forgotten { fact }) => (
            format!(
                "forgot `{fact}` \u{2014} expired with its retirement noted; the cycle archives the file"
            ),
            false,
        ),
    }
}

/// The shared state one prepared call mutates, named — or `None` for a
/// call that touches no shared state this plan can name.
///
/// The one-writer discipline, made structural rather than advisory (the
/// KI-172/173/177 lesson — three separate occurrences of two writers on
/// one path — and the memory writer's lost-update class):
///
/// - `remember` and `supersede` take the memory writer's slot: two
///   concurrent writes to one corpus is the lost update.
/// - `write_file` takes the path's own slot, so two writes to one path
///   run serially in the model's order rather than being silently
///   reordered.
/// - `bash` takes the shell's slot: a shell command can write anything,
///   so two of them are two writers on the whole tree.
/// - `improve` takes the proposals store's slot, and a lane call takes
///   its lane's.
///
/// Everything else — `read_file`, `list_dir`, `use_skill`, and a
/// delegation, which is independent by construction — carries no writer
/// and runs together. A `write_file` whose path does not parse carries
/// none either: that write cannot happen, so there is no writer to hold
/// apart.
fn writer_of(prepared: &Prepared) -> Option<String> {
    match prepared {
        Prepared::Settled { .. } => None,
        Prepared::Serial { call } => {
            let name = call.function.name.as_str();
            if name == tools::SUBAGENT_TOOL {
                None
            } else {
                tools::lane_of(name).map(|(lane, _)| format!("lane:{lane}"))
            }
        }
        Prepared::Ready(work) => match work {
            Work::Remember { .. } | Work::Supersede { .. } => Some("memory".to_string()),
            Work::Improve { .. } => Some("proposals".to_string()),
            Work::UseSkill { .. } => None,
            Work::Builtin { tool, arguments, .. } => match tool {
                Tool::Bash => Some("shell".to_string()),
                Tool::WriteFile => arguments
                    .get("path")
                    .and_then(|path| path.as_str())
                    .filter(|path| !path.is_empty())
                    .map(|path| format!("path:{path}")),
                Tool::ReadFile | Tool::ListDir => None,
            },
        },
    }
}

/// One call's place in the execution plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PlanStep {
    /// The wave this call runs in. Waves run in order; the calls inside
    /// one run together.
    wave: usize,
    /// True when the call needs the loop's own `&mut self` for its
    /// whole execution, so it never overlaps another call. This is a
    /// structural fact, not a conflict: a delegation borrows the
    /// parent's gate and progress sink, and a lane call borrows the
    /// run's roster.
    exclusive: bool,
}

/// One named writer conflict: the shared state more than one call in
/// the batch mutates, and the calls that carry it in the batch's own
/// order.
///
/// The plan names the conflict rather than silently reordering
/// anything: the concurrent execution of the pair is refused, the
/// writer is named, and the calls run serially in the order the model
/// asked for.
#[derive(Debug, Clone, PartialEq, Eq)]
struct WriterConflict {
    /// The writer, named: `path:src/agent.rs`, `memory`, `shell`, ...
    writer: String,
    /// The batch positions that carry it, in the batch's own order.
    positions: Vec<usize>,
}

/// The execution plan for one turn's batch.
#[derive(Debug, Clone)]
struct BatchPlan {
    /// One step per prepared call, in the batch's own order.
    steps: Vec<PlanStep>,
    /// The writer conflicts the plan held apart, named.
    conflicts: Vec<WriterConflict>,
    /// How many waves the plan runs.
    waves: usize,
    /// The bound every wave is under.
    bound: usize,
}

impl BatchPlan {
    /// One progress line naming the batch: the calls, the waves, the
    /// bound, and every writer conflict the plan held apart.
    ///
    /// `None` for the ordinary one-call turn, which is byte-for-byte
    /// the run this crate has always had.
    fn line(&self, calls: &[ToolCall]) -> Option<String> {
        if self.steps.len() < 2 {
            return None;
        }
        let mut line = format!(
            "\u{2014} batch: {} call{} in {} wave{} (bound {})",
            self.steps.len(),
            if self.steps.len() == 1 { "" } else { "s" },
            self.waves,
            if self.waves == 1 { "" } else { "s" },
            self.bound
        );
        for conflict in &self.conflicts {
            let named: Vec<String> = conflict
                .positions
                .iter()
                .filter_map(|position| calls.get(*position))
                .map(|call| call.id.clone())
                .collect();
            line.push_str(&format!(
                " \u{b7} one writer `{}`: {} run serially",
                conflict.writer,
                named.join(" \u{2192} ")
            ));
        }
        Some(line)
    }
}

/// Plan one turn's batch: which calls run together, which are held apart
/// by a shared writer, and which conflicts that names.
///
/// The algorithm, stated so the plan is auditable rather than magical:
///
/// - the batch is walked in the model's own order, and nothing is ever
///   reordered;
/// - a call with a writer lands in the first wave *after* the last wave
///   that carried the same writer, which is the one-writer discipline:
///   two `write_file`s to one path run serially, in the model's order;
/// - a call without one lands in the first wave with room;
/// - no wave carries more than `bound` calls, so a batch larger than
///   [`DELEGATION_CONCURRENCY`] never exceeds it however it is shaped.
///
/// The plan is a function of the batch and the bound and nothing else —
/// no clock, no randomness — which is what makes it testable without a
/// run.
fn plan_batch(prepared: &[Option<Prepared>], bound: usize) -> BatchPlan {
    let bound = bound.max(1);
    let mut steps: Vec<PlanStep> = Vec::with_capacity(prepared.len());
    let mut sizes: Vec<usize> = Vec::new();
    let mut last_wave: HashMap<String, usize> = HashMap::new();
    let mut carriers: HashMap<String, Vec<usize>> = HashMap::new();
    for (index, gate) in prepared.iter().enumerate() {
        let writer = gate.as_ref().and_then(writer_of);
        if let Some(writer) = &writer {
            carriers.entry(writer.clone()).or_default().push(index);
        }
        // A writer's next call cannot share a wave with its last one.
        let earliest = writer
            .as_ref()
            .and_then(|writer| last_wave.get(writer))
            .map_or(0, |wave| wave + 1);
        let mut wave = earliest;
        while wave < sizes.len() && sizes[wave] >= bound {
            wave += 1;
        }
        if wave == sizes.len() {
            sizes.push(0);
        }
        sizes[wave] += 1;
        steps.push(PlanStep {
            wave,
            exclusive: matches!(gate, Some(Prepared::Serial { .. })),
        });
        if let Some(writer) = writer {
            last_wave.insert(writer, wave);
        }
    }
    let mut conflicts: Vec<WriterConflict> = carriers
        .into_iter()
        .filter(|(_, positions)| positions.len() > 1)
        .map(|(writer, positions)| WriterConflict { writer, positions })
        .collect();
    // Sorted by writer, so the line the plan prints — and any later
    // record that carries it — reads the same on every run.
    conflicts.sort_by(|left, right| left.writer.cmp(&right.writer));
    BatchPlan { steps, conflicts, waves: sizes.len(), bound }
}

/// Drive `works` to completion with at most `bound` in flight,
/// answering in the order the works were given.
///
/// The bounded runner is the one place the fan-out's ceiling is real, so
/// it is stated here and tested directly rather than inferred from a
/// wave's shape. It polls the running futures and starts a queued one
/// the moment a slot frees — the packing a semaphore would give, without
/// spawning, so no future has to be `Send` and a panic propagates
/// instead of vanishing into a join error.
///
/// Every future is polled to completion before this returns, so the
/// answer carries one entry per work, in order, with no holes.
async fn run_bounded<T, F>(works: Vec<F>, bound: usize) -> Vec<T>
where
    F: std::future::Future<Output = T>,
{
    let bound = bound.max(1);
    let total = works.len();
    let mut queue: VecDeque<Pin<Box<F>>> = works.into_iter().map(Box::pin).collect();
    let mut running: Vec<(usize, Pin<Box<F>>)> = Vec::with_capacity(bound.min(total));
    let mut done: Vec<Option<T>> = (0..total).map(|_| None).collect();
    let mut next = 0usize;
    std::future::poll_fn(|cx| {
        loop {
            // Poll everything in flight. Each running future is polled
            // with this task's context, so a pending one has already
            // arranged to wake it — returning `Pending` below parks the
            // task rather than spinning.
            let mut index = 0;
            while index < running.len() {
                match running[index].1.as_mut().poll(cx) {
                    Poll::Ready(value) => {
                        let (slot, _) = running.swap_remove(index);
                        done[slot] = Some(value);
                    }
                    Poll::Pending => index += 1,
                }
            }
            // Fill the freed slots, and poll the newly started futures
            // on the next pass so an instantly-ready one does not wait
            // for a wake nothing will send.
            let mut started = 0;
            while running.len() < bound {
                let Some(work) = queue.pop_front() else { break };
                running.push((next, work));
                next += 1;
                started += 1;
            }
            if started == 0 {
                break;
            }
        }
        if running.is_empty() && queue.is_empty() {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    })
    .await;
    done.into_iter().flatten().collect()
}

/// What a control file's JSON says. `{"steer": "<text>"}` is a steer and
/// `{"stop": true}` is a stop; anything else — an empty steer, a `stop`
/// that is not true, a JSON array — is not a control, and the reader
/// names it rather than guessing.
fn parse_control(value: &serde_json::Value) -> Option<Control> {
    if value.get("stop").and_then(|stop| stop.as_bool()) == Some(true) {
        return Some(Control::Stop);
    }
    let text = value
        .get("steer")
        .and_then(|steer| steer.as_str())
        .map(str::trim)
        .filter(|text| !text.is_empty())?;
    Some(Control::Steer(text.to_string()))
}

/// How long one steer reads before it is cut. A steer is an
/// instruction, not a document; a novel belongs in the task, and an
/// unbounded owner message is an unbounded context cost on every turn
/// that follows.
const STEER_LIMIT: usize = 2_000;

/// The frame a steer rides in: a user-role message labelled as the
/// owner's, never as the model's own words — the same discipline the
/// approver already carries, where a forwarded call is named for the
/// subagent it came from. A model that could not tell its own reasoning
/// from the owner's interruption would read a steer as something it
/// thought of.
fn steer_frame(text: &str) -> String {
    format!(
        "[the owner of this run steered it through its control file \u{2014} this is the owner speaking, not you]\n{text}"
    )
}

/// One steer's text, bounded, with the cut marked so the model knows
/// whether the instruction arrived whole.
fn steer_text(text: &str) -> String {
    let mut bounded: String = text.chars().take(STEER_LIMIT).collect();
    if text.chars().count() > STEER_LIMIT {
        bounded.push('\u{2026}');
    }
    bounded
}

/// How many distinct new identifiers one `write_file` is checked for. A
/// bound, not a proof: a file that introduces more is checked on its
/// first `GUARD_CANDIDATES` names in order of appearance. The guard is a
/// flag, not a certification, and an unbounded scan would stall the
/// write it is meant to police.
const GUARD_CANDIDATES: usize = 512;

/// How many flags one write's note carries before it says how many more
/// there are. A wall of notes is a note nobody reads — the guard that
/// cries wolf gets ignored, and then it is worse than nothing.
const GUARD_NOTES: usize = 3;

/// What the write-time guard decided about one `write_file` call.
#[derive(Debug, Clone, PartialEq, Eq)]
enum WriteGuard {
    /// Nothing the file introduces is new: the write proceeds.
    Clean,
    /// The write proceeds, and the model is told which name it
    /// introduced and what the project already calls it.
    Flagged { note: String },
    /// The write does not happen: the file introduced a case or
    /// separator variant of an established term.
    Refused { reason: String },
}

/// Check the identifiers one `write_file` call introduces against the
/// project's vocabulary, and say what to do about each.
///
/// The three verdicts, and why each is what it is:
///
/// - **Refused** — a case or separator variant of an established term
///   (`judgeRoute` beside `judge_route`). That is drift with no other
///   explanation, so the write does not land and the refusal names both
///   names.
/// - **Flagged** — a novel identifier with a canonical near-neighbour
///   (`route_judge` beside `judge_route`). The write lands and the flag
///   rides the result the model already reads, naming the candidate, so
///   the model can rename.
/// - **Clean** — a genuinely new concept, which the vocabulary does not
///   carry and nothing is near. Not flagged, because a guard that
///   refuses legitimate new names gets ignored.
///
/// Conservative by construction, and cheap by construction: the
/// vocabulary is derived once per process per root and cached inside the
/// nomenclature module, so this reads the cache rather than re-deriving
/// per call, and a name the vocabulary already carries is skipped in one
/// lookup instead of a scan.
fn write_guard(root: &std::path::Path, arguments: &serde_json::Value) -> WriteGuard {
    let Some(content) = arguments.get("content").and_then(|content| content.as_str()) else {
        // No content to name: nothing to check. A write with no content
        // fails on its own, with its own reason.
        return WriteGuard::Clean;
    };
    let vocabulary = crate::nomenclature::vocabulary(root);
    let mut flagged: Vec<(String, String, String)> = Vec::new();
    let mut seen: Vec<String> = Vec::new();
    for token in identifier_tokens(content).into_iter().take(GUARD_CANDIDATES) {
        if seen.contains(&token) {
            continue;
        }
        seen.push(token.clone());
        // The vocabulary's canonical form for this name, in both
        // spellings: the parts joined as written, and the parts
        // lowercased. The vocabulary keeps a compound's own case when
        // the tree wrote it camelCase (`recallCount` establishes
        // `recall_Count`) beside the lowercased one, and a case or
        // separator variant has to find its term in one lookup rather
        // than a scan.
        let parts = identifier_parts(&token);
        let spellings = [
            parts.join("_"),
            parts.iter().map(|part| part.to_lowercase()).collect::<Vec<_>>().join("_"),
        ];
        let mut established = false;
        for spelling in spellings {
            let Some(entry) = vocabulary.get(&spelling) else { continue };
            established = true;
            // Established. The term itself is nothing to flag; a case or
            // separator variant of it is drift with no other
            // explanation, and the module's own verdict is what says so.
            if token != entry.term && crate::nomenclature::is_near_exact(&token, &entry.term) {
                let site = entry
                    .sites
                    .first()
                    .map(|site| format!("{}:{}", site.file, site.line))
                    .unwrap_or_else(|| "in this tree".to_string());
                return WriteGuard::Refused {
                    reason: format!(
                        "error: `write_file` refused: `{token}` is a case or separator variant of the established term `{}` ({site}); rename it to the established term",
                        entry.term
                    ),
                };
            }
            // The term itself: nothing to flag, and the other spelling
            // is the same term.
            break;
        }
        if established {
            continue;
        }
        // Novel. Is there a canonical near-neighbour?
        if let Some(convention) = crate::nomenclature::check_in(&vocabulary, &token) {
            if let Some(canonical) = convention.term {
                let site = convention
                    .sites
                    .first()
                    .map(|site| format!("{}:{}", site.file, site.line))
                    .unwrap_or_else(|| "in this tree".to_string());
                flagged.push((token, canonical, site));
            }
        }
    }
    if flagged.is_empty() {
        return WriteGuard::Clean;
    }
    let mut note = String::from(
        "note: this write introduces names with canonical near-neighbours in this project's vocabulary:",
    );
    for (candidate, canonical, site) in flagged.iter().take(GUARD_NOTES) {
        note.push_str(&format!(
            "\n- `{candidate}` \u{2014} the established term is `{canonical}` ({site}); consider renaming"
        ));
    }
    let more = flagged.len().saturating_sub(GUARD_NOTES);
    if more > 0 {
        note.push_str(&format!("\n- \u{2026}and {more} more"));
    }
    WriteGuard::Flagged { note }
}

/// The identifier-shaped names one file's text carries, in order of
/// first appearance: a maximal run of letters, digits, and underscores
/// that starts with a letter or an underscore — the same definition of a
/// word the vocabulary's own extractor uses. Sub-three-character names
/// are dropped: the vocabulary carries none, so nothing that short can
/// be an established term or a variant of one.
fn identifier_tokens(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut current = String::new();
    for character in text.chars() {
        if character.is_alphanumeric() || character == '_' {
            current.push(character);
        } else if !current.is_empty() {
            if current.chars().count() >= 3 {
                out.push(std::mem::take(&mut current));
            } else {
                current.clear();
            }
        }
    }
    if current.chars().count() >= 3 {
        out.push(current);
    }
    out
}

/// One identifier split on `_`, `-`, and camelCase boundaries, in order:
/// `recall_count` -> recall, count; `recallCount` -> recall, count;
/// `HTTPServer` -> http, server.
///
/// This mirrors the vocabulary's own splitter, which is private to the
/// nomenclature module. The mirror is what lets the guard find a case or
/// separator variant in one lookup — the vocabulary stores its terms in
/// exactly this canonical form — instead of scanning every entry per
/// candidate, which is the cost the module's own bounds exist to avoid.
/// The verdict itself is the module's
/// ([`crate::nomenclature::is_near_exact`]), so a divergence here costs
/// a missed flag, never a wrong refusal.
fn identifier_parts(identifier: &str) -> Vec<String> {
    let mut parts: Vec<String> = Vec::new();
    for segment in identifier.split(['_', '-']) {
        if segment.is_empty() {
            continue;
        }
        let chars: Vec<char> = segment.chars().collect();
        let mut current = String::new();
        for (index, character) in chars.iter().enumerate() {
            let previous = if index == 0 { None } else { Some(chars[index - 1]) };
            let next = chars.get(index + 1).copied();
            let boundary = match (previous, character.is_uppercase()) {
                (Some(previous), true) => {
                    previous.is_lowercase()
                        || previous.is_ascii_digit()
                        || (next.map(|n| n.is_lowercase()).unwrap_or(false)
                            && previous.is_uppercase())
                }
                _ => false,
            };
            if boundary && !current.is_empty() {
                parts.push(std::mem::take(&mut current));
            }
            current.push(*character);
        }
        if !current.is_empty() {
            parts.push(current);
        }
    }
    parts
}

/// Parse a call's arguments for display only.
fn parse_arguments(arguments: &str) -> serde_json::Value {
    serde_json::from_str(arguments).unwrap_or(serde_json::Value::Null)
}

/// One-line summary of what a call does, for prompts and UI events.
fn call_summary(call: &ToolCall) -> String {
    // A subagent's forwarded call: `subagent → <tool>`, carrying the goal
    // beside the call's own arguments. The summary names the goal and the
    // inner call, so an approval prompt answers "whose call is this, and
    // what does it do?" in one line.
    if let Some(inner) = call.function.name.strip_prefix(&tools::subagent_call_prefix()) {
        let arguments = parse_arguments(&call.function.arguments);
        let goal = arguments.get("goal").and_then(|goal| goal.as_str()).unwrap_or_default();
        let inner_call = ToolCall {
            id: call.id.clone(),
            kind: call.kind.clone(),
            function: crate::api::FunctionCall {
                name: inner.to_string(),
                arguments: arguments
                    .get("arguments")
                    .map(|arguments| arguments.to_string())
                    .unwrap_or_default(),
            },
        };
        let inner_summary = call_summary(&inner_call);
        return if goal.is_empty() {
            inner_summary
        } else {
            format!("{goal} \u{b7} {inner_summary}")
        };
    }
    // The parent's own delegation: the goal is the whole summary, because
    // the goal is the fact the owner needs to answer "may this run?".
    if call.function.name == tools::SUBAGENT_TOOL {
        return subagent_goal(&call.function.arguments);
    }
    if call.function.name == tools::IMPROVE_TOOL {
        // The proposal an `improve` call would file: the fact the owner
        // needs to answer "may this run?".
        return improve_proposal(&call.function.arguments);
    }
    if call.function.name == tools::USE_SKILL_TOOL {
        // The skill a `use_skill` call would load: the fact the user
        // needs to answer "may this run?".
        return crate::skills::requested_skill(&call.function.arguments)
            .unwrap_or_else(|| call.function.name.clone());
    }
    // The provenance gate's synthetic call is named
    // `supersede (confirmed-by)`, so the prefix match covers it: the
    // prompt then names the fact, the justification, and — in the
    // call's own name — the confirmed-by basis.
    if call.function.name.starts_with(tools::REMEMBER_TOOL) {
        return memory::remember_summary(&call.function.arguments);
    }
    if call.function.name.starts_with(tools::SUPERSEDE_TOOL) {
        return memory::supersede_summary(&call.function.arguments, None);
    }
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
    if call.function.name == tools::USE_SKILL_TOOL {
        return format!("{}: {}", tools::USE_SKILL_TOOL, call_summary(call));
    }
    if call.function.name == tools::SUBAGENT_TOOL {
        return format!("{}: {}", tools::SUBAGENT_TOOL, subagent_goal(&call.function.arguments));
    }
    if call.function.name == tools::IMPROVE_TOOL {
        return format!("{}: {}", tools::IMPROVE_TOOL, improve_proposal(&call.function.arguments));
    }
    if call.function.name.starts_with(tools::REMEMBER_TOOL)
        || call.function.name.starts_with(tools::SUPERSEDE_TOOL)
        || tools::lane_of(&call.function.name).is_some()
    {
        return format!("{}: {}", call.function.name, call_summary(call));
    }
    let tool = Tool::from_name(&call.function.name);
    let arguments = parse_arguments(&call.function.arguments);
    match tool {
        Some(tool) => format!("{}: {}", tool.name(), tool.summarize(&arguments, 72)),
        None => format!("{}: (unknown tool)", call.function.name),
    }
}

/// One line naming the proposal an `improve` call would file, for an
/// approval prompt and a UI event: the proposal's name and its target,
/// bounded. A missing or malformed name reads as the empty string
/// rather than a guess — the parse that follows names the real refusal.
fn improve_proposal(arguments: &str) -> String {
    let parsed = serde_json::from_str::<serde_json::Value>(arguments).ok();
    let name = parsed
        .as_ref()
        .and_then(|arguments| {
            arguments.get("name").and_then(|name| name.as_str()).map(str::to_string)
        })
        .unwrap_or_default();
    let target = parsed
        .as_ref()
        .and_then(|arguments| {
            arguments.get("target").and_then(|target| target.as_str()).map(str::to_string)
        })
        .unwrap_or_default();
    let mut bounded: String = name.chars().take(48).collect();
    if name.chars().count() > 48 {
        bounded.push('\u{2026}');
    }
    if target.is_empty() {
        bounded
    } else {
        format!("{bounded} ({target})")
    }
}

/// The goal one `subagent` call delegates, bounded for one line. A
/// missing or malformed goal reads as the empty string rather than a
/// guess — the parse that follows names the real refusal.
fn subagent_goal(arguments: &str) -> String {
    let goal = serde_json::from_str::<serde_json::Value>(arguments)
        .ok()
        .and_then(|arguments| {
            arguments.get("goal").and_then(|goal| goal.as_str()).map(str::to_string)
        })
        .unwrap_or_default();
    let mut bounded: String = goal.chars().take(72).collect();
    if goal.chars().count() > 72 {
        bounded.push('\u{2026}');
    }
    bounded
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

    /// A scratch home, unique per call, for a test that needs a session
    /// log on disk.
    fn scratch(tag: &str) -> std::path::PathBuf {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let serial = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "fleet-agent-{tag}-{}-{serial}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn a_run_past_the_skills_soft_cap_still_declares_every_skill() {
        // The cap warns; it never hides. A run carrying more skills than
        // the soft cap declares every one of them in its prompt, and the
        // warning line is the only thing that changes.
        let home = scratch("skills-cap");
        let mut session = crate::session::Session::create(&home, "m").unwrap();
        let client = Client::new("http://127.0.0.1:9", "k");
        let count = SKILLS_PROMPT_SOFT_CAP + 1;
        let skills: Vec<crate::skills::Skill> = (0..count)
            .map(|index| crate::skills::Skill {
                name: format!("skill-{index:02}"),
                description: format!("description {index}"),
                body: "body".to_string(),
                path: std::path::PathBuf::from("/x/SKILL.md"),
                modified: None,
            })
            .collect();
        let options = AgentOptions {
            model: "m".to_string(),
            max_turns: 1,
            ui_tx: None,
            lane_specs: None,
            calib: None,
            skills,
            memory: None,
            scope: None,
            improve_home: None,
        };
        let mut progress = Vec::new();
        let agent =
            Agent::new(&client, options, &mut session, Box::new(ApproveAll), &mut progress)
                .unwrap();
        let prompt = agent.system_prompt(None);
        for index in 0..count {
            assert!(prompt.contains(&format!("skill-{index:02}")), "skill {index} is missing");
        }
        // At the cap and under it, no warning line is built at all.
        assert_eq!(skills_prompt_growth_line(SKILLS_PROMPT_SOFT_CAP), None);
        assert_eq!(skills_prompt_growth_line(0), None);
        let line = skills_prompt_growth_line(count).expect("the warning line");
        assert!(line.contains(&format!("{count} skills declared")), "{line}");
        assert!(line.contains("curation retires unused ones"), "{line}");
        let _ = std::fs::remove_dir_all(&home);
    }

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
    fn a_use_skill_call_summarizes_as_the_skill_it_would_load() {
        // The approval prompt and the progress line name the skill, which
        // is the fact a user needs to answer "may this run?".
        let named = call("c1", "use_skill", r#"{"name":"bash-recovery"}"#);
        assert_eq!(call_summary(&named), "bash-recovery");
        assert!(describe_call(&named).starts_with("use_skill: bash-recovery"));
        // Arguments that name no skill still summarize without a panic.
        let unnamed = call("c2", "use_skill", "{not json");
        assert_eq!(call_summary(&unnamed), "use_skill");
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

    // --- memory ---

    /// A scratch corpus root with `facts` written into it, unique per
    /// call.
    fn corpus(tag: &str, facts: &[(&str, &str)]) -> std::path::PathBuf {
        let dir = scratch(tag);
        for (name, text) in facts {
            std::fs::write(dir.join(format!("{name}.md")), text).unwrap();
        }
        dir
    }

    /// One confirmed-by fact and one inferred fact in the writer's
    /// field order.
    fn confirmed(name: &str, description: &str) -> String {
        format!(
            "---\nname: {name}\ndescription: \"{description}\"\ntype: feedback\ndate: {}\nbasis: confirmed-by\ntier: durable\nshape: []\nrecall-count: 0\nsuperseded-by:\nvalid-until:\n---\n\nThe body.\n",
            crate::calib::today()
        )
    }

    fn inferred(name: &str, description: &str) -> String {
        format!(
            "---\nname: {name}\ndescription: \"{description}\"\ntype: project\ndate: {}\nbasis: inferred\ntier: durable\nshape: []\nrecall-count: 0\nsuperseded-by:\nvalid-until:\n---\n\nThe body.\n",
            crate::calib::today()
        )
    }

    /// An agent over `root`'s corpus, for the memory tests.
    fn agent_over(
        home: &std::path::Path,
        root: &std::path::Path,
        approver: Box<dyn Approver>,
    ) -> Agent<'static> {
        // The client is never called by these tests: they drive
        // `execute_call` directly, the way the loop does.
        let client = Box::leak(Box::new(Client::new("http://127.0.0.1:9", "k")));
        let session = Box::leak(Box::new(
            crate::session::Session::create(home, "m").unwrap(),
        ));
        let memory = Arc::new(Memory::load_with(root, Box::new(crate::memory::KeywordOnly)));
        let options = AgentOptions {
            model: "m".to_string(),
            max_turns: 1,
            ui_tx: None,
            lane_specs: None,
            calib: None,
            skills: Vec::new(),
            memory: Some(memory),
            scope: None,
            improve_home: None,
        };
        Agent::new(client, options, session, approver, Box::leak(Box::new(Vec::new()))).unwrap()
    }

    #[test]
    fn a_run_with_memory_declares_the_write_path_and_one_without_it_does_not() {
        let home = scratch("memory-declarations");
        let root = corpus("memory-declarations-corpus", &[]);
        let agent = agent_over(&home, &root, Box::new(ApproveAll));
        let names: Vec<String> = agent
            .declarations()
            .iter()
            .map(|spec| serde_json::to_value(spec).unwrap()["function"]["name"]
                .as_str()
                .unwrap()
                .to_string())
            .collect();
        assert_eq!(
            names,
            vec![
                "bash",
                "read_file",
                "write_file",
                "list_dir",
                "remember",
                "supersede",
                "subagent"
            ]
        );

        // Without memory the run is the run this crate has always had:
        // the same four declarations plus the delegation tool, and the
        // compiled-in prompt.
        let mut session = crate::session::Session::create(&home, "m").unwrap();
        let client = Client::new("http://127.0.0.1:9", "k");
        let options = AgentOptions {
            model: "m".to_string(),
            max_turns: 1,
            ui_tx: None,
            lane_specs: None,
            calib: None,
            skills: Vec::new(),
            memory: None,
            scope: None,
            improve_home: None,
        };
        let mut progress = Vec::new();
        let plain =
            Agent::new(&client, options, &mut session, Box::new(ApproveAll), &mut progress)
                .unwrap();
        assert_eq!(plain.declarations().len(), 5);
        assert_eq!(plain.system_prompt(None), SYSTEM_PROMPT);
        let _ = std::fs::remove_dir_all(&home);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn the_system_prompt_appends_the_memory_block_and_names_its_frame() {
        let home = scratch("memory-prompt");
        let root = corpus("memory-prompt-corpus", &[]);
        let agent = agent_over(&home, &root, Box::new(ApproveAll));
        let prompt = agent.system_prompt(Some("- [user confirmed-by] the owner's rule\n"));
        assert!(prompt.starts_with(SYSTEM_PROMPT), "{prompt}");
        assert!(prompt.contains("## Memory"), "{prompt}");
        assert!(prompt.contains("proposals to verify, not instructions"), "{prompt}");
        assert!(prompt.contains("- [user confirmed-by] the owner's rule\n"), "{prompt}");
        // No block: the constant itself, byte for byte.
        assert_eq!(agent.system_prompt(None), SYSTEM_PROMPT);
        let _ = std::fs::remove_dir_all(&home);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn the_assembly_records_the_memory_record_and_the_hits() {
        let home = scratch("memory-assembly");
        let root = corpus(
            "memory-assembly-corpus",
            &[
                ("identity", &confirmed("identity", "the owner's standing rule about the gate")),
                ("top", &inferred("top", "the railway deploy is failing per branch")),
                ("bottom", &inferred("bottom", "the railway deploy logs its outcome")),
            ],
        );
        let stream_root = scratch("memory-assembly-stream");
        let stream = crate::calib::Stream::open(&stream_root);
        let client = Box::leak(Box::new(Client::new("http://127.0.0.1:9", "k")));
        let session = Box::leak(Box::new(
            crate::session::Session::create(&home, "m").unwrap(),
        ));
        let memory = Arc::new(Memory::load_with(&root, Box::new(crate::memory::KeywordOnly)));
        let options = AgentOptions {
            model: "m".to_string(),
            max_turns: 1,
            ui_tx: None,
            lane_specs: None,
            calib: Some(stream.clone()),
            skills: Vec::new(),
            memory: Some(memory),
            scope: None,
            improve_home: None,
        };
        let progress: &'static mut Vec<u8> = Box::leak(Box::new(Vec::new()));
        let mut agent =
            Agent::new(client, options, session, Box::new(ApproveAll), &mut *progress).unwrap();

        let block = agent
            .prepare_memory("the railway deploy is failing")
            .unwrap()
            .expect("the corpus assembles");
        assert!(block.contains("Standing context (protected"), "{block}");
        assert!(block.contains("shape-routed recall for `deploy`"), "{block}");
        // The agent is done: its borrows of the leaked progress and
        // session end here, and both are read back below.
        drop(agent);
        // The progress line names the shape and the fact count.
        let written = String::from_utf8(std::mem::take(progress)).unwrap();
        assert!(written.contains("memory: shape deploy"), "{written}");
        assert!(written.contains("3 facts"), "{written}");

        // The `memory` record: the shape and every fact name, in block
        // order — protected tier first.
        let records = crate::session::replay(session.path()).unwrap();
        match records.iter().find(|record| matches!(record, Record::Memory { .. })) {
            Some(Record::Memory { shape, facts }) => {
                assert_eq!(shape, "deploy");
                assert_eq!(facts, &vec!["identity".to_string(), "top".to_string(), "bottom".to_string()]);
            }
            other => panic!("expected a memory record, got {other:?}"),
        }
        // The recall hits: one row per fact that entered, at the score
        // it entered at — and none for the protected tier, which did
        // not compete.
        drop(stream);
        let rows = crate::calib::read_day(&stream_root, &crate::calib::today(), 64);
        let hits: Vec<&str> = rows
            .iter()
            .filter(|row| row.op == crate::calib::OP_RECALL_HIT)
            .map(|row| row.fact.as_str())
            .collect();
        assert_eq!(hits, vec!["top", "bottom"]);
        let _ = std::fs::remove_dir_all(&home);
        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&stream_root);
    }

    #[tokio::test]
    async fn an_empty_assembly_records_nothing_and_adds_no_block() {
        let home = scratch("memory-empty");
        // A corpus with nothing that can match: one unrelated fact.
        let root = corpus(
            "memory-empty-corpus",
            &[("walnut", &inferred("walnut", "the walnut table texture"))],
        );
        let mut agent = agent_over(&home, &root, Box::new(ApproveAll));
        let block = agent.prepare_memory("the railway deploy is failing").unwrap();
        assert_eq!(block, None);
        // No record, no rows: the run is byte-for-byte unchanged.
        let records = crate::session::replay(
            &crate::home::sessions_dir(&home).join(
                std::fs::read_dir(crate::home::sessions_dir(&home))
                    .unwrap()
                    .flatten()
                    .next()
                    .unwrap()
                    .file_name(),
            ),
        )
        .unwrap();
        assert!(
            !records.iter().any(|record| matches!(record, Record::Memory { .. })),
            "{records:?}"
        );
        let _ = std::fs::remove_dir_all(&home);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_remember_call_summarizes_as_the_fact_and_its_basis() {
        let named = call(
            "c1",
            "remember",
            r#"{"name":"the-gate-is-local","description":"the gate is local","basis":"confirmed-by","tier":"working"}"#,
        );
        assert_eq!(
            call_summary(&named),
            "remember the-gate-is-local (confirmed-by working)"
        );
        assert!(describe_call(&named).starts_with("remember: remember the-gate-is-local"));
        // Arguments that name no fact still summarize without a panic.
        let unnamed = call("c2", "remember", "{not json");
        assert_eq!(call_summary(&unnamed), "remember");
        let empty = call("c3", "remember", r#"{"description":"no name"}"#);
        assert_eq!(call_summary(&empty), "remember");
    }

    #[test]
    fn a_supersede_call_summarizes_as_the_fact_and_the_justification() {
        let named = call(
            "c1",
            "supersede",
            r#"{"name":"a-guess","justification":"the deploy moved to per-branch builds"}"#,
        );
        assert_eq!(
            call_summary(&named),
            "supersede a-guess \u{2014} the deploy moved to per-branch builds"
        );
        assert!(describe_call(&named).starts_with("supersede: supersede a-guess"));
        // The provenance gate's synthetic call names the basis in its
        // own name, and its summary still names the fact.
        let provenance = call(
            "c1-provenance",
            "supersede (confirmed-by)",
            r#"{"name":"load-bearing","justification":"the owner restated it"}"#,
        );
        assert_eq!(
            call_summary(&provenance),
            "supersede load-bearing \u{2014} the owner restated it"
        );
        let unnamed = call("c2", "supersede", "{not json");
        assert_eq!(call_summary(&unnamed), "supersede");
    }

    /// An approver that answers from a script: one boolean per call, in
    /// order. The shape a test needs to separate the ordinary gate's
    /// answer from the provenance gate's.
    struct Scripted {
        answers: std::sync::Arc<std::sync::Mutex<std::collections::VecDeque<bool>>>,
    }

    impl Scripted {
        fn new(answers: [bool; 2]) -> Scripted {
            Scripted {
                answers: std::sync::Arc::new(std::sync::Mutex::new(
                    std::collections::VecDeque::from(answers),
                )),
            }
        }
    }

    impl Approver for Scripted {
        fn approve(&mut self, _call: &ToolCall) -> bool {
            self.answers.lock().unwrap().pop_front().unwrap_or(false)
        }
    }

    #[tokio::test]
    async fn a_confirmed_by_supersede_needs_the_provenance_approval_both_ways() {
        let home = scratch("provenance-gate");
        let root = corpus(
            "provenance-gate-corpus",
            &[("load-bearing", &confirmed("load-bearing", "the owner's standing rule about the gate"))],
        );
        let arguments = r#"{"name":"load-bearing","justification":"the owner restated the rule","replacement":"load-bearing-v2","description":"the owner's new standing rule about the gate"}"#;

        // Refused: the tool call is approved, the provenance
        // confirmation is not. Nothing moves.
        let mut agent = agent_over(&home, &root, Box::new(Scripted::new([true, false])));
        agent.execute_call(&call("c1", "supersede", arguments)).await.unwrap();
        assert!(!root.join("load-bearing-v2.md").exists(), "no replacement was written");
        let text = std::fs::read_to_string(root.join("load-bearing.md")).unwrap();
        assert!(!text.contains("superseded-by: load-bearing-v2"), "{text}");

        // Allowed: both gates approve. The replacement lands and the
        // old fact is folded into it.
        let mut agent = agent_over(&home, &root, Box::new(Scripted::new([true, true])));
        agent.execute_call(&call("c2", "supersede", arguments)).await.unwrap();
        assert!(root.join("load-bearing-v2.md").exists(), "the replacement was written");
        let old = std::fs::read_to_string(root.join("load-bearing.md")).unwrap();
        assert!(old.contains("superseded-by: load-bearing-v2"), "{old}");
        let _ = std::fs::remove_dir_all(&home);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn an_inferred_supersede_lands_on_the_ordinary_gate_alone() {
        let home = scratch("provenance-inferred");
        let root = corpus(
            "provenance-inferred-corpus",
            &[("a-guess", &inferred("a-guess", "an inferred guess about the deploy"))],
        );
        // One answer is scripted and only one is asked: the inferred
        // tier needs no provenance confirmation.
        let mut agent = agent_over(&home, &root, Box::new(Scripted::new([true, false])));
        let arguments = r#"{"name":"a-guess","justification":"the deploy moved to per-branch builds","replacement":"a-better-guess","description":"an inferred fact about the per-branch deploy"}"#;
        agent.execute_call(&call("c1", "supersede", arguments)).await.unwrap();
        assert!(root.join("a-better-guess.md").exists());
        let replacement = std::fs::read_to_string(root.join("a-better-guess.md")).unwrap();
        assert!(
            replacement.contains("Supersedes `a-guess` \u{2014} the deploy moved to per-branch builds"),
            "{replacement}"
        );
        // A justification-free supersession is a model-visible error,
        // not a write.
        let mut agent = agent_over(&home, &root, Box::new(ApproveAll));
        agent
            .execute_call(&call("c2", "supersede", r#"{"name":"a-guess","justification":""}"#))
            .await
            .unwrap();
        assert!(std::fs::read_to_string(root.join("a-guess.md"))
            .unwrap()
            .contains("valid-until:\n"));
        let _ = std::fs::remove_dir_all(&home);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn a_remember_call_writes_the_fact_the_loader_reads_back() {
        let home = scratch("remember-tool");
        let root = corpus("remember-tool-corpus", &[]);
        let mut agent = agent_over(&home, &root, Box::new(ApproveAll));
        let arguments = r#"{"name":"a-new-fact","description":"a fact the model just learned","basis":"inferred","tier":"working","valid-until":"2099-01-01","body":"The full content."}"#;
        agent.execute_call(&call("c1", "remember", arguments)).await.unwrap();
        let loaded = crate::memory::load_facts(&root);
        assert_eq!(loaded.len(), 1);
        let fact = &loaded[0];
        assert_eq!(fact.name, "a-new-fact");
        assert_eq!(fact.description, "a fact the model just learned");
        assert_eq!(fact.basis, crate::memory::Basis::Inferred);
        assert_eq!(fact.tier, crate::memory::Tier::Working);
        assert_eq!(fact.valid_until, "2099-01-01");
        assert_eq!(fact.body, "The full content.");
        // The tool's result rode the log, so the model's own write is
        // reconstructable.
        let records = crate::session::replay(&session_path_of(&home)).unwrap();
        let result = records
            .iter()
            .find(|record| matches!(record, Record::ToolResult { name, .. } if name == "remember"))
            .expect("the remember tool_result");
        let Record::ToolResult { output, approved, .. } = result else { unreachable!() };
        assert!(approved);
        assert!(output.contains("remembered `a-new-fact`"), "{output}");
        // A write without its basis is a model-visible error, not a
        // file.
        agent
            .execute_call(&call(
                "c2",
                "remember",
                r#"{"name":"no-basis","description":"a fact with no provenance"}"#,
            ))
            .await
            .unwrap();
        assert_eq!(crate::memory::load_facts(&root).len(), 1);
        let _ = std::fs::remove_dir_all(&home);
        let _ = std::fs::remove_dir_all(&root);
    }

    /// The one session log under `home`, for a result assertion.
    fn session_path_of(home: &std::path::Path) -> std::path::PathBuf {
        crate::home::sessions_dir(home)
            .join(
                std::fs::read_dir(crate::home::sessions_dir(home))
                    .unwrap()
                    .flatten()
                    .next()
                    .unwrap()
                    .file_name(),
            )
    }

    // --- compaction -------------------------------------------------------

    /// An agent over a session log at `path`, for the bookkeeping tests.
    /// Its client is never called.
    fn agent_over_log(path: &std::path::Path) -> Agent<'static> {
        let client = Box::leak(Box::new(Client::new("http://127.0.0.1:9", "k")));
        let session = Box::leak(Box::new(crate::session::Session::open(path).unwrap()));
        let options = AgentOptions {
            model: "m".to_string(),
            max_turns: 1,
            ui_tx: None,
            lane_specs: None,
            calib: None,
            skills: Vec::new(),
            memory: None,
            scope: None,
            improve_home: None,
        };
        let progress: &'static mut Vec<u8> = Box::leak(Box::new(Vec::new()));
        Agent::new(client, options, session, Box::new(ApproveAll), progress).unwrap()
    }

    #[tokio::test]
    async fn a_resumed_agent_numbers_its_turns_from_the_logs_own() {
        // A log with no compaction: the seeded conversation's turn
        // numbering is the one the log itself implies, so a resumed run's
        // turns continue rather than restart.
        let home = scratch("compaction-turns");
        let mut written = crate::session::Session::create(&home, "m").unwrap();
        written.record(&Record::User { content: "the task".into() }).unwrap();
        written
            .record(&Record::Assistant {
                content: Some("first".into()),
                tool_calls: Vec::new(),
            })
            .unwrap();
        written
            .record(&Record::Assistant {
                content: Some("second".into()),
                tool_calls: Vec::new(),
            })
            .unwrap();
        let path = written.path().to_path_buf();

        let mut agent = agent_over_log(&path);
        assert_eq!(agent.next_turn, 3, "two answers have landed");
        assert_eq!(agent.message_turns, vec![1, 1, 2]);
        // The next push takes turn 3, and an assistant answer advances it.
        agent.push_message(Message::text(Role::User, "keep going")).unwrap();
        assert_eq!(agent.message_turns.last().copied(), Some(3));
        assert_eq!(agent.next_turn, 3);
        agent
            .push_message(Message {
                role: Role::Assistant,
                content: Some("third".into()),
                tool_calls: Vec::new(),
                tool_call_id: None,
            })
            .unwrap();
        assert_eq!(agent.message_turns.last().copied(), Some(3));
        assert_eq!(agent.next_turn, 4);
        std::fs::remove_dir_all(&home).ok();
    }

    #[tokio::test]
    async fn a_compacted_log_seeds_the_compacted_shape_and_its_numbering() {
        // The log a fold leaves behind replays to the framed summary plus
        // the verbatim tail, and the turns continue from the log's own
        // last answer — which is what a second compaction's range names.
        let home = scratch("compaction-seed");
        let mut written = crate::session::Session::create(&home, "m").unwrap();
        written.record(&Record::User { content: "the task".into() }).unwrap();
        for index in 0..6 {
            written
                .record(&Record::Assistant {
                    content: None,
                    tool_calls: vec![crate::api::ToolCall {
                        id: format!("call-{index}"),
                        kind: "function".to_string(),
                        function: crate::api::FunctionCall {
                            name: "bash".to_string(),
                            arguments: "{}".to_string(),
                        },
                    }],
                })
                .unwrap();
            written
                .record(&Record::ToolResult {
                    call_id: format!("call-{index}"),
                    name: "bash".to_string(),
                    output: format!("output {index}"),
                    truncated: false,
                    approved: true,
                })
                .unwrap();
        }
        written
            .record(&Record::Compaction {
                summary: "- the summary".to_string(),
                first_turn: 1,
                last_turn: 2,
                tokens_before: 120_000,
                tokens_after: 900,
            })
            .unwrap();
        let path = written.path().to_path_buf();

        let agent = agent_over_log(&path);
        // The compacted shape: the framed summary, then the tail.
        assert_eq!(agent.messages.len(), 1 + 4 * 2, "{:?}", agent.messages);
        assert_eq!(
            agent.messages[0].content.as_deref(),
            Some("Summary of turns 1 through 2, compacted to fit the window:\n\n- the summary")
        );
        assert_eq!(agent.messages[1].tool_calls[0].id, "call-2");
        // Turn 6 was the last answer, so the next request is turn 7.
        assert_eq!(agent.next_turn, 7);
        assert_eq!(agent.message_turns.len(), agent.messages.len());
        assert_eq!(agent.message_turns[0], 1, "the summary carries the range's first turn");
        std::fs::remove_dir_all(&home).ok();
    }

    // --- the bounded fan-out (D4) -----------------------------------------

    /// The prepared form of one built-in call, for the plan tests: the
    /// gate is not what those tests are about, so the call is prepared
    /// by hand the way the gate would have.
    fn ready_builtin(call: &ToolCall) -> Prepared {
        let tool = Tool::from_name(&call.function.name).expect("a built-in tool");
        let arguments = serde_json::from_str(&call.function.arguments).unwrap();
        Prepared::Ready(Work::Builtin { tool, arguments, root: std::path::PathBuf::from("/root") })
    }

    /// The prepared form of a whole batch of built-in calls, in order.
    fn prepared_of(calls: &[ToolCall]) -> Vec<Option<Prepared>> {
        calls.iter().map(|call| Some(ready_builtin(call))).collect()
    }

    /// How many calls each wave of a plan carries, in wave order.
    fn wave_sizes(plan: &BatchPlan) -> Vec<usize> {
        let mut sizes = vec![0usize; plan.waves];
        for step in &plan.steps {
            sizes[step.wave] += 1;
        }
        sizes
    }

    #[test]
    fn the_config_seam_can_only_lower_the_bound() {
        // Absent, malformed, zero, or above the ceiling: the constant
        // stands. A bound that silently becomes one because of a typo is
        // a bound nobody asked for.
        for requested in [None, Some(""), Some("  "), Some("abc"), Some("0"), Some("7"), Some("3")] {
            assert_eq!(
                clamp_concurrency(DELEGATION_CONCURRENCY, requested),
                DELEGATION_CONCURRENCY,
                "{requested:?} must leave the constant standing"
            );
        }
        // A smaller positive number lowers it, and that is the only
        // direction the seam moves.
        assert_eq!(clamp_concurrency(DELEGATION_CONCURRENCY, Some("1")), 1);
        assert_eq!(clamp_concurrency(DELEGATION_CONCURRENCY, Some(" 2 ")), 2);
        assert_eq!(DELEGATION_CONCURRENCY, 3, "the default the plan names");
    }

    #[test]
    fn a_batch_larger_than_the_bound_never_exceeds_it() {
        // Seven disjoint reads under a bound of three: three waves, none
        // larger than the bound, whatever the batch's size.
        let calls: Vec<ToolCall> = (0..7)
            .map(|index| {
                call(&format!("c{index}"), "read_file", &format!(r#"{{"path":"/f{index}"}}"#))
            })
            .collect();
        let plan = plan_batch(&prepared_of(&calls), DELEGATION_CONCURRENCY);
        assert_eq!(plan.waves, 3, "{:?}", wave_sizes(&plan));
        assert!(
            wave_sizes(&plan).iter().all(|size| *size <= DELEGATION_CONCURRENCY),
            "{:?}",
            wave_sizes(&plan)
        );
        // The seam's lowered bound is honored: seven calls under two.
        let tight = plan_batch(&prepared_of(&calls), clamp_concurrency(3, Some("2")));
        assert_eq!(tight.bound, 2);
        assert_eq!(tight.waves, 4, "{:?}", wave_sizes(&tight));
        assert!(wave_sizes(&tight).iter().all(|size| *size <= 2), "{:?}", wave_sizes(&tight));
    }

    #[test]
    fn disjoint_calls_run_together() {
        // Three writes to three paths and two reads: the independent
        // calls share a wave, and only the bound pushes the rest into the
        // next one.
        let calls = vec![
            call("c1", "write_file", r#"{"path":"/a","content":"x"}"#),
            call("c2", "write_file", r#"{"path":"/b","content":"x"}"#),
            call("c3", "write_file", r#"{"path":"/c","content":"x"}"#),
            call("c4", "read_file", r#"{"path":"/d"}"#),
            call("c5", "read_file", r#"{"path":"/e"}"#),
        ];
        let plan = plan_batch(&prepared_of(&calls), DELEGATION_CONCURRENCY);
        assert_eq!(plan.steps[0].wave, 0);
        assert_eq!(plan.steps[1].wave, 0);
        assert_eq!(plan.steps[2].wave, 0);
        assert_eq!(plan.steps[3].wave, 1);
        assert_eq!(plan.steps[4].wave, 1);
        assert!(plan.conflicts.is_empty(), "{:?}", plan.conflicts);
    }

    #[test]
    fn two_writes_to_one_path_run_serially_and_the_conflict_is_named() {
        let calls = vec![
            call("c1", "write_file", r#"{"path":"/same","content":"first"}"#),
            call("c2", "read_file", r#"{"path":"/other"}"#),
            call("c3", "write_file", r#"{"path":"/same","content":"second"}"#),
        ];
        let plan = plan_batch(&prepared_of(&calls), DELEGATION_CONCURRENCY);
        // The two writes to one path are held apart: the second runs
        // after the first, in the model's own order — never silently
        // reordered, and never together.
        assert_eq!(plan.steps[0].wave, 0);
        assert_eq!(plan.steps[2].wave, 1, "the second write waits for the first");
        // The conflict is refused by name: the writer, and both calls in
        // the batch's order.
        assert_eq!(plan.conflicts.len(), 1);
        assert_eq!(plan.conflicts[0].writer, "path:/same");
        assert_eq!(plan.conflicts[0].positions, vec![0, 2]);
        let line = plan.line(&calls).expect("a batch of three names itself");
        assert!(line.contains("path:/same"), "{line}");
        assert!(line.contains("c1 \u{2192} c3"), "{line}");
        assert!(line.contains("run serially"), "{line}");
    }

    #[test]
    fn the_memory_writes_serialize_against_each_other() {
        // `remember` and `supersede` both mutate the corpus: two
        // concurrent writes to one corpus is the lost-update class.
        let root = corpus("plan-memory-corpus", &[]);
        let memory = Arc::new(Memory::load_with(&root, Box::new(crate::memory::KeywordOnly)));
        let request =
            memory::Supersede::parse(&serde_json::json!({"name":"b","justification":"j"})).unwrap();
        let prepared = vec![
            Some(Prepared::Ready(Work::Remember {
                memory: memory.clone(),
                arguments: serde_json::json!({"name":"a"}),
            })),
            Some(Prepared::Ready(Work::Supersede {
                memory,
                calib: None,
                request,
                approved: true,
            })),
        ];
        let plan = plan_batch(&prepared, DELEGATION_CONCURRENCY);
        assert_ne!(plan.steps[0].wave, plan.steps[1].wave);
        assert_eq!(plan.conflicts.len(), 1);
        assert_eq!(plan.conflicts[0].writer, "memory");
        assert_eq!(plan.conflicts[0].positions, vec![0, 1]);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn two_shell_calls_take_the_one_writer() {
        // A shell command can write anything, so two of them are two
        // writers on the whole tree — the KI-172/173/177 class.
        let calls = vec![
            call("c1", "bash", r#"{"command":"ls"}"#),
            call("c2", "bash", r#"{"command":"pwd"}"#),
        ];
        let plan = plan_batch(&prepared_of(&calls), DELEGATION_CONCURRENCY);
        assert_ne!(plan.steps[0].wave, plan.steps[1].wave);
        assert_eq!(plan.conflicts[0].writer, "shell");
    }

    #[tokio::test]
    async fn the_bounded_runner_never_exceeds_the_bound() {
        // Seven works, each recording how many were in flight when it
        // ran: the peak is the bound and nothing above it, and every work
        // answers in the order it was given.
        let in_flight = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let peak = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let works: Vec<_> = (0..7)
            .map(|index| {
                let in_flight = in_flight.clone();
                let peak = peak.clone();
                async move {
                    let now = in_flight.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
                    peak.fetch_max(now, std::sync::atomic::Ordering::SeqCst);
                    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
                    in_flight.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
                    index
                }
            })
            .collect();
        let answers = run_bounded(works, DELEGATION_CONCURRENCY).await;
        assert_eq!(answers, (0..7).collect::<Vec<_>>(), "the answers keep their order");
        assert!(
            peak.load(std::sync::atomic::Ordering::SeqCst) <= DELEGATION_CONCURRENCY,
            "peak in flight was {}",
            peak.load(std::sync::atomic::Ordering::SeqCst)
        );
        // A lowered bound is the runner's too.
        let works: Vec<_> = (0..5).map(|index| async move {
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
            index
        }).collect();
        assert_eq!(run_bounded(works, 1).await, (0..5).collect::<Vec<_>>());
    }

    /// A progress sink that shares its bytes with the test, so a test can
    /// read the lines the loop wrote once the agent is done with them.
    #[derive(Clone, Default)]
    struct SharedProgress(std::sync::Arc<std::sync::Mutex<Vec<u8>>>);

    impl std::io::Write for SharedProgress {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl SharedProgress {
        fn text(&self) -> String {
            String::from_utf8_lossy(&self.0.lock().unwrap()).to_string()
        }
    }

    /// An agent over a fresh session under `home` with a shared progress
    /// sink, for the batch tests. Its client is never called: they drive
    /// `execute_calls` directly, the way the loop does.
    fn agent_over_home<'a>(
        home: &std::path::Path,
        approver: Box<dyn Approver>,
        progress: &'a mut dyn Write,
    ) -> Agent<'a> {
        let client = Box::leak(Box::new(Client::new("http://127.0.0.1:9", "k")));
        let session = Box::leak(Box::new(crate::session::Session::create(home, "m").unwrap()));
        let options = AgentOptions {
            model: "m".to_string(),
            max_turns: 1,
            ui_tx: None,
            lane_specs: None,
            calib: None,
            skills: Vec::new(),
            memory: None,
            scope: None,
            improve_home: None,
        };
        Agent::new(client, options, session, approver, progress).unwrap()
    }

    /// The call ids a log's tool results carry, in the order they landed.
    fn result_ids(records: &[Record]) -> Vec<String> {
        records
            .iter()
            .filter_map(|record| match record {
                Record::ToolResult { call_id, .. } => Some(call_id.clone()),
                _ => None,
            })
            .collect()
    }

    #[tokio::test]
    async fn a_batch_of_independent_writes_executes_and_every_outcome_is_logged() {
        let home = scratch("batch-exec");
        let files = scratch("batch-exec-files");
        let mut progress = SharedProgress::default();
        let mut agent = agent_over_home(&home, Box::new(ApproveAll), &mut progress);
        let calls: Vec<ToolCall> = (0..5)
            .map(|index| {
                call(
                    &format!("c{index}"),
                    "write_file",
                    &format!(
                        r#"{{"path":"{}","content":"aaa"}}"#,
                        files.join(format!("f{index}.txt")).display()
                    ),
                )
            })
            .collect();
        // The assistant turn lands first, exactly as the loop records it
        // before the batch runs: the batch is what that record carries.
        agent
            .push_message(Message {
                role: Role::Assistant,
                content: None,
                tool_calls: calls.clone(),
                tool_call_id: None,
            })
            .unwrap();
        agent.execute_calls(&calls).await.unwrap();
        drop(agent);
        // Every file landed: the calls ran, together or in waves, and
        // none was dropped.
        for index in 0..5 {
            let written = std::fs::read_to_string(files.join(format!("f{index}.txt"))).unwrap();
            assert_eq!(written, "aaa");
        }
        // The log records the batch — the assistant turn carries every
        // call, in the model's order — and each call's outcome, in that
        // same order. The concurrency is reconstructable from the log
        // alone (hard rule 5).
        let records = crate::session::replay(&session_path_of(&home)).unwrap();
        let batch = records
            .iter()
            .find_map(|record| match record {
                Record::Assistant { tool_calls, .. } => Some(tool_calls.clone()),
                _ => None,
            })
            .expect("the assistant turn carries the batch");
        let batch_ids: Vec<String> = batch.iter().map(|entry| entry.id.clone()).collect();
        let call_ids: Vec<String> = calls.iter().map(|entry| entry.id.clone()).collect();
        assert_eq!(batch_ids, call_ids);
        assert_eq!(result_ids(&records), call_ids, "every outcome, in the batch's order");
        let approved: Vec<bool> = records
            .iter()
            .filter_map(|record| match record {
                Record::ToolResult { approved, .. } => Some(*approved),
                _ => None,
            })
            .collect();
        assert_eq!(approved, vec![true; 5]);
        // The plan named itself on the progress line: the bound, the
        // waves, and — here — no conflict at all.
        let written = progress.text();
        assert!(written.contains("batch: 5 calls in 2 waves (bound 3)"), "{written}");
        assert!(!written.contains("one writer"), "{written}");
        let _ = std::fs::remove_dir_all(&home);
        let _ = std::fs::remove_dir_all(&files);
    }

    #[tokio::test]
    async fn two_writes_to_one_path_run_in_order_through_the_loop() {
        let home = scratch("serial-exec");
        let files = scratch("serial-exec-files");
        let mut progress = SharedProgress::default();
        let mut agent = agent_over_home(&home, Box::new(ApproveAll), &mut progress);
        let path = files.join("one.txt");
        let calls = vec![
            call(
                "c1",
                "write_file",
                &format!(r#"{{"path":"{}","content":"aaa"}}"#, path.display()),
            ),
            call(
                "c2",
                "write_file",
                &format!(r#"{{"path":"{}","content":"bbb"}}"#, path.display()),
            ),
        ];
        agent.execute_calls(&calls).await.unwrap();
        drop(agent);
        // Both writes landed, and the second is the one that stands: the
        // model's order was kept, not silently reordered.
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "bbb");
        let records = crate::session::replay(&session_path_of(&home)).unwrap();
        assert_eq!(result_ids(&records), vec!["c1".to_string(), "c2".to_string()]);
        // And the conflict was named rather than reordered silently.
        let written = progress.text();
        assert!(written.contains("one writer `path:"), "{written}");
        assert!(written.contains("c1 \u{2192} c2"), "{written}");
        assert!(written.contains("run serially"), "{written}");
        let _ = std::fs::remove_dir_all(&home);
        let _ = std::fs::remove_dir_all(&files);
    }

    #[tokio::test]
    async fn a_denied_call_in_a_batch_still_lands_its_denial() {
        // The gate is crossed one call at a time even in a concurrent
        // batch: a denial settles at the gate, the call never runs, and
        // its result still rides the log.
        let home = scratch("batch-denial");
        let files = scratch("batch-denial-files");
        let mut progress = SharedProgress::default();
        let mut agent = agent_over_home(&home, Box::new(Scripted::new([true, false])), &mut progress);
        let calls = vec![
            call(
                "c1",
                "write_file",
                &format!(r#"{{"path":"{}","content":"aaa"}}"#, files.join("kept.txt").display()),
            ),
            call(
                "c2",
                "write_file",
                &format!(r#"{{"path":"{}","content":"bbb"}}"#, files.join("denied.txt").display()),
            ),
        ];
        agent.execute_calls(&calls).await.unwrap();
        drop(agent);
        assert!(files.join("kept.txt").exists(), "the approved call ran");
        assert!(!files.join("denied.txt").exists(), "the denied call did not run");
        let records = crate::session::replay(&session_path_of(&home)).unwrap();
        let denied = records
            .iter()
            .find_map(|record| match record {
                Record::ToolResult { call_id, approved, output, .. } if call_id == "c2" => {
                    Some((*approved, output.clone()))
                }
                _ => None,
            })
            .expect("the denied call's result landed");
        assert!(!denied.0, "the log carries the gate's own answer");
        assert!(denied.1.contains("did not approve"), "{}", denied.1);
        let _ = std::fs::remove_dir_all(&home);
        let _ = std::fs::remove_dir_all(&files);
    }

    // --- live control (D5) -------------------------------------------------

    #[tokio::test]
    async fn an_absent_control_file_costs_nothing() {
        let home = scratch("control-absent");
        let mut progress = SharedProgress::default();
        let mut agent = agent_over_home(&home, Box::new(ApproveAll), &mut progress);
        assert!(!agent.control_path().exists());
        // Reading an absent file is one failed open: nothing, no line,
        // no file created.
        assert_eq!(agent.read_control(), None);
        assert_eq!(agent.read_control(), None);
        drop(agent);
        assert!(progress.text().is_empty(), "{:?}", progress.text());
        let _ = std::fs::remove_dir_all(&home);
    }

    #[tokio::test]
    async fn a_steer_rides_the_next_turn_as_the_owners_message_and_is_consumed() {
        let home = scratch("control-steer");
        let mut progress = SharedProgress::default();
        let mut agent = agent_over_home(&home, Box::new(ApproveAll), &mut progress);
        let control = agent.control_path();
        std::fs::write(&control, r#"{"steer":"check the gate first"}"#).unwrap();
        // Read once: the steer rides, and the file is gone — one-shot, so
        // it cannot ride two turns.
        assert_eq!(
            agent.read_control(),
            Some(Control::Steer("check the gate first".to_string()))
        );
        assert!(!control.exists(), "the steer is consumed on read");
        // A second read finds nothing.
        assert_eq!(agent.read_control(), None);
        // It rides the next turn as a user-role message labelled as the
        // owner's, never as the model's own words.
        agent.apply_steer("check the gate first").unwrap();
        let last = agent.messages.last().expect("the steer rode");
        assert_eq!(last.role, Role::User);
        let content = last.content.clone().unwrap_or_default();
        assert!(content.contains("the owner speaking, not you"), "{content}");
        assert!(content.contains("check the gate first"), "{content}");
        drop(agent);
        // And it is logged like every other model-visible message, so a
        // steered run's trajectory is reconstructable (hard rule 5).
        let records = crate::session::replay(&session_path_of(&home)).unwrap();
        let steered = records
            .iter()
            .find(|record| matches!(record, Record::User { content } if content.contains("steered")));
        match steered {
            Some(Record::User { content }) => {
                assert!(content.contains("check the gate first"), "{content}");
            }
            other => panic!("expected the steered user record, got {other:?}"),
        }
        let _ = std::fs::remove_dir_all(&home);
    }

    #[tokio::test]
    async fn a_malformed_control_file_is_named_and_left_in_place() {
        let home = scratch("control-malformed");
        let mut progress = SharedProgress::default();
        let mut agent = agent_over_home(&home, Box::new(ApproveAll), &mut progress);
        let control = agent.control_path();
        std::fs::write(&control, "{not json").unwrap();
        assert_eq!(agent.read_control(), None);
        // A malformed file is evidence of something, not litter: it stays,
        // and one line names it (hard rule 10).
        assert!(control.exists(), "the malformed file is left in place");
        // A file naming neither a steer nor a stop is named too.
        std::fs::write(&control, r#"{"halt":true}"#).unwrap();
        assert_eq!(agent.read_control(), None);
        drop(agent);
        let written = progress.text();
        assert!(written.contains("is not valid JSON"), "{written}");
        assert!(written.contains("names no steer and no stop"), "{written}");
        let _ = std::fs::remove_dir_all(&home);
    }

    #[tokio::test]
    async fn a_stop_lands_the_partial_result_with_the_typed_reason() {
        let home = scratch("control-stop");
        let mut progress = SharedProgress::default();
        let mut agent = agent_over_home(&home, Box::new(ApproveAll), &mut progress);
        // The owner writes the control file before the run's first model
        // call: the stop lands at turn one, so the client is never asked
        // and no token is spent after it.
        std::fs::write(agent.control_path(), r#"{"stop": true}"#).unwrap();
        let error = agent.run("the goal").await.unwrap_err();
        // The typed reason, verbatim — the one thing the error must not do
        // is be vague. It is what the delegation path lands as the
        // stopped child's reason, beside the partial result it replays
        // from the child's own log and the log path itself.
        assert_eq!(error.to_string(), "stopped by the owner");
        // The run's own log names who stopped it, so the stop is
        // reconstructable rather than silent.
        let records = crate::session::replay(&session_path_of(&home)).unwrap();
        assert!(
            matches!(records.last(), Some(Record::End { stop_reason, .. }) if stop_reason == "stopped by the owner"),
            "{records:?}"
        );
        drop(agent);
        assert!(progress.text().contains("stopped by the owner"), "{:?}", progress.text());
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn a_steer_is_bounded_and_its_frame_names_the_owner() {
        // A steer is an instruction, not a document.
        assert_eq!(steer_text("short"), "short");
        let long = "x".repeat(STEER_LIMIT + 10);
        let bounded = steer_text(&long);
        assert_eq!(bounded.chars().count(), STEER_LIMIT + 1);
        assert!(bounded.ends_with('\u{2026}'), "{bounded}");
        let framed = steer_frame("look at the gate");
        assert!(framed.contains("the owner speaking, not you"), "{framed}");
        assert!(framed.ends_with("look at the gate"), "{framed}");
    }

    #[test]
    fn the_control_file_shapes_parse_and_nothing_else_does() {
        assert_eq!(parse_control(&serde_json::json!({"stop": true})), Some(Control::Stop));
        assert_eq!(
            parse_control(&serde_json::json!({"steer": "go left"})),
            Some(Control::Steer("go left".to_string()))
        );
        assert_eq!(parse_control(&serde_json::json!({"steer": "  go left  "})), Some(Control::Steer("go left".to_string())));
        for not_a_control in [
            serde_json::json!({}),
            serde_json::json!({"stop": false}),
            serde_json::json!({"steer": ""}),
            serde_json::json!({"steer": 7}),
            serde_json::json!([1, 2]),
            serde_json::json!("stop"),
        ] {
            assert_eq!(parse_control(&not_a_control), None, "{not_a_control}");
        }
    }

    // --- the write-time guard (D7) -----------------------------------------

    /// A scratch project root whose `src/` establishes `judge_route` (and
    /// its parts) in the identifier channel, for the guard's tests. The
    /// vocabulary derives from the tree, so the tree is the fixture.
    fn vocabulary_root(tag: &str) -> std::path::PathBuf {
        let root = scratch(tag);
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(
            root.join("src/lib.rs"),
            "pub fn judge_route() -> u8 { 1 }\npub fn judge_route_again() -> u8 { judge_route() }\n",
        )
        .unwrap();
        root
    }

    #[test]
    fn a_near_exact_variant_of_an_established_term_is_refused() {
        // `judgeRoute` beside an established `judge_route` is a case
        // variant with no other explanation: drift, refused, with both
        // names named.
        let root = vocabulary_root("guard-refuse");
        let guard = write_guard(
            &root,
            &serde_json::json!({"path":"/x","content":"let judgeRoute = judge_route();"}),
        );
        match guard {
            WriteGuard::Refused { reason } => {
                assert!(reason.contains("judgeRoute"), "the candidate is named: {reason}");
                assert!(reason.contains("judge_route"), "the canonical term is named: {reason}");
                assert!(reason.contains("src/lib.rs"), "the evidence rides: {reason}");
            }
            other => panic!("expected a refusal, got {other:?}"),
        }
        // A separator variant spelled with the camelCase boundary is the
        // same drift, and a case variant with the separator kept.
        let guard = write_guard(
            &root,
            &serde_json::json!({"path":"/x","content":"let Judge_Route = judge_route();"}),
        );
        assert!(matches!(guard, WriteGuard::Refused { .. }), "{guard:?}");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_novel_name_with_a_canonical_neighbour_is_flagged_with_the_candidate() {
        // `route_judge` beside an established `judge_route` is a novel
        // name for an existing concept: the flag names the candidate, so
        // the model can rename.
        let root = vocabulary_root("guard-flag");
        let guard = write_guard(
            &root,
            &serde_json::json!({"path":"/x","content":"let route_judge = judge_route();"}),
        );
        match guard {
            WriteGuard::Flagged { note } => {
                assert!(note.contains("route_judge"), "the candidate is named: {note}");
                assert!(note.contains("judge_route"), "the canonical term is named: {note}");
            }
            other => panic!("expected a flag, got {other:?}"),
        }
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_genuinely_new_concept_is_not_flagged() {
        // The conservative half: a guard that refuses legitimate new
        // names gets ignored, and then it is worse than nothing.
        let root = vocabulary_root("guard-new");
        let guard = write_guard(
            &root,
            &serde_json::json!({"path":"/x","content":"let wobblegong_vexil = 1;"}),
        );
        assert_eq!(guard, WriteGuard::Clean);
        // The established term itself is nothing to flag either.
        let guard = write_guard(
            &root,
            &serde_json::json!({"path":"/x","content":"fn judge_route() -> u8 { judge_route() }"}),
        );
        assert_eq!(guard, WriteGuard::Clean);
        // A write with no content has no names to check.
        let guard = write_guard(&root, &serde_json::json!({"path":"/x"}));
        assert_eq!(guard, WriteGuard::Clean);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn a_refused_write_does_not_land_and_a_flagged_one_carries_the_note() {
        // The guard rides the write's own path: a refusal stops the
        // write, a flag lets it land with the note on the result the
        // model already reads, and neither adds an approval gate.
        let root = vocabulary_root("guard-work");
        let files = scratch("guard-work-files");
        let refused = files.join("refused.rs");
        let (output, _) = run_work(Work::Builtin {
            tool: Tool::WriteFile,
            arguments: serde_json::json!({
                "path": refused.display().to_string(),
                "content": "let judgeRoute = 1;"
            }),
            root: root.clone(),
        })
        .await;
        assert!(output.contains("write_file` refused"), "{output}");
        assert!(output.contains("judgeRoute"), "{output}");
        assert!(!refused.exists(), "a refused write does not land");
        let flagged = files.join("flagged.rs");
        let (output, _) = run_work(Work::Builtin {
            tool: Tool::WriteFile,
            arguments: serde_json::json!({
                "path": flagged.display().to_string(),
                "content": "let route_judge = 1;"
            }),
            root: root.clone(),
        })
        .await;
        assert_eq!(std::fs::read_to_string(&flagged).unwrap(), "let route_judge = 1;");
        assert!(output.starts_with("wrote 20 bytes to"), "{output}");
        assert!(output.contains("route_judge"), "the candidate is named: {output}");
        assert!(output.contains("judge_route"), "the canonical term is named: {output}");
        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&files);
    }

    #[test]
    fn the_guard_reads_the_vocabularys_canonical_form() {
        // The lookup key mirrors the vocabulary's own: the same parts in
        // the same order, however they are spelled.
        assert_eq!(identifier_parts("recall_count"), vec!["recall", "count"]);
        assert_eq!(identifier_parts("recallCount"), vec!["recall", "Count"]);
        assert_eq!(identifier_parts("HTTPServer"), vec!["HTTP", "Server"]);
        assert_eq!(identifier_parts("judge-route"), vec!["judge", "route"]);
        // The tokens a file introduces are identifier-shaped, in order,
        // and nothing that short can be an established term. A kebab
        // spelling is two words to the tokenizer — the same definition
        // the vocabulary's own extractor uses.
        assert_eq!(
            identifier_tokens("let judge_route = routeJudge(); // done"),
            vec!["let", "judge_route", "routeJudge", "done"]
        );
        assert!(identifier_tokens("ab cd ef").is_empty(), "sub-three-character names are dropped");
    }
}
