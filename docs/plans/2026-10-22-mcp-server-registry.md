---
status: draft
created: 2026-10-22
updated: 2026-10-23
slug: mcp-server-registry
---

# Plan: the MCP server registry — configured servers, and the machine's unconfigured ones

## Context

The owner's request: a surface to manage the MCP servers set up on the
harness, and also the ones available on the local machine but not
configured for the harness yet. Today Fleet's MCP story is the six
vendored lanes — `sys1`, `sdm1`, `sem1`, `gen1`, `blend1`, `browser` —
held in the closed static registry `lanes::LANES` (`src/lanes.rs:72`), a
generic stdio client (`src/mcp.rs`), and a generic model-facing namespace
— the model calls `mcp__<svc>__<tool>`, built by `tools::namespaced` and
`lane_tool_name` (`src/tools.rs:1095`). The mechanism is general; only
the registry is closed. `~/.fleet` carries no `lanes.json` — every lane
runs on defaults — and there is no way to add any other MCP server at
all.

The machine already carries unconfigured servers in other harnesses'
config files. Re-scanned 2026-10-23 (names and shapes only, no values):

| Source | File | Entries |
|---|---|---|
| Claude Desktop | `~/Library/Application Support/Claude/claude_desktop_config.json` | 11 stdio |
| Claude Code | `~/.claude.json`, top-level `mcpServers` | 1 HTTP (tinyfish) |
| Cursor | `~/.cursor/mcp.json` | 1 HTTP (tinyfish) |
| Codex | `~/.codex/config.toml`, `mcp_servers` | 2 stdio, 1 HTTP (tinyfish) |

**Sixteen entries, thirteen stdio and three HTTP.** The stdio ones are
spawnable by the client Fleet already has; the HTTP ones are `tinyfish`
twice over plus Codex's `tinyfish`, and stay unsupported by shape this
slice. Three entries carry an `env` block — `openweather` names
`OWM_API_KEY` and holds its **value in plain text**, `node-code-sandbox-mcp`
names `FILES_DIR`, and Codex's `node_repl` names fifteen machine paths and
timeouts — which is why D3 below is names-only rather than a copy.

Nothing in the repo reads any of these files today (re-verified
2026-10-23: no source or service names any of the four paths), and Fleet
must never write them — import is one-way, into Fleet's own home.

**State — why this is still a draft.** The plan was drafted 2026-10-22
(`2ef9de3`) in a fleet session that did not go as planned: the drafting
runs hit the 25-turn cap twice, writing the file was refused eight times
by the nomenclature write guard (KI-284), and the owner then pivoted to
the chat-UX work — so `/goal implement as designed` implemented
`2026-10-22-chat-ux-polish.md`, not this. Nothing here is built: no
`src/mcp_registry.rs`, no `set_server`/`remove_server`, no `mcp` verb, no
`~/.fleet/mcp.json`, no `docs/features/mcp.md`. Reviewed against the tree
2026-10-23 and corrected below (the lane count, the entry arithmetic, the
settings writer claim, the actual seams).

## What we should do

**The build order, and where you can review it.** Five increments, each
gated and committed before the next. The CLI is the review surface that
does not wait for the face: `fleet mcp list` is usable after 1,
`scan`/`import` after 2, a real run can call an imported server after 3,
`health` after 4, and the card is live after 5. The slice is not
delivered until the card is live (hard rule 11 — the visual surface is a
per-slice deliverable), but you can exercise the registry from the CLI as
soon as each lands.

1. The registry file and the CLI that reads and writes it. Nothing
   spawns yet, so this is reviewable without risk to a run.
2. Discovery and import. Reviewable with `fleet mcp scan` on this
   machine: sixteen rows, thirteen importable, three refused by shape,
   no value anywhere in the output or the file.
3. Run wiring — the only increment that changes how a run behaves. Its
   integration test is over the scripted server, and the live check is
   one imported server called once from a chat.
4. Health. `fleet mcp health` against the same server.
5. The settings card at `#/settings`, live on the loopback face.

**Four decisions to confirm before increment 1** (I recommend the first
option in each):

