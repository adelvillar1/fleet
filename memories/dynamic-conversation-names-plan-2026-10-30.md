---
name: dynamic-conversation-names-plan-2026-10-30
description: "LANDED d8b3447 (pushed d1289ad..d8b3447) — the dynamic title ladder (owner > goal > plan > first words), read-time, no model; title_from marks the rung; harness 474/51, suite 1,842, live-verified"
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

Owner ask 2026-10-30 — *"the name of a conversation would be dynamic and based on the last plan or deliverables that were being done"* (cognitive load on the main page and the sessions list). Plan `docs/plans/2026-10-30-dynamic-conversation-names.md` **LANDED whole** — commit `d8b3447`, pushed `d1289ad..d8b3447`; all eight criteria met, plan completed.

**The design — a title ladder, read-time derivation, no model call**, in the sessions read's `title` field every surface already reads (the work rail, the sessions list, the landing's recent rows, the conversation header):

1. **The owner's rename** (`sessions-titles.json`, the pencil) — never overridden.
2. **The standing goal** (the goals sidecar, `session_goal`; the composer's `+goal` chip writes it) — the DYNAMIC rung: it moves when the work moves.
3. **The last plan's first meaningful line** — `summarize` carries `last_plan` from the pass it already runs; the LAST plan record, headings stripped (a conversation that planned twice is named by where it landed).
4. **`first_text`** — the fallback, demoted to last.

Each row carries **`title_from`** (`owner | goal | plan | first`) — a derived name is marked as derived; the header's tooltip says what named a conversation, so a stale goal is diagnosable in one hover.

**Landed as:** the pure fn `title_ladder(owner, goal, plan, first)` extracted beside `owner_recent` (the read calls it), three pins (`the_summary_carries_the_last_plan`, `the_title_ladder_orders_the_rungs`, `the_owners_rename_wins_after_a_goal_lands`), the harness NAME case pinning the three rungs on the rail and the tooltip. The bot inbox's `goal` field reuses the once-read value. Face harness 474/51; suite 1,842/0/5. **Live-verified:** one real conversation names itself by its goal rung, one by the owner's rename, the rest by first words.

**Considered and rejected**: naming by the last user message — it changes every turn. The goal and the plan are the owner's own declarations.

**Plan-gate lesson folded in**: a close-out criterion (D8) whose body referenced the Verification section read as UNCOVERED; restating the gate commands in the criterion's own body only moved the flag p=0.18→0.34 — still advisorial. Waive borderline coverage flags with justification; do not chase them.

**How to apply:** when a surface names an object, derive from the owner's own declarations in staleness order rather than generating; put the ladder in a pure fn so the pins don't need the read's machinery. Recorded in the session's recap at `docs/recaps/2026-10-30-the-landing-rethought-and-dynamic-names.md` (committed b306a71, pushed — the day's third recap; the curator session kept its own). Related: [[ki-372-the-owners-screen-is-class-yours]], [[landing-and-rail-details-plan-2026-10-30]] (the empty-title fallback this built on), [[session-recap-2026-10-30-landing-rethought-and-dynamic-names]].
