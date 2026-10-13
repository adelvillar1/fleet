# Session Recap — Fleet follow-ups: the brain's config default and a self-contained bearer

**Date:** 2026-10-13 **Branch:** `master` (default, all work) + `fleet` (working branch), both level with each other **Plan:** `docs/plans/2026-10-12-fleet-dashboards.md` — "router board and second-brain board", already `completed` **Repo:** fork at `git@github.com:adelvillar1/deepseek-harness.git` (SSH, private); `upstream` stays pull-only

## Summary

The first pass left two defects behind: `@fleet/dsh-brain` crashed on activation in the fleet profile, and the router board read its bearer out of another application's file. The owner challenged both — why was a plugin failing to activate "out of scope", and why was the board pointing at a roster file instead of carrying its own credential. Both challenges were correct, and both exposed the same weakness: the smoke test I had reported as passing was weaker than it sounded, because it validated the three boards while the profile around them was still broken.

The brain now activates. The router board's bearer is a config value of this application rather than a pointer at a file in another one. Both are proven against the real fleet profile and the real router, not just in tests.

## Plans worked on

### `docs/plans/2026-10-12-fleet-dashboards.md` — router board and second-brain board

No status change: the plan was already `completed`, and this session's work was closing the two defects that completion had left behind rather than adding scope. The plan's machine-local override surface turned out to be the wrong shape for the bearer, which is the substance of the second fix below.

## Commits

| Part | Message |
|------|---------|
| base | Fleet: the shared board seam, and the three boards migrated onto it |
| docs | docs: correct the fleet-dashboards recap's commit table |
| this session | Fleet: the brain's missing config default, and a self-contained bearer |

The first row is the base this session started from, already recorded in [2026-10-12-fleet-dashboards.md](2026-10-12-fleet-dashboards.md); the two below it are this session's.

---

## What was fixed

### `@fleet/dsh-brain` did not activate in the fleet profile

It threw `TypeError: Cannot read properties of undefined (reading 'skillRoots')`. The cause is three facts that only collide at runtime: `apply(ctx, config)` declares `config` as required, every field of `Config` is optional, and the bundle's `cordis.patch.yml` inserts the row with no `config` key. Cordis hands the entry whatever the row carries, so a row without a `config` key delivers `undefined`, and the first property read throws. The spread on the line above survives `undefined` silently, which is why the failure landed one line further down rather than at the signature.

The package's own tests always mounted it with an object, so the path a real composition takes was never exercised. That is the gap that let this ship: the unit tests were green and the profile was red, and nothing connected the two.

The fix is the default `config: Config = {}`, which is what the code already assumes. `ResolvedConfig` is `Required<Omit<Config, 'skillRoots' | 'codeRoots'>>` — the two root fields are deliberately excluded from the resolved config because they are derived from the working directory rather than defaulted, so reading them off an empty object is the intended no-configuration behavior, not a fallback bolted on.

A new test mounts the plugin the way the profile does, with no second argument at all, and asserts both tools register and the prompt context lands. The live boot then confirmed it end to end: recall live over five stores, session indexing, the shape router over two routing rows and seven shape keywords, the skill provider, the sleep cycle over the memories tree, skill indexing over two roots, and code indexing over one repo — with no error anywhere in the log.

### The router board's bearer was a pointer into another application

The board discovered its bearer from `~/.agnostic-router-kit/roster.json`. Checking that path showed it was wrong twice over, and the investigation is what changed the fix rather than just its parameters.

The roster is authored in the kit's *checkout* and `kit apply` renders it into the kit's state home — both kits document exactly that pipeline in their READMEs. So a roster at `~/.agnostic-router-kit/roster.json` is not a stale path that needs correcting; it is a path the roster never occupies. Meanwhile the router actually listening on port 8300 is ZCode's own server at `~/.zcode/router/server.js`, whose config names its roster at `~/Projects/zcode-router-kit/roster.json` — a checkout path that varies per machine. Both kits' rosters carry the identical token, so any of the three candidate files would have worked on this machine, which is precisely why guessing one would have been patching a guess: it would have been right here and wrong on the next machine, for a reason nobody could see.

The owner's direction settled it: the bearer should be self-contained in this application's config and not a pointer at an existing one. So `rosterPath` and `rosterToken` are gone. `routerToken` is now the whole mechanism — a nonempty value rides every request, an empty one reads bare, and nothing is read from disk. The board therefore cannot inherit a credential from whatever kit happens to be installed beside it, and a forgotten token surfaces as four named 401 refusals rather than as an empty panel that looks like a quiet router.

The machine-local profile layer carries this machine's token, which is the only place a machine's own credential belongs; the repo's bundle patch stays insert-only and carries no values.

---

## Verification results

533 tests pass across the five fleet packages, and per-file coverage is 100% on statements, branches, functions, and lines for every source file in `dsh-board-seam`, `dsh-router-board`, `dsh-second-brain`, `dsh-tower`, and `dsh-brain`. Typecheck is clean on both the host and client faces. `verify-package-dependencies`, `verify-client-packages`, publint, and jscpd's own exit for the packages in scope all behave as before, and oxlint reports zero findings on the changed packages.

The router board's reads were proved in both directions against the live router on 127.0.0.1:8300, using the package's own built reader and the token the profile carries: with the bearer all four reads land, and with an empty one all four refuse naming their path and the HTTP 401. The fleet profile's composed config was dumped to confirm `fleet-router-board` carries the token and `fleet-brain` still carries no `config` key — the exact case that used to throw.

`verify-export-jsdoc` and `verify-md-wrap` are red, and jscpd reports one clone; all three are pre-existing and name none of the files changed here. `verify-md-wrap`'s findings are all in `docs/plans/2026-10-12-fleet-native-app.md`, and the jscpd clone is between `dsh-brain/src/memory-file.ts` and `skill-index.ts`, neither of which this session touched.

## Files changed

`dsh-brain` changed two files for 23 insertions and one deletion. `dsh-router-board` changed six — the config surface lost a field, the reader lost its file access, and the README's configuration table, bearer section, and limitations list were rewritten to describe a config-carried credential instead of a discovered one. The recap for the previous session was corrected twice: once for a commit table that claimed two commits where one existed, and once to point at this recap rather than repeat it.

---

## Open questions / next steps

- **The tower's text panel is still unstyled beyond the spacing normalization.** The plan anticipated that "a later chunk may retire or restyle its text panel once these boards land." The seam migration made it share the boards' shell and spacing; a fuller restyle of its project rows is still open. This is now the only open item the fleet-dashboards plan left behind.

## Notes

The scoping error is the part worth remembering. "The package is unmodified in my tree" is a causal fact, and I used it as though it were a scoping justification. It is neither: the acceptance criteria I reported were measured against the fleet profile booting, so a profile with a crashing plugin meant I had validated a degraded system and called it green. The right test for scope is not "did I cause this" but "does the surface I just claimed works, work" — and the honest answer required booting the profile and reading the log rather than reading my own test results.

The roster investigation makes the same point from the other side. The instinct on finding a missing file is to find the right file. Here the file was never supposed to exist where the board looked, and the router serving the port was a different application entirely — so the defect was not a wrong path but a wrong dependency. Removing the file read was a smaller change than fixing the path and a strictly more correct one, because it cannot be wrong on a machine whose layout differs.

One process note: I ran `dsh fleet` to smoke the profile, which is a web profile, so it started a web server and tried to open a browser. It was killed and the only remaining listener from it is a session server that predates the session; nothing was left running.
