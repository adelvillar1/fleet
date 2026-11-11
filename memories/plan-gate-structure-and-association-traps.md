---
name: plan-gate-structure-and-association-traps
description: "feedback 2026-11-05 — the plan-gate structure preflight requires a literal `## Approach` heading (house-style section names starve arch-gate), command-led criteria under-associate with prose steps, the 2000-char section cap truncates the body the gate reads, and a section must name a criterion by its id or carry its assertion verbatim (a thematic echo is not an association); the plan-surface mapper keys on literal paths inside criterion text"
type: feedback
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

Drafting the goal-task-pane plan 2026-11-05 hit three plan-gate mechanics the
house docs don't name:

- The `[STRUCTURE]` preflight demands a literal `## Approach` heading — a
  house-styled section name ("The design") starves arch-gate's claim read even
  when the content is exactly an approach. Name it Approach.
- A criterion led by raw commands ("**C11 — the gate.** `cargo fmt …`") stays
  MISSING (p≈0.30) no matter how the body commits to it in prose; leading the
  criterion with the commitment ("the gate runs whole locally before any push")
  cleared it in one pass. Lead criteria with the assertion, append the commands.
- The plan-surface module mapper keys on literal paths inside criterion text
  (C9→scripts was really tests/face/harness.mjs; C3→target was src/replay), so
  path-bearing criteria map to the named string, not the touched module —
  disposition the noise once rather than rewording criteria to please it.
- **The 2000-char section cap is a live trap (KI-396, hit again 2026-11-06).**
  `_PLAN_GATE_SECTION_CHARS = 2000` truncates each section, and the gate
  judges only what it sees — a step added past the cap moved no verdict at
  all. On the route-judges-the-work plan the Approach section was 2210
  chars and its last step (the live proof, C5) was truncated away, so the
  gate reported C5 MISSING; tightening the section to 1758 chars cleared it
  in one pass. Keep every section under 2000 chars, and when a criterion
  goes MISSING, check the section length before rewriting the criterion.
