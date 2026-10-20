# The terminal face — `fleet`

`fleet` with no subcommand is the ratatui surface over the same agent
core the CLI and the browser face drive. It is keyboard-first and
denser than the browser: the conversation is the center, and the right
pane is a tabbed run card and board set rather than a dock.

## The shape

At 120 columns the frame is the conversation on the left and a tabbed
pane on the right (`run router brain tower`), with a status bar, a
message box, and a footer of keys. Below 80 columns the right pane
folds into a one-line run strip above the message box; the conversation
keeps the width.

```
┌ conversation ──────────────────────────────┐ run  router  brain  tower
│ you  build the tui revamp                  │ ┌ the run ──────────────┐
│ fleet  Starting with the shell.            │ │ in flight  0:42       │
│ ✓ bash  cargo test  done                   │ │ ◉ act  bash · 3 calls │
│                                            │ │ gate on · sys1        │
│                                            │ │ branches 1            │
│                                            │ │ ● 3f2a1c done 4 turns │
│                                            │ │ [s] stop  [i] steer   │
├────────────────────────────────────────────┴─┴───────────────────────┤
│ ● stepfun · auto  t7  12.4k tok  ctx 38% ▰▰▰▰▱▱  pane run           │
│ ▍ message                                                            │
│ Enter send · Ctrl-P palette · Ctrl-R run · Ctrl-T plan · q quit      │
└──────────────────────────────────────────────────────────────────────┘
```

The status bar's context gauge is a `LineGauge` fed by the last
streamed `usage` event; the conversation has a `Scrollbar`; the boards
use the tab row as their title. The palette and the tables are the only
surfaces that need their own state (`ListState`/`TableState`).

## Modes

| Mode | What it is |
|---|---|
| Chat | typing, streamed tokens, tool cards, the plan card |
| Run | the stop, the steer, and the branch cursor |
| Steer | the one-line steer editor inside Run |
| Palette | every pane, panel, and action, filtered by typing |
| Sessions | the archive as a styled table |
| Checkpoints | the turn table; Enter arms a rewind, `d` opens the diff |
| Diff | files and hunks; `r` rejects the hunk, `R` the file |
| Confirm | `y` lands the armed rewind/reject; any other key walks away |
| Panel | memory, skills, calibration, or delegation, list + detail |
| Approval | the tool call's yes/no/always prompt |
| Lanes | the vendored lanes' health |
| Settings | the scalar knobs and both lists, cards not a bare table |
| Explain | today's explanation card |

## The run card and the controls

The card is reduced from the same `UiEvent` log the conversation reads:
route → think → act → verify, the turn, the tool, the calls, the facts,
the lanes, the checkpoint, the delegation branches, and the last step.
The verify step names the failover gate from `route::calibration_config`
(`gate on · sys1` / `gate shut`). A stopped run renders as **stopped by
you**, not a failure: the core reports a stop through `Failed`, and only
the reason string separates the two.

`Ctrl-R` opens the controls. `s` writes `{"stop":true}` and `i` opens
the one-line steer. The writes go through `delegate::write_control`, the
same `<sessions>/<session>.control` file the agent's turn loop reads
before every model call. The latency is one step, the card says
`stopping…` until the run's end record lands, and the control file is
cleared when the run returns so a stop on the last step cannot end the
next run.

## Plan mode

`Ctrl-T` toggles plan mode for the next run. A plan run's final answer
is kept as the plan card; `Ctrl-E` loads it into the message box, where
the owner edits and sends it. The TUI never submits a plan on its own.

## The palette and the direct keys

`Ctrl-P` opens the palette. Every letter filters (including `j` and
`k`, which are not movement here); ↑/↓ move; Enter performs the chosen
action. The direct keys remain: `Ctrl-O` sessions, `Ctrl-K`
checkpoints, `Ctrl-L` lanes, `Ctrl-,` settings, `Ctrl-X` explain,
`Ctrl-B` cycle the pane, `Ctrl-R` run controls, `Ctrl-T` plan, `Ctrl-E`
load the plan, `q` quit from an empty prompt, `Ctrl-C` always quits.

## The trust trio

- **Checkpoints** are read from the home's `checkpoints` store. Enter
  on a turn arms a rewind; `y` restores the tree, appends the `rewind`
  record, cuts the log at it, reseeds the view, and drops the cached
  route in the agent thread.
- **Diff** re-reads the tree against the selected checkpoint. Each hunk
  is shown with its old/new lines; `r` arms a hunk reject and `R` arms
  the whole file. A reject re-derives the diff before writing, then
  re-reads it, so the panel shows the tree as it now is.
- **Plan mode** is the fourth trust-trio capability and lives above.

Every trust write is confirmed with the sentence naming what it puts
back; there is no unconfirmed destructive key.

## The knowledge panels

The palette opens memory, skills, calibration, and delegation. Each is
an on-demand read on its own thread — the library scan and the archive
replay are too heavy for the 30-second board cadence — and each renders
a list on the left and a detail column on the right (stacked below 100
columns). `f` cycles the panel's own filter:

- memory: all / live / protected / retired
- skills: all / used / unused / proposals / skipped
- calibration: all / fit / informing
- delegation: all / live / history

The rows are the substrate's own reads (`boards::read_memory`,
`read_skills`, `read_delegation`, `calib::read_day` + `calibration`),
not a second derivation. A read that fails lands a named note in the
panel; a read that finishes after the panel closed is dropped by the
generation stamp.

## Widgets and the contract

Only the already-declared `ratatui 0.30.2` is used. The revamp added
`Tabs`, `Table`/`TableState`, `LineGauge`, `Sparkline`, `Scrollbar`/
`ScrollbarState`, `List`/`ListState`, and `Cell`/`Row`; the boards and
settings keep their hand-composed cards. No crate was added, so the
one-static-binary contract is unchanged.

The 0.30 traps are respected: the gauge's ratio is clamped, the
scrollbar's content length is only rendered when it is non-zero, and
the deprecated `Table::highlight_style`, `LineGauge::line_set`, and
`LineGauge::gauge_style` are not used.

## Verification

The state machine and key mapping are tested without a terminal; the
renderer uses `TestBackend` at 120×40, 80×24, and a 20×20
does-not-panic case, asserting the rendered text and, where precision
matters, the buffer cells and cursor. A live terminal pass in tmux
covers the run card, stop/steer, plan mode, the palette, sessions,
checkpoints, the diff and confirm, and every knowledge panel.
