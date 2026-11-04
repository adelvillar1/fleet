---
name: skills-rail-plan-and-the-approve-defect
description: "project 2026-10-30 — the skills revamp LANDED whole (9bb15e9, 3c2be1f, 8c36c6f): the rail of 13 sections, the declared category table, the pane, and the approve root cause (KI-374 — improve's repo cargo gate ran over a home-side write with no cargo on the face's PATH) fixed and proven live on the owner's queue"
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

Project 2026-10-30 — the owner asked to arrange the skills surface like the
settings/bots revamps ([[fleet-settings-abstraction-search]]), mid-plan
added "make sure that all action buttons on skills work because right now
the approval of suggested skills is not working", then said
"implement both plans in sequence". **LANDED whole** in three commits:
`9bb15e9` (the depth rule + KI-374), `3c2be1f` (the category table),
`8c36c6f` (rail + rows + pane + harness 435/46 → later 451/47 with
memory's case); plan status active→landed, all ten criteria ticked with
evidence in the file.

**What landed:**

- **S0, the approve fix (KI-374):** `improve::apply` runs the four cargo
  steps only when the staged change names repo paths; a home-side write
  (repo_paths empty — a SKILL.md into the canonical library) lands on its
  stage validations + pre-image stash and records one honest
  "home-side write" gate step. One rule fixes face AND CLI. Pinned by
  `a_skill_proposal_lands_without_a_gate_and_names_why` (green with a
  cargo path that cannot exist) and `a_repo_side_apply_still_needs_its_gate`.
- **K2 proven live:** the approve of `procedure-read-file-bash` through
  the real face returned 200 applied; the library took the skill; the
  queue dropped. It exposed a second layer: two of the six drafts were
  REFUSED-STAMPED by the owner's pre-fix clicks and the queue couldn't
  tell — `skills::Proposal` now carries `status` (read off the stamped
  frontmatter, default pending) and the face's needs-you shows only
  pending/waiting, so disposed drafts leave the queue.
- **S1, the table:** 452 live skills (the 453rd dir is `.git` — no
  SKILL.md), frontmatter carries no categories (6 `category:` lines
  total), so `src/skills-categories.json` is declared data — exact-name
  assignment, `category_of`, fallback "everything-else" visible on the
  rail (the newly approved skill landed there at 1, working as designed).
  Counts: building 108, web 46, design 63, media 79, documents 41,
  data 73, platforms 28, machine 14.
- **S2/S3, the surface:** 13 sections (needs-you → categories →
  declined/curation/retired), `#/skills/<section>` remembered, cfg-item
  one-line rows with the USES COUNT on every row (the owner's mid-build
  steer: "show the count of how many times a skill has been used" — the
  approved `use_skill` scan already carried it), budget 30 + named rest,
  search renders a cross-section results card, the pane card carries the
  body + copy/path/ask/retire. Measurement: 2,418 px / 213 controls on
  one board → largest section 1,493 px / 34.
- The improve CLI tests now pin `FLEET_SKILLS_DIR` to the scratch home
  ([[a-test-that-only-passed-while-the-code-refused]]).

**How to apply:** never put a repo gate on a home-side write; a queue of
actions must read the writers' stamped status, not file existence; when a
library carries no categories, declare the table and let a visible
fallback section report gaps.
