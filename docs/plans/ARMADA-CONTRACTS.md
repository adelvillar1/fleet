# Armada — the contract ledger (what each module offers / expects)

**The interface registry — one of the three ledgers.** The
[decomposition ledger](ARMADA-DECOMPOSITION.md) records the *atoms* (the build
map); the [design-decisions ledger](ARMADA-DESIGN-DECISIONS.md) records the
*choices* (D0–D57); this records the *interfaces* — for every module, what it
**OFFERS** (the exact calls and types other modules may use) and what it
**EXPECTS** (the offers it consumes, naming the provider). It removes the
guesswork from execution: when you build a module you know precisely what it
must provide and what it may rely on, and every seam is written down once.

How to read it:
- **OFFERS** — the module's public contract. Signatures are Rust-flavored and
  normative; a change here is a contract change (update the consumers).
- **EXPECTS** — the offers this module consumes, each naming its provider
  (`module::offer`). An EXPECTS that names no OFFER below (or in the existing
  fleet seams) is a **broken seam** — fix it before building.
- **CONSUMED BY** — the reverse index (derived from EXPECTS): who relies on
  this module's offers. Touch an offer, check this list.

The consistency rule: **every EXPECTS resolves to an OFFER.** The new
modules' offers are design (settled by the plans, cited D-ids); the existing
fleet seams are verified against the code at **0.8.20** (file:line), refreshed
2026-11-19 by `docs/plans/2026-11-19-armada-bridge-console.md` C1. The stamp
names the version it was verified at so the next drift is detectable rather
than silent — it had sat at 0.6.30 for five minor releases.

## 0. Shared types (defined once, in the bedrock)

```text
InstallId    = String   // 32 hex, D7; identity::install()
PrincipalId  = String   // lowercase slug, D8; lane 1 = "owner"
DeviceId     = String   // a paired client, D2
Seq          = u64      // the journal's total order, D10
Dimension    = project|storage|resource|corpus|tool|skill|workflow|loop|governance   // D14
ActionClass  = read|write|dispatch|destructive|governance                            // D14
Scope        = instance|project(id)|team(id)|org                                     // D14
```

## 1. Existing fleet seams (verified at 0.8.20 — armada consumes these)

These OFFERS already exist; the new modules EXPECT them. They are the raw
material, not armada builds. Every citation below was re-read in the tree on
2026-11-19; where 0.6.30's line drifted, the current line stands.

| Provider | Offer (verified) | Evidence |
|---|---|---|
| `home` | `fleet_home(fleet_home_env, user_home) -> PathBuf`; `sessions_dir(home)`; `read_soul(home)` | `src/home.rs:13,29,45` |
| `filelock` | `FileLock::beside(path)`; `FileLock::take(lock) -> FileLock` | `src/filelock.rs:33,42` |
| `gui::bridge` | `run(session, task)`; `run_as(session, task, kind, role)`; `run_as_principal(...)` (L5/D34); `run_with(...)` / `run_with_budget(...)`; `state(session) -> Value`; `huddle(goal, personas)`; `huddle_breakout(...)`; `hand_to_desk(id)` (D39); `projects() -> Value` (D40); `assign_task(...)` (L7/D41–D42); `armada() -> Value`; `armada_dispatch(body) -> Result<Value>` (L9); `answer(...)`; `stop(steer)`; `resume(asked)` | `src/gui/bridge.rs:5528,5545,5624,5580,5648,1367,974,1238,1013,1841,1081,2080,2120,6602,6221,6358` |
| `memory` | `load_facts(root) -> Vec<Fact>`; `resolve_root_from(cwd, home)`; `Memory::remember(&Remember)`; `Memory::supersede(...)`; `recall(instruction) -> Vec<ScoredFact>`; `rewrite_field`; `archive_fact` | `src/memory.rs:1023,969,4873,5169,3096` |
| `memory_pull` | `trait Source { harness(); project(); read() -> Snapshot }`; `pull(source)`; `Snapshot { revision, facts, skips }` | `src/memory_pull.rs:72,731,60` |
| `contract` | `admits_concurrent(reading, in_flight) -> Admission`; `footprint::admit(available, reserve, estimate) -> Admission`; `Oversight::measure(&Readout) -> Standing` | `src/contract.rs:90,266`; `src/footprint.rs:415` |
| `channels` | `trait Channel`; `Policy::authorize(&peer) -> Result<Capability, Refusal>`; `Peer { channel, id, name }` | `src/channels/mod.rs:637,296,271` |
| `remote::relay` | `join()`; `poll() -> Vec<Frame>`; the sealing (X25519+HKDF+AES-GCM) | `src/remote/relay.rs:358,393` |
| `agent` | `Agent::prepare_call` (the choke point); `declaration_set` (the filter); `AgentOptions` (the run context) | `src/agent.rs:5402,2507,502` |
| `approvals` | `Gate` (the mode gate); the in-turn approval (hard rule 7) | `src/approvals.rs:262` |
| `workspace` **(new at 0.8.20)** | `WorkObject`; `link(from, to, kind)`; `view(home, filter) -> Vec<ViewRow>`; the append-only graph at `graph_path(home)` (D37) | `src/workspace.rs:71,278,414,190` |
| `roles` **(new at 0.8.20)** | `Role { name, description, tools, skills, shapes, workload, overlay }`; `load_all(home)`; `catalog(home) -> Vec<RoleEntry>` — **persona overlays for delegation, carrying no authority** | `src/roles.rs:30,217,263` |
| `plugins::face` **(new at 0.8.20)** | `load_registry(home)`; `save_registry(...)`; `list(home)`; the registry at `<home>/plugins.json`; the bundle at `<home>/plugins/<name>/manifest.json` | `src/plugins/face.rs:172,182,309,18` |
| `mcp_registry` **(new at 0.8.20)** | `enabled_specs(...)` — the server registry the MCP client resolves through | `src/mcp_registry.rs:246` |

