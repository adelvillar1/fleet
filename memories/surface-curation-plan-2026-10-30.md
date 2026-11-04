---
name: surface-curation-plan-2026-10-30
description: "Plan draft at docs/plans/2026-10-30-surface-curation.md — the curator (per-turn memory+tools) instead of mem1/tool1 lane processes; C0-C3 fixes, C4 the lane gated"
type: project
date: 2026-11-04
basis: inferred
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

Plan IMPLEMENTED 2026-10-30 as commit a7007a4 + the K5 completion babb109 (real session 997a54b4's four steers replayed against the production embedder through maybe_curate's algorithm: fires at steers 1 and 9, holds at 2/3 with cos 0.882-0.884, floor 0.78 stable across 0.72-0.84, zero mis-fires; short-imperative caveat recorded) (`feat(surface): the curator`) — status completed, K1-K8 checked, Landed section written. Gate: fmt 0, clippy 0, build 0, test 1,645 lib + 188 integration, 0 failed.

What landed beyond the draft's skeleton:
- C0: `Record::LaneFailed` widened to any death; `declared_lane_tools()` (live only) beside `lane_tools()` (log shape); the face's lane card carries `withdrawn` (3 app.js sites), TUI `LaneCard.withdrawn` renders the death.
- C1: `activated` = `BTreeMap<String,u64>`, `ACTIVATION_TURNS=3`, `call_lane` re-stamps; sys1 `tool_route` task registered (VENDOR.json `fleetAdditions`).
- C2: `Memory::instruction_vector`, `maybe_curate` in the steer path, first steer curates by definition (`steered_once` flag), keyword-only corpus = every steer drifts; `DRIFT_FLOOR=0.78` measured live (unrelated steer pairs 0.647-0.729, paraphrases 0.846-0.925 — steer pairs read HIGHER than KI-058's fact bands because imperative asks share a register); `Record::Curated { turn, shape, facts }` as a NEW variant (not a Memory stamp — `.find()` readers keep meaning "the opening block"); note = system message at the request tail, turn-scoped.
- C3: `withhold_core` + `CORE_KEEP` (8 working tools + capability trio + memory pair + subagent/process/use_skill); `Record::Declarations` gained `withheld` (serde default); real surface measured **28 declarations / 7,293 tokens / 0 withheld** — dormant by design (the plan quotes it).
- KI-375/376/377 filed and closed in the same landing.

Trap hit twice: `Record` new variants break EVERY exhaustive match — the compiler finds them (bridge no-card group, tui entry_of, eval, tool_advisor); and `tools::declarations(&lanes)` ALWAYS includes the 11 built-ins, so "empty declarations" is really "len()==11".

Answers the owner's question "would a mem1 and tool1 carry the most relevant and available memories and tools at any given time". **The answer baked into the plan: a curator, not two lane services.** Both decisions already exist and are good — memory is a six-channel hybrid (weights in `memory.rs:1265-1296`, budget 2048) and tools have a 32/9000 tier plus a 0.68-floor ranker. The defects are timing and scope, not mechanism.

Increments:
- **C0** dead lane stops declaring — reuse `Record::LaneFailed` (`session.rs:146`), widen doc from "run start", teach the replay readers (`agent.rs:1601`, `gui/bridge.rs:5048`, `tui/app.rs:3114`) that LaneFailed-after-LaneTools withdraws.
- **C1** activation expiry (turn-stamped `activated`, `ACTIVATION_TURNS` ≈3) + register a `tool_route` task in vendored sys1 with a `capability` head. NOT a rename to `route` — `route` answers workload/execution, `parse_capability` reads `capability`.
- **C2** memory re-curates on drift, delivered at the message TAIL (system prompt byte-identical, pinned) — `Record::Memory` needs a turn stamp or a new variant.
- **C3** ranker over core tools (the token win) + measurement.
- **C4** deferred: the curation judgment as a sys1 task, with entry criteria (cosine drift mis-firing on real sessions, or wanting to swap salience policy without a rebuild).

Design rules the plan commits to: trigger on drift + tool failure (never a tick); deliver at the tail (protect prompt-prefix caching); cache verdicts by hash (`route_cache` pattern); expiry is part of activation; availability is a logged transition.

Four reasons no mem1/tool1 process: propose/dispose house rule (sem1's "embeddings propose, decision layers dispose"); fact-vector state duplication; the 8901 load amplifier (KI-325/344/346); prompt-cache churn. Lane spawn measured live 53–99 ms (blend1 721 ms).

Related: [[router-lanes-deep-dive-defects-2026-10-30]] holds the three defects this plan closes.
