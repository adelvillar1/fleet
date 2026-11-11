---
name: the-resource-contract-admit-work-settle-curtail
description: "owner ruling 2026-11-09 — fleet must never take more than is available; measure, run work, re-measure, and CURTAIL runaway work (a sys1/LLM call worth its cost if it prevents a crash); the contract's numbers are LEARNED (recursive learning) from the MACHINE'S environment profile and the RISK APPETITE — which is INFERRED too (a habitually under-10%-free machine means a tight owner and makes fleet TIGHTER, never looser) — never code constants; drafted as the four-beat resource contract plan (C1–C12)"
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

Owner, 2026-11-09, going one step past the budget-and-awareness ruling
([[a-budget-per-store-beat-clean-up-after]]): *"what I don't want is fleet trying
to use more than what's available … disk is not infinite, measure, run work,
re-measure, etc, and if there is any runaway work then curtail the behavior …
does it take an llm or sys1 call for that, that's fine. an llm or sys1 call
that prevents the environment from crashing it's worth the cost."*

**Why:** measurement alone is still passive. `scripts/footprint.sh`, `fleet
doctor`'s `caches`, and `fleet footprint` observe and grade — nothing *admits*
work against headroom and nothing *settles* afterwards, so a runaway is found by
the sweep the next day. That is the same rescue shape the owner rejected.

**How to apply:**
- Every unit of work (a run, a build, a release cut) moves through four beats:
  **measure → admit or refuse → work → settle and curtail.** Deterministic
  arithmetic for "3 GiB of room, 8 GiB of work" (refuse before starting, naming
  both numbers); there is nothing for a model to judge there.
- Work declares its cost where it is knowable (release build ~8 GiB, lane
  bootstrap ~2 GiB); a run's writes are unknowable, so it gets a per-turn
  budget and turn-boundary measurement.
- **The sys1 arm fires only on an abnormal reading** (never per turn, inside the
  judge budget): "this run grew 4 GiB — the work it is doing, or a runaway?" It
  answers one word with evidence — `expected` (raise that run's recorded budget,
  so the same shape never asks twice) or `runaway` (curtail). A refused answer
  is `expected` and records itself, so a lane hiccup cannot silently kill work.
  The owner's cost licence is explicit: an LLM/sys1 call that prevents an
  environment crash is worth it.
- **BUILT (2026-11-10, owner: "approved and build it"): the deterministic half
  is live** — `fleet footprint` prints `memory` / `processes` vs their ceilings,
  the `environment` line (median standing free share → inferred appetite →
  fractional reserve, with the source named), and the disk budget table;
  `src/footprint.rs` carries `Env`/`appetite`/`reserve_kib`/`admit`/`settle`
  (6 tests); `footprint.sh --record` appends the free-space series. NOT built:
  admit/settle wiring, turn-boundary measurement, the sys1 arm, the calibration
  rows. See [[self-healing-footprint-built-contract-half]] before claiming
  fleet is self-healing.
- Landed as a plan, not code: `docs/plans/2026-11-09-the-resource-contract.md`
  (draft, `44741c2` then amended `f801da7`, `1ab0ef7`, `7db08c5`, now C1–C12;
  `in progress` at `bcc59cd`) — `Contract::admit/settle`
  pure functions in `src/footprint.rs`, the release path, per-run admission with
  turn-boundary measurement, the single sys1 call, `op=footprint` calibration
  rows, a forced-runaway test, and the recursive loop below.
- **The owner's second steer: "we can measure and calibrate — recursive learning
  is the shape that we need here."** Measurement alone is an instrument; the
  contract's numbers must be learned from its own outcomes, on the existing
  calib → fit-rule → demand-map → improve-gate machinery:
  - one `op=footprint` row per unit of work (work class, admitting estimate,
    settled delta, reserve at admission, verdict, the sys1 words);
  - **the admitted estimate is history** — the median of the last ten settles
    for that work class, the declared constant only a prior until three exist
    (a build that keeps settling at 5 GiB stops being budgeted at 8);
  - **the resource judge is graded by what happened next** — a `runaway` on work
    that then completed cleanly counts against it, an `expected` before a breach
    counts against it, the same discipline as `fleet judge --agree/--disagree`;
  - learning lands only as a gated change via `fleet improve` (nothing drifts
    silently);
  - **the safety floors never learn** — the free-space reserve and memory
    ceiling are the owner's fixed declaration; only work estimates/budgets
    calibrate, since a floor history could lower is not a safety floor.
