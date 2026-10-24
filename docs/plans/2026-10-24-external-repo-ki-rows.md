---
status: active
created: 2026-10-24
updated: 2026-10-24
slug: external-repo-ki-rows
---

# Plan: the external-repo KI rows — seven rows, six repos

## Context

The burn-down's out-of-scope list named the external-repo rows as
wanting "their own session"; the owner called for exactly that. The
rows are write-time snapshots and the repos have moved under them: the
survey found gen1's remote added and master even with `origin/master`
(KI-004's ask already done by the owner), sys1's plan-surface work
landed through `c6e978c` with only a two-line `provider or []` guard
left uncommitted in `api.py` (KI-003's "land or discard" answered by
the work), and the router-kit failure log confirming the row's
diagnosis verbatim — Node 20 lacks `.mts` type stripping and the run
never installs before `npm test`. KI-207's owner choice was put to the
owner and answered: **update the driver**.

Each upstream repo keeps its own rules: its CLAUDE.md is read before
the first commit there, commits carry explicit paths, and a feature
branch pushes to its own tracking branch — nothing merges to anyone's
main without the owner.

## Approach

The rows in dependency order: the evidence-only close first, then sys1
(land the guard, then re-vendor from the landed sha — KI-029 waits on
KI-003 exactly as the row says), then the two single-repo fixes
(router-kit's CI, sem1's CLAUDE.md), then blend1's adapter filing, then
the driver. Fleet's ledger closes each row in the same turn as its
evidence; each upstream repo's own gate runs before its commit.

### Increment 0 — the evidence close (C0, KI-004)

gen1 carries `origin = git@github.com:adelvillar1/gen1.git` and
`origin/master..master` is empty — the remote exists and master is
pushed. The row closes naming that, no code.

### Increment 1 — land sys1's last delta (C1, KI-003)

`src/sys1/api.py`'s two-line None-guard
(`",".join(provider or [])`) commits on `feature/plan-surface` — the
branch's own tests run first (the repo's pytest), the branch pushes to
its tracking branch, and nothing merges to main. The row closes: the
four files are landed (three by the owner's commits, one by this).

### Increment 2 — re-vendor sys1 from a landed sha (C2, KI-029)

With the guard landed, `services/sys1/src` already matches upstream
HEAD byte for byte; the remaining delta is `service/mcp_server.py`.
Re-vendor per the `syncRule`: copy `src/` and `service/mcp_server.py`
at the new recorded sha, update `VENDOR.json` (commit, subject,
`fleetVendor` note replaced by the landed provenance), and run fleet's
sys1 lane tests so the vendored copy proves itself. The row closes:
the vendored copy's provenance is a commit, not a working tree.

### Increment 3 — router-kit's CI runs what the repo runs (C3, KI-001)

The row's named fix, confirmed by the run log (`ERR_UNKNOWN_FILE_EXTENSION` on
`.mts`, `ERR_MODULE_NOT_FOUND` on `workflow-plane`): `ci.yml` moves to
a Node line that strips types (22 or later; this machine runs 24 and
the probes pass on it) and runs the install before `npm test`. The
suite runs locally first under the same Node; the change commits in
that repo and pushes, and the row closes on the run's own verdict.

### Increment 4 — sem1's methodology layer (C4, KI-005)

The row's trigger — "when sem1 is next touched" — fires now. A slim
`CLAUDE.md` on the sibling pattern (what sem1 is, hard rules, where
things live, the commands, the session protocol) scaffolds in
`~/Projects/sem1`, modeled on the sibling repos', and commits there.
The row closes.

### Increment 5 — file blend1's adapter upstream (C5, KI-143)

`services/blend1/service/mcp_server.py` (566 lines) is fleet's carry;
upstream `~/Projects/blend1` has no MCP server. The adapter files
upstream on a branch beside its test, importing only what main's
library carries — a branch, not a merge; merging is the owner's. The
row closes when the filing is a commit upstream, which is what the row
names as the carry's end state.

### Increment 6 — the driver, by the owner's ruling (C6, KI-207)

`cua-driver update --apply` runs, the binary's version is read back,
and one live MCP call through fleet's own client proves the
compatibility the row died on. The row closes with both.

## Acceptance criteria

- [ ] **C0 — KI-004.** The row reads closed naming the remote and the
  even-with-origin state, no code in that commit.
- [ ] **C1 — KI-003.** The guard is a commit on `feature/plan-surface`
  (branch tests green first), the branch is pushed, and the row closes:
  no uncommitted sys1 work remains.
- [ ] **C2 — KI-029.** `services/sys1/` matches upstream at a recorded
  landed sha, `VENDOR.json` carries that sha and a provenance note that
  names a commit, fleet's sys1 lane tests pass, and the row closes.
- [ ] **C3 — KI-001.** `ci.yml` installs before `npm test` on a Node
  that strips types; the suite passes locally under that Node; the
  change is pushed; the row closes on the CI run's verdict.
- [ ] **C4 — KI-005.** `~/Projects/sem1/CLAUDE.md` exists on the
  sibling pattern and is committed; the row closes.
- [ ] **C5 — KI-143.** The MCP adapter and its test are commits on a
  branch of upstream blend1, importing only main's library surface; the
  row closes on the filing.
- [ ] **C6 — KI-207.** The driver binary is updated per the owner's
  recorded ruling, its version reads back aligned, one live MCP call
  answers, and the row closes with both.
- [ ] **C7 — close-out.** Fleet's ledger carries every closure with its
  evidence; the fleet gate is green by exit code read directly; commits
  carry explicit paths per repo; the plan's `status` agrees.

## Files to be touched

Fleet: `docs/known-issues.md`, `services/sys1/**`, this plan.
Upstream: `~/Projects/sys1` (`src/sys1/api.py`), 
`~/Projects/agnostic-router-kit` (`.github/workflows/ci.yml`),
`~/Projects/sem1` (`CLAUDE.md`), `~/Projects/blend1`
(`service/mcp_server.py`, its test).

## Out of scope

- Merging any upstream branch to its main (blend1's
  `feature/escalation-and-sys1-score` merge and sys1's branch both
  stay the owner's call).
- The gen1/sem1 untracked scratch (`docs/architecture` drafts,
  `egg-info`, `uv.lock`) — present in the survey, untouched.
- The remaining owner-decision and deck rows — they stay their own
  sessions.

## Verification

Per upstream repo: its own test command before its commit. Fleet: the
gate's four commands, exit codes read directly. Ledger rows updated in
the same turn as their evidence.

## Linked artifacts

- `docs/known-issues.md` — the seven rows
- `docs/recaps/2026-10-24-known-issues-burn-down.md` — the session
  that deferred these here
- `services/sys1/VENDOR.json`, `services/blend1/VENDOR.json` — the
  cross-repo pins this work realigns
