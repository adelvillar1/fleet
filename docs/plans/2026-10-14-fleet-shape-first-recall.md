# Slice 19 — shape-first recall: the shape drives, the words corroborate (2026-10-14)

status: planned

## Goal

The owner's directive, stated twice and sharpened the second time:
**problem shape must drive semantic search.** "A keyword or phrase in
the wrong context is meaningless, while a problem shape tied to the
word phrases that were recorded during the resolution are key
information." Fleet's recall today has it backwards: 0.80 of the
scoring weight is word-based (vector 0.55 + keyword 0.25) and the shape
is a 0.15 additive boost — a tiebreaker under the words. This slice
inverts it: the shape partitions the pool, the words rank within it,
and the write path records the phrases that resolved the problem so a
later recall for the shape finds them.

## The formal framing (the owner's, 2026-10-14)

A problem is a **minima in a multidimensional space**; a solution is a
**vector that drives toward that minima**; and a value that is a
minimum for one dimension can be meaningless in the multidimensional
context. Everything below follows from it: the query defines a
multidimensional *region* (the shape); a candidate fact is a *vector* —
its direction is the vocabulary recorded during the resolution, its
magnitude is the evidence it actually drove (the recall-hit history,
the confirmed-by provenance); and no single dimension may dominate the
ranking, because the per-dimension minimum is exactly the failure mode
(a homonym across shapes, a synonym within one). Fleet's recall today
has it backwards: 0.80 of the scoring weight is word-based (vector 0.55
+ keyword 0.25) and the shape is a 0.15 additive boost — a tiebreaker
under the words.

## The problem shape (PS-D, the recall half)

"**Have I seen this shape before, and what resolved it?**" — the naive
path is a word-similarity recall that fails in both directions: a
homonym across shapes surfaces the wrong fact confidently, and a
synonym within one shape misses the right one. Word-similarity is
noise; the shape is the invariant. The shelf has two halves — the
approaches that fit the shape (prevention) and the shapes that broke
(avoidance) — and recall must serve both.

## Acceptance criteria

- **M1 — the shape partitions the recall.** The instruction's
  classified shape selects the primary pool (the facts carrying it);
  every other fact is spillover. A same-shape fact with weak word
  evidence outranks a wrong-shape fact with strong word evidence —
  pinned by a test constructed to fail under the old weighting (the
  old score ranks the wrong-shape word-strong fact first; the new
  ordering must not). An unclassifiable instruction keeps today's
  behavior (the word-ranked pool is the primary pool) — the
  classification's failure must never starve recall.
- **M2 — the spillover is named and conditional.** Off-shape facts
  enter the assembly only when the primary pool is empty, or its best
  score is below a named floor (`SPILLOVER_FLOOR`), or the budget has
  room after the primary pool is in — and every off-shape fact is
  MARKED in the assembly (the line carries `off-shape`) so the model
  knows its context is weaker. Never silently mixed into the shape's
  answers.
- **M3 — the failure shapes are a first-class axis.** The shape
  vocabulary gains the failure shapes — what kind of thing broke (the
  avoidance half of the shelf) — beside the structure shapes. A fact
  tagged with a failure shape is recalled when the instruction matches
  that failure shape, through the same partition. The write path's
  parser accepts the failure-shape tags; the taxonomy is documented.
- **M4 — the write path records the resolution's phrases.** The
  `remember` tool's description instructs the model to record the
  phrases that resolved the problem — the vocabulary that worked —
  and they ride in the fact's indexed text beside the description and
  the shape tags, so a later recall for the shape matches the phrases
  that resolved it. The tool's description is the instruction's home
  (the slice-9 lesson: an instruction with no home where the model
  sees it does not exist). A round-trip test pins it: write with the
  resolution's phrases, recall by the shape, the phrases match.
- **M5 — the eval scenarios.** Deterministic scenarios for the
  inversion (M1's test at the corpus level), the spillover's
  conditions, the failure-shape recall, and the phrases' round-trip —
  scripted seams, no paid calls.
- **M6 — the surface and the docs.** `fleet memories --recall` shows
  the partition: the primary pool's facts, then the spillover marked
  off-shape, at the house bar (never a plain-text table).
  `docs/features/memory.md` carries the shape-first design, the failure
  axis, the phrases' recording, and the weights' new meaning (the
  word terms now rank WITHIN the shape, not across it).
- **M7 — the gate.** The full suite green, nothing deleted,
  warning-free; every error or warning encountered lands in the ledger
  with its evidence.

## Design decisions

1. **The shape is structural, not a boost.** The partition replaces the
   additive term; the existing weights (vector, keyword, importance,
   decay) become the within-shape ranking. A fact's multi-shape tags
   put it in the primary pool when any matches.
2. **The classification's failure degrades to today's behavior.** No
   shape → the word-ranked pool is the primary pool. The shape must
   never starve recall; it reorders it.
3. **The failure axis joins the structure axis, it does not replace
   it.** A fact can carry both (a rendering failure is a structure
   shape and a failure shape); the partition matches either.
4. **The phrases are the model's to record, the path's to carry.** The
   tool's description asks for them; the indexed text carries them; the
   scoring treats them as the within-shape word evidence they are.

## Out of scope

- Indexing the ledger's KI rows into the recall substrate as
  failure-shaped facts (the failure shapes' first source is the
  `remember` path; the ledger's indexing is a named follow-on — 171
  failure records are the avoidance half's bulk).
- The corpus's shape-aware retrieval (blend1's per-project corpus
  searches by cosine today; the shape partition there is the same
  pattern applied to a second store — named, not built).
- The judged rows' shape-keyed recall (slice 18 builds the rows; their
  recall through this partition rides the same path once they are
  facts).

## Assumptions (listed, not asked)

- The instruction's shape classification stays the fork's keyword
  taxonomy, extended with the failure shapes; an unclassifiable
  instruction is the documented degradation, not an error.
- The spillover's floor is a named constant sized against the existing
  MIN_RECALL_SCORE, stated with the reasoning.
- The recall-hit feedback (the recall-count re-ranking) keeps its
  meaning within the new ordering.
