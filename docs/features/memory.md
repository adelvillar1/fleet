# Memory (slice 8, slice 19)

Fleet's semantic memory: the git-tracked `memories/` fact substrate,
hybrid retrieval with the vector path probed rather than assumed,
shape-first recall, the failure axis with its three signs, the
resolution's phrases, recall-hit feedback, the protected tier with
budget-aware turn-context assembly, the `brain_remember`-shaped write
path, the scheduled consolidation cycle, provenance as the permission
model, and the turn-path wiring that carries the assembled block into
every run.

Part 1 landed the substrate, retrieval, routing, feedback, and the
protected tier; part 2 landed the write path, the consolidation cycle,
provenance, and the turn wiring. Slice 19 inverted the ranking: the
shape partitions the recall and the words corroborate within it. All
of it is in `src/memory.rs`.

```
src/memory.rs      the substrate, the frontmatter subset, the embedder
                   trait and its two backends, the index, the ranker,
                   shape routing, feedback, assembly, the write path,
                   the consolidation cycle, the surfaces
src/calib.rs       the `recall_hit` and `consolidate` ops in the
                   stream's vocabulary
src/agent.rs       the turn-path wiring: the assembled block, the
                   `memory` record, the `remember`/`supersede` tools
src/tools.rs       the two tool declarations, beside `use_skill`
src/session.rs     the `memory` record
src/cli.rs         `fleet memories` and its four flags, `fleet remember`
src/main.rs        the command wiring, the per-run corpus load, the
                   scheduled cycle in chat
src/tui/           the status bar's memory line
memories/          the fact tree, git-tracked; `.recall-index.json`
                   beside it is generated and gitignored, and
                   `.archive/` holds what the cycle forgot
```

## The substrate

One markdown file per fact under `memories/`, at the repo root:

```
memories/
  fleet-the-gate-is-local-and-its-exit-code-is-read.md
  owner-github-handle-and-daily-driver.md
  .recall-index.json     generated; gitignored, like __pycache__
  .archive/              what the consolidation cycle forgot
```

The frontmatter carries `name`, `description`, `type`, `date`, `basis`,
`tier`, `shape`, `phrases`, `missing`, `recall-count`,
`superseded-by`, and `valid-until`, with the file's body as the fact's
full content — the fork's `memories/` tree (234 facts in exactly this
shape) is the spec, with `phrases` and `missing` Fleet's own slice-19
additions beside their sibling `shape`. The description is the
one-sentence fact: what the index embeds and what an assembled turn
carries; the body is the full content on disk, read on demand the way a
skill's body is.

Facts persist across sessions because they are files. A bad one reverts
cleanly because they are git-tracked. No code path in this slice
rewrites history; the in-place rewrites are the recall-count bump and
the lifecycle stamps, each bounded to its own frontmatter line.

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
vocabulary is a malformed fact. `tier` is `working`, `durable`, or
`episodic` (absent reads as `durable`, so the corpus's own pre-tier
facts load unchanged). `recall-count` must be a non-negative integer
when present. The frontmatter's `name` must equal the file stem: one
fact, one name, because the name is the index key and the calibration
row's key.

Two markers retire a fact: a non-empty `superseded-by` folds it into
another, and a `valid-until` before today expires it. A retired fact
loads (it is still a file, and the surface counts it) but stays out of
recall — the fork's `isForgotten`, ported, because a vector store does
not decay.

## Retrieval: the shape partitions, the words corroborate

The owner's directive, stated twice and sharpened both times:
**problem shape must drive semantic search.** "A keyword or phrase in
the wrong context is meaningless, while a problem shape tied to the
word phrases that were recorded during the resolution are key
information." A problem is a minima in a multidimensional space; a
fact is a vector whose direction is the vocabulary recorded during the
resolution and whose magnitude is the evidence it actually drove; and
**a failure is a vector in the opposite direction** — an anti-vector
whose direction is the vocabulary of the trajectory that diverged. A
value that is a minimum for one dimension can be meaningless in the
multidimensional context, so no single channel may dominate the score.

Recall therefore runs in two movements:

1. **The shape partitions.** The instruction classifies into a shape —
   a structure shape from the fork's Wave D keyword taxonomy, or a
   failure shape from Fleet's own failure record — and that shape
   selects the **primary pool**: the facts carrying it. Every other
   fact above the floor is **spillover**. A fact with several tags is
   in the pool when any of them matches, so a fact carrying both a
   structure shape and a failure shape answers either axis. The shape
   is structural: it carries no additive term of its own, and no term
   in the score can move a fact across the partition.
2. **The words rank within it.** Inside the primary pool the facts
   rank by their evidence as a multidimensional vector, as a weighted
   sum of named channels:

| Channel | Weight | Meaning |
|---|---|---|
| `W_VECTOR` | 0.15 | cosine to the instruction (the embedded direction) |
| `W_KEYWORD` | 0.15 | the instruction's stems the fact's topic text covers |
| `W_TRAJECTORY` | 0.30 | the instruction's stems the recorded **phrases** cover — the fact's direction |
| `W_HISTORY` | 0.25 | `recall-count` on a logarithmic curve over `RECALL_CEILING` (128), earned by use |
| `W_FRESHNESS` | 0.10 | exponential freshness, half-life `DECAY_HALFLIFE_DAYS` (90) |
| `W_PROVENANCE` | 0.05 | the basis: confirmed-by 1.0, inferred 0.5 |

**Direction outranks proximity.** The semantic channels (vector,
keyword) measure how NEAR a fact sits to the query; the direction
channels (trajectory, history, provenance) measure which way its vector
points. The owner's ruling governs the weighting, and it is verified by
arithmetic rather than intuition: the proximity pair is 0.30 of the
composite and the direction channels are 0.60, so the margin is
structural rather than a coincidence of one corpus.

