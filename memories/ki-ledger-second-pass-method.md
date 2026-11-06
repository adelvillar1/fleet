---
name: ki-ledger-second-pass-method
description: "feedback 2026-10-25 — a closed KI row's status text is a claim to re-verify against the code, not a record to trust; the word \"open\" in a row's body is not the row's status"
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

The 2026-10-25 second pass over `docs/known-issues.md` (315 rows) found
the ledger nearly burned down — my first "56 open rows" count was wrong
because I grepped the word `open` anywhere in a row, and closed rows'
bodies quote it constantly ("opens the vault", "openFile"). The honest
read is the status cell's leading claim, and the audit is against the
CODE, not the row: KI-250's status said the vault overlay reached "every
construction site" — true for the GUI's boot, false for `sys1_judges`,
`prepare_with_pin`, the scheduled runs, and the mid-run re-route, which
read the plain dotenv (fixed, c1e3caf). KI-226's fix list had the row's
examples except `Popen`.

**Why:** a closed row's prose overclaims exactly where the fix was
partial, and the row stops being re-read — the same failure
[[a-cheap-partial-fix-leaves-the-line-in-the-description]] names one
layer up.

**How to apply:** in a ledger pass, read the status cell, then verify the
claim in the code; when a claim is short, make it true rather than
re-closing it. **Run `scripts/ki_ledger.py docs/known-issues.md` before
every ledger commit** — it is the repo's own checker and the last
exchange proved why: four rows were malformed (KI-312 and KI-319 missing
the sixth next-action column; KI-252 with a lost closing pipe and a stray
prose pipe; KI-316 with a stray pipe in its evidence — KI-084's
signature). With the cells shifted, the checker read the WRONG column as
the status, so KI-316 and KI-317 — genuinely open — were being counted
as closed. A malformed row does not just look wrong; it silently
misreports the open count. The true open count as of 2026-10-26 is
**four: KI-249, KI-252, KI-316, KI-317** — the earlier "only KI-249"
reading was the damage talking.

**Ledger state as of 2026-10-26:** KI-249 open (its bounds await the
embed tail — the load tail measured at p50 41 ms / max 2,183 ms, and the
per-embed half landed as
[[instrumentation-must-measure-what-the-bound-bounds]]); KI-252 open
(its vector moves await the measurement gate); KI-316/317 open (the
corpus-refresh task-string family). The LayerFS rows are both closed:
KI-313 with aba4fa6's provision record, and KI-314 with f11d2a8 — the
child-execution seam the owner ruled done, so the earlier "deferred by
design" note was superseded the same evening.
