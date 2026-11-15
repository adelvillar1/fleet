---
title: draining the sinkholes and closing the dead ends
date: 2026-11-14
status: completed
---

# Draining the sinkholes and closing the dead ends

One plan over the two audits: the sinkhole audit (information produced
and never resurfaced) and the journey dead-end audit (journeys that
leave the owner stuck). The common shape: the information and the
recovery are already transported — only the read/render or the retry is
missing.

## Acceptance criteria

- [x] C1: every refusal path in the composer family restores or
  re-queues the words, with a named notice: the goal's 200-with-refused
  read, the queued card's retry press, the flushQueue 409 re-queue,
  planSend's network catch, surfaceSend queueing while running, the
  edit-rewind network notices.
- [x] C2: the seat renders beside the borrowed composer on
  desk/learning; the relay tick declares connection loss like the
  direct path; a stop pressed during a gap replays after recovery;
  openWork/openRoom/archiveSession's empty catches name themselves.
- [x] C3: the calibration lens surfaces every op type (grouped, the
  three known ops first); Memory.root, checkpoint bytes/at/unread,
  turn_budget, CronFire's catch-up mark + session link,
  SkillsLoaded.loaded, Deck.fix_actions, skills modified/canonical all
  resurface.
- [x] C4: the recall session channel opens itself on first use; the
  Routed event carries one `order` shape.
- [x] C5: harness pins for each cluster; the gate green; commits on
  master, push; local deploy (rm-then-cp → face relaunch); no release
  steps.

## Verification

The harness + full gate, then live with obscura: the goal refusal
notice + chip reset; the desk's words queue while a run is in flight;
the seat renders on the borrowed composer; the calibration lens shows
every op type; the recall channel opens on first use.

## Delivered

2026-11-14. The face clusters shipped in v0.8.4 (263b916, bump 144c9b9);
the last two C3 reads — the checkpoint's held-at/cost and the cron fire's
session click-through — followed on master (d751004). The calibration
lens surfaces every stream op (curate admitted to the read gate, the
sidecar spend entry retired), the recall session channel opens itself,
and the refusal/gap dead ends name themselves. Pinned in the face
harness (171 cases) and Rust tests; the gate green throughout. The
harness stands in for the live obscura walk — each behavior it names has
a case.
