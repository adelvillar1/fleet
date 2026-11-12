---
status: completed
created: 2026-11-11
updated: 2026-11-12
slug: armada-l5-communication
---

# Plan: armada L5 — communication, the principal at the edge

## Context

The communication edge is where armada's instances converse instead of merely
receiving commands. Fleet already has a working plugin SDK — the `Channel`
trait (`src/channels/mod.rs:636-672`: `name/serve/reply/present_seat`), a
`Driver` + `Policy` that turns one inbound delivery into one run in the peer's
own session, and webhook + telegram plugins. But four responsible contracts
are missing, and each is a silent failure today:

- **No durable outbox.** `reply`/`present_seat` are `let _ = ureq::post`
  (`src/channels/webhook.rs:140-143`, `src/channels/telegram.rs:115-117` and
  `129-131`) — a failed POST is lost, and the transport's outcome is thrown
  away, so nothing can grade it.
- **No identity graph.** A `Peer` is `(channel, id)` by explicit design — "the
  same human on Telegram and on a webhook is two peers with one owner"
  (`src/channels/mod.rs:67-93`, pinned by test). There is no resolution to a
  principal, so the guard cannot see one authority across channels.
- **No durable bus.** `Driver::watch` (`src/channels/mod.rs:792-875`) consumes
  the bridge's ephemeral broadcast and silently continues on `Lagged`
  (`mod.rs:871`) — a lagged or restarted driver loses seats and outcomes. The
  telegram poll offset is in-memory only (`telegram.rs:52`), so a restart
  re-polls. The channel activity log is a capped-500 rewrite
  (`src/channels/service.rs:48,200-229`), not a durable trail.
- **No gated dispatch.** `Driver::deliver` authorizes the peer's capability
  (`Policy::authorize`, `mod.rs:295-301`) then calls `bridge.run(&session,
  &task)` (`mod.rs:929-931`) — the peer evaporates: `bridge.run`
  (`src/gui/bridge.rs:4642-4648`) and `Command::Run` (`bridge.rs:80`) carry no
  principal, so the run is an unscoped owner run.

This plan builds the four atoms L5.1–L5.4 as responsible contracts that
compose the bedrock (L1 identity, L2 journal, L3 security) rather than
re-implementing it. It is wave 3 of the decomposition (§4).

## Design choices inherited (the ledger)

Conforms to **D0–D6** (shared): the responsible-citizen shape (D3), trust
earned by measurements (D4), the stack (D5), and non-breaking self-containment
(D6 — owner holds all, calib stays advisory, embedded Rust bus, JSONL before
SQLite). Builds on **D7–D9** (L1: the install-id and the principal the graph
resolves to and the run carries), **D10–D13** (L2: the journal the outbox and
bus ride, the durable named consumers, the additive `record()` hook), and
**D14–D19** (L3: the grant schema, the uniform guard the dispatch runs through,
the per-principal audit, the governance boundary the identity graph joins).
Composes **D20–D22** (L6: the push + control frame the relay channel consumes).
Honors D6 hard: nothing the owner does today starts being refused.

## Decision-model check

Deterministic communication plumbing — persist/retry/curtail a send, resolve a
peer to a principal, replay a journal cursor, gate a dispatch through the
guard. No per-item classify/gate/score/extract decision; **no sys1 job fits**
(authority is code-enforced at L3, resolution is a lookup, retry is a declared
schedule that learns from its settles).

## Approach

One path: keep the `Channel` SDK as the plugin surface and wrap it in four
responsible contracts, each discharging its discipline *through* the layer
below (D5). The outbox settles through the journal (L2), the identity graph
resolves through L1's principal store, the bus **is** the journal (no second
broker), and gated dispatch authorizes through L3's guard. The
responsible-citizen shape (D3) is baked in, not left to the caller: a dispatch
is admitted (edge capability + guard + capacity) or refused by name; a send is
*measured* (the transport returns its outcome instead of `let _`), settled,
retried, and curtailed named; a consumer measures its lag and replays rather
than dropping; every dispatch is graded against its run's `finished`/`failed`
settle (D4). Nothing is trusted by assertion — a peer's authority is a measured
pairing resolved to a principal, and a send's success is the HTTP response.

