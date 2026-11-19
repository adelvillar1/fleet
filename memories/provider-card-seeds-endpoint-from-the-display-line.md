---
name: provider-card-seeds-endpoint-from-the-display-line
description: "The settings provider card seeds its endpoint field from the list row's display detail (\"base · KEY · enabled\") and saves it back as base_url, so a disable or save press writes the caption into the provider's endpoint (KI-559); the card also saves the post-filter models list, so re-enabling a model deletes it (KI-560)."
type: project
date: 2026-11-17
basis: inferred
tier: durable
shape: [rendering, drops-the-row]
phrases: [override.detail seeds base.value, "cardEntry posts base_url: base.value.trim()", catalog.providers carries no base_url, settings::list_states provider_detail, normalize_base_url, disabled_models filtered from models at load]
missing: []
premises: []
recall-count: 19
superseded-by:
valid-until:
source:
---

Both defects live on the provider card in `src/gui/assets/app.js` and are readable straight off the live read.

**KI-559 — the endpoint is seeded from a caption.** `providerSections` (app.js:21972) does `var detail = (override && override.detail) || '';` then `base.value = detail`, and `cardEntry` (app.js:22079) posts `base_url: base.value.trim()`. The `detail` string is `settings::provider_detail` (src/settings.rs:3510) — `"<base_url> · <KEY_ENV> · enabled"` — a display line, not a value. Compounding it, the read publishes `base_url` on `router.roster` rows (src/gui/bridge.rs:12202) but NOT on `catalog.providers` rows (bridge.rs:12232-12246), and the card is handed the catalog row. So there is no clean value available to the card at all.

Live instance: the owner disabled `stepfun`, and only that entry carries a mangled base_url — `https://api.stepfun.ai/step_plan/v1 · STEPFUN_API_KEY · enabled` — because the disable press wrote the caption back. The request URL becomes `<that>/chat/completions`, which curl rejects with exit 3. The disable itself was the owner's correct intent; the base_url corruption was collateral.

The fix: add `base_url` to `catalog_document`'s provider block (the roster row already computes it) and seed the card from `provider.base_url`, never from `override.detail`. The detail line is a caption.

**KI-560 — re-enabling a model deletes it.** `route.rs:1474-1479` retains `candidate.models` minus `disabled_models` at load, and bridge.rs publishes that filtered list as `catalog.providers[].models`. The card's switch-off handler only mutates its local `disabled` array, and the save posts the filtered `provider.models` plus the new off list. A model switched back ON is therefore in neither array and vanishes from the provider entirely — proven in a scratch home: entry `[a,b,c]` with `[b]` off, saved as `[a,c]` with no off list.

`route::Candidate`'s own comment states the intent the read violates: "`models` stays the provider's own full list … the off list is the deliberate act."

Filed as KI-559 and KI-560 in `docs/known-issues.md`; `scripts/ki_ledger.py` reports 560 rows, 4 open, all cell counts agreeing.
