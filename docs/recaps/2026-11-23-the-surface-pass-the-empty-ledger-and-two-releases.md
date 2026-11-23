# Session Recap — 2026-11-23: the surface pass, an empty ledger, and two releases

**Summary.** One day, two releases. **v0.9.2** (`3a9b9cb3`) shipped the rail-as-map
and the drawer's bodies at the session's start. **v0.9.3** (`bb070183`) closes the
day with the surface pass finished (R1–R10), routing folded into Reviews (D3), the
known-issues ledger driven from seven open to **zero**, the face harness moved to
Node 26 (the V8 segfault's real fix), and a new tokens-by-project instrument built
end to end from an approved mockup. Gate: **2,824 Rust tests**, the face harness
**2,007 checks across 290 cases** on Node 26.11.1.

---

## The releases

- **v0.9.2 — the map and the drawer** (`3a9b9cb3`). The rail became a map of
  work-kind regions with Home at its head; the drawer opened each object's own
  body (code, tree, markdown, xlsx, pptx via the sandboxed vendored renderers);
  the four themes re-derived into Indigo Studio. Its changelog section is the
  full list.
- **v0.9.3 — the surface pass, the empty ledger, Node 26, tokens** (`bb070183`,
  tag pushed to gitea **and** github; the gated four-leg gate ran on a detached
  worktree of the tag's commit before the tag existed). Assets: ASSETS_TBD.

## The surface pass, finished (plan completed)

`docs/plans/2026-11-23-the-surface-pass.md` — R1–R10, every row shipped, with the
method and per-surface audit in `docs/architecture/2026-11-23-surface-audit.md`:

- **Connections is a knowledge page** (`8c131cac`) — the graph left the aside
  (it fought the drawer for the right edge); its nodes drill in place, its legend
  chips isolate a kind, and a skill's row opens its lens instead of dying. The
  drawer's third tab became `work` (`1b40ed0b`).
- **The grammar** — the read-only spine strip retired from six surfaces
  (`d976eba9`); a stat is a door onto the section that holds its population
  (`c4199e74`); empty and loading states name their one press, and **empty is not
  refused** was fixed at the bridge (`37d8009b`); one page head with the design
  system's tokens holding (`a905d8ed`); the router's chains flattened and the
  address told the truth (`0e19ee8d`); every div-button carries a cursor and
  answers the keyboard through one central rule (`393390cd`); a form inside a
  card is a section (`217d6e59`); the grid's spans are declared and measured with
  rails open/collapsed and the PWA at 390/820/844/1180 (`7df6d062`); armada's
  dead click-divs became buttons (`b728d517`).
- **Routing folded into Reviews (D3)** (`4108b60c` mockup, `82f2f691` code) — the
  standalone Router page retired; `#/router` folds silently onto
  `#/calibration/routing`, which is Reviews' first section and landing and
  carries the live decision, the declared chain (rows, not cards in a card),
  cost × quality, the workload mix and the day's route rows. The strip follows
  the selected section; a section that cannot paint names it where the owner
  looks.
- **The conversations list carries human work only** (`ed1fc28d`, `4b433fd8`) —
  system and scheduled sessions stay off the list, the archive and the graph.

## The known-issues burndown: seven open → zero

`docs/known-issues.md` now reads **593 rows, 0 open** (the checker's own count).

- **KI-560** (`7710a81c`) — re-enabling a model on the provider card used to
  *delete* it: the card saved the post-filter `models`. The read now publishes the
  declared `catalog_models` beside the enabled list, and the card saves that.
- **KI-562** — the reload-all sequence drives `writeSettings` (which now returns
  ok/refused/unreachable), holds its press for the whole run, counts per provider,
  and keeps its closing sentence in face state across the final repaint.
- **KI-566**, **KI-584**, **KI-592** (the landing under the open drawer) and
  **KI-594** (mock ports) closed with pins.
- **KI-595 — found by the gate itself**: the mixed-day budget test read a budget
  another parallel test had lowered mid-flight (`$0.01` is `0.005` to cents). It
  now holds the shared env lock and clears the seam; the green claim then waited
  for **two consecutive full runs**.
- **KI-593** — the V8 mark-compact segfault: **fixed by pinning the runtime**.
  Both harnesses run on Node 26 or newer and exit 2 below it, naming the binary;
  `tests/common::node()` resolves the runtime regardless of PATH order (this
  machine's `node` is 24.19.0); the `--no-concurrent-marking` workaround was
  removed on purpose so the gate exercises the default GC. Evidence: 20 unflagged
  full runs on 26.11.1, zero crashes. `FACE_PROGRESS=1` keeps a silent segv
  attributable.

## Tokens by project (plan completed)

`docs/plans/2026-11-23-tokens-by-project-plugin.md`, the mockup approved as drawn
(`6883a54e`), built (`953a5213`):

- **`plugins/tokens/`** — untrusted, asks only `cache`, group `system`, label
  `tokens`. One card per project: the donut sliced by the models that worked
  across **every session scoped to that project**, the centre the project's
  reported total, the legend below with each model's tokens and share (readable
  without hovering), the provider's full breakdown on hover or focus ("not
  reported" as words), and a strip naming the calls no session can claim:
  `unattributed`, `unbound`, `unknown session`.
- **The data seam** — `calib::RouteRow.session` (the conversation id, distinct
  from the derived HTTP `session_id`), stamped by `Prepared::record_route`;
  `session::workspace_of` resolves the project from the session's Start record (the
  archive's inference for old logs, marked `inferred`); `cache_report` answers
  `projects` from the same walk, its `models`/`totals` untouched.
- Measured on the bundle in WebKit at 1440/820/390 (document width equals the
  viewport, cards reflow, legend rows 44px); render in the pack at
  `docs/plans/mockups/2026-11-23-tokens-by-project/png/bundle-1440.png`.

## Numbers

- `cargo test`: **2,824 passed / 0 failed**; fmt, clippy (`-D warnings`) and the
  release build clean.
- Face harness: 2,007 checks across 290 cases, on Node 26.11.1.
- Ledger: 593 rows, **0 open**.
- One day's commits: 42 (from v0.9.2's cut through v0.9.3's).

## Doc updates

- **Applied**: `TECHNICAL-DOCUMENTATION.md` now names all three shipped bundles
  (telemetry, cache, tokens) in the plugin-host line; the surface-pass audit, the
  D3 pack and the T1 pack are committed under `docs/architecture/` and
  `docs/plans/mockups/`.
- **Deferred, proposed for `FUNCTIONAL-SPECIFICATIONS.md`** (owner's call):

  > **§ The surfaces.** Destinations in four groups plus Home, one flat level,
  > reachable from the rail or the ⌘K palette. Every destination shares one
  > grammar: a page head (the region's eyebrow, the destination's title, one
  > sentence of purpose), a strip of numbers that are **doors** onto the section
  > holding their population, a board of cards that each act, drill or explain,
  > and the drawer for context. The standalone router destination retired: routing
  > is **Reviews' first section**, and `#/router` folds onto it silently.
  >
  > **Beside the cache surface.** The **tokens** surface reads the same accounting
  > by project: one card per project, the donut sliced by the models that worked
  > across every session scoped to that project, the legend below with each
  > model's tokens and share, the provider's own breakdown on hover, and three
  > named buckets (`unattributed`, `unbound`, `unknown session`) for the calls no
  > session can claim — never guessed into a project.

## Notes

- The github mirror raced the master push (both remotes were already at
  `bb070183`); the tags pushed to both remotes cleanly.
- The release gate ran on a detached worktree and settled 4.97 GiB of gate
  artifacts; the assets preflight read 55 GiB free.
- `plan_check.py` still warns about two **2026-10-29** drafts (`turn-feedback`,
  `workspace-history-truth`) — untouched this session, and the owner's call.
  Three earlier plans remain active (kanzo-board, relevance-pass,
  armada-bridge-console).
- Node 26 is now a hard requirement for the harnesses: a bare
  `node tests/face/harness.mjs` refuses on this machine's 24.19.0 by design; the
  Rust gate resolves 26 itself.
- Install the new instrument per home: `fleet plugins install plugins/tokens`.

## Next steps

- Deploy v0.9.3 to the app when the owner wants it (quit → rm → cp → re-sign →
  open); the release assets are the source. Nothing was relaunched to verify.
- Decide the two 2026-10-29 drafts (close, resume, or cancel).
- Optional: report the v25 V8 crash upstream with the 290-context reproducer.