- **The same plan's second pass (2026-11-06) hit the same class twice more.**
  A criterion led by the bare word "the pins and the gate" stayed MISSING
  (p=0.25) while its body plainly committed to pins; rewording it to name
  the pins ("the resolver, the judgment input, the plan judgment, and the
  non-plan expansion each carry a pin in `tests/integration.rs`") cleared
  it. And a Risks section whose bullets did not name a criterion drew a
  SCOPE? flag (p=0.47); prefixing each bullet with the criterion it
  threatens ("The log's plan-write signal is noisy (C0, C2)") cleared it.
  The pattern is the same one the first bullet names: the gate associates a
  criterion with a section by the words they share, so a criterion whose
  section never repeats its vocabulary reads as uncovered.
- **The fourth pass (2026-11-06, the same plan) hit the same class twice
  more.** A criterion led by the bare phrase "the pins and the gate" stayed
  MISSING (p=0.25) even though its body plainly committed to pins; rewording
  it to name the pins ("the resolver, the judgment input, the plan judgment,
  and the non-plan expansion each carry a pin in `tests/integration.rs`")
  cleared it. And a Risks section whose bullets did not name a criterion drew
  a SCOPE? flag (p=0.47); prefixing each bullet with the criterion it
  threatens ("The log's plan-write signal is noisy (C0, C2)") cleared it.
  So the association rule is broader than "repeat the criterion's words": a
  section must name the criterion **by its id** (C0, C6) or carry the
  assertion verbatim, and a bare thematic echo ("the pins", "the risks") is
  not an association.
- **The third pass (2026-11-06, the same plan) hit it twice more, and the
  second was a new shape.** A live-proof criterion whose only body mention
  was the phrase "the live proof" stayed MISSING (p=0.39) even after the
  section was reworded; writing the full assertion into the Approach's
  numbered step ("the open-ledger plan is judged to its execution verdict,
  then a goal … routes with the delegation nudge firing") cleared it. And a
  Dependencies section whose bullets named no criterion drew SCOPE?
  (p=0.47); prefixing each with the criterion it serves ("C6's live case",
  "the home of the … tasks (C1, C5, C7)") cleared it. So the association
  rule is broader than "repeat the criterion's words": a section must name
  the criterion **by its id** (C0, C6) or carry the assertion verbatim, and
  a bare thematic echo ("the live proof", "the dependencies") is not an
  association.

**Why:** these are advisorial-classifier shapes, not plan defects — two flags
sat in the 0.3–0.6 judgment band after two reword passes, and the right move
was the disposition, not a third rewrite. **How to apply:** draft with `## Approach`,
assertion-led criteria, and treat sub-0.4 MISSING/SCOPE? flags on sections the
body plainly covers as disposition candidates after ONE reword attempt.
Extends [[plan-gate-indexes-criteria-zero-based]].

**The renumber cascade (2026-11-06, the reload-reattach plan):** inserting
a criterion mid-list (C4, the state-document leg) shifted every later
criterion's label and RE-TRIGGERED MISSING on criteria that had already
passed (the gate criterion and the docs criterion, 0.24/0.44) even though
the Approach's final step named both by id — the insertion invalidates prior
gate passes wholesale. The body did commit to both; dispositioned rather
than re-churned. When a plan amendment inserts criteria, expect every later
label's association to break and batch the dispositions instead of rewriting.

**The goal-task-pane plan's own gate pass (2026-11-05) added two more
shapes.** A criterion led by the bare word "the pins and the gate" stayed
MISSING (p=0.25) while its body plainly committed to pins; rewording it to
name the pins cleared it. And a Risks section whose bullets named no
criterion drew SCOPE? (p=0.47); prefixing each bullet with the criterion it
threatens cleared it. So the association rule is broader than "repeat the
criterion's words": **a section must name the criterion by its id (C0, C6)
or carry the assertion verbatim** — a bare thematic echo ("the pins", "the
risks") is not an association. Also: the plan-surface mapper keys on literal
paths inside criterion text, so path-bearing criteria map to the named string
(C9→scripts was really tests/face/harness.mjs) — disposition the noise once.

**The component-catalog plan's gate pass (2026-11-07) added two NEW shapes.**
(1) A `## Use cases` section's `- [ ]` boxes are read by the gates as CRITERIA —
they shift every criterion number by the box count and mis-associate the
coverage flags (a mockup criterion landed under C2 instead of C0). Drop the
section when the use cases are thin; the feature was a tool and a deliverable
and the two boxes bought nothing. (2) The coverage model does not credit a
criterion whose body step names it in PROSE — the mockup criterion (C0) stayed
MISSING (p≈0.28–0.36, stable across four runs) even with a detailed step 0
describing the sheet, the measurement, the PNG read, and the owner read, and
even after the directory was created. Same class as the gate/docs criteria on
the accepted plans: the disposition is the answer, not a reword. Also confirmed
again: the arch-gate's DRIFT on the first wording ("generalize the deck's
pattern") was real — the tech doc documents `deck` as *the* structured-deliverable
family and the canvas frame as opaque HTML, so the catalog had to be framed as a
new SIBLING family beside `deck`, not a generalization of it.

**The owner-inbox plan's gate pass (2026-11-07) added the same two shapes a
third time, plus the section-cap check:** the gate criterion and the docs
criterion stayed MISSING (0.15/0.27) after the cron/doctor classes
renumbered the list, even though the Approach's step 4 named all four gate
commands and all three doc targets by name; the Precedents section (0.38)
and the Risks section (0.48) drew SCOPE? in the judgment band; and the UX
routes section drew SCOPE? at 0.09 — the strongest scope-narrowing a section
can be ("no routes, one localStorage key"). All dispositioned with reasons
rather than re-churned — **the disposition is the answer once the body
plainly commits; a third rewrite of the same criterion is the churn the
house warns against.** And the section-cap check (every section under 2000
chars) stays the first thing to verify when a criterion goes MISSING.
