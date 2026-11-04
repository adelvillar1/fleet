---
name: bots-surface-mints-the-bot-pairing-card
description: "project 2026-10-29/30 — #/bots gives the bot experience the console's own pairing pattern (POST /api/pair/window {"face":"bot"} → relay origin + #k fragment, or LAN with a plain-HTTP note), and after the owner twice said he saw NOTHING the row became its OWN CARD under the page head (measured rowTop 369, above the fold at 700px) while the counters strip gained "build <hash>"; commit 17b176d, harness 393/43"
type: project
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

2026-10-29 late: the owner's correction — "from the fleet UI when I go
to robots I should be able to use the same pairing pattern that I do on
fleet. It should show me a qr code for the bot experience like it works
on the regular fleet experience" — landed as commit `6e6ae83`, plan
`docs/plans/2026-10-29-bots-pairing-card.md` completed, gate 1,813,
harness 392/43.

**The shape:** `POST /api/pair/window` takes an optional
`{"face":"bot"}`; the console's `pair_window()` is byte-for-byte
unchanged. The bridge gained `bot_face` state (`attach_bot_face(port)`
at bind, `attach_bot_relay(url, secret)` when `--bot-relay` dials) and
`pair_window_for(face)`: relay URL with `?c=` AND its own `#k=` secret
fragment when dialed, else `pairing_url(address, bot.port)` with a
plain-HTTP note and **no** secret. One shared window behind both cards —
whichever opened last owns the live code ([[pairing-at-an-origin-arms-the-transport]]).

**The surface, corrected after the owner twice said he saw nothing
("I don't see the pair a phone to the bot experience anywhere on the
ui" → "I cannot see the pair option within the page on the local
machine", commit `17b176d`):** the row was in the right place relative
to its card but the WRONG PLACE ON THE PAGE — it lived inside the
paired-devices card, which follows the service card, so at 1280×700 its
top sat at ≈747px, below the fold: "a look at the top of robots was a
look at no pairing", exactly what he reported. It is now its OWN card
(`boardCard` "pair a phone · the bot experience") inserted directly
after the page head + counters strip — measured `rowTop` 369 at both
1440×900 and 1280×700, above the fold — with the action `show QR` →
`openPair('bot')` → `mintPair(face)` body `{face:'bot'}`. The first
attempt rendered it ABOVE the "Bots" title (y=183), which read wrong;
"top of the surface" means under the head, not before it. The harness
now pins placement: the surface's **first** `.bots-act` is the pairing.

**And every window names its build:** the counters strip's stamp reads
`<read_at> · build <hash>` from `document.documentElement.dataset.build`
(set at boot from the page's own `app.js?v=` hash). That one visible
fact settles "is this window running the new code" — an evening of
cache guessing could not, and asking for devtools was the wrong ask.

**How to apply:** when the owner says a shipped control is missing, the
question is not only "is the build live" but **"is it above the fold,
inside the right card, under the right heading"** — render at a SHORT
viewport (700px is a real laptop window; 900 hid nothing) and measure
the control's top, not just its presence. Put the owner's own action at
the top of its surface, under the page head. When they doubt the build,
prove served bytes (`curl -s …/app.js | grep <new strings>`, the
`app.js?v=` pin, the binary's embedded copy) and then point at the
build stamp they can read — [[verify-the-binary-is-newer-than-the-source]]
and [[the-screenshot-is-the-visual-review]].

**Harness traps this case walked through (reuse them):** the fake
`location_` had no `hostname` (so `canPair()` was false for every boot —
add `hostname: '127.0.0.1'`); the fake `El` had no `childNodes` alias
(`this.childNodes = this.children`); the fake `querySelectorAll` speaks
**classes not attribute selectors** (`.bots-act` + text, never
`[data-act="…"]`); ancestors do not aggregate descendant `textContent`
(assert on a leaf's own text); panel arrival needs an explicit
`page.hash('#/bots')` (the initial hash fires no `hashchange`); a bots
case needs a WHOLE channels fixture (an empty object starves the later
cards). Slicing the cases array by an index marker once deleted 40
entries — restore from HEAD and re-apply only the deliberate edits
([[a-repeated-marker-slice-clobbers-whatever-follows]]).

**And the card's promise was still hollow that evening:** the owner took
the card, paired with the QR, and got "400 after pairing" — a real seal
defect downstream of pairing, not the card ([[ki-371-the-sealed-frame-boundary]]);
once it rendered, the inbox he saw was the *machine's* cron traffic
([[ki-372-the-owners-screen-is-class-yours]]). A QR that pairs correctly
is only the first hop of "the bot experience is available".

**Also hardened:** the two new polling tests' deadlines 30 s → **60 s**
after one caught loaded-suite flake (KI-288's class) — harden, don't
retry ([[a-test-that-passes-on-an-idle-machine-has-not-been-tested]]).
