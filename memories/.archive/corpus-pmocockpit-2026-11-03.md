---
name: corpus-pmocockpit-2026-11-03
description: "pmocockpit's blend1 corpus stands at 72 items over 5 source files, unchanged by the 2026-11-03T01:10Z window's catch-up refresh, store byte-identical to the 2026-10-19 build (manifest fdfeb38f, vectors.bin 402c442d), and the report's missing_sources is empty for the first time as both zero-hit globs moved to the new empty_wildcards."
type: project
date: 2026-11-03
basis: inferred
tier: durable
shape: [state-sync, resolved]
phrases: [fleet mcp call blend1 refresh, corpus-refresh-pmocockpit, action unchanged, empty_wildcards, sources_added sources_removed, missing_sources empty]
missing: []
recall-count: 0
superseded-by:
valid-until: 2026-11-02
---

The 2026-11-03T01:10Z `corpus-refresh-pmocockpit` window was served by the daemon's dispatch (claim file stamped 01:10Z, last_run 2026-11-03T01:10Z) and the run itself landed in this catch-up at 2026-11-03T01:12Z through the KI-369 spawn route — one `fleet mcp call blend1 refresh` command against the job's own seeded arguments, exit 0, no hand-written stdio driver. Reply, verbatim:

{"project": "pmocockpit", "source_files": 5, "items": 72, "dropped_duplicates": 0, "changed": false, "action": "unchanged", "store": "/Users/alejandrodelvillar/.local/share/blend1/vectors/pmocockpit", "missing_sources": [], "empty_wildcards": ["/Users/alejandrodelvillar/Projects/pmocockpit/*.md", "/Users/alejandrodelvillar/.fleet/canvas/digests/pmocockpit/*.md"], "sources_added": [], "sources_removed": []}

Verified rather than taken on the report: the store's model-keyed files at `…/vectors/pmocockpit/embeddinggemma-2/manifest.json` and `vectors.bin` (the reply's path resolves one model subdir deeper) carry mtime 2026-10-19T09:33:23Z and sizes 72890 / 221184, sha256 fdfeb38f6c617eb3a75371f062528da36a535eadfb4afddd5a651ac7ba0998db / 402c442d815713aa4510fa98aa6138a70d8d2284db5aa79928b08be15ce52c46 — byte-identical to the 2026-11-02 fact's recorded values, with mtimes predating this run entirely, so nothing has written to the store since the 2026-10-19 build; the manifest parses to 72 entries / 768 dims / embeddinggemma-2 / llama-server. The five sources are unchanged: four docs/plans files (two dated 2026-08-26, two 2026-08-30) and docs/recaps/SESSION-RECAP-2026-08-30.md, all last modified 2026-08-26/30, and the pmocockpit repo root holds no .md at all.

`missing_sources` is now EMPTY for the first time on this job, and both zero-hit globs moved to the new `empty_wildcards` — KI-343's split, first exercised on this project: `/Users/alejandrodelvillar/Projects/pmocockpit/*.md` is KI-343's standing root-glob entry (no .md at the root to match) and `/Users/alejandrodelvillar/.fleet/canvas/digests/pmocockpit/*.md` is the bound-board digest source, still structurally absent (no `digests/` directory anywhere under ~/.fleet/canvas; canvas holds board-1 and sample-board only) — the same KI-393 shape fleet's own window recorded 2026-11-02T00:10Z. `sources_added`/`sources_removed` both empty, confirming the source set equals the stored manifest's five files.

Ledger state at fire: the cron row carries last_run 2026-11-03T01:10Z with last_outcome null and last_success 2026-11-02T01:10Z, runs == successes == 7 — the null is the running daemon's KI-363 remedy (mark_dispatched clears the outcome at dispatch; the daemon is the master worktree's release binary, ~/Projects/fleet-master-wt/target/release/fleet per the com.fleet.gui.plist, while this checkout predates that fix). The claim file `.fleet/cron/claims/corpus-refresh-pmocockpit-2026-11-03T01:10Z` is stamped 01:10Z, so the window was dispatched, not missed; this catch-up's end record fills the row's outcome. This is the second pmocockpit corpus fact in the KI-386 canonical shape (corpus at the head, dashed date at the tail, check-ignored by .gitignore:37).
Retired 2026-11-03: The first write of this fact mis-framed the run as a catch-up after a missed window; the session log for the 01:10Z window (20261103T011052Z-32796862, kind: scheduled) is this run's own, so the framing is corrected to the daemon-dispatched scheduled window and the phrases re-stamped.
