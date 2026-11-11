# Armada — a direction note (vision, not a plan)

**Status:** direction / vision. Captured 2026-11-11 from the owner's
ideation, on their word that they see it "implemented sooner rather than
later." This is the seed for a program, not a contract — it carries no
acceptance criteria. When a lane is ready to build, draft a plan from it
(`docs/plans/`). The owner's load-bearing rulings are recorded verbatim in
§6 so a future session inherits the decisions, not just the shape.

## 1. The thesis

Fleet solved *one agent, one machine, reachable from anywhere* (the relay).
Armada is the next epoch: *many agents, many machines, one shared mind —
and a shared hand.* The naming family is deliberate:

- **fleet** — the agent: one self-contained install, one home.
- **armada** — the owner's installs, commanded together, thinking from one
  corpus.
- **fleets org** — the multi-tenant product: many users on an armada.

The shared corpus is the load-bearing idea (the mind); the command-and-
control is the orchestration layer (the hand). Neither is foreign
infrastructure — both connect seams fleet already has (§4).

## 2. The two growth lanes (sequenced)

**Lane 1 — single-owner armada.** One owner, many installs. A shared
corpus (memories, skills, tools) synced across installs; a flagship that
commands them over the relay; deployment of new installs (local or VPS)
that inherit the corpus. The primitives built here — per-identity 2FA,
scoped authority, the command-class gate, the audit stream, per-home
credential separation — are exactly what lane 2 generalizes.

