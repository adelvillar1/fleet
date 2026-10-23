---
status: active
created: 2026-10-23
updated: 2026-10-23
slug: tool-surface-and-lane-resolution
---

# Plan: the tool surface, resolved by the lanes

## Goal

Give the model a real action space and stop every part of a tool
call that is currently a guess: the harness resolves the tool by exact
name, declared alias, or lane-ranked retrieval; grounds every argument
in a catalog fleet already holds; dispatches capability calls to the
best available lane; and records each resolution so the improvement is
measurable.

The analysis behind it is
`docs/architecture/tool-surface.md`. This plan implements it and adds
the lane-resolution layer.

## Context

### What the model has today

Four built-ins — `bash`, `read_file`, `write_file`, `list_dir`
(`src/tools.rs:1150-1159`) — plus eleven beside-the-four tools declared
by run shape (`src/agent.rs:1317-1400`), plus whatever MCP lanes have
joined. `read_file` takes only `path` and reads the whole file
(`src/tools.rs:1642-1648`); `write_file` replaces the whole file
(`1693`); `bash` takes only `command` and is capped at 120 seconds
(`20`); there is no search primitive. One 64 KiB bound is applied after
the full output is already in memory (`17`, `1303-1320`).

The loop around them is stronger than the tools: calls are batched into
waves, writer conflicts deferred (`src/agent.rs:2346-2401`,
`4020-4066`), and concurrency capped at three (`133`).

### The six guesses this plan removes

These are the concrete meanings of "a guess", each provable in the
tree:

| Guess | Why it is a guess | Removed by |
|---|---|---|
| A vendored lane's tool is called by a name the model learned from memory | Lanes join lazily, so their tools are absent from the first request's declarations (`src/agent.rs:4780-4818`); the model cannot see `sys1_classify` unless it recalls the name | Capability resolution: the model names an intent; the router calls the lane. Lane identity never enters the declaration list |
| Search is `bash grep` | The four built-ins have no search, and shell output is unbounded then cut (`src/tools.rs:1475-1527`) | `grep`, `glob` with caps, counts and truncation flags |
| Arguments are invented | Schemas carry free strings; fleet's catalogs (`settings::catalog`, `1903`) are never in a tool schema; no schema is validated locally | Enums from the catalog, defaults with their source named, path suggestions from the real tree, `error[validation]` naming the field |
| One lane is chosen by whoever is named | `mcp__<lane>__<tool>` is a name convention, not a dispatch policy (`src/tools.rs:1033-1037`) | A capability registry with candidates, health, warm-first order, and a named reason |
| "No tool fits" is never an option | Every unknown name is a dead end: `error: unknown tool \`x\`` with no list (`src/agent.rs:2786`); same for skills and lanes | An explicit abstention result, plus nearest-name and alias resolution |
| A call's quality is unknown | The log records tool results, but not how the tool was chosen, at what cost, or whether the model recovered | A `ToolResolution` record per call and `fleet tools audit` rates |

### The MCP manager slice, and its state

Another session owns the MCP manager. Its plan,
`docs/plans/2026-10-22-mcp-server-registry.md`, is partially built on
this tree:

| Increment | State | Evidence |
|---|---|---|
| 1. Registry, validation, writers, `fleet mcp` list/enable/disable/remove | landed | `a063f66`, `src/mcp_registry.rs:98-326` |
| 2. Discovery and import, names-and-shapes only | landed | `82b7679`, `src/mcp_registry.rs:528-1103` |
| 3. Run wiring — enabled servers pre-join | landed | `32a1099`, `enabled_specs` at `src/mcp_registry.rs:156` |
| 4. Health probe | in the working tree, uncommitted | `src/cli.rs`, `src/main.rs`, `src/mcp_registry.rs` and `tests/integration.rs` carry the probe |
| 5. Settings board card | the last phase | no `servers_document` and no servers card in `src/gui/assets/app.js` yet; the manager plan flips to completed when the card lands |

