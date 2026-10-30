---
name: fleet-memories-substrate-is-one-markdown-file-per-fact
description: "Fleet's durable memory is the git-tracked memories/ tree: one markdown file per fact with frontmatter name, description, type, date, basis, tier, shape, phrases, missing, recall-count, superseded-by, and valid-until — the fork's brain substrate, so a bad fact is a revert and slice 12's gated self-modification has a structurally reversible substrate."
type: reference
date: 2026-10-13
basis: confirmed-by
tier: durable
shape: []
phrases: ["the shape partitions the pool and the words rank within it", "direction outranks proximity", "the recall index is keyed by a digest of the indexed text", "one bounded hop over the frontmatter's own edges"]
missing: []
recall-count: 336
superseded-by:
valid-until:
---

The description is the one-sentence fact the index embeds and an assembled turn carries; the body is the fact's full content.

Retrieval is a hybrid rank over six named channels — vector 0.15, keyword 0.15, trajectory 0.30 (the recorded phrases, the largest single channel), history 0.25, freshness 0.10, provenance 0.05 — where the shape PARTITIONS the pool and the words rank within it, so no term can move a fact across the partition. Direction (trajectory + history + provenance, 0.60) outranks proximity (vector + keyword, 0.30): a fact whose whole case is the neighborhood tops out at 0.45. Three signs are derived from the tags — `resolved:`, `diverged:` (an anti-vector to avoid, not a candidate), and `incomplete:` (drove partway and stalled, ranking exactly as a resolution with its missing companions named). Off-shape facts enter only through the spillover's three named routes and are marked `off-shape` in their line.

The recall index (`memories/.recall-index.json`, gitignored like `__pycache__`) is keyed to the embedder AND to a digest of each fact's indexed text, so a recall-count bump — which rewrites the file in place and moves its mtime — does not invalidate the vector (KI-228). One bounded hop over the frontmatter's own edges (`superseded-by`, `missing`) surfaces a superseded fact's replacement and an incomplete fact's named companions, each marked `via: <edge>`.

The write path, the consolidation cycle, and the turn-path wiring are slice 8 part 2. The recall-count bump runs under a per-fact flock (KI-062); the corpus resolves home-first (KI-087); the probe's answer is cached per process (KI-064).

Source: docs/features/memory.md and the fork's packages/fleet/dsh-brain, ported 2026-10-13; updated 2026-10-18 for the slice-19 channels, the digest-keyed index, and the recall graph.
