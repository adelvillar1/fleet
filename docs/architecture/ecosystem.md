# The ecosystem Fleet is meant to absorb

Owner's intent (2026-10-13): Fleet becomes the self-contained Rust harness for the whole Fleet-of-Ones ecosystem — the agent core that already lives here, plus the lanes, the calibration layer, the router, second-brain, and the dashboards. This file is the verified map of that ecosystem and where each piece stands.

**Verification caveat:** states below are git-level, verified 2026-10-13 against each repo's working tree. Runtime health (do the MCP servers answer, do the dashboards render) has **not** been probed — that is a follow-up, not a claim.

## The lanes — "the sys1 house pattern"

**The lanes are parts of the harness, not external dependencies.** All four are vendored *into* the fork at `services/<svc>/` (each with `src/`, `service/mcp_server.py`, `LICENSE`, and a `VENDOR.json` pinning the upstream repo + commit), and `@fleet/dsh-components` launches each bundled copy as an MCP stdio child (`python3 -u <home>/service/mcp_server.py`), registering tools under `mcp__sys1__*`, `mcp__sdm1__*`, `mcp__sem1__*`, `mcp__gen1__*`. Home resolution: a machine override (`homes` row or `DSH_COMPONENT_<SVC>_HOME` — a live checkout) wins, then the bundled copy, then the component is skipped with one named log line and the boot continues. Vendored in one commit (2026-10-09, "Wave B: the services bundle into the fork"); ~12,400 LoC of Python bundled; the sync rule is "upstream repos stay the development home; re-vendor by copying src/ and service/mcp_server.py at a recorded sha".

The external repos are the development homes; the harness copy is what a clone carries:

| Repo | Role | Tools published | External state (2026-10-13) | Vendored pin |
|---|---|---|---|---|
| sys1 | Judgment: decision models over text (Jev, Drex, GLiDE, Julia, Liquid d1, GLiNER-2.5-Decide, local GLiNER2) | `sys1_classify` | branch `feature/plan-surface`; 4 uncommitted files (MCP provider accept, safety task allow-list, api, tasks) — **not vendored** | `c1f7383` = HEAD |
| sdm1 | Tabular: structured-data foundation models (NVIDIA SDM/Kumo-Tabular, Prior Labs TabPFN hosted) | `sdm1_classify`, `sdm1_health` | clean on `main`, pushed | `eeb3314` = HEAD |
| sem1 | Embeddings: language and images as geometry — "vectors in, vectors out, never verdicts" | `sem1_embed`, `sem1_probe`, `sem1_index`, `sem1_search` | clean on `main`, pushed; **no CLAUDE.md** (no methodology layer) | `a338912` = HEAD |
| gen1 | Media generation: renders the gates judge | `gen1_imagine` | clean on `master` | `c340313` = HEAD |

All four pins match the upstream repos' committed HEADs, so the bundled copies are in sync as of 2026-10-09; sys1's uncommitted feature-branch work is the only drift, and it exists in no commit.

**Runtime probe (2026-10-13, this session):** all four bundled cores were booted directly from `services/<svc>/service/mcp_server.py` with an MCP `initialize` + `tools/list` handshake over stdio. All four answered initialize (serverInfo name + version 0.1.0) and published their tool lists. The cores are stdlib-only at module level — heavy lanes (sentence-transformers, GLiNER2.5, kokoro-onnx, llama-server) live in external venvs installed by `services/bootstrap.mjs`, and a provider whose lane is absent degrades to nothing while the service still boots, which is what the probe exercised. One deviation found: sys1 and gen1 answer the `notifications/initialized` notification with a JSON-RPC `-32601` error instead of ignoring it (sdm1 and sem1 handle it correctly — their "ignore notifications and answer ping per MCP 2024-11-05" fix landed after sys1/gen1 were vendored).

