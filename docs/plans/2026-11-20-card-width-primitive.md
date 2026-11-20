---
status: draft
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
