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
use std::time::Instant;

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

/// One instruction from the browser to the agent thread. Two arms since
/// G6, where the TUI's two became this face's two as well: the sessions
/// panel can re-anchor the face to an archived log, which is what the
/// TUI's picker always did. The view side reseeds before the command is
/// sent, so a resumed face never shows the old conversation's tail.
pub(crate) enum Command {
    /// Run this task in this session. The session is part of the command
    /// because the thread drives more than one: a phone and a desktop
    /// may each be working in a different conversation.
    Run(String, String),
    /// Plan this task: the run plans and does not write, and its final
    /// message records as a plan the owner edits and builds.
    Plan(String, String),
    /// Re-anchor to this archived session's log. The next task replays
    /// it and continues it; the route is judged fresh, because the
    /// conversation the judgment serves is not the one that was judged.
    Resume(String),
    /// The session was rewound to this turn: the thread's prepared route
    /// is dropped, because the conversation it answered is gone.
    Rewind(u64),
}

/// The agent thread's inputs, as one struct because the thread's own
/// function already reads its seven neighbours and clippy counts
/// arguments.
struct AgentInputs {
    home: PathBuf,
    model: String,
    /// The session to log runs in, when one already exists — a resumed
    /// session. `None` is the launch case, and the log is created on the
    /// first run rather than at launch (KI-022).
    session_id: Option<String>,
    ui_tx: UnboundedSender<UiEvent>,
    approval_tx: UnboundedSender<ApprovalRequest>,
    command_rx: UnboundedReceiver<Command>,
    base_url: String,
    /// The corpus root the consolidator walks — the same root the
    /// browser's memory panel reads, resolved once by the bridge.
    memories_root: PathBuf,
    /// The shared face state, so the thread can name the session it is
    /// driving: the events it emits have to land in that session's log,
    /// and the thread is the only place that knows which one is running.
    state: Arc<Mutex<GuiState>>,
}

/// One attached session's live face: its ordered event log, whether a
/// run is in flight in it, and the approval it owes an answer to. A
/// session that has never been attached has no record here — the reads
/// that matter are the ones a surface has actually opened.
struct SessionState {
    events: VecDeque<Value>,
    /// How many events the cap pushed out the front. The page says the
    /// earlier ones are gone rather than implying it holds everything.
    dropped: usize,
    running: bool,
    /// The pending approval, with the responder the agent is blocked on.
    /// `None` when no call is waiting.
    seat: Option<ApprovalRequest>,
}

impl SessionState {
    fn new() -> SessionState {
        SessionState {
            events: VecDeque::new(),
            dropped: 0,
            running: false,
            seat: None,
        }
    }
}

/// The live face's shared truth: every attached session, the launch
/// session a request that names none gets, and the session the agent
/// thread is driving right now.
///
/// A session is a row in a table rather than the one thing the face
/// anchors, because a phone and a desktop are two surfaces over one
/// core and each may be looking at a different conversation. The launch
/// session is fixed at boot — it is the default a paired device gets
/// when it names nothing, and it does not move when the desktop resumes
/// somewhere else — and the driving session is the one whose events are
/// crossing the wire, which is exact because the thread's command loop
/// is serial.
struct GuiState {
    sessions: std::collections::HashMap<String, SessionState>,
    /// The launch session's id, once the thread has named it. `None`
    /// until then: the log is created on the first task (KI-022), so a
    /// face that is opened and closed without a prompt leaves no empty
    /// log behind.
    launch: Option<String>,
    driving: Option<String>,
}

impl GuiState {
    /// The record for `id`, created empty on first sight: a surface that
    /// opens a session has attached to it, and the events it then reads
    /// are the ones that arrive from here on.
    fn attached(&mut self, id: &str) -> &mut SessionState {
        self.sessions
            .entry(self.key(id))
            .or_insert_with(SessionState::new)
    }

    /// The record for `id` when it exists, without creating one.
    fn attached_if(&self, id: &str) -> Option<&SessionState> {
        self.sessions.get(&self.key(id))
    }

    /// The map key an addressed session reads: the empty string is the
    /// face's own launch session, which is the pending log before the
    /// thread names it and the named one after.
    fn key(&self, id: &str) -> String {
        if id.is_empty() {
            self.launch.clone().unwrap_or_default()
        } else {
            id.to_string()
        }
    }

    /// Where an event belongs: the session being driven, or the launch
    /// session when nothing is — which is where the core's own events
    /// (a cron fire, a skills load) land. Before the launch session has
    /// an id, that is the pending key: the log accumulates under it and
    /// moves when the thread names it.
    fn target(&self) -> String {
        self.driving
            .clone()
            .or_else(|| self.launch.clone())
            .unwrap_or_default()
    }

