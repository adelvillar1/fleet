# Deep dive: DoubleAI's "Can LLMs redesign themselves as they train?", read against Fleet's — 2026-11-07

Source: [Can LLMs redesign themselves as they
train?](https://www.doubleai.com/research/can-llms-redesign-themselves-as-they-train)
(DoubleAI research blog, 2026-11-06). Companion to
`deep-dive-2026-11-07-memory.md` and
`deep-dive-2026-11-07-memory-dreaming.md`; every fleet-side claim was
read at its file:line this session, and the article's claims are the
article's own. Where this doc infers beyond either source, it says so.

The article's thesis in one line: put an agent inside a neural
network's training run and let it **redesign the network while it
still learns** — reading its internal state the way mechanistic
interpretability does, then changing the data, the objective, the
update rule, or the architecture. Fleet is not a neural network, but
it is a system that grades itself and modifies itself, and the
article's mechanisms map onto that problem more directly than the
memory article's did: both are about *when and how a system may edit
itself while it runs*.

## The article's mechanisms, in its own shape

- **Mechanistic intervention.** The agent reads the live network's
  internal state — not to explain a finished model, but to change one
  that is still training. Interpretability's tools (probes, ablations,
  attribution) become actuators.
- **The phantom gradient.** ∂L/∂h at every token and layer: a map of
  *unmet demand*, where SGD is not satisfying what the loss wants.
  Raw, it misleads (it mostly encodes *which token is the answer*);
  centred — subtract the mean over documents sharing the correct next
  token — it encodes *what the context asked for*. Persistent phantom
  gradients across checkpoints mark where to intervene.
- **Code layers behind zero gates.** The agent writes a small program,
  plants it at a chosen layer behind a learnable scalar initialised to
  zero: `h = h + Attn + MLP + g·code(...)`. The edit is discrete; the
  gate makes it continuous for the optimizer — useful layers open
  within ~100 steps, useless ones stay shut.
- **The recipe.** Grade per cell (task × input size) at checkpoints →
  high-flat-error cells are targets → collect the wrong tokens →
  backprop each token's loss alone → read phantom gradients → cluster
  by direction (spherical k-means on gradient + hidden-state
  fingerprint) → read the cluster's examples and *name the failure
  mode* → propose candidate code layers → score on held-out documents
  against a linear baseline → race each survivor against a **twin**
  (identical run, layer off — rules out the lottery ticket) and a
  **sham** (scrambled information — rules out raw capacity) → a
  causality check → insert behind the gate.
- **Evolution is mandatory.** A lookup layer present from step 0
  reaches 55% recall on out-of-window variables; the same layer
  inserted halfway reaches 72% — despite near-identical loss curves.
  Present from the start, the network leans on it instead of building
  its own circuits; inserted after the skills exist, it composes with
  them. *Timing is a design variable.*
- **Jaggedness.** Ordinary training on CLRS-Text solves the input
  lengths it saw and fails the sizes in between (33%); code layers
  grown during training raise the in-between accuracy to 79% and the
  overall to 84% vs 55% — and twice the training does not close the
  gap. Code generalises by construction: a program applies the same
  rule at every length.
- **Closing the loop — the self-probe.** DeepSeek-V4.1-Flash, as the
  agent on itself: it fitted a linear probe on its own residual stream
  (seven deepest blocks) predicting whether the just-written answer
  is right, then wrapped it in a code layer that *controls sampling*:
  probe says wrong → discard, resample, up to three times. 64% → 71%
  on hidden contests. The probe rejects four in five wrong answers
  and one in fifteen right ones; it is not length (longest-solution
  restart gains <2 points); the knowledge appears only once the answer
  is written. The model finds where its own knowledge lives and acts
  on it.
- **Qwen writes its own policy.** Qwen3-8B, given probes on its own
  chain of thought, wrote its own stop/restart/reject policy in Python
  — 59% → 66%. The critical ingredient: before writing each policy it
  first writes **analysis code over its own graded attempts**
  (distributions of each signal on right vs wrong answers, separation
  tests, a candidate rule tried on the training problems), reads the
  output, then writes the policy. Without it, the right signals and
  guessed thresholds.
- **Looking ahead.** Alignment through code (code is readable and
  testable; weights are not); the tools are noisy (the phantom
  gradient shows *where*, not *what*); code layers need real kernels;
  recursive self-improvement as a growing library of intervention
  tools.

## Where fleet already lives the article's principles

