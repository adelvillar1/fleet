//! The TUI's state machine: everything the interface knows, with no
//! terminal, no crossterm, and no I/O of its own. The event loop feeds it
//! key actions and UI events and acts on the effects it returns;
//! rendering only reads it. Keeping it pure is what makes the interaction
//! testable without a terminal.

use std::time::Instant;

use crate::agent::{Approval, SubagentStage, UiEvent};
use crate::api::Usage;
use crate::boards::{BoardData, BoardKind, Boards};
use crate::lanes::LaneHealth;
use crate::session::{Record, SessionSummary};

/// The sessions table's filter. A picker that quietly hides rows is a
/// picker the owner cannot trust, so the active filter is named in the
/// title and cycled with one key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SessionFilter {
    #[default]
    All,
    Open,
    Ended,
    Delegated,
    Compacted,
}

impl SessionFilter {
    pub fn next(self) -> SessionFilter {
        match self {
            SessionFilter::All => SessionFilter::Open,
            SessionFilter::Open => SessionFilter::Ended,
            SessionFilter::Ended => SessionFilter::Delegated,
            SessionFilter::Delegated => SessionFilter::Compacted,
            SessionFilter::Compacted => SessionFilter::All,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            SessionFilter::All => "all",
            SessionFilter::Open => "open",
            SessionFilter::Ended => "ended",
            SessionFilter::Delegated => "delegated",
            SessionFilter::Compacted => "compacted",
        }
    }

    fn matches(self, summary: &SessionSummary) -> bool {
        match self {
            SessionFilter::All => true,
            SessionFilter::Open => !summary.ended,
            SessionFilter::Ended => summary.ended,
            SessionFilter::Delegated => {
                summary.kind.as_deref() == Some(crate::session::SUBAGENT_KIND)
            }
            SessionFilter::Compacted => summary.compacted,
        }
    }
}

/// One rendered block in the conversation pane.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Entry {
    /// Something the user submitted.
    User(String),
    /// Assistant text: a completed turn's text.
    Assistant(String),
    /// One tool call, with where it stands and what came back.
    Tool(ToolCard),
    /// One lane joining the run, with the tools it brought.
    Lane(LaneCard),
    /// The skills library at session start, when something was skipped.
    Skills(SkillsCard),
    /// One compaction: the turns it folded into a summary, and what the
    /// context cost before and after.
    Compaction(CompactionCard),
    /// One scheduled fire: the job, the minute it ran for, how it
    /// ended, and the session it wrote. A fact about the machine, not
    /// something the model said — the run belongs to a job, in a log
    /// of its own, never a turn in this conversation.
    Cron(CronCard),
    /// One deck the run built. The browser face renders the fragment; a
    /// terminal carries the fact that it exists and its shape.
    Deck(DeckCard),
}

/// One tool call as a card: name, one-line summary, a status dot, and the
/// result preview once there is one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolCard {
    pub name: String,
    pub summary: String,
    pub status: ToolStatus,
    pub preview: String,
}

/// One lane joining a run: its name and how many tools it brought into
/// the model's tool set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaneCard {
    pub lane: String,
    pub tool_count: usize,
}

/// The skills library at session start, when something was skipped: how
/// many loaded, and which directories did not. A fact about the
/// library, not something the model said — its own card instead of bare
/// stderr over the pane (KI-216).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillsCard {
    pub loaded: usize,
    pub skipped: Vec<String>,
}

/// One compaction as a card, in the same family as a tool card: how many
/// turns became one summary, and what the context cost either side of
/// the fold.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompactionCard {
    pub turns: usize,
    pub tokens_before: u64,
    pub tokens_after: u64,
}

/// One scheduled fire as a card, in the same family as a tool card: the
/// job's name, the minute it ran for (named as a catch-up when the
/// window was missed while fleet was down), how the run ended, and the
/// session log it wrote — the retained output, reachable from the card
/// (KI-090).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CronCard {
    pub job: String,
    /// The minute the fire ran for, stamped.
    pub minute: String,
    /// Whether the fire served a window missed while fleet was down.
    pub catch_up: bool,
    /// How it ended: `completed`, `failed`, `skipped-needs-approval`,
    /// or `panicked`.
    pub outcome: String,
    /// The one-line reason.
    pub detail: String,
    /// The session log the run wrote, when it got as far as one.
    pub session: Option<String>,
}

/// One deck the run built, as a terminal carries it: the kind, the
/// size, and where to see it. The fragment itself rides the event log
/// for the browser face, which can render it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeckCard {
    /// `presentation`, `infographic` or `poster`.
    pub project: String,
    /// How many nodes it carries.
    pub nodes: usize,
}

/// Where a tool call stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolStatus {
    /// Requested, waiting on the approval gate.
    Pending,
    /// Approved (or latched) and executing.
    Approved,
    /// Refused by the user.
    Denied,
    /// Executed; `preview` carries the result's first line.
    Done,
}

/// The right pane's surface: the live run, or one of the three boards.
/// A tab, not a cycle hidden behind a key, so where the owner is stays
/// visible.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pane {
    Run,
    Router,
    Tower,
    Brain,
}

impl Pane {
    /// Every pane, in tab order. Run leads because it is the surface a
    /// run is watched on; the boards keep the router → brain → tower
    /// order the old Ctrl-B cycle used.
    pub const ALL: [Pane; 4] = [Pane::Run, Pane::Router, Pane::Brain, Pane::Tower];

    pub fn label(self) -> &'static str {
        match self {
            Pane::Run => "run",
            Pane::Router => "router",
            Pane::Tower => "tower",
            Pane::Brain => "brain",
        }
    }

    /// The board this pane shows, when it shows one.
    pub fn board(self) -> Option<BoardKind> {
        match self {
            Pane::Run => None,
            Pane::Router => Some(BoardKind::Router),
            Pane::Tower => Some(BoardKind::Tower),
            Pane::Brain => Some(BoardKind::Brain),
        }
    }

    pub fn next(self) -> Pane {
        match self {
            Pane::Run => Pane::Router,
            Pane::Router => Pane::Brain,
            Pane::Brain => Pane::Tower,
            Pane::Tower => Pane::Run,
        }
    }

    pub fn index(self) -> usize {
        Pane::ALL.iter().position(|pane| *pane == self).unwrap_or(0)
    }
}

/// Where the current run is in its loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Stage {
    #[default]
    Route,
    Think,
    Act,
    Verify,
}

impl Stage {
    pub const ALL: [Stage; 4] = [Stage::Route, Stage::Think, Stage::Act, Stage::Verify];

    pub fn label(self) -> &'static str {
        match self {
            Stage::Route => "route",
            Stage::Think => "think",
            Stage::Act => "act",
            Stage::Verify => "verify",
        }
    }

    pub fn index(self) -> usize {
        Stage::ALL
            .iter()
            .position(|stage| *stage == self)
            .unwrap_or(0)
    }
}

/// One delegated child, as the run card shows it. The parent's subagent
/// tool card keeps the lifecycle dot; this is the branch's own state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Branch {
    pub session: String,
    pub goal: String,
    pub stage: String,
    pub calls: u64,
    pub tool: String,
    pub outcome: String,
    pub turns: u64,
}

/// The live run, reduced from the same `UiEvent` log the conversation
/// reads. The browser face's process card is the same idea; the TUI's
/// copy is denser and carries the terminal's own controls.
#[derive(Debug, Clone)]
pub struct RunState {
    pub active: bool,
    /// The composer's plan-mode toggle for the next run.
    pub plan: bool,
    pub stage: Stage,
    pub started: Option<Instant>,
    pub ended: Option<Instant>,
    pub turn: u64,
    pub tool: String,
    pub calls: u64,
    pub facts: usize,
    pub shape: String,
    /// The last request's prompt size: the context the next model call
    /// will carry, and what the status gauge divides by the budget.
    pub context: u64,
    pub lanes: u64,
    pub checkpoint: Option<u64>,
    pub provider: String,
    pub model: String,
    pub workload: String,
    /// `Some(true)` means the calibration may move the failover's
    /// preference; `Some(false)` is the gate shut. `None` is a home
    /// whose config could not be read at launch.
    pub gate: Option<bool>,
    pub branches: Vec<Branch>,
    pub last: String,
    pub stop_reason: Option<String>,
    pub stopping: bool,
    /// Tokens per streamed response, for the pane's sparkline. This is
    /// not the spend line: the status bar sums the finished report.
    pub tokens: Vec<u64>,
}

impl Default for RunState {
    fn default() -> RunState {
        RunState {
            active: false,
            plan: false,
            stage: Stage::Route,
            started: None,
            ended: None,
            turn: 0,
            tool: String::new(),
            calls: 0,
            facts: 0,
            shape: String::new(),
            context: 0,
            lanes: 0,
            checkpoint: None,
            provider: String::new(),
            model: String::new(),
            workload: String::new(),
            gate: None,
            branches: Vec::new(),
            last: String::new(),
            stop_reason: None,
            stopping: false,
            tokens: Vec::new(),
        }
    }
}

impl RunState {
    /// One line for the narrow layout and the status bar.
    pub fn strip(&self) -> String {
        let mut parts: Vec<String> = Vec::new();
        if self.active {
            parts.push("in flight".to_string());
        } else if let Some(reason) = &self.stop_reason {
            parts.push(reason.clone());
        } else {
            parts.push("idle".to_string());
        }
        if self.turn > 0 {
            parts.push(format!("t{}", self.turn));
        }
        if !self.tool.is_empty() {
            parts.push(self.tool.clone());
        }
        if self.calls > 0 {
            parts.push(format!("{} call(s)", self.calls));
        }
        if self.gate == Some(true) {
            parts.push("gate on".to_string());
        } else if self.gate == Some(false) {
            parts.push("gate shut".to_string());
        }
        parts.join(" \u{b7} ")
    }
}

/// What the interface is doing right now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Typing and reading the conversation.
    Chat,
    /// The session picker is open over the conversation.
    Sessions,
    /// An approval prompt is waiting on a key.
    Approval,
    /// The lane health view is open over the conversation.
    Lanes,
    /// The latest explanation card is open over the conversation.
    Explain,
    /// The settings pane is open over the conversation.
    Settings,
    /// One setting is being edited inside the settings pane.
    SettingsEdit,
    /// A destructive removal is armed and the pane is asking whether it
    /// lands. The CLI's `force` flag is a shell word; a pane has nowhere
    /// to put one, so the question is what stands in for it.
    SettingsConfirm,
    /// The run controls are open over the conversation: a branch cursor,
    /// a stop, and a steer.
    Run,
    /// The one-line steer editor inside the run controls.
    Steer,
    /// The command palette: a filtered list of every surface and action.
    Palette,
    /// The checkpoint list over the conversation.
    Checkpoints,
    /// The diff for one checkpoint.
    Diff,
    /// A trust-trio write is armed and waiting for `y`.
    Confirm,
    /// One knowledge panel over the conversation.
    Panel,
}

/// One key press, translated for the current mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Type a character into the input box.
    Input(char),
    /// Delete the character before the cursor.
    Backspace,
    /// Delete the character at the cursor (forward delete).
    Delete,
    /// Delete the word before the cursor (alt+backspace, ctrl+w).
    DeleteWord,
    /// Move the input cursor one character left.
    CursorLeft,
    /// Move the input cursor one character right.
    CursorRight,
    /// Move the input cursor to the start of the input.
    CursorHome,
    /// Move the input cursor to the end of the input.
    CursorEnd,
    /// Submit the input box.
    Submit,
    /// Leave the TUI.
    Quit,
    /// Open the session picker.
    OpenSessions,
    /// Open the settings pane.
    OpenSettings,
    /// Commit the setting being edited.
    SettingsCommit,
    /// Abandon the setting being edited.
    SettingsCancel,
    /// Arm the destructive confirmation for the selected list entry.
    SettingsRemove,
    /// The armed removal lands.
    SettingsConfirm,
    /// The armed removal is abandoned, with nothing written.
    SettingsAbandon,
    /// Open the lane health view.
    OpenLanes,
    /// Open the latest explanation card.
    OpenExplain,
    /// Cycle the right pane to the next tab.
    CycleBoard,
    /// Open the command palette.
    OpenPalette,
    /// Show one pane/tab.
    OpenPane(Pane),
    /// Open one knowledge panel.
    OpenPanel(PanelKind),
    /// Cycle the open panel's filter.
    CyclePanelFilter,
    /// Open the checkpoints view.
    OpenCheckpoints,
    /// Open the diff review for the selected checkpoint.
    OpenDiff,
    /// Move the diff's hunk cursor.
    MoveHunk(i32),
    /// Reject the selected hunk.
    RejectHunk,
    /// Reject the whole selected file.
    RejectFile,
    /// Land the armed trust-trio write.
    TrustConfirm,
    /// Abandon the armed write.
    TrustCancel,
    /// Open the run controls.
    OpenRunControls,
    /// Stop the run in flight.
    StopRun,
    /// Open the one-line steer editor inside the run controls.
    OpenSteer,
    /// Toggle plan mode for the next run.
    TogglePlan,
    /// Load the ready plan into the message box for editing.
    LoadPlan,
    /// Close whatever overlay is open.
    CloseOverlay,
    /// Cycle the sessions picker's filter.
    CycleSessionFilter,
    /// Move the picker selection; positive is down.
    Move(i32),
    /// Pick the selected session or branch.
    Pick,
    /// Answer the pending approval.
    Approve(Approval),
    /// Scroll the conversation; positive is up.
    Scroll(i32),
}

/// Something the event loop must do outside the state machine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    /// Send this task to the agent. `plan` is the composer's plan-mode
    /// toggle for this run.
    Run { task: String, plan: bool },
    /// Resume this session id: seed the conversation and append to its log.
    Resume(String),
    /// Answer the pending approval.
    Answer(Approval),
    /// Write the run's control file: `steer` is the one-line text, and
    /// `None` is a stop.
    Control { steer: Option<String> },
    /// Restore the tree to this turn's checkpoint, cut the log, and
    /// reseed the view.
    Rewind(u64),
    /// Put this hunk's checkpoint lines back.
    RejectHunk {
        turn: u64,
        file: String,
        hunk: usize,
    },
    /// Put the whole file back.
    RejectFile { turn: u64, file: String },
    /// Leave the TUI.
    Quit,
}

/// One command in the palette. The labels are the surface's own words,
/// so the palette and the key hints cannot drift.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PaletteItem {
    pub label: &'static str,
    pub detail: &'static str,
    pub action: Action,
}

/// The knowledge panels the palette opens. They are on-demand reads,
/// not the 30-second boards: the library scan and the archive replay are
/// too heavy to run on a cadence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelKind {
    Memory,
    Skills,
    Calibration,
    Delegation,
    /// The channel service: the channels, the peers, the door, and what
    /// they have done. It reads the same document the browser's Bots
    /// surface reads, so the two cannot disagree.
    Bots,
}

impl PanelKind {
    pub fn label(self) -> &'static str {
        match self {
            PanelKind::Memory => "memory",
            PanelKind::Skills => "skills",
            PanelKind::Calibration => "calibration",
            PanelKind::Delegation => "delegation",
            PanelKind::Bots => "bots",
        }
    }
}

/// The channel service as the Bots panel reads it. It is the same
/// document the browser renders, unpacked: the service's own status,
/// every channel's paired and pending peers, and the newest activity.
#[derive(Debug, Clone)]
pub struct BotsSnapshot {
    pub service: Option<crate::channels::Status>,
    pub stale: bool,
    pub channels: Vec<BotsChannel>,
    pub activity: Vec<crate::channels::service::Activity>,
}

#[derive(Debug, Clone)]
pub struct BotsChannel {
    pub name: String,
    pub peers: Vec<crate::channels::PairedPeer>,
    pub pending: Vec<crate::channels::PendingPeer>,
}

impl BotsSnapshot {
    /// Read it from the home: the status document, the stores, and the
    /// activity log. A home with nothing configured is an empty snapshot
    /// rather than an error — the panel says "nothing configured", which
    /// is what the machine is.
    pub fn read(home: &std::path::Path) -> BotsSnapshot {
        let service = crate::channels::Status::load(home);
        let stale = service
            .as_ref()
            .map(|status| status.is_stale(std::time::SystemTime::now()))
            .unwrap_or(false);
        let channels = crate::channels::CHANNELS
            .iter()
            .map(|name| BotsChannel {
                name: (*name).to_string(),
                peers: crate::channels::PeerStore::load(home, name)
                    .map(|store| store.peers)
                    .unwrap_or_default(),
                pending: crate::channels::PendingStore::load(home, name).peers,
            })
            .collect();
        BotsSnapshot {
            service,
            stale,
            channels,
            activity: crate::channels::service::activity(home, 50),
        }
    }
}

/// One calibration read, as the panel needs it.
#[derive(Debug, Clone)]
pub struct CalibrationData {
    pub day: String,
    pub rows_read: usize,
    pub cells: Vec<crate::calib::CalibrationCell>,
}

/// The landed data behind a panel.
#[derive(Debug, Clone)]
pub enum PanelData {
    Memory(crate::boards::MemorySnapshot),
    Skills(crate::boards::SkillsSnapshot),
    Calibration(CalibrationData),
    Delegation(crate::boards::DelegationSnapshot),
    Bots(BotsSnapshot),
}

/// One selectable row in a knowledge panel. The list is generic so the
/// table has one shape; each panel decides what the row says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PanelRow {
    pub label: String,
    pub meta: String,
    pub detail: String,
}

/// The trust-trio write a confirmation is about to land.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrustAction {
    Rewind(u64),
    RejectHunk {
        turn: u64,
        file: String,
        hunk: usize,
    },
    RejectFile {
        turn: u64,
        file: String,
    },
}

/// One armed trust-trio write, with the sentence the confirmation shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrustConfirm {
    pub title: String,
    pub detail: String,
    pub action: TrustAction,
    /// The view `esc` returns to when the write is abandoned.
    pub back: Mode,
}

/// The prompt shown in the approval modal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalPrompt {
    pub name: String,
    pub summary: String,
}

/// One armed destructive removal: the commit key it lands through — the
/// writer's own branch, never a pane-local path — and the sentence
/// naming exactly what will be gone. The pane asks before it lands
/// because the CLI's `force` flag is a shell word and a pane has nowhere
/// to put one; the writer's compiled-in rule still answers when the
/// removal lands, so the two surfaces cannot disagree about what is
/// destructive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsRemoval {
    pub key: String,
    pub detail: String,
}

/// One selectable row in the settings pane, in render order: the scalar
/// knobs, then the provider cards, then the workload cards. The
/// selection indexes this list, so walking off the last scalar lands on
/// the first provider card rather than on a second cursor the owner has
/// to discover.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsRow {
    /// The `n`-th scalar knob, as `settings::states` returned it.
    Scalar(usize),
    /// The `n`-th provider override the home's config.json carries.
    Provider(usize),
    /// The `n`-th workload preference the home's routing.json carries.
    Workload(usize),
}

/// The status bar's facts.
#[derive(Debug, Clone)]
pub struct Status {
    pub model: String,
    pub session_id: String,
    /// The current run's turn, 0 when idle.
    pub turn: u64,
    /// Tokens counted since this view opened. A resume restarts the
    /// count: the counters describe the conversation on screen, and a
    /// resumed log replays runs whose usage the live stream never sees.
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    /// True while a run is in flight.
    pub working: bool,
    /// The active provider for the current/last routed run, and how it was
    /// chosen (`judged`/`fallback`/`pinned`). `None` until a routed run
    /// reports one.
    pub provider: Option<String>,
    pub route_how: Option<String>,
    /// The turns whose file state the checkpoint store holds, newest
    /// last. The TUI's rewind affordance reads it; the browser face
    /// reads the store itself.
    pub checkpoints: Vec<u64>,
    /// The routing mode at launch, before any run reports: `auto-router`,
    /// or `pinned to <pin>` when the home carries one. The bar names the
    /// mode, never the compiled-in default model — the default is the
    /// DeepSeek reserve, and naming it as "the model in use" is the
    /// misdirection this field exists to prevent (KI-217).
    pub routing: Option<String>,
    /// What the last run's assembled memory carried: the shape the
    /// instruction routed under (empty when unclassified) and how many
    /// facts entered the turn's context. `None` until a run assembles
    /// something, and reset when a resumed log is seeded — it describes
    /// the conversation on screen.
    pub memory: Option<(String, usize)>,
}

