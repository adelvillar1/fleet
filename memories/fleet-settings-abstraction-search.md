---
name: fleet-settings-abstraction-search
description: "2026-10-23 the owner deferred settings simplification (\"I haven't found the right abstraction yet\"); 2026-10-29 he supplied it across five planning turns (the rail, card density, the pane idea, the whole rewrite, sectionRail extraction); the build then SHIPPED 2026-10-30 (S1–S10 green — the plan's status is completed); on 2026-11-01 the owner called the shipped UX \"atrocious\", launching a NEW research+plan layer above the bones — see [[settings-ux-plan-2026-11-01]]"
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

Owner statement, 2026-10-23 (end-of-session reflection, in the context of
[[fleet-anti-hermes-constitution]]): "I will simplify the settings, I just
haven't found the right abstraction yet." Simplification was deliberately
deferred until the abstraction was found — no deletion-based simplifying, which
would produce Hermes' complexity with fewer features.

## 2026-10-29 — the owner found the abstraction: the work rail's pattern

The owner's own words: "a revamping of the settings page, we should use the
same rail approach we are using with the work rail, a collapsible rail with
icons for the sections and each section presents only the items for that
section … usability and reducing friction is the overarching goal."

**Measured before planning** (Chromium, 1440×900, `#/settings` on this
machine): **13 cards on one board** (router, providers, lanes, MCP servers,
tools, roles, vault, spend caps, other knobs, remembered rules, cron, identity
slot, env counterparts), **18,448 px of page**, **678 interactive controls in
the DOM at once**, and the 11 "section" chips only `scrollIntoView` — a chip
jumps you into a page you then read past, and nothing says which card you are
in. The document behind it is small: 23 scalar knobs, 7 list overrides
(6 workload, 1 provider), 24 rules, 15 cron jobs, 9 env names, one SOUL slot.

**The plan (docs/plans/2026-10-29-settings-rail.md, `draft`, not built)** —
eight sections in the owner's language: models (router + providers + workload
overrides), specialists (lanes + MCP servers + tools), roles, trust (vault +
approvals + remembered rules), money (caps + budgets), schedule (cron),
interface (UI knobs + SOUL), environment (env names + remaining knobs). The
rail reuses the work rail's geometry: `icon()` glyph rows with counts, fold to
a 48 px icon strip, remembered choice, `aria-current`, arrow-key roving. A
section becomes an ADDRESS (`#/settings/<section>`, bare resumes the last), so
other surfaces' actions can deep-link — the onboarding's "change it" →
`#/settings/models`, "seal your keys" → `#/settings/trust`. A filter over all
sections answers "where is the budget" without knowing which of eight owns it.
Writes keep the same ops but repaint only their own section.

**Two things the plan had to be corrected on while checking the code** (worth
keeping — they are why the plan is cheaper than it looked):
- The "changed" mark on a rail row needs no new read: a scalar carries
  `source` (`file`/`env` vs the compiled default), a list override carries
  `reverts` (an override is revertible; a seed is not), and rules/cron/servers
  are themselves overrides.
- Nothing in the document or the ops changes — this is the SURFACE, not the
  contract. Say so in the plan's out-of-scope or the revamp balloons.

## 2026-10-29, next turn — the second half: wasted space inside a card

The owner's amendment while still planning: "reducing wasted space on cards,
reduce scrolling by collapsing cards that are too long and that would only be
occasionally reviewed… let's think of approaches."

**Measured card heights (same machine, total 18,001 px):** providers 4,397
(148 controls, only 54 rows — heavy form rows), cron 3,181 (15 jobs), tools
2,065 (review-only), knobs 1,679, router 1,480 (98 controls, 4 tiers), rules
1,460 (24 rules), servers 830, vault 722, roles 491, caps 478, env 459 (zero
controls — pure diagnostic), lanes 440, SOUL 319. **The top four are 63% of the
page**, and they are two different problems: *many items* (cron, rules) versus
*heavy rows* (providers, router).

**Four approaches adopted, ranked by what they buy per unit of new state:**
1. **Row budget per list with the count named** ("15 jobs · 10 more") — no
   stored state, honest count; chosen for cron, rules, env names, servers.
2. ~~Compact rows with an inline edit disclosure~~ — **SUPERSEDED the same
   planning turn by the owner's pane idea** (below). The heavy-row fix
   stands; the *surface* changed: the row is one line (`id · endpoint · key
   set`), and its press opens the item in the right pane, not inline.
3. **Move the diagnostics out** — env counterparts (459 px, 0 controls) is a
   `fleet doctor` fact, not a setting; it becomes a disclosure in environment.
   Tools is review-only → a summary count.
