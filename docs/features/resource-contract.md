# The resource contract

Fleet measures what it is using, refuses work the machine cannot host, settles what work actually took, and learns its own estimates from those settlements. The owner's ruling (2026-11-09) is the whole of it:

> *"I don't want fleet trying to use more than what's available… measure, run work, remeasure, and if there is any runaway work then curtail the behavior. An LLM or sys1 call that prevents the environment from crashing is worth the cost."*

The distinction that shapes every surface: fleet used to be a **good** citizen (it cleaned up afterwards) and had to become a **responsible** one (a store has a number it stays under, and work is refused before it starts rather than rescued after).

## The four beats

Every unit of work — a release gate, a release assets cut, a lane bootstrap, a run in a folder — moves through:

1. **Measure.** Free space from `statvfs`, the process tree from `ps`, and the reserve derived from the environment's own history.
2. **Admit or refuse.** The work declares an estimate; if the headroom above the reserve does not exceed it, the work is refused *before it starts*, with both numbers named. Deterministic: "6 GiB of room, 8 GiB of work" needs no judgment.
3. **Work.**
4. **Settle and curtail.** Re-measure. A delta past the work's declared budget is an overage; a delta past the *reserve* is a **breach**, which pauses the run and puts the numbers in front of the owner. No judge is asked — a breach is arithmetic — and the run continues only on a yes. A no, or nobody there to answer, ends it.

**A yes at a breach buys one run's budget of room, not a licence on the disk.** The run may keep working down to `reserve − budget`, is asked again only if it goes below *that*, and a second yes moves the floor by another budget and no further. The grant lowers the floor; it never excuses the budget, so the overage arm still grades the work.

## The numbers, and whose they are

| Number | Value | Where it lives |
|---|---|---|
| Memory ceiling | 2 GiB | `src/footprint.rs` `MEMORY_CEILING_KIB` |
| Process ceiling | 80 | `src/footprint.rs` `PROCESS_CEILING` |
| Reserve share, roomy / balanced / tight | 4% / 8% / 12% of the volume | `Appetite::reserve_share` |
| Estimate, release gate / assets | 8 GiB | `ESTIMATE_RELEASE_GATE_KIB`, `ESTIMATE_RELEASE_ASSETS_KIB` |
| Estimate, lane bootstrap | 2 GiB | `ESTIMATE_LANE_BOOTSTRAP_KIB` |
| A run's own budget | 2 GiB | `RUN_BUDGET_KIB` |
| Store budgets | target 20 GiB, one lane 20, lanes 24, `~/.fleet` 12 | `scripts/footprint.sh` |
| Standing window | 30 daily readings, median | `footprint::STANDING_WINDOW` |
| Near the floor | under 10% free | `footprint::NEAR_FLOOR` |
| Volatile | spread over 5 points of the volume | `footprint::VOLATILE_MAD` |

**These are the owner's numbers, settled 2026-11-10** (*"leave them like that"*). They are declared in exactly those places, so changing one is one edit in one place. Two rules keep them honest:

- **The reserve never learns.** Only estimates and per-work budgets calibrate. A floor that history could lower is not a floor.
- **A tight machine tightens fleet, never loosens it.** The appetite is inferred from the standing free fraction (the median of the last **thirty** readings, so one good day after a cleanup does not read as roomy) **and its spread** (the median absolute deviation). A machine whose free share swings by more than five points has no margin to absorb a runaway even when its median reads comfortable, so it is graded one step tighter — never one step looser, so a cleanup spree cannot buy a looser contract.
- **Only estimates and work budgets calibrate; a floor that history could lower is not a floor.**

## What derives, and from what

Measured once a day by `scripts/footprint.sh --record`, into
`<FLEET_HOME>/footprint-history.jsonl`: the volume, the free space, and memory
in use. One reader turns that history into a profile — `src/footprint_history.rs`,
which `fleet footprint`, `fleet doctor` and the agent loop all call — and the
profile plus the owner's declaration produce every number the contract runs on:

| Number | Derivation | Constant only when… |
|---|---|---|
| Reserve | the appetite's share of the volume | never — the share is a declared constant |
| Run budget | the appetite's share of the volume, floored at 2 GiB and capped at four of them | the history is empty |
| Turn cadence | 2 turns when tight, 5 otherwise | the history is empty |
| Upkeep interval | headroom above the reserve ÷ the fill rate, floored at one day and capped by the appetite | the volume is not filling |
| Memory pressure | the newest reading's used share | never recorded |

The profile names its machine (`laptop` when `pmset`/`/sys` find a battery,
`server` otherwise, `FLEET_MACHINE` overriding) because a full disk on a laptop
costs its owner the machine.

**The upkeep interval is consumed, not printed.** `scripts/daily-maintenance.sh`
reads `upkeep_days` from `fleet footprint --json` as the build lanes' retention
window, so a machine filling fast revisits its caches sooner than one that is
not — and `FLEET_LANE_KEEP_DAYS` stays the owner's override.

## Declaring it instead of inferring it

Two settings, both reverted by `fleet settings unset <key>` back to inference:

- `appetite` — `tight`, `balanced`, `roomy` (or `steady`, the plan's word for
  `roomy`). It sets the reserve share **and** the cadence, the run budget and
  the upkeep interval: one grade is in force, and every derived number reads it.
  The report names the owner's hand beside the machine's own grade —
  *"declared tight, the machine infers roomy"*.
- `footprint.reserve_gib` — the number itself rather than the word. It wins over
  the appetite, and every refusal then reads as the owner's own figure
  (`reserve_source: declared floor`).

## Where the judgment goes

One question has no arithmetic answer: *"this run grew 4 GiB — is that the work, or a runaway?"* That is sys1's, and the plan's rule is that it fires **only on an abnormal reading**, never per turn. `runaway` curtails, `expected` raises that run's recorded budget so the identical shape does not ask twice, and a lane that does not answer is read as `expected` and records itself as `judge-unavailable` — never a silent stop on a hiccup.

The breach pause is the owner's own question, so it goes to the **approval seat**, not to sys1: the console prints the clause and takes `y`, the TUI seat and the browser seat show it as the prompt's summary. An approver that cannot ask — an unattended job, a headless batch — answers **no**, because a machine at its floor cannot wait for an answer that will never come.

## The recursion

- **One row per unit of work**, `op=footprint` on the calibration stream: the class, the admitting estimate, the reserve at admission and its source, the headroom, the settled delta, the verdict and how it was decided.
- **The estimate is history, not a constant.** A class's admitted cost is the median of its last ten settlements, with the declared prior until three exist. A release gate that keeps settling at 5 GiB stops being budgeted at 8. A settlement of exactly zero measured nothing and does not teach.
- **The judge is graded by what happened next.** A `runaway` followed by another runaway counts for it; followed by a clean settle, against it. That grading is readable by the same fit rule the router trusts.

## The surfaces

- `fleet footprint` — the process tree against its ceilings, the environment's clause (the inference, its evidence and the number it produced), the derived cadence, run budget and upkeep interval, and the store budgets' pointer. `--json` publishes the same facts as one document; `--class <work>` adds that class's estimate, headroom and admission.
- `fleet footprint --class <work> --estimate` — what the class's own history says it costs, and whether that came from its prior or its settlements.
- `fleet footprint --class <work> --record` — the admission as a stream row. `--settle <before_kib>` records the settlement and exits non-zero on a runaway. Both take the class's prior from the same reading, so a run's number follows the profile.
- `fleet doctor` — the `disk` line names the **same reserve** the footprint report derives *and* the sentence that produced it — the machine, the standing share, the spread, the fill rate and the memory pressure — so a refusal reads as a decision on either surface.
- `scripts/release.sh` — both modes admit against the contract before building and settle after their work, through the binary rather than a re-derived shell number; the declared floor remains the named fallback when no binary on the tree answers.

## What is not covered

Predicting a run's write volume in advance (unknowable; the per-turn reading is the answer), curtailing processes fleet did not spawn (the contract governs fleet's own work), and CPU or network budgets (the same shape, a later increment).