/// What the agent thread reports beside the `UiEvent` stream: exact token
/// accounting, which the stream does not carry.
#[derive(Debug, Clone)]
pub enum StatusUpdate {
    Usage(Usage),
}

/// Cap on the input box, so a runaway paste cannot grow without bound.
const MAX_INPUT: usize = 8_192;

/// The whole TUI state.
#[derive(Debug, Clone)]
pub struct App {
    pub entries: Vec<Entry>,
    /// Assistant text still streaming; flushed into `entries` when the
    /// turn's next event arrives.
    streaming: String,
    pub input: String,
    /// The input cursor, a character index into `input`. Editing keeps
    /// it clamped to the input's length, so it never points past a char.
    pub cursor: usize,
    pub mode: Mode,
    /// The picker's rows, oldest first, exactly as `session::list` returns.
    pub sessions: Vec<SessionSummary>,
    /// The picker's filter, named in the picker's title.
    pub session_filter: SessionFilter,
    /// The lane health rows, in registry order, exactly as
    /// `lanes::health_all` returns.
    pub lanes: Vec<LaneHealth>,
    /// True while the lane probe is in flight and no rows have landed.
    pub lanes_loading: bool,
    /// Which probe the open lanes view belongs to. Every `open_lanes`
    /// stamps a new generation, so rows from a probe the user has already
    /// closed can never render into a freshly opened modal: the event loop
    /// cannot tell them apart by mode alone, because both arrive while the
    /// modal is open.
    pub lanes_probe: u64,
    pub selected: usize,
    /// The settings pane's rows, exactly as `settings::states` returns
    /// them, and the one being edited: its key and the buffer the owner
    /// types into. The buffer starts as the setting's current value, so
    /// an edit is a change rather than a retype.
    pub settings: Vec<crate::settings::State>,
    /// The two list-shaped surfaces beside the scalars: the provider
    /// overrides and the workload preferences the home's own files
    /// carry, exactly as `settings::list_states` reads them.
    pub settings_lists: Vec<crate::settings::ListState>,
    pub settings_home: Option<std::path::PathBuf>,
    pub editing: Option<(String, String)>,
    /// The armed destructive removal, while the pane is asking about it.
    pub settings_removal: Option<SettingsRemoval>,
    /// A settings write's outcome, named on one line rather than
    /// silently changing nothing: a refusal the loader would have made
    /// is shown here, in the pane, where the owner is looking.
    pub settings_note: Option<String>,
    pub approval: Option<ApprovalPrompt>,
    pub status: Status,
    /// A failure banner for the conversation pane.
    pub error: Option<String>,
    /// Lines scrolled back from the bottom; 0 means following the tail.
    pub scrolled_back: usize,
    /// True when the view sticks to the newest content.
    pub following: bool,
    /// The right pane's tab and the live run it can show.
    pub pane: Pane,
    pub run: RunState,
    /// The ready plan's text. `Ctrl-E` loads it into the message box;
    /// the TUI never sends it on its own.
    pub plan_text: Option<String>,
    /// The one-line steer buffer inside the run controls.
    pub steer_input: String,
    /// The palette's filter and cursor. It starts empty on every open,
    /// so a previous command never leaks into the next one.
    pub palette_query: String,
    pub palette_selected: usize,
    /// The checkpoint list and the diff it can open.
    pub checkpoints_view: Vec<crate::checkpoints::Snapshot>,
    pub diff_view: Vec<crate::checkpoints::FileDiff>,
    pub diff_turn: u64,
    pub diff_file: usize,
    pub diff_hunk: usize,
    /// The armed trust-trio write, while the confirmation is asking.
    pub confirm: Option<TrustConfirm>,
    /// The open knowledge panel, its landing data, and the generation
    /// its read belongs to.
    pub panel_kind: Option<PanelKind>,
    pub panel: Option<PanelData>,
    pub panel_loading: bool,
    pub panel_generation: u64,
    pub panel_filter: usize,
    /// Why the open panel's read did not land, if it did not.
    pub panel_note: Option<String>,
    /// The three read-only boards: each one's last snapshot with its
    /// named refusal.
    pub boards: Boards,
    /// The latest explanation card's lines, or `None` when today's
    /// stream holds no explanation — the headless card, read back from
    /// the file it indexed, so the two surfaces agree by construction.
    pub explain: Option<Vec<String>>,
    pub should_quit: bool,
}

impl App {
    /// A fresh app on a new session.
    pub fn new(model: &str, session_id: &str) -> App {
        App {
            entries: Vec::new(),
            streaming: String::new(),
            input: String::new(),
            cursor: 0,
            mode: Mode::Chat,
            sessions: Vec::new(),
            session_filter: SessionFilter::default(),
            lanes: Vec::new(),
            lanes_loading: false,
            lanes_probe: 0,
            explain: None,
            settings: Vec::new(),
            settings_lists: Vec::new(),
            settings_home: None,
            editing: None,
            settings_removal: None,
            settings_note: None,
            selected: 0,
            approval: None,
            status: Status {
                model: model.to_string(),
                session_id: session_id.to_string(),
                turn: 0,
                prompt_tokens: 0,
                completion_tokens: 0,
                working: false,
                provider: None,
                route_how: None,
                checkpoints: Vec::new(),
                routing: None,
                memory: None,
            },
            error: None,
            scrolled_back: 0,
            following: true,
            pane: Pane::Router,
            run: RunState::default(),
            plan_text: None,
            steer_input: String::new(),
            palette_query: String::new(),
            palette_selected: 0,
            checkpoints_view: Vec::new(),
            diff_view: Vec::new(),
            diff_turn: 0,
            diff_file: 0,
            diff_hunk: 0,
            confirm: None,
            panel_kind: None,
            panel: None,
            panel_loading: false,
            panel_generation: 0,
            panel_filter: 0,
            panel_note: None,
            boards: Boards::new(),
            should_quit: false,
        }
    }

    /// Apply one key action; the effect, if any, is the loop's to perform.
    pub fn apply(&mut self, action: Action) -> Option<Effect> {
        match self.mode {
            Mode::Approval => match action {
                Action::Approve(decision) => {
                    self.mode = Mode::Chat;
                    self.approval = None;
                    // The call the prompt was about now executes, so its
                    // card shows approved until its result lands.
                    self.mark_pending_approved();
                    Some(Effect::Answer(decision))
                }
                // Quitting under a modal drops the responder, which denies.
                Action::Quit => {
                    self.should_quit = true;
                    Some(Effect::Quit)
                }
                _ => None,
            },
            // The settings pane is a menu over the scalars and the two
            // lists beside them: move, edit the selection, close. Editing
            // and the removal affordance are the only ways out that
            // change anything, and the writer is the CLI's own.
            Mode::Settings => match action {
                Action::Move(delta) => {
                    let rows = self.settings_rows();
                    self.move_selection(delta, rows.len());
                    None
                }
                Action::Pick => {
                    self.edit_selected_row();
                    None
                }
                Action::SettingsRemove => {
                    self.arm_removal();
                    None
                }
                Action::CloseOverlay => {
                    self.mode = Mode::Chat;
                    self.settings_note = None;
                    None
                }
                Action::Quit => {
                    self.should_quit = true;
                    Some(Effect::Quit)
                }
                _ => None,
            },
            // The armed removal is answered or abandoned on any key: `y`
            // lands it through the writer's own branch, and everything
            // else walks away with nothing written.
            Mode::SettingsConfirm => match action {
                Action::SettingsConfirm => {
                    self.confirm_removal();
                    None
                }
                Action::Quit => {
                    self.should_quit = true;
                    Some(Effect::Quit)
                }
                _ => {
                    self.abandon_removal();
                    None
                }
            },
            // The editor types, commits, or abandons. A commit that the
            // loader's own validation refuses lands back in the pane
            // with its reason named, having changed nothing.
            Mode::SettingsEdit => match action {
                Action::Input(character) => {
                    self.edit_push(character);
                    None
                }
                Action::Backspace => {
                    self.edit_pop();
                    None
                }
                Action::Submit => {
                    self.commit_setting();
                    None
                }
                Action::CloseOverlay => {
                    self.cancel_setting();
                    None
                }
                Action::Quit => {
                    self.should_quit = true;
                    Some(Effect::Quit)
                }
                _ => None,
            },
            Mode::Sessions => match action {
                Action::Move(delta) => {
                    self.move_selection(delta, self.session_rows().len());
                    None
                }
                Action::CycleSessionFilter => {
                    self.cycle_session_filter();
                    None
                }
                Action::Pick => self.pick_session(),
                Action::CloseOverlay => {
                    self.mode = Mode::Chat;
                    None
                }
                Action::Quit => {
                    self.should_quit = true;
                    Some(Effect::Quit)
                }
                _ => None,
            },
            Mode::Lanes => match action {
                Action::Move(delta) => {
                    self.move_selection(delta, self.lanes.len());
                    None
                }
                Action::CloseOverlay => {
                    self.mode = Mode::Chat;
                    None
                }
                Action::Quit => {
                    self.should_quit = true;
                    Some(Effect::Quit)
                }
                // The lanes are information, not a menu: nothing to pick.
                _ => None,
            },
            // The explanation card is information too: it scrolls and it
            // closes, and it never types into the conversation.
            Mode::Explain => match action {
                Action::Move(delta) => {
                    let rows = self.explain.as_ref().map(|lines| lines.len()).unwrap_or(0);
                    self.move_selection(delta, rows);
                    None
                }
                Action::CloseOverlay => {
                    self.mode = Mode::Chat;
                    None
                }
                Action::Quit => {
                    self.should_quit = true;
                    Some(Effect::Quit)
                }
                _ => None,
            },
            // One knowledge panel: a cursor, a filter, and close.
            Mode::Panel => match action {
                Action::Move(delta) => {
                    self.move_selection(delta, self.panel_rows().len());
                    None
                }
                Action::CyclePanelFilter => {
                    self.cycle_panel_filter();
                    None
                }
                Action::CloseOverlay => {
                    self.mode = Mode::Chat;
                    None
                }
                Action::Quit => {
                    self.should_quit = true;
                    Some(Effect::Quit)
                }
                _ => None,
            },
            // The checkpoint list: a cursor, the rewind, and the diff.
            Mode::Checkpoints => match action {
                Action::Move(delta) => {
                    self.move_selection(delta, self.checkpoints_view.len());
                    None
                }
                Action::Pick => {
                    self.arm_rewind();
                    None
                }
                Action::CloseOverlay => {
                    self.mode = Mode::Chat;
                    None
                }
                Action::Quit => {
                    self.should_quit = true;
                    Some(Effect::Quit)
                }
                _ => None,
            },
            // The diff: a file cursor, a hunk cursor, and two rejects.
            // The write is armed first; `y` lands it in the confirm view.
            Mode::Diff => match action {
                Action::Move(delta) => {
                    self.diff_file = clamp_step(self.diff_file, delta, self.diff_view.len());
                    self.diff_hunk = 0;
                    None
                }
                Action::MoveHunk(delta) => {
                    let hunks = self
                        .selected_diff_file()
                        .map(|file| file.hunks.len())
                        .unwrap_or(0);
                    self.diff_hunk = clamp_step(self.diff_hunk, delta, hunks);
                    None
                }
                Action::RejectHunk => {
                    self.arm_reject_hunk();
                    None
                }
                Action::RejectFile => {
                    self.arm_reject_file();
                    None
                }
                Action::CloseOverlay => {
                    self.mode = Mode::Checkpoints;
                    None
                }
                Action::Quit => {
                    self.should_quit = true;
                    Some(Effect::Quit)
                }
                _ => None,
            },
            // An armed trust-trio write: `y` lands it, everything else
            // walks away with the tree untouched.
            Mode::Confirm => match action {
                Action::TrustConfirm => self.land_trust(),
                Action::TrustCancel => {
                    self.cancel_trust();
                    None
                }
                Action::Quit => {
                    self.should_quit = true;
                    Some(Effect::Quit)
                }
                _ => {
                    self.cancel_trust();
                    None
                }
            },
            // The run controls: a stop, a steer, and a branch cursor.
            // The stop is written through the agent's control file, so
            // its latency is the run's next model call and the card
            // says `stopping` until the end record lands.
            Mode::Run => match action {
                Action::Move(delta) => {
                    self.move_selection(delta, self.run.branches.len());
                    None
                }
                Action::StopRun => self.stop_run(),
                Action::OpenSteer => {
                    if self.run.active {
                        self.steer_input.clear();
                        self.mode = Mode::Steer;
                    }
                    None
                }
                Action::CloseOverlay => {
                    self.mode = Mode::Chat;
                    None
                }
                Action::Quit => {
                    self.should_quit = true;
                    Some(Effect::Quit)
                }
                _ => None,
            },
            // The steer editor is one line; it mirrors the message box's
            // editing keys but has its own buffer, so a steer never
            // consumes the task the owner was composing.
            Mode::Steer => match action {
                Action::Input(character) => {
                    if self.steer_input.chars().count() < 2000 {
                        self.steer_input.push(character);
                    }
                    None
                }
                Action::Backspace => {
                    self.steer_input.pop();
                    None
                }
                Action::Submit => self.send_steer(),
                Action::CloseOverlay => {
                    self.mode = Mode::Run;
                    None
                }
                Action::Quit => {
                    self.should_quit = true;
                    Some(Effect::Quit)
                }
                _ => None,
            },
            Mode::Palette => match action {
                Action::Input(character) => {
                    self.palette_query.push(character);
                    self.palette_selected = 0;
                    None
                }
                Action::Backspace => {
                    self.palette_query.pop();
                    self.palette_selected = 0;
                    None
                }
                Action::Move(delta) => {
                    self.palette_move(delta);
                    None
                }
                Action::CloseOverlay => {
                    self.mode = Mode::Chat;
                    None
                }
                Action::Quit => {
                    self.should_quit = true;
                    Some(Effect::Quit)
                }
                // `Pick` is performed by the event loop: it owns the
                // reads the chosen action may need.
                _ => None,
            },
            Mode::Chat => match action {
                Action::Input(character) => {
                    self.push_input(character);
                    None
                }
                Action::OpenPalette => {
                    self.open_palette();
                    None
                }
                Action::OpenPane(pane) => {
                    self.pane = pane;
                    if let Some(board) = pane.board() {
                        self.boards.active = board;
                    }
                    None
                }
                Action::Backspace => {
                    self.backspace();
                    None
                }
                Action::Delete => {
                    self.delete();
                    None
                }
                Action::DeleteWord => {
                    self.delete_word();
                    None
                }
                Action::CursorLeft => {
                    self.cursor_left();
                    None
                }
                Action::CursorRight => {
                    self.cursor_right();
                    None
                }
                Action::CursorHome => {
                    self.cursor = 0;
                    None
                }
                Action::CursorEnd => {
                    self.cursor = self.input.chars().count();
                    None
                }
                Action::Submit => self.submit(),
                // The palette reaches the stop and the steer from any
                // mode; their keys live in the run controls.
                Action::StopRun => self.stop_run(),
                Action::OpenSteer => {
                    if self.run.active {
                        self.steer_input.clear();
                        self.mode = Mode::Steer;
                    }
                    None
                }
                Action::TogglePlan => {
                    self.run.plan = !self.run.plan;
                    None
                }
                Action::LoadPlan => {
                    if let Some(plan) = self.plan_text.take() {
                        self.input = plan;
                        self.cursor = self.input.chars().count();
                        if self.mode == Mode::Run {
                            self.mode = Mode::Chat;
                        }
                    } else {
                        self.error = Some("no plan is ready to edit".to_string());
                    }
                    None
                }
                Action::OpenRunControls => {
                    self.selected = 0;
                    self.steer_input.clear();
                    self.mode = Mode::Run;
                    None
                }
                Action::OpenLanes => {
                    self.open_lanes();
                    None
                }
                Action::CycleBoard => {
                    self.cycle_pane();
                    None
                }
                Action::Scroll(delta) => {
                    self.scroll_by(delta);
                    None
                }
                Action::Quit => {
                    self.should_quit = true;
                    Some(Effect::Quit)
                }
                _ => None,
            },
        }
    }

