---
name: fleet-phone-pairing-two-reference-architectures
description: "2026-10-18 owner exploration — fleet phone/remote access needs the union of two external designs (ZCode's relay+UI in [[zcode-remote-session-bot-gateway-architecture]], OpenMausBot's pairing/trust/transport); the five gaps this named are now all closed by [[fleet-remote-access-implemented]]"
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

Owner question 2026-10-18: "what would it take to add a phone pairing and/or a
proxy/relay to access sessions on phones and other devices while using fleet?"
Research turn. The owner then named **two pieces deliberately**: the
QR-to-Safari-to-sessions flow (ZCode's Web Remote Control, dissected in
[[zcode-remote-session-bot-gateway-architecture]]) **and**
github.com/milind-soni/OpenMausBot, which supplies the layer ZCode's relay
assumes: **how the phone is allowed in**. The eventual design is the union,
not either alone.

**OpenMausBot's model** (read `shared/pairing-link.ts`,
`companion/src/{devices,listener,routes,endpoints}.ts`, `docs/self-hosting.md`,
`docs/desktop-companion.md`): loopback is **owner trust**, but a **browser never
is** (any request with `Sec-Fetch-Site`/`Sec-Fetch-Dest`, or a WebSocket
handshake with an `Origin`, needs a session); pairing is a 12-char single-use
code (5 minutes, 5 attempts) or an `openmausbot://pair?address=…&token=…&code=…`
link whose QR pins a **P-256 HPKE `secretKey`**; device bearer tokens stored as
**sha256 only**; the companion listener is **default-deny** (every route
explicitly allowlisted, responses scrubbed, per-device capabilities default
off); three transports — Tailscale MagicDNS+HTTPS, a pinned `cloudflared`
tunnel landing on a separate listener treated as "through a proxy" (never
owner), own domain; QR endpoint candidates ranked hosted→tailnet→LAN→bonjour,
cap 8; phone app = install-to-home-screen.

**The five gaps this exploration named, all since closed** by
[[fleet-remote-access-implemented]]: (1) one token per launch, never
persisted → `fleet pair`/`fleet devices` with sha256-only device tokens;
(2) `server::loopback_address` refusing any non-loopback host → a second,
explicitly-remote listener with a default-deny allowlist; (3) sessions
could not be grouped by project → a `workspace` field on the log's `Start`
record; (4) the seat's first-answer-wins race → seat arbitration naming who
answered; (5) a desktop-first web app → the phone layout and the
project-grouped inbox.

**Why:** the two references are the design's whole justification, and the
OMB trust model is the half that is easy to skip — a phone that can drive a
run must not also be able to revoke devices or write settings, and the
default-deny route list is what enforces that rather than a comment.

**How to apply:** when a future remote-access change adds a route to the
remote listener, ask which allowlist entry covers it and add it
deliberately; a route the loopback face has is not thereby a route a paired
device may call. Related: [[fleet-lane-service-target-architecture]] (the
design), [[fleet-remote-access-implemented]] (what landed),
[[gui-revamp-browser-face-decision]].
