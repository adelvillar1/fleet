---
name: remote-relay-openship-plan-2026-10-29
description: "project 2026-10-29 — the relay-is-the-remote-path plan IMPLEMENTED, DEPLOYED and proven live: fleet.opsh.io on OpenShip serves the paired face at the origin root, frame bodies are sealed end to end with a key from an X25519 exchange the relay cannot derive, the secret rides URLs as a fragment; commits 9981877 + 30b38b2, plan completed, KI-367 open"
type: project
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

2026-10-29 the owner ruled ("I like option A hosted on OpenShip…") and
the same day the plan `docs/plans/2026-10-29-remote-relay-openship.md`
was implemented whole: **the outbound relay on OpenShip Cloud is now
fleet's primary remote path** — `https://fleet.opsh.io` (free subdomain,
edge TLS), the LaunchAgent gained `--relay`, `remote.public_url` names
the origin. Committed `9981877` (the increment) + `30b38b2` (docs, plan
completed with proofs, KI-367, recap), pushed, gate 1,811 passed / 5
ignored, harness 384 checks / 42 cases. See [[tailscale-remote-access]]
for the demotion.

**What the design became** (four amendments forced by drafting/build,
each recorded in the plan BEFORE the code it shaped):
- **The pairing secret was the wrong key root** — the relay holds it at
  join, so deriving frame keys from it is not E2E (see
  [[e2e-keys-cannot-come-from-a-secret-the-intermediary-holds]]). The
  frame key comes from an X25519 exchange through the relay
  (`GET/POST /api/relay/handshake`): device holds a per-launch scalar,
  phone its own, HKDF-SHA-256 → AES-256-GCM salted with the session id,
  nonce in the body, frame id (answers) or request path (requests) as
  the AAD. Unsealed `/api/` frames refused 400 by name. Crates:
  x25519-dalek, aes-gcm, hkdf, sha2, base64, rand. Phone side:
  WebCrypto (X25519 + HKDF + AES-GCM) in a `window.fetch` patch in
  app.js, keyed on `localStorage["fleet_relay_k"]`; seal boundary is
  the relay.rs serve loop so the handler keeps reading plaintext.
- **The relay carries the CONSOLE face** (the owner's 2026-10-28
  correction in listener.rs), not the bot shell — the install set is
  the console's and the phone wrapper lives in app.js.
- **The install set rides as plaintext frames** (browser navigations
  cannot present headers or open ciphertext) — public code, named.
  Icons are bytes in a text frame protocol → base64 under
  `x-fleet-b64`, decoded by relay.py.
- **The secret rides the URL FRAGMENT** (`#k=…`) — browsers never send
  it; app.js files it into localStorage on boot, the pairing page
  preserves it across the post-pairing redirect. The QR card embeds it
  (bridge.attach_relay_secret). Root-mode requests present it as the
  `x-fleet-relay` header; `x-fleet-enc: 1` marks sealed bodies; both
  headers are stripped before forwarding. **The fragment is no longer
  the only way in** (KI-368, fixed `efd067f`): a phone that arrives by
  typing the origin and pairs by code never had the fragment, and
  paired unsealed — the console's EventSource then read "stream down"
  and the bot 403'd. Now relay.py stamps its forwarded frames
  `x-fleet-via: relay` and the redemption reply carries the face's
  relay secret for a relay-forwarded pairing only (the socket path
  passes `None` regardless of headers; pinned by the handler test
  `a_relayed_redemption_carries_the_secret_and_a_plain_one_does_not`),
  and the pairing page stores it before the redirect — see
  [[pairing-at-an-origin-arms-the-transport]].

**Proven live** (`tmp/relay-live-probe.py`, kept uncommitted — needs a
fresh mint per face restart): pairing page + redemption through the
public origin, the exchange, a SEALED session creation opened from
ciphertext, the unsealed refusal named. Integration test
`the_reference_relay_carries_sealed_frames` runs real relay.py as a
child (needs `python3 -u` — piped stdout block-buffers the banner;
port 0 → the banner prints the BOUND port). Capturing-relay unit test
pins the bytes. Resilience: a relay redeploy clears all sessions and
the face rejoins unaided ≤30 s; devices that stop polling are pruned
after 90 s and headered root paths route BY THE SECRET, so a stale
session never blocks. Ten-minute soak: no churn.

**KI-367 (open, the owner's call):** the relay's `POST /join` is
ungated — a stranger who finds the origin can hold sessions
(availability nuisance only; every headered path still demands the
secret, wrong secrets 403 by name). Options: admission token in the
relay env, an OpenShip edge rule on /join, or accept. Also open: the
owner's one-tap iPhone acceptance (Add to Home Screen on the proven
install set) and the permanent domain (one `remote.public_url` change).

**Per-launch facts:** each face launch mints a NEW relay session id +
secret — a face restart invalidates the old QR's `#k=` and the phone
re-handshakes on next page boot via the gen stamp (first 8 hex of the
sid). OpenShip deployment mechanics live in
[[openship-deployment-facts-2026-10-29]] and the
openship-cloud-deployment skill. The face runs `fleet gui --no-open
--remote --relay https://fleet.opsh.io` under com.fleet.gui, and since
that same evening ALSO `--bot-relay https://fleetbot.opsh.io` — the bot
face got its own second relay service ([[both-faces-required-console-and-bot]],
commit b39b258, harness 388/42), so this plan's "the relay carries the
CONSOLE face" bullet describes the console relay only. The LaunchAgent
now writes `~/Library/Logs/fleet-gui.log` — the launch words (both
entry URLs with their `#k=` secrets) are readable there, and each face
restart mints fresh ones.

**2026-10-29 late: the pairing pattern the owner asked for** — "this
experience is wrong… it should show me a qr code for the bot
experience like it works on the regular fleet experience" — landed as
commit `6e6ae83` and plan `2026-10-29-bots-pairing-card.md` (completed):
`POST /api/pair/window {"face":"bot"}` builds the bot card (relay
origin with its own `#k=` fragment, or the LAN address with a
plain-HTTP note when no bot relay dials), and the paired-devices card
on `#/bots` leads with the **show QR** action opening the same shared
scrim — one window across both cards, newest code wins. Gate 1,813,
harness 392/43. Details and the harness traps in
[[bots-surface-mints-the-bot-pairing-card]].
