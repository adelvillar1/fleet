---
name: fleet-conversation-control-shipped
description: "2026-10-23 — the conversation-control slice SHIPPED in 70dc3bf (goal, steer, btw, fold/pin/pause/resume, the ordered reorderable queue, session names) after the owner redefined the queue mid-build; the control file now carries five verbs"
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

The slice that [[fleet-run-control-and-timeout-gaps]] paused mid-build was
completed and pushed 2026-10-23 in `70dc3bf` (plan
`docs/plans/2026-10-23-conversation-control.md`, completed; gate 1498
green).

**The control file now carries five verbs**, all read at the one seam
before every model call: `Steer` and `Stop` (pre-existing), plus `Fold`,
`Pin`, and `Pause`/`Resume`. The face posts any of them verbatim through
`POST /api/control`, and the agent's own parser is the only thing that
decides what the file means.

- `/goal` — the session's standing objective: sidecar
  `sessions-goals.json` (workdirs' pattern) with a **pending key that
  survives the prune and moves when the first run names the session**, a
  `goal` record in the log, a `## Goal` prompt section, a run-bar chip.
- `/steer` — the composer front door to the existing steer path.
- `/btw` — one tool-less call through a route prepared for the aside
  (fixed task text, so the decision cache serves later asks), on **its
  own runtime** because the router's stream future is not `Send`; the
  run's log gains nothing.
- `/compact` — real now through `Control::Fold`; it had printed "the
  fold is not wired yet".
- `/pin` — a mid-run re-route through a swapped **dialer** client; the
  run's own client is untouched.
- `/pause`, `/resume` — the loop freezes before the next model call.
- `/rename` — `sessions-titles.json`; the archive falls back to the
  session's first message line (read in the summary, no model call).
- **The queue** is an ordered list: drag to reorder, **send first**
  promotes, × drops; the run's end sends the head and the next follows.

**Owner rulings captured here:** the queue is an ordered list the owner
reorders ("drag and drop or a direct send so that it's read first") —
the one-slot shape was superseded mid-build; and `/btw` is "a side
conversation while things are executing".

**Why:** these are the seams everything else in run control rides, and
the delivery commit is the evidence claim.

**How to apply:** build new run controls on `Control` +
`POST /api/control`, never a second channel. Remember the three
constraints found while building: the router's future is not `Send`
(use a blocking task with its own runtime), a sidecar key with no log of
its own is pruned unless deliberately kept, and the face's event
reducer must learn `face.session` from the `session` event or anything
keyed on it drifts ([[a-reducer-dropped-case-goes-silent]]). Related:
[[fleet-conversation-control-inventory]].
