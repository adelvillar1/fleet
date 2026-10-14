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
src/cli.rs           `fleet eval [--last]`, `fleet doctor --diagnose <session> [--ledger <path>]`
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

## The improve-gate hook

The AC that closes the loop with slice 12. A **source edit** — a
proposal whose change is a `patch`, the one change that edits the
repo's source — runs the eval corpus as a gate step *after* the four
cargo steps (which have already rebuilt the binary, so the corpus grades
the edited code). A regression refuses the apply with the scenario
named; the `eval_corpus` step rides the `propose` row's gate evidence
like any step. A skill/memory/config target's gate is unchanged — no
corpus step.

The production probe (`eval::subprocess_probe`) runs `fleet eval` as a
subprocess so the gate grades the freshly-built binary, and so the gate
never blocks on the async runtime it is called from (KI-021). The
refusal is driven deterministically in tests through a stand-in probe —
the same seam the gate's stand-in `cargo` uses.

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
failure's signature, or naming the row a filing would add. The filing
(`file_ledger_row`) is append-or-update by a stable key — the failure's
signature, a 16-hex hash of class, layer, and summary — so re-diagnosing
updates the row rather than duplicating it, and it never rewrites an
unrelated row. The ledger (`docs/known-issues.md`) stays the
human-facing system of record (directive 3); the automation writes only
where `--ledger <path>` points it, and the bare `--diagnose` names the
repo's own ledger as the file it *would* write and writes nothing.

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
