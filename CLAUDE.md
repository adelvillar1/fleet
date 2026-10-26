# Fleet

A native Rust agent for the DeepSeek API — and the intended home of a self-contained Rust harness for the whole Fleet-of-Ones ecosystem. Repo: `git@github.com:adelvillar1/fleet.git` (private, MIT). Status: alpha 0.2.0 — well past the agent-core slice: routing, memory recall, delegation, scheduling, self-calibration, and the TUI are all in-repo (see Today's state).

## What Fleet is

Slice 1 shipped: one static binary with its own `ureq`-based DeepSeek chat-completions client, a closed four-tool set (`bash`, `read_file`, `write_file`, `list_dir`), terminal tool approvals, append-only JSONL session logs with resume, and `doctor`. No Node runtime, no harness checkout, no dsh launch path.

The owner's intent (2026-10-13): Fleet is the self-contained Rust successor to the DeepSeek Harness fork's "fleet profile". It is meant to absorb the entire ecosystem, not just the agent core:

- the sys1 house lanes — sys1 (judgment), sdm1 (tabular), sem1 (embeddings), gen1 (media), plus the composed blend1 lane — vendored as Python service cores under `services/`, driven through fleet's own MCP client
- dev-decisions — the verification and calibration layer: absorbed as the calibration stream's row vocabulary and log root (`src/calib.rs`)
- agnostic-router-kit — model routing: the precursor, not a dependency. Its roster and workload preference table were the reference for Fleet's own router (`src/route.rs`); the kit's operator API, `/dashboard`, and port 8300 are the precursor's own and are read by nothing in Fleet
- second-brain — the localhost control tower: its derived state files are read read-only through the second-brain board (never exposed beyond the machine)
- the dashboards — router board, second-brain board, and tower over the board seam: rendered read-only in the TUI's board pane (`src/boards.rs`) for fleet's own surface, replacing the fork's TypeScript plugins here

Verified inventory and per-repo state: [docs/architecture/ecosystem.md](docs/architecture/ecosystem.md). Current crate architecture: [docs/architecture/deep-dive-2026-10-15.md](docs/architecture/deep-dive-2026-10-15.md) (the current-state survey; `overview.md` is the slice-1 snapshot with a banner).

## Today's state (2026-10-16)

- In-repo and green: `cargo test` → 1013 passed, 1 ignored at the last increment's commit; the gate runs whole locally — `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings`, `cargo build --release`, `cargo test`, all four exit 0 (rustfmt and clippy were installed 2026-10-15, closing KI-007, and the first full clippy pass landed clean). The crate is 45 modules (~70k lines): the agent core, the five vendored lanes over the hand-rolled MCP client, multi-provider routing with the sys1 judgment and failover, the git-tracked memory substrate with six-channel recall, session-archive recall, skills, delegation to depth 2 with the dependency graph and cron, the calibration stream with its judge/delegate/explain beats and the eval corpus, the settings writer, and the ratatui TUI. The 2026-10-15 sessions landed the deep dive, its six-phase findings plan, and the dogfooding viability fixes (`docs/plans/2026-10-15-dogfooding-viability-fixes.md`: the bash wall-clock bound, the approval gate's label-blind class checks, the routing record's name, the consolidation fold's honesty, the torn-tail tolerance). 2026-10-16 landed the owner's own-home ruling (`docs/plans/2026-10-16-own-home-credentials.md`): credentials resolve from the process environment and `<FLEET_HOME>/.env` only, with the DeepSeek Harness credential-store leg and its helpers removed outright. 2026-10-16 also landed the router ruling (`docs/plans/2026-10-16-fleet-routes-through-its-own-router.md`): Fleet routes through the in-process `route::select` it owns — the `agnostic-router-kit` is the precursor, not a dependency, and its roster entry is gone entirely; the router board reads Fleet's own calibration rows and dials nothing.
- Environmental blockers, none of them code:
  - **CI is red by budget.** GitHub Actions minutes are exhausted; jobs never dispatch (job record: `runner_id: 0`, zero steps, ~3s). Until the budget resets, the gate must be run locally on every change.
  - **DeepSeek is funded, and it is the pay-as-you-go reserve** — owner-confirmed 2026-10-15 (the earlier 402 was transient; KI-008 records it): the account has balance, and the roster walks the other candidates first. Since the owner's 2026-10-21 ruling it is **every walk's failover tail**: no workload targets it, but the router appends `deepseek/deepseek-flash` as the last entry of every order it builds (judged, fallback, pinned, side-routes), so it serves only after every routed candidate has failed — and the decision row names it when it does (`served by the failover tail`). The `allow_api_models` gate keeps it out like every hosted model. The live-API e2e stays `#[ignore]`d by default and can run on demand with the key set.
