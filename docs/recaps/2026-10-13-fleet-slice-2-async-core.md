# Session Recap — slice 2: the async core, tokio plus streamed completions

**Date:** 2026-10-13 **Branch:** `master` **Plan:** `docs/plans/2026-10-13-fleet-everyday-harness.md` — slice 2 of 15 **Repo:** `git@github.com:adelvillar1/fleet.git` (SSH, private)

## Summary

Fleet's agent core is now asynchronous and streams. The DeepSeek client gained `chat_stream`: SSE parsed line by line over ureq's reader, with the blocking socket work on a spawned blocking task and events crossing to the async side over an unbounded channel — no futures `Stream` trait, and ureq stays the HTTP stack. The agent loop consumes tokens as they arrive and writes them raw to the progress writer, so a headless run shows tokens live and slice 3's TUI receives the same events. The session log's vocabulary is unchanged, and all pre-existing tests were adapted, none deleted.

## Commits

| Hash | Message |
|------|---------|
| `2851b82` | feat: the async core — tokio plus streamed chat-completions (slice 2) |
| `77dba81` | fix: the assistant turn keeps its tool calls in the log and the live conversation (KI-016) |
| `59a485f` | fix: a stream closing without finish_reason still delivers its tool calls (KI-020) |

## What was built

- `Cargo.toml`: tokio, exactly the five features the plan approved (`rt-multi-thread, macros, sync, time, io-util`) — the only new dependency.
- `src/api.rs`: `chat_stream(&self, request, on_event) -> ChatResponse` driving the whole exchange and returning the same assembled response `chat()` returns, so the loop keeps one downstream path. `StreamEvent::{Content, ToolCalls, Usage}`; OpenAI-style tool-call deltas assembled by index and emitted complete; the streamed request carries `stream: true` + `stream_options.include_usage` so token accounting stays exact. A dropped receiver stops the reader; a truncated body is a decode failure, not a silent empty answer.
- `src/agent.rs`: `run` is async; deltas go to the progress writer as raw text; the existing progress lines and the final-answer-to-stdout behavior are untouched.
- `src/main.rs`: `#[tokio::main]`; `run_command`/`chat_command`/`drive` async. `cli/session/home/tools/doctor/net` stay synchronous, as designed.
- `src/bin/mock-server.rs`: a scripted `sse` frame mode (close-delimited, no Content-Length, optional `delay_ms`) beside the intact document mode.
- `tests/integration.rs`: `#[tokio::test]`; the loop fixtures streamed; one new deterministic progressive-delivery test (write-order snapshots, no wall-clock); the non-streaming `chat()` path pinned by its own test.

## What was fixed along the way

- **KI-016 (pre-existing since slice 1, found by the implementer, fixed this session):** the loop took the tool calls out of the assistant message *before* logging it, so the record serialized without `tool_calls` — a replayed log could not reconstruct the turn (hard rule 5, model-visible ⟺ logged), a resumed run sent an invalid assistant→tool sequence, and the same take also stripped the calls from the live conversation, so the next request *in the same run* carried the invalid sequence too. Fixed by logging and keeping the message intact and cloning the calls for execution; the regression test asserts both the record and the next request carry the call. Old logs stay readable (`tool_calls` is serde-default); pre-fix logs replay the old invalid sequence on resume — recorded as a residual in the ledger.
- **KI-020 (found by the stage-2 quality review, fixed this session):** assembled tool calls were emitted only at `finish_reason`, so a stream closing with `[DONE]` alone would silently drop the model's calls. Now emitted at close as well; a gapped tool-call index is a decode failure rather than a silent mis-assembly. Both covered by unit tests.

## Verification results

- Gate, parent-run after each commit: `cargo build` exit 0, **zero warnings**, `cargo test` exit 0 — **53 unit + 14 integration passed, 1 ignored** (baseline was 56 passed: 45 unit + 11 integration; +8 unit SSE/stream tests, +3 integration, none deleted — the spec reviewer verified test-name set equality against the baseline).
- Real-binary smoke against the mock server: two-turn streamed tool loop, exit 0, log vocabulary identical, both requests carrying `stream:true` + `include_usage`, tokens live on stderr, final answer on stdout.
- Two-stage review, both on step-5-preview (per the delegation directive): spec compliance — **SPEC-COMPLIANT**, all 12 checks pass, scope exactly the 8 expected files; code quality — **APPROVED**, one Important finding (KI-020) fixed before push, minors triaged (dead accumulator removed, `Usage` doc corrected, KI-016 residual recorded).
- fmt/clippy did **not** run (KI-007 — absent components); the crate is written in the existing rustfmt-clean style, and `cargo build` is warning-free.

## Process events recorded in the ledger

- KI-015: slice-2 attempt 1 ran on hy4-preview because the Hermes delegation slot was stale-pinned (2026-09-03, when hy4 was the parent model); the parent-model directive (2026-10-13) overrode it. Attempt discarded uncommitted; delegation re-pinned to step-5-preview; the cost-guard allowlist corrected; slice 2 re-dispatched on the right model. The rule — verify each child's model via `delegate_task action='list'` — held for both reviews.
- KI-014: a red test reached origin earlier in the session via a status-losing pipe; the gate is now run with exit codes read, never piped.

## Open questions / next steps

- **Slice 3 — the TUI** (ratatui), the plan's next slice: `fleet` with no args launches; chat/run panes with streamed tokens; approvals from the keyboard; session browse. The UX-co-equal directive means every later slice lands its surface here.
- **KI-017** (open, blocks slice 3's responsiveness): tool execution, doctor's TCP probe, and chat's stdin read are blocking calls on tokio workers — wrap tool execution in `spawn_blocking` when the TUI lands.
- **KI-018** (open, test-only): the mock server dies on a request-less connection; `continue` instead of `break` when next touched.
- **KI-019** (open): the assembled `finish_reason` is `None`; derive it when a consumer needs it.
- Environmental, unchanged: Actions budget out (gates stay local-only), rustfmt/clippy absent, DeepSeek balance 0 (the live e2e stays `#[ignore]`d).
