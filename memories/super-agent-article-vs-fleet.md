---
name: super-agent-article-vs-fleet
description: "The offloop.org DeepSeek Super Agent article's own benchmark shows delegation-first prompting closes most of the router-architecture gap, which supports rather than threatens Fleet's single-orchestrator design."
type: design
date: 2026-11-02
basis: inferred
tier: durable
shape: [docs]
phrases: [super agent, router and worker sessions, delegation-first prompt, dsh-superagent, offloop article, scheduling experiment]
missing: []
recall-count: 0
superseded-by: super-agent-explorations-for-fleet
valid-until:
---

Source: https://offloop.org/learn/deepseek-super-agent-design (Nov 2026, about dsh-superagent / the DeepSeek Harness lineage Fleet forked from). Architecture: persistent Router layer + parallel Worker sessions + event triggers + thread subscriptions + watchdog; explicitly research software, single-repo scope.

Its scheduling benchmark (~300 messages, ~20 work requests, coding replaced by a time-consuming tool, n=5): P50 completion 12.4 min for Router+Workers vs 13.8 min for a single session with subagents under a delegation-first prompt vs 20.4 min neutral prompt; cache-read tokens 23.23M vs 10.82M vs 6.05M. So the delegation-first prompt closes ~90% of the latency gap at less than half the token cost — most of the "super agent" benefit lives at the prompt layer, not the architecture layer. Removing direct delivery (events routed to the right worker) +37% tokens at equal time.

The article itself concedes it measures coordination under concurrent requests, not code quality, and is not a comparison against Claude Code/Codex.

Fleet mapping (2026-10-30): Fleet is deliberately the single-session-with-subagents column — in-process router (model routing, not task routing), orchestrator + bounded delegation (n+2 depth, capability-as-scope, owner steer/stop), no event entry points, knowledge-persistent (memories/plans/recaps/logs) instead of process-persistent. The owner's quota rulings (spend other plans first, DeepSeek in reserve) rule out a 4x cache-read multiple for concurrency a solo operator doesn't have. Adoptable idea, unbuilt: a watchdog as continuous sys1 judging over the calibration stream (fleet calib/judge already has the data).
