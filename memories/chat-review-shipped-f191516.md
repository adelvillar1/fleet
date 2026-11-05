---
name: chat-review-shipped-f191516
description: "2026-10-28 — the owner's chat review (duplicated messages, turn counts that counted tool-call rounds, near-invisible controls) shipped as f191516 with a fourth defect found beside them — delegation events named only the child so the parent face filtered the documented live cards away; four KI rows, exchange-unit ruling, always-visible controls"
type: project
date: 2026-11-05
basis: confirmed-by
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

2026-10-28 (fleet): the owner's review verbatim — "User messages are
duplicated, the turn count makes no sense as it's counting events such as tool
calls instead of an user-harness exchange. Also the message controls seem to
be almost nonexistent on the ui" — shipped as `f191516` (six files, gate green
on the staged tree: fmt 0, clippy 0, release build, 1,594 lib tests + the face
harness at 274 checks across 29 cases, ledger clean; foreign hunks in
app.js/bridge.rs/known-issues.md staged as HEAD-derived blobs so the
concurrent session's resume-record work stayed unstaged).

The four findings, KI-359..362, all closed-fixed with pins:

1. **Duplicates (KI-359).** `ownerRow` appended the raw text as its own span
   before the session-ref/plain branches, and both branches appended again —
   the line survived when the edit affordance was built above it (ee5b305).
   Server-side was exact (zero consecutive duplicate `user` records in the
   last 40 logs), so it was render-only; the harness missed it because its
   owner-text check used `.some()` — one copy already satisfies it.
2. **The unit (KI-360).** the count read `UiEvent::Turn` — one per model
   request — so one message with eight tool rounds read "turn 8". Every
   conversation-scale display (facts row, receipt evidence, composer hint,
   process pill/meta/strip/drawer, session summary) now counts **exchanges**
   via one `exchangesIn` helper that excludes the two system frames — the
   steer frame (`[the owner of this run steered it through its control file`)
   and the scheduled frame (`[scheduled run `) — because a reseed turns them
   into owner cards the live stream never carried, so counting them would
   make the total move on reload. Budget/log/navigation keeps `turns`
   (runbar turns-left vs `runs.max_turns`, the cap note, turn marks +
   rewind's `checkpoints/<session>/<turn>/`, compaction folds, records
   stream, child-run telemetry).
3. **Controls (KI-361).** `.row-rewind` sat at `opacity: 0` until hover and
   the owner's edit floated at the bubble's shoulder where narrow screens
   clip it; replies had no control at all — on a touch face (the PWA) hover
   does not exist. The gates are gone and every message carries a `.msg-tools`
   footer (edit + copy on the owner's bubble, copy under replies) with copy's
   label flip as its trace (the harness pins the clipboard payload — the
   stub runs `setTimeout` immediately, so `copied` is unobservable there).
4. **Delegation past the filter (KI-362, found while testing #2).**
   `UiEvent::Subagent` names only the CHILD session (the branch key) while
   `record()` stamped it under the driving session, so `apply`'s session
   filter dropped every delegation event on the parent face — no compact
   card, no branch line, no click into the child's replay — while
   `docs/features/gui.md` promised all three. Fix: `record()` stamps `parent`
   beside the child key, `apply` admits an event whose `parent` is this
   conversation (other conversations' children still drop); pinned by
   `a_delegation_event_carries_the_conversation_it_rides` and the REVIEW
   harness case's honest child-named branch click.

**Why:** the review's three defects were all invisible-from-where-the-owner-
sits (a duplicated render, a mislabeled unit, hover-gated affordances), and
the fourth sat under a docs promise nothing exercised — every one was found
by reading the face against the owner's words, not by a red test.

**How to apply:** when the owner reports a chat defect, derive the count's
unit from what they read it as (exchanges = their messages, machine frames
excluded so live and reseeded views agree), make controls visible without
hover before adding new ones, and check a documented UI promise by pushing
the real event shape through the harness — a feature no test drives is
exactly where a session filter hides. Related:
[[gui-client-verified-by-a-vm-harness]],
[[shared-file-commits-can-sweep-another-sessions-work]],
[[a-reload-moves-the-total-unless-system-frames-are-excluded]].
