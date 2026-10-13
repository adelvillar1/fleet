//! Rendering: reads the state machine, writes cells. No behavior lives
//! here — every decision was made in `app.rs` before the frame is drawn.

use ratatui::layout::{Constraint, Flex, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Clear, Paragraph, Wrap};
use ratatui::Frame;

use super::app::{App, Entry, Mode, ToolStatus};
use crate::lanes::LaneState;

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
        Mode::Lanes => render_lanes(frame, app),
    }
}

/// Every pane's title, one style: the borders stay dim, the words on them
/// do not — dim-on-black titles were the least readable text on screen.
fn title_style() -> Style {
    Style::new().fg(palette::TEXT)
}

/// The conversation pane: user turns, assistant text, and tool cards with
/// their status dots, newest at the bottom.
fn render_conversation(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let block = Block::bordered()
        .title(" conversation ")
        .title_style(title_style())
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
    // A fresh session is not an empty void: the pane says what this is and
    // what to do, so the first frame reads as intentional rather than broken.
    if lines.is_empty() {
        let welcome = Text::from(vec![
            Line::from(Span::styled(
                "fleet",
                Style::new().fg(palette::ACCENT).add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                "a native agent on this machine.",
                Style::new().fg(palette::TEXT),
            )),
            Line::from(Span::styled(
                "type a task below — tools ask before they run.",
                Style::new().fg(palette::DIM),
            )),
        ]);
        frame.render_widget(Paragraph::new(welcome), inner);
        return;
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
        .title_style(title_style())
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
        .title_style(title_style())
        .border_style(Style::new().fg(palette::ACCENT));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.height == 0 || inner.width == 0 {
        return;
    }

    let (lines, cursor) =
        input_lines(&app.input, app.cursor, inner.width as usize, inner.height as usize);
    let paragraph = Paragraph::new(Text::from(lines));
    frame.render_widget(paragraph, inner);
    if let Some((x, y)) = cursor {
        frame.set_cursor_position((inner.x + x, inner.y + y));
    }
}

/// The key hints.
fn render_footer(frame: &mut Frame<'_>, area: Rect) {
    let hints = "enter send · alt+enter newline · ctrl+o sessions · ctrl+l lanes · q quit";
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(hints, Style::new().fg(palette::TEXT)))),
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
        .title_style(title_style())
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
        .title_style(title_style())
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

/// The lanes modal's height in rows: two lines per lane, the border, and a
/// line of headroom. Four two-line rows do not need 62% of the screen —
/// the headless card renders the same facts in far less, and the surfaces
/// are meant to read as one.
fn lanes_height(rows: usize) -> u16 {
    let wanted = (rows as u16).saturating_mul(2).saturating_add(4);
    wanted.clamp(4, 20)
}

