//! The bridge between the agent core and the browser face.
//!
//! The TUI drives the agent from a thread of its own and renders what
//! crosses back ([`crate::tui`]); this module drives the same agent from
//! a thread of its own and turns what crosses back into JSON the page
//! reduces. The agent core is untouched by either face, which is the
//! point of G1's seam: a third surface is a new bridge, not a new agent.
//!
//! ## What lives here
//!
//! - **The agent thread**, mirroring `tui::spawn_agent` call for call:
//!   one thread, one 1-worker runtime, the consolidator and the
//!   scheduler beside it, skills loaded once, the route prepared once on
//!   the first task, memory loaded per task, one agent per task over a
//!   freshly opened log. The only differences are the prompt line — the
//!   owner is in a browser, and the run says so — and the approver.
//! - **The seat.** [`GuiApprover`] asks the browser and waits, exactly
//!   as `TuiApprover` asks the pane and waits. The difference is a
//!   remembered-rule check before the prompt: the owner's "yes, and
//!   don't ask again" writes a rule through the one writer, and every
//!   later call in that class crosses without a prompt, which is what
//!   "prompts thin out over time" means concretely.
//! - **The ordered log.** Every event the core emits lands in one
//!   `VecDeque` and crosses one broadcast channel. `/api/state` hands the
//!   page the log so a refresh replays the conversation it was watching,
//!   and the stream carries what happens next — so one derivation of
//!   "what does the header say" serves both, and a second one cannot
//!   disagree with the first.
//!
//! ## Lock discipline
//!
//! Two mutexes exist — the shared state and the remembered rules — and
//! neither is ever held across a blocking wait. The seat's answer takes
//! the seat out of the state, drops the lock, writes the rule, swaps the
//! rules, and only then sends on the responder; the approver reads the
//! rules and drops that lock before it blocks on the decision. A mutex
//! held across a `recv` is a deadlock the browser can trigger by
//! reloading.

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread;

use serde_json::{json, Value};
use tokio::sync::broadcast;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::agent::{
    Agent, AgentOptions, Approval, ApprovalRequest, Approver, SubagentStage, UiEvent,
    DEFAULT_MAX_TURNS,
};
use crate::api::{Client, ToolCall};
use crate::approvals;
use crate::gui;
use crate::session::{self, Session};

/// How many events the ordered log holds. A run that streams more than
/// this keeps the tail: the transcript is the recent conversation, not
/// an archive, and the session log on disk is the archive.
const MAX_EVENTS: usize = 4096;

/// The broadcast channel's depth. A browser that falls behind lands on
/// `/api/state` for the replay, so a lagging stream drops events rather
/// than holding the run — and it says so with a `dropped` event.
const BROADCAST_DEPTH: usize = 256;

/// One instruction from the browser to the agent thread. A browser
/// session picks its session at launch and never switches, so this is
/// one arm where the TUI's is two: there is no resume to post.
pub(crate) enum Command {
    /// Run this task in the session this bridge was built for.
    Run(String),
}

/// The agent thread's inputs, as one struct because the thread's own
/// function already reads its seven neighbours and clippy counts
/// arguments.
struct AgentInputs {
    home: PathBuf,
    model: String,
    session_id: String,
    ui_tx: UnboundedSender<UiEvent>,
    approval_tx: UnboundedSender<ApprovalRequest>,
    command_rx: UnboundedReceiver<Command>,
    base_url: String,
}

/// The live face's shared truth: the ordered event log, whether a run is
/// in flight, and the approval the browser owes an answer to.
struct GuiState {
    events: VecDeque<Value>,
    /// How many events the cap pushed out the front. The page says the
    /// earlier ones are gone rather than implying it holds everything.
    dropped: usize,
    running: bool,
    /// The pending approval, with the responder the agent is blocked on.
    /// `None` when no call is waiting.
    seat: Option<ApprovalRequest>,
}

/// The channels a detached bridge exposes, so a test can drive the core
/// without a thread, a corpus, or a scheduler behind it.
pub struct Sources {
    /// Where core events arrive.
    pub ui: UnboundedSender<UiEvent>,
    /// Where approval prompts arrive.
    pub seat: UnboundedSender<ApprovalRequest>,
}

/// The browser face's end of the agent core: the thread, the ordered
/// log, the remembered rules, and the seat.
pub struct Bridge {
    home: PathBuf,
    model: String,
    session: String,
    state: Arc<Mutex<GuiState>>,
    rules: Arc<Mutex<approvals::Rules>>,
    events: broadcast::Sender<Value>,
    /// The agent thread's command channel. `None` on a detached bridge,
    /// which has no thread to command — so `run` says so rather than
    /// queueing a task nobody will read.
    commands: Option<UnboundedSender<Command>>,
    /// Held so the thread outlives the bridge: a `JoinHandle` dropped
    /// detaches rather than joins, which is what a browser face wants.
    _agent: Option<thread::JoinHandle<()>>,
}

impl Bridge {
    /// A bridge over an existing home and session, with no agent behind
    /// it: the channels come back as [`Sources`] and the caller drives
    /// them. This is the seam the server's tests and this module's own
    /// use, and it deliberately spawns nothing else — no consolidator, no
    /// scheduler — because those touch the machine's real memories
    /// corpus and cron store, and a test must not.
    ///
    /// Must be called from inside a tokio runtime: the fan-out that turns
    /// core events into JSON is a task of the caller's, so a browser face
    /// and a test drive it identically.
    pub fn detached(home: PathBuf, model: String, session: String) -> (Bridge, Sources) {
        let (ui_tx, ui_rx) = tokio::sync::mpsc::unbounded_channel();
        let (seat_tx, seat_rx) = tokio::sync::mpsc::unbounded_channel();
        let (events, _) = broadcast::channel(BROADCAST_DEPTH);
        let state = Arc::new(Mutex::new(GuiState {
            events: VecDeque::new(),
            dropped: 0,
            running: false,
            seat: None,
        }));
        let rules = Arc::new(Mutex::new(crate::settings::approval_rules(&home)));
        tokio::spawn(fan_out(ui_rx, seat_rx, Arc::clone(&state), events.clone()));
        let bridge = Bridge {
            home,
            model,
            session,
            state,
            rules,
            events,
            commands: None,
            _agent: None,
        };
        (
            bridge,
            Sources {
                ui: ui_tx,
                seat: seat_tx,
            },
        )
    }

