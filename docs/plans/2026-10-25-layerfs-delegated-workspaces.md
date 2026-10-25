---
status: completed
created: 2026-10-25
updated: 2026-10-25
slug: layerfs-delegated-workspaces
---

# LayerFS workspaces for delegated children

## Context

Delegated children run in the parent's working folder — `FileAccess`
is inherited, so every child of a fan-out shares one tree, and the
owner has real incidents of agents stepping into each other's code
(the interleaved-hunk commit that swept another session's work, this
repo's own ledger). LayerFS
(<https://github.com/Ephemeral-AI-Lab/layerfs>, MIT, v1.1.0) is the
substrate built for exactly this: every agent mounts a complete
writable Workspace over one shared committed history, writes land in a
per-Workspace overlay, and a Commit is a compare-and-swap publish —
two agents changing the same file produce a named `Conflict`, never a
silent overwrite. Its dedup is measured (19.9× vs Btrfs+zstd over 157
snapshots; a real 8-agent dsh run held 4.93 TB of snapshot history in
8.9 GB).

The owner's ruling, 2026-10-25: worth adopting, because the incidents
are real, and cheap, because the design effort is already spent —
LayerFS's core is itself Rust, and its `layerfs` command is one SDK
call per command with JSON answers, built for an agent whose only tool
is Bash. The dsh-layerfs plugin (TypeScript, for the DeepSeek Harness)
proves the integration pattern: a host that sets up the environment,
working rules carried in the agent's brief, and records read back for
display. Fleet ports the *pattern*, not the code: LayerFS joins the
lanes' precedent — an external tool with a pinned identity, driven as
a subprocess, absent by name when it is not there.

## Approach

**Per-delegation workspaces first, not per-tool-call.** The incidents
are fan-out incidents. One child, one Workspace, for the child's whole
life: mount at spawn, commit-or-discard and unmount at completion,
always — including failure, in the same cleanup path that already
returns the partial result (subagents.rs decision 5, never silence).
The dsh default cadence (mount/exec/commit per Bash call) is heavier
than fleet's loop needs and is a later increment if ever.

**The seam already exists.** A child is an `Agent` over a `Session`
with an inherited `FileAccess`; `FileAccess::Folder(path)` already
makes bash spawn at a folder and refuse paths that leave it. The arm
sets the child's access to the mounted Workspace's path. No new access
model, no new tool: the four built-ins work inside the mount through
ordinary filesystem calls, which is what a LayerFS FUSE Workspace is
for.

**The driver is a thin CLI spawn, never an SDK port.** A new
`src/layerfs_tools.rs` spawns `layerfs <family> <call>` (resolved
through the child PATH like the lanes' `python3`), parses the JSON
answer, and maps refusals to typed errors — the `mcp.rs` shape. No
Node runtime enters fleet's build (hard rule 6): the CLI's own
implementation language is its business, as python3 is the lanes'.
Availability is its own once-per-process gate, orthogonal to every
other key, with the named absence everywhere — the `sys1_extracts()`
pattern from the extraction leg, whose E0-first discipline this plan
copies whole: **measure the machine before the driver compiles against
the documented shape** (the deck-suggest lesson).

**The rules ride the brief, and only the parent publishes.** dsh's
working rules port verbatim in spirit: one Workspace per agent; the
child commits to its own Branch and never publishes; merge and
publish to `main` are the parent's calls; a `Conflict` is a
first-class answer the parent reads, never something to route around.
The Branch and Workspace ids land in the delegation's calibration row,
so the history is reconstructable from the log (hard rule 5).

**Opt-in by naming a project; default OFF for everything else.** The
CLI's settings are `LAYERFS_*` env names; fleet's own knob is one:
a run (or session) that names a LayerFS project gets workspaces for
its delegations; a run that does not is byte-identical to today's —
the what-the-model-saw test pins that. Git stays the durable spine:
LayerFS's Store is Disposable-only (no crash durability, by the
project's own law), so nothing lands in a Workspace that is not also
committed to git on publish. The anti-hermes test holds: the owner
never needs to know the knob exists unless a project turns it on.

**macOS is the host to prove, not assume.** Store calls are Linux-only;
on macOS they ride a long-lived Linux worker container
(`@layerfs/runtime`), and Sandboxes are Docker containers everywhere.
So the feature requires Docker Desktop on this machine — the heaviest
dependency fleet has ever carried, and the reason E0 measures the
worker lifecycle (start, secret, idle exit, reclaim) before any of
this compiles.

## Acceptance criteria

- [x] **C0 — the probe gate** *(measured 2026-10-25 — the full ladder
      and the worker lifecycle below, exit 0)*.
  `scripts/layerfs-probe.sh` measures the
      real machine before the driver exists: init a scratch project,
      fork, mount, exec, commit, unmount, and the macOS worker's
      lifecycle (start, a second client, idle exit, reclaim), with
      per-step latency recorded. Every difference from the documented
      contract is recorded in this plan's Approach before
      `layerfs_tools.rs` parses its first real answer. Blocked-until-
      measurable, like the extraction leg's E0: no Docker, no
      measurement, no driver.
- [x] **C1 — the driver.** `src/layerfs_tools.rs` spawns the CLI and
      parses JSON into typed results and typed refusals; availability
      is its own once-per-process gate with the named absence; unit
      tests against a stub `layerfs` executable (the double pattern)
      pin the parse, the refusal shapes, and the timeout.
- [x] **C2 — the delegation arm** *(landed, stub-pinned, and the one
      live run executed and recorded below; its two findings: the
      branch-grammar fix (KI-315, closed same day) and the
      child-execution seam (KI-314, the next increment) )*. An opted-in run's child mounts one
      Workspace at spawn, runs with `FileAccess::Folder` at the mount
      path, and on every completion path — success, failure, turn-cap
      — commits-or-discards per policy and unmounts; only the parent
      merges and publishes; a same-file collision answers `Conflict`
      to the parent as a tool result. The delegation calibration row
      carries the Branch and Workspace ids. Pinned by integration
      tests with the stub; one live run recorded in the increment's
      report.
- [x] **C3 — the boundary** *(byte-identity, doctor, wire refusal, and
      the checkout pin — `CHECKOUT_PIN` — all landed)*. Default OFF: a run that names no project
      is byte-identical in what the model sees (the reconstruction
      test). `fleet doctor` names the CLI's presence or absence. The
      checkout identity fleet pins is named (a VENDOR-style pin), and
      a CLI that answers a different wire version is refused by name,
      not parsed hopefully.
- [x] **C4 — close-out.** The full gate green by exit code; the
      ledger carries whatever the live run opened; this plan's status
      updates in the same commit that lands the work.

## Files to be touched

`scripts/layerfs-probe.sh` (new: E0's harness) ·
`src/layerfs_tools.rs` (new: the driver, the gate, the typed errors) ·
`src/subagents.rs` (the arm: mount at spawn, cleanup at completion,
the row's ids) · `src/agent.rs` (only if the Folder access needs a
hook; the enum already exists) · `src/doctor.rs` + `src/main.rs` (the
CLI's presence probe) · `src/settings.rs` (the one knob) ·
`tests/integration.rs` (the stub-driven arm tests) ·
`docs/known-issues.md` (whatever E0 and the live run open).

## Out of scope

- **Per-tool-call workspaces** (dsh's mount/exec/commit cadence) — a
  later increment against measured need, not the incidents'.
- **The parent's own session moving into a Workspace** — only
  delegated children move; the owner's working folder and fleet's own
  session logs/memory stay where they are.
- **Replacing git or becoming the durable record** — Disposable-only
  Store by LayerFS's own ruling; git commits on publish remain the
  spine.
- **Work/History tabs** (dsh's display layer) — records ride the
  session log and the calibration row first; a face comes after the
  arm is proven, per UX-is-co-equal the surface lands with the
  increment that needs it, not before.
- **Windows/WSL2, the Magpie MCP route, porting the dsh plugin** —
  macOS + Docker is the host this plan proves.
- **Retiring the turn-checkpoint store** — named overlap (both capture
  a workdir tree); this plan does not touch it, and a later ruling
  decides whether an opted-in project needs both.

## Verification

E0 is measured, not tested; its numbers land in this plan. C1 is
pinned by unit tests over a stub CLI executable (the mock-mcp double
pattern: a pinned port/file, fail-closed). C2 is pinned by
integration tests driving a delegation through the stub end to end —
mount, scoped access, commit-or-discard on a failing child, unmount,
the `Conflict` answer — plus one live run against a real Store and
Docker, recorded like the extraction leg recorded its first call. C3
is the what-the-model-saw byte-equality test (both declaration paths,
per the reconstruction rule) and the doctor card. The gate runs whole
before every commit, exit codes read directly.

## Status — 2026-10-25 (implemented; the live wire measured, the seam named)

- **C0 — measured end to end.** The environment was provisioned the
  same day and `scripts/layerfs-probe.sh` ran the whole ladder
  (exit 0; `tmp/layerfs-probe/result.json` = `{"status":"ok", …}`):
  init through `@layerfs/runtime`'s `start_init` — the sanctioned
  macOS path, 406 ms, volume `layerfs-fleet-probe-store-mvo74iik` —
  then fork **55 ms**, sandbox create **288 ms**, mount **62 ms**,
  exec **104 ms**, write **60 ms**, commit **63 ms**, unmount **61
  ms**, close **201 ms**; two Store clients in flight together **60
  ms** (both exit 0); and the worker/sandbox lifecycle — worker start
  **581 ms** cold / **58 ms** warm, sandbox **idle exit 121,225 ms**
  after unmount (the documented two-minute self-end, observed), and
  **reclaim 391 ms with the same sandbox number reused**. The
  provision chain, in order: the pinned checkout at `ad78914` with
  `npm install` in `api/` and `api/agent/cli`; the sealed Linux
  bundle built by `tools/worktree.py` (`build-linux-25b0bed7197842688bed6e1364c9b25e`
  — `layerfs-daemon`, `layerfs.node`, `worker/server.js`, the CLI,
  182 hashed files, `manifest.json` status `sealed`); the runtime
  image `sha256:3d27e5c11e5786e309ec3e03f93ae536eb36e6e5eb3714d5eb3300a36157add0`
  (node:24.21.0-bookworm — the same digest dsh's Dockerfile pins);
  the home's worker started by `@layerfs/runtime` (endpoint
  `<home>/worker/endpoint.json`); init by `start_init`. KI-313
  closes with this record.
- **C1 — landed.** `src/layerfs_tools.rs`: the CLI-spawn driver with
  the once-per-process gate, the `SUPPORTED_WIRE` refusal (fleet
  drives 1.1.0; the CLI breaks its API between minors), the typed
  error custody (refusal/timeout/decode/spawn/failed), and the
  opt-in knob's grammar. Eleven unit tests over the stub pin the
  grammar and parse, Conflict-as-answer, refusal codes, the wire
  refusal, decode-with-bounded-body, the budget's one attempt, the
  named absence, the knob, the branch-name fold against a real
  session stamp, the pre-spawn refusal of an illegal branch, and the
  checkout pin's shape.
- **C2 — landed; the live run ran.** The arm
  in `src/subagents.rs`: mount at spawn (branch
  `fleet-child-<session>`, folded into the wire's grammar by
  `child_branch_name`),
  `FileAccess::Folder` at the mount path, settlement at every
  completion path (completed/capped → Commit, then unmount; failure →
  discard), the parent's merge `main` ← child plus publish as the
  harness's act — the child never sees a layerfs call — a `Conflict`
  rendered with its paths and the branch kept, and the branch/
  Workspace ids on the delegation row (`workspace_branch`,
  `workspace`). Three integration tests drive a delegation through the
  stub: the child's `pwd` lands at the mount and never at the
  parent's tree (argv grammar checked call by call, Commit ordered
  before unmount); default-off never consults the driver (no argv log
  exists); a same-file collision renders `CONFLICT with main on 1
  path` to the parent while the outcome stays the child's success.
  The row-ids and render are pinned by
  `the_workspace_rides_the_row_and_the_render`. **The live run:**
  `a_layerfs_live_driver_roundtrip_against_the_real_wire`
  (`#[ignore]`d; run 2026-10-25 against the provisioned env with
  `cargo test --test integration a_layerfs_live -- --ignored
  --nocapture`) — open (fork + sandbox + mount) **580 ms**, settle
  (Commit + unmount + close + merge into main + publish) **361 ms**,
  the in-sandbox write read back **from `main` after publish**, and
  the host/sandbox namespace boundary asserted — the host meets
  ENOENT at the mount the sandbox serves. Two findings landed with
  it, same day: **(a)** the wire's `HistoryName` grammar refused the
  arm's `fleet/<session>` branch names (slash + uppercase stamp)
  while the stub accepted them — a real delegation would have failed
  at its first fork; fixed by `child_branch_name`'s fold plus the
  pre-spawn `history_name_ok` refusal, grammar tests pinning both
  (KI-315, closed same day). **(b)** the mount is sandbox-local, so
  an opted-in child's host-side processes cannot work at it — the
  child-execution seam (dsh routes file ops through `layerfs fs.*`
  and command runs through `workspace exec`) is the next increment
  (KI-314, open).
- **C3 — landed, pin included.** `CHECKOUT_PIN` names the checkout
  the way a VENDOR.json names a carry —
  `github.com/Ephemeral-AI-Lab/layerfs@ad78914e36b258b360daf13aec164b5655814689 (wire 1.1.0)`
  — carried on `fleet doctor`'s layerfs line and pinned by
  `the_checkout_pin_names_the_commit_and_the_wire`; the gate's wire
  refusal keeps the two halves honest when either moves. The
  byte-identity holds two ways: no knob → no spawn → no block (the
  integration test asserts the absence itself), and `fleet doctor`'s
  new line reads `off (FLEET_LAYERFS_PROJECT, unset · …)` or FAILs
  only when the knob is on over a machine that cannot answer —
  28 doctor tests. The settings surface names the env-only knob
  without a value. The wire-version refusal is C1's gate.
- **C4 — close-out: green by exit code.** `cargo fmt --all --check`,
  `cargo clippy --all-targets -- -D warnings`, `cargo build
  --release`, `cargo test` — 0, 0, 0, 0; **1706 passed, 0 failed,
  3 ignored** (the live-API e2e, the `#[ignore]`d LayerFS live roundtrip
  — which RAN on demand and passed — and one pre-existing). The ledger
  carries KI-313 (closed with the provision record), KI-314 (the
  child-execution seam, open), KI-315 (the branch-grammar defect,
  closed same day), and this status lands in the same
  commit as the work. The road to the green gate is part of the
  record: two process-global test bugs of this arm's own making were
  caught by the suite — a stub deleted while the gate's cached path
  still pointed at it (panic under `AMBIENT_LOCK` poisoned the shared
  mutex for the env tests behind it), and the knob as process-wide
  env letting a concurrent test's delegation mount a workspace inside
  another test's run (observed: a child pwd'd into a foreign mount).
  The fixes are structural — one never-deleted stub per process, the
  knob as a per-thread override production never sets, assertions
  outside the lock — and the caught-by-the-suite story is the
  standing lesson's eleventh form.

## Linked artifacts

- LayerFS — <https://github.com/Ephemeral-AI-Lab/layerfs>; the agent
  guide (`api/how-to-use/agent.md` at the pinned tag) is the CLI's
  contract; the Store concurrency model and the Disposable-only ruling
  are why git stays the spine.
- dsh-layerfs —
  <https://github.com/Ephemeral-AI-Lab/dsh-layerfs>: the integration
  pattern (host, brief rules, records) this plan ports.
- `docs/plans/2026-10-19-sys1-extraction-leg.md` — the E0-first
  discipline, the once-per-process availability gate, and the stub
  tests this plan copies.
- `src/subagents.rs` — the five decisions (loop reuse, same approver,
  depth two, never silence, the recorded row) the arm must not break.
- `src/agent.rs` `FileAccess::Folder` — the seam the child rides.
