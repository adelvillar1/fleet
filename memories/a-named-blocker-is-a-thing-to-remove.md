---
name: a-named-blocker-is-a-thing-to-remove
description: "feedback 2026-10-26 — when I name a blocker or diagnose a defect, the owner expects it removed, not documented; "why do we have to wait? you should be able to trigger it" and "why are you fighting something that's clearly wrong"
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

Two corrections inside one hour on 2026-10-26, same shape:

- KI-249's measurement "waits for the first live run" — I explained the
  dependency chain and offered to kickstart the GUI. Owner: **"why do we
  have to wait? you should be able to trigger it."** The row was recorded
  minutes later (GUI restart's catch-up plus a by-hand CLI run).
- The github server blocking every headless run — I diagnosed it fully
  (imported ballast from claude-desktop, actively harmful, the class fix
  vs the instance fix) and closed with "your call on which, or both."
  Owner: **"then disable it, why are you fighting something that's
  clearly wrong."**

**Why:** a thorough diagnosis that ends in a menu reads as hedging. When
the owner asks "why is X like this", they have usually already decided
what should happen; presenting options asks them to do the concluding
step I exist for. It also burns a full turn per round trip.

**How to apply:** finish the thought. If one branch is obviously right,
take it and report the action; mention the alternative in one sentence as
"the class fix remains open", not as a question. When I name a blocker,
the next thing I say is how I removed it — or, if it genuinely needs the
owner, the single specific ask (a key, a ruling), never a survey.

**Confirmed the same evening.** Taking the instance fix and naming the
class fix in exactly one sentence got the arc the guidance predicts:
the owner asked "how do we fix that issue?", the design answer
followed, then "create a plan for this fix" — and the plan was written,
gate-passed, built, and committed within the hour. The sequence is
instance fix → one-sentence class note → owner asks how → design →
plan → build, with no survey at any step.

**The mirror form, same day: green is not done.** After two LayerFS
plans went completed with live-verified gates, I reported the state as
essentially finished — and the owner answered **"that means that we are
far from done here, this is not a seamless experience."** The plans
measured the CAPABILITY; the seven-export launch ceremony, the six
facts doctor never checks, the `/tmp` checkout that macOS purges, the
scratch-project knob, and the unverified reboot path were all still on
the owner. A completed plan with a green gate certifies what was built,
never what it costs to USE — before calling anything done, inventory
the setup the user must perform by hand and say those costs out loud
first (see [[layerfs-expert-study-2026-10-25]] for the full seam list
and the productization increment it became).
Related: [[owner-stated-facts-are-authoritative]],
[[react-to-cancelled-foreground-calls]], [[execute-don-t-clarify]],
[[fleet-anti-hermes-constitution]].
