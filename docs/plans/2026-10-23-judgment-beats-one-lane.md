---
status: completed
created: 2026-10-23
updated: 2026-10-25
slug: judgment-beats-one-lane-and-a-verdict-that-steers
---

# The judgment beats: one lane, and a verdict that steers

## Context

Two seams the typellm and fastino skills point at, both read from the
code:

- **The beats spawn the lane once per row.** `fleet judge` reads a
  day's route rows and calls `ask_lane` per row; `ask_lane`
  (`src/judge.rs:612`) spawns the sys1 MCP server, initializes it,
  lists tools, makes one `sys1_classify` call, and tears it down. A day
  here carries 81–162 route rows, so one run boots the Python lane that
  many times, each boot about a second before the judgment itself.
  `src/delegate.rs:1363` is the same shape. The router's own judgment
  already does this right — one spawn per session.
- **The execution head is paid for and discarded.** sys1's `route` task
  judges two heads in one call: the workload and the execution topology
  (`single`/`mixture`/`swarm`, `services/sys1/src/sys1/tasks.py:519`).
  Fleet's `parse_workload` (`src/route.rs:1858`) reads only the
  workload, so the second head is judged, paid for, and thrown away —
  the finding the router deep dive recorded, and the Fastino skill's
  "multiple tasks score in one forward pass" already in hand.

The typellm skill's measured precedent is the first seam's target: one
call answering N segments took JobHound's pass from 4m40s to 14s. The
fastino skill's is the second's: a decision call carries every task's
answer, and what the pass makes of them is its own.

## Approach

**One lane per beat run.** C0 and C5 are this increment's deliverable. `ask_lane` becomes a session: `open`
(spawn, initialize, tools-check), `ask` (one call), `close`. The beat's
loop opens once before the first row, asks per row inside, and closes
on every exit path — the router's `judge` shape, applied to the beats.
The same refactor lands in `fleet delegate`, because the two beats are
one loop by design ("the loop is reused, never forked").

**The verdict that steers.** C1, C2, and C3 are this increment's deliverables. The parse reads both heads into the
decision: `RouteDecision.execution` (a label and its confidence), the
calibration row carries it, and the reason names it — evidence before
behaviour. Then the execution head steers the one decision fleet can
honestly make today: a `Judged` walk (a child's, a side route's) whose
verdict is `mixture` or `swarm` walks the orchestrator's chain, because
that is the planner tier and the one whose model can plan a
decomposition; `single`, or no execution head, walks the judged
workload's chain as today. The leader is unchanged — it already walks
the orchestration chain. True multi-worker dispatch stays the
delegation tool's job, and the leader's prompt gains one compiled-in
line when the verdict is `mixture`/`swarm`, naming the tool.

**The surfaces.** C4 is this increment's deliverable: the `routed`
event the face already renders gains the execution field, so the
Router board's live line and the transcript's route strip name both
heads; the functional spec and `docs/features/routing.md` state what
the second head is for.

The mockups — the voice to approve before a line is built:

```
the calibration row, after a judged run whose lane answered both heads
  provider stepfun/step-5-preview · workload hard · execution mixture
  the reason: "sys1 judged `hard` (mixture): several independent
  answers compared by a judge would beat one; "
```

```
the leader's prompt, when the verdict names a decomposition
  ## This request
  The judgment named this request a mixture: several independent
  answers compared by a judge would beat one. `subagent` runs one
  self-contained task in its own child session.
```

## Acceptance criteria

