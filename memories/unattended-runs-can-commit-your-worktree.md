---
name: unattended-runs-can-commit-your-worktree
description: "feedback 2026-10-20 — CORRECTED: b235df7 was NOT an unattended run and NOT an unexplained commit. It was the owner's cleanup session, built deliberately to fix the mess an earlier errant UI session made. The lesson is about MY inference, not the commit"
type: feedback
date: 2026-11-04
basis: confirmed-by
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

**Correction (owner, 2026-10-20):** "this was not an unattended session, it was
a session fixing a large number of issues from an errant session that was
working on UI but made a complete mess of the app." My earlier account here was
wrong. `b235df7` is legitimate history: a deliberate cleanup session that
rounded up the mess an errant UI-working session left in the app — the session
that produced the "amateur hour" GUI the owner later rejected. Its message
described the route/session (KI-255) fix while its diff also carried the
in-flight GUI work, because cleaning up the mess required touching those files.

**What was actually right and what was wrong.** Right: read the commit itself
(`git log`, `--name-status`, the real hunks) before explaining it, and report
which files are committed versus still dirty. Wrong: treating an unexpected
commit as an unattended-agent event on the strength of a reflog shape
(commit → reset → recommit → memory-bookkeeping commit) plus "no other sessions
at this time". That inference chain was a guess dressed as a conclusion, and I
wrote the owner a warning about a defect that did not exist.

**Why:** an owner statement that rules out my hypothesis is decisive. "There
are no other sessions" ruled out a parallel human session — it did not license
"therefore an unattended cron run did it", especially when the commit message
and the reflog were both readable and pointed at an ordinary interactive
cleanup. {{owner-stated-facts-are-authoritative}} applies: when the owner states
a fact, act on it rather than building a rival explanation around it.

**How to apply:** before alleging a process defect (an unattended run, a
side-effecting agent, a rogue cron), read the commit and its authorship trail
first. An unexpected commit during long work is a fact to report plainly ("X
landed at T; it carries A and B; my work is still dirty"), not a verdict about
who or what made it. Only claim a defect when the evidence is the defect; an
inference chain that merely fails to exclude a cause is not evidence for it.

Related: [[fleet-gui-editorial-desk-redesign]], [[verify-what-renders-not-what-commits]],
[[shared-file-commits-can-sweep-another-sessions-work]],
[[owner-stated-facts-are-authoritative]].