That changes this plan's job in one important way: the registry and
discovery are consumption seams, not work to duplicate. Because the
manager is in its last phase, this plan is sequenced behind it:
increments 1–3 touch none of its files and can start now; the face
work waits for the servers card to commit, so the two sessions never
edit the same function in flight.

One real conflict remains, resolved in D12. Their increment 3 declares
every enabled server's tools from the first request, and their C2 says
so. This plan tiers large catalogs behind `tool_search`. Below the
threshold both are true; above it, reachability replaces declaration.
The manager delivers its contract as written; increment 4 here is the
supersession, recorded in `docs/features/mcp.md` when it lands, and the
owner decides then whether the completed manager plan's C2 is amended.
Rewriting a delivered criterion to fit a later plan is not how the
ledger should work.

### What the research says

The numbers that order this plan, all from the analysis:

- Interface choices moved SWE-bench Lite by up to 7.7 points: no edit
  tool 10.3, whole-file view 12.7, iterative search 12.0, versus 18.0
  with windowed reads, a linted edit tool and capped search.
- Content-anchored edits corrupt 0.01% of files in 8,320 trials;
  line-number-anchored edits corrupt 99.1% under a one-line shift
  (arXiv 2609.11957). So `edit` is content-anchored, and line numbers
  are a reading aid only.
- Tool retrieval quality dominates selection: NDCG@5 84.9 for a
  semantic retriever versus 17.0 for BM25 (ToolLLM, arXiv 2307.16789);
  Gorilla's BM25 retriever dropped AST accuracy to 17.03% against
  91.26% with oracle docs. Geometry over generated guesses.
- Selection degrades past 30–50 visible tools, and 5 MCP servers cost
  ~55k tokens of definitions (Anthropic). Unfiltered 158-tool exposure
  dropped OSWorld-MCP success 20.5% → 15.5%.
- Returning a section instead of the whole output cost 7.9–11.3 points;
  naming the authoritative output file recovered 6.3–7.9 (arXiv
  2610.26836).
- Tool descriptions are part of the interface: 97.1% of 856 MCP
  descriptions had at least one smell, and rewriting them raised median
  success 5.85 points (arXiv 2602.14878).
- Silent tool errors roughly halve success versus loud ones (RITE,
  arXiv 2610.27777). Error classes and hints are not polish.

## What we should do

Seven increments, each gated and committed. The review surfaces, in
order of when they become visible:

1. **The ledger** — after increment 1, `fleet tools audit <session>`
   prints what the model asked for, what resolved, and how.
2. **The action space** — after increment 2, the model can search,
   read a window, and edit a file without rewriting it.
3. **The channel** — after increment 3, a tool is one module and a bad
   argument names its field.
4. **The catalog** — after increment 4, forty tools are declared as
   twenty-eight and the rest are one search away.
5. **The lanes** — after increment 5, the retrieval is geometry and the
   dispatch is a policy; nothing in the path is generated.
6. **The grounding** — after increment 6, no argument is a free guess.
7. **The protocol and the report** — after increment 7, MCP results are
   typed, and the rates are on a card.

The slice is not delivered until the Settings tool card and the
transcript's resolution line are live on the loopback face (hard rule
11), and the audit's numbers are printed from a real session.

## Approach

**D1 — the model names intent; the harness resolves the call.** One
resolution path, in order: exact declared name; declared alias; catalog
retrieval (increment 5); refusal. A call never reaches execution
without a `ToolResolution`. The model does not choose a lane, a
transport, or a version of a tool.

**D2 — lanes are the resolver, not tools the model calls.** No new
model-visible lane tool is added. Three lane roles, each with an
existing contract:

- **sem1 ranks** by embedding the query and the tool cards
  (`sem1_embed`, which takes a batch and returns vectors with dims and
  model — `services/sem1/service/mcp_server.py:95-145`).
- **sys1's existing route verdict** selects a tool profile: the
  workload it already judged at run start (`src/route.rs`) reorders the
  core set and biases retrieval. No new sys1 task is created, so the
  vendored service and `services/sys1/VENDOR.json` are untouched.
- **dispatch** prefers a lane that is already warm, then a healthy one,
  then registry order.

