---
name: fleet-vision-gemma4-default
description: "owner directive 2026-10-17 — Ollama Cloud gemma4:31b wired as fleet's vision reserve (435278f) and added to zcode's provider config; the owner's real key was pasted 2026-10-18 into zcode's ollama-cloud entry and ~/.fleet/.env per the own-home ruling; the placeholder had been schema-valid (a 401 waiting to happen)"
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

Owner ruling, 2026-10-17: "wire ollama cloud as a provider, and use gemma4 as
a vision model for both zcode and fleet." **Done — both sides landed.**

**Fleet (435278f):** an `ollama-cloud` candidate appended to `default_roster()`
alongside blend1 — `https://ollama.com/v1`, the key named `OLLAMA_API_KEY` and
resolved from fleet's own home — and `gemma4:31b` added to the vision
workload's chain behind GLM-5.3-Flash and step-5-preview. Declared only, like
Liquid: it enters no other workload's chain, and a pin or chain entry in the
owner's config is what moves it. gemma4's context window is not declared by
Ollama Cloud in a verified form, so it keeps the compaction floor rather than
being given a number nobody has checked — the window-table test names it absent
for exactly that reason. Five existing assertions grew by one candidate
(counts and chain-order lists), and one new test verifies the vision chain
reaches gemma4 and names the endpoint and key var.

**Zcode (2026-10-18, this session):** the `gemma4:31b` rule landed in
`provider_config.json` as a `manualProviderModelRules` entry plus an
`ollama-cloud` `providerRules` entry, and the third
[[zcode-provider-config-degrades-on-validation-failure]] outage was exactly
this edit (the stray `properties.supportsImage`, repaired 20:47, with the
rule's `providerId` fixed from a `stepfun` template leftover to
`ollama-cloud`). **The owner pasted the real key** and it was wired into the
ollama-cloud entry's `access.apiKey` (replacing the placeholder) and appended
to `~/.fleet/.env` as `OLLAMA_API_KEY`. Two numbers in the rule were LEFT
as-is and flagged to the owner, not invented: `maxOutputTokens` 168000 exceeds
its own `contextWindow` 131072 (both template-copied from a stepfun sibling),
and the `reasoningLevel` map is the generic stepfun thinking-params template.

**The key location ruling.** The owner initially said "the api key should be on
the hermes config", which collides with the own-home ruling
([[fleet-roster-migrated-from-router-kit]]): no cross-harness config reads.
Flagged rather than complying, proposed `~/.fleet/.env` beside the other eight
keys or better the AES-256-GCM vault ([[fleet-vault-credentials]]). The owner
did not explicitly overrule the ruling; the key was placed in fleet's own home
per the standing ruling. **If the owner later says to read hermes' config, that
is an explicit overrule of the own-home ruling and should be recorded as one.**

**Key finding that shaped the design:** zcode's `provider_config.json` stores
**literal API key values** for every real provider (deepseek 35 chars, mimo 51
chars, stepfun 64 chars, one 580-char entry). No env-var reference mechanism
exists. That is the exact problem the fleet vault solves for fleet's own keys.
And the placeholder episode is the corollary: a placeholder `apiKey` string is
schema-VALID, so the Zod gate says nothing about a provider that will 401 on
every call — validation success is not functional success.

**Why:** the vision workload is one of the few places fleet already has a clean
swappable seam, so the model swap itself is cheap and reversible.

**How to apply:** the `Candidate` goes in `route.rs`'s default roster with the
Ollama Cloud `base_url` and a `key_env`; the key goes in `~/.fleet/.env` or the
vault **unless the owner explicitly overrules the own-home ruling**. Do not
read hermes' config without that explicit overrule. Related:
[[fleet-deck-visual-pipeline]] (the vision workload, the vision tool, and the
grader), [[fleet-vault-credentials]],
[[fleet-roster-migrated-from-router-kit]], [[blend1-web-leg-and-corpus]],
[[zcode-provider-config-degrades-on-validation-failure]].
