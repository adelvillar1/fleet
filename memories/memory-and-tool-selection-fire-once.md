---
name: memory-and-tool-selection-fire-once
description: "Measured 2026-10-30 — memory is assembled once against the opening task and frozen into the system prompt; tool activations never expire and dead lanes keep declaring — the gap is WHEN the decision is made, not where it is hosted"
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

The owner asked 2026-10-30 whether adding `mem1` and `tool1` lanes ("carry the most relevant memories and tools at any given moment") would improve decision-making. I mapped both paths rather than building. The selection machinery is good; it fires once, at the wrong moment.

**Memory** — `memory.rs` scores six channels: vector cosine 0.15, keyword 0.15, **trajectory phrase coverage 0.30**, recall history 0.25, 90-day freshness 0.10, provenance 0.05 (`:1265-1296`). Shape-partitioned, 2,048-token budget, protected tier never evicted (`:3078`). **But `prepare_memory(task)` runs at `agent.rs:2255`, before the `while` loop at `:2260`, against the ORIGINAL task only**, and its block is baked into the system prompt (`:2256`). Every later turn reuses that string verbatim; a steer does not re-rank it; compaction replays it unchanged. The only per-turn recompute is the model choosing to call `recall` (8 facts, capped, read-only). A session that starts on "fix the lane probe" and ends three hours later debugging memory ranking still carries the lane-probe memories.

**Tools** — per-turn assembly (`agent.rs:2315`) but run-scoped state: `tool_search` inserts into `self.activated` (`:4174`, `:4184`) and **nothing ever removes it** — every use is insert, read, init, or declare. The field comment says "the next request" (`:1127`) and the model is told so (`:4177`), but five tools activated in turn 2 are still declared in turn 40. And `LaneChild::Dead` deliberately keeps its tools declared for the rest of the run (`:6555-6569`, `:6834-6844`) so the log's `lane_tools` record stays consistent — so a lane that dies at turn 3 advertises five failing tools to the end. **There is no availability predicate**: "declared" is never "available".

**Why lanes are the wrong container for the selection** (four reasons, strongest first):
1. It violates the house propose/dispose rule stated in sem1's own doc — *"vectors in, vectors out — embeddings propose, the decision layers dispose."* A lane returning "use these three" is disposing; no lane does that today.
2. State duplication: the 114 fact vectors are loaded once into the agent. A `mem1` would reload them per spawn or ship 768 floats × 114 facts per turn.
3. It feeds the 8901 load-amplifier trap — [[one-backend-many-callers-is-a-load-amplifier]], KI-325/344/346 are all saturation wedges there. In-process re-curation adds zero callers.
4. Prompt-cache churn: re-assembling memory into the system prompt every turn guarantees a miss on every request; the prefix must stay byte-stable.

**Where the lane pattern does fit: sys1.** Its contract is already "closed-set label + confidence over text, fail-open" — exactly a per-turn `curate`/`tool_route` task returning `{focus, needs_recall, staleness}`, with `focus` from the class vocabulary `tool_router.rs:285-310` already declares. Same cache pattern as `route_cache.rs`. It would also repair a live defect rather than add surface — see [[router-lanes-deep-dive-defects-2026-10-30]] where `tool_route` is unregistered.

**The shape I recommended** (3 slices, in place before any new component): (1) re-assemble memory on topic drift (cheap cosine between successive user turns; unrelated 0.58-0.66 vs related 0.77-0.93, `gui/related.rs:28-31`) and on a failing tool result; scope activations to N turns; drop dead-lane tools on the `Dead` transition with a log record replacing the declaration. (2) Extend the ranker to core tools, not just lane tools. (3) Only then register the sys1 curation task for the *label* while Rust keeps the *selection*.

One-line version I gave the owner: **"you want a curator, not two more lane services — the curator's judgment belongs in a lane while its bookkeeping belongs in the loop."** No plan file written yet; the owner has not chosen a slice. Next KI is 375.
