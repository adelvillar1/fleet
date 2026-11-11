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
  the control tunnel.

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

## 7. Open forks (to settle when a lane is drafted)

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
- **Authority transport:** the per-user authority context signed into the
  relay frame by the flagship, so even a compromised instance cannot
  fabricate a privileged call. (The mechanism that makes "actor-blind"
  real rather than aspirational.)
- **The hard/behavioral rule line:** which org rules are code-enforced
  (non-negotiable) vs prompt-shapeable. Likely the first thing an org admin
  sets; fleet's existing guards are the default hard floor, admins add
  above it.
- **Corpus conflict at scale:** the substrate's supersede semantics under
  many concurrent writers across installs (KI-285's two-writer shape,
  distributed). Git is the natural medium; the merge/supersede rules need
  an authority dimension in lane 2 (can user A overwrite user B's fact?).

## 8. Where lane 1 starts

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