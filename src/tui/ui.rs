//! Rendering: reads the state machine, writes cells. No behavior lives
//! here — every decision was made in `app.rs` before the frame is drawn.

use ratatui::layout::{Constraint, Flex, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Clear, Paragraph, Wrap};
use ratatui::Frame;

use super::app::{App, Entry, Mode, ToolStatus};

/// The palette. One home per role, so the surface stays coherent as later
/// slices extend it.
mod palette {
    use ratatui::style::Color;

    pub const ACCENT: Color = Color::Cyan;
    pub const USER: Color = Color::Blue;
    pub const DIM: Color = Color::DarkGray;
    pub const TEXT: Color = Color::Gray;
    pub const PENDING: Color = Color::Yellow;
    pub const APPROVED: Color = Color::Cyan;
    pub const DENIED: Color = Color::Red;
    pub const DONE: Color = Color::Green;
    pub const WORKING: Color = Color::Yellow;
    pub const IDLE: Color = Color::Green;
    pub const BOARD: Color = Color::DarkGray;
}

/// Draw one frame.
pub fn render(frame: &mut Frame<'_>, app: &App) {
    let [main, error_bar, status, input, footer] = Layout::vertical([
        Constraint::Min(3),
        Constraint::Length(if app.error.is_some() { 1 } else { 0 }),
        Constraint::Length(1),
        Constraint::Length(5),
        Constraint::Length(1),
    ])
    .areas(frame.area());

    let [conversation, board] =
        Layout::horizontal([Constraint::Min(20), Constraint::Length(26)]).areas(main);

    render_conversation(frame, app, conversation);
    render_board(frame, board);
    if let Some(message) = &app.error {
        render_error(frame, message, error_bar);
    }
    render_status(frame, app, status);
    render_input(frame, app, input);
    render_footer(frame, footer);

    match app.mode {
        Mode::Chat => {}
        Mode::Sessions => render_picker(frame, app),
        Mode::Approval => render_approval(frame, app),
    }
}

/// The conversation pane: user turns, assistant text, and tool cards with
/// their status dots, newest at the bottom.
fn render_conversation(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let block = Block::bordered()
        .title(" conversation ")
        .border_style(Style::new().fg(palette::DIM));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.height == 0 {
        return;
    }

    let mut lines: Vec<Line<'_>> = Vec::new();
    for entry in app.conversation_lines() {
        lines.extend(entry_lines(&entry, inner.width as usize));
    }
    let total = lines.len();
    let visible = inner.height as usize;
    let furthest = total.saturating_sub(visible);
    let offset = if app.following {
        furthest
    } else {
        furthest.saturating_sub(app.scrolled_back)
    };
    let paragraph = Paragraph::new(Text::from(lines))
        .scroll((offset.min(u16::MAX as usize) as u16, 0));
    frame.render_widget(paragraph, inner);
}

/// The board pane mount point; slice 14 fills it.
fn render_board(frame: &mut Frame<'_>, area: Rect) {
    let block = Block::bordered()
        .title(" board ")
        .border_style(Style::new().fg(palette::BOARD));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let placeholder = Paragraph::new(Text::from(vec![
        Line::from(""),
        Line::from(Span::styled("board mounts here", Style::new().fg(palette::BOARD))),
        Line::from(Span::styled("slice 14 fills it", Style::new().fg(palette::BOARD))),
    ]))
    .wrap(Wrap { trim: false });
    frame.render_widget(placeholder, inner);
}

/// The one-line failure banner.
fn render_error(frame: &mut Frame<'_>, message: &str, area: Rect) {
    let line = Line::from(vec![
        Span::styled(" ! ", Style::new().fg(palette::DENIED).add_modifier(Modifier::BOLD)),
        Span::styled(message, Style::new().fg(palette::DENIED)),
    ]);
    frame.render_widget(Paragraph::new(line), area);
}

/// The status bar: model, session, turn, tokens, and the run's state.
fn render_status(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let status = &app.status;
    let (dot, state) = if status.working {
        (palette::WORKING, "working")
    } else {
        (palette::IDLE, "idle")
    };
    let turn = if status.turn == 0 { "—".to_string() } else { status.turn.to_string() };
    let line = Line::from(vec![
        Span::styled(" ● ", Style::new().fg(dot)),
        Span::styled(state, Style::new().fg(dot)),
        Span::styled("  ", Style::new()),
        Span::styled(&status.model, Style::new().fg(palette::ACCENT)),
        Span::styled("  session ", Style::new().fg(palette::DIM)),
        Span::styled(short_id(&status.session_id), Style::new().fg(palette::TEXT)),
        Span::styled("  turn ", Style::new().fg(palette::DIM)),
        Span::styled(turn, Style::new().fg(palette::TEXT)),
        Span::styled("  tokens ", Style::new().fg(palette::DIM)),
        Span::styled(
            format!("{}/{}", status.prompt_tokens, status.completion_tokens),
            Style::new().fg(palette::TEXT),
        ),
    ]);
    frame.render_widget(Paragraph::new(line), area);
}

