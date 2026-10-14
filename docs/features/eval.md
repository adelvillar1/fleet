# The eval corpus and self-diagnosis (slice 13)

Three mechanisms, one slice: a fixed, graded, deterministic **eval
corpus** that is the harness's model of its own performance; the
**improve-gate hook** that refuses a source self-edit which regresses
it; and **diagnosis** — per-layer `doctor` probes, typed failure
classification, evidence-first drafting, and automated ledger filing.

Spec: the fork's `benchmarks/` tree (the scenario families' shapes) and
`@fleet/dsh-sdm1`'s telemetry loop (the outcome-scoring vocabulary) —
read as references, never as dependencies (hard rule 6). Ported: the
fixed-corpus shape, the per-scenario grading, the score-as-a-stream-row
idea, and the layer-probe/evidence-first-drafting discipline. Deferred:
the fork's median-timing budgets and machine-profile scaling — a Rust
unit that replays a recorded session grades an *outcome* (does the log
reconstruct, does the request match), not a wall-clock median.

```
src/eval.rs          new: the corpus, the scenarios, the runner, the row, the probe, the surface
src/calib.rs         the `eval_corpus` op: the row, the reader, the card
src/improve.rs       the eval gate step for a source edit (the `patch` change)
src/doctor.rs        the failure classes, the per-layer probes, the diagnosis, the ledger filing
src/cron.rs          the built-in daily eval cadence on the scheduler's own hook
src/cli.rs           `fleet eval [--last]`, `fleet doctor --diagnose <session> [--ledger <path>]`, `fleet doctor --open [--ledger <path>]`
src/main.rs          the dispatch, the boot probes, the eval and diagnose commands
tests/eval.rs        new: the corpus runs green and grades a broken scenario, end to end
```

## The corpus

Six scenarios, fixed in `eval::corpus()` — adding one is a code change,
never a runtime file. Each is a scripted mock-server run with asserted
outcomes, driven by a **plain single-endpoint client** pointed at the
mock, so no run can reach the live API (KI-066 applied structurally, not
by a guard). Deterministic: same corpus, same scripts, same verdicts.

| Scenario | Family | Asserts |
|---|---|---|
| `two-turn-tool-loop` | tool-loop | the run completes in two turns; the second request saw the tool result; the log reconstructs the turn and the assistant carries its call |
| `session-corpus-replay` | session-corpus | the log reconstructs; the reconstructed conversation equals what the request actually carried (before the final answer); the assistant carries its call |
| `agent-continuation` | agent-continuation | a resumed session continues coherently — the resumed request carries the prior turns before the new task |
| `conversation-fold` | conversation-fold | a conversation past the budget folds; a `compaction` record lands; the post-fold request carries the framed summary |
| `subagent-delegation` | subagent | a delegated task runs in its own session on the parent's model; two logs exist; the parent's log carries the delegation result |
| `memory-recall` | memory | a durable fact enters the turn's context; the `memory` record names the shape and the facts (keyword-only, so the shared llama-server is never touched) |

The first three port the fork's `benchmarks/` families the plan names;
the last three are the everyday shapes the crate already proves.

Every driver fault is a **graded verdict, never a panic**: a scratch
dir that will not create, a session log that will not open, a mock
child that exits before its port — each fails the scenario naming the
fault, and `fleet eval` exits on the verdict. A panic in a driver is
the KI-027 spawn/port pattern promoted into the production path: exit
101, with the improve gate's refusal misattributed to a "regressed"
corpus. One coupling to know before editing:
`conversation-fold` asserts the exact model-visible summary prefix
(`Summary of turns 1 through 2, compacted to fit the window:`) — a
cosmetic copy edit to that frame fails the corpus by design, because
the frame is what the scenario grades.

## The runner, the scores, and the schedule

`fleet eval` runs the whole corpus headless, prints house cards (one per
scenario, the failure named), and lands **one `eval_corpus` row** in the
calibration stream. `fleet eval --last` reads the most recent row back
without running. The row's shape:

```json
{"op":"eval_corpus","ts":"…","passed":6,"failed":0,"score":1.0,
 "scenarios":[{"scenario":"two-turn-tool-loop","family":"tool-loop","passed":true}, …]}
```

The aggregate score is `passed / total`; a failed scenario carries its
`failure` name, so the corpus's failures are named, not just counted.

The corpus runs on the **slice-11 scheduler's own hook** — not a
job-store entry, because the corpus is a deterministic run, not an agent
conversation. `cron::EVAL_SCHEDULE = "0 4 * * *"` (daily, 04:00 UTC) is
the cadence; the tick decides it beside the cron jobs with the same
once-per-minute latch (`cron::eval_due`, pure and testable), and its row
lands in the stream like any run. A scheduler that does not wire the
cadence runs no corpus, so the cron-job tests are unchanged.