- **Registry file.** `mcp.json` as a sibling of `lanes.json` (recommended:
  two concepts, two shapes, one writer pattern) — rather than extending
  `lanes.json`, whose `set_lane` refuses credential-shaped names outright,
  which is the opposite of what a server registry must store.
- **Transports.** stdio only this slice; HTTP/SSE entries are reported
  unsupported by shape and refused on import (recommended) — rather than
  adding a transport now.
- **When enabled servers join.** Pre-join at run start (recommended:
  a lazy join would never declare the tools, so the model could not call
  them). The cost to accept is explicit: every enabled server's tool list
  rides every run's declarations. If that cost feels wrong once two or
  three servers are enabled, the alternative is a per-chat enable, which
  is a different surface (a run-bar field) and not this slice.
- **The scan's sys1 judgment.** Keep it, advisory and badge-only
  (recommended: it orders sixteen rows and falls back to *unclassified*
  without keys) — or drop it and let the list keep its file order.

**Two open defects that shape the work.** KI-284: the write guard refuses
content-constrained spellings, so plan, fixture, and doc writes go
through the bash path until it closes (this file's own history is the
evidence). KI-289: a deliverable has no address on the face, so the
increment-5 report links the route (`#/settings`), not the file, until
that row closes.

## Approach

**D1 — configured servers live in a sibling `mcp.json`, not in
`lanes.json`.** The lanes file is the fleet-owned lane configuration with
its own shape (home, env, per-tool defaults) guarded by `set_lane`'s
registry-name check (`src/settings.rs:2399`) and read through
`lanes::read_lanes` / `lanes_path`. Stuffing generic servers into it would
entangle two concepts the code keeps apart. The registry file sits beside
it in the fleet home: same read-merge-write pattern, its own shape.

