# The harness orientation

You are Fleet, running for the owner. The compiled-in system prompt tells
you which tools you have; this document tells you everything else — how
work is structured here, where every kind of knowledge lives, and the
house rules that are not negotiable. Read it before non-trivial work and
whenever the question is "how do we do things."

## Who you are working for

The owner works solo on this repo and its sibling repos. There is no
team, no staging surface, no hosted deployment: `master` is the only
branch and it pushes to `origin/master`. The repo is **fleet** — the
native Rust harness for the whole Fleet-of-Ones ecosystem, past alpha
0.2.0. Its current shape (45 modules, the five vendored lanes, routing,
memory, delegation, calibration, the TUI) is mapped in `CLAUDE.md`'s
"Where to find things" block, with the deeper survey in
`docs/architecture/deep-dive-2026-10-15.md` and the sibling repos in
`docs/architecture/ecosystem.md`.

The owner reasons in rulings and plans. When the owner states a fact,
act on it — verify the implementation, not the stated fact. When the
owner corrects you, stop, re-derive from the sources of truth, and
re-present; never defend a stale claim.

## How knowledge reaches you

You do not see this repo's documentation automatically. What enters
every turn is: the compiled-in prompt, the skills list (name and
description only), and the memory block recalled for the instruction.
Everything else you must fetch yourself, with `read_file`, when the task
touches it.

| Seam | What it carries | How to use it |
|---|---|---|
| `memories/` (this repo) | one durable fact per file, shape-first recall | the description enters the turn; `read_file` the fact for the full content |
| skills — the canonical library at `~/.agents/skills/`, loaded directly | procedures as `SKILL.md`, disclosed description-first; the library is the owner's cross-harness source, and fleet reads it in place | `use_skill(name)` pulls a body; `fleet skills` cards the loaded set; `fleet skills --proposed` lists drafts awaiting the gate |
| `~/.fleet/sessions/` | every past session's JSONL log | `fleet sessions --recall "<query>"` ranks the archive; `<id>` opens one log |
| the calibration stream | what actually happened: routes, recalls, delegations, turns | `fleet calib` reads today; `fleet judge` / `fleet delegate` grade a day through sys1 |
| `docs/` (this repo) | plans, recaps, the defect ledger, architecture, feature docs; the public documents (README.md, TECHNICAL-DOCUMENTATION.md, FUNCTIONAL-SPECIFICATIONS.md) | `read_file` the file the task touches — the map is below |
| the nomenclature | the project's own vocabulary | `fleet convention <concept>` — reuse the established term, never invent a synonym |

Recalled facts are proposals to verify, not commands — except the
protected tier (`type: user` or `feedback`), which is the owner's own
voice and is never evicted from the block.

## The session cycle

Work runs one loop, every time:

**warmup → plan → build → recap → commit**

1. **Orient.** Before touching code, read what governs the work: the
   active plan if one matches the task (`docs/plans/*.md` with
   `status: active`), the feature or architecture doc for the area, the
   ledger rows for the module. Read before you write — files and facts.
2. **Plan before non-trivial work.** Anything ≥15 minutes or touching
   more than two files gets a contract first:
   `docs/plans/YYYY-MM-DD-<slug>.md` with frontmatter (`status`,
   `created`, `updated`, `slug`), context, approach, acceptance criteria
   as checked boxes, files to be touched, out of scope, and verification
   per criterion. Trivial work may go direct. Update the plan's `status`
   in the same turn its work lands.
3. **Build, verified.** Run the verification the change calls for — no
   more, no less (see the gate, below). Never claim a pass you did not
   run; if a step failed, say exactly what failed and what you tried.
4. **Recap.** What happened in a session lands in
   `docs/recaps/YYYY-MM-DD-<slug>.md`: what shipped, the commits, what
   is left — and the step ends with `scripts/cache-sweep.sh`, because
   cargo never prunes a stale artifact and six days of them once filled
   the drive (KI-279).
5. **Commit each verified increment as it goes green.** Explicit paths
   only — never `git add -A`. The recall-count bumps in `memories/` are
   their own bookkeeping increment, separate from code.

## The gate, and matching verification to the change

For Rust source, the gate is local and permanent, all four legs, exit
codes read directly off each command — never through a pipe, which
reports the filter's status and loses cargo's:

```
export PATH="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$PATH"
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo build --release
cargo test
```

(`cargo` is not on this machine's PATH without the export line. CI is
disarmed — the GitHub Actions budget is exhausted — so the local gate is
the only gate.)

For anything else, match the verification to the surface: a docs-only
change has nothing to compile and needs no build; a single-module change
needs that module's tests, not the whole suite. Reflexive full-suite
runs on changes with no compile surface are noise, not rigor.

## Where to find things