    /// Opens the door the agent thread reads commands from and hands
    /// back the receiving end. `spawn` gives it to the thread it
    /// starts; a test holds it to stand the door open with no thread
    /// behind it, which is the one way to watch a run be accepted
    /// without a network answering it.
    ///
    /// Crate-private because a door is not something the outside hands
    /// out: the only doors are the one `spawn` opens and the one a test
    /// opens standing in for it.
    pub(crate) fn connect(&mut self) -> tokio::sync::mpsc::UnboundedReceiver<Command> {
        let (commands, receiver) = tokio::sync::mpsc::unbounded_channel();
        self.commands = Some(commands);
        receiver
    }

    /// A bridge with the agent thread behind it: the thread this module
    /// spawns is the same shape `tui::spawn_agent` spawns, with the
    /// browser's prompt line and the seat's approver.
    pub fn spawn(home: PathBuf, model: String, session: String, base_url: String) -> Bridge {
        let (mut bridge, sources) = Bridge::detached(home.clone(), model.clone(), session.clone());
        let command_rx = bridge.connect();
        let handle = thread::spawn(move || {
            let _ = spawn_agent(AgentInputs {
                home,
                model,
                session_id: session,
                ui_tx: sources.ui,
                approval_tx: sources.seat,
                command_rx,
                base_url,
            });
        });
        Bridge {
            _agent: Some(handle),
            ..bridge
        }
    }

    /// Subscribe to the live stream. A page that has just loaded uses
    /// [`Bridge::state`] for the replay and this for what comes next; the
    /// two agree because they are the same log.
    pub fn subscribe(&self) -> broadcast::Receiver<Value> {
        self.events.subscribe()
    }

    /// The whole live face as one document: the session, the model, the
    /// context budget the meter divides by, the ordered log, the seat,
    /// and the remembered rules. Everything the header derives — spend,
    /// context, route, turns, stop reason — the page reduces out of the
    /// log, so there is one derivation and not two.
    pub fn state(&self) -> Value {
        let state = lock(&self.state);
        let rules = lock(&self.rules);
        json!({
            "session": self.session,
            "model": self.model,
            "budget": crate::compaction::COMPACTION_TOKEN_BUDGET,
            "running": state.running,
            "dropped": state.dropped,
            "events": state.events,
            "approval": state.seat.as_ref().map(|seat| {
                json!({
                    "tool": seat.call.function.name,
                    "summary": seat.summary,
                    "label": approvals::Rule::for_call(&seat.call).label(),
                    "allowlistable": approvals::allowlistable(&seat.call),
                })
            }),
            "rules": rules.list().iter().map(rule_value).collect::<Vec<_>>(),
        })
    }

    /// The router panel's document: the router board's own reads, shaped
    /// into the browser's card language, plus the route-decision cache's
    /// counts. Read-only, and read off the async worker — it touches the
    /// roster, the calibration log, and the cache file, and a page
    /// request must not hold the runtime while they are read.
    ///
    /// The live decision is deliberately absent. It rides the event log
    /// the conversation already carries — the `routed` event holds the
    /// decision's own facts — so the panel reduces it client-side like
    /// every other number on this page, and this document cannot
    /// disagree with the conversation about which route served the run
    /// in flight.
    pub async fn router(&self) -> Value {
        let home = self.home.clone();
        match tokio::task::spawn_blocking(move || router_read(&home)).await {
            Ok(document) => document,
            // A join failure means the worker died, which this read has
            // no path to: the panel says the read did not land rather
            // than hanging the page on a document that never came.
            Err(_) => json!({
                "type": "router",
                "refused": ["the router read did not land"],
            }),
        }
    }

    /// The delegation panel's document: the live run's tree, the past
    /// runs' trees, and the declared dependency graph with the unlock
    /// conditions the code actually requires. Read-only, and read off
    /// the async worker for the same reason the router read is — it
    /// touches every session log's opening record and the graph file,
    /// and a page request must not hold the runtime while they are read.
    ///
    /// The live tree is rooted at this face's own session, so the panel
    /// shows the run the owner is watching rather than whichever run
    /// happens to be newest.
    pub async fn delegation(&self) -> Value {
        let home = self.home.clone();
        let session = self.session.clone();
        match tokio::task::spawn_blocking(move || delegation_read(&home, &session)).await {
            Ok(document) => document,
            Err(_) => json!({
                "type": "delegation",
                "refused": ["the delegation read did not land"],
            }),
        }
    }

    /// Hand the browser's task to the agent thread. The owner's message
    /// lands in the log first, so a refresh replays what was asked and
    /// the header's running flag moves on the same event.
    pub fn run(&self, task: &str) -> Result<(), String> {
        let task = task.trim();
        if task.is_empty() {
            return Err("a run needs a task".to_string());
        }
        {
            let mut state = lock(&self.state);
            if state.running {
                return Err("a run is already in flight; let it finish first".to_string());
            }
            state.running = true;
        }
        record(
            &self.state,
            &self.events,
            json!({"type": "message", "role": "owner", "text": task}),
        );
        let delivered = match self.commands.as_ref() {
            Some(commands) => commands.send(Command::Run(task.to_string())).is_ok(),
            None => false,
        };
        if !delivered {
            // The command had nowhere to go, so the run never started:
            // the flag comes back and the log says the run failed, which
            // is what the owner's browser must show rather than a face
            // that says "running" forever.
            let mut state = lock(&self.state);
            state.running = false;
            drop(state);
            record(
                &self.state,
                &self.events,
                json!({"type": "failed", "reason": "this face has no agent behind it"}),
            );
            return Err("this face has no agent behind it".to_string());
        }
        Ok(())
    }

