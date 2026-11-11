---
status: draft
created: 2026-11-11
updated: 2026-11-11
slug: armada-l6-relay
---

# Plan: armada L6 — the relay transport, a self-contained nervous system

## Context

Armada's control lane rides the relay, which is already a two-way sealed
frame tunnel (dial-out, sid addressing, X25519 + AES-GCM). But two gaps
block armada: the **broker is `relay/relay.py` — Python, in-memory**, a
hard-rule-12 self-containment violation; and the protocol is
request→one-answer, device-pull only, so there is **no push** (the phone's
`/events` degrades to polling). This plan makes the transport self-contained
and push-capable — atoms L6.1–L6.3. The control *vocabulary* is L8's; L6
provides the frame kind and the lane it rides.

## Design choices inherited (the ledger)

Conforms to **D0–D6** (shared), **D7–D9** (L1: the install-id + principal a
control frame carries), **D18** (L3: the assertion the frame carries,
Ed25519). Honors **D6**: the broker becomes Rust (no foreign runtime); the
fleet-side sealing client is unchanged.

## Decision-model check

Deterministic transport — route sealed frames by sid, push, ack. The broker
is dumb (it parses no verb, holds no authority). **No sys1 job fits.**

## Approach

A new `src/remote/broker.rs` is a self-contained Rust broker, run as a mode
of the same binary (`fleet relay --broker`) — no separate runtime, no
Python. It routes sealed frames by sid exactly as `relay.py` does, but adds
**push with ack + cursor**: an unsolicited frame is queued for a sid,
delivered, and acked; a client that missed a poll detects the gap from the
cursor and re-requests. The frame gains a `kind: "control"` discriminator
carrying `{verb, install, principal, assertion?, payload, nonce, issued_at}`
(D7/D8/D18), sealed by the existing crypto — the broker routes it dumbly.
The sealing (X25519 + HKDF + AES-GCM, AAD bound to path/frame-id) is
unchanged. `relay/relay.py` retires.

**Trust is earned by measurement here too** (the ledger's cross-cutting
obligation, L6 row): a frame is trusted *delivered* only when it is acked —
never because it was sent; an unacked frame is a named gap the cursor
exposes and the client re-requests, not a silent loss. The broker measures
queue depth and consumer lag and names a breach; a sid's liveness is earned
by its poll/heartbeat, not assumed by registration (L8's roster consumes
this). The transport reports its outcomes so the layers above can grade
them.

High-level steps:

1. `src/remote/broker.rs`: the Rust broker (sid → frame queue), run as
   `fleet relay --broker`; the join-proof + sid addressing preserved.
2. Push/ack/cursor: unsolicited frames queued per sid, delivered + acked;
   a gap is detectable and re-requestable.
3. The `kind: "control"` frame discriminator (the vocabulary is L8's); the
   sealing unchanged.
4. Retire `relay/relay.py`; `fleet relay` / doctor reports broker status.

## Acceptance criteria

- [ ] C0: a Rust broker (`fleet relay --broker`) routes sealed frames by sid; the existing fleet client connects unchanged; no Python is involved (self-contained).
- [ ] C1: the broker pushes an unsolicited frame to a sid with ack + cursor; a client that missed a poll detects the gap and re-requests (a test pins delivery + ack + gap recovery).
- [ ] C2: a `kind:"control"` frame carries `{verb, install, principal, assertion?, payload, nonce, issued_at}` sealed by the existing crypto; the broker routes it without parsing the verb.
- [ ] C3: the sealing is unchanged — a tampered frame is rejected (AAD bound to path/frame-id); a test pins it.
- [ ] C4: relay-down is a named degradation (the client names it, never a silent stop); the four-leg gate passes.
- [ ] C5: `fleet relay` / doctor renders the broker status and connected sids.
- [ ] C6: delivery is trusted only on ack — a test pins that an unacked frame is a named gap (re-requestable), the broker measures queue depth/lag and names a breach, and a sid with no recent poll/heartbeat is reported not-live (liveness earned, not assumed).

## Files to be touched

**Backend:**
- `src/remote/broker.rs` — new: the Rust broker.
- `src/remote/relay.rs` — push/ack/cursor + the control-frame kind.
- `src/remote/mod.rs` — `mod broker`.
- `src/cli.rs`, `src/main.rs` — `fleet relay --broker` + the status verb.
- `src/doctor.rs` — the broker check.

**Retired:**
- `relay/relay.py` — replaced by the Rust broker (the Dockerfile retires with it).

**Tests:**
- `src/remote/broker.rs`, `src/remote/relay.rs` — routing, push/ack/gap, sealing, control frame.

**Documentation:** see Linked artifacts.

## Out of scope

- The control vocabulary (dispatch/state/corpus-sync/…) — L8.
- The flagship role, roster, heartbeat — L8.
- Authority-transport verification at the instance guard — L8 consumes L3.
- SSH/QUIC alternative transport (muxin-style) — a later option.
- A durable broker queue (in-memory + ack/cursor now; durability a later hardening).

## Verification

**C0/C1/C2:** `cargo test -p fleet --lib remote` — routing, push/ack/gap
recovery, the control frame; an integration test runs the broker + a client.

**C3:** a tampered-frame test asserts rejection.

**C4:** the four-leg gate; a relay-down test asserts the named degradation.

**C5:** `fleet relay` renders status.

## Linked artifacts

- `docs/features/relay.md` — update: the Rust broker + push + control frame.
- `TECHNICAL-DOCUMENTATION.md` — the relay transport contract.
- `ARMADA-DESIGN-DECISIONS.md` — append D20–D22 (below).
- `../architecture/2026-11-11-armada-atomic-decomposition.md` — L6 atoms ticked when shipped.

## Risks

- **Self-containment of the broker** — it must ship in the bundle or be built by install. Mitigation: a mode of the same binary (`fleet relay --broker`), no separate runtime.
- **Push changes the protocol** — the phone client must handle push. Mitigation: backward-compatible (poll still works; push additive).
- **In-memory broker queue** — a broker restart loses queued frames. Mitigation: the ack/cursor lets a client detect + re-request; broker restart is a named degradation; a durable queue is a later hardening.

## Dependencies

- `2026-11-11-armada-l1-identity.md` (install-id + principal on the frame); `2026-11-11-armada-l3-security.md` (D18, the assertion the control frame carries). Inherits ledger D0–D19.

## Design choices this layer settles (append to the ledger)

- **D20 — the broker is Rust, a mode of the fleet binary (`fleet relay
  --broker`), dumb, routing sealed frames by sid; `relay/relay.py` retires.**
  *Constrains:* L8 (the control lane), self-containment (hard rule 12).
- **D21 — push with ack + cursor: unsolicited frames queued per sid,
  delivered + acked; a gap is detectable and re-requestable.** *Constrains:*
  L8 (flagship→instance push), L5 (the relay channel).
- **D22 — the frame gains `kind:"control"` carrying `{verb, install,
  principal, assertion?, payload, nonce, issued_at}`, sealed by the existing
  crypto; the broker parses no verb.** *Constrains:* L8 (the vocabulary rides
  this kind), the dumb-broker invariant (§8.5).

## Notes

The transport is armada's nervous system, made self-contained. The broker
stays dumb (all intelligence at the endpoints — the flagship signs, the
instance verifies); L6 only adds the Rust broker, push, and the control-frame
kind. Approve this and the ledger carries D20–D22 into L8.