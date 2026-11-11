---
name: bot-pwa-plan-rewritten-2026-10-28
description: "2026-10-28 — after the owner approved the rebuilt B0 mockups (\"looks good\"), the bot-PWA plan was rewritten whole as a current contract (R1–R6 rulings, D1–D12 decisions, P1–P4 backend increments ahead of the B increments, 18 criteria, a History section); SUPERSEDED the same day — the owner said \"implement plan as designed\" and the whole plan landed as 2976098, status now completed"
type: project
date: 2026-11-11
basis: inferred
tier: durable
shape: []
phrases: []
missing: []
premises: []
recall-count: 0
superseded-by:
valid-until:
source: zcode:fleet:content:285983102772d99aaed2c496d57acdf6f320aad7347d3891e9bb9e2378839d34
---

2026-10-28, after the assistant-first mockups (and the goal / destructive
rulings they carry) were reviewed: **"looks good, let's update the entire
bot pwa plan with the new requirements, and implementation before
continuing"**. The plan at `docs/plans/2026-10-20-bot-pwa.md` was 539
lines of accreted audit sections; it was rewritten whole (391 lines) as a
current contract rather than patched again. The old copy is recoverable at
git `eac265b`.

**Shape the rewrite settled on** — worth copying for any long-lived plan:

- **Rulings first, dated, verbatim.** R1 assistant-centric (see
  [[bot-pwa-is-assistant-centric-not-work-centric]]), R2 the six-persona
  roster ([[bot-pwa-persona-roster-six]]), R3 personas are skill lists,
  R4 goal-invariant multi-persona conversations, R5 the binary approval
  rule ([[one-goal-many-personas-destructive-is-the-only-gate]]), R6
  derived identity. A later ruling supersedes earlier *framing*, never an
  earlier *commit*.
- **Decisions D1–D12** — the six original ones survive (one process/one
  bridge, remote root is the bot shell, own PWA identity, separate client,
  inbox is a projection, the route that exists); D7–D10 are new: personas
  ARE `roles`, the goal is the invariant, only destructive acts take the
  seat, identity is derived never an asset.
- **Four backend increments ahead of the GUI ones:** **P1** the six role
  files (with a test that every named skill resolves in
  `~/.agents/skills`), **P2** `goal` + `personas` on the session and the
  join record, **P3** the destructive class on `src/approvals.rs` (no
  always-allow), **P4** the inbox revision — group by persona, and give
  `archived` its own arm because the archive verb (`9f877f4`) left it
  falling through to `finished`.
- **Landed work recorded, not re-planned:** B2/B3 marked done at
  `7d2c81e` and re-scoped as *extended* by P2/P4
  ([[bot-pwa-b2-b3-landed-2026-10-26]]); B0 marked produced
  2026-10-28.
- **18 criteria** grouped personas / conversation / approval / surface /
  close-out; a **History** section keeping the 10-25 and 10-26 audits and
  the pivot so nothing the old document established is lost.

**Status stays `draft`** — "looks good" approved the *mockups*, not the
plan's gate: D11 still closes only on the owner's dated line in the
mockups' README, and B1 does not start before it. `src/` remains
untouched.

**Open questions carried into the plan rather than answered by me:** the
backfill for pre-P2 sessions (no goal/persona → *unassigned*, never
invented into one); whether a mid-run redirect is a new conversation (the
assumption) or an owner-authored goal-change event in the log; how
`bash` destructiveness is classified (per tool and argument shape,
conservative default); shared conversations filed under the lead persona;
the token cost of multi-persona streams; a role naming a missing skill
must refuse to load by name.

**Why:** a plan that grows by appended audit sections stops being a
contract — the owner reads it as history. **How to apply:** when rulings
change a plan's shape, rewrite it whole around the rulings (rulings →
decisions → increments → criteria → history), record what has landed
inside the increments instead of adding "audit" headings, and never let a
rewrite imply an approval it did not get. Related:
[[bot-pwa-b0-mockups-built-2026-10-28]],
[[plan-brainstorming-before-development]].
