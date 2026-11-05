---
name: typellm-provider-in-sys1
description: "2026-10-24 owner directive mid-session — TypeLLM added as sys1's tenth core provider (f0faebc, pushed); the wire was learned from the installed SDK's runtime then live, and the key rode the env chain only, never a committed file"
type: project
date: 2026-11-05
basis: confirmed-by
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

The owner's mid-session directive ("Include TypeLLM as a sys1 provider, check
the skill to understand the capabilities") plus the API key pasted in
conversation. The typellm skill carried everything but the wire itself — the
SDK (`~/Projects/jobhound/apps/worker/.venv/.../typellm/runtime.py`) hid the
endpoint, so the wire was learned in two steps: read the SDK, then probe live.

**The wire (probed live 2026-10-24):** `POST https://api.typellm.ai/v1/generate`,
`Authorization: Bearer TYPELLM_API_KEY`, body
`{context, questions, images: [], options: {mode: null, seed}, timeout: 85}`.
Enum fields are `{"type": "string", "enum": [...], "return_probabilities": true,
"instructions": ...}`; noul is `{"type": "boolean"}` — the wire REFUSES
`type: "bool"` ("return_probabilities ... only supported for enum or boolean
fields" — the 400 that taught this). Answers carry
`{value, probabilities, confidence}` with probabilities keyed by the labels
(strings "true"/"false" for booleans); usage is `{input_tokens,
thinking_tokens}`, no cost field. The vendor's ceiling is 90 s; the wire sends
85 (the skill's own default).

**The sys1 mapping (upstream commit `f0faebc` on `feature/plan-surface`,
pushed):** `_typellm_wire` (one POST per classify, context bounded by
`typellm_max_chars` 200k), `Head::to_typellm_head` — score heads answer an
enum of the ABSOLUTE level digits (score_min..score_max, rubric folded into
instructions; the parser shifts nothing, unlike the 0-indexed
drex/glide/scaledown wires), descriptions fold into instructions as a rubric —
and `parse_typellm` — value renames to the shared wire; noul carries P(yes)
from the distribution (the scaledown semantic); a missing answer declines by
name. Jev-class head-set inheritance (`heads_for` tuple), tenth core provider,
**not in the default route chain and floors unfitted — the scaledown
admission**. Proof ran through sys1's own `classify()`: the change task's
three heads answered with labels and full distributions.

Key handling is part of the fact: the owner supplied the key in conversation;
it rode the env chain only (inline env for the probe, never a committed file,
never printed) — sys1's own hard rule about key values. Related:
[[fleet-ki-triage-pass-one]] (the roster's provider rows),
[[owner-is-a-team-of-one]] (rulings arrive mid-session and shape the plan).
