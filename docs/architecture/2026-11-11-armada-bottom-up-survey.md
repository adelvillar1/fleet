# Armada, bottom-up — the fleet-first seams and the sequencing approach

> **Superseded as the approach.** The owner reframed armada as a new
> bottom-up build of layers fitted into fleet, each exposing well-defined
> contracts armada consumes — not a widening of existing seams. The
> contract-first atomic decomposition is now
> [`2026-11-11-armada-atomic-decomposition.md`](2026-11-11-armada-atomic-decomposition.md).
> This doc is retained as the **current-state evidence** (the §2 corrections
> and §3 seam map, with file:line); its §4–§5 framing no longer governs.

**Status:** direction / survey, not a plan. A deep dive taken 2026-11-11
against fleet 0.6.30 (`c20666c`), reading the armada direction note
(`2026-11-11-armada-direction.md`) against the actual code. It carries no
acceptance criteria; each increment below becomes its own plan in
`docs/plans/` when the owner says build. It is the map from 0.6.30 to an
armada-capable fleet, built **fleet-first, bottom-up, without breaking
fleet** — the owner's constraint: the feature set lands first, armada is
implemented on top of it once the set is done.

## 1. The thesis, and what the constraint forces

The direction note frames armada as four layers — the substrate (§7),
the augmented relay (§8), the communication layer (§11), the team
workspace (§12) — plus the shared corpus (§3) and the deployment payload
(§3). The owner's constraint re-reads every one of them through a single
question: **does this pay for itself in single-owner fleet today, and can
it land without breaking what fleet already does?** If yes, it is a
fleet-first foundation and it goes early. If it only makes sense with
many installs or many users, it is armada-only and it waits.

The payoff of building this way is the direction note's own §7.7 claim
made real: a foundation built once for one owner is the same code the org
generalizes. The risk the constraint guards against is the opposite — a
"security pass" or a "multi-user retrofit" bolted onto a single-owner
assumption after the fact. So the discipline is: **build the abstraction
with one principal now, keep the owner's behavior identical, and let
armada add principals and installs as configuration.**

Three facts from the code shape everything below:

- **There is no principal anywhere.** `grep principal src/` is empty.
  Every guard answers "may this call run?" from mode + tool + arguments +
  folder, never from "who is acting." The substrate (§7) is therefore
  more net-new than "generalize a seam" — the seam is the folder grant's
  *shape* (default-deny, refused by name), not an existing identity.
- **There is no durable, total-ordered event journal.** The face's event
  stream is an in-memory broadcast; the session log is per-session; the
  calib stream is deliberately lossy. The communication layer (§11) and
  the audit substrate (§7.6) both need a durable home that does not exist
  yet.
- **There is no install identity and no corpus remote-sync.** No
  install-id/instance-id in `home.rs`/`install.rs`; `memory.rs` never
  invokes git (a daily shell script commits, nothing pushes, no merge
  code). The shared mind (§3) is genuinely unbuilt beyond the local pull.

## 2. Corrections to the direction note's seed claims

The note's §4 seam table is directionally right but several seeds are
mislabeled or absent. Each correction changes a build estimate, so they
are recorded here with evidence (paths relative to the repo root):

