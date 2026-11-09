# The learning loop

Learning is a first-class work surface: the **measure → plan → teach**
loop, with the practice beside it, all riding the substrate the rest of
the product already reads. A lesson is a real session in a real log —
the owner's own work, not a delegated child — whose `start` record
carries the kind (`teaching` or `practice`) and whose run carries the
role whose voice the model hears: the **prober**, the **teacher**, or
the **practitioner**. The plan:
[`docs/plans/2026-11-08-learning-loop.md`](../plans/2026-11-08-learning-loop.md);
the design record it implements is
[`docs/architecture/deep-dive-2026-11-08-learnharness.md`](../architecture/deep-dive-2026-11-08-learnharness.md);
the screens it was built against live in
`docs/plans/mockups/2026-11-08-learning/`.

## The theory it implements

The loop is the LearnHarness design, read against the repo: the agents
plan and teach lessons from the learner's current understanding to their
goal. The load-bearing ideas, each of which is a rule the code carries:

- **Two knowledge states.** The current understanding — what the
  learner holds, safe to build on — and the goal, often hard for the
  learner to articulate. The probe brackets both.
- **The terminal condition is self-evidence.** Not recall, not
  coverage: the goal must *feel obviously true*. The lesson ends when
  the learner's own words say so.
- **Teaching is facts in the right order, one reasoning step at a
  time.** Each step attaches to what is held by a single step, so it
  lands as obviously true; errors surface the moment they are stepped
  on and are repaired before anything is built on them.
- **The minimal tree.** The smallest dependency graph from what the
  learner holds to the goal: nothing held re-taught, nothing off-path
  taught.
- **The check after every step** — the error-catching mechanism.

## The surface

`#/learning` sits in the **work** group of the rail — learning is work
the owner opens the way they open a chat, not a category of its own.

The pane is the same shell the desk rides: **the projects rail on the
left, the lesson's conversation and composer in the centre, and the
app's own collapsible pane on the right**, whose leading tab (`the
work`) carries the lesson's plan, the graph, the practice and the
history.

- **The rail** reads `GET /api/projects`: every project with its lesson
  count, and the selected project's lessons under it. A lesson's project
  is its session's own working directory — the fact the archive groups
  by — so a lesson born in a folder lands under it and one born without
  lands in the unbound bucket.
- **The work** (the pane's leading tab) carries the four sections as
  chips — the lesson, the graph, the practice, the history — with the
  lesson's plan beside them.
- **The conversation** is the lesson session's own log, replayed — a
  lesson is a real session, so its thread is the conversation in the
  centre. The composer
  beneath answers, asks, or says "next step"; its role chip is the role
  the run rides (teacher / practitioner), and the practice chip starts a
  practice beside the lesson (`POST /api/learning/start`). A run into an
  existing lesson names that session (`?session=`), so the composer
  never forks a second conversation beside the log.
- **No dead ends:** an empty rail offers the one act that fills it
  ("start a lesson"), a lesson with no plan renders its own action, and
  a run in flight turns the composer's button into Stop.

The sections themselves are the mockup's four:

- **the lesson** — the plan's coarse view: each step with its state
  (*held*, *you are here*, *next*, *later*), the position (`step 3 of
  4 · 2 checked`), and **what you hold** — the foundations, the
  propositions the learner already has. The plan's resolution does not
  leak: the learner sees the map, never the teaching's reasoning about
  it.
- **the graph** — the propositions the lessons built, each with the
  chain it rests on (`because a + b`). The ideas and their connections,
  never a mastery.
- **practice** — the entry and the outcomes: the settings posed, the
  answers, and whether each held. What happened, never a score.
- **history** — the lesson and practice sessions, newest first.

The two entries — **start a lesson** and **start a practice** — post the
task with the kind and the role; the bridge births the session with the
lesson kind on its `start` record and the role as its persona, so the
overlay rides the run's own seam (the same seam a delegation's role
overlay uses), and the new session's id arrives on the event stream.

## The loop

1. **Measure.** The prober asks, one question at a time, through the
   lesson session's own turns. Each answer is captured whole — the
   learner's words, their thinking, and the confidence they stated —
   and the probe draws the bracket: what the learner holds, where it
   ends, and the goal, pinned from the learner's own articulation.
2. **Plan.** The minimal tree builds in the graph module from the
   bracketed states: the propositions the teaching moves through, the
   single-step dependencies, and the critical path as the teaching
   order. The build refuses a tree that cannot be taught — a premise
   naming nothing, a goal naming no node, a cycle, or a proposition the
   goal never reaches.
3. **Teach.** One reasoning step at a time — predict, explain, check —
   never re-teaching a foundation, repairing a confusion before
   anything is built on it, and stopping on a confusion that will not
   repair rather than building on ground that is not held. The goal's
   self-evidence is the only terminal condition.
4. **Practice.** Varied problems that require the knowledge to come to
   mind **unnamed** — a setting that covers the proposition's claim is
   refused, because a problem that names the answer teaches the name
   instead of the knowledge. One problem is active at a time.

## The records

Everything the loop produces rides the substrate and the calibration
stream — there is no learning silo:

- **the probe answers** — `probe` facts, each with the answer, the
  thinking, and the confidence; the bracket lands beside them.
- **the lesson plan** — a `lesson-plan` fact carrying the goal, the
  teaching order, and the Mermaid map the learner sees.
- **the checks** — one `check` fact per taught step: the prediction,
  the check, and every repair with the confusion it answered.
- **the lesson** — the terminal record: the goal's self-evidence and
  whether the lesson completed.
- **the propositions and chains** — `proposition` facts whose
  `premises` field carries the chain's joint-premise structure (see
  [memory](memory.md)); the extraction pipeline audits the teaching
  prose against the claims and merges at the 80% uncovered-token gate,
  deterministically — the ordered union of premises, the first writer's
  claim standing.
- **the practice outcomes** — `practice` facts: the setting, the
  answer, and whether it held.
- **the calibration rows** — `learn_probe` (the probe ran, what it
  pinned) and `learn_check` (one per taught step: the proposition, the
  learner's own confusion, whether it held).

**The evidence discipline**, as a ruling: sessions are what happened;
the graph is the intended map; confidence is self-reported; and
model-authored content can be wrong. The graph's record carries the
claim, the premises, and the provenance — no confidence, mastery, score,
or held line. The session's records carry the held and the answer.

## The feeds

The learning record's friction is the loop's richest signal, and it
already has consumers:

- **the demand map** reads the `learn_check` rows' confusions as the
  `learn` family — the class is the proposition the step taught, and a
  row is bad when the learner named a confusion. A repeating confusion
  is a cluster the beat files as a shape proposal in the improve store,
  owner-applied like every other proposal.
- **the drafter** reads the lesson session's own transcript: a
  procedure that recurs — the steps that worked — is skill-shaped
  feedstock, and the drafter files a `proposed: true` draft the owner
  gates.

Each proposal lands through its surface's own gate; nothing
self-applies.

## What this does not do

The host-specific machinery (the shell, the providers, the dictation)
is the repo's own; the background research workers map to the lanes and
the subagents. The extraction pipeline's quality is bounded by its gate
and the dream pass's reading, not guaranteed by it. And the first
lesson's live proof — the owner's own — is the feature's remaining gate.
