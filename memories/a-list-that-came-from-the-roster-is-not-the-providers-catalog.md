---
name: a-list-that-came-from-the-roster-is-not-the-providers-catalog
description: "feedback 2026-11-01 — I shipped "all the models" as the roster's compiled copy (1 for zai where the provider lists 11) and never asked whether the list had ever been DISCOVERED; the reload that would have discovered it refused every compiled provider, so seven of eight catalogs could never be the provider's own"
type: —
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

The 2026-11-01 settings build. After the connect→discover→pin flow went
live, the owner asked the question I had not: *"are they really all of
the models available or you are taking the lazy route of copying what was
available there? the idea is to test that you can get the models
presented by the provider and their settings."* They were not. Every
compiled roster list still stood unchanged — zai-coding-plan showed 1
model where the provider lists **11**, token-plan 1 where it lists
**20**, deepseek 1 where it lists 2.

Two defects hid behind it, both only visible from a live reload:

1. `rediscover_provider` looked the provider up **only in the owner's
   `config.json`**, so every compiled-in provider answered `` `x` names
   no configured provider `` and the reload never ran for it — the
   button existed and did nothing for seven of eight.
2. The real home's `ollama-cloud` entry carried **neither `base_url`
   nor `key_env`**, so the reload derived the key name from the id
   (`OLLAMA-CLOUD_API_KEY`) — impossible (the real one is
   `OLLAMA_API_KEY`), and it failed before anyone could see it.

**Why:** a settings surface that displays a list has implicitly claimed
it queried the thing that owns the list. A compiled default is
indistinguishable on screen from a discovery result, so "the model list
works" was true of the RENDER and false of the DATA — and the owner's
whole point was that discovery is the feature ("if the provider exposes
it I get all the models and their settings"). A discovery path that
only runs for entries the owner has already saved is discovery that
never runs, because the entries all come from the roster.

**How to apply:** when a surface shows a list a remote owns, count it
against the remote — `provider lists N` versus `the read says M` — and
treat any provider whose list equals its compiled default as
**undiscovered**, not as current. Make the reload fall back to the
compiled roster for providers the config does not carry, and let a
stale entry inherit what it is missing (`base_url`, `key_env`) rather
than deriving it, because a derived name for an entry that never had one
is a name no key is stored under. Show the remote's numbers, not a
"declared" badge: deepseek's `context_length` 1,048,576 became the row's
`1M window`, while the providers whose listings publish no window
honestly show none. And do the reload on a COPY of the real home first
(copy `config.json routing.json .env vault.json fleet.env ui.json` — a
full `rsync` of `~/.fleet` hangs), then on the real one. **The reload
is now an app action too** (`b03520b`, accounts card: "reload every
provider" → each enabled non-lane provider asked in turn, answering
"reloaded N of M" and naming the ones that did not — liquid only while
its `base_url` was the full `…/chat/completions` endpoint instead of
the `/openai/v1` origin; corrected 2026-11-01, it now answers with five
models and the home reads 76. See
[[liquid-is-two-services-lfm-chat-and-d1-system-one]]). Its harness pin counts only the calls the press
made (`calls.slice(len_before)`), because an earlier click in the same
page had already posted one. **The owner's standard for a provider edit is the whole lifecycle on one
card** — *"I just don't see in that screen a way to add a provider, which
is a glaring gap / or modify it, or delete it"* — and after the catalogs
were fixed they cut the diagnosis short (*"you are taking the lazy route
of copying what was available there"*) and named the test: *"the idea is to
test that you can get the models presented by the provider and their
settings."* So the acceptance for any discovery path is a LIVE per-provider
comparison (`provider lists N` vs `the read says M`) plus the declared
settings, run through the app's own action — not a stub. Related:
[[settings-ux-plan-2026-11-01]],
[[a-live-probe-inside-a-pre-existing-development-provider-is-not-a-provider]],
[[a-kernel-is-not-the-product]].
