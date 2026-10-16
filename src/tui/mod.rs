//! The TUI: a ratatui surface over the async agent core.
//!
//! Threading: this module runs the crossterm event loop and the rendering
//! on the caller's thread; the agent runs on a dedicated thread with its
//! own tokio runtime, so no network or tool work ever blocks the
//! interface. The two sides meet at four channels:
//!
//! - `UiEvent`s (agent → UI): streamed tokens, turns, tool calls, results.
//! - `Command`s (UI → agent): run this task, resume that session.
//! - `ApprovalRequest`s (agent → UI): one prompt, one answer back.
//! - `StatusUpdate`s (agent → UI): exact token accounting per run.
//!
//! The session log is written exactly as the headless path writes it: the
//! TUI changes what the user sees, never what is recorded.

pub mod app;
pub mod events;
pub mod ui;

use std::io;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

use ratatui::crossterm::event::{Event, KeyEvent};
use ratatui::Terminal;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::agent::{
    Agent, AgentOptions, ApprovalRequest, Approver, TuiApprover, UiEvent, DEFAULT_MAX_TURNS,
};
use crate::api::Client;
use crate::boards::{self, BoardUpdate};
use crate::lanes::{self, LaneHealth};
use crate::route;
use crate::session::{self, Session};

use app::{Action, App, Effect, StatusUpdate};
use events::action_for;

/// How long the event loop waits for a key before redrawing. The agent's
/// channels are drained once per tick — they wake no waiter of their own
/// — so a token delta is at most one tick behind the frame that shows it.
const POLL: Duration = Duration::from_millis(100);

/// What the TUI needs resolved before it starts.
pub struct Config {
    /// The fleet home that owns the sessions directory and the routing
    /// config. Credentials are resolved per run by the router, so no single
    /// key is required up front.
    pub home: PathBuf,
    /// The model to run.
    pub model: String,
}

/// One instruction from the interface to the agent thread.
enum Command {
    /// Run this task in the current session.
    Run(String),
    /// Append the next run to this session's log.
    Resume(String),
}

/// The agent thread's inputs.
struct AgentConfig {
    home: PathBuf,
    model: String,
    session_id: String,
    ui_tx: UnboundedSender<UiEvent>,
    status_tx: UnboundedSender<StatusUpdate>,
    approval_tx: UnboundedSender<ApprovalRequest>,
    command_rx: UnboundedReceiver<Command>,
}

/// Restores the terminal when the TUI ends, including on a panic, so the
/// shell is never left in raw mode, on the alternate screen, or with
/// bracketed paste still armed.
struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        // Paste mode goes off before the screen is restored: a shell
        // left with bracketed paste on mangles every later command line.
        let _ = ratatui::crossterm::execute!(
            io::stdout(),
            ratatui::crossterm::event::DisableBracketedPaste
        );
        let _ = ratatui::try_restore();
    }
}

/// The terminal the TUI draws on.
type UiTerminal = Terminal<ratatui::backend::CrosstermBackend<io::Stdout>>;

