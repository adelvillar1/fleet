---
name: zcode-vs-hermes-prompt-comparison
description: "2026-10-16 comparison of what zcode and hermes seed to new sessions — hermes' behavior-contract style (SOUL.md hot-loaded, model-gated execution discipline, injection-scanned context files, cache byte-stability) vs zcode's fixed operating manual; best-of synthesis is the input for fleet's future SYSTEM_PROMPT slice"
type: project
date: 2026-11-05
basis: inferred
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

The owner asked (2026-10-16) to compare zcode's session seed against hermes agent's, to take the best of both — the real application is designing fleet's `SYSTEM_PROMPT`, currently two generic sentences in `src/agent.rs:42`.

**Hermes** (source read at `~/.hermes/hermes-agent/agent/system_prompt.py` + `prompt_builder.py`; persona at `~/.hermes/SOUL.md`): `SOUL.md` in the hermes home is the primary identity, re-read every message (hot-swappable); non-negotiables include claim-time verification ("every claim about live state backed by tool output from THIS session; done = every acceptance criterion tool-verified"); mandatory dev flow in the persona (plan → delegate_task → spec-review → quality-review; direct implementation only <15 min/≤2 files; recap rule); model-gated guidance blocks — `EXECUTION_GUIDANCE_MODELS` covers deepseek/kimi/qwen/glm/mimo/minimax, exactly fleet's roster families (prose math, no read-back after writes, premature "done"); parallel-tool-call block with the round-trip-cost rationale; context-file chain (`.hermes.md` → merged `AGENTS.md` dir chain → `CLAUDE.md` → `.cursorrules`) with truncation caps, shadowing, and a prompt-injection scan that blocks project files; byte-stability engineering (pinned workspace snapshot, resolved-once parts) to protect provider prefix caching.

**ZCode** (known from the inside): fixed compiled-in manual — lead-with-the-outcome communication contract, end-turn discipline ("if your last paragraph is a plan, do the work now"), guidance lives in tool descriptions rather than prompt blocks, match-verification-scope-to-change, one-fact-one-file memory.

**How to apply:** the fleet SYSTEM_PROMPT slice (offered, not yet approved) should take from hermes: claim-time verification near-verbatim, the execution-discipline block ungated (fleet's whole roster is the affected families), a hot identity file in `<FLEET_HOME>` read per run, injection scan + bounds on repo-assembled context (recalled facts, skill bodies, nomenclature walks), byte-stability, the recap rule. From zcode: lead-with-outcome, end-turn discipline, tool-mechanics-in-tool-descriptions. Related: [[fleet-harness-orientation-doc]], [[orchestrator-routes-through-hard]].
