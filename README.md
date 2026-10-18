# Fleet

Fleet is a native Rust agent for the [DeepSeek API](https://api-docs.deepseek.com): one static binary with its own HTTP client, agent loop, tools, and session persistence. It has no Node runtime, no profile bundles, and no dependency on a DeepSeek Harness checkout — `cargo build` from this directory is the whole install.

Fleet is based on DeepSeek Harness: the agent-loop discipline, the append-only session log, the model-visible-implies-logged rule, and the tool-approval posture all follow the harness's architecture, reimplemented natively in Rust. Fleet is MIT-licensed; see [LICENSE](LICENSE). DeepSeek Harness is separately MIT-licensed by its authors.

## Status

Alpha (crate version 0.2.0). Dependencies are `tokio`, `ureq` (HTTP), `serde`/`serde_json`, and `ratatui`/`crossterm` (the TUI); everything else is standard library.

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
cd fleet
cargo build --release                     # one static binary at target/release/fleet
export DEEPSEEK_API_KEY=sk-...            # or add KEY=VALUE to ~/.fleet/.env
target/release/fleet install              # creates the home, writes the config scaffold, runs the doctor gate
target/release/fleet doctor               # the gate on its own: credentials, home, API, shell, roster, ledger, lanes
target/release/fleet chat                 # the everyday surface — one session, many tasks, /exit to leave
```

`fleet install` is idempotent. It creates the home (`FLEET_HOME`, else `~/.fleet`) with `sessions/`, `memories/`, `skills/`, `cron/`, and `proposals/`, writes the minimal `config.json` scaffold (`{"pin": null, "providers": []}` — the roster's override seam over the compiled-in defaults) only when absent, and ends on `fleet doctor`'s checks: the install's exit code is the doctor's, so a red gate fails the install. A second run changes nothing and names what exists; an existing `config.json` is named and left untouched.

The five vendored lanes are Python services, and their heavy dependencies are an optional second step:

```sh
target/release/fleet install --lanes      # per-lane venvs under <home>/venvs/<lane>/
```

The specialist lanes are **sys1** (judgment), **sdm1** (tabular), **sem1** (embeddings), **gen1** (media), and **blend1** (the grounded lane). blend1 is the blended lane of the sys1 house pattern: one `ask()` runs the d1 router, the semantic and tabular lanes in parallel, and synthesizes with Liquid's LFM2.5-24B-A2B only when the router clears its floor — a cheap model whose reasoning is grounded on TabPFN's computed facts rather than invented, because LLMs do not do math and TabPFN does. See [docs/features/blend1.md](docs/features/blend1.md).

Each lane is idempotent and degrades without: a venv that cannot be built — no network, a failing pip, no `uv`/`python3` — is skipped with a named line and the install still succeeds; the lane stays declared and simply dead when its tools are called. No step of the quick start needs Node or a DeepSeek Harness checkout — the lanes are vendored in this repository under `services/`.

## Commands

### `fleet install [--lanes]`

Create the home layout (`sessions/`, `memories/`, `skills/`, `cron/`, `proposals/`) and the config scaffold, then run the doctor gate — the install's exit code is the doctor's. Idempotent: re-running names what exists and never rewrites an existing `config.json`. `--lanes` also bootstraps the heavy-lane venvs under `<home>/venvs/<lane>/` (sys1's GLiNER, sdm1's models, sem1's sentence-transformers, gen1's kokoro, blend1's union of the three siblings' sets), each lane skipped by name when it cannot be built. See Quick start above.

### `fleet run "<task>" [options]`

Run one task to completion through the tool loop. Options: `--model <name>` (default `deepseek-chat`), `--max-turns <n>` (default `25`), `--yes`/`-y` (approve every tool call without prompting), `--session <id>` (resume a saved session; see `fleet sessions`).

Each tool call prints for approval (except with `--yes`): `y` approves, `n` or Enter denies, `a` approves this and every later call in the run. Denied calls are returned to the model as `denied: …`; tool errors, unknown tools, and malformed arguments are returned as `error: …` so the model can recover.

Exit codes: `0` completed, `1` runtime failure, `2` usage or configuration, `3` the turn cap was reached (resume with `--session <id>`).

### `fleet chat [options]`

The same loop interactively: one session, many tasks, `you>` prompt, `/exit` or `/quit` to leave. API errors print and continue; only a local I/O failure ends the chat.

### `fleet gui [--port N] [--no-open] [--host H]`

Serve the app's browser face on loopback (default `127.0.0.1:3080`): the conversation first, the intelligence surfaces and the tower console beneath it, and the trust trio — per-turn checkpoints with rewind, plan mode, and the diff review — riding the conversation. The URL prints with a per-launch token; a non-loopback host is refused by name. See [`docs/features/gui.md`](docs/features/gui.md).

### `fleet sessions`

List saved sessions: id, model, state (`completed`, `max-turns`, `interrupted`, `unreadable`), last activity, oldest first.

### `fleet doctor`

One line per check — credentials, home, API reachability, shell, the routing roster, the calibration ledger, and one named line per MCP core — exiting nonzero when any check fails. `--diagnose <session>` classifies one session's failure and drafts an evidence-cited recap; `--open` reads the ledger's open action items back.

### `fleet version`, `fleet help`

## Configuration

| Setting | Where Fleet reads it |
|---|---|
| API key | `DEEPSEEK_API_KEY`, else `KEY=VALUE` in `<home>/.env` |
| API base | `DEEPSEEK_BASE_URL` (default `https://api.deepseek.com`) |
| Home | `FLEET_HOME`, else `~/.fleet` |

The `.env` fallback is a convenience, not a config contract: `KEY=VALUE` lines and `#` comments, malformed lines skipped. Fleet reads its own home and nothing else — no other harness's config file is ever read (the owner's 2026-10-16 ruling). Credentials never print; `fleet doctor` masks the key to its last four characters.

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
src/route.rs       multi-provider routing: roster, sys1-judged selection, failover
src/calib.rs       the calibration stream: devdec-vocabulary rows, bounded writer
src/doctor.rs      the boot checks and the per-layer diagnosis
src/net.rs         loopback probe used by doctor
src/install.rs     fleet install: home layout, config scaffold, heavy-lane venvs
src/lanes.rs       the vendored lane cores: home resolution, launch specs, health probe
src/lane_client.rs the lane-kind candidate's boundary: a chat request in, a grounded answer out
src/mcp.rs         the hand-rolled MCP stdio client (spawn, initialize, tools/list, tools/call)
src/route_cache.rs the route-decision cache: one recorded judgment per task
src/settings.rs    one writer over config.json / routing.json / fleet.env
src/boards.rs      the read-only boards: routing state, second-brain derived state, git facts
src/memory.rs      the git-tracked fact substrate: six-channel recall, assembly
src/session_recall.rs the session archive as synthetic facts through the same ranker
src/skills.rs      skills: SKILL.md loading, progressive disclosure, the self-authoring drafter
src/subagents.rs   delegation: the child agent, its scope, the depth cap
src/roles.rs       the named specializations a delegation runs under
src/delegate.rs    the delegation judgment beat
src/graph.rs       the dependency graph: scores, gates, critical path
src/cron.rs        scheduled runs: the job store, the five-field parser, catch-up
src/judge.rs       the routing judgment beat
src/eval.rs        the eval corpus: fixed scenarios, graded headless against the mock
src/improve.rs     self-maintenance: gated proposals, apply/undo
src/explain.rs     the explanation beat over blend1
src/checkpoints.rs the trust trio's floor: per-turn file-state capture, restore, and diff
src/gui/           the browser face: server, bridge, themes, embedded assets
src/tui/           the ratatui surface: app state, events, render, loop
src/bin/mock-server.rs   the scripted mock API the integration tests drive
src/bin/mock-mcp.rs      the scripted fake MCP stdio server the integration tests drive
tests/integration.rs     end-to-end tests: parser, client, tool loop, log
tests/install.rs         the install's acceptance: layout, lanes, the doctor gate
```

## Platform support

macOS (arm64, x86_64) and Linux (x86_64, arm64) are built and run in this repository's own workflow. Windows (x86_64) is a target the crate's code anticipates — the per-OS shell probe (`sh -c true` / `cmd /C ver`) is written for it and the crate uses no other platform-specific APIs — but no Windows build has run here, so the claim is scoped to what is verified until a cross-compile check (`cargo check --target x86_64-pc-windows-msvc`) lands (KI-011).

The release path is `cargo build --release` — one binary at `target/release/fleet`, which is what the quick start above runs. Per-OS cross-built release artifacts are a release-process step, not built in this repository (CI is disarmed by owner decision): the cross-build commands and the install story live in [docs/features/install.md](docs/features/install.md).
