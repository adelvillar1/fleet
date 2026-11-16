# The deck

A deck is a **structured visual deliverable** — a presentation, an
infographic or a poster — built as a value the agent passes to the `deck`
tool and a surface renders. The structure is what separates content from
design: the module decides whether the output looks considered or
generated, so every constraint is a **refusal**, not a suggestion in a
prompt.

## The contract

- The schema is plain serde (`src/deck.rs`): `project_type`, an optional
  `theme`, and its `nodes`. A node carries an id, a `layout`, an optional
  `eyebrow`, a `headline` that **states its conclusion**, and its
  `components` (bullets, metrics, steps, contrast, quote, chart, prose).
- The bounds differ by type: a presentation is paced (2–40 nodes), an
  infographic and a poster are one frame.
- `check` refuses: a headline that is a topic or a question (the Pyramid
  Principle), sub-points that overlap (MECE), a bullet with no `**bold**`,
  and any broken bound — each naming the node and the rule.
- `review` reports what will not fit; `auto_fit` repairs in place (text
  compressed to the measured budgets, a roomier layout where compression
  would cut content) and names every repair beside the render.

## Suggest mode

`deck` with `suggest: { intent }` ranks the layouts by semantic match
against the **local embedding model** (the loopback llama-server recall
also uses, `127.0.0.1:8901`), best first, and — when an asset library is
configured (`FLEET_ASSETS_DIR`) — names the assets that match, with the
grader certifying each asset's aspect against the project type. The
planner builds with the winner unless the content says otherwise.

## Semantic clustering (a long source into blocks by meaning)

`deck` with `source: { path | text, project_type }` builds the deck from a
**long document** instead of hand-written nodes:

1. **Split** — the source's paragraphs become candidate blocks; a
   paragraph under twelve words joins the one before it (a heading is not
   a block). Past 60 blocks the source is sampled evenly across the
   document, and the note names both counts.
2. **Embed** — each block through the same loopback embedder recall uses.
   An absent or busy backend is a **named refusal**, never a partial
   cluster set presented as complete.
3. **Cluster** — deterministic agglomerative grouping over cosine
   distance: merge the closest pair while the group count is over the
   type's ceiling, or while it is over its floor and the pair's mean
   cosine is at or above `CLUSTER_FLOOR` (0.75, chosen by measurement).
4. **Map** — a presentation gets one node per cluster (headline = the
   cluster's own first sentence, bounded; body = its members' leads). An
   infographic or a poster is one frame, so the clusters become its
   sections (a poster keeps its three-section ceiling). Every mapped deck
   goes through the same `check` / `auto_fit` / render path a hand-built
   one does.

**The measurement gate.** The clustering ships **opt-in**
(`deck.clustering`, default off) because its measured agreement has not
cleared the gate. The committed fixture (six labeled groups, thirty
blocks) scores **0.732 pairwise agreement** on the loopback embedder
(within-group mean cosine 0.790 against 0.717 between) where the gate is
**0.90**; the test `the_clustering_measurement` prints the score and pins
it as a regression bound. A domain-tuned embedder, a reranker, or a
two-stage centroid pass is what would move it — until then the setting
stays off and the tool's refusal names the knob.

## Where it is verified

- `deck::tests` — the split/sampling seams, the absent-embedder
  degradation, the labeled-group recovery, the per-type mapping, the
  setting's round-trip, and the `#[ignore]`d live measurement.
- `tools::tests` — the source mode's refusals (both/neither path and
  text, binary content, a source and a deck together, the setting off by
  name) and a clustered deck certified by the grader.
