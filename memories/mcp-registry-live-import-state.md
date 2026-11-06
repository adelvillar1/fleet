---
name: mcp-registry-live-import-state
description: "2026-10-23 evening — the owner said \"import them\": all 13 available scanned servers are in the LIVE ~/.fleet/mcp.json; 6 probe ok (49 tools), 7 failed with named reasons (4 env-unresolved via the vault, computer-use has no cwd field, fli crashes, vizro times out); dappier's URL-embedded key is the one registry residual; 2026-10-26 — the imported servers are ballast fleet never uses, and github's unset token refuses every headless run"
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

State of the live MCP registry on this machine, 2026-10-23 evening,
right after the owner's "import them". Runtime state — re-read
`~/.fleet/mcp.json` / `fleet mcp` before relying on it.

**Imported: all 13 available entries from the scan** (11 from
claude-desktop, `node_repl` and `computer-use` from codex; the 3
tinyfish HTTP entries were refused by shape, by design). Sources rode
the entries; `computer-use` imported **disabled** because Codex had it
disabled — importing never turns on what a source turned off.

**Probed ok — 6 servers, 49 tools that pre-join every owner-facing
run:** wikipedia-mcp (22), filesystem (14), dappier (8), time (2),
duckduckgo (2), fetch (1). First probe round had duckduckgo timing out
at 10s — it was the docker image PULL; the second round came back ok.
Lesson: a first-probe timeout on a containerized server deserves one
retry before diagnosis.

**Failed with named reasons recorded in `last_probe` — 7:**
- **4 env-unresolved** (the designed refusal, not defects): `github`
  (GITHUB_PERSONAL_ACCESS_TOKEN), `openweather` (OWM_API_KEY),
  `node-code-sandbox-mcp` (FILES_DIR), `node_repl` (its fifteen Codex
  paths). They worked in their own harnesses because those processes
  carried the values; fleet's child env is minimal by KI-031. To light
  them up the OWNER sets the names in `fleet vault` / `~/.fleet/.env` —
  I never move secret values. Until then each refuses its run by name.
- `computer-use`: Codex's config used a relative path + `cwd`, and the
  registry carries no cwd field — binary unresolvable.
- `fli`: crashes at import time (Python traceback in the recorded
  reason; broken before fleet touched it).
- `vizro`: `initialize` timed out at 10s twice — possibly needs a
  longer warm-up than the probe budget.

**The one credential in the registry file:** dappier's API key rides
inside its own URL (`https://mcp.dappier.com/mcp?apiKey=…`) — the
documented residual; the URL *is* the launch so it cannot be extracted,
the registry stores it verbatim, no card prints it. Everything else
greps clean. (Separately: the GitHub PAT that leaked into one session
log before the `-e` extraction fix — owner advised to rotate.)

Related: [[fleet-mcp-server-registry-plan-unbuilt]] (the build),
[[mcp-manager-build-state-and-coordination]] (ownership).

**2026-10-26 — the owner asked "why are we using a github server?"
The answer: we are not.** Fleet's code references none of the twelve
enabled imported servers — no built-in, skill, or lane names
`mcp__github__*` (or any other imported server) — so they are ballast
whose only live effect is harm: `enabled_specs` resolves every enabled
server's env as fatal, so github's unset
GITHUB_PERSONAL_ACCESS_TOKEN refuses every headless `fleet run` before
its corpus load (see
[[registry-server-seeding-is-all-or-nothing]]). The import mirrored each
source's own enabled flag, so github arrived enabled because Claude
Desktop had it enabled — fleet never chose it.

**The ruling landed the same evening: disabled, and verified.** The four
enabled servers whose env resolves to nothing — `github`,
`node-code-sandbox-mcp`, `node_repl`, `openweather` — are now
`"enabled": false` in the live registry; the other eight (fetch, time,
wikipedia, duckduckgo, filesystem, fli, dappier, vizro) carry no env
requirements and were left alone. The write went **directly to
`~/.fleet/mcp.json`** rather than through `fleet mcp disable`, because
the tree carried a parallel session's in-flight KI-314 edit that did
not compile; verification ran on a HEAD worktree build
(`/tmp/head-build` + its own target dir), where a by-hand `fleet run`
got past seeding, answered, and closed `completed` — the run path is
unblocked end to end. The registry edit is exactly what the CLI
performs, byte-shaped the same (serde pretty, sorted keys).
**Closed 2026-10-26 (`d84d7d9`):** the class defect is fixed —
`enabled_specs` resolves per entry and returns the specs beside the
named absences, so an unkeyed enabled server costs the run that
server's tools (named at startup, in the skills-skipping voice) and
never the run; only an unreadable or unparseable registry file fails
the call. The incident's own shape is the regression pin. Details in
[[registry-server-seeding-is-all-or-nothing]].

**2026-10-27 — vizro removed from the machine entirely (the owner's
ask: "completely remove the vizro mcp").** Its entry is out of
`~/.fleet/mcp.json` (12 servers remain; backup `mcp.json.bak-vizro`)
and out of Claude Desktop's `claude_desktop_config.json` — the source
harness the entry's `source` field named; backup beside it. The
**32 leaked `docker run -i --rm mcp/vizro` containers** are gone and
the `mcp/vizro:latest` image is untagged and deleted; the Docker
Desktop MCP catalog's local cache entry (through `owner: mckinsey`)
is cut too (`~/.docker/mcp/catalogs/docker-mcp.yaml`, backup
`.bak-vizro`) — it returns only if Docker Desktop refreshes its public
catalog. **The leak mechanism is worth carrying:** `--rm` fires only on
EXIT, and a client that dies without closing the container's stdio
leaves the server running forever — fleet spawns the docker-backed
servers (fetch, duckduckgo, time, wikipedia) the same way, so those
are the ones to watch for the same accumulation. The probes'
`vizro initialize timed out` history above stands as why fleet never
got a useful session out of it in the first place.
**2026-10-27, minutes later — the four docker-backed servers are gone
too (the owner: "let's remove those legacy mcps").** `duckduckgo`,
`fetch`, `time`, `wikipedia-mcp` are cut from both `~/.fleet/mcp.json`
(backup `.bak-legacy`; 8 entries remain, only **dappier, filesystem,
fli** still enabled) and Claude Desktop's config (backup `.bak-legacy`;
6 remain). **Same day the owner then said Claude Desktop itself is not
installed ("haven't used it in many, many months") — its config edits
above went into the trash with the whole app leftovers** (see
[[claude-desktop-leftovers-2026-10-27]]); fleet's `~/.fleet/mcp.json`
is the registry of record and survives. Their containers and images no longer exist on the machine:
the owner had just changed Docker Desktop's configuration to reclaim
the resources it was hoarding, and the forced restart reset the
container store wholesale (0 containers / 0 images / 0 volumes — which
is also why the LayerFS worker vanished and the 9:06 run's subagent
got "the LayerFS worker did not answer"; the substrate needs
`fleet install` to re-pull the pinned runtime image and start the
worker again — OFFERED, not run). The docker-backed leak-watch from
the addendum above is moot for these four; what remains enabled is
npx/binary-backed (dappier's URL carries its key; filesystem is
Desktop/Downloads; fli is the Python binary). And the sweep order
rule that vizro taught in the same hour: **edit the configs BEFORE
removing containers, or the live spawner respawns them mid-sweep.**
One entry this pass surfaced and deliberately left alone: Claude
Desktop's `github` server carries its PAT **inline in the config file
itself** — I was editing that file and did not touch it; rotating that
token is the owner's call.
