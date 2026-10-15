# Memory (slice 8, slice 19)

Fleet's semantic memory: the git-tracked `memories/` fact substrate,
hybrid retrieval with the vector path probed rather than assumed,
shape-first recall, the failure axis with its two signs, the
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
`tier`, `shape`, `phrases`, `recall-count`, `superseded-by`, and
`valid-until`, with the file's body as the fact's full content — the
fork's `memories/` tree (234 facts in exactly this shape) is the spec,
with `phrases` Fleet's own slice-19 addition beside its sibling
`shape`. The description is the one-sentence fact: what the index
embeds and what an assembled turn carries; the body is the full content
on disk, read on demand the way a skill's body is.

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
| `W_VECTOR` | 0.20 | cosine to the instruction (the embedded direction) |
| `W_KEYWORD` | 0.20 | the instruction's tokens the fact's topic text covers |
| `W_TRAJECTORY` | 0.25 | the instruction's tokens the recorded **phrases** cover — the fact's direction |
| `W_HISTORY` | 0.20 | `recall-count` over `RECALL_CEILING` (8), earned by use |
| `W_FRESHNESS` | 0.10 | exponential freshness, half-life `DECAY_HALFLIFE_DAYS` (90) |
| `W_PROVENANCE` | 0.05 | the basis: confirmed-by 1.0, inferred 0.5 |

The topical pair (`W_VECTOR` + `W_KEYWORD`) is 0.40 — less than half
the composite — so a fact that merely talks about the neighborhood
cannot outrank the vector that drove toward this minima. The trajectory
channel is the largest single one, because the phrases recorded during
the resolution are the owner's key information. The magnitude is
history plus provenance (0.25 together): a confirmed-by vector that
recall has surfaced eight times has driven somewhere; an inferred one
is a proposal at half strength, not erased. Freshness is the smallest
channel on purpose — a fresh fact is not a better vector. All six are
named constants in `src/memory.rs`, documented and tunable.

