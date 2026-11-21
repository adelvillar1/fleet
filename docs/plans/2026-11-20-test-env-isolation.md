---
status: active
created: 2026-11-20
updated: 2026-11-20
slug: test-env-isolation
---

# One isolation primitive for the process's shared state in tests

Twenty-three ledger rows name the parallel-suite flake class, and each
was fixed on the test that flaked: process-environment races (KI-574,
closed today by serializing two armada tests), a test that unlocked the
process-global vault `OnceLock` (KI-389), port waits (KI-027), timing
sized for an idle machine (KI-287/288). The class is environment
isolation; the fixes are per-test.

## Context

- KI-574's row: two armada tests unsets and sets `OPENSHIP_*` vars while
  `cargo test` runs them concurrently against one process environment;
  whichever unsets first makes the other read the wrong refusal. Closed
  today — by what shape, the next session should read before refactoring.
- KI-389: a test called `home::unlock_vault`, a process-global switch in
  a `OnceLock`, and leaked the vault to every later test.
- The suite runs ~2,450 tests, most under default parallelism; the flake
  cost is a full-suite run to discover one test's isolation bug.

## Status pass, 2026-11-20 (superseded)

Two instances landed the same day this plan was written, both red in a
full `cargo test` and green in isolation:

- **KI-578** — three `armada::join` tests inherited the machine's git
  default branch; fixed by pinning each bare repo's HEAD and asserting
  every git exit status.
- **KI-579** — `route::tests::an_exhausted_walk_names_each_cause_and_its_repair`
  read the process env through `resolve`'s key fallback; fixed by naming
  a variable nothing sets, with the roster's documented mapping pinned
  by a pure assertion.

Both fixes are per-test. The general guard this plan describes (a
crate-wide serialization primitive for env- and cwd-sensitive tests,
with the census as the list of what must take it) is still open, and
these two rows are its evidence.

## Status pass, 2026-11-20 (scoping)

Measured before starting, because the plan names "the known offenders" and
the tree disagrees:

- **85 call sites** of `std::env::set_var` / `std::env::remove_var` across
  ~20 modules — `layerfs_tools` 10, `gui/server` 9, `armada/deploy` 8,
  `tools` 5, `layerfs_view` 5, `footprint_history` 5, `subagents` 4, and
  four each in `judge/tests`, `gui/bridge`, `explain/tests`,
  `delegate/tests`, `armada/join`.
- Only **3** go through an `unsafe { }` wrapper; the rest are the plain
  calls, so the guard in AC2 would red on 82 sites the moment it lands.

That changes the shape of this plan: it is a migration of 85 sites, not of
"the known offenders plus a guard". The guard should land **after** the
support module and the bulk migration, or it needs 85 named exemptions —
and 85 exemptions is a list nobody reads, which is the failure mode
`guard-list-inversion` was built to avoid.

Two instances are already closed and are the evidence base: **KI-578**
(a test inheriting the machine's git default branch — fixed by pinning the
default, not by env) and **KI-579** (`route`'s test reading a provider key
from the process env — fixed by naming a variable nothing sets).

## Pass 1 landed, 2026-11-20

`src/testsupport.rs` exists (registered in `lib.rs` under `#[cfg(test)]`, and
in the census so it cannot be forgotten): a process-wide `env_lock()` that
recovers from poison, and an RAII `EnvVar::set` / `EnvVar::unset` that
restores the previous value — including absence — on drop, with the unsafe
call isolated behind one documented SAFETY condition.

First cluster migrated: **`lanes`** — its private `ENV_LOCK` and `EnvVar`
duplicate are gone, and all five of its environment tests take the shared
lock. Its own 25 tests pass unchanged.

Pass 2 landed: **four more duplicate guards are gone** — `explain/tests`,
`home`, `mcp_registry`, and `lanes` (pass 1). The crate now has exactly one
`ENV_LOCK` and one `EnvVar`, both in `testsupport`; a scan for
`static ENV_LOCK` returns only that module.

Pass 3 started (two of the largest clusters): **`gui/server`** (9 sites — three
inline set/restore blocks, including one that mutated the environment while an
async server was serving) and **`armada/deploy`** (7 sites — its private
`ENV_LOCK` and its `unsafe { remove_var }` restores are gone). Both files now
contain zero `std::env::set_var`.

That pass also produced the pattern's one real hazard: `testsupport::env_lock`
is a `std::sync::Mutex`, and two of the migrated tests are async — clippy's
`await_holding_lock` caught the guard held across an await. The allowance is
per-test with its reason (the variable must stay set for the whole request, and
nothing inside the future takes the same lock), not module-wide. The full lib
suite runs clean in 63s, so the lock does not deadlock under the real
scheduler — which is the only evidence that matters for that lint.

Pass 3, second half: **the two locks are now one.** `layerfs_tools` had its own
`EnvGuard` *and* a crate-level `TEST_ENV_LOCK`; `layerfs_view` and `subagents`
took the latter too — so two per-module locks and one shared meant env windows
could still overlap across the two. All 40 sites now take
`testsupport::env_lock()`, the `TEST_ENV_LOCK` static is deleted from `lib.rs`,
and `layerfs_tools`' 27 tests each got the lock they never took.

