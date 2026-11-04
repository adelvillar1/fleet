---
name: pin-sized-defects-closed-2026-11-01
description: "project 2026-11-01 — five ledger defects closed with pins in one commit (be4488a): the sweep's double lifecycle row, the signless age, the trial reason naming the passing term, the silent turn's missing failure record, the write-guard's two over-fires; gate 1,896/0"
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

Plan `docs/plans/2026-11-01-lifecycle-and-turn-fixes.md` walked complete in one
session; every row closed with its pin as the evidence:

- **KI-384** — `lifecycle::sweep`'s own record (kind rides the row) is the only
  writer; `tools_sweep`'s second pass that re-filed every applied disable with
  a `tools: ` prefix is gone. `lifecycle_row` stays for the manual
  `disable`/`enable` verbs. Pin: two candidates → exactly two rows, settled
  (see [[an-async-writer-makes-a-pin-race-itself]]).
- **KI-383** — `days_between` returns `None` on a reversed pair; `None` is the
  conservative "not old enough" both callers already treat. First cut's guard
  was inverted (see [[a-correct-order-makes-difference-negative]]).
- **KI-382** — the Trial skip names the term that HOLDS: runs cleared →
  `first declared <date>, A of D days`; age cleared → the runs; both short →
  both. The old line printed `50 of 10 runs` — an impossible-looking pair
  while age was the holder. Pin renders all three shapes.
- **KI-352** — `Record::TurnFailed { stage, error, route }` lands in the
  session log **before** the typed error propagates: stage `model` at the
  stream call, `empty` at a zero-choice 200; `route = hash16(task)` (the route
  rows' key). Bookkeeping in the replay ignore-list, the bridge's transcript,
  and the TUI's `entry_of`. This closes the "preamble then nothing" gap the
  owner's "why is this session stuck?" found (session 20261028T162708Z).
- **KI-353** — two exemptions in `guard_content`: every-part-uppercase is a
  NAMESPACE (`HOME`, `FLEET_HOME`, `MAX_TOKENS`) skipped whole; and a refusal
  whose claimed established form is written at none of the entry's own sites
  (`HashMap` stored as `Hash_Map`) treats the token as the established
  spelling — `nomenclature::term_is_written`, read on the refusal path only,
  an unreadable site counting as unwritten. The `judgeRoute` refusal stands
  beside the pins.

Close-of-record: full gate green by exit code (fmt 0, clippy 0, build 0, test
1,896 passed / 0 failed across all targets), checker 391 rows / 32 open / zero
problems. Two catches on the way: the inverted sign guard, and the async-writer
pin race — both memory'd separately. Unpushed at session end: `e7a0a21`,
`d62c217`, `4e12b01`, `be4488a` on local master.