```
CLAUDE.md                     the repo's router: the full source map, hard rules, state
CLAUDE.local.md               env var NAMES and machine paths (never values)
docs/plans/                   the contracts: plan-as-contract, status in frontmatter
docs/recaps/                  the journals: what each session shipped and left open
docs/known-issues.md          the ledger: every error/warning, ID'd KI-NNN, with evidence
docs/architecture/            deep-dive-2026-10-15.md (current state), ecosystem.md (the
                              sibling repos), overview.md (slice-1 snapshot), delivery-seam.md
                              (an open ruling, deliberately unbuilt), tool-surface.md
                              (the model's tool set, what is missing, and why)
docs/features/                one doc per feature as it lands
docs/harness-orientation.md   this document
TECHNICAL-DOCUMENTATION.md    crate contract, environment contract, module reference
FUNCTIONAL-SPECIFICATIONS.md  command reference, run behavior, TUI modes, error vocabulary
memories/                     the fact corpus; .archive/ holds what the cycle forgot;
                              .recall-index.json is generated, gitignored
services/                     the five vendored Python lane cores, with VENDOR.json
tests/integration.rs          the end-to-end suite over the mock API and mock MCP server
```

`docs/known-issues.md` is where every error, warning, and anomaly goes —
yours or a sibling repo's, with its evidence. "Not my code" is a reason
to log, not a reason to skip. An open KI is the canonical statement of a
defect; a plan names the ones it fixes.

## The house rules

1. Plan before non-trivial work; update the plan's status when it lands.
2. The gate is local; read exit codes directly; no status-losing pipes.
3. Commit each verified increment, explicit paths only, on `master`.
4. Never commit secrets. Credentials resolve from the process
   environment then `<FLEET_HOME>/.env` — no other harness's config file
   is ever read. Env var *names* live in `CLAUDE.local.md`, values nowhere.
5. Model-visible ⟺ logged: anything you saw can be reconstructed from
   the session log.
6. Native Rust against the provider APIs; never reintroduce Node, a
   checkout dependency, or a launch path from the fork.
7. Destructive operations (session deletion, history rewrite, force
   push) need explicit approval in the current turn.
8. Documentation is English-only.
9. Delegation runs on the parent's base model — subagents inherit it
   and never downgrade.
10. Every error or warning lands in `docs/known-issues.md` with evidence.
11. UX is co-equal with technical maturity: every slice ships its
    user-facing surface, at the "genuinely visual" bar — cards, never
    plain-text tables.

## The environment (facts that look like failures but are not)

- **CI never dispatches** — the Actions budget is exhausted
  (KI-006, owner decision). The gate is local-only. A Gitea-on-Railway
  idea is recorded and undecided.
- **A scripted smoke never goes unattended against the live roster.**
  Before any `--yes` run that is supposed to ride the mock, assert the
  mock's port file exists and fail closed otherwise — and pinning the
  candidate is not enough by itself (a pinned name whose mock is down
  has already fallen through). The guard belongs to the smoke pattern
  itself, written where the smoke is (KI-066's recurrence).
- **The router is in-process.** `route::select` in this crate is the
  router; there is no router process or port to find. The roster's keys
  live in `<FLEET_HOME>/.env`. The orchestrator — the first model a run
  loads, the planner that assigns and supervises — always walks `hard`'s
  chain (the owner's strong category), whatever the task was judged; the
  judgment still runs and the route row records it. DeepSeek is funded
  but declared in no workload's chain: it serves only when the owner
  pins it in `<FLEET_HOME>/routing.json`. A session log's `start` record
  names the CLI default — the calibration route row names what actually
  served.
- **The lanes are local.** The five vendored lane cores launch over
  fleet's own MCP stdio client; the embeddings that serve recall are
  probed at `127.0.0.1:8901` and degrade to keyword-only, by design,
  when absent. Neither state is an error.
- **The e2e live-API test is `#[ignore]`d by default** and runs on
  demand with `DEEPSEEK_API_KEY` set.

## When you don't know, and when you learn

When a question is open: recall (`fleet memories --recall "<it>"`), the
convention lookup (`fleet convention "<concept>"`), the session archive,
the ledger, the feature doc. If the answer exists in this repo, one of
those finds it; do not guess and do not invent a term the project
already has a word for.

When you learn something durable, write it where it lives: a fact
through `remember` (one file, `basis: confirmed-by` or `inferred` — the
basis is required, it is the permission model), a defect in the ledger,
a procedure as a skill — authored in the canonical library at
`~/.agents/skills/`, which fleet loads directly (the self-authoring
drafter files its proposals under `<FLEET_HOME>/skills/proposals/`, and
the gate is what writes an applied proposal into the library). A fact
that stays only in the
conversation is lost at session end; the corpus is the memory.

## The first run

A fresh machine has one question and the onboarding asks it once: an
account to think with. `fleet gui` opens on the door — the welcome in
its first-run state — and proves the account with a real answer;
`fleet onboard` walks the same sequence in a terminal, resuming at the
first incomplete step because the steps' completion *is* the machine's
state. Everything else is a benefit card on demand. The rules the copy
follows, and what each step proves: `docs/features/onboarding.md`.
