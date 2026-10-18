---
name: trajectory-phrases-wiring
description: "`trajectory_phrases` derives its direction from successful tool calls in the current task window only."
type: project
date: 2026-10-15
basis: inferred
tier: durable
shape: [change]
phrases: [sed -n '2010,2075p' src/agent.rs, src/agent.rs, grep -n "trajectory_phrases" src/agent.rs]
missing: []
recall-count: 97
superseded-by:
valid-until:
---

`trajectory_phrases` derives its phrases from the current task's trailing window only: after the last user message, it walks assistant turns in reverse and pairs each tool call with its tool result by call id. It includes only successful calls—those whose result exists and does not start with `error:` or `denied`—and extracts one phrase per call via `call_phrase`: the bash command, the file path, or the tool name. Failed calls, denied calls, calls with no result yet, and earlier tasks' calls are excluded.
