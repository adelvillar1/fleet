# Slice 6 — the calibration stream (2026-10-13)

**Plan:** `docs/plans/2026-10-13-fleet-everyday-harness.md` (slice 6).
**Feature:** `docs/features/calibration.md`.
**Ledger:** KI-048 (the log-root decision), KI-049, KI-050, KI-051.

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
  its way out and a wedged writer cannot hold a shutdown open — and the
  drain's expiry counts what it could not write rather than losing it
  silently.
- **Slice 5's `DecisionLog` is absorbed.** `route.rs` calls the stream;
  the writer has one home. The decision row's shape is unchanged.
- **Three graded ops.** `route` (slice 5's shape), `tool_screen` (one row
  per approval decision, recorded after the decision and never inside it),
  and `turn` (one row per run ending, graded ok/failed with the partial
  endings named). `eval_corpus` is reserved for slice 13.
- **The hash is devdec's.** `input_hash` is the 16 hex characters of
  SHA-256, hand-rolled in `calib::sha256` (no crypto dependency, one
  primitive, tested against the FIPS vector for `"abc"`). devdec's own
  rows name the field `input_sha256`; Fleet's is `input_hash` because the
  join is on the value, and the feature doc says so.
- **The first reader.** At route-prepare time the stream is read back:
  today's file, the last 512 rows, and a candidate with three consecutive
  failed attempts is demoted to the end of the session's failover walk.
  sys1's selection is untouched — demotion reorders the walk only, and
  the demotion rides in the decision row's `reason`. A served attempt
  resets the streak; the read is cross-process by intent (today's file is
  shared) and both facts are documented.
- **`fleet calib`.** Today's rows as cards, one per op with its counts,
  the writer's persisted counters (labeled last-writer, since the state
  file is last-writer-wins across processes), and the last few rows.
  `--day` for a past day, calendar-validated. No new TUI surface
  (documented in the feature doc).
- **`fleet doctor`** gained one informational line: the resolved
  calibration root and whether today's file exists — the cheapest
  mitigation for the hardcoded-literal hazard below.

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

- Gate, parent-run after each commit: `cargo build --all-targets` exit 0,
  warning-free; `cargo test` → **249 passed, 1 ignored (198 unit + 51
  integration)** in the final state. rustfmt and clippy are still absent
  on this machine (KI-007) and were not run.
- The AC's literal check, parent-verified: the external `devdec` CLI is
  installed (`~/.local/bin/dev-decisions`, v0.3.0). Real `fleet run`s
  landed rows in `~/.local/share/dev-decisions/logs/2026/10/13/events.jsonl`
  beside devdec's own, and `dev-decisions log --tail 3` prints Fleet's
  `route`/`tool_screen`/`turn` rows beside devdec's — exit 0, no adapter.
  (The bare `log` subcommand needs `--tail`; without it it prints nothing,
  a CLI quirk worth knowing.)
- The "a full disk never fails a decision" half: with the day's file made
  unwritable, a real run logged `the calibration append failed
  (best-effort, the run is unaffected): Permission denied (os error 13)`
  once per row and still completed with exit 0 and its answer on stdout.
- The demotion is exercised end to end by an integration test with four
  scripted mocks: three graded failures against the kit, sys1 judging
  `hard` (zai), and the walk ordering zai → mimo → stepfun with the
  demoted kit never tried.
- The review round's fixes are negative-probed: the state temp name is
  per-writer, and the drain-expiry counting is proven by a wedged-writer
  test (a reader-less FIFO blocks the writer in `open` until the budget
  expires, and the backlog lands in `dropped`).

## Review cycle

- **Stage 1 (spec, step-5-preview): SPEC-COMPLIANT** — all 12 checks; the
  one non-blocking gap (the streamed path's zero `total_tokens`, disclosed
  nowhere) was fixed by the parent in `e4d81fd` and folded into KI-019.
- **Stage 2 (quality, step-5-preview): APPROVED** — no criticals, no
  importants; twelve minors, eight taken in `c8678e2`: KI-050's false
  clause corrected (a serve DOES reset the demotion streak — the ledger
  must not invite a future reader to "fix" correct behavior), the state
  file's temp name made unique per writer, drain-expiry drops counted,
  the counter labeled last-writer, the crate's newest production `expect`
  removed, `--day` calendar-validated, three disclosure sentences added
  (signal death records no turn row; cross-process demotion is intended;
  the hash field's divergence joins on the value), and doctor's
  calibration line. The rest are disclosed or accepted.

## What the rows do not carry

No task text and no key. A row carries a name, a hash, and a count:
`route`'s `input_hash` hashes the task, `tool_screen`'s hashes the call's
arguments, and both keep the length beside the hash.

## Not done

- No decay window or cross-day streak on the demotion (KI-050): the
  threshold is today's last 512 rows and nothing more.
- `dev-decisions log`'s text renderer prints `?` for Fleet's rows' `repo`
  and `verdict` (KI-049) — cosmetic, and an owner decision.
- A signal death (Ctrl-C/SIGTERM) records no turn row at all; the crate
  has no signal handling, so `interrupted` is reachable only via I/O
  failures today. Documented, not built.
- The hardcoded devdec literal is a maintenance hazard (devdec moving its
  `LOG_DIR` silently re-routes the stream); doctor's line makes it
  visible, and nothing more.
- Slice 13's `eval_corpus` op is reserved and unimplemented, as designed.
