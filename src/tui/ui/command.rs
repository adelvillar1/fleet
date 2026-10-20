//! The command palette: one filtered list of every pane, overlay, and
//! run action. The direct keys remain; this is how the owner finds the
//! surface they do not have a key for.

use ratatui::layout::{Constraint, Flex, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, List, ListItem, ListState, Paragraph};
use ratatui::Frame;

use crate::tui::app::App;
use crate::tui::ui::palette;

pub(super) fn render(frame: &mut Frame<'_>, app: &App) {
    let area = panel(frame.area());
    frame.render_widget(Clear, area);
    let block = Block::bordered()
        .title(" command ")
        .title_style(Style::new().fg(palette::TEXT).add_modifier(Modifier::BOLD))
        .border_style(Style::new().fg(palette::ACCENT));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.height < 3 || inner.width == 0 {
        return;
    }
    let [filter, list_area, hint_area] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .areas(inner);
    let filter_line = Line::from(vec![
        Span::styled(" \u{25b8} ", Style::new().fg(palette::USER)),
        Span::styled(app.palette_query.clone(), Style::new().fg(palette::TEXT)),
        Span::styled("\u{258c}", Style::new().fg(palette::ACCENT)),
    ]);
    frame.render_widget(Paragraph::new(filter_line), filter);

    let items = app.palette_filtered();
    if items.is_empty() {
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                " no command matches",
                Style::new().fg(palette::DIM),
            ))),
            list_area,
        );
    } else {
        let rows: Vec<ListItem> = items
            .iter()
            .map(|item| {
                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!("{:<16}", item.label),
                        Style::new().fg(palette::TEXT),
                    ),
                    Span::styled(item.detail, Style::new().fg(palette::DIM)),
                ]))
            })
            .collect();
        let list = List::new(rows)
            .highlight_style(
                Style::new()
                    .bg(palette::ACCENT)
                    .fg(palette::ON_ACCENT)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("\u{25b8} ");
        let selected = app.palette_selected.min(items.len().saturating_sub(1));
        let visible = list_area.height as usize;
        let offset = selected.saturating_sub(visible.saturating_sub(1));
        let overflow = items.len() > visible;
        let table_area = Rect {
            width: list_area.width.saturating_sub(u16::from(overflow)),
            ..list_area
        };
        let mut state = ListState::default()
            .with_offset(offset)
            .with_selected(Some(selected));
        frame.render_stateful_widget(list, table_area, &mut state);
        super::widgets::scrollbar(frame, list_area, items.len(), offset, visible);
    }
    let hint = Line::from(Span::styled(
        " type to filter \u{b7} \u{2191}\u{2193} move \u{b7} enter pick \u{b7} esc close",
        Style::new().fg(palette::DIM),
    ));
    frame.render_widget(Paragraph::new(hint), hint_area);
}

fn panel(area: Rect) -> Rect {
    let [horizontal] = Layout::horizontal([Constraint::Percentage(62)])
        .flex(Flex::Center)
        .areas(area);
    let height = area.height.saturating_sub(6).clamp(6, 18);
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect {
        x: horizontal.x,
        y,
        width: horizontal.width,
        height,
    }
}
