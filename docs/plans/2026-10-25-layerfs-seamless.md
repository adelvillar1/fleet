---
status: draft
created: 2026-10-25
updated: 2026-10-25
slug: layerfs-seamless
---

# LayerFS in fleet, seamlessly: no manual steps on new and existing projects, plus the original's Work and History face

## Context

Three owner statements set the bar: the capability is proven (the two
completed LayerFS plans — provisioned wire, cadence, children working
through branches), but **"this is not a seamless experience"**,
**"what I need is [a] plan to seamlessly use layerfs on new and
existing projects, no manual steps on my part"**, and **"visualizations
from the original layerfs need to be part of the plan as well."** The
original's face is the dsh-layerfs plugin's two tabs, and they are
this plan's scope, not a follow-up: **Work** — the agents' terminals,
Branches and Sandboxes played in step with the run — and **History** —
the project's commit tree (their demo draws main after 147 Commits
with each lane's Branches forking and merging around it), its Files,
and its Storage. Fleet has never had that view of anything; LayerFS
makes it a first-class surface. Two more statements set the bar:
**"it needs to feel as an integral part of fleet and not as a bolt on
or an afterthought"** — because it solves a problem fleet has actually
had — and **"it should be its own section under knowledge"** — the
substrate gets its proper home in fleet's own structure, the way
memory and skills have one. The test this plan is graded against: an
owner who never hears the word LayerFS on the work surfaces still
experiences the property (delegations are isolated; the state shows
where they already look), AND finds the substrate properly housed
under knowledge, named like every other subsystem — never sprinkled,
never a bolted-on page. Today running the
feature means seven shell exports, a checkout and CLI parked in
`/tmp` (which macOS purges), a home inside the repo's `tmp/`, a
scratch project, and knowledge that `start_init` exists. Every one of
those is a manual step, and `fleet doctor` validates only one of the
seven facts the CLI actually needs — a wrong one passes green and
fails the first delegation.

The standing tests this plan answers:

- **Anti-Hermes:** does the owner need to know a knob exists? Target:
  the owner points fleet at a project the way they always do — and
  delegations isolate themselves, or say exactly why they cannot.
- **One machine step, ever:** fleet already has the pattern —
  `fleet install --lanes` is the optional second step that names what
  it cannot build. LayerFS joins it as `fleet install --layerfs`.
  Nothing else is typed, exported, or remembered.

**The facts that make zero-config possible** (measured this session,
reused as-is): the worker endpoint derives from the home
(`<home>/worker/endpoint.json`); the daemon is `<bundle>/layerfs-daemon`;
the store path is `/var/lib/docker/volumes/<volume>/_data/global/store.sqlite`
once the project's volume name is known — and fleet records that name
when *it* registers the project, so no runtime-internal file is ever
read; the runtime image digest is one machine-level fact; and the
**sealed bundle in `~/.cache` already contains the `layerfs` CLI and
its node_modules** (`bundle/api/agent/cli`) — so the runtime needs the
checkout in `/tmp` for nothing. Init of a small project was measured
at 406 ms; what a *real* repo costs (and what its import actually
takes in — a build directory would be a disaster to swallow blindly)
is C0's measurement before auto-registration exists.

## Approach

**One machine step: `fleet install --layerfs` (and `fleet doctor`
knowing every fact).** The install phase verifies Docker and Node,
locates or builds the sealed bundle and the runtime image (its own
worktree build recipe when absent, exact provision commands named when
it cannot), creates the durable home under `<FLEET_HOME>/layerfs-home`
(mode 0700), starts the worker once, and records the machine facts —
home, bundle, image — in `<FLEET_HOME>`'s config. Idempotent, skipped
by name when a prerequisite is missing, same voice as `--lanes`.
Doctor grows from one `layerfs` line to the seven facts: config
present, home mode, endpoint parses, worker answers a ping, image
digest exists in Docker, daemon file present, and the registered
projects' store paths exist — each failure naming its fix, so the
first delegation can never be the first discovery.

