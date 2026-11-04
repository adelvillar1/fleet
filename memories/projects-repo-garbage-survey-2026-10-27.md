---
name: projects-repo-garbage-survey-2026-10-27
description: "2026-10-27 — the house-wide repo cleanup EXECUTED in full (docs/plans/2026-10-27-projects-repo-hygiene, C0–C5): 60.4 → 38.2 GB swept by name with churn byte-identical, gc measured honestly (819→816 MB — packs order, not size), the daily habit now machine-wide via leg 5 (closed artifact list, 21d-commit + 14d-mtime guards), review HTML untracked in both VTT repos"
type: project
date: 2026-11-04
basis: inferred
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

**Owner, 2026-10-27: "each of the repos with active history need to be
reviewed to find 'garbage' like we did in fleet"** — the fleet pattern
([[fleet-repo-garbage-taxonomy-2026-10-26]]) applied house-wide, then
planned as `docs/plans/2026-10-27-projects-repo-hygiene.md` and
**IMPLEMENTED in full the same day** (C0 `e5dd080`, C1 `60eaf91`,
C2+C3 `749dd93`, C5 `f55dd70`; C4's commits live in the target repos).

**Executed state: `~/Projects` went 60.4 GB → 38.2 GB** (from ~64 GB at
the first survey).

- **C1 — the by-name sweep.** The 23 measured artifact dirs went
  through `repo-hygiene.sh --purge` (each name verified against the
  closed artifact list before removal) + `cargo clean` on the
  fleet-remote worktree. Pin held: all twelve swept repos' `git status
  --porcelain` counts byte-identical before/after; active repos
  (design-canvas, synapticweb, mahjong-vtt) untouched.
- **C2 — measured honestly, and the plan's estimate was WRONG:**
  simcityclone's 1,194 loose objects (819 MB) packed to an 816 MB pack
  — **~3 MB back; the objects were already-compressed game assets.**
  "gc reclaims loose-object bloat" is false when the content is
  compressed: gc restores ORDER, it does not shrink size. Only
  deepseek-harness (mixed content) saved ~42 MB. Both fsck clean.
- **C3 — the permanent habit is machine-wide.**
  `scripts/repo-hygiene.sh` (`--report` / `--dry-run` / `--live` /
  `--purge PATH` / `--root`) runs as **leg 5 of
  `scripts/daily-maintenance.sh`** (the 09:20 LaunchAgent): repos with
  last commit > 21 days get artifact dirs pruned when dir mtime > 14
  days; the artifact list is CLOSED (target, node_modules, .next,
  dist, build, out, coverage, .turbo, .venv, venv, tmp, .cache);
  non-list dirs are NAMED in
  `~/Library/Logs/repo-hygiene.log`, never removed; loose-object gc
  weekly past 500. Proven on a stale/active fixture: stale pruned,
  active left, committed source survives.
- **C4 — the history bleed stopped:** dnd-vtt untracked 525 files and
  mahjong-vtt 188 from `docs/e2e-review/` (269 + 218 MB that will
  never re-enter a pack), local files kept, `check-ignore` confirmed,
  explicit-path commits in each repo. cruisingintelligence's 1.45 GiB
  media pack (65 MB guide, 52 MB .wav, 47 MB .docx) stays the OWNER'S
  residual decision — history rewrites remain out of scope.

The survey's own stale claims, corrected by execution: the survey said
simcityclone's gc was "≈500 MB back" (measured ~3 MB), and du showed
fleet-remote's target at 7.7 G while cargo clean removed 25.9 GiB —
see [[du-undercounts-apfs-clones]]. Worktree gotcha that cost the
first survey a `no-git`: check `.git` as a FILE too (see the baseline
table's fleet-remote row).

Related: [[fleet-recap-ends-with-cache-sweep]] (the fleet-only
original), [[mcp-vizro-removed-2026-10-27]] (the same-day Docker
residual note).
