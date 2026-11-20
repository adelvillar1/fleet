---
status: draft
created: 2026-11-20
updated: 2026-11-20
slug: bridge-module-split
---

# Split `bridge.rs` by route family

`src/gui/bridge.rs` is 21,942 lines (production lines end at 12,306),
68 `pub fn`, and the second-hottest file in the repo (229 commits in the
last 30 days). It is the face's whole server-side surface in one file.
This plan cuts it along the routes it already implements.

**Sequenced after `guard-list-inversion`.** The census guards pin module
identity, so a split today would silently shrink what they scan; with the
inversion landed, the guards account for new modules by construction.

## Context

- Route families are already visible in the file's function names and
  the router table: sessions/archive, settings, providers/router,
  inbox/bots, update/desk, memory/learning, and the shared plumbing
  (state types, serialization, write-lock discipline).
- Tests live in-file from line 12,306 on and move with their family.
- No public API exists outside the crate for this module (`pub(crate)`),
  so the split is contained to the crate.

## Approach

1. Record the pre-split census: every `pub fn` name, every route path
   string, and the test count, into this plan's Status pass.
2. Create `src/gui/bridge/mod.rs` with the shared plumbing and one
   submodule per family (`sessions.rs`, `settings.rs`, `providers.rs`,
   `inbox.rs`, `update.rs`, `memory.rs`, `shared.rs` — the exact cuts are
   named from the router table, not invented).
3. Move one family per commit, its tests with it, re-exporting the same
   names from `mod.rs` so callers (`server.rs`, `spine.rs`, `main.rs`) do
   not move in the same commit.
4. Re-run the census after each commit: `pub fn` names, route path
   strings, and test counts must be equal to the pre-split numbers.
5. Regenerate the guard tables (the inversion makes this a red-then-add,
   not a remembered edit).

## Acceptance Criteria

- [ ] The `pub fn` census, the route path-string set, and the test count
      are identical before and after (recorded per commit in the Status
      pass).
- [ ] No family module exceeds ~2,500 production lines — the split's
      point is reviewability, so a 9,000-line "sessions" module is not a
      split.
- [ ] Callers outside `src/gui/bridge/` are untouched except `mod`
      declarations; the crate's `pub(crate)` surface is unchanged.
- [ ] The census guards pass with the new modules accounted for.
- [ ] All existing bridge/server tests pass unchanged (they are the
      behavioural evidence — no test is rewritten to fit the split).
- [ ] The full gate exits 0.

## Files to be touched

- `src/gui/bridge.rs` → `src/gui/bridge/mod.rs` + family submodules
- `src/gui/bridge/tests/` — the in-file tests, split by family
- `docs/architecture/structure-guards.md` — the census entry, if the
  inversion lands as designed

## Out of scope

- Any behaviour change, route rename, or handler refactor beyond moving.
- The JS side (`app.js` is its own plan).
- `server.rs`'s connection handling.

## Verification

1. `cargo test -p fleet --lib gui::bridge` after each family move.
2. The census diffs per commit.
3. The full four-leg gate once, at the end.
4. A live face smoke: start `fleet gui`, open two surfaces, confirm the
   routes answer (the server tests cover the protocol; this proves the
   composition).

## Linked artifacts

- `docs/plans/2026-11-20-guard-list-inversion.md` — the prerequisite.
- `docs/architecture/deep-dive-2026-11-20-fleet-state.md` — F5.
