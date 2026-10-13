# Fleet — a fully native Rust agent app

Status: active — slice 1 complete; the live-API task completion (C3) is
blocked by account balance, not by code. The crate moved to its own
repository on 2026-10-13; see "Fleet left this repository" below. Plan-gate:
this document.

---
created: 2026-10-12
updated: 2026-10-13
slug: fleet-native-app
---

## Goal

Fleet is a native Rust application, not a wrapper around the Node/TS harness:
no embedded Node runtime, no staged harness payload, no checkout dependency.
The owner's direction (2026-10-12): "fully written in rust and not a wrapper
to the existing code." Fleet keeps its attribution — it is based on DeepSeek
Harness and follows its architecture shapes (session logs, tool calling,
model-visible ⟺ logged) — while implementing the agent experience directly
against the DeepSeek API in Rust.

The earlier direction (one binary bundling a Node runtime plus a staged
`pnpm deploy` payload) is abandoned. Its one durable by-product landed
separately as a harness fix: `apps/cli` now carries the
`@deepseek-ai/cordis-plugin-group` peer so `pnpm deploy` trees boot.

## Design

### Slice 1 — native core (this slice)

- `api.rs` — DeepSeek chat-completions client over HTTPS (`ureq` + rustls;
  the std-only no-dependency constraint from the launcher experiment is
  dropped, because TLS cannot be hand-rolled responsibly). Typed
  request/response, function-calling tools, error mapping (401 credentials,
  429 rate limit, 5xx, network).
- `session.rs` — append-only JSONL session log at `~/.fleet/sessions/`,
  format version 1 (`start`, `user`, `assistant`, `tool_result`, `end`
  records). Replay rebuilds the message list for resume. Model-visible ⟺
  logged: every request message and response lands in the log.
- `tools.rs` — closed tool set: `bash` (with timeout + combined output
  capture), `read_file`, `write_file`, `list_dir`. Output truncated at a
  fixed budget with an explicit marker.
- `agent.rs` — the loop: user task → request → tool calls → approval gate →
  execution → tool results → repeat until the model answers without tool
  calls or `--max-turns` is reached. Tool approval prompts on the terminal
  unless `--yes`; a denied tool records a denial result the model can see.
- `cli.rs` / `main.rs` — grammar: `fleet run "<task>"`, `fleet chat`,
  `fleet sessions`, `fleet doctor`, `fleet version`, `fleet help`. The
  launcher-era `status` command and the node/harness discovery checks go
  away with the wrapper.
- `doctor.rs` — reworked checks: `DEEPSEEK_API_KEY` present, `~/.fleet`
  writable, API host reachable, shell available.
- Credential resolution (owner direction, 2026-10-12): environment, then
  `~/.fleet/.env`, then the DeepSeek Harness credential store's `refs`
  block (`$DSH_HOME/.credentials.yaml`, default `~/.dsh/.credentials.yaml`)
  read-only — one stored key serves both harnesses, with no checkout or
  Node dependency.

### Later slices (out of scope here)

Streaming responses; more tools (web fetch/search, subagents); compaction
when a session outgrows the window; a native UI surface; cross-platform
release builds per OS.

## Verification

Per slice: `cargo test` (unit + integration, including a scripted two-turn
tool loop against a local mock server), `cargo clippy` clean, and a live
one-turn smoke against the real API when `DEEPSEEK_API_KEY` is set
(self-skipping `#[ignore]` test, mirroring the repo's test:e2e policy).

## Acceptance criteria (plan-gate)

- C1: `cargo test` green, including the scripted tool loop, session
  roundtrip, and API error mapping.
- C2: `fleet doctor` reports all checks with real status on this machine.
- C3: `fleet run "…"` completes a real task end to end against the live API
  with a tool executing (ignored test; requires the key).
- C4: `fleet chat` holds a multi-turn conversation and `--session` resumes
  one from disk.
- C5: no Node invocation, no checkout reference, and no dsh launch path
  remains anywhere in the Fleet crate.

### Fleet left this repository (2026-10-13)

The owner's intent for Fleet was always decoupling from the DeepSeek Harness fork, not merely runtime self-containment, so the crate now lives in its own repository: [adelvillar1/fleet](https://github.com/adelvillar1/fleet), private, MIT-licensed, with its own CI running fmt, clippy, build and test. The crate was extracted with its two commits intact and removed from this fork's tree. The local checkout is `~/Projects/fleet`. C5's wording changed with the move because "under `fleet/`" no longer names a path in this repository; the substance is unchanged, and the standalone repository satisfies it by construction. This plan stays here as the design record for slice 1; Fleet's own plan, slices 2 and onward, and its later work belong to that repository.

### Slice 1 result (2026-10-12)

C1 met: 44 unit + 11 integration tests green — the two-turn tool loop,
resume seeding from the log, denials, unknown tools, malformed arguments,
oversized-output truncation, filesystem tools, and 401 error mapping,
driven through the real HTTP client against a scripted mock API run as a
child process. C2 met: `fleet doctor` reports every check ok on this
machine, the key resolved from the Harness credential store. C4's resume
half met by the integration test; its live-chat half waits on C3. C3
blocked externally: the stored key authenticates and reaches
api.deepseek.com, which answers 402 Insufficient Balance — the typed 402
mapping surfaced it correctly, and the task completes once the account has
balance. C5 met: no Node invocation, checkout reference, or dsh launch
path remains in the crate. `cargo clippy` could not run: this machine
has a bare stable toolchain without the rustup shim needed to add the
component; a clean full rebuild emits zero warnings instead.
