# Deep dive: LearnHarness read against Fleet — learning as a first-class work-menu feature

Source: [amosblomqvist/learnharness](https://github.com/amosblomqvist/learnharness)
(AGPL-3.0), read from a full clone at fidelity — the README, the
architecture, the teaching theory, and the bundled skill. Every
fleet-side claim rides this session's verified reads; the LearnHarness
claims are the repo's own files.

LearnHarness in one line: **an agent harness for learning** — the
agents plan and teach lessons from the learner's current understanding
to their goal, building a picture of what is understood that carries
across sessions. The owner's ask: learning is part of the work
process; build something similar as a first-class citizen in fleet's
work menu.

## The theory (the canonical layer)

Ten sections; the load-bearing ones:

1. **Two knowledge states.** Current understanding — the held
   propositions, accepted as obviously true, safe to build on — and
   the goal understanding, often hard for the learner to articulate.
   The probe brackets both.
2. **The terminal condition is self-evidence.** Not recall, not
   coverage: the goal must *feel obviously true*. Memorized facts rot;
   held facts persist. Retention is the felt experience of being safe
   to lock in (nothing more fundamental can contradict it) and being
   connected (held by its relations, not rehearsal).
3. **The click.** Loose facts collapse into generators: the pile
   becomes a derivation. Knowing why over knowing that.
4. **Knowledge is structure**: propositions as nodes, connections as
   edges; a new proposition connecting to nothing is fragile.
5. **Teaching = facts in the right order, one reasoning step at a
   time.** Each step attaches to what is held by a single step, so it
   lands as obviously true; errors surface the moment they are stepped
   on and are repaired before anything is built on them.
6. **Foundations**: derive when possible; place an unconditionally
   true, simply motivated fact when not.
7. **Two optimizations.** The learner's effort goes into the material
   (the system absorbs the logistics — research, fact-checking,
   planning, ordering); the **minimal tree** — the smallest dependency
   graph from current to goal; nothing held re-taught, nothing off-path
   taught.
8. **Presentation**: the clearest form per step; no meta-narration;
   every sentence installs understanding or is overhead.
9. **A check after every step** — the error-catching mechanism.
10. **The demands**: the prober produces precise states; planning is
    real work (the minimal tree is searched); teaching moves in
    single, checked steps; the plan's resolution must not leak into
    the lesson; delivery is efficient by default.

## The loop and the graph

- **Measure.** The prober finds where the understanding ends on each
  part the lesson depends on; answers, confidence, and thinking bracket
  the states. The goal is pinned down.
- **Plan.** The minimal tree, shown to the learner as a coarse map;
  a validated Mermaid DAG, replaceable.
- **Teach.** One reasoning step at a time: predict, explain, check;
  confusions repaired before continuing. The aim: the goal feels
  obviously true.
- **Practice.** Separate sessions; varied problems requiring the
  knowledge to come to mind *without being named*; hints adapt.
- **The knowledge graph.** Propositions and reasoning chains
  (premises that **jointly** imply a conclusion), extracted from
  teaching prose and imported notes; provenance to source sessions;
  future sessions read it; it stores the ideas and their connections,
  **not mastery ratings**. Extraction is post-hoc, gated by an
  uncovered-token threshold, and merged deterministically
  (extract → audit → merge).
- **The evidence discipline.** Sessions record what happened; the
  graph is the intended map, not a stored mastery assessment. The
  learner's confidence is self-reported; the thinking fields are the
  richest data; model-authored content can be wrong.

## What fleet already has to build on

- **The session machinery** — sessions, records, the run loop, the
  approvals: the lesson and practice sessions' host.
- **The dependency graph** (`src/graph.rs`) — scores, gates, critical
  path, fact-based verification: the lesson plan's natural home. A
  Mermaid DAG plan validates against it.
- **The roles** (`src/roles.rs`) — the named specializations a
  delegation runs under: the Prober, the Teacher, the Practitioner.
- **The memory substrate** — facts with provenance and the recall
  graph: the graph's *propositions* could be a fact type; the
  *chains* (joint premises) need a structure the pairwise recall
  graph does not have.
- **The calibration stream** — the learning events (probe answers,
  checks, practice outcomes) as rows.
- **The eval corpus** — the fixed graded suite: the practice problem
  set's sibling.
- **The face's rail and panes** — the Learning section's surface.
- **The dream pass** — the extraction pipeline's sibling (a
  model-driven pass over records, gated, writing to a store).

## The design: Learning in the work menu

The rail gains a **Learning** section; the feature's components:

1. **The lesson session** (a session kind). The measure → plan →
   teach loop, hosted in fleet's session machinery: the prober's
   questions and the learner's answers (with thinking and confidence)
   as session records; the plan as a Mermaid DAG in the graph module;
   the teaching as the session's turns. The learner's thinking
   fields ride the records — the richest data, per the theory.
