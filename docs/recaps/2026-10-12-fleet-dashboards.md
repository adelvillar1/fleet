# Session Recap — Fleet dashboard plugins and the shared board seam

**Date:** 2026-10-12 **Branch:** `master` (default, all work) + `fleet` (working branch), both level with each other **Plan:** `docs/plans/2026-10-12-fleet-dashboards.md` — "router board and second-brain board" **Repo:** fork at `git@github.com:adelvillar1/deepseek-harness.git` (SSH, private); `upstream` stays pull-only at `dsh-v0.2.1-alpha.2`

## Summary

The owner's direction was that the visual parts of the agnostic-router-kit and the full functionality of the second brain had to exist as harness plugins with genuinely visual dashboards, because "the current control tower view is just text and honestly not very helpful." Three dual-face fleet bundles now deliver that: `@fleet/dsh-router-board` renders the model router's steering — which tier serves which models, how the share sits, where the judge and the cascade intervene — `@fleet/dsh-second-brain` renders the second-brain tower's earned autonomy, quality loops, claims-versus-evidence, and maturity weeks, and `@fleet/dsh-tower` was migrated onto the same foundation.

The load-bearing decision of the session was to stop the three boards each writing the same snapshot machinery. `@fleet/dsh-board-seam` now owns the cached snapshot, the self-rescheduled cadence, the guarded HTTP route with its `no-store` headers, and the browser page shell; each board owns only its own source read, its cadence, and its route path. Extracting it turned six dependency-policy violations into zero, killed every cross-file clone between the boards, and made the boards' host halves type-only importers of the seam.

The second decision came out of the first. With the boards importing the seam only for types, their module graphs are light, and Cordis activates sibling entries in import-resolution order rather than composed order — so both boards' `apply` ran before the seam provided its service and threw. The tower, which imports `dsh-session` and so resolves slower, won the race by accident. The fix was to stop reading the service defensively and start declaring it: each board now exports `inject = ['boardSeam']` and reads `ctx.boardSeam`, and the seam declares the service non-optional. Cordis's deferred activation holds a plugin whose injected service is absent inactive rather than letting it run half-initialized, so mount order became immaterial and a composition without the seam degrades visibly instead of throwing at boot.

## Plans worked on

### `docs/plans/2026-10-12-fleet-dashboards.md` — router board and second-brain board

Status after this session: **complete** — both boards shipped, the tower migrated onto the same seam, all five acceptance criteria met. The plan's own note that "a later chunk may retire or restyle its text panel once these boards land" is what the seam migration of `dsh-tower` discharged.

## Commits

| Part | Message |
|------|---------|
| boards and seam | Fleet: the shared board seam, and the three boards migrated onto it |

The two new boards and the seam migration landed in one commit rather than two. The seam was extracted from the boards while they were still uncommitted, so there was never a point at which the boards existed without it.

---

## What was added

### `@fleet/dsh-board-seam` — the package

Twenty-three files and 572 lines of source across its host and browser halves. The seam is itself a dual-face bundle: a host half providing one service, and a browser half providing the fetch-and-poll source, the display formatting, and the page shell every fleet board renders inside.

The host service is `boardSeam.register(id, definition)`. A board hands over its own path, its cadence, and its own read of its own source; the seam builds the cached-snapshot service, kicks the opening read off without awaiting it, arms the self-rescheduling `.unref()` timer under a double-arm guard, and registers the guarded route. The board's own service is what the board then provides to its browser half, so the wire contract never crosses a package boundary twice.

The seam's `register` runs the opening read and arms the cadence itself rather than leaving that to the caller, because both belong to the board's definition and the service owns the guard that makes arming idempotent. A board that registered twice would otherwise arm two timers against one cache.

### The three boards on the seam

