---
name: zcode-router-config-vs-app-provider-order
description: "zcode's routing has two config files that can disagree — the router's own config (~/.zcode/router/config.json) already routes images to the multimodal mimo, while the app-side providerOrder (~/.zcode/v2/provider_config.json) was pinning every request to the kit's auto-router on 8300"
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

zcode's model selection is governed by two *separate* files, and they
can silently disagree:

- **The router's own config** is `~/.zcode/router/config.json`. Its
  `routing` block is loaded straight into `R` (server.js line 71,
  `let R = config.routing`), so `routing.omniModel`,
  `routing.wideModel` and `routing.workloads` are the router's whole
  routing table. `omniModel` and `wideModel` both point at
  `xiaomi-mimo/mimo-v2.6-pro`, and the capability rule fires when a
  request carries images (`images + otherParts > 0`) and routes to it.
  This side was already correct for vision.
- **The app-side provider order** is `~/.zcode/v2/provider_config.json`.
  Its `config.providerOrder` array decides which provider a *new session*
  lands on. It had `auto-router` at the head — a provider whose `baseUrl`
  is `http://127.0.0.1:8300/v1`, i.e. the kit — so every request went out
  to 8300 and the kit picked `stepfun/step-5-preview`, overriding the
  app's own multimodal pin (`builtin:zai-start-plan/GLM-5.3-Flash`).
  `auto-router`'s provider *definition* lives in the same file's
  `providerConfigRules.providerRules` and can stay while its order entry
  is removed — only the pin is taken out, not the capability.

Removing `auto-router` alone does **not** restore vision: the next
provider in line was `deepseek`, and neither `deepseek-flash` nor
`deepseek-v4-pro` declares image support. In that config the only models
declaring `supportsImage: true` are `xiaomi-mimo/mimo-v2.6-pro` and
`mimo-v2.6-flash`. So the fix was to move `xiaomi-mimo` to the head of
`providerOrder`, which lands a new session on a multimodal provider
directly instead of routing through the kit at all.

**Why:** the router config looks like the place to fix routing and is
already right; the thing actually pinning the wrong model is a different
file entirely. Fixing one and not the other either does nothing or lands
on a text-only model. This is the same two-file split the owner has
already ruled on for Fleet — the kit is the precursor, not the router —
and it recurs here in zcode's own harness config.

**How to apply:** when a vision/routing symptom appears in zcode, read
`~/.zcode/router/config.json`'s `routing` block *and*
`~/.zcode/v2/provider_config.json`'s `providerOrder` separately, and
check which models actually declare `supportsImage` before concluding.
Both changes need a *new session* — nothing flips mid-session. Note that
`~/.zcode/v2/provider_config.json` holds live plaintext API keys for
deepseek, mimo and token-plan. See also
[[zcode-session-model-overrides-the-config-pin]] and
[[fleet-routes-through-its-own-router]].
