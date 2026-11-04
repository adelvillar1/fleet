---
name: fleet-deck-triad-autofit-and-layout-index
description: "2026-10-19 — the triad blueprint's deck side is built end-to-end — KI-251 closed (deck::auto_fit, the GLiDE self-correction loop: compress to measured budgets, balance columns, Layout::roomier swap, consultant verdict frozen) and KI-252 move 1 landed (the layout template index — eight embedded layout descriptions matched by cosine over the local EmbeddingGemma, exposed as the deck tool's suggest mode and declared in its spec); the deck run's event now carries the grader's faults plus both decks (as written …"
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

The owner re-sent the triad blueprint (EmbeddingGemma 2 vectors + Jev/GLiDE
deterministic gates + a small LLM for text) naming Liquid's LFM2.5-24B-A2B, the
sys1 pool, and the local EmbeddingGemma 2 — with "it was supposed to be built."
The audit verdict: the triad's bones were built ([[blend1-is-the-latency-triad]]
— blend1's lane IS LFM2.5-24B-A2B + sys1 behind the GLiDE gate, live and
verified; the local embedder on 127.0.0.1:8901 answers the type-ahead and
recall) but the deck pipeline's self-correction loop and vector layer were not.

**KI-251 closed (`25cdbaf`, gate green 1126) — `deck::auto_fit`, the
blueprint's step 4b, deterministic, no model call:**
- text compressed to the grader's own measured budgets (`holds` ×
  `CHARS_PER_LINE`), with the bolded takeaway kept whole — a clipped bullet
  keeps its `**span**`, because a truncated bullet that lost its emphasis
  would lose the rule that made it scannable;
- headlines trimmed to their box with `is_conclusion` re-verified; heaviest
  columns balanced toward their lightest;
- `Layout::roomier` — the swap ladder (split_two_column/comparison/timeline/
  metric_row/chart → full_bleed) for text compression must not cut away;
- the consultant verdict is frozen: `auto_fit` snapshots `check(&deck)` as a
  baseline and reverts any pass that ADDS a refusal the input did not have.
  The is-empty revert was wrong — a deck that arrived with a deck-level bounds
  refusal (the test fixture's one-node presentation) reverted every repair
  forever;
- `run_deck` runs it: an overfull deck is repaired and rendered with the
  repairs named beside the verdict; only unrepairable residuals return to the
  agent, now carrying the exact budgets (`layout_holds()` = 13 lines ×
  `CHARS_PER_LINE` = 52) to compress against.

**KI-252 move 1 landed (`81d694b`, gate green 1134) — the layout template
index:** `LAYOUT_EXAMPLES`, one embedded description per layout written the way
an owner describes what they want (not the schema's words);
`deck::suggest_layout(s)_with(embedder, intent)` cosine-ranks the index over
the local embedder, `None` with no backend (the intent skill's own table
answers then — the match refines the classification, it does not replace it).
The `deck` tool carries a suggest mode: pass `suggest.intent`, get the ranked
layouts with scores. **Verified live through a real agent run: "a single number
that carries the whole point" ranked `metric_callout` at 0.844.**

**The live-run lesson (general): a tool capability the spec does not declare
does not exist for the model.** The first live run had suggest wired into
`run_deck` but absent from `deck_spec()`'s schema — the model improvised
around the undeclared argument, serde silently swallowed the unknown field,
and the model accurately reported "the deck tool has no suggest mode." The
fix was one schema property plus a description that names the mode. When a
new tool mode lands, the spec IS the rollout.

**Still open in KI-252 (moves 2-3):** the Matryoshka pre-filter (slice the
endpoint's vectors to 128d for the recall fan-out, full-width for the final
rank — measured against today's single-pass recall) and the cross-modal asset
library (icons/brand graphics keyed by embedded captions, the grader
certifying aspect and spacing before placement). These unlock the blueprint's
use cases 1, 2, 4, and 5 (brand-theme matching, icon sourcing, truncated RAG,
clustering-for-infographics).

**The action card landed (`4daaf0a`, gate green 1136, KI-254 closed) — the
blueprint's proactive-card engine, for the triggers the grader owns.** The
deck run split into `run_deck_with_event` → `DeckRun { outcome, event:
Option<DeckEventData> }`, where `DeckEventData` carries the project, the node
count, the fragment, the grader's faults as JSON values (`deck::fault_value`:
trigger/node/the measured numbers), the deck **as written**, and the
auto-fit repair's own deck — the fix pre-baked before the card is ever
shown. `run_deck` wraps it for the model-only path. `UiEvent::Deck` carries
the same fields through the bridge into the page, except the faults ride as
a JSON-array STRING, because the event derives Eq and `serde_json::Value`
breaks Eq. The pane renders the action card when faults exist: each fault
named with its numbers, and two one-click renders — **keep the repaired
render** or **revert to as written** — applied client-side through
`POST /api/deck/render` (a checked deck renders with its review line; a
consultant-breaking deck is a 409 naming the rule, so a click cannot make
the server ship a deck that lies). The card re-derives from the answer's own
faults: an applied repair empties it, a revert repopulates it. What the
auto-fit already decided is still visible as named repairs in the tool's
answer. The embedder-detected triggers (a chronology in a bullet block,
data shapes, an empty frame) are the card sources the vector layer adds
next — recorded in the KI-254 row.

**Test-fixture lesson, hit three times in one feature:** a one-node
presentation violates the deck-level minimum ("at least 2 nodes"), so any
assertion shaped `check(&fitted).is_empty()` fails on a pre-existing
deck-level refusal that the repair correctly preserved — assert the
consultant verdict is UNCHANGED instead, use infographic fixtures for
single-frame decks (exactly 1 node is their type's own bound), and when a
deck run needs to verify layout repair, count the faults not the
consultant refusals.

**Why:** the owner's blueprint list is the working backlog for fleet's
deck/visual pipeline, and the triad's members are now the ones the owner
named — Liquid's LFM2.5-24B-A2B and sys1 behind blend1's gate, the local
EmbeddingGemma 2 for vectors.

**How to apply:** deck repairs are deterministic and verified by three
auto-fit tests (compress-keeps-takeaway, roomier-upgrade, clean-deck-untouched)
plus the tool-level `the_deck_tool_auto_fits_an_overfull_deck_and_renders_it`
and `the_deck_run_carries_the_faults_and_the_prebaked_fix`; a new tool mode
ships with its spec property in the same commit. Related:
[[fleet-deck-visual-pipeline]], [[blend1-is-the-latency-triad]],
[[fleet-vault-credentials]], [[fleet-web-cache-tinyfish]].
