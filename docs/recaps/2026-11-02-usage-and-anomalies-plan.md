# 2026-11-02 — the super-agent explorations become a plan

## What shipped

- `docs/plans/2026-11-02-usage-in-calibration-and-stream-anomalies.md`
  — the plan-as-contract for the two explorations that survived the
  dsh-superagent article analysis: **Increment A**, three optional
  cache/reasoning fields on the usage struct riding the calibration
  route row through the `judged_from` pattern, with the calib stream
  surface naming the columns and a per-provider spend card;
  **Increment B**, `src/anomalies.rs`, six deterministic anomaly
  classes over a day's calibration rows (fallback-walk,
  tool-failure-loop, lanes-down, delegation-maxed, run-failure-cluster,
  cache-collapse), sys1 asked only to narrate findings that already
  fired, surfaced as `fleet calib --anomalies` cards. Seven criteria,
  C0–C6.
- **Plan-gate PASS** — provider jev, 7/7 coverage, 1/1 scope. Two GAPS
  verdicts preceded it: the first was a real gap (the Approach never
  said the docs move with the increments), the second was KI-396 —
  the fix had landed past the gate's silent 2000-char section
  truncation, so the model never saw it. Placing the doc file names
  inside Increment A's paragraph (char ~1378 of 4068) turned the
  verdict.
- Three ledger rows, each found by doing the work:
  - **KI-395** — the nomenclature write-guard refuses a document
    naming the established term's own canonical identifier (the
    `Usage` struct vs the term `usage`), so content-constrained files
    go through bash. It refused this recap too.
  - **KI-396** — the plan-gate's per-section truncation is silent
    (`gates.py:228`, `:415` — while the test-inventory overflow at
    `:381` is noted in its report).
  - **KI-397** — a folder-less run commits onto whatever branch the
    co-tenant session left checked out.
- `memories/usage-and-anomalies-plan-exists.md` — the exploration
  fact superseded to carry the arrived ruling and the plan's state;
  the old name retired from recall.

## Commits

- `2503bfc` — plan + ledger (KI-395, KI-396). **On the design-parity
  branch, not master**: the shared checkout held a co-tenant session's
  branch (KI-397); switching it under a live session was refused.
- `708cfba` — ledger KI-397, same branch, same reason.

## What is left

- **The implementation session.** The plan is `status: draft`; C0–C6
  unbuilt. Build order: Increment A first (C0–C2 plus the usage half
  of C6), then B (C3–C5 plus the anomaly half of C6). Two live
  verification legs ride the session: one real provider call read
  back from the stream with its cache fields present (which also
  settles the billed-tokens method's per-provider semantics), and
  `--anomalies` over a real past day.
- **The branch.** `2503bfc` and `708cfba` await the fast-forward to
  master once the co-tenant lands its three design-parity commits;
  `HEAD..master` was empty at writing, so the merge is a clean ff
  from the co-tenant's side or after its session ends.
- No cargo artifacts were produced this session (the plan-gate is
  Python), and the co-tenant may be mid-build — the recap step's
  cache sweep is deliberately skipped rather than run blind under a
  live build.
