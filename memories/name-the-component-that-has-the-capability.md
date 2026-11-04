---
name: name-the-component-that-has-the-capability
description: "when a capability spans components, say which component has it rather than declaring the system lacks it — the owner corrected "fleet has no browser engine so there is no screenshot" when the app is the server and the browser is the rasterizer"
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

On 2026-10-17 I told the owner the vision feedback loop was blocked because
"fleet has no browser engine, so there is no screenshot for a vision model to
look at." The owner's correction: "Isn't the entire app itself a server that
renders the gui." Of course — `fleet gui` serves the deck as HTML and the
browser renders it. The engine was never missing; it lives one component over,
and the app was already wired to it.

**Why it matters:** declaring that a *system* lacks a capability one of its own
components has is a reasoning error that closes off work that was never
blocked. It also lands as a criticism of the owner's architecture when it is
really a limitation of my framing. The honest sentence names the component:
fleet cannot rasterize HTML *itself*, the browser can, and the owner is the
bridge between them — which is a division of labour, not a gap. Framed that
way the missing piece became small (a full-page view), and it shipped.

**How to apply:** before saying "X can't do Y", ask which component could, and
whether the system already wires them together. Fleet shells out to `open` /
`xdg-open` for the browser, the browser renders whatever the server serves, the
`computer_use` lane drives a real browser, and the owner can take a screenshot.
Most "fleet can't" claims are really "fleet doesn't do this part itself" — and
the second phrasing is both truer and more useful, because it points at the
seam instead of at a wall. Related: [[fleet-deck-visual-pipeline]],
[[owner-stated-facts-are-authoritative]].