A capability-class task in sys1 would be a cleaner coarse decision, but
its task vocabulary is static (`services/sys1/src/sys1/tasks.py:565-590`,
`services/sys1/service/mcp_server.py:59-99`) and adding one is an
upstream change plus re-vendor. It is named as a follow-up, not a
prerequisite: the plan must not depend on a lane change to ship.

**D3 — the catalog is one snapshot with provenance and a tier.** A
`ToolCatalog` built at run start from the run's admitted native tools,
the tools of enabled MCP servers only, and the run's activated set.
Each entry carries `source` (`native` / `mcp{server}`), `policy`
(read-only, writes, shell, network, always-ask), `tokens` (definition
size estimate), `tier`, and the canonical spec. Ordering is
deterministic: native in declaration order, then servers by name, then
tools by name. Health, warmness, and activation never reorder an
existing entry; they only decide what is added.

**D4 — tiers: core is declared, the rest is searchable.** Core is the
eight built-ins plus the beside-the-four tools the run shape admits.
Everything else is deferred when the catalog crosses either bound:
more than 28 visible tools, or more than 8,000 estimated definition
tokens (characters / 4). `tool_search` is declared only when something
is deferred. Its result names the activated tools; the next request
declares them, appended in activation order. This is the answer to the
MCP manager's named risk ("every enabled server's tools ride every
run") without changing their files: one small server stays fully
declared, a large one costs one search.

**D5 — retrieval is geometry, not generation.** `tool_search(query)`
embeds the query with `sem1_embed` and ranks the deferred cards by
cosine. Card vectors are cached in `<FLEET_HOME>/tool-cards.json`
keyed by a hash of (name, description, schema), so the batch embed runs
when the catalog changes, not per search. The result carries the score
and the ranking source. When sem1 is unavailable, a lexical ranker
(token overlap over name and description) runs and the result says so.
No language model is in this path; a test proves resolution completes
with the model client unreachable.

**D6 — abstention is a rule.** If the best score is below the floor, or
the top two are within the margin, the result says "no tool fits" and
names the nearest two with their scores; it does not activate either.
The model may then act without a tool, or call one of the two by name
(a grounded choice, not a guess). The floor and margin are constants
tuned on a fixture corpus and recorded in the resolution row.

**D7 — the profile comes from a verdict that already exists.** The
run's `RouteDecision` workload selects one of a small number of
profiles: which capabilities are ordered first in the core set, and
which class retrieval boosts. The decision is already in hand at run
start — `self.client.route_decision()` (`src/agent.rs:1636`, the
method at `src/api.rs:588`) — so no extra ask is added. A
`standard_code` verdict orders edit, grep, read, bash; a `prose` verdict
orders web, read, write; a `deep_context` verdict orders grep, glob,
read. A profile never removes an ability and never changes an existing
tool's name. An absent or low-confidence verdict selects the default
profile and says so.

**D8 — dispatch is a policy.** A capability with more than one provider
(e.g. `web`: blend1's web leg, then fleet's own fetch) resolves by:
already-warm provider, then healthy, then declared order. Failover is
allowed only for read-only or open-world capabilities, and each attempt
is recorded. A write capability whose provider fails refuses by name
rather than trying a second provider, because a half-applied write in
two places is worse than a refusal. The chosen provider and the reason
ride the result header so the owner can see who answered.

**D9 — arguments are grounded.** Every closed set that fleet knows
becomes a schema `enum`: MCP server names (enabled only), tool names
for `tool_search`, todo statuses, `computer_use` actions, lane-backed
provider names. Values come from `settings::catalog` (models,
providers, workloads, env names, paths, tools, skills, shapes) and from
`mcp_registry::read_servers` for server names. Defaults declared in
`lanes.json` or a server's own defaults are applied and named in the
result ("used `sem1_embed` default model google/embeddinggemma-2").
A path that does not exist returns the nearest actual entries in its
parent directory, computed by string distance over the real listing.
There is no model in argument repair; a bad argument is a typed error
with valid values.

