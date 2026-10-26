---
name: ark-corpus-338-rebuilt-2026-10-24
description: "agnostic-router-kit's blend1 corpus is 338 items over 45 sources after the 2026-10-24T00:40Z refresh, rebuilt (not changed) by KI-297's duplicate-key compare defect, store shas carried in-turn"
type: project
date: 2026-10-24
basis: inferred
tier: durable
shape: [state-sync, drops-the-row, incomplete]
phrases: [BLEND1_SEM1_PYTHONPATH, services/blend1/service/mcp_server.py, refresh_corpus, agnostic-router-kit, KI-297, KI-310]
missing: [KI-297's derive-time dedupe fix in services/blend1/src/blend1/corpus.py — until it lands, every window rebuilds identical text and reports changed]
recall-count: 0
superseded-by:
valid-until: 2026-10-25
---

agnostic-router-kit's blend1 corpus is 338 items over 45 source files after the 2026-10-24T00:40Z scheduled refresh (first run of this job): report {source_files: 45, items: 345, changed: true, action: rebuilt, missing_sources: []}, the 345-vs-338 delta being KI-297's seven duplicate-key pairs (the root instructions file's two "## Today's state" sections + five repeated headings in docs/recaps/SESSION-RECAP-2026-10-06.md), so the hash gate reports changed forever and identical text is re-embedded every window. Store: ~/.local/share/blend1/vectors/agnostic-router-kit/embeddinggemma-2/ (manifest.json 455475 B, vectors.bin 1038336 B = exactly 338×768×4), shas read in the same turn: manifest d58ae85fb691a11a10fc98855e43adff823cab69520d79a0d607be768f305aa3, vectors.bin 3e6d1f3807d993544424e33f12526affb0cbca92598c354cf591f97abb6d50e4, both mtimes Oct 23 19:44:07. Composition: 32 entries over the three root contract docs (CLAUDE.md 6, TECHNICAL-DOCUMENTATION.md 15, FUNCTIONAL-SPECIFICATIONS.md 11) + 306 over 42 docs-tree files; zero entries under the untracked refresh-* dirs. Nothing content-changed versus the Oct 22 19:57 build — the rebuild was triggered by KI-297's compare-shape defect, not by doc edits. Route used: the lane tool mcp__blend1__refresh stays unavailable natively; drove the vendored server at ~/Projects/fleet/services/blend1/service/mcp_server.py over newline-delimited JSON-RPC with the complete launch-spec wiring — BLEND1_PYTHONPATH plus all four BLEND1_<SVC>_PYTHONPATH (sys1/sem1/sdm1/gen1) and cwd = the lane home; a first call with only BLEND1_PYTHONPATH failed with typed -32603 "ModuleNotFoundError: No module named 'sem1'", logged as KI-310. Embedder live at 127.0.0.1:8901 throughout.
Retired 2026-10-26: This fact's whole mechanism claim is retired: it says the hash gate "reports changed forever" and that KI-297's derive-time dedupe was missing, but the dedupe landed (fleet ebaeeb4, 2026-10-24) and the 2026-10-25T00:40Z catch-up this turn reported changed=false, action=unchanged, items 338, dropped_duplicates 7. Everything in it that is still true — the 338/45 corpus, the store path and shas, the 32-over-root-docs + 306-over-docs-tree composition, and the spawn route — is carried verbatim in the superseding…
