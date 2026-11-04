---
name: quota-is-the-one-unlockable-commodity-failure
description: "Owner review 2026-10-31 — the router's committed candidate must hand the walk to the next provider on quota; quota is the one class allowed to clear the commit"
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

Owner review, 2026-10-31: *"the router is not switching providers
automatically from the fallback chain if there is one reporting that
quota is exhausted."* The committed candidate failed every error
fatally — `chat_stream`'s committed branch returned on any failure
without consulting `failover_eligible`, so a provider that went out of
money after one good turn failed the run typed, every turn.

**Why:** quota is a **persistent state whose recovery is a different
provider**, which is the whole reason the order has a tail (the deepseek
failover). Every other committed failure — auth, server, timeout — is
transient or fatal-by-design, and a mid-stream failure is fatal whatever
the class (a partial answer cannot be resumed).

**How to apply:** LANDED as `8b3564b` (pushed) — the committed path
unlocks on `quota` only: the attempt is recorded, `committed` clears,
and the walk starts at `position + 1` (the exhausted candidate is not
retried in the same walk). The decision row reads as two attempts.
Pinned by `a_committed_quota_failure_unlocks_the_walk`; the existing
committed-failure test keeps the fatal path green. The integration
test's seam is `Router::commit_for_tests` (driving committed state live
needs a second turn, which needs a tool call, which needs a lane — one
seam instead of a fixture maze); the redundant second attempt record I
first wrote was removed — one provider never appears twice in one walk.
New failure classes: check whether the recovery is a *different
provider* before deciding it is fatal on commit — quota is the shape,
not the only instance (a provider-keyed outage would be the next).
Shipped in the same commit as [[fold-count-is-the-freshness-measure]].
