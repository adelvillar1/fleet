# The relay broker — the self-contained nervous system (armada L6)

A dumb switchboard between a phone (or the flagship) and a fleet: it routes
sealed frames by session id (`sid`) and understands nothing of their content
(D20, §8.5). It replaces `relay/relay.py` — a mode of the same binary
(`fleet relay --broker`), no Python, no foreign runtime (hard rule 12). The
fleet-side client (`src/remote/relay.rs`) connects unchanged; the sealing
(X25519 + HKDF + AES-GCM, AAD bound to path/frame-id) is end-to-end between
the endpoints and untouched — the broker routes opaque frames.

## What L6 adds over the reference broker

- **Push with ack + cursor** (D21): an unsolicited frame is queued for a sid
  with a sequence number; the poll hands it out with `push_seq`, and an ack
  advances the cursor. A frame is trusted *delivered* only when acked — an
  unacked frame is a named gap the cursor exposes and the client re-requests,
  never a silent loss (the cross-cutting trust obligation).
- **The control-frame discriminator** (D22): a frame may carry
  `kind:"control"` with `{verb, install, principal, assertion?, payload,
  nonce, issued_at}` (D7/D8/D18). The broker routes it dumbly — it parses no
  verb, holds no authority. The vocabulary is L8's.
- **Liveness earned, not assumed** (D4): a sid is live only while it polls
  (a 90s window); the broker measures queue depth and names a breach past
  1024 frames. The roster (L8) consumes this.

The queue is in-memory (a durable broker queue is a later hardening); the
ack/cursor lets a client detect and re-request a gap across a restart.

## The contract (for the layers above)

```text
Broker::new(join_secret)                  // empty secret fails closed
broker.join(sid, secret, presented)       // the join proof gates registration
broker.push(sid, frame) -> seq            // queue a frame (control or face)
broker.poll(sid) -> [frames with push_seq]
broker.ack(sid, seq) -> { acked, next_seq, gap }
broker.status() -> { devices: [{sid, live, queued, acked, next_seq}], count }
broker.secret_ok(sid, presented) / prune()
broker::serve(port, join_secret)          // the HTTP mode
```

HTTP endpoints: `POST /join`, `GET /poll?sid=`, `POST /frame?sid=`,
`POST /push?sid=`, `POST /ack?sid=`, `GET /status`, and the phone's forward
`/s/<sid>/<path>`. See `docs/plans/ARMADA-CONTRACTS.md` §7 and
`ARMADA-DESIGN-DECISIONS.md` D20–D22.

## Surface

- **`fleet relay --broker [--port <p>] [--join-secret <s>]`** runs the broker
  (the join secret also resolves from `FLEET_RELAY_JOIN_SECRET`; absent, every
  join is refused by name).
- **`fleet relay [--url <u>]`** reports a running broker's status.

## Verification

`cargo test -p fleet --lib remote::broker` pins: join registration + wrong
proof refused + taken sid refused + idempotent re-join; a no-secret broker
fails closed; push queues + poll hands out with a sequence; delivery trusted
only on ack + a gap named; a control frame routes without the broker parsing
its verb; liveness earned by polling + stale pruned; the queue bound names a
breach; an unknown sid refused by name. A live HTTP exercise confirms
join → push → poll → ack → status end to end, and the fail-closed join.