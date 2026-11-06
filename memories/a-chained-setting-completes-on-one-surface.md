---
name: a-chained-setting-completes-on-one-surface
description: "feedback 2026-11-01 — the owner's settings verdict was not about rows or gaps but about CHAINS; I had no add/edit/reorder/remove flow for a provider or a lane, and once shown one they generalised it to every chained setting (\"that philosophy should permeate to every surface that has chained settings, the flow should help the user get things done\")"
type: feedback
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

Third turn of the 2026-11-01 settings work, after the interaction-gap
plan and the model-first mockup. The owner asked me to walk the
add/edit/remove flow for a provider — *"I just don't see it here and
that is important to me"* — and named the expectation: *"If I add a
provider I expect that if the provider exposes it I get all the models
and their settings. This flow should also include the ability for me
to assign the workloads and order. A user should not be jumping
through 3 or 4 different screens to do that flow. The same with the
lanes providers."* Then, once the account card existed:
*"that's what I mean by coherent flow and that philosophy should
permeate to every surface that has chained settings."*

Then the existential frame, which is the reason to remember it: *"I
built this harness, and I know functionality and settings, but the
settings experience feels like a wall, even to me, imagine users that
have never interacted with the harness. that's what drove me away from
hermes agent to begin with, and that's why I built this harness."*

**Why:** my gaps were all *single-row* gaps — a confirmation, an undo,
a filter, a validation. Every one can be fixed while the screen still
requires the owner to carry a chain across four screens by memory: key
in one card, models in another, the walk in a third, the removal in a
fourth. A flow is judged by whether the TASK finishes on one surface,
not by whether each row is clean — and my own model list ("press a
model to switch") and my per-row budgets were still an inventory.

**Follow-on, same session:** after the chain cards were mocked the owner
reported the real face again — *"I just don't see in that screen a way
to add a provider, which is a glaring gap / or modify it, or delete it"*.
It was there, **sixty-eight model rows below the fold**, because I put
the model list first and bounded nothing. A flow that exists below the
fold does not exist: bound the long list, and put the section's ACTIONS
(connect, then one row per provider) directly under it.

**How to apply:** for any setting that is a chain (a provider to its
models and their settings to the walks it serves to its removal; a lane
to its tools to its walk; a job to its task and cadence; a scan to the
servers it finds), build ONE card that carries every step — opened
where the press was, with the side pane standing down so there is
exactly one surface — and make **adding the same card, empty**. Keep the
chain's ENTRY POINT visible: the action row sits in the section's own
viewport, never under a list long enough to bury it. Cover
the whole lifecycle in that card: discover, assign, order (up/down/
stop), and remove with the blast radius named. Then translate the
first screen for a newcomer: a fresh home opens on a short ladder of
presses with benefit sentences, and the counts and chip wait until
there is something to count. Rows speak the owner's words with the
config key on the card. The test is "does the task finish here", never
"is the row one line". Related:
[[the-surface-keeps-the-machinery-and-hides-the-answer]],
[[fleet-anti-hermes-constitution]], [[settings-ux-plan-2026-11-01]].
