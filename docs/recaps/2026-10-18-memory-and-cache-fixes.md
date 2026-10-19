# The memory's own cache was eating itself, and a graph needed its edges named

Status: complete
Date: 2026-10-18

Branch: `master`. One session, one plan, three instructions from the
owner: a deep dive on how fleet manages memories and cache, then a plan
to address what the dive found plus an evaluation of whether an n-gram
or a knowledge graph would improve recall, then the implementation of
that plan. Nothing is committed — the whole increment sits uncommitted
on top of `2340733`, which is the ledger recap's commit.

The arc: the dive found seven defects and one recall problem nobody had
named. The recall problem turned out to be the interesting one — the
history channel had saturated — and it is also what the n-gram and
knowledge-graph questions had to be answered against. Measuring first
changed the answer on both.

## The deep dive found a cache that invalidated itself

`memories/.recall-index.json` keys each cached vector on the fact file's
`secs:nanos:len` stamp. The recall-count bump rewrites the file in place
through a temp and a rename, so every bump moves the mtime, changes the
stamp, and re-embeds a fact whose content never changed. The corpus's
three most-recalled facts carry counts of 103, 103 and 98 — they had
been re-embedded on essentially every load, and the embedding round trip
is the load's whole cost on a machine with a live vector path. The
module's own doc says the intent is that a *changed* fact is re-embedded
and an untouched one keeps its vector; the implementation read metadata,
so a metadata-only change was treated as a content change.

The fix keys the entry on a digest of the text the vector was actually
computed from — `calib::hash16(&fact.indexed_text())`, the same hash the
route cache joins on. `recall-count` is outside `indexed_text`, so a bump
changes nothing.

## The measurement decided the two evaluation questions

The owner asked whether an n-gram channel or a knowledge graph would
improve recall. Both answers came from measuring the ranker over the
live 24-fact corpus rather than from reasoning about it.

**N-grams: no.** A light stemmer recovers 6 of 99 query tokens — a 6%
coverage gap. The real failures are that 71% of the corpus clears the
`MIN_RECALL_SCORE` floor for a typical instruction, the top-1/top-2
spread is 0.000–0.066, and two of the five wrong answers are *exact ties
broken by the alphabet*. Character overlap moves every fact together, so
it reorders nothing where the ordering is already wrong, and it costs a
seventh weight in a composite the owner has ruled on twice.

**Knowledge graph: yes, staged.** The substrate already declares three
edge types and the retriever walked none of them. `missing` names the
companions an incomplete trajectory needed and is deliberately excluded
from the indexed text — but nothing ever *fetched* the named fact, so the
corpus declared an edge and the retriever rendered it as prose. That one
is real and now works.

The third edge type did not survive contact with the implementation. A
fact stamped `superseded-by` is *retired* by `Fact::forgotten`, so it
never enters a pool and there is no forward source to expand from. What
the edge can do is vouch: a retired fact whose own vocabulary matches
points at the fact that replaced it, and a corpus whose only match is a
folded-away fact has an EMPTY primary pool — exactly when the
replacement must still surface. So a retired fact is a source and never
a hit, and the hop's early return had to go because it gated that path
on a non-empty pool.

## Three design corrections the implementation forced

- **`GRAPH_DISCOUNT` was removed.** It made admission arithmetically
  impossible — a source scoring 0.30 admitted nobody at
  `0.30 × 0.5 < 0.20`. The bar is now the source's own total against the
  floor every other fact faces.
- **The history curve's margin narrowed, and that is the honest
  consequence.** The canonical direction-over-proximity pair went from
  0.45 vs 0.545 (margin 0.095) to 0.45 vs 0.5028 (margin 0.053), because
  four recalls now read history 0.331 instead of 0.5. The direction
  fact no longer buys a quarter of its score from being recalled four
  times, which is the ruling applied more strictly, not less.
- **Two tests pinned behaviour the plan intentionally changed.** The
  two-signs test now expects the divergence to lead — its trajectory is
  1.0 against the resolution's 0.667 — and the outcome-tags test now
  supplies `missing` on its incomplete fact. Both carry the reason in
  place rather than being worked around.

## The gate

All four steps exit 0: `cargo fmt --all --check`,
`cargo clippy --all-targets -- -D warnings`, `cargo build --release`,
`cargo test` — **1210 passed, 0 failed, 0 ignored** across every target,
with 1079 of those in the lib (1065 before this increment). Fourteen new
tests, and the two measurement tests read this repository's own corpus
through `CARGO_MANIFEST_DIR`, so the numbers the plan rests on are the
gate's own rather than a script's.

