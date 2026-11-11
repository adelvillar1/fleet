---
name: armada-multi-install-c2-idea-2026-11-11
description: "project 2026-11-11 — the owner's emerging \"armada\" idea, a command-and-control for multiple fleet installs sharing one corpus of memories/skills/tools, EXTENDED to deploying fleet from armada to local/VPS instances that inherit the corpus, to the RELAY as the two-way control lane, to hardening trust with a 2FA layer over device pairing, and to a SECOND GROWTH LANE — multiple users on an armada/\"fleets org\" (2FA prepares the identity substrate), and to SOUL.md as the ORG's shared constitution — not a user se…"
type: project
date: 2026-11-11
basis: inferred
tier: durable
shape: []
phrases: []
missing: []
premises: []
recall-count: 0
superseded-by:
valid-until:
source: zcode:fleet:content:285983102772d99aaed2c496d57acdf6f320aad7347d3891e9bb9e2378839d34
---

The owner's idea, "bouncing around my head for a few days" (2026-11-11):
fleet is maturing and reachable from anywhere via the relay, so what happens
with MULTIPLE installs? Maybe **armada** — a command-and-control for fleets
with a SHARED corpus of memories, skills, tools, etc. The name fits the
Fleet-of-Ones ecosystem. Captured 2026-11-11 as a direction note
(`docs/architecture/2026-11-11-armada-direction.md`) on the owner's ruling
"capture it because I see this being implemented sooner rather than later" —
a vision doc to seed plans from, not yet a plan or a committed build. The
rulings below are the owner's; the framing around them is my proposal.

My framing (a proposal, NOT an owner ruling): fleet already solved "one
agent, one machine, reachable from anywhere"; armada is "many agents, many
machines, one shared mind." The shared corpus is the load-bearing new part;
the C2 is orchestration over two seams fleet already has — the git-tracked
memory substrate (sync the corpus over a git remote, the substrate's own
supersede/merge rules handle conflict, recall-count feedback goes
cross-install) and the relay (a flagship control plane the installs dial
out to). Skills are already a shared library (~/.agents/skills).

