---
name: mahjong-vtt-corpus-unchanged-oct-25
description: "mahjong-vtt's blend1 corpus is 618 items over 73 source files and was unchanged by the 2026-10-25T01:05Z catch-up refresh, with the embeddinggemma-2 store untouched since Oct 24 00:19Z and no source added or removed."
type: project
date: 2026-10-26
basis: inferred
tier: durable
shape: [state-sync]
phrases: [blend1 refresh mahjong-vtt, spawn route: blend1 lane child over stdio JSON-RPC, BLEND1_SEM1_PYTHONPATH, sources as absolute globs: mahjong-vtt/docs/**/*.md, action: unchanged, manifest.json entries 618]
missing: []
recall-count: 0
superseded-by:
valid-until:
---

The 2026-10-25T01:05Z scheduled window for job `corpus-refresh-mahjong-vtt` was missed while fleet was down and was caught up by hand.

Route: the vendored blend1 lane child (`services/blend1/service/mcp_server.py`, cwd `services/blend1`, `mcp::BASE_ENV` + `BLEND1_PYTHONPATH` + `BLEND1_{SYS1,SEM1,SDM1,GEN1}_PYTHONPATH`) driven over stdio JSON-RPC — initialize (2024-11-05), notifications/initialized, tools/call refresh. The lane tool `mcp__blend1__refresh` is absent from a driver session's toolset (KI-221/KI-224), so the spawn route is the working one.

Result: `{"project": "mahjong-vtt", "source_files": 73, "items": 618, "dropped_duplicates": 2, "changed": false, "action": "unchanged", "store": "~/.local/share/blend1/vectors/mahjong-vtt", "missing_sources": []}`.

The sources passed were the project's own default docs set as absolute globs — `<project>/docs/**/*.md` plus `CLAUDE.md`, `TECHNICAL-DOCUMENTATION.md`, `FUNCTIONAL-SPECIFICATIONS.md` — because the child's cwd is the lane's home, where the cwd-relative default set resolves to nothing and the lane answers `action: "no-source"` (KI-316, reproduced for this project in the same run). A pre-run comparison of the live store's absolute-path keys against the repo's current docs found 0 files added and 0 removed; the manifest (618 entries, 768 dims, embeddinggemma-2) still carries mtime Oct 24 00:19:28, so `unchanged` is corroborated by the store, not just reported.

The embedder behind the refresh leg was live: llama-server embeddinggemma-2 BF16 on 127.0.0.1:8901, `/health` → `{"status":"ok"}` — no embedding call was needed because the hash set matched.
