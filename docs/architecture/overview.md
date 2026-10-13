# Fleet crate architecture (slice 1)

The whole binary is ~2,300 lines of Rust across eleven modules; dependencies are `ureq` (HTTP), `serde`, `serde_json` — everything else is std.

## Module map

| Module | Responsibility |
|---|---|
| `main.rs` | argv dispatch, credential resolution, wiring for `run` / `chat` / `sessions` / `doctor` |
| `cli.rs` | the grammar: flags anywhere before the task words, usage errors name the offending flag |
| `api.rs` | chat-completions client: typed request/response, tool declarations, `ApiError`/`ClientError` taxonomy (401 credentials, 402 balance, 429 rate limit, network, decode) |
| `agent.rs` | the tool loop, the `Approver` gate (`ApproveAll` / `DenyAll` / `ConsoleApprover`), turn cap, progress lines |
| `tools.rs` | the closed four-tool set; every failure is a model-visible result, never a panic |
| `session.rs` | append-only JSONL at `<home>/sessions/<id>.jsonl`, format version 1 |
| `home.rs` | `FLEET_HOME` / `~/.fleet`, `.env` fallback, harness `refs` reader |
| `doctor.rs` | the four checks as pure evaluators over gathered facts (hermetic in tests) |
| `net.rs` | TCP reachability probe for doctor |
| `bin/mock-server.rs` | the scripted mock API the integration suite drives as a child process |

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

## Testing posture

Zero real network by design: the integration suite spawns `mock-server` as a child process (ureq's reads misbehave when client and loopback server share a process on this machine) and asserts on both the run report and the recorded request bodies. The one live-API test is `#[ignore]`d and self-skips without a key.
