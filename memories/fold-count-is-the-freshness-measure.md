---
name: fold-count-is-the-freshness-measure
description: "Owner ruling 2026-10-31 — the turn count retires from the conversation's surfaces; the fold count (how many times it outgrew its window) is the freshness measure"
type: feedback
date: 2026-11-04
basis: confirmed-by
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

Owner ruling, 2026-10-31: *"we need to remove the turn count, if
anything compaction count is the real measure of a conversation
freshness."* The turn count only says how many model calls a run made —
the loop's iteration, not the conversation's depth. A conversation
folded five times is deep; a ten-turn conversation that never compacted
is shallow.

**Why:** the turn count read as a size/freshness claim on the
delegation tree cards, the replay line, and the live branch note, where
it never answered the question the owner was asking.

**How to apply:** LANDED as `8b3564b` (pushed) — the freshness measure
is the **fold count** — how many `Compaction` records the log carries.
`SessionSummary.compactions`, the delegation tree's nodes
(`read_compactions` in boards.rs, a whole-file count beside
`read_stop`'s tail read), and the replay read all carry it;
`foldsWording(count)` in `app.js` renders "N folds" / "never folded" /
"" (absent), and a zero says so rather than showing a dash. A turn count
survives only as a **remaining budget** (`runTurns`, "turns left") or as
a compaction event's own words ("N turns folded") — never a size claim.
When a live event does not carry the fold count, the line drops the
count rather than inventing one ([[the-owner-s-work-is-the-view]]). One
trap the build hit: the tree's nodes needed a NEW field and every
literal construction of `SessionSummary`/`DelegationNode` broke in
turn (see [[assistant-record-field-cascade]] — same class, five
constructors across four files).

2026-11-03, the boundary made explicit after the owner reported turn
counts still on the PWA: the retirement removed turn counts from the
FRESHNESS surfaces only. What deliberately remains — the 10-31 plan's own
carve-out — is the composer's turns-left **budget** counter (`run-turns`,
shown only while a run is live with a budget) and the fold buttons' "turn
N" labels (a rewind affordance). The owner's report turned out to be
KI-408's stale serving binary (the daemon ran a build from before that
day's fixes), not this retirement regressing — after the reload the
browser and the PWA serve the same face.
---
next-session, type: project,
originSessionId: session_51b168bc9066536c50ecdf2a59b560e4
