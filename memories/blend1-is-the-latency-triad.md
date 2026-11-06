---
name: blend1-is-the-latency-triad
description: "2026-10-17 — the owner proposed building an ultra-low-latency agentic triad (EmbeddingGemma 2 vectors + Jev/GLiDE System-1 deterministic decisioning + a small LLM for System-2 text) with five visual use cases; the triad ALREADY EXISTS as blend1; the \"layout geometry is the gap\" claim I made three times was wrong — deck::review already measures it"
type: project
date: 2026-11-04
basis: confirmed-by
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

The owner proposed (2026-10-17) combining **EmbeddingGemma 2** (vector/semantic
space), **Jev/GLiDE** (System-1 deterministic decisioning) and a **small LLM**
(System-2 local text manipulation) into "an ultra-low latency, highly resilient
agentic harness", with five high-value use cases: zero-shot brand-aware theme
injection, cross-asset sourcing and visual assembly, real-time spatial content
squishing and layout swapping, sub-second hybrid context filtering via Matryoshka
truncation, and adaptive content clustering for a single infographic. They asked
whether to drill into the prompt structure for clean layout JSON or the vector
index schema for cross-modal assets.

**The triad already exists as `blend1`.** Per `docs/features/blend1.md`: one
`ask()` runs the **d1 router (sys1) solo first** as the front door; then the fact
lanes — the local **EmbeddingGemma-2 semantic lane (sem1)** and the Prior Labs
**TabPFN** lane (sdm1) — run concurrently; and **Liquid's LFM2.5-24B-A2B
synthesizes only when the router clears `ROUTE_FLOOR` (0.70)**. A **GLiDE
validation gate** then rates the synthesized answer against the ask and the
givens before it ships, with one bounded retry and a critic chain behind it. And
Jev is in the lane registry too — the mapping is `glide → FASTINO, jev →
TYPESAFE, drex → DREX, julia → SUPERSONIC, liquid → LIQUID`. So sys1, GLiDE, Jev
and Liquid LFM2.5 are all present and vendored; the owner was describing a lane
that runs today, not one to build.

**The real gap is a layout engine with measurable boxes.** Use cases 1, 3 and 5
all need "does this text fit this slot" answered *as a number*, and fleet cannot
answer it: `deck::Layout` is a shape enum and `deck::Component` is content, with
**no geometry anywhere**. So every downstream decision is a guess. The proposed
next increment: give each layout real geometry — slots with a width in characters
and a height in lines — and have the planner **measure** drafted content against
them, refusing or re-routing *before* render, deterministically.

**CORRECTION (2026-10-17, later the same day): that paragraph is wrong, and I
repeated it three times.** The geometry already exists as `deck::review`: it
estimates each component's lines against a box's using **named nominal
constants** (`LINE_PX`, `BOX_PX`, `CHARS_PER_LINE`, `HEADLINE_HOLDS`) and reports
`Overfull` / `Unbalanced` / `HeadlineOverflow`, and it is **wired** — `run_deck`
runs parse → `check` (the consultant constraints, the floor) → `review` (the
ceiling) → `render`. So "no geometry anywhere" was false when written, and
asserting it three times without opening the co-edited file is the failure
[[read-a-coedited-file-before-adding-to-it]] names. What the grader genuinely
does *not* yet do is a **re-layout ladder** — on overflow it reports the fault
but offers nowhere roomier to go; a `roomier()` on `Layout` (a `SplitTwoColumn`
or `Comparison` node that overflows suggests `FullBleed`) folded into
`LayoutFault` as a variant is the small addition worth making. Full record in
[[fleet-deck-visual-pipeline]].

**Viability verdict (2026-10-17), after the small-LLM trial.** Asked directly
whether "this approach" is viable, the honest answer is two-part. **The local
small LLM is not viable on this hardware** — not because the model failed
(Ternary-Bonsai-2B works on the PrismML fork, is fast at 0.2–0.5s on short
prompts and good at generation) but because an 11K-token read took 20.5s and the
owner lacks the resources to run it faster. That is **conditional, not
permanent**: better hardware or a smaller/faster model reopens it. What is *not*
conditional is the fork requirement — upstream llama.cpp genuinely cannot load
the packs that make the model small. **The broader approach is viable and mostly
already built**: the triad (blend1), the TinyFish web leg, the deck pipeline,
the contrast guard, the credentials vault, the file pane, the action card. See
[[ternary-bonsai-needs-the-fork]] for the trial and
[[blend1-web-leg-and-corpus]] for the remaining gaps.

**A Matryoshka prefilter is nearly free** (use case 4): `HttpEmbedder::
vector_info()` already reports the dimensionality, and truncating a vector to 128
is a slice. It is a small addition to the recall path, not a project. An asset
library to search (use cases 1 and 2) is the one genuinely new corpus.

**One part of the blueprint was pushed back on.** "The Small LLM truncates
strings to fit `max_chars`" spends a model call on arithmetic: once GLiDE has
computed the number, cutting a string to it is `&s[..n]`. The loop is right, the
truncator is the wrong primitive for the final cut. The honest split: the LLM
writes concisely **up front** (which the pyramid and MECE constraints already
force, since a headline that runs long is refused), the spatial check
**verifies** deterministically, and on overflow either **re-draft** (the model,
meaningfully — rewrite for concision) or **re-layout** (deterministic — swap to a
roomier template). Answered the owner's either/or question with **neither
first**: the prompt structure and the index schema are both designing against
constraints nothing can yet verify, so geometry comes before either.

**Why:** the finding collapsed a large proposed build into a small one, and it
is the same lesson as [[name-the-component-that-has-the-capability]] — the
capability was one component over. Recording the exact lane mapping saves the
next session from re-deriving that blend1 *is* the triad.

**How to apply:** when the owner proposes an architecture, first check whether
the lane registry and `docs/features/*.md` already describe it. Build the
measurement primitive before the model call that would replace it. Related:
[[fleet-deck-visual-pipeline]], [[fleet-browser-lane-architecture]],
[[fleet-routes-through-its-own-router]].
