---
name: self-improvement-program-merged-and-delivered-2026-11-07
description: "project 2026-11-07 — the self-improvement program (C0–C9) is COMPLETED, MERGED to master at f6f5f0d (fast-forward), and DELIVERED — the dmg rebuilt, /Applications replaced, the face relaunched on the new build; C2's probe path pinned, the re-dispatch's live firing the one open link"
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

The owner's steer (2026-11-07): *"implement the plan as designed using a worktree to avoid collisions with other sessions"*, then *"merge and rebuild the app"*. The program of [[self-improvement-program-implemented-2026-11-07]] is **COMPLETE** — all ten criteria ticked, the plan's status completed, the recap landed (`docs/recaps/2026-11-07-self-improvement.md`).

**The phases, all landed on branch `self-improvement` in the worktree `../fleet-self-improvement`:** C0 the demand map (`c11ef8d`, `src/demand.rs`), C1 the clustering beat (`4e35f67`), C4 the analysis (`ca86897`), C6 the gaps (`0a17f5d`+`bba79b8`), C5 the run/session scopes (`76ccaf5`), C7 the capability library (`40286e0`), C2 the self-probe (`377155a`), C3 the twin/sham race (`0cfc10d`), C8 the gate, C9 the docs (`46a14f6`).

**The merge — DONE, fast-forward.** Master was an ancestor, so `git push origin self-improvement:master` moved the trunk `1eaba7a..f6f5f0d` in one ref move — no merge commit, no conflict, the branch's twelve commits landed in order. The worktree's branch is pushed for reference.

**The delivery — DONE.** The Tauri dmg rebuilt from the merged tree (`fleet_0.4.0-beta.1_aarch64.dmg`, 9.53 MiB), the old `/Applications/fleet.app` moved aside (not deleted — the destructive gate blocks `rm -rf`), the fresh app installed, quarantine cleared, **ad-hoc re-signed** (`codesign --force --sign -` — the signature leg [[the-served-face-is-the-installed-copy]] records; without it the app dies on launch), and the face relaunched on it — pid 98678 on 3080, its own `/api/memory` answering the new document (the proof fact `basis: validated` with its `source` pointer).

**C2's verification, closed as far as the evidence allows:** the probe's lane path is PINNED (a scripted-lane test drives `probe_outcome` over a fixture child log — the lane answers `block` at 0.9, the grade reads `wrong`, `should_rerun` fires; `3327702`), and the probe ran LIVE on five real rows (four October environmental failures + one fresh false-premise delegation) — the lane's verdicts at 0.42–0.60, all correctly under the 0.80 bar. The re-dispatch's live firing awaits a confidently-wrong verdict: an environment failure or a false premise is not a reason to re-run the child, and the bar is the design's conservatism. Manufacturing one honestly is the next session's.

**The gate:** fmt 0, clippy 0 (all targets), build 0, test 0, harness 984/116 — the test leg needs `BROWSER_CHROME` on this machine (the browser lane's Chrome is not discoverable; the same failure hits the main tree — environmental, the lane's error names the remedy).

**Why:** the article's argument for the phantom gradient — every other instrument reads the demand map — is the program's own sequencing, and the worktree kept a multi-session program landable one phase at a time without colliding with the parallel sessions.
**How to apply:** the branch is merged; the worktree stays for reference. The 7 shape proposals the live clustering beat filed sit in `~/.fleet/proposals/` — the owner's apply is the naming gate. The re-dispatch's live firing is the one open proof. Related: [[self-improvement-program-implemented-2026-11-07]], [[self-improvement-deep-dive-2026-11-07]], [[dream-dmg-installed-face-relaunched-2026-11-07]], [[the-served-face-is-the-installed-copy]], [[house-build-loop-sticky]].
