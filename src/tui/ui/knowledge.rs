//! The knowledge panels: one list, one detail column. The rows are the
//! state machine's, already filtered; this file only draws them.

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Cell, Clear, Paragraph, Row, Table, TableState, Wrap};
use ratatui::Frame;

use crate::tui::app::{App, PanelData, PanelKind};
use crate::tui::ui::palette;

pub(super) fn render(frame: &mut Frame<'_>, app: &App) {
    let kind = app.panel_kind.unwrap_or(PanelKind::Memory);
    let area = centered(frame.area(), 88, 80);
    frame.render_widget(Clear, area);
    let block = Block::bordered()
        .title(format!(
            " {} — {} · f filter · esc close ",
            kind.label(),
            app.panel_filter_label()
        ))
        .title_style(Style::new().fg(palette::TEXT).add_modifier(Modifier::BOLD))
        .border_style(Style::new().fg(palette::ACCENT));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.width == 0 || inner.height == 0 {
        return;
    }
    if app.panel_loading {
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                " reading the machine\u{2026}",
                Style::new().fg(palette::DIM),
            ))),
            inner,
        );
        return;
    }
    if app.panel.is_none() {
        let said = app
            .panel_note
            .as_deref()
            .map(str::to_string)
            .unwrap_or_else(|| " no read landed".to_string());
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                said,
                Style::new().fg(if app.panel_note.is_some() {
                    palette::PENDING
                } else {
                    palette::DIM
                }),
            ))),
            inner,
        );
        return;
    }
    // The board and the second brain are card stacks, not list+detail:
    // three columns of work, and one document's sections. They branch
    // before the table so a list row never stands in for a card.
    match app.panel.as_ref() {
        Some(PanelData::Board(board)) => {
            render_board(frame, board, inner);
            return;
        }
        Some(PanelData::SecondBrain(brain)) => {
            render_second_brain(frame, brain, inner);
            return;
        }
        _ => {}
    }
    let rows = app.panel_rows();
    let refused = refused(app);
    let [list_area, detail_area] = if inner.width >= 100 {
        let [left, right] =
            Layout::horizontal([Constraint::Percentage(62), Constraint::Percentage(38)])
                .areas(inner);
        [left, right]
    } else {
        let [top, bottom] =
            Layout::vertical([Constraint::Percentage(55), Constraint::Percentage(45)]).areas(inner);
        [top, bottom]
    };
    let list_block = Block::bordered()
        .title(" list ")
        .title_style(Style::new().fg(palette::DIM))
        .border_style(Style::new().fg(palette::DIM));
    let list_inner = list_block.inner(list_area);
    frame.render_widget(list_block, list_area);
    let detail_block = Block::bordered()
        .title(" detail ")
        .title_style(Style::new().fg(palette::DIM))
        .border_style(Style::new().fg(palette::DIM));
    let detail_inner = detail_block.inner(detail_area);
    frame.render_widget(detail_block, detail_area);
    render_detail(frame, app, &rows, &refused, detail_inner);
    // The delegation panel's graph tile: the two divergence counts and
    // the plan's declared workload, above the list its rows join.
    let graph = match app.panel.as_ref() {
        Some(PanelData::Delegation(panel)) => panel.graph.as_ref(),
        _ => None,
    };
    let [tile_area, table_area] = Layout::vertical([
        Constraint::Length(if graph.is_some() { 2 } else { 0 }),
        Constraint::Min(3),
    ])
    .areas(list_inner);
    if let Some(graph) = graph {
        let tile = vec![
            Line::from(vec![
                Span::styled(
                    " divergences ",
                    Style::new()
                        .fg(palette::ACCENT)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!(
                        "never spawned {} \u{b7} ran twice {} \u{b7} {}",
                        graph.divergences.never_spawned,
                        graph.divergences.ran_twice,
                        graph.divergences.basis()
                    ),
                    Style::new().fg(palette::TEXT),
                ),
            ]),
            Line::from(Span::styled(
                match &graph.declared_workload {
                    Some(workload) => format!(" declared workload \u{b7} {workload}"),
                    None => " declared workload \u{b7} no plan declares this graph's node set"
                        .to_string(),
                },
                Style::new().fg(palette::DIM),
            )),
        ];
        frame.render_widget(Paragraph::new(Text::from(tile)), tile_area);
    }
    if rows.is_empty() {
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                " nothing matches this filter",
                Style::new().fg(palette::DIM),
            ))),
            table_area,
        );
        return;
    }
    // The graph's third column exists only when the document joined: the
    // divergence word is the one thing the trees cannot say, and a
    // header over an empty column would claim a join that did not land.
    let has_state = graph.is_some();
    let header = if has_state {
        Row::new(vec![
            Cell::from("name"),
            Cell::from("divergence"),
            Cell::from("what it says"),
        ])
    } else {
        Row::new(vec![Cell::from("name"), Cell::from("what it says")])
    }
    .style(Style::new().fg(palette::DIM).add_modifier(Modifier::BOLD));
    let body: Vec<Row> = rows
        .iter()
        .map(|row| {
            let mut cells = vec![Cell::from(row.label.clone())];
            if has_state {
                let word = row.state.clone().unwrap_or_default();
                let shown = crate::cards::clip(&word, 15);
                cells.push(Cell::from(shown).style(Style::new().fg(if word.is_empty() {
                    palette::DIM
                } else {
                    palette::PENDING
                })));
            }
            cells.push(Cell::from(row.meta.clone()).style(Style::new().fg(palette::DIM)));
            Row::new(cells)
        })
        .collect();
    let widths: Vec<Constraint> = if has_state {
        vec![
            Constraint::Min(18),
            Constraint::Length(16),
            Constraint::Length(20),
        ]
    } else {
        vec![Constraint::Min(28), Constraint::Length(24)]
    };
    let table = Table::new(body, widths)
        .header(header)
        .column_spacing(1)
        .row_highlight_style(
            Style::new()
                .bg(palette::ACCENT)
                .fg(palette::ON_ACCENT)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("\u{25b8} ");
    let overflow = rows.len() > table_area.height.saturating_sub(1) as usize;
    let table_area = Rect {
        width: table_area.width.saturating_sub(u16::from(overflow)),
        ..table_area
    };
    let mut state =
        TableState::default().with_selected(Some(app.selected.min(rows.len().saturating_sub(1))));
    frame.render_stateful_widget(table, table_area, &mut state);
    super::widgets::scrollbar(
        frame,
        list_inner,
        rows.len(),
        state.offset(),
        table_area.height.saturating_sub(1) as usize,
    );
}

