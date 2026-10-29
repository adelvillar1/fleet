# The relay is the remote path: OpenShip, the sealed frames, the origin that installs

**2026-10-29 (third session)** · the owner's rearchitecture ruling
implemented whole — the outbound relay on OpenShip Cloud is now the
primary way fleet is reached from anywhere, the origin root serves the
paired face so the PWA installs, and every content-bearing frame is
sealed end to end with a key the relay cannot derive · the live probe
paired through the public origin and opened a sealed session creation
from ciphertext · the redeploy resilience proved itself twice · commit
`9981877` (the increment) plus the docs and the plan. Ledger checker
green: **367 rows, 32 open** (KI-367 filed).

## The ruling

"I like option A hosted on OpenShip, we can use the free domain they
provide and later map it to a more permanent home. I'm already paying
for the OpenShip Pro tier so no need to go anywhere else." — the
outbound relay over Tailscale, whose `serve` was measured never
configured: no HTTPS origin existed, so the installable PWA had no
origin and away-from-home had only the Telegram channel.

## What landed

- **The deployment** — `https://fleet.opsh.io` (OpenShip Cloud, the
  free subdomain, edge TLS), a compose-kind service from two files:
  `relay/relay.py` and a new `relay/Dockerfile` (python:3.12-slim,
  unprivileged, port from the environment). `remote.public_url` names
  the origin; the LaunchAgent gained `--relay https://fleet.opsh.io`;
  `healthz` answers publicly and the face joined it (devices: 1).
- **The root is the device** — the dedicated relay forwards the origin
  root to the joined device, so the shell, the manifest, the worker,
  and the icons resolve at the origin root. Measured through the
  public origin: `/` 200, `/manifest.webmanifest`
  `application/manifest+json`, `/sw.js`, both icons `image/png` — the
  icons ride as base64 under a marker header the relay consumes,
  because the frame protocol is text and an icon is bytes. The
  per-session `/s/<sid>/` shape stays for compatibility.
- **The frames are sealed end to end** — the frame key comes from an
  X25519 exchange through the relay (`GET`/`POST
  /api/relay/handshake`): the device holds a per-launch scalar, the
  phone holds its own, and the relay and its edge hold no scalar —
  HKDF-SHA-256 into AES-256-GCM, salted with the session id, the nonce
  riding the body, the frame id or request path as the binding. An
  unsealed `/api/` frame is refused by name. The relay's own threat
  model paragraph in `relay.py` says what is now true instead of
  promising a later increment.
- **The secret left the URLs** — the entry URL and the QR card carry
  `#k=<secret>`, a fragment browsers never send; the page files it
  into storage on boot and the pairing page preserves it across the
  post-pairing redirect. On the wire the URL names nothing.
- **The pairing page and the redemption ride the relay** — the relay
  path's handler answers `GET /pair` and `POST /api/pair` before the
  allowlist, exactly as the socket path does; the relayed pairing page
  is pinned by its own test.
- **Eviction and secret routing** — device sessions that stop polling
  are pruned after ninety seconds, and headered root paths route by
  the presented secret itself, so a redeployed relay and a sleeping
  face never leave a corpse between the phone and its fleet.

## Proven, not asserted

- **The live probe** (`tmp/relay-live-probe.py`, the phone's role in
  python): the pairing page through the public origin, the code
  redeemed (`{"type":"paired"}`), the exchange through the gate, a
  **sealed session creation opened from ciphertext** — a real
  conversation started on the Mac from the public internet with
  end-to-end sealed frames — and the unsealed frame refused 400, named.
- **The integration test** (`the_reference_relay_carries_sealed_frames`)
  runs the real `relay.py` as a child over the real handler and pins
  the wire: the request bytes and the answer bytes carry no plaintext;
  the endpoints decode.
- **The capturing relay** (the unit test) pins every byte the relay
  held against the task text — the audit's captured-bytes standard.
- **Resilience, twice by accident and once by design** — a redeploy
  cleared all sessions and the running face rejoined unaided within
  thirty seconds (`devices: 1`, no face restart); the ten-minute soak
  ran sealed round trips through the edge every sixty seconds with no
  churn.
- **The gate, whole**: fmt 0, clippy 0, build 0, test 0 — 1,810
  passed, 5 ignored across 11 suites; the face harness **384 checks
  across 42 cases** (the new RL case pins the fragment capture, the
  loopback inertness, the header names, the bare install set, and the
  poll-not-subscribe stream).