/// The input box with its cursor.
fn render_input(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let block = Block::bordered()
        .title(" message ")
        .border_style(Style::new().fg(palette::ACCENT));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.height == 0 || inner.width == 0 {
        return;
    }

    let (lines, cursor) = input_lines(&app.input, inner.width as usize, inner.height as usize);
    let paragraph = Paragraph::new(Text::from(lines));
    frame.render_widget(paragraph, inner);
    if let Some((x, y)) = cursor {
        frame.set_cursor_position((inner.x + x, inner.y + y));
    }
}

/// The key hints.
fn render_footer(frame: &mut Frame<'_>, area: Rect) {
    let hints = "enter send · alt+enter newline · ctrl+o sessions · ↑↓ scroll · q quit · y/n/a approve";
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(hints, Style::new().fg(palette::DIM)))),
        area,
    );
}

/// The approval modal over the conversation.
fn render_approval(frame: &mut Frame<'_>, app: &App) {
    let Some(prompt) = &app.approval else { return };
    let area = centered(64, 36, frame.area());
    frame.render_widget(Clear, area);
    let block = Block::bordered()
        .title(" approval ")
        .border_style(Style::new().fg(palette::PENDING));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let body = Text::from(vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(&prompt.name, Style::new().fg(palette::ACCENT).add_modifier(Modifier::BOLD)),
            Span::styled("  ", Style::new()),
            Span::styled(&prompt.summary, Style::new().fg(palette::TEXT)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(" [y]", Style::new().fg(palette::DONE).add_modifier(Modifier::BOLD)),
            Span::styled(" yes   ", Style::new().fg(palette::TEXT)),
            Span::styled("[n]", Style::new().fg(palette::DENIED).add_modifier(Modifier::BOLD)),
            Span::styled(" no   ", Style::new().fg(palette::TEXT)),
            Span::styled("[a]", Style::new().fg(palette::PENDING).add_modifier(Modifier::BOLD)),
            Span::styled(" always", Style::new().fg(palette::TEXT)),
        ]),
        Line::from(Span::styled(
            " any other key denies this call",
            Style::new().fg(palette::DIM),
        )),
    ]);
    frame.render_widget(Paragraph::new(body).wrap(Wrap { trim: false }), inner);
}

/// The session picker over the conversation: oldest first, exactly as
/// `fleet sessions` lists them.
fn render_picker(frame: &mut Frame<'_>, app: &App) {
    let area = centered(78, 62, frame.area());
    frame.render_widget(Clear, area);
    let block = Block::bordered()
        .title(" sessions — enter resume, esc close ")
        .border_style(Style::new().fg(palette::ACCENT));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.height == 0 {
        return;
    }

    if app.sessions.is_empty() {
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                " no sessions yet",
                Style::new().fg(palette::DIM),
            ))),
            inner,
        );
        return;
    }

    let visible = inner.height as usize;
    // Keep the selection in view.
    let first = app.selected.saturating_sub(visible.saturating_sub(1));
    let mut lines: Vec<Line<'_>> = Vec::new();
    for (index, summary) in app.sessions.iter().enumerate().skip(first).take(visible) {
        let selected = index == app.selected;
        let marker = if selected { "▸" } else { " " };
        let (dot, state) = if summary.ended {
            (palette::DONE, "ended")
        } else {
            (palette::PENDING, "open")
        };
        let current = summary.id == app.status.session_id;
        let mut spans = vec![
            Span::styled(format!("{marker} "), Style::new().fg(palette::ACCENT)),
            Span::styled("● ", Style::new().fg(dot)),
            Span::styled(
                summary.id.clone(),
                if selected {
                    Style::new().fg(palette::TEXT).add_modifier(Modifier::BOLD)
                } else {
                    Style::new().fg(palette::TEXT)
                },
            ),
            Span::styled(format!("  {}  ", summary.model), Style::new().fg(palette::DIM)),
            Span::styled(state, Style::new().fg(dot)),
        ];
        if current {
            spans.push(Span::styled("  (current)", Style::new().fg(palette::ACCENT)));
        }
        lines.push(Line::from(spans));
    }
    frame.render_widget(Paragraph::new(Text::from(lines)), inner);
}

