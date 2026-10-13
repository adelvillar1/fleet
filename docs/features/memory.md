# Memory (slice 8, part 1)

Fleet's semantic memory: the git-tracked `memories/` fact substrate,
hybrid retrieval with the vector path probed rather than assumed,
Wave-D shape-routed recall, recall-hit feedback, and the protected tier
with budget-aware turn-context assembly.

Part 1 lands the substrate, retrieval, routing, feedback, and the
protected tier. Part 2 (a separate round) owns the write path
(`brain_remember`-shaped), the consolidation cycle, provenance as the
permission model, and the turn-path wiring — the seams are named below.

```
src/memory.rs      the substrate, the frontmatter subset, the embedder
                   trait and its two backends, the index, the ranker,
                   shape routing, feedback, assembly, the surface
src/calib.rs       the `recall_hit` op in the stream's vocabulary
src/doctor.rs      the recall-path check: which backend is live
src/cli.rs         `fleet memories` and its two flags
src/main.rs        the command wiring, and doctor's recall line
memories/          the fact tree, git-tracked; `.recall-index.json`
                   beside it is generated and gitignored
```

## The substrate

One markdown file per durable fact under `memories/`, at the repo root:

```
memories/
  fleet-the-gate-is-local-and-its-exit-code-is-read.md
  owner-github-handle-and-daily-driver.md
  .recall-index.json     generated; gitignored, like __pycache__
```

The frontmatter carries `name`, `description`, `type`, `date`, `basis`,
`shape`, `recall-count`, `superseded-by`, and `valid-until`, with the
file's body as the fact's full content — the fork's `memories/` tree
(234 facts in exactly this shape) is the spec. The description is the
one-sentence fact: what the index embeds and what an assembled turn
carries; the body is the full content on disk, read on demand the way a
skill's body is.

Facts persist across sessions because they are files. A bad one reverts
cleanly because they are git-tracked. No code path in this slice
rewrites history; the one in-place rewrite is the recall-count bump,
and it is bounded to that single frontmatter line.

The frontmatter parse is a **sibling** of the skills parser, not a
reuse — the subsets differ in three load-bearing ways: memory accepts a
bare optional field (`superseded-by:` with no value, the fork's writer's
own "unset" rendering), an indented continuation line (the fork's
writer folds a long plain scalar onto one), and a bracketed
`shape: [a, b]` list, where the skills parser rejects empty values and
every indented line. A malformed fact is skipped by name with one
logged line; the others still load. `basis` is `confirmed-by` or
`inferred` (absent reads as `inferred` — a fact the session merely
produced is a guess until someone confirms it); a value outside the
vocabulary is a malformed fact. `recall-count` must be a non-negative
integer when present. The frontmatter's `name` must equal the file
stem: one fact, one name, because the name is the index key and the
calibration row's key.

Two markers retire a fact: a non-empty `superseded-by` folds it into
another, and a `valid-until` before today expires it. A retired fact
loads (it is still a file, and the surface counts it) but stays out of
recall — the fork's `isForgotten`, ported, because a vector store does
not decay.

## Retrieval: hybrid ranking, the vector path probed

A fact's score is a weighted sum of four named channels:

| Channel | Weight | Meaning |
|---|---|---|
| `W_VECTOR` | 0.55 | cosine similarity to the instruction |
| `W_KEYWORD` | 0.25 | the instruction's tokens the fact's indexed text covers |
| `W_IMPORTANCE` | 0.10 | `recall-count` over `RECALL_CEILING` (8), earned by use |
| `W_DECAY` | 0.10 | exponential freshness, half-life `DECAY_HALFLIFE_DAYS` (90) |

Shape routing rides beside them as `SHAPE_BOOST` (0.15), and
`MIN_RECALL_SCORE` (0.20) is the floor below which a fact is noise for
this instruction. All five are named constants in `src/memory.rs`,
documented and tunable. `basis` deliberately does **not** enter the
score: the plan's rule is that importance is earned by use, not asserted
at write, and basis is provenance — the permission model part 2's
guardrail reads, not a rank.

**The vector backend is pluggable behind the `Embedder` trait with two
implementations.** The real one is a loopback HTTP call to the
llama-server embeddings route — the same `127.0.0.1:8901` sem1's default
provider serves — with the crate's existing ureq client: no new crate,
no key, no internet. On this machine the probe finds
`embeddinggemma-2` live and serving 768-dim vectors. The other
implementation is the **keyword-only degradation**, a peer rather than a
stub: three of the four channels still rank, and recall still answers.

The path is **probed, not assumed**. The probe is one real embedding
round trip — a `/health` poll would prove nothing about the embeddings
route — and anything that does not answer a usable vector (connection
refused, non-200, unreadable body, empty vector) is the keyword-only
degradation. When the degradation is live, cached vectors are ignored
as well as new ones, so what `doctor` names can never contradict what
recall used.

**The index is in-process.** Embeddings are computed at load and cached
in `memories/.recall-index.json` — generated, gitignored, like
`__pycache__` — keyed by fact name with the file's `secs:nanos:len`
stamp. A fact whose stamp matches keeps its cached vector; a changed
fact is re-embedded; the cache is rewritten only when something changed,
through a unique temp file and a rename, best-effort and silent (a
failed write costs the next load a recompute and nothing else). The
every-turn path never depends on the sem1 child.

## Routing: shape-routed recall

