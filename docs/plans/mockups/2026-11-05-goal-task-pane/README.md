# The goal's tasks — a pane card (mockup)

`goal-task-pane.html` is one navigable page covering the whole matrix:
**4 states × 3 pane widths × 4 themes = 48 cells**, each addressable by
URL:

```
goal-task-pane.html?state=<running|done|long|empty>&w=<300|440|720>&theme=<paper|console|graphite|contrast>
```

The rack at the top switches state, width and theme and writes them
back to the URL, so a review comment can name one cell exactly.

## Why this exists

The owner's ask: while a goal runs, the right-side collapsible pane
should show the running list of tasks and each one's completion status.
The find behind `docs/plans/2026-11-05-goal-task-pane.md`: the `todo`
tool already IS the run's task list — the model maintains it, it
survives folds — and the face has never heard of it. This mockup is the
design pass for the card that surfaces it.

## The tokens are copied, not invented

Every colour, size, radius and shadow here is hand-copied from
`src/gui/assets/app.css` — the four real themes, the real type scale,
the pane's real card anatomy (`.pane-head`, `.pane-tabs`, `.pane .card`,
`.card-head`, `.count`, the process stepper). The translation to
Rust-side CSS is meant to be a translation. If a value here is not in
`app.css`, it is a bug in this mockup, not a new token.

One new family, prefixed `gp-` (goal pane) so it cannot collide with an
owned `.pane` / `.card` / `.field` name — the house collision lesson.

## What it demonstrates

- **The card lives in the run tab, after the process card**, and reads
  as the same family: the run's stepper above, the goal's tasks below.
- **The marks are the pane's own idiom.** Pending is the hollow dot,
  in-progress is the current dot (accent ring + wash, bold text),
  completed is the filled accent dot with settled text — the process
  stepper's vocabulary, not a new one. The goal's own state word
  (`active` / `done`) and the `2 of 5` pill carry the app's
  `working` / `done` colours.
- **The done state shows the tool's note** — the `goal` tool's note
  rendered in the card, not buried in a transcript tool-result.
- **Empty is honest absence.** The `empty` state shows the pane with no
  card at all: the face never fabricates rows from tool calls.
- **A long list scrolls inside the card** (`long`), proving C4's
  internal-scroll clause at the grip's widths.

## The measurement

48/48 cells clean (script: `tmp/goal-pane-mockup/measure.mjs`, CDP,
Chrome headless, deviceScaleFactor 2):

- **No page overflow, no pane overflow, no clipped text** in any cell —
  `overflow-wrap: anywhere` wraps at the 300px minimum, nothing clips.
- **Ink ≥ 4.5:1 everywhere; the worst measured text pair is 5.39:1**
  (the count pill on its wash, console theme). The settled task text —
  the lightest ink on the lightest ground — measures 5.62:1 minimum.
- **Row heights 31–49px** (49 = a wrapped two-line row at 300px); the
  long state's internal scroll proven (`scrollHeight > clientHeight`).

## The read

Nine cells eye-read, covering every state in a light and a dark theme
and both width extremes: `running` (paper 300/440, console 440,
contrast 720), `done` (console 440, paper 440), `long` (graphite 440),
`empty` (paper 440, graphite 440). The read caught one defect the
measurement called green: the done state's process card still showed
`act` as the current step — a visual lie in the mockup's own context
card, fixed by painting the run's steps done with the goal.

The mockup's scrollbar in the `long` state is the platform default;
the implementation may style it subtler, but the scroll itself is the
design.
