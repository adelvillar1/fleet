# Deep dive: the memory substrate, 2026-11-07

A current-state survey of Fleet's memory implementation — `src/memory.rs`
(12,344 lines), `src/recall_graph.rs`, `src/session_recall.rs`, and every
seam that reads or writes the corpus — read in its primary sources on
`face-plugin-host` at `16fc679`. Companion to `deep-dive-2026-10-15.md`
(crate-wide) and the contract in `docs/features/memory.md`; where the
feature doc and the code disagree, this names it. Every claim below was
read at its file:line this session; the two places the dive infers
beyond what the code says are labeled as inference.

## The substrate

One markdown file per fact under `memories/`, loaded by
`load_facts_matching` (`memory.rs:988`) — `*.md` at one level, sorted by
name, a malformed fact skipped by name with one logged line, the
stem-equals-name rule enforced on the live corpus and relaxed on
`.archive/` (`load_facts_archived`, `:980`). The frontmatter parser is
the skills parser's sibling, not a reuse: bare optional fields
(`superseded-by:` empty), indented continuation scalars, and bracketed
flow lists are accepted, everything else the fork's writer spoke loads
unchanged.

**The corpus root resolves home-first** (`resolve_root_from`,
`memory.rs:916`): the fleet home's own `memories/` wins the moment it
HOLDS facts (`holds_facts`, `:896` — an empty install scaffold must not
shadow a walked-up corpus), then a 12-deep ancestor walk from the cwd.
This closes KI-074/087. Verified live shape: `~/.fleet/memories/` is
empty today, so every run in this repo reads the repo's own corpus.

## Retrieval

**The embedder is probed, never assumed.** `ProbeSlot`
(`memory.rs:2428`) is a mutex-guarded enum — Unprobed, Probing, Ready,
Failed(since) — with a 60 s cooldown (`:2439`). `probed_embedder()`
(`:2464`) never blocks: an unresolved probe answers `KeywordOnly` and
the probe lands on its own thread. This is the KI-385 family's fix —
the old `OnceLock::get_or_init` held a tokio worker for up to sixteen
seconds and a failed startup probe stayed failed for the process's
life. `probed_embedder_ready(timeout)` (`:2491`) is the one-shot CLI's
seam (`fleet memories --recall` waits; a server never does). The probe
is one real embedding round trip against
`http://127.0.0.1:8901/v1/embeddings` (`:1035`), overridable by
`FLEET_EMBEDDING_ENDPOINT`; retries and embed attempts are bounded
constants (`:1104-1144`).

**The vector index** (`Index`, `:1367`) lives at
`memories/.recall-index.json`, gitignored. Entries are keyed by the
digest of the indexed text (KI-228 — a recall-count bump moves mtime but
not the digest, so the veterans stopped being re-embedded every load),
and the whole cache is invalidated by an embedder mismatch
(`keyed_for`, `:1422`) — a model swap can never serve stale-dimension
vectors. `load_with` (`:2610`) embeds only what changed, rewrites the
index only when something changed or an entry orphaned (KI-231,
`:2686`), and collects per-embed wall clocks for the KI-249
measurement. `load_cached` (`:2578`) is the request-path read: index
only, no embeds, no writes. `from_facts` (`:2732`) builds the same
Memory over the session archive's synthetic facts, with the archive's
own vectors — and a synthetic Memory must never cross the hit-recording
path, because a synthetic fact's path is a session log.

**The shape partitions, the words corroborate.** `partition_with_shape`
(`:2828`) classifies the instruction by keyword tables (the fork's Wave
D structure table first, then Fleet's failure table, then the outcome
tags), embeds the instruction, and scores every non-protected,
non-working, non-forgotten fact through `Memory::score` (`:3103`). The
partition is structural: no term in `Score::compute` (`:1626`) can move
a fact across it — the shape is deliberately not an argument to the
scorer. The six channels are the named constants (`:1475-1515`):
vector 0.15, keyword 0.15, trajectory 0.30, history 0.25, freshness
0.10, provenance 0.05. History is the log curve over RECALL_CEILING 128
(`history_term`, `:1547` — a count of 128+ reads a flat 1.0);
freshness halves every 90 days. `MIN_RECALL_SCORE` 0.20 is the floor
(`:1578`), `SPILLOVER_FLOOR` 0.50 the weak-pool signal (`:1600`).
The stemmer is deliberately short — 15 suffix rules (`:1769`) — measured
at 6 of 99 query tokens against the corpus's code-shaped prose.

**The spillover enters through three named routes** (`assemble_within_shape`,
`:3529-3578`): empty pool, pool-best under the floor, or budget room —
each route rendered as the reason the surface prints, every off-shape
line marked.

