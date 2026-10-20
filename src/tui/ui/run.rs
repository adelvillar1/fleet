//! The run card, its controls, and the compact strip.
//!
//! The browser face has the same surface as a tall card; here the pane
//! is narrow and the owner is keyboard-first, so the card is a vertical
//! stepper with the branches under it and the two controls on the last
//! line. Everything it draws is reduced in `app.rs` from the same event
//! log the conversation reads.

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Clear, Paragraph, Sparkline, Wrap};
use ratatui::Frame;
use unicode_width::UnicodeWidthStr;

use crate::tui::app::{App, Branch, Stage};
use crate::tui::ui::palette;

/// The run pane, below the tab bar. Its own border so the tab row above
/// it stays borderless and one row tall.
pub(super) fn render(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let block = Block::bordered()
        .title(" the run ")
        .title_style(Style::new().fg(palette::TEXT).add_modifier(Modifier::BOLD))
        .border_style(Style::new().fg(palette::ACCENT));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.height == 0 || inner.width == 0 {
        return;
    }
    let spark_h = if app.run.tokens.is_empty() { 0 } else { 2 };
    let [body, spark, controls] = Layout::vertical([
        Constraint::Min(1),
        Constraint::Length(spark_h),
        Constraint::Length(1),
    ])
    .areas(inner);
    let mut lines = head_lines(app, inner.width as usize);
    lines.extend(stage_lines(app));
    lines.extend(branch_lines(app, inner.width as usize));
    frame.render_widget(
        Paragraph::new(Text::from(lines)).wrap(Wrap { trim: false }),
        body,
    );
    if spark_h > 0 {
        let data: Vec<u64> = app.run.tokens.to_vec();
        let sparkline = Sparkline::default()
            .data(data)
            .style(Style::new().fg(palette::ACCENT));
        frame.render_widget(sparkline, spark);
    }
    frame.render_widget(Paragraph::new(control_line(app)), controls);
}

fn head_lines(app: &App, width: usize) -> Vec<Line<'static>> {
    let run = &app.run;
    let state = if run.stopping {
        "stopping"
    } else if run.active {
        "in flight"
    } else if let Some(reason) = &run.stop_reason {
        reason.as_str()
    } else {
        "idle"
    };
    let color = if run.active {
        palette::WORKING
    } else if run.stop_reason.as_deref() == Some("failed") {
        palette::DENIED
    } else {
        palette::IDLE
    };
    let elapsed = match (run.started, run.ended) {
        (Some(start), Some(end)) => format_elapsed(end.duration_since(start)),
        (Some(start), None) if run.active => format_elapsed(start.elapsed()),
        _ => "\u{2014}".to_string(),
    };
    let mut lines = vec![Line::from(vec![
        Span::styled(
            format!(" {state}"),
            Style::new().fg(color).add_modifier(Modifier::BOLD),
        ),
        Span::styled("  ", Style::new()),
        Span::styled(elapsed, Style::new().fg(palette::DIM)),
    ])];
    if app.plan_text.is_some() {
        lines.push(Line::from(Span::styled(
            " plan ready \u{b7} ctrl+e edits",
            Style::new().fg(palette::APPROVED),
        )));
    } else if run.plan {
        lines.push(Line::from(Span::styled(
            if run.active {
                " plan mode \u{b7} the run writes nothing"
            } else {
                " next run plans"
            },
            Style::new().fg(palette::DIM),
        )));
    }
    if run.started.is_none() {
        lines.push(Line::from(Span::styled(
            " ask for something; the loop lands here",
            Style::new().fg(palette::DIM),
        )));
        return lines;
    }
    if !run.model.is_empty() {
        lines.push(Line::from(Span::styled(
            clip(&format!(" {} \u{b7} {}", run.provider, run.model), width),
            Style::new().fg(palette::DIM),
        )));
    }
    if !run.last.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled(" last ", Style::new().fg(palette::DIM)),
            Span::styled(
                clip(&run.last, width.saturating_sub(6)),
                Style::new().fg(palette::TEXT),
            ),
        ]));
    }
    let mut meta: Vec<String> = Vec::new();
    if run.turn > 0 {
        meta.push(format!("t{}", run.turn));
    }
    if run.calls > 0 {
        meta.push(format!(
            "{} call{}",
            run.calls,
            if run.calls == 1 { "" } else { "s" }
        ));
    }
    if run.facts > 0 {
        meta.push(format!("{} facts", run.facts));
    }
    if !run.branches.is_empty() {
        meta.push(format!("{} deleg", run.branches.len()));
    }
    if run.lanes > 0 {
        meta.push(format!("{} lanes", run.lanes));
    }
    if let Some(turn) = run.checkpoint {
        meta.push(format!("cp {turn}"));
    }
    if !meta.is_empty() {
        lines.push(Line::from(Span::styled(
            format!(" {}", meta.join(" \u{b7} ")),
            Style::new().fg(palette::DIM),
        )));
    }
    lines
}

