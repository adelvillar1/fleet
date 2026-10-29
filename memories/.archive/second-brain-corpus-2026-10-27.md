---
name: second-brain-corpus-2026-10-27
description: "second-brain's blend1 corpus is 143 items over 20 source files, unchanged by the 2026-10-27T00:45Z scheduled refresh, with the store untouched since Oct 19 and no missing sources"
type: state-sync
date: 2026-10-27
basis: inferred
tier: working
shape: [state-sync]
phrases: [corpus-refresh-second-brain, second-brain blend1 refresh unchanged, tools/call refresh project second-brain, /tmp/blend1-refresh-second-brain-20261027T0045z.py, KI-316 cwd target repo, BLEND1_SEM1_PYTHONPATH sibling wiring]
missing: []
recall-count: 0
superseded-by: consolidation-2026-10-29
valid-until: 2026-10-28
---

The 2026-10-27T00:45Z scheduled run of job corpus-refresh-second-brain spawned the vendored blend1 lane child (python3 -u ~/Projects/fleet/services/blend1/service/mcp_server.py, cwd /Users/alejandrodelvillar/Projects/second-brain per KI-316, BLEND1_PYTHONPATH + BLEND1_{SYS1,SEM1,SDM1,GEN1}_PYTHONPATH per src/lanes.rs sibling_paths, PATH front-loading /usr/local/bin for python3 3.13.6 per KI-324) and called tools/call "refresh" with project "second-brain". Reply: source_files 20 (18 docs/**/*.md + root contract docs FUNCTIONAL-SPECIFICATIONS.md and TECHNICAL-DOCUMENTATION.md; AGENTS.md/README.md/second-brain.md/knowledge.md are NOT in the default set), items 143, dropped_duplicates 0, changed false, action "unchanged", missing_sources [], store ~/.local/share/blend1/vectors/second-brain. Store byte-state verified untouched: manifest.json and vectors.bin mtimes still Oct 19 19:47:19 2026, manifest count 143 = entries 143, model embeddinggemma-2 dims 768. Run driver staged at the run-unique path /tmp/blend1-refresh-second-brain-20261027T0045z.py (shared /tmp names have been overwritten by concurrent refreshes before). Embedding server 127.0.0.1:8901 healthy but unused (steady state = file reads + hashes, no embedding).
