---
status: done
created: 2026-10-13
slug: fleet-slice-4-mcp-lanes
---

# Recap: slice 4 — the MCP stdio client + the vendored lane cores

Plan: [2026-10-13-fleet-everyday-harness.md](../plans/2026-10-13-fleet-everyday-harness.md). All six acceptance criteria met; gate green.

## What landed

- **`src/mcp.rs`** — the hand-rolled async MCP stdio client. `McpServer::spawn(spec)` runs the child under `tokio::process`; a reader task drains stdout into an id-keyed pending map; writes are newline-delimited JSON-RPC 2.0 (no Content-Length headers). `initialize` (protocol 2024-11-05, then the `notifications/initialized` notification), `list_tools`, `call_tool`. Budgets: initialize 10s, tools/list 10s, tools/call 60s. Teardown: `close()` closes stdin, waits 5s, kills on expiry; `Drop` is the best-effort net. Errors are typed `McpError` values — a dead child fails its waiter with `Closed`, a silent one with `Timeout`; never a hang, never a panic.
- **`src/lanes.rs`** — the registry (sys1/sdm1/sem1/gen1), home resolution in the fork's order (`FLEET_LANE_<SVC>_HOME` → bundled `services/`, overridable with `FLEET_SERVICES_ROOT` or discovered by walking up from the executable), launch specs replicating the fork's launcher exactly (`python3 -u <home>/service/mcp_server.py`, `<SVC>_PYTHONPATH=<home>/src`, cwd = home), the sequential health probe, and the headless ANSI card renderer in the TUI palette.
- **`services/`** — the four lane cores vendored from the fork's working tree (`__pycache__` excluded per the VENDOR.json discipline), each `VENDOR.json` carried with a `fleetVendor` provenance note; sys1's note records that the copy matches the upstream *working tree* with its 4 uncommitted files (KI-003), not the pinned commit. `services/README.md` documents the per-lane launch in English.
- **`fleet lanes`** — probes every lane through the client and renders bordered cards with status dots, role/version/tool-count, tool names, and the launch command; exit 0 only when all four are ok. Parent-run against the final tree: `4 lanes · 4 ok · 0 not ok`, exit 0 — **with the child environment scrubbed** (see the review round), the real proof the passthrough allowlists are complete.
- **TUI Ctrl+L** — `Mode::Lanes` in the state machine, the prober on its own thread with a current-thread runtime (no runtime-context blocking, KI-021's class), rows arriving over a channel drained each tick with a probe generation counter against staleness, a bordered modal with dots, tool names, and latency.
- **Tests** — unit coverage for framing/correlation/parsing in `mcp.rs` (including the KI-002 id-`null` stray), `lanes.rs` coverage including the env construction, and app/events/ui coverage for the lanes view. Integration: the real sem1 driven end to end (initialize, tools/list, a `-32602` refusal, the `sem1_probe` success path, clean teardown), the registry probe over the real lane, fake-server tests via `src/bin/mock-mcp.rs` (stray tolerance, dead child, silent child, out-of-order concurrent replies, late reply after timeout, a child that goes away mid-call), plus binary-level `fleet lanes` tests (real lanes, wrong root → exit 1, home override). Final totals: **126 unit + 34 integration passed, 1 ignored**, zero build warnings. The 110 pre-existing tests all still pass, none deleted.
- **Dependencies unchanged** — no new crates; tokio gains its `process` feature only.

## Review cycle

- **Stage 1 (spec, step-5-preview): SPEC-COMPLIANT** — all 12 checks; vendored fidelity verified by `diff -r` against the fork (byte-exact `.py`; only VENDOR.json/`__pycache__` differences); scope exactly the expected file set.
- **Stage 2 (quality, step-5-preview): REQUEST_CHANGES** — its first delivery was lost to a user interrupt and recovered from the live transcript; narrowly scoped, no criticals. Both Important findings fixed in `7bb8140`:
  1. **Lane children inherited the full ambient environment** including `DEEPSEEK_API_KEY` — an accident of `tokio::process`, not a decision. Now `env_clear()` + a minimal base (PATH/HOME/LANG/TMPDIR, carried only when the caller has them) + a per-lane passthrough allowlist derived from each lane's source (sys1's provider keys and state locators, sdm1's TABPFN, sem1's ST venv and vectors dir, gen1's DASHSCOPE/STEPFUN/kokoro locators); `DEEPSEEK_API_KEY` is on no list, and a test asserts that.
  2. **`write_frame` held a `std::sync::Mutex` guard across the write's awaits** — a `!Send` future that would have failed to compile under slice 5's concurrent calls, plus a stall shape on the current-thread runtime. Now `tokio::sync::Mutex`; write serialization unchanged.
- Minors fixed: poison recovery logs to stderr; the reader distinguishes closed / not-valid-UTF-8 / past the 8 MiB line cap; relative `FLEET_SERVICES_ROOT`/`FLEET_LANE_*_HOME` fail typed; the probe generation counter; the modal gained latency parity and a row-count-sized height; KI-029's note clarified.

## Ledger

KI-028–KI-032 added at implementation (sys1/gen1's nested error shape; the sys1 working-tree provenance; health probes MCP surface only; lane env inheritance; the bare `cargo run` doc breakage). KI-002 closed — handled at the client boundary with test evidence. The review round closed KI-031 and added KI-033–KI-040 (the `!Send` write lock, the non-UTF-8 message, the missing line cap, relative homes, probe staleness — all fixed; plus KI-038, a pre-existing `temp_home` scratch-collision flake found by the new line-cap test, KI-039 sys1's arbitrary `SYS1_*` namespace, KI-040 `McpError` can't derive `Clone` — logged, open).

## Not done / deferred

- The agent's tool loop gains no MCP tools this slice — slice 5's roster routing owns that (plan scope). The hardened write path and id-keyed correlation were built for exactly that.
- CLAUDE.md's narrative sections still read "slice 1 … 55 passed": the edit is gated on interactive approval, which timed out again this session (silence is not consent). The module map and common-commands sections did land earlier. The accurate state is this recap, the plan doc, and `docs/architecture/overview.md`.
- Provider-level lane health (llama-server, TABPFN key, GLiNER/kokoro venvs) is not probed — KI-030.
- rustfmt/clippy still unavailable on this machine (KI-007); style matched by hand.
- The `fleet lanes` card wraps the launch command mid-token — lossless by design and documented; the reviewer's call was leave.
