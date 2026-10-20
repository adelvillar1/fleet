# Technical documentation

This file is the project's module-by-module technical reference. It
aggregates the existing sources: `docs/architecture/overview.md`,
`docs/architecture/deep-dive-2026-10-15.md`, `docs/architecture/ecosystem.md`,
`docs/features/*.md`, and the module doc comments in `src/*.rs`.

If a module or surface has no entry here, it is either covered in one of the
architecture docs above or missing. The "Source" column points to the file
whose claims were aggregated here; when the code changes, follow that file.

## Crate contract

- One static Rust binary; no Node runtime, no checkout dependency.
- Tested with zero real network by design: the integration suite drives
  `src/bin/mock-server.rs` as a child process and `src/bin/mock-mcp.rs` for
  MCP edge cases.
- The live-API end-to-end test is `#[ignore]`d and self-skips without a key.
- Credentials never print; `fleet doctor` masks to the last four characters.
- Model-visible output is never a panic: every tool failure is a model-visible
  `error:` result, and the session log reconstructs exactly what the model saw.
- The local gate is the only gate while CI is disarmed by budget:
  `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings`,
  `cargo build --release`, `cargo test`.

## Environment contract

- `FLEET_HOME` defaults to `~/.fleet`.
- Roster keys resolve from the process environment then `<FLEET_HOME>/.env` only.
  Fleet never reads another harness's credential store.
- `DEEPSEEK_API_KEY` may live in either leg; the machine's local quirks are
  recorded in `CLAUDE.local.md` (gitignored).
- Session logs live under `<FLEET_HOME>/sessions/`.

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
The DeepSeek chat-completions client: typed request/response, streaming SSE
parse, tool declarations, and the `ApiError`/`ClientError` taxonomy. The
taxonomy includes 401 credentials, 402 balance, 429 rate limit, network, and
decode failures.

#### `agent.rs`
The tool loop, turn cap, `UiEvent` stream, and run-scoped `LaneRoster`. One
turn reads the owner's control file, assembles the memory block once at run
start, streams the request with dynamic declarations, gates approvals one call
at a time, executes tool waves, logs every message, and checks the window
between turns.

#### `tools.rs`
The closed built-in tool set. Every failure is a model-visible result, never a
panic. This module also declares the tool vocabulary the dynamic surfaces
assemble: built-ins, `mcp__<svc>__<tool>`, skills, memory tools, `subagent`,
and `web`.

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
Selection: pin wins, otherwise sys1_classify judges the workload, the
preference table walks the declared chain, and gates can promote the default
workload as `Fallback`. The 2026-10-16 ruling removed the precursor from the
roster; the chain starts at `zai-coding-plan` and DeepSeek serves only when
pinned.

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
Delegation to depth 2. A child runs on the parent's own routed client and
model; scope is declarative; approval is forwarded with calls renamed
`subagent -> <tool>`; the result is never silence.

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
One writer over `config.json` / `routing.json` / `fleet.env` / `lanes.json`,
and the secret writer over the vault and `.env`. The CLI, TUI, and GUI
settings surfaces read and write through this module. `router_config` reads
the tiers, gates, pin, and roster through the router's own loader and key
resolution, naming the first candidate that would serve each tier and the
first blocker when none would. `catalog` is the one read every GUI picker
draws from: roster models plus the current table's values, provider states,
env names with presence/source and never a value, paths, tools, skills, and
shapes. `set_lane` validates a lane object against the lane registry and
writes it atomically; `set_secret` seals into the vault when the process
holds the passphrase, else writes one atomic `.env` line.

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
control-file protocol in `delegate.rs`.

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

## Known documentation gaps

| Area | Status | Likely source |
|---|---|---|
| `docs/features/settings.md` | Covered by `docs/features/gui.md`'s "The configuration surface" section | `src/settings.rs`, `src/cli.rs`, `src/tui/app.rs`, `docs/architecture/deep-dive-2026-10-15.md` |
| `docs/architecture/ecosystem.md` routing rows | Dated in places | `docs/plans/2026-10-16-fleet-routes-through-its-own-router.md`, `src/route.rs` |
| `services/README.md` lane count | Corrected in code; doc wording should be re-verified | `src/lanes.rs`, `fleet doctor` |

These gaps are noted here as missing coverage, not as defects in code. When
this file falls out of sync with the sources above, prefer fixing the source
module doc or feature doc and then re-aggregating this file.
