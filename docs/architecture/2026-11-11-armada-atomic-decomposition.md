# Fitting fleet for armada — the contract-first atomic decomposition

**Status:** direction / decomposition, not a plan. Taken 2026-11-11 against
fleet 0.6.30. The armada direction note
(`2026-11-11-armada-direction.md`) is a **vision**; this breaks it into
**atomic units we build bottom-up**, each layer added to fleet exposing
**well-defined contracts that armada consumes**. No acceptance criteria
here — each atom becomes its own plan in `docs/plans/` when the owner says
build. The current-state evidence (what fleet has today, with file:line)
is in `2026-11-11-armada-bottom-up-survey.md`.

## 0. The discipline

The owner's rule, by example: *the security layer we add to fleet must
support the armada vision and provide well-defined contracts armada can
use.* Generalized:

1. **Fleet is fitted, not forked.** Every layer is added to fleet (one
   binary, one workspace, flagship-is-fleet). Armada is the upper layer
   that consumes the contracts; it is not a separate runtime.
2. **Contract-first.** Before building a layer's atoms, define the contract
   it exposes — the exact types and calls armada will use. The contract is
   the anchor that keeps a bottom-up build aligned with the vision instead
   of drifting into a retrofit.
3. **Bottom-up, atomic.** Build the smallest coherent unit that satisfies a
   contract, in dependency order. An atom is one buildable, verifiable
   increment with its own surface (hard rule 11).
4. **One principal now, many later.** Every contract is principal-agnostic
   from the first atom. Lane 1 runs with one principal (the owner); the org
   multiplies principals and leaves the contracts unchanged.
5. **Without breaking fleet.** Each atom preserves single-owner behavior.
   The owner principal holds every grant by default; default-deny
   bootstraps to "owner allowed everything." Nothing the owner does today
   starts being refused.
6. **Every contract is a responsible citizen.** The same discipline fleet
   applies to its own work — measure → admit/refuse → work → settle/curtail
   → learn, never more than is available, a named degradation never a silent
   failure (the resource contract, hard rule 12) — is baked into every
   contract's *shape*, not left to the caller. A contract admits bounded
   work, settles what it spent, curtails on breach, names every refusal and
   degradation, and learns its estimates from its settles. The upper layers
   inherit responsible behavior by construction, not by convention.
7. **Contracts stack.** Each layer's contract is composed from the contracts
   below it and discharges its responsibility *through* them — the guard
   settles its audit through the journal, the control plane admits dispatch
   through the instance's capacity contract. Responsibility composes up the
   stack; no layer re-implements the discipline the layer below already
   guarantees.
8. **Trust is earned by measurements.** Fleet's epistemology infuses every
   layer: nothing is trusted by assertion. A grant, an estimate, a decision,
   a fact, an instance's autonomy is trusted only to the degree measured,
   graded evidence has earned it. A decision is graded against independent
   evidence *before* it may change behavior (the calibration gate); an
   estimate learns from its settles (the median of the last ten); a fact
   climbs the basis ladder (`inferred → validated → confirmed-by`) only on
   evidence; autonomy is earned, never assumed. The grading loop is part of
   every contract, not an optional add-on — "graded, not just recorded."

**What is already done (the only completed prerequisite):** the
cross-harness memory pull (`src/memory_pull.rs`, the `Source` trait +
`LocalHarness` + `RemoteInstance` stub) and local git sync of the corpus;
and capacity (the resource contract + multi-session admission). Everything
below is new design.

## 1. Shared foundations — vocabulary, the contract shape, the stack

### 1.0 The vocabulary (the types every contract speaks)

These are defined once, in the bedrock, and referenced by every layer:

```text
InstallId    — stable identity of one fleet install; minted at install,
               persisted in the home. Today: nothing (must be minted).
PrincipalId  — the unit of authority an action acts for. Lane 1: exactly
               one, `owner`. Lane 2: many. Distinct from the device.
DeviceId     — a paired client (phone/tablet). Exists today (Device.id);
               maps to a PrincipalId, never the other way.
Seq          — a monotonic u64, the journal's total order.
Dimension    — project | storage | resource | corpus | tool | skill |
               workflow | loop | governance        (the closed set, R4)
ActionClass  — read | write | dispatch | destructive | governance
Scope        — instance | project:<id> | team:<id> | org
               (lane 1 uses instance + project; team/org are data, not code)
```

### 1.1 The responsible-citizen contract — the shape every layer carries

Every layer's contract carries this discipline *alongside* its domain calls
(the `guard.check`, `journal.append`, `corpus.sync` of §3–§4). It is the
resource contract's `admit → work → settle → curtail → learn`, generalized
from disk to every layer, with the calibration gate and the earning of
trust made explicit (disciplines 6 and 8):

```text
measure() -> Facts                        // measured, never assumed
admit(estimate) -> Admit | Refuse { name, remedy }   // bounded by the facts
work(...) -> Outcome                      // the domain call
settle(actual) -> ()                      // account what was spent
grade(outcome, evidence) -> Verdict       // independent judgment, BEFORE
                                          //   behavior may change
learn() -> estimate'                      // trust earned: next estimate =
                                          //   f(the last settles)
curtail() -> NamedDegradation             // on breach; named, never silent
```

A contract that asserts trust without measurement, refuses silently, or
spends without settling is **not a fleet contract**. The domain call is the
visible half; this discipline is the load-bearing half. Concretely, per
layer:

- **Security** — authority is *earned by measured proof*: a destructive act
  needs a fresh 2FA assertion (a measurement of live human authority), every
  guard decision is settled to the audit and graded; an agent's reach is
  bounded by granted, measured capability, never assumed.
- **Capacity** — the estimate is the median of the last ten settles; a
  budget is trusted only as far as its track record earns; a breach
  curtails and names itself.
- **Corpus** — a fact climbs `inferred → validated → confirmed-by` only on
  evidence (the dream pass measures it against its source); recall rank is
  earned by measured recall, not assertion.
- **Control plane** — an instance earns dispatch autonomy by measured
  reliable behavior; a dispatch is graded against its outcome before the
  pattern changes; relay-down is a named degradation, never a silent stop.

### 1.2 The contract stack

The layers stack; each contract is composed from the contracts below and
discharges its responsibility *through* them (discipline 7):

```text
   L8  control plane     ─┐  the hand
   L7  workspace          │
   L5  communication      │  each layer composes the responsible
   L4  corpus             │  contracts below it; admission, grading,
   L6  relay transport    │  settlement, and earned trust propagate
   L3  security           │  up the stack by construction
   L2  journal            │
   L1  identity          ─┘  the bedrock
```

The control plane does **not** re-implement admission, grading, or
settlement: a dispatch admits through the instance's capacity contract, is
authorized through the security guard (which settles its audit through the
journal), rides the relay's sealed-frame contract, syncs the corpus within
its budget, and journals the outcome. Because every layer is a responsible
citizen that earns trust by measurement, the stack is trustworthy end-to-end
**by construction** — the upper layers inherit the discipline, they do not
re-state it. This is what "fleet fitted for armada" means: not a control
plane bolted onto an agent, but a stack of responsible, measured contracts
the control plane stands on.

## 2. The upper-layer contracts — what armada consumes (the anchor)

These are the contracts the **upper layers** (armada's control plane, the
org, the workspace) require. They are defined first so every fleet layer
below is built to satisfy them. They are the "why" of the bottom-up build.

### 2.1 The control plane (armada's hand, §8)