4. **Default-collapsed review-only cards** using the house's `details` idiom
   (the mechanism-disclosure pattern).

**Rejected with reasons** (this list is the part worth remembering — the owner
asked for approaches, and rejecting one is a decision to keep):
- **Nested per-card scroll boxes** (max-height + inner scroll): shortens the
  page but creates scroll traps, two scroll areas in one column, worst on touch.
- **Per-card remembered fold as the PRIMARY mechanism**: a state map nobody
  asked for, and the first visit is still 18k.
- **A global compact/detailed density switch**: a second visual language for
  the same controls.

**The measured target the increment is judged by:** no section above ~1,200 px
on arrival, no card above ~700 px unless the owner opened it — the largest
offender today is 4,397. Pinned by S8 plus a Chromium probe quoting
before/after heights, because on this project a layout claim is
[[a-layout-claim-is-measured-not-asserted]].

**Why this satisfies the deferred abstraction test:** [[a-top-level-surface-needs-an-address]]
said a capability living eleven cards down a long page does not exist to the
owner — the rail resolves exactly that by giving each domain a place, one at a
time, and the card-height work makes each place readable without scrolling.
The earlier candidate framing (five questions; modes-not-sliders;
[[fleet-config-surface-is-the-settings-page]] for the shipped board) is not
what the owner chose; the *navigation plus the per-card density* is the
abstraction, not the taxonomy. Related: [[no-form-asks-for-a-value-fleet-already-has]]
still governs every field inside a section.

## 2026-10-29, third planning turn — the item opens in the RIGHT PANE (owner's idea)

The owner's amendment: "maybe we can click on setting items and bring out the
right side pane to display the details and allow edits from that pane." This
**replaced approach 2's inline disclosure** and is now the plan's main
density mechanism. It is not a new surface: the pane's object card **already
hosts editable forms** (the fact editor — `fact-edit`/`fact-save`/
`fact-field`), and `openDrawer` is how every selected object reaches it. The
measured fix it buys: providers' 4,397 px card becomes ~10 one-line rows.

The rules the plan holds it to (these are the ones a build must not drop):
- **The row is one line**: value + the marks that matter (`env`, `override`,
  a live dot). ~30 px/row.
- **The press opens the item's card in the pane**, the pane's tabs (run/
  files/preview/web) standing down while it is open; closing returns them.
- **One write path**: the card posts the same `writeSettings` ops; on success
  the row's summary + marks re-render in place.
- **The one-press rule** (or the pane becomes friction): a value settleable in
  one press — a checkbox, a 2–3-choice mode, the vault unlock — STAYS IN THE
  ROW. Only what must be *read* before writing opens in the pane.
