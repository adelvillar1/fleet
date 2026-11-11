---
name: edit-anchors-with-escape-sequences-double-the-backslash
description: "feedback 2026-11-06 — an Edit old_string containing a literal backslash-u escape that exists AS TEXT in the file (e.g. items.join('\\u0000') in app.js) must be passed with a DOUBLED backslash (`\\\\u0000`); the parameter unescapes `\\u0000` into a real NUL character and the match silently fails — and a success report is not proof the edit is on disk, re-read the region when a later run contradicts it"
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

**Why:** editing `app.js`'s `paintQueue` key line — the file literally
contains `items.join('\u0000')` (six characters: backslash, u, 0, 0, 0, 0).
Passing the anchor with `\u0000` unescaped made the tool parameter carry a
real NUL character, so the match could not hit the source text. The first
attempt still REPORTED success (a co-tenant's stale-buffer write was also
in flight — two causes stacked), and the next harness run failed with
`[object Object]` rows because the old string renderer was still on disk.

**How to apply:** when an Edit anchor contains a backslash escape that is
literal source text (regex-ish strings, `join('\u0000')`, `printf '\n'`
patterns, doc comments quoting escapes), double the backslashes in the
parameter (`\\u0000`) so the tool receives the characters the file holds.
After any batch of edits, re-read or re-grep the touched region before
building on it — a success report says the tool ran, not that the file now
says what you meant (same family as
[[a-parallel-session-co-edits-your-file]]'s re-read-after-every-batch, and
[[verify-the-binary-is-newer-than-the-source]]'s check-the-artifact rule).
