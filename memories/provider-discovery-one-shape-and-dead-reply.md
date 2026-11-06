---
name: provider-discovery-one-shape-and-dead-reply
description: "project 2026-11-03 — the connect flow's model discovery accepts ONLY {\"data\":[{\"id\":…}]}, its Discovery reply hardcoded models:[] and settings_written:0, and disabled_models are stripped from the picker entirely (so a provider's model list can be silently empty); the dead reply and the pasted-endpoint trap are FIXED (980e350), the escape hatches landed (521724c — models_url + the owner's models field), the shape restriction and the disabled-models half are still open"
type: project
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

The owner reported on 2026-11-03: *"when I try to add opencode chat
completions (provider) on fleet with a valid key no list of models is
returned, the expected behavior is what zcode does that fetches the list
of all models, even disabled ones."* They had a session stop mid-flight
trying to work it out.

## What I proved first — the happy path works

Against a controlled listing (`{"data":[{"id":"alpha-1","context_length":131072},…]}`)
the whole pipeline is sound: the connect returned
`{"discovery":{"ok":true},"key_store":".env"}` and `config.json` carried
`models: ['alpha-1','beta-2','gamma-3']` plus per-model `model_settings`
windows. So the bug is **not** the pipeline.

## Three real defects

1. **The discovery accepts exactly one shape** (`src/settings.rs`):
   `value.get("data").and_then(as_array)` — only the OpenAI
   `{"data":[{"id":…}]}`. A top-level array, `{"models":[…]}`, or a nested
   per-provider map all fail with *"the provider lists no models"*. The
   provider is still saved (KI-407: a listing failure is not a reason to
   refuse the write) but with `models: []` — **a silently empty provider**.
   *Not the cause of the owner's 404 — their shape was right.*
2. **The Discovery reply was dead** (`src/settings.rs`): on success it
   hardcoded `models: Vec::new(), settings_written: 0` even though the
   models were found and written. **FIXED** at 980e350 — it now reports
   both, and the bridge's reply shape carries them.
3. **Disabled models are removed from the picker entirely**
   (`src/route.rs`: `candidate.models.retain(|m| !off.contains(m))`, and
   the catalog's model list is built from `candidate.models`).
   `disabled_models` is carried per-provider only, which the model list
   does not read. **STILL OPEN — that is the "even disabled ones" gap**:
   ZCode lists them flagged; Fleet hides them.

## Resolved the same day — the listing was at a different prefix

The owner's next message gave the real error (*"the listing answered HTTP
404"*) and the endpoint, and a live probe settled it: **OpenCode's model
listing is at `https://opencode.ai/inference/v1/models`, not beside the
chat base** (`/inference/openai/v1`), and it is **public** while the chat
needs the key. See
[[opencode-listing-lives-at-a-different-prefix]] for the measured table.

That became **`docs/plans/2026-11-03-provider-connect-honesty.md`**,
implemented and committed the same day as **980e350**, then **completed as
521724c** (pushed origin/master, the Tauri app rebuilt) — see
[[provider-connect-honesty-plan-drafted-2026-11-03]] for the landed state
and the live verification (38 models). The escape hatches that landed:
`ProviderEntry.models_url` (the listing at another prefix), the connect
card's editable models-endpoint field, and a **models-list field** (one per
line) so a provider with no listing at all carries the models its owner
names — a failed discovery never overwrites them. The one-shape parser and
the disabled-models picker gap are still open.

Related: [[a-list-that-came-from-the-roster-is-not-the-providers-catalog]]
(the sibling lesson — a list that never came from the provider),
[[a-live-probe-inside-a-pre-existing-development-provider-is-not-a-provider]]
(prove connect against a genuinely new provider),
[[composer-actions-plan-drafted-2026-11-03]] (where F6–F8 would land).
