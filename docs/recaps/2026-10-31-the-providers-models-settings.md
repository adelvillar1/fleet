# The provider's models, discovered: a key in settings, the catalog out

**2026-10-31** · the day's third session — after the shakeout's pass
two and the conversation's density, the settings rethink the owner
asked for: *add a provider and a valid key in Settings and it loads all
the models*, then enable/disable and assign workloads with **primary**
and **secondary** named — with the steer *"it should add all the models
from the provider"* and the amendment *"auto configure the model's
settings like context window, output size, etc. if that information is
available from the provider."* The plan was drafted, amended, and
implemented in one session: paste-key → vault → `/v1/models` verified →
catalog + declared settings stored → per-model toggles → primary/
secondary on the workload cards. Gate green at **1,854 tests / 0
failed**; the face harness at **525 checks across 54 cases**; ledger
clean at **384 rows, 36 open**.

## The plan, and its criteria

`docs/plans/2026-10-31-provider-models-settings.md` — **completed**,
15/15 boxes, walked in the goal's completion audit with fresh evidence
per criterion:

- **P1** the key lands through the vault seam and is never read back —
  pinned by `a_save_with_a_key_stores_it_and_merges_the_catalog` and the
  read models' `key_present`.
- **P2** discovery named both ways — `discovery_reads_the_catalog_and_its_declared_settings`
  (the Bearer line on the wire) and `a_refused_key_is_named_by_the_discovery`
  (`the key was refused (401)`); the save proceeds with the failure
  recorded.
- **P3** the off list filters the walk — `a_disabled_model_leaves_the_roster_at_the_load`.
- **P4** reload rewrites the catalog and the hand override survives —
  `a_hand_override_survives_the_rediscovery`.
- **P5** provenance rides the values (`provider` / `hand`).
- **P6** the window resolves config → table → `None` —
  `the_declared_window_resolves_config_first`; `serving_window` reads it.
- **P7** `max_tokens` is sent only when declared — both wire shapes
  pinned in `stream_request_carries_the_stream_and_usage_flags`.
- **P8/P9** the harness drives the assignment: the picker offers the
  enabled model only, and raising a secondary writes the swapped order
  (`b/two` before `a/one`) to the unchanged `candidates` shape.
- **P10** the gate — fmt 0, clippy 0, build 0, test 1,854/0, harness
  525/54 — and the docs landed in the same commits.

## Commits

| Hash | What |
|---|---|
| `91b7db1` | the plan: a provider and a key, the catalog, enable/disable, primary/secondary |
| `81955e4` | the amendment: the declared settings ride the discovery (context window, output ceiling) |
| `33a5679` | the landing: P0–P3, 13 files, the discovery engine, the two editors |
| `fc420a4` | the audit's two pins: the picker is the enabled list, the reorder writes |

## What was added

**The key's round trip.** The provider card's paste field writes the
value through `set_secret` — sealed into the vault when the process
holds the passphrase, one atomic `.env` line otherwise, under the
provider's key name — then verifies it with `GET {base}/models` before
saving. The reply names where the key landed and what listing said; a
`401`, an unreachable base, and an empty listing are three distinct,
named sentences on the card.

