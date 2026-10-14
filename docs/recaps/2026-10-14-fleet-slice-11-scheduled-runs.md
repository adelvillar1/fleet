# Slice 11 — scheduled runs (2026-10-14)

**Plan:** `docs/plans/2026-10-13-fleet-everyday-harness.md` (slice 11).
**Feature:** `docs/features/cron.md`. **Ledger:** KI-087..KI-091.

## What landed

`src/cron.rs` (new) — the minute, the parser, the store, the run, the
scheduler, and the surface — plus `src/cli.rs` (`fleet cron`), `src/main.rs`
(the dispatch and the `chat` scheduler thread), `src/tui/mod.rs` (the TUI
scheduler thread), and the integration tests.

- **The store.** One JSON document at `<FLEET_HOME>/cron/jobs.json`,
  rewritten atomically (temp file beside it, then the rename) on every
  change. A missing file is an empty store; a file that is not a job
  store is a typed error with the reason and the offending job named —
  not JSON, the wrong document shape, an unknown field, a name that is
  not a stem, a schedule that does not parse, a `last_run` that is not a
  minute, an empty task, or a duplicated name. Every mutation round-trips
  through the same validator a load uses, and a refused mutation writes
  nothing.
- **The parser.** Hand-rolled, no new dependency, five fields
  (`minute hour day-of-month month day-of-week`), `*`, `,`, `-`, and
  `/step` over numeric values only. Day-of-week 0 and 7 both mean Sunday
  (7 folds onto 0). A step attaches to `*` or to a range and to nothing
  else — `5/2` is refused rather than read as `5-max/2`. The
  day-of-month/day-of-week rule is Vixie's, ported from the fork. Named
  errors: the field count, a value outside its range, a step that is not
  a positive integer (`*/0`), an inverted range, an empty list element,
  an element outside the dialect.
- **The search is pure.** `next_after` and `previous_at_or_before` are
  functions of (expression, from-minute) — the seam the tests drive, with
  a minute constructed rather than waited for. Both are bounded by a
  ten-year horizon: long enough for every expression that ever matches
  (the longest real gap is the eight years between February 29 2096 and
  February 29 2104, across the non-leap century 2100), short enough that
  `0 0 31 4 *` resolves to "never" instead of scanning toward the year
  9999.
- **The scheduler.** A detached thread in `chat` and the TUI — the
  `spawn_consolidator` precedent — with its own current-thread runtime,
  because a fire is an async agent run. The catch-up pass runs once at
  startup, then a tick every `CRON_TICK_SECONDS = 1`. The tick re-reads
  the store (which is what makes `fleet cron add` from another shell land
  within a second) and costs one `contains` check per enabled job per
  second — not a next-fire walk. The fired-minute latch is a
  `HashMap<String, Minute>` claimed at decision time, so a job is not
  decided twice for one minute whether its run succeeds or fails, and a
  run that outlasts its own minute cannot fire twice.
- **Catch-up.** Exactly one run for the missed window, resolved near the
  decision (the fork's rule) rather than by replaying the backlog — a job
  down for a week with an hourly schedule gets one run, not 168.
  **A never-run job does not catch up**: its first fire is its first
  matching minute going forward, because "never run" read as "everything
  since the epoch was missed" would fire a job the moment it was created
  (a `* * * * *` job added at 12:00:30 would immediately run for 12:00 —
  a run the owner never scheduled, and with `auto_approve` a real spend).
  The catch-up is named in the run's own first message, which lands in
  its log: `[scheduled run — catch-up for job `hourly`: the
  2026-10-14T12:00Z window was missed while fleet was down]`.
- **The run.** An ordinary headless agent run: the same loop, the same
  routed client and roster (`run_job` resolves the route for the job's
  own task, one judgment per scheduled run, exactly like `fleet run`),
  its own session log under the sessions directory, the memory corpus and
  the skills wired the way `fleet run` wires them, and its turns landing
  calibration rows like any run. The corpus root is an explicit input to
  `execute` — the caller decides, the way it decides `home` — with
  `run_job` resolving it through the same `resolve_root(cwd)` walk
  `fleet run` uses, so production is unchanged and a test can drive the
  path without writing into the repo's own corpus (KI-091).
- **The auto-approve policy.** Nobody is there to approve a tool call in
  an unattended run, and an unattended run must not silently auto-approve
  a dangerous tool. `auto_approve` true declares the job unattended and
  its calls run (a run-scoped always, the way `--yes` is, which the
  screening row names `always`); false skips each call — the gate denies
  it, the first tool it denied is captured, the job's `last_outcome`
  records `skipped-needs-approval`, and the scheduler's line names the
  tool and the reason. The denial rides the ordinary `tool_result` path
  into the log with `approved: false`.
- **Retention and delivery.** Retention is the ordinary session log —
  resumable, listed by `fleet sessions` — plus the job's
  `last_run`/`last_outcome`. Delivery to a gateway-connected surface is
  deliberately not built and named as such in the feature doc: the crate
  has no gateway-connected surface, and a `deliver` field that silently
  did nothing would be a lie. The record's shape leaves the seam (the
  fork's delivery-receipt vocabulary is named there for the slice that
  builds it).
