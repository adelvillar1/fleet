# Recap — slice 13: the eval corpus and self-diagnosis

Date: 2026-10-14 · Slice 13 of `docs/plans/2026-10-13-fleet-everyday-harness.md`.

## What landed

Three mechanisms, one slice, closing the self-improvement ladder's
"self-diagnose" rung.

**The eval corpus** (`src/eval.rs`, new). A fixed, graded, deterministic
suite of six scenarios — the fork's three named families (session-corpus
replay, agent-continuation, conversation-fold) plus the everyday shapes
the crate already proves (two-turn tool loop, subagent delegation, memory
recall). Each is a scripted mock-server run with asserted outcomes,
driven by a plain single-endpoint client so no run can reach the live
API (KI-006's smoke-safety applied structurally). `fleet eval` runs it,
prints house cards, and lands one `eval_corpus` row; `fleet eval --last`
reads it back. The corpus rides the slice-11 scheduler's own hook on a
daily cadence.

**The improve-gate hook** (`src/improve.rs`). A source edit (a `patch`
change) runs the corpus as a gate step after the four cargo steps, and a
regression refuses the apply with the scenario named — the AC that closes
the loop with slice 12. Driven deterministically through a stand-in
probe; production reaches the corpus through a `fleet eval` subprocess so
the gate grades the freshly-built binary and never blocks on the async
runtime.

**Diagnosis** (`src/doctor.rs`). Per-layer probes name the roster, the
ledger, and each MCP core with its reason; the failure classification
maps the crate's typed errors to the plan's five words (network / auth /
quota / decode / tool); and `fleet doctor --diagnose <session>` drafts an
evidence-first recap and files (or, without `--ledger`, names) its ledger
row, append-or-update by the failure's signature — the human-owned
`docs/known-issues.md` is written only where a path is pointed.

## What was ported, and what was deferred

Ported from the fork (read-only, never a dependency — hard rule 6): the
fixed-corpus shape, the per-scenario grading, the score-as-a-stream-row
idea, and the layer-probe / evidence-first-drafting discipline. Deferred
and logged: the fork's median-timing budgets (the corpus grades outcomes,
not latencies) and sdm1's turn-duration prediction (KI-107 — needs a
training set this crate does not collect).

## Gate

Local, warning-free: `cargo build --all-targets` → 0 warnings, exit 0;
`cargo test` → 438 unit + 7 (tests/improve.rs) + 2 (tests/eval.rs) + 90
integration pass, 1 ignored (the live-API e2e), 0 failed. `fleet eval`
smoke-verified green end to end (six scenarios, score 1.00) and
`fleet eval --last` reads the row back. rustfmt/clippy were not run
(KI-007); the existing style was matched. The repo's tracked `memories/`
and `docs/known-issues.md` were not dirtied by the test run (KI-091's
rule) — the only ledger change is this slice's intended KI-104–KI-107
append.

New ledger rows: KI-104 (the eval gate's production path is unexercised
here until rustfmt/clippy land), KI-105 (the eval latch is in-memory), KI-106
(the diagnose default ledger path is a cwd walk), KI-107 (sdm1's duration
prediction deferred).