**D10 — results are typed; the text is composed from the type.** A
`ToolResult` carries `text`, `structured`, `truncated {omitted, artifact}`,
`artifacts`, and `error {class, message, hint}`. The model still
receives text, because every provider accepts text; the log receives
the structure. Error classes: `validation`, `denied`, `not_found`,
`timeout`, `conflict`, `lane_dead`, `internal`. An MCP server's
`structuredContent` is validated against its `outputSchema` when it
declares one, and kept beside the text.

**D11 — context treats results as artifacts.** Before a fold, tool
results older than the last six are replaced by a stub naming the tool,
byte count, and artifact path; `bash` and oversized reads spill to
`<FLEET_HOME>/sessions/<id>/calls/<n>.out`. The todo ledger is
re-injected after every fold. Recovery is `read_file` on the artifact,
a tool the model already has.

**D12 — the MCP manager owns its slice; this plan consumes it.** The
boundary is explicit because both plans touch `src/agent.rs`:

| Area | Owner | Rule here |
|---|---|---|
| `src/mcp_registry.rs`, `mcp.json`, `fleet mcp` verbs, the settings servers card | MCP manager | read `enabled_specs` and `read_servers` only; never edit |
| Which specs join at run start | MCP manager (landed, `32a1099`) | seed already happens; this plan decides visibility |
| `declarations()` ordering and tiers | this plan | one function, used by both |
| `src/tools.rs`, new `tool_*` modules, `src/mcp.rs` protocol, `src/compaction.rs`, the session record | this plan | build |
| `src/gui/assets/app.js`, `app.css` | both, in that order | the manager's servers card in `paintSettings` is its last phase and lands first; this plan's transcript resolution line and Settings tool card follow in increment 7, after that commit |
| `src/cli.rs`, `src/main.rs` | MCP manager, in flight | this plan adds one verb, `fleet tools`, after their health verb lands |

Two non-negotiables. First, a discovered-but-unconfigured entry is a
name in the UI and nothing else: it never enters the catalog and can
never be dispatched. Second, an enabled server's tools enter the
catalog with `Source::Mcp { server }`; namespacing, collision handling
and reserved names stay the registry's job (`src/mcp_registry.rs:42`).

**D13 — prompt stability is part of the design.** The core declaration
list is byte-stable for the same profile and catalog, so the prompt
cache survives a run. Only `tool_search` activations append, and only
once each. A server that joins mid-run does not reorder the prefix.

### The decision-model check

The per-item decisions in this plan, and where they go:

| Decision, repeated per call | Where it goes | Why not a model |
|---|---|---|
| Which of N tools fits this query | sem1 cosine over cached cards | geometry is exact and cheap; a generated choice is the thing being removed |
| Does any tool fit | a similarity floor plus a margin | a declared rule, recorded; the model may then choose between two named candidates |
| Which provider serves this capability | warm → healthy → registry order | a policy over observable state |
| Is this argument a known value | catalog lookup and string distance | the value is already in fleet's hands |
| Which tools to declare upfront | profile plus catalog tier | deterministic, cache-stable |

There is one decision this plan deliberately does **not** send to a
model: the coarse capability class (read / search / edit / execute /
web / …). sys1 is the right lane for a closed-set label, but its task
vocabulary is static and the capability-class task does not exist; the
plan uses the route verdict sys1 already produced instead, and names
the class task as a follow-up. No decision in the resolution path is
generated.

## The app

Mockups to approve before a line is built. All three read from the same
resolution record.

**The transcript's tool row.** The collapsed line gains how the call
was resolved, after the existing duration:

```
✓ edit src/tools.rs · 34ms · exact · drift-guarded
✓ mcp__sdm1__sdm1_score  · 180ms · retrieved 0.88 (sem1) · profile: code
✓ bash "cargo test --lib" · 41s · exact · spilled 214 KiB → calls/12.out
✗ str_replace src/main.rs · not found — nearest declared tool: edit
```

Clicking opens the resolution detail: requested name, resolved name,
method, score and ranker, profile, argument sources (model or harness
default), truncation and artifact.

**The Settings tool card.** Beside, not inside, the MCP servers card
(the manager owns that one): a card titled *the tools*, with the
declared count and the deferred count, one row per visible source
(native, then each enabled server with its count), the profile the
last run used, and one action: *audit this session*.

