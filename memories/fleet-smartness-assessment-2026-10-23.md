---
name: fleet-smartness-assessment-2026-10-23
description: "2026-10-23 the owner asked "how smart is fleet?" — my assessment: a solidly current-gen agent loop inside an unusually smart institution (self-measurement); the harness buys ~15–25% effective capability; and the proposed A/B benchmark that would convert the essay into a number"
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

Owner question 2026-10-23, after the chat-excellence plan shipped: *"I
know it's hard to gauge without benchmarks, but how smart do you think
this harness (fleet) is?"*

**The assessment I gave:** a solidly current-gen agent loop wrapped in an
unusually smart *institution* — the gap between those two halves is the
real characterization.

- **The ceiling is mostly the model's.** A harness adds maybe 15–25%
  effective capability on multi-step work (context quality, tool
  selection, verification discipline, recoverability). Fleet earns that
  margin in: the "done means tool-verified" contract + gate culture
  (fewer fabricated completions), window-aware compaction, judged
  routing with failover, and the operator ergonomics just shipped
  (edit-and-resend, content search, the context inspector).
- **Genuinely ahead of anything at this scale: self-knowledge.** The
  calibration stream, the judged beats, the eval corpus gating changes,
  thresholds measured live (the 0.68 floor), the 300+-row honest
  ledger, plan-gate scoring plans before they're built — plus hard rule
  5 and the byte-equality pin make "what did the model see" a CHECKABLE
  claim. Most harnesses fly blind on their own behavior; this
  compounds.
- **The lanes are an unusual asset used unevenly** — route labels
  partially decorative ([[fleet-router-deep-dive-2026-10-21]]), the
  embedding recall measured badly
  ([[embeddings-cannot-label-a-session-project]]); shape-routing and
  literal search do more real work than the semantic layer.
- **Where it's not smart: long-horizon autonomy.** Depth-2 delegation,
  no parallel subagent farms, no sandbox-first execution, no
  speculative parallelism. On a 40-task backlog a frontier closed-loop
  agent wins on throughput; fleet wins on auditability of every step.
  The eval corpus (20 scenarios) catches regressions but is small
  enough to overfit to; KI-284's write guard is a dumb tax on a smart
  system.

**The sharpest honest point, and the proposed experiment:** fleet is one
of the few harnesses that could MEASURE its own smartness and hasn't at
scale. The A/B I proposed: same model, bare loop vs fleet, ~50 real
tasks from the owner's actual repos, scoring completion rate,
turns-to-done, wasted calls, and spend — the instrumentation already
exists, it's a day of setup, and I'd expect fleet to win on
turns-to-done and waste more than on raw completion (exactly the margin
a harness is supposed to buy). Nobody has run it; the offer stands.

**Same-day update (owner, evening):** "the router has an agent fanout
execution type (swarm)" — the tool-use improvement plan is being implemented
and the execution-topology verdict is live, which directly closes the
long-horizon-parallelism weakness named above; the honest follow-up is
whether fan-out actually wins (same task set through `single` vs `swarm`,
scored by the calibration stream). Velocity context the owner take pride in,
verified: first commit 2026-10-12, 487 commits total, **253 in the last 7
days** (~36/day), 57 plan contracts, gate 1,591 — and the reasons it is real
rather than lucky: fleet is a successor (architecture lessons from the fork
came free), the process discipline is the accelerator (ledger + gates +
memory turn failures into rules), and fleet builds fleet (same-day
dogfooding is the fastest bug-finding loop).

Related: [[chat-excel-direction-2026-10-23]],
[[fleet-tool-surface-analysis-2026-10-23]],
[[fleet-judgment-beats-spawn-per-row]],
[[harness-landscape-converged-spec-2026-10-16]].
