---
status: completed
created: 2026-10-29
updated: 2026-10-30
slug: settings-rail
---

# Plan: the settings revamp — a rail for the sections, one-line rows, and the item in the pane

## Context

The settings surface is the last long page in the face. Measured on this
machine (`#/settings`, Chromium 1440×900):

- **13 cards on one board**, **18,448 px tall**, **678 interactive
  controls** in the DOM at once — every knob, rule, job and server
  rendered whether or not the owner is looking at it.
- **11 chips that only scroll.** A chip jumps the viewport into a page
  you then read past, and nothing on the page says which card you are in.
- The document behind it is small and honest: 23 scalar knobs, 7 list
  overrides, 24 remembered rules, 15 cron jobs, 9 env names, one
  identity slot.

The work rail already solved this shape of problem — a declared group
list, one destination at a time, a collapse to icons, the choice
remembered. The settings surface is the one place that idiom has not
reached, and it is also where the height problem is worst: four cards
(providers 4,397 px, cron 3,181, tools 2,065, knobs 1,679) are 63% of
the page.

## Decisions (the owner's, dated and quoted)

1. **2026-10-29 — the rail.** *"we should use the same rail approach we
   are using with the work rail, a collapsible rail with icons for the
   sections and each section presents only the items for that section.
   usability and reducing friction is the overarching goal."*
2. **2026-10-29 — the height.** *"reduce wasted space on cards, reduce
   scrolling by collapsing cards that are too long and that would only be
   occasionally reviewed."*
3. **2026-10-29 — the item in the pane.** *"maybe we can click on
   setting items and bring out the right side pane to display the details
   and allow edits from that pane."*

The three compose into one shape: **rail → one section → one-line rows →
the item in the pane.** The rail answers "which list", the rows answer
"which setting", the pane answers "read it and change it".

## Approach

### The shape

Three columns of navigation in one product, each at its own scale — the
global rail goes to a *destination*, the settings rail moves *within one
destination*, the row picks the *item*:

```
global rail │ settings rail (8 icons) │ one section: a list of rows
                                        └─ a row's press ─▶ the right pane:
                                           the item's card, editable
```

### Eight sections

The thirteen cards group into eight, named in the owner's language and
ordered by what a person arrives to do:

| Section | Holds | Rail badge |
|---|---|---|
| **models** | the router (tiers, gates, resolved column), providers, workload overrides | n overrides |
| **specialists** | the lanes, the MCP servers, the tools | n servers |
| **roles** | the delegation roles | n roles |
| **trust** | the vault, the approvals mode, the remembered rules | n rules |
| **money** | the spend caps and the budget knobs | — |
| **schedule** | the cron jobs | n jobs |
| **interface** | the interface knobs and the identity slot | — |
| **environment** | the env-var counterparts and the remaining knobs | n names |

A rail row is the console's existing link row (`icon()` glyph, name,
count). A section holding an override carries a **changed** mark — the
signal is already in the document and needs no new read: a scalar carries
`source` (`file`/`env` rather than the compiled default), a list override
carries `reverts`, and rules, jobs and configured servers *are*
overrides.

### The rail is the work rail's idiom

Collapses to the same 48 px icon strip, with the same toggle and the same
remembered choice (`localStorage["fleet-settings-rail"]`). A `nav` of
buttons with `aria-current` on the open section and arrow-key roving.
The old `sectionNav` chips retire — the list *is* the navigation.

### The row is one line

Name, current value (or state), and the marks worth a glance: `env` when
the value comes from the environment, `override` when it differs from the
compiled default and can be reverted, a live dot where a rule or job is
on. At ~30 px a row the providers' 4,397 px card becomes about ten lines.

