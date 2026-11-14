# The runtime question: kiri as the layerfs worker's host

Direction note, 2026-11-14. The owner's ruling during the way-back work:
the layerfs worker keeps going stale across Docker restarts (the published
port is reassigned and the endpoint record cannot follow — KI-540), and if
Docker is that flimsy a tool, the runtime should switch.

## The candidate

[Kiri](https://github.com/libracoder/kiri) (Apache-2.0) — a small, fast
container runtime for macOS/Windows/Linux, written in Rust. It **speaks
the Docker Engine API**: `docker`, Compose, buildx and `docker login`
work against it unchanged, with a `kiri` Docker context set up on first
open. Benchmarks claim ~2.5 ms container start through the Engine API
(~80× faster than Docker Desktop), a ~15 MB daemon, and a VM that sleeps
when idle and wakes in ~0.15 s.

## Why this fits fleet without a code change

Fleet's layerfs integration is bound to the **Docker Engine API and the
`docker` CLI** — `docker info` probes, `docker port`, the worker
container spawn (`node layerfs-worker.mjs`), image inspection. None of it
is Docker-Desktop-bound. The owner installs kiri, kiri's context takes
over the `docker` CLI, `kiri import` brings the existing images and
volumes over, and the worker re-provisions onto it through the ordinary
`fleet install` / the face's repair press.

## What kiri changes about the failure mode

- **Port stability across restarts is the bet being tested.** The
  stale-record failure (KI-540) exists because Docker Desktop reassigns
  the worker's published host port on restart. If kiri's published port
  survives restarts, the stale-record path becomes rare and the light
  repair (the endpoint rewrite) covers the residue.
- **Daemon wake latency** changes the health probe's shape: a sleeping
  kiri daemon wakes in ~0.15 s, so a worker that answers slowly is a
  container problem, not a desktop problem.

## The try-it protocol

1. Install kiri (macOS 14+, Apple silicon: dmg from runkiri.com).
2. `kiri import` — the layerfs image and volumes come over.
3. Switch the `docker` context to kiri; stop Docker Desktop.
4. `fleet repair` from the face (or re-run `fleet install`'s worker leg)
   — the worker re-provisions onto kiri through the same Engine API.
5. Restart the machine; watch the health card. If the endpoint record
   survives the restart, the flimsiness is Docker Desktop's and the
   switch is proven. If it recurs under kiri, the record-reconcile repair
   stays the answer and the runtime question reopens.

## The switch, executed 2026-11-14 — and the boundary it found

Kiri 0.2.0 installed (`/Applications/Kiri.app`, the CLI on PATH), the
`kiri` docker context created against its socket
(`~/.kiri/run/docker.sock`), the pinned runtime image pulled by repo
digest, and the fleet project's store volume copied over
(34.1M). Fleet's provisioning records the image reference in the
runtime-portable form now (`repo@sha256:…` — a bare config-digest pin
only resolved on the runtime that computed it; kiri recomputes local
IDs), so the reference travels with the machine, not with Docker
Desktop.

**The boundary, resolved the same day**: the layerfs worker's
socket/volume mounts needed no bundle change — the host compat socket
`/var/run/docker.sock` (Docker Desktop's own symlink, dead after the
quit) repointed at kiri's live Engine API socket, and the worker's
socket mount and inner docker CLI reached kiri unchanged. The worker
runs under kiri (KI-543 closed). Fleet's half: the provisioning records
the image reference in the runtime-portable repo-digest form — a bare
config-digest pin only resolved on the runtime that computed it, and
kiri honours the repo digest instead. The recorded port followed kiri's
assignment (56511), and the health probe reads clean.

Docker Desktop is quit on this machine; the substrate's delegation
sandboxes run under kiri.

## The line

Fleet stays runtime-agnostic by construction: it speaks the Engine API,
not a vendor. The image-reference portability is fleet's own now; the
socket/volume layout lives in the layerfs bundle, and that is the piece
the kiri switch still has to carry across.
