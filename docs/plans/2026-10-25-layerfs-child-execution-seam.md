---
status: active
created: 2026-10-25
updated: 2026-10-25
slug: layerfs-child-execution-seam
---

# The child-execution seam: LayerFS-backed tools for opted-in delegations

## Context

KI-314, filed by the delegated-workspaces plan's own live run: a
LayerFS mount lives in the **sandbox's namespace** — the host meets
`ENOENT` at `/workspaces/N` while an in-sandbox `workspace exec` reads
and writes the same path. Fleet's children spawn host-side, so the
first arm's `FileAccess::Folder` at the mount confined an opted-in
child to a path only the sandbox can serve: isolated, but unable to
work. The ledger named the closure path: design against dsh's proven
pattern — file operations through the SDK's branch verbs, command runs
through `workspace exec` — "their minimalist preset: file ops via
`layerfs fs.*`, commands via `workspace exec`."

**The wire was reconciled before this plan existed** (C0's discipline,
measured on this machine 2026-10-25, three findings):

1. **A fresh mount sees the branch's current head** — a mount created
   after an SDK `fs write` returned the new content on its first exec.
2. **A held mount pins its base** — an SDK `fs write` after the mount
   stayed invisible to that mount's execs (it kept returning v1).
3. **Command writes publish only through Commit** — a file written
   inside an exec answered `NoSuchPath` to an SDK read until
   `workspace commit`, after which the SDK read returned it; a fresh
   mount then saw every prior commit.

⇒ The only coherent cadence is **per-command**: every bash call mounts
the branch's current head, runs, commits, unmounts — which is
verbatim dsh's own doctrine (their README's idea-01: "each Bash call
mounts a Workspace, runs, commits and unmounts"). File operations
need no mount at all: the SDK's `fs.*` verbs read and write the branch
directly, and the single child is the branch's only writer, so
command/file sequences cannot interleave.

Two more facts from the same runs, load-bearing for the contract:

- **`exec`'s cwd is the workspace root**, and **absolute paths inside
  commands resolve against the CONTAINER root** — they never touch
  the workspace. So the child's bash contract is the ordinary
  terminal one: your working directory is the project root; shell
  commands use paths relative to it (exactly how any agent works in
  any checkout).
- The **`layerfs fs.*` verbs already mirror fleet's built-ins**:
  bounded windowed reads, `fs edit` with exactly-one-match (fleet's
  anchored-edit contract), `fs grep`/`fs glob`, `fs write` reading
  bytes from stdin (binary included).

## Approach

**A third `FileAccess` variant, set only by the opted-in arm.**
`FileAccess::LayerFs` carries the child's context — project, branch,
the sandbox handle — and replaces `Folder`-at-mount. When
`FLEET_LAYERFS_PROJECT` is unset the variant never exists, so every
byte-identity guarantee the last plan pinned stays pinned by the same
tests.

