---
name: layerfs-expert-study-2026-10-25
description: "2026-10-25 owner asked to become an expert on LayerFS (Ephemeral-AI-Lab/layerfs) — full repo studied at /tmp/layerfs-study; the model, architecture, receipts, and the DeepSeek-Harness-lineage connection to fleet"
type: reference
date: 2026-11-05
basis: confirmed-by
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

Owner ask 2026-10-25: "become an expert on LayerFS"
(https://github.com/Ephemeral-AI-Lab/layerfs). Cloned shallow at
`/tmp/layerfs-study` (v1.1.0-era, MIT, source-only — nothing published to
npm) and read end to end: README, concepts, the 14-crate core architecture
papers, the JS SDK/CLI surface, the Store concurrency model, release
records, benchmarks, and both plugins.

**What it is:** "Ephemeral Workspaces. Durable Shared History" — the
filesystem substrate under agents. Each agent mounts a complete writable
Workspace over shared committed history, works, commits what changed, and
goes away.

**The model:** one SQLite Store file (LayerStacks → immutable Layers;
Branches → immutable Commits; globally-deduped content-addressed canonical
objects, append-only). A Workspace is an ephemeral writable view of one
Branch head = immutable committed base (whole tree incl. `.git`,
`node_modules`, ignored files — instant bootstrap) + its own per-mount
overlay SQLite file (template-copied at mount, deleted at unmount).
Lifecycle explicit: Create → Exec → Commit (CAS on the Branch head) → End;
Clean End refuses uncommitted state, Discard abandons it, End NEVER
commits implicitly. Two Commits changing one file are refused `Conflict`
unless a `.lww` rule says newest-wins. Projections: Materialize (host
dir) or FUSE. Execution: Docker Sandboxes, one daemon each, ≤16
Workspaces per Sandbox; the daemon supervises no commands.

**Storage receipts:** 19.9× smaller than Btrfs+zstd over 157 snapshots
(100 MB vs 1,994 MB); real dsh run 8.90 GB for 4.93 TB of history (166×);
~5.1 KB per Commit fixed cost — commit COUNT, not bytes, drives storage;
piece-compare on write (identical bytes → no Commit), whole-file deltas
(~88% of records at ~4.4% size), CDC chunks ≥128 KiB. F-027 (decided)
cuts the fixed part ~41%.

**Architecture:** core/ = 14 Rust crates (~96k production lines) in two
clusters — one: content/storage/history/persistence/project/telemetry;
two: overlay/workspace/fuse/daemon/store-service/api/bridge/sandbox.
Store concurrency is a binding owner decision: ONE global database, ONE
writer at a time, short holds (only head-check/merge/publish holds the
lane), no retry — one attempt with exact failure custody. JS API 1.1.0:
`@layerfs/agent` (29 calls: fs/branch/sandbox/workspace; positional args
only, LAYERFS_* env settings, an options object is refused),
`@layerfs/backend` (read-only history/storage + the LAYERFS_TRACE
call-trace JSONL), `@layerfs/runtime` (hosts: Init, the macOS/Windows
worker, cleanup), plus the `layerfs` CLI (one SDK call per command, for
Bash-only agents). Store calls are in-process Linux-only; macOS rides a
Linux worker container, Windows a WSL2 worker.

**Honest limits:** Disposable profile ONLY (no crash/power-loss
durability — Durable execution disabled by owner until reauthorization);
no FIFOs/sockets/devices/setid in the FUSE view; slower sequential I/O
(fio 0.31× seq write); single-writer by design; 1.1.0 broke the 1.0.0 SDK
under an explicit recorded owner waiver; the release shipped with waived
gates (NOT_RUN Linux FUSE tests).

**The fleet connection — why this matters to us:** LayerFS × DSH
(dsh-layerfs, its own repo) is a plugin for **DeepSeek Harness** — the
project fleet descends from (see [[fleet-origin-lineage]]). It adds a
"minimalist" preset (bash + subagents, NO read/write/edit tools — every
file op goes through LayerFS), Work/History tabs drawing each agent's
Branches live, and ten scripted no-LLM demo scenarios. The philosophy is
kin to fleet's own: per-tool-call checkpointing is fleet's session-JSONL
instinct with the FILESYSTEM as the log; its Store model (one writer,
exact-conflict refusal, no-retry custody) mirrors fleet's one-writer/
audit-trail constitution; its engineering culture (source-pinned docs
updated in the same commit, PASS/NOT_RUN/FAIL honesty, owner waivers
recorded, per-commit production-LOC accounting) is visibly the same
school as fleet's gate/ledger discipline. A magpie-layerfs plugin also
exists (MCP tools layerfs_provision/checkpoint/fork/publish/exec/history/
resolve/status).

**Owner verdict 2026-10-25 (adopted, not just studied):** "it's worth
using it because we have had several instances of agents stepping into
the code" — the dogfooding co-edit incidents ([[shared-file-commits-can-
sweep-another-sessions-work]], [[fleet-deck-two-sessions-collide]]) are
the named pain LayerFS fixes structurally: one Workspace per agent, CAS
Commit, named `Conflict` instead of a silent overwrite. The owner's shape
for getting there: "the deepseek harness version should be easy to
create as a plugin." My correction, accepted in the exchange: the
dsh-layerfs plugin is TypeScript for the TS harness and fleet's hard
rules forbid Node/dsh launch paths, so the CODE cannot be ported — but
the PATTERN is small and copyable (it changes no harness code: an
env-setup host, a persona cheat-sheet "one Workspace per agent, only the
main agent publishes, unmount everything", a records reader for the UI),
and fleet would drive LayerFS the way it drives the vendored lanes: an
external tool with a pinned version via the `layerfs` CLI, zero new
crates. Two costs the owner must rule on before a plan: (1) Docker is
mandatory (Sandboxes are containers on every platform; macOS adds the
Linux worker container for Store calls) — the heaviest dependency fleet
has carried, so it must be a per-project opt-in ("this checkout uses
LayerFS workspaces") or it brushes the anti-Hermes constitution
([[fleet-anti-hermes-constitution]]); (2) LayerFS is Disposable-only, so
git stays the durable spine — LayerFS is the collision-avoidance layer,
real commits land in git on publish, and it never becomes the only copy
of anything. Recommended starting point: delegation children spawn with
a Workspace each and the parent merges published Branches — smallest
blast radius on the dogfooding workflow, and the exact shape of the
stepping-on-toes incidents. Next step offered: a draft plan at
docs/plans/ (house protocol) covering the opt-in boundary, the tool
surface, and the macOS worker lifecycle — awaiting the owner's go.

**Plan drafted and committed 2026-10-25 (`351a4a6`, status: draft):**
`docs/plans/2026-10-25-layerfs-delegated-workspaces.md`. Shape: C0 is
E0's probe gate (`scripts/layerfs-probe.sh` measures this Mac's Docker,
init→mount→exec→commit→unmount latency, and the macOS worker's
lifecycle before any driver parses a real answer — the sys1-extraction
plan's E0-first discipline, which survived that leg's Fastino rebuild);
C1 a thin `src/layerfs_tools.rs` CLI-spawn driver (the mcp.rs shape,
once-per-process availability gate, no Node in fleet's build); C2 the
delegation arm — one Workspace per child at spawn, child commits to its
own Branch, only the parent merges/publishes, `Conflict` is a first-class
answer, Branch/Workspace ids in the delegation calibration row; C3
opt-in by naming a project, byte-identical default off, doctor probe,
wire-version refusal; C4 close-out. The seam that made it cheap:
`FileAccess::Folder` already exists — the child's tools just run at the
mount path. Per-delegation cadence, not dsh's per-tool-call one.
Out of scope: the parent's own session, replacing git, Work/History
tabs, Windows. A lesson from the commit itself: I put a "Production
LOC: 145417 -> 145417" line in the first commit message — a number I
never measured, in a convention this repo does not use (that's
LayerFS's own AGENTS.md rule, absorbed during the study and misapplied)
— and amended it out (`5e17e0a` → `351a4a6`). Fleet commit messages
carry no LOC accounting; don't import another repo's conventions while
fresh in them.

**README attribution landed (`22d09ed`):** the owner asked for "the
proper attribution to layerfs on the readme" — it went under "What is
next" (where the DeepSeek Harness lineage is already credited inline in
"The arc"), NOT "What ships today", because the plan is a draft: LayerFS
named and linked with its MIT license, the shape stated (external tool
reached through its `layerfs` command, nothing vendored, no runtime in
the binary — also the license-clean posture), and a link to the plan so
credit and contract point at each other. The placement rule for the
next attribution-like addition: unbuilt work gets named where next
things are named, and the README never claims a feature before its
increment ships.

**Implemented same day (`0ab087c`; plan → active, C1/C2/C3/C4 done):**
the driver (`src/layerfs_tools.rs` — CLI spawn, once-per-process gate,
1.1.0 wire refusal, Conflict-as-answer), the arm (`src/subagents.rs` —
mount at spawn, `FileAccess::Folder` at the mount, settle at every exit,
merge `main ← child` + publish as the parent's act, Conflict with paths
in the tool result, branch/Workspace ids on the delegation row), doctor
line + settings footer, `scripts/layerfs-probe.sh`. Full gate green by
exit code (0/0/0/0; 1703 passed, 0 failed, 2 ignored). C0's live probe
is blocked and NAMED: the ladder stops at the CLI rung (docker 29.7.2
✓, node v24 ✓, the addon/CLI/worker-bundle/image never built here) —
KI-313 carries the provision steps, and C2's live run + C3's checkout
pin wait on it. Two test-harness bugs of the arm's own died in the
gate: see [[a-test-that-passes-on-an-idle-machine-has-not-been-tested]]
forms 11–12 (a stub deleted under a cached path poisoned the shared env
lock; the knob as process-env flipped a concurrent test's delegation
onto the arm — fixed as a per-thread override).

**KI-313 lifted the same day (`aba4fa6`; plan → completed, 5/5
criteria).** Provision chain (recorded in CLAUDE.local + the plan's C0):
pinned checkout `/tmp/layerfs-study@ad78914` + npm workspaces; the
sealed Linux bundle via `tools/worktree.py register/prepare/build
--platform linux --profile release`
(`build-linux-25b0bed7197842688bed6e1364c9b25e` — daemon + linux
`layerfs.node` + `worker/server.js` + CLI, 182 hashed files);
runtime image `sha256:3d27e5c11e57…` (node:24.21.0-bookworm); worker +
`start_init` via a three-line `@layerfs/runtime` script (init 406ms,
volume created); `LAYERFS_STORE_PATH` =
`/var/lib/docker/volumes/<volume>/_data/global/store.sqlite` (the
worker-visible path — dsh's host does the same). Probe exit 0 with the
whole ladder: fork 55ms, sandbox 288ms, mount 62ms, exec 104ms,
commit 63ms, two clients 60ms concurrent, **idle exit 121,225ms** (the
two-minute self-end observed), **reclaim 391ms reusing the number**.
C2's live roundtrip passed (open 580ms, settle 361ms, file read back
from main). **The live run earned its keep twice:** (1) the arm's
`fleet/<session>` branch names violate the wire's `HistoryName`
grammar (slash + uppercase stamp) — every real delegation would have
refused at first fork while every stub went green; fixed by
`child_branch_name` fold + pre-spawn `history_name_ok` (KI-315, closed
same day) — **a stub that accepts what the wire refuses launders an
invalid value to green; one live run is the only thing that catches
that class.** (2) the mount is sandbox-local (host meets ENOENT; the
in-sandbox exec reads/writes the same path) → the child-execution seam
is the named next increment, **KI-314 open** (superseded by the owner's
closing ruling below): design it against dsh's pattern (file ops via
`layerfs fs.*`, commands via `workspace exec`), never wire children at
the mount path until it exists. Final gate 1706 passed / 0 failed /
3 ignored (fmt/clippy/build/test all 0).

**Owner ruling the same evening (`f3f6b67`): "the layerfs work is done,
no longer a blocker."** The ledger now says what the tree showed: KI-313
closed with aba4fa6's provision record (the row's ask — create
credentials and provision — had been answered, so it stops asking the
owner for a key), and KI-314 reclassified from a blocker-shaped gap to
**deferred by design as the next increment** — it was never a blocker to
the arm: the child's host-side processes work at the checkout's own
paths, the mount stays sandbox-local, and the seam (dsh's `fs.*` host
writes + `workspace exec`, both measured at 60ms/104ms) is the named
target. The plan's close-out carries the same words, so the two
artifacts agree. With this, the ledger's only genuinely open row is
KI-249 (the embed-timeout bounds await the first live run's
`embed_timing` rows — zero recorded as of 2026-10-25).

**KI-314 closed the same day — the child-execution seam
(`f11d2a8`; plan `2026-10-25-layerfs-child-execution-seam.md`
completed C0-C4).** Three wire findings reconciled BEFORE coding (a
fresh mount sees the branch head; a HELD view pins its base; command
writes publish only through Commit) — which made dsh's cadence the
only coherent shape: **mount → exec → commit → unmount per bash call
(measured live: 267 ms for a trivial command)**, file ops through the
SDK's branch verbs with NO mount at all. The design: a third
`FileAccess::LayerFs` (project/branch/sandbox, set only by the opted
arm) routes the fs-class through `Tool::execute_in_layerfs` — the
sibling of the host wrappers with the same `bound_with_spill` —
`edit` reuses a behavior-preserving `fileops::edit_text` extraction
so its whole contract (closest line, ambiguity's lines, diff) is one
implementation, and no-route tools (apply_patch, repo_map,
file_change, vision, background bash) answer NAMED refusals. Three
wire-forced disclosures ship as footers, never silence: branch grep
is LITERAL and case-sensitive (no regex engine) with dropped options
named; branch glob has no ignore filtering and no mtimes; read's
totals header only when the window reaches the end (the wire carries
no line totals). Live run re-asserted all three findings on the wire
and walked the child workflow end to end (write → command reads its
own file off a fresh mount → read sees the checkpoint → parent merges
→ `fs read main` proves publication). **Co-edited-tree note:** the
final gate ran over the STAGED tree in a detached worktree (0/0/0/0,
1712/0/4) because the shared worktree carried the other session's
in-flight corpus edits that turned one corpus-pin test red (their
HEAD passes it — proven; owner confirmed their fix done; neither
memory.rs nor memories/ was touched here). Also that session filed
KI-316/317/318 meanwhile — the ledger keeps moving.

**Owner: "far from done… not a seamless experience" (2026-10-25) —
the EXPERIENCE half is the open work.** Two completed capability plans
≠ a usable feature. The exact seam gaps I named back and the owner
confirmed: (1) **seven manual exports** to launch a run
(LAYERFS_HOME/WORKER_ENDPOINT/IMAGE/DAEMON/STORE_PATH,
FLEET_LAYERFS_BIN, FLEET_LAYERFS_PROJECT) — of these at most two facts
are irreducible (which project; where the bundle is); endpoint derives
from home, store path from the project's volume, daemon from the
bundle, image from the deployment — **fleet should compose the CLI's
env per spawn from its own home, owner sets one fact (the project)**;
(2) **doctor is blind to six of the seven** — its `layerfs` line checks
CLI/wire/pin/knob but not endpoint/image/daemon/store path, so a wrong
one passes green and fails the first delegation; (3) the **checkout +
CLI live in `/tmp`** (macOS purges; needs a durable home, only
FLEET_LAYERFS_BIN re-pointed since the bundle hashes are
path-independent) and the LayerFS home is parked inside the repo's
`tmp/`, coupling worker mounts to the repo path; (4) the knob points
at the **scratch project** — real use needs a project init'd from the
actual repo, without the owner knowing `start_init` exists; (5) the
**reboot story is assumed, not owned** (worker `unless-stopped` rides
Docker autostart, nothing verifies the endpoint after a restart —
first delegation post-reboot is the untested case). The
default-off boundary still protects non-opted-in runs, but the
standing anti-Hermes test ([[fleet-anti-hermes-constitution]]) — "does
the owner need to know this knob exists?" — fails the moment the
feature turns on: seven knobs, a memory file (CLAUDE.local), and a
`/tmp` that forgets. **Done looks like:** fleet resolves everything
from `<FLEET_HOME>` + one named project, composes the env per spawn so
a bare `fleet run` works with zero shell setup, doctor checks all
seven facts naming what's missing and its fix (including "the worker
is down, start Docker"), durable homes, one command to init the
project you're standing in, survives a reboot. This is a third
increment — the productization layer over two finished capability
plans — offered as a plan; if sequencing one seam first, **doctor's
blind spot is the one that turns silent first-call failures into named
ones**. Lesson for any future feature: capability-complete behind a
default-off boundary still ships the seam to whoever opts in — count
the env ceremony, the doctor coverage, the durability of every path,
and the reboot path as acceptance criteria, not follow-ups.

**The owner refined the ask mid-turn — the plan's real title is
"seamless, no manual steps, new AND existing projects" (`5e52823`,
status: draft):** after I offered the third increment they said
"what I need is plan to seamlessly use layerfs on new and existing
projects, no manual steps on my part" — and separately asked for the
**visualization** ("we also need to have the visualization part of
layerfs to show in fleet").
`docs/plans/2026-10-25-layerfs-seamless.md`, five criteria: C0
measures a REAL repo's init cost first (wall time, counts, volume
size, **what the import swallows** — a `target/` taken in blindly
would be the opposite of seamless, so the bound comes from the number
not a guess); C1 the driver composes every LAYERFS_* value per spawn
from `<FLEET_HOME>` facts + a registration, CLI runs from the sealed
bundle (`bundle/api/agent/cli` ships node_modules, so the `/tmp`
checkout retires from the runtime path entirely); C2 folder→project
auto-registration (first delegation in any folder registers once,
hit costs nothing, **over-bound folders fall back to today's default
with one named notice — never a blind import**); C3
`fleet install --layerfs` as the ONE machine step (the `--lanes`
precedent) + doctor growing to all seven facts with fixes;
C4 the proof: a live delegation with **zero LAYERFS_* in the
process env** on a fresh folder. The viz's measurements are banked:
branch rows give `base/head/forkedFrom`, commit rows give
`parent/secondParent/via` (full DAG), **but `list_commits main`
answers `NoSuchBranch`** — main's trunk is not CLI-reachable, so the
History tree's center line must come from `@layerfs/backend` (ships
inside the sealed bundle). Owner's sequencing: seamless FIRST, face
after.

**Then the owner put the viz IN the plan (`b7a3298`): "visualizations
from the original layerfs need to be part of the plan as well"** —
dsh-layerfs's two tabs are in scope, not a follow-up. The plan
retitled "…plus the original's Work and History face" and grown to
seven criteria: C4 the face's reads (CLI tree reads + a backend
reader — `node <bundle>/api/...` through the same composed env — for
`get_history`/`get_storage`, shaped into one view model; the
child↔branch join via the `fleet-child-<session>` fold; a
backend-script failure = a NAMED ABSENCE on the view, never an empty
lie); C5 the face itself (History = the DAG with main's trunk,
branches at fork points, commits with merge links and messages;
Work = per-child lanes against the tree replayed with the session,
plus sandbox/workspace state from `list_sandboxes`/
`list_workspaces`; a Files browser over `fs list`/`fs read`;
Storage from `get_storage`'s logical/canonical/stored + dedup
ratios; genuinely visual — an actual graph, not a table — with
`surfaces.md`'s twelfth destination and FUNCTIONAL's surface list
shipping IN the increment, per the docs gate); C6 renumbered
close-out (now also proves the destination renders the same
clean-environment run, endpoint verified by curl against the live
face). Named OUT with reasons: live process streaming/terminal
panes (the backend process API — a later increment; v1 shows
recorded lanes + current state) and the magpie trajectory-style
bough rendering (dsh's tabs are the reference the owner named).
Work's data is nearly free from fleet's OWN records — dsh needed
extra machinery for the command rows fleet's session log already
carries. Two design calls still flagged for the owner: C3's
install-as-the-only-step, and C2's over-bound named-fallback.

**The owner's provider rule joins the plan (`43fef21`): "if at any
point in time layerfs needs support from the existing lanes, that they
are used as providers… We don't build other if we already have a
solution for it."** The lanes ARE the providers for every capability
the seam or the face might need — **sem1** embeddings/vector matching
(never a new embedder or second vector path), **sys1**
classification/judgment (labeling what a child's lane did),
**sdm1** tabular scoring (attention ranking, figure tables), **gen1**
media, **blend1** composed grounded answers ("what changed and why") —
each invoked the way fleet already invokes them (lane registry, MCP,
existing per-tool economics), never a bespoke client. Placement for
the gate: the Approach carries the full mapping; **C4 and C5 each
carry the check** where semantic creep would tempt (matching/ranking/
labeling in the reads; judgment/semantic/composed in the render —
each must NAME its lane as the provider instead of growing a
model-free imitation); out-of-scope forbids new provider machinery
flatly (embedders, classifiers, vector stores, search engines,
explainers — a capability no lane carries is named unavailable and
queued for a lane decision, never built as a one-off); linked to the
extraction-leg plan as the precedent (its second-vendor ruling, the
KI-315-class verdict: what's wrongly built as a second provider for a
capability the pool carries should not stay). Stated honestly in the
plan: nothing in the current design needs a lane YET (reads are
LayerFS's own API, the view model is deterministic assembly) — the
rule binds the increments as they grow, which is exactly why it sits
in C4/C5 rather than only in prose. The generalizable form for future
plans: when a feature needs intelligence (classify, embed, rank,
generate, explain), the design step lists WHICH LANE provides it;
a plan that adds provider machinery without such a line is missing a
criterion.

**"Integral, not a bolt on — and its own section under knowledge"
(`49d00d8`), then the name (`dad33e4`).** Two more qualifiers on the
seamless plan: (a) **"it needs to feel as an integral part of fleet and
not as a bolt on or an afterthought"** — graded by placement, not
sentiment: the default behavior IS fleet's (isolated delegation when
the machine can hold one, one machine-property notice when it cannot —
the lanes' absence voice, not a mode the owner turns on); the EXISTING
surfaces carry the state (Delegation destination rows grow each child's
branch + merge status with a click-through into the face; the chat
subagent card already shows the workspace line — the face is where you
see the tree, the seams are where you notice it, so nothing lives only
on a page the owner must remember to visit); fleet's vocabulary is
primary (lanes labeled by child goal and session, the `fleet-child-…`
fold decodes the branch SECOND; LayerFS nouns appear as substrate terms
the way fleet mentions MCP or SQLite — how it works, not what it is);
docs tell fleet's story (the README delegation bullet and FUNCTIONAL's
delegation surface grow isolation as a property of fleet in the lanes'
voice — never a section titled "LayerFS integration"; the HOW gets
`docs/features/<name>.md` beside its siblings; install/doctor present
it as one more layer of the machine in the roster's table/voice); and
**C3 reframed: `fleet install` provisions it as a STANDARD phase of
the machine's setup, not an opt-in feature flag** (skip-by-name when a
prerequisite is missing, `--lanes` voice, C0's numbers set what it
builds unattended). (b) **"it should be its own section under
knowledge"** — the face is the FOURTH ENTRY UNDER the GUI's existing
`knowledge` group (`memory · skills · second-brain · …`), the nav's
four-groups-one-level shape untouched, NOT a fifth top-level
destination bolted beside chat; `surfaces.md` gains `### <name> —
*knowledge*` beside its siblings' sections; the feature doc lives
under docs/features/. The two qualifiers compose: on the WORK
surfaces the property is integral (nobody hears the word LayerFS while
experiencing isolation), under KNOWLEDGE the substrate is properly
housed and named like every subsystem.

**The name: `workspace history` (route `workspace-history`), decided
by `fleet convention` not taste.** The owner offered "shared history"
or "workspace history"; the house naming law answered: `workspace` is
ESTABLISHED (15 identifier files, 10 docs — the seam's landed code
among them) and `history` too (the boards already read it), while
`shared history` returns *no established term, nothing near it* — two
fresh coins — AND it is the vendor's own tagline ("Durable Shared
History") in fleet's mouth, which the vocabulary rule exists to
prevent. So the knowledge-group entry and route are
`workspace history` / `workspace-history`, `surfaces.md`'s section is
`### Workspace history — *knowledge*`, the feature doc is
`docs/features/workspace-history.md`, and **LayerFS stays named only
where it IS the substrate** (the CLI, the knob, the install phase,
Branch/Commit/Layer inside the section where precision needs it).
Process worth reusing: when the owner offers two names, run
`fleet convention "<candidate>"` on both before picking — the
nomenclature lookup (`src/nomenclature.rs`, lexical-only on purpose:
semantic nearest = synonym = the drift it exists to catch) gives an
established-or-not verdict, and the vendor-tagline smell is a
disqualifier. The plan file keeps the slug `layerfs-seamless` (plan
artifacts are about the tech; only the SURFACE took the new name).

**State of the contract after `dad33e4` (status: draft, seven
criteria C0–C6):** the seamless plan now carries all five owner
inputs — seamless/no-manual-steps (C0 measure a real repo's init incl.
what the import swallows → C1 composed env + bundle-borne CLI + durable
homes → C2 folder→project auto-register with over-bound named fallback
→ C3 `fleet install` standard phase + doctor's seven facts), the
original's face (C4 reads incl. backend-from-bundle for main's trunk —
`list_commits main` → `NoSuchBranch` — and the `fleet-child-<session>`
fold; C5 the dsh-shaped History/Work/Files/Storage section under the
knowledge group, named `workspace history`, delegation rows'
click-through riding C5), integral-under-knowledge (the four
placements), lanes-as-providers (checked IN C4/C5, forbidden flat in
out-of-scope), closing on C6's clean-environment proof (zero LAYERFS_*
in the process env, fresh folder, register-and-go, face rendering that
same run live). Awaited: the owner's word to start at C0.

**The seamless plan: C0–C6 COMPLETE (2026-10-26; `2fd4522` repair,
`be9c2b2` C4, `9781c9f` C5, `00267a2` C6 close-out; status completed).**
C0 measured a real repo (zcode-router-kit: 1,236 entries / 946 files /
18.7 MB init in 2,511 ms; the import swallows everything with no
exclude → the bound is entry/byte-shaped and mandatory: 2 GiB / 100k
entries). C1 the composed env + the bundle-borne CLI + the durable
home (`~/.fleet/layerfs-home`). C2 folder→project registration with
the bound's pre-check and the named fallback. C3 `fleet install`'s
standard phase + doctor's seven facts. C4 the view model + the
backend reader. C5 the **workspace history** section — the knowledge
group's fourth entry, History/Work/Files/Storage, the delegation rows'
click-through. C6 the clean-environment live proof.

**The install target caught C3's gate lie, and the live run caught
nine more** — the run's value was the defects, not the timings. Three
classes to remember: (1) **a name that exists in two places and agrees
in neither's test** — the install copied `layerfs-register.mjs`, the
driver looked for `register.mjs`, and both test fixtures wrote their
own short name, so every stub was green and every clean-environment
registration failed; fixed with one constant per script plus a pin
that the name the driver spawns is a name `scripts/` ships. (2) **a
fact the process env happened to carry during development** — the
Sandbox's `deploy.volume` is the Store's own Docker volume (derived
from the store path), the face's reads composed nothing (a clean-env
GUI answered every read with a named absence), the history script
resolved against the fleet home instead of the LayerFS home, and the
register rode the worker's endpoint for its Init (which installs the
Store and cannot answer through a worker serving an installed one).
The rule: anything the arm's spawns need, the face's reads and the
install's provisioning must compose from the machine's own record —
**a clean env is the test, and every green that needed the developer's
shell is a lie**. (3) **the stub pinned a shape the wire does not
have** — main carries no merge COMMIT (its head stays null while the
layers chain), so the merge state and the DAG's spine read the layers'
`sourceBranch`; and the child join read the whole-stream tail, which
on a busy mixed-op day holds no delegation row (the op-bounded reader
is the right one). Also: `Verified` must still install the scripts
(copy-if-absent, embedded in the binary — a checkout-relative copy
installs nothing from any other directory), and the register is
idempotent by name (`Exists` is the answer). **The probe is now
self-sufficient** (it derives endpoint, image from the bundle's
manifest, daemon, and the scratch project's store path from the
register's answer; its init rides the sanctioned runtime route):
green exit 0 on the durable home — init 38ms, fork 58ms, sandbox
372ms, mount 104ms, exec 128ms, commit 115ms, idle-exit 121,117ms,
reclaim 319ms reusing the number. **The destination live:** branches
with states (merged/idle from the layers), each child's session and
outcome, the trunk's layers naming their settlements, the figures,
main's own listing carrying the merged child's file, and the
delegation rows' branches. Gate: lib 1550/0, integration 178/0, face
227 checks. Also KI-314's seam note still stands (the child's
host-side processes work at the checkout's own paths; the mount is
sandbox-local) — the live run re-confirmed it: the child wrote inside
its workspace, the parent's tree was untouched until publish.