/// Run the TUI until the user quits. Errors are startup failures, reported
/// with the terminal already restored.
pub fn run(config: Config) -> Result<(), String> {
    // The guard outlives the terminal it protects, so any exit path —
    // orderly, error, or panic — restores the shell.
    let mut terminal =
        ratatui::try_init().map_err(|error| format!("the terminal could not start: {error}"))?;
    let _guard = TerminalGuard;

    // `try_init` arms raw mode and the alternate screen but not bracketed
    // paste; without it a multi-line paste delivers its newlines as
    // Enter and submits the lines one by one.
    ratatui::crossterm::execute!(
        io::stdout(),
        ratatui::crossterm::event::EnableBracketedPaste
    )
    .map_err(|error| format!("the terminal could not start: {error}"))?;

    let session_id = create_session(&config.home, &config.model)?;
    let (ui_tx, ui_rx) = tokio::sync::mpsc::unbounded_channel::<UiEvent>();
    let (status_tx, status_rx) = tokio::sync::mpsc::unbounded_channel::<StatusUpdate>();
    let (approval_tx, approval_rx) = tokio::sync::mpsc::unbounded_channel::<ApprovalRequest>();
    let (command_tx, command_rx) = tokio::sync::mpsc::unbounded_channel::<Command>();

    let agent = spawn_agent(AgentConfig {
        home: config.home.clone(),
        model: config.model.clone(),
        session_id: session_id.clone(),
        ui_tx,
        status_tx,
        approval_tx,
        command_rx,
    });

    let mut app = App::new(&config.model, &session_id);
    let (lanes_tx, lanes_rx) = tokio::sync::mpsc::unbounded_channel::<(u64, Vec<LaneHealth>)>();
    // The boards: their sources resolved once, exactly the way every
    // credential in this crate resolves (the environment, then the fleet
    // home's `.env`), and one refresher thread that reads all three on
    // its own cadence. The interface only ever receives finished reads.
    let dotenv = crate::home::read_dotenv(&config.home);
    let (boards_tx, boards_rx) =
        boards::spawn_refresher(boards::resolve_sources(&config.home, &dotenv));
    let outcome = event_loop(
        &config.home,
        &mut app,
        &mut terminal,
        ui_rx,
        status_rx,
        approval_rx,
        &command_tx,
        lanes_rx,
        &lanes_tx,
        boards_rx,
        &boards_tx,
    );
    // The agent thread is detached: it ends between runs once the command
    // channel closes, and the process's exit ends it mid-run, which leaves
    // the documented interrupted-log shape (a log without its `end`).
    drop(command_tx);
    drop(agent);
    outcome
}

/// Create the launch session and return its id.
fn create_session(home: &Path, model: &str) -> Result<String, String> {
    let session = Session::create(home, model)
        .map_err(|error| format!("could not create the session: {error}"))?;
    Ok(session.id().to_string())
}

/// The event loop: drain the agent's channels, read the keyboard, render.
/// The loop never blocks on the network; `poll` only ever waits on input.
///
/// The parameters are the four channels the agent thread and the UI meet
/// at, plus the surfaces they feed — they are the wiring itself, and a
/// struct would bundle them without reducing what the loop is handed.
#[allow(clippy::too_many_arguments)]
fn event_loop(
    home: &Path,
    app: &mut App,
    terminal: &mut UiTerminal,
    mut ui_rx: UnboundedReceiver<UiEvent>,
    mut status_rx: UnboundedReceiver<StatusUpdate>,
    mut approval_rx: UnboundedReceiver<ApprovalRequest>,
    command_tx: &UnboundedSender<Command>,
    mut lanes_rx: UnboundedReceiver<(u64, Vec<LaneHealth>)>,
    lanes_tx: &UnboundedSender<(u64, Vec<LaneHealth>)>,
    mut boards_rx: UnboundedReceiver<BoardUpdate>,
    boards_tx: &UnboundedSender<boards::BoardCommand>,
) -> Result<(), String> {
    let mut pending: Option<ApprovalRequest> = None;
    while !app.should_quit {
        while let Ok(event) = ui_rx.try_recv() {
            app.apply_ui_event(event);
        }
        while let Ok(update) = status_rx.try_recv() {
            app.apply_status(update);
        }
        while let Ok(request) = approval_rx.try_recv() {
            app.show_approval(&request.call.function.name, &request.summary);
            pending = Some(request);
        }
        // Lane health lands asynchronously from the prober thread. Which
        // rows belong to the open view is the state machine's call — the
        // generation stamped at `open_lanes` — so the loop only forwards;
        // it never guesses from the mode, which cannot tell a fresh
        // modal's rows from a previous probe's.
        while let Ok((probe, rows)) = lanes_rx.try_recv() {
            app.set_lanes(probe, rows);
        }
        // The boards' reads land here: a landed snapshot replaces the last
        // one, a refusal keeps it and names the reason. Both are already
        // bounded, so the loop never waits on a source.
        while let Ok(update) = boards_rx.try_recv() {
            app.apply_board(update.kind, update.outcome);
        }

        if ratatui::crossterm::event::poll(POLL)
            .map_err(|error| format!("the keyboard could not be read: {error}"))?
        {
            match ratatui::crossterm::event::read() {
                Ok(Event::Key(key)) => dispatch_key(
                    home,
                    app,
                    &key,
                    command_tx,
                    lanes_tx,
                    boards_tx,
                    &mut pending,
                ),
                // A paste is typing: the text lands in the input box with
                // its newlines intact, never submitted line by line.
                Ok(Event::Paste(text)) => app.paste(&text),
                _ => {}
            }
        }

        terminal
            .draw(|frame| ui::render(frame, app))
            .map_err(|error| format!("the frame could not be drawn: {error}"))?;
    }
    Ok(())
}

