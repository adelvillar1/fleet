---
status: draft
created: 2026-10-13
updated: 2026-10-13
slug: fleet-everyday-harness
---

# Plan: Fleet, the everyday native harness

## Context

The owner wants a fully working, installable, launchable native application to use as the every-day harness — the agent loop, sessions, skills, memory, subagents, scheduling, routing, and the boards — in one self-contained unit. Architecture is the owner's non-concern; the outcome is the concern. Time and token budgets are explicitly not constraints. The owner is open to pivoting to a different base harness if that is the better path.

### The base decision: no pivot (and why)

The question was "full pivot and rewrite onto a different base, or finish this one." The answer is **finish this one** — build Fleet out on the base already chosen — for three reasons:

1. **The ecosystem is base-agnostic, so a pivot buys nothing where it matters.** The lanes integrate over MCP (a standard protocol) and the calibrations/boards integrate over file contracts and read-only HTTP. None of that work changes with the base. What a pivot changes is only the *shell* — and the shell is exactly what slice 1 already started natively.
2. **On this base, every remaining piece has a working, tested reference implementation in a codebase the owner owns.** Streaming, skills, subagents, compaction, scheduling, routing, calibration, and the boards all exist in the DeepSeek Harness fork's fleet profile (`packages/fleet/dsh-*`, `services/`, `docs/fleet/architecture.md`). Each slice below is a port with a spec, not an invention. A third-party base throws the spec away and makes every integration a fresh design.
3. **A pivot rents an architecture and its mismatches become permanent constraints.** The credible native-ish candidates (codex-cli: Rust but OpenAI-loop-shaped; opencode: excellent but TypeScript) each mismatch this stack — DeepSeek/z.ai roster routing, sys1 judgment under the loop, the dev-decisions calibration vocabulary — in places that would be fought forever. The owner explicitly does not want legacy constraints from earlier decisions; inheriting a foreign loop's seams is the largest such constraint available.

The fork is therefore the **spec and integration lab**, not a shipping competitor: it keeps the canonical dsh-* implementations and the vendored cores; Fleet becomes the shipping native core; the two never both ship the same feature.

### What "everyday harness" means here (the target surface)

Verified against the fork's package surface and the owner's working style: streaming chat and headless runs; sessions with resume; a closed-but-extensible tool set plus MCP tools; multi-provider routing with judgment and failover; skills; durable memory with recall; subagents with isolated contexts; compaction; scheduled runs; the router/second-brain/tower boards; approvals; one-binary install with `doctor` as the gate.

### Current state (verified 2026-10-13)

Slice 1 is in-repo and green: 55 tests (44 unit + 11 integration), binary smoke-verified, methodology layer landed (`a7ad239`, `aea7c69`, `107610e`). The client is non-streaming; there is no MCP client, no routing, no skills/memory/subagents/compaction/cron, no TUI, no installer. The four lane cores are vendored in the fork at `services/<svc>/` (pins at upstream HEADs) and boot-verified this session. The router decision core, calibration stream, and boards exist as fork packages with the architecture doc as their record.

## Approach

A Rust core that **composes** rather than reimplements: the agent loop, client, sessions, and approvals are native (slice 1, done); the lanes are Python services spawned as MCP stdio children (they can never be Rust — the ML ecosystem is Python); routing, calibration, skills, memory, subagents, compaction, and cron are native ports of the fork's implementations, each slice naming its spec; the boards are native read-only surfaces over the same sources the fork's boards read.

Standing rules for every slice: model-visible ⟺ logged; no secrets in the repo; the local gate (`cargo fmt --check`, `clippy -D warnings`, `build --release`, `test`) runs before every push because CI is budget-blocked; unit + integration tests at the current bar, with MCP slices tested against the real vendored cores on loopback; no duplication of anything the fork already ships as canonical.

**Foundation decision — async.** Slice 2 adopts tokio. Streaming reads, concurrent MCP children, subagents, and scheduled runs are concurrent by nature; std threads would be fought for the rest of the app's life. This is the one place a small upfront rewrite prevents permanent debt.

