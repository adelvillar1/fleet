//! The checkpoint list, the diff review, and the one confirmation that
//! stands before a write. The tree and the log are the owner's; every
//! write here asks first and names exactly what it puts back.

use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Cell, Clear, Paragraph, Row, Table, TableState, Wrap};
use ratatui::Frame;

use crate::tui::app::App;
use crate::tui::ui::palette;

pub(super) fn render_checkpoints(frame: &mut Frame<'_>, app: &App) {
    let area = centered(frame.area(), 72, 60);
    frame.render_widget(Clear, area);
    let block = Block::bordered()
        .title(" checkpoints — enter rewinds, d opens the diff, esc closes ")
        .title_style(Style::new().fg(palette::TEXT).add_modifier(Modifier::BOLD))
        .border_style(Style::new().fg(palette::ACCENT));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.height == 0 || inner.width == 0 {
        return;
    }
    if app.checkpoints_view.is_empty() {
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                " no checkpoint has landed for this session yet",
                Style::new().fg(palette::DIM),
            ))),
            inner,
        );
        return;
    }
    let header = Row::new(vec![
        Cell::from("turn"),
        Cell::from("files"),
        Cell::from("bytes"),
        Cell::from("captured"),
    ])
    .style(Style::new().fg(palette::DIM).add_modifier(Modifier::BOLD));
    let rows: Vec<Row> = app
        .checkpoints_view
        .iter()
        .map(|snapshot| {
            Row::new(vec![
                Cell::from(format!("{}", snapshot.turn)),
                Cell::from(format!("{}", snapshot.files)),
                Cell::from(crate::boards::grouped(snapshot.bytes)),
                Cell::from(snapshot.at.clone()),
            ])
        })
        .collect();
    let table = Table::new(
        rows,
        [
            Constraint::Length(7),
            Constraint::Length(8),
            Constraint::Length(14),
            Constraint::Min(20),
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
    let mut state = TableState::default().with_selected(Some(
        app.selected
            .min(app.checkpoints_view.len().saturating_sub(1)),
    ));
    frame.render_stateful_widget(table, inner, &mut state);
}

pub(super) fn render_diff(frame: &mut Frame<'_>, app: &App) {
    let area = centered(frame.area(), 88, 82);
    frame.render_widget(Clear, area);
    let block = Block::bordered()
        .title(format!(
            " diff — turn {} — r rejects the hunk, R the file, tab moves hunks, esc closes ",
            app.diff_turn
        ))
        .title_style(Style::new().fg(palette::TEXT).add_modifier(Modifier::BOLD))
        .border_style(Style::new().fg(palette::ACCENT));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.height == 0 || inner.width == 0 {
        return;
    }
    if app.diff_view.is_empty() {
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                " the tree matches this checkpoint — nothing to review",
                Style::new().fg(palette::DIM),
            ))),
            inner,
        );
        return;
    }
    let width = inner.width as usize;
    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut selected_at: Option<usize> = None;
    for (file_index, file) in app.diff_view.iter().enumerate() {
        let selected_file = file_index == app.diff_file;
        lines.push(Line::from(vec![
            Span::styled(
                if selected_file { " ▸ " } else { "   " },
                Style::new().fg(palette::ACCENT),
            ),
            Span::styled(
                clip(&file.path, width.saturating_sub(16)),
                Style::new()
                    .fg(if selected_file {
                        palette::TEXT
                    } else {
                        palette::DIM
                    })
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("  {}", kind_label(file.kind)),
                Style::new().fg(palette::DIM),
            ),
        ]));
        if file.too_large {
            lines.push(Line::from(Span::styled(
                "   too large to align line by line; R puts the whole file back",
                Style::new().fg(palette::PENDING),
            )));
            continue;
        }
        for (hunk_index, hunk) in file.hunks.iter().enumerate() {
            let selected_hunk = selected_file && hunk_index == app.diff_hunk;
            if selected_hunk {
                selected_at = Some(lines.len());
            }
            lines.push(Line::from(vec![
                Span::styled(
                    if selected_hunk {
                        "   \u{25b8} "
                    } else {
                        "     "
                    },
                    Style::new().fg(palette::ACCENT),
                ),
                Span::styled(
                    format!("@@ +{} @@", hunk.new_at + 1),
                    Style::new().fg(palette::DIM),
                ),
            ]));
            for old in &hunk.old {
                lines.push(Line::from(vec![
                    Span::styled("     - ", Style::new().fg(palette::DENIED)),
                    Span::styled(
                        clip(old, width.saturating_sub(8)),
                        Style::new().fg(palette::DENIED),
                    ),
                ]));
            }
            for new in &hunk.new {
                lines.push(Line::from(vec![
                    Span::styled("     + ", Style::new().fg(palette::DONE)),
                    Span::styled(
                        clip(new, width.saturating_sub(8)),
                        Style::new().fg(palette::DONE),
                    ),
                ]));
            }
        }
    }
    let visible = inner.height as usize;
    let offset = selected_at
        .map(|at| at.saturating_sub(visible.saturating_sub(1)))
        .unwrap_or(0);
    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .wrap(Wrap { trim: false })
            .scroll((offset.min(u16::MAX as usize) as u16, 0)),
        inner,
    );
}

pub(super) fn render_confirm(frame: &mut Frame<'_>, app: &App) {
    let Some(confirm) = &app.confirm else {
        return;
    };
    let area = centered(frame.area(), 58, 28);
    frame.render_widget(Clear, area);
    let block = Block::bordered()
        .title(format!(" {} ", confirm.title))
        .title_style(Style::new().fg(palette::TEXT).add_modifier(Modifier::BOLD))
        .border_style(Style::new().fg(palette::PENDING));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.height == 0 || inner.width == 0 {
        return;
    }
    let lines = vec![
        Line::from(Span::styled(
            confirm.detail.clone(),
            Style::new().fg(palette::TEXT),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled(" [y] ", Style::new().fg(palette::ACCENT)),
            Span::styled("land it", Style::new().fg(palette::DIM)),
            Span::styled("   [esc] ", Style::new().fg(palette::ACCENT)),
            Span::styled("walk away", Style::new().fg(palette::DIM)),
        ]),
    ];
    frame.render_widget(
        Paragraph::new(Text::from(lines)).wrap(Wrap { trim: false }),
        inner,
    );
}

fn kind_label(kind: crate::checkpoints::DiffKind) -> &'static str {
    match kind {
        crate::checkpoints::DiffKind::Added => "added",
        crate::checkpoints::DiffKind::Removed => "removed",
        crate::checkpoints::DiffKind::Modified => "modified",
    }
}

fn centered(area: Rect, percent: u16, height: u16) -> Rect {
    let width = (area.width.saturating_mul(percent) / 100)
        .max(40)
        .min(area.width);
    let height = (area.height.saturating_mul(height) / 100)
        .max(6)
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

fn clip(text: &str, width: usize) -> String {
    use unicode_width::UnicodeWidthStr;
    let width = width.max(4);
    if text.width() <= width {
        return text.to_string();
    }
    let mut out = String::new();
    let mut used = 0;
    for character in text.chars() {
        let w = unicode_width::UnicodeWidthChar::width(character).unwrap_or(0);
        if used + w >= width {
            break;
        }
        out.push(character);
        used += w;
    }
    out.push('\u{2026}');
    out
}
