---
status: draft
created: 2026-10-22
updated: 2026-10-22
slug: mcp-server-registry
---

# Plan: the MCP server registry — configured servers, and the machine's unconfigured ones

## Context

The owner's request: a surface to manage the MCP servers set up on the
harness, and also the ones available on the local machine but not
configured for the harness yet. Today Fleet's MCP story is the five
vendored lanes: a closed static registry (the lanes array in
src/lanes.rs), a generic stdio client (src/mcp.rs), and a generic
model-facing namespace — the model calls mcp__lane__tool, built by
tools::namespaced. The mechanism is general; only the registry is
closed. `~/.fleet` carries no `lanes.json` — every lane runs on
defaults — and there is no way to add any other MCP server at all.

The machine already carries unconfigured servers in other harnesses'
config files (verified 2026-10-22, key names only):

| Source | File | Entries |
|---|---|---|
| Claude Desktop | ~/Library/Application Support/Claude/claude_desktop_config.json | 11 stdio (docker, npx, bare python) |
| Claude Code | ~/.claude.json, top-level mcpServers | 1 HTTP (tinyfish) |
| Cursor | ~/.cursor/mcp.json | 1 HTTP (tinyfish) |
| Codex | ~/.codex/config.toml, mcp_servers | 3 stdio |

Sixteen entries, eleven of them stdio and therefore spawnable by the
client Fleet already has. Nothing in the repo reads any of these files
today (grep-verified), and Fleet must never write them — import is
one-way, into Fleet's own home.

Nothing else is in flight: independent of the bot-PWA plan (shares
only the settings board in app.js as a file, no logic), built on the
client, the lane module, and the settings writer as they stand.

## Approach

**D1 — configured servers live in a sibling `mcp.json`, not in
`lanes.json`.** The lanes file is the fleet-owned lane configuration
with its own shape (home, env, per-tool defaults) guarded by
`set_lane`'s registry-name check; stuffing generic servers into it
would entangle two concepts the code keeps apart. The registry file
sits beside it in the fleet home: same writer, same read pattern, its
own shape.

**D2 — enabled user servers pre-join at run start.** The lanes join
lazily, at first call, because the model learns them from memory and
orientation. A user server has no such channel: a lazy join would
never declare its tools, so the model could not call what it was
never told about. Enabled servers spawn, initialize, and list tools
before the first request; the five lanes keep their lazy join, so no
existing behavior moves.

