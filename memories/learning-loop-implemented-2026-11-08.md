---
name: learning-loop-implemented-2026-11-08
description: "project 2026-11-08 — the Learning feature is IMPLEMENTED whole, the plan COMPLETED (C0–C11 ticked, status completed, the first-lesson proof recorded as the owner's DEFERRED residual), MERGED to master (f93544d) and DELIVERED — the one-folder rebuild, the dmg at fleet/target/release/bundle/dmg, the app installed and the face live on the new binary"
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

*"I have seen the mockups, I'll test later. you can close the plan now. rebuild the app"* — the owner's C0 read closed the last gate: the plan is **completed**, every criterion ticked (C0 read, C1–C11 landed; C7 had been missed and was ticked with its evidence when the pre-commit `plan_check` refused the closure), the branch merged to master and the app delivered.

**The commits.** C1/C2 `50b380f` (the lesson kind, the prober and the probe loop), C3 `d4516f2` (the minimal tree), C4 `e063b4f` (the teaching loop), C5 `3f2de8c` (the knowledge graph, the `premises` write path, the gated extraction), C6 `928f762` (the practice), C7 `88ae9d5` (the Learning rail group and the pane's four sections, the `RunAs` learning seam), C8/C9 `bd6bc8c` (the `learn_check` row → the demand map's `learn` family, the clustering beat, the drafter; the evidence discipline), C11 `801d9da` (docs), C10 gate ticked `10c1791`, the role seed `415d3b6`, the merge with master's hybrid retrieval `8891c21`, the close-of-record recap `e5ee292`.

**The gate before the merge:** fmt and clippy clean, the release build green, 2,024 lib tests and every integration suite pass (the two lanes tests green under `BROWSER_CHROME` — omit it and they fail; the browser lane's environmental input, not a defect), the face harness 998 checks across 117 cases, `plan_check.py` clean.

**The delivery, and the correction that shaped it.** The owner refused a second folder — *"fleet is a single unit … there should be just one single fleet folder on the local machine. the build needs to happen there, that folder should contain the objects of the merged tree."* So the dmg built in a worktree's `target/` was lost when the lane was pruned, and the delivery was redone from the main tree: the main checkout was switched from `face-plugin-host` (which is an ANCESTOR of master — a plain fast-forward, its 7 unpushed commits and all commits exist in master) to `master`, `cargo tauri build` run there, the face binary rebuilt and reinstalled, the app installed. The in-flight work the main tree held (the `gui.md` inbox section, the owner-inbox mockups, the composer-clipboard and waza plans, the memories' recall counts, two plan docs' status edits) lives in a NAMED stash — `stash@{0}: the main tree's in-flight work at the switch to master (2026-11-08)` — nothing lost; reconstructing it at the same paths under master's layout will need a patch path, not a pop, because master's `gui.md` and the two plan docs already carry the co-tenant's versions.

**The rail correction, and the state it left.** The owner's next turn was *"the learning rail item should have been part of work and not a new category"*: the row moved from its own fifth group into **work** (beside the desk), the fifth group deleted, the harness re-pinned (999 checks / 117 cases — the four-group count restored and a new check asserting the learning link's `data-group === 'work'`), the surfaces doc and the feature doc reworded. Committed `9df0130`, pushed; gate green (fmt, clippy, 2,024 lib tests, `plan_check`).

**The closing turn, and the one thing to answer next time.** *"close the plan, C0 is done and the first lesson is deferred until I have time to test"* — C0 ticked with your mockup read, **C7 was still `[ ]`** (the face surface landed at `88ae9d5` but its box was never ticked — the pre-commit `plan_check.py` hook refused the closure and named it), so C7 was ticked with its evidence; the State section now records the residual in KI-398's form — **"the live proof is deferred by the owner until they have time to test"**, with its owner and state named — and `plan_check.py` passes. Committed `f93544d` (docs-only, so no rebuild: the dmg and both binaries already carry the code), pushed to `origin/master`. `~/.fleet/bin/fleet` at 3080 and `/Applications/fleet.app` both answered 200.

**The live proof is DEFERRED, not abandoned.** Do not count it as open work, and do not re-derive it: when the owner comes back, the only step is `#/learning` → **start a lesson** and let the prober run. The harness, the module tests, the bridge test and the CLI's `--kind` filter pin the machinery.

**Where the artifacts are now:** the tree `/Users/alejandrodelvillar/Projects/fleet` on `master` @ `f93544d` (the rail fix `9df0130`); the dmg `/Users/alejandrodelvillar/Projects/fleet/target/release/bundle/dmg/fleet_0.4.0-beta.1_aarch64.dmg` (11.63 MiB, rebuilt after the rail fix; its packaged binary carrying the learning surface, the work-group placement and `hybrid_search`); `~/.fleet/bin/fleet` replaced and the LaunchAgent restarted (port 3080, 200, serving the corrected `app.js`); `/Applications/fleet.app` reinstalled from the dmg and relaunched (its own port answered 200, the served `app.js` carrying the learning row in the work group); the earlier build moved aside to `fleet-previous-2224.app`; no worktrees remain.

**Why:** the owner's whole-feature ruling, then the one-folder ruling, then the rail-placement ruling — the product is one tree on this machine, the build belongs in it, and its primary taxonomy is the owner's.
**How to apply:** the plan is closed and the owner's first lesson is deferred — do not reopen the plan or rebuild for it; the live CLI proof already showed `fleet sessions --kind teaching` listing the teaching session, `--kind practice` none, the default both. Related: [[a-rails-primary-taxonomy-is-the-owners]], [[one-fleet-folder-the-build-belongs-there]], [[learning-loop-plan-whole-feature-2026-11-08]], [[learnharness-deep-dive-2026-11-08]], [[the-served-face-is-the-installed-copy]].
