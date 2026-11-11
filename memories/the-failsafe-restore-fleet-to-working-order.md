---
name: the-failsafe-restore-fleet-to-working-order
description: "Owner ruling 2026-11-09 — the main session and its delegation chain are the FAILSAFE; any failure turns the chain toward restoring fleet to working order, never refusing work; the ultimate goal is SELF-HEALING and the only unrecoverable failure is no connectivity with no local model"
type: feedback
date: 2026-11-11
basis: inferred
tier: durable
shape: []
phrases: []
missing: []
premises: []
recall-count: 0
superseded-by:
valid-until:
source: zcode:fleet:content:285983102772d99aaed2c496d57acdf6f320aad7347d3891e9bb9e2378839d34
---

Owner ruling 2026-11-09, consolidated across six messages after the
KI-493 incident (two delegated children died at LayerFS workspace open):
**the main session and its delegation chain are the failsafe — in the
presence of a failure their goal is restoring fleet to working order,
never refusing work.** Its parts:

- fleet needs a **diagnostic loop** that knows a service is down and says
  so — not discovery at the moment of use.
- a **critical service down (LayerFS-class)** means the child must NOT
  attempt to route: it runs on the **main session's default model**
  and still does the work.
- **any** refusal, failed call, or spent quota defaults back to the main
  session's routing chain; no bare refusals.
- on a sys1, sem1, layerfs, or any service failure the main session
  **keeps working AND tries to fix what's broken** ("get fleet into
  working order without refusals").
- **the ultimate goal is SELF-HEALING**: there is always a mode where
  the main session can bring fleet back to working order — reverting a
  change that broke it, riding out a model failure. The **only true
  unrecoverable failure is no connectivity with no local model set up**;
  a configured local model is the escape from that floor.

**Why:** the incident's refusals were infrastructure deaths presented as
work-stopping errors; the owner reads that as the system refusing work it
could have degraded into.

**How to apply:** degrade loudly and keep the audit trail (hard rule 5) —
every degradation names the service, the cause, and the repair; repairs go
through the standing approval gate, bounded to one attempt per
down-transition. The plan shaped from this ruling is
[[failsafe-restore-plan-drafted-2026-11-09]]. Related:
[[layerfs-worker-dies-with-dockers-data]], [[open-ki-ledger-deep-dive-2026-11-09]].