    /// Apply one event off the agent's channel.
    pub fn apply_ui_event(&mut self, event: UiEvent) {
        match event {
            // The model's thinking is the face's row first; the TUI's own
            // Thought block is a later increment, so the pane reads the
            // answer and the calls as it does today.
            UiEvent::Reasoning { .. } => {}
            UiEvent::Token(delta) => self.streaming.push_str(&delta),
            UiEvent::Turn(turn) => {
                self.flush_streaming();
                self.run.active = true;
                self.run.stage = Stage::Think;
                self.run.turn = turn;
                self.run.last = format!("turn {turn}");
                self.status.turn = turn;
                self.status.working = true;
            }
            // The browser face renders a deck; a terminal cannot. The
            // deck still lands as its own card-shaped line so the run
            // that produced it reads as having produced something, and
            // the fragment itself stays in the log for the browser.
            UiEvent::Deck { project, nodes, .. } => {
                // The browser face renders a deck; a terminal cannot. The
                // deck still lands as its own card, the shape a cron fire
                // uses — a fact about the run rather than model text — so
                // the run that produced it reads as having produced
                // something. The fragment itself stays in the log for the
                // browser.
                self.flush_streaming();
                self.entries.push(Entry::Deck(DeckCard { project, nodes }));
            }
            UiEvent::Usage {
                prompt_tokens,
                completion_tokens,
            } => {
                // The per-response numbers feed the bar's totals and the
                // run card's sparkline. Summing these events, rather
                // than a finished `RunReport` that arrives on its own
                // channel, is what keeps a rewind honest: the totals
                // belong to the view, and a seed resets them before any
                // late report can repopulate them.
                self.run.context = prompt_tokens;
                self.status.prompt_tokens += prompt_tokens;
                self.status.completion_tokens += completion_tokens;
                let total = prompt_tokens + completion_tokens;
                self.run.tokens.push(total);
                if self.run.tokens.len() > 64 {
                    self.run.tokens.remove(0);
                }
            }
            UiEvent::ToolCall { name, summary, .. } => {
                self.flush_streaming();
                self.run.stage = Stage::Act;
                self.run.calls += 1;
                self.run.tool = name.clone();
                self.run.last = if summary.is_empty() {
                    name.clone()
                } else {
                    format!("{name} \u{b7} {summary}")
                };
                self.entries.push(Entry::Tool(ToolCard {
                    name,
                    summary,
                    status: ToolStatus::Pending,
                    preview: String::new(),
                }));
            }
            UiEvent::ToolResult {
                name,
                preview,
                approved,
                ..
            } => {
                let status = if approved {
                    ToolStatus::Done
                } else {
                    ToolStatus::Denied
                };
                self.run.stage = Stage::Act;
                self.run.tool = name.clone();
                self.run.last = format!(
                    "{name} \u{b7} {}",
                    if approved { "done" } else { "refused" }
                );
                // The card this result belongs to is the newest one for the
                // tool that is still open.
                let card = self
                    .entries
                    .iter_mut()
                    .rev()
                    .find_map(|entry| match entry {
                        Entry::Tool(card) if card.name == name => Some(card),
                        _ => None,
                    })
                    .filter(|card| {
                        matches!(card.status, ToolStatus::Pending | ToolStatus::Approved)
                    });
                match card {
                    Some(card) => {
                        card.status = status;
                        card.preview = preview;
                    }
                    // A result with no open card still belongs on screen.
                    None => self.entries.push(Entry::Tool(ToolCard {
                        name,
                        summary: String::new(),
                        status,
                        preview,
                    })),
                }
            }
            UiEvent::LaneJoined { lane, tool_count } => {
                // A lane joining the run is a fact about the run, not
                // something the model said: it flushes the streaming tail
                // and lands as its own block, exactly like a tool card.
                self.flush_streaming();
                self.run.stage = Stage::Act;
                self.run.lanes += 1;
                self.run.last = format!("lane {lane} \u{b7} {tool_count} tool(s)");
                self.entries
                    .push(Entry::Lane(LaneCard { lane, tool_count }));
            }
            UiEvent::SkillsLoaded { loaded, skipped } => {
                // The library's session-start report: a fact about the
                // library, not something the model said — its own card,
                // never bare stderr over the pane (KI-216).
                self.flush_streaming();
                self.entries
                    .push(Entry::Skills(SkillsCard { loaded, skipped }));
            }
            UiEvent::Session { id } => {
                // The log this run created, named once (KI-022): the
                // status bar shows the session it is logging into, and
                // nothing before this arrives — a launch that has not
                // run yet has no session to name.
                self.status.session_id = id;
            }
            UiEvent::Routed {
                provider,
                model,
                how,
                workload,
                ..
            } => {
                // The route this run takes: the status bar shows the active
                // provider and how it was chosen; the run card keeps the
                // model and the workload, because the next step needs to
                // name what actually answered.
                self.run.provider = provider.clone();
                self.run.model = model;
                self.run.workload = workload.unwrap_or_default();
                if self.run.turn == 0 {
                    self.run.stage = Stage::Route;
                }
                self.run.last = format!("routed to {provider}");
                self.status.provider = Some(provider);
                self.status.route_how = Some(how);
            }
            UiEvent::Checkpoint { turn, files, .. } => {
                // A turn's file state landed in the store: the rewind
                // affordance becomes real at this turn. The TUI names
                // the turn in its own status line rather than carrying
                // the store's counts, which the browser face renders.
                self.status.checkpoints.push(turn);
                self.run.checkpoint = Some(turn);
                self.run.last = format!("checkpoint \u{b7} turn {turn}");
                let _ = files;
            }
            UiEvent::Compacted {
                turns,
                tokens_before,
                tokens_after,
            } => {
                // A compaction is a fact about the run, not something the
                // model said: it flushes the streaming tail and lands as
                // its own block, exactly like a lane joining. The summary
                // it produced is in the next request, and the log's
                // `compaction` record carries it.
                self.flush_streaming();
                self.entries.push(Entry::Compaction(CompactionCard {
                    turns,
                    tokens_before,
                    tokens_after,
                }));
            }
            UiEvent::Memory { shape, facts } => {
                // What the run's assembled memory carried: the status bar
                // shows the shape and the fact count, the live half of the
                // `memory` record the log carries.
                self.run.stage = Stage::Act;
                self.run.facts = facts;
                self.run.shape = shape.clone();
                self.run.last = format!("memory \u{b7} {facts} facts");
                self.status.memory = Some((shape, facts));
            }
            UiEvent::CronFire {
                job,
                minute,
                catch_up,
                outcome,
                detail,
                session,
            } => {
                // A scheduled fire is a fact about the machine, not
                // something the model said: it flushes the streaming
                // tail and lands as its own block, the same family as a
                // lane joining or a compaction. The run lives in its own
                // session log — the conversation pane is where the
                // outcome is read, not where the run happened.
                self.flush_streaming();
                self.entries.push(Entry::Cron(CronCard {
                    job,
                    minute,
                    catch_up,
                    outcome,
                    detail,
                    session,
                }));
            }
            UiEvent::Subagent {
                goal,
                session,
                stage,
            } => {
                // The run card keeps one branch per child, rewritten by
                // each stage. The conversation's subagent tool card
                // below still owns the call's lifecycle dot.
                self.run.stage = Stage::Act;
                self.apply_branch(&goal, &session, &stage);
                // A delegated child run, reported while the parent's turn
                // waits for it. It lands on the `subagent` call's own tool
                // card — the same card the ordinary ToolCall event opened
                // and the ordinary ToolResult event closes — so the
                // conversation shows one card that goes pending, working,
                // done, with the goal on it and the outcome at the end.
                //
                // These events own the card's detail, never its lifecycle:
                // the status dot is the ordinary ToolCall/ToolResult pair's,
                // exactly as it is for every other tool. A child that stops
                // without its result arriving still leaves the outcome in
                // the preview, which is the one case the pair cannot cover.
                self.flush_streaming();
                let card = self
                    .entries
                    .iter_mut()
                    .rev()
                    .find_map(|entry| match entry {
                        Entry::Tool(card) if card.name == crate::tools::SUBAGENT_TOOL => Some(card),
                        _ => None,
                    })
                    .filter(|card| {
                        matches!(card.status, ToolStatus::Pending | ToolStatus::Approved)
                    });
                let detail = match stage {
                    SubagentStage::Spawned => None,
                    SubagentStage::Working { calls, tool } => Some(format!(
                        "child {session} \u{b7} {calls} call(s) \u{b7} last {tool}"
                    )),
                    SubagentStage::Done { outcome, turns } => Some(format!(
                        "child {session} \u{b7} {outcome} \u{b7} {turns} turn(s)"
                    )),
                };
                match card {
                    Some(card) => {
                        if card.summary.is_empty() {
                            card.summary = goal;
                        }
                        if let Some(detail) = detail {
                            card.preview = detail;
                        }
                    }
                    // No open card: a `Spawned` stage opens one, and any
                    // other stage still belongs on screen.
                    None => self.entries.push(Entry::Tool(ToolCard {
                        name: crate::tools::SUBAGENT_TOOL.to_string(),
                        summary: goal,
                        status: ToolStatus::Pending,
                        preview: detail.unwrap_or_default(),
                    })),
                }
            }
            UiEvent::Finished { stop_reason } => {
                self.flush_streaming();
                self.run.active = false;
                self.run.ended = Some(Instant::now());
                self.run.stage = Stage::Verify;
                self.run.stopping = false;
                self.run.stop_reason = Some(stop_reason.clone());
                self.run.last = format!("finished \u{b7} {stop_reason}");
                self.status.working = false;
                self.status.turn = 0;
                // The run controls were a view of a run that no longer
                // exists; leaving them over the composer would swallow
                // the next task as unhandled keys.
                if matches!(self.mode, Mode::Run | Mode::Steer) {
                    self.mode = Mode::Chat;
                }
                // A plan-mode run's final answer is the plan the owner
                // edits. Keep it in the state machine; `Ctrl-E` loads it
                // into the message box, and nothing sends it on its own.
                if self.run.plan {
                    self.plan_text = self.entries.iter().rev().find_map(|entry| match entry {
                        Entry::Assistant(text) if !text.trim().is_empty() => Some(text.clone()),
                        _ => None,
                    });
                }
            }
            // The harness asked for the report it was owed: the run is
            // still going, and the card says why another request is
            // being paid for.
            UiEvent::ReportRepair { attempt } => {
                self.run.active = true;
                self.run.stage = Stage::Think;
                self.run.last = format!("no report \u{b7} asked once ({attempt})");
            }
            UiEvent::Failed(message) => {
                self.flush_streaming();
                self.run.active = false;
                self.run.ended = Some(Instant::now());
                self.run.stage = Stage::Verify;
                self.run.stopping = false;
                self.status.working = false;
                self.status.turn = 0;
                if matches!(self.mode, Mode::Run | Mode::Steer) {
                    self.mode = Mode::Chat;
                }
                // The core reports the owner's stop through `failed`; only
                // the reason separates it from a failure. The card must
                // not call the owner's own decision an error.
                if message.contains("stopped by the owner") {
                    self.run.stop_reason = Some("stopped by you".to_string());
                    self.run.last = "stopped by you".to_string();
                } else {
                    self.run.stop_reason = Some("failed".to_string());
                    self.run.last = format!("failed \u{b7} {message}");
                    self.error = Some(message);
                }
            }
        }
    }

    /// Apply the agent thread's beside-the-stream reports.
    pub fn apply_status(&mut self, update: StatusUpdate) {
        let StatusUpdate::Usage(usage) = update;
        self.status.prompt_tokens += usage.prompt_tokens;
        self.status.completion_tokens += usage.completion_tokens;
    }

    /// Open the picker over `sessions` (oldest first, as listed).
    pub fn open_sessions(&mut self, sessions: Vec<SessionSummary>) {
        self.sessions = sessions;
        // The filter is the picker's own state and it does not survive a
        // close: a list that quietly hides rows on the next visit is a
        // list the owner cannot trust.
        self.session_filter = SessionFilter::All;
        self.selected = 0;
        self.mode = Mode::Sessions;
    }

    /// The picker's rows under the active filter, in the archive's own
    /// oldest-first order.
    pub fn session_rows(&self) -> Vec<&SessionSummary> {
        self.sessions
            .iter()
            .filter(|summary| self.session_filter.matches(summary))
            .collect()
    }

    /// Cycle the picker's filter and put the cursor back at the top:
    /// the row that was selected under the old filter may not exist.
    pub fn cycle_session_filter(&mut self) {
        self.session_filter = self.session_filter.next();
        self.selected = 0;
    }

    /// Open the checkpoint list. The rows are newest first, exactly as
    /// the store's own `list` returns them.
    pub fn open_checkpoints(&mut self, rows: Vec<crate::checkpoints::Snapshot>) {
        self.checkpoints_view = rows;
        self.selected = 0;
        self.mode = Mode::Checkpoints;
    }

    /// The turn the checkpoint cursor names, for the loop's diff read.
    pub fn checkpoint_turn(&self) -> Option<u64> {
        self.checkpoints_view
            .get(self.selected)
            .map(|snapshot| snapshot.turn)
    }

    /// Show one checkpoint's diff.
    pub fn open_diff(&mut self, turn: u64, rows: Vec<crate::checkpoints::FileDiff>) {
        self.diff_view = rows;
        self.diff_turn = turn;
        self.diff_file = 0;
        self.diff_hunk = 0;
        self.mode = Mode::Diff;
    }

    pub fn selected_diff_file(&self) -> Option<&crate::checkpoints::FileDiff> {
        self.diff_view.get(self.diff_file)
    }

    pub fn selected_hunk(&self) -> Option<&crate::checkpoints::Hunk> {
        self.selected_diff_file()
            .and_then(|file| file.hunks.get(self.diff_hunk))
    }

    /// Arm a rewind of the selected checkpoint.
    pub fn arm_rewind(&mut self) {
        let Some(snapshot) = self.checkpoints_view.get(self.selected) else {
            return;
        };
        self.confirm = Some(TrustConfirm {
            title: format!("rewind to turn {}", snapshot.turn),
            detail: format!(
                "restores {} file(s) from the checkpoint at {} and cuts the log after that turn",
                snapshot.files, snapshot.at
            ),
            action: TrustAction::Rewind(snapshot.turn),
            back: Mode::Checkpoints,
        });
        self.mode = Mode::Confirm;
    }

    /// Arm a hunk's rejection; a file with no alignable hunk (an addition
    /// or a removal) arms the whole-file form instead.
    pub fn arm_reject_hunk(&mut self) {
        let Some(file) = self.selected_diff_file() else {
            return;
        };
        let path = file.path.clone();
        let turn = self.diff_turn;
        let hunk = self.diff_hunk;
        if file.hunks.is_empty() {
            self.confirm = Some(TrustConfirm {
                title: format!("reject {path}"),
                detail: "puts the checkpoint's whole file back".to_string(),
                action: TrustAction::RejectFile { turn, file: path },
                back: Mode::Diff,
            });
            self.mode = Mode::Confirm;
            return;
        }
        if file.hunks.get(hunk).is_none() {
            return;
        }
        self.confirm = Some(TrustConfirm {
            title: format!("reject hunk {} of {path}", hunk + 1),
            detail: "puts this hunk's checkpoint lines back".to_string(),
            action: TrustAction::RejectHunk {
                turn,
                file: path,
                hunk,
            },
            back: Mode::Diff,
        });
        self.mode = Mode::Confirm;
    }

    /// Arm a whole file's rejection.
    pub fn arm_reject_file(&mut self) {
        let Some(file) = self.selected_diff_file() else {
            return;
        };
        self.confirm = Some(TrustConfirm {
            title: format!("reject {}", file.path),
            detail: "puts the checkpoint's whole file back".to_string(),
            action: TrustAction::RejectFile {
                turn: self.diff_turn,
                file: file.path.clone(),
            },
            back: Mode::Diff,
        });
        self.mode = Mode::Confirm;
    }

    /// Land the armed write: the effect the loop performs.
    pub fn land_trust(&mut self) -> Option<Effect> {
        let confirm = self.confirm.take()?;
        self.mode = confirm.back;
        match confirm.action {
            TrustAction::Rewind(turn) => {
                self.mode = Mode::Chat;
                Some(Effect::Rewind(turn))
            }
            TrustAction::RejectHunk { turn, file, hunk } => {
                Some(Effect::RejectHunk { turn, file, hunk })
            }
            TrustAction::RejectFile { turn, file } => Some(Effect::RejectFile { turn, file }),
        }
    }

    /// Abandon the armed write, with nothing touched.
    pub fn cancel_trust(&mut self) {
        if let Some(confirm) = self.confirm.take() {
            self.mode = confirm.back;
        }
    }

    /// Open one knowledge panel in its loading state. The returned
    /// generation stamps the read, so a panel the owner has already
    /// closed cannot receive a late answer.
    pub fn open_panel(&mut self, kind: PanelKind) -> u64 {
        self.panel_generation = self.panel_generation.wrapping_add(1);
        self.panel_kind = Some(kind);
        self.panel = None;
        self.panel_loading = true;
        self.panel_filter = 0;
        self.panel_note = None;
        self.selected = 0;
        self.mode = Mode::Panel;
        self.panel_generation
    }

    /// Land a panel read. A read from a superseded generation, or for a
    /// panel that is no longer open, is dropped.
    pub fn apply_panel(
        &mut self,
        generation: u64,
        kind: PanelKind,
        result: Result<PanelData, String>,
    ) {
        if generation != self.panel_generation || self.panel_kind != Some(kind) {
            return;
        }
        self.panel_loading = false;
        match result {
            Ok(data) => {
                self.panel = Some(data);
                self.panel_note = None;
                self.selected = 0;
            }
            Err(reason) => {
                self.panel_note = Some(reason.clone());
                self.error = Some(reason);
            }
        }
    }

