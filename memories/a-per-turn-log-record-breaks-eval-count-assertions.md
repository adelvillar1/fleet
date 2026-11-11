---
name: a-per-turn-log-record-breaks-eval-count-assertions
description: "feedback 2026-11-04 — adding a per-turn log Record broke src/eval.rs's exact record-count assertions (\"the log reconstructs the turn\", records.len() == 8/9); the compiler finds field/pattern sites but NEVER numeric count assertions"
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

Adding `Record::Usage` (one bookkeeping line per response) turned the eval
corpus red with "the log reconstructs the turn" — the failures were exact
record-count assertions in `src/eval.rs` (`records.len() == 8` / `== 9`), not
reconstruction bugs: the replay skips the new variant correctly.

**Why:** extends [[assistant-record-field-cascade]] with the third cascade form
the compiler cannot find — a field addition and a new variant both surface as
E0063/E0004 at every site, but a NUMERIC assertion that pins the log's shape
stays green at compile time and only fails at run. The count is an honest
assertion (the log's shape changed), so the fix is updating the number by the
scenario's request count (+1 per response), not loosening it.

**How to apply:** after adding a log record written per turn/response, grep
`src/eval.rs` for `records.len() ==` and `run.records.len() ==` before running
the suite, and derive each new count from that scenario's request count. Re-run
the affected test file — a cancelled run leaves the fix unverified.

**2026-11-04 extensions (the Record::Usage landing):** (1) derive the counts by
temporary instrumentation (eprintln the actual length inside the failing assert),
then set them — the landed counts were 10/10/11, not the analytically guessed
10/10/10. (2) POSITION-based asserts in the same tests shift too: `records[3]`
became `records[4]` etc. — the honest fix reads positions structurally (the
assistant's call found by predicate, not by index) where the check's intent
allows it. (3) A CANCELLED tool call leaves its edit unapplied even if the
command printed nothing alarming — after any cancellation, verify the target
file actually changed before believing the edit landed (this bit twice: the
count edit and the instrumentation both silently no-op'd on non-existent
anchors after a cancel).
