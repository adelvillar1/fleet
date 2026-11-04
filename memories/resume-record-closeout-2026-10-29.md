---
name: resume-record-closeout-2026-10-29
description: "project 2026-10-29 — the 2026-10-28 resume-record work (a continued conversation reads `resumed`, not terminated) closed out and committed as 364c09c: gate exit 0, KI-365 filed and closed, plan completed, FS updated, recap written"
type: project
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

The resume-record slice (drafted 2026-10-28, implemented in the tree)
closed out 2026-10-29: full gate green by exit code read directly (fmt/
clippy/build 0, test 0 — integration target 149 passed, 3 ignored),
KI-365 filed-and-closed for the defect ("a continued conversation reads
as terminated": summarize read only the log's last line, work_state
mapped ended to the terminated words, Session::open recorded nothing;
the fix is the `resume` record + the whole-file fold + the `resumed`
state word), and `docs/plans/2026-10-28-resume-record.md` → completed.
Session recap at `docs/recaps/2026-10-29-face-icons-resume-closeout.md`
(ledger 365 rows, 31 open).

**Committed on the owner's word** ("update the docs then commit"):
`364c09c` carries the resume slice + plan + the whole ledger (KI-365
plus the earlier rows' cell repairs riding); the icons landed as
[[turn-controls-icons-composer-verbs]] `a563b7e`; docs as `9a11b7d`;
memories churn as `0a41b3a`.

The FS `resumed` word was applied the same turn — with a location
correction: the proposal had named the stop-reason table, but `resumed`
is a derived STATE word, never a stop reason, so it went into the two
places the state vocabulary lives (the work-list state-words list and
the run's-end bullet). When applying a proposed doc edit, re-derive
where the fact belongs before pasting it. Named-not-touched: the tree's
strays (home.txt, rr.json, the `.lock` sidecars, mod.rs.bak-resume,
system.drawio/svg, services/browser/generated/) per the
never-delete-unknowns rule. See [[relaunch-is-not-a-code-event]].
