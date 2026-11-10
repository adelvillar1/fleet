# Deep dive: Princeton/Meta's Strategist, read against Fleet's self-improvement loop — 2026-11-10

Source: [*Strategist: Evolving Reasoning with Agents*](https://arxiv.org/html/2611.03618v1)
(Lu, Xu, Hua, Liu — Princeton University and Meta, arXiv 2611.03618v1).
Companion to `deep-dive-2026-11-10-ernie-evolve.md` (the same day's
first dive) and `deep-dive-2026-11-07-self-improvement.md`; every
fleet-side claim was read at its file:line this session, and the
paper's claims are the article's own — section references are its own
numbering. Where this doc infers beyond either source, it says so.

The paper's thesis in one line: an AI system is half model, half
**inference strategy** — the reasoning procedure that turns model calls
into a response — and that second half is written by hand and never
improves; make it *cumulative* instead of per-task: strategies are
executable blueprints, a shared library compounds every task's search
into the next task's starting point ("The part of an AI system that is
not improved on its own now does"). Fleet self-improves a different
half — the harness itself — so the comparison is not strategy-vs-
strategy but *loop-vs-loop*: how their search accumulates versus how
fleet's propose-gate-lesson loop accumulates, and what fleet's loop is
missing that their ablation just priced.

## The paper's loop, in its own shape

- **The Strategy Blueprint** (§2): an executable Python program with
  one shared interface — `execute(llm, task) -> str` — that captures a
  reasoning flow *without* task prompts; built from two foundations
  (`chat`, `agent`), routed across three model tiers × two modes
  (fast/balanced/complex × nothink/thinking). Four protocols govern
  the library graph: shared interface, no forward calls, no cycles, no
  duplicates. Because every blueprint can be a component inside any
  other, the library compounds by composition depth.
- **The library**: blueprints plus **measurement cards** (measured
  accuracy and cost, written after first evaluation) plus
  **experience notes** ("when it works and when it does not") — all
  three indexed for retrieval. 1,865 blueprints after 30 searches
  (A.4).
- **The four-step pipeline** (§3, Algorithm 1): **Retrieve** — two
  textual queries per iteration, a *relevance* query and a deliberate-
  drift *inspiration* query, scored hybrid TF–IDF + sentence-embedding
  (α = 0.4), top-5 each = 10 blueprints seen (full cards for relevance
  hits, one-liners for inspiration); **Propose** — K = 5 candidates,
  every slate forced to include ≥1 of each kind (new / composed /
  reuse) and ≥1 of each target (accuracy / efficiency / balanced),
  invalid slates rejected and the meta-agent re-prompted; **Evaluate**
  — four disjoint splits: *training* (strategist sees full traces),
  *validation* (aggregate scores only), *hidden* (never shown; final
  selection under a 2%-rule: win = >2 accuracy points or >2% cost,
  worse on neither), *test* (held out entirely, the only unbiased
  number); **Accumulate** — the library stores blueprints *not*
  strategies (task prompts discarded → cross-task transfer), each with
  analysis, card, and notes, deduplicated by AST hash over the
  `execute` body with string literals elided.
- **The proposal's honesty checks** (A.4): six required JSON fields
  (`name`, `kind`, `target`, `tier_mode_anchors`, `why`, `builds_on`)
  plus search-side fields (`search_queries`, `read_cards`,
  `experience`); a non-fatal AST audit verifies `builds_on` matches
  the code's imports — **warnings are fed into the next iteration's
  prompt rather than killing the run**.
- **Results** (§5): 24 wins of 30 benchmarks, 74.7% vs the strongest
  baseline's 74.0% at 39% lower weighted cost; the library alone
  (one-shot retrieval, no search) lands within 2 points of the
  baseline on all 10 held-out benchmarks and above it on 8
  (77.8% → 81.4%); discovered strategies transfer unchanged to GPT-5
  and Gemini-2.5 (wins 7/10 and 8/10, −35%/−40% USD).
- **The ablations that price the loop** (§7): zero-shot (no library,
  one iteration) 39.4% vs 78.7% full — and crucially, *iteration
  without the library* scores 65.4%: the loop still runs, but
  "without inspirations, it goes on to a narrow repetition." The
  library, not the iteration, carries the value.
- **The honesty section** (§6): training–hidden gap stays 1.2–2.5
  points over iterations (no drift into noise), and selection luck is
  stated outright — the hidden-set pick sits +4.3 over baseline there
  but only +0.7 on the untouched test split, so every table reports
  test.

## Where fleet already lives the article's principles

- **The loop's shape, at proposal granularity.** Retrieve = the
  filing confirmation hands the last five lessons for the target kind
  (`lessons_for`, `src/improve.rs:745`, appended at `:1141`); propose
  = the `improve` tool, evidence-required and owner-gated; evaluate =
  the four-leg gate plus the deterministic corpus step for source
  edits (`src/improve.rs:1775`); accumulate = one lesson per applied
  change with its evidence and a `measured|unmeasured` confidence
  (`Lesson::from_proposal`, `src/improve.rs:630`), rendered as the
  capability library (`src/improve.rs:3019`). The article's
  retrieve-propose-evaluate-accumulate runs across fleet's sessions as
  one long loop instead of ten iterations of five.
- **A measured reference in the proposer's prompt.** Their strategist
  is shown the full baseline matrix with BL highlighted and the 2%
  rule before proposing (A.4); fleet's `improve` tool declaration
  orders the same discipline — "`fleet improve analyze` FIRST: it
  measures the graded rows … so the acceptance criteria carry measured
  thresholds, not guessed ones" (`src/tools.rs:1747`) — with
  `analyze` itself reading the stream per class (`src/improve.rs:3091`).
- **Rejection that does not kill the run.** Invalid slates re-prompt
  their meta-agent (A.4); fleet's invalid filing is a named refusal
  and the session continues (`parse_change`, `src/improve.rs:363`,
  name-uniqueness at `:1091`), and their AST `builds_on` audit has a
  fleet twin in the drift guard: a target that moved since the draft
  is refused by name (`src/improve.rs:1920`) over content shas stamped
  at filing.
- **Structural dedup.** Their AST hash with string literals elided
  matches fleet's artifact identity — the proposal's `artifact_sha` is
  a hash over the change with prompts-shaped text normalized
  (`src/improve.rs:341`) — same instinct: duplicates are detected on
  structure, not on prose.
- **The library's growth path has a second engine.** The clustering
  beat groups a period's failures and files shape proposals
  (`src/demand.rs:369`, `:617`) — fleet's accumulate step firing
  *without* a proposal ever having been written, feeding the same
  library their pipeline feeds.
- **The one strategy layer that already learns.** The article's target
  layer — how models are called — is the half fleet already improves
  without a new plan: fit cells may move the failover's preference
  (`src/calib.rs:2965`), the judge beat grades the routing decisions,
  and `route::select` walks the learned chain. The route preference
  table is fleet's answer to "the part that is not improved on its
  own": it is improved, on the calibration stream.
- **The retrieval engine is already ahead of theirs.** Their hybrid is
  TF–IDF (α = 0.4) + MiniLM cosine over card text (A.4); fleet ships
  BM25 (`src/lexical.rs:18`), RRF at K = 60 (`src/fusion.rs:12`), and
  a probed vector path behind one `search` (`src/hybrid.rs:224`).
  The gap below is not the engine — it is which store the engine is
  allowed to read.

## The gaps, as improvement areas

Each area names the article's finding, fleet's current state with
this session's reads, and the candidate direction. These are dive
findings, not decisions.

### 1. The lesson store is filtered, not searched — and has no inspiration query

Their ablation is the loudest number in the paper: iterate *without*
the library and you lose 13.3 points — not because iteration is
useless, but because "without inspirations, it goes on to a narrow
repetition" (§7). Fleet's library is real, but its read is a filter:
`lessons_for` selects by target kind and returns the five newest
(`src/improve.rs:745`); nothing queries lessons by content. The
lessons do not even live where the good retrieval lives — recall
indexes the `memories/` fact corpus (`src/memory.rs:1-12`), while
lessons sit in `<home>/lessons/` (`src/improve.rs:593`) outside it.
So the two-query pattern — one *relevance* query, one *deliberate-
drift inspiration* query ("multi-path verification on long-context
QA" asked of a short-form task) — has no home, even though fleet's
engine could out-score theirs on day one. Candidate: give the lesson
(and capability) store a query-time read through the existing hybrid
machinery — indexed on the trio they index (name/cue/lesson as the
structural fields, card and evidence as the behavioral fields) — and
put *both* query results into the filing confirmation where
`lessons_for`'s five already appear (`src/improve.rs:1141`), the
inspiration hits fetched cross-target on purpose. Note the boundary
this must respect: read-time search over `<home>/lessons/`, not
writing lessons into the git-tracked `memories/` substrate — the
provenance contract belongs to facts, and a lesson is not a fact.

### 2. The card carries confidence but no measurements — and is never revised

Their card is written after first evaluation and carries the measured
accuracy and cost (A.4), and it is what the retrieval ranks on. Fleet's
lesson carries `confidence: measured|unmeasured` where *measured* means
"the corpus was green at apply" — a boolean about the gate, not a
number, and nothing ever rewrites it (`Lesson::from_proposal`,
`src/improve.rs:630`). The ERNIE dive's area 5 already queued the
missing loop — grade an applied proposal by the row that followed it,
`grade_judgments`-style (`src/calib.rs:1085`); this paper adds the
**destination shape** for that readback: the settlement should write
*into* the card as measured numbers plus "when it works / when it does
not," and the card must become part of the retrieval index (area 1),
so later filings rank on behavior rather than prose. Borrow their
honesty rule with it: report the untouched number, keep selection
signal separate from teaching signal — their A.3 keeps experience
notes derived from training traces only, and their §6 states the
unbiased test figure because the hidden-set pick flatters by 3.6
points. When fleet grades proposals by outcomes, the same discipline
applies: the class-trend readback is the teaching signal; the unbiased
number is what gets quoted.

### 3. Failures never reach the next attempt — now argued by two articles

Their AST-audit warnings are **fed into the next iteration's prompt
rather than killing the run** (A.4), and a violating slate is re-
prompted: failure is context for the next proposal, by construction.
Fleet's refusals ride back only into their own proposal's file
(`last_refusal` stamped at `src/improve.rs:1875`, shown on that card
at `:2670`), and lessons — the thing the *next* filing is handed — are
written only on an applied verdict (the `Applied` arm of `finish`,
`src/improve.rs:1835`). A fresh proposal targeting a file that has
already refused twice is handed applied history and nothing else. The
ERNIE dive already queued this as its area 3; a second independent
source, whose mechanism is precisely "warnings flow forward," makes
the fix the same one: the filing confirmation lists recent refusals
for the target beside the lessons, in the same place `lessons_for`
already runs. Same seam, one store read more.

### 4. Search-side fields are enforced; fleet's analysis-first is prose

Their proposal must carry `search_queries`, `read_cards`, and
`experience` — what the meta-agent *read* is part of the submission,
schema-checked (A.4). Fleet's equivalent discipline is a tool
description: analyze first (`src/tools.rs:1747`) — an instruction no
machine checks, and the filing confirmation reports lessons read but
no analysis performed. Candidate shape (cheap, structural): the
filing path already writes a `propose` row with the artifact sha —
have it also record whether an `analyze` row for a nearby window
exists, and surface "no analysis on record" on the filing reply
rather than refusing (fleet's refusals are named, not preventive
here). The deeper version — refusing evidence-free filings — is
already half-built: the lesson's evidence rule refuses a lesson
without justification (`src/improve.rs:630`); whether filing itself
should carry the same requirement for `basis: inferred` is a question
this dive does not settle.

## What fleet has that the article doesn't

- **A deterministic scorer.** Their four splits, 2%-rule, and selection-
  luck accounting all exist because their dev sets are noisy samples.
  Fleet's corpus is scripted and exact: at gate time, strict
  inequality is a perfect threshold (the race's `applied > twin`,
  `src/improve.rs:2837`, and the demand trend's ε = 0.05,
  `src/demand.rs:40`, are the two places noise actually exists —
  stream rates and resource readings, both already banded).
- **The placebo control.** They ablate the *library*; they never ask
  whether one candidate's gain is the candidate or the circumstance.
  Fleet races every candidate against a twin and a sham
  (`src/improve.rs:2862`) — a per-candidate causal test the paper has
  no counterpart for.
- **Provenance on what is remembered.** Their notes are free text;
  fleet's memory facts carry `basis`, supersession edges, and the
  confirmed-by permission model — and the just-drafted
  `improvement-modes` plan keeps that rule non-negotiable under every
  mode (its C3), where their loop ships `s*` automatically.
- **Human-gated landing.** They select and return the winner without a
  human in the loop (Algorithm 1); fleet's ruling — the tool files,
  the mode or the owner applies — is the same stance the ERNIE dive
  recorded, and this paper's fully-automatic `s*` selection is exactly
  what fleet's modes deliberately do not become.
- **Retrieval engine.** BM25 + RRF-60 + probed vectors vs TF–IDF +
  MiniLM at α = 0.4 — fleet's is the stronger instrument (see area 1:
  the gap is the store it reads).

## Scale, named as non-adoption

- **The K = 5 × N = 10 slate search.** Their loop needs a fixed dev
  set, a scorer, and ~20,000 candidate-sample executions per benchmark
  (A.4) because the task is fixed and answer-checked. Fleet's "task"
  is an open-ended session; its corpus is a regression gate, not an
  optimization surface. Evidence-driven filing is fleet's answer to
  "propose"; quotas (≥1 new/composed/reuse per round) would file
  proposals nobody evidenced. Not a gap — a different loop shape.
- **The blueprint graph as executable strategy.** Fleet's strategy
  layer is the compiled harness itself — routing, lanes, roles,
  compaction, the tool loop — plus skills; synthesizing a per-task
  Python strategy would displace the architecture rather than improve
  it (hard rule 6: native port, not a fork of a strategy runtime).
  The four protocols' kernel (composition without cycles or
  duplicates) already exists where fleet composes — knowledge edges
  (`recall_graph`) and the skills cap — and needs no redesign here.
