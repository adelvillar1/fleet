---
status: active
created: 2026-11-22
plan: relevance-pass
---

# The relevance pass — pages lead with what needs you and what changed

The owner's verdict after two visual rounds: *"I was expecting real change on
the UI … no improved usability and information relevance."* They are right, and
this time the failure is named precisely: everything shipped so far **refinished
the information without re-organising it** — a typeface, sentences, and a card
vocabulary on top of the same pages reading in the same order. The plan's own
rows never asked what the page is *for*. This pass does, per surface, and the
owner's standing directive carries it: no deferrals, no lazy shortcuts.

**The design contract** (the owner's questions, answered on their behalf — the
prompt timed out; these are the recorded defaults, revisable at any row):

- Every surface leads with **what needs a decision** (doors owed an answer,
  replies owed, offers awaiting accept) and then **what changed since the
  owner last looked**. Inventory with neither **collapses below** — collapsed,
  not deleted; a `data-collapsed` row the owner can open.
- **The rail** becomes state groups — *needs-you* / *working* / *idle* — the
  first two expanded, the rest one openable group per category.
- **Depth order**: sessions first (read live work, answer), then the inbox
  door, then board + delegation, then the newer surfaces.

## Per-surface contracts

- **The landing (work surface)**: the first block is **needs you** — derived
  from the same `counts.needs`/`counts.working` the rail badge already
  computes: doors owed an answer (the held deliveries the door derives),
  sessions whose run awaits the owner's approval, kanzo cards in the offered
  state waiting an accept. The second block is **changed since you** (newest
  events across the tracked sessions — newest first, with the session's name).
  The archive is the collapsed remainder.
- **Sessions**: ordering is **needs-you first** (a session whose run asks for
  approval outranks one merely running, which outranks idle), and the
  transcript pins the approval request where the run asks for it. Idle sessions
  collapse below the fold.
- **The door**: the held deliveries keep their class (the triage ruling) but
  the page is ordered by **age of the held item** and the count strip retires
  (a number with no action attached).
- **The board**: leads with the plan whose **state changed since last look**
  and any divergence (`never spawned`, `ran twice`), then done counts collapse.
- **Delegation**: leads with what is **blocked** — `never spawned` before
  `ran twice` before running — because blocked work is the decision surface;
  the tree collapses beneath the blockers.
- **Armada + tuios section**: an instance whose daemon is down or whose agents
  are in a non-idle state reads as a need; a healthy idle fleet says its one
  sentence and yields the page.
- **The rail**: state groups as above; the badges are the groups' own counts.

## The rules (inherited)

- Derivations come from the reads that already exist (`counts.needs/working`,
  the door's held rows, the delegation overlay, the kanzo ledger); a block that
  needs a read that doesn't exist extends the READ, never rederives client-side.
- No data is deleted: "retire from the page" is a collapse, and the row
  remains one press away.
- The four-sentence law holds — a leading block IS a sentence with its
  evidence, not a new kind of widget.
- Everything red-proved: neuter a "needs you" derivation (drop the door's held
  rows, drop the approval signal) and its surface's case fails **by naming the
  item that vanished** — an assertion that would pass with the feature removed
  is not a case.
- Numeric ACs where applicable: the landing's first block paints without
  scrolling on a 900×810 stage; the needs-you order is asserted against a
  fixture with three sessions in three states.

## Acceptance criteria

- [ ] The landing's first block is needs-you (doors owed + approvals awaited +
      offers posted), asserted against a fixture containing all three, and its
      order is doors → approvals → offers.
- [ ] The "changed since you" block names N events with their sessions,
      newest first, from the log — red-proof: a fixture event the block omits.
- [ ] Sessions ordering: approval-needed > running > idle, asserted; idle
      collapsed; the transcript pins the approval request in view.
- [ ] The board leads with changed-state + divergence; done collapses.
- [ ] Delegation leads with blocked (never-spawned first).
- [ ] The rail carries three groups; needs-you and working expand.
- [ ] The owner can collapse/expand every inventory block; the collapsed state
      persists per surface (localStorage, the face's existing persistence).
- [ ] Full gate green; harness count never falls; the plan's per-surface rows
      ticked as they land.
