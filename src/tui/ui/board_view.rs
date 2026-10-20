//! The composed boards: the router's model mix as a real bar chart and
//! the day's served-rate as a labelled gauge, the tower's reports-vs-
//! commits series and project ranking, and the line composers that stay
//! lines because their width is 42 columns.
//!
//! The charts are fed by the snapshots the boards already read. The
//! tower's series comes from the brain slot (`reports.report_daily` and
//! `tower.timeline`), not a second read: the two boards are the same
//! machine seen from two angles.

use std::collections::BTreeMap;

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{
    Axis, Bar, BarChart, Block, Chart, Dataset, Gauge, GraphType, Paragraph, Sparkline,
};
use ratatui::Frame;

use crate::boards::{self, BoardData, BoardKind, BrainSnapshot, RouterSnapshot, TowerSnapshot};
use crate::tui::ui::palette;

/// The router board: the model mix as bars and the day's served rate as
/// a gauge, with the existing tier/served/ledger lines below.
pub(super) fn render_router(frame: &mut Frame<'_>, snapshot: &RouterSnapshot, area: Rect) {
    if area.height == 0 || area.width == 0 {
        return;
    }
    // The day's shape is the widest fact the router has. The model mix
    // is a second chart only when there is more than one model to
    // compare; a single-model "bar chart" is a line with a block on it.
    let chart_h = if area.height >= 20 && !snapshot.hours.is_empty() {
        5
    } else if area.height >= 16 && !snapshot.hours.is_empty() {
        4
    } else {
        0
    };
    let bars_h = if snapshot.models.len() > 1 && area.height >= 26 {
        (snapshot.models.len() as u16).clamp(1, 3)
    } else {
        0
    };
    let gauge_h = u16::from(area.height >= 22);
    if chart_h == 0 && bars_h == 0 {
        render_lines(
            frame,
            super::router_lines(snapshot, area.width as usize),
            area,
        );
        return;
    }
    let [chart_area, bars_area, gauge_area, rest] = Layout::vertical([
        Constraint::Length(chart_h),
        Constraint::Length(bars_h),
        Constraint::Length(gauge_h),
        Constraint::Min(1),
    ])
    .areas(area);
    if chart_h > 0 {
        render_busy_hours(frame, snapshot, chart_area);
    }
    if bars_h > 0 {
        render_model_bars(frame, snapshot, bars_area);
    }
    if gauge_h > 0 {
        render_served_gauge(frame, snapshot, gauge_area);
    }
    render_lines(
        frame,
        super::router_lines(snapshot, rest.width as usize),
        rest,
    );
}

/// The busiest hours of the day as horizontal bars. A 24-column time
/// axis does not survive 42 terminal columns; the busiest hours are the
/// same comparison without pretending the axis is readable.
fn render_busy_hours(frame: &mut Frame<'_>, snapshot: &RouterSnapshot, area: Rect) {
    let mut hours: Vec<&boards::HourColumn> = snapshot.hours.iter().collect();
    hours.sort_by(|left, right| right.calls.cmp(&left.calls));
    let bars: Vec<Bar> = hours
        .into_iter()
        .take(area.height as usize)
        .map(|column| {
            Bar::with_label(format!("{:02}:00", column.hour), column.calls)
                .text_value(boards::grouped(column.calls))
        })
        .collect();
    let max = snapshot
        .hours
        .iter()
        .map(|column| column.calls)
        .max()
        .unwrap_or(1)
        .max(1);
    let chart = BarChart::horizontal(bars)
        .max(max)
        .bar_width(1)
        .bar_gap(0)
        .bar_style(Style::new().fg(palette::ACCENT))
        .value_style(Style::new().fg(palette::TEXT))
        .label_style(Style::new().fg(palette::DIM));
    frame.render_widget(chart, area);
}

/// The day's models as horizontal bars: one row per model, its name on
/// the left and its call count at the end of the row.
fn render_model_bars(frame: &mut Frame<'_>, snapshot: &RouterSnapshot, area: Rect) {
    let bars: Vec<Bar> = snapshot
        .models
        .iter()
        .take(area.height as usize)
        .map(|model| {
            let name = model
                .model
                .rsplit('/')
                .next()
                .unwrap_or(&model.model)
                .to_string();
            Bar::with_label(clip(&name, 14), model.calls).text_value(boards::grouped(model.calls))
        })
        .collect();
    let max = snapshot
        .models
        .iter()
        .map(|model| model.calls)
        .max()
        .unwrap_or(1)
        .max(1);
    let chart = BarChart::horizontal(bars)
        .max(max)
        .bar_width(1)
        .bar_gap(0)
        .bar_style(Style::new().fg(palette::ACCENT))
        .value_style(Style::new().fg(palette::TEXT))
        .label_style(Style::new().fg(palette::DIM));
    frame.render_widget(chart, area);
}

