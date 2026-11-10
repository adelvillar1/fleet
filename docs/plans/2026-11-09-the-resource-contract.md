---
status: in progress
created: 2026-11-09
updated: 2026-11-10
slug: the-resource-contract
---

# Plan: the resource contract — admit, work, settle, curtail

## Context

The owner's ruling (2026-11-09): *"I don't want fleet trying to use more than
what's available. Disk is not infinite: measure, run work, re-measure — and if
there is runaway work, curtail the behavior. If that takes an LLM or sys1 call
to prevent the environment from crashing, it's worth the cost."*

What exists today is **measurement, not a contract**: `scripts/footprint.sh`
declares budgets and grades stores, `fleet doctor` reports them, and
`fleet footprint` grades this process tree against memory/process ceilings. All
of it is observation. Nothing *admits* work against the headroom, and nothing
*settles* afterwards — so a runaway is found by the sweep the next day, which is
exactly the pattern the owner rejected.

## The contract

Every unit of work (a run, a build, a release cut) moves through four beats:

1. **Measure.** Free space (instant, `statvfs`), the process tree's memory and
   count (`ps`, ~5 ms), and the declared reserve — the floor work may never eat
   into (default: 20 GiB free, 2 GiB RAM).
2. **Admit or refuse.** Work declares an estimated cost where one is knowable
   (a release build: ~8 GiB; a lane bootstrap: ~2 GiB; a run: unknown → the
   run's own per-turn budget). If `headroom < estimate`, the work is **refused
   before it starts**, naming both numbers and the cheapest remedy. This is
   deterministic; there is nothing to judge about "3 GiB of room, 8 GiB work".
3. **Work.**
4. **Settle and curtail.** Re-measure. A delta over the work's declared budget
   is a **runaway**: stop the work, keep its artifacts for the diagnosis window
   (the `.failed-build` rule), then clean what that work created, and land a row
   the calibration stream can cluster. Deterministic for the bounds.

## Where the judgment goes

One question has no arithmetic answer: *"this run grew 4 GiB — is that the work
it is doing, or a runaway?"* That is sys1's, and the cost is justified exactly
when a crash is otherwise likely:

- It fires **only on an abnormal reading**, never per turn (a bounded call, the
  existing judge-budget rule).
- It receives the numbers and the evidence: pre/post measurements, the delta,
  the run's last turns' tool calls (what it was writing), and the declared
  budget.
- It answers one of two words — `expected` or `runaway` — with a sentence of
  evidence; `runaway` curtails. `expected` raises that run's recorded budget so
  the same shape does not ask twice.
- A refusal to answer is `expected` for the interval and lands a row: never a
  silent stop on a lane hiccup.

## The recursive loop (the owner's shape, 2026-11-09)

Measurement alone is an instrument. *"We can measure and calibrate — recursive
learning is the shape we need"* means the contract's own numbers are learned
from its own outcomes, on the machinery this repo already runs:

- **One row per unit of work**, in the calibration stream (`op=footprint`): the
  work class (`release-gate`, `release-assets`, `run:<folder>`, `lane-bootstrap`),
  the estimate that admitted it, the settled delta, the reserve at admission,
  the verdict (admitted, refused, curtailed, expected), and the sys1 judgment's
  own words when one fired.
- **The estimate is history, not a constant.** A work class's admitted cost is
  the median of its last N settles (N = 10); the declared constant is the prior
  until three settlements exist. A release build that keeps settling at 5 GiB
  stops being budgeted at 8 — that self-correction is the recursion, and it is
  arithmetic over rows, not a model.
- **The judge is graded by what happened next.** A `runaway` verdict followed by
  a clean completion (the work finished, no reserve breached) counts against
  the judge; an `expected` followed by a crash counts against it. That grading
  is read by the same fit rule the router trusts — the discipline of
  `fleet judge --agree/--disagree`, applied to the resource judge itself.
- **Learning lands as a change, through the gate.** The demand-map beat clusters
  the rows; `fleet improve` can propose a structural correction (a cap, an
  estimate floor) and applies it only through the existing gate. Nothing drifts
  silently.
- **The safety floors never learn.** The reserve (20 GiB free, the memory
  ceiling) is the owner's declaration and stays fixed; only estimates and
  budgets for *work* calibrate. A floor that history could lower is not a
  safety floor.

## The environment, and the risk the owner takes (the closing frame)

Responsible execution is not a constant: it is a function of the machine and of
what a bad day costs the person using it. A constrained laptop needs tighter
upkeep than a build box with a terabyte spare, and code that hardcodes one
answer is wrong on the other machine.

So the contract's numbers derive from two inputs, one measured and one
declared:

- **The environment, learned.** The volume's size and standing free space, the
  fill rate (GiB/day, from the footprint rows), memory and swap pressure, and
  whether the machine is a laptop (constrained) or a server (roomy). The fill
  rate is what sets the *cadence*: a machine losing 2 GiB a day at 120 GiB free
  does not need the same upkeep interval as one at 25 GiB free. The profile is
  re-derived as rows land — that is the learning, and it is arithmetic.
- **The risk appetite, declared.** Only the owner knows what a full disk costs
  them this week. One setting — `steady` (keep a wide reserve, refuse early),
  `balanced`, or `tight` (accept a narrow reserve for throughput) — plus the
  option to name the floor outright. Never inferred from history.

The declared budgets become derived, not fixed: the reserve is the tighter of
the environment's floor and the owner's appetite; the upkeep cadence follows
the fill rate; per-run budgets follow the appetite. The constants in the
footprint script today are **priors for a fresh machine**, and the first
recorded profile and the first declared appetite replace them.

This is also the answer to the earlier question of whose numbers they are: not
mine, and not the code's — the machine's, bounded by the owner's tolerance.

### The appetite is inferred too — and a tight machine tightens fleet

The owner's correction (2026-11-09): the risk appetite does not have to be
declared, because the environment has already been telling us.

- **The signal is the standing free fraction**, not a moment's free space: the
  median of the last thirty daily readings (as a share of the volume) and the
  volatility around it — how often it dips near the floor, and how fast it
  climbs back. A machine that habitually sits under 10% free has an owner who
  tolerates risk, and that is a fact about the *environment's reality*.
- **The direction is the counterintuitive part, and it is the rule**: a
  habitually full machine makes fleet **tighter**, never looser. The owner's
  tolerance is not a budget grant — it is evidence that there is no margin to
  absorb a runaway. So an inferred `tight environment` raises the reserve as a
  *fraction of the volume* (not an absolute that a smaller disk would wave
  through), shortens the upkeep cadence, lowers the per-run budget, and refuses
  earlier. A roomy machine is the one that gets the looser numbers.
- **The inference is named with its evidence**: `reserve 42 GiB: laptop, 9% free
  for 30 days, filling 1.2 GiB/day — inferred tight; say "steady" to override`.
  The declared setting wins where it exists, and the report says which of the
  two produced the number.

## Acceptance criteria

- [x] C1 — A `Contract` with pure `admit(facts, estimate)` and
      `settle(before, after, budget)` in `src/footprint.rs`, hermetic and
      pinned: headroom over estimate admits, under refuses with both numbers;
      a delta over budget settles as runaway, under as clean.
- [x] C2 — The release path admits before it builds (replacing the bare floor
      check) and settles after the assets upload, cleaning the lane on a
      runaway and keeping the diagnosis window on a failure.
- [x] C3 — A run admits before its first turn while working in a folder, and
      measures at turn boundaries (cheap: `statvfs` + `ps`); a breach mid-run
      stops the run with the numbers in the transcript, not after the disk is
      full.
- [x] C4 — The abnormal reading calls sys1 exactly once per reading with the
      facts and the run's recent tool calls; `runaway` curtails, `expected`
      raises that run's budget; a refused answer is `expected` and records it.
- [x] C5 — Every admission, refusal, settlement and judgment lands a calibration
      row (`op=footprint`) carrying the work class, the admitting estimate, the
      settled delta, the reserve at admission, the verdict and the sys1 words,
      so the demand map can cluster the runaways and the estimate can be
      derived from them.
- [x] C6 — A forced-runaway test: a scratch folder filled until the run's budget
      trips, asserting the run is curtailed, its transcript names the numbers,
      and the files it created are cleaned after the diagnosis window.
- [x] C7 — Docs: `docs/features/` gains the contract's own page; `fleet doctor`
      and `fleet footprint` name the same declared numbers.

- [x] C8 — The admitted estimate for a work class is the median of its last ten
      settlements, with the declared constant as the prior until three exist;
      a test with a deliberately wrong prior corrects after the settles land.
- [ ] C12 — PARTLY: the risk appetite is inferred from the standing free fraction
      (median of the last thirty daily readings) and its volatility, with the
      declared setting overriding it; a habitually under-10%-free machine
      yields *tighter* controls — a reserve as a fraction of the volume, a
      shorter cadence, a lower per-run budget — and the report names the
      inference and its evidence beside the number it produced.
- [ ] C10 — NOT BUILT: an environment profile (volume size, standing free space, fill
      rate, memory pressure, laptop or server) is derived from the footprint
      rows, and a declared risk appetite (`steady`/`balanced`/`tight`, or an
      explicit reserve) bounds it; the reserve, the cadence and the per-run
      budgets derive from the two, so no number in this contract is a code
      constant once a profile exists.
- [ ] C11 — PARTLY: both name the appetite and the reserve beside the numbers
      they produced ("reserve 18 GiB of 460 GiB (inferred; 1 record(s), standing
      25% free)"), so a refusal reads as a decision. Not yet named: the profile
      (laptop/server) and the fill rate ("laptop, steady, filling 1.8 GiB/day").
- [x] C9 — The resource judge's verdicts are graded by the subsequent outcome
      (a `runaway` on work that then completed cleanly counts against it; an
      `expected` before a breach counts against it), and that grading is
      readable by the fit rule.

## The next slice, in build order

The deterministic core is in `src/footprint.rs` (`admit`, `settle`,
`Env`/`appetite`/`reserve_kib`, six tests). The rest is wiring, in this order —
each step lands with its own test, and only the sys1 arm needs the model:

1. **C2 — the release path settles.** `scripts/release.sh`'s `preflight_disk`
   already measures and refuses; replace its hardcoded floor with the reserve
   `fleet footprint` derives, and after the assets upload compare the lane's
   size delta against the estimate the admission used. Over → the
   `.failed-build` marker, the diagnosis window, then the existing clean.
2. **C1 at the seams.** Boundary pairs: `admit` at exactly the reserve
   (equal → refused), `settle` at exactly the budget (equal → not a runaway).
3. **C3 — the run loop measures.** `src/agent.rs` already has the turn boundary
   (the context-budget check). Admit before the first turn when a folder is
   granted; re-measure there (`statvfs` + `parse_ps`), stopping through the same
   path a seat denial uses and writing the numbers as a `Record`. Test the
   cadence: every turn may be too often; every fifth is the guess to measure.
4. **C5 — the rows.** `calib.rs`'s row writer takes a shape-typed record; add
   `op=footprint` with the work class, estimate, settled delta, reserve at
   admission, verdict. The demand map already reads rows by `op`.
5. **C4 — the sys1 arm**, last because it needs rows to grade it: one bounded
   ask through the existing lane client, fed the numbers and the run's recent
   tool calls; `runaway` stops, `expected` raises that run's recorded budget,
   and a refused answer is `expected` and records itself.
6. **C8/C9 — the learning.** The estimate is the median of the last ten settles
   per work class; the judge's grading rides `delegate.rs`'s
   outcome-evidence pattern. Both are pure reads over rows, testable without a
   lane.
7. **C6 — the runaway proof.** One integration test: a scratch folder filled
   past the run's budget, asserting the stop, the transcript's numbers, and the
   cleanup after the marker's grace.

## Not in scope

- Predicting a run's write volume (unknowable; the per-turn measurement is the
  answer).
- Curtailling other processes on the machine: the contract governs fleet's own
  work.
- CPU and network budgets: the same shape, a later increment.

## Risks

- **A false curtail is worse than a slow run.** The sys1 arm exists for that;
  a deterministic breach still stops, but the budget is per-run and raised by a
  judged `expected`, so the second identical shape does not pay twice.
- **Self-measurement during a heavy run is a moving target.** The report names
  the reading's own interval, and the process ceiling counts descendants —
  never the measuring process's ancestors.
- **The numbers are the owner's.** Defaults are declared in one place; the
  first task is to confirm them on this machine (2 GiB / 80 processes / 20 GiB
  free floor are proposals, not facts).
