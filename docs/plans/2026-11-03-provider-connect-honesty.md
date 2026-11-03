---
status: draft
created: 2026-11-03
updated: 2026-11-03
slug: provider-connect-honesty
---

# Plan: connecting a provider never dead-ends

## Context

The owner tried to connect OpenCode's OpenAI-compatible inference endpoint
and got: *"the provider's models could not be discovered: the listing
answered HTTP 404."*

What is true of that endpoint, measured 2026-11-03:

- The endpoint is `https://opencode.ai/inference/openai/v1/chat/completions`
  — the **full endpoint**, not the base. The base is
  `https://opencode.ai/inference/openai/v1`.
- The key is valid: `POST` on the chat endpoint answers (a 400 about the
  model, not a 401).
- **The inference API publishes no model listing.** `GET` on
  `/inference/openai/v1/models`, `/inference/openai/models`,
  `/inference/models`, `/models` and `/api/models` all answer 404 (or a
  302 to a login page).

So the discovery cannot work for this provider — there is nothing to
discover. Three defects make that fatal rather than merely limited:

1. **A pasted endpoint is taken as a base.** The chat URL is built as
   `{base}/chat/completions` (`src/api.rs:693`), so pasting the endpoint
   yields `…/chat/completions/chat/completions`. The same trap the repo
   already recorded once (`memories/liquid-is-two-services-…`): *"a
   base_url pasted as the full `/chat/completions` endpoint 404s
   everything"*.
2. **A failed discovery dead-ends.** The provider is saved with
   `models: []` (KI-407's ruling — a listing failure is not a reason to
   refuse the write) and the connect card offers no way to supply them.
   The only route is hand-editing `config.json`.
3. **The discovery's reply is dead.** `Discovery` hardcodes
   `models: Vec::new()` and `settings_written: 0`
   (`src/settings.rs:3241-3246`), so the reply cannot say what was found.

## Approach

**Normalize at the boundary, and make the listing optional.**

1. **F1 — a pasted endpoint normalizes to its base.** A helper strips a
   trailing `/chat/completions`, `/responses`, `/completions` or `/models`
   from a base URL. Applied where the owner's paste lands (the connect
   write) and where the chat URL is built, so both paths agree.
2. **F2 — the connect card carries a models field.** The owner can supply
   the models when the listing is unavailable, and see what was
   discovered when it is. The entry already has a `models` field; the
   card just never exposed it.
3. **F3 — the discovery's reply tells the truth.** It reports the models
   found and the settings written, and a 404 is named distinctly from a
   transport failure — *"this provider publishes no listing"* rather than
   a bare status code.

The docs move with the increments: the provider-connect wording lands
with F2.

## Use cases

- **The owner with an endpoint, not a base.** Paste the full
  `/chat/completions` URL; Fleet stores the base and both the chat and
  the listing work.
- **The owner whose provider publishes no listing.** The connect card
  says so and offers a models field; the provider is usable immediately.
- **The owner who wants to know what landed.** The reply names the models
  discovered and the windows written.

## Acceptance criteria

- [ ] **C0 — a pasted endpoint normalizes.** An entry whose `base_url`
      ends in `/chat/completions` is stored as the base, and both the
      chat URL and the listing URL are built from it. Pinned by a test
      over the normalizer and the stored entry.
- [ ] **C1 — a provider with no listing is connectable.** With the
      listing answering 404, the connect card offers a models field; the
      entry the owner submits carries their models, and the provider
      appears in the roster and the model picker. Pinned by a test over
      the settings write and the catalog.
- [ ] **C2 — the reply is honest.** A successful discovery reports the
      models found and the settings written; a 404 reports that the
      provider publishes no listing. Pinned by tests over both paths.
- [ ] **C3 — the connect card says what happened.** On a failed
      discovery the card names the reason and shows the models field; on
      a success it names the models. Pinned by a face-harness case.
- [ ] **C4 — the gate is green.** fmt, clippy `-D warnings`, release
      build, and the suite, each read by exit code (KI-014).

## Files to be touched

| File | Change |
|---|---|
| `src/settings.rs` | F1's normalizer; F3's honest `Discovery`; the normalized base in `set_provider_with_key` |
| `src/api.rs` | F1: the same normalizer where the chat URL is built |
| `src/gui/assets/app.js` | F2/F3: the connect card's models field and its answer |
| `src/gui/bridge.rs` | F3: the discovery's reply shape |
| `tests/` | C0–C2's pins |

## Out of scope

- **A curated catalogue for providers that publish none.** OpenCode's
  models are not listable from the endpoint; a built-in list would be a
  second source of truth that drifts. The owner supplies them.
- **Probing endpoint variants.** Trying `/models` on guessed paths is a
  rabbit hole; the 404 is answered honestly instead.

## Verification

- **The gate, whole.**
- **A live connect against a mock with no listing** (404): the card
  offers the models field and the provider lands.
- **A live connect with a pasted endpoint:** the stored base is the base.

## Risks

- **Normalizing a base that legitimately ends in `/models`.** The
  suffixes stripped are endpoint paths, not version segments; a base
  ending in one of them is an endpoint by construction.
- **A manually supplied model list is unverified.** The owner can name a
  model the provider does not serve. The walk will fail on it and name
  the refusal — the same as a typo in any roster.

## Dependencies

- Rust 1.99.1 and the gate's four legs.
- The connect flow: `src/gui/bridge.rs` (`settings_apply`'s `provider`
  op), `src/settings.rs` (`set_provider_with_key`).

## Notes

- **The principle:** a connect that cannot discover must still connect.
  The listing is a convenience, not a gate — KI-407 ruled that for the
  write, and this extends it to the surface.
