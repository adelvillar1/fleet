---
name: ki-ledger-pipes-shift-naive-column-reads
description: "A naive split-on-pipe column read of docs/known-issues.md misreads rows whose cells carry pipes — KI-331 read as open (it is closed); only ki_ledger.py's parse is honest for counts and statuses"
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

Parsing `docs/known-issues.md` rows with `line.split('|')` and reading
column 5 as status misreports rows whose earlier cells contain literal
pipes (severity parentheticals, evidence quotes). KI-331 came back
"open" on the shifted column and is in fact closed; a naive parse also
found 430 rows where the file holds 501.

**Why:** table cells in this ledger are free text and regularly carry
`|`; only a right-anchored read (status = second-to-last cell) or the
checker's own parser sees the true columns.

**How to apply:** for any count, status, or per-row question, run
`python3 scripts/ki_ledger.py docs/known-issues.md` and, for individual
rows, parse with the same convention the checker uses (right-anchored
cells), never a forward split. Related: [[open-ki-ledger-deep-dive-2026-11-09]].
