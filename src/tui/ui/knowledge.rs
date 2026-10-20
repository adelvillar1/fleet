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
    if rows.is_empty() {
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                " nothing matches this filter",
                Style::new().fg(palette::DIM),
            ))),
            list_inner,
        );
        return;
    }
    let header = Row::new(vec![Cell::from("name"), Cell::from("what it says")])
        .style(Style::new().fg(palette::DIM).add_modifier(Modifier::BOLD));
    let body: Vec<Row> = rows
        .iter()
        .map(|row| {
            Row::new(vec![
                Cell::from(row.label.clone()),
                Cell::from(row.meta.clone()).style(Style::new().fg(palette::DIM)),
            ])
        })
        .collect();
    let table = Table::new(body, [Constraint::Min(28), Constraint::Length(24)])
        .header(header)
        .column_spacing(1)
        .row_highlight_style(
            Style::new()
                .bg(palette::ACCENT)
                .fg(palette::ON_ACCENT)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▸ ");
    let overflow = rows.len() > list_inner.height.saturating_sub(1) as usize;
    let table_area = Rect {
        width: list_inner.width.saturating_sub(u16::from(overflow)),
        ..list_inner
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
        Some(PanelData::Delegation(snapshot)) => snapshot.refused.clone(),
        _ => Vec::new(),
    }
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
