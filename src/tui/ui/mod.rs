//! Rendering: reads the state machine, writes cells. No behavior lives
//! here — every decision was made in `app.rs` before the frame is drawn.

use ratatui::layout::{Constraint, Flex, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{
    Block, Cell, Clear, LineGauge, Paragraph, Row, Scrollbar, ScrollbarOrientation, ScrollbarState,
    Table, TableState, Tabs, Wrap,
};
use ratatui::Frame;
use unicode_width::UnicodeWidthStr;

use super::app::{App, Entry, Mode, Pane, SettingsRow, ToolStatus};
use crate::boards::{self, BoardData, BoardKind, Section, Slot};

mod board_view;
mod command;
mod knowledge;
mod lanes;
mod run;
mod trust;
mod widgets;

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
    /// Text drawn on an accent-filled cell, as the palette's selection is.
    pub const ON_ACCENT: Color = Color::Black;
}

/// The board pane's width. Slice 3 mounted the pane at 26 columns as a
/// placeholder; the three boards are composed surfaces — cards, dots,
/// bars — and a card with a name and its facts does not fit in 24 inner
/// columns, so the pane takes what it needs and the conversation keeps
/// the rest.
const BOARD_WIDTH: u16 = 42;

/// The pane needs this much room beside a usable conversation. Below it
/// the right column folds into the one-line run strip above the input.
const PANE_MIN_WIDTH: u16 = 80;

/// Draw one frame.
pub fn render(frame: &mut Frame<'_>, app: &App) {
    let pane = frame.area().width >= PANE_MIN_WIDTH;
    let strip = !pane && (app.run.active || app.run.stop_reason.is_some());
    let [main, strip_bar, error_bar, status, input, footer] = Layout::vertical([
        Constraint::Min(3),
        Constraint::Length(if strip { 1 } else { 0 }),
        Constraint::Length(if app.error.is_some() { 1 } else { 0 }),
        Constraint::Length(1),
        Constraint::Length(5),
        Constraint::Length(1),
    ])
    .areas(frame.area());

    if pane {
        let [conversation, right] =
            Layout::horizontal([Constraint::Min(20), Constraint::Length(BOARD_WIDTH)]).areas(main);
        render_conversation(frame, app, conversation);
        render_pane(frame, app, right);
    } else {
        render_conversation(frame, app, main);
        if strip {
            run::render_strip(frame, app, strip_bar);
        }
    }
    if let Some(message) = &app.error {
        render_error(frame, message, error_bar);
    }
    render_status(frame, app, status);
    render_input(frame, app, input);
    render_footer(frame, app, footer);

    match app.mode {
        Mode::Chat => {}
        Mode::Sessions => render_picker(frame, app),
        Mode::Approval => render_approval(frame, app),
        Mode::Lanes => lanes::render(frame, app),
        Mode::Explain => render_explain(frame, app),
        Mode::Settings | Mode::SettingsEdit => render_settings(frame, app),
        Mode::SettingsConfirm => render_settings_confirm(frame, app),
        Mode::Run => run::render_controls(frame, app),
        Mode::Steer => run::render_steer(frame, app),
        Mode::Palette => command::render(frame, app),
        Mode::Checkpoints => trust::render_checkpoints(frame, app),
        Mode::Diff => trust::render_diff(frame, app),
        Mode::Confirm => trust::render_confirm(frame, app),
        Mode::Panel => knowledge::render(frame, app),
    }
}

/// The right pane: a real tab row over the run card or a board. The
/// tabs are what replaced the invisible Ctrl-B cycle; the key still
/// moves between them, but the owner can see where they are.
fn render_pane(frame: &mut Frame<'_>, app: &App, area: Rect) {
    if area.height == 0 {
        return;
    }
    let [tabs_area, content] =
        Layout::vertical([Constraint::Length(1), Constraint::Min(1)]).areas(area);
    let show_run = app.pane == Pane::Run;
    // The board tab follows the board the refresher is pointed at, which
    // is also the board the pane renders. A test or a direct set of
    // `boards.active` therefore stays coherent without a second field
    // to keep in step.
    let selected = if show_run {
        0
    } else {
        1 + match app.boards.active {
            BoardKind::Router => 0,
            BoardKind::Brain => 1,
            BoardKind::Tower => 2,
        }
    };
    let titles = Pane::ALL.iter().map(|pane| pane.label());
    let tabs = Tabs::new(titles)
        .select(selected)
        .style(Style::new().fg(palette::DIM))
        .highlight_style(
            Style::new()
                .fg(palette::ACCENT)
                .add_modifier(Modifier::BOLD),
        )
        .divider(" ");
    frame.render_widget(tabs, tabs_area);
    if show_run {
        run::render(frame, app, content);
    } else {
        render_board(frame, app, content, app.boards.active);
    }
}

/// The settings pane: every addressable knob as a scrolled list of
/// cards — never a plain-text table, the house UX rule — with the
/// selection marked and the editor replacing the list while a value is
/// being typed. The value's source rides each row, because a value the
/// environment owns would shadow a file write and the owner deserves to
/// know that before editing it. The two list-shaped surfaces — the
/// provider overrides and the workload preferences the home's own files
/// carry — render as cards beside the scalars, so the pane reaches
/// every surface the CLI's `fleet settings` addresses.
fn render_settings(frame: &mut Frame<'_>, app: &App) {
    // The modal's width comes from the frame, so every line is composed
    // for the columns it will actually be drawn in — the same share
    // `centered_sized` lays out, read off the frame independently of the
    // height the lines then decide.
    let [columns] = Layout::horizontal([Constraint::Percentage(SETTINGS_MODAL_PERCENT)])
        .flex(Flex::Center)
        .areas(frame.area());
    let width = columns.width.saturating_sub(2).max(1) as usize;
    let editing = app.mode == Mode::SettingsEdit;
    let mut lines = if editing {
        editor_lines(app, width)
    } else {
        settings_lines(app, width)
    };
    let title = if editing {
        format!(
            " settings — editing {} — enter writes, esc abandons ",
            app.editing_label()
        )
    } else {
        " settings — enter edits, r removes, esc close ".to_string()
    };
    let area = centered_sized(
        SETTINGS_MODAL_PERCENT,
        settings_height(lines.len()),
        frame.area(),
    );
    frame.render_widget(Clear, area);
    let block = Block::bordered()
        .title(title)
        .title_style(title_style())
        .border_style(Style::new().fg(palette::ACCENT));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.height == 0 {
        return;
    }
    let visible = inner.height as usize;
    if lines.len() > visible {
        let hidden = lines.len() - visible + 1;
        lines.truncate(visible.saturating_sub(1));
        lines.push(Line::from(Span::styled(
            format!(" +{hidden} more"),
            Style::new().fg(palette::DIM),
        )));
    }
    frame.render_widget(Paragraph::new(Text::from(lines)), inner);
}

/// The settings modal's height in rows: one row per rendered line, the
/// border, and a line of headroom — clamped, so a small frame cannot
/// push the modal off screen and a long pane names what it holds back
/// rather than being cut silently. The two list sections made the pane
/// taller than a fixed share of a 24-row frame, and a card clipped by
/// its own modal is the failure the boards already learned from.
fn settings_height(lines: usize) -> u16 {
    (lines as u16).saturating_add(4).clamp(4, 44)
}

/// The editor's lines: the buffer as one line, with the row's own
/// summary above it so an edit is made against what it changes.
fn editor_lines(app: &App, width: usize) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();
    if let Some((key, buffer)) = &app.editing {
        if let Some(state) = app.settings.iter().find(|state| state.key == key) {
            lines.push(line_of(
                vec![(state.summary.to_string(), Style::new().fg(palette::DIM))],
                width,
            ));
        }
        lines.push(Line::from(Span::styled(
            format!("> {buffer}"),
            Style::new().fg(palette::TEXT),
        )));
    }
    lines
}

/// The pane's list: the scalar knobs exactly as they rendered before
/// the list surfaces arrived, then a Providers section, then a Workloads
/// section — one card per entry the home's own files carry, with the
/// compiled-in fallback named where a file carries none.
fn settings_lines(app: &App, width: usize) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();
    let selected = app.settings_rows().get(app.selected).copied();
    for (index, state) in app.settings.iter().enumerate() {
        let selected = matches!(selected, Some(SettingsRow::Scalar(row)) if row == index);
        let marker = if selected { "▸" } else { " " };
        let value = state.rendered();
        let source = state.source();
        let key_style = if selected {
            Style::new().fg(palette::ACCENT)
        } else {
            Style::new().fg(palette::TEXT)
        };
        lines.push(Line::from(vec![
            Span::styled(format!("{marker} "), key_style),
            Span::styled(state.key.to_string(), key_style),
            Span::styled(
                format!("  {value}"),
                Style::new().fg(if state.from_env {
                    palette::PENDING
                } else {
                    palette::TEXT
                }),
            ),
            Span::styled(format!("  ({source})"), Style::new().fg(palette::DIM)),
        ]));
        if selected {
            // The selection's own summary, indented under it: what the
            // knob does is what the owner needs before editing it.
            lines.push(line_of(
                vec![(
                    format!("  {}", state.summary),
                    Style::new().fg(palette::DIM),
                )],
                width,
            ));
        }
    }
    // --- the provider cards -------------------------------------------
    lines.push(section("Providers"));
    let providers: Vec<&crate::settings::ListState> = app
        .settings_lists
        .iter()
        .filter(|entry| entry.kind == crate::settings::ListKind::Provider)
        .collect();
    if providers.is_empty() {
        lines.push(line_of(
            vec![(
                "   no provider overrides — the compiled-in roster serves".to_string(),
                Style::new().fg(palette::DIM),
            )],
            width,
        ));
    }
    for (index, entry) in providers.iter().enumerate() {
        let selected = matches!(selected, Some(SettingsRow::Provider(row)) if row == index);
        lines.extend(list_card(entry, selected, width));
    }
    // --- the workload cards -------------------------------------------
    lines.push(section("Workloads"));
    let workloads: Vec<&crate::settings::ListState> = app
        .settings_lists
        .iter()
        .filter(|entry| entry.kind == crate::settings::ListKind::Workload)
        .collect();
    if workloads.is_empty() {
        lines.push(line_of(
            vec![(
                "   no workload preferences — the compiled-in table serves".to_string(),
                Style::new().fg(palette::DIM),
            )],
            width,
        ));
    }
    for (index, entry) in workloads.iter().enumerate() {
        let selected = matches!(selected, Some(SettingsRow::Workload(row)) if row == index);
        lines.extend(list_card(entry, selected, width));
    }
    // The last write's outcome, named: a refusal the loader would have
    // made is shown here rather than silently changing nothing.
    if let Some(note) = &app.settings_note {
        lines.push(Line::from(Span::styled(
            note.to_string(),
            Style::new().fg(palette::PENDING),
        )));
    }
    lines.push(line_of(
        vec![(
            format!(
                "{} knobs · {} provider overrides · {} workload preferences",
                app.settings.len(),
                providers.len(),
                workloads.len()
            ),
            Style::new().fg(palette::DIM),
        )],
        width,
    ));
    lines.push(line_of(
        vec![(
            "enter edits · r removes a list entry · esc close".to_string(),
            Style::new().fg(palette::DIM),
        )],
        width,
    ));
    lines
}

/// One list entry as a card: its key as the head, the class its removal
/// belongs to beside it, and the entry's own detail indented underneath.
/// A card, never a table row — the removal's class rides the head
/// because the owner needs to know which kind of `r` they are about to
/// press, and the file-added class is the destructive one.
fn list_card(
    entry: &crate::settings::ListState,
    selected: bool,
    width: usize,
) -> Vec<Line<'static>> {
    let marker = if selected { "▸" } else { " " };
    let key_style = if selected {
        Style::new()
            .fg(palette::ACCENT)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::new().fg(palette::TEXT)
    };
    let (class, class_style) = match (entry.kind, entry.reverts) {
        (crate::settings::ListKind::Provider, true) => ("override", palette::DIM),
        (crate::settings::ListKind::Provider, false) => ("added by the file", palette::PENDING),
        (crate::settings::ListKind::Workload, true) => ("preference", palette::DIM),
        (crate::settings::ListKind::Workload, false) => ("added by the file", palette::PENDING),
    };
    let mut lines = vec![line_of(
        vec![
            (format!("{marker} "), Style::new().fg(palette::ACCENT)),
            (entry.key.clone(), key_style),
            (format!("  {class}"), Style::new().fg(class_style)),
        ],
        width,
    )];
    lines.extend(wrapped(
        "   ",
        &entry.detail,
        Style::new().fg(palette::DIM),
        width,
    ));
    if selected {
        // The affordance, named where the selection is: an entry is
        // edited with enter and removed with `r`, and a knob has no
        // removal at all.
        lines.push(line_of(
            vec![(
                "   enter edits this entry · r removes it".to_string(),
                Style::new().fg(palette::DIM),
            )],
            width,
        ));
    }
    lines
}

/// The destructive confirmation over the pane that armed it: exactly
/// what will be removed, named, and the two answers. The pane asks
/// because the CLI's `force` flag is a shell word and a pane has nowhere
/// to put one — the question is what stands in for it, and the writer's
/// own compiled-in rule still answers when the removal lands.
fn render_settings_confirm(frame: &mut Frame<'_>, app: &App) {
    let Some(removal) = &app.settings_removal else {
        return;
    };
    // The modal's width comes from the frame first, so every line is
    // composed for the columns it will actually be drawn in — the
    // same share `centered_sized` lays out, read off the frame
    // independently of the height the lines then decide. The height
    // comes from the lines, because a card clipped by its own modal
    // is the failure the boards already learned from — and this
    // modal's answers line is the one text that must never be the
    // clipped one: it is the only place that says how to answer a
    // destructive question.
    let [columns] = Layout::horizontal([Constraint::Percentage(SETTINGS_CONFIRM_PERCENT)])
        .flex(Flex::Center)
        .areas(frame.area());
    let width = columns.width.saturating_sub(2).max(1) as usize;
    let mut lines = vec![Line::from("")];
    lines.extend(wrapped(
        " ",
        &removal.detail,
        Style::new().fg(palette::TEXT),
        width,
    ));
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled(
            " [y]",
            Style::new()
                .fg(palette::DENIED)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" remove   ", Style::new().fg(palette::TEXT)),
        Span::styled("any other key", Style::new().fg(palette::DIM)),
        Span::styled(" abandons, writing nothing", Style::new().fg(palette::TEXT)),
    ]));
    let area = centered_sized(
        SETTINGS_CONFIRM_PERCENT,
        settings_height(lines.len()),
        frame.area(),
    );
    frame.render_widget(Clear, area);
    let block = Block::bordered()
        .title(" remove — y confirms, any other key abandons ")
        .title_style(title_style())
        .border_style(Style::new().fg(palette::PENDING));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.height == 0 {
        return;
    }
    frame.render_widget(Paragraph::new(lines), inner);
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

    // One column for the scrollbar, reserved whenever there is room: a
    // bar that appears and disappears reflows the whole conversation.
    let bar_area = (inner.width > 12).then(|| Rect {
        x: inner.x + inner.width - 1,
        y: inner.y,
        width: 1,
        height: inner.height,
    });
    let text_width = inner
        .width
        .saturating_sub(if bar_area.is_some() { 1 } else { 0 });
    let entries = app.conversation_lines();
    let mut lines: Vec<Line<'_>> = Vec::new();
    for entry in &entries {
        lines.extend(entry_lines(entry, text_width as usize));
    }
    lines.extend(run::plan_lines(app, text_width as usize));
    // A fresh session is not an empty void: the pane says what this is and
    // what to do, so the first frame reads as intentional rather than
    // broken. The session-start skills card is the one entry that is not
    // a conversation, so the welcome still leads when it is the only one.
    let only_skills = entries.len() == 1 && matches!(&entries[0], Entry::Skills(_));
    if lines.is_empty() {
        frame.render_widget(Paragraph::new(Text::from(welcome_lines(app))), inner);
        return;
    }
    if only_skills {
        let mut with_welcome = welcome_lines(app);
        with_welcome.push(Line::from(""));
        with_welcome.extend(lines);
        lines = with_welcome;
    }
    let total = lines.len();
    let visible = inner.height as usize;
    let furthest = total.saturating_sub(visible);
    let offset = if app.following {
        furthest
    } else {
        furthest.saturating_sub(app.scrolled_back)
    };
    let paragraph =
        Paragraph::new(Text::from(lines)).scroll((offset.min(u16::MAX as usize) as u16, 0));
    let text_area = bar_area
        .map(|_| Rect {
            width: inner.width.saturating_sub(1),
            ..inner
        })
        .unwrap_or(inner);
    frame.render_widget(paragraph, text_area);
    if let Some(bar) = bar_area {
        // A bar with nothing to say is not drawn; `content_length == 0`
        // renders nothing anyway, but the branch makes that explicit.
        if total > visible {
            let mut state = ScrollbarState::new(total)
                .position(offset)
                .viewport_content_length(visible);
            frame.render_stateful_widget(
                Scrollbar::new(ScrollbarOrientation::VerticalRight)
                    .style(Style::new().fg(palette::DIM)),
                bar,
                &mut state,
            );
        }
    }
}

/// The opening card: what this is, what to do, and what the machine
/// already holds. Used both when the conversation is empty and when the
/// only thing on screen is the library's session-start report.
fn welcome_lines(app: &App) -> Vec<Line<'static>> {
    let mut lines = vec![
        Line::from(Span::styled(
            "fleet",
            Style::new()
                .fg(palette::ACCENT)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "a native agent on this machine.",
            Style::new().fg(palette::TEXT),
        )),
        Line::from(Span::styled(
            "type a task below — tools ask before they run.",
            Style::new().fg(palette::DIM),
        )),
    ];
    if let Some(said) = welcome_facts(app) {
        lines.push(Line::from(Span::styled(
            said,
            Style::new().fg(palette::DIM),
        )));
    }
    lines
}

/// The one welcome line the opening board reads can already answer. It
/// is absent while the opening reads are in flight, not a zero that
/// would read as "this machine holds nothing".
fn welcome_facts(app: &App) -> Option<String> {
    let mut parts: Vec<String> = Vec::new();
    if let Some(BoardData::Router(snapshot)) = &app.boards.slot(BoardKind::Router).snapshot {
        if let Some(totals) = &snapshot.totals {
            parts.push(format!("{} route calls today", totals.calls));
        }
    }
    if let Some(BoardData::Tower(snapshot)) = &app.boards.slot(BoardKind::Tower).snapshot {
        parts.push(format!("{} projects", snapshot.projects.len()));
    }
    if parts.is_empty() {
        None
    } else {
        Some(format!(" {}", parts.join(" \u{b7} ")))
    }
}