/// One conversation entry as wrapped lines: tool calls are cards with a
/// status dot and a result preview.
fn entry_lines(entry: &Entry, width: usize) -> Vec<Line<'static>> {
    let width = width.max(8);
    match entry {
        Entry::User(text) => wrap(text, width)
            .into_iter()
            .enumerate()
            .map(|(index, chunk)| {
                if index == 0 {
                    Line::from(vec![
                        Span::styled("you ", Style::new().fg(palette::USER).add_modifier(Modifier::BOLD)),
                        Span::styled("▸ ", Style::new().fg(palette::USER)),
                        Span::styled(chunk, Style::new().fg(palette::TEXT)),
                    ])
                } else {
                    Line::from(Span::styled(format!("    {chunk}"), Style::new().fg(palette::TEXT)))
                }
            })
            .collect(),
        Entry::Assistant(text) => wrap(text, width)
            .into_iter()
            .map(|chunk| Line::from(Span::styled(chunk, Style::new().fg(palette::TEXT))))
            .collect(),
        Entry::Tool(card) => {
            let (dot, state) = match card.status {
                ToolStatus::Pending => (palette::PENDING, "pending"),
                ToolStatus::Approved => (palette::APPROVED, "approved"),
                ToolStatus::Denied => (palette::DENIED, "denied"),
                ToolStatus::Done => (palette::DONE, "done"),
            };
            let mut lines = vec![Line::from(vec![
                Span::styled(format!(" {} ", dot_marker(card.status)), Style::new().fg(dot)),
                Span::styled(
                    card.name.clone(),
                    Style::new().fg(palette::ACCENT).add_modifier(Modifier::BOLD),
                ),
                Span::styled("  ", Style::new()),
                Span::styled(card.summary.clone(), Style::new().fg(palette::DIM)),
                Span::styled(format!("  {state}"), Style::new().fg(dot)),
            ])];
            if !card.preview.is_empty() {
                for chunk in wrap(&card.preview, width.saturating_sub(4).max(8)) {
                    lines.push(Line::from(vec![
                        Span::styled("   └ ", Style::new().fg(palette::DIM)),
                        Span::styled(chunk, Style::new().fg(palette::DIM)),
                    ]));
                }
            }
            lines
        }
    }
}

/// The dot itself: filled for settled states, open for the pending one.
fn dot_marker(status: ToolStatus) -> &'static str {
    match status {
        ToolStatus::Pending => "◌",
        ToolStatus::Approved => "●",
        ToolStatus::Denied => "✕",
        ToolStatus::Done => "✓",
    }
}

