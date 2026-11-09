---
name: a-mislabeled-affordance-reads-as-missing
description: "feedback 2026-10-22 — the owner asked for a way to continue a session from the sessions page and the action already existed under the label \"resume this session here\"; when the owner asks for X, check whether X exists under another name before building, and ship labels in the owner's words"
type: feedback
date: 2026-11-04
basis: confirmed-by
tier: durable
shape: []
phrases: []
missing: []
recall-count: 13
superseded-by:
valid-until:
---

The owner's third dogfooding finding (2026-10-22): "every session should
have a way to navigate to the conversations page with that session
loaded, so that I can keep on working on that session." The capability
already existed and worked — the row's action posted `/api/resume`, which
reseeds the session into the bridge and navigates to the conversation —
but it was labelled **"resume this session here"**, which reads like
starting a run in place, so it read as missing. Relabelled in both
places (the row: "continue" / "open this conversation and keep working on
it"; the replay panel: "continue in the conversation"), shipped a26c78e,
verified live (a follow-up appends to the resumed session's log on disk).

**Why:** a capability labelled in the implementer's vocabulary is
invisible to the person looking for it in their own. The owner lands on
these surfaces first and reports from use, not from source; a mislabel
costs a whole round trip.

**How to apply:** when the owner reports something missing, grep for the
*behavior* they describe (reseed + navigate) before writing any code —
and when you ship an affordance, label it with the sentence the owner
would use. Related:
[[a-top-level-surface-needs-an-address]] (the other direction of the
same law — a capability with no address doesn't exist),
[[fleet-owner-dogfooding-phase]].
