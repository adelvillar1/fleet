# Known issues

The ledger for every error, warning, and defect encountered in Fleet's orbit — this crate, the vendored cores, the sibling repos, the toolchain, the environment. **Nothing is ignored for being "not my code," pre-existing, or someone else's repo** (owner directive, 2026-10-13). An item is worked or explicitly deferred by the owner, and it closes only with evidence.

Severity: `blocker` (stops the build or daily use) · `defect` (wrong behavior, needs a fix) · `gap` (missing something expected) · `environmental` (outside the code, owner action) · `risk` (no defect yet, exposure exists).

| ID | Severity | Issue | Evidence | Status | Next action |
|---|---|---|---|---|---|
| KI-001 | defect | agnostic-router-kit CI is red on **real test failures**, not the budget condition: runs 37863487221 (55s), 37857919072 (1m9s), 37696108358 (50s) dispatched with a runner and 7 steps and failed on steps, Oct 7–9 — predating the Actions-budget exhaustion. A prior recap wrongly conflated this with Fleet's never-dispatched condition | `gh api repos/adelvillar1/agnostic-router-kit/actions/runs/<id>/jobs`, 2026-10-13 | open | reproduce locally in that repo, read the failing step, fix or file there |
| KI-002 | defect | The vendored sys1 and gen1 MCP cores answer the `notifications/initialized` notification with a JSON-RPC `-32601` error instead of ignoring it (MCP 2024-11-05); sdm1 and sem1 fixed this after sys1/gen1 were vendored | stdio probe of `services/<svc>/service/mcp_server.py` in the fork, 2026-10-13 | open | fix at the vendoring boundary (slice 4) or upstream in sys1/gen1, then re-vendor |
| KI-003 | gap | sys1 has 4 uncommitted files on branch `feature/plan-surface` (`docs/plans/2026-10-04-canonical-provider-env.md`, `service/mcp_server.py`, `src/sys1/api.py`, `src/sys1/tasks.py`) — unlanded work that exists in no commit and is vendored nowhere | `git status` in `~/Projects/sys1`, 2026-10-13 | open | owner decision: land on a branch, or discard |
| KI-004 | risk | gen1 has **no git remote** (local `master` only) — no backup, no push target | `git branch -a` in `~/Projects/gen1`, 2026-10-13 | open | owner decision: add a remote and push |
| KI-005 | gap | sem1 is the only ecosystem repo with no `CLAUDE.md` — no methodology layer | `ls ~/Projects/sem1`, 2026-10-13 | open | scaffold the slim layer when sem1 is next touched |
| KI-006 | environmental | Fleet CI is red by budget: GitHub Actions minutes exhausted, jobs never dispatch (`runner_id: 0`, zero steps, ~3s) — user-confirmed | run 38390172300 job record + user statement, 2026-10-13 | open | owner: budget reset/top-up, or make the repo public |
| KI-007 | environmental | Local toolchain cannot run the first two CI steps: rustfmt and clippy are absent (rustup `profile = "minimal"`) and there is no rustup shim binary on the machine to add them | `cargo fmt`/`cargo clippy` → "no such command"; `~/.rustup/settings.toml`, 2026-10-13 | open | restore a rustup shim (brew or rustup-init), then `rustup component add rustfmt clippy` |
| KI-008 | environmental | DeepSeek account balance is 0 — the API answers 402 Insufficient Balance, blocking the live-API e2e (slice-1 plan criterion C3) and any real `fleet run` | plan record; `fleet doctor` credentials ok, 2026-10-13 | open | owner: top up the account |
| KI-009 | defect | The previous session's standalone-repo recap claimed agnostic-router-kit showed the same never-dispatched CI condition as Fleet — wrong (see KI-001) | re-probe, 2026-10-13 | **closed — corrected** | the correction is recorded in the fleet repo's recaps and ecosystem doc so the claim does not propagate |

## Rules for this ledger

- Log on encounter: an error or warning found during any Fleet work lands here the moment it is seen, with the command or probe that produced it as evidence — before any fix, and even when the fix is immediate.
- No silent dismissals: "pre-existing," "not our repo," "unrelated to the task," and "CI is red anyway" are not closures. Deferral is explicit and owner-approved.
- Close with evidence: the fixing commit, the green run, or the owner's decision is linked in the row.
- The ledger feeds the plan: an item that blocks a slice is named in that slice; slice 13 automates the evidence gathering, but this file stays the system of record.