    /// The browser's answer to the pending seat.
    ///
    /// Four words, and they are the seat's four buttons: `once` allows
    /// this call, `always` allows it and the rest of the run, `deny`
    /// refuses it, and `remember` allows it and writes the class through
    /// the one writer so later runs never ask. A `remember` whose write
    /// fails still answers the prompt — the owner said yes to this call,
    /// and a settings failure is not a reason to strand the run — and the
    /// failure rides the reply so the page can say so.
    pub fn answer(&self, decision: &str) -> Result<Value, AnswerError> {
        let approval = match decision {
            "once" => Approval::Once,
            "always" => Approval::Always,
            "deny" => Approval::Denied,
            "remember" => Approval::Once,
            other => {
                return Err(AnswerError::Unknown(format!(
                    "`{other}` is not a decision this seat answers"
                )))
            }
        };
        // The seat comes out first and the lock drops with it: the write
        // below is a file write, and a mutex held across it would stall
        // every state read for as long as the disk takes.
        let seat = {
            let mut state = lock(&self.state);
            state.seat.take().ok_or(AnswerError::NoSeat)?
        };
        let mut rule = None;
        let mut error = None;
        if decision == "remember" {
            match crate::settings::allow_tool(&self.home, &seat.call) {
                Ok(written) => {
                    // The file is the source of truth, so the shared copy
                    // is re-read rather than patched: a hand-edit between
                    // the launch and the click is respected.
                    let fresh = crate::settings::approval_rules(&self.home);
                    let listed = fresh.list().iter().map(rule_value).collect::<Vec<_>>();
                    *lock(&self.rules) = fresh;
                    rule = Some(written.label());
                    let _ = self.events.send(json!({"type": "rules", "rules": listed}));
                }
                Err(message) => error = Some(message),
            }
        }
        let _ = seat.responder.send(approval);
        let _ = self.events.send(json!({"type": "seat_cleared"}));
        Ok(json!({
            "answered": decision,
            "rule": rule,
            "error": error,
        }))
    }
}

/// Why an answer did not cross. The two are different failures for the
/// page and different answers on the wire: a seat that never stood is a
/// conflict with what the browser thinks it is looking at, and a word
/// this seat does not answer is a malformed request that leaves the seat
/// exactly where it was.
#[derive(Debug)]
pub enum AnswerError {
    /// No approval is waiting.
    NoSeat,
    /// The word is not one of the four.
    Unknown(String),
}

impl std::fmt::Display for AnswerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AnswerError::NoSeat => write!(f, "no approval is waiting for an answer"),
            AnswerError::Unknown(word) => write!(f, "{word}"),
        }
    }
}

impl std::error::Error for AnswerError {}

/// One rule as the trust surface reads it. The label is derived here
/// rather than stored, so the rule the file holds and the line the page
/// shows cannot drift.
fn rule_value(rule: &approvals::Rule) -> Value {
    json!({
        "tool": rule.tool,
        "program": rule.program,
        "label": rule.label(),
    })
}

/// One read of the router board, shaped for the browser. Every field
/// names what the board read, so the panel's cards are the board's own
/// facts rather than a second opinion about them.
fn router_read(home: &std::path::Path) -> Value {
    let dotenv = crate::home::read_dotenv(home);
    let sources = crate::boards::resolve_sources(home, &dotenv);
    let snapshot = crate::boards::read_router(&sources.router, &crate::calib::today());
    // The cache's counts, and whether the file on disk belongs to the
    // lane that would judge now. A cache written by another lane is
    // discarded by `select` whole, so the panel that showed its entries
    // as live would be showing a router that does not read them.
    let cache = crate::route_cache::Cache::load(home);
    let key = crate::lanes::spec_for(crate::route::JUDGE_LANE)
        .map(|spec| crate::route_cache::Cache::key_for(&spec))
        .unwrap_or_default();
    let stats = cache.stats();
    let (judge_lane, default_workload) = match snapshot.judge {
        Some((lane, workload)) => (lane, workload),
        None => (String::new(), None),
    };
    let (payg_allowed, payg_providers) = snapshot.payg.unwrap_or((false, Vec::new()));
    json!({
        "type": "router",
        "read_at": snapshot.read_at,
        "judge": {
            "lane": judge_lane,
            "default_workload": default_workload,
        },
        "pin": snapshot.pin,
        "payg": {"allowed": payg_allowed, "providers": payg_providers},
        "tiers": snapshot.tiers.iter().map(|chain| json!({
            "tier": chain.tier,
            "primary": chain.primary,
            "candidates": chain.candidates,
        })).collect::<Vec<_>>(),
        "totals": snapshot.totals.map(|totals| json!({
            "calls": totals.calls,
            "ok": totals.ok,
            "errors": totals.errors,
            "prompt_tokens": totals.prompt_tokens,
            "completion_tokens": totals.completion_tokens,
            "total_tokens": totals.total_tokens,
            "success_rate": totals.success_rate(),
        })),
        "models": snapshot.models.iter().map(|row| json!({
            "key": row.key,
            "provider": row.provider,
            "model": row.model,
            "calls": row.calls,
            "errors": row.errors,
            "total_tokens": row.total_tokens,
        })).collect::<Vec<_>>(),
        "recent": snapshot.recent.iter().map(|row| json!({
            "at": row.at,
            "provider": row.provider,
            "model": row.model,
            "outcome": row.outcome,
            "ok": row.ok(),
            "providers_used": row.providers_used,
            "workload": row.workload,
            "ms": row.ms,
            "reason": row.reason,
        })).collect::<Vec<_>>(),
        "hours": snapshot.hours.iter().map(|column| json!({
            "hour": column.hour,
            "calls": column.calls,
            "errors": column.errors,
        })).collect::<Vec<_>>(),
        "judged": snapshot.judge_counters.map(|counters| json!({
            "fresh": counters.fresh,
            "cached": counters.cached,
            "errors": counters.errors,
        })),
        "workloads": snapshot.workloads.iter().map(|(workload, count)| json!({
            "workload": workload,
            "count": count,
        })).collect::<Vec<_>>(),
        "guidance": snapshot.guidance.iter().map(|(candidate, clause)| json!({
            "candidate": candidate,
            "clause": clause,
        })).collect::<Vec<_>>(),
        "refused": snapshot.refused,
        "cache": {
            "entries": stats.entries,
            "keyed": cache.keyed_for(&key),
            "workloads": stats.workloads.iter().map(|(workload, count)| json!({
                "workload": workload,
                "count": count,
            })).collect::<Vec<_>>(),
        },
    })
}

