---
title: the substrate heals itself — the app drives the fix
date: 2026-11-15
status: completed
---

# The substrate heals itself — the app drives the fix

The substrate-repair effort (2026-11-13) made the layerfs repair an in-app
*ask-then-act* press and named "auto-repair without asking" out of scope.
The owner's ruling now (2026-11-15): the app should **drive** the fixes —
a stopped worker container starts itself, and when no runtime is present
the app offers to install kiri. This extends that effort and reverses the
auto-repair carve-out for the substrate. It also retires the stale "start
Docker Desktop" wording the kiri switch left behind.

## Acceptance criteria

- [x] C1: runtime detection. `health::detect_runtime` names the runtime
  (kiri / Docker / none) and tells installed-but-asleep from absent;
  `classify_runtime` is pure and pinned. The repair and provision wording
  is runtime-neutral — no "Docker Desktop" command narrated under kiri.
- [x] C2: the app starts the container. The health loop, when the worker
  is down and a runtime answers, runs the worker start itself behind the
  repair guard + a cooldown; a sleeping daemon is woken first. The face
  paints **"starting container…"** (progress, not a press) from the
  `starting` flag, and repaints healthy (or names the failure) when the
  start lands.
- [x] C3: no runtime → the install offer. With no runtime installed the
  docker service carries `needs_runtime`; the face paints **"no container
  runtime found"** + an **"Install kiri and start the container"** press.
  `POST /api/repair/install-runtime` (loopback-only, not on the remote
  allowlist) downloads the kiri dmg and opens its installer; the health
  loop wakes the daemon and starts the worker once the runtime answers.
- [x] C4: pins + gate. Harness cases for the starting progress and the
  install offer; Rust tests for the classifier branches, the neutral
  wording, and the heal states. The gate green; local deploy.

## The line held

Fleet stays runtime-agnostic — it speaks the Engine API, not a vendor.
Detection only *names* the runtime for honest messages; the install path
fetches the installer the owner asked for (kiri, the chosen runtime). The
manual repair press stays for a worker that will not auto-start.

## Verification

The harness (177 cases) pins the starting card and the install offer; the
health tests pin the classifier, the wording, and the heal states. Live:
stop the worker (`docker stop layerfs-worker`) and the face shows
"starting container…" then heals itself; with no runtime, the card offers
to install kiri.