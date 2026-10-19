# Memory and cache: the fixes, and whether n-grams or a graph would help

Status: complete
Date: 2026-10-18
Landed: 2026-10-18

## Why

The 2026-10-18 deep dive of `src/memory.rs` (9,469 lines) and the five
caches found seven defects, and a measurement of the live 24-fact corpus
found a recall problem nobody had named: **the history channel has
saturated**. Sixty-seven percent of the corpus sits at or above
`RECALL_CEILING` (8) — the median recall-count is 56 — so the channel
contributes a constant 0.250 to two-thirds of the facts and
discriminates only among the remaining third. A never-recalled fact
carries a 0.25 structural handicap that the word channels, measured at
0.04–0.15 in practice, cannot overcome.

That finding drives the second half of this plan, and it is also what
the n-gram and knowledge-graph questions have to be answered against.

## The measurement the recall half rests on

Replaying the ranker over the live corpus (`memories/`, 24 facts,
keyword-only machine so the vector channel is 0) with twelve
instructions whose answer exists in the corpus:

- **The floor admits 71% of the corpus.** For a typical instruction
  16.9 of 24 facts clear `MIN_RECALL_SCORE` (0.20). `ASSEMBLY_TOKEN_BUDGET`
  is 2048 tokens, which the docs size at ~25–30 facts, so the budget
  admits almost everything the floor admits. The floor is doing almost
  no noise control.
- **Every winner is at the history ceiling.** In all twelve runs the
  top fact carries `recall-count >= 8`, so `hist = 0.250`. The standing
  term (history 0.25 + freshness ~0.10 + provenance 0.05 = 0.40) is
  71% of the winning score; the word channels contribute at most 0.18.
- **Correct fact ranked #1 in 7 of 12.** The five failures are not
  relevance failures of the corpus — the answer exists and its words
  match — but ordering failures. Two of them are **exact ties broken by
  the name**: `own-home-credentials-ruling` and `router-enforcement-ruling`
  score identically (kw 0.060, traj 0.120, hist 0.250, fresh 0.098,
  prov 0.050 = 0.578) and the alphabet picks the wrong one.
- **Morphology is not the problem, and neither is the standing term's
  size.** A light suffix stemmer recovers 6 of 99 query tokens — 6% more
  coverage. And a word-strong fact *does* beat a saturated wordless one:
  full keyword plus a 0.5 trajectory match scores 0.481 against a
  103-recall wordless fact's 0.400. The gap that matters is that the
  recorded `phrases` do not echo the instruction's nouns —
  `own-home-credentials-ruling` scores trajectory 0.000 against "where
  do credentials resolve from" because its phrases carry the
  resolution's vocabulary (`FLEET_HOME`, `.env`) and not the task's. In
  the twelve runs the word channels deliver 0.04–0.15, not the 0.60 they
  are capable of.

**So the honest split of the recall work is:** R1 and R2 are correctness
fixes — a saturated channel is a dead channel, and an alphabetical
tie-break is an arbitrary relevance signal — and together they move the
measured set from 7/12 to 8/12. The dominant cause of the failures is
the phrase-vocabulary gap, which no reweighting reaches and which R4b
addresses on the writer's side.

## The n-gram question: no

An n-gram channel would not fix any of the above, and the measurement
says why.

- **It targets the wrong failure.** The tokens that fail to match
  exactly are 6% of the query. The failures are saturation, ties, and
  phrase vocabulary — none of which character overlap addresses.
- **It would move every fact together.** An n-gram channel adds a
  roughly uniform bonus across the corpus, so it reorders nothing where
  the ordering is already wrong, and it raises the 71% that already
  clear the floor.
- **It costs a seventh weight** in a composite the owner has already
  ruled on twice (the direction-outranks-proximity reweighting, then
  its own correction at KI-195). Adding a channel to re-litigate a
  settled weighting is the wrong instrument.

