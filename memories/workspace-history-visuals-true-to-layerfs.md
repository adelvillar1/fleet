---
name: workspace-history-visuals-true-to-layerfs
description: "project 2026-10-29 — the honesty pass over Workspace History: a head-null merged branch keeps its lane/Work row/ring-graft, forks sit at forkedFrom not base, lanes follow parent chains, counts say "the newest N", lww parses objects"
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

2026-10-29 (fleet, commit after the chat review + turn feedback): the owner
asked that the workspace-history visualizations "stay true to the LayerFS
project". An audit against the pinned substrate (`ad78914`) found four
material drifts, all fixed and pinned (nine new harness checks, 293 across
30 cases; four view tests):

- **A merged branch with `head: None` was drawn as idle.** `get_history`
  skips a head-null branch's commits, so filtering on `head` printed
  finished work as "forked, no work" AND dropped the merge proof (the
  graft filter needed a commit slot). Now state `merged` keeps the lane,
  the Work row (`merged · the layer carries its commit`) and a ring on the
  band named by the Layer's `sourceCommit`; the dashed stem appears only
  when that commit is on the page.
- **Forks were placed at the Layer `base`**, never `forked_from` (a
  Commit) — every fork inside one layer collapsed onto one spine slot.
- **Lanes were time-ordered**, implying linear parentage; they now walk
  each commit's own `parent` (a parent outside the page starts a new
  segment). Merge evidence stays where the substrate puts it: the Layer
  (`sourceBranch`/`sourceCommit`), not the commit.
- **Counts described the page as the Store**: "the newest N" when the
  substrate's caps cut it (64 branches / 12 KiB layers / 56 KiB commits),
  the same note on the Files tabs, commits on cut branches counted under
  the tree, `lww` parsed as the wire's OBJECT shape
  (`{path, kept, deleted, rule}` — it was parsed as strings and always
  empty), dedupe computed from bytes, `↓ L<k>` marked `(page)`.

**Why:** the vocabulary (band/leaf/lane/graft) is ours to draw, but the
facts must be the substrate's; the audit's tell for every drift was a UI
sentence claiming more than the read carried ("a leaf for every commit",
"forked, no work").

**How to apply:** when drawing a substrate, filter on the state the
server derived, never on a nullable field that a cap can void; place an
edge from the field the substrate calls the edge (`forkedFrom`,
`sourceCommit`); any count off a capped read says "newest" or names the
cut. Docs: `docs/features/workspace-history.md`,
`docs/plans/2026-10-29-workspace-history-truth.md`. The untracked
`docs/architecture/system.drawio`/`.svg` are a truncated fleet
routing/tool/MCP sketch with no generator — NOT LayerFS visualizations;
deleting them is the owner's call. Related:
[[layerfs-history-instrument-2026-10-26]],
[[layerfs-worker-store-recovery]].
