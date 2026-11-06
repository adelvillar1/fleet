---
name: right-fixes-not-easy-fixes-beyond-the-plan
description: "feedback 2026-11-02 — the owner stopped the closeout run with \"we have to do the right fixes not the cheap fixes\" after I wired three shortcuts (a coordinate-splicing accept_hunk, a schema validator silent about the keywords it skips, a compiled_default that nulls all but one key) and had already ticked a criterion on the strength of the weak one; a tick must describe the fix that landed, not the criterion's ideal"
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

Executing the closeout plan ([[open-work-closeout-2026-11-02]]) I was
ticking criteria as fast as I could wire their surfaces. The owner
stopped the run: *"we have to do the right fixes not the cheap fixes"*.
Three concrete shortcuts were on the table, and one had already been
written into a plan:

- **`accept_hunk`** (written, uncommitted, never wired) spliced
  `hunk.old.len()` lines at `new_at` — a coordinate from a diff taken
  *before* any other hunk was rejected. Once one hunk goes back, later
  coordinates shift and the next accept lands in the wrong place: silent
  file corruption. The right shape is an anchored apply (re-read, match
  the exact old block, refuse by name when the file moved) plus
  per-hunk state so the card shows where each press left things.
  **Built 2026-11-02:** `Hunk` carries its context **per side** (three
  lines each way, trimmed so a neighbour's changed lines never enter this
  hunk's anchor), `locate()` finds the block between that context and
  returns `Err` when it appears twice, and **both** `reject_hunk` and
  `accept_hunk` go through it — idempotent, refusing with "the file
  changed since the review" otherwise. The reject had the same
  stale-coordinate defect, so the right fix repaired the shipped verb
  too. The per-hunk accept then landed end to end with the body
  carrying the **reviewed hunk** (its blocks and context), not an index,
  because a decision changes what a fresh diff would number.
- **`validate_structured`** (C13, ticked) checked root type, `required`
  and property types, then returned `Ok` for every construct it does not
  implement — so a schema using `items`, `enum`, `oneOf` or nested
  objects "validates" without being checked. A validator that is silent
  about what it skips is the same class of lie as an untested bound.
  **Built 2026-11-02:** a three-way verdict — `Ok`, `Mismatch(reason)`,
  or `Unchecked(keywords)` naming every keyword the schema declares that
  the check does not read (known-unread words and words it has never
  heard of alike), and the seam turns that into "the result was not fully
  checked against the tool's own outputSchema: the schema uses `oneOf`…".
  What is read grew too: `enum`, `const`, array `items`.
- **`compiled_default`** (U2) knew exactly one key (`approvals`); every
  other scalar carried `null` with a nice sentence in the card. The
  criterion asked the read to carry each default; the right fix is the
  real fallback table. **Built 2026-11-02:** ten settings tied to the
  constant the code itself falls back on (`DEFAULT_MAX_TURNS`,
  `DEFAULT_RETIRE_AFTER_SESSIONS`, `Gates::default()`,
  `resolve(None, None).label()`, …), each entry read from that constant,
  with the pin being the tie — and it caught the first draft's wrong
  `approvals` claim (the default is `ask`, the prompting mode, not the
  yes-guarded word the comment guessed).

**Why:** the plan's criteria are the contract, and a tick is a claim
that the contract is met. Ticking "validated" over a validator that
skips half its input is worse than leaving the box open, because it
retires the work — the next audit reads the tick and stops. Cheap fixes
are recognisable by their tells: a splice that assumes an unchanged
index, a success branch for an unimplemented case, a table of one, a
field that carries `null` plus prose, a suppression where a name was
owed.

**The fourth and fifth right-fixes, found after the pause.** The
closeout kept producing the same shape once the plan was being executed
rather than wired:
- **The seat's keyboard path** (gui-revamp C12) first guarded on "an
  input holds the focus" — but the boot's own autofocus on the composer
  meant every digit read as the owner typing, so the seat could never be
  answered. The right shape is the design, not a special case: the seat
  **takes the focus when it arrives** (it is the one thing on screen that
  needs an answer) unless the owner is mid-sentence in the composer, and
  the guard reads *where the focus is* — a seat button or nothing in
  particular answers; an input holding it is typing.
- **The tools card's audit row** (tool-surface C16) printed the raw
  counts; the owner compares the *rates*. Truncated and redundant now
  ride as shares of the session's calls with the counts beside them.

**How to apply:**
- Before ticking, ask *what would this criterion have to be true of?*
  and check the implementation against that sentence, not against
  "something now exists".
- When a guard reads a state that another feature also sets (a focus, a
  flag, a cache), check the *other* feature's writer before trusting the
  guard — the conflict is the design, not a condition to special-case.
- When an artefact cannot be built the full way (a live run, a store on
  another host), leave it open naming what is missing — the same
  discipline as [[a-ruling-has-to-name-what-the-check-can-see]] and
  [[completion-verifier-audits-tracker-and-clauses]].
- A half-built function that is not yet committed is cheaper to back out
  than to wire; say "this is the cheap shape, the right one is X" before
  building on it.
- The owner's earlier form of the same ask — *"right fix or easy fix?"* —
  is the standing re-audit of a whole slate, not just the last item
  ([[defer-is-a-counting-claim-check-the-bite-count]]).
- The owner's steer that followed — *"the beta pass I will do through the
  own fleet application, it doesn't make much sense to do it through
  zcode"* — is the same principle applied to verification: a live
  behaviour belongs to the product, not to the harness.

Related: [[open-work-closeout-2026-11-02]],
[[a-plan-never-ticked-is-not-a-status]],
[[completion-verifier-audits-tracker-and-clauses]].