`@fleet/dsh-router-board` (1,399 lines of source) reads the router's operator API read-only — state, usage, suggest, and workflow runs — with a bearer token discovered from the roster file, and renders tier chains, the usage mix, judge and cascade counters, and the run history as hand-rolled SVG. `@fleet/dsh-second-brain` (2,025 lines of source) reads the derived second-brain state files read-only and renders gates, harness outcomes, quality loops, report rows with their conflicts, maturity weeks, and the tower's own projects and timeline.

Neither launches, writes, or proxies anything toward its source. Every wire payload is validated by a parser that names what it rejected rather than casting, and a refused read keeps the last snapshot visible — an error state is a row on the board, not a blank one.

`@fleet/dsh-tower` lost its own `route.ts` and `service.ts` (160 lines) and its private copy of the snapshot machinery, keeping only the portfolio scan, the turn reporting to the two feeds, and its route path.

---

## What was fixed

### The client bundles could not resolve the seam's browser half

All three boards' client bundles failed to build with `MISSING_EXPORT` for `boardSlots` and `BoardShell`. The seam's `lib/client.js` is a `window.__ModuleLoader__.load({ id, factory })` closure factory — a CommonJS-shaped artifact, not a static ES module — so rolldown cannot statically resolve named imports from it. The `dsh-client-bundle-purity` plugin governs only `@deepseek-ai/*` specifiers, so `@fleet/dsh-board-seam/client` fell through to rolldown's normal resolution and was inlined by attempt.

The fix is the manifest's `dsh.client.external` field, which declares the module-table rows a client bundle must keep external. Each board now declares `"external": ["@fleet/dsh-board-seam/client"]`, and each built bundle contains a `require("@fleet/dsh-board-seam/client")` with zero inlined seam source and no dynamic chunks. The specifier resolves because `stripClientSuffix` maps `<id>/client` and the bare package name to the same graph row, and the seam's browser half registers under its bare package name.

### publint rejected the new packages' `files` fields

The narrowed `files` fields were not narrow enough: the four packages shipped closure-style `lib/client.js` artifacts whose publication payload publint flags. Each `files` field now names `lib/index.js`, `lib/client.js`, `lib/types/**/*.d.ts`, and `cordis.patch.yml` — the exact artifact set each face emits. publint exits 0 with zero fleet findings.

### The tower's client section spacing was inconsistent with the boards

The tower's portfolio panel used 32px between its sections where the two new boards used 16px for the same visual role. The tower was normalized to 16px, so three boards read as one surface. This was a deliberate visual correction, not a merge artifact.

---

## What was changed

### The boards declare their dependency instead of probing for it

Each board's host entry previously read `ctx.get('boardSeam')` and threw a named error when it was absent. That pattern is wrong twice over here: it loses the race described above, and the throw is unreachable code the moment the race is fixed, which would fail the per-file coverage gate.

Each board now exports `inject = ['boardSeam']` and reads `ctx.boardSeam` directly, matching the in-repo precedent in `packages/fleet/dsh-router/src/index.ts`, which declares `inject = ['agents', 'decisions', 'llm', 'tools']` over `@fleet/dsh-decisions`' non-optional `decisions` service. The seam's Context declaration moved from `boardSeam?: BoardSeam` to `boardSeam: BoardSeam`.

One consequence is worth recording: with `inject` declared and the seam absent, `ctx.plugin()` resolves successfully and the board's `apply` simply never runs. The three boards' "omits the seam" tests were rewritten to assert the board stays inactive and provides no service — and, for the router board and the tower, that no read and no scan happens — rather than asserting a throw.

### Two tsconfig programs reference the new packages

`tsconfig.base.json` gained source-mode aliases for `@fleet/dsh-board-seam`, `@fleet/dsh-router-board`, and `@fleet/dsh-second-brain`, and `tsconfig.host.json` lists the three packages in its host program, so the source plane resolves them without touching the root solution.

### The tower's stale devDependency was removed

`@deepseek-ai/dsh-client-store` left `dsh-tower`'s devDependencies; the tower's source and tests no longer import it. `pnpm-lock.yaml` records the removal alongside the three new importers.

---

## Acceptance criteria