/// Translate one key press into state changes and the effects the loop
/// performs around them.
fn dispatch_key(
    home: &Path,
    app: &mut App,
    key: &KeyEvent,
    command_tx: &UnboundedSender<Command>,
    lanes_tx: &UnboundedSender<(u64, Vec<LaneHealth>)>,
    boards_tx: &UnboundedSender<boards::BoardCommand>,
    pending: &mut Option<ApprovalRequest>,
) {
    let Some(action) = action_for(app, key) else {
        return;
    };
    // The cycle asks for a read now: the cadence already covers every
    // board, and the one the user just turned to should not wait for it.
    let cycling = matches!(action, Action::CycleBoard);
    // The picker's rows are filesystem reads, so the loop loads them and
    // hands them to the state machine.
    if matches!(action, Action::OpenSessions) {
        open_picker(home, app);
        return;
    }
    // The lanes' rows are child-process probes, so the loop hands the
    // state machine an open view and starts the prober.
    if matches!(action, Action::OpenLanes) {
        open_lanes_view(app, lanes_tx);
        return;
    }
    // The settings pane reads the home's files and writes through the
    // same writer the CLI calls, so the loop hands the state machine the
    // home and the pane does the rest. No lane call: a setting is a file
    // read, never an ask.
    if matches!(action, Action::OpenSettings) {
        app.open_settings(home.to_path_buf());
        return;
    }
    // The explanation card is a read of the beat's own artifact — the
    // file its row indexes — so the loop reads it and hands the state
    // machine the lines. No lane call: the card is what was already
    // spent, never a new ask.
    if matches!(action, Action::OpenExplain) {
        open_explanation(app, home);
        return;
    }
    if let Some(effect) = app.apply(action) {
        apply_effect(home, app, effect, command_tx, pending);
    }
    if cycling {
        let _ = boards_tx.send(boards::BoardCommand::Refresh);
    }
}

/// Load the picker's rows and open it over the conversation. A failed
/// read is an error, not an empty picker: "no sessions yet" would be a
/// lie about a directory the interface simply could not read.
fn open_picker(home: &Path, app: &mut App) {
    match session::list(home) {
        Ok(sessions) => app.open_sessions(sessions),
        Err(error) => {
            app.error = Some(format!("could not list the sessions: {error}"));
        }
    }
}

/// Open the latest explanation card over the conversation. A day with
/// no explanation says so rather than rendering an empty card, and a
/// home the read cannot reach names the reason — the same named
/// refusals the headless surface carries.
///
/// This is the one read-only reach into the explanation beat the
/// socket-guard test allows: it reads the beat's own artifact (the
/// file its row indexes) and hands the state machine its lines. It
/// never constructs a row, never makes an ask, and never runs on a
/// synchronous per-turn path — the modal opens on an explicit Ctrl+X.
fn open_explanation(app: &mut App, home: &Path) {
    let user_home = std::env::var("HOME").ok();
    let root = crate::calib::resolve_root(home, user_home.as_deref());
    match crate::explain::latest_card(&root, home) {
        // The card rides ANSI escapes for the terminal; the modal's
        // width budget must be spent on visible columns only, so the
        // escapes are stripped before the lines enter the state machine
        // — a named reason cut in half by its own color codes is not a
        // named one (KI-140's class).
        Some(card) => {
            let lines = crate::eval::strip_ansi(&card)
                .lines()
                .map(str::to_string)
                .collect();
            app.open_explain(lines);
        }
        None => app.error = Some(
            "no explanation recorded today yet \u{b7} `fleet explain --proposal <name>` writes one"
                .to_string(),
        ),
    }
}

