---
status: draft
created: 2026-10-27
updated: 2026-10-27
slug: projects-repo-hygiene
---

# Projects repo hygiene: the measured sweep, and the permanent habit

> **Status: DRAFT** — awaiting the owner's word to start at C0.

The fleet cleanup (2026-10-26, `4612e8d`) fixed one repo's bloat with a
standing rule: a daily sweep that prunes build artifacts past 7 days,
retires `tmp/` past 14, self-commits the memory churn, and NAMES
orphans it does not understand. This plan applies the same pattern to
every repository on the machine — first the measured one-time sweep of
what already accumulated, then the permanent mechanism so it never
accumulates again.

## Context

Measured 2026-10-27 across `~/Projects` (39 directories; the survey
script's output is the baseline, C0 records it in full):

- **~64 GB total**, of which ~30 GB is rebuildable artifact mass and
  ~3.6 GB is git object stores.
- **Class 1 — stale build artifacts, ~14 GB.** Repos whose last commit
  is a month or more old, still carrying regenerable trees:
  cruisingintelligence (node_modules 2.1 G, .next 750 M — last commit
  Sep 22), commissiontracker (.next 2.0 G, node_modules 1.2 G — Sep 21),
  deepseek-harness (node_modules 2.0 G — Oct 13), elo-scenario-lab
  (node_modules 743 M, .cache 115 M, `.venv` 335 M **and** `venv`
  383 M — two virtualenvs, the May one stale), TTESS (1.7 G — Sep 28),
  pmocockpit (1.1 G — Sep 12), dnd-vtt (459 M — Sep 21), simcityclone
  (dist 562 M — Aug 26), catan (264 M), threeui (238 M), helmsman
  (290 M), pampa-wineclub (323 M).
- **Class 2 — one worktree's frozen build: 7.7 GB.** `fleet-remote` is
  a fleet worktree (`.git` → `fleet/.git/worktrees/fleet-remote`,
  last commit 2026-10-20) whose `target/` is an Oct 18 build.
- **Class 3 — git object stores, 3.6 GB, two diseases.**
  (a) *Neglect*: simcityclone holds **819 MB of 1,194 loose objects,
  never packed** (gc packs them, ~500 M back); deepseek-harness has
  1,489 loose beside a 285 M pack. (b) *Media in history*: the
  cruisingintelligence pack is **1.45 GiB** whose top blobs are a
  65 MB InsightGuide, a 52 MB `.wav`, a 47 MB `.docx`; the dnd-vtt
  (363 MiB) and mahjong-vtt (260 MiB) packs are dominated by
  `docs/e2e-review/...` HTML files of 29–38 MB each — the same
  review-artifacts-in-git disease fleet's ledger already names. gc
  cannot shrink reachable blobs; only a history rewrite would.
- **Not garbage, named so it stays named:** fleet's own 14 G `target/`
  (active working set under the existing 7-day sweep), the active
  repos' working sets (design-canvas, synapticweb, mahjong-vtt — all
  committed this week), and `synapticweb-tmp` (292 KB husk — negligible,
  listed for the owner to delete whole if desired).
