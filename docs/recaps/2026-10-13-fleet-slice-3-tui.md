# Session Recap — slice 3: the TUI, a daily-drivable surface

**Date:** 2026-10-13 **Branch:** `master` **Plan:** `docs/plans/2026-10-13-fleet-everyday-harness.md` — slice 3 of 15 **Repo:** `git@github.com:adelvillar1/fleet.git` (SSH, private)

## Summary

`fleet` with no arguments now launches a ratatui TUI over the async core: a conversation pane that renders streamed tokens as they arrive, tool cards with status dots and result previews, a keyboard approval modal with the headless y/n/a semantics, a status bar (model, session, turn, tokens, run state), a session picker with resume, and the board pane mount point slice 14 fills. Headless `run`/`chat`/`sessions`/`doctor` are byte-for-byte unchanged — the agent gained an optional `UiEvent` channel beside the existing progress writes, and `None` is exactly the old behavior. The agent runs on its own thread and runtime so no network or tool work ever blocks the interface, and tool execution moved to `spawn_blocking` (KI-017's first item).

## Commits

| Hash | Message |
|------|---------|
| `2837354` | feat: the TUI — a ratatui surface as the default fleet invocation (slice 3) |
| `1c25e44` | fix: the TUI's first frame reads as intentional, not broken (visual pass) |
| `c48d3a8` | fix: the input box edits where the cursor is, and a paste types (review fixes) |

## What landed

- `src/tui/` — `app.rs` (pure state machine: modes, actions, scroll, picker, log seeding), `events.rs` (per-mode key mapping), `ui.rs` (rendering), `mod.rs` (crossterm loop, four channels, panic-safe terminal restore).
- `src/agent.rs` — the `UiEvent` seam (Token/Turn/ToolCall/ToolResult/Finished/Failed), `AgentOptions.ui_tx`, async `execute_call` with tools on `spawn_blocking`, `TuiApprover` beside `ConsoleApprover` with identical semantics.
- `src/main.rs` — no-args dispatches to the TUI; every subcommand path untouched.
- Input editing: cursor movement (Left/Right/Home/End/Delete/word-delete) with a visible caret, bracketed paste enabled/disabled cleanly, `Event::Paste` inserts without submitting.

## Verification results

- Gate, parent-run after each commit: `cargo build` and `cargo build --release` exit 0, **zero warnings**; `cargo test` exit 0 — **92 unit + 18 integration passed, 1 ignored** (baseline 53 + 14; nothing deleted). fmt/clippy still unavailable (KI-007).
- **The visual pass** (the owner's directive that a UI's feel is agent-verifiable): the cua-driver is broken on this machine (KI-024 — `rpds.rpds` missing), so the pass ran the real binary in a real tmux PTY, captured each state as ANSI, rendered frames to PNG with a small PIL script, and assessed them with `vision_analyze` plus deterministic probes (tmux's cursor report, the buffer's cell styles, the mock's request log). Driven end to end: launch → streamed tokens (`● working`, turn 1) → approval modal → `y` → `✓ list_dir . done` with preview → final answer → tokens `58/32` exactly as scripted → picker → resume → clean quit; the session log carried the identical vocabulary (`tool_result` with `approved: true`, then `end`).
- Visual findings and their fate: the fresh-state void (fixed — the pane now says what fleet is), dim titles/footer (fixed — readable), and a "half-styled border" that a TestBackend buffer probe proved was a capture artifact, not a defect — the vision claim dissolved against the measurement, and nothing was changed to satisfy it.
- Post-fix interaction checks, all live: caret at the exact predicted cell after Home+6×Right (x=7, y=31), Delete at the cursor, a two-line bracketed paste inserting without submitting with `\r` normalized, the resume-while-working banner (`! wait for the run to finish…`), and the picker's IO error surfacing (`! could not list the sessions: Not a directory`).

## Review cycle

- **Stage 1 (spec, step-5-preview): SPEC-COMPLIANT** — all 12 checks, zero test deletions, scope exactly the expected files, both commits.
- **Stage 2 (quality, step-5-preview): REQUEST_CHANGES** — no criticals; two Important findings on the UX-co-equal directive (no in-input cursor movement; bracketed paste off) plus five cheap minors and a width limitation to log. All seven fixed in `c48d3a8` (+545/−33, 12 new tests) and each re-verified live or by test above.
- Both reviews ran on step-5-preview after the KI-015 mis-route correction; each child's model verified via `delegate_task action='list'`.

## Known issues touched

- **KI-021 (closed)**: `blocking_recv` panics inside a runtime — the first approval killed the agent thread; fixed with a `std::sync::mpsc` responder.
- **KI-024 (open, environmental)**: the cua-driver is broken here; the tmux+vision workaround ran this slice's visual pass and is recorded as the fallback.
- **KI-025/026/027 (open, logged)**: char-vs-cell width in the TUI's wrapping; tmux's paste line-break as `\r`; a pre-existing integration flake in `wait_for_port` (reproduced on the parent commit — 2 of 6 runs — so no current change causes it).
- **KI-022/023 (open)**: launch creates an empty session log; `#[tokio::main]`'s idle workers while the TUI runs its own runtime.

## Open questions / next steps

- **The owner's hands**: pane proportions, the `q`-only-when-empty tradeoff, and the streaming redraw cadence (channels drain once per 100 ms tick — KI-023's neighborhood) are feel judgments verified structurally but not judged.
- **Slice 4 — the MCP stdio client** plus the vendored lane cores (sys1/sdm1/sem1/gen1 from the fork's `services/`): spawn, initialize, tools/list, tools/call, teardown; `fleet lanes` lists health. This is where the ecosystem starts landing in Rust.
- KI-017's remaining items (doctor's probe, chat's stdin) stay deferred to the slices that need them.