| The note says | The code actually shows | Evidence |
|---|---|---|
| "the desk has deliverables and lessons" (§12.5) | `desk.rs` holds **deliverables only**; lessons are memory facts written by `lesson.rs` into `memories/`. Four stores, not one (desk, memory, roles, sessions). | `src/desk.rs:184-245`; `src/lesson.rs:362-451` |
| the folder grant generalizes to the substrate (§7.2) | The grant exists **only on the GUI chat surface**. CLI, TUI, cron, eval all run `FileAccess::Unrestricted` — the launch dir is an implicit grant. | `src/main.rs:3506`, `src/tui/mod.rs:922`, `src/cron.rs:1584`, `src/eval.rs:1391` |
| the channel `Peer` is the proto-principal (§11.4) | `Peer` is a real (identity, capability) check, but it **evaporates at `bridge.run`** — the run becomes an ordinary owner run; the peer never reaches the approver, `FileAccess`, or calib. | `src/channels/mod.rs:295-301,929-931` |
| audit rides the calib stream (§7.6) | calib is **lossy by contract** (`try_send` drops on a full queue, counted) and advisory; `ToolScreenRow` has **no actor/session/principal**. It cannot be the durable audit home without a new durable store. | `src/calib.rs:1921-1939,110-127` |
| the relay "is already a deployment" (§8.5) | The broker is **`relay/relay.py` — Python, in-memory, zero persistence**. A foreign runtime: the one self-containment violation in this area (hard rule 12). The fleet-side client is pure Rust and is the half worth keeping. | `relay/relay.py:149-162` |
| a fresh install clones "with a fresh install-id" (§3) | **No install-id exists.** The only ids are paired-device ids (`Device.id`) and ephemeral relay `sid`s. Identity must be minted. | `src/remote/pairing.rs:308-324`; `src/remote/listener.rs:1235-1244` |
| the corpus is git-synced (§3) | Local git tracking only, via `scripts/daily-maintenance.sh:86-98` (commit, **no push**). **No merge/conflict code** in `memory.rs`. Remote-sync is unbuilt. | `src/memory.rs:14-17` |
| "eight rail pages" (CLAUDE.md) / "fifteen" (surfaces.md) | **19 destinations, 4 groups** in the single declaration. Both docs lag. | `src/gui/assets/app.js:275-319` |
| the journal is "fleet's existing JSONL/SQLite" (§11.4) | **No SQLite anywhere in the binary.** JSONL only, and no record type carries a global sequence number. | grep across `src/` |

The net effect: **§7 (substrate) and §11 (communication) are foundations
to build, not seams to widen**, and the relay needs a Rust broker before
it can be armada's self-contained nervous system. The corpus pull
(0.6.30) is the one piece that is exactly as the note describes — its
`Source` trait is a clean, test-proven seam.

## 3. The seam map — layer by layer

### 3.1 The substrate (§7): principal, grant, guard, assertion, audit

