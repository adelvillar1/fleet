---
name: fleet-tui-revamp
description: "owner direction 2026-10-20 — the TUI was left behind by the GUI and must reach feature parity, built from ratatui 0.30.2 alone (zero new crates); IMPLEMENTED e8ebdec (tabbed pane, run card with branches/gate/stop/steer, Ctrl-P palette, sessions table, checkpoints+diff, four knowledge panels) then the composition pass 968471c (router BarChart+Gauge, tower Chart+Sparkline+project bars, lanes Table, scrollbars everywhere); still open: related rows, replay preview"
type: project
date: 2026-11-04
basis: confirmed-by
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

The owner, right after the GUI completion landed: "the app has left behind the tui
and even though they are different use cases, the tui should have as much
functionality as possible… research what's available from the ratatui project, and
if we need to add something, we should also do that." Then, after the revamp
shipped: "are we making good use of the widgets? … this could elevate the
experience like you did with the GUI." Both went through plan mode (the second as
a "board composition pass") and both were implemented and pushed the same day.

**Research first (what the dependency already gives us).** fleet is on
`ratatui = "0.30.2"` (split into ratatui-core 0.1.2 / ratatui-widgets 0.3.2 /
ratatui-crossterm 0.1.2, all under the familiar paths, default features on). No
crate was needed: `Table`/`TableState`, `Tabs`, `Gauge`/`LineGauge`,
`Sparkline`, `BarChart`, `Chart` (+`Axis`/`Dataset`/`GraphType`), `Canvas`,
`Scrollbar`/`ScrollbarState`, `List`/`ListState` were all already there. Before
the revamp the TUI had only ever used `Paragraph` and `Clear`. **0.30 traps that
would fail `clippy -D warnings` or panic**: `Gauge::percent`/`ratio` panic out of
range (clamp the ratio); `ScrollbarState` renders nothing with content_length 0;
`Table::highlight_style`, `LineGauge::line_set` and `gauge_style` are deprecated
(use `row_highlight_style`, `filled_symbol`/`unfilled_symbol`, `filled_style`);
`Tabs::padding` takes two `Into<Line>`; `Table::new` requires widths.

