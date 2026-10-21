# The lanes: what decides, what measures, what indexes, what renders, and what speaks

Why Fleet is built around five specialized model services and one
composition of them, and why the language model is the last component
to speak rather than the only one that ever runs.

## The goal

An LLM is one component, not the harness. Every piece of work Fleet
accepts goes to the component that can actually do it: a **decision** to
the models that decide, a **measurement** to the models that compute it,
a **similarity** to geometry, a **render** to generators, and the
**speaking** — turning evidence into words the owner can act on — to the
language model, which is the only one of the five that has any business
generating language.

The house sentence says it: *sys1 judges what the work says, sdm1 scores
what the work measures, sem1 indexes what the work looks like, gen1
renders it.* The language model is not in that sentence on purpose.

## The five lanes

| Lane | Owns | Returns | Why it beats the LLM there |
|---|---|---|---|
| **sys1** — judgment | classifying a request, a diff, a dependency set, a document's drift; grading the day's decisions and delegations | a label from a **closed set**, with a confidence and a verdict | it cannot invent a category; a low-confidence answer is a named fallback, and a failed one fails open as `error` |
| **sdm1** — tabular | a number from rows: classification or regression over a table | a **computed score with an interval** | arithmetic is not generated; "LLMs do not do math" is the whole reason this lane exists |
| **sem1** — embeddings | similarity and recall: which fact, which skill, which session means this | **vectors in, vectors out — never verdicts** | geometry is exact and fast; asking a model to rank by meaning is asking it to guess |
| **gen1** — media | images the gates judge, and the judging of images against a brief | pixels, and a verdict on pixels | a model asked to reason about a picture *described in words* misses the artifact; one that sees it does not |
| **blend1** — grounded composition | the end-to-end ask that composes all three siblings under one answer | an answer whose reasoning **stands on computed facts** | it is the proof of the whole claim, below |

### Where the owner meets them

sys1 is the most visible: it is the judge behind the router's workload
verdicts (quick, standard code, hard, prose, deep context — and the
chain each gets), behind the delegation graph's order, behind
`fleet judge` grading the day's routes and `fleet delegate` grading its
delegations, and behind the Router board's "ask sys1 for a judgment."
sem1 is behind memory's recall, the skills search, and the sessions
ranking — the numbers on the Memory surface are its work. sdm1 and gen1
are behind the composed ask and the document path — the places a number
or a picture is the answer. blend1 is the lane the `composed` workload
targets: the grounding provider for work that needs facts before prose.

## How the lanes control hallucinations

Not by instructing the model to be careful — by construction:

1. **Closed output spaces.** A decision model answers from a declared
   label set or refuses; a measurement is a number; a similarity is a
   vector. None of these can produce a fluent invention, because none
   of them produces fluent anything. The failure mode of a lane is a
   named absence, never a confident mistake.
2. **Confidence gates, in the path rather than in the prompt.** The
   router's gate routes a verdict below the owner's floor to the default
   workload and records that it did; blend1's floor means the language
   model **speaks only when the decision model clears it** — below the
   floor the answer is the evidence itself, not prose about it.
3. **The generative leg is gated, bounded, and validated.** In blend1
   the synthesizer runs last, its answer is rated by a validation gate
   before it ships, there is one bounded retry, and a critic chain
   behind that. The model's claim is treated as a hypothesis with a
   price, not as the output.
4. **Everything that served is graded.** The calibration stream records
   every route, screening and run ending, and a gate decides — on
   evidence, not assumption — whether the day's outcomes may move the
   failover. Autonomy is earned, and the Calibration surface is where
   you watch it being earned.

## How the lanes make smaller models smarter

The harness's ordinary runs are the fast, cheap models — flash-tier —
and that is only safe because the cognitive work the language model is
worst at is done by components that are cheap, fast, and reliable
beside it:

- **Judgment** goes to decision models whose verdicts are calibrated and
  gateable, so the router does not need the strongest model to choose
  the strongest model.
- **Measurement** goes to tabular models, so no reasoning capacity is
  spent on arithmetic that would be guessed anyway.
- **Recall** goes to embedding geometry, warm and local, so context
  arrives as retrieved fact rather than as model memory.
- **Rendering and seeing** go to generators, so the language model
  never has to describe what it has not seen.

What remains for the language model is synthesis and speech: taking the
decided, measured, retrieved and rendered parts and saying what they
mean. That is why a small model, standing on the lanes, is trusted with
the work the same model would fail at alone.

## The router is the dispatcher that makes it one system

The router's job is to send each piece of work to the right chain by its
difficulty — and now, since the owner's 2026-10-21 rulings, to let the
judged workload steer the model tier for every route but the leading
session, with the failover tail behind it and the calibration gate
watching. The lanes are the reason the router's choice is meaningful at
all: it is not choosing between flavors of the same thing, it is
choosing which kind of mind answers.
