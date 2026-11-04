---
name: a-1991-lint-fix-can-be-wrong
description: "feedback 2026-11-02 — the 1.99.1 clippy suggestions are not all applicable: array_chunks is not a real method (as_chunks is), a sort_by_key closure must destructure the tuple's real arity, and let-chains are edition-2024-only while fleet is edition 2021 — a co-tenant's parked lint fixes were all three wrong"
type: feedback
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

Fixing the 41 new 1.99.1 clippy lints, the co-tenant session's parked work
carried three wrong fixes: `message.array_chunks::<64>()` (no such method —
clippy's own suggestion was `as_chunks`), a `sort_by_key(|(when, _)| …)` over a
4-tuple (E0308, expected a tuple with 4 elements), and a `if … && let Some(x) =
…` let-chain (E0728, edition 2024 only; fleet is edition 2021). The cleanup
commit had to repair all three.

**Why:** the lints are mechanical in name but each suggestion must be checked
against the actual API and the crate's edition — `chunks_exact_to_as_chunks`
suggests `as_chunks`, not `array_chunks`; `sort_by_key`'s closure receives the
whole element; let-chains are not in edition 2021.

**How to apply:**
- `chunks_exact(n)` → `as_chunks::<n>()` (returns `&[[T; n]]` plus a remainder)
  — NOT `array_chunks`, which does not exist on slices/Vec.
- `sort_by_key` closure takes the element: for a tuple, destructure its real
  arity (`|(when, _, _, _)|`), or use `sort_by_key(|one| Reverse(one.0))`.
- Let-chains (`if x && let Some(y) = z`) need edition 2024 — fleet is 2021;
  write the nested `if` or `if let … { if … }`.
- After any lint sweep, `cargo build --all-targets` is the check — a
  syntactically-legal wrong fix (the 2-tuple closure) compiles far enough to
  hide until the type error.

Related: [[rust-1991-toolchain-move]] (the move and the 41),
[[a-toolchain-move-is-a-shared-default]],
[[a-parallel-session-can-reintroduce-your-fixed-lints]].
