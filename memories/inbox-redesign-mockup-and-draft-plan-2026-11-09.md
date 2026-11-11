---
name: inbox-redesign-mockup-and-draft-plan-2026-11-09
description: "project 2026-11-09 — the inbox redesign LANDED and the plan is COMPLETED: store with signatures, segments, menus, bring-back and instant press shipped across 0.6.8–0.6.13 (a burned 0.6.11); 0.6.11 and 0.6.12 were deleted at the owner's word, so the releases page runs 0.6.10 → 0.6.13"
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

Owner request 2026-11-09: *"a full redesign of the inbox, it needs more and better
controls — there is no dismiss option, and the setup button doesn't work —
context aware controls, plus dismissal and archive."* Then the goal: *"implement
as designed, pay very close attention to dead buttons, we cannot have that."*

**Root causes (both confirmed live):** the inbox was a **derived view, not a
store** — five machineries read live, so a row could only leave by its fact
resolving (KI-497) — and every onboarding row's button set
`location.hash = '#/chat'`, the conversation the owner was already reading,
while the ladder's real surfaces are elsewhere (KI-496).

**Shipped, completed, and live on 0.6.13** (each release a defect the LIVE read
found after the previous one, not the tests):

- **0.6.8** — the design: `<FLEET_HOME>/inbox.json` with `dismissed` +
  `archived` marks keyed `class:id` carrying the fact's **signature** (a
  dismissed row returns when its fact changes); the read filters and returns
  the archive; one `POST /api/inbox` (dismiss/archive/restore/undismiss, 400
  otherwise); Waiting/Archived segments, class filters, per-row `…` menu,
  keyboard `j/k/enter/d`, refusals painted in the pane. Seats are exempt — a
  seat holds a run.
- **0.6.9** — the archive answered but the row stayed in the queue: the filter
  knew only `dismissed`, and the "still waiting" flag must be computed BEFORE
  the filter hides the rows.
- **0.6.10** — every group header rendered `[object HTMLSpanElement]`: the
  heads were built with `node(name, className, text)` handed a LIST of
  elements, which the helper writes as textContent.
- **0.6.11** — the owner's inbox read emptied (*"I dismissed them to try out if
  it worked"* — the dismiss had worked; nothing self-dismissed) because
  `undismiss` existed with no control reaching it. The read now returns
  `dismissed`, and the pane has a `dismissed · N` toggle with **bring back**.
  This cut was **burned**: the bump's output piped through `head` (SIGPIPE)
  left `tauri.conf.json` at 0.6.10, the assets cut refused, and the empty
  release briefly took `releases/latest` (feed 404) until marked a prerelease.
- **0.6.12** — the same content with the version sites repaired.
- **0.6.13** — a decision's reply carries a full read (the doctor probe), so a
  press changed nothing for seconds. The row's fate now paints in the same
  tick (`working — dismissing` in the hint) and the reply reconciles; ages read
  "just now", not "now ago".

**Final state:** the plan `docs/plans/2026-11-09-inbox-controls.md` is
**completed** (10/10), the recap is
`docs/recaps/2026-11-09-the-inbox-that-can-be-worked.md`, the face serves
**0.6.13** and the feed names it. The owner then asked to delete 0.6.11 and
0.6.12 — both **release and tag** were removed
(`gh release delete --cleanup-tag`), so the releases page runs 0.6.10 → 0.6.13
with holes at 11 and 12; the changelog keeps both sections as the record and
KI-500 notes the deletion. An empty release holding `latest` kills the feed
even after it is demoted — mark it a prerelease.

Underlying reads: `docs/features/inbox.md`, KI-496/497/499/500. Related:
[[owner-inbox-plan-drafted]] (the five classes),
[[a-queued-control-that-silently-returns-is-dead]] (the dead-button class this
run kept re-learning), [[the-release-is-the-owners-no-op]] (the ~10-min cut).
