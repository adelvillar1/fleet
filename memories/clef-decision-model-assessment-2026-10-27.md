---
name: clef-decision-model-assessment-2026-10-27
description: "2026-10-27 clef arc: assessment → owner ruling to rationalize production → clef provider LANDED in sys1 (10 tests) → shadow A/B over 25 real route states FAILED the promotion gate (32% workload / 56% verdict agreement; all 16 standard_code states downgraded to quick) → NOT promoted; production chain stays glide,drex,jev + d1; clef is a registered bench candidate with a named deficiency. Owner: 'the bench is for giving people the choice' — the bench stays undefined; production is what rationalizes."
type: project
date: 2026-11-05
basis: confirmed-by
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

2026-10-27 owner ask: compare https://ollama.com/library/clef to the sys1/fleet model options, goal = rationalize the model count for maintainability.

**Clef:** Cloudflare's 27B decision model (Qwen3.8-27B fine-tune), Apache-2.0, served through Ollama's `/v1/systemone` — the SAME System One wire sys1's d1 provider speaks (providers.py:560, "the TypeSafe System One API as implemented by Liquid"). Non-autoregressive: typed questions over a state → typed answers with probabilities, one joint pass, text+images. Beats Jev on most of the community Decision Index (~209ms vs 524ms median claimed), loses ANLI. Smallest local tag 18GB; `clef:cloud` = 64K ctx, per-input-token billing, input capped at 65,536 tokens on cloud.

**The estate inventory (2026-10-27, verified live):** decision = Liquid AI d1 via sys1 (vision-capable, on every ask and every fleet route); fleet workloads = token-plan/qwen3.8-flash, xiaomi-mimo/mimo-v2.6-flash, xiaomi-mimo/mimo-v2.6-pro, zai-coding-plan/GLM-5.3-Flash (routing.json, gates.min_confidence 0.72); blend1 generate = lfm2.5-24b-a2b via Liquid; critics = qwen3.8-flash (TOKEN_PLAN_API_KEY) + glm-5.3-flash (OLLAMA_API_KEY); embeddings = EmbeddingGemma-2 local llama-server :8901 (768d); tabular = TabPFN v3.5 hosted (Prior Labs, max 160 classes); voice = gen1 speak cascade (dashscope/stepfun); validate = GLiDE (Fastino) → d1 escalation; **escalation leg UNCONFIGURED** (named skip). Ollama installed (/usr/local/bin/ollama), no daemon, no local models.

**The assessment:** clef competes for exactly ONE seat — the decision plane (same wire, same job, multimodal matches d1's vision). It cannot touch the workload roster (it does not generate), embeddings, tabular, or voice. **16GB M4 — local clef does not fit in any quantization**; clef:cloud is the viable shape.

**HOSTING CORRECTION (owner): "clef is cloud hosted by ollama"** — Cloudflare made the MODEL; Ollama's own cloud hosts the cloud variant and the `/v1/systemone` endpoint. The provider rides the **same OLLAMA_API_KEY account the glm-5.3-flash critic already uses**.

**The fuller System One roster (owner: "what about replacing other system one models, like jev, julia, etc?")** — sys1 registers Decide, Jev, local GLiNER2, ModernBERT on import; a candidate bench of CLM-8B, openjev, Kev, Tev1, Laya; the routed chain is **`glide,drex,jev`** (api.py:149); `jev_model: jev-1.13.0` via api.typesafe.ai/v1/systemone. Jev = TypeSafe AI's flagship (choice/score/noul, RLCD-trained, "the only published-in-the-open calibrated probability" in the family). The **Jev Decision Index is a COMMUNITY leaderboard** (multimodalart's HF space, ~43 evals, 230+ models; standings seen: Torchcast Decision 27B 70.2%, Jev 1.1 66.3%) — NOT Cloudflare's own. Clef claims #1 (beat Jev 3 of 4 TypeSafe evals by 0.3–2.9 points). Ecosystem caveat: a decision model was demonstrated jailbreakable for ~50 cents.