```
the tools                         19 declared · 21 deferred
  native                              8   core
  beside the four                    11   core
  sem1 (enabled)                      3   deferred
  playwright (enabled)               12   deferred
  profile: standard_code · threshold 28 tools / 8k tokens
  [ audit this session ]
```

**The audit card.** `fleet tools audit <session>` and the card action
render the same numbers: schema-valid rate, argument-error classes,
p95 result bytes, truncation share, redundant-call rate, recovery rate
after an error, resolutions by method (exact / alias / retrieved /
refused), and the calls per completed task.

## Increments

1. **Truth and the ledger.** Fix the five surface defects the analysis
   proved, then record every call's resolution and add the audit verb.
   Nothing about selection changes yet; this increment exists so every
   later claim is measurable.
2. **The core eight.** `bash` timeout and background with output
   retrieval, `read_file` window with line numbers, `edit`, `grep`,
   `glob`, `todo`. Implement them inside the current dispatch if
   increment 3 has not landed; shape their results as `ToolResult` so
   the move is not a rewrite.
3. **The channel.** The tool trait and registry, schema validation,
   typed results, error classes, artifacts, read-before-write tracking,
   a cancellation token, and a per-turn call cap.
4. **The catalog and tiers.** `ToolCatalog`, deterministic ordering,
   core/deferred split, `tool_search` with the lexical ranker and
   activation, the alias map, and visibility of enabled MCP tools only.
5. **Lane resolution.** sem1 retrieval with cached card vectors, the
   abstention rules, the sys1 profile, and the dispatch policy.
6. **Grounding and context.** Catalog-backed enums and defaults, path
   suggestions, result elision with artifacts, ledger re-injection.
7. **The protocol and the report.** MCP annotations, pagination,
   structured output, image artifacts, cancellation; the Settings tool
   card; the audit card; the live verification.

## Acceptance criteria

- [ ] **C0 — the surface stops lying.** `vision` refuses an absolute
  path and any path outside the run root, canonicalizes before reading,
  and sniffs the media type instead of defaulting to PNG
  (`src/agent.rs:923-975`). Plan mode refuses every file-writing tool by
  name — `write_file`, `edit`, `remember`, `supersede`, `improve`,
  `plan`, `computer_use` — and the plan-mode prompt plus the feature doc
  state the remaining boundary honestly: `bash` is allowed for reading
  and is not made read-only, so the guarantee is tool-level. Each
  refusal is pinned by a test, and the documented exception is pinned by
  a prompt test. `computer_use` attaches the screenshot as an artifact
  or its description drops the claim that the image rides to the model
  (`src/tools.rs:699-701` vs `src/agent.rs:3805-3816`). The TUI seeds
  user servers (`src/tui/mod.rs:842-849`). `supersede` matches its name
  exactly (`src/agent.rs:2680`). `write_file` writes atomically.
  Each is pinned by a test.
- [ ] **C1 — one resolution record per call.** Every call that reaches
  execution writes a `ToolResolution` with requested name, resolved
  name, method (`exact` | `alias` | `retrieved` | `refused`), reason
  (score, ranker, profile, or refusal), catalog size and tier, and
  argument sources. Pinned by tests for the four methods plus a denial;
  the record count equals the call count in a three-call session.
- [ ] **C2 — the audit prints the rates.** `fleet tools audit <session>`
  reads the log and prints schema-valid rate, argument-error classes,
  p95 result bytes, truncation share, redundant-call rate, recovery
  rate, and resolutions by method. A fixture session with injected
  counts produces exact numbers, not approximations.
- [ ] **C3 — ranged, line-numbered reads.** `read_file` takes
  `offset`/`limit`, returns line numbers, a `path (N lines)` header,
  and `(showing 1–200 of 3,841; continue with offset=201)`. A 4,000-line
  fixture read at offset 3,800 returns 200 lines with the correct
  numbers and a continuation line; the test asserts the bytes read are
  bounded by the window, not the file. Empty file, past-EOF, and binary
  files each return a named notice.
