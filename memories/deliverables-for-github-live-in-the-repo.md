---
name: deliverables-for-github-live-in-the-repo
description: "feedback 2026-10-21 — screenshots taken for the owner must be committed into the repo (docs/images/), not left in the session's visualization directory, or GitHub shows nothing; verify with a sha comparison, and assume a stale tab before a failed push"
type: feedback
date: 2026-11-05
basis: confirmed-by
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

The owner reported "none of that was there" after checking GitHub: the
surface screenshots lived only in the session's Gen-UI visualization
directory (`~/.zcode/v2/visualizations/<session>/`), which the repo never
sees. Fixed by committing them into `docs/images/` (JPEG-85) and
referencing them from the README and the functional spec.

A second report — "the files look like nothing was updated" — was a
stale browser tab: local, `origin/master`, and GitHub's default branch
were already at the same commit.

**Why:** a deliverable the owner will view on GitHub is only real when it
is in the repository; the visualization directory is for the session's
inline rendering, not for delivery.

**How to apply:** when a task produces images or other artifacts the owner
will look at on GitHub, commit them under `docs/images/` (or the relevant
docs home) in the same increment, add the pointer to CLAUDE.md's index,
and verify with `gh api repos/<owner>/<repo>/contents/<file>` comparing
the returned sha against `git hash-object <file>` — for every file the
task touched. When the owner reports GitHub looking unchanged, check the
remote state first (cache is far more likely than a failed push), then
tell them to hard-refresh. Related: [[rebuild-is-not-commit-push-or-ship]]
(delivery has legs; "committed" is not "visible").
