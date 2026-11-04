---
name: ternary-bonsai-2b-declined-for-small-llm
description: "prism-ml/Ternary-Bonsai-2-2B-gguf for fleet's small-LLM role — assessed from the card and declined, then TRIED on the PrismML fork and found to work well; finally set aside 2026-10-17 for resource reasons (20.5s for an 11K-token read), with the text layer staying on Liquid's hosted API"
type: project
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

The owner proposed `prism-ml/Ternary-Bonsai-2-2B-gguf` (HuggingFace) as the small
local LLM for the visual pipeline's System-2 text work. Card read 2026-10-17.

> **REVERSED the same day.** The owner ran the PrismML fork themselves
> (Atomic Chat on `http://127.0.0.1:1337/v1`, serving the PQ2_0 pack upstream
> refuses) and it answers correctly and fast. The three reasons below are each
> still factually true — the fork IS required, it IS a VLM, thinking mode IS
> wrong for this job — but "declined" was the wrong conclusion, because the
> fork requirement turned out to be a solved problem on this machine rather
> than a blocker. **Read [[ternary-bonsai-needs-the-fork]] for the measured
> verdict; the rulings below still stand, the decline does not.**

**What it is:** ~1.88B LM params plus a ~0.33B vision tower, derived from
Qwen3.5-2B, Apache 2.0, 262,144-token context (18 linear-attention + 6
full-attention layers). The LM weights are true ternary — {−1, 0, +1} with FP16
group scales and a blockwise Hadamard rotation folded in. Three GGUF packings of
the same weights: PTQ1_0 ≈ 0.43 GB, PQ2_0 ≈ 0.52 GB, Q2_0 ≈ 0.55 GB.

**Why declined — three reasons, each verified against the card:**

1. **The runtime is a fork.** The card is explicit that upstream llama.cpp runs
   only Q2_0 and *rejects PQ2_0 and PTQ1_0 as unknown types*. The two smaller
   packs need `PrismML-Eng/llama.cpp` (branch `prism`) with custom ternary
   kernels that consume the trits directly and never expand back to FP16. So
   the 429 MB pack — the one that makes the size argument — will not load on
   the llama-server fleet already runs sem1 on. That puts a third-party fork's
   kernel correctness in the path every deck depends on.
2. **It is a vision-language model.** You carry a vision tower, a VLM chat
   template and thinking-mode behaviour for a pure-text job. The mmproj is
   optional; the rest is not.
3. **Thinking mode fights the latency goal.** The card recommends temp 1.0,
   presence_penalty 1.0 and *at least 8,192 output tokens* in thinking mode. The
   role is "shorten this bullet to 60 characters" — a model that deliberates for
   8k tokens before answering is exactly wrong, and forcing non-thinking plus
   retuning sampling is real integration cost.

No benchmark table on the card; 10,865 downloads/month and 24 likes is a weak
signal for a critical-path model.

**The ruling — adopt the architecture, not the model:**

- **The small-LLM role should be a lane or provider with a declared `base_url`**
  — the shape sem1 already is (a loopback llama-server on 8901) and the shape
  `CandidateKind::Http` already carries — so any model drops in without touching
  the harness, and swapping is an afternoon.
- **Measure, don't trust download counts.** Fleet already has `src/eval.rs` and
  `fleet eval` with a scenario corpus and a gate probe. Write scenarios for the
  actual job — compress to N characters without losing the number; draft a
  bullet from a chunk that keeps MECE; refuse a headline that is not a
  conclusion — and score candidates against them. That turns "which small
  model" from taste into measurement.
- **Ask whether the text layer needs to be local at all.** The layers that
  genuinely win from local are already local — sys1's decisioning, sem1's
  vectors, GLiDE's deterministic gate. Text *generation* is where the roster
  already has fast hosted options, with per-provider latency in the calibration
  stream. A local 2B has to beat those on measured quality at the target
  length, not on principle.

**The blocker is unchanged:** until a layout slot can say "I am 60 characters
wide at this font", no small model can be told what to fit — you would be
prompting against a number nothing computes. That is
[[blend1-is-the-latency-triad]]'s layout-geometry gap, and it is still first.

**Correction, after it was actually run (2026-10-17, later same day).** The
three reasons above were read off the card. The owner then ran the PrismML fork
(`http://127.0.0.1:1337/v1`) and the model **works**: correct answers, 0.2-0.5s
on short prompts, and good at the generation tasks — it hit an 80-character
budget exactly and kept every number compressing to 60. Thinking mode is
disabled per-request via `chat_template_kwargs: {"enable_thinking": false}`;
temperature does not rescue it (tested 0.0 and the card's own 1.0 — both
exhaust the budget and return empty). It is unreliable at *judgment* (it called
a topic headline a conclusion and scored two identical claims as different),
which fleet's `deck::check` already gets right deterministically.

So the fork requirement is real and verified, the VLM point is cosmetic (the
mmproj is optional), and the thinking problem has a working remedy. What
actually decided it was resources: the owner ruled "I don't have enough
resources to run a model at faster speeds", which the 20.5s-for-11K-tokens
latency measurement supports. The text layer stays on LFM2.5-24B-A2B through
blend1's generative lane. Full record in [[ternary-bonsai-needs-the-fork]].

**Why:** a plausible-sounding model with a real HuggingFace page and a small
download number is exactly the kind of thing that gets adopted on the size
argument alone. Recording the fork requirement — which the download count does
not surface — saves the next session from re-deriving it, and the
swappable-lane ruling keeps the decision reversible. The second lesson is the
one this file earned by being wrong first: a card is not a measurement, and the
conclusion here was reached without running the thing.

**How to apply:** before proposing a specific quantized model for a
latency-critical role, check (a) does it load in the llama.cpp fleet already
runs, (b) is it the right modality, (c) do its default sampling parameters
match the job's length budget. Then measure on the eval corpus rather than
arguing. Related: [[blend1-is-the-latency-triad]],
[[fleet-deck-visual-pipeline]], [[fleet-routes-through-its-own-router]].