/// The day's served rate as a labelled block bar. It is a `Gauge`, not
/// the status bar's `LineGauge`: the bar is the point of the block, and
/// the count is already in the ledger line beneath it.
fn render_served_gauge(frame: &mut Frame<'_>, snapshot: &RouterSnapshot, area: Rect) {
    let Some(totals) = &snapshot.totals else {
        return;
    };
    let Some(rate) = totals.success_rate() else {
        return;
    };
    let ratio = (rate as f64 / 100.0).clamp(0.0, 1.0);
    let gauge = Gauge::default()
        .ratio(ratio)
        .label(format!("{rate}% served"))
        .gauge_style(Style::new().fg(if rate >= 90 {
            palette::DONE
        } else if rate >= 70 {
            palette::PENDING
        } else {
            palette::DENIED
        }))
        .use_unicode(true);
    frame.render_widget(gauge, area);
}

/// The tower board: the fourteen-day reports-vs-commits series, the
/// project ranking as bars, and the existing project cards.
pub(super) fn render_tower(
    frame: &mut Frame<'_>,
    app: &crate::tui::app::App,
    snapshot: &TowerSnapshot,
    area: Rect,
) {
    if area.height == 0 || area.width == 0 {
        return;
    }
    let brain = match &app.boards.slot(BoardKind::Brain).snapshot {
        Some(BoardData::Brain(brain)) => Some(brain.as_ref()),
        _ => None,
    };
    let series = brain.and_then(tower_series);
    let chart_h = if area.height >= 26 && series.is_some() {
        9
    } else {
        0
    };
    let bars_h = if area.height >= 12 {
        (snapshot.projects.len() as u16).clamp(1, 6)
    } else {
        0
    };
    let [chart_area, bars_area, rest] = Layout::vertical([
        Constraint::Length(chart_h),
        Constraint::Length(bars_h),
        Constraint::Min(1),
    ])
    .areas(area);
    let mut lead: Vec<Line<'static>> = Vec::new();
    if chart_h > 0 {
        if let Some(series) = &series {
            let [report_area, commit_area] =
                Layout::vertical([Constraint::Length(chart_h - 3), Constraint::Length(3)])
                    .areas(chart_area);
            render_series(frame, series, report_area);
            render_commits(frame, series, commit_area);
        }
    } else if area.height >= 26 {
        // The chart is omitted only for a named reason: a brain board
        // that has not landed, or one whose read refused.
        let reason = match &app.boards.slot(BoardKind::Brain).snapshot {
            Some(BoardData::Brain(brain)) => brain
                .reports
                .reason()
                .map(str::to_string)
                .or_else(|| brain.tower.reason().map(str::to_string))
                .unwrap_or_else(|| "the brain board has not landed".to_string()),
            _ => "the brain board has not landed".to_string(),
        };
        lead.push(Line::from(Span::styled(
            format!(" chart \u{b7} {reason}"),
            Style::new().fg(palette::DIM),
        )));
    }
    if bars_h > 0 {
        render_project_bars(frame, snapshot, bars_area);
    }
    let mut lines = lead;
    lines.extend(super::tower_lines(snapshot, rest.width as usize));
    render_lines(frame, lines, rest);
}

/// The commits series at its own scale. A shared axis with the reports
/// would flatten it into the baseline; a sparkline is the readable shape
/// for a series an order of magnitude smaller.
fn render_commits(frame: &mut Frame<'_>, series: &Series, area: Rect) {
    if area.height == 0 {
        return;
    }
    let [label, spark] = Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(area);
    let total: u64 = series.commits.iter().sum();
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            format!(" commits by day \u{b7} {total}"),
            Style::new().fg(palette::DIM),
        ))),
        label,
    );
    let data: Vec<u64> = series.commits.to_vec();
    frame.render_widget(
        Sparkline::default()
            .data(data)
            .style(Style::new().fg(palette::TEXT)),
        spark,
    );
}

