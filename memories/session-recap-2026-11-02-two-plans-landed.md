---
name: session-recap-2026-11-02-two-plans-landed
description: "project 2026-11-02 — the session's arc: the usage/anomalies plan implemented whole (140c739) and the face plugin host drafted, amended on two owner rulings, implemented and completed (c8dde93), with the 1.99.1 toolchain cleanup folded in between; gate 1,996/0, harness 678/0 — SUPERSEDED as the session's record by session-recap-2026-11-02-session-parity-and-plugin-host, which carries the wider four-plan arc"
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

The session's two goals, in the order that made sense (the smaller
calibration-only plan first, then the face plugin host), both landed and
completed on origin/master:

1. **Usage in the calibration stream, and the stream's own anomaly pass**
   (`140c739`, plan completed, nine criteria ticked) — Increment A the usage
   columns riding the route rows (`Usage`'s cache/reasoning `Option` fields
   with the providers' wire names, `billed_tokens()` the one reading;
   `RouteRow.usage` set from the serving attempt; the `tokens:` line and the
   closing `spend` card rendered only when usage rides); Increment B
   `src/anomalies.rs` (six deterministic detectors — fallback-walk,
   tool-failure-loop, lanes-down, delegation-maxed, run-failure-cluster,
   cache-collapse — at most two lane asks, zero on a dry run);
   Increment C the trajectory shape (`RunReport.turn_curve`, `ShapeRow`,
   `fleet calib --shape`).
2. **The face plugin host** (`c8dde93`, plan completed, ten criteria C0–C9
   ticked) — the format + registry + `fleet plugins` lifecycle; the rail
   mounting and the one-card management chains; the two tiers' frames
   (sandboxed untrusted, plain privileged); the eval suite; the creator
   flow's drafts-never-self-applied; the promotion ladder; the demo bundle.

**Between them:** the owner's *"we had to upgrade rust to 1.99.1 and that
generated some conflicts, the session cleaned all that up"* — the cleanup
commit `eabe031` carried the 41 lint fixes AND the co-tenant's parked plugin
work, on `face-plugin-host`; the follow-up session verified the committed
state (fmt 0, clippy 0, build 0, test 1,996/0, harness 678/0) and landed the
finish at `c8dde93`.

**Two live legs remain named-pending in the plans' own criteria:** the usage
plan's real-provider live run (the cache fields read back from the stream) and
the plugin host's demo scratch-home creator run + the eval suite's rendered
browser-lane pass. Both are the plans' verification steps, recorded as pending
rather than claimed.

**The session's reusable lessons** (each its own memory):
[[a-harness-find-returns-an-array]], [[a-python-literal-sweep-eats-non-literal-sites]],
[[a-parallel-session-can-reintroduce-your-fixed-lints]],
[[a-mid-build-park-can-be-committed-by-the-cleanup]],
[[a-1991-lint-fix-can-be-wrong]].

Related: [[usage-plan-amended-trajectory-shape-2026-11-02]],
[[creator-mode-plugin-host-exploration-2026-11-02]],
[[session-parity-remainders-plan-drafted-2026-11-02]],
[[session-header-parity-plan-drafted-2026-11-02]],
[[rust-1991-toolchain-move]].
