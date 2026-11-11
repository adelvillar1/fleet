---
name: the-build-loop-ux-verdict-2026-11-07
description: "feedback 2026-11-07 — the owner's UX verdict on the goal run's long silent compile-heavy batches (\"do you think this is a good user experience? we clearly need to fix whatever is broken with this process and fix it for good\"); three root causes, and the fix package LANDED at 701e256"
type: feedback
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

After the /steer "why is this taking so long?" (see
[[verification-depth-matches-the-change]]'s 2026-11-07 section for the cost
form), the owner escalated to a process verdict: "do you think this is a good
user experience? we clearly need to fix whatever is broken with this process
and fix it for good." Then: "yes, let's fix it."

**The diagnosis given (three root causes):**
1. **The repo's structural guards are invisible until they bite.** Fleet
   enforces cross-cutting invariants with text-scan tests and named constants
   — the judge/delegate/explain socket guards, `MAX_DEFINITION_TOKENS`, the
   calib `OPS` list assertion, harness count checks — and none are surfaced
   anywhere a session looks before editing. All four were hit by failing the
   suite, each costing a compile cycle to learn.
2. **The expensive loop was spent on failures readable in advance.** Each
   `cargo` invocation on the 70k+ line crate is a minute-plus compile; the
   pins on the edited surfaces were findable with seconds-cheap greps that
   were never run.
3. **Part is inherent** — 1,944 tests, a four-leg gate, a monolith crate.
   The fix makes failures preventable, not compiles free.

**The fix package — LANDED at 701e256 (2026-11-07):**
- `docs/architecture/structure-guards.md` — the inventory, six guard classes
  (A socket scans + the row-construction restriction, B tool-surface bounds
  + the move protocol, C calib row discipline, D request-path/cold-build
  split, E struct fan-out sites, F harness counts), each with trigger and
  pin home — every entry read from the guard's own source.
- `scripts/edit-preflight.sh` — grep-based, seconds, no compile; names the
  guards the changed files meet plus the exact pin command, with a LIVE
  grep for beat references in scanned modules; smoked to name all six.
- `~/.agents/skills/house-build-loop/SKILL.md` — the habit: pins before
  edits, preflight before the first compile, scoped runs while iterating,
  one full gate at the end, one progress line between every compile cycle
  (the narration rule — silence reads as stuck).
- CLAUDE.md's session protocol carries one line pointing at both, and the
  architecture index names the doc. NOTE: CLAUDE.md was mid-edit by the
  parallel session — the first Edit was rejected stale; re-read before
  editing shared files.

**Why:** the owner experienced an hour of near-silent batches and had to ask
why it was slow; the knowledge to avoid most of the loop already existed in
the repo, in the one place discovered by being wrong.
**How to apply:** every large build session now: preflight the touched-file
list BEFORE the first edit, read what it names, iterate with scoped test
runs, narrate between compile cycles, gate once. The paused dream-plan work
([[memory-dreaming-plan-implemented-2026-11-07]]) finishes under this loop.
Related: [[verification-depth-matches-the-change]].