fn render_detail(
    frame: &mut Frame<'_>,
    app: &App,
    rows: &[crate::tui::app::PanelRow],
    refused: &[String],
    area: Rect,
) {
    let Some(row) = rows.get(app.selected.min(rows.len().saturating_sub(1))) else {
        return;
    };
    let mut lines = Vec::new();
    for reason in refused {
        lines.push(Line::from(Span::styled(
            reason.clone(),
            Style::new().fg(palette::PENDING),
        )));
    }
    lines.push(Line::from(vec![
        Span::styled(
            row.label.clone(),
            Style::new().fg(palette::TEXT).add_modifier(Modifier::BOLD),
        ),
        Span::styled("  ", Style::new()),
        Span::styled(row.meta.clone(), Style::new().fg(palette::DIM)),
    ]));
    for text in row.detail.lines() {
        if text.trim().is_empty() {
            lines.push(Line::from(""));
        } else {
            lines.push(Line::from(Span::styled(
                text.to_string(),
                Style::new().fg(palette::TEXT),
            )));
        }
    }
    frame.render_widget(
        Paragraph::new(Text::from(lines)).wrap(Wrap { trim: false }),
        area,
    );
}

fn refused(app: &App) -> Vec<String> {
    match app.panel.as_ref() {
        Some(PanelData::Memory(snapshot)) => snapshot.refused.clone(),
        Some(PanelData::Skills(snapshot)) => snapshot.refused.clone(),
        Some(PanelData::Delegation(panel)) => panel.snapshot.refused.clone(),
        Some(PanelData::Board(board)) => board.refused.clone(),
        Some(PanelData::SecondBrain(brain)) => brain.refused.clone(),
        _ => Vec::new(),
    }
}

