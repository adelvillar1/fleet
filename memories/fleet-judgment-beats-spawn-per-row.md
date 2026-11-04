---
name: fleet-judgment-beats-spawn-per-row
description: "2026-10-23 — fleet judge/delegate spawn the sys1 MCP lane once PER ROW and the execution head was parsed away; 2026-10-23 evening the owner reported the plan IS being implemented and the router now HAS the agent fan-out execution type (swarm)"
type: project
date: 2026-11-04
basis: confirmed-by
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

Found 2026-10-23 by mapping the typellm and fastino skills onto fleet's
seams (the owner asked where fleet could improve using them). Two costs,
both read from the code:

- **One lane spawn per row.** `fleet judge` (`src/judge.rs`) calls
  `ask_lane` per route row, and `ask_lane` spawns the sys1 MCP server,
  initializes it, lists tools, makes one `sys1_classify` call and tears
  it down; `src/delegate.rs` has the same shape. A day on this machine
  carries 81–162 route rows (calibration stream, 2026-10-23), so one run
  boots the Python lane that many times at ~1 s a boot. The router's own
  `judge` already does it right — one spawn per session.
- **The execution head is paid for and discarded.** sys1's `route` task
  judges two heads in one call: the workload and the execution topology
  (`single`/`mixture`/`swarm`). `parse_workload` reads only the workload
  — the same fact recorded in [[fleet-router-deep-dive-2026-10-21]].

**The pattern that transfers, not the vendor.** typellm's measured win
is one call answering N segments (JobHound 4m40s → 14s, 74 → 10 calls);
fastino's is multi-task scoring in one forward pass. Neither API is
adopted — the batch *shape* is what fleet lacks. The sys1 batch class
(one `sys1_classify` call, answers read back in order) is upstream work
plus a re-vendor, and is deliberately out of the plan.

**The plan:** `docs/plans/2026-10-23-judgment-beats-one-lane.md`, six
criteria C0–C5, plan-gate PASS (6/6), status `draft` awaiting the
owner's approval of two mockups (the calibration row naming both heads;
the leader's prompt line). C0 `ask_lane` becomes a session (open/ask/
close, one per beat run, shared by judge and delegate) pinned by a
counting seam; C1 both heads parsed and recorded; C2 a `Judged` walk
whose verdict is `mixture`/`swarm` walks the orchestrator's chain (C3:
the leader gets one prompt line naming `subagent`); C4 the `routed`
event and the docs; C5 one open and no row left unjudged.

**Also considered, held for a second plan:** Fastino/TypeLLM used as
real APIs — GLiNER extraction feeding the proposal gate (~$0.000002 per
posting), and Decide for a fact's `shape`. The skill's caveats are
load-bearing: structure scalars come back null or hallucinated,
relations return no pairs on real text, batched `text: [...]` 503s.

**Why:** the boot cost is pure waste with no behavioral gain, and the
execution head is the cheapest way to make the sys1 judgment
load-bearing instead of decorative.

**How to apply:** when adding a per-item lane call anywhere in fleet,
open the lane once per run; never claim the router's judgment steers
assignment until C2 lands. Related:
[[fleet-router-rules-everything-but-the-leader]],
[[orchestrator-routes-through-hard]].
