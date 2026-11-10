# Deep dive: Baidu's ERNIE Evolve, read against Fleet's self-improvement loop — 2026-11-10

Source: [ERNIE Evolve](https://ernie.baidu.com/research/posts/ernie-evolve/)
(Baidu research blog). Companion to
`deep-dive-2026-11-07-self-improvement.md` — that dive read DoubleAI's
article against fleet's loop *as designed*; the plan it proposed landed
the same day (`docs/plans/2026-11-07-self-improvement.md`, completed,
C0–C9), so this dive reads the article against the loop *as built*:
the demand map, the clustering beat, the twin/sham race, `analyze`,
the session-scoped beats, `eval --gaps`, and the capability library are
all in `src/` and were read at their lines this session. The article's
claims are the article's own; where this doc infers beyond either
source, it says so.

The article's thesis in one line: run software R&D as a **recursive
self-improvement loop** — an agent pipeline that proposes code
changes, evaluates them on real workloads, and feeds every result,
success or failure, back into the next direction — and it beats the
public best on training-algorithm and kernel-optimization benchmarks
while also shipping real production gains (623.6% of baseline
generation speed on one inference stack in four rounds).

Fleet is not optimizing a model or a GPU kernel; it optimizes
**itself** — its own source, skills, memory, and config. That makes
the article's central structural problem — *how do you trust the
measure when the thing under measurement can edit the measuring
instrument?* — worse for fleet than for ERNIE, and it is where this
dive's sharpest finding lands.

## The article's loop, in its own shape

- **Two nested loops.** The *outer* loop decides the research
  direction; the *inner* loop develops and tests improvements within
  a direction.
- **Five roles.** The **Director** picks the next direction and a
  starting version (original code, best version, an intermediate, or
  latest upstream main), drawing on an **Experience Bank** (past
  candidates, code changes, results, failure reasons) and a
  **Knowledge Base** (papers, open-source code). The **Selector**
  picks a candidate from the current lane's tree and states the
  hypothesis, weighing scores, code changes, failure reasons, and
  prior decisions. The **Executor** implements it in an isolated
  environment. The **Evaluator** — a *fixed evaluation program, kept
  outside the optimized repo and invisible to the agents* — grades it
  on real workloads, excludes out-of-scope changes, and accepts
  gains only **above a noise threshold**. The **Reviewer** checks the
  code change against the results and rejects candidates that depart
  from the stated direction or exploit evaluation loopholes.
- **Parallel research lanes, each a tree of candidate versions.**
  Every code change and result is recorded so later rounds inherit
  it; unsuccessful approaches are not retried without new
  justification.
- **Failure is data.** The article documents its losers alongside its
  winners: a collectives change that cost 14.0%, an MoE comm flag
  that moved nothing above noise, a parallelism hypothesis mostly
  rejected. The experience bank is built from refusals as much as
  gains.
- **Production rounds, not just benchmarks.** Each round states the
  hypothesis, the change, the measured delta, and what was ruled out —
  with end-to-end load tests as the evaluator, because single changes
  (0.3–2%) often sit under measurement noise.
- **The safety stance.** RSI must remain human-controlled,
  interruptible, and bounded in goals, permissions, and resources.

## Fleet's loop, as built — and where it already lives the article's principles

- **The Evaluator's isolation is fleet's biggest divergence — see the
  gaps below.** Everything else the article specifies, fleet has a
  counterpart of, most of them read at these lines:
- **The fixed evaluation program.** Fleet's corpus
  (`src/eval.rs`) is exactly ERNIE's shape: fixed, graded,
  deterministic, scripted against the mock, every failure a graded
  verdict — and it gates every source apply (the `eval_corpus` step,
  `src/improve.rs:1775`, refusing on regression with the scenario
  named, `eval_step_may_apply` at `src/improve.rs:1385`).
- **The Experience Bank.** Three stores together: the calibration
  stream (every `propose` verdict with its gate steps and artifact
  sha, `finish` at `src/improve.rs:1826`), the proposal store itself
  (status + `last_refusal` stamped back into the file, shown on the
  card at `src/improve.rs:2670`), and the lesson store — one lesson
  per **applied** change, handed to the next filing for the same
  target kind (`lessons_for`, `src/improve.rs:745`, appended to the
  filing confirmation at `src/improve.rs:1141`), rendered as the
  capability library (`src/improve.rs:3019`).
- **The Director's instrument.** The demand map
  (`src/demand.rs:151`) is fleet's phantom gradient: per-class
  bad-rate and trend over a window, the improving left alone and the
  flat/worsening named as targets, thin samples never read as healthy
  (`DEMAND_MIN_ROWS`, `src/demand.rs:36`). The clustering beat groups
  a period's failures and files shape proposals
  (`src/demand.rs:369`, `:617`) — the outer loop's
  "failure → direction" step, owner-gated.
- **Analysis before hypothesis.** `fleet improve analyze`
  (`src/improve.rs:3091`) measures which classes fail and the
  confidence threshold that separates right from wrong before a
  proposal's acceptance criteria are written, and the `improve`
  tool's declaration names it as the first step
  (`src/tools.rs:1747`). ERNIE's Selector weighs the same evidence;
  fleet mechanizes the threshold question the article leaves to the
  model's judgment.
- **Real-vs-placebo, stricter than the article.** The race
  (`src/improve.rs:2862`, verdict at `:2837`) runs the corpus on the
  applied tree, the twin (change reverted) and the sham (diff
  scrambled) and refuses a change either baseline matches. ERNIE has
  a noise threshold and a Reviewer; fleet has a causal control the
  article does not — a deterministic sham proving the gain came from
  the change's information, not its bulk.
- **The gap instrument.** `fleet eval --gaps`
  (`src/eval.rs:5559`) names the failure classes the stream shows
  but the corpus does not cover, and `--promote` (`src/eval.rs:5615`)
  seeds a new scenario from a gap's examples — the evaluator growing
  itself, which the article lists as a *future* direction ("AI
  improving benchmarks/eval suites").
- **Bounded, batch-only judgments.** The judge and delegate beats
  carry a named daily cap, `--dry-run`, and no synchronous path may
  name them (`src/judge.rs` module doc) — the article's "bounded in
  resources" as structure rather than stance.
- **Human-controlled, interruptible — by ruling.** ERNIE's safety
  paragraph is fleet's hard rules restated: the `improve` tool files,
  the owner's `fleet improve apply` applies, destructive changes wait
  explicitly. The article treats human oversight as a constraint it
  operates under; fleet's is architectural.

## The gaps, as improvement areas

Each area names the article's finding, fleet's current state with
this session's reads, and the candidate direction. These are dive
findings, not decisions.

### 1. The evaluator lives inside the tree a proposal can edit

The article's sharpest structural rule: the evaluation program is
**kept outside the optimized repo, invisible to agents** — because a
candidate that can edit its own measuring instrument is not a
candidate, it is a verdict forged in advance. Fleet's `patch` change
writes any repo-relative path (`stage_patch`, `src/improve.rs:2241`).
The refusals it does carry are precise precedents — absolute and
`..` paths, a `memories/` patch refused because `supersede` is the
only door (`src/improve.rs:2268`), a target with uncommitted work —
and **none of them covers the evaluator**. Nothing refuses a patch
whose path is `src/eval.rs`, the corpus's drivers, or the tests that
pin them.

Why the existing machinery does not close it on its own:

- The **gate** passes: weakening a scenario keeps fmt, clippy, build,
  and test green — the test *is* the weakened scenario.
- The **corpus step** runs the rebuilt binary against the now-soft
  scenarios: green, and where some scenarios were failing before, a
  *higher* score lands as a "measured gain" (`contrast_summary`,
  `src/improve.rs:1465`).
- The **race** only helps when someone runs it, and even then an
  evaluator rewrite moves the instrument in the applied arm while
  the twin reverts it — a softened corpus beating its own old
  baseline is exactly the case the strict `applied > twin` check
  (`:2837`) was not built to refuse, since a genuine score rise and
  an instrument rewrite are indistinguishable by score alone.

The owner applies every proposal, so this is defense-in-depth against
a model-filed proposal whose card reads plausibly — and against the
owner's own fatigue. The candidate direction follows fleet's own
precedent, not ERNIE's repo split (fleet is one repo by founding
rule): a **protected-path set in `stage_patch`** — the evaluator's
sources, its scenario fixtures, the tests that pin them, and the
improve gate itself — refused by name, exactly the `memories/`
refusal's shape, with `fleet eval --promote` left standing as the
one legitimate door for growing the corpus. The article's *"invisible
to agents"* cannot be literally honored in a self-contained single
repo (the model can read anything), so fleet's honest form of the
rule is *unwritable through the self-modification door*.

### 2. The declared target and the patched path are never checked against each other

The article's Reviewer "rejects candidates departing from the
intended direction". Fleet's mechanical near-equivalent already
exists for one op: a skill proposal whose SKILL.md names a different
`name` than the declared target is refused
(`src/improve.rs:1969`). The `patch` op has no counterpart: the
frontmatter may declare `target-name: src/agent.rs` while the change
patches `src/eval.rs`, and the apply stages it — the drift guard
(`src/improve.rs:1906`, `:1920`) stamps the *patch's own* path, so it
guards staleness, not scope. The card shows both strings, so a
reading owner can catch the mismatch; a structural check cannot be
tired. Candidate: require `change.path` to equal the declared
`target-name` for `target: source` proposals, refused by name — a few
lines, with the skill op as the in-repo precedent. (Multi-file
intent becomes what it honestly is: multiple proposals, or a target
word that names the scope and a path check that understands it —
the dive does not settle which.)

### 3. Refusals are experience the next filing cannot see

The article is explicit: unsuccessful approaches are recorded and
**not retried without new justification**; the Selector weighs
failure reasons. Fleet's refusals ride back into *their own*
proposal's file (`last_refusal`, stamped into the frontmatter at
`src/improve.rs:1875`, shown on that card at `:2670`) — so the same proposal cannot
rediscover its failure. But the lessons handed to a *new* filing
come only from the lesson store, which `finish` writes **only on an
applied verdict** (`src/improve.rs:1835`): a fresh proposal targeting
a file that has already seen two refused changes is handed the applied
history and nothing else. The refusal knowledge exists on disk; the
read-before-propose seam does not read it. Candidate: the filing
confirmation also lists recent refused (and waiting) proposals for
the same target, each with its `last_refusal` — one store read in
the same place `lessons_for` already runs.