/// One read of the delegation board, shaped for the browser. The tree
/// nodes and the graph nodes ride through as they were read — the panel
/// draws them, and a document that re-decided what a node's state was
/// would be a second opinion about the graph's own facts.
fn delegation_read(home: &std::path::Path, session: &str) -> Value {
    let snapshot = crate::boards::read_delegation(home, session);
    json!({
        "type": "delegation",
        "read_at": snapshot.read_at,
        "depth_cap": snapshot.depth_cap,
        "live": snapshot.live.as_ref().map(delegation_node),
        "history": snapshot.history.iter().map(delegation_node).collect::<Vec<_>>(),
        "graph": snapshot.graph.as_ref().map(graph_document),
        "refused": snapshot.refused,
    })
}

/// One tree node and its children, recursively.
fn delegation_node(node: &crate::boards::DelegationNode) -> Value {
    json!({
        "id": node.id,
        "model": node.model,
        "kind": node.kind,
        "depth": node.depth,
        "goal": node.goal,
        "status": node.status,
        "stop_reason": node.stop_reason,
        "turns": node.turns,
        "children": node.children.iter().map(delegation_node).collect::<Vec<_>>(),
    })
}

/// The dependency graph: the nodes in declaration order, the edges as
/// index pairs into that order, and the graph's own aggregates.
fn graph_document(graph: &crate::boards::GraphSnapshot) -> Value {
    json!({
        "nodes": graph.nodes.iter().map(|node| json!({
            "id": node.id,
            "cost": node.cost,
            "score": node.score,
            "peak": node.peak,
            "status": node.status,
            "state": node.state,
            "approach": node.approach,
            "paths": node.paths,
            "capabilities": node.capabilities,
            "prereqs": node.prereqs,
            "unlock": {
                "score_zero": node.unlock.score_zero,
                "resource_free": node.unlock.resource_free,
                "deps_verified": node.unlock.deps_verified,
                "unlocked": node.unlock.unlocked,
                "waiting": node.unlock.waiting,
                "failed": node.unlock.failed,
                "unresolved": node.unlock.unresolved,
                "held": node.unlock.held.iter().map(|(holder, path)| json!({
                    "holder": holder,
                    "path": path,
                })).collect::<Vec<_>>(),
            },
            "reason": node.reason,
            "delta": node.delta.as_ref().map(|delta| json!({
                "state": delta.state,
                "reason": delta.reason,
            })),
        })).collect::<Vec<_>>(),
        "edges": graph.edges.iter().map(|(from, to)| json!({
            "from": from,
            "to": to,
        })).collect::<Vec<_>>(),
        "critical_path": graph.critical_path,
        "blocks_most_work": graph.blocks_most_work,
        "revisions": graph.revisions,
        "diff_score": graph.diff_score,
        "replan": graph.replan,
    })
}

/// The poison-tolerant lock, the house idiom: a panic in one task must
/// not make every later read of the face's state a panic too.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|poison| {
        eprintln!(
            "fleet gui: a bridge mutex was poisoned by a panic; recovering the value it guards"
        );
        PoisonError::into_inner(poison)
    })
}

/// One event into the ordered log and out to the stream. The log is what
/// a refresh replays; the broadcast is what a live page reduces. The
/// two are written from the same value, so they cannot disagree.
fn record(state: &Mutex<GuiState>, events: &broadcast::Sender<Value>, value: Value) {
    let mut notice = None;
    {
        let mut state = lock(state);
        // The run is over the moment the core says so, whichever word it
        // used: a header that kept counting after the loop stopped would
        // be lying about where the owner is.
        if matches!(
            value.get("type").and_then(Value::as_str),
            Some("finished") | Some("failed")
        ) {
            state.running = false;
        }
        if state.events.len() >= MAX_EVENTS {
            state.events.pop_front();
            state.dropped += 1;
            if state.dropped == 1 {
                notice = Some(json!({"type": "dropped", "dropped": state.dropped}));
            }
        }
        state.events.push_back(value.clone());
    }
    if let Some(notice) = notice {
        let _ = events.send(notice);
    }
    let _ = events.send(value);
}

