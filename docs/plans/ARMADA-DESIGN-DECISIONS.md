# Armada — the design-decisions ledger

**The awareness backbone.** Armada is planned bottom-up, one layer at a
time. Every layer's plan **reads the choices settled below it** (citing
their `D` ids) and **appends its own** when drafted. A plan that contradicts
a settled choice is wrong by construction — this ledger is what keeps the
stack from diverging. It is the shared context the plan-backlog workflow
warns is missing when plans are drafted in fresh isolation.

This ledger records the **choices**. Its two siblings complete the picture:
[`ARMADA-DECOMPOSITION.md`](ARMADA-DECOMPOSITION.md) records the **atoms** (the
build map — what pieces, in what order), and
[`ARMADA-CONTRACTS.md`](ARMADA-CONTRACTS.md) records the **interfaces** (what
each module offers / expects). Decomposition = *the pieces*; choices = *what
and why*; contracts = *the exact seam*. Read all three before building.

Rules:
- `D` ids are **stable and never reused or reordered**; upper layers cite
  them. A choice is superseded only by a new `D` that names the old one and
  why (the corpus's own supersede discipline, applied to design).
- A choice is **settled** when its layer's plan is approved. Until then it
  is *proposed* — upper-layer drafts conform to it, and if the owner changes
  it, the ledger updates and the upper plans re-conform.
- Each entry: the decision, the rationale, and **what it constrains above**.
- The decomposition (`../architecture/2026-11-11-armada-atomic-decomposition.md`)
  defines the contracts; this ledger records the **concrete choices** that
  realize them (representations, file homes, invariants).

## Shared choices (settled in the decomposition)

- **D0 — InstallId is a stable per-install identity, minted at install,
  persisted in the home.** Distinct from any device or session. *Constrains:*
  the corpus cursor/provenance (L4), the roster key (L8), deployment (L9).
  *Representation settled by L1.*
- **D1 — PrincipalId is the unit of authority, distinct from the device.**
  Lane 1 has exactly one principal, `owner`; every contract is
  principal-agnostic so lane 2 multiplies principals without change.
  *Constrains:* the guard (L3), the workspace assignee (L7), the identity
  graph (L5), the control frame (L8), the org (lane 2). *Representation
  settled by L1.*
- **D2 — DeviceId maps to a PrincipalId, never the reverse.** A paired
  client carries a principal's authority; the device is the channel, not the
  authority. *Constrains:* pairing (L1), the identity graph (L5).
- **D3 — Every contract carries the responsible-citizen shape:**
  `measure → admit/refuse → work → settle → grade → learn → curtail`, with
  named refusal and named degradation, never silent failure. *Constrains:*
  every layer's contract (L1–L9).
- **D4 — Trust is earned by measurements.** Nothing is trusted by assertion:
  a decision is graded against independent evidence before it changes
  behavior; an estimate learns from its settles; a fact climbs the basis
  ladder only on evidence; autonomy is earned, never assumed. *Constrains:*
  every layer's grading/learning loop.
- **D5 — The contracts stack:** L1 identity → L2 journal → L3 security →
  {L4 corpus, L5 communication, L6 relay, L7 workspace} → L8 control plane →
  L9 deployment. Each layer discharges responsibility *through* the contracts
  below; the upper layers inherit the discipline, they do not re-state it.
  *Constrains:* the dependency order; no layer re-implements a lower
  guarantee.
- **D6 — Without breaking fleet:** the owner principal holds every grant by
  default (default-deny bootstraps to "owner allowed everything"); calib
  stays advisory (the journal is a new durable home, not a calib promotion);
  JSONL before SQLite (no new foreign dependency); the relay broker becomes
  Rust. *Constrains:* every atom's bootstrap and the structure guards it
  meets.

## The cross-cutting obligation: trust is earned by measurements (D4, made verifiable)

This is not a slogan — it is a **per-layer acceptance obligation**. Every
layer's plan must name *what it measures*, *how trust is earned from that
measurement* (the grade/learn loop), and a *verifiable property* that proves
it. A layer that trusts by assertion — a delivery assumed, a peer trusted
because it spoke, an instance dispatched because it registered, a fact
believed because it arrived — fails this obligation. The bedrock layers
(L1, L2) *provide the substrate* measurement rides; the decision layers
*earn trust* through it; the control plane *withholds autonomy* until
measurement earns it.

| Layer | What it measures | How trust is earned (the loop) | Verifiable property |
|---|---|---|---|
| L1 identity | pairing events | a device→principal link exists only on a measured pairing, settled to the journal | no link without a pairing event |
| L2 journal | event volume, fsync cadence | the retention budget learns from its settles; durability is a measured, declared choice | budget curtails named; cadence declared |
| L3 security | guard decisions, assertion freshness | authority granted only on fresh measured proof (2FA); an escalation attempt is a measured security signal | destructive without fresh proof → `StepUpRequired` |
| L4 corpus | sync outcomes, fact basis | the merge graded by read-back; a fact climbs the basis ladder only on evidence; the estimate learns from the last ten settles | sync reports done only after the read-back grade |
| L5 communication | dispatch outcomes, pairing, outbox depth/lag | a peer's authority earned by measured pairing; dispatch graded against finished/failed; the retry backoff learns | unlinked peer → named refusal; every dispatch graded |
| L6 relay | frame delivery (ack), queue lag, liveness | a frame is trusted delivered only when acked; a gap is named, not silent; liveness earned by heartbeats, not assumed | an unacked frame → named gap + re-request |
| L7 workspace | bot work outcomes | a task is `done` only when the deliverable passes the end gate; a bot's work graded against its outcome | `done` only on a passed deliverable, else `blocked` named |
| L8 control plane | instance reliability, dispatch outcomes | an instance earns dispatch autonomy by measured reliability; roster liveness by heartbeats; a dispatch graded before the pattern changes | a fresh instance is un-calibrated until it settles runs |
| L9 deployment | install doctor + first settles | a fresh install is trusted only after it passes doctor and settles a few runs | deploy graded by doctor + settles, not by arrival |

Every plan below carries a criterion discharging its row. The grading loop
is the contract, not an add-on (decomposition §1.1: `grade` before behavior
changes, `learn` from the settles).

## L1 — Identity

*Settled by `2026-11-11-armada-l1-identity.md` (built, shipped v0.6.31).*

- **D7 — InstallId = 32 hex (16 random bytes, `rand`), minted once at
  install, persisted `<home>/install-id`, read by `identity::install()`.**
  Rationale: the relay-sid shape, no new dependency; stable across restarts,
  a fresh home is a fresh instance. *Constrains:* L4 cursor/provenance, L8
  roster key, L9 deployment.
- **D8 — PrincipalId = a lowercase slug; lane 1 = `owner`; the registry +
  device map live in `<home>/identity.json`; `src/identity.rs` is the single
  home (`principal_for`, `device_principal`).** Rationale: one store, one
  module; principal-agnostic so lane 2 adds rows not code. *Constrains:* L3
  guard, L5 identity graph, L7 assignee, L8 frame, lane 2.
- **D9 — the principal rides `AgentOptions.principal` (default `owner`); a
  child inherits its parent's; distinct from the SOUL `identity`; the guard
  reads it.** Rationale: authority travels with the run, never the actor;
  the confused-deputy boundary (R3) starts here. *Constrains:* L3 guard
  input, L7 bot deputy, L8 frame principal.

## L2 — The event journal

*Proposed by `2026-11-11-armada-l2-journal.md` (draft).*

- **D10 — the journal is day-partitioned append-only JSONL at
  `<home>/journal/YYYY/MM/DD.jsonl`; `Event {seq, ts, kind, install,
  principal?, session?, payload}`; a global monotonic `Seq` (u64) persisted
  at `<home>/journal/seq`.** Rationale: total order across days, the L1
  identity on every event, no new dependency. *Constrains:* L3 audit, L5
  bus, L7 events, L8 tracing.
- **D11 — durability = O_APPEND + flush per record, fsync at a declared
  cadence; a retention budget curtails old day-files by age, named.**
  Rationale: a responsible citizen (D3) — bounded, never a silent drop
  (unlike calib). *Constrains:* every producer.
- **D12 — durable named consumers persist a cursor at
  `<home>/journal/consumers/<name>.cursor`; `subscribe(consumer, from)`
  replays from it.** Rationale: a lagged/restarted consumer never loses an
  event. *Constrains:* L5 consumers, L8 federation.
- **D13 — `record()` hooks the journal additively (append + broadcast);
  calib and the session log are untouched.** Rationale: non-breaking (D6);
  the journal is a new home, not a promotion. *Constrains:* L5 (SSE becomes
  a projection later).

## L3 — Security / authority

*Proposed by `2026-11-11-armada-l3-security.md` (draft).*

- **D14 — `Grant = (principal, dimension, scope, action-class)`; the closed
  enums (Dimension/ActionClass/Scope); the store at `<home>/grants.json`;
  default-deny; the folder grant is the first row.** Rationale: one schema,
  eight dimensions, not eight permission systems (R4). *Constrains:* L7
  invocation, L8 control verbs, lane 2 roles.
- **D15 — the uniform `guard.check` at `prepare_call`; lane-1 owner wildcard
  bootstrap (owner holds all).** Rationale: one choke point; non-breaking
  (D6). *Constrains:* every action; L8 verifies the frame's principal here.
- **D16 — per-principal declaration filter at `declaration_set`; an
  ungranted tool is not declared.** Rationale: R3 — the agent's tools are
  scoped to the principal. *Constrains:* L7 bot scope, L8 dispatch, lane 2.
- **D17 — every guard decision settles a `guard` journal event (principal,
  dimension, scope, class).** Rationale: durable per-principal audit (D4,
  §7.6), not lossy calib. *Constrains:* L8 federation, lane 2 audit.
- **D18 — `Assertion {principal, class, scope, target, issued_at, nonce,
  signature}`, Ed25519 via `ring`; flagship private / instance public;
  step-up accepts in-turn approval (lane 1) or a valid assertion (org);
  custody + issuance at L8.** Rationale: the agent cannot forge (R5);
  non-breaking step-up. *Constrains:* L8 authority transport, lane 2 factors.
- **D19 — the governance layer (grant store, guard config, roles, org SOUL)
  is immutable to the agent; grant-making is the root privilege, refused by
  name and audited.** Rationale: R3 — the boundary is actor-blind, the
  governance layer owned by the control plane. *Constrains:* L8, lane 2.

## L4 — The shared corpus

*Proposed by `2026-11-11-armada-l4-corpus.md` (draft).*

- **D23 — `corpus.sync(remote)` = fetch → `corpus.merge(rules)` → push; the
  remote + install-scoped cursor at `<home>/corpus-sync.json`; git is
  transport only, the semantic merge is fleet's; no remote = named refusal,
  fleet stays local.** Rationale: the substrate is git-tracked but the daily
  script never pushes — the remote is the missing half; opt-in (D6).
  *Constrains:* L8 corpus-sync verb, L9 inheritance, the shared mind.
- **D24 — concurrent recall-count bumps merge by sum/delta over the merge
  base (`ours + theirs − base`), never last-writer-wins; the count stays in
  frontmatter.** Rationale: a grow-only rank signal — a fact recalled on A
  bumps its rank for B. *Constrains:* the merge driver, future counters.
- **D25 — auto-generated replacement/archive serials carry the install-id's
  first 8 hex, so two installs never mint the same name (the distributed
  KI-285 fix).** Rationale: the free-serial scan is a local decision two
  installs make identically → add/add; the writer's identity (D7)
  disambiguates. *Constrains:* supersede/import naming, L9 inheritance.
- **D26 — two installs superseding one fact resolve to a single live
  frontier: higher basis wins (confirmed-by > validated > inferred), tie →
  later date, tie → greater install-id; the loser folds with a named
  retirement note, read-back verified, never hard-deleted; an unresolvable
  conflict is a named refusal that curtails the sync.** Rationale: the basis
  ladder (D4) is the authority-free winner rule for lane 1. *Constrains:*
  lane 2 adds the authority dimension; the merge driver.
- **D27 — `RemoteInstance` implements `Source` by reading the remote corpus
  at a git commit (`revision = git:<commit>`), feeding the unchanged
  `pull()`; the `NotYetBuilt` stub retires; provenance
  `armada:<project>:<commit>`.** Rationale: `pull()` is already
  source-agnostic. *Constrains:* L8 corpus-sync, L9 register_join.
- **D28 — every sync settles a `corpus_sync` journal event (install,
  principal, remote, before/after revision, merged/summed/resolved/refused
  counts) and the merged tree is graded by read-back before done; the
  estimate learns from the last ten settles.** Rationale: D3 + D4 — graded,
  not just recorded. *Constrains:* L8 tracing, the corpus-sync outcome.
- **D29 — the recall index, lock siblings, temps, and per-install state
  (cursors, install-id, identity, estimates) never sync — only the tracked
  fact tree moves; a fresh install clones the remote with a fresh install-id
  (identity is not in the corpus).** Rationale: the index is a local cache;
  the portability ruling. *Constrains:* L9 provision, the org corpus.

## L5 — Communication

*Proposed by `2026-11-11-armada-l5-communication.md` (draft).*

- **D30 — the durable outbox: `reply`/`present_seat` return a named Result;
  every outbound appends to the journal (`kind="outbox"`) before the attempt,
  attempted inline, settled `delivered` on success; a failure requeues to a
  durable `outbox-worker` consumer (D12) with a backoff that learns from
  settles, curtailing to a named `undeliverable` after a bounded budget;
  at-least-once (may duplicate, never lose).** Rationale: today
  `let _ = ureq::post` loses a failed reply (a silent failure, D3).
  *Constrains:* L6 relay sends, L8 replies, lane 2 chat/mail.
- **D31 — the identity graph: `identity.resolve(peer) -> PrincipalId`
  resolves a Peer/Device to one principal via edges in `<home>/identity.json`
  (extending L1's store, D8 — one home); lane 1 links every peer/device to
  `owner` by explicit edge.** Rationale: the principal is the unit at the
  comm edge (D1/D2). *Constrains:* L5 dispatch, L6 relay channel, L8 frame.
- **D32 — resolve is total-or-named-refusal: an unlinked peer is
  `Refusal::Unknown`, never a silent fallback to `owner` (a fallback is a
  confused-deputy escalation); the peer→principal edges are governance
  (D19), linked only at pairing.** Rationale: trust earned by measured
  pairing (D4). *Constrains:* L5 admission, L8 pairing, lane 2.
- **D33 — the bus IS the journal: `bus.publish` = `journal.append`,
  `bus.subscribe` = `journal.subscribe` (D12), no second broker;
  `Driver::watch` becomes the durable `channel-driver` consumer (the
  `Lagged → continue` drop retires); channel inbound cursors (telegram
  offset) persist.** Rationale: a lagged/restarted consumer never loses an
  event (D3/D12). *Constrains:* L7 events, L8 tracing.
- **D34 — gated dispatch threads the principal: `Driver::deliver` resolves
  peer→principal (D31), checks the edge capability (Task/Observe, retained),
  then runs under that principal via `AgentOptions.principal` (D9) so the
  guard (D15) sees it — the peer no longer evaporates at `bridge.run`.**
  Rationale: non-breaking (D6) — the edge capability composes with, not
  replaces, the principal guard. *Constrains:* L7 invocation, L8 dispatch.
- **D35 — the edge settles + grades every dispatch: a `dispatch` journal
  event (channel, peer, principal, session, admitted/refused, outcome),
  graded against the run's finished/failed settle (D4); outbox depth +
  consumer lag measured, breach curtails named.** Rationale: graded, not
  just recorded (D4). *Constrains:* L8 dispatch grading, lane 2 audit.
- **D36 — one principal at the omnichannel edge; the relay is a Channel impl
  consuming L6 push (D21) + control frames (D22); a relay frame's device
  resolves through the same identity graph (D31) to the same principal as a
  channel message.** Rationale: instances converse; the edge is
  principal-unified by construction. *Constrains:* L6 relay channel, L8,
  lane 2 chat/mail.

## L6 — The relay transport

*Proposed by `2026-11-11-armada-l6-relay.md` (draft).*

- **D20 — the broker is Rust, a mode of the fleet binary (`fleet relay
  --broker`), dumb, routing sealed frames by sid; `relay/relay.py` retires.**
  Rationale: self-containment (hard rule 12); the dumb-broker invariant
  (§8.5). *Constrains:* L8 control lane.
- **D21 — push with ack + cursor: unsolicited frames queued per sid,
  delivered + acked; a gap is detectable and re-requestable.** Rationale:
  the phone gets a live stream, not polling; a missed poll loses nothing.
  *Constrains:* L8 flagship→instance push, L5 relay channel.
- **D22 — the frame gains `kind:"control"` carrying `{verb, install,
  principal, assertion?, payload, nonce, issued_at}`, sealed by the existing
  crypto; the broker parses no verb.** Rationale: authority rides in the
  frame (§8.4), the broker stays dumb. *Constrains:* L8 vocabulary.

## L7 — The workspace

*Proposed by `2026-11-11-armada-l7-workspace.md` (draft).*

- **D37 — the workspace is `src/workspace.rs`, an object-keyed graph:
  `WorkObject {id, kind}` (task|thread|deliverable|bot|lesson); only `task`
  is store-owned — the rest are typed references derived on read from the
  session/desk/memory/roles stores (no parallel store, KI-300); tasks +
  edges persist as append-only JSONL at `<home>/workspace/graph.jsonl`
  folded on read, exempt from journal retention; NOT `graph.rs`.**
  Rationale: object-keyed not folder-keyed; reuse not rebuild; durable work
  objects cannot live in the age-curtailed journal (D11). *Constrains:*
  L7.2–L7.4, L8 reads the graph, lane 2.
- **D38 — `Task {id, status: working|blocked|done, assignee: PrincipalId}`:
  a stable cross-session id, assignee = the L1 principal (D8); the `todo`
  renderer + per-session `Tasks` record are unchanged — the workspace task
  is the additive durable unit.** Rationale: `todo` has no id/assignee/
  blocked. *Constrains:* L7.4 assignment, L8 dispatch, lane 2.
- **D39 — `Edge {from, to, kind}` with a closed kind vocabulary;
  `graph.link` refuses a dangling endpoint/unknown kind by name;
  `hand_to_desk` fixed to attach the room's thread + bind the room's
  workspace (not the server cwd), recording the thread→deliverable edge.**
  Rationale: the missing reverse edges; `desk.session` stays None today.
  *Constrains:* L7.3 traversal, L7.4 linked deliverable.
- **D40 — surfaces are views: `projects()` becomes a projection of
  `graph.view(filter)`, grouped by the workspace binding derived from each
  object's store; the folder-keyed output shape is preserved so the face is
  unchanged.** Rationale: object-keyed is the contract (§2.3). *Constrains:*
  L9 surfaces, lane 2.
- **D41 — bot invocation is a workspace action: `task.assign(task,
  principal) -> Run` reuses `bridge.run_as` under the role overlay and
  `huddle` for multi-bot; a huddle STAYS a session (KI-300); the bot is the
  deputy under the user's principal (D9).** Rationale: reuse roles + huddle
  + bridge.run, never rebuild. *Constrains:* L8 dispatch, lane 2.
- **D42 — invocation is gated: `task.assign` admits through `guard.check`
  (D15) under the assignee principal; the bot's tools are the principal's
  grants narrowed by the role overlay (D16); a destructive bot task hits
  step-up (D18); a refusal is named + settles a `guard` event (D17) — the
  workspace does not re-implement the guard.** Rationale: contracts stack
  (D5). *Constrains:* L8, lane 2.
- **D43 — settlement + grading: every work-object mutation settles a `work`
  journal event (D10); `task.assign` settles the run's outcome to the task —
  `done` only when the deliverable passes the desk end gate, else `blocked`
  named; a bot's work graded against its outcome, never trusted by
  assertion (D4).** Rationale: graded not just recorded; named refusal
  (D3). *Constrains:* L8 tracing, lane 2.

## L8 — The control plane

*Proposed by `2026-11-11-armada-l8-control-plane.md` (draft).*

- **D44 — registration + roster + heartbeat: an instance registers on join
  (install-id D7, sid, version, capacity); the flagship roster at
  `<home>/roster.json`; liveness earned by heartbeats, not assumed.**
  Rationale: the roster makes "control individual instances" concrete;
  liveness is measured (D4). *Constrains:* L9 deployment, lane 2.
- **D45 — the authority transport: every control frame carries the principal
  (D8) + (destructive/governance) a fresh assertion (D18) signed by the
  flagship Ed25519 key; the instance verifies before obeying, then its own
  guard (D15); the relay cannot fabricate, an instance cannot escalate, an
  agent cannot borrow.** Rationale: §8.4 — authority rides in the frame.
  *Constrains:* lane 2 (many principals).
- **D46 — the control vocabulary reuses the bridge verbs: dispatch→bridge.run,
  state→bridge.state, corpus-sync→L4 (D23), version-roll/retire (destructive),
  health (read), register/heartbeat (governance); each through the guard +
  the instance's admission.** Rationale: an armada-dispatched run is just
  another guarded, admitted run. *Constrains:* L9 surfaces, lane 2.
- **D47 — the flagship is a fleet install in a flagship role (role + config,
  one binary); the signing key lives at the flagship, never on an instance
  the agent can reach.** Rationale: flagship-is-fleet (§3); custody is
  load-bearing. *Constrains:* L9, the custody/rotation fork.
- **D48 — the human second factor (TOTP or push, factor-agnostic) issues the
  fresh per-action assertion the agent cannot forge; L3.7 verifies.**
  Rationale: R5 — the agent can never satisfy 2FA. *Constrains:* lane 2
  per-user factors; the second-factor fork.
- **D49 — instance autonomy earned by measurement: a fresh instance is
  un-calibrated until it settles runs; a scheduled reliability beat grades
  dispatch outcomes over the journal before the pattern changes.**
  Rationale: the cross-cutting obligation (D4); the flagship cannot trust a
  fresh install by arrival. *Constrains:* lane 2 per-user quota.
- **D50 — relay-down is a named degradation: instances run autonomously, the
  corpus syncs over git, live control resumes on return.** Rationale: the
  failsafe ruling at armada scale (§8.6). *Constrains:* degradation behavior.

## L9 — Deployment + surfaces

*Proposed by `2026-11-11-armada-l9-deployment.md` (draft).*

- **D51 — provision→register→join: a local spawn lays out a new home; a VPS
  is reached over SSH/cloud and dropped the self-contained binary (the
  release raw-binary channel); the target runs install, mints a fresh
  install-id (D7), clones the corpus remote (D29), registers (D44);
  credentials out-of-band.** Rationale: fleet deploying fleet, self-contained
  (hard rule 12). *Constrains:* lane 2 provisioning.
- **D52 — fresh-install corpus inheritance: clone the remote with a fresh
  install-id (identity not in the corpus, D29); un-calibrated until it
  settles (D49).** Rationale: a fresh instance thinks from the fleet's
  memory. *Constrains:* the shared mind's distribution.
- **D53 — the surface progression (§9): CLI/TUI → face/PWA → Tauri over the
  one `src/armada` library; one implementation, four surfaces; UX co-equal
  (hard rule 11).** Rationale: fleet's proven arc reused. *Constrains:* lane 2
  team surfaces.
- **D54 — the TUI armada board: instances, health, capacity, dispatch (a
  ratatui pane).** *Constrains:* the terminal-first surface.
- **D55 — the face armada section: roster board, dispatch composer, corpus
  view; centre-conversation + context-pane; the chat composer borrowed never
  rebuilt.** *Constrains:* the PWA surface, lane 2.
- **D56 — fleet-desktop wraps the armada face over the same server/bridge (no
  second frontend, no second transport).** *Constrains:* the native shell.
- **D57 — deployment graded by measurement: provision is a governance action
  (step-up); a fresh install trusted only after doctor + settles.**
  Rationale: the cross-cutting obligation (D4) at the edge. *Constrains:* the
  trust boundary.