| # | Criterion | Status | Evidence |
|---|-----------|--------|----------|
| C1 | Both plugins mount in the fleet profile; each adds a sidebar panel whose board renders real data from its source | Met | The fleet profile boots with all three boards active; `/fleet-second-brain/state` serves 166,868 bytes of real derived state across gates, harness, loops, reports, maturity, and tower, and `/fleet-tower/portfolio` serves 28 projects and 5 skipped entries |
| C2 | Each board is visual — cards, bars/heatmaps/sparklines, status dots — with no plain-text-table-only surface | Met | Hand-rolled SVG donuts, bars, heatmaps, and sparklines over CSS-module cards using `--dsw-alias-*` tokens; no chart library is a dependency of any of the three packages |
| C3 | Every wire payload is validated; a refused read keeps the last snapshot visible | Met | Validate-don't-cast parsers per read, each rejecting with a named reason; the client source keeps the last snapshot on a failed poll |
| C4 | Matched gates pass for both packages; unit/slot tests green | Met | 390 tests across the four packages pass; per-file coverage is 100% on statements, branches, functions, and lines for every source file in all four; typecheck, `verify-package-dependencies`, `verify-client-packages`, publint, and jscpd all exit 0 |
| C5 | Both plugins stay read-only toward their sources | Met | Neither board writes, launches, or proxies; the tower's own writes are confined to the calibration ledger and the harness-runs mirror, which are its feeds rather than its source |

### Verification results

The four packages hold 390 tests across 17 spec files, all passing. Per-file coverage is 100% on all four metrics for every source file in `dsh-board-seam`, `dsh-router-board`, `dsh-second-brain`, and `dsh-tower` — both the host `src` and the browser `src/client` halves.

The live smoke test drove the real fleet profile: all three boards activated with no activation errors, all three routes served HTTP 200 through the Host/Origin fence and browser authentication, and the client bundles were loaded and executed against a simulated module table with every external request resolving.

---

## What was fixed after the first push

Two defects survived this pass: `@fleet/dsh-brain` did not activate in the fleet profile, and the router board discovered its bearer from another application's roster file. Both were fixed the following day, together with the investigation that explained why each had survived the smoke test, and both are recorded in [2026-10-13-fleet-followups.md](2026-10-13-fleet-followups.md).

---

## Files changed

The three new packages contribute 69 files and 8,396 lines. `dsh-tower` changed 14 files for 192 insertions and 540 deletions. `pnpm-lock.yaml` gained the three new importers and lost the tower's stale `dsh-client-store` entry.

The largest additions are `packages/fleet/dsh-second-brain/tests/board.spec.ts` at 1,796 lines and `packages/fleet/dsh-second-brain/src/client/BoardPage.tsx` at 791.

---

## Open questions / next steps

- **The tower's text panel was not restyled beyond the spacing normalization.** The plan anticipated that "a later chunk may retire or restyle its text panel once these boards land." The seam migration made it share the boards' shell and spacing; a fuller restyle of its project rows is still open.

---

## Notes

The seam extraction was driven by evidence rather than taste. Six `verify-package-dependencies` violations, six cross-file clones between the boards, and a 226-line diff in the tower's page component all pointed at the same duplication, and the seam is what removed all three at once. The rule that decided it is the repo's own: a capability seam comprises Service Definition, Service Provider, and Consumer roles and is complete, never one role.

The activation race is the part worth remembering. `ctx.get()` looks like the defensive choice and is actually the fragile one: it reads the service at whatever moment the fiber happens to run, which under Cordis's concurrent entry initialization is import-resolution order, not composed order. `inject` converts the dependency into a deferral the runtime honors, which is why the fix made the problem disappear rather than relocating it.

Every commit in this work is reachable from both `master` and `fleet`, which are level with each other. `upstream` was not advanced. A concurrent session landed the native Rust agent slice 1 on `master` during this work; this session's changes were verified and committed on top of that HEAD rather than against the earlier one.
