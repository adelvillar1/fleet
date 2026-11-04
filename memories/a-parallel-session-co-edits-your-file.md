---
name: a-parallel-session-co-edits-your-file
description: "Observed 2026-10-31 — a parallel session's uncommitted work rides your bridge edits (the replay read's compactions field arrived mid-build); fold complementary hunks in deliberately and never assume the diff is yours"
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

Observed 2026-10-31 while building `quota-failover-and-compaction-count`:
`src/gui/bridge.rs` carried an **uncommitted** change I had not written —
the replay read's `compactions` field with a comment citing the owner's
same-day ruling (the parallel shakeout session answering the same
request from the other direction). The `git status` I ran at the start
of the router review already showed the file modified.

**Why:** the full-diff inspection prevented both failure modes —
overwriting their hunk with my own edit of the same region, and
silently shipping an unreviewed third-party change as mine. My own
tree-wide constructor sweep then left one wrong-indentation line in
that same file, which only the fmt check caught: co-edited files get
both hand-edits *and* mechanical edits, so neither eye nor style alone
covers them.

**How to apply:** at the start of any file edit, `git diff` that file
if it was clean at session start and is dirty now. Diffs that are
complementary (their hunks carry a different feature) are folded in
deliberately and credited in the commit message ("folding the parallel
session's replay read"); diffs that overlap the same lines need the
other session's outcome read first. The standing rule's name:
[[shared-file-commits-can-sweep-another-sessions-work]] — that memory
covers committing someone's work by accident; this one covers BUILDING
in a file someone else is mid-edit in.

**2026-11-01's second sighting, and the tell that names it:** after I
appended one row to `docs/known-issues.md` through `fleet ki append`
(single-row writer — it appends and renames, never re-renders), the
diff carried **~30 severity-cell normalizations I had not written**
(`note`→`gap`/`risk`/`improvement`, `warning`/`flake`/`hang`→`defect`,
`environment`→`environmental` — KI-391's drift being swept) and the
open count moved 32→30 while my own row added one. The rule that caught
it: **when a diff shows more than you did, read the writer's code
before claiming or "fixing" anything** — proving `append_ledger_row`
was append-only (one `push_str` + atomic rename, no row re-render) is
what established the sweep was someone else's, so the tree was left
untouched and flagged to the owner instead of reconciled by guesswork.
