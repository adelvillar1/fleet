---
title: dogfood glitches — the drawer is the only right surface, steady streaming, the queued message sends on demand
date: 2026-11-13
status: completed (2026-11-13; every criterion ticked, the four-leg gate green, harness 1215 checks across 166 cases, KI-529..532 filed)
---

# Dogfood glitches — drawer collapse, stream flicker, queued-message send

Dogfooding on the v0.8.0 face surfaced four items, reported by the owner
2026-11-13. Three are face defects; one is a standing rule (no cuts or
releases without the owner's explicit go-ahead — fixes land as gate-green
commits on master and a local deploy only, no version bump, no tag, no
release).

Evidence from the owner, verbatim: "during a chat the right side drawer
still leaves a column open as a transparent layer, I thought we were
collapsing everything on a single drawer", "the stream is flickering a lot
on the chat area, like it's redrawing several times per second", "a queued
message cannot be sent right away the send button is not active on the
message".

## Root causes (verified in the tree)

1. The pane is a U13 drawer overlay, but two CSS rules still reserve grid
   tracks for it (`app.css` 858-860 icons-rail, 11152-11155 tissue-off) —
   a phantom 440px track squeezes the stage. And the chat's own receipts
   column (`.thread-pane`, 280px, 60% transparent, U39) sits beside the
   transcript with no rule hiding it when the drawer opens — the
   transparent layer beside the drawer. Owner ruling: **fold the receipts
   into the drawer**; the chat is transcript-only.
2. `.log` carries `scroll-behavior: smooth` (app.css 2519) while `settle()`
   assigns `scrollTop` once per rAF flush (app.js 1272) — a smooth-scroll
   animation restarted every frame. `assistantRow` re-serializes the whole
   accumulated text per frame (`textContent +=`, app.js 1989); thinking
   deltas rewrite their node unbatched (app.js 3224); `paintProcess`
   rebuilds the run card from scratch each second (app.js 4100).
3. The queue head's send button is disabled by design (`first.disabled =
   index === 0`, app.js 3509) and the composer button becomes Stop while a
   run is in flight — no active path pushes a queued message through.
   Owner ruling: **send-now stops the run, then the message sends at the
   end**; the composer's Send button queues while busy; Stop moves onto
   the working line.

## Acceptance criteria

- [x] C1: the shell keeps rail / stage / connections with the drawer open
  in every state (icons rail, tissue-off) — no rule reserves a
  `--pane-width` grid track; the stage is never squeezed; the stale dock
  description (index.html 1157-1160) reads drawer language.
- [x] C2: the receipts live in the inspector drawer as a tab (pivot-on-click
  kept, U3); the 280px thread-pane column, its CSS block, its
  `data-thread-pane` toggle and localStorage key, and the conv-head
  collapse chip are retired.
- [x] C3: streaming is steady — stick-to-bottom instant per frame (the
  `.log` smooth rule gone), streamed text appended as text nodes (no
  per-frame full re-serialization), thinking deltas rAF-batched, the run
  card's one-second tick updates in place (keyed rebuild on change).
- [x] C4: a queued message sends on demand — the head row's send control is
  active (stops the run; the finished handler flushes the head), the
  composer's Send button queues while busy, Stop lives on the working line
  (the drawer run-tab stop stays).
- [x] C5: harness cases pin each fix; the four-leg gate green;
  `ki_ledger.py` + `plan_check.py` clean; committed on master; local
  deploy (rebuild → `~/.fleet/bin/fleet` → face relaunch); NO release
  steps.

## Out of scope (named, not built)

- Desk/learning `surfaceSend` silently dropping text while a run is in
  flight (its own queue wiring is a larger surface decision).
- The dispatch hero's refusal copy.
- The `usage` event's full `paint()` (block-boundary, not per-token).

## Verification

`node tests/face/harness.mjs` (new cases + the 166 existing), the
four-leg gate, the two ledger scripts, then a live check on the relaunched
face: streaming without flicker, drawer open with no column behind it and
receipts as a tab, queue a message mid-run and send it now.
