---
name: extraction-leg-repointed-to-sys1
description: "2026-10-25 owner ruling — the extraction leg was rebuilt from Cloudflare Clairic onto sys1's own hosted models (jobhound locate→segment→verify→reconcile); the wrong shape does not stay"
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

2026-10-25, commit `6b9f3af`: after the owner's GLiNER challenge and the
[[sys1-pool-is-all-hosted-no-extraction-wire]] deep dive, the ruling was
**"if we need to rewrite things to use sys1 appropriately we do that, and don't
invent a new approach because what's wrongly built should not stay."** The
Clairic client, the `CLOUDFLARE_ACCOUNT_ID`/`WORKERS_AI_API_TOKEN` pair, and the
four-verb field types are DELETED — `src/extract.rs` is now: locate (one Fastino
GLiNER 2.5 Multi entity call, fields as entity names, spans+confidence) →
segment (pure Rust, jobhound's merge-fragments rule; uppercase joins the
sentence-continuation set because prose cuts at "end. The") → verify (one noul
call per carrying segment over the sys1 chain glide→drex→jev, keys-present
filtered) → reconcile asymmetrically (jobhound's measured floors: assert 0.85,
veto 0.35, contested between with one capped adjudication to the chain's next
verifier; every answer carries its quote). Absence is structural (named reason,
no p_null); estimates/forecasts have no enum variant at all — the strongest form
of the illustrative-refusal law. KI-312 closed dissolved: the live probe
(`scripts/extract-probe.sh`) needs only the already-present `FASTINO_API_KEY`.

**Why:** a second vendor + second key for a capability the tier's own vendor
serves was the wrong shape; the E0 credential blocker was answering the wrong
question.

**How to apply:** when a leg duplicates a capability the keyed tier already
carries, re-point before extending; when the owner says rewrite, delete the old
shape rather than shimming it. Inheritors E3–E6 still wait on the live
measurement (run the probe, record latencies in the plan's re-point section).