/// Wrap `text` into `width`-wide lines, breaking on spaces where possible
/// so words stay whole. Exact line counts matter: the conversation's
/// scroll offset is computed from them.
fn wrap(text: &str, width: usize) -> Vec<String> {
    let width = width.max(8);
    let mut lines: Vec<String> = Vec::new();
    for paragraph in text.split('\n') {
        if paragraph.is_empty() {
            lines.push(String::new());
            continue;
        }
        let mut current = String::new();
        for word in paragraph.split_inclusive(' ') {
            let word_width = word.chars().count();
            // A word longer than the line is split hard; it has no choice.
            if !current.is_empty() && current.chars().count() + word_width > width {
                lines.push(current.trim_end().to_string());
                current.clear();
            }
            for character in word.chars() {
                current.push(character);
                if current.chars().count() >= width {
                    lines.push(current.trim_end().to_string());
                    current.clear();
                }
            }
        }
        if !current.is_empty() {
            lines.push(current.trim_end().to_string());
        }
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

/// The input box's visible lines (its tail) and where its cursor sits.
fn input_lines(input: &str, width: usize, height: usize) -> (Vec<Line<'static>>, Option<(u16, u16)>) {
    let width = width.max(8);
    let all: Vec<String> = wrap(input, width);
    let height = height.max(1);
    let skip = all.len().saturating_sub(height);
    let visible: Vec<String> = all.iter().skip(skip).cloned().collect();
    let lines: Vec<Line<'static>> = visible
        .iter()
        .map(|line| Line::from(Span::styled(line.clone(), Style::new().fg(palette::TEXT))))
        .collect();
    let last_width = visible.last().map(|line| line.chars().count()).unwrap_or(0) as u16;
    let row = visible.len().saturating_sub(1) as u16;
    let cursor = if last_width < width as u16 {
        Some((last_width, row))
    } else {
        None
    };
    (lines, cursor)
}

/// A centered overlay of the given percentages.
fn centered(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let [area] =
        Layout::vertical([Constraint::Percentage(percent_y)]).flex(Flex::Center).areas(area);
    let [area] =
        Layout::horizontal([Constraint::Percentage(percent_x)]).flex(Flex::Center).areas(area);
    area
}

/// The session id's distinctive tail, so the status bar keeps its shape.
fn short_id(id: &str) -> String {
    let count = id.chars().count();
    if count <= 24 {
        id.to_string()
    } else {
        format!("…{}", id.chars().skip(count - 23).collect::<String>())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::UiEvent;
    use crate::session::SessionSummary;
    use crate::tui::app::{Action, ApprovalPrompt, App, Entry, Mode};
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    /// Render `app` into a test terminal and return the drawn text.
    fn draw(app: &App, width: u16, height: u16) -> String {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| render(frame, app)).unwrap();
        let buffer = terminal.backend().buffer().clone();
        let mut text = String::new();
        for y in 0..height {
            for x in 0..width {
                text.push_str(buffer[(x, y)].symbol());
            }
            text.push('\n');
        }
        text
    }

    fn fresh() -> App {
        App::new("deepseek-chat", "20261013T101010Z-abcdef")
    }

    #[test]
    fn every_mode_renders_without_a_panic() {
        let mut app = fresh();
        app.apply_ui_event(UiEvent::Turn(2));
        app.apply_ui_event(UiEvent::Token("thinking out loud".to_string()));
        app.apply_ui_event(UiEvent::ToolCall {
            name: "bash".to_string(),
            summary: "ls -la".to_string(),
        });
        app.apply_ui_event(UiEvent::ToolResult {
            name: "bash".to_string(),
            preview: "exit 0".to_string(),
            approved: true,
        });
        app.entries.push(Entry::User("do the thing".to_string()));

        let chat = draw(&app, 80, 24);
        assert!(chat.contains("conversation"), "{chat}");
        assert!(chat.contains("board"), "{chat}");
        assert!(chat.contains("deepseek-chat"), "{chat}");
        assert!(chat.contains("thinking out loud"), "{chat}");
        assert!(chat.contains("do the thing"), "{chat}");
        assert!(chat.contains("ls -la"), "{chat}");
        assert!(chat.contains("exit 0"), "{chat}");
        assert!(chat.contains("session"), "{chat}");

        // The picker overlay.
        app.open_sessions(vec![SessionSummary {
            id: "20261013T101010Z-abcdef".to_string(),
            model: "deepseek-chat".to_string(),
            created: "2026-10-13T10:10:10Z".to_string(),
            ended: true,
            path: std::path::PathBuf::from("/tmp/x.jsonl"),
        }]);
        let picker = draw(&app, 80, 24);
        assert!(picker.contains("sessions"), "{picker}");
        assert!(picker.contains("20261013T101010Z-abcdef"), "{picker}");
        assert!(picker.contains("ended"), "{picker}");
        assert!(picker.contains("(current)"), "{picker}");

        // The approval modal.
        let mut approving = fresh();
        approving.approval = Some(ApprovalPrompt {
            name: "bash".to_string(),
            summary: "rm -rf build".to_string(),
        });
        approving.mode = Mode::Approval;
        let modal = draw(&approving, 80, 24);
        assert!(modal.contains("approval"), "{modal}");
        assert!(modal.contains("rm -rf build"), "{modal}");
        assert!(modal.contains("always"), "{modal}");

        // A tiny terminal still renders: the panes clamp instead of dying.
        let tiny = draw(&fresh(), 20, 6);
        assert!(tiny.contains("conversation"), "{tiny}");
    }

    #[test]
    fn the_status_bar_shows_the_error_banner_when_a_run_fails() {
        let mut app = fresh();
        app.apply_ui_event(UiEvent::Failed("402 Insufficient Balance".to_string()));
        let drawn = draw(&app, 80, 24);
        assert!(drawn.contains("402 Insufficient Balance"), "{drawn}");
    }

    #[test]
    fn a_scrolled_back_conversation_keeps_its_older_lines() {
        let mut app = fresh();
        for index in 0..40 {
            app.entries.push(Entry::Assistant(format!("line {index}")));
        }
        let tail = draw(&app, 60, 12);
        assert!(tail.contains("line 39"), "{tail}");
        assert!(!tail.contains("line 0"), "{tail}");
        // Scrolled back far enough, the oldest lines are visible again.
        app.apply(Action::Scroll(50));
        let back = draw(&app, 60, 12);
        assert!(back.contains("line 0"), "{back}");
        assert!(!back.contains("line 39"), "{back}");
    }

    #[test]
    fn wrapping_keeps_words_whole_and_hard_splits_the_impossible() {
        assert_eq!(wrap("one two three", 7), vec!["one two", "three"]);
        assert_eq!(wrap("a\n\nb", 10), vec!["a", "", "b"]);
        // Width is clamped to 8, and a single word wider than the line is
        // split hard rather than dropped.
        assert_eq!(wrap("abcdefghij", 4), vec!["abcdefgh", "ij"]);
        assert_eq!(wrap("0123456789abcdefghij", 8), vec!["01234567", "89abcdef", "ghij"]);
        assert_eq!(wrap("", 10), vec![""]);
    }
}