    /// The open panel's filter words, for the title.
    pub fn panel_filter_label(&self) -> &'static str {
        let kind = self.panel_kind.unwrap_or(PanelKind::Memory);
        let words: &[&str] = match kind {
            PanelKind::Memory => &["all", "live", "protected", "retired"],
            PanelKind::Skills => &["all", "used", "unused", "proposals", "skipped"],
            PanelKind::Calibration => &["all", "fit", "informing"],
            PanelKind::Delegation => &["all", "live", "history"],
            PanelKind::Bots => &["service", "peers", "pending", "activity"],
        };
        words[self.panel_filter.min(words.len() - 1)]
    }

    /// Cycle the open panel's filter.
    pub fn cycle_panel_filter(&mut self) {
        let kind = self.panel_kind.unwrap_or(PanelKind::Memory);
        let len = match kind {
            PanelKind::Memory => 4,
            PanelKind::Skills => 5,
            PanelKind::Calibration => 3,
            PanelKind::Delegation => 3,
            PanelKind::Bots => 4,
        };
        self.panel_filter = (self.panel_filter + 1) % len;
        self.selected = 0;
    }

    /// The rows the panel's list shows, under its active filter.
    pub fn panel_rows(&self) -> Vec<PanelRow> {
        match &self.panel {
            Some(PanelData::Memory(snapshot)) => snapshot
                .facts
                .iter()
                .filter(|fact| match self.panel_filter {
                    1 => !fact.retired,
                    2 => fact.protected && !fact.retired,
                    3 => fact.retired,
                    _ => true,
                })
                .map(|fact| PanelRow {
                    label: fact.name.clone(),
                    meta: format!("{} \u{b7} {} recall(s)", fact.fact_type, fact.recall_count),
                    detail: format!("{}\n\n{}\n\n{}", fact.description, fact.body, fact.path),
                })
                .collect(),
            Some(PanelData::Skills(snapshot)) => match self.panel_filter {
                3 => snapshot
                    .proposals
                    .iter()
                    .map(|proposal| PanelRow {
                        label: proposal.name.clone(),
                        meta: proposal.kind.clone(),
                        detail: format!(
                            "{}\n\n{}\n\n{}",
                            proposal.description, proposal.evidence, proposal.path
                        ),
                    })
                    .collect(),
                4 => snapshot
                    .skips
                    .iter()
                    .map(|skip| PanelRow {
                        label: skip.name.clone(),
                        meta: "skipped".to_string(),
                        detail: skip.reason.clone(),
                    })
                    .collect(),
                _ => snapshot
                    .skills
                    .iter()
                    .filter(|skill| match self.panel_filter {
                        1 => skill.uses > 0,
                        2 => skill.uses == 0,
                        _ => true,
                    })
                    .map(|skill| PanelRow {
                        label: skill.name.clone(),
                        meta: format!(
                            "{} use{}",
                            skill.uses,
                            if skill.uses == 1 { "" } else { "s" }
                        ),
                        detail: format!("{}\n\n{}", skill.description, skill.path),
                    })
                    .collect(),
            },
            Some(PanelData::Calibration(data)) => data
                .cells
                .iter()
                .filter(|cell| match self.panel_filter {
                    1 => cell.fit.label() == "fit",
                    2 => cell.informs_failover(),
                    _ => true,
                })
                .map(|cell| PanelRow {
                    label: cell.provider.clone(),
                    meta: format!(
                        "{} \u{b7} {}",
                        if cell.workload.is_empty() {
                            "no shape"
                        } else {
                            cell.workload.as_str()
                        },
                        cell.fit.label()
                    ),
                    detail: format!(
                        "{} judged \u{b7} {} right \u{b7} {} wrong \u{b7} {} insufficient\n\
                         weighted wrong {:.2} \u{b7} insufficient {:.2} \u{b7} span {:.2}\n\
                         spend {:.4} \u{b7} gate {}",
                        cell.judged,
                        cell.right,
                        cell.wrong,
                        cell.insufficient,
                        cell.weighted_wrong_rate,
                        cell.weighted_insufficient_rate,
                        cell.confidence_span,
                        cell.spend_usd,
                        if cell.informs_failover() {
                            "informing"
                        } else {
                            "not informing"
                        }
                    ),
                })
                .collect(),
            Some(PanelData::Delegation(snapshot)) => {
                let mut rows = Vec::new();
                if self.panel_filter != 2 {
                    if let Some(live) = &snapshot.live {
                        push_node(&mut rows, live, 0);
                    }
                }
                if self.panel_filter != 1 {
                    for root in &snapshot.history {
                        push_node(&mut rows, root, 0);
                    }
                }
                rows
            }
            Some(PanelData::Bots(snapshot)) => self.bots_rows(snapshot),
            None => Vec::new(),
        }
    }

    /// The Bots panel's rows: four views of one document. The service's
    /// channels, the peers, the door, and the activity — the same four
    /// things the browser's surface cards answer, in the one list this
    /// panel has.
    ///
    /// Start and stop are not here, and the service view says so: a port
    /// belongs to the process that holds it, and this terminal is not
    /// that process. A panel that offered a button it could not press
    /// would be a panel that lies.
    fn bots_rows(&self, snapshot: &BotsSnapshot) -> Vec<PanelRow> {
        match self.panel_filter {
            1 => snapshot
                .channels
                .iter()
                .flat_map(|channel| {
                    channel.peers.iter().map(|peer| PanelRow {
                        label: if peer.name.is_empty() {
                            peer.id.clone()
                        } else {
                            peer.name.clone()
                        },
                        meta: format!(
                            "{} \u{b7} {}",
                            channel.name,
                            match peer.capability {
                                crate::channels::Capability::Task => "may act",
                                crate::channels::Capability::Observe => "may only listen",
                            }
                        ),
                        detail: format!(
                            "id       {}\npaired   {}\ncapability {}\n{}\n{}",
                            peer.id,
                            peer.created_at,
                            match peer.capability {
                                crate::channels::Capability::Task =>
                                    "may act \u{2014} it can hand fleet a task",
                                crate::channels::Capability::Observe =>
                                    "may only listen \u{2014} it is told things, and cannot task",
                            },
                            match &peer.callback {
                                Some(callback) =>
                                    format!("callback {callback}"),
                                None => "callback none stored".to_string(),
                            },
                            "Throw it out with `fleet channels revoke-webhook <id>`, or from the Bots surface."
                        ),
                    })
                })
                .collect(),
            2 => snapshot
                .channels
                .iter()
                .flat_map(|channel| {
                    channel.pending.iter().map(|peer| PanelRow {
                        label: if peer.name.is_empty() {
                            peer.id.clone()
                        } else {
                            peer.name.clone()
                        },
                        meta: format!(
                            "{} \u{b7} {} attempt{}",
                            channel.name,
                            peer.attempts,
                            if peer.attempts == 1 { "" } else { "s" }
                        ),
                        detail: format!(
                            "id         {}\nfirst seen {}\nlast seen  {}\n\nIt tried to reach a channel and is not paired. Let it in from the Bots surface, or with `fleet channels pair-webhook` (a webhook peer) or `fleet channels allow-telegram <chat-id>` (a chat).",
                            peer.id, peer.first_seen, peer.last_seen
                        ),
                    })
                })
                .collect(),
            3 => snapshot
                .activity
                .iter()
                .rev()
                .map(|line| PanelRow {
                    label: format!(
                        "{} {}",
                        line.at,
                        line.peer.clone().or_else(|| line.peer_id.clone()).unwrap_or_default()
                    ),
                    meta: line.kind.clone(),
                    detail: format!(
                        "at      {}\nchannel {}\npeer    {}\ntask    {}\nsession {}\ndetail  {}",
                        line.at,
                        line.channel,
                        line.peer.clone().unwrap_or_default(),
                        line.task.clone().unwrap_or_default(),
                        line.session.clone().unwrap_or_default(),
                        line.detail.clone().unwrap_or_default(),
                    ),
                })
                .collect(),
            _ => {
                let mut rows = Vec::new();
                match &snapshot.service {
                    Some(status) => {
                        for report in &status.channels {
                            rows.push(PanelRow {
                                label: report.name.clone(),
                                meta: format!(
                                    "{}{}",
                                    report.phase.label(),
                                    match report.port {
                                        Some(port) => format!(" \u{b7} 127.0.0.1:{port}"),
                                        None => String::new(),
                                    }
                                ),
                                detail: format!(
                                    "state      {}{}\nsince      {}{}\nservice    pid {} \u{b7} started {}\n           beating every {}s{}\n\nStart and stop live in the face that hosts the service (`fleet gui --channels`, or `channels.enabled` in the settings). This terminal reads the same document that face writes; it cannot hold the port itself.",
                                    report.phase.label(),
                                    if snapshot.stale { " (last report)" } else { "" },
                                    report.since,
                                    match &report.error {
                                        Some(error) => format!("\nerror      {error}"),
                                        None => String::new(),
                                    },
                                    status.pid,
                                    status.started,
                                    status.heartbeat_secs,
                                    if snapshot.stale {
                                        " \u{2014} the heartbeat has stopped, so this is the last report of a process that is no longer running"
                                    } else {
                                        ""
                                    },
                                ),
                            });
                        }
                        if status.channels.is_empty() {
                            rows.push(PanelRow {
                                label: "no channels".to_string(),
                                meta: status.pid.to_string(),
                                detail: "The service is running but hosts no channel in this build.".to_string(),
                            });
                        }
                    }
                    None => rows.push(PanelRow {
                        label: "no service".to_string(),
                        meta: String::new(),
                        detail: "No channel service has run in this home. `channels.enabled` in the settings, or `fleet gui --channels`, turns it on.".to_string(),
                    }),
                }
                rows
            }
        }
    }

    /// Open the settings pane over the conversation, with the states the
    /// writer reads. The home rides the pane because a write needs it,
    /// and a pane that could show a value it could not change would be a
    /// lie about what is configurable here. The two list-shaped surfaces
    /// are read beside the scalars, so the pane carries every surface the
    /// CLI's `fleet settings` addresses.
    pub fn open_settings(&mut self, home: std::path::PathBuf) {
        self.settings = crate::settings::states(&home);
        self.settings_lists = crate::settings::list_states(&home);
        self.settings_home = Some(home);
        self.editing = None;
        self.settings_removal = None;
        self.settings_note = None;
        self.selected = 0;
        self.mode = Mode::Settings;
    }

    /// Every row the settings pane can select, in the order it renders
    /// them: the scalar knobs, then the provider cards, then the workload
    /// cards.
    pub fn settings_rows(&self) -> Vec<SettingsRow> {
        let mut rows: Vec<SettingsRow> =
            (0..self.settings.len()).map(SettingsRow::Scalar).collect();
        let mut providers = 0usize;
        let mut workloads = 0usize;
        for entry in &self.settings_lists {
            match entry.kind {
                crate::settings::ListKind::Provider => {
                    rows.push(SettingsRow::Provider(providers));
                    providers += 1;
                }
                crate::settings::ListKind::Workload => {
                    rows.push(SettingsRow::Workload(workloads));
                    workloads += 1;
                }
            }
        }
        rows
    }

    /// The `index`-th entry of one list, in the order `list_states`
    /// returned them.
    fn list_entry(
        &self,
        kind: crate::settings::ListKind,
        index: usize,
    ) -> Option<&crate::settings::ListState> {
        self.settings_lists
            .iter()
            .filter(|entry| entry.kind == kind)
            .nth(index)
    }

    /// Begin editing the selected row. A scalar's buffer starts as its
    /// current value, so an edit is a change rather than a retype — and
    /// a value the environment owns starts empty, because typing over it
    /// would be writing a file the env var shadows. A list entry seeds
    /// with the writer's own shape: the entry's JSON under
    /// `provider.add:`, its comma-joined candidates under
    /// `workload.set:` — the same buffer the CLI's key would carry, so
    /// committing routes through the writer's own function.
    pub fn edit_selected_row(&mut self) {
        let Some(row) = self.settings_rows().get(self.selected).copied() else {
            return;
        };
        let (key, seed) = match row {
            SettingsRow::Scalar(index) => {
                let Some(state) = self.settings.get(index) else {
                    return;
                };
                let seed = if state.from_env {
                    String::new()
                } else {
                    state.value.clone().unwrap_or_default()
                };
                (state.key.to_string(), seed)
            }
            SettingsRow::Provider(index) => {
                let Some(entry) = self.list_entry(crate::settings::ListKind::Provider, index)
                else {
                    return;
                };
                (format!("provider.add:{}", entry.seed), entry.seed.clone())
            }
            SettingsRow::Workload(index) => {
                let Some(entry) = self.list_entry(crate::settings::ListKind::Workload, index)
                else {
                    return;
                };
                (format!("workload.set:{}", entry.key), entry.seed.clone())
            }
        };
        self.editing = Some((key, seed));
        self.settings_note = None;
        self.mode = Mode::SettingsEdit;
    }

    /// The editor's title, named for a human. The key rides the writer's
    /// whole shape — a provider entry's JSON, a workload's name — so the
    /// title names the entry rather than repeating its JSON back at the
    /// owner.
    pub fn editing_label(&self) -> String {
        let Some((key, _)) = self.editing.as_ref() else {
            return String::new();
        };
        if let Some(entry) = key.strip_prefix("provider.add:") {
            let id = serde_json::from_str::<serde_json::Value>(entry)
                .ok()
                .and_then(|value| {
                    value
                        .get("id")
                        .and_then(|id| id.as_str())
                        .map(str::to_string)
                });
            return match id {
                Some(id) => format!("provider {id}"),
                None => "provider entry".to_string(),
            };
        }
        if let Some(name) = key.strip_prefix("workload.set:") {
            return format!("workload {name}");
        }
        key.clone()
    }

    /// Commit the setting being edited, through the same writer the CLI
    /// calls. The pane then re-reads every state and both lists, so the
    /// card the owner is looking at is the resolved value rather than
    /// what they typed — a clamped similarity or a rejected word shows
    /// its real result.
    pub fn commit_setting(&mut self) {
        let Some((key, buffer)) = self.editing.take() else {
            return;
        };
        self.write_setting(&key, &buffer);
        self.mode = Mode::Settings;
    }

    /// One write through the writer the CLI calls. The key decides the
    /// branch: a plain key goes through `settings::set`, and the four
    /// list keys go through the writer's own list functions — never a
    /// pane-local path, so the two surfaces cannot drift. A refusal
    /// lands back as the note, naming the rule that stopped it, and a
    /// success re-reads the scalars and both lists so the cards show the
    /// resolved result.
    fn write_setting(&mut self, key: &str, buffer: &str) {
        let Some(home) = self.settings_home.clone() else {
            return;
        };
        let outcome = if key.starts_with("provider.add:") {
            // The buffer is the entry's own JSON, which is what
            // `set_provider` parses; the key carries the same JSON so the
            // editor's title can name the entry.
            crate::settings::set_provider(&home, buffer)
                .map(|id| format!("provider `{id}` written"))
        } else if let Some(id) = key.strip_prefix("provider.remove:") {
            // The pane always formats a non-empty id into the key (the
            // confirm passes an empty buffer), so the fallback is the
            // CLI's key shape preserved — the value rides the argument
            // there — rather than a live path.
            let id = if id.is_empty() { buffer } else { id };
            crate::settings::remove_provider(&home, id, false).map(|()| {
                format!(
                    "provider `{id}`'s override removed — it reverts to the compiled-in default"
                )
            })
        } else if let Some(name) = key.strip_prefix("workload.set:") {
            let candidates: Vec<String> = buffer
                .split(',')
                .map(|candidate| candidate.trim().to_string())
                .filter(|candidate| !candidate.is_empty())
                .collect();
            crate::settings::set_workload(&home, name, &candidates)
                .map(|()| format!("workload `{name}` written"))
        } else if let Some(name) = key.strip_prefix("workload.remove:") {
            // As above: the pane's key always carries the name; the
            // empty case is the CLI's key shape preserved.
            let name = if name.is_empty() { buffer } else { name };
            crate::settings::remove_workload(&home, name, false).map(|()| {
                format!(
                    "workload `{name}`'s preference removed — it reverts to the compiled-in table"
                )
            })
        } else {
            crate::settings::set(&home, key, buffer).map(|()| "written".to_string())
        };
        match outcome {
            Ok(message) => {
                self.settings_note = Some(message);
                self.settings = crate::settings::states(&home);
                self.settings_lists = crate::settings::list_states(&home);
                // A removal shrinks the rows; a selection left past
                // the end renders no cursor and answers nothing — the
                // clamp keeps the pane live, the same one
                // `move_selection` applies.
                let rows = self.settings_rows().len();
                self.selected = self.selected.min(rows.saturating_sub(1));
            }
            Err(error) => {
                // A refusal is a value the owner reads: it names the rule
                // that stopped the edit, and nothing changed.
                self.settings_note = Some(error);
            }
        }
    }

    /// Abandon the edit, leaving the file untouched.
    pub fn cancel_setting(&mut self) {
        self.editing = None;
        self.settings_note = None;
        self.mode = Mode::Settings;
    }

    /// Arm the destructive confirmation for the selected list entry. The
    /// pane never passes `force`: the CLI's flag is a shell word, and a
    /// pane has nowhere to put one — so the question is what stands in
    /// for it, and the writer's own compiled-in rule still answers when
    /// the removal lands. Each class is named in the question, because
    /// an owner confirming a removal deserves to know which kind it is.
    pub fn arm_removal(&mut self) {
        let Some(row) = self.settings_rows().get(self.selected).copied() else {
            return;
        };
        let entry = match row {
            SettingsRow::Scalar(_) => {
                // A knob has no removal: it is cleared by emptying its
                // value, which is an ordinary edit.
                self.settings_note =
                    Some("a knob is cleared by emptying its value, not removed".to_string());
                return;
            }
            SettingsRow::Provider(index) => {
                self.list_entry(crate::settings::ListKind::Provider, index)
            }
            SettingsRow::Workload(index) => {
                self.list_entry(crate::settings::ListKind::Workload, index)
            }
        };
        let Some(entry) = entry else { return };
        let (key, detail) = match (entry.kind, entry.reverts) {
            (crate::settings::ListKind::Provider, true) => (
                format!("provider.remove:{}", entry.key),
                format!(
                    "remove provider `{}`'s override — the compiled-in default comes back",
                    entry.key
                ),
            ),
            (crate::settings::ListKind::Provider, false) => (
                format!("provider.remove:{}", entry.key),
                format!(
                    "remove provider `{}` — this removes it from every walk; the compiled-in default does not come back",
                    entry.key
                ),
            ),
            (crate::settings::ListKind::Workload, true) => (
                format!("workload.remove:{}", entry.key),
                format!(
                    "remove workload `{}`'s preference — the compiled-in table's candidates come back",
                    entry.key
                ),
            ),
            (crate::settings::ListKind::Workload, false) => (
                format!("workload.remove:{}", entry.key),
                format!(
                    "remove workload `{}` — this leaves the workload with no candidates at all; the compiled-in table does not come back",
                    entry.key
                ),
            ),
        };
        self.settings_removal = Some(SettingsRemoval { key, detail });
        self.settings_note = None;
        self.mode = Mode::SettingsConfirm;
    }

    /// The armed removal, confirmed. It lands through the writer's own
    /// branch with `force` left false: the pane's question is the
    /// confirmation, and the writer's compiled-in rule decides whether
    /// the edit is a revert or a refusal. Either way the note names what
    /// happened.
    fn confirm_removal(&mut self) {
        let Some(removal) = self.settings_removal.take() else {
            return;
        };
        self.write_setting(&removal.key, "");
        self.mode = Mode::Settings;
    }

    /// The armed removal, abandoned: nothing is written, and the note
    /// says so — an abandoned question is not a silent one.
    fn abandon_removal(&mut self) {
        self.settings_removal = None;
        self.settings_note = Some("nothing removed".to_string());
        self.mode = Mode::Settings;
    }

    /// The key of the setting being edited, or `None` outside the editor.
    pub fn editing_key(&self) -> Option<&str> {
        self.editing.as_ref().map(|(key, _)| key.as_str())
    }

    /// The buffer of the setting being edited, or `None` outside it.
    pub fn editing_buffer(&self) -> Option<&str> {
        self.editing.as_ref().map(|(_, buffer)| buffer.as_str())
    }

    /// Push one character into the edit buffer.
    pub fn edit_push(&mut self, character: char) {
        if let Some((_, buffer)) = self.editing.as_mut() {
            buffer.push(character);
        }
    }

    /// Drop the character before the buffer's end.
    pub fn edit_pop(&mut self) {
        if let Some((_, buffer)) = self.editing.as_mut() {
            buffer.pop();
        }
    }

    /// Open the explanation card over the conversation, with the lines
    /// the headless card rendered. The lines are the same text
    /// `fleet explain` prints — the surface reads the file the beat
    /// wrote, so the two cannot drift.
    pub fn open_explain(&mut self, lines: Vec<String>) {
        self.explain = Some(lines);
        self.selected = 0;
        self.mode = Mode::Explain;
    }

    /// Open the lane health view. The rows arrive asynchronously from the
    /// prober; until they do the view says it is probing. Returns the probe
    /// generation this view belongs to, which the prober's rows must carry
    /// to be applied.
    pub fn open_lanes(&mut self) -> u64 {
        self.lanes_probe = self.lanes_probe.wrapping_add(1);
        self.lanes = Vec::new();
        self.lanes_loading = true;
        self.selected = 0;
        self.mode = Mode::Lanes;
        self.lanes_probe
    }

    /// The prober's rows for probe `probe` landed. Rows from a probe the
    /// user has already closed are dropped: the mode alone cannot tell
    /// them from the open view's own rows, because a stale probe's rows
    /// arrive while the fresh modal is open too.
    pub fn set_lanes(&mut self, probe: u64, lanes: Vec<LaneHealth>) {
        if probe != self.lanes_probe {
            return;
        }
        self.lanes = lanes;
        self.lanes_loading = false;
        self.selected = 0;
    }

    /// Cycle the right pane to the next tab. A board tab points the
    /// refresher at the board it shows; the run tab asks for no read.
    pub fn cycle_pane(&mut self) {
        self.pane = self.pane.next();
        if let Some(board) = self.pane.board() {
            self.boards.active = board;
        }
    }

    /// The failover gate read at launch. `None` is a home whose config
    /// could not be read, which the card says rather than guessing on.
    pub fn set_gate(&mut self, informs: Option<bool>) {
        self.run.gate = informs;
    }

    /// Open the command palette. Every open starts empty: a filter that
    /// survived the last command would hide the command the owner wants.
    pub fn open_palette(&mut self) {
        self.palette_query.clear();
        self.palette_selected = 0;
        self.mode = Mode::Palette;
    }

    /// Every palette command, in the order the palette shows them when
    /// the filter is empty. Direct keys stay for muscle memory; this is
    /// how the owner finds a surface they do not have a key for.
    pub fn palette_items(&self) -> Vec<PaletteItem> {
        vec![
            PaletteItem {
                label: "run controls",
                detail: "stop, steer, branches",
                action: Action::OpenRunControls,
            },
            PaletteItem {
                label: "run pane",
                detail: "the live loop",
                action: Action::OpenPane(Pane::Run),
            },
            PaletteItem {
                label: "router",
                detail: "the declared chain and the day's rows",
                action: Action::OpenPane(Pane::Router),
            },
            PaletteItem {
                label: "brain",
                detail: "the derived state",
                action: Action::OpenPane(Pane::Brain),
            },
            PaletteItem {
                label: "tower",
                detail: "the projects and their evidence",
                action: Action::OpenPane(Pane::Tower),
            },
            PaletteItem {
                label: "memory",
                detail: "the corpus, its tiers, and the day's hits",
                action: Action::OpenPanel(PanelKind::Memory),
            },
            PaletteItem {
                label: "skills",
                detail: "the library, proposals, and skips",
                action: Action::OpenPanel(PanelKind::Skills),
            },
            PaletteItem {
                label: "calibration",
                detail: "the judged fit cells",
                action: Action::OpenPanel(PanelKind::Calibration),
            },
            PaletteItem {
                label: "delegation",
                detail: "the child runs and the graph",
                action: Action::OpenPanel(PanelKind::Delegation),
            },
            PaletteItem {
                label: "bots",
                detail: "the channels, who is paired, and what they asked for",
                action: Action::OpenPanel(PanelKind::Bots),
            },
            PaletteItem {
                label: "sessions",
                detail: "the archive; enter resumes",
                action: Action::OpenSessions,
            },
            PaletteItem {
                label: "lanes",
                detail: "the vendored lanes' health",
                action: Action::OpenLanes,
            },
            PaletteItem {
                label: "checkpoints",
                detail: "rewind to a turn",
                action: Action::OpenCheckpoints,
            },
            PaletteItem {
                label: "plan mode",
                detail: "toggle the next run's plan bit",
                action: Action::TogglePlan,
            },
            PaletteItem {
                label: "stop the run",
                detail: "write the run's control file",
                action: Action::StopRun,
            },
            PaletteItem {
                label: "steer the run",
                detail: "send one line to the run",
                action: Action::OpenSteer,
            },
            PaletteItem {
                label: "settings",
                detail: "every configurable surface",
                action: Action::OpenSettings,
            },
            PaletteItem {
                label: "explain",
                detail: "today's explanation card",
                action: Action::OpenExplain,
            },
            PaletteItem {
                label: "quit",
                detail: "leave fleet",
                action: Action::Quit,
            },
        ]
    }

    /// The commands the current filter keeps, in display order.
    pub fn palette_filtered(&self) -> Vec<PaletteItem> {
        let asked = self.palette_query.trim().to_ascii_lowercase();
        self.palette_items()
            .into_iter()
            .filter(|item| {
                asked.is_empty()
                    || format!("{} {}", item.label, item.detail)
                        .to_ascii_lowercase()
                        .contains(&asked)
            })
            .collect()
    }

    /// Move the palette cursor, clamped to the filtered list.
    pub fn palette_move(&mut self, delta: i32) {
        let len = self.palette_filtered().len();
        if len == 0 {
            self.palette_selected = 0;
            return;
        }
        let last = len as i32 - 1;
        self.palette_selected = (self.palette_selected as i32 + delta).clamp(0, last) as usize;
    }

    /// Commit the highlighted command: close the palette and hand the
    /// action back to the event loop, which performs the reads the
    /// state machine cannot (sessions, lanes, settings).
    pub fn palette_pick(&mut self) -> Option<Action> {
        let items = self.palette_filtered();
        let action = items.get(self.palette_selected)?.action;
        self.mode = Mode::Chat;
        Some(action)
    }

    /// Land one finished board read. A refusal keeps the last snapshot
    /// and names the reason; a success clears both.
    pub fn apply_board(&mut self, kind: BoardKind, outcome: Result<BoardData, String>) {
        self.boards.apply(kind, outcome);
    }

    /// Show an approval prompt; the responder stays with the event loop.
    pub fn show_approval(&mut self, name: &str, summary: &str) {
        self.approval = Some(ApprovalPrompt {
            name: name.to_string(),
            summary: summary.to_string(),
        });
        self.mode = Mode::Approval;
    }

    /// Replace the conversation with a resumed session's log, exactly like
    /// `--session` seeds a headless run.
    pub fn seed(&mut self, records: &[Record]) {
        self.entries = records.iter().filter_map(entry_of).collect();
        self.streaming.clear();
        self.error = None;
        self.scrolled_back = 0;
        self.following = true;
        self.status.turn = 0;
        self.status.working = false;
        // The run card describes the run this view was watching; a
        // resumed log replaces that view, so the card comes back idle.
        // The plan toggle and the gate are the next run's, and stay.
        self.run.active = false;
        self.run.ended = None;
        self.run.started = None;
        self.run.stage = Stage::Route;
        self.run.turn = 0;
        self.run.tool.clear();
        self.run.calls = 0;
        self.run.facts = 0;
        self.run.shape.clear();
        self.run.context = 0;
        self.run.lanes = 0;
        self.run.checkpoint = None;
        self.run.branches.clear();
        self.run.last.clear();
        self.run.stop_reason = None;
        self.run.stopping = false;
        self.run.tokens.clear();
        self.plan_text = None;
        // The counters belong to the view being replaced, not to the log
        // being replayed: the resumed conversation's earlier runs never
        // streamed through here, so the count restarts at zero.
        self.status.prompt_tokens = 0;
        self.status.completion_tokens = 0;
        // The memory fact belongs to the view being replaced too: it
        // describes the last run this window assembled, and a resumed
        // log's earlier runs assembled theirs elsewhere.
        self.status.memory = None;
        self.clamp_cursor();
    }

    /// The conversation's text, streaming tail included, for rendering.
    pub fn conversation_lines(&self) -> Vec<Entry> {
        let mut lines = self.entries.clone();
        if !self.streaming.is_empty() {
            lines.push(Entry::Assistant(self.streaming.clone()));
        }
        lines
    }

    /// Type one character at the cursor, honoring the input cap.
    pub fn push_input(&mut self, character: char) {
        if self.input.chars().count() < MAX_INPUT {
            let byte = self.char_index_to_byte(self.cursor);
            self.input.insert(byte, character);
            self.cursor += 1;
        }
    }

    /// Insert pasted text at the cursor without submitting it: a paste is
    /// typing, never an Enter. Terminals disagree on the line ending they
    /// deliver inside a bracketed paste — `\n`, `\r\n`, or a bare `\r`
    /// (tmux's choice) — and all of them mean a newline here. One
    /// trailing newline is stripped, the one terminals append to the
    /// payload, and the cap still applies so a runaway paste truncates
    /// instead of growing forever.
    pub fn paste(&mut self, text: &str) {
        let text = text.replace("\r\n", "\n").replace('\r', "\n");
        let text = text.strip_suffix('\n').unwrap_or(&text);
        let room = MAX_INPUT.saturating_sub(self.input.chars().count());
        let inserted: String = text.chars().take(room).collect();
        let byte = self.char_index_to_byte(self.cursor);
        self.input.insert_str(byte, &inserted);
        self.cursor += inserted.chars().count();
    }

    /// Delete the character before the cursor.
    pub fn backspace(&mut self) {
        if self.cursor == 0 {
            return;
        }
        let byte = self.char_index_to_byte(self.cursor - 1);
        self.cursor -= 1;
        self.input.remove(byte);
    }

    /// Delete the character at the cursor, pulling the tail left.
    pub fn delete(&mut self) {
        if self.cursor >= self.input.chars().count() {
            return;
        }
        let byte = self.char_index_to_byte(self.cursor);
        self.input.remove(byte);
    }

    /// Delete the word before the cursor: the run of non-space characters
    /// plus the spaces in front of it, the way a shell's alt+backspace
    /// eats a word.
    pub fn delete_word(&mut self) {
        let chars: Vec<char> = self.input.chars().collect();
        let mut start = self.cursor;
        while start > 0 && chars[start - 1] == ' ' {
            start -= 1;
        }
        while start > 0 && chars[start - 1] != ' ' {
            start -= 1;
        }
        let from = self.char_index_to_byte(start);
        let to = self.char_index_to_byte(self.cursor);
        self.input.replace_range(from..to, "");
        self.cursor = start;
    }

    /// Move the cursor one character left, stopping at the start.
    pub fn cursor_left(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    /// Move the cursor one character right, stopping at the end.
    pub fn cursor_right(&mut self) {
        self.cursor = (self.cursor + 1).min(self.input.chars().count());
    }

    /// Keep the cursor inside the input after anything else changes it.
    fn clamp_cursor(&mut self) {
        self.cursor = self.cursor.min(self.input.chars().count());
    }

    /// The byte offset of the `index`-th character.
    fn char_index_to_byte(&self, index: usize) -> usize {
        self.input
            .char_indices()
            .nth(index)
            .map(|(byte, _)| byte)
            .unwrap_or(self.input.len())
    }

    fn submit(&mut self) -> Option<Effect> {
        let text = self.input.trim().to_string();
        if text.is_empty() {
            return None;
        }
        self.input.clear();
        self.cursor = 0;
        // The headless chat's `/exit` and `/quit` keep their meaning here.
        if text == "/exit" || text == "/quit" {
            self.should_quit = true;
            return Some(Effect::Quit);
        }
        // A new run resets the run card but keeps the composer's plan
        // toggle and the gate the launch read: both belong to the next
        // run, not to the one that just ended.
        let plan = self.run.plan;
        let gate = self.run.gate;
        self.run = RunState {
            plan,
            active: true,
            started: Some(Instant::now()),
            gate,
            last: "the task is with the agent".to_string(),
            ..RunState::default()
        };
        self.pane = Pane::Run;
        self.plan_text = None;
        self.error = None;
        self.status.working = true;
        self.status.turn = 0;
        self.entries.push(Entry::User(text.clone()));
        self.scrolled_back = 0;
        self.following = true;
        Some(Effect::Run { task: text, plan })
    }

    /// Stop the run in flight through its control file. The session id
    /// is the file's own key, so a run that has not created its log yet
    /// is a named refusal rather than a file nothing reads.
    fn stop_run(&mut self) -> Option<Effect> {
        if !self.run.active {
            self.error = Some("no run is in flight to stop".to_string());
            return None;
        }
        if self.status.session_id.is_empty() {
            self.error = Some(
                "this run has no session log yet; there is nothing beside it to write".to_string(),
            );
            return None;
        }
        if self.run.stopping {
            return None;
        }
        self.run.stopping = true;
        self.run.last = "stopping \u{b7} the run ends at its next step".to_string();
        Some(Effect::Control { steer: None })
    }

    /// Send the one-line steer. A steer rides one turn, and the control
    /// file is consumed by the loop; a second one is a second write.
    fn send_steer(&mut self) -> Option<Effect> {
        let text = self.steer_input.trim().to_string();
        if !self.run.active || text.is_empty() {
            self.mode = Mode::Run;
            return None;
        }
        if self.status.session_id.is_empty() {
            self.error = Some(
                "this run has no session log yet; there is nothing beside it to write".to_string(),
            );
            return None;
        }
        let preview: String = text.chars().take(60).collect();
        self.steer_input.clear();
        self.run.last = format!("steer sent \u{b7} {preview}");
        self.mode = Mode::Run;
        Some(Effect::Control { steer: Some(text) })
    }

    fn pick_session(&mut self) -> Option<Effect> {
        // Resuming mid-run would mix the in-flight run's tokens into the
        // seeded view: the log stays correct, the counters would not.
        if self.status.working {
            self.error = Some("wait for the run to finish before resuming a session".to_string());
            return None;
        }
        let picked = self.session_rows().get(self.selected)?.id.clone();
        self.mode = Mode::Chat;
        Some(Effect::Resume(picked))
    }

    fn move_selection(&mut self, delta: i32, len: usize) {
        if len == 0 {
            self.selected = 0;
            return;
        }
        let last = len as i32 - 1;
        let next = self.selected as i32 + delta;
        self.selected = next.clamp(0, last) as usize;
    }

    fn scroll_by(&mut self, delta: i32) {
        if delta > 0 {
            self.scrolled_back = self.scrolled_back.saturating_add(delta as usize);
            self.following = false;
        } else {
            self.scrolled_back = self.scrolled_back.saturating_sub((-delta) as usize);
            self.following = self.scrolled_back == 0;
        }
    }

    /// Move the streaming tail into the conversation as its own block.
    fn flush_streaming(&mut self) {
        if !self.streaming.is_empty() {
            self.entries
                .push(Entry::Assistant(std::mem::take(&mut self.streaming)));
        }
    }

    /// One delegated child's branch, rewritten by each stage. The list
    /// is capped: a run can spawn many children, and the run card shows
    /// the newest few rather than growing without bound.
    fn apply_branch(&mut self, goal: &str, session: &str, stage: &SubagentStage) {
        let (kind, calls, tool, outcome, turns) = match stage {
            SubagentStage::Spawned => ("spawned", 0, String::new(), String::new(), 0),
            SubagentStage::Working { calls, tool } => {
                ("working", *calls, tool.clone(), String::new(), 0)
            }
            SubagentStage::Done { outcome, turns } => {
                ("done", 0, String::new(), outcome.clone(), *turns)
            }
        };
        let suffix = if tool.is_empty() {
            String::new()
        } else {
            format!(" \u{b7} {tool}")
        };
        self.run.last = format!("delegation \u{b7} {kind}{suffix}");
        if let Some(branch) = self
            .run
            .branches
            .iter_mut()
            .find(|branch| branch.session == session)
        {
            branch.stage = kind.to_string();
            branch.calls = calls;
            branch.tool = tool;
            branch.outcome = outcome;
            branch.turns = turns;
        } else {
            self.run.branches.push(Branch {
                session: session.to_string(),
                goal: goal.to_string(),
                stage: kind.to_string(),
                calls,
                tool,
                outcome,
                turns,
            });
            if self.run.branches.len() > 32 {
                self.run.branches.remove(0);
            }
        }
    }

    /// Flip the newest open tool card to approved, so the run's progress
    /// is visible while the call executes.
    fn mark_pending_approved(&mut self) {
        if let Some(Entry::Tool(card)) = self
            .entries
            .iter_mut()
            .rev()
            .find(|entry| matches!(entry, Entry::Tool(_)))
        {
            if matches!(card.status, ToolStatus::Pending | ToolStatus::Approved) {
                card.status = ToolStatus::Approved;
            }
        }
    }
}

