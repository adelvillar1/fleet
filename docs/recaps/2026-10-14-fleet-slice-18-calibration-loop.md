# Slice 18 — the calibration loop closes: fleet's routes are judged (2026-10-14)

**Plan:** `docs/plans/2026-10-14-fleet-calibration-loop.md`.
**Feature:** `docs/features/calibration-loop.md`.
**Ledger:** KI-176..KI-194.

## What landed

The loop closes: the day's route rows are judged against their outcome
evidence through the vendored sys1 lane under the ported discipline,
the judgments ride back as rows, and the failover's preference consults
them conservatively.

- **The judgment op** (`src/judge.rs`, ~1,300 lines): batch-only
  `fleet judge [--day <date>] [--dry-run]`, one bounded ask per route
  row, the lane is the router's own (sys1 / `sys1_classify` / route,
  resolved through `lanes::spec_for` — no dev-decisions runtime
  dependency). Every failure named: an unconfigured lane, a spawn, a
  handshake, a missing tool, a refused call, an unstructured answer,
  a fail-open.
- **The three-way verdict** — the owner's three vector kinds: `right`,
  `wrong` (drove away from the minima), and **`insufficient`** (the
  direction was right and the minima was unreached for want of a
  companion). The third state came from the owner's own formalization
  and rode its own commit (`f55be4c`) after the first landed. Three
  insufficient signals: a failover followed the served turn on an
  AVAILABILITY class with a companion serving (previously graded right,
  hiding the companion absence), the answer needed the escalation leg
  (the row's own sys1 verdict was `escalate`), and the attempt succeeded
  but the task did not complete (`turn_stalled`). Wrong outranks
  insufficient; the two negative states are never folded.
- **The ported discipline**: eval-only with the socket-guard test,
  batch-only, dry-run free, a named budget (`JUDGE_BUDGET_USD_DAY`
  0.05, config can only lower), the spend sidecar landing even when the
  row write fails, the task's text never entering the judgment (the
  rows are redacted by construction and the prompt says so).
- **The per-CELL calibration surface** (`fleet calib --calibration`):
  provider × workload shape, never the provider average — the framing's
  keying, which the plan's first wording had wrong. Cards at the house
  bar with status dots, the three rates kept apart, the fit state
  (8 rows / 0.30 span / both floors 0.50) and what an unfit cell still
  needs.
- **The conservative ride-back**: the failover's preference consults
  the wrong-rate alone, never at or before the start index, only when a
  healthy later candidate exists, config-gated with the off-state
  restoring the pre-slice behavior exactly. The insufficient-rate
  **never skips** — it names the companions in the route row's reason.
  The declared chain and its head are untouched.
- **The human seam**: `--agree` / `--disagree` / `--waive`, landing a
  later append-only row; a disagreed insufficient inverts to right,
  because the owner's grading is the ground truth that the companion
  was there.

## What is verified

- The gate on the merged tree: `cargo build --all-targets` exit 0,
  **zero warnings**; `cargo test` exit 0 — **734 passed, 2 ignored**
  (618 lib + 6 blend1 + 2 eval + 7 improve + 11 install + 90
  integration); `memories/` clean.
- The real binary against the unconfigured lane: exit 1, one named
  refusal, zero rows landed, the stream unchanged — the
  consuming-lanes rule's outcome confirmed live. Dry run: exit 0, no
  ask, no row. Unknown key: exit 1 with the named refusal card.
- The PTY captures (taken through `pty.fork` — tmux cannot start from a
  subagent's terminal on this machine, KI-178): the calibration cards
  with the third state at 120x40 and 80x24, and the judge batch card
  re-taken after the round-two fix, showing all three verdict colours
  and a tally that agrees with its rows. 61 lines, max plain width 72
  in both sizes.
- The reviews: spec REQUEST-CHANGES (C7/C8 partial — the batch card's
  verdict rendering and three false doc claims); quality
  REQUEST-CHANGES (one important — the same card defect, found
  independently by both — plus the fit rule's false arithmetic, the
  zero-confidence row, the eval scenario's accidental fit leg, and the
  iteration cap's residue). All fixed in `294eeae`.

## What the reviews caught

Both reviewers found the same defect, and the iteration cap is why:
`judged_lines` mapped anything that was not `wrong` to `("right", OK)`
inline, never calling the `verdict_style` helper that already existed —
so a batch whose eight rows all graded insufficient printed
`right <provider> for shape <shape>` in green directly above the same
card's `0 right · 0 wrong · 8 insufficient`. The third state's colour
never reached the beat's own headline surface, and the judge-card
captures predated the prompt change that introduced it, so a session
that captured everything else never saw the card.

Behind it, three honest findings worth keeping: the fit rule's stated
arithmetic was false in both directions it argued (three-of-eight is
0.375, so the first unweighted crossing is *four* — and in the weighted
regime three confident wrongs *do* cross the floor; the thresholds are
defensible, the reasoning was not); a zero-confidence row was a
zero-weight row, so a half-wrong cell could skip a provider on the
strength of four rows that reported no confidence (now refused by name,
with the count rendered); and the eval scenario's fit leg was
accidental — the mock re-served its first answer to every ask, so the
beat's own confidence never reached the cell math. That last one is
fixed by content-keyed script entries, which the parent applied in this
slice's closing commit.

## Not done

- **KI-191**: the real vendored sys1 lane answers every tools/call with
  a JSON-RPC `result.error` (`can only join an iterable`) for any
  non-empty text, and `src/mcp.rs`'s `parse_call` reads that as empty
  content with `is_error` false — the lane's own error is dropped and
  the refusal names a parse failure instead. The rule's outcome holds
  (no row, no influence, nonzero exit); the fix is to have `parse_call`
  read a JSON-RPC `result.error` as a named lane error.
- KI-188: the ride-back's read is bounded by `READ_CAP` (512 rows), so
  on a heavier day a cell's earlier judged rows fall out of the tail and
  the ride-back silently stops informing.
- KI-190's follow-through is landed (the content-keyed mock and the
  re-pointed legs); the remaining named items are the second-disposition
  idempotence (KI-192) and the sidecar's shared machinery still having
  no test outside the beat.
- The live paid batch (no live ask has been made; the budget's
  arithmetic is verified by construction and unit tests).
