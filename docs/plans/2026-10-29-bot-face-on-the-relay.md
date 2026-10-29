---
status: completed
created: 2026-10-29
updated: 2026-10-29
slug: bot-face-on-the-relay
---

# Plan: the bot face on the relay — a second service, the same seal

## Context

The owner's ruling, 2026-10-29: "the bot needs to be available on the
relay, or a separate relay with a separate service" — extending the
both-faces ruling (the work-centric console AND the assistant-first bot
experience) to the remote path. Measured at the ruling: the console is
relayed and sealed at `https://fleet.opsh.io`; the bot face listens on
`:3081` at the LAN address only. The owner left the shape open; this
plan takes **a separate relay with a separate service** — the relay is
already a single-device switchboard, so a second deployment of the same
two files needs zero relay changes, gives the bot its own origin (the
two-PWA-installs rule: an origin is the install's identity), and keeps
a bot redeploy from ever touching the console path.

## Decision-model check

No sys1 fit: routing and deployment plumbing over a seal that exists.

## Approach

The face dials a second relay client for the bot listener — its own
session, its own secret, its own handler answering as `RemoteFace::Bot`
— and the bot origin's root serves the bot shell's install set the way
the console origin serves the console's. The transport the phone runs
is the same X25519-then-AES-GCM seal; the bot shell's wrapper is its
own copy (the faces are separate embedded assets by design), simpler
because bot.js has one fetch seam. Pairing happens at each origin
separately — two domains cannot share a cookie — which is the
two-installs rule carried to its honest end.

High-level steps:

1. **The flag and the dial** — `fleet gui --bot-relay <url>` (default
   absent: the bot face keeps its LAN-only shape). The bot listener's
   session and secret are captured at bind, and a second relay client
   joins the bot relay; the launch prints the bot entry URL with its
   fragment.
2. **The bot handler's own answers** — the relay handler carries its
   face: the icons come from the bot set (`bot_icon_for`), the
   allowlist is the bot's own default-deny list, the pairing page and
   redemption and handshake answer as they do on the socket path.
3. **The bot shell's transport** — `bot.js` gains the fragment capture,
   the handshake, and the seal at its one `api()` seam; the relayed
   stream is a poll (the conversation's own refresh loop on an
   interval), never a subscription the transport cannot carry.
4. **The second service** — the same two files deploy as `fleetbot` on
   the existing OpenShip project (slot two of forty); the LaunchAgent
   gains `--bot-relay https://fleetbot.opsh.io`.
5. **Proven like the console** — the phone-role probe pairs at the bot
   origin, runs the exchange, and opens a sealed read from ciphertext;
   the gate runs whole; the docs name both origins and the two-pairing
   shape.

## Use cases

- [x] A: On the phone, the assistant experience opens from its own
  origin over HTTPS, paired and sealed exactly like the console —
  the inbox, a conversation, a dispatch, a seat.
- [x] B: The two installs coexist (console and bot, two origins), and
  pairing one never pairs the other.

## Acceptance criteria

- [x] The bot relay deploys as a second compose-kind service on the
  `fleet-relay` project from the same two files, `openship deployment
  get` reports `ready`, and `GET https://<bot-subdomain>/healthz`
  answers 200 over a verified TLS cert.
- [x] With `--bot-relay` set, the face dials the bot relay (the bot
  origin's healthz counts its device), the origin root serves the bot
  shell's install set — `/` (the bot page), `/bot.js`, `/bot.css`,
  `/bot.webmanifest` as `application/manifest+json`, `/bot-sw.js`,
  `/bot-icon-192.png` and `/bot-icon-512.png` as `image/png` — and the
  launch prints the bot entry URL with its secret as a fragment.
- [x] A fresh pairing at the bot origin redeems through it, the bot
  allowlist holds through the relay (a non-allowlisted route answers
  403 by name; a wrong secret 403; an unsealed `/api/` frame 400),
  and pairing at the bot origin leaves the console origin's devices
  untouched.
- [x] Every content-bearing frame through the bot relay is sealed with
  the same X25519-then-AES-GCM contract: the phone-role probe runs the
  handshake at the bot origin and opens a sealed read from ciphertext,
  the wire carries no plaintext, and the relayed stream is a poll.
- [x] `docs/features/remote-access.md` and `docs/features/bot-app.md`
  name the two origins, the two pairings, and the bot entry URL; the
  technical documentation's pairing bullet says both faces are on the
  relay; and the gate ran whole with exit codes read directly.

## Landed — 2026-10-29, met with proofs

- **The second service** — `bot` (`svc_eYFSu8b7B-s2XQiw`) on the
  `fleet-relay` project, `https://fleetbot.opsh.io`, from the same two
  files. The first stack deploy failed with the platform's own trap (a
  single-service id on a two-service stack: "No image is available for
  bot"); the redeploy passed both ids and went `ready`
  (`dep_z_STSHuArzKW0n3-`). `GET /healthz` answers 200 over a verified
  cert.
- **The face dials it** — the LaunchAgent carries `--bot-relay
  https://fleetbot.opsh.io`; the launch log (the agent now writes
  `~/Library/Logs/fleet-gui.log` — the launch words were dying in
  launchd's void, which is why the entry URLs kept escaping) prints
  both entry URLs; the bot origin counts its device and serves its
  install set: `/` 200, `/bot.webmanifest` 200
  `application/manifest+json`, `/bot-sw.js`, both bot icons `image/png`.
- **Pairing at its own origin** — the live probe redeemed a fresh code
  at `fleetbot.opsh.io` (`{"type":"paired"}`), ran the X25519 exchange
  through the bot relay, and **opened a sealed conversation creation
  from ciphertext** — a real bot conversation started through the
  public internet with end-to-end sealed frames. The wrong-secret 403
  and the unsealed 400 (`this relay seals its content`) hold here too.
- **The boundary** — the bot handler answers as `RemoteFace::Bot` (the
  bot allowlist, the bot icon set via the face-aware `icon_answer`),
  and a pairing at the bot origin wrote its own device record, leaving
  the console origin's devices untouched.
- **The gate** — fmt 0, clippy 0, build 0, test 0 on the whole tree;
  the face harness **388 checks across 42 cases** (the RL case now pins
  the bot transport's twin: the seal headers, the bot install set bare,
  the second dial, the face-aware icons).

## Files to be touched

**Backend / API:**
- `src/cli.rs` — `--bot-relay` (parse, help, the remote options field)
- `src/gui/mod.rs` — the bot listener's captured credentials, the
  second relay client and handler, the bot entry URL print
- `src/remote/listener.rs` — the relay handler's icon answer picks by
  face (the bot set for the bot face)

**Frontend (the bot shell):**
- `src/gui/assets/bot.js` — the transport: fragment capture, the
  handshake, the seal at the `api()` seam, the relayed poll

**Tests:**
- `tests/face/harness.mjs` — the bot transport's source contract
- `tests/integration.rs` — the relay-in-the-loop shape extended if the
  bot handler needs its own pin (the icon set by face)

**Documentation (see Linked artifacts below):**
- `docs/features/remote-access.md` — the two origins
- `docs/features/bot-app.md` — the install origin becomes real
- `TECHNICAL-DOCUMENTATION.md` — the pairing bullet

## Out of scope

- One-relay face-aware routing — rejected: relay changes, multi-session
  ambiguity, no benefit over two dumb switchboards.
- Sharing one pairing across the two origins — impossible across
  domains and wrong under the two-installs rule anyway.
- The QR card gaining a bot tab — the bot origin's own page is its
  door; the card stays the console's.
- The permanent domain — unchanged follow-up, now two settings.

## Verification

**Deployment and the origin**
```sh
openship deployment get <bot-deployment>     # ready
curl -sS https://<bot-subdomain>/healthz     # 200, devices: 1 after the face dials
curl -sSI https://<bot-subdomain>/bot.webmanifest   # application/manifest+json
```

**The sealed path at the bot origin**
The live probe with `--origin <bot-subdomain>`: pair (a fresh mint),
handshake, a sealed `/api/inbox` read opened from ciphertext; a wrong
secret 403; an unsealed frame 400. The wire assertions stay in the
integration test; the live probe proves the deployment.

**The gate**
fmt, clippy, build, test — exit codes read directly.

## Linked artifacts

- `docs/features/remote-access.md` — the two relays, the two pairings,
  the bot entry URL.
- `docs/features/bot-app.md` — the install origin is the bot relay
  origin (the tailnet sentence is already gone; this makes the relay
  sentence true for the bot).
- `TECHNICAL-DOCUMENTATION.md` — the pairing bullet names both faces.
- `FUNCTIONAL-SPECIFICATIONS.md` — none (no remote-access passage; the
  feature docs are the homes, per the previous plan's disposition).

## Risks

- **Two pairings is two steps for the owner** — accepted: it is the
  two-installs rule's honest cost, and each pairing is one scan.
- **The bot shell's poll interval through the relay** — the same
  2.5 s shape the console uses; the soak already proved the cadence.
- **A second service slot** — two of forty; nothing.

## Dependencies

- `2026-10-29-remote-relay-openship.md` (completed) — the seal, the
  relay, the deployment shape this plan reuses whole.
- The both-faces ruling (memory: `both-faces-required-console-and-bot`).

## Notes

The owner's words, 2026-10-29: "sorry, but the bot needs to be
available on the relay, or a separate relay with a separate service."
The separate service is the shape that adds no mechanism: the relay
stays a single-device switchboard, the bot gets its own origin, and
every property the console earned — the seal, the fragment, the
install set, the rejoin — is the bot's by construction rather than by
new code.
