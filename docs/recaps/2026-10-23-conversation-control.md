# Conversation control: the commands and the run's own dials

Status: completed
Date: 2026-10-23

The owner's ruling: controlling the conversation is primordial; not
being able to adjust a running session is a deal breaker. Built against
`docs/plans/2026-10-23-conversation-control.md`, whose voice was
approved before a line was written, and whose C3 the owner redefined
mid-build into an ordered, reorderable queue.

## What changed

**The control file grows from two verbs to five**, all read at the one
seam before every model call: `Steer` and `Stop` (existing), plus
`Fold`, `Pin`, and `Pause`/`Resume`. The face posts any of them
verbatim through `POST /api/control`, and the agent's own parser is
the only thing that decides what the file means.

- `/goal <text>` — the session's standing objective: a per-session
  sidecar (`sessions-goals.json`, the workdirs' pattern, a pending key
  that moves when the first run names the session), a `goal` record in
  the log, a `## Goal` section in every run's prompt, a chip in the run
  bar (the folder field's own shape) and a strip line per change.
- `/steer <text>` — the composer's front door to the existing steer
  path; with no run in flight it is refused by name and writes nothing.
- `/btw <question>` — one tool-less call through a route prepared for
  the aside itself (a fixed task text, so the decision cache serves
  every ask after the first), its own runtime because the router's
  stream future is not `Send`; the answer lands as a card and the
  run's log gains nothing.
- `/compact` — dead before (`the fold is not wired yet`); now writes
  `Control::Fold`, and the loop folds at the next boundary. With no
  run it says honestly there is nothing assembled to fold.
- `/pin <provider|provider/model|auto>` — a mid-run pin re-prepares the
  running session's route and swaps the client every dialing call
  reads (`the dialer`), the run's own client untouched; with no run it
  sets the next run's pin, the same choice the select makes.
- `/pause` and `/resume` — the loop freezes before its next model call
  and continues from the same turn; a stop lands while it waits.
- **The queue** — typing while a run executes appends to an ordered
  list above the composer: drag to reorder (pointer drag, and
  Alt+↑/↓), **send first** promotes a row to the head, × drops it. The
  run's end sends the head; the next row follows when that run ends.
  Client-side state, and the chip says a reload drops it.
- `/rename <title>` — `sessions-titles.json`; the archive's rows show
  the title, else the session's first message line (read locally in
  the summary the archive already builds, never a model call).

**The two fidelity fixes.** The ↩ rewind renders hidden on turns of a
live run (the bridge refuses it there) and unhides when the run ends;
the Stop button's title states the graceful boundary.

## The criteria, walked

- **C0** — `a_goal_rides_the_prompt_and_the_compiled_constant_holds_without_one`
  (the section, the early return, the goal ahead of skills); live: the
  sidecar's pending key moved to the session and the log's `goal`
  record carries the text.
- **C1** — `a_steer_with_no_run_writes_no_control_file`; the command
  posts `/api/steer`.
- **C2** — `a_side_ask_answers_and_the_runs_log_gains_nothing` (a
  one-request SSE mock, the provider named, the log's record count
  unchanged and the aside's text absent).
- **C3** — live: two rows queued, `queued · 2`, send-first on the last
  promoted it, and the drain went 2 → 1 → none, one message per run in
  order. The face has no committed unit harness, so the live pass is
  the evidence; the plan says so.
- **C4/C5/C6** — `the_control_file_shapes_parse_and_nothing_else_does`
  (fold, pause, resume, pin) and
  `a_pinned_route_is_read_by_the_dialer_and_leaves_the_runs_client`;
  live: all four commands fired, their notices and the strip lines
  landed.
- **C7** — the archive's newest row rendered its first message line as
  the title; `/rename` writes the sidecar.
- **C8/C9** — the rewind guard and the stop's title; the functional
  spec's "In the conversation" section names every command, the queue,
  and the boundary.

Gate: 1498 passed, 1 ignored. One environmental blocker met twice: the
machine's embed service flapped (KI-296) and the gate's deck test needs
it — restarted, green.

## What is left

- KI-296 — the embed endpoint flapping; the probe reads a flap as "no
  backend", which degrades a healthy machine to keyword-only.
- KI-287 sharpened — `truncated` still carries the wall-clock budget.
- The queue is client-side by design; a reload drops it (the chip says
  so). Server-side queueing, a true pause indicator repainted by the
  run's own events, and the TUI's forms of these commands are not
  built.

Cache sweep at recap: free space on the volume: 83.1 GiB → 83.1 GiB (release targets untouched)