The topical channels score the fact's **topic text** (the description
plus the shape tags); the trajectory channel scores the fact's
**phrases** (the resolution's own vocabulary). The two dimensions
cannot double-count each other. The vector channel embeds the fact's
**indexed text** — the topic text with the phrases riding along — so
the embedding carries the direction too.

`MIN_RECALL_SCORE` (0.20) is the floor below which a fact is noise for
this instruction. Its meaning is exact under the named weights: an
unrelated fact's best case is freshness plus provenance (0.15) plus one
shared stopword's worth of topical overlap (0.04) — 0.19, just under —
and a quarter-strength match in any single channel beside standing
clears it. A heavily-recalled fact that matches nothing still clears
it, ranks last, and is the budget's first drop, exactly as before.

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
- **(b)** the pool's best score is below `SPILLOVER_FLOOR` (0.45) — the
  shape matched without corroborating itself in a second dimension, so
  the strongest off-shape fact enters even under budget pressure,
  beside the weak pool;
- **(c)** the budget has room after the pool is in.

`SPILLOVER_FLOOR` is sized against the composite's 1.00 and against
`MIN_RECALL_SCORE` (0.20): full topical coverage with freshness and
provenance is 0.35, and four recalls' history (0.10) or a 0.8
trajectory match (0.20) carries a same-shape fact over 0.45 — the
point at which the shape's own answer is multidimensional. Every
off-shape fact that enters is MARKED `off-shape` in its assembly line
and on its card, and the surface names the route that admitted it.
Never silently mixed into the shape's answers.

**The failure axis.** The shape vocabulary gains the failure shapes —
what kind of thing broke, the avoidance half of the shelf — named for
Fleet's own failure record in `docs/known-issues.md`:

| Shape | What broke | Drawn from |
|---|---|---|
| `fail-open` | a dependency was unavailable and the path proceeded on a degraded answer instead of refusing | KI-044 (route selection's `sys1_classify` fails open, every run falls back), KI-110 (the improve-gate's probe failed open and could grade the wrong binary) |
| `drops-the-row` | a writer wrote a record the reader silently discarded, so the system behaved as if it never existed | KI-164 (a JSON `null` made `parse_row` drop the whole line), KI-171 (a hand-written row missing `family` dropped silently) |
| `bound-exceeded` | an unbounded or wrongly-bounded read or write overran its budget, undercounted, or rode control characters into the surface | KI-060 (the description unbounded), KI-167 (`READ_CAP=512` undercounts), KI-120/KI-122 (the tower scan and the second-brain read unbounded), KI-035 (no cap on a lane's stdout line) |
| `claim-outran-evidence` | a doc, a recap, a plan's assumption, or a test asserted more than the code or the run proved | KI-160 ("fleet's suite covers the lane" overclaimed), KI-159 (two doc claims read stronger than the code), KI-009 (a recap claimed the wrong CI state), KI-133 (the feature doc's network wording overclaimed) |

A fact tagged with a failure shape is recalled through the same
partition as a resolution, and the write path accepts the tags (the
three-tag cap is across both axes).

**The two signs.** A fact carries a sign, derived from its tags rather
than stored: a failure-shape tag makes the fact an **anti-vector**, and
every other fact is a **resolution**. A resolution drives toward the
minima; a failure drives the opposite way. An anti-vector that matches
the current trajectory is highly relevant — it is the avoidance half of
the shelf — and it surfaces as prominently as a resolution: same pool,
same ranking, no penalty. What keeps it from being read as a candidate
is the marker: every assembly line and every card names its sign
(`resolved:` / `diverged:`), the divergence renders in a caution color
on the surface, and the recall section carries a one-line legend when
it holds an anti-vector. The model must never read an anti-vector as a
candidate solution.

**The phrases.** The write path records the phrases that resolved the
problem — the vocabulary that drove — and, for a failure, the phrases
of the trajectory that was tried and diverged. They ride the fact's
`phrases` frontmatter field beside `shape`, they are part of the
indexed text the vector channel embeds, and they are the trajectory
channel's own text. A later recall for the shape matches them even when
the question is worded differently — that is the slice-9 lesson's
point: the instruction's only home is the tool's description.

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
                tier (default project)
  shape:        comma-separated tags from the structure shapes
                (rendering, state-sync, deploy, build, change, deps,
                docs) and the failure shapes (fail-open, drops-the-row,
                bound-exceeded, claim-outran-evidence), at most three
                in total
  phrases:      the phrases that resolved the problem — the vocabulary
                that drove toward the answer, quoted as it was used;
                for a failure, the trajectory that was tried and
                diverged
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
shape matches them even when the question is worded differently, and it
asks for the sign: when what is being recorded is a failure, tag it
with a failure shape and record the trajectory that was tried and
diverged. The phrases ride an array of strings — a phrase legitimately
contains commas — bounded at `MAX_PHRASES` (6) and `MAX_PHRASE_CHARS`
(160), sanitized the way the description is, and case-preserving
because they are the vocabulary as it was used. They render in the
document beside their sibling `shape`, and a modify keeps the direction
the superseded fact recorded.

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
`type`, `date`, `basis`, `tier`, `shape`, `recall-count`,
`superseded-by`, `valid-until` — with an unset optional field bare, the
fork writer's own "unset" rendering, through a unique temp file and a
rename. The file is the record: a written fact loads with every field,
which is what makes a bad fact a revert.

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

Facts whose `type` marks identity or standing preferences — `user` and
`feedback` in the fork's taxonomy — are the **protected tier**. The
injection budget can never evict them. This is the deviation from both
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
  named on the surface: provenance decides, not age.
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
The corpus is loaded per run (one probe round trip plus the cached
index), so a fact an earlier run remembered is on disk by the time the
next one assembles. The headless path prints one progress line naming
the shape and the fact count; the TUI's status bar carries the same
fact.

## The surfaces

```
fleet remember --name <name> --description "<fact>" --basis confirmed-by|inferred
                                [--type <kind>] [--shape a,b] [--tier working|durable]
                                [--valid-until <date>] [--body "<text>"]
                                write one fact, confirmed with its card; the
                                CLI form of the remember tool — the user typing
                                the write is the approval, so there is no gate
fleet memories                              cards: name, type, shape, basis,
                                            recall-count, valid-until, path, and
                                            the phrases a fact carries; the
                                            protected/retired tallies; the backend
                                            the probe found
fleet memories --recall "<instruction>"     the shape-first recall: the primary
                                            pool's cards, then the off-shape
                                            spillover marked and named for the
                                            route that admitted it, each card
                                            naming the evidence that ranked it
                                            (the channels, the trajectory, the hit
                                            history, the basis, the sign), then the
                                            block that would enter a turn's
                                            context, then what the budget dropped
fleet memories --shape <name>               the cards filtered by shape — a
                                            structure shape or a failure shape; an
                                            empty filter names both taxonomies
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
plain text color and a divergence's in a caution color, so the operator
reads "what worked here" and "what diverged here" at a glance; the
spillover's divider names the route that admitted it. The flags do not
combine where they answer different questions: `--recall` and `--shape`
are two questions, `--archive` is its own surface, and `--consolidate`
runs the cycle and then cards the corpus (optionally filtered by
`--shape`). The corpus is the nearest `memories/` at or above the
working directory (the fork's `resolveMemoryRoot` rule); in everyday
use that is the repo root. The exit code is always success — this is a
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
  several: the topical pair dropped from 0.80 to 0.40, the trajectory
  channel (the resolution's phrases) joined at 0.25, the magnitude
  (history 0.20 plus provenance 0.05) became visible to the ranking,
  and freshness stayed the smallest channel. `basis` entered the score
  for the first time — it was read only by the permission model before.
- The failure shapes joined as a first-class axis with a sign: a
  failure is an anti-vector, recalled through the same partition and
  marked so it is never read as a candidate.

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