- **The surface.** `fleet cron` lists as cards (name, schedule, next
  fire, last run and outcome) in the house card style; `add` /
  `enable` / `disable` / `remove` mutate by name through the same
  validator, each printing the job's card beside the named line for what
  changed. The listing is a read surface (exit 0 with no jobs); a typed
  refusal prints its reason and exits 1.

## Evidence

- Gate, run at the end and read without a status-losing pipe:
  `cargo build --all-targets` exit 0, **zero warnings**; `cargo test`
  exit 0 — **371 unit + 88 integration passed, 1 ignored** in the final
  state (baseline 347 unit + 83 integration; nothing deleted, no test
  weakened). `Cargo.toml`/`Cargo.lock` unchanged. rustfmt/clippy still
  absent (KI-007) — style matched by hand.
- The unit tests pin the parser's dialect and every named validation
  error (field count, each range, `*/0`, an inverted range, `5/2`, a junk
  element, 7-as-Sunday), the Vixie day rule, both search directions, the
  unsatisfiable expression, the minute round trip and its rejections, the
  store's round trips and every typed refusal, the malformed store's five
  shapes, the tick's fire-and-latch, the catch-up for a missed window and
  for a served one, the never-run rule, the catch-up framing, and the
  card render.
- The integration tests drive the scheduler end to end against the
  scripted mock through an injected runner (the production shape with
  the route preflight taken out), so a whole pass is deterministic with
  no sys1 spawn and no clock: a job fires on its minute and only once
  (with the store's `last_run`/`last_outcome` recorded and the framed
  task in the run's own log), a job added a moment ago is picked up by
  the next tick, a missed window catches up exactly once with the
  catch-up named in the log, a never-run job does not catch up and spends
  nothing, and the auto-approve policy both ways (the skipped call is
  denied in the log and the outcome records the skip; the declared job's
  call runs and completes).
- **The real-binary scheduler-thread smoke.** A scratch `FLEET_HOME`
  with the scripted mock as the only enabled roster candidate (every
  other candidate explicitly disabled, the port file asserted to exist,
  and the config's port asserted equal to the port file's port — the
  KI-066 discipline) and one `* * * * *` job: `fleet chat` fired it on
  its minute, the run reached the mock (`model: mock-model` in the
  captured request, the framed task in the request body), the session log
  landed under the scratch home's sessions dir, and the job's
  `last_run`/`last_outcome` were recorded. **No live provider was
  touched.** The first attempt's `failed` outcome was the smoke's own
  under-provisioned script — an every-minute job keeps firing across the
  hold, so one scripted answer left the second fire with the mock's
  designed `mock exhausted` 500 — not a defect; the second attempt used a
  one-shot minute expression.
- The `fleet cron` surface was exercised by hand against a scratch home:
  the empty listing, `add` (with and without `--auto-approve`), the
  two-job listing, the store file's exact JSON, `disable`, `enable`,
  `remove`, and every typed refusal (`*/0`, a taken name, a bad name, a
  name not in the store, a non-JSON store, a record with an unparseable
  schedule).

## Review cycle

- Not run as a separate stage: this slice was built by a delegated
  subagent on the parent's base model, and the parent re-verifies the
  gate, the tests, the smoke, and the ledger rows itself. Everything
  above is stated so that re-verification is mechanical.

## Not done (slice 12+)

- Self-maintenance (12), diagnosis (13), boards (14), install (15).
- Delivery to a gateway-connected surface (the seam is in the record's
  shape; nothing silently does nothing).
- The minors logged: KI-087 (the KI-074 recurrence on the scheduled-run
  path, with the smoke pattern named as the trigger), KI-088 (the
  unattended denial's model-visible text names "the user"), KI-089 (the
  latch is per-process, so two schedulers on one machine can both fire),
  KI-090 (the TUI owes the cron fire a card rather than a stderr line),
  KI-091 (the first integration tests dirtied the repo's git-tracked
  corpus through the recall-hit write path; fixed by making the corpus
  root an explicit input).
