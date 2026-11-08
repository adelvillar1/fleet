---
name: a-top-level-surface-needs-an-address
description: "feedback 2026-10-21 — the owner asked \"there should be a vault settings section so that a vault can be onboarded\" after the onboarding form had already shipped inside Settings; a capability that lives only as a card deep in a long page does not exist to someone looking for its own section — give significant capabilities a rail destination, and build it from the page's own card so there is one implementation"
type: feedback
date: 2026-11-04
basis: confirmed-by
tier: durable
shape: []
phrases: []
missing: []
recall-count: 2
superseded-by:
valid-until:
---

I shipped first-run vault onboarding (`1d6f946`) as a card inside the Settings
page and reported it as done. The owner's reply was "there should be a vault
settings section so that a vault can be onboarded" — the feature was there,
eleven cards down, behind a chip bar. To him it did not exist.

**Why:** "it is on the page" and "it has a place" are different claims. A long
settings page is a scroll; a capability the owner has to hunt for reads as
missing, and the correction costs a second pass over work that was already
correct. The same shape as [[verify-what-renders-not-what-commits]]: what
ships must be what the owner can find, not what the commit contains.

**How to apply:** when a capability is its own domain (credentials, identity,
access) rather than one knob among many, give it a destination — a route in
`ROUTES`, an entry in `GROUPS`, a `<main id="panel-…">` in the index, an
`el.panelX` + `PANEL_EL` entry, an `arrive` handler, an entry in `BUILT` so the
palette does not badge it "later" — and make the panel reuse the settings
card function (pass an id argument) rather than fork a second implementation.
Then prove it the way the face is proved: load `#/…` with Playwright, assert
the card and the rail link exist and there are no console errors, and check
the **served** `/app.js` with curl as well. Building the route is not enough:
my first attempt registered the destination but the page rendered empty
because I passed a bare string where `surfaceHead` expects an instrument node;
the browser reported `appendChild … not a Node` and only the Playwright console
listener caught it. Related: [[fleet-config-surface-is-the-settings-page]],
[[fleet-gui-editorial-desk-redesign]].
