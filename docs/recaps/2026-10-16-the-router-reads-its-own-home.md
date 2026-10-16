# The router reads its own home

Status: complete
Date: 2026-10-16

Branch: `master`, solo. The session migrated the routing roster off
the zcode-router-kit's configuration and then removed the last leg
that let Fleet read another harness's config — both under the
owner's direction, the second as an explicit ruling: "fleet's router
should never be reading a config file from a different harness."

## The roster migration closes KI-047

The kit's local token now rides Fleet's own environment as
`AGNOSTIC_ROUTER_TOKEN`, sourced from the kit's own `localToken`
during the migration, alongside every other roster key — all of them
landing in `<FLEET_HOME>/.env` with the pre-migration file backed up
at `.env.bak-migration`. Values were never printed; only names and
masked confirmations crossed the session. The pin the owner asked to
drop is cleared through the settings writer, so route selection is a
fresh sys1 judgment per run rather than a configured shortcut.

The proof is live: `fleet providers` shows router-kit `✓ configured`
at the head, and the first un-pinned judged run walked once and
recorded `stepfun served` with a real judgment behind it — which is
also where the owner's "I don't remember having mimo as a preferred
provider" was settled. The owner's memory was right: stepfun heads
every workload in all three kit sources (the live `config.json`, its
`.bak-kit`, and the checkout's `roster.json`), and Fleet's
compiled-in port matches exactly. mimo is fallback-only except for
the kit-internal omni/wide/MoA chains. The kit's two extra local
converters (`codebuddy` at :8787, `opencodex` at :10100) stay out of
the roster deliberately — `fleet settings provider.add:` is the seam
if the owner ever wants them.

## The ruling, and what it actually removed

With every key already in Fleet's own `.env`, the third credential
leg — a fall-through to `~/.dsh/.credentials.yaml`'s `refs` block
carried from slice 1 — had no remaining purpose, and the owner ruled
it out outright. The resolution chain is now exactly the process
environment then `<FLEET_HOME>/.env`, enforced in three places
(`route.rs::resolve_key`, `main.rs::setting`, and
`home.rs::resolve_setting`), with the `harness_credentials_path` and
`harness_refs` helpers deleted along with their unit tests so the
read cannot return through dead code.

A fourth site the plan had not named surfaced during the work:
`src/install.rs`'s credential-chain note called the removed helper
directly. The plan's file list grew by one, and the convergence
section records it.

The ruling is pinned rather than merely applied. One new test writes
a decoy harness credential store, points `DSH_HOME` at it exactly as
the removed leg read it, and asserts the key still does not resolve
while Fleet's own `.env` key does. This is why the plan's "grep
returns nothing in `src/`" check has one intentional exception — the
only remaining hit for `credentials.yaml` in the source is that
test, which names the file it refuses to read.

## The surfaces that named the chain

Every one now tells the truth, corrected in the same turn as the
code: doctor's not-set detail, `fleet --help`'s environment block,
the install report's credentials line, CLAUDE.md's hard rule 4 and
its `home.rs` map line, the README's configuration table and its
paragraph, and CLAUDE.local.md's env table (the `DSH_HOME` row is
gone; the machine-paths line now states Fleet never reads the
store). The architecture and feature docs that described the chain
were corrected too — `docs/features/install.md`'s two sample output
blocks and credentials paragraph, `overview.md`'s request-flow step
1, `ecosystem.md`'s cross-repo facts, the run-path diagram in
`deep-dive-2026-10-15.md`, the `CREDENTIAL_ENV` doc in `install.rs`,
and `Candidate::key_env`'s doc in `route.rs`. Two artifacts were
left as dated history rather than rewritten: KI-010's evidence row
and the 2026-10-12 native-app plan, both of which describe what was
true when they were written.

The ruling also landed in the memory corpus as a durable fact
(`memories/own-home-credentials-ruling.md`), written through
`fleet remember` so the substrate itself carries it, with its
`phrases` carrying the vocabulary the direction channel ranks on.

## The gate

Whole, exit codes read directly at every leg: `cargo fmt --all
--check`, `cargo clippy --all-targets -- -D warnings`, `cargo build
--release`, `cargo test` — all four exit 0. Final count 1012 passed
/ 2 ignored, which is net minus one against the previous 1013: two
harness-leg tests removed, one ruling test added. No test had
pinned the old leg, so nothing else moved. A live `fleet doctor` is
green with credentials resolving from `~/.fleet/.env` while
`DEEPSEEK_API_KEY` is absent from the shell — the first leg is no
longer the only path to a key.

## Commits

| Commit | What |
|---|---|
| `73ae2a0` | KI-047 closes: the kit's token rides Fleet's own env on the migrated roster, and the first un-pinned judged walk served its workload head |
| `7962229` | the plan: fleet reads its own home only |
| `6bf970d` | the removal: the harness credential leg gone everywhere, every doc surface corrected, the ruling pinned |
| `b8530be` | the ruling lands in the memory corpus as a durable fact |
| `4038b00` | CLAUDE.md state refreshed, the count follows the gate |

## CLAUDE.local.md changes

- Environment table: `DEEPSEEK_API_KEY`'s row now reads `env →
  <FLEET_HOME>/.env` only; the `DSH_HOME` row is removed.
- Machine paths: the harness credential store's line now states
  Fleet never reads it, citing the 2026-10-16 ruling.

## What is left

Nothing in the plan's scope — C1 through C6 all graded met in the
plan's convergence section. The boards' read-only seams (the router
kit's operator API, the second-brain state files) were named in the
plan as ruled separately and untouched: they read live state over
surfaces the owner designed as dashboards, not config files. The
second dive's remaining findings stay parked for a later plan, and
the attended dogfooding experiment — work the harness on real
tasks, watch `fleet calib`/`judge`/`delegate` — is unblocked and now
runs on a router that answers to nothing but Fleet's own home.
