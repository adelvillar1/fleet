# Memory (slice 8)

Fleet's semantic memory: the git-tracked `memories/` fact substrate,
hybrid retrieval with the vector path probed rather than assumed,
Wave-D shape-routed recall, recall-hit feedback, the protected tier
with budget-aware turn-context assembly, the `brain_remember`-shaped
write path, the scheduled consolidation cycle, provenance as the
permission model, and the turn-path wiring that carries the assembled
block into every run.

Part 1 landed the substrate, retrieval, routing, feedback, and the
protected tier; part 2 landed the write path, the consolidation cycle,
provenance, and the turn wiring. Both are in `src/memory.rs`.

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
`tier`, `shape`, `recall-count`, `superseded-by`, and `valid-until`,
with the file's body as the fact's full content — the fork's
`memories/` tree (234 facts in exactly this shape) is the spec. The
description is the one-sentence fact: what the index embeds and what an
assembled turn carries; the body is the full content on disk, read on
demand the way a skill's body is.

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
at write, and basis is provenance — the permission model's guardrail
reads, not a rank.

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
  shape:        comma-separated tags from rendering, state-sync,
                deploy, build, change, deps, docs (at most three)
  basis:        confirmed-by | inferred (required — every write)
  tier:         working | durable (default durable)
  valid-until:  YYYY-MM-DD (required for working; none for durable)
  body:         the fact's full content
)
```

The same field names are the CLI's flags (`fleet remember --name …
--description … --basis …`), and one validator serves both surfaces, so
they cannot drift. `basis` is required on every write and `valid-until`
on every working-tier one, because those two rules are what make the
corpus's provenance and lifecycle mean anything. The name is validated
as a file stem — no path separator, no `.` or `..`, no dotfile — so a
write can never address a file outside the corpus, and a name that is
already taken is refused: the modify path is `supersede`, and a silent
overwrite would erase a confirmed-by fact's provenance.

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
                                            recall-count, valid-until, path;
                                            the protected/retired tallies; the
                                            backend the probe found
fleet memories --recall "<instruction>"     the ranked assembly: every fact
                                            above the floor with its score and
                                            the channels that produced it, then
                                            the block that would enter a turn's
                                            context, then what the budget
                                            dropped
fleet memories --shape <name>               the cards filtered by structure
                                            shape; an empty filter names the
                                            taxonomy
fleet memories --consolidate                one consolidation pass, with what
                                            it did as cards, then the corpus
                                            the pass left behind
fleet memories --archive                    the facts the cycle's forget
                                            action archived
```

`--recall` is a **dry run**: printing changes no recall-count and
records no row — the turn path that records hits is the agent's. The
flags do not combine where they answer different questions: `--recall`
and `--shape` are two questions, `--archive` is its own surface, and
`--consolidate` runs the cycle and then cards the corpus (optionally
filtered by `--shape`). The corpus is the nearest `memories/` at or
above the working directory (the fork's `resolveMemoryRoot` rule); in
everyday use that is the repo root. The exit code is always success —
this is a read surface over Fleet's own files, like `fleet calib` and
`fleet skills` — except `fleet remember`, which exits 1 on a typed
refusal (a taken name, a missing basis, a working fact with no
`valid-until`).

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
