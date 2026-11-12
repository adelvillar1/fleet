---
name: a-declaration-is-not-an-execution
description: "Fixing a tool's declaration did not make it executable — the scope gate still refused it, and only the live run showed it"
type: feedback
date: 2026-11-04
basis: inferred
tier: durable
shape: []
phrases: []
missing: []
recall-count: 1
superseded-by:
valid-until:
---

KI-410 took two passes. The first fix declared `send_file` on every run
(making the model *see* it), and the live proof showed the call still
refused: "send_file is outside this subagent's admitted scope". The
second fix exempted the delivery tools from `scope_refusal`. Same pass:
`author_persona` was declared but refused as an unknown tool, because
`prepare_call` resolves only the base-tool table and every extra tool
needs its own branch.

**Why:** a declaration and an execution are separate gates. The harness
cannot see a scope refusal — the model can name the tool and still not
use it — so a green declaration test proves nothing about usability.

**How to apply:** when a tool must work under a narrowed scope (a role, a
subagent, a delegated child), pin BOTH gates: the declaration and the
execution/refusal path. And drive it live — the declaration test passed
while the tool was unusable. Related: [[huddle-plan-completed-2026-11-04]]
