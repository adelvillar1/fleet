---
name: fleet-memory-cache-fix-plan-drafted
description: "2026-10-18 — the plan at docs/plans/2026-10-18-memory-and-cache-fixes.md is IMPLEMENTED and the gate is green (1210 passed); increments M1-M8, R1-R4, D1 all landed, KI-228..235 filed and closed"
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

After the 2026-10-18 deep dive ([[fleet-memory-cache-deep-dive-findings]])
and the recall measurement ([[fleet-recall-measured-state]]), the owner
said "let's create a plan to address all these issues" and asked whether
an n-gram and/or knowledge graph would improve recall, then said
"implement the plan as designed". **Both done.** The plan is at
**`docs/plans/2026-10-18-memory-and-cache-fixes.md`**, now `status:
complete`, and the work is uncommitted on `master`.

**Gate:** `cargo fmt --all --check`, `cargo clippy --all-targets --
-D warnings`, `cargo build --release`, `cargo test` — all four exit 0,
**1210 passed, 0 failed, 1 ignored** (the lib went 1065 → 1079). Fourteen
new tests. Two new modules: `src/recall_graph.rs` and
`src/filelock.rs` (the latter now shared by the corpus's `lock_fact` and
the count store, rather than two copies of the `flock` declaration).

**Part 1 — the eight defects, M1–M8, all closed-fixed.** M1 the index
entry now keys on `calib::hash16(&fact.indexed_text())` instead of
`secs:nanos:len`, so a recall-count bump invalidates nothing (KI-228).
M2 `Cache::rekey` clears `entries` on a producer mismatch (KI-229). M3
the cache lookup moved above the `sys1_judges()` early return, so a
cached verdict answers on a keyless machine (KI-230). M4 a failed embed
rewrites nothing — the condition is "a new vector landed or a fact
disappeared" (KI-231). M5 `nomenclature::invalidate(root)` called after
a `WriteFile` lands (KI-232). M6 `bridge::rank` takes `probed_embedder()`
(KI-233) — and its documented degradation is now honest, returning every
candidate at 0.0 rather than an empty list. M7 `/memories/*.lock` is
gitignored (KI-234). M8 the count store's bump runs under a shared
`filelock::FileLock` (KI-235).

**Part 2 — the recall fixes.** R1 `RECALL_CEILING` 8 → 128 on a
logarithmic curve. R2 the tie-break is total, then trajectory, then
name. R3 a ~30-line suffix stemmer on both sides of the lexical
channels. R4 one bounded graph hop.

**Part 3 — D1.** Six stale doc claims corrected, a recall-graph section
added to memory.md, and the corpus's own most-recalled fact
(`fleet-memories-substrate-is-one-markdown-file-per-fact.md`) rewritten
rather than superseded — it had described the pre-slice-19 four-channel
design and, at `recall-count: 103` the highest in the tree, outranked the
current design doc under the documented weights.

**Three places the implementation diverged from the plan, all forced by
evidence:** `GRAPH_DISCOUNT` was removed (it made admission arithmetically
impossible — a 0.30 source admitted nobody at `0.30 × 0.5 < 0.20`); the
`superseded-by` edge had to be re-pointed because the substrate *retires*
the fact that carries it (see [[an-edge-whose-source-is-retired-cannot-be-walked]]);
and two existing tests pinned behaviour the plan intentionally changed,
so they were updated with the reason recorded.

**Out of scope, worth not re-deriving:** an n-gram channel (declined with
the measurement); a general knowledge-graph store (the edges are
frontmatter fields the substrate already declares); cross-machine cache
sharing; TTL or LRU anywhere; and raising `MIN_RECALL_SCORE` to fix the
71% admission rate (the failures are ordering failures, not admission
failures).

**Why:** the plan is the contract, and the divergences plus the declined
options are the parts a future session would otherwise re-derive or get
wrong.

**How to apply:** the work is **uncommitted** — verify `git status`
before assuming it landed. Related:
[[fleet-recall-measured-state]] (the evidence, now with the post-fix
numbers), [[fleet-recall-bump-invalidates-the-embedding-cache]] and
[[fleet-route-cache-adopts-foreign-verdicts]] (the two headline defects
M1 and M2 fixed), and [[fleet-memory-cache-deep-dive-findings]] (the
cache inventory, now including the two new modules).
