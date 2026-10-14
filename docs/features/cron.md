# Scheduled runs (slice 11)

Cron-shaped scheduling with a persistent job store, catch-up on missed
windows, and `fleet cron`. A scheduled task fires on time and after a
missed window; a job's output is retained in its own ordinary session
log.

Spec: the fork's `packages/schedule` + `packages/jobs` — read as a
reference, never as a dependency (hard rule 6). Ported vocabulary: the
job record's shape, the catch-up rule (one run for the missed window,
resolved near the decision and not across the whole missed history),
the reminder framing a scheduled turn carries, and the delivery seam,
which this crate deliberately does not build (see *Delivery* below).

```
src/cron.rs        new: the minute, the parser, the store, the run, the scheduler, the surface
src/cli.rs         `fleet cron`: list, add, enable, disable, remove
src/main.rs        `fleet cron` dispatch, and the scheduler thread in `chat`
src/tui/mod.rs     the scheduler thread beside the consolidator's
tests/integration.rs  the acceptance tests, driven through the mock server
```

## The store

One JSON document, `<FLEET_HOME>/cron/jobs.json`, rewritten atomically
(temp file beside it, then the rename — the house pattern) on every
change. A reader therefore sees either the whole previous document or
the whole new one, never a half-written store:

```json
{
  "jobs": [
    {
      "name": "daily-digest",
      "schedule": "0 7 * * *",
      "task": "Summarize yesterday's session logs into a digest.",
      "auto_approve": false,
      "enabled": true,
      "last_run": "2026-10-14T07:00Z",
      "last_outcome": "completed"
    }
  ]
}
```

The record is the whole contract:

- `name` — a validated stem: lowercase letters, digits, and hyphens, up
  to 48 characters, no leading or trailing hyphen. The memory corpus's
  own name rule, applied to the key inside the one store document: no
  path separator, no `.` or `..`, no dotfile, no space.
- `schedule` — the five-field cron expression, stored exactly as typed
  (trimmed) and re-validated on every load.
- `task` — the goal, **self-contained**: a scheduled run sees no
  conversation, so the task has to stand on its own.
- `auto_approve` — whether the job is declared unattended (see *The
  run*).
- `enabled` — a disabled job stays in the store and never fires.
- `last_run` — the minute the job last ran *for*, `YYYY-MM-DDTHH:MMZ`,
  or `null`. A catch-up run records the window it served, not the wall
  clock it served it at, which is what keeps a restart from catching up
  on a window already served.
- `last_outcome` — `completed`, `failed`, `skipped-needs-approval`, or
  `null`.

A missing file is an empty store, not an error. A file that exists but
is not a job store is a **typed error**, never a silent empty read: not
JSON, the wrong document shape, a record with an unknown field, a name
that is not a stem, a schedule that does not parse, a `last_run` that is
not a minute, an empty task, or a duplicated name each fail with the
reason named and the offending job identified.

Every mutation — `add`, `enable`, `disable`, `remove`, and the
scheduler's own `record_run` — round-trips through the same validator a
load uses, so a store this crate wrote is a store this crate can read,
and a refused mutation writes nothing.

## The parser

A hand-rolled five-field parser: `minute hour day-of-month month
day-of-week`, no dependencies and no names. The supported dialect:

| form | meaning |
|---|---|
| `*` | every value in the field |
| `5` | one value |
| `1-5` | an inclusive range |
| `*/15` | every 15th value from the field's minimum |
| `0-30/10` | every 10th value inside a range |
| `1,15,30` | a comma-separated list of any of the above |

Ranges: minute 0-59, hour 0-23, day-of-month 1-31, month 1-12,
day-of-week 0-7 where **both 0 and 7 mean Sunday** (7 folds onto 0, so a
matched set never depends on which spelling was typed). A step attaches
to `*` or to a range and to nothing else — `5/2` is refused rather than
read as `5-max/2`, the fork's dialect.

Validation errors are named, each carrying the field and the offending
text: the field count, a value outside its range, a step that is not a
positive integer (`*/0` lands here), an inverted range, an empty list
element, and an element outside the dialect.

The day-of-month/day-of-week rule is Vixie's, ported from the fork: when
either field is a `*` the two must both match (the star one always does,
so the rule degrades to the other field); when both are restricted,
either matching suffices. `0 0 13 * 5` therefore fires on every Friday
the 13th *and* on every 13th *and* on every Friday.

The next-fire computation is a pure function of (expression,
from-minute): `next_after` walks forward day by day, and
`previous_at_or_before` walks back, each bounded by a ten-year horizon —
long enough for every expression that ever matches (the longest real gap
is the eight years between February 29 2096 and February 29 2104, across
the non-leap century 2100) and short enough that an unsatisfiable
expression like `0 0 31 4 *` resolves to "never" instead of scanning
toward the year 9999. That purity is the seam the tests drive: a minute
is constructed, not waited for.

## The scheduler

A detached background thread in `chat` and in the TUI — the
`spawn_consolidator` precedent — owning its own current-thread tokio
runtime, because a fire is an ordinary async agent run. The catch-up
pass runs once at startup, then the loop ticks every
`CRON_TICK_SECONDS = 1` for the life of the process.

The tick is cheap: the store is re-read whole (it is one small JSON
document, and re-reading is what makes `fleet cron add` from another
shell land within a second), and each enabled job costs one `contains`
check per field against the current minute — not a next-fire walk.

