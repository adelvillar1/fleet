---
name: provider-connect-honesty-plan-drafted-2026-11-03
description: "project 2026-11-03 — docs/plans/2026-11-03-provider-connect-honesty.md COMPLETED (521724c, C0–C4 ticked, pushed origin/master, Tauri app rebuilt) — normalize a pasted endpoint to its base, let a provider name its own models endpoint AND supply its models by hand, and make the Discovery reply honest; verified live against OpenCode (38 models)"
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

Drafted 2026-11-03 after the owner's OpenCode connect failed with
*"the provider's models could not be discovered: the listing answered
HTTP 404"*. The contract is
**`docs/plans/2026-11-03-provider-connect-honesty.md`** — and it was
**IMPLEMENTED WHOLE, status `completed`, all five criteria ticked**,
committed as **980e350** (F1/F3) then **521724c** (F2 + the pins),
**pushed to origin/master**, and the Tauri app rebuilt from it.

## What landed

| | Fix | Where |
|---|---|---|
| **F1** | `normalize_base_url()` — strip a pasted `/chat/completions`, `/responses`, `/completions` or `/models`; applied in `set_provider_with_key_with` **and** where the chat URL is built (`api.rs`) | `src/settings.rs`, `src/api.rs` |
| **F2** | `ProviderEntry.models_url: Option<String>` — a provider whose listing is not `{base_url}/models` names it; the connect card gained an editable **models endpoint** field **and a models-list field** (one per line) so a provider that publishes no listing is connectable with the models the owner names | `src/route.rs`, `src/gui/assets/app.js` |
| **F3** | The `Discovery` reply reports the models found and the settings written (both were hardcoded empty), and a 404 is named distinctly | `src/settings.rs`, `src/gui/bridge.rs` |

Pins: `a_pasted_endpoint_normalizes_to_its_base`,
`a_pasted_endpoint_is_stored_as_the_base_it_names` (the stored entry AND
the discovery asked the base),
`a_provider_with_its_own_listing_is_asked_there`,
`a_missing_listing_is_named_as_no_listing`,
`a_provider_with_no_listing_carries_the_models_its_owner_named` (a 404
listing + owner models → the entry carries them, the catalog — the
picker's source — lists them), and harness case CA-C3 (a failed
discovery names the reason; a success names the models; the write
carries the owner's models).

## Verified live against the real endpoint

```
base_url:  https://opencode.ai/inference/openai/v1      ← normalized
models_url: https://opencode.ai/inference/v1/models
models:    38
catalog:   opencode/big-pickle, opencode/beam-free, opencode/flint-40b-free, …
```

The provider is connected in the owner's home (`opencode`, key
`OPENCODE_API_KEY` in `~/.fleet/.env`). Gate: fmt, clippy, release build,
**2,008 tests / 0 failed** at F1/F3; **2,012 / 0** after F2's completion.

## Still open

**Disabled models are hidden from the picker** — `src/route.rs` strips
them from `candidate.models` and the catalog's model list is built from
that, so a provider's off-list never reaches the picker. That is the
"even disabled ones" half of the owner's ask and is deliberately
**not** in this plan (a picker that shows them needs them kept out of
the walk and the pin — a separate change).

Related: [[opencode-listing-lives-at-a-different-prefix]] (the measured
addresses this plan exists for),
[[provider-discovery-one-shape-and-dead-reply]] (the three defects the
plan's F2/F3 close),
[[composer-actions-plan-drafted-2026-11-03]] (F6–F8 were folded into this
plan instead), [[a-deferred-action-is-an-error-to-the-owner]].
