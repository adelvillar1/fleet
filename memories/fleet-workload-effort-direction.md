---
name: fleet-workload-effort-direction
description: "2026-10-21 owner direction + researched facts for per-workload reasoning-effort control on the router — judged workload picks effort, orchestrator ruling still picks the tier; per-provider parameter table and the MiMo reasoning_content prerequisite"
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

Owner direction 2026-10-21 (router unpinned again): the router should let them set
an effort level per task type — e.g. GLM-5.3-Flash at max effort for hard tasks,
medium/low for easy ones. Same model, effort keyed by the **judged workload** —
which is the design insight: the orchestrator ruling ([[orchestrator-routes-hard]])
keeps choosing the model tier, the judged workload chooses how hard it thinks, and
sys1's judgment becomes load-bearing for the first time.

Researched facts (owner's roster, 2026-10-21):
- Polling can't discover capabilities — the OpenAI-compatible `/models` returns ids
  only. The pattern is a compiled-in per-(provider, model) effort table beside
  route.rs's CONTEXT_WINDOWS, plus an optional one-shot probe; zcode's own
  optionSpecs table is the same precedent.
- zai-coding-plan / GLM-5.3-Flash: `reasoning_effort` = low/high/max (default max);
  coding-plan endpoint maps none/minimal/low→low, medium/high→high, xhigh/max→max;
  thinking can NOT be disabled on GLM-5.3 (`thinking.type: disabled` errors; low is the floor).
- stepfun step-3.7-flash: `reasoning_effort` = low/medium/high (OpenAI-compatible).
- xiaomi-mimo: `reasoning_effort` + hybrid thinking; **prerequisite**: MiMo multi-turn
  tool sessions require `reasoning_content` passed back on prior assistant tool-call
  turns or responses come back empty — fleet's src/api.rs has ZERO reasoning_content
  handling today. Own increment.
- token-plan / qwen: `enable_thinking` (bool) + `thinking_budget` (token cap), not levels;
  `enable_thinking: true` errors on the NON-streaming path — which is what the vision
  side-route uses (client.chat, agent.rs).
- deepseek-chat: `reasoning_effort` = none/low/high (default high)/max.
- ollama-cloud gemma4: `think` low/medium/high/none, with a known forwarding bug on the
  OpenAI-compatible path (ollama#15293). blend1 is an MCP lane (lane's business); liquid
  is declared-only.

**Why:** the owner asked not knowing what each model supports or whether endpoints can be
polled; the answer (declare + probe, per-provider adapter) and the design shape
(effort by judged workload, clamped to declared levels, named on the decision row,
riding calib::RouteRow) were worked out and accepted as the direction, not yet built.

**How to apply:** any plan for this is plan-sized (api.rs, route.rs, settings.rs,
routing.json schema, the face — draft as docs/plans/2026-10-21-workload-effort.md
per [[plan-is-a-file-not-prose]]); scope the MiMo reasoning_content passback as its
own increment ahead of or with the effort work. See [[fleet-router-deep-dive-2026-10-21]]
for the pipeline this plugs into.
