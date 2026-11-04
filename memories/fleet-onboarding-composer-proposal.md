---
name: fleet-onboarding-composer-proposal
description: "2026-10-23 owner proposal — a composer-interface onboarding screen that pitches lanes by name ("setup a system one model like jev, julia1, d1, glider, glide, typellm and you could get this — give me the API key and I'll set it up for you"), the just-in-time-configuration candidate first artifact of the settings abstraction"
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

Owner proposal 2026-10-23 (end-of-day design conversation, NOT yet built): an onboarding
screen built on the composer interface — not a wizard — that pitches capabilities by
name: "hey, if you setup a system one model like jev, julia1, d1, gliner, glide, typellm,
you could get this. If you give me the API key I can set it up for you" — and the same
for the other lanes (sdm1/TabPFN, blend1/composed grounding, gen1/media+voice, browser/
live web).

**Why it composes without new construction** (verified in-tree the same day): the catalog
already knows every provider's `key_env` and reports presence/unset with source; the
vault already has first-run onboarding that creates and seals; the route tiers already
resolve live, so the route bar flips from "no route yet" to a judged route the moment the
key lands — the promise kept in the same surface the promise was made in; and the MCP
registry (see [[mcp-registry-live-import-state]]) already proved the scan-and-offer
pattern.

**Design notes**: offers must read presence from the catalog, not a hardcoded list,
ordered by routing value (the sys1 judge first — it upgrades everything else: judged
routing, plan-gate, judge/delegate/explain, the convention retrieval half of the write
guard); the local lane (sem1/EmbeddingGemma-2) never asks for a key — it offers to start
its server. The trap to avoid is the wizard with fields: the composer stays the
interface, the offer is one sentence, the action is one paste, and "I can set it up for
you" means the agent writes through its own writers — the same stance as
[[no-form-asks-for-a-value-fleet-already-has]] and guarded by
[[fleet-anti-hermes-constitution]].

**Significance**: this is just-in-time configuration — "work now; fleet tells you, at the
moment of the task, what would make it better, and does the setup when you hand it the
key" — and the candidate first artifact of the abstraction the owner said they hadn't
found yet ([[fleet-settings-abstraction-search]]). The settings board stays for the five
questions; the onboarding composer is configuration pitched as capability.
