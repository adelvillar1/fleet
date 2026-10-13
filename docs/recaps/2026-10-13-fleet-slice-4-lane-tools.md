---
status: done
created: 2026-10-13
slug: fleet-slice-4-lane-tools
---

# Recap: slice 4's completion — the lanes' tools callable inside a run

Plan: [2026-10-13-fleet-slice-4-lane-tools.md](../plans/2026-10-13-fleet-slice-4-lane-tools.md). Companion to [the slice-4 recap](2026-10-13-fleet-slice-4-mcp-lanes.md): slice 4's client, vendored cores, and `fleet lanes` landed first; this round closed the plan's remaining slice-4 acceptance criteria — "their tools callable inside a run".

## What landed

- **The namespace** (`src/tools.rs`): a lane's tools enter the model-visible set as `mcp__<svc>__<short>` — every lane names its tools after itself (`sem1_probe`), so a literal `mcp__<svc>__<tool>` would stutter (`mcp__sem1__sem1_probe`); the redundant prefix drops, both ends are recorded (`name` + `lane_tool`), unprefixed names are kept, and a colliding short name iterates to a unique declared name. The four built-ins keep their names and are always declared.
- **Lazy, run-owned lifecycle** (`src/agent.rs`): no lane process starts unless its tool is called; on first call the lane spawns through the env-scrubbed path (KI-031 not regressed — one spawn site), initializes, tools/list runs, and its tools join the declarations for the rest of the run. Every lane a run spawned closes when the run ends — `close_all` is awaited unconditionally after the run task, so completed / max-turns / API-error / log-write-error all funnel through it, and a panic unwinds into Drop's best-effort kill.
- **Dead child degrades**: a dead or never-started lane returns a typed failure in the built-ins' `error:` vocabulary; the run continues; the lane is marked Dead and fails fast with no respawn loop. It keeps its tools declared — the model saw them and the `lane_tools` record must keep matching later requests (KI-042).
- **Approvals unchanged**: the same `Approver`, invoked before the lane branch, so a denied lane call spawns nothing; the summary names the lane's role, its own tool name, and compacted arguments, with the namespaced name rendered beside it in console and TUI.
- **Reconstructability**: one new log record type, `lane_tools`, written exactly once per join, carrying each tool's name/lane_tool/description/input_schema — the request the model saw stays reconstructable (hard rule 5's intent). `messages()` filters it; the TUI's seeding renders it as a lane card, so a resumed log shows its lanes.
- **Result mapping**: MCP content arrays map onto the built-ins' tool_result shape — text joined, non-text noted as `[type]`, `isError` prefixed `error:` — bounded by `MAX_TOOL_OUTPUT` with the real `truncated` flag.
- **UX**: `→ lane sem1 joined — 4 tools` in headless progress; `UiEvent::LaneJoined` renders a lane card in the TUI; tool cards carry the namespaced name.

## Verification

- Gate, parent-run after each commit: `cargo build --all-targets` exit 0, **zero warnings**; `cargo test` exit 0 — **150 unit + 43 integration passed, 1 ignored** (completion-round baseline 126 + 34; none deleted). fmt/clippy unavailable (KI-007).
- The two-turn integration test drives the **real vendored sem1** through the mock API: turn 1 declares exactly the four built-ins (lazy), turn 2 declares eight (the four `mcp__sem1__*` names in order), the probe answer rides back in the tool message, and the log carries `lane_tools` before `tool_result`. `fleet lanes` still 4 ok, exit 0.
- Negative probes (the fixes are non-vacuous): with the bound removed, the test's record carries 262,826 bytes with `truncated: false` — the defect verbatim; without the stderr drain, the chatty-child test times out.
- Both reviews ran on step-5-preview (verified via `action='list'`). Spec: **SPEC-COMPLIANT**, all 12. Quality: **APPROVED**, no criticals — one Important taken in the fix round, two minors fixed, one pre-existing hazard decided and logged.

## Review cycle and fixes

- The implementer's first gate run failed one test — diagnosed as its own wrong expectation (it asserted a `bash` fixture where the seeded record was a lane result), plus a second failure from a test not taking the env lock its neighbours use. Both fixed at the cause; five consecutive green runs after. The parent required the failing test named before any re-run, per the KI-027/KI-038 flake discipline.
- Fix round `28fca49`: lane output bounded through the built-ins' cap with the real `truncated` flag (KI-041 closed); the collision fallback iterates to a unique name; the MCP child's stderr is drained into a bounded 64 KiB tail attached to the startup handshake (KI-043 — drain, not silence, because a missing venv's diagnostic lands on stderr and nowhere else).

## Known issues

KI-041 closed-fixed; KI-043 added (the stderr hazard, decision, evidence); KI-042 (dead lane keeps tools declared) and KI-030 (health probes the MCP surface only) remain open by design; KI-028's paths became reachable with this wiring and its row was updated honestly.

## Deferred

- The agent's MCP wiring stops at tool calls: **slice 5** owns provider routing (the owner's declared chain: router kit first, then Z.ai, MiMo, StepFun, Qwen Token plan, DeepSeek last) with `sys1_classify` judging per-task routes.
- sys1/sdm1/gen1 are driven at initialize/tools/list level; only sem1 has an end-to-end call test.
- A live-API run (the DeepSeek account has balance but is the last-resort route — the roster doesn't exist yet, so no routing test is possible until slice 5).
