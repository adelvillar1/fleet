---
status: draft
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

## Status pass, 2026-11-20

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