Clippy caught two things worth recording: asserting on a constant
(`assert!(GRAPH_HOPS >= 1)`) is vacuous and was replaced with the
structural assertions that actually test something, and `graph_hits` had
grown to eight arguments — the query now rides in as one `Query` value.

## Commits

No commits yet — the increment is uncommitted on `master`.

---

## What was added

- `src/recall_graph.rs` — one bounded hop over the frontmatter's own
  edges (`superseded-by`, `missing`), with a shared `shape` tag
  deliberately excluded because the partition already reads it. Both
  bounds are named constants that zero the mechanism without removing
  it.
- `src/filelock.rs` — the per-file `flock` extracted from `memory.rs`'s
  `FactLock` and shared with the session archive's count store. Two
  `unsafe extern "C"` blocks declaring `flock` in two modules was two
  places for the same primitive to drift.
- `Memory::graph_hits` and the `Partition.graph` / `Assembly.graph`
  fields, with `via: <edge>` on `ScoredFact` and an "Adjacent memory"
  section in the rendered block. A graph-sourced fact never records a
  recall hit, because it did not rank.
- The stemmer (`stem` / `stems`) beside the tokenizer, applied to both
  sides of every lexical comparison so the channel stays symmetric.
- KI-228 through KI-235, eight rows in `docs/known-issues.md`.

## What was fixed

- KI-228 — the recall index re-embedded every bumped fact; now keyed on a
  digest of the indexed text.
- KI-229 — the route cache adopted the foreign verdicts it promised to
  discard; `Cache::rekey` now clears them.
- KI-230 — a comment claimed the cache answered before the key check
  while the early return ran first; the lookup moved above it.
- KI-231 — a failing embed rewrote the pruned index on every load
  forever; the rewrite now needs a new vector or a genuinely orphaned
  entry.
- KI-232 — the nomenclature cache's own `fresh()` had no production
  caller; the write guard now invalidates the root it just changed.
- KI-233 — the GUI's type-ahead paid the probe's five-second budget per
  request; it now takes the probed answer.
- KI-234 — the fact lock's `.lock` sibling was not gitignored.
- KI-235 — the count store's bump was a lost-update race across
  processes.

## What was changed

- The history channel is now logarithmic over `RECALL_CEILING` 128
  instead of linear over 8. Sixteen of 24 corpus facts sat at or above
  the old ceiling (median recall-count 56), so the channel contributed a
  constant 0.250 to two-thirds of the corpus and handed every
  never-recalled fact a 0.25 deficit. The curve keeps 0 at 0.0 and the
  ceiling at 1.0, so the weight and the ruling are untouched while the
  middle spreads out.
- `by_rank` takes the trajectory term as its second key. The name alone
  was deciding exact ties arbitrarily — `own-home-credentials-ruling`
  beat `router-enforcement-ruling` at an identical 0.578 only because
  `o` precedes `r`.
- The writer now refuses an `incomplete` fact with an empty `missing`.
  Fifteen corpus facts carried the field and all fifteen carried it
  empty, so the third sign rendered with nothing to finish the
  trajectory with and the graph had no edge to walk.

## Files changed

**Memory substrate**
- `src/memory.rs` — the digest-keyed index, the log history curve, the
  trajectory tie-break, the stemmer, `graph_hits`, the `via` marker, the
  "Adjacent memory" section, the R4b write validation, and the
  measurement tests
- `src/recall_graph.rs` — new; the edge extraction and the bounded hop
- `src/session_recall.rs` — the count store's bump runs under the shared
  lock
- `memories/fleet-memories-substrate-is-one-markdown-file-per-fact.md` —
  rewritten; it described the pre-slice-19 four-channel design and
  omitted `tier`, `phrases` and `missing`, and its `recall-count` of 103
  was the highest in the tree, so the stale description outranked the
  current one

**Routing**
- `src/route.rs` — the cache lookup moved above the key check, and
  `rekey` called before it
- `src/route_cache.rs` — `Cache::rekey`

**Guard and surfaces**
- `src/nomenclature.rs` — `invalidate`
- `src/agent.rs` — the write guard invalidates the root after a landed
  `WriteFile`
- `src/gui/bridge.rs` — `rank` takes the probed embedder, and a
  keyword-only answer returns zero scores rather than an empty list
- `src/lib.rs` — the two new modules

**Documentation**
- `docs/plans/2026-10-18-memory-and-cache-fixes.md` — the plan, status
  complete, with the R4c measurement recorded
- `docs/known-issues.md` — KI-228 through KI-235
- `docs/features/memory.md` — the weight table and canonical numbers,
  the protected tier's synonyms, the home-first corpus rule, the
  per-process probe, the bump's lock, and a new recall-graph section
- `TECHNICAL-DOCUMENTATION.md` — the route cache is a persistent file,
  not session-scoped
