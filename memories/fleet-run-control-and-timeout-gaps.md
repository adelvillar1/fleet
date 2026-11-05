---
name: fleet-run-control-and-timeout-gaps
description: "KI-248/KI-249, owner-raised 2026-10-19 — run control is the owner's deal-breaker; stop+steer shipped to both faces and Stop reached the composer; 2026-10-23 the owner approved and then REDESIGNED the conversation-control plan (queue became an ordered, reorderable list), and its build is paused mid-plan"
type: project
date: 2026-11-05
basis: confirmed-by
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

Two owner-raised product gaps, filed 2026-10-19 as open ledger rows during the
type-ahead/token work:

**KI-248 — a run in flight could not be stopped, steered, or conversed beside.** The
owner: "calls hang, no way to stop, steer, or do a side conversation with the
orchestrator." The three surfaces were named in ascending scope: a **stop** (cancel
between turns), an **inject** (fold an owner message into the running turn), and a
**side conversation** that can steer or stop the run as its outcome.

**Update 2026-10-20 — stop and steer SHIPPED for the browser face** (`Bridge::stop(steer)`
→ `delegate::write_control` → `<sessions>/<session>.control`, `POST /api/stop` and
`/api/steer`, a control card, cleanup after every run; ddb8724, cd8ad1a), and the TUI
found stop/steer too (Ctrl-R). **2026-10-22 — Stop reached the composer** (29d1fed): the
Send button becomes Stop in place while a run is in flight, posting the same route, with
the square glyph drawn in CSS.

**Owner ruling 2026-10-23 — controlling the conversation is primordial.** "Not being able
to adjust a running session is a deal breaker for user experience." The inventory read
from the code, and the ranked direction, is in [[fleet-conversation-control-inventory]].

**2026-10-23 — approved, then the queue's design was replaced.** The owner said include
every gap in one plan, and the plan (`docs/plans/2026-10-23-conversation-control.md`, ten
criteria C0–C9, plan-gate PASS) was approved at `/goal implement plan as designed`. Mid-
build the owner redirected the queue: "the queue should allow reordering of the queue
items using drag and drop or a direct send so that it's read first" — an ordered list
with drag reorder and a send-first action, not the one-slot queue the plan had. C3 was
amended (327cacc) and the partial single-slot code was left uncommitted, correctly. The
goal was then paused; see the inventory note for how far the build got.

**KI-249 — network timeouts tighter than the work they bound.** The owner: "the timeouts
need to be extended because they are not realistic." The embedder probe answers within 5s
and steady-state embeds within 10s against a warm local GGUF's ~20-30ms; the model call
is allowed 600s. The row's action stays deliberate: measure a day of real durations and
set the bounds from the measured tail.

**2026-10-25 — the measurement is not waiting on a run, it is waiting on a recorder.**
The `embed_timing` writer landed 10-24 17:31 (`8c05b28`); the stream holds zero rows
(checked 10-21 through 10-25) because the two paths that record both load the corpus
first — the CLI `fleet run` (`main.rs`, recorded before any model call, so any run
writes one) and the scheduled jobs (`cron.rs`'s `run_job`, guarded on
`Some(memory)` + `Some(calib)`; the face's own runs load memory but do NOT record).
The scheduled path is inert because `cron::Scheduler::spawn` is hosted inside the
`fleet gui` process, and that process (PID 28667) started 10-23 3PM — before the
writer landed — so its scheduler still runs the pre-writer code and fires jobs that
record nothing. **2026-10-25/26 — the trigger ran, and it measured the wrong half.** The
restart happened on the owner's word ("restart the gui then"): `/api/state`
on port 3080 checked first (nothing in flight), then `launchctl kickstart -k
gui/$(id -u)/com.fleet.gui` → new PID 77972 running the current release binary,
and its catch-up ran the missed fires. The owner's next question ("why do we
have to wait? you should be able to trigger it") pushed the by-hand run: a CLI
`fleet run` first refused at registry seeding — the all-or-nothing
`enabled_specs` shape in [[registry-server-seeding-is-all-or-nothing]] — and
with dummy env values passed in the environment only it recorded. **17
`corpus-load` rows on 2026-10-26: p50 41 ms, p90 385 ms, max 2,183 ms (the cold
index), facts 50–61** — the warm load sits two orders of magnitude under every
existing bound, so no load-path bound moves. The sharper find: those rows
measure the LOAD, which makes no embed call, while `PROBE_TIMEOUT`/
`EMBED_TIMEOUT` bound the embedder's CALLS — the instrumentation measured the
wrong half of the row ([[instrumentation-must-measure-what-the-bound-bounds]]).
Fixed in `3172e63`: `Memory::load_with` collects the load-time index build's
per-embed wall clocks (`build_embeds_ms`), and both stream-holding callers
record one `embed` row per call, pinned by the hybrid-ranker test (four embeds
→ four timings; a cached reload → none). The bounds now wait for the embed tail,
which arrives on the next corpus change.

**Why:** both are the owner's own reports of fleet failing in real use, and the
conversation-control one is explicitly a deal-breaker rather than a backlog item — and
the owner's instinct is to sweep the whole gap list into one plan rather than slice it,
then to refine a control's shape once they have seen it written down.

**How to apply:** when touching the run loop or the composer, treat run control as the
primary UX axis, not a polish item. Design against the existing control-file/checkpoint
seams. When a plan's design is approved but the owner re-specs one element mid-build,
stop building that element, amend the plan first, and leave the superseded code
uncommitted rather than shipping a shape they have just replaced. For KI-249, find which
bound actually fires before touching constants — and check that the measurement's
recorder is live code on a running process before concluding the data is missing. Related:
[[fleet-conversation-control-inventory]], [[fleet-gui-editorial-desk-redesign]],
[[fleet-tui-revamp]].
