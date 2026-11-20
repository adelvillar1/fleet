---
status: active
created: 2026-11-20
updated: 2026-11-20
slug: card-width-primitive
---

# One width-aware card builder, and a property test that holds it

KI-078 and its three recurrences are one class: a fixed-width surface is
handed a line built by concatenation, and a fixed pane cuts it mid-token.
Each instance was fixed before commit; the class was never stopped.

## Context

- The ledger carries KI-078 plus three explicit `RECURRENCE` rows
  (compaction card, cron fire card, judge batch card), each fixed by
  rebuilding that one card in the two-line shape another card already
  used.
- A width guard exists — `CARD_WIDTH` at `src/calib.rs:3292` — but the
  ledger's own post-mortem says it "never fired because its fixture
  carried a long provider with a four-character shape": the guard pins
  fixtures, not builder shapes.
- `unicode-width` is already a declared dependency (`Cargo.toml`, added
  for KI-025), so display-cell measurement is available without a new
  crate.

## Status pass, 2026-11-20

Measured before writing: the plan expected four surfaces drawing at the
house width; the tree carries **eighteen** identical copies —
`const CARD_WIDTH: usize = 72;` in `calib`, `cron`, `delegate`, `demand`,
`desk`, `doctor`, `eval`, `explain`, `graph`, `improve`, `judge`, `lanes`,
`mcp_registry`, `memory`, `route`, `session_recall`, `settings`, `skills` —
all agreeing on 72, which is what made the sweep safe.

Landed: `src/cards.rs` holds `CARD_WIDTH` and `CARD_INNER` (`CARD_WIDTH - 4`,
the border and padding), every one of the eighteen now imports it, and the
census from `2026-11-20-guard-list-inversion.md` carries the guard: a module
that declares its own `const CARD_WIDTH` fails
`cards::tests::the_house_width_has_one_home` by name. The full lib suite is
**2435 passed / 0 failed**; the card surfaces' own width assertions (20 of
them) were unchanged and every one still passes.

## Status pass, 2026-11-20 (second)

The measuring and fitting primitives are in `cards`, each with its own test:

- `width_of` — display cells, ANSI colour stripped, CJK two cells, combining
  marks none. Every `chars().count()` width check in this crate was wrong for
  a CJK name, which is how KI-140 stayed green.
- `fit(text, width)` — wraps, never truncates; hard-splits a word that cannot
  fit. Property-tested at **every width 20..120** against a 90-character id,
  CJK, emoji and combining marks.
- `clip(text, width)` — for a field that cannot wrap, ending in `…` so the
  loss is visible.

First builder converted: `calib::op_card`. Its detail lines now wrap through
`fit`, and its header clips to half the card. The card-level test is the
red-proof the plan asked for: before the fix, `op_card` with a 120-character
op name rendered a **133-cell line against a 72-cell card**; after it, every
line fits.

Measured inventory for the remaining work: four card builders in `calib`
(`op_card` done; `spend_card`, `stream_card`, `calibration_card` to go) plus
the cron, judge and delegate cards. `dashes_for` measured characters, not
cells, and is fixed.

All four `calib` builders are converted now: `op_card` wraps its detail and
clips its header, `calibration_card` wraps the five body lines that carry a
variable (`{line}`) rather than a number, and `spend_card` clips the one
unbounded field on its row — the provider id. `stream_card` needed nothing:
every line it draws is a count or a fixed phrase.

The rule moved too. `"\u{2500}".repeat(n)` was spelled in **45 places across
17 modules** — the plan expected four builders in `calib` plus cron, judge
and delegate; the tree had seventeen modules computing how much box to
draw. All 45 call `cards::rule(width)` now, and
`the_rule_has_one_home` refuses a new one: red-proved by adding a stray
repeat to `desk.rs`, which the guard named — `["desk.rs"]` — before it was
removed.

Judge and cron are converted too: the judgment card wraps its body lines and
clips its title to half the card, and the cron card clips the job name the
same way (its task already wrapped through `skills::wrap_detail`). Delegate's
frame is converted too — body wrapped, title clipped, its three warning rows
and two dim rows wrapped — so all four card families in the tree measure in
cells.

Remaining: a family-level test per frame (the `calib` card has one; judge,
cron and delegate are proven by their existing suites plus the primitives'
property test, not by a test that hands them an over-long field), and AC1's
wording ("every card builder renders through the one helper" — the border
does, the row shape is still per-family).

**A bug this plan's own test missed, found by judge's:** `fit` measured each
word by summing `UnicodeWidthChar` over its characters, so an ANSI colour
sequence counted as visible text and a line that fitted wrapped anyway —
judge's card tests caught it as two broken layouts. `fit` is escape-aware
now, the property test carries a coloured fixture, and
`wrapping_carries_colour_through` asserts a sequence is never split.

## Approach

1. Inventory every fixed-width card builder: `grep -rn 'CARD_WIDTH\|repeat("─")'`
   over `src/`, plus the TUI's box-drawing sites in `src/tui/ui/`.
   Expect calib (4 cards), cron (fire/catch-up), judge (batch/disposition),
   delegate. Record the list in this plan's Status pass.
2. One builder: `card(width, head, rows)` (name per the nomenclature
   check, not invented here) that (a) measures in display cells, (b)
   wraps rows rather than truncating them, (c) caps each field at a share
   of the width with an explicit ellipsis, and (d) is the only way a
   fixed-width card is rendered.
3. Property-style test: for each card builder, render across widths
   20..120 with adversarial fixtures — a 90-character provider id, CJK,
   emoji, combining marks — and assert no rendered line exceeds the pane
   width in display cells. Run the same fixtures against the pre-change
   builder to record the red (the KI-078 shape must fail).
4. Replace each builder one card at a time, each replacement its own
   commit, so a regression names its card.

## Acceptance Criteria

- [ ] Every fixed-width card builder in the inventory renders through the
      one helper; a text-scan guard refuses a new `repeat("─")`-style
      separator outside the helper.
- [ ] The width property test covers every inventoried card, at widths
      20..120, with the adversarial fixtures; the pre-change builders
      fail it (red-proof recorded in the plan).
- [ ] KI-078's own fixture (the 24-char provider id beside a 12-char
      disposition) passes at 72 and at 80 without the width guard's
      fixture blind spot.
- [ ] No card's existing content is dropped: the before/after render of
      each card at width 72 is compared line-by-line in the commit.
- [ ] The full gate exits 0.

## Files to be touched

- `src/calib.rs` — the four card builders
- `src/cron.rs`, `src/judge.rs` (or its card module), `src/delegate.rs`
- `src/tui/ui/mod.rs` — box-drawing sites if the inventory names them
- `docs/architecture/structure-guards.md` — the new guard's entry

## Out of scope

- The face's HTML cards: they wrap via CSS and are a different class
  (KI-078's recurrences were all terminal surfaces).
- Card content and ordering — this plan changes only how a line is fitted
  to a width.

## Verification

1. Scoped: `cargo test -p fleet --lib calib` / `cron` / `judge` while
   iterating.
2. The property test, run alone and in the suite.
3. The full four-leg gate once, at the end.
4. A hand run of `fleet calib` / `fleet cron` / `fleet judge` at an 80
   and a 120 column terminal to confirm the visual result.

## Linked artifacts

- KI-078 and its three recurrence rows in `docs/known-issues.md`.
- `docs/architecture/deep-dive-2026-11-20-fleet-state.md` — F3.1.
