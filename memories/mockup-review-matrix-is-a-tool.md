---
name: mockup-review-matrix-is-a-tool
description: "feedback 2026-10-28 — a mockup review deliverable should be one navigable tool covering the whole matrix (state switcher, 430→900 width control, the product's real themes, state/width in the URL), verified headlessly across every cell; measuring beats eyeballing, and the caveat a tool writes about itself is not trusted"
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

Feedback from building the bot-PWA B0 mockups (see
[[bot-pwa-b0-mockups-built-2026-10-28]]). Also true of the earlier
second-brain mockup round ([[plan-brainstorming-before-development]]:
"a mockup the owner can navigate beats prose about a mockup").

**Build one tool, not four static pages.** Each mockup file carries its
own state switcher (every state the plan names), a 430→900 width control
(the reflow is container queries on the phone frame, so the mockup *is*
the shell and its own width decides the layout), and the product's real
themes — so the owner opens a folder, not a slide deck. Put state and
width in the URL (`?state=filter&w=900&theme=console`) so any frame can
be revisited or screenshotted exactly.

**Measure, don't eyeball.** A headless pass over the whole matrix —
every file-state × every width × every theme — asserting no horizontal
overflow, every control ≥44 px, every field ≥16 px, and every text
element at WCAG AA — caught five real defects in a set of pages that
looked finished in three screenshots. Generate the matrix in code and
report `PROBLEMS: none` explicitly, the same discipline as a gate.

**A redesign is every screen, not the one you were asked about.** After
the assistant-centric inversion only `inbox.html` was rebuilt; the
dispatch sheet's background list and the whole install screen still
showed the work-centric inbox — captions and screenshots looked right
while half the folder contradicted itself. Grep the old copy
(`grep -ln "3 workspaces\|class=\"ws\"" *.html`) after any vocabulary
change, and re-render every state rather than the one under review.

**Edit files whole, never splice by index.** Two `index()`-based
splices on `install.html` silently deleted an entire screen state (the
slice spanned the wrong `.scroll`); the screenshot showed an empty phone
and the automated matrix still said `PROBLEMS: none` because a missing
element passes every per-element assertion. Rebuilding the file from
scratch took one call; when a splice touches repeated boilerplate,
rewrite the file.

**When a measurement disagrees with a render, believe the render** and
fingerprint the reporting value (see
[[color-mix-tokens-freeze-at-root]] for the ButtonFace case). A checker
with no known-artifact list manufactures failures.

**Keep the checks honest across page loads:** a shared browser context
caches the stylesheet, so a stale CSS read made a later file look broken
after the source was fixed. One context per case.

**A review override can be a requirement.** "The tint needs good
readability on both themes for this experience" is not a taste note: it
is a property to compute. Use the hue as a glyph's *ink* and it fails AA
in paper (4.19–4.49), and on a translucent wash it drifts with whatever
sits behind — so make the disc opaque and derive the ink per theme to a
stated margin, then print the full ratio table (persona × theme) in the
README so the next editor can see what the design depends on. Put a
bound the design relies on in handoff notes, and widen the contrast walk
to the new elements — a token that was never checked is not verified.

**A token used but never defined falls back silently.** Giving each
persona a tint from the lane palette (`var(--lane-0…3)`) resolved to
nothing: the mockups copy tokens from the console's `app.css` but had
never copied the lane hues, so every mark silently took the accent and the
screenshots looked plausible. The same silent fallback bit again when a
persona was RENAMED (`scout`→`scholar`) and its `data-p` rule was left
behind — the ring showed the accent for one persona and the checker
reported 5.04 where the design promised 5.2. Probe the **computed** value
per variant (`getComputedStyle` per `data-p` in paper *and* a dark theme)
rather than trusting that a distinct look implies a distinct value — a
fallback is indistinguishable from a design choice in a render.

**Repeat-marker anchors are the splice trap, not the escape hatch.** A
second round of `index()`-based edits matched the *control button*
(`data-scn="filter"`) instead of the scenario `<div>`, deleted three
scenarios, and a later slice inside `install.html` swallowed a whole
screen again — ten minutes each to detect because the automated matrix
passes a missing element. When a file has repeated boilerplate (scenario
markers, list blocks, identical headers), rewrite the file whole; index
splicing is only safe against a string you have confirmed unique.

**A wide layout's fixed column needs `min-width: 0`.** The 900 px
two-pane rendered 239 px wide: a `flex: 0 0 412px` column cannot honour
its basis while a non-shrinkable child (a seven-chip scrolling rail)
holds its min-content width open. `min-width: 0` on the column and on the
scenario lets the inner overflow:auto do its job — and an overflow
assertion at every width is what caught it, since 430 px was clean.

**A frame's own width is the spec — pin it.** Adding the goal line as a
header sibling let the conversation frame shrink to 875 px when asked for
900 (`flex-shrink` on a fixed `width`), and only the width assertion
caught it. `flex: 0 0 auto` on the device frame.

**A pinned line inside a padded scroller never covers cleanly.** The goal
looked like a paint bug over the table behind it; a sticky element in a
scroller offsets from the padding box, so the last line of content shows
through its padding. Put what must never scroll away in the *chrome*
between the header and the scroller, not inside it.

**A sheet must scroll, and a new token needs every theme.** The
add-a-persona sheet's last option was clipped at 430 px (`max-height` on
the sheet, `overflow-y: auto` on its body); and the destructive button's
ink token (`--on-down`) was added to paper only, so it fell back to white
on the dark themes' light red at 3.02 — the contrast walk caught what a
plausible render did not.

**A tool's caveat about itself is not authority.** Not saving claims
made inside `tools/hooks.md` about tool-hooks behaviour without checking
— the artifact under review is the render, not its self-description.
Related: [[verify-what-renders-not-what-commits]],
[[what-was-ambiguous-ends-in-a-plan-not-a-lecture]].