**The recall graph** (`recall_graph.rs`) walks one hop
(`GRAPH_HOPS = 1`) from the primary pool's top three
(`GRAPH_SOURCES = 3`), plus every retired fact that vouches through
`superseded-by` — the empty-primary-pool case is exactly when the
replacement must surface (`graph_hits`, `memory.rs:2961-2985`). A
neighbour enters on its source's floor-passing authority, marked
`via: <edge>`; a shared shape tag is deliberately not an edge. Dangling
edge names drop silently (the corpus is hand-editable). `missing` edges
are nearly theoretical today — see the census.

**The two-stage prefilter** (`SLICE_DIMS` 128, `PREFILTER_K` 24,
`:1682-1720`) ranks the Matryoshka slice then re-ranks the survivors
full-width — and it is **off in every constructor** (`two_stage:
AtomicBool::new(false)` at `:2597`, `:2701`, `:2744`) with no consumer
outside `memory.rs`. The KI-252 measurement gate has not flipped it;
the single full-width pass remains the contract.

## Assembly and the turn path

`assemble_within_shape` (`memory.rs:3515`) adds, in order: the
protected tier (all of it, never dropped — `:3540`), the primary pool
(strongest always enters — the fork's `renderBlock` rule, `:3549`),
the marked spillover by its route, the caller's working facts, then the
graph neighbours — each stage budget-bounded except the first, with
every drop named in `Assembly.dropped`. The budget is
`ASSEMBLY_TOKEN_BUDGET = 2048` (`:3416`), estimated at 4 chars/token
(`estimate_tokens`, `:3665`). A line renders sign-first
(`recalled_line`, `:3688`): `resolved:` / `diverged:` / `incomplete:`,
the score, the basis, the off-shape and via markers, and the missing
companions — an anti-vector can never be read as a candidate.

`AgentOptions::memory` (`agent.rs:546`) arms the turn path.
`prepare_memory` (`agent.rs:2564`) assembles once against the opening
task, records one `Record::Memory` (the shape and every fact name, in
block order — hard rule 5's reconstructability), arms the pending hits
(KI-068: they fire only after the first successful model response, so a
run that never answers records nothing), and seeds the curation base
with the task's own vector. Since the design-board and desk work, the
same block also carries `## Design` and `## Desk` sections (K6, the
desk plan's C13) — independent of the corpus, and a seam the memory
feature doc does not name yet.

**Mid-run curation** rides the steer path: `apply_steer` →
`maybe_curate` (`agent.rs:3926`). The cosine against the last curated
instruction compares under `DRIFT_FLOOR = 0.78` (`:210`, measured
2026-10-30 on steer-shaped pairs); the sys1 `curate` judgment — a
three-judge panel (glide, drex, jev) fanned out in one classify call —
votes beside it under `CURATE_GATE = 0.6`, and
`CURATE_JUDGED_DECIDES = true` (`:246`) gives the panel the deciding
seat, per the owner's 2026-10-30 award; the rollback is one constant.
The verdict caches in the route cache under the pair's hash; every
judged decision writes one calib `curate` row; the first steer always
curates; the base advances to each steer so a slow walk accumulates; a
lane-named focus label that the shape vocabulary carries becomes the
re-curation's shape hint. The curated block rides the message tail —
never inside the system prompt, whose byte-stability the provider
prefix cache keys on — and re-arms its own hits once.

**The model's tools**: `remember` (`tools.rs:1358`) — its description
is the slice-19 instruction's home (the phrases that resolved, the
failure's diverging trajectory, the incomplete's companions); `supersede`
(`:65`); and `recall` (`:1323`), a dry-run read that records no hit.
The tool set is `AgentOptions`-gated (`agent.rs:2324`), and a
delegate's declared scope can never admit them ("remember, supersede,
lane tools and improve are never admissible" — the `subagent` spec).

## The write paths

One writer serves the tool, the CLI (`fleet remember`, main.rs:1692),
and the face: `Remember::parse` (`memory.rs:4020`) validates the name
as a file stem (lowercase/digits/hyphens, no dotfiles — `:3973`),
requires `basis` on every write and `valid-until` on every working
write, refuses `episodic` (the cycle's tier alone), bounds and
sanitizes the description (control chars stripped, 512 chars, marked
cut) and refuses a body past 64 KiB, and — R4b (`:4105`) — refuses an
`incomplete` fact that names no companions. A taken name is refused:
the modify path is `supersede`, never a silent overwrite.

`Memory::remember` (`:4574`) writes through a temp file + rename and
indexes the fact live (KI-332: the next recall in this same task can
surface it); `index_live`/`forget_live` (`:4612-4631`) keep the in-memory
corpus moving with the disk. `FactEdit` (`:4147`) is the surface's
in-place edit through `rewrite_field` (`:3270`) — the one rewriter,
frontmatter-line-preserving, under the per-fact `.lock` flock (KI-062).

`Memory::supersede` (`:4852`) is the permission model: a confirmed-by
fact refuses without an approval in hand (`:4881`); an inferred one
supersedes on its recorded justification. The modify path writes the
replacement (always `inferred` — a supersession is a proposal),
inherits-or-clears phrases and companions, stamps the old fact, walks
the whole `superseded-by` chain (KI-355, `:4977`), reads the fold back
from disk before reporting done (KI-356, `:5030`), and only then
records the calib row. The forget path expires to the day before
today, appends the retirement note, lands both writes before the row.
A stamp that fails names what landed and what did not; nothing reports
success over a disk that disagrees. `Supersede::parse`'s KI-402 guard
(`:4773`) refuses replacement-shaped arguments with no `replacement` —
the worst failure shape here was retiring the fact the caller meant to
reword.

## The consolidation cycle

`consolidate(root, today, stream)` (`memory.rs:5461`) runs three
actions over the same four candidate filters `preview` reports
(`:5286-5351`) — the face's needs-you queue and the pass cannot drift:

- **compress** — working facts past `valid-until` or
  `WORKING_MAX_AGE_DAYS` (7), folded into the pass's episodic summary
  and stamped `superseded-by` it.
- **supersede** — durable inferred facts recall has never surfaced,
  past `STALE_AFTER_DAYS` (180), folded; a confirmed-by fact in the
  same shape is named and left (`untouched_candidates`), an incomplete
  one is named and kept (`kept_candidates`) — a summary bullet cannot
  carry the missing companions.
- **forget** — an expired fact no summary carries moves to
  `memories/.archive/` under a serial-suffixed name claimed with
  `create_new` (`archive_target`, `:5648` — the collision check and the
  claim are one step).

The ordering is KI-212's: the summary writes first; if the fold cannot
land, nothing is stamped, nothing is recorded, nothing is archived —
every source fact stays live for the next pass. Every action appends
one `consolidate` calib row.

The cycle runs on a timer (`spawn_consolidator`, `:5718` — one pass at
once, then hourly, rows flushed before the sleep per KI-063) from three
long-lived processes: `chat` (`main.rs:450`), the TUI
(`tui/mod.rs:694`), and the face's bridge (`bridge.rs:11413`); on
demand from `fleet memories --consolidate` (`main.rs:1662`); and once
per `fleet hygiene` (`main.rs:3655`, the session-close retention's
memory leg). The face exposes it as `POST /api/memories/consolidate`
(`server.rs:784`) behind the needs-you section.

## The satellite seams

- **The session archive as synthetic facts** (`session_recall.rs`):
  `facts_from_sessions` (`:97`) derives one fact per session log,
  `SessionIndex` (`:432`) caches their vectors under the sessions dir,
  `RecallCounts` (`:674`) is the archive's own feedback loop, and
  `record_hit` (`:791`) writes `recall_hit` rows naming session ids —
  the same row vocabulary, a different store.
- **The face's memory surface** (`bridge.rs:5178` `memory_read`): the
  rail sections (needs you, type words, retired, archive, today's
  recall) over the same filters, with the in-place edit and the
  supersede click as the owner's verbs; the supersede handler
  (`bridge.rs:11413` neighborhood) runs the same module path with the
  approval already in hand.
- **The design-board writer** (`bridge.rs:8737`): the face writes one
  `design-board-<slug>` fact per canvas project through
  `Memory::remember` — a system writer landing confirmed-by facts, with
  slug- and project-keyed phrases so an inherited direction cannot go
  stale on a rename. A delete goes through the real `Supersede` forget
  path with the KI-402-correct argument set.
- **Huddle knowledge** (the C15/C16 ruling, no new store): a room's
  head derives what it recalled from the log's `Memory` records and
  what it produced from approved `remember` results
  (`session_knowledge`, `bridge.rs:5790`) — a refused write is not a
  lesson.
- **The house seed** (`scripts/house-seed.py`): lands the house fact
  pack through the CLI writer — the writer is the only frontmatter
  authority, the collision refusal makes it idempotent — and refuses
  `~/.fleet/memories` by name, because a home corpus holding facts
  shadows every project corpus (the misdirection the house-methodology
  session fixed).

## The corpus today (census, 2026-11-07)

639 facts in the repo corpus. Types: project 401, feedback 196,
reference 17, state-sync 8, user 5, episodic 5, design 4, a literal `—`
on 3. Basis: confirmed-by 217, inferred 422. Shapes: 447 facts carry
none; the tagged 192 split state-sync 154, resolved 52, change 19,
docs 14, deps 8, fail-open 6, build 3, drops-the-row 2, rendering 1,
foreign-root 1. 190 facts carry phrases; 4 carry `missing` entries;
42 carry a `superseded-by` stamp; **zero** carry a `valid-until`.
Recall-count is 0 on 569 (89%); the veterans sit at 577–597 (capped to
history 1.0 at 128). The archive holds 21 files; the vector index is
6.18 MB.

## Findings

1. **The protected tier has outgrown the budget that was sized around
   it — verified.** 201 facts are protected (196 feedback + 5 user);
   rendered through the real `protected_line` shape
   (`[type basis] description`, the module's own 4-chars/token
   estimator) they cost ≈15,065 tokens against
   `ASSEMBLY_TOKEN_BUDGET = 2048`. The assembly adds the whole tier
   and never drops it — that is the tier's contract — so every
   assembled turn in this repo carries ~7× the budget in standing
   facts before any recall, and the budget now governs only the
   recalled and working stages. The 2,048 figure was derived against
   the seed corpus's 7 protected facts (~450 tokens); the transplant
   and three days of session writes grew the tier 29×. Mitigations
   already in place: the tier is name-sorted so the block is
   byte-stable and prefix-cache-friendly, and 128k context makes the
   spend affordable — but the design line "bounds a turn to ~25-30
   facts" no longer describes this corpus. Candidate remedies, for a
   plan to decide, not this dive: a strongest-first cap on the tier
   (with the never-empty rule the recall half already runs), a tier
   split, or a budget sized to the tier. (Inference beyond the
   arithmetic: which remedy is right.)

2. **The corpus is 89% never-recalled — verified, and expected by the
   transplant's own design.** 569 of 639 facts have recall-count 0;
   the history channel is dormant for all but ~70 facts, and ~7
   veterans hold the top of every ranking at the 1.0 ceiling. The
   transplant anticipated exactly this ("veterans hold the top ten
   until the new facts are used"). The near-term consequence: the
   supersede action of the cycle cannot touch them before
   STALE_AFTER_DAYS (180) from their write dates, so the corpus grows
   monotonically for months; the phrase channel (190 facts) and the
   graph's Missing edge (4) are the only direction evidence most of
   the corpus carries. (Inference: whether that dormancy is harmless
   is the measured set's question to answer.)

3. **The failure axis is thin beside the ledger it was drawn from —
   verified.** Ten facts carry failure shapes (fail-open 6,
   drops-the-row 2, rendering 1, foreign-root 1) against a
   known-issues ledger that closed 28 rows in the last session alone.
   The anti-vector shelf — the avoidance half the shape design exists
   to stock — is barely filled, and 52 of the tagged facts carry
   `resolved` (the candidate sign) while only 4 name what they were
   missing. The KI record is not being distilled into facts at anywhere
   near its rate.

4. **`state-sync` concentrates a quarter of the corpus — verified
   count, inferred cause.** 154 facts (24%) carry the tag, 52 of them
   also `resolved`. A state-sync instruction's primary pool can hold
   154 candidates, which dilutes the partition's selectivity for
   exactly the shape the transplant used most. Looks like the
   transplant batches' default tag more than a measured taxonomy; a
   re-tag pass is a candidate, not a ruling.

5. **The working tier has never been used — verified.** Zero facts
   carry a `valid-until`, so the compress action has never had
   material. Session-scoped memory (the tier whose whole point is
   "what was I doing") is not being written by any surface today.

6. **Two small data hygiene notes — verified.** Three facts carry a
   literal `—` as their `type` (they land in the face's
   everything-else section), and the episodic counts disagree by one
   (5 facts typed episodic, 4 tiered episodic) — one summary predates
   the tier field or carries a stray type. Cosmetic; both are
   one-write fixes.

## What holds (verified this session)

The write path's refusals and bounds are exactly as documented; the
KI-402 forget-vs-modify guard, the KI-355 chain fold, and the KI-356
read-back are in place; the probe never blocks and retries on cooldown;
the index is digest- and embedder-keyed with orphan pruning; the
consolidation ordering is fold-first with named failures; the locks are
per-fact and poison-recovered; the root rule is home-first with a
holds-facts guard and the home corpus is empty today; the seed script's
refusal holds; the curation panel is wired, cached, row-recorded, and
one constant from rollback; and the face, the CLI, the tools, and the
seed all write through the one `Remember` validator.

## Not covered here

The calibration stream's read side (`fleet calib`'s grading of
`recall_hit`/`consolidate`/`curate` rows), the eval corpus's use of
memory, and the TUI's memory card — each is a consumer, not the
substrate, and none was read for this dive beyond its entry seam.
