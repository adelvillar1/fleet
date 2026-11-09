---
name: layerfs-worker-down-blocks-delegation-and-its-own-triage
description: "When the LayerFS worker is down, subagent delegation fails with `layerfs refused ... ECONNREFUSED 127.0.0.1:49473` at 0 turns, and the child's session log that would explain it is unreadable from a chat with no working folder — so the failure blocks the shell that could diagnose it."
type: project
date: 2026-11-09
basis: inferred
tier: durable
shape: [state-sync, foreign-root, incomplete]
phrases: [LayerFS worker did not answer, connect ECONNREFUSED 127.0.0.1:49473, "layerfs refused [Failed]", the call was not sent, subagent failed ... 0 turns, the log holds what it did]
missing: ["read the child's ~/.fleet/sessions/<id>.jsonl log, which sits outside the MCP filesystem server's Desktop/Downloads allowlist", probe port 49473 directly — no generic HTTP/TCP fetch tool exists in the deferred catalog and browse is disabled, determine whether the 49473 port is persisted state (stale dial) or the worker's live bind port, read fleet's layerfs spawn/exit logs under ~/.fleet/logs]
premises: []
recall-count: 0
superseded-by:
valid-until:
source:
---

Triage of the 2026-11-09 layerfs failure, as far as the tools reached.

EVIDENCE (verified in session):
- `subagent` failed twice, ~17m33s apart (child session ids 20261109T144208Z-e6452e31 and 20261109T145941Z-b1cb8a38), both times with the identical port: "the delegated workspace could not open: layerfs refused [Failed]: the LayerFS worker did not answer (connect ECONNREFUSED 127.0.0.1:49473); the call was not sent", 0 turns, no partial answer.
- Same port on both attempts means this is not a race on a freshly-assigned port. Either the worker never started, or the client is dialling a persisted port nothing binds.
- ECONNREFUSED (not a timeout) means the TCP connect was actively rejected — the listener is absent, not hung.

WHY THE TRIAGE STALLED (the compound failure, and the reason this fact is tagged incomplete):
- The chat had no working folder, so `bash`/`read_file`/`list_dir` are refused by name — the shell that would answer every question above.
- The diagnostic artifact that survives a failed delegation is the child's log at /Users/alejandrodelvillar/.fleet/sessions/<id>.jsonl. `read_document` needs the chat's folder; the filesystem MCP server is allowlisted to ~/Desktop and ~/Downloads only. So the log is on disk and unreachable.
- `browse` and `use_skill` were disabled ("no use over the measured window"), and `tool_search` over 35 deferred tools surfaced no generic HTTP/TCP or shell tool.

THE SHAPE WORTH REMEMBERING: a dead workspace worker is not just "delegation is unavailable". It removes the only path that could run the diagnostic commands, while the diagnostic log it wrote sits outside every readable root. Setting a working folder is the single grant that breaks the deadlock — it restores bash, which restores `fleet tools enable` and the log read together.

STILL OPEN: the cause itself. Unconfirmed hypotheses, cheapest first: (a) the layerfs binary is missing or failed to spawn; (b) a persisted port in FLEET_HOME state is stale after a restart; (c) the worker binds a different port than the one the client dials. First commands when a shell exists: `lsof -nP -iTCP:49473 -sTCP:LISTEN`, `ps aux | grep -i layerfs`, `grep -rn 49473 ~/.fleet --include='*.json'`, then tail the newest file in ~/.fleet/logs.
