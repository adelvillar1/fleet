---
name: a-closed-pre-filter-reader-needs-its-own-reader
description: "feedback 2026-10-29 — the calibration stream's read_day filters by a closed OPS list, so a new row kind (lifecycle) never came back; the fix was a narrow typed reader beside the filter, not widening the shared contract — and the first attempt silently duplicated the serde `op` tag"
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

2026-10-29, the onboarding's skip decisions ride the calibration
stream's existing `lifecycle` rows (the plan's "no new op"). The rows
were written — the day file held them — and `read_day` returned zero.
Cause: `parse_row` accepts a row only if its op is in `OPS`, a closed
ten-item list the demotion/calibration readers depend on; `lifecycle`
is deliberately not in it.

**The fix and the trap in it:** widening `OPS` would have changed what
the demotion reader grades, so the answer is a **second, narrow, typed
reader** over the same day-walk (`lifecycle_rows_recent`), sharing the
layout and the torn-tail tolerance. The first attempt added
`pub op: String` to `LifecycleRow` — and the `Row` enum's serde `tag`
already writes `"op"`, so every line became
`{"op":"lifecycle","op":"lifecycle",…}` and serde refused the duplicate
field. Reading produced nothing again, and only printing the parse error
named it. The field needed `#[serde(default = …, skip_serializing)]`:
the tag writes it, the struct reads it.

**Why:** a closed pre-filter is a contract about what a reader grades,
not an obstacle to route around — honour it with a sibling reader. And a
typed struct for a tagged enum needs the tag field excluded from
serialization or the writer emits it twice.

**How to apply:** when a row written by the shared writer never comes
back from the shared reader, check the reader's accepted-op list FIRST.
When adding the tag field to a row struct, `skip_serializing` it. And
when a read silently returns nothing, print the parse error before
theorizing — "0 rows" and "0 rows parsed" have different causes.