The house pattern in one sentence (from sem1's README): sys1 judges what the work says, sdm1 scores what the work measures, sem1 indexes what the work looks like, gen1 renders it.

## Calibration and routing

- **dev-decisions** — the verification and calibration layer: gates operations (commit hygiene, plan contracts, rendered UI), classifies diffs with multiple providers, logs every decision and graded outcome to JSONL. Clean on `main` with three feature branches (tabular, semantic, media-generation lanes).
- **agnostic-router-kit** — a local, harness-neutral model router and the workflow plane it runs; owns `lib/workflow/` and the proxy; depends on no harness. Serves its own dashboard at `http://127.0.0.1:8300/dashboard` (usage ledger, provider enable/disable, quota, delegation, live run activity). On `master` with 4 untracked `docs/architecture/refresh-*` dirs; has the full methodology layer (CLAUDE.md + TECHNICAL-DOCUMENTATION.md + FUNCTIONAL-SPECIFICATIONS.md). Its Oct 7–9 CI runs dispatched and failed on real test steps — those predate the Actions-budget exhaustion and are a separate, genuine red.

## Second-brain

`~/Projects/second-brain` — one live control tower for the solo developer's multi-harness agent fleet: every chat, plan, recap, and skill inventoried, classified, reconciled against git, served as a localhost dashboard. **Never exposed beyond the machine.** Clean on `main`; last commit 2026-10-09 added the harness-outcomes ingester (`record_harness.py` → `harness_state.json`, harness job in the build chain, Harness outcomes card on the cockpit).

## The dashboards (fork-side, TypeScript)

In the DeepSeek Harness fork, `packages/fleet/dsh-*` composes the ecosystem into the harness's "fleet profile":

- `dsh-board-seam` — one board implementation shared by all dashboards (cached-snapshot service, one guarded GET route, fetch-and-poll browser source, page shell).
- `dsh-router-board` — the agnostic-router-kit plane made visible: state, usage, suggest, workflow-runs, read-only over `127.0.0.1:8300` with the roster bearer. Completed 2026-10-12 (plan `docs/plans/2026-10-12-fleet-dashboards.md`).
- `dsh-second-brain` — second-brain's derived state (`reports_state.json`, `gates_state.json`, `harness_state.json`, `maturity_state.json`, `loops_state.json`, `tower_data.json` at `~/.second-brain`) as a read-only board. Completed 2026-10-12.
- `dsh-tower` — git-facts portfolio awareness and turn reporting.
- `dsh-brain`, `dsh-sem1`, `dsh-sys1`, `dsh-sdm1`, `dsh-decisions`, `dsh-doctor`, `dsh-components`, `dsh-router`, `dsh-fleet-ui` — the lane wirings, boot probes, credential resolution, and branding for the profile.

As of the follow-ups recap (2026-10-13): 533 tests green across the five fleet packages; the two activation/bearer defects found that day were fixed and proven against the live profile.

## What "absorbing into Fleet" means per piece

Nothing below is built — this is the shape of the work, for slice-2 planning:

| Piece | Absorption shape |
|---|---|
| Lanes (sys1/sdm1/sem1/gen1) | Fleet already speaks MCP-shaped tool calls; the lanes stay services, Fleet mounts them as tools (stdio JSON-RPC children) instead of shelling out |
| dev-decisions | the calibration ledger becomes a Fleet-side recorder; gates run before/after each run |
| agnostic-router-kit | routing becomes a client concern in `api.rs` (declared workload chains, quota steering, failover) or a local sidecar; its dashboard data is read-only over its operator API |
| second-brain | derived state stays in second-brain; Fleet gets a read-only board surface |
| Dashboards | native Rust surfaces (or an embedded web view) over the same read-only sources the dsh-* plugins read today |

## Known cross-repo facts

- The DeepSeek Harness credential store (`~/.dsh/.credentials.yaml`, `refs` block) is the shared key mechanism — Fleet reads it read-only and so does the fork.
- The router kit's roster is authored in its checkout and rendered by `kit apply` into its state home; do not point at a checkout path from another app (the defect fixed 2026-10-13).
- gen1's media lane and the router kit share a machine-row JSON contract (`devdec` wire vocabulary) for calibration.
