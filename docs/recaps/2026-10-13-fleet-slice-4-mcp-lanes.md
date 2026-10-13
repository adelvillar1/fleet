---
status: done
created: 2026-10-13
slug: fleet-slice-4-mcp-lanes
---

# Recap: slice 4 — the MCP stdio client + the vendored lane cores

Plan: [2026-10-13-fleet-slice-4-mcp-lanes.md](../plans/2026-10-13-fleet-slice-4-mcp-lanes.md). All six acceptance criteria met; gate green.

## What landed

- **`src/mcp.rs`** — the hand-rolled async MCP stdio client. `McpServer::spawn(spec)` runs the child under `tokio::process`; a reader task drains stdout into an id-keyed pending map; writes are newline-delimited JSON-RPC 2.0 (no Content-Length headers). `initialize` (protocol 2024-11-05, then the `notifications/initialized` notification), `list_tools`, `call_tool`. Budgets: initialize 10s, tools/list 10s, tools/call 60s (interpreter startup is sub-second; tool calls do real work). Teardown: `close()` closes stdin, waits 5s, kills on expiry; `Drop` is the best-effort net (EOF + `start_kill`). Errors are typed `McpError` values — a dead child fails its waiter with `Closed`, a silent one with `Timeout`; never a hang, never a panic.
- **`src/lanes.rs`** — the registry (sys1/sdm1/sem1/gen1), home resolution in the fork's order (`FLEET_LANE_<SVC>_HOME` → bundled `services/`, itself overridable with `FLEET_SERVICES_ROOT` or discovered by walking up from the executable), launch specs replicating the fork's launcher exactly (`python3 -u <home>/service/mcp_server.py`, `<SVC>_PYTHONPATH=<home>/src`, cwd = home), the sequential health probe, and the headless ANSI card renderer in the TUI palette.
- **`services/`** — the four lane cores vendored from the fork's working tree (`__pycache__` excluded per the VENDOR.json discipline), each `VENDOR.json` carried with a `fleetVendor` provenance note; sys1's note records that the copy matches the upstream *working tree* with its 4 uncommitted files (KI-003), not the pinned commit. `services/README.md` documents the per-lane launch in English.
- **`fleet lanes`** — probes every lane through the client and renders bordered cards with status dots, role/version/tool-count, tool names, and the launch command; exit 0 only when all four are ok. Smoke: `4 lanes · 4 ok · 0 not ok`, exit 0.
- **TUI Ctrl+L** — `Mode::Lanes` in the state machine, the prober on its own thread with a current-thread runtime (no runtime-context blocking, KI-021's class), rows arriving over a channel drained each tick, a bordered modal with dots and tool names. Verified in a real tmux PTY: modal opens, all four lanes render with green dots, Esc closes, q quits.
- **Tests** — 10 unit tests for framing/correlation/parsing in `mcp.rs` (including the KI-002 id-`null` stray), 8 in `lanes.rs`, plus app/events/ui coverage for the lanes view. Integration: the real sem1 driven end to end (initialize, tools/list, a `-32602` refusal, the `sem1_probe` success path, clean teardown), the registry probe over the real lane, and 4 fake-server tests via the new `src/bin/mock-mcp.rs` (stray tolerance, dead child, silent child, out-of-order concurrent replies), plus 3 binary-level `fleet lanes` tests (real lanes, wrong root → unavailable exit 1, home override). Totals: **142 passed, 1 ignored (115 unit + 27 integration)**, zero build warnings. The 110 pre-existing tests all still pass, none deleted.
- **Dependencies unchanged** — no new crates; tokio gains its `process` feature only.

## Ledger

KI-028–KI-032 added (sys1/gen1's nested error shape; the sys1 working-tree provenance; health probes MCP surface only; lane children inherit ambient credentials; the bare `cargo run` doc breakage). KI-002 closed — handled at the client boundary, with the test evidence named.

## Not done / deferred

- The agent's tool loop gains no MCP tools this slice — slice 5's roster routing owns that (plan scope).
- CLAUDE.md's "Today's state" section still reads "slice 1 … 55 passed": the edit is gated on interactive approval, which timed out this session. The module map and common-commands sections of the same file did land. The accurate state is this recap, the plan doc (status: done), and `docs/architecture/overview.md`.
- Provider-level lane health (llama-server, TABPFN key, GLiNER/kokoro venvs) is not probed — KI-030.
- rustfmt/clippy still unavailable on this machine (KI-007); style matched by hand.
