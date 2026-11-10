# Handoff — the resource contract: build it whole

**Written 2026-11-09 by the session that designed it.** Read this before the
plan; then read the plan. Everything below was measured or built in that
session; where a number is a proposal rather than a fact, it says so.

- **Plan (the contract):** [`docs/plans/2026-11-09-the-resource-contract.md`](2026-11-09-the-resource-contract.md)
  — status `in progress`, criteria C1–C12, and the build order at the end.
- **Recap of the session that got here:** [`docs/recaps/2026-11-09-one-composer.md`](../recaps/2026-11-09-one-composer.md)
  — the surfaces work plus the disk incident (KI-502), the duplicate artifacts
  (KI-504), the budget/ruling (KI-505), the burned release (KI-503).
- **Repo state:** `master` at `ea78df2`; shipped **0.6.20** (the doctor's `disk`
  and `caches` lines, the release preflight). The `fleet footprint` verb is
  committed but **not released**, so the installed binary does not have it.

## The owner's ruling, in their words (this is the spec's source)

1. *"I want fleet to run as lean as possible. We cannot have this pattern of
   using resources until near exhaustion to clean up. It needs to be a
   responsible citizen, not just a good citizen."*
2. *"fleet needs to be aware of what it is using at all times — disk, system
   memory, even processes it has spawned — and then determine if it's being
   responsible… whether that's a sys1 decision or some deterministic approach,
   that's a mechanics choice."*
3. *"What I don't want is fleet trying to use more than what's available… measure,
   run work, remeasure, and if there is any runaway work then curtail the
   behavior. An LLM or sys1 call that prevents the environment from crashing is
   worth the cost."*
4. *"We can measure and calibrate — recursive learning is the shape we need."*
5. *"Responsible execution is a matter of environment and risk the person is
   willing to take; a constrained environment like this laptop needs better
   upkeep. That's what fleet needs to learn."*
6. *"The risk appetite can be inferred as well: if an environment is constantly
   under 10% of available space you can infer the user likes risk, and tighter
   controls on fleet are needed."* — **the direction matters: habitually tight
   ⇒ fleet tighter, never looser.**
7. *"It needs to go a little further… and then determine if it's being
   responsible… recursive learning is the shape."*
8. *"We need to build the whole thing."* (the instruction this handoff serves)

## What is already built (do not rebuild)

### Rust, `src/footprint.rs` — the contract's arithmetic, 6 tests, green

| Item | Meaning |
|---|---|
| `Proc`, `parse_ps`, `tree_of`, `memory_kib` | the live process tree from `ps -o pid=,ppid=,rss= -ax`; a sibling is never ours |
| `Line`, `evaluate_tree`, `render` | the tree graded against `MEMORY_CEILING_KIB` (2 GiB) and `PROCESS_CEILING` (80); the report names the heaviest pid |
| `Env { free_fractions, fill_gib_per_day }`, `Env::standing_free` | the environment's history; the **median** share, so one good day reads as nothing |
| `Appetite { Tight, Balanced, Roomy }`, `appetite(&Env)` | `< 10%` standing free ⇒ `Tight`, `< 25%` ⇒ `Balanced`, else `Roomy`; no history ⇒ `Balanced` |
| `Appetite::reserve_share` | **fractions of the volume**: tight `0.12`, balanced `0.08`, roomy `0.04` |
| `reserve_kib(total, env, declared)` | the reserve **and** its source (`"declared"` wins, else `"inferred"`) |
| `admit(available, reserve, estimate) -> Admission` | headroom vs need; refused with both numbers |
| `settle(before, after, budget) -> Settlement` | the delta; `runaway` when it exceeds the budget |

### `fleet footprint` (a CLI verb; `src/cli.rs` + `src/main.rs`)

Live output on this machine:

```
memory       ok    13 MiB of 2048 MiB (11 of 13 in pid 8264)
processes    ok    2 of 80
environment  roomy — reserve 18 GiB of 460 GiB (inferred; 1 record(s), standing 25% free)
disk         the budget table is scripts/footprint.sh (stores against their declared budgets)
```

It reads the tree from its own pid and the environment from
`~/.fleet/footprint-history.jsonl`, one record per `scripts/footprint.sh
--record`.

### Scripts (all committed, all smoke-tested this session)