Two primitives came out of it, because the migration needed them:

- `EnvVar::set_os(name, value)` — the path-shaped value that made a module keep
  its own guard in the first place.
- `EnvVar::swapping(name)` with `.to(..)` / `.clear()` — one guard a test can
  move several times. Five `set` guards in a row restore five *intermediate*
  values, so the last drop puts back the second-to-last rather than the
  original; `a_swapping_guard_moves_the_value_and_restores_the_original` pins
  that.

A duplicate-lock pass was needed: inserting `env_lock()` into tests that already
took `TEST_ENV_LOCK` and then rewriting the latter would have made one test take
the same non-reentrant mutex twice — a deadlock, not a flake. Twelve duplicate
guards were collapsed; the full lib suite runs clean in 58s.

Third batch, 25 sites over six files: `footprint_history` (5, one test moving
one knob four ways — the `swapping` guard's motivating case), `gui/mod` (3),
`armada/join` (4, two windows around two joins), `tools` (5), and the identical
copies of the guard in `judge/tests` and `delegate/tests` (4 each).

`tools` held a real leak rather than a race: `BLEND1_VECTORS_DIR` was set for a
fixture store and then `remove_var`'d — so a machine that had the variable set
lost it for the rest of the run. `swapping` restores it.

Remaining: `subagents` 4, `gui/bridge` 4, `layerfs_view` 3, `judge` 3,
`identity` 2, `shellpath` 1, and the two production sites (`apply_env` and
`layerfs_tools`'"'"' own removal) that the guard deliberately does not cover. Surveyed properly before starting it, and the split is
not what the first count suggested — whole-file test modules (`delegate/tests`,
`judge/tests`, `testsupport`) carry no `#[cfg(test)]` marker inside them, so a
naive scan reads them as production:

- **~80 test sites** — `layerfs_tools` 9, `gui/server` 9, `armada/deploy` 7,
  `tools` 5, `footprint_history` 5, `armada/join` 4, `gui/bridge` 4,
  `subagents` 4, `judge` 3, `gui/mod` 3, `identity` 2, and others.
- **5 production sites**, and only two are legitimate: `main.rs:36` loads the
  `.env` once at startup; `shellpath.rs:118` is the documented PATH merge.
  The other three are **KI-582**, logged: `layerfs_view::history()` writes the
  machine environment into the process per call, which in the async face races
  between concurrently opened projects.

That last finding is why the survey was worth doing: the migration would have
walked past it, because a production site is not a test-isolation problem. The guard in AC2
lands **after** the bulk, per the ruling, so it never needs an 85-entry
exemption list.

## Approach

1. One support module behind `#[cfg(test)]` — the crate's existing test
   support location if it has one, otherwise a small `src/testsupport.rs`
   declared in `lib.rs` under `#[cfg(test)]`:
   - `env_guard()`: a process-wide lock plus an RAII guard that records
     the old values of every var it sets/unset and restores them on drop.
   - `vault_guard()`: the same shape for the vault `OnceLock`.
   - `port()`: a single allocator that binds-then-releases or hands out
     unique ports, replacing per-test port picking where it appears.
2. Migrate the known offenders first: KI-574's two armada tests, KI-389's
   vault test, and the tests named in the port-wait rows.
3. Add a text-scan guard: outside the support module, no test calls
   `std::env::set_var` / `remove_var` directly (the same census shape the
   structure guards use). That is what makes the primitive hold — the
   guard, not the convention.
4. Run the full suite twice under default parallelism to prove the class
   quiet, and record both runs.

## Acceptance Criteria

- [ ] `env_guard`, `vault_guard` and `port` exist once; the migrated
      tests use them and no longer mutate the process environment
      directly.
- [ ] A text-scan guard reds when a test outside the support module calls
      `std::env::set_var`/`remove_var`.
- [ ] Two consecutive full-suite runs at default parallelism are green
      (the two runs are recorded in the plan's Status pass).
- [ ] The migration does not change what any migrated test asserts.
- [ ] The full four-leg gate exits 0.

## Files to be touched

- `src/testsupport.rs` (new, `#[cfg(test)]`) or the existing support module
- `src/armada/*.rs` — KI-574's two tests
- `src/memory.rs` — KI-389's vault test
- `docs/architecture/structure-guards.md` — the new guard's entry

## Out of scope

- Rewriting every test at once: the guard stops new offenders; existing
  ones migrate as their modules are touched, with the known four first.
- The integration suite's mock-server port protocol (`wait_for_port`
  already exists and is pinned by KI-027's fix).

## Verification

1. The migrated tests, alone and in the suite.
2. Two full-suite runs, default parallelism, both recorded.
3. The full four-leg gate once, at the end.

## Linked artifacts

- KI-027, KI-287, KI-288, KI-389, KI-574 — the class this plan closes.
- `docs/architecture/deep-dive-2026-11-20-fleet-state.md` — F3.4.
