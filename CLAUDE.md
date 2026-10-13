# Fleet

A native Rust agent for the DeepSeek API — and the intended home of a self-contained Rust harness for the whole Fleet-of-Ones ecosystem. Repo: `git@github.com:adelvillar1/fleet.git` (private, MIT). Status: alpha 0.2.0, slice 1 (agent core) only.

## What Fleet is

Slice 1 shipped: one static binary with its own `ureq`-based DeepSeek chat-completions client, a closed four-tool set (`bash`, `read_file`, `write_file`, `list_dir`), terminal tool approvals, append-only JSONL session logs with resume, and `doctor`. No Node runtime, no harness checkout, no dsh launch path.

The owner's intent (2026-10-13): Fleet is the self-contained Rust successor to the DeepSeek Harness fork's "fleet profile". It is meant to absorb the entire ecosystem, not just the agent core:

- the sys1 house lanes — sys1 (judgment), sdm1 (tabular), sem1 (embeddings), gen1 (media) — already parts of the fork as vendored service cores (boot-verified 2026-10-13), not yet in this Rust crate
- dev-decisions — the verification and calibration layer
- agnostic-router-kit — model routing, the workflow plane, and its `/dashboard`
- second-brain — the localhost control tower (never exposed beyond the machine)
- the dashboards — router board, second-brain board, and tower over the board seam (today TypeScript plugins in the fork)

None of that is in this repository yet. Verified inventory and per-repo state: [docs/architecture/ecosystem.md](docs/architecture/ecosystem.md). Current crate architecture: [docs/architecture/overview.md](docs/architecture/overview.md).

## Today's state (2026-10-13)

- Slices 1-6 in-repo and green: `cargo test` → 245 passed, 1 ignored (194 unit + 51 integration), build warning-free. Debug binary smoke-verified through slice 6: `version`, `help`, `doctor`, `sessions`, `run`, `lanes`, `providers`, and `calib` (the calibration stream), with exit codes 0/2/3 as documented.
- Environmental blockers, none of them code:
  - **CI is red by budget.** GitHub Actions minutes are exhausted; jobs never dispatch (job record: `runner_id: 0`, zero steps, ~3s). Until the budget resets, the gate must be run locally on every change.
  - **rustfmt and clippy are not installed** on this machine (rustup `profile = "minimal"`, no rustup shim). `cargo fmt --check` and `cargo clippy -D warnings` cannot run here; match the existing style and say so in the recap when they were not run.
  - **DeepSeek account balance is 0** — the API answers 402 Insufficient Balance, so the live-API e2e (plan criterion C3) and any real `fleet run` fail with the typed 402 mapping until top-up.
- Ecosystem repos (sys1, sdm1, sem1, gen1, dev-decisions, agnostic-router-kit, second-brain) live outside this repo; their git-level states are verified in the ecosystem doc. Runtime health of those services has not been probed. Nothing has been absorbed into Rust yet.

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
src/agent.rs            the tool loop, approvals, turn cap, progress reporting
src/mcp.rs              the hand-rolled MCP stdio client (spawn, initialize, tools/list, tools/call, teardown)
src/lanes.rs            the four vendored lane cores: home resolution, launch specs, health probe
src/tools.rs            bash, read_file, write_file, list_dir; output capping
src/session.rs          JSONL session log: append, replay, list, resume
src/home.rs             FLEET_HOME / ~/.fleet, sessions dir, .env fallback, harness refs
src/doctor.rs           the four startup checks
src/net.rs              loopback probe used by doctor
src/route.rs            multi-provider routing: roster, sys1-judged selection, failover, decision rows
src/calib.rs            the calibration stream: devdec-vocabulary rows, bounded writer, demotion reader, `fleet calib`
src/tui/                the ratatui surface: app.rs state, events.rs keys, ui.rs render, mod.rs loop
src/bin/mock-server.rs  the scripted mock API the integration tests drive
src/bin/mock-mcp.rs     the scripted fake MCP stdio server the integration tests drive
services/               the four vendored Python lane cores with VENDOR.json; services/README.md documents the launch
tests/integration.rs    end-to-end tests: parser, client, tool loop, log, real sem1 lane via MCP
docs/plans/             plan-as-contract, YYYY-MM-DD-<slug>.md (slice records + dashboards record inherited from the fork)
docs/recaps/            session recaps, YYYY-MM-DD-<slug>.md
docs/known-issues.md    the ledger: every error/warning encountered, with evidence
docs/architecture/      overview.md (this crate), ecosystem.md (the repos Fleet will absorb)
docs/features/          feature docs as they land
CLAUDE.local.md         gitignored: env var names, machine paths, local quirks (no values)
```

## Common commands

`cargo` is not on PATH on this machine — see the PATH line in `CLAUDE.local.md`.

```sh
cargo test                          # the gate's core: 245 passed, 1 ignored
cargo build --release               # binary at target/release/fleet
cargo run --bin fleet -- doctor     # credentials, home, API, shell; masks the key to its last 4 chars
cargo run --bin fleet -- run "<task>" --yes
cargo run --bin fleet -- sessions
cargo run --bin fleet -- lanes      # probe the four vendored lanes, health cards
cargo run --bin fleet -- providers  # the routing roster in chain order, per-provider state
cargo run --bin fleet -- calib      # the calibration stream: today's rows by op, the writer's counters
DEEPSEEK_API_KEY=sk-... cargo test --test integration -- --ignored   # live e2e; needs account balance
```

## Branch and environment topology

Single branch `master` → `origin/master`. No staging/production split: solo tool, no hosted surface. If a deploy surface ever appears, add `develop` and a topology table here.

## Housekeeping protocol

- Hard rule about what to do → this file. Env var name or machine path → `CLAUDE.local.md`. What happened in a session → `docs/recaps/`. What to build → `docs/plans/`. Architecture or ecosystem detail → `docs/architecture/`.
- This file stays ≤300 lines. When it grows, extract to a doc and leave a pointer — never append. One home per fact; no duplicated pointers. No "Updated YYYY-MM-DD" markers; git history owns dates.

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
