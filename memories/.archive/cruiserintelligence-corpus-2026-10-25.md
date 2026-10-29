---
name: cruiserintelligence-corpus-2026-10-25
description: "cruiserintelligence's blend1 corpus stayed at 40 source files / 334 items through the 2026-10-25T01:15Z catch-up refresh, which reported changed: false, action: unchanged."
type: project
date: 2026-10-26
basis: inferred
tier: durable
shape: [state-sync, resolved]
phrases: [cruiserintelligence, 40 source files, 334 items, action: unchanged, catch-up for the missed 2026-10-25T01:15Z window, embeddinggemma-2 store]
missing: []
recall-count: 41
superseded-by:
valid-until: 2026-10-28
---

The 2026-10-25T01:15Z window for `corpus-refresh-cruiserintelligence` was missed while fleet was down; the catch-up ran 2026-10-26 and the corpus is unchanged.

The refresh report: `{"project": "cruiserintelligence", "source_files": 40, "items": 334, "dropped_duplicates": 0, "changed": false, "action": "unchanged", "store": "/Users/alejandrodelvillar/.local/share/blend1/vectors/cruiserintelligence", "missing_sources": []}`.

Corroboration that this is a genuine steady state and not a silent no-op: the store's own manifest (`~/.local/share/blend1/vectors/cruiserintelligence/embeddinggemma-2/manifest.json`) carries exactly 334 entries over exactly 40 distinct source keys, both files keep their Oct 19 09:32 mtimes (no re-embed), and a live `search` through the same child returned real cosine hits (0.7758 / 0.7747 on "materialized view refresh pipeline").

The source set is the repo's 37 `docs/**/*.md` plus the three root contract docs `CLAUDE.md`, `TECHNICAL-DOCUMENTATION.md`, `FUNCTIONAL-SPECIFICATIONS.md` — enumerated from the manifest's absolute keys, and matched exactly by what is on disk now (no docs added or removed since the Oct 19 build).

The store is keyed by absolute path since the KI-223 fix, so the cwd the child launches from no longer perturbs the key set and `unchanged` is comparable run to run.
Retired 2026-10-29: Stale store state: the 2026-10-29T01:15Z scheduled refresh rebuilt the corpus to 41 sources / 338 items (immediate second call answered unchanged at the same counts), and the delta — +1 file / +4 items, root README.md admitted by the Oct-28 widened root glob, KI-348's third project — is recorded in the successor fact cruiserintelligence-blend1-corpus-2026-10-29 and in docs/known-issues.md KI-348's row.
