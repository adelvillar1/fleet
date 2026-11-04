---
name: fleet-memory-cache-deep-dive-findings
description: "The 2026-10-18 memory-and-cache deep dive's cache inventory and smaller findings — all eight defects are now CLOSED-FIXED (KI-228..235); the inventory now includes two new modules, recall_graph and filelock"
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

The 2026-10-18 deep dive of fleet's memory subsystem and its caches (three
parallel sweeps: route/session caches, all other cache sites, and the
docs; plus the core of `src/memory.rs` read directly). The two headline
findings are in [[fleet-recall-bump-invalidates-the-embedding-cache]] and
[[fleet-route-cache-adopts-foreign-verdicts]]. This file holds the rest.
**All eight defects were fixed the same day** — see
[[fleet-memory-cache-fix-plan-drafted]].

**The cache inventory, as it stands after the fixes.** Five real caches:
`memories/.recall-index.json` (fact embeddings, keyed to embedder+dims and
now to a **digest of each fact's indexed text** rather than its file
stamp), `sessions/.recall-index.json` (a deliberately separate second
index so a model swap invalidates one corpus and not both),
`sessions/.recall-counts.json` (the archive's own feedback store, beside
the logs rather than inside them, now written under a shared
`filelock::FileLock`), `<FLEET_HOME>/route-cache.json` (sys1 verdicts
keyed by task hash + lane identity, now with a real `rekey` discard), and
`nomenclature::CACHE` (a process-wide derived vocabulary per project root,
now with `invalidate`). Plus three correct bounded ones: `route::AVAILABLE`
(OnceLock bool, KI-044), `memory::probed_embedder` (OnceLock, KI-064), and
graph's per-call score memo. **There is no HTTP response cache anywhere** —
not in api.rs, not in agent.rs; the only reuse is transport-level
connection pooling, and the only prompt-caching is the provider's own,
which compaction.rs exploits by arranging the summarizer request as a
genuine prefix of the last routed request. **No TTL and no LRU exist
anywhere in the codebase** — every invalidation is digest/stamp-equality,
whole-file discard on a key mismatch, never-invalidate-for-process-life,
or call-scoped.

**The five smaller defects, all closed-fixed.** (1) A fact whose embed
fails was `continue`d in `load_with` with no `fresh` entry, so
`fresh.len() != index.facts.len()` triggered a rewrite — re-attempted,
re-failed, and re-written on every load forever (KI-231; the condition is
now "a new vector landed or a fact disappeared"). (2)
`nomenclature::fresh()` existed to detect that a file changed after
derivation, but its only callers were its own test; the write-time guard
read the never-invalidated cache (KI-232; `invalidate` is now called after
a `WriteFile` lands). (3) The GUI's composer type-ahead re-probed the
embedder per request (KI-233; `probed_embedder()` now, and its documented
degradation returns every candidate at 0.0 rather than an empty list).
(4) KI-062's per-fact `.lock` siblings were never unlinked and were not
gitignored (KI-234; `/memories/*.lock` is now ignored). (5)
`RecallCounts::bump` was a non-atomic read-modify-write across processes
(KI-235; now under a flock).

**Documentation drift, now corrected.** The cwd-walk corpus rule that
KI-087's home-first rule superseded, the route cache's "within a session"
scoping, the per-run probe that KI-064 made per-process, the unmentioned
`PROTECTED_SYNONYMS`, and the corpus's own most-recalled fact describing
the pre-slice-19 four-channel design — all corrected in D1.

**How to apply:** the dive's tests were green (80 memory, 10
session_recall, 5 route_cache; 1065 in the lib), so none of these was a
red-suite item — they were all silent. That is the recurring shape in this
subsystem: the defects are invisible because recall still works. Re-read
the index and route-cache code before assuming the docs describe them.
Related: [[fleet-memories-tests-need-their-own-home]] for the
corpus-resolution rule the docs got wrong.