**"Is it seamless? What about existing and new projects?" — the
owner's question, answered with measurements (2026-10-26).** New
folder on a provisioned machine: YES, proven live (clean env, zero
facts, auto-registered, ran). Existing registered project (the kit):
YES. Three honest gaps, in the order they matter: (1) **the fleet repo
itself — the case that motivated the whole adoption ("several
instances of agents stepping into the code") — CANNOT register: 35 GB
and 228,916 entries against the 2 GiB / 100k bound, entirely because
of `target/`** (the repo minus target/ is 127 MB / 7,818 entries,
well inside). A delegation there takes the named fallback and runs
WITHOUT isolation — the collision-prone default the owner wanted
gone. It is one `scripts/cache-sweep.sh` away from working, but
nothing runs that sweep automatically and the runtime's import has no
exclude, so the flagship case stays on the wrong side of its own
bound. (2) **A merged child's work never lands in the working tree** —
after publish it lives on the Store's `main`, readable through the
face's Files pane (text-only) and nowhere on the host; the
materialize half of KI-314's seam is still open, so "the isolation
works, the reconciliation home does not." (3) **A brand-new machine
is not seamless**: the sealed bundle must be built once by hand
(clone the pinned checkout, `npm install` in `api/`, the python build
tool) — `fleet install` names it as its provision but does not do it.

**The next plan drafted the morning after (`ea3aae4`, status: draft):
`docs/plans/2026-10-26-layerfs-closing-the-seams.md`.** The owner asked
"how do we fix this / prevent it / what are the risks" about the three
residue gaps, then "create the plan." Two wire questions were measured
BEFORE writing, which reshaped the plan: **the runtime image is Docker
Hub's official `node`** (repo digest `node@sha256:3d27e5…` — pull by
tag, verify by digest, never build; the new-machine gap is only the
bundle), and **`branch diff_branches <from> <to>` answers per-path
changes with kinds and before/after** — the materialize's changed set
is one call, not a tree walk. Four criteria risk-ordered: (C0) the
import carries what the repo DECLARES — `git ls-files` when the folder
is a git repo (`.gitignore` is the owner's already-made decision; the
fleet repo is 436 files / 14.7 MB declared vs 35 GB / 228,916 walked;
`.git` NOT carried — a frozen snapshot is worse than absent; an ignored
`.env` can never enter the Store is the named security win); (C1) the
install builds the bundle like it builds lane venvs (pull-by-digest
verify is the supply-chain guard, the probe gates the record); (C2)
settle applies the delta home through the parent's own gated write
path — the conflict design is a **spawn-time digest snapshot of the
tracked tree** checked at settle (host-diverged-since-fork = named
conflict, untouched), child-paths-only, post-merge-only, one grouped
approval in ask mode, NO second knob, and the two-mains residue (Store
main drifting from the host as the owner edits) stated IN the
criterion — correctness holds because a stale base can only conflict,
never overwrite; (C3) the prevention guards as tests (the clean-env
live script kept as `scripts/layerfs-live.sh`, scrubbed-env pins,
fixtures that cite captures, the conflict path forced once on the real
tree). Prevention rules from this session's catches, now plan text:
compose from the record and test with the env ABSENT; one constant per
shared name pinned to disk; a stub's answer shape cites its capture;
C0 measures the motivating case, not a stand-in.

