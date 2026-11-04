---
name: seed-per-visit-state-once-not-per-replay
description: "feedback 2026-10-23 — the face's stream onopen re-loads state and replays a second time AFTER the boot's own scroll-settle marked everything seen, so seeding per-visit UI state (the since-you-were-here divider) per replay erased the first pass's divider; per-visit state is seeded once per conversation per visit, keyed on the session change"
type: feedback
date: 2026-11-04
basis: inferred
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

The since-you-were-here divider (chat/composer build, 2026-10-23) was
seeded inside `replay(state)` — read the seen stamp, mark the first
event after it. It never fired in the harness: the boot's load
replayed once (divider correct), the owner's scroll-to-bottom then
marked everything seen, and the stream's `onopen` handler re-loaded
and replayed a SECOND time — re-reading the now-advanced stamp and
erasing the divider the first pass had drawn. Diagnosis took a probe
inside the seeder printing `(session, stamp, dividerAt)` per replay.

**Why:** replay is not once-per-visit. The face loads on boot, again
on stream-open, again on resume — and any per-visit decision (what had
the owner seen, what was unread) re-derived inside replay gets
recomputed against state the visit itself has since changed.

**How to apply:** state that means "this visit" (seen stamps, divider
position, unread baseline) is seeded once per conversation per visit —
key the seed on the session changing, not on replay running — and
replays only re-render from the already-seeded decision. Same shape as
the drafts lesson from the same build (`swapDraft` keys on the session
change). Watch for it anywhere a load path is both an initializer and
a refresher. Pinned by the C6 harness case; the probe-then-remove
debug loop is in the transcript if the shape recurs.