- `docs/architecture/deep-dive-2026-10-15.md` — the corpus resolution
  rule
- `CLAUDE.md` — the source map gains the two modules; the recall-count
  bump is no longer the corpus's only runtime edit
- `.gitignore` — `/memories/*.lock`

## Doc updates applied

- `docs/features/memory.md` — the history channel's row and its curve,
  the canonical 0.45/0.5028 pair and its 0.053 margin, the spillover
  arithmetic that reads off it, the protected tier's synonyms (KI-059),
  the home-first corpus rule (KI-087), the per-process probe (KI-064),
  the bump's per-fact flock (KI-062), both lexical channels' stemming,
  and a new "The recall graph" section
- `TECHNICAL-DOCUMENTATION.md` § the route cache — now names the
  persistent file beside the roster rather than "within a session"
- `docs/architecture/deep-dive-2026-10-15.md` — the corpus resolution is
  home-first, not cwd-walked
- `CLAUDE.md` — the source map and the runtime-edit line
- `docs/known-issues.md` — eight rows, all closed-fixed
- `memories/fleet-memories-substrate-is-one-markdown-file-per-fact.md` —
  rewritten in place rather than superseded, because it is the
  substrate's own self-description and superseding it would retire the
  fact that says what the substrate is

## Doc updates deferred (debt)

Four stale claims the deep dive found and this increment did not reach,
because none of them is about the memory or cache subsystems:

- `CLAUDE.md:21` and `:99` — two test-count claims that contradict each
  other (1013/1 against 1012/2) and are both wrong; the gate now reports
  1210/0. **Deferred because**: it is a one-line fix but the number moves
  again the moment anything lands, so it wants a convention rather than
  a patch. **Target**: the next increment, ideally as "see the gate"
  rather than a literal.
- `FUNCTIONAL-SPECIFICATIONS.md:19-20` — lists `fleet sessions
  --consolidate` and `fleet sessions --archive`, which do not exist;
  they belong to `fleet memories`. **Target**: the next CLI-surface
  increment.
- `README.md:70-72` — still describes `fleet sessions` as the plain
  oldest-first list; the archive plan made it cards. **Target**: same.
- `docs/features/memory.md:866` — "Fleet's own nine facts" against 24 on
  disk. **Deferred because**: it is prose inside a historical passage
  about the seed corpus, and rewriting it wants the passage re-read
  rather than a number swapped. **Target**: the next memory-doc pass.

## Open questions / next steps

- **The dominant recall failure is still open and named.** The recorded
  `phrases` carry the resolution's vocabulary rather than the task's
  nouns, so `own-home-credentials-ruling` scores trajectory 0.000
  against "where do credentials resolve from". No reweighting reaches
  it. R4b is half of the fix on the writer's side; the other half is
  teaching the writer to record the task's nouns beside the resolution's.
- **The recall graph is infrastructure waiting on its data.** It does
  nothing on the live corpus yet — 0 `superseded-by` edges and 15 empty
  `missing` fields at the dive. R4b makes the writer populate the
  companions and the cycle already stamps `superseded-by` when it folds,
  so the edges arrive as the corpus is used. The next measurement of the
  twelve-instruction set is what says whether the hop earns its keep.
- **`SPILLOVER_FLOOR`'s reasoning has a hole R1 made smaller but did not
  close.** The doc justifies 0.50 by "the most a fact with no direction
  evidence can score is 0.45" — which assumes history 0. A heavily
  recalled fact with no direction can reach 0.70 under the old curve and
  still clears 0.50 under the new one. The floor's arithmetic should be
  restated with the history term included, or the floor moved.
- **The whole increment is uncommitted.** Seventeen modified files, two
  new modules, one new plan. The owner has not reviewed it.

## Notes

- **Measuring before answering changed both evaluation answers.** The
  n-gram case is the clear one: the morphology gap is 6% of query tokens
  and the failure mode is saturation and ties, so an n-gram channel
  would have been a seventh weight solving nothing. The knowledge-graph
  case went the other way — it looked like a foreign addition until the
  measurement showed the substrate already declared two edges the
  retriever ignored.
- **The `superseded-by` correction is the kind of thing only an
  implementation finds.** The plan named it as one of three edge types
  to walk. Writing the test showed the source is retired by design, so
  the edge can only ever vouch. A plan that had not been implemented
  would have shipped a dead code path with a green test that only
  exercised the `missing` half.
- **The route cache's KI-229 is worth remembering as a shape.** The doc
  promised "discarded rather than served" and the code suppressed the
  read while leaving the entries in place — so the next write-through
  adopted them. A promise about a cache's *contents* needs a test that
  inspects the contents, not one that inspects the read.
- `tmp/` is untracked tool noise, not part of this increment.