The non-breaking bootstrap keeps single-owner behavior exactly. Every paired
peer and device links to `owner` (D31); the owner wildcard (D15) grants what it
grants today; the edge capability (`Task`/`Observe`) is retained as the
channel-edge admission so an Observe peer is still refused by name (D34). The
outbox attempts inline so the synchronous happy path — and the tests that pin
it — is unchanged; only the failure path becomes durable and retried. The
principal is additive on the run (D9), the journal hook is additive (D13), and
calib stays advisory (D6).

High-level steps:

1. **L5.1 — durable outbox.** New `src/channels/outbox.rs`: `outbox.send`
   journals the reply (`kind="outbox"`) *before* the attempt, attempts inline
   through the channel, settles `delivered` on success, requeues on failure; a
   single durable named consumer (`outbox-worker`, D12) retries with a declared
   backoff and curtails to a named `undeliverable` after a bounded attempt
   budget. `Channel::reply`/`present_seat` return `Result<(), SendError>`;
   webhook + telegram report their outcome; `Driver::watch` routes sends
   through the outbox.
2. **L5.2 — identity graph.** Extend `src/identity.rs` (L1) with
   `resolve(peer)` / `resolve_device(device)` reading peer→principal edges in
   `identity.json` (D8, one store). Pairing writes the edge (lane 1 → `owner`);
   an unlinked peer is a named refusal, never a silent `owner` fallback; the
   edges are governance, immutable to the agent (D19).
3. **L5.3 — bus over the journal.** `bus.publish`/`bus.subscribe` are thin over
   `journal.append`/`journal.subscribe` (D10/D12). `Driver::watch` becomes the
   durable named consumer `channel-driver`, replaying seat/finished/failed from
   its persisted cursor — retire the broadcast `Lagged` drop. Persist the
   telegram poll offset so a restart resumes.
4. **L5.4 — gated dispatch.** `Driver::deliver` resolves peer→principal (D31),
   checks the edge capability, then runs under that principal: the resolved
   principal rides the run command into `AgentOptions.principal` (L1's field,
   D9) so the guard (D15) sees it. Settle a `dispatch` journal event and grade
   it against the run's outcome (D35).
5. **Surface (hard rule 11).** `fleet channels` / the inbox render the outbox
   depth + `undeliverable`, the consumer lag, and each peer's resolved
   principal. The relay `Channel` impl composes L6 push (D36); its live
   transport is L6's.

## Acceptance criteria

- [x] C0: a channel reply whose first POST fails is persisted in the outbox and delivered once the callback recovers (a test fails then succeeds the POST and pins delivery) — no reply is silently lost.
- [x] C1: a reply that fails past the attempt budget is curtailed to a named `undeliverable` degradation recorded in the journal and the channel surface (a test pins the bound and the name).
- [x] C2: `identity.resolve` returns one PrincipalId for a Telegram peer, a webhook peer, and a paired device linked to the same principal; lane 1 resolves `owner` (a test pins cross-channel + device resolution).
- [x] C3: an unlinked peer resolves to a named refusal, never a silent `owner` fallback, and the peer→principal edges are not writable by the agent (a test pins both).
- [x] C4: `Driver::watch` consumes the journal from a persisted cursor and delivers a seat/outcome emitted during a simulated lag or restart (a test pins no-loss), replacing the broadcast's `Lagged` drop.
- [x] C5: the telegram poll offset persists across restart so a restarted channel resumes rather than re-polling from zero (a test pins it).
- [x] C6: an inbound delivery runs under the resolved principal — the run's `AgentOptions.principal` equals the resolved principal and the guard reads it (a test pins the principal on the run).
- [x] C7: single-owner behavior is unchanged — an Observe peer is still refused by name, an owner Task peer still dispatches, the four-leg gate passes, and the existing channel tests' behavior is preserved.
- [x] C8: every inbound dispatch settles a `dispatch` journal event carrying channel, peer, resolved principal, session, admitted/refused, and outcome (a test pins it).

