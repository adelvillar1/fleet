---
name: hybrid-retrieval-delivered-2026-11-08
description: "project 2026-11-08 — the hybrid retrieval layer is IMPLEMENTED, COMPLETED and MERGED to master at 29daca3 AND DELIVERED — the dmg rebuilt from that master (10.09 MiB), the app installed by the owner, the face relaunched and serving on 3080 (pid 98678, /api/home 200), and the installed binary proven to carry the new hybrid_search tool"
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

The owner's goal — "implement plan as designed using a worktree for the plan
to avoid collisions with other work" — ran on the hybrid-retrieval plan and
closed the whole loop, including the delivery the owner then asked for
("rebuild the app").

**The lane.** `/Users/alejandrodelvillar/Projects/fleet-hybrid-retrieval`,
branch `hybrid-retrieval`, forked from master at `de55b62`. The plan completed
(status `completed`, all ten criteria ticked), the merge ran as
`git push origin hybrid-retrieval:master` — a fast-forward
(`de55b62..29daca3`) — and the lane was pruned (8.9 GiB freed). The plan's
status is `completed` on master; `plan_check.py` passes.

**The delivery.** `cargo tauri build` from master at `29daca3` produced
`fleet_0.4.0-beta.1_aarch64.dmg` (10.09 MiB, 21:24). The owner copied the app
to `/Applications/fleet.app` (binary dated 21:23). The face was relaunched
(`open -a fleet`) and serves on 3080 — pid 98678, `GET /api/home` 200 (30,484
bytes), `GET /app.js` 200. **The installed binary carries `hybrid_search`
three times** (the tool name, its spec, its dispatch) — the proof the new work
rides the running app, not just the repo. The `BM25_K1`/`RRF_K` constant names
do NOT appear in the binary: a release build strips symbol names, so a
bundle-marker grep must use a string literal from the served path
([[tauri-dmg-rebuilt-2026-11-05]]'s marker lesson, re-earned).

**Why:** the completion answer had named the code state without the delivery
state — [[rebuild-is-not-commit-push-or-ship]]'s exact failure — and the
owner's follow-up made the delivery the work. The dock app is its own door and
ages independently of both the dmg and the LaunchAgent face
([[tauri-dmg-rebuilt-2026-11-05]]).

**How to apply:** ALL LEGS CLOSED. The local machine's face runs the new build.
Related: [[fleet-search-and-recall-vs-zvec-2026-11-08]] (the analysis and the
plan), [[two-sessions-one-repo-master-worktree]] (the lane machinery),
[[dream-dmg-installed-face-relaunched-2026-11-07]] (the previous delivery),
[[tauri-dmg-rebuilt-2026-11-05]] (the dmg/install state and the marker lesson).
