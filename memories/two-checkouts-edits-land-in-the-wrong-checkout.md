---
name: two-checkouts-edits-land-in-the-wrong-checkout
description: "feedback 2026-11-07 — the shell's cwd resets to the MAIN checkout between Bash commands, so a scripted edit with a relative path silently lands in face-plugin-host's copy while the work builds in a worktree; two of five edits landed wrong (one partially reverting another) and the symptom was exact — the served face missing one feature whose API worked; absolute paths in every scripted edit, grep the worktree after each batch, and a harness/API disagreement points at the drift"
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

Implementing the owner-inbox plan in worktree
`~/Projects/fleet-owner-inbox` (2026-11-07), two of five scripted edits
landed in the **main checkout's** `src/gui/assets/app.js` instead of the
worktree's copy: the ROUTES entry (which makes the route dispatch) and the
BUILT list (which keeps the palette from labelling it "later"). The
harness's post-fix run still failed PALETTE and N3; the CDP live proof
showed `#/inbox` rendering the palette's "later" note while
`GET /api/owner-inbox` returned a full document — the API half and the
face half disagreed because one feature was split across two files in two
checkouts.

**Why it happens:** every Bash command's cwd resets to the main checkout
(`/Users/alejandrodelvillar/Projects/fleet`) when the command ends. A
python patch with `p = 'src/gui/assets/app.js'` runs wherever the command
happens to cd — and a batch where one edit carries the `cd` and the next
doesn't splits one feature's edits across two trees, with the later edit
in the worktree partially masking the earlier one in main.

**The discipline:**
- Every scripted edit names the worktree ABSOLUTELY
  (`p = '/Users/.../fleet-owner-inbox/src/...'`) — never a relative path
  while a worktree is in play.
- After each batch, grep the worktree's file for the edit's own marker;
  a success report from the tool is not proof the edit is in the tree you
  are building (see [[edit-anchors-with-escape-sequences-double-the-backslash]]
  for the sibling trap — the same "success is not proof" lesson).
- The diagnostic signature: the API returns the new shape while the served
  face renders the old one (or vice versa) — check WHICH tree the served
  binary was built from and which tree the edits landed in, before
  theorizing.
- Revert the stray edit in the other checkout (`git checkout --` there)
  so the co-tenant's tree stays clean, then re-apply in the worktree.
- The harness failures point straight at the drift: cases that read the
  app's declarations (PALETTE/BUILT, ROUTES-driven mounting) fail while
  the API-backed cases pass.

Same family as [[a-parallel-session-co-edits-your-file]] (the co-tenant's
edits on your files) and [[a-mid-build-park-can-be-committed-by-the-cleanup]]
— a shared machine with several checkouts makes "which tree am I touching"
a question every command must answer.

**The git-operation form (2026-11-07, later the same day):** a
`cherry-pick` of the worktree-convention commit onto master ran in the
MAIN checkout (the cwd reset again) and silently did nothing — that
branch already contained the commit, so the pick was empty ("no changes
added to commit") and the follow-up push answered "Everything
up-to-date" while master still lacked the change. A git operation that
no-ops in the checkout it landed in reads as done. The cure, exercised
exactly as the rule prescribes: `git worktree add
../fleet-master-merge master`, cherry-pick there, push,
`git worktree remove` — one sitting. The rule itself now lives in
CLAUDE.md's topology section (plan worktrees, absolute paths,
worktree-local gates, ref-based merges, one-session deployment); the
no-plan ruling: CLAUDE.md is the designated home for a convention the
worktrees already exercised, and a plan is drafted only if the worktree
lifecycle gets automated — which it then did ([[two-sessions-one-repo-master-worktree]]).

**The same hazard bit the machinery build too:** the ROUTES and BUILT
edits were applied a second time in the wrong checkout (the main one)
while the worktree's copy lacked them — the served shell's
`grep -c "['inbox', 'the inbox']"` returning 0 while the worktree's file
carried the line was the tell, and the fix was the same: revert the
stray, apply absolutely, verify by grep.
