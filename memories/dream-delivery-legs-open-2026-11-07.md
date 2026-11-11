---
name: dream-delivery-legs-open-2026-11-07
description: "project 2026-11-07 — SUPERSEDED the same day by [[dream-dmg-installed-face-relaunched-2026-11-07]]: the dmg was rebuilt and installed and the face relaunched on the new build; the merge to master FOLLOWED (1eaba7a) and is now DONE. Survives as the record of the delivery-state answer and the merge's pre-state"
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

The owner asked (2026-11-07), after the dream-plan completion: *"does it need to be merged and the app rebuilt?"* The answer given, and the state it named. **SUPERSEDED the same day** — the dmg leg and the relaunch leg closed (see [[dream-dmg-installed-face-relaunched-2026-11-07]]); the merge analysis below stands as the open leg's record.

**The merge — DONE (2026-11-07, `1eaba7a`).** The owner approved; the merge ran through a SCRATCH WORKTREE (`git worktree add /tmp/fleet-merge master` then `git merge face-plugin-host --no-ff`) so the parallel session's uncommitted files were never touched. Two doc conflicts, both resolved to MASTER's newer sides: the CLAUDE.md plan-worktree paragraph (master's carries the `scripts/plan-worktree.sh` sentence) and the add/add plan file (master's is the COMPLETED version with boxes ticked; the branch's was the earlier draft). The code (bridge, app.js, harness) auto-merged. The merged tree's gate: fmt 0, clippy 0, build 0, test 0, harness 984 checks / 116 cases; both refs pushed (`master` 9985e0f..1eaba7a, `face-plugin-host` e56e404..a7ea917); the scratch worktree removed with `--force` (it held only regenerable build artifacts). The analysis below stands as the record of the state before the merge.

**The app — the binary was built, signed, and installed; the RELAUNCH was the missing leg, and it then CLOSED** ([[dream-dmg-installed-face-relaunched-2026-11-07]]: the dmg rebuilt, /Applications replaced, the new app opened as pid 48011 on 3080 serving the new document). The state below stands as the pre-delivery record: `target/release/fleet` was copied over `~/.fleet/bin/fleet` and ad-hoc re-signed (`codesign --force --sign -`) — that is how the live C7 proof ran — and the face process in the owner's window (pid 20950, `fleet gui --no-open --remote --relay https://fleet.opsh.io --bot-relay https://fleetbot.opsh.io`) had still been running the OLD build.

**Why:** the completion answer named the build state without the delivery state — the exact failure [[rebuild-is-not-commit-push-or-ship]] records (a plan completing does not complete the delivery).
**How to apply:** the next session's first move is the delivery pass: relaunch the face (kill pid 20950, relaunch the same command, prove the served bytes carry the new pane), then take the merge through a scratch worktree once the owner gives the word — and say the delivery state in the closing line. Related: [[memory-dreaming-plan-implemented-2026-11-07]], [[the-served-face-is-the-installed-copy]].