- Ecosystem repos (sys1, sdm1, sem1, gen1, dev-decisions, agnostic-router-kit, second-brain) live outside this repo; their git-level states are verified in the ecosystem doc. The fork stays the spec/integration lab, never a shipping competitor.

## Hard rules

1. **Plan before non-trivial work.** Draft `docs/plans/YYYY-MM-DD-<slug>.md` with acceptance criteria before any feature ≥15 min or >2 files; update its `status` in the same turn the work lands. Trivial work may go direct.
2. **The gate is local and permanent:** `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings`, `cargo build --release`, `cargo test`. Run it before every push and **read the exit code — never through a status-losing pipe** (KI-014). CI is disarmed by owner decision (no Actions funds); the workflow is manual-dispatch only until the owner re-arms it.
3. **Commit each verified increment as it goes green.** Explicit paths only — never `git add -A`. `master` is the only branch (solo tool, no hosted surface).
4. **Never commit secrets.** Credentials resolve at runtime, from Fleet's own home and nothing else: `DEEPSEEK_API_KEY` env → `<FLEET_HOME>/.env`. No other harness's config file is ever read (the owner's 2026-10-16 ruling). This repo stores no keys; env var names live in `CLAUDE.local.md`.
5. **Model-visible ⟺ logged** (inherited from the harness): anything the model sees must be reconstructable from the session log. The system prompt and tool declarations are compiled-in constants, reconstructed per request, never logged.
6. **Native port, not a fork.** New work is Rust against the DeepSeek API; never reintroduce Node, a checkout dependency, or a dsh launch path (plan criterion C5).
7. **Destructive operations** (session deletion, history rewrite, force push) require explicit approval in the current turn.
8. **Documentation is English-only** — no Chinese or other non-English docs in this repo (a deliberate departure from the fork's bilingual convention).
9. **The leader plans on `hard`; the router rules everything else** (the owner's 2026-10-21 ruling, narrowing 2026-10-16's). The leading session — the run that assigns work and supervises it — walks the `hard` workload's chain whatever its task was judged. A delegated child is routed by the router over its own brief (`route::prepare_child`: the judged workload's chain, under the parent's pin), never handed the parent's committed candidate. No surface seeds a provider's model as a default: `route::AUTO_MODEL` until the route answers, and `--model` is a pin resolved by name. Every walk closes on the deepseek-flash tail.
10. **Every error or warning is logged, not ignored** — anything encountered, in this crate or any sibling repo, lands in `docs/known-issues.md` with its evidence. "Not my code" is a reason to log, not a reason to skip.
11. **UX is co-equal with technical maturity** — every slice ships its user-facing surface with the feature; UX is a per-slice deliverable at the "genuinely visual, never plain-text tables" bar, never a polish finale.

## Where to find things

```
src/main.rs             dispatch, credential resolution, run/chat/sessions/lanes/doctor wiring
src/cli.rs              argument parsing, help text, usage errors
src/api.rs              DeepSeek chat-completions client (ureq), streaming, typed errors
src/boards.rs           the three read-only boards: Fleet's own routing state, the second-brain's derived state files, git facts per project
src/agent.rs            the tool loop, approvals, turn cap, progress reporting
src/tools.rs            the eight built-ins, schemas and validation, alias resolution, output capping
src/fileops.rs          window reads, atomic writes, the anchored edit and its diff
src/tool_advisor.rs     the session scan and the build/fix/adopt/retire advice
src/tool_audit.rs       one session's tool rates: validity, classes, p95, recovery, resolutions
src/search.rs           grep and glob, capped and counted
src/tool_router.rs      sys1 capability parsing, sem1 ranking, floor/margin, lexical fallback
src/approvals.rs        the three-mode approval gate: ask, yes, yes-guarded, and the always-ask class
src/compaction.rs       context-window management: summarize-and-continue under a token budget
src/mcp.rs              the hand-rolled MCP stdio client (spawn, initialize, tools/list, tools/call, teardown)
src/lanes.rs            the six vendored lane cores: home resolution, launch specs, health probe
src/mcp_registry.rs     the MCP server registry: mcp.json, the scan over foreign harness configs, import, health, the fleet mcp cards
src/lane_client.rs      the lane-kind candidate's boundary: a chat request in, blend1's grounded answer out
src/session.rs          JSONL session log: append, replay, list, resume
src/home.rs             FLEET_HOME / ~/.fleet, sessions dir, .env fallback (the whole credential chain)
src/doctor.rs           the startup checks
src/net.rs              loopback probe used by doctor
src/route.rs            multi-provider routing: roster, sys1-judged selection, failover, decision rows
src/route_cache.rs      the route-decision cache: one recorded judgment per task, keyed to the judging lane
src/calib.rs            the calibration stream: devdec-vocabulary rows, bounded writer, demotion reader, `fleet calib`
src/memory.rs           the git-tracked fact substrate: six-channel recall, assembly, the recall-count feedback loop, the digest-keyed recall index
src/recall_graph.rs     one bounded hop over the frontmatter's own edges (superseded-by, missing)
src/filelock.rs         the shared per-file flock behind every read-modify-write
src/session_recall.rs   the session archive as synthetic facts through the same ranker
src/skills.rs           skills: SKILL.md loading, the trigger-shaped prompt lines, the 60-skill cap, the owner's retire list, progressive disclosure via `use_skill`, the self-authoring drafter, `fleet skills`
src/nomenclature/       the canonical identifier vocabulary and the `convention` lookup
src/subagents.rs        delegation: the child agent, its scope, the depth cap
src/roles.rs            the named specializations a delegation runs under
src/delegate.rs         the delegation judgment beat: `fleet delegate` grades a day's delegations through sys1
src/graph.rs            the dependency graph: scores, gates, critical path, fact-based verification
src/cron.rs             scheduled runs: the job store, the five-field parser, catch-up, `fleet cron`
src/judge.rs            the routing judgment beat: `fleet judge` grades a day's decisions through sys1
src/eval.rs             the eval corpus: 20 fixed scenarios, graded headless against the mock
src/improve.rs          self-maintenance: gated proposals, `fleet improve apply/undo`
src/explain.rs          the explanation beat: `fleet explain` over blend1, eval-only and batch-only
src/settings.rs         one writer over config.json / routing.json / fleet.env; the CLI and TUI settings surfaces
src/install.rs          `fleet install`: the home layout, config scaffold, lane venvs, doctor gate
src/tui/                the ratatui surface: app.rs state, events.rs keys, ui.rs render, mod.rs loop
src/bin/mock-server.rs  the scripted mock API the integration tests drive
src/bin/mock-mcp.rs     the scripted fake MCP stdio server the integration tests drive
memories/               the repo's own fact corpus: one markdown file per fact; the recall-count bump and the lifecycle stamps are its runtime edits
services/               the five vendored Python lane cores with VENDOR.json; services/README.md documents the launch
scripts/cache-sweep.sh  the recap step's sweep: stale target/debug artifacts pruned by age (7d), a 15 GiB tiered backstop, target/release never touched
scripts/daily-maintenance.sh  the machine's daily upkeep (LaunchAgent local.fleet-maintenance, 09:20): the sweep, tmp/ entries older than 14d retired, the memories/ churn committed under chore(memories), untracked top-level orphans NAMED in ~/Library/Logs/fleet-maintenance.log and never deleted
tests/integration.rs    end-to-end tests: parser, client, tool loop, log, real sem1 lane via MCP
docs/plans/             plan-as-contract, YYYY-MM-DD-<slug>.md (slice records + dashboards record inherited from the fork)
docs/recaps/            session recaps, YYYY-MM-DD-<slug>.md
docs/known-issues.md    the ledger: every error/warning encountered, with evidence
docs/architecture/      deep-dive-2026-10-15.md (current state), overview.md (slice-1 snapshot), ecosystem.md (the repos Fleet absorbs), surfaces.md (the what and why of all eleven destinations), lanes.md (why five specialized services stand between the owner and the LLM), tool-surface.md (the model's tools, the gap, and the 2026 build order)
docs/images/            the surface screenshots the README and the functional spec render
README.md                the product's front door: the arc, what ships, what is next (product-manager register)
TECHNICAL-DOCUMENTATION.md   the system of record: interface contracts, environments, runbooks, verification, risk, sequencing (technical-program-manager register)
FUNCTIONAL-SPECIFICATIONS.md   what the product does, surface by surface (product-manager register)
docs/features/          feature docs as they land
docs/harness-orientation.md  the harness's own orientation: the session cycle, the knowledge seams, the house rules
CLAUDE.local.md         gitignored: env var names, machine paths, local quirks (no values)
```

## Common commands

`cargo` is not on PATH on this machine — see the PATH line in `CLAUDE.local.md`.

```sh
cargo test                          # the gate's core: 1012 passed, 2 ignored at the last increment
cargo build --release               # binary at target/release/fleet
cargo run --bin fleet -- doctor     # credentials, home, API, shell; masks the key to its last 4 chars
cargo run --bin fleet -- run "<task>" --yes
cargo run --bin fleet -- sessions   # the archive as cards; --recall ranks it, <id> opens one log
cargo run --bin fleet -- compact    # fold a session's stored context on demand
cargo run --bin fleet -- lanes      # probe the six vendored lanes, health cards
cargo run --bin fleet -- mcp        # the MCP server registry: cards; scan/import/enable/disable/remove/health
cargo run --bin fleet -- providers  # the routing roster in chain order, per-provider state
cargo run --bin fleet -- calib      # the calibration stream: today's rows by op, the writer's counters
cargo run --bin fleet -- settings   # every configurable surface; `set <key> <value>` writes one
cargo run --bin fleet -- memories   # the fact corpus as cards; --recall ranks for an instruction
cargo run --bin fleet -- remember   # write one fact (the CLI form of the remember tool)
cargo run --bin fleet -- skills     # the loaded skills; --proposed lists drafts awaiting the gate
cargo run --bin fleet -- cron       # the scheduled jobs; add/enable/disable/remove by name
cargo run --bin fleet -- improve    # the self-maintenance proposals; apply runs the gate
cargo run --bin fleet -- eval       # the fixed graded corpus, headless; --last reads the last run
cargo run --bin fleet -- explain    # explain one recorded verdict through blend1 (eval-only)
cargo run --bin fleet -- judge      # grade a day's routing decisions through sys1
cargo run --bin fleet -- delegate   # grade a day's delegations against their outcome evidence
cargo run --bin fleet -- roles      # the named delegation specializations
cargo run --bin fleet -- convention # what this project already calls a concept
cargo run --bin fleet -- graph      # the declared dependency graph as cards
cargo run --bin fleet -- install    # the home layout and lane venvs
DEEPSEEK_API_KEY=sk-... cargo test --test integration -- --ignored   # live e2e; #[ignore]d by default, runs on demand
scripts/cache-sweep.sh [--all] [--dry-run]   # the recap step's sweep: stale debug artifacts pruned by age, 15 GiB backstop
```

## Branch and environment topology

Single branch `master` → `origin/master`. No staging/production split: solo tool, no hosted surface. If a deploy surface ever appears, add `develop` and a topology table here.

## Housekeeping protocol

- Hard rule about what to do → this file. Env var name or machine path → `CLAUDE.local.md`. What happened in a session → `docs/recaps/`. What to build → `docs/plans/`. Architecture or ecosystem detail → `docs/architecture/`.
- This file stays ≤300 lines. When it grows, extract to a doc and leave a pointer — never append. One home per fact; no duplicated pointers. No "Updated YYYY-MM-DD" markers; git history owns dates.
- CLAUDE.md's state bullets go stale between sessions; cross-check them against the latest recap, the known-issues ledger, and `git log` before quoting them.

## Session protocol

warmup (this file → latest recap → active plans) → plan (`docs/plans/`) → build (run the gate locally) → recap (`docs/recaps/` — the ledger must be well-formed before the document is generated: `python3 scripts/ki_ledger.py docs/known-issues.md` reports zero problems, since a malformed row shifts its cells and misreports the open count; then `scripts/cache-sweep.sh`, the recap step's closing sweep, because cargo never prunes: KI-279, 159 GiB of dead test binaries — automated daily by `scripts/daily-maintenance.sh` (the LaunchAgent), so a missed recap no longer means an unbounded target) → commit with explicit paths.

## Ecosystem at a glance

One line each; verified states and absorption notes in [docs/architecture/ecosystem.md](docs/architecture/ecosystem.md).

| Repo | Role | State (git-level, verified 2026-10-13) |
|---|---|---|
| sys1 | judgment lane; decision models; MCP `sys1_classify` | `feature/plan-surface`, 4 uncommitted files |
| sdm1 | tabular lane; foundation models; MCP classify/health | clean on `main` |
| sem1 | embeddings lane; MCP embed/probe/index/search | clean on `main`; no CLAUDE.md |
| gen1 | media lane; MCP `gen1_imagine` | clean on `master` |
| dev-decisions | calibration/verification layer; JSONL ledger | clean on `main` + 3 feature branches |
| agnostic-router-kit | model router, workflow plane, `/dashboard` — the precursor to Fleet; a separate app, not a Fleet dependency | `master`, 4 untracked docs dirs; full methodology present |
| second-brain | localhost control-tower dashboard | clean on `main`; last commit 2026-10-09 |
| deepseek-harness (fork) | the TS "fleet profile": `packages/fleet/dsh-*` wires all of the above into the harness | parent repo; slice-1 and dashboards records originated here |