If fuzzy matching is ever wanted it should be a **query-side typo
correction**, not a channel: correct the instruction's tokens against
the corpus vocabulary before scoring, and leave the composite alone.
That is a separate plan and it is not this one.

## The knowledge-graph question: yes, staged, and modest

The graph is not a foreign addition — the substrate already declares
three edge types and the retriever walks none of them:

- `superseded-by` (fact → fact): **0 populated edges** in the live
  corpus.
- `missing` (fact → named companions): **15 facts carry the field, all
  empty**. The third sign therefore never fires in practice.
- shared `shape` tag: 4 clusters (`change` 12, `docs` 6, `build` 2,
  `deps` 2) — the only edge type in use, and the partition already
  reads it.

`missing` is the sharpest case: it names the absent companions a
trajectory needed, it is deliberately excluded from `indexed_text`
(`src/memory.rs:393`) because an absent companion is not the fact's
direction, and **nothing ever fetches the named fact**. The corpus
declares an edge and the retriever renders it as prose.

So the graph pays off — but only once the edges are populated, which is
why R4 is staged behind writer-side work rather than shipped as
infrastructure ahead of its data.

---

# Part 1 — the seven defects

## M1 — the index stamp keys on metadata, so every bump re-embeds

**KI-228.** `file_stamp` is `secs:nanos:len` (`src/memory.rs:785`) and
`Index` reuses a vector only when `entry.stamp == fact.stamp`
(`src/memory.rs:1972`). `rewrite_field` writes a temp and renames over
the fact, so the mtime moves. **Every recall-count bump invalidates
that fact's cached vector and forces a re-embed on the next load.**

The corpus's three most-recalled facts carry counts of 103, 103 and 98.
They have been bumped roughly a hundred times each, and each bump
invalidated their vector. The design intent at `src/memory.rs:780-784`
is that "a changed fact is re-embedded, an untouched one keeps its
cached vector" — where *changed* means the fact's content. The
implementation reads metadata, so a metadata-only change is treated as
a content change. The test `the_index_caches_vectors_and_rebuilds_only_what_changed`
(`src/memory.rs:6955`) pins content-change and stamp-match but never
exercises the bump, so nothing guards it.

**Design.** Key the entry on a digest of the text the vector was
actually computed from, not on the file's metadata:
`IndexEntry { digest: String, vector: Vec<f32> }` where
`digest = calib::hash16(&fact.indexed_text())` — the same hash the
route cache already uses. `recall-count` is not in `indexed_text`, so a
bump changes nothing; a description, shape or phrase edit changes the
digest and re-embeds exactly that fact.

**Acceptance criteria.**
- **C1** `a_recall_count_bump_re_embeds_nothing`: load, bump a fact's
  count through `Memory::recall_hits`, reload — the embed call count is
  zero and the index file is byte-identical.
- **C2** `a_content_change_still_re_embeds_only_that_fact`: appending to
  one fact's body changes its description-bearing text and re-embeds
  exactly one.
- **C3** The stamp stays on `Fact` (the session archive reads it off its
  own logs and the two caches must agree on what "changed" means); only
  the index's key changes.
- **C4** A pre-digest index reads as empty and rebuilds — the same
  tolerance `a_legacy_index_without_the_key_is_rebuilt_not_served`
  already pins for the embedder key.

## M2 — the route cache adopts the verdicts it promised to discard

**KI-229.** `src/route_cache.rs:13-19` promises "A cache written by
another lane … is discarded rather than served," and `src/route.rs:1457`
repeats it. The implementation only suppresses *serving*: `judge` skips
the `get` when `keyed_for` is false, but `entries` is never cleared. On
the next fresh verdict, `put` + `flush(new_key)`
(`src/route.rs:1489-1497`) serializes the whole struct — **including the
other lane's verdicts — under the new key**, and they are served as
this lane's from then on.

The test `a_cache_keyed_to_another_lane_is_discarded_whole`
(`src/route_cache.rs:225`) asserts only `keyed_for` outcomes, never that
entries were dropped, so the named discipline is unenforced.