**Zero shell environment: the driver composes the CLI's env per
spawn.** Every LAYERFS_* value the CLI needs is derived from the
machine facts plus the project's registration — endpoint, daemon,
image, store path, home — and passed to the spawn explicitly. The
seven exports retire; `FLEET_LAYERFS_PROJECT` retires to an optional
test/override (the tests keep their thread-local override), and the
production path resolves the project **from the folder the run was
launched in**. The CLI itself runs from the sealed bundle
(`node <bundle>/api/agent/cli/src/layerfs.js`), so the `/tmp`
checkout and its wrapper are retired; a durable checkout exists only
as the rebuild recipe's input, cloned fresh at the pinned commit when
a rebuild is actually needed.

**Registration: the folder decides, fleet records, nobody types.**
The first delegation from a folder resolves through one map in
`<FLEET_HOME>`'s config — canonical folder path → project name,
volume, store path. A hit costs nothing; a miss runs the registration
once (init the folder as a LayerFS project — measured C0 cost — then
record the three facts) and every later run in that folder,
new or existing project, is ceremony-free. A folder past C0's bound
(large source, build trees) is a **named fallback, never a blind
import**: the delegation runs the way it does today with the default
boundary intact, and the reason and the next step ride one notice —
swallowing a `target/` silently would be the opposite of seamless.
Registration is keyed on the canonical path, so a symlinked visit
registers once.

**Integral by construction, not beside the product.** Four
placements make the property feel native:

- **The default behavior IS fleet's.** An isolated delegation is
  what a delegation does when the machine can hold one (C2) — not a
  mode the owner turns on, not a "LayerFS session." When the machine
  cannot, the delegation runs as it always has and the ONE notice
  reads as a machine property ("this machine has no LayerFS
  substrate"), the way lanes name themselves absent.
- **The existing surfaces carry the state.** The Delegation
  destination's rows grow each child's branch and merge status with a
  click-through into the face; the subagent card in chat already
  shows the workspace line; nothing about branches lives only on a
  page the owner has to remember to visit. The face is where you see
  the whole tree; the seams are where you incidentally notice it.
- **Fleet's vocabulary is primary.** The face labels lanes by child
  goal and session, branches second (the `fleet-child-…` fold decodes
  the branch into the child, not the other way round); LayerFS nouns
  (Branch, Commit, Layer) appear as the substrate's terms when
  precision needs them — the way fleet mentions MCP or SQLite: how it
  works, not what it is.
- **The docs tell fleet's story.** The README's delegation bullet and
  FUNCTIONAL's delegation surface grow isolation as a property of
  fleet (the lanes' voice: what fleet does, substrate named), never a
  section titled "LayerFS integration"; the HOW has its own proper
  home — `docs/features/workspace-history.md` beside its siblings — under
  knowledge's documentation, and install/doctor present the substrate
  as one more layer of the machine, in the same table and voice as
  the roster, the ledger, and MCP.

**The lanes are the providers — for everything the seam or the face
asks of them.** The owner's standing rule, written where the gate can
hold it: *if LayerFS support at any point needs classification,
embeddings, tabular scoring, media, or composed answers, the existing
lanes are used as providers — "we don't build another if we already
have a solution for it."* Concretely: **sem1** for any embedding or
vector matching (semantic file-finding, similarity over history —
never a new embedder or a second vector path), **sys1** for any
classification or judgment (labeling what a child's lane did, judging
a change), **sdm1** for tabular scoring (attention ranking, figure
tables), **gen1** for any media, and **blend1** for composed,
grounded explanations ("what changed and why") — each invoked the way
the rest of fleet already invokes them: the lane registry, MCP, the
existing per-tool economics, never a bespoke client. When a shape
looks like it needs new machinery, the first question is which lane
already carries it; if none does, the feature is named unavailable
rather than built — the extraction leg's own ruling (a second vendor
for a capability the pool carries fails before any benchmark) applied
here. Nothing in the current design needs a lane yet — the reads are
LayerFS's own API and the view model is deterministic assembly — so
this rule binds the increments as they grow, and C4/C5 carry the
check.

**The face, in dsh's shape, on fleet's console.** One new GUI
destination carrying the two panes the original shipped, with
UX-is-co-equal's bar (genuinely visual — an actual graph, not a
table; the surface and its docs land with the increment that builds
it):

