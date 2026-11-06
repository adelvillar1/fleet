---
name: a-ledger-row-can-name-the-wrong-cause
description: "a KI row's stated mechanism can be false — KI-285 blamed \"a counter instead of reading the table\" when the writer already derived max+1 from the table and the real defect was an unlocked read-modify-write; read the code before acting on a row's next-action, and check whether the repo already fixed that shape elsewhere"
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

Asked to "address the counter issue" (KI-285, duplicate ledger ids), I went to
the code first and found the row's diagnosis wrong: `next_ki_id`
(`src/doctor.rs`) already reads the table and returns the largest present id
plus one — there is no counter. What was missing was the **lock**: the filing
read the table, computed the id, and wrote it back unprotected, so two filings
that both read max=296 both append KI-297 — the lost-update shape the repo had
already fixed one file over for recall counts (KI-235, `filelock::FileLock`).

The fix was then small and provable: hold `FileLock::take(&beside(path))` across
read→max→write, and pin it with a test that releases eight filings off a
`Barrier` and asserts KI-002..009. The essential step was deleting the lock
line and re-running — it failed with eight filings sharing KI-002, then passed
restored. A concurrency pin
that has never been seen red is not a pin — see
[[a-regression-test-must-fail-when-the-bug-returns]].

**Why:** a ledger row is written under encounter pressure, from a symptom, and
its "next action" reads like a spec. Acting on it as written would have
"fixed" a counter that does not exist and left the actual race in place — and
the owner's request ("address the counter issue") carried the same wrong frame,
so the correction had to be surfaced, not silently substituted.

**How to apply:** before implementing a row's named action, open the code it
names and confirm the mechanism; when the stated cause is wrong, say so in the
close-out and rewrite the row's mechanism cell, because the next reader inherits
it. Then look for the same shape already solved elsewhere in the repo (here:
`filelock`, KI-235) rather than inventing a new guard. Residual exposure should
be named too: a hand-typed id and a stale-base commit can still collide, and
`scripts/ki_ledger.py` reports duplicates but nothing runs it — wiring it into
a gate or hook is the owner's policy call, since hard rule 2 fixes the gate's
four commands. Related: [[a-validator-that-pads-hides-the-shift]] (the same
lint found KI-285 was missing its severity cell since it was written),
[[fleet-ki-triage-pass-one]].

**Fixed 2026-10-23, `faa8483`** on `master`: `file_ledger_row` takes the
sibling `FileLock` before reading, the row's mechanism cell was rewritten and
closed, and the ledger's Rules section gained "an id comes from the table".
`scripts/ki_ledger.py` then read 297 rows / 33 open / zero problems. Residual
named there, unchanged: a hand-typed id and a stale-base commit can still
collide, and the lint runs only when someone runs it.

**2026-10-24 extension — the mechanism can be right and the row still be
wrong.** Auditing the owner-decision rows against commit history found four
that needed no owner at all: KI-117 said "blocked on owner decision" while its
own next-action carried the ruling ("keep the dependency as it is"); KI-135's
gap had closed under the capability surface (`measure`→sdm1, `imagine`→gen1,
commit 5897c86); KI-309's mis-recorded shas were already corrected in the
superseding fact, which followed the row's own rule; KI-024 was overtaken by
the cua-driver 0.37.0 update. One row cited the wrong FILE (KI-268's rank test
lives in `src/gui/server.rs:3151`, the /api/rank route, not route.rs), and one
was waiting on a condition that had silently landed (KI-190's eval.rs round).
The audit closed four, re-pointed one, unblocked one, in commit `8f613c4`.
The rule that generalizes KI-017's stale next-action: **a row is a write-time
snapshot exactly like a recap — verify it against `git log` on its named
files and the live tree before honoring its next-action**, and when a
"waiting on the owner" row is found settled, close it naming the evidence
rather than re-asking the owner a question their own history answers.
Related: [[external-repo-ki-closures-2026-10-24]].
