---
status: draft
created: 2026-11-03
updated: 2026-11-03
slug: provider-add-is-independent-of-its-catalog
---

# Plan: adding a provider is independent of reading its catalog

## Context

KI-407. The owner's report: **adding a model should not depend on existing
models or providers — they are independent, and no check should poll for the
existing ones.** Hinging a validation on one provider's ability to answer
was, in the owner's words, "not a great choice".

The defect is in Fleet's own provider-add path, and it is the same shape as
the upstream opencode bug the owner hit while setting that tool up.

`set_provider_with_key_with` (`src/settings.rs:3025`) is the single writer
behind the settings face's provider card (`src/gui/bridge.rs:8654`). It does
three things in order:

1. writes the key to the store (`src/settings.rs:3042-3050`);
2. polls the provider's `/models` listing (`discover_provider_models`,
   `src/settings.rs:2757`) through the key just resolved;
3. writes the entry — **or refuses**.

The refusal is `src/settings.rs:3130-3152`. When the listing read fails
**and** the entry carries no `models` yet, the function returns the error
string `"the provider's models could not be discovered: {error}"`. The key
was already stored. The entry was not. The GUI surfaces the refusal and the
card will not save.

So a provider cannot be added at all unless its catalog can be read at that
moment. That is a real cost on this machine: a gateway that gates `/models`
on a subscription tier, a plan-scoped key the listing endpoint does not
honor, a base URL that does not exist yet, or a provider that is briefly
down — every one of them refuses the save.

**The code already knows this is wrong.** The comment on the branch says
"a provider may be added before it is funded", and the sibling branch right
below it implements exactly that for an entry that already carries models:
the failure "rides the reply, never swallowed". The `empty` guard is what
makes the two cases different, and the guard is the defect — it is a check
on the existing entry, not on the write the owner asked for.

**The compounding detail.** The key is written before the listing read. A
refused save therefore leaves the credential stored and the provider absent:
the owner sees a card that will not save *and* a secret that is already on
disk. That is the state the owner met with opencode — one credential in
`auth.json`, a model list that disagreed with the runtime registry, and a
chat that failed with `ProviderModelNotFoundError` naming a provider that
was never loaded.

**What "independent" means here, stated as the rule the fix enforces:**

- Adding a provider is a write of the entry the owner supplied. It succeeds
  or fails on the entry's own validity — never on a network answer.
- Reading a provider's catalog is a separate operation with its own result.
  It can fail, and its failure is reported, never swallowed — but it never
  gates the write.
- A provider entry is not required to name models. `set_provider` today
  refuses an empty `models` list (`src/settings.rs:3190-3194`); that
  refusal is part of the same coupling and goes with it.

## Approach

**Separate the write from the listing read, and make the entry the source of
truth.** The save lands on the entry as supplied. The catalog read runs when
it can, and its outcome — models merged, or a named failure — rides the
reply. Nothing about the listing's success or failure changes whether the
provider exists.

Three increments, each independently valuable:

### F1 — the save lands when the listing cannot be read

`set_provider_with_key_with` stops returning an error when the catalog read
fails. Instead:

- the entry is saved with the models it already carried (or none);
- the reply carries the failure, unchanged in wording, so the face can show
  *"saved, catalog not read: ..."* rather than a refusal;
- a successful read still merges the catalog exactly as it does today.

The `empty` guard at `src/settings.rs:3137-3150` is deleted. Its comment is
replaced with the rule it used to encode, so the next reader sees why a
failed listing does not refuse a save.

`rediscover_provider` (`src/settings.rs:2935`) is the explicit re-list and
already returns a failed outcome rather than an error — it needs no change.
It is the shape F1 brings the write path into agreement with.

### F2 — a provider entry may name no models

`set_provider` refuses an empty `models` list at `src/settings.rs:3190-3194`.
That refusal is what makes a model-less entry unwritable, and it is the same
coupling seen from the other side. An entry with no models is a provider
that is declared but not yet catalogued: it is visible, it is not usable by
a walk until it has models, and the roster already has the machinery for
that (an entry with no candidates serves nothing and the walk falls through
— the same fail-open the routing chain already documents).

The refusal is replaced by a warning on stderr, in the same advisory shape
as `session_constant_warning` (`src/settings.rs:3199`): the save lands, the
owner is told the provider has no models yet.

This is the increment that makes "adding a model" not depend on existing
models: a provider can be created empty, and its models added later.

### F3 — the key write and the entry write are one outcome

Today the key is stored first and the entry write can then refuse, leaving a
stored secret with no provider. F1 removes the refusal, so the two writes
cannot diverge on a listing failure. What remains is the ordinary failure of
the entry write itself (`write_atomic`): the secret is stored and the entry
is not. That is a real, narrow case — a disk error — and the fix is to
report it honestly rather than to reorder a side effect that must happen
before the catalog read can resolve the key. The reply already names the
store (`key_store`); the fix adds the entry write's outcome to the same
reply, so a divergence is visible rather than silent.

## What the fix must not do

- **It must not swallow a catalog failure.** The failure rides the reply, in
  the discovery's own words. The existing comment's "never swallowed" is the
  requirement; the bug is only that it was also never allowed to be
  non-fatal.
- **It must not weaken the entry's own validation.** A misspelled field is
  still a named refusal (`validate_provider_entry`). An empty `id` is still
  a refusal. A `providers` value that is not a list is still a refusal.
  What goes is the network precondition, not the entry's integrity.
- **It must not make a model-less provider silently usable.** F2's warning
  is the visible half; the roster's existing "no candidates serves nothing"
  is the behavior half. Neither is new code.

## Verification

**Tests** (the gate is `cargo build --all-targets` then `cargo test`, exit
code read on the cargo invocation itself, never through a pipe):

1. `a_provider_saves_when_its_listing_cannot_be_read` — a stub answering
   403 on `/models`; the entry names no models; the save returns
   successfully, the provider is in `config.json`, and the reply's failure
   field carries the listing's reason. This is the regression pin for
   KI-407's core.
2. `a_provider_with_no_models_saves_and_says_so` — an entry with no `models`
   key at all saves, and the entry in the file carries no models.
3. `a_provider_added_with_no_models_is_not_usable_until_it_has_some` — the
   roster load leaves the provider present and serving nothing, so a walk
   naming it falls through rather than failing the run.
4. The existing `a_save_with_a_key_stores_it_and_merges_the_catalog`
   (`src/settings.rs:1867`) stays green: a successful listing still merges
   the catalog and the models it found are written.
5. The existing `a_refused_key_is_named_by_the_discovery`
   (`src/settings.rs:1856`) is updated to the new contract: the listing read
   names the refusal, and the save still lands.

**Live** — the owner's face, not a unit test, is the surface that was broken:

- drive the settings provider card against a base URL that answers 403 on
  `/models` and confirm the card saves, the provider appears in
  `fleet providers`, and the catalog's failure is shown rather than a
  refusal;
- then re-list it against a working endpoint and confirm the models arrive.

## Out of scope

- The upstream opencode defect itself (the CLI `models` command reading the
  static catalog while the server registry derives from auth/env/hooks) is
  not Fleet's code. It is the incident that surfaced the pattern and is
  recorded in KI-407's evidence as the motivating case.
- A background re-list that retries the catalog later. F1 makes the save
  land; a later successful re-list is already available through the
  explicit `provider-discover` op. Making it automatic is a separate
  decision with its own cost.