- [ ] **C4 — `edit` replaces one anchored passage.** It requires a
  prior read of the path this run, requires a unique match unless
  `replace_all`, returns the nearest line on zero matches, returns the
  match count and line numbers on multiple, writes atomically, and
  returns a capped diff. Six tests, one per behavior.
- [ ] **C5 — search is capped and counted.** `grep` returns files,
  content, or counts, caps at a declared maximum with "narrow the
  query", honors `.gitignore` in a repo, and reports the match count.
  `glob` sorts by mtime and caps at 100 with a truncation flag. Neither
  returns an iterative next/prev stream.
- [ ] **C6 — a command can outlive a turn.** `bash` takes `timeout_ms`
  (clamped) and `run_in_background`; a background run returns a handle,
  and its output is retrievable. A timed-out foreground command is
  promoted to background rather than killed silently. Output over the
  budget spills to an artifact path that the result names.
- [ ] **C7 — a ledger survives a fold.** `todo` replaces the full list,
  accepts status aliases, and its items are re-injected after a
  compaction. A fixture with 30 calls and a forced fold retains every
  item and every artifact path.
- [ ] **C8 — one module per tool.** The registry lists every native
  tool with a spec; a test iterates the registry and validates every
  schema. A wrong-typed argument returns
  `error[validation]: field \`x\` expects …; valid: …`; an unknown
  argument warns and runs. `Stop` during `bash` returns within one
  second. A turn with more than 40 calls is refused by name.
- [ ] **C9 — forty tools cost twenty-eight declarations.** With a
  fixture catalog of 40 tools, the first request declares at most 28
  and the remainder are reachable through `tool_search`, which
  activates at most five for the next request. Two builds of the same
  catalog produce byte-identical declaration order. A disabled server's
  tools are absent; a discovered-but-unconfigured entry never appears.
  An alias resolves (`str_replace` → `edit`) and a name collision
  disables the alias rather than shadowing a real tool.
- [ ] **C10 — retrieval is geometry.** Over a fixture corpus of 24
  queries and a 40-tool catalog, the expected tool is in the top three
  for at least 20. The resolution records `sem1` and the scores. With
  sem1 unreachable, the lexical ranker runs and the record says so.
  Below the floor, the result is "no tool fits" plus the nearest two,
  and nothing is activated. The resolution completes with the model
  client unreachable, proving no generation in the path.
- [ ] **C11 — the profile comes from the route verdict.** The workload
  is read from `Client::route_decision` (`src/api.rs:588`) at run start,
  not re-asked. A `standard_code` verdict orders code tools first; a
  `prose` verdict orders web and read first; an absent or
  low-confidence verdict selects the default and records that. No
  profile removes a core tool. A test drives each verdict and asserts
  the order and the record.
- [ ] **C12 — dispatch is a policy.** A capability with two providers
  chooses the warm one over a cold healthy one, the healthy over the
  dead, and records the reason. A read-only capability fails over once
  and records both attempts. A write capability whose provider fails
  refuses by name without a second attempt.
- [ ] **C13 — arguments are grounded.** A call with
  `model: "step-3-flash"` either matches a catalog value or errors with
  the candidates; a path that does not exist lists the three nearest
  actual entries; a defaulted argument names its source. A test asserts
  no model request is made while resolving an argument.
- [ ] **C14 — old results become artifacts.** After a fold, an old
  result is a stub naming tool, bytes, and path; every path in the
  stubs exists. A 30-call fixture compacts and recovers a spilled
  result with `read_file`.
- [ ] **C15 — MCP results are typed.** `readOnlyHint` and
  `destructiveHint` select an approval tier but never grant; pagination
  follows `nextCursor`; `structuredContent` is validated when an
  `outputSchema` exists and text is the fallback; an image result
  becomes an artifact; a timeout sends `notifications/cancelled`; a
  pre-join failure is recorded in the session, not only the progress
  log.
- [ ] **C16 — the gate is green.** `cargo fmt --all --check`,
  `cargo clippy --all-targets -- -D warnings`, `cargo build --release`,
  `cargo test`, each exit code read on the cargo invocation itself. The
  Settings tool card and the transcript resolution line are verified
  live on the loopback face, and the audit numbers come from a real
  session.

