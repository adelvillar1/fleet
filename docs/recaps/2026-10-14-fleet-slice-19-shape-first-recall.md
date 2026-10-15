# Slice 19 — shape-first recall: the shape drives, the words corroborate (2026-10-14)

**Plan:** `docs/plans/2026-10-14-fleet-shape-first-recall.md`.
**Feature:** `docs/features/memory.md`. **Ledger:** KI-195..KI-200
(the slice's own rows; the parallel slice 18's are KI-176..194).

## What landed

The owner's directive, implemented as the recall layer's design law:
**problem shape drives semantic search**, and the direction a fact
points outranks how near it sits.

- **The partition** (`src/memory.rs`): `classify_shape` selects the
  primary pool and no term can cross it — the old `SHAPE_BOOST` is gone,
  verified against the pre-slice constants in git history. An
  unclassifiable instruction degrades to today's behavior: the
  word-ranked pool IS the primary pool.
- **Six channels within the shape, no dominant dimension**:
  W_VECTOR 0.15 + W_KEYWORD 0.15 (proximity 0.30) against
  W_TRAJECTORY 0.30 + W_HISTORY 0.25 + W_PROVENANCE 0.05 (direction
  0.60), W_FRESHNESS 0.10. **The first weighting failed the owner's
  ruling and the review round corrected it** — see below.
- **The named-conditional spillover**: off-shape facts enter only
  through three named routes (an empty pool, the best below
  `SPILLOVER_FLOOR` 0.50, budget room), every entered fact marked
  off-shape with the admitting route named.
- **The failure axis**, drawn from the actual ledger: six shapes —
  fail-open, drops-the-row, bound-exceeded, claim-outran-evidence, plus
  the two the review added (fixed-width-clip, five recurrences;
  foreign-root, four, one root cause) — with their KI sources asserted
  in a test.
- **The outcome axis and the third vector** (the review round): the
  tags are three axes — structure (what kind of problem), failure (what
  kind of thing broke), outcome (what became of the trajectory). The
  sign is derived from them, so no stored field can contradict the
  tags: any failure tag ⇒ `diverged:`, else an `incomplete` tag ⇒
  **`incomplete:`**, else `resolved:`. A `missing: [...]` frontmatter
  field names the absent companions and is deliberately excluded from
  the indexed text — an absent companion is not the fact's direction. An
  incomplete vector ranks **exactly** as a resolution: same pool, same
  weights, no penalty. The consolidation cycle keeps an incomplete
  durable fact rather than folding it, because a summary bullet would
  keep the sentence and lose the absent pieces.
- **The phrases' channel**: the resolution's recorded vocabulary, the
  trajectory channel's input, capped and sanitized, with the remember
  tool's description as the instruction's home (the slice-9 lesson).
- **The surface**: `fleet memories --recall` renders the partition as
  cards at the house bar — the shape header, the pool, the off-shape
  section with its admitting route, the three-sign legend, the caution
  colour on a divergence — bounded at 80 and 120 through the shared
  word-aware wrapper.
- **The four eval scenarios** in the memory family: shape-inversion,
  spillover-routes, failure-shape-recall, phrases-round-trip.

## What is verified

- The gate on the merged tree: `cargo build --all-targets` exit 0,
  **zero warnings**; `cargo test` exit 0 — **734 passed, 2 ignored**;
  `memories/` clean.
- **The owner's ruling, pinned where it can actually fail**
  (`direction_outranks_proximity_with_the_vector_channel_live_for_both`,
  src/memory.rs:5327): the scripted embedder serves both facts *and*
  the query (three calls), so the vector channel is live for both — the
  slice's own proof ran KeywordOnly, where the channel was dead for
  both and the case never ran. A fact with **maximum proximity** (every
  word matched, vector 1.0, fresh, confirmed-by) and **zero direction**
  scores **0.45**; a fact whose description shares *no token with the
  question* (the question says "wobbly", the resolution says "shim the
  walnut table legs until it is level") with the resolution's own
  phrases and four recalls scores **0.665**. Maximum proximity loses to
  direction by 0.215, with the arithmetic hand-checked in the test.
- The third vector end to end: write "the bootstrap was correct but the
  torch stack was missing" with its companions, recall by the shape, and
  the block names the torch stack and does NOT read as done
  (`the_incomplete_vector_round_trips_and_the_block_does_not_read_as_done`).
- The failure-pool's sign fix: one failure shape, one pool, three signs
  in rank order — a resolution recorded under a failure shape is a
  candidate, not an anti-vector.
- The reviews: spec REQUEST-CHANGES (M5's scenarios absent — now landed;
  the ledger and housekeeping, now done); quality REQUEST-CHANGES with
  two criticals, both fixed in `e3ea0e2`.

## What the reviews caught

**Critical 1 — the weighting failed the owner's ruling.** With vector
0.20 + keyword 0.20 (0.40) against trajectory 0.25 + history 0.20 +
provenance 0.05, a fact with maximum proximity and zero direction
scored 0.55 while a direction-strong fact scored 0.49 — the proximity
fact won, exactly the case the owner named. The slice's own proof test
could not see it because it ran KeywordOnly, where the vector channel
was dead for both facts. Fixed by rebalancing to proximity 0.30 against
direction 0.60 (0.45 against 0.545, margin 0.095), re-deriving
`SPILLOVER_FLOOR` from 0.45 to 0.50 so a zero-direction fact can no
longer stand alone as the shape's own answer, and pinning the invariant
with the vector channel live. The floor's own sizing arithmetic was
wrong the same way — "full topical coverage is 0.35" is the keyword
channel alone; both channels are 0.45.

**Critical 2 — the third vector had no home.** The two-sign taxonomy
recorded an incomplete solution as `resolved:` — the one most mistaken
for done: the next session adopts a stalled vector as done, its phrases
score as a full direction, and the missing companions are unrecordable.
Tagged with a failure shape instead, it flipped to `diverged:` and
warned off a correct direction. Both wrong, in opposite directions.
Fixed by the outcome axis, the `missing` field, the no-penalty ranking,
the tool descriptions' vocabulary, and the cycle's decision.

Behind them: the failure-pool's sign problem (a fix recorded under a
failure shape rendered `diverged:`, warning off a correct trajectory);
two of the ledger's most recurrent failure classes had no shape at all;
the slice's own new surface carried the fixed-width clipping class the
ledger has logged five times; and the doc overclaimed three numbers.

## Not done

- The CLI's `remember` still has no `--phrases` flag (KI-175) — a
  CLI-written fact carries no direction; `missing` is tool-only for the
  same reason.
- The phrases' gaming exposure (a cooperating writer can parrot the
  question and score a full trajectory match) — named in the doc's
  limitations, not defended against.
- `src/memory.rs` is 7,878 lines, roughly twice the next-largest
  module — a split candidate (recall/scoring vs write/cycle), not a
  defect.
- The corpus's shape-aware retrieval (blend1's per-project corpus still
  searches by cosine) and the ledger's KI rows indexed as
  failure-shaped facts — both named in the plan's out-of-scope.
