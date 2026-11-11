---
name: fleet-text-size-belongs-to-the-reading-surface
description: "2026-10-23 (KI-307, c96d729 + 10b8bd6) — I built the owner's \"font size\" ask on the composer field (25a93f8) and they corrected it within the day: \"what I really wanted was the font size of the chat above the composer\". The conversation carries an 85–170% MULTIPLIER (--chat-scale on the log) so a reply, its headings and its code keep their ratios; then the run's own telemetry — thought rows, command cards and their detail, receipts, the run line — joined it, while card furniture and everything outside the t…"
type: feedback
date: 2026-11-04
basis: confirmed-by
tier: durable
shape: []
phrases: []
missing: []
recall-count: 29
superseded-by:
valid-until:
---

Owner, 2026-10-23, the same day the first version shipped: *"before we engage in
that, I probably misspoke with the font size capability, what I really wanted was
the font size of the chat above the composer."*

**The correction, not the build, is the lesson.** The request was "font size on
the composer… but only there, as the other text is perfect". I read it literally
and put the control on the textarea — a real, tested, gated, live-verified
feature (KI-307 as first written) that solved the wrong problem. The owner had
named the *neighbour* of the surface they meant. When an owner asks for control
over a small component "and not the rest", the rest to check is whether the
component is a *field* (where they type) or the *reading surface next to it*
(what they are looking at). A reading request wants the thing being read.

**The mechanism that survived: a multiplier, not a pixel step.** Text in a reply
is not one size — the prose is 15px, its headings 18/16/14.5, its code 12 — so a
single px value per element would need an override per selector and would drift.
`--chat-scale` is set once on the transcript's scroller (`#log`) and read by
`.owner`, `.assistant`, and every `.assistant .md-*` size as
`calc(<base>px * var(--chat-scale, 1))`; the scale is six steps 0.85 … 1.7,
default 1, kept as an **index** in `localStorage["fleet-chat-size"]` clamped on
read (an index, not the number, so the scale can change without a stored value
landing between steps). **A new element inside the transcript inherits the factor
only if you also author it as a calc** — that is the one maintenance cost, and
the harness pins the two that matter (`.md-p`, `.md-code`).

**The column moves with the type.** At 170% an 820px column would hold about 40
characters. `.stream` is `min(calc(820px * var(--chat-scale)), calc(100% - 64px))`
so the measure stays roughly constant in characters — measured, the stream is
820px at 100/130/170% and 697px at 85%, while the log stays 884px.

**Prove the scope with computed styles, both ways.** Live at 1440 on a resumed
conversation: reply 15 → 19.5 → 25.5 → 12.75px across 100/130/170/85%, the
owner's bubble 14 → 18.2 → 23.8 → 11.9px, and **the composer stayed 14.5px and a
thought row 11px at every step**. Exactly as in the composer-only version, a
narrow scope claim needs a measured pair, not an eyeball.

**What stayed out, then came in on request.** The composer field, the run
strip, the conversation head, the rail and the panels stay out permanently. The
activity rows — thought rows, command cards and their expanded detail,
receipts, the run line — were first left out as a judgement call (11–12px run
telemetry, not prose); the owner's next message was "yes, let's do the telemetry
part as well" (`10b8bd6`), so they now carry the factor too, while a card's own
furniture (its 20px metric and uppercase caption) and the glyphs stay measured.
Naming the boundary as a revisit-able choice is what made the follow-up a
one-line decision; the harness pins both sides of the text/furniture line.

**Two smaller lessons.**
- The obvious control home was unusable: the composer's footer row is already
  full at its 794px fixed width (see [[fleet-work-column-follows-the-rail]]), so
  the press pair went into the conversation head instead — where the head was
  already the "you are here" row.
- The first harness assertion was wrong, not the code: a stored index of `4`
  lands at 150%, which is **not** the top (1.7), so "the top dims its own press"
  failed. A bound assertion must use the actual bound — `99` for the clamp, and a
  middle step asserting the way up is still open.

Delivery: `c96d729`, gate 1512 passed / 1 ignored, harness C7 rewritten (16
checks) in place of the composer case; KI-307 amended rather than duplicated, so
the row carries the capability, its wrong first home and the corrected mechanism.

Related: [[fleet-work-column-follows-the-rail]] (its KI-307 section describes the
superseded composer build and the 794px footer measurement),
[[gui-client-verified-by-a-vm-harness]] (the stub needs `style.setProperty` to be
readable for this class of check), [[gui-composer-width-is-not-the-windows]].