- **The owner's third steer (the closing frame): "responsible execution is a
  matter of environment and risk. A constrained environment like this laptop
  needs better upkeep than others, and that's what fleet needs to learn."** So
  no number in the contract may stay a code constant; it derives from two
  inputs, one measured and one declared (plan C10/C11, committed `1ab0ef7`):
  - **the environment, LEARNED** from the footprint rows: volume size, standing
    free space, the fill rate (GiB/day — this sets the upkeep CADENCE), memory
    and swap pressure, laptop vs server; re-derived as rows land, arithmetic;
  - **the risk appetite, DECLARED** by the owner only — `steady` / `balanced` /
    `tight`, or the floor named outright — never inferred from behaviour
    (inferring tolerance from usage is how a system becomes cheap with the
    owner's time);
  - the reserve is the tighter of the environment's floor and the appetite; the
    cadence follows the fill rate; per-run budgets follow the appetite. The
    constants in `scripts/footprint.sh` are **priors for a fresh machine**,
    replaced by the first recorded profile plus the declared appetite.
  - A refusal must read as a decision, not an oracle — report it with its
    reasoning ("reserve 35 GiB: laptop, steady, filling 1.8 GiB/day") so the
    owner can see and change the appetite.
- **The owner's fourth steer CORRECTS the third on one point: the appetite can
  be INFERRED as well.** "If an environment is constantly under 10% of
  available space you can infer that the user likes risk and tighter controls
  on fleet are needed" (plan C12, `7db08c5`). An earlier draft said appetite is
  declared-only; that is now wrong where a declaration is absent.
  - The signal is the **standing free FRACTION**, not a moment's free space:
    the median of the last thirty daily readings as a share of the volume, plus
    the volatility (how often it nears the floor, how fast it recovers).
  - **The direction is the counterintuitive rule: a habitually full machine
    tightens fleet, never relaxes it.** Tolerance is evidence there is no
    margin to absorb a runaway, not a budget grant. So an inferred tight
    environment raises the reserve **as a fraction of the volume** (never an
    absolute a smaller disk would wave through), shortens the cadence, lowers
    the per-run budget and refuses earlier; the roomy machine earns the looser
    numbers. Getting this backwards — rewarding a full disk with more room —
    is the worst version of learning.
  - Report the inference with its evidence and let the declared setting win:
    "reserve 42 GiB: laptop, 9% free for 30 days, filling 1.2 GiB/day —
    inferred tight; say 'steady' to override". The report names which input
    (inferred or declared) produced the number.
- Risks I recorded rather than hid: a false curtail is worse than a slow run
  (hence the `expected` budget raise); self-measurement during a heavy run is a
  moving target; and 2 GiB memory / 80 processes / 20 GiB free floor are
  placeholders — priors to be replaced by the machine's profile and the owner's
  appetite, not numbers the code owns.
- Out of scope recorded in the plan: predicting a run's write volume,
  curtailing other processes, CPU and network budgets (same shape, later).

- **The owner's fifth steer (2026-11-11), applied to concurrent runs:**
  "concurrency should be allowed as long as the environment where fleet is
  running can handle it; if resources are too constrained then it should
  politely say so and tell the user that it needs to be reconsidered or
  queued." So admission is GENEROUS within the machine's capacity (the
  learned profile, never a fixed pool constant) and COURTEOUS at its edge —
  a named card offering reconsider-or-queue, not a silent queue or a terse
  refusal. Landed as the governing rule + C3 of
  [[multi-session-plan-drafted-ki-525-2026-11-11]].

Related: [[a-budget-per-store-beat-clean-up-after]],
[[build-lanes-in-tmp-eat-the-drive]], [[checkpoints-unbounded-72g-disk]].
