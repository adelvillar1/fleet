---
name: a-card-builder-returns-sections-not-a-node
description: "feedback 2026-11-02 — the settings pane's row card builders return [[label, node]] sections; the vault row returned a bare node and the press threw `sections.forEach is not a function` (KI-400). A row that renders fine can still be unpressable, so pin the press, and assert on a subtree the harness can reach"
type: feedback
date: 2026-11-04
basis: inferred
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

Pinning settings-ux U5, the new case opened the trust board's vault row
and the render threw: `renderSections(container, sections)` walks a
list, and that row's `card: () => vaultCard(...)` returned the node
itself. Every sibling row returns `[['label', node]]`. The row rendered,
looked pressable, and broke only on press — KI-400, fixed the same
hour (`[['the vault', vaultCard(...)]]`).

**Why:** the face has one contract for a settings row (`cfgItem({card})`
→ `openSettingCard` → `openDrawer` → `renderSections`) and no type
checks it. A wrong shape in one builder is invisible until something
presses it — and the harness had no case that pressed that row.

**How to apply:**
- A row's card builder returns **sections**, never a node. When adding
  one, copy a working sibling's shape.
- **Pin the press, not the render.** "The row is on the board" passed
  for months; the case that clicks it and asserts the drawer's title is
  what catches the contract break.
- The flat harness has no element ids for dynamically built panes, so
  read built nodes through `page.rows('.selector')[0]` and
  `page.text('dock-object-body')` — `page.el(id)` is null for anything
  the face creates after boot.
- A vault fixture with `exists: false` takes a different branch than the
  one carrying the sentence under test — give the fixture the state the
  assertion is about.

Related: [[the-pane-card-is-live-the-board-pair-was-dead]],
[[a-flat-dom-cannot-prove-structure]],
[[a-plan-never-ticked-is-not-a-status]].
