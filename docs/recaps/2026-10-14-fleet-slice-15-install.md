# Slice 15 — install and launch (2026-10-14)

**Plan:** `docs/plans/2026-10-13-fleet-everyday-harness.md` (slice 15, the
last). **Feature:** `docs/features/install.md`. **Ledger:** KI-127..KI-134.

## What landed

`src/install.rs` (new, ~1,400 lines with its tests) plus the CLI, the
dispatch, the README, and `tests/install.rs` (11 acceptance tests driving
the real binary).

- **The home layout**, idempotent: `sessions`, `memories`, `skills`,
  `cron`, `proposals`, each named created or present, never destroyed; a
  file where a directory belongs is a named hard failure, never a
  silent overwrite.
- **The config scaffold**: exactly `{\n  "pin": null,\n  "providers":
  []\n}\n` — the roster's override seam — written only when absent and
  never rewritten; an existing file (even invalid JSON, even a dangling
  symlink) is named and left exactly as it is.
- **The heavy-lane venvs**, ported natively from the fork's
  `services/bootstrap.mjs`: `uv venv --python 3.12` (the fork's pin) or
  `python3 -m venv` when uv is absent, then `uv pip install` / the
  venv's own pip; one venv per vendored core under
  `<home>/venvs/<lane>/`; the wiring named (`SEM1_ST_VENV`,
  `SDM1_MODELS_VENV`, `GEN1_KOKORO_PYTHON`, sys1's `config.toml`
  `local_venv` key). Present ⇒ detected, not rebuilt. Degrade-without:
  a lane that cannot build is skipped with its reason named, the install
  stays green, and the lane stays declared and dead — slice 4's
  dead-lane behavior.
- **Bounded and streamed**: every lane subprocess runs under
  `LANE_INSTALL_TIMEOUT_SECS = 900` (kill, clean, skip-with-reason on
  expiry), both pipes drained on detached reader threads so a chatty
  child cannot deadlock the wait, and the child's lines stream to the
  operator as they arrive — the report's head prints before the
  bootstrap and its tail after, so a multi-minute install can neither
  hang forever nor look frozen.
- **The receipt**: `venvs/<lane>/.fleet-lane` — the lane's name and
  role, its package list, the UTC minute — written once the packages
  land. A venv is Present only when `bin/python` exists **and** the
  receipt names the lane, so an interrupted run's half-built venv is
  rebuilt or skipped, never misread as installed; the removal is
  conditional on the directory not having existed before this run.
- **Doctor as the gate**: one shared `doctor_checks()`; the install adds
  no check and its exit code is the doctor's (green 0; a named failure
  nonzero — proven both ways).
- **The release seam**: `cargo build --release` is the release path;
  per-OS cross artifacts are named as the not-built-yet (the CI is
  disarmed by owner decision) with the cross-build commands recorded.
- **The README's quick start is literally true** — build → install →
  doctor → chat, every command real — and proven by execution against a
  scratch FLEET_HOME with the release binary: install exit 0 with a
  green doctor, a second run naming every step present with the
  scaffold's bytes untouched, and `fleet chat` completing a real task
  end to end (routed to the live router kit, exit 0).

## Evidence

- Gate, parent-run after each commit: `cargo build --all-targets` exit 0,
  **zero warnings**; `cargo test` exit 0 — **508 unit + 2 + 7 + 11 + 90
  integration, 2 ignored** (the live-API e2e and the boards' live read);
  nothing deleted.
- The real `fleet install --lanes` on this machine built all four venvs
  with the machine's own `uv` (Python 3.12): 2.4 GB with the torch
  stack; a second run named all four present; sem1's python imports
  `sentence-transformers` 6.1.0; sys1's imports `gliner2.classification`
  with no manual step.
- The vendoring-boundary fix: the bare `gliner2` the fork's bootstrap
  and sys1's own setup note name leaves torch uninstalled
  (`ModuleNotFoundError` at `gliner2/models/base.py:21` — the classifier
  imports torch at module load), so the lane could not serve at all.
  Fleet's port installs `gliner2[local]`; the upstream carry is logged
  (KI-127).
- The degrade-without proofs: a controlled PATH with only `sh` skips all
  four lanes with per-lane reasons, exit 0; a failing pip step skips with
  the reason named, exit 0, and leaves no venv behind.

## Review cycle

- **Stage 1 (spec, step-5-preview): SPEC-COMPLIANT** — all nine checks;
  the AC's chat half hand-proven and recorded with the deterministic
  core in the suite, disclosed as out-of-suite because it is
  non-deterministic.
- **Stage 2 (quality, step-5-preview): REQUEST_CHANGES** — no criticals;
  two Importants, both fixed in `abfa2f9`: the unbounded silent lane
  subprocesses (now bounded and streamed, KI-129) and the interrupted
  run's venv misread as Present (now the receipt, KI-130). Plus the
  minors: the dangling-symlink write-through (KI-131), the invisible
  degradation (now a summary line, KI-132), and the network wording
  (KI-133).
- The review also surfaced a **pre-existing** defect the slice
  faithfully preserved: the doctor's roster check prints FAIL and exits
  0 on an invalid config. Logged as KI-134 for the doctor's owner
  rather than quietly changed mid-slice.
- **Both reviews dispatched in ONE parallel call** — the pattern's
  sixth use.

## The plan's fifteen slices

Slices 2-15 are landed and pushed. The remaining work is the converge
pass: re-reading the plan's acceptance criteria (C1-C10) against the
actual code and closing anything partial or missing.