| File | What it does now |
|---|---|
| `scripts/footprint.sh` | declares the budgets (`target` 20 GiB, one lane 20, lanes total 24, `~/.fleet` 12 — env-overridable `FLEET_BUDGET_*`), measures, prints the table, **exits 1 when a store is over**; `--json`; `--record` writes `~/.fleet/footprint.json` **and appends** `footprint-history.jsonl` (capped 90) |
| `scripts/cache-sweep.sh` | age/size prune of `target/debug`; **dedupe** (keep `FLEET_KEEP_ARTIFACTS`, default **2** — one per live build configuration, see below); `prune_build_dirs` (1.3 GiB of superseded build-script outputs); `prune_bundles` (abandoned `rw.<pid>.*.dmg`, keep newest 2 dmgs); `clean_failed` (a dir with a `.failed-build` older than `FLEET_FAILED_GRACE_MIN`, default 60, is cleaned whole) |
| `scripts/release.sh` | `preflight_disk`: refuses a volume under `FLEET_DISK_FLOOR_GIB` (20), cleans the gate lane past `FLEET_GATE_TARGET_MAX_GIB` (20); **releases the gate lane after the cut publishes** unless `FLEET_KEEP_GATE_LANE=1` |
| `scripts/release-gate-run.sh` | writes `<lane>/.failed-build` on a red leg, clears it on a green one |
| `scripts/daily-maintenance.sh` | legs: cache sweep, `tmp/`, corpus, checkpoints, repo hygiene, worktree sweep, **8** build lanes (warm ≤3 days **and** under the 20 GiB cap), **9** footprint (`--record`), **10** orphan naming; gate logs pruned past 7 days |
| `src/doctor.rs` | `disk` line (`evaluate_disk`, `DISK_FLOOR_GIB` 20) and `caches` line (`evaluate_caches`, reads the sidecar, never walks 15 GiB) |

### Ledger rows to read first

KI-502 (the lanes ate the drive), KI-503 (the burned 0.6.18), KI-504 (12.8 GiB
of duplicate compiles; the two-copy rule), KI-505 (no store had a budget).

## The numbers, and whose they are

- **Reserve fractions, ceilings, budgets are settled** (the owner's 2026-11-10
  ruling: *"leave them like that"*). Tight 12% / balanced 8% / roomy 4%; 2 GiB
  memory; 80 processes; 20 GiB free floor; lane 20 GiB. They live in exactly
  three places — `src/footprint.rs` (constants), `scripts/footprint.sh`
  (budgets), `src/doctor.rs` (`DISK_FLOOR_GIB`) — and C3/C4 locked them in as
  declared rather than waiting on confirmation.
- **Never learn a safety floor downward.** Only estimates and per-work budgets
  calibrate. A floor that history can lower is not a floor.
- **Keep two artifacts per crate per profile, not one.** Measured: a plain
  `cargo build --release` and a `cargo tauri build` produce two distinct
  `fleet-desktop` rlibs (86 MB each) with live fingerprints; dropping either
  costs a full LTO rebuild (3m18s measured). Five buys nothing: at 5 the sweep
  finds nothing left to drop.

## What is NOT built (the work)

The plan's build order is the authority; the seams are:

1. **C2 — release path settles** (`scripts/release.sh`): replace the hardcoded
   floor in `preflight_disk` with the reserve `fleet footprint --json` derives
   (the binary is `target/release/fleet` during a cut; guard its absence), and
   after the assets upload compare the lane's size delta against the admitting
   estimate — over ⇒ the `.failed-build` marker, the diagnosis window, then the
   existing clean.
2. **C1 at the seams**: `admit` at exactly the reserve (equal ⇒ refused),
   `settle` at exactly the budget (equal ⇒ not a runaway).
3. **C3 — the run loop** (`src/agent.rs`, the turn boundary where the
   context-budget check already runs): admit before the first turn when a
   folder is granted; re-measure at the boundary (`statvfs` + `parse_ps`); a
   breach stops through the same path a seat denial uses and writes the numbers
   as a `Record`. **Measure the cadence** — every turn may be too often; every
   fifth is the guess.
4. **C5 — the rows** (`src/calib.rs`, the shape-typed writer): `op=footprint`
   with work class (`release-gate`, `release-assets`, `run:<folder>`,
   `lane-bootstrap`), the admitting estimate, the settled delta, the reserve at
   admission, the verdict, and the sys1 words when one fired. The demand map
   reads by `op` already.
