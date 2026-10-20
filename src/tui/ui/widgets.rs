//! Small shared renderers that are not a surface of their own: the
//! scrollbar every long list uses, so a clipped list says it is clipped.

use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::{Scrollbar, ScrollbarOrientation, ScrollbarState};
use ratatui::Frame;

use super::palette;

/// A vertical scrollbar over the right edge of `area`, drawn only when
/// the content is longer than the viewport. `position` is the first
/// visible row; `viewport` is the number of visible rows.
pub(super) fn scrollbar(
    frame: &mut Frame<'_>,
    area: Rect,
    content: usize,
    position: usize,
    viewport: usize,
) {
    if content <= viewport || area.height == 0 || area.width == 0 {
        return;
    }
    let bar = Rect {
        x: area.x + area.width - 1,
        y: area.y,
        width: 1,
        height: area.height,
    };
    let mut state = ScrollbarState::new(content)
        .position(position)
        .viewport_content_length(viewport);
    frame.render_stateful_widget(
        Scrollbar::new(ScrollbarOrientation::VerticalRight).style(Style::new().fg(palette::DIM)),
        bar,
        &mut state,
    );
}
