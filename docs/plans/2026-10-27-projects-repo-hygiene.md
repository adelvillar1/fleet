---
status: completed
created: 2026-10-27
updated: 2026-10-27
slug: projects-repo-hygiene
---

# Projects repo hygiene: the measured sweep, and the permanent habit

> **Status: COMPLETE** — C0–C5 landed 2026-10-27.

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

## Baseline — measured 2026-10-27 (C0)

```
REPO                        TOTAL       GIT  ARTIFACTS  LASTCOMM  CHURN  ARTIFACT-DIRS
TTESS                    1790.0M      37M   1675.5M  2026-09-28     9   node_modules(869M) .next(807M)
agnostic-router-kit       583.3M      16M      0.0M  2026-10-24     7  
blend1                     31.1M       1M     14.4M  2026-10-24     1   .venv(14M)
catan                     304.4M       9M    263.3M  2026-09-12     5   node_modules(264M)
codebuddy2openai           24.4M       0M     23.3M  2026-06-19     3   .venv(24M)
commissiontracker        3213.3M       5M   3197.7M  2026-09-21     7   node_modules(1.2G) .next(2.0G) out(3.4M)
cruiserintelligence      2119.9M       0M      0.0M  2026-05-29     6  
cruisingintelligence     9313.3M    1652M   2907.7M  2026-09-22    30   node_modules(2.1G) .next(749M)
deepseek-harness         2837.7M     301M   2102.2M  2026-10-13     4   node_modules(2.0G) coverage(61M)
design-canvas            2116.6M       5M   2053.3M  2026-10-26     8   node_modules(874M) .next(1.2G)
dev-decisions              12.2M       5M      0.0M  2026-10-14     2  
dnd-vtt                  1631.1M     364M    459.9M  2026-09-21     6   node_modules(459M)
elo-scenario-lab         2813.3M       6M   1576.6M  2026-09-12     8   node_modules(743M) .cache(115M) .venv(335M) venv(383M)
fleet-remote             7925.5M       0M   7915.5M  no-git            target(7.7G)
fleet                   14857.7M     119M  14719.9M  2026-10-27    23   target(14G) tmp(20M)
fuiz                        0.0M       0M      0.0M  no-git           
gen1                       98.8M       0M     93.3M  2026-10-09     2   .venv(94M)
helmsman                  306.6M       7M    290.0M  2026-10-09     2   node_modules(290M)
jobhound-reference        128.8M       0M      0.0M  no-git           
jobhound                 1117.7M       7M      0.0M  2026-10-26     0  
keydock                     0.0M       0M      0.0M  no-git           
llmtests                  395.5M       0M      0.0M  no-git           
mahjong-vtt              1396.6M     262M    584.4M  2026-10-26    35   node_modules(584M)
metaskillsdb                0.0M       0M      0.0M  no-git           
openship                   95.5M       0M      0.0M  no-git           
pampa-wineclub            569.9M      22M    323.3M  2026-10-22     2   node_modules(323M)
pmocockpit               1081.1M       7M   1067.7M  2026-09-12     2   node_modules(872M) .next(195M)
riderscout-saas-migrated    324.4M      12M      0.0M  2026-09-12    10  
sdm1                       54.4M       1M     50.0M  2026-10-09     2   .venv(51M)
second-brain               66.6M       2M     32.2M  2026-10-09     8   .venv(32M)
sem1                      898.8M       1M      0.0M  2026-10-24     4  
simcityclone             2461.1M     819M    726.6M  2026-08-26     1   node_modules(164M) dist(562M)
synapticweb-tmp             0.0M       0M      0.0M  2026-08-06    20  
synapticweb               590.0M       4M    582.2M  2026-10-26     2   node_modules(441M) .next(142M)
sys1                       43.3M       2M     31.1M  2026-10-24    10   .venv(32M)
three-lanes               270.0M       0M      0.0M  no-git           
threeui                   432.2M      52M    237.7M  2026-08-23     1   node_modules(153M) dist(85M)
zcode-router-kit           20.0M      10M      0.0M  2026-10-14     3
```

