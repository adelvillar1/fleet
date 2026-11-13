---
title: the substrate repair is an in-app action — ask, then do
date: 2026-11-13
status: completed (2026-11-13; the card asks and the press runs install's own leg — pinned by the harness ask+trigger case and the route's hermetic refusal; the gate green)
---

# The substrate repair is an in-app action — ask, then do

The owner's dogfood report (2026-11-13): the face's services card showed
"the worker does not answer on port 60207 — the substrate is down
(KI-493's shape…) · repair: fleet install (re-provisions the worker and
re-records the endpoint)" — a diagnosis narrating a terminal command. The
ruling: the surface should ask "do you want to repair it?" and trigger the
action. That is what the self-healing effort was about: diagnose → offer →
act, never narrate.

The pieces already exist: the diagnosis is `health.rs`'s layerfs probe
(→ the `/api/state` services document + live `health` events), the face
paints down-cards in `paintServices` (text only), and the repair itself is
already one callable library function — `install::layerfs_step`,
idempotent, prerequisites named by its own plan, safe while the face
serves. What was missing was the seam.

## Acceptance criteria

- [x] C1: `bridge.repair_layerfs` runs `install::layerfs_step` on the
  blocking pool behind an in-flight guard; on success it re-probes and
  returns the fresh services document AND broadcasts the `health` event;
  on `Skipped { reason, provision }` it errs naming both.
- [x] C2: `POST /api/repair/layerfs` answers 200 with the fresh services
  document, 409 with the refusal; the remote listener's allowlist does
  NOT gain the route (the machine's repair verb stays on the machine).
- [x] C3: `paintServices` paints the diagnosis, then "Do you want to
  repair it?" with a Repair button for a down layerfs (disabled +
  "repairing…" in flight; the reply repaints; a refusal names its cause
  where the press stood). The face stops painting the `repair:` command
  suffix for layerfs; other down services keep their text.
- [x] C4: the harness pins the ask + the press + the in-flight state; the
  route's refusal path is pinned hermetically (a configured-but-unresolvable
  bundle → Skipped → 409 naming the provision); the existing diagnosis
  pins stay.
- [x] C5: KI-539 filed; the gate green; commits on master, push; local
  deploy (rm-then-cp → face relaunch); no release steps.

## Verification

The gate and harness, then live: with the substrate down the card asks;
pressing Repair runs the leg and the card flips to healthy (the health
event repaints it everywhere); a refusal names its cause.

## Out of scope (named, not built)

The TUI's services read; the repair route on the paired-device allowlist;
auto-repair without asking (the ruling is ask-then-act).
