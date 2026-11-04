---
name: landing-and-rail-details-plan-2026-10-30
description: "Plan 2026-10-30 LANDED (d1289ad, pushed e099d40..d1289ad) — the rail collapse leveled at the head (28.5px == the pane icon's center), the instrument's buttons wrap whole (flex-shrink broke labels inside fixed-height buttons), the landing greets the owner's work (needs-you rows, waiting queues, titled recents)"
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

Plan `docs/plans/2026-10-30-landing-and-rail-details.md` **LANDED whole** — commit `d1289ad`, pushed `e099d40..d1289ad`; all eight criteria met, the plan completed. Three owner-reported frictions, each diagnosed live before building:

1. **The rail collapse moved to the head** — a `surfaces-head` row carrying the brand mark and the right-aligned toggle; its height is `calc(var(--topbar-height) / 2)` so the toggle's center measured **28.5px — exactly the pane expand icon's center line** across the seam (the mark is taller than the row and overflows transparently by design). Icons mode keeps the mark centered with the chevron absolutely positioned on the head's edge.
2. **The instrument's buttons fit** — the real defect: flex-shrink squeezed three buttons past their labels' width, breaking the text INSIDE the fixed 28px buttons (`.surface-tool-act` had no wrap). Whole buttons now wrap (`flex: 0 0 auto; white-space: nowrap` on the button, `flex-wrap` + `row-gap` on the container) and the card grows; measured whole at 1440 and 1100.
3. **The landing greets the owner's work** — `owner_recent` is exactly `class == "yours"` (the `None` arm was a fossil of the pre-stamping era; KI-372's rule at its third surface, pinned by `the_owner_recent_answers_the_owners_rows_only`); the greet renders *what needs you* (needs-approval/paused owner runs, one row to the conversation) + *what is waiting* (skills proposals, the memory cycle queue, calibration short cells — each only when non-empty, from `home.waiting`, cached 30s in the home read).

**Two catches the build made on itself** (recorded in the plan's Landed section): the three recent rows that looked like machine noise were `class: yours` with **empty titles** — the owner's own runs reading as id fragments, so an untitled conversation now reads as "a conversation · <when>"; and the greet's waiting count included disposed drafts (said 6, the queue held 3) — it now filters by the proposal's stamped status.

**Why:** the owner ruled the collapse belongs at the top beside the pane expand, the instrument's text overflowed its buttons, and "the landing page needs to be rethought as I don't think it's showing relevant actions any more."

**How to apply:** the greet's data lives in the home read (`needs_yours` + `waiting`), so any new queue surfaces through it the same way; the center-line alignment trick (parent height = the reference row's half minus shared padding) levels any two pieces of chrome across a seam. Delivery state: committed AND pushed this turn ([[rebuild-is-not-commit-push-or-ship]]).
