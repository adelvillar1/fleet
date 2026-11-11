---
name: sys1-is-the-abstraction-a-vendor-key-gate-is-not
description: "Owner ruling 2026-11-05 — sys1 was built to abstract the System One models and providers, so a vendor-key list that decides \"can anything judge?\" is a second, stale source of truth; judge through the lane and let its answer decide"
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

The owner's correction while I was chasing a seven-test flake: *"the judge
should be invoked through sys1 and not a direct call to fastino"*, then
*"sys1 was built to abstract the calls to the different system 1 models
and providers."*

The flake's root cause was exactly that violation. `route::sys1_judges()`
gated the route's judgment on `SYS1_DECISION_KEYS` — a hand-copied list of
provider env names (FASTINO, TYPESAFE, DREX, SUPERSONIC, LIQUID, CLM,
OPENJEV, KEV, TOGETHER, LAYA) — and it was already stale: no `TYPELLM`,
`SCALEDOWN`, or `CLEF` (providers sys1 actually ships). Worse, it read the
process-global `FLEET_HOME`, which the integration tests mutate per test:
under load a test's scratch home won the race, the gate answered "sys1
cannot judge", and seven route tests fell back. Every fix that kept the
gate (once-lock keyed by home, `Mutex`, then `RwLock` around the ambient
env) made the suite slower or deadlocked — the owner: *"we need to
address this issue of the suite running slow, this is not the way it
should be."*

**The fix (95dc98c):** the gate is deleted. The judgment is attempted
through the lane and sys1 answers with its verdict or its own named error
— both are the route's fallback signal. The curate judgment drops the
same gate. No env read in the route path, no lock, nothing to serialize:
integration went 166/0 in ~7s, three times under four CPU burners.

**Why:** sys1 exists precisely so Fleet has ONE place that knows which
System One providers exist and how to reach them. A consumer-side copy of
that knowledge drifts silently and turns a capability question into an
environment question.

**How to apply:** never ask a vendor's key whether sys1 can judge — ask
sys1. The cost of the correct shape is one lane spawn the first time on a
machine with no providers configured (~1.2s once, KI-044's measurement),
and that is the price of asking the abstraction instead of guessing at
its internals. See also [[read-the-services-own-config-first]] (the same
service, mischaracterized twice) and
[[a-load-sensitive-test-is-a-defect-in-the-code]] (this flake is its
sixth shape: a consumer-side cache of a provider's availability, read
from mutable process env).

**CLOSED the same day (4820c72, plan 9/9).** The extraction leg was the
last violation — it posted to Fastino's `gliner2.5-multi-v1` for LOCATE
and glide/drex/jev `/v1/systemone` for VERIFY. It is now a lane client
(`sys1_extract` + `sys1_classify` with an inline `task_spec`), and the
owner's ruling reads cleanly against it. The follow-on audit (e86cd4e)
found the one non-product caller left — `scripts/extract-probe.sh` — and
retargeted it at the lane; every other decision surface already went
through sys1. See [[extraction-leg-through-sys1-2026-11-05]] for what
shipped, the four defects the live walk found, and the cleared
near-misses (language-model posts, blend1's library calls, sem1's
loopback embedder).
