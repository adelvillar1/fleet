---
name: e2e-keys-cannot-come-from-a-secret-the-intermediary-holds
description: "feedback 2026-10-29 — a frame key derived from a secret the relay also holds (and whose TLS edge terminates and reads the join) is not end-to-end encryption; the endpoints need a Diffie-Hellman exchange the intermediary only watches, and the plan's crypto section must be amended before building, not after"
type: feedback
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

The relay plan's drafted crypto ("frame key = HKDF over the pairing
secret both sides already hold") was **provably wrong before any code**:
the relay LEARNS that secret at join (it must, to gate the session), and
the OpenShip TLS edge terminates and reads the join body too — so keys
derived from it are readable by both. "End to end" means the key root is
something NEITHER the intermediary nor its edge ever sees.

**Why:** the only honest construction is a key exchange through the
intermediary — X25519 where each endpoint holds a scalar and the wire
carries only public keys. Pre-shared alternatives all fail here: the
pairing secret reaches the relay at join; the pairing CODE is redeemed
through the relay in plaintext; the device token is HttpOnly so the
phone's JS cannot read it.

**How to apply:** (1) When a plan says "sealed/E2E", re-derive who holds
every candidate key root BEFORE writing the criterion — the drafting
caught this only because the recon read the join flow. (2) The shipped
shape, reusable: `GET /handshake` returns the device's public + a gen
stamp + the session id (the HKDF salt); the phone generates a scalar,
derives, `POST /handshake` announces its public; both derive
HKDF-SHA-256(salt=session id) → AES-256-GCM, nonce in the body, frame id
or request path as the AAD so bodies cannot be replayed onto other
frames. Per-launch rotation handled by the gen stamp (a phone
re-handshakes when the device relaunched). (3) House rule held: the plan
criterion was amended FIRST, then built — mid-implementation design
corrections update the contract before the code.