```text
roster.register(install: InstallId, sid, version, capacity) -> ()
roster.heartbeat(install) -> ()
roster.list() -> [Instance { install, sid, version, capacity, live }]
control.dispatch(install, task, principal: PrincipalId, assertion?) -> Run
control.state(install) -> State
control.corpus_sync(install) -> ()
control.version_roll(install, version) -> ()   // destructive
control.retire(install) -> ()                  // destructive
```
Consumes: Identity (install + principal), Security (every control verb is
guarded; destructive verbs need a fresh assertion), Relay (the sealed
control frame), Corpus (corpus-sync), Capacity (the instance admits for
itself). **Invariant:** a control frame carries `{verb, install, principal,
assertion?, payload, nonce, issued_at}`, signed by the flagship; the
instance verifies the signature *before* it obeys, then runs the verb
through its own guard. The relay cannot fabricate; an instance cannot
escalate; an agent cannot borrow beyond its principal.

### 2.2 The org (lane 2, §7.7) — a generalization, not a new contract

```text
users.create(principal, factors) -> ()
roles.assign(principal, role) -> ()   // a role is a named bundle of grants
grants.team_org_scope(principal, team|org, ...) -> ()
```
Consumes the **same** Security contract (§3.3) with more principals: a role
stamps its grants onto a principal; the guard, assertion, and audit are
unchanged. This is why the security contract must be principal-agnostic
from atom one.

### 2.3 The workspace (§12.5)

```text
graph.objects(filter) -> [WorkObject]
graph.link(from, to, kind) -> ()
task.assign(task, principal: PrincipalId) -> Run   // bot invocation
```
Consumes: Identity (assignee = principal), Security (invocation is
granted; a destructive bot task hits step-up), Journal (work-object
events), Corpus (docs write back with provenance).

## 3. The fleet layers — contract exposed + atoms

Each layer below is added to fleet. The **contract** is what it exposes for
the upper layers; the **atoms** build it bottom-up. Atom ids are stable
(`L<layer>.<n>`); dependencies name other atoms. Every contract carries the
responsible-citizen shape of §1.1 (measure / admit / settle / grade / learn /
curtail) alongside its domain calls — only the domain calls are shown below,
to keep each layer readable; the discipline is implicit in every one.

### 3.1 Layer L1 — Identity (bedrock)

**Contract exposed**
```text
identity.install() -> InstallId                 // stable, minted once
identity.principal_for(action) -> PrincipalId   // who the action acts for
identity.device_principal(device: DeviceId) -> PrincipalId
```
**Armada consumes it** everywhere: control frames carry the principal; the
roster keys by install-id; the workspace assignee is a principal; the
communication identity graph resolves to a principal; the org user store
maps devices to principals.

**Atoms**
- **L1.1 — mint the install-id.** A stable id created at `fleet install`,
  persisted in the home, read by everything. *Raw material:* home layout
  (`src/install.rs`), no id today. *Surface:* `fleet identity` / settings
  identity slot. *Deps:* none.
- **L1.2 — the PrincipalId type + owner bootstrap.** One principal,
  `owner`, resolved for every action. *Deps:* none.
- **L1.3 — per-run principal propagation.** Thread the principal through
  `AgentOptions` and the `Approver` trait; a delegated child inherits its
  parent's principal (never its own). *Raw material:* `src/agent.rs:492`,
  `src/subagents.rs:2305-2308`. *Deps:* L1.2.
- **L1.4 — device→principal map.** Pairing resolves a device to a
  principal. *Raw material:* `Device.id` (`src/remote/pairing.rs:308-324`).
  *Deps:* L1.2.

### 3.2 Layer L2 — The event journal (bedrock)

**Contract exposed**
```text
journal.append(event: Event) -> Seq             // total-ordered, durable
journal.subscribe(consumer, from: Seq) -> Stream<Event>  // durable cursor
journal.replay(from: Seq) -> [Event]
Event { seq, ts, kind, install: InstallId, principal?: PrincipalId,
        session?, payload }
```
**Armada consumes it:** the communication bus rides it; the security audit
lands in it; workspace events ride it; the control plane traces causation
across installs (a dispatch on A → a run on B → a corpus commit on C); the
journal federates across the roster.

**Atoms**
- **L2.1 — the Seq + Event type + sequencer.** A monotonic counter and the
  typed event. *Deps:* L1.1 (install id on the event).
