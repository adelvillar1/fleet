---
name: conversation-density-plan-2026-10-31
description: "LANDED 2026-10-31 (5b14bd9, pushed ea1742a) — the density plan built whole: one-line tool/thought cards with ICONS type icons, the thinking streamed live (UiEvent::Thinking), the exchange summarized at the final output, the working line on the conversation with the composer back to idle words; the screenshot caught the grid body stacking the status (47→32px)"
type: project
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

Plan `docs/plans/2026-10-31-conversation-density.md`, drafted 2026-10-31
and gate-PASSed (plan-gate 12/12, arch-gate PASS), **LANDED 2026-10-31
as `5b14bd9`** (pushed with the recap at `ea1742a`) — the owner's
request and steer: *"while the turn is going the tools should
show as a single line card that I can expand and collapse, the thinking
should be a single line showing the stream and once completed should
also show as a card like the tools, once a full turn is completed all
should be summarized… the results of the request… should show as a
fully expanded card"* plus *"tools and thought should have an icon on
the card that show what they are"* and the steer *"it should not show in
the composer because it's a property of the conversation"* — the run's
live state (spinner, "working", elapsed, a seat's wait) belongs at the
tail of the conversation and the composer's hint keeps only its idle
words in every state.

**Why:** the deep dive found three facts: the tool card is
glyph+name+summary+status (the ~3 lines the owner counts) though its
detail is already collapsed by default; reasoning reaches the face as
**one event at completion** (`UiEvent::Reasoning` fires when the answer
begins), so thinking is invisible during a turn and unopened after; and
the turn mark (D8) folds manually but never summarizes — its rule was
"nothing auto-collapses, quiet by default", which the owner is now
revising for a completed turn only.

**How to apply:** increments P0–P5 — two new `ICONS` entries
(`tools`/`thinking`) rendered by `icon()` on both cards (the ASCII
`GLYPHS` letters leave the card heads); `UiEvent::Thinking(delta)` rides
the reasoning deltas (the `Token` precedent) into one live line replaced
by the collapsed thought card; the collapsed tool card is one line
(icon, tool, status) with everything else one press away; at
`closeAssistant` the current turn's tool/thought rows fold through the
**existing** `toggleTurn` machinery (never a second folding mechanism)
and the mark counts `turn N · M tools · thought Ns`; and P4 moves the
working line to a `working-live` element at the conversation's tail
(ticked by the same `busyTick`, the seat's wait swapping in), with
`paintComposer`'s working/seat branches and the `busy` class removed.
Nine criteria R1–R9 (R9 asserts the line's presence AND the composer's
silence); the harness case prefix is `TURN`. Related:
[[fold-count-is-the-freshness-measure]] (the display-count family),
[[a-dated-ruling-supersedes-a-pinned-tests-reason]] (the revised rule
lives in the new plan's Decisions).

**Landed with two catches the plan did not predict.** (1) The build
followed the plan literally — fold at `closeAssistant` — but that
function also fires mid-turn on every `tool_call`; the fold moved to
the `finished`/`failed` site guarded by `!face.replaying`, so a replayed
history never folds for the first time (R7) while a live run's end
does. (2) The live probe measured the tool cards at **47px — two
lines**: the card body is a `display:grid`, so the status span stacked
UNDER the tool name; the body is now a baseline flex row (title
truncating, status `margin-left:auto`) and the cards measure **32px —
one line — at 1440 and 1100** on five real tool rows. 509 harness
checks never saw it; the screenshot did. Gate: fmt/clippy/build 0,
1,848/0/5 (`--lib` 1,657), harness 509 checks / 54 cases (the D1 case),
the wire pin `the_thinking_delta_shape`. The recap
(`docs/recaps/2026-10-31-the-conversations-density.md`, `ea1742a`,
pushed) also flipped the quota plan's frontmatter `active → completed` —
its work had landed at `8b3564b` with the status left behind; check the
frontmatter of a plan whose Landed section exists.
