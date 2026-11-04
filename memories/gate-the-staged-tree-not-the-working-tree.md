---
name: gate-the-staged-tree-not-the-working-tree
description: "the fleet gate must run on the committed tree (git write-tree → detached worktree), because a working-tree green hid four files the persona commit missed and HEAD did not compile"
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

2026-10-28 (fleet, bot-PWA): the four-step gate (fmt/clippy/build/test) was run
against the **working tree** for the whole persona session and stayed green, but
the persona record's `Assistant.persona` field landed without updating four
files (channels/mod.rs, delegate/tests.rs, tool_advisor.rs, tool_audit.rs) —
`HEAD` did not compile, and only a detached-worktree check of the *staged* tree
found it. The same sweep risk cuts the other way: staging a co-edited file
(bridge.rs) pulled another session's uncommitted resume-record work into my
commit, caught only by reading the staged diff.

**Why:** the gate answers for the bytes it compiles. The working tree is a
superset of any commit — it carries other sessions' in-flight work and one's own
uncommitted fixture fixes — so green there says nothing about what ships.

**How to apply:** before `git commit`, gate the tree you are about to write:
`TREE=$(git write-tree); COMMIT=$(git commit-tree $TREE -p HEAD -m x); git worktree add --detach /tmp/check $COMMIT`,
run the gate there (keep the worktree's target/ across re-checks with
`git checkout --detach <new>`, not remove+add), and **read every staged diff for
foreign content** before staging a shared file. Fix forward when a commit has
already shipped broken: history rewriting is the destructive path.

2026-10-28 (f191516, the chat review): the same discipline under a real-time
sibling — the bot session committed `f7c672d` (touching gui.md, a file I also
staged) between my staging and my commit; the tree still came out right
because the index carried their landed gui.md + my edits on top, and the
final proof was `git rev-parse <sha>^{tree}` == the gate's write-tree hash
plus a `git diff f7c672d f191516` showing only my 4 removed lines (mine,
not theirs). A sibling landing a commit mid-gate does not invalidate the run
if your index already held their content — verify by diffing your commit
against THEIRS, file by file, before trusting it.

2026-10-29 (402d947, turn feedback): the blob recipe for a co-edited file
the sibling still holds uncommitted — build the blob as **worktree minus
their hunks** when the foreign bits are two small transforms (app.js: the
`resumed` STATE_LABELS line + the summary's resumed-arm reshape, both
re-shapable with asserted exact-string pairs), and as **HEAD plus only my
edits** when their hunks are complex (bridge.rs: four anchored pairs —
typed small ones, and span replacements whose start/end anchors exist in
BOTH HEAD and the worktree). Assert `count(old) == 1` per pair so a
sibling's later edit fails the build instead of sliding in; the unstaged
`git diff` afterward should show exactly their hunks and nothing else.
Related: [[shared-file-commits-can-sweep-another-sessions-work]].
