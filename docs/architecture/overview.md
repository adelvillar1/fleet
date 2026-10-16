# Fleet crate architecture

> **Stale snapshot.** This document describes the slice-1 crate (four
> tools, four lanes, no router, no memory, no TUI settings). The
> application is now 45 modules and ~70k lines. The current-state
> survey is [deep-dive-2026-10-15.md](deep-dive-2026-10-15.md); the
> sections below remain accurate for the cores they name.

The whole binary is Rust across the modules below; dependencies are `ureq` (HTTP), `ratatui`/`crossterm` (TUI), `tokio` (async runtime, child processes), `serde`/`serde_json` — everything else is std.

## Module map

| Module | Responsibility |
|---|---|
| `main.rs` | argv dispatch, credential resolution, wiring for `run` / `chat` / `sessions` / `lanes` / `doctor` |
| `cli.rs` | the grammar: flags anywhere before the task words, usage errors name the offending flag |
| `api.rs` | chat-completions client: typed request/response, streaming SSE parse, tool declarations, `ApiError`/`ClientError` taxonomy (401 credentials, 402 balance, 429 rate limit, network, decode) |
| `agent.rs` | the tool loop, the `Approver` gate (`ApproveAll` / `DenyAll` / `ConsoleApprover` / `TuiApprover`), turn cap, `UiEvent` stream, and the run-scoped `LaneRoster` behind the lanes' tools |
| `tools.rs` | the closed four-tool set; every failure is a model-visible result, never a panic. Also the tool-declaration vocabulary: the built-ins, the `mcp__<svc>__<tool>` namespace, and the pure assembly of one request's declarations |
| `session.rs` | append-only JSONL at `<home>/sessions/<id>.jsonl`, format version 1 |
| `home.rs` | `FLEET_HOME` / `~/.fleet`, `.env` fallback, harness `refs` reader |
| `mcp.rs` | the hand-rolled MCP stdio client: spawn, initialize (2024-11-05), tools/list, tools/call, teardown; id-keyed pending waiters, per-op budgets, typed `McpError` |
| `lanes.rs` | the four vendored lane cores: home resolution (env override → bundled `services/`), launch specs, the `fleet lanes` health probe and card renderer |
| `doctor.rs` | the four checks as pure evaluators over gathered facts (hermetic in tests) |
| `net.rs` | TCP reachability probe for doctor |
| `tui/` | the ratatui surface: `app.rs` state machine, `events.rs` key mapping, `ui.rs` rendering, `mod.rs` the event loop over the async agent |
| `bin/mock-server.rs` | the scripted mock API the integration suite drives as a child process |
| `bin/mock-mcp.rs` | the scripted fake MCP stdio server the integration suite drives for the paths the real lanes cannot produce deterministically |
| `services/` | the four vendored Python lane cores (`sys1`, `sdm1`, `sem1`, `gen1`), each with its `VENDOR.json`; launch documented in `services/README.md` |

## Request flow (`fleet run "<task>"`)

1. Resolve credentials (env → `<FLEET_HOME>/.env`, and nothing else — no other harness's config) and base URL.
2. Open or create the session log; replay seeds the message list, so a resumed run continues with its own prior answers in view.
3. Each turn: prepend the compiled-in system prompt, send messages + the four tool declarations, accumulate usage.
4. No tool calls → record `end` (`completed`), print the final answer, exit 0.
5. Tool calls → approval gate per call → execute → record `tool_result` with exactly the text the model receives (denials, unknown tools, malformed arguments, and tool errors are all model-visible results).
6. Turn cap reached → record `end` (`max-turns`), exit 3; resume with `--session <id>`.

## The lanes' tools inside a run

A lane's tools join the model-visible tool set as `mcp__<svc>__<tool>` — the redundant lane prefix in the lane's own tool name is dropped, so sem1's `sem1_probe` is called as `mcp__sem1__probe`; a tool whose name does not start with the lane's keeps it. The four built-ins are always declared first.

Spawn is lazy and run-scoped (`agent.rs`'s `LaneRoster`): no lane process starts unless one of its tools is called, and every child a run spawns closes when the run ends, on every exit path. A lane that cannot spawn, cannot initialize, cannot list, or dies under a call is dead for the rest of the run — later calls fail fast with the recorded reason, and nothing respawns — while its tools stay declared, because the model saw them and the log's `lane_tools` record must keep matching the requests that followed. A lane failure is never fatal to a run: it is a typed failure result the model reads, in the same `error: …` vocabulary the built-ins use. A lane call goes through the same `Approver`, the same `Record::ToolResult`, and the same `Role::Tool` message as a built-in; a denied lane call spawns nothing.

Where a lane's launch spec comes from is injectable (`AgentOptions::lane_specs`): `None` is the production path, the vendored registry resolved by name (`lanes::spec_for`) with its named reason when a lane cannot launch. Tests inject a scripted fake's spec so the loop's lane paths run without a real lane.

## Session log format (v1)

One JSON object per line: `start` (v, model, created), `user`, `assistant` (content + tool_calls), `tool_result` (call_id, name, output, truncated, approved), `lane_tools` (lane, and each tool's model-visible name, the lane's own name, description, and inputSchema), `end` (stop_reason, turns). A log without `end` was interrupted. `lane_tools` is bookkeeping like `start`/`end`: it replays to no message, and it exists because the tool declarations are dynamic — a lane's tools enter the model's set mid-run, so the log is the only place the request the model saw can be reconstructed from. Replay fails loud, naming file and line. Bump `SESSION_FORMAT_VERSION` only when existing logs cannot be read safely.

## Invariants carried over from the harness

- **Model-visible ⟺ logged**: the log reconstructs exactly what the model saw; the system prompt and tool declarations are constants, reconstructed per request, never logged.
- Tool output is capped at 64 KiB on a char boundary with a `[truncated]` marker; `bash` has a 120s wall-clock cap and drains stdout/stderr on their own threads (no pipe-buffer deadlock).
- Credentials never print; `doctor` masks to the last four characters.

## Lane health (`fleet lanes`)

The four lane cores launch exactly as the fork's `dsh-components` bundle launches them: `python3 -u <home>/service/mcp_server.py`, `<SVC>_PYTHONPATH=<home>/src` in the child env, the lane's home as cwd. `src/mcp.rs` speaks newline-delimited JSON-RPC 2.0 (no Content-Length headers); a reader task routes replies to id-keyed waiters and ignores notifications, unknown ids, and the id-`null` stray sys1/gen1 emit (KI-002). Every wait carries a budget (initialize 10s, tools/list 10s, tools/call 60s); teardown closes stdin, waits 5s, then kills. `fleet lanes` probes each lane through the client and renders cards (headless ANSI, TUI palette); the TUI's Ctrl+L view renders the same `LaneHealth` rows. The lanes' tools are callable inside a run under the `mcp__<svc>__*` namespace, spawned lazily per run — see the section above.

## Testing posture

Zero real network by design: the integration suite spawns `mock-server` as a child process (ureq's reads misbehave when client and loopback server share a process on this machine) and asserts on both the run report and the recorded request bodies. The MCP suite drives the real vendored sem1 lane end to end — through the client (initialize, tools/list, a tools/call round trip, teardown) and through a full two-turn agent run against the mock API whose turn 1 calls `mcp__sem1__probe`, skipped with a loud note when `python3` is absent — plus `mock-mcp` for stray frames, crashed children, timeouts, out-of-order replies, a dead child mid-run, and an EOF marker that proves the run closed the child it spawned. The one live-API test is `#[ignore]`d and self-skips without a key.
