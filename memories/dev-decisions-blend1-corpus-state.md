---
name: dev-decisions-blend1-corpus-state
description: "dev-decisions's blend1 corpus is 79 items over 10 source files (dev-decisions/docs/**/*.md — the repo holds no root contract docs), unchanged by the 2026-10-27T00:50Z scheduled refresh, with all sources present."
type: project
date: 2026-10-27
basis: inferred
tier: durable
shape: [state-sync]
phrases: [corpus-refresh-dev-decisions, dev-decisions/docs/**/*.md, no-source, unchanged, BLEND1_SEM1_PYTHONPATH, services/blend1/service/mcp_server.py]
missing: []
recall-count: 0
superseded-by:
valid-until:
---

The dev-decisions project's blend1 fact corpus measured 2026-10-27T00:50Z (scheduled job corpus-refresh-dev-decisions): 10 source files → 79 items, dropped_duplicates 0, changed=false, action=unchanged, missing_sources=[], store ~/.local/share/blend1/vectors/dev-decisions (manifest.json + vectors.bin mtime Oct 20 19:55:17 2026, 79 entries / 768 dims / embeddinggemma-2). Corroborated filesystem-side: no doc under dev-decisions/docs is newer than the store build. The store's own keys were the source-set authority (KI-316 route): the project's ten keys are all under docs/ (linkedin-post.md, 4 plans, 5 recaps) and the repo root has no CLAUDE.md/TECHNICAL-DOCUMENTATION.md/FUNCTIONAL-SPECIFICATIONS.md, so the single absolute glob dev-decisions/docs/**/*.md reproduces the key set exactly. Spawn spec that worked: cwd services/blend1, PATH front-loading /usr/local/bin (python 3.13.6, KI-324), BLEND1_PYTHONPATH=services/blend1/src plus BLEND1_{SYS1,SEM1,SDM1,GEN1}_PYTHONPATH=services/<svc>/src (the refresh leg imports sem1 at call time), initialize + notifications/initialized + tools/call refresh over newline-delimited JSON (KI-318), 180 s reply deadline (KI-325); embedder probed healthy first (POST /v1/embeddings → 200 in 0.85 s).