## The slices

Order is dependency-driven: each slice unblocks the next and nothing is built ahead of its consumer.

### Slice 2 — the async core: tokio + streaming (foundation)

Adopt tokio; rework `api.rs` to stream chat-completions (SSE parse over ureq's reader); refactor the agent loop to consume tokens as they arrive while keeping every existing behavior and log shape. Spec: the fork's `llm/stream` seam. AC: a streamed two-turn tool loop against the mock server passes; token events surface progressively; the 55 existing tests still green (adapted); usage accounting unchanged.

### Slice 3 — MCP stdio client + the vendored cores

New `src/mcp.rs`: spawn `python3 -u <home>/service/mcp_server.py` children, speak JSON-RPC over stdio (initialize, notifications/initialized, tools/list, tools/call), register their tools in the agent loop under `mcp__<svc>__*` namespaces. Home resolution ported from `dsh-components`: machine home override → bundled copy under `services/` (vendored into this repo with the same `VENDOR.json` discipline) → unavailable with one named log line, boot continues. Fixes the sys1/gen1 `notifications/initialized` deviation at the vendoring boundary or upstream. AC: all four cores launch from a fresh clone with no machine config; their tools callable inside a run; a dead child degrades without killing the run; integration tests drive the real cores.

### Slice 4 — multi-provider routing

New `src/route.rs`: a roster of OpenAI-compatible gateways (DeepSeek direct, z.ai Coding Plan, Xiaomi MiMo, and the router kit at `127.0.0.1:8300` as escape hatch) in config; per-task route selection that asks `sys1_classify` (the slice-3 MCP tool) instead of a ported judge; failover walks the declared chain on failure classes; every decision appends a calibration row (slice 5's writer). Spec: `@fleet/dsh-router`'s decision semantics and the fork's route/failover row vocabulary. AC: a judged `auto` turn routes inside the roster; a failed head fails over to the next candidate; the kit serves as last resort; decisions are logged.

### Slice 5 — the calibration stream

New `src/calib.rs`: one `events.jsonl` per UTC day under the dev-decisions log root, in `devdec`'s row vocabulary, through a serialized non-blocking queue. Spec: `@fleet/dsh-decisions`. AC: route/failover/screening rows land; the external `devdec` CLI reads them beside its own; a full disk never fails a decision.

### Slice 6 — the TUI

A ratatui interface over the async core: chat and run panes with streamed tokens, the approval prompt, session browse/resume, tool-call progress, and a board pane mount point. Spec: the fork web client's information architecture (not its code). AC: `fleet` with no args launches the TUI; a full task runs interactively; approvals work from the keyboard; headless `run`/`chat`/`doctor` unchanged.

### Slice 7 — skills

SKILL.md loading from a skills directory with frontmatter, progressive disclosure (description first, body on demand), and selection by the loop. Spec: the fork's `packages/skill` and the owner's existing skill conventions. AC: skills load with malformed ones skipped by name; a relevant skill's body enters context when selected; `fleet skills` lists them.

### Slice 8 — memory (the brain)

Durable facts in a git-tracked `memories/` tree (one file per fact with lifecycle frontmatter) plus sem1-indexed recall over session journals and skills, refreshed before each instruction; a `brain_remember`-shaped write path. Spec: `@fleet/dsh-brain` and the fork's Wave D slices. AC: facts persist across sessions; recall returns ranked matches for a new instruction; the sleep/consolidation cycle runs on a timer.

### Slice 9 — subagents

Spawn child agents with isolated contexts and scoped tools; results return with verification (the child's claims are checked before the parent reports them). Spec: the fork's `packages/subagent`. AC: a delegated task runs in its own session log; the parent verifies external side effects itself; failures return partial results, not silence.

### Slice 10 — compaction

Context-window management: summarize-and-continue under a token budget with the summary logged as a session record so model-visible ⟺ logged survives. Spec: the fork's `packages/compaction`. AC: a session outgrowing the window compacts and resumes coherently; the log reconstructs every request.

### Slice 11 — scheduled runs

Cron-shaped scheduling with a persistent job store, catch-up on missed runs, and delivery to a gateway-connected surface. Spec: the fork's `packages/schedule` + `packages/jobs`. AC: a scheduled task fires on time and after a missed window; `fleet cron` lists/edits jobs; a job's output is retained.

### Slice 12 — the boards

Native read-only surfaces: router board (the kit's operator API), second-brain board (the six derived state files), tower (git facts per project). TUI panes and/or a small embedded web server with hand-rolled SVG/CSS — no chart library, matching the fork's approach. Spec: `dsh-router-board`, `dsh-second-brain`, `dsh-tower`, `dsh-board-seam`. AC: each board renders real data from its source; a refused read keeps the last snapshot visible and names the error.

### Slice 13 — install and launch

`fleet install`: home layout, config scaffold, optional heavy-lane venvs (a port of `services/bootstrap.mjs` — per-lane, idempotent, degrade-without), `fleet doctor` as the gate; release binaries per OS; the README's quick start becomes literally true on a clean machine. AC: on a clean machine, install → doctor green → `fleet chat` completes a real task; no step requires the fork or Node.

## Acceptance criteria (plan-gate)

- C1: `fleet install` on a clean machine produces a working home; `fleet doctor` green.
- C2: `fleet chat` streams; routes across the roster with sys1 judgment and failover; completes a real multi-step task end to end.
- C3: all four lanes callable as tools inside a run; calibration rows land in the dev-decisions stream and `devdec` reads them.
- C4: skills load and are selected; durable memory persists across sessions and recalls by shape.
- C5: subagents run isolated tasks; the parent verifies their results.
- C6: sessions survive compaction and resume; a scheduled task fires on time and after a missed window.
- C7: the three boards render real data read-only; a refused read keeps the last snapshot.
- C8: the local gate green at every slice; a release binary installs and launches on macOS and Linux.

## Files to be touched (first slices)

- `Cargo.toml` — tokio, ratatui/crossterm (slice 6); nothing else without a plan line.
- `src/api.rs` — streaming rework (slice 2).
- `src/agent.rs` — async loop, MCP tool registration, routing hook, calibration hook.
- `src/mcp.rs`, `src/route.rs`, `src/calib.rs` — new (slices 3–5).
- `src/tui/`, `src/skills.rs`, `src/memory.rs`, `src/subagent.rs`, `src/compact.rs`, `src/cron.rs`, `src/boards/`, `src/install.rs` — new (slices 6–13).
- `services/` — the vendored cores with `VENDOR.json` (slice 3).
- `tests/` — each slice's integration coverage at the current bar.

## Out of scope

- Reimplementing the lanes natively (they are Python services by nature).
- Shipping the fork's dsh-* packages as products; the fork stays the integration lab.
- The fork's web/desktop client parity; Fleet's surfaces are the TUI plus embedded read-only boards.
- Multi-tenant, hosted, or team anything; this is a solo daily driver.
- Windows as a first-class target until the core is daily-driven on macOS/Linux.

## Verification

Per slice: the local gate plus that slice's ACs exercised for real — MCP slices against the real vendored cores, routing against the live roster once the account has balance, boards against the live kit and the second-brain tree. The live-API e2e stays `#[ignore]`d until the DeepSeek balance is topped up; every other claim is tool-verified in-session.

## Linked artifacts

- `docs/architecture/overview.md` — slice-1 crate architecture (updated per slice).
- `docs/architecture/ecosystem.md` — the verified ecosystem map and the per-piece absorption shapes.
- `docs/plans/2026-10-12-fleet-native-app.md` — slice 1's record; this plan supersedes it as the forward plan (it stays `active` only for C3's balance blocker).
- The fork's `docs/fleet/architecture.md` and `docs/fleet/runtime.md` — the spec for the ports.