## 2. L1 — `src/identity.rs` (D7–D9)

**OFFERS**
```text
identity::install() -> InstallId                       // read-or-mint, persisted
identity::principal_for(ctx: &RunContext) -> PrincipalId   // lane 1: owner
identity::device_principal(device: &DeviceId) -> Option<PrincipalId>
identity::link_device(device: &DeviceId, principal: &PrincipalId) -> Result<()>  // governance, settles a pairing event
identity::registry() -> IdentityRegistry               // the store view
```
**EXPECTS**
- `home::fleet_home` — where `install-id` / `identity.json` live.
- `filelock::FileLock::take` — atomic store writes.
- `journal::append` — the measured pairing event (L2; D4 cross-cutting).

**CONSUMED BY** L3 (`principal_for`), L4 (`install`), L5 (`device_principal`, `link_device`), L6 (`install`), L7 (assignee), L8 (`install`, `principal_for`), L9 (`install`).

## 3. L2 — `src/journal.rs` (D10–D13)

**OFFERS**
```text
journal::append(event: Event) -> Seq                   // total-ordered, durable
journal::subscribe(consumer: &str, from: Seq) -> EventStream   // durable cursor
journal::replay(from: Seq) -> Vec<Event>
journal::cursor(consumer: &str) -> Seq
journal::retention() -> Budget                         // learns from settles
Event { seq: Seq, ts, kind: String, install: InstallId,
        principal: Option<PrincipalId>, session: Option<String>, payload: Value }
```
**EXPECTS**
- `home::fleet_home` — the journal home.
- `filelock::FileLock::take` — the single-writer Seq counter.
- `identity::install` — the install on every event.

**CONSUMED BY** L1 (pairing), L3 (guard audit), L4 (`corpus_sync`), L5 (bus, outbox, dispatch), L7 (`work`), L8 (tracing, reliability beat).

## 4. L3 — `src/security.rs` (D14–D19)

**OFFERS**
```text
guard::check(principal, dimension, scope, class) -> Decision
Decision = Granted | Denied { name: String } | StepUpRequired
guard::filter_declarations(principal, decls: ToolSet) -> ToolSet   // ungranted tool not declared
assertion::sign(flagship_key, principal, class, scope, target) -> Assertion   // flagship only
assertion::verify(token: &Assertion, public_key, principal, class, scope, target) -> bool
grants::store() -> GrantStore ; grants::grant(Grant) ; grants::revoke(Grant)   // governance
Grant = (PrincipalId, Dimension, Scope, ActionClass)
Assertion { principal, class, scope, target, issued_at, nonce, signature }   // Ed25519, ring
```
**EXPECTS**
- `identity::principal_for` — the principal the guard reads (D9).
- `journal::append` — the `guard` audit event (D17).
- `agent::prepare_call` — the hook site (D15).
- `agent::declaration_set` — the filter site (D16).
- `approvals` in-turn approval — lane-1 step-up proof (D18, non-breaking).

**CONSUMED BY** L5 (gated dispatch), L7 (invocation), L8 (authority transport, verb gating), L9 (provision governance).

## 5. L4 — `src/corpus.rs` (D23–D29)

