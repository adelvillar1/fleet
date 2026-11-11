---
name: the-judgment-beat-socket-guard
description: "project 2026-11-07 — judge::tests::no_synchronous_path_references_the_judgment_beat text-scans a named module list (memory.rs, agent.rs, tools.rs, calib.rs, route.rs, tui/*…) for `judge::`/`crate::judge`; lane-consuming machinery for scheduled work goes in its OWN module (src/dream.rs), reachable only from scheduled threads/CLI/face"
type: project
date: 2026-11-11
basis: inferred
tier: durable
shape: []
phrases: []
missing: []
premises: []
recall-count: 0
superseded-by:
valid-until:
source: zcode:fleet:content:285983102772d99aaed2c496d57acdf6f320aad7347d3891e9bb9e2378839d34
---

The beats' socket guards are TEXT scans: `judge::tests::no_synchronous_path_references_the_judgment_beat` (and the delegate/explain siblings) carry a hardcoded module list (`include_str!` of doctor.rs, memory.rs, agent.rs, tools.rs, session.rs, calib.rs, route.rs, compaction.rs, cron.rs, improve.rs, boards.rs, skills.rs, subagents.rs, tui/*) and assert none of them contains the strings `judge::`, `crate::judge`, `JUDGE_BUDGET_ENV`, `JUDGE_ESTIMATED_ASK_USD`. The intent: "a beat that spends a lane ask must be unreachable from a path that runs on every turn."

Putting `DreamValidator` in judge.rs and calling it from `memory.rs`'s scheduled thread tripped the scan even though the scheduled thread is NOT a per-turn path. The honest fix was architectural, not a test edit: the lane-owning validator moved to its own `src/dream.rs` (registered in lib.rs), whose doc states the guard relation — memory.rs runs the pass LANE-FREE (the judge is an injected `FnMut` closure), and dream.rs is reached only from the scheduled thread, the CLI verb, and the face runner, the seams that already own scheduled lane work. This also makes the C0 fixture test possible: the pass takes scripted verdicts with no lane at all.

**Why:** the guard list is string-based, so a legitimate call from a non-per-turn path in a scanned module still reds it; the fix is module placement, not weakening the scan.
**How to apply:** any new machinery that spends lane asks outside a run (scheduled passes, CLI beats) lives in its own module outside the scanned list, and the scanned synchronous modules reference only the injected-closure seam. Related: [[memory-dreaming-plan-implemented-2026-11-07]], [[sys1-vendored-tasks-py-seam-and-the-python39-trap]].
