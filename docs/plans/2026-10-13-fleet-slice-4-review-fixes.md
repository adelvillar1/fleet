---
status: done
created: 2026-10-13
updated: 2026-10-13
slug: fleet-slice-4-review-fixes
---

# Plan: Slice-4 quality-review fixes

## Context

Slice 4 landed in `f365d6c` (the MCP stdio client, the four vendored lane cores, `fleet lanes`, the TUI Ctrl+L modal). Its stage-2 quality review completed but the delivery was lost to a user interrupt; the findings were recovered from the live transcript. Verdict label unrecoverable — treated as **REQUEST_CHANGES**. No criticals. This plan closes the Important findings and the cheap minors, with tests, a green local gate, and a commit. No push.

## Acceptance criteria

- **AC1 (env inheritance, KI-031).** `src/mcp.rs`'s spawn path calls `env_clear()` and builds the child's environment explicitly: a minimal base (`PATH`, `HOME`, `LANG`, `TMPDIR` — each carried only when the caller has it) plus the spec's own entries plus a per-lane passthrough allowlist declared in `src/lanes.rs`. `DEEPSEEK_API_KEY` is in no allowlist.
- **AC2.** A unit test proves the constructed child environment excludes `DEEPSEEK_API_KEY` while including `PATH` and a lane allowlist entry, and an integration test proves it cross-process (the fake server reports its own environment back through the client).
- **AC3.** `./target/debug/fleet lanes` prints 4 ok and exits 0 after the change — run by hand, output captured. A lane that fails to boot means its allowlist was incomplete; the fix is to add what it needs, never to restore inheritance.
- **AC4 (stdin lock, review's Important).** `McpServer`'s `stdin` field is a `tokio::sync::Mutex<Option<ChildStdin>>`; write serialization and take-on-close behavior are unchanged; a test pins that a call future is `Send` and that two spawned tasks share one server.
- **AC5 (minors).** Poison recovery logs one line to stderr; a non-UTF-8 stdout line is its own typed error, not "closed its output"; a single stdout line is capped at 8 MiB with a typed protocol error beyond it; a relative `FLEET_SERVICES_ROOT` / `FLEET_LANE_*_HOME` fails with a typed error; the TUI lanes modal guards stale probe rows with a generation counter.
- **AC6 (test gaps).** Cheap tests for late-reply-after-timeout (the waiter entry is removed, the late reply is dropped cleanly) and teardown-during-call (closing while a call is in flight yields a typed error, not a hang).
- **AC7 (TUI parity).** The Ctrl+L modal renders each lane's latency and sizes its height from the row count instead of a fixed 62%.
- **AC8 (ledger).** KI-031 closes with evidence; every new finding gets its own row (next id after KI-032).

## Design decisions (binding)

- **The child environment is a decision, not an accident.** `tokio::process::Command` inherits the caller's whole environment, so today every lane child sees `DEEPSEEK_API_KEY` and every other credential-shaped name on the machine. `spawn_with` now calls `env_clear()` and builds the environment from `child_env(spec, parent)` — a pure function over an injected parent map, which is what makes the policy testable without mutating the test process.
- **Passthrough is an allowlist, declared per lane.** A lane names the ambient vars it legitimately needs: the credentials its providers read, and the locators for machine-local resources it cannot discover itself (a venv, an endpoint, a store root, a log root, an env file). Generic ambient configuration is not carried. `DEEPSEEK_API_KEY` appears in no lane's list.
- **The stdin lock is a tokio mutex.** The guard is held across the write's await; that is only sound — and only `Send` — with `tokio::sync::Mutex`. Slice 5 drives concurrent calls through `tokio::spawn`, which would not compile against a `std::sync::MutexGuard` held across an await. `Drop` uses `try_lock` (best effort, as `Drop` already is).
- **The reader bounds its own line.** `fill_buf`/`consume` with an explicit 8 MiB ceiling instead of `lines()`, which has no bound. One frame is one line; the biggest a lane legitimately emits is a batch of vectors, kilobytes. Past the cap the waiters fail with a typed `Protocol` error.
- **Staleness is the state machine's call.** The event loop cannot tell a fresh modal's rows from a previous probe's rows by mode alone — both arrive while the modal is open. A generation counter stamped at `open_lanes` and checked in `set_lanes` makes the decision where the rest of the view's decisions live.

## Work order

1. Plan (this file).
2. `src/mcp.rs`: `env_clear` + `child_env` + `passthrough` on `ServerSpec`; tokio stdin mutex; poison-recovery stderr line; bounded line reader with typed `NotUtf8` / `TooLong` ends.
3. `src/lanes.rs`: per-lane `passthrough`; absolute-path validation for `FLEET_SERVICES_ROOT` and `FLEET_LANE_*_HOME` with a typed `RootError`.
4. `src/tui/`: probe generation counter; modal latency and row-count height.
5. `src/bin/mock-mcp.rs`: a huge-line entry and an environment-echo entry for the new tests.
6. `tests/integration.rs` + unit tests for every criterion above.
7. Gate: `cargo build --all-targets` (warning-free), `cargo test` — exit codes read directly, never through a pipe. Then `./target/debug/fleet lanes`, output captured.
8. Ledger: close KI-031, add the new rows, update the KI-029 VENDOR.json note.
9. Commit with explicit paths; no push.

## Verification evidence

- `cargo build --all-targets` — exit code and warning count.
- `cargo test` — full summary lines.
- `./target/debug/fleet lanes` — full output, 4 ok, exit 0.
- `git show --stat` of the commit.

## Risks

- **A lane stops booting under `env_clear`.** Then its allowlist is incomplete; the fix is to read its source and add what it needs. Full inheritance is not a fallback.
- **`python3` resolution.** The lanes launch as `python3`, resolved through the child's PATH, so PATH must be in the base. Verified by the boot run.
- **An owner's `SYS1_*` config override is not carried** (sys1's config reads the whole `SYS1_` namespace; the allowlist is name-based). Logged as a new ledger row rather than silently dropped.
