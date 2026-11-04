---
name: vision-chain-walks-glm-then-mimo
description: "owner steer 2026-11-03 — vision work should go to any configured model that has vision; the vision workload now walks GLM-5.3-Flash → MiMo-2.6-Flash → step-5-preview → gemma4, because fleet has NO capability metadata on models"
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

Owner steer, verbatim: *"delegate vision models to any model that is setup on
zcode that has vision, GLM-5.3-Flash, MiMo-2.6-Flash, etc."*

**Landed the same turn:** the vision workload's chain is now
`GLM-5.3-Flash → MiMo-2.6-Flash → step-5-preview → gemma4:31b`, closing on the
failover tail like every order the router builds. The vision tool, the design
review read, and any image-bearing task all walk it. Pinned by
`the_vision_workloads_chain_reaches_gemma4_on_ollama_cloud`, whose order list
grew by the MiMo entry.

**The finding that matters more than the swap: fleet has NO capability
metadata on models.** Nothing in the roster says which models are multimodal —
so "has vision" is *declared* in the chain, never discovered. That is the
same shape as the earlier gemma4 work ([[fleet-vision-gemma4-default]]), and
it is why the owner's "any model that is setup on zcode that has vision" cannot
be satisfied literally today: the only way to add a multimodal is to name it.

**What makes this manageable rather than a defect:** the vision workload is one
of the editable tiers in settings — `compiled_in_workloads()` lists every
preference, vision included — so the owner can add any configured model to that
chain **per-home, with failover**, with no code change. The missing piece is
discoverability: you have to know that "the vision tier" is where multimodal
eligibility lives. A **"vision models" card** in the router settings was
offered — naming the chain, showing which entries resolve right now (key
present or not), and letting the owner add any model from the catalog — and
not yet built.

**Why:** the owner's mental model is "the models I configured are the ones
fleet should use", and a chain that silently omits a configured multimodal
reads as fleet not knowing about it. **How to apply:** when the owner names a
model family for a capability, check whether fleet can *discover* that
capability; if it can only declare it, say so and offer the settings-backed
surface rather than editing the compiled chain for every new model. Related:
[[fleet-vision-gemma4-default]] (the earlier vision reserve and the same
declared-not-discovered shape), [[liquid-is-two-services-lfm-chat-and-d1-system-one]]
(a model that is not what its name suggests), [[huddle-plan-implemented-12-of-14]].
