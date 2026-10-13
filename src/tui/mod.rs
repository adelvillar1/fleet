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
    Agent, AgentOptions, ApprovalRequest, Approver, DEFAULT_MAX_TURNS, TuiApprover, UiEvent,
};
use crate::api::Client;
use crate::session::{self, Session};

use app::{Action, App, Effect, StatusUpdate};
use events::action_for;

/// How long the event loop waits for a key before redrawing. Token deltas
/// and agent traffic wake it immediately through their channels.
const POLL: Duration = Duration::from_millis(100);

/// What the TUI needs resolved before it starts.
pub struct Config {
    /// The resolved API key.
    pub key: String,
    /// The resolved API base URL.
    pub base_url: String,
    /// The fleet home that owns the sessions directory.
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
    base_url: String,
    key: String,
    model: String,
    session_id: String,
    ui_tx: UnboundedSender<UiEvent>,
    status_tx: UnboundedSender<StatusUpdate>,
    approval_tx: UnboundedSender<ApprovalRequest>,
    command_rx: UnboundedReceiver<Command>,
}

/// Restores the terminal when the TUI ends, including on a panic, so the
/// shell is never left in raw mode or on the alternate screen.
struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
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
    let mut terminal = ratatui::try_init()
        .map_err(|error| format!("the terminal could not start: {error}"))?;
    let _guard = TerminalGuard;

    let session_id = create_session(&config.home, &config.model)?;
    let (ui_tx, ui_rx) = tokio::sync::mpsc::unbounded_channel::<UiEvent>();
    let (status_tx, status_rx) = tokio::sync::mpsc::unbounded_channel::<StatusUpdate>();
    let (approval_tx, approval_rx) = tokio::sync::mpsc::unbounded_channel::<ApprovalRequest>();
    let (command_tx, command_rx) = tokio::sync::mpsc::unbounded_channel::<Command>();

    let agent = spawn_agent(AgentConfig {
        home: config.home.clone(),
        base_url: config.base_url,
        key: config.key,
        model: config.model.clone(),
        session_id: session_id.clone(),
        ui_tx,
        status_tx,
        approval_tx,
        command_rx,
    });

    let mut app = App::new(&config.model, &session_id);
    let outcome = event_loop(
        &config.home,
        &mut app,
        &mut terminal,
        ui_rx,
        status_rx,
        approval_rx,
        &command_tx,
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
fn event_loop(
    home: &Path,
    app: &mut App,
    terminal: &mut UiTerminal,
    mut ui_rx: UnboundedReceiver<UiEvent>,
    mut status_rx: UnboundedReceiver<StatusUpdate>,
    mut approval_rx: UnboundedReceiver<ApprovalRequest>,
    command_tx: &UnboundedSender<Command>,
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

        if ratatui::crossterm::event::poll(POLL)
            .map_err(|error| format!("the keyboard could not be read: {error}"))?
        {
            if let Ok(Event::Key(key)) = ratatui::crossterm::event::read() {
                dispatch_key(home, app, &key, command_tx, &mut pending);
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
    pending: &mut Option<ApprovalRequest>,
) {
    let Some(action) = action_for(app, key) else { return };
    // The picker's rows are filesystem reads, so the loop loads them and
    // hands them to the state machine.
    if matches!(action, Action::OpenSessions) {
        let sessions = session::list(home).unwrap_or_default();
        app.open_sessions(sessions);
        return;
    }
    if let Some(effect) = app.apply(action) {
        apply_effect(home, app, effect, command_tx, pending);
    }
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
            base_url,
            key,
            model,
            session_id,
            ui_tx,
            status_tx,
            approval_tx,
            command_rx,
        } = config;
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
            let client = Client::new(base_url, key);
            let mut current_session = session_id;
            let mut commands = command_rx;
            while let Some(command) = commands.recv().await {
                match command {
                    Command::Run(task) => {
                        let path = session::session_path(&home, &current_session);
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
                            ui_tx: Some(ui_tx.clone()),
                        };
                        let approver: Box<dyn Approver> =
                            Box::new(TuiApprover::new(approval_tx.clone()));
                        // Progress text has a sink here: the pane renders the
                        // events, and the log records exactly what headless
                        // records.
                        let mut progress = io::sink();
                        let mut agent = match Agent::new(
                            &client,
                            options,
                            &mut log,
                            approver,
                            &mut progress,
                        ) {
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
        });
    })
}