- **Per-step tier routing inside one strategy.** Their cost engine
  mixes fast/balanced/complex per step because three frozen small
  models price like tiered FLOPs (tier ratios 1 / 2.83 / 6.67); fleet
  routes whole decisions over hosted providers with the resource
  contract pricing the machine itself. Different economics, already
  measured — no absorption.
- **The four-way split machinery.** Borrow its discipline (area 2's
  note), not its machinery: fleet has one fixed corpus today; when
  settlement lands, the *separation of signals* is the transferable
  part.

## What this dive does not settle

Whether the lesson store's query-time read should live in `demand`/
`improve` seams or a new read function beside `lessons_for` (and
whether capabilities ride the same index); whether measurement cards
ship first with apply-time numbers or wait for the settlement
readback (ERNNIE area 5) — they are one work item or two, and the
order is a plan's question; whether filing-time analysis should be
*recorded* (area 4's soft form) or *required* (the hard form, which
changes the tool's contract); whether fleet's actual proposal volume
justifies the inspiration-drift query at all — their library amortizes
over 30 tasks while fleet files rarely, so area 1 should ship measured
(hits actually used) rather than assumed; and how the improvement-
modes draft plan's attempt seam interacts with areas 1–2 if autonomous
mode starts filing-and-applying faster than lessons accumulate — the
loop's cadence may become binding before its retrieval does.