/// The lane health view over the conversation: one card-like row per
/// lane — status dot, name, role, version, tool count, state — with the
/// tool names (or the failure reason) underneath, in registry order.
fn render_lanes(frame: &mut Frame<'_>, app: &App) {
    let area = centered_sized(78, lanes_height(app.lanes.len()), frame.area());
    frame.render_widget(Clear, area);
    let block = Block::bordered()
        .title(" lanes — esc close ")
        .title_style(title_style())
        .border_style(Style::new().fg(palette::ACCENT));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.height == 0 {
        return;
    }

    if app.lanes.is_empty() && app.lanes_loading {
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                " probing the lanes…",
                Style::new().fg(palette::DIM),
            ))),
            inner,
        );
        return;
    }

    let mut lines: Vec<Line<'_>> = Vec::new();
    for (index, lane) in app.lanes.iter().enumerate() {
        let selected = index == app.selected;
        let marker = if selected { "▸" } else { " " };
        let (dot, color) = match lane.state {
            LaneState::Ok => ("●", palette::DONE),
            LaneState::Unavailable => ("○", palette::DIM),
            LaneState::Error => ("✕", palette::DENIED),
        };
        let version = lane.server_version.as_deref().unwrap_or("—");
        let name_style = if selected {
            Style::new().fg(palette::TEXT).add_modifier(Modifier::BOLD)
        } else {
            Style::new().fg(palette::TEXT)
        };
        lines.push(Line::from(vec![
            Span::styled(format!("{marker} "), Style::new().fg(palette::ACCENT)),
            Span::styled(format!("{dot} "), Style::new().fg(color)),
            Span::styled(lane.name.clone(), name_style),
            Span::styled(format!("  {}  ", lane.role), Style::new().fg(palette::DIM)),
            Span::styled(format!("v{version}"), Style::new().fg(palette::DIM)),
            Span::styled(
                format!("  {} tool{}", lane.tools.len(), if lane.tools.len() == 1 { "" } else { "s" }),
                Style::new().fg(palette::TEXT),
            ),
            Span::styled(format!("  {}", lane.state.label()), Style::new().fg(color)),
        ]));
        // The latency, exactly as the headless card renders it — the two
        // surfaces are meant to read as one.
        if lane.source.is_some() {
            lines.last_mut().expect("the row just pushed").push_span(Span::styled(
                format!("  · {} ms", lane.elapsed_ms),
                Style::new().fg(palette::DIM),
            ));
        }
        // The detail line: the tool names when the lane answered, the
        // reason it could not when it did not.
        let detail = if lane.tools.is_empty() {
            lane.detail.clone()
        } else {
            lane.tools.join(" · ")
        };
        lines.push(Line::from(Span::styled(
            format!("   {detail}"),
            Style::new().fg(palette::DIM),
        )));
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
        Entry::Lane(card) => {
            // A lane joining the run is a one-line fact in the same
            // family as a tool card: a dot, the lane, and what it brought.
            let tools = if card.tool_count == 1 { "1 tool" } else { " tools" };
            let count = if card.tool_count == 1 { String::new() } else { card.tool_count.to_string() };
            vec![Line::from(vec![
                Span::styled(" \u{2713} ", Style::new().fg(palette::DONE)),
                Span::styled(
                    card.lane.clone(),
                    Style::new().fg(palette::ACCENT).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!(" joined \u{b7} {count}{tools}"),
                    Style::new().fg(palette::DIM),
                ),
            ])]
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
/// so words stay whole. The break lands *after* the space, so no
/// character is dropped, moved, or invented: the lines' lengths account
/// for every input character, and the cursor's row and column are found
/// by walking them with the same rules. Exact line counts matter too —
/// the conversation's scroll offset is computed from them.
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
                lines.push(std::mem::take(&mut current));
            }
            for character in word.chars() {
                current.push(character);
                if current.chars().count() >= width {
                    lines.push(std::mem::take(&mut current));
                }
            }
        }
        if !current.is_empty() {
            lines.push(current);
        }
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

/// Where the cursor's `index`-th character lands once `text` is wrapped:
/// the row and column in characters, read straight off `wrap`'s output so
/// the two can never disagree. A cursor on a line boundary shows at the
/// next line's start, keeping it inside the box; the end of the text
/// stays where it is. Saturates at the end of the text.
fn cursor_position(text: &str, index: usize, width: usize) -> (usize, usize) {
    let index = index.min(text.chars().count());
    let width = width.max(8);
    let paragraphs: Vec<&str> = text.split('\n').collect();
    let mut row = 0usize;
    let mut seen_input = 0usize;
    for (number, paragraph) in paragraphs.iter().enumerate() {
        let para_len = paragraph.chars().count();
        let lines = wrap(paragraph, width);
        let last_para = number + 1 == paragraphs.len();
        if index <= seen_input + para_len {
            // The cursor is inside this paragraph (or at its end).
            let within = index - seen_input;
            let mut seen = 0usize;
            for (offset, line) in lines.iter().enumerate() {
                let len = line.chars().count();
                let last_line = offset + 1 == lines.len();
                if within < seen + len {
                    return (row + offset, within - seen);
                }
                if within == seen + len {
                    // A boundary shows on the next line when there is
                    // one — here or in the next paragraph — so the caret
                    // stays inside the box; the text's very end stays.
                    if !last_line || !last_para {
                        return (row + offset + 1, 0);
                    }
                    return (row + offset, len);
                }
                seen += len;
            }
        }
        seen_input += para_len + 1;
        row += lines.len();
    }
    // Unreachable: the clamped index always lands in a paragraph.
    (row.saturating_sub(1), 0)
}

