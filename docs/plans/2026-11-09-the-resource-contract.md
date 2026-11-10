---
status: draft
created: 2026-11-09
updated: 2026-11-09
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

## Acceptance criteria

- [ ] C1 — A `Contract` with pure `admit(facts, estimate)` and
      `settle(before, after, budget)` in `src/footprint.rs`, hermetic and
      pinned: headroom over estimate admits, under refuses with both numbers;
      a delta over budget settles as runaway, under as clean.
- [ ] C2 — The release path admits before it builds (replacing the bare floor
      check) and settles after the assets upload, cleaning the lane on a
      runaway and keeping the diagnosis window on a failure.
- [ ] C3 — A run admits before its first turn while working in a folder, and
      measures at turn boundaries (cheap: `statvfs` + `ps`); a breach mid-run
      stops the run with the numbers in the transcript, not after the disk is
      full.
- [ ] C4 — The abnormal reading calls sys1 exactly once per reading with the
      facts and the run's recent tool calls; `runaway` curtails, `expected`
      raises that run's budget; a refused answer is `expected` and records it.
- [ ] C5 — Every admission, refusal, settlement and judgment lands a calibration
      row (`op=footprint`) so the demand map can cluster the runaways.
- [ ] C6 — A forced-runaway test: a scratch folder filled until the run's budget
      trips, asserting the run is curtailed, its transcript names the numbers,
      and the files it created are cleaned after the diagnosis window.
- [ ] C7 — Docs: `docs/features/` gains the contract's own page; `fleet doctor`
      and `fleet footprint` name the same declared numbers.

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
