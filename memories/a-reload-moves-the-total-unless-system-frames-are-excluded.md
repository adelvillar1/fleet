---
name: a-reload-moves-the-total-unless-system-frames-are-excluded
description: "feedback 2026-10-28 — a count derived from owner messages changes across a reload unless system frames (steer, scheduled) are excluded: the live stream never carried them but a reseed's cards do; pin a display count with a `.some()`-style weak assertion and the duplicate defect sails through"
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

2026-10-28 (fleet, the chat review): the exchange count had to agree between
the live view and the view after a reload, and the two paths disagree by
construction. The live stream carries `message/owner` events only from
dispatch (the bridge broadcast); steers arrive as `control` events and
scheduled prompts are never broadcast at all (cron builds its Agent directly
with `progress = sink`). A reload reseeds from CARDS, which render every
`Record::User` as an owner message — including the steer frame
(`[the owner of this run steered it through its control file …]`) and the
scheduled frame (`[scheduled run …]`). So an unfiltered count reads N live
and N+steers+scheduled after refresh — the number moves without anything
happening. One predicate over both lists (events and cards) fixes it:
exclusions are keyed on the frame prefixes, which are stable literals in
`agent.rs` and `cron.rs`.

**Why:** a total that changes on reload tells the owner their session
changed while they were away — the same trust damage as a count that counts
the wrong thing. And the same review found the harness's twin weakness: the
duplicate-message defect passed because the owner-text check was
`.some(el => el.textContent === …)` — a predicate one copy already satisfies.
A check written as "exists" cannot catch "exists twice".

**How to apply:** any count the owner reads (exchanges, totals, tallies)
gets one derivation function applied to whatever list the surface holds —
`exchangesIn(face.events)` live, `exchangesIn(replay.cards)` in the drawer —
so live and reseeded views agree by construction; exclude system frames by
their stable prefixes, not by guessing. Assert multiplicity with a
**count** (`filter(…).length === 1`), never `.some()`/`.includes()` for
anything that could render twice.

2026-10-29 (turn feedback, 402d947): the frame rule crossed the
client/server seam — the fork's cut counts non-frame owner messages
server-side (`is_system_frame` in bridge.rs) to agree with the exchange the
client stamped, so the prefixes now exist byte-for-byte in two languages
and a drift would cut the log where the count says a reply ended but the
log says a frame began; both sides are pinned with tests that include the
frames, and the pairing is documented at both sites. Related:
[[chat-review-shipped-f191516]],
[[a-predicate-naming-shapes-that-never-occur]],
[[a-fixture-corpus-flatters-a-retriever]],
[[turn-feedback-shipped-402d947]].
