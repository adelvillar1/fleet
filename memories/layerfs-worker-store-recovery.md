---
name: layerfs-worker-store-recovery
description: "project 2026-10-29 — the owner's Docker config change wiped the LayerFS worker container, its image, and every Store volume; the recovery route (image = official node by digest, fleet install, DECLARED re-registration, config re-record) and the fact that the pre-reset history is gone"
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

2026-10-29 (fleet): Workspace History refused with
`ECONNREFUSED 127.0.0.1:54277` → then, worker back, "the Store's directory
… cannot be examined: ENOENT". Cause: the owner's Docker configuration
change reset the container store — the `layerfs-worker` container, the
runtime image, and every `layerfs-*` volume went with it, while
`~/.fleet/layerfs-home/worker/endpoint.json` kept the dead port (a stale
runtime-written value; the port appears in no config the owner edits).

Recovery, in order, each step verified:

1. **Image** — `docker pull node@sha256:3d27e5c1…`: the bundle manifest's
   `images.runtime` is `docker.io/library/node` **by digest**. Nothing to
   build; a pull restores it.
2. **Worker** — `fleet install` (its substrate step runs
   `layerfs-worker.mjs`); doctor's `layerfs-home` line flips to
   "worker answering · image present". No LaunchAgent supervises it, so a
   container loss stays broken until install runs again.
3. **Volumes** — `docker volume create <name>` for each of
   `config.json`'s `layerfs.projects[*].volume`.
4. **Folder** — the project's runtime record still names the dead volume
   (`ENOENT` even after the volume exists), so the project must be
   re-registered: `runtime.project.remove(name, {confirm, stopSandboxes})`
   then the register path. **Register through the DECLARED path** — stage
   `git ls-files --stage -z` to a scratch dir (498 files / 20.3 MB for
   fleet) and point `scripts/layerfs-register.mjs` at the staging dir; a
   first attempt pointed at the repo folder itself and imported the whole
   24.9 GB tree including `target/`. Then re-record `volume`/`store` in
   `config.json`.
5. The installed store can't be repaired — **the pre-reset history is gone
   by fact**; the Store restarts at its Init layer (main + one layer + the
   nine figures). Other registered projects restarted empty too and
   re-initialize when next used.

**Why:** the panel's old refusal pasted the SDK's node stack, so a
container/volume loss read as a script bug; every step of the recovery is
invisible unless you know the runtime's own records.

**How to apply:** treat this as the runbook for "Workspace History refused
with ECONNREFUSED / ENOENT"; before pulling, read the manifest's
`images.runtime` rather than guessing a tag; never register a repo folder
by hand — the declared import is the bound. The panel now names both
failures before the script runs (KI-364). Related:
[[layerfs-history-instrument-2026-10-26]],
[[layerfs-expert-study-2026-10-25]], [[fleet-gui-is-a-launchagent]].