/// The input box's visible lines and where its cursor sits among them.
/// The window follows the tail, but never scrolls the cursor out of
/// view: wherever the cursor sits, it stays on screen.
fn input_lines(
    input: &str,
    cursor: usize,
    width: usize,
    height: usize,
) -> (Vec<Line<'static>>, Option<(u16, u16)>) {
    let width = width.max(8);
    let all: Vec<String> = wrap(input, width);
    let height = height.max(1);
    let (cursor_row, cursor_col) = cursor_position(input, cursor, width);
    // The window follows the tail, but never scrolls the cursor out of
    // view: it starts at the tail unless the cursor sits above it, and
    // never more than a screen above the cursor.
    let tail = all.len().saturating_sub(height);
    let skip = tail.max(cursor_row.saturating_sub(height - 1)).min(cursor_row);
    let visible: Vec<String> = all.iter().skip(skip).take(height).cloned().collect();
    let lines: Vec<Line<'static>> = visible
        .iter()
        .map(|line| Line::from(Span::styled(line.clone(), Style::new().fg(palette::TEXT))))
        .collect();
    let row = cursor_row.saturating_sub(skip);
    let cursor = (row < visible.len() && cursor_col < width).then_some((cursor_col as u16, row as u16));
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

/// A centered overlay of the given width percentage and an exact height in
/// rows. `centered`'s percentages cannot express a small height — 4% of a
/// 24-row frame rounds to zero rows, and a zero-row modal renders nothing
/// — so a view sized from its own content asks for rows.
fn centered_sized(percent_x: u16, height: u16, area: Rect) -> Rect {
    let height = height.min(area.height);
    let [area] = Layout::vertical([Constraint::Length(height)]).flex(Flex::Center).areas(area);
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
    fn the_lanes_view_renders_dots_names_and_tool_counts() {
        let mut app = fresh();
        app.apply(crate::tui::app::Action::OpenLanes);
        // Before the probe lands, the view says it is probing.
        let probing = draw(&app, 80, 24);
        assert!(probing.contains("lanes"), "{probing}");
        assert!(probing.contains("probing the lanes"), "{probing}");

        app.set_lanes(
            app.lanes_probe,
            vec![
                crate::lanes::LaneHealth {
                    name: "sem1".to_string(),
                    role: "embeddings".to_string(),
                    home: Some(std::path::PathBuf::from("/repo/services/sem1")),
                    source: Some(crate::lanes::HomeSource::Bundled),
                    state: LaneState::Ok,
                    server_version: Some("0.1.0".to_string()),
                    tools: vec!["sem1_embed".to_string(), "sem1_probe".to_string()],
                    detail: "SEM1_PYTHONPATH=…/src python3 -u …/service/mcp_server.py".to_string(),
                    elapsed_ms: 53,
                },
                crate::lanes::LaneHealth {
                    name: "gen1".to_string(),
                    role: "media".to_string(),
                    home: None,
                    source: None,
                    state: LaneState::Unavailable,
                    server_version: None,
                    tools: Vec::new(),
                    detail: "no services root found".to_string(),
                    elapsed_ms: 0,
                },
            ],
        );
        let drawn = draw(&app, 80, 24);
        assert!(drawn.contains("lanes"), "{drawn}");
        assert!(drawn.contains("sem1"), "{drawn}");
        assert!(drawn.contains("embeddings"), "{drawn}");
        assert!(drawn.contains("v0.1.0"), "{drawn}");
        assert!(drawn.contains("2 tools"), "{drawn}");
        assert!(drawn.contains("ok"), "{drawn}");
        assert!(drawn.contains("sem1_embed · sem1_probe"), "{drawn}");
        // The latency renders here exactly as the headless card renders
        // it — the two surfaces read as one.
        assert!(drawn.contains("· 53 ms"), "{drawn}");
        // The unavailable lane shows its dot, its state, and the reason.
        assert!(drawn.contains("gen1"), "{drawn}");
        assert!(drawn.contains("unavailable"), "{drawn}");
        assert!(drawn.contains("no services root found"), "{drawn}");
        // A tiny terminal still renders the modal without a panic.
        let tiny = draw(&app, 20, 6);
        assert!(tiny.contains("lanes"), "{tiny}");
    }

    #[test]
    fn the_lanes_modal_is_sized_from_its_rows_not_a_fixed_share() {
        // Four two-line rows must not cost 62% of the screen.
        assert_eq!(lanes_height(0), 4, "the probing view is one line plus its border");
        assert_eq!(lanes_height(4), 12, "four rows: two lines each, plus four");
        // Clamped at both ends, so a small frame or an unexpected row
        // count cannot push the modal off screen.
        assert_eq!(lanes_height(40), 20);
        assert!(lanes_height(4) < 62, "the modal must not keep the fixed share");
    }

    #[test]
    fn a_lane_joining_renders_its_name_and_its_tool_count() {
        let mut app = fresh();
        app.apply_ui_event(UiEvent::ToolCall {
            name: "mcp__sem1__probe".to_string(),
            summary: "embeddings \u{b7} probe {}".to_string(),
        });
        app.apply_ui_event(UiEvent::LaneJoined { lane: "sem1".to_string(), tool_count: 4 });
        let drawn = draw(&app, 80, 24);
        // The lane's block names the lane and what it brought, and the
        // tool card beside it shows the model-visible namespaced name.
        assert!(drawn.contains("sem1"), "{drawn}");
        assert!(drawn.contains("joined"), "{drawn}");
        assert!(drawn.contains("4 tools"), "{drawn}");
        assert!(drawn.contains("mcp__sem1__probe"), "{drawn}");
        // A single tool reads as one, not "1 tools".
        let mut single = fresh();
        single.apply_ui_event(UiEvent::LaneJoined { lane: "gen1".to_string(), tool_count: 1 });
        let one = draw(&single, 80, 24);
        assert!(one.contains("1 tool"), "{one}");
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
    fn a_fresh_session_says_what_it_is() {
        let drawn = draw(&fresh(), 80, 24);
        assert!(drawn.contains("fleet"), "{drawn}");
        assert!(drawn.contains("type a task below"), "{drawn}");
    }

    #[test]
    fn the_input_blocks_four_borders_share_one_style() {
        // The tmux capture showed the input box's top and right borders
        // styled while its left and bottom rendered default — either a
        // ratatui split or a capture artifact. The buffer is the truth.
        let app = fresh();
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let buffer = terminal.backend().buffer();
        // In an 80x24 frame the input box owns rows 18..22: row 18 is its
        // top border, row 22 its bottom.
        let corners = [(0, 18), (79, 18), (0, 19), (79, 19), (0, 22), (79, 22)];
        let styles: Vec<_> = corners.iter().map(|(x, y)| buffer[(*x, *y)].fg).collect();
        assert!(
            styles.windows(2).all(|pair| pair[0] == pair[1]),
            "the input border is not one style: {styles:?}"
        );
    }

    #[test]
    fn wrapping_keeps_words_whole_and_hard_splits_the_impossible() {
        // The break lands after the space, so the wrapped lines keep
        // every input character — that exactness is what the cursor
        // walks.
        assert_eq!(
            wrap("aaaa bbbb cccc dddd", 10),
            vec!["aaaa bbbb ", "cccc dddd"]
        );
        // No character is dropped or moved: the lines' lengths still
        // account for every input character, breaks included.
        let wrapped = wrap("aaaa bbbb cccc dddd", 10);
        let accounted: usize = wrapped.iter().map(|line| line.chars().count()).sum();
        assert_eq!(accounted, "aaaa bbbb cccc dddd".chars().count());
        assert_eq!(wrap("a\n\nb", 10), vec!["a", "", "b"]);
        // Width is clamped to 8, and a single word wider than the line is
        // split hard rather than dropped.
        assert_eq!(wrap("abcdefghij", 4), vec!["abcdefgh", "ij"]);
        assert_eq!(wrap("0123456789abcdefghij", 8), vec!["01234567", "89abcdef", "ghij"]);
        assert_eq!(wrap("", 10), vec![""]);
    }

    #[test]
    fn the_cursor_walks_the_wrapped_lines() {
        // "aaaa bbbb cccc dddd" wraps at width 10 into "aaaa bbbb " /
        // "cccc dddd". The cursor on the first "c" (index 10) sits at
        // the start of the second row.
        assert_eq!(cursor_position("aaaa bbbb cccc dddd", 10, 10), (1, 0));
        // Parked on the first "d" (index 15): still row 1, column 5.
        assert_eq!(cursor_position("aaaa bbbb cccc dddd", 15, 10), (1, 5));
        // Past the end: row 1, column 9.
        assert_eq!(cursor_position("aaaa bbbb cccc dddd", 19, 10), (1, 9));
        // A newline is its own row; the next line starts at column 0.
        assert_eq!(cursor_position("ab\ncd", 3, 10), (1, 0));
        assert_eq!(cursor_position("ab\ncd", 4, 10), (1, 1));
        assert_eq!(cursor_position("ab\ncd", 5, 10), (1, 2));
        // A hard-split word breaks exactly where wrap pushes it (width
        // is clamped to 8, so "abcdefghij" splits as "abcdefgh"/"ij").
        assert_eq!(cursor_position("abcdefghij", 10, 4), (1, 2));
        // An index past the end saturates instead of panicking.
        assert_eq!(cursor_position("ab", 99, 10), (0, 2));
        assert_eq!(cursor_position("", 3, 10), (0, 0));
        // The cursor's row and column always land where the wrapped
        // output can show them: the row exists, and the line on it is at
        // least `col` characters long.
        for text in ["aaaa bbbb cccc dddd", "ab\ncd", "abcdefghij", "a b c d e f g"] {
            for width in [8, 10, 20] {
                let lines = wrap(text, width);
                let chars = text.chars().count();
                for index in 0..=chars {
                    let (row, col) = cursor_position(text, index, width);
                    assert!(row < lines.len(), "{text} @{index} w{width} -> {row},{col}");
                    assert!(
                        col <= lines[row].chars().count(),
                        "{text} @{index} w{width} -> {row},{col} in {:?}",
                        lines
                    );
                }
            }
        }
    }

    #[test]
    fn the_input_box_renders_the_cursor_on_the_right_cell() {
        // "hello there friends" is 19 characters; the input box's inner
        // width in a 20-wide terminal is 18, so it wraps once into
        // "hello there " / "friends". With the cursor on the 't' of
        // "there" (index 6) it belongs at row 0, column 6 — one cell
        // inside the left border, on the box's first text row.
        let mut app = fresh();
        app.paste("hello there friends");
        app.cursor = 6;
        let backend = TestBackend::new(20, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let position = terminal.backend().cursor_position();
        assert_eq!(position, ratatui::layout::Position { x: 7, y: 15 });
        assert!(terminal.backend().cursor_visible());
        // At the end of the same input the cursor sits past "friends",
        // on the second wrapped row.
        app.cursor = 19;
        terminal.draw(|frame| render(frame, &app)).unwrap();
        assert_eq!(
            terminal.backend().cursor_position(),
            ratatui::layout::Position { x: 8, y: 16 }
        );
    }

    #[test]
    fn the_input_box_keeps_the_cursor_in_view() {
        // Four lines of input in a three-row window: the tail shows the
        // last three, but the cursor on the first line pulls the window
        // up rather than hiding the cursor off-screen.
        let mut app = fresh();
        app.paste("aa\nbb\ncc\ndd");
        app.cursor = 1; // inside the first line
        let (lines, cursor) = input_lines(&app.input, app.cursor, 12, 3);
        assert_eq!(lines.len(), 3);
        assert!(lines[0].to_string().contains("aa"), "{lines:?}");
        assert_eq!(cursor, Some((1, 0)));
        // The tail alone: the cursor at the end keeps the last row.
        app.cursor = 11;
        let (lines, cursor) = input_lines(&app.input, app.cursor, 12, 3);
        assert_eq!(cursor, Some((2, 2)));
        assert!(lines[2].to_string().contains("dd"), "{lines:?}");
    }
}