**Discovery is the catalog.** `discover_provider_models` reads
`data[].id` plus the settings a provider declares (`context_length`/
`max_output_tokens`, with the `max_model_len`/`max_completion_tokens`
variants, and ollama's native `POST /api/show` leg for `:11434` bases)
into `model_settings` with per-field provenance. `rediscover_provider`
is the reload — the same key resolution the walk uses.
`disabled_models` rides the config merge and the roster load filters
the set out of `Candidate::models`, so a disabled model leaves every
walk, pin, and picker while a model discovered tomorrow is usable by
default.

**The declared settings act.** `declared_window`/`declared_output`
resolve config first, the compiled `CONTEXT_WINDOWS` table second,
`None` third — a discovered model's compaction budget follows its
declared window the moment it serves. `ChatRequest` gained the optional
`max_tokens`, sent only when declared: a field fleet never sent before.

**The two cards say the roles.** The workload card names the head
**primary** and the rest **secondary**, reorders them, and adds
secondaries from the enabled catalog only; a workload outside
`JUDGED_WORKLOADS` (the six the sys1 route task answers) is marked
**unjudged** with its reason. The provider card carries the key field,
the catalog rows with toggles and declared settings, and the reload.

## Files changed

- **Backend**: `src/settings.rs` (the discovery engine, the key write,
  the rediscovery, the read-model fields), `src/route.rs`
  (`ModelSettings`, `disabled_models`, the merge filter,
  `declared_window`/`declared_output`, `JUDGED_WORKLOADS`), `src/api.rs`
  (`ChatRequest.max_tokens`), `src/agent.rs` (the request's ceiling,
  the config-first `serving_window`), `src/gui/bridge.rs` (the two
  intercepted ops, the read models), `src/main.rs`, `src/eval.rs`
  (literal updates).
- **Frontend**: `src/gui/assets/app.js` — the live `providerSections`
  card (key, catalog, reload), the `tierSections` assignment editor;
  the dead `providersCard`/`providerEditor` pair deleted.
- **Tests**: `tests/face/harness.mjs` (the PM case, 4 checks), the
  settings/route/api pins.
- **Docs**: `docs/features/routing.md` (the settings-surface section),
  `TECHNICAL-DOCUMENTATION.md` (the settings entry), the plan.

## Doc updates applied

- `docs/features/routing.md` § "The settings surface: a provider, a
  key, its models" — the full flow: the vault seam, discovery and the
  named failures, declared settings with precedence, the off list, and
  the primary/secondary/unjudged surface.
- `TECHNICAL-DOCUMENTATION.md` § `settings.rs` — the discovery
  sentence: `set_provider_with_key`, `discover_provider_models`,
  `rediscover_provider`, and the provenance rule.

## Doc updates proposed (pending the owner's approval)

### Proposed update: `FUNCTIONAL-SPECIFICATIONS.md` § "the settings surfaces" (~line 143)

```markdown
Open a provider's card, paste its key, and save: the key is stored
(vault or .env, never shown again) and the card comes back with every
model the provider serves, each with an on-off toggle and the window
and output ceiling the provider declared. A reload re-lists on demand.
Each workload's card names its primary — what a judged walk tries
first — and its secondaries, reorderable, addable from the enabled
models only; a workload name outside the judged set is marked as
serving only by pin or default, never by judgment.
```

## CLAUDE.local.md changes

None this session.

## Open questions / next steps

- **The discovery against the real roster is untested on real
  endpoints** — every pin runs over the stub listing. The owner's first
  real paste-key (zai, stepfun, or an added provider) is the live proof;
  the failure sentences are ready to show.
- **Drex's key is still absent** — the sys1 panel and the route keep
  falling to the two judges that answer; unrelated to this plan but
  named here.
- **`corpus-refresh-dev-decisions`'s 2026-10-30 failure** still stands
  unexamined (the store is current, so it cost nothing).
- Two long-running orphans reported by the sweep, left alone by rule:
  an `npx node-code-sandbox-mcp` (5 days) and blend1's own service venv
  (3 days) — both another session's or services, not this one's.

## Notes

- **A brace-matching patcher is the wrong tool for code edits.** The
  literal-patching script I wrote for `ChatRequest`'s 15 sites
  corrupted six files (it swallowed code after closing braces);
  every hit was repaired by hand from `git diff` and the suite proved
  it — patch structured files structurally, or with the compiler as the
  guide.
- **Dead code that looks live**: `providersCard`/`providerEditor` had
  zero callers (the pane card replaced them months ago) — my form edits
  landed there first. When rewritten code "doesn't run", grep for the
  function's callers before extending it; the pair was deleted in the
  same edit.
- **The harness's fake DOM matches attribute selectors against element
  *properties*** — `aria-label`/`data-act` queries can never match;
  use `El.find` property predicates. A `page.el(id)` accessor was added
  for pane contents (the registry, not the tree, holds them).
- Sweeps: process registry **0 entries** (nothing registered to kill);
  cache sweep **free space 53.0 → 53.1 GiB**, release untouched.
- The working tree's `memories/` modifications are the consolidation's
  recall-count bumps (runtime state, deliberately left) and
  `scripts/refresh-cruiserintelligence-*` belong to another session.