The incoming instruction classifies into one structure shape from the
fork's Wave D keyword taxonomy — `rendering`, `state-sync`, `deploy`,
`build`, `change`, `deps`, `docs`, ported verbatim from
`memories/shapes.json` — by substring match on the lowercased text,
first shape in the taxonomy's order winning (the same order the memory
importer tags in). Keyword-backed, not model-backed, because the model
path proved unreliable. A fact carrying the classified shape takes
`SHAPE_BOOST`.

An unclassifiable instruction answers from the surface alone: no shape,
no boost, the hybrid score alone — the documented fallback, named on the
surface ("shape: none — surface recall (the documented fallback)"), not
an error.

## Feedback: the measure rung

`Memory::recall_hits` is the feedback entry point. Every hit updates
the fact's `recall-count` in its file — a bounded, frontmatter-
preserving rewrite: the one line it owns is re-rendered, every other
byte including the body is preserved, and the write lands through a
unique temp file and a rename — then re-ranks on the next recall (the
importance channel rose), and appends one `recall_hit` calibration row
naming the fact, the shape, and the score. The row is
`{"op":"recall_hit","fact":…,"shape":…,"score":…}` in devdec's stream,
so `fleet calib` cards it and the loop's effect is auditable. A file
that cannot be rewritten logs one line and the hit is still recorded —
the audit trail must not depend on the corpus being writable.

Never-recalled facts age toward consolidation: part 2's cycle reads
`recall-count` as its signal.

## The protected tier and assembled context

Facts whose `type` marks identity or standing preferences — `user` and
`feedback` in the fork's taxonomy — are the **protected tier**. The
injection budget can never evict them. This is the deviation from both
current designs, and the fix for the live system's mis-tiered cap, where
the most load-bearing facts contended with task facts in the smallest
space.

Turn context is **assembled**, not dumped: protected tier first, then
shape-routed recall, then recent working memory, within
`ASSEMBLY_TOKEN_BUDGET` (512 tokens, estimated at one token per four
characters — the fork's own estimate and budget). The protected tier is
added first and is never dropped, even when it alone exceeds the budget
— that is the tier's whole point. The strongest recalled fact always
enters (the fork's `renderBlock` rule: a budget too small for it still
carries the strongest match rather than an empty block); the rest enter
while they fit, and the ones that do not are named in the surface's
"dropped under budget" line. Protected facts are excluded from the
recall ranking — no fact enters a turn twice.

The observed failure this avoids: a turn about one topic injected with
unrelated facts. The floor, the ranking, and the budget together are
the mechanism; the floor alone is the coarse guard, the budget the fine
one.

## The surface

```
fleet memories                              cards: name, type, shape, basis,
                                            recall-count, valid-until, path;
                                            the protected/retired tallies; the
                                            backend the probe found
fleet memories --recall "<instruction>"     the ranked assembly: every fact
                                            above the floor with its score and
                                            the channels that produced it, then
                                            the block that would enter a turn's
                                            context, then what the budget dropped
fleet memories --shape <name>               the cards filtered by structure
                                            shape; an empty filter names the
                                            taxonomy
```

`--recall` is a **dry run**: printing changes no recall-count and
records no row. The turn path that records hits is part 2's. The two
flags do not combine — one prints an assembly, the other filters cards.

The corpus is the nearest `memories/` at or above the working directory
(the fork's `resolveMemoryRoot` rule); in everyday use that is the repo
root. The exit code is always success — this is a read surface over
Fleet's own files, like `fleet calib` and `fleet skills`.

## `fleet doctor`'s recall line

```
recall       ok    vector path live — http://127.0.0.1:8901/v1/embeddings (768-dim embeddings serve recall)
```

or, when no local model answers:

```
recall       ok    embeddings unavailable — recall is keyword-only
```

The check passes in both states — keyword-only recall is a working
configuration, not a failure, and a clean machine has no local
embedding model, so doctor (the install gate) stays green — and its
detail names the path either way. Never silent: that is the live
system's failure this line exists to fix, where an absent embedding
stack silently degraded recall over 11k entries.

## Part 2's seams, named

- **The write path.** `brain_remember`-shaped: append one fact file
  through the same frontmatter shape, `valid-until` required for
  session-scoped facts and defaulted for working memory, `basis`
  recorded on every durable write, three speeds (working, durable,
  episodic). The substrate and the index already accept a new file; the
  writer renders the fork's field order.
- **Consolidation.** The scheduled cycle: compress old working memory
  into episodic summaries, supersede stale facts, forget expired ones,
  log every action as a calibration row. `recall-count` is its signal,
  and `superseded-by`/`valid-until` are its markers.
- **Provenance as the permission model.** `basis` is the guardrail's
  input: an `inferred` fact is supersedable on the loop's own
  justification, a `confirmed-by` one requires approval.
- **The turn path.** `Memory::recall_hits` and `Memory::assemble` are
  the entry points the agent loop calls per instruction; part 1 has no
  caller for them, which is why `--recall` is a dry run.
- **The working tier.** `assemble` takes the working-memory slice as an
  input; part 1's callers pass none, and the tier's place in the order
  and the budget is already reserved.

## What part 1 does not do

No write path, no consolidation cycle, no turn-path wiring, no new TUI
surface (the assembly enters the agent's context in part 2), and no
import of the fork's 234-fact corpus — the seed corpus is Fleet's own
nine facts, each one true and cited.
