---
status: draft
created: 2026-10-31
updated: 2026-10-31
slug: provider-models-settings
---

# Plan: models and providers in settings — add a provider, get its models, assign them per workload

## Context

The settings revamp gave the providers and workloads surfaces the rail
idiom, but their mechanics are still the compiled-in roster's: the owner
types everything by hand, and the router walks what was typed.

**The providers card** (`settings-providers`,
`src/gui/assets/app.js:15830`) edits the roster's config.json overrides
one field at a time: a base URL, a **key env name** (a name, never a
value — `app.js:15860`'s own caption), **models as hand-typed chips**
(`app.js:15864`), billing, kind, enabled. Adding a provider means
already knowing its model ids and creating the key env var somewhere
outside fleet. **The workloads editor** (`app.js:15700`) writes
`routing.json`'s ordered candidate list per workload — a flat set of
`provider/model` strings where *primary* is implicit (position one) and
*secondary* is the unmarked rest. There is no enable/disable per model,
no discovery, and no named primary.

The owner's direction, 2026-10-31, and its steer the same day:

1. *"if I add a provider and valid key in the settings it should load
   all of the available providers, at which point I can enable/disable
   and assign the workload types and if they are primary or secondary
   for each work load I choose"* — the flow is provider + key in,
   catalog out, assignment next.
2. The steer, ruling the first sentence's scope: *"it should add all
   the models from the provider"* — one provider's add loads that
   **provider's models**; the catalog comes from the provider, not from
   hand-typing.

The substrate this plan stands on is all landed: the settings rail and
its `settings_write` write path (`src/gui/bridge.rs:6870`'s
`provider`/`workload` actions), the vault seam (a secret seals into the
vault when the passphrase is present, `<FLEET_HOME>/.env` otherwise —
`src/settings.rs:1490`'s own test), the router's config merge per
provider id (`src/route.rs:989`), and `routing.json`'s ordered
candidate list the walk already honors. Verdicts in the route cache are
label-level, so a preference write applies to the next judgment with no
invalidation (the surface-curation deep dive's finding).

## Decisions (the owner's, dated and quoted)

1. **2026-10-31 — the flow.** *"if I add a provider and valid key in
   the settings it should load all of the available providers, at which
   point I can enable/disable and assign the workload types and if they
   are primary or secondary for each work load I choose"* — provider +
   key in; the model catalog, the enable/disable, and the per-workload
   primary/secondary assignment are the experience.
2. **2026-10-31 — the steer.** *"it should add all the models from the
   provider"* — the model list is **discovered from the provider**, not
   hand-typed; the catalog is the provider's own, complete.
3. **2026-10-31 — the settings come with the model.** *"we also need to
   auto configure the model's settings like context window, output
   size, etc. if that information is available from the provider"* —
   discovery stores what the provider declares (the context window, the
   output ceiling), the stored value configures the run, and a hand
   override wins over the provider's word.

## Approach

**The key lands where secrets live, and the provider is verified by
listing it.** The provider form's key field takes the **value**
(write-only, never echoed — the read model carries only `key_present`,
as today). On save, the bridge writes the secret through the existing
vault seam under the provider's `<ID>_API_KEY` name — sealed into the
vault when the passphrase is present, `~/.fleet/.env` otherwise — and
then immediately verifies it the only way a model provider verifies:
`GET {base_url}/v1/models` with the key. Every HTTP candidate in the
roster already speaks the OpenAI shape (`route.rs` posts
`/chat/completions`), so `/v1/models` is the one listing contract. The
result is named on the form: the discovered count, or the failure —
`401` (the key is wrong), unreachable (the base URL is wrong), or no
listing endpoint (some proxies don't implement it). A failed discovery
does not refuse the save — a provider may be added before it is funded
— but the save says so rather than pretending.

**Discovery is the catalog.** The response's `data[].id` list is
written to `config.json providers[id].models`, replacing hand-typing as
the source of truth. A **reload models** action per provider
re-discovers on demand — a provider that ships new models tomorrow gets
them on the next reload, which is the steer's own promise. Lane
candidates (`blend1`) have no HTTP listing surface and are excluded
from discovery; their model entry stays as declared.

**The settings the provider declares ride the discovery.** Fleet
already derives the compaction budget from a per-model context window —
but the source is a hand-maintained compiled-in table
(`CONTEXT_WINDOWS`, `src/route.rs:88`), whose own doc comment names the
carve-out this decision closes: the OpenAI-compatible `/v1/models`
carries no such fact, *where fleet reads it today*. Discovery reads
what the provider actually declares, per provider, best-effort: an
OpenRouter-style listing's `context_length` and `max_output_tokens`
fields, and ollama's native `POST /api/show` (`context_length`) for the
roster's ollama candidates — the exact source that table's comment
already names. What is found is written per model into a new
`model_settings` map in the provider's config entry
(`{context_window, max_output_tokens}`), shown in the provider editor
with its provenance (`from the provider` / `hand-set`), and a hand
override wins over the provider's word. What the provider does not
declare stays unset, and the compiled-in behavior is unchanged — the
table remains the fallback, so the own-home declaration and the
provider's word never fight: config first, table second, `None` third.
The output ceiling is sent as `max_tokens` on requests **only when it
is declared** — a value fleet never sends today — so the wire changes
only for models whose provider named the number, and the editor can
clear it.

**Enable/disable per model, with the steer's default.** `models` stays
the provider's full discovered catalog; the owner's off list is a new
`disabled_models` per provider. The default is ON — *"it should add all
the models"* — so a model discovered tomorrow is usable without a
visit, and disabling is the deliberate act. The roster load filters
`disabled_models` out of `Candidate::models`, so a disabled model never
enters a chain, a pin, or a workload picker; the walk's existing
`unusable()` handling covers a model a provider later drops.

**Primary/secondary per workload is the walk order, said out loud.**
`routing.json`'s ordered candidate list is already the walk's
preferences — no new file shape, because order *is* the truth the
router reads. The change is the surface: per workload, the assigned
chain reads **primary** (the head, marked) then **secondary** (the
walk's tail), reorderable, addable only from the enabled catalog across
providers (a `provider/model` picker), removable. The file's shape is
untouched; the UI stops making the owner infer the roles from list
position.

**The judged-vocabulary constraint, named in the UI.** The workload
names sys1 judges are a closed set (`quick`, `standard_code`,
`frontend`, `hard`, `prose`, `deep_context` — the vendored route task's
label list) plus fleet's own `vision`/`composed`. A routing.json
workload outside that set is reachable only by pin or the
default-workload gate — never by judgment — so the workloads surface
marks such a workload **unjudged** instead of implying the router routes
to it. Adding judged labels is a sys1 task change and out of scope.

**The parallel-session hazard is a named risk, not a surprise.**
`bridge.rs`, `app.js` and `harness.mjs` are shared surfaces with other
in-flight work; the plan's edits git-diff those files at edit time and
fold complementary hunks deliberately (the 2026-10-31 lesson).

## Increments

Each lands whole — code, checks, docs — and each is green on its own.

### P0 — the key lands through settings, the provider is verified

The provider form gains a key **value** field (write-only, placeholder
"paste the key — it is stored, never shown"); the `provider` write
action accepts it, resolves the secret's name (`<ID>_API_KEY`,
overridable by the existing key-env field), writes it through the vault
seam, and runs discovery before replying. The reply names the outcome:
`discovered N models`, or the failure — `the key was refused (401)`,
`the base URL did not answer`, `the provider lists no models` (the
non-listing case, with hand-typed models staying as the fallback). The
key-env-name field stays for owners who manage keys outside fleet; a
save with an env name and no value skips the write and the discovery.

*Checks:* P1, P2.

### P1 — discovery is the catalog and its settings, and the walk honors the off list

The discovered `data[].id` list writes `providers[id].models`; the
per-provider **reload models** action re-discovers and rewrites. Where
the provider declares model settings, discovery reads them too: the
listing's `context_length` / `max_output_tokens` fields when present,
and ollama's `POST /api/show` for the ollama candidates — written to
the provider's `model_settings` map with the values the provider
named, nothing guessed. The provider form's models chips become the
catalog read (with a disable/enable toggle per model) over the new
`disabled_models` write, and each model's row shows its declared
settings with a hand override beside them. The roster load filters
`disabled_models` out of `Candidate::models` (`src/route.rs`'s config
merge), so the walk, the pin resolution, and `fleet providers` all see
only enabled models.

*Checks:* P3, P4, P5.

### P2 — the declared settings act

`context_window()` reads the provider's `model_settings` first, the
compiled table second, and returns `None` third — the compaction
budget and the turn loop's window check follow it with no change of
their own, which is the whole point: a model discovered with a
declared window gets a real fold fraction the moment it serves, where
today it would fall back to the floor. The output ceiling is sent as
`max_tokens` on the chat request only when the serving model declares
one — a field fleet never sends today — and clearing the value in the
editor stops sending it. Both are pinned over the stub listing
endpoint and the request bodies the mock sees.

*Checks:* P6, P7.

### P3 — the workloads surface: primary and secondary, per workload

Each workload's editor reads its `routing.json` chain as **primary**
(the head, marked as such) then **secondary** entries; a picker adds
`provider/model` pairs from the enabled catalog only; reorder moves
primary with the head; remove takes an entry out. The add-workload form
keeps working, and a workload whose label is outside the judged set is
marked **unjudged** with the reason (reachable by pin or the default
gate only). The `workload` write action is unchanged — the file's
ordered-candidates shape already carries everything this surface needs.

*Checks:* P8, P9.

### P4 — the harness, the measurement, the docs

A harness case per new control: paste-key save (the vault write named),
discovery success and each named failure, the model disable toggle, the
reload action, the primary mark, the reorder, the picker's
enabled-only list. The before/after control counts quoted. The docs:
`docs/features/`'s settings passage (or the router's, wherever the
providers card is documented) and `TECHNICAL-DOCUMENTATION.md`'s
settings entry.

*Checks:* P10.

## Use cases

- [ ] A: The owner adds a provider, pastes its key, and the provider's
      full model list appears — nothing hand-typed.
- [ ] B: The owner disables a model and no walk, pin, or picker names
      it again.
- [ ] C: The owner assigns a model **primary** for `hard` and the next
      `hard`-judged run walks it first, with the previous chain behind
      it as secondary.
- [ ] D: A wrong key or a wrong base URL is named on the form — never a
      silent save.
- [ ] E: A provider whose listing declares `context_length` serves a
      discovered model whose compaction budget is derived from that
      window, with no hand edit — and the editor shows where the number
      came from.

## Acceptance criteria

- [ ] **P1 — the key is stored, never shown.** A save with a key value
      writes the secret through the vault seam under the provider's key
      name (vault when the passphrase is present, `.env` otherwise);
      every read model carries `key_present` and never the value; the
      write-only field does not echo it back. Pinned by a settings test
      and a harness check on the read.
- [ ] **P2 — discovery is named, both ways.** A key that lists answers
      with the model count and writes the catalog; a `401`, an
      unreachable base URL, and a no-listing provider each name
      themselves on the form and in the response; a failed discovery
      still saves, with the failure recorded. Pinned by a bridge test
      over a stub listing endpoint.
- [ ] **P3 — the off list filters the walk.** A model in
      `disabled_models` is absent from the loaded roster's
      `Candidate::models`; a pin naming it meets the existing named
      refusal; a workload chain entry naming it reads as unusable in
      the walk. Pinned by a roster-load test.
- [ ] **P4 — reload is the catalog's heartbeat.** Reload re-discovers
      and rewrites `models`; a model the provider stopped listing
      leaves the catalog (and a workload naming it reads unusable); a
      new model appears enabled.
- [ ] **P5 — the declared settings are stored with provenance.** A
      listing that names `context_length` (or ollama's `api/show`) writes
      `model_settings[model].context_window`; a hand override edits the
      same field and wins over the provider's word on every later
      discovery; what the provider does not declare stays unset. Pinned
      by a discovery test over the stub endpoint.
- [ ] **P6 — the window acts.** `context_window()` resolves config
      first, the compiled table second, `None` third; a discovered model
      with a declared window derives its compaction budget from it (the
      turn loop's window check follows); an undeclared model behaves
      exactly as today. Pinned by a route test reading a config-written
      window over the table's own entry.
- [ ] **P7 — the output ceiling is sent only when declared.** A model
      with `max_output_tokens` declared rides the request as
      `max_tokens`; a model without it rides no such field; clearing the
      editor value stops the send. Pinned by the mock's request bodies.
- [ ] **P8 — primary is the head, secondary is the walk.** The
      workloads editor marks the head primary; reorder writes the new
      order to `routing.json` and the file's shape is unchanged; the
      next judgment walks the new head first. Pinned by a write test
      reading the file back.
- [ ] **P9 — the picker is the enabled catalog.** The workload picker
      offers `provider/model` pairs over enabled models only — a
      disabled model is not addable, on any provider.
- [ ] **P10 — the gate and the docs.** fmt, clippy, build, test green by
      exit code; the harness cases green with the counts quoted; the
      docs updated in the same landing.

## Files to be touched

**Backend:**
- `src/settings.rs` — `set_provider` gains the key-value write (the
  vault seam), `disabled_models`, and `model_settings`; the discovery
  helper (the `/v1/models` fetch, the optional metadata read, ollama's
  `/api/show` leg, named failures)
- `src/gui/bridge.rs` — the `provider` write action carries the key and
  the discovery result; `router_document`'s read model (the catalog,
  the off list, `key_present`); the workloads read carries primary/
  secondary as the ordered chain already is
- `src/route.rs` — the config merge filters `disabled_models`;
  `context_window()` resolves `model_settings` first, the compiled
  table second
- `src/api.rs` — `ChatRequest` gains the optional `max_tokens` field,
  set only when the serving model declares one

**Frontend:**
- `src/gui/assets/app.js` — the provider form's key field, the model
  toggles, reload; the workloads editor's primary mark, reorder, and
  enabled-only picker; the unjudged mark
- `src/gui/assets/app.css` — only if the primary mark needs a token

**Tests:**
- `tests/face/harness.mjs` — the per-control cases (P7)
- `src/settings.rs` module tests — the vault write, the discovery
  failures over a stub endpoint
- `src/route.rs` module tests — the off-list filter

**Documentation:**
- `docs/features/` — the settings passage's providers and workloads
  sections
- `TECHNICAL-DOCUMENTATION.md` — the settings entry

**Note for the implementer:** `bridge.rs`, `app.js` and `harness.mjs`
carry parallel work; git-diff each at edit time and fold complementary
hunks deliberately, crediting them in the commit.

## Out of scope

- **New judged workload labels** — the closed set lives in the vendored
  sys1's route task; adding labels is a judgment-surface change with
  its own plan.
- **Non-OpenAI discovery** — providers that list by other means (a
  GraphQL catalog, a dashboard export) stay on hand-typed models; the
  named no-listing failure is their path.
- **Lane-candidate discovery** — `blend1`'s model entry stays declared.
- **Cost/spend display per model** — the calibration surface's job, not
  the catalog's.
- **Mid-run provider changes** — the existing run-scoped rule stands: a
  config change applies to the next run, never the one in flight.

## Verification

**P1/P2** — the vault write and the named discoveries:
```bash
cargo test --lib secret && cargo test --lib discovery
```
Expected: green over a stub listing endpoint — the sealed write, the
count, the 401, the unreachable, the no-listing.

**P3/P4** — the off list:
```bash
cargo test --lib disabled_models
```
Expected: the filter, the pin refusal, the unusable chain entry.

**P5/P6/P7** — the declared settings:
```bash
cargo test --lib model_settings
```
Expected: the write with provenance, the override winning, the window
resolved config-first, the compaction budget following it, and the
request body carrying `max_tokens` only for a declared model.

**P5/P6** — the surface:
```bash
node tests/face/harness.mjs
```
Expected: every new control green; the counts quoted in this plan.

**P7** — the gate, whole:
```bash
cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo build && cargo test
```
Expected: exit 0 each.

## Linked artifacts

- `docs/plans/2026-10-29-settings-rail.md` — the surface idiom this
  plan's cards live in (`sectionRail`, `cfgItem`, the pane)
- `src/route.rs:839` — the `routing.json` contract P2's surface presents
- `src/settings.rs:1490` — the vault seam P0 writes through
- `docs/plans/2026-10-30-surface-curation.md` — the walk's reading of
  preferences and the label-level cache, verified there

## Risks

- **A discovered window is wrong and shrinks the compaction budget.**
  Mitigation: the editor shows the value's provenance (`from the
  provider` / `hand-set`) beside it, the hand override wins on every
  later discovery, and the compiled table stays the fallback for
  anything undeclared — no provider word can erase a fleet declaration
  the owner has not overridden.
- **`max_tokens` truncates a long answer.** Mitigation: the field is
  sent only when the provider declared it, the editor shows and clears
  it, and the compiled-in behavior (no such field) is every undeclared
  model's.
- **A key value transits the browser.** Mitigation: the field is
  write-only and never echoed; the secret lands in the vault or `.env`
  under the provider's name; the read model carries `key_present`
  only. The face is localhost-served, but the rule is the value never
  comes back.
- **Discovery against a non-listing provider fails.** Mitigation: the
  named no-listing failure keeps hand-typed models as the fallback,
  and the save proceeds with the failure recorded rather than refused.
- **The catalog drifts from the provider between reloads.** Mitigation:
  reload is one press; a dropped model that a workload still names
  reads as unusable in the walk (the existing named handling), never a
  silent failure.
- **Parallel sessions on the shared surfaces.** Mitigation: the
  named implementer note — diff at edit time, fold deliberately.
- **The owner disables a model a workload depends on.** Mitigation: the
  picker is enabled-only going forward, and the chain entry naming a
  disabled model reads unusable in the walk — named, not silent; the
  workloads editor marks such an entry on read.

## Dependencies

- The settings revamp (`2026-10-29-settings-rail.md`, completed) — the
  rail, the write path, the pane.
- The vault seam — landed and tested (`src/settings.rs:1490`).
- The router's config merge and `routing.json` — landed, unchanged by
  this plan's writes.
