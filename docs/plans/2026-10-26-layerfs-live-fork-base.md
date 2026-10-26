# The live fork base: the child sees the host as it stands

The closing-the-seams plan completed with one residue it named itself:
**the child's fork base is the frozen registration import.** A folder
registers once; the Store's `main` holds the tree as it stood at that
moment; every later child forks from that frozen base. The owner's host
edits after registration — days of work — are invisible to every child
until this plan lands. The residue was demonstrated live during C3's
conflict forcing: the child read a two-line file while the host carried
four, because the owner's edit never reached the fork base.

The same plan named the remedy: **a delta re-import at spawn.** This
plan builds it, plus the two smaller leftovers that share its machinery:
the settle's publish can refuse `NoCommit` on a lagging worker (one
retry, the closed KI-321 row's own remedy note), and main-moved paths
the host never touched stay unapplied forever (bring them home when it
is safe — the host's untouched files converge to main).

## Context

Measured and live-proven 2026-10-26:

- **The stale base is real.** Round three of the forced-conflict
  forcing: the child read `notes.txt` as the pre-edit two-line version
  while the host carried four lines — the owner's edit predates the
  child but postdates the registration, and nothing carries it into the
  fork base.
- **The delta is computable cheaply, in git's own terms.** The declared
  import is exactly the tracked tree, so the delta is a GIT tree diff:
  a temp-index `git write-tree` hashes the worktree's tracked content
  in milliseconds (no blob writes for unchanged files), and
  `git diff --name-status <recorded-tree> <current-tree>` names exactly
  the Added/Modified/Deleted paths — no per-file Store calls.
- **The wire carries the apply side.** `fs write` / `fs rm` onto the
  child's branch puts the host's current content into the fork base;
  the settle's digest check then works against a base that matches the
  host, which is what makes its verdicts exact.
- **The publish refusal is a race the row already characterized.**
  KI-321's first instances: a merge lands, the publish refuses
  `[NoCommit]`, a later read shows the work absorbed. The row's remedy
  note: one retry before surfacing.

The owner's standing rules bind as before. **Anti-Hermes:** the sync is
not a knob — it runs at spawn because a child working from a stale base
is a defect, not a preference. **Lanes as providers:** nothing here
needs a lane — git plumbing, fs verbs and digests are deterministic.
**Integral, not bolt-on:** the sync rides the spawn that already exists;
the reconciliation rides the settle that already exists; the face's
merge states already show what happened.

## Approach

One increment's spine, two smaller legs. The spine: the registration
records the git tree it imported; the spawn computes the host's current
tree the same way, diffs, and writes the delta onto the child's branch
before the child's first turn — so the fork base is the host AS IT
STANDS. Main-newer paths (another child's merge the host hasn't
received) are skipped in the child's favor of main, and instead ride
the settle home: the apply reconciles the host to main for paths the
host never touched. The publish retry is one line in `merge_back`.

The ordering rule that makes this safe: the host is the truth for the
child's starting point; main is the truth for everything the host
hasn't touched; the digest snapshot is the truth about what the owner
touched during the delegation. Each writer only ever writes the side it
owns.

## Acceptance criteria

- [ ] **C0 — the delta measured, the tree-id trick proven.** On the
      fleet repo: the temp-index `git write-tree` (index copy,
      `read-tree HEAD`, `add -u`, `write-tree`) answers in
      milliseconds and changes iff tracked content changes; the
      recorded-tree → current-tree diff names Added/Modified/Deleted
      correctly against a hand-made delta; and the one expensive
      unknown is measured — `fs read main <path>` per delta candidate
      (a handful of calls) to decide main-newer vs host-newer. The
      registration's record grows the imported tree id, written at
      registration, read fresh at spawn. Nothing else in this plan
      freezes until these numbers exist.
- [ ] **C1 — the fork base is the host as it stands.** At spawn, for a
      git-registered folder: compute the current tree, diff against the
      recorded tree, and for each delta path carry the host's current
      content onto the child's branch (`fs write` for Added/Modified,
      `fs rm` for Deleted) — EXCEPT main-newer paths (main's version
      differs from the recorded tree: another child's merged work the
      host hasn't received), which are left for the settle's
      reconciliation. The delta is bounded by the bound already in
      place and by a named cap (a delta past it is the named fallback:
      the child runs on the frozen base, one notice — the pre-C0 world,
      named rather than silent). Non-git folders: no sync, named once,
      unchanged behavior otherwise. The registration's recorded tree
      advances at every apply-home and every sync, so consecutive
      delegations diff against what the last one actually reconciled.
      Pinned: a fixture repo whose host changes between registration
      and spawn — the child reads the post-change content; a
      main-newer case — the child reads main's version and the settle
      brings it home.
- [ ] **C2 — the settle reconciles, and the publish retries.**
      `merge_back` retries the publish ONCE after a short pause when it
      refuses `[NoCommit]` (the lagging-worker race the row named); a
      second refusal is surfaced exactly as today. The apply-home grows
      the reconciliation: main-moved paths (skipped in C1) whose host
      hash still equals the spawn hash — the host never touched them —
      are brought home alongside the child's own applied paths, in the
      same grouped approval, so the host converges to main for
      everything the owner did not touch this window. The result's
      notes name all three lists: the child's work applied, main's work
      reconciled, the owner's paths held.
- [ ] **C3 — close-out: the residue retired, live.** The round that
      failed during the last plan's forcing now passes: the owner edits
      a tracked file after registration, the delegation runs, and the
      child READS the post-edit content — the stale-base demonstration
      becomes the stale-base retirement. The standing
      `scripts/layerfs-live.sh` grows the same check (the fresh folder's
      second delegation sees the first delegation's applied file), so
      the proof is repeatable, and runs green. Docs in the same
      increments: the feature doc's named residue retires into the
      shipped behavior, surfaces.md's paragraph matches, FUNCTIONAL
      reads as shipped. The gate green by exit code; the status flips
      in the same commit that lands the work.

## Files to be touched

`src/layerfs_tools.rs` (the tree-id helpers beside the declared import,
the registration's recorded tree, the sync at spawn, the publish retry,
the report's third list) · `scripts/layerfs-register.mjs` (the imported
tree id in the answer) · `src/subagents.rs` (the spawn's sync pass, the
apply's reconciliation list) · `scripts/layerfs-live.sh` (the
second-delegation check) · `docs/features/workspace-history.md`,
`docs/architecture/surfaces.md`, `FUNCTIONAL-SPECIFICATIONS.md` (the
residue retires) · this plan.

## Out of scope

- **Untracked files entering the child's view** — declared-only stays:
  the sync diffs the tracked tree, and an untracked file's absence from
  the child's view is the design's named edge, not a gap this plan
  papers over.
- **The substrate's branch bookkeeping** (branches carrying
  `head: None` after merges land on their layers, publish `NoCommit`
  on an integration branch whose work is demonstrably absorbed) — the
  vendor conversation the closing plan opened; this plan's retry is
  the fleet-side hedge, not the fix.
- **Non-git folders' sync** — the whole-tree walk has no cheap tree id;
  their children run on the frozen base with the named notice, exactly
  as today.
- **Live process streaming, the sandbox terminal, new provider
  machinery** — unchanged from both predecessor plans' out-of-scope.

## Verification

The gate whole and local, read by exit code, the staged tree compiled
in a detached worktree before every commit (the co-edited-tree
discipline). C0's numbers land in this file as they are measured. C1
and C2's live records land as they land: the stale-base retirement (the
round-three scenario, now passing), the main-newer reconciliation, the
publish retry observed once in the wild or forced by a paused worker.
The standing `scripts/layerfs-live.sh` runs green with the grown check.

## Status — 2026-10-26 (draft; the residue live-proven, the remedy scoped, awaiting the word)

The three leftovers this plan closes are the ones its predecessor named
on the way out: the delta re-import at spawn (demonstrated live in the
conflict forcing's third round), the publish retry (KI-321's own remedy
note, kept when the row closed), and the main-moved reconciliation (the
gap the C2 classification's skip arm opened, stated in that criterion).
The machinery is measured where it was uncertain — the tree-id trick is
git plumbing, the delta is a name-status diff, the apply side is the
wire's own fs verbs. Awaited: the owner's word to start at C0.

## Linked artifacts

- `docs/plans/2026-10-26-layerfs-closing-the-seams.md` — COMPLETE; its
  C2 criterion named this plan's spine as its residue.
- `docs/plans/2026-10-25-layerfs-seamless.md` — COMPLETE; the
  substrate's provisioning and the face.
- `docs/known-issues.md` KI-321 — closed; its remedy note is this
  plan's publish retry.
- `scripts/layerfs-live.sh` — the standing proof this plan grows.