**The dispatch routes by tool class at the seam that already
exists** (`run_builtin`'s access match in `agent.rs`):

- **fs-class → the driver**: `bash` runs through `run_command`
  (mount → exec → commit → unmount, one attempt each, typed custody,
  an unchanged-state commit read as success — the CLI creates no
  Commit for unchanged state, by its own law); `read_file`, `list_dir`,
  `write_file`, `edit`, `grep`, `glob` run through new driver verbs
  over the branch (`fs read/list/write/edit/grep/glob`). The path
  rule mirrors `Folder`: a view path resolves under the branch root,
  and a path that escapes it is refused by name.
- **Everything else keeps host behavior** (`use_skill`, `remember`,
  `subagent`, `todo`, web): it never touches the child's tree.
- **Where no honest route exists, a named refusal** — never a silent
  wrong-tree answer: `apply_patch` (no CLI verb; a translation
  through `fs apply` is a later increment if a role needs it),
  `repo_map` (it would map the host tree the child does not live in),
  the vision image loader (CLI reads are text-only; branch images
  cannot be materialized honestly), and the document/send tools (same
  reason). Each refusal names itself and the reason.

**The child sees the same declarations and a terminal contract.** No
spec changes: the prompt's working-surface section names the branch
view as the working directory and states the shell rule (commands run
at the project root; prefer relative paths inside shell commands) —
the Folder section's voice, restated for a view that lives in the
sandbox. Model-visible text changes only in opted-in runs.

**Settle sheds the held mount.** Commands checkpoint themselves (the
dsh doctrine — every step restorable), so there is no overlay left to
settle at completion: the arm closes the Sandbox, and the existing
merge/publish/`Conflict`/row-id policy runs unchanged. A failed
child's work stays on **its branch** — isolation is the protection,
and the parent's merge gate is still the only door to `main`; the
first arm's discard-of-overlay is superseded by per-command commits
and is recorded as such.

**Two deliberate approximations, named here so they are decisions:**
the write-time vocabulary guard keeps the host cwd as its root (it is
advisory drift detection over the same codebase the branch derives
from, not a boundary — the boundary is the path rule), and
`read_marker` returns no marker for a LayerFs read (the branch is
child-exclusive, so the read-then-edit staleness it guards cannot
occur, and `fs edit` keeps its own exactly-one-match).

## Acceptance criteria

- [ ] **C0 — the cadence and the verbs.** `src/layerfs_tools.rs`
      gains `run_command` — mount → exec → commit → unmount per call,
      each step one attempt with the driver's typed custody — and the
      branch fs verbs (`read`, `list`, `write`, `edit`, `grep`,
      `glob`) with the view path rule (escape refused by name). Stub
      tests pin: the argv **order** around one bash call (mount, exec,
      commit, unmount), the fs-call argv shapes, an unchanged-state
      commit reading as success, the refusal shapes, and the escape
      refusal.
- [ ] **C1 — the arm.** `FileAccess::LayerFs` set at spawn in place
      of `Folder`-at-mount; the dispatch routes the fs-class to the
      driver, non-fs tools host-side, and the no-route tools to named
      refusals; the prompt names the working surface and the shell
      rule; settle closes the Sandbox with merge/publish/conflict and
      the row ids unchanged. Default off stays byte-identical (the
      no-knob integration test and the reconstruction test keep
      pinning it), and a stub integration pins the **coherence
      sequence** by argv: an `fs write`, then a bash call's
      mount/exec/commit/unmount, then an `fs read` seeing what the
      command wrote.
- [ ] **C2 — the live run.** An `#[ignore]`d test against the
      provisioned wire asserts the three reconciled findings (fresh
      mount = current head; a held view pins; command writes publish
      only through Commit) and runs the child-shaped workflow —
      `fs write`, mount sees it, exec mutates, commit, `fs read` sees
      the mutation — recording the per-command overhead in this plan.
- [ ] **C3 — close-out.** The full gate green by exit code; KI-314
      closed with this plan's evidence; `status: completed` in the
      same commit that lands the work.

## Files to be touched

`src/layerfs_tools.rs` (run_command, the fs verbs, the path rule,
the `WorkspaceHandle` the variant carries) · `src/agent.rs` (the
variant and every `FileAccess` match: prompt, execution dispatch,
write-time guard root, vision, document tools, `read_marker`,
checkpoint root) · `src/subagents.rs` (the arm: variant instead of
Folder-at-mount; settle sheds the mount) · `tests/integration.rs` +
the driver's unit tests (cadence, coherence sequence, byte-identity —
the first arm's mount-at-spawn assertions move to the per-command
shape) · `docs/known-issues.md` (KI-314 closes).

## Out of scope

- **Materializing the branch onto the host** (no CLI export verb
  exists; composed `fs read` walks are not a tree sync).
- **`apply_patch` through `fs apply`** — named refusal until a role
  declares the need; then its own increment.
- **Translating absolute paths inside shell commands** — the terminal
  contract instead (cwd = project root), which is what every agent
  already does in every checkout.
- **The parent's own session** (still host), Windows, the Work/
  History display, and per-tool-call mounts for *file* operations
  (file ops never mount — only `bash` does, per command).

## Verification

Driver unit tests over the stub (argv order, shapes, refusals, the
path rule); the stub integration's coherence sequence; the live run
(`LAYERFS_*` + `FLEET_LAYERFS_BIN` set, `cargo test --test
integration <name> -- --ignored --nocapture`) asserting the three
wire findings and the workflow, its timings recorded here; then the
gate's four commands by exit code, the ledger, and the status flip.

## Linked artifacts

- `docs/plans/2026-10-25-layerfs-delegated-workspaces.md` — the plan
  whose live run filed KI-314; its C2 arm is this plan's starting
  point and its settlement policy is amended here.
- `docs/known-issues.md` — KI-314 (this plan closes it), KI-313 (the
  environment this plan's live run uses), KI-315 (the grammar rule
  `child_branch_name` already enforces for this plan's branch names).
- dsh-layerfs — the cadence this plan ports: mount/exec/commit per
  Bash call, file ops through the SDK, commands through
  `workspace exec`; their minimalist preset is the proven precedent.
- The wire findings above — measured on this machine 2026-10-25 by
  the probe's sibling experiments; re-asserted by C2's live run so
  they cannot silently drift.