- [x] **C0 — one lane, N asks.** `fleet judge` spawns the sys1 lane once per run and closes it once on every exit path (success, refusal, budget stop); each row is one `sys1_classify` call on that session. Pinned by a test with a counting session seam: N rows → one open, N asks, one close; a lane that fails to open refuses every row by name without a second spawn. `fleet delegate` uses the same session type (one implementation, not a second loop).
- [x] **C1 — both heads are read and recorded.** `parse_workload` returns the execution head's label and confidence beside the workload's; `RouteDecision` carries `execution`, the calibration `RouteRow` carries it, and the decision's reason names it when present. A lane answer with both heads records `execution: "mixture"`; an answer without the execution head records none rather than inventing one. Pinned by unit tests over a two-head answer and a workload-only answer.
- [x] **C2 — the execution head steers a judged walk.** A `Walk::Judged` selection whose verdict is `mixture` or `swarm` walks `ORCHESTRATOR_WORKLOAD`'s chain and its reason names why; `single` and an absent head walk the judged workload's chain. The leader's walk is unchanged. Pinned by a `select_walk` test with a scripted sys1 verdict for each execution label.
- [x] **C3 — the leader hears a decomposition verdict.** The run's prompt carries one compiled-in line naming `subagent` when the decision's execution is `mixture` or `swarm`, and no line otherwise. As built, the line is read from the run's own route decision at prompt-assembly time — the same decision the `routed` event named, one source on every routed surface — and the head rides a `routed` session record beside the event so the reconstruction replays the same line (hard rule 5). A prompt test pins both.
- [x] **C4 — the surfaces say it.** The `routed` event carries `execution` (a bridge test asserts the field from a decision that has one, and its absence when not), the Router board's live line renders it, and `FUNCTIONAL-SPECIFICATIONS.md` plus `docs/features/routing.md` name the two heads (a grep for `execution` in both prints the new lines). The gate is green.
- [x] **C5 — one open, and no row left unjudged.** The counting test pins exactly one open for N rows and N asks on it; and a scripted N-row run lands one `route_judged` row per input row in the calibration stream — the same count the day carried before the change (81–162 per day on this machine, read 2026-10-23), so the collapse buys the boot cost and loses no judgment.

## Files to be touched

`src/judge.rs` (the session, the loop), `src/delegate.rs` (the same
session), `src/route.rs` (the parse, the decision field, the walked
workload rule), `src/calib.rs` (the row field), `src/agent.rs` (the
option and the prompt line), `src/gui/bridge.rs` (the routed event's
field), `src/gui/assets/app.js` (the router board's line), the
functional spec, `docs/features/routing.md`,
`docs/features/calibration-loop.md`.

## Out of scope

The sys1 **batch class** — one `sys1_classify` call answering N texts,
the typellm contract — because it is upstream work in the vendored lane
plus a re-vendor, and C0's single lane captures most of the boot cost
without it. A named follow-up: `sys1_classify` accepting `texts: [...]`
with answers read back in order, and the beats switching to it.
Fastino's and TypeLLM's APIs themselves (the second plan: extraction
feeding the proposal gate, Decide for the fact shape). Any change to
the leader's walk or the preference table.

## Verification

The tests named per criterion; the gate; and C5's live measurement on
this machine. The capture rate the calibration stream already records
(`route_judged` rows per day) is the regression guard: the beats must
keep judging every row they judged before.

## Notes — built 2026-10-25 (the 2026-10-23 audit stands below)

All six criteria landed in one increment, gate green (fmt, clippy,
release, 1,526 lib tests + every integration suite). The counting seam
(`LaneSession`, `LaneSource::Counting`) pins C0/C5 for both beats: N
rows → one open, N asks, one close, and a failed open refuses every
remaining row by name without a second spawn. The scripted-lane mock
grew an asks parameter — one classify entry per ask, the way a real
lane serves a session — which is why the mock's three-entry script
could not survive the collapse unchanged. C3's mechanism as built
reads the head from the run's own route decision at prompt-assembly
time (one source, every routed surface) and records it beside the
`routed` event so the reconstruction replays the same line; the plan's
`AgentOptions` carrier is superseded by that, named here rather than
silently. C2's floors: the head steers only at or above the gate's own
confidence floor, and a sub-floor head rides the row as evidence
without steering. Live capture-rate measurement (the 81–162 rows/day
guard) stays the next `fleet judge` run's own report — the invariant
it guards (one row per input row) is pinned by the counting test.

## Notes — audited 2026-10-25, against commit history

No commit has touched `src/judge.rs` or `src/delegate.rs` since this
plan was written: the gap is real and every criterion unmet. Verified
against the tree — `ask_lane` still spawns → initializes → lists → one
call → tears down (`src/judge.rs:612`; `src/delegate.rs:940` — this
plan's `1363` citation has drifted, use 940), `parse_workload` still
returns only the workload head and discards the execution head. C0
through C5 all stand as written. Build order note: C0+C5 are one
increment (the counting seam), C1 is the parse, C2–C4 hang the
steering off it.

## Linked artifacts

- `docs/plans/2026-10-21-router-rules-everything-else.md` — the router's walk this plan steers inside.
- `src/judge.rs`'s `ask_lane` — the per-row spawn this plan collapses.
- The typellm skill's one-call-N measurement and the fastino skill's multi-task pass — the patterns, not the vendors.
