---
name: a-plan-never-ticked-is-not-a-status
description: "feedback 2026-11-02 — a plan whose criteria were never ticked as the work landed is stale in BOTH directions (things that shipped read as open, and the owner's "did you leave U9 open?" is answered wrongly either way); audit each criterion against the code, write verdict + evidence per line, and tick at the moment of landing"
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

The owner asked *"did you leave something called U9 open in one of the
plans?"* and then *"yes, other plans are waiting on it"*. The
settings-ux plan's eleven criteria were **all still `[ ]`** — including
U8 (the model-first section) and U10 (the onboarding ladder), which had
shipped weeks-of-commits earlier — while **U6's `money` section, which
the owner had ordered DELETED, was still rendering** (`id: 'money'` in
`app.js`, and the harness still asserted eight rail sections). So the
plan read as "almost nothing done" where most was done, and as
"money is gone" where it was not. U9 was genuinely half-built: the
provider card carried key, models, toggles and removal, but the walks
half — up/down/stop ordering and the add-to-a-walk picker — did not
exist anywhere.

Two other plans were **waiting on U9 by name** (the headers plan
deferred the GUI row to it; the remainders plan marked its W4
"OPEN BY DESIGN — waits for U9"), and nothing in the plan file said so.
A deferral recorded in plan B to a criterion in plan A is only as good
as A's status being true.

**Why:** a checkbox is a claim about the code made at some past moment.
Unticked = "never claimed", not "not done"; ticked = "claimed once",
which may have rotted. Reading ticks as state is the same defect as
trusting a green gate over an uncommitted hunk
([[a-green-gate-can-ride-an-uncommitted-hunk]]) or a verdict over a
re-read ([[completion-verifier-audits-tracker-and-clauses]]).

**How to apply:**
- Before answering "is X open?", grep the criterion's distinctive words
  in `app.js` / the bridge / the harness — the code, not the plan.
- When a plan is being worked at all, write a **status-pass table** into
  the plan (criterion | state | evidence) and tick a box only for a
  clause-by-clause fulfilled criterion, with the pin or commit named in
  the parenthetical.
- Tick **at landing time**, in the same commit as the work — a plan file
  that is only rewritten in batch audits will be stale again next week.
- When a criterion lands, grep every other plan that names it and flip
  its deferral text from "waits for X" to "unblocked by X", naming the
  cut.
- **A batch status pass must audit against the tree, not the plan.** On
  2026-11-02, four parallel read-only audits (one per plan, each
  returning `id | verdict | evidence | note` with a `path:line`, test
  name or sha) walked 61 stale criteria: 25 had shipped, 34 were half
  shipped, and a few were **wording residues** describing behaviour a
  later ruling removed (gui-revamp's per-launch URL token, docs-speak's
  "exactly three files" grep) — those close by restating the criterion
  with the superseding ruling cited, never by ticking.
- **Never reopen a completed plan to hold its residuals** — the plans
  README forbids it. Put them in ONE new plan whose criteria own them,
  and give that plan a checker criterion so the next sweep is a command.
  "One" is load-bearing: the 2026-11-02 residuals plan was superseded
  within the hour by `2026-11-02-open-work-closeout.md`, because the
  owner's next ask ("a plan to address all residuals except …") wanted
  the whole open queue — four active plans, one needing its own status
  pass, and the divergence from a ruling — in a single contract
  ([[completed-plan-unticked-criteria-census-2026-11-02]]).
- Measure the count correctly: scope to `## Acceptance criteria`, or
  Goal/use-case checkboxes inflate it
  ([[completed-plan-unticked-criteria-census-2026-11-02]]).

**The status field is the second half, and it is easy to leave.** On
2026-11-02, with every criterion ticked and `scripts/plan_check.py`
passing, the owner asked *"did you mark the plan as completed, and ticked
off all the work?"* — and both plans still read `status: active`. The
plans README's own rule is *"update `status` in the same turn the work
lands"*; a completed plan left `active` is the same stale-read defect as
an unticked one, and the checker only inspects plans already marked
completed, so it cannot see this. **How to apply:** the close-out of a
plan is two edits, not one — tick the criteria **and** flip the status in
the same commit, then run the checker to confirm. When the owner asks
"is it done?", the answer names both halves.

Related: [[a-plan-marked-met-can-hide-missing-behaviour]],
[[settings-ux-plan-2026-11-01]],
[[a-chained-setting-completes-on-one-surface]],
[[open-work-closeout-2026-11-02]].
