---
name: fleet-conversation-control-inventory
description: "2026-10-23 inventory of every session/conversation control fleet has, and the ranked gaps the owner called a deal-breaker — queue a message mid-run is #1, then the dead /compact, the mid-session route switch, a hard interrupt, a true pause"
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

The owner (2026-10-23): "controlling the conversation is primordial as not being able to
adjust a running session is a deal breaker for user experience." Asked what other
session-control commands exist or could exist, and the answer read from the code:

**Exists.** Composer bar: permissions (Ask before / Autonomous / Full access), the
provider pin, the working folder, Plan mode, Send/Stop. The seat: once / always /
remember, plus `/approve`. Slash commands: `theme`, `pane`, `go`, `clear`, `help`,
`review`, and `compact` — **which is dead in the face**, it prints "the fold is not wired
yet" because the agent reads its forced-fold flag once at run start. Transcript/process:
a per-turn ↩ rewind (the bridge refuses it mid-run), the process card with stage dots and
the stop/steer box, the run receipt. The run's control file carries only `Steer(text)`
and `Stop`, read before every model call. Around the session: open/continue from the
sessions page, checkpoints and `/review`, automatic compaction, `runs.max_turns`, cron.

**The ranked gaps** as recorded for the owner, highest value first: (1) no way to
**queue a message while a run executes** — the composer disables and you wait; (2)
`/compact` is a lie in the face — wire a per-turn `Control::Fold` on the same file the
steer rides; (3) the **route cannot change mid-session** — the pin applies to the next
run (a per-turn `Control::Pin`); (4) Stop is graceful only — a long tool call finishes
first, and a hard interrupt exists for a bash child but not for a model stream; (5) no
true **pause**; (6) sessions have no names; (7) the ↩ rewind affordance still shows on
live-run turns where clicking it is refused.

**Why:** this is the owner's primary UX axis, and the inventory is what keeps the next
plan from re-discovering it.

**How to apply:** the owner overruled the recommended slicing ("I think we should include
them all in the plan"), so one plan carries every gap and the three commands
(`docs/plans/2026-10-23-conversation-control.md`, renamed from
`2026-10-23-chat-commands-goal-steer-btw.md`, plan-gate PASS 10/10, criteria C0–C9) under
one organizing idea: the control file the agent already reads before each model call
grows from two verbs to five (`Steer`, `Stop` existing; `Fold`, `Pin`, `Pause` new).

**The queue was then redesigned by the owner, and it matters (2026-10-23).** C3 was
written as a ONE-SLOT queue; the owner: "the queue should allow reordering of the queue
items using drag and drop or a direct send so that it's read first." C3 and its mockup
were amended (327cacc): an **ordered list** above the composer — Enter appends, a row
drags to reorder, a **send first** action promotes one to the head, a × drops it — and
the flush drains **one message per run in the owner's order**. The single-slot code
written before the amendment is superseded and must not ship; it was the reason the work
stopped at the plan instead of committing a design the owner had just replaced.

**Build state (2026-10-23, goal paused mid-plan).** Done and green, uncommitted-in-one-
tree at the pause: C0 the goal (sidecar + `POST /api/goal` + a `## Goal` prompt section +
a `goal` log record + `/goal` + run-bar chip, prompt unit test), C1 `/steer` (command;
the bridge's "no run is in flight" refusal pinned by a test), C2 `/btw` (`POST /api/btw`,
one tool-less call through the routed `chat_stream`, the run's log gaining nothing,
bridge test green). Two build facts worth keeping: `Client::chat()` is the
single-endpoint path and a routed client has no base URL (`RelativeUrlWithoutBase`), so a
side ask must cross `chat_stream`; and the router's streamed future is not `Send`, so an
async bridge method drives it on a `spawn_blocking` + current-thread runtime (the agent
thread's own shape). Remaining: the reworked queue, Fold/Pin/Pause, `/rename`, the two
fidelity fixes, docs + live pass.

Related: [[fleet-run-control-and-timeout-gaps]],
[[fleet-composer-needs-permissions-and-a-route-pin]].