**The closing-the-seams plan: C0–C2 landed, C3 paused mid-proof
(2026-10-26; `beafd6f` C0, `76a58f1` C1+C2; owner paused the goal with
"why is this taking so long?" — answered honestly: the live end-to-end
proof kept finding real defects, and each cost a live-run diagnosis).**
C0 the declared import: a git repo's registration reads its own
declaration (`git ls-files --stage`, NUL-safe; files copied, symlinks
recreated, gitlinks skipped and named), the bound counts the declared
set, the driver stages into `<layerfs-home>/staging/<name>-<pid>/` and
removes it on every path. THE MOTIVATING CASE LANDED LIVE: this
repository itself registered (436 files / 14.7 MB declared vs 35 GB
walked) and a delegation in it ran isolated. C1 the install builds its
bundle unattended (clone keyed by commit, digest-pinned official
images, register/prepare/build via the checkout's own tool) — built a
real 22 MB sealed bundle live in an isolated scratch HOME; the worker
start then named the engine-global container collision (the runtime's
own guard: one `layerfs-worker` container per machine prefix). C2 the
settle brings the merge home: spawn-time content hashes of the
declared tree, the changed set classified at settle (child moved +
host untouched → applied; host moved → HELD, named, never
overwritten; child version == fork-time version → main moved, skipped),
the apply is the ARM's act — ONE grouped approval, then the parent's
own write path (`run_write_file`, now `pub(crate)`, the per-path lock
both rides); a refused merge applies nothing. **Three structural facts
the implementation forced:** (1) **the fork base is a LAYER, not a
Branch** — `diff_branches` takes Branches or main and the plan's
literal `<forkedFrom>` diff is not wire-addressable when main has
never committed, so the spawn hashes carry the base and the settle
diffs child-vs-main before the merge; (2) **the delegation path
resolves FRESH** — machine facts, registrations and the CLI gate are
resolved per call now, because the live run claimed the once-per-
process gate cache and every stub test beside it inherited the real
CLI; `run_on` (the plain delegation harness) pins a scratch
`FLEET_HOME`, because this repository itself becoming registered broke
every plain delegation test's premise; (3) **the driver's `fs_write`
parsed ANY JSON answer as success** — including the wire's refusal
shape (`code` field) — so a refused write reported "wrote 8 bytes"
with no Commit on the branch; fixed with a refusal check that names
the code. **C3 remains:** the live script (`scripts/layerfs-live.sh`,
written, its first runs exposed defect 3 and the hang), the
forced-conflict record, the docs (surfaces/feature/FUNCTIONAL/
TECHNICAL), the full gate, and the plan's status flip. Also discovered
while diagnosing the hang: a killed `fleet run` can leave a CLI holding
the Store's single-writer lane, so the next run's writes block until
the stuck process is cleared — reads still answer, which masks it.

**The closing-the-seams plan: COMPLETE (2026-10-26; `beafd6f` C0,
`76a58f1` C1+C2, `cf73fc9` C3, `02f7506` KI-321 closed).** All four
criteria landed with live proofs. C0: the declared import — git
repositories stage exactly what `git ls-files` names (the fleet repo:
436 files / 14.7 MB against a 35 GB / 228,916-entry working tree) — the
fleet repo registered live and a delegation ran isolated in it. C1: the
install builds its own sealed bundle unattended (clone the pin keyed by
commit, pull the two OFFICIAL images by digest — `rust@…` builds,
`node@…` runs, both pinned beside CHECKOUT_PIN — then the checkout's
own register/prepare/build); live-built a real 22 MB bundle in a
scratch HOME. C2: the settle brings the merge home — diff child vs main
pre-merge (the fork base is a LAYER when main never committed: no base
branch to name — the spawn hashes carry the base instead), classify by
the spawn-time content hashes (child moved + host untouched → applied;
host moved → HELD, named, never overwritten; child's version ==
fork-time → main moved, skipped), then the arm's ONE grouped approval
and the parent's own write path per path. **The forced conflict held
live**: the owner rewrote a tracked file between fork and settle →
`held at conflict: report.txt — you changed it since the child forked,
so nothing was overwritten`. C3: `scripts/layerfs-live.sh` is the
standing clean-env proof (exit-coded; stamp the content per run — the
wire's piece-compare answers no Commit for identical bytes; run --yes —
the apply's grouped approval correctly denies to a headless ask-mode
run). **The live rounds caught three driver defects, each now
structural**: fs_write parsed ANY JSON as success (refusals named now);
the apply-home read the branch path host-relative where the view is
root-absolute (one path, two forms); the delegation path resolves its
machine facts/registrations/CLI FRESH per call (the live run claimed
the once-per-process gate cache and poisoned every stub test beside
it) — and `run_on` pins a scratch FLEET_HOME so plain delegation tests'
premises never ride on the real repo's registration. KI-321 closed
(`02f7506`) with the green standing proof. Substrate findings for the
vendor conversation: branches carry head=None after merges land on
their layers, and a merged integration branch can refuse publish
`NoCommit`. Residue (named in the plan): the child's view of untouched
files ages until a delta re-import at spawn; child-vs-child contested
paths are last-apply-wins on the host.

**The owner's post-build question — "what part of completing the
implementation was ambiguous?" — answered 2026-10-26.** The design's
intent was never unclear; the letter pointed at mechanisms that didn't
exist or were untestable as written. Five judgment calls, ranked: (1)
**the changed-set mechanism was wrong on the wire** — the plan named
`diff_branches <forkedFrom> <child>`, but the fork base is a Layer id
when main never committed and the wire refuses diff_branches with
anything but Branches or main; the redesign (diff child vs main, spawn
hashes decide which differing paths the CHILD moved) is a different
machine than the plan described. (2) **staging placement** — the
plan's letter put it in the register script; it landed in the driver
(same contract, testable where the tests live, no shell/JSON
round-trip). (3) **the snapshot's scope and strength** — git-only (a 2
GiB whole-tree hash per delegation is the wrong cost, so non-git
folders get NO home apply, named); std SipHash documented as
change-detection, not a cryptographic boundary; and two classification
arms the text didn't spell out (host == child's version → already
home; child's version == fork-time → main moved, skip). (4) **C1's
build recipe** — the vendor tool requires images BEFORE register (it
never pulls), needs the resources.json path threaded register →
prepare → build, and `npm install` in api/ proved unnecessary; the
plan's "pull by tag, verify against the manifest" became
pull-by-digest directly (the verification IS the pull). (5) **the
grouped approval's concrete shape** — a synthetic tool call (name
`layerfs`, arguments `{"apply_home": {paths, held}}`) the existing
gate classifies sanely, plus the ruling that a headless ask-mode
denial is CORRECT (nobody at the seat), which forced the live script
to `--yes`. The discipline: every letter-versus-intent divergence was
recorded in the plan's status section as it was made, so the document
reflects what was built.

**The live-fork-base plan: COMPLETE (2026-10-26, `fb82f48`).** The
stale-fork-base residue retired: the registration records the git tree
its declared import staged, and each spawn diffs the host's current
tree against it (temp-index `git write-tree` — 13–49 ms warm on the
fleet repo) and carries the host's newer tracked paths onto the fresh
fork before the child's first turn. Three-way classification from
main's versions: main == recorded → the host's change is newer,
carried; main != recorded → main-newer, skipped and reconciled home by
the settle in the same grouped approval (the result names applied /
reconciled / held). `merge_back` retries the publish once after 400 ms
on `NoCommit` (KI-321's remedy note). The recorded tree advances at
every sync, so consecutive delegations diff against what was actually
reconciled. **Live proofs:** the retirement scenario (host edit AFTER
registration, the child READ the post-edit content — the exact failure
from the conflict forcing) and layerfs-live.sh's new second-delegation
check (the second child reads the first's stamped file), green end to
end. **The pin caught a defect:** recorded_content read the tree from
the process's cwd — the object lives in the folder's own .git; the
read runs in the folder's repo now. The owner's rule stands: no
leftovers — every named residue gets a plan, and every plan's own
residue gets named on its way out.
