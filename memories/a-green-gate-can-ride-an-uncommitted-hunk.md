---
name: a-green-gate-can-ride-an-uncommitted-hunk
description: "Feedback 2026-10-31: the gate runs on the WORKING tree, so a fix hunk that never got committed leaves master's test tree uncompilable while every local run stays green — notice fix-shaped hunks in files you must touch and either carry them in your commit (named in the message) or flag them."
type: feedback
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

The compaction plan's commit (8b3564b) shipped `src/tui/app.rs` without
the `compactions: None` constructor field its own `SessionSummary`
change required — the fix sat uncommitted in the working tree, so every
local `cargo test` passed and master's committed test tree did not
compile. The density session found it only because `git diff` on a file
it had to touch showed the foreign one-line hunk.

**Why:** the gate judges the working tree, not the commit; an uncommitted
fix the gate depends on is invisible until someone checks out the
commit. Green locally is not green on master.

**How to apply:** before committing a file that was already `M` in
`git status` at session start, read its full diff — a one-line
fix-shaped hunk (a missing struct field, a constructor arm) rides your
commit with a sentence naming it, rather than being left to break the
next clean checkout. The density commit carried it named in the message
(`5b14bd9`); the related discipline is [[gate-the-staged-tree-not-the-working-tree]]
— run the gate on the committed tree, not the working one. Related:
[[verify-the-binary-is-newer-than-the-source]]. **2026-11-01's push
form:** when the owner orders a push, the pre-push inventory must
extend to *staged-vs-unstaged* — check `git diff --cached` beside
`git status`, because a commit can carry hunks the author staged
deliberately AND hunks that were already sitting in the file (the
parallel-session sweep class); the push itself is only as honest as the
commits it lifts, so list what the push carries (`git log @{u}..HEAD`)
and say whose commits they are. **Second 2026-11-01 form:** a commit
made on the WRONG BRANCH (master, when the lane was design-parity) is
fixed by cherry-picking onto the branch from a `git worktree add` and
then `git reset --soft` + `git checkout HEAD -- <paths>` on master —
NEVER `reset --hard` with an uncommitted corpus cluster in the tree;
`git checkout -- <paths>` restores from the INDEX (still carrying the
staged commit) and must be `checkout HEAD --` instead.
