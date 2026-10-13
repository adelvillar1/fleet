---
status: done
created: 2026-10-13
updated: 2026-10-13
slug: fleet-slice-4-lane-tools
---

# Plan: Slice 4 remainder — the lanes' tools callable inside a run

## Context

Slice 4 (`f365d6c`, review fixes `7bb8140`) landed the MCP stdio client (`src/mcp.rs`), the four vendored lane cores (`services/`), `fleet lanes`, and the TUI's Ctrl+L view — and deliberately left the agent loop untouched: "the agent's tool loop does not gain MCP tools this slice — slice 5's roster routing owns that". The slice-4 plan's own acceptance line nevertheless names the wiring ("register their tools in the agent loop under `mcp__<svc>__*` namespaces"), and the crate cannot route (slice 5) or calibrate (slice 6) against lanes it cannot call. This plan closes that remainder: the lanes' tools become callable inside a run, under the namespace, with the run owning its lane children.

## Acceptance criteria

- **AC1 (namespace).** A lane's tools enter the model-visible tool set as `mcp__<svc>__<tool>`, carrying the lane's own description and inputSchema verbatim. The four built-ins keep their names and are always declared, first.
- **AC2 (lazy per run).** No lane process starts unless one of its tools is called. The first call spawns the lane, initializes it, runs `tools/list`, and its tools join the declarations for the rest of the run. Every lane a run spawns closes when the run ends, on every exit path.
- **AC3 (dead child degrades).** A call whose child has died — or that fails to spawn, initialize, or list — returns a typed failure result to the model: the `tool_result` is recorded, the run continues, and the lane is dead for the rest of the run (fail fast, no respawn loop). A lane that never starts never aborts the run.
- **AC4 (approvals).** MCP tool calls go through the same `Approver` as built-ins, with a summary naming the lane and the call. A denied lane call spawns nothing.
- **AC5 (model-visible ⟺ logged).** When a lane joins a run, one `lane_tools` session-log record carries the lane and each tool's name/description/inputSchema, so the request the model saw is reconstructable from the log. Replay skips the record rather than failing on it.
- **AC6 (result shape).** An MCP `tools/call` content array maps onto the built-ins' `tool_result` shape: text blocks joined, non-text blocks noted by type, `isError` recorded as a failure with its text.
- **AC7 (UX).** A lane joining a run is visible on both surfaces: a headless progress line, and a `UiEvent::LaneJoined` rendered in the TUI conversation. Tool cards show the namespaced name.
- **AC8 (no env regression).** Lane children keep slice 4's scrubbed environment (KI-031): `env_clear` plus the base, the spec's own entries, and the lane's allowlist.
- **AC9 (gate).** `cargo build --all-targets` warning-free, `cargo test` green, no pre-existing test deleted, commit with explicit paths, no push.

## Design decisions (binding)

- **The namespace drops the lane's own prefix.** Every vendored lane names its tools after itself (`sem1_probe`, `sys1_classify`, `sdm1_health`, `gen1_imagine`), so a literal `mcp__<svc>__<tool>` namespace would read `mcp__sem1__sem1_probe`. The namespace drops that redundant prefix, so the model calls `mcp__sem1__probe`; a tool whose name does not start with the lane's keeps its own name. Two tools whose short names meet fall back to the lane's full name, which cannot collide. Both ends of the mapping are recorded in the log (`name` and `lane_tool`).
- **The run owns its lane children, lazily.** `LaneRoster` (in `agent.rs`) holds one entry per lane the run has met: the child and its roster while live, the reason once dead. A lane spawns on its first call and closes in `Agent::run`'s tail, on every exit path. Nothing is respawned: a dead lane's reason is recorded once and reused, so a broken lane costs one spawn attempt rather than one per call.
- **A dead lane keeps its declarations.** The model saw the lane's tools, and the log records them; withdrawing them mid-run would make `lane_tools` stop matching the requests that followed. Calls to a dead lane fail fast with a typed failure instead.
- **The same gate, the same record, the same message.** A lane call is approved by the same `Approver`, recorded as the same `Record::ToolResult`, and pushed as the same `Role::Tool` message. Only the execution site differs.
- **One vocabulary extension, tolerated by readers.** `Record::LaneTools` is bookkeeping like `start`/`end`: it replays to no message. Same format version — old readers skip what they do not know, and the reader here skips it.
- **Where a lane's spec comes from is injectable.** `AgentOptions::lane_specs` defaults to `None` = the vendored registry (`lanes::spec_for`, resolved lazily with its named reasons). Tests inject a scripted fake's spec so the loop's lane paths run without a real lane; the real-lane path is exercised unchanged.

## Work order

1. Plan (this file).
2. `src/tools.rs`: the namespace vocabulary (`lane_tool_name`, `lane_of`, `LaneTool`, `LaneTools`), the pure declaration assembly, and the roster namespacing.
3. `src/mcp.rs`: `CallResult::text()` — the content-array mapping.
4. `src/lanes.rs`: `spec_for(name)` — a lane resolved by name with its reason.
5. `src/agent.rs`: `LaneRoster` (lazy spawn, fail-fast, run-scoped teardown), the dynamic declarations, `UiEvent::LaneJoined`, the lane branch in `execute_call`, lane summaries.
6. `src/session.rs`: the `lane_tools` record and the replay skip.
7. `src/tui/`: the `Entry::Lane` block, its event arm, its seeding arm, and its render.
8. `src/bin/mock-mcp.rs`: an `on_eof` marker so a test can prove the run closed the child it spawned.
9. Tests: unit (assembly, mapping, fail-fast, summaries, record round-trip) and integration (the real sem1 two-turn run, dead child mid-run, UX + teardown, refusal, denial, unlaunchable lane).
10. Gate: `cargo build --all-targets` (warning-free), `cargo test` — exit codes read directly, never through a pipe. Five consecutive runs.
11. Ledger and architecture doc; commit with explicit paths; no push.

## Verification evidence

- `cargo build --all-targets` — exit 0, zero warnings.
- `cargo test` — 144 unit + 40 integration (1 ignored) passed, exit 0; five consecutive runs identical.
- The real-sem1 two-turn run: turn 1 declares only the four built-ins, turn 2 declares eight, the log carries `lane_tools` + the `tool_result`, and the probe answer reaches the model.
- `./target/debug/fleet lanes` unchanged (4 ok) — the health path was not touched.

## Risks

- **A lane's tools stay declared after its child dies** (deliberate, AC3): a model that keeps calling a dead lane burns turns on typed failures. Logged as a risk rather than silently accepted.
- **MCP results are not bounded by `MAX_TOOL_OUTPUT`**: a lane returning a large payload rides into the log and the model's context uncapped, where built-in output is capped. Logged as an improvement.
- **sys1/gen1's nested JSON-RPC error shape (KI-028) is now reachable** through the loop: a refused sys1/gen1 call reads as a content-less success, which the mapping surfaces as "(the tool returned no content)" rather than the server's message.