    /// The thread named the launch session's id: the pending log moves to
    /// the real key, so a face that was reading the pending key reads the
    /// same events under the id it will show.
    fn name_launch(&mut self, id: &str) {
        if let Some(pending) = self.sessions.remove("") {
            self.sessions.insert(id.to_string(), pending);
        }
        self.launch = Some(id.to_string());
    }
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
    /// The corpus root the browser's memory panel reads and the agent
    /// thread's consolidator walks, resolved once so the two cannot
    /// disagree about which corpus this face is looking at. The rule is
    /// the agent's own: [`crate::memory::resolve_root`] off the process's
    /// working directory.
    memories: PathBuf,
    /// The directory this face was launched from, which is the root the
    /// pane's tree and file reads are contained to. It is the process's
    /// own working directory rather than anything read from another
    /// harness's config — the own-home ruling, applied to a browser that
    /// can now ask for a file.
    cwd: PathBuf,
    state: Arc<Mutex<GuiState>>,
    rules: Arc<Mutex<approvals::Rules>>,
    events: broadcast::Sender<Value>,
    /// The welcome's one bounded read: the run archive, the corpus
    /// summary, the skill count, and the graph's next node, cached for
    /// half a minute so a reload is not a second walk of the machine.
    home_cache: Arc<Mutex<Option<(Instant, Value)>>>,
    /// The cross-kind related index: the sidecar for skills, projects,
    /// routes, and cells beside the two recall caches' facts and
    /// sessions. Shared rather than moved because its warm pass and its
    /// reads run on blocking workers.
    related: Arc<crate::gui::related::RelatedIndex>,
    /// The browser's own usage counters: what it accepts, opens, and
    /// uses. Counters only; the adaptation reads them and the settings
    /// knob switches it off.
    usage: Arc<crate::gui::usage::UsageStore>,
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
    pub fn detached(home: PathBuf, model: String, session: Option<String>) -> (Bridge, Sources) {
        let (ui_tx, ui_rx) = tokio::sync::mpsc::unbounded_channel();
        let (seat_tx, seat_rx) = tokio::sync::mpsc::unbounded_channel();
        let (events, _) = broadcast::channel(BROADCAST_DEPTH);
        // The launch session, when one already exists: a resumed face's
        // log. A launch starts with none — the agent thread creates the
        // log on the first task and names the id back through the event
        // stream (KI-022) — so the face's log lives under a pending key
        // that is not yet a session id, and moves when the id arrives.
        let launch = session;
        let mut sessions = std::collections::HashMap::new();
        sessions.insert(String::new(), SessionState::new());
        let state = Arc::new(Mutex::new(GuiState {
            sessions,
            launch,
            driving: None,
        }));
        let rules = Arc::new(Mutex::new(crate::settings::approval_rules(&home)));
        tokio::spawn(fan_out(ui_rx, seat_rx, Arc::clone(&state), events.clone()));
        let cwd = std::env::current_dir().unwrap_or_default();
        let memories = crate::memory::resolve_root(&cwd);
        let related = Arc::new(crate::gui::related::RelatedIndex::new(
            home.clone(),
            memories.clone(),
        ));
        let usage = Arc::new(crate::gui::usage::UsageStore::new(home.clone()));
        let bridge = Bridge {
            home,
            model,
            memories,
            cwd,
            state,
            rules,
            events,
            home_cache: Arc::new(Mutex::new(None)),
            related,
            usage,
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

    /// Point the memory panel at a corpus other than the one the working
    /// directory resolves. The production path never calls this — the
    /// rule is the agent's own, resolved once in [`Bridge::detached`] —
    /// so this exists for a test that must read a corpus it built rather
    /// than the machine's.
    #[cfg(test)]
    pub(crate) fn point_at_memories(&mut self, root: PathBuf) {
        self.memories = root;
    }

    /// Point the trust trio's tree at a directory other than the one the
    /// process was launched from. The production rule is the process's
    /// own working directory — the same root the tools write to — and
    /// a test that changed the process's cwd to honour it would be
    /// mutating a global every other test in the binary reads. The seam
    /// keeps the production rule out of the test's way instead.
    #[cfg(test)]
    pub(crate) fn point_at_cwd(&mut self, root: PathBuf) {
        self.cwd = root;
    }

    /// A bridge with the agent thread behind it: the thread this module
    /// spawns is the same shape `tui::spawn_agent` spawns, with the
    /// browser's prompt line and the seat's approver.
    pub fn spawn(
        home: PathBuf,
        model: String,
        session: Option<String>,
        base_url: String,
    ) -> Bridge {
        let (mut bridge, sources) = Bridge::detached(home.clone(), model.clone(), session.clone());
        let command_rx = bridge.connect();
        // The related index's cold half — every skill description and
        // every project's identity — is embedded once, off the request
        // path. A click that arrives first answers from the two recall
        // caches and names the index as warming.
        bridge.related.warm();
        // One resolution, shared: the consolidator the thread starts and
        // the browser's memory panel read the same corpus because they
        // are handed the same root, not because they each walked for one.
        let memories = bridge.memories.clone();
        let state = Arc::clone(&bridge.state);
        let handle = thread::spawn(move || {
            let _ = spawn_agent(AgentInputs {
                home,
                model,
                session_id: session,
                ui_tx: sources.ui,
                approval_tx: sources.seat,
                command_rx,
                base_url,
                memories_root: memories,
                state,
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

    /// The launch session's id: the default a request that names no
    /// session gets, fixed at boot so a paired device's requests have a
    /// stable answer whatever the desktop is looking at.
    pub fn launch(&self) -> String {
        lock(&self.state).launch.clone()
    }

    /// One session's live state as one document: the session, the model,
    /// the context budget the meter divides by, the ordered log, the
    /// seat, and the remembered rules. Everything the header derives —
    /// spend, context, route, turns, stop reason — the page reduces out
    /// of the log, so there is one derivation and not two.
    ///
    /// A session that has never been attached reads as empty rather than
    /// as an error: the archive's other sessions are listed by
    /// [`Bridge::sessions`], and this is the state of one the caller has
    /// actually opened.
    pub fn state(&self, session: &str) -> Value {
        let state = lock(&self.state);
        let rules = lock(&self.rules);
        let session = state.session.clone();
        // The gate the verify step names: the same knob the failover's
        // decision reads, never a second interpretation of it. It is one
        // small config file and the page asks for state once per load,
        // so it is read here rather than kept as another copy of a fact
        // the route module owns.
        let gate = crate::route::calibration_config(&self.home);
        // A session nobody has opened reads as empty rather than as an
        // error, and it reads the same keys the attached document does:
        // a surface that asks about one gets an answer it can render
        // without branching on the shape.
        let Some(attached) = state.attached_if(session) else {
            return json!({
                "session": session,
                "model": self.model,
                "workspace": self.cwd.display().to_string(),
                "budget": crate::compaction::COMPACTION_TOKEN_BUDGET,
                "gate": {
                    "name": "sys1",
                    "informs_failover": gate.informs_failover,
                },
                "running": false,
                "dropped": 0,
                "events": [],
                "approval": null,
                "rules": rules.list().iter().map(rule_value).collect::<Vec<_>>(),
                "attached": false,
            });
        };
        json!({
            "session": session,
            "model": self.model,
            "workspace": self.cwd.display().to_string(),
            "budget": crate::compaction::COMPACTION_TOKEN_BUDGET,
            "gate": {
                "name": "sys1",
                "informs_failover": gate.informs_failover,
            },
            "running": attached.running,
            "dropped": attached.dropped,
            "events": attached.events,
            "approval": attached.seat.as_ref().map(|seat| {
                json!({
                    "tool": seat.call.function.name,
                    "summary": seat.summary,
                    "label": approvals::Rule::for_call(&seat.call).label(),
                    "allowlistable": approvals::allowlistable(&seat.call),
                })
            }),
            "rules": rules.list().iter().map(rule_value).collect::<Vec<_>>(),
            "attached": true,
        })
    }

    /// The welcome's document: the four questions the home answers, in
    /// one bounded read. It is assembled from the same panel reads, so
    /// the home and a panel cannot disagree, and cached for half a minute
    /// so a reload or a return to the conversation does not walk the
    /// corpus, the library, and every session log again.
    ///
    /// Nothing here invents a number. The run archive, the corpus, the
    /// library, and the graph are the reads they claim to be; a read that
    /// refused rides its reason in `refused` and the client keeps the
    /// last good value beside it.
    pub async fn home(&self) -> Value {
        const HOME_TTL: std::time::Duration = std::time::Duration::from_secs(30);
        {
            let cache = lock(&self.home_cache);
            if let Some((at, document)) = cache.as_ref() {
                if at.elapsed() < HOME_TTL {
                    return document.clone();
                }
            }
        }

        let state = self.state();
        // The four reads are independent; run them together so the
        // welcome's first paint pays the slowest one, not their sum.
        let (sessions, memory, skills, delegation) = tokio::join!(
            self.sessions(),
            self.memory(),
            self.skills(),
            self.delegation()
        );

        let runs = sessions
            .get("sessions")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let recent: Vec<Value> = runs.iter().take(3).cloned().collect();
        let timeline: Vec<Value> = runs
            .iter()
            .map(|row| {
                json!({
                    "id": row.get("id").cloned().unwrap_or(Value::Null),
                    "created": row.get("created").cloned().unwrap_or(Value::Null),
                    "kind": row.get("kind").cloned().unwrap_or(Value::Null),
                    "ended": row.get("ended").cloned().unwrap_or(Value::Null),
                })
            })
            .collect();

        let facts = memory
            .get("facts")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let live: Vec<&Value> = facts
            .iter()
            .filter(|fact| fact.get("retired").and_then(Value::as_bool) != Some(true))
            .collect();
        let protected = live
            .iter()
            .filter(|fact| fact.get("protected").and_then(Value::as_bool) == Some(true))
            .count();
        let mut named: Vec<&Value> = live.clone();
        named.sort_by_key(|fact| {
            std::cmp::Reverse(
                fact.get("recall_count")
                    .and_then(Value::as_u64)
                    .unwrap_or(0),
            )
        });
        let top: Vec<Value> = named
            .iter()
            .take(2)
            .map(|fact| {
                json!({
                    "name": fact.get("name").cloned().unwrap_or(Value::Null),
                    "recalls": fact.get("recall_count").cloned().unwrap_or(Value::Null),
                })
            })
            .collect();
        let dates: Vec<Value> = live
            .iter()
            .filter_map(|fact| fact.get("date").cloned())
            .collect();

        let skill_count = skills
            .get("skills")
            .and_then(Value::as_array)
            .map(|rows| rows.len())
            .unwrap_or(0);
        let proposals = skills
            .get("proposals")
            .and_then(Value::as_array)
            .map(|rows| rows.len())
            .unwrap_or(0);

        let nodes = delegation
            .get("graph")
            .and_then(|graph| graph.get("nodes"))
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let mut ready: Vec<&Value> = nodes
            .iter()
            .filter(|node| node.get("state").and_then(Value::as_str) == Some("ready"))
            .collect();
        ready.sort_by_key(|node| {
            node.get("score")
                .and_then(Value::as_i64)
                .unwrap_or(i64::MAX)
        });
        let running = nodes
            .iter()
            .find(|node| node.get("state").and_then(Value::as_str) == Some("running"));
        let next = running.or_else(|| ready.first().copied());
        let next = next.map(|node| {
            json!({
                "id": node.get("id").cloned().unwrap_or(Value::Null),
                "state": node.get("state").cloned().unwrap_or(Value::Null),
                "approach": node.get("approach").cloned().unwrap_or(Value::Null),
                "score": node.get("score").cloned().unwrap_or(Value::Null),
                "peak": node.get("peak").cloned().unwrap_or(Value::Null),
            })
        });

        let mut refused: Vec<Value> = Vec::new();
        for document in [&sessions, &memory, &skills, &delegation] {
            if let Some(rows) = document.get("refused").and_then(Value::as_array) {
                refused.extend(rows.iter().cloned());
            }
        }

        let document = json!({
            "type": "home",
            "workspace": state.get("workspace").cloned().unwrap_or(Value::Null),
            "session": state.get("session").cloned().unwrap_or(Value::Null),
            "model": state.get("model").cloned().unwrap_or(Value::Null),
            "runs": {
                "count": runs.len(),
                "recent": recent,
                "timeline": timeline,
            },
            "memory": {
                "live": live.len(),
                "protected": protected,
                "top": top,
                "dates": dates,
                "day": memory.get("day").cloned().unwrap_or(Value::Null),
            },
            "skills": {
                "count": skill_count,
                "proposals": proposals,
            },
            "next": next,
            "refused": refused,
        });
        *lock(&self.home_cache) = Some((Instant::now(), document.clone()));
        document
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
    pub async fn delegation(&self, session: &str) -> Value {
        let home = self.home.clone();
        let session = session.to_string();
        match tokio::task::spawn_blocking(move || delegation_read(&home, &session)).await {
            Ok(document) => document,
            Err(_) => json!({
                "type": "delegation",
                "refused": ["the delegation read did not land"],
            }),
        }
    }

    /// The memory panel's document: the corpus, the day's recall hits,
    /// and the corpus root named.
    ///
    /// The live run's own recall is not here. It is already in the event
    /// log as its `memory` card, and the panel reduces it off the log the
    /// way the router panel reduces its own decision — one source per
    /// picture, not two.
    pub async fn memory(&self) -> Value {
        let home = self.home.clone();
        let memories = self.memories.clone();
        match tokio::task::spawn_blocking(move || memory_read(&home, &memories)).await {
            Ok(document) => document,
            Err(_) => json!({
                "type": "memory",
                "refused": ["the memory read did not land"],
            }),
        }
    }

    /// The skills panel's document: the library with its skips, the
    /// proposals awaiting a decision, and the curation candidates.
    pub async fn skills(&self) -> Value {
        let home = self.home.clone();
        match tokio::task::spawn_blocking(move || skills_read(&home)).await {
            Ok(document) => document,
            Err(_) => json!({
                "type": "skills",
                "refused": ["the skills read did not land"],
            }),
        }
    }

    /// The sessions panel's document: the archive as the summaries the
    /// log heads carry, newest first, with the session this face
    /// anchors named so the panel can say which row is live.
    pub async fn sessions(&self, current: &str) -> Value {
        let home = self.home.clone();
        let current = current.to_string();
        match tokio::task::spawn_blocking(move || sessions_read(&home, &current)).await {
            Ok(document) => document,
            Err(_) => json!({
                "type": "sessions",
                "refused": ["the archive read did not land"],
            }),
        }
    }

    /// One archived session's replay, as the cards its records shape
    /// into — the same builder the resume reseeds from, so what the
    /// panel previews is byte-for-byte what a resumed conversation
    /// starts from.
    pub async fn replay(&self, asked: &str) -> Value {
        let home = self.home.clone();
        let asked = asked.to_string();
        match tokio::task::spawn_blocking(move || replay_read(&home, &asked)).await {
            Ok(document) => document,
            Err(_) => json!({
                "type": "replay",
                "refused": ["the replay read did not land"],
            }),
        }
    }

    /// The calibration panel's document: the day's stream rows grouped
    /// by op, the judged quality's fit cells with the readiness the fit
    /// rule computes, and the failover gate's state named rather than
    /// left to be inferred.
    pub async fn calibration(&self) -> Value {
        let home = self.home.clone();
        match tokio::task::spawn_blocking(move || calibration_read(&home)).await {
            Ok(document) => document,
            Err(_) => json!({
                "type": "calibration",
                "refused": ["the calibration read did not land"],
            }),
        }
    }

    /// The tower console's document: the git facts per project from the
    /// tower board's own read, and the second-brain's reports and tower
    /// aggregates beside them. Two reads, one document, and every number
    /// traces to one of them. A refused part lands as its named reason
    /// while the rest still reports — the console keeps its last good
    /// numbers and the reason rides along, so a read that did not land
    /// is a banner, not a blank screen.
    pub async fn tower(&self) -> Value {
        let home = self.home.clone();
        match tokio::task::spawn_blocking(move || tower_read(&home)).await {
            Ok(document) => document,
            Err(_) => json!({
                "type": "tower",
                "refused": ["the tower read did not land"],
            }),
        }
    }

    /// The settings panel's document: every knob with its source, the
    /// two list surfaces and the rules as the third, the env status by
    /// name, the cron jobs, and the SOUL.md slot. Every value rides the
    /// readers the CLI reads, so the browser and `fleet settings`
    /// cannot disagree about what the home holds.
    pub async fn settings(&self) -> Value {
        let home = self.home.clone();
        match tokio::task::spawn_blocking(move || settings_read(&home)).await {
            Ok(document) => document,
            Err(_) => json!({
                "type": "settings",
                "refused": ["the settings read did not land"],
            }),
        }
    }

    /// One settings write, through the writer the surface owns, and the
    /// refreshed document back: one round-trip, so the panel never
    /// paints a state the writer did not confirm. A refusal names
    /// itself the way the writer's own refusal does — the GUI is a
    /// client of these functions, never a second opinion about them.
    pub async fn settings_write(&self, body: &str) -> Result<Value, String> {
        let arguments: serde_json::Value = serde_json::from_str(body)
            .map_err(|error| format!("the write is not a JSON object: {error}"))?;
        let op = arguments
            .get("op")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| "a settings write names its op".to_string())?
            .to_string();
        let home = self.home.clone();
        // The write runs off the async worker: every one of these is a
        // file write, and a page request must not hold the runtime
        // while the disk answers.
        let op_for_reply = op.clone();
        let landed = tokio::task::spawn_blocking(move || settings_apply(&home, &op, &arguments))
            .await
            .map_err(|_| "the settings write did not land".to_string())??;
        Ok(json!({ "type": "settings_write", "op": op_for_reply, "wrote": landed }))
    }

    /// The second-brain panel's document: the six derived state files,
    /// each section landed or named. The same read the TUI's brain board
    /// rides, shaped for the browser.
    pub async fn brain(&self) -> Value {
        let home = self.home.clone();
        match tokio::task::spawn_blocking(move || brain_read(&home)).await {
            Ok(document) => document,
            Err(_) => json!({
                "type": "brain",
                "refused": ["the second-brain read did not land"],
            }),
        }
    }

    /// One in-place edit of a fact, through the substrate's own rewriter.
    ///
    /// The body is the arguments object the browser handed over. A
    /// refusal rides the reply as its reason rather than a bare status,
    /// because the reason is the part the owner needs: `no fact named …`,
    /// or `… was folded into …`.
    pub async fn fact_edit(&self, body: &str) -> Result<Value, String> {
        let arguments: serde_json::Value = serde_json::from_str(body)
            .map_err(|error| format!("the edit is not a JSON object: {error}"))?;
        let request =
            crate::memory::FactEdit::parse(&arguments).map_err(|error| error.to_string())?;
        let root = self.memories.clone();
        match tokio::task::spawn_blocking(move || crate::memory::edit(&root, &request)).await {
            Ok(Ok(fact)) => Ok(json!({
                "type": "fact",
                "edited": true,
                "fact": {
                    "name": fact.name,
                    "description": fact.description,
                    "fact_type": fact.fact_type,
                    "tier": fact.tier.label(),
                    "basis": fact.basis.label(),
                    "recall_count": fact.recall_count,
                    "valid_until": fact.valid_until,
                    "shape": fact.shape,
                    "phrases": fact.phrases,
                    "missing": fact.missing,
                    "body": fact.body,
                    "path": fact.path.display().to_string(),
                },
            })),
            Ok(Err(error)) => Err(error.to_string()),
            Err(_) => Err("the edit did not land".to_string()),
        }
    }

    /// Approve one skills proposal through the apply path.
    ///
    /// The click is the approval, and the gate is the real one: the same
    /// four cargo steps `fleet improve apply` runs, in the same order, on
    /// the ambient toolchain, with the change kept only when every step
    /// is green. A skills write is home-side, so it creates no commit —
    /// the pre-image stash beside it is what makes it restorable, by
    /// `fleet improve undo`.
    ///
    /// Only a skills proposal is approved here. A proposal whose target
    /// is the source or the config is refused by name, because this is
    /// the skills panel's approve and the eval-corpus step that grades a
    /// source edit is a CLI step, not a button.
    pub async fn skill_apply(&self, name: &str) -> Result<Value, String> {
        let name = name.trim();
        if name.is_empty() {
            return Err("an approve needs a proposal name".to_string());
        }
        let home = self.home.clone();
        let asked = name.to_string();
        let applied_name = name.to_string();
        let verdict = tokio::task::spawn_blocking(move || {
            // The target is read before anything is staged, so a
            // proposal this surface does not own never reaches the gate.
            match crate::improve::load_one(&home, &asked) {
                Some(proposal) if proposal.target == crate::improve::Target::Skill => {}
                Some(proposal) => {
                    return crate::improve::Verdict::Refused {
                        reason: format!(
                            "`{}` targets the {}, which is not a skill; approve it from the CLI, where its gate is the one it needs",
                            proposal.name,
                            proposal.target.label()
                        ),
                    }
                }
                None => {
                    return crate::improve::Verdict::Refused {
                        reason: format!("no proposal named `{asked}` is awaiting a decision"),
                    }
                }
            }
            let repo = std::env::current_dir().unwrap_or_default();
            let user_home = std::env::var("HOME").ok();
            let stream = crate::calib::Stream::open(crate::calib::resolve_root(
                &home,
                user_home.as_deref(),
            ));
            let gate = crate::improve::Gate::local(&repo);
            let Some(canonical) = crate::skills::resolve_skills_dir(
                std::env::var("FLEET_SKILLS_DIR").ok().as_deref(),
                user_home.as_deref(),
            ) else {
                return crate::improve::Verdict::Refused {
                    reason: "no skills library resolvable \u{2014} set FLEET_SKILLS_DIR or HOME"
                        .to_string(),
                };
            };
            // The eval-corpus probe is deliberately unwired: it grades a
            // source edit, and a source target is refused above.
            let ctx = crate::improve::Apply {
                home: &home,
                repo: &repo,
                gate: &gate,
                approved: true,
                stream: Some(&stream),
                eval: None,
                skills_dir: &canonical,
            };
            crate::improve::apply(&asked, &ctx)
        })
        .await
        .map_err(|_| "the approve did not land".to_string())?;
        match verdict {
            crate::improve::Verdict::Applied { summary, commit } => Ok(json!({
                "type": "skill",
                "verdict": "applied",
                "name": applied_name,
                "summary": summary,
                "commit": commit,
            })),
            crate::improve::Verdict::Refused { reason } => Err(reason),
            crate::improve::Verdict::Waiting { reason } => Err(reason),
        }
    }

    /// Rank `candidates` against `asked` by cosine, for the composer's
    /// command type-ahead.
    ///
    /// The candidates are the browser's own command declarations — the
    /// actions only the page can perform — and this ranks them, so the
    /// command set lives in one place and the ranking is the
    /// substrate's. A machine with no embedding backend answers with
    /// every score zero rather than refusing: the type-ahead falls back
    /// to its lexical ranking, which is the honest degradation and not an
    /// error the owner has to know about.
    pub async fn rank(&self, asked: &str, candidates: &[String]) -> Value {
        let asked = asked.trim().to_string();
        let candidates = candidates.to_vec();
        let ranked = tokio::task::spawn_blocking(move || {
            // KI-233: the probed answer, not a fresh probe. `probe` is a
            // real round trip on its own budget, so calling it here paid
            // the probe's five-second connect budget per request on a
            // machine with no embedding backend — exactly the cost
            // KI-064 removed from the corpus path, reintroduced in the
            // browser path. `probed_embedder` is the same answer the
            // corpus loads take, once per process, and a keyword-only
            // answer still degrades to the lexical ranking exactly as
            // before.
            let embedder = crate::memory::probed_embedder();
            let query = embedder.embed(&asked);
            // A machine with no embedding backend answers with every
            // score zero rather than refusing. An empty list would be a
            // refusal by another name — the browser would have nothing
            // to order — so the candidates all ride through at zero and
            // the type-ahead's own lexical pass decides.
            candidates
                .iter()
                .enumerate()
                .map(|(index, text)| {
                    let score = match &query {
                        Some(query) => embedder
                            .embed(text)
                            .map(|vector| crate::memory::cosine(query, &vector))
                            .unwrap_or(0.0),
                        None => 0.0,
                    };
                    json!({"index": index, "score": score})
                })
                .collect::<Vec<_>>()
        })
        .await
        .unwrap_or_default();
        let mut ranked = ranked;
        ranked.sort_by(|left, right| {
            right["score"]
                .as_f64()
                .unwrap_or(0.0)
                .partial_cmp(&left["score"].as_f64().unwrap_or(0.0))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        json!({"type": "rank", "ranked": ranked})
    }

    /// The inspector's related suggestions: one selected object, the
    /// other objects nearest it across every kind, by the related
    /// index's own read. Off the async worker because it touches the
    /// corpus, the session archive, and the sidecar — and because a
    /// cold corpus may embed a route row.
    pub async fn related(&self, kind: &str, id: &str, limit: usize) -> Value {
        let index = Arc::clone(&self.related);
        let kind = kind.to_string();
        let id = id.to_string();
        let failure = json!({
            "type": "related",
            "kind": kind.clone(),
            "id": id.clone(),
            "refused": ["the related read did not land"],
        });
        match tokio::task::spawn_blocking(move || index.related(&kind, &id, limit)).await {
            Ok(document) => document,
            Err(_) => failure,
        }
    }

    /// The browser's usage counters and whether the adaptation is on.
    /// A read of the GUI's own file plus the one settings knob, never a
    /// look at any text the owner wrote.
    pub fn usage(&self) -> Value {
        self.usage.document(crate::gui::usage::adapt(&self.home))
    }

    /// Record one counter and answer with the whole document, so the
    /// page's ranking state is the store's state rather than a local
    /// guess. The write is off the async worker; a refused event is the
    /// store's own sentence.
    pub async fn record_usage(
        &self,
        section: &str,
        id: &str,
        event: &str,
    ) -> Result<Value, String> {
        let store = Arc::clone(&self.usage);
        let section = section.to_string();
        let id = id.to_string();
        let event = event.to_string();
        let adapt = crate::gui::usage::adapt(&self.home);
        match tokio::task::spawn_blocking(move || {
            store.record(&section, &id, &event)?;
            Ok::<Value, String>(store.document(adapt))
        })
        .await
        {
            Ok(result) => result,
            Err(_) => Err("the usage write did not land".to_string()),
        }
    }

    /// The theme set's measured contrast, for the action card that offers
    /// a readable alternative when the current theme does not clear the
    /// floor. A measurement, not an opinion — the card offers a number
    /// the browser can check.
    pub fn contrast(&self) -> Value {
        let reports = crate::gui::themes::measure_all();
        json!({
            "type": "contrast",
            "floor": crate::gui::themes::MIN_TEXT,
            "themes": reports.iter().map(|report| json!({
                "theme": report.theme,
                "worst": report.worst,
                "holds": report.holds(),
                "failing": report.failing.iter().map(|(token, ratio)| json!({
                    "token": token, "ratio": ratio
                })).collect::<Vec<_>>(),
                "line": report.line(),
            })).collect::<Vec<_>>(),
        })
    }

    /// The pane's web read: fetch one page and reduce it to what is
    /// readable. The guard is the substrate's — a private or loopback
    /// address is refused, so a browser that can name a URL cannot name
    /// the embedding model or the cloud metadata service.
    pub async fn web(&self, url: &str) -> Value {
        let url = url.trim().to_string();
        match tokio::task::spawn_blocking(move || crate::web::fetch(&url)).await {
            Ok(Ok(page)) => {
                let readable = crate::web::readable(&page.body);
                json!({
                    "type": "web",
                    "url": page.url,
                    "status": page.status,
                    "content_type": page.content_type,
                    "title": readable.title,
                    "blocks": readable.blocks.iter().map(|block| match block {
                        crate::web::Block::Heading { level, text } => json!({
                            "kind": "heading", "level": level, "text": text
                        }),
                        crate::web::Block::Paragraph(text) => json!({
                            "kind": "paragraph", "text": text
                        }),
                        crate::web::Block::Item(text) => json!({
                            "kind": "item", "text": text
                        }),
                        crate::web::Block::Code(text) => json!({
                            "kind": "code", "text": text
                        }),
                        crate::web::Block::Quote(text) => json!({
                            "kind": "quote", "text": text
                        }),
                    }).collect::<Vec<_>>(),
                    "links": readable.links.iter().map(|(label, href)| json!({
                        "label": label, "href": href
                    })).collect::<Vec<_>>(),
                })
            }
            Ok(Err(reason)) => json!({"type": "web", "refused": [reason.to_string()]}),
            Err(_) => json!({"type": "web", "refused": ["the fetch did not land"]}),
        }
    }

    /// The pane's tree read, off the working directory.
    pub async fn tree(&self, asked: &str) -> Value {
        let root = self.cwd.clone();
        let asked = asked.to_string();
        match tokio::task::spawn_blocking(move || tree_read(&root, &asked)).await {
            Ok(document) => document,
            Err(_) => json!({"type": "tree", "refused": ["the tree read did not land"]}),
        }
    }

    /// The pane's file read, off the working directory.
    pub async fn file(&self, asked: &str) -> Value {
        let root = self.cwd.clone();
        let asked = asked.to_string();
        match tokio::task::spawn_blocking(move || file_read(&root, &asked)).await {
            Ok(document) => document,
            Err(_) => json!({"type": "file", "refused": ["the file read did not land"]}),
        }
    }

    /// Hand the browser's task to the agent thread as a plan-mode run:
    /// the same door, the plan arm. The owner's message lands in the log
    /// first, exactly as a run's does, so a refresh replays what was
    /// asked.
    pub fn plan(&self, session: &str, task: &str) -> Result<(), String> {
        self.dispatch(session, task, |session, task| {
            Command::Plan(session.to_string(), task.to_string())
        })
    }

    /// Hand the browser's task to the agent thread. The owner's message
    /// lands in the log first, so a refresh replays what was asked and
    /// the header's running flag moves on the same event.
    pub fn run(&self, session: &str, task: &str) -> Result<(), String> {
        self.dispatch(session, task, |session, task| {
            Command::Run(session.to_string(), task.to_string())
        })
    }

    /// The owner's one instruction to the run in flight, through the
    /// control file the agent's turn loop reads before every model call:
    /// a stop, or a steer that rides exactly one turn. `None` steers
    /// nothing and stops; `Some(text)` steers and does not stop.
    ///
    /// Both refusals are named rather than silent: a control with no run
    /// in flight would sit until some later run read it, and a control
    /// for a session whose log does not exist is a file nothing reads.
    /// The event lands in the ordered log as its own `control` card, so
    /// a refresh replays the owner's intervention rather than losing it.
    pub fn stop(&self, steer: Option<&str>) -> Result<Value, String> {
        let steer = match steer {
            Some(text) => {
                let text = text.trim();
                if text.is_empty() {
                    return Err("a steer needs some text".to_string());
                }
                Some(text)
            }
            None => None,
        };
        let session = {
            let state = lock(&self.state);
            if !state.running {
                return Err("no run is in flight to stop or steer".to_string());
            }
            if state.session.is_empty() {
                return Err(
                    "this run has no session log yet; there is nothing beside it to write"
                        .to_string(),
                );
            }
            state.session.clone()
        };
        crate::delegate::write_control(&self.home, &session, steer)?;
        let kind = if steer.is_some() { "steer" } else { "stop" };
        let value = json!({
            "type": "control",
            "kind": kind,
            "session": session,
            "steer": steer,
        });
        record(&self.state, &self.events, value.clone());
        Ok(value)
    }

    /// The shared door: the task validated, the flag set, the owner's
    /// message logged, and the command handed to the thread. One body
    /// for both arms, so a plan-mode run and an ordinary one cannot
    /// disagree about what happens before the agent sees the task.
    fn dispatch(
        &self,
        session: &str,
        task: &str,
        command: impl FnOnce(&str, &str) -> Command,
    ) -> Result<(), String> {
        let task = task.trim();
        if task.is_empty() {
            return Err("a run needs a task".to_string());
        }
        {
            let mut state = lock(&self.state);
            let attached = state.attached(session);
            if attached.running {
                return Err("a run is already in flight; let it finish first".to_string());
            }
            attached.running = true;
        }
        record(
            &self.state,
            &self.events,
            session,
            json!({"type": "message", "role": "owner", "text": task}),
        );
        let delivered = match self.commands.as_ref() {
            Some(commands) => commands.send(command(session, task)).is_ok(),
            None => false,
        };
        if !delivered {
            // The command had nowhere to go, so the run never started:
            // the flag comes back and the log says the run failed, which
            // is what the owner's browser must show rather than a face
            // that says "running" forever.
            {
                let mut state = lock(&self.state);
                state.attached(session).running = false;
            }
            record(
                &self.state,
                &self.events,
                session,
                json!({"type": "failed", "reason": "this face has no agent behind it"}),
            );
            return Err("this face has no agent behind it".to_string());
        }
        Ok(())
    }

    /// Attach this face to an archived session's log: the sessions
    /// panel's resume. The session's record is built from the log's own
    /// records — the same cards a replay previews — and the agent thread
    /// is told to continue that log on its next task, its route judgment
    /// dropped so the resumed conversation is judged as its own. Other
    /// attached sessions are untouched: a phone looking at another
    /// conversation keeps looking at it. A run in flight in THIS session
    /// refuses, a name that is not a session id is refused for its shape
    /// before the disk is touched, and a name that does not open is
    /// refused by naming itself — a failed resume leaves the conversation
    /// exactly as it was.
    pub async fn resume(&self, asked: &str) -> Result<Value, String> {
        let id = asked.trim();
        if !session_id_is_shape(id) {
            return Err(format!("`{id}` is not a session id"));
        }
        {
            let state = lock(&self.state);
            if state
                .attached_if(id)
                .map(|attached| attached.running)
                .unwrap_or(false)
            {
                return Err(
                    "a run is in flight in that session; let it finish before resuming".to_string(),
                );
            }
        }
        let home = self.home.clone();
        let owned = id.to_string();
        let seeded = match tokio::task::spawn_blocking(move || replay_cards(&home, &owned)).await {
            Ok(Ok(seeded)) => seeded,
            Ok(Err(reason)) => return Err(reason),
            Err(_) => return Err("the replay did not land".to_string()),
        };
        {
            let mut state = lock(&self.state);
            let attached = state.attached(id);
            attached.events = seeded.into();
            attached.dropped = 0;
            attached.running = false;
            attached.seat = None;
        }
        // The thread learns the new attachment even when it has nothing
        // to run yet: the reply says whether the command landed, because
        // a face with no agent behind it still resumes as a viewer — the
        // reads are all view-side — but its next run would say so
        // anyway, and the reply saying it twice would be a lie.
        let delivered = self
            .commands
            .as_ref()
            .map(|commands| commands.send(Command::Resume(id.to_string())).is_ok())
            .unwrap_or(false);
        record(
            &self.state,
            &self.events,
            id,
            json!({"type": "resumed", "session": id, "agent": delivered}),
        );
        Ok(json!({"type": "resume", "session": id, "agent": delivered}))
    }

    /// Rewind the session to a turn's checkpoint: the tree is restored
    /// to the state that turn left, the log gains the rewind record the
    /// next reseed cuts at, the face reseeds from the log before it, and
    /// the thread drops its prepared route. A run in flight refuses —
    /// rewinding a tree the run is still writing would be a race, not a
    /// rewind.
    pub async fn rewind(&self, session: &str, turn: u64) -> Result<Value, String> {
        {
            let state = lock(&self.state);
            if state
                .attached_if(session)
                .map(|attached| attached.running)
                .unwrap_or(false)
            {
                return Err("a run is in flight; let it finish before rewinding".to_string());
            }
        }
        let home = self.home.clone();
        let session = session.to_string();
        let root = self.cwd.clone();
        let store = home.join("checkpoints");
        let in_closure = session.clone();
        let landed = tokio::task::spawn_blocking(move || {
            let (restored, removed) = crate::checkpoints::restore(&root, &store, &in_closure, turn)
                .map_err(|error| format!("the rewind could not restore the tree: {error}"))?;
            // The record rides the log the session already anchors, in
            // append mode: a log is append-only, so the abandoned future
            // stays and the reseed cuts at this record.
            let path = session::session_path(&home, &in_closure);
            let mut log = Session::open(&path)
                .map_err(|error| format!("the session log could not be opened: {error}"))?;
            log.record(&session::Record::Rewind {
                turn,
                restored: restored as u64,
                removed: removed as u64,
            })
            .map_err(|error| format!("the rewind could not be recorded: {error}"))?;
            Ok::<(usize, usize), String>((restored, removed))
        })
        .await
        .map_err(|_| "the rewind did not land".to_string())??;
        // The face reseeds from the log before the cut, exactly as a
        // resume reseeds: one translator, so what the browser shows is
        // what the next task seeds from.
        let home = self.home.clone();
        let owned = session.clone();
        let cards = tokio::task::spawn_blocking(move || {
            let path = session::session_path(&home, &owned);
            let records = session::replay(&path).map_err(|error| error.to_string())?;
            let cut = session::rewind_cut(&records);
            Ok::<Vec<Value>, String>(cards_from_records(cut).0)
        })
        .await
        .map_err(|_| "the reseed did not land".to_string())??;
        {
            let mut state = lock(&self.state);
            let attached = state.attached(&session);
            attached.events = cards.into();
            attached.dropped = 0;
            attached.running = false;
            attached.seat = None;
        }
        if let Some(commands) = self.commands.as_ref() {
            let _ = commands.send(Command::Rewind(turn));
        }
        record(
            &self.state,
            &self.events,
            &session,
            json!({
                "type": "rewound",
                "turn": turn,
                "restored": landed.0,
                "removed": landed.1,
            }),
        );
        Ok(json!({"type": "rewind", "turn": turn, "restored": landed.0, "removed": landed.1}))
    }

    /// The checkpoints this session's store holds, newest first: the
    /// rewind affordance reads it, and each entry names the turn and
    /// what the capture reached.
    pub async fn checkpoints(&self, session: &str) -> Value {
        let home = self.home.clone();
        let session = session.to_string();
        let store = home.join("checkpoints");
        match tokio::task::spawn_blocking(move || crate::checkpoints::list(&store, &session)).await
        {
            Ok(snapshots) => json!({
                "type": "checkpoints",
                "snapshots": snapshots.iter().map(|snapshot| json!({
                    "turn": snapshot.turn,
                    "at": snapshot.at,
                    "files": snapshot.files,
                    "bytes": snapshot.bytes,
                    "unread": snapshot.unread,
                })).collect::<Vec<_>>(),
            }),
            Err(_) => {
                json!({"type": "checkpoints", "refused": ["the checkpoint read did not land"]})
            }
        }
    }

    /// The run's changes against a checkpoint's turn: the diff review's
    /// document. The turn is the checkpoint to measure against; the
    /// face offers the turns the store holds.
    pub async fn diff(&self, session: &str, turn: u64) -> Value {
        let home = self.home.clone();
        let session = session.to_string();
        let root = self.cwd.clone();
        let store = home.join("checkpoints");
        match tokio::task::spawn_blocking(move || {
            crate::checkpoints::diff(&root, &store, &session, turn)
        })
        .await
        {
            Ok(Ok(files)) => json!({
                "type": "diff",
                "turn": turn,
                "files": files.iter().map(diff_file_json).collect::<Vec<_>>(),
            }),
            Ok(Err(error)) => json!({
                "type": "diff",
                "refused": [format!("the diff could not be read: {error}")],
            }),
            Err(_) => json!({"type": "diff", "refused": ["the diff did not land"]}),
        }
    }

    /// Reject one hunk, or a whole file when the hunk is the file's own:
    /// the checkpoint's lines go back where the run's lines went. The
    /// hunk's coordinate rides the request, so a reject is the reverse
    /// of exactly what that hunk changed and nothing else.
    pub async fn diff_reject(&self, session: &str, body: &str) -> Result<Value, String> {
        let arguments: serde_json::Value = serde_json::from_str(body)
            .map_err(|error| format!("the reject is not a JSON object: {error}"))?;
        let turn = arguments
            .get("turn")
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| "a reject names the checkpoint's turn".to_string())?;
        let path = arguments
            .get("path")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| "a reject names the file".to_string())?
            .to_string();
        let hunk = arguments.get("hunk").and_then(serde_json::Value::as_u64);
        let owned = path.clone();
        let home = self.home.clone();
        let session = session.to_string();
        let root = self.cwd.clone();
        let store = home.join("checkpoints");
        tokio::task::spawn_blocking(move || {
            if let Some(index) = hunk {
                let files = crate::checkpoints::diff(&root, &store, &session, turn)
                    .map_err(|error| format!("the diff could not be read: {error}"))?;
                let file = files
                    .iter()
                    .find(|file| file.path == owned)
                    .ok_or_else(|| format!("`{owned}` carries no change against turn {turn}"))?;
                let hunk = file
                    .hunks
                    .get(index as usize)
                    .ok_or_else(|| format!("`{owned}` carries no hunk {index}"))?;
                crate::checkpoints::reject_hunk(&root, file, hunk)
                    .map_err(|error| format!("the hunk could not be rejected: {error}"))?;
            } else {
                crate::checkpoints::reject_file(&root, &store, &session, turn, &owned)
                    .map_err(|error| format!("the file could not be rejected: {error}"))?;
            }
            Ok::<(), String>(())
        })
        .await
        .map_err(|_| "the reject did not land".to_string())??;
        Ok(json!({"type": "diff_reject", "turn": turn, "path": path, "hunk": hunk}))
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
    pub fn answer(&self, session: &str, decision: &str) -> Result<Value, AnswerError> {
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
            state
                .attached(session)
                .seat
                .take()
                .ok_or(AnswerError::NoSeat)?
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
    // The vault laid over the dotenv: a sealed key answers here exactly
    // where a plaintext key would have, so the router and the agent see
    // the same credentials either way.
    let passphrase = std::env::var("FLEET_VAULT_PASSPHRASE").ok();
    let dotenv = crate::home::read_dotenv_with_vault(home, passphrase.as_deref());
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

/// The memory panel's document, hand-built from the read's snapshot:
/// memory.rs carries no serde derives, so the shapes are translated here
/// rather than derived twice.
fn memory_read(home: &std::path::Path, memories: &std::path::Path) -> Value {
    let user_home = std::env::var("HOME").ok();
    let calib_root = crate::calib::resolve_root(home, user_home.as_deref());
    let snapshot = crate::boards::read_memory(memories, &calib_root, &crate::calib::today());
    json!({
        "type": "memory",
        "read_at": snapshot.read_at,
        "day": snapshot.day,
        "root": snapshot.root,
        "facts": snapshot.facts.iter().map(|fact| json!({
            "name": fact.name,
            "description": fact.description,
            "fact_type": fact.fact_type,
            "basis": fact.basis,
            "tier": fact.tier,
            "date": fact.date,
            "recall_count": fact.recall_count,
            "protected": fact.protected,
            "retired": fact.retired,
            "superseded_by": fact.superseded_by,
            "valid_until": fact.valid_until,
            "shape": fact.shape,
            "phrases": fact.phrases,
            "missing": fact.missing,
            "body": fact.body,
            "path": fact.path,
        })).collect::<Vec<_>>(),
        "hits": snapshot.hits.iter().map(|hit| json!({
            "ts": hit.ts,
            "fact": hit.fact,
            "shape": hit.shape,
            "score": hit.score,
        })).collect::<Vec<_>>(),
        "refused": snapshot.refused,
    })
}

/// The pane's two reads, and the boundary they hold.
///
/// The root is the process's own working directory, resolved the way the
/// memory corpus resolves — nearest ancestor carrying `memories/`, else
/// the cwd — so the browser sees the tree this face was launched from
/// and nothing above it. Every path is canonicalized and checked against
/// that root, so a `..` that escapes is refused by name rather than
/// served. A credential file is refused by name too: the own-home ruling
/// is that no credential ever reaches the browser, and a file browser is
/// precisely where one would leak.
///
/// How many entries one tree read returns. A directory with more is
/// named as truncated rather than walked to exhaustion — the pane is for
/// exploring, and `target/` has a hundred thousand entries.
const TREE_ENTRY_CAP: usize = 400;

/// How many bytes of one file the pane carries. The corpus measured in
/// the thousands; a source file in the tens of thousands; a log or a
/// build artifact in the millions. The cap is the artifact case.
const FILE_BYTE_CAP: usize = 512 * 1024;

/// Directories the tree skips by name: build output and version control
/// internals, which are noise to browse and enormous to walk.
const TREE_SKIP: &[&str] = &[".git", "target", "node_modules", ".DS_Store"];

/// Resolve one pane path against the root, or name why it cannot be
/// read. The check is the canonicalized path's prefix, not the string's,
/// so a symlink out of the tree is refused too.
fn pane_path(root: &std::path::Path, asked: &str) -> Result<std::path::PathBuf, String> {
    let asked = asked.trim().trim_start_matches('/');
    if asked.is_empty() {
        return Ok(root.to_path_buf());
    }
    // The lexical check runs before anything touches the disk, so a path
    // that walks out of the root is refused for that reason whether or
    // not it exists — and a path that does not exist is not mistaken for
    // an escape. The canonical check below then catches a symlink that
    // points out, which the lexical walk cannot see.
    let mut depth: i64 = 0;
    for part in asked.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                depth -= 1;
                if depth < 0 {
                    return Err("outside the working directory".to_string());
                }
            }
            _ => depth += 1,
        }
    }
    let candidate = root.join(asked);
    let canonical = candidate
        .canonicalize()
        .map_err(|error| format!("{}: {error}", candidate.display()))?;
    let root = root
        .canonicalize()
        .map_err(|error| format!("{}: {error}", root.display()))?;
    if !canonical.starts_with(&root) {
        return Err("outside the working directory".to_string());
    }
    Ok(canonical)
}

/// The viewer's language for one file, from its extension — the handful
/// this repository actually carries, and a plain default for the rest.
fn viewer_language(path: &std::path::Path) -> &'static str {
    match path.extension().and_then(|ext| ext.to_str()) {
        Some("rs") => "rust",
        Some("js") => "javascript",
        Some("css") => "css",
        Some("html") => "html",
        Some("json") => "json",
        Some("md") => "markdown",
        Some("toml") => "toml",
        Some("yml" | "yaml") => "yaml",
        Some("sh") => "shell",
        _ => "text",
    }
}

/// One directory's entries, directories first then files, each with its
/// size in bytes.
fn tree_read(root: &std::path::Path, asked: &str) -> Value {
    let directory = match pane_path(root, asked) {
        Ok(path) => path,
        Err(reason) => return json!({"type": "tree", "refused": [reason]}),
    };
    if !directory.is_dir() {
        return json!({"type": "tree", "refused": [format!("{} is not a directory", directory.display())]});
    }
    let mut entries: Vec<(bool, String, u64)> = Vec::new();
    let Ok(read) = std::fs::read_dir(&directory) else {
        return json!({"type": "tree", "refused": [format!("{} could not be read", directory.display())]});
    };
    for entry in read.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if TREE_SKIP.contains(&name.as_str()) {
            continue;
        }
        let is_dir = entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false);
        let size = entry.metadata().map(|meta| meta.len()).unwrap_or(0);
        entries.push((is_dir, name, size));
    }
    let total = entries.len();
    entries.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(&right.1)));
    entries.truncate(TREE_ENTRY_CAP);
    json!({
        "type": "tree",
        "root": root.display().to_string(),
        "path": asked,
        "truncated": total > TREE_ENTRY_CAP,
        "entries": entries.iter().map(|(is_dir, name, size)| json!({
            "name": name,
            "kind": if *is_dir { "dir" } else { "file" },
            "size": size,
        })).collect::<Vec<_>>(),
    })
}

/// One file's text, its language, and whether the read was cut. A file
/// that is not UTF-8 is refused rather than mangled — the viewer shows
/// text, and a binary shown as text is a lie about what is there.
fn file_read(root: &std::path::Path, asked: &str) -> Value {
    // The credential check is lexical and first, so a dotenv is refused
    // by name whether or not it exists — a browser that can ask for a
    // file is exactly where a credential would leak, and the refusal must
    // not depend on the file being there to refuse.
    let name = asked.rsplit('/').next().unwrap_or(asked);
    if name == ".env" || name.starts_with(".env.") {
        return json!({"type": "file", "refused": ["a credential file is never served"]});
    }
    let path = match pane_path(root, asked) {
        Ok(path) => path,
        Err(reason) => return json!({"type": "file", "refused": [reason]}),
    };
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) => return json!({"type": "file", "refused": [format!("{error}")]}),
    };
    let truncated = bytes.len() > FILE_BYTE_CAP;
    let cut = if truncated {
        &bytes[..FILE_BYTE_CAP]
    } else {
        &bytes[..]
    };
    let text = match String::from_utf8(cut.to_vec()) {
        Ok(text) => text,
        Err(_) => {
            return json!({"type": "file", "refused": [format!("{} is not text", path.display())]})
        }
    };
    json!({
        "type": "file",
        "path": asked,
        "name": name,
        "language": viewer_language(&path),
        "bytes": bytes.len(),
        "truncated": truncated,
        "text": text,
    })
}

