---
name: composer-actions-plan-drafted-2026-11-03
description: "project 2026-11-03 — docs/plans/2026-11-03-composer-actions-apply-on-submit.md COMPLETED (b2a4f9f, C0–C6 all ticked, gate 2,012/0, harness 698/67 after the C1/CA3/CA4 pins, pushed origin/master at 29b5187 with the provider work, the Tauri app rebuilt): F1 the approval wait polls the control file non-consumingly so a stop ends the wait, F2 opening a run in flight loads its view (the resume refusal had skipped the state read), F5 the queue rows keyed like the seat, F3/F4 the scope wording; the F3 \"go live\" di…"
type: project
date: 2026-11-04
basis: inferred
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

The owner asked *"create a plan for the fixes"* after the composer-action
audit. The contract is
**`docs/plans/2026-11-03-composer-actions-apply-on-submit.md`** — and on
the owner's *"implement the composer actions plan as designed"* it was
**IMPLEMENTED WHOLE, status `completed`, all six criteria ticked with
evidence, committed as `b2a4f9f`** on `face-plugin-host`, **pushed to
origin/master** (with 980e350/191eeab/8ce0c0d, the provider-connect work),
and the Tauri app rebuilt from it. Gate: fmt,
clippy, release build, **2,010 tests / 0 failed**; face harness **693
checks across 66 cases**.

## What each increment landed

| | Fix | Pin |
|---|---|---|
| **F1** | `GuiApprover::approve` polls the session's control file **without consuming it** (`agent::control_file_stops`) before raising a seat and inside its wait loop (a `try_recv` + 120 ms poll replaced the blocking `recv`); a stop ends the wait with the call refused, the file left for the loop's own read. `stop()` resolves the pending seat, drops the responder, and broadcasts `answered` with `by: "the stop you pressed"` + a `reason` field — a stop is not a denial | C0/C1 over the real bridge |
| **F2** | `openWork` now treats the resume's 409 (*"a run is in flight in that session"*) as the cue to **load the view anyway** — the state read carries that run's own log and seat. The refusal had skipped the state read: empty log, no seat, pill "idle" | CA2 + CA2b over the harness |
| **F3** | The mode and the provider pin say **"applies to the next run"** where they paint (the `runPerms` title, the two selects' titles) | CA3 |
| **F4** | `/compact` says it folds **at the next turn boundary**, not "now" | CA4 |
| **F5** | `paintQueue` keyed on `face.queueKey` (what the rows say, in order) — the seat's own pattern | CA5 |

F1's sharp edge: the poll must **not consume** the control file. Consuming
it there would eat the stop and leave a run that refuses one call and
carries on; the loop's own `read_control` is what stops the run. See
[[the-approval-wait-blocks-every-run-control]] for the full before/after.

## The F2 root cause is worth remembering as a shape

The live symptom (empty transcript, hidden seat, pill "idle" on a session
whose `/api/state` carried 113 events, `running: true` and a standing
approval) **was not the replay path** — the harness case CA2 proves
`replay()` paints both from a state document. The cause was one step
earlier: `openWork` → `POST /api/resume` → **the bridge refuses a session
with a run in flight** → and the face treated that refusal as "don't
load". The harness exonerates the code you suspected and points at the
call one step before it.

## The F3 decision — resolved as "name the scope"

The plan defaulted to naming ("applies to the next run", the folder's own
honesty) over going live (re-reading the mode per call, which changes
what a run *is* and can retroactively refuse calls already made). That
default is what shipped. **Going live remains a one-line follow-up** if
the owner ever rules for it.

## F6–F8 became their own plan, and landed first

The owner's OpenCode connect report grew into
**`docs/plans/2026-11-03-provider-connect-honesty.md`** — implemented and
committed as **980e350** *before* this plan was (see
[[provider-connect-honesty-plan-drafted-2026-11-03]]). Note: that plan's
own file was left `status: draft` with C0–C4 unticked at commit time —
the exact debt [[a-plan-never-ticked-is-not-a-status]] names; tick it
before anyone trusts it. The disabled-models picker half of the owner's
ask is still open and lives in neither plan's landed work — see
[[provider-discovery-one-shape-and-dead-reply]].

Related: [[the-approval-wait-blocks-every-run-control]] (the mechanism and
the live proof), [[the-queue-rows-rebuild-under-the-cursor]] (F5's cause),
[[a-deferred-action-is-an-error-to-the-owner]] (the ruling),
[[an-effect-with-no-visible-trace-reads-as-broken]] (the sibling lesson),
[[gui-client-verified-by-a-vm-harness]] (how the pins were run).

## The turn-count report that closed the day — KI-408, not a regression

The owner then reported "turn count indicators" still present on the PWA
while the browser face had lost them. The turn strings in the served
app.js matched the current source exactly — the discrepancy was
**KI-408**: the daemon serving :3080 ran the fleet-master-wt worktree's
stale binary, so BOTH faces lagged the pushed master and the owner's
second viewer exposed it. The serving binary now has one home
([[fleet-gui-is-a-launchagent]] has the recipe and the restart facts).
What deliberately remains after the 2026-10-31 retirement: the composer's
turns-left budget counter (`run-turns`, shown only while a run is live
with a budget) and the fold buttons' "turn N" labels — the 10-31 plan's
own carve-out, not a regression. The day closed at origin/master
**29b5187** (the serving-home runbook) with the Tauri app rebuilt at
521724c.
