---
name: zcode-session-model-overrides-the-config-pin
description: "A stripped image attachment means the session is not on the pinned multimodal model, not that the pin lacks vision — zcode's config pins GLM-5.3-Flash (text/image/video) but a session can silently run on stepfun/step-5-preview, which accepts no image input"
type: project
date: 2026-11-05
basis: inferred
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

The zcode config (`~/.zcode/cli/config.json`) pins
`builtin:zai-start-plan/GLM-5.3-Flash`, whose declared input modalities
are text, image, and video — it is multimodal. A session does not have to
run on the pin, though: on 2026-10-16 this one ran all 267 requests on
`stepfun/step-5-preview`, which accepts no image input, so every image
attachment arrived stripped as "the selected model does not support image
input." That message reads as a statement about the model's capabilities,
but it is really a statement about which model the *session* is on.

`config.json` had not been written since 2026-10-06, so the override is
session-level — a `/model` switch or a `--model` flag at launch — and it
holds for the whole session once it takes. It cannot be flipped from
inside the session.

**Why:** the stripped-image message points at the wrong cause, so
diagnosing from it alone concludes "the pinned model isn't multimodal"
when the truth is that the pin is being ignored. A pin that gets silently
overridden is worse than no pin, because it makes the config lie.

**How to apply:** when an attachment comes back stripped, read the
session's own request log (`~/.zcode/cli/rollout/model-io-sess_*.jsonl`,
the `model` field) to see what actually ran, and tell the user the session
needs switching rather than trying to fix it in-session. See also
[[owner-stated-facts-are-authoritative]].
