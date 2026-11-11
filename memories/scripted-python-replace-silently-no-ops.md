---
name: scripted-python-replace-silently-no-ops
description: "feedback 2026-11-10 — a python heredoc `s.replace(old, new)` silently no-ops when the anchor doesn't match, and an unconditional success print masks it; verify the text landed (grep for the NEW text) before claiming the edit, and never trust a count of prints over a count of changes"
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

Three times in one session (2026-11-10, the grounded-selection arc) a
python heredoc edit reported success while doing nothing: (1) the
feature-doc edits left a DUPLICATE section and silently dropped two
others, discovered only by listing the headings; (2) the failure-review
insert asserted an anchor that didn't exist byte-identical and the
unconditional `print("post-mortem written")` reported success while
`git add` found no changes; (3) the KI-516 lock edit's anchor missed on
whitespace and the test kept failing.

**Why:** `str.replace` returns an unchanged string when the anchor is
absent — no error, no signal. A `print("ok")` after it prints regardless.
**How to apply:** after any scripted edit, assert the NEW text is present
(`assert new_text in p.read_text()`) in the same script, or grep the file
for the new content before moving on; for structural doc edits prefer
reading the region and using Edit/Write directly over find-and-replace
heredocs; and when a later command contradicts an edit's success, suspect
the no-op first. Same family as
[[edit-anchors-with-escape-sequences-double-the-backslash]] and
[[say-only-what-you-have-read]] — a success report is not proof.
