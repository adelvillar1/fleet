# Session Recap — 2026-11-10: the resource contract, built and driven

**Summary.** The resource contract went from an instrument to a working contract.
It now admits work against the headroom above a reserve, settles what the work
actually took, refuses and curtails through the numbers, asks a judge on the one
reading arithmetic cannot settle, and learns its own estimates from its own
settlements. Two sessions of design produced the arithmetic; this one built the
healing. **Nine of the plan's twelve criteria are ticked and tool-verified.**

## What landed

Four commits, each gated locally before it went green:

| Commit | What it is |
|---|---|
| `ec4b9d5` | C1, C5, C8, C9 — the reading, the row, and what the rows teach |
| `9c9a4fa` | C2 — the release path admits and settles, plus three defects the drive found |
| `d474fe3` | C3, C6, C7 — the run loop measures; the docs and the doctor say one number |
| `5fd15af` | C4 — the abnormal reading asks a judge, once and bounded |

### The arithmetic (`src/footprint.rs`)

`admit()` is now **strict at the seam**: an estimate exactly equal to the
headroom is refused, because work costing exactly what is spare consumes the
reserve to its floor. `settle()` stays inclusive: equal to the budget is not a
runaway. Both pinned as a boundary pair.

`Reading` gathers one pass — the process tree graded against its ceilings, the
volume's free space and size from one `statvfs`, the reserve the history implies
and the appetite that produced it — and publishes it as JSON. Work classes carry
their prior estimates in one place, so confirming the numbers is one edit each.

### The door (`fleet footprint`)

`--json` publishes the reading; `--class <work>` adds that class's estimate,
headroom and admission; `--estimate` asks what the class's own history says it
costs; `--record` lands the admission; `--settle <before_kib>` records the
settlement and exits non-zero on a runaway.

### The ledger (`op=footprint`)

`FootprintRow` carries the class, the admitting estimate, the reserve and its
source, the headroom, the settled delta, the verdict and how it was decided.
`estimate_for()` is the prior until three settlements, then the median of the
last ten. `grade_judgments()` scores a judged verdict by the row that followed
it, and a verdict with no outcome yet keeps no rate rather than a flattering one.

### The run loop (`src/contract.rs`)

Its own module, because the agent loop must not carry the arithmetic or name the
judge. `Work` admits; `Oversight` carries the baseline, the reserve and the
cadence; `measure()` grades a reading into `Inside` / `Overage` / `Encroached`;
`apply()` takes the judge's answer and raises this run's budget on `Expected`.
`Encroached` outranks `Overage`: a machine at its floor has no room for a
judgment to be wrong about. The cadence is one named number (5) and the tests
state what it buys. A folderless chat, and every pre-existing run, pays one
branch — the suite is unchanged and green.

### The judge (`src/judge.rs`)

One bounded ask on an abnormal reading only, under the same daily cap and
per-ask reservation the judgment beat uses. An answer that did not judge is never
read as consent: a classification label, an empty answer, an ambiguous sentence,
a spent budget and a lane that refused all read as `Unavailable`, which the
contract records as `expected` with `decided_by: judge-unavailable`. `runaway`
is checked before `expected`, so "this is not expected, it is a runaway" curtails.

**The agent reaches it through a declared capability** (`ResourceJudge` on
`AgentOptions`), not a direct call. That is Guard A's own intent made structural:
the socket guards keep every lane-spending beat unreachable from the turn path,
and `agent.rs` carries the pointer without naming the beat. The TUI takes the
judge through its `Config` and `main` binds it; the bridge binds its own.

## Three defects, each found by driving the arm

1. **`release.sh` threw its own refusal away.** `reading=$(...) || reading=""`
   discarded the JSON exactly when the verdict was *refused* — the binary exits
   non-zero on a refusal — so the refusal arm always fell through to the bare
   floor and the build proceeded. The shell idiom that reads as defensive error
   handling was the bug. (KI-506)
2. **The volume was sized from the wrong disk.** The reserve came from the
   history's volume while free space was measured on the path's volume, so a
   2 GiB scratch volume got a 460 GiB drive's reserve. `volume_facts()` now takes
   free and total from one `statvfs`. (KI-507)
3. **A settlement of zero taught zero.** Six synthetic probe rows drove
   `release-gate`'s estimate from 8 GiB to **44 bytes** — a budget that would
   admit any work on any headroom. A delta of zero measured nothing and no longer
   teaches; the estimate still learns *downward* on real settles, which C8
   requires. (KI-508)

The refusal arm was proven against a real 2 GiB ramdisk: the binary answered
`admitted:false`, and after the fixes the script refuses the build naming both
numbers.

## What is not built

- **C10, the environment profile** — volume size, fill rate, memory pressure,
  laptop-or-server, with every number derived from it. Not started.
- **C11, partly** — both surfaces name the appetite and the reserve and its
  source; neither names the profile or the fill rate yet.
- **C12, partly** — the inference and the fraction-of-volume reserve are live and
  pinned; the cadence and the per-run budget do not yet derive from the appetite.
- The judge's real lane path is wired and unit-pinned through the counting
  session, but it has not been driven against a live sys1 on this machine.

## The numbers still needing the owner

The reserve fractions (tight 12% / balanced 8% / roomy 4%), the 2 GiB memory
ceiling, the 80-process ceiling, the 20 GiB floor and the per-class estimates are
**proposals**, declared in three files and confirmed by nobody. The plan says so
in its own words and the handoff says ask, don't assume.

## Files changed

`src/footprint.rs`, `src/contract.rs` (new), `src/calib.rs`, `src/judge.rs`,
`src/agent.rs`, `src/session.rs`, `src/doctor.rs`, `src/main.rs`, `src/cli.rs`,
`src/lib.rs`, `src/gui/bridge.rs`, `src/tui/mod.rs`, `src/tui/app.rs`,
`scripts/release.sh`, `tests/integration.rs`, and the docs:
`docs/features/resource-contract.md` (new), the plan's criteria, and KI-506/507/508.

## Open questions / next steps

- **C10 next**, because C11 and C12 both read the profile it derives.
- The owner's numbers, whenever they are given — one edit each, three places.
- Whether a deterministic breach hard-stops or pauses for a yes. It hard-stops
  today, which is the plan's default; the alternative is the owner's call.
