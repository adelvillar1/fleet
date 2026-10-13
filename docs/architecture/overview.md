# Fleet crate architecture

The whole binary is Rust across the modules below; dependencies are `ureq` (HTTP), `ratatui`/`crossterm` (TUI), `tokio` (async runtime, child processes), `serde`/`serde_json` — everything else is std.

## Module map

| Module | Responsibility |
|---|---|
| `main.rs` | argv dispatch, credential resolution, wiring for `run` / `chat` / `sessions` / `lanes` / `doctor` |
| `cli.rs` | the grammar: flags anywhere before the task words, usage errors name the offending flag |
| `api.rs` | chat-completions client: typed request/response, streaming SSE parse, tool declarations, `ApiError`/`ClientError` taxonomy (401 credentials, 402 balance, 429 rate limit, network, decode) |
| `agent.rs` | the tool loop, the `Approver` gate (`ApproveAll` / `DenyAll` / `ConsoleApprover` / `TuiApprover`), turn cap, `UiEvent` stream |
| `tools.rs` | the closed four-tool set; every failure is a model-visible result, never a panic |
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

1. Resolve credentials (env → `<FLEET_HOME>/.env` → harness credential store `refs`, read-only) and base URL.
2. Open or create the session log; replay seeds the message list, so a resumed run continues with its own prior answers in view.
3. Each turn: prepend the compiled-in system prompt, send messages + the four tool declarations, accumulate usage.
4. No tool calls → record `end` (`completed`), print the final answer, exit 0.
5. Tool calls → approval gate per call → execute → record `tool_result` with exactly the text the model receives (denials, unknown tools, malformed arguments, and tool errors are all model-visible results).
6. Turn cap reached → record `end` (`max-turns`), exit 3; resume with `--session <id>`.

## Session log format (v1)

One JSON object per line: `start` (v, model, created), `user`, `assistant` (content + tool_calls), `tool_result` (call_id, name, output, truncated, approved), `end` (stop_reason, turns). A log without `end` was interrupted. Replay fails loud, naming file and line. Bump `SESSION_FORMAT_VERSION` only when existing logs cannot be read safely.

## Invariants carried over from the harness

- **Model-visible ⟺ logged**: the log reconstructs exactly what the model saw; the system prompt and tool declarations are constants, reconstructed per request, never logged.
- Tool output is capped at 64 KiB on a char boundary with a `[truncated]` marker; `bash` has a 120s wall-clock cap and drains stdout/stderr on their own threads (no pipe-buffer deadlock).
- Credentials never print; `doctor` masks to the last four characters.

## Lane health (`fleet lanes`)

The four lane cores launch exactly as the fork's `dsh-components` bundle launches them: `python3 -u <home>/service/mcp_server.py`, `<SVC>_PYTHONPATH=<home>/src` in the child env, the lane's home as cwd. `src/mcp.rs` speaks newline-delimited JSON-RPC 2.0 (no Content-Length headers); a reader task routes replies to id-keyed waiters and ignores notifications, unknown ids, and the id-`null` stray sys1/gen1 emit (KI-002). Every wait carries a budget (initialize 10s, tools/list 10s, tools/call 60s); teardown closes stdin, waits 5s, then kills. `fleet lanes` probes each lane through the client and renders cards (headless ANSI, TUI palette); the TUI's Ctrl+L view renders the same `LaneHealth` rows. The agent's tool loop does not gain MCP tools at this slice — that is slice 5's roster routing.

## Testing posture

Zero real network by design: the integration suite spawns `mock-server` as a child process (ureq's reads misbehave when client and loopback server share a process on this machine) and asserts on both the run report and the recorded request bodies. The MCP suite drives the real vendored sem1 lane end to end (initialize, tools/list, a tools/call round trip, teardown; skipped with a loud note when `python3` is absent) plus `mock-mcp` for stray frames, crashed children, timeouts, and out-of-order replies. The one live-API test is `#[ignore]`d and self-skips without a key.
