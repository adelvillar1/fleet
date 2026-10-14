//! The TUI's state machine: everything the interface knows, with no
//! terminal, no crossterm, and no I/O of its own. The event loop feeds it
//! key actions and UI events and acts on the effects it returns;
//! rendering only reads it. Keeping it pure is what makes the interaction
//! testable without a terminal.

use crate::agent::{Approval, SubagentStage, UiEvent};
use crate::api::Usage;
use crate::lanes::LaneHealth;
use crate::session::{Record, SessionSummary};

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
    /// Open the lane health view.
    OpenLanes,
    /// Close whatever overlay is open.
    CloseOverlay,
    /// Move the picker selection; positive is down.
    Move(i32),
    /// Pick the selected session.
    Pick,
    /// Answer the pending approval.
    Approve(Approval),
    /// Scroll the conversation; positive is up.
    Scroll(i32),
}

/// Something the event loop must do outside the state machine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    /// Send this task to the agent.
    Run(String),
    /// Resume this session id: seed the conversation and append to its log.
    Resume(String),
    /// Answer the pending approval.
    Answer(Approval),
    /// Leave the TUI.
    Quit,
}

/// The prompt shown in the approval modal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalPrompt {
    pub name: String,
    pub summary: String,
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
    /// What the last run's assembled memory carried: the shape the
    /// instruction routed under (empty when unclassified) and how many
    /// facts entered the turn's context. `None` until a run assembles
    /// something, and reset when a resumed log is seeded — it describes
    /// the conversation on screen.
    pub memory: Option<(String, usize)>,
}

