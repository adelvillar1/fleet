---
name: a-memory-index-line-is-not-the-plans-state
description: "feedback 2026-11-02 — a warmup state summary built from MEMORY.md's one-line hooks (and from a plan's `[ ]` boxes) reports work as open that landed days ago; read the plan file's own status-pass table and the commit history before saying what is unbuilt"
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

A warmup turn reported the settings-ux plan as "U1–U4 + U6 open, U5/U7
unmeasured" — a line lifted from MEMORY.md's index hook, written
2026-11-01. The owner challenged it: *"are you reading from recaps or
the actual plan files? some of this work is already done. check against
commit history."* The plan file's own **Status pass — 2026-11-02** table
says **all U1–U11 built**, each with evidence and the gate green for
that pass. The index hook was a day stale and the plan was the state.

**Why:** MEMORY.md's hooks are one-line summaries written when the fact
was saved; they are recall cues, not live state, and they go stale the
moment the work moves. A plan's `[ ]` boxes are worse — unticked means
"never claimed", not "not done"
([[a-plan-never-ticked-is-not-a-status]]). Both read as state and both
lie in the same direction: work that shipped reads as open. The owner
has now corrected this class of answer twice in two days (the U9
challenge, then this one).

**How to apply:**
- For any "what is open / what is left" question, read the **plan file
  itself** — its `## Status pass` table (criterion | state | evidence)
  is the state; a `status: active` frontmatter is not, since completed
  plans stay `active` until someone flips them.
- Never quote a MEMORY.md hook as a plan's status. Hooks are pointers
  to the memory file, and the memory file is itself a snapshot.
- Check the **commit record** before calling anything unbuilt:
  `git log master..HEAD` (a branch with no commits of its own has none
  of its work committed), and grep the tree for the criterion's
  distinctive marker — `ShapeRow` absent from the tree AND from master
  means C8 not started, whatever the plan's boxes say.
- Distinguish **built** from **committed**: on 2026-11-02 the usage
  plan's C0–C2 existed only as an uncommitted working tree, one
  `git checkout` from lost, with all ten boxes still `[ ]`.
- Report the correction plainly when caught: name what you read, what
  the file says, and what the commits show — the owner's challenge is
  the check working, not an accusation.

Related: [[a-plan-never-ticked-is-not-a-status]],
[[settings-ux-plan-2026-11-01]],
[[completed-plan-unticked-criteria-census-2026-11-02]],
[[a-green-gate-can-ride-an-uncommitted-hunk]].
