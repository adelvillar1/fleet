---
status: draft
created: 2026-10-24
updated: 2026-10-24
slug: embed-bounds-from-measured-tail
---

# Plan: the embed bounds, set from the measured tail (KI-249's second half)

## Context

The bounded waits around the embedder are guesses the owner has
reported biting: the probe allows 5 s, steady-state embeds 10 s
(`src/memory.rs` PROBE_TIMEOUT/EMBED_TIMEOUT — a warm local embed is
~20–30 ms, so these bind a cold or loaded server, not a warm one),
while a model call is allowed 600 s (`src/api.rs`). The row's own law:
a bound that never fires is dead weight, and one that fires during real
work is a lie about the backend — so the honest fix measures before
extending.

The measurement half landed 2026-10-24 (`embed_timing` rows on the
calibration stream: the corpus load's wall clock and fact count, wired
at the run path and the scheduled load) and this plan is its second
half, gated on the data existing. Nothing here runs until the window
criterion is met; the plan is the contract the pass executes against.

## Approach

The increments and the criteria they carry — each increment is its
criterion's deliverable, and no criterion is satisfied by any other:
increment 1 reads the accumulated data and states the tails (C1's
deliverable); increment 2 sets each bound from its measurement with the
headroom rule named, or leaves it and says why (C2's deliverable, the
plan's decision point); increment 3 re-points the row and the constants'
comments so the provenance lives beside the code (C3's deliverable); C0
is the window gate the whole plan waits on and increment 1's reader is
what names its shortfall.

### Increment 0 — the window (C0)

The pass runs only when the stream carries a measurable day: at least
20 `embed_timing` `corpus-load` rows spanning at least 24 hours, and at
least one day of routed model calls (the route rows' `latency_ms`,
which the stream has carried since slice 8). `fleet calib` already
reads days; the tail reader (increment 1) is what names the shortfall
when the window is not met.

### Increment 1 — the tails, derived and named (C1)

A small reader in `src/calib.rs` — `embed_tail(root, days) -> Tail`
with a test — that walks the `embed_timing` day files and the route
rows' `latency_ms` and returns, per kind: count, median, p95, and max.
No new surface beyond the function and its test; the numbers are the
deliverable, printed by whoever runs the pass.

### Increment 2 — the bounds from the tails (C2, KI-249's decision point)

C2 is this increment's whole deliverable: each constant is set from its
measured tail by one named rule — the new
bound is the measured max times three, floored at the warm path's ten
multiples (a warm embed at 30 ms keeps a 300 ms floor no matter how
quiet the day was) and capped at the model call's 600 s (an embed bound
past the model's own is dead weight by definition). The probe's bound
gets the same rule over its own kind's tail. A bound the rule leaves
where it was is left exactly there, and the comment says the measurement
confirmed it rather than the guess.

### Increment 3 — provenance beside the code (C3)

Each constant's comment names what it measured — the window's dates,
the row count, the median/p95/max — so the next reader does not have to
trust the number. The KI-249 row closes with the same numbers and the
commit.

### Close-out — the gate and the provenance (C4)

C4's steps, in order: run the gate's four commands and read each exit
code directly — `cargo fmt --all --check`, `cargo clippy --all-targets
-- -D warnings`, `cargo build --release`, `cargo test`; then update the
two constants' comments and `docs/known-issues.md`'s KI-249 row in the
same commit, quoting the tail reader's own output. The criterion is the
commit that shows both.

## Acceptance criteria

- [ ] **C0 — the window gate.** The pass states the window's shortfall
      by name when the stream carries fewer than 20 `embed_timing` rows
      or less than 24 hours of span, and proceeds only when met.
- [ ] **C1 — the tails.** `calib::embed_tail` returns count, median,
      p95, and max per kind from real day files, pinned by a test over
      a written fixture.
- [ ] **C2 — the bounds by rule.** PROBE_TIMEOUT and EMBED_TIMEOUT
      each equal the rule's output over their measured tail (max × 3,
      floored at 10× the warm median, capped at 600 s), or are left
      unchanged with the measurement-confirmed comment; by test where a
      constant moved.
- [ ] **C3 — the provenance.** Each constant's comment names the
      window, the row count, and the tail it encodes; the KI-249 row
      closes with the same numbers.
- [ ] **C4 — close-out.** The full gate green by exit code; the ledger
      and the plan agree.

## Files to be touched

`src/calib.rs` (the tail reader + its test); `src/memory.rs` (the two
constants' values and comments); `docs/known-issues.md`.

## Out of scope

- The model call's 600 s bound — its tail already exists in route rows
  and 600 s has never been reported biting; if the tail contradicts
  that, it is a follow-up row, not this plan's silent extra.
- The probe's retry pause (750 ms, KI-296's) — measured separately if
  it ever fires in real work.
- Any change to the `embed_timing` vocabulary itself.

## Verification

The tail reader's test over a written fixture; the full gate's four
commands by exit code at close-out; the constants' comments quoting the
reader's own output.

## Linked artifacts

- `docs/known-issues.md` — KI-249
- `src/calib.rs` — the `embed_timing` vocabulary the window reads