**Design.** `Cache::rekey(&mut self, key: &str)` clears `entries` when
the stored key is a different producer's, called once in `judge` after
the spec resolves and before the lookup. A run that judges nothing
flushes nothing (the `sys1_judges` early return precedes any write), so
a foreign cache is only destroyed once this lane has a verdict of its
own to put in its place — which is the documented promise.

**Acceptance criteria.**
- **C1** `a_foreign_key_clears_the_entries_it_refuses_to_serve`: load a
  cache keyed to lane-a carrying two verdicts, `rekey` to lane-b, assert
  `entries` is empty and a subsequent flush writes an empty map.
- **C2** `the_same_key_keeps_its_entries`: `rekey` to the stored key is
  a no-op.
- **C3** An end-to-end test through `judge` (or the closest seam): a
  lane-b verdict recorded after adopting a lane-a file does not serve
  lane-a's tasks.

## M3 — a comment contradicts the code it annotates

**KI-230.** `src/route.rs:1449-1452` reads "The cache above still
answers first: a recorded verdict is a judgment that was made, on
whatever machine made it." But `if !sys1_judges() { return … }` at
`src/route.rs:1453` executes **before** the cache lookup at
`src/route.rs:1460`. On a machine with no resolvable decision key (the
ten listed at `src/route.rs:1392`) cached verdicts are never served —
which also contradicts KI-044's own recorded rationale.

**Design.** Move the cache lookup above the `sys1_judges` check. The
order becomes: resolve spec → `rekey` → cache lookup → key check →
spawn. A cached verdict then answers on a keyless machine, which is
what a persistent cache under `FLEET_HOME` is for; a miss on a keyless
machine still returns early and pays no spawn.

**Acceptance criteria.**
- **C1** A test pinning the order: with `sys1_judges` false and a
  matching cache entry, `judge` returns the recorded verdict with
  `JudgedFrom::Cached` and spawns nothing.
- **C2** With `sys1_judges` false and no entry, `judge` returns
  `(None, JudgedFrom::None)` and spawns nothing — KI-044's behaviour is
  unchanged for the miss.

## M4 — a failed embed rewrites the index on every load

**KI-231.** In `load_with` (`src/memory.rs:1967-2007`) a fact whose
embed fails is `continue`d: no vector, no `fresh` entry. Then
`fresh.len() != index.facts.len()` triggers `Index::write` with the
pruned map. A fact whose stamp changed while the embedding server was
down is re-attempted, re-fails, prints one line, and re-writes — on
every load, forever.

**Design.** Rewrite only for two reasons: a new vector landed, or an
entry exists for a fact that no longer does. A failed embed costs a
retry next load and no write.

```rust
let orphans = index.facts.keys().any(|name| !fresh.contains_key(name)
    && facts.iter().any(|fact| &fact.name == name));
if changed || orphans { Index::write(root, key.as_ref(), &fresh); }
```

**Acceptance criteria.**
- **C1** `a_failed_embed_rewrites_nothing`: an embedder that fails for
  one fact leaves the index file byte-identical across two loads.
- **C2** A deleted fact's entry is still pruned.
- **C3** The eprintln per failing fact per load is unchanged — the
  naming is not the problem, the rewrite is.

## M5 — the write guard reads a vocabulary that cannot see its own writes

**KI-232.** `nomenclature::CACHE` (`src/nomenclature.rs:1398`) returns
the same derived `Vocabulary` for a root for the process's life.
`fresh()` (`src/nomenclature.rs:1432`) exists precisely to detect that a
file changed after derivation, and its only callers are its own test.
The write-time guard at `src/agent.rs:3659` reads the cache, so a term
introduced after derivation is reported as novel — or a drift as clean
— for the rest of the process. The cache is also unbounded by root, and
every read clones the whole `Vocabulary` under the mutex.