### 4. The loop's own demand is invisible to the demand map

The demand map reads five row families — `route_judged`,
`eval_corpus`, `delegation`, `validate`, `learn_check`
(`src/demand.rs:161–199`) — and drops everything else at `:200`. The
`propose` rows are not among them, so the one class of decisions the
self-improvement loop makes about *itself* — which target kinds get
filed, which get refused, which land — has no bad-rate, no trend, no
thin flag. ERNIE's Director and Selector both feed on the full
record, refusals included; fleet's outer loop can see the
workload×provider demand of its routing but not the refusal rate of
its own proposals. Candidate: a `selfimprove` family in
`collect()` — class = target kind (or target name), bad = refused —
which also gives area 3 a headline number ("this target has refused
three times in the window") instead of a list.

### 5. An applied proposal is never settled against reality

The article closes every candidate through **real workloads**: the
change ships into the production measurement, the result enters the
experience bank, and the next round starts from that number. Fleet's
source proposals are measured exactly once — the deterministic
corpus at apply time — and then the expected effect, which the
proposal itself declares (`effect`, `src/improve.rs:1186`), is never
read back. The lesson records the effect as prose with confidence
`measured` meaning *measured on the corpus*, not *observed to have
happened* (`Lesson::from_proposal`, `src/improve.rs:630`). Fleet
already owns the pattern for closing this: the resource contract's
`grade_judgments` (`src/calib.rs:1085`) grades a judged verdict by
**the row that follows it** — outcome over intention, and a verdict
with no outcome keeps no rate rather than a flattering one. The
candidate is that same shape for proposals: N days after an apply,
grade it against the demand trend of the class it targeted —
improving, flat, or worse — land the grade as a row, and let the
capability library's confidence become "held in the wild" beside
"held on the corpus". The honest open question this dive does not
settle: **how does a patch's target map to a demand class?** A
proposal targeting `src/route.rs` plausibly maps to a `route/`
family; a proposal targeting `src/tools.rs` maps to nothing a class
names. The mapping may need a declared field on the proposal (the
effect statement in structured form), and some proposals may honestly
grade as *unmeasurable* — which the row should say rather than
invent.

### 6. The outer loop stops at printing

ERNIE's Director *acts* on the experience bank — picks the direction,
picks the starting version — with the human oversight expressed as
the safety stance. Fleet's outer loop ends at `fleet demand` printing
cards that a human (or the drafting model in a session) reads before
filing anything. That is the owner's ruling and this area does not
propose breaching it. The candidate is narrower: make the read
complete rather than automated — areas 3 and 4 are what the printed
view is missing (the refusals, the loop's own demand), and with them
the Director's evidence base is whole even though the decision stays
the owner's. Worth stating as the article's one *deliberately
non-absorbed* piece: the autonomous direction pick.

## Where fleet is ahead, named honestly

- **The race.** A deterministic twin and sham is a stricter causal
  test than a noise threshold plus reviewer judgment; ERNIE cannot
  cheaply scramble a 50,000-submission benchmark, fleet's mock corpus
  makes it one line.
- **Promotable gaps.** The evaluator growing itself from the
  stream's own uncovered classes (`eval --gaps` + `--promote`) is
  the article's roadmap item; fleet shipped it.
- **Determinism as noise control.** ERNIE needs the noise threshold
  because load tests are noisy; fleet's corpus is scripted, so
  strict inequality at the race is already a perfect threshold *for
  the synthetic half* — and area 5's real-workload readback is where
  fleet will need ERNIE's epsilon, which the demand map already owns
  (`DEMAND_TREND_EPSILON`, `src/demand.rs:40`).
- **One artifact, five instruments.** ERNIE's roles are pipelines
  over one codebase; fleet's demand map, analyze, race, gaps, and
  capability library read *one* calib stream and *one* proposal
  store — the same evidence, five questions, no second corpus to
  keep in sync.

## Scale, named as non-adoption

Parallel research lanes, candidate version trees, and
start-point selection (best / intermediate / upstream) are the
article's answer to optimizing a large system under a numeric
objective with many candidates alive at once. Fleet's inner loop is
sequential by design — one proposal, gate, verdict — and its version
history *is* git plus the undo seam; the race already runs three
arms of one candidate. At fleet's scale the lanes would be cost
without concurrency (every arm pays the full gate), and the
owner-gated single-file apply is the tree. Not a gap; the article's
own scale (700+ developers' submissions, ~50,000 candidates on one
benchmark) is the difference.

## What this dive does not settle

Whether the protected-path set (area 1) should extend to the gate's
own scripts and the structure-guard tests, or only the corpus; the
scope-vs-path question in area 2 (strict equality vs a declared
scope list); the target→class mapping that area 5's settlement
requires, and whether an unmeasurable proposal should refuse, grade
as unmeasured, or both; whether the propose-family demand (area 4)
belongs in `collect()` or in the improve surface's own view; and in
what order any of this becomes a plan — area 1 is the one the
article argues is structural rather than cosmetic, because every
other measure in the loop is read through the instrument area 1
would let a proposal rewrite.