/// What the agent thread reports beside the `UiEvent` stream: exact token
/// accounting, which the stream does not carry.
#[derive(Debug, Clone, Copy)]
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
    pub approval: Option<ApprovalPrompt>,
    pub status: Status,
    /// A failure banner for the conversation pane.
    pub error: Option<String>,
    /// Lines scrolled back from the bottom; 0 means following the tail.
    pub scrolled_back: usize,
    /// True when the view sticks to the newest content.
    pub following: bool,
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
            lanes: Vec::new(),
            lanes_loading: false,
            lanes_probe: 0,
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
                memory: None,
            },
            error: None,
            scrolled_back: 0,
            following: true,
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
            Mode::Sessions => match action {
                Action::Move(delta) => {
                    self.move_selection(delta, self.sessions.len());
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
            Mode::Chat => match action {
                Action::Input(character) => {
                    self.push_input(character);
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
                Action::OpenLanes => {
                    self.open_lanes();
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
            UiEvent::Token(delta) => self.streaming.push_str(&delta),
            UiEvent::Turn(turn) => {
                self.flush_streaming();
                self.status.turn = turn;
                self.status.working = true;
            }
            UiEvent::ToolCall { name, summary } => {
                self.flush_streaming();
                self.entries.push(Entry::Tool(ToolCard {
                    name,
                    summary,
                    status: ToolStatus::Pending,
                    preview: String::new(),
                }));
            }
            UiEvent::ToolResult { name, preview, approved } => {
                let status = if approved { ToolStatus::Done } else { ToolStatus::Denied };
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
                self.entries.push(Entry::Lane(LaneCard { lane, tool_count }));
            }
            UiEvent::Routed { provider, how } => {
                // The route this run takes: the status bar shows the active
                // provider and how it was chosen.
                self.status.provider = Some(provider);
                self.status.route_how = Some(how);
            }
            UiEvent::Memory { shape, facts } => {
                // What the run's assembled memory carried: the status bar
                // shows the shape and the fact count, the live half of the
                // `memory` record the log carries.
                self.status.memory = Some((shape, facts));
            }
            UiEvent::Subagent { goal, session, stage } => {
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
                    SubagentStage::Working { calls, tool } => {
                        Some(format!("child {session} \u{b7} {calls} call(s) \u{b7} last {tool}"))
                    }
                    SubagentStage::Done { outcome, turns } => {
                        Some(format!("child {session} \u{b7} {outcome} \u{b7} {turns} turn(s)"))
                    }
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
            UiEvent::Finished { .. } => {
                self.flush_streaming();
                self.status.working = false;
                self.status.turn = 0;
            }
            UiEvent::Failed(message) => {
                self.flush_streaming();
                self.status.working = false;
                self.status.turn = 0;
                self.error = Some(message);
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
        self.selected = 0;
        self.mode = Mode::Sessions;
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

    /// Show an approval prompt; the responder stays with the event loop.
    pub fn show_approval(&mut self, name: &str, summary: &str) {
        self.approval =
            Some(ApprovalPrompt { name: name.to_string(), summary: summary.to_string() });
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
        self.entries.push(Entry::User(text.clone()));
        self.scrolled_back = 0;
        self.following = true;
        Some(Effect::Run(text))
    }

    fn pick_session(&mut self) -> Option<Effect> {
        // Resuming mid-run would mix the in-flight run's tokens into the
        // seeded view: the log stays correct, the counters would not.
        if self.status.working {
            self.error = Some("wait for the run to finish before resuming a session".to_string());
            return None;
        }
        let picked = self.sessions.get(self.selected)?.id.clone();
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
            self.entries.push(Entry::Assistant(std::mem::take(&mut self.streaming)));
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

/// One log record as a conversation block. Assistant turns that carried
/// neither text nor calls are transport noise, not conversation.
fn entry_of(record: &Record) -> Option<Entry> {
    match record {
        Record::User { content } => Some(Entry::User(content.clone())),
        Record::Assistant { content, tool_calls } => {
            let text = content.clone().unwrap_or_default();
            if text.trim().is_empty() && tool_calls.is_empty() {
                None
            } else {
                Some(Entry::Assistant(text))
            }
        }
        Record::ToolResult { name, output, approved, .. } => Some(Entry::Tool(ToolCard {
            name: name.clone(),
            summary: String::new(),
            status: if *approved { ToolStatus::Done } else { ToolStatus::Denied },
            preview: crate::agent::preview(output, 80),
        })),
        // A resumed conversation shows the lanes its earlier runs joined,
        // so the tool cards that follow read as the lane's own.
        Record::LaneTools { lane, tools } => {
            Some(Entry::Lane(LaneCard { lane: lane.clone(), tool_count: tools.len() }))
        }
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
        App::new("deepseek-chat", "session-1")
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
        assert_eq!(effect, Effect::Run("hi".to_string()));
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
        app.apply_ui_event(UiEvent::Finished { stop_reason: "completed".to_string() });
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
        }));
        app.seed(&[Record::User { content: "old task".to_string() }]);
        assert_eq!(app.status.prompt_tokens, 0);
        assert_eq!(app.status.completion_tokens, 0);
        // The count then runs for the resumed conversation only.
        app.apply_status(StatusUpdate::Usage(Usage {
            prompt_tokens: 7,
            completion_tokens: 3,
            total_tokens: 10,
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
        assert_eq!(app.conversation_lines(), vec![Entry::Assistant("hello".to_string())]);
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
        });
        app.apply_ui_event(UiEvent::ToolResult {
            name: "bash".to_string(),
            preview: "total 0".to_string(),
            approved: true,
        });
        let Entry::Tool(card) = &app.entries[0] else { panic!("expected a tool card") };
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
        });
        // Spawned with no open card still belongs on screen.
        let mut fresh = crate::tui::app::App::new("deepseek-chat", "session-1");
        fresh.apply_ui_event(UiEvent::Subagent {
            goal: "count the files".to_string(),
            session: "child-1".to_string(),
            stage: SubagentStage::Spawned,
        });
        let Entry::Tool(card) = &fresh.entries[0] else { panic!("expected a tool card") };
        assert_eq!(card.status, ToolStatus::Pending);
        assert_eq!(card.summary, "count the files");

        // With the card open, the child's progress updates it in place and
        // never adds a second one.
        app.apply_ui_event(UiEvent::Subagent {
            goal: "count the files".to_string(),
            session: "child-1".to_string(),
            stage: SubagentStage::Working { calls: 2, tool: "bash".to_string() },
        });
        assert_eq!(app.entries.len(), 1, "{:?}", app.entries);
        let Entry::Tool(card) = &app.entries[0] else { panic!("expected a tool card") };
        assert_eq!(card.preview, "child child-1 \u{b7} 2 call(s) \u{b7} last bash");
        // The status dot is the ordinary pair's, not the child's: a working
        // child leaves its card pending until the call's own result lands.
        assert_eq!(card.status, ToolStatus::Pending);

        app.apply_ui_event(UiEvent::Subagent {
            goal: "count the files".to_string(),
            session: "child-1".to_string(),
            stage: SubagentStage::Done { outcome: "completed".to_string(), turns: 3 },
        });
        app.apply_ui_event(UiEvent::ToolResult {
            name: "subagent".to_string(),
            preview: "subagent completed \u{b7} 3 turns".to_string(),
            approved: true,
        });
        assert_eq!(app.entries.len(), 1, "{:?}", app.entries);
        let Entry::Tool(card) = &app.entries[0] else { panic!("expected a tool card") };
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
        });
        app.apply_ui_event(UiEvent::LaneJoined { lane: "sem1".to_string(), tool_count: 4 });
        app.apply_ui_event(UiEvent::ToolResult {
            name: "mcp__sem1__probe".to_string(),
            preview: "reachable".to_string(),
            approved: true,
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
                Entry::Lane(LaneCard { lane: "sem1".to_string(), tool_count: 4 }),
            ]
        );
    }

    /// A resumed conversation shows the lanes its earlier runs joined, so
    /// the lane's tool cards that follow read as the lane's own.
    #[test]
    fn seeding_a_log_shows_the_lanes_that_joined_it() {
        let mut app = app();
        app.seed(&[
            Record::User { content: "probe it".to_string() },
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
                Entry::Lane(LaneCard { lane: "sem1".to_string(), tool_count: 2 }),
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
            Record::Start { v: 1, model: "m".to_string(), created: "c".to_string(), kind: None, parent: None },
            Record::Skills {
                skills: vec![crate::skills::SkillRef {
                    name: "bash-recovery".to_string(),
                    description: "Recover a failed bash call".to_string(),
                }],
            },
            Record::User { content: "run it".to_string() },
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
        });
        app.apply_ui_event(UiEvent::ToolResult {
            name: "bash".to_string(),
            preview: "denied: the user did not approve this call.".to_string(),
            approved: false,
        });
        let Entry::Tool(card) = &app.entries[0] else { panic!("expected a tool card") };
        assert_eq!(card.status, ToolStatus::Denied);
    }

    #[test]
    fn finished_and_failed_end_the_working_state() {
        let mut app = app();
        app.apply_ui_event(UiEvent::Turn(3));
        app.apply_ui_event(UiEvent::Finished { stop_reason: "completed".to_string() });
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
        app.apply_ui_event(UiEvent::Memory { shape: "deploy".to_string(), facts: 3 });
        assert_eq!(app.status.memory, Some(("deploy".to_string(), 3)));
        // An unclassified assembly says so with an empty shape.
        app.apply_ui_event(UiEvent::Memory { shape: String::new(), facts: 1 });
        assert_eq!(app.status.memory, Some((String::new(), 1)));
        // Seeding a resumed log clears it: the fact describes the
        // conversation on screen, and a resumed log's earlier runs
        // assembled theirs elsewhere.
        app.seed(&[Record::User { content: "old task".to_string() }]);
        assert_eq!(app.status.memory, None);
    }

    #[test]
    fn usage_accumulates_across_runs() {
        let mut app = app();
        app.apply_status(StatusUpdate::Usage(Usage {
            prompt_tokens: 10,
            completion_tokens: 5,
            total_tokens: 15,
        }));
        app.apply_status(StatusUpdate::Usage(Usage {
            prompt_tokens: 20,
            completion_tokens: 7,
            total_tokens: 27,
        }));
        assert_eq!(app.status.prompt_tokens, 30);
        assert_eq!(app.status.completion_tokens, 12);
    }

    #[test]
    fn the_picker_moves_clamped_and_picks_a_session() {
        let mut app = app();
        app.open_sessions(vec![
            summary("first"),
            summary("second"),
            summary("third"),
        ]);
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
        app.set_lanes(app.lanes_probe, vec![lane_health("sem1", crate::lanes::LaneState::Ok, 4)]);
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
        app.set_lanes(stale, vec![lane_health("sys1", crate::lanes::LaneState::Ok, 1)]);
        assert!(app.lanes.is_empty(), "stale rows must not render");
        assert!(app.lanes_loading, "the fresh probe is still in flight");
        // The fresh probe's own rows still land.
        app.set_lanes(fresh, vec![lane_health("sem1", crate::lanes::LaneState::Ok, 4)]);
        assert_eq!(app.lanes.len(), 1);
        assert_eq!(app.lanes[0].name, "sem1");
        assert!(!app.lanes_loading);
    }

    #[test]
    fn the_lanes_view_holds_no_menu_semantics() {
        let mut app = app();
        app.apply(Action::OpenLanes);
        app.set_lanes(app.lanes_probe, vec![lane_health("sem1", crate::lanes::LaneState::Ok, 4)]);
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
        let Entry::Tool(card) = &app.entries[0] else { panic!("expected a tool card") };
        assert_eq!(card.status, ToolStatus::Approved);
        // Its result lands as done.
        app.apply_ui_event(UiEvent::ToolResult {
            name: "bash".to_string(),
            preview: "exit 0".to_string(),
            approved: true,
        });
        let Entry::Tool(card) = &app.entries[0] else { panic!("expected a tool card") };
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
            Record::Start { v: 1, model: "m".to_string(), created: "c".to_string(), kind: None, parent: None },
            Record::User { content: "resumed task".to_string() },
            Record::Assistant { content: Some(String::new()), tool_calls: vec![] },
            Record::Assistant { content: Some("done".to_string()), tool_calls: vec![] },
            Record::ToolResult {
                call_id: "c1".to_string(),
                name: "bash".to_string(),
                output: "ok".to_string(),
                truncated: false,
                approved: true,
            },
            Record::End { stop_reason: "completed".to_string(), turns: 2 },
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
            id: id.to_string(),
            model: "deepseek-chat".to_string(),
            created: "2026-10-13T00:00:00Z".to_string(),
            ended: false,
            kind: None,
            path: std::path::PathBuf::from(format!("/tmp/{id}.jsonl")),
        }
    }
}
