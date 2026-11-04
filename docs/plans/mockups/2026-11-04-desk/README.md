# Desk — where knowledge work happens

Five screens, drawn before any code (the mockups-first ruling). Plan:
[`docs/plans/2026-11-04-desk.md`](../../2026-11-04-desk.md).

The palette is `app.css`'s, hand-copied (the huddles set's `mockup.css`,
shared verbatim): the reference scale, the alias tier and the component
tokens keep their names and values, so the implementation is a
translation and not a reinvention. Every screen links one `mockup.css`
and carries no build step; the theme switcher in the corner sets
`data-theme` and writes `?theme=` so a dark screen is the same file.

| screen | what it decides |
|---|---|
| `desk.html` | The desk shelf: the split (the desk rail by project, the shelf, what the desk owes you), rows carrying the binding, the claim, the evidence status (asserted / contested / open refusals), states active / review / ended. The surface's default state. |
| `brief.html` | The brief composer: one composer, three fields (name, audience, the claim), the binding picker, the sources field with four labeled kinds (file, fact, url, project knowledge), a captured URL's provenance in plain words, and the injection framing at the seam. Applies on submit. |
| `deliverable.html` | The deliverable with its stage rail (claims / outline / render / review / knowledge, each state visible), the outline with a **named gap** rendered as a gap, the composition's origin references (frame, huddle, session — a reference is not a copy), and the produced deck in the pane with its check chips. |
| `review.html` | The review view: claims beside the sentences they were read from, verdict chips with the floor's confidence, the unsourced-assertion refusal naming its near miss and overlap, the vision pass on the pixels, and the end action refused while a refusal stands. |
| `knowledge.html` | The knowledge moment: the deterministic digest, where it landed (memory lifecycle facts, the corpus seed, the bounded assembled-context line), what a skill finds (the shape routed the methodology), the distil action. |

## What it is, and is not

The desk is a **place** — sources on one side, the draft in the middle,
the marked-up review on top — in Fleet's **paper**: serif deliverable
names, the warm page, mono for times, paths and confidences. It is not
a file manager, not a rich-text editor, not a second design surface.
Desk is the surface; a **deck** is one artifact it produces — the words
stay distinct on every screen.

## How it was verified

Rendered in Chromium at **1440×968** and measured in all four themes
(paper, console, graphite, contrast) — twenty screen-theme combos:

| fact | target | measured |
|---|---|---|
| horizontal overflow | 0 | 0 on all twenty |
| elements past the right edge | 0 | 0 |
| clipped text (non-ellipsis) | 0 | 0 |
| rail rows | ≥34px (the app's rail row) | 38px |
| primary and secondary presses | ≥30px | 36px |
| chip ink on its surface (accent / ok / down / warn, the pairs used) | ≥4.5:1 | 4.86 – 7.78:1 |
| disc mark ink (surface on the 68% lane mix, the pairs used) | ≥4.5:1 | 6.87 – 11.72:1 |

The raw lane inks fall under the bar in paper and contrast (3.17 – 4.32)
— which is exactly why nothing uses them as text; the discs mix them
68% with ink, the huddles set's own fix.

And the read: every screen was read as a PNG, in paper and in the dark
themes, and the read caught what the measurement called clean — the
first brief render drew its fields inline because the imported sheet
already owns `.field` (the huddles composer's), and the overflow pass
had nothing to flag. The composer's classes are desk-prefixed now; the
lesson is the 2026-11-04 one, again: measure AND read.
