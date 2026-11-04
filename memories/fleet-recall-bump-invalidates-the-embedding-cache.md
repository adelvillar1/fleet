---
name: fleet-recall-bump-invalidates-the-embedding-cache
description: "CLOSED-FIXED KI-228 (2026-10-18) — the recall-count feedback loop invalidated the embedding cache it depended on, because the index's staleness key was file metadata rather than content; the entry now keys on a digest of the indexed text"
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
[[fleet-memory-cache-fix-plan-drafted]] and filed as **KI-228**,
closed-fixed.

**The defect.** `file_stamp` was `secs:nanos:len` of the fact file
(`src/memory.rs:785`), and the recall index reused a cached vector only
when `entry.stamp == fact.stamp`. `rewrite_field` — the one in-place
rewriter, used by the recall-count bump — wrote a temp file and renamed it
over the fact, so the file's mtime changed. **Every recall-count bump
therefore changed the stamp and forced a re-embed of that fact on the next
load.** The corpus's three most-recalled facts carry counts of 103, 103
and 98; each had been bumped ~100 times, and each bump invalidated their
cached vector, so they were re-embedded on essentially every load — and
the embedding round trip is the load's whole cost on a machine with a live
vector path.

The design intent at `src/memory.rs:780-784` is that "a changed fact is
re-embedded, an untouched one keeps its cached vector", where *changed*
means content. The implementation read metadata, so a metadata-only change
was treated as a content change. The test
`the_index_caches_vectors_and_rebuilds_only_what_changed` pinned
content-change → re-embed and stamp-match → no embed, but never exercised
the bump path, so nothing guarded it.

**The fix.** The entry is keyed by a digest of the text the vector was
actually computed from — `calib::hash16(&fact.indexed_text())`, the same
hash the route cache joins on. `recall-count` is deliberately outside
`indexed_text`, so a bump changes nothing; a description, shape or phrase
edit re-embeds exactly that fact. Pinned by
`a_recall_count_bump_re_embeds_nothing`, which bumps a fact through
`recall_hits` and asserts the next load embeds nothing and does not even
rewrite the index.

**Why:** the history channel's feedback loop and the vector channel's
cache were in direct tension — the more a fact is recalled, the more often
it was re-embedded. It was invisible because recall still worked; only the
load-time embed cost and the index rewrite rate moved.

**How to apply:** the stamp stays on `Fact` (the session archive reads it
off its own logs and the two caches must agree on what "changed" means);
only the index's key changed. Related:
[[fleet-memory-corpus-dirties-the-tree]] records the same bump's other
consequence (dirtying the tracked tree), and
[[fleet-memory-cache-deep-dive-findings]] holds the rest of that dive's
findings.
