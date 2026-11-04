---
name: usage-plan-amended-trajectory-shape-2026-11-02
description: "project 2026-11-02 — the usage/anomalies plan amended with Increment C (the trajectory shape, C7–C9); SUPERSEDED THE SAME DAY — the whole plan LANDED and its nine criteria tick with evidence (140c739), after A 0253e89 / B 7aa0544 / C 43b37e5; survives as the plan's record and the 'check the tree, not the boxes' lesson"
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

The owner asked (2026-11-02, after the harness-paper comparison) to "create the plan" for the trajectory-shape idea. Amended `docs/plans/2026-11-02-usage-in-calibration-and-stream-anomalies.md` (the offloop-article plan — **now `status: active`**, flipped from draft) with **Increment C — the run's shape rides the stream**, APPENDED as criteria C7–C9 (criteria order is identity; never reorder after gating):

- **C7 — the curve is captured, not re-derived**: `RunReport` carries per-turn (prompt, completion) snapshots + the compaction count, using `maybe_compact`'s rule (`last_prompt_tokens`, `src/agent.rs:2630` — the reported-or-estimated prompt size computed every turn and currently discarded) — never a second estimate rule.
- **C8 — the shape rides the vocabulary**: a `ShapeRow` written once per run beside the route row, serde-defaulted on the `judged_from` pattern; pre-field rows read absent.
- **C9 — the surface answers the rot question**: `fleet calib --shape` — per-run curve cards + a day closing card (median per-turn growth, doubled runs, compactions, max-turns endings). Numbers and names only: **no sys1 ask, no anomaly fired**; a doubled-context anomaly class is the named follow-up, written only if the day cards earn it.

Out of scope, named: any change to when compaction fires (`maybe_compact` untouched — the curve measures); the paper's Levenshtein/Jaccard similarity apparatus (training-eval tooling); per-delegation curves. Live leg: one real scratch-home run writes its ShapeRow and `--shape` reads it back — the Zhang/Khattab context-rot question asked against fleet's own traffic for the first time. See [[harness-compositional-generalization-paper-vs-fleet]] (the repo corpus carries the full adopt/reject mapping).

**State, verified 2026-11-02 against the tree and the commits (the
owner's "check against commit history").** **C0–C2 are built but
UNCOMMITTED**: `Usage`
carries `cache_read_tokens` / `cache_creation_tokens` /
`reasoning_tokens` as `Option`s with `billed_tokens()` (C0);
`RouteRow.usage` + `RouterState.usage` set from the serving attempt
(C1); `ReadRow.usage`, the `tokens:` line and the closing `spend` card,
each rendered only when usage rides, with a surface test over a
new-shape and an old-shape row (C2). **C3–C9 are not started** —
`src/anomalies.rs` does not exist and `ShapeRow` appears nowhere in the
tree or on master. All ten boxes are still `[ ]` and there is no
status-pass table yet. `git log master..HEAD` is **empty**: the branch
`calib-usage-anomalies` points at master's tip (`3ebf1fe`), so the
whole C0–C2 body is one `git checkout` from lost and the gate has not
been run on it. The two live legs (a real provider call read back from
the stream; `--anomalies` over a real past day) are still owed.

Gates: arch-gate **PASS stable** after one real fix (see [[a-hedged-cross-module-seam-row-drifts-the-arch-gate]]); plan-gate flagged C7/C8/C9 MISSING at p=0.37–0.45 — the same trio blind spot as every plan gated that session (each carries its own approach step, verification block, and files rows) — waived with the standing reason. plan-surface order C0→C9 matches the checkboxes.

**SUPERSEDED THE SAME DAY — the plan LANDED WHOLE.** The owner's *"check
against commit history"* was right: by the time the research turn ended, the
co-tenant session had committed **Increment A** (`0253e89` — the usage rides
the route rows), **B** (`7aa0544` — `src/anomalies.rs`, the stream's anomaly
pass), **C** (`43b37e5` — the trajectory shape), the docs (`0259bb4`), and
**ticked all nine criteria with their evidence** (`140c739` — "the usage
plan's nine criteria tick with their evidence — completed"). The state
paragraph above is the snapshot as it was written; the plan file and the
commit log are the state. The lesson is the one the owner was pointing at,
and it is now its own memory: [[a-memory-index-line-is-not-the-plans-state]]
— a `[ ]` box and a MEMORY.md hook are both snapshots, and the tree moves
under them.

**The landed shape, verified in the tree (2026-11-02).** Increment A:
`Usage` carries `cache_read_tokens`/`cache_creation_tokens`/`reasoning_tokens`
as `Option`s with the providers' own wire names
(`cache_read_input_tokens`, `cache_creation_input_tokens`) and
`billed_tokens()` as the one reading (a subset of prompt, never an addition);
`RouterState.usage` is set from the serving attempt and `RouteRow.usage`
rides it, absent when the walk never served; `ReadRow.usage`, the route
card's `tokens: N billed · M from cache` line and the closing `spend` card
render **only when usage rides** (a day of old rows renders exactly as
before — the pin covers both shapes). Increment B: `src/anomalies.rs`'s six
detectors each fire on their scripted day and stay silent on a quiet one
(`a_quiet_day_finds_nothing`), thresholds as named consts, and
`the_pass_asks_at_most_twice_and_never_on_a_dry_run` pins the counting seam
(two asks on a firing day, zero on a dry run). Increment C:
`RunReport.turn_curve` (one reported-or-estimated snapshot per turn, the
`maybe_compact` reading), `ShapeRow` written beside the turn row, and
`fleet calib --shape`'s curve cards + the closing rot question. Two folds
with credit landed the same evening (`fbb47e6`): the co-tenant's C8
candidates fix in `capabilities.rs` and the `Server.heartbeat` visibility
their `gui_sse` test needed. Gate 1,989/0 at that commit; the trunk landing
is `c8dde93` at 1,996/0 — see [[session-recap-2026-11-02-session-parity-and-plugin-host]]
(the session's close-of-record; [[session-recap-2026-11-02-two-plans-landed]]
is the earlier, narrower recap).
