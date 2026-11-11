---
name: a-dated-ruling-supersedes-a-pinned-tests-reason
description: "feedback 2026-10-28 — when an owner ruling changes a contract an old test pins, the test's REASON is what is superseded, not just the assertion; find the pinned test in the same commit, update its comment to cite the new ruling, and keep the parts of the old pin the ruling did not touch"
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

2026-10-28, implementing the bot-PWA plan's R5 ("destructive acts are
held for approval and flagged"). The approvals gate carried a pinned
test: `a_non_bash_tool_is_never_destructive`, whose comment argued
"`write_file` can overwrite a path, but the path is the run's own
business: the class is about the shell's reach, and only bash has it."
The new ruling contradicts that reason — an overwrite that discards
content IS destructive regardless of shell reach — and the console's own
precedent already treated write_file as the risky tool (`replay_tool`
refuses re-running it: "a re-run of write_file would rewrite the tree
with no checkpoint behind it").

**What was done:** `write_file` joined `Gate::is_destructive` (fail-closed
on unreadable arguments, the same rule bash uses), the old test was
renamed to `read_shaped_tools_are_never_destructive_but_the_write_is`,
its comment now cites the ruling and what it supersedes, and the parts of
the old pin the ruling did NOT touch (read/list_dir stay out of the
class even when the path names a credential) were kept and still assert.

**Why:** a pinned test is not a veto — it is a dated record of a decision,
and a newer dated decision wins. Leaving the old test red, or deleting it
silently, both lose the history; the honest move is to update the test so
its comment says what superseded it and when. **How to apply:** before
changing behavior an old test pins, grep the test's comment for its
*reason*; if the new ruling addresses that reason, rewrite the test to
name the ruling (dated) and carry forward every assertion the ruling did
not reach. Related: [[one-goal-many-personas-destructive-is-the-only-gate]],
[[ledger-rows-lag-their-fixes]].
