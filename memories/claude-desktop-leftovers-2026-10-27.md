---
name: claude-desktop-leftovers-2026-10-27
description: "2026-10-27 — the owner said Claude Desktop is NOT on this machine ("haven't used it in many, many months") yet it had left 10 GB behind; verified absent (no bundle, no process, no keychain), the whole Application Support/Claude moved to Trash and the prefs plist deleted; fleet's mcp.json source.harness "claude-desktop" means a scanned CONFIG FILE, not the app"
type: project
date: 2026-11-05
basis: confirmed-by
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

**Owner, 2026-10-27: "I no longer have Claude Desktop on this machine. I
haven't used it in many, many months."** I had just edited its config
file during the vizro sweep — the file's existence had read to me as an
installed app, and the owner corrected that reading.

**Verified absent before touching anything:** no `Claude.app` in
`/Applications` or `~/Applications`, `mdfind` on its bundle id
(`com.anthropic.claudefordesktop`) empty, no running process, no
keychain entries matching anthropic/claude.

**What it had left behind — 10 GB in one directory plus a plist:**
- `~/Library/Application Support/Claude` = **10 GB**: `vm_bundles` 8.0 G,
  `Cache` 1.2 G, `Code Cache` 269 M, `claude-code-vm` 223 M,
  `claude-code` 196 M, `local-agent-mode-sessions` 119 M, extension
  caches ~190 M. All app runtime state — inspected (du by subdirectory)
  before deleting; nothing of the owner's in it.
- `~/Library/Preferences/com.anthropic.claudefordesktop.plist` (4 K) —
  deleted outright.
- Caches / HTTPStorages / WebKit: no anthropic or claude entries.

**The move:** the whole directory went to
`~/.Trash/Claude-Desktop-leftovers-20261027` (a move, not `rm -rf` —
recoverable if anything was wrong), and I said out loud that **the 10 GB
only frees when the Trash is emptied** (I don't empty it: it would take
whatever else sits in there). The earlier vizro/legacy backups inside
that directory (`.bak-vizro`, `.bak-legacy`) went with it — harmless,
fleet's own `~/.fleet/mcp.json` keeps its copies.

**The correction worth carrying: fleet's registry `source.harness:
"claude-desktop"` means the SCAN read a config FILE at
`~/Library/Application Support/Claude/claude_desktop_config.json`, not
that the app exists or runs.** A harness can import from a config whose
app was uninstalled months earlier — check the app bundle, not the
config path, when something claims to be "from Claude Desktop". Same
for `~/.docker/mcp/*` and any `source` field in fleet's registry: the
source is a file location, not proof of a live app.

Related: [[mcp-vizro-removed-2026-10-27]] (the sweep that led here),
[[mcp-registry-live-import-state]] (the registry's state; its addenda
supersede the older "left alone" lines).