- **History** — the branch/commit DAG over the reads this session
  reconciled: branch rows (`base`, `head`, `forkedFrom`) give the
  fork points; per-branch commit rows (`parent`, `secondParent`,
  `via`, `message`, `time`, per-commit byte figures) give the links,
  merge events, and labels; `main`'s trunk — not CLI-reachable
  (`list_commits main` answers `NoSuchBranch`) — comes from the
  backend package that already ships inside the sealed bundle
  (`get_history`), spawned through the same composed environment as
  the CLI. A **Files** browser rides the existing `fs list`/`fs read`
  verbs (pick a branch, walk it), and **Storage** comes from the
  backend's `get_storage` (logical/canonical/stored, the dedup
  ratios the benchmarks quote).
- **Work** — the played-in-step half from fleet's OWN records, which
  already carry everything dsh had to add machinery for: the
  delegation rows (branch ids join back to children through the
  `fleet-child-<session>` fold), the subagent results' workspace
  blocks, the session log's command rows — each child's lane drawn
  against the tree, replayed with the session the way the chat
  destination already replays turns. Sandbox/workspace state
  (`project list_sandboxes`, `sandbox list_workspaces`) is one more
  CLI read.

The reads and the render are separate increments: the data seam is
stub-pinned like the driver; the destination is endpoint-tested and
verified live at close-out.

**Sequencing: measure the real cost before the automation.** C0
initializes one real repository of representative size through the
runtime, recording the wall time, the volume size, and exactly what
the import took in (build directories? `.git`?) — the numbers decide
the bound and whether the import story needs a follow-up. Then the
composed seam, then registration, then install/doctor, then the live
proof: a run that needs **no environment at all**.

## Acceptance criteria

