---
status: active
created: 2026-10-26
updated: 2026-10-26
slug: mcp-registry-named-absence
---

# The MCP registry's named absence

## Context

One unkeyed server refuses every run. `mcp_registry::enabled_specs`
resolves every enabled entry's env names in one pass and
`spec_from_entry` errors on the first name that resolves to nothing —
so `seed_user_servers` fails, `main.rs` returns
`AgentError::Registry`, and the run dies before its memory load. The
2026-10-25 incident: the MCP scan had imported twelve servers from this
machine's other harnesses (eleven from claude-desktop, two from codex),
`github` among them enabled because the source harness had it enabled,
and its `GITHUB_PERSONAL_ACCESS_TOKEN` resolves to nothing — so every
headless `fleet run` refused while the owner's browser face kept
working. The instance was fixed by disabling the four unkeyed entries
(`github`, `node-code-sandbox-mcp`, `node_repl`, `openweather`); the
class is open, and the next lost token or fresh import refuses every
run again.

The router already owns the right pattern for exactly this shape: an
unkeyed candidate is a named skip per candidate and the walk continues
(`route::admissible`, the `unkeyed` list on `WalkReport`) — a keyless
provider is never a failed route. The registry should read the same
way: a server whose env resolves to nothing costs the run that
server's tools, named, and never the run itself.

## Approach

**Resolve per entry, name what could not resolve.** C0 is this
increment's deliverable. `enabled_specs` returns
`Result<(HashMap<String, ServerSpec>, Vec<(String, String)>), String>`:
the specs it could build, plus the absences as `(server, reason)`
pairs. The outer `Err` stays for what deserves to stay fatal — a
registry file that cannot be read or parsed, which is the owner's
config being broken, not a missing key. `spec_from_entry` keeps its
per-entry `Result`; the partition is `enabled_specs`' own.

**The seeding carries the absence out.** C1 is this increment's
deliverable. `AgentOptions::seed_user_servers` stops `?`-ing the
resolution away and hands the absences to its caller, which prints one
line each at startup — the same moment and voice as the
`fleet: skills: skipping …` lines the run already prints: `fleet:
server `github`: env `GITHUB_PERSONAL_ACCESS_TOKEN` resolves to
nothing — its tools are not declared this run`.

**Every caller keeps its honesty, loses only the fatal.** C2 is this
increment's deliverable. `cron.rs`'s catch-up arm already prints a
named line on the error path and keeps its per-job refusal when a
job's `--tool` names an absent server; with the change its `Ok` arm
walks the absences and prints the same line. `bridge.rs`'s lane-call
path stops mapping the registry refusal into a failed call: the specs
it needs resolve, or the name falls through to `lanes::spec_for`
exactly as an unknown name does today. `fleet mcp health` and the
doctor card already report per-server state, so the surface story
needs nothing new.

**The regression pin.** C3 is this increment's deliverable: the test
that would have caught the incident — a run over a registry with one
unkeyed enabled server seeds, runs, and records its absence by name —
plus the ledger row (KI-319) filed in the same commit as the fix, with
the github incident as its evidence.

## Acceptance criteria

- [ ] **C0 — the contract.** `enabled_specs` resolves per entry and
  returns the specs it could build beside the named absences; an entry
  whose env resolves to nothing is skipped WITH its reason while its
  peers still resolve; a registry file that cannot be read or parsed
  still fails loud. Pinned by unit tests over a registry with one
  unkeyed entry and over an unreadable one.
- [ ] **C1 — the seeding tolerates it.** `seed_user_servers` over a
  registry with one unkeyed enabled server returns `Ok` with the
  absence named, and the caller's startup line names the server and
  its missing env. Pinned by a unit test over `seed_user_servers`.
- [ ] **C2 — the callers keep their honesty.** The cron catch-up prints
  the absence per server and still refuses a job whose `--tool` names
  an absent server; the bridge's lane-call path resolves or falls
  through without a registry refusal. Pinned by tests over both paths.
- [ ] **C3 — the run-level regression pin.** A run over a registry with
  one unkeyed enabled server completes — the memory load, the route,
  the answer — and its startup names the absence. Pinned by the
  scripted-lane run path (no paid call). KI-319 lands in the same
  commit with the github incident as its evidence.

## Files to be touched

**The registry** — `src/mcp_registry.rs` (the contract, the partition,
the tests)

**The seeding and its callers** — `src/agent.rs`
(`seed_user_servers`), `src/main.rs` (the startup line), `src/cron.rs`
(the catch-up arm), `src/gui/bridge.rs` (the lane-call path)

**The ledger** — `docs/known-issues.md` (KI-319, filed at landing)

## Out of scope

- **Which servers are enabled.** The import's enable-by-default and
  the twelve-entry ballast are product questions; this plan changes
  what an unkeyed entry COSTS, never what is enabled.
- **The vault overlay's reach.** Already landed (KI-250's second pass);
  the registry reads the dotenv the callers hand it.
- **A new surface.** `fleet mcp health` and the doctor card already
  report per-server state; the startup line is the run's own.

## Verification

The tests named per criterion, then the whole gate — `cargo fmt --all
--check`, `cargo clippy --all-targets -- -D warnings`, `cargo build
--release`, `cargo test`. The incident itself is the acceptance case:
with the four servers re-enabled and their env still unset, a run must
complete with four named absences — that is the shape C3 pins.

## Linked artifacts

- `src/route.rs` `admissible` / `WalkReport::unkeyed` — the router's
  own named-skip pattern this plan ports
- The 2026-10-25 registry cleanup (the four disables) — the instance
  fix this plan generalizes
- `src/mcp_registry.rs` `enabled_specs` — the fail-fast this plan
  replaces