## Files to be touched

**Backend:**
- `src/channels/outbox.rs` — new: the durable outbox, the retry consumer, the named curtail.
- `src/channels/mod.rs` — `Channel` send methods return `Result`; `Driver` routes sends through the outbox; `Driver::watch` becomes a journal consumer; `Driver::deliver` resolves the principal + gated dispatch; `mod outbox`.
- `src/channels/webhook.rs` — `reply`/`present_seat` report their outcome.
- `src/channels/telegram.rs` — `reply`/`present_seat` report their outcome; the poll offset persists.
- `src/channels/service.rs` — the sink/watcher wire the outbox + consumer; the surface reads outbox/lag/principal.
- `src/identity.rs` — `resolve(peer)`/`resolve_device` + the peer→principal edges (extends L1).
- `src/gui/bridge.rs` — the principal-aware run (the command carries the principal into `AgentOptions.principal`); the `dispatch` journal event.
- `src/remote/pairing.rs` — pairing writes the peer/device→principal edge.
- `src/cli.rs`, `src/main.rs` — `fleet channels` renders the outbox/lag/principal.

**Tests:**
- `src/channels/outbox.rs`, `mod.rs`, `webhook.rs`, `telegram.rs` — retry/curtail, cross-channel resolve, consumer no-loss, offset persistence, gated dispatch, dispatch settle.

**Documentation:** see Linked artifacts.

## Out of scope

- The live relay transport (Rust broker, push/ack, control frame) — L6; L5 settles the transport-agnostic contract and resolves a relay frame's device to a principal.
- The control vocabulary, roster, flagship role, assertion issuance — L8.
- Multiple principals / the org user store / roles-as-templates — lane 2 (the contracts are principal-agnostic now).
- Workspace work-object events — L7 (rides the same bus).
- Migrating the face's SSE stream to a journal projection — a later L5/L8 step (D13 keeps the broadcast additive now).
- Promoting calib or merging the session log (forbidden by D6).

## Verification

**C0:** `cargo test -p fleet --lib channels::outbox` — a fake callback fails the first POST then recovers; the test asserts the reply was persisted and is delivered on retry (no silent loss).

**C1:** `cargo test -p fleet --lib channels::outbox` — a callback that never recovers hits the attempt budget; the test asserts the named `undeliverable` curtail in the journal and the channel surface (bound + name pinned).

**C2:** `cargo test -p fleet --lib identity` — a Telegram peer, a webhook peer, and a device linked to one principal all resolve to it; lane 1 resolves `owner`.

**C3:** `cargo test -p fleet --lib identity` — an unlinked peer resolves to a named refusal (not `owner`); an agent attempt to write the peer→principal edges is refused by name.

**C4:** a channels test journals a seat/finished event while the `channel-driver` consumer is paused/restarted, then asserts delivery on resume from the persisted cursor (no loss).

**C5:** a telegram test advances the offset, drops the channel, recreates it, and asserts the poll resumes at the persisted offset (not zero).

**C6:** a channels test delivers an inbound and asserts the run's `AgentOptions.principal` equals the resolved principal and the guard decision names it.

**C7:** the four-leg gate (`cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings`, `cargo build --release`, `cargo test`); the existing webhook/telegram tests (Observe refusal, Task acceptance) pass unchanged in behavior.

**C8:** a channels test delivers an inbound (accepted and refused) and asserts the `dispatch` journal event's channel/peer/principal/session/admitted/outcome fields.

## Linked artifacts

- `docs/features/communication.md` — new: the Channel SDK + outbox + identity graph + bus + gated dispatch contract.
- `TECHNICAL-DOCUMENTATION.md` — the communication contract.
- `ARMADA-DESIGN-DECISIONS.md` — append D30–D36 (below).
- `../architecture/2026-11-11-armada-atomic-decomposition.md` — L5 atoms ticked when shipped.

