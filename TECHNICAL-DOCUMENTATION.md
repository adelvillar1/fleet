# Technical documentation

The system of record for running this project: what the boundaries are,
what the environment needs, how to operate it, how work is verified and
released, what is at risk, and what is next. It aggregates the module
doc comments in `src/*.rs`, the architecture notes
([docs/architecture/surfaces.md](docs/architecture/surfaces.md),
[docs/architecture/lanes.md](docs/architecture/lanes.md),
`ecosystem.md`, `deep-dive-2026-10-15.md`), and the feature docs.

If a module or boundary has no entry here, it is either covered in the
sources above or missing; the module reference's Source column points to
the file whose claims were aggregated, and when the code changes, follow
that file.

## System overview and boundaries

One static Rust binary, no Node runtime and no checkout dependency; the
crate is the whole install. Dependencies are `tokio`, `ureq` (HTTP),
`serde`/`serde_json`, and `ratatui`/`crossterm` (the TUI); everything
else is standard library. Five subsystems, each with a boundary:

- **The agent core** (`agent.rs`, `tools.rs`, `approvals.rs`,
  `compaction.rs`, `session.rs`) — the tool loop, the closed built-in
  set, the approval gate, the fold, and the append-only log. Its
  boundary promise: every tool failure is a model-visible `error:`
  result, never a panic, and the log reconstructs exactly what the model
  saw.
- **Routing** (`route.rs`, `route_cache.rs`) — the roster, the workload
  preference table, selection, and failover. Its boundary promise:
  nothing is hardcoded; the router decides unless the owner pins, and
  routing never blocks a run.
- **The lanes** (`lanes.rs`, `mcp.rs`, `lane_client.rs`) — the five
  vendored Python services driven over the hand-rolled MCP stdio client.
  Their boundary promise: a lane is launched, probed, and closed by
  fleet, and a dead lane fails fast by name without killing the run.
  What the lanes are for, and why a specialized model beats a language
  model at each job, is [docs/architecture/lanes.md](docs/architecture/lanes.md).
- **Knowledge** (`memory.rs`, `session_recall.rs`, `skills.rs`,
  `nomenclature.rs`) — facts, the archive as synthetic facts, the skills
  library, and the vocabulary. Their boundary promise: memory is a
  git-tracked folder of markdown files anyone can read, and a proposal
  never applies silently.
- **The surfaces** (`gui/`, `tui/`, `cli.rs`) — three faces over the one
  core. Their boundary promise: every read the browser performs is
  spawn_blocking off the async worker, and every write rides the module
  the CLI rides — the browser has no second truth.
- **The evidence spine** (`calib.rs`, `eval.rs`, `judge.rs`,
  `delegate.rs`, `explain.rs`) — recording, grading, and the gates that
  decide whether evidence may change behavior.

## Interface contracts

Four boundaries carry every fact in the system; each is documented by
its contract, and the implementation is the module reference.

**The CLI.** `fleet <verb>` with flags anywhere before the task words;
usage errors name the offending flag. Exit codes: `0` completed,
`1` runtime failure, `2` usage or configuration, `3` the turn cap was
reached (resume with `--session <id>`). The authoritative list of
headless entry points is the subcommand surface (`fleet help`).