**What shipped (e8ebdec, 2026-10-20).** `src/tui/ui.rs` became `src/tui/ui/`
(`mod.rs` layout, `run.rs`, `command.rs`, `trust.rs`, `knowledge.rs`); new
`src/tui/panels.rs` holds the knowledge reads.
- **Tabbed right pane** (`Pane::Run|Router|Brain|Tower`, order chosen so the
  hidden Ctrl-B cycle's router→brain→tower order is preserved) with a real
  `Tabs` row. The tab row cost one line: two board tests needed 40→42 rows.
- **Run card** (`RunState` reduced in `app.rs` from the same UiEvents as the
  GUI): route/think/act/verify, tally, live delegation branches, the sys1 gate
  from `route::calibration_config` at launch, a `Sparkline` of per-response
  tokens. **Stop/steer** via `Effect::Control` → `delegate::write_control`;
  `Command::Plan` + `Effect::Run { task, plan }` for plan mode (Ctrl-T;
  Ctrl-E loads the plan into the message box, never sends).
- **Ctrl-P command palette** (`List` + `ListState`), sessions as a `Table`
  with an `f` filter, **Ctrl-K checkpoints** (Enter arms a rewind, `d` opens
  the diff; `r` rejects the hunk, `R` the file; every trust write behind a
  `Mode::Confirm` that `y` lands and any other key abandons), and four
  knowledge panels (memory/skills/calibration/delegation) read on their own
  thread with a generation stamp that drops a late read for a closed panel.
- Status bar `LineGauge` for context; conversation `Scrollbar`; a small
  markdown pass for assistant text; a welcome line naming the opening board
  counts (`37 route calls today · 31 projects`).

**The composition pass (968471c, 2026-10-20) — the answer to the widget
question.** The audit found the gap was never widget COVERAGE but
COMPOSITION: `render_board` returned one `Vec<Line>` into one `Paragraph`, so
no widget ever got a `Rect`. New files `ui/board_view.rs` (router + tower),
`ui/lanes.rs`, `ui/widgets.rs` (a shared `scrollbar(frame, area, content,
position, viewport)`):
- **Router**: busiest hours of the day as a horizontal `BarChart` (hour label,
  calls, bar), the served rate as a block `Gauge`, the model mix a second
  `BarChart` only when >1 model took a call (a one-bar chart is a line with a
  block on it). The ledger was moved ABOVE the served rows so a short pane
  truncates the list, not the counts. `router_lines` lost its hand-drawn
  `boards::bar` model bars.
- **Tower**: reports-by-day from the brain slot's `reports.report_daily` as a
  `Chart`, and commits from `tower.timeline` as a `Sparkline` at its own scale
  — a SHARED axis flattened commits into the baseline (multiples: points of 5
  vs bars of 45). Top-6 projects by 30-day commits as a horizontal `BarChart`
  above the existing cards. The chart is taken from the brain slot, so no new
  read; if that slot has not landed it is omitted with a named line, never
  drawn empty.
- **Lanes**: the health cards became a `Table` (lane/role/state/tools/version+
  latency) with the lane's tools and blend1's doctor sub-lanes as spanning
  rows; scrollbar when it overflows. `WEB_LEG` ("web") renders as "web leg".
- **Scrollbars** now on sessions, checkpoints, knowledge panels, palette, lanes
  — drawn only on overflow. Deliberately NOT used: `Canvas` (a labeled graph's
  hard part is layout, not drawing), `Calendar`, `Logo`, `Mascot`, `Fill`.
- Responsive rule: router charts collapse to lines below ~16 inner rows, tower
  keeps only bars+cards below 26; the tail is always named `+N more`.
- Tests added: router bars/gauge at 120×42 and their collapse at 80×24, tower
  `reports by day` + `commits by day` and omission at 80×24, a long sessions
  list shows the scrollbar (`▲`).

**Still open (deferred, not claimed):** (1) no **related-object rows** in the
TUI — cross-kind suggestions remain GUI-only, so "move `src/gui/related.rs` to
`src/related.rs`" did NOT happen; (2) the sessions picker resumes but has no
read-only replay preview.

**What the implementation taught (beyond the plans).**
- **A `j`/`k` movement binding breaks a palette that filters by typing** —
  typing "skills" produced "sillfs". Any filter-with-typing surface must keep
  every letter as text (↑/↓ only).
- **A modal whose run ends must close itself**: the run controls stayed over
  the composer after the run ended, so the next task was swallowed as
  unhandled keys. `Finished`/`Failed` now return to Chat.
- **`Table`'s `highlight_symbol` consumes cells of the first column** — a
  percentage-width first column clips the id under the cursor; use fixed
  widths sized around the symbol.
- **Summing spend from the `Usage` events, not the finished `RunReport`, makes
  a rewind honest**: the TUI thread's late report on its own channel
  re-populated counters a seed had just zeroed (seen live `66787/94` after a
  rewind). An "ignore next usage" flag broke an existing seed test, so it was
  removed and the per-run report stopped being sent at all.
- **`delegate/tests.rs`'s synchronous-path guard** (source grep over a fixed
  module list via `include_str!`, copied into delegate/explain/judge tests)
  refuses the module name in the TUI; `delegate::write_control` is now allowed
  by filtering exactly that line, because it writes a file and spends no lane
  ask. Moving a file means updating every `include_str!` copy and re-sizing the
  arrays (they are fixed-length `[(&str,&str); N]`).
- **A chart at 42 columns needs its data chosen, not just its widget** — the
  first tower attempt used a time axis with edge labels and produced daily dots
  plus an unreadable line; the second used the same axis for both series and
  flattened commits. Ask "what is the comparison" before "what shape".
- **A wizard in a 32-row terminal collides with a 42-column pane**: `fleet`
  (no subcommand) is the TUI; `fleet chat` is the plain REPL. Draw the shape
  first (ASCII mockup with real data in it), then pick the widget.
- `fmt --all` reformatted the GUI completion's untouched lines too — committed
  separately (36bf3fd).

**State:** pushed `e8ebdec` (revamp), `36bf3fd` (GUI fmt/clippy), `e2b65c2`
(memories), `968471c` (composition). After the composition pass the gate was
**1318 passed, 1 ignored** (1174 lib + 103 integration + 28 others); release
built. Docs: `docs/features/tui.md` (its "The composed boards" section names
each widget and why), plus README / FUNCTIONAL-SPECIFICATIONS (the TUI mode
table) / TECHNICAL-DOCUMENTATION (module list includes `board_view.rs`,
`lanes.rs`, `widgets.rs`). Live-verified in tmux at 120×40 and 80×24: router
bars + served gauge, tower reports chart + commits sparkline + project bars,
lanes table with doctor rows, sessions scrollbar, and a stopped/steered run.

**How to apply:** the TUI is the second full face — treat "the GUI has X" as a
question about whether the TUI should too, and reach for the built-in ratatui
widgets before proposing a crate. A board that returns one `Vec<Line>` into one
`Paragraph` will never use a widget: split the pane into regions first. A run's
controls are the control file and nothing else. Related:
[[fleet-run-control-and-timeout-gaps]], [[fleet-gui-editorial-desk-redesign]],
[[fleet-tui-status-bar-routing-label]].
