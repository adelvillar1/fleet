---
name: router-picks-the-model-for-usage
description: "owner steer 2026-11-04 — usage/budget/cache reading follows the model the ROUTER selected for the main session; never a provider-specific default, no provider privileged"
type: feedback
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

Owner steer, 2026-11-04, on the context-health build: **"why are you passing
deepseek specific usage, it should be the model selected by the router for the
main session."**

**Why:** fleet is multi-provider; the model is the ROUTER's decision (the pin, the
judged walk, the failover tail). Any usage arithmetic that names a provider — a
DeepSeek-tuned floor, a DeepSeek-only wire-name parse — is a provider assumption
wearing a generic costume. The honest shape reads whatever the SERVED response
carries: the budget from `budget_for(serving_window())` where serving_window is
the router's own decision, and cache breakdowns per wire format (DeepSeek
`prompt_cache_hit_tokens`, OpenAI `prompt_tokens_details.cached_tokens`, Anthropic
`cache_read_input_tokens`) — no provider privileged, absent never zero.

**How to apply:** when adding any per-request measure, derive the model from the
route decision (or the pin / the log's `End.served_model` for a session's last
serving), never from a compiled provider name; when a provider's wire shape must
be known, read it as one format among the formats the client already carries, and
say which provider's names each branch reads. Related:
[[context-health-research-2026-11-04]], [[router-enforcement-ruling]].

**RESOLVED 2026-11-04:** the owner's follow-up — "as long as it's not pinned to
an individual model that is going to fail as soon as we use a different one then
it's okay" — approved the per-format shape (the alias/capture never changes
another provider's parse), and the 100k `COMPACTION_TOKEN_BUDGET` floor STAYS as
the no-declared-window fallback only; the owner removed it from the surface as a
denominator, not as a fallback. Landed in the context-health build (8117496).
