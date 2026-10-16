# Fleet

A native Rust agent for the DeepSeek API — and the intended home of a self-contained Rust harness for the whole Fleet-of-Ones ecosystem. Repo: `git@github.com:adelvillar1/fleet.git` (private, MIT). Status: alpha 0.2.0 — well past the agent-core slice: routing, memory recall, delegation, scheduling, self-calibration, and the TUI are all in-repo (see Today's state).

## What Fleet is

Slice 1 shipped: one static binary with its own `ureq`-based DeepSeek chat-completions client, a closed four-tool set (`bash`, `read_file`, `write_file`, `list_dir`), terminal tool approvals, append-only JSONL session logs with resume, and `doctor`. No Node runtime, no harness checkout, no dsh launch path.

The owner's intent (2026-10-13): Fleet is the self-contained Rust successor to the DeepSeek Harness fork's "fleet profile". It is meant to absorb the entire ecosystem, not just the agent core:

- the sys1 house lanes — sys1 (judgment), sdm1 (tabular), sem1 (embeddings), gen1 (media), plus the composed blend1 lane — vendored as Python service cores under `services/`, driven through fleet's own MCP client
- dev-decisions — the verification and calibration layer: absorbed as the calibration stream's row vocabulary and log root (`src/calib.rs`)
- agnostic-router-kit — model routing: the roster and the workload preference table are ported (`src/route.rs`); the kit's operator API and `/dashboard` are read read-only through the router board
- second-brain — the localhost control tower: its derived state files are read read-only through the second-brain board (never exposed beyond the machine)
- the dashboards — router board, second-brain board, and tower over the board seam: rendered read-only in the TUI's board pane (`src/boards.rs`) for fleet's own surface, replacing the fork's TypeScript plugins here

Verified inventory and per-repo state: [docs/architecture/ecosystem.md](docs/architecture/ecosystem.md). Current crate architecture: [docs/architecture/deep-dive-2026-10-15.md](docs/architecture/deep-dive-2026-10-15.md) (the current-state survey; `overview.md` is the slice-1 snapshot with a banner).

## Today's state (2026-10-16)

- In-repo and green: `cargo test` → 1013 passed, 2 ignored at the last increment's commit; the gate runs whole locally — `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings`, `cargo build --release`, `cargo test`, all four exit 0 (rustfmt and clippy were installed 2026-10-15, closing KI-007, and the first full clippy pass landed clean). The crate is 45 modules (~70k lines): the agent core, the five vendored lanes over the hand-rolled MCP client, multi-provider routing with the sys1 judgment and failover, the git-tracked memory substrate with six-channel recall, session-archive recall, skills, delegation to depth 2 with the dependency graph and cron, the calibration stream with its judge/delegate/explain beats and the eval corpus, the settings writer, and the ratatui TUI. The 2026-10-15 sessions landed the deep dive, its six-phase findings plan, and the dogfooding viability fixes (`docs/plans/2026-10-15-dogfooding-viability-fixes.md`: the bash wall-clock bound, the approval gate's label-blind class checks, the routing record's name, the consolidation fold's honesty, the torn-tail tolerance).
- Environmental blockers, none of them code:
  - **CI is red by budget.** GitHub Actions minutes are exhausted; jobs never dispatch (job record: `runner_id: 0`, zero steps, ~3s). Until the budget resets, the gate must be run locally on every change.
  - **DeepSeek is funded, and it is the chain's last resort** — owner-confirmed 2026-10-15 (the earlier 402 was transient; KI-008 records it): the account has balance, and the roster walks the other candidates first, so DeepSeek answers when no other viable provider remains (down or out of quota). The live-API e2e stays `#[ignore]`d by default and can run on demand with the key set.
- Ecosystem repos (sys1, sdm1, sem1, gen1, dev-decisions, agnostic-router-kit, second-brain) live outside this repo; their git-level states are verified in the ecosystem doc. The fork stays the spec/integration lab, never a shipping competitor.

## Hard rules

