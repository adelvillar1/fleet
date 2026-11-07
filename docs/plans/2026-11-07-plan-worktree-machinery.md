---
status: draft
created: 2026-11-07
updated: 2026-11-07
slug: plan-worktree-machinery
---

# Plan: plan-worktree machinery — the convention, automated, with a hygiene cycle

## Context

The owner's ruling (2026-11-07): plan-based worktrees become the
standing answer to sessions colliding, **implemented with a hygiene
cycle so resources are used wisely** — the machinery scripted, not a
manual dance each session performs from memory.

The convention is already law and already proven:
CLAUDE.md's topology section (the `plan worktrees` block, `c7ac56e`)
carries the rules — event-scoped worktrees added at plan start and
pruned at merge, absolute paths inside, the gate in the worktree's own
`target/`, one deployment at a time, ref-based merges. Two plans ran
through the manual flow cleanly (`reload-reattach`, `owner-inbox`).
What the manual flow cannot do is the **cycle**: the disk guard before
an add, the stale sweep while nobody is looking, the freed-space report
when a worktree closes. The resource reality is the driver — the drive
hit **100% twice this week** (checkpoints at 69 G, pruned 55 G), and
each worktree's `target/` costs 5–8 GiB.

The house already owns two patterns this machinery follows, not
invents: **repo-hygiene**'s closed-list discipline (prune only what the
script's own convention names — `fleet-<slug>` worktrees — report
unknowns, never delete them) and **daily-maintenance**'s cycle (the
LaunchAgent's legs run whether or not the owner remembers). The script
joins the cycle; the cycle makes the wisdom automatic.

Decision-model check: nothing here is a per-item judgment — the verbs
are mechanical (create, report, prune by age and merge state). No sys1
job; said so rather than left unnamed.

## Direction lock

- **One script, four verbs.** `scripts/plan-worktree.sh add|remove|status|sweep` —
  the convention's whole lifecycle in one place, self-documenting via
  `--help`. No fleet CLI verb (this is house tooling, not product).
- **The naming convention is the closed list.** Only `fleet-<slug>`
  worktrees beside the main checkout are this script's to manage;
  anything else in `git worktree list` is reported and never touched.
- **Nothing is deleted while it holds work.** Uncommitted changes or an
  unmerged branch refuse the removal without `--force` — the burn-down's
  stash discipline: nothing lost.
- **The sweep prunes bytes, not history.** The cycle prunes `target/`
  (regenerable by one build) freely on stale worktrees; it removes a
  whole worktree only when the branch is fully merged into master AND
  the tree is stale (21 days, the repo-hygiene guard), with `--dry-run`
  the default for the cycle and `--live` the explicit act.
- **The add is guarded.** A worktree costs 5–8 GiB; below a free-space
  floor the add refuses with the number, named — KI-431's law (a named
  degradation, never a silent one).

## Approach

