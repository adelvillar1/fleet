---
name: sys1-pool-is-all-hosted-no-extraction-wire
description: "Verified 2026-10-24 by doctor run — every usable sys1 provider is hosted API; the \"local venv\" GLiNER providers are dead code on this machine; no sys1 wire does span extraction"
type: feedback
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

Verified 2026-10-24 by running `sys1 doctor` (PYTHONPATH=src, services/sys1): the only
available providers are **decide, drex, glide, jev, liquid — all hosted APIs**. The four
"local" registrations (`local`, `decide_1b`, `decide_multi`, `modernbert`) are dead on this
machine: venv not found at `/private/tmp/gliner-decide` (a reboot-wiped default). julia/
scaledown/typellm lack keys in `~/.config/dev-decisions/env` (which holds only DREX,
FASTINO, LIQUID, TYPESAFE); the candidate tier is flag-disabled. The router's default chain
is `glide,drex,jev` — "hosted-only by design — local/modernbert are never auto-routed"
(config.py). I wrongly claimed "GLiNER 2.5 runs from your local venv, already wired" — the
owner corrected me: **run `sys1 doctor` before asserting what the pool can serve.**

Second fact, same audit: all four sys1 wires (openai/jev/gliner/modernbert) answer
classification heads — **no sys1 wire extracts spans**. GLiNER 2.5 Multi's extraction
(entities/structures/relations, spans+confidence) is NOT wired anywhere; the wired GLiNER
is hosted Decide = classification. The extraction-class capability exists at the already-
keyed vendor: Fastino hosted `POST /v1/gliner-2` or `/v1/chat/completions` with model
`fastino/gliner2.5-multi-v1` ($0.03/1M input, 4,096-token pass, async jobs for >1M token
inputs) — the re-target candidate for the extraction leg instead of Cloudflare Clairic.
See [[fleet-judgment-beats-spawn-per-row]], [[fleet-tool-surface-analysis-2026-10-23]].