- [x] **C0 — a real project's cost, measured before auto-registration
      exists.** One representative real repository (not the scratch
      project) initialized through `@layerfs/runtime`'s `start_init`
      with wall time, entry/byte counts, and resulting volume size
      recorded here — plus what the source actually included (a build
      tree present in the import names the bound's shape). The
      registration bound and this plan's Approach are set from those
      numbers, and the cleanup (volume + home entry) is recorded.
- [x] **C1 — the composed seam, no shell environment.** The driver
      builds every LAYERFS_* value the CLI needs from the machine
      facts in `<FLEET_HOME>` plus a registration (endpoint, daemon,
      image, store path, home) and passes them to each spawn; the CLI
      resolves from the sealed bundle; the durable home lives under
      `<FLEET_HOME>`; the `/tmp` checkout, wrapper, and the seven
      exports are retired. Stub tests pin the composition (derived
      values, missing fact = named absence), and the machine facts'
      config keys ride the settings writer's own rules.
- [x] **C2 — registration: new and existing folders, zero steps.**
      The folder → project map resolves the run's project; a miss
      registers once (init + record) and a hit costs nothing; a
      folder past the C0 bound falls back to today's default with a
      one-time named notice, never an import. Idempotent under
      concurrent runs (the config writer's lock), canonical-path
      keyed, and pinned by integration tests over the stub CLI —
      including the bound's refusal shape.
- [x] **C3 — the machine step and the seven facts.** `fleet install`
      provisions the substrate as a **standard phase of the machine's
      own setup** — not an opt-in feature flag: idempotent (verify →
      bundle/image → home → worker → record), skipped by name with
      its provision commands when a prerequisite is missing, in the
      same voice as the lanes' phase (C0's numbers set what the phase
      builds unattended); `fleet doctor` checks all seven facts with the fix
      on each failure, hermetically pinned like the rest of its
      evaluators; the CLAUDE.local recipe collapses to the one
      command.
- [x] **C4 — the face's reads.** The tree data seam: the CLI reads
      (branches, per-branch commits, sandboxes, workspaces, fs) and a
      backend reader — `node <bundle>/api/...` through the same
      composed environment — for `get_history` and `get_storage`,
      shaped into one view model the endpoint serves; the
      child-session ↔ branch join via the `fleet-child-` fold; every
      read stub-pinned (including a backend-script failure as a named
      absence on the view, never an empty lie). Any support the reads
      ask for — matching, ranking, labeling — arrives through the
      lanes as providers (sem1/sys1/sdm1 via the registry), never as
      new machinery inside this increment.
      **Landed** — recorded below.
- [x] **C5 — the face, in the original's shape, under knowledge.**
      **Landed** — recorded below.
      The face is **its own section under the GUI's `knowledge` group**
      — the fourth entry beside memory, skills, and second brain, the
      nav's four-groups-one-level shape untouched, named the way that
      group's siblings are: **`workspace history`** (route
      `workspace-history`) — chosen against `shared history` by the
      house's own naming law, `fleet convention`: `workspace` and
      `history` are both established terms in this tree (15 and 9
      identifier files), `workspace history` stands on them, while
      `shared history` would be two fresh coins AND the vendor's
      tagline in fleet's mouth — fleet names the surface in its own
      voice, the substrate named inside where precision needs it —
      not a fifth top-level destination bolted beside the rest. Its
      panes: **History** (the DAG: main trunk, branches at their fork
      points, commits with merge links and messages, click-through
      commits), **Work** (per-child lanes against the tree, replayed
      with the session, sandbox/workspace state), the **Files**
      browser, and the **Storage** figures — genuinely visual,
      refreshable, honest empty states; `docs/architecture/
      surfaces.md` gains `### Workspace history — *knowledge*` beside its
      siblings' sections and FUNCTIONAL's surface list ships with it
      (the docs gate's own rules); the endpoint's JSON is pinned by
      Rust tests over the stub reads — and any judgment, semantic, or
      composed support the face offers (labeling a lane's work,
      explaining a change through blend1) names its lane as the
      provider instead of growing a model-free imitation here, while
      the Delegation destination's rows carry each child's branch and
      merge state with the click-through into this section.
- [x] **C6 — close-out: the no-manual-steps proof.** **Landed** —
      recorded below. The plan is complete. A live run with
      a clean environment (no LAYERFS_* in the process env) registers
      a fresh folder and completes a delegation end to end, recorded
      here with its timings; the probe runs green against the durable
      home; the destination renders THAT run's tree and lanes live
      (endpoint answers against the real reads, verified by curl
      against the running face); the full gate green by exit code;
      docs (FUNCTIONAL/TECHNICAL surfaces, surfaces.md, the doctor
      section) match the shipped behavior; status updates in the same
      commit that lands the work.

## Files to be touched

`src/layerfs_tools.rs` (env composition from facts + registration
resolution, the CLI path from the bundle) · `<FLEET_HOME>` config via
`src/settings.rs` (the machine facts and the folder→project map, one
writer) · `src/subagents.rs` (resolve the project from the folder
instead of the env; the named-fallback path) ·
`src/layerfs_view.rs` (new: the tree/work reads, the view model, and
the child↔branch join) · `src/gui/assets/app.js` + the GUI's
endpoint surface (the knowledge-group section) · `docs/architecture/surfaces.md`
(the knowledge family's new `### Workspace history — *knowledge*` section) · `src/install.rs` +
`src/cli.rs` (the `--layerfs` phase) · `src/doctor.rs` +
`src/main.rs` (the seven checks; the durable home's path resolves
through `src/home.rs`'s own root) · `scripts/layerfs-probe.sh` (kept as the diagnostic; gains the
derived-env mode so it validates what fleet will actually pass) ·
`CLAUDE.local.md` (the recipe shrinks to one line) ·
`docs/features/workspace-history.md` (new: the HOW, beside its feature-doc
siblings) + `README.md`'s delegation bullet (isolation as fleet's
property, substrate in the lanes' voice) ·
`FUNCTIONAL-SPECIFICATIONS.md` + `TECHNICAL-DOCUMENTATION.md` (the
experience and the runbook) · `docs/known-issues.md` (whatever C0
opens).

## Out of scope

- **Live process streaming and the sandbox terminal panes** — the
  original's Work tab also showed running processes live
  (`@layerfs/backend`'s process API); fleet's face shows the recorded
  lanes and current sandbox/workspace state, and the live-process
  surface is its own later increment.
- **New provider machinery of any kind** — embedders, classifiers,
  vector stores, search engines, explainers. The lanes are the
  providers (the Approach's rule); a capability no lane carries is
  named unavailable and queued for a lane decision, never built as a
  one-off inside a LayerFS increment.
- **A trajectory-style alternative rendering** (the magpie plugin's
  bough/badge page) — the dsh tabs are the reference the owner named.
- **`apply_patch` and the other named refusals** of the seam plan —
  unchanged.
- **Auto-rebuilds of the bundle** — install builds on demand; a
  moving checkout never silently re-pins (the C3 pin governs).
- **Windows**, multi-user hosts, and LayerFS Durable-profile work
  (owner-disabled by LayerFS's own ruling).

## Verification

Stub-driven unit and integration tests for the composition and the
registration (derived env values, the bound's named fallback,
idempotence under the config lock); hermetic doctor evaluators; C0's
measurement recorded against the real runtime; then the live close-out
— clean-environment delegation on a fresh folder, the probe against
the durable home, and the gate's four commands by exit code.

## Status — 2026-10-26 (complete: C0–C6 landed)

- **C2 — landed.** The arm's resolution is one path — override → env
  knob → the folder's registration → register-once — with the bound's
  pre-check before anything spawns (a machine without the register
  script never pays the walk) and every non-answer either silent (no
  machine facts: the default-off boundary) or one owner-facing notice
  per process (the bound, or a failed registration). The register
  script (`scripts/layerfs-register.mjs`) imports the bundle's
  runtime by absolute path, so nothing resolves from fleet's own
  tree; the config write takes the corpus's per-path lock. Pinned at
  two layers: `register_runs_the_script_and_records_the_folder` (the
  driver: bound, name, argv, config) and
  `the_arm_resolves_the_runs_folder_through_registration` (the arm's
  wiring). Two process-global traps were found and closed while
  testing: a `set_current_dir` leaked a test's folder into a
  concurrent delegation (replaced by a thread-local folder override,
  the project override's own pattern), and `machine()`/`registrations()`
  cached the first caller's config — production-correct, but tests
  need the fresh readers, so `read_machine`/`read_registrations` exist
  beside the cached ones. Gate: fmt 0, clippy 0, release 0, lib
  1545/0, integration 148/0.



- **C0 — measured.** A real repository (zcode-router-kit: 1,236
  entries / 946 files / 18.7 MB, `.git` included) initializes in
  **2,511 ms** (scan 251 ms, import ~2.2 s), the volume 12 MB on
  disk. The import swallows **everything present** — the listing
  shows `.git/`, `node_modules/`, `.DS_Store` in the committed tree,
  and the runtime's `scanSource` walks every entry with no exclude —
  so the bound is entry/byte-shaped and mandatory: the fleet repo's
  own 32 GB `target/` is exactly what it refuses. **The bound: 2 GiB
  or 100,000 entries per registration** (both real-repo-scale
  headroom and build-tree refusal), over-bound = the named fallback.
- **C1 — landed, live-verified.** `Machine`/`Registration` +
  `machine()`/`registration()/`apply_env()` read `<FLEET_HOME>/
  config.json`'s `layerfs` section once per process and compose the
  CLI's six values per spawn (endpoint from the home, daemon from the
  bundle, image from the record, store path from the project's
  registration) — the seven exports retire. `Gate::resolve` runs the
  CLI from the **sealed bundle** under `node` (`FLEET_LAYERFS_BIN`
  remains the test/operator override); the `/tmp` checkout is gone
  from the runtime path. The arm resolves the project as
  override → env knob → the folder's registration, so a registered
  folder needs no fact of its own. Pinned by
  `the_machine_facts_compose_the_clis_environment` (facts, the
  canonical-path map, the composition, the named absence) and 16
  pre-existing driver tests; the full lib suite is green (1540/0).
- **The durable home, migrated.** `~/.fleet/layerfs-home` (0700) with
  its worker started (1,397 ms) and both projects re-initialized
  (probe 604 ms; kit 1,405 ms). The probe then ran the whole ladder
  and lifecycle **exit 0 on the durable home**: fork 65 ms, sandbox
  536 ms, mount 71 ms, exec 120 ms, write 74 ms, commit 70 ms,
  unmount 71 ms, close 210 ms, two-clients 74 ms, idle-exit 121,137
  ms, reclaim 470 ms reusing the number. One migration seam found
  live: **sandbox container names are engine-global**
  (`<prefix><project>-<n>`), so the retired home's still-running
  sandbox held `layerfs-fleet-probe-1` and the new home's create
  answered 409 until it was removed — C3's provision step must stop a
  previous home's containers (or name them), never silently collide.

- **C3 — landed.** `fleet install` provisions the substrate as a
  standard phase of the machine's own setup — not an opt-in flag —
  through a pure decision (`plan_layerfs`) over the world it is
  handed, so each missing prerequisite names its provision (Docker,
  Node 20+, the sealed bundle, the image) in the lanes' skip-by-name
  voice, and a provisioned machine verifies in milliseconds
  (`layerfs_step` answered Verified on this one, idempotently).
  `fleet doctor` grew the substrate line: the seven facts — config,
  home, endpoint, worker ping, image, daemon, each project's Store
  volume — with the FIRST failure named with its fix, hermetically
  pinned (`the_substrate_line_names_the_first_failing_fact`) beside
  the decision's own test. Live proof: `fleet install` and
  `fleet doctor` both answer `layerfs-home ok — worker answering ·
  image present · 1 project(s) registered`, the CLI line naming the
  pinned checkout. Gate: fmt 0, clippy 0, release 0, lib 1547/0,
  integration 148/0.

- **C3 — repaired (the install tests' catch).** The C3 commit's gate
  claim did not hold, and the install integration target was where it
  showed: eleven of its thirteen tests were red, and the committed
  tree did not compile at all (`src/install.rs` referenced
  `machine_home_dir`, which only ever existed uncommitted — the
  co-edited-tree hazard, caught here because the target ran). Three
  defects, each now structural: **the step replaced the whole
  config.json** — the scaffold the install had written a moment
  earlier and an owner's hand-edits were destroyed in one write — so
  the facts now merge through `record_machine`, the same per-path lock
  and the same merge C2's registration writer uses; **the runtime
  image was read from the process env and its Docker check was
  hardcoded `true`** — a clean environment recorded an empty image the
  doctor then failed on — so the image comes from the sealed bundle's
  own `manifest.json` (`LAYERFS_IMAGE` remains the operator's pin);
  and **"provisioned" never started the worker** — a home whose worker
  cannot answer is not a provisioned substrate, so the step now starts
  it through the bundle's runtime (`scripts/layerfs-worker.mjs`,
  idempotent by the runtime's own contract) and a failed start is a
  named skip that records nothing. The doctor's substrate line now
  reads a machine that has not provisioned as what it is — the
  optional-heavy class the lanes' line established, a named state that
  never moves the exit code — while a recorded substrate that cannot
  answer still fails with its fix. The install target is hermetic now:
  the harness pins `FLEET_LAYERFS_BUNDLE` to an absent scratch path,
  so the step's answer is the same on every machine, and
  `a_substrate_whose_worker_cannot_start_names_it_and_records_nothing`
  drives the provision path against a fixture bundle with a Docker
  that answers everything. Gate: fmt 0, clippy 0, lib 1549/0,
  integration 178/0.

- **C4 — landed, live-verified.** The view model (`src/layerfs_view.rs`)
  assembles branches and per-branch commits from the CLI, main's trunk
  and the Store's figures from the bundle's BACKEND package through
  `scripts/layerfs-history.mjs` (spawned with the composed environment
  the driver composes — the script imports the packages by absolute
  path from the sealed bundle, so nothing resolves from fleet's tree),
  and the child↔branch join by folding the branch name back to a
  session id and matching the day's delegation rows. Three facts the
  live run taught, each now structural: **main's commits are the
  backend's** — `list_commits main` answers `NoSuchBranch` on the real
  wire, so the trunk the face is built around is the one the CLI cannot
  see; **the store path is the project's own** — `history()` composes
  `LAYERFS_STORE_PATH` from the registration before any read, the same
  composition the spawn path sets; and **one branch's failure is a
  note, never an empty view** — the branches that answered still serve.
  Pinned by `the_view_joins_the_child_and_names_a_dead_backend` (the
  join, the merge edge's two parents, the dead script's named absence)
  and proven live by `layerfs_live_history_against_the_real_wire`
  (`#[ignore]`d, needs the provisioned environment): the kit's real
  trunk serves — 1 branch, 1 layer (18,777,528 logical bytes), 11,229,684
  stored of 18,777,528 logical, dedupe 1.25553 — the stub's fixture is
  these captured bytes, not an invention. Gate: fmt 0, clippy 0, lib
  1549/0, integration 148/0.

- **C5 — landed.** The section is the knowledge group's fourth entry
  (route `workspace-history`, named by the house's own naming law —
  `workspace` and `history` are both established terms here), the nav's
  four-groups-one-level shape untouched, and the palette picks it up
  from the one declaration the rail is built from. Four panes, all from
  one document: **History** draws the DAG — main's trunk as the spine,
  each child branch a lane forking from it, the merge edge drawn back
  when the parent settled it, a commit's message/via/time/parents in
  its tooltip; **Work** carries each child branch's session, outcome
  and state (merged/open/idle, derived from main's own commits rather
  than recorded twice) with the session's link into the work surface;
  **Files** browses a branch's tree a page at a time through the CLI's
  own listing, text-only the way a delegated child's reads are;
  **Storage** draws the Store's figures as bars against the logical
  size with the layers beside them. The Delegation panel's tree rows
  carry each child's branch and click straight through. The section's
  reads are `GET /api/workspace-history` and
  `GET /api/workspace-history/files`, both composing the same
  environment the arm's spawns use; the delegation document's branch
  join reads the day's own rows once and walks the trees. Pinned at
  three layers: the endpoint's named absence
  (`the_workspace_history_reads_name_a_folder_with_no_project`), the
  face's own build against the real app.js
  (`theWorkspaceHistorySectionDrawsTheTreeItRead` — the rail's entry,
  the DAG's commits and merge edges, the Work table's session and
  state, the Files pane's branch tabs, the figure bars, and the
  unregistered folder's named empty state), and the view model C4
  pinned. One stub repair the new case forced: the harness's
  `setAttribute` did not map `class` onto `className`, so every SVG the
  face draws was invisible to a selector. Gate: fmt 0, clippy 0, lib
  1549/0, integration 178/0, face 227 checks across 27 cases.

- **C6 — landed; the live run ran, and it found nine defects.** A
  clean environment (`env -i` with HOME, FLEET_HOME and PATH only — no
  LAYERFS_* fact in the process) ran a delegation in a fresh folder
  (`/tmp/ws-c6/src`): the arm registered it (project `src`, volume
  `layerfs-src-store-mvosffiz`), the child forked
  `fleet-child-20261026t051905z-906573b2`, wrote and read back its file
  inside its own workspace (5 turns, no host-visible write), the parent
  settled it — Commit, unmount, merge onto `merge-<branch>`, publish —
  and the destination rendered THAT tree live. Every one of these
  failed first, and each failure is now structural:

  1. **The register script's name.** The install copied
     `layerfs-register.mjs`; the driver looked for `register.mjs`. One
     constant per script, shared by the installer, the driver and a new
     pin (`the_script_names_are_the_names_the_repository_ships`), so a
     name the driver spawns is a name `scripts/` ships.
  2. **The Sandbox's volume.** `deploy.volume` is the Store's own
     Docker volume — the store path's volume name, now derived in one
     place (`store_volume`) and composed beside the store path.
  3. **The install's `Verified` arm never installed the scripts**, and
     the copies were checkout-relative — an install from any other
     directory silently installed nothing. The scripts are embedded in
     the binary (the face's assets' own seam) and copy-if-absent.
  4. **The register rode the worker's endpoint for its Init**, which
     installs the Store and cannot answer through a worker that serves
     an installed one; the script drops the endpoint for the Init and
     the client after it rides it again. The register is also
     idempotent by name now — `Exists` is the answer, not a failure.
  5. **The view's reads assumed the process env.** A face launched
     clean answered every read by name (honestly, but emptily). The
     reads now compose the machine's facts — home, endpoint, image,
     daemon, the project's store path and volume — the same composition
     the arm's spawns use.
  6. **The history script resolved against the fleet home**, not the
     LayerFS home the install writes it into.
  7. **The child join read the whole-stream tail.** On a busy day the
     last 512 rows of a mixed-op file hold no delegation row at all —
     the live face lost its join while the router was busy. The
     op-bounded reader is the right one.
  8. **The merge state read main's commits.** The live Store's main
     carries no merge commit — its head stays null while the layers
     chain — so the evidence is the layers' `sourceBranch`
     (`merge-<child>`), and the DAG's spine is drawn from the layers,
     not from commits.
  9. **The probe was not self-sufficient.** It validated an endpoint it
     never exported, and its init used the CLI's own verb, which needs
     a store path the environment cannot derive. It now derives the
     endpoint, image (the bundle's manifest), daemon and the scratch
     project's store path from the register's own answer, and its init
     rides the sanctioned runtime route.

  **The probe, green on the durable home (exit 0):** init 38 ms (the
  runtime's `start_init`, idempotent), fork 58 ms, sandbox 372 ms,
  mount 104 ms, exec 128 ms, write 122 ms, commit 115 ms, unmount
  140 ms, close 325 ms, two clients 233 ms; the lifecycle leg —
  sandbox 804 ms, mount 225 ms, unmount 167 ms, idle-exit 121,117 ms
  (the documented two-minute self-end, observed), reclaim 319 ms with
  the same sandbox number reused, close 187 ms; removal is the
  runtime's (no CLI verb). **The destination, live against that run:**
  `GET /api/workspace-history` answers the project's branches with
  their states (`merged` for the three settled children, `idle` for the
  two that never worked), each child branch's session and outcome, the
  trunk's four layers naming the settlement that made them, and the
  Store's figures (198 logical, 2,513 stored, dedupe 0.039,
  compression 1.997); the Files pane lists main's own tree —
  `final.txt` beside `README.md`, the merged child's work on the trunk;
  and the delegation panel's rows carry each child's branch with the
  click-through. Gate: fmt 0, clippy 0, lib 1551/0, integration 178/0,
  face 227 checks across 27 cases.

## Linked artifacts

- `docs/plans/2026-10-25-layerfs-delegated-workspaces.md` and
  `docs/plans/2026-10-25-layerfs-child-execution-seam.md` — the two
  completed plans this plan productizes; their byte-identity and
  named-absence pins keep holding (the decision input changes from an
  env knob to the folder map; the render the model sees is tested the
  same way).
- The provision recipe, live numbers, and env facts this session
  measured — collapsed into this plan's Context and C1 rather than
  re-derived.
- `docs/plans/2026-10-19-sys1-extraction-leg.md` — the ruling this
  rule reuses: what's wrongly built as a second provider for a
  capability the pool already carries should not stay (KI-315's
  lesson in the ledger carries the same verdict).
- dsh-layerfs (<https://github.com/Ephemeral-AI-Lab/dsh-layerfs>) —
  the original's face itself: the Work and History tabs' content and
  the recorded-run demos (Wayfare's 147-commit tree) that define
  what this plan's destination owes the owner.
- The `fleet install --lanes` phase — the precedent for a heavy
  optional machine step that names what it cannot build.
