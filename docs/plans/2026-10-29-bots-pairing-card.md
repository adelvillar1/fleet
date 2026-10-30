---
status: completed
created: 2026-10-29
updated: 2026-10-29
slug: bots-pairing-card
---

# Plan: the Robots surface mints the bot's pairing card

## Context

The owner's correction (2026-10-29): "from the fleet UI when I go to
robots I should be able to use the same pairing pattern that I do on
fleet. It should show me a qr code for the bot experience like it works
on the regular fleet experience." Today the QR card lives only on the
console's topbar and always names the console origin — pairing the bot
means typing a code at a URL from the launch log, which is a different
pattern from the one the owner already uses.

## Decision-model check

No sys1 fit: one card, one origin choice, no per-item judgment.

## Approach

The pairing window already mints the code and the card renderer
(`paintPair`) already draws URL + code + QR + origin lines. The bot
card is the same pattern with a different origin behind it: the
`/api/pair/window` route takes an optional `{"face":"bot"}`, the bridge
builds the bot origin (its relay URL with the entry fragment when
`--bot-relay` dials, else the LAN address and port), and the Robots
surface gains a pairing card whose button opens the same scrim with a
bot-faced mint. The window stays shared — one live code, whichever card
opened last, which is the "newest code wins" rule the console card
already speaks.

High-level steps:

1. **The bridge knows the bot face** — the gui attaches the bot's port
   at bind and its relay URL and secret when `--bot-relay` dials;
   `pair_window` keeps its console shape and a sibling
   `pair_window_for(face)` builds the bot origin (fragment and all),
   falling back to the LAN address with the plain-HTTP note.
2. **The route takes the face** — `POST /api/pair/window` reads its
   body; absent or any face but `bot` is the console card, unchanged.
3. **The Robots card** — `devicesCard()` on `#/bots` gains a pairing
   action: a button that opens the existing scrim and mints
   `{"face":"bot"}`, so the QR, the code, the TTL note, and the
   newest-wins rule are literally the same card.

## Use cases

- [x] A: From #/bots on the loopback face, the owner presses the bot
  pair action and scans a QR that opens the bot experience on the
  phone — the same card pattern the topbar already gives for the
  console.
- [x] B: With `--bot-relay` dialed, the bot card's URL is the bot
  relay origin with its own secret fragment; without it, the card is
  the LAN address with an honest plain-HTTP note.

## Acceptance criteria

- [x] `POST /api/pair/window` with `{"face":"bot"}` answers a card
  whose URL is the bot origin (`--bot-relay` URL when dialed, with
  `#k=` carrying that face's secret; else the LAN address and bot port
  with a plain-HTTP note), while a bodyless or `{"face":"console"}`
  request answers the console card byte-for-byte as before.
- [x] The Robots surface shows a pairing action that opens the shared
  card with the bot mint, and the card it renders carries the QR, the
  code, the TTL note, and the bot origin's lines — one window, newest
  code wins across both cards.
- [x] A phone scanning the bot card pairs at the bot origin through the
  relay (the KI-368 arming included: the reply carries the secret) and
  the bot PWA installs from there.
- [x] The face harness pins the surface's contract and the bridge's
  origin choice, and the gate ran whole with exit codes read directly.

## Landed — 2026-10-29, met with proofs

- **The route** — `POST /api/pair/window {"face":"bot"}` live against
  the loopback face: origin `https://fleetbot.opsh.io`, kind
  `bot_relay`, label "your bot relay origin", the URL carrying
  `?c=` **and** `#k=` (the bot face's own secret), an SVG QR; the
  bodyless call answered the console card with no `face` field,
  unchanged. Pinned by `the_bots_card_names_the_bot_origin` (the relay
  shape with its fragment, the LAN shape with the plain-HTTP note and
  no secret, the console card untouched) — and the shape also covers
  the `--bot-relay`-absent fallback.
- **The surface** — the press, through a real Chromium: the row sits
  first in the paired-devices card, `show QR` opens the shared scrim,
  and the mint's body carries `face: bot` (harness: **392 checks
  across 43 cases**, the new BOTS case included). The screenshots are
  the review: `tmp/shots-bots/bots-pair-row.png` (the row in place)
  and `bots-pair-card.png` (the card: QR, code, the bot origin's URL
  with its fragment, "via your bot relay origin", the five-minute
  note).
- **A phone's scan** — the card's URL is the bot origin's pairing page
  with the entry fragment; KI-368's arming carries the secret again in
  the reply, so either route leaves the phone sealed (the live probe
  proved the redemption shape at this origin the same day).
- **The gate** — fmt 0, clippy 0, build 0, test 0 — **1,813 ran**;
  the two new polling tests' deadlines raised to the house's sixty
  seconds (the loaded-suite flake class, KI-288's shape, caught once
  in the full run and hardened rather than retried).

## Files to be touched

**Backend / API:**
- `src/gui/bridge.rs` — the bot-face state, `pair_window_for`, the
  route's body
- `src/gui/server.rs` — the route reads `{"face": …}`
- `src/gui/mod.rs` — the two attaches

**Frontend:**
- `src/gui/assets/app.js` — the Robots pairing action, `mintPair(face)`

**Tests:**
- `src/gui/bridge.rs` tests — the bot card's origin and fragment
- `tests/face/harness.mjs` — the surface's contract

**Documentation (see Linked artifacts below):**
- `docs/features/remote-access.md` — where the bot card lives
- `docs/features/bot-app.md` — the pairing pattern is the console's

## Out of scope

- A second mint window (the shared window is the existing contract).
- Pairing on the phone itself — the card lives on the loopback face,
  exactly as the console's does.
- Changing the topbar card — it stays the console's.

## Verification

**The bot card's document**
```sh
curl -s -X POST http://127.0.0.1:3080/api/pair/window \
  -H 'content-type: application/json' -d '{"face":"bot"}'
```
Expected: the bot origin, `#k=` carrying the bot secret, the code and
QR; the bodyless call unchanged.

**The surface** — render `#/bots` at 1440×900 and read the PNG: the
pairing action is visible; pressing it shows the card with the bot
origin's lines.

**The gate** — fmt, clippy, build, test, harness — exit codes read
directly.

## Linked artifacts

- `docs/features/remote-access.md` — the pairing section names the
  Robots card as the bot's door.
- `docs/features/bot-app.md` — pairing follows the console's pattern.
- `TECHNICAL-DOCUMENTATION.md` — none beyond the existing Pairing
  bullet (the shape does not change; the bullet already names both
  origins).
- `FUNCTIONAL-SPECIFICATIONS.md` — none (no remote-access passage; the
  feature docs are the homes).

## Risks

- **The shared window means opening one card kills the other's
  unscanned code** — accepted: it is the console card's own rule,
  already stated in the card's text.

## Dependencies

- `2026-10-29-remote-relay-openship.md` and
  `2026-10-29-bot-face-on-the-relay.md` (both completed) — the origins
  and secrets this card points at.
- KI-368's fix — the redemption carries the secret the fragment also
  carries; either path arms the phone.

## Notes

The owner's words, 2026-10-29: "this experience is wrong… I should
show me a qr code for the bot experience like it works on the regular
fleet experience." The pattern — press, scan, paired — is the unit;
the origin behind it is the only thing that differs, and the card says
which.
