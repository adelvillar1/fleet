---
name: huddle-plan-implemented-15-of-17
description: "project 2026-11-04 — docs/plans/2026-11-03-huddles.md at 15 of 17 ticked (C1–C12, C15, C16, C17); KI-410 closed (delivery is declared on every run, not behind the speaker's scope); the owner's knowledge-space ruling landed as the room's knowledge section and a distil action; C13 and C14 remain"
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

`docs/plans/2026-11-03-huddles.md` is **status: active with 15 of 17 criteria
ticked** — C1–C12 plus the knowledge phase C15, C16, C17. Commits on
`face-plugin-host` run 2fb0951 → e407c75, each gate-green before it landed.
**Only C13 (the gate) and C14 (the live proof) remain.** Supersedes
[[huddle-plan-implemented-12-of-14]], which survives as the record of the
owner's corrections up to that point.

## The last stretch, in order

1. **KI-410 closed (5d38708).** The live proof found it: no role's `tools:`
   list admitted `send_file`, so a speaker's narrowed scope refused it and the
   tool was never declared — C4's scope met C6's deliverable, and the in-line
   card could only come from an unscoped run. The model named it itself
   (*"my toolset in this run is read_file and list_dir. There is no
   write_file, no send_file, and no bash"*). The drift was in the code's own
   comment: it said *"declared on every run"* while both the document reader
   and the hand-over were gated on `admitted(...)`. Both are now pushed
   unconditionally; the scope still narrows the **work** tools, pinned by
   three delegation integration tests and by
   `a_narrowed_scope_still_declares_delivery`. See
   [[delivery-is-not-behind-the-scope]].
2. **The catalog and the engagement (f9c2676).** The authoring door searches
   the home's roles FIRST and offers a match with a **convene** press — a hire
   is storage, so reuse is free. An engagement is **standing or temporal**
   (`Join.temporal`): a temporal hire joins for this huddle and is released
   when the room is archived, its role file staying for the next engagement.
   Pinned by `a_temporal_hire_leaves_when_the_room_ends`. See
   [[hires-are-a-catalog-with-engagements]].
3. **The knowledge space (e407c75).** The owner's largest point — *"a huddle
   would not only bring deliverables, but lessons learned, process
   improvements, etc. a huddle is a knowledge space"* — landed as:
   - **C15** — the room shows what it **recalled** (the facts that rode its
     runs, off the `memory` records) and what it **wrote** (the approved
     `remember` results), so a room that produced no files but taught the home
     three things reads as three, not zero.
   - **C16** — a **"distil what we learned"** action starts a run whose brief
     turns the room's lessons into facts and skill proposals through the
     existing writers; the owner reviews the proposals as now.
   - **C17** — the rail's outcome count widened to files **plus** the
     knowledge the room wrote; the other outcome kinds are derivable from the
     log and follow the same seam.
   Pinned by a Rust test for the derivation (a refused write is not a lesson)
   and by the harness for the line and the distil action.

## What remains

- **C13** — the gate is green on everything built (fmt 0 · clippy
  `-D warnings` 0 · 2,031 tests / 0 failed · harness 771 checks / 81 cases),
  but the **release build is stale** (binary Nov 3 19:19, last commit 20:15),
  the harness has no **end → rail** case, and the per-speaker **scope** is
  Rust-tested only — so C13's wording needs amending rather than a pin.
- **C14** — five of seven steps observed live (create, convene, a mention
  routing a turn, breakout, end). The **deliverable is no longer blocked**
  (KI-410 closed, so a scoped bot can write *and* hand over), but the
  **recommendation** and the **hire** have still never been driven live —
  both are harness-pinned only. The next pass is one scratch-home run where a
  bot hits a domain it lacks.
- **Still open from the owner's corrections:** the **outcomes** half (the
  rail's count is files + knowledge; a decision asked, a review returned, a
  persona recommended and a gap escalated still read as zero) and the
  **hire's contract** (the card says who joined, not what they are for — the
  overlay is invisible until someone opens `roles/`).

**Why:** the plan is the contract and its last two criteria are the ones a
next session must finish. **How to apply:** read the plan file's own Status
pass (not the memory hooks) for the tick state; rebuild release before
claiming C13; and when the owner returns to the mockups, remember they have
already seen them and found them unremarkable — the iteration is theirs to
direct. Related: [[huddle-plan-implemented-12-of-14]] (the corrections up to
that point), [[huddle-plan-drafted-2026-11-03]] (the plan's shape and the
owner's four decisions), [[hires-are-a-catalog-with-engagements]] (the
catalog, the engagement, and cost being OUT),
[[delivery-is-not-behind-the-scope]] (KI-410's fix and the outcomes gap),
[[the-room-hires-its-own-consultant]] (the auto-hire),
[[a-delivery-is-not-a-file]] (the delivery kinds),
[[read-the-services-own-config-first]] (what sys1 is),
[[say-only-what-you-have-read]] (the hard ask),
[[do-not-guess-what-a-subsystem-needs]] (the sys1 rebuke),
[[vision-chain-walks-glm-then-mimo]] (the vision steer).