/// Turns everything the core emits into what the page reduces. One
/// function, so the transcript, the tool cards, and the intelligence
/// cards all read the same shape the stream carried — a second mapping
/// would be a second answer to what an event means.
fn describe(event: &UiEvent) -> Value {
    match event {
        UiEvent::Token(text) => json!({"type": "token", "text": text}),
        UiEvent::Turn(turn) => json!({"type": "turn", "turn": turn}),
        UiEvent::Usage {
            prompt_tokens,
            completion_tokens,
        } => json!({
            "type": "usage",
            "prompt": prompt_tokens,
            "completion": completion_tokens,
        }),
        UiEvent::ToolCall { name, summary } => {
            json!({"type": "tool_call", "tool": name, "summary": summary})
        }
        UiEvent::ToolResult {
            name,
            preview,
            approved,
        } => json!({
            "type": "tool_result",
            "tool": name,
            "preview": preview,
            "approved": approved,
        }),
        UiEvent::LaneJoined { lane, tool_count } => {
            json!({"type": "lane", "lane": lane, "tools": tool_count})
        }
        UiEvent::Routed {
            provider,
            model,
            how,
            workload,
            verdict,
            judged_from,
            reason,
            order,
        } => json!({
            "type": "routed",
            "provider": provider,
            "model": model,
            "how": how,
            "workload": workload,
            "verdict": verdict,
            "judged_from": judged_from,
            "reason": reason,
            "order": order
                .iter()
                .map(|(provider, model)| json!({"provider": provider, "model": model}))
                .collect::<Vec<_>>(),
        }),
        UiEvent::Compacted {
            turns,
            tokens_before,
            tokens_after,
        } => json!({
            "type": "compacted",
            "turns": turns,
            "before": tokens_before,
            "after": tokens_after,
        }),
        UiEvent::Memory { shape, facts } => {
            json!({"type": "memory", "shape": shape, "facts": facts})
        }
        UiEvent::SkillsLoaded { loaded, skipped } => {
            json!({"type": "skills", "loaded": loaded, "skipped": skipped})
        }
        UiEvent::Subagent {
            goal,
            session,
            stage,
        } => {
            // The stage's own payload rides nested, so the card renders
            // one shape and the stage decides what it says: a spawned
            // child has nothing to report, a working one has a call
            // count and a tool, a finished one has an outcome and a turn
            // count.
            let (stage, detail) = match stage {
                SubagentStage::Spawned => ("spawned", json!({})),
                SubagentStage::Working { calls, tool } => {
                    ("working", json!({"calls": calls, "tool": tool}))
                }
                SubagentStage::Done { outcome, turns } => {
                    ("done", json!({"outcome": outcome, "turns": turns}))
                }
            };
            json!({
                "type": "delegation",
                "goal": goal,
                "session": session,
                "stage": stage,
                "detail": detail,
            })
        }
        UiEvent::CronFire {
            job,
            minute,
            catch_up,
            outcome,
            detail,
            session,
        } => json!({
            "type": "cron",
            "job": job,
            "minute": minute,
            "catch_up": catch_up,
            "outcome": outcome,
            "detail": detail,
            "session": session,
        }),
        UiEvent::Finished { stop_reason } => {
            json!({"type": "finished", "reason": stop_reason})
        }
        UiEvent::Failed(reason) => json!({"type": "failed", "reason": reason}),
    }
}

/// The seat as the page shows it. The tool name rides raw, label and
/// all, because that is what the prompt reads; the rule's label is the
/// unlabelled class, because that is what "don't ask again" would
/// remember; and `allowlistable` is carried so the page can hide the
/// choice honestly rather than offer a button the writer would refuse.
fn describe_seat(call: &ToolCall, summary: &str) -> Value {
    json!({
        "type": "seat",
        "tool": call.function.name,
        "summary": summary,
        "label": approvals::Rule::for_call(call).label(),
        "allowlistable": approvals::allowlistable(call),
    })
}

/// Drains both core channels into the ordered log for as long as either
/// is open. Both close only when the agent thread ends, which is the
/// bridge's own end: the channels are the thread's lifetime.
///
/// The two branches are guarded by their own done flags rather than by
/// `recv()` returning `None`, because a closed channel answers at once
/// and forever — an unguarded select would spin. The loop's condition
/// keeps at least one branch enabled, which is what `select!` requires.
async fn fan_out(
    mut ui_rx: UnboundedReceiver<UiEvent>,
    mut seat_rx: UnboundedReceiver<ApprovalRequest>,
    state: Arc<Mutex<GuiState>>,
    events: broadcast::Sender<Value>,
) {
    let mut ui_done = false;
    let mut seat_done = false;
    while !(ui_done && seat_done) {
        tokio::select! {
            event = ui_rx.recv(), if !ui_done => match event {
                Some(event) => record(&state, &events, describe(&event)),
                None => ui_done = true,
            },
            request = seat_rx.recv(), if !seat_done => match request {
                Some(request) => {
                    let pending = describe_seat(&request.call, &request.summary);
                    lock(&state).seat = Some(request);
                    let _ = events.send(pending);
                }
                None => seat_done = true,
            },
        }
    }
    // The thread is gone, so nothing is waiting on a responder any more:
    // a seat left standing would be a prompt the page can answer into a
    // channel nobody reads. Clearing it is the honest end of the run.
    let standing = lock(&state).seat.take().is_some();
    if standing {
        let _ = events.send(json!({"type": "seat_cleared"}));
    }
}

/// The approver the browser face runs on: `TuiApprover`'s round-trip,
/// with the remembered rules consulted before the prompt.
///
/// The mode and its `Gate` are a headless concept — a face with an owner
/// in it asks, and the only question is which of the calls it asks
/// about. That is what the remembered rules answer, and they answer it
/// first, so a class the owner has already ruled on crosses without a
/// prompt at all.
pub struct GuiApprover {
    requests: UnboundedSender<ApprovalRequest>,
    rules: Arc<Mutex<approvals::Rules>>,
    /// The run-scoped "allow everything" latch, reported through
    /// [`Approver::latched`] exactly as the console's and the TUI's are.
    always: bool,
}

impl GuiApprover {
    /// An approver that prompts `requests` and consults `rules` first.
    pub fn new(
        requests: UnboundedSender<ApprovalRequest>,
        rules: Arc<Mutex<approvals::Rules>>,
    ) -> GuiApprover {
        GuiApprover {
            requests,
            rules,
            always: false,
        }
    }
}