**Design.** `nomenclature::invalidate(root: &Path)` drops the root's
entry; the write guard calls it after a landed write. The guard is the
one caller that knows it just changed the tree, so it is the one caller
that can invalidate without a walk. `fresh()` stays as the advisory
check for readers that do not know.

**Acceptance criteria.**
- **C1** A test: derive a vocabulary, write a file carrying a new term,
  invalidate, re-read — the new term is present.
- **C2** Without the invalidate, the term is absent (the current
  behaviour, pinned so the fix is provable).
- **C3** The guard's own tests still pass unchanged.

## M6 — the GUI's type-ahead re-probes the embedder per request

**KI-233.** `bridge::rank` (`src/gui/bridge.rs:649-651`) calls
`memory::probe(&endpoint)` fresh on every call rather than
`memory::probed_embedder()`. On a machine with no embedding backend each
request pays the probe's 5-second connect budget; on this machine it
pays a real round trip per request. This is exactly the cost KI-064
removed from the corpus path, reintroduced in the browser path.

**Design.** One line: `crate::memory::probed_embedder()`. The
documented degradation is unchanged — a keyword-only backend's `embed`
returns `None`, so `rank` still answers with every score zero and the
type-ahead falls back to its lexical ranking.

**Acceptance criteria.**
- **C1** `rank` on a keyword-only machine makes zero HTTP calls.
- **C2** The ranked output is unchanged on a machine with a live
  backend.

## M7 — the fact lock's sibling is not gitignored

**KI-234.** `FactLock::take` creates `<fact>.md.lock` beside the fact
(`src/memory.rs:2371`); `Drop` releases the `flock` but never unlinks
the file — correctly, since unlinking a flocked file another process
holds is a race. `.gitignore` covers `/memories/*.tmp` and the recall
index but not `*.lock`; `git check-ignore -v memories/x.md.lock`
confirms it is not ignored. The loader filters on `.md` so recall is
unaffected, but the repo corpus shows untracked `memories/*.lock` after
any recall-count bump.

**Design.** Add `/memories/*.lock` to `.gitignore` with the reason. The
file is deliberately left on disk.

**Acceptance criteria.**
- **C1** `git check-ignore` ignores a `.lock` sibling.
- **C2** A recall-count bump leaves `git status` clean.

## M8 — the count store's bump is a lost-update race

**KI-235.** `RecallCounts::bump` (`src/session_recall.rs:682`) is a
non-atomic read-modify-write of a shared file across processes. The
temp-plus-rename discipline prevents torn files, not lost updates: two
concurrent recalls can drop an increment. Entries for deleted sessions
are never pruned, so a future log reusing an id inherits the count.

**Design.** One `.recall-counts.json.lock` beside the store, taken with
`flock(LOCK_EX)` for the read-modify-write. `FactLock` is private to
`memory.rs`; the smallest honest move is a small shared lock helper both
modules use, rather than a second copy of the `unsafe extern "C"` block.
Pruning stays out of scope — a count for a deleted session is evidence,
not garbage.

**Acceptance criteria.**
- **C1** Two concurrent bumps both land (a test that spawns two bumps
  against one store and reads 2).
- **C2** A malformed or absent store still reads as empty and never
  errors — the existing tolerance is unchanged.
- **C3** The lock file is beside the store in `~/.fleet/sessions`, never
  inside a session log.

---

# Part 2 — the recall fixes

## R1 — the history channel stops saturating

The channel is `min(count, RECALL_CEILING) / RECALL_CEILING`
(`src/memory.rs:1322`) with `RECALL_CEILING = 8` (`src/memory.rs:1237`),
ported from the fork's `recallWindow`. The corpus has outgrown it by
13×: 16 of 24 facts are at or above the ceiling and the median is 56.
A channel that clamps two-thirds of its inputs to 1.0 contributes a
constant and stops discriminating — and it hands every never-recalled
fact a 0.25 deficit.

