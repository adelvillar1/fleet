# The audit: the leg measured, the rows trued against commit history

**2026-10-26** · plan `2026-10-19-sys1-extraction-leg.md` audited against
commit history (the owner's challenge: "the extraction leg should be
done, most of the KI items should be closed") · E0 ran live for the
first time and its numbers landed · KI-289 closed by row repair · the
ledger checker green (**321 rows, 5 open**).

## The verdict the audit produced

**The leg is not done, and the status field was right to say active.**
E1 and E2 landed twice over (the Clairic build `ae5675a`, then the
owner-ruled re-point rebuild `6b9f3af`), but commit history held no E0:
`tmp/extract-probe/` did not exist — the harness was committed runnable
and never run. This session ran `scripts/extract-probe.sh` against
`~/.fleet/.env` (the key the machine already holds since the re-point
dissolved KI-312): **locate 200 in 1.26 s**, **verify 200 in 0.57 s**,
all four fields answered at 0.94–0.99 confidence, shapes captured under
`tmp/extract-probe/` (gitignored, key-free). The wire reconciles
against `src/extract.rs` exactly — entities keyed `entities.{field}`
with `{text, confidence, start, end}`, verify keyed `answers.{name}`
with `type/noul/confidence`, and `p_true` reads `noul` first. E0-C1/C2/C3
**met**, recorded in the plan the same turn. E1-C4 (the doctor probe)
and E3–E6 have no code; the plan stays active.

**The KI count was five, not the four my first scans reported — and the
fifth was hiding in plain sight.** The naive split-on-pipe scans missed
KI-249 (its cells contain literal pipes) and misread KI-289, whose close
had landed in the **next-action cell** while the **status cell** still
carried the old ruling text. The checker's own parser
(`scripts/ki_ledger.py`, code-span- and escape-aware) was the
authoritative instrument both times. Repairs:

- **KI-289 → closed (row repair, not new work).** The fix is real: the
  `#/file/<path>` hash route in `showRoute` (named "The deliverable
  address (KI-289)" in `src/gui/assets/app.js`), landed in `74d97be
  feat(face): the deliverable address`. The row had said neither open
  nor closed — its close sat in the next-action cell while the status
  cell carried the old ruling. It now says closed where the checker
  reads it, and names the route, the commit, and the residual gap (no
  face-harness pin covers the route). The open set is unchanged at
  five: **KI-249, KI-252, KI-316, KI-317, KI-320.**
- **KI-320 → open, remainder named honestly.** Remedy (b)'s workspace
  facts DID land: `render_workspace` + `apply_home` in
  `src/subagents.rs` render branch, committed, merged/conflict, and the
  held-at-conflict note, verified before printing. What did not land:
  the render names **no project and no `fs read main /<path>` command**
  — the store address a parent needs to verify a child's path in one
  step. The row carries that split.
- **KI-252 stays open honestly**: the corpus is 70 facts against the
  500-fact re-measure floor the row itself set.
- **KI-316 stays open honestly**: `SEED_MANIFEST` is still a 3-tuple;
  the durable fix (the source set riding the manifest) has not landed,
  and the `seed_missing` reversion path still threatens the two fixed
  jobs.

## The lesson the audit itself taught

Twice this session a hand-rolled parser disagreed with a committed
checker, and twice the committed checker was right. The house rule that
surfaced it: **the tool that defines the format reads the format** —
`ki_ledger.py` owns the ledger's grammar (code spans, escaped pipes,
the emphasis-stripping `is_open`), and any ad-hoc count that disagrees
with it is a bug in the ad-hoc parser, not a stale count. The warmup's
"5 open" was correct on the first read; my re-scans were the drift.

## The state

- Plan: **active** — E0 met (measured 2026-10-26), E1 landed minus C4's
  doctor probe, E2 landed, E3–E6 next.
- Ledger: 321 rows — 312 closed, **5 open** (KI-249, KI-252, KI-316,
  KI-317, KI-320), 4 noted.
- The next increment: E1-C4's doctor probe, or E3 (the calibration
  loop's inheritance, which also pays KI-249's window).

## The commits

```
d29df09 docs(audit): E0 measured live, KI-289's close landed where it belongs
```
