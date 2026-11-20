---
status: active
created: 2026-11-20
updated: 2026-11-20
slug: press-feedback-primitive
---

# A press either acts, says why not, or is disabled

KI-442 (an empty submit did nothing), KI-556 (a tier card's arrows did
nothing while looking enabled), KI-563 (the same arrows, still live) and
KI-561 (a card that told the owner the wrong thing) are one class: a
control that looks live and produces no visible consequence. KI-561's fix
already found the right shape; it landed on one card.

## Context

- The sweep that found KI-563 drove **27 settings cards and 40 presses**
  and found only five silent ones — a cheap, repeatable procedure that
  lives in one session's notes, not in the harness.
- KI-561's fix shape (from the ledger): the answer line remembered in
  `face` state rather than painted into a node the repaint rebuilds, a
  busy word for the sequence, the press disabled while in flight, and the
  test asserting **what the owner is told** — not that a request was made.
  The harness case at that row's time asserted only "one POST happened",
  which is why the broken feedback path stayed green.
- The harness already has the machinery: `tests/face/harness.mjs` drives
  ~188 cases / ~1,303 checks.

## Status pass, 2026-11-20

Landed: `pressFeedback(key, role)` hoisted to the app's top level — the
answer line (`role="alert"`, remembered per key so a repaint cannot wipe
it), `say(text, bad)`, and `hold(press, label)` for the in-flight window.
Adopted on three cards that mutate state through a press: the provider
reload (KI-561's own card, whose local shape it now replaces), **reload
them all** (which had no busy word and no disabled window — the silent
sequence), and the connect card.

Worth recording: the first attempt hoisted the helper *inside*
`settingsSections`, so sibling cards could not see it, and one replacement
landed on the connect card instead of the reload-all card. The harness
caught both (`pressFeedback is not defined`, `fb is not defined`) — 206
cases earning their keep.

**Still owed:** AC1's sweep (which cards mutate state and do not use the
helper) and AC2's harness case that drives every control of every settings
card and fails on a silent press.

## Approach

1. Extract KI-561's card shape into one small helper in
   `src/gui/assets/app.js` (answer line + busy word + disabled window),
   and adopt it on the cards KI-561/562 already touched (the
   provider-reload single and reload-all cards are explicit in the
   ledger's open rows).
2. State the rule where the next author reads it: a control either acts,
   paints a named reason it cannot, or renders disabled. Put it beside
   the helper's definition and in the settings feature doc.
3. Extend the harness sweep from a one-off procedure to a case: for every
   card in the settings surface, press every control and assert the
   document changed (state, notice, or disabled) — a no-op press fails,
   and the failure names the card and the control.
4. Red-proof: re-create KI-563's last-row down-arrow (the handler that
   returns without a message) in a scratch run and confirm the sweep
   fails naming it; remove.

## Acceptance Criteria

- [ ] One helper renders the answer/busy/disabled shape; every card that
      mutates state through a press uses it (the sweep enumerates them).
- [ ] The harness case drives every control of every settings card and
      fails on a press that changes nothing and says nothing — red-proved
      against KI-563's shape.
- [ ] The harness asserts what the owner is told (the answer line's text)
      for the provider reload card, not just that a request was made
      (KI-561's lesson, pinned).
- [x] The settings feature doc states the rule in one paragraph
      (`docs/features/gui.md`, "The press rule").
- [ ] `cargo test` and the harness suite exit 0. The harness is green on
      the extraction (**1480 checks, 206 cases**); `cargo test` on this tree
      is still to run.

## Files to be touched

- `src/gui/assets/app.js` — the helper and its adopters
- `tests/face/harness.mjs` — the sweep as a case
- `docs/features/` — the settings doc's rule paragraph

## Out of scope

- Restyling cards or the drawer.
- The TUI's key handlers (terminal surfaces have their own feedback
  shape; the card-width plan covers their clipping).
- KI-562's own fix if a separate session lands it first — this plan
  adopts its card into the helper when it does.

## Verification

1. The harness case, run alone.
2. The full harness suite.
3. A hand press-through of the settings surface in the live face.
4. The full four-leg gate once, at the end (the JS is `include_str!`'d,
   so the binary must rebuild).

## Linked artifacts

- KI-442, KI-556, KI-561, KI-562, KI-563 — the rows behind the rule.
- `docs/architecture/deep-dive-2026-11-20-fleet-state.md` — F3.3.