(The columns: total du; `.git` du; regenerable-artifact du; last
commit; `git status --porcelain` lines; the artifact dirs over 1 MB.)

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
- [x] **C1 — the stale artifacts are swept, by name. Landed 2026-10-27.** The exact dirs
      listed in Context (Class 1) plus `fleet-remote`'s target via
      `cargo clean` are removed; the sweep script prints each dir's
      size as it goes and a before/after total; every repo's `git
      status --porcelain` count is unchanged after its sweep (churn
      preserved); active repos (design-canvas, synapticweb,
      mahjong-vtt, fleet) are untouched. Reclaimed: ~21.7 GB.
- [x] **C2 — the neglected stores are packed. Landed 2026-10-27.** `git gc` on
      simcityclone and deepseek-harness; `count-objects -v` before and
      after recorded here. simcityclone's 819 MB of loose objects
      packs down (~500 M expected).
- [x] **C3 — the permanent pass runs daily. Landed 2026-10-27.**
      `scripts/repo-hygiene.sh` (dry-run and live modes) is invoked
      from `scripts/daily-maintenance.sh` and the LaunchAgent picks it
      up; a dry run answers the survey table; the live rules are: last
      commit > 21 days AND artifact-dir mtime > 14 days AND the dir
      name is on the closed list; every action logged with sizes;
      non-list top-level dirs NAMED in the log; loose-object gc
      weekly past 500. Proven live: a stale repo's artifact dir is
      pruned by the run, an active repo's is left.
- [x] **C4 — the history stops growing. Landed 2026-10-27.** dnd-vtt and mahjong-vtt
      carry the untrack + ignore for `docs/e2e-review/` (local files
      kept); the next review written there stays on disk and out of
      the pack (verified by a commit that does not pick the dir up).
      cruisingintelligence's media is named as the owner's residual
      decision in this plan, untouched.
- [x] **C5 — the instrument is kept. Landed 2026-10-27.** The survey lives as
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

## Status record

- **C0 — landed 2026-10-27.** The 39-row survey table is in this file
  above.
- **C1 — landed 2026-10-27.** The measured list went through
  `repo-hygiene.sh --purge` (each dir name-verified against the closed
  list before removal, each size logged): 23 artifact dirs totalling
  ~14.6 GB, plus `cargo clean` on the fleet-remote worktree — cargo
  reports **25.9 GiB removed** (du had shown 7.7 GB of it; APFS clone
  accounting undercounts). **~/Projects: 60.4 GB → 38.2 GB.** The pin
  held: all twelve swept repos' `git status --porcelain` counts are
  byte-identical before and after, and the active repos' sizes are
  unchanged (design-canvas 2.1 G, synapticweb 591 M, mahjong-vtt 1.4 G).

- **C2 — landed 2026-10-27, and the plan's expectation was WRONG.**
  simcityclone: 1,194 loose objects (838,964 KiB) packed to an
  835,970 KiB pack — **819 MB → 816 MB, ~3 MB back**. The objects were
  already-compressed game assets; packing consolidated them (0 loose,
  fsck clean) but did not shrink. deepseek-harness: 1,489 loose packed,
  size-pack 291,470 → 248,818 KiB (~42 MB). Both fsck clean. Recorded
  as measured: the loose-object problem was ORDER, not size.
- **C3 — landed 2026-10-27.** `scripts/repo-hygiene.sh` (report /
  dry-run / live / purge / --root) behind `daily-maintenance.sh`'s new
  leg 5. Proven live on a stale/active fixture: the stale repo (commit
  30 d, dir mtime 37 d) is pruned by the dry-run's own naming and the
  live run, the active repo (commit and dir fresh) is left, and the
  stale repo's committed source survives. The LaunchAgent picks leg 5
  up at the next 09:20 with the rest of the maintenance.

- **C4 — landed 2026-10-27.** dnd-vtt untracked 525 files under
  `docs/e2e-review` (269 MB of reviews that will never re-enter a
  pack), mahjong-vtt 188 (218 MB); both committed with explicit paths
  (`.gitignore` + the dir), local files kept on disk, `git status` on
  the dir answers empty (ignored), `check-ignore` confirms, and the
  ignore entry names this plan's C4 in its comment. cruisingintelligence's
  media stays named as the owner's residual decision.
- **C5 — landed 2026-10-27.** `--report` is the kept instrument (the
  survey re-run is one command); `CLAUDE.md`'s scripts list names the
  script beside the maintenance script with its guards; the gate runs
  by exit code; this status flips in the landing commit.

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