fn stage_lines(app: &App) -> Vec<Line<'static>> {
    let run = &app.run;
    let at = run.stage.index();
    let failed = run.stop_reason.as_deref() == Some("failed");
    Stage::ALL
        .iter()
        .enumerate()
        .map(|(index, stage)| {
            let (marker, color) = if index == at && run.active {
                ("\u{25c9}", palette::ACCENT)
            } else if index == at && failed {
                ("\u{25cf}", palette::DENIED)
            } else if index < at || (!run.active && run.started.is_some()) {
                ("\u{25cf}", palette::DONE)
            } else {
                ("\u{25cb}", palette::DIM)
            };
            Line::from(vec![
                Span::styled(format!(" {marker} "), Style::new().fg(color)),
                Span::styled(
                    format!("{:<7}", stage.label()),
                    Style::new().fg(if index == at {
                        palette::TEXT
                    } else {
                        palette::DIM
                    }),
                ),
                Span::styled(stage_note(app, *stage), Style::new().fg(palette::DIM)),
            ])
        })
        .collect()
}

fn stage_note(app: &App, stage: Stage) -> String {
    let run = &app.run;
    match stage {
        Stage::Route => {
            if run.provider.is_empty() {
                "waiting for the decision".to_string()
            } else {
                let mut said = run.provider.clone();
                if !run.workload.is_empty() {
                    said.push_str(&format!(" \u{b7} {}", run.workload));
                }
                said
            }
        }
        Stage::Think => {
            if run.turn > 0 {
                format!("turn {}", run.turn)
            } else {
                "the model's turn".to_string()
            }
        }
        Stage::Act => {
            if !run.tool.is_empty() {
                format!(
                    "{} \u{b7} {} call(s)",
                    clip(&run.tool, 20),
                    run.calls.max(1)
                )
            } else if run.calls > 0 {
                format!("{} call(s)", run.calls)
            } else if run.active {
                "no tool yet".to_string()
            } else {
                "no tool this run".to_string()
            }
        }
        Stage::Verify => match run.gate {
            Some(true) => "gate on \u{b7} sys1".to_string(),
            Some(false) => "gate shut".to_string(),
            None => "gate unknown".to_string(),
        },
    }
}

fn branch_lines(app: &App, width: usize) -> Vec<Line<'static>> {
    if app.run.branches.is_empty() {
        return Vec::new();
    }
    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!(" branches {}", app.run.branches.len()),
            Style::new().fg(palette::DIM).add_modifier(Modifier::BOLD),
        )),
    ];
    let shown = app.run.branches.len().min(3);
    for branch in app.run.branches.iter().rev().take(shown).rev() {
        lines.push(branch_line(branch, width));
    }
    if app.run.branches.len() > shown {
        lines.push(Line::from(Span::styled(
            format!(" +{} more", app.run.branches.len() - shown),
            Style::new().fg(palette::DIM),
        )));
    }
    lines
}

fn branch_line(branch: &Branch, width: usize) -> Line<'static> {
    let (dot, color) = match branch.stage.as_str() {
        "done" if branch.outcome == "completed" => ("\u{25cf}", palette::DONE),
        "done" => ("\u{25cf}", palette::DENIED),
        "working" => ("\u{25c9}", palette::ACCENT),
        _ => ("\u{25cb}", palette::DIM),
    };
    let note = if branch.stage == "working" {
        format!(
            "{} call(s){}",
            branch.calls,
            if branch.tool.is_empty() {
                String::new()
            } else {
                format!(" \u{b7} {}", branch.tool)
            }
        )
    } else if branch.stage == "done" {
        format!("{} \u{b7} {} turn(s)", branch.outcome, branch.turns)
    } else {
        clip(&branch.goal, 32)
    };
    let id = short_session(&branch.session);
    Line::from(vec![
        Span::styled(format!(" {dot} "), Style::new().fg(color)),
        Span::styled(id, Style::new().fg(palette::TEXT)),
        Span::styled(
            format!("  {}", clip(&note, width.saturating_sub(14))),
            Style::new().fg(palette::DIM),
        ),
    ])
}

