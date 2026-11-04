---
name: huddle-plan-drafted-2026-11-03
description: "project 2026-11-03 — docs/plans/2026-11-03-huddles.md ACTIVE (C1–C14, six phases, gated clean) and PHASE 0 LANDED — the four mockups drawn, rendered and measured at 1440×900; a huddle is a session type, not a second agent; the owner answered the open decisions (a bot brings its role's tools AND skills, a mention auto-runs within the turn budget, a recommendation is advisory, sys1 judging destructiveness rides globally)"
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

The plan is at `docs/plans/2026-11-03-huddles.md`, **status: active** — the
owner approved it with `/goal implement plan as designed`. The ideation and
the rulings live in [[huddle-surface-ideation-2026-11-03]]; the open decisions
it answers are in [[huddle-decisions-open-before-the-plan]].

## Phase 0 is landed — the four mockups (C1)

`docs/plans/mockups/2026-11-03-huddles/` carries `rail.html`, `huddle.html`,
`seat.html`, `breakout.html`, a shared `mockup.css`, a `README.md`, and four
rendered PNGs. **C1 is satisfied** (drawn, rendered, measured — see
[[huddle-mockups-measured-not-read]] for the verification method and its
limit). The screens fix the shape every later phase builds to.

## The owner answered the open decisions (turn 7)

1. **A bot brings its role's tools *and* skills.** *"bots should have the
   skills they need from those assigned to them and skills related to the
   bot and the problem shape"* — the plan reads this as **both**: the
   speaking member's role tools and role skills, plus skills ranked for the
   problem shape; a role that names no tools or skills leaves the huddle's
   own standing. (The ideation turn had read this as skills-only; the plan
   chose tools+skills and flagged it as a decision the owner can reverse.)
2. **A mention auto-runs within the huddle's turn budget.** `@you` always
   waits and raises a needs-you; a mention of a non-member is refused by
   name. Bot→bot mentions auto-run.
3. **A recommendation is advisory.** The bot proposes convening a role the
   brief needs; the owner presses one convene. Auto-convening is recorded as
   a later ruling, not this plan.
4. **sys1 judging destructiveness rides globally**, not only in huddles —
   it changes the approval gate, and the plan flags that in Risks. **Amended
   later the same day:** when asked how wide sys1's reach should be, the owner
   chose **only the actions the standing destructive class already flags** —
   one lane call per held action, not every write-capable call
   ([[sys1-judges-what-is-destructive]]).

## The plan's shape

**A huddle is a session type, not a second agent.** The session already
carries members, goal, bylines and the overlay seam; the loop already
composes a guardrail base with a voice on top. Three model seams — the
mention that switches the speaker, the scope that follows the speaker, the
recommendation that grows the membership — plus one surface and the two
identity rulings. Six phases, each independently shippable; mockups first.

| Phase | Criteria |
|---|---|
| 0 · Mockups | C1 — four screens drawn and read at 1440×900 |
| 1 · The model | C2 huddle = one session · C3 mention routes (`Mention` record, kinds directive/ask/review) · C4 per-speaker tools + skills + ranked fit · C5 capability-gap recommendation → one convene press |
| 2 · The surface | C6 deliverables in-line → pane preview · C7 rail with active/inactive/ended · C8 the room (head, member stack with voices, transcript, @ picker, mode chips) |
| 3 · Breakouts | C9 child huddles carrying the parent's goal |
| 4 · sys1 | C10 destructiveness judged by sys1, verdict on the seat, lane-down fallback names itself |
| 5 · Identity | C11 optional avatar + free name · C12 SOUL as guardrail, voice inside it, closed by a standing constraint |
| 6 · Verification | C13 gate + harness pins · C14 live end-to-end on a scratch home |

**Decision-model check:** one per-item judgment, the owner's own — sys1
judges destructiveness (accuracy and accountability pay; the lane call is
spent once per held action; the fallback keeps the gate alive when the lane
is down). The capability-gap recommendation and the skill selection are
**not** sys1 jobs: the recommendation is the speaking model's own proposal
over the role roster, and the skill ranking rides the existing
router/ranker machinery.

## Gates

- **plan-gate: PASS** — 23/23 covered, 5/5 in scope.
- **arch-gate: PASS with 3 conventions gaps.** One stable DRIFT surfaced
  (re-run twice, identical) and was **resolved rather than papered over**:
  the bot-app ruling *assistant-centric, not work-centric* read as
  forbidding a console room. It does not — it orients the **bot face**; the
  console keeps its own idiom. The precedent section now says so explicitly.
- The three gaps are undocumented areas, not defects: the use cases,
  `tests/face/harness.mjs`, and the new `Mention` record. The tech doc does
  not cover them yet — worth documenting as this work lands.

**Why:** the plan is the contract the next session builds against, and its
open items (the tools+skills reading, the global sys1 scope) are the places
the owner is most likely to want a change. **How to apply:** when the owner
returns to the huddle, read the plan file (not the memory hooks) for its
state; the four decisions above are the ones to confirm or reverse; and
carry the SOUL-as-guardrail ruling into the prompt section, because it
changes what the settings identity slot ships as its starting text.
Related: [[huddle-surface-ideation-2026-11-03]],
[[huddle-decisions-open-before-the-plan]],
[[huddle-mockups-measured-not-read]],
[[a-bot-soul-is-its-role-overlay]],
[[sys1-judges-what-is-destructive]],
[[a-bot-name-is-free-and-its-avatar-optional]],
[[a-new-plan-must-answer-the-past-plans]],
[[plugins-ux-plan-drafted-2026-11-03]] (the mockups-first method).