**Design.** A logarithmic curve over a wider ceiling, so the channel
spans the observed range instead of falling off a cliff at 8:

```
history = ln(1 + min(count, RECALL_CEILING)) / ln(1 + RECALL_CEILING)
```

with `RECALL_CEILING` raised to 128. The endpoints are preserved (0
recalls → 0.0, the ceiling → 1.0), the weight and the ruling are
untouched, and the measured corpus spreads out:

| recall-count | today | log/128 |
|---|---|---|
| 0 | 0.000 | 0.000 |
| 1 | 0.125 | 0.145 |
| 8 | 1.000 | 0.452 |
| 27 | 1.000 | 0.686 |
| 56 | 1.000 | 0.832 |
| 103 | 1.000 | 0.955 |

**Acceptance criteria.**
- **C1** `RECALL_CEILING` is 128 and the curve is logarithmic; the two
  endpoints are pinned by a test.
- **C2** The channel discriminates across the observed range instead of
  clamping two-thirds of it: recall-counts 0, 1, 8, 27, 56 and 103 map
  to history terms 0.000, 0.145, 0.452, 0.686, 0.832 and 0.955, pinned
  by a test. The measured set goes from 7/12 correct at rank 1 to 8/12.
- **C3** A fact with full keyword coverage and a 0.5 trajectory match
  still beats a 103-recall wordless fact (0.486 against 0.389) — the
  direction-outranks-proximity ruling is preserved, not weakened.
- **C4** The calibration stream's meaning is unchanged: a `recall_hit`
  row's score is still the score the fact entered the turn at, and the
  curve is monotonic in count, so more recalls still means more history.
- **C5** The docs (`docs/features/memory.md:117-124`) and the module doc
  at `src/memory.rs:1235-1237` are updated in the same increment.

## R2 — the tie-break prefers direction over the alphabet

`by_rank` (`src/memory.rs:2324`) sorts by total descending, then name
ascending. Two of the five measured ranking failures were **exact ties
decided by the alphabet** — `own-home-credentials-ruling` beat
`router-enforcement-ruling` at an identical 0.578 only because `o`
precedes `r`. The name is a deterministic tie-break, which is right for
reproducibility and wrong as a relevance signal.

**Design.** Sort by total, then by the trajectory term, then by name.
Within a tie, prefer the fact that drove. The name stays the final
tie-break so the ordering is still fully deterministic.

**Acceptance criteria.**
- **C1** Two facts with equal totals and different trajectory terms rank
  the stronger trajectory first.
- **C2** Two facts equal in total and trajectory still rank by name, so
  the ordering stays deterministic and the existing tests hold.
- **C3** `by_rank`'s doc comment names why trajectory is the second key.

## R3 — the tokenizer stems

The docs already name the gap: "The trajectory channel measures literal
token overlap with no stemming"
(`docs/recaps/2026-10-15-the-recall-loop-closes.md:81-86`). The
measurement puts the cost at 6% of query tokens — `facts→fact`,
`fixed→fix`, `derived→deriv`, `resolved→resolv`, `served→serv`. Small,
but it is free and it is the documented gap.

**Design.** A light suffix stripper in `tokens()` (`src/memory.rs:1387`),
applied to both sides of every comparison so the channel stays
symmetric. No new dependency, no Porter stemmer — a ~30-line rule table
covering the English inflections the corpus actually uses (`-s`, `-es`,
`-ed`, `-ing`, `-ly`, `-ment`, `-ness`, `-ation`/`-ations`, `-ies`→`y`).
The vector channel is untouched; stemming is a lexical-channel concern.

**Acceptance criteria.**
- **C1** `tokens("the recalled facts resolved the deployment")` and
  `tokens("recall fact resolve deploy")` intersect.
- **C2** The stemmer never maps two distinct corpus terms together
  (`deploy` and `deployment` may merge; `deps` and `depend` may not).