The maximum a fact with NO direction evidence can score is full topical
coverage plus standing — vector 0.15 + keyword 0.15 + freshness 0.10 +
provenance 0.05 = **0.45**. A direction-strong fact (a 0.8 trajectory
match, four recalls' history, confirmed-by, and a weak 0.2 cosine)
scores 0.15 × 0.2 + 0.30 × 0.8 + 0.25 × 0.331 + 0.10 + 0.05 = **0.5028**
and wins by **0.053**. The margin was 0.095 under the linear curve, where
four recalls already scored history 0.5; the log curve reads the same four
recalls as 0.331, and the narrower margin is the honest consequence — history
is magnitude, not direction, so a fact no longer buys a quarter of its score
from being recalled four times. The first slice-19 reweighting (0.20 each, 0.40
together) did NOT hold that line: it left a maximum-proximity,
zero-direction fact at 0.55 against the direction-strong fact's 0.49,
so proximity won the moment the vector channel served a fact whose
words matched perfectly. The topical pair is 0.15 each now, the
trajectory channel is the largest single one at 0.30 (the phrases
recorded during the resolution are the owner's key information), and
the magnitude — history 0.25 plus provenance 0.05 — is the smaller half
of the direction group by design: a confirmed-by vector that recall has
surfaced eight times has driven somewhere, an inferred one is a proposal
at half strength, not erased. Freshness is the smallest channel on
purpose — a fresh fact is not a better vector. All six are named
constants in `src/memory.rs`, documented and tunable.

The topical channels score the fact's **topic text** (the description
plus the shape tags); the trajectory channel scores the fact's
**phrases** (the resolution's own vocabulary). The two dimensions
cannot double-count each other. The vector channel embeds the fact's
**indexed text** — the topic text with the phrases riding along — so
the embedding carries the direction too. The **missing** companions are
deliberately excluded from the indexed text: an absent companion is not
the fact's direction, and embedding it would score a fact for what it
does not have.

`MIN_RECALL_SCORE` (0.20) is the floor below which a fact is noise for
this instruction, and its meaning is exact under the named weights.
Standing alone — freshness 0.10 plus provenance 0.05, no match at all —
is 0.15, under the floor: a fact never enters merely for standing. A
quarter-strength match in a DIRECTION channel beside standing clears it
(trajectory 0.225, history 0.2125), while the topical pair needs a
third of its weight to reach it (0.15 × 1/3 + 0.15 = 0.20, exactly at
the floor). The ruling reads in miniature: a fact enters when it points
somewhere, not merely when it sits near. A heavily-recalled fact that
matches nothing still clears it, ranks last, and is the budget's first
drop, exactly as before.

The floor is **not** a guard against a near-miss, and the doc used to
overclaim here. An unrelated fact's best case on this machine's normal
configuration — the vector path live, the same domain — is 0.15 × 0.6049
(KI-058's measured unrelated cosine) + 0.10 + 0.05 = **0.24**, which is
OVER the floor; on a keyword-only machine the best case is 0.10 + 0.05
+ 0.15 × 0.2 (one shared token in five) = 0.18, under it. What keeps a
near-miss from being read as the shape's answer is the shape partition
and the spillover's routes, not the floor.

An **unclassifiable instruction** is the documented degradation: no
shape, no partition, and the word-ranked pool IS the primary pool. The
classification's failure must never starve recall; it only fails to
reorder it. The surface names it — "shape: none — surface recall (the
documented fallback)".

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
recall used. The endpoint is the loopback default unless
`FLEET_EMBEDDING_ENDPOINT` names another — a second model on another
port, or a llama-server bound elsewhere; resolution is env → default
constant, the house style, and an empty value counts as unset.

**The index is in-process.** Embeddings are computed at load and cached
in `memories/.recall-index.json` — generated, gitignored, like
`__pycache__` — keyed by fact name with the file's `secs:nanos:len`
stamp. A fact whose stamp matches keeps its cached vector; a changed
fact is re-embedded; the cache is rewritten only when something changed,
through a unique temp file and a rename, best-effort and silent (a
failed write costs the next load a recompute and nothing else). The
every-turn path never depends on the sem1 child.

**The index is keyed to the embedder.** It carries the embedder's
identity — the endpoint or model name — and the vector length it
served. On load, a mismatch invalidates the whole cache and rebuilds:
a model swap leaves stale-dimension vectors that cosine's length guard
scores 0.0, which would silently kill the vector channel while `doctor`
names it live — the exact silent degradation this slice exists to
prevent. An index in the pre-key format (no embedder, no dims) reads as
a mismatch and rebuilds, so an existing cache self-heals on the first
load. The atomic rewrites — the index and the recall-count bump — land
through a unique `.<pid>.<n>.tmp` and a rename; a crash mid-write can
leave one behind, and `memories/*.tmp` is gitignored so an orphan is
never a fact.

## Routing: the partition, the spillover, and the failure axis

The instruction classifies by keywords — substring match on the
lowercased text, keyword-backed rather than model-backed because the
model path proved unreliable. The fork's Wave D table is consulted
first (`rendering`, `state-sync`, `deploy`, `build`, `change`, `deps`,
`docs`, ported verbatim from `memories/shapes.json`, with the
taxonomy's order deciding when two shapes' terms both appear), and
Fleet's own failure table second, so the fork's order rule is
undisturbed.

**The spillover is named and conditional.** Off-shape facts enter the
assembly only through one of three named routes:

- **(a)** the primary pool is empty — the off-shape facts are the
  answer;
- **(b)** the pool's best score is below `SPILLOVER_FLOOR` (0.50) — the
  shape matched without corroborating itself in a second dimension, so
  the strongest off-shape fact enters even under budget pressure,
  beside the weak pool;
- **(c)** the budget has room after the pool is in.

`SPILLOVER_FLOOR` is sized against the composite's 1.00 and against
`MIN_RECALL_SCORE` (0.20), and it was re-derived when the weights moved:
**0.50 is the point at which the shape's own answer carries DIRECTION.**
The most a fact with no direction evidence can score is full topical
coverage (vector 0.15 + keyword 0.15) with freshness and provenance
(0.15) — 0.45 — so the neighborhood alone can no longer stand alone as
the shape's answer. A same-shape fact clears it with a 0.2 trajectory
match on top of full topical coverage (0.51), or two recalls' history
(0.5065); the direction-strong fact scores 0.5028 and clears it with
0.0028 to spare — thin, because the history curve no longer pads it. Every off-shape fact that enters is MARKED `off-shape`
in its assembly line and on its card, and the surface names the route
that admitted it. Never silently mixed into the shape's answers.

**The failure axis.** The shape vocabulary gains the failure shapes —
what kind of thing broke, the avoidance half of the shelf — named for
Fleet's own failure record in `docs/known-issues.md`:

| Shape | What broke | Drawn from |
|---|---|---|
| `fail-open` | a dependency was unavailable and the path proceeded on a degraded answer instead of refusing | KI-044 (route selection's `sys1_classify` fails open, every run falls back), KI-110 (the improve-gate's probe failed open and could grade the wrong binary) |
| `drops-the-row` | a writer wrote a record the reader silently discarded, so the system behaved as if it never existed | KI-164 (a JSON `null` made `parse_row` drop the whole line), KI-171 (a hand-written row missing `family` dropped silently) |
| `bound-exceeded` | an unbounded or wrongly-bounded read or write overran its budget, undercounted, or rode control characters into the surface | KI-060 (the description unbounded), KI-167 (`READ_CAP=512` undercounts), KI-120/KI-122 (the tower scan and the second-brain read unbounded), KI-035 (no cap on a lane's stdout line) |
| `claim-outran-evidence` | a doc, a recap, a plan's assumption, or a test asserted more than the code or the run proved | KI-160 ("fleet's suite covers the lane" overclaimed), KI-159 (two doc claims read stronger than the code), KI-009 (a recap claimed the wrong CI state), KI-133 (the feature doc's network wording overclaimed) |
| `fixed-width-clip` | a fixed-width surface was handed a line built by concatenation rather than bounded to the pane's own width, and it clipped or hard-wrapped the line mid-token | KI-078 (the approval modal cut a long call summary mid-word; the subagent card's state token pushed off the line), KI-085 (the compaction card cut mid-token at 52 columns), KI-096 (the cron fire card cut at 50), KI-115 (a widened pane cut the compaction card again), KI-179 (three card lines grew past `CARD_WIDTH` at once) — five recurrences, with KI-055's one shared word-aware wrapper the named remedy and still open |
| `foreign-root` | a path was resolved from the working directory alone, so a run started outside any intended root silently adopted a foreign one | KI-074 (`resolve_root`'s ancestor walk adopted a corpus found further up), KI-087 (the scheduled run reproduced it verbatim), KI-099 (the improve gate gates against the wrong tree), KI-106 (`doctor --diagnose`'s default ledger path) — four surfaces, one root cause |

A fact tagged with a failure shape is recalled through the same
partition as a resolution, and the write path accepts the tags (the
three-tag cap is across all three axes).

**The outcome tags.** A failure shape is the PROBLEM's shape, not a
claim about how the trajectory ended — which is what the two-sign rule
got wrong. A fact about how a `drops-the-row` bug was FIXED is tagged
with the failure shape because that is the problem's shape, and under
the two-sign rule it rendered `diverged:`, warning the model off a
correct trajectory while the failure-shape header promised "what broke
before, and what resolved it" with the resolved half unable to occur. A
third axis fixes it:

| Tag | Axis | What it says |
|---|---|---|
| `incomplete` | outcome | the direction was right and the companions it needed were absent, so it drove partway and stalled |
| `resolved` | outcome | the trajectory reached the minima — load-bearing only beside a failure shape, where it says the fix landed and the fact is a candidate, not an anti-vector |

An outcome tag is not a problem shape: a fact carries one beside a
structure or failure shape, never instead of one. They classify last,
so an instruction that names a problem shape still routes to it, and an
instruction that names only an outcome ("the torch stack was missing")
routes to the third sign — the "have I stalled here before?" question.

**The three signs.** A fact carries a sign, derived from its tags rather
than stored, and the precedence is the order that keeps an anti-vector
from being read as a candidate:

- any failure tag ⇒ **divergence**, unless an outcome tag overrides it;
- `incomplete` ⇒ **incomplete** — it wins over a failure tag, because a
  stalled trajectory is not an anti-vector;
- `resolved` ⇒ **resolution** — even inside a failure-shape pool;
- nothing else ⇒ **resolution**.

A resolution drives toward the minima; a divergence drives the opposite
way; an incomplete drove the right direction and stalled. An anti-vector
that matches the current trajectory is highly relevant — it is the
avoidance half of the shelf — and it surfaces as prominently as a
resolution: same pool, same ranking, no penalty. The third sign is not a
penalty either: **an incomplete vector ranks exactly as a resolution**,
same pool and same weights, and what distinguishes it is the marker and
the missing companions named beside it. Every assembly line and every
card names its sign (`resolved:` / `diverged:` / `incomplete:`), the
divergence renders in a caution color and an incomplete one in the
accent on the surface, and the recall section carries a one-line legend
naming all three whenever it holds anything other than a resolution.
The model must never read an anti-vector as a candidate solution, and
never read a stalled trajectory as done.

**The phrases and the missing companions.** The write path records the
phrases that resolved the problem — the vocabulary that drove — and, for
a failure, the phrases of the trajectory that was tried and diverged.
They ride the fact's `phrases` frontmatter field beside `shape`, they are
part of the indexed text the vector channel embeds, and they are the
trajectory channel's own text. A later recall for the shape matches them
even when the question is worded differently — that is the slice-9
lesson's point: the instruction's only home is the tool's description.
The `missing` field is the third vector's: the companions an incomplete
trajectory needed and did not have, case-preserving, capped at
`MAX_MISSING` (6) and `MAX_MISSING_CHARS` (160) like the phrases, and
deliberately excluded from the indexed text. They ride the fact's
assembly line (`· missing: …`) and its card, so the absent pieces are
named where the fact is ranked rather than lost.

## Feedback: the measure rung

`Memory::recall_hits` is the feedback entry point. Every hit updates
the fact's `recall-count` in its file — a bounded, frontmatter-
preserving rewrite under a per-fact flock on a sibling `.lock` (KI-062), so a
consolidation pass and a live bump cannot lose an update: the one line it owns
is re-rendered, every other byte including the body is preserved, and the write
lands through a unique temp file and a rename — then re-ranks on the next recall (the
importance channel rose), and appends one `recall_hit` calibration row
naming the fact, the shape, and the score. The row is
`{"op":"recall_hit","fact":…,"shape":…,"score":…}` in devdec's stream,
so `fleet calib` cards it and the loop's effect is auditable. A file
that cannot be rewritten logs one line and the hit is still recorded —
the audit trail must not depend on the corpus being writable.

The turn path records what the assembly actually surfaced, through
`Memory::record_hits`: one ranking per turn, and the row's score is the
score the fact entered the turn at. A fact the budget dropped never
reached the model, so it is not a hit.

Never-recalled facts age toward consolidation: the cycle reads
`recall-count` as its signal.

## The write path: `remember`

`Remember` is the write path — the `brain_remember` shape the fork's
writer speaks, mechanism over prose. One structured request:

```
remember(
  name:         lowercase letters, digits, hyphens; the file stem and
                the key every surface uses (required)
  description:  the fact as one sentence (required)
  type:         free-form kind; user and feedback are the protected
                tier, with the fork's synonyms mapped onto them
                (identity/owner → user, preference/standing-preference →
                feedback — KI-059); default project
  shape:        comma-separated tags from the structure shapes
                (rendering, state-sync, deploy, build, change, deps,
                docs), the failure shapes (fail-open, drops-the-row,
                bound-exceeded, claim-outran-evidence, fixed-width-clip,
                foreign-root), and the outcome tags (incomplete,
                resolved), at most three in total
  phrases:      the phrases that resolved the problem — the vocabulary
                that drove toward the answer, quoted as it was used;
                for a failure, the trajectory that was tried and
                diverged
  missing:      the companions an INCOMPLETE trajectory was missing —
                the absent pieces that kept a correct direction from
                reaching the minima. Name them and tag the fact
                `incomplete`
  basis:        confirmed-by | inferred (required — every write)
  tier:         working | durable (default durable)
  valid-until:  YYYY-MM-DD (required for working; none for durable)
  body:         the fact's full content
)
```

The same field names are the CLI's flags (`fleet remember --name …
--description … --basis …`), and one validator serves both surfaces, so
they cannot drift — the CLI form has no `phrases` flag yet; the tool is
the phrases' home, and a CLI flag is a named follow-on. `basis` is
required on every write and `valid-until` on every working-tier one,
because those two rules are what make the corpus's provenance and
lifecycle mean anything. The name is validated
as a file stem — no path separator, no `.` or `..`, no dotfile — so a
write can never address a file outside the corpus, and a name that is
already taken is refused: the modify path is `supersede`, and a silent
overwrite would erase a confirmed-by fact's provenance.

**The phrases are the trajectory channel's input, and the tool's
description is their instruction's home** — the slice-9 lesson: an
instruction with no home where the model sees it does not exist. The
description asks for the exact vocabulary that drove toward the answer
(commands, file names, the words that worked) so a later recall for the
shape matches them even when the question is worded differently; it asks
for the sign (tag a failure with a failure shape and record the
trajectory that was tried and diverged); and it asks for the third
sign's outcome — tag a trajectory `incomplete` and name its companions
in `missing`, so "the bootstrap was correct but the torch stack was
missing" is recordable at all rather than being written as a resolution.
The phrases and the companions ride arrays of strings — a phrase
legitimately contains commas — bounded at `MAX_PHRASES` (6) and
`MAX_MISSING` (6), each entry at `MAX_PHRASE_CHARS` /
`MAX_MISSING_CHARS` (160), sanitized the way the description is, and
case-preserving because they are the vocabulary as it was used. They
render in the document beside their sibling `shape`, and a modify keeps
the direction the superseded fact recorded.

**KI-060's fix lives here.** The writer bounds and sanitizes the
description — control characters removed (not replaced, so a stripped
escape sequence leaves its payload visible and inert), then trimmed,
then bounded at `MAX_DESCRIPTION_CHARS` (512) with a marked cut — so
control characters never reach the turn context or the card renderer.
The name, type, and justification take the same treatment; the body is
bounded by bytes (`MAX_BODY_BYTES`, 64 KiB) and refused past it rather
than silently cut, because a fact that lost its tail would read as
whole.

The document renders in the fork's field order — `name`, `description`,
`type`, `date`, `basis`, `tier`, `shape`, `phrases`, `missing`,
`recall-count`, `superseded-by`, `valid-until` — with an unset optional
field bare, the fork writer's own "unset" rendering, through a unique
temp file and a rename. The file is the record: a written fact loads
with every field, which is what makes a bad fact a revert.

## The recall graph: one bounded hop over the frontmatter's own edges

The frontmatter carries relations between facts, and until the 2026-10-18
plan the ranker read none of them for retrieval:

- **`missing`** — the companions an incomplete trajectory needed and did
  not have. The field names them, and it is deliberately excluded from
  `Fact::indexed_text` because an absent companion is not the fact's
  direction — but nothing ever *fetched* the named fact either, so the
  corpus declared an edge and the retriever rendered it as prose. Since
  R4b an `incomplete` fact must name its companions: a third sign with
  nothing to finish the trajectory with is not a sign.
- **`superseded-by`** — this fact was folded into another, so the other
  is the current truth. The substrate *retires* the fact that carries the
  stamp (`Fact::forgotten` reads a non-empty value as retired), so it
  never enters a pool and there is no forward source to expand from.
  What it can still do is **vouch**: a retired fact whose own vocabulary
  matches an instruction points at the fact that replaced it, and that
  replacement is what the turn actually needs. A corpus whose only match
  is a fact the cycle folded away has an EMPTY primary pool, and that is
  exactly when the replacement must still surface.
- **a shared `shape` tag** — deliberately NOT an edge. The partition
  already reads it, so a shape hop would re-admit facts the ranking has
  already ordered: noise, not expansion.

`src/recall_graph.rs` builds the adjacency map at load and the ranker
takes **one hop** from a bounded number of the primary pool's strongest
hits, plus every retired fact that vouches. A neighbour enters on its
source's authority rather than its own score — the bar is the source's
own total against `MIN_RECALL_SCORE`, the same floor every other fact
faces — and the neighbour's own score rides along unchanged and honest.
Every neighbour is MARKED `via: <edge>` in its assembled line, the same
discipline the spillover's `off-shape` marker runs on, so a fact that
entered on another fact's authority is never silently mixed into the
shape's own answer. `GRAPH_HOPS` and `GRAPH_SOURCES` zero the whole
mechanism without removing it.

The graph does nothing on this repo's corpus yet — 0 `superseded-by`
edges and 15 empty `missing` fields at the 2026-10-18 dive — and that is
expected: R4b makes the writer populate the companions, and the
consolidation cycle already stamps `superseded-by` when it folds, so the
edges arrive as the corpus is used. The measured twelve-instruction set
(`the_measured_set_holds_the_floor_the_reweighting_bought`) is the gate
that watches whether they help.

## Three speeds

The `tier` field is how long a fact lives:

- **working** — session-scoped. It expires on its `valid-until` (which
  the writer requires), it enters a turn through the working tier
  rather than the ranking, and the consolidation cycle compresses it
  into an episodic summary. `Memory::working` is the tier's reader:
  every non-forgotten working fact that is not already protected, most
  recent first.
- **durable** — basis-recorded, no expiry. The tier the corpus's own
  facts are, and the default when the frontmatter carries no `tier`.
- **episodic** — a consolidated summary, written by the cycle and never
  by a write; durable-shaped and recallable. A hand-written `episodic`
  fact is refused: it would put an unaudited fact in the recallable
  store with no cycle pass behind it.

A working fact never enters the ranked section and the working tier at
once — `recall` excludes the working tier the way it excludes the
protected tier. No fact enters a turn twice.

## The protected tier and assembled context

Facts whose `type` marks identity or standing preferences are the
**protected tier**. The injection budget can never evict them. The tier is
`user` and `feedback` in the fork's taxonomy, and the loader maps the fork's
documented synonyms onto them — `identity`/`owner` → `user`,
`preference`/`standing-preference` → `feedback` (KI-059) — because a fact the
fork meant to protect would otherwise ride the ordinary tiers here. This is the deviation from both
current designs, and the fix for the live system's mis-tiered cap, where
the most load-bearing facts contended with task facts in the smallest
space.

Turn context is **assembled**, not dumped: protected tier first, then
shape-routed recall, then recent working memory, within
`ASSEMBLY_TOKEN_BUDGET` (2048 tokens, estimated at one token per four
characters — the fork's own estimate). The protected tier is added
first and is never dropped, even when it alone exceeds the budget —
that is the tier's whole point. The strongest recalled fact always
enters (the fork's `renderBlock` rule: a budget too small for it still
carries the strongest match rather than an empty block); the rest enter
while they fit, and the ones that do not are named in the surface's
"dropped under budget" line. Protected facts are excluded from the
recall ranking, and working facts from both the ranking and the
protected tier's duplicate — no fact enters a turn twice.

The budget is 2048 against DeepSeek's 128k context, not the fork's
512: the seed corpus's seven protected facts cost ~450 tokens on their
own (the module's own estimate, measured on the real tree), so at 512
the second ranked fact — ~85 tokens — did not fit and ranked recall
got exactly one fact beside the tier. The floor (`MIN_RECALL_SCORE`,
0.20) is the coarse noise control and the budget the fine one; 2048
bounds a turn to ~25-30 facts while letting the protected tier and
ranked recall coexist.

The observed failure this avoids: a turn about one topic injected with
unrelated facts. The floor, the ranking, and the budget together are
the mechanism; the floor alone is the coarse guard, the budget the fine
one.

## Mid-run curation: the block re-ranks when the conversation moves

The assembly runs once, against the opening task — and a session does
not stay on its opening task. The turn loop's drift trigger re-runs the
same ranker against the current instruction when the conversation has
moved: a steer's vector is compared against the last curated
instruction's, and a cosine under `DRIFT_FLOOR` (0.78, measured on this
machine's embeddinggemma over steer-shaped pairs — unrelated steer
pairs score 0.647–0.729, paraphrases 0.846–0.925) re-curates. The
**first steer always curates**: the conversation changed hands mid-run,
so the opening block is stale whatever the vectors say. A corpus with
no serving embedder treats every steer as drifted — the safe bias when
movement cannot be measured.

The curated note rides the **message tail**, not the system prompt: a
system message at the end of that one request's message list, never
inside the system message itself, whose byte-stability the
provider-side prefix cache keys on. It is turn-scoped — the next
request does not carry it — and the log's `curated` record names the
turn, the shape, and the facts, so the note is reconstructable like the
opening block is. Recall hits re-arm with the curation (KI-068's rule,
one seam later): a hit means the model saw the fact, once per
curation. The comparison base advances to each steer even when no
curation fires, so a slow walk of small shifts accumulates instead of
hiding. The cost is bounded by the trigger: one instruction embed per
steer to measure, plus the ranking's own embed on a real curation —
the same shape the `recall` tool pays — against fact vectors that are
already cached.

## The curation judgment: the lane's vote beside the floor

The drift trigger's cosine is a distance, and a distance cannot read a
move: a steer that contradicts the thread it lands in scores high
against that thread's own words. The `curate` task in the vendored sys1
judges the move — a two-part pair (the instruction the block was last
curated against, then the steer) answered with `needs_recall`,
`focus`, and `staleness`, cached in the route cache under the pair's
hash and behind the same decision-keys gate the router runs (an
unconfigured machine pays nothing).

The composition is measured, and the measurement put the judgment in an
advisory seat: replayed against real session `997a54b4`'s four steers
through a real sys1 child, the judged decisions were identical to the
cosine-only ones — zero new mis-fires — but did not dominate (no early
fire; three of four votes under the 0.6 confidence gate). So the lane
votes in the calibration stream's `curate` rows — verdict, cosine,
decision, trigger — while the cosine decides. The flag that awards the
seat (`CURATE_JUDGED_DECIDES`) flips on a replay where the judgment
earns it, not on appetite. The focus label is the one contribution that
acts today: when the lane names a shape the corpus's vocabulary carries
and the curation fires anyway, the re-curation routes under it instead
of the keyword classifier's word, and the `curated` record names which
did it.

## The consolidation cycle

A timer runs the cycle — `CONSOLIDATE_INTERVAL` (hourly) — in the
long-lived processes: `chat` and the TUI spawn
`memory::spawn_consolidator`, which takes one pass at once and then one
every hour for the life of the process. The live system's consolidation
is manual and its episodic store has been dormant for weeks; the timer
is the fix. `fleet memories --consolidate` runs one pass on demand and
prints what it did.

One pass, three actions, each fact taking the ones its state calls for:

- **compress** — a working fact that is past its `valid-until` or older
  than `WORKING_MAX_AGE_DAYS` (7) is folded into the pass's episodic
  summary and stamped `superseded-by` it. The file stays; the summary's
  body carries the folded bullets, and its description — the part
  recall surfaces — names the fold.
- **supersede** — a durable `inferred` fact recall has never surfaced,
  older than `STALE_AFTER_DAYS` (180), is folded into the same summary
  and stamped. A `confirmed-by` fact in that shape is left alone and
  named on the surface: provenance decides, not age. An **incomplete**
  fact in that shape is left alone and named too — the third-vector
  rule, and a decision worth naming: the summary's bullets carry a
  fact's name, date, and description, never its missing companions, so
  folding one would keep the sentence and lose the absent pieces, which
  are the fact's whole point. An incomplete durable inferred fact is
  never folded, whatever its age.
- **forget** — a fact whose `valid-until` has passed and that no summary
  carries is moved to `memories/.archive/`. Nothing is deleted: the
  file stays, reversible by a move back, and diffable in git — it
  simply leaves the loader's read, which takes `memories/*.md` and
  never a subdirectory. A name the archive already holds takes a
  serial suffix — `.archive/<name>.md`, then `.archive/<name>.2.md`,
  `.archive/<name>.3.md` — so a re-used fact name never clobbers the
  copy already there; the name is claimed in one step, so two
  archivers cannot pick the same serial.

A fact that is past its `valid-until` gets two of them, in order:
compress folds its content into the summary, then forget archives the
expired original. What recall surfaces afterwards is the summary's
**description** — "the consolidation cycle's `<day>` pass — N facts
folded" — because a recalled fact enters a turn as its description,
never its body. The folded bullets (each fact's name, date, and
description) live in the summary's **body** on disk: the audit record
of what was compressed, not context the model sees. The archived
original leaves the working tree but stays on disk, reversible by a
move back.

The pass takes `today` as a parameter — the seam a test injects a fixed
day through, so the cycle is deterministic rather than sleep-dependent.
Every action appends one `consolidate` calibration row:
`{"op":"consolidate","action":"compress|supersede|forget","fact":…,"reason":…}`,
with the reason the named bound or the proposal's justification. The
summary is one fact per pass, named `consolidation-<date>` (then `-2`,
`-3`, so two passes never collide), listing everything the pass folded.

## Provenance as the permission model

`Memory::supersede` is the modify/forget path, and the rule lives at
the module seam so a programmatic caller gets the same refusal an
interactive one does:

```
supersede(
  name:           the fact being retired (required)
  justification:  why — required, recorded in the calibration row and
                  the replacement's body
  replacement:    the new fact's name, for the modify path; omit to
                  forget
  description:    the replacement's one-sentence fact (required with a
                  replacement)
  body, type, shape, valid-until:
                  the replacement's own fields; each defaults to the
                  superseded fact's
  missing:        the replacement's missing companions. **Inherit-or-clear**:
                  omit it to keep the companions the trajectory was
                  missing (a reworded fact does not unmiss them), pass
                  an explicit empty list to clear them because they
                  arrived, or pass a list to state the new ones. There
                  is no way to say "unknown" — an absent argument is an
                  inheritance, and a list is a statement
)
```

- A **confirmed-by** fact cannot be modified or forgotten without
  approval. The tool crosses the ordinary approval gate like every
  call, and a confirmed-by target then asks the Approver a second time,
  with a prompt that names the fact and its confirmed-by basis: the
  ordinary gate approves tool calls, and this one approves the
  modification of a load-bearing fact. A latched approver (`--yes`, the
  `a` key) answers both without a prompt, which is what `--yes` means.
- An **inferred** fact is supersedable on its justification — the
  justification is the permission, and it is recorded in the
  calibration row and the replacement's body. A supersession without
  one is a typed error.

The modify path writes the replacement (always `inferred` — a
supersession is a proposal until someone confirms it) and stamps the
old fact `superseded-by` it; the forget path stamps `valid-until` to
the day before the retirement and appends the reason to the body, so
the loader retires it now and the cycle's next pass archives the file.
Nothing is deleted either way. This is slice 12's guardrail input: the
self-maintenance loop may supersede an inferred fact on its own
justification, and a confirmed-by one waits for approval.

A stamp that fails to land fails the whole supersession with a typed
`WriteError::Stamp` and records no calibration row: the audit trail
cannot claim an action the files did not take. The modify path writes
the replacement first and stamps after, so its error names the
replacement that landed beside the stamp that did not; the forget path
lands both of its writes — the expiry and the retirement note — before
its row.

## The turn path

The assembled context enters the agent's runs. A run that carries a
corpus (`AgentOptions::memory`) does four things a run without one does
not:

1. **The system prompt gains the block** — the protected tier, the
   shape-routed recall, and the recent working memory, in a `## Memory`
   frame that says how to read it: recalled facts are proposals to
   verify, the protected tier is the owner's own.
2. **The write path is declared** — `remember` and `supersede` beside
   `use_skill`, after the four built-ins. A durable write is a side
   effect, so it crosses the same Approver as the other tools, and its
   summary names the fact and its basis.
3. **One `memory` session-log record per run** — the shape and every
   fact name the block carried, in block order. The block is dynamic
   two ways over (the corpus's files change between runs, and the block
   depends on the instruction), so the record is what keeps the prompt
   the model saw reconstructable from the log (hard rule 5, the `skills`
   record's precedent). It replays to no message.
4. **The recall hits are recorded** — one `recall_hit` row per fact that
   entered, at the score it entered at.

An assembly with nothing in it adds no block, no record, and no rows: a
memory-less turn is byte-for-byte the run this crate has always had.
The corpus is loaded per task, and the probe's answer is cached per process
in a `OnceLock` (KI-064), so a chat session pays one round trip rather than
one per message — a fact an earlier run remembered is on disk by the time the
next one assembles. The headless path prints one progress line naming
the shape and the fact count; the TUI's status bar carries the same
fact.

## The surfaces

The face's memory surface is the work rail's idiom, shared with settings,
bots, and skills: a rail of sections, one section at a time, one-line
rows, the right pane for the item. The sections: **needs you** — the
consolidation cycle's queue, computed and written nowhere by the same
filters the pass acts on (`memory::preview`), each set named with what
the cycle would do and why the confirmed and incomplete exceptions were
left, with **run the cycle now** landing one pass through the cycle's
own function; the **type words** — user, feedback, project, reference,
episodic (the protected synonyms mapped, KI-059), and `everything else`
for the free-form types the substrate permits; then **retired**,
**archive** (what the cycle forgot, read-only), and **today's recall**.
A section is an address (`#/memory/<section>`, remembered); the search
reads every field across every section; rows carry the basis (the
permission model's input) and the recall count; the row budget shows the
first 30 with the count named. A row's press opens the pane card: the
body, the in-place edit (the substrate's own rewriter, unchanged), and
**supersede** — the modify/forget verb, whose click is the approval the
provenance rule requires and whose justification is the parser's
requirement either way.

```
fleet remember --name <name> --description "<fact>" --basis confirmed-by|inferred
                                [--type <kind>] [--shape a,b] [--tier working|durable]
                                [--valid-until <date>] [--body "<text>"]
                                write one fact, confirmed with its card; the
                                CLI form of the remember tool — the user typing
                                the write is the approval, so there is no gate
fleet memories                              cards: name, type, shape, basis,
                                            recall-count, valid-until, path, the
                                            phrases a fact carries, and its
                                            missing companions; the
                                            protected/retired tallies; the backend
                                            the probe found
fleet memories --recall "<instruction>"     the shape-first recall: the primary
                                            pool's cards, then the off-shape
                                            spillover marked and named for the
                                            route that admitted it, each card
                                            naming the evidence that ranked it
                                            (the channels, the trajectory, the hit
                                            history, the basis, the sign, and the
                                            missing companions), then the block
                                            that would enter a turn's context,
                                            then what the budget dropped
fleet memories --shape <name>               the cards filtered by shape — a
                                            structure shape, a failure shape, or
                                            an outcome tag; an empty filter names
                                            all three taxonomies
fleet memories --consolidate                one consolidation pass, with what
                                            it did as cards, then the corpus
                                            the pass left behind
fleet memories --archive                    the facts the cycle's forget
                                            action archived
```

`--recall` is a **dry run**: printing changes no recall-count and
records no row — the turn path that records hits is the agent's. Its
header names the shape and its axis, the backend, how many facts
ranked, how many are in the pool and how many are off-shape, and how
many fit the budget. The pool's cards render a resolution's sign in the
plain text color, a divergence's in a caution color, and an incomplete
one in the accent, so the operator reads "what worked here", "what
diverged here", and "what stalled here" at a glance; the spillover's
divider names the route that admitted it.

**The surface's lines are bounded.** The recall surface's status line,
its dividers, and a card's standing line are concatenations that grow
with the corpus — a backend label, a fact count, a route reason, every
tag a fact carries — and an unbounded concatenation on a fixed-width
surface is the `fixed-width-clip` class (KI-078, KI-085, KI-096,
KI-115, KI-179: five recurrences on five surfaces, and this surface was
the sixth). They wrap to `SURFACE_WIDTH` (110) through the shared
word-aware wrapper, at a word boundary rather than mid-token, and the
card's own lines wrap to `CARD_WIDTH`; a test pins both at 80 and at
120 columns. The block that would enter a turn's context is not a
terminal surface, so it is not bounded to a column count. The flags do not
combine where they answer different questions: `--recall` and `--shape`
are two questions, `--archive` is its own surface, and `--consolidate`
runs the cycle and then cards the corpus (optionally filtered by
`--shape`). The corpus resolves home-first (KI-087, closing KI-074's root cause): the
fleet home's own `memories/` wins the moment it HOLDS facts, and only then does
the ancestor walk from the working directory apply — so a run started outside
any intended corpus cannot silently adopt a foreign one. In everyday use that
is the repo root. The exit code is always success — this is a
read surface over Fleet's own files, like `fleet calib` and `fleet
skills` — except `fleet remember`, which exits 1 on a typed refusal (a
taken name, a missing basis, a working fact with no `valid-until`).

## What changed from the boost design

Slice 8's routing added `SHAPE_BOOST` (0.15) to a composite whose word
channels carried 0.80 (vector 0.55 + keyword 0.25). That failed the
owner's directive in both directions: a homonym across shapes surfaced
the wrong fact confidently — the boost could not overcome a word gap —
and a synonym within one shape missed the right one, because nothing in
the score could see that a fact had driven toward this problem before.
Slice 19 removed the constant and inverted the design:

- The shape is structural. It selects the primary pool; every other
  fact is spillover, admitted only through the three named routes and
  marked when it enters. No term in the score can cross the partition,
  so a wrong-shape fact with perfect word overlap cannot outrank the
  right shape's own answer.
- The words corroborate within the shape, as one dimension among
  several: the topical pair dropped from 0.80 to 0.30 (vector 0.15 +
  keyword 0.15), the trajectory channel (the resolution's phrases)
  joined at 0.30, the magnitude (history 0.25 plus provenance 0.05)
  became visible to the ranking, and freshness stayed the smallest
  channel. `basis` entered the score for the first time — it was read
  only by the permission model before. The first reweighting put the
  topical pair at 0.40 and still failed the owner's ruling: a
  maximum-proximity, zero-direction fact could score 0.55 against a
  direction-strong fact's 0.49. The second reweighting is the one that
  holds the line — proximity 0.30 against direction 0.60, with a 0.095
  margin on the canonical pair of numbers.
- The failure shapes joined as a first-class axis with a sign: a
  failure is an anti-vector, recalled through the same partition and
  marked so it is never read as a candidate.
- The third sign joined them. An incomplete solution — direction right,
  companions absent — ranks exactly as a resolution and is surfaced
  with its missing companions named, so the vector most mistaken for
  done is never read as finished. The outcome tags (`incomplete`,
  `resolved`) are what let a failure-shape fact say what became of its
  trajectory, which the two-sign rule could not: a fix recorded under
  the problem's own shape used to render as an anti-vector.
- `SPILLOVER_FLOOR` moved from 0.45 to 0.50 when the weights did, so
  the point at which the shape's own answer may stand alone is the
  point at which it carries direction.

## Limitations

- **The phrases can be gamed.** A fact can record phrases that parrot
  the question — "the railway deploy is failing" as a resolution's
  recorded vocabulary — and score a full trajectory match without
  having driven anywhere. Nothing in the path can tell recorded
  vocabulary from invented vocabulary; what limits the damage is that
  the trajectory channel is one dimension among six, and the
  partition and the spillover's routes still decide which pool a fact
  is read from. A future slice could cross-check a fact's phrases
  against the recall hits it earns.
- **A failure-shape fact is only as right as its outcome tag.** A fact
  tagged with a failure shape and no outcome tag reads as an
  anti-vector; a fix recorded without `resolved` warns the model off a
  correct trajectory, and a stall recorded without `incomplete` reads
  as either done or diverged. The tool's description is the
  instruction's home and it names all three, but the tag is the
  model's to write.
- **The floor does not separate a near-miss.** The live embedding
  model's cosines do not separate relevance on their own (KI-058), so
  an unrelated same-domain fact can clear `MIN_RECALL_SCORE` on the
  vector channel alone (0.24 on this machine's normal configuration).
  The partition and the marked spillover are what keep it from being
  read as the shape's answer.
- **The consolidation cycle cannot carry a third vector's
  companions.** Folding an incomplete fact would keep its sentence and
  lose the named absent pieces, so the cycle leaves one alone whatever
  its age — which means an incomplete fact the corpus never uses stays
  in the tree until a human retires it.

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

## What this slice does not do

No import of the fork's 234-fact corpus — the seed corpus is Fleet's
own nine facts, each one true and cited. No model-written memory prose:
the write path is mechanism-constrained and the cycle's summaries are
deterministic. No separate session-scoped store: a working-tier fact is
a file in the corpus like any other, and the cycle is what keeps the
tree from growing without bound.
