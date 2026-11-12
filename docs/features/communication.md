# The communication layer — the responsible omnichannel edge (armada L5)

The channel SDK stays the plugin surface; L5 wraps it in four responsible
contracts that compose the bedrock (L1 identity, L2 journal, L3 security)
rather than re-implementing it. Nothing is trusted by assertion: a peer's
authority is a measured pairing resolved to a principal, and a send's
success is the transport's own outcome.

## The four contracts

- **The durable outbox** (D30): a reply is journaled *before* the attempt,
  attempted inline (the synchronous happy path is unchanged), settled
  `delivered` on success, requeued on failure. A retry consumer with a
  declared backoff curtails to a named `undeliverable` after a bounded
  attempt budget — never a silent loss. Delivery is at-least-once.
- **The identity graph** (D31/D32): `identity::resolve(home, channel, peer)`
  resolves a channel peer to one principal through peer→principal edges in
  `identity.json` (L1's store, never a second). The same human on Telegram,
  webhook, and a paired device links to one principal. An unlinked peer is a
  **named refusal**, never a silent `owner` fallback (a confused-deputy
  escalation). The edges are governance, written at pairing, immutable to
  the agent (D19).
- **The bus is the journal** (D33): `Driver::watch` delivers live events
  from the broadcast and recovers from its persisted journal cursor on a lag
  or restart — the broadcast's silent `Lagged` drop is retired. A lagged or
  restarted driver loses no seat or outcome.
- **Gated dispatch** (D34): `Driver::deliver` resolves the peer's principal,
  checks the edge capability (`Task`/`Observe`, retained), and runs under
  that principal — it rides `Command::Run` into `AgentOptions.principal` so
  the guard (L3) reads it. Every dispatch settles a `dispatch` journal event
  (channel, peer, principal, session, admission), graded against the run's
  outcome (D35).

The telegram poll offset persists (`<home>/channels/telegram-offset`) so a
restart resumes rather than re-polls from zero (C5).

## Non-breaking bootstrap

Every paired peer and device links to `owner` (lane 1); the owner wildcard
grants what it grants today; an Observe peer is still refused by name. The
outbox attempts inline so the synchronous happy path — and the tests that
pin it — is unchanged; only the failure path becomes durable. The principal
is additive on the run, the journal hook is additive, calib stays advisory.

## Surface

`fleet channels` / the inbox render the outbox depth, the consumer lag, and
each peer's resolved principal. The relay `Channel` impl composes L6 push
(D36); its live transport is L6's.

## Verification

`cargo test -p fleet --lib channels:: identity::` pins: the outbox
failed-then-delivered recovery + the named `undeliverable` curtail + the
synchronous happy path; cross-channel + device resolution to one principal;
the unlinked-peer named refusal; the dispatch event carrying channel, peer,
principal, admission; the telegram offset persisting across a restart; the
watcher presenting the seat + posting the outcome. The full gate (2234 lib +
172 integration + 16 install) and the face harness confirm single-owner
behavior is unchanged.