- **C3** The skills module's tokenizer is *not* changed — the two
  surfaces share the tokenizer today and memory's stemming is memory's
  business, so the shared function gains a stemming variant rather than
  changing behaviour under skills.

## R4 — one bounded graph hop, staged

**R4a — the edges and the hop.** Extract an adjacency map at load:
`superseded-by` targets, `missing` names, and shape-cluster peers, each
resolved by fact name and dropped silently when the name does not
resolve. In `partition`, after ranking, take **one bounded hop** from
the top three primary hits: pull in facts reachable by one edge that
are not already in either pool, mark them `via: <edge>` the way
`off_shape` is marked, score them at a discount of the source hit's
total, and admit them only if that clears `MIN_RECALL_SCORE`. The
assembly line names the edge, so a graph-sourced fact is never silently
mixed into the shape's answers.

The discount and the hop count are named constants that can be zeroed,
so the whole mechanism is one constant away from off.

**R4b — populate the edges.** The graph is infrastructure without data:
0 `superseded-by` edges and 15 empty `missing` fields today. So:
- `Remember::parse` requires `missing` when the fact's sign would be
  `incomplete` — today the field is optional and unpopulated, so the
  third sign never fires.
- The consolidation cycle's `supersede` action already writes
  `superseded-by` (`src/memory.rs:4265`); it is the edge producer and
  needs no change.
- The writer's `phrases` should echo the task's nouns beside the
  resolution's vocabulary — the measured trajectory-0.000 failures are
  an authoring gap, not a ranking one.

**R4c — measure, then keep or drop.** Run the twelve-instruction set
against a fixture corpus with populated edges. The increment ships only
if the correct fact's rank improves and no instruction's top result
gets worse. If it does not, the constants go to zero and the module
stays as the documented seam.

**Acceptance criteria.**
- **C1** A fixture where the answer is reachable only by a
  `superseded-by` or `missing` edge ranks it; the same fixture with the
  hop disabled does not.
- **C2** A fixture where expansion would pull in noise does not admit
  the noise — the discount plus the floor holds.
- **C3** Every graph-sourced fact is marked in its assembled line.
- **C4** No unbounded expansion: one hop, three sources, and the hop
  count is a named constant.
- **C5** A `missing` name that does not resolve to a fact is dropped
  silently — the corpus is hand-editable and a dangling name is not an
  error.

---

# Part 3 — the docs

## D1 — the documentation follows the code

The deep dive found a set of stale claims. The ones that touch these two
subsystems:

- `docs/features/memory.md:708-710` and
  `docs/architecture/deep-dive-2026-10-15.md:130-131` still document the
  cwd-walk corpus rule that KI-087's home-first rule superseded.
- `TECHNICAL-DOCUMENTATION.md:112-114` scopes the route cache to
  "within a session" when it is a persistent file under `FLEET_HOME`.
- `docs/features/memory.md:642` still describes a per-run embedding
  probe that KI-064 made per-process.
- `docs/features/memory.md:346-350` describes the recall-count bump
  with no lock; KI-062 added one.
- `docs/features/memory.md:469-472` names only `user` and `feedback` as
  protected; KI-059 added `PROTECTED_SYNONYMS`.
- `memories/fleet-memories-substrate-is-one-markdown-file-per-fact.md`
  describes the pre-slice-19 four-channel design and omits `tier`,
  `phrases` and `missing`. Its `recall-count` is 103 — the highest in
  the tree — so under the documented weights the stale description of
  the substrate outranks the current one. This fact is rewritten, not
  superseded, because it is the substrate's own self-description.

**Acceptance criteria.**
- **C1** Every claim above is corrected in the same increment that
  changes the code it describes.
