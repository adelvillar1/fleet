# Closing the seams: what the import carries, where the work lands, what a new machine needs

The seamless plan (`2026-10-25-layerfs-seamless.md`, completed `00267a2`)
delivered the substrate, the face, and the clean-environment proof — and
its close-out's live run, plus this morning's measurements, put three
gaps on the table. Each is measured; none is speculative. This plan
closes them in risk order, each increment its own blast radius.

## Context

Measured 2026-10-26 against the provisioned machine and the live Store:

1. **The fleet repo — the case that motivated the substrate — cannot
   register.** 35 GB and 228,916 entries against the registration bound
   of 2 GiB and 100,000, so a fleet-repo delegation falls back to the
   un-isolated default with one named notice: the collision-prone
   behavior this substrate exists to prevent, persisting in the one repo
   where the collisions happened. The overage is entirely `target/`
   (35 GB of 35 GB); `/target` is the FIRST LINE of the repo's own
   `.gitignore`, and what git declares is **436 files, 14.7 MB, zero
   credential-shaped paths**. The repo has already made the decision the
   bound exists to force; the registration just never asked it.
2. **The child's merged work never reaches the working tree.** The C6
   live run settled a delegation and the file landed on the Store's
   `main` — present in the face's Files pane, absent from the folder the
   run worked in (checked after settle: not there). The isolation half
   works; the reconciliation half does not exist.
3. **A new machine stops at the sealed bundle.** `fleet install` names
   the missing bundle and skips; the build chain (clone the pinned
   checkout, `npm install` in `api/`, the build tool) is manual. The
   other prerequisite turns out to be free: the runtime image's repo
   digest is `node@sha256:3d27e5c11e57…` — Docker Hub's OFFICIAL node
   image, pullable by tag and verifiable by digest, never built.

And the wire answers the materialize question directly:
`branch diff_branches <from> <to>` answers per-path changes with kinds
and before/after sides (measured against the live Store: one call
naming `/final.txt`, kind `Added`). `fs blame`, `fs stat` and
`fs diff_files` sit beside it. The changed set is one read, not a tree
walk.

The owner's standing inputs bind as before. **Integral, not bolt-on:**
the delta rides the settle that already exists, the face already shows
the merge state, and there is no new knob — the import carries what the
repo declares, which is a decision the owner already made when they
wrote the `.gitignore`. **Lanes as providers:** nothing in this plan
needs a lane — diff, digests and apply are deterministic reads and
writes; if an increment grows a judgment need (labeling a conflict for
the owner, say), the lane names itself there and none is built here.
**Anti-Hermes:** a new machine goes from a manual three-command chain to
the `fleet install` it already runs.

## Approach

Three increments in risk order. The import changes a guarantee and says
so in the docs that ship with it. The install build moves a manual chain
into the machine's own setup, the way the lanes' venvs are built. The
materialize writes the host tree — the incident class that started this
work — and therefore carries the heaviest mitigations in the plan:
approval-gated, child-paths-only, post-merge-only, digest-checked
against the fork point, and dead already if the delegation knob is off.

## Acceptance criteria

- [ ] **C0 — the import carries what the repo declares.** When the
      folder is a git repository, registration stages and imports
      exactly what git tracks (`git ls-files` — one implementation of
      ignore semantics, git's own, honoring `.gitignore` by
      construction); a non-git folder keeps today's whole-tree walk with
      the bound as the guard. The register script gains the manifest (a
      JSON file of relative paths, no shell splitting); it stages the
      named paths into a scratch dir under the LayerFS home, inits from
      there, records, and removes the staging dir on every path (success
      and failure). The bound's pre-check counts the MANIFEST for git
      repos (436 files / 14.7 MB for this repo — no full-tree walk) and
      the walk for the rest. Named honestly, in the docs that ship: the
      Workspace is the declared source tree — `.git` is not carried (a
      frozen snapshot would be worse than absent; git history stays the
      host checkout's job), untracked files are not carried (the
      branch view shows the declared tree), submodules are named gaps,
      symlinks carry as links — and the security win is stated beside
      the cost: an ignored `.env` can never enter the Store. Edge cases
      pinned: a repo with nothing tracked is the named fallback; spaces
      and unusual bytes in paths ride the JSON manifest untouched.
      **The motivating case proves itself: the fleet repo registers for
      real, and a delegation launched in it runs isolated** — the
      one-notice fallback is gone from the repo this work exists for.