- **L2.2 — durable append.** JSONL + the sequencer; the fsync decision made
  explicitly (flush ≠ fsync today). SQLite is a deliberate dependency
  decision fleet has not made — JSONL first. *Deps:* L2.1.
- **L2.3 — durable named consumer + cursor.** A consumer group with a
  persisted offset; replay from a cursor. *Deps:* L2.2.
- **L2.4 — the journal behind `record()`.** The face event stream becomes a
  projection of the journal; seat/control/queued-run events (lost on crash
  today) are journaled. *Raw material:* `src/gui/bridge.rs:12340`.
  *Non-breaking:* add a new journal; do **not** promote calib (stays
  advisory) and do **not** merge the session log. *Deps:* L2.2, L2.3.

### 3.3 Layer L3 — Security / authority (the owner's example)

**Contract exposed**
```text
Grant = (principal: PrincipalId, dimension: Dimension, scope: Scope,
         class: ActionClass)
guard.check(principal, dimension, scope, class) -> Decision
Decision = Granted | Denied { name } | StepUpRequired
assertion.verify(token, principal, class, scope, target) -> bool
audit.record(decision) -> ()                    // rides the journal
```
`Assertion { principal, class, scope, target, issued_at, nonce, signature }`
— signed by the flagship key, fresh per action, **the agent holds no signing
key and cannot forge it**. The governance layer (grant store, guard config,
roles, org SOUL) is **immutable to every agent**; grant-making is the root
privilege, a `governance` action on the `governance` dimension.

**Armada consumes it:** the control plane's authority transport verifies the
frame's assertion at this guard before obeying (§2.1); the org assigns roles
as grant templates and issues per-user assertions (§2.2); the workspace
gates bot invocation; communication gates dispatch. **Default-deny** at
every layer; the agent inherits its principal's grants and escalates
nowhere; an ungranted tool is not even declared.

**Atoms**
- **L3.1 — the grant schema + store.** The `Grant` tuple, the `Dimension` /
  `ActionClass` / `Scope` enums, a grant store. The folder grant is the
  first row `(owner, project, project:<id>, write)`. *Raw material:*
  `sessions-workdirs.json` (`src/gui/bridge.rs:205`), the refusal shape
  (`src/agent.rs:5193-5202`). *Deps:* L1.2.
- **L3.2 — the uniform guard.** One `guard.check` at `prepare_call`
  (`src/agent.rs:5184`), default-deny, owner-holds-all bootstrap so nothing
  the owner does today is refused. *Deps:* L3.1, L1.3.
- **L3.3 — per-principal declaration filter.** Filter tool declarations by
  the principal's grants at `declaration_set` (`src/agent.rs:2443`) — an
  ungranted tool is not declared. *Deps:* L3.1, L1.3.
- **L3.4 — guard audit → journal.** Every grant/deny with principal,
  dimension, scope, class lands in the journal. *Raw material:* calib
  `ToolScreenRow` is actor-less and lossy — this is a new durable row, not a
  calib promotion. *Deps:* L3.2, L2.4.
- **L3.5 — the assertion format + verification.** The signed per-action
  token; verification at the guard; signing-key custody at the control
  plane. *Deps:* L3.2. (The human second factor itself lands with the
  control plane, L8; the *format* is fixed here.)
- **L3.6 — the governance boundary.** The `governance` class; the grant
  store / guard config / roles / org SOUL immutable to the agent; grant-
  making as the root privilege, refused by name and audited. *Deps:* L3.2,
  L3.4.
- **L3.7 — step-up 2FA.** The fresh human-out-of-band challenge (TOTP or
  push) for `destructive`/`governance`, even when the grant is held; the
  agent can never satisfy it. *Deps:* L3.5. (Second-factor choice is an
  open fork; the contract is factor-agnostic.)

### 3.4 Layer L4 — The shared corpus

