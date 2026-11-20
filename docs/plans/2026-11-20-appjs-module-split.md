---
status: draft
created: 2026-11-20
updated: 2026-11-20
slug: appjs-module-split
---

# Split `app.js` along state ownership

`src/gui/assets/app.js` is 31,986 lines / 1,392,784 bytes in one module,
and the hottest file in the repo (276 commits in 30 days). Splitting it is
possible today — the server already serves `core/store.js`,
`core/router.js`, `core/dom.js` as separate hash-versioned module scripts —
but it must follow the compression plan: every reply closes its connection
(`src/gui/server.rs:254-257`) and nothing is compressed, so each extra
module is another connection and, on the relay, another framed round trip.

**Sequenced after `asset-compression`.** Split first and the phone pays
for the split; compress first and the added requests are cheap.

## Context

- No build step may be introduced (hard rule 12, and `src/gui/mod.rs`'s
  own doc: "no framework, no build step"). Browser-native ES modules are
  the only tool.
- `app.js` holds face state globally; the cut has to follow state
  ownership, not line ranges, or the split becomes a second global layer.
- `tests/face/harness.mjs` drives ~188 cases / ~1,303 checks against the
  document — that is the behavioural net for the move, and it must pass
  unmodified.

## Approach

1. Record the pre-split baseline: request count and total bytes for a cold
   first load (localhost:3000 `/`), and the harness's pass count.
2. Draw the module boundary from state ownership, using the existing
   section banners and the `face.<x>` state object as the map: candidates
   are `state.js` (the `face` object and its persistence), `api.js` (fetch
   + SSE), `render/*.js` (painters per surface), `routes.js`, and
   `app.js` as the bootstrap that imports them. Names come from the
   surface vocabulary, not invention.
3. Move one module per commit behind native `import`/`export`; each module
   is served from the asset table in `server.rs` with the same
   `?v=ASSET_HASH` grammar.
4. After each move, run the harness case that covers the moved surfaces;
   after the last, the whole harness.
5. Measure the cold-load request count and bytes against the baseline and
   record both in the Status pass. The acceptance is *no worse than +5 %*
   on bytes with compression in place; the request count is reported, not
   capped (the compression plan is what makes it affordable).

## Acceptance Criteria

- [ ] The whole harness suite passes unmodified (1,303 checks at this
      plan's writing).
- [ ] Cold-load bytes ≤ baseline + 5 % with compression enabled; both
      numbers recorded.
- [ ] No build step, bundler, or new dependency: every module is served
      by `server.rs`'s asset table and loads via native `import`.
- [ ] No global is introduced that the pre-split code did not already
      have; the `face` state object remains the single owner of client
      state.
- [ ] Each moved surface's harness case is green in the commit that moves
      it.
- [ ] The full gate exits 0 and the binary-size delta is recorded.

## Files to be touched

- `src/gui/assets/app.js` → `src/gui/assets/app/*.js` + the bootstrap
- `src/gui/server.rs` — the asset table and the shell's script tags
- `src/gui/assets/index.html` — module entries (the shell is rewritten at
  serve time; confirm which of the two carries the tags)

## Out of scope

- Minification or bundling.
- Behaviour changes: this plan moves code, it does not fix it.
- The bot face's `bot.js` (a separate, smaller surface).

## Verification

1. The harness case per moved surface, then the whole suite.
2. Cold-load measurement before/after, with compression on.
3. A hand navigation of every rail destination in the live face.
4. The full four-leg gate once, at the end.

## Linked artifacts

- `docs/plans/2026-11-20-asset-compression.md` — the prerequisite.
- `docs/architecture/deep-dive-2026-11-20-fleet-state.md` — F5.
