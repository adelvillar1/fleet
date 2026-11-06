---
name: delivery-is-not-behind-the-scope
description: "project/feedback 2026-11-04 — KI-410 closed: the comment said \"declared on every run\" while the code gated both the document reader and the hand-over on the speaker's scope, so a scoped bot could write a file and had no way to hand it over; delivery is now declared on every run, and the owner's wider point is that a huddle's OUTCOMES (not just its files) are what the surface should count"
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

**KI-410 is closed** (fix 5d38708, ledger 409 rows / 15 open). The live proof
found it: no role's `tools:` list admits `send_file`, so a speaker's narrowed
scope refused it and the tool was never declared — C4's per-speaker scope met
C6's deliverable, and the in-line card could only ever come from an unscoped
run. The model named it itself: *"my toolset in this run is read_file and
list_dir. There is no write_file, no send_file, and no bash"*.

The drift was in the code's own comment: it said `read_document` and
`send_file` are *"declared on every run that has one"* while both were gated
on `admitted(...)`, i.e. on the speaker's scope. Both are now pushed
unconditionally. The scope still narrows the **work** tools — three delegation
integration tests pin that, and `a_narrowed_scope_still_declares_delivery`
pins the new contract (a clerk narrowed to `read_file` still gets
`read_document` and `send_file`).

**Why:** a delivery is not a work tool, so it does not belong behind the scope
— and the owner chose that reading over "a role that should hand over work
names `send_file`" (which would have kept the scope absolute but made delivery
per-role). **How to apply:** when a comment and the code disagree about a
boundary, trust the comment's *intent* only after checking which one the
product needs — here the comment was right and the code had drifted.

## The wider point the owner made — PARTLY LANDED (e407c75)

The **knowledge** half landed: the room now shows what it recalled and what it
wrote, a distil action turns the room's lessons into facts and skill
proposals, and the rail's outcome count is files **plus** knowledge written —
so a room that taught the home three things reads as three, not zero. See
[[a-huddle-is-a-knowledge-space]]. The **outcome kinds** half (a decision
asked, a review returned, a persona recommended, a gap escalated) is still
open — they are derivable from the log and follow the same seam.

The owner's correction was that I had narrowed "delivery" to *a file*: *"what if
the delivery is not a file but a request for a decision, or basically saying I
cannot deliver this because I don't have the tools or skills and I need help,
or we need a persona that we don't have, or let's discuss in the huddle
because I need more information. you are being too black or white"* — see
[[a-delivery-is-not-a-file]].

The **outcomes** half is still unbuilt: the rail's `deliverables` counts
approved `send_file` results and nothing else, so a decision the room forced
onto the owner, a review it returned, a persona it recommended, and a gap it
escalated all read as **zero** on the row that is supposed to show what the
huddle produced. The offered shape is the union — files, decisions asked,
reviews returned, personas recommended, gaps escalated — on the rail row and
the room head. I recommended it as the next build.

## Also still open, from the same audit

- **The hire's contract is invisible.** When the model authors a persona the
  card says who joined but not what they are *for*; the substance is the
  overlay, and nothing shows it until someone opens `roles/`. The hire card
  should carry the one-line purpose and the voice, editable in place.
- **Cost per room.** Every persona is a model call per turn, and a room that
  hires three specialists can burn quietly. Fleet has spend caps and per-run
  usage; the *room* is the unit that should show it. Lower priority, but it is
  what makes "hire freely" safe to actually use.

Related: [[huddle-plan-implemented-12-of-14]] (the plan's state and the
owner's corrections), [[a-capability-gap-is-a-hiring-decision]] and
[[the-room-hires-its-own-consultant]] (the hiring flow this unblocks),
[[not-every-gap-is-a-persona]] (the owner's re-cut of the gap card),
[[a-harness-find-returns-an-array]] (the harness seams this kept hitting).
