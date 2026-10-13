# The ecosystem Fleet is meant to absorb

Owner's intent (2026-10-13): Fleet becomes the self-contained Rust harness for the whole Fleet-of-Ones ecosystem — the agent core that already lives here, plus the lanes, the calibration layer, the router, second-brain, and the dashboards. This file is the verified map of that ecosystem and where each piece stands.

**Verification caveat:** states below are git-level, verified 2026-10-13 against each repo's working tree. Runtime health (do the MCP servers answer, do the dashboards render) has **not** been probed — that is a follow-up, not a claim.

## The lanes — "the sys1 house pattern"

| Repo | Role | Interface | State (2026-10-13) |
|---|---|---|---|
| sys1 | Judgment: decision models over text (Jev, Drex, GLiDE, Julia, Liquid d1, GLiNER-2.5-Decide, local GLiNER2) | MCP server, stdio JSON-RPC: `sys1_classify` | branch `feature/plan-surface`; 4 uncommitted files (MCP provider accept, safety task allow-list, api, tasks) |
| sdm1 | Tabular: structured-data foundation models (NVIDIA SDM/Kumo-Tabular, Prior Labs TabPFN hosted) | MCP: `sdm1_classify`, `sdm1_health` | clean on `main`, pushed |
| sem1 | Embeddings: language and images as geometry — "vectors in, vectors out, never verdicts" | MCP: `sem1_embed`, `sem1_probe`, `sem1_index`, `sem1_search` | clean on `main`, pushed; **no CLAUDE.md** (no methodology layer) |
| gen1 | Media generation: renders the gates judge | MCP: `gen1_imagine` | clean on `master` |

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
