---
name: fleet-route-cache-adopts-foreign-verdicts
description: "CLOSED-FIXED KI-229/KI-230 (2026-10-18) — the route-decision cache promised to discard a foreign lane's cache whole but only suppressed serving it, so the entries got re-labelled under the new lane's key on the next flush; and sys1_judges() early-returned before the cache read"
type: project
date: 2026-11-04
basis: inferred
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

Found in the 2026-10-18 memory/cache deep dive; **fixed the same day** by
[[fleet-memory-cache-fix-plan-drafted]] and filed as **KI-229 and KI-230**,
both closed-fixed.

**The two defects.** (1) The module doc (`src/route_cache.rs:13-19`)
promised "A cache written by another lane … is discarded rather than
served," and `src/route.rs:1457-1459` repeated it. The implementation only
suppressed *serving*: `judge` skipped the `get` when `keyed_for` was
false, but `entries` was never cleared. On the next fresh verdict,
`cache.put(hash, …)` + `cache.flush(new_key)` (`src/route.rs:1489-1497`)
serialized the whole struct — **including the other lane's verdicts —
under the new key**, and from then on they were served as this lane's.
Relocating the sys1 service or swapping its model silently re-labelled
every previously recorded verdict. The test
`a_cache_keyed_to_another_lane_is_discarded_whole` asserted only
`keyed_for` outcomes, never that entries were dropped, so the named
discipline was unenforced.

(2) `src/route.rs:1449-1452` commented "The cache above still answers
first: a recorded verdict is a judgment that was made, on whatever machine
made it," but `if !sys1_judges() { return … }` at `:1453` executed
**before** the cache lookup at `:1460`. On a machine with no resolvable
decision key, cached verdicts were never served at all — which also
contradicted KI-044's own recorded rationale and made the cache a
per-machine artifact rather than a portable one.

**The fix.** `Cache::rekey(key)` clears `entries` when the stored key
belongs to a different producer, called once in `judge` after the spec
resolves. The key itself is left for `flush` to set, so a run that judges
nothing writes nothing and a foreign file survives intact for the lane
that wrote it. The lookup moved above the key check, so the order is:
resolve spec → `rekey` → cache lookup → key check → spawn. Pinned by
`a_foreign_key_clears_the_entries_it_refuses_to_serve` and
`a_cached_verdict_answers_before_the_key_check`.

**Why this mattered beyond the two bugs:** the cache's central promise is
"a model swap must not serve a verdict the new lane never produced"
(docs/plans/2026-10-15-route-decision-cache.md:36-39). A lane
*relocation* is caught by the key; a lane *model change* under an
unchanged launch spec still is not, because `spec.env`/`passthrough` are
absent from the key — the module names that limit itself, so it is
disclosed rather than hidden.

**How to apply:** the keyed-discard discipline is now real in both
caches; if a third cache is added, copy `rekey`, not just `keyed_for`.
Related: [[fleet-memory-cache-deep-dive-findings]] holds the rest of the
dive, including the sibling caches that got the keyed-discard right.