The cadence's fire rides the **same `CatchUnwind` guard every job fire
gets**: a panic inside it would otherwise unwind through the scheduler
thread's `block_on` and kill the tick loop for the life of the process —
silently, one stderr line over the TUI's alternate screen, and every
scheduled job with it. Caught, it is one named line (`fleet: cron: eval
corpus panicked: …; the minute stays claimed`) and the loop carries on.
A test drives a panicking cadence through the seam, the shape the cron
fix-round used for job fires.

## The improve-gate hook

The AC that closes the loop with slice 12. A **source edit** — a
proposal whose change is a `patch`, the one change that edits the
repo's source — runs the eval corpus as a gate step *after* the four
cargo steps (which have already rebuilt the binary, so the corpus grades
the edited code). A regression refuses the apply with the scenario
named; the `eval_corpus` step rides the `propose` row's gate evidence
like any step. A skill/memory/config target's gate is unchanged — no
corpus step.

When the **cargo gate is red**, the corpus never runs, and the row
records the step as **not-run** — `ok: false`, the detail naming it
(`not run — the cargo gate refused first`). The row then says the corpus
was not reached rather than leaving the step absent, where an operator
could not tell "not reached" from "passed". The refusal itself still
names the cargo steps that failed.

The production probe (`eval::subprocess_probe`) runs `fleet eval` as a
subprocess so the gate grades the freshly-built binary, and so the gate
never blocks on the async runtime it is called from (KI-021). The
refusal is driven deterministically in tests through a stand-in probe —
the same seam the gate's stand-in `cargo` uses.

The production probe **fails closed**. It refuses — naming why, and
recording the `eval_corpus` step failed — unless the current executable
resolves *and* lives under the gate root's own `target/` directory: an
installed or stale binary elsewhere would grade the wrong build and read
as a false green, and an unresolvable one would silently unwire the
step. Its child is bounded by `eval::PROBE_TIMEOUT_SECS` (120s; a
drain thread plus a kill on expiry keeps a hung corpus a named failure,
never a hung gate). The scenario names greped out of `fleet eval`'s
cards are ANSI-stripped, so a palette reset never rides into the gate
step's detail.

## Diagnosis

`doctor`'s per-layer probes name each layer the plan lists with its
reason on failure: credentials, home, the API endpoint, the shell, the
**roster** (`evaluate_roster`), the **ledger** (`evaluate_ledger` — the
calibration stream's writability, a new exit-code-failing check), and
**each MCP core** (`evaluate_mcp_core`, one named line per vendored
lane's home resolution). The dead-port and missing-home layers already
failed; the ledger is the new one. A broken layer is named at boot.

The failure classification maps the crate's existing typed errors to the
plan's five words (`classify_error`): transport → network, malformed 2xx
→ decode, 401/403 → auth, 402/429 → quota, other answered status →
network; a tool result opening `error:` → tool (`classify_tool`); and
`unknown` for a failure that fits none, named honestly rather than
forced.

Evidence-first drafting and the ledger filing: `fleet doctor --diagnose
<session>` replays a log, classifies what is deterministically there,
and drafts a recap composed from the ledger and the session log
(`render_diagnosis`) — citing the ledger rows that already hold the
failure's signature, or naming the row a filing would add. An orderly
completion is **not a failure**: a healthy session diagnoses nothing,
files nothing, and says so (`no failure diagnosed — the run completed`).
The filing (`file_ledger_row`) is append-or-update by a stable key — the
failure's signature, a 16-hex hash of class, layer, and summary — so
re-diagnosing updates the row rather than duplicating it, and it never
rewrites an unrelated row. An update **preserves the row owner's
cells**: the status and the next action stay exactly as the human left
them, and only the issue and the evidence refresh — a re-diagnosis is
evidence gathering, not triage, so a row a human closed stays closed
with its next action intact. The ledger is an LF file: the update path
rebuilds with `lines().join("\n")` plus one trailing newline, which
normalizes a CRLF ledger and adds a missing final newline. The ledger
(`docs/known-issues.md`) stays the human-facing system of record
(directive 3); the automation writes only where `--ledger <path>` points
it, and the bare `--diagnose` names the repo's own ledger as the file it
*would* write and writes nothing.

The action-items read-back: `fleet doctor --open [--ledger <path>]`
reads the ledger and surfaces its **open rows** — every row whose
status cell is `open` — as cards carrying the id, the severity, the
issue, and the next action. An unclosed action item stays visible until
its status leaves `open`. The bare form names the repo's own
`docs/known-issues.md` (resolved from the working directory, KI-099's
family) and writes nothing; `--ledger` points the read at a file.
Either way it is read-only — the human-owned guard, unchanged.

## Deliberately not built

- **sdm1's turn-duration prediction** — a learned model of how long a
  turn takes. The corpus grades outcomes, not latencies; a duration
  predictor needs a training set this crate does not collect yet. Logged
  as the deferred half of the sdm1 telemetry port.
- **Cross-day eval streaks** — a regression that escalates across days.
  The stream carries every run's row; a cross-day reader is a later
  calibration slice (KI-050's family).
- **Auto-filing to the repo ledger without a named path** — the write is
  gated behind `--ledger` so the human-owned ledger stays human-owned
  until the owner points the automation at a file.
