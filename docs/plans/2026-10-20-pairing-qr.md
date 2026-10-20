---
status: completed
created: 2026-10-20
updated: 2026-10-20
slug: pairing-qr
---

# Plan: Pair from the screen — a QR card with a Tailscale-aware URL

## Outcome (2026-10-20)

All four increments landed. The topbar's QR icon (loopback face only)
opens a card that mints a single-use window on demand and shows a
scannable QR, the URL, the code, the origin, and the expiry. The URL is
`remote.public_url` when set, else a detected Tailscale name verified by
`/api/state` (401 is the paired face, 200 is the loopback face and is
refused as a pairing origin), else the listener's address with the
plain-HTTP note. The launch block uses the explicit public URL; the card
does the tailnet detection so launch stays fast. `POST /api/pair/window`
is loopback only, and the remote allowlist answers 403 for it.

Verified live: with the public URL set, the card showed a code and QR,
"new code" rotated it, the URL's pairing page opened with the code
prefilled, the phone submitted and reached the shell at the remote root,
the pair icon was hidden on the phone, and the mint answered 403 there.
With no public URL, Tailscale was detected, the probe timed out, and the
card fell back to the LAN address with the note naming `tailscale serve`
in 4.2 seconds. The tailnet-verified branch was not exercised live on
this machine (it has no serve mapping); its logic is covered by the
probe and by the origin resolution's tests.

Found and fixed in the round: KI-276, the probe's bound did not cover a
blocked resolver, so the card hung; the whole probe now runs on a worker
with a four-second bound. The TUI settings test rendered at 24 lines,
which the new key pushed past; it now renders tall enough for the
sections, because the pane scrolls.

The four-step gate is green: 1280 library tests plus every integration
suite, fmt and clippy clean, release built.

## Context

Pairing today prints a URL and an ASCII QR in the terminal at
`fleet gui --remote` launch, and `fleet pair` prints a code. That works
at a desk with the terminal visible, and not at all when the phone is on
Tailscale: the printed URL is this machine's LAN address over HTTP, which
iOS cannot open to a tailnet address (ATS refuses cleartext), and the QR
in a terminal is not easy to scan.

The pieces already exist: `PairingWindow::mint` writes a single-use,
five-minute code; `pairing_url` builds `/pair?c=<code>`; `qr::encode`
builds the matrix. What is missing is a URL that is right for the
owner's network, and a surface to show it from the app.

## Approach

**P1 — one origin resolution, stated honestly.** `remote::pairing_origin`
answers with a base URL and where it came from:

1. `remote.public_url` in the home's config (a new settings key): the
   HTTPS origin a proxy serves the remote face on. Trailing slash
   trimmed; non-http(s) values are ignored with a note.
2. Else, Tailscale: `tailscale status --json` gives `Self.DNSName` when
   the daemon is running; the origin is `https://<dns name>`. It is
   **verified** by asking `<origin>/api/state` with a short timeout — the
   remote listener answers 401 without a device cookie, the loopback
   face answers 200. A 401 means the tailnet origin reaches the paired
   face; a 200 means the proxy points at the loopback face and `/pair`
   is not there; anything else is unreachable. Both non-401 outcomes
   fall back with a note that names what to point at.
3. Else, the listener's own address: `http://<address>:<port>`, with a
   note that this is plain HTTP and iOS will refuse a tailnet address.

The launch block (`fleet gui --remote`) uses the same resolution for its
printed URL and QR, so the terminal and the card cannot disagree.

**P2 — the window and the drawing.** `POST /api/pair/window` (loopback
only) mints a fresh window and answers one document: the URL, the code
in display form, the origin and its kind, the note, the TTL, and the QR
as an SVG string built from the existing matrix (`QrCode::to_svg`, black
on white whatever the theme). It refuses with a named 409 when this
process does not host the remote face, because a code for a listener
that is not running is a code that cannot be redeemed.

**P3 — the icon and the card.** A QR icon in the topbar, shown only when
the page is the loopback face (not the paired phone, not the relayed
path). It opens a focused card: the QR, the URL with a copy action, the
code, the origin line with its note, the expiry, and a "new code"
action. The newest window wins — the card says so — so re-opening it
invalidates the previous invitation rather than leaving two live codes
in the owner's screenshots.

**P4 — tests and the reading rules.** The SVG is tested for its shape;
the explicit public URL is tested without touching the network; the
window route is tested with a public URL in the home so the origin
resolution returns immediately; the remote allowlist test grows the new
route and asserts a 403. Docs name the setting, the Tailscale command
shape, and the fallbacks.

## Acceptance Criteria

1. With `remote.public_url` set, the card's URL is
   `<public_url>/pair?c=<code>` and its origin kind says the setting.
2. With no setting and a running Tailscale daemon, a tailnet origin that
   answers 401 to `/api/state` is used as `https://<dns name>`; one that
   answers 200 or does not answer is not used, and the card says why.
3. With neither, the URL is the remote listener's address and the card
   says the connection is plain HTTP.
4. `POST /api/pair/window` mints a window, returns the SVG, and a code
   that redeems once; a second click mints a new one and the first stops
   working.
5. With the remote face not hosted, the route answers 409 naming
   `fleet gui --remote`, and the card shows that sentence.
6. The QR icon appears on the loopback face only; clicking it shows the
   card; Escape, the close button, and the scrim all dismiss it.
7. A phone opening the card's URL with the code pairs and reaches the
   shell: redeem the code, then `GET /` with the cookie is the face.
8. The remote face's allowlist answers 403 for `POST /api/pair/window`.
9. The four-step gate is green and a live Chromium pass shows the card's
   QR, URL, code, origin note, and the new-code behavior.

## Files to be touched

| Increment | Files |
|---|---|
| P1 | `src/remote/mod.rs` (origin resolution, Tailscale probe), `src/gui/mod.rs` (launch uses it), `src/settings.rs` (`remote.public_url`) |
| P2 | `src/remote/qr.rs` (`to_svg`), `src/gui/bridge.rs` (attach the remote face, `pair_window`), `src/gui/server.rs` (the route) |
| P3 | `src/gui/assets/index.html` (icon, card markup), `src/gui/assets/app.js` (open/paint/new/close), `src/gui/assets/app.css` (the card) |
| P4 | tests in `qr.rs`, `remote/mod.rs`, `gui/bridge.rs`/`server.rs`, `remote/listener.rs`; `docs/features/gui.md`, `docs/features/remote-access.md` |

## Out of scope

- Writing a Tailscale serve configuration for the owner; the card names
  what to point it at.
- Multiple live pairing windows or per-device invitations; one newest
  window is the existing rule.
- A QR on the remote face itself (a paired phone does not pair phones).
- Revoking devices from the card; `fleet devices` and the access lists
  keep that.

## Verification

- Unit: the SVG's shape, the explicit origin, the route's document with
  a public URL, the remote 403.
- Live: a scratch face with `--remote --remote-host 127.0.0.2`; open the
  icon, read the URL and code, redeem them from a second browser
  context, and reach the shell. Screenshot the card.
- The four-step gate.

## Linked artifacts

- `src/remote/pairing.rs` (the window, TTL, single use).
- `src/remote/qr.rs` (the hand-rolled encoder and its vectors).
- `docs/features/gui.md` and `docs/features/remote-access.md`.