**Lane 2 — the fleets org (multi-user).** Many users on an armada. This is
a *phase change*, not a feature: fleet is single-owner at the root (the
own-home ruling, the approval gate, SOUL.md, the contract's "owner's
appetite," the substrate's provenance all assume one owner). Lane 2
generalizes every one of those from one owner to many: user identity
distinct from device identity, RBAC over the command classes, a
multi-tenant corpus with attribution and visibility, per-user resource
quota, and per-user audit. Build lane 1's primitives once; lane 2 is mostly
generalizing them, not rebuilding.

The bridge between the lanes is **2FA + scoped authority + audit**: the
identity/authority substrate that scales from one owner to many.

## 3. The architecture — three planes

Split the planes by what each medium is good at:

- **The mind — the corpus, synced over git.** The memory substrate is
  already git-tracked markdown with supersede semantics, `source`
  provenance, and a recall-count feedback loop. The armada corpus is a git
  remote that installs pull/push; the substrate's own merge/supersede rules
  handle conflict, and a fact recalled on machine A bumps its rank for
  machine B. Skills (already a shared library at `~/.agents/skills`) and
  tools (the MCP registry) sync the same way. Bulk knowledge rides git, not
  the control tunnel. The corpus has two feeders: the **cross-harness
  memory pull** (`src/memory_pull.rs`, built — the local half) imports a
  project's facts from other installed harnesses' memory stores into the
  substrate, and the **remote-instance feeder** (armada, the global half)
  will distribute the corpus across installs. One corpus, two feeders —
  harnesses feed it locally, armada distributes it globally. The pull's
  `Source` abstraction is the seam the remote feeder plugs into: a
  `RemoteInstance` named stub today, the same revision-cursor interface
  (a git commit) tomorrow.

- **The hand — command-and-control, over the relay.** The relay is already
  a two-way frame tunnel: an instance dials out, polls for inbound frames,
  pushes outbound frames back. Armada is a *new, privileged peer* on that
  tunnel, not a new transport. Each instance holds a relay session id
  (`sid`); the flagship addresses a specific instance by sid. Control
  frames (`dispatch`, `state`, `corpus-sync`, `version-roll`, `retire`)
  reuse the bridge's existing verbs — dispatch is `bridge.run`, state is
  `bridge.state` — so the per-install admission gate (§5) also gates
  armada dispatch. The relay stays a dumb broker; the intelligence is at
  the endpoints. Dial-out gives NAT/firewall traversal for free (a VPS
  behind NAT, a laptop on cellular — no inbound ports). Relay-down is a
  named degradation: instances run autonomously and the corpus still syncs
  over git (the failsafe ruling at armada scale).

- **The body — deployment, the self-contained payload.** Fleet is one
  static binary plus vendored lanes, and `fleet install` already lays out
  the home, scaffolds config, builds the lane venvs, and runs the doctor
  gate. The deployment payload is therefore already a unit: armada drops
  the binary on a target and runs the install — no Node, no checkout, no
  foreign runtime (hard rule 12 doing real work). The release artifacts
  (the raw binary channel especially) are what armada pushes. Local
  instance = armada spawns a new home directly; VPS = armada reaches it
  over SSH (or a cloud API) and drops the same payload. A fresh install
  inherits the corpus by cloning the remote with a fresh install-id, and
  registers with the flagship over the relay.

**The flagship should itself be a fleet install** running in a flagship
role — same binary, plus the control-plane role. Then "deploy fleet from
armada" is fleet deploying fleet, fully self-contained, no foreign control-
plane service to maintain.

## 4. The seams it reuses (nothing foreign)

| Armada need | Fleet seam it grows from |
|---|---|
| Shared corpus sync | the git-tracked memory substrate (supersede, provenance, recall-count) |
| Cross-harness corpus feeder | the memory pull (`src/memory_pull.rs`): `Source` abstraction, revision cursor, import through the `Remember` writer — `LocalHarness` now, `RemoteInstance` stub for armada |
| Shared skills / tools | `~/.agents/skills` library; the MCP registry |
| Two-way control lane | the relay frame tunnel (`src/remote/relay.rs`), dial-out + `sid` addressing |
| Control-frame auth | the relay join-proof + sealed frames (X25519 + AES-GCM + HKDF) |
| Deployment payload | the static binary + `fleet install` + the release raw-binary channel |
| Per-install / per-user admission | the resource contract (`src/contract.rs`, `src/footprint.rs`) |
| The security DNA | the destructive gate (hard rule 7), the always-ask class, the nomenclature write-guard, the own-home ruling |
| Least-privilege seed | the folder grant (KI-492: no grant = no filesystem, refused by name) |
| Audit | the calibration stream (`src/calib.rs`) |
| 2FA primitives | the crate's crypto (HMAC via sha2/hkdf for TOTP, X25519 for signed assertions) + the vault |
| Constitution propagation | SOUL.md read per run (a change reaches the next run on every install) |

## 5. Capacity stays local; the mind is shared

The resource contract is **per-install** (each machine's learned profile)
and, in lane 2, **per-user** (quota within the machine's capacity). The
corpus is shared; capacity is not. A 1 GB VPS and a laptop share a mind but
not a budget. Armada dispatch must respect each install's admission — the
flagship cannot overwhelm a constrained instance, because the instance
admits for itself. A fresh install has no footprint history yet: its
contract starts at the inferred default and *learns* its machine, so
armada treats a new install as un-calibrated until it settles a few runs.
The multi-session admission gate (the 2026-11-11 plan) is the per-install
primitive armada extends.

## 6. The governance & security model — the owner's rulings

These are the owner's decisions (2026-11-11), recorded so a future session
inherits them. They form one coherent model; each grows from a fleet seam.

**R1 — Org SOUL is the constitution, not a user setting.** SOUL.md becomes
a shared artifact of the armada/org that dictates the org's behavior. A
user may have preferences, but **org rules override the user.** The prompt
assembly layers: org SOUL (authoritative base) → org policy → user
preferences (subordinate, bounded) → skills → memory. A fresh install
inherits the org SOUL with the corpus; user preferences stay local to the
user's home. Editing the org SOUL is a high-authority command (2FA +
org-admin), propagated via the corpus, read per run so it is always
current across the fleet.

**R2 — A prompt is not a security boundary.** Split org rules into two
kinds: *behavioral* rules (voice, values, priorities) ride the prompt as
SOUL; *hard* rules ("never exfiltrate credentials," "always gate
destructive ops," "never exceed this budget") are enforced in **code** —
the same deterministic guards fleet already has, synced org-wide. If org
rules live only in SOUL text, a user or a jailbreak can override them and
"org overrides user" is a polite fiction. The constitution needs teeth:
soft rules in the prompt, hard rules in the guards.

**R3 — Armada owns the org; the boundary is actor-blind.** Armada owns the
org and everything that drives and controls it. An unprivileged user must
not be able to change that **personally or via an agent** — the boundary
must be very clear. The threat is the *confused deputy*: a user borrowing
the agent's privilege. So the boundary is enforced at the **action**, in
**code**, against the **originating user's authority** — it does not matter
whether the request comes from a user typing or an agent acting. The agent
inherits its user's (lack of) privilege and cannot escalate. Three
consequences: (a) the governance layer (guards, roles, org SOUL, the
authority checks themselves) is **immutable to the agent**, owned by
armada, changed only via the privileged control plane; (b) org governance
is an **explicit named class** of operations (edit org SOUL, change policy,
modify guards, manage users/roles, deploy/retire, roll versions), each
code-gated, refused by name and audited; (c) the agent's tools are scoped
to the user's authority — an unprivileged user's agent does not even have
the governance tools declared.

**R4 — Least privilege at every layer.** Authority is scoped at the user
level across **every layer**: projects, storage, resources, corpus
sections, tools, skills, workflows, loops. This must be **one coherent
authorization substrate**, not eight ad-hoc permission systems (the gaps
live in the seams). A single **per-user capability context** — grants
across every dimension — travels with every action and is checked by a
uniform guard at every layer. **Default-deny**: anything not granted is
refused. The agent inherits the user's grants at every layer and can
escalate nowhere. Resources → per-user quota; corpus → section-scoped
reads/writes (a user's agent only recalls what they can read); tools,
skills, workflows, and loops are declared per grant (the self-improvement
and calibration loops are themselves granted capabilities, not universal).
The seed is the folder grant (KI-492): "no grant = no access," generalized
from one capability to every capability. It is recursive: **the authority
to grant authority is the root privilege, scoped tightest.**

**R5 — Destructive grants demand step-up 2FA, even when held.** Holding a
destructive grant is necessary but not sufficient; executing the
destructive act demands a **fresh 2FA check at the user level**, per
action. This is fleet's hard rule 7 ("explicit approval in the current
turn") hardened to org scale — the always-ask class as step-up
authentication. The crucial property: **the agent can never satisfy the
2FA** — the challenge is answered by the human out-of-band (TOTP, a push
approval), never by the agent. So even a fully compromised agent holding
every destructive grant cannot execute a destructive act without the
human's live proof. The guard checks two things in code, both required:
the user holds the grant, **and** a fresh 2FA assertion scoped to *this
action* exists (signed by the flagship per user, verified at the guard,
never asserted by the agent). Per-action freshness, not a blanket window.
The destructive class is named (delete, purge, rewrite, force-push, retire,
roll-version, revoke-grants). Recursive: granting destructive authority is
itself destructive, so it too demands step-up.

**The model in one stack:** org SOUL (constitution: prompt for behavior,
code for hard rules) → least privilege (one per-user capability context,
default-deny, uniform guard at every layer) → actor-blind boundary (the
agent inherits grants everywhere, escalates nowhere; governance immutable
to it) → step-up 2FA (destructive grants demand fresh human proof the agent
cannot forge) → audit (every grant check and destructive attempt logged
per user). Identity (pairing) is the channel; 2FA is fresh authority;
scoped grants are least privilege; the three compose.

## 7. The foundational substrate — designed once, the org is a generalization

The owner's insight (2026-11-11): the security model, the multi-user
aspects, the grants, the 2FA — **planned right out of the gate** — make the
fleet-org approach far simpler. The reason is structural: if the
authorization substrate is *principal-agnostic* from the start, then lane 2
is "add principals and roles," not a security retrofit. The substrate is
built once in lane 1 with a single principal (the owner); the org multiplies
the principals and leaves the substrate unchanged. This section designs that
substrate concretely so the rulings R1–R5 become a buildable layer, not just
principles.

**The principle: one principal now, many later, same code.** Every action
carries a *principal* — the unit of authority. In lane 1 there is exactly
one (the owner), so the abstraction looks like overhead; it is not. The
cost is an id, a grant store, and a guard that reads the id. The payoff is
that the guard, the 2FA assertion, the audit, and the governance boundary
are *identical* in lane 2 — only the number of principals and the role
templates change. Designing the abstraction with one principal is what makes
the org configuration rather than reconstruction.

### 7.1 The principal model

A **principal** is who an action acts for, distinct from the **device** it
arrives on. Pairing (R3's channel) authenticates a device; the principal is
the authority the device carries. Lane 1: one principal, `owner`, bound to
the owner's paired devices. Lane 2: many principals (users), plus the rule
that an **agent acts under its user's principal** — never its own. The
principal id travels with every action (every tool call, every relay frame);
the guard reads *that*, never the actor. This is the mechanism that makes
R3's actor-blind boundary real: a user typing and an agent acting present
the same principal, so the guard cannot tell them apart and does not need
to. Device→principal mapping is an identity concern (lane 2's user store);
the substrate only ever sees the principal.

### 7.2 The grant schema

A **grant** is a tuple: `(principal, dimension, scope, action-class)`.

- **dimension** — the layer: `project`, `storage`, `resource`, `corpus`,
  `tool`, `skill`, `workflow`, `loop`, `governance`. The closed set R4 names.
- **scope** — the reach: `instance`, `project:<id>`, `team:<id>`, `org`.
  The vocabulary exists from the start; lane 1 uses `instance` and
  `project:<id>`, lane 2 adds `team`/`org`. A scope is data, not code.
- **action-class** — the verb tier: `read`, `write`, `dispatch`,
  `destructive`, `governance`. The taxonomy is the same for one owner or a
  thousand users; `destructive` and `governance` are the step-up tiers (R5).

**Default-deny**: an action with no matching grant is refused by name. The
capability context is the principal's grant set, resolved per action. The
schema is dimension-based so adding a dimension or a scope is a row, not a
code path — the eight layers R4 names are eight dimensions of one store, not
eight permission systems. The **folder grant (KI-492) is the first row**:
`(owner, project, project:<id>, write)` generalized from "no folder = no
filesystem" to the full schema.

### 7.3 The uniform guard

One guard vocabulary checks the capability context at every layer. It is
the same code in both lanes:

1. resolve the action's `(dimension, scope, action-class)`;
2. look for a matching grant in the originating principal's context;
3. if the class is `destructive`/`governance`, also verify a fresh 2FA
   assertion (§7.4);
4. grant or refuse-by-name, and audit the decision (§7.6).

The guard holds **twice** for agents: the agent's tool declaration is
filtered by the principal's grants (an ungranted tool is not even declared),
and any raw attempt still hits the guard. The governance layer (§7.5) is
outside the agent's writable surface, so the guard cannot be edited by the
thing it bounds.

### 7.4 The 2FA assertion

A **signed, per-action, fresh token**: `(principal, action-class, scope,
target, issued-at, nonce)`, signed by the flagship's key, verified at the
guard. The properties that matter:

- **The agent cannot forge it** — it holds no signing key, and the challenge
  is answered by the human out-of-band (TOTP or a push approval). This is
  R5's load-bearing guarantee: a compromised agent with every destructive
  grant still cannot produce a valid assertion.
- **Per-action freshness** — the token names the target and carries a nonce
  and a short lifetime; it is not a blanket "2FA done, act freely" window.
  Fleet's hard rule 7 ("approval in the current turn") is the seed.
- **Verified, never asserted** — the guard checks the flagship's signature;
  an instance (or an agent) cannot self-assert authority. The signing key
  lives at the control plane, never on an instance the agent can reach.

Lane 1: the owner's second factor. Lane 2: per-user factors. The assertion
*format* is designed once; only the issuer's user store grows.

### 7.5 The governance layer and the root privilege

The grant store, the guard configuration, the role definitions, and the org
SOUL are **immutable to every agent**, owned by the control plane, changed
only through a `governance`-class action (which itself demands step-up 2FA).
**Grant-making is the root privilege**: the authority to grant authority is
a `governance` action on the `governance` dimension, scoped tightest and
audited hardest. Least privilege is recursive all the way down — the layer
that assigns grants is the most-protected surface, and an agent can never
reach it.

### 7.6 The audit substrate

Every guard decision (grant/deny, with principal, dimension, scope, class)
and every destructive attempt is logged per principal, riding the existing
calibration stream (`src/calib.rs`). Designed now with one principal, it
generalizes to many without change — hard rule 5's "model-visible ⟺ logged"
grows its sibling "principal-action ⟺ logged." An agent attempting an
escalation is itself a security signal the audit surfaces, not just a denied
call.

### 7.7 What this buys the org

Because the substrate is principal-agnostic, **lane 2 reduces to four
additions, none a rewrite**: (1) a user store + device→principal mapping
(identity); (2) **roles as grant templates** (a role is a named bundle of
grants; assigning a role stamps its grants onto a principal); (3) per-user
2FA factors; (4) the `team`/`org` scopes already in the schema. The guard,
the assertion format, the audit, and the governance boundary are *unchanged*.
The org is identity + configuration over a substrate that already works —
which is exactly why planning it right out of the gate makes the org simple.

**Build-order implication for lane 1:** build the substrate *with* the
principal abstraction even though there is one principal. Start from the
folder grant and generalize it to the schema; add the guard, the assertion
format, and the audit as the first pieces of armada, not as a later
"security pass." The multi-user org then inherits a working authorization
layer instead of bolting one onto a single-owner assumption.

## 8. The augmented relay — command & control

The relay is armada's nervous system: the two-way lane that carries the
substrate's authority (§7) from the flagship to each instance. The owner's
ruling (2026-11-11): the relay becomes a two-way C2 lane, not just the phone
surface. The key realization is that **the relay is already a two-way frame
tunnel** — armada is a new privileged peer on an existing seam, not new
transport. This section designs the augmentation concretely.

### 8.1 The seam being augmented

Today the instance dials out to the relay (`fleet gui --remote --relay`),
polls for inbound frames (the phone's commands), and pushes outbound frames
(the face's events). Addressing is by relay session id (`sid`); the join is
gated by a join-proof (`FLEET_RELAY_JOIN_SECRET`); frames are sealed
(X25519 + AES-GCM + HKDF). The relay is a broker: it routes sealed frames
between endpoints and understands nothing of their content. Dial-out means
**no inbound ports** — a VPS behind NAT or a laptop on cellular reaches the
flagship with no firewall holes. The relay is the rendezvous.

### 8.2 The flagship peer and instance registration

The flagship is a fleet install in a flagship role (§3) that connects to the
relay as a privileged peer. Each instance **registers** on join: a
registration frame carries its instance id, its `sid`, its version, its
capabilities, and its **resource profile** (the contract's reading — free
space, reserve, appetite). The flagship keeps a **roster** of instances
(id ↔ sid, version, capacity, liveness) and a **heartbeat** tracks which are
live. The roster is what makes "control individual instances" concrete: the
flagship addresses an instance by id, the relay routes to its sid. The
resource profile in the roster is what lets dispatch respect each instance's
admission (§5) — the flagship cannot overwhelm a small VPS because the VPS
admits for itself.

### 8.3 The control frame vocabulary

Beyond the phone's face API, armada adds control verbs. Each **reuses the
bridge verb** the phone already drives, so an armada-dispatched run is just
another run subject to the instance's own admission and guard:

| Verb | Bridge seam | Class (§7.2) |
|---|---|---|
| `dispatch` | `bridge.run` (task, folder, pin) | dispatch |
| `state` | `bridge.state` / sessions | read |
| `corpus-sync` | the substrate's git sync trigger | write |
| `version-roll` | the updater channel | destructive |
| `retire` | instance lifecycle | destructive |
| `health` | the health loop's report | read |
| `register` / `heartbeat` | the roster seam | governance |

A control frame is `{kind: "control", verb, instance, principal, assertion?,
payload, nonce, issued-at}` — sealed by the relay's existing crypto. The
request/response shape rides the same poll/push the phone uses; the flagship
sends a verb, the instance answers on the outbound stream.

### 8.4 The authority transport (the load-bearing piece)

Every control frame carries the **originating principal** (§7.1) and, for
`destructive`/`governance` verbs, a **fresh 2FA assertion** (§7.4) — both
**signed by the flagship's key**. The instance verifies the flagship's
signature *before it obeys*, then runs the verb through its own guard
(§7.3), which re-checks the principal's grants and the assertion. This is
what makes the actor-blind boundary real *across the relay*:

- The **relay cannot fabricate** a privileged call — it forwards sealed
  frames and holds no flagship signing key.
- A **compromised instance cannot escalate** — it can neither forge the
  flagship's signature nor self-assert authority; the guard verifies, never
  trusts.
- An **agent on the instance cannot borrow** the frame's authority beyond
  its principal's grants — the frame's principal is the ceiling, and the
  agent inherits it (§7.1).

The signing key lives at the control plane, never on an instance the agent
can reach. This resolves the "authority transport" fork: the authority rides
*in the frame*, signed, verified at the guard — not asserted by the instance.

### 8.5 The relay stays a dumb broker

The relay routes sealed frames by sid and understands nothing of armada —
no verb parsing, no authority logic, no roster. All intelligence is at the
endpoints (flagship signs, instance verifies). This keeps the relay simple,
keeps it self-contained (it is already a deployment), and means the relay
never sees a plaintext principal or assertion. The augmentation is entirely
in the frame vocabulary and the endpoints; the broker is unchanged.

### 8.6 The two planes and degradation

Control frames are small and latency-sensitive — the relay carries them.
The **bulk corpus rides git**, not the relay: a `corpus-sync` verb says "sync
now," and git does the heavy lift (§3). Relay-down is a **named degradation,
not a refusal** (the failsafe ruling at armada scale): instances keep running
autonomously, the corpus still syncs over git, and live control resumes when
the relay returns. The fleet never stops because its nervous system is quiet.

### 8.7 What lane 1 builds vs lane 2

Lane 1: the flagship peer, instance registration + roster + heartbeat, the
control vocabulary, and the **authority transport with one principal** (the
owner). The substrate (§7) and the relay augmentation are built together —
the frame carries the principal and the assertion from the first cut. Lane
2: the roster grows to many instances, frames carry many principals, and
the flagship's user store issues per-user assertions. The frame format, the
broker, and the verification are unchanged — the org rides the same nervous
system.

## 9. The surface progression — fleet's pattern, reused

The owner's ruling (2026-11-11): **armada follows the same patterns as
fleet** — a Rust-based app, starting with a CLI and TUI, then moving to a
frontend and PWA, then to a Tauri app. This is not a new stack; it is fleet's
proven surface progression reused, and the reason it can be reused is that
**the flagship is a fleet install** (§3). Armada's surfaces are fleet's
surfaces extended, not a parallel app.

### 9.1 Why reuse the pattern

Fleet already walked this arc: slice 1 was a static Rust binary with a CLI
and a ratatui TUI (`src/tui/`); the face came next (the web app the server
renders, `src/gui/`); the native shell last (`fleet-desktop`, `src-tauri/`,
a Tauri window on the same server). The two-crate workspace (`fleet` +
`fleet-desktop`) ships both. Armada inherits every payoff of that arc:

- **Self-containment (hard rule 12)** — one Rust binary, no Node, no foreign
  runtime, no checkout. The control plane is Rust in the workspace; the
  surfaces are the ones fleet already builds and bundles.
- **The progression is de-risked** — each stage shipped for fleet once, so
  armada rides a known path rather than inventing a frontend stack.
- **The flagship-is-fleet identity** — armada's CLI is `fleet armada …`,
  its TUI is a pane in fleet's ratatui surface, its face is a section of
  fleet's web app, its Tauri app is fleet-desktop. One codebase, one install.

### 9.2 The four stages, concretely

1. **Rust core + CLI + TUI (the first cut).** The control-plane logic — the
   roster, the authority transport (§8), the corpus sync, the admission — is
   Rust. The CLI gains `fleet armada` verbs (`roster`, `dispatch`, `status`,
   `sync`, `register`); the ratatui TUI gains an armada board (instances,
   health, capacity, dispatch). Headless and terminal-first, exactly as
   fleet's slice 1 was — the surface a script and an SSH session can drive.
2. **Frontend + PWA.** The face gains an armada section: the roster board,
   the dispatch composer, the corpus view. PWA means installable and
   offline-capable, reachable from anywhere over the relay (§8) — the
   owner's phone commands the fleet the same way it commands one instance
   today. The surface follows the owner's standing rulings: a centre
   conversation with a context pane on the right, the chat composer borrowed
   never rebuilt, the owner's work as the view.
3. **Tauri app.** fleet-desktop wraps the armada face in the native shell,
   the same way it wraps fleet's face today — no second frontend, no second
   transport, the bridge and server reused.

Each stage is a shipping increment with its own gate, not a polish finale —
**UX is co-equal (hard rule 11)**: every stage ships its surface at the
"genuinely visual, never plain-text tables" bar.

### 9.3 The workspace shape

Armada lands as a module (or a sibling crate) in the fleet workspace, not a
separate repo with its own runtime. The control plane is library code the
CLI, the TUI, the face, and the Tauri shell all call — one implementation,
four surfaces, the same seam fleet's bridge already is. The relay
augmentation (§8) is part of that core; the surfaces render it. This keeps
the whole armada self-contained and installable as fleet is, and means an
instance and a flagship differ by *role and configuration*, not by binary.

## 10. Prior art: muxin

The owner's reference (2026-11-11): [muxin](https://github.com/LumaryLabsLLC/muxin)
(Lumary Labs, Apache-2.0, Rust/GPUI, muxin.dev) — "all your machines, one
app, for you and your AI agents." It is a fleet-management platform already:
agent sessions, remote desktops, files, logs, runners, and GitHub across
Macs/Linux/Windows, with an MCP server. It validates armada's shape and
offers concrete patterns to borrow. Armada borrows its *architecture*, keeps
fleet's *surfaces* (§9, the owner's Tauri ruling — not muxin's GPUI), and adds
what muxin does not have: the shared corpus, the security substrate, the org.

### 10.1 Patterns to borrow

- **The on-machine agent + persistence.** muxin uploads a small binary
  (`muxen-agent`) to each managed machine over SSH; it holds terminal
  sessions independently of the app (tmux-like — closing the app detaches,
  reopening reattaches with scrollback). This is exactly armada's instance
  model (§3): a small agent per machine, the flagship connects to it. The
  persistence pattern — sessions survive the control plane's absence — is
  worth adopting: an instance keeps working when the flagship is offline
  (armada's degradation ruling, §8.6, made concrete).
- **Agent-first, MCP-first.** Every muxin capability is a structured JSON
  tool via `muxin mcp`, *and* a CLI command (`--json` everywhere), *and* a
  GUI path — so any MCP agent can operate the whole fleet programmatically.
  Armada's control vocabulary (§8.3) should be exposed the same way: the
  relay verbs as MCP tools, so an agent can command the fleet, not just a
  human. The window/CLI/MCP sharing config state and running simultaneously
  validates §9.3's "one implementation, four surfaces."
- **Connection progression + mutual cert pinning.** muxin reaches machines
  over SSH first, then optionally upgrades to QUIC with **mutual certificate
  pinning**, with Tailscale roaming and automatic rebuild on network change
  (local addresses preferred). Armada's authority transport (§8.4) can borrow
  the mutual cert pinning — the flagship↔instance pair pins each other's
  certs, so a rogue peer cannot impersonate either end. The resilience
  patterns (rebuild on roam, prefer local) are worth adopting.
- **Boxes — autonomous isolated tasks.** muxin runs autonomous agent tasks
  in fresh Docker containers (own checkout, own branch, idle pause/resume,
  shared credentials). Fleet already has LayerFS delegated workspaces;
  armada's autonomous dispatch can ride the same box pattern, scheduled
  across the roster.
- **Sharing with admission control.** muxin's P2P sharing has admission
  control and view/type permissions per guest — a concrete, working grant
  pattern that informs §7's grant schema (a share is a scoped, revocable
  grant to a principal).
- **Auto-join provisioning.** muxin provisions DigitalOcean droplets / local
  VMs that join the fleet automatically — armada's "deploy fleet from
  armada" (§3) with a concrete reference for the provision→register→join
  flow.

### 10.2 The contrasts (where armada deliberately differs)

- **Transport: relay vs SSH/QUIC.** muxin dials *in* over SSH (then QUIC),
  which needs inbound access to the machine. Armada's instances dial *out*
  to the relay (§8), needing no inbound ports — the right default for
  arbitrary instances behind NAT. Armada can offer both: the relay for any
  instance, SSH/QUIC for owned machines where it is available, muxin-style.
- **Surface: Tauri vs GPUI.** muxin is GPUI (Zed's GPU framework). Armada
  follows fleet's ratatui→face/PWA→Tauri progression (§9) by the owner's
  ruling — reusing fleet's proven, self-contained surface stack rather than
  adopting a new one.
- **Single operator vs org.** muxin is "you and your agents" — one operator.
  Armada's lane 2 is the multi-user fleets org (§7's principal model), which
  muxin does not address.

### 10.3 Where armada goes beyond muxin

muxin manages *machines and sessions*; armada shares a *mind*. The three
things muxin does not have, which are armada's distinctive layer:

1. **The shared corpus** — the git-synced knowledge across instances (§3),
   fed by the cross-harness pull. muxin moves files and sessions; armada
   moves *knowledge*, so a fresh instance thinks from the fleet's memory.
2. **The security substrate** — the principal/grant/guard/2FA layer (§7),
   least privilege at every layer with step-up 2FA the agent cannot forge.
   muxin's sharing permissions are a seed; armada's substrate is the full
   model, designed once for the org.
3. **The org** — multi-user principals and roles (§7.7), the fleets org
   built on the substrate. muxin is single-operator; armada generalizes.

**The synthesis:** borrow muxin's instance-agent + persistence, its
MCP-first agent-first surface, its cert-pinned connection progression, its
boxes and auto-join provisioning; keep fleet's Tauri surface stack and the
dial-out relay; add the shared corpus, the substrate, and the org. muxin
proves the fleet-management shape works; armada adds the shared mind and the
org-ready security model on top.

## 11. The communication layer — omni's pattern, fleet first

The owner's reference (2026-11-11): [omni](https://github.com/automagik-si/omni)
(automagik-si, TypeScript/Bun) — "a universal event-driven omnichannel
platform for AI agents to communicate across any messaging platform." Its
octopus metaphor is the architecture: channel plugins are the tentacles,
NATS JetStream carries the nerve impulses (events), and the core is the brain
(identity resolution + routing). Every state change is journaled in
PostgreSQL (total-ordered) and published over NATS — **the journal is the
source of truth** for replay, tracing, and durable consumption. The owner's
intent: **implement this on fleet first, then extend to armada.**

### 11.1 The pattern to borrow

An event-driven, journal-backed omnichannel communication layer, in five
pieces:

1. **The event journal** — every state change is a total-ordered event,
   appended to a durable journal that is the source of truth. Replay,
   causation tracing, and durable named consumers all read the journal. This
   is fleet's hard rule 5 ("model-visible ⟺ logged") generalized from the
   session log to *every* communication event.
2. **The pub/sub transport** — events publish to durable consumers; channels
   and agents subscribe. Omni uses NATS JetStream; fleet borrows the
   *pattern* (durable consumers, schema enforcement, causation tracing), not
   the broker (§11.4).
3. **Channel plugins (omnichannel)** — a unified send/receive API across
   platforms behind a Channel SDK: webhook, Slack, Discord, Telegram, etc.
   Each channel is a plugin at the edge; the core never knows the platform.
4. **The identity graph** — resolve the same person across channels into one
   identity. This is armada's **principal model (§7.1)** at the communication
   edge: a message from Slack and a message from the relay resolve to the
   same principal, so the guard (§7.3) sees one authority, not many channels.
5. **Agent dispatch** — provider bindings (webhook, claude-code, a2a),
   response gates, debouncing, rate limits, session strategies. This is
   armada's control dispatch (§8.3) with omni's gating: a response gate is
   the approval seam, a rate limit is the resource contract, a session
   strategy is the run's scope.

### 11.2 Fleet already has the seeds

Fleet is not starting from zero — the pieces exist, scattered:

- **`src/channels/`** — the channel service (webhook today; the uncommitted
   work this session touched `channels/mod.rs` and `webhook.rs`). The plugin
   edge is already shaped.
- **The bridge event stream** — the broadcast the face reads over SSE is the
   pub/sub seam.
- **The session JSONL log** — an append-only journal already; the calib
   stream (`src/calib.rs`) is a second journal. Unifying them into one
   total-ordered event journal is the consolidation omni's pattern asks for.
- **The relay** — the phone/bot channel (§8) is already a channel plugin in
   embryo.

So "fleet first" means: unify these into one event-driven communication
layer — a journal-backed bus with channel plugins, identity resolution, and
gated agent dispatch — rather than build a new subsystem. The face, the CLI,
and the relay all read the same journal.

### 11.3 Extending to armada

The communication layer is what makes armada's instances *converse*, not
just receive commands. The relay (§8) is the transport; omni's pattern is
the communication model riding it:

- The **identity graph** resolves a principal across instances and channels
  — the substrate's principal (§7.1) becomes the communication layer's
  identity, so authority and messaging share one resolution.
- The **event journal** federates across the roster: an instance's events
  join the fleet's journal, so the flagship traces causation across machines
  (a dispatch on A caused a run on B caused a corpus commit on C).
- The **agent dispatch** is the control vocabulary (§8.3) with omni's gates —
  response gates = the approval seam, rate limits = the contract, session
  strategies = the run scope. A2A routing is instance-to-instance dispatch.

### 11.4 The self-containment contrast

Omni is Bun/TypeScript + NATS JetStream + PostgreSQL. Fleet borrows the
**architecture, not the stack** (hard rule 12): the journal is fleet's
existing JSONL/SQLite (no Postgres dependency), the bus is an embedded Rust
pub/sub (no NATS broker to run), the channels are Rust plugins (no Bun). The
pattern — journal-as-truth, durable consumers, omnichannel plugins, identity
graph, gated dispatch — is what transfers; the foreign runtimes do not. This
keeps the communication layer installable as fleet is, one binary.

**Build order:** the communication layer is a fleet-first candidate (the
seeds exist), and it is the piece that turns armada from a command plane
into a communicating fleet. It composes with the substrate (§7, identity =
principal) and the relay (§8, transport) rather than replacing either.

## 12. The team workspace — orbit's inspiration for the org

The owner's insight (2026-11-11): "it's easy to work by yourself, but teams
need special tools." Single-owner armada (lane 1) is a command plane; the
multi-user org (lane 2) is a **team workspace**, and
[orbit](https://github.com/coollabsio/orbit) (coollabsio, Apache-2.0, Rust +
React, orbit.so) — "a self-hosted workspace for teams: tasks, docs, chat, and
mail in one app," the open-source Slack/Linear/Notion alternative — shows the
tool set a team actually needs. Armada's org is not just "more principals
commanding agents"; it is a place where humans and agents *collaborate*.

### 12.1 The team-tool layer

Orbit's four surfaces are the shape lane 2 grows into, each composing with a
piece armada already designs:

- **Tasks** (workflows, list/board/timeline, sub-issues, labels, GitHub sync)
  — the work-tracking layer. A task becomes a dispatch (§8.3): assign it to
  an instance or an agent, track working/blocked/done, link the PR. Fleet's
  goal and huddle machinery is the seed; the org generalizes it from one
  owner's goals to a team's board.
- **Docs** (page tree, live collaborative editing, version history) — the
  human-facing view of the **shared corpus**. The substrate's facts are the
  knowledge; docs are the collaborative surface over them. The corpus sync
  (§3, the pull) feeds the docs; editing a doc writes back to the corpus with
  the principal's provenance (§7.1).
- **Chat** (channels, DMs, threads) — the team conversation, carried by the
  **omni communication layer (§11)**. The journal-backed bus is the
  transport; chat is the team-facing surface over it, with the identity
  graph resolving who said what across channels.
- **Mail** (a shared inbox) — the team's external correspondence, the same
  omnichannel edge (§11) turned toward email.

### 12.2 Orbit's admin layer validates the substrate

Orbit's admin features map almost one-to-one onto §7 — independent
confirmation that the substrate is the right shape:

| Orbit admin | Armada substrate |
|---|---|
| roles | roles as grant templates (§7.7) |
| audit log | the per-principal audit (§7.6) |
| rate limits | the resource contract's per-user quota (§5, §7.2) |
| API tokens | a principal's credentials (the device→principal mapping, §7.1) |
| workspace export | the corpus as the portable, git-synced unit (§3) |
| MCP server with OAuth | the agent-first surface (§11) + the authority transport (§8.4) |

The lesson: a team workspace *needs* roles, audit, rate limits, and tokens —
which is exactly the substrate §7 designs once, out of the gate. Building the
substrate in lane 1 means lane 2's team workspace inherits its admin layer
already working, rather than bolting on permissions after the collaboration
tools exist.

### 12.3 The composition

The org is four layers composing, not a new monolith:

1. **The substrate (§7)** — principals, grants, guards, 2FA, audit. The
   security and admin floor (orbit's admin layer).
2. **The communication layer (§11)** — the journal-backed omnichannel bus.
   The transport for chat and events (orbit's chat, omni's backbone).
3. **The shared corpus (§3 + the pull)** — the git-synced knowledge. The
   substance behind docs (orbit's docs, armada's distinctive mind).
4. **The team-workspace surfaces** — tasks, docs, chat, mail. The
   collaboration tools (orbit's shape), rendered on fleet's surface stack
   (§9, Tauri/PWA).

### 12.4 The contrast and the lane-2 implication

Orbit is a standalone workspace (Rust + React + PostgreSQL, Docker-deployed);
armada borrows the **tool shape, not the stack** — the surfaces ride fleet's
Tauri/PWA progression (§9), the data rides the corpus and the journal, the
security rides the substrate. Self-contained, one install (hard rule 12).

The implication for lane 2: the org succeeds only if it is a workspace teams
*want* to work in, not just a control plane they tolerate. Orbit proves the
tool set (tasks/docs/chat/mail) is what teams reach for; armada's advantage
is that agents are first-class citizens of that workspace — a task dispatches
to an instance, a doc is written by a principal human or agent, the chat
carries agent events — all on the substrate that makes it safe. **The
substrate built in lane 1 is what lets lane 2 be a team workspace instead of
a security retrofit.**

### 12.5 The fleet-first refactoring — the workspace as connective tissue

The owner's product insight (2026-11-11): a slimmed-down orbit could refactor
fleet's *whole bots-and-desk experience* today, not just the future org —
because **each work type has a surface but no connective tissue.** The desk
has deliverables and lessons, the conversation has chat, the bots have
huddles, learning has lessons — but nothing ties a task to its conversation
to its deliverable to the bot that did it to the lesson it taught. The
surfaces are silos.

The refactoring is to make the workspace the **work-object graph** — task ↔
thread ↔ deliverable ↔ bot ↔ lesson — and let the existing surfaces become
*views over that graph* rather than separate rooms. The slimmed-down orbit
(tasks + docs + chat, the mail and the heavy PM dropped) is the spine; the
desk's deliverables/lessons, the conversation, and the huddles hang off it.

**Bot invocation becomes a workspace action, not a ceremony.** Today a bot
is summoned through a separate huddle convocation or the bot relay. In the
workspace:

- **Assign a task to a bot** — the assignee is a persona; assigning
  dispatches the task (`bridge.run` under the role overlay), and the bot's
  work lands as a deliverable *linked to the task*, with working/blocked/done
  status (omni's session hooks, §11).
- **@mention a bot in a thread** — the huddle pattern made contextual: the
  bot is summoned into the conversation, replies in-thread, and its work
  stays connected to that thread.
- **Co-author a doc** — a bot writes a section into the corpus with
  provenance (the bot's role acting under the user's principal).

This reuses fleet's machinery — `src/roles.rs` (personas), the huddle
(convocation), `bridge.run` (dispatch), the role overlay — rather than
rebuilding it; the workspace makes invocation contextual and the results
connected. The substrate governs it: a principal invokes a bot only if
granted (§7.2), the bot acts under the user's principal (§7.1, the bot is
the deputy), and a destructive bot task hits the step-up gate (§7.4).

**Why fleet-first:** the connective tissue is valuable for one owner now,
and it is the *same* workspace the org grows into (§12.3) — so the graph
built for a single owner is the team workspace later, not a rewrite. The
refactoring pays for itself twice: it connects fleet's siloed surfaces today
and lays the org's collaboration floor tomorrow.

## 13. Open forks (to settle when a lane is drafted)

- **Flagship shape:** synchronous remote-control (dispatch and watch live,
  like the face today) vs intent-based dispatch (drop a goal, the instance
  admits and executes on its schedule, reports back). Decides how much
  state the relay carries.
- **Org tenancy:** users share the same installs (a team commanding a
  common fleet — tenancy on the control side) vs each user owns installs
  (an org of personal fleets sharing a corpus — tenancy on the knowledge
  side). Diverges hard on the control plane.
- **Second factor:** time-based TOTP (simple, offline) vs push approval
  (smoother; could ride the bot-relay phone pairing already in place).
- **Authority transport — designed in §8.4:** the principal + assertion ride
  signed in the relay frame, verified at the instance's guard. The remaining
  detail is **signing-key custody and rotation** at the control plane (where
  the flagship key lives, how it rotates, how a compromised key is revoked
  across the roster).
- **The hard/behavioral rule line:** which org rules are code-enforced
  (non-negotiable) vs prompt-shapeable. Likely the first thing an org admin
  sets; fleet's existing guards are the default hard floor, admins add
  above it.
- **Corpus conflict at scale:** the substrate's supersede semantics under
  many concurrent writers across installs (KI-285's two-writer shape,
  distributed). Git is the natural medium; the merge/supersede rules need
  an authority dimension in lane 2 (can user A overwrite user B's fact?).
- **Substrate scope in lane 1 (§7):** build the full principal/grant/guard/
  assertion/audit substrate now (the org-ready foundation, §7.7's
  recommendation) vs a minimal single-owner version generalized later. The
  owner's lean is the full substrate out of the gate — the cost is small and
  the org becomes configuration; the fork is how much of it lane 1 carries
  before the first armada cut.
- **Role granularity:** a role as a flat grant bundle vs composable/
  hierarchical roles (a role inherits another's grants). Flat is simpler and
  enough for most orgs; hierarchy adds power and the recursion to audit.
- **Workspace shape (§9.3):** armada as a module inside the `fleet` crate vs
  a sibling crate in the workspace. A module is simpler and shares the
  bridge directly; a sibling crate isolates the control plane's compile
  surface and lets an instance build without it. The flagship-is-fleet
  identity holds either way.

## 14. Where lane 1 starts

The multi-session work (the 2026-11-11 plan) is the first primitive:
per-install admission, concurrent runs, the polite reconsider-or-queue
refusal. Lane 1 builds outward from there — the corpus git-sync, the
flagship role on the relay, the deployment payload, per-identity 2FA and
the command-class gate. Each is a plan of its own; this note is the map
they share.

## Pointers

- `docs/plans/2026-11-11-multi-session.md` — the per-install admission
  primitive (lane 1's first brick).
- `docs/plans/2026-11-09-the-resource-contract.md` — the admit/settle
  arithmetic capacity stays local on.
- `docs/architecture/surfaces.md`, `docs/architecture/lanes.md` — the
  surfaces and lanes armada commands.
- `CLAUDE.md` hard rules 4 (own-home), 7 (destructive approval), 12
  (self-containment) — the rulings the security model generalizes.