impl Approver for GuiApprover {
    fn approve(&mut self, call: &ToolCall) -> bool {
        if self.always {
            return true;
        }
        // The lock is taken, read, and dropped before the prompt: the
        // seat's `remember` swaps this same mutex, and holding it across
        // the blocking recv would deadlock the face on a reload.
        if lock(&self.rules).covers(call) {
            return true;
        }
        let (responder, decision) = std::sync::mpsc::channel();
        let request = ApprovalRequest {
            call: call.clone(),
            summary: crate::agent::call_summary(call),
            responder,
        };
        // A browser that went away denies rather than hanging the run.
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

/// Runs the agent on its own thread and runtime, so the browser's tasks
/// never wait on the network or on a tool. The shape is `spawn_agent`'s
/// in the TUI, call for call; the differences are the prompt line and
/// the approver, and both are the point of this face.
fn spawn_agent(inputs: AgentInputs) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let AgentInputs {
            home,
            model,
            session_id,
            ui_tx,
            approval_tx,
            command_rx,
            base_url,
        } = inputs;
        let rules = Arc::new(Mutex::new(crate::settings::approval_rules(&home)));
        // The scheduled consolidation cycle: one pass now, then one every
        // hour for the life of the process — the timer the live system
        // never had. Its own stream, because the cycle must run even when
        // routing never produced one.
        let user_home = std::env::var("HOME").ok();
        let calib_root = crate::calib::resolve_root(&home, user_home.as_deref());
        let memories_root =
            crate::memory::resolve_root(&std::env::current_dir().unwrap_or_default());
        crate::memory::spawn_consolidator(memories_root, crate::calib::Stream::open(&calib_root));
        // The scheduled jobs: the catch-up pass once at startup, then one
        // tick per second for the life of the process. A fire is an
        // ordinary headless run in its own session log, never a turn in
        // the session this face is driving, and its outcome rides this
        // face's own event channel and lands as a card (KI-090).
        crate::cron::Scheduler::spawn(home.clone(), Some(ui_tx.clone()));
        let runtime = match tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
        {
            Ok(runtime) => runtime,
            Err(error) => {
                let _ = ui_tx.send(UiEvent::Failed(format!(
                    "the agent runtime failed to start: {error}"
                )));
                return;
            }
        };
        runtime.block_on(async move {
            let dotenv = crate::home::read_dotenv(&home);
            // The skills are loaded once, when the agent thread starts,
            // and declared on every run it drives: the files can change
            // between runs, and each run's `skills` record pins what that
            // run actually saw. A clean load emits nothing.
            let user_home = std::env::var("HOME").ok();
            let skills_override = std::env::var("FLEET_SKILLS_DIR").ok();
            let (skills, skill_skips) =
                crate::skills::resolve_skills_dir(skills_override.as_deref(), user_home.as_deref())
                    .map(|dir| crate::skills::load_reported(&dir))
                    .unwrap_or_default();
            if !skill_skips.is_empty() {
                let _ = ui_tx.send(UiEvent::SkillsLoaded {
                    loaded: skills.len(),
                    skipped: skill_skips.into_iter().map(|skip| skip.name).collect(),
                });
            }
            // The route is chosen once, on the first task, and the
            // committed router is reused for the rest of the session —
            // one sys1 judgment, not one per task.
            let mut prepared: Option<crate::route::Prepared> = None;
            let mut judge_task = String::new();
            let current_session = session_id;
            let mut commands = command_rx;
            while let Some(command) = commands.recv().await {
                match command {
                    Command::Run(task) => {
                        if prepared.is_none() {
                            match crate::route::prepare(
                                &home,
                                &dotenv,
                                &task,
                                &std::collections::HashMap::new(),
                            )
                            .await
                            {
                                Ok(fresh) => {
                                    judge_task = task.clone();
                                    prepared = Some(fresh);
                                }
                                Err(error) => {
                                    let _ = ui_tx
                                        .send(UiEvent::Failed(format!("routing config: {error}")));
                                    continue;
                                }
                            }
                        }
                        let prepared = prepared.as_ref().expect("prepared on the first run");
                        let client = Client::routed(prepared.router.clone());
                        let path = session::session_path(&home, &current_session);
                        // The corpus is loaded per task, off the async
                        // worker: the files can change between runs, and a
                        // fact an earlier run of this session remembered is
                        // on disk by the time this one assembles.
                        let memory = tokio::task::spawn_blocking(|| {
                            let root = crate::memory::resolve_root(
                                &std::env::current_dir().unwrap_or_default(),
                            );
                            std::sync::Arc::new(crate::memory::Memory::load(&root))
                        })
                        .await
                        .ok();
                        // One agent per task over a freshly opened log: the
                        // replay seeds the conversation exactly as a
                        // resumed headless run is seeded.
                        let mut log = match Session::open(&path) {
                            Ok(log) => log,
                            Err(error) => {
                                let _ = ui_tx.send(UiEvent::Failed(format!(
                                    "could not open the session log: {error}"
                                )));
                                continue;
                            }
                        };
                        let options = AgentOptions {
                            model: model.clone(),
                            max_turns: DEFAULT_MAX_TURNS,
                            identity: crate::home::read_soul(&home),
                            ui_tx: Some(ui_tx.clone()),
                            lane_specs: None,
                            calib: Some(prepared.calib.clone()),
                            skills: skills.clone(),
                            role_overlay: None,
                            memory,
                            scope: None,
                            // The self-maintenance tool rides the
                            // interactive surfaces: a session can file a
                            // proposal, and only the owner's `fleet improve
                            // apply` runs one.
                            improve_home: Some(home.clone()),
                            // The owner is in a browser, and the run says
                            // so: the prompt line is what tells the model
                            // its progress is being watched live and where
                            // the owner's messages arrive.
                            gui_prompt: Some(gui::prompt_line(&base_url)),
                            // The interactive surface has no forced-fold
                            // flag: its owner is present, so the budget's
                            // own timing is the right one.
                            compact_now: false,
                        };
                        let approver: Box<dyn Approver> =
                            Box::new(GuiApprover::new(approval_tx.clone(), Arc::clone(&rules)));
                        // Progress text has a sink here: the page renders
                        // the events, and the log records exactly what
                        // headless records.
                        let mut progress = std::io::sink();
                        let mut agent =
                            match Agent::new(&client, options, &mut log, approver, &mut progress) {
                                Ok(agent) => agent,
                                Err(error) => {
                                    let _ = ui_tx.send(UiEvent::Failed(error.to_string()));
                                    continue;
                                }
                            };
                        let _ = agent.run(&task).await;
                    }
                }
            }
            // The session's routing decision is recorded once, when the
            // agent thread ends — best-effort, never fatal.
            if let Some(prepared) = &prepared {
                prepared.record_route(&judge_task);
            }
            // And the self-authoring half runs once, on the session this
            // face drove: its lessons are drafted into proposals awaiting
            // the slice-12 gate, best-effort, never applied here.
            let user_home = std::env::var("HOME").ok();
            let path = crate::session::session_path(&home, &current_session);
            let skills_override = std::env::var("FLEET_SKILLS_DIR").ok();
            let filed = match crate::skills::resolve_skills_dir(
                skills_override.as_deref(),
                user_home.as_deref(),
            ) {
                Some(canonical) => {
                    crate::skills::draft_session(&canonical, &home, &path, user_home.as_deref())
                }
                None => Vec::new(),
            };
            if !filed.is_empty() {
                eprintln!(
                    "fleet: skills: drafted {} proposal(s) awaiting the gate in {}",
                    filed.len(),
                    crate::skills::proposals_dir(&home).display()
                );
            }
        });
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::FunctionCall;

    fn call(tool: &str, arguments: &str) -> ToolCall {
        ToolCall {
            id: "call_1".to_string(),
            kind: "function".to_string(),
            function: FunctionCall {
                name: tool.to_string(),
                arguments: arguments.to_string(),
            },
        }
    }

    /// A bridge over a home that has no config, so its rules read as
    /// empty, with the command door held open by the test: the run is
    /// accepted and stays in flight, which is what a face waiting on an
    /// agent thread looks like.
    async fn bridge() -> (Bridge, Sources, UnboundedReceiver<Command>) {
        let home = std::env::temp_dir().join(format!("fleet-gui-bridge-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).expect("a temp home");
        let (mut bridge, sources) = Bridge::detached(
            home,
            "deepseek-chat".to_string(),
            "session-test".to_string(),
        );
        let door = bridge.connect();
        (bridge, sources, door)
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn the_owner_message_lands_before_the_run_starts() {
        let (bridge, _sources, _door) = bridge().await;
        bridge
            .run("  look at the router  ")
            .expect("the run is accepted");
        let state = bridge.state();
        let events = state["events"].as_array().expect("an event log");
        assert_eq!(events.len(), 1, "{events:?}");
        assert_eq!(events[0]["type"], "message");
        assert_eq!(events[0]["role"], "owner");
        assert_eq!(events[0]["text"], "look at the router");
        assert_eq!(state["running"], true);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_run_that_is_already_in_flight_is_refused() {
        let (bridge, _sources, _door) = bridge().await;
        bridge.run("first").expect("the first run is accepted");
        let second = bridge.run("second");
        assert!(second.is_err(), "{second:?}");
        let state = bridge.state();
        assert_eq!(
            state["events"].as_array().map(Vec::len),
            Some(1),
            "the refused second task left no message"
        );
    }

    /// A face with no agent thread behind it refuses the run and comes
    /// back: the message stays in the log, the failure is named, and the
    /// running flag is not left standing over a run that never started.
    #[tokio::test(flavor = "multi_thread")]
    async fn a_run_with_no_agent_behind_it_is_refused_and_the_flag_comes_back() {
        let home = std::env::temp_dir().join(format!("fleet-gui-bridge-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).expect("a temp home");
        let (bridge, _sources) = Bridge::detached(
            home,
            "deepseek-chat".to_string(),
            "session-test".to_string(),
        );
        let refused = bridge.run("who is there");
        assert!(refused.is_err(), "{refused:?}");
        let state = bridge.state();
        let events = state["events"].as_array().expect("an event log");
        assert_eq!(events.len(), 2, "{events:?}");
        assert_eq!(events[1]["type"], "failed");
        assert_eq!(state["running"], false);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_seat_crosses_the_approver_and_the_answer_clears_it() {
        let (bridge, sources, _door) = bridge().await;
        let shell = call("bash", r#"{"command":"cargo test"}"#);
        let (responder, reply) = std::sync::mpsc::channel();
        sources
            .seat
            .send(ApprovalRequest {
                call: shell.clone(),
                summary: "run cargo test".to_string(),
                responder,
            })
            .expect("the seat arrives");
        assert!(
            reply.try_recv().is_err(),
            "the seat stands until it is answered"
        );
        // The fan-out is a task, so the seat needs a moment to land.
        for _ in 0..50 {
            if bridge.state()["approval"].is_object() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        let seat = &bridge.state()["approval"];
        assert_eq!(seat["tool"], "bash");
        assert_eq!(seat["label"], "bash · cargo");
        assert_eq!(seat["allowlistable"], true);
        assert_eq!(seat["summary"], "run cargo test");
        let answered = bridge.answer("once").expect("the seat is answered");
        assert_eq!(answered["answered"], "once");
        assert!(bridge.state()["approval"].is_null());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn an_answer_with_no_seat_is_a_conflict_not_a_denial() {
        let (bridge, _sources, _door) = bridge().await;
        let answered = bridge.answer("deny");
        assert!(answered.is_err(), "{answered:?}");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_word_that_is_not_a_decision_is_refused_with_the_seat_standing() {
        let (bridge, sources, _door) = bridge().await;
        let (responder, decision) = std::sync::mpsc::channel();
        sources
            .seat
            .send(ApprovalRequest {
                call: call("write", r#"{"path":"a.txt","content":"x"}"#),
                summary: "write a.txt".to_string(),
                responder,
            })
            .expect("the seat arrives");
        for _ in 0..50 {
            if bridge.state()["approval"].is_object() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        let refused = bridge.answer("sure");
        assert!(refused.is_err(), "{refused:?}");
        // A malformed answer is not the owner saying no: the seat stands.
        assert!(bridge.state()["approval"].is_object());
        assert!(decision.try_recv().is_err(), "nothing was sent back");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_remembered_rule_stops_the_prompt_and_rides_the_rules_event() {
        let (bridge, sources, _door) = bridge().await;
        let (responder, _decision) = std::sync::mpsc::channel();
        sources
            .seat
            .send(ApprovalRequest {
                call: call("bash", r#"{"command":"cargo build"}"#),
                summary: "run cargo build".to_string(),
                responder,
            })
            .expect("the seat arrives");
        for _ in 0..50 {
            if bridge.state()["approval"].is_object() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        let mut stream = bridge.subscribe();
        let answered = bridge.answer("remember").expect("the rule is remembered");
        assert_eq!(answered["answered"], "remember");
        assert_eq!(answered["rule"], "bash · cargo");
        let state = bridge.state();
        let rules = state["rules"].as_array().expect("a rules list");
        assert_eq!(rules.len(), 1, "{rules:?}");
        assert_eq!(rules[0]["label"], "bash · cargo");
        // And the live page hears about it without asking for state.
        let event: Value = serde_json::from_str(
            &stream
                .try_recv()
                .expect("the rules event crossed")
                .to_string(),
        )
        .expect("json");
        assert_eq!(event["type"], "rules");
        assert_eq!(event["rules"][0]["label"], "bash · cargo");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_destructive_call_cannot_be_remembered_and_says_why() {
        let (bridge, sources, _door) = bridge().await;
        let (responder, _decision) = std::sync::mpsc::channel();
        sources
            .seat
            .send(ApprovalRequest {
                call: call("bash", r#"{"command":"rm -rf /tmp/x"}"#),
                summary: "remove /tmp/x".to_string(),
                responder,
            })
            .expect("the seat arrives");
        for _ in 0..50 {
            let approval = bridge.state()["approval"].clone();
            if approval.is_object() {
                assert_eq!(
                    approval["allowlistable"], false,
                    "the seat must not offer a promise the writer refuses"
                );
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        let answered = bridge
            .answer("remember")
            .expect("the prompt is still answered");
        assert_eq!(
            answered["answered"], "remember",
            "the owner's word rides the reply, not the class the seat gave it"
        );
        assert!(answered["rule"].is_null(), "{answered}");
        assert!(
            answered["error"]
                .as_str()
                .is_some_and(|message| message.contains("cannot be remembered")),
            "{answered}"
        );
        assert!(
            bridge.state()["rules"]
                .as_array()
                .is_none_or(|rules| rules.is_empty()),
            "{}",
            bridge.state()["rules"]
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn the_core_events_land_ordered_and_the_state_replays_them() {
        let (bridge, sources, _door) = bridge().await;
        sources
            .ui
            .send(UiEvent::Routed {
                provider: "stepfun".to_string(),
                model: "step-5-preview".to_string(),
                how: "judged".to_string(),
                workload: Some("hard".to_string()),
                verdict: Some("pass".to_string()),
                judged_from: "fresh".to_string(),
                reason: "the orchestrator walks hard's chain".to_string(),
                order: vec![("stepfun".to_string(), "step-5-preview".to_string())],
            })
            .expect("the route lands");
        sources
            .ui
            .send(UiEvent::Usage {
                prompt_tokens: 1200,
                completion_tokens: 80,
            })
            .expect("the usage lands");
        sources
            .ui
            .send(UiEvent::Subagent {
                goal: "read the router".to_string(),
                session: "child-1".to_string(),
                stage: SubagentStage::Working {
                    calls: 2,
                    tool: "bash".to_string(),
                },
            })
            .expect("the delegation lands");
        sources
            .ui
            .send(UiEvent::Finished {
                stop_reason: "completed".to_string(),
            })
            .expect("the finish lands");
        for _ in 0..50 {
            if bridge.state()["events"].as_array().map(Vec::len) == Some(4) {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        let state = bridge.state();
        let events = state["events"].as_array().expect("an event log");
        assert_eq!(events.len(), 4, "{events:?}");
        assert_eq!(events[0]["type"], "routed");
        assert_eq!(events[0]["provider"], "stepfun");
        // The decision's own facts ride the event, whole: the page
        // reduces the live decision out of the log like every other
        // number rather than asking the router a second time.
        assert_eq!(events[0]["model"], "step-5-preview");
        assert_eq!(events[0]["workload"], "hard");
        assert_eq!(events[0]["judged_from"], "fresh");
        assert_eq!(events[0]["order"][0]["provider"], "stepfun");
        assert_eq!(events[1]["type"], "usage");
        assert_eq!(events[1]["prompt"], 1200);
        assert_eq!(events[2]["type"], "delegation");
        assert_eq!(events[2]["stage"], "working");
        assert_eq!(events[2]["detail"]["calls"], 2);
        assert_eq!(events[3]["type"], "finished");
        assert_eq!(state["running"], false, "the run ended");
        assert_eq!(state["budget"], crate::compaction::COMPACTION_TOKEN_BUDGET);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_failed_run_ends_the_running_flag() {
        let (bridge, sources, _door) = bridge().await;
        bridge.run("go").ok();
        sources
            .ui
            .send(UiEvent::Failed("routing config: no roster".to_string()))
            .expect("the failure lands");
        for _ in 0..50 {
            if !bridge.state()["running"].as_bool().unwrap_or(true) {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        let state = bridge.state();
        assert_eq!(state["running"], false);
        assert_eq!(state["events"][1]["type"], "failed");
    }
}
