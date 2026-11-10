# Session Recap — 2026-11-10: the channel corrected

**Summary.** The failsafe shipped, then the owner found what the local
delivery had skipped: nothing reached GitHub releases, so the Tauri
updater — which reads GitHub's `latest.json` and nothing else — could
never serve it. The session corrected the release behavior itself:
0.6.22 and 0.6.23 were cut retroactively, the LayerFS substrate was
repaired on the owner's word (KI-493 closed), two live-surface defects
the owner saw mid-run were fixed (0.6.23), the auto-update channel
gained a permanent guard in the health loop (0.6.24), and the disk the
cuts filled was swept with a standing fix so no cut accumulates a lane
again. Free space: 124 GiB against the morning's ~94.

## The release behavior corrected

- **The miss.** "Deliver" had been executed as `fleet reload` + a local
  dmg — a dev-time install, not a release. GitHub's page stopped at
  0.6.21 while the machine ran 0.6.22 and 0.6.23. The owner's word:
  *"0.6.22 was never pushed to the github releases, make sure that
  this behavior is corrected"* — and later, the principle: *"if the
  delivery is not done to github the autoupdate on the tauri app is
  bound to fail."*
- **The retroactive cuts.** 0.6.22 from a worktree at its own commit
  (`0222481`), 0.6.23 from HEAD — each through the full runbook: gate
  on a detached worktree, tag, release, signed assets, `latest.json`,
  anonymous read-back. 0.6.22's first attempt was refused by the
  runbook's own notes check: the parallel sessions had anchored new
  changelog sections above it, so the sections are now in descending
  version order.
- **0.6.24 through the corrected pipeline in one session** — the
  first version to go bump → cut → `fleet update` → live face without
  a local install in between.
- **The lesson is recorded where sessions read it**: the release
  runbook memory now says a version bump is not delivered until
  `scripts/release.sh` has cut it, and never report a version
  delivered while `gh release view` 404s.

## The live-surface defects (0.6.23, KI-510/511)

- **The router page blanked mid-run.** `decisionCard`'s facts edit had
  orphaned its rows array; the paint threw on the first `routed` event
  and died after clearing the board — blank exactly while work was
  being routed, fine when idle. The rows append to their list again;
  the harness case paints a routed event and asserts the facts, the
  execution head, and the walk.
- **The history pane showed nothing where a named refusal belonged.**
  The Docker reset had removed every registered project's Store
  volume; the read named it, the pane didn't. It now renders a
  "stores lost to the Docker reset" card — each project, its cause,
  the script noise trimmed — and each line carries a **re-register**
  button (`POST /api/workspace-history/re-register`), pressed live for
  the fleet folder: re-registered as `fleet-2`, history starting empty.

## The LayerFS substrate repaired (KI-493 closed)

The owner's word unblocked the provisioning: the pinned image pulled
(digest verified `sha256:3d27e5c11e57…`), `fleet install` re-spawned
the worker, doctor reads `layerfs-home ok · worker answering · image
present · 6 project(s) registered`, and the live face's services
block read **layerfs up** — the down→up transition the health loop
exists to catch, caught on the real substrate. The probe's own first
smoke test then caught its bug: the lanes marker was `VENDOR.json`,
which the newer browser lane doesn't carry, so healthy cores read
DOWN (KI-509, fixed — the marker is the lane's own `mcp_server.py`).

## The auto-update channel's guard (0.6.24, KI-512)

The health loop's slow tier carries a **release** line: a build whose
version has no release tag reads DOWN — "the auto-update channel will
never serve this build" — with the cut as its repair; a tagged build
whose published feed names an older version reads DOWN as half-cut.
Pinned by `an_uncut_or_half_cut_build_names_itself_and_the_cut` and
`versions_compare_numerically_not_lexically`. The gate's detached
worktree earned its keep on the same cut: it caught a test race the
working tree had hidden twice — the health tests shared one temp home
keyed by pid, so parallel fixture-deletes raced probes; each test owns
a labeled home now.

## The disk (KI-514)

~30 GiB since the morning was the three cuts' release-gate lane
(12 GiB of regenerable cargo cache in `/tmp`) plus 52 cut logs, plus
transient build traffic. Swept through a new `cache-sweep.sh
--gate-lane` tier (declines under a live cargo and while the lane was
written in the last 10 minutes), and the systemic fix: `release.sh`'s
settle now **drops the lane the moment the cut is green** — a settle
that recorded the spend and released nothing was half a settle. Free
space 124 GiB; the updater signing key pair kept.

## The pins

2333 Rust tests, 1096 harness checks across 137 cases; the ledger at
514 rows, well-formed; `plan_check` green. The releases page runs
0.6.21 → 0.6.22 → 0.6.23 → 0.6.24 (Latest), each with the full asset
set, and `fleet update` on the live face answers "nothing to do" at
0.6.24 — the channel reads true end to end.

## Files changed

`src/health.rs` (the release line, the labeled fixtures), the face
(`app.js` — decisionCard, the history card; `server.rs`/`bridge.rs` —
the re-register route), `scripts/release.sh` (the settle releases the
lane), `scripts/cache-sweep.sh` (`--gate-lane`), `CHANGELOG.md` (the
sections reordered; 0.6.22–0.6.24), the ledger (KI-509–514),
`docs/features/{health,routing,subagents}.md`, three version
declarations per cut, the runbook memory (outside the repo).

## Open questions / next steps

- **KI-513, filed and deferred**: the health card's slow-tier lines
  (docker, lanes, release) show only on alternate passes — a fast
  pass replaces the whole set instead of merging. The merge fix rides
  the next face cut.
- **0.6.24's dmg is the only local bundle**; the releases page's dmg
  is GitHub's own — nothing to reconcile.
- The remaining ~0.3 GiB of `/tmp` scratch (test homes, the 2026-11-08
  stale app copy) ages out on reboot, or one more word sweeps it.
- Two co-tenant plan drafts sit in stashes (`grounded-selection`,
  `improvement-modes-inbox`) — parked for the cuts, not lost; the
  current files are on disk.
