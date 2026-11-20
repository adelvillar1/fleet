---
status: active
created: 2026-11-20
updated: 2026-11-20
slug: guard-list-inversion
---

# Guard lists: make coverage grow with the crate, not with the guard

The structure guards scan modules named by hand. A module added to
`lib.rs` tomorrow is scanned by nobody until someone remembers it — the
inverse of what a guard is for. This plan inverts that direction and is
the prerequisite for the `bridge.rs` split (a split changes module
identity, and today that silently changes what the guards scan).

## Context

- `docs/architecture/structure-guards.md` inventories the guard classes;
  the socket guards are the clearest instance: `src/judge/tests.rs` (and
  its delegate/explain siblings) hold literal pairs —
  `("doctor.rs", include_str!("../doctor.rs"))` at `src/judge/tests.rs:1557`
  — inside a hand-written list. The delegate guard's list is documented as
  "27 modules".
- `src/lib.rs` declares 99 modules today (measured 2026-11-20).
- `include_str!` needs a literal path, so a fully derived list is not
  possible in a `const` — the workable inversion is a **checked table**:
  keep the pairs, and add a test that parses `lib.rs` and asserts the
  table's module set equals the crate's module set. A new module then
  reds the build until it is either added or explicitly exempted with a
  named reason.
- The 156 `include_str!` sites total (per the OMH dive) are not in scope:
  most are content scans, not module-census guards. The inversion applies
  to the census guards first.

## Status pass, 2026-11-20

Step 1 landed: `src/testcensus.rs` carries the census — one table of
**142 module files** (`include_str!`-literal, so a missing file is a
compile error), a run-time walk of `lib.rs` plus every nested `mod`
declaration, and `the_census_matches_the_crates_module_tree`, which
asserts the table and the walked tree agree in **both directions**. The
test's first run reddened naming `testcensus.rs` itself — the intended
behaviour, and the self-registration is now in the table.

Measured at this pass: the three beat guards scan 30 module files between
them; 113 modules are scanned by none of them. The guard migration below
(the second half of this plan) is what closes that gap; it is not the
table's job to decide what a guard asserts.

## Status pass, 2026-11-20 (second)

Steps 2 and 3 are landed for the delegation guard. `testcensus` gained
`matches(pattern, module)` (exact, `dir/*`, `*/suffix`, `dir/**`) and
`unaccounted(scanned, exempt)`, plus their own proof test.

Measured before the change, then fixed: the delegation guard scanned **27
of 142** modules from a hand-kept table — **115 modules it never looked
at**. Scanning the whole census showed that expansion is nearly free: only
six modules in the crate name the beat at all, four were already declared
drivers, and the remaining three — `approvals.rs`, `onboard.rs`,
`settings.rs` — name only its constants (`DELEGATE_TOOL`,
`DELEGATE_BUDGET_USD_DAY`). They are now named drivers with their reasons,
not silent omissions.

`DRIVERS` is module-level, `CENSUS_EXEMPT_DRIVERS` names `main.rs` (the
binary root, which the census cannot list), and
`every_module_is_accounted_for` reds on a driver that names no module —
the typo direction the union check would otherwise miss.
`delegate::tests`: **54 passed, 0 failed**, scan set 27 → 142.

Still owed: the same inversion for `judge` (13 modules name it) and
`explain` (8), then `structure-guards.md`'s convention.

## Approach

1. Inventory the census-shaped guards: the three beat guards (judge,
   delegate, explain) and any other guard whose list is "every module
   except these". Name each in this plan's Status pass.
2. Add one shared test helper — `scanned_modules()` — that reads
   `include_str!("../lib.rs")`, extracts `mod X;` declarations, and
   resolves each to its file path(s) (`src/x.rs` or `src/x/mod.rs`, and
   directory submodules where the guard's list already includes them).
3. Each census guard splits its list into: `SCANNED` (the table it has)
   and `EXEMPT` (module → one-line reason, e.g. the guard's own tests).
   A new test per guard, `every_module_is_accounted_for`, asserts
   `lib.rs`'s set == `SCANNED ∪ EXEMPT` — both directions: a module in
   the crate but in neither list reds; a stale entry naming no module
   reds.
4. Prove the inversion by red-proof: add a throwaway module to `lib.rs`
   in a scratch commit, confirm the guard reds naming it, remove it.
5. Record the convention in `structure-guards.md` so the next guard is
   born accounted-for.

## Acceptance Criteria

- [ ] Every census-guard's `SCANNED ∪ EXEMPT` equals `lib.rs`'s module
      set, asserted by a test that reads `lib.rs` at compile time.
- [ ] Red-proof: a new `mod` line in `lib.rs` with no guard entry fails
      the suite with the new module named; the revert is clean.
- [ ] No guard's effective scanned set shrinks: before/after sets are
      compared and any difference is named in the plan's Status pass.
- [ ] `structure-guards.md` states the convention (checked table, not
      hand-appended list) and the exemption rule (one-line reason, never
      silent).
- [ ] The full gate exits 0.

## Files to be touched

- `src/judge/tests.rs`, `src/delegate/tests.rs`, `src/explain/tests.rs`
- one shared test-support helper (location decided by the first edit;
  `src/lib.rs`'s `#[cfg(test)]` support module if one exists)
- `docs/architecture/structure-guards.md`

## Out of scope

- The 156 content-scan `include_str!` sites that are not censuses.
- Changing what any guard asserts about the modules it already scans.
- A `build.rs` code generator (the checked table is the smaller change
  and needs no new build step).

## Verification

1. `cargo test -p fleet --lib judge` / `delegate` / `explain` scoped.
2. The red-proof in a scratch worktree or a scratch commit, reverted.
3. The full four-leg gate once, at the end.

## Linked artifacts

- `docs/architecture/structure-guards.md` — the inventory this plan
  extends.
- `docs/plans/2026-11-20-bridge-module-split.md` — the split that needs
  this landed first.
- `docs/architecture/deep-dive-2026-11-20-fleet-state.md` — F3.