**Real seams.** `prepare_call` (`src/agent.rs:5184`) is the single choke
point every built-in/lane call crosses — the natural home for a uniform
`(principal, dimension, scope, action-class)` guard. `declaration_set`
(`src/agent.rs:2443`) already filters tool declarations per-scope;
per-principal filtering is the same predicate shape (this is R3's "an
ungranted tool is not even declared"). The folder-grant refusal
(`src/agent.rs:5193-5202`) is the default-deny seed: refused by name,
model-visible, names the remedy. `Policy::authorize`
(`src/channels/mod.rs:295`) is the one existing single-site
(identity, capability) check to generalize.

**Genuinely missing.** A principal type and per-run propagation (children
inherit everything *except* an identity that never existed —
`src/subagents.rs:2305-2308,2624-2654`); a grant store (today only
`sessions-workdirs.json`, one dimension, one implicit owner, plus
`approval_rules` keyed `(tool, program)` with no principal/expiry);
assertion verification (no signatures, no 2FA, no freshness anywhere —
consent is a prompt answer or a config word); durable per-principal audit.

**Traps.** bash is not sandboxed — the folder bounds file tools only, so a
storage-dimension guard around `resolve_in_root` misses the shell
(`src/tools.rs:4066-4068`). Four distinct `Gate` types collide by name
(approvals/layerfs/cron/improve). Check order is load-bearing and
documented (folder before the gate, scope after — `src/agent.rs:2690-2695`).
The local GUI has no auth beyond loopback Host/Origin (`src/gui/server.rs:264-281`).

### 3.2 The communication layer (§11): journal, bus, channels, identity, dispatch

**Real seams.** The `Channel` trait (`src/channels/mod.rs:636-672`) is a
working plugin SDK — four methods (`name/serve/reply/present_seat`),
push-style inbound, normalized `Inbound`, webhook + telegram already
built. `Driver` + `Policy` is gated dispatch. `record()`
(`src/gui/bridge.rs:12340`) is the single seam every UI event crosses —
the natural journal append point; `fan_out` and the SSE `stream_events`
become projections of it. The session JSONL gives durable per-session
replay with a typed, torn-tail-tolerant reader. `Driver::watch`
(`src/channels/mod.rs:792`) is already a named-consumer prototype (it just
consumes an ephemeral broadcast today).

**Genuinely missing.** A total-ordered durable journal (no global sequence
exists in any record type); durable named consumers with cursors (the
broadcast drops on lag, `Driver::watch` silently continues); a durable
outbox (channel `reply`/`present_seat` are `let _ = ureq::post` — a failed
POST is lost); an identity graph (`Peer` is `(channel, id)` by explicit
design — "the same human on Telegram and webhook is two peers with one
owner", pinned by test); push over the relay (request/response only).

**Traps.** The SSE "replay" is a 4096-cap in-memory deque, not a journal
(`src/gui/bridge.rs:61,12377`). calib cannot be promoted to source of
truth without breaking its lossy contract. The channel activity log is
capped-500 rewrite-on-append, not an append log (`src/channels/service.rs:211-229`).
Telegram's offset is in-memory only. Flush ≠ fsync everywhere. Queued runs
and standing seats are in-memory — a crash loses them.

### 3.3 The workspace (§12.5): the work-object graph

**Real seams.** The thread (session JSONL) is the richest object —
personas, join/leave, mention, byline, tasks, goal, parent links
(`src/session.rs:202-655`). A huddle **is a session with personas**, not a
separate object (`src/gui/bridge.rs:748-773`); a parallel huddle store
would violate the KI-300 one-derivation rule. `projects()` already unions
deliverables + lessons + practices **by workspace folder**
(`src/gui/bridge.rs:1586-1677`) — the nearest existing thing to a graph
view, but folder-keyed, not object-keyed. Roles (`src/roles.rs:30-53`) are
prompt/skill/tool overlays — specialization, explicitly not a security
boundary (`src/agent.rs:61-65`).

**Genuinely missing.** A first-class **task** object with status +
assignee + cross-session identity. The `todo` tool is a renderer; the only
persistence is a full-replacement `Tasks` record in one log, no
id/assignee/blocked (`src/session.rs:194-197`, `src/tools.rs:3384-3423`).
`graph.rs` nodes have real status machinery but are cwd-bound dev-plan
orchestration, not a runtime work graph. The missing edges: task↔anything,
thread→deliverable (reverse), bot↔deliverable (typed — today only
`send_file` prose parsing), deliverable↔lesson, bot↔lesson, goal↔task.
`hand_to_desk` loses the thread (`desk.session` stays None) and binds the
server's cwd, not the room's workspace (`src/gui/bridge.rs:782-806`).

**Traps.** Three meanings of "deliverable" (DeskProject, a handed file, a
deck render). The bot-run audit spine (`DelegatedRunRow`) is lossy calib —
unsuitable as a durable work-object store. Huddles are face-only (the TUI
has no convene).

### 3.4 The shared corpus (§3): git-sync, portability, merge

**Real seams.** The fact substrate is exactly as designed: one markdown
file per fact, fixed frontmatter, `Basis` provenance (`confirmed-by` /
`validated` / `inferred`, the last two forge-proof on the write path),
supersede-with-chain-fold and read-back verification, recall-count
feedback (`src/memory.rs:5165-5458,3234-3244`). The pull's `Source` trait
(`src/memory_pull.rs:72-81`) is the clean seam: `harness()`, `project()`,
`read() -> Snapshot{revision, facts, skips}`. `RemoteInstance`
(`:449-476`) is a named stub returning `NotYetBuilt`; `pull()` is fully
source-agnostic (test C4 drives a scripted source through the identical
path). The revision cursor already produces `git:<HEAD>`. Skills
(`~/.agents/skills`) and `mcp.json` (env-names-only by design) are
sync-ready artifacts.

**Genuinely missing.** Install identity; a corpus remote + pull/push
driver; **any** merge/conflict rule. The recall-count is a read-modify-write
counter under a local-only flock — federation needs a sum/delta merge, not
last-writer-wins. The recall index is digest-keyed, gitignored, local —
never sync it.

**Traps.** Daily counter churn dominates the corpus commit — naive
pull/push conflicts on counter lines constantly. KI-285's two-writer lost
update goes distributed: `replacement_name`'s free-serial scan and
`archive_target`'s serials are local-disk decisions two installs make
identically → add/add and `-rN` collisions. Supersede's atomicity assumes
single-writer disk. Skills sync changes *other* harnesses' behavior; the
retire list must stay per-install (KI-215). `mcp.json` mixes syncable
config with local runtime state (`last_probe`, `enabled`).

### 3.5 The augmented relay (§8): flagship, roster, control, authority

**Real seams.** The fleet-side relay client is production-grade Rust:
dial-out, `sid` addressing, join-proof, sealed frames (X25519 + HKDF +
AES-256-GCM, AAD bound to path/frame-id so bodies can't replay across
frames) — `src/remote/relay.rs:126-280`. The bridge verbs control frames
would reuse (`run/state/answer/stop/resume/control`) exist
(`src/gui/bridge.rs`). A run-control mechanism already exists (`bridge.control`
+ the `.control` file, `src/delegate.rs:2460-2477`).

**Genuinely missing.** A Rust broker (today Python, in-memory). Push
semantics — the protocol is strictly request→one-answer, device-initiated
pull; `/events` degrades to polling (`src/remote/listener.rs:597-612`).
A control-frame discriminator, broker-side push/queue for unsolicited
frames, and an ack/cursor story (frames are handed out once and cleared).
The flagship peer, roster, heartbeat, registration, and the authority
transport (signed principal + assertion in the frame) are all net-new.

### 3.6 Capacity (§5) and deployment (§3) — mostly done / mostly armada

Capacity is **built**: the resource contract (`src/contract.rs`,
`src/footprint.rs`) and multi-session admission (`admits_concurrent`,
0.6.28) are the per-install primitive armada extends; a fresh install
starts un-calibrated and learns. Deployment is **mostly armada**: `fleet
install` already lays out the home and runs the doctor gate; the
provision→register→join flow and "deploy fleet from armada" are the
armada-only remainder.

## 4. Fleet-first vs armada-only

| Piece | Pays for itself in single-owner fleet? | Armada role | Verdict |
|---|---|---|---|
| Event journal + durable bus + outbox (§11) | **Yes** — crash-safe runs/seats, real face replay, reliable channel replies, cross-stream tracing | the journal federates across the roster; the audit home | **fleet-first, early** |
| Principal + grant + uniform guard (§7) | **Yes** — coherent authority on *all* surfaces (fixes headless Unrestricted), Peer identity survives into runs, durable audit | the org adds principals + roles, substrate unchanged | **fleet-first, early** |
| Work-object graph (§12.5) | **Yes** — connects the silos, contextual bot invocation (the owner's product insight) | the same graph is the team workspace | **fleet-first, mid** |
| Identity graph at the channel edge (§11.4) | **Yes** — one owner across telegram/webhook/relay is one identity | the principal at the comm edge | **fleet-first, mid** (rides substrate) |
| Install-id + corpus portability + merge rules (§3 prereq) | **Partly** — corpus backup/restore, multi-machine readiness | fresh-install inheritance, distributed supersede | **fleet-first-ish, mid** |
| Rust relay broker + push (§8 prereq) | **Yes** — hard rule 12 fix, better phone experience | the self-contained control lane | **fleet-first-ish, mid** |
| 2FA assertion format + step-up (§7.4) | **Weakly** — hardens hard rule 7, but no second factor today | the org's per-user step-up | **late lane-1 / lane-2** |
| Corpus remote-sync driver (§3) | **No** — one install has nothing to sync with | the shared mind | **armada-only** |
| Flagship role + roster + heartbeat + control verbs + authority transport (§8) | **No** — no roster with one install | the hand | **armada-only** |
| Deployment payload / provision→register→join (§3) | **No** | the body | **armada-only** |
| Armada surfaces (§9): `fleet armada`, TUI board, face section | **No** | the control plane's face | **armada-only** |
| Org: user store, roles-as-templates, per-user 2FA, team/org scopes, team surfaces (§7.7, §12) | **No** | lane 2 | **lane-2** |

## 5. The sequencing approach — dependency-ordered phases

Each brick is a plan of its own, lands with its own gate and its own
surface (hard rule 11), and preserves single-owner behavior. The order is
forced by dependencies, not preference: the journal is the home the
substrate's audit and the workspace's events ride; the substrate's
principal is the workspace's assignee and the channel edge's identity;
install-id and the Rust broker are the corpus-sync and control-plane
prerequisites.

**Phase 0 — already laid (the first bricks).** Resource contract +
multi-session admission (capacity, §5) and the cross-harness pull (the
corpus's local feeder, §3). Done at 0.6.28–0.6.30.

**Phase 1 — the two foundations.**

1. **The event journal + durable pub/sub + outbox.** Put a durable,
   total-ordered journal behind `record()`; make the SSE stream and
   `Driver::watch` consumers of it with cursors; add a durable channel
   outbox; journal the seat/control/queued-run events that die on crash
   today. **Why first:** it is additive — it cannot change authority
   semantics, so it is the safest "without breaking fleet" brick; it fixes
   real reliability gaps the beta shakeout hits (a crash loses queued runs
   and standing seats; channel replies are lost; face replay is a capped
   deque); and it is the durable home the substrate's audit (§7.6) needs,
   so building it first means the substrate lands its audit durably from
   day one. **Non-breaking shape:** add a *new* journal for UI/seat/control
   events + the outbox; do **not** promote calib (keep it advisory) and do
   **not** merge the session log into it. JSONL with a monotonic sequencer
   is the lower-risk path; SQLite is a deliberate dependency decision
   fleet has not made (none exists today).
2. **The principal + grant + uniform guard substrate, one principal.**
   Thread a principal through `AgentOptions` and the `Approver` trait;
   generalize the folder grant into the `(principal, dimension, scope,
   action-class)` store; land the uniform guard at `prepare_call` and the
   per-principal declaration filter at `declaration_set`; make `Peer`
   survive into the run; audit every guard decision into the Phase-1
   journal. **Why second:** it is the load-bearing security foundation the
   owner wants "right out of the gate" (§7.7) — and journal-first honors
   that, because both are lane-1 foundations before any control plane.
   **Non-breaking shape:** the owner principal holds every grant by
   default; default-deny bootstraps to "owner allowed everything," so
   nothing the owner does today starts being refused. This is a careful
   behavior-preserving refactor, not a big-bang. Design the grant schema
   and assertion *format* here even though 2FA verification lands later.

**Phase 2 — the connective tissue (rides Phase 1).**

3. **The work-object graph.** A first-class task object (id, status
   working/blocked/done, assignee = principal); the missing edges
   (task↔thread↔deliverable↔bot↔lesson); fix `hand_to_desk` to carry the
   thread; surfaces become views over the graph; bot invocation becomes a
   workspace action (assign a task to a persona → `bridge.run` under the
   role overlay, work lands as a linked deliverable; @mention in-thread;
   co-author a doc with provenance). Reuse roles/huddle/bridge.run, never
   rebuild; a huddle stays a session (KI-300). The assignee is the Phase-1
   principal; work-object events ride the Phase-1 journal.
4. **The identity graph at the channel edge.** Resolve `Peer`/`Device`
   across channels to one principal — the substrate's principal at the
   communication edge (§11.4). Folds naturally into the substrate's
   device→principal mapping; one owner today, many users in lane 2.

**Phase 3 — armada prerequisites that still serve fleet.**

5. **Install identity + corpus portability + merge rules.** Mint an
   install-id; corpus export/import; the recall-count sum/delta merge rule
   and the distributed KI-285 fix (serial collisions, add/add). Fleet-first
   value: corpus backup/restore and multi-machine readiness. This is the
   substrate the remote-sync driver needs before it can exist.
6. **The Rust relay broker + push semantics.** Replace `relay.py` with a
   self-contained Rust broker (hard rule 12); add push/event frames with
   ack/cursor. Fleet-first value: the relay stops being a foreign runtime
   and the phone gets a live stream instead of polling. The control lane's
   foundation.

**Phase 4 — armada itself (only after the feature set).**

7. **Corpus remote-sync.** The `RemoteInstance` feeder over the `Source`
   trait, git pull/push, the `corpus-sync` verb, fresh-install inheritance.
8. **The flagship role + roster + heartbeat + control vocabulary +
   authority transport.** Registration, the roster (id↔sid, version,
   capacity, liveness), dispatch reusing `bridge.run`, and the signed
   principal + assertion in every control frame, verified at the instance
   guard before it obeys. The 2FA assertion *verification* (format designed
   in Phase 1) lands here with the flagship's signing key.
9. **The deployment payload.** Provision→register→join; local spawn and
   VPS drop of the same self-contained binary.
10. **The armada surfaces (§9).** `fleet armada` CLI verbs, the ratatui
    armada board, the face armada section, fleet-desktop — one
    implementation, four surfaces, the bridge seam reused.

**Lane 2 — the org (a generalization, not a rebuild).** User store +
device→principal mapping; roles as grant templates; per-user 2FA factors;
the `team`/`org` scopes already in the schema; the team-workspace surfaces
(tasks/docs/chat/mail) over the same graph, journal, and corpus. The
guard, assertion format, audit, and governance boundary are unchanged.

**The recommendation, plainly:** start with **Phase 1 brick 1, the event
journal.** It is the safest non-breaking brick, it fixes reliability the
shakeout depends on, and it is the home everything else's audit and events
ride. Design the substrate's schema alongside it; build the substrate
second. Armada's control plane (Phase 4) is the *last* thing built, exactly
as the owner asked — by then every primitive it needs already works for
one owner.

## 6. The "without breaking fleet" contract

Every increment inherits these; they are the difference between a
foundation and a retrofit:

- **Single-owner bootstrap.** The owner principal holds all grants; the
  guard's default-deny must not refuse anything the owner does today.
  Headless surfaces gaining a grant model must default to the launch-dir
  grant they implicitly have now, not to refusal.
- **calib stays advisory.** The journal is a new durable home; promoting
  calib to source of truth breaks its lossy backpressure contract.
- **Self-containment (hard rule 12).** JSONL sequencer before SQLite (a
  dependency fleet has not made); the relay broker becomes Rust; no foreign
  runtime enters the binary.
- **The structure guards** (`docs/architecture/structure-guards.md`): a
  principal field on `Fact`/`Remember`/`FactEdit` triggers the struct-field
  fan-out (every constructor in the same commit); a new calib `Row` variant
  triggers row discipline (variant + `op()` + `ts()` + `OP_*` + `OPS` +
  the pinned test); a new tool triggers the tool-surface bounds
  (`MAX_VISIBLE_TOOLS` 36, `MAX_DEFINITION_TOKENS` 10,050); the journal must
  not make the request path build the cold corpus; the socket guards keep
  lane-spending beats off the synchronous path. Run `scripts/edit-preflight.sh`
  before the first compile.
- **Behavior-preserving refactor discipline.** Pins before edits, scoped
  test runs, one full four-leg gate at the end (hard rule 2). The principal
  threading is the risky one — it touches every tool call's authority path,
  so it lands behind tests that prove the owner's existing flows are
  byte-for-byte unchanged.

## 7. Open forks that bear on the order (from §13)

- **Substrate scope** — full principal/grant/guard/assertion/audit now vs a
  minimal single-owner version. The owner leans full (§7.7); Phase 1 builds
  the full schema with one principal and defers only 2FA *verification*.
- **Workspace shape** — armada as a module in the `fleet` crate vs a sibling
  crate. Decides whether an instance can build without the control plane;
  bears on Phase 4, not the foundations.
- **Second factor** — TOTP vs push approval (could ride the bot-relay phone
  pairing). Bears on Phase 4's assertion verification.
- **Flagship shape** — synchronous remote-control vs intent-based dispatch.
  Decides how much state the relay carries; bears on Phase 4.
- **Corpus conflict rule** — the recall-count merge and the authority
  dimension on supersede (can user A overwrite user B's fact?). Phase 3
  builds the single-owner merge; lane 2 adds the authority dimension.

## Pointers

- `docs/architecture/2026-11-11-armada-direction.md` — the vision this survey grounds.
- `docs/architecture/surfaces.md`, `docs/architecture/structure-guards.md` — the surfaces and the invariants.
- `docs/plans/2026-11-09-the-resource-contract.md`, `docs/plans/2026-11-11-multi-session.md`, `docs/plans/2026-11-11-cross-harness-memory-pull.md` — Phase 0, laid.
- `CLAUDE.md` hard rules 2 (gate), 5 (model-visible ⟺ logged), 7 (destructive approval), 11 (UX co-equal), 12 (self-containment).