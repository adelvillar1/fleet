---
name: owner-stated-facts-are-authoritative
description: "When the owner states something as fact, act on it — don't spend turns verifying it; the failure mode is re-checking a stated fact instead of applying the design ruling behind it"
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

When the owner states a fact, treat it as settled and move to the consequence.
The 2026-10-16 router session cost the owner's patience over exactly this:
they said port 8300 was the zcode router, I spent turns confirming it against
the machine anyway, and they had to say "you don't need to check what I'm
telling you as a fact because it is."

**Why:** the verification was never the missing piece. The owner's stated fact
was correct and actionable on its own; what was actually missing was the design
it implied — Fleet routing through a router Fleet owns — and every turn spent
re-confirming the fact was a turn not spent applying it. Worse, the checking
read as ignoring the request, which is why it escalated ("for fucks sake, when
we started this harness I asked that fleet needed to be a self-contained app").
The same rule covers stated design intent: "the precursor is not the solution"
was a design ruling, not a deployment detail to investigate.

**How to apply:** write the stated fact down and go straight to what follows
from it. Verify the *implementation* against the fact — grep the source, read
the config, run the command — not the fact itself. If a stated fact turns out
wrong, that surfaces on its own during the work and costs one correction,
which is far cheaper than turn-by-turn checking. A confirmation request
("want me to check that?") is still asking; just do the work the fact implies.

Related: [[fleet-routes-through-its-own-router]].
