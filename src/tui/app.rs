//! The TUI's state machine: everything the interface knows, with no
//! terminal, no crossterm, and no I/O of its own. The event loop feeds it
//! key actions and UI events and acts on the effects it returns;
//! rendering only reads it. Keeping it pure is what makes the interaction
//! testable without a terminal.

use crate::agent::{Approval, UiEvent};
use crate::api::Usage;
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
}

/// One key press, translated for the current mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Type a character into the input box.
    Input(char),
    /// Delete the character before the cursor.
    Backspace,
    /// Submit the input box.
    Submit,
    /// Leave the TUI.
    Quit,
    /// Open the session picker.
    OpenSessions,
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
    /// Tokens counted across every run in this session.
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    /// True while a run is in flight.
    pub working: bool,
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
    pub mode: Mode,
    /// The picker's rows, oldest first, exactly as `session::list` returns.
    pub sessions: Vec<SessionSummary>,
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
            mode: Mode::Chat,
            sessions: Vec::new(),
            selected: 0,
            approval: None,
            status: Status {
                model: model.to_string(),
                session_id: session_id.to_string(),
                turn: 0,
                prompt_tokens: 0,
                completion_tokens: 0,
                working: false,
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
                    self.move_selection(delta);
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
            Mode::Chat => match action {
                Action::Input(character) => {
                    self.push_input(character);
                    None
                }
                Action::Backspace => {
                    self.input.pop();
                    None
                }
                Action::Submit => self.submit(),
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
    }

    /// The conversation's text, streaming tail included, for rendering.
    pub fn conversation_lines(&self) -> Vec<Entry> {
        let mut lines = self.entries.clone();
        if !self.streaming.is_empty() {
            lines.push(Entry::Assistant(self.streaming.clone()));
        }
        lines
    }

    /// Type one character, honoring the input cap.
    pub fn push_input(&mut self, character: char) {
        if self.input.chars().count() < MAX_INPUT {
            self.input.push(character);
        }
    }

    fn submit(&mut self) -> Option<Effect> {
        let text = self.input.trim().to_string();
        if text.is_empty() {
            return None;
        }
        self.input.clear();
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
        let picked = self.sessions.get(self.selected)?.id.clone();
        self.mode = Mode::Chat;
        Some(Effect::Resume(picked))
    }

    fn move_selection(&mut self, delta: i32) {
        if self.sessions.is_empty() {
            self.selected = 0;
            return;
        }
        let last = self.sessions.len() as i32 - 1;
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
        app.apply(Action::Backspace);
        assert_eq!(app.input.chars().count(), MAX_INPUT - 1);
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
            Record::Start { v: 1, model: "m".to_string(), created: "c".to_string() },
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
            path: std::path::PathBuf::from(format!("/tmp/{id}.jsonl")),
        }
    }
}