5. **C4 — the sys1 arm** (after C5, because it needs rows to be graded): one
   bounded ask through the existing lane client on an **abnormal reading only**,
   fed the numbers and the run's recent tool calls; `runaway` curtails,
   `expected` raises that run's recorded budget, a refused answer is `expected`
   and records itself. Respect the judge-budget rule.
6. **C8/C9 — the learning**: the estimate is the median of the last ten settles
   per work class (prior = the constant until three exist); the judge's verdicts
   are graded by the subsequent outcome (the `delegate.rs` evidence pattern).
   Both are pure reads over rows and testable without a lane.
7. **C6 — the runaway proof**: a scratch folder filled past the run's budget,
   asserting the stop, the transcript's numbers, and the cleanup after the
   grace.

## Delivery (the house's own flow — do not improvise)

- Gate: `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings`,
  `cargo build --release`, `cargo test`, `node tests/face/harness.mjs`,
  `python3 scripts/ki_ledger.py`, `python3 scripts/plan_check.py`.
- Cut: `scripts/release.sh tag vX.Y.Z` then **immediately**
  `scripts/release.sh assets vX.Y.Z`. **Never** let a co-tenant commit land
  between them: `assets` refuses when HEAD ≠ tag, and an empty release holds
  `releases/latest` and 404s the feed (KI-503 — it happened twice).
- Install: `fleet update` (it replaces the binary through a rename; **never**
  `cp` over a running binary — KI-491), then restart the face:
  `kill <the `fleet gui --no-open --remote …` pid>` and relaunch it with the
  same flags; verify `GET /api/version`.

## Traps that cost this session time (read before you debug)

1. **A rebuilt face can look old.** The service worker and Chrome's HTTP cache
   serve the previous `?v=<hash>` asset; a hash navigation (`#/desk` → `#/desk`)
   is a same-document move and reloads nothing. Use `Page.reload
   {ignoreCache:true}` in a **fresh profile**, and check
   `document.scripts[last].src` before concluding the code is wrong.
2. **Do not `git stash` to "test the committed version" of a script you just
   edited** — the stash reverts the very file under test (this started a stray
   gate build). Extract the function into `/tmp` and run it instead.
3. **`cargo test --lib` alone fails ~41 tests** (`agent`, `delegate`) with
   "the mock-server binary was not found": the fixture binary is built by
   `--all-targets`. Run the full `cargo test`.
4. **A doctor check must not need a binary on PATH.** `df` vanished under the
   install tests' restricted PATH and the check reported a broken machine; the
   free-space read is `statvfs` (`libc`, declared in `Cargo.toml`) now.
5. **BSD `sed` has no `\|` alternation** — use `sed -E '(a|b)'`.
6. **`rm -rf` is gated** in the tool layer ("destructive and irreversible").
   Prefer `cargo clean`, `git worktree remove`, or the house's own scripts —
   and if you must delete, do it in a script that logs what it took.
7. **The face's process is not `fleet gui --port 3080`.** It is
   `~/.fleet/bin/fleet gui --no-open --remote --relay …`; `pkill` on the wrong
   pattern leaves the old binary serving.
8. **The face harness's DOM stub is flat** (`documentElement.children` is every
   id, no nesting from the markup) — `#panel-chat #composer` can never match;
   select by id, and assert placement through `element.parentNode`.
9. **`is_multiple_of`**: the toolchain's clippy rejects `len() % 2 == 0`.
10. **A co-tenant session works in this repo.** It commits (moving HEAD) and
    leaves untracked plans (`docs/plans/2026-11-09-failsafe-restore.md` was not
    yours). Re-read on-disk before batch edits; commit only your paths.

## Open decisions the owner must make (ask, don't assume)

- ~~The reserve fractions and ceilings~~ — settled 2026-11-10: *"leave them
  like that."* No longer an open decision.
- On a deterministic breach: hard-stop the run, or pause it for a yes?

## The one-line state of the world

The instrument is real and green; the contract's arithmetic is real and green;
**the healing — refusing, curtailing, cleaning, learning — is not built.**
C2 first, then C3, then the rows (C5), then the judge (C4), then the learning
(C8/C9), then the proof (C6).
