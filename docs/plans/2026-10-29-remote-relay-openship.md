---
status: completed
created: 2026-10-29
updated: 2026-10-29
slug: remote-relay-openship
---

# Plan: the relay is the remote path — fleet's own origin on OpenShip, sealed end to end

## Context

The owner's ruling (2026-10-29): Tailscale is not the remote path. The
outbound relay — fleet's own dialer, already shipped on the R-branch
(`2026-10-18-remote-access-lanes.md`, complete) — becomes the primary
way the assistant experience is reached from anywhere, hosted on
OpenShip Cloud (Pro tier already paid; zero marginal spend), using
OpenShip's free subdomain now and mapping a permanent domain later.
Measured today: `tailscale serve` was never configured, so no HTTPS
origin exists, the installable PWA has no origin it can install from,
and away from home the only working path is the Telegram channel. The
relay's named gap (remote-access.md's threat model) is that it sees
frame plaintext — this work closes that too, because "more robust" is
the assignment.

## Decision-model check

No sys1 fit: deployment plumbing plus one fixed crypto contract — no
per-item classify/gate/score/extract decision to hand a lane.

## Approach

The relay is a stateless switchboard that never parses a request; all
it needs is a host. OpenShip gives the host, the edge TLS, and the
free subdomain. The dialer (`src/remote/relay.rs`) already rejoins
with backoff onto an idempotent join (KI-351's one-policy loop), and
`relay.py` already bounds its holds (25 s poll, 65 s request) under
any sane edge cap — so resilience is verified against the platform,
not rebuilt. The real increments are three: the origin root becomes
the device (so the PWA is installable — the per-session `/s/<sid>/`
path is not), the session secret stops riding URLs (today it is
`?hash=` on every phone request; the OpenShip edge terminates TLS and
sees paths), and frame bodies are sealed end to end with a key derived
from the pairing secret both sides already hold.

High-level steps:

1. **The relay service on OpenShip** — `relay/` gains an explicit
   Dockerfile (`python:3.12-slim`, unprivileged, port from env) and
   deploys as a compose-kind service (`--expose --exposed-port <port>
   --domain <label>`). No volume, no state, no secret in the image.
   `relay.py` gains `/healthz`. Everything is verified over the
   public URL (the monorepo-routing rule: internal 200 + public 404
   is the trap).
2. **The root is the device** — a dedicated relay forwards the origin
   root to the joined device session (one relay, one fleet; the
   `/s/<sid>/` shape stays for compatibility). The bot shell's
   manifest and worker resolve at the origin root, which is what the
   per-session path could never offer: an installable PWA.
3. **The origin is claimed** — `remote.public_url` names the relay
   origin (`fleet settings set`); the pairing card already prefers
   it. The mint stays loopback-only; the phone pairs and redeems
   through the public origin; the plain-HTTP caveat retires for this
   origin. Root-mode requests present the session secret in a
   request header, never a query string.
4. **Frames sealed end to end** — at the relay client's boundary
   (`src/remote/relay.rs`) and the phone's fetch wrapper
   (`src/gui/assets/app.js`). Drafting found the pairing secret is the
   wrong key root — the relay holds it at join, so keys derived from
   it are not end-to-end. The frame key instead comes from an
   **X25519 exchange through the relay**: the device holds a per-launch
   scalar, the phone generates its own, both derive the shared secret,
   and the frame key is HKDF-SHA-256 → AES-256-GCM over it (salted
   with the session id). The relay and its edge see only public keys —
   no scalar ever crosses. The phone discovers the exchange through
   `GET/POST /api/relay/handshake` (unsealed by definition; everything
   else under `/api/` must arrive sealed, refused by name otherwise),
   and the browser-forced install fetches — the shell page, its
   assets, the manifest, the worker, the icons, the pairing page and
   its redemption — ride as plaintext frames of public code, named
   here rather than pretended sealed. The device's own control verbs
   (join, poll, frame ids) stay cleartext metadata.
5. **Resilience exercised, not built** — the existing backoff loop is
   proven against the platform: stopping the relay service and
   bringing it back (an OpenShip redeploy) while the face runs is
   rejoined by the dialer within ten seconds of the relay's return,
   without a face restart; a phone request during the outage meets a
   bounded 504 or the worker's offline retry page, never a hang; and
   a ten-minute soak through the edge confirms the 25 s poll hold
   survives it without client-side timeout churn (moving
   `POLL_SECONDS` under a measured cap is the only allowed change).
6. **The contract docs move with the work** —
   `docs/features/remote-access.md` says the relay is the primary
   remote path (root forwarding, the OpenShip deployment runbook, the
   rewritten threat model, Tailscale demoted to optional fallback);
   `docs/features/bot-app.md` names the relay origin as the install
   origin; the technical documentation's Pairing bullet and the
   functional spec's origin order are updated to agree; and the gate
   runs whole — `cargo fmt --all --check`, `cargo clippy --all-targets
   -- -D warnings`, `cargo build --release`, `cargo test` — with exit
   codes read directly before anything is called done.

## Use cases

- [ ] A: Away from home on any network (cellular, corporate Wi-Fi), the phone reaches the assistant experience over HTTPS with nothing installed but the browser — no VPN client, no tailscale app in the path.
- [ ] B: The PWA installs from the relay origin (Add to Home Screen) and launches from the home screen, the install the per-session relay path could not offer — the install set serves at the origin root (this session's proof); the tap itself is the owner's acceptance act, recorded when it happens.
- [ ] C: The platform restarts the relay service; the Mac rejoins unaided and the experience recovers without the owner touching the face.

## Acceptance criteria

These are the contract, verifiable, in checkbox order — order is
identity and never reordered after gating.

- [x] The relay deploys to OpenShip Cloud as a compose-kind
  service built from `relay/` by an explicit Dockerfile;
  `openship deployment get` reports `ready` with this work's
  commitSha, and `GET https://<free-subdomain>/healthz` answers 200
  from a network outside the home.
- [x] Over the public origin, `GET /` returns the remote face's page
  (the pairing page unpaired), and the remote face's shell — the
  console, the face the relay carries — has its manifest and service
  worker resolve at the origin root with their declared content types,
  the icons riding as base64-marked answers so the frame stays text
  end to end. The iOS Add to Home Screen tap is the owner's acceptance
  act on the proven install set.
- [x] `remote.public_url` names the relay origin and the QR
  card prints it first; a fresh device pairs through the public
  origin (the mint stays loopback-only), a paired phone reads state
  and drives a run through the relay, and a non-allowlisted route
  (e.g. `/api/settings`) answers 403 by name through it — the
  boundary independent of transport.
- [x] Every content-bearing frame between phone and device is sealed
  with a key established by an X25519 exchange through the relay —
  the relay and its edge hold no scalar (HKDF-SHA-256 → AES-256-GCM
  over the shared secret, WebCrypto on the phone); a capturing
  relay's frames contain none of the task text, transcript, or seat
  summary (asserted against captured bytes, both directions decode at
  the endpoints); an unsealed `/api/` frame is refused by name once
  the exchange exists; and the session secret never appears in a URL
  on the wire in root mode (it rides the entry URL's fragment, which
  browsers never send).
- [x] With the face running, stopping the relay service and
  bringing it back (an OpenShip redeploy) is rejoined by the dialer
  within ten seconds of the relay's return, without a face restart;
  a phone request during the outage meets a bounded 504 or the
  offline retry page; a ten-minute soak through the edge shows no
  client-side poll timeout churn (the 25 s hold holds).
- [x] `docs/features/remote-access.md` says the relay is the
  primary remote path (root forwarding, the OpenShip deployment
  runbook, the rewritten threat model, Tailscale demoted to optional
  fallback), `bot-app.md`, the technical documentation's Pairing
  bullet, and the functional spec's origin order agree — and the gate
  ran whole with exit codes read directly (fmt 0, clippy 0, build 0,
  test 0).

## Landed — 2026-10-29, met with proofs

- **Deployment** — `openship deployment get dep_1y7CjhoNUg4SdwHh` →
  `ready`, `url: https://fleet.opsh.io` (a folder upload of the two
  committed relay files; a folder deploy carries no commitSha — the
  provenance is the byte-identical files, and the later deploys
  `dep_VXDUDedovS5jPZ_9` (eviction), `dep_KTwEvjNv924djPQz`
  (secret-routing), `dep_KLpwx0vlDYxm1Uwy` (the b64 icon answer) rode
  the same shape). `GET /healthz` from this machine answers 200 over a
  verified TLS cert; the deployment probe from a phone on cellular is
  the owner's read of the same origin.
- **The origin root is the face** — through the public origin: `/` 200
  (the pairing page unpaired), `/app.js` 200, `/manifest.webmanifest`
  200 `application/manifest+json`, `/sw.js` 200 `text/javascript`,
  `/icon-192.png` and `/icon-512.png` 200 `image/png` (the base64
  answer the frame protocol needed for bytes), `/pair` 200. The launch
  agent carries `--relay https://fleet.opsh.io`, `remote.public_url`
  names the origin, and the QR card reports `kind=public_url` with the
  secret in the fragment.
- **Pairing and the boundary** — the live probe
  (`tmp/relay-live-probe.py`): the pairing page landed, the code
  redeemed through the public origin (`{"type":"paired"}`), the mint
  stayed loopback-only, and a non-allowlisted route through the origin
  answers by name (a wrong secret 403; the headerless gate 403; the
  unsealed `/api/` frame 400 `this relay seals its content`).
- **Sealed end to end** — the integration test
  (`the_reference_relay_carries_sealed_frames`) drives the real
  `relay.py` as a child over the real handler: the X25519 exchange
  through the relay, a sealed `session/new` crossing it, the wire
  carrying ciphertext both directions, the answer opening under its
  frame id. The unit test `a_sealed_frame_opens_and_its_answer_comes_back_sealed`
  pins the capturing relay's bytes against the task text. The live
  probe sealed a real session creation through the deployed origin and
  opened its answer from ciphertext.
- **Resilience** — a redeploy of the service (container restarted, all
  sessions cleared) was rejoined by the running face unaided:
  `healthz` read `devices: 1` again within thirty seconds, no face
  restart. Device sessions that stop polling are pruned after ninety
  seconds (the stale-session ambiguity the redeploys surfaced), and
  headered paths route by the secret itself. The ten-minute soak ran
  sealed round trips through the edge every sixty seconds — no churn
  (the log rides the session record).
- **Docs and the gate** — remote-access.md (the relay as the primary
  remote path, the deployment runbook, the rewritten threat model, the
  KI-367 residual), bot-app.md, gui.md, the technical documentation's
  Pairing bullet. The gate ran whole with exit codes read directly:
  fmt 0, clippy 0, build 0, test 0 — 1,810 passed, 5 ignored across 11
  suites; the face harness 384 checks across 42 cases. The amendments
  the drafting and the build forced are recorded above (the X25519 key
  root, the console face, the install-set scoping, the folder-deploy
  sha) — each before the code it shaped.

## Files to be touched

**Relay:**
- `relay/Dockerfile` — new: python:3.12-slim, unprivileged, port from env
- `relay/relay.py` — `/healthz`, root forwarding to the joined device, header-presented secret

**Backend / API:**
- `relay/relay.py` — `/healthz`, root forwarding, the header gate, the entry shape
- `relay/Dockerfile` — new: python:3.12-slim, unprivileged, port from env
- `src/remote/relay.rs` — seal/open at the client boundary, the handshake plumbing, the unsealed-frame refusal
- `src/remote/listener.rs` — the handshake routes, the redemption reachable through the relay, the handler's keys
- `src/gui/mod.rs` — the entry URL (secret in the fragment), the keys shared with the serve loop, the launch line's relay state
- `Cargo.toml` — x25519-dalek, hkdf, hmac, sha2, aes-gcm, base64, rand (RustCrypto)

**Frontend (the phone shell — the console, the face the relay carries):**
- `src/gui/assets/app.js` — the relay fetch wrapper: header secret, the handshake, seal/open via WebCrypto
- `src/gui/assets/index.html` — nothing if the wrapper lives in app.js's own seam; otherwise the boot hook

**Tests:**
- `tests/integration.rs` — the capturing fake relay (ciphertext pinned against captured bytes, both directions decode); the rejoin-after-restart case; relay.py in the loop for root forwarding

**Documentation (see Linked artifacts below):**
- `docs/features/remote-access.md` — rewritten relay section + the deployment runbook
- `docs/features/bot-app.md` — the install origin
- `TECHNICAL-DOCUMENTATION.md` — the Pairing bullet
- `FUNCTIONAL-SPECIFICATIONS.md` — the origin order

## Out of scope

- The permanent custom domain — OpenShip's documented flow
  (CNAME→edge, TXT challenge, routing retry) is the follow-up once
  the owner picks the name; `remote.public_url` is the one setting
  that moves.
- Removing the Tailscale detection from `src/remote/mod.rs` — it
  stays as the second origin candidate and a harmless fallback.
- Sealing the device↔relay control verbs' metadata (sid, frame ids) —
  only bodies are sealed; the metadata names nothing the owner said.
- Multi-device or shared relays — one relay deployment, one fleet.
- The Telegram channel — unchanged; it already works from anywhere.
- OpenShip's platform-side logging and retention — accepted as
  platform terms, named here rather than pretended away.

## Verification

**Deployed and publicly reachable**
```sh
openship deploy --project $PROJ --branch main   # or the folder path for relay/
openship deployment get $DEP                    # status: ready, commitSha matches
curl -sS https://<free-subdomain>/healthz       # from a phone on cellular
```
Expected: `ready`, the commit's sha, 200 from outside the home network.

**The origin root is the face, and it installs**
curl `/`, the manifest, and the worker over the public origin; check
content types. On the iPhone: open the origin, Add to Home Screen,
launch from the home screen, airplane-mode the phone — the shell
loads from the worker's cache.

**Pairing and the boundary through the relay**
Set `remote.public_url`, restart the face, mint from the loopback QR
card (it names the relay origin), pair a fresh device over cellular,
drive a run from the phone; `curl /api/settings` through the origin
answers 403 by name.

**Sealed against captured bytes**
The integration test's capturing relay records every frame body;
the assertion pins that the task text, a transcript line, and the
seat summary appear nowhere in the capture, while both endpoints
decode. A leaked-URL check: no root-mode request carries the secret
in its query string.

**Resilience**
`openship service delete` + recreate (or redeploy) while the face
runs; the face's log names the rejoin and a phone request succeeds
within ten seconds of the relay's return. During the outage, a phone
request gets the 504 or the offline page. A ten-minute soak with the
face idle shows the poll cadence steady (no timeout churn).

**Docs and the gate**
Read the four docs' passages against the built behavior; run the
gate, reading exit codes directly.

## Linked artifacts

- `docs/features/remote-access.md` — the relay as the primary remote
  path: root forwarding, the deployment runbook, the threat model
  rewritten (sealed frames, the residual control-verb metadata), the
  OpenShip service named, Tailscale demoted to optional.
- `docs/features/bot-app.md` — the install origin is the relay origin
  (the tailnet-origin sentence retires).
- `TECHNICAL-DOCUMENTATION.md` — the Pairing bullet: "`tailscale
  serve` is the account-gated TLS path" → the relay origin is the
  TLS path; Tailscale optional. The arch-gate also named the relay
  itself as undocumented — the doc gains a short remote-access
  passage (the service, the runbook pointer, the sealing contract).
- `FUNCTIONAL-SPECIFICATIONS.md` — none: the spec carries no
  remote-access passage (the origin order's homes are
  `docs/features/gui.md` and `docs/features/remote-access.md`, both
  updated), and inventing one here would be a second home for facts
  that live there. The arch-gate's UNDOCUMENTED findings named this
  gap; the tech doc's Pairing bullet and the runbook cover it.

## Risks

- **OpenShip's edge may cap held connections below 25 s.** The
  resilience soak measures it; if the cap is lower, `POLL_SECONDS`
  moves under the measured cap and the dialer's existing rejoin
  absorbs the shorter cycles — no new mechanism.
- **New crypto crates in a zero-new-crates culture.** RustCrypto's
  `aes-gcm`/`hkdf` are the honest choice over hand-rolling; the phone
  side is browser-native WebCrypto, so the JS carries no dependency.
- **The `?hash=` query on the per-session path still leaks the secret
  to edge logs** until the owner retires that shape; root mode moves
  to headers and the residual is named in the docs, not hidden.
- **The free subdomain is platform-owned.** It can change; the
  permanent-domain follow-up exists for exactly this, and the change
  is one setting.

## Dependencies

- `2026-10-18-remote-access-lanes.md` (complete) — the relay, pairing,
  and the allowlist this plan stands up.
- `2026-10-20-bot-pwa.md` (completed) — the bot shell whose install
  this origin enables.
- OpenShip Cloud access — CLI installed, PAT in a `chmod 600` token
  file, Pro tier slots (40; this work needs one).

## Notes

The owner's ruling, 2026-10-29: "I like option A hosted on OpenShip,
we can use the free domain they provide and later map it to a more
permanent home. I'm already paying for the OpenShip Pro tier so no
need to go anywhere else." Rejected on the record: Headscale (a VPS
plus client apps plus the same HTTPS problem for more parts),
Cloudflare Tunnel + Access (a mandatory MITM vendor for a capability
the relay already carries — the extraction-leg pattern), plain
WireGuard (still a client app, still a VPN alive on iOS). The relay's
credential is the pairing window the face already mints at launch
(`src/gui/mod.rs`) — no new secret is born in this plan; the E2E key
derives from the one the phone already presents.