**OFFERS**
```text
corpus::sync(remote: &Remote) -> Result<SyncOutcome>   // fetch → merge → push
corpus::remote() -> Option<Remote>                     // config; None = named refusal
corpus::merge(base: Tree, ours: Tree, theirs: Tree, rules) -> Merged   // pure
merge::recall_count(base, ours, theirs) -> u64         // sum/delta, D24
merge::supersede_winner(candidates) -> Frontier        // basis ladder, D26
RemoteInstance: Source                                 // the feeder, D27 (read at git commit)
SyncOutcome { before, after, merged, summed, resolved, refused }
```
**EXPECTS**
- `memory::{load_facts, remember, supersede, rewrite_field, archive_fact}` — the substrate.
- `memory_pull::{Source, pull}` — the unchanged import path.
- `identity::install` — serials (D25) + provenance.
- `journal::append` — the `corpus_sync` event + read-back grade (D28).
- git (shell: fetch / push / rev-parse) — transport only.

**CONSUMED BY** L8 (`corpus-sync` verb), L9 (fresh-install inheritance).

## 6. L5 — `src/channels/` extended (D30–D36)

**OFFERS**
```text
Channel::reply(to, text) -> Result<(), SendError>      // was fire-and-forget; now measured
Channel::present_seat(to, seat) -> Result<(), SendError>
outbox::send(reply: Reply, text: String) -> Result<()> // persist-before-attempt, durable retry
identity_graph::resolve(peer: &Peer) -> Result<PrincipalId, Refusal>   // total-or-named, D32
bus::publish(event: Event) ; bus::subscribe(consumer: &str) -> EventStream   // = journal, D33
dispatch::gated(task: String, principal: PrincipalId) -> Result<Run>   // guarded, D34
```
**EXPECTS**
- `journal::{append, subscribe}` — the bus + outbox (D33, D30).
- `identity::{device_principal, link_device}` — the graph store (D31).
- `security::guard::check` — gated dispatch (D34).
- `bridge::run` — the dispatch.
- `agent::AgentOptions.principal` — thread the resolved principal (D9).

**CONSUMED BY** L6 (the relay channel), L7 (work events), L8 (instance→flagship replies).

## 7. L6 — `src/remote/broker.rs` + `relay.rs` extended (D20–D22)

**OFFERS**
```text
broker::route(frame: Frame, sid: Sid)                  // dumb, Rust, by sid
broker::push(frame: Frame, sid: Sid) -> Ack            // unsolicited, ack + cursor
broker::liveness(sid: Sid) -> Liveness                 // heartbeat-measured, D4
relay::send(frame: Frame, sid: Sid) -> Answer          // existing, sealed
Frame { kind: "control"|"face", verb, install: InstallId,
        principal: PrincipalId, assertion: Option<Assertion>,
        payload, nonce, issued_at }                    // D22
```
**EXPECTS**
- `identity::install` — the frame's install.
- `security::Assertion` — the frame carries it (D18).
- `remote::relay` sealing — X25519+HKDF+AES-GCM, unchanged.

**CONSUMED BY** L5 (the relay channel), L8 (the control lane).

## 8. L7 — `src/workspace.rs` (D37–D43)

**OFFERS**
```text
graph::link(from: ObjId, to: ObjId, kind: EdgeKind) -> Result<()>   // refuses dangling/unknown
graph::view(filter: Filter) -> Vec<WorkObject>
task::assign(task: TaskId, principal: PrincipalId) -> Result<Run>  // bot invocation, gated
desk::attach_session(project: ProjectKey, session: SessionId)      // the hand_to_desk fix
WorkObject { id, kind: task|thread|deliverable|bot|lesson }
Edge { from, to, kind } ; Task { id, status: working|blocked|done, assignee: PrincipalId }
```
**EXPECTS**
- `identity::principal_for` — the assignee (D38).
- `journal::append` — the `work` event (D43).
- `security::guard::check` — invocation gating (D42).
- `bridge::{run_as, huddle}` — the invocation seam (D41).
- `session` / `desk` / `memory`(lesson) / `roles` stores — derived references (D37, KI-300).

**CONSUMED BY** L8 (reads the work graph), L9 (the surfaces).

## 9. L8 — `src/armada/` (D44–D50)

