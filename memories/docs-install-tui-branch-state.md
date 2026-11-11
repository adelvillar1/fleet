---
name: docs-install-tui-branch-state
description: "project 2026-11-06 — ALL FOUR plans implemented on docs-install-tui and MERGED to origin/master at eb57d8f: readme 5/5, seat KI-467, hygiene 8/8 with live proofs, route-judgment 9/10 with C7's live proof; the face reloaded and the dmg rebuilt; the branch, the worktrees and the stale branches PRUNED; the master worktree's prune LOST the merged artifacts and the face reverted to stale code — rebuilt from the main checkout after merging master into face-plugin-host; both apps carrying the merged code at 0.4.0-…"
type: project
date: 2026-11-11
basis: inferred
tier: durable
shape: []
phrases: []
missing: []
premises: []
recall-count: 0
superseded-by:
valid-until:
source: zcode:fleet:content:285983102772d99aaed2c496d57acdf6f320aad7347d3891e9bb9e2378839d34
---

The owner set (2026-11-06) the goal: "implement plans you just created
starting with the readme and install but on a separate branch because the
KI burn down is still ongoing." The KI burn-down (the live run on
face-plugin-host) was mid-work with its own uncommitted tree, so the plans
were implemented on a **separate worktree**: `~/Projects/fleet-docs-wt`,
branch `docs-install-tui`. The main checkout's work was never touched.

**ALL FOUR PLANS LANDED AND MERGED to origin/master at `eb57d8f`** (the
owner's "merge, push to remote, rebuild, and reload" steer):

- **readme-install-and-tui-images 5/5** (`b7bd882`, cherry-picked to master
  as `24ad44b`, then carried through the merge) — the Install section, six
  TUI screens by `scripts/tui-shots.py`
  ([[readme-install-and-tui-images-plan-drafted]]).
- **seat-decision-labels ✓** (`141fe72`, KI-467 after the numbering
  collision) — [[seat-decision-buttons-duplicate-their-labels]].
- **session-close-hygiene 8/8** (`7e6b745` + `555392a` + `df693be`) —
  [[session-close-hygiene-plan-drafted]], live proofs quoted.
- **route-judges-the-work 9/10** (`cf9a90e` + `6a75b60` + `6cfd0b8`) —
  [[route-judges-the-work-implemented]], C7's live proof landed.

**The merge was three-stage** (master kept moving under the branch):
1. b7bd882 cherry-picked to master as 24ad44b (the README work alone).
2. The branch rebased onto master (b7bd882 deduped, 062ee28 skipped — the
   burn-down fixed the same lints at 935d148).
3. The full merge (d5e9770) landed after the burn-down's release rollback
   (10acb5e) committed; then the fmt/clippy fixes (91cbe5f) and the KI
   numbering resolution (eb57d8f) pushed to origin/master.

**The delivery:** the face reloaded twice (once from the docs worktree's
build, once from the master worktree's), pid 4666, port 3080 answering 200,
the served app.js carrying the seat fix (2 × `node('button', 'decision')`,
0 × the old buggy call). The Tauri dmg rebuilt at `fleet_0.3.0-beta_aarch64.dmg`
(9.77 MiB) from the merged master.

**The gate on the merged tree:** fmt 0, clippy 0, lib **1934/0**, harness
**963/110**. The merge brought master's fmt drift (the burn-down's session
didn't run fmt) and a needless-return clippy lint in the board-fact seam —
both fixed mechanically.

**PRUNED (2026-11-07, the owner's "yes").** The branch audit confirmed
design-parity stale (zero unique patches by `git cherry`, the merge at
ecbf280) and the other branches fully merged. The owner approved, and:
- **Branches deleted** (local + remote): design-parity, docs-install-tui,
  reload-reattach, calib-usage-anomalies, session-header-parity,
  session-parity-remainders, provider-session-headers.
- **Worktrees removed** (with --force where the recall-count bumps and the
  fmt drift were the dirty state): fleet-docs-wt, fleet-master-wt,
  fleet-reload-reattach, and the three /tmp detached-HEAD checkouts
  (fleet-c13-gate, fleet-c13-old, fleet-gate).
- **Remaining:** face-plugin-host (the burn-down's working branch) and
  master (the trunk), plus remote-access. Two worktrees: the main checkout
  and fleet-remote.
- The KI numbering collision resolved THREE times (the seat row renumbered
  KI-462→KI-466→KI-467 as both sessions landed rows independently); the
  ledger at 467 rows, 6 open, checker green.

**The prune LOST the merged artifacts (2026-11-07).** The master worktree's
removal deleted the merged release binary and dmg alongside the tree — and
the face reverted to serving STALE code (zero seat-fix occurrences in the
served app.js, the route-judgment wiring absent from the binary's strings).
The rebuild had to happen from the main checkout after merging master into
face-plugin-host (the ledger conflict carried KI-468/469 — the burn-down's
later door and mention-scanner finds — beside the branch's KI-467). Both
apps rebuilt: the release binary + the face reloaded (pid 53096, port 3080
→ 200, the served app.js carrying 2 × the corrected node call), the Tauri
dmg at 0.4.0-beta.1 (11.00 MiB, the version the burn-down's Cargo.toml
merge left). See [[worktree-prune-loses-built-artifacts]].

**How to apply.** The branch is merged, pushed, pruned. The open-ledger
plan's status stays `active` because C12 (the four decision rows) is the
owner's by the plan's own text. The route-judgment plan's C7 live proof
landed. The worktree pattern
([[two-sessions-one-repo-master-worktree]]) is still the shape for a
parallel session, but the docs worktree's lifecycle is closed. The
branch parity audit's detail: design-parity's 39-commit "ahead" count
was the branch's own origin ref not being pushed since the merge —
`git cherry` found zero unique patches, and the branch and its remote
ref deleted cleanly. Related:
[[rebuild-is-not-commit-push-or-ship]] (the delivery standing ask,
including the served-code check's own false negative: an inline
`curl | grep -c` returned 0 while the file-based re-check returned 2 —
the face was restarting under the reload, and the browser was caching
the old app.js the owner's screenshot showed).
