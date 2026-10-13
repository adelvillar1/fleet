# Slice 6 — the calibration stream (2026-10-13)

**Plan:** `docs/plans/2026-10-13-fleet-everyday-harness.md` (slice 6).
**Feature:** `docs/features/calibration.md`.
**Ledger:** KI-048 (the log-root decision), KI-049, KI-050.

## What landed

`src/calib.rs` — the calibration stream, and Fleet's audit spine.

- **The writer.** One `events.jsonl` per UTC day under dev-decisions' own
  log root, in devdec's row vocabulary, through a bounded `sync_channel`
  (1024 rows) drained by one background thread. `record` is `try_send`:
  enqueue never blocks, a full queue drops and counts, a write failure
  logs and counts, and a decision is never blocked or failed by its own
  audit trail. One `write_all` per row under `O_APPEND` keeps two
  processes from interleaving a row. The last handle's drop drains the
  queue, bounded at two seconds, so a row is never lost to a process on
  its way out and a wedged writer cannot hold a shutdown open.
- **Slice 5's `DecisionLog` is absorbed.** `route.rs` calls the stream;
  the writer has one home. The decision row's shape is unchanged.
- **Three graded ops.** `route` (slice 5's shape), `tool_screen` (one row
  per approval decision, recorded after the decision and never inside it),
  and `turn` (one row per run ending, graded ok/failed with the partial
  endings named). `eval_corpus` is reserved for slice 13.
- **The hash is devdec's.** `input_hash` is the 16 hex characters of
  SHA-256, hand-rolled in `calib::sha256` (no crypto dependency, one
  primitive, tested against the FIPS vector for `"abc"`).
- **The first reader.** At route-prepare time the stream is read back:
  today's file, the last 512 rows, and a candidate with three consecutive
  failed attempts is demoted to the end of the session's failover walk.
  sys1's selection is untouched — demotion reorders the walk only, and
  the demotion rides in the decision row's `reason`.
- **`fleet calib`.** Today's rows as cards, one per op with its counts,
  the writer's persisted counters, and the last few rows. `--day` for a
  past day. No new TUI surface (documented in the feature doc).

## The root, and why

dev-decisions names its log root with neither an env var nor a config
key: `LOG_DIR` is a hardcoded literal in `scripts/devdec/config.py`, and
the only overridable paths in its whole config surface are `providers.*`.
Slice 5's placement (`<FLEET_HOME>/logs`) therefore stranded Fleet's rows
where no devdec reader would ever look. The stream now resolves:
`config.json`'s `calib.log_root` → dev-decisions' own literal
`<user home>/.local/share/dev-decisions/logs` → `<FLEET_HOME>/logs` when
no user home is known. Recorded as KI-048.

## Evidence

- Gate: `cargo build --all-targets` exit 0, warning-free; `cargo test` →
  245 passed, 1 ignored (194 unit + 51 integration). rustfmt and clippy
  are still absent on this machine (KI-007) and were not run.
- The AC's literal check: the external `devdec` CLI is installed
  (`~/.local/bin/dev-decisions`, a shim over
  `~/Projects/dev-decisions/scripts/dev_decisions.py`, v0.3.0). Two real
  `fleet run` invocations landed 5 rows in
  `~/.local/share/dev-decisions/logs/2026/10/13/events.jsonl` beside 1313
  of devdec's own, and `dev-decisions log` printed them in both text and
  JSON formats with exit 0 — the rows are read beside devdec's own with
  no adapter.
- The "a full disk never fails a decision" half: with the day's file made
  unwritable, a real run logged `the calibration append failed
  (best-effort, the run is unaffected): Permission denied (os error 13)`
  once per row and still completed with exit 0 and its answer on stdout.
- The demotion is exercised end to end by an integration test with four
  scripted mocks: three graded failures against the kit, sys1 judging
  `hard` (zai), and the walk ordering zai → mimo → stepfun with the
  demoted kit never tried.

## What the rows do not carry

No task text and no key. A row carries a name, a hash, and a count:
`route`'s `input_hash` hashes the task, `tool_screen`'s hashes the call's
arguments, and both keep the length beside the hash.

## Not done

- No decay window or cross-day streak on the demotion (KI-050): the
  threshold is today's last 512 rows and nothing more.
- `dev-decisions log`'s text renderer prints `?` for Fleet's rows' `repo`
  and `verdict` (KI-049) — cosmetic, and an owner decision.
- Slice 13's `eval_corpus` op is reserved and unimplemented, as designed.