2. **The lesson plan** (the minimal tree). The prober's bracketed
   states in, the DAG out: the nodes are the propositions to teach,
   the edges the single-step dependencies, the critical path the
   teaching order. The plan doc carries the coarse view the learner
   sees; the DAG is validated.
3. **The knowledge graph rides the memory substrate.** The learning
   record is project knowledge, not a silo: the propositions are facts
   (a `proposition` type in the substrate's vocabulary), the chains
   are facts carrying the joint-premise structure (a `premises` field
   — the piece the pairwise recall graph lacks), the provenance is
   the fact's `source` field, and the practice outcomes are facts.
   The extraction pipeline (extract → audit → merge) runs over the
   teaching prose, gated, and writes through the same writer; the
   recall, the dream pass, and the calibration rows all apply. A
   future session working the project recalls what the learner holds
   like any other project knowledge — the "what you hold" section is
   the assembled context reading the substrate, not a parallel store.
4. **The practice sessions.** Varied problems requiring the knowledge
   to come to mind unnamed; one active problem per turn; the outcomes
   as session evidence (never graph mastery). The problem set joins
   the eval corpus's world.
5. **The face surface.** The Learning rail section: the active
   lesson (the plan's coarse view, the current position), the graph
   view (the propositions and chains), the practice entry, the
   history. The learner's side: the teaching turns, the checks, the
   practice problems.
6. **The evidence discipline**, as a ruling: sessions are what
   happened; the graph is the intended map; confidence is
   self-reported; model-authored content can be wrong; the thinking
   fields are data, not decoration.
7. **The self-improvement feed.** The learning record's friction is
   the loop's richest signal, and it already has consumers: the
   learning events (probe answers, checks, practice outcomes) join
   the calibration stream as rows the demand map reads — the
   learner's confusions become demand classes; the recurring
   confusions are the clustering beat's shape-proposal feed (the
   C1 beat's output lands as `improve` proposals, owner-applied, as
   it already does); and the friction itself — what the learner
   struggled with, what the practice exposed — is proposal
   feedstock: "the confusion about X says the docs lack Y", "the
   lesson's gap says the eval corpus lacks Z". The dream pass reads
   the teaching prose for the extraction and the learner's records
   for the confusions, in one pass over the same records.
8. **The capability feed.** The learning record's friction also
   names what the project *lacks* — and the lack's kind decides the
   surface, each with machinery that already exists:
   - **Personas.** A lesson that repeatedly needs an expertise the
     project has no one in signals a role — the roles module, and
     the huddle's `author_persona` precedent (a domain the room has
     no one in is bridged by authoring a purpose-built persona).
   - **Skills.** A lesson's procedure that recurs — the steps that
     worked, the checks that caught the confusions — signals a
     skill, and the self-authoring drafter is its consumer (the
     drafter already files `proposed: true` drafts from sessions;
     the teaching prose is skill-shaped feedstock).
   - **Tools.** Friction no tool addresses — the learner's workflow
     needing a capability the compiled-in tool set lacks — signals
     a tool, and the `improve` proposal store is its backlog.
   - **Workflows.** A lesson's procedure that repeats across lessons
     signals a workflow — the cron's scheduled runs, the plan
     templates, the huddle workflows are its consumers.
   The classifying rule is the friction's lack kind, and the
   huddle's capability-gap card is the precedent: the signal
   already exists in one surface; the learning record generalizes
   it. Each proposal lands through its surface's own gate — the
   role overlay's approval, the drafter's owner-gated drafts, the
   improve store, the cron — never a silent self-application.

## The first steps (the plan's shape)

The feature is a program: the lesson session kind and its records
first (the host — and the records ride the substrate from the
start: the probe answers and the checks are facts and calib rows,
not a parallel store), then the prober role and the probe loop, then
the plan (the graph integration), then the teaching loop, then the
graph's chain facts and their extraction, then the practice, then
the face surface, with the self-improvement and
capability feeds wired at each step the evidence lands.
The theory's demands are the acceptance criteria: the prober's states
are precise (the plan's ceiling), the teaching moves in single
checked steps, the plan's resolution does not leak, the logistics are
the system's, and the goal is self-evidence — measured by the
learner's own "obviously true", not by coverage.

## What this dive does not settle

The extraction pipeline's quality (the graph is only as good as the
extract-audit-merge); the learner's thinking capture (the fields must
beearned, not demanded); the face surface's design (a new pane family —
the mockups-first rule applies); and the scope of the first plan (the
lesson loop alone is a program). Each is a plan's question.