/// Flatten one delegation tree into selectable rows, keeping the depth
/// as indentation so the panel reads as a tree rather than a list.
fn push_node(rows: &mut Vec<PanelRow>, node: &crate::boards::DelegationNode, depth: usize) {
    let indent = "  ".repeat(depth);
    let turn = node
        .turns
        .map(|turns| format!(" \u{b7} {turns} turn(s)"))
        .unwrap_or_default();
    rows.push(PanelRow {
        label: format!("{indent}{}", short_session_id(&node.id)),
        meta: node.status.clone(),
        detail: format!(
            "{}{}\n\nmodel {}\nstop {}{}",
            node.goal.as_deref().unwrap_or("no goal recorded"),
            if depth == 0 { "" } else { " (child)" },
            node.model,
            node.stop_reason.as_deref().unwrap_or("\u{2014}"),
            turn
        ),
    });
    for child in &node.children {
        push_node(rows, child, depth + 1);
    }
}

fn short_session_id(id: &str) -> String {
    id.rsplit_once('Z')
        .map(|(head, tail)| format!("{}Z{}", &head[head.len().saturating_sub(5)..], tail))
        .unwrap_or_else(|| id.chars().take(20).collect())
}

/// Step a cursor and clamp it to a list of `len` items.
fn clamp_step(current: usize, delta: i32, len: usize) -> usize {
    if len == 0 {
        0
    } else {
        (current as i32 + delta).clamp(0, len as i32 - 1) as usize
    }
}

