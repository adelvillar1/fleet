---
name: the-surface-keeps-the-machinery-and-hides-the-answer
description: "feedback 2026-11-01 — the settings review's whole point was to REMOVE complexity; I shipped ten interaction-gap fixes plus a models mockup that still opened on workload tiers, candidate chains, budget dashboards and billing classes; the owner named the bar ("more like zcode, deepseek harness, less like hermes") and cut the money section outright — then closed the relocation too ("they can fork the harness and add it")"
type: —
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

Two turns of the 2026-11-01 settings work, on top of a UX research plan
whose stated goal was already "remove the complexity, make it intuitive,
with great flow and coherence, remove the broken flows":

1. After I shipped the ten-gap plan and the mockups, the owner asked
   *"what about models and providers? the experience there is really
   unintuitive, it should be more like zcode, deepseek harness, and less
   like hermes"* — my mockup had faithfully reproduced the shipped
   section: `the tiers` (workload → primary → fallbacks), `the gates`
   (pin, confidence floor, default workload), providers keyed
   `billing · kind`. Every row was one line and each opened a card; the
   order was a failover engine's, not the owner's question.
2. Then *"also budget is not a concern, at least for me, I use
   coding/token plans so this is the wrong thing to worry about"*, and
   after I removed the section but relocated the three caps into
   environment's catch-all: *"again money I don't care, you can remove
   that"*, then *"if anyone cares about that, they can fork the harness
   and add it, to me is a non-issue."* The caps left the GUI entirely;
   the CLI and the document keep them.

**Why:** my gaps were all *interaction* gaps (confirmation, undo,
filter, validation) — none asked whether the surface answers the
owner's actual question first. A settings page organized by the
implementation's concepts (workloads, chains, billing classes, spend
windows) reads as machinery however clean its rows are, and
[[fleet-anti-hermes-constitution]] already named the failure mode: a
harness that became so convoluted to configure it stopped being used.

**How to apply:** to redesign a settings section, start from the
question the owner asks in their own words — "what model am I on, and
how do I change it" — and make the front page that answer with one
press behind it; demote the engine's vocabulary (chains, gates, tiers)
behind a disclosure. Copy the *shape* of a reference the owner names
(ZCode: one list, current marked, switch in place). And take a "not a
concern" ruling literally TWICE over: first DELETE the section rather
than downsizing it, and then do not RELOCATE its rows either — a
second turn that removes a dashboard is asking where the rows went,
and the honest answer is "a fork can add them back"; a surface for a
non-concern is a fork's choice, never a default.

**Follow-on the same day:** the steer went one level deeper than
section order — the owner found no add/edit/reorder/remove flow at all
and generalised the requirement to every chained setting; see
[[a-chained-setting-completes-on-one-surface]]. Related:
[[fleet-settings-abstraction-search]], [[settings-ux-plan-2026-11-01]].

**The chrome rule, named 2026-11-04** (from the icons pass): the owner
wants controls that are *chrome* to be icons only — *"the 'collapse the
rail' + chevron should be just an icon, there is no need for the text as
it diverges from the design system"*. An explanatory label on a control
whose meaning an icon carries is the same machinery-on-the-surface failure
as a dashboard for a non-concern: the words belong in `aria-label`/`title`,
not on the strip. The section rails' fold (chevron alone) is the precedent
the main rail must match. See [[every-rail-icon-audited-2026-11-04]].