/// Open the lanes view and start the prober. The probe runs on its own
/// thread with its own current-thread runtime — the event-loop thread
/// never spawns children or waits on them, and the prober is not inside
/// a runtime context, so nothing here can block-in-runtime panic
/// (KI-021's class). The generation `open_lanes` stamped rides with the
/// rows, so a probe the user has already closed cannot land in the view
/// that replaced it.
fn open_lanes_view(app: &mut App, lanes_tx: &UnboundedSender<(u64, Vec<LaneHealth>)>) {
    let generation = app.open_lanes();
    let lanes_tx = lanes_tx.clone();
    thread::spawn(move || {
        let runtime = match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(runtime) => runtime,
            Err(error) => {
                let _ = lanes_tx.send((
                    generation,
                    lanes::unavailable_all(&format!(
                        "the lane probe runtime failed to start: {error}"
                    )),
                ));
                return;
            }
        };
        let health = runtime.block_on(lanes::health_all());
        let _ = lanes_tx.send((generation, health));
    });
}

/// Perform what a state change cannot do from inside the state machine.
fn apply_effect(
    home: &Path,
    app: &mut App,
    effect: Effect,
    command_tx: &UnboundedSender<Command>,
    pending: &mut Option<ApprovalRequest>,
) {
    match effect {
        Effect::Run(task) => {
            let _ = command_tx.send(Command::Run(task));
        }
        Effect::Resume(id) => {
            let path = session::session_path(home, &id);
            match session::replay(&path) {
                Ok(records) => {
                    app.seed(&records);
                    app.status.session_id = id.clone();
                    let _ = command_tx.send(Command::Resume(id));
                }
                Err(error) => {
                    app.error = Some(format!("could not open session `{id}`: {error}"));
                }
            }
        }
        Effect::Answer(decision) => {
            if let Some(request) = pending.take() {
                let _ = request.responder.send(decision);
            }
        }
        Effect::Quit => app.should_quit = true,
    }
}

