# Slice 20 — the delegated vector: fleet's delegation learns its own direction (2026-10-14)

status: planned

## Goal

Close the five gaps between fleet's delegation and the discipline that
built it. Fleet already carries the core of that discipline — fresh
context, the same model structurally, the parent's approver, a depth
cap, never silence, and the self-report warning in the tool's own
description (src/subagents.rs:10-30, src/tools.rs's `subagent_spec`).
What it lacks is everything that makes delegation *learn*: a structured
brief, a stream that records what each brief actually drove, a judge
that grades it, roles to specialize the child, concurrency to fan out,
control once a child is running, the depth to delegate twice, and the
project's own vocabulary to name things the way the project already
names them.

This slice lands all seven, ordered by dependency: **the brief (D1),
the loop (D2), the roles (D3), the fan-out (D4), the control (D5), the
n+2 depth (D6), and the nomenclature guard (D7).** The loop is the
heart; D1 makes its data gradeable, D3 gives it an actor dimension, D4
and D5 are the mechanics the loop then measures, D6 is the one place
the loop's own framing bites back — a brief at depth 2 is a paraphrase
of a paraphrase, so the parent chain that produced it has to be
recorded from the first row rather than reconstructed later — and D7 is
the part that pays for the rest, because drift caught at write time
costs one pass while drift caught in review costs a fix round.

## The formal framing (the owner's, 2026-10-14)

A delegation brief is a **vector**. Its proximity is how similar it is
to briefs that have been written before; its direction is whether that
brief's *shape* drove toward verified-complete work. The owner's law
governs: "you cannot steer towards a direction if you don't have
historical data. but you can always record the suggested result and
what the other lanes record as the right outcome. for instance if a
solution with a given set of directions is validated, then the set of
vectors belonging to that solution gain a boost in trajectory
shaping." The log comes first, the loop closes later.

The three vector kinds apply unchanged: a brief whose child reported an
artifact its own log never wrote is **wrong** — it drove away, and
reusing that shape drives away again. A brief whose child completed the
stated task but whose parent's session shows the work continuing on the
same paths is **insufficient** — the direction was right and the minima
was unreached for want of a companion (a missing constraint, a missing
path, a missing definition of done, a longer turn budget), and each
insufficient reason NAMES the absent companion. A brief whose task
completed and whose claim verified against the child's log is
**right**. And the owner's recall ruling applies to the brief's
retrieval too: a brief that is semantically near a past brief but
carries no direction evidence must not outrank one whose shape
demonstrably drove.

**Nomenclature is the case where proximity is distrusted outright.** A
child naming something in a fresh context generates a plausible name
from its own priors — a near-synonym with high semantic density and no
direction, which is the drift D7 exists to catch. The ruling's
corollary: for naming, *direction means the term is established in this
tree*, and a name that means the right thing and appears nowhere must
not be surfaced as canonical. That is why the lookup is lexical and
evidence-bearing rather than semantic: a vector search returns the
nearest term, and the nearest term to a concept is usually the synonym
that is the drift.

## The problem shape (PS-E, the delegation half)

"**Which brief shapes actually drive toward done, and which only look
like they do?**" The naive path delegates and hopes: every brief is
written from the parent's intuition, every child's summary is believed
because it reads well, and nothing records whether the shape worked. A
confident summary with a high semantic density and no verified outcome
is the exact failure the owner named for recall — it gets reused
because nothing pays attention to its direction.

## The honest baseline (verified against the tree, 2026-10-14)

- Fresh context, the same routed client with no per-call model
  parameter, the parent's approver labelled with the goal, depth 1
  enforced structurally, and a failure returning the partial result
  plus the typed reason plus the child's log path: all landed
  (src/subagents.rs).
- The self-report warning is in the tool description: "The summary is
  the subagent's own report, not evidence: verify any external side
  effect it claims."
- **The gaps**, each with its evidence: (1) the brief is a goal string
  only — no `context`, no output contract (src/tools.rs's
  `subagent_spec`); (2) nothing grades a delegation's outcome — the
  child's log lands (`SubagentResult.log`) and is never read; (3)
  `Scope` is a tool list, not a role — the child takes the parent's
  compiled-in system prompt; (4) `src/agent.rs:873` executes tool calls
  serially (`for call in &message.tool_calls`), so no fan-out; (5) a
  running child cannot be steered or stopped; (6) the chain stops at
  one level — `SUBAGENT_MAX_DEPTH` reads 1 and the `subagent` tool is
  outside a child's scope (src/subagents.rs:60, :69-70), so no row
  anywhere records which brief produced which child; (7) nothing
  retrieves the project's canonical vocabulary at the moment of naming,
  so a fresh context invents a near-synonym — the drift caught in
  review rather than at write time (KI-137, KI-151, KI-152, and the
  parent's own `"r#match"`).

## Acceptance criteria

### D1 — the structured brief

- The `subagent` tool gains two optional arguments: **`context`** (the
  background the child needs and cannot see) and **`output`** (the
  shape of the answer the parent will accept — the definition of done).
- The child's first user message composes the three parts between
  explicit markers (`TASK`, `CONTEXT`, `WHAT DONE LOOKS LIKE`), so a
  non-conforming model cannot read the background as the task.
- **Backward compatible**: an absent `context`/`output` composes
  exactly today's message; every existing caller and test is unchanged.
- The description states the contract in the tool's own words, and the
  approval prompt and the progress line render the brief's three parts
  (bounded, never a raw dump).
- The brief's shape is recorded on the delegation row D2 reads.

### D2 — the delegation stream, the judge, and the calibration surface

- A new op (`delegation_judged`) keyed by the child's session id,
  carrying the brief's shape, the scope, the outcome, the turns, the
  spend, the duration, the child's log path, **and the parent chain** —
  the immediate parent's session id and the root's, beside the depth —
  redacted by design, the summary text NOT among the evidence. The
  chain is what makes a depth-2 row joinable to the brief that
  actually set its direction (D6), and it is recorded from the first
  row because a chain cannot be backfilled honestly.
- **`fleet delegate [--day <date>] [--dry-run]`**: batch-only, one
  bounded ask per delegation row, judging it against its outcome
  evidence through the vendored sys1 lane — the ported discipline
  verbatim: no turn's synchronous path references the beat (pinned by a
  socket-guard test in slice 18's shape), a dry run makes no ask, a
  named daily budget (`DELEGATE_BUDGET_USD_DAY`, config can only
  lower), the spend sidecar lands even when the row write fails, and a
  lane that fails open is a named refusal — no row, no influence, the
  recorded decision standing (the consuming-lanes rule).
- **The exit-code rule, contractual**: a batch that obtained no
  judgment (every row refused, an empty day, an over-budget day) exits
  1; a dry run, a judging batch, and an all-already-judged batch exit
  0, so a twice-daily cron is not a false alarm.
- **The three-way verdict**, with delegation-specific signals:
  - *wrong* — the child's claimed artifact or command is absent from
    its own session log (a claim the log contradicts);
  - *insufficient* — the child completed its stated task but the
    parent's session shows the work continuing on the same paths (the
    brief was missing a companion), or the child stopped at max-turns
    with its direction right (a longer turn budget was the companion).
    Each reason NAMES the absent companion;
  - *right* — the task completed and the claim verified against the
    log.
  Wrong outranks insufficient; the two negative states are never
  folded. The verdict's confidence and reason ride the row.
- **`fleet calib --delegation`** renders per-(provider × brief-shape)
  cells at the house bar — cards with status dots, the three rates kept
  apart, the fit state (slice 18's thresholds ported), what an unfit
  cell still needs — never the average.
- **The human seam**: `fleet delegate --agree/--disagree/--waive
  <key>`, landing a later append-only row; a disagreed verdict inverts,
  because the owner's grading is the ground truth on whether a brief
  shape drove.
- **The conservative ride-back**: when the model asks for a role or a
  turn cap for a brief shape, the suggestion consults the calibration —
  a fit cell whose confidence-weighted wrong-rate crosses a named floor
  moves the suggestion among the remaining candidates, and the
  delegation row names the calibration that caused it. The
  insufficient-rate NEVER moves anything: it notes the missing
  companions. Config-gated (`delegation.informs_failover`, default on)
  with the off-state restoring the pre-slice behavior exactly.
- A deterministic eval scenario (`delegation-loop-closes`) grades the
  loop through scripted seams: the rows beside the delegations, the
  three verdicts with their signals, the fit rule, the conservative
  ride-back, the dry run's no-spend, the fail-open refusal, and the
  human seam — no paid calls.

### D3 — roles

- **`~/.fleet/roles/<name>.md`** (resolved through `home.rs`'s own
  conventions, never a foreign layout): a role's name, description,
  system-prompt overlay, admitted tool set, skill set, and the memory
  shapes its recall draws from. `fleet roles` lists them as cards.
- The `subagent` tool gains a **`role`** argument; an unknown role is a
  named refusal, never a silent default.
- The role's overlay rides the child's system prompt beside the
  compiled-in one, delimited so the child knows what is standing
  instruction and what is this delegation's role. The overlay is
  bounded and sanitized like every other model-visible text.
- The delegation cell gains the role dimension: per-(role ×
  brief-shape), and the ride-back's suggestion consults the role's own
  cell. An unfit role's cell renders as unfit with what it needs,
  never as a score.
- A role with no overlay is still a role: the tool scope and the skill
  set are the specialization, and the prompt stays the parent's.

### D4 — parallel fan-out

- The agent loop executes a turn's independent tool calls
  **concurrently**, bounded by a named constant
  (`DELEGATION_CONCURRENCY`, default 3, config can only lower).
- **The one-writer discipline, made structural** (the KI-172/173/177
  lesson, and the memory writer's lost-update class): calls that
  mutate shared state — `remember`, `supersede`, and two `write_file`s
  to the same path — run serially; disjoint and read-only calls run
  together. A conflict is refused by name, never silently reordered.
- **Approvals still gate**: a concurrent batch's calls reach the owner
  through the parent's gate one at a time, each labelled with the
  subagent and its goal — the owner decides, never the model.
- **Model-visible ⟺ logged** (hard rule 5): the session log records
  the batch, which calls ran together, and each call's outcome, so the
  concurrency is reconstructable from the log alone.
- The TUI's subagent stage renders the concurrent children (name,
  goal, progress), and the approval prompt names whose call it is.
- A deterministic eval scenario grades the bound, the serialization,
  and the log's reconstructability through scripted seams.

### D5 — live control

- A running child can be **steered** and **stopped** through a named
  surface: `fleet delegate --steer <session> <text>` and
  `fleet delegate --stop <session>`.
- The control channel is a file under the child's own delegation
  directory; the child's turn loop reads it before each model call.
- A steer rides the child's next turn as a message labelled as the
  owner's, never as the model's own — the same labelling the approver
  already carries.
- A stop lands the partial result with the typed reason
  (`stopped by the owner`) and the child's log path — never silence,
  the existing discipline.
- The control's own effect is logged, so a steered child's trajectory
  is reconstructable.

### D6 — the n+2 delegation model

Fleet's chain today is agent → subagent and stops: the `subagent` tool
is outside a child's admitted scope, so the loop refuses the call by
name and `SUBAGENT_MAX_DEPTH` (src/subagents.rs:60) names the cap. This
part raises it to two, matching the discipline that built fleet
(`max_spawn_depth: 2` in Hermes's own config, exercised in the record:
ten grandchild spawns observed across 587 spawn trees, none deeper).

- **The cap becomes 2, still structural.** `SUBAGENT_MAX_DEPTH` reads 2;
  the `subagent` tool is admitted to a depth-1 child's scope and absent
  from a depth-2 child's, so the depth-2 call is refused by name citing
  the cap — the existing refusal shape, not a new one. `Scope::depth`
  already carries the accounting (src/subagents.rs:86).
- **The parent chain rides every row** (D2's shape): the immediate
  parent's session id, the root's, and the depth. A grandchild's row
  joins to the brief that set its direction, and the chain is
  reconstructable from the log alone (hard rule 5).
- **The approval labelling compounds.** An approval prompt at depth 2
  names the whole chain — root goal → parent goal → this goal — bounded
  and delimited, because an owner approving a call whose originating
  intent is two briefs away is approving a paraphrase. The same chain
  rides the progress line and the TUI's subagent stage, indented.
- **Turn caps inherit downward.** A child's cap cannot exceed its
  parent's remaining turns, and `SUBAGENT_TURN_CAP`'s default steps
  down with depth — a depth-2 chain cannot burn the parent's whole
  budget on one branch. The rule is enforced at construction, like the
  no-downgrade rule, not asked of the model.
- **The calibration's cell gains depth.** The cell key is
  (role × brief-shape × depth), because a depth-2 brief is a different
  shape than a depth-1 brief: its information has been through one
  more paraphrase, and its direction is correspondingly less reliable.
  Depth-2 cells will almost always be thin, so the fit requirement
  (slice 18's 8 rows) means they read unfit and inform nothing — which
  is the correct conservative default, and the reason depth 2 is safe
  is the fit rule, not the depth cap alone.
- **The ride-back names the depth it is moving.** A skip or a note
  caused by a depth-2 cell says so in the delegation row's reason, so
  the owner can see that a thin deep sample moved a suggestion.
- A deterministic eval scenario (`delegation-depth-two`) grades the
  chain through scripted seams: a parent spawns a child, the child
  spawns a grandchild, the grandchild's `subagent` call is refused by
  name citing the cap; the rows carry the chain; the approval labelling
  names it; the turn-cap inheritance holds; and a thin depth-2 cell
  reads unfit and moves nothing.

### D7 — the nomenclature guard

This is the part that pays for the rest. Drift caught at write time
costs one pass; drift caught in review costs a fix round, and this
session spent four of them. The evidence is first-person: the parent's
own `"r#match"` (the project's convention is serde's derived `match`,
visible in every sibling field in that file), and the children's
KI-137 (MIT against a vendored Apache-2.0 LICENSE), KI-151 (`main`
against the branch that was actually the tip), KI-152
(`"escalated":false`, a state the core never produces).

The root cause is the fresh-context contract doing its job. The
canonical vocabulary is distributed across the tree's identifiers and
the docs' decisions, and nothing retrieves it at the moment of naming.
So a child generates a plausible name from its own priors, and the
result is a near-synonym with high semantic density and zero
direction.

- **A lexical convention lookup.** A new tool, `convention`, taking a
  `concept`: "what does this project already call this?" The answer is
  the established term with its file:line sites, or a plain statement
  that no established term exists. **Never a ranked list of
  near-synonyms** — see the design decision.
- **The extractor is derived, never hand-maintained.** A new module
  derives the project's identifier vocabulary from the tree — the
  identifiers in the source files and the doc vocabulary — ranked by
  establishment (how many distinct files carry the term, how recently).
  Deterministic: no model sits in the retrieval path. Bounded and
  cached, because the tree is large (the fleet corpus is 44 sources /
  319 entries at 36.8s to build, so the lookup cannot re-derive per
  call).
- **The declaration rides the brief.** D1's `context` carries the task
  domain's vocabulary, derived by the same extractor, so the child
  starts with the canonical words in context rather than having to
  remember to look.
- **The write-time guard.** On `write_file`, the new identifiers are
  checked against the vocabulary. A novel identifier with a canonical
  near-neighbour is **flagged in the tool result with the candidate
  named** — the model reads it and can rename. A near-exact variant (a
  case or separator difference from an established term) is
  **refused**, because that is drift with no other explanation. A
  genuinely new concept is not flagged. Conservative by construction: a
  guard that cries wolf gets ignored, and then it is worse than
  nothing.
- **Drift is a D2 verdict signal, not a separate mechanism.** A
  delegation whose child introduced a novel identifier with a
  canonical near-neighbour grades **wrong** — it drove away from the
  project's trajectory. Mechanically checkable by reading the child's
  session log's `write_file` calls against the vocabulary, the same
  join the route rows already do.
- **The surface.** `fleet convention <concept>` at the CLI and a card
  in the TUI — the concept, the established term, its sites, and the
  honest "no established term" state. The guard's flag rides the
  `write_file` result the model already reads; no new approval gate.
- **A deterministic eval scenario** (`nomenclature-guard`): a concept
  with an established term returns it with sites; a concept with none
  says so; a write introducing a near-exact variant is refused naming
  the canonical term; a novel name for an existing concept is flagged
  with the candidate; a genuinely new concept is not flagged; a
  drifted delegation grades wrong.
- **The ledger as a source, named as the follow-on.** Every KI row
  records the wrong name and the right one, which makes the ledger a
  labelled training set of exactly this drift. Once enough rows
  accumulate they become the extractor's third source — named here,
  not built.

**File ownership, stated rather than discovered.** The extractor and
the lookup are a new module and build in parallel with D1–D6. The tool
declaration shares `src/tools.rs` with D1 and the write hook shares
`src/agent.rs` with D4, so those two pieces sequence behind them — the
one-writer-per-path discipline the parallel slices already learned.

## Design decisions

1. **The loop is reused, never forked.** D2's beat is an ordinary
   judge over the ordinary stream, in slice 18's shape: the same
   ported discipline, the same three-way vocabulary, the same per-cell
   surface with a different actor. A second implementation of the
   discipline is a second place for it to drift.
2. **The child's log is the evidence, not its summary.** The summary
   is the child's own report; the log is what the child actually did.
   Grading the summary grades the model's confidence in itself. This
   is the same redaction-by-design principle the route rows carry, and
   it is why the row shape has no text keys.
3. **The brief's shape is the cell key, not the brief's text.** Two
   briefs that read differently but ask for the same kind of work are
   the same vector; two briefs that read alike and ask for different
   kinds are different vectors. The shape is what the calibration
   keys on, and the text stays out of the row.
4. **The role is a profile, not a prompt string.** Hermes's profiles
   carry their own skills, memories, and model config; fleet's analog
   lives in FLEET_HOME under fleet's own conventions. The overlay is
   delimited and bounded, because a role that can rewrite the standing
   instructions is a role that can be prompt-injected by its own file.
5. **Concurrency is bounded and disjoint-by-default.** The bound is a
   named constant the config can only lower; the serialization rule is
   structural, not advisory. An unbounded fan-out turns one bad brief
   into N bad briefs on one clock.
6. **Control is a file the child polls, not a signal.** A signal
   cannot carry a steer's text and cannot be logged. The file is the
   child's own, under its own delegation directory, and reading it
   between turns costs nothing when it is absent.
7. **Depth is two, and the fit rule is what makes it safe.** A brief
   at depth 2 is a paraphrase of a paraphrase: its direction was set
   two hand-offs ago, which is exactly the case the owner's framing
   says to distrust — high semantic density, no verified direction. The
   depth cap bounds the blast radius; the fit requirement (a thin
   depth-2 cell reads unfit and informs nothing) bounds the influence.
   Neither alone would do: the cap without the fit rule lets a lucky
   deep sample move routing, and the fit rule without the cap lets an
   unbounded chain form. `SUBAGENT_MAX_DEPTH` names the cap, and a
   depth-3 call is refused structurally.
8. **Nomenclature is the one place proximity is distrusted outright.**
   A semantic search returns the nearest term, and the nearest term to
   a concept is usually a synonym — which is the drift. Slice 19 made
   direction outweigh proximity for facts; for naming, direction means
   *the term is established in this tree*, and a name that means the
   right thing and appears nowhere must not be surfaced as canonical.
   The lookup is lexical, evidence-bearing, and deterministic for that
   reason, and the vector channel is not consulted for it.
9. **The vocabulary is derived, never hand-maintained.** A hand-written
   conventions file goes stale and then lies — the drift it was meant
   to prevent, one step removed. The tree is the ground truth for what
   is established; the docs' decisions are the tiebreaker when the tree
   and the intent disagree; the ledger's wrong-name/right-name rows are
   the labelled record of drift that already escaped.
10. **The guard flags, it does not forbid — except near-exact.** A
    novel name for an existing concept is flagged with the canonical
    candidate named, so the model can rename; a case or separator
    variant of an established term is refused, because that is drift
    with no other explanation. A guard that refuses legitimate new
    names gets ignored, and an ignored guard is worse than none.

## Out of scope

- A hand-maintained conventions file as the vocabulary's source. The
  extractor derives it from the tree; a curated file goes stale and
  then lies, which is the drift one step removed. The docs' decisions
  remain the tiebreaker when the tree and the intent disagree.
- The ledger as a third extractor source (D7 names it as the
  follow-on): the wrong-name/right-name pairs are a labelled training
  set of this exact drift, and they become a source once enough rows
  accumulate.
- Depth-3 delegation and beyond. Two levels is the cap this slice lands
  and the record it was drawn from (Hermes's `max_spawn_depth: 2`, with
  no depth-2 spawn observed across 587 trees); a third level is a
  different argument with a different blast radius, and the structural
  refusal already names the cap that would have to move.
- Cross-machine or cross-process delegation; the child is an ordinary
  in-process `Agent` over an ordinary `Session`.
- The ops surface (boards, dispatchers, scheduled fan-out) — fleet is
  a single-user native binary, and the cron jobs that would drive a
  daily `fleet delegate` are the existing cron slice's shape, not a
  new mechanism.
- Judging the child's answer QUALITY (as opposed to the delegation's
  outcome) — the explanation beat's subject beside the eval corpus's
  grades.

## Assumptions (listed, not asked)

- The sys1 judgment's spend is small but real; D2's budget bounds it
  and the dry run rehearses it, in slice 18's shape.
- The parent's session log reliably follows the child's, so "the work
  continued on the same paths" is joinable; where it is not, the
  insufficient signal is absent rather than guessed.
- Roles are the owner's to write; a role file is trusted input in the
  same way a skill file is, and the overlay is bounded and delimited
  for exactly that reason.
- The concurrency bound of 3 is sized for a single owner's machine and
  the embedder's single slot; a heavier bound is a config change, not
  a code change.
- A depth-2 brief is composed by the model at depth 1, so the chain's
  direction is only as good as the middle brief. That is why the cell
  keys on depth and why the fit requirement — not the depth cap — is
  what keeps a thin deep sample from moving anything: the slice assumes
  depth 2 will be rare enough to stay unfit, and if it stops being rare
  the calibration will say so.
