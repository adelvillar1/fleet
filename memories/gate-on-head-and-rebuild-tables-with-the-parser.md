---
name: gate-on-head-and-rebuild-tables-with-the-parser
description: "feedback 2026-11-01 — check HEAD and GATE on it before committing; edit markdown tables with the checker's own parser, never hand-sliced cell boundaries"
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

Two traps from the 2026-11-01 ledger session, both cost a wrong
commit or a corrupted file:

1. **Gate on HEAD, don't just read it.** `git rev-parse --abbrev-ref
   HEAD` printed `design-parity` in the same command block as my
   commit — I ran them together and the commit landed on the other
   session's branch anyway. The rule: the branch check and the commit
   must be one gated act — if HEAD is not master, stop and move the
   work (cherry-pick through a second worktree), never commit and
   notice after.

2. **Rebuild ledger rows from the checker's parser.** A status-close
   script that hand-sliced the row at `| KI-xxx | ` + offset anchored
   at the SEVERITY cell (the id prefix ends there), not the status
   cell — five rows became five-cell ghosts in one write. The safe
   method: parse with `scripts/ki_ledger.py`'s `split_cells`, replace
   `cells[4]`/`cells[5]`, rebuild `'| ' + ' | '.join(cells) + ' |'`,
   and confirm with the checker before anything else.

**Why:** both failures looked like the work was done — the commit
existed, the rows "closed" — and both were invisible until a later
check (the branch log, the checker's cell-count problems).

**How to apply:** any commit in a possibly-shared checkout gets a
gated HEAD check; any ledger row edit gets the parse-rebuild, never a
hand-sliced offset. See [[two-sessions-one-repo-master-worktree]].