**OFFERS**
```text
roster::register(install: InstallId, sid: Sid, version: String, capacity: Reading)
roster::heartbeat(install: InstallId) ; roster::list() -> Vec<Instance>
control::dispatch(install, task, principal, assertion: Option<Assertion>) -> Result<Run>
control::state(install) ; control::corpus_sync(install) ; control::version_roll(install, version)
control::retire(install) ; control::health(install)        // each guarded + admitted, D46
authority::sign_frame(flagship_key, frame) -> Frame        // flagship only
authority::verify_frame(public_key, frame) -> bool         // instance, before obeying, D45
flagship::role() -> Role                                   // role + config, one binary, D47
factor::issue(principal, action) -> Result<Assertion>      // human 2FA, agent cannot forge, D48
reliability::grade()                                       // scheduled beat over the journal, D49
Instance { install, sid, version, capacity, liveness, calibrated }
```
**EXPECTS**
- `identity::{install, principal_for}` — D7/D8.
- `security::{guard::check, assertion}` — D15/D18.
- `corpus::sync` — the corpus-sync verb (D23).
- `channels::dispatch::gated` — gated dispatch (D34).
- `remote::broker::{push, liveness}` + `relay` — the control lane (D21/D22).
- `bridge::run` — dispatch reuses it (D46).
- `contract::admits_concurrent` — the instance admits for itself (§5).
- `journal::{append, subscribe}` — tracing + the reliability beat (D49).

**CONSUMED BY** L9 (deployment + the surfaces).

## 10. L9 — `src/armada/deploy.rs` + surfaces (D51–D57)

**OFFERS**
```text
deploy::provision(target: Target) -> Result<InstallId>   // local spawn | VPS over SSH
deploy::register_join(install: InstallId) -> Result<()>  // inherit corpus + register
fleet armada { roster | dispatch | status | sync | deploy }   // CLI
tui::armada_board ; gui::armada_section ; fleet_desktop::armada   // the surfaces, D53–D56
Target = Local { home } | Vps { ssh, ... }
```
**EXPECTS**
- `install` (`fleet install`) — the home layout + doctor gate.
- `corpus` (clone remote) — fresh-install inheritance (D29/D52).
- `armada::roster::register` — join (D44).
- `armada::control` — the verbs the surfaces drive (D46).
- `tui` / `gui` / `src-tauri` — the surface stack (§9).

**CONSUMED BY** the owner; lane 2 (org provisioning + team surfaces).

## 10a. As-built record — v0.6.38 → v0.7.0 (recorded 2026-11-12)

The L7–L9 sections above were drafted at plan time; these are the signatures
as built (append-only amendment, per the ledger rules — the drafted text
stands as intent, this is the contract in force). Everything here is verified
by the module tests cited in the plans.

**§8 as-built — `src/armada/` (roster, authority, control)**

```text
roster::register(home, install_id, sid, version, capacity_kib, now) -> Result<Instance>
roster::heartbeat(home, sid, now) -> Result<()>          // cannot register an unknown sid
roster::live_instances(home, now) -> Vec<Instance>       // liveness earned, D44
roster::{load(home) -> Roster, save(home, roster)}      // atomic tmp+rename
authority::mint_flagship_key(home) -> Result<Ed25519KeyPair>   // once; PKCS#8 0600 (L8 filled the custody gap, D47)
authority::{load_flagship_key(home), load_public_key(home)}
authority::sign_frame(key, verb, install, principal, payload, nonce, issued_at, class, scope, target) -> ControlFrame
authority::verify_frame(frame, class, public_key, now) -> Result<(), String>   // forged/unsigned/stale/mismatch refused by name, D45
control::Verb { Dispatch|State|CorpusSync|VersionRoll|Retire|Health|Register|Heartbeat }  // .class()/.dimension(), D46
control::admit(home, frame, public_key, now) -> Result<Admitted>               // authority → guard → step-up, D15/D18/D45
control::execute(home, admitted) -> Result<Outcome{Done|Dispatch{task,principal}|Destructive{verb,target}}>
control::grade(home, sid) -> Reliability{settled, succeeded, calibrated, reliability}  // D49
control::relay_down_note() -> &'static str              // D50, named degradation
Instance { install_id, sid, version, capacity_kib, last_heartbeat, registered_at }  // .live(now)
```

**§9 as-built — `src/workspace.rs` (L7)**

```text
read(home) -> Graph{tasks: BTreeMap<String,Task>, edges: Vec<Edge>}   // append-only JSONL folded on read, D37
link(home, from: WorkObject, to: WorkObject, kind: EdgeKind) -> Result<(), String>
                                                                      // dangling/mistyped refused by name, D39
link_named(home, from_id, from_kind, to_id, to_kind, edge_kind)       // the CLI seam, C0
view(home, filter: Option<Kind>) -> Vec<ViewRow{object, binding, entry, practice}>  // D40
create_task(home, title, assignee: PrincipalId, binding) -> Result<Task>
set_status(home, task_id, status) -> Result<Task>
admit_assignment(home, task, principal) -> Result<()>                 // L3 guard + guard journal event, D42
settle_work(home, task_id, deliverable, principal) -> Result<Status>  // graded by the desk end gate, D43
Kind = task|thread|deliverable|bot|lesson ; Status = working|blocked|done
```