fn control_line(app: &App) -> Line<'static> {
    if app.run.stopping {
        return Line::from(Span::styled(
            " stopping \u{b7} the run ends at its next step",
            Style::new().fg(palette::PENDING),
        ));
    }
    if app.run.active {
        return Line::from(vec![
            Span::styled(" [s] ", Style::new().fg(palette::ACCENT)),
            Span::styled("stop", Style::new().fg(palette::DIM)),
            Span::styled("  [i] ", Style::new().fg(palette::ACCENT)),
            Span::styled("steer", Style::new().fg(palette::DIM)),
        ]);
    }
    if app.plan_text.is_some() {
        return Line::from(vec![
            Span::styled(" [ctrl-e] ", Style::new().fg(palette::ACCENT)),
            Span::styled("edit the plan", Style::new().fg(palette::DIM)),
        ]);
    }
    Line::from(Span::styled(
        " [ctrl-r] run controls",
        Style::new().fg(palette::DIM),
    ))
}

/// The run controls overlay: the same state as the pane, with a branch
/// cursor and the two instructions.
pub(super) fn render_controls(frame: &mut Frame<'_>, app: &App) {
    let area = centered(frame.area(), 64, 14);
    frame.render_widget(Clear, area);
    let block = Block::bordered()
        .title(" run controls ")
        .title_style(Style::new().fg(palette::TEXT).add_modifier(Modifier::BOLD))
        .border_style(Style::new().fg(palette::ACCENT));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.height == 0 || inner.width == 0 {
        return;
    }
    let mut lines = vec![
        Line::from(vec![
            Span::styled(" state  ", Style::new().fg(palette::DIM)),
            Span::styled(
                if app.run.active { "in flight" } else { "idle" },
                Style::new().fg(if app.run.active {
                    palette::WORKING
                } else {
                    palette::IDLE
                }),
            ),
        ]),
        Line::from(vec![
            Span::styled(" gate   ", Style::new().fg(palette::DIM)),
            Span::styled(
                match app.run.gate {
                    Some(true) => "on \u{b7} sys1",
                    Some(false) => "shut",
                    None => "unknown",
                },
                Style::new().fg(palette::TEXT),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            " branches",
            Style::new().fg(palette::DIM).add_modifier(Modifier::BOLD),
        )),
    ];
    if app.run.branches.is_empty() {
        lines.push(Line::from(Span::styled(
            "  no child has been spawned",
            Style::new().fg(palette::DIM),
        )));
    } else {
        for (index, branch) in app.run.branches.iter().enumerate() {
            let selected = index == app.selected;
            lines.push(Line::from(vec![
                Span::styled(
                    if selected { " \u{25b8} " } else { "   " },
                    Style::new().fg(palette::ACCENT),
                ),
                Span::styled(
                    short_session(&branch.session),
                    Style::new().fg(palette::TEXT),
                ),
                Span::styled(
                    format!("  {} \u{b7} {}", branch.stage, branch.goal),
                    Style::new().fg(if selected {
                        palette::TEXT
                    } else {
                        palette::DIM
                    }),
                ),
            ]));
        }
    }
    lines.push(Line::from(""));
    if app.run.active {
        lines.push(Line::from(vec![
            Span::styled(" [s] ", Style::new().fg(palette::ACCENT)),
            Span::styled("stop", Style::new().fg(palette::DIM)),
            Span::styled("  [i] ", Style::new().fg(palette::ACCENT)),
            Span::styled("steer", Style::new().fg(palette::DIM)),
            Span::styled("  [esc] ", Style::new().fg(palette::ACCENT)),
            Span::styled("close", Style::new().fg(palette::DIM)),
        ]));
    } else {
        lines.push(Line::from(Span::styled(
            " no run is in flight \u{b7} ask for something first",
            Style::new().fg(palette::DIM),
        )));
        lines.push(Line::from(vec![
            Span::styled(" [esc] ", Style::new().fg(palette::ACCENT)),
            Span::styled("close", Style::new().fg(palette::DIM)),
        ]));
    }
    frame.render_widget(
        Paragraph::new(Text::from(lines)).wrap(Wrap { trim: false }),
        inner,
    );
}

/// The one-line steer editor.
pub(super) fn render_steer(frame: &mut Frame<'_>, app: &App) {
    let area = centered(frame.area(), 60, 5);
    frame.render_widget(Clear, area);
    let block = Block::bordered()
        .title(" steer the run ")
        .title_style(Style::new().fg(palette::TEXT).add_modifier(Modifier::BOLD))
        .border_style(Style::new().fg(palette::ACCENT));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.height == 0 || inner.width == 0 {
        return;
    }
    let line = Line::from(vec![
        Span::styled(" \u{25b8} ", Style::new().fg(palette::USER)),
        Span::styled(app.steer_input.clone(), Style::new().fg(palette::TEXT)),
        Span::styled("\u{258c}", Style::new().fg(palette::ACCENT)),
    ]);
    let lines = vec![
        line,
        Line::from(Span::styled(
            " enter sends \u{b7} esc cancels",
            Style::new().fg(palette::DIM),
        )),
    ];
    frame.render_widget(Paragraph::new(Text::from(lines)), inner);
}

/// The compact run strip for a terminal with no room for the pane.
pub(super) fn render_strip(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let color = if app.run.active {
        palette::WORKING
    } else if app.run.stop_reason.as_deref() == Some("failed") {
        palette::DENIED
    } else {
        palette::IDLE
    };
    let text = format!(" \u{25cf} {}", app.run.strip());
    let hint = " [ctrl-r]";
    let width = area.width as usize;
    let hint = if width > text.width() + hint.len() {
        hint
    } else {
        ""
    };
    let line = Line::from(vec![
        Span::styled(text, Style::new().fg(color)),
        Span::styled(hint, Style::new().fg(palette::DIM)),
    ]);
    frame.render_widget(Paragraph::new(line), area);
}

/// The plan card appended to the conversation when a plan-mode run
/// finishes. It is not an `Entry`: it is a property of the state
/// machine, and loading it is an action, not a message.
pub(super) fn plan_lines(app: &App, width: usize) -> Vec<Line<'static>> {
    let Some(plan) = app.plan_text.as_ref() else {
        return Vec::new();
    };
    let width = width.max(20);
    let head_rule: String = "\u{2500}".repeat(width.saturating_sub(14));
    let foot_rule: String = "\u{2500}".repeat(width.saturating_sub(38).max(1));
    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("\u{250c} plan ready {head_rule}"),
            Style::new().fg(palette::APPROVED),
        )),
    ];
    for text in plan.lines().filter(|line| !line.trim().is_empty()).take(6) {
        lines.push(Line::from(vec![
            Span::styled(" \u{2502} ", Style::new().fg(palette::DIM)),
            Span::styled(
                clip(text, width.saturating_sub(4)),
                Style::new().fg(palette::TEXT),
            ),
        ]));
    }
    lines.push(Line::from(Span::styled(
        format!("\u{2514} ctrl+e loads it into the message box {foot_rule}"),
        Style::new().fg(palette::DIM),
    )));
    lines
}

fn centered(area: Rect, percent: u16, height: u16) -> Rect {
    let width = (area.width.saturating_mul(percent) / 100)
        .max(36)
        .min(area.width);
    let height = height.min(area.height).max(1);
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect {
        x,
        y,
        width,
        height,
    }
}

fn format_elapsed(elapsed: std::time::Duration) -> String {
    let seconds = elapsed.as_secs();
    format!("{}:{:02}", seconds / 60, seconds % 60)
}

fn short_session(id: &str) -> String {
    let tail = id
        .rsplit_once('Z')
        .map(|(_, tail)| tail.trim_start_matches('-'));
    match tail {
        Some(tail) if !tail.is_empty() => tail.chars().take(7).collect(),
        _ => id.chars().take(7).collect(),
    }
}

fn clip(text: &str, width: usize) -> String {
    let width = width.max(1);
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