The constraints I named (grounded in standing rulings, see
[[the-resource-contract-admit-work-settle-curtail]] and
[[self-containment-is-a-standing-contract]]): share KNOWLEDGE never
credentials or machine config (the own-home ruling); the resource contract
stays PER-INSTALL (each machine's learned profile is its own), so armada
dispatch must respect each install's admission — the multi-session
concurrency gate ([[multi-session-plan-drafted-ki-525-2026-11-11]]) is a
primitive armada needs; every install needs an identity for provenance.

THE OPEN FORK that shapes everything: a **flagship you command** (dispatch
work to installs, watch them all — real C2, but a service that must meet
self-containment, or a fleet install in "flagship mode") vs a **shared
mind** (installs stay autonomous, think from one corpus — simpler peer git
sync, no dispatch). They share the corpus layer but diverge on the control
plane. My lean was corpus-via-git + C2-via-a-flagship-on-the-relay. It's a
PROGRAM, not a plan (like learning was). Do not treat any of this as decided
until the owner rules.

THE OWNER'S EXTENSION (2026-11-11, same ideation): "also deploying fleet from
Armada to other instances, local or vps and then they inherit the shared
corpus." Armada is not just C2 over existing installs — it GROWS the fleet:
deploy → seed → command → share → update → retire. Still ideation, no plan.

My framing of the deployment leg (proposal, NOT a ruling): self-containment
(hard rule 12) is what makes it tractable — the install payload is ALREADY a
unit (one static binary + vendored lanes + `fleet install` lays out the home,
config, lane venvs, doctor gate), so armada drops the binary and runs the
install, never assembles a runtime; the release artifacts (the raw-binary
channel especially) are the payload. Local instance = spawn a new FLEET_HOME
directly; VPS = reach it over SSH/cloud API, same payload. The relay's
dial-out model suits VPS (no inbound ports) and doubles as the registration
channel (join-proof). Corpus inheritance = git clone of the armada remote +
a fresh install-id (inherits the mind, starts its own recall/provenance
history). Credentials NEVER ride the corpus — provisioned out-of-band;
armada's SSH/cloud creds are its OWN secrets. Two things this forces open:
(1) the flagship should itself be a fleet install in flagship mode (fleet
deploying fleet, fully self-contained, no foreign control-plane service);
(2) version coherence (armada pins a version, rolls updates via the updater
channel) and contract cold-start (a fresh VPS has no footprint history — its
contract starts inferred and LEARNS that machine, so dispatch treats a new
install as un-calibrated until it settles a few runs; a 1 GB VPS and the
laptop share a mind, not a capacity). I offered again to write a direction
note into docs/; the owner has not answered. See
[[self-containment-is-a-standing-contract]] and
[[the-resource-contract-admit-work-settle-curtail]].

THE OWNER'S SECOND EXTENSION (2026-11-11, same ideation): "the relay could be
a two way lane for Armada to connect and control individual instances of
fleet." The relay becomes the C2 transport, not just the phone surface. Still
ideation, no plan.

My framing of the relay leg (proposal, NOT a ruling): the relay is ALREADY a
two-way frame tunnel (the instance dials out, polls inbound frames, pushes
events back), so armada is a new PRIVILEGED PEER on an existing seam, not new
infrastructure. Addressing is already there — each install holds a relay
session id (sid); the flagship addresses an instance by sid. Control frames
(dispatch/state/corpus-sync/version-roll/retire) reuse the SAME bridge verbs
the phone drives, so the multi-session admission gate
([[multi-session-plan-drafted-ki-525-2026-11-11]]) also gates armada dispatch
— the flagship cannot overwhelm a small VPS because the VPS admits for itself.
Split the planes: relay for small latency-sensitive CONTROL frames, git for
the BULK corpus (relay says "sync now", git does the lift). Dial-out gives NAT
traversal for free (VPS behind NAT, laptop on cellular — no inbound ports).
Relay-down degrades to autonomous installs + git sync (the failsafe ruling at
armada scale, [[the-failsafe-restore-fleet-to-working-order]]). THE LOAD-BEARING
NEW DECISION: trust/authority — control frames are powerful (dispatch, roll
versions, retire), so the join-proof must grow into an authority model: an
instance verifies a control frame comes from ITS legitimate flagship (not a
rogue peer on the relay), and the flagship holds authenticated authority over
exactly its armada; the relay can stay a dumb broker. The converging shape:
git-synced corpus (the mind) + flagship-that-is-fleet (the hand) + relay as
two-way control lane (the nervous system), guarded by per-instance admission
and a flagship↔instance trust handshake. NEW OPEN FORK I posed (unanswered):
synchronous control (dispatch and watch the run live, like the face today) vs
control BY INTENT (drop a goal/task, the instance admits and executes on its
own schedule, reports back) — remote-control vs dispatch-board; it decides how
much state the relay carries. See [[a-relay-join-secret-lives-outside-the-dotenv]].

THE OWNER'S THIRD EXTENSION (2026-11-11, same ideation): "the relay connections
are already using device pairing, so identity is somewhat secure, maybe we can
add a 2FA layer to harden it." The owner names pairing as the existing identity
layer and proposes 2FA on top. Still ideation, no plan.

My framing of the trust leg (proposal, NOT a ruling): pairing is stronger than
"somewhat" — it is a cryptographic identity (the device holds the `#k=` pairing
key, the join-proof bootstraps trust, frames are SEALED via X25519 + AES-GCM +
HKDF), giving real confidentiality + channel authentication. What pairing gives
is IDENTITY ("this is device X on a sealed channel"); what it lacks is FRESH
HUMAN AUTHORITY for a powerful action now (a stolen laptop / leaked pairing key
inherits the channel). So 2FA should gate AUTHORITY not identity, and only the
HIGH-AUTHORITY command class (dispatch / version-roll / retire / corpus-push),
not every frame — exactly fleet's always-ask approval philosophy extended across
the relay. Two constraints on WHERE it lives: (1) the instance is HEADLESS (a VPS
has no human), so 2FA happens at the FLAGSHIP where the owner is — the owner
authenticates (TOTP/passkey), the flagship issues high-authority frames carrying
a SIGNED ASSERTION that 2FA was satisfied, the instance verifies the flagship's
signature + freshness; the relay stays a dumb broker that never sees the 2FA.
(2) The primitives already exist (HMAC via sha2/hkdf for TOTP, X25519 for the
signed assertion, the vault for the TOTP seed) — buildable self-contained, no
foreign auth service; challenge-response signed by the owner's key is the TOTP
alternative. Also harden with SCOPED grants (least privilege): a flagship
credential grants control over ITS armada's instances and SPECIFIC command
classes, so a compromised flagship cannot retire an install it was not granted.
The trust stack settles into three layers: relay seals the channel (pairing) →
flagship proves a human authorized the powerful commands (2FA) → instance checks
the flagship's scoped authority before obeying. NEW OPEN FORK I posed
(unanswered): TIME-based 2FA (a TOTP code, simple/offline) vs PUSH-based (an
approval ping to the phone, smoother but needs a live channel) — a push could
ride the existing bot-relay phone pairing, an elegant reuse. See
[[a-relay-join-secret-lives-outside-the-dotenv]] and
[[self-containment-is-a-standing-contract]].

THE OWNER'S FOURTH EXTENSION (2026-11-11, same ideation): "the 2FA also would
prepare for a second growth lane which is multiple users on an armada/fleets
org." The owner names TWO GROWTH LANES: lane one = single-owner armada
(multiple installs, one owner), lane two = MULTI-USER org ("fleets org"), and
2FA is the bridge — per-user 2FA is the identity substrate multi-user needs.
Still ideation, no plan.

My framing of the multi-user leg (proposal, NOT a ruling): multi-user is a
PHASE CHANGE, not a feature — fleet is single-owner at the root (the own-home
ruling, the approval gate, SOUL.md, the contract's "owner's appetite," the
substrate's provenance all assume ONE owner), so many users breaks that
assumption everywhere at once; it is a second epoch. But the primitives lane
one builds ARE the multi-user substrate in embryo, so the sequencing works:
build them once for one owner, generalize to many. What multi-user forces open:
(1) USER identity ≠ DEVICE identity — pairing authenticates a device; multi-user
needs a user account distinct from the machine, with per-user 2FA (the flagship
grows into an org identity provider); (2) authority becomes ROLE-BASED — roles
over the command classes (who dispatches / rolls versions / retires / reads vs
writes the corpus), the scoped-grants idea becomes per-user RBAC hanging on the
same command-class gate 2FA uses; (3) the corpus gets ATTRIBUTION + VISIBILITY —
provenance grows from "which machine/session" to "which user," plus a scope
(private / team / org-wide), and supersede gains an authority dimension (can A
overwrite B's fact?) — a permissions layer on the substrate itself, the hardest
part; (4) credential isolation at org scale — each user's keys from their own
space, never the shared corpus or another user's (own-home becomes sacred);
(5) PER-USER resource quota — the contract learns the machine, multi-user adds
whose run gets the capacity, so one user cannot exhaust the org's fleet (the
multi-session gate extends to multi-user admission); (6) AUDIT becomes essential
— who dispatched/changed/retired what; the calib stream is already an audit
substrate, extended to per-user action logs (hard rule 5 "model-visible ⟺ logged"
grows a sibling "user-action ⟺ logged"). Naming family: fleet = the agent,
armada = your installs, fleets org = the multi-tenant product. NEW OPEN FORK I
posed (unanswered): do users SHARE THE SAME INSTALLS (a team commanding a common
fleet — tenancy on the CONTROL side, a shared control plane with RBAC over common
machines) or does EACH USER OWN THEIR INSTALLS (an org of personal fleets sharing
a corpus — tenancy on the KNOWLEDGE side, federated personal fleets with a shared
mind)? Those diverge hard. See [[self-containment-is-a-standing-contract]] and
[[the-resource-contract-admit-work-settle-curtail]].

THE OWNER'S FIFTH EXTENSION — A RULING (2026-11-11, same ideation): "I see
SOUL.md as a shared artifact of the armada/fleets org, it's what dictates the
behavior of the org. So, realistically it should not be a user setting, a user
can have preferences but the org rules override the user." This is an OWNER
RULING about the org's governance shape (within still-uncommitted ideation):
SOUL.md is the ORG's shared constitution, NOT a per-user setting; user
preferences exist but are SUBORDINATE — org rules override the user. It
resolves the multi-user identity fragmentation: one org character, many users
customizing within it. Consistent with fleet's existing non-negotiable-floor
governance (the always-ask class, the contract floor, the router ruling) and
with [[a-bot-soul-is-its-role-overlay]] (the owner's SOUL as the general
guardrail over the bots' own voices).

My framing of the SOUL leg (proposal, NOT a ruling): the prompt-assembly
layering becomes org SOUL (authoritative base) → org policy → user preferences
(subordinate, bounded) → skills → memory — today's compiled-in base → SOUL →
skills → memory with an authoritative org layer inserted on top and the
personal SOUL demoted to preferences riding UNDER the org rules. A fresh
install inherits the org SOUL with the corpus on deployment; user preferences
stay local to the user's home (own-home holds — preferences personal, never
synced as org authority). THE LOAD-BEARING DISTINCTION I insisted on: A PROMPT
IS NOT A SECURITY BOUNDARY — "org overrides user" is only real if rules split
in two: BEHAVIORAL rules (voice, values, priorities) ride the prompt as SOUL;
HARD rules (never exfiltrate credentials, always gate destructive ops, never
exceed budget) must be enforced in CODE — the same deterministic guards fleet
already has (the destructive gate, the resource contract), synced org-wide —
so a contradictory user preference is REFUSED by the guard, not out-argued in
text; org rules living only in SOUL text make "org overrides user" a polite
fiction (jailbreak/drift overrides them). Two governance consequences: (1)
editing the org SOUL is a HIGH-AUTHORITY command (2FA + org-admin role, the
command-class gate), propagated via corpus sync — users edit only their own
preferences; (2) propagation is clean because fleet reads SOUL PER RUN — a
change reaches the next run on every install, no restart, the constitution
always current fleet-wide. NEW OPEN FORK I posed (unanswered): where is the
line between a HARD org rule (code-enforced, non-negotiable) and a BEHAVIORAL
one (prompt, shapeable) — that line is the org's real policy surface, likely
the first thing an org admin sets; does the org ship a default hard-rule set
(fleet's existing guards as the floor) with admins adding above it? See
[[a-bot-soul-is-its-role-overlay]] and [[self-containment-is-a-standing-contract]].

THE OWNER'S SIXTH EXTENSION — A SECURITY RULING (2026-11-11, same ideation):
"armada owns the org and everything that drives it and controls it, an
unprivileged user should not be able to change that either personally or via
an agent, the boundary must be very clear." An OWNER RULING about the
governance boundary (within still-uncommitted ideation): armada OWNS the org
and its whole control/governance layer; an unprivileged user must not change
it — neither directly NOR through their agent; the boundary must be explicit.
The "or via an agent" is the load-bearing part: it names the CONFUSED-DEPUTY
threat (a user borrowing the agent's privilege, or prompt-injecting the model,
to cross a line they couldn't cross themselves).

My framing of the boundary leg (proposal, NOT a ruling): the boundary must be
ACTOR-BLIND and enforced AT THE ACTION, IN CODE, against the ORIGINATING
USER's authority — it cannot matter whether the request comes from a user
typing or an agent acting; the guard asks only "does the user this session
acts for hold org-admin authority?", the agent's judgment is irrelevant, and
an unprivileged user's agent inherits the user's LACK of privilege and cannot
escalate (this defeats both the direct and the prompt-injection path). Three
things must hold, none in the prompt: (1) the GOVERNANCE LAYER IS IMMUTABLE TO
THE AGENT — the guards, roles, org SOUL, and the authority checks themselves
are outside the agent's writable surface, owned by armada, read-only at most;
if the agent could edit its own guard or grant itself a role the boundary is
theater (fleet's write-guard philosophy taken to its conclusion); they change
only via the privileged control plane (2FA + org-admin, audited); (2) org
governance is an EXPLICIT NAMED CLASS of operations (edit org SOUL, change
policy, modify guards, manage users/roles, deploy/retire installs, roll
versions), each code-gated by org-admin authority — fleet's always-ask CLASS +
structure-guard inventory pattern, not a fuzzy "be careful"; an unprivileged
user/agent touching any gets a NAMED refusal logged as an audit event (an
agent attempting escalation is itself a security signal); (3) the agent's
TOOLS ARE SCOPED to the user's RBAC — an unprivileged user's agent doesn't even
have the governance tools DECLARED (fleet already filters tools per run), so
the boundary holds twice: tool absent + authority guard on any raw attempt.
This is fleet's existing security DNA at org scale (the destructive gate =
code not prompt, the nomenclature write-guard scans whole content ignoring
model intent, the own-home ruling). One sentence: armada owns the governance
layer; it is immutable to every agent; org-governance ops are a named class
gated in code by the originating user's org-admin authority; an unprivileged
user cannot cross it personally or through any agent, because the agent can
only spend the authority its user already holds. NEW OPEN FORK I posed
(unanswered): the AUTHORITY CONTEXT travels with every action (each tool call
/ control frame carries the originating user's identity + grants, verified at
the guard, never asserted by the agent) — does that authority live IN THE
RELAY FRAME itself, signed by the flagship per user, so even a compromised
instance cannot fabricate a privileged call? See
[[self-containment-is-a-standing-contract]] and [[a-bot-soul-is-its-role-overlay]].

THE OWNER'S SEVENTH EXTENSION — A LEAST-PRIVILEGE RULING (2026-11-11, same
ideation): "authority should be scoped at the user level, for projects,
storage, resources, sections of the corpus where the user has access, tools,
skills, workflows, loops. every layer of fleet should have a least privilege
approach to what a person can do." An OWNER RULING (within still-uncommitted
ideation): least privilege is the UNIVERSAL posture, not just the
org-governance class — authority is scoped PER USER across EVERY fleet layer:
projects, storage, resources, corpus sections, tools, skills, workflows,
loops. The governance boundary of the sixth extension becomes the tightest
instance of a general rule.

My framing of the least-privilege leg (proposal, NOT a ruling): THE
ARCHITECTURAL CONSEQUENCE — this must be ONE COHERENT AUTHORIZATION SUBSTRATE,
not eight separate permission systems; ad-hoc per-layer checks leave gaps in
the seams between them, and an agent/user needs only one gap. The clean shape:
a single PER-USER CAPABILITY CONTEXT (grants across every dimension) that
TRAVELS WITH EVERY ACTION and is checked by a uniform guard vocabulary at
every layer; DEFAULT-DENY — anything not explicitly granted is refused. The
actor-blind rule applies uniformly: the agent INHERITS the user's grants at
every layer (tools filtered to tool grants, skills to skill grants, recall to
readable corpus sections, runs to project/resource grants, workflows/loops to
what the user may invoke) and can escalate nowhere. Layer mappings onto seams
fleet already has: RESOURCES → per-user quota (the contract's
admit/work/settle/curtail runs per user within machine capacity; the
multi-session gate [[multi-session-plan-drafted-ki-525-2026-11-11]] is the
primitive, multi-user quota its generalization); CORPUS → section-scoped reads
+ writes (recall filtered to readable sections, supersede checks write
authority — a permissioned mind); TOOLS/SKILLS/WORKFLOWS/LOOPS → declared per
grant, and the LOOPS themselves (self-improvement, calibration, learning) are
GRANTED capabilities, not universal — a user who can't invoke the
self-improvement loop can't propose changes through it. THE SEED ALREADY IN
FLEET: the folder grant ([[runs-refuse-the-filesystem-with-no-folder]], KI-492:
no folder granted = no filesystem access, refused by name) is least privilege
in embryo — the owner is generalizing "no grant = no access" from ONE
capability to EVERY capability. RECURSIVE: the authority to GRANT authority is
the ROOT privilege, scoped tightest — the governance layer that assigns grants
is itself the most-protected surface (immutable to agents, 2FA-gated,
org-admin only); least privilege all the way down with grant-making power at
the bottom of the trust funnel. The org security model now complete: one
per-user capability context, default-deny, uniform guard at every layer; agent
inherits its user's grants everywhere, escalates nowhere; contract/corpus/
tools/skills/workflows/loops all scoped; the power to grant is the root
privilege locked hardest. I offered a THIRD time to capture the whole vision
as a direction note in docs/ (vision doc, not a plan, won't touch the
multi-session cut); the owner has not answered. See
[[runs-refuse-the-filesystem-with-no-folder]],
[[the-resource-contract-admit-work-settle-curtail]], and
[[self-containment-is-a-standing-contract]].

THE OWNER'S EIGHTH EXTENSION — A STEP-UP 2FA RULING (2026-11-11, same
ideation): "destructive grants may need a 2FA check at the user level even if
the user has the grant." An OWNER RULING (within still-uncommitted ideation):
holding a destructive grant is NECESSARY BUT NOT SUFFICIENT — executing the
destructive act demands a FRESH user-level 2FA challenge at the moment of the
action. Authorization (the standing grant) ≠ step-up authentication (fresh
proof now). The bank pattern: logged in and authorized to wire money, but the
wire still triggers a fresh 2FA.

My framing of the step-up leg (proposal, NOT a ruling): fleet already has the
seed in hard rule 7 — destructive ops require "explicit approval IN THE CURRENT
TURN," already a freshness requirement (no pre-approving destruction for later);
this hardens the always-ask class into a step-up challenge at org scale. THE
LOAD-BEARING INSIGHT: THE AGENT CAN NEVER SATISFY THE 2FA — the challenge is
answered by the HUMAN out-of-band (a TOTP code, a push approval to their phone),
never by the agent; the agent can hold the destructive grant and REQUEST the
action, but only the user's live response satisfies it, so even a fully
compromised agent inheriting every destructive grant cannot execute a
destructive act without the human's fresh proof — the confused-deputy defense
taken to its conclusion (the most dangerous actions require a human-in-the-loop
proof the deputy structurally cannot forge). Mechanically the guard checks TWO
things in code, both required: (1) the originating user holds the destructive
grant (the capability context), AND (2) a fresh 2FA assertion scoped to THIS
ACTION exists — signed by the flagship per user, verified at the guard, never
asserted by the agent; per-action (or a very short step-up window), NOT a
blanket "2FA done, destroy freely for an hour" (fleet's "in the current turn" is
that per-action freshness). The destructive class is a NAMED enumerable set
(delete, purge, rewrite, force-push, retire, roll-version, revoke-grants) — the
always-ask pattern, code-enforced. It COMPOSES with least privilege: every layer
scopes grants, and the DESTRUCTIVE SUBSET of each layer's grants demands step-up
(purge a corpus section, retire an install, wipe storage, force-push a project —
all destructive, all step-up, all actor-blind). RECURSIVE: GRANTING someone
destructive authority is ITSELF a destructive act, so it too demands step-up
2FA — the root privilege locked hardest, all the way down. The org security
model is now complete and internally consistent: org SOUL (shared constitution —
prompt for behavior, code for hard rules) + least privilege (one per-user
capability context, default-deny, uniform guard at every layer) + actor-blind
boundary (agent inherits its user's grants everywhere, escalates nowhere,
governance immutable to it) + step-up 2FA (destructive grants demand fresh human
proof per action the agent cannot forge) + audit (every grant check and
destructive attempt logged per user) — every piece growing from a seam fleet
already has. I offered a FOURTH time to capture the whole arc as a direction
note in docs/; the owner has not answered. See
[[the-resource-contract-admit-work-settle-curtail]] and
[[self-containment-is-a-standing-contract]].

THE OWNER'S CAPTURE RULING (2026-11-11, closing the ideation): "capture it
because I see this being implemented sooner rather than later." The whole
arc is now written up as a direction note at
`docs/architecture/2026-11-11-armada-direction.md` — vision, not a plan (no
acceptance criteria); it carries the thesis, the two growth lanes, the three
planes (corpus/git, control/relay, deployment/binary), the reused-seams
table, the five security rulings R1–R5 verbatim, the open forks, and where
lane 1 starts (the multi-session admission primitive). A future session
seeds plans from it. The owner intends implementation soon, so treat this as
an emerging program, not idle speculation — but no lane is committed to a
build until the owner drafts/approves its plan. See
[[multi-session-plan-drafted-ki-525-2026-11-11]] (lane 1's first brick).

THE OWNER'S NINTH EXTENSION — A DESIGN RULING (2026-11-11, after the cuts):
"the way I see armada is that there are components like the security model,
the multi-user aspects of fleet, the grants, the 2FA, getting that planned
right out of the gate would make creating fleet org based approach much
simpler — that's something that we can start doing on the vision document."
An OWNER RULING: develop the foundational components INTO the vision doc NOW
(not wait for a lane plan), because planning them right from the start makes
the org simple. Executed the same turn: the direction note gained **§7 "The
foundational substrate — designed once, the org is a generalization"** (the
old open-forks/where-lane-1-starts sections renumbered §8/§9).

The load-bearing principle (my framing around the ruling): make the
authorization substrate PRINCIPAL-AGNOSTIC from the start — build it in lane
1 with ONE principal (the owner); the org then multiplies principals and
leaves the substrate UNCHANGED, so lane 2 is identity + configuration, not a
security retrofit. The concrete design now in §7: (7.1) a PRINCIPAL (who an
action acts for) distinct from the DEVICE (pairing); an agent acts under its
user's principal, never its own; the principal id travels with every action
and the guard reads THAT, never the actor — the mechanism making R3
actor-blind real. (7.2) a GRANT is a tuple (principal, dimension, scope,
action-class): dimensions = project/storage/resource/corpus/tool/skill/
workflow/loop/governance (eight dimensions of ONE store, not eight permission
systems); scopes = instance/project:<id>/team:<id>/org (lane 1 uses the first
two, the vocabulary exists from the start); action-classes = read/write/
dispatch/destructive/governance (destructive+governance are the step-up
tiers); default-deny; the folder grant KI-492 is the FIRST ROW generalized to
the schema. (7.3) one uniform GUARD at every layer, holding twice for agents
(tool filtered out + authority check). (7.4) the 2FA ASSERTION is a signed
per-action fresh token (principal, action-class, scope, target, issued-at,
nonce), flagship-signed, guard-verified, agent-unforgeable (no key, human
out-of-band), per-action freshness not a blanket window. (7.5) the governance
layer immutable to agents; grant-making is itself a gated step-up governance
action (the root privilege). (7.6) AUDIT: every guard decision per principal
rides the calib stream ("principal-action ⟺ logged"). (7.7) WHAT THIS BUYS THE
ORG: lane 2 reduces to FOUR additions, none a rewrite — a user store +
device→principal mapping, ROLES AS GRANT TEMPLATES (a role = a named bundle of
grants stamped onto a principal), per-user 2FA factors, and the team/org
scopes already in the schema; guard/assertion/audit/governance unchanged.
BUILD-ORDER IMPLICATION: lane 1 builds the substrate WITH the principal
abstraction even though there is one principal — start from the folder grant,
generalize to the schema, add guard+assertion+audit as armada's first pieces,
not a later "security pass." TWO NEW OPEN FORKS added: how much of the
substrate lane 1 carries before the first armada cut (owner lean: the full
thing out of the gate), and flat vs hierarchical/composable roles. See
[[runs-refuse-the-filesystem-with-no-folder]] (the first grant row) and
[[memory-pull-from-other-harnesses-idea-2026-11-11]] (the corpus's feeder).

THE OWNER'S TENTH EXTENSION (2026-11-11, same turn): "also the planning for
the augmented relay functionality for command & control" — develop the relay
C2 into the vision doc concretely too, alongside the substrate. Executed: the
direction note gained **§8 "The augmented relay — command & control"** (open
forks renumbered §9, where-lane-1-starts §10; the doc now reads thesis → lanes
→ planes → seams → capacity → rulings R1–R5 → §7 substrate → §8 relay → forks
→ where to start). The design (my framing around the owner's ruling): (8.1)
the seam — the relay is ALREADY a two-way sealed frame tunnel (dial-out, sid
addressing, join-proof, X25519/AES-GCM); armada is a new privileged peer, not
new transport; dial-out = NAT traversal free. (8.2) flagship peer + instance
REGISTRATION (id↔sid, version, capabilities, RESOURCE PROFILE) + roster +
heartbeat; the profile in the roster is what lets dispatch respect each
instance's admission. (8.3) control VOCABULARY dispatch/state/corpus-sync/
version-roll/retire/health/register, each REUSING the bridge verb the phone
drives, tagged with the §7 action-class; frame = {kind:"control", verb,
instance, principal, assertion?, payload, nonce, issued-at}. (8.4) THE
AUTHORITY TRANSPORT (load-bearing): every control frame carries the principal
+ a fresh 2FA assertion SIGNED BY THE FLAGSHIP, verified at the instance's
guard BEFORE it obeys — so the relay cannot fabricate (no signing key), a
compromised instance cannot escalate (verifies, never trusts), an agent cannot
borrow beyond its principal; the signing key lives at the control plane, never
on an instance the agent can reach. This RESOLVES the old "authority transport"
fork — authority rides IN the frame, signed; the remaining detail is
signing-key custody/rotation/revocation across the roster. (8.5) the relay
stays a DUMB BROKER (routes sealed frames by sid, understands nothing of
armada, never sees a plaintext principal/assertion; all intelligence at the
endpoints). (8.6) two planes — relay for control, git for bulk corpus
(corpus-sync says "sync now", git lifts); relay-down = named degradation
(autonomous runs + git sync), never a stopped fleet. (8.7) lane 1 builds the
flagship peer + roster + vocabulary + authority transport WITH ONE principal;
lane 2 grows the roster/principals, frame format + broker + verification
unchanged. THE THROUGH-LINE connecting §7 and §8: the frame CARRIES the
substrate's authority — principal + assertion flagship-signed, re-checked by
the instance's guard; authority and transport are ONE design built together
from the first cut. Next piece I offered (unanswered): the corpus SYNC layer
(the git feeder/distributor §8.6 hand-waves) tying pull + armada + relay. See
[[a-relay-join-secret-lives-outside-the-dotenv]] and
[[multi-session-plan-drafted-ki-525-2026-11-11]] (per-instance admission).

THE OWNER'S ELEVENTH EXTENSION — A SURFACE RULING (2026-11-11): "also armada
should follow the same patterns as fleet, a rust based app but starting with
a CLI and TUI, then we move to frontend and PWA, and then to a Tauri app." An
OWNER RULING: armada reuses fleet's proven surface progression, NOT a new
stack. Executed: the direction note gained **§9 "The surface progression —
fleet's pattern, reused"** (open forks renumbered §10, where-lane-1-starts
§11). The four stages: (1) Rust core + CLI (`fleet armada` verbs roster/
dispatch/status/sync/register) + ratatui TUI board — terminal-first, script/
SSH-drivable, like fleet slice 1; (2) frontend + PWA — the face gains an
armada section (roster board, dispatch composer, corpus view), installable/
offline/reachable over the relay so the phone commands the fleet like one
instance; (3) Tauri app — fleet-desktop wraps the armada face, bridge+server
reused, no second frontend/transport. WHY reuse: self-containment (hard rule
12, one Rust binary no Node), a de-risked progression (each stage shipped for
fleet once), and the FLAGSHIP-IS-FLEET identity (one codebase, one install —
instance vs flagship differ by ROLE+CONFIG not binary). UX co-equal (hard
rule 11) each stage; the owner's standing surface rulings carry over (centre
conversation + right context pane, borrowed composer, owner's work as view —
see [[a-surface-is-a-conversation-in-the-centre-and-a-context-pane-on-the-right]]
and [[a-chat-composer-is-borrowed-never-rebuilt]]). §9.3 workspace shape:
armada as a MODULE or SIBLING CRATE in the fleet workspace (one implementation,
four surfaces, the bridge seam); NEW OPEN FORK module-vs-sibling-crate added.

THE OWNER'S TWELFTH EXTENSION — PRIOR ART (2026-11-11): "maybe we can use
muxin as inspiration https://github.com/LumaryLabsLLC/muxin". Read the repo
(Lumary Labs, Apache-2.0, Rust/GPUI, muxin.dev — "all your machines, one app,
for you and your AI agents": agent sessions, remote desktops, files, logs,
runners, GitHub across Macs/Linux/Windows, with an MCP server). Executed: the
direction note gained **§10 "Prior art: muxin"** (open forks §11, where-lane-1
§12). THE SYNTHESIS: borrow muxin's ARCHITECTURE, keep fleet's SURFACES (§9,
the owner's Tauri ruling — NOT muxin's GPUI), add what muxin lacks (the shared
corpus, the security substrate, the org). BORROW: (a) the on-machine agent +
persistence (muxen-agent, a small binary per machine, tmux-like sessions that
survive the app closing = armada's instance model + §8.6 degradation made
concrete); (b) agent-first/MCP-first (every capability = MCP tool + --json CLI
+ GUI path, window/CLI/MCP share config state — validates §9.3 one-impl-four-
surfaces; expose the relay verbs as MCP tools so an agent commands the fleet);
(c) connection progression + MUTUAL CERT PINNING (SSH→QUIC, Tailscale roaming,
rebuild-on-network-change — borrow the cert pinning for §8.4 authority
transport so neither end is impersonated); (d) boxes (isolated autonomous
Docker tasks — fleet has LayerFS already); (e) sharing with admission control
+ view/type permissions (a concrete grant pattern for §7); (f) auto-join
provisioning (droplets/VMs join the fleet — the deploy→register→join flow).
CONTRASTS (deliberate): transport relay dial-OUT (no inbound ports, right for
arbitrary NAT instances) vs muxin SSH dial-IN (offer both); surface Tauri vs
GPUI; single-operator vs armada's multi-user org. WHERE ARMADA GOES BEYOND:
muxin manages MACHINES+SESSIONS, armada shares a MIND — the shared corpus
(moves knowledge not just files), the security substrate (muxin's sharing
permissions are a seed, §7 is the full org-ready model), and the org (muxin is
single-operator). ONE-LINE THESIS: muxin proves the fleet-management shape
works; armada adds the shared mind + the org-ready security model on top. The
doc now reads thesis → lanes → planes → seams → capacity → rulings R1–R5 →
§7 substrate → §8 relay → §9 surfaces → §10 muxin → §11 forks → §12 where to
start. Next piece I offered (unanswered): the corpus SYNC layer (the git
feeder/distributor §8.6 hand-waves) tying pull + muxin's file/session movement
+ armada's shared mind. See [[memory-pull-from-other-harnesses-idea-2026-11-11]]
(the corpus's cross-harness feeder) and [[self-containment-is-a-standing-contract]].

THE OWNER'S THIRTEENTH EXTENSION — A COMMUNICATION-LAYER RULING (2026-11-11):
"also use [omni] as inspiration for a communication layer ... we could
implement it on fleet first and extend to armada." (The owner first named
"orbit" github coollabsio/orbit — read it, found a self-hosted Slack/Linear/
Notion workspace, NOT a communication layer; the owner corrected: "sorry,
wrong project I meant omni", https://github.com/automagik-si/omni.) An OWNER
RULING: borrow omni's event-driven omnichannel pattern for a communication
layer, build it on FLEET FIRST then extend to armada. Read omni (automagik-si,
TypeScript/Bun + NATS JetStream + PostgreSQL — "a universal event-driven
omnichannel platform for AI agents to communicate across any messaging
platform"; octopus metaphor: channel plugins = tentacles, NATS = nerve
impulses, core = brain doing identity resolution + routing; every state change
journaled total-ordered in Postgres + published over NATS, the JOURNAL is the
source of truth for replay/tracing/durable consumption). Executed: the
direction note gained **§11 "The communication layer — omni's pattern, fleet
first"** (open forks renumbered §12, where-lane-1-starts §13; doc now thesis →
lanes → planes → seams → capacity → rulings R1–R5 → §7 substrate → §8 relay →
§9 surfaces → §10 muxin → §11 omni communication → §12 forks → §13 where to
start). THE FIVE PIECES TO BORROW (my framing around the ruling): (1) the EVENT
JOURNAL — every state change a total-ordered durable event, source of truth
for replay/causation-tracing/durable consumers = hard rule 5 ("model-visible ⟺
logged") generalized to ALL communication; (2) PUB/SUB transport — durable
named consumers, schema enforcement, causation tracing; (3) CHANNEL PLUGINS
(omnichannel) — unified send/receive behind a Channel SDK (webhook/Slack/
Discord/Telegram), the core never knows the platform; (4) the IDENTITY GRAPH —
resolve the same person across channels into one identity = armada's PRINCIPAL
model (§7.1) at the communication edge, so messaging and authority share one
resolution; (5) AGENT DISPATCH — provider bindings (webhook/claude-code/a2a),
response gates, debouncing, rate limits, session strategies = armada's control
dispatch (§8.3) where a response gate = the approval seam, a rate limit = the
resource contract, a session strategy = the run scope. FLEET ALREADY HAS THE
SEEDS (so "fleet first" is a CONSOLIDATION not a new subsystem): src/channels/
(the plugin edge, webhook today — the uncommitted work this session touched
channels/mod.rs + webhook.rs), the bridge event stream (the SSE pub/sub seam),
the session JSONL log + calib stream (two journals to unify into one
total-ordered event journal), the relay (a channel plugin in embryo). EXTEND
TO ARMADA: the relay (§8) is the transport, omni's pattern the model riding it
— the identity graph resolves principals across instances/channels, the
journal federates across the roster so the flagship traces causation across
machines (dispatch on A → run on B → corpus commit on C), agent dispatch is the
control vocabulary with omni's gates, A2A routing = instance-to-instance
dispatch. SELF-CONTAINMENT CONTRAST (hard rule 12): omni is Bun/TypeScript +
NATS JetStream + PostgreSQL; fleet borrows the ARCHITECTURE not the STACK — the
journal is fleet's existing JSONL/SQLite (no Postgres), the bus an embedded
Rust pub/sub (no NATS broker to run), the channels Rust plugins (no Bun); one
binary, no foreign runtime. BUILD ORDER: the communication layer is a
fleet-first candidate (the seeds exist) and the piece that turns armada from a
command plane into a COMMUNICATING fleet; it COMPOSES with the substrate (§7,
identity = principal) and the relay (§8, transport), replacing neither. Next
piece I offered (unanswered): the corpus SYNC layer (the git feeder/distributor
§8.6 hand-waves) tying pull + omni's journal + armada's shared mind. See
[[memory-pull-from-other-harnesses-idea-2026-11-11]],
[[self-containment-is-a-standing-contract]], and
[[a-relay-join-secret-lives-outside-the-dotenv]].

THE OWNER'S FOURTEENTH EXTENSION — THE ORG TEAM-WORKSPACE RULING (2026-11-11):
"now multi-user would eventually demand something like orbit as inspiration,
because it's easy to work by yourself but teams need special tools.
https://github.com/coollabsio/orbit". An OWNER RULING: lane 2 (the multi-user
org) needs a TEAM WORKSPACE, and orbit (coollabsio, Apache-2.0, Rust+React,
orbit.so — "a self-hosted workspace for teams: tasks, docs, chat, and mail in
one app," the open-source Slack/Linear/Notion alternative) shows the tool set
teams reach for. NOTE: this is the DELIBERATE orbit citation — distinct from
the thirteenth extension's accidental "orbit" that was corrected to omni;
orbit = the team-workspace inspiration (lane 2), omni = the communication
layer (fleet first). Executed: the direction note gained **§12 "The team
workspace — orbit's inspiration for the org"** (open forks renumbered §13,
where-lane-1-starts §14; doc now thesis → lanes → planes → seams → capacity →
rulings R1–R5 → §7 substrate → §8 relay → §9 surfaces → §10 muxin → §11 omni
communication → §12 orbit team workspace → §13 forks → §14 where to start).
THE DESIGN (my framing around the ruling): (12.1) the TEAM-TOOL LAYER — orbit's
four surfaces each compose with a piece already designed: TASKS (workflows/
board/timeline/sub-issues/GitHub sync) → a task becomes a dispatch (§8.3),
fleet's goal+huddle machinery generalizes from one owner's goals to a team's
board; DOCS (page tree/collaborative editing/version history) → the
human-facing view of the SHARED CORPUS, editing a doc writes back with the
principal's provenance (§7.1); CHAT (channels/DMs/threads) → carried by the
OMNI communication layer (§11), identity graph resolves who said what; MAIL
(shared inbox) → the omnichannel edge turned toward email. (12.2) THE KEY
FINDING — orbit's ADMIN LAYER VALIDATES THE SUBSTRATE, mapping almost 1:1 onto
§7: roles→roles-as-grant-templates (§7.7), audit log→per-principal audit
(§7.6), rate limits→the contract's per-user quota (§5/§7.2), API tokens→a
principal's credentials (§7.1 device→principal), workspace export→the
git-synced corpus (§3), MCP+OAuth→agent-first surface (§11) + authority
transport (§8.4). Independent confirmation the substrate is the right shape:
a team workspace NEEDS exactly roles/audit/rate-limits/tokens, which §7
designs once out of the gate — so lane 2 inherits its admin layer WORKING, not
a permissions retrofit after the collaboration tools exist. (12.3) THE ORG IS
FOUR LAYERS COMPOSING, not a monolith: substrate (§7) + communication (§11) +
shared corpus (§3+pull) + team-workspace surfaces (orbit's shape on fleet's
Tauri/PWA stack §9). (12.4) CONTRAST: orbit is standalone Rust+React+PostgreSQL
Docker-deployed; armada borrows the TOOL SHAPE not the STACK (hard rule 12,
self-contained one install). LANE-2 THESIS: the org succeeds only if it is a
workspace teams WANT to work in, not a control plane they tolerate — agents
are first-class citizens (a task dispatches to an instance, a doc is written
by a human OR agent principal, the chat carries agent events), all on the
substrate that makes it safe; the substrate built in lane 1 is what lets lane
2 be a team workspace instead of a security retrofit. The three prior-art
pillars now compose: muxin = fleet management, omni = communication, orbit =
team workspace, each with the substrate + relay rather than replacing them.
Next piece I offered (unanswered): the corpus SYNC layer. See
[[memory-pull-from-other-harnesses-idea-2026-11-11]] (the corpus behind docs),
[[self-containment-is-a-standing-contract]], and
[[a-huddle-is-a-knowledge-space]] (fleet's team-knowledge seed).

THE OWNER'S FIFTEENTH EXTENSION — A FLEET-FIRST REFACTORING RULING (2026-11-11):
"a slimmed down version of orbit could serve as a refactoring of the whole
bots and desk experience on fleet, each work type has a surface but no
connective tissue" + "you could easily invoke bots from an orbit like user
experience". An OWNER PRODUCT INSIGHT: orbit's workspace pattern is not only
lane 2's future — a SLIMMED-DOWN orbit (tasks+docs+chat, mail/heavy-PM dropped)
could refactor fleet's CURRENT bots-and-desk experience, because today each
work type has a surface but NO CONNECTIVE TISSUE (the desk has deliverables/
lessons, the conversation has chat, the bots have huddles, learning has
lessons — silos; nothing ties a task to its conversation to its deliverable
to the bot that did it to the lesson it taught). Executed: the direction note
gained **§12.5 "The fleet-first refactoring — the workspace as connective
tissue"** (under the orbit §12; section numbering unchanged, forks still §13,
where-to-start §14). THE DESIGN (my framing around the ruling): the workspace
becomes the WORK-OBJECT GRAPH — task ↔ thread ↔ deliverable ↔ bot ↔ lesson —
and the existing surfaces become VIEWS OVER THE GRAPH rather than separate
rooms; the slimmed-down orbit is the spine the desk/conversation/huddles hang
off. BOT INVOCATION BECOMES A WORKSPACE ACTION, NOT A CEREMONY (today a bot is
summoned via a separate huddle convocation or the bot relay): (a) ASSIGN A
TASK TO A BOT — the assignee is a persona, assigning dispatches the task
(bridge.run under the role overlay), the bot's work lands as a deliverable
LINKED TO THE TASK with working/blocked/done status (omni's session hooks
§11); (b) @MENTION A BOT IN A THREAD — the huddle pattern made contextual, the
bot summoned INTO the conversation, replies in-thread, work stays connected;
(c) CO-AUTHOR A DOC — a bot writes a section into the corpus with provenance
(the bot's role acting under the user's principal). REUSES fleet's machinery
not rebuilds: src/roles.rs (personas), the huddle (convocation), bridge.run
(dispatch), the role overlay — the workspace makes invocation contextual and
results connected. The SUBSTRATE governs it: a principal invokes a bot only
if granted (§7.2), the bot acts under the user's principal (§7.1, the bot is
the deputy), a destructive bot task hits the step-up gate (§7.4). WHY
FLEET-FIRST: the connective tissue is valuable for ONE owner now AND is the
SAME workspace the org grows into (§12.3) — the graph built for a single
owner is the team workspace later, not a rewrite; the refactoring PAYS FOR
ITSELF TWICE (connects fleet's siloed surfaces today, lays the org's
collaboration floor tomorrow). See [[desk-learning-project-surfaces-drafted-2026-11-09]]
(the desk's current work-column shape), [[huddle-surface-ideation-2026-11-03]]
(the Slack-shaped huddle seed), [[a-chat-composer-is-borrowed-never-rebuilt]],
and [[a-huddle-is-a-knowledge-space]].

STATE UPDATE (2026-11-11): the direction note (§1–§14 incl. §7 substrate, §8
relay, §9 surfaces, §10 muxin, §11 omni, §12 orbit + §12.5 fleet-first
refactoring) was COMMITTED at 9d9b6de (with the identity-first README) and
PUSHED to origin/master — no longer uncommitted. The cross-harness pull (the
corpus's feeder, §3/§4 two-feeders note) is BUILT + verified + committed at
2907d19 (see [[memory-pull-from-other-harnesses-idea-2026-11-11]]). The README
was then reframed benefit-led per the owner's why+payoff ruling
([[readme-leads-with-why-and-benefit-not-your-keys]]); that reframe is
uncommitted. The corpus SYNC layer (the git feeder/distributor §8.6 hand-waves)
remains the next offered-but-unanswered piece. The vision doc is still a
VISION (no lane committed to a build until the owner drafts/approves its plan).
