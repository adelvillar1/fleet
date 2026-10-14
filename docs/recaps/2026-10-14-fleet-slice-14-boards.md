# Slice 14 — the boards (2026-10-14)

**Plan:** `docs/plans/2026-10-13-fleet-everyday-harness.md` (slice 14).
**Feature:** `docs/features/boards.md`. **Ledger:** KI-115..KI-126.

## What landed

`src/boards.rs` (new, 2,568 lines) — three read-only boards in the TUI's
board pane — plus the TUI wiring (Ctrl+B cycles; the pane title and the
status bar follow) and the refresher.

- **The router board** reads the kit's operator API at
  `127.0.0.1:8300/api/{state,usage,suggest}`: the host, the judge mode,
  the payg guard, the workload tier chains with their resolved primary
  and fallback counts, the last served calls with a status dot each,
  the ledger's calls/ok%/errors/tokens, the judge and cascade counters,
  the workload mix, and per-model call bars. The kit's `localToken`
  resolves through the kit's own paths chain
  (`$AGNOSTIC_ROUTER_DIR` → `$AGNOSTIC_ROUTER_KIT_HOME` →
  `~/.agnostic-router-kit` → **`~/.zcode/router`**, where this machine's
  live router actually renders its runtime — KI-116), with
  `AGNOSTIC_ROUTER_URL`/`TOKEN` overriding through the same env→`.env`
  chain as every other credential. The token never enters a snapshot, a
  log line, or the repo — now a tested invariant, not a prose claim.
- **The second-brain board** reads the six derived state files
  (`gates_state`, `harness_state`, `loops_state`, `reports_state`,
  `maturity_state`, `tower_data`) as cards: gate verdicts with graded
  progress and fit chips, the HITL inbox, the loops, a maturity
  sparkline, and the doing/conflict counts.
- **The tower** cards every project under `~/Projects` (bounded at 64):
  branch, dirty dot and count, head sha and subject, 30-day commit
  count, ahead/behind — dirtiest first, with a `+N more` marker.
- **The snapshot contract** (the AC): each board keeps its last good
  render; a refused read — a dead port, a bad token, a missing or
  wrong-shape state file, a git failure — keeps the snapshot visible
  and names the reason at the pane's top with the last good read's
  minute. A never-read board shows its empty state naming the reason.
  Per-read refusals inside a snapshot name themselves and the rest
  still renders. Read-only everywhere, pinned by a before/after
  fingerprint test.
- **The refresher**: one thread, its own current-thread runtime, all
  three boards read concurrently on the blocking pool at start and
  every `BOARD_REFRESH_SECS = 30`, plus on demand; `READ_TIMEOUT_SECS
  = 8`, `GIT_TIMEOUT_SECS = 5`, `MAX_PROJECTS = 64`.
- **No new dependencies** (Cargo.toml/Cargo.lock untouched) and no
  embedded web server — the plan's "and/or" allows the TUI panes, and
  the browser face is named as the not-built-yet with its contract
  ported as the refresher.

## Evidence

- Gate, parent-run after each commit: `cargo build --all-targets` exit 0,
  **zero warnings**; `cargo test` exit 0 — **508 unit + 2 + 7 + 11 + 90
  integration** in the final state (baseline 451 + 2 + 7 + 90; nothing
  deleted; 29 new tests for the boards plus the fix round's).
- The ignored live read against the machine's real sources: the router
  board saw 5 tiers, the judge cascade, 19,873 calls and 8 models; the
  second-brain board landed 6 of 6 state files; the tower carded 30
  repos (25 dirty, 5 skipped). A PTY smoke of the real TUI cycled all
  three boards.
- Each board renders from its source under test: a scripted three-route
  kit, fixture state files, a scratch git repo — plus every refusal path
  (dead port, bad token with the kit's own sentence, missing file,
  wrong-shape file, unlistable root, broken gitdir).

## Review cycle

- **Stage 1 (spec, step-5-preview): SPEC-COMPLIANT** — all checks; the
  visual bar judged met (tier rows with fallback counts, status dots,
  per-model bars, a maturity sparkline, per-project cards — never a
  plain-text table).
- **Stage 2 (quality, step-5-preview): REQUEST_CHANGES** — no criticals;
  five Importants, all fixed in `3cfa187`:
  1. no aggregate bound on the tower scan (30 × 5 sequential bounded
     calls could freeze a snapshot for ~12.5 minutes and stretch every
     later cadence) — now `TOWER_SCAN_BUDGET_SECS = 20` with the
     truncation named ("N of M projects — the scan budget was spent");
  2. the router board's field-shape drift was silent while the brain
     board named it — now `state_drift`/`UsageRead::drift` name each
     absence and each dropped row, and the live read carries a drift
     canary;
  3. the second-brain read was unbounded — now
     `BRAIN_READ_BUDGET_SECS = 10` across the six files plus
     `FILE_READ_TIMEOUT_SECS = 5` per file on a reader thread;
  4. a healthy board never showed its read time — now the header line
     carries it at both widths;
  5. two prose-only invariants untested — the refresher surviving a
     panicking read (the crate's hard-won CatchUnwind lesson), and the
     token never entering a snapshot or rendered text.
  Plus the minors: Ctrl+B's dedupe (one press had been costing two
  scans), the 80-column board assertion, KI-118's 4→5-call evidence
  correction, and KI-126 for the live test's machine-specific port pin.

## Not done (slice 15)

- The fork's browser face (the seam is named); `/api/workflow-runs`
  (this kit answers 404 — the board reads state/usage/suggest).
- KI-116 (the kit's deployed runtime root is machine-specific),
  KI-117 (serde_json's sorted map defeats payload ordering), KI-118
  (the tower's fixed background cost), KI-126.
