---
name: color-mix-tokens-freeze-at-root
description: "feedback 2026-10-28 — a custom property whose value is a color-mix() of theme tokens, declared at :root, computes against the light palette once and inherits that resolved value into every theme; and Chromium reports the UA ButtonFace (#efefef) as the computed background of a transparent <button>, which a contrast walker reads as a real paint"
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

Building the bot-PWA mockups, two CSS facts cost four bad rounds of
"dark theme is broken" diagnosis. Both are worth recognising on sight.

**1. Derived custom properties must live where the theme lives.** The
theme was scoped onto the device element (`.phone[data-theme="console"]`)
so the review page kept its own colours. `--seat-bg` and `--bubble-bg`
were `color-mix(in srgb, var(--accent) 7%, var(--surface))` **declared
at `:root`** — a custom property's `var()` is substituted where it is
declared, so they resolved once against the paper palette and inherited
that frozen light tint into console and graphite. Symptom: a lavender
card carrying the dark theme's near-white text (≈1.0:1). Fix: declare
the derived tokens on the element that carries the theme, and give the
device `color: var(--ink)` so descendants inherit the themed ink instead
of the page's.

**2. Chromium's ButtonFace lies about transparent buttons.** For a
`<button>` with `background: transparent`, `getComputedStyle().backgroundColor`
returns `rgb(239, 239, 239)` — the UA's ButtonFace — while the painted
pixels are the parent's. Any contrast checker that walks up for "the
first opaque background" picks that grey and reports failures that do
not exist (an inbox row at 1.07:1 that renders perfectly). Also set
`appearance: none` globally on `button`: without it Chrome paints that
face behind controls that meant to be transparent (chevrons, overflow
dots). Ignore `#efefef` specifically on `BUTTON` in the checker, and
verify the suspected colour against a screenshot before chasing it.

**Why:** both announce themselves as a design defect in the frame that
looks wrong, and the actual cause is a resolution order or a reporting
artifact — so the fix lands far from the diagnosis unless it is made.

**How to apply:** in any themed CSS, put every `color-mix()`/derived
token on the themed container, never on `:root`; when a computed-style
check disagrees with a render, believe the render and fingerprint the
reporting value. Related:
[[verify-what-renders-not-what-commits]],
[[a-media-query-adds-no-specificity]] (a specificity trap from the same
family, caught the same way).
