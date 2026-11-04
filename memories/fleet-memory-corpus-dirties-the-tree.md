---
name: fleet-memory-corpus-dirties-the-tree
description: "Fleet's git-tracked memory corpus dirties the working tree through ordinary use — every recall bumps `recall-count:` in a tracked fact file, so the bumps accumulate uncommitted and get mistaken for session work; commit them as bookkeeping, and ignore the untracked tool-internal noise"
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

Fleet tracks its memory corpus (`memories/*.md`) in git, and the substrate
writes to those files during ordinary use: each recall bumps a fact's
`recall-count:` line, and the recall-index rebuild writes beside the tree
(gitignored as `/memories/.recall-index.json` and `/memories/*.tmp`). So a
`git status` after any real Fleet session shows a batch of modified facts
that are **bookkeeping, not session work** — one `recall-count` line each.

Disposition that worked (2026-10-16, session wrap-up): commit them as their
own small increment with explicit paths ("the recall counts follow the
recalls"), rather than folding them into a feature commit or leaving them to
accumulate. Ten facts had piled up across several sessions before this.

It also recurs *within* a session, not just across them: writing the recap
and refreshing `CLAUDE.md` at wrap-up recalled the same eleven facts again
and re-dirtied the tree, so a second bookkeeping increment was needed after
the feature commit had already landed. Check `git status` once more at the
very end rather than assuming the first bookkeeping commit cleared it.

Do not mistake the other untracked entries for repo content either:
`.DS_Store` files, `.v2c/`, and `.video_agent/` are tool-internal noise
(plugin roots from the video tools) — leave them alone, and never reach for
`git add -A`, which the house rules forbid anyway.

Recurrence confirmed 2026-10-20: sixteen facts had drifted from ordinary recall
during the GUI session, so the wrap-up needed `git add memories/*.md` and its
own `chore(memories): the recall bumps` commit (`022fbd3`) placed *before* the
feature commit — the tree's only remaining untracked entries were `.zcode/`
and a plan draft, neither ours to commit.

**The cron jobs' corpus snapshots are a second, bigger class (2026-10-23).**
Sixteen dated `memories/*-corpus-*.md` files sat untracked — one snapshot per
project per scheduled window, written through `remember` by the
`corpus-refresh-*` jobs. No such file has ever been committed, so left alone
they accumulate forever and make every warmup's `git status` look like a mess
the owner attributes to the app ("seems like it made a mess"). Disposition:
added `/memories/*-corpus*-20[0-9][0-9]-*.md` and
`/memories/*-corpus-*-items*.md` to `.gitignore` — keyed on the writer's own
stamp (a date, or an item count), never on the word `corpus`, so a durable fact
named for its claim (`blend1-corpus-refresh-never-converges`) stays tracked.
Prove a new rule before committing it: feed the real names plus negative
controls to `git check-ignore --no-index --stdin` and require the exact count.
The files stay on disk because recall reads them; only git stops seeing them.
Their content is also a liability — they carried a wrong date (`2026-10-24`
for runs whose blend1 stores were written Oct 22 19:57 CDT), so a note taken
from a snapshot is worth checking against the store's mtimes.

**The substrate also corrects itself, and the correction must outrank the first
fact (2026-10-23).** Two `nomenclature-*` facts were written one day apart with
opposite bodies ("scans backtick spans only" vs "scans whole content"); the
later one carried `superseded-by` correctly, so **commit a superseded pair
together** — committing only the newer file would leave the wrong body live in
the tree. The same batch held a wrong date (`2026-10-24` for runs whose blend1
stores were written Oct 22 19:57 CDT) that the third ledger sighting corrected;
a memory written from a scheduled job inherits the job's clock, so check a date
claim against the store's mtimes before trusting it.

**The whole dirty batch can be churn-only — verify per file before treating
any of it as work (2026-10-26).** When the owner asked "why do we have so
much garbage in the repo?", the measured answer was that all 18 modified
`memories/` files were PURE bookkeeping — a per-file diff classified against
the bookkeeping lines alone (`recall-count`, `superseded-by`) found zero
content changes — plus 9 untracked files (new facts other sessions wrote,
plus `.archive/` moves). None of it is garbage; it is the substrate working,
sitting uncommitted because nothing commits it on a cadence. The check that
settles the question in one pass: for each dirty file, count diff lines that
are neither bookkeeping keys nor blanks — a count of zero everywhere means
the whole batch is one `chore(memories)` commit away from clean.

**Resolved for good 2026-10-26 (`4612e8d`):** the cadence now exists —
`scripts/daily-maintenance.sh` (LaunchAgent `local.fleet-maintenance`,
daily 09:20) commits the churn itself as leg three: `git add -- memories/`
then `chore(memories): the corpus' counters, archives and new facts`,
memories/ paths ONLY (the explicit-path rule keeps a concurrent session's
code WIP out), skipped when clean, logged and retried tomorrow when the
index is locked. Safe by construction: every corpus file is written whole
through the guarded write path, so any instant snapshot is a complete
fact. First run landed 25 files (`4f31786`) and the tree read clean.

Related: [[fleet-roster-migrated-from-router-kit]] (the corpus gained a
ruling fact the same session, written through `fleet remember`),
[[fleet-dogfooding-viability-verdict]],
[[shared-file-commits-can-sweep-another-sessions-work]],
[[fleet-repo-garbage-taxonomy-2026-10-26]].
