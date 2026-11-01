---
status: complete
created: 2026-11-01
updated: 2026-11-01
slug: ledger-tooling
---

# Plan: the ledger's own tooling — statuses refused at the checker, appends under the lock

## Context

The 2026-11-01 warmup opened the ledger and found `ki_ledger.py` red:
eight problems, all four malformed rows (KI-385/388/389/390) filed by
the KI-385 sweep with their evidence merged into the issue cell — five
cells where the header declares six, so the checker read each row's
next action as its status and reported both the cell count and a
missing next action. The rows are repaired (`e7a0a21`), the checker is
green, and the class that let them stand is KI-322's ask: the checker
validates cell shape but not vocabulary, so a status cell reading
neither open nor closed nor noted survives until a human notices.

Two more rows in the same class: KI-331 records that the id rule
("the largest present plus one, derived from the file at write time
under the ledger's own lock") governs only the Rust path —
`file_ledger_row` — while a scheduled catch-up's hand append reads the
table, takes max+1, and edits with nothing serializing the three
steps; two runs have already shared an id live (the checker caught it
after the fact, and a sibling's vanished anchor nearly cost a row).
And this session's survey filed KI-391: the severity vocabulary in the
table is a half-undocumented extension of the preamble's six words.

This plan gives the ledger the tooling its own rules assume: the
checker refuses a malformed status at file time, and runs append
through the lock instead of by hand.

## Approach

**The checker polishes the vocabulary it already parses; the CLI
supplies the append the lock was built for.**

- **L1 — status vocabulary (KI-322).** `is_open` already strips
  emphasis to read the leading word; the same strip feeds a
  `status_word()` that must be one of `open`, `closed`, `noted`,
  `none` — the three words the table actually carries plus the
  `none` variant KI-322's next action names. A row whose leading
  word is anything else is a problem at its line, named like the
  cell-count problems, and the checker exits 1. The check reads the
  same padded cells the field checks read, so a short row reports its
  short-cell problem and never a phantom vocabulary problem on top.
- **L2 — the append verb (KI-331).** `fleet ki append` takes the
  five row cells (severity, issue, evidence, status, next action),
  and `doctor::append_ledger_row` does the write: sibling lock
  (`FileLock::beside`, the primitive `file_ledger_row` already
  holds), `next_ki_id` over the text read *under that lock*, the row
  rendered in the table's own six-column shape, atomic rename. The
  id therefore cannot be shared by two concurrent appenders — the
  lost-update shape KI-235/KI-285 named, now closed for the hand
  path too. The verb prints the id it filed so a run can cite it.
  The diagnose path keeps its own signature-keyed update semantics;
  this is the plain append beside it, not a rework of it.
- **L3 — the rules point at the verb.** The ledger's "an id comes
  from the table" rule gains the sentence that runs append with
  `fleet ki append` instead of editing the table by hand, and KI-322
  and KI-331 close with their evidence in the commit that lands their
  fix. The contract docs carry the new verb where the commands are
  listed (FUNCTIONAL-SPECIFICATIONS' verbs, TECHNICAL-DOCUMENTATION's
  interface contracts).
- **L4 — close-out.** The full gate by exit code, the checker green
  over the real ledger, and the plan's criteria walking in order.

## Acceptance criteria

- [x] **L1 — the checker refuses an unknown status.** *Landed
      `d62c217`: a scratch ledger whose status reads `in-progress` is
      refused at its line (exit 1, the problem naming row and cell);
      the real ledger stays green. The table's own joined `closed-fixed`
      shape landed in the vocabulary after the first pass split it —
      70 rows carry it.*
- [x] **L2 — the append rides the lock.** *Landed: `fleet ki append`
      derives the id under `FileLock::beside`, refuses a pipe/newline
      cell and a status outside the vocabulary at the writer, and
      prints the id. Pinned by `concurrent_hand_appends_never_share_an_id`
      (8 released together → KI-002…KI-009, every row six cells) and
      `the_hand_append_refuses_the_row_the_checker_would`; proven live
      — the verb filed KI-002 into a scratch ledger the checker read
      green, and refused an `in-progress` status by name.*
- [x] **L3 — the rule and the docs name the verb.** *The ledger's id
      rule now names `fleet ki append` and both refusals;
      FUNCTIONAL-SPECIFICATIONS' verbs table and TECHNICAL-DOCUMENTATION's
      CLI contract carry the verb; KI-322 closed at `d62c217`, KI-331
      closed here with its pins as evidence.*
- [x] **L4 — close-out.** *The full gate green by exit code — fmt 0,
      clippy 0, build 0, test 1699 passed / 0 failed / 2 ignored (plus
      the integration binaries' suites, all green) — and
      `python3 scripts/ki_ledger.py` reads 391 rows, 37 open, zero
      problems.*

## Files to be touched

`scripts/ki_ledger.py` (the status vocabulary), `src/doctor.rs`
(`append_ledger_row` + its concurrency test), `src/cli.rs` (the `ki`
parse, help line), `src/main.rs` (the dispatch), `docs/known-issues.md`
(KI-322/KI-331 close, the rule sentence), `FUNCTIONAL-SPECIFICATIONS.md`
and `TECHNICAL-DOCUMENTATION.md` (the verb's entries).

## Out of scope

- **Severity normalization** — KI-391's owner ruling decides whether
  the preamble gains the words in use or the rows normalize; this
  plan only leaves the row open with its survey evidence.
- **The corpus-refresh family** — the ~15 rows whose root is the
  scheduled jobs' spawn/runbook contract (KI-387 et al.) are their
  own plan, not this one.
- **The diagnose path's semantics** — signature-keyed updates,
  preserved status cells: unchanged.

## Verification

The gate's four commands by exit code at close-out; L1's two checker
runs (scratch red, real green); L2's concurrency test in
`src/doctor.rs` beside the existing eight-filings test; the ledger
checker over the real file as the last act before the commit.

## Linked artifacts

- `docs/known-issues.md` — KI-322 (status vocabulary), KI-331 (the
  hand-append race), KI-391 (the severity drift, left open)
- `src/doctor.rs` — `file_ledger_row`, `next_ki_id`, the FileLock
  discipline this plan extends to the hand path
