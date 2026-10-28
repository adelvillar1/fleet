---
name: deepseek-harness-blend1-corpus-state-2026-10-26
description: "deepseek-harness's blend1 corpus is 3279 items over 456 source files, unchanged by the 2026-10-26 catch-up refresh for the missed 2026-10-25T00:55Z window, with all four declared sources present."
type: project
date: 2026-10-26
basis: inferred
tier: durable
shape: [state-sync]
phrases: [mcp__blend1__refresh not native — vendored services/blend1/service/mcp_server.py, BLEND1_PYTHONPATH plus sibling *_PYTHONPATH spawn env, tools/call refresh with project deepseek-harness and explicit sources, action "unchanged", missing_sources [], ~/.local/share/blend1/vectors/deepseek-harness]
missing: []
recall-count: 52
superseded-by:
valid-until: 2026-10-27
---

The deepseek-harness project's blend1 fact corpus is 3279 items over 456 source files (docs/**/*.md resolved to 453 files + CLAUDE.md + TECHNICAL-DOCUMENTATION.md + FUNCTIONAL-SPECIFICATIONS.md; missing_sources []) and was unchanged by the 2026-10-26T00:24Z catch-up refresh for the missed 2026-10-25T00:55Z window: the refresh report said action "unchanged" with changed false, so no embed ran. Store read back on disk: ~/.local/share/blend1/vectors/deepseek-harness/embeddinggemma-2/ (the embedder-keyed path), manifest.json count 3279 / dims 768 / model embeddinggemma-2 / provider llama-server; both files mtime 2026-10-20T20:03:32Z; shasum -a 256 manifest.json 14d8602093513effb674c80b8e276302a53e03bc931ad297f98f548e01c64ac8, vectors.bin 7e502ef51085082429ea34d228bf54bccf1c4ec1f8aff2bd88d49ef04ab2b3cc (hashed in the same turn as the fact, KI-309's rule); zero docs newer than the manifest. Route: the vendored server at services/blend1/service/mcp_server.py speaks NEWLINE-DELIMITED JSON-RPC, not Content-Length framing (KI-318 — framed requests are silently discarded and the driver hangs); the child must run with the TARGET PROJECT's repo as cwd because refresh's default source set (docs/**/*.md + the root contract docs) is cwd-relative (KI-316) — corpus keys are absolute paths since KI-223, so cwd picks the source set, never identities. Spawn env: BLEND1_PYTHONPATH plus the four sibling *_PYTHONPATH vars (BLEND1_SYS1/SEM1/SDM1/GEN1_PYTHONPATH, lanes.rs sibling_paths). mcp__blend1__refresh is not native in this harness.
Retired 2026-10-28: The 2026-10-26 corpus state (3279 items / 456 files) is stale: the 2026-10-28T00:55Z scheduled refresh rebuilt the store to 3320 items / 466 files with new store hashes, verified this session on disk.
