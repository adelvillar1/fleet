---
name: layerfs-worker-dies-with-dockers-data
description: "LayerFS's worker is a long-lived container from node@sha256:3d27…; the endpoint.json it dials is a persisted port, and when Docker's store is empty the substrate is dead with ECONNREFUSED — fleet doctor names it, fleet install is the only repair (KI-493)"
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

Triage 2026-11-09 of two subagent failures 17 minutes apart, both
`the delegated workspace could not open: layerfs refused [Failed]: the LayerFS
worker did not answer (connect ECONNREFUSED 127.0.0.1:49473)`, 0 turns.

**What the worker is.** Not a local process: one long-lived Linux container
started through the bundle's runtime image, pinned
`node@sha256:3d27e5c11e5786e309ec3e03f93ae536eb36e6e5eb3714d5eb3300a36157add0`.
`~/.fleet/layerfs-home/worker/endpoint.json` records the port the CLI dials — a
**persisted** address, not a per-call one, which is why both attempts named the
same port. That is not a race; it is a stale pointer.

**Why it died here.** Docker's daemon is up but its store is empty — zero
containers, zero volumes (the registered `layerfs-fleet-store-*` among them),
zero images. The container cannot exist and the image cannot be run with
`--pull never`. Confirmed by: nothing listening on the port, no layerfs process,
`endpoint.json` mtime 2026-10-29, and `fleet doctor`'s
`layerfs-home FAIL the worker does not answer (Connection refused) — start it
with fleet install`.

**How to apply:** the repair is provisioning, not code —
`docker pull node@sha256:3d27…` then `fleet install`, which spawns
`layerfs-worker.mjs` → `runtime.worker.start()` (idempotent) and re-records the
endpoint. Awaiting the owner's go (a large pull). The substrate's death is
invisible until a delegation needs it, so `fleet doctor` is the surface to
check first; the child's log is a one-line `start` record — the delegation never
reached the model.

**Re-verified still live 2026-11-09** (later the same day, KI-ledger deep
dive): port 49473 still REFUSED, `docker ps -a`/`docker images` still
header-only. Every delegation dies at workspace open until the pull runs —
this is the top item of the deep dive's work order
([[open-ki-ledger-deep-dive-2026-11-09]]).
