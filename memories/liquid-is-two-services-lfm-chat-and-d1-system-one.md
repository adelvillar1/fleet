---
name: liquid-is-two-services-lfm-chat-and-d1-system-one
description: "reference 2026-11-01 — Liquid exposes TWO things on different endpoints, and fleet only ever wants the chat one as a provider; d1 (System One) is sys1's hosted decision model at /decisions/v1/systemone and never belongs in the provider model list"
type: reference
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

Asked "which model from liquid? system 1 or lfm?", the answer is **lfm, not
System One** — they are different services behind the same vendor:

- **lfm** — Liquid's OpenAI-compatible chat surface,
  `https://api.liquid.ai/openai/v1`. Its `/models` listing (live 2026-11-01,
  five entries) is `lfm2.5-24b-a2b`, `lfm3-1.7b`, `lfm3-audio-2b`,
  `lfm3-omni-2b`, `lfm3-vl-2b` — published in that order, `lfm2.5-24b-a2b`
  first (the 24B the composed blend1 lane synthesizes with). None of them
  declares a context window, so their settings rows stay bare.
- **d1 / System One** — the hosted *decision* model at
  `/decisions/v1/systemone`, what **sys1** calls for its judgments
  (`liquid_model: "d1"` in sys1's config). It is not a chat model, so it has
  no place in a provider's model list.

**Why:** the fleet roster's `liquid` entry is `lfm2.5-24b-a2b` and the vendor
name invites the conflation; picking d1 as the provider's model would route
chat traffic at a decisions endpoint.

**How to apply:** when asked about "liquid's model", name the surface — chat
(lfm, `/openai/v1`) versus decisions (d1/System One, `/decisions/v1/...`) —
before naming an id. And check that a provider's `base_url` is the ORIGIN:
the real home's `liquid` entry carried
`https://api.liquid.ai/openai/v1/chat/completions` (the full endpoint pasted
as the base), so `/models` and every chat call hit a doubled path and 404'd;
setting it to `/openai/v1` and reloading brought the five models in (the home
then read 76). A live chat call on that base with `lfm2.5-24b-a2b` answered
normally (90 completion tokens, 79 of them reasoning, cost $0.000064), so the
corrected entry is proven for chat and not only for listing. The same 404 was
why `reload every provider` reported liquid as not answering — see [[a-list-that-came-from-the-roster-is-not-the-providers-catalog]].
