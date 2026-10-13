---
status: done
created: 2026-10-13
updated: 2026-10-13
slug: fleet-slice-4-mcp-lanes
---

# Plan: Slice 4 — the MCP stdio client + the vendored lane cores

## Context

Slices 1–3 landed the async agent core, streaming, and the ratatui TUI (92 unit + 18 integration tests green, zero build warnings). The four house lanes — sys1 (judgment), sdm1 (tabular), sem1 (embeddings), gen1 (media) — live in the deepseek-harness fork under `services/<lane>/` as stdlib-only Python MCP stdio servers, boot-verified 2026-10-13. This slice brings the plumbing into Fleet: a hand-rolled MCP stdio client, the vendored cores, and a health surface. It implements the "Slice 4 — MCP stdio client + the vendored cores" section of [2026-10-13-fleet-everyday-harness.md](2026-10-13-fleet-everyday-harness.md) with one deliberate scoping difference: **the agent's tool loop does not gain MCP tools in this slice** (that is slice 5's roster routing). This slice is the client, the vendored cores, and lane health — plumbing plus visibility, not tool exposure.

## Acceptance criteria

- **AC1** — `src/mcp.rs`: an async MCP stdio client that spawns a server child, performs `initialize` (protocol version 2024-11-05; tolerates servers that answer `notifications/initialized` with `-32601` — sys1 and gen1 do), `tools/list`, `tools/call`, and clean teardown (close stdin, wait with a timeout, kill). Newline-delimited JSON-RPC 2.0 framing (no Content-Length headers); requests correlate by id; notifications carry none; unknown notifications and id-less stray frames are ignored.
- **AC2** — the four lane cores vendored at `services/<lane>/` from the fork's working tree (including each `VENDOR.json`), with a documented launch per lane that works from the vendored copy.
- **AC3** — `fleet lanes`: a headless command listing each lane's health — spawn through the client, initialize, tools/list, report state and tool count, tear down. Genuinely visual: cards with status dots and real layout, never a plain-text table; the TUI's palette is the reference.
- **AC4** — the TUI ships its surface: a lanes view reachable with Ctrl+L, reusing the picker/modal pattern (app.rs state machine + mod.rs effects + ui.rs rendering + events.rs key mapping).
- **AC5** — integration tests drive a real vendored lane end to end (sem1: initialize, tools/list, a tools/call round trip, teardown) plus a scripted fake MCP server in the mock-server style for the paths the real lanes cannot produce deterministically (stray id-null frames, crash → error not hang, timeout).
- **AC6** — headless run/chat/sessions/doctor and the TUI unchanged in behavior; all 110 existing tests (92 unit + 18 integration) stay green, none deleted.

## Design decisions (binding)

- **Hand-rolled client, no rmcp/mcp crate.** `src/mcp.rs` with an async API: `McpServer::spawn(lane) -> Result<McpServer>`, `initialize()`, `list_tools()`, `call_tool(name, args)`, `close()`/`Drop` teardown. tokio::process for the child; a reader task drains stdout into an id-keyed pending-response map; writes are newline-delimited JSON to stdin. Timeouts on every wait — initialize 10s, tools/list 10s, tools/call 60s (interpreter startup plus stdlib imports is sub-second on this machine; tool calls do real work — embeddings, classification, image generation — so they get the long budget). Errors are typed values, never panics; a crashed child is an error, not a hang.
- **Lane registry `src/lanes.rs`.** The four lanes with vendored paths, launch commands (`python3 -u <home>/service/mcp_server.py`, working dir = the vendored lane dir, `<SVC>_PYTHONPATH=<home>/src` in the child env — replicated faithfully from the fork's `dsh-components` launcher), and VENDOR.json parsing. Home resolution mirrors the fork: per-lane env home override → bundled `services/` root (env `FLEET_SERVICES_ROOT` override, else discovered by walking up from the executable) → unavailable with a named reason.
- **`fleet lanes` renders from `src/lanes.rs` + `src/mcp.rs`** as ANSI cards in the TUI palette; the Ctrl+L modal renders the same `LaneHealth` data through the app state machine.
- **Dependency list stays minimal**: std + tokio + serde/serde_json, all already present. Nothing new.

## Work order

1. Plan (this file).
2. Read the fork's launcher, VENDOR.json pins, and entry points (done during planning — recorded below).
3. Boot-verify each lane from a scratch copy with the documented launch (done during planning — recorded below).
4. Vendor into `services/`; write `services/README.md`; carry VENDOR.json with the working-tree provenance note.
5. `src/mcp.rs` + `src/lanes.rs` with unit tests for JSON-RPC framing, correlation, and VENDOR.json parsing.
6. `fleet lanes` + the Ctrl+L modal.
7. `src/bin/mock-mcp.rs` (scripted fake) + integration tests driving the real sem1.
8. Full gate: `cargo build` (warning-free), `cargo test` — exit codes read directly, never through a pipe.
9. Commit with explicit paths; no push.

## Verification evidence captured during planning

Per-lane boot from a scratch copy of the fork's `services/` tree, launched exactly as the fork's launcher does (`python3 -u <home>/service/mcp_server.py`, `<SVC>_PYTHONPATH=<home>/src`, cwd `<home>`, python3 = /usr/local/bin/python3 3.13.6):

| Lane | initialize | tools/list | Notes |
|---|---|---|---|
| sys1 | ok | 1 tool (`sys1_classify`) | answers `notifications/initialized` with an id-null `-32601` stray frame (KI-002) |
| sdm1 | ok | 2 tools (`sdm1_classify`, `sdm1_health`) | clean |
| sem1 | ok | 4 tools (`sem1_embed`, `sem1_probe`, `sem1_index`, `sem1_search`) | clean — the integration candidate |
| gen1 | ok | 1 tool (`gen1_imagine`) | same id-null `-32601` stray as sys1 |

The fork's vendored copies match the upstream repos' working trees exactly; sys1's copy carries the 4 uncommitted files of KI-003 (89 insertions beyond the pinned commit c1f73836). That provenance is recorded in the carried VENDOR.json.

## Risks

- **python3 absent on a future machine** → lanes report `unavailable` with the reason named; the command stays green-ish (exit 1) and the rest of Fleet is untouched (AC6).
- **Heavy provider lanes absent** (sem1's llama-server, sys1's GLiNER venv, gen1's kokoro) → the cores still boot (stdlib-only); tools that need a provider fail at call time. `fleet lanes` reports MCP-surface health only; provider reachability is out of this slice's scope (logged as an improvement).