1. **Plan before non-trivial work.** Draft `docs/plans/YYYY-MM-DD-<slug>.md` with acceptance criteria before any feature ≥15 min or >2 files; update its `status` in the same turn the work lands. Trivial work may go direct.
2. **The gate is local and permanent:** `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings`, `cargo build --release`, `cargo test`. Run it before every push and **read the exit code — never through a status-losing pipe** (KI-014). CI is disarmed by owner decision (no Actions funds); the workflow is manual-dispatch only until the owner re-arms it.
3. **Commit each verified increment as it goes green.** Explicit paths only — never `git add -A`. `master` is the only branch (solo tool, no hosted surface).
4. **Never commit secrets.** Credentials resolve at runtime: `DEEPSEEK_API_KEY` env → `<FLEET_HOME>/.env` → the DeepSeek Harness credential store's `refs` block (read-only). This repo stores no keys; env var names live in `CLAUDE.local.md`.
5. **Model-visible ⟺ logged** (inherited from the harness): anything the model sees must be reconstructable from the session log. The system prompt and tool declarations are compiled-in constants, reconstructed per request, never logged.
6. **Native port, not a fork.** New work is Rust against the DeepSeek API; never reintroduce Node, a checkout dependency, or a dsh launch path (plan criterion C5).
7. **Destructive operations** (session deletion, history rewrite, force push) require explicit approval in the current turn.
8. **Documentation is English-only** — no Chinese or other non-English docs in this repo (a deliberate departure from the fork's bilingual convention).
9. **Delegation runs on the parent's base model** (step-5-preview class) — subagents inherit it and never downgrade; build-time delegation follows the same rule.
10. **Every error or warning is logged, not ignored** — anything encountered, in this crate or any sibling repo, lands in `docs/known-issues.md` with its evidence. "Not my code" is a reason to log, not a reason to skip.
11. **UX is co-equal with technical maturity** — every slice ships its user-facing surface with the feature; UX is a per-slice deliverable at the "genuinely visual, never plain-text tables" bar, never a polish finale.

## Where to find things

```
src/main.rs             dispatch, credential resolution, run/chat/sessions/lanes/doctor wiring
src/cli.rs              argument parsing, help text, usage errors
src/api.rs              DeepSeek chat-completions client (ureq), streaming, typed errors
src/boards.rs           the three read-only boards: the router kit's operator API, the second-brain's derived state files, git facts per project
src/agent.rs            the tool loop, approvals, turn cap, progress reporting
src/tools.rs            bash, read_file, write_file, list_dir; output capping; the tool-declaration vocabulary
src/approvals.rs        the three-mode approval gate: ask, yes, yes-guarded, and the always-ask class
src/compaction.rs       context-window management: summarize-and-continue under a token budget
src/mcp.rs              the hand-rolled MCP stdio client (spawn, initialize, tools/list, tools/call, teardown)
src/lanes.rs            the five vendored lane cores: home resolution, launch specs, health probe
src/lane_client.rs      the lane-kind candidate's boundary: a chat request in, blend1's grounded answer out
src/session.rs          JSONL session log: append, replay, list, resume
src/home.rs             FLEET_HOME / ~/.fleet, sessions dir, .env fallback, harness refs
src/doctor.rs           the startup checks
src/net.rs              loopback probe used by doctor
src/route.rs            multi-provider routing: roster, sys1-judged selection, failover, decision rows
src/route_cache.rs      the route-decision cache: one recorded judgment per task, keyed to the judging lane
src/calib.rs            the calibration stream: devdec-vocabulary rows, bounded writer, demotion reader, `fleet calib`
src/memory.rs           the git-tracked fact substrate: six-channel recall, assembly, the recall-count feedback loop
src/session_recall.rs   the session archive as synthetic facts through the same ranker
src/skills.rs           skills: SKILL.md loading, progressive disclosure via `use_skill`, the self-authoring drafter, usage-scan curation, `fleet skills`
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
memories/               the repo's own fact corpus: one markdown file per fact; the recall-count bump is its only runtime edit
services/               the five vendored Python lane cores with VENDOR.json; services/README.md documents the launch
tests/integration.rs    end-to-end tests: parser, client, tool loop, log, real sem1 lane via MCP
docs/plans/             plan-as-contract, YYYY-MM-DD-<slug>.md (slice records + dashboards record inherited from the fork)
docs/recaps/            session recaps, YYYY-MM-DD-<slug>.md
docs/known-issues.md    the ledger: every error/warning encountered, with evidence
docs/architecture/      deep-dive-2026-10-15.md (current state), overview.md (slice-1 snapshot), ecosystem.md (the repos Fleet absorbs)
docs/features/          feature docs as they land
CLAUDE.local.md         gitignored: env var names, machine paths, local quirks (no values)
```

## Common commands

`cargo` is not on PATH on this machine — see the PATH line in `CLAUDE.local.md`.

```sh
cargo test                          # the gate's core: 1013 passed, 2 ignored at the last increment
cargo build --release               # binary at target/release/fleet
cargo run --bin fleet -- doctor     # credentials, home, API, shell; masks the key to its last 4 chars
cargo run --bin fleet -- run "<task>" --yes
cargo run --bin fleet -- sessions   # the archive as cards; --recall ranks it, <id> opens one log
cargo run --bin fleet -- compact    # fold a session's stored context on demand
cargo run --bin fleet -- lanes      # probe the five vendored lanes, health cards
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
```

## Branch and environment topology

Single branch `master` → `origin/master`. No staging/production split: solo tool, no hosted surface. If a deploy surface ever appears, add `develop` and a topology table here.

## Housekeeping protocol

- Hard rule about what to do → this file. Env var name or machine path → `CLAUDE.local.md`. What happened in a session → `docs/recaps/`. What to build → `docs/plans/`. Architecture or ecosystem detail → `docs/architecture/`.
- This file stays ≤300 lines. When it grows, extract to a doc and leave a pointer — never append. One home per fact; no duplicated pointers. No "Updated YYYY-MM-DD" markers; git history owns dates.
- CLAUDE.md's state bullets go stale between sessions; cross-check them against the latest recap, the known-issues ledger, and `git log` before quoting them.

## Session protocol

warmup (this file → latest recap → active plans) → plan (`docs/plans/`) → build (run the gate locally) → recap (`docs/recaps/`) → commit with explicit paths.

## Ecosystem at a glance

One line each; verified states and absorption notes in [docs/architecture/ecosystem.md](docs/architecture/ecosystem.md).

| Repo | Role | State (git-level, verified 2026-10-13) |
|---|---|---|
| sys1 | judgment lane; decision models; MCP `sys1_classify` | `feature/plan-surface`, 4 uncommitted files |
| sdm1 | tabular lane; foundation models; MCP classify/health | clean on `main` |
| sem1 | embeddings lane; MCP embed/probe/index/search | clean on `main`; no CLAUDE.md |
| gen1 | media lane; MCP `gen1_imagine` | clean on `master` |
| dev-decisions | calibration/verification layer; JSONL ledger | clean on `main` + 3 feature branches |
| agnostic-router-kit | model router, workflow plane, `/dashboard` | `master`, 4 untracked docs dirs; full methodology present |
| second-brain | localhost control-tower dashboard | clean on `main`; last commit 2026-10-09 |
| deepseek-harness (fork) | the TS "fleet profile": `packages/fleet/dsh-*` wires all of the above into the harness | parent repo; slice-1 and dashboards records originated here |