## Risks

- **`Channel` send-signature fan-out** — `reply`/`present_seat` returning `Result` touches both plugins, the driver, and every test that calls them. Mitigation: update all sites in the same commit; `scripts/edit-preflight.sh` names them; the inline attempt keeps the observable happy path (and its pins) unchanged.
- **Outbox double-send** — at-least-once delivery means a crash between send and settle can duplicate a reply. Mitigation: settle-before-requeue, the worker drains only unsettled entries; a duplicate reply is named and preferred over a lost one (D3).
- **`Command`/run principal fan-out** — carrying the principal into the run touches the `Command` enum and `AgentOptions` initializers (struct-field fan-out, structure-guards §E). Mitigation: default `owner`, update every constructor in the same commit; the guard's owner wildcard keeps lane 1 unrefused.
- **Watch timing** — moving `Driver::watch` from the broadcast to the journal changes delivery latency (fsync cadence, D11). Mitigation: the consumer reads promptly and measures its lag (D4); the journal hook is additive (D13) so the face still streams.
- **Capability vs authority** — conflating the edge capability with the principal grant would break the Observe refusal. Mitigation: capability stays the edge admission, the guard is additive (D34); a test pins Observe still refused.

## Dependencies

- `2026-11-11-armada-l1-identity.md` (the principal + device map the graph resolves through), `2026-11-11-armada-l2-journal.md` (the durable home + named consumers), `2026-11-11-armada-l3-security.md` (the guard the dispatch runs through). Inherits ledger D0–D19.
- Composes `2026-11-11-armada-l6-relay.md` (D20–D22) for the relay channel; the live transport is L6's (wave 2, lands before L5).

## Design choices this layer settles (append to the ledger)

- **D30 — the durable outbox.** `Channel::reply`/`present_seat` return a named
  `Result<(), SendError>` (the transport reports its outcome — measured, not
  asserted). Every outbound goes through `outbox.send`: the reply is appended
  to the journal (`kind="outbox"`) *before* the attempt, attempted inline (the
  happy path stays synchronous), settled `delivered` on success; a failure is
  requeued and a single durable named consumer (`outbox-worker`, D12) retries
  with a declared backoff that learns from its settles, curtailing to a named
  `undeliverable` degradation after a bounded attempt budget. Durability is the
  journal's (D11), not a second log; delivery is at-least-once (a crash between
  send and settle may duplicate, never lose). *Rationale:* today
  `let _ = ureq::post` (`webhook.rs:140`, `telegram.rs:115`) loses a failed
  reply — a silent failure (D3). *Constrains:* L6 relay-channel sends, L8
  instance→flagship replies, lane 2 org chat/mail — every outbound rides the
  outbox.
- **D31 — the identity graph.** `identity.resolve(peer) -> PrincipalId`
  resolves a `Peer (channel, id)` or a `DeviceId` to one principal through
  edges stored in `<home>/identity.json` (extending L1's single store, D8 — one
  home, not a second). Lane 1: every paired peer and device links to `owner`;
  the "two peers with one owner" by accident (`mod.rs:67-72`) becomes "two
  peers, one principal" by explicit edge. Composes L1 `device_principal` (D8)
  and resolves to the same principal the run carries (D9). *Rationale:* the
  principal must be the unit at the communication edge so a Slack message and
  a relay frame resolve to one authority (D1/D2). *Constrains:* L5 gated
  dispatch, L6 relay channel, L8 frame principal, lane 2 org user store.
