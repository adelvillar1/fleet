---
name: tool-declaration-assertions-read-names-not-just-counts
description: "feedback 2026-10-23 — changing tool declarations only breaks a test if the test reads names; assertions that pin positions (`second[4..8]`, `records[5]`) redden on any new sibling RECORD even when nothing regressed, so the honest test matches by kind or by name and the positional ones get a finder helper"
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

Three declaration-shape changes in one session (adding `edit`/`grep`/`glob`/
`todo`, then tiering, then a new `ToolResolution` record per call) taught the
difference between a test that catches a real regression and one that merely
counts positions:

- **Positional declaration slices** — `assert_eq!(&second[4..8], &[…lane
  tools…])` — passed while the *content* was wrong (fields moved to
  `[15..19]`) and only reddened once the next insertion shifted them. A test
  that pins an index reddens on every sibling added, which teaches you to
  bump numbers rather than read them.
- **Counts alone** (`len(), 12`) caught the real bug once — the tiering
  appended the lanes a second time — but they also had to be bumped six times
  for legitimate additions, which is exactly the mechanical edit
  [[adding-a-tool-breaks-the-declaration-counts]] warned about.
- **A new bookkeeping record** (`ToolResolution`) shifts every
  `records[3]`/`records[5]` fixture even though the conversation is
  unchanged; the fix is a `first_result(&records)` finder, not five new
  indices. `eval.rs` counts (`records.len() == 6`) had to move too.

**Why:** a positional assertion silently converts "I added a sibling" into
"the product broke", and the cheapest response to that is a number bump —
which is also the cheapest response to a real break. Name-based assertions
(`contains(&"mcp__sem1__probe")`, `find_map(|r| matches!(r, …))`) survive
insertions and still fail when the thing is actually missing.

**How to apply:** when you must change the tool set or the session record
enum, expect a red wave and answer it by *reading* each failure — names for
declarations, a kind-finder for records — before touching a number. Related:
[[adding-a-tool-breaks-the-declaration-counts]],
[[a-test-that-passes-alone-has-not-been-tested]].