**What stays in the row** — the rule that keeps the pane from becoming
friction: a control you can settle in **one press** (a checkbox, a
two-or-three-choice mode, the vault's unlock). Anything you must *read
before you write* opens in the pane.

### The item opens in the pane

Not a new surface: the pane's object card already hosts editable forms —
the fact editor (fields, save, revert) — and `openDrawer` is how every
selected object reaches it. Settings items join that family.

- The press opens the item's card **full height in the pane, the tabs
  (run/files/preview/web) standing down** while it is open; closing
  returns them.
- The card carries the fields, a provenance line in the owner's words
  ("set in the environment — a file write would not take effect until it
  is unset"), the revert where the document offers one, and Save/Cancel.
- **One write path**: the card posts the same `writeSettings` ops; on
  success the row's summary and marks re-render in place, so the change
  lands where the owner will look for it.
- **Adding is a row too**: "+ add" at the end of a list (a rule, a job, a
  provider, a server) opens the card in its new state.
- **Keyboard**: Enter opens, Escape closes and returns focus to the row,
  the card takes focus on open, and closing with unsaved edits asks first.
- Below 1180 px the pane is already the overlay that hosts object cards
  for facts — the same press works there with no second design.

### The long lists get a budget

The two lists that are long by nature rather than by row weight (cron 15,
rules 24) show the first N with the rest behind one press and the count
named ("24 rules · 12 more"). No stored state, deterministic, honest.

**Rejected, with reasons.** A nested scroll box per card shortens the page
but creates scroll traps — two scroll areas in one column, worse on touch;
a per-card remembered fold as the *primary* mechanism carries a state map
nobody asked for and leaves the first visit at 18k; a global
compact/detailed switch invents a second visual language for the same
controls.

### Addressability: a section and an item are places

- `#/settings/<section>` opens the section; bare `#/settings` resumes the
  remembered one; a reload keeps it; an unknown segment falls back to
  `models`.
- `#/settings/<section>/<item>` opens the section *and* the item's card —
  a setting is as linkable as a fact.
- The onboarding's strategy preview's "change it" lands on
  `#/settings/models`; its "seal your keys" card lands on
  `#/settings/trust`.

### Finding a knob

A search box at the head of the rail: typing matches item labels across
**every** section (a knob's key or summary, a server's name, a rule's
tool, a job's name), narrows the rail to the owning sections, and opens
the chosen item's card. "Where is the budget" answers itself without
knowing which section owns it.

### Reuse, not new chrome

The rail reuses the work rail's geometry and the console's own rows, the
fold strip, `icon()`, `pageHead`, and the card system. No new colour,
radius, font or spacing token — the harness asserts it, as it does for
the onboarding.

**The height target.** On arrival, no section above ~1,200 px and no card
above ~700 px unless the owner opened it. The largest offender today is
4,397 px.

## Increments

Each lands whole — code, checks, docs — and each is green on its own.

### R0 — the rail and the sections

`index.html` becomes rail + content column; the eight sections are
declared once (id, icon, name, the document fields they own); the rail
renders with counts and changed marks, collapses to icons, remembers the
choice, and takes the keyboard. The section route (`#/settings/<section>`,
remembered, fallback) lands here. Sections still render their existing
cards — tall, but only one section's worth. `sectionNav` retires.

**The rail is extracted as a helper, not written inline**: this surface
is the first of two (the bots page is the second —
`2026-10-29-bots-rail.md` depends on it), so `sectionRail(declaration,
options)` lands here with the rows, the collapse and its persistence key,
`aria-current`, arrow roving, and the row-budget helper. Two consumers
are what prove it a helper.

*Checks:* S1, S2, S3, S6.

### R1 — the one-line rows

Each kind gets a row renderer driven from the document — provider, tier,
rule, job, server, role, lane, tool, env name, scalar knob — and the
sections render as lists. One-line controls stay in the row.

*Checks:* S8 (the row shape and the section heights), S5 (the marks).

### R2 — the item in the pane

The row's press opens the item's card: full-height, tabs stood down,
fields + provenance + revert + Save/Cancel, the same write ops, the row
refreshed on success, Escape/focus/dirty-close, and the item route.

*Checks:* S9, S7.

### R3 — the budget and the review-only defaults

The row budget for cron and rules with the count named; tools and env
names as lists whose rows open **read-only** cards.

*Checks:* S8 (the budget and no card above the target).

### R4 — the filter

Search across every section, the rail narrowed to the owning sections,
the chosen item's card opened.

*Checks:* S4.

### R5 — the measurement and the crossings

A Chromium probe quoting the before/after (18,448 px / 678 controls →
the section heights), the docs, and the two crossings from the
onboarding's cards to `#/settings/models` and `#/settings/trust`.

*Checks:* S10, and the doc updates.

## Acceptance criteria

- **[x] S1 — one section at a time.** Only the open section's cards and rows
  exist in the DOM: interactive controls at once drop from 678 to the
  largest section's own count (target ≤ 80). Harness check over a fixed
  settings document.
- **[x] S2 — the rail is the work rail's idiom.** Collapses to a 48 px icon
  strip, survives a reload, `aria-current` marks the open section, arrow
  keys move through the rows. Harness checks on the handlers and the
  persistence key.
- **[x] S3 — a section is an address.** `#/settings/<section>` opens it; bare
  `#/settings` resumes the remembered one; a reload keeps it; an unknown
  segment falls back to `models`. Harness checks on the route.
- **[x] S4 — the filter finds across sections.** Typing a knob's label
  narrows the rail to its owning section and opens the item's card.
  Harness check using a known key.
- **[x] S5 — the rail says what is worth knowing.** Counts on the rows that
  have them, and a changed mark on any section holding an override, both
  from the document. Harness checks.
- **[x] S6 — no new chrome.** No new token, no literal colour, no font
  outside the declared set. The harness's token assertion extends to the
  settings rail.
- **[x] S7 — the write path is unchanged and lighter.** The ops and the
  document are untouched; a write repaints its own section (and the row),
  not the board. Harness check.
- **[x] S8 — the height, measured.** On arrival no section exceeds ~1,200 px
  and no card exceeds ~700 px unopened: one-line rows with the item in
  the pane for providers, tiers and every multi-field item; a budget with
  a named count on cron and rules; tools and env as read-only rows.
  Harness checks on the row shape and the budget, plus a Chromium probe
  quoting the heights.
- **[x] S9 — the item is addressable in the pane.** The press opens the card
  (tabs stood down), the card posts the same op, the row re-renders on
  success, Escape closes and returns focus, closing with unsaved edits
  asks first, and `#/settings/<section>/<item>` opens both. Harness
  checks on each.
- **[x] S10 — the gate.** fmt, clippy, build, test with exit codes read
  directly; the harness green.

## Files

- `src/gui/assets/index.html` — the settings surface as rail + content
  column.
- `src/gui/assets/app.js` — the section declaration and rail render; the
  eight section painters; the row renderers; the item card and its
  writes; the filter; the section and item routes; `sectionNav` retires;
  the onboarding's two crossings.
- `src/gui/assets/app.css` — the settings rail (the work rail's geometry
  reused), the rows and their marks, the pane card's full-height mode,
  the filter.
- `tests/face/harness.mjs` — S1–S9, and the existing settings assertions
  re-pointed at the rail.
- `docs/features/gui.md` — the settings surface's structure, the two
  addresses, and what stays inline versus the pane.
- `FUNCTIONAL-SPECIFICATIONS.md` — the settings surface as sections and
  items.

## Out of scope

- **The settings document's shape.** The bridge's read and the write ops
  stay exactly as they are — this is the surface, not the contract.
- **Merging with the global surfaces rail.** Two navigations at one
  scale would be worse than one long page.
- **Change history or diffing beyond the changed mark.** A follow-up if
  the mark proves useful.
- **A global compact/detailed density switch.** One visual language, not
  two; density is per-item, decided by what the item is for.
- **A ninth section.** The eight are what the document carries; a ninth
  arrives with a document field.

## Verification

Per increment, the harness carries its criteria against a fixed settings
document; the gate runs whole with exit codes read directly after every
increment. The live pass is the numbers: the probe measures the section
heights and the controls-at-once count on this machine before and after,
and the plan is not done until the arrival heights meet the target — the
18,448 px page and 678 controls are what the revamp has to move.

## Landed 2026-10-30

R0 landed as `ba14abe`, R1–R4 as `82a25ff`, R5 with them. Measured on
this machine: the board 18,448 px / 678 controls → the deepest section
1,198 px with 21 controls on arrival; every section under the 1,200 px
target; the models section 5,710 → 1,112 px after the rows.

The rail is `sectionRail(declaration, options)` — the shared helper the
bots plan consumes — and its row is the app rail's own metrics (34 px,
10 px gap, 8 px radius, 13 px/500, the accent bar at the left edge for
the current section), declared once rather than copied.

Five defects found by measuring and fixed: `.stage-board`'s later
`display:block` beat the split grid; the section body took one column of
the board's twelve; `HTMLCollection` has no `forEach` (the budget's
first press threw); the three budget caps appeared in two sections; and
the servers card's scan/import/toggle left with the cards until the row
model re-homed them in the pane.

Pinned by the `SET the settings rail carries one section at a time`
harness case (twelve checks: the eight rows, one section, the row
metrics, the counts, the section route, the one-line rows, the card in
the pane with the tabs down, the write through one op, the filter).