**OWNER CORRECTION: "we don't have any local models for system one"** — every System One seat is hosted (Jev→TypeSafe, d1→Liquid, glide→Fastino, drex hosted); the local GLiNER2/ModernBERT providers are extraction/classification, NOT System One. The decision plane is 100% dependent on external APIs.

**OWNER RULING (the rationalization):** "the bench is for giving people the choice and not for me to decide, but my decision is that we need to be smarter about system 1 models, because I doubt we need as many as we have today" + "leave the bench undefined as is; rationalize what is currently in production; use clef where it makes more sense and replace the ones where clef fits better given that it has a wider functionality footprint." The measured baseline: the route cache held **77 real states — 75% escalate past tier 1** (verdicts escalate 58/pass 19; workloads quick 39/standard_code 35; execution single 76/swarm 1; median confidence 0.67, 45/77 under the 0.72 gate) — every tier carries real load.

## The implementation (plan approved, C0–C4)

**C0 LANDED — the clef provider in sys1.** `_clef_wire` (a `_liquid_wire` clone: plain-string state, `Head.to_drex_head()` questions, images passthrough up to 4, `clef_max_chars` 150K, usage/cost telemetry, `parse_liquid`), config keys `clef_api_url=https://ollama.com/v1/systemone` / `clef_model=clef:cloud` / `clef_api_key_env=OLLAMA_API_KEY`, `_DEFAULT_FLOORS["clef"]=0.7`, `_requires_key`, jev-family head-set inheritance (tasks.py), CORE pin updated in test_provider_flags.py, `TestClefProvider` (10 tests). sys1 suite green. Wire-factory architecture: adding a jev-family provider = config keys + wire factory + register() + parser + floor + head-set tuple + requires-key + routing note + tests (9 touchpoints).

**C1 RAN — the shadow A/B FAILED the promotion gate.** `scripts/ab_clef_shadow.py`: joined 25/77 cache entries to the session logs (scan ALL user records — follow-up runs judge their own task; the other 52 predate the logs or ran off-session), replayed each through clef:cloud live. The wire worked first call (http 200, structured answers). **Result: workload agreement 32% (8/25), verdict agreement 56% (14/25) — systematic, not noise: ALL 16 cached `standard_code` states classified `quick` under clef, and its pass rate doubled the chain's escalate discipline.** Latency 400–880ms per state (the cloud round-trip dominates the index's 209ms median); clef:cloud's response carries no cost field (cost unmeasured). The gate (≥90% workload / ≥85% verdict) failed decisively.

**Decision: NOT PROMOTED.** Production unchanged — `glide,drex,jev` on the chain, d1 on the fleet/blend1 seats, the route cache intact. Clef stays a **registered, live-verified bench candidate** with a named deficiency on the house corpus: coding-agent workload tiering (the index's classification domains are not the house's tiers). Re-entry path: a criteria-encoding iteration on the workload head, or an escalate-class A/B against jev alone. C2 (promotion), C3 (vision seat), C4 (retirement) were gated off by C1's evidence and never executed — the plan's own gate did its job.

**Corrections the plan recorded:** (1) the escalation leg is a generative **chat** lane (BLEND1_ESCALATION_MODEL/BASE, OpenAI-compatible, S2_SYSTEM_TEMPLATE) — clef is non-autoregressive and can NEVER fill it; (2) the raw decision states are stored NOWHERE (route rows carry `input_hash` "never the text", by design) — the A/B join via the session logs is exact because `judge()` hashed the identical string; (3) fleet's judge sends the RAW task text as the state (`judge_live: text: task`) — the route task description's "compact state" is aspirational doc.

Artifacts: sys1 `docs/ab/2026-10-27-clef-shadow.md` (every divergence listed) and `docs/plans/2026-10-27-clef-provider.md`. The sys1 tree carries the provider + tests + CORE pin uncommitted (owner commits). Related: [[sys1-pool-is-all-hosted-no-extraction-wire]], [[blend1-is-the-latency-triad]], [[sem1-probe-shadow-second-call]], [[api-key-env-file-map]].
