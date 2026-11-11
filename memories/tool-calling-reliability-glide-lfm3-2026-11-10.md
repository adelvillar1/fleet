---
name: tool-calling-reliability-glide-lfm3-2026-11-10
description: "project 2026-11-10 — tool/skill-selection deep dive → grounded-selection plan DRAFTED and gated (C0–C9, plan-gate 10/10 on --provider glide, arch PASS); owner corrected me twice (Fastino = sys1's chain head, Liquid = sys1 d1 + blend1 LFM2.5 lane) then ruled \"I'm the house, exploration is cheap, don't block on house-says\""
type: project
date: 2026-11-11
basis: inferred
tier: durable
shape: []
phrases: []
missing: []
premises: []
recall-count: 0
superseded-by:
valid-until:
source: zcode:fleet:content:285983102772d99aaed2c496d57acdf6f320aad7347d3891e9bb9e2378839d34
---

Owner ask 2026-11-10: make tool/skill selection faster and better — deep dive
on fleet's tool-calling implementation + whether a **GLiDE forward pass
(≤255 options) then Liquid AI LFM3-1.7B for the actual tool calling** makes a
more grounded process. The owner then said **"let's do them all, they all
have value and aggregated on top of each other they unlock compounded gains."**

**The plan: `docs/plans/2026-11-10-grounded-selection.md` (status: draft,
awaiting the owner's "looks good" to go active).** Originally ten criteria in
four phases — **then EXPANDED the same day to 20 criteria in six phases**
(Tier 1 prose + Tier 2 heuristic→head folded in, Tier 3 hosted-only, local
inference out by the owner's memory ruling) — see
[[grounded-selection-expanded-tier123-2026-11-10]]. The original four phases:

- **Phase 0 (C0, C1):** baseline first — wrong-tool rate, malformed-arg rate,
  repair-turn rate, mean declaration tokens/request into
  `docs/features/selection.md`; then the offline head-to-head replaying
  recorded turns through GLiDE/d1/Drex as `tool_route` heads, grading top-1
  accuracy + margin AUC. **Kill gate: no head beats baseline → program stops.**
- **Phase 1 (C2, C3):** catalog-as-choice top-k injection (tiering stays as
  the >255 fallback), skill selection by classification via `use_skill`.
- **Phase 2 (C4, C5):** verify-before-execute head verdict before
  `prepare_call`; repair leg = head-rename → LFM3 arg-regen (model id probed
  first, fallback lfm2.5-24b-a2b) → old error-result loop; margin threshold →
  clarifying question (pinned by a test that removes the threshold).
- **Phase 3 (C6–C9):** GLiDE+d1 ensemble disagreement, four heads in ONE
  `sys1_classify` call, speculative approval-warm on the first tool-call
  fragment, graded selection stream into calib rows + `fleet judge`.

New module `src/select.rs`; out of scope: LFM3 never authors a turn (repair
only), no new vendor/key, approvals untouched, no telemetry UI.
**Gate results: plan-gate coverage 10/10 via `--provider glide` (the default
jev chain still 402-dead), one residual SCOPE flag on the Approach intro —
conscious trade-off, not waived; arch-gate PASS with 31 conventions gaps (all
"tech doc doesn't cover the seam yet" = the linked artifacts' job).**

**Fleet's tool-calling map (subagent-verified, file:line):** the routed big
model emits the call; `parse_sse` (api.rs:1256) concatenates index-keyed
fragments (gapped index = Decode error, KI-020); arguments are never parsed
client-side — decode failures are model-visible results by design (api.rs:18-35).
`execute_calls` (agent.rs:4740) gates (approval, arg-JSON parse,
`tools::validate`, edit-needs-read, unknown-tool+Levenshtein), plans serial vs
concurrent waves (bound 3, ≤40/turn), and feeds every call a `tool_result` —
bad calls self-correct next turn, never abort the run. **A two-stage selector
already exists**: `tool_search` (agent.rs:6103) = sys1 classify → sem1 embed
rank → floor 0.68 / margin 0.02 → activate for 3 turns, plus workload
declaration tiering. Real failure history is argument-level (KI-016/019/020/
376/377/485); `tool_audit` records rates but **no baseline number exists yet**
(now C0).

**GLiDE** = Fastino Labs' decision model (v1.2 weights 2026-10-22, from
Qwen3.8-27B, also 0.4B; hosted `api.fastino.ai/v1/systemone`): one forward
pass, NO token generation, calibrated probabilities over ≤255 options (+yes/no,
ordinal score), top-minus-second confidence; deterministic unbatched. NOT a
tool caller. **LFM3-1.7B** = Liquid AI (2026-10-27): 1.73B, 256K ctx, native
tool calling + deferred tools, ~1 GB @4-bit, runs llama.cpp/vLLM/SGLang,
vendor says **"not recommended for agentic coding or complex multi-step
reasoning."** Whether `api.liquid.ai/openai/v1` serves LFM3-1.7B is UNVERIFIED
(blend1 pins `lfm2.5-24b-a2b`).

**Owner correction #1 — Fastino is already in sys1:** my "GLiDE is a new
external service" claim collapsed on pushback. Verified: `config.py:69-72`
vendors `glide_api_url = https://api.fastino.ai/v1/systemone`, model
`fastino/glide`, `FASTINO_API_KEY` (legacy key map, config.py:418);
`providers.py:307-361` `_glide_wire`/`parse_glide`; `routing.py:38` GLiDE is
HEAD of sys1's default chain `["glide", "drex", "jev"]` with size exclusion.
Adoption lineage: `docs/plans/2026-09-28-fastino-decide-adoption.md`,
`2026-10-01-glide-provider-and-router.md`.

**Owner correction #2 — Liquid AI is already in sys1 AND blend1:**
- **sys1 = decision half**: `config.py:125-128` Liquid **d1** hosted at
  `https://api.liquid.ai/decisions/v1/systemone`, key `LIQUID_API_KEY`,
  150K-char bound; `providers.py:1135-1147` the `liquid` provider registered
  (probed live 2026-10-05, plan `docs/plans/2026-10-05-liquid-d1-provider.md`);
  `tasks.py:461-463` `vision_qc` is liquid-only; d1 also runs
  `needs_generation`/`needs_live_facts` heads.
- **blend1 = generative half**: `lanes.py:187-190` generate lane speaks
  Liquid's OpenAI-compatible API (`POST https://api.liquid.ai/openai/v1/chat/
  completions`, `lfm2.5-24b-a2b`, same key — "one vendor, one key");
  `orchestrator.py` route lane (sys1 + Liquid d1 decides `needs_generation`) →
  generate lane (LFM2.5-24B synthesizes), `ROUTE_FLOOR = 0.70`.

So the proposal IS the blend1 pipeline applied to tool calling — one-to-one:
GLiDE ≤255 pass → sys1 classify (chain head); "needs the real thing" → d1's
route heads; LFM3 tool calling → the proven Liquid chat wire (model-id swap,
pending the LFM3 hosting check).

**Owner correction #3 — the exploration posture (verbatim intent):** *"I'm the
house and I may choose to pivot whenever it makes sense. Experiment and fail
fast using AI is incredibly cheap, so exploration should not be opposed when
I'm the one proposing it... don't think of blocking because the 'house says'."*
After two corrections I had slid into rule-citing caution (rule 9, weak-half
pushback); the owner reclaimed authorship of the rules. **How to apply:** when
the owner proposes an exploration, my job is to map it onto the estate and
design the fail-fast, not to audit it against house doctrine — cite a rule only
when the owner asks. Related: [[do-not-guess-what-a-subsystem-needs]] (read the
subsystem first — this session proved it twice), [[read-the-services-own-config-first]].

**Nine patterns — selection becomes a forward pass instead of a generation**
(all now criteria in the plan; each independently killable):
1. **Catalog-as-choice**: whole tool catalog (built-ins + aliases + skill
   triggers + lane tools) as ≤255 option labels; one GLiDE pass per turn,
   inject only top-k schemas → collapses ~10K declaration tokens/request.
2. **Skill retrieval by classification**: skill catalog as options, one pass
   per OWNER turn, inject the winner's SKILL.md via `use_skill`; frees sem1 for
   semantic recall over memories/ (its genuine win).
3. **Ensemble-by-disagreement**: GLiDE + d1 in parallel on the same choice;
   agreement → act, disagreement → margin is the ambiguity signal; live twin
   race grading itself against `tool_audit` outcomes (calib rows per head).
4. **Verify-before-execute**: emitted call must argue its own case (choice
   head {this tool, other, none} + ordinal intent↔args score); below floor →
   repair leg: GLiDE picks the right tool → LFM3 regenerates ARGS only (1.7B's
   actual strength, and as repair its "not for agentic coding" caveat doesn't
   bind) → only then the old error-result loop.
5. **Ambiguity → a question, not a guess**: top-minus-second below threshold
   buys a clarifying question at the selection seam (replaces silent guessing /
   Levenshtein ≤3).
6. **Speculative pre-approval/pre-fetch**: selection is fast enough to run
   WHILE the model streams — warm the approval seat and pre-read the target
   file before arguments finish assembling.
7. **Multi-head single pass** (the highest-conviction one): one sys1 request
   answers tool choice + skill choice + workload (feeds `route::select`) +
   destructive yes/no — turn routing becomes a byproduct of selection; stops
   `judge` being a separate beat.
8. **Selection as a graded stream**: chosen option + confidence + margin +
   `tool_audit` outcome = continuous head calibration → thresholds, chain order
   (`glide,drex,jev`), which vendor serves `tool_route` become LEARNED not
   configured; new demand-map feedstock ("head said" vs "outcome was").
9. **LFM3 as the repair/low-stakes author**: confident selection + low-stakes
   authoring (schema fill, arg reformat) → Liquid chat wire (proven in blend1,
   one model-id away) instead of the routed flagship; flagship plans, small
   model fills forms.

**Where rule 9 sits (for the record, not as a veto):** fleet's chat workload
routing stays untouched — Liquid legs live INSIDE a lane exactly as blend1's
generate lane already does (lane cores own their internal model legs). The
version I'd still flag if asked: LFM3 replacing main-model emission outright
(the plan's out-of-scope says the same).

**Post-plan turns (same session):** before activation the owner asked for
OTHER surfaces to expand sys1 + Liquid small models ("forget about house
rules… exploration") — answered with the 23-item three-tier map now saved at
[[sys1-small-model-expansion-map-2026-11-10]] (compaction→LFM3 is the
biggest spend; the pattern is a head-client call in front of a string rule).
The fold-vs-separate-ledger question was ANSWERED: **"let's include tier 1
and 2 into the plan"** (then "tier 3 if it's a hosted api model is fine") —
the plan grew to 20 criteria in six phases, see
[[grounded-selection-expanded-tier123-2026-11-10]]; local models stay out
(owner: the environment lacks memory). Also: the plan file was DELETED TWICE
mid-draft by a co-tenant session (Write reported success both times; only
`ls` caught it; the owner later confirmed "the other session … was stepping
over your work") — re-stat after every write while a co-tenant is live.

**RESOLVED 2026-11-10 — the plan was activated and EXECUTED to its terminal
state the same day:** the owner approved it with the cleanup ruling, then
ruled *"execution should not stop until it's done."* LFM3-1.7B IS served on
api.liquid.ai (probed; the hosted list also carries lfm3-vl-2b, audio, omni);
the C1 kill gate fired (heads dull at selection — best 56.7% top-1 vs the
model's own 99.1%), C2–C9 stopped open, the generation seam (C10) and the
destructive head (C12) landed. Full record:
[[grounded-selection-expanded-tier123-2026-11-10]]. This file keeps the
tool-calling map, the two owner corrections (Fastino in sys1, Liquid in
sys1+blend1), and correction #3 (exploration posture).
