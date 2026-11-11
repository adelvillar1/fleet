---
name: a-mockup-token-that-does-not-exist-reads-as-black
description: "feedback 2026-11-09 — two mockups in one day referenced CSS custom properties that are not in the app's token sheet; an unresolved var() falls back to the initial value, so titles render black on a dark theme (1.12:1) and only the contrast measurement finds it"
type: feedback
date: 2026-11-11
basis: inferred
tier: durable
shape: []
phrases: []
missing: []
premises: []
recall-count: 0
superseded-by:
valid-until:
source: zcode:fleet:content:285983102772d99aaed2c496d57acdf6f320aad7347d3891e9bb9e2378839d34
---

Feedback 2026-11-09 — while drawing the inbox and then the desk/learning mockups
I wrote classes against token names I had not checked:
`--dsw-alias-ink-2` (the real one is `--dsw-alias-ink-secondary`) and an
unstyled `.mk-title` / `.mk-msg` with no `color` at all.

**The failure is silent and looks like a theme bug.** An unresolved `var(--x)`
falls back to the property's initial value (`color: canvastext` → black);
against a dark theme that is ~1.12:1, and the page still *looks* laid out
correctly, so the eye passes it. The contrast pass caught it twice: the
token reference read 1.12:1 and, separately, the primary-button white-on-accent
read 2.93:1 (the KI-476 shape).

**Why:** a mockup is meant to be read as the shipped surface; a colour it
invents makes the review judge a page the app will never render.

**How to apply:**
- Grep the app's token sheet for every `--dsw-alias-*` you write; a token you
  cannot find is a typo, not a new token.
- Give every text-carrying class an explicit `color`, even the ones you think
  inherit — inheritance from `body` is fine, from nothing is black.
- Never style a filled action with raw white on the accent: use the
  accent-wash + `--dsw-alias-accent-lift` pair the face uses, or the dark
  themes fail AA (KI-476, measured again here at 2.93:1).
- Inline the token sheet into the mockup so a measurement run without the app
  server still resolves every variable — and re-measure after the fix.

Related: [[measuring-a-mockup-by-reading-the-png]],
[[a-contrast-measurement-needs-the-alpha-channel]].