- The 30 lines of uncommitted churn in cruisingintelligence (and every
  other repo's churn) are source work: **never swept**, whatever else
  happens around them.

The fleet precedent binds as before: artifacts on a FIXED LIST are
regenerable and get pruned by age; anything not understood is NAMED,
never deleted; `generated/`'s lesson — check what a directory is
before judging it dead — is why the sweep list is closed, not a
pattern match.

## Approach

Three legs, in dependency order.

**The one-time sweep (C1–C2)** removes exactly the measured artifact
dirs above — never `rm -rf` by pattern, always the named list with a
size recorded before and after — and runs `git gc` on the two
loose-object repos. Churn, source, and `.git` internals beyond gc are
untouched.

**The permanent mechanism (C3)** extends the existing
`scripts/daily-maintenance.sh` (the 09:20 LaunchAgent) with a
repo-hygiene pass over `~/Projects`: for every git repo whose last
commit is older than 21 days, prune artifact dirs — from the same
closed list (`target`, `node_modules`, `.next`, `dist`, `build`, `out`,
`coverage`, `.turbo`, `.venv`, `venv`, `tmp`) — whose **directory mtime
is older than 14 days**. The mtime guard is what keeps active repos
safe even when their commit date is old: an active repo's artifact
dirs are fresh because its tools touch them. Every action logs with a
size; `--dry-run` answers the survey table without touching anything;
anything not on the list is named in the log, never removed. `git gc
--quiet` runs on repos whose loose-object count exceeds 500, throttled
to weekly.

**The bleed rule (C4)** stops the history class from growing: the
e2e-review artifacts in dnd-vtt and mahjong-vtt are untracked
(`git rm -r --cached`, keeping every local file) and ignored, so the
next review lands on disk, not in the pack. The cruisingintelligence
media and every history rewrite are explicitly out of scope — those
are the owner's history to keep or rewrite, not this plan's.

## Acceptance criteria

- [ ] **C0 — the baseline is recorded.** The survey's full table
      (repo, total, .git, artifacts, last commit, churn) lands in this
      plan file as measured 2026-10-27, so every later number has a
      before.
- [ ] **C1 — the stale artifacts are swept, by name.** The exact dirs
      listed in Context (Class 1) plus `fleet-remote`'s target via
      `cargo clean` are removed; the sweep script prints each dir's
      size as it goes and a before/after total; every repo's `git
      status --porcelain` count is unchanged after its sweep (churn
      preserved); active repos (design-canvas, synapticweb,
      mahjong-vtt, fleet) are untouched. Reclaimed: ~21.7 GB.
- [ ] **C2 — the neglected stores are packed.** `git gc` on
      simcityclone and deepseek-harness; `count-objects -v` before and
      after recorded here. simcityclone's 819 MB of loose objects
      packs down (~500 M expected).
- [ ] **C3 — the permanent pass runs daily.**
      `scripts/repo-hygiene.sh` (dry-run and live modes) is invoked
      from `scripts/daily-maintenance.sh` and the LaunchAgent picks it
      up; a dry run answers the survey table; the live rules are: last
      commit > 21 days AND artifact-dir mtime > 14 days AND the dir
      name is on the closed list; every action logged with sizes;
      non-list top-level dirs NAMED in the log; loose-object gc
      weekly past 500. Proven live: a stale repo's artifact dir is
      pruned by the run, an active repo's is left.
- [ ] **C4 — the history stops growing.** dnd-vtt and mahjong-vtt
      carry the untrack + ignore for `docs/e2e-review/` (local files
      kept); the next review written there stays on disk and out of
      the pack (verified by a commit that does not pick the dir up).
      cruisingintelligence's media is named as the owner's residual
      decision in this plan, untouched.
- [ ] **C5 — the instrument is kept.** The survey lives as
      `scripts/repo-hygiene.sh --report`; `CLAUDE.md`'s scripts list
      names it beside the maintenance script. The gate is green (the
      change is shell-only; fmt/clippy/test run anyway, by exit code)
      and this file's status flips in the landing commit.

## Files to be touched

`scripts/repo-hygiene.sh` (new: report, dry-run, live) ·
`scripts/daily-maintenance.sh` (the invocation) · `CLAUDE.md` (the
scripts list) · this plan. The sweeps and untracks land in the TARGET
repos (cruisingintelligence, commissiontracker, deepseek-harness,
elo-scenario-lab, TTESS, pmocockpit, dnd-vtt, mahjong-vtt, simcityclone,
catan, threeui, helmsman, pampa-wineclub, fleet-remote) — their own
commits where a commit is made (C4), explicit paths everywhere, never
`git add -A`.

## Out of scope

- **History rewrites** (filter-repo/BFG) for the media already in
  cruisingintelligence's, dnd-vtt's, or mahjong-vtt's packs — the
  owner's call, destructive to history. This plan only stops growth.
- **Anything outside `~/Projects`** — Docker's own data dir (32.7 GB,
  a different lever), `coreMLCache` (3.3 GB, framework-owned),
  the Trash.
- **Uncommitted source work** — every repo's churn is untouched by
  every increment.
- **Deleting whole repos** (including `synapticweb-tmp`, listed for
  the owner instead) and **retiring or archiving stale projects** —
  a separate ruling.

## Verification

The survey re-run after C1/C2 (same script, `--report`): the artifact
mass drops by the recorded amounts and the churn counts are byte-identical
per repo. C3's proof is live: the LaunchAgent's next 09:20 run executes
the hygiene pass (its log names what it pruned and what it left), and a
stale-vs-active pair demonstrates the mtime guard. C4's proof is a
commit in dnd-vtt or mahjong-vtt whose status shows the review dir
ignored. The gate whole and local where the repo is fleet.

## Linked artifacts

Precedent: `scripts/daily-maintenance.sh`, `scripts/cache-sweep.sh`,
the 2026-10-26 fleet garbage taxonomy (`4612e8d`), the
`generated/`-was-not-garbage lesson. The survey script's output is
reproduced by `scripts/repo-hygiene.sh --report` once C3 lands.
