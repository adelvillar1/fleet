---
name: conversation-sessions-home-plan-2026-10-29
description: "project 2026-10-29 — the owner's conversation/sessions UX report diagnosed at four seams and IMPLEMENTED in full as 806abdd (docs/plans/2026-10-29-conversation-sessions-home.md, N0 rail = owner's work only, N1 the home always reachable, N2 New mints nothing until words, N3 live rail badges); archive census + the session-log record shape are in here"
type: project
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

2026-10-29 the owner reported the face's conversation/sessions friction
("yet another look… they feel somewhat disconnected… no easy way to jump
through active working sessions… landing on the main conversation page is a
one and done deal… a lot of noise on the Work rail with non-user initiated
sessions") and added mid-turn: **system initiated sessions should be only
available via a log for analysis, never on the user screen** (the escalation
of [[the-owners-work-is-the-view]]).

The four diagnosed seams (all in `src/gui/assets/app.js`):

- `#/chat` and `#/sessions` render the SAME panel (`panel-chat`); the only
  difference is the work column (`showRoute`, ~14025). The home (greeting,
  counts, recent) exists only as the no-conversation state, so neither rail
  destination is whole.
- The home is one-and-done: rail "conversation" and Escape both walk to
  `#/chat`, which with a loaded session shows the bare transcript; boot
  restores the last panel from `fleet-panel` localStorage; the only door
  back is New — which mints a start record immediately, so "getting back"
  costs an empty session (34 such shells in the archive).
- The rail's count badges are DEAD structure: `surfaces-count` spans are
  created with `dataset.count` (~14783) and no code ever fills them.
- The work groups filter by STATE not class (~7880), so scheduled/subagent
  rows in needs/working/archived states leak onto the user screen; the
  "background · N runs · hidden" toggle line is itself permanent noise.

Archive census (verified 2026-10-29, 538 logs): 362 scheduled + 41 subagent
+ 135 kind-less (34 empty shells + 101 with text — owner conversations plus
face-harness/manual probes). The rail's newest-200 slice: 141 scheduled + 28
subagent + 30 kind-less = 85% system. All 362 scheduled logs carry the kind
stamp (no backfill needed). `sessions-archived.json` absent is BENIGN — an
empty registry removes the file (same rule as titles). `fleet eval` runs in
scratch homes; the kind-less probes are the face harness and manual checks
against the real home (a stamping decision named out-of-scope in the plan).

Tool lesson from the census: session-log records are TOP-LEVEL typed —
`user`, `assistant`, `tool_result` are the record's `type`, NOT `message`
records with a `role` field. Scanning the archive with the wrong shape
returned zero owner text twice before the third scan read it right.

Plan drafted, status `draft`, awaiting the owner's "implement": N0 the rail
renders class-yours only (system sessions stay on `fleet cron` / `fleet
sessions` / the calibration stream), N1 the home always reachable (detach
without minting; recent list re-enters), N2 New creates the session at
first send so abandoned composes leave no shell, N3 the badges come alive
(needs+working counts, click-through). Each with a face-harness check.

**LANDED 2026-10-29 as `806abdd`** (owner steer mid-turn: "build the entire
plan, not just this portion"). N0: one predicate (`railRow` = class-yours
and not a shell) across every group; the background line/toggle/family
code deleted; the bridge's counts owner-scoped (a head cannot count a row
the list will not render — KI-300's law), all-class background count kept
as the document's own fact; welcome recent filtered at both sources
(bridge `owner_recent`, the face's fallback). N1: arriving at `#/chat`
detaches (draft swaps, pointer clears, welcome shows, nothing server-side);
the stream guard skips named events for a detached face while the
`session` ADOPTION event still lands — a send from the home follows its
own conversation. N2: New and /clear walk home with a pending intent
(`face.pendingNew`), create-then-send in one turn, pending draft under
`fleet-draft-new`, rail + @-list skip wordless unkinded logs. N3: the
dead count spans carry needs+working from every surface, hidden when
quiet. The @-list deliberately KEEPS system sessions (summoned search,
not ambient screen). Harness green 323 checks / 35 cases (rewrote C1/C3/
C4 to the new contracts, added the three N cases); gate whole, exit 0;
gui rebuilt+restarted. Plan status completed. Named follow-up: the ~59
harness/probe sessions in the archive need their own stamping decision
— SOLVED the same day as the probe stamp ([[probe-stamp-2026-10-29]],
`5b55768`, KI-366).
