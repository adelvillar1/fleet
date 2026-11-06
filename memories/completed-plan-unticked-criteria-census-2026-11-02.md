---
name: completed-plan-unticked-criteria-census-2026-11-02
description: "project 2026-11-02 — the \"what else was left behind?\" sweep, CORRECTED against git and the tree (8 completed plans / 61 unticked criteria under Acceptance criteria, not 17/107), SWEPT the same day (25 ticked with path:line, 34 residuals), then CONSOLIDATED into docs/plans/2026-11-02-open-work-closeout.md (C1–C22, six increments) on the owner's \"create a plan to address all residuals except session parity\"; the residuals plan is cancelled; KI-399 filed for O1's ruled-mechanism divergence; pushed 0cecba0"
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

2026-11-02, after U9's audit the owner asked *"makes me wonder what other
deliverables were left behind?"*. A sweep of `docs/plans/*.md`
(frontmatter `status` + unticked `- [ ]` boxes) and the KI ledger, then
— on the owner's *"compare against git history first, I'm surprised that
there are this many gaps"* — the census was re-derived from git and the
tree.

**The first count was inflated by a counting mistake.** It reported
**17 plans / 107 boxes**, because it counted EVERY `- [ ]` in a plan;
several plans put their Goal and use-cases in checkboxes too. Scoped to
each plan's own `## Acceptance criteria` heading the real number is
**8 completed plans / 61 unticked criteria**: tool-surface 17,
gui-revamp 14, conversation-control 10, ki-triage 8, docs-speak 5,
skills-the-model-can-reach 5, projects-repo-hygiene 1, open-calls 1.
KI-398's row was rewritten with the recount and pushed `3bd35ad`.

**Then the sweep itself (owner: "let's clean the book keeping gap
first").** Four parallel audits walked every criterion against the tree
— code, tests and commits, never the plan's own prose. Result, pushed
`30d7a8d`:

- **25 criteria ticked**, each with the `path:line` that proves it
  (tool-surface C5/C6, gui-revamp C5/C10/C14/C15, conversation-control
  C0/C1/C2/C7/C9, ki-triage C0/C1/C3/C6/C7, docs-speak C0/C1,
  skills C0/C1, projects-repo C0).
- **34 residuals** (criteria whose other half shipped) collected with
  what is missing named per row — the plans README forbids reopening a
  completed plan, and a residual with no owner is the same defect one
  level down.
- **The residuals split in three kinds, and that split is the useful
  part:** evidence gaps (tool-surface C1/C7/C14/C15 — the record and the
  fold artifacts the rest of that plan's claims lean on), test-only gaps
  (six conversation-control rows, gui-revamp C1/C13), and **wording
  gaps** (gui-revamp C0/C6 describe the per-launch URL token that
  commit `919416a` removed; docs-speak C4's "exactly three files cite
  each note" no longer holds — 18 and 6 do). A wording residue closes by
  restating the criterion with the superseding ruling cited, never by
  ticking it.

**Then the owner said "create a plan to address all residuals except
session parity reminders, that is being done by another session"** —
pushed `0cecba0`. `docs/plans/2026-11-02-open-work-closeout.md` is now
the ONE contract for the whole open queue: **C1–C22 across six
increments**, ordered by risk — E1 the security divergence (O1/KI-399),
E2 the measured embed bound (KI-249), E3 the settings surface
(U1–U10), E4 the evidence residuals, E5 the small closes, E6 the
close-out tail (the router's composed leg, the sys1 extraction leg's
status pass, beta B5, and the checker KI-398 asks for). **C22 is the
queue's own closing rule** (every residual row ticked in its owner's
status pass or dropped with its reason and ruling cited). It excludes
session-parity-remainders C4 and the two unstarted drafts (capability
pitch, usage-in-calibration).

The intermediate `2026-11-02-completed-plan-residuals.md` (written
about an hour earlier, R1–R6) was **cancelled and folded into the
closeout plan** — two homes for one queue is the defect the work
exists to remove. KI-398 now cites the closeout plan's C21 (the
checker) and KI-399 its C1.

**The find that is not bookkeeping: O1 diverged from its ruling.**
`2026-11-01-open-calls.md` ruling (a) was to validate a relay join's
`sid`/`hash` against the relay's own minted-session store; what shipped
is a shared deployment join secret (`relay/relay.py:250`) and the code
never reads `relay-sessions.json`. A secret-holder can still assert a
new `sid`, and the tested refusal is "no session proof", not "not a
minted session". Filed as **KI-399**.

**plan-gate does not read criterion IDs in the text.** It scored the
closeout plan 17/22 covered, "missing" C11 and C18–C21 even though the
increments name those ids explicitly; it matched on semantic embedding,
not on the `C<n>` token. Adding `**Closes:** C…` lines moved coverage
around non-monotonically (19/22, then 17/22), so the gate's MISSING
list is a matcher artifact and the commit message said so rather than
claiming a pass.

**A tool bug worth remembering from this pass:** the first status-pass
script silently missed EVERY tick (`pat.search(t, re.M)` passes `re.M`
as the *position* argument, not flags — `search(string, pos, endpos)`).
Twenty-five `MISS tick` lines were the tell; the fix was a plain prefix
match on each line. Revert the eight plan files and re-run rather than
patching around a half-applied edit.

Related: [[a-plan-never-ticked-is-not-a-status]],
[[completion-verifier-audits-tracker-and-clauses]],
[[settings-ux-plan-2026-11-01]],
[[corpus-contract-and-open-calls-plans-2026-11-01]],
[[plan-gate-indexes-criteria-zero-based]].