/// The board pane: one of the three read-only surfaces, whichever is
/// active. The pane's title names it, the status bar repeats the name, and
/// a refused read keeps the last snapshot on screen with the reason on one
/// named line beside the time of the last good read. A healthy board names
/// its own read time on that same line, so the age of the data on screen
/// is never a thing the operator has to guess at.
fn render_board(frame: &mut Frame<'_>, app: &App, area: Rect, kind: BoardKind) {
    let slot = app.boards.slot(kind);
    let block = Block::bordered()
        .title(format!(" board \u{2014} {} ", kind.label()))
        .title_style(title_style())
        .border_style(Style::new().fg(palette::ACCENT));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.height == 0 || inner.width == 0 {
        return;
    }

    let width = inner.width as usize;
    // The header rides the top of the pane. A refused read names its
    // reason there, wrapped because a narrow pane clipping a named error
    // is the same failure as not naming it, with the minute of the last
    // good read beneath it. A healthy board names its own read time on
    // that same line: without it the operator cannot tell a second-old
    // snapshot from a twelve-minute one, which is the whole difference
    // between a live board and a stale one — and a scan the tower's
    // budget cut short is exactly the case where the age of the data is
    // the fact at issue.
    let header: Vec<Line<'static>> = match &slot.error {
        Some(reason) => {
            let budget = width.saturating_mul(3).saturating_sub(3);
            let reason = boards::trim_to(reason, budget);
            let mut lines: Vec<Line<'static>> = wrap(&format!(" ! {reason}"), width)
                .into_iter()
                .map(|chunk| Line::from(Span::styled(chunk, Style::new().fg(palette::DENIED))))
                .collect();
            let stamp = match &slot.snapshot {
                Some(snapshot) => format!("   last good read {}", snapshot.read_at()),
                None => "   no snapshot yet".to_string(),
            };
            lines.push(Line::from(Span::styled(
                stamp,
                Style::new().fg(palette::DIM),
            )));
            lines
        }
        None => match &slot.snapshot {
            Some(snapshot) => vec![Line::from(Span::styled(
                format!("   read {}", snapshot.read_at()),
                Style::new().fg(palette::DIM),
            ))],
            // A board still on its opening read says it is reading; a
            // board whose read was refused says so and names the reason.
            // Neither has a read time to name yet.
            None => Vec::new(),
        },
    };
    let content = if header.is_empty() {
        inner
    } else {
        let [top, rest] =
            Layout::vertical([Constraint::Length(header.len() as u16), Constraint::Min(1)])
                .areas(inner);
        frame.render_widget(Paragraph::new(Text::from(header)), top);
        rest
    };
    if content.height == 0 || content.width == 0 {
        return;
    }
    match &slot.snapshot {
        Some(BoardData::Router(snapshot)) => board_view::render_router(frame, snapshot, content),
        Some(BoardData::Tower(snapshot)) => board_view::render_tower(frame, app, snapshot, content),
        Some(BoardData::Brain(snapshot)) => board_view::render_lines(
            frame,
            brain_lines(snapshot, content.width as usize),
            content,
        ),
        None => board_view::render_lines(
            frame,
            board_lines(kind, slot, content.width as usize),
            content,
        ),
    }
}

/// The active board's content: the snapshot that landed, the read in
/// flight, or the empty state naming why nothing has.
fn board_lines(kind: BoardKind, slot: &Slot, width: usize) -> Vec<Line<'static>> {
    let Some(snapshot) = &slot.snapshot else {
        // A board still on its opening read says it is reading; a board
        // whose read was refused says so and names the reason. Neither
        // ever shows an empty pane.
        let (state, reason) = if slot.loading {
            (" reading\u{2026}".to_string(), None)
        } else {
            (
                " no snapshot yet".to_string(),
                Some(
                    slot.error
                        .clone()
                        .unwrap_or_else(|| "the read has not landed yet".to_string()),
                ),
            )
        };
        let mut lines = vec![
            Line::from(Span::styled(
                format!(" {} board", kind.label()),
                Style::new()
                    .fg(palette::ACCENT)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                state,
                if reason.is_some() {
                    Style::new().fg(palette::DENIED)
                } else {
                    Style::new().fg(palette::DIM)
                },
            )),
        ];
        if let Some(reason) = reason {
            lines.push(line_of(
                vec![(format!(" {reason}"), Style::new().fg(palette::DIM))],
                width,
            ));
        }
        return lines;
    };
    match snapshot {
        BoardData::Router(snapshot) => router_lines(snapshot, width),
        BoardData::Brain(snapshot) => brain_lines(snapshot, width),
        BoardData::Tower(snapshot) => tower_lines(snapshot, width),
    }
}

/// The router board: Fleet's own routing state — the workload tier
/// chains with their candidates, the gates, what Fleet just served, and
/// the day's counts. Nothing here is dialed: the router is this crate's
/// own, so the board reads what the router reads.
pub(super) fn router_lines(snapshot: &boards::RouterSnapshot, width: usize) -> Vec<Line<'static>> {
    let judge = snapshot
        .judge
        .as_ref()
        .map(|(mode, _)| mode.clone())
        .unwrap_or_else(|| "\u{2014}".to_string());
    let mut lines = vec![line_of(
        vec![
            (" fleet ".to_string(), Style::new().fg(palette::DIM)),
            ("router own".to_string(), Style::new().fg(palette::TEXT)),
            (format!("  judge {judge}"), Style::new().fg(palette::DIM)),
        ],
        width,
    )];
    // The gates and the pin, the two facts that shape every route: what
    // confidence a judgment needs to be trusted, what an unmapped ask
    // defaults to, and what the owner pinned if anything.
    if let Some((_, Some(default_workload))) = &snapshot.judge {
        let pin = snapshot.pin.as_deref().unwrap_or("\u{2014}");
        lines.push(line_of(
            vec![
                (" gates ".to_string(), Style::new().fg(palette::DIM)),
                (
                    format!("default {default_workload}"),
                    Style::new().fg(palette::TEXT),
                ),
                (format!("  pin {pin}"), Style::new().fg(palette::DIM)),
            ],
            width,
        ));
    }
    lines.push(Line::from(""));
    lines.push(section("tiers"));
    for chain in &snapshot.tiers {
        let extra = chain.candidates.len().saturating_sub(1);
        let tail = if extra > 0 {
            format!("+{extra}")
        } else {
            String::new()
        };
        // The resolved primary is the fact worth the whole line; the
        // fallback count shows when it fits beside it, and `line_of`
        // drops it when it does not.
        let used = 3 + chain.tier.chars().count() + 1;
        let room = width.saturating_sub(used + tail.chars().count() + 1);
        let room = if chain.primary.chars().count() <= room {
            room
        } else {
            width.saturating_sub(used)
        };
        lines.push(line_of(
            vec![
                (" \u{25cf} ".to_string(), Style::new().fg(palette::DONE)),
                (chain.tier.clone(), Style::new().fg(palette::TEXT)),
                (
                    format!(" {}", boards::trim_to(&chain.primary, room)),
                    Style::new().fg(palette::ACCENT),
                ),
                (format!(" {tail}"), Style::new().fg(palette::DIM)),
            ],
            width,
        ));
    }

    lines.push(Line::from(""));
    lines.push(section("ledger"));
    if let Some(totals) = &snapshot.totals {
        let rate = totals
            .success_rate()
            .map(|rate| format!("{rate}%"))
            .unwrap_or_else(|| "\u{2014}".to_string());
        let errors = if totals.errors > 0 {
            palette::DENIED
        } else {
            palette::TEXT
        };
        lines.push(line_of(
            vec![
                (" calls ".to_string(), Style::new().fg(palette::DIM)),
                (
                    boards::grouped(totals.calls),
                    Style::new().fg(palette::TEXT),
                ),
                ("  ok ".to_string(), Style::new().fg(palette::DIM)),
                (rate, Style::new().fg(palette::DONE)),
                ("  err ".to_string(), Style::new().fg(palette::DIM)),
                (boards::grouped(totals.errors), Style::new().fg(errors)),
            ],
            width,
        ));
        // The day's route rows carry no token counts, so the board says so
        // rather than estimating one from a character count.
        lines.push(line_of(
            vec![
                (" tokens ".to_string(), Style::new().fg(palette::DIM)),
                (
                    "not recorded on the route row".to_string(),
                    Style::new().fg(palette::DIM),
                ),
            ],
            width,
        ));
    }
    if let Some(judge) = &snapshot.judge_counters {
        lines.push(line_of(
            vec![
                (" judge ".to_string(), Style::new().fg(palette::DIM)),
                (
                    format!(
                        "{} fresh \u{b7} {} cached",
                        boards::grouped(judge.fresh),
                        boards::compact(judge.cached)
                    ),
                    Style::new().fg(palette::TEXT),
                ),
                (
                    format!("  {} err", boards::grouped(judge.errors)),
                    Style::new().fg(if judge.errors > 0 {
                        palette::PENDING
                    } else {
                        palette::DIM
                    }),
                ),
            ],
            width,
        ));
    }
    lines.push(Line::from(""));
    lines.push(section("served"));
    for row in snapshot.recent.iter().take(6) {
        let (glyph, color) = if row.ok() {
            ("\u{25cf}", palette::DONE)
        } else {
            ("\u{2715}", palette::DENIED)
        };
        // The row names its own outcome, and the model it names is the
        // served candidate's own: the provider prefix is what the tier
        // lines above already say, so it is the part a narrow pane drops
        // first. The workload rides the tail for the same reason — every
        // workload is named in the tiers block.
        let model = row
            .model
            .rsplit('/')
            .next()
            .unwrap_or(&row.model)
            .to_string();
        let workload = row.workload.clone().unwrap_or_default();
        let ms = row
            .ms
            .map(|ms| format!("{}ms", boards::grouped(ms)))
            .unwrap_or_default();
        let room =
            width.saturating_sub(3 + row.outcome.chars().count() + 1 + ms.chars().count() + 1);
        lines.push(line_of(
            vec![
                (format!(" {glyph} "), Style::new().fg(color)),
                (row.outcome.clone(), Style::new().fg(color)),
                (
                    format!(" {}", boards::trim_to(&model, room)),
                    Style::new().fg(palette::TEXT),
                ),
                (format!(" {ms}"), Style::new().fg(palette::DIM)),
                (format!(" {workload}"), Style::new().fg(palette::DIM)),
            ],
            width,
        ));
    }
    if snapshot.recent.is_empty() {
        lines.push(line_of(
            vec![(
                " no route rows today".to_string(),
                Style::new().fg(palette::DIM),
            )],
            width,
        ));
    }

    // The workload mix: where the day's routes actually went.
    if !snapshot.workloads.is_empty() {
        let mix: Vec<String> = snapshot
            .workloads
            .iter()
            .take(3)
            .map(|(workload, calls)| format!("{workload} {}", boards::compact(*calls)))
            .collect();
        lines.push(line_of(
            vec![
                (" mix ".to_string(), Style::new().fg(palette::DIM)),
                (mix.join(" \u{b7} "), Style::new().fg(palette::TEXT)),
            ],
            width,
        ));
    }
    // Fleet's own routing guard: whether a pay-as-you-go provider may be
    // routed to at all, and which ones are payg.
    if let Some((allowed, providers)) = &snapshot.payg {
        let (label, color) = if *allowed {
            ("payg on".to_string(), palette::PENDING)
        } else {
            ("payg off".to_string(), palette::DONE)
        };
        let mut pieces = vec![
            (" guard ".to_string(), Style::new().fg(palette::DIM)),
            (label, Style::new().fg(color)),
        ];
        if !providers.is_empty() {
            pieces.push((
                format!(" \u{b7} {}", providers.join(", ")),
                Style::new().fg(palette::DIM),
            ));
        }
        lines.push(line_of(pieces, width));
    }
    // The model mix is the board's `BarChart`, above these lines; the
    // guidance sentence is the only part that stays a line.
    for (tier, sentence) in snapshot.guidance.iter().take(1) {
        lines.push(line_of(
            vec![
                (format!(" \u{2514} {tier} "), Style::new().fg(palette::DIM)),
                (sentence.clone(), Style::new().fg(palette::DIM)),
            ],
            width,
        ));
    }
    for reason in &snapshot.refused {
        lines.extend(refused_lines("read", reason, width));
    }
    lines
}

/// The second-brain board: the derived state as cards — the
/// earned-autonomy gate board, the HITL inbox, the quality loops and
/// their graded rows, the maturity sparkline, and the doing/conflict
/// counts.
fn brain_lines(snapshot: &boards::BrainSnapshot, width: usize) -> Vec<Line<'static>> {
    let mut lines = vec![
        line_of(
            vec![
                (" second-brain ".to_string(), Style::new().fg(palette::DIM)),
                (
                    snapshot.home.display().to_string(),
                    Style::new().fg(palette::TEXT),
                ),
            ],
            width,
        ),
        Line::from(""),
    ];
    match &snapshot.gates {
        Section::Landed(gates) => {
            lines.push(section("gates"));
            lines.push(line_of(
                vec![
                    (" verdicts ".to_string(), Style::new().fg(palette::DIM)),
                    (
                        boards::grouped(gates.totals.verdicts),
                        Style::new().fg(palette::TEXT),
                    ),
                    ("  dispositions ".to_string(), Style::new().fg(palette::DIM)),
                    (
                        boards::grouped(gates.totals.dispositions),
                        Style::new().fg(palette::TEXT),
                    ),
                ],
                width,
            ));
            for gate in gates.board.iter().take(5) {
                let fit = if gate.fit_ready {
                    " fit".to_string()
                } else {
                    String::new()
                };
                let graded = Style::new().fg(if gate.fit_ready {
                    palette::DONE
                } else {
                    palette::DIM
                });
                lines.push(line_of(
                    vec![
                        (" \u{25cf} ".to_string(), Style::new().fg(palette::DONE)),
                        (boards::cell(&gate.gate, 12), Style::new().fg(palette::TEXT)),
                        (
                            format!(" {}v", boards::grouped(gate.verdicts)),
                            Style::new().fg(palette::DIM),
                        ),
                        (format!("  {}", gate.graded_progress), graded),
                        (fit, Style::new().fg(palette::DONE)),
                    ],
                    width,
                ));
            }
            if !gates.inbox.is_empty() {
                lines.push(line_of(
                    vec![
                        (" hitl ".to_string(), Style::new().fg(palette::DIM)),
                        (
                            format!("{} open", gates.inbox.len()),
                            Style::new().fg(palette::PENDING),
                        ),
                    ],
                    width,
                ));
                for row in gates.inbox.iter().take(2) {
                    lines.push(line_of(
                        vec![
                            (" \u{2715} ".to_string(), Style::new().fg(palette::DENIED)),
                            (boards::cell(&row.op, 14), Style::new().fg(palette::TEXT)),
                            (
                                format!(" {}", row.verdict),
                                Style::new().fg(palette::DENIED),
                            ),
                        ],
                        width,
                    ));
                }
            }
        }
        Section::Refused(reason) => lines.extend(refused_lines("gates", reason, width)),
    }

    lines.push(Line::from(""));
    match &snapshot.harness {
        Section::Landed(harness) => {
            lines.push(section("harness"));
            let converged = harness.loops.iter().map(|row| row.converged).sum::<u64>();
            let runs = harness.loops.iter().map(|row| row.runs).sum::<u64>();
            lines.push(line_of(
                vec![
                    (" loops ".to_string(), Style::new().fg(palette::DIM)),
                    (boards::grouped(runs), Style::new().fg(palette::TEXT)),
                    ("  converged ".to_string(), Style::new().fg(palette::DIM)),
                    (boards::grouped(converged), Style::new().fg(palette::DONE)),
                ],
                width,
            ));
            for row in harness.recent.iter().take(2) {
                let color = if row.outcome == "ok" {
                    palette::DONE
                } else {
                    palette::DENIED
                };
                lines.push(line_of(
                    vec![
                        (" \u{25cf} ".to_string(), Style::new().fg(color)),
                        (boards::cell(&row.kind, 10), Style::new().fg(palette::TEXT)),
                        (
                            format!(
                                " {} \u{b7} {}",
                                boards::trim_to(&row.label, width.saturating_sub(18)),
                                row.outcome
                            ),
                            Style::new().fg(palette::DIM),
                        ),
                    ],
                    width,
                ));
            }
        }
        Section::Refused(reason) => lines.extend(refused_lines("harness", reason, width)),
    }

    lines.push(Line::from(""));
    match &snapshot.loops {
        Section::Landed(loops) => {
            lines.push(section("loops"));
            let converged = loops
                .loops
                .iter()
                .filter(|row| row.verdict == "passed")
                .count();
            lines.push(line_of(
                vec![
                    (" loops ".to_string(), Style::new().fg(palette::DIM)),
                    (
                        boards::grouped(loops.loops.len() as u64),
                        Style::new().fg(palette::TEXT),
                    ),
                    ("  converged ".to_string(), Style::new().fg(palette::DIM)),
                    (
                        boards::grouped(converged as u64),
                        Style::new().fg(palette::DONE),
                    ),
                    (
                        format!(
                            "  pending {}",
                            loops
                                .loops
                                .iter()
                                .map(|row| row.pending_review)
                                .sum::<u64>()
                        ),
                        Style::new().fg(palette::PENDING),
                    ),
                ],
                width,
            ));
            for row in loops.loops.iter().take(3) {
                let color = if row.verdict == "passed" {
                    palette::DONE
                } else {
                    palette::PENDING
                };
                lines.push(line_of(
                    vec![
                        (" \u{25cf} ".to_string(), Style::new().fg(color)),
                        (boards::cell(&row.name, 14), Style::new().fg(palette::TEXT)),
                        (
                            format!(" {} \u{b7} {} iter", row.verdict, row.iterations),
                            Style::new().fg(palette::DIM),
                        ),
                    ],
                    width,
                ));
            }
            if let Some((name, graded)) = loops.graded.first() {
                lines.push(line_of(
                    vec![
                        (" graded ".to_string(), Style::new().fg(palette::DIM)),
                        (boards::cell(name, 14), Style::new().fg(palette::TEXT)),
                        (
                            format!(
                                " {} ({} confirmed)",
                                boards::grouped(graded.n),
                                boards::grouped(graded.confirmed)
                            ),
                            Style::new().fg(palette::DIM),
                        ),
                    ],
                    width,
                ));
            }
        }
        Section::Refused(reason) => lines.extend(refused_lines("loops", reason, width)),
    }

    lines.push(Line::from(""));
    match &snapshot.maturity {
        Section::Landed(maturity) => {
            lines.push(section("maturity"));
            // The output series is the fleet's own volume; the state's
            // first series stands in when it names none.
            let series = maturity
                .series
                .iter()
                .find(|(name, _)| name == "output")
                .or_else(|| maturity.series.first());
            if let Some((name, values)) = series {
                lines.push(line_of(
                    vec![
                        (" ".to_string(), Style::new()),
                        (boards::cell(name, 12), Style::new().fg(palette::TEXT)),
                        (" ".to_string(), Style::new()),
                        (
                            boards::sparkline(values, 12),
                            Style::new().fg(palette::ACCENT),
                        ),
                        (
                            format!(" {}", values.last().copied().unwrap_or(0)),
                            Style::new().fg(palette::DIM),
                        ),
                    ],
                    width,
                ));
            }
            if let Some((name, at)) = maturity.milestones.last() {
                lines.push(line_of(
                    vec![
                        (" \u{2514} ".to_string(), Style::new().fg(palette::DIM)),
                        (format!("{name} {at}"), Style::new().fg(palette::DIM)),
                    ],
                    width,
                ));
            }
        }
        Section::Refused(reason) => lines.extend(refused_lines("maturity", reason, width)),
    }

    lines.push(Line::from(""));
    match &snapshot.reports {
        Section::Landed(reports) => {
            lines.push(section("reports"));
            let conflicts = if reports.open_conflicts > 0 {
                palette::PENDING
            } else {
                palette::DONE
            };
            lines.push(line_of(
                vec![
                    (" doing ".to_string(), Style::new().fg(palette::DIM)),
                    (
                        boards::grouped(reports.doing.len() as u64),
                        Style::new().fg(palette::TEXT),
                    ),
                    ("  conflicts ".to_string(), Style::new().fg(palette::DIM)),
                    (
                        boards::grouped(reports.open_conflicts),
                        Style::new().fg(conflicts),
                    ),
                ],
                width,
            ));
            for row in reports.doing.iter().take(2) {
                lines.push(line_of(
                    vec![
                        (" \u{25cf} ".to_string(), Style::new().fg(palette::PENDING)),
                        (
                            boards::cell(&row.project, 12),
                            Style::new().fg(palette::TEXT),
                        ),
                        (
                            format!(" {}", boards::trim_to(&row.title, width.saturating_sub(18))),
                            Style::new().fg(palette::DIM),
                        ),
                    ],
                    width,
                ));
            }
        }
        Section::Refused(reason) => lines.extend(refused_lines("reports", reason, width)),
    }

    match &snapshot.tower {
        Section::Landed(tower) => {
            let dirty: u64 = tower.projects.iter().map(|project| project.dirty).sum();
            lines.push(line_of(
                vec![
                    (" tower ".to_string(), Style::new().fg(palette::DIM)),
                    (
                        format!("{} projects \u{b7} {} dirty", tower.projects.len(), dirty),
                        Style::new().fg(palette::TEXT),
                    ),
                ],
                width,
            ));
        }
        Section::Refused(reason) => lines.extend(refused_lines("tower data", reason, width)),
    }
    lines
}

