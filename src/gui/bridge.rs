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
    /// Run this task in this session, working in `folder` when the
    /// chat names one. The session is part of the command because the
    /// thread drives more than one: a phone and a desktop may each be
    /// working in a different conversation. The folder rides here
    /// rather than being read when the thread wakes, so a choice made
    /// while the run was being accepted cannot move under it.
    Run(String, String, Option<PathBuf>),
    /// Plan this task: the run plans and does not write, and its final
    /// message records as a plan the owner edits and builds.
    Plan(String, String, Option<PathBuf>),
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
    /// When that seat was raised, in epoch milliseconds, so the wait is
    /// honest on every path a seat can arrive by — the stream's event and
    /// the state document alike. 0 when no seat is waiting.
    seat_at: u64,
}

impl SessionState {
    fn new() -> SessionState {
        SessionState {
            events: VecDeque::new(),
            dropped: 0,
            running: false,
            seat: None,
            seat_at: 0,
        }
    }
}

/// Now, in epoch milliseconds — the clock a seat's wait is measured on.
fn epoch_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|span| span.as_millis() as u64)
        .unwrap_or_default()
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
    /// A re-run waiting on its seat (D11): the ask-class call the owner
    /// pressed re-run on, standing the seat the run path would stand.
    /// Executed by `answer` when the decision lands, never logged.
    pending_replay: Option<(String, String, serde_json::Value)>,
    /// The launch session's id, once the thread has named it. `None`
    /// until then: the log is created on the first task (KI-022), so a
    /// face that is opened and closed without a prompt leaves no empty
    /// log behind.
    launch: Option<String>,
    driving: Option<String>,
    /// The face's run policy: how tool calls are approved for the next
    /// run, and the provider the next route is pinned to. The command
    /// loop is serial, so one pair is the whole state.
    permissions: crate::approvals::Mode,
    provider_pin: Option<String>,
    /// The next run's turn budget: the home's config by default, the
    /// run body's override when the composer named one for this run.
    /// Serial like the pair beside it; 0 means the config value.
    turn_budget: usize,
    /// Each attached chat's working folder, keyed the way sessions are:
    /// the launch session's pending key is the empty string until the
    /// thread names the log, then it moves with it (see
    /// [`GuiState::name_launch`]). The value is the canonical absolute
    /// path the run's tools work in; an absent entry is the launch
    /// folder. Persisted in `<FLEET_HOME>/sessions-workdirs.json`, so a
    /// reload restores the choice.
    workdirs: std::collections::HashMap<String, String>,
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
    /// same events under the id it will show. A folder chosen before the
    /// first task moves with it and is returned, because the sidecar is
    /// keyed by session id and the caller is the one that owns the home.
    fn name_launch(&mut self, id: &str) -> Option<String> {
        if let Some(pending) = self.sessions.remove("") {
            self.sessions.insert(id.to_string(), pending);
        }
        self.launch = Some(id.to_string());
        let folder = self.workdirs.remove("");
        if let Some(folder) = &folder {
            self.workdirs.insert(id.to_string(), folder.clone());
        }
        folder
    }

    /// The folder this chat's runs work in, when the owner set one.
    fn workdir_of(&self, session: &str) -> Option<String> {
        self.workdirs.get(&self.key(session)).cloned()
    }

    /// Set or clear a chat's folder. `None` is no folder — and
    /// therefore no filesystem access — which is the absence of an
    /// entry rather than an entry naming one.
    fn set_workdir(&mut self, session: &str, folder: Option<&str>) -> Option<String> {
        let key = self.key(session);
        match folder {
            Some(folder) => self.workdirs.insert(key, folder.to_string()),
            None => self.workdirs.remove(&key),
        }
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
    /// The channel service this face hosts, when it hosts one. The face
    /// builds the service after the bridge (the service needs it), so
    /// this is attached rather than constructed — and a face with no
    /// service leaves it `None`, which is what makes the control plane's
    /// start say "not hosted here" rather than pretend.
    channels: Mutex<Option<Arc<crate::channels::ChannelService>>>,
    /// The remote face this process hosts: the address a phone reaches
    /// and the port it reaches it on. Attached at launch, because the
    /// pairing card has to know whether a code can be redeemed at all.
    remote: Mutex<Option<(std::net::IpAddr, u16)>>,
    /// The workspace-calls feed's tail state: per-trace-file read
    /// offsets, so a poll answers only what moved. Shared because the
    /// reads run on blocking workers while the bridge answers requests.
    trace_tail: Arc<std::sync::Mutex<crate::layerfs_view::TraceTail>>,
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
            pending_replay: None,
            launch,
            driving: None,
            permissions: crate::approvals::Mode::Ask,
            provider_pin: None,
            turn_budget: 0,
            workdirs: load_workdirs(&home),
        }));
        let rules = Arc::new(Mutex::new(crate::settings::approval_rules(&home)));
        // The fan-out persists a folder that was chosen before the
        // launch session had an id, so it needs the home the sidecar
        // lives in.
        let fan_home = home.clone();
        tokio::spawn(fan_out(
            ui_rx,
            seat_rx,
            Arc::clone(&state),
            events.clone(),
            fan_home,
        ));
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
            channels: Mutex::new(None),
            remote: Mutex::new(None),
            trace_tail: Arc::new(std::sync::Mutex::new(
                crate::layerfs_view::TraceTail::default(),
            )),
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

    /// Name the session being driven, exactly as the agent thread does
    /// before each run. The seam exists so a test can stand in for the
    /// thread — the seat a run raises belongs to the session the thread
    /// is driving, and a test with no thread has to say which one that
    /// is.
    #[cfg(test)]
    pub(crate) fn drive(&self, session: &str) {
        lock(&self.state).driving = Some(session.to_string());
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
        lock(&self.state).launch.clone().unwrap_or_default()
    }

    /// The home this face's sessions live under. A channel driver asks
    /// for it to create a peer's first log; nothing else outside this
    /// module has any business writing there. It is not `home`, which is
    /// the welcome's document.
    pub fn home_path(&self) -> PathBuf {
        self.home.clone()
    }

    /// The model this face runs. A channel driver's peer sessions run
    /// the same model the face does, because it is the same core.
    pub fn model(&self) -> String {
        self.model.clone()
    }

    /// Create a new conversation and return its id: the New control's
    /// one write. The log exists immediately, empty but for its `start`
    /// record, so the work list can show the conversation the owner just
    /// made — and the next run, posted with this id, writes into it.
    pub fn new_session(&self) -> Result<String, String> {
        let created = session::Session::create(&self.home, &self.model)
            .map_err(|error| format!("could not create a conversation: {error}"))?;
        Ok(created.id().to_string())
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
        // The gate the verify step names: the same knob the failover's
        // decision reads, never a second interpretation of it. It is one
        // small config file and the page asks for state once per load,
        // so it is read here rather than kept as another copy of a fact
        // the route module owns.
        let gate = crate::route::calibration_config(&self.home);
        // The map key the addressed session reads: the empty string is
        // the face's own launch session, which is the pending log before
        // the thread names it and the named one after.
        let key = state.key(session);
        // A session nobody has opened reads as empty rather than as an
        // error, and it reads the same keys the attached document does:
        // a surface that asks about one gets an answer it can render
        // without branching on the shape.
        let Some(attached) = state.attached_if(session) else {
            return json!({
                "session": key,
                "model": self.model,
                "workspace": self.cwd.display().to_string(),
                "folder": folder_value(state.workdir_of(session).as_deref(), &self.cwd),
            "goal": session_goal(&self.home, &key),
                "budget": crate::compaction::COMPACTION_TOKEN_BUDGET,
                "turn_budget": crate::route::run_config(&self.home).max_turns,
                "gate": {
                    "name": "sys1",
                    "informs_failover": gate.informs_failover,
                },
                "running": false,
                // The session this face is executing, if any. A surface
                // reading a conversation that is not the driving one
                // knows a run is elsewhere instead of guessing from a
                // process-global flag (the run belongs to a session).
                "driving": state.driving.clone(),
                "dropped": 0,
                "events": [],
                "approval": null,
                "rules": rules.list().iter().map(rule_value).collect::<Vec<_>>(),
                "attached": false,
            });
        };
        json!({
            "session": key,
            "model": self.model,
            "workspace": self.cwd.display().to_string(),
            "folder": folder_value(state.workdir_of(session).as_deref(), &self.cwd),
            "goal": session_goal(&self.home, &key),
            "budget": crate::compaction::COMPACTION_TOKEN_BUDGET,
            "turn_budget": crate::route::run_config(&self.home).max_turns,
            "gate": {
                "name": "sys1",
                "informs_failover": gate.informs_failover,
            },
            "running": attached.running,
            "driving": state.driving.clone(),
            "dropped": attached.dropped,
            "events": attached.events,
            "approval": attached.seat.as_ref().map(|seat| {
                json!({
                    // The seat names its session on both delivery paths:
                    // the stream's `seat` event carries it, and this
                    // document's copy has to as well, or an answer to a
                    // seat that arrived by the state read posts
                    // session-less and is answered against the launch
                    // session (KI-286).
                    "session": key,
                    "since": attached.seat_at,
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

        // The welcome is this face's own conversation: the launch
        // session, which is the one a visitor's browser opens whether or
        // not anything has been resumed.
        let launch = self.launch();
        let state = self.state(&launch);
        // The four reads are independent; run them together so the
        // welcome's first paint pays the slowest one, not their sum.
        let (sessions, memory, skills, delegation) = tokio::join!(
            self.sessions(&launch, ""),
            self.memory(),
            self.skills(),
            self.delegation(&launch)
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
        let cwd = self.cwd.clone();
        match tokio::task::spawn_blocking(move || memory_read(&home, &memories, &cwd)).await {
            Ok(document) => document,
            Err(_) => json!({
                "type": "memory",
                "refused": ["the memory read did not land"],
            }),
        }
    }

    /// One document attachment: the local lane first, Perception when the
    /// file is one local extraction cannot read. Escalation needs the
    /// owner's consent, because it sends the document to a second party;
    /// the first call answers with what it would send and why.
    pub async fn attach(&self, body: &str) -> Result<Value, String> {
        let arguments: serde_json::Value = serde_json::from_str(body)
            .map_err(|error| format!("the attach is not a JSON object: {error}"))?;
        let asked = arguments
            .get("path")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_string();
        if asked.is_empty() {
            return Err("an attach names a file path".to_string());
        }
        let consent = arguments
            .get("consent")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        let home = self.home.clone();
        let cwd = self.cwd.clone();
        let decision = tokio::task::spawn_blocking(move || attach_local(&cwd, &asked, consent))
            .await
            .map_err(|_| "the attach did not land".to_string())??;
        match decision {
            Attachment::Ready(value) | Attachment::Needs(value) => Ok(value),
            Attachment::Escalate { path, name, reason } => {
                // The lane holds the credential and runs the vendor flow;
                // fleet only hands it the path.
                let spec = crate::lanes::spec_for("sys1")
                    .map_err(|error| format!("the sys1 lane cannot start: {error}"))?;
                let parsed = crate::perception::parse_via_lane(&path, &name, &home, spec).await?;
                Ok(json!({
                    "type": "attachment",
                    "name": name,
                    "text": parsed.markdown,
                    "parser": parsed.parser,
                    "pages": parsed.pages,
                    "units": parsed.units,
                    "job_id": parsed.job_id,
                    "reason": reason,
                    "truncated": parsed.truncated.map(|(original, retained)| json!({
                        "original": original,
                        "retained": retained,
                    })),
                }))
            }
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

    /// The workspace-history section's document: the launch folder's
    /// project as one tree — the branches with their fork bases, their
    /// newest commits, main's Layers, the Store's figures, and each
    /// child branch's state and session, all from ONE backend read that
    /// carries a `version`. A `since` version the substrate still holds
    /// current answers `unchanged` and the caller's view stands. The
    /// read is the substrate's own, so an unprovisioned machine or an
    /// unregistered folder names itself here rather than drawing an
    /// empty tree.
    pub async fn workspace_history(&self, since: Option<String>) -> Value {
        let cwd = self.cwd.clone();
        match tokio::task::spawn_blocking(move || {
            crate::layerfs_view::workspace_history(&cwd, since.as_deref())
        })
        .await
        {
            Ok(document) => document,
            Err(_) => json!({
                "type": "workspace-history",
                "refused": ["the workspace-history read did not land"],
            }),
        }
    }

    /// The instrument's file read: text, blame, revisions, or a node's
    /// diff — one route, four modes, caps named when they bite.
    pub async fn workspace_file(
        &self,
        ref_: &str,
        path: &str,
        mode: &str,
        from: Option<String>,
        to: Option<String>,
        page: u32,
    ) -> Value {
        let cwd = self.cwd.clone();
        let ref_ = ref_.to_string();
        let path = path.to_string();
        let mode = mode.to_string();
        match tokio::task::spawn_blocking(move || {
            crate::layerfs_view::workspace_file(
                &cwd,
                &ref_,
                &path,
                &mode,
                from.as_deref(),
                to.as_deref(),
                page,
            )
        })
        .await
        {
            Ok(document) => document,
            Err(_) => json!({
                "type": "workspace-file",
                "refused": ["the workspace-file read did not land"],
            }),
        }
    }

    /// The live feed's read: the substrate's calls, tailed from the
    /// machine's traces. The tail state is this bridge's own, so a poll
    /// answers only what moved since the last one.
    pub async fn workspace_calls(&self, since: Option<String>) -> Value {
        let cwd = self.cwd.clone();
        let tail = Arc::clone(&self.trace_tail);
        match tokio::task::spawn_blocking(move || {
            let mut tail = tail
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            crate::layerfs_view::workspace_calls(&cwd, since.as_deref(), &mut tail)
        })
        .await
        {
            Ok(document) => document,
            Err(_) => json!({
                "type": "workspace-calls",
                "refused": ["the workspace-calls read did not land"],
            }),
        }
    }

    /// The Files pane's one page of a branch's tree. Read-only by
    /// construction: the pane lists and reads text, and every write
    /// verb stays the delegated child's, behind the arm.
    pub async fn workspace_files(&self, branch: &str, path: &str, page: u32) -> Value {
        let cwd = self.cwd.clone();
        let branch = branch.to_string();
        let path = path.to_string();
        match tokio::task::spawn_blocking(move || {
            crate::layerfs_view::workspace_files(&cwd, &branch, &path, page)
        })
        .await
        {
            Ok(document) => document,
            Err(_) => json!({
                "type": "workspace-files",
                "refused": ["the workspace-files read did not land"],
            }),
        }
    }

    /// Who may reach this face: the paired devices and the channel
    /// peers, as one document. The store holds digests, so this names
    /// who is paired and never what would let them in — the same masking
    /// rule the rest of the surface holds.
    ///
    /// A loopback-face read only: the remote face's allowlist keeps this
    /// closed, because a phone that could revoke devices is a phone that
    /// could lock its owner out.
    pub async fn access(&self) -> Value {
        let home = self.home.clone();
        match tokio::task::spawn_blocking(move || access_read(&home)).await {
            Ok(document) => document,
            Err(_) => json!({
                "type": "access",
                "refused": ["the access read did not land"],
            }),
        }
    }

    /// Forget a paired device or a channel peer by kind and id. The
    /// store's own save is the writer, so the revocation is the same one
    /// the CLI performs — there is no second path to the file.
    pub async fn access_revoke(&self, kind: &str, id: &str) -> Result<Value, String> {
        let home = self.home.clone();
        let kind = kind.trim().to_string();
        let id = id.trim().to_string();
        tokio::task::spawn_blocking(move || match kind.as_str() {
            "device" => {
                let mut store = crate::remote::pairing::DeviceStore::load(&home)
                    .map_err(|error| error.to_string())?;
                if store.revoke(&id) {
                    store.save(&home).map_err(|error| error.to_string())?;
                    Ok(json!({"type": "revoked", "kind": "device", "id": id}))
                } else {
                    Err(format!("no paired device named `{id}`"))
                }
            }
            "peer" => {
                // Every channel's store is tried: a peer's id is unique
                // within its channel, so the first store that knows it is
                // the one that paired it.
                for channel in crate::channels::CHANNELS {
                    let mut store = crate::channels::PeerStore::load(&home, channel)
                        .map_err(|error| error.to_string())?;
                    if store.revoke(&id) {
                        store
                            .save(&home, channel)
                            .map_err(|error| error.to_string())?;
                        let mut pending = crate::channels::PendingStore::load(&home, channel);
                        pending.forget(&id);
                        let _ = pending.save(&home, channel);
                        return Ok(json!({
                            "type": "revoked",
                            "kind": "peer",
                            "channel": channel,
                            "id": id,
                        }));
                    }
                }
                Err(format!("no channel peer named `{id}`"))
            }
            other => Err(format!("`{other}` is not a thing that can be revoked")),
        })
        .await
        .map_err(|_| "the revocation did not land".to_string())?
    }

    /// The channel service's control plane as one bounded document: the
    /// service's own state (and whether its heartbeat still beats), every
    /// channel's paired and pending peers, and the newest activity lines
    /// with the bound named so a surface can say what it is not showing.
    ///
    /// It reads the same files `fleet channels` and the TUI read, so the
    /// three cannot disagree about who can reach the face or whether the
    /// service is up.
    pub async fn channels_read(&self) -> Value {
        let home = self.home.clone();
        let mut document = tokio::task::spawn_blocking(move || channels_read(&home))
            .await
            .unwrap_or_else(|_| {
                json!({
                    "type": "channels",
                    "refused": ["the channel read did not land"],
                })
            });
        // A peer waiting on an answer is the thing an owner most needs to
        // notice, and only this process knows it: the seat lives in the
        // live state, not on disk. The activity log names the session
        // each peer's delivery landed in, so the two meet here.
        let waiting = {
            let state = lock(&self.state);
            state
                .sessions
                .iter()
                .filter(|(_, session)| session.seat.is_some())
                .map(|(key, _)| key.clone())
                .collect::<Vec<_>>()
        };
        let mut peer_waiting = 0u64;
        if let Some(lines) = document.get("activity").and_then(Value::as_array) {
            let mut seen: Vec<&str> = Vec::new();
            for line in lines.iter().rev() {
                let Some(session) = line.get("session").and_then(Value::as_str) else {
                    continue;
                };
                if seen.contains(&session) {
                    continue;
                }
                seen.push(session);
                if waiting.iter().any(|held| held == session) {
                    peer_waiting += 1;
                }
            }
        }
        document["waiting_seats"] = json!(peer_waiting);
        document
    }

    /// Start one channel through the service the face hosts. A face with
    /// no service says so rather than pretending: the surface's start
    /// button is only meaningful in the process that owns the ports.
    pub async fn channel_start(&self, name: &str) -> Result<Value, String> {
        let Some(service) = self.hosted_channels() else {
            return Err(
                "no channel service is hosted here; start the face with the channel service on"
                    .to_string(),
            );
        };
        let report = service.start(name.trim()).await;
        Ok(json!({
            "type": "channel",
            "name": report.name,
            "phase": report.phase,
            "port": report.port,
            "error": report.error,
        }))
    }

    /// Stop one channel.
    pub async fn channel_stop(&self, name: &str) -> Result<Value, String> {
        let Some(service) = self.hosted_channels() else {
            return Err("no channel service is hosted here; there is nothing to stop".to_string());
        };
        let report = service.stop(name.trim());
        Ok(json!({
            "type": "channel",
            "name": report.name,
            "phase": report.phase,
        }))
    }

    /// Pair a peer from the app. The token the pairing mints is in this
    /// reply and nowhere else: the store keeps its digest, no later read
    /// carries it, and the surface shows it in a card that is dismissed
    /// once. That is the rule the device pairing already holds.
    pub async fn channel_pair(
        &self,
        channel: &str,
        id: &str,
        name: &str,
        capability: &str,
        callback: &str,
    ) -> Result<Value, String> {
        let channel = channel.trim().to_string();
        let id = id.trim().to_string();
        let name = name.trim().to_string();
        let callback = callback.trim().to_string();
        let capability = parse_capability(capability)?;
        let home = self.home.clone();
        tokio::task::spawn_blocking(move || {
            let pairing = crate::channels::pair(
                &home,
                &channel,
                &id,
                &name,
                capability,
                if callback.is_empty() {
                    None
                } else {
                    Some(&callback)
                },
            )?;
            Ok(json!({
                "type": "paired",
                "channel": pairing.channel,
                "id": pairing.id,
                "name": pairing.name,
                "capability": match pairing.capability {
                    crate::channels::Capability::Task => "task",
                    crate::channels::Capability::Observe => "observe",
                },
                // The one and only copy. A surface that re-reads this
                // gets `null`, because nothing else has it.
                "token": pairing.token,
                "callback": pairing.callback,
                "reveal": "this is the only time the token is shown; store it now",
            }))
        })
        .await
        .map_err(|_| "the pairing did not land".to_string())?
    }

    /// Change a paired peer's capability. `Observe` is real here: a peer
    /// demoted to listening keeps its conversation and loses the door a
    /// task comes through.
    pub async fn channel_capability(
        &self,
        channel: &str,
        id: &str,
        capability: &str,
    ) -> Result<Value, String> {
        let channel = channel.trim().to_string();
        let id = id.trim().to_string();
        let capability = parse_capability(capability)?;
        let home = self.home.clone();
        tokio::task::spawn_blocking(move || {
            crate::channels::set_capability(&home, &channel, &id, capability)?;
            Ok(json!({
                "type": "capability",
                "channel": channel,
                "id": id,
                "capability": match capability {
                    crate::channels::Capability::Task => "task",
                    crate::channels::Capability::Observe => "observe",
                },
            }))
        })
        .await
        .map_err(|_| "the capability change did not land".to_string())?
    }

    /// Refuse a pending peer: forget the attempts so the surface stops
    /// offering it. It is not blocked — there is nothing to block — it is
    /// simply not remembered until it tries again.
    pub async fn channel_forget_pending(&self, channel: &str, id: &str) -> Result<Value, String> {
        let channel = channel.trim().to_string();
        let id = id.trim().to_string();
        let home = self.home.clone();
        tokio::task::spawn_blocking(move || {
            let mut pending = crate::channels::PendingStore::load(&home, &channel);
            if !pending.forget(&id) {
                return Err(format!("no pending {channel} peer named `{id}`"));
            }
            pending
                .save(&home, &channel)
                .map_err(|error| error.to_string())?;
            Ok(json!({"type": "forgotten", "channel": channel, "id": id}))
        })
        .await
        .map_err(|_| "the change did not land".to_string())?
    }

    /// The service this face hosts, when it hosts one. The face attaches
    /// it after building it — the service needs the bridge, so it cannot
    /// be a constructor argument without a cycle.
    pub fn hosted_channels(&self) -> Option<Arc<crate::channels::ChannelService>> {
        lock(&self.channels).clone()
    }

    /// Attach the hosted service, so the control plane's start and stop
    /// reach the process that owns the ports.
    pub fn attach_channels(&self, service: Arc<crate::channels::ChannelService>) {
        *lock(&self.channels) = Some(service);
    }

    /// Attach the remote face's address and port, so the pairing card can
    /// mint a code for a listener that actually exists.
    pub fn attach_remote(&self, address: std::net::IpAddr, port: u16) {
        *lock(&self.remote) = Some((address, port));
    }

    /// The remote face this process hosts, when it hosts one.
    pub fn remote_face(&self) -> Option<(std::net::IpAddr, u16)> {
        *lock(&self.remote)
    }

    /// Mint a pairing window and the card's document: the URL on the
    /// origin that will work from the phone, the code in display form,
    /// and the QR. Loopback only — the remote listener's allowlist has no
    /// such route, and a paired phone does not pair phones.
    pub async fn pair_window(&self) -> Result<Value, String> {
        let Some((address, port)) = self.remote_face() else {
            return Err(
                "the remote face is not running in this process; start `fleet gui --remote`"
                    .to_string(),
            );
        };
        let home = self.home_path();
        tokio::task::spawn_blocking(move || {
            let (_window, code) = crate::remote::pairing::PairingWindow::mint(&home)
                .map_err(|error| format!("could not mint a pairing code: {error}"))?;
            let origin = crate::remote::pairing_origin(&home, address, port);
            let url = crate::remote::pairing_url_at(&origin.base, &code);
            let svg = crate::remote::qr::encode(&url)
                .map(|qr| qr.to_svg())
                .map_err(|error| format!("could not draw the QR: {error}"))?;
            Ok(json!({
                "type": "pair-window",
                "url": url,
                "code": crate::remote::pairing::display_code(&code),
                "origin": origin.base,
                "origin_kind": origin.kind.key(),
                "origin_label": origin.kind.label(),
                "note": origin.note,
                "ttl_minutes": crate::remote::pairing::WINDOW_TTL_MINUTES,
                "svg": svg,
            }))
        })
        .await
        .map_err(|_| "the pairing window did not land".to_string())?
    }

    /// The sessions panel's document: the archive as the summaries the
    /// log heads carry, newest first, with the session this face
    /// anchors named so the panel can say which row is live.
    pub async fn sessions(&self, current: &str, query: &str) -> Value {
        let home = self.home.clone();
        let current = current.to_string();
        let query = query.to_string();
        // The live facts come from the face's own state, not the log: an
        // approval and a driving run are process truths, and the archive
        // is where the owner looks when they are waiting on one.
        let live = {
            let state = lock(&self.state);
            LiveFace {
                driving: state.driving.clone(),
                seats: state
                    .sessions
                    .iter()
                    .filter(|(_, attached)| attached.seat.is_some())
                    .map(|(id, _)| id.clone())
                    .collect(),
            }
        };
        match tokio::task::spawn_blocking(move || sessions_read(&home, &current, &live, &query))
            .await
        {
            Ok(document) => document,
            Err(_) => json!({
                "type": "sessions",
                "refused": ["the archive read did not land"],
            }),
        }
    }

    /// One recorded call, re-run (D11): through the tools' own executor
    /// — the same bound, the same approval classes — with the result
    /// landing in the reply the drawer reads, never in the session log:
    /// the log is the model's context, and a result the model never
    /// asked for must not enter it. The write class is refused by name
    /// (a re-run would rewrite the tree with no checkpoint behind it);
    /// an ask-class call stands the seat the run path would stand, and
    /// `answer` executes it when the decision lands.
    pub async fn replay_tool(
        &self,
        session: &str,
        name: &str,
        arguments: &str,
    ) -> Result<Value, String> {
        if name == "write_file" {
            return Err(
                "a re-run of write_file would rewrite the tree with no checkpoint behind it; \
                 edit-and-resend is the correction path — it rewinds and checkpoints"
                    .to_string(),
            );
        }
        let parsed: serde_json::Value = serde_json::from_str(arguments)
            .map_err(|error| format!("the arguments are not a JSON object: {error}"))?;
        // A lane call re-runs through the roster's one-call seam — the
        // health probe's own pattern: spawn, handshake, call, close.
        if let Some(rest) = name.strip_prefix("mcp__") {
            let (lane, tool) = rest
                .split_once("__")
                .ok_or("a lane call is mcp__<lane>__<tool> — the recorded name did not split")?;
            let lane = lane.to_string();
            let tool = tool.to_string();
            let lane_key = lane.clone();
            let tool_name = tool.clone();
            let dotenv: std::collections::BTreeMap<String, String> =
                crate::home::read_dotenv_with_process_vault(&self.home)
                    .into_iter()
                    .collect();
            // The 2026-10-26 plan: the registry's named absence is not a
            // failed call — an unkeyed server costs its own tools, and a
            // name the registry cannot serve falls through to the
            // vendored lanes exactly as an unknown name does.
            let mut registry = crate::mcp_registry::enabled_specs(&self.home, &dotenv)
                .map_err(|reason| format!("the registry refused: {reason}"))?
                .specs;
            let spec = match registry.remove(&lane_key) {
                Some(spec) => spec,
                None => crate::lanes::spec_for(&lane_key)?,
            };
            let outcome = tokio::task::spawn_blocking(move || async move {
                let server = crate::mcp::McpServer::spawn(&spec)
                    .map_err(|error| format!("the lane could not start: {error}"))?;
                server
                    .initialize()
                    .await
                    .map_err(|error| format!("the lane's handshake failed: {error}"))?;
                let result = server
                    .call_tool(&tool_name, parsed)
                    .await
                    .map_err(|error| format!("the call failed: {error}"))?;
                let _ = server.close().await;
                Ok::<_, String>(result)
            })
            .await
            .map_err(|_| "the replay did not land".to_string())?
            .await;
            let result = outcome?;
            let bounded = crate::tools::bound(crate::tools::ToolOutcome {
                output: result.text(),
                truncated: false,
            });
            return Ok(json!({
                "type": "replay",
                "name": name,
                "output": bounded.output,
                "truncated": bounded.truncated,
                "logged": false,
            }));
        }
        // A built-in: the executor's own bound, and the approval gate's
        // own question. A call the gate would ask about stands the seat.
        let tool = crate::tools::Tool::from_name(name)
            .ok_or_else(|| format!("`{name}` is not a tool this face re-runs"))?;
        if tool == crate::tools::Tool::WriteFile {
            return Err("the write class is refused by name".to_string());
        }
        let call = crate::api::ToolCall {
            id: "replay".to_string(),
            kind: "function".to_string(),
            function: crate::api::FunctionCall {
                name: name.to_string(),
                arguments: arguments.to_string(),
            },
        };
        let mode = { lock(&self.state).permissions };
        let gate = crate::approvals::Gate::new(mode);
        if !gate.approves_without_asking(&call) {
            let session_key = if session.is_empty() {
                {
                    lock(&self.state).target()
                }
            } else {
                session.to_string()
            };
            {
                let mut state = lock(&self.state);
                state.pending_replay = Some((session_key.clone(), name.to_string(), parsed));
                let attached = state.attached(&session_key);
                attached.seat = Some(ApprovalRequest {
                    call: call.clone(),
                    summary: crate::agent::call_summary(&call),
                    responder: {
                        let (tx, _rx) = std::sync::mpsc::channel();
                        tx
                    },
                });
                attached.seat_at = epoch_millis();
            }
            let summary = crate::agent::call_summary(&call);
            record(
                &self.state,
                &self.events,
                &session_key,
                json!({"type": "seat", "tool": name, "summary": summary,
                       "label": summary, "allowlistable": false}),
            );
            return Ok(json!({
                "type": "replay",
                "seat": true,
                "name": name,
                "logged": false,
            }));
        }
        let outcome = tool
            .execute(&parsed)
            .map_err(|error| format!("the re-run was refused: {error}"))?;
        Ok(json!({
            "type": "replay",
            "name": name,
            "output": outcome.output,
            "truncated": outcome.truncated,
            "logged": false,
        }))
    }

    /// What the model saw (D10): one session's log reconstructed into
    /// the request its run sent — the system prompt, the message list,
    /// the tool declarations — by the agent's own composer, so there is
    /// no second implementation to drift. The surface supplies the
    /// facts the log cannot hold: the identity, the gui line, and the
    /// file access the run carried.
    pub async fn context(&self, session: &str, base_url: &str) -> Value {
        let home = self.home.clone();
        let session = session.to_string();
        // The run path composes its line from the serving base URL; the
        // reconstructor wants the same words the run carried.
        let gui_prompt = crate::gui::prompt_line(base_url);
        let gui_context = tokio::task::spawn_blocking(move || {
            let path = crate::home::sessions_dir(&home).join(format!("{session}.jsonl"));
            let records = crate::session::replay(&path)
                .map_err(|error| format!("the log could not be read: {error}"))?;
            let identity = crate::home::read_soul(&home);
            Ok::<_, String>(crate::agent::Agent::reconstruct_document(
                &records,
                identity.as_deref(),
                Some(gui_prompt.as_str()),
                &crate::agent::FileAccess::Unrestricted,
            ))
        })
        .await;
        match gui_context {
            Ok(Ok(document)) => document,
            Ok(Err(error)) => json!({ "type": "context", "refused": [error] }),
            Err(_) => json!({ "type": "context", "refused": ["the context read did not land"] }),
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
        // The two registry ops that need the runtime pre-empt the
        // blocking writer: the scan reads the machine's foreign configs
        // and answers with its document, and the probe spawns a child
        // and awaits its handshake. Everything else is a file write and
        // runs off the async worker, the way every other op does.
        if op == "server-scan" {
            let scan = scan_document(&home);
            return Ok(json!({
                "type": "settings_write",
                "op": op,
                "scan": scan,
            }));
        }
        if op == "server-probe" {
            let name = arguments
                .get("name")
                .and_then(Value::as_str)
                .ok_or_else(|| "the probe names its server".to_string())?
                .to_string();
            let dotenv: std::collections::BTreeMap<String, String> =
                crate::home::read_dotenv_with_process_vault(&home)
                    .into_iter()
                    .collect();
            let probe = crate::mcp_registry::probe_server(&home, &name, &dotenv).await?;
            crate::settings::record_server_probe(&home, &name, probe.clone())?;
            return Ok(json!({
                "type": "settings_write",
                "op": op,
                "probe": {
                    "name": name,
                    "ok": probe.ok,
                    "server_name": probe.server_name,
                    "server_version": probe.server_version,
                    "tools": probe.tools,
                    "reason": probe.reason,
                    "at": probe.at,
                },
            }));
        }
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

    /// The second-brain tool plugin: the tool's home and address, whether
    /// its own server is answering, and the pipeline state it reports.
    /// Separate from [`Bridge::brain`] because this one crosses the
    /// process boundary: a tool that is down costs a two-second probe,
    /// and the state read should not wait behind it.
    pub async fn second_brain_plugin(&self) -> Value {
        let home = self.home.clone();
        match tokio::task::spawn_blocking(move || second_brain_plugin_read(&home)).await {
            Ok(document) => document,
            Err(_) => json!({
                "type": "second_brain_plugin",
                "refused": ["the second-brain plugin read did not land"],
            }),
        }
    }

    /// The plugin's one write: a refresh of one of the tool's own
    /// pipeline jobs. The job name is the tool's vocabulary, validated by
    /// the adapter before the call and again by the tool itself.
    pub async fn second_brain_act(&self, body: &str) -> Result<Value, String> {
        let arguments: serde_json::Value = serde_json::from_str(body)
            .map_err(|error| format!("the action is not a JSON object: {error}"))?;
        let action = arguments
            .get("action")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_string();
        if action != "refresh" {
            return Err("the second-brain plugin's one action is `refresh`".to_string());
        }
        let job = arguments
            .get("job")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_string();
        let home = self.home.clone();
        let asked = job.clone();
        let reply = tokio::task::spawn_blocking(move || second_brain_refresh(&home, &asked))
            .await
            .map_err(|_| "the second-brain action did not land".to_string())??;
        Ok(json!({
            "type": "second_brain_action",
            "action": "refresh",
            "job": job,
            "reply": reply,
        }))
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
    /// Retire one skill from every run's prompt, or return it. The
    /// write is fleet-local — `<home>/skills-retired.json` through the
    /// one writer — and the library is never touched: the owner
    /// retires, the skill stays in the library one unretire from
    /// returning (KI-215).
    pub async fn skill_retire(&self, name: &str, retire: bool) -> Result<Value, String> {
        let name = name.trim();
        if name.is_empty() {
            return Err("a retire needs a skill name".to_string());
        }
        let home = self.home.clone();
        let asked = name.to_string();
        tokio::task::spawn_blocking(move || {
            let outcome = if retire {
                crate::settings::retire_skill(&home, &asked)
            } else {
                crate::settings::unretire_skill(&home, &asked)
            };
            outcome.map(|()| {
                json!({
                    "type": "skill_retired",
                    "name": asked,
                    "retired": retire,
                })
            })
        })
        .await
        .unwrap_or_else(|_| Err("the retire did not land".to_string()))
    }

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
            rank_with(&*embedder, &asked, &candidates)
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

    /// The folder picker's listing: the directories under one absolute
    /// path, anywhere the process can read. Not the pane's tree — that
    /// one serves content and is contained to the launch directory by
    /// design; this one serves names to a chooser whose choice
    /// `resolve_folder` validates on its own.
    pub async fn tree_abs(&self, asked: &str) -> Value {
        let asked = asked.to_string();
        match tokio::task::spawn_blocking(move || tree_abs_read(&asked)).await {
            Ok(document) => document,
            Err(_) => json!({"type": "tree", "abs": true, "refused": ["the listing did not land"]}),
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
        let folder = self.run_folder(session, None)?;
        self.full_access_floor(folder.as_deref(), lock(&self.state).permissions)?;
        self.dispatch(session, task, |session, task| {
            Command::Plan(session.to_string(), task.to_string(), folder)
        })
    }

    /// Hand the browser's task to the agent thread. The owner's message
    /// lands in the log first, so a refresh replays what was asked and
    /// the header's running flag moves on the same event.
    pub fn run(&self, session: &str, task: &str) -> Result<(), String> {
        let folder = self.run_folder(session, None)?;
        self.full_access_floor(folder.as_deref(), lock(&self.state).permissions)?;
        self.dispatch(session, task, |session, task| {
            Command::Run(session.to_string(), task.to_string(), folder)
        })
    }

    /// The same run, with the face's standing choices: how this run's
    /// tool calls are approved, the provider its route is pinned to
    /// (`None` is `auto`), and this chat's working folder (`None` uses
    /// the stored choice). The mode and the pin are read by the agent
    /// thread when it prepares the run; the folder is resolved here,
    /// before the command is sent, so the run's root cannot move under
    /// it. Both apply to this run and not to the face's other sessions,
    /// and a full-access run without a folder is a 400: the mode is a
    /// grant only a working folder can bound.
    pub fn run_with(
        &self,
        session: &str,
        task: &str,
        mode: crate::approvals::Mode,
        pin: Option<String>,
        folder: Option<&str>,
    ) -> Result<(), RunError> {
        let folder = self
            .run_folder(session, folder)
            .map_err(RunError::BadRequest)?;
        self.full_access_floor(folder.as_deref(), mode)
            .map_err(RunError::BadRequest)?;
        {
            let mut state = lock(&self.state);
            state.permissions = mode;
            state.provider_pin = pin;
        }
        self.dispatch(session, task, |session, task| {
            Command::Run(session.to_string(), task.to_string(), folder)
        })
        .map_err(RunError::Refused)
    }

    /// The same run, with a per-run turn budget named by the composer.
    /// `None` is the home's config value; a named budget is clamped to
    /// the compiled default exactly the way the config value is — a
    /// request can tighten the cap, never raise it.
    pub fn run_with_budget(
        &self,
        session: &str,
        task: &str,
        mode: crate::approvals::Mode,
        pin: Option<String>,
        folder: Option<&str>,
        budget: Option<usize>,
    ) -> Result<(), RunError> {
        if let Some(turns) = budget {
            lock(&self.state).turn_budget = turns.min(crate::agent::DEFAULT_MAX_TURNS);
        }
        self.run_with(session, task, mode, pin, folder)
    }

    /// Full access is never permitted on a chat with no working folder:
    /// the mode is "nothing asks", and a chat with no folder has no
    /// filesystem access to bound. A run that somehow reaches here with
    /// both — a stored mode from before the folder was cleared, a
    /// remote caller that named no folder — is refused by name rather
    /// than quietly downgraded.
    fn full_access_floor(
        &self,
        folder: Option<&std::path::Path>,
        mode: crate::approvals::Mode,
    ) -> Result<(), String> {
        if mode == crate::approvals::Mode::Yes && folder.is_none() {
            return Err(
                "full access needs a working folder: choose one in the run bar, or use \
                 autonomous or ask"
                    .to_string(),
            );
        }
        Ok(())
    }

    /// Resolve one folder the browser named: the pane's containment
    /// rule for a relative path, and any existing directory for an
    /// absolute one, canonicalized. Public because the server refuses a
    /// bad folder with a 400 naming the reason before a run is
    /// dispatched.
    pub fn chat_folder(&self, asked: &str) -> Result<PathBuf, String> {
        resolve_folder(&self.cwd, asked)
    }

    /// Set or clear this chat's working folder from the run bar. The
    /// choice is stored for the chat, persisted in the sidecar, and
    /// recorded as one line in its transcript. A run in flight is
    /// untouched: the folder it accepted rides its own command, so a
    /// change applies to the next run.
    pub fn set_folder(&self, session: &str, asked: Option<&str>) -> Result<Value, String> {
        let folder = self.store_folder(session, asked)?;
        Ok(self.folder_event(session, folder.as_deref(), None))
    }

    /// The folder this run will use. An explicit choice is validated
    /// and stored, then used. With none, the stored choice is used when
    /// it still resolves; a stored folder that has gone missing is
    /// cleared, falls back to no folder, and names the fallback as the
    /// run's first event. A run with no folder has no filesystem
    /// access.
    fn run_folder(&self, session: &str, asked: Option<&str>) -> Result<Option<PathBuf>, String> {
        if asked.is_some() {
            let folder = self.store_folder(session, asked)?;
            self.folder_event(session, folder.as_deref(), None);
            return Ok(folder);
        }
        let Some(stored) = lock(&self.state).workdir_of(session) else {
            return Ok(None);
        };
        match resolve_folder(&self.cwd, &stored) {
            Ok(path) => Ok(Some(path)),
            Err(reason) => {
                {
                    let mut state = lock(&self.state);
                    state.set_workdir(session, None);
                }
                self.persist_workdirs();
                self.folder_event(session, None, Some((&stored, &reason)));
                Ok(None)
            }
        }
    }

    /// Store one chosen folder (or clear it) and persist the sidecar.
    /// `None` is no folder — and therefore no filesystem access. A
    /// folder is a grant, so choosing the launch directory grants it
    /// exactly like any other: the owner's explicit choice is the only
    /// thing that opens access.
    /// Ask the model a question *beside* the run: one tool-less call
    /// through a route prepared for the aside itself, so the answer
    /// costs one model call and never enters the run's conversation.
    /// The route is prepared with a fixed task text, so the
    /// route-decision cache serves every aside after the first.
    pub async fn btw(&self, session: &str, text: &str) -> Result<Value, String> {
        let text = text.trim().to_string();
        if text.is_empty() {
            return Err("a side ask needs a question".to_string());
        }
        let home = self.home.clone();
        let pin = lock(&self.state).provider_pin.clone();
        let session = session.to_string();
        // The router's streamed walk holds a `&mut dyn FnMut` across an
        // await, so its future is not `Send` — the agent thread's own
        // shape, applied here: a blocking task with a runtime of its own.
        tokio::task::spawn_blocking(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|error| format!("the side ask needs a runtime: {error}"))?;
            runtime.block_on(async move {
                let dotenv = crate::home::read_dotenv(&home);
                // The route is prepared for the aside itself, with a
                // fixed task text, so the decision cache serves every ask
                // after the first.
                let prepared = crate::route::prepare_with_pin(
                    &home,
                    &dotenv,
                    "a side question about the work in progress",
                    &std::collections::HashMap::new(),
                    pin.as_deref(),
                )
                .await
                .map_err(|error| error.to_string())?;
                let client = Client::routed(prepared.router.clone());
                let messages = vec![
                    crate::api::Message::text(
                        crate::api::Role::System,
                        "The owner asked a side question while a run is executing. Answer it briefly and directly — this reply is not part of the run's conversation and cannot call tools.",
                    ),
                    crate::api::Message::text(crate::api::Role::User, text.clone()),
                ];
                let mut streamed = String::new();
                let answered = client
                    .chat_stream(
                        &crate::api::ChatRequest {
                            model: "",
                            messages: &messages,
                            tools: Vec::new(),
                        },
                        |event| {
                            if let crate::api::StreamEvent::Content(chunk) = event {
                                streamed.push_str(&chunk);
                            }
                        },
                    )
                    .await
                    .map_err(|error| format!("the side ask failed: {error}"))?;
                let answer = if !streamed.trim().is_empty() {
                    streamed
                } else {
                    answered
                        .choices
                        .first()
                        .and_then(|choice| choice.message.content.clone())
                        .unwrap_or_else(|| "the model answered with no text".to_string())
                };
                Ok(json!({
                    "type": "btw",
                    "session": session,
                    "question": text,
                    "answer": answer,
                    "model": prepared.decision.model,
                    "provider": prepared.decision.provider,
                }))
            })
        })
        .await
        .map_err(|_| "the side ask did not land".to_string())?
    }

    /// Name a session: the owner's title for an archive whose rows
    /// otherwise fall back to the first message's line.
    pub fn set_title(&self, session: &str, asked: &str) -> Result<Value, String> {
        let session = if session.is_empty() {
            self.launch()
        } else {
            session.to_string()
        };
        if session.is_empty() {
            return Err("there is no session to name yet".to_string());
        }
        let title = asked.trim();
        let mut titles = load_titles(&self.home);
        if title.is_empty() {
            titles.remove(&session);
        } else {
            titles.insert(session.clone(), title.to_string());
        }
        save_titles(&self.home, &titles);
        Ok(json!({
            "type": "title",
            "session": session,
            "title": title,
        }))
    }

    /// One control into the run in flight, its body written verbatim:
    /// a fold, a pin, a pause, a resume, or a steer. The guards are the
    /// stop's own — the run must be in flight, and the driving session
    /// is the one controlled.
    pub fn control(&self, body: &serde_json::Value) -> Result<Value, String> {
        let session = {
            let state = lock(&self.state);
            let driving = state.target();
            let running = state
                .attached_if(&driving)
                .map(|attached| attached.running)
                .unwrap_or(false);
            if !running {
                return Err("no run is in flight to control".to_string());
            }
            driving
        };
        crate::delegate::write_control_value(&self.home, &session, body)?;
        let text = body
            .as_object()
            .and_then(|object| object.keys().next().cloned())
            .unwrap_or_default();
        record(
            &self.state,
            &self.events,
            &session,
            json!({"type": "control", "control": text, "session": session, "body": body}),
        );
        Ok(json!({"type": "control", "session": session, "control": text}))
    }

    /// Set or clear this session's standing objective, and show the
    /// change where the owner is: one transcript line, carrying what
    /// every following run will be told.
    pub fn set_goal(&self, session: &str, asked: Option<&str>) -> Result<Value, String> {
        // A goal set before the first run rides the pending key, the
        // workdirs' own shape: the thread names the launch session on the
        // first task and the goal moves with it.
        let session = if session.is_empty() {
            self.launch()
        } else {
            session.to_string()
        };
        let text = asked
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .map(str::to_string);
        let mut goals = load_goals(&self.home);
        match &text {
            Some(text) => {
                goals.insert(session.clone(), text.clone());
            }
            None => {
                goals.remove(&session);
            }
        }
        save_goals(&self.home, &goals);
        record(
            &self.state,
            &self.events,
            &session,
            json!({"type": "goal", "text": text, "session": session}),
        );
        Ok(json!({
            "type": "goal",
            "session": session,
            "text": text,
        }))
    }

    fn store_folder(&self, session: &str, asked: Option<&str>) -> Result<Option<PathBuf>, String> {
        let asked = asked.map(str::trim).filter(|asked| !asked.is_empty());
        let folder = match asked {
            Some(asked) => Some(self.chat_folder(asked)?),
            None => None,
        };
        let stored = folder.as_ref().map(|path| path.display().to_string());
        {
            let mut state = lock(&self.state);
            state.set_workdir(session, stored.as_deref());
        }
        self.persist_workdirs();
        Ok(folder)
    }

    fn persist_workdirs(&self) {
        let map = lock(&self.state).workdirs.clone();
        save_workdirs(&self.home, &map);
    }

    /// One transcript line for a folder that was chosen, cleared, or
    /// fallen back from. `fallback` is the stored path and the reason
    /// it could not be used.
    fn folder_event(
        &self,
        session: &str,
        folder: Option<&std::path::Path>,
        fallback: Option<(&str, &str)>,
    ) -> Value {
        let workspace = self.cwd.display().to_string();
        let event = match (folder, fallback) {
            (Some(path), _) => json!({
                "type": "folder",
                "path": path.display().to_string(),
                "name": folder_name(path),
                "rel": rel_to_root(&self.cwd, path),
                "workspace": workspace,
                "note": format!("this chat works in {}", folder_name(path)),
            }),
            (None, Some((missing, reason))) => json!({
                "type": "folder",
                "path": null,
                "name": null,
                "fallback": true,
                "missing": missing,
                "workspace": workspace,
                "note": format!(
                    "the folder {missing} is no longer usable ({reason}); this chat now has no working folder, so the filesystem tools are unavailable until one is set"
                ),
            }),
            (None, None) => json!({
                "type": "folder",
                "path": null,
                "name": null,
                "workspace": workspace,
                "note": "this chat has no working folder: the filesystem tools are unavailable until one is set",
            }),
        };
        record(&self.state, &self.events, session, event.clone());
        event
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
            // The run in flight is the driving session — the one whose
            // events are crossing the wire, which is exact because the
            // thread's command loop is serial — and that session's own
            // `running` is the flag that says so. A face with two
            // sessions open stops the run, not the one it is looking at.
            let driving = state.target();
            let running = state
                .attached_if(&driving)
                .map(|attached| attached.running)
                .unwrap_or(false);
            if !running {
                return Err("no run is in flight to stop or steer".to_string());
            }
            if driving.is_empty() {
                return Err(
                    "this run has no session log yet; there is nothing beside it to write"
                        .to_string(),
                );
            }
            driving
        };
        crate::delegate::write_control(&self.home, &session, steer)?;
        let kind = if steer.is_some() { "steer" } else { "stop" };
        let value = json!({
            "type": "control",
            "kind": kind,
            "session": session,
            "steer": steer,
        });
        record(&self.state, &self.events, &session, value.clone());
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
            attached.seat_at = 0;
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

    /// Answer the seat in this session. First answer wins — the seat is
    /// taken out of the session's state before anything else happens, so
    /// a second answer finds no seat and is told so — and the answer is
    /// broadcast with who gave it, so the surfaces that lost the race
    /// render the resolution instead of a stale button.
    ///
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
    pub fn answer(
        &self,
        session: &str,
        decision: &str,
        by: &AnsweredBy,
    ) -> Result<Value, AnswerError> {
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
        //
        // Which session's seat: the one asked for, and — for the
        // session-less shape a page that predates the seat's own name
        // posts — the session being driven. Seats are stored under the
        // driving session (`target()`), which is the face's launch
        // session until a channel peer's run takes over; answering a
        // session-less post against the launch session alone refused
        // with `NoSeat` while the peer's seat stood and its run hung.
        let (seat, answered_session) = {
            let mut state = lock(&self.state);
            let asked = if session.is_empty()
                && state
                    .attached_if(&state.target())
                    .is_some_and(|attached| attached.seat.is_some())
            {
                state.target()
            } else {
                session.to_string()
            };
            let attached = state.attached(&asked);
            let seat = attached.seat.take().ok_or(AnswerError::NoSeat)?;
            attached.seat_at = 0;
            (seat, asked)
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
        // The resolution rides the same stream the seat did, naming the
        // session, the decision, and who answered: every surface watching
        // that session sees the seat close, rather than leaving a button
        // standing for a seat that is gone.
        let _ = self.events.send(json!({
            "type": "answered",
            "session": answered_session,
            "decision": decision,
            "by": by.label(),
            // The seat's own words ride the broadcast, so every surface
            // can record what was allowed — and a replay rebuilds the
            // same line from the event alone.
            "tool": seat.call.function.name,
            "summary": seat.summary,
        }));
        Ok(json!({
            "answered": decision,
            "rule": rule,
            "error": error,
        }))
    }
}

/// Who answered a seat, so the broadcast that closes it can say where
/// the answer came from. A seat may be answered from the desktop's
/// browser, a paired device, or a channel peer; the rule is first answer
/// wins, and the surfaces that lost the race deserve to know who won
/// rather than to wonder whether their click landed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnsweredBy {
    /// The desktop's own browser.
    Desktop,
    /// A paired device, by the name its owner gave it.
    Device(String),
    /// A channel peer, by the name it paired under.
    Peer(String),
}

impl AnsweredBy {
    /// How the broadcast names it.
    fn label(&self) -> String {
        match self {
            AnsweredBy::Desktop => "this browser".to_string(),
            AnsweredBy::Device(name) => format!("the device {name}"),
            AnsweredBy::Peer(name) => format!("the channel peer {name}"),
        }
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

/// Why a run did not start, with the answer the browser gives: a
/// request that breaks one of the face's rules is a 400, and a face
/// that cannot take the run itself is a 409. The distinction matters to
/// the page — a 400 is the owner's choice to correct, a 409 is the
/// face's state to wait on.
#[derive(Debug)]
pub enum RunError {
    /// The request breaks a rule: a folder outside the launch
    /// directory, or full access with no folder.
    BadRequest(String),
    /// The face cannot take the run: one is in flight, or this face has
    /// no agent behind it.
    Refused(String),
}

impl std::fmt::Display for RunError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RunError::BadRequest(message) | RunError::Refused(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for RunError {}

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
    let passphrase = crate::home::vault_passphrase();
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
    // The workspace join: the day's delegation rows name the branch
    // each child's work stands on, so a row in this panel carries the
    // click-through into the workspace-history section. The map is
    // read once and walked down the trees — one source per picture.
    let branches = crate::layerfs_view::session_branches();
    let node = |node: &crate::boards::DelegationNode| delegation_node(node, &branches);
    json!({
        "type": "delegation",
        "read_at": snapshot.read_at,
        "depth_cap": snapshot.depth_cap,
        "live": snapshot.live.as_ref().map(node),
        "history": snapshot.history.iter().map(node).collect::<Vec<_>>(),
        "graph": snapshot.graph.as_ref().map(graph_document),
        "refused": snapshot.refused,
    })
}

/// The memory panel's document, hand-built from the read's snapshot:
/// memory.rs carries no serde derives, so the shapes are translated here
/// rather than derived twice.
fn memory_read(home: &std::path::Path, memories: &std::path::Path, cwd: &std::path::Path) -> Value {
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
            // The pane is contained to the launch directory, so a fact
            // inside it is asked for relatively and one outside it is
            // named rather than handed a path the reader would refuse.
            "relative": std::path::Path::new(&fact.path)
                .strip_prefix(cwd)
                .ok()
                .and_then(|path| path.to_str())
                .map(str::to_string),
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

/// Resolve one chat folder. A relative path is the pane's own rooted
/// form and uses the pane's rule, which catches `..` and a symlink out.
/// An absolute path may name any existing directory on the disk (the
/// owner's 2026-10-21 ruling: the picker browses the whole hierarchy and
/// the choice may land anywhere in it) — the tools' own cage still holds
/// inside whatever folder is chosen. A path that does not exist, or is
/// not a directory, is refused by name. The reply is canonical, so the
/// sidecar stores one spelling of a folder and two chats that name the
/// same tree agree about it.
fn resolve_folder(root: &std::path::Path, asked: &str) -> Result<PathBuf, String> {
    let asked = asked.trim();
    let canonical_root = root
        .canonicalize()
        .map_err(|error| format!("{}: {error}", root.display()))?;
    if asked.is_empty() {
        return Ok(canonical_root);
    }
    let given = std::path::Path::new(asked);
    if !given.is_absolute() {
        return pane_path(root, asked);
    }
    let canonical = given
        .canonicalize()
        .map_err(|error| format!("{asked}: {error}"))?;
    if !canonical.is_dir() {
        return Err(format!("{asked} is not a directory"));
    }
    Ok(canonical)
}

/// A folder as the state document and the transcript carry it: its
/// canonical path, the basename the run bar shows, and its path
/// relative to the launch directory (the form the picker browses and
/// the browser sends back). `None` is no folder and rides as JSON null.
fn folder_value(path: Option<&str>, root: &std::path::Path) -> Value {
    match path {
        Some(path) => {
            let path = std::path::Path::new(path);
            json!({
                "path": path.display().to_string(),
                "name": folder_name(path),
                "rel": rel_to_root(root, path),
            })
        }
        None => Value::Null,
    }
}

/// One folder's path relative to the launch directory, in the pane's
/// rooted form: `""` for the root itself, `docs` for a child. The two
/// sides are canonicalized because a launch directory can have two
/// spellings on macOS (`/var` and `/private/var`), and a prefix check
/// on the wrong one would report every folder as outside.
fn rel_to_root(root: &std::path::Path, path: &std::path::Path) -> String {
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    path.strip_prefix(&root)
        .map(|rel| rel.to_string_lossy().replace('\\', "/"))
        .unwrap_or_default()
}

/// The name a folder is shown by: its basename, or the whole path when
/// it has none (a filesystem root).
fn folder_name(path: &std::path::Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

/// The sidecar each session's owner-set title is stored in: the
/// workdirs' pattern, one file beside the other fleet-owned sidecars.
const TITLES_FILE: &str = "sessions-titles.json";

fn titles_path(home: &std::path::Path) -> PathBuf {
    home.join(TITLES_FILE)
}

fn load_titles(home: &std::path::Path) -> std::collections::HashMap<String, String> {
    let text = match std::fs::read_to_string(titles_path(home)) {
        Ok(text) => text,
        Err(_) => return std::collections::HashMap::new(),
    };
    let stored: std::collections::HashMap<String, String> =
        serde_json::from_str(&text).unwrap_or_default();
    stored
        .into_iter()
        .filter(|(id, _)| session::session_path(home, id).exists())
        .collect()
}

fn save_titles(home: &std::path::Path, titles: &std::collections::HashMap<String, String>) {
    let path = titles_path(home);
    if titles.is_empty() {
        let _ = std::fs::remove_file(&path);
        return;
    }
    let Ok(text) = serde_json::to_string_pretty(titles) else {
        return;
    };
    let temp = path.with_extension(format!("{}.tmp", std::process::id()));
    if std::fs::write(&temp, text).is_err() {
        return;
    }
    let _ = std::fs::rename(&temp, &path);
}

/// The sidecar each session's standing objective is stored in — the
/// workdirs' pattern, one file beside the other fleet-owned sidecars.
const GOALS_FILE: &str = "sessions-goals.json";

fn goals_path(home: &std::path::Path) -> PathBuf {
    home.join(GOALS_FILE)
}

/// Read the per-session goals. An entry whose session log is gone is
/// dropped, like the workdirs map.
fn load_goals(home: &std::path::Path) -> std::collections::HashMap<String, String> {
    let text = match std::fs::read_to_string(goals_path(home)) {
        Ok(text) => text,
        Err(_) => return std::collections::HashMap::new(),
    };
    let stored: std::collections::HashMap<String, String> =
        serde_json::from_str(&text).unwrap_or_default();
    stored
        .into_iter()
        // The pending key never has a log of its own — it IS the goal
        // waiting for the first run to name the session — so it survives
        // the prune every real session id meets.
        .filter(|(id, _)| id.is_empty() || session::session_path(home, id).exists())
        .collect()
}

/// Write the per-session goals, atomically and with a writer-unique
/// temp name, the workdirs' own rules.
fn save_goals(home: &std::path::Path, goals: &std::collections::HashMap<String, String>) {
    let path = goals_path(home);
    if goals.is_empty() {
        let _ = std::fs::remove_file(&path);
        return;
    }
    let Ok(text) = serde_json::to_string_pretty(goals) else {
        return;
    };
    let temp = path.with_extension(format!("{}.tmp", std::process::id()));
    if std::fs::write(&temp, text).is_err() {
        return;
    }
    let _ = std::fs::rename(&temp, &path);
}

/// One session's standing objective, when it has one.
pub fn session_goal(home: &std::path::Path, session: &str) -> Option<String> {
    load_goals(home).remove(session)
}

/// The thread named the launch session: a goal set before the first run
/// moves from the pending key to the real one, exactly as the folder
/// does.
fn move_pending_goal(home: &std::path::Path, id: &str) {
    let mut goals = load_goals(home);
    let Some(goal) = goals.remove("") else {
        return;
    };
    goals.insert(id.to_string(), goal);
    save_goals(home, &goals);
}

/// The sidecar each chat's folder is stored in. It is a surface
/// setting, so it lives beside the other fleet-owned sidecars rather
/// than in the session log's format, which is the conversation.
const WORKDIRS_FILE: &str = "sessions-workdirs.json";

fn workdirs_path(home: &std::path::Path) -> PathBuf {
    home.join(WORKDIRS_FILE)
}

/// Read the per-chat folders. An entry whose session log is gone is
/// dropped: a folder nothing will ever run in is noise, and the next
/// save writes the pruned map.
fn load_workdirs(home: &std::path::Path) -> std::collections::HashMap<String, String> {
    let text = match std::fs::read_to_string(workdirs_path(home)) {
        Ok(text) => text,
        Err(_) => return std::collections::HashMap::new(),
    };
    let stored: std::collections::HashMap<String, String> =
        serde_json::from_str(&text).unwrap_or_default();
    stored
        .into_iter()
        .filter(|(id, _)| session::session_path(home, id).exists())
        .collect()
}

/// Write the per-chat folders, atomically and with a writer-unique
/// temp name. Best-effort: a sidecar that cannot be written leaves the
/// in-memory choice working for this process, and the failure is not
/// worth failing a run over — the transcript line already told the
/// owner what the chat is doing.
fn save_workdirs(home: &std::path::Path, workdirs: &std::collections::HashMap<String, String>) {
    let path = workdirs_path(home);
    if workdirs.is_empty() {
        let _ = std::fs::remove_file(&path);
        return;
    }
    let Ok(text) = serde_json::to_string_pretty(workdirs) else {
        return;
    };
    let temp = path.with_extension(format!("{}.tmp", std::process::id()));
    if std::fs::write(&temp, text).is_err() {
        return;
    }
    let _ = std::fs::rename(&temp, &path);
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

/// The picker's listing under one absolute path: directories only —
/// dot-directories included, since they are real folders an owner may
/// mean — files filtered out, sorted without regard to case, capped like
/// the pane's tree. An empty path is the filesystem root. A relative
/// path, a non-directory and an unreadable directory are each refused by
/// name; nothing here is contained, because the choice this feeds is
/// validated by [`resolve_folder`] when it is made.
fn tree_abs_read(asked: &str) -> Value {
    let asked = asked.trim();
    let directory = if asked.is_empty() {
        PathBuf::from("/")
    } else {
        PathBuf::from(asked)
    };
    if !directory.is_absolute() {
        return json!({"type": "tree", "abs": true, "refused": [format!("{asked} is not an absolute path")]});
    }
    if !directory.is_dir() {
        return json!({"type": "tree", "abs": true, "refused": [format!("{} is not a directory", directory.display())]});
    }
    let read = match std::fs::read_dir(&directory) {
        Ok(read) => read,
        Err(error) => {
            return json!({"type": "tree", "abs": true, "refused": [format!("{} could not be read: {error}", directory.display())]});
        }
    };
    let mut names: Vec<String> = read
        .flatten()
        .filter(|entry| entry.path().is_dir())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort_by_key(|name| name.to_lowercase());
    let total = names.len();
    names.truncate(TREE_ENTRY_CAP);
    let shown = directory.canonicalize().unwrap_or(directory);
    json!({
        "type": "tree",
        "abs": true,
        "path": shown.display().to_string(),
        "truncated": total > TREE_ENTRY_CAP,
        "entries": names.iter().map(|name| json!({"name": name, "kind": "dir"})).collect::<Vec<_>>(),
    })
}

/// What one attach decision is: a local read, a question about the hosted
/// parser, or the escalation the caller may take after consent.
#[derive(Debug)]
enum Attachment {
    Ready(Value),
    Needs(Value),
    Escalate {
        path: PathBuf,
        name: String,
        reason: String,
    },
}

/// One attachment, from the workspace path the browser asked for.
///
/// Local extraction runs first for every format it owns. A PDF with no
/// text layer, and every format local extraction does not read, is the
/// hosted parser's case; the first call without `consent` answers with
/// what would be sent and why, and only a second call with consent hands
/// the path to sys1 — the lane that holds the credential.
fn attach_local(cwd: &std::path::Path, asked: &str, consent: bool) -> Result<Attachment, String> {
    let full = pane_path(cwd, asked).map_err(|reason| format!("the path was refused: {reason}"))?;
    let metadata = std::fs::metadata(&full)
        .map_err(|error| format!("{} could not be read: {error}", full.display()))?;
    if !metadata.is_file() {
        return Err("the path is not a file".to_string());
    }
    if metadata.len() > crate::documents::MAX_BYTES {
        return Err(format!(
            "the file is larger than the {} MB attachment bound",
            crate::documents::MAX_BYTES / (1024 * 1024)
        ));
    }
    let name = full
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("document")
        .to_string();
    let route = crate::documents::route(&full);
    let escalation = match route {
        crate::documents::Route::Local | crate::documents::Route::Pdf => {
            let services = crate::lanes::services_root()
                .map_err(|_| "the documents lane is not installed".to_string())?;
            match crate::documents::extract(&full, &services) {
                crate::documents::Outcome::Extracted(text) => {
                    return Ok(Attachment::Ready(json!({
                        "type": "attachment",
                        "name": name,
                        "text": text.markdown,
                        "parser": text.parser,
                        "pages": text.pages,
                        "units": null,
                        "job_id": null,
                        "truncated": null,
                    })));
                }
                crate::documents::Outcome::Escalate { reason } => reason,
                crate::documents::Outcome::Refused { code, reason } => {
                    return Err(format!(
                        "the local extractor refused the document ({code}): {reason}"
                    ));
                }
            }
        }
        crate::documents::Route::Escalate => {
            "local extraction does not read this format; Perception parses it".to_string()
        }
        crate::documents::Route::Unsupported => {
            let extension = full
                .extension()
                .and_then(|value| value.to_str())
                .unwrap_or("?");
            return Err(format!("no parser for .{extension}"));
        }
    };
    if !consent {
        return Ok(Attachment::Needs(json!({
            "type": "attach_needs",
            "name": name,
            "parser": "nace",
            "reason": escalation,
        })));
    }
    Ok(Attachment::Escalate {
        path: full,
        name,
        reason: escalation,
    })
}

/// One file's text, its language, and whether the read was cut. A file
/// that is not UTF-8 is refused rather than mangled — the viewer shows
/// text, and a binary shown as text is a lie about what is there.
fn file_read(root: &std::path::Path, asked: &str) -> Value {
    // The credential check is lexical and first, so a dotenv is refused
    // by name whether or not it exists — a browser that can ask for a
    // file is exactly where a credential would leak, and the refusal must
    // not depend on the file being there to refuse. The prefix covers
    // every dotenv spelling, `.envrc` included (KI-238).
    let name = asked.rsplit('/').next().unwrap_or(asked);
    if name.starts_with(".env") {
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
pub fn session_id_is_shape(id: &str) -> bool {
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
    // The resolutions ride beside the results they explain. A replay
    // that dropped them would show a call with no account of how it was
    // chosen.
    let resolutions: std::collections::HashMap<String, String> = records
        .iter()
        .filter_map(|record| match record {
            session::Record::ToolResolution {
                call_id,
                method,
                reason,
                ..
            } => Some((call_id.clone(), format!("{method} \u{b7} {reason}"))),
            _ => None,
        })
        .collect();
    for record in records {
        match record {
            session::Record::Start { .. }
            | session::Record::End { .. }
            | session::Record::ToolResolution { .. }
            | session::Record::LaneFailed { .. }
            | session::Record::Declarations { .. }
            | session::Record::Routed { .. } => {}
            // Bookkeeping, like start and end — it replays to no message
            // — but it carries the facts the inbox shows, so the replay
            // surfaces it rather than dropping it.
            session::Record::Channel {
                channel,
                peer_id,
                peer,
            } => {
                cards.push(json!({
                    "type": "channel",
                    "channel": channel,
                    "peer_id": peer_id,
                    "peer": peer,
                }));
            }
            // The model's thinking, as one block per turn: the
            // transcript shows it closed and opens it on a press, and a
            // replay rebuilds the same row from this record.
            session::Record::Reasoning { text, seconds } => {
                cards.push(json!({
                    "type": "reasoning",
                    "text": text,
                    "seconds": seconds,
                }));
            }
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
            session::Record::Assistant {
                content,
                tool_calls,
            } => {
                if let Some(text) = content {
                    cards.push(json!({"type": "message", "role": "assistant", "text": text}));
                }
                // The calls the answer requested, before their results:
                // the replay shows the same pair the live stream does, and
                // the call carries its arguments so an expanded row names
                // what actually ran (the result alone cannot).
                for call in tool_calls {
                    cards.push(json!({
                        "type": "tool_call",
                        "tool": call.function.name,
                        "summary": crate::agent::call_summary(call),
                        "arguments": call.function.arguments,
                    }));
                }
            }
            session::Record::ToolResult {
                call_id,
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
                    "resolution": resolutions.get(call_id),
                }));
            }
            session::Record::LaneTools { lane, tools } => {
                cards.push(json!({"type": "lane", "lane": lane, "tools": tools.len()}));
            }
            session::Record::Goal { text } => {
                // The standing objective rides the prompt, so the replay
                // shows the line the run carried.
                cards.push(json!({"type": "goal", "text": text}));
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
/// The access document: the paired devices and the channel peers, with
/// names and stamps and never a token.
/// `task` or `observe`, as a surface's body spells it. A body that names
/// neither is refused by name rather than defaulting: a capability is a
/// grant, and a grant nobody typed is the wrong default.
fn parse_capability(word: &str) -> Result<crate::channels::Capability, String> {
    match word.trim() {
        "task" | "act" => Ok(crate::channels::Capability::Task),
        "observe" | "listen" => Ok(crate::channels::Capability::Observe),
        other => Err(format!(
            "`{other}` is not a capability; `task` acts, `observe` listens"
        )),
    }
}

/// The channel control plane as one document: the service's state, every
/// channel's paired and pending peers, and the newest activity.
///
/// The bound is named, not implied: a surface that shows fifty lines and
/// says "the newest 50" is telling the truth, and one that shows fifty
/// of three hundred silently is not.
fn channels_read(home: &std::path::Path) -> Value {
    const ACTIVITY_SHOWN: usize = 50;
    let service = crate::channels::service::document(home);
    let mut channels = Vec::new();
    for name in crate::channels::CHANNELS {
        let peers = crate::channels::PeerStore::load(home, name)
            .map(|store| {
                store
                    .peers
                    .iter()
                    .map(|peer| {
                        json!({
                            "channel": name,
                            "id": peer.id,
                            "name": peer.name,
                            "capability": match peer.capability {
                                crate::channels::Capability::Task => "task",
                                crate::channels::Capability::Observe => "observe",
                            },
                            "paired": peer.created_at,
                            // A URL, not a secret: the surface says where
                            // an outcome goes.
                            "callback": peer.callback,
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let pending = crate::channels::PendingStore::load(home, name)
            .peers
            .iter()
            .map(|peer| {
                json!({
                    "channel": name,
                    "id": peer.id,
                    "name": peer.name,
                    "attempts": peer.attempts,
                    "first_seen": peer.first_seen,
                    "last_seen": peer.last_seen,
                })
            })
            .collect::<Vec<_>>();
        channels.push(json!({
            "name": name,
            "peers": peers,
            "pending": pending,
        }));
    }
    let activity = crate::channels::service::activity(home, ACTIVITY_SHOWN);
    // The Telegram credential, by presence and source only. The value is
    // never part of this document.
    let (telegram_present, telegram_source) =
        crate::settings::secret_state(home, "FLEET_TELEGRAM_BOT_TOKEN");
    json!({
        "type": "channels",
        "read_at": crate::boards::now_stamp(),
        "service": service,
        "channels": channels,
        "activity": activity,
        "activity_bound": ACTIVITY_SHOWN,
        "telegram": {
            "present": telegram_present,
            "source": telegram_source,
        },
    })
}

fn access_read(home: &std::path::Path) -> Value {
    let devices = crate::remote::pairing::DeviceStore::load(home)
        .map(|store| {
            store
                .devices
                .iter()
                .map(|device| {
                    json!({
                        "id": device.id,
                        "name": device.name,
                        "paired": device.created_at,
                        "seen": device.last_seen_at,
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    // Every channel's store, not one of them: a paired Telegram chat is
    // as much a way into this face as a paired webhook, and a read that
    // walked only the webhook would tell the owner half the truth about
    // who can reach them.
    let peers = crate::channels::CHANNELS
        .iter()
        .flat_map(|channel| {
            crate::channels::PeerStore::load(home, channel)
                .map(|store| {
                    store
                        .peers
                        .iter()
                        .map(|peer| {
                            json!({
                                "channel": channel,
                                "id": peer.id,
                                "name": peer.name,
                                "capability": match peer.capability {
                                    crate::channels::Capability::Task => "may act",
                                    crate::channels::Capability::Observe => "may only listen",
                                },
                                "paired": peer.created_at,
                            })
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default()
        })
        .collect::<Vec<_>>();
    json!({
        "type": "access",
        "read_at": crate::boards::now_stamp(),
        "devices": devices,
        "peers": peers,
    })
}

/// How fresh an unfinished log's last write must be for the archive to
/// call it active rather than interrupted. There is no heartbeat record
/// — a turn writes, a pause does not — so the window is a heuristic: ten
/// minutes covers a model call plus a tool, and a pause shows as
/// `paused` from its control file instead. A surface that says
/// "active" is saying "written recently", not "running".
const LIVE_WINDOW_SECS: u64 = 600;

/// The live facts a row cannot read from its own log: which session this
/// face is driving, and which sessions hold an approval. Gathered from
/// the face's state before the archive walk.
#[derive(Default, Clone)]
struct LiveFace {
    driving: Option<String>,
    seats: std::collections::HashSet<String>,
}

/// A row's work state, derived once here so the page never guesses:
/// what needs the owner first, then what is running, then how it ended.
fn work_state(entry: &session::SessionSummary, live: &LiveFace, paused: bool) -> &'static str {
    if live.seats.contains(&entry.id) {
        return "needs-approval";
    }
    if paused {
        return "paused";
    }
    // The log's own end is authoritative: a run that wrote its end is
    // over, whatever a stale driver flag says for the instant between.
    if entry.ended {
        return match entry.stop_reason.as_deref() {
            Some("max-turns") => "capped",
            Some(reason) if reason.starts_with("stopped") => "stopped",
            _ => "completed",
        };
    }
    if live.driving.as_deref() == Some(entry.id.as_str()) {
        return "working";
    }
    // A log with no owner message is a conversation created and not yet
    // used: New writes the start record, and the next message is what
    // makes it work.
    if entry.first_text.is_none() {
        return "new";
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|age| age.as_secs())
        .unwrap_or_default();
    match entry.last_write {
        Some(write) if now.saturating_sub(write) <= LIVE_WINDOW_SECS => "active",
        _ => "interrupted",
    }
}

/// Whose work a row is: the owner's own conversation, a scheduled job's
/// run, a delegated child, or a peer's channel conversation.
fn work_class(entry: &session::SessionSummary) -> &'static str {
    match entry.kind.as_deref() {
        Some(session::SCHEDULED_KIND) => "background",
        Some(session::SUBAGENT_KIND) => "delegated",
        _ if entry.channel.is_some() => "channel",
        _ => "yours",
    }
}

/// The job a scheduled run belongs to, read from the frame
/// `cron::frame_task` writes into the run's first message. `None` for
/// anything that is not a scheduled run or whose frame cannot be read.
fn scheduled_job(entry: &session::SessionSummary) -> Option<String> {
    if entry.kind.as_deref() != Some(session::SCHEDULED_KIND) {
        return None;
    }
    let text = entry.first_text.as_deref()?;
    let start = text.find("job `")? + "job `".len();
    let rest = &text[start..];
    let end = rest.find('`')?;
    let name = rest[..end].trim();
    (!name.is_empty()).then(|| name.to_string())
}

/// Whether the owner paused this run: the control file the agent reads
/// before every model call, read here without consuming it.
fn paused_by_owner(home: &std::path::Path, id: &str) -> bool {
    let path = crate::home::sessions_dir(home).join(format!("{id}.control"));
    let Ok(text) = std::fs::read_to_string(&path) else {
        return false;
    };
    serde_json::from_str::<Value>(&text)
        .ok()
        .and_then(|value| value.get("pause").and_then(Value::as_bool))
        == Some(true)
}

/// The literal content match pass (D9): the newest-first logs' owner and
/// assistant text, first match wins the row, the matched line rides it.
/// The bound is named on the document — "no match" always means "no
/// match in the first SEARCH_BOUND logs", never a lie. Literal, not
/// semantic: the 2026-10-20 measurement scored this corpus's cached
/// session vectors 1/81 top-1 with a 0.00 median margin, so a semantic
/// pass would be a classifier nobody measured against ground truth.
pub const SEARCH_BOUND: usize = 200;

fn session_match(home: &std::path::Path, id: &str, needle: &str) -> Option<String> {
    let path = home.join("sessions").join(format!("{id}.jsonl"));
    let text = std::fs::read_to_string(&path).ok()?;
    let needle = needle.to_lowercase();
    for line in text.lines() {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        // The log's record kinds, not the stream's event vocabulary:
        // `user` and `assistant` are the message records.
        match value.get("type").and_then(|kind| kind.as_str()) {
            Some("user") | Some("assistant") => {}
            _ => continue,
        }
        let said = value
            .get("content")
            .and_then(|content| content.as_str())
            .unwrap_or_default();
        if said.to_lowercase().contains(&needle) {
            let clipped: String = said.chars().take(160).collect();
            return Some(clipped);
        }
    }
    None
}

fn sessions_read(home: &std::path::Path, current: &str, live: &LiveFace, query: &str) -> Value {
    let titles = load_titles(home);
    let entries = match session::list(home) {
        Ok(entries) => entries,
        Err(error) => {
            return json!({
                "type": "sessions",
                "refused": [format!("the archive could not be read: {error}")],
            })
        }
    };
    // The content pass (D9): only when the query asks, and only over
    // the newest SEARCH_BOUND logs — the bound rides the document so a
    // miss is honest about what it searched.
    let needle = if query.trim().is_empty() {
        None
    } else {
        Some(query.trim().to_string())
    };
    let entries: Vec<_> = entries.into_iter().take(SEARCH_BOUND).collect();
    let searched = needle.as_ref().map(|_| entries.len());
    let sessions = entries
        .iter()
        .rev()
        .map(|entry| {
            let paused = paused_by_owner(home, &entry.id);
            json!({
                "id": entry.id,
                // The owner's title when one is set, else the session's
                // first message line — the archive reads locally, never a
                // model call.
                "title": titles
                    .get(&entry.id)
                    .cloned()
                    .or_else(|| entry.first_text.clone()),
                // The model that actually served (KI-073's end record),
                // or null while the run is in flight — never the start
                // record's pre-route stamp, which is a default, not a
                // decision. The face renders a dash for it.
                "model": entry.serving_model(),
                "created": entry.created,
                "ended": entry.ended,
                // How it ended, in the log's own words, and the turns it
                // counted: a row can say "capped at 8 turns" instead of
                // leaving the owner to open it and guess.
                "stop_reason": entry.stop_reason,
                "turns": entry.turns,
                // The log's write time: the archive's liveness signal,
                // bounded by the live window rather than a heartbeat the
                // log does not carry.
                "last_write": entry.last_write,
                "kind": entry.kind,
                "workspace": entry.workspace,
                "workspace_inferred": entry.workspace_inferred,
                "channel": entry.channel,
                "compacted": entry.compacted,
                "current": entry.id == current,
                // Whose work this is, and — for a scheduled run — which
                // job, so the background group can be one line per job
                // instead of a pile of anonymous runs.
                "class": work_class(entry),
                "job": scheduled_job(entry),
                // The one derived word the list groups by.
                "state": work_state(entry, live, paused),
                // The content match's line, when a query asked for one
                // (D9): the row names why it matched, the whole line
                // clipped, never a summary a model wrote.
                "matched": needle
                    .as_deref()
                    .and_then(|needle| session_match(home, &entry.id, needle)),
            })
        })
        .collect::<Vec<_>>();
    // The counts the page shows are derived here, once, from the same
    // state words it renders: a head that re-derived them in the browser
    // would be a second answer to what the archive holds (KI-300).
    let count = |wanted: &[&str]| {
        sessions
            .iter()
            .filter(|row| {
                row.get("state")
                    .and_then(Value::as_str)
                    .map(|state| wanted.contains(&state))
                    .unwrap_or(false)
            })
            .count()
    };
    let needs = count(&["needs-approval", "paused"]);
    let working = count(&["working"]);
    let background = sessions
        .iter()
        .filter(|row| {
            matches!(
                row.get("class").and_then(Value::as_str),
                Some("background") | Some("delegated") | Some("channel")
            )
        })
        .count();
    let total = sessions.len();
    let earlier = total.saturating_sub(needs + working + background);
    // The owner's own conversations: what the work list shows by default.
    // The app's background work is counted separately so the head can
    // name the owner's work without the page subtracting anything itself.
    let yours = needs + working + earlier;
    json!({
        "type": "sessions",
        "read_at": crate::boards::now_stamp(),
        "current": current,
        "search": searched.map(|searched| json!({
            "query": query,
            "bound": SEARCH_BOUND,
            "searched": searched,
        })),
        "counts": {
            "needs": needs,
            "working": working,
            "background": background,
            "earlier": earlier,
            "yours": yours,
            "total": total,
        },
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
                    crate::settings::Kind::Usd(_) => "usd",
                    crate::settings::Kind::CountTo(_) => "count",
                },
                "file": match state.file {
                    crate::settings::File::Config => "config",
                    crate::settings::File::Routing => "routing",
                    crate::settings::File::Env => "fleet.env",
                },
                "summary": state.summary,
                "value": state.rendered(),
                "source": state.source(),
                "ceiling": match state.kind {
                    crate::settings::Kind::Usd(ceiling) => json!(ceiling),
                    crate::settings::Kind::CountTo(maximum) => json!(maximum),
                    _ => Value::Null,
                },
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
    let passphrase = crate::home::vault_passphrase();
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
    let router = crate::settings::router_config(home, &dotenv);
    let catalog = crate::settings::catalog(home, &dotenv);
    let lanes = lanes_document(home);
    let servers = servers_document(home);
    let roles = roles_document(home);
    let vault = crate::settings::vault_state(home);
    let (telegram_present, telegram_source) =
        crate::settings::secret_state(home, "FLEET_TELEGRAM_BOT_TOKEN");
    // The tool surface as the settings board reads it: the native
    // tools, the tier bounds, and the last session's measured calls.
    let last_audit = crate::session::list(home)
        .ok()
        .and_then(|mut list| list.pop())
        .and_then(|summary| {
            crate::tool_audit::audit(&summary.path).ok().map(|audit| {
                json!({
                    "session": summary.id,
                    "calls": audit.calls,
                    "errors": audit.errors,
                    "truncated": audit.truncated,
                    "redundant": audit.redundant,
                    "p95_bytes": audit.p95_bytes,
                    "methods": audit.methods,
                })
            })
        });
    // The lifecycle column: every native tool's state, the disabled
    // names, and the last applied sweep \u{2014} the card's "what is on
    // and what the evidence says about it".
    let thresholds = crate::settings::lifecycle_thresholds(home);
    let disabled_tools = crate::settings::tools_disabled(home);
    let lifecycle_scan = crate::tool_advisor::scan(home, thresholds.window);
    let lifecycle_metrics = crate::tool_advisor::lifecycle_metrics(&lifecycle_scan);
    let states: std::collections::BTreeMap<String, String> = lifecycle_metrics
        .iter()
        .map(|metrics| {
            (
                metrics.name.clone(),
                crate::lifecycle::state_of(metrics, &thresholds, &disabled_tools)
                    .label()
                    .to_string(),
            )
        })
        .collect();
    let tools_document = json!({
        // The whole native surface, not only the built-ins: a tool the
        // model can call must be a row the owner can see the state of,
        // because the enable action lives on that row.
        "native": crate::agent::Agent::base_declarations(&[])
            .iter()
            .map(|spec| {
                json!({
                    "name": spec.name(),
                    "summary": spec.description(),
                    "state": states.get(spec.name()),
                })
            })
            .collect::<Vec<_>>(),
        "deferred_above": crate::agent::MAX_VISIBLE_TOOLS,
        "definition_tokens": crate::agent::MAX_DEFINITION_TOKENS,
        "calls_per_turn": crate::agent::MAX_CALLS_PER_TURN,
        "last_session": last_audit,
        "disabled": disabled_tools,
        "last_sweep": crate::settings::last_sweep(home),
        "window": thresholds.window,
        "skills": {
            "retired": crate::skills::retired(home),
        },
        "workflows": {
            "disabled": crate::settings::workflows_disabled(home),
        },
    });
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
        "router": router_document(&router),
        "catalog": catalog_document(&catalog),
        "lanes": lanes,
        "servers": servers,
        "tools": tools_document,
        "roles": roles,
        "vault": {
            "names": vault.names,
            "exists": vault.exists,
            "passphrase": vault.unlocked,
            "plaintext": vault.plaintext.iter().map(|entry| json!({
                "name": entry.name,
                "sealed": entry.sealed,
            })).collect::<Vec<_>>(),
        },
        "telegram": {
            "present": telegram_present,
            "source": telegram_source,
        },
    })
}

fn roles_document(home: &std::path::Path) -> Value {
    let entries = crate::roles::catalog(home)
        .into_iter()
        .map(|entry| match entry.role {
            Some(role) => json!({
                "name": role.name,
                "description": role.description,
                "tools": role.tools,
                "skills": role.skills,
                "shapes": role.shapes,
                "overlay": role.overlay,
                "path": role.path.display().to_string(),
                "modified": role.modified.and_then(|at| crate::session::iso_utc_stamp(at).ok()),
                "broken": Value::Null,
            }),
            None => json!({
                "name": entry.name,
                "description": "",
                "tools": [],
                "skills": [],
                "shapes": [],
                "overlay": "",
                "path": "",
                "modified": Value::Null,
                "broken": entry.broken,
            }),
        })
        .collect::<Vec<_>>();
    json!({
        "dir": crate::roles::roles_dir(home).display().to_string(),
        "entries": entries,
    })
}

fn router_document(config: &crate::settings::RouterConfig) -> Value {
    json!({
        "spend_guard": config.spend_guard,
        "min_confidence": config.min_confidence,
        "default_workload": config.default_workload,
        "pin": config.pin,
        "orchestrator": config.orchestrator,
        "tiers": config.tiers.iter().map(|tier| json!({
            "workload": tier.workload,
            "candidates": tier.candidates,
            "resolved": tier.resolved,
            "reason": tier.reason,
            "reverts": tier.reverts,
        })).collect::<Vec<_>>(),
        "roster": config.roster.iter().map(|entry| json!({
            "id": entry.id,
            "base_url": entry.base_url,
            "models": entry.models,
            "kind": entry.kind,
            "billing": entry.billing,
            "enabled": entry.enabled,
            "key_present": entry.key_present,
            "key_required": entry.key_required,
            "key_env": entry.key_env,
            "disabled_reason": entry.disabled_reason,
        })).collect::<Vec<_>>(),
        "refused": config.refused,
    })
}

fn catalog_document(catalog: &crate::settings::Catalog) -> Value {
    json!({
        "models": catalog.models.iter().map(|model| json!({
            "value": model.value,
            "provider": model.provider,
            "model": model.model,
            "known": model.known,
            "enabled": model.enabled,
            "key_present": model.key_present,
            "billing": model.billing,
            "kind": model.kind,
        })).collect::<Vec<_>>(),
        "providers": catalog.providers.iter().map(|entry| json!({
            "id": entry.id,
            "kind": entry.kind,
            "billing": entry.billing,
            "enabled": entry.enabled,
            "key_present": entry.key_present,
        })).collect::<Vec<_>>(),
        "workloads": catalog.workloads,
        "env_names": catalog.env_names.iter().map(|name| json!({
            "name": name.name,
            "present": name.present,
            "source": name.source,
        })).collect::<Vec<_>>(),
        "paths": catalog.paths.iter().map(|(value, label)| json!({
            "value": value,
            "label": label,
        })).collect::<Vec<_>>(),
        "tools": catalog.tools,
        "skills": catalog.skills,
        "shapes": catalog.shapes,
    })
}

/// The lane configuration as the cards read it: the file's own document,
/// one row per registered lane with its resolved home, and the pickable
/// values the lane's defaults publish.
/// The MCP registry's document for the settings read: one row per
/// configured server — its enabled state, its env NAMES (the registry
/// has no values), its tool count from the last probe, the probe line,
/// and the import source. The args ride `display_args`' redaction, so
/// even a value the file itself carries never reaches the page.
fn servers_document(home: &std::path::Path) -> Value {
    let servers = match crate::mcp_registry::read_servers(home) {
        Ok(servers) => servers,
        Err(error) => {
            return json!({
                "servers": [],
                "refused": [error],
            });
        }
    };
    let rows = servers
        .into_iter()
        .map(|(name, entry)| {
            let (program, args) = crate::mcp_registry::display_entry(&entry);
            json!({
                "name": name,
                "enabled": entry.enabled,
                "program": program,
                "args": args,
                "env_names": entry.env_names,
                "last_probe": entry.last_probe.map(|probe| json!({
                    "at": probe.at,
                    "ok": probe.ok,
                    "server_name": probe.server_name,
                    "server_version": probe.server_version,
                    "tools": probe.tools,
                    "reason": probe.reason,
                })),
                "source": entry.source.map(|source| json!({
                    "harness": source.harness,
                    "imported": source.imported,
                })),
            })
        })
        .collect::<Vec<_>>();
    json!({ "servers": rows })
}

fn lanes_document(home: &std::path::Path) -> Value {
    let document = match crate::lanes::read_lanes(home) {
        Ok(document) => document,
        Err(error) => {
            return json!({
                "document": {},
                "lanes": [],
                "refused": [error],
            });
        }
    };
    let lanes = crate::lanes::LANES
        .iter()
        .map(|lane| {
            let configured = document
                .get("lanes")
                .and_then(|lanes| lanes.get(lane.name))
                .cloned()
                .unwrap_or(Value::Null);
            let (resolved_home, source, unavailable) = match crate::lanes::resolve_home(lane) {
                crate::lanes::LaneHome::Resolved { home, source } => (
                    Some(home.display().to_string()),
                    Some(source.to_string()),
                    None,
                ),
                crate::lanes::LaneHome::Unavailable { reason } => (None, None, Some(reason)),
            };
            let choices = crate::lanes::choices(lane.name)
                .into_iter()
                .map(|choice| {
                    json!({
                        "value": choice.value,
                        "label": choice.label,
                        "provider": choice.provider,
                        "kind": choice.kind,
                    })
                })
                .collect::<Vec<_>>();
            json!({
                "name": lane.name,
                "role": lane.role,
                "configured": configured,
                "resolved_home": resolved_home,
                "source": source,
                "unavailable": unavailable,
                "choices": choices,
            })
        })
        .collect::<Vec<_>>();
    json!({
        "document": document,
        "lanes": lanes,
        "refused": [],
    })
}

/// The scan's document for the settings card's discovery half: the
/// rows in review order, each with its harness, badge, status, and the
/// redacted launch line — never a value.
fn scan_document(home: &std::path::Path) -> Value {
    let report = crate::mcp_registry::scan(home);
    let rows = report
        .rows
        .iter()
        .map(|row| {
            let (status, launch, env_names) = match &row.shape {
                crate::mcp_registry::ScanShape::Stdio {
                    program,
                    args,
                    env_names,
                    ..
                } => (
                    if row.configured {
                        "configured".to_string()
                    } else {
                        "available".to_string()
                    },
                    {
                        let mut shown = vec![program.clone()];
                        shown.extend(crate::mcp_registry::display_args(args));
                        shown.join(" ")
                    },
                    env_names.clone(),
                ),
                crate::mcp_registry::ScanShape::Unsupported { reason } => {
                    ("unsupported".to_string(), reason.clone(), Vec::new())
                }
            };
            json!({
                "name": row.name,
                "harness": row.harness,
                "file": row.file,
                "badge": row.badge.label(),
                "status": status,
                "launch": launch,
                "env_names": env_names,
            })
        })
        .collect::<Vec<_>>();
    json!({ "rows": rows, "skips": report.skips })
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
    let list = |key: &str| {
        arguments
            .get(key)
            .and_then(serde_json::Value::as_array)
            .map(|list| {
                list.iter()
                    .filter_map(serde_json::Value::as_str)
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    };
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
        "server" => {
            let name = text("name")?;
            match arguments.get("config") {
                // The full entry: an import-shaped write or a hand edit
                // through the card. The registry's own validation rules.
                Some(config) => crate::settings::set_server(home, &name, config),
                None => {
                    let enabled = flag("enabled")
                        .ok_or_else(|| "the write says enable or disable".to_string())?;
                    crate::settings::set_server_enabled(home, &name, enabled)
                }
            }
        }
        "server-remove" => {
            let name = text("name")?;
            crate::settings::remove_server(home, &name)
        }
        // The tool lifecycle: disable or enable one native tool through
        // the same writer the CLI rides. A protected tool refuses here
        // too \u{2014} the card is not a way around the gate.
        "tool" => {
            let name = text("name")?;
            let disabled =
                flag("disabled").ok_or_else(|| "the write says disable or enable".to_string())?;
            if disabled && crate::lifecycle::PROTECTED_TOOLS.contains(&name.as_str()) {
                return Err(format!(
                    "`{name}` is protected \u{2014} the working core the sweep never touches"
                ));
            }
            crate::settings::set_tool_disabled(home, &name, disabled)?;
            Ok(name)
        }
        "server-import" => {
            let name = text("name")?;
            let harness = arguments.get("harness").and_then(Value::as_str);
            let report = crate::mcp_registry::scan(home);
            let mut found: Vec<&crate::mcp_registry::ScanRow> =
                report.rows.iter().filter(|row| row.name == name).collect();
            if let Some(harness) = harness {
                found.retain(|row| row.harness == harness);
            }
            match found.as_slice() {
                [one] => crate::mcp_registry::import(home, one, &crate::boards::now_stamp()
                    .chars()
                    .take(10)
                    .collect::<String>()),
                [] => Err(format!(
                    "`{name}` is not in the scan's answer; run a scan first"
                )),
                many => Err(format!(
                    "`{name}` is discovered under more than one harness ({}); the import                      names one",
                    many.iter().map(|row| row.harness).collect::<Vec<_>>().join(", ")
                )),
            }
        }
        "lane" => {
            let lane = text("lane")?;
            let config = arguments
                .get("config")
                .cloned()
                .ok_or_else(|| "the write names its config".to_string())?;
            crate::settings::set_lane(home, &lane, &config)
        }
        "role" => {
            let name = text("name")?;
            let fields = crate::roles::RoleFields {
                description: text("description")?,
                tools: list("tools"),
                skills: list("skills"),
                shapes: list("shapes"),
                overlay: arguments
                    .get("overlay")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
            };
            crate::roles::write(home, &name, &fields)?;
            Ok(name)
        }
        "role-remove" => {
            let name = text("name")?;
            crate::roles::remove(home, &name)?;
            Ok(name)
        }
        "secret" => {
            let name = text("name")?;
            let value = text("value")?;
            crate::settings::set_secret(home, &name, &value)
        }
        "secret-remove" => {
            let name = text("name")?;
            crate::settings::remove_secret(home, &name)
        }
        "vault-unlock" => {
            let passphrase = text("passphrase")?;
            crate::settings::vault_unlock(home, &passphrase)?;
            Ok("unlocked".to_string())
        }
        "vault-lock" => {
            crate::settings::vault_lock();
            Ok("locked".to_string())
        }
        "vault-init" => {
            let migrated = crate::settings::vault_init(home)?;
            Ok(migrated.to_string())
        }
        "vault-onboard" => {
            let passphrase = text("passphrase")?;
            let migrated = crate::settings::vault_onboard(home, &passphrase)?;
            Ok(migrated.to_string())
        }
        "vault-reset" => {
            let passphrase = text("passphrase")?;
            let (backup, migrated) = crate::settings::vault_reset(home, &passphrase)?;
            Ok(format!("{backup}:{migrated}"))
        }
        "vault-seal-env" => {
            let name = text("name")?;
            crate::settings::vault_seal_env(home, &name)
        }
        "vault-remove-env" => {
            let name = text("name")?;
            crate::settings::vault_remove_env(home, &name)?;
            Ok(name)
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
    let passphrase = crate::home::vault_passphrase();
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

// --- the second-brain tool plugin -------------------------------------------

/// The tool's port: `SECOND_BRAIN_PORT` in the process environment or the
/// home's dotenv, the same resolution the settings surface uses, then the
/// tool's own default. A value that is not a port is ignored rather than
/// guessed at, so a typo leaves the known address standing.
fn second_brain_port(dotenv: &std::collections::HashMap<String, String>) -> u16 {
    crate::home::resolve_setting("SECOND_BRAIN_PORT", dotenv)
        .and_then(|value| value.trim().parse::<u16>().ok())
        .filter(|port| *port != 0)
        .unwrap_or(crate::plugins::second_brain::DEFAULT_PORT)
}

/// The tool adapter and the port it would listen on. The home is the
/// board's own resolution — `SECOND_BRAIN_HOME`, then the linked
/// `~/.second-brain`, then `~/Projects/second-brain` — so the plugin and
/// the state read cannot disagree about which tool this machine has.
fn second_brain_resolve(
    home: &std::path::Path,
) -> (Option<crate::plugins::second_brain::SecondBrain>, u16) {
    let passphrase = crate::home::vault_passphrase();
    let dotenv = crate::home::read_dotenv_with_vault(home, passphrase.as_deref());
    let sources = crate::boards::resolve_sources(home, &dotenv);
    let port = second_brain_port(&dotenv);
    (
        sources
            .brain_home
            .map(|brain_home| crate::plugins::second_brain::SecondBrain::new(brain_home, port)),
        port,
    )
}

/// The plugin's facts and its pipeline, as the browser reads them. A tool
/// that is down is a fact with its reason, not an empty read: the page
/// keeps showing the state the tool last published and says why refresh
/// cannot reach it.
fn second_brain_plugin_read(home: &std::path::Path) -> Value {
    let (tool, port) = second_brain_resolve(home);
    let address = format!("http://127.0.0.1:{port}");
    let jobs = json!(crate::plugins::second_brain::JOBS
        .iter()
        .map(|(name, label)| json!({ "name": name, "label": label }))
        .collect::<Vec<_>>());
    let Some(tool) = tool else {
        return json!({
            "type": "second_brain_plugin",
            "plugin": {
                "name": "second brain",
                "role": "tool",
                "home": null,
                "address": address,
                "live": false,
                "has_tool": false,
                "jobs": jobs,
            },
            "pipeline": null,
            "refused": ["the second-brain plugin has no home on this machine"],
            "read_at": crate::boards::now_stamp(),
        });
    };
    let mut refused: Vec<String> = Vec::new();
    let pipeline = match tool.status() {
        Ok(pipeline) => Some(pipeline),
        Err(reason) => {
            refused.push(reason);
            None
        }
    };
    json!({
        "type": "second_brain_plugin",
        "plugin": {
            "name": "second brain",
            "role": "tool",
            "home": tool.home().display().to_string(),
            "address": tool.address(),
            "live": pipeline.is_some(),
            "has_tool": tool.has_tool(),
            "jobs": jobs,
        },
        "pipeline": pipeline.as_ref().map(pipeline_json),
        "refused": refused,
        "read_at": crate::boards::now_stamp(),
    })
}

fn pipeline_json(pipeline: &crate::plugins::second_brain::Pipeline) -> Value {
    json!({
        "running": pipeline.running,
        "job": &pipeline.job,
        "current": pipeline.current,
        "steps": pipeline
            .steps
            .iter()
            .map(|step| json!({
                "label": step.label,
                "status": step.status,
                "ms": step.ms,
            }))
            .collect::<Vec<_>>(),
        "log": &pipeline.log,
        "started": &pipeline.started,
        "finished": &pipeline.finished,
        "error": &pipeline.error,
        "last_ok": &pipeline.last_ok,
        "built_at": &pipeline.built_at,
    })
}

/// One refresh, forwarded to the tool. A home that resolves but holds no
/// `control_tower.py` is named as such rather than as a dead server: the
/// misconfiguration and the outage have different fixes.
fn second_brain_refresh(home: &std::path::Path, job: &str) -> Result<Value, String> {
    let (tool, port) = second_brain_resolve(home);
    let Some(tool) = tool else {
        return Err("the second-brain plugin has no home on this machine".to_string());
    };
    if !tool.has_tool() {
        return Err(format!(
            "the second-brain tool is not in {} (no control_tower.py), so there is nothing at \
             port {port} to refresh; set SECOND_BRAIN_HOME to the tool's repository",
            tool.home().display()
        ));
    }
    tool.refresh(job)
}

fn attention_row_json(row: &crate::boards::AttentionRow) -> Value {
    json!({
        "project": row.project,
        "what": row.what,
        "evidence": row.evidence,
        "detail": row.detail,
        "when": row.when,
    })
}

fn atlas_project_json(project: &crate::boards::AtlasProject) -> Value {
    json!({
        "name": project.name,
        "lifecycle": project.lifecycle,
        "role": project.role,
        "commits_30d": project.commits_30d,
        "dirty": project.dirty,
        "open_plans": project.open_plans,
        "drift_days": project.drift_days,
        "weeks": project.weeks,
        "reports": project.reports,
        "total": project.total,
        "stale_plans": project.stale_plans,
        "conflicts": project.conflicts,
        "handoff": project.handoff.as_ref().map(|handoff| json!({
            "date": handoff.date,
            "path": handoff.path,
            "title": handoff.title,
            "score": handoff.next_score,
            "next": handoff.next_lines,
        })),
    })
}

/// The second-brain panel's document: the six sections, each landed or
/// named, shaped for the browser.
fn brain_read(home: &std::path::Path) -> Value {
    // The vault laid over the dotenv: a sealed key answers here exactly
    // where a plaintext key would have, so the router and the agent see
    // the same credentials either way.
    let passphrase = crate::home::vault_passphrase();
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
                "feed": reports.feed.iter().map(|row| json!({
                    "ts": row.ts,
                    "kind": row.kind,
                    "project": row.project,
                    "harness": row.harness,
                    "summary": row.summary,
                    "commit": row.commit,
                })).collect::<Vec<_>>(),
                "trust": reports.trust.iter().map(|row| json!({
                    "harness": row.harness,
                    "claimed": row.claimed,
                    "evidenced": row.evidenced,
                    "precision": row.precision,
                })).collect::<Vec<_>>(),
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
    if let Some(tower) = snapshot.tower.landed() {
        let reports = snapshot.reports.landed();
        let windows = crate::boards::pulse_windows(&tower.generated_at);
        let attention = crate::boards::attention_queue(tower, reports);
        let projects = crate::boards::atlas_projects(tower, reports);
        document.insert("state_at".into(), json!(tower.generated_at));
        document.insert(
            "events_since".into(),
            json!(crate::boards::ledger_events_since(
                &snapshot.home,
                &tower.generated_at
            )),
        );
        document.insert(
            "pulse".into(),
            json!({
                "weeks": windows.iter().map(|(start, end)| json!({
                    "start": start, "end": end,
                })).collect::<Vec<_>>(),
            }),
        );
        document.insert(
            "attention".into(),
            json!({
                "total": attention.total,
                "counts": {
                    "drift": attention.drift.len(),
                    "stale": attention.stale.len(),
                    "handoff": attention.handoff.len(),
                    "conflicts": attention.conflicts.len(),
                },
                "drift": attention.drift.iter().map(attention_row_json).collect::<Vec<_>>(),
                "stale": attention.stale.iter().map(attention_row_json).collect::<Vec<_>>(),
                "handoff": attention.handoff.iter().map(attention_row_json).collect::<Vec<_>>(),
                "conflicts": attention.conflicts.iter().map(attention_row_json).collect::<Vec<_>>(),
            }),
        );
        document.insert(
            "projects".into(),
            json!(projects.iter().map(atlas_project_json).collect::<Vec<_>>()),
        );
        document.insert(
            "commits".into(),
            json!(crate::boards::commits_by_day(tower)
                .iter()
                .map(|(day, count)| json!({ "day": day, "count": count }))
                .collect::<Vec<_>>()),
        );
    } else if let Some(reason) = snapshot.tower.reason() {
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
        "retired": snapshot.retired,
        "sessions": snapshot.sessions,
        "unused": snapshot.unused,
        "curation_note": snapshot.curation_note,
        "refused": snapshot.refused,
    })
}

/// One tree node and its children, recursively.
fn delegation_node(
    node: &crate::boards::DelegationNode,
    branches: &std::collections::HashMap<String, String>,
) -> Value {
    json!({
        "id": node.id,
        "model": node.model,
        "kind": node.kind,
        "depth": node.depth,
        "goal": node.goal,
        "status": node.status,
        "stop_reason": node.stop_reason,
        "turns": node.turns,
        // The branch this child's work stands on, when it worked one:
        // the row's link into the workspace-history section.
        "workspace_branch": branches.get(&node.id),
        "children": node.children.iter().map(|child| delegation_node(child, branches)).collect::<Vec<_>>(),
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
fn record(state: &Mutex<GuiState>, events: &broadcast::Sender<Value>, session: &str, value: Value) {
    // Every event names the session it belongs to, so a surface reading
    // one conversation can drop another's tokens instead of appending a
    // background run's words to the transcript it happens to have open.
    // The field is the session the thread is driving; an empty one is a
    // face whose launch log does not exist yet, and stays untagged.
    let mut value = value;
    if !session.is_empty() {
        if let Some(object) = value.as_object_mut() {
            object
                .entry("session")
                .or_insert_with(|| Value::String(session.to_string()));
        }
    }
    let mut notice = None;
    // The run is over the moment the core says so, whichever word it used:
    // a header that kept counting after the loop stopped would be lying
    // about where the owner is.
    let finished = matches!(
        value.get("type").and_then(Value::as_str),
        Some("finished") | Some("failed")
    );
    {
        let mut state = lock(state);
        {
            let attached = state.attached(session);
            if finished {
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
        // The run that was driving is over. Clearing it here — at the one
        // seam every event crosses, beside the flag it belongs with —
        // closes the window where the archive still said a finished run
        // was driving and a send in another conversation queued behind
        // work that had ended (KI-301).
        if finished && state.driving.as_deref() == Some(session) {
            state.driving = None;
        }
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
pub(crate) fn describe(event: &UiEvent) -> Value {
    match event {
        UiEvent::Token(text) => json!({"type": "token", "text": text}),
        UiEvent::Reasoning { text, seconds } => json!({
            "type": "reasoning",
            "text": text,
            "seconds": seconds,
        }),
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
        UiEvent::ToolCall {
            name,
            summary,
            arguments,
        } => json!({
            "type": "tool_call",
            "tool": name,
            "summary": summary,
            // The full call, so an expanded row names what ran rather
            // than the clipped line.
            "arguments": arguments,
        }),
        UiEvent::ToolResult {
            name,
            preview,
            approved,
            resolution,
        } => json!({
            "type": "tool_result",
            "tool": name,
            "preview": preview,
            "approved": approved,
            "resolution": resolution,
        }),
        UiEvent::LaneJoined { lane, tool_count } => {
            json!({"type": "lane", "lane": lane, "tools": tool_count})
        }
        // A file handed to the owner: the card is the delivery, and the
        // path is what the owner opens.
        UiEvent::Artifact { path, size, note } => json!({
            "type": "artifact",
            "path": path,
            "size": size,
            "note": note,
        }),
        UiEvent::Routed {
            provider,
            model,
            how,
            workload,
            verdict,
            judged_from,
            reason,
            order,
            execution,
        } => json!({
            "type": "routed",
            "provider": provider,
            "model": model,
            "how": how,
            "workload": workload,
            "verdict": verdict,
            "judged_from": judged_from,
            "reason": reason,
            "order": order,
            "execution": execution,
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
        // The harness asked for the report it was owed. The page shows
        // it as a line, so the extra model call is visible rather than
        // a turn that seemed to do nothing.
        UiEvent::ReportRepair { attempt } => {
            json!({"type": "repair", "attempt": attempt})
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
    home: PathBuf,
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
                    // shows (KI-022). A folder chosen before that first
                    // task moves with it and lands in the sidecar under
                    // the id it will be read back by.
                    if let UiEvent::Session { id } = &event {
                        if lock(&state).name_launch(id).is_some() {
                            let map = lock(&state).workdirs.clone();
                            save_workdirs(&home, &map);
                        }
                        // Independent of the folder: a goal set before the
                        // first run moves to the session's own key.
                        move_pending_goal(&home, id);
                    }
                    // The event belongs to the session the thread is
                    // driving, which is exact because the thread's
                    // command loop is serial: the events for one task all
                    // arrive before the next command is read.
                    let session = lock(&state).target();
                    let mut described = describe(&event);
                    // The events that close a run or raise a seat name
                    // their session, for the reason the seat does.
                    if let Some(object) = described.as_object_mut() {
                        if matches!(
                            object.get("type").and_then(Value::as_str),
                            Some("finished") | Some("failed")
                        ) {
                            object.insert("session".to_string(), json!(session));
                        }
                    }
                    record(&state, &events, &session, described);
                }
                None => ui_done = true,
            },
            request = seat_rx.recv(), if !seat_done => match request {
                Some(request) => {
                    let session = lock(&state).target();
                    let mut pending = describe_seat(&request.call, &request.summary);
                    // The seat names its session: seats are per-session,
                    // and a surface watching another conversation must
                    // not be shown one that is not theirs.
                    if let Some(object) = pending.as_object_mut() {
                        object.insert("session".to_string(), json!(session));
                        // The seat names when it was raised, so the wait a
                        // surface shows is the seat's own clock and not the
                        // page's.
                        object.insert("since".to_string(), json!(epoch_millis()));
                    }
                    let mut state = lock(&state);
                    let attached = state.attached(&session);
                    attached.seat = Some(request);
                    attached.seat_at = epoch_millis();
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
    /// The run's approval mode, the same gate the console and the TUI
    /// run: `ask` prompts for every call (minus remembered rules),
    /// `yes-guarded` runs everything but the destructive class, and
    /// `yes` runs everything but the always-ask class.
    gate: approvals::Gate,
    /// The run-scoped "allow everything" latch, reported through
    /// [`Approver::latched`] exactly as the console's and the TUI's are.
    always: bool,
}

impl GuiApprover {
    /// An approver in `mode` that prompts `requests` and consults
    /// `rules` first.
    pub fn new(
        requests: UnboundedSender<ApprovalRequest>,
        rules: Arc<Mutex<approvals::Rules>>,
        mode: approvals::Mode,
    ) -> GuiApprover {
        GuiApprover {
            requests,
            rules,
            gate: approvals::Gate::new(mode),
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
        // The mode is the second answer, after the owner's remembered
        // rules: the gate knows the destructive and always-ask classes,
        // so a call it clears needs no prompt and a call it does not
        // still reaches the seat.
        if self.gate.approves_without_asking(call) {
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
        self.always || self.gate.mode() == approvals::Mode::Yes
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
            // The loader's skips are reported once, when the agent thread
            // starts: a malformed skill is a named line, not a per-run
            // one. What a run *declares* is read per run below, because
            // the files change between runs and the owner's retirements
            // have to reach the run in flight of them.
            let user_home = std::env::var("HOME").ok();
            let skills_override = std::env::var("FLEET_SKILLS_DIR").ok();
            if let Some(dir) =
                crate::skills::resolve_skills_dir(skills_override.as_deref(), user_home.as_deref())
            {
                let skips = crate::skills::load_reported(&dir).1;
                if !skips.is_empty() {
                    let _ = ui_tx.send(UiEvent::SkillsLoaded {
                        loaded: crate::skills::load(&dir).len(),
                        skipped: skips.into_iter().map(|skip| skip.name).collect(),
                    });
                }
            }
            // The declared list for one run: the library with the owner's
            // retired filtered out (KI-215's shape — the owner retires on
            // the skills page, the library is never written). Read per
            // run, so a retirement reaches the next run without a
            // restart, and each run's `skills` record pins what that run
            // actually saw.
            let declared_skills = || {
                crate::skills::resolve_skills_dir(
                    std::env::var("FLEET_SKILLS_DIR").ok().as_deref(),
                    std::env::var("HOME").ok().as_deref(),
                )
                .map(|dir| {
                    crate::skills::note_retired(&home);
                    crate::skills::load_declared(&dir, &home)
                })
                .unwrap_or_default()
            };
            // The route is chosen once per session, on that session's
            // first task, and the committed router is reused for the rest
            // of it — one sys1 judgment, not one per task. A session this
            // thread has never driven has no entry and is judged when its
            // first task arrives.
            let mut routes: std::collections::HashMap<String, Option<crate::route::Prepared>> =
                std::collections::HashMap::new();
            // The provider each prepared route was judged under: a chat
            // that changes its pin is re-routed rather than served the
            // old decision.
            let mut route_pins: std::collections::HashMap<String, Option<String>> =
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
                let plan_mode = matches!(command, Command::Plan(_, _, _));
                match command {
                    Command::Run(driving, task, folder) | Command::Plan(driving, task, folder) => {
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
                                        // The goal set before this run rides the
                                        // pending key: move it here, ahead of the
                                        // options below, so the run declares it
                                        // (the bridge's own move lags the thread).
                                        move_pending_goal(&home, &id);
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
                        let pin = lock(&state).provider_pin.clone();
                        if route_pins.get(&driving).cloned().flatten() != pin {
                            routes.insert(driving.clone(), None);
                        }
                        let slot = routes.entry(driving.clone()).or_insert(None);
                        if slot.is_none() {
                            match crate::route::prepare_with_pin(
                                &home,
                                &dotenv,
                                &task,
                                &std::collections::HashMap::new(),
                                pin.as_deref(),
                            )
                            .await
                            {
                                Ok(fresh) => {
                                    judge_tasks.insert(driving.clone(), task.clone());
                                    *slot = Some(fresh);
                                    route_pins.insert(driving.clone(), pin.clone());
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
                        // The turn budget: the home's config value,
                        // tightened by the face's per-run override when
                        // the run body named one. The pair is cleared
                        // after it is read — a budget is this run's
                        // choice, not the chat's new standing policy.
                        let mut turn_budget = crate::route::run_config(&home).max_turns;
                        {
                            let state = lock(&state);
                            let named = state.turn_budget;
                            if named > 0 {
                                turn_budget = named;
                            }
                        }
                        lock(&state).turn_budget = 0;
                        // The registry's enabled servers seed the run's
                        // lane specs, so the run pre-joins them before
                        // its first request (D2). A name the env chain
                        // cannot resolve fails the run by name — the
                        // face reads it as a failed run, the model
                        // spawned nothing half-configured.
                        let dotenv: std::collections::BTreeMap<String, String> =
                            crate::home::read_dotenv_with_process_vault(&home)
                                .into_iter()
                                .collect();
                        let mut seed = AgentOptions::default();
                        let lane_specs = match seed.seed_user_servers(&home, &dotenv) {
                            Ok(absent) => {
                                // The named absences (the 2026-10-26
                                // plan): one line per unkeyed server, the
                                // run's own startup voice — the face's
                                // stderr is where its runs print.
                                for (server, reason) in absent {
                                    eprintln!(
                                        "fleet: server `{server}`: {reason} — its tools are not declared this run"
                                    );
                                }
                                seed.lane_specs
                            }
                            Err(error) => {
                                let _ = ui_tx.send(UiEvent::Failed(format!(
                                    "the run did not start: {error}"
                                )));
                                continue;
                            }
                        };
                        let options = AgentOptions {
                            // The session's standing objective, read per
                            // run so a change reaches the next run with
                            // no restart.
                            goal: session_goal(&home, &driving),
                            model: model.clone(),
                            max_turns: turn_budget,
                            identity: crate::home::read_soul(&home),
                            ui_tx: Some(ui_tx.clone()),
                            lane_specs,
                            calib: Some(prepared.calib.clone()),
                            skills: declared_skills(),
                            role_overlay: None,
                            memory,
                            scope: None,
                            // The self-maintenance tool rides the
                            // interactive surfaces: a session can file a
                            // proposal, and only the owner's `fleet improve
                            // apply` runs one.
                            improve_home: Some(home.clone()),
                            route_home: None,
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
                            // The chat's working folder, resolved when
                            // the run was accepted. A chat that never
                            // chose one has no filesystem access at
                            // all: the four built-ins are refused by
                            // name, and the refusal tells the model to
                            // ask for a folder.
                            file_access: match folder {
                                Some(root) => crate::agent::FileAccess::Folder(root),
                                None => crate::agent::FileAccess::None,
                            },
                            plan_mode,
                        };
                        let mode = lock(&state).permissions;
                        let approver: Box<dyn Approver> = Box::new(GuiApprover::new(
                            approval_tx.clone(),
                            Arc::clone(&rules),
                            mode,
                        ));
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
                        if let Some(id) = &current_session {
                            routes.insert(id.clone(), None);
                            judge_tasks.remove(id);
                        }
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

/// The rank route's scoring, extracted for the seam's own tests (KI-268):
/// the ask embeds once, each candidate embeds beside it, and a candidate
/// whose embed fails rides at exactly 0.0 — a PARTIAL embed is a shape the
/// route permits, and the reply still orders descending with no refusal.
/// `rank` hands this the process's probed embedder; the tests hand it a
/// double that answers for some candidates and not others.
pub(crate) fn rank_with(
    embedder: &dyn crate::memory::Embedder,
    asked: &str,
    candidates: &[String],
) -> Vec<serde_json::Value> {
    let query = embedder.embed(asked);
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
            serde_json::json!({"index": index, "score": score})
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::FunctionCall;

    /// A scratch home unique per call.
    fn scratch(tag: &str) -> std::path::PathBuf {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let serial = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "fleet-bridge-{tag}-{}-{serial}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// The content match pass (D9): a query drives the rows from the
    /// logs' own owner and assistant text, the matched line rides the
    /// row, and the bound is named — a miss is honest about what it
    /// searched.
    #[test]
    fn a_content_query_names_the_matched_line_and_its_bound() {
        let home = scratch("sessions-search");
        std::fs::create_dir_all(home.join("sessions")).unwrap();
        let write = |id: &str, lines: &str| {
            std::fs::write(home.join("sessions").join(format!("{id}.jsonl")), lines).unwrap();
        };
        write(
            "alpha",
            concat!(
                r#"{"type":"start","v":1,"model":"m","created":"2026-10-23T09:00:00Z"}"#,
                "\n",
                r#"{"type":"user","content":"where did I put the deployment notes"}"#,
                "\n",
                r#"{"type":"assistant","content":"they are in docs/deploy.md"}"#,
                "\n",
            ),
        );
        write(
            "beta",
            concat!(
                r#"{"type":"start","v":1,"model":"m","created":"2026-10-23T08:00:00Z"}"#,
                "\n",
                r#"{"type":"user","content":"unrelated words"}"#,
                "\n",
            ),
        );
        let live = LiveFace::default();
        let document = sessions_read(&home, "", &live, "deployment notes");
        let rows = document["sessions"].as_array().unwrap();
        let matched: Vec<&str> = rows
            .iter()
            .filter(|row| !row["matched"].is_null())
            .map(|row| row["id"].as_str().unwrap())
            .collect();
        assert_eq!(
            matched,
            vec!["alpha"],
            "only the owner-text hit carries a line"
        );
        let hit = rows
            .iter()
            .filter_map(|row| row["matched"].as_str())
            .collect::<Vec<_>>();
        assert!(
            hit.iter().any(|line| line.contains("deployment notes")),
            "the owner's line rides the row: {hit:?}"
        );
        let document = sessions_read(&home, "", &live, "deploy.md");
        let rows = document["sessions"].as_array().unwrap();
        let hit = rows
            .iter()
            .filter_map(|row| row["matched"].as_str())
            .collect::<Vec<_>>();
        assert!(
            hit.iter().any(|line| line.contains("docs/deploy.md")),
            "the assistant's answer matches: {hit:?}"
        );
        assert_eq!(
            document["search"]["bound"].as_u64(),
            Some(SEARCH_BOUND as u64)
        );
        assert_eq!(document["search"]["searched"].as_u64(), Some(2));
        let plain = sessions_read(&home, "", &live, "");
        assert!(plain["search"].is_null(), "no query, no bound named");
        let document = sessions_read(&home, "", &live, "nothing says this");
        assert!(document["sessions"]
            .as_array()
            .unwrap()
            .iter()
            .all(|row| row["matched"].is_null()));
        let _ = std::fs::remove_dir_all(&home);
    }

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
            "deepseek-flash".to_string(),
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
    async fn a_pairing_card_mints_a_window_and_names_its_origin() {
        let (bridge, _sources, _door) = bridge().await;
        let home = bridge.home_path();
        std::fs::write(
            home.join("config.json"),
            r#"{"remote":{"public_url":"https://phone.example/"}}"#,
        )
        .unwrap();
        bridge.attach_remote("192.0.2.10".parse().unwrap(), 3080);

        let document = bridge.pair_window().await.expect("a pairing window");
        let url = document["url"].as_str().expect("a url");
        assert!(
            url.starts_with("https://phone.example/pair?c="),
            "the configured origin is used: {url}"
        );
        assert_eq!(document["origin_kind"], "public_url");
        assert_eq!(document["origin"], "https://phone.example");
        assert!(
            document["code"].as_str().unwrap().contains('-'),
            "{document}"
        );
        let svg = document["svg"].as_str().expect("an svg");
        assert!(svg.starts_with("<svg"), "{svg}");
        assert!(svg.contains("viewBox"), "{svg}");

        // Opening the card again mints the newest window; the first code
        // stops being the live one, which the card says.
        let again = bridge.pair_window().await.expect("a second window");
        assert_ne!(document["url"], again["url"]);
        std::fs::remove_dir_all(&home).ok();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_pairing_card_without_the_remote_face_names_the_flag() {
        let (bridge, _sources, _door) = bridge().await;
        let error = bridge.pair_window().await.expect_err("no remote face");
        assert!(error.contains("--remote"), "{error}");
    }

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

    /// A scratch tree for a test's launch directory, unique per call.
    fn tree(tag: &str) -> PathBuf {
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let serial = NEXT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let dir =
            std::env::temp_dir().join(format!("fleet-gui-{tag}-{}-{serial}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a scratch tree");
        dir
    }

    /// C4 (as amended): a chat's folder is kept per session, persisted
    /// in the sidecar, and restored after a reload; a second chat starts
    /// with no folder, and therefore no filesystem access.
    #[tokio::test(flavor = "multi_thread")]
    async fn a_chat_folder_is_per_session_and_survives_a_reload() {
        let (mut bridge, _sources, _door) = bridge().await;
        let root = tree("folder-per-session");
        std::fs::create_dir_all(root.join("docs")).unwrap();
        bridge.cwd = root.clone();
        let home = bridge.home_path();
        let session = Session::create(&home, "m").unwrap();
        let id = session.id().to_string();

        bridge
            .set_folder(&id, Some("docs"))
            .expect("a folder inside the launch directory");
        let state = bridge.state(&id);
        assert_eq!(state["folder"]["name"], "docs", "{state}");
        assert_eq!(
            state["folder"]["path"],
            root.join("docs")
                .canonicalize()
                .unwrap()
                .display()
                .to_string()
        );
        let sidecar = std::fs::read_to_string(home.join(WORKDIRS_FILE)).expect("the sidecar");
        assert!(sidecar.contains(&id), "{sidecar}");

        // A second face over the same home restores the choice; another
        // chat has none and therefore no filesystem access.
        let (reloaded, _sources) =
            Bridge::detached(home.clone(), "m".to_string(), Some(id.clone()));
        let state = reloaded.state(&id);
        assert_eq!(state["folder"]["name"], "docs", "{state}");
        let other = Session::create(&home, "m").unwrap();
        assert!(
            reloaded.state(other.id())["folder"].is_null(),
            "a second chat starts with no folder"
        );

        std::fs::remove_dir_all(&root).ok();
    }

    /// At the bridge seam: a relative `..` walk and a relative symlink
    /// out keep the pane's refusal, a path that does not exist is refused
    /// by name, and — since the 2026-10-21 ruling — an absolute folder
    /// outside the launch directory is accepted, canonical: the picker
    /// browses the whole hierarchy and the choice may land anywhere in it.
    #[tokio::test(flavor = "multi_thread")]
    async fn a_relative_escape_is_refused_and_an_absolute_folder_anywhere_is_accepted() {
        let (mut bridge, _sources, _door) = bridge().await;
        let root = tree("folder-refuse");
        bridge.cwd = root.clone();
        let attached = bridge.launch();

        let error = bridge.chat_folder("../").unwrap_err();
        assert!(error.contains("outside the working directory"), "{error}");
        bridge
            .set_folder(&attached, Some("../"))
            .expect_err("a `..` walk is refused");
        assert!(bridge.state(&attached)["folder"].is_null());

        let outside = tree("folder-refuse-outside");
        let accepted = bridge
            .chat_folder(&outside.display().to_string())
            .expect("an absolute folder anywhere is the owner's grant");
        assert_eq!(accepted, outside.canonicalize().expect("canonical"));

        let error = bridge.chat_folder("no-such-dir").unwrap_err();
        assert!(error.contains("no-such-dir"), "{error}");

        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&outside, root.join("link")).unwrap();
            let error = bridge.chat_folder("link").unwrap_err();
            assert!(error.contains("outside the working directory"), "{error}");
        }

        std::fs::remove_dir_all(&root).ok();
        std::fs::remove_dir_all(&outside).ok();
    }

    /// C6 (as amended): a stored folder that has gone missing falls back
    /// to no folder — not to the launch directory — and the fallback is
    /// the run's first event.
    #[tokio::test(flavor = "multi_thread")]
    async fn a_deleted_folder_falls_back_and_names_itself() {
        let (mut bridge, _sources, _door) = bridge().await;
        let root = tree("folder-fallback");
        let docs = root.join("docs");
        std::fs::create_dir_all(&docs).unwrap();
        bridge.cwd = root.clone();
        let home = bridge.home_path();
        let attached = bridge.launch();
        bridge.set_folder(&attached, Some("docs")).expect("stored");
        std::fs::remove_dir_all(&docs).unwrap();

        bridge
            .run_with(&attached, "work", crate::approvals::Mode::Ask, None, None)
            .expect("the run is accepted");
        let state = bridge.state(&attached);
        let events = state["events"].as_array().expect("an event log");
        let last = events.len() - 1;
        assert_eq!(events[last]["type"], "message", "{events:?}");
        assert_eq!(events[last - 1]["type"], "folder", "{events:?}");
        assert_eq!(events[last - 1]["fallback"], true, "{events:?}");
        assert!(
            events[last - 1]["missing"]
                .as_str()
                .unwrap_or_default()
                .ends_with("docs"),
            "{events:?}"
        );
        assert!(
            events[last - 1]["note"]
                .as_str()
                .unwrap_or_default()
                .contains("no working folder"),
            "{events:?}"
        );
        assert!(state["folder"].is_null(), "the stale choice is cleared");
        let sidecar = std::fs::read_to_string(home.join(WORKDIRS_FILE)).unwrap_or_default();
        assert!(
            !sidecar.contains("docs"),
            "the stale entry is gone: {sidecar}"
        );

        std::fs::remove_dir_all(&root).ok();
    }

    /// C10: full access is a grant only a folder can bound. The run
    /// route refuses it by name, and a stored full mode from before the
    /// folder was cleared refuses the next run through the other door
    /// too rather than arming it.
    #[tokio::test(flavor = "multi_thread")]
    async fn full_access_is_refused_without_a_folder() {
        let (mut bridge, _sources, _door) = bridge().await;
        let root = tree("full-access");
        std::fs::create_dir_all(root.join("docs")).unwrap();
        bridge.cwd = root.clone();
        let attached = bridge.launch();

        let refused = bridge
            .run_with(&attached, "work", crate::approvals::Mode::Yes, None, None)
            .expect_err("full access with no folder");
        match refused {
            RunError::BadRequest(message) => assert!(
                message.contains("full access needs a working folder"),
                "{message}"
            ),
            other => panic!("expected a 400, got {other:?}"),
        }

        // Stored full + a folder, then the folder is cleared: the
        // stored-mode door refuses too.
        bridge
            .run_with(
                &attached,
                "work",
                crate::approvals::Mode::Yes,
                None,
                Some("docs"),
            )
            .expect("a folder bounds full access");
        bridge.set_folder(&attached, None).expect("cleared");
        let refused = bridge.run(&attached, "again").expect_err("refused");
        assert!(
            refused.contains("full access needs a working folder"),
            "{refused}"
        );

        std::fs::remove_dir_all(&root).ok();
    }

    /// The control plane's document names every channel, its paired and
    /// pending peers, and the activity's bound — and a face that hosts no
    /// service says so rather than claiming one.
    #[tokio::test(flavor = "multi_thread")]
    async fn the_control_plane_names_every_channel_and_its_bound() {
        let home = std::env::temp_dir().join(format!(
            "fleet-control-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).expect("a temp home");
        let mut store = crate::channels::PeerStore::default();
        store.pair(
            "wh1",
            "the CI box",
            "unused",
            crate::channels::Capability::Task,
            "2026-10-20T00:00:00Z",
        );
        store.save(&home, "webhook").expect("the webhook store");
        let mut pending = crate::channels::PendingStore::default();
        pending.note("4242", "Alejandro", "2026-10-20T15:00:00Z");
        pending.save(&home, "telegram").expect("the pending store");

        let (bridge, _sources) = Bridge::detached(
            home.clone(),
            "deepseek-flash".to_string(),
            Some("session-test".to_string()),
        );
        let document = bridge.channels_read().await;
        assert_eq!(document["type"], "channels");
        let channels = document["channels"].as_array().expect("the channel list");
        assert_eq!(
            channels.len(),
            crate::channels::CHANNELS.len(),
            "every channel is named, not only the one with peers"
        );
        let telegram = channels
            .iter()
            .find(|channel| channel["name"] == "telegram")
            .expect("telegram is named");
        assert_eq!(telegram["pending"][0]["id"], "4242");
        assert_eq!(telegram["pending"][0]["attempts"], 1);
        assert_eq!(telegram["peers"].as_array().map(Vec::len), Some(0));
        // The activity's bound is named rather than implied.
        assert_eq!(
            document["activity_bound"].as_u64(),
            Some(50),
            "the bound is in the document the surface renders"
        );
        // A face hosting no service says so.
        assert_eq!(document["service"]["running"], false);
        assert!(document["service"]["note"].is_string());
        // And no start is pretended: there is no service in this process.
        assert!(bridge.channel_start("webhook").await.is_err());
        assert!(bridge.channel_stop("webhook").await.is_err());
        std::fs::remove_dir_all(&home).ok();
    }

    /// "Who can reach this face" walks every channel's store, not the
    /// webhook's alone: a paired Telegram chat is as much a way in, and
    /// a read that named one channel told the owner half the truth.
    #[test]
    fn the_access_read_names_every_channels_peers() {
        let home = std::env::temp_dir().join(format!(
            "fleet-access-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).expect("a temp home");

        let mut webhook = crate::channels::PeerStore::default();
        webhook.pair(
            "wh1",
            "the CI box",
            "unused",
            crate::channels::Capability::Task,
            "2026-10-20T00:00:00Z",
        );
        webhook.save(&home, "webhook").expect("the webhook store");
        let mut telegram = crate::channels::PeerStore::default();
        telegram.pair(
            "4242",
            "Alejandro",
            "unused",
            crate::channels::Capability::Observe,
            "2026-10-20T00:00:00Z",
        );
        telegram
            .save(&home, "telegram")
            .expect("the telegram store");

        let read = access_read(&home);
        let peers = read["peers"].as_array().expect("the peer list");
        assert_eq!(peers.len(), 2, "both channels are walked: {peers:?}");
        let telegram_peer = peers
            .iter()
            .find(|peer| peer["channel"] == "telegram")
            .expect("the telegram peer is named");
        assert_eq!(telegram_peer["id"], "4242");
        assert_eq!(telegram_peer["capability"], "may only listen");
        let webhook_peer = peers
            .iter()
            .find(|peer| peer["channel"] == "webhook")
            .expect("the webhook peer is named");
        assert_eq!(webhook_peer["capability"], "may act");
        std::fs::remove_dir_all(&home).ok();
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
        let log = Session::create(&bridge.home, "deepseek-flash").expect("the log");
        let id = log.id().to_string();
        drop(log);
        bridge.drive(&id);
        lock(&bridge.state).attached(&id).running = true;
        let stop = bridge.stop(None).expect("the stop lands");
        assert_eq!(stop["kind"], "stop");
        assert_eq!(stop["session"], id.as_str());
        let control = crate::home::sessions_dir(&bridge.home).join(format!("{id}.control"));
        let body = std::fs::read_to_string(&control).expect("the control file");
        assert_eq!(body.trim(), r#"{"stop":true}"#, "{body}");
        let state = bridge.state(&id);
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

        // A run whose log does not exist yet has nothing to write beside:
        // the thread is driving the pending key, which is the launch
        // session before it has been named.
        bridge.drive("");
        lock(&bridge.state).attached("").running = true;
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
        let log = Session::create(&home, "deepseek-flash").expect("the log");
        let id = log.id().to_string();
        drop(log);
        let control = crate::home::sessions_dir(&home).join(format!("{id}.control"));
        std::fs::write(&control, "{\"stop\":true}\n").expect("the standing control");
        let (mut bridge, sources) =
            Bridge::detached(home.clone(), "deepseek-flash".to_string(), Some(id.clone()));
        let door = bridge.connect();
        let _thread = spawn_agent(AgentInputs {
            home: home.clone(),
            model: "deepseek-flash".to_string(),
            session_id: Some(id.clone()),
            ui_tx: sources.ui,
            approval_tx: sources.seat,
            command_rx: door,
            base_url: "http://127.0.0.1:1".to_string(),
            memories_root: home.clone(),
            state: Arc::clone(&bridge.state),
        });
        bridge.run(&id, "go").expect("the run is accepted");
        // The run's teardown removes the control file; under a loaded
        // suite that can outrun a three-second poll, and a bound the
        // machine cannot meet is a flake, not a failure.
        for _ in 0..1500 {
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
            Bridge::detached(home.clone(), "deepseek-flash".to_string(), None);
        assert_eq!(bridge.state("")["session"], "", "{:?}", bridge.state(""));
        let door = bridge.connect();
        let _thread = spawn_agent(AgentInputs {
            home: home.clone(),
            model: "deepseek-flash".to_string(),
            session_id: None,
            ui_tx: sources.ui,
            approval_tx: sources.seat,
            command_rx: door,
            base_url: "http://127.0.0.1:1".to_string(),
            memories_root: home.clone(),
            state: Arc::clone(&bridge.state),
        });
        bridge.run("", "go").expect("the run is accepted");
        // The session event lands, and the state document names the id.
        // The wait is on the event the assertion below reads, not on the
        // state field alone: the field is set on the run's thread while
        // the event rides the broadcast queue, and under a loaded suite
        // the queue lags the field past a one-second poll (the skills
        // load alone reads hundreds of files on this machine).
        let has_session_event = |state: &serde_json::Value| {
            let id = state["session"].as_str().unwrap_or("");
            !id.is_empty()
                && state["events"].as_array().is_some_and(|events| {
                    events
                        .iter()
                        .any(|event| event["type"] == "session" && event["id"] == id)
                })
        };
        for _ in 0..600 {
            if has_session_event(&bridge.state("")) {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        let state = bridge.state("");
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
        bridge
            .run(&attached, "first")
            .expect("the first run is accepted");
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
            "deepseek-flash".to_string(),
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
        let answered = bridge
            .answer(&attached, "once", &AnsweredBy::Desktop)
            .expect("the seat is answered");
        assert_eq!(answered["answered"], "once");
        assert!(bridge.state(&attached)["approval"].is_null());
    }

    /// KI-286: a seat belongs to the session being driven, and a
    /// session-less answer — the shape a page that predates the seat's
    /// own name posts — belongs there too. Answering it against the
    /// face's launch session refused with `NoSeat` while the seat stood
    /// and the run hung behind it; a channel peer's run is where the two
    /// sessions diverge, and its seat shows on every surface.
    /// One-request chat-completions mock: reads the request to its
    /// declared length (the KI-280 lesson), then answers with a fixed
    /// completion.
    fn mock_chat() -> u16 {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("a mock");
        let port = listener.local_addr().expect("an address").port();
        std::thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buf = Vec::new();
                let mut chunk = [0u8; 4096];
                let head_end;
                loop {
                    match stream.read(&mut chunk) {
                        Ok(0) => return,
                        Ok(n) => buf.extend_from_slice(&chunk[..n]),
                        Err(_) => return,
                    }
                    if let Some(at) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                        head_end = at + 4;
                        break;
                    }
                }
                let head = String::from_utf8_lossy(&buf[..head_end]).to_ascii_lowercase();
                let length = head
                    .lines()
                    .find_map(|line| line.strip_prefix("content-length:"))
                    .and_then(|value| value.trim().parse::<usize>().ok())
                    .unwrap_or(0);
                while buf.len() < head_end + length {
                    match stream.read(&mut chunk) {
                        Ok(0) => break,
                        Ok(n) => buf.extend_from_slice(&chunk[..n]),
                        Err(_) => break,
                    }
                }
                // The streamed shape the router's walk parses.
                let chunk = serde_json::json!({
                    "choices": [{"delta": {"content": "the row opens the session in the conversation"}}]
                });
                let body = format!("data: {chunk}\n\ndata: [DONE]\n\n");
                let _ = write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
            }
        });
        port
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_side_ask_answers_and_the_runs_log_gains_nothing() {
        // C2: /btw is answered beside the run — one tool-less call — and
        // the session's log gains no message or turn from it.
        let (bridge, _sources, _door) = bridge().await;
        let home = bridge.home_path();
        let port = mock_chat();
        // Every workload walks the stand-in, so whichever the judgment
        // names, the aside reaches the mock.
        std::fs::write(
            home.join("config.json"),
            format!(
                r#"{{"providers":[{{"id":"stand-in","base_url":"http://127.0.0.1:{port}/v1","key_env":"STAND_IN_KEY","models":["auto"],"key_required":false}}]}}"#
            ),
        )
        .unwrap();
        std::fs::write(
            home.join("routing.json"),
            r#"{"workloads":[{"workload":"quick","candidates":["stand-in/auto"]},{"workload":"standard_code","candidates":["stand-in/auto"]},{"workload":"hard","candidates":["stand-in/auto"]},{"workload":"prose","candidates":["stand-in/auto"]},{"workload":"deep_context","candidates":["stand-in/auto"]}],"gates":{"default_workload":"standard_code"}}"#,
        )
        .unwrap();
        let mut session = crate::session::Session::create(&home, "m").expect("a log");
        let id = session.id().to_string();
        session
            .record(&crate::session::Record::End {
                stop_reason: "completed".to_string(),
                turns: 1,
                served_model: None,
            })
            .expect("an end");
        let before = crate::session::replay(session.path())
            .expect("the log")
            .len();

        let answered = bridge
            .btw(&id, "what does the picker's row action do?")
            .await
            .expect("the aside is answered");
        assert!(
            answered["answer"]
                .as_str()
                .unwrap_or_default()
                .contains("opens the session"),
            "{answered}"
        );
        assert_eq!(answered["provider"], "stand-in", "{answered}");
        assert_eq!(
            answered["question"],
            "what does the picker's row action do?"
        );

        let after = crate::session::replay(session.path()).expect("the log");
        assert_eq!(after.len(), before, "the log gained no record");
        assert!(
            !after
                .iter()
                .any(|record| format!("{record:?}").contains("row action")),
            "the aside's text is absent from the log"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_steer_with_no_run_writes_no_control_file() {
        // C1: a steer is about the run that is executing. With none in
        // flight it is refused by name, and nothing lands in a control
        // file the next run would read as its own.
        let (bridge, _sources, _door) = bridge().await;
        let error = bridge
            .stop(Some("check the gate first"))
            .expect_err("a steer with no run in flight");
        assert!(error.contains("no run is in flight"), "{error}");
        let session = bridge.launch();
        if !session.is_empty() {
            let control =
                crate::session::session_path(&bridge.home, &session).with_extension("control");
            assert!(!control.exists(), "no control file was written");
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_session_less_answer_answers_the_seat_being_driven() {
        let (bridge, sources, _door) = bridge().await;
        let driven = "20261021T000000Z-channelpeer";
        bridge.drive(driven);
        let shell = call("bash", r#"{"command":"echo hi"}"#);
        let (responder, reply) = std::sync::mpsc::channel();
        sources
            .seat
            .send(ApprovalRequest {
                call: shell,
                summary: "run echo hi".to_string(),
                responder,
            })
            .expect("the seat arrives");
        for _ in 0..50 {
            if bridge.state(driven)["approval"].is_object() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        assert!(
            bridge.state(driven)["approval"].is_object(),
            "the seat stands under the session being driven"
        );
        // No session named: the launch session has no seat, the driven
        // one does, and the answer belongs to the seat.
        let answered = bridge
            .answer("", "once", &AnsweredBy::Desktop)
            .expect("the session-less answer reaches the seat being driven");
        assert_eq!(answered["answered"], "once");
        assert!(bridge.state(driven)["approval"].is_null());
        assert!(
            reply.try_recv().is_ok(),
            "the approval the responder waited for crossed"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn an_answer_with_no_seat_is_a_conflict_not_a_denial() {
        let (bridge, _sources, _door) = bridge().await;
        let attached = bridge.launch();
        let answered = bridge.answer(&attached, "deny", &AnsweredBy::Desktop);
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
        let refused = bridge.answer(&attached, "sure", &AnsweredBy::Desktop);
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
        let answered = bridge
            .answer(&attached, "remember", &AnsweredBy::Desktop)
            .expect("the rule is remembered");
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
            .answer(&attached, "remember", &AnsweredBy::Desktop)
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

    #[test]
    fn the_routed_event_carries_the_execution_head_or_names_its_absence() {
        // C4's bridge half: the `routed` event carries the execution
        // head when the decision has one, and null when it does not —
        // the page renders both from the same event, and an answer
        // without the head never invents a label.
        let base = |execution: Option<crate::route::ExecutionHead>| UiEvent::Routed {
            provider: "stepfun".to_string(),
            model: "step-5-preview".to_string(),
            how: "judged".to_string(),
            workload: Some("hard".to_string()),
            verdict: Some("pass".to_string()),
            judged_from: "fresh".to_string(),
            reason: "sys1 judged `hard`".to_string(),
            order: vec![("stepfun".to_string(), "step-5-preview".to_string())],
            execution,
        };
        let rendered = crate::gui::bridge::describe(&base(None));
        assert_eq!(rendered["execution"], serde_json::Value::Null);
        let rendered = crate::gui::bridge::describe(&base(Some(crate::route::ExecutionHead {
            label: "swarm".to_string(),
            confidence: 0.91,
        })));
        assert_eq!(rendered["execution"]["label"], "swarm");
        assert_eq!(rendered["execution"]["confidence"], 0.91);
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
                execution: Some(crate::route::ExecutionHead {
                    label: "mixture".to_string(),
                    confidence: 0.86,
                }),
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
        // The execution head rides the event whole (C4): the live line
        // and the transcript's route strip name both heads from the
        // same facts.
        assert_eq!(events[0]["execution"]["label"], "mixture");
        assert_eq!(events[0]["execution"]["confidence"], 0.86);
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
        // The dotenv family is refused by name, with the reason the pane
        // shows — `.envrc` beside `.env` and the dotted variants
        // (KI-238).
        for spelling in [".env", ".env.local", ".envrc"] {
            let refused = bridge.file(spelling).await;
            assert_eq!(
                refused["refused"][0], "a credential file is never served",
                "{spelling}"
            );
        }
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
        let mut log = Session::create(&home, "deepseek-flash").expect("the log");
        let id = log.id().to_string();
        log.record(&session::Record::User {
            content: "carry me over".to_string(),
        })
        .expect("the record");
        let (mut bridge, _sources) = Bridge::detached(
            home,
            "deepseek-flash".to_string(),
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
            launch["events"]
                .as_array()
                .is_none_or(|events| events.is_empty()),
            "the launch session kept its own log: {launch}"
        );
    }

    /// Two surfaces race to answer one seat: the first answer lands, the
    /// second finds no seat, and the resolution is broadcast naming who
    /// answered — so the surface that lost does not leave a button
    /// standing for a decision already made.
    #[tokio::test(flavor = "multi_thread")]
    async fn the_first_answer_wins_and_the_resolution_names_who_answered() {
        let (bridge, sources, _door) = bridge().await;
        let attached = bridge.launch();
        let shell = call("bash", r#"{"command":"cargo test"}"#);
        let (responder, reply) = std::sync::mpsc::channel();
        sources
            .seat
            .send(ApprovalRequest {
                call: shell,
                summary: "run cargo test".to_string(),
                responder,
            })
            .expect("the seat arrives");
        for _ in 0..50 {
            if bridge.state(&attached)["approval"].is_object() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        assert!(
            bridge.state(&attached)["approval"].is_object(),
            "the seat is standing"
        );

        // The desktop answers first.
        let won = bridge
            .answer(&attached, "once", &AnsweredBy::Desktop)
            .expect("the first answer lands");
        assert_eq!(won["answered"], "once");

        // The phone answers second, and finds no seat.
        let lost = bridge.answer(
            &attached,
            "deny",
            &AnsweredBy::Device("Alejandro's iPhone".to_string()),
        );
        assert!(
            matches!(lost, Err(AnswerError::NoSeat)),
            "the second answer found a seat: {lost:?}"
        );

        // The agent's responder got the first decision, and only it.
        let delivered = reply.recv().expect("a decision arrived");
        assert_eq!(delivered, Approval::Once);

        // The seat is gone from the session's state.
        assert!(bridge.state(&attached)["approval"].is_null());
    }

    /// The access document names who may reach this face and never what
    /// would let them, and the revocation forgets one through the store's
    /// own save — the same one the CLI performs.
    #[tokio::test(flavor = "multi_thread")]
    async fn the_access_document_names_the_paired_and_the_revocation_forgets_one() {
        let home =
            std::env::temp_dir().join(format!("fleet-gui-bridge-access-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).expect("a temp home");
        // A paired device and a paired channel peer.
        let mut devices = crate::remote::pairing::DeviceStore::default();
        let device_token = "device-token-abc".to_string();
        devices.add("Alejandro's iPhone", &device_token, "2026-10-18T10:00:00Z");
        devices.save(&home).unwrap();
        let mut peers = crate::channels::PeerStore::default();
        peers.pair(
            "abc",
            "the CI box",
            "peer-token-xyz",
            crate::channels::Capability::Task,
            "2026-10-18T11:00:00Z",
        );
        peers.save(&home, "webhook").unwrap();

        let (bridge, _) = Bridge::detached(
            home.clone(),
            "deepseek-flash".to_string(),
            Some("session-test".to_string()),
        );
        let document = bridge.access().await;
        assert_eq!(document["devices"].as_array().map(Vec::len), Some(1));
        assert_eq!(document["peers"].as_array().map(Vec::len), Some(1));
        assert_eq!(document["devices"][0]["name"], "Alejandro's iPhone");
        assert_eq!(document["peers"][0]["name"], "the CI box");
        // The masking rule: names and stamps, never a credential.
        let text = document.to_string();
        assert!(
            !text.contains(&device_token),
            "the device token rode: {text}"
        );
        assert!(
            !text.contains("peer-token-xyz"),
            "the peer token rode: {text}"
        );
        assert!(!text.contains("token_hash"), "a digest rode: {text}");

        // The revocation goes through the store's own save, and the
        // device's token stops working.
        let id = document["devices"][0]["id"].as_str().unwrap().to_string();
        let revoked = bridge.access_revoke("device", &id).await.expect("revoked");
        assert_eq!(revoked["kind"], "device");
        let reloaded = crate::remote::pairing::DeviceStore::load(&home).unwrap();
        assert!(reloaded.device_for_token(&device_token).is_none());
        // A name that is not there is refused by naming itself.
        assert!(bridge.access_revoke("device", "nobody").await.is_err());
        // And a kind that is not a thing is refused for its shape.
        assert!(bridge.access_revoke("settings", "x").await.is_err());
        std::fs::remove_dir_all(&home).ok();
    }

    /// Two sessions attached to one bridge hold independent transcripts,
    /// running flags, and seats: a phone and a desktop looking at two
    /// conversations do not disturb each other.
    #[tokio::test(flavor = "multi_thread")]
    async fn two_attached_sessions_are_independent() {
        let home =
            std::env::temp_dir().join(format!("fleet-gui-bridge-two-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).expect("a temp home");
        let (mut bridge, _sources) = Bridge::detached(
            home.clone(),
            "deepseek-flash".to_string(),
            Some("session-one".to_string()),
        );
        // The door stands open with no thread behind it, so the commands
        // are accepted and nothing runs: this test is about the state,
        // not the agent.
        let _door = bridge.connect();
        let one = bridge.launch();
        let two = "session-two".to_string();

        // A run starts in one; the other session's flag does not move.
        bridge.run(&one, "work in one").expect("accepted");
        // No thread stands behind this detached bridge, so the test
        // names the driving session the way the thread would.
        bridge.drive(&one);
        assert_eq!(bridge.state(&one)["running"], true);
        // Both documents name the session this face is executing, so a
        // surface reading two knows a run is working in the other one
        // rather than inferring it from a process-global flag (KI-283's
        // shape, one field over).
        assert_eq!(bridge.state(&one)["driving"], one.as_str());
        assert_eq!(
            bridge.state(&two)["driving"],
            one.as_str(),
            "the other conversation is told where the run is"
        );
        assert_eq!(bridge.state(&two)["running"], false, "{two} is not running");
        assert_eq!(
            bridge.state(&two)["attached"],
            false,
            "{two} was never opened"
        );

        // The owner's message landed in one's log and not in two's.
        let events = bridge.state(&one)["events"]
            .as_array()
            .expect("a log")
            .len();
        assert_eq!(events, 1, "one event in one's log");
        assert_eq!(
            bridge.state(&two)["events"]
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
        assert!(
            second.is_ok(),
            "the command is accepted; the thread is serial, not blocked: {second:?}"
        );
        assert_eq!(bridge.state(&one)["running"], true);
        assert_eq!(bridge.state(&two)["running"], true);

        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn the_attach_decision_is_local_ready_or_a_question() {
        let cwd = std::env::current_dir().expect("the cwd");
        match attach_local(&cwd, "services/documents/fixtures/simple.docx", false)
            .expect("the local decision")
        {
            Attachment::Ready(value) => {
                assert_eq!(value["type"], "attachment");
                assert_eq!(value["parser"], "local:docx");
            }
            other => panic!("{other:?}"),
        }
        match attach_local(&cwd, "services/documents/fixtures/blank.pdf", false)
            .expect("the question")
        {
            Attachment::Needs(value) => {
                assert_eq!(value["type"], "attach_needs");
                assert!(
                    value["reason"].as_str().unwrap_or("").contains("scan"),
                    "{value}"
                );
            }
            other => panic!("{other:?}"),
        }
        match attach_local(&cwd, "services/documents/fixtures/pixel.png", true)
            .expect("the escalation")
        {
            Attachment::Escalate { name, .. } => assert_eq!(name, "pixel.png"),
            other => panic!("{other:?}"),
        }
    }

    // --- work and the conversation: the row's state, class, and job ---

    /// One summary row for the derivation tests.
    fn row(
        id: &str,
        kind: Option<&str>,
        channel: Option<(&str, &str, &str)>,
        ended: bool,
        stop_reason: Option<&str>,
        first_text: Option<&str>,
        last_write: Option<u64>,
    ) -> crate::session::SessionSummary {
        crate::session::SessionSummary {
            id: id.to_string(),
            model: "deepseek-flash".to_string(),
            first_text: first_text.map(str::to_string),
            created: "2026-10-23T00:00:00Z".to_string(),
            ended,
            kind: kind.map(str::to_string),
            workspace: None,
            workspace_inferred: false,
            channel: channel.map(|(a, b, c)| (a.to_string(), b.to_string(), c.to_string())),
            compacted: false,
            served_model: None,
            stop_reason: stop_reason.map(str::to_string),
            turns: ended.then_some(3),
            last_write,
            path: std::path::PathBuf::from(format!("/tmp/{id}.jsonl")),
        }
    }

    fn now_secs() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|age| age.as_secs())
            .unwrap_or_default()
    }

    #[test]
    fn the_work_state_names_what_needs_the_owner_first() {
        let live = LiveFace::default();
        let done = row(
            "a",
            None,
            None,
            true,
            Some("completed"),
            Some("t"),
            Some(now_secs()),
        );
        assert_eq!(work_state(&done, &live, false), "completed");
        let capped = row(
            "b",
            None,
            None,
            true,
            Some("max-turns"),
            Some("t"),
            Some(now_secs()),
        );
        assert_eq!(work_state(&capped, &live, false), "capped");
        let stopped = row(
            "c",
            None,
            None,
            true,
            Some("stopped by the owner"),
            Some("t"),
            Some(now_secs()),
        );
        assert_eq!(work_state(&stopped, &live, false), "stopped");
        // Unfinished and recently written is active; stale is interrupted.
        let fresh = row("d", None, None, false, None, Some("t"), Some(now_secs()));
        assert_eq!(work_state(&fresh, &live, false), "active");
        let stale = row("e", None, None, false, None, Some("t"), Some(1));
        assert_eq!(work_state(&stale, &live, false), "interrupted");
        // A log with no owner message is a conversation created and unused.
        let created = row("f", None, None, false, None, None, Some(now_secs()));
        assert_eq!(work_state(&created, &live, false), "new");
        // The owner's pause outranks the log; an approval outranks both.
        assert_eq!(work_state(&fresh, &live, true), "paused");
        let mut seated = LiveFace::default();
        seated.seats.insert("d".to_string());
        assert_eq!(work_state(&fresh, &seated, true), "needs-approval");
        // A run this face drives is working — but the log's own end wins
        // over a stale flag for the instant between.
        let driving = LiveFace {
            driving: Some("d".to_string()),
            seats: Default::default(),
        };
        assert_eq!(work_state(&fresh, &driving, false), "working");
        assert_eq!(work_state(&capped, &driving, false), "capped");
    }

    #[test]
    fn the_class_and_the_job_come_from_the_log_shape() {
        let scheduled = row(
            "s",
            Some(crate::session::SCHEDULED_KIND),
            None,
            true,
            Some("completed"),
            Some("[scheduled run \u{2014} job `corpus-refresh-fleet` at 2026-10-23T00:10Z]"),
            Some(1),
        );
        assert_eq!(work_class(&scheduled), "background");
        assert_eq!(
            scheduled_job(&scheduled).as_deref(),
            Some("corpus-refresh-fleet")
        );
        // The catch-up frame names its job the same way.
        let catch_up = row(
            "c",
            Some(crate::session::SCHEDULED_KIND),
            None,
            false,
            None,
            Some("[scheduled run \u{2014} catch-up for job `sem1-refresh`: the 2026-10-22T00:25Z window was missed while fleet was down]"),
            Some(1),
        );
        assert_eq!(scheduled_job(&catch_up).as_deref(), Some("sem1-refresh"));
        let child = row(
            "k",
            Some(crate::session::SUBAGENT_KIND),
            None,
            true,
            Some("completed"),
            Some("t"),
            Some(1),
        );
        assert_eq!(work_class(&child), "delegated");
        assert!(scheduled_job(&child).is_none());
        let peer = row(
            "p",
            None,
            Some(("telegram", "42", "owner")),
            false,
            None,
            Some("t"),
            Some(1),
        );
        assert_eq!(work_class(&peer), "channel");
        let mine = row("m", None, None, true, Some("completed"), Some("t"), Some(1));
        assert_eq!(work_class(&mine), "yours");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn sessions_names_the_paused_run_the_capped_one_and_the_driven_one() {
        let home = std::env::temp_dir().join(format!("fleet-gui-work-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).expect("a temp home");
        let mut ended = crate::session::Session::create(&home, "deepseek-flash").unwrap();
        ended
            .record(&crate::session::Record::User {
                content: "the task".into(),
            })
            .unwrap();
        ended
            .record(&crate::session::Record::End {
                stop_reason: "max-turns".into(),
                turns: 8,
                served_model: None,
            })
            .unwrap();
        let ended_id = ended.id().to_string();
        let mut paused = crate::session::Session::create(&home, "deepseek-flash").unwrap();
        paused
            .record(&crate::session::Record::User {
                content: "hold this".into(),
            })
            .unwrap();
        let paused_id = paused.id().to_string();
        std::fs::write(
            crate::home::sessions_dir(&home).join(format!("{paused_id}.control")),
            "{\"pause\": true}",
        )
        .unwrap();
        let mut working = crate::session::Session::create(&home, "deepseek-flash").unwrap();
        working
            .record(&crate::session::Record::User {
                content: "still going".into(),
            })
            .unwrap();
        let working_id = working.id().to_string();

        let live = LiveFace {
            driving: Some(working_id.clone()),
            seats: Default::default(),
        };
        let document = sessions_read(&home, "", &live, "");
        let rows = document["sessions"].as_array().expect("rows");
        let row = |id: &str| {
            rows.iter()
                .find(|row| row["id"] == id)
                .cloned()
                .unwrap_or_else(|| panic!("{id} in {document}"))
        };
        let capped = row(&ended_id);
        assert_eq!(capped["state"], "capped");
        assert_eq!(capped["turns"], 8);
        assert_eq!(capped["stop_reason"], "max-turns");
        assert_eq!(capped["class"], "yours");
        assert!(capped["last_write"].as_u64().is_some(), "{capped}");
        assert_eq!(row(&paused_id)["state"], "paused");
        assert_eq!(row(&working_id)["state"], "working");
        std::fs::remove_dir_all(&home).ok();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_new_conversation_is_a_real_log_and_the_archive_shows_it() {
        let home = std::env::temp_dir().join(format!("fleet-gui-new-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).expect("a temp home");
        let (mut bridge, _sources) =
            Bridge::detached(home.clone(), "deepseek-flash".to_string(), None);
        // The door stands open with no thread behind it: the command is
        // accepted and the owner's message is recorded, which is the
        // record this test is about.
        let _door = bridge.connect();
        let first = bridge.new_session().expect("created");
        let second = bridge.new_session().expect("created again");
        assert_ne!(first, second);
        let dir = crate::home::sessions_dir(&home);
        assert!(dir.join(format!("{first}.jsonl")).exists(), "{first}");
        assert!(dir.join(format!("{second}.jsonl")).exists(), "{second}");
        // Two logs, each with its own start record and nothing else.
        for id in [&first, &second] {
            let records =
                crate::session::replay(&dir.join(format!("{id}.jsonl"))).expect("replays");
            assert_eq!(records.len(), 1, "{id}: {records:?}");
            assert!(matches!(records[0], crate::session::Record::Start { .. }));
        }
        // A run posted to the new conversation lands in ITS session: the
        // route names the session, and the bridge records the owner's
        // message as that session's first event (the thread writes it to
        // the log when it picks the run up; the log line is the live
        // pass's evidence, the event is this seam's).
        bridge
            .run(&second, "work in the new conversation")
            .expect("accepted");
        let second_events = bridge.state(&second)["events"]
            .as_array()
            .cloned()
            .expect("the new session's log");
        assert!(
            second_events.iter().any(|event| {
                event["type"] == "message"
                    && event["role"] == "owner"
                    && event["text"] == "work in the new conversation"
            }),
            "the run's message landed in the new session: {second_events:?}"
        );
        assert_eq!(
            bridge.state(&first)["events"]
                .as_array()
                .map(Vec::len)
                .unwrap_or(0),
            0,
            "the conversation the run was not posted to gained nothing"
        );
        // Both logs still hold their one start record: the run's own
        // records are the thread's to write, and this face has none.
        for id in [&first, &second] {
            let records =
                crate::session::replay(&dir.join(format!("{id}.jsonl"))).expect("replays");
            assert_eq!(records.len(), 1, "{id}: {records:?}");
        }
        // The archive shows both as conversations (one now carrying its
        // first message).
        let document = bridge.sessions("", "").await;
        let rows = document["sessions"].as_array().expect("rows");
        for id in [&first, &second] {
            let row = rows
                .iter()
                .find(|row| row["id"] == id.as_str())
                .unwrap_or_else(|| panic!("{id} in {document}"));
            assert_eq!(row["state"], "new", "{row}");
            assert_eq!(row["class"], "yours", "{row}");
        }
        std::fs::remove_dir_all(&home).ok();
    }

    // KI-301: a run's end clears the session it was driving, at the one
    // seam every event crosses — so the next send in any conversation is
    // accepted instead of queueing behind work that is over.
    #[tokio::test(flavor = "multi_thread")]
    async fn a_finished_run_stops_driving_the_face() {
        let home = std::env::temp_dir().join(format!("fleet-gui-finish-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).expect("a temp home");
        let (bridge, _sources) = Bridge::detached(
            home.clone(),
            "deepseek-flash".to_string(),
            Some("one".to_string()),
        );
        bridge.drive("one");
        assert_eq!(bridge.state("one")["driving"], "one");
        assert_eq!(
            bridge.state("two")["driving"],
            "one",
            "both documents agree"
        );
        record(
            &bridge.state,
            &bridge.events,
            "one",
            json!({"type": "finished", "reason": "stop"}),
        );
        assert!(
            bridge.state("one")["driving"].is_null(),
            "the finished run still drives: {}",
            bridge.state("one")["driving"]
        );
        assert!(
            bridge.state("two")["driving"].is_null(),
            "the other conversation is told too"
        );
        std::fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn a_reasoning_record_replays_as_a_thought_card() {
        let records = vec![session::Record::Reasoning {
            text: "count the rows".to_string(),
            seconds: 5,
        }];
        let (cards, omitted) = cards_from_records(&records);
        assert_eq!(omitted, 0, "{cards:?}");
        assert_eq!(cards[0]["type"], "reasoning", "{cards:?}");
        assert_eq!(cards[0]["text"], "count the rows");
        assert_eq!(cards[0]["seconds"], 5);
    }

    #[test]
    fn a_replayed_call_carries_the_command_it_ran() {
        let records = vec![session::Record::Assistant {
            content: Some("running it".to_string()),
            tool_calls: vec![call("bash", "{\"command\":\"echo hi\"}")],
        }];
        let (cards, _) = cards_from_records(&records);
        assert_eq!(cards[0]["type"], "message", "{cards:?}");
        assert_eq!(cards[1]["type"], "tool_call", "{cards:?}");
        assert_eq!(cards[1]["tool"], "bash");
        assert_eq!(cards[1]["summary"], "echo hi");
        // The full call, not the clipped line: the face's expanded row
        // names what ran from this.
        assert_eq!(cards[1]["arguments"], "{\"command\":\"echo hi\"}");
    }

    #[test]
    fn every_recorded_event_names_its_session() {
        let state = Mutex::new(GuiState {
            pending_replay: None,
            sessions: Default::default(),
            launch: Some("s".to_string()),
            driving: Some("s".to_string()),
            permissions: crate::approvals::Mode::Ask,
            provider_pin: None,
            turn_budget: 0,
            workdirs: Default::default(),
        });
        let (events, mut seen) = broadcast::channel(4);
        record(&state, &events, "s", json!({"type": "token", "text": "hi"}));
        let logged = {
            let state = lock(&state);
            state.sessions["s"]
                .events
                .back()
                .cloned()
                .expect("one event logged")
        };
        assert_eq!(logged["session"], "s", "{logged}");
        let streamed = seen.try_recv().expect("one event broadcast");
        assert_eq!(streamed["session"], "s", "{streamed}");
        // A face whose launch log does not exist yet has no id to name.
        let empty = Mutex::new(GuiState {
            pending_replay: None,
            sessions: Default::default(),
            launch: None,
            driving: None,
            permissions: crate::approvals::Mode::Ask,
            provider_pin: None,
            turn_budget: 0,
            workdirs: Default::default(),
        });
        record(&empty, &events, "", json!({"type": "token", "text": "hi"}));
        let untagged = {
            let state = lock(&empty);
            state.sessions[""]
                .events
                .back()
                .cloned()
                .expect("one event logged")
        };
        assert!(untagged.get("session").is_none(), "{untagged}");
    }
}