/// A session id this face will open: the shape `Session::create` mints —
/// a UTC stamp, a dash, a short suffix — and nothing else. The check is
/// lexical and first, because the id names a file under the home: a
/// traversal-shaped id is refused for its shape, whether or not a file
/// with that name could exist.
fn session_id_is_shape(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// How many cards a replay carries. A long session's log is the model's
/// whole history with it, and a document that shipped all of it would
/// make the panel pay for a conversation the owner has not opened yet;
/// the newest cards win, and the count left out is named beside the
/// cut — a bounded read says what it left unread.
const REPLAY_CARD_CAP: usize = 400;

/// The tool-result preview cap, the live path's own (`agent::preview`
/// at 80): a replayed card shows what a live card showed, so a
/// conversation cannot grow more detailed by being replayed.
const REPLAY_PREVIEW: usize = 80;

/// The cards a log's records shape into, oldest first, with the count
/// the cap left out. This is the one translator from records to the
/// card vocabulary the live fan-out emits, and both readers share it —
/// the replay panel previews exactly what a resume reseeds, so the two
/// cannot disagree about what a session says.
///
/// The route card is deliberately absent: the route decision rides the
/// calibration stream, not the session log, and a replay that invented
/// one would be a second opinion about a fact it does not hold. The
/// bookkeeping records (`start`, `end`) shape the replay's header
/// instead of cards, which is where a conversation's facts belong.
fn cards_from_records(records: &[session::Record]) -> (Vec<Value>, usize) {
    let mut cards: Vec<Value> = Vec::new();
    for record in records {
        match record {
            session::Record::Start { .. } | session::Record::End { .. } => {}
            session::Record::Rewind {
                turn,
                restored,
                removed,
            } => {
                cards.push(json!({
                    "type": "rewound",
                    "turn": turn,
                    "restored": restored,
                    "removed": removed,
                }));
            }
            session::Record::Plan { text } => {
                cards.push(json!({"type": "plan", "text": text}));
            }
            session::Record::User { content } => {
                cards.push(json!({"type": "message", "role": "owner", "text": content}));
            }
            session::Record::Assistant { content, .. } => {
                if let Some(text) = content {
                    cards.push(json!({"type": "message", "role": "assistant", "text": text}));
                }
            }
            session::Record::ToolResult {
                name,
                output,
                approved,
                ..
            } => {
                cards.push(json!({
                    "type": "tool_result",
                    "tool": name,
                    "preview": crate::agent::preview(output, REPLAY_PREVIEW),
                    "approved": approved,
                }));
            }
            session::Record::LaneTools { lane, tools } => {
                cards.push(json!({"type": "lane", "lane": lane, "tools": tools.len()}));
            }
            session::Record::Skills { skills } => {
                // The log pins what the run declared, not what the loader
                // skipped — a skipped name lives in the loader's own
                // report — so a replayed card says what the record says
                // and is silent about what it does not hold.
                cards.push(json!({"type": "skills", "loaded": skills.len()}));
            }
            session::Record::Memory { shape, facts } => {
                cards.push(json!({"type": "memory", "shape": shape, "facts": facts.len()}));
            }
            session::Record::Compaction {
                first_turn,
                last_turn,
                tokens_before,
                tokens_after,
                ..
            } => {
                let turns = last_turn.saturating_sub(*first_turn) + 1;
                cards.push(json!({
                    "type": "compacted",
                    "turns": turns,
                    "before": tokens_before,
                    "after": tokens_after,
                }));
            }
        }
    }
    let omitted = cards.len().saturating_sub(REPLAY_CARD_CAP);
    if omitted > 0 {
        cards.drain(..omitted);
    }
    (cards, omitted)
}

/// The replay as a reseed: the cards alone, or the named reason it
/// could not happen. This is [`Bridge::resume`]'s read half.
fn replay_cards(home: &std::path::Path, id: &str) -> Result<Vec<Value>, String> {
    if !session_id_is_shape(id) {
        return Err(format!("`{id}` is not a session id"));
    }
    let path = session::session_path(home, id);
    let records = session::replay(&path)
        .map_err(|error| format!("could not open session `{id}`: {error}"))?;
    Ok(cards_from_records(&records).0)
}

/// One archived session's replay document: the head the `start` record
/// pins, the summary the `end` record and the compaction marks carry,
/// and the cards.
fn replay_read(home: &std::path::Path, asked: &str) -> Value {
    let id = asked.trim();
    if !session_id_is_shape(id) {
        return json!({
            "type": "replay",
            "refused": [format!("`{id}` is not a session id")],
        });
    }
    let path = session::session_path(home, id);
    let records = match session::replay(&path) {
        Ok(records) => records,
        Err(error) => {
            return json!({
                "type": "replay",
                "refused": [format!("could not open session `{id}`: {error}")],
            })
        }
    };
    let (cards, omitted) = cards_from_records(&records);
    let (mut model, mut created, mut kind, mut parent) = (String::new(), String::new(), None, None);
    let mut ended = false;
    let mut served_model: Option<String> = None;
    let mut stop_reason = String::new();
    let mut turns = 0u64;
    let mut compacted = false;
    for record in &records {
        match record {
            session::Record::Start {
                model: m,
                created: c,
                kind: k,
                parent: p,
                ..
            } => {
                model = m.clone();
                created = c.clone();
                kind = k.clone();
                parent = p.clone();
            }
            session::Record::End {
                stop_reason: s,
                turns: t,
                served_model: answered,
                ..
            } => {
                ended = true;
                stop_reason = s.clone();
                turns = *t;
                served_model = answered.clone();
            }
            session::Record::Compaction { .. } => compacted = true,
            _ => {}
        }
    }
    json!({
        "type": "replay",
        "id": id,
        // The model a surface may name: the served one (KI-073), the
        // nominal stamp only on an ended unrouted run, null in flight —
        // the same rule the archive row reads.
        "model": if ended {
            served_model.as_deref().map(str::to_string).unwrap_or(model)
        } else {
            String::new()
        },
        "created": created,
        "kind": kind,
        "parent": parent,
        "ended": ended,
        "stop_reason": stop_reason,
        "turns": turns,
        "compacted": compacted,
        "omitted": omitted,
        "cards": cards,
    })
}

/// The archive as the sessions panel reads it: newest first, because
/// the archive reads backward, with this face's own session named. A
/// log that fails to parse stays in the list as `unreadable` — the
/// picker's own rule, and hiding a log the owner knows exists would be
/// the archive pretending to be more complete than it is.
fn sessions_read(home: &std::path::Path, current: &str) -> Value {
    let entries = match session::list(home) {
        Ok(entries) => entries,
        Err(error) => {
            return json!({
                "type": "sessions",
                "refused": [format!("the archive could not be read: {error}")],
            })
        }
    };
    let sessions = entries
        .iter()
        .rev()
        .map(|entry| {
            json!({
                "id": entry.id,
                // The model that actually served (KI-073's end record),
                // or null while the run is in flight — never the start
                // record's pre-route stamp, which is a default, not a
                // decision. The face renders a dash for it.
                "model": entry.serving_model(),
                "created": entry.created,
                "ended": entry.ended,
                "kind": entry.kind,
                "workspace": entry.workspace,
                "compacted": entry.compacted,
                "current": entry.id == current,
            })
        })
        .collect::<Vec<_>>();
    json!({
        "type": "sessions",
        "read_at": crate::boards::now_stamp(),
        "current": current,
        "sessions": sessions,
    })
}

/// The calibration panel's document. The rows come through the reader's
/// own cap (the last [`crate::calib::READ_CAP`] of the day, named in
/// the document), grouped by op for the stream view; the fit cells are
/// the calibration surface's own computation, not a re-derivation, so
/// this panel and `fleet calib --calibration` cannot disagree about
/// whether a provider's sample is fit.
fn calibration_read(home: &std::path::Path) -> Value {
    let user_home = std::env::var("HOME").ok();
    let root = crate::calib::resolve_root(home, user_home.as_deref());
    let day = crate::calib::today();
    let rows = crate::calib::read_day(&root, &day, crate::calib::READ_CAP);
    let gate = crate::route::calibration_config(home).informs_failover;
    let cells = crate::calib::calibration(&rows);
    let mut stream: std::collections::BTreeMap<&str, Vec<Value>> = Default::default();
    for row in &rows {
        stream
            .entry(row.op.as_str())
            .or_default()
            .push(row_json(row));
    }
    json!({
        "type": "calibration",
        "read_at": crate::boards::now_stamp(),
        "day": day,
        "root": root.display().to_string(),
        "cap": crate::calib::READ_CAP,
        "rows_read": rows.len(),
        "gate": {
            "informs_failover": gate,
            "name": "calibration.informs_failover",
        },
        "fit_rule": {
            "rows": crate::calib::FIT_MIN_ROWS,
            "span": crate::calib::FIT_MIN_SPAN,
        },
        "cells": cells.iter().map(cell_json).collect::<Vec<_>>(),
        "stream": stream,
    })
}

/// One stream row, shaped for the browser: the fields its op carries,
/// and none of the fields it does not — a `turn` row has no confidence
/// and a `propose` row has no provider, and a document that shipped
/// zeros for both would be inventing structure the row never had.
fn row_json(row: &crate::calib::ReadRow) -> Value {
    let mut out = serde_json::Map::new();
    out.insert("op".into(), json!(row.op));
    out.insert("ts".into(), json!(row.ts));
    let mut put = |key: &str, value: Value| {
        out.insert(key.into(), value);
    };
    match row.op.as_str() {
        "route" => {
            put("provider", json!(row.provider));
            put("model", json!(row.model));
            put("workload", json!(row.workload));
            put("how", json!(row.how));
            put("judged_from", json!(row.judged_from));
            put("providers_used", json!(row.providers_used));
            put(
                "attempts",
                json!(row
                    .attempts
                    .iter()
                    .map(|attempt| json!({
                        "provider": attempt.provider,
                        "outcome": attempt.outcome,
                        "class": attempt.class,
                    }))
                    .collect::<Vec<_>>()),
            );
            put("input_chars", json!(row.input_chars));
        }
        "route_judged" => {
            put("route_key", json!(row.route_key));
            put("workload", json!(row.workload));
            put("verdict", json!(row.verdict));
            put("confidence", json!(row.confidence));
            put("disposition", json!(row.disposition));
            put("lane", json!(row.lane));
        }
        "turn" => {
            put("outcome", json!(row.outcome));
            put("stop_reason", json!(row.stop_reason));
            put("turns", json!(row.turns));
            put("reason", json!(row.reason));
            put("input_chars", json!(row.input_chars));
            put("input_hash", json!(row.input_hash));
        }
        "tool_screen" => {
            put("tool", json!(row.tool));
            put("outcome", json!(row.outcome));
        }
        "recall_hit" => {
            put("fact", json!(row.fact));
            put("shape", json!(row.shape));
            put("score", json!(row.score));
        }
        "consolidate" => {
            put("action", json!(row.action));
            put("reason", json!(row.reason));
        }
        "propose" => {
            put("proposal", json!(row.proposal));
            put("target", json!(row.target));
            put("target_name", json!(row.target_name));
            put("basis", json!(row.basis));
            put("verdict", json!(row.verdict));
            put("input_sha256", json!(row.input_sha256));
            put(
                "gate",
                json!(row
                    .gate
                    .iter()
                    .map(|step| json!({"step": step.step, "ok": step.ok, "detail": step.detail}))
                    .collect::<Vec<_>>()),
            );
        }
        "eval_corpus" => {
            put("passed", json!(row.passed));
            put("failed", json!(row.failed));
            put("score", json!(row.score));
            put("input_sha256", json!(row.input_sha256));
            put(
                "scenarios",
                json!(row
                    .scenarios
                    .iter()
                    .map(|scenario| json!({
                        "scenario": scenario.scenario,
                        "family": scenario.family,
                        "passed": scenario.passed,
                        "failure": scenario.failure,
                    }))
                    .collect::<Vec<_>>()),
            );
        }
        "explain" => {
            put("explained_op", json!(row.explained_op));
            put("selector", json!(row.selector));
            put("cost_usd", json!(row.cost_usd));
            put("latency_ms", json!(row.latency_ms));
            put("eval_only", json!(row.eval_only));
        }
        // A future slice's op crosses whole, its fields the reader
        // already parsed: an op this match does not name still rides,
        // because dropping a row the stream holds would make the panel
        // quieter than the day was.
        _ => {
            put("verdict", json!(row.verdict));
            put("outcome", json!(row.outcome));
        }
    }
    Value::Object(out)
}

/// One fit cell, shaped for the browser: the counts, the weighted
/// rates, the fit state with what it still needs, and the spend.
fn cell_json(cell: &crate::calib::CalibrationCell) -> Value {
    json!({
        "provider": cell.provider,
        "workload": cell.workload,
        "judged": cell.judged,
        "right": cell.right,
        "wrong": cell.wrong,
        "insufficient": cell.insufficient,
        "waived": cell.waived,
        "overridden": cell.overridden,
        "weighted_wrong_rate": cell.weighted_wrong_rate,
        "weighted_insufficient_rate": cell.weighted_insufficient_rate,
        "confidence_span": cell.confidence_span,
        "zero_confidence": cell.zero_confidence,
        "fit": cell.fit.label(),
        "needs_rows": cell.needs_rows(),
        "needs_span": cell.needs_span(),
        "informs_failover": cell.informs_failover(),
        "spend_usd": cell.spend_usd,
        "cost_per_right": cell.cost_per_right,
    })
}

/// The settings panel's document. The knobs, the lists, the rules, the
/// env names, the jobs, and the SOUL slot — each read through the
/// surface's own reader, so this document is the CLI's answer in JSON
/// rather than a parallel truth.
fn settings_read(home: &std::path::Path) -> Value {
    let scalars = crate::settings::states(home)
        .into_iter()
        .map(|state| {
            json!({
                "key": state.key,
                "kind": match state.kind {
                    crate::settings::Kind::Text => "text",
                    crate::settings::Kind::Toggle => "toggle",
                    crate::settings::Kind::Choice(_) => "choice",
                    crate::settings::Kind::Ratio => "ratio",
                    crate::settings::Kind::Count => "count",
                },
                "file": match state.file {
                    crate::settings::File::Config => "config",
                    crate::settings::File::Routing => "routing",
                    crate::settings::File::Env => "fleet.env",
                },
                "summary": state.summary,
                "value": state.rendered(),
                "source": state.source(),
            })
        })
        .collect::<Vec<_>>();
    let lists = crate::settings::list_states(home)
        .into_iter()
        .map(|entry| {
            json!({
                "kind": match entry.kind {
                    crate::settings::ListKind::Provider => "provider",
                    crate::settings::ListKind::Workload => "workload",
                },
                "key": entry.key,
                "detail": entry.detail,
                "reverts": entry.reverts,
                "seed": entry.seed,
            })
        })
        .collect::<Vec<_>>();
    let rules = crate::settings::approval_rules(home)
        .list()
        .iter()
        .map(|rule| json!({"label": rule.label(), "tool": rule.tool, "program": rule.program}))
        .collect::<Vec<_>>();
    // The env status, by name only: whether each env-var counterpart
    // the settings writer owns resolves right now, and from where. A
    // VALUE never rides the document — the doctor's masking rule.
    // The vault laid over the dotenv: a sealed key answers here exactly
    // where a plaintext key would have, so the router and the agent see
    // the same credentials either way.
    let passphrase = std::env::var("FLEET_VAULT_PASSPHRASE").ok();
    let dotenv = crate::home::read_dotenv_with_vault(home, passphrase.as_deref());
    let fleet_env = crate::home::read_fleet_env(home);
    let env = crate::settings::env_keys()
        .into_iter()
        .map(|name| {
            let present = std::env::var(name).is_ok_and(|value| !value.is_empty())
                || dotenv.contains_key(name)
                || fleet_env.contains_key(name);
            json!({"name": name, "present": present})
        })
        .collect::<Vec<_>>();
    let cron = match crate::cron::Store::open(home) {
        Ok(store) => store
            .jobs()
            .iter()
            .map(|job| {
                json!({
                    "name": job.name,
                    "schedule": job.schedule,
                    "task": job.task,
                    "enabled": job.enabled,
                    "auto_approve": job.auto_approve,
                    "last_run": job.last_run,
                    "last_outcome": job.last_outcome,
                })
            })
            .collect::<Vec<_>>(),
        Err(_) => Vec::new(),
    };
    let soul = crate::home::read_soul(home);
    json!({
        "type": "settings",
        "read_at": crate::boards::now_stamp(),
        "scalars": scalars,
        "lists": lists,
        "rules": rules,
        "env": env,
        "cron": cron,
        "soul": {
            "present": soul.is_some(),
            "text": soul.unwrap_or_default(),
        },
    })
}

/// One settings write, dispatched to the writer the surface owns. The
/// browser's rule form is the file's own shape — `{tool, program?}` —
/// and it rides `allow_tool` through a synthesized call, so every class
/// check (the always-ask and destructive classes, the empty-rule
/// refusal) is the seat's own and not a second implementation of it.
fn settings_apply(
    home: &std::path::Path,
    op: &str,
    arguments: &serde_json::Value,
) -> Result<String, String> {
    let text = |key: &str| {
        arguments
            .get(key)
            .and_then(serde_json::Value::as_str)
            .map(str::to_string)
            .ok_or_else(|| format!("the write names its {key}"))
    };
    let flag = |key: &str| arguments.get(key).and_then(serde_json::Value::as_bool);
    match op {
        "set" => {
            let key = text("key")?;
            let value = text("value")?;
            crate::settings::set(home, &key, &value)?;
            Ok(key)
        }
        "provider" => {
            let entry = text("entry")?;
            crate::settings::set_provider(home, &entry)
        }
        "provider-remove" => {
            let id = text("id")?;
            crate::settings::remove_provider(home, &id, flag("force").unwrap_or(false))?;
            Ok(id)
        }
        "workload" => {
            let workload = text("workload")?;
            let candidates = arguments
                .get("candidates")
                .and_then(serde_json::Value::as_array)
                .map(|list| {
                    list.iter()
                        .filter_map(serde_json::Value::as_str)
                        .map(str::to_string)
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            crate::settings::set_workload(home, &workload, &candidates)?;
            Ok(workload)
        }
        "workload-remove" => {
            let workload = text("workload")?;
            crate::settings::remove_workload(home, &workload, flag("force").unwrap_or(false))?;
            Ok(workload)
        }
        "rule" => {
            // The browser sends the file's own shape; the call is
            // synthesized so `allow_tool` derives the rule the way the
            // seat's remember does — one place a rule is built.
            let tool = text("tool")?;
            let program = arguments
                .get("program")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string);
            let arguments_json = match &program {
                Some(program) => serde_json::json!({ "command": program }).to_string(),
                None => "{}".to_string(),
            };
            let call = crate::api::ToolCall {
                id: "settings-rule".to_string(),
                kind: "function".to_string(),
                function: crate::api::FunctionCall {
                    name: tool,
                    arguments: arguments_json,
                },
            };
            let rule = crate::settings::allow_tool(home, &call)?;
            Ok(rule.label())
        }
        "rule-remove" => {
            let tool = text("tool")?;
            let program = arguments
                .get("program")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string);
            let value = serde_json::json!({"tool": tool, "program": program});
            let rule = crate::approvals::Rule::from_value(&value)
                .ok_or_else(|| "that is not a rule the file can carry".to_string())?;
            crate::settings::forget_tool(home, &rule)?;
            Ok(rule.label())
        }
        "cron" => {
            let mut store = crate::cron::Store::open(home).map_err(|error| error.to_string())?;
            let name = text("name")?;
            let schedule = text("schedule")?;
            let task = text("task")?;
            let job = store
                .add(
                    &name,
                    &schedule,
                    &task,
                    flag("auto_approve").unwrap_or(false),
                )
                .map_err(|error| error.to_string())?;
            Ok(job.name)
        }
        "cron-enabled" => {
            let mut store = crate::cron::Store::open(home).map_err(|error| error.to_string())?;
            let name = text("name")?;
            let job = store
                .set_enabled(&name, flag("enabled").unwrap_or(true))
                .map_err(|error| error.to_string())?;
            Ok(job.name)
        }
        "cron-remove" => {
            let mut store = crate::cron::Store::open(home).map_err(|error| error.to_string())?;
            let name = text("name")?;
            let job = store.remove(&name).map_err(|error| error.to_string())?;
            Ok(job.name)
        }
        "soul" => {
            crate::home::write_soul(home, &text("text")?)
                .map_err(|error| format!("the SOUL slot could not be written: {error}"))?;
            Ok("SOUL.md".to_string())
        }
        other => Err(format!("`{other}` is not a write this surface performs")),
    }
}

/// One file's change, shaped for the browser: the kind, the hunks with
/// their coordinates, and the too-large flag so the panel can offer the
/// whole-file reject honestly instead of a partial alignment.
fn diff_file_json(file: &crate::checkpoints::FileDiff) -> Value {
    json!({
        "path": file.path,
        "kind": match file.kind {
            crate::checkpoints::DiffKind::Added => "added",
            crate::checkpoints::DiffKind::Removed => "removed",
            crate::checkpoints::DiffKind::Modified => "modified",
        },
        "too_large": file.too_large,
        "hunks": file.hunks.iter().map(|hunk| json!({
            "old": hunk.old,
            "new": hunk.new,
            "new_at": hunk.new_at,
        })).collect::<Vec<_>>(),
    })
}

/// The tower console's document. The git facts and the brain's
/// aggregates are two reads that fail for different reasons — the
/// projects root may not exist, the second-brain may not be installed —
/// and each names itself without silencing the other. The ranked list
/// rides in the board's own dirtiest-first order; the console's bars
/// are a rendering of it, not a second sort.
fn tower_read(home: &std::path::Path) -> Value {
    // The vault laid over the dotenv: a sealed key answers here exactly
    // where a plaintext key would have, so the router and the agent see
    // the same credentials either way.
    let passphrase = std::env::var("FLEET_VAULT_PASSPHRASE").ok();
    let dotenv = crate::home::read_dotenv_with_vault(home, passphrase.as_deref());
    let sources = crate::boards::resolve_sources(home, &dotenv);
    let mut refused: Vec<String> = Vec::new();
    let mut document = serde_json::Map::new();
    document.insert("type".into(), json!("tower"));
    document.insert("read_at".into(), json!(crate::boards::now_stamp()));
    document.insert(
        "root".into(),
        json!(sources.tower_root.display().to_string()),
    );
    match crate::boards::read_tower(&sources.tower_root, &sources.bounds) {
        Ok(snapshot) => {
            document.insert(
                "projects".into(),
                json!(snapshot
                    .projects
                    .iter()
                    .map(project_facts_json)
                    .collect::<Vec<_>>()),
            );
            document.insert(
                "skipped".into(),
                json!(snapshot
                    .skipped
                    .iter()
                    .map(|(name, reason)| json!({"name": name, "reason": reason}))
                    .collect::<Vec<_>>()),
            );
            document.insert("beyond_bound".into(), json!(snapshot.beyond_bound));
            document.insert("unread".into(), json!(snapshot.unread));
        }
        Err(reason) => {
            document.insert("projects".into(), json!([]));
            document.insert("skipped".into(), json!([]));
            document.insert("beyond_bound".into(), json!(0));
            document.insert("unread".into(), json!(0));
            refused.push(reason);
        }
    }
    let Some(brain) = sources.brain_home.clone() else {
        refused.push("the second-brain board has no home on this machine".to_string());
        document.insert("refused".into(), json!(refused));
        return Value::Object(document);
    };
    let snapshot = match crate::boards::read_brain(&brain, &sources.bounds) {
        Ok(snapshot) => snapshot,
        Err(reason) => {
            refused.push(reason);
            document.insert("refused".into(), json!(refused));
            return Value::Object(document);
        }
    };
    if let Some(reports) = snapshot.reports.landed() {
        document.insert(
            "kpis".into(),
            json!({
                "doing": reports.doing_total,
                "pending_verification": reports.pending_verification,
                "open_conflicts": reports.open_conflicts,
                "suppressed": reports.suppressed,
                "malformed": reports.malformed,
                "events_total": reports.events_total,
            }),
        );
        document.insert(
            "coverage".into(),
            json!(reports.coverage.as_ref().map(|coverage| json!({
                "window_days": coverage.window_days,
                "with_reports": coverage.with_reports,
                "commit_only": coverage.commit_only,
                "ratio": coverage.ratio,
            }))),
        );
        document.insert(
            "report_daily".into(),
            json!(reports
                .report_daily
                .iter()
                .map(|(day, count)| json!({"day": day, "count": count}))
                .collect::<Vec<_>>()),
        );
        document.insert(
            "feed".into(),
            json!(reports
                .feed
                .iter()
                .map(|row| json!({
                    "ts": row.ts,
                    "kind": row.kind,
                    "project": row.project,
                    "harness": row.harness,
                    "summary": row.summary,
                    "task": row.task,
                    "commit": row.commit,
                }))
                .collect::<Vec<_>>()),
        );
        document.insert(
            "conflicts".into(),
            json!(reports
                .conflicts
                .iter()
                .map(|row| json!({
                    "task": row.task,
                    "class": row.class,
                    "severity": row.severity,
                    "claim": row.claim,
                    "reality": row.reality,
                }))
                .collect::<Vec<_>>()),
        );
        document.insert(
            "trust".into(),
            json!(reports
                .trust
                .iter()
                .map(|row| json!({
                    "harness": row.harness,
                    "claimed": row.claimed,
                    "evidenced": row.evidenced,
                    "precision": row.precision,
                }))
                .collect::<Vec<_>>()),
        );
        document.insert("reports_at".into(), json!(reports.generated_at));
    } else if let Some(reason) = snapshot.reports.reason() {
        refused.push(reason.to_string());
    }
    if let Some(tower) = snapshot.tower.landed() {
        document.insert("brain_at".into(), json!(tower.generated_at));
        document.insert(
            "timeline".into(),
            json!(tower
                .timeline
                .iter()
                .map(|row| json!({
                    "date": row.date,
                    "subject": row.subject,
                    "project": row.project,
                }))
                .collect::<Vec<_>>()),
        );
        document.insert(
            "harness".into(),
            json!(tower
                .harness
                .iter()
                .map(|row| json!({
                    "harness": row.harness,
                    "total": row.total,
                    "user": row.user,
                }))
                .collect::<Vec<_>>()),
        );
        document.insert(
            "brain_projects".into(),
            json!(tower
                .projects
                .iter()
                .map(|project| json!({
                    "name": project.name,
                    "lifecycle": project.lifecycle,
                    "branch": project.branch,
                    "last_commit": project.last_commit,
                    "commits_30d": project.commits_30d,
                    "dirty": project.dirty,
                    "plans": project.plans.iter().map(|(status, count)| json!({
                        "status": status, "count": count,
                    })).collect::<Vec<_>>(),
                }))
                .collect::<Vec<_>>()),
        );
    } else if let Some(reason) = snapshot.tower.reason() {
        refused.push(reason.to_string());
    }
    document.insert("refused".into(), json!(refused));
    Value::Object(document)
}

/// One project's git facts, shaped for the console's ranked list. A
/// `None` stays `null`: git could not answer, and the console says so
/// rather than rendering a zero nobody measured.
fn project_facts_json(project: &crate::boards::ProjectFacts) -> Value {
    json!({
        "name": project.name,
        "branch": project.branch,
        "head": project.head,
        "subject": project.subject,
        "when": project.when,
        "commits_30d": project.commits_30d,
        "dirty": project.dirty,
        "ahead": project.ahead,
        "behind": project.behind,
        "notes": project.notes,
    })
}

/// The second-brain panel's document: the six sections, each landed or
/// named, shaped for the browser.
fn brain_read(home: &std::path::Path) -> Value {
    // The vault laid over the dotenv: a sealed key answers here exactly
    // where a plaintext key would have, so the router and the agent see
    // the same credentials either way.
    let passphrase = std::env::var("FLEET_VAULT_PASSPHRASE").ok();
    let dotenv = crate::home::read_dotenv_with_vault(home, passphrase.as_deref());
    let sources = crate::boards::resolve_sources(home, &dotenv);
    let mut refused: Vec<String> = Vec::new();
    let mut document = serde_json::Map::new();
    document.insert("type".into(), json!("brain"));
    document.insert("read_at".into(), json!(crate::boards::now_stamp()));
    let Some(brain) = sources.brain_home.clone() else {
        refused.push("the second-brain board has no home on this machine".to_string());
        document.insert("refused".into(), json!(refused));
        return Value::Object(document);
    };
    let snapshot = match crate::boards::read_brain(&brain, &sources.bounds) {
        Ok(snapshot) => snapshot,
        Err(reason) => {
            refused.push(reason);
            document.insert("refused".into(), json!(refused));
            return Value::Object(document);
        }
    };
    document.insert("home".into(), json!(snapshot.home.display().to_string()));
    if let Some(gates) = snapshot.gates.landed() {
        document.insert(
            "gates".into(),
            json!({
                "generated": gates.generated,
                "board": gates.board.iter().map(|row| json!({
                    "gate": row.gate,
                    "verdicts": row.verdicts,
                    "dispositions": row.dispositions,
                    "graded_progress": row.graded_progress,
                    "graded_rows": row.graded_rows,
                    "fit_ready": row.fit_ready,
                    "override_rate": row.override_rate,
                })).collect::<Vec<_>>(),
                "inbox": gates.inbox.iter().map(|row| json!({
                    "op": row.op,
                    "verdict": row.verdict,
                    "ts": row.ts,
                    "command": row.command,
                })).collect::<Vec<_>>(),
                "totals": {
                    "events": gates.totals.events,
                    "verdicts": gates.totals.verdicts,
                    "dispositions": gates.totals.dispositions,
                },
                "daily": gates.daily.iter().map(|(day, count)| json!({
                    "day": day, "count": count,
                })).collect::<Vec<_>>(),
            }),
        );
    } else if let Some(reason) = snapshot.gates.reason() {
        refused.push(reason.to_string());
    }
    if let Some(harness) = snapshot.harness.landed() {
        document.insert(
            "harness".into(),
            json!({
                "generated": harness.generated,
                "loops": harness.loops.iter().map(|row| json!({
                    "name": row.name,
                    "runs": row.runs,
                    "converged": row.converged,
                    "last_status": row.last_status,
                })).collect::<Vec<_>>(),
                "recent": harness.recent.iter().map(|row| json!({
                    "ts": row.ts,
                    "kind": row.kind,
                    "label": row.label,
                    "outcome": row.outcome,
                })).collect::<Vec<_>>(),
            }),
        );
    } else if let Some(reason) = snapshot.harness.reason() {
        refused.push(reason.to_string());
    }
    if let Some(loops) = snapshot.loops.landed() {
        document.insert(
            "loops".into(),
            json!({
                "derived_at": loops.derived_at,
                "loops": loops.loops.iter().map(|row| json!({
                    "name": row.name,
                    "family": row.family,
                    "verdict": row.verdict,
                    "iterations": row.iterations,
                    "pending_review": row.pending_review,
                })).collect::<Vec<_>>(),
                "graded": loops.graded.iter().map(|(name, row)| json!({
                    "name": name, "n": row.n, "confirmed": row.confirmed,
                })).collect::<Vec<_>>(),
                "findings": loops.findings,
            }),
        );
    } else if let Some(reason) = snapshot.loops.reason() {
        refused.push(reason.to_string());
    }
    if let Some(reports) = snapshot.reports.landed() {
        document.insert(
            "reports".into(),
            json!({
                "generated_at": reports.generated_at,
                "doing_total": reports.doing_total,
                "pending_verification": reports.pending_verification,
                "open_conflicts": reports.open_conflicts,
                "suppressed": reports.suppressed,
                "malformed": reports.malformed,
                "events_total": reports.events_total,
            }),
        );
    } else if let Some(reason) = snapshot.reports.reason() {
        refused.push(reason.to_string());
    }
    if let Some(maturity) = snapshot.maturity.landed() {
        document.insert(
            "maturity".into(),
            json!({
                "generated": maturity.generated,
                "weeks": maturity.weeks,
                "series": maturity.series.iter().map(|(name, points)| json!({
                    "name": name, "points": points,
                })).collect::<Vec<_>>(),
                "milestones": maturity.milestones.iter().map(|(when, what)| json!({
                    "when": when, "what": what,
                })).collect::<Vec<_>>(),
            }),
        );
    } else if let Some(reason) = snapshot.maturity.reason() {
        refused.push(reason.to_string());
    }
    document.insert("refused".into(), json!(refused));
    Value::Object(document)
}

/// The skills panel's document, hand-built from the read's snapshot for
/// the same reason: skills.rs derives nothing.
fn skills_read(home: &std::path::Path) -> Value {
    let override_dir = std::env::var("FLEET_SKILLS_DIR").ok();
    let user_home = std::env::var("HOME").ok();
    let canonical =
        crate::skills::resolve_skills_dir(override_dir.as_deref(), user_home.as_deref());
    let snapshot = crate::boards::read_skills(canonical.as_deref(), home);
    json!({
        "type": "skills",
        "read_at": snapshot.read_at,
        "canonical": snapshot.canonical,
        "skills": snapshot.skills.iter().map(|skill| json!({
            "name": skill.name,
            "description": skill.description,
            "uses": skill.uses,
            "modified": skill.modified,
            "path": skill.path,
        })).collect::<Vec<_>>(),
        "skips": snapshot.skips.iter().map(|skip| json!({
            "name": skip.name,
            "reason": skip.reason,
        })).collect::<Vec<_>>(),
        "proposals": snapshot.proposals.iter().map(|proposal| json!({
            "name": proposal.name,
            "description": proposal.description,
            "kind": proposal.kind,
            "evidence": proposal.evidence,
            "body": proposal.body,
            "path": proposal.path,
        })).collect::<Vec<_>>(),
        "merges": snapshot.merges.iter().map(|merge| json!({
            "survivor": merge.survivor,
            "absorbed": merge.absorbed,
            "similarity": merge.similarity,
            "threshold": merge.threshold,
        })).collect::<Vec<_>>(),
        "threshold": snapshot.threshold,
        "sessions": snapshot.sessions,
        "unused": snapshot.unused,
        "curation_note": snapshot.curation_note,
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
fn record(
    state: &Mutex<GuiState>,
    events: &broadcast::Sender<Value>,
    session: &str,
    value: Value,
) {
    let mut notice = None;
    {
        let mut state = lock(state);
        let attached = state.attached(session);
        // The run is over the moment the core says so, whichever word it
        // used: a header that kept counting after the loop stopped would
        // be lying about where the owner is.
        if matches!(
            value.get("type").and_then(Value::as_str),
            Some("finished") | Some("failed")
        ) {
            attached.running = false;
        }
        if attached.events.len() >= MAX_EVENTS {
            attached.events.pop_front();
            attached.dropped += 1;
            if attached.dropped == 1 {
                notice = Some(json!({"type": "dropped", "dropped": attached.dropped}));
            }
        }
        attached.events.push_back(value.clone());
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
        // The browser face drives its own agent thread, which creates
        // the log on the first run exactly as the TUI's does (KI-022):
        // the page's session line reads the id it is given, and nothing
        // before that.
        UiEvent::Session { id } => json!({"type": "session", "id": id}),
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
        UiEvent::Checkpoint { turn, files, bytes } => {
            json!({"type": "checkpoint", "turn": turn, "files": files, "bytes": bytes})
        }
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
        UiEvent::Deck {
            project,
            nodes,
            html,
            faults,
            original_deck,
            fix_deck,
            fix_actions,
        } => json!({
            "type": "deck",
            "project": project,
            "nodes": nodes,
            "html": html,
            // The action card's measured half and its payloads: the
            // faults ride as a JSON array in a string, and the two decks
            // are the render endpoint's own inputs.
            "faults": faults,
            "original_deck": original_deck,
            "fix_deck": fix_deck,
            "fix_actions": fix_actions,
        }),
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
                Some(event) => {
                    // The log the agent thread created on its first task
                    // becomes this face's launch session: the pending log
                    // moves to the id, so a face that was reading the
                    // pending key reads the same events under the id it
                    // shows (KI-022).
                    if let UiEvent::Session { id } = &event {
                        lock(&state).name_launch(id);
                    }
                    // The event belongs to the session the thread is
                    // driving, which is exact because the thread's
                    // command loop is serial: the events for one task all
                    // arrive before the next command is read.
                    let session = lock(&state).target();
                    record(&state, &events, &session, describe(&event));
                }
                None => ui_done = true,
            },
            request = seat_rx.recv(), if !seat_done => match request {
                Some(request) => {
                    let session = lock(&state).target();
                    let pending = describe_seat(&request.call, &request.summary);
                    lock(&state).attached(&session).seat = Some(request);
                    let _ = events.send(pending);
                }
                None => seat_done = true,
            },
        }
    }
    // The thread is gone, so nothing is waiting on a responder any more:
    // a seat left standing would be a prompt the page can answer into a
    // channel nobody reads. Clearing it is the honest end of the run.
    let session = lock(&state).target();
    let standing = lock(&state).attached(&session).seat.take().is_some();
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

/// Remove the control beside a session's log when a run ends. The agent
/// consumes one on read; a stop that lands after the last check is never
/// read, and a file left behind would end the next run in this session
/// before its first turn. Best-effort: an absent file is the ordinary
/// case, and a removal that fails leaves evidence rather than a panic.
fn clear_control(path: &std::path::Path) {
    let _ = std::fs::remove_file(path.with_extension("control"));
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
            memories_root,
            state,
        } = inputs;
        let rules = Arc::new(Mutex::new(crate::settings::approval_rules(&home)));
        // The scheduled consolidation cycle: one pass now, then one every
        // hour for the life of the process — the timer the live system
        // never had. Its own stream, because the cycle must run even when
        // routing never produced one.
        let user_home = std::env::var("HOME").ok();
        let calib_root = crate::calib::resolve_root(&home, user_home.as_deref());
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
            // The route is chosen once per session, on that session's
            // first task, and the committed router is reused for the rest
            // of it — one sys1 judgment, not one per task. A session this
            // thread has never driven has no entry and is judged when its
            // first task arrives.
            let mut routes: std::collections::HashMap<String, Option<crate::route::Prepared>> =
                std::collections::HashMap::new();
            let mut judge_tasks: std::collections::HashMap<String, String> =
                std::collections::HashMap::new();
            // The launch session does not exist yet: it is created on the
            // first task, so a browser face that is opened and closed
            // without a prompt leaves no empty log behind (KI-022),
            // exactly as the TUI's thread does.
            let mut current_session: Option<String> = session_id;
            let mut commands = command_rx;
            while let Some(command) = commands.recv().await {
                // Which arm this task arrived on, read before the match
                // consumes it: the two share one body, and the flag is
                // the only difference between a run and a plan.
                let plan_mode = matches!(command, Command::Plan(_, _));
                match command {
                    Command::Run(driving, task) | Command::Plan(driving, task) => {
                        // The session this task drives. An empty name is
                        // the face's own launch session, which is created
                        // on this first task: a face that is opened and
                        // closed without a prompt leaves no empty session
                        // behind (KI-022), the same rule the TUI's thread
                        // follows. The id rides back on the event stream,
                        // which shows the session the page is logging into
                        // and nothing before that.
                        let driving = if driving.is_empty() {
                            match &current_session {
                                Some(id) => id.clone(),
                                None => match Session::create(&home, &model) {
                                    Ok(fresh) => {
                                        let id = fresh.id().to_string();
                                        current_session = Some(id.clone());
                                        let _ = ui_tx.send(UiEvent::Session { id: id.clone() });
                                        id
                                    }
                                    Err(error) => {
                                        let _ = ui_tx.send(UiEvent::Failed(format!(
                                            "could not create the session: {error}"
                                        )));
                                        continue;
                                    }
                                },
                            }
                        } else {
                            current_session = Some(driving.clone());
                            driving
                        };
                        // The face's events belong to the session being
                        // driven, which is exact because the thread's
                        // command loop is serial.
                        lock(&state).driving = Some(driving.clone());
                        let slot = routes.entry(driving.clone()).or_insert(None);
                        if slot.is_none() {
                            match crate::route::prepare(
                                &home,
                                &dotenv,
                                &task,
                                &std::collections::HashMap::new(),
                            )
                            .await
                            {
                                Ok(fresh) => {
                                    judge_tasks.insert(driving.clone(), task.clone());
                                    *slot = Some(fresh);
                                }
                                Err(error) => {
                                    // A stop can be written while the
                                    // route is being judged. This run
                                    // will never read it, and a later
                                    // run in the same session must not.
                                    if let Some(id) = current_session.as_deref() {
                                        clear_control(&session::session_path(&home, id));
                                    }
                                    let _ = ui_tx
                                        .send(UiEvent::Failed(format!("routing config: {error}")));
                                    continue;
                                }
                            }
                        }
                        let prepared = slot.as_ref().expect("prepared on the first run");
                        let client = Client::routed(prepared.router.clone());
                        let path = session::session_path(
                            &home,
                            current_session.as_deref().unwrap_or_default(),
                        );
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
                                clear_control(&path);
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
                            // The trust trio's floor: this face's runs
                            // checkpoint per turn, under the home's own
                            // store, so a run can be rewound from the
                            // browser and diffed against a turn.
                            checkpoints: Some(home.join("checkpoints")),
                            plan_mode,
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
                                    clear_control(&path);
                                    let _ = ui_tx.send(UiEvent::Failed(error.to_string()));
                                    continue;
                                }
                            };
                        let _ = agent.run(&task).await;
                        // A control that landed on the run's last step is
                        // never read by the loop. Left in place it would
                        // stop the next run in this session before that
                        // run's first turn, which is not what the owner
                        // asked for.
                        clear_control(&path);
                    }
                    Command::Rewind(_turn) => {
                        // The face already restored the tree, recorded
                        // the rewind, and reseeded its own view. The
                        // thread's half is the prepared route: the
                        // judgment answered the conversation that is
                        // gone, and the next task is judged as its own.
                        routes.insert(current_session.clone(), None);
                        judge_tasks.remove(&current_session);
                    }
                    Command::Resume(id) => {
                        // The attach is the TUI's resume, one surface
                        // over: the session id becomes the one this loop
                        // drives next, and the prepared route decision
                        // goes with it — that judgment answered the
                        // conversation the face came in on, and the
                        // resumed one deserves its own. Other sessions'
                        // routes stand: a phone working in another
                        // conversation keeps its judgment.
                        current_session = Some(id.clone());
                        routes.insert(id.clone(), None);
                        judge_tasks.remove(&id);
                    }
                }
            }
            // Every session's routing decision is recorded once, when
            // the agent thread ends — best-effort, never fatal. A thread
            // that drove three conversations records three judgments.
            for (session, prepared) in &routes {
                if let Some(prepared) = prepared {
                    let judged = judge_tasks.get(session).cloned().unwrap_or_default();
                    prepared.record_route(&judged);
                }
            }
            // And the self-authoring half runs once, on the session this
            // face drove: its lessons are drafted into proposals awaiting
            // the slice-12 gate, best-effort, never applied here.
            let user_home = std::env::var("HOME").ok();
            let path = match current_session.as_deref() {
                Some(id) => crate::session::session_path(&home, id),
                // A face that never ran has no session to draft from;
                // the pass is skipped rather than reading a log that
                // does not exist (KI-022).
                None => {
                    return;
                }
            };
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
    /// A bridge over a temp home of its own. The home is unique per call
    /// rather than per process: tests in this binary run concurrently, and
    /// a shared home lets one test's `remove_dir_all` delete the directory
    /// out from under another's settings write — a flake the G4 gate hit
    /// and the plan named rather than fixed at the time.
    async fn bridge() -> (Bridge, Sources, UnboundedReceiver<Command>) {
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let serial = NEXT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let home =
            std::env::temp_dir().join(format!("fleet-gui-bridge-{}-{serial}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).expect("a temp home");
        let (mut bridge, sources) = Bridge::detached(
            home,
            "deepseek-chat".to_string(),
            Some("session-test".to_string()),
        );
        let door = bridge.connect();
        (bridge, sources, door)
    }

    /// The welcome's one read is assembled from the panel reads and
    /// carries the four answers the home renders: the workspace, the run
    /// archive, the corpus summary, the library count, and the graph's
    /// next node (null when no graph is declared). It is cached, so a
    /// second call inside the window is the same document.
    #[tokio::test(flavor = "multi_thread")]
    async fn home_summarises_the_reads_it_claims() {
        let (bridge, _sources, _door) = bridge().await;
        let first = bridge.home().await;
        assert_eq!(first["type"], "home");
        assert!(first["workspace"].is_string(), "{first:?}");
        assert_eq!(first["runs"]["count"], 0, "{first:?}");
        assert!(first["runs"]["recent"].is_array());
        assert!(first["memory"]["live"].is_number());
        assert!(first["memory"]["top"].is_array());
        assert!(first["skills"]["count"].is_number());
        assert!(first["next"].is_null() || first["next"]["id"].is_string());
        assert!(first["refused"].is_array());
        let second = bridge.home().await;
        assert_eq!(first, second, "the cached read is the same document");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn the_owner_message_lands_before_the_run_starts() {
        let (bridge, _sources, _door) = bridge().await;
        let attached = bridge.launch();
        bridge
            .run(&attached, "  look at the router  ")
            .expect("the run is accepted");
        let state = bridge.state(&attached);
        let events = state["events"].as_array().expect("an event log");
        assert_eq!(events.len(), 1, "{events:?}");
        assert_eq!(events[0]["type"], "message");
        assert_eq!(events[0]["role"], "owner");
        assert_eq!(events[0]["text"], "look at the router");
        assert_eq!(state["running"], true);
    }

    /// The owner's stop and steer write the file the turn loop reads,
    /// and both refusals are named: no run in flight, and a run whose
    /// log does not exist yet. The control rides the ordered log as its
    /// own card, so a refresh replays what the owner did.
    #[tokio::test(flavor = "multi_thread")]
    async fn a_control_rides_beside_the_log_and_is_refused_when_idle() {
        let (bridge, _sources, _door) = bridge().await;
        assert!(
            bridge
                .stop(None)
                .unwrap_err()
                .contains("no run is in flight"),
            "an idle face refuses a control"
        );
        let log = Session::create(&bridge.home, "deepseek-chat").expect("the log");
        let id = log.id().to_string();
        drop(log);
        {
            let mut state = lock(&bridge.state);
            state.session = id.clone();
            state.running = true;
        }
        let stop = bridge.stop(None).expect("the stop lands");
        assert_eq!(stop["kind"], "stop");
        assert_eq!(stop["session"], id.as_str());
        let control = crate::home::sessions_dir(&bridge.home).join(format!("{id}.control"));
        let body = std::fs::read_to_string(&control).expect("the control file");
        assert_eq!(body.trim(), r#"{"stop":true}"#, "{body}");
        let state = bridge.state();
        let events = state["events"].as_array().expect("an event log");
        assert_eq!(events.last().expect("the control")["type"], "control");
        assert_eq!(events.last().expect("the control")["kind"], "stop");

        // A steer replaces the control with its text, and an empty one
        // is refused before anything is written.
        bridge
            .stop(Some("read the gate first"))
            .expect("the steer lands");
        let body = std::fs::read_to_string(&control).expect("the steer file");
        assert!(body.contains("read the gate first"), "{body}");
        assert!(
            bridge.stop(Some("   ")).is_err(),
            "an empty steer is refused"
        );
        assert_eq!(
            std::fs::read_to_string(&control).expect("the steer file"),
            body,
            "a refused steer leaves the standing control alone"
        );

        // A run whose log does not exist yet has nothing to write beside.
        lock(&bridge.state).session = String::new();
        assert!(
            bridge
                .stop(None)
                .unwrap_err()
                .contains("no session log yet"),
            "a faceless run refuses a control"
        );
    }

    /// A control written while the route is being judged is never read
    /// by a run that fails before the loop; left in place it would end
    /// the next run in the same session. The thread clears it.
    #[tokio::test(flavor = "multi_thread")]
    async fn a_leftover_control_does_not_survive_a_run_that_never_started() {
        let home = std::env::temp_dir().join(format!(
            "fleet-gui-control-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).expect("a temp home");
        let log = Session::create(&home, "deepseek-chat").expect("the log");
        let id = log.id().to_string();
        drop(log);
        let control = crate::home::sessions_dir(&home).join(format!("{id}.control"));
        std::fs::write(&control, "{\"stop\":true}\n").expect("the standing control");
        let (mut bridge, sources) =
            Bridge::detached(home.clone(), "deepseek-chat".to_string(), Some(id.clone()));
        let door = bridge.connect();
        let _thread = spawn_agent(AgentInputs {
            home: home.clone(),
            model: "deepseek-chat".to_string(),
            session_id: Some(id),
            ui_tx: sources.ui,
            approval_tx: sources.seat,
            command_rx: door,
            base_url: "http://127.0.0.1:1".to_string(),
            memories_root: home.clone(),
        });
        bridge.run("go").expect("the run is accepted");
        for _ in 0..300 {
            if !control.exists() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        assert!(
            !control.exists(),
            "the run never read the control, and the next one must not either"
        );
    }

    /// KI-022: the launch face has no session until the first task
    /// creates the log it runs in. The test drives the real agent thread
    /// against a home with no roster, so the run fails at routing — but
    /// the log is created before the route is judged, the id rides back
    /// on the event stream, and the state document names it. A face that
    /// is opened and closed without a prompt leaves no log at all.
    #[tokio::test(flavor = "multi_thread")]
    async fn the_first_task_creates_the_log_the_run_is_logged_in() {
        let home = std::env::temp_dir().join(format!(
            "fleet-gui-lazy-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).expect("a temp home");
        // The launch state: no session, because none exists yet.
        let (mut bridge, sources) =
            Bridge::detached(home.clone(), "deepseek-chat".to_string(), None);
        assert_eq!(bridge.state()["session"], "", "{:?}", bridge.state());
        let door = bridge.connect();
        let _thread = spawn_agent(AgentInputs {
            home: home.clone(),
            model: "deepseek-chat".to_string(),
            session_id: None,
            ui_tx: sources.ui,
            approval_tx: sources.seat,
            command_rx: door,
            base_url: "http://127.0.0.1:1".to_string(),
            memories_root: home.clone(),
        });
        bridge.run("go").expect("the run is accepted");
        // The session event lands, and the state document names the id.
        for _ in 0..100 {
            if !bridge.state()["session"].as_str().unwrap_or("").is_empty() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        let state = bridge.state();
        let id = state["session"].as_str().expect("a session id").to_string();
        assert!(
            session_id_is_shape(&id),
            "the created log's id is the shape Session::create mints: {id}"
        );
        assert!(
            home.join("sessions").join(format!("{id}.jsonl")).is_file(),
            "the log the run created exists on disk"
        );
        let events = state["events"].as_array().expect("an event log");
        assert!(
            events.iter().any(
                |event| event["type"] == "session" && event["id"].as_str() == Some(id.as_str())
            ),
            "the id rode the event stream: {events:?}"
        );
        let _ = std::fs::remove_dir_all(&home);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_run_that_is_already_in_flight_is_refused() {
        let (bridge, _sources, _door) = bridge().await;
        let attached = bridge.launch();
        bridge.run(&attached, "first").expect("the first run is accepted");
        let second = bridge.run(&attached, "second");
        assert!(second.is_err(), "{second:?}");
        let state = bridge.state(&attached);
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
            Some("session-test".to_string()),
        );
        let attached = bridge.launch();
        let refused = bridge.run(&attached, "who is there");
        assert!(refused.is_err(), "{refused:?}");
        let state = bridge.state(&attached);
        let events = state["events"].as_array().expect("an event log");
        assert_eq!(events.len(), 2, "{events:?}");
        assert_eq!(events[1]["type"], "failed");
        assert_eq!(state["running"], false);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_seat_crosses_the_approver_and_the_answer_clears_it() {
        let (bridge, sources, _door) = bridge().await;
        let attached = bridge.launch();
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
            if bridge.state(&attached)["approval"].is_object() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        let seat = &bridge.state(&attached)["approval"];
        assert_eq!(seat["tool"], "bash");
        assert_eq!(seat["label"], "bash · cargo");
        assert_eq!(seat["allowlistable"], true);
        assert_eq!(seat["summary"], "run cargo test");
        let answered = bridge.answer(&attached, "once").expect("the seat is answered");
        assert_eq!(answered["answered"], "once");
        assert!(bridge.state(&attached)["approval"].is_null());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn an_answer_with_no_seat_is_a_conflict_not_a_denial() {
        let (bridge, _sources, _door) = bridge().await;
        let attached = bridge.launch();
        let answered = bridge.answer(&attached, "deny");
        assert!(answered.is_err(), "{answered:?}");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_word_that_is_not_a_decision_is_refused_with_the_seat_standing() {
        let (bridge, sources, _door) = bridge().await;
        let attached = bridge.launch();
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
            if bridge.state(&attached)["approval"].is_object() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        let refused = bridge.answer(&attached, "sure");
        assert!(refused.is_err(), "{refused:?}");
        // A malformed answer is not the owner saying no: the seat stands.
        assert!(bridge.state(&attached)["approval"].is_object());
        assert!(decision.try_recv().is_err(), "nothing was sent back");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_remembered_rule_stops_the_prompt_and_rides_the_rules_event() {
        let (bridge, sources, _door) = bridge().await;
        let attached = bridge.launch();
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
            if bridge.state(&attached)["approval"].is_object() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        let mut stream = bridge.subscribe();
        let answered = bridge.answer(&attached, "remember").expect("the rule is remembered");
        assert_eq!(answered["answered"], "remember");
        assert_eq!(answered["rule"], "bash · cargo");
        let state = bridge.state(&attached);
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
        let attached = bridge.launch();
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
            let approval = bridge.state(&attached)["approval"].clone();
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
            .answer(&attached, "remember")
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
            bridge.state(&attached)["rules"]
                .as_array()
                .is_none_or(|rules| rules.is_empty()),
            "{}",
            bridge.state(&attached)["rules"]
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn the_core_events_land_ordered_and_the_state_replays_them() {
        let (bridge, sources, _door) = bridge().await;
        let attached = bridge.launch();
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
            if bridge.state(&attached)["events"].as_array().map(Vec::len) == Some(4) {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        let state = bridge.state(&attached);
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
        // The gate the verify step names is the failover's own knob.
        assert_eq!(state["gate"]["name"], "sys1");
        assert_eq!(
            state["gate"]["informs_failover"],
            crate::route::calibration_config(&bridge.home).informs_failover
        );
    }

    /// The pane's tree: one directory's entries, with each entry's kind
    /// and size. The root is the process's own working directory — the
    /// same rule the memory corpus resolves by — so the browser sees the
    /// tree this face was launched from and nothing above it.
    #[tokio::test(flavor = "multi_thread")]
    async fn the_tree_reads_one_directory_under_the_root() {
        let (bridge, _sources, _door) = bridge().await;
        let here = std::env::current_dir().expect("a working directory");
        let document = bridge.tree("src").await;
        assert_eq!(document["type"], "tree");
        assert_eq!(document["root"], here.display().to_string());
        let entries = document["entries"].as_array().expect("the entries");
        assert!(
            entries
                .iter()
                .any(|entry| entry["name"] == "gui" && entry["kind"] == "dir"),
            "{document}"
        );
        // A path that escapes the root is refused by name, and the
        // refusal says so rather than serving the parent.
        let escaped = bridge.tree("../../etc").await;
        assert_eq!(escaped["refused"][0], "outside the working directory");
    }

    /// The pane's file read: the text, its language for the viewer, and
    /// whether it was cut. A credential file is refused by name — the
    /// own-home ruling is that no credential ever reaches the browser,
    /// and a file browser is exactly where that would leak.
    #[tokio::test(flavor = "multi_thread")]
    async fn the_file_read_carries_the_text_and_refuses_a_credential() {
        let (bridge, _sources, _door) = bridge().await;
        let document = bridge.file("src/gui/assets/app.css").await;
        assert_eq!(document["type"], "file");
        assert_eq!(document["language"], "css");
        let text = document["text"].as_str().expect("the text");
        assert!(text.contains("--dsw-alias-page"), "{text}");
        assert_eq!(document["truncated"], false);
        // The dotenv is refused by name, with the reason the pane shows.
        let dotenv = bridge.file(".env").await;
        assert_eq!(dotenv["refused"][0], "a credential file is never served");
    }

    /// A deck rides the event log the way every other intelligence card
    /// does: the agent emits it, `describe` shapes it, and the page
    /// reduces it out of the one log. The fragment rides the event rather
    /// than the tool result the model reads, so the two audiences get
    /// what each asked for.
    #[tokio::test(flavor = "multi_thread")]
    async fn a_deck_rides_the_event_log_with_its_fragment() {
        let (bridge, sources, _door) = bridge().await;
        let attached = bridge.launch();
        sources
            .ui
            .send(UiEvent::Deck {
                project: "presentation".to_string(),
                nodes: 2,
                html: "<section class=\"deck\" data-type=\"presentation\">x</section>".to_string(),
                faults: "[{\"trigger\":\"overfull\",\"node\":\"slide_01\"}]".to_string(),
                original_deck: Some("{}".to_string()),
                fix_deck: Some("{}".to_string()),
                fix_actions: vec!["`slide_01`: compressed".to_string()],
            })
            .expect("the deck lands");
        // The fan-out is a task of the caller's, so the event needs a
        // moment to cross it.
        for _ in 0..50 {
            let events = bridge.state(&attached)["events"]
                .as_array()
                .map(Vec::len)
                .unwrap_or(0);
            if events > 0 {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        let document = bridge.state(&attached);
        let events = document["events"].as_array().expect("the event log");
        let deck = events
            .iter()
            .find(|event| event["type"] == "deck")
            .expect("the deck event");
        assert_eq!(deck["project"], "presentation");
        assert_eq!(deck["nodes"], 2);
        assert_eq!(
            deck["html"],
            "<section class=\"deck\" data-type=\"presentation\">x</section>"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_failed_run_ends_the_running_flag() {
        let (bridge, sources, _door) = bridge().await;
        let attached = bridge.launch();
        bridge.run(&attached, "go").ok();
        sources
            .ui
            .send(UiEvent::Failed("routing config: no roster".to_string()))
            .expect("the failure lands");
        for _ in 0..50 {
            if !bridge.state(&attached)["running"].as_bool().unwrap_or(true) {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        let state = bridge.state(&attached);
        assert_eq!(state["running"], false);
        assert_eq!(state["events"][1]["type"], "failed");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_resume_reseeds_the_view_and_hands_the_thread_the_anchor() {
        // A home with one real log, unique per call: the resume reads it,
        // and the door receives what the view already showed.
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let serial = NEXT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let home =
            std::env::temp_dir().join(format!("fleet-gui-resume-{}-{serial}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).expect("a temp home");
        let mut log = Session::create(&home, "deepseek-chat").expect("the log");
        let id = log.id().to_string();
        log.record(&session::Record::User {
            content: "carry me over".to_string(),
        })
        .expect("the record");
        let (mut bridge, _sources) = Bridge::detached(
            home,
            "deepseek-chat".to_string(),
            Some("session-test".to_string()),
        );
        let attached = bridge.launch();
        let mut door = bridge.connect();
        // A run in flight in the session being resumed refuses before
        // anything on the face is touched. A run in some OTHER session
        // does not: two surfaces may work in two conversations, and one
        // running does not freeze the other.
        lock(&bridge.state).attached(&id).running = true;
        let refused = bridge.resume(&id).await.unwrap_err();
        assert!(refused.contains("in flight"), "{refused}");
        lock(&bridge.state).attached(&id).running = false;
        // A shape that is not an id is refused for its shape, before the
        // disk is asked anything.
        assert!(bridge.resume("../escape").await.is_err());
        let resumed = bridge.resume(&id).await.expect("resumed");
        assert_eq!(resumed["session"], id.as_str());
        assert_eq!(resumed["agent"], true, "the door took the command");
        match door.recv().await.expect("the command") {
            Command::Resume(delivered) => assert_eq!(delivered, id),
            _ => panic!("a resume delivered some other command"),
        }
        // The resumed session's record holds the log's cards, and the
        // launch session's record is untouched by the resume — the two
        // are independent now, which is the whole point of the table.
        let state = bridge.state(&id);
        assert_eq!(state["session"], id.as_str(), "{state}");
        let events = state["events"].as_array().expect("an event log");
        assert!(
            events
                .iter()
                .any(|event| event["type"] == "message" && event["text"] == "carry me over"),
            "the view reseeded from the log: {events:?}"
        );
        let launch = bridge.state(&attached);
        assert!(
            launch["events"].as_array().is_none_or(|events| events.is_empty()),
            "the launch session kept its own log: {launch}"
        );
    }

    /// Two sessions attached to one bridge hold independent transcripts,
    /// running flags, and seats: a phone and a desktop looking at two
    /// conversations do not disturb each other.
    #[tokio::test(flavor = "multi_thread")]
    async fn two_attached_sessions_are_independent() {
        let home = std::env::temp_dir().join(format!(
            "fleet-gui-bridge-two-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).expect("a temp home");
        let (mut bridge, _sources) = Bridge::detached(
            home.clone(),
            "deepseek-chat".to_string(),
            "session-one".to_string(),
        );
        // The door stands open with no thread behind it, so the commands
        // are accepted and nothing runs: this test is about the state,
        // not the agent.
        let _door = bridge.connect();
        let one = bridge.launch();
        let two = "session-two".to_string();

        // A run starts in one; the other session's flag does not move.
        bridge.run(&one, "work in one").expect("accepted");
        assert_eq!(bridge.state(&one)["running"], true);
        assert_eq!(bridge.state(&two)["running"], false, "{two} is not running");
        assert_eq!(bridge.state(&two)["attached"], false, "{two} was never opened");

        // The owner's message landed in one's log and not in two's.
        let events = bridge.state(&one)["events"].as_array().expect("a log").len();
        assert_eq!(events, 1, "one event in one's log");
        assert_eq!(
            bridge
                .state(&two)["events"]
                .as_array()
                .map(Vec::len)
                .unwrap_or(0),
            0,
            "two's log is empty"
        );

        // A second run in the other session is refused while the first is
        // in flight in ITS session — the thread's loop is serial — but the
        // two sessions' flags say so independently.
        let second = bridge.run(&two, "work in two");
        assert!(second.is_ok(), "the command is accepted; the thread is serial, not blocked: {second:?}");
        assert_eq!(bridge.state(&one)["running"], true);
        assert_eq!(bridge.state(&two)["running"], true);

        let _ = std::fs::remove_dir_all(&home);
    }
}
