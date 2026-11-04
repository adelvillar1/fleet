---
name: a-fixture-corpus-flatters-a-retriever
description: "feedback 2026-10-23 — the tool-search floor shipped at 0.35 after passing a 24-query corpus built from synthetic cards with one distinctive token each; against the live embedder a realistic catalog put no-match requests at 0.60–0.67, so the floor could never abstain. A threshold is a measurement, and the corpus it is measured on decides the answer"
type: feedback
date: 2026-11-04
basis: inferred
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

The tool surface shipped `FLOOR = 0.35` and `MARGIN = 0.02` for
`tool_search`, "tested at every layer" — but every test ran a deterministic
double over a fixture catalog of 40 cards built as
`"tool c{i} — capability token{i} work"` with queries `"do token{i} now"`.
That corpus is a clean separation by construction, and I wrote it that way
because it made the activation path easy to assert. When the same pipeline
was finally pointed at the live embedding server the numbers were different
in kind, not degree:

- realistic tool cards, real requests: **0.691–0.802**
- requests that should match nothing: **0.602–0.668**
- two tools with near-identical meaning: **0.009–0.010** apart

Against that, 0.35 is unreachable at the bottom and 0.02 is the same order as
the near-duplicate gap. The abstention behavior — the whole point of
"no tool fits" — could never have fired in production, and no test could
have said so.

**Why:** a synthetic corpus encodes the answer into the input. One rare token
per document makes retrieval look like lookup; it measures the scorer's
plumbing, never its judgment. A fixture embedder compounds it: the threshold
is only as real as the vectors it was tuned on.

**How to apply:** when a number decides behavior (a floor, a margin, a cap, a
confidence gate), it is a measurement and needs a measured corpus — real
descriptions from the real catalog, real requests, and explicit no-match
queries — run against the real model, with the boundary reported (min of the
true set, max of the negative set, the gap). Keep the synthetic corpus for CI
asserting the pipeline, and label it as such. Ship a script that reproduces
the calibration so the next catalog change can move the constant. And state
the limitation the data reveals rather than smoothing it over — here, that
without the classifier's class text, a query containing the word "TODO"
ranks the *todo* tool first.

Related: [[embeddings-cannot-label-a-session-project]] (the same embedder's
soft cosines, measured three days earlier for a different feature),
[[fleet-tool-surface-analysis-2026-10-23]],
[[a-green-gate-covers-what-was-built-not-what-was-left-out]].