- [ ] **C1 — the install builds the sealed bundle.** `fleet install`'s
      substrate phase grows the build the way the lanes' phase builds
      venvs: clone the pinned checkout (`CHECKOUT_PIN` names it — the
      same pin the doctor carries), `npm install` in `api/`, the build
      tool, then read the fresh bundle's manifest and `docker pull` the
      runtime image BY TAG, verifying the pulled digest matches the
      manifest's recorded one — a mismatch is a named failure, not a
      silent accept (the supply-chain guard). The probe runs against
      what was built before anything is recorded; the facts land in the
      config only after it answers. Streaming progress (the lanes'
      precedent — a minutes-long build cannot look frozen), idempotent
      (a present bundle and image verify in milliseconds, today's
      behavior), and each missing prerequisite names its skip: git,
      network, disk. Hermetically pinned with a controlled PATH (the
      lanes' fake-uv pattern): a stub git/npm/docker that materialize a
      fake bundle and assert the recorded facts, the probe's call, and
      each named skip. The build's wall time is recorded in this plan
      when it first runs.
- [ ] **C2 — settle applies the delta home.** After the Store merge and
      publish SUCCEED, the parent applies the child's changed paths to
      the host tree through the parent's own write path. The changed
      set is `branch diff_branches <forkedFrom> <child>` (one call,
      measured). The conflict check is a digest snapshot, not a guess:
      at spawn, the arm hashes the tracked tree (14.7 MB here —
      milliseconds); at settle, a path whose host digest changed since
      spawn AND the child changed is a **named conflict** — the file is
      left untouched, both versions named (the host's in place, the
      branch's readable in the face). Everything else applies: Added
      creates (with its parent directory), Modified writes the branch's
      head bytes, Removed removes — deletions follow the merge the same
      as additions, and a host-diverged deletion conflicts the same as
      a modification. The apply rides the approval gate as ONE grouped
      action in ask mode ("apply the merge home: N file(s) — M held at
      conflict"), and proceeds in the yes modes; there is no second
      knob (the delegation knob already kills the whole path: no
      workspace, no delta, no apply). The subagent result carries the
      applied and held lists by path; the calib row's shape is
      unchanged (its outcome is the child's). **The residue is named
      here on purpose:** the Store's `main` drifts from the host as the
      owner edits — the digest check catches exactly the files both
      sides touched, and the child's view of untouched files ages until
      a later increment adds a delta re-import at spawn. Correctness
      holds (a stale base can only conflict, never overwrite); the
      staleness is the named cost.
- [ ] **C3 — close-out: the prevention guards, as tests.** The
      clean-environment live script is kept (`scripts/layerfs-live.sh` —
      the C6 proof, exit-coded) and runs green against the durable home
      with a delegation registered, isolated, settled AND applied home.
      The scrubbed-env pins exist for every env-composed read (the
      view's reads pass with no `LAYERFS_*` in the process — the C6
      lesson, now a test). Every new fixture cites its capture: the
      diff answer's stub is the live wire's bytes, and any shape
      invented before the wire is read names itself pending-live in the
      test. The motivating case's record lands here with its numbers:
      the fleet repo's registration, its first isolated delegation, the
      settle's apply, and the conflict case forced once on purpose (an
      owner edit between fork and settle) to prove the named-conflict
      path on the real tree. Docs ship in the same increments: the
      guarantee change in `surfaces.md` and the feature doc, the
      install's build phase in the install doc, FUNCTIONAL and
      TECHNICAL reading as shipped. The gate green by exit code; the
      status flips in the same commit that lands the work.

## Files to be touched

`src/layerfs_tools.rs` (the manifest-carrying register: the git-declared
walk, the bound on the declared set, staging) ·
`scripts/layerfs-register.mjs` (the manifest argv, the staged init, the
scratch dir's lifecycle) · `src/install.rs` (the build phase: clone,
npm, the tool, pull-by-digest with verification, the probe gate) ·
`src/subagents.rs` (the spawn digest snapshot, the settle's diff +
conflict + apply, the result's applied/held lists) ·
`scripts/layerfs-live.sh` (new: the kept clean-env diagnostic) ·
`src/gui/assets/app.js` (only if the face names held conflicts —
the Work pane's outcome already carries the child's; the apply list
rides the tool result) · `docs/architecture/surfaces.md`,
`docs/features/workspace-history.md`, `FUNCTIONAL-SPECIFICATIONS.md`,
`TECHNICAL-DOCUMENTATION.md` (the guarantee change and the build phase,
in the increments that ship them) · this plan.

## Out of scope

- **Untracked files entering the Store** — declared-only is the design;
  the owner's untracked WIP stays host-side, and the named residue (the
  child's view of it) is C2's stated cost, not a bug to patch here.
- **A delta re-import at spawn** (refreshing the Store's view of the
  host before each fork) — the named follow-up that retires C2's
  residue; it wants its own measurement of the refresh cost first.
- **A registry-published image** — the image is official node, pulled
  and digest-verified; nothing to publish, nothing to build.
- **Live process streaming, the sandbox terminal, a trajectory-style
  rendering** — unchanged from the seamless plan's out-of-scope, for the
  same reasons.
- **New provider machinery of any kind** — embedders, classifiers,
  vector stores, search engines, explainers. The lanes are the
  providers; a capability no lane carries is named unavailable and
  queued for a lane decision, never built as a one-off inside these
  increments.

## Verification

The gate whole and local — fmt, clippy, build, test, read by exit code
(the house's own rule) — with the staged tree compiled in a detached
worktree before every commit (the co-edited-tree discipline). Each
criterion's live record lands in the status section as it lands: C0's
fleet-repo registration and isolated delegation, C1's build timing,
C2's apply and its forced conflict, C3's clean-env run. The probe stays
the standing substrate diagnostic; the live script joins it as the
standing seam diagnostic.

## Status — 2026-10-26 (draft; measurements recorded, awaiting the word)

The three gaps are measured, not estimated, and the measurements are in
this file: the fleet repo's shape (35 GB / 228,916 entries, all
`target/`; 436 files / 14.7 MB declared; `/target` is `.gitignore`'s
first line), the settled file's absence from the host tree, the image's
official-node repo digest, and the diff verb's answer shape against the
live Store. The criteria are ordered by risk — import, build, apply —
and C2's conflict design (the digest snapshot against the fork point)
is written down before any code, because it is the increment that
touches the owner's tree. Awaited: the owner's word to start at C0.

## Linked artifacts

- `docs/plans/2026-10-25-layerfs-delegated-workspaces.md` and
  `docs/plans/2026-10-25-layerfs-child-execution-seam.md` — the
  capability plans; their byte-identity and named-absence pins keep
  holding.
- `docs/plans/2026-10-25-layerfs-seamless.md` — completed `00267a2`;
  this plan's three gaps are its close-out's honest residue, measured
  the morning after it landed.
- `scripts/layerfs-probe.sh` — the standing substrate diagnostic, green
  exit 0 on the durable home (its numbers in the seamless plan's C6).
- The live captures this plan's numbers come from: the C6 run's Store
  (`layerfs-src-store-mvosffiz`), the diff answer for
  `fleet-child-20261026t051905z-906573b2`, and the image's
  `RepoDigests` as inspected 2026-10-26.
