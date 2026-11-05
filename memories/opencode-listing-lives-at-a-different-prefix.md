---
name: opencode-listing-lives-at-a-different-prefix
description: "reference 2026-11-03 — OpenCode's OpenAI-compatible chat base and its model listing are at DIFFERENT prefixes (chat …/inference/openai/v1/chat/completions, listing …/inference/v1/models), and the listing is PUBLIC while the chat needs a key; measured live with the owner's key"
type: reference
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

Measured 2026-11-03 against the live service with the owner's key
(`oc_sk_…`, since rotated away — never store it):

| What | Address | Result |
|---|---|---|
| Chat endpoint | `https://opencode.ai/inference/openai/v1/chat/completions` | answers — a 400 about the model, **not** a 401, so the key is valid |
| **Model listing** | **`https://opencode.ai/inference/v1/models`** | **200**, OpenAI shape `{"object":"list","data":[{"id":…}]}`, ~95 KB |
| Same-prefix listing | `…/inference/openai/v1/models` | **404, empty body** |
| Other guesses | `/inference/openai/models`, `/inference/models`, `/models`, `/api/models` | 404 (or a 302 to a login page) |

**The trap.** The base implied by the chat endpoint is
`https://opencode.ai/inference/openai/v1`, and Fleet builds the listing as
`{base}/models` — which 404s. The listing lives at a **different prefix**
(`/inference/v1`, not `/inference/openai/v1`), and it is **public** (200
with no `Authorization` header) while the chat needs the key. The vendor's
own docs name it: `https://opencode.ai/v2/docs/console/models` →
`GET https://opencode.ai/inference/v1/models`.

**Why this matters beyond one provider.** The owner's report — *"the
provider's models could not be discovered: the listing answered HTTP
404"* — is the same class as the repo's earlier
[[a-list-that-came-from-the-roster-is-not-the-providers-catalog]] and the
[[liquid-is-two-services-lfm-chat-and-d1-system-one]] trap: a base URL
pasted as a full endpoint, and a listing that is not where the chat is.
It is also the **second** time a provider's listing has been at an
unexpected address, which is why the general fix is a `models_url` field
on the provider entry rather than a per-provider special case.

**What the fix carries** (landed 2026-11-03 as **980e350** — see
[[provider-discovery-one-shape-and-dead-reply]] and
[[provider-connect-honesty-plan-drafted-2026-11-03]]):

- `ProviderEntry.models_url` — where the listing lives when it is not
  `{base_url}/models` (in `src/route.rs`), and the connect card's
  editable **models endpoint** field derived from the endpoint.
- `normalize_base_url()` — strip a pasted `/chat/completions`,
  `/responses`, `/completions` or `/models` so the stored base is a base
  (the chat URL is built as `{base}/chat/completions`, so an unnormalized
  paste asks `…/chat/completions/chat/completions`).
- A 404 named distinctly — *"this provider publishes no listing at that
  address (HTTP 404) — name its models endpoint, or add the models by
  hand"* — rather than a bare status code.

Verified live with the fix: the base normalized, **38 models** discovered
(`opencode/big-pickle`, `opencode/beam-free`, `opencode/flint-40b-free`,
…), all 38 in the catalog. The provider is connected in the owner's home
(`opencode`, key `OPENCODE_API_KEY` in `~/.fleet/.env`), and 521724c added
the connect card's models-endpoint field plus a models-list field for
providers that publish no listing at all.

Related: [[composer-actions-plan-drafted-2026-11-03]] (F6–F8's home),
[[a-deferred-action-is-an-error-to-the-owner]] (the ruling the whole
audit ran under).