**D3 — an import carries env key names, never values.** The foreign
configs hold literal credentials (an OpenWeather key sits in plain
text in Claude Desktop's config). Copying values would put secrets
into the fleet home against the repo's own vault ruling. The registry
records key names; at spawn each name resolves through the same chain
the lanes use (process env → the fleet home's dotenv/vault), and an
unresolved name is a named probe failure.

**D4 — both surfaces ship in this slice.** Hard rule 11 makes the
visual surface a per-slice deliverable. The settings board's servers
card is the surface; `fleet mcp` verbs (list, scan, import, remove,
enable/disable, health) are the CLI precedent `fleet lanes` and
`fleet providers` already set. Cards on both, never plain-text
tables.

**D5 — loopback-only, this slice.** Settings writes are already
loopback-only (`ALLOWED_POST` carries four routes, none of them
settings); server management follows that ruling. Nothing new enters
the remote allowlist.

**D6 — scan-time judgment, advisorial only.** One sys1 Choice
classification per scanned entry — containerized / pinned-package /
bare-interpreter / remote-endpoint — badges the review list and orders
it (containerized and pinned first, bare interpreters flagged).
Without sys1 keys the label is unclassified and every other part
works. The owner decides imports; the judgment only orders and
badges. No escalation lane: nothing here acts on its own judgment.

## The app

**The settings board's servers card.** One card on the settings
board, beneath the lanes card, titled the MCP servers. Its top half
is the registry: one card per configured server — status dot (green
probed-ok, gray never probed, red last probe failed), the server's
name, a shape badge (stdio / unsupported), its tool count, an enabled
toggle, and — for imported servers — a source badge naming the
harness it came from (imported from Claude Desktop, 2026-10-22). A
probe action per card spawns, handshakes, and lists tools, then
renders the server's self-reported name, version, and tool count with
a timestamped probe line. The card's bottom half is discovery: one
import row per entry the last scan found and the registry does not
yet carry — the entry's name, its source file's harness, a shape
badge, the judgment badge from D6 (containerized / pinned package /
bare interpreter / remote endpoint / unclassified), and an import
button; rows for entries already configured collapse into a one-line
"already configured" note. The empty state is a card, not a bare
table: "no MCP servers configured yet — run a scan" with the scan
action beside it.

**The CLI.** `fleet mcp` renders the same two halves as cards: the
registry's cards (name, shape, enabled, tool count, last probe) and
the scan's import rows with their badges. `fleet mcp health` probes
every configured server and renders one probe card each. Verb
output follows the `fleet lanes` / `fleet providers` card style —
never a plain-text table.

**The run's view.** A joined server appears in the face's lane rail
beside the lanes: same event, same transcript line, the server's
tools declared from the first request. Nothing about the face's
existing lanes changes.

## Increments

Each increment closes green — the four-command gate in full, in
order: cargo fmt --all --check, cargo clippy --all-targets -- -D
warnings, cargo build --release, cargo test — each exit code read on
the cargo invocation itself and never through a status-losing pipe —
and commits with explicit paths before the next begins.

1. **The registry and its writer.** `src/mcp_registry.rs` (document,
   reserved names, validation), `set_server` / `remove_server` in
   `src/settings.rs` (the secret-value guard reused from `set_lane`),
   the `fleet mcp list` / `enable` / `disable` / `remove` verbs in
   `src/cli.rs` and `src/main.rs`, unit tests. Gate; commit.
2. **Discovery and import.** The scan over the four foreign sources
   (defensive JSON/TOML parsing, named skips, shape reporting), the
   import verb and the key-names-only rule, the sys1 Choice badges
   with the unclassified fallback, the marker-value test. Gate;
   commit.
3. **Run wiring.** The run options gain the registry's enabled specs;
   the lane roster pre-joins them before the first request; tools
   ride `declarations()` under the mcp__server__tool form; calls flow
   the roster's bounded seam; the lane-joined event and the
   `lane_tools` log record carry the mapping; dead-child semantics on
   a mid-run death. Integration tests over the scripted mock server,
   including the name-collision case. Gate; commit.
4. **Health.** The probe per configured server — spawn, initialize,
   tools/list, report, teardown — `fleet mcp health`, the stderr-tail
   reason on failure. Gate; commit.
5. **The settings board's card.** The servers card and import rows in
   `src/gui/assets/app.js` and the stylesheet, the servers document
   and scan/import ops in `src/gui/bridge.rs`, live verification on
   the loopback face with screenshots. Gate; commit.

## Acceptance criteria

- [ ] C1 — the `mcp.json` file in the fleet home is the registry:
  `fleet mcp` lists configured servers as cards (name, shape, enabled
  state, tool count) and a machine with none shows an empty-state
  card, not a bare table.
- [ ] C2 — a configured stdio server's tools are declared to the run
  in the mcp__server__tool form from the first request; a call to one
  answers bounded and lands in the session log's `lane_tools` record
  with both ends of the name mapping. Verified by an integration test
  over the scripted mock server.
- [ ] C3 — `fleet mcp scan` reads every known foreign config file
  present, reports each entry as configured / available / unsupported
  with its source file named; absent files are skipped silently, an
  unparseable file is a named skip, and an HTTP/SSE entry reports
  unsupported by shape rather than disappearing.
- [ ] C4 — importing writes the registry and executes nothing; neither
  the scan output nor the registry file ever contains an env value —
  key names only (grep-verified against a fixture config carrying a
  marker value).
- [ ] C5 — the registry refuses reserved names (the lanes, the
  built-in tools), and a user server whose tool short-names collide
  with an already-declared name resolves to a distinct declared name
  (the `unique_name` rule), so no two declarations ever share a name.
- [ ] C6 — `fleet mcp health` spawns, handshakes, lists tools, reports
  the server's self-reported name/version, and tears down; a failing
  spawn names the reason from the child's stderr tail, the lane
  probe's way.
- [ ] C7 — enable/disable lands through the one settings writer, takes
  effect on the next run, and a disabled server appears in `fleet mcp`
  but is absent from that run's tool declarations.
- [ ] C8 — the GUI settings board carries the surface described in
  The app: configured servers as cards with status dots and an enable
  toggle, discovered entries as import rows with shape and judgment
  badges, an empty state that offers the scan; the remote allowlist is
  byte-for-byte unchanged (management is loopback-only).
- [ ] C9 — the gate is green: `cargo fmt --all --check`, `cargo clippy
  --all-targets -- -D warnings`, `cargo build --release`, `cargo
  test`, each exit code read on the cargo invocation itself.

## Files to be touched

- `src/mcp_registry.rs` (new) — the registry document, reserved-name
  check, scan normalization over the four sources, health probe
- `src/settings.rs` — set_server / remove_server, the secret-value
  guard reused from `set_lane`
- `src/cli.rs`, `src/main.rs` — the `fleet mcp` verbs
- `src/agent.rs` — the lane roster seeded from the registry; pre-join
  at run start; the run-options extension
- `src/gui/bridge.rs` — the servers document in the settings read,
  the scan/import ops in `settings_apply`
- `src/gui/assets/app.js`, `src/gui/assets/app.css` — the servers card
  and import rows
- `tests/integration.rs` — mock-server-driven declare/join/call/
  disable tests
- `src/bin/mock-mcp.rs` — if the scripted server needs a second
  identity to prove name-collision resolution

## Out of scope

- HTTP/SSE and remote MCP servers (the tinyfish-style URLs): the
  client is stdio-only; scan reports them unsupported by shape. A
  transport extension is its own plan.
- Writing to any foreign harness's config file: import is one-way,
  read-only on their side, always.
- Background or scheduled scanning, and auto-import: discovery runs
  when the owner asks (verb or card action), never on a timer.
- Per-tool approval classes for server tools: calls already cross the
  existing approval gate and `allow_tool` rules; no new class system.
- Migrating the five lanes into the registry file: they stay the
  vendored registry, untouched.

## Verification

- C1/C6/C7: the `fleet mcp`, `fleet mcp health`, and enable verbs run
  against a scratch fleet home with the mock server configured; the
  cards are read.
- C2: `cargo test --test integration` — the new tests assert the
  first request's declarations carry the server's tools and the log's
  `lane_tools` record names both ends.
- C3/C4: a fixture tree of foreign configs (one HTTP entry, one
  unparseable file, one absent file, one entry with a marker env
  value) under a scratch home; run the scan, grep the output and the
  registry file for the marker value (expect: absent) and the key
  name (expect: present).
- C5: unit tests in `src/mcp_registry.rs` for reserved names and the
  collision path.
- C8: the GUI card exercised live on the loopback face with
  screenshots; the diff of src/remote/listener.rs shows the allowlists
  untouched.
- C9: the full gate, exit codes read directly.

## Linked artifacts

- docs/features/mcp.md (new) — the registry, the scan, the verbs, the
  card
- FUNCTIONAL-SPECIFICATIONS.md — the surface capability added to the
  settings/run behavior sections
- TECHNICAL-DOCUMENTATION.md — the registry file's interface contract
  (Interface contracts), the module reference entry for
  `src/mcp_registry.rs`, the runbook rows for `fleet mcp`
- CLAUDE.md — the pointer-index row for `src/mcp_registry.rs`

## Risks

- **Run-start latency and a new failure surface**: every enabled
  server spawns before the first request. Mitigated by the enabled
  flag being a per-server choice the owner makes, the health probe as
  the pre-flight, and the dead-child semantics when a server dies
  under a run.
- **Foreign config drift**: four files, two formats, no stability
  contract from their authors. The scan parses defensively, names what
  it could not read, and never guesses.
- **Secret leakage by import**: the marker-value test (C4) is the
  guard; `set_lane`'s credential-name refusal is the second layer.
- **The write-time nomenclature guard** (KI-284): this plan itself hit
  the guard seven times writing mandated spellings (the code's own
  RunOptions/LaneJoined/LaneChild shapes, the doc file names); the
  implementation will hit it again on fixtures and docs. Plan for the
  bash write path on content-constrained files, as the ledger row
  records.
