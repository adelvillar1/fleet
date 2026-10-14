# Slice 17 — the explanation beat: fleet explains its own verdicts (2026-10-14)

**Plan:** `docs/plans/2026-10-14-fleet-explain-the-verdict.md`.
**Feature:** `docs/features/explain.md`. **Ledger:** KI-163..KI-171.

## What landed

`fleet explain --proposal <name> | --eval <scenario> [--dry-run]` — the
second beat on every verdict that today names its failure without its
cause. The command reads the verdict's row from the calibration stream,
composes one ask to the vendored blend1 lane from the verdict's
evidence plus the judged artifact, and renders a card; Ctrl+X opens the
same card in the TUI. The verdicts covered: the improve gate's per
proposal (the failing or refused step, the per-step evidence —
including the refusal that fires most on a machine without rustfmt) and
the eval corpus's failed scenario.

**The discipline is the port, and the point** (dev-decisions' synthesis
lane, ported; fleet's beat is native through its own vendored lane, no
CLI dependency):

- **eval-only.** The explanation never gates, blocks, or applies
  anything. The verdict stands exactly as recorded; the row carries
  `eval_only: true`; no verdict reads the `explain` op. The socket-guard
  test is the proof — it reads eleven modules' sources and asserts none
  names the beat, and that only calib and explain construct the row.
- **batch-only.** No synchronous hook path references the module.
- **`--dry-run` spends nothing** — no ask, no row, no file.
- **artifact integrity.** The beat runs against the recorded artifact;
  a sha mismatch is a named refusal (explaining a drifted artifact is
  worse than not explaining). The verdict rows gained the identity the
  beat keys on: the staged content's sha for a proposal, the corpus's
  fixed definition for the eval.
- **recoverable text.** The prose lands at
  `<home>/explanations/<input_sha256>.md`, written atomically and
  hash-verified before the row lands; the row and the session log stay
  redacted — no prompt or answer text enters either.
- **budget.** `EXPLAIN_BUDGET_USD_DAY = 0.50` (~275 synthesized asks at
  the sibling's measured rate), with `FLEET_EXPLAIN_BUDGET_USD_DAY`
  able only to lower it; a failed ask's spend rides a home-side
  sidecar so the cap moves even when the calib append fails.

**PS-C rides along:** the improve apply refuses a proposal whose
target's content drifted since the draft, beside the existing
dirty-tree guard — the same integrity rule on the apply path.

## Evidence

- Gate, parent-run after each commit: `cargo build --all-targets` exit
  0, **zero warnings**; `cargo test` exit 0 — **546 lib + 6 blend1 + 2
  eval + 7 improve + 11 install + 90 integration, 2 ignored** (baseline
  537 lib preserved and exceeded; +9 lib tests; nothing deleted).
- The critical's fix, reproduced independently by the parent against
  the real vendored lane with `LIQUID_API_KEY` forced empty: the beat
  answers *"the generative lane failed open and its fallback is not an
  explanation: generate lane is not configured…"*, **exit 1**, **no
  explanations file, no explain row** — the verdict standing.
- The real-binary surface: the dry-run card at 120x40 and 80x24 (every
  drawn line ≤ 72 by a deterministic probe); a drifted artifact and an
  unknown selector exit 1 with named refusals; an over-budget day
  refuses naming the spend while a dry run still rehearses.
- The reviews: spec REQUEST_CHANGES on one half-criterion (the `--eval`
  path's coverage — fixed) plus two housekeeping notes (this recap and
  the plan's status); quality REQUEST_CHANGES with one critical, three
  importants, ten minors — the critical and the importants fixed, four
  minors fixed, five logged with their next actions.

## What the reviews caught

The quality review's critical was found by running the real thing:
against the live vendored lane with no key, blend1 answered with its
own *fallback error prose* — and the beat wrote it to the explanations
file, indexed it with a row, and exited 0. The beat had fabricated an
explanation out of the lane's error, the exact silent degradation this
slice exists to prevent. The fix parses the lane's `generated.status`
and `route.status` through and treats a non-ok generative leg as a
named refusal. The three importants were equally real: the TUI modal
rendered the card's ANSI escapes as literal text and clipped it (47 of
72 columns — the KI-140 class); the recoverable text was written
non-atomically and a second explanation silently overwrote the first;
and an ask whose row failed to land spent money the budget never saw.

The parent's own verification of the fix hit the KI-164 class once
more (KI-171): a hand-written fixture row missing a required field was
dropped silently by the reader, so the downstream refusal read as "the
scenario has not failed today" while the row sat on disk. The fixture
was wrong, not the reader — but the lesson is logged: assert a
hand-written row reads back before drawing conclusions from a refusal.

## Not done

- **PS-B, the calibration loop closing** — dev-decisions' judgment lane
  judging fleet's route rows and the judgments riding back into the
  router's selection. The next slice's named subject.
- KI-166..KI-170: the corpus identity's version coupling, the budget
  read's 512-row bound, the stale verdict pairing, the sha-less row,
  and the double-printed refusal reason — each logged with its next
  action.
- The doctor's beat (its artifact is not yet hashed) and the boards'
  explanation feed.
