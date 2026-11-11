---
status: completed
created: 2026-11-11
updated: 2026-11-11
slug: multi-session
---

# Plan: multi-session — two active runs that never wedge the face

## Context

The owner ran a KI-ledger burndown session from the Tauri app, then opened
a second session (a comparison of the obscura project's browser use against
fleet's) to test two active sessions at once. The KI session ended up
showing task failures and the second session vanished from the rail. The
triage (KI-525) found the cause: the face is **single-run** — one serial
agent thread (`src/gui/bridge.rs:13063`) with a single `driving` field —
while its *state* is already a per-session table (`GuiState.sessions`).
Starting a second run while the first drove, combined with a face restart,
left the two out of sync: `/api/state` reported `running=true` on an empty
attached chat (a single start line, no transcript) while `driving` still
named the dead prior run. Nothing was executing, but the stuck flag made
`dispatch` refuse every new send ("a run is already in flight"), wedging the
composer behind a phantom run. The second session never executed into a log,
so it was invisible; the first session's interrupted run left its task card
stuck at in_progress/pending.

The state table's own comment names the intent — "a phone and a desktop are
two surfaces over one core and each may be looking at a different
conversation" — but execution never grew past one serial loop. This plan
makes the intent real: two (or a bounded few) active runs, each in its own
session, that the resource contract admits and the face renders honestly.

## Decision-model check

No new sys1 judgment shape fits this work. The concurrency admission is the
resource contract's **deterministic** `admitting` bound (`src/contract.rs:74`),
not a classify/score/extract decision — it answers "is there budget for a
second run" from the machine's learned profile, the same arithmetic the
footprint plan built. Per-run provider routing already rides sys1
(`route::prepare_child` / `route::select`) and is unchanged. So this plan
adds no head and no escalation lane; it spends no judgment budget.

## Approach

One path, in two stages — the safety fix ships first and alone, the
concurrency feature lands behind it.

**The owner's governing rule (2026-11-11):** concurrency is allowed as long
as the environment fleet is running in can handle it. The limit is the
machine's actual capacity — the resource contract's learned profile — never
an arbitrary fixed pool size. When resources are too constrained, fleet
**politely says so**: a named, courteous card telling the owner the run
needs to be reconsidered or queued, not a silent queue and not a terse
refusal. Admission is generous within capacity and honest at its edge.

**Stage 1 — make the single-run model honest (closes KI-525).** The defect
is a desync between the per-session `running` flags and the one thread's
reality. Three moves: (1) on spawn/restart, reconstruct `running=false` and
`driving=None` unless a run is genuinely in flight — a stale flag never
survives a restart; (2) the dispatch guard's refusal becomes a named card
the composer paints ("a run is already in flight in <session> — let it
finish or stop it"), never a silent wedge; (3) creating, joining, or
switching a session clears any orphaned flag for the attached chat and
repaints its folder (KI-524's seam). This is a bug fix, small, and ships as
a beta cut before any concurrency work.

**Stage 2 — bounded per-session run loops.** Lift the single serial command
loop to a small pool of run loops keyed by session id, reusing the existing
per-session `SessionState` as the source of truth (its `running` and `seat`
are already per-session). `driving` becomes a per-session property rather
than one global field; the events `fan_out` already routes by session, so
each run's events land in its own log. Before a second run starts, the
bridge asks `contract::admitting`: within the environment's capacity it
admits and spawns the run; beyond it, fleet paints a **polite named card**
telling the owner this run needs to be reconsidered or queued, and offers
the queue — never silently over-spending, never a terse refusal. The
ceiling is the machine's capacity as the contract reads it, not a constant.

Steps:

1. Stage 1: honest state reconstruction on spawn/restart; the named
   refusal card; the session-lifecycle flag/folder cleanup. Pin each.
2. Stage 2: factor the command loop body into a per-session run loop; key
   the pool by session; make `driving` per-session; keep `fan_out`'s
   session routing.
3. Wire `contract::admitting` as the admission gate before a concurrent
   run; over-capacity → a polite named card offering reconsider-or-queue.
4. The face harness gains the two-session, restart-recovery, and admission
   cases; the four-leg gate runs whole.

## Acceptance criteria

- [x] **C0 — the wedge is gone.** A face restart mid-flow reconstructs
      `running=false` and `driving=None` unless a run is truly executing;
      KI-525's exact repro (a stale `running=true` on an empty attached
      chat whose `driving` names a dead run) no longer occurs. Pinned by a
      bridge test that rebuilds state with a phantom flag and asserts the
      clean reconstruction.
- [x] **C1 — a second run is never a phantom.** With a run in flight, a
      send into another session answers with a named refusal or a visible
      queued state the owner can see — never a stuck `running=true` on a
      chat with no transcript. Pinned by a bridge test asserting the
      refusal/queue event and that the second chat's log is not left empty
      behind a true flag.
- [x] **C2 — two sessions execute concurrently within budget.** Two admitted
      runs both grow their logs turn by turn, each transcript renders its
      own events, and no event lands in the wrong session's log. Pinned by
      a two-run bridge test driving both sessions and asserting per-session
      event isolation.
- [x] **C3 — concurrency follows the environment's capacity, politely.** A
      second run is admitted whenever `contract::admitting` reads the
      environment as able to handle it; when resources are too constrained,
      fleet paints a courteous named card telling the owner the run needs to
      be reconsidered or queued, and offers the queue — never a silent
      over-spend, never a terse refusal. Pinned by a test that tightens the
      budget and asserts both the admitted path and the polite
      reconsider-or-queue card with its reason.
- [x] **C4 — session lifecycle strands nothing.** Creating, joining,
      switching, or archiving a session leaves no orphaned `running` flag
      and repaints the correct folder (KI-524's seam); the rail lists every
      session that has a log. Pinned by the lifecycle tests.
- [x] **C5 — the gate is green and the harness carries it.** `cargo fmt
      --all --check`, `cargo clippy --all-targets -- -D warnings`, `cargo
      build --release`, `cargo test` all exit 0; the face harness adds the
      two-session, restart-recovery, and admission cases.

## Verification evidence (2026-11-11)

Implemented as one coherent change (the dispatcher + per-session `drive_run`
threads, `contract::admits_concurrent`, the polite `queued` card, per-run
event forwarders for isolation). Verified by the session itself, not only
the implementer's report:

- **Four-leg gate, exit codes read directly:** `cargo fmt --all --check` = 0,
  `cargo clippy --all-targets -- -D warnings` = 0 (0 warnings), `cargo build
  --release` = 0, `cargo test` = 0 (2,389 passed across the suites, 0 failed).
- **Face harness:** `node tests/face/harness.mjs` → 1,108 checks across 140
  cases, exit 0 (the two-session, restart-recovery, and capacity-refusal
  cases included).
- **C0** — `a_restart_reconstructs_no_phantom_running_flag` + face
  `aReopenedFaceIsIdleAndNotWedged`.
- **C1** — `a_run_refused_by_capacity_paints_a_polite_queue_and_clears_the_flag`
  + face `aCapacityRefusalPaintsAPoliteQueuedCard`.
- **C2** — `two_concurrent_runs_grow_their_own_logs_and_stay_isolated` (real
  agent threads, both logs grow, no cross-session events) + face
  `twoConcurrentSessionsRenderTheirOwnTranscripts`.
- **C3** — `contract::tests::{concurrency_is_admitted_while_headroom_covers_the_cumulative_budget,
  concurrency_is_refused_when_the_cumulative_budget_outgrows_headroom,
  the_cumulative_boundary_is_strict}` + the bridge refusal/admitted tests.
- **C4** — `the_session_lifecycle_strands_no_running_flag` (KI-524's folder
  repaint already landed at `7b15f13`).
- **C5** — the gate and harness above.

Design deviation (recorded): each admitted run gets its own thread +
current-thread runtime rather than `tokio::spawn` on the shared runtime,
because `Agent<'a>` borrows `dyn Write`/`dyn Approver` and is `!Send` — the
same shape `spawn_agent` itself uses, genuinely concurrent. Residual (not a
criterion): the stop verb still targets the single `driving` session, so it
stops one run at a time; `driving` now hands back to a running peer.

## Files to be touched

- Backend: `src/gui/bridge.rs` (state reconstruction, the dispatch guard,
  the per-session run loop, `driving`), `src/gui/mod.rs` (spawn),
  `src/gui/server.rs` (the run/state routes), `src/contract.rs` (the
  admission wiring).
- Frontend: `src/gui/assets/app.js` (the refusal/queued card, the rail, the
  folder repaint).
- Tests: the bridge tests in `src/gui/bridge.rs`, the face harness.
- Docs: the linked artifacts below; `docs/known-issues.md` (KI-525 closes
  with evidence).

## Out of scope

- **Unlimited concurrency** — the resource contract bounds the pool; this
  plan never raises the ceiling past what the machine admits.
- **The relay/phone surface's own concurrency** — the relay is a separate
  spawned task; its multi-device behavior is its own concern.
- **A new sys1 judgment head** — admission is deterministic; no head ships.
- **Cross-machine or distributed runs** — one machine, one home.
- **Stopping at the visible queue** is a scope decision the owner may make
  at review: Stage 1 alone makes two sessions *safe* (one runs, one queues
  by name); Stage 2 makes them *concurrent*. The recommendation is to go to
  Stage 2, since the owner's ask was two active runs.

## Verification

- C0/C1/C4: the bridge tests run scoped (`cargo test -p fleet --lib
  gui::bridge`), then the whole gate; KI-525's live repro re-run against a
  scratch face (restart mid-flow, read `/api/state`).
- C2/C3: the two-run and admission bridge tests; a live scratch-face probe
  starting two runs and reading both `/api/session` transcripts.
- C5: the four-leg gate whole, exit codes read directly (KI-014); the face
  harness's new cases green.

## Linked artifacts

- `docs/features/multi-session.md` — new: the run pool, the admission gate,
  the queued/refusal states, the per-session driving model.
- `TECHNICAL-DOCUMENTATION.md` section "Interface contracts" — the run/state
  routes' concurrent semantics; section "Module reference" — the bridge's
  per-session run loop.
- `FUNCTIONAL-SPECIFICATIONS.md` section "Run behavior" — two active runs,
  the named refusal/queue, the admission bound.

## Risks

- **The serial-thread lift touches the approval flow.** The seat is already
  per-session, but any global assumption about "the one waiting approval"
  must be found and made per-session; the always-ask class and the stop
  verb's "driving session" logic (`bridge.rs:5305`) need re-reading.
- **Shared seams must stay session-keyed.** The events broadcast, the route
  cache (`route_cache`), and the workdirs map are shared; a concurrent run
  must not read another's route or folder.
- **Model spend multiplies.** The contract gates it, but the contract's own
  wiring is partly unbuilt (the handoff plan) — Stage 2 depends on it.
- **The resource contract is a hard dependency.** If `admitting` is not yet
  wired into the run loop, Stage 2 cannot admit honestly; Stage 1 still
  ships independently.

## Dependencies

- `docs/plans/2026-11-09-the-resource-contract.md` — the `admitting`/`settle`
  arithmetic Stage 2 gates on (and its handoff's unbuilt wiring).
- KI-524 (the absent-folder repaint, the uncommitted fix in the tree) and
  KI-525 (this find) — Stage 1 closes both.