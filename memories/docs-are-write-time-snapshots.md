---
name: docs-are-write-time-snapshots
description: "Owner instruction 2026-10-17 — "check your assertions against git history, the documents might be stale": recaps, plan checkboxes and memory SHAs are write-time snapshots; re-derive state from git before quoting them"
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

Mid-warmup on 2026-10-17 the owner interrupted the state summary with:
**"check your assertions against git history, the documents might be
stale."** Running that check found three real divergences in one tree:

1. A memory cited `ba08055` for the window-aware compaction — a commit a
   rebase left dangling, **not an ancestor of master**; the work actually
   sits at `1060c19`.
2. One commit (`435278f`, ollama cloud + gemma4) had landed *after* the
   latest recap with nothing recording it.
3. The gui-revamp plan's acceptance checkboxes lag the commits — only
   C3/C4 were ticked while G1–G5 plus the whole looks pass were in
   history.

**Why:** recaps, plan checkboxes and even memory SHAs describe the tree
as it was when written; merges, rebases, and commits-by-another-session
overtake them silently. CLAUDE.md's own housekeeping note says its state
bullets go stale; the same holds for every derived document.

**How to apply:** at session start (and before quoting any "what shipped"
claim), derive state from `git log`/`git status` and treat recap, plan,
and memory claims as pointers to verify, not facts to relay. SHAs in
memories are the least durable part — when a memory names a commit,
`merge-base --is-ancestor` it before building on it, and fix the memory
when the pointer is dead. Related: [[owner-stated-facts-are-authoritative]]
(the stated *fact* is trusted; the recorded *pointer* is verified),
[[fleet-memory-corpus-dirties-the-tree]].