**The latch.** A job fires at most once per matching minute per job,
held in a `HashMap<String, Minute>` in memory. The latch is claimed at
decision time, so a job the tick has decided to fire is not decided
again for that minute whether its run succeeds or fails, and a run that
outlasts its own minute cannot fire twice. The same latch covers the
catch-up pass: a startup pass that runs twice, or a tick inside the
missed minute, does not serve the window again.

Two long-lived processes on one machine — a `chat` and a TUI — each own
their own latch, so the same job can fire in both. That is the design's
rule (the latch is in memory); it is logged as a known issue rather than
papered over.

## Catch-up

At scheduler start, each **enabled** job whose `last_run` is older than
its most recent matching minute gets **exactly one** catch-up run — the
missed window, resolved near the decision (the fork's rule: a recurring
decision resolves "near the decision's local date, not across its missed
history") rather than by replaying the whole backlog the process was
down for. A job down for a week with an hourly schedule gets one run,
not 168.

**A job that has never run does not catch up.** Its first fire is its
first matching minute going forward. The reason: there is no window to
catch up to — the job's history is empty, so "the missed window" is
undefined, and reading "never run" as "everything since the epoch was
missed" would fire a job the moment it was created. A `* * * * *` job
added at 12:00:30 would immediately run for 12:00 — a run the owner
never scheduled, and with `auto_approve` a real spend. The fork's rule
is the same shape: a committed target must be strictly future at
creation, and a dispatch resolves one latest-due occurrence rather than
a backlog.

The catch-up is named in the run's own first message, which lands in its
session log:

```
[scheduled run — catch-up for job `hourly`: the 2026-10-14T12:00Z window was missed while fleet was down]
Summarize the hour's session logs.
```

and on the surface that started the scheduler:

```
fleet: cron: job `hourly` catch-up for 2026-10-14T12:00Z — completed in 1 turn (session 20261014T123000Z-1a2b3c)
```

## The run

A fire is an ordinary headless agent run: the same loop, the same routed
client and roster, its own session log under the sessions directory, the
memory corpus and the skills wired the way `fleet run` wires them, and
its turns landing calibration rows like any run. `run_job` resolves the
route for the job's own task — one judgment per scheduled run, exactly
like `fleet run` — and records the decision row.

The corpus root is an explicit input to `execute`, the way `home` is:
the caller decides, and `run_job` resolves it with the same
`resolve_root(cwd)` walk `fleet run` uses. Production behavior is
unchanged; the explicit input is what lets a test drive this path
without writing recall hits into whatever corpus its own working
directory sits above (KI-091).

The framing every scheduled turn carries is the fork's reminder framing,
ported: the model must be able to tell a scheduled turn from a user
sitting at the keyboard, and the log must reconstruct the turn exactly
(hard rule 5), so the framing rides the task itself.

**The auto-approve policy.** Nobody is there to approve a tool call in
an unattended run, and an unattended run must not silently auto-approve a
dangerous tool. The job's `auto_approve` decides:

- `true` — the job is declared unattended and its calls run. The gate is
  a run-scoped always, the way `--yes` is, and the calibration screening
  row names it `always`.
- `false` — every call is **skipped and named**: the gate denies it, the
  first tool it denied is captured, the job's `last_outcome` records
  `skipped-needs-approval`, and the scheduler's line names the tool
  (`skipped `bash`: the job is not declared unattended and no one is here
  to approve it`). The denial rides the ordinary `tool_result` path into
  the log with `approved: false`, so the run's own record shows exactly
  which call did not happen.

An unattended run's per-turn progress has no surface to land on — no
foreground is watching — so it goes to the log and the scheduler names
the outcome on the surface that is watching.

## Retention

A job's output is retained the ordinary way: **the run's own session
log**, under `<FLEET_HOME>/sessions/`, resumable and listed by
`fleet sessions` like any other log, plus the job's `last_run` and
`last_outcome` fields in the store. There is no second copy and no
side channel: a scheduled run is a session, and a session is a log.

## Delivery — deliberately not built

The crate has no gateway-connected surface yet, and a `deliver` field
that silently did nothing would be a lie. The seam is in the record's
shape: `last_run`/`last_outcome` are the receipt half a delivery writer
would append to, and the fork's `packages/schedule` carries the full
vocabulary (a delivery receipt per occurrence, a bounded delivery
history, retention bounds in days and records) for the slice that builds
it. Until then the retention above *is* the delivery: the log is where
the output went.

## The surface

`fleet cron` with no subcommand lists the jobs as cards — name, schedule,
next fire, last run and outcome — in the house card style, never a
plain-text table:

```
fleet cron — 2 jobs in ~/.fleet/cron/jobs.json

┌─ daily-digest ───────────────────────────── enabled ┐
│ 0 7 * * *
│ Summarize yesterday's session logs into a digest.
│ next fire 2026-10-15T07:00Z · auto-approve off
│ last run 2026-10-14T07:00Z · completed
└──────────────────────────────────────────────────────┘
```

`fleet cron add --name <name> --schedule "<expr>" --task "<goal>"
[--auto-approve]` creates one; `enable`, `disable`, and `remove` mutate
by name, every one of them through the same validator and the same
atomic rewrite. The listing is a read surface (exit 0 even with no
jobs); a typed refusal — a taken name, a schedule that does not parse, a
name not in the store — prints its reason and exits 1.