/// The tower: per-project cards — branch, dirty dot, head commit, the
/// trailing-thirty-day count, ahead/behind — dirtiest first.
pub(super) fn tower_lines(snapshot: &boards::TowerSnapshot, width: usize) -> Vec<Line<'static>> {
    let mut lines = vec![
        line_of(
            vec![
                (" root ".to_string(), Style::new().fg(palette::DIM)),
                (
                    snapshot.root.display().to_string(),
                    Style::new().fg(palette::TEXT),
                ),
            ],
            width,
        ),
        line_of(
            vec![
                (" repos ".to_string(), Style::new().fg(palette::DIM)),
                (
                    boards::grouped(snapshot.projects.len() as u64),
                    Style::new().fg(palette::TEXT),
                ),
                ("  skipped ".to_string(), Style::new().fg(palette::DIM)),
                (
                    boards::grouped(snapshot.skipped.len() as u64),
                    Style::new().fg(palette::DIM),
                ),
                (
                    format!(
                        "  30d {}",
                        snapshot
                            .projects
                            .iter()
                            .map(|project| project.commits_30d)
                            .sum::<u64>()
                    ),
                    Style::new().fg(palette::DIM),
                ),
            ],
            width,
        ),
        Line::from(""),
    ];
    for project in &snapshot.projects {
        let (dot, color) = if project.dirty > 0 {
            ("\u{25cf}", palette::PENDING)
        } else {
            ("\u{25cb}", palette::DONE)
        };
        let branch = project
            .branch
            .clone()
            .unwrap_or_else(|| "\u{2014}".to_string());
        let mut pieces = vec![
            (format!(" {dot} "), Style::new().fg(color)),
            (
                boards::cell(&project.name, 16),
                Style::new().fg(palette::TEXT),
            ),
            (
                format!(" {}", boards::trim_to(&branch, 10)),
                Style::new().fg(palette::DIM),
            ),
        ];
        if project.dirty > 0 {
            pieces.push((
                format!(" {}d", project.dirty),
                Style::new().fg(palette::PENDING),
            ));
        }
        if let Some(ahead) = project.ahead {
            pieces.push((format!(" \u{2191}{ahead}"), Style::new().fg(palette::DIM)));
        }
        if let Some(behind) = project.behind {
            pieces.push((format!(" \u{2193}{behind}"), Style::new().fg(palette::DIM)));
        }
        lines.push(line_of(pieces, width));
        // The head commit underneath, in the card family's two-line shape.
        let head = match (&project.head, &project.subject) {
            (Some(sha), Some(subject)) => format!("{sha} {subject}"),
            (Some(sha), None) => sha.clone(),
            _ => project
                .notes
                .first()
                .cloned()
                .unwrap_or_else(|| "no commits yet".to_string()),
        };
        lines.push(line_of(
            vec![
                ("   \u{2514} ".to_string(), Style::new().fg(palette::DIM)),
                (head, Style::new().fg(palette::DIM)),
            ],
            width,
        ));
    }
    if snapshot.beyond_bound > 0 {
        lines.push(line_of(
            vec![(
                format!(" +{} beyond the scan's bound", snapshot.beyond_bound),
                Style::new().fg(palette::DIM),
            )],
            width,
        ));
    }
    // A scan the aggregate budget cut short names what it holds back, so
    // a truncated scan never reads as a machine with fewer projects than
    // it has.
    if snapshot.unread > 0 {
        let read = snapshot.projects.len() + snapshot.skipped.len();
        lines.extend(truncated_lines(read, snapshot.unread, width));
    }
    lines
}

/// A scan the aggregate budget cut short, named on two lines the way a
/// refused read is: how many directories were read, and why the rest
/// were not. Two lines rather than one because a composed sentence does
/// not fit the 40 columns an 80-column terminal gives the pane, and a
/// clipped name is not a named truncation.
fn truncated_lines(read: usize, unread: usize, width: usize) -> Vec<Line<'static>> {
    vec![
        line_of(
            vec![(
                format!(" {read} of {} projects", read + unread),
                Style::new().fg(palette::PENDING),
            )],
            width,
        ),
        line_of(
            vec![(
                "   the scan budget was spent".to_string(),
                Style::new().fg(palette::DIM),
            )],
            width,
        ),
    ]
}

/// A board section header.
fn section(title: &str) -> Line<'static> {
    Line::from(Span::styled(
        format!(" {title}"),
        Style::new()
            .fg(palette::ACCENT)
            .add_modifier(Modifier::BOLD),
    ))
}

/// One refused read inside a snapshot, named on the board that carries it:
/// the read's name on its own line and its reason beneath it, because a
/// state file's path is long and a clipped path is not a named refusal.
fn refused_lines(name: &str, reason: &str, width: usize) -> Vec<Line<'static>> {
    vec![
        line_of(
            vec![(
                format!(" \u{2715} {name}"),
                Style::new().fg(palette::DENIED),
            )],
            width,
        ),
        line_of(
            vec![(format!("   {reason}"), Style::new().fg(palette::DIM))],
            width,
        ),
    ]
}

/// Compose one line from styled pieces, dropping the tail when the pieces
/// do not fit: a board line never wraps and never overflows its pane, so a
/// long name or a long sentence costs its own tail rather than the
/// layout's.
fn line_of(pieces: Vec<(String, Style)>, width: usize) -> Line<'static> {
    let mut used = 0usize;
    let mut spans = Vec::with_capacity(pieces.len());
    for (text, style) in pieces {
        let room = width.saturating_sub(used);
        if room == 0 {
            break;
        }
        let text = boards::trim_to(&text, room);
        used += text.chars().count();
        spans.push(Span::styled(text, style));
    }
    Line::from(spans)
}

/// The one-line failure banner.
fn render_error(frame: &mut Frame<'_>, message: &str, area: Rect) {
    let line = Line::from(vec![
        Span::styled(
            " ! ",
            Style::new()
                .fg(palette::DENIED)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(message, Style::new().fg(palette::DENIED)),
    ]);
    frame.render_widget(Paragraph::new(line), area);
}

/// The status bar: model, session, turn, tokens, and the run's state.
fn render_status(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let status = &app.status;
    // The context gauge gets its own cells on the right where the bar is
    // wide enough; on a narrow terminal the status line stands alone.
    let (line_area, gauge_area) = if area.width >= 80 {
        let [left, right] =
            Layout::horizontal([Constraint::Min(40), Constraint::Length(18)]).areas(area);
        (left, Some(right))
    } else {
        (area, None)
    };
    let (dot, state) = if status.working {
        (palette::WORKING, "working")
    } else {
        (palette::IDLE, "idle")
    };
    let turn = if status.turn == 0 {
        "—".to_string()
    } else {
        status.turn.to_string()
    };
    let mut spans = vec![
        Span::styled(" ● ", Style::new().fg(dot)),
        Span::styled(state, Style::new().fg(dot)),
        Span::styled("  ", Style::new()),
    ];
    // The bar names the routing mode, never the compiled-in default
    // model: the default is the DeepSeek reserve, and naming it as the
    // model in use is the exact misdirection the route surfacing exists
    // to prevent (KI-217). The mode reads `auto`; a pin names its
    // PROVIDER (`pinned · stepfun`), the owner's preference — the model
    // is the router's business. After a run, the route event refines the
    // label with the provider that actually served. The model name
    // renders only when no routing state is known at all.
    match (&status.route_how, status.routing.as_deref()) {
        (Some(how), _) => {
            spans.push(Span::styled("  route ", Style::new().fg(palette::DIM)));
            if how == "pinned" {
                spans.push(Span::styled("pinned", Style::new().fg(palette::ACCENT)));
                if let Some(provider) = &status.provider {
                    spans.push(Span::styled(
                        format!(" \u{b7} {provider}"),
                        Style::new().fg(palette::ACCENT),
                    ));
                }
            } else {
                spans.push(Span::styled("auto", Style::new().fg(palette::ACCENT)));
                spans.push(Span::styled(
                    format!(" ({how})"),
                    Style::new().fg(palette::DIM),
                ));
            }
        }
        (None, Some(mode)) => {
            spans.push(Span::styled("  route ", Style::new().fg(palette::DIM)));
            spans.push(Span::styled(
                mode.to_string(),
                Style::new().fg(palette::ACCENT),
            ));
        }
        (None, None) => spans.push(Span::styled(
            &status.model,
            Style::new().fg(palette::ACCENT),
        )),
    }
    spans.push(Span::styled("  session ", Style::new().fg(palette::DIM)));
    spans.push(Span::styled(
        // A launch that has not run yet has no session: the log is
        // created on the first task (KI-022), and naming an id that
        // does not exist would be a lie about where runs are logged.
        if status.session_id.is_empty() {
            "\u{2014}".to_string()
        } else {
            short_id(&status.session_id)
        },
        Style::new().fg(palette::TEXT),
    ));
    spans.push(Span::styled("  turn ", Style::new().fg(palette::DIM)));
    spans.push(Span::styled(turn, Style::new().fg(palette::TEXT)));
    spans.push(Span::styled("  tokens ", Style::new().fg(palette::DIM)));
    spans.push(Span::styled(
        format!("{}/{}", status.prompt_tokens, status.completion_tokens),
        Style::new().fg(palette::TEXT),
    ));
    // What the last run's assembled memory carried: the shape the
    // instruction routed under and the fact count. Slice 8 part 2's
    // half of the turn-path surfacing — the `memory` record's live
    // counterpart, and absent on a run whose assembly was empty.
    // Plan mode is a composer toggle with consequences for the next
    // run, so it belongs where the owner types, not only in the run card.
    if app.run.plan {
        spans.push(Span::styled("  plan ", Style::new().fg(palette::DIM)));
        spans.push(Span::styled("on", Style::new().fg(palette::PENDING)));
    }
    // The active surface, so the pane's title is never the only place
    // it is named. The run tab names itself; the board tabs name the
    // board the refresher is pointed at.
    if app.pane == Pane::Run {
        spans.push(Span::styled("  pane ", Style::new().fg(palette::DIM)));
        spans.push(Span::styled("run", Style::new().fg(palette::ACCENT)));
    } else {
        spans.push(Span::styled("  board ", Style::new().fg(palette::DIM)));
        spans.push(Span::styled(
            app.boards.active.label(),
            Style::new().fg(palette::ACCENT),
        ));
    }
    if let Some((shape, facts)) = &status.memory {
        spans.push(Span::styled("  memory ", Style::new().fg(palette::DIM)));
        let named = if shape.is_empty() {
            "no shape".to_string()
        } else {
            shape.clone()
        };
        let plural = if *facts == 1 { "" } else { "s" };
        spans.push(Span::styled(
            format!("{named} \u{b7} {facts} fact{plural}"),
            Style::new().fg(palette::ACCENT),
        ));
    }
    // KI-077: a width budget for the bar. The segments used to run to
    // whatever width they needed and the last one clipped mid-word
    // unreadable; now the bar is trimmed to the area by priority: the
    // state and the routing mode are what the bar is for, and the
    // board, memory, tokens, turn, and session segments are what a
    // narrow bar drops — in that order. The session id is the last to
    // go, because it is already abbreviated and its tail is the part
    // that identifies the log.
    let width = line_area.width as usize;
    // The boundaries: the spans are built in a fixed order, and the
    // budget walks them by priority group rather than by position.
    let total = spans.iter().map(|span| span.width()).sum::<usize>();
    let line = if total <= width {
        Line::from(spans)
    } else {
        // Drop whole segments from the end until it fits: the order the
        // spans were pushed is the order they matter in, so the tail
        // segments are the cheap ones.
        let mut kept: Vec<Span> = Vec::new();
        let mut used = 0usize;
        for span in &spans {
            let w = span.width();
            if used + w > width {
                break;
            }
            kept.push(span.clone());
            used += w;
        }
        Line::from(kept)
    };
    frame.render_widget(Paragraph::new(line), line_area);
    if let Some(gauge_area) = gauge_area {
        render_context_gauge(frame, app, gauge_area);
    }
}

/// The context meter: the last request's prompt against the compaction
/// budget. A gauge, not a second number in the bar, because the shape is
/// the point — how full the window is before the next fold.
fn render_context_gauge(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let budget = crate::compaction::COMPACTION_TOKEN_BUDGET as f64;
    let ratio = if budget > 0.0 {
        (app.run.context as f64 / budget).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let percent = (ratio * 100.0).round() as u16;
    let gauge = LineGauge::default()
        .ratio(ratio)
        .label(format!("ctx {percent}%"))
        .filled_style(Style::new().fg(if percent >= 90 {
            palette::DENIED
        } else if percent >= 70 {
            palette::PENDING
        } else {
            palette::ACCENT
        }))
        .unfilled_style(Style::new().fg(palette::DIM));
    frame.render_widget(gauge, area);
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

    let (lines, cursor) = input_lines(
        &app.input,
        app.cursor,
        inner.width as usize,
        inner.height as usize,
    );
    let paragraph = Paragraph::new(Text::from(lines));
    frame.render_widget(paragraph, inner);
    if let Some((x, y)) = cursor {
        frame.set_cursor_position((inner.x + x, inner.y + y));
    }
}

/// The key hints. The run's own controls lead while it is in flight; the
/// rest of the keys stay for the idle face.
fn render_footer(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let hints = if app.run.active {
        "enter send \u{b7} ctrl+p palette \u{b7} ctrl+r run controls \u{b7} ctrl+t plan \u{b7} ctrl+o sessions \u{b7} ctrl+k checkpoints \u{b7} q quit"
    } else {
        "enter send \u{b7} ctrl+p palette \u{b7} ctrl+r run \u{b7} ctrl+t plan \u{b7} ctrl+o sessions \u{b7} ctrl+k checkpoints \u{b7} ctrl+l lanes \u{b7} ctrl+, settings \u{b7} q quit"
    };
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            hints,
            Style::new().fg(palette::TEXT),
        ))),
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
    // KI-078: the summary rides its own line, not the name's. They used
    // to share one, so a long tool name pushed the summary off the line
    // entirely — and for a delegation the goal IS the summary, so the
    // operator approved a task they could not read. On its own line the
    // summary wraps at the box width like every other wrapped surface.
    let body = Text::from(vec![
        Line::from(""),
        Line::from(Span::styled(
            &prompt.name,
            Style::new()
                .fg(palette::ACCENT)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            &prompt.summary,
            Style::new().fg(palette::TEXT),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                " [y]",
                Style::new().fg(palette::DONE).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" yes   ", Style::new().fg(palette::TEXT)),
            Span::styled(
                "[n]",
                Style::new()
                    .fg(palette::DENIED)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" no   ", Style::new().fg(palette::TEXT)),
            Span::styled(
                "[a]",
                Style::new()
                    .fg(palette::PENDING)
                    .add_modifier(Modifier::BOLD),
            ),
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
        .title(format!(
            " sessions — {} · f filter · enter resume, esc close ",
            app.session_filter.label()
        ))
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
    let rows = app.session_rows();
    if rows.is_empty() {
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                format!(
                    " no session matches {} · f cycles the filter",
                    app.session_filter.label()
                ),
                Style::new().fg(palette::DIM),
            ))),
            inner,
        );
        return;
    }

    let header = Row::new(vec![
        Cell::from("id"),
        Cell::from("served by"),
        Cell::from("state"),
        Cell::from("marks"),
        Cell::from("started"),
    ])
    .style(Style::new().fg(palette::DIM).add_modifier(Modifier::BOLD));
    let body: Vec<Row> = rows
        .iter()
        .map(|summary| {
            let (dot, state) = if summary.ended {
                (palette::DONE, "● ended")
            } else {
                (palette::PENDING, "● open")
            };
            let mut marks: Vec<&str> = Vec::new();
            if summary.id == app.status.session_id {
                marks.push("current");
            }
            if summary.kind.as_deref() == Some(crate::session::SUBAGENT_KIND) {
                marks.push("subagent");
            }
            if summary.compacted {
                marks.push("compacted");
            }
            Row::new(vec![
                Cell::from(session_label(&summary.id)),
                Cell::from(summary.serving_model().unwrap_or("routing…").to_string()),
                Cell::from(state).style(Style::new().fg(dot)),
                Cell::from(marks.join(" · ")).style(Style::new().fg(palette::DIM)),
                Cell::from(short_stamp(&summary.created)).style(Style::new().fg(palette::DIM)),
            ])
        })
        .collect();
    let table = Table::new(
        body,
        [
            // The highlight symbol eats two cells of the first column.
            Constraint::Length(22),
            Constraint::Length(15),
            Constraint::Length(9),
            Constraint::Length(12),
            Constraint::Min(10),
        ],
    )
    .header(header)
    .column_spacing(1)
    .row_highlight_style(
        Style::new()
            .bg(palette::ACCENT)
            .fg(palette::ON_ACCENT)
            .add_modifier(Modifier::BOLD),
    )
    .highlight_symbol("▸ ");
    let overflow = rows.len() > inner.height.saturating_sub(1) as usize;
    let table_area = Rect {
        width: inner.width.saturating_sub(u16::from(overflow)),
        ..inner
    };
    let mut state =
        TableState::default().with_selected(Some(app.selected.min(rows.len().saturating_sub(1))));
    frame.render_stateful_widget(table, table_area, &mut state);
    widgets::scrollbar(
        frame,
        inner,
        rows.len(),
        state.offset(),
        table_area.height.saturating_sub(1) as usize,
    );
}

