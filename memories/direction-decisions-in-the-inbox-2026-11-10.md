---
name: direction-decisions-in-the-inbox-2026-11-10
description: "project 2026-11-10 — direction decisions in the inbox moved from recommendation to DRAFTED (the demand row is C6 of docs/plans/2026-11-10-improvement-modes-inbox.md); the demand map has NO face presence (a completed plan's unbuilt UX-route promise) — its first face surface is the plan's new #/calibration/demand section"
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

2026-11-10, after the ERNIE Evolve dive ([[ernie-evolve-deep-dive-2026-11-10]]),
the owner asked *"should direction decisions be presented on the inbox as action
items?"* My answer was yes-with-bounds; the owner then steered the work into
modes and asked for a plan, so **the recommendation is now DRAFTED as C6 of
`docs/plans/2026-11-10-improvement-modes-inbox.md`**
([[improvement-modes-inbox-plan-drafted-2026-11-10]], status draft, uncommitted,
awaiting the owner's review — the originally offered name
`2026-11-10-demand-direction-inbox.md` was superseded).

The bounds, now contract language:

- **ONE row** from a trend-filtered `demand::top()` — only Flat or
  Worsening classes at/above `DEMAND_MIN_ROWS`, highest bad-rate;
  improving/thin classes never produce a row (the map's leave-alone
  half), and a pile of demand rows would bury the seats/doctor rows
  that are genuinely *owed*.
- **Signature `family|class|trend|rate-decile`** — never raw rate
  (rates drift per window recompute → the KI-497 trap of a dismissed
  row resurrecting daily); a trend flip or decile jump re-asks.
- **Mode-independent**: direction stays the owner's under ask,
  autonomous, and full alike (area 6's honest completion — make the
  read complete, never automate the decision).
- **TTL-cached** 14-day `demand::collect` in `spawn_blocking` (a read
  must not pay the scan per poll, and must never `record_view`).
- **Route `#/calibration/demand`** — a new computed section on the
  calibration surface, which also lands the self-improvement plan's
  unbuilt UX promise ("the demand map joins the face's calibration
  surface as a section" was never in its acceptance criteria — C0
  pinned only the CLI verb + calib row, so a completed plan carries
  that unbuilt surface promise; the demand row would otherwise be the
  map's FIRST face surface).

**How to apply:** build it from the plan's C6 + its harness case, not
from this note alone. Related: [[ernie-evolve-deep-dive-2026-11-10]],
[[improvement-modes-inbox-plan-drafted-2026-11-10]],
[[owner-inbox-plan-drafted]], [[inbox-redesign-mockup-and-draft-plan-2026-11-09]].