**§10 as-built — `src/armada/deploy.rs` + the surfaces (L9)**

```text
deploy::bootstrap_corpus(home, remote) -> Result<()>                  // clone the mind, not the identity, D52
deploy::provision_local(home, remote: Option<&str>) -> Result<ProvisionReport>
deploy::vps_commands(host, user, home, payload_url, remote) -> Vec<String>  // pure; credentials out-of-band, D51
deploy::register_fresh(flagship_home, install_id, sid, version, capacity_kib, now) -> Result<Instance>
deploy::trusted(home, sid, doctor_passed) -> bool                     // doctor AND settles, D57
deploy::provision_class() -> ActionClass::Governance                  // step-up, D18/D19
deploy::journal_provision(home, install_id, target, corpus_cloned)
fleet armada { roster | status | sync | deploy <home> [--remote <url>] | dispatch <sid> <task> }   // C5
tui BoardKind::Armada + board_view::render_armada     // the armada board (genuinely visual, hard rule 11)
gui GET /api/armada ; POST /api/armada/dispatch       // the face section reads the same library
fleet-desktop wraps the same face                      // no second frontend, U29/D56
```

**Face seams the reskin consumes (the U-reskin plan's EXPECTS)** — all reads
over existing stores, no new capability: `/api/skills` (exists), the tools
registry read (new), the workflow store read (new), `/api/calibration`
(exists), `/api/desk` (exists), `/api/onboard` (exists).

## 10b. As-built record — the 0.8.20 seam refresh (recorded 2026-11-19)

The refresh `2026-11-19-armada-bridge-console.md` C1 performed, so the next
reader sees what changed rather than only the result. Every row of §1 was
re-read in the tree; the drift it found:

**Citations that moved (0.6.30 → 0.8.20).** `agent::prepare_call` 5184 → 5402
(a second `prepare_call_checked` exists at 6451 and is *not* the choke point
the contract names — the un-checked one is); `agent::declaration_set`
2443 → 2507; `AgentOptions` 492 → 502; `approvals::Gate` 258 → 262;
`channels::Policy::authorize` 295 → 296 and `trait Channel` 636 → 637;
`memory::remember` 4869 → 4873; `memory::supersede` 5165 → 5169;
`memory::recall` 3092 → 3096.

**Offers the ledger named but no longer existed, or never did.** The
`remote::relay` row cited `serve(...)` at 452 and sealing at 126 — neither is
a function at those lines; the real offers are `join` (358) and `poll` (393),
and the sealing is module-level (X25519+HKDF+AES-GCM), not a `serve` call.
The row is rewritten to what exists.

**New seams this plan consumes (recorded because they are EXPECTS now).**
`workspace` (`WorkObject`/`link`/`view`/`graph_path`, D37 landed after the
ledger was written); `roles` (`load_all`/`catalog` — persona overlays that
carry **no authority**, which is what makes the plan's role templates new
work rather than a rename); `plugins::face` (the face plugin registry at
`plugins.json` + per-bundle `manifest.json` — the carrier candidate, consumed
by nothing here); `mcp_registry::enabled_specs` (the client-side registry the
new served endpoint's counterpart resolves through).

**The stale claim worth naming.** The §9 as-built block and §10's surface row
describe the face armada section as "the roster board, a dispatch composer,
and a corpus view". That is what shipped at 0.7.0 and it is still what the
tree has — the console this plan builds is the *replacement*, so those rows
stay accurate until C6 lands and are rewritten then, not now.

## 11. The bidirectional consistency check

Every EXPECTS above resolves to an OFFER in §1–§10. The load-bearing seams,
read both ways:

- `identity::principal_for` — offered by L1; expected by L3, L7, L8.
- `journal::append` — offered by L2; expected by L1, L3, L4, L5, L7, L8.
- `security::guard::check` — offered by L3; expected by L5, L7, L8.
- `security::Assertion` — offered by L3; expected by L6, L8.
- `corpus::sync` — offered by L4; expected by L8, L9.
- `remote::broker::push` — offered by L6; expected by L5, L8.
- `bridge::run` / `run_as` — offered by existing fleet; expected by L5, L7, L8.
- `contract::admits_concurrent` — offered by existing fleet; expected by L8.

**The rule for execution:** before building a module, read its OFFERS (what
you must provide) and its EXPECTS (what you may rely on, already offered
below). Before changing an offer, read its CONSUMED BY. A new seam is added
here *first*, then built — the contract precedes the code, so the layers
compose without guesswork.