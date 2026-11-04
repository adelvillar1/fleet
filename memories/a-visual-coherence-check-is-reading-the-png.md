---
name: a-visual-coherence-check-is-reading-the-png
description: "feedback 2026-11-01 — a mockup's numbers and layout only cohere when the rendered PNG is read at full size; the state pass found truncated placeholders, flat pane spacing, cards touching the head, and copy inventing a value, none of which any assertion would catch"
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

Building the settings-surface mockups (2026-11-01) I verified
programmatically first — no console/page errors, no horizontal
overflow at 1480/1100/860/420, all 39 rows opening a card — and the
rendered pixels still carried four defects an assertion cannot see:
the pane's label/value pairs stacked flush (no card gap), the
confirmation card sat ABOVE the page head because its slot was the
first child, the key placeholder was truncated by its own save
button, and a refusal string I had typed invented "0.90 is not" — a
value nothing in the state was looking at. A second finding from the
same pass: the mock's own "day so far" card contradicted the row
marks above it ($0.03 spent against a $0.02 cap while the row said
"$0.01 today") — invented numbers must agree with each other.

**Why:** every check I ran was structural or numeric (counts,
overflow, errors), and each defect is a *reading* defect — it needs
the image, at size, against the product's own typography and
spacing.

**How to apply:** for a mockup or any state-dependent screen, shoot
each state to a PNG and READ it — at least the default, one
alternate treatment, one refusal, and one narrow viewport — then
re-read after each fix, because a CSS change moves what you already
approved. Same rule as [[the-screenshot-is-the-visual-review]],
extended to mockups: coherence (do the numbers agree, does the
element sit under the element it belongs to) is only visible.

**2026-11-02, the gallery shoot of the LIVE product (not a mockup):**
the same read found two shipped defects — every act on a selected
page's title bar was dead (a drag gesture's `preventDefault` ate the
button's click; KI-394) and a select clipped its selected option
mid-word (fixed-width knobs sharing a row evenly). Both were green in
the face harness, which drives listeners directly with no default-action
suppression, so **a live browser press is the only proof an act is
pressable**. And the inverse discipline: when a shot looks wrong,
re-take it in a FRESH page before calling it a defect — one page's
sequence of screenshots produced a pane-less frame that a fresh load
rendered perfectly, and I spent a long detour diagnosing a ghost. The
owner's "why is this task taking so long?" is the tell that the read
had stopped paying for itself.
