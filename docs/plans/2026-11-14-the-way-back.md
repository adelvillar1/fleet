---
title: the way back — the archive lists itself, the threads lens remembers, the runtime question
date: 2026-11-14
status: completed (2026-11-14; the archive lists itself, the threads lens remembers, the light repair reconciles a stale record, kiri captured as direction; Chrome-specific API-read stall filed open as KI-541 — obscura is the verification engine until it closes; the gate green)
---

# The way back

The owner's dogfood verdict on 0.8.3: can't go back to a session in
threads, sessions are not listed, layerfs still complains, and if Docker
is that flimsy we should switch to kiri. "The app took a giant leap back."

The diagnosis, all pinned: the sessions route auto-opens the newest row
and wipes the list the owner asked for (openWork's pick at app.js:26389,
then face.sessions = null + a full re-read); U49 retired the landing's
recent list AND the rail carries no archive destination, so with 815
conversations archived there is no visible way back; the crumb at
#/sessions mislabels as "threads"; and layerfs's endpoint record goes
stale on every Docker restart (the port reassignment — diagnosed and
named in KI-540, repaired in one press, but the pattern repeats).

**The honest answer: the shell is not the problem.** The spine, the
lenses, the drawer and the reads all work (verified live). The leap back
was the auto-open anti-pattern plus a discoverability hole U49 cut too
wide. Both are small fixes.

## Acceptance criteria

- [x] C1: the sessions route renders the list and stays on it; the only
  auto-open left is a needs-you row; the crumb reads "archive"; the C8
  pins reworked.
- [x] C2: the threads lens regains a compact "pick up where you left
  off" (three owned/live threads + the open-the-archive door) rendered
  only when there is something to return to; the rail gains an archive
  destination carrying the needs/working badge (moved off the chat link).
- [x] C3: a stale-record repair takes the light path — read the running
  container's published port, rewrite endpoint.json, re-probe; seconds,
  no re-provision. A dead worker keeps the full install leg.
- [x] C4: kiri captured as direction (Docker Engine API compatible; no
  fleet code change; the owner tries it on this machine).
- [x] C5: the gate; commits on master, push; local deploy (rm-then-cp →
  face relaunch); no release steps.

## Verification

The gate + harness, then live: #/sessions lists and stays; clicking a row
opens it; needs-you still bounces; #/chat shows the question + the
pick-up rows + the archive door; the rail carries archive; a stale-record
repair reconciles in seconds.

## Out of scope (named, not built)

Abandoning the shell; kiri as a code change; retroactive un-archiving.
