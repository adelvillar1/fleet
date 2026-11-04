---
name: a-media-query-adds-no-specificity
description: "feedback 2026-10-20 — responsive overrides written as bare class selectors lose to state rules like body[data-aside] .face, so the narrow layout silently never applied; and narrowing passes tested only at desktop width, plus per-row audits that accept a descendant button"
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

Three verification traps, all found by rendering the shipped Bots/sessions UI
in Chromium against a seeded scratch home ([[fleet-e2ba136ae3eba04b]] session
2026-10-20):

**1. A media query adds no specificity.** `@media (max-width: 900px) { .face {…} }`
never beat the base sheet's `body[data-aside="closed"] .face` — attribute plus
class beats a single class, media query or not. The grid stayed two-track and
the stage (assigned to track 1 by the same block) rendered in the rail's 236px
column beside 664px of empty canvas. Every panel, not only the new one. The
fix answers each state at its own weight:
`:is(body[data-rail], body[data-aside], body[data-dock]) .face` and the
two-attribute combinations.

**2. "Verified live" at desktop widths is not verified.** The port's own pass
ran at 1440/1600 and read green while the phone layout — the layout the remote
face exists for — was unusable. Render at the widths the feature is FOR, not
the widths the harness happens to use. On a fresh page per width: resizing one
page can carry boot-time state and hide the bug.

**3. An audit that greps only leaf elements misses the parent object.** A
"no dead rows" rule that looks for a button inside a row passes while the row
itself — the thing an owner clicks — is inert, because the badge or button
beneath it satisfies the check. What surfaced it: a screenshot with refusals
and state lines that looked like rows and did nothing. Audit the object the
owner aims at, at every width, and watch for rows whose only content is text.

**Why:** a narrow-layout override that loses on specificity fails silently —
the CSS is present, the page loads, the console is clean, and "no overflow" is
true because the layout is simply narrower, not broken.

**How to apply:** when writing a responsive rule, mirror every state selector
that comes after it at equal or higher specificity (or use `:is()` to collect
them). When verifying a surface, always include the smallest width the design
promises and do it on a fresh load. When auditing interaction, check the
container as well as the leaf. Related:
[[verify-what-renders-not-what-commits]], [[gui-client-verified-by-a-vm-harness]]
(Playwright is the truth, the stub is not).
