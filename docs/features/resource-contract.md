# The resource contract

Fleet measures what it is using, refuses work the machine cannot host, settles what work actually took, and learns its own estimates from those settlements. The owner's ruling (2026-11-09) is the whole of it:

> *"I don't want fleet trying to use more than what's available… measure, run work, remeasure, and if there is any runaway work then curtail the behavior. An LLM or sys1 call that prevents the environment from crashing is worth the cost."*

The distinction that shapes every surface: fleet used to be a **good** citizen (it cleaned up afterwards) and had to become a **responsible** one (a store has a number it stays under, and work is refused before it starts rather than rescued after).

## The four beats

Every unit of work — a release gate, a release assets cut, a lane bootstrap, a run in a folder — moves through:

1. **Measure.** Free space from `statvfs`, the process tree from `ps`, and the reserve derived from the environment's own history.
2. **Admit or refuse.** The work declares an estimate; if the headroom above the reserve does not exceed it, the work is refused *before it starts*, with both numbers named. Deterministic: "6 GiB of room, 8 GiB of work" needs no judgment.
3. **Work.**
4. **Settle and curtail.** Re-measure. A delta past the work's declared budget is an overage; a delta past the *reserve* stops immediately, because a machine at its floor has no margin for a judgment to be wrong about.

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

**These are the owner's numbers, settled 2026-11-10** (*"leave them like that"*). They are declared in exactly those places, so changing one is one edit in one place. Two rules keep them honest:

- **The reserve never learns.** Only estimates and per-work budgets calibrate. A floor that history could lower is not a floor.
- **A tight machine tightens fleet, never loosens it.** The appetite is inferred from the standing free fraction (the median of the recorded history, so one good day after a cleanup does not read as roomy). A machine that habitually sits under 10% free is evidence of *no margin*, so fleet raises the reserve share, shortens its cadence and refuses earlier.

## Where the judgment goes

One question has no arithmetic answer: *"this run grew 4 GiB — is that the work, or a runaway?"* That is sys1's, and the plan's rule is that it fires **only on an abnormal reading**, never per turn. `runaway` curtails, `expected` raises that run's recorded budget so the identical shape does not ask twice, and a lane that does not answer is read as `expected` and records itself as `judge-unavailable` — never a silent stop on a hiccup.

## The recursion

- **One row per unit of work**, `op=footprint` on the calibration stream: the class, the admitting estimate, the reserve at admission and its source, the headroom, the settled delta, the verdict and how it was decided.
- **The estimate is history, not a constant.** A class's admitted cost is the median of its last ten settlements, with the declared prior until three exist. A release gate that keeps settling at 5 GiB stops being budgeted at 8. A settlement of exactly zero measured nothing and does not teach.
- **The judge is graded by what happened next.** A `runaway` followed by another runaway counts for it; followed by a clean settle, against it. That grading is readable by the same fit rule the router trusts.

## The surfaces

- `fleet footprint` — the process tree against its ceilings, the environment's appetite and reserve, and the store budgets' pointer. `--json` publishes the same facts as one document; `--class <work>` adds that class's estimate, headroom and admission.
- `fleet footprint --class <work> --estimate` — what the class's own history says it costs, and whether that came from its prior or its settlements.
- `fleet footprint --class <work> --record` — the admission as a stream row. `--settle <before_kib>` records the settlement and exits non-zero on a runaway.
- `fleet doctor` — the `disk` line names the **same reserve** the footprint report derives, so a refusal reads as a decision on either surface.
- `scripts/release.sh` — both modes admit against the contract before building and settle after their work, through the binary rather than a re-derived shell number; the declared floor remains the named fallback when no binary on the tree answers.

## What is not covered

Predicting a run's write volume in advance (unknowable; the per-turn reading is the answer), curtailing processes fleet did not spawn (the contract governs fleet's own work), and CPU or network budgets (the same shape, a later increment).