- **D32 — resolve is total-or-named-refusal; the graph is governance.** An
  unlinked peer resolves to a named refusal (`Refusal::Unknown`), never a
  silent fallback to `owner` (a fallback would be a confused-deputy
  escalation). The peer→principal edges are governance (D19): immutable to the
  agent, linked only by the owner/control plane at pairing. *Rationale:* trust
  is earned by a measured pairing, not asserted by the message (D4); the edge
  is an authority boundary. *Constrains:* L5 dispatch admission, L8
  pairing/link, lane 2 (linking a user's channels is a governance act).
- **D33 — the bus is the journal.** `bus.publish(event)` = `journal.append`;
  `bus.subscribe(consumer)` = `journal.subscribe(consumer, from)` (D12) — no
  second broker, embedded Rust (D6). `Driver::watch` becomes the durable named
  consumer `channel-driver`, replaying seat/finished/failed from its persisted
  cursor instead of the lossy broadcast; the `Lagged → continue` silent drop
  (`mod.rs:871`) is retired. Channel inbound cursors persist too (the telegram
  offset, `telegram.rs:52`, becomes a durable cursor so a restart resumes, not
  re-polls). *Rationale:* a lagged/restarted consumer must never lose an event
  (D3/D12); the broadcast drops on lag today. *Constrains:* L7 workspace
  events, L8 cross-install tracing, any channel/agent consumer.
- **D34 — gated dispatch threads the principal.** `Driver::deliver` resolves
  peer→principal (D31), checks the edge capability (`Task`/`Observe`, retained
  as the channel-edge admission), then runs under that principal: the resolved
  principal rides the run command into `AgentOptions.principal` (D9) so the
  guard (L3, D15) sees it — the peer no longer evaporates at `bridge.run`
  (`mod.rs:929-931`). Capability is the edge admission composed with, not
  replaced by, the principal guard. *Rationale:* non-breaking (D6) — an Observe
  peer is still refused by the edge, an owner Task peer still dispatches under
  the owner wildcard; lane 2 bounds a non-owner principal further. *Constrains:*
  L7 bot invocation, L8 control dispatch, lane 2 per-user dispatch.
- **D35 — the communication edge settles and grades every dispatch.** Every
  inbound dispatch settles a `dispatch` journal event (channel, peer, resolved
  principal, session, admitted/refused, outcome); the outcome is graded against
  the run's `finished`/`failed` settle (D4), not asserted; outbox depth and
  consumer lag are measured, and a breach curtails named (`undeliverable`,
  relay-down). *Rationale:* "graded, not just recorded" (D4) — the guard audit
  (D17) is the authority record; this is the communication-edge record that
  earns trust by measurement. *Constrains:* L8 dispatch grading/autonomy, lane 2
  per-user communication audit.
- **D36 — one principal at the omnichannel edge; the relay is a channel.** The
  `Channel` SDK + outbox + identity graph are transport-agnostic, so the relay
  plugs in as a `Channel` impl consuming L6 push (D21) + control frames (D22);
  a relay frame's device resolves through the same identity graph (D31) to the
  same principal as a channel message, so the guard sees one authority across
  the omnichannel edge. The org's chat and mail reuse the same SDK + bus.
  *Rationale:* instances converse (not just receive commands); the edge is
  principal-unified by construction (D1/D2/D5). *Constrains:* L6 relay channel,
  L8 control lane, lane 2 org chat/mail.

## Notes

L5 is the edge where the stack becomes a conversation. The four contracts are
additive and non-breaking: the SDK stays, the owner keeps every grant, the
Observe refusal stands, and the synchronous happy path is preserved — only the
silent failures (lost reply, evaporated peer, dropped seat, unscoped run)
become named, durable, and graded. Approve this and the ledger carries D30–D36
into L7 (workspace), L8 (control plane), and lane 2 (the org), all of which
consume the same principal-unified edge.

**Delivered 2026-11-12, cut v0.6.36.** All nine criteria (C0–C8) met and
tested; the four-leg gate green on `39fa329` (2234 lib + 172 integration +
16 install + 7 face harness, settle clean). The published feed reads back
0.6.36 with the cli row's sha256 matching the served raw binary
(`463eff41aacb…`); the installed CLI answers `fleet 0.6.36`. The SSE-test
failure that blocked the cut was a resource-admission *queue* on a
disk-constrained host (the run was refused, never spawned), fixed by pinning
the test's reserve — the `write_concurrent_roster` lever; the probe's
paused-retry budget was widened for the gate's parallel load (KI-296/KI-405).