---
name: ki-371-the-sealed-frame-boundary
description: "project 2026-10-30 (KI-371, 950f52d) — pairing succeeded but every read 400'd then went silently blank; three stacked defects on the relay seal (a bodyless GET marked sealed, the sealed body read as raw bytes instead of base64 TEXT, an empty handshake catch swallowing it), plus a 65 s hang mid-restart; all fixed and verified in a real browser at both origins"
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

The owner paired through the QR and got **"a 400 error after pairing."**
Reproducing in Chromium against `https://fleetbot.opsh.io/pair?c=…#k=…`
gave the body verbatim — and it was three defects stacked, each hiding
the one before it:

1. **`/api/inbox` 400 `a sealed frame did not open: a sealed body was
   too short to carry a nonce`.** The fetch wrapper set `x-fleet-enc: 1`
   on EVERY content request, including a **bodyless GET**; the device's
   seam opened the empty body and refused it. The flag means "this BODY
   is sealed" — it is now set only when a body is actually sent, while
   the ANSWER to a content path seals whenever the exchange exists. The
   device's plaintext refusal likewise targets only a non-empty body.
2. **A blank face with zero HTTP failures.** `openSealedReply` read
   `reply.arrayBuffer()` as the sealed payload, but the sealed body is
   **base64 TEXT** (nonce ‖ ciphertext ‖ tag) — 12 bytes were sliced off
   the ASCII and AES-GCM threw `OperationError`, which the handshake's
   `.catch(() => {})` swallowed. Now `reply.text()` → base64-decode →
   slice the nonce, in BOTH shells (`app.js`, `bot.js`), and a
   handshake failure sets `data-relay-error` and `console.error`s.
3. **A 65-second hang mid-restart.** The relay kept a device row whose
   poller was gone and held the phone's request the full answer
   timeout. It now answers an instant named **503** (`fleet is
   restarting — the device is not polling`) when `last_poll` is older
   than 40 s.

**The diagnostic that narrowed it, worth reusing:** the key
fingerprints matched on both sides (device log `established, key prefix
a1b46902`; page `exportKey` of its own AES key), which CLEARED the
crypto and left only the framing. A hand-driven XHR with its own
derivation decrypted the same bytes fine — proving protocol correct and
wrapper wrong. "Both sides say the same key" is the fastest fork between
key-derivation and byte-framing bugs.

**Verified, not asserted:** pairing at the bot origin then painted the
real inbox — 50 conversations, 13 needing the owner — every `/api/inbox`
sealed and opened; the console origin painted the full console with its
build stamp. The five 403s it shows are the remote allowlist refusing
`/api/settings|home|usage|contrast` BY DESIGN (refusal bodies are sealed
too), not defects. Gate: fmt/clippy/build/test 0, harness 393/43.

**Operational tidbits:** restarting the face while a phone holds a
poll makes every request during that window slow or 503 — wait for
`{"devices":1}` on both `healthz` endpoints and let the stale device
expire (~90 s TTL) before judging a run; a scripted repro needs a
20 s settle after pairing, not 4 s.

Related: [[pairing-at-an-origin-arms-the-transport]],
[[the-relay-session-is-durable-ki-370]],
[[e2e-keys-cannot-come-from-a-secret-the-intermediary-holds]],
[[bots-surface-mints-the-bot-pairing-card]].
