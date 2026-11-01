---
status: complete
created: 2026-11-01
updated: 2026-11-01
slug: lifecycle-and-turn-fixes
---

# Plan: five pin-sized defects from the ledger's open rows

## Context

The 2026-11-01 ledger triage grouped the 38 open rows into clusters;
this plan is cluster A — five defects, each already diagnosed by its
own row, each small enough to fix with a pin in one sitting. They share
no root: three live in the lifecycle sweep, one in the turn loop's
logging contract, one in the nomenclature write-guard. The rows carry
the evidence; this plan carries the order and the pins.

## Approach

**Fix each where its row says the defect lives; every fix lands with
the test that names it.**

- **T1 — KI-384, the double-counted disable.** `lifecycle::sweep`
  records each applied disable itself (its `record` closure carries
  the kind), and `tools_sweep` then records them all again through
  `lifecycle_row` with a `tools: ` prefix. The CLI loop
  (`src/main.rs`, the second `for (kind, result) in &plan` pass) is
  the duplicate — drop it. `lifecycle_row` stays: the manual
  `tools disable`/`enable` verbs still file through it.
- **T2 — KI-383, the signless age.** `days_between` returns
  `difference.unsigned_abs()`, so a swapped pair reads as a plausible
  age. It returns `None` when the difference is negative — a reversed
  pair is unrepresentable, and `None` is already the conservative
  "not old enough" both callers treat. Both current callers pass
  earlier-first (verified at the fix); the four existing pins keep
  their order, and a reversed-order pin joins them.
- **T3 — KI-382, the trial reason that names the passing term.** The
  skip line prints `trial — N of M runs` even when the runs cleared
  the bar and AGE holds the tool in trial. The reason now names the
  holding term: runs short → `trial — N of M runs`; runs cleared and
  age short → `trial — first declared <date>, A of D days`; both
  short → both terms. The format is decided in the one place the
  Trial skip is pushed.
- **T4 — KI-352, the silent turn.** A turn whose model call produces
  no output leaves the log with a complete preamble and a gap. The
  fix appends a typed failure record — the error, the stage, the
  route row's hash — whenever a turn ends with no model output, so
  the stall is diagnosable from the log alone. The site is found by
  reading the turn loop's end paths against the row's own session
  (20261028T162708Z-997a54b4) before the code is touched.
- **T5 — KI-353, the guard's two over-fires.** The write-guard must
  (a) not rule a case/separator variant when its claimed established
  form exists nowhere in the vocabulary it derived, and (b) exempt
  environment-variable spellings — all-caps with underscores are a
  namespace, not prose drift. The anti-vector (whole-file scans and
  content-constrained files go through bash) is untouched.

## Acceptance criteria

- [x] **T1 — one row per applied disable.** *Landed: the CLI's second
      recording pass is gone; `sweep`'s own record is the only writer.
      Pinned by `an_applied_sweep_records_each_disable_once` — two
      candidates, two rows — which first flaked under the full suite
      by counting before the stream's async drain; it now settles on
      `pending()` and fails loudly on a dropped/failed row.*
- [x] **T2 — a reversed pair is `None`.** *Landed: the difference's
      sign refuses the swap. Pinned in
      `days_are_counted_across_months_and_years`; the first cut had
      the guard inverted (the correct pair's difference is negative)
      and the existing pins caught it.*
- [x] **T3 — the reason names the holder.** *Landed: three shapes —
      age, runs, both. Pinned by
      `the_trial_reason_names_which_term_holds`.*
- [x] **T4 — the silent turn leaves a record.** *Landed:
      `Record::TurnFailed { stage, error, route }` at both failure
      sites (the stream call, the zero-choice 200), bookkeeping
      everywhere it replays. Pinned by
      `a_turn_that_dies_without_model_output_leaves_a_typed_record`;
      the KI-352 evidence's dropped `write_file` result stayed out of
      scope — it is a separate seam, not this turn loop's.*
- [x] **T5 — the guard stops over-firing.** *Landed: the all-caps
      namespace exemption and the unwritten-claim refusal cut
      (`term_is_written`). Pinned by
      `the_guard_exempts_a_namespace_and_a_form_the_tree_never_wrote`
      with the `judgeRoute` refusal standing beside it.*
- [x] **Close-out.** *All five rows closed with their pins as
      evidence; the full gate green by exit code — fmt 0, clippy 0,
      build 0, test 1,896 passed / 0 failed across all targets — and
      the ledger checker reads 391 rows, 32 open, zero problems.*

## Files to be touched

`src/lifecycle.rs` (T1's record site, T2, T3 + their pins),
`src/main.rs` (T1's dropped loop), `src/agent.rs` (T4's record),
`src/nomenclature.rs` (T5's two exemptions), `docs/known-issues.md`
(the five closes).

## Out of scope

- KI-391's severity normalization (the owner's ruling).
- The corpus-refresh cluster (its own plan).
- The dropped `write_file` result the KI-352 evidence mentions
  alongside the four silent turns — a separate seam; if the T4
  investigation shows it is the same defect, it widens T4, and the
  plan says so in the close.

## Verification

The gate's four commands by exit code; each pin's own run; the ledger
checker over the real file as the last act before the commit.

## Linked artifacts

- `docs/known-issues.md` — KI-382, KI-383, KI-384, KI-352, KI-353
