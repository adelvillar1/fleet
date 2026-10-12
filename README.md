# Fleet

Fleet is a native Rust agent for the [DeepSeek API](https://api-docs.deepseek.com): one static binary with its own HTTP client, agent loop, tools, and session persistence. It has no Node runtime, no profile bundles, and no dependency on a DeepSeek Harness checkout — `cargo build` from this directory is the whole install.

Fleet is based on DeepSeek Harness: the agent-loop discipline, the append-only session log, the model-visible-implies-logged rule, and the tool-approval posture all follow the harness's architecture, reimplemented natively in Rust. Both projects are MIT-licensed; see [LICENSE](../LICENSE).

## Status

Alpha (crate version 0.2.0). Dependencies are `ureq` (HTTP), `serde`, and `serde_json`; everything else is standard library.

## Build and test

```sh
cd fleet
cargo build --release   # binary at target/release/fleet
cargo test              # unit + integration tests, loopback only, no key needed
```

The integration suite exercises the real HTTP client against a local mock chat-completions server built from the crate (`src/bin/mock-server.rs`) and run as a child process. One test is a real-API e2e and is `#[ignore]`d:

```sh
DEEPSEEK_API_KEY=sk-... cargo test --test integration -- --ignored
```

## Quick start

```sh
export DEEPSEEK_API_KEY=sk-...
fleet doctor
fleet run "list the Rust files in this directory and say how many there are" --yes
```

## Commands

### `fleet run "<task>" [options]`

Run one task to completion through the tool loop. Options: `--model <name>` (default `deepseek-chat`), `--max-turns <n>` (default `25`), `--yes`/`-y` (approve every tool call without prompting), `--session <id>` (resume a saved session; see `fleet sessions`).

Each tool call prints for approval (except with `--yes`): `y` approves, `n` or Enter denies, `a` approves this and every later call in the run. Denied calls are returned to the model as `denied: …`; tool errors, unknown tools, and malformed arguments are returned as `error: …` so the model can recover.

Exit codes: `0` completed, `1` runtime failure, `2` usage or configuration, `3` the turn cap was reached (resume with `--session <id>`).

### `fleet chat [options]`

The same loop interactively: one session, many tasks, `you>` prompt, `/exit` or `/quit` to leave. API errors print and continue; only a local I/O failure ends the chat.

### `fleet sessions`

List saved sessions: id, model, state (`completed`, `max-turns`, `interrupted`, `unreadable`), last activity, oldest first.

### `fleet doctor`

One line per check — credentials, home, API reachability, shell — exiting nonzero when any check fails.

### `fleet version`, `fleet help`

## Configuration

| Setting | Where Fleet reads it |
|---|---|
| API key | `DEEPSEEK_API_KEY`, else `KEY=VALUE` in `<home>/.env`, else the DeepSeek Harness credential store's `refs` (`$DSH_HOME/.credentials.yaml`, default `~/.dsh/.credentials.yaml`) |
| API base | `DEEPSEEK_BASE_URL` (default `https://api.deepseek.com`) |
| Home | `FLEET_HOME`, else `~/.fleet` |

The `.env` fallback is a convenience, not a config contract: `KEY=VALUE` lines and `#` comments, malformed lines skipped. The Harness credential store is read-only — one stored key serves both harnesses. Credentials never print; `fleet doctor` masks the key to its last four characters.

## Sessions

Every run and chat persists an append-only JSONL log at `<home>/sessions/<id>.jsonl`. Records are `start` (session and model), `user`, `assistant` (content and tool calls), `tool_result` (output, approval, truncation), and `end` (stop reason and turn count). The system prompt and tool declarations are compiled-in constants, reconstructed per request rather than logged.

Resuming with `--session <id>` replays the log and fails loudly, naming the file and line, if a record is unreadable; the next request is seeded from exactly what the log holds, so the model continues with its own prior answers in view. Tool output is capped at 64 KiB per result, cut on a char boundary with a `[truncated]` marker.

## Source layout

```
src/main.rs        dispatch, credential resolution, run/chat/sessions/doctor wiring
src/cli.rs         argument parsing, help text, usage errors
src/api.rs         DeepSeek chat-completions client (ureq), typed errors
src/agent.rs       the tool loop, approvals, turn cap, progress reporting
src/tools.rs       bash, read_file, write_file, list_dir; output capping
src/session.rs     JSONL session log: append, replay, list, resume
src/home.rs        FLEET_HOME / ~/.fleet, sessions dir, .env fallback
src/doctor.rs      the four startup checks
src/net.rs         loopback probe used by doctor
src/bin/mock-server.rs   the scripted mock API the integration tests drive
tests/integration.rs     end-to-end tests: parser, client, tool loop, log
```

## Platform support

macOS (arm64, x86_64), Linux (x86_64, arm64), and Windows (x86_64). The crate uses no platform-specific APIs beyond the per-OS shell probe (`sh -c true` / `cmd /C ver`) and compiles with the usual Rust targets.
