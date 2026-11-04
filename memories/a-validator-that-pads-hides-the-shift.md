---
name: a-validator-that-pads-hides-the-shift
description: "feedback 2026-10-18 — a checker that pads a short row so its field checks can run hides the shift it is padding for, and the count it reports moves with the shift; a reported count must be the tool's count, not a hand count, because the two can disagree while every check passes"
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

`scripts/ki_ledger.py` fills a row that has too few cells with empty strings so
its field checks (ID, severity, status, next action) can still run. That
padding is what hid the KI triage's own miscount on 2026-10-18: fourteen rows
carried an unescaped pipe — a `— moved to batch 3 —` annotation appended as a
new cell instead of inside the next-action cell — and each read as a 7-cell
row, got padded to 6, and reported no missing field. The shift moved the STATUS
read instead, which is why the parser said 12 open while the sweep's commit
said 217 of 227 closed. Two more rows were wrong in the same window for reasons
no check could see: KI-128's fix had landed in `ddb9152` while its row still
read `open`, and KI-207 sat open in the file with no hand count ever finding
it.

**Why:** a check that repairs its input before checking it converts "this row
is malformed" into "this row reads oddly", and the report stops being evidence
about the input. The count is derived from the same shifted cells, so the
number and the field checks are wrong in the same direction and agree with each
other — which is what makes the disagreement with an outside count look like
the outside count's error.

**How to apply:** report the tool's count, never a hand count, and know what the
tool's checks structurally cannot see. When a padding check and a manual count
disagree, the padding is the first suspect, not the human. A check that pads
should either refuse or report the padding rather than silently repairing.

The same trap fires when *writing* a row rather than reading one. Appending
twelve rows on 2026-10-20, a literal pipe inside a cell — `--capability
task|observe` — split the row into seven cells and my own column check caught
it while a naive one would not have; the ledger's parser tolerates short rows
by padding, so the row read oddly rather than failing. Escape it as `\|`. And
run `scripts/ki_ledger.py` after appending rather than eyeballing: it reports
`rows: N open: M` and the agreement of every row with the header, which is the
count to quote (256 → 268 rows, 21 → 22 open, for the port's twelve). Same
instinct as [[fleet-gate-exit-code-trap]] — a filter that swallows the signal it
was meant to report — and the case is recorded in
[[fleet-ki-triage-pass-one]]; the filing discipline itself is in
[[defects-the-work-closes-belong-in-the-ledger]].
