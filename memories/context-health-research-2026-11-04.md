---
name: context-health-research-2026-11-04
description: "project 2026-11-04 LANDED — the context-health plan completed 11/11 (8117496 spine, d23ea4c face, 077ee01 docs, pushed face-plugin-host): the budget follows the serving model everywhere, the working line reads health not turns, the fold logs what survived, the cache parse is per-format, the resume seed reads the log"
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

2026-11-04: the deferred budget discussion ([[turn-budget-shape-is-a-deferred-discussion]])
became the ask (measure context health, show it on the conversation beside the
working spinner), was drafted (0f97627), steered twice mid-build, and **LANDED
WHOLE** — `docs/plans/2026-11-04-context-health.md` status complete, **11 of 11
criteria ticked with evidence**, pushed to origin/face-plugin-host through
**077ee01** (spine **8117496**, face **d23ea4c**).

**What shipped:**
- `UiEvent::Usage` carries budget_tokens (from `budget_for(serving_window())` — a
  mid-run pin changes the budget with the model), folds, verbatim_share,
  cache_read_tokens; `describe()` maps all four.
- The state read derives the budget for the NEXT run's model (pin → log's
  `End.served_model` via the new `route::declared_window_for_model` by-model scan →
  configured model), with `budget_declared: false` naming the floor.
- The working line reads occupancy, the folds, and turns-to-the-fold (only on
  three climbing readings); the turn/exchange count left the surface. The head
  meter: calm / high / fold-imminent (the `full` band retired), hover names the
  declaring model (or the named floor), folds, share, cache-hit ratio.
- `session.rs` gained `Survival` + `Compaction.survival` + `Record::Usage`
  (bookkeeping, replays to no message); `compaction.rs` gained `verbatim_share()`
  + `survival_check()`; the fold runs the survival check over task/goal/deliverable
  paths/ledger items. `Usage.cache_read_tokens` reads per format (Anthropic rename,
  DeepSeek `prompt_cache_hit_tokens` alias, OpenAI nested
  `prompt_tokens_details` captured + folded by `Usage::absorbed()` at the two chat
  parse sites — never serialized).
- The resume seed: the state read takes the log's last usage record + fold count.

**The live proof** (`/tmp/fleet-health-proof`, machine credentials in env): 17
real runs; the router served step-5-preview (budget 614,400 — follows the pick);
a mid-run config override dropped it to 100,000 without restart; the fold fired
via the owner's `{"fold": true}` control file — 198,036 → 42,186 tokens, survival
kept 1/1; the wire event `{"budget":100000,"cache_read":3968,"prompt":49724,
"share":97}` caught on `/events` (not `/api/stream`); the state seeded the reopened
conversation. See [[driving-a-live-fold-2026-11-04]].

**Verification:** gate green (1,838 lib + 164 integration + eval, harness
844/95); the eval corpus's log-shape counts re-measured 10/10/11 and
position-based asserts shifted one slot
([[a-per-turn-log-record-breaks-eval-count-assertions]]). Delivery: binary
through a new inode + bootout/bootstrap (KI-408 runbook), served face verified,
dmg rebuilt and its packaged binary checked for the new markers, KI-426 filed
(load-sensitive layerfs stubs). Recap:
`docs/recaps/2026-11-04-context-health.md`.

**Honest residues:** the 100k floor stays as the no-declared-window fallback
only (the owner ruled "as long as it's not pinned to an individual model" — the
fallback is fine, its DeepSeek-tuning comment is pre-existing); survival's prose
containment is crude with the manifest seam deferred; the one-arithmetic seam
the proof caught — a window override under a DIFFERENT provider than the one
that served reaches the gauge's by-model scan but not the fold's per-pair lookup
(the fold is the correct one). Related: [[router-picks-the-model-for-usage]].
