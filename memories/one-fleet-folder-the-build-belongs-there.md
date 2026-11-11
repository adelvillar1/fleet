---
name: one-fleet-folder-the-build-belongs-there
description: "owner ruling 2026-11-08 — fleet is one folder on this machine; the build happens there and that folder holds the merged tree's objects; a worktree build's artifacts are not the deliverable and die with the lane"
type: feedback
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

*"why are we using a different project folder? fleet is a single unit"* then, after I started a throwaway master worktree to rebuild: *"we should not have a separate folder, there should be just one single fleet folder on the local machine. the build needs to happen there, that folder should contain the objects of the merged tree."*

**Two mistakes in one delivery.** I answered "what is the path for the current dmg?" with a path inside the pruned learning worktree (`fleet-learning-loop/target/...`) — gone with the lane, so the dmg I had announced was not there — and then I tried to fix it by adding a temporary `fleet-dmg` worktree. Both read to the owner as "a different project folder", which is exactly what the house topology was meant to be an internal convenience for, never a shape the owner has to track.

**Why:** the plan-worktree convention exists so parallel *sessions* do not collide. It is not a topology the product lives in, and the owner holds the product to one tree with one set of objects. A lane is event-scoped and pruned — so anything built inside it (the dmg, the release binary) is event-scoped too and must never be named as the delivery.

**How to apply:**
- **The deliverable is built in the main tree.** The main folder is `/Users/alejandrodelvillar/Projects/fleet`; the dmg lands at `fleet/target/release/bundle/dmg/` and the binary at `fleet/target/release/fleet`. A dmg path inside a `fleet-<slug>` lane is never the answer.
- **Switch the main tree to the trunk when the feature is on master.** Check first that the working branch is an ANCESTOR of master (`git merge-base --is-ancestor <branch> master`) — then `git switch master` is a plain checkout, and unpushed branch commits are preserved by master. Stash in-flight work with `-u` and a NAMED message before switching, and say where it went.
- **No worktrees left behind.** `git worktree list` should show the one folder; prune any leftover master checkout or lane, and never keep a resident master checkout just as a build host.
- **Check the mounts before a tauri build.** A stale `rw.*.dmg` mount (`hdiutil: couldn't unmount — Resource busy`) kills `bundle_dmg.sh`; `hdiutil detach /dev/diskN -force`, then rebundle. And `cargo tauri build` alone does NOT refresh `target/release/fleet` — `cargo build --release` is a separate pass.
- **The installed app's ad-hoc signature is expected** — `/"code has no resources but signature indicates they must be present"` appears on every fleet.app, old backups included; launch the new one (`open -a /Applications/fleet.app`) and check its extra port rather than treating the message as a defect.

Related: [[learning-loop-implemented-2026-11-08]], [[two-sessions-one-repo-master-worktree]], [[the-served-face-is-the-installed-copy]], [[rebuild-is-not-commit-push-or-ship]].