## Progress (2026-10-23)

Landed:

- **Increment 1, the correctness half.** `vision` refuses an absolute
  path and canonicalizes before reading; media types come from bytes;
  plan mode refuses every file-writing tool; `supersede` dispatches on
  an exact name; `write_file` is atomic; the TUI seeds user MCP servers;
  `computer_use` no longer claims an image arrives.
- **Increment 2.** The eight built-ins, with windowed line-numbered
  reads, `edit`, `grep`, `glob`, `todo`, `bash` timeout and background,
  and `list_dir` depth.
- **Increment 3's visible half.** Schema validation before execution,
  foreign-name aliases, nearest-name recovery, and `error[validation]`
  messages that name the field.
- **Increment 4.** The catalog tier at 28 tools / 8,000 tokens, with a
  byte-stable core and activated tools appended. Pure and unit-tested.
- **Increment 5.** `tool_search` runs sys1's new `tool_route` task for
  the capability class, sem1 for cosine ranking with cached card
  vectors, and a floor/margin rule; lexical fallback when sem1 is
  unavailable. No language model is in the resolution path.

Not built, and named rather than implied:

- The `ToolResolution` session record, `fleet tools audit`, and the
  resolution rates.
- Tool-result elision with spill artifacts and the todo ledger's
  re-injection after a fold.
- The typed `ToolResult`/artifacts/cancellation/per-turn cap.
- The MCP protocol stage: annotations, pagination, structured content,
  image artifacts, cancellation.
- The route-verdict tool profile (the class decision covers selection;
  the workload still does not reorder the core set).

The gate is green: `cargo fmt --all --check`, `cargo clippy
--all-targets -- -D warnings`, `cargo build --release`, `cargo test`
(1,560 tests). The sys1 lane gained a task in a vendored file
(`services/sys1/src/sys1/tasks.py`); the upstream core needs the same
task to survive a re-vendor.

## Files to be touched

- `src/tools.rs` — the eight built-ins, the tool trait and registry,
  specs, descriptions
- `src/tool_catalog.rs` (new) — the snapshot, sources, tiers, ordering,
  aliases, cards
- `src/tool_router.rs` (new) — resolution, sem1 retrieval, profiles,
  dispatch policy, abstention
- `src/tool_result.rs` (new) — result, error classes, artifacts,
  truncation
- `src/agent.rs` — declarations from the catalog, dispatch through the
  router, `ToolResolution` writes, the cancellation token, the per-turn
  cap, the profile from the route decision
- `src/api.rs` — tool specs unchanged in shape; structured result
  plumbing where a provider needs text only
- `src/session.rs` — the `ToolResolution` record and replay
- `src/compaction.rs` — elision, artifact stubs, ledger re-injection
- `src/mcp.rs` — annotations, pagination, structured content, images,
  cancellation
- `src/settings.rs` — read the existing `catalog`; no writer changes
- `src/subagents.rs` — scope resolved through the same catalog, aliases
  honored, `ToolResult` carried back
- `src/cli.rs`, `src/main.rs` — the `fleet tools` verb (bare card and
  `audit`), after the MCP manager's health verb lands
- `src/gui/assets/app.js`, `app.css` — the transcript resolution line,
  the Settings tool card, the audit card
- `tests/integration.rs`, `src/bin/mock-mcp.rs` — the fixtures: a
  40-tool catalog, a deterministic embedding double, a 4,000-line tree,
  a spill case
- docs: `docs/features/tools.md` (new),
  `FUNCTIONAL-SPECIFICATIONS.md`, `TECHNICAL-DOCUMENTATION.md`,
  `CLAUDE.md`, `docs/architecture/tool-surface.md` (status note)

## Out of scope

- **A capability-class task in sys1.** It is the cleaner coarse
  decision, but the lane's task vocabulary is static and adding one is
  an upstream change plus re-vendor. Named follow-up; this plan uses
  the verdict sys1 already produces.