**The face's HTTP surface.** Hand-rolled HTTP/1.1 on loopback
(`127.0.0.1:3080` by default); the address alone opens it and a
non-loopback host is refused by name. The routes: `/api/run`,
`/api/folder`, `/api/tree` (the pane's contained listing, and
`abs=1` for the folder picker's whole-hierarchy listing),
`/api/file`, `/api/attach`, `/api/settings`, `/api/state`,
`/api/rank`, `/api/related`, `/api/pair/*`, and `/events` — the ordered
event stream the transcript is built from. A paired device reaches the
allowlisted routes with its cookie; the device token is stored as a
digest; `Host`/`Origin` are checked. Every write rides `settings.rs`.

**The MCP lane contract.** stdio, newline-delimited JSON-RPC 2.0
(2024-11-05), spawn → initialize → tools/list → tools/call → teardown,
id-keys pending waiters, per-op timeouts, and named errors only — a
`-32602` reply names the offending field, never a traceback.

**The home's files.** `config.json` (pin, provider overrides, the calib
and skills knobs), `routing.json` (the workload tiers and the gates),
`lanes.json` (per-lane configuration), `mcp.json` (the MCP server
registry: program, args, env NAMES — never values — enabled, the import
source, the last probe), `vault.json` (the sealed
credentials), `sessions/` (the logs), `sessions-workdirs.json` (the
chats' folders), `route-cache.json` (one recorded judgment per task,
keyed to the judging lane), `ui.json` (the browser's counters), and the
calibration day-files under
`~/.local/share/dev-decisions/logs/YYYY/MM/DD/events.jsonl` unless
`config.json`'s `calib.log_root` names another root.

## Environments and dependencies

- **Home.** `FLEET_HOME`, else `~/.fleet`; created by `fleet install`
  with `sessions/`, `memories/`, `skills/`, `cron/`, `proposals/`.
- **Keys.** Each roster candidate names an env var; values resolve from
  the process environment then `<FLEET_HOME>/.env` only — fleet reads
  its own home and nothing else (the owner's 2026-10-16 ruling). Keys
  never print; `fleet doctor` masks to the last four characters. Sealed
  values live in the vault, unlocked per process.
- **The lanes.** Five vendored services under `services/`; heavy
  dependencies (GLiNER, the tabular models, sentence-transformers,
  kokoro, blend1's union) are per-lane venvs under
  `<home>/venvs/<lane>/`, built by `fleet install --lanes`, each skipped
  by name when it cannot be built.
- **The embedding server.** A local llama-server on `127.0.0.1:8901`
  (`FLEET_EMBEDDING_ENDPOINT` overrides), probed once per process; its
  absence degrades recall and ranking to the lexical path by name.
- **The face as a service.** The `com.fleet.gui` LaunchAgent runs the
  binary with `--remote`; restart it with
  `launchctl kickstart -k gui/$(id -u)/com.fleet.gui` after a rebuild.
- **Toolchain.** `cargo` is not on PATH on the owner's machine; the
  toolchain bin is exported first (recorded in `CLAUDE.local.md`,
  gitignored). CI is disarmed by budget: the local gate is the only
  gate.
- **Documents.** Local extraction runs `services/documents/extract.py`
  under a 120-second bound (Office via OOXML parts, legacy via
  LibreOffice, PDFs via `pdftotext`); the hosted parser is the
  escalation, keyed by `NACE_API_KEY` falling back to `DREX_API_KEY`.

## Operational runbooks

- **The gate.** `cargo fmt --all --check`, `cargo clippy --all-targets --
  -D warnings`, `cargo build --release`, `cargo test` — read by exit
  code, never through a status-losing pipe. 1,489 tests at the last
  increment; one live-API e2e is `#[ignore]`d and runs on demand with
  the key set.
- **Rebuild and restart.** `cargo build --release` puts the binary at
  `target/release/fleet` (`~/.local/bin/fleet` is a symlink to it); the
  running face must be kickstarted or it keeps serving the old embedded
  assets. Verify with a grep of the served page for the change.
- **The cache sweep.** `scripts/cache-sweep.sh` prunes stale
  `target/debug` artifacts (age first, a 15 GiB tiered backstop second,
  `target/release` never touched) and declines while a build runs; it
  ends the recap step.
- **Pairing and the remote path.** `fleet channels pair-webhook` or the
  face's pairing card mints a device credential; the webhook token is
  shown once. The remote path is the **outbound relay** on OpenShip
  (`https://fleet.opsh.io`, the deployment runbook lives in
  `docs/features/remote-access.md`): the face dials out, the origin root
  serves the paired face, frame bodies are sealed end to end with a key
  from an X25519 exchange the relay only watched, and the session
  secret rides URLs as a fragment browsers never send. Tailscale is the
  optional fallback origin; without either, the pairing QR points at
  plain-HTTP LAN.
- **The vault.** Unlock per process (`FLEET_VAULT_PASSPHRASE` or the
  face), lock again, or `vault_reset` (archives the old vault, onboards
  a new one — the sealed copies are lost, the plaintext `.env` is not).
- **Cron.** `fleet cron` lists the jobs; the nightly corpus-refresh jobs
  carry their task string in the store, and a job whose stored task
  diverges from the manifest's is reconciled on the next seed (KI-277).

## Verification and release

Work flows plan → build → recap → document, and each stage has a gate:
`plan-surface` precomputes the criterion-to-module map and the
dependency order; `plan-gate` checks criterion coverage, verifiability,
and scope; `evidence-gate` checks that each criterion's evidence exists;
`arch-gate` checks plan claims against this document; `docs-gate` checks
linked-artifact coverage and staleness. The Rust gate is the four
commands above. `fleet eval` is the fixed corpus, and the improve gate
refuses a source self-edit that regresses it. Release is a commit on
`master` (the only branch), pushed, with the face restarted — a change
is not shipped until it is rebuilt, restarted, committed, and pushed.

## Risk register

Drawn from the open ledger (`docs/known-issues.md`); severity as
recorded there.

| Risk | State |
|---|---|
| Windows support unverified — anticipated in code, no build run (KI-011) | open |
| The nightly corpus-refresh jobs fail in place; the task-string reconciliation is the named fix (KI-277) | open |
| A scheduled run's child still inherits the parent's client — `improve_home` doubles as the route home (KI-278) | open |
| The face serves no `/favicon.ico`; Chrome logs one 404 per load (KI-282) | open |
| Tests write calibration rows into the machine's real stream — the rows pollute the boards until a test home carries its own root | open, proposed |
| `docs/architecture/ecosystem.md` routing rows dated in places | open, doc |

## Sequencing

In flight, named with its reason: effort levels per workload (the
router's third dimension; research recorded 2026-10-21); the `composed`
workload's reachability by judgment (sys1's label set is upstream's);
the bot-first phone shell (the owner's 2026-10-20 ruling; the plan is
drafted); the cron children routing fix (KI-278). Landed this week: the
failover tail and the router rulings (`4bd2f18`), the folder picker's
whole-hierarchy browsing (`c40319d`), the context-window table and the
`deepseek-flash` rename (`9c72bbb`).

## Module reference

### Core loop

| Module | Source |
|---|---|
| `main.rs` | `CLAUDE.md` pointer index; `docs/architecture/overview.md`; `docs/architecture/deep-dive-2026-10-15.md` |
| `cli.rs` | `docs/architecture/overview.md`; `docs/architecture/deep-dive-2026-10-15.md` |
| `api.rs` | `docs/architecture/overview.md`; `docs/features/routing.md` |
| `agent.rs` | `docs/architecture/overview.md`; `docs/architecture/deep-dive-2026-10-15.md`; `docs/features/memory.md`; `docs/features/subagents.md` |
| `tools.rs` | `docs/architecture/overview.md`; `docs/features/skills.md` |
| `approvals.rs` | `docs/architecture/overview.md`; `docs/features/self-maintenance.md` |
| `compaction.rs` | `docs/features/compaction.md`; `docs/architecture/deep-dive-2026-10-15.md` |
| `session.rs` | `docs/architecture/overview.md`; `docs/features/subagents.md`; `docs/features/memory.md` |

#### `main.rs`
Dispatch, credential resolution, wiring for `run` / `chat` / `sessions` /
`lanes` / `doctor`. The run-scoped lane roster is created here; background
threads for compaction and consolidation are spawned from here.

#### `cli.rs`
The headless grammar: flags anywhere before the task words, usage errors name
the offending flag. The subcommand surface is the authoritative list of
headless entry points.

#### `api.rs`
The chat-completions client: typed request/response, streaming SSE
parse, tool declarations, and the `ApiError`/`ClientError` taxonomy. The
taxonomy includes 401 credentials, 402 balance, 429 rate limit, network, and
decode failures.

#### `agent.rs`
The tool loop, turn cap, `UiEvent` stream, and run-scoped `LaneRoster`. One
turn reads the owner's control file, assembles the memory block once at run
start, streams the request with dynamic declarations, gates approvals one call
at a time, executes tool waves, logs every message, and checks the window
between turns. `AgentOptions::file_access` is the run's access, one of
three states: `Unrestricted` (the process's working directory — the CLI,
the TUI, and headless runs, launched from a directory), `Folder` (a
chat's working folder: the built-ins work under it, the turn checkpoint
captures it, and a subagent inherits it), and `None` (a chat with no
folder: the four built-ins are refused by name before the approval
gate, no checkpoint is captured, and the refusal asks the owner for a
folder).

The run's system prompt is `SYSTEM_PROMPT` (the voice) followed by
`REPORTING_CONTRACT` — the stop's contract: what happened, where it
stands, what is next, and never silence after a tool call. A `SOUL.md`
identity replaces the voice but the contract is appended either way, so
the expectation of a report is not something an identity slot can drop.
`default_prompt()` is the composition a run with no override carries.

The stop is also enforced, not only instructed. When a run that put tool
calls through the gate stops with an empty message — or a bare "done" —
the loop asks once (`is_silent_stop` + `REPORT_INSTRUCTION` appended to
that one request's system prompt). The silent turn is not pushed to the
conversation, the report it returns is logged as the ordinary assistant
turn, and the ask is emitted as a `ReportRepair` event so the extra
model call is visible. One ask per run; a second silent stop ends with
the stop reason `completed without a report` rather than `completed`.

#### `tools.rs`
The closed built-in tool set. Every failure is a model-visible result, never a
panic. This module also declares the tool vocabulary the dynamic surfaces
assemble: built-ins, `mcp__<svc>__<tool>`, skills, memory tools, `subagent`,
and `web`. [`Tool::execute`] is the unrestricted run (the process's working
directory, paths as named); [`Tool::execute_in`] is a chat folder, where
`bash` starts in the folder and a relative path resolves under it. The folder
is a boundary on the three file tools and the shell's starting directory, not
a sandbox: a subprocess can still `cd` wherever the owner's account can.

#### `approvals.rs`
The three-mode approval gate: ask, yes, yes-guarded. The class checks are
label-aware across delegation depth 2.

#### `compaction.rs`
Summarize-and-continue under `COMPACTION_TOKEN_BUDGET`. The trigger runs once
per window crossing; the summary is logged as a session record; the tail keeps
the most recent four turns verbatim.

#### `session.rs`
Append-only JSONL session logs at format version 1. Replay fails loud, naming
file and line. A log without `end` was interrupted. Bump `SESSION_FORMAT_VERSION`
only when existing logs cannot be read safely.

### Routing and lanes

| Module | Source |
|---|---|
| `route.rs` | `docs/features/routing.md`; `docs/architecture/deep-dive-2026-10-15.md`; `docs/architecture/ecosystem.md` |
| `route_cache.rs` | `docs/features/routing.md`; `docs/architecture/deep-dive-2026-10-15.md` |
| `lane_client.rs` | `docs/features/blend1.md`; `docs/architecture/deep-dive-2026-10-15.md` |
| `lanes.rs` | `docs/architecture/overview.md`; `docs/features/install.md`; `services/README.md` |
| `mcp.rs` | `docs/architecture/overview.md`; `docs/features/blend1.md` |
| `judge.rs` | `docs/features/calibration-loop.md`; `docs/architecture/deep-dive-2026-10-15.md` |

#### `route.rs`
The roster, the workload preference table, route selection, and failover.
Selection: an exact pair pin wins, otherwise sys1_classify judges the
workload; the leader walks the `hard` chain and every other route walks
the judged workload's own; gates can promote the default workload as
`Fallback`; every order closes on the deepseek-flash failover tail. The
2026-10-16 ruling removed the precursor from the roster; the chain
starts at `zai-coding-plan`.

#### `route_cache.rs`
One recorded judgment per task, keyed to the judging lane. The cache prevents
redundant sys1 judgments across identical tasks, across sessions — it is a
persistent file beside the roster it describes, not a per-session one.

#### `lane_client.rs`
The lane-kind candidate boundary: a chat request in, blend1's grounded answer
out. The adapter translates Fleet's request vocabulary into the lane's own
tool calls and normalizes the result.

#### `lanes.rs`
Home resolution: env override, the home's `lanes.json`, bundled `services/`,
or unavailable with one named log line. Launch specs, health probes, the
lane catalog the settings pickers read, the `fleet lanes` card renderer, and
the launch-time configuration live here. `configured_spec` injects a lane's
configured env values and the `.env`/vault values for the names the lane
declares (process env wins; an undeclared name never crosses); per-call
defaults merge under the model's lane arguments, which always win.

#### `mcp.rs`
The hand-rolled MCP stdio client: spawn, initialize, tools/list, tools/call,
teardown. It speaks newline-delimited JSON-RPC 2.0, id-keys pending waiters,
and applies per-op timeouts.

#### `mcp_registry.rs`
The MCP server registry (`docs/features/mcp.md`): the `mcp.json` document,
its validation (reserved names — the six lanes and the built-in tools —
name shapes, and the no-value schema whose `env_names` is the only env
field), the scan over the four foreign harness configs with its shape
reader for Codex's TOML subset, the `-e NAME=VALUE` extraction, the
advisorial badges, the `fleet mcp` card renderer, and the health probe.
The writers live in `settings.rs` beside `set_lane`.

#### `judge.rs`
The routing judgment beat: `fleet judge` grades a day's route decisions through
sys1. The grader sees redacted route rows only — workload, verdict, attempts,
outcomes — never task text.

### Memory and skills

| Module | Source |
|---|---|
| `memory.rs` | `docs/features/memory.md`; `docs/architecture/deep-dive-2026-10-15.md` |
| `session_recall.rs` | `docs/features/memory.md`; `docs/architecture/deep-dive-2026-10-15.md` |
| `skills.rs` | `docs/features/skills.md` |
| `nomenclature.rs` | `docs/features/skills.md`; `docs/features/memory.md` |

#### `memory.rs`
The fact substrate, hybrid ranker, recall hits, write path, consolidation
cycle, and provenance model. Facts are one markdown file per durable entry
under `memories/`, with hybrid ranking from vector, keyword, history,
freshness, trajectory, and provenance channels. The protected tier (`user`,
`feedback`) never competes and is never evicted, even past budget. The
consolidation cycle runs on a timer and logs `consolidate` rows.

#### `session_recall.rs`
The session archive as synthetic facts through the same ranker. One synthetic
fact per log whose phrases are successful tool calls, whose shape is the
outcome word, and whose basis is `inferred`. Its own count store is beside the
archive, never the session log.

#### `skills.rs`
Skills loading, progressive disclosure via `use_skill`, the self-authoring
drafter, usage-scan curation, and the `fleet skills` surface. A malformed
skill is skipped by name with one logged line.

#### `nomenclature.rs`
The canonical identifier vocabulary and the `convention` lookup. It names the
project's established terms and warns on drift.

### Delegation and scheduling

| Module | Source |
|---|---|
| `subagents.rs` | `docs/features/subagents.md`; `docs/architecture/deep-dive-2026-10-15.md` |
| `delegate.rs` | `docs/features/calibration-loop.md`; `docs/architecture/deep-dive-2026-10-15.md` |
| `graph.rs` | `docs/architecture/deep-dive-2026-10-15.md`; `docs/features/cron.md` |
| `roles.rs` | `docs/features/subagents.md` |
| `cron.rs` | `docs/features/cron.md`; `docs/architecture/deep-dive-2026-10-15.md` |

#### `subagents.rs`
Delegation to depth 2. A child is routed by the router over its own brief
(`route::prepare_child`, `Walk::Judged`) under the parent's pin, and runs on
its own routed client; a parent on a direct, unrouted client — every test
double — hands the child the same client. Scope is declarative; approval is
forwarded with calls renamed `subagent -> <tool>`; the result is never
silence.

#### `delegate.rs`
The delegation judgment beat: `fleet delegate` grades a day's delegations
against outcome evidence and lands `delegation_judged` rows.

#### `graph.rs`
The declared dependency graph: scores, gates, critical path, and fact-based
verification. Nodes start when prerequisites verify; the resource edge prevents
concurrent execution of competing paths.

#### `roles.rs`
Named delegation specializations: one frontmatter file per role under
`<FLEET_HOME>/roles/`, carrying description, admitted tools, skills, memory
shapes, and an optional system-prompt overlay. `catalog` names every role and
every broken file; `write` validates the loader's name rule, preserves
unknown frontmatter keys, and writes atomically; `remove` deletes by name.

#### `cron.rs`
Scheduled runs: the job store, five-field parser, catch-up on missed windows,
and delivery through the same agent loop. `fleet cron` lists as cards.

### Self-calibration and self-maintenance

| Module | Source |
|---|---|
| `calib.rs` | `docs/features/calibration.md`; `docs/features/calibration-loop.md`; `docs/architecture/deep-dive-2026-10-15.md` |
| `eval.rs` | `docs/features/eval.md`; `docs/architecture/deep-dive-2026-10-15.md` |
| `improve.rs` | `docs/features/self-maintenance.md`; `docs/architecture/deep-dive-2026-10-15.md` |
| `explain.rs` | `docs/features/explain.md`; `docs/architecture/deep-dive-2026-10-15.md` |

#### `calib.rs`
The audit stream: one `events.jsonl` per UTC day, non-blocking bounded queue,
serialized writer, and drain-at-teardown semantics. Supported ops include
`route`, `tool_screen`, `turn`, `recall_hit`, `consolidate`, `propose`,
`eval_corpus`, `explain`, `route_judged`, and `delegation`.

#### `eval.rs`
The graded eval corpus: fixed scenarios, headless mock-driven runs, pass/fail
per scenario, aggregated score, and one `eval_corpus` row. The improve gate
uses the corpus as a regression check for source edits.

#### `improve.rs`
Self-maintenance proposals for skills, memory, and config. Every change is a
proposal; applying runs the full local gate; destructive changes require
approval; reverts are byte-for-byte.

#### `explain.rs`
Evidence-first explanation of recorded verdicts through blend1. Eval-only and
batch-only by construction.

### Surfaces and support

| Module | Source |
|---|---|
| `home.rs` | `docs/features/install.md`; `CLAUDE.md` hard rule 4 |
| `settings.rs` | `docs/features/settings` behavior surfaced through `src/cli.rs`; `docs/architecture/deep-dive-2026-10-15.md` |
| `boards.rs` | `docs/features/boards.md`; `docs/architecture/deep-dive-2026-10-15.md` |
| `install.rs` | `docs/features/install.md`; `docs/architecture/overview.md` |
| `doctor.rs` | `docs/architecture/overview.md`; `docs/architecture/deep-dive-2026-10-15.md` |
| `net.rs` | `docs/architecture/overview.md` |
| `tui/` | `docs/architecture/deep-dive-2026-10-15.md`; `docs/architecture/overview.md` |
| `gui/` | `docs/features/gui.md` |
| `checkpoints.rs` | `docs/features/gui.md` (the trust trio) |
| `deck.rs` | `docs/features/gui.md`; the deck pipeline's render stage |
| `vault.rs` | AES-256-GCM at rest, keyed by a passphrase |

#### `home.rs`
Fleet home resolution, `.env` fallback, and credential resolution. This is
the single source of truth for "Fleet reads Fleet's home and nothing else."

#### `settings.rs`
One writer over `config.json` / `routing.json` / `fleet.env` / `lanes.json` /
`mcp.json` (`set_server` / `remove_server` / `set_server_enabled` /
`record_server_probe` — the read-merge-write pattern of `set_lane`, without
its credential-name refusal, which a server registry cannot reuse), and the
secret writer over the vault and `.env`. The CLI, TUI, and GUI
settings surfaces read and write through this module. `router_config` reads
the tiers, gates, pin, and roster through the router's own loader and key
resolution, naming the first candidate that would serve each tier and the
first blocker when none would. `catalog` is the one read every GUI picker
draws from: roster models plus the current table's values, provider states,
env names with presence/source and never a value, paths, tools, skills, and
shapes. `set_lane` validates a lane object against the lane registry and
writes it atomically; `set_secret` seals into the vault when the process
holds the passphrase, else writes one atomic `.env` line. The process's
vault passphrase is a `home`-owned unlock state (launch-time
`FLEET_VAULT_PASSPHRASE`, overridable in place by the GUI and lockable
again); `vault_verify` refuses a passphrase that does not open the
existing entries, `vault_init` creates and migrates the credential-shaped
`.env` names without dropping their plaintext lines, and `vault_seal_env`
seals one plaintext value server-side. `vault_onboard` is the first-run
path (choose the passphrase, create, migrate, unlock); `vault_reset`
archives the existing vault as `vault.json.bak-<stamp>` and onboards a
new one, so a forgotten passphrase loses the sealed copies but never the
evidence or the plaintext `.env` credentials.

#### `boards.rs`
The three read-only boards: Fleet's own routing state, the second-brain's
derived state files, and git facts per project. A refused read keeps the last
snapshot visible and names why.

#### `install.rs`
Home layout creation, config scaffold, and optional lane venv bootstrap. The
install is idempotent; doctor is its gate.

#### `doctor.rs`
Startup checks as pure evaluators over gathered facts. A broken layer is named
at boot with its reason.

#### `net.rs`
Loopback reachability probe used by doctor.

#### `tui/`
The ratatui surface: `app.rs` state machine, `events.rs` key mapping,
`panels.rs` the knowledge panels' on-demand reads, `ui/` the renderers
(`mod.rs` layout, `board_view.rs` the composed router/tower boards,
`run.rs` the run card and controls, `command.rs` the palette, `trust.rs`
checkpoints/diff/confirm, `knowledge.rs` the memory, skills,
calibration and delegation panels, `lanes.rs` the lane table, and
`widgets.rs` the shared scrollbar), and `mod.rs` the event loop.
The right pane is a tabbed surface (run, router, brain, tower); the
conversation carries the plan card and a scrollbar; the status bar carries
a context gauge. Stop/steer ride the agent's control file, rewind/diff ride
`checkpoints.rs`, and plan mode is the per-run `AgentOptions` bit.

#### `gui/`
The browser face: `mod.rs` the serve entry, `server.rs` the hand-rolled
HTTP/1.1 loop, `bridge.rs` the agent thread and the ordered event log,
`related.rs` the cross-kind related index beside the two recall caches,
`usage.rs` the browser's counters in `<home>/ui.json`, `themes.rs` the
theme set, and `assets/` the embedded page. Every read is a
spawn_blocking off the async worker; every write rides the module the
CLI rides. The trust trio's floor is `checkpoints.rs`; the deck pipeline
renders through `deck.rs`; the run's stop and steer ride the agent's own
control-file protocol in `delegate.rs`. A chat's working folder is the
bridge's state, persisted in `<home>/sessions-workdirs.json` and resolved
against the launch directory when the run is accepted; the folder picker
browses the whole hierarchy through the `abs` listing and commits through
the same writer; the checkpoint store stays under the home and only its
capture root follows the folder.

### Integration harness

| Module | Source |
|---|---|
| `tests/integration.rs` | `docs/architecture/overview.md`; `docs/features/*.md` |
| `src/bin/mock-server.rs` | `docs/architecture/overview.md` |
| `src/bin/mock-mcp.rs` | `docs/architecture/overview.md` |

#### `tests/integration.rs`
End-to-end tests for parser, client, tool loop, session log, lane paths, and
eval scenarios.

#### `src/bin/mock-server.rs`
The scripted mock API the integration suite drives as a child process.

#### `src/bin/mock-mcp.rs`
The scripted fake MCP stdio server for deterministic paths the real lanes
cannot reproduce.

### Documents and Perception

`src/documents.rs` decides the local route by extension and runs
`services/documents/extract.py` under a 120-second bound. Office files are
read as their OOXML parts, legacy binaries convert through LibreOffice
`--safe-mode`, PDFs through `pdftotext`, and a PDF with no text layer is
the escalation case. Macros are never executed and embedded objects are
never opened.

`src/perception.rs` is the hosted client: an upload grant, a multipart
upload, `POST /v1/documents/parse`, and `GET /v1/documents/jobs/{id}`
polling to a terminal status (`queued`, `running`, then `succeeded`,
`failed` or `cancelled`), with results cached by FNV-1a content hash under
`<FLEET_HOME>/perception-cache`. The key is `NACE_API_KEY`, falling back
to `DREX_API_KEY`, read through the vault/dotenv readers; the API is
server-side only (the `/v1` routes carry no CORS header). The attach
route is `POST /api/attach`; the browser never sees the key or the upload
token.

## Known documentation gaps

| Area | Status | Likely source |
|---|---|---|
| `docs/features/settings.md` | Covered by `docs/features/gui.md`'s "The configuration surface" section | `src/settings.rs`, `src/cli.rs`, `src/tui/app.rs`, `docs/architecture/deep-dive-2026-10-15.md` |
| `docs/architecture/ecosystem.md` routing rows | Dated in places | `docs/plans/2026-10-16-fleet-routes-through-its-own-router.md`, `src/route.rs` |
| `services/README.md` lane count | Corrected in code; doc wording should be re-verified | `src/lanes.rs`, `fleet doctor` |

These gaps are noted here as missing coverage, not as defects in code. When
this file falls out of sync with the sources above, prefer fixing the source
module doc or feature doc and then re-aggregating this file.