**Contract exposed**
```text
Source { harness(), project(), read() -> Snapshot }   // EXISTS (the pull)
corpus.sync(remote) -> Result                          // pull + push
corpus.merge(rule) -> Result                           // conflict rules
Snapshot { revision, facts, skips }                    // EXISTS
```
**Armada consumes it:** the shared mind; the control plane's `corpus-sync`
verb; fresh-install inheritance; workspace docs over the corpus; the org's
multi-tenant corpus with attribution.

**Atoms**
- **L4.1 — corpus remote + sync driver.** A configured git remote; pull/push
  with the install-scoped cursor. *Raw material:* the pull's cursor sidecar
  (`src/memory_pull.rs:540`); local git tracking only today (no push, no
  merge code). *Deps:* L1.1.
- **L4.2 — the recall-count merge rule.** Concurrent increments merge by
  sum/delta, not last-writer-wins. *Raw material:* `bump_recall_count`
  (`src/memory.rs:3234-3244`), a local-flock counter. *Deps:* L4.1.
- **L4.3 — distributed supersede / serial-collision rule.** The KI-285
  two-writer shape, distributed: `-rN` replacement serials and archive
  serials must not collide across installs; supersede folds stay atomic.
  *Deps:* L4.1.
- **L4.4 — the RemoteInstance feeder.** Implement `Source` for a remote
  install over the same revision-cursor interface (a git commit). *Raw
  material:* the `RemoteInstance` stub (`src/memory_pull.rs:449-476`),
  `pull()` already source-agnostic. *Deps:* L4.1, L4.2, L4.3.

### 3.5 Layer L5 — Communication

**Contract exposed**
```text
Channel { name(), serve(sink), reply(to, text), present_seat(to, seat) }  // EXISTS
bus.publish(event) / bus.subscribe(consumer)         // durable, over L2
identity.resolve(peer) -> PrincipalId                // the identity graph
dispatch.gated(task, principal) -> Run               // guarded
outbox.send(reply) -> ()                             // durable, retried
```
**Armada consumes it:** instances converse (not just receive commands); the
omnichannel edge; the relay as a channel; the org's chat and mail. The
identity graph is the principal at the communication edge — a Slack message
and a relay frame resolve to one principal, so the guard sees one authority.

**Atoms**
- **L5.1 — the durable outbox.** Channel sends persisted + retried (today
  `let _ = ureq::post` loses a failed reply). *Raw material:*
  `src/channels/webhook.rs:140-143`. *Deps:* L2.2.
- **L5.2 — the identity graph.** Resolve `Peer`/`Device` across channels to
  one principal (today "two peers with one owner" by design). *Raw
  material:* `Peer` (`src/channels/mod.rs:73-93`). *Deps:* L1.4, L3.1.
- **L5.3 — the pub/sub bus over the journal.** Durable named consumers for
  channels and agents; `Driver::watch` becomes a journal consumer. *Raw
  material:* `src/channels/mod.rs:792`. *Deps:* L2.3.
- **L5.4 — gated dispatch.** Inbound dispatch runs through the guard under
  the resolved principal (today the peer evaporates at `bridge.run`). *Raw
  material:* `Policy::authorize` (`src/channels/mod.rs:295`),
  `mod.rs:929-931`. *Deps:* L5.2, L3.2.

### 3.6 Layer L6 — The relay transport

**Contract exposed**
```text
relay.send(frame, sid) -> Answer        // EXISTS, sealed (X25519+AES-GCM)
relay.push(frame, sid) -> ()            // NEW: unsolicited push
broker.route(frame)                     // Rust, dumb, by sid
Frame { kind, verb, install, principal, assertion?, payload, nonce, issued_at }
```
**Armada consumes it:** the control lane (§2.1) and the relay channel;
dial-out NAT traversal; relay-down degradation (instances run autonomously,
corpus still syncs over git).