- **Escape closes + returns focus to the row**; closing with unsaved edits
  asks first (the rewind idiom's confirm).
- **Addressable to the item**: `#/settings/<section>/<item>` opens the section
  AND the card — a setting as linkable as a fact.
- Below 1180 px the pane is already the object-card overlay, so the same press
  works with no second design.

Plan criteria moved to **S1–S10** (S8 = the measured height with rows+pane+
budget; S9 = the item-in-pane open/write/re-render/Escape/dirty-close + the
deep link; S10 = the gate). Still `draft`, **NOT built** — the owner has said
"plan first" three times on this surface.

## 2026-10-29, fourth turn — "create a plan": rewritten WHOLE, and it now owns a rail HELPER

The owner said *"create a plan to do the settings revamp."* The file had
grown by amendment (four approach sections, three competing mechanisms), so it
was **rewritten whole** rather than patched — the same rewrite pattern as
[[bot-pwa-plan-rewritten-2026-10-28]]: **Decisions (the owner's, dated and
quoted verbatim) → Approach → Increments R0–R5, each naming the criteria it
carries → S1–S10 → Files → Out of scope → Verification.** 299 lines, `draft`.

The one structural change: **R0 extracts `sectionRail(declaration, options)`**
— rows of (icon, name, badge, changed-mark), collapse-to-icons with its own
persistence key, `aria-current`, arrow roving, and the row-budget helper —
instead of writing the rail inline, **because a second surface already
consumed the idiom** (the bots page; see [[bots-page-on-the-personas-rail]]).
Two consumers are what prove it a helper; naming that dependency inside the
plan is what keeps the second build from copying the first.

Build order the dependency forces: settings **R0** first (it ships the rail),
then bots **B0** which consumes it; the rest of each runs apart.

## 2026-10-30, fifth turn — "implement both plans in sequence": the build PAUSED mid-R0

The owner said *"implement both plans in sequence"* and the build began. Written
and unverified when the goal was paused: `index.html` (a `.stage-split` rail
container on BOTH surfaces), the **shared `sectionRail(container, declaration,
options)`** + **`budgetRows(rows, limit, label, open, onToggle)`** helpers,
`knobsCard/rulesCard/cronCard/soulCard/envCard` extracted from `paintSettings`,
the eight-section declaration, `settingsSectionId()`/`rememberSection()`
(hash → remembered `fleet-settings-section` → `models`), and a
`.section-rail` CSS block. Nothing built, tested, or committed since the build
was cancelled mid-run.

**The pause came from the owner's design-language correction: "remember to keep
consistency on the design language of the app for both surfaces."** Measuring my
new `.section-rail-row` against the app's own `.surfaces-link` showed a
near-copy that had drifted on EVERY metric — height 34 px vs `padding: 7px 8px`,
gap 10 vs 9, radius 8 vs 7, type 13 px/500 vs 12 px/400, icon box 18 vs 16, no
`--fast` transition, no `:active` scale — and worst, the **current-row
indicator**: the real rail uses raised ground + an inset hairline + the 2 px
accent BAR at the row's left edge, while mine substituted an accent wash and
bold text. Two answers to "you are here" on two surfaces.

**The correction, recorded as the rule for resumes:** the section rail is not a
lookalike, it is the SAME rule — declare `.surfaces-link, .section-rail-row { … }`
and `.surfaces-count, .section-rail-count { … }` as the pane's cards became
`.rail .card, .pane .card`, with no new tokens and the harness's no-new-token
assertion extended to the rail (asserting the shared selectors exist instead of
a parallel rule). This is [[a-relocated-child-loses-the-ancestor-that-styled-it]]
(KI-373) in its third form: near-copy styling is the same defect as losing an
ancestor's rule.

**Resume order:** fold the rail CSS into the shared declarations FIRST, then
re-measure both surfaces against each other before continuing — consistency is
the first increment, not an end-of-build cleanup.

## 2026-10-30 — the build SHIPPED (the "PAUSED mid-R0" state above is stale)

The plan completed the same day: `ba14abe` (R0), `82a25ff` (R1–R4 + R5),
plan status `completed`. Measured on landing: the board 18,448 px / 678
controls → the deepest section 1,198 px / 21 controls on arrival; every
section under the 1,200 px target; models 5,710 → 1,112 px. Five defects
found by measuring and fixed (display:block beating the split grid; the
HTMLCollection `forEach`; budget caps appearing twice; the servers card's
controls leaving with the cards). The S-series (S1–S10) is all green.

## 2026-11-01 — the owner's verdict: still atrocious → the next layer

The owner: *"settings user experience is atrocious"* — despite the shipped
revamp. The diagnosis: the revamp fixed the MACRO shape (page → sections →
rows → pane) but not the layer above the bones — what happens after a press
(confirmation lands off-surface, a dead fetch is silent, no undo), what a
value means (raw numbers, no translated consequence), and how a wrong turn
is undone (destructive acts unlabeled). Deep research + ten evidence-backed
gaps + seven increments (U1–U7) landed as
[[settings-ux-plan-2026-11-01]] (plan `2026-11-01-settings-ux.md`,
`410eb4b`, NOT built). One extra find worth keeping: **S9's three-part
item route never shipped** — `settingsSectionId()` reads only `parts[1]`,
no `parts[2]` handler exists, and the harness pinned the press-to-card
rather than the route (a criterion that can pass without its own promise).
The eight sections remain the owner's ruling; the new plan names that in
out-of-scope so it is not silently reopened.

## 2026-11-01, later the same day — the steer goes DEEPER than the gaps

The plan's gaps were interaction gaps; the owner came back for the
TAXONOMY: *"what about models and providers? … it should be more like
zcode, deepseek harness, and less like hermes."* Then *"budget is not a
concern… I use coding/token plans"* and, after I kept them, *"again
money I don't care, you can remove that."* So the surface changed
SHAPE: models opens on the MODEL (every `provider/model` listed, the
live one marked, one press to switch, accounts without a `billing`
class, the workload chains + gates behind `the walk` door), and the
money section is GONE — the caps leave the GUI entirely (a fork may
add a surface; the CLI and document still write them).
**Seven sections.** This is the plan's own out-of-scope line being
overridden by the owner, which is the rule: a section's shape is his to
move; "not reopened" only blocks ME from reopening it. Recorded as
G11 + U8 (sequenced first) and U6-withdrawn at `f7681c7`, the caps'
removal at `9b246a3`; the lesson generalizes in
[[the-surface-keeps-the-machinery-and-hides-the-answer]].