1. **`scripts/plan-worktree.sh`** — the four verbs:
   - `add <slug> [--base <ref>]` — the disk guard first (refuse below
     20 GiB free, the floor `--floor`-adjustable); create
     `../fleet-<slug>` on branch `<slug>` (default base `master`,
     `--base` for a plan riding a working branch, as the burn-down's
     did); print the three rules at the moment they matter (absolute
     paths, the worktree's own gate, one deployment at a time).
   - `remove <slug> [--force] [--push]` — refuse while the tree holds
     uncommitted changes or the branch is unmerged (unless `--force`);
     `git worktree remove`, delete the local branch, report the bytes
     freed; `--push` deletes the branch on origin too.
   - `status` — every plan worktree: branch, ahead/behind master,
     merged-or-not, `target/` size, last commit age, and the machine's
     free space. The report the owner reads before deciding.
   - `sweep [--live] [--dry-run]` — the cycle: prune `target/` on
     **merged** worktrees stale past 21 days; **remove** a worktree
     only merged AND stale, `--dry-run` printing the actions, `--live`
     performing them; unknown worktrees and anything holding work are
     named, never touched.
2. **The cycle** — `scripts/daily-maintenance.sh` gains the leg:
   `plan-worktree.sh sweep --dry-run` daily, its output in the log
   (named, never silent), `--live` remaining the owner's explicit act.
   The LaunchAgent already carries the schedule; no new timer.
3. **The convention's doc gains the pointer** — CLAUDE.md's topology
   block names the script as the machinery, one line: the manual dance
   is automated, the rules unchanged.
4. **Pinned by a smoke fixture** — `tests/scripts/plan-worktree.sh`
   builds a scratch git fixture (a bare origin + a main checkout +
   one plan worktree), drives the four verbs against it, and asserts
   the outcomes: the guard refusing under a faked floor, the add's
   rules printed, the remove refusing on uncommitted work, the sweep's
   dry-run naming and its live pruning, the freed-space report. Exit
   codes read directly.

## Precedents this plan answers

- **repo-hygiene** (`scripts/repo-hygiene.sh`): the closed list, the
  age guards, the named unknowns, the `--report`/`--live` split — the
  sweep is that pattern pointed at plan worktrees.
- **daily-maintenance** (`scripts/daily-maintenance.sh`): the cycle
  runs whether or not the owner remembers; the sweep joins it as a
  leg, output in the log.
- **KI-446**: the worktrees build in their own `target/` — the
  machinery never shares a `CARGO_TARGET_DIR`, and the add's printed
  rules say so.
- **KI-431**: the disk guard refuses with the number named — a
  degradation stated, never silent.
- **The checkpoint prunes** (the 69 G week): the same resource law the
  cycle serves — the machine's limits are managed by the machinery,
  not by the owner's memory.

## Use cases

- **UC1 — a plan starts.** `plan-worktree.sh add owner-inbox` — the
  guard passes, the worktree and branch exist, the three rules are on
  screen, and the session works inside with absolute paths.
- **UC2 — the drive is tight.** 12 GiB free: the add refuses with the
  number and names the sweep as the way out; the sweep prunes the
  stale worktree's target and the add succeeds after.
- **UC3 — a plan lands.** Merged by ref, then
  `plan-worktree.sh remove <slug> --push` — the worktree pruned, the
  branch gone local and remote, the freed bytes reported.
- **UC4 — the quiet month.** A merged worktree the owner forgot: the
  nightly sweep names it in the log, and one `--live` pass takes it —
  the cycle closing what memory would leave open.

## UX routes

No HTTP routes, no product surface. House tooling: one script, one
maintenance leg, one smoke fixture.

## Acceptance criteria

- [ ] **C1 — the add verb works and guards.** `add` creates the
      worktree and branch (default base master, `--base` honored),
      prints the three rules, and refuses below the free-space floor
      with the number named. Pinned by the smoke fixture.
- [ ] **C2 — the remove verb refuses loss and reports freedom.**
      `remove` refuses uncommitted changes and unmerged branches
      without `--force`; with them clean it prunes the worktree and
      branch and reports the bytes freed; `--push` removes the origin
      branch. Pinned by the smoke fixture.
- [ ] **C3 — status answers before deciding.** `status` lists every
      plan worktree with branch, merge state, `target/` size, commit
      age, and the machine's free space — and names non-plan worktrees
      as not-its-business. Pinned by the smoke fixture.
- [ ] **C4 — the sweep prunes by the closed list.** Dry-run names the
      actions; live prunes `target/` on merged stale worktrees and
      removes only merged AND stale ones; unmerged or unknown worktrees
      are named and never touched. Pinned by the smoke fixture.
- [ ] **C5 — the cycle runs it.** daily-maintenance gains the sweep
      leg (`--dry-run`), logged like the other legs. Pinned by the
      script's own leg list and a smoke run of the leg command.
- [ ] **C6 — the convention names its machinery.** CLAUDE.md's
      topology block gains the one-line pointer to the script. Verified
      by read.
- [ ] **C7 — the live proof.** The real machine: `add` a scratch plan,
      `status` reports it, `remove` frees it — the byte numbers quoted
      in the recap; the cycle's dry-run leg shown in the maintenance
      log.
- [ ] **C8 — the gate.** Shellcheck-clean script, the smoke fixture
      green, and — the change carrying no Rust — the four-command gate
      run whole anyway (fmt/clippy/build/test), exit codes read
      directly.
- [ ] **C9 — the docs.** The script's `--help` is the surface doc;
      CLAUDE.md's topology pointer (C6) is the convention doc; the
      recap quotes the proof.

## Files to be touched

- `scripts/plan-worktree.sh` — the machinery.
- `scripts/daily-maintenance.sh` — the sweep leg.
- `tests/scripts/plan-worktree.sh` — the smoke fixture.
- `CLAUDE.md` — the topology pointer.

## Out of scope

- **A `fleet` CLI verb or any product surface.** House tooling stays in
  `scripts/`; the product never knows the developer's worktrees exist.
- **Shared `CARGO_TARGET_DIR`.** KI-446 forbids it; each worktree
  builds its own, the disk guard prices that honestly.
- **Deployment coordination (a lock or marker).** One-session-at-a-time
  stays a convention; a lock is machinery for a problem the house has
  not actually had.
- **The checkpoints retention leg.** Same resource law, separate
  machinery, already tracked in daily-maintenance's own plan.

## Verification

```sh
# the smoke fixture
tests/scripts/plan-worktree.sh

# the hygiene leg, dry-run by hand
scripts/plan-worktree.sh status
scripts/plan-worktree.sh sweep --dry-run

# the gate — the whole suite anyway, exit codes read directly
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo build --release
cargo test
```

C7's live pass: the real machine's add/status/remove cycle quoted in
the recap, the maintenance log's sweep line beside it.

## Linked artifacts

- `CLAUDE.md` — the topology section's pointer (the convention's home).
- `scripts/daily-maintenance.sh` — the cycle's leg list.
- `docs/recaps/` — the proof.

## Risks

- **The sweep removing a worktree the owner still wanted.** Merged AND
  21-days-stale AND dry-run-by-default AND named in the log first —
  four guards; the residual is a worktree the owner forgot for three
  weeks, which is the definition of the shelf.
- **The free-space floor's number.** 20 GiB is generous against an
  8 GiB cost, but this drive's baseline shifts with checkpoints;
  `--floor` exists and the number is named in the refusal.
- **daily-maintenance running git commands.** The sweep needs the repo
  context; the leg runs it with the script's own repo-root resolution
  (the repo-hygiene pattern), never the caller's cwd.
- **The smoke fixture's git flakiness.** The fixture builds its own
  bare origin and checkout in a temp dir; no network, no machine state
  — the KI-426 lesson (a load-sensitive fixture is a defect) priced in
  from the start.
