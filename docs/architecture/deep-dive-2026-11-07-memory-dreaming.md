# Deep dive: Anthropic's "The components of agent memory", read against Fleet's — 2026-11-07

Source: [The components of agent memory](https://www.anthropic.com/research/components-of-agent-memory)
(Anthropic Research, published 2026-11-06 — one day before this dive).
Companion to `deep-dive-2026-11-07-memory.md`, which surveyed the
substrate itself; every fleet-side claim here rides that dive's
verified reads, and the article's claims are the article's own,
quoted or summarized from its text. Where this doc infers beyond
either source, it says so.

The article's thesis in one line: agents capture experience in the
moment but can only organize it backward — so a **dreaming** pass
(model-driven reflection over the session logs, offline) is the
mechanism that corrects notes, builds representations, keeps the
injected layer small, and fills the gaps individual sessions leave.
Fleet has the capture half and the janitorial half; the dreaming half
is the one it has not built — and, read closely, the article is mostly
about the half fleet has not built.

## The article's components, in its own shape

- **Capture vs consolidation.** The hippocampus/replay analogy: logs
  are the in-the-moment capture; consolidation replays them offline
  into long-term knowledge (the Letta/UC Berkeley sleep-time compute
  precedent).
- **Memory notes as predictions.** Writing a rule predicts the pattern
  recurs. Their CL-Bench learning loop: record the failure, theorize,
  test the theory, write the rule **with how it is known to be true**,
  warn about discovered traps, correct wrong entries. Model
  generations diverged exactly there — scattered untested guesses,
  then a consolidated reference with checked/unconfirmed marks, then
  confirm-after-grading with corrections, then the closed loop.
- **Salience: write what is not readable elsewhere.** Their Pokémon
  comparison: a model that archived everything (413 files, 225
  folders, continuously stale, spending turns maintaining memory
  instead of playing) lost badly to one that wrote nine files about
  what the screen does not say — strategy, plans, tested rules. A
  fact readable off the environment every turn is redundant; a fact
  that exists only if worked out disappears unless written down.
- **Steering: what memory is for, not how to use it.** General
  guidance beat prescriptive rules; their "record only failures" rule
  *lost* much of memory's measured boost. Freedom to judge salience
  was the winning setting for capable models.
- **Dreaming corrects memory.** The Mt. Moon case: a wrong note
  trusted by a later session cost the whole run. The fix was a pass
  that checks notes against the session transcripts — the transcript
  is the ground truth — and rewrites what disagrees.
- **Dreaming builds representations.** Wikis: linked pages per
  subject, maintained by dreaming, used *with* search. On a 420M-token
  corpus, wiki+search beat search alone (61% vs 53%) and saved ~20k
  tokens per question — but showed no gain on single-document
  retrieval, and one wiki grew larger than the documents it
  abstracted (the Borges map; it abstracted nothing).
- **Dreaming keeps the injected layer small.** Promoted notes are
  injected into every session: they cost tokens, a wrong one rides
  into every session, and longer instruction files reduce adherence.
  Claude Code's own auto memory injects only the first 200 lines /
  25 KB of `MEMORY.md` — an index, not the content. Much of
  dreaming's work is demotion: move details down, leave links.
- **Dreaming fills gaps and focuses growth.** Cross-session passes
  repoint dangling links, merge duplicates, and can be steered by
  instructions about what matters (their contract-terms failure went
  from 72% to 100% by giving each contract its own page, written by
  one agent, quoting terms word-for-word).

## Where the article validates what fleet already built

This is worth saying plainly, because the correspondence is unusually
close — fleet's shape was designed independently and the article's
measured findings land on fleet's side of most of them:

- **Rules with provenance and trap warnings.** Fleet's `basis`
  (confirmed-by / inferred) is the article's "how it knew the rule
  was true"; the failure shapes and the three signs are the trap
  warnings — an anti-vector is a discovered trap, recalled so it is
  avoided; `supersede` with a recorded justification is the
  correction path. The article's highest-performing memory behavior
  (Opus 5's closed loop) is the shape fleet's write path already
  demands.
- **Replay marking.** The article's marked memories replay more; the
  `recall-count` feedback loop is fleet's version — use is the
  chemical marker, and the history channel (0.25) is the replay
  weight.
- **Deterministic summaries.** Fleet's consolidation cycle is
  deliberately no-model-call, which the article's contract-terms
  incident (concurrent agents overwriting a shared table) argues for:
  fleet cannot overwrite itself that way — the per-fact flock and the
  taken-name refusal make concurrent writes named refusals.
- **Bounded abstraction.** The Borges hazard (a representation larger
  than what it abstracts) is structurally hard for fleet: a fact is
  one 512-char description plus a bounded body, and the assembled
  block carries descriptions only.

## The gaps, as improvement areas

Each area below names the article's evidence, fleet's current state
(with the deep dive's file:line), and the candidate direction. These
are dive findings, not decisions — several touch standing owner
rulings, and the doc says where.

### 1. Dreaming itself is the missing half

The article's core mechanism has no fleet counterpart. Fleet's
consolidation cycle (`memory.rs:5461`) is deterministic janitorial
work — compress expired working facts, fold never-recalled inferred
ones, archive the expired — and `memory.rs` contains no read of any
session transcript (verified: no `replay`/transcript path in the
module). Nothing checks a fact against the evidence that produced it;
nothing merges duplicates; nothing notices a pattern across sessions;
nothing populates the graph's edges from what actually happened.

Fleet's prerequisites are all present and unusually strong:

- **The ground truth exists on disk.** Hard rule 5 (model-visible ⟺
  logged) means every run's full transcript is replayable from the
  session archive — the article's validation source, already a fleet
  invariant.
- **The correction path is pre-authorized.** Slice 12's guardrail
  already rules that the self-maintenance loop "may supersede an
  inferred fact on its own justification, and a confirmed-by one
  waits for approval" (`Memory::supersede`, `memory.rs:4881`). A
  dreaming pass that proposes corrections through `supersede` needs
  no new permission model — only a runner.
- **The scheduler seam exists.** `spawn_consolidator`
  (`memory.rs:5718`) is a detached hourly thread in all three
  long-lived processes; a dreaming pass is the same seam with a model
  call in it.
- **The proposal gate exists.** `improve` files proposals that never
  self-apply (`improve.rs`, `fleet improve apply` is the owner's,
  through the gate) — the safe landing for dream-generated *new*
  facts, as opposed to dream-generated *corrections*, which
  `supersede`'s existing permission already covers.

The candidate shape: a dream pass that (a) re-checks `inferred` facts
against the sessions that produced them, (b) repoints or proposes
corrections through the existing supersede/proposal paths, (c)
populates the graph's `missing`/vouch edges from observed
trajectories, and (d) distills recurring cross-session patterns into
gated proposals. What a plan must decide: the model seam (which lane
judges — sys1 is the house's judgment lane, blend1 the grounded
writer), the cadence and budget, and whether corrections auto-land on
inferred facts (the slice-12 reading says yes) or always queue as
proposals.

**The validation movement, concretely (2026-11-07, the owner's
steer).** Sys1 is the judge; the transcript is the ground truth it
judges against — validation in the abstract is a plausibility vote
and makes nothing trustworthy (the article's Mt. Moon note was wrong
from the start and read as plausible; only the transcript catches
that). The shape follows the `curate` task's own pattern: a `validate`
task_spec, a fact-and-transcript pair on the wire, a verdict of
`supported` / `contradicted` / `unverifiable`, cached in the route
cache keyed to the pair, the panel and confidence gate it earns if a
single judge proves thin. The failure direction is fail-safe by
construction: validation is an upgrade, so a lane that cannot answer
— no keys, dead lane, unparseable verdict — leaves the fact `inferred`
and nothing is lost. The queue is not the corpus: 422 facts are
inferred, and the `recall_hit` rows already rank which of them the
model actually sees — validate the surfaced first, the never-touched
tail last or never. And the measured-set gate before any auto-upgrade
lands: run the validator against facts the ledger already proves
wrong, the same way the curate seat was awarded on its replay record
with a one-constant rollback.

**The basis upgrade: `validated`, the third value.** A sys1 pass must
not stamp `confirmed-by` — that word means the owner stated or
verified it, it is the permission model's input (protected tier,
supersede needs approval, the cycle leaves it, provenance ranks it
1.0), and machine validation writing it would launder a model's
judgment into owner authority. The honest record is a third value on
the basis axis: `validated` — evidence-checked by the lane, no owner
authority claimed. The ladder reads: `inferred` (a guess) <
`validated` (checked against its evidence) < `confirmed-by` (the
owner stood behind it); the provenance channel weights it between
0.5 and 1.0, a plan's arithmetic to set. **Why not `judged`:** the
word is taken — the route-judgment work landed `judged` as the plan
vocabulary's status with `judge_plan`/`plan_judge` as sys1 task
names, and one word, one meaning is the house's own nomenclature
rule. The division of labor the grammar wants: the *act* is the calib
row's op (`validate`), the *verdict* is the judgment's vocabulary,
and the basis value names the fact's standing afterwards.
`Basis::parse` (`memory.rs:207`) currently refuses any third value —
a malformed fact — so this is a schema change the plan carries, with
two rulings beside it: does a `validated` fact still supersede on
justification alone (the slice-12 rule for inferred; the
recommendation here: yes, when the justification names the new
evidence), and does the cycle treat it like confirmed-by (leave it)
or inferred (fold it stale; the recommendation: leave it, like the
provenance it has partly earned).

### 2. The injected layer must stay small — external evidence for the deep dive's finding 1

The article's promotion section is, almost verbatim, an argument
against the protected tier's current size: injected notes "use up
valuable tokens", "an incorrect note can be carried into every
session", "loading a note also doesn't guarantee that Claude acts on
it", "longer instruction files reduce adherence", and the top layer
"has to stay relatively small" — Claude Code holds it to 200 lines /
25 KB *of index*. Fleet injects 201 protected facts at ≈15k tokens of
content against a 2,048-token budget (the deep dive's finding 1).

The article sharpens the remedy: the injected layer should be an
**index**, with dreaming constantly demoting detail out of it and
leaving links behind. Fleet's descriptions *are* the content already,
and bodies are read on demand — so the structural move is not a format
change but a size discipline: a cap with the strongest-first rule the
recall half already runs, and/or dream-time demotion of protected
facts whose standing has decayed (the cycle today names confirmed-by
facts untouched by design — `untouched_candidates`, `memory.rs:5320` —
so the cap, not the cycle, is where this lands). What a plan must
decide: whether the tier's "never evicted" guarantee gets a size
corollary, and what happens to the overflow (an overflow section,
on-demand reads, or demotion proposals).

### 3. Facts carry no evidence pointer, so correction has nothing to check against

`Remember` (`memory.rs:3994`) carries basis but no source: nothing
says *which session* produced a fact. The article's dreaming works
because the note can be checked against its transcript; fleet's facts
cannot be cheaply checked because the evidence link was never
recorded. This pointer is the prerequisite for area 1's validation
movement — with it, a `validate` judgment is "load the named session,
replay, judge the pair"; without it, the pass must *find* the
transcript first, which is where wrong sessions get matched to right
facts. The candidate is small and mechanism-shaped: an optional
`source` frontmatter field (a session id or archive pointer) on the
write path — the model filling it is steering, not prose, and the
`recall_hit`/`curate` calib rows already demonstrate the
row-names-its-evidence pattern. This one field is what turns area 1's
validation from "search the whole archive" into "read the named
session". It also serves the feature doc's own recorded limitation
(the phrases can be gamed; "a future slice could cross-check a
fact's phrases against the recall hits it earns").

### 4. Capture outruns use — the Sonnet 3.7 trajectory, measured

The article's salience finding is fleet's census: the model that wrote
everything spent its turns maintaining memory; the model that wrote
nine files won. Fleet: 639 facts in ~3 days, 89% never recalled, 401
of them `project`-typed, many recording state the environment
re-derives (git history, the ledger, the plans themselves). The
article's rule of thumb maps directly: **memory is for what the
environment does not say back** — a fact readable off `git log` or
`docs/plans/` is redundant.

Two candidate directions, both steering-shaped rather than
mechanism-shaped: the `remember` tool's description (the steering
surface fleet already maintains) gains the "not readable elsewhere"
test; and a dream pass proposes retirement of re-derivable facts
(through supersede, on justification). One honest tension to carry:
the article also measured that "record only failures" *lost* the
boost — so fleet's failure-axis emphasis must not become a
write-suppressing rule; the aim is a salience bar, not a narrower
taxonomy.

### 5. The graph is the wiki seam, but the edges never arrived

The article's wikis are model-built and model-maintained link maps;
fleet's recall graph (`recall_graph.rs`) is the same idea, but the
edges ride the frontmatter and the corpus declares almost none — 4
`missing` edges and 42 `superseded-by` stamps across 639 facts, with
`GRAPH_SOURCES = 3` / `GRAPH_HOPS = 1` waiting on material. The
deep-dive census's state-sync concentration (154 facts, 24%) makes a
hub-and-link shape attractive: a subject-hub fact per recurring theme,
its `missing` field naming the companion facts, maintained by dreaming
rather than by hand. The Borges bound is fleet's own: hubs abstract
(their descriptions are the 512-char content), they do not transcribe.

### 6. Cadence by access — the article's open question, fleet's rows already answer it

The article leaves dreaming's cadence open and suggests tuning it by
"how often memories are accessed". Fleet's calibration stream records
one `recall_hit` row per surfaced fact with its score — the access
log exists. A dreaming scheduler can spend its budget where access is
dense (frequently-surfaced facts are the ones a wrong note poisons
hardest) and skip the never-touched tail the cycle already owns. The
hourly consolidator thread is the natural host; the risk to bound is
cost, which the route cache's decision-keys gate (an unconfigured
machine pays nothing) already answers for the curate judgment.

### 7. Close the learning loop into the corpus

The article's CL-Bench loop ends with a rule in memory. Fleet's
grading beats — `fleet judge`, `fleet delegate`, the eval corpus — end
in calib rows, not in the corpus: the verdicts are recorded but
nothing turns a graded failure into a fact correction. The seam is
the improve proposal (gated, owner-applied) for new rules and the
slice-12 supersede for corrections; the missing piece is a step that
reads a day's grading rows and drafts the proposals. That is
dreaming again — area 1 with the calib stream as its input instead of
the raw transcripts.

## The owner-ruling tensions, named

Two standing rulings bound these areas, and neither is this dive's to
relax:

- **"No model-written memory prose: the write path is
  mechanism-constrained and the cycle's summaries are
  deterministic"** (`docs/features/memory.md`). Dreaming's corrections
  and proposals are model-written. The existing machinery suggests the
  house shape: corrections ride `supersede` (mechanism-constrained,
  justification required, provenance-gated) and new facts ride
  `improve` proposals (never self-applying) — the model proposes,
  the mechanisms and the owner decide. Whether auto-landing on
  inferred facts fits the ruling is the plan's first question.
- **The mechanism-constrained write path is itself prescriptive**
  where the article found freedom to judge salience outperformed
  fixed rules. Fleet's tool asks for structure (shape, phrases,
  missing, basis) — that structure is load-bearing downstream (the
  partition, the signs, the permission model), so the article's
  finding does not argue it away; it argues for watching the write
  rate and the recall coverage as the measure of whether the
  friction suppresses salient writes the schema has no home for.
- **The basis vocabulary is the permission model's input, so the
  `validated` value is a schema and ruling change, not a stamp.**
  `Basis::parse` refuses a third value today; the provenance
  channel's weight for it, the supersede rule it earns, and the
  cycle's treatment of it are three rulings the plan must carry (the
  recommendations are in area 1). Until they land, machine judgment
  can grade and propose, but the corpus's provenance ladder stays
  two-rung.

## What this dive does not settle

Whether dreaming pays for itself at fleet's corpus scale (the
article's gains are measured on 420M-token corpora and long-horizon
games); the tier-cap arithmetic's owner preference; and whether the
working tier's disuse (census finding 5) is a defect or a fact of how
fleet is used. The `validated` naming and the validator's shape are
recommended here, not ruled; the two supersede-and-cycle rulings it
carries remain the owner's. Each is a plan's question; the two with
weight are the dreaming pass (area 1) and the injected-layer cap
(area 2), and area 3's evidence pointer is the small unblock that
makes the first one cheap.
