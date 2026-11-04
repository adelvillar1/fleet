---
name: ternary-bonsai-needs-the-fork
description: "prism-ml/Ternary-Bonsai-2-2B works on the PrismML llama.cpp fork (the owner runs one on 127.0.0.1:1337) — thinking must be OFF (empty content past its budget with it on, 0.2-0.3s with it off); good at generation, bad at judgment; the constraint is latency, not context"
type: reference
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

Tried the owner's suggested small-LLM candidate for the deck pipeline's text
layer: `prism-ml/Ternary-Bonsai-2-2B-gguf` (~1.88B LM params + 0.33B vision
tower, Qwen3.5-2B base, Apache 2.0, 256k context, ternary {−1,0,+1} weights).

**It works — on the PrismML fork.** Stock homebrew llama.cpp 0.6.0 was tried
first and fails: PQ2_0 and PTQ1_0 are refused at load (`failed to read tensor
info` — unknown quant type), and Q2_0 loads but produces multilingual word
salad on every prompt, including "The capital of France is" at temperature 0.
Upstream's standard dequantization misreads weights the card says are
"consumed directly by custom kernels and never expanded back to FP16". The
owner then ran the fork (`PrismML-Eng/llama.cpp`, branch `prism`) via Atomic
Chat on **http://127.0.0.1:1337/v1**, serving the PQ2_0 pack — and it answers
correctly.

**Thinking must be off for short constrained tasks, and temperature does not
rescue it.** Tested as a 2x2 matrix on the same prompt (compress to <=60
chars, keep every number), because the first bad run confounded thinking with
temperature=0 — exactly the card's "avoid greedy decoding with thinking on"
warning:

| thinking | temp | result |
|---|---|---|
| on | 0.0 | empty content, 1323 chars of reasoning, 400-token cap hit, 6.1s |
| on | 1.0 (card's own rec) | empty content, 1558 chars of reasoning, cap hit, 5.7s |
| off | 0.0 / 0.2 / 0.7 | all correct, 13-14 tokens, 0.2-0.3s |

So the card's warning describes a real failure (the reasoning loop exhausts
the budget) but its remedy does not fix this task class — even at the
recommended temperature the model deliberates past its budget on a job that
warrants none. `chat_template_kwargs: {"enable_thinking": false}` in the
request body is the load-bearing detail; `reasoning_effort: "none"` works too.

**Context is not the constraint; latency is.** The owner's endpoint accepted
an 11,230-token prompt (55KB) and answered correctly, but took 20.5s against
0.2-0.3s for short prompts. So it suits short generation and is wrong for
long-context reading, which the deterministic paths already cover instantly.

**Measured on the deck pipeline's actual tasks (thinking off, temp 0.2):**

| task | latency | result |
|---|---|---|
| compress to ≤60 chars, keep numbers | 0.47s / 13 tok | 64 chars, every number kept — 4 over budget |
| bullet ≤80 chars with **bold** | 0.38s / 18 tok | exactly 80 chars, bolded — correct |
| "Market Analysis 2026" conclusion or topic? | 0.17s | **CONCLUSION** — wrong, it is a topic |
| "Q3 growth came from regional expansion" conclusion or topic? | 0.17s | CONCLUSION — correct |
| MECE: "54% increase in APAC sales velocity" vs "APAC sales velocity rose 54%" | 0.19s | **DIFFERENT** — wrong, same claim |

**The division this confirms:** it is good at *generation* (draft, compress,
rewrite — fast and near the budget) and unreliable at *judgment* (pyramid,
MECE). Fleet already has deterministic judgment that is correct on both cases
the model gets wrong: `deck::check` refuses a topic headline and scores MECE
overlap at 0.6 on content words. So the model belongs behind those checks,
never in front of them — which is the architecture argued for in the deck
work, now measured rather than asserted.

**How to apply:** if it is wired in, the request must carry
`chat_template_kwargs: {"enable_thinking": false}` — `ChatRequest` in
`src/api.rs` has no extension field today, so that is the one change needed.
The lane/provider shape is otherwise unchanged: a declared `base_url` drops it
in without touching the harness. Judge candidates on this task suite and the
latency curve, not on download count or parameter size.

**On the "8K window":** the owner stated it, and it is below what the running
server actually accepts (an 11,230-token prompt went through fine). The card
declares 262,144. The honest position is three different numbers for three
different things — the card's declaration, the owner's observed effective
window, and what the server will take — and the one that matters for design is
the measured latency curve, not any of the context figures.

**Decision (owner, 2026-10-17): set aside for now.** The owner ruled to stay on
the Liquid AI API for the small-LLM role — not a quality judgment but a resource
one: "I don't have enough resources to run a model at faster speeds", which the
20.5s/11K-token latency measurement supports directly. The text layer stays on
LFM2.5-24B-A2B through blend1's generative lane, and no local GGUF model enters
fleet's critical path. Revisit only if local inference gets materially faster;
what would need re-measuring is the latency curve, not the quality — the
generation tasks already passed.

Related: [[blend1-is-the-latency-triad]], [[fleet-deck-visual-pipeline]],
[[your-test-harness-caps-are-not-the-models]],
[[ternary-bonsai-2b-declined-for-small-llm]],
[[fleet-routes-through-its-own-router]].