/// How many cards one column draws before it says `+N more`. The panel
/// scrolls nothing: a column is a stack, and a stack that runs off the
/// pane names what it did not draw.
const BOARD_CARD_CAP: usize = 6;

/// The work board: three derived columns as card stacks in the house
/// card frame. The strip is the columns' own counts — the same numbers
/// the stacks drew — and an empty board says so rather than dressing a
/// zero as a column.
fn render_board(frame: &mut Frame<'_>, board: &crate::tui::app::WorkBoard, area: Rect) {
    let width = (area.width as usize).min(crate::cards::CARD_WIDTH);
    let text = width.saturating_sub(4).max(8);
    let mut lines: Vec<Line> = Vec::new();
    let total: usize = board.columns.iter().map(|column| column.count()).sum();
    if total == 0 {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            " no plan carries work right now",
            Style::new().fg(palette::TEXT).add_modifier(Modifier::BOLD),
        )));
        for reason in &board.refused {
            for piece in crate::cards::fit(reason, text) {
                lines.push(Line::from(Span::styled(
                    format!(" {piece}"),
                    Style::new().fg(palette::PENDING),
                )));
            }
        }
        if board.refused.is_empty() {
            lines.push(Line::from(Span::styled(
                " the read answered with nothing \u{2014} no approved plan and no ad-hoc task",
                Style::new().fg(palette::DIM),
            )));
        }
        frame.render_widget(Paragraph::new(Text::from(lines)), area);
        return;
    }
    let strip = board
        .columns
        .iter()
        .map(|column| format!("{} {}", column.label, column.count()))
        .collect::<Vec<_>>()
        .join(" \u{b7} ");
    lines.push(Line::from(vec![
        Span::styled(
            " work board ",
            Style::new()
                .fg(palette::ACCENT)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(strip, Style::new().fg(palette::TEXT)),
        Span::styled(
            format!(" \u{b7} read {}", board.read_at),
            Style::new().fg(palette::DIM),
        ),
    ]));
    for column in &board.columns {
        // A column with no cards is not drawn: the strip already carries
        // its zero, and a header over nothing reads as a broken read.
        if column.cards.is_empty() {
            continue;
        }
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled(
                format!(" {} ", column.label),
                Style::new()
                    .fg(palette::ACCENT)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{} task(s)", column.count()),
                Style::new().fg(palette::DIM),
            ),
        ]));
        lines.push(Line::from(Span::styled(
            format!(" {}", crate::cards::rule(text)),
            Style::new().fg(palette::DIM),
        )));
        for (index, card) in column.cards.iter().take(BOARD_CARD_CAP).enumerate() {
            if index > 0 {
                lines.push(Line::from(Span::styled(
                    format!(" {}", crate::cards::rule(text)),
                    Style::new().fg(palette::DIM),
                )));
            }
            for piece in crate::cards::fit(&card.plan, text) {
                lines.push(Line::from(Span::styled(
                    format!(" {piece}"),
                    Style::new().fg(palette::TEXT).add_modifier(Modifier::BOLD),
                )));
            }
            let state = crate::cards::clip(&card.state, text.saturating_sub(12));
            lines.push(Line::from(vec![
                Span::styled(
                    format!(" {} task(s)", card.count),
                    Style::new().fg(palette::DIM),
                ),
                Span::styled(format!(" \u{b7} {state}"), Style::new().fg(palette::DIM)),
            ]));
        }
        if column.cards.len() > BOARD_CARD_CAP {
            lines.push(Line::from(Span::styled(
                format!(
                    " +{} more card(s)",
                    column.cards.len().saturating_sub(BOARD_CARD_CAP)
                ),
                Style::new().fg(palette::DIM),
            )));
        }
        lines.push(Line::from(Span::styled(
            format!(" {}", crate::cards::rule(text)),
            Style::new().fg(palette::DIM),
        )));
    }
    for reason in &board.refused {
        for piece in crate::cards::fit(reason, text) {
            lines.push(Line::from(Span::styled(
                format!(" {piece}"),
                Style::new().fg(palette::PENDING),
            )));
        }
    }
    frame.render_widget(Paragraph::new(Text::from(lines)), area);
}