**D1a — the registry's env entries are names, and the schema has no
value field.** This is not `set_lane`'s guard reused: `set_lane` *refuses*
a credential-shaped name ("keys belong in the vault or `.env`, never in
`lanes.json`", `src/settings.rs:2446`), which is right for lane
configuration and exactly wrong for a server registry, whose whole job is
to record `OWM_API_KEY`. So the registry inverts it: a name is what is
stored, a value cannot be, and there is no field to put one in. At spawn
each name resolves through the chain the lanes already use (process env →
the home's dotenv/vault), and an unresolved name is a named probe
failure.

**D2 — enabled user servers pre-join at run start.** The lanes join
lazily at first call (`LaneRoster::join`, `src/agent.rs:4836`) because the
model learns them from memory and orientation. A user server has no such
channel: a lazy join would never declare its tools, so the model could
not call what it was never told about. The seam already exists —
`AgentOptions.lane_specs: Option<HashMap<String, ServerSpec>>`
(`src/agent.rs:283`) feeds `LaneRoster::new` — so enabled servers are
seeded into that map at run start, while the six lanes keep their lazy
join and no existing behavior moves.

**D3 — an import carries env key names, never values.** The foreign
configs hold literal credentials (D3's own case is `OWM_API_KEY` in plain
text in Claude Desktop's config, and Codex's `node_repl` carries fifteen
machine paths that are configuration, not secrets). The registry records
names; values never cross, in the file, the scan output, or the log.

**D4 — both surfaces ship in this slice.** Hard rule 11 makes the visual
surface a per-slice deliverable. The settings board's servers card is the
surface; `fleet mcp` verbs (list, scan, import, remove, enable/disable,
health) are the CLI precedent `fleet lanes` and `fleet providers` already
set. Cards on both, never plain-text tables. The seams are the existing
ones: the settings document built by `settings_read`
(`src/gui/bridge.rs:4141`) gains a `servers` key beside `lanes` (built by
`lanes_document`, `:4370`), the writes ride `settings_apply` (`:4431`)
whose ops are already named per object (`set`, `provider`, `lane`, …), and
the card is one more `boardCard` in `paintSettings`
(`src/gui/assets/app.js:9502`).

**D5 — loopback-only, this slice.** The remote face cannot read or write
settings at all: `/api/settings` is absent from both `ALLOWED_GET` and
`ALLOWED_POST` (`src/remote/listener.rs`), and management follows that
ruling. Nothing new enters either allowlist.

**D6 — scan-time judgment, advisorial only.** One sys1 Choice
classification per scanned entry — containerized / pinned-package /
bare-interpreter / remote-endpoint — badges the review list and orders it
(containerized and pinned first, bare interpreters flagged). Without sys1
keys the label is unclassified and every other part works. The owner
decides imports; the judgment only orders and badges. No escalation lane:
nothing here acts on its own judgment.

**D7 — the registry is read on every run, so its read must be cheap and
flat.** `mcp.json` is a small object read whole at run start, like
`lanes.json`; no database, no cache, no watcher. A malformed file is a
named refusal on the card and a named refusal on the CLI, and a run
proceeds with no user servers rather than failing to start.

**D8 — the six lanes are reserved, and so are the built-in tools.** A
server may not take `sys1`, `sdm1`, `sem1`, `gen1`, `blend1`, `browser`,
or a built-in tool name, because the model-facing namespace is
`mcp__<name>__<tool>` and a lane-named server would be indistinguishable
from the lane. Inside one server, short-name collisions resolve through
`unique_name` (`src/tools.rs:1131`), so no two declarations ever share a
name.

## The app

**The settings board's servers card.** One card on the settings board,
beneath the lanes card (`boardCard(12, 'the lanes', …)`), titled the MCP
servers. Its top half is the registry: one row per configured server —
status dot (green probed-ok, gray never probed, red last probe failed),
the server's name, a shape badge (stdio / unsupported), its tool count,
an enabled toggle, and — for imported servers — a source badge naming the
harness it came from (imported from Claude Desktop, 2026-10-23). A probe
action per row spawns, handshakes, and lists tools, then renders the
server's self-reported name, version, and tool count with a timestamped
probe line. The card's bottom half is discovery: one import row per entry
the last scan found and the registry does not yet carry — the entry's
name, its source file's harness, a shape badge, the judgment badge from
D6 (containerized / pinned package / bare interpreter / remote endpoint /
unclassified), and an import button; rows for entries already configured
collapse into a one-line "already configured" note, and unsupported-shape
rows carry a disabled import with the reason. The empty state is a card,
not a bare table: "no MCP servers configured yet — run a scan" with the
scan action beside it.

**The CLI.** `fleet mcp` renders the same two halves as cards: the
registry's rows (name, shape, enabled, tool count, last probe) and the
scan's import rows with their badges. `fleet mcp health` probes every
configured server and renders one probe card each. Verb output follows
the `fleet lanes` / `fleet providers` card style — never a plain-text
table. `fleet mcp` is a free verb today (no `mcp` arm in `src/cli.rs`).

**The run's view.** A joined server appears in the face's lane rail
beside the lanes: same `LaneJoined` event, same transcript line, the
server's tools declared from the first request. Nothing about the face's
existing lanes changes.

## Increments

Each increment closes green — the four-command gate in full, in order:
`cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings`,
`cargo build --release`, `cargo test` — each exit code read on the cargo
invocation itself and never through a status-losing pipe — and commits
with explicit paths before the next begins. Content-constrained files are
written through the bash path until KI-284 closes.

1. **The registry and its writer.** `src/mcp_registry.rs` (document,
   reserved names per D8, validation, the no-value schema of D1a),
   `set_server` / `remove_server` / `set_server_enabled` in
   `src/settings.rs` (the read-merge-write pattern of `set_lane`, *not*
   its credential-name refusal), the `fleet mcp list` / `enable` /
   `disable` / `remove` verbs in `src/cli.rs` and `src/main.rs`, unit
   tests. Gate; commit.
2. **Discovery and import.** The scan over the four foreign sources
   (defensive JSON/TOML parsing, named skips, shape reporting), the
   import verb and the key-names-only rule, the sys1 Choice badges with
   the unclassified fallback, the marker-value test. Gate; commit.
3. **Run wiring.** `AgentOptions.lane_specs` gains the registry's enabled
   specs at run start; the roster pre-joins them before the first
   request; tools ride `declarations()` under the `mcp__<server>__<tool>`
   form; calls flow the roster's bounded seam; `LaneJoined` and the
   `lane_tools` log record carry the mapping; dead-child semantics on a
   mid-run death. Integration tests over the existing scripted server
   (`src/bin/mock-mcp.rs`), including the name-collision case. Gate;
   commit.
4. **Health.** The probe per configured server — spawn, initialize,
   `tools/list`, report, teardown — `fleet mcp health`, the stderr-tail
   reason on failure. Gate; commit.
5. **The settings board's card.** The servers card and import rows in
   `src/gui/assets/app.js` and the stylesheet, `servers_document` beside
   `lanes_document` and the `server`/`server-scan`/`server-import` ops in
   `settings_apply`, live verification on the loopback face at
   `#/settings` with a screenshot. Gate; commit.

## Acceptance criteria

- [ ] C1 — the `mcp.json` file in the fleet home is the registry:
  `fleet mcp` lists configured servers as cards (name, shape, enabled
  state, tool count) and a machine with none shows an empty-state card,
  not a bare table.
- [ ] C2 — a configured stdio server's tools are declared to the run in
  the `mcp__<server>__<tool>` form from the first request; a call to one
  answers bounded and lands in the session log's `lane_tools` record with
  both ends of the name mapping. Verified by an integration test over the
  scripted mock server.
- [ ] C3 — `fleet mcp scan` reads every known foreign config file
  present, reports each entry as configured / available / unsupported with
  its source file named; absent files are skipped silently, an
  unparseable file is a named skip, an HTTP/SSE entry reports unsupported
  by shape rather than disappearing, and importing an unsupported shape is
  a named refusal.
- [ ] C4 — importing writes the registry and executes nothing; neither
  the scan output, the face's servers document, nor the registry file ever
  contains an env value — key names only (grep-verified against a fixture
  config carrying a marker value), and the registry's shape carries no
  field a value could be written into.
- [ ] C5 — the registry refuses the six lane names and the built-in tool
  names (D8), and a user server whose tool short-names collide with an
  already-declared name resolves to a distinct declared name (the
  `unique_name` rule), so no two declarations ever share a name.
- [ ] C6 — `fleet mcp health` spawns, handshakes, lists tools, reports
  the server's self-reported name/version, and tears down; a failing spawn
  names the reason from the child's stderr tail, the lane probe's way.
- [ ] C7 — enable/disable lands through the one settings writer, takes
  effect on the next run, and a disabled server appears in `fleet mcp` but
  is absent from that run's tool declarations.
- [ ] C8 — the GUI settings board carries the surface described in The
  app: configured servers as rows with status dots and an enable toggle,
  discovered entries as import rows with shape and judgment badges, an
  empty state that offers the scan, at the address `#/settings`; the
  remote allowlists are byte-for-byte unchanged (`/api/settings` stays
  absent from both `ALLOWED_GET` and `ALLOWED_POST`).
- [ ] C9 — the gate is green: `cargo fmt --all --check`, `cargo clippy
  --all-targets -- -D warnings`, `cargo build --release`, `cargo test`,
  each exit code read on the cargo invocation itself.
- [ ] C10 — the three outputs agree: the CLI's scan, the face's servers
  document, and the session log carry the same names and shapes, and never
  a value. One fixture read three ways, one answer.

## Files to be touched

- `src/mcp_registry.rs` (new) — the registry document, reserved-name
  check, scan normalization over the four sources, health probe
- `src/settings.rs` — `set_server` / `remove_server` /
  `set_server_enabled` (the read-merge-write pattern; no credential-name
  refusal — D1a)
- `src/cli.rs`, `src/main.rs` — the `fleet mcp` verbs
- `src/agent.rs` — `AgentOptions.lane_specs` seeded from the registry, the
  roster's pre-join at run start
- `src/gui/bridge.rs` — `servers_document` in the settings read, the
  servers ops in `settings_apply`
- `src/gui/assets/app.js`, `src/gui/assets/app.css` — the servers card
  and import rows in `paintSettings`
- `tests/integration.rs` — mock-server-driven declare/join/call/disable
  tests
- `src/bin/mock-mcp.rs` — **already exists** (slice-4, `28fca49`): a
  scripted stdio server with `reply`/`stray`/`hang`/`exit`/`env`
  directives. Reuse it; the collision case needs only a script whose
  `initialize` result carries a different `serverInfo.name`, not a second
  binary

## Out of scope

- HTTP/SSE and remote MCP servers (the `tinyfish`-style URLs): the client
  is stdio-only; the scan reports them unsupported by shape and the import
  refuses them. A transport extension is its own plan.
- Writing to any foreign harness's config file: import is one-way,
  read-only on their side, always.
- Background or scheduled scanning, and auto-import: discovery runs when
  the owner asks (verb or card action), never on a timer.
- Per-tool approval classes for server tools: calls already cross the
  existing approval gate and `allow_tool` rules; no new class system.
- Migrating the six lanes into the registry file: they stay the vendored
  registry, untouched.
- A per-chat server enable (the run-bar field): if the pre-join cost ever
  needs narrowing, that is a follow-up surface, not this slice.

## Verification

- C1/C6/C7: the `fleet mcp`, `fleet mcp health`, and enable verbs run
  against a scratch fleet home with the mock server configured; the cards
  are read.
- C2: `cargo test --test integration` — the new tests assert the first
  request's declarations carry the server's tools and the log's
  `lane_tools` record names both ends.
- C3/C4/C10: a fixture tree of foreign configs (one HTTP entry, one
  unparseable file, one absent file, one entry with a marker env value)
  under a scratch home; run the scan three ways (CLI, the servers
  document, the log line) and grep each for the marker value (expect:
  absent) and the key name (expect: present).
- C5: unit tests in `src/mcp_registry.rs` for the reserved names and the
  collision path.
- C8: the GUI card exercised live on the loopback face at `#/settings`
  with a screenshot; `git diff src/remote/listener.rs` empty for the
  allowlists.
- C9: the full gate, exit codes read directly.

## Linked artifacts

- `docs/features/mcp.md` (new) — the registry, the scan, the verbs, the
  card
- `FUNCTIONAL-SPECIFICATIONS.md` — the surface capability added to the
  settings and run-behavior sections
- `TECHNICAL-DOCUMENTATION.md` — the registry file's interface contract
  (Interface contracts), the module reference entry for
  `src/mcp_registry.rs`, the runbook rows for `fleet mcp`
- `CLAUDE.md` — the pointer-index row for `src/mcp_registry.rs`
- `docs/known-issues.md` — KI-284's write-path consequence while this
  builds, KI-289 for the deliverable link

## Risks

- **Every enabled server's tools ride every run.** Pre-join spends context
  on each run for each enabled server. Mitigated by the enabled flag being
  a per-server choice, the tool count shown on the row, and the health
  probe as the pre-flight; the escape hatch (per-chat enable) is named in
  Out of scope, not designed here.
- **Run-start latency and a new failure surface.** Every enabled server
  spawns before the first request; a dead child mid-run needs the lane's
  named-failure semantics rather than a silent empty tool set.
- **Foreign config drift.** Four files, two formats, no stability contract
  from their authors (Codex's `mcp_servers` carries fields the others do
  not: `cwd`, `enabled`, `startup_timeout_sec`, `bearer_token_env_var`).
  The scan parses defensively, names what it could not read, and never
  guesses.
- **Secret leakage by import.** The marker-value test (C4) plus a schema
  with no value field (D1a) are the two layers; `set_lane`'s
  credential-name refusal is deliberately *not* reused, because it would
  refuse the very names this registry exists to store.
- **The write-time nomenclature guard** (KI-284): this plan itself was
  refused eight times writing mandated spellings (`mcp__`, `LANES`,
  `RunOptions`, `LaneJoined`, `LaneChild`, `FUNCTIONAL`, `CLAUDE`); the
  implementation will hit it again on fixtures and docs. The bash write
  path on content-constrained files is the documented workaround.
- **The settings board is a 12-column board**, not a list: the card must
  use `boardCard` / `surfaceInstrument` like every other card, or it will
  read as a foreign object on the page.