## What the build caught (and the plan's honest amendments)

Three facts forced plan amendments before the code that shaped them —
each recorded in the plan's own record:

1. **The pairing secret was the wrong key root**: the relay holds it at
   join, so keys derived from it are not end-to-end. The X25519
   exchange replaced it.
2. **The face the relay carries is the console** (the owner's
   2026-10-28 correction in `listener.rs`), not the bot shell — the
   install set, the phone wrapper (`app.js`, not `bot.js`), and the
   criterion wording all moved.
3. **The install set rides as plaintext frames** — a browser
   navigation cannot present a header or open a ciphertext; public
   code, named rather than pretended sealed. Also: a folder deploy
   carries no commitSha; the provenance is the byte-identical files.

The build itself caught two more: the relay path's router never
answered the pairing page or the icons (the socket path's pre-allowlist
branches had no relay counterparts), and Python's piped stdout buffers
(the integration test's banner needed `-u`).

## KI-367: the open join (filed, options named)

The deployment probe itself demonstrated the one real exposure: `POST
/join` is ungated, so a stranger who finds the origin can hold sessions
(a self-healing availability nuisance since the eviction, not a
confidentiality loss — every headered path still demands the session
secret, wrong secrets answer 403 by name). Options are in the row for
the owner's ruling: an admission token in the relay's environment, an
edge rule limiting `/join`, or accept.

## Open questions / next steps

- **The owner's one-tap acceptance**: open `https://fleet.opsh.io` on
  the iPhone, Add to Home Screen, launch, airplane-mode test. The
  install set is proven; the gesture is the owner's.
- **The permanent domain** — OpenShip's documented custom-domain flow
  plus one `remote.public_url` change, when the owner picks the name.
- **KI-367's disposition** — the three options, the owner's call.
- **`/events` over the relay is a poll now** (2.5 s), which is what the
  design said a phone does well; if the owner wants live-er, the
  relay's long-poll is the seam.
- **Tailscale** — demoted to the second origin candidate; the tailnet
  still exists. Its serve config remains unset; nothing depends on it.

## Files changed

- **Rust**: `src/remote/relay.rs` (the seal, the exchange, the serve
  loop's boundary, the refusal), `src/remote/listener.rs` (the
  handshake and redemption and pairing-page and icon routes, the
  handler's keys), `src/gui/mod.rs` (the entry URL, the keys, the
  launch line), `src/gui/bridge.rs` (the card's fragment),
  `Cargo.toml`/`Cargo.lock` (x25519-dalek, aes-gcm, hkdf, sha2,
  base64, rand — the seal's own comment says why they cannot be
  swapped).
- **Relay**: `relay/relay.py` (healthz, the root, the gate, eviction,
  secret routing, the b64 answers), `relay/Dockerfile` (new).
- **Face**: `src/gui/assets/app.js` (the transport: the fragment, the
  handshake, the wrapper, the sealed poll).
- **Tests**: `tests/integration.rs` (the relay in the loop),
  `tests/face/harness.mjs` (the RL case, 384/42).
- **Docs**: `docs/features/remote-access.md` (rewritten relay section
  + the deployment runbook), `docs/features/gui.md`,
  `docs/features/bot-app.md`, `TECHNICAL-DOCUMENTATION.md` (the
  Pairing bullet), `docs/known-issues.md` (KI-367),
  `docs/plans/2026-10-29-remote-relay-openship.md` (completed, proofs
  recorded), `CLAUDE.md` untouched (no rule changed).
- **Deployment**: OpenShip project `fleet-relay`
  (`proj_EhUyMW71ICHc5xzG`), service `relay`
  (`svc_yTCvrwn3nwbhTquS`), four deployments, the live one
  `dep_KLpwx0vlDYxm1Uwy`.

## Notes

- The face runs `fleet gui --no-open --remote --relay
  https://fleet.opsh.io` under `com.fleet.gui` (PID changed by the two
  kickstarts; the live one carries the icon fix).
- A face restart invalidates the relay session (per-launch mint) — the
  phone re-handshakes on its next page boot (the gen stamp), and the
  old QR's secret stops working, which is the same behavior the
  pairing window always had.
- Probes kept, not committed: `tmp/relay-live-probe.py` (the live
  phone role; needs a fresh mint per face restart — the stored secret
  is per-launch by design). The platform's edge refuses non-browser
  user agents before the relay; probes present a browser's.
- The soak's log: `/tmp/relay-soak.log` — ten sealed round trips, no
  churn.
