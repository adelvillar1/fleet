---
name: beta-shakeout-pass-two-2026-10-31
description: "the shakeout's pass two (2026-10-31): the whole manifest walked deep - 52 rows held, KI-380 closed at all three legs, KI-381 (fleet onboard unreachable, the missing needs_runtime arm) fixed and pinned; B1-B4 ticked, B5 waits on pass three after the owner restarts the face"
type: episodic
date: 2026-10-31
basis: inferred
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

Pass two of the beta shakeout (2026-10-31) walked groups A-H and HH with zero model spend: the spend rows ran on a keyless scratch face, so every failure was a named absence. KI-380's fix: the conn-down banner, bounded 5s read retries, the tab-title seat cue from any seat path, the heartbeat silence watch (15s gate on the 10s beat), and the always-is-run-scoped pin. KI-381: Onboard was missing from needs_runtime in main.rs - every fleet onboard fell through to the sync dispatch's internal error; the e2e pin drives the real binary twice. Isolation lesson: a scratch home with an empty memories/ scaffold adopts the repo corpus by the KI-087 rule (the walk checks the home first but only when it HOLDS facts), so probes land in the repo tree - the honest-empty probes need a foreign cwd. B5 and pass three wait on the owner's restart of the live face.