- **Alignment through code, as a ruling.** The article's clearest
  argument — whatever moves into code becomes testable — is fleet's
  founding constraint: the write path is mechanism-constrained, no
  model-written memory prose, proposals are structured and gated
  (`improve.rs`'s module doc). Fleet's self-modification is *already*
  code, reviewable and reversible.
- **The objective test.** The article notes interpretability lacks an
  objective test and intervention supplies one. Fleet's eval corpus
  (`eval.rs`) is exactly that: a fixed, graded, deterministic suite —
  and the slice-12 source gate runs it as a step, so a source
  self-edit survives only if the corpus does not regress, with the
  scenario named.
- **The gate, in discrete form.** The article's zero-init learnable
  gate turns a discrete edit into a continuous one. Fleet's gate
  (fmt, clippy, build, test; apply-to-tree, revert on failure) is the
  discrete version — and the owner's ruling *requires* the discrete
  form: nothing applies silently. The article's mechanism is a
  lower-resolution version of the same idea, not a better one for
  fleet.
- **Named failure modes.** The article's clustering step ends by
  *naming the failure mode* from examples. Fleet's failure-shape
  taxonomy (fail-open, drops-the-row, bound-exceeded,
  claim-outran-evidence, fixed-width-clip, foreign-root) is exactly
  that naming, drawn from the ledger — and the shapes generalize by
  construction, the article's own criterion for code.
- **Evidence-validated intervention.** The dream pass (landed today)
  is the article's "objective test" applied to memory: a fact's
  transcript is the ground truth, sys1 judges the fact against it,
  and a measured bar gates the action. The article's Mt. Moon
  argument — a wrong note trusted by later sessions — is the failure
  the pass exists to catch.

## The gaps, as improvement areas

Each area names the article's finding, fleet's current state (with
this session's reads), and the candidate direction. These are dive
findings, not decisions.

### 1. The phantom gradient — a per-cell demand map

The article's sharpest instrument is a *demand map*: per cell, per
checkpoint, where is the system failing to learn. Fleet grades in
aggregate. The judge beat reads a day's `route` rows and lands
`route_judged` rows; the delegate beat grades a day's delegations; the
eval corpus grades twenty fixed scenarios; the dream pass judges
sourced facts. Every one of those rows is already per-decision and
per-scenario — the raw material for a demand map exists in the calib
stream today. What does not exist: the aggregation. Which decision
classes (workloads, providers, shapes) stay high-error across runs?
Which scenarios fail flat? The article's rule — cells that are still
improving are left to gradient descent; high-flat cells are the
targets — is computable from fleet's own rows and unowned. Candidate:
a demand view over the calib stream (per class, per checkpoint run),
feeding the beats' priorities.

### 2. Failure clustering and the shape proposals

The article's recipe clusters wrong tokens by direction, reads the
clusters' examples, and names the failure mode. Fleet names failure
modes by hand: a ledger row's class is filed by the session that hit
it, and the shape taxonomy grows by human decision. The raw material
for the clustering step is already recorded — the judge's verdicts,
the dream pass's contradicted facts, the ledger's own rows. What's
missing is the machinery: group the fresh failures, surface the
cluster's examples, propose a shape tag for the mode. The dream
pass's `contradicted` verdicts are the cleanest feed: a fact the
evidence opposes is a named failure looking for its class. Candidate:
a clustering step over the graded rows that proposes shape tags for
the modes it finds (into the improve proposal store, owner-applied).

### 3. The self-probe that acts per decision

DeepSeek's loop — probe its own output, resample what the probe
predicts wrong — is the article's most transferable mechanism. Fleet's
probes are pre-decision (the route's sys1 judgment before a walk) or
post-hoc (the judge, the delegate, the eval corpus). The one
per-decision probe that acts is the **curate judgment**: a steer is
judged against the last curated instruction, and a confident verdict
re-ranks the memory block before the next request. That is the
pattern; the article generalizes it. Candidate directions, in fleet's
idiom: a probe on a run's own outputs (the answer's or the decision's
confidence) that triggers the fleet equivalent of resampling —
re-route, re-run, or ask — per decision rather than per day.

### 4. The twin and the sham — racing proposals

The article races every code candidate against a twin (identical run,
layer off) and a sham (scrambled information), and drops any candidate
a sham matches. Fleet's source gate is a before/after on the same
suite: apply, run the gate, revert on failure. That rules out
*breakage*; it does not rule out *placebo* — a proposal that passes
the gate because the suite is weak, or because any change would pass.
The eval corpus is the instrument: a proposal's effect could be raced
against a no-op apply on held-out scenarios (the twin) and against a
scrambled patch (the sham). The article's methodology is a strict
upgrade over "the gate is green"; fleet has the corpus to run it.

### 5. Analysis before policy

Qwen's finding is the article's most practical: the model that writes
analysis code over its own graded attempts before writing its policy
dramatically outperforms the one that guesses thresholds. Fleet's
proposals carry a justification naming an outcome ("the session's
evidence, the failing test, or the graded outcome behind it" —
`improve.rs`), but the analysis is the drafting model's in-session
judgment; no machinery reads the graded rows and measures the
thresholds first. The dream pass's measured-set gate is the existing
instance of the pattern — score candidates on held-out facts before
any auto-landing — and the judge/delegate rows are the raw material
for the general version. Candidate: a proposal drafter that reads the
day's graded rows and produces the analysis (which classes fail,
how consistently, what threshold separates them) *before* the
proposal's acceptance criteria are written.

### 6. Timing — intervene at demand, not at day's end

The evolution-is-mandatory experiment: the same layer inserted
halfway beats the one present from step 0, because the network has
already built its own skills and the layer composes with them. Fleet's
interventions are batched — a day's grading in the beats, an hourly
dream pass, a session-close hygiene — and the improve proposals apply
at the owner's convenience. The article's finding is about *when* an
intervention lands relative to the system's own development. Fleet's
analog: a correction applied while the context that produced it is
still live (the session, the run) composes with skills the system
already has; the same correction applied a day later lands on a
system that has moved. The dream pass's hourly cadence is the right
shape; the beats' day-end cadence is the batch the finding argues
against. Candidate: the beats' cadence as a measured question, not a
fixed convention — and the session-close hygiene's consolidation as
the precedent for at-demand intervention.

### 7. Jaggedness — test the in-betweens

The article's jaggedness result: solve the lengths you trained on,
fail the sizes between them; code generalizes by construction. Fleet
measures itself on twenty fixed scenarios and the shapes taxonomy is
its generalization mechanism — a shape tag applies across instances,
the same "by construction" argument. What fleet does not do is *gap
testing*: scenarios between the fixed ones, decision classes between
the graded ones. The calib stream's per-decision rows make the
in-betweens measurable (a workload or provider class with no eval
coverage is a gap by definition). Candidate: derive gap scenarios
from the rows' classes — the eval corpus's own taxonomy, extended
with the classes the stream shows but the corpus does not cover.

### 8. The growing library of intervention tools

The article's coda: every probe and code layer discovered during
training joins an ever-growing library; a model that improves both
how it trains and the tools it trains with is on the direct path to
recursive self-improvement. Fleet's library is the home-side proposal
and skill stores — proposals that landed are not mined for reusable
tools. The improve proposal is a *change*; the article's unit is a
*capability*. A proposal that landed and proved itself (the gate
green, the corpus unregressed) is a candidate tool: a check, a
pattern, a threshold — the dream pass's own history (a measured gate
earned by a fixture corpus) is the precedent. Candidate: a
capability view over the applied proposals — what did the system learn
to do about itself — feeding the skill drafter's feedstock.

## The owner-ruling tensions, named

- **Autonomy.** The article's agent acts inside the training loop
  without a human in it. Fleet's ruling is the opposite and explicit:
  the `improve` tool *files*, the owner's `fleet improve apply`
  applies, destructive changes wait for explicit approval. Every
  candidate above is shaped to that ruling — proposals, measured
  gates, owner-applied — and none of them proposes otherwise.
- **Continuous vs discrete.** The zero-init gate is a beautiful
  mechanism for a system whose optimizer can open it. Fleet's
  "optimizer" is the owner plus the local gate; the discrete form is
  the honest one at this autonomy level. The article's gate informs
  the *measured-set* pattern (a candidate must prove itself before it
  acts), not the autonomy.

## What this dive does not settle

Which of the eight areas is worth a plan and in what order; whether
the demand map belongs in the calib surface or a beat of its own; the
cost of racing proposals against twins and shams at fleet's corpus
size; and whether the article's timing finding survives contact with
fleet's actual workload mix. Each is a plan's question — and area 1
(the demand map) is the one the other seven would read from, which is
the article's own argument for the phantom gradient.