- **HTTP/SSE MCP transports.** The manager's plan excludes them and
  this plan does not add them. A server the client cannot speak is
  reported by shape, never half-called.
- **A code-execution meta-tool.** The token savings are real and large,
  but fleet's `bash` is unsandboxed, so a second executor adds risk
  without a boundary. It waits for a sandbox.
- **Embeddings for repository search.** `grep` and `glob` come first.
  Cursor's measured gains are largest on 1,000+ file repos and modest
  otherwise.
- **`apply_patch` and per-model presets.** A follow-up once `edit` has
  an audit baseline; Cline and Factory both show the split is worth
  having, but only after the content-anchored default is measured.
- **Auto-import or auto-enable of discovered servers.** Discovery stays
  a management action; nothing discovered becomes callable.
- **A second memory store over MCP.** Fleet's memory is the source of
  truth.

## Verification

- C0/C3–C8/C14/C15: unit tests in the owning module plus the
  integration suite; the fixtures are code under `tests/` and
  `src/bin/mock-mcp.rs`.
- C1/C2: a fixture session with known calls and a deliberately broken
  call per error class; the audit's numbers are asserted exactly.
- C9–C13: the 40-tool fixture catalog, the 24-query corpus, a
  deterministic embedding double for CI, and a live sem1 run on this
  machine for the retrieval score. The corpus lives in
  `src/tool_router.rs`'s tests so a threshold change is a test change.
- C16: the four gate commands in order, exit codes read directly; the
  face verified at `#/settings` and in a live run's transcript with a
  screenshot.
- A replay A/B after increment 5: a recorded trajectory's calls are
  replayed through the new surface in a scratch worktree, and the
  results are diffed against the old implementation. This is the honest
  test of "did it get better", and it uses the log fleet already writes.

## Linked artifacts

- `docs/architecture/tool-surface.md` — the analysis, inventory and
  evidence this plan implements.
- `docs/plans/2026-10-22-mcp-server-registry.md` — the manager slice
  this plan consumes; its D2 risk ("every enabled server's tools ride
  every run") is answered by D4 here.
- `src/settings.rs`'s `catalog` (`1903`) — the existing one-read catalog
  for argument values; this plan does not build a second one.
- `src/mcp_registry.rs`'s `enabled_specs` (`156`) — the only MCP
  surface the tool layer reads.

## Risks

- **sem1 is a dependency on the resolution path.** Mitigation: card
  vectors are cached; the lexical ranker runs when sem1 is down and
  says so; the catalog below the threshold needs no retrieval at all.
  No run fails because a lane is cold.
- **Retrieval quality is unproven on fleet's catalog.** Mitigation: the
  24-query fixture gates the thresholds, and the resolution record
  makes a bad rank visible. Below the threshold the feature is inert,
  so a bad ranker cannot make the small case worse.
- **Deferred tools add a round trip.** Mitigation: only above 28 tools
  or 8k tokens; the core set is a complete coding surface on its own;
  the search result activates tools for the next request, so one call
  buys the whole turn.
- **A wrong profile.** Mitigation: profiles reorder and boost, never
  remove; the default profile is a valid surface; the record names the
  verdict that selected it.
- **The MCP manager's delivered C2 says "declared from the first
  request".** Below the threshold that holds; above it, reachability
  replaces declaration. The manager's plan ships as written and this
  plan records the supersession in `docs/features/mcp.md` rather than
  retro-editing a completed contract. The alternative — declaring 60
  server tools — is the measured way to make selection worse.
- **Merge conflicts with the manager.** Mitigation: the ownership table
  in D12; this plan does not edit `mcp_registry.rs`, `cli.rs`,
  `main.rs`, or `paintSettings`. Where both need `declarations()`, one
  ordering function serves both.
- **Prompt-cache regressions.** Mitigation: fixed ordering and
  append-only activation; a test compares two declarations byte for
  byte.
- **The write-time nomenclature guard (KI-284).** Plan, doc, and
  fixture writes may be refused on mandated spellings; the bash write
  path is the documented workaround, and this plan's own files should
  be written through it if the guard fires.