**Atoms**
- **L6.1 — the Rust broker.** Replace `relay/relay.py` (Python, in-memory —
  a hard-rule-12 violation) with a self-contained Rust broker; keep the
  fleet-side Rust client. *Raw material:* `src/remote/relay.rs:126-280`
  (sealing), `relay/relay.py:149-162`. *Deps:* L1.1.
- **L6.2 — push / event frames + ack/cursor.** Unsolicited frames with
  broker-side queue + ack (today request→one-answer, device-pull only;
  `/events` degrades to polling). *Raw material:*
  `src/remote/listener.rs:597-612`. *Deps:* L6.1.
- **L6.3 — the control-frame discriminator.** A `kind: "control"` frame
  carrying the §2.1 vocabulary, sealed by the existing crypto. *Deps:*
  L6.2, L3.5 (the assertion it carries).

### 3.7 Layer L7 — The workspace (work-object graph)

**Contract exposed**
```text
WorkObject { id, kind }                 // task | thread | deliverable | bot | lesson
Edge { from, to, kind }
Task { id, status: working|blocked|done, assignee: PrincipalId }
graph.link(from, to, kind) / graph.view(filter)
task.assign(task, principal) -> Run     // bot invocation as a workspace action
```
**Armada consumes it:** the team workspace (§2.3); the org's tasks/docs/chat;
contextual bot invocation (assign a task to a persona → `bridge.run` under
the role overlay, work lands as a linked deliverable; @mention in-thread;
co-author a doc with provenance).

**Atoms**
- **L7.1 — the task object.** A first-class task with id, status, assignee
  (today `todo` is a renderer; no cross-session identity). *Raw material:*
  `src/session.rs:194-197`. *Deps:* L1.2.
- **L7.2 — the work-object edges.** task↔thread↔deliverable↔bot↔lesson;
  fix `hand_to_desk` to carry the thread (today `desk.session` stays None).
  *Raw material:* `src/gui/bridge.rs:782-806`, `src/desk.rs:202-203`.
  *Deps:* L7.1, L2.4.
- **L7.3 — surfaces as views.** The work surfaces read the graph (the
  folder-keyed `projects()` union becomes object-keyed). *Raw material:*
  `src/gui/bridge.rs:1586-1677`. *Deps:* L7.2.
- **L7.4 — bot invocation as a workspace action.** Assign/@mention/co-author
  reuse roles + huddle + `bridge.run`; a huddle stays a session (KI-300);
  invocation gated by the guard, the bot the deputy under the user's
  principal. *Deps:* L7.2, L3.2.

### 3.8 Layer L8 — The control plane (armada's hand)

**Contract exposed:** §2.1 (roster + control verbs). This layer **is**
armada's command plane; it consumes every layer below.

**Atoms**
- **L8.1 — registration + roster + heartbeat.** Instances register on join
  (id, sid, version, capacity); the flagship keeps the roster + liveness.
  *Deps:* L1.1, L6.3.
- **L8.2 — the authority transport.** Every control frame carries the
  principal + (for destructive/governance) a fresh assertion, signed by the
  flagship; the instance verifies before obeying, then runs its own guard.
  *Deps:* L3.5, L3.2, L6.3.
- **L8.3 — the control vocabulary.** dispatch/state/corpus-sync/version-roll/
  retire/health, each reusing the bridge verb and the instance's admission.
  *Deps:* L8.1, L8.2, L4.4.
- **L8.4 — the flagship role.** Instance vs flagship = role + config, one
  binary; the signing key lives at the flagship, never on an instance the
  agent can reach. *Deps:* L8.2.
- **L8.5 — the human second factor.** The out-of-band challenge that issues
  the assertion (TOTP or push), which L3.7 verifies. *Deps:* L3.7, L8.4.

### 3.9 Layer L9 — Deployment + surfaces

**Contract exposed**
```text
deploy.provision(target) -> InstallId   // local spawn | VPS drop
deploy.register_join(install) -> ()     // inherits corpus, joins roster
fleet armada { roster | dispatch | status | sync | register }   // CLI
```
**Atoms**
- **L9.1 — provision→register→join.** Drop the self-contained binary, run
  install, inherit the corpus, register. *Raw material:* `fleet install`
  (`src/install.rs`), the release raw-binary channel. *Deps:* L4.4, L8.1.
