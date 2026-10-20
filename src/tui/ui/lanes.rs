//! The lane health view: one table row per lane, its detail underneath,
//! and blend1's doctor sub-lanes as rows of their own. The table is the
//! right shape here: every lane carries the same columns, and the owner
//! needs to scan them, not read five prose cards.

use ratatui::layout::{Constraint, Flex, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{
    Block, Cell, Clear, Paragraph, Row, Scrollbar, ScrollbarOrientation, ScrollbarState, Table,
    TableState,
};
use ratatui::Frame;

use crate::lanes::LaneState;
use crate::tui::app::App;
use crate::tui::ui::palette;

const MODAL_PERCENT: u16 = 90;
const DOCTOR_LANE: &str = "blend1";

pub(super) fn render(frame: &mut Frame<'_>, app: &App) {
    let [columns] = Layout::horizontal([Constraint::Percentage(MODAL_PERCENT)])
        .flex(Flex::Center)
        .areas(frame.area());
    let rows = rows(app);
    let height = (rows.len() as u16 + 3)
        .clamp(8, 24)
        .min(frame.area().height);
    let area = Rect {
        x: columns.x,
        y: frame.area().y + frame.area().height.saturating_sub(height) / 2,
        width: columns.width,
        height,
    };
    frame.render_widget(Clear, area);
    let block = Block::bordered()
        .title(" lanes \u{2014} esc close ")
        .title_style(Style::new().fg(palette::TEXT).add_modifier(Modifier::BOLD))
        .border_style(Style::new().fg(palette::ACCENT));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.height == 0 || inner.width == 0 {
        return;
    }
    if app.lanes.is_empty() && app.lanes_loading {
        frame.render_widget(
            Paragraph::new(Line::from(ratatui::text::Span::styled(
                " probing the lanes\u{2026}",
                Style::new().fg(palette::DIM),
            ))),
            inner,
        );
        return;
    }
    if rows.is_empty() {
        frame.render_widget(
            Paragraph::new(Line::from(ratatui::text::Span::styled(
                " no lane health has landed",
                Style::new().fg(palette::DIM),
            ))),
            inner,
        );
        return;
    }
    let row_count = rows.len();
    let overflow = row_count > inner.height.saturating_sub(1) as usize;
    let table_area = Rect {
        width: inner.width.saturating_sub(u16::from(overflow)),
        ..inner
    };
    let visible = table_area.height.saturating_sub(1) as usize;
    let selected_row = selected_row(app);
    let offset = selected_row.saturating_sub(visible.saturating_sub(1));
    let header = Row::new(vec![
        Cell::from("lane"),
        Cell::from("role"),
        Cell::from("state"),
        Cell::from("tools"),
        Cell::from("v / ms"),
    ])
    .style(Style::new().fg(palette::DIM).add_modifier(Modifier::BOLD));
    let table = Table::new(
        rows,
        [
            Constraint::Length(16),
            Constraint::Length(12),
            Constraint::Length(13),
            Constraint::Length(7),
            Constraint::Min(16),
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
    .highlight_symbol("\u{25b8} ");
    let mut state = TableState::default()
        .with_offset(offset)
        .with_selected(Some(selected_row));
    frame.render_stateful_widget(table, table_area, &mut state);
    if overflow {
        let bar = Rect {
            x: inner.x + inner.width - 1,
            y: inner.y,
            width: 1,
            height: inner.height,
        };
        let mut scrollbar = ScrollbarState::new(row_count)
            .position(offset)
            .viewport_content_length(visible);
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .style(Style::new().fg(palette::DIM)),
            bar,
            &mut scrollbar,
        );
    }
}

/// Every row the table draws, in registry order: the lane, its detail,
/// and blend1's doctor rows. The selected lane's row index is tracked by
/// `selected_row`, because the table's rows are not one per lane.
fn rows(app: &App) -> Vec<Row<'static>> {
    let mut rows = Vec::new();
    for lane in &app.lanes {
        let (dot, color) = match lane.state {
            LaneState::Ok => ("\u{25cf}", palette::DONE),
            LaneState::Unavailable => ("\u{25cb}", palette::DIM),
            LaneState::Error => ("\u{2715}", palette::DENIED),
        };
        let version = lane
            .server_version
            .as_deref()
            .map(|version| format!("v{version}"))
            .unwrap_or_else(|| "\u{2014}".to_string());
        rows.push(Row::new(vec![
            Cell::from(format!("{dot} {}", lane.name)).style(Style::new().fg(palette::TEXT)),
            Cell::from(lane.role.clone()).style(Style::new().fg(palette::DIM)),
            Cell::from(lane.state.label()).style(Style::new().fg(color)),
            Cell::from(format!(
                "{} tool{}",
                lane.tools.len(),
                if lane.tools.len() == 1 { "" } else { "s" }
            ))
            .style(Style::new().fg(palette::TEXT)),
            Cell::from(format!("{version} \u{b7} {} ms", lane.elapsed_ms))
                .style(Style::new().fg(palette::DIM)),
        ]));
        let detail = if lane.tools.is_empty() {
            lane.detail.clone()
        } else {
            lane.tools.join(" \u{b7} ")
        };
        rows.push(Row::new(vec![Cell::from(format!("   {detail}"))
            .column_span(5)
            .style(Style::new().fg(palette::DIM))]));
        if lane.name == DOCTOR_LANE {
            rows.push(Row::new(vec![Cell::from("   doctor")
                .column_span(5)
                .style(
                    Style::new().fg(palette::DIM).add_modifier(Modifier::BOLD),
                )]));
            if lane.doctor.is_empty() {
                rows.push(Row::new(vec![Cell::from(format!(
                    "   {}",
                    if lane.tools.is_empty() {
                        lane.detail.clone()
                    } else {
                        "the probe's doctor report did not land".to_string()
                    }
                ))
                .column_span(5)
                .style(Style::new().fg(palette::DENIED))]));
            } else {
                for entry in &lane.doctor {
                    let (dot, color) = if entry.available {
                        ("\u{25cf}", palette::DONE)
                    } else {
                        ("\u{25cb}", palette::DENIED)
                    };
                    let note = if entry.available {
                        entry.provider.clone()
                    } else {
                        entry.reason.clone()
                    };
                    let label = if entry.lane == crate::lanes::WEB_LEG {
                        "web leg"
                    } else {
                        entry.lane.as_str()
                    };
                    rows.push(Row::new(vec![Cell::from(format!(
                        "   {dot} {:<10} {note}",
                        label
                    ))
                    .column_span(5)
                    .style(Style::new().fg(color))]));
                }
            }
        }
    }
    rows
}

/// The table row of the selected lane (its first row, not its detail).
fn selected_row(app: &App) -> usize {
    let mut row = 0;
    for (index, lane) in app.lanes.iter().enumerate() {
        if index == app.selected {
            return row;
        }
        row += 2;
        if lane.name == DOCTOR_LANE {
            row += 1 + lane.doctor.len().max(1);
        }
    }
    row
}