- **C2** `CLAUDE.md:80` ("the recall-count bump is its only runtime
  edit") is corrected — the lifecycle stamps are the second.

---

## R4c — what the measurement decided

Run over this repository's own 24-fact corpus on the keyword-only
machine, inside the gate (`the_measured_set_holds_the_floor_the_reweighting_bought`):

| | before | after |
|---|---|---|
| correct fact ranked #1 | 7 of 12 | **8 of 12** |
| correct fact in the top 3 | 10 of 12 | **10 of 12** |

**R1 and R2 ship.** They are worth 1 of 12, and that 1 is
`orchestrator-walks-the-hard-chain`, which `own-home-credentials-ruling`
had taken on an exact tie decided by the alphabet. Both are correctness
fixes — a saturated channel is a dead channel, and an alphabetical
tie-break is an arbitrary relevance signal — so they ship on that ground
rather than on the size of the win, and the plan's own estimate of
8 of 12 held.

**R4a and R4b ship.** The `missing` edge demonstrably reaches a
companion the lexical channels cannot: on the fixture, the companion
scores 0.1875 on its own — under the floor, in no pool — and the edge
surfaces it marked `via: missing`. The `superseded-by` edge needed a
correction the implementation forced: the substrate *retires* the fact
that carries the stamp (`Fact::forgotten` reads a non-empty value as
retired), so there is no forward source to expand from. What the edge
can do is vouch — a retired fact whose own vocabulary matches points at
the its replacement, and a corpus whose only match is a folded-away fact
has an EMPTY primary pool, which is exactly when the replacement must
still surface. So a retired fact is a source and never a hit, and the
hop's early return was removed because it gated that path on a non-empty
pool.

**The graph does nothing on the live corpus yet**, and that is expected:
0 `superseded-by` edges and 15 empty `missing` fields. R4b makes the
writer require `missing` on an `incomplete` fact, and the consolidation
cycle already stamps `superseded-by` when it folds, so the edges arrive
as the corpus is used. `GRAPH_HOPS` and `GRAPH_SOURCES` zero the whole
mechanism without removing it.

**The dominant cause of the remaining failures is unchanged and named:**
the recorded `phrases` carry the resolution's vocabulary rather than the
task's nouns, so `own-home-credentials-ruling` scores trajectory 0.000
against "where do credentials resolve from". No reweighting reaches
that; it is an authoring gap, and it is the next thing to fix.

## The gate

`cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings`,
`cargo build --release`, `cargo test` — all four exit 0, **1210 passed,
0 failed, 1 ignored** (the lib went from 1065 to 1079 tests; the two new
modules carry their own suites). The two measurement tests read this
repository's own corpus through `CARGO_MANIFEST_DIR`, so the numbers the
plan rests on are the gate's own rather than a script's.

## Ordering

M1 first: it is the one defect that makes every other load more
expensive, and R1's reweighting is meaningless to measure while the
most-recalled facts are re-embedded on every load. M2 and M3 are one
sitting in `route.rs`. M4–M8 are independent. R1 and R2 are correctness
fixes that should land together so the re-measurement is against the new
curve and the new tie-break at once — but they are worth 1 of 12 on the
measured set, so they are not where the recall win is. R3 is cheap and
independent. R4a can be built any time; R4b is the increment that
actually moves recall, and R4c gates whether the hop ships.

## Out of scope

- **An n-gram channel.** Declined above, with the measurement.
- **A general knowledge-graph store.** The edges are frontmatter fields
  the substrate already declares. A graph database, a second index, or
  a new crate would reverse two rulings to solve a problem the
  frontmatter already expresses.
- **Cross-machine cache sharing.** Both on-disk caches are local state
  beside the corpus they describe, the way the roster is.
- **TTL or LRU anywhere.** Every invalidation rule in this codebase is
  stamp-equality, keyed discard, or process-scoped. Adding a clock-based
  expiry would be the first, and it would expire evidence the
  calibration stream wants.
- **Raising `MIN_RECALL_SCORE` to fix the 71% admission rate.** Tempting
  and wrong: the floor's arithmetic is documented and correct under the
  weights, and the measured failures are ordering failures, not
  admission failures — the right fact is usually in the pool, just not
  first. Moving the floor would hide that.