/// A session id as the picker reads it: the day, the minute, and the
/// hash — enough to tell two runs apart without spelling 27 characters.
fn session_label(id: &str) -> String {
    let bytes = id.as_bytes();
    if bytes.len() >= 16 && bytes.get(8) == Some(&b'T') && bytes.get(15) == Some(&b'Z') {
        let hash: String = id
            .rsplit_once('-')
            .map(|(_, hash)| hash.chars().take(6).collect())
            .unwrap_or_default();
        if let (Some(month), Some(day), Some(hour), Some(minute)) =
            (id.get(4..6), id.get(6..8), id.get(9..11), id.get(11..13))
        {
            return format!("{month}-{day} {hour}:{minute} \u{b7} {hash}");
        }
    }
    id.chars().take(20).collect()
}

/// A session's created stamp, shortened to the part a picker reads. The
/// archive's ids are the same stamp, so this is a rendering of the id's
/// own prefix rather than a second date.
fn short_stamp(stamp: &str) -> String {
    let text = String::from(stamp);
    if text.len() >= 16 && text.as_bytes().get(10) == Some(&b'T') {
        format!("{} {}", &text[5..10], &text[11..16])
    } else {
        text.chars().take(16).collect()
    }
}

/// The lanes modal's height in rows: one row per rendered line, the
/// border, and a line of headroom. A lane that carries a `doctor` report
/// (blend1) renders one line per sub-lane, so the modal is sized from the
/// lines it actually drew rather than a fixed two-per-lane share — a
/// card clipped by its own modal is the failure the boards already
/// learned from. Clamped at both ends, so a small frame or an unexpected
/// line count cannot push the modal off screen.
#[cfg(test)]
fn lanes_height(lines: usize) -> u16 {
    let wanted = (lines as u16).saturating_add(4);
    wanted.clamp(4, 20)
}

/// The explanation modal's width, as a share of the frame: the card is
/// 72 columns wide, so the modal takes the whole frame rather than a
/// share — a card clipped by its own modal is the failure the boards
/// learned from.
const EXPLAIN_MODAL_PERCENT: u16 = 96;

/// The explanation modal's height: one row per card line, the border,
/// and a line of headroom — clamped, so a small frame cannot push the
/// modal off screen, and a card longer than the pane names what it is
/// holding back rather than silently cutting its tail.
fn explain_height(lines: usize) -> u16 {
    (lines as u16).saturating_add(4).clamp(4, 30)
}

/// The latest explanation card over the conversation: the headless
/// card's own lines, drawn verbatim in a modal that scrolls. The two
/// surfaces agree because they are the same text — the loop reads the
/// file the beat wrote and hands the state machine its lines.
/// The settings modal's width, as a percentage of the frame.
const SETTINGS_MODAL_PERCENT: u16 = 70;

/// The destructive confirmation's width, as a share of the frame: the
/// same share the sessions picker takes, so the two overlays read as
/// one family.
const SETTINGS_CONFIRM_PERCENT: u16 = 78;

fn render_explain(frame: &mut Frame<'_>, app: &App) {
    let [columns] = Layout::horizontal([Constraint::Percentage(EXPLAIN_MODAL_PERCENT)])
        .flex(Flex::Center)
        .areas(frame.area());
    let width = columns.width.saturating_sub(2).max(1) as usize;
    let Some(card) = app.explain.as_ref() else {
        return;
    };
    let mut lines: Vec<Line<'static>> = card
        .iter()
        .map(|line| line_of(vec![(line.clone(), Style::new().fg(palette::TEXT))], width))
        .collect();
    let area = centered_sized(
        EXPLAIN_MODAL_PERCENT,
        explain_height(lines.len()),
        frame.area(),
    );
    frame.render_widget(Clear, area);
    let block = Block::bordered()
        .title(" explain — esc close ")
        .title_style(title_style())
        .border_style(Style::new().fg(palette::ACCENT));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.height == 0 {
        return;
    }
    let visible = inner.height as usize;
    if lines.len() > visible {
        let hidden = lines.len() - visible + 1;
        lines.truncate(visible.saturating_sub(1));
        lines.push(Line::from(Span::styled(
            format!(" +{hidden} more"),
            Style::new().fg(palette::DIM),
        )));
    }
    frame.render_widget(Paragraph::new(Text::from(lines)), inner);
}

/// The lanes modal's width, as a share of the frame. Wide enough for a
/// lane row plus a doctor row with its reason; the same share the
/// sessions picker takes, so the two overlays read as one family.
fn wrapped(indent: &str, text: &str, style: Style, width: usize) -> Vec<Line<'static>> {
    let room = width.saturating_sub(indent.chars().count()).max(8);
    let chunks = wrap(text, room);
    let mut out = Vec::with_capacity(chunks.len());
    for (index, chunk) in chunks.into_iter().enumerate() {
        let pad = if index == 0 {
            indent.to_string()
        } else {
            " ".repeat(indent.chars().count())
        };
        let style = if index == 0 {
            style
        } else {
            Style::new().fg(palette::DIM)
        };
        out.push(Line::from(Span::styled(format!("{pad}{chunk}"), style)));
    }
    out
}

/// One conversation entry as wrapped lines: tool calls are cards with a
/// status dot and a result preview.
/// The assistant's text with the little markdown the model actually
/// writes: headings, bullets, numbered steps, fenced code, and inline
/// code. A full parser is not the job; a terminal that shows `##` and
/// backticks raw reads worse than the words.
fn markdown_lines(text: &str, width: usize) -> Vec<Line<'static>> {
    let width = width.max(8);
    let mut lines = Vec::new();
    let mut fence = false;
    for raw in text.lines() {
        if raw.trim_start().starts_with("```") {
            fence = !fence;
            continue;
        }
        if fence {
            for chunk in wrap(raw, width.saturating_sub(3).max(4)) {
                lines.push(Line::from(vec![
                    Span::styled(" \u{2502} ", Style::new().fg(palette::DIM)),
                    Span::styled(chunk, Style::new().fg(palette::DIM)),
                ]));
            }
            continue;
        }
        let trimmed = raw.trim_end();
        if trimmed.is_empty() {
            lines.push(Line::from(""));
            continue;
        }
        if let Some((level, title)) = heading(trimmed) {
            let style = Style::new()
                .fg(if level <= 2 {
                    palette::ACCENT
                } else {
                    palette::TEXT
                })
                .add_modifier(Modifier::BOLD);
            for chunk in wrap(title, width) {
                lines.push(Line::from(Span::styled(chunk, style)));
            }
            continue;
        }
        let (prefix, rest) = if let Some(rest) = trimmed
            .strip_prefix("- ")
            .or_else(|| trimmed.strip_prefix("* "))
            .or_else(|| trimmed.strip_prefix("+ "))
        {
            ("\u{2022} ", rest)
        } else {
            ("", trimmed)
        };
        let room = width.saturating_sub(prefix.width()).max(4);
        for (index, chunk) in wrap(rest, room).into_iter().enumerate() {
            let mut spans = Vec::new();
            if index == 0 && !prefix.is_empty() {
                spans.push(Span::styled(prefix, Style::new().fg(palette::DIM)));
            } else if !prefix.is_empty() {
                spans.push(Span::styled("  ", Style::new()));
            }
            spans.extend(inline_spans(&chunk));
            lines.push(Line::from(spans));
        }
    }
    lines
}

fn heading(line: &str) -> Option<(usize, &str)> {
    let hashes = line.chars().take_while(|c| *c == '#').count();
    if hashes == 0 || hashes > 4 {
        return None;
    }
    let title = line[hashes..].trim_start();
    (!title.is_empty()).then_some((hashes, title))
}

/// Inline code: backtick spans in the accent, the rest in the text
/// colour. Paired by position; an unbalanced backtick styles the tail.
fn inline_spans(text: &str) -> Vec<Span<'static>> {
    text.split('`')
        .enumerate()
        .filter(|(_, piece)| !piece.is_empty())
        .map(|(index, piece)| {
            Span::styled(
                piece.to_string(),
                if index % 2 == 1 {
                    Style::new().fg(palette::ACCENT)
                } else {
                    Style::new().fg(palette::TEXT)
                },
            )
        })
        .collect()
}

