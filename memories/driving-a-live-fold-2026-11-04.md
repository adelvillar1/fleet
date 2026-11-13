---
name: driving-a-live-fold-2026-11-04
description: "feedback 2026-11-04 — the reliable recipe for proving a compaction fold live: models dodge tool-call puzzles (three one-turn answers), so force the two-turn shape with bash chaining and the owner's {\"fold\": true} control file; the usage event is readable on GET /events, not /api/stream"
type: feedback
date: 2026-11-04
basis: confirmed-by
tier: durable
shape: []
phrases: []
missing: []
recall-count: 285
superseded-by:
valid-until:
---

The context-health live proof (2026-11-04, scratch home
`/tmp/fleet-health-proof`) took 17 real runs to land a fold, and the failures
were instructive:

- **Models dodge tool-call puzzles.** Tasks meant to require `read_file`
  ("report its second line verbatim; do not answer from memory") were answered
  in ONE turn — the model guessed or skipped the read — and `maybe_compact`
  only fires at a boundary BETWEEN turns of one run, so one-turn runs never
  fold even when the conversation is over budget. Sequential queued runs also
  never fold (each is its own one-turn run).
- **The reliable recipe:** a task that chains `bash` (`wc -l`, then read, then
  answer) plus, mid-run, the owner's own forced-fold control — write
  `{"fold": true}` to `<FLEET_HOME>/sessions/<id>.control` (the watcher
  consumes it; compact_now fires at the next boundary regardless of budget).
- **The usage event streams on `GET /events`** (SSE), not `/api/stream` —
  catching it live while a run executes proves the wire (budget, folds, share,
  cache_read) without any DOM drive.
- The window override for a cheap fold: `model_settings.<model>.context_window`
  under the provider entry that ACTUALLY serves (the router picked stepfun, not
  deepseek — put the override under the serving provider or the fold's per-pair
  lookup never sees it).

**Why:** a live fold is the plan criterion that cannot be unit-pinned honestly,
and the naive way (stuff a conversation past the budget) fights both the 64KB
run-body cap and model compliance. **How to apply:** for any fold live proof,
pin deepseek-flash, chain bash, write the fold control mid-run, and read the
assertions off `/events` + the compaction record. Related:
[[context-health-research-2026-11-04]], [[fleet-gui-is-a-launchagent]].
