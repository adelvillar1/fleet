---
name: corpus-dev-decisions-2026-11-01
description: "dev-decisions' blend1 corpus is 142 items over 12 sources, unchanged by the 2026-11-01T00:50Z scheduled refresh, the store byte-identical to the Oct 28 build"
type: project
date: 2026-11-01
basis: inferred
tier: durable
shape: [state-sync, resolved]
phrases: [corpus-refresh-dev-decisions, tools/call refresh, BLEND1_PYTHONPATH, action unchanged, dev-decisions docs/**/*.md, services/blend1/service/mcp_server.py stdio]
missing: []
recall-count: 0
superseded-by:
valid-until:
---

The 2026-11-01T00:50Z scheduled run of job corpus-refresh-dev-decisions drove the established spawn route: a unique temp driver (/tmp/blend1-refresh-devdec-20261101-0050-j7k2.py — write_file accepted the numeric-suffix filename this run; the Oct-31 run's write_file refusal of a driver file did not recur) spawned python3 -u services/blend1/service/mcp_server.py (cwd services/blend1) with the src/lanes.rs launch spec — BLEND1_PYTHONPATH=services/blend1/src plus BLEND1_{SYS1,SEM1,SDM1,GEN1}_PYTHONPATH=services/<svc>/src — and drove newline-delimited stdio JSON-RPC (protocolVersion 2024-11-05, initialize -> notifications/initialized -> tools/call refresh), skipping id-1 notifications when reading, per src/mcp.rs framing.

Report: {project dev-decisions, source_files 12, items 142, dropped_duplicates 0, changed false, action unchanged, store ~/.local/share/blend1/vectors/dev-decisions, missing_sources []} against explicit absolute globs (~/Projects/dev-decisions/docs/**/*.md plus *.md at the root). Store verified byte-identical at two levels: manifest.json + vectors.bin sha256s (manifest 490ed5d79956fd5f…, vectors f09250f7fc7c5ac2…, both mtime 2026-10-28 12:38CDT, unchanged through the run) match the values the Oct-28 rebuild fact recorded; manifest count 142 = report items, 12 distinct docs in entry keys = source_files. Steady state reads files + hashes only — no embedding call, the 8901 embedder unused.

Concurrent-server finding: a long-lived services/blend1/service/mcp_server.py (PID 73239, parent the fleet gui daemon PID 57212, `fleet gui --remote --relay`) was already running — the GUI session's interactive lane server, not a corpus writer; it idles on stdio absent a refresh call, and corpus._rebuild() is atomic tmp+rename anyway, so any interleave is safe. Preceded by the fleet GUI daemon at 18:59 local — the 00:50Z window fell in the 19:00 CDT hour.

jobs.json record_run convention (persisted from the superseded Oct-23 fact): the corpus-refresh-dev-decisions entry's last_run is stamped with the minute the run was FOR, recorded under ~/.fleet/cron/jobs.lock with a temp-file rename.