/// The project ranking: trailing thirty days of commits, the same sort
/// the tower's cards use, as horizontal bars.
fn render_project_bars(frame: &mut Frame<'_>, snapshot: &TowerSnapshot, area: Rect) {
    let mut ranked: Vec<&boards::ProjectFacts> = snapshot.projects.iter().collect();
    ranked.sort_by(|left, right| right.commits_30d.cmp(&left.commits_30d));
    let top = ranked
        .iter()
        .take(area.height as usize)
        .map(|project| {
            Bar::with_label(clip(&project.name, 16), project.commits_30d)
                .text_value(boards::grouped(project.commits_30d))
        })
        .collect::<Vec<_>>();
    let max = ranked
        .iter()
        .map(|project| project.commits_30d)
        .max()
        .unwrap_or(0)
        .max(1);
    let chart = BarChart::horizontal(top)
        .max(max)
        .bar_width(1)
        .bar_gap(0)
        .bar_style(Style::new().fg(palette::ACCENT))
        .value_style(Style::new().fg(palette::TEXT))
        .label_style(Style::new().fg(palette::DIM));
    frame.render_widget(chart, area);
}

/// One day's two counts, aligned over the same union of days.
struct Series {
    days: Vec<String>,
    reports: Vec<u64>,
    commits: Vec<u64>,
}

/// The brain's two series, zero-filled over the last fourteen days of
/// their union. `None` when neither section landed: an empty chart
/// would read as "no activity", which is a different fact from "no
/// data".
fn tower_series(brain: &BrainSnapshot) -> Option<Series> {
    let reports = brain.reports.landed()?;
    let tower = brain.tower.landed()?;
    let mut report_days: BTreeMap<String, u64> = BTreeMap::new();
    for (day, count) in &reports.report_daily {
        *report_days.entry(day.clone()).or_default() += *count;
    }
    let mut commit_days: BTreeMap<String, u64> = BTreeMap::new();
    for row in &tower.timeline {
        let day = row.date.clone();
        *commit_days.entry(day).or_default() += 1;
    }
    let mut days: Vec<String> = report_days
        .keys()
        .chain(commit_days.keys())
        .cloned()
        .collect();
    days.sort();
    days.dedup();
    let days = days.split_off(days.len().saturating_sub(14));
    if days.is_empty() {
        return None;
    }
    let reports = days
        .iter()
        .map(|day| report_days.get(day).copied().unwrap_or(0))
        .collect();
    let commits = days
        .iter()
        .map(|day| commit_days.get(day).copied().unwrap_or(0))
        .collect();
    Some(Series {
        days,
        reports,
        commits,
    })
}

fn render_series(frame: &mut Frame<'_>, series: &Series, area: Rect) {
    let last = series.days.len().saturating_sub(1) as f64;
    let reports: Vec<(f64, f64)> = series
        .reports
        .iter()
        .enumerate()
        .map(|(index, count)| (index as f64, *count as f64))
        .collect();
    let peak = series.reports.iter().copied().max().unwrap_or(0).max(1) as f64;
    let total: u64 = series.reports.iter().sum();
    let first = series
        .days
        .first()
        .map(|day| day.get(5..).unwrap_or(day).to_string())
        .unwrap_or_default();
    let last_day = series
        .days
        .last()
        .map(|day| day.get(5..).unwrap_or(day).to_string())
        .unwrap_or_default();
    let chart = Chart::new(vec![Dataset::default()
        .name("reports")
        .marker(ratatui::symbols::Marker::Block)
        .graph_type(GraphType::Bar)
        .style(Style::new().fg(palette::ACCENT))
        .data(&reports)])
    .block(
        Block::bordered()
            .title(format!(" reports by day \u{b7} {total} "))
            .title_style(Style::new().fg(palette::DIM))
            .border_style(Style::new().fg(palette::DIM)),
    )
    .x_axis(
        Axis::default()
            .bounds([0.0, last.max(1.0)])
            .labels(vec![first, last_day])
            .style(Style::new().fg(palette::DIM)),
    )
    .y_axis(
        Axis::default()
            .bounds([0.0, peak])
            .labels(vec!["0".to_string(), boards::grouped(peak as u64)])
            .style(Style::new().fg(palette::DIM)),
    )
    .legend_position(None)
    .style(Style::new().fg(palette::DIM));
    frame.render_widget(chart, area);
}

/// Draw a board's line content, naming the tail it had to hold back.
pub(super) fn render_lines(frame: &mut Frame<'_>, mut lines: Vec<Line<'static>>, area: Rect) {
    if area.height == 0 || area.width == 0 {
        return;
    }
    let visible = area.height as usize;
    if lines.len() > visible {
        let hidden = lines.len() - visible + 1;
        lines.truncate(visible.saturating_sub(1));
        lines.push(Line::from(Span::styled(
            format!(" +{hidden} more"),
            Style::new().fg(palette::DIM),
        )));
    }
    frame.render_widget(Paragraph::new(Text::from(lines)), area);
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