/// One log record as a conversation block. Assistant turns that carried
/// neither text nor calls are transport noise, not conversation.
fn entry_of(record: &Record) -> Option<Entry> {
    match record {
        // Bookkeeping: a rewind marks a cut the pane's own seed applies,
        // and a plan rides the assistant message beside it.
        Record::Rewind { .. } => None,
        Record::Plan { .. } => None,
        // Bookkeeping: the standing goal rides the prompt, and the run
        // card is where a TUI reads it.
        Record::Goal { .. } => None,
        // Bookkeeping: the channel a peer arrived on is not a
        // conversation block.
        Record::Channel { .. } => None,
        // Bookkeeping: how a call was resolved is audit material, not a
        // conversation block.
        Record::ToolResolution { .. } => None,
        // Bookkeeping: a server that did not join is run metadata.
        Record::LaneFailed { .. } => None,
        // The model's thinking is a card the face draws first; the TUI's
        // own Thought block is a later increment, so a resumed log's
        // reasoning does not become a pane entry here.
        Record::Reasoning { .. } => None,
        Record::User { content } => Some(Entry::User(content.clone())),
        Record::Assistant {
            content,
            tool_calls,
        } => {
            let text = content.clone().unwrap_or_default();
            if text.trim().is_empty() && tool_calls.is_empty() {
                None
            } else {
                Some(Entry::Assistant(text))
            }
        }
        Record::ToolResult {
            name,
            output,
            approved,
            ..
        } => Some(Entry::Tool(ToolCard {
            name: name.clone(),
            summary: String::new(),
            status: if *approved {
                ToolStatus::Done
            } else {
                ToolStatus::Denied
            },
            preview: crate::agent::preview(output, 80),
        })),
        // A resumed conversation shows the lanes its earlier runs joined,
        // so the tool cards that follow read as the lane's own.
        Record::LaneTools { lane, tools } => Some(Entry::Lane(LaneCard {
            lane: lane.clone(),
            tool_count: tools.len(),
        })),
        // A `compaction` record replays to one card: the turns it folded,
        // rebuilt from the range it names. The summary itself is not the
        // conversation's text — it stands in for turns the log still
        // holds — so the card names the fold rather than repeating it.
        Record::Compaction {
            first_turn,
            last_turn,
            tokens_before,
            tokens_after,
            ..
        } => Some(Entry::Compaction(CompactionCard {
            turns: crate::compaction::turns_in_range(*first_turn, *last_turn),
            tokens_before: *tokens_before,
            tokens_after: *tokens_after,
        })),
        // The skills record is bookkeeping about the system prompt, like
        // `start` and `end`: it replays to no conversation block. A
        // skill's use shows as the ordinary tool card its `tool_result`
        // seeded — there is deliberately no separate skills surface in
        // the TUI this slice.
        Record::Skills { .. } => None,
        // The `memory` record is the same shape one seam over: the
        // shape and fact names the assembled block carried, recorded
        // because the prompt is dynamic. The run's memory line lives in
        // the status bar, not in the conversation.
        Record::Memory { .. } => None,
        Record::Start { .. } | Record::End { .. } => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app() -> App {
        App::new("deepseek-flash", "session-1")
    }

    /// A scratch home the settings pane can read and write.
    fn settings_home(tag: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("fleet-tui-settings-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// The pane opens over every setting, moves its selection, and
    /// closes — the menu shape, over the writer's own states.
    #[test]
    fn the_settings_pane_opens_moves_and_closes() {
        let home = settings_home("menu");
        let mut app = app();
        app.open_settings(home.clone());
        assert_eq!(app.mode, Mode::Settings);
        assert!(
            !app.settings.is_empty(),
            "the pane carries the writer's states"
        );
        assert_eq!(app.selected, 0);
        app.apply(Action::Move(1));
        assert_eq!(app.selected, 1);
        // The move is clamped at the list's end rather than wrapping.
        for _ in 0..app.settings.len() + 4 {
            app.apply(Action::Move(1));
        }
        assert_eq!(app.selected, app.settings.len() - 1);
        app.apply(Action::Move(-99));
        assert_eq!(app.selected, 0);
        app.apply(Action::CloseOverlay);
        assert_eq!(app.mode, Mode::Chat);
        let _ = std::fs::remove_dir_all(&home);
    }

    /// An edit seeds from the setting's current value, commits through
    /// the CLI's own writer, and the pane re-reads so the card shows the
    /// RESOLVED value rather than what was typed — a clamped similarity
    /// or a rejected word shows its real result.
    #[test]
    fn an_edit_seeds_from_the_value_and_commits_through_the_writer() {
        let home = settings_home("edit");
        let mut app = app();
        app.open_settings(home.clone());
        // Select the pin and edit it.
        let pin = app
            .settings
            .iter()
            .position(|state| state.key == "pin")
            .expect("pin is a setting");
        app.selected = pin;
        app.apply(Action::Pick);
        assert_eq!(app.mode, Mode::SettingsEdit);
        assert_eq!(app.editing_key(), Some("pin"));
        assert_eq!(app.editing_buffer(), Some(""), "an unset value seeds empty");
        for character in "stepfun".chars() {
            app.apply(Action::Input(character));
        }
        assert_eq!(app.editing_buffer(), Some("stepfun"));
        app.apply(Action::Submit);
        assert_eq!(app.mode, Mode::Settings, "a commit returns to the list");
        // The write landed, and the pane re-read it.
        let written = app
            .settings
            .iter()
            .find(|state| state.key == "pin")
            .expect("pin");
        assert_eq!(written.value.as_deref(), Some("stepfun"));
        assert_eq!(written.source(), "file");
        // And the file itself carries it.
        let text = std::fs::read_to_string(home.join("config.json")).unwrap();
        assert!(text.contains("stepfun"), "{text}");
        let _ = std::fs::remove_dir_all(&home);
    }

    /// A refused edit lands back in the pane with its reason named and
    /// nothing changed — the GUI is never a way around the loader's own
    /// validation.
    #[test]
    fn a_refused_edit_names_its_reason_and_changes_nothing() {
        let home = settings_home("refused");
        let mut app = app();
        app.open_settings(home.clone());
        let approvals = app
            .settings
            .iter()
            .position(|state| state.key == "approvals")
            .expect("approvals is a setting");
        app.selected = approvals;
        app.apply(Action::Pick);
        for character in "yolo".chars() {
            app.apply(Action::Input(character));
        }
        app.apply(Action::Submit);
        assert_eq!(app.mode, Mode::Settings);
        let note = app.settings_note.clone().expect("the refusal is named");
        assert!(note.contains("approvals"), "{note}");
        assert!(note.contains("yes-guarded"), "{note}");
        // Nothing landed.
        assert!(std::fs::read_to_string(home.join("config.json")).is_err());
        let _ = std::fs::remove_dir_all(&home);
    }

    /// Escape abandons an edit and leaves the file untouched.
    #[test]
    fn escape_abandons_an_edit() {
        let home = settings_home("abandon");
        let mut app = app();
        app.open_settings(home.clone());
        let pin = app
            .settings
            .iter()
            .position(|state| state.key == "pin")
            .expect("pin");
        app.selected = pin;
        app.apply(Action::Pick);
        for character in "nonsense".chars() {
            app.apply(Action::Input(character));
        }
        app.apply(Action::CloseOverlay);
        assert_eq!(app.mode, Mode::Settings);
        assert!(app.editing.is_none());
        assert!(std::fs::read_to_string(home.join("config.json")).is_err());
        let _ = std::fs::remove_dir_all(&home);
    }

    /// A value the environment owns seeds the editor EMPTY: typing over
    /// it would write a file the env var shadows, which is a change that
    /// looks like it landed and does not.
    #[test]
    fn an_env_owned_value_seeds_the_editor_empty() {
        let home = settings_home("env-owned");
        let mut app = app();
        app.open_settings(home.clone());
        let endpoint = app
            .settings
            .iter()
            .position(|state| state.key == "FLEET_EMBEDDING_ENDPOINT")
            .expect("the endpoint is a setting");
        app.selected = endpoint;
        app.apply(Action::Pick);
        assert_eq!(app.editing_buffer(), Some(""));
        let _ = std::fs::remove_dir_all(&home);
    }

    /// A scratch home carrying one compiled-in provider override, one
    /// provider the file itself added (fully specified, so the roster
    /// still loads it), and both classes of workload.
    fn list_home(tag: &str) -> std::path::PathBuf {
        let home = settings_home(tag);
        crate::settings::set_provider(&home, r#"{"id":"stepfun","enabled":false}"#).unwrap();
        crate::settings::set_provider(
            &home,
            r#"{"id":"my-own-gateway","base_url":"http://127.0.0.1:9/v1","key_env":"MY_OWN_GATEWAY_KEY","models":["m-1"]}"#,
        )
        .unwrap();
        crate::settings::set_workload(&home, "quick", &["stepfun/step-3.7-flash".to_string()])
            .unwrap();
        crate::settings::set_workload(
            &home,
            "my-own-shape",
            &["stepfun/step-3.7-flash".to_string()],
        )
        .unwrap();
        home
    }

    /// The row indices of one kind of list card, in pane order.
    fn rows_of(app: &App, keep: impl Fn(SettingsRow) -> bool) -> Vec<usize> {
        app.settings_rows()
            .into_iter()
            .enumerate()
            .filter(|(_, row)| keep(*row))
            .map(|(index, _)| index)
            .collect()
    }

    /// The pane's rows are the scalars beside the two lists, and picking
    /// a provider card opens the editor seeded with that entry's own
    /// JSON — the buffer the writer parses, so committing routes through
    /// the writer's own `set_provider`.
    #[test]
    fn the_pane_carries_the_two_lists_and_seeds_the_editor_from_the_entry() {
        let home = list_home("lists");
        let mut app = app();
        app.open_settings(home.clone());
        let providers = rows_of(&app, |row| matches!(row, SettingsRow::Provider(_)));
        let workloads = rows_of(&app, |row| matches!(row, SettingsRow::Workload(_)));
        assert_eq!(providers.len(), 2, "two provider cards");
        assert_eq!(workloads.len(), 2, "two workload cards");
        // The scalars come first, then the providers, then the workloads.
        assert!(
            providers[0] >= app.settings.len(),
            "the scalars render first"
        );
        assert!(workloads[0] > providers[1], "the workloads render last");

        // Picking the compiled-in override seeds its own JSON.
        app.selected = providers[0];
        app.apply(Action::Pick);
        assert_eq!(app.mode, Mode::SettingsEdit);
        let key = app.editing_key().expect("editing").to_string();
        assert!(key.starts_with("provider.add:"), "{key}");
        let seeded: serde_json::Value =
            serde_json::from_str(app.editing_buffer().expect("a buffer"))
                .expect("the entry's own JSON");
        assert_eq!(seeded["id"], "stepfun");
        assert_eq!(seeded["enabled"], false);
        // Committing it lands through the writer's own set_provider.
        app.apply(Action::Submit);
        assert_eq!(app.mode, Mode::Settings);
        assert_eq!(
            app.settings_note.as_deref(),
            Some("provider `stepfun` written")
        );

        // Picking a workload card seeds the comma-joined candidates under
        // the writer's own key.
        app.selected = workloads[0];
        app.apply(Action::Pick);
        assert_eq!(app.editing_key(), Some("workload.set:quick"));
        assert_eq!(app.editing_buffer(), Some("stepfun/step-3.7-flash"));
        let _ = std::fs::remove_dir_all(&home);
    }

    /// Removing an override of a compiled-in provider reverts it from
    /// the pane: the file's entry goes, and the compiled-in roster still
    /// walks the provider. The question is asked first, and `y` is what
    /// lands it.
    #[test]
    fn removing_a_compiled_in_override_reverts_it_from_the_pane() {
        let home = list_home("revert");
        let mut app = app();
        app.open_settings(home.clone());
        app.selected = rows_of(&app, |row| matches!(row, SettingsRow::Provider(_)))[0];
        app.apply(Action::SettingsRemove);
        assert_eq!(app.mode, Mode::SettingsConfirm);
        let detail = app
            .settings_removal
            .as_ref()
            .map(|removal| removal.detail.clone())
            .expect("the removal is armed");
        assert!(detail.contains("stepfun"), "{detail}");
        assert!(detail.contains("comes back"), "{detail}");
        // `y` lands it.
        app.apply(Action::SettingsConfirm);
        assert_eq!(app.mode, Mode::Settings);
        assert!(app.settings_removal.is_none());
        assert!(
            app.settings_note
                .as_deref()
                .is_some_and(|note| note.contains("stepfun")),
            "{:?}",
            app.settings_note
        );
        // The file's entry is gone, and the compiled-in roster still
        // walks the provider — a revert, not a removal.
        let text = std::fs::read_to_string(home.join("config.json")).unwrap();
        assert!(!text.contains("stepfun"), "{text}");
        let roster = crate::route::load_roster(&home).expect("the roster still loads");
        assert!(
            roster
                .candidates
                .iter()
                .any(|candidate| candidate.id == "stepfun"),
            "the compiled-in roster still walks it"
        );
        let _ = std::fs::remove_dir_all(&home);
    }

    /// Removing one the file itself added refuses, and the refusal names
    /// the rule — the writer's own sentence, read from the crate's
    /// compiled-in roster, exactly as the CLI enforces it.
    #[test]
    fn removing_a_file_added_provider_refuses_and_names_the_rule() {
        let home = list_home("refuse");
        let mut app = app();
        app.open_settings(home.clone());
        app.selected = rows_of(&app, |row| matches!(row, SettingsRow::Provider(_)))[1];
        app.apply(Action::SettingsRemove);
        assert_eq!(app.mode, Mode::SettingsConfirm);
        let detail = app
            .settings_removal
            .as_ref()
            .map(|removal| removal.detail.clone())
            .expect("the removal is armed");
        assert!(detail.contains("every walk"), "{detail}");
        assert!(detail.contains("does not come back"), "{detail}");
        app.apply(Action::SettingsConfirm);
        // The writer refused, by name, and nothing was written.
        let note = app.settings_note.clone().expect("the refusal is named");
        assert!(note.contains("my-own-gateway"), "{note}");
        assert!(note.contains("compiled-in roster"), "{note}");
        assert!(note.contains("every walk"), "{note}");
        let text = std::fs::read_to_string(home.join("config.json")).unwrap();
        assert!(text.contains("my-own-gateway"), "{text}");
        assert_eq!(app.mode, Mode::Settings);
        let _ = std::fs::remove_dir_all(&home);
    }

    /// The question's two outcomes: `y` lands the removal, and every
    /// other key abandons it with nothing written and a note that says
    /// so.
    #[test]
    fn the_confirmation_asks_and_every_other_key_abandons_it() {
        let home = list_home("confirm");
        let mut app = app();
        app.open_settings(home.clone());
        app.selected = rows_of(&app, |row| matches!(row, SettingsRow::Provider(_)))[0];
        let carries = || std::fs::read_to_string(home.join("config.json")).unwrap();

        // Escape abandons: nothing written, the note says so.
        app.apply(Action::SettingsRemove);
        app.apply(Action::CloseOverlay);
        assert_eq!(app.mode, Mode::Settings);
        assert!(app.settings_removal.is_none());
        assert_eq!(app.settings_note.as_deref(), Some("nothing removed"));
        assert!(carries().contains("stepfun"), "nothing was written");

        // A stray key abandons too — the question is total.
        app.apply(Action::SettingsRemove);
        app.apply(Action::Move(1));
        assert_eq!(app.mode, Mode::Settings);
        assert!(app.settings_removal.is_none());
        assert_eq!(app.settings_note.as_deref(), Some("nothing removed"));
        assert!(carries().contains("stepfun"), "nothing was written");

        // `y` lands it.
        app.apply(Action::SettingsRemove);
        app.apply(Action::SettingsConfirm);
        assert!(!carries().contains("stepfun"), "the removal landed");
        let _ = std::fs::remove_dir_all(&home);
    }

    /// A knob has no removal: `r` on a scalar says how one is cleared
    /// rather than arming a question about nothing.
    #[test]
    fn arming_a_removal_on_a_knob_names_how_one_is_cleared() {
        let home = list_home("knob-remove");
        let mut app = app();
        app.open_settings(home.clone());
        app.selected = 0;
        app.apply(Action::SettingsRemove);
        assert_eq!(app.mode, Mode::Settings, "no question about a knob");
        assert!(app.settings_removal.is_none());
        assert!(
            app.settings_note
                .as_deref()
                .is_some_and(|note| note.contains("emptying its value")),
            "{:?}",
            app.settings_note
        );
        let _ = std::fs::remove_dir_all(&home);
    }

    /// The workload surfaces are reachable the same way, with the
    /// writer's own validation naming a bad candidate and the same
    /// destructive rule answering a removal.
    #[test]
    fn a_workload_round_trips_through_the_pane_with_a_bad_candidate_refused() {
        let home = list_home("workload");
        let mut app = app();
        app.open_settings(home.clone());
        let workloads = rows_of(&app, |row| matches!(row, SettingsRow::Workload(_)));
        app.selected = workloads[0];

        // A bad candidate is refused by the writer's own name for it.
        app.apply(Action::Pick);
        assert_eq!(app.editing_key(), Some("workload.set:quick"));
        let seeded = app.editing_buffer().map(str::len).unwrap_or(0);
        for _ in 0..seeded {
            app.apply(Action::Backspace);
        }
        for character in "stepfun".chars() {
            app.apply(Action::Input(character));
        }
        app.apply(Action::Submit);
        let note = app.settings_note.clone().expect("the refusal is named");
        assert!(note.contains("provider/model"), "{note}");
        assert!(note.contains("stepfun"), "{note}");
        // Nothing landed, and the card still reads what it read.
        let text = std::fs::read_to_string(home.join("routing.json")).unwrap();
        assert!(!text.contains("\"stepfun\""), "{text}");

        // A well-formed list lands, and the pane re-reads the card.
        app.selected = workloads[0];
        app.apply(Action::Pick);
        let seeded = app.editing_buffer().map(str::len).unwrap_or(0);
        for _ in 0..seeded {
            app.apply(Action::Backspace);
        }
        for character in "zai-coding-plan/GLM-5.3-Flash,stepfun/step-3.7-flash".chars() {
            app.apply(Action::Input(character));
        }
        app.apply(Action::Submit);
        assert_eq!(
            app.settings_note.as_deref(),
            Some("workload `quick` written")
        );
        let card = app
            .settings_lists
            .iter()
            .find(|entry| entry.key == "quick")
            .expect("the card");
        assert!(card.detail.contains("GLM-5.3-Flash"), "{}", card.detail);
        let text = std::fs::read_to_string(home.join("routing.json")).unwrap();
        assert!(text.contains("GLM-5.3-Flash"), "{text}");

        // Removing the compiled-in workload's preference reverts it, and
        // the compiled-in table still serves the workload.
        app.selected = rows_of(&app, |row| matches!(row, SettingsRow::Workload(_)))[0];
        app.apply(Action::SettingsRemove);
        assert_eq!(app.mode, Mode::SettingsConfirm);
        let detail = app
            .settings_removal
            .as_ref()
            .map(|removal| removal.detail.clone())
            .expect("the removal is armed");
        assert!(detail.contains("quick"), "{detail}");
        assert!(detail.contains("come back"), "{detail}");
        app.apply(Action::SettingsConfirm);
        assert!(
            app.settings_note
                .as_deref()
                .is_some_and(|note| note.contains("quick")),
            "{:?}",
            app.settings_note
        );
        assert!(
            !app.settings_lists.iter().any(|entry| entry.key == "quick"),
            "the card is gone"
        );
        let roster = crate::route::load_roster(&home).expect("the roster still loads");
        assert!(
            crate::route::preference_for(&roster.preferences, "quick").is_some(),
            "the compiled-in table still carries it"
        );

        // The file-added workload refuses, naming the rule.
        app.selected = rows_of(&app, |row| matches!(row, SettingsRow::Workload(_)))[0];
        app.apply(Action::SettingsRemove);
        app.apply(Action::SettingsConfirm);
        let note = app.settings_note.clone().expect("the refusal is named");
        assert!(note.contains("my-own-shape"), "{note}");
        assert!(note.contains("compiled-in table"), "{note}");
        assert!(note.contains("no candidates"), "{note}");
        let text = std::fs::read_to_string(home.join("routing.json")).unwrap();
        assert!(text.contains("my-own-shape"), "{text}");
        let _ = std::fs::remove_dir_all(&home);
    }

    /// The card a seeded `bash` log record becomes.
    fn seeded_card() -> Entry {
        Entry::Tool(ToolCard {
            name: "bash".to_string(),
            summary: String::new(),
            status: ToolStatus::Done,
            preview: "ok".to_string(),
        })
    }

    #[test]
    fn submit_pushes_a_user_entry_and_asks_for_the_run() {
        let mut app = app();
        app.push_input('h');
        app.push_input('i');
        let effect = app.apply(Action::Submit).unwrap();
        assert_eq!(
            effect,
            Effect::Run {
                task: "hi".to_string(),
                plan: false
            }
        );
        assert_eq!(app.entries, vec![Entry::User("hi".to_string())]);
        assert!(app.input.is_empty());
        assert!(app.following);
    }

    #[test]
    fn empty_submit_and_exit_slash_commands() {
        let mut app = app();
        assert_eq!(app.apply(Action::Submit), None);
        for character in "/exit".chars() {
            app.apply(Action::Input(character));
        }
        let effect = app.apply(Action::Submit).unwrap();
        assert_eq!(effect, Effect::Quit);
        assert!(app.should_quit);
    }

    #[test]
    fn input_caps_and_backspaces() {
        let mut app = app();
        for _ in 0..(MAX_INPUT + 10) {
            app.push_input('x');
        }
        assert_eq!(app.input.chars().count(), MAX_INPUT);
        assert_eq!(app.cursor, MAX_INPUT);
        app.apply(Action::Backspace);
        assert_eq!(app.input.chars().count(), MAX_INPUT - 1);
        assert_eq!(app.cursor, MAX_INPUT - 1);
    }

    #[test]
    fn typing_edits_at_the_cursor_not_the_end() {
        let mut app = app();
        for character in "ac".chars() {
            app.apply(Action::Input(character));
        }
        // Park the cursor between the two characters.
        app.apply(Action::CursorHome);
        app.apply(Action::CursorRight);
        assert_eq!(app.cursor, 1);
        app.apply(Action::Input('b'));
        assert_eq!(app.input, "abc");
        assert_eq!(app.cursor, 2);
        // Backspace eats the character before the cursor.
        app.apply(Action::Backspace);
        assert_eq!(app.input, "ac");
        assert_eq!(app.cursor, 1);
        // Delete eats the character at the cursor.
        app.apply(Action::Delete);
        assert_eq!(app.input, "a");
        assert_eq!(app.cursor, 1);
        // Delete at the end is a no-op, never a panic.
        app.apply(Action::Delete);
        assert_eq!(app.input, "a");
    }

    #[test]
    fn the_cursor_clamps_at_both_ends() {
        let mut app = app();
        for character in "hi".chars() {
            app.apply(Action::Input(character));
        }
        app.apply(Action::CursorHome);
        assert_eq!(app.cursor, 0);
        // Left at the start and right past the end both saturate.
        app.apply(Action::CursorLeft);
        assert_eq!(app.cursor, 0);
        app.apply(Action::CursorEnd);
        assert_eq!(app.cursor, 2);
        app.apply(Action::CursorRight);
        assert_eq!(app.cursor, 2);
        // Editing keeps the cursor inside the input it just changed.
        app.apply(Action::CursorHome);
        app.apply(Action::Backspace);
        assert_eq!(app.cursor, 0);
        assert_eq!(app.input, "hi");
    }

    #[test]
    fn word_delete_eats_the_word_before_the_cursor() {
        let mut app = app();
        app.paste("one two three");
        app.apply(Action::DeleteWord);
        assert_eq!(app.input, "one two ");
        assert_eq!(app.cursor, 8);
        app.apply(Action::DeleteWord);
        assert_eq!(app.input, "one ");
        app.apply(Action::DeleteWord);
        assert_eq!(app.input, "");
        // Nothing left to eat, and still no panic.
        app.apply(Action::DeleteWord);
        assert_eq!(app.input, "");
        assert_eq!(app.cursor, 0);
    }

    #[test]
    fn paste_types_without_submitting() {
        let mut typed = app();
        typed.paste("line one\nline two");
        // No Enter, no submit: the newlines are input newlines.
        assert_eq!(typed.input, "line one\nline two");
        assert_eq!(typed.cursor, 17);
        assert_eq!(typed.entries.len(), 0);
        // A single trailing newline is the terminal's, not the user's.
        typed.paste(" more\n");
        assert_eq!(typed.input, "line one\nline two more");
        // Terminals deliver paste line endings three ways; all of them
        // are newlines here (tmux sends a bare \r).
        let mut carriage = app();
        carriage.paste("one\rtwo\r");
        assert_eq!(carriage.input, "one\ntwo");
        let mut crlf = app();
        crlf.paste("one\r\ntwo\r\n");
        assert_eq!(crlf.input, "one\ntwo");
        // The cap still applies to a runaway paste.
        let mut greedy = app();
        greedy.paste(&"x".repeat(MAX_INPUT + 50));
        assert_eq!(greedy.input.chars().count(), MAX_INPUT);
        assert_eq!(greedy.cursor, MAX_INPUT);
    }

    #[test]
    fn resume_is_refused_while_a_run_is_in_flight() {
        let mut app = app();
        app.open_sessions(vec![summary("other")]);
        app.apply_ui_event(UiEvent::Turn(1));
        // The picker stays open, the error explains why, nothing resumes.
        let effect = app.apply(Action::Pick);
        assert_eq!(effect, None);
        assert_eq!(app.mode, Mode::Sessions);
        assert_eq!(
            app.error.as_deref(),
            Some("wait for the run to finish before resuming a session")
        );
        // Once the run ends, the same pick resumes.
        app.apply_ui_event(UiEvent::Finished {
            stop_reason: "completed".to_string(),
        });
        app.error = None;
        let effect = app.apply(Action::Pick).unwrap();
        assert_eq!(effect, Effect::Resume("other".to_string()));
        assert_eq!(app.mode, Mode::Chat);
    }

    #[test]
    fn seeding_resets_the_token_counters() {
        let mut app = app();
        app.apply_status(StatusUpdate::Usage(Usage {
            prompt_tokens: 40,
            completion_tokens: 12,
            total_tokens: 52,
            finish_reason: None,
        }));
        app.seed(&[Record::User {
            content: "old task".to_string(),
        }]);
        assert_eq!(app.status.prompt_tokens, 0);
        assert_eq!(app.status.completion_tokens, 0);
        // The count then runs for the resumed conversation only.
        app.apply_status(StatusUpdate::Usage(Usage {
            prompt_tokens: 7,
            completion_tokens: 3,
            total_tokens: 10,
            finish_reason: None,
        }));
        assert_eq!(app.status.prompt_tokens, 7);
        assert_eq!(app.status.completion_tokens, 3);
    }

    #[test]
    fn token_events_stream_into_one_assistant_block() {
        let mut app = app();
        app.apply_ui_event(UiEvent::Turn(1));
        app.apply_ui_event(UiEvent::Token("he".to_string()));
        app.apply_ui_event(UiEvent::Token("llo".to_string()));
        assert_eq!(
            app.conversation_lines(),
            vec![Entry::Assistant("hello".to_string())]
        );
        // The next turn's first event flushes the stream into a block.
        app.apply_ui_event(UiEvent::Turn(2));
        assert_eq!(app.entries, vec![Entry::Assistant("hello".to_string())]);
        assert_eq!(app.status.turn, 2);
        assert!(app.status.working);
    }

    #[test]
    fn tool_call_and_result_fill_one_card_in_order() {
        let mut app = app();
        app.apply_ui_event(UiEvent::ToolCall {
            name: "bash".to_string(),
            summary: "ls -la".to_string(),
            arguments: None,
        });
        app.apply_ui_event(UiEvent::ToolResult {
            name: "bash".to_string(),
            preview: "total 0".to_string(),
            approved: true,
            resolution: None,
        });
        let Entry::Tool(card) = &app.entries[0] else {
            panic!("expected a tool card")
        };
        assert_eq!(card.status, ToolStatus::Done);
        assert_eq!(card.preview, "total 0");
        assert_eq!(card.summary, "ls -la");
    }

    /// A delegated child run lands on the `subagent` call's own card: the
    /// ordinary ToolCall event opens it, the child's progress fills it,
    /// and the ordinary ToolResult event closes it — one card, in the
    /// house style, with the goal on it and the outcome at the end.
    #[test]
    fn a_subagent_progress_lands_on_the_tool_card() {
        let mut app = app();
        app.apply_ui_event(UiEvent::Turn(1));
        app.apply_ui_event(UiEvent::ToolCall {
            name: "subagent".to_string(),
            summary: "count the files".to_string(),
            arguments: None,
        });
        // Spawned with no open card still belongs on screen.
        let mut fresh = crate::tui::app::App::new("deepseek-flash", "session-1");
        fresh.apply_ui_event(UiEvent::Subagent {
            goal: "count the files".to_string(),
            session: "child-1".to_string(),
            stage: SubagentStage::Spawned,
        });
        let Entry::Tool(card) = &fresh.entries[0] else {
            panic!("expected a tool card")
        };
        assert_eq!(card.status, ToolStatus::Pending);
        assert_eq!(card.summary, "count the files");

        // With the card open, the child's progress updates it in place and
        // never adds a second one.
        app.apply_ui_event(UiEvent::Subagent {
            goal: "count the files".to_string(),
            session: "child-1".to_string(),
            stage: SubagentStage::Working {
                calls: 2,
                tool: "bash".to_string(),
            },
        });
        assert_eq!(app.entries.len(), 1, "{:?}", app.entries);
        let Entry::Tool(card) = &app.entries[0] else {
            panic!("expected a tool card")
        };
        assert_eq!(
            card.preview,
            "child child-1 \u{b7} 2 call(s) \u{b7} last bash"
        );
        // The status dot is the ordinary pair's, not the child's: a working
        // child leaves its card pending until the call's own result lands.
        assert_eq!(card.status, ToolStatus::Pending);

        app.apply_ui_event(UiEvent::Subagent {
            goal: "count the files".to_string(),
            session: "child-1".to_string(),
            stage: SubagentStage::Done {
                outcome: "completed".to_string(),
                turns: 3,
            },
        });
        app.apply_ui_event(UiEvent::ToolResult {
            name: "subagent".to_string(),
            preview: "subagent completed \u{b7} 3 turns".to_string(),
            approved: true,
            resolution: None,
        });
        assert_eq!(app.entries.len(), 1, "{:?}", app.entries);
        let Entry::Tool(card) = &app.entries[0] else {
            panic!("expected a tool card")
        };
        assert_eq!(card.status, ToolStatus::Done);
        assert_eq!(card.summary, "count the files");
        // The result's own preview is what stays: the card reads like every
        // other tool card's.
        assert_eq!(card.preview, "subagent completed \u{b7} 3 turns");
        // The parent's turn counter and working state are untouched by the
        // child's own events.
        assert_eq!(app.status.turn, 1);
        assert!(app.status.working);
    }

    /// A lane joining the run lands as its own block, between the call
    /// that triggered it and the result that follows.
    #[test]
    fn a_lane_joining_lands_as_its_own_block() {
        let mut app = app();
        app.apply_ui_event(UiEvent::Turn(1));
        app.apply_ui_event(UiEvent::Token("half a thought".to_string()));
        app.apply_ui_event(UiEvent::ToolCall {
            name: "mcp__sem1__probe".to_string(),
            summary: "embeddings \u{b7} probe {}".to_string(),
            arguments: None,
        });
        app.apply_ui_event(UiEvent::LaneJoined {
            lane: "sem1".to_string(),
            tool_count: 4,
        });
        app.apply_ui_event(UiEvent::ToolResult {
            name: "mcp__sem1__probe".to_string(),
            preview: "reachable".to_string(),
            approved: true,
            resolution: None,
        });
        // The streaming tail flushed when the lane landed, and the lane's
        // block sits between the call and its result.
        assert_eq!(
            app.entries,
            vec![
                Entry::Assistant("half a thought".to_string()),
                Entry::Tool(ToolCard {
                    name: "mcp__sem1__probe".to_string(),
                    summary: "embeddings \u{b7} probe {}".to_string(),
                    status: ToolStatus::Done,
                    preview: "reachable".to_string(),
                }),
                Entry::Lane(LaneCard {
                    lane: "sem1".to_string(),
                    tool_count: 4
                }),
            ]
        );
    }

    /// The library's session-start report lands as its own card, never
    /// as stderr over the pane (KI-216) — and it flushes the streaming
    /// tail the way every fact-about-the-session card does.
    #[test]
    fn the_skills_library_report_lands_as_its_own_card() {
        let mut app = app();
        app.apply_ui_event(UiEvent::Token("half a thought".to_string()));
        app.apply_ui_event(UiEvent::SkillsLoaded {
            loaded: 441,
            skipped: vec![
                "creative-ideation".to_string(),
                "deli-autoresearch".to_string(),
                "dev-decisions.bak-20260926-124047".to_string(),
                "tools".to_string(),
                ".git".to_string(),
            ],
        });
        assert_eq!(
            app.entries,
            vec![
                Entry::Assistant("half a thought".to_string()),
                Entry::Skills(SkillsCard {
                    loaded: 441,
                    skipped: vec![
                        "creative-ideation".to_string(),
                        "deli-autoresearch".to_string(),
                        "dev-decisions.bak-20260926-124047".to_string(),
                        "tools".to_string(),
                        ".git".to_string(),
                    ],
                }),
            ]
        );
    }

    /// A resumed conversation shows the lanes its earlier runs joined, so
    /// the lane's tool cards that follow read as the lane's own.
    #[test]
    fn seeding_a_log_shows_the_lanes_that_joined_it() {
        let mut app = app();
        app.seed(&[
            Record::User {
                content: "probe it".to_string(),
            },
            Record::LaneTools {
                lane: "sem1".to_string(),
                tools: vec![
                    crate::tools::LaneTool {
                        name: "mcp__sem1__probe".to_string(),
                        lane_tool: "sem1_probe".to_string(),
                        description: "probe".to_string(),
                        input_schema: serde_json::json!({ "type": "object" }),
                    },
                    crate::tools::LaneTool {
                        name: "mcp__sem1__embed".to_string(),
                        lane_tool: "sem1_embed".to_string(),
                        description: "embed".to_string(),
                        input_schema: serde_json::json!({ "type": "object" }),
                    },
                ],
            },
            Record::ToolResult {
                call_id: "call-1".to_string(),
                name: "mcp__sem1__probe".to_string(),
                output: "reachable".to_string(),
                truncated: false,
                approved: true,
            },
        ]);
        assert_eq!(
            app.entries,
            vec![
                Entry::User("probe it".to_string()),
                Entry::Lane(LaneCard {
                    lane: "sem1".to_string(),
                    tool_count: 2
                }),
                // The lane's own tool result, under its namespaced name.
                Entry::Tool(ToolCard {
                    name: "mcp__sem1__probe".to_string(),
                    summary: String::new(),
                    status: ToolStatus::Done,
                    preview: "reachable".to_string(),
                }),
            ]
        );
    }

    /// A `skills` record is bookkeeping about the system prompt, like
    /// `start` and `end`: seeding a log that carries one adds no
    /// conversation block, and a skill's use shows as the ordinary tool
    /// card its `tool_result` seeded — no separate skills surface.
    #[test]
    fn seeding_a_log_skips_the_skills_record() {
        let mut app = app();
        app.seed(&[
            Record::Start {
                v: 1,
                model: "m".to_string(),
                created: "c".to_string(),
                kind: None,
                parent: None,
                workspace: None,
            },
            Record::Skills {
                skills: vec![crate::skills::SkillRef {
                    name: "bash-recovery".to_string(),
                    description: "Recover a failed bash call".to_string(),
                }],
            },
            Record::User {
                content: "run it".to_string(),
            },
            Record::Assistant {
                content: None,
                tool_calls: vec![crate::api::ToolCall {
                    id: "call-1".to_string(),
                    kind: "function".to_string(),
                    function: crate::api::FunctionCall {
                        name: "use_skill".to_string(),
                        arguments: r#"{"name":"bash-recovery"}"#.to_string(),
                    },
                }],
            },
            Record::ToolResult {
                call_id: "call-1".to_string(),
                name: "use_skill".to_string(),
                output: "# bash-recovery".to_string(),
                truncated: false,
                approved: true,
            },
        ]);
        assert_eq!(
            app.entries,
            vec![
                Entry::User("run it".to_string()),
                // The assistant turn that requested the call renders as
                // its own (empty) block, exactly as it always has.
                Entry::Assistant(String::new()),
                // The skill's use is an ordinary tool card.
                Entry::Tool(ToolCard {
                    name: "use_skill".to_string(),
                    summary: String::new(),
                    status: ToolStatus::Done,
                    preview: "# bash-recovery".to_string(),
                }),
            ]
        );
    }

    #[test]
    fn a_denied_result_marks_the_card_denied() {
        let mut app = app();
        app.apply_ui_event(UiEvent::ToolCall {
            name: "bash".to_string(),
            summary: "rm -rf /".to_string(),
            arguments: None,
        });
        app.apply_ui_event(UiEvent::ToolResult {
            name: "bash".to_string(),
            preview: "denied: the user did not approve this call.".to_string(),
            approved: false,
            resolution: None,
        });
        let Entry::Tool(card) = &app.entries[0] else {
            panic!("expected a tool card")
        };
        assert_eq!(card.status, ToolStatus::Denied);
    }

    #[test]
    fn finished_and_failed_end_the_working_state() {
        let mut app = app();
        app.apply_ui_event(UiEvent::Turn(3));
        app.apply_ui_event(UiEvent::Finished {
            stop_reason: "completed".to_string(),
        });
        assert!(!app.status.working);
        assert_eq!(app.status.turn, 0);

        app.apply_ui_event(UiEvent::Turn(4));
        app.apply_ui_event(UiEvent::Failed("402 Insufficient Balance".to_string()));
        assert!(!app.status.working);
        assert_eq!(app.error.as_deref(), Some("402 Insufficient Balance"));
    }

    #[test]
    fn the_status_bar_carries_what_the_last_run_assembled() {
        let mut app = app();
        // Absent until a run assembles something.
        assert_eq!(app.status.memory, None);
        app.apply_ui_event(UiEvent::Memory {
            shape: "deploy".to_string(),
            facts: 3,
        });
        assert_eq!(app.status.memory, Some(("deploy".to_string(), 3)));
        // An unclassified assembly says so with an empty shape.
        app.apply_ui_event(UiEvent::Memory {
            shape: String::new(),
            facts: 1,
        });
        assert_eq!(app.status.memory, Some((String::new(), 1)));
        // Seeding a resumed log clears it: the fact describes the
        // conversation on screen, and a resumed log's earlier runs
        // assembled theirs elsewhere.
        app.seed(&[Record::User {
            content: "old task".to_string(),
        }]);
        assert_eq!(app.status.memory, None);
    }

    #[test]
    fn usage_accumulates_across_runs() {
        let mut app = app();
        app.apply_status(StatusUpdate::Usage(Usage {
            prompt_tokens: 10,
            completion_tokens: 5,
            finish_reason: None,
            total_tokens: 15,
        }));
        app.apply_status(StatusUpdate::Usage(Usage {
            prompt_tokens: 20,
            completion_tokens: 7,
            finish_reason: None,
            total_tokens: 27,
        }));
        assert_eq!(app.status.prompt_tokens, 30);
        assert_eq!(app.status.completion_tokens, 12);
    }

    #[test]
    fn the_picker_moves_clamped_and_picks_a_session() {
        let mut app = app();
        app.open_sessions(vec![summary("first"), summary("second"), summary("third")]);
        assert_eq!(app.mode, Mode::Sessions);
        assert_eq!(app.selected, 0);
        app.apply(Action::Move(1));
        assert_eq!(app.selected, 1);
        app.apply(Action::Move(-1));
        assert_eq!(app.selected, 0);
        // Clamped at both ends, never wrapping.
        app.apply(Action::Move(-5));
        assert_eq!(app.selected, 0);
        app.apply(Action::Move(99));
        assert_eq!(app.selected, 2);
        let effect = app.apply(Action::Pick).unwrap();
        assert_eq!(effect, Effect::Resume("third".to_string()));
        assert_eq!(app.mode, Mode::Chat);
    }

    #[test]
    fn an_empty_picker_picks_nothing_and_escape_closes_it() {
        let mut app = app();
        app.open_sessions(Vec::new());
        app.apply(Action::Move(1));
        assert_eq!(app.apply(Action::Pick), None);
        app.apply(Action::CloseOverlay);
        assert_eq!(app.mode, Mode::Chat);
    }

    #[test]
    fn ctrl_l_opens_the_lanes_view_and_the_rows_fill_it() {
        let mut app = app();
        app.apply(Action::OpenLanes);
        assert_eq!(app.mode, Mode::Lanes);
        // The probe is in flight: the view is open but empty.
        assert!(app.lanes.is_empty());
        assert!(app.lanes_loading);
        // The prober's rows land and the loading state clears.
        app.set_lanes(
            app.lanes_probe,
            vec![lane_health("sem1", crate::lanes::LaneState::Ok, 4)],
        );
        assert!(!app.lanes_loading);
        assert_eq!(app.lanes.len(), 1);
        // The view scrolls its rows and closes on escape.
        app.apply(Action::Move(1));
        assert_eq!(app.selected, 0);
        app.apply(Action::Move(-1));
        assert_eq!(app.selected, 0);
        app.apply(Action::CloseOverlay);
        assert_eq!(app.mode, Mode::Chat);
    }

    #[test]
    fn rows_from_a_previous_probe_never_render_into_a_fresh_modal() {
        let mut app = app();
        app.apply(Action::OpenLanes);
        let stale = app.lanes_probe;
        // The user closes it and opens it again before the first probe's
        // rows land: a fresh generation, a fresh (empty) view.
        app.apply(Action::CloseOverlay);
        app.apply(Action::OpenLanes);
        let fresh = app.lanes_probe;
        assert_ne!(stale, fresh, "each open stamps a new probe");
        // The stale rows arrive while the fresh modal is open — the mode
        // alone would let them through, which is the whole bug.
        app.set_lanes(
            stale,
            vec![lane_health("sys1", crate::lanes::LaneState::Ok, 1)],
        );
        assert!(app.lanes.is_empty(), "stale rows must not render");
        assert!(app.lanes_loading, "the fresh probe is still in flight");
        // The fresh probe's own rows still land.
        app.set_lanes(
            fresh,
            vec![lane_health("sem1", crate::lanes::LaneState::Ok, 4)],
        );
        assert_eq!(app.lanes.len(), 1);
        assert_eq!(app.lanes[0].name, "sem1");
        assert!(!app.lanes_loading);
    }

    #[test]
    fn the_lanes_view_holds_no_menu_semantics() {
        let mut app = app();
        app.apply(Action::OpenLanes);
        app.set_lanes(
            app.lanes_probe,
            vec![lane_health("sem1", crate::lanes::LaneState::Ok, 4)],
        );
        // Enter picks nothing; typing is swallowed; only quit and close
        // answer.
        assert_eq!(app.apply(Action::Pick), None);
        assert_eq!(app.apply(Action::Input('x')), None);
        assert_eq!(app.mode, Mode::Lanes);
        assert_eq!(app.apply(Action::Quit), Some(Effect::Quit));
        assert!(app.should_quit);
    }

    fn lane_health(name: &str, state: crate::lanes::LaneState, tools: usize) -> LaneHealth {
        LaneHealth {
            name: name.to_string(),
            role: "embeddings".to_string(),
            home: Some(std::path::PathBuf::from("/repo/services/sem1")),
            source: Some(crate::lanes::HomeSource::Bundled),
            state,
            server_version: Some("0.1.0".to_string()),
            tools: (0..tools).map(|index| format!("tool_{index}")).collect(),
            doctor: Vec::new(),
            detail: "SEM1_PYTHONPATH=…/src python3 -u …/service/mcp_server.py".to_string(),
            elapsed_ms: 53,
        }
    }

    #[test]
    fn the_approval_modal_owns_every_key() {
        let mut app = app();
        app.apply_ui_event(UiEvent::ToolCall {
            name: "bash".to_string(),
            summary: "ls -la".to_string(),
            arguments: None,
        });
        app.show_approval("bash", "ls -la");
        assert_eq!(app.mode, Mode::Approval);
        // Typing and submitting are swallowed while the modal is up.
        assert_eq!(app.apply(Action::Input('x')), None);
        assert_eq!(app.apply(Action::Submit), None);
        let effect = app.apply(Action::Approve(Approval::Once)).unwrap();
        assert_eq!(effect, Effect::Answer(Approval::Once));
        assert_eq!(app.mode, Mode::Chat);
        assert!(app.approval.is_none());
        // The approved call now shows as running.
        let Entry::Tool(card) = &app.entries[0] else {
            panic!("expected a tool card")
        };
        assert_eq!(card.status, ToolStatus::Approved);
        // Its result lands as done.
        app.apply_ui_event(UiEvent::ToolResult {
            name: "bash".to_string(),
            preview: "exit 0".to_string(),
            approved: true,
            resolution: None,
        });
        let Entry::Tool(card) = &app.entries[0] else {
            panic!("expected a tool card")
        };
        assert_eq!(card.status, ToolStatus::Done);
    }

    #[test]
    fn scrolling_back_stops_following_and_returning_rejoins_the_tail() {
        let mut app = app();
        assert!(app.following);
        app.apply(Action::Scroll(3));
        assert_eq!(app.scrolled_back, 3);
        assert!(!app.following);
        app.apply(Action::Scroll(-1));
        assert_eq!(app.scrolled_back, 2);
        app.apply(Action::Scroll(-2));
        assert_eq!(app.scrolled_back, 0);
        assert!(app.following);
        // Never negative.
        app.apply(Action::Scroll(-9));
        assert_eq!(app.scrolled_back, 0);
    }

    #[test]
    fn seeding_replaces_the_conversation_with_the_log() {
        let mut app = app();
        app.push_input('x');
        app.apply_ui_event(UiEvent::Token("half a thought".to_string()));
        app.apply_ui_event(UiEvent::Failed("boom".to_string()));
        let records = vec![
            Record::Start {
                v: 1,
                model: "m".to_string(),
                created: "c".to_string(),
                kind: None,
                parent: None,
                workspace: None,
            },
            Record::User {
                content: "resumed task".to_string(),
            },
            Record::Assistant {
                content: Some(String::new()),
                tool_calls: vec![],
            },
            Record::Assistant {
                content: Some("done".to_string()),
                tool_calls: vec![],
            },
            Record::ToolResult {
                call_id: "c1".to_string(),
                name: "bash".to_string(),
                output: "ok".to_string(),
                truncated: false,
                approved: true,
            },
            Record::End {
                stop_reason: "completed".to_string(),
                turns: 2,
                served_model: None,
            },
        ];
        app.seed(&records);
        assert_eq!(
            app.entries,
            vec![
                Entry::User("resumed task".to_string()),
                Entry::Assistant("done".to_string()),
                seeded_card(),
            ]
        );
        assert!(app.streaming.is_empty());
        assert!(app.error.is_none());
        assert!(app.following);
    }

    fn summary(id: &str) -> SessionSummary {
        SessionSummary {
            first_text: None,
            id: id.to_string(),
            model: "deepseek-flash".to_string(),
            created: "2026-10-13T00:00:00Z".to_string(),
            ended: false,
            kind: None,
            workspace: None,
            workspace_inferred: false,
            channel: None,
            compacted: false,
            served_model: None,
            stop_reason: None,
            turns: None,
            last_write: None,
            path: std::path::PathBuf::from(format!("/tmp/{id}.jsonl")),
        }
    }

    /// A compaction lands as its own block in the house card family: a
    /// dot, what happened, and what it cost — never the summary's prose,
    /// which belongs to the request it rides in.
    #[test]
    fn a_compaction_lands_as_its_own_card() {
        let mut app = app();
        app.apply_ui_event(UiEvent::Turn(3));
        app.apply_ui_event(UiEvent::Token("half a thought".to_string()));
        app.apply_ui_event(UiEvent::Compacted {
            turns: 6,
            tokens_before: 120_000,
            tokens_after: 900,
        });
        // The streaming tail flushed when the card landed, and the run's
        // own turn counter is untouched: a compaction is a fact about the
        // conversation, not a turn of it.
        assert_eq!(
            app.entries,
            vec![
                Entry::Assistant("half a thought".to_string()),
                Entry::Compaction(CompactionCard {
                    turns: 6,
                    tokens_before: 120_000,
                    tokens_after: 900,
                }),
            ]
        );
        assert_eq!(app.status.turn, 3);
        assert!(app.status.working);
    }

    /// A scheduled fire lands as its own block in the house card
    /// family: the job, the minute, the outcome, and the session it
    /// wrote — the run belongs to a job in a log of its own, never a
    /// turn in this conversation (KI-090).
    #[test]
    fn a_scheduled_fire_lands_as_its_own_card() {
        let mut app = app();
        app.apply_ui_event(UiEvent::Turn(3));
        app.apply_ui_event(UiEvent::Token("half a thought".to_string()));
        app.apply_ui_event(UiEvent::CronFire {
            job: "daily-digest".to_string(),
            minute: "2026-10-14T12:00Z".to_string(),
            catch_up: false,
            outcome: "completed".to_string(),
            detail: "completed in 3 turns".to_string(),
            session: Some("20261014T120000Z-1a2b3c".to_string()),
        });
        // The streaming tail flushed when the card landed, and the run's
        // own turn counter is untouched: a fire is a fact about the
        // machine, not a turn of this conversation.
        assert_eq!(
            app.entries,
            vec![
                Entry::Assistant("half a thought".to_string()),
                Entry::Cron(CronCard {
                    job: "daily-digest".to_string(),
                    minute: "2026-10-14T12:00Z".to_string(),
                    catch_up: false,
                    outcome: "completed".to_string(),
                    detail: "completed in 3 turns".to_string(),
                    session: Some("20261014T120000Z-1a2b3c".to_string()),
                }),
            ]
        );
        assert_eq!(app.status.turn, 3);
        assert!(app.status.working);

        // A fire with no session — a run that failed before its log —
        // still lands, and a catch-up window is carried as one.
        app.apply_ui_event(UiEvent::CronFire {
            job: "nightly".to_string(),
            minute: "2026-10-14T03:00Z".to_string(),
            catch_up: true,
            outcome: "panicked".to_string(),
            detail: "the fire panicked: boom".to_string(),
            session: None,
        });
        let last = app.entries.last().expect("the second card");
        match last {
            Entry::Cron(card) => {
                assert!(card.catch_up, "the catch-up window is carried");
                assert_eq!(card.outcome, "panicked");
                assert_eq!(card.session, None);
            }
            other => panic!("expected a cron card, got {other:?}"),
        }
    }

    /// A resumed conversation shows the compactions its earlier runs
    /// performed, rebuilt from the range each record names.
    #[test]
    fn seeding_a_log_shows_the_compactions_it_performed() {
        let mut app = app();
        app.seed(&[
            Record::User {
                content: "the task".to_string(),
            },
            Record::Assistant {
                content: None,
                tool_calls: Vec::new(),
            },
            Record::Compaction {
                summary: "- did the thing".to_string(),
                first_turn: 1,
                last_turn: 2,
                tokens_before: 120_000,
                tokens_after: 900,
            },
            Record::Assistant {
                content: Some("carried on".to_string()),
                tool_calls: Vec::new(),
            },
        ]);
        assert_eq!(
            app.entries,
            vec![
                Entry::User("the task".to_string()),
                // The summary stands in for turns 1 through 2; the card
                // names the fold rather than repeating the summary.
                Entry::Compaction(CompactionCard {
                    turns: 2,
                    tokens_before: 120_000,
                    tokens_after: 900,
                }),
                Entry::Assistant("carried on".to_string()),
            ]
        );
    }

    fn routed() -> UiEvent {
        UiEvent::Routed {
            provider: "stepfun".to_string(),
            model: "step-5-preview".to_string(),
            how: "judged".to_string(),
            workload: Some("hard".to_string()),
            verdict: Some("pass".to_string()),
            judged_from: "fresh".to_string(),
            reason: "the orchestrator walks hard's chain".to_string(),
            order: vec![("stepfun".to_string(), "step-5-preview".to_string())],
        }
    }

    #[test]
    fn a_run_walks_the_loop_and_reduces_its_branches() {
        let mut app = app();
        app.push_input('g');
        app.push_input('o');
        assert_eq!(
            app.apply(Action::Submit),
            Some(Effect::Run {
                task: "go".to_string(),
                plan: false
            })
        );
        assert!(app.run.active);
        assert_eq!(app.run.stage, Stage::Route);
        assert_eq!(app.pane, Pane::Run);

        app.apply_ui_event(routed());
        assert_eq!(app.run.provider, "stepfun");
        assert_eq!(app.run.model, "step-5-preview");
        assert_eq!(app.run.workload, "hard");

        app.apply_ui_event(UiEvent::Turn(3));
        assert_eq!(app.run.stage, Stage::Think);
        assert_eq!(app.run.turn, 3);

        app.apply_ui_event(UiEvent::ToolCall {
            name: "bash".to_string(),
            summary: "cargo test".to_string(),
            arguments: None,
        });
        assert_eq!(app.run.stage, Stage::Act);
        assert_eq!(app.run.calls, 1);
        assert_eq!(app.run.tool, "bash");

        app.apply_ui_event(UiEvent::Memory {
            shape: "change".to_string(),
            facts: 24,
        });
        assert_eq!(app.run.facts, 24);
        assert_eq!(app.run.shape, "change");

        app.apply_ui_event(UiEvent::Checkpoint {
            turn: 3,
            files: 2,
            bytes: 10,
        });
        assert_eq!(app.run.checkpoint, Some(3));

        app.apply_ui_event(UiEvent::Subagent {
            goal: "read the router".to_string(),
            session: "20261020T000000Z-abcdef".to_string(),
            stage: SubagentStage::Spawned,
        });
        app.apply_ui_event(UiEvent::Subagent {
            goal: "read the router".to_string(),
            session: "20261020T000000Z-abcdef".to_string(),
            stage: SubagentStage::Working {
                calls: 2,
                tool: "bash".to_string(),
            },
        });
        app.apply_ui_event(UiEvent::Subagent {
            goal: "read the router".to_string(),
            session: "20261020T000000Z-abcdef".to_string(),
            stage: SubagentStage::Done {
                outcome: "completed".to_string(),
                turns: 4,
            },
        });
        assert_eq!(app.run.branches.len(), 1);
        assert_eq!(app.run.branches[0].stage, "done");
        assert_eq!(app.run.branches[0].outcome, "completed");
        assert_eq!(app.run.branches[0].turns, 4);

        app.apply_ui_event(UiEvent::Usage {
            prompt_tokens: 33_360,
            completion_tokens: 3,
        });
        assert_eq!(app.run.context, 33_360);
        assert_eq!(app.run.tokens, vec![33_363]);

        app.apply_ui_event(UiEvent::Finished {
            stop_reason: "completed".to_string(),
        });
        assert!(!app.run.active);
        assert_eq!(app.run.stage, Stage::Verify);
        assert_eq!(app.run.stop_reason.as_deref(), Some("completed"));
        assert!(!app.status.working);
    }

    #[test]
    fn plan_mode_keeps_the_plan_and_ctrl_e_loads_it_without_sending() {
        let mut app = app();
        app.apply(Action::TogglePlan);
        assert!(app.run.plan);
        app.apply(Action::Input('p'));
        assert_eq!(
            app.apply(Action::Submit),
            Some(Effect::Run {
                task: "p".to_string(),
                plan: true
            })
        );
        app.apply_ui_event(UiEvent::Token("the plan".to_string()));
        app.apply_ui_event(UiEvent::Finished {
            stop_reason: "completed".to_string(),
        });
        assert_eq!(app.plan_text.as_deref(), Some("the plan"));
        // Ctrl-E copies it into the message box; it never submits.
        assert_eq!(app.apply(Action::LoadPlan), None);
        assert_eq!(app.input, "the plan");
        assert!(app.plan_text.is_none());
        assert!(app
            .entries
            .iter()
            .any(|entry| entry == &Entry::User("p".to_string())));
    }

    #[test]
    fn stop_and_steer_need_a_run_and_a_log_and_record_the_intent() {
        let mut app = app();
        assert_eq!(app.apply(Action::StopRun), None);
        assert!(app.error.is_some());
        // A run is in flight but the agent has not created the log yet:
        // the control file has no beside-the-log home, so it is refused.
        app.apply(Action::Input('x'));
        app.apply(Action::Submit);
        app.status.session_id.clear();
        assert_eq!(app.apply(Action::StopRun), None);
        assert!(app
            .error
            .as_deref()
            .is_some_and(|message| message.contains("no session log")));
        // Once the log exists, the stop is one control write.
        app.status.session_id = "20261020T000000Z-abcdef".to_string();
        assert_eq!(
            app.apply(Action::StopRun),
            Some(Effect::Control { steer: None })
        );
        assert!(app.run.stopping);
        // A steer is its own one-line write.
        app.apply(Action::OpenSteer);
        for character in "read the gate".chars() {
            app.apply(Action::Input(character));
        }
        assert_eq!(
            app.apply(Action::Submit),
            Some(Effect::Control {
                steer: Some("read the gate".to_string())
            })
        );
        assert_eq!(app.mode, Mode::Run);
        assert!(app.run.last.contains("steer sent"));
    }

    fn snapshot(turn: u64) -> crate::checkpoints::Snapshot {
        crate::checkpoints::Snapshot {
            turn,
            at: "2026-10-20T00:00:00Z".to_string(),
            files: 4,
            bytes: 1024,
            unread: 0,
        }
    }

    fn diff_file() -> crate::checkpoints::FileDiff {
        crate::checkpoints::FileDiff {
            path: "src/lib.rs".to_string(),
            kind: crate::checkpoints::DiffKind::Modified,
            hunks: vec![crate::checkpoints::Hunk {
                old: vec!["old line".to_string()],
                new: vec!["new line".to_string()],
                new_at: 2,
            }],
            too_large: false,
        }
    }

    #[test]
    fn the_checkpoint_cursor_arms_a_rewind_and_the_confirm_lands_it() {
        let mut app = app();
        app.open_checkpoints(vec![snapshot(4), snapshot(2)]);
        assert_eq!(app.mode, Mode::Checkpoints);
        assert_eq!(app.checkpoint_turn(), Some(4));
        app.arm_rewind();
        assert_eq!(app.mode, Mode::Confirm);
        assert_eq!(
            app.land_trust(),
            Some(Effect::Rewind(4)),
            "the armed turn is the one the effect names"
        );
        assert_eq!(app.mode, Mode::Chat);
        // The second row is the older checkpoint.
        app.open_checkpoints(vec![snapshot(4), snapshot(2)]);
        app.apply(Action::Move(1));
        assert_eq!(app.checkpoint_turn(), Some(2));
    }

    #[test]
    fn the_diff_arms_a_hunk_or_a_file_and_a_cancel_touches_nothing() {
        let mut app = app();
        app.open_diff(4, vec![diff_file()]);
        assert_eq!(app.mode, Mode::Diff);
        app.arm_reject_hunk();
        assert_eq!(app.mode, Mode::Confirm);
        assert_eq!(
            app.land_trust(),
            Some(Effect::RejectHunk {
                turn: 4,
                file: "src/lib.rs".to_string(),
                hunk: 0,
            })
        );
        app.arm_reject_file();
        assert_eq!(
            app.land_trust(),
            Some(Effect::RejectFile {
                turn: 4,
                file: "src/lib.rs".to_string(),
            })
        );
        // Esc in the confirm view returns to the diff with nothing armed.
        app.arm_reject_file();
        app.apply(Action::TrustCancel);
        assert_eq!(app.mode, Mode::Diff);
        assert!(app.confirm.is_none());
    }

    #[test]
    fn usage_events_feed_the_bar_and_the_run_card() {
        let mut app = app();
        app.apply_ui_event(UiEvent::Usage {
            prompt_tokens: 100,
            completion_tokens: 10,
        });
        assert_eq!(app.status.prompt_tokens, 100);
        assert_eq!(app.status.completion_tokens, 10);
        assert_eq!(app.run.context, 100);
        assert_eq!(app.run.tokens, vec![110]);
        app.apply_ui_event(UiEvent::Usage {
            prompt_tokens: 20,
            completion_tokens: 2,
        });
        assert_eq!(app.status.prompt_tokens, 120);
        assert_eq!(app.status.completion_tokens, 12);
        // The context is the last request, not the sum.
        assert_eq!(app.run.context, 20);
        assert_eq!(app.run.tokens, vec![110, 22]);
    }

    fn memory_snapshot() -> crate::boards::MemorySnapshot {
        let fact = crate::boards::MemoryFact {
            name: "a-fact".to_string(),
            description: "the sentence".to_string(),
            fact_type: "project".to_string(),
            basis: "inferred".to_string(),
            tier: "durable".to_string(),
            date: "2026-10-20".to_string(),
            recall_count: 3,
            protected: true,
            retired: false,
            superseded_by: String::new(),
            valid_until: String::new(),
            shape: Vec::new(),
            phrases: Vec::new(),
            missing: Vec::new(),
            body: "the whole body".to_string(),
            path: "memories/a-fact.md".to_string(),
        };
        crate::boards::MemorySnapshot {
            read_at: "2026-10-20T00:00:00Z".to_string(),
            day: "2026-10-20".to_string(),
            root: "memories".to_string(),
            facts: vec![fact],
            hits: Vec::new(),
            refused: Vec::new(),
        }
    }

    #[test]
    fn a_panel_read_lands_only_for_its_generation_and_filters() {
        let mut app = app();
        let generation = app.open_panel(PanelKind::Memory);
        assert!(app.panel_loading);
        assert_eq!(app.mode, Mode::Panel);
        // A late read from a previous open is dropped.
        app.apply_panel(
            generation.wrapping_sub(1),
            PanelKind::Memory,
            Ok(PanelData::Memory(memory_snapshot())),
        );
        assert!(app.panel.is_none());
        app.apply_panel(
            generation,
            PanelKind::Memory,
            Ok(PanelData::Memory(memory_snapshot())),
        );
        assert!(!app.panel_loading);
        let rows = app.panel_rows();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].label, "a-fact");
        assert!(rows[0].detail.contains("the whole body"));
        assert_eq!(app.panel_filter_label(), "all");
        app.cycle_panel_filter();
        assert_eq!(app.panel_filter_label(), "live");
        app.cycle_panel_filter();
        assert_eq!(app.panel_filter_label(), "protected");
        assert_eq!(app.panel_rows().len(), 1);
        app.cycle_panel_filter();
        assert_eq!(app.panel_filter_label(), "retired");
        assert!(app.panel_rows().is_empty());
    }

    #[test]
    fn an_owner_stop_is_not_a_failure() {
        let mut app = app();
        app.apply(Action::Input('x'));
        app.apply(Action::Submit);
        app.apply_ui_event(UiEvent::Failed("stopped by the owner".to_string()));
        assert_eq!(app.run.stop_reason.as_deref(), Some("stopped by you"));
        assert!(app.error.is_none());
        assert_eq!(app.mode, Mode::Chat);
        // A real failure still banners.
        let mut failed = self::app();
        failed.apply(Action::Input('x'));
        failed.apply(Action::Submit);
        failed.apply_ui_event(UiEvent::Failed("the provider refused".to_string()));
        assert_eq!(failed.run.stop_reason.as_deref(), Some("failed"));
        assert!(failed.error.is_some());
    }
}