- **L9.2 — the CLI verbs.** `fleet armada …`. *Deps:* L8.3.
- **L9.3 — the TUI armada board.** Instances, health, capacity, dispatch.
  *Deps:* L8.3.
- **L9.4 — the face armada section.** Roster board, dispatch composer,
  corpus view. *Deps:* L8.3.
- **L9.5 — fleet-desktop.** The native shell wraps the armada face. *Deps:*
  L9.4.

### 3.10 Lane 2 — the org (a generalization, deferred)

Atoms (each consumes the unchanged contracts): user store + device→principal
mapping (L1); roles as grant templates (L3); per-user 2FA factors (L3);
team/org scopes (L3, already data); team-workspace surfaces tasks/docs/chat/
mail (L7 + L5). **None is a rewrite** — the guard, assertion, audit, and
governance boundary are unchanged. This is the payoff of contract-first.

## 4. The bottom-up wave order (dependency-forced)

```
Wave 0  done            memory pull + git sync; capacity
Wave 1  bedrock         L1 Identity ─┐  L2 Journal ─┐   (independent)
Wave 2  foundations     L3 Security (L1,L2)   L6 Relay transport (L1)
Wave 3  the mind+edges  L4 Corpus (L1)   L5 Communication (L1,L2,L3)
                        L7 Workspace (L1,L2,L3)
Wave 4  the hand        L8 Control plane (L1,L3,L4,L5,L6)
Wave 5  the body+face   L9 Deployment + surfaces (L4,L8)
Lane 2  the org         generalization of L1/L3/L5/L7
```

**The bottleneck is Identity (L1).** Almost every layer needs the principal
or the install-id: Security, Corpus, Communication, Workspace, Relay,
Control plane. It is the smallest bedrock piece and unblocks the most. The
Journal (L2) is the second bedrock — the durable home Security's audit and
Communication's bus ride. **Security (L3) is the load-bearing foundation
the owner named** — built right after the bedrock, principal-agnostic from
atom one, so the org is configuration not a retrofit.

## 5. The constraints every atom inherits

- **Self-contained (hard rule 12):** JSONL journal before SQLite; the Rust
  broker; no foreign runtime enters the binary.
- **Fleet's surface stack (§9):** CLI/TUI → face/PWA → Tauri; each atom
  ships its surface at the "genuinely visual" bar (hard rule 11).
- **Without breaking fleet:** owner-holds-all bootstrap; calib stays
  advisory; behavior-preserving refactor with pins proving the owner's
  flows unchanged.
- **The structure guards** (`structure-guards.md`): a principal field on
  `Fact`/`Remember` triggers the struct-field fan-out; a new calib row
  triggers row discipline; a new tool triggers the tool-surface bounds; the
  journal must not make the request path build the cold corpus; socket
  guards keep lane-spending beats off the synchronous path. Run
  `scripts/edit-preflight.sh` before the first compile; one four-leg gate
  at the end (hard rule 2).

## 6. Where to start

**Wave 1, the bedrock: L1 Identity then L2 Journal.** L1.1 (mint the
install-id) and L1.2 (the PrincipalId + owner bootstrap) are the first
atoms — small, non-breaking, and they unblock Security, Corpus, Workspace,
Relay, and the Control plane at once. The Journal follows as the durable
home. Then **L3 Security**, the owner's example, built against its contract
with one principal — the layer that makes the org configuration rather than
a security retrofit. Armada's control plane (L8) is the *last* thing built;
by then every contract it consumes already works for one owner.

## Pointers

- `2026-11-11-armada-direction.md` — the vision being decomposed.
- `2026-11-11-armada-bottom-up-survey.md` — the current-state evidence (file:line).
- `CLAUDE.md` hard rules 2, 5, 7, 11, 12; `structure-guards.md`.