/// Run the agent on its own thread and runtime, so the interface thread
/// never waits on the network or on a tool.
fn spawn_agent(config: AgentConfig) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let AgentConfig {
            home,
            model,
            session_id,
            ui_tx,
            status_tx,
            approval_tx,
            command_rx,
        } = config;
        // The scheduled consolidation cycle: one pass now, then one
        // every hour for the life of the process — the timer the live
        // system never had. Its own stream, because the cycle must run
        // even when routing never produced one; two writer threads in
        // one process are safe by the stream's own construction.
        let user_home = std::env::var("HOME").ok();
        let calib_root = crate::calib::resolve_root(&home, user_home.as_deref());
        let memories_root =
            crate::memory::resolve_root(&std::env::current_dir().unwrap_or_default());
        crate::memory::spawn_consolidator(memories_root, crate::calib::Stream::open(&calib_root));
        // The scheduled jobs: the catch-up pass once at startup, then one
        // tick per second for the life of the process. The scheduler owns
        // its own thread and runtime — a fire is an ordinary headless run
        // in its own session log, never a turn in the session this
        // interface is driving — and it reads the job store fresh every
        // tick, so `fleet cron add` from another shell lands within a
        // second. A finished fire rides the interface's own event
        // channel and lands as a card (KI-090); the store and latch
        // lines the pass names still ride stderr.
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
            // between runs, and each run's `skills` record pins what
            // that run actually saw.
            let user_home = std::env::var("HOME").ok();
            let skills_override = std::env::var("FLEET_SKILLS_DIR").ok();
            let (skills, skill_skips) =
                crate::skills::resolve_skills_dir(skills_override.as_deref(), user_home.as_deref())
                    .map(|dir| crate::skills::load_reported(&dir))
                    .unwrap_or_default();
            // The skips are a fact about the library, not something the
            // model said: one card, never bare stderr over the pane
            // (KI-216). A clean load sends nothing.
            if !skill_skips.is_empty() {
                let _ = ui_tx.send(crate::agent::UiEvent::SkillsLoaded {
                    loaded: skills.len(),
                    skipped: skill_skips.into_iter().map(|skip| skip.name).collect(),
                });
            }
            // The route is chosen once, on the first run, and the committed
            // router is reused for the rest of the session — one sys1
            // judgment, not one per task.
            let mut prepared: Option<route::Prepared> = None;
            let mut judge_task = String::new();
            let mut current_session = session_id;
            let mut commands = command_rx;
            while let Some(command) = commands.recv().await {
                match command {
                    Command::Run(task) => {
                        if prepared.is_none() {
                            match route::prepare(
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
                        // The corpus is loaded per run, off the async
                        // worker: the files can change between runs, and
                        // a fact an earlier run of this session
                        // remembered is on disk by the time this one
                        // assembles. A load that cannot complete leaves
                        // the run memory-less rather than failing it.
                        let memory = tokio::task::spawn_blocking(|| {
                            let root = crate::memory::resolve_root(
                                &std::env::current_dir().unwrap_or_default(),
                            );
                            std::sync::Arc::new(crate::memory::Memory::load(&root))
                        })
                        .await
                        .ok();
                        // One agent per task over a freshly opened log: the
                        // replay seeds the conversation exactly as a resumed
                        // headless run is seeded.
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
                            // proposal, and only the owner's `fleet
                            // improve apply` runs one.
                            improve_home: Some(home.clone()),
                            // The interactive surface has no forced-fold
                            // flag: its owner is present, so the budget's
                            // own timing is the right one.
                            compact_now: false,
                        };
                        let approver: Box<dyn Approver> =
                            Box::new(TuiApprover::new(approval_tx.clone()));
                        // Progress text has a sink here: the pane renders the
                        // events, and the log records exactly what headless
                        // records.
                        let mut progress = io::sink();
                        let mut agent =
                            match Agent::new(&client, options, &mut log, approver, &mut progress) {
                                Ok(agent) => agent,
                                Err(error) => {
                                    let _ = ui_tx.send(UiEvent::Failed(error.to_string()));
                                    continue;
                                }
                            };
                        if let Ok(report) = agent.run(&task).await {
                            let _ = status_tx.send(StatusUpdate::Usage(report.usage));
                        }
                    }
                    Command::Resume(id) => current_session = id,
                }
            }
            // The session's routing decision is recorded once, when the
            // agent thread ends — best-effort, never fatal.
            if let Some(prepared) = &prepared {
                prepared.record_route(&judge_task);
            }
            // And the self-authoring half runs once, on the session the
            // TUI drove: its lessons are drafted into proposals awaiting
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
    use app::Mode;

    #[test]
    fn a_failed_sessions_read_is_an_error_not_an_empty_picker() {
        // A `sessions` path that is a file, not a directory: read_dir
        // fails with something other than NotFound, which is exactly the
        // IO error the picker must report instead of swallowing.
        let home = std::env::temp_dir().join(format!("fleet-picker-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).unwrap();
        std::fs::write(home.join("sessions"), b"not a directory").unwrap();

        let mut app = App::new("m", "s");
        open_picker(&home, &mut app);
        // The picker never opened, and the banner says why.
        assert_eq!(app.mode, Mode::Chat);
        assert_eq!(app.sessions.len(), 0);
        let error = app.error.clone().expect("an error banner");
        assert!(error.contains("could not list the sessions"), "{error}");

        let _ = std::fs::remove_dir_all(&home);
    }
}
