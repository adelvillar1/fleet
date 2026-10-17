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
    /// The corpus root the consolidator walks — the same root the
    /// browser's memory panel reads, resolved once by the bridge.
    memories_root: PathBuf,
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
            memories: crate::memory::resolve_root(&std::env::current_dir().unwrap_or_default()),
            cwd: std::env::current_dir().unwrap_or_default(),
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

    /// Point the memory panel at a corpus other than the one the working
    /// directory resolves. The production path never calls this — the
    /// rule is the agent's own, resolved once in [`Bridge::detached`] —
    /// so this exists for a test that must read a corpus it built rather
    /// than the machine's.
    #[cfg(test)]
    pub(crate) fn point_at_memories(&mut self, root: PathBuf) {
        self.memories = root;
    }

    /// A bridge with the agent thread behind it: the thread this module
    /// spawns is the same shape `tui::spawn_agent` spawns, with the
    /// browser's prompt line and the seat's approver.
    pub fn spawn(home: PathBuf, model: String, session: String, base_url: String) -> Bridge {
        let (mut bridge, sources) = Bridge::detached(home.clone(), model.clone(), session.clone());
        let command_rx = bridge.connect();
        // One resolution, shared: the consolidator the thread starts and
        // the browser's memory panel read the same corpus because they
        // are handed the same root, not because they each walked for one.
        let memories = bridge.memories.clone();
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
        let endpoint = crate::memory::embedding_endpoint();
        let ranked = tokio::task::spawn_blocking(move || {
            let embedder = crate::memory::probe(&endpoint).embedder();
            let Some(query) = embedder.embed(&asked) else {
                return Vec::new();
            };
            candidates
                .iter()
                .enumerate()
                .map(|(index, text)| {
                    let score = embedder
                        .embed(text)
                        .map(|vector| crate::memory::cosine(&query, &vector))
                        .unwrap_or(0.0);
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
        UiEvent::Deck {
            project,
            nodes,
            html,
        } => json!({
            "type": "deck",
            "project": project,
            "nodes": nodes,
            "html": html,
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
            memories_root,
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
        sources
            .ui
            .send(UiEvent::Deck {
                project: "presentation".to_string(),
                nodes: 2,
                html: "<section class=\"deck\" data-type=\"presentation\">x</section>".to_string(),
            })
            .expect("the deck lands");
        // The fan-out is a task of the caller's, so the event needs a
        // moment to cross it.
        for _ in 0..50 {
            let events = bridge.state()["events"]
                .as_array()
                .map(Vec::len)
                .unwrap_or(0);
            if events > 0 {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        let document = bridge.state();
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
