---
name: the-screenshot-is-the-visual-review
description: "feedback 2026-10-29 — a harness-green build can still render the wrong screen; LOOK at a Chromium render of the real page after every state-dependent change, and 2026-10-30 added the dimension that hid a shipped control for an evening — render at the OWNER'S viewport height (700px, not 900) and at the origin HE installed from, then measure a control's top against the fold, not just its presence"
type: feedback
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

2026-10-29, the onboarding door: the face harness was green at 365
checks, the door's eyebrow/title/field/press all asserted — and the
**screenshot showed the full power-user welcome underneath it**. One
`data-door` attribute plus a CSS hide rule fixed it; the next screenshot
caught the mirror bug — that rule also hid the door's OWN disclosure.

**Why:** a DOM harness verifies the elements a case looks for; it
cannot see what a person sees — leftover blocks, or a control that is
present, correct, and **below the fold** ([[verify-what-renders-not-what-commits]]
applied to one's own build).

**The 2026-10-30 return, which cost an evening:** the bots pairing row
was real, shipping, and asserted by the harness — and the owner saw
nothing, twice, on `localhost:3080`. My screenshots were taken at
**1440×900**, where the row sat at ≈747px and was visible; at his
**1280×700** window it was below the fold of a card that followed
another card. A second miss: I rendered `127.0.0.1:3080` while he had
installed the PWA from **`localhost:3080`** — different origin, and I
kept answering about the face I had tested rather than the one he was
looking at.

**The 2026-10-31 form — the first paint is the screen nobody tests.** A
new surface (the design canvas, [[design-canvas-native-plan-2026-10-31]])
passed 543 harness checks and rendered correctly — and the *first* render
showed the board at a default transform with its second frame half off the
viewport. Nothing was broken; the surface simply had no opinion about how it
should first appear, so the owner's first look was a clipped board. The fix
was two lines (fit on the first paint of an unremembered view), the re-render
proved it (both frames at 40%), and no test would ever have asked. **Render
the surface as a NEW USER sees it, not only as your fixture leaves it** — and
treat "the default view" as a design decision the render has to answer.

**How to apply:** after any state-dependent screen change, render the
real page in Chromium and READ the PNG — and render it **at the owner's
window height** (probe 1280×700 *and* 1440×900) **and at the origin he
actually uses** (ask, or use the one he named: `localhost`, not
`127.0.0.1`, for an installed PWA). Measure the control's `top` against
`innerHeight`, not just its existence in the DOM — a control that is
"in the DOM and below the fold" is, to the owner, missing. Screenshot
again after every CSS/fix rule, and write a harness check for anything
the picture revealed.
