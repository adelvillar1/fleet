---
name: mcp-vizro-removed-2026-10-27
description: "2026-10-27 — vizro MCP removed machine-wide at the owner's ask: 32 leaked `docker run -i --rm` containers + image + Claude Desktop's source entry + fleet's registry entry + the local catalog entry, all with backups; the leak shape (--rm fires only on exit) applies to fleet's other docker-backed servers, and the catalog entry returns on a Docker Desktop refresh"
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

**Owner, 2026-10-27: "there are multiple copies of mcp/vizro … help me
completely remove the vizro mcp from this machine."** Done and verified
clean: 0 containers, 0 images, 0 process, no config names it.

**The chain and the leak mechanism (the reusable part).** The source was
**Claude Desktop's** `claude_desktop_config.json`
(`docker run -i --rm mcp/vizro`), imported into fleet's registry on
2026-10-23 (`source: {harness: "claude-desktop", imported: ...}` — fleet
mirrors a source's own enabled flag, it never chose it). The "multiple
copies" were **32 running containers accumulating for up to 25 hours**:
`--rm` fires only on EXIT, and a client that dies without closing the
container's stdio leaves the server running forever with no client.
**Fleet spawns its docker-backed servers (`mcp/fetch`, `mcp/duckduckgo`,
`mcp/time`, `mcp/wikipedia-mcp`) the identical way** — they were running
at the time of the sweep — so the same accumulation applies to them; a
bulk `docker rm -f $(docker ps -aq --filter ancestor=<image>)` is the
sweep, and it must run BEFORE the configs are edited or the next session
respawns. (Also: an `--rm` container that never exits explains why the
count could grow while I was sweeping — the spawner was still live.)

**What was removed, with backups kept:**
- all 32 containers (`docker rm -f`, explicit ids), then
  `docker rmi mcp/vizro:latest` (un-tagged and deleted);
- `~/.fleet/mcp.json` → `servers.vizro` cut (12 servers remain;
  backup `mcp.json.bak-vizro`; JSON round-trip validated before write);
- Claude Desktop's `claude_desktop_config.json` → `vizro` cut (10
  remain; backup `.bak-vizro`) — Claude was NOT running, checked first;
- `~/.docker/mcp/catalogs/docker-mcp.yaml` → vizro's 35-line entry cut
  through `owner: mckinsey` (backup `.bak-vizro`; seam verified
  line-by-line since no pyyaml was available).

**Two caveats recorded where they matter:** the catalog file is a cache
of Docker's PUBLIC catalog, so the entry **returns if Docker Desktop
refreshes** — it is an offer list, not an install; and the registry
memory ([[mcp-registry-live-import-state]]) got a dated addendum with
the removal (its older "vizro was left alone" line is now superseded).
zcode session logs that still mention vizro are historical records, left
alone on purpose.

**Same turn, the owner: "let's remove those legacy mcps" — the four
docker-backed servers.** `duckduckgo`, `fetch`, `time`, `wikipedia-mcp`
cut from BOTH configs (backups `.bak-legacy`; fleet now **8 entries,
only dappier / filesystem / fli enabled**; Claude **6**). Their
containers and images needed no sweep: the owner had **changed Docker
Desktop's configuration themselves to reclaim the resources it was
hoarding, and the forced restart reset the container store wholesale**
(0 containers / 0 images / 0 volumes — confirmed by `docker system df`
and a Docker Desktop process start of 09:02). Two consequences they
accepted without a fight: the vizro leak-watch is moot for these four
(nothing left to leak), and **the reset also took the LayerFS worker
and the pinned runtime image** — the 9:06 run's subagent got "the
LayerFS worker did not answer", so delegations need `fleet install` to
re-pull `node@sha256:3d27e5c…` and start the worker again (offered,
not run — it re-downloads hundreds of MB right after they slimmed
Docker). What remains enabled is npx/binary-backed, so the `--rm`
leak shape no longer applies to anything still installed.

**The sweep's own order lesson (from the vizro pass):** edit the
configs BEFORE sweeping containers, or the live spawner respawns them
mid-sweep — and a spawner still live explains a count that grows while
you remove. One more entry the same pass surfaced and I did NOT touch
(said out loud instead): Claude Desktop's `github` server carries its
PAT **inline in its own config** — a credential in a file I was
already editing; rotating it is the owner's call, never mine.
