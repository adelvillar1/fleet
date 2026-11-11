---
name: a-worktree-forked-from-a-working-branch-misses-master
description: "feedback 2026-11-08 — a plan worktree forked from a working branch (face-plugin-host) lacks master's newer work, so a feature whose feeds ride master's machinery cannot build; fork from master or merge it in before the first compile"
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

The learning-loop lane was created from `face-plugin-host` (the branch the main checkout sat on), and the first `cargo check` failed on symbols that exist only on master — `calib::LearnProbeRow`'s siblings, the demand map, the improve store. The Learning plan's C8 feeds ride the self-improvement machinery, which landed on master AFTER `face-plugin-host` diverged: master was 23 commits ahead, the branch 5 ahead, and none of the branch's commits were on master.

**Why:** `git worktree add ../fleet-<slug> -b <slug>` takes whatever ref you give it, and the session's cwd is the main checkout's branch — so "the current branch" is not "the trunk". A lane forked from a working branch is a fork of that branch's moment, not of the trunk's, and the divergence is invisible until a symbol the plan assumes is missing.

**How to apply:** fork plan worktrees from `master` (the convention's own default), or merge master in before the first compile and re-run the gate on the merged tree ([[two-sessions-one-repo-master-worktree]]'s "the base moves under a lane"). When the merge conflicts, both sides' additions are usually real — here the definition bound's doc comment carried two independent crossings (the hybrid retrieval's and the learning record's) and the resolution named both. Related: [[two-checkouts-edits-land-in-the-wrong-checkout]], [[a-parallel-session-can-reintroduce-your-fixed-lints]].