fn entry_lines(entry: &Entry, width: usize) -> Vec<Line<'static>> {
    let width = width.max(8);
    match entry {
        Entry::User(text) => wrap(text, width)
            .into_iter()
            .enumerate()
            .map(|(index, chunk)| {
                if index == 0 {
                    Line::from(vec![
                        Span::styled(
                            "you ",
                            Style::new().fg(palette::USER).add_modifier(Modifier::BOLD),
                        ),
                        Span::styled("▸ ", Style::new().fg(palette::USER)),
                        Span::styled(chunk, Style::new().fg(palette::TEXT)),
                    ])
                } else {
                    Line::from(Span::styled(
                        format!("    {chunk}"),
                        Style::new().fg(palette::TEXT),
                    ))
                }
            })
            .collect(),
        Entry::Assistant(text) => markdown_lines(text, width),
        Entry::Tool(card) => {
            let (dot, state) = match card.status {
                ToolStatus::Pending => (palette::PENDING, "pending"),
                ToolStatus::Approved => (palette::APPROVED, "approved"),
                ToolStatus::Denied => (palette::DENIED, "denied"),
                ToolStatus::Done => (palette::DONE, "done"),
            };
            // KI-078: the goal is bounded before the state token rides
            // the line. For a subagent the goal IS the content — an
            // operator approving a delegation reads it — and an
            // unbounded one pushed the state token off the line entirely.
            // The budget is what the line has left after the name, the
            // marker, and the state, so a short goal renders whole.
            let state_w = state.chars().count() + 2;
            let budget = width
                .saturating_sub(card.name.chars().count() + 4 + state_w)
                .max(12);
            let goal = bound(&card.summary, budget);
            let mut lines = vec![Line::from(vec![
                Span::styled(
                    format!(" {} ", dot_marker(card.status)),
                    Style::new().fg(dot),
                ),
                Span::styled(
                    card.name.clone(),
                    Style::new()
                        .fg(palette::ACCENT)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("  ", Style::new()),
                Span::styled(goal, Style::new().fg(palette::DIM)),
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
            let tools = if card.tool_count == 1 {
                "1 tool"
            } else {
                " tools"
            };
            let count = if card.tool_count == 1 {
                String::new()
            } else {
                card.tool_count.to_string()
            };
            vec![Line::from(vec![
                Span::styled(" \u{2713} ", Style::new().fg(palette::DONE)),
                Span::styled(
                    card.lane.clone(),
                    Style::new()
                        .fg(palette::ACCENT)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!(" joined \u{b7} {count}{tools}"),
                    Style::new().fg(palette::DIM),
                ),
            ])]
        }
        Entry::Skills(card) => {
            // The library's session-start report, in the lane-card
            // family: a dot, the loaded count, and the skipped names —
            // wrapped through the shared word-aware helper, because a
            // fixed-width surface given a concatenated line cuts its
            // tail (KI-078's class; KI-216 is this line's reason).
            let mut detail = format!("{} loaded", card.loaded);
            if !card.skipped.is_empty() {
                let names = card.skipped.join(", ");
                detail.push_str(&format!(" \u{b7} {} skipped: {names}", card.skipped.len()));
            }
            let mut lines = vec![Line::from(vec![
                Span::styled(" \u{2713} ", Style::new().fg(palette::DONE)),
                Span::styled(
                    "skills",
                    Style::new()
                        .fg(palette::ACCENT)
                        .add_modifier(Modifier::BOLD),
                ),
            ])];
            for piece in wrap(&detail, width) {
                lines.push(Line::from(Span::styled(
                    format!("  {piece}"),
                    Style::new().fg(palette::DIM),
                )));
            }
            lines
        }
        Entry::Compaction(card) => {
            // A compaction is the same family as a tool card, rendered in
            // the same two-line shape: the dot, the bold accent name, the
            // dim detail, and the settled state — with the token counts
            // underneath, wrapped, exactly as a tool card wraps its
            // preview. The summary itself is in the next request, not
            // something to render here.
            // The settled line carries the dot, the verb, the fold's size,
            // and the state — nothing else. The summary phrase and the
            // token counts ride the wrapped detail line, because a fixed
            // width surface given a line built by concatenation cuts its
            // tail (KI-078, KI-085, KI-096): at the width this pane ships
            // beside the boards, `6 turns \u{2192} 1 summary  done` loses
            // its state word.
            let mut lines = vec![Line::from(vec![
                // KI-086: a system-level marker, so a first-time reader
                // does not take a green check for a tool named
                // "compacted". The doubled diamond reads as the harness’s own
                // event rather than the tool grammar’s single check, and the
                // system tag names it outright — the card is about the
                // context, not a call.
                Span::styled(" \u{25c6} ", Style::new().fg(palette::DIM)),
                Span::styled(
                    "compacted",
                    Style::new().fg(palette::DIM).add_modifier(Modifier::BOLD),
                ),
                Span::styled("  system  ", Style::new().fg(palette::DIM)),
                Span::styled(
                    format!(
                        "{} turn{}",
                        card.turns,
                        if card.turns == 1 { "" } else { "s" }
                    ),
                    Style::new().fg(palette::DIM),
                ),
                Span::styled("  done", Style::new().fg(palette::DONE)),
            ])];
            for chunk in wrap(
                &format!(
                    "\u{2192} 1 summary \u{b7} ~{} \u{2192} ~{} tokens",
                    card.tokens_before, card.tokens_after
                ),
                width.saturating_sub(4).max(8),
            ) {
                lines.push(Line::from(vec![
                    Span::styled("   \u{2514} ", Style::new().fg(palette::DIM)),
                    Span::styled(chunk, Style::new().fg(palette::DIM)),
                ]));
            }
            lines
        }
        Entry::Deck(card) => {
            // A deck is the same two-line family: a short settled line
            // naming the verb and the kind, and the shape wrapped under
            // it. A terminal cannot render the fragment — the browser
            // face does — so the card says what was built and where to
            // see it, and the fragment itself rides the log.
            let mut lines = vec![Line::from(vec![
                Span::styled(" \u{25c7} ", Style::new().fg(palette::ACCENT)),
                Span::styled(
                    "built",
                    Style::new()
                        .fg(palette::ACCENT)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("  ", Style::new()),
                Span::styled(
                    format!("a {} of {} node(s)", card.project, card.nodes),
                    Style::new().fg(palette::TEXT),
                ),
            ])];
            for chunk in wrap(
                "open the browser face to see it",
                width.saturating_sub(4).max(8),
            ) {
                lines.push(Line::from(vec![
                    Span::styled("   \u{2514} ", Style::new().fg(palette::DIM)),
                    Span::styled(chunk, Style::new().fg(palette::DIM)),
                ]));
            }
            lines
        }
        Entry::Cron(card) => {
            // A scheduled fire is the same family again, in the same
            // two-line shape as the compaction card: a short settled
            // line — the dot, the bold accent verb, the job, the state —
            // and everything else wrapped underneath, exactly as a tool
            // card wraps its preview. The first line stays short on
            // purpose: a job name, a minute phrase, and a state word
            // concatenated onto one line is the fixed-width overflow
            // the card family already learned from (KI-078, KI-085).
            // The run's own log is the retained output; the session id
            // on this card is what makes it reachable.
            let (glyph, state, color) = match card.outcome.as_str() {
                "completed" => ("\u{2713}", "done", palette::DONE),
                "skipped-needs-approval" => ("\u{2715}", "skipped", palette::PENDING),
                // A failed run and a panicking fire both read as a
                // settled failure; the detail line names which.
                _ => ("\u{2715}", "failed", palette::DENIED),
            };
            let minute = if card.catch_up {
                format!("catch-up for {}", card.minute)
            } else {
                format!("at {}", card.minute)
            };
            let mut lines = vec![Line::from(vec![
                Span::styled(format!(" {glyph} "), Style::new().fg(color)),
                Span::styled(
                    "scheduled",
                    Style::new()
                        .fg(palette::ACCENT)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("  ", Style::new()),
                Span::styled(
                    card.job.clone(),
                    Style::new()
                        .fg(palette::ACCENT)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(format!("  {state}"), Style::new().fg(color)),
            ])];
            for chunk in wrap(
                &format!("{minute} \u{b7} {}", card.detail),
                width.saturating_sub(4).max(8),
            ) {
                lines.push(Line::from(vec![
                    Span::styled("   \u{2514} ", Style::new().fg(palette::DIM)),
                    Span::styled(chunk, Style::new().fg(palette::DIM)),
                ]));
            }
            // The session id stands on its own line: a word wrap splits
            // `session <id>` between the two, and a session id cut in half
            // is not reachable (KI-078's class, at the pane's shipped
            // width beside the boards).
            if let Some(session) = &card.session {
                for chunk in wrap(
                    &format!("session {session}"),
                    width.saturating_sub(4).max(8),
                ) {
                    lines.push(Line::from(vec![
                        Span::styled("   \u{2514} ", Style::new().fg(palette::DIM)),
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
/// so words stay whole. The break lands *after* the space, so no
/// character is dropped, moved, or invented: the lines' lengths account
/// for every input character, and the cursor's row and column are found
/// by walking them with the same rules. Exact line counts matter too —
/// the conversation's scroll offset is computed from them.
/// One line bounded to `most` characters, with the cut marked. KI-078:
/// the tool card's goal rides the name's line, and an unbounded goal
/// pushed the state token off the line — so the goal is bounded to
/// what the line has left, and the mark says it was cut rather than
/// pretending the whole goal was there.
fn bound(text: &str, most: usize) -> String {
    let count = text.chars().count();
    if count <= most {
        return text.to_string();
    }
    if most <= 1 {
        return "\u{2026}".to_string();
    }
    let cut: String = text.chars().take(most - 1).collect();
    format!("{cut}\u{2026}")
}

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
            let word_width = word.width();
            // A word longer than the line is split hard; it has no choice.
            // KI-025: the widths are display CELLS, not characters — CJK,
            // emoji, and combining marks occupy a different number of
            // columns than of `char`s, and a character count wrapped them
            // wrong.
            if !current.is_empty() && current.width() + word_width > width {
                lines.push(std::mem::take(&mut current));
            }
            for character in word.chars() {
                current.push(character);
                if current.width() >= width {
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
                // Characters find the line; the COLUMN is the line's
                // display width up to the cursor's character. The two
                // measures are different things, and a CJK line's
                // column is not its character count (KI-025).
                let len = line.chars().count();
                let last_line = offset + 1 == lines.len();
                if within < seen + len {
                    let prefix: String = line.chars().take(within - seen).collect();
                    return (row + offset, prefix.width());
                }
                if within == seen + len {
                    // A boundary shows on the next line when there is
                    // one — here or in the next paragraph — so the caret
                    // stays inside the box; the text's very end stays.
                    if !last_line || !last_para {
                        return (row + offset + 1, 0);
                    }
                    return (row + offset, line.width());
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
    let skip = tail
        .max(cursor_row.saturating_sub(height - 1))
        .min(cursor_row);
    let visible: Vec<String> = all.iter().skip(skip).take(height).cloned().collect();
    let lines: Vec<Line<'static>> = visible
        .iter()
        .map(|line| Line::from(Span::styled(line.clone(), Style::new().fg(palette::TEXT))))
        .collect();
    let row = cursor_row.saturating_sub(skip);
    let cursor =
        (row < visible.len() && cursor_col < width).then_some((cursor_col as u16, row as u16));
    (lines, cursor)
}

/// A centered overlay of the given percentages.
fn centered(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let [area] = Layout::vertical([Constraint::Percentage(percent_y)])
        .flex(Flex::Center)
        .areas(area);
    let [area] = Layout::horizontal([Constraint::Percentage(percent_x)])
        .flex(Flex::Center)
        .areas(area);
    area
}

/// A centered overlay of the given width percentage and an exact height in
/// rows. `centered`'s percentages cannot express a small height — 4% of a
/// 24-row frame rounds to zero rows, and a zero-row modal renders nothing
/// — so a view sized from its own content asks for rows.
fn centered_sized(percent_x: u16, height: u16, area: Rect) -> Rect {
    let height = height.min(area.height);
    let [area] = Layout::vertical([Constraint::Length(height)])
        .flex(Flex::Center)
        .areas(area);
    let [area] = Layout::horizontal([Constraint::Percentage(percent_x)])
        .flex(Flex::Center)
        .areas(area);
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
    use crate::lanes::LaneState;

    /// The settings pane renders as a scrolled list of rows, never a
    /// plain-text table, and the editor replaces the list while a value
    /// is being typed. The buffer is the truth.
    /// KI-077: the status bar fits its width. The route segment — what
    /// the bar is for — survives a narrow bar, and the segments that
    /// follow are what get dropped rather than clipping the last one
    /// mid-word.
    /// KI-025: the wrap and cursor math measures display CELLS, not
    /// characters. A CJK line is two columns per character, so a width
    /// of 10 holds five of them, not ten — and the cursor's column is
    /// the display width of the prefix, not its character count.
    #[test]
    fn the_width_math_measures_display_cells() {
        // Five CJK characters are ten cells: a width-10 line holds all
        // five on one line, where a character count would have split.
        let cjk = "\u{4f60}\u{597d}\u{4e16}\u{754c}\u{6d4b}\u{8bd5}";
        assert_eq!(cjk.chars().count(), 6);
        assert_eq!(cjk.width(), 12);
        let lines = wrap(cjk, 12);
        assert_eq!(lines.len(), 1, "{lines:?}");
        // Below the whole line's width it wraps at the cell budget: four
        // CJK characters are exactly eight cells, so a width-8 line holds
        // them and the rest rides the next line.
        let narrow = wrap(cjk, 8);
        assert_eq!(
            narrow,
            vec!["\u{4f60}\u{597d}\u{4e16}\u{754c}", "\u{6d4b}\u{8bd5}"]
        );
        // The cursor at character 2 of a CJK line is at column 4.
        let (row, col) = cursor_position(cjk, 2, 12);
        assert_eq!(row, 0);
        assert_eq!(col, 4, "two CJK characters are four columns");
        // ASCII is unaffected: one character is one cell, and the
        // word-aware break still keeps words whole where it can.
        assert_eq!(wrap("hello world", 5), vec!["hello ", "world"]);
    }

    #[test]
    fn the_status_bar_fits_its_width_and_keeps_the_route() {
        let mut routed = fresh();
        routed.status.routing = Some("auto".to_string());
        let chat = draw(&routed, 60, 24);
        assert!(chat.contains("route auto"), "the route survives: {chat}");
        // The bar fits its width: the route survives, and the line that
        // carries it is not wider than the area — what used to happen
        // is the last segment clipping mid-word off the edge.
        let bar = chat
            .lines()
            .find(|line| line.contains("route auto"))
            .expect("the route segment");
        assert!(
            bar.chars().count() <= 60,
            "the bar fits its width: {bar} ({})",
            bar.chars().count()
        );
    }

    /// KI-078: the tool card's goal is bounded before the state token,
    /// so a long delegation goal cannot push the state off the line.
    #[test]
    fn a_long_goal_cannot_push_the_state_token_off_the_line() {
        let long = "g".repeat(200);
        assert_eq!(bound(&long, 20).chars().count(), 20, "{}", bound(&long, 20));
        assert!(
            bound(&long, 20).ends_with('\u{2026}'),
            "{}",
            bound(&long, 20)
        );
        assert_eq!(bound("short", 20), "short");
        assert_eq!(bound("", 20), "");
    }

    /// KI-086: a compaction card carries the system marker, so a
    /// first-time reader does not take it for a tool named "compacted".
    #[test]
    fn a_compaction_card_carries_the_system_marker() {
        let mut app = fresh();
        app.apply_ui_event(crate::agent::UiEvent::Compacted {
            turns: 6,
            tokens_before: 91_234,
            tokens_after: 21_003,
        });
        let chat = draw(&app, 80, 24);
        assert!(chat.contains("system"), "{chat}");
        assert!(chat.contains("compacted"), "{chat}");
    }

    #[test]
    fn the_settings_pane_renders_its_rows_and_the_editor_replaces_them() {
        let home = std::env::temp_dir().join(format!("fleet-tui-render-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).unwrap();
        let mut app = App::new("deepseek-flash", "session-1");
        app.open_settings(home.clone());
        let backend = TestBackend::new(100, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let text = buffer_text(terminal.backend().buffer());
        // The pane names itself and its keys.
        assert!(text.contains("settings"), "{text}");
        assert!(text.contains("enter edits"), "{text}");
        assert!(text.contains("pin"), "{text}");
        assert!(text.contains("approvals"), "{text}");
        // Every setting's row carries its value and source.
        assert!(text.contains("unset"), "{text}");
        // Now the editor: the list is replaced by the buffer.
        app.apply(Action::Pick);
        for character in "stepfun".chars() {
            app.apply(Action::Input(character));
        }
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let editing = buffer_text(terminal.backend().buffer());
        assert!(editing.contains("enter writes"), "{editing}");
        assert!(editing.contains("> stepfun"), "{editing}");
        // And the list's own rows are gone from the editor.
        assert!(!editing.contains("enter edits"), "{editing}");
        let _ = std::fs::remove_dir_all(&home);
    }

    /// One buffer's text, as one string, for a render assertion.
    fn buffer_text(buffer: &ratatui::buffer::Buffer) -> String {
        let mut out = String::new();
        for y in 0..buffer.area.height {
            for x in 0..buffer.area.width {
                out.push(buffer[(x, y)].symbol().chars().next().unwrap_or(' '));
            }
            out.push('\n');
        }
        out
    }

    /// A scratch home the settings pane can read and write, carrying
    /// both classes of list entry.
    fn list_home(tag: &str) -> std::path::PathBuf {
        let home =
            std::env::temp_dir().join(format!("fleet-tui-lists-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).unwrap();
        crate::settings::set_provider(&home, r#"{"id":"stepfun","enabled":false}"#).unwrap();
        crate::settings::set_provider(
            &home,
            r#"{"id":"my-own-gateway","base_url":"http://127.0.0.1:9/v1"}"#,
        )
        .unwrap();
        crate::settings::set_workload(&home, "quick", &["stepfun/step-3.7-flash".to_string()])
            .unwrap();
        home
    }

    /// The settings modal's interior: the rows and columns strictly
    /// inside its border. An assertion about the pane's own content must
    /// never trip over another pane's border, so the modal is located by
    /// its title and walked down its left edge.
    fn settings_interior(text: &str) -> String {
        let rows: Vec<Vec<char>> = text.lines().map(|line| line.chars().collect()).collect();
        let top = rows
            .iter()
            .position(|row| {
                let line: String = row.iter().collect();
                line.contains(" settings ") && line.contains('\u{250c}')
            })
            .expect("the settings modal's top border");
        let left = rows[top]
            .iter()
            .position(|cell| *cell == '\u{250c}')
            .expect("the modal's left edge");
        let right = rows[top]
            .iter()
            .rposition(|cell| *cell == '\u{2510}')
            .expect("the modal's right edge");
        let mut bottom = top;
        while bottom + 1 < rows.len() && rows[bottom + 1].get(left) == Some(&'\u{2502}') {
            bottom += 1;
        }
        let mut out = String::new();
        for row in &rows[top + 1..bottom] {
            let line: String = row[left + 1..right.min(row.len())].iter().collect();
            out.push_str(line.trim_end());
            out.push('\n');
        }
        out
    }

    /// The pane renders the two list surfaces as cards beside the scalar
    /// knobs, never a plain-text table: one card per entry the file
    /// carries, the compiled-in fallback named where a file carries none,
    /// and no table glyphs anywhere inside the pane.
    #[test]
    fn the_settings_pane_renders_the_lists_as_cards_and_never_a_table() {
        let home = list_home("cards");
        let mut app = App::new("deepseek-flash", "session-1");
        app.open_settings(home.clone());
        let interior = settings_interior(&draw(&app, 120, 60));
        // The scalars are still there, beside the lists.
        assert!(interior.contains("pin"), "{interior}");
        assert!(interior.contains("approvals"), "{interior}");
        assert!(interior.contains("unset"), "{interior}");
        // The two sections, named.
        assert!(interior.contains("Providers"), "{interior}");
        assert!(interior.contains("Workloads"), "{interior}");
        // One card per entry the file carries, with its own facts.
        assert!(interior.contains("stepfun"), "{interior}");
        assert!(interior.contains("disabled"), "{interior}");
        assert!(interior.contains("my-own-gateway"), "{interior}");
        assert!(interior.contains("http://127.0.0.1:9/v1"), "{interior}");
        assert!(interior.contains("stepfun/step-3.7-flash"), "{interior}");
        // The removal's class rides each card: the override reverts, the
        // file-added one is the destructive class.
        assert!(interior.contains("override"), "{interior}");
        assert!(interior.contains("added by the file"), "{interior}");
        // And the affordance is named on the pane's own footer.
        assert!(interior.contains("r removes a list entry"), "{interior}");
        // Cards, never a table: no column separators, no junction
        // glyphs, no double rules anywhere inside the pane.
        for glyph in [
            '\u{2500}', '\u{2502}', '\u{250c}', '\u{2510}', '\u{2514}', '\u{2518}', '\u{251c}',
            '\u{2524}', '\u{252c}', '\u{2534}', '\u{253c}', '\u{2550}', '\u{2551}',
        ] {
            assert!(
                !interior.contains(glyph),
                "a table glyph rendered inside the pane: {glyph:?}"
            );
        }
        // The selection's own card names its affordance in place.
        let row = app
            .settings_rows()
            .iter()
            .position(|row| matches!(row, SettingsRow::Provider(_)))
            .expect("a provider card");
        app.selected = row;
        let selected = settings_interior(&draw(&app, 120, 60));
        assert!(selected.contains("enter edits this entry"), "{selected}");
        assert!(selected.contains("r removes it"), "{selected}");
        let _ = std::fs::remove_dir_all(&home);
    }

    /// A file that carries no list entries says so, and names what
    /// serves instead — an empty section is never a silently blank one.
    #[test]
    fn an_empty_lists_section_names_what_serves_instead() {
        let home =
            std::env::temp_dir().join(format!("fleet-tui-empty-lists-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).unwrap();
        let mut app = App::new("deepseek-flash", "session-1");
        app.open_settings(home.clone());
        let interior = settings_interior(&draw(&app, 120, 60));
        assert!(interior.contains("Providers"), "{interior}");
        assert!(
            interior.contains("no provider overrides — the compiled-in roster serves"),
            "{interior}"
        );
        assert!(interior.contains("Workloads"), "{interior}");
        assert!(
            interior.contains("no workload preferences — the compiled-in table serves"),
            "{interior}"
        );
        let _ = std::fs::remove_dir_all(&home);
    }

    /// The armed removal asks in a modal of its own, naming exactly what
    /// will be removed and the two answers — and the pane's list is not
    /// drawn underneath the question.
    #[test]
    fn the_destructive_confirmation_names_what_it_will_remove() {
        let home = list_home("confirm");
        let mut app = App::new("deepseek-flash", "session-1");
        app.open_settings(home.clone());
        let row = app
            .settings_rows()
            .iter()
            .position(|row| matches!(row, SettingsRow::Provider(_)))
            .expect("a provider card");
        app.selected = row;
        app.apply(Action::SettingsRemove);
        assert_eq!(app.mode, Mode::SettingsConfirm);
        let text = draw(&app, 120, 60);
        assert!(text.contains("y confirms"), "{text}");
        assert!(text.contains("any other key abandons"), "{text}");
        assert!(text.contains("stepfun"), "{text}");
        assert!(text.contains("comes back"), "{text}");
        // The pane's own list is replaced by the question.
        assert!(!text.contains("enter edits"), "{text}");
        // The file-added class names the blast radius instead. Abandoning
        // the first question writes nothing, exactly as it promises.
        app.apply(Action::SettingsAbandon);
        assert_eq!(app.mode, Mode::Settings);
        app.selected = app
            .settings_rows()
            .iter()
            .enumerate()
            .filter(|(_, row)| matches!(row, SettingsRow::Provider(_)))
            .nth(1)
            .map(|(index, _)| index)
            .expect("a second provider card");
        app.apply(Action::SettingsRemove);
        let destructive = draw(&app, 120, 60);
        assert!(destructive.contains("my-own-gateway"), "{destructive}");
        assert!(destructive.contains("every walk"), "{destructive}");
        assert!(destructive.contains("does not come back"), "{destructive}");
        let _ = std::fs::remove_dir_all(&home);
    }

    use crate::agent::UiEvent;
    use crate::session::SessionSummary;
    use crate::tui::app::{Action, App, ApprovalPrompt, Entry, Mode, SettingsRow};
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
        App::new("deepseek-flash", "20261013T101010Z-abcdef")
    }

    #[test]
    fn every_mode_renders_without_a_panic() {
        let mut app = fresh();
        app.apply_ui_event(UiEvent::Turn(2));
        app.apply_ui_event(UiEvent::Token("thinking out loud".to_string()));
        app.apply_ui_event(UiEvent::ToolCall {
            name: "bash".to_string(),
            summary: "ls -la".to_string(),
            arguments: None,
        });
        app.apply_ui_event(UiEvent::ToolResult {
            name: "bash".to_string(),
            preview: "exit 0".to_string(),
            approved: true,
            resolution: None,
        });
        app.entries.push(Entry::User("do the thing".to_string()));

        let chat = draw(&app, 80, 24);
        assert!(chat.contains("conversation"), "{chat}");
        assert!(chat.contains("board"), "{chat}");
        assert!(chat.contains("deepseek-flash"), "{chat}");
        assert!(chat.contains("thinking out loud"), "{chat}");
        assert!(chat.contains("do the thing"), "{chat}");
        assert!(chat.contains("ls -la"), "{chat}");
        assert!(chat.contains("exit 0"), "{chat}");
        assert!(chat.contains("session"), "{chat}");

        // The bar names the routing mode, never the default model (KI-217):
        // at launch the mode is the home's own state — `auto`, or the
        // owner's pin naming its PROVIDER — and the compiled-in
        // `deepseek-flash` never renders.
        let mut routed_launch = fresh();
        routed_launch.status.routing = Some("auto".to_string());
        let chat = draw(&routed_launch, 80, 24);
        assert!(chat.contains("route auto"), "{chat}");
        assert!(!chat.contains("deepseek-flash"), "{chat}");

        // A pin names the provider (`stepfun`, not `stepfun/step-5-preview`):
        // the model is the router's business, the provider is the owner's.
        let mut pinned_launch = fresh();
        pinned_launch.status.routing = Some("pinned \u{b7} stepfun".to_string());
        let chat = draw(&pinned_launch, 80, 24);
        assert!(chat.contains("pinned \u{b7} stepfun"), "{chat}");
        assert!(!chat.contains("deepseek-flash"), "{chat}");

        // After a run reports, the route event refines the label — a pinned
        // run names the provider that served, an auto run stays `auto`.
        let mut pinned_run = fresh();
        pinned_run.status.routing = Some("auto".to_string());
        pinned_run.status.route_how = Some("pinned".to_string());
        pinned_run.status.provider = Some("deepseek".to_string());
        let chat = draw(&pinned_run, 80, 24);
        assert!(chat.contains("route pinned \u{b7} deepseek"), "{chat}");
        assert!(!chat.contains("route auto"), "{chat}");
        let mut judged_run = fresh();
        judged_run.status.routing = Some("auto".to_string());
        judged_run.status.route_how = Some("judged".to_string());
        let chat = draw(&judged_run, 80, 24);
        assert!(chat.contains("auto (judged)"), "{chat}");

        // The picker overlay.
        app.open_sessions(vec![SessionSummary {
            first_text: None,
            id: "20261013T101010Z-abcdef".to_string(),
            model: "deepseek-flash".to_string(),
            created: "2026-10-13T10:10:10Z".to_string(),
            ended: true,
            kind: None,
            workspace: Some("/Users/owner/projects/fleet".to_string()),
            workspace_inferred: false,
            channel: None,
            compacted: false,
            served_model: None,
            stop_reason: None,
            turns: None,
            last_write: None,
            path: std::path::PathBuf::from("/tmp/x.jsonl"),
        }]);
        let picker = draw(&app, 120, 40);
        assert!(picker.contains("sessions"), "{picker}");
        assert!(picker.contains("abcdef"), "{picker}");
        assert!(picker.contains("ended"), "{picker}");
        assert!(picker.contains("current"), "{picker}");
        // An ordinary session carries no compaction marker.
        assert!(!picker.contains("compacted"), "{picker}");

        // A delegated child's log is an ordinary session with a marker:
        // the picker names the kind beside the row.
        let mut children = fresh();
        children.open_sessions(vec![SessionSummary {
            first_text: None,
            id: "20261013T111111Z-fedcba".to_string(),
            model: "deepseek-flash".to_string(),
            created: "2026-10-13T11:11:11Z".to_string(),
            ended: true,
            kind: Some(crate::session::SUBAGENT_KIND.to_string()),
            workspace: None,
            workspace_inferred: false,
            channel: None,
            compacted: false,
            served_model: None,
            stop_reason: None,
            turns: None,
            last_write: None,
            path: std::path::PathBuf::from("/tmp/child.jsonl"),
        }]);
        let child_picker = draw(&children, 120, 40);
        assert!(child_picker.contains("subagent"), "{child_picker}");
        assert!(child_picker.contains("fedcba"), "{child_picker}");

        // A compacted session replays as a compacted shape, so the picker
        // marks it before it is opened.
        let mut folded = fresh();
        folded.open_sessions(vec![SessionSummary {
            first_text: None,
            id: "20261013T121212Z-cabbag".to_string(),
            model: "deepseek-flash".to_string(),
            created: "2026-10-13T12:12:12Z".to_string(),
            ended: true,
            kind: None,
            workspace: None,
            workspace_inferred: false,
            channel: None,
            compacted: true,
            served_model: None,
            stop_reason: None,
            turns: None,
            last_write: None,
            path: std::path::PathBuf::from("/tmp/folded.jsonl"),
        }]);
        let folded_picker = draw(&folded, 120, 40);
        assert!(folded_picker.contains("compacted"), "{folded_picker}");
        assert!(folded_picker.contains("cabbag"), "{folded_picker}");

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

        // The settings pane, its editor, and the armed removal — at a
        // standard size and at a tiny one, where a modal smaller than
        // its own border still renders rather than panicking.
        let home =
            std::env::temp_dir().join(format!("fleet-tui-every-mode-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).unwrap();
        crate::settings::set_provider(&home, r#"{"id":"stepfun","enabled":false}"#).unwrap();
        crate::settings::set_workload(&home, "quick", &["stepfun/step-3.7-flash".to_string()])
            .unwrap();
        let mut settings = fresh();
        settings.open_settings(home.clone());
        // Tall enough for every section: the settings table grows with the
        // product (remote.public_url was the row that pushed Providers
        // past a 24-line view), and the pane scrolls, so the assertion is
        // about the sections existing, not about one screen holding all.
        let pane = draw(&settings, 80, 60);
        assert!(pane.contains("Providers"), "{pane}");
        assert!(pane.contains("Workloads"), "{pane}");
        assert!(pane.contains("stepfun/step-3.7-flash"), "{pane}");
        let tiny_pane = draw(&settings, 20, 6);
        assert!(tiny_pane.contains("settings"), "{tiny_pane}");
        settings.selected = settings
            .settings_rows()
            .iter()
            .position(|row| matches!(row, SettingsRow::Provider(_)))
            .expect("a provider card");
        settings.apply(Action::Pick);
        let editor = draw(&settings, 80, 24);
        assert!(editor.contains("enter writes"), "{editor}");
        assert!(editor.contains("provider stepfun"), "{editor}");
        let _ = draw(&settings, 20, 6);
        settings.apply(Action::CloseOverlay);
        settings.apply(Action::SettingsRemove);
        assert_eq!(settings.mode, Mode::SettingsConfirm);
        let question = draw(&settings, 80, 24);
        assert!(question.contains("y confirms"), "{question}");
        let tiny_question = draw(&settings, 20, 6);
        assert!(!tiny_question.is_empty());
        let _ = std::fs::remove_dir_all(&home);

        // A tiny terminal still renders: the panes clamp instead of dying.
        let tiny = draw(&fresh(), 20, 6);
        assert!(tiny.contains("conversation"), "{tiny}");
    }

    #[test]
    fn the_explain_modal_renders_a_real_cards_full_width_at_80_columns() {
        // The explanation modal's width budget is spent on visible
        // columns only: the card rides ANSI escapes for the terminal, and
        // the loop strips them before the lines enter the state machine,
        // so a real card's widest line survives intact at 80 columns
        // instead of being cut mid-escape (KI-140's class). The card is
        // the one `explained_card` builds — the same function the beat
        // and the TUI surface both render through.
        let verdict = crate::explain::VerdictView {
            op: "propose".to_string(),
            selector: "fix-the-ranker".to_string(),
            verdict: "refused".to_string(),
            reason: "test failed: the_ranker_orders_a_corpus".to_string(),
            steps: vec![
                ("fmt".to_string(), true, "ok".to_string()),
                (
                    "test".to_string(),
                    false,
                    "error: test failed: the_ranker_orders_a_corpus".to_string(),
                ),
            ],
            input_sha256: Some("a1b2c3d4e5f60718".to_string()),
        };
        // A long answer so the prose wraps to the card's full inner
        // width — the line most at risk of a mid-escape cut when the
        // escapes are counted as visible columns.
        let answer = "The `test` step refused the change because the patched comparator sorts \
                      on the score's absolute value rather than its rank, so the ranking \
                      assertion no longer holds and the corpus regresses on every ordering \
                      check that depends on the rank comparison the patch replaced."
            .to_string();
        let reply = crate::explain::BlendReply {
            final_answer: answer,
            needs_generation: Some(0.978),
            path: "synthesized".to_string(),
            answer_shape: "synthesized".to_string(),
            validation_outcome: "passed".to_string(),
            answers_the_question: Some(true),
            contradicts_facts: Some(false),
            number_check: "ok".to_string(),
            cost_usd: Some(0.0018),
            latency_ms: Some(15_400),
            generated_status: "ok".to_string(),
            route_status: "ok".to_string(),
            generated_error: None,
        };
        let card = crate::explain::explained_card(
            &verdict,
            "a1b2c3d4e5f60718",
            &reply,
            std::path::Path::new("/x/explanations/a1b2c3d4e5f60718.md"),
        );
        // The loop strips ANSI before the lines enter the state machine —
        // the same transformation `open_explanation` applies.
        let lines: Vec<String> = crate::eval::strip_ansi(&card)
            .lines()
            .map(str::to_string)
            .collect();
        let widest = lines
            .iter()
            .max_by_key(|line| line.chars().count())
            .expect("the card has lines")
            .clone();

        let mut app = fresh();
        app.open_explain(lines);
        app.mode = Mode::Explain;
        let drawn = draw(&app, 80, 40);

        // Every drawn row fits the 80-column frame: the modal is 96% of
        // it, so its inner width is under 80 and no row overflows.
        for row in drawn.lines() {
            assert!(
                row.chars().count() <= 80,
                "a modal row overflows 80 columns: {row:?}"
            );
        }
        // The card's widest plain line survives intact — proof the width
        // budget was spent on visible columns, not eaten by the escapes
        // that would have been counted had they not been stripped.
        assert!(
            drawn.contains(widest.as_str()),
            "the modal cut the card's widest line ({}) — the escapes were not stripped",
            widest.chars().count()
        );
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
                    doctor: Vec::new(),
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
                    doctor: Vec::new(),
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

    /// A router snapshot with Fleet's own routing facts: the compiled-in
    /// chain and workload table, and a day of route rows.
    fn router_snapshot() -> crate::boards::RouterSnapshot {
        crate::boards::RouterSnapshot {
            read_at: "2026-10-16T09:41Z".to_string(),
            judge: Some(("sys1".to_string(), Some("standard_code".to_string()))),
            pin: None,
            payg: Some((false, vec!["deepseek".to_string()])),
            // The tier chains are derived from the compiled-in preference
            // table rather than hand-written, so a fixture can never
            // disagree with the table about what a workload's own chain is.
            tiers: crate::route::default_preferences()
                .iter()
                .filter(|preference| {
                    ["standard_code", "quick", "hard"].contains(&preference.workload)
                })
                .map(|preference| {
                    let candidates: Vec<String> = preference
                        .candidates
                        .iter()
                        .map(|(provider, model)| format!("{provider}/{model}"))
                        .collect();
                    crate::boards::TierChain {
                        tier: preference.workload.to_string(),
                        primary: candidates.first().cloned().unwrap_or_default(),
                        candidates,
                    }
                })
                .collect(),
            totals: Some(crate::boards::UsageTotals {
                calls: 19_873,
                ok: 19_646,
                errors: 227,
                // The route row carries no token counts, and the board
                // says so rather than estimating them.
                prompt_tokens: 0,
                completion_tokens: 0,
                total_tokens: 0,
            }),
            models: vec![
                crate::boards::ModelRow {
                    key: "stepfun/step-3.7-flash".to_string(),
                    provider: "stepfun".to_string(),
                    model: "step-3.7-flash".to_string(),
                    calls: 3_720,
                    errors: 0,
                    total_tokens: 0,
                },
                crate::boards::ModelRow {
                    key: "stepfun/step-5-preview".to_string(),
                    provider: "stepfun".to_string(),
                    model: "step-5-preview".to_string(),
                    calls: 1_791,
                    errors: 0,
                    total_tokens: 0,
                },
            ],
            recent: vec![
                crate::boards::RecentRow {
                    at: "2026-10-16T09:31:57Z".to_string(),
                    provider: "stepfun".to_string(),
                    model: "stepfun/step-3.7-flash".to_string(),
                    outcome: "served".to_string(),
                    providers_used: vec!["stepfun".to_string()],
                    workload: Some("quick".to_string()),
                    ms: Some(2_000),
                    reason: None,
                },
                crate::boards::RecentRow {
                    at: "2026-10-16T09:15:41Z".to_string(),
                    provider: "zai-coding-plan".to_string(),
                    model: "zai-coding-plan/GLM-5.3-Flash".to_string(),
                    outcome: "failed".to_string(),
                    providers_used: vec!["stepfun".to_string(), "zai-coding-plan".to_string()],
                    workload: Some("standard_code".to_string()),
                    ms: Some(812),
                    reason: Some("upstream refused".to_string()),
                },
            ],
            hours: vec![
                crate::boards::HourColumn {
                    hour: 9,
                    calls: 12_004,
                    errors: 141,
                },
                crate::boards::HourColumn {
                    hour: 14,
                    calls: 7_869,
                    errors: 86,
                },
            ],
            judge_counters: Some(crate::boards::JudgeCounters {
                fresh: 581,
                cached: 15_738,
                errors: 321,
            }),
            workloads: vec![("standard_code".to_string(), 10_557)],
            guidance: vec![(
                "stepfun".to_string(),
                "demoted \u{2014} walked last".to_string(),
            )],
            refused: Vec::new(),
        }
    }

    /// A second-brain snapshot with the derived state's own facts.
    fn brain_snapshot() -> crate::boards::BrainSnapshot {
        crate::boards::BrainSnapshot {
            read_at: "2026-10-16T09:41Z".to_string(),
            home: std::path::PathBuf::from("/Users/owner/.second-brain"),
            gates: Section::Landed(crate::boards::GatesRead {
                generated: "2026-10-04T16:13:35Z".to_string(),
                board: vec![crate::boards::GateRow {
                    gate: "plan-gate".to_string(),
                    verdicts: 24,
                    dispositions: 9,
                    graded_progress: "9/20".to_string(),
                    graded_rows: 9,
                    fit_ready: true,
                    override_rate: 0.125,
                }],
                inbox: vec![crate::boards::InboxRow {
                    op: "zcode-gate".to_string(),
                    verdict: "block".to_string(),
                    ts: "2026-10-02T20:09:10Z".to_string(),
                    command: "cd /repo && python3".to_string(),
                }],
                totals: crate::boards::GateTotals {
                    events: 107,
                    verdicts: 80,
                    dispositions: 16,
                },
                daily: vec![("2026-09-22".to_string(), 4)],
            }),
            harness: Section::Refused("harness_state.json does not exist".to_string()),
            loops: Section::Landed(crate::boards::LoopsRead {
                derived_at: "2026-10-04T11:13:42".to_string(),
                loops: vec![crate::boards::LoopRow {
                    name: "ui-capture".to_string(),
                    family: "ui".to_string(),
                    verdict: "passed".to_string(),
                    iterations: 3,
                    pending_review: 1,
                }],
                graded: vec![(
                    "ux_capture".to_string(),
                    crate::boards::GradedRow {
                        n: 127,
                        confirmed: 45,
                    },
                )],
                findings: vec!["one finding".to_string()],
            }),
            reports: Section::Landed(crate::boards::ReportsRead {
                generated_at: "2026-10-04 16:18:56".to_string(),
                doing: vec![crate::boards::DoingRow {
                    id: "t1".to_string(),
                    project: "fleet".to_string(),
                    title: "slice 14".to_string(),
                    harness: "zcode".to_string(),
                    opened: "2026-10-04 16:00:00".to_string(),
                    last: "2026-10-04 16:18:56".to_string(),
                }],
                doing_total: 1,
                pending_verification: 1,
                conflicts: Vec::new(),
                open_conflicts: 10,
                trust: Vec::new(),
                suppressed: 0,
                malformed: 0,
                events_total: 119,
                coverage: None,
                report_daily: Vec::new(),
                report_daily_by_project: Vec::new(),
                feed: Vec::new(),
            }),
            maturity: Section::Landed(crate::boards::MaturityRead {
                generated: "2026-10-04T11:13:42".to_string(),
                weeks: vec!["2026-09-14".to_string(), "2026-09-21".to_string()],
                series: vec![("output".to_string(), vec![79, 173])],
                milestones: vec![("tower online".to_string(), "2026-10-01".to_string())],
            }),
            tower: Section::Landed(crate::boards::TowerDataRead {
                generated_at: "2026-10-09 01:52".to_string(),
                projects: vec![
                    crate::boards::TowerProject {
                        name: "fleet".to_string(),
                        role: "own".to_string(),
                        lifecycle: "active".to_string(),
                        branch: Some("master".to_string()),
                        last_commit: "2026-10-14".to_string(),
                        commits_30d: 12,
                        dirty: 0,
                        plans: vec![("done".to_string(), 9)],
                        in_progress: Vec::new(),
                        planned: Vec::new(),
                        drift_days: None,
                        handoff: None,
                        daily: Vec::new(),
                    },
                    crate::boards::TowerProject {
                        name: "second-brain".to_string(),
                        role: "own".to_string(),
                        lifecycle: "active".to_string(),
                        branch: Some("main".to_string()),
                        last_commit: "2026-10-09".to_string(),
                        commits_30d: 40,
                        dirty: 3,
                        plans: vec![("done".to_string(), 4)],
                        in_progress: Vec::new(),
                        planned: Vec::new(),
                        drift_days: None,
                        handoff: None,
                        daily: Vec::new(),
                    },
                ],
                timeline: Vec::new(),
                harness: vec![crate::boards::HarnessMixRow {
                    harness: "zcode".to_string(),
                    total: 23,
                    user: 23,
                }],
            }),
        }
    }

    /// A tower snapshot with two projects' git facts.
    fn tower_snapshot() -> crate::boards::TowerSnapshot {
        crate::boards::TowerSnapshot {
            read_at: "2026-10-16T09:41Z".to_string(),
            root: std::path::PathBuf::from("/Users/owner/Projects"),
            projects: vec![
                crate::boards::ProjectFacts {
                    name: "fleet".to_string(),
                    branch: Some("master".to_string()),
                    head: Some("a421e82".to_string()),
                    subject: Some("docs: KI-114 lands".to_string()),
                    when: Some("2026-10-14".to_string()),
                    commits_30d: 12,
                    dirty: 2,
                    ahead: Some(1),
                    behind: Some(0),
                    notes: Vec::new(),
                },
                crate::boards::ProjectFacts {
                    name: "second-brain".to_string(),
                    branch: Some("main".to_string()),
                    head: Some("91c0f31".to_string()),
                    subject: Some("derive: the loops state".to_string()),
                    when: Some("2026-10-09".to_string()),
                    commits_30d: 40,
                    dirty: 0,
                    ahead: None,
                    behind: None,
                    notes: Vec::new(),
                },
            ],
            skipped: vec![("blend1".to_string(), "no .git".to_string())],
            beyond_bound: 0,
            unread: 0,
        }
    }

    fn board_app(kind: crate::boards::BoardKind, snapshot: crate::boards::BoardData) -> App {
        let mut app = fresh();
        app.boards.apply(kind, Ok(snapshot));
        app.boards.active = kind;
        app
    }

    #[test]
    fn the_router_board_renders_fleets_own_routing_state() {
        let mut app = board_app(
            crate::boards::BoardKind::Router,
            crate::boards::BoardData::Router(Box::new(router_snapshot())),
        );
        // One row more than the old board-only layout: the tab row is
        // the pane's new title line.
        let drawn = draw(&app, 120, 42);
        // The pane's title names the board, and the status bar repeats it.
        assert!(drawn.contains("board \u{2014} router"), "{drawn}");
        assert!(drawn.contains("board router"), "{drawn}");
        // The router is Fleet's own: the header says so, and names the
        // judge and the workload an unmapped ask defaults to.
        assert!(drawn.contains("router own"), "{drawn}");
        assert!(drawn.contains("judge sys1"), "{drawn}");
        assert!(drawn.contains("default standard_code"), "{drawn}");
        assert!(drawn.contains("pin \u{2014}"), "{drawn}");
        // No host, no port, no token: nothing here is dialed.
        assert!(!drawn.contains("127.0.0.1"), "{drawn}");
        // The tier chains with their primaries and their fallback counts.
        assert!(drawn.contains("standard_code"), "{drawn}");
        assert!(drawn.contains("stepfun/step-5-preview"), "{drawn}");
        assert!(drawn.contains("+2"), "{drawn}");
        // What it just served: the walk's own outcome word and dot, the
        // model that served, the workload sys1 judged, the latency.
        assert!(drawn.contains("served"), "{drawn}");
        assert!(drawn.contains("failed"), "{drawn}");
        assert!(drawn.contains("step-3.7-flash"), "{drawn}");
        assert!(drawn.contains("quick"), "{drawn}");
        assert!(drawn.contains("2,000ms"), "{drawn}");
        assert!(drawn.contains("GLM-5.3-Flash"), "{drawn}");
        // The ledger's counters, and the honest token line.
        assert!(drawn.contains("19,873"), "{drawn}");
        assert!(drawn.contains("98%"), "{drawn}");
        assert!(drawn.contains("not recorded on the route row"), "{drawn}");
        assert!(drawn.contains("3,720"), "{drawn}");
        // The judge split, the workload mix, and Fleet's own payg guard.
        assert!(drawn.contains("15,738 cached"), "{drawn}");
        assert!(drawn.contains("standard_code 10,557"), "{drawn}");
        assert!(drawn.contains("guard payg off \u{b7} deepseek"), "{drawn}");
        // Fleet's own failover feedback, out of the day's route rows.
        assert!(drawn.contains("stepfun demoted"), "{drawn}");
        assert!(drawn.contains("walked last"), "{drawn}");
        // The board is genuinely visual: a bar and a sparkline's glyphs.
        assert!(drawn.contains('\u{2588}'), "{drawn}");
        // A second cycle moves to the second-brain board.
        app.apply(crate::tui::app::Action::CycleBoard);
        assert_eq!(app.boards.active, crate::boards::BoardKind::Brain);
        let cycled = draw(&app, 120, 42);
        assert!(cycled.contains("board \u{2014} second-brain"), "{cycled}");
        assert!(cycled.contains("board second-brain"), "{cycled}");
    }

    #[test]
    fn the_second_brain_board_renders_the_derived_state_as_cards() {
        let app = board_app(
            crate::boards::BoardKind::Brain,
            crate::boards::BoardData::Brain(Box::new(brain_snapshot())),
        );
        let drawn = draw(&app, 120, 40);
        assert!(drawn.contains("board \u{2014} second-brain"), "{drawn}");
        assert!(drawn.contains(".second-brain"), "{drawn}");
        // The gate board: its totals and its earned-autonomy rows.
        assert!(drawn.contains("verdicts"), "{drawn}");
        assert!(drawn.contains("80"), "{drawn}");
        assert!(drawn.contains("plan-gate"), "{drawn}");
        assert!(drawn.contains("9/20"), "{drawn}");
        // The HITL inbox, with the blocking verdict named.
        assert!(drawn.contains("hitl 1 open"), "{drawn}");
        assert!(drawn.contains("zcode-gate"), "{drawn}");
        assert!(drawn.contains("block"), "{drawn}");
        // The loops and their graded rows.
        assert!(drawn.contains("ui-capture"), "{drawn}");
        assert!(drawn.contains("127"), "{drawn}");
        // The maturity sparkline and the doing/conflict counts.
        assert!(drawn.contains("output"), "{drawn}");
        assert!(drawn.contains("173"), "{drawn}");
        assert!(drawn.contains('\u{2584}'), "{drawn}");
        assert!(drawn.contains("slice 14"), "{drawn}");
        assert!(drawn.contains("conflicts"), "{drawn}");
        // One read that did not land is named on the board that carries it.
        assert!(
            drawn.contains("harness_state.json does not exist"),
            "{drawn}"
        );
    }

    #[test]
    fn the_router_composes_the_busiest_hours_and_the_served_gauge() {
        let app = board_app(
            crate::boards::BoardKind::Router,
            crate::boards::BoardData::Router(Box::new(router_snapshot())),
        );
        let drawn = draw(&app, 120, 42);
        assert!(drawn.contains("09:00"), "{drawn}");
        assert!(drawn.contains("% served"), "{drawn}");
        assert!(drawn.contains("tiers"), "{drawn}");
        // The chart collapses at 80x24; the ledger and tiers stay.
        let narrow = draw(&app, 80, 24);
        assert!(!narrow.contains("09:00"), "{narrow}");
        assert!(narrow.contains("ledger"), "{narrow}");
    }

    #[test]
    fn the_tower_composes_the_reports_chart_and_the_project_bars() {
        let mut app = board_app(
            crate::boards::BoardKind::Tower,
            crate::boards::BoardData::Tower(Box::new(tower_snapshot())),
        );
        let mut brain = brain_snapshot();
        if let crate::boards::Section::Landed(reports) = &mut brain.reports {
            reports.report_daily =
                vec![("2026-10-02".to_string(), 4), ("2026-10-03".to_string(), 6)];
        }
        if let crate::boards::Section::Landed(tower) = &mut brain.tower {
            tower.timeline = vec![crate::boards::TimelineRow {
                date: "2026-10-03".to_string(),
                subject: "the chart".to_string(),
                project: "fleet".to_string(),
            }];
        }
        app.apply_board(
            crate::boards::BoardKind::Brain,
            Ok(crate::boards::BoardData::Brain(Box::new(brain))),
        );
        let drawn = draw(&app, 120, 42);
        assert!(drawn.contains("reports by day"), "{drawn}");
        assert!(drawn.contains("commits by day"), "{drawn}");
        assert!(drawn.contains("fleet"), "{drawn}");
        // The chart is omitted at 80x24; the project bars and cards stay.
        let narrow = draw(&app, 80, 24);
        assert!(!narrow.contains("reports by day"), "{narrow}");
        assert!(narrow.contains("fleet"), "{narrow}");
    }

    #[test]
    fn the_tower_board_renders_git_facts_per_project() {
        let app = board_app(
            crate::boards::BoardKind::Tower,
            crate::boards::BoardData::Tower(Box::new(tower_snapshot())),
        );
        let drawn = draw(&app, 120, 40);
        assert!(drawn.contains("board \u{2014} tower"), "{drawn}");
        assert!(drawn.contains("/Users/owner/Projects"), "{drawn}");
        // The dirty project leads, with its branch, its dirty count, and
        // its ahead count; the clean one follows with its head commit.
        assert!(drawn.contains("fleet"), "{drawn}");
        assert!(drawn.contains("master"), "{drawn}");
        assert!(drawn.contains("2d"), "{drawn}");
        assert!(drawn.contains("a421e82"), "{drawn}");
        assert!(drawn.contains("docs: KI-114 lands"), "{drawn}");
        assert!(drawn.contains("second-brain"), "{drawn}");
        assert!(drawn.contains("91c0f31"), "{drawn}");
        assert!(drawn.contains("skipped"), "{drawn}");
    }

    #[test]
    fn a_refused_read_keeps_the_last_snapshot_and_names_the_error() {
        // The pane's contract, carried by a board whose source can still
        // fail: Fleet's own router read cannot refuse — there is nothing
        // to dial — so the tower scan names its refusal.
        let mut app = board_app(
            crate::boards::BoardKind::Tower,
            crate::boards::BoardData::Tower(Box::new(tower_snapshot())),
        );
        app.apply_board(
            crate::boards::BoardKind::Tower,
            Err("the tower could not list /nowhere: no such file or directory".to_string()),
        );
        let drawn = draw(&app, 120, 40);
        // The last snapshot is still on screen, whole.
        assert!(drawn.contains("fleet"), "{drawn}");
        assert!(drawn.contains("master"), "{drawn}");
        // And the refusal is named, with the time of the last good read.
        assert!(
            drawn.contains("the tower could not list /nowhere"),
            "{drawn}"
        );
        assert!(drawn.contains("last good read"), "{drawn}");
    }

    #[test]
    fn a_board_that_has_never_read_names_its_reason() {
        let mut app = fresh();
        app.apply_board(
            crate::boards::BoardKind::Tower,
            Err("the tower could not list /nowhere: no such file or directory".to_string()),
        );
        app.boards.active = crate::boards::BoardKind::Tower;
        let drawn = draw(&app, 120, 40);
        assert!(drawn.contains("no snapshot"), "{drawn}");
        assert!(
            drawn.contains("the tower could not list /nowhere"),
            "{drawn}"
        );
        // A board still on its opening read says it is reading rather
        // than lying about having nothing.
        let mut loading = fresh();
        loading.boards.active = crate::boards::BoardKind::Brain;
        let waiting = draw(&loading, 120, 40);
        assert!(waiting.contains("reading"), "{waiting}");
        assert!(!waiting.contains("no snapshot"), "{waiting}");
    }

    #[test]
    fn a_board_longer_than_its_pane_names_what_it_holds_back() {
        let mut snapshot = tower_snapshot();
        for index in 0..12 {
            snapshot.projects.push(crate::boards::ProjectFacts {
                name: format!("project-{index:02}"),
                branch: Some("main".to_string()),
                head: Some("0000000".to_string()),
                subject: Some("a commit".to_string()),
                when: Some("2026-10-01".to_string()),
                commits_30d: 1,
                dirty: 0,
                ahead: None,
                behind: None,
                notes: Vec::new(),
            });
        }
        let app = board_app(
            crate::boards::BoardKind::Tower,
            crate::boards::BoardData::Tower(Box::new(snapshot)),
        );
        let drawn = draw(&app, 120, 24);
        assert!(drawn.contains("more"), "{drawn}");
        // The pane never overflows its frame: the last row is the marker.
        let rows: Vec<&str> = drawn.lines().collect();
        assert!(rows.len() <= 24, "{} rows", rows.len());
    }

    #[test]
    fn a_healthy_board_names_its_read_time_at_both_widths() {
        // The read time is the fact that tells a second-old snapshot from
        // a twelve-minute one, so a healthy board names it too — not only
        // a refused one.
        for (width, height) in [(120, 40), (80, 24)] {
            for (kind, snapshot) in [
                (
                    crate::boards::BoardKind::Router,
                    crate::boards::BoardData::Router(Box::new(router_snapshot())),
                ),
                (
                    crate::boards::BoardKind::Brain,
                    crate::boards::BoardData::Brain(Box::new(brain_snapshot())),
                ),
                (
                    crate::boards::BoardKind::Tower,
                    crate::boards::BoardData::Tower(Box::new(tower_snapshot())),
                ),
            ] {
                let app = board_app(kind, snapshot);
                let drawn = draw(&app, width, height);
                assert!(
                    drawn.contains("read 2026-10-16T09:41Z"),
                    "{width}x{height}: {drawn}"
                );
                // The pane never overflows its frame either.
                let rows: Vec<&str> = drawn.lines().collect();
                assert!(
                    rows.len() <= height as usize,
                    "{} rows at {width}",
                    rows.len()
                );
            }
        }
    }

    #[test]
    fn the_boards_render_their_content_at_80_columns() {
        // The pane is 42 columns wide, so an 80-column terminal is the
        // narrowest the boards ever get: the composed cards must still
        // carry their facts there (the KI-115 class).
        let router = board_app(
            crate::boards::BoardKind::Router,
            crate::boards::BoardData::Router(Box::new(router_snapshot())),
        );
        let drawn = draw(&router, 80, 24);
        assert!(drawn.contains("board \u{2014} router"), "{drawn}");
        assert!(drawn.contains("router own"), "{drawn}");
        assert!(drawn.contains("default standard_code"), "{drawn}");
        assert!(drawn.contains("step-3.7-flash"), "{drawn}");
        // The chain and the rows it just served are the facts the narrowest
        // pane carries. The ledger sits below the fold there, and the board
        // names what it is holding back rather than clipping its tail
        // silently — the count is the fixture's, the naming is the
        // invariant (the KI-115 class).
        assert!(
            drawn
                .lines()
                .any(|line| line.contains(" more") && line.contains('+')),
            "{drawn}"
        );

        let brain = board_app(
            crate::boards::BoardKind::Brain,
            crate::boards::BoardData::Brain(Box::new(brain_snapshot())),
        );
        let drawn = draw(&brain, 80, 24);
        assert!(drawn.contains("board \u{2014} second-brain"), "{drawn}");
        assert!(drawn.contains("plan-gate"), "{drawn}");
        assert!(drawn.contains("9/20"), "{drawn}");
        assert!(drawn.contains("zcode-gate"), "{drawn}");
        assert!(
            drawn.contains("harness_state.json does not exist"),
            "{drawn}"
        );
        // The second-brain board is the longest of the three: at 24 rows
        // it names what it holds back rather than clipping its tail.
        assert!(drawn.contains("more"), "{drawn}");

        let tower = board_app(
            crate::boards::BoardKind::Tower,
            crate::boards::BoardData::Tower(Box::new(tower_snapshot())),
        );
        let drawn = draw(&tower, 80, 24);
        assert!(drawn.contains("board \u{2014} tower"), "{drawn}");
        assert!(drawn.contains("fleet"), "{drawn}");
        assert!(drawn.contains("master"), "{drawn}");
        assert!(drawn.contains("a421e82"), "{drawn}");
        assert!(drawn.contains("2d"), "{drawn}");
    }

    #[test]
    fn a_day_with_no_routes_renders_the_chain_and_says_the_ledger_is_empty() {
        // A fresh install, or a day Fleet has not routed on: the tier
        // chains still render and the ledger says it has seen nothing.
        // There is nothing to dial, so there is no drift to name.
        let mut snapshot = router_snapshot();
        snapshot.totals = None;
        snapshot.models = Vec::new();
        snapshot.recent = Vec::new();
        snapshot.judge_counters = None;
        snapshot.workloads = Vec::new();
        snapshot.guidance = Vec::new();
        let app = board_app(
            crate::boards::BoardKind::Router,
            crate::boards::BoardData::Router(Box::new(snapshot)),
        );
        // The tab row costs one line from the old board-only layout.
        for (width, height) in [(120, 42), (80, 25)] {
            let drawn = draw(&app, width, height);
            assert!(drawn.contains("no route rows today"), "{width}: {drawn}");
            // The chain is what a route selects, so it renders regardless.
            assert!(drawn.contains("standard_code"), "{width}: {drawn}");
            assert!(drawn.contains("stepfun/step-5-preview"), "{width}: {drawn}");
            // The gates render, and the guard still names the reserve.
            assert!(drawn.contains("default standard_code"), "{width}: {drawn}");
            assert!(drawn.contains("guard payg off"), "{width}: {drawn}");
        }
    }

    #[test]
    fn the_tower_names_a_scan_its_budget_cut_short() {
        let mut snapshot = tower_snapshot();
        snapshot.unread = 3;
        let app = board_app(
            crate::boards::BoardKind::Tower,
            crate::boards::BoardData::Tower(Box::new(snapshot)),
        );
        for (width, height) in [(120, 40), (80, 24)] {
            let drawn = draw(&app, width, height);
            // Two projects and one skipped directory were read of six.
            assert!(drawn.contains("3 of 6 projects"), "{drawn}");
            assert!(drawn.contains("the scan budget was spent"), "{drawn}");
        }
    }

    #[test]
    fn the_rendered_board_carries_no_token() {
        // The kit's localToken is a credential: it authorizes the
        // operator API and appears nowhere in a snapshot or in the
        // rendered board, at any width the pane is drawn at.
        const TOKEN: &str = "kit-local-token-4f9c2b7e";
        for (kind, snapshot) in [
            (
                crate::boards::BoardKind::Router,
                crate::boards::BoardData::Router(Box::new(router_snapshot())),
            ),
            (
                crate::boards::BoardKind::Brain,
                crate::boards::BoardData::Brain(Box::new(brain_snapshot())),
            ),
            (
                crate::boards::BoardKind::Tower,
                crate::boards::BoardData::Tower(Box::new(tower_snapshot())),
            ),
        ] {
            let app = board_app(kind, snapshot);
            for (width, height) in [(120, 40), (80, 24)] {
                let drawn = draw(&app, width, height);
                assert!(!drawn.contains(TOKEN), "{width}x{height}: {drawn}");
                assert!(!drawn.contains("Bearer"), "{width}x{height}: {drawn}");
                assert!(
                    !drawn.contains("authorization"),
                    "{width}x{height}: {drawn}"
                );
            }
        }
    }

    #[test]
    fn a_long_sessions_list_shows_the_scrollbar() {
        let mut app = fresh();
        app.open_sessions(
            (0..40)
                .map(|index| crate::session::SessionSummary {
                    first_text: None,
                    id: format!("20261013T1010{index:02}Z-{index:06x}"),
                    model: "deepseek-flash".to_string(),
                    created: "2026-10-13T10:10:10Z".to_string(),
                    ended: index % 3 == 0,
                    kind: None,
                    workspace: None,
                    workspace_inferred: false,
                    channel: None,
                    compacted: false,
                    served_model: None,
                    stop_reason: None,
                    turns: None,
                    last_write: None,
                    path: std::path::PathBuf::from("/tmp/x.jsonl"),
                })
                .collect(),
        );
        let drawn = draw(&app, 120, 40);
        assert!(drawn.contains("\u{25b2}"), "the scrollbar thumb: {drawn}");
    }

    #[test]
    fn the_lanes_modal_is_sized_from_its_rows_not_a_fixed_share() {
        // A four-lane modal is eight two-line rows plus headroom; it must
        // not keep a fixed 62% of the screen.
        assert_eq!(lanes_height(1), 5, "one line plus the border and headroom");
        assert_eq!(
            lanes_height(8),
            12,
            "four two-line rows: the old shape's height"
        );
        // Clamped at both ends, so a small frame or an unexpected row
        // count cannot push the modal off screen.
        assert_eq!(lanes_height(0), 4);
        assert_eq!(lanes_height(40), 20);
        assert!(
            lanes_height(8) < 62,
            "the modal must not keep the fixed share"
        );
    }

    /// blend1's card is the fifth one, and it renders at the house visual
    /// bar: the lane's name and role, its probe status dot, and its
    /// `doctor()` per-lane availability as named rows with status dots —
    /// green with its backing provider when a lane can answer, dim with
    /// the named reason when it cannot. The web leg rides the same
    /// report but is not a sub-lane: it renders its own accented line,
    /// the same distinction the CLI's card draws.
    #[test]
    fn blend1s_card_renders_its_doctor_rows_with_dots_and_named_reasons() {
        let mut app = fresh();
        app.apply(crate::tui::app::Action::OpenLanes);
        app.set_lanes(app.lanes_probe, vec![blend1_health()]);
        let drawn = draw(&app, 120, 40);
        // The lane's name and its role, and its probe state.
        assert!(drawn.contains("blend1"), "{drawn}");
        assert!(drawn.contains("grounded"), "{drawn}");
        assert!(drawn.contains("ok"), "{drawn}");
        // The five sub-lanes, each named, each with its dot.
        for lane in ["route", "semantic", "tabular", "generate", "validate"] {
            assert!(
                drawn.contains(lane),
                "{lane} is missing from the card: {drawn}"
            );
        }
        // The web leg's own line: its accented label — which only the
        // web line carries, so a sub-lane-shaped web row cannot pass
        // for it — and its named reason.
        assert!(
            drawn.contains("web leg"),
            "the web leg's own line is missing: {drawn}"
        );
        assert!(drawn.contains("missing TINYFISH_API_KEY"), "{drawn}");
        // An available lane names what backs it; an unavailable one names
        // what is missing — never a bare dot.
        assert!(drawn.contains("liquid (d1) via sys1"), "{drawn}");
        assert!(drawn.contains("tabpfn-hosted (Prior Labs)"), "{drawn}");
        assert!(
            drawn.contains("missing LIQUID_API_KEY (set it in ~/.config/blend1/env)"),
            "{drawn}"
        );
        // The lane's own tools, as the headless card names them.
        assert!(
            drawn.contains("ask · doctor · search · refresh · seed"),
            "{drawn}"
        );
        // The table keeps the status dots and the named reasons; it is
        // not a plain-text dump.
        assert!(!drawn.contains("| name |"), "{drawn}");
        // At 80 columns the doctor rows still carry their facts.
        let narrow = draw(&app, 80, 24);
        for lane in ["route", "semantic", "tabular", "generate", "validate"] {
            assert!(
                narrow.contains(lane),
                "{lane} is missing at 80 columns: {narrow}"
            );
        }
        assert!(
            narrow.contains("web leg"),
            "the web leg survives 80 columns: {narrow}"
        );
        // A tiny frame still renders the modal without a panic.
        let tiny = draw(&app, 20, 6);
        assert!(tiny.contains("lanes"), "{tiny}");
    }

    /// A refused probe keeps blend1's card and names the error — the
    /// boards' snapshot-and-name contract, on the lanes modal.
    #[test]
    fn a_refused_blend1_probe_keeps_its_card_and_names_the_error() {
        let mut app = fresh();
        app.apply(crate::tui::app::Action::OpenLanes);
        app.set_lanes(
            app.lanes_probe,
            vec![crate::lanes::LaneHealth {
                name: "blend1".to_string(),
                role: "grounded".to_string(),
                home: Some(std::path::PathBuf::from("/repo/services/blend1")),
                source: Some(crate::lanes::HomeSource::Bundled),
                state: LaneState::Error,
                server_version: None,
                tools: Vec::new(),
                doctor: Vec::new(),
                detail: "`tools/list` timed out after 10s".to_string(),
                elapsed_ms: 10_000,
            }],
        );
        let drawn = draw(&app, 120, 40);
        // The card is still there, with its dot and its state.
        assert!(drawn.contains("blend1"), "{drawn}");
        assert!(drawn.contains("grounded"), "{drawn}");
        assert!(drawn.contains("error"), "{drawn}");
        // And the reason is named, not swallowed.
        assert!(drawn.contains("timed out"), "{drawn}");
        // The doctor section names that its report did not land rather
        // than rendering an empty section.
        assert!(drawn.contains("doctor"), "{drawn}");
    }

    /// A blend1 health row with a full doctor report, as the probe's
    /// `doctor` tool answers it.
    fn blend1_health() -> crate::lanes::LaneHealth {
        crate::lanes::LaneHealth {
            name: "blend1".to_string(),
            role: "grounded".to_string(),
            home: Some(std::path::PathBuf::from("/repo/services/blend1")),
            source: Some(crate::lanes::HomeSource::Bundled),
            state: LaneState::Ok,
            server_version: Some("0.1.0".to_string()),
            tools: vec![
                "ask".to_string(),
                "doctor".to_string(),
                "search".to_string(),
                "refresh".to_string(),
                "seed".to_string(),
            ],
            doctor: vec![
                crate::lanes::LaneAvailability {
                    lane: "route".to_string(),
                    available: true,
                    reason: "liquid (d1) via sys1".to_string(),
                    provider: "liquid (d1) via sys1".to_string(),
                },
                crate::lanes::LaneAvailability {
                    lane: "semantic".to_string(),
                    available: true,
                    reason: "768d, 60ms".to_string(),
                    provider: "llama-server (EmbeddingGemma-2)".to_string(),
                },
                crate::lanes::LaneAvailability {
                    lane: "tabular".to_string(),
                    available: true,
                    reason: "tabpfn-hosted (Prior Labs)".to_string(),
                    provider: "tabpfn-hosted (Prior Labs)".to_string(),
                },
                crate::lanes::LaneAvailability {
                    lane: "generate".to_string(),
                    available: false,
                    reason: "missing LIQUID_API_KEY (set it in ~/.config/blend1/env)"
                        .to_string(),
                    provider: "lfm2.5-24b-a2b via api.liquid.ai".to_string(),
                },
                crate::lanes::LaneAvailability {
                    lane: "validate".to_string(),
                    available: true,
                    reason: "GLiDE (Fastino) key set; inconclusive verdicts escalate to the critic chain".to_string(),
                    provider: "glide → d1 (oversized states)".to_string(),
                },
                // The web leg rides the same Vec the live probe
                // produces — the render's own-line path is what the
                // fixture must exercise, so it carries a sixth entry
                // exactly as the probe does.
                crate::lanes::LaneAvailability {
                    lane: crate::lanes::WEB_LEG.to_string(),
                    available: false,
                    reason: "missing TINYFISH_API_KEY".to_string(),
                    provider: "tinyfish search + fetch".to_string(),
                },
            ],
            detail: "BLEND1_PYTHONPATH=/repo/services/blend1/src python3 -u /repo/services/blend1/service/mcp_server.py".to_string(),
            elapsed_ms: 53,
        }
    }

    #[test]
    fn a_lane_joining_renders_its_name_and_its_tool_count() {
        let mut app = fresh();
        app.apply_ui_event(UiEvent::ToolCall {
            name: "mcp__sem1__probe".to_string(),
            summary: "embeddings \u{b7} probe {}".to_string(),
            arguments: None,
        });
        app.apply_ui_event(UiEvent::LaneJoined {
            lane: "sem1".to_string(),
            tool_count: 4,
        });
        let drawn = draw(&app, 80, 24);
        // The lane's block names the lane and what it brought, and the
        // tool card beside it shows the model-visible namespaced name.
        assert!(drawn.contains("sem1"), "{drawn}");
        assert!(drawn.contains("joined"), "{drawn}");
        assert!(drawn.contains("4 tools"), "{drawn}");
        assert!(drawn.contains("mcp__sem1__probe"), "{drawn}");
        // A single tool reads as one, not "1 tools".
        let mut single = fresh();
        single.apply_ui_event(UiEvent::LaneJoined {
            lane: "gen1".to_string(),
            tool_count: 1,
        });
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
        assert_eq!(
            wrap("0123456789abcdefghij", 8),
            vec!["01234567", "89abcdef", "ghij"]
        );
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
        for text in [
            "aaaa bbbb cccc dddd",
            "ab\ncd",
            "abcdefghij",
            "a b c d e f g",
        ] {
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

    /// The compaction card renders in the house card style: the tool
    /// card's two-line shape — a dot, the bold accent name, the dim
    /// detail, the settled state — with the token counts wrapped
    /// underneath, so nothing is cut off at the pane's edge.
    #[test]
    fn the_compaction_card_renders_in_the_house_card_style() {
        let mut app = fresh();
        app.apply_ui_event(UiEvent::Turn(3));
        app.apply_ui_event(UiEvent::Compacted {
            turns: 6,
            tokens_before: 120_000,
            tokens_after: 900,
        });
        // The detail line's whole phrase needs the room a wide frame
        // gives the conversation beside the board pane.
        let chat = draw(&app, 120, 24);
        assert!(chat.contains("compacted"), "{chat}");
        assert!(chat.contains("6 turns"), "{chat}");
        assert!(
            chat.contains("→ 1 summary · ~120000 → ~900 tokens"),
            "{chat}"
        );
        // At 80 columns the settled line still carries its state word: a
        // fixed-width surface given a concatenated line cuts its tail
        // (KI-078, KI-085, KI-096), which is why the summary phrase moved
        // to the detail line in the first place.
        let narrow = draw(&app, 80, 24);
        assert!(narrow.contains("6 turns  done"), "{narrow}");
        // A one-turn fold never says "1 turns".
        let mut single = fresh();
        single.apply_ui_event(UiEvent::Compacted {
            turns: 1,
            tokens_before: 100_001,
            tokens_after: 40,
        });
        let rendered = draw(&single, 120, 24);
        assert!(rendered.contains("1 turn  done"), "{rendered}");
        assert!(rendered.contains("→ 1 summary"), "{rendered}");
    }

    /// The cron fire card renders in the house card style too: the tool
    /// card's two-line shape — a dot, the bold accent verb, the job, the
    /// minute, the settled state — with the outcome and the session id
    /// wrapped underneath, so nothing is cut off at the pane's edge.
    #[test]
    fn the_cron_fire_card_renders_in_the_house_card_style() {
        let mut app = fresh();
        app.apply_ui_event(UiEvent::CronFire {
            job: "daily-digest".to_string(),
            minute: "2026-10-14T12:00Z".to_string(),
            catch_up: false,
            outcome: "completed".to_string(),
            detail: "completed in 3 turns".to_string(),
            session: Some("20261014T120000Z-1a2b3c".to_string()),
        });
        // The whole detail line needs the room a wide frame gives the
        // conversation beside the board pane.
        let chat = draw(&app, 120, 24);
        assert!(chat.contains("scheduled"), "{chat}");
        assert!(chat.contains("daily-digest"), "{chat}");
        assert!(chat.contains("done"), "{chat}");
        // The minute and the outcome ride the wrapped detail line, and the
        // session id stands on its own so no wrap can halve it.
        assert!(chat.contains("at 2026-10-14T12:00Z"), "{chat}");
        assert!(chat.contains("completed in 3 turns"), "{chat}");
        assert!(chat.contains("session 20261014T120000Z-1a2b3c"), "{chat}");
        // At 80 columns the settled line still carries its state word.
        let narrow = draw(&app, 80, 24);
        assert!(narrow.contains("daily-digest  done"), "{narrow}");

        // A catch-up window is named as one on the card.
        let mut catch_up = fresh();
        catch_up.apply_ui_event(UiEvent::CronFire {
            job: "hourly".to_string(),
            minute: "2026-10-14T12:00Z".to_string(),
            catch_up: true,
            outcome: "skipped-needs-approval".to_string(),
            detail: "skipped `bash`: the job is not declared unattended and no one is here to approve it"
                .to_string(),
            session: Some("20261014T120000Z-4d5e6f".to_string()),
        });
        let rendered = draw(&catch_up, 120, 24);
        assert!(
            rendered.contains("catch-up for 2026-10-14T12:00Z"),
            "{rendered}"
        );
        assert!(rendered.contains("skipped"), "{rendered}");
        assert!(
            rendered.contains("no one is here to approve it"),
            "{rendered}"
        );

        // A failed fire names its failure and carries no session.
        let mut failed = fresh();
        failed.apply_ui_event(UiEvent::CronFire {
            job: "nightly".to_string(),
            minute: "2026-10-14T03:00Z".to_string(),
            catch_up: false,
            outcome: "panicked".to_string(),
            detail: "the fire panicked: boom".to_string(),
            session: None,
        });
        let rendered = draw(&failed, 120, 24);
        assert!(rendered.contains("failed"), "{rendered}");
        assert!(rendered.contains("the fire panicked: boom"), "{rendered}");
    }

    #[test]
    fn a_launch_that_has_not_run_yet_names_no_session() {
        // KI-022: the log is created on the first task, so a fresh launch
        // has no session id to show. The bar says so with a dash rather
        // than naming an id that does not exist, and the id the first run
        // creates is what appears.
        let mut app = App::new("deepseek-flash", "");
        let bar = draw(&app, 120, 24);
        assert!(
            bar.contains("session \u{2014}"),
            "an absent session reads as a dash: {bar}"
        );

        app.apply_ui_event(UiEvent::Session {
            id: "20261017T090000Z-abcdef".to_string(),
        });
        assert_eq!(app.status.session_id, "20261017T090000Z-abcdef");
        let named = draw(&app, 120, 24);
        assert!(
            named.contains("session 20261017T090000Z-abcdef"),
            "the created session's id is named: {named}"
        );
    }

    fn running_app() -> App {
        let mut app = fresh();
        app.set_gate(Some(true));
        app.apply(crate::tui::app::Action::Input('g'));
        app.apply(crate::tui::app::Action::Submit);
        app.apply_ui_event(UiEvent::Routed {
            provider: "stepfun".to_string(),
            model: "step-5-preview".to_string(),
            how: "judged".to_string(),
            workload: Some("standard_code".to_string()),
            verdict: Some("pass".to_string()),
            judged_from: "fresh".to_string(),
            reason: "the orchestrator walks hard's chain".to_string(),
            order: vec![("stepfun".to_string(), "step-5-preview".to_string())],
        });
        app.apply_ui_event(UiEvent::Turn(2));
        app.apply_ui_event(UiEvent::ToolCall {
            name: "bash".to_string(),
            summary: "cargo test".to_string(),
            arguments: None,
        });
        app.apply_ui_event(UiEvent::Subagent {
            goal: "read the router".to_string(),
            session: "20261020T000000Z-abcdef".to_string(),
            stage: crate::agent::SubagentStage::Working {
                calls: 2,
                tool: "bash".to_string(),
            },
        });
        app.apply_ui_event(UiEvent::Usage {
            prompt_tokens: 33_360,
            completion_tokens: 3,
        });
        app
    }

    #[test]
    fn the_run_pane_shows_the_loop_the_gate_and_the_controls() {
        let app = running_app();
        let drawn = draw(&app, 120, 40);
        assert!(drawn.contains("the run"), "{drawn}");
        assert!(drawn.contains("in flight"), "{drawn}");
        assert!(drawn.contains("route"), "{drawn}");
        assert!(drawn.contains("think"), "{drawn}");
        assert!(drawn.contains("act"), "{drawn}");
        assert!(drawn.contains("verify"), "{drawn}");
        assert!(drawn.contains("gate on \u{b7} sys1"), "{drawn}");
        assert!(drawn.contains("[s] stop"), "{drawn}");
        assert!(drawn.contains("[i] steer"), "{drawn}");
        assert!(drawn.contains("branches 1"), "{drawn}");
        assert!(drawn.contains("abcdef"), "{drawn}");
        // The gauge names the context the last request carried.
        assert!(drawn.contains("ctx 33%"), "{drawn}");
    }

    #[test]
    fn the_narrow_layout_keeps_the_run_strip_and_no_pane() {
        let app = running_app();
        let drawn = draw(&app, 60, 24);
        assert!(drawn.contains("in flight"), "{drawn}");
        assert!(drawn.contains("[ctrl-r]"), "{drawn}");
        assert!(!drawn.contains("the run "), "{drawn}");
    }

    #[test]
    fn the_run_controls_overlay_names_the_branches_and_the_two_verbs() {
        let mut app = running_app();
        app.apply(crate::tui::app::Action::OpenRunControls);
        let drawn = draw(&app, 120, 40);
        assert!(drawn.contains("run controls"), "{drawn}");
        assert!(drawn.contains("stop"), "{drawn}");
        assert!(drawn.contains("steer"), "{drawn}");
        assert!(drawn.contains("read the router"), "{drawn}");
        // The steer editor is its own one-line modal.
        app.apply(crate::tui::app::Action::OpenSteer);
        for character in "one word".chars() {
            app.apply(crate::tui::app::Action::Input(character));
        }
        let steered = draw(&app, 120, 40);
        assert!(steered.contains("steer the run"), "{steered}");
        assert!(steered.contains("one word"), "{steered}");
    }

    #[test]
    fn the_checkpoints_table_names_the_turns_and_the_diff_names_the_hunks() {
        let mut app = fresh();
        app.open_checkpoints(vec![crate::checkpoints::Snapshot {
            turn: 4,
            at: "2026-10-20T00:00:00Z".to_string(),
            files: 407,
            bytes: 20_516_618,
            unread: 0,
        }]);
        let checkpoints = draw(&app, 120, 40);
        assert!(checkpoints.contains("checkpoints"), "{checkpoints}");
        assert!(checkpoints.contains("turn"), "{checkpoints}");
        assert!(checkpoints.contains("407"), "{checkpoints}");

        app.open_diff(
            4,
            vec![crate::checkpoints::FileDiff {
                path: "src/lib.rs".to_string(),
                kind: crate::checkpoints::DiffKind::Modified,
                hunks: vec![crate::checkpoints::Hunk {
                    old: vec!["old line".to_string()],
                    new: vec!["new line".to_string()],
                    new_at: 2,
                }],
                too_large: false,
            }],
        );
        let diff = draw(&app, 120, 40);
        assert!(diff.contains("diff \u{2014} turn 4"), "{diff}");
        assert!(diff.contains("src/lib.rs"), "{diff}");
        assert!(diff.contains("modified"), "{diff}");
        assert!(diff.contains("old line"), "{diff}");
        assert!(diff.contains("new line"), "{diff}");
        assert!(diff.contains("r rejects the hunk"), "{diff}");

        app.arm_reject_hunk();
        let confirm = draw(&app, 120, 40);
        assert!(confirm.contains("reject hunk 1 of src/lib.rs"), "{confirm}");
        assert!(confirm.contains("[y] land it"), "{confirm}");
    }

    #[test]
    fn the_welcome_names_what_the_opening_boards_hold() {
        let mut app = fresh();
        app.boards.apply(
            BoardKind::Router,
            Ok(BoardData::Router(Box::new(router_snapshot()))),
        );
        app.boards.apply(
            BoardKind::Tower,
            Ok(BoardData::Tower(Box::new(tower_snapshot()))),
        );
        let drawn = draw(&app, 120, 40);
        assert!(drawn.contains("route calls today"), "{drawn}");
        assert!(drawn.contains("projects"), "{drawn}");
    }

    #[test]
    fn a_knowledge_panel_names_itself_and_its_loading_state() {
        let mut app = fresh();
        let generation = app.open_panel(crate::tui::app::PanelKind::Memory);
        let loading = draw(&app, 120, 40);
        assert!(loading.contains("memory \u{2014} all"), "{loading}");
        assert!(loading.contains("reading the machine"), "{loading}");
        app.apply_panel(
            generation,
            crate::tui::app::PanelKind::Memory,
            Err("the corpus read did not land".to_string()),
        );
        let failed = draw(&app, 120, 40);
        assert!(
            failed.contains("the corpus read did not land"),
            "the panel names the read's own refusal: {failed}"
        );
    }

    /// A drawn frame as words: box glyphs and punctuation dropped, runs
    /// of whitespace collapsed. The panels wrap prose and draw borders on
    /// every line, so an assertion about what a panel says has to be
    /// about its words rather than its line breaks.
    fn words(frame: &str) -> String {
        frame
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { ' ' })
            .collect::<String>()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// The Bots panel reads the same document the browser renders, and it
    /// says plainly which process holds the port: a panel that offered a
    /// start it could not perform would be a panel that lies.
    #[test]
    fn the_bots_panel_names_the_service_and_says_who_holds_the_port() {
        use crate::channels::service;
        use crate::channels::{Capability, ChannelReport, PeerStore, PendingStore, Phase, Status};
        let home = std::env::temp_dir().join(format!(
            "fleet-tui-bots-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).unwrap();
        let mut store = PeerStore::default();
        store.pair(
            "wh1",
            "the CI box",
            "unused",
            Capability::Task,
            "2026-10-20T00:00:00Z",
        );
        store.save(&home, "webhook").unwrap();
        let mut pending = PendingStore::default();
        pending.note("4242", "Alejandro", "2026-10-20T15:00:00Z");
        pending.save(&home, "telegram").unwrap();
        let status = Status {
            pid: 4242,
            started: "2026-10-20T15:00:00Z".to_string(),
            updated: "2026-10-20T15:00:10Z".to_string(),
            updated_unix: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            heartbeat_secs: 10,
            channels: vec![ChannelReport {
                name: "webhook".to_string(),
                phase: Phase::Live,
                port: Some(3081),
                error: None,
                since: "2026-10-20T15:00:00Z".to_string(),
            }],
        };
        status.save(&home).unwrap();
        service::record(
            &home,
            service::Activity {
                at: "2026-10-20T15:00:05Z".to_string(),
                kind: "delivery".to_string(),
                channel: "webhook".to_string(),
                peer: Some("the CI box".to_string()),
                peer_id: Some("wh1".to_string()),
                session: Some("20261020T150000Z-abc".to_string()),
                task: Some("check the build".to_string()),
                detail: None,
            },
        );

        let mut app = fresh();
        let generation = app.open_panel(crate::tui::app::PanelKind::Bots);
        app.apply_panel(
            generation,
            crate::tui::app::PanelKind::Bots,
            Ok(crate::tui::app::PanelData::Bots(
                crate::tui::app::BotsSnapshot::read(&home),
            )),
        );
        let service_view = draw(&app, 120, 40);
        // The detail column wraps and the pane's borders ride every line,
        // so the prose is compared as words: punctuation and box glyphs
        // dropped, whitespace collapsed. What is asserted is the sentence
        // the panel says, not where the pane broke it.
        let flat = words(&service_view);
        // The title is one line, so the raw buffer has it whole; the prose
        // is compared as words.
        assert!(
            service_view.contains("bots \u{2014} service"),
            "{service_view}"
        );
        assert!(flat.contains("webhook"), "{service_view}");
        assert!(flat.contains("127 0 0 1 3081"), "{service_view}");
        assert!(
            flat.contains("Start and stop live in the face that hosts the service"),
            "the panel says who holds the port: {service_view}"
        );

        // The peers view names the capability in words.
        app.cycle_panel_filter();
        let peers = words(&draw(&app, 120, 40));
        assert!(peers.contains("the CI box"), "{peers}");
        assert!(peers.contains("may act"), "{peers}");

        // The door names the peer that tried.
        app.cycle_panel_filter();
        let door = words(&draw(&app, 120, 40));
        assert!(door.contains("Alejandro"), "{door}");
        assert!(door.contains("pending"), "{door}");
        assert!(door.contains("attempt"), "{door}");

        // The activity names the delivery and its session.
        app.cycle_panel_filter();
        let activity = words(&draw(&app, 120, 40));
        assert!(activity.contains("delivery"), "{activity}");
        assert!(activity.contains("check the build"), "{activity}");
        assert!(activity.contains("20261020T150000Z abc"), "{activity}");
        std::fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn the_plan_card_offers_the_edit_door() {
        let mut app = fresh();
        app.plan_text = Some("the next increment".to_string());
        let drawn = draw(&app, 120, 40);
        assert!(drawn.contains("plan ready"), "{drawn}");
        assert!(drawn.contains("the next increment"), "{drawn}");
        assert!(
            drawn.contains("ctrl+e loads it into the message box"),
            "{drawn}"
        );
    }
}