/// The second brain: the morning read's five sentences, then the six
/// derived state sections — each landed with its counts and rows, or
/// refused by name. A section that did not land never prints a zero.
fn render_second_brain(
    frame: &mut Frame<'_>,
    brain: &crate::tui::app::SecondBrainPanel,
    area: Rect,
) {
    let width = (area.width as usize).min(crate::cards::CARD_WIDTH);
    let text = width.saturating_sub(4).max(8);
    let mut lines: Vec<Line> = vec![Line::from(vec![
        Span::styled(
            " second-brain ",
            Style::new()
                .fg(palette::ACCENT)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(brain.home.clone(), Style::new().fg(palette::TEXT)),
        Span::styled(
            format!(" \u{b7} read {}", brain.read_at),
            Style::new().fg(palette::DIM),
        ),
    ])];
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        " morning read",
        Style::new()
            .fg(palette::ACCENT)
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(Span::styled(
        format!(" {}", crate::cards::rule(text)),
        Style::new().fg(palette::DIM),
    )));
    for sentence in &brain.morning {
        for (index, piece) in crate::cards::fit(sentence, text.saturating_sub(3))
            .into_iter()
            .enumerate()
        {
            if index == 0 {
                lines.push(Line::from(vec![
                    Span::styled(" \u{25cf} ", Style::new().fg(palette::DONE)),
                    Span::styled(piece, Style::new().fg(palette::TEXT)),
                ]));
            } else {
                lines.push(Line::from(Span::styled(
                    format!("   {piece}"),
                    Style::new().fg(palette::TEXT),
                )));
            }
        }
    }
    for section in &brain.sections {
        lines.push(Line::from(""));
        match (&section.landed, &section.refused) {
            (Some(counts), _) => {
                lines.push(Line::from(vec![
                    Span::styled(
                        format!(" {}", section.name),
                        Style::new()
                            .fg(palette::ACCENT)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(format!(" \u{b7} {counts}"), Style::new().fg(palette::DIM)),
                ]));
                lines.push(Line::from(Span::styled(
                    format!(" {}", crate::cards::rule(text)),
                    Style::new().fg(palette::DIM),
                )));
                for row in &section.rows {
                    for (index, piece) in crate::cards::fit(row, text.saturating_sub(3))
                        .into_iter()
                        .enumerate()
                    {
                        let prefix = if index == 0 { " \u{b7} " } else { "   " };
                        lines.push(Line::from(Span::styled(
                            format!("{prefix}{piece}"),
                            Style::new().fg(palette::TEXT),
                        )));
                    }
                }
            }
            (None, Some(reason)) => {
                lines.push(Line::from(vec![
                    Span::styled(
                        format!(" {}", section.name),
                        Style::new()
                            .fg(palette::ACCENT)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(" \u{b7} did not land", Style::new().fg(palette::PENDING)),
                ]));
                for piece in crate::cards::fit(reason, text.saturating_sub(3)) {
                    lines.push(Line::from(Span::styled(
                        format!(" {piece}"),
                        Style::new().fg(palette::PENDING),
                    )));
                }
            }
            (None, None) => {
                lines.push(Line::from(Span::styled(
                    format!(" {} \u{b7} the read said nothing", section.name),
                    Style::new().fg(palette::DIM),
                )));
            }
        }
    }
    if brain.sections.is_empty() {
        for reason in &brain.refused {
            for piece in crate::cards::fit(reason, text) {
                lines.push(Line::from(Span::styled(
                    format!(" {piece}"),
                    Style::new().fg(palette::PENDING),
                )));
            }
        }
    }
    frame.render_widget(Paragraph::new(Text::from(lines)), area);
}

fn centered(area: Rect, percent: u16, height: u16) -> Rect {
    let width = (area.width.saturating_mul(percent) / 100)
        .max(50)
        .min(area.width);
    let height = (area.height.saturating_mul(height) / 100)
        .max(8)
        .min(area.height);
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect {
        x,
        y,
        width,
        height,
    }
}
