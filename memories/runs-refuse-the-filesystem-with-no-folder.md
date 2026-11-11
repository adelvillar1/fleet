---
name: runs-refuse-the-filesystem-with-no-folder
description: "A run's filesystem grant is stored per chat (and a \"\" fleet default); the absent path is silent and the run bar paints the attached session's folder, so a chat can refuse every fs tool while the bar shows a folder — the grant is set, the silence is KI-492"
type: project
date: 2026-11-11
basis: inferred
tier: durable
shape: []
phrases: []
missing: []
premises: []
recall-count: 0
superseded-by:
valid-until:
source: zcode:fleet:content:285983102772d99aaed2c496d57acdf6f320aad7347d3891e9bb9e2378839d34
---

Triage 2026-11-09 of the owner's report that a chat refused `bash`, `read_file`,
`glob` and `grep` with *"this chat has no working folder, so `<tool>` cannot
touch the filesystem"* — and that the delegated children died at workspace open.

**The gate is real and path-blind.** `run_folder(session, asked)` reads the
chat's own choice from `<FLEET_HOME>/sessions-workdirs.json`, else the fleet
default under the **empty-string key**, else none. None = no filesystem at all.
The failed chat (`20261109T143803Z-20e7f621`) had no entry and there was no
default at the time — so every fs tool refused, and the `start` event still
carried `workspace: /Users/alejandrodelvillar/Projects/fleet`, which is why it
looked like a folder existed.

**Two display/silence defects (KI-492, still open):** the absent-folder arm
returns `Ok(None)` **without emitting a folder event**, so the transcript never
says why; and the run bar paints the *attached session's* `effective_folder`,
so a previous chat's folder can sit on screen while a new chat has none.

**How to apply:** to grant access without the UI, POST the face's own route —
`curl -X POST localhost:3080/api/folder -d '{"folder":"<repo>"}'` — which writes
the event, persists `"": "<repo>"` in `sessions-workdirs.json`, and makes every
new chat inherit it (`/api/state` then answers `folder: {name, path}`). Do not
hand-edit the sidecar. Before diagnosing "tools refused", check `/api/state`'s
`folder` — the bar is not the truth. Same session also logged two dead MCP
lanes (`computer-use` entry carries no `command`; `fli` dies on a fastmcp
import) — KI-494. Related: [[layerfs-worker-dies-with-dockers-data]].
