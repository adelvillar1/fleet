# Session Recap — 2026-11-23: the surface pass, an empty ledger, and two releases

**Summary.** One day, **three releases**. **v0.9.2** (`3a9b9cb3`) shipped the
rail-as-map and the drawer's bodies at the session's start. **v0.9.3** (`bb070183`)
closed the day with the surface pass finished (R1–R10), routing folded into
Reviews (D3), the known-issues ledger driven from seven open to **zero**, the face
harness moved to Node 26 (the V8 segfault's real fix), and a new tokens-by-project
instrument built end to end from an approved mockup. **v0.9.4** (`e2d26d41`)
followed within the hour, after the owner reported the conversations list still
showing non-human sessions and the tokens plugin missing: **every session is now
tagged to a user or the system at birth**, the classifier under the old logs was
tightened against the real archive, the plugin is installed, and the deployed face
answers 0.9.4. Gate: **2,826 Rust tests**, the face harness **2,007 checks across
290 cases** on Node 26.11.1.

---

## The releases

- **v0.9.2 — the map and the drawer** (`3a9b9cb3`). The rail became a map of
  work-kind regions with Home at its head; the drawer opened each object's own
  body (code, tree, markdown, xlsx, pptx via the sandboxed vendored renderers);
  the four themes re-derived into Indigo Studio. Its changelog section is the
  full list.
- **v0.9.4 — ownership at birth, and the harness's traffic out of the list**
  (`e2d26d41`, tag pushed to both remotes, gate green on a detached worktree;
  assets: `fleet_0.9.4_aarch64.dmg`, the CLI tarball, the raw CLI feed artifact,
  the signed updater + `.sig`, `latest.json` naming 0.9.4, cli sha `b6831fd5e4a5`).
- **v0.9.3 — the surface pass, the empty ledger, Node 26, tokens** (`bb070183`,
  tag pushed to gitea **and** github; the gated four-leg gate ran on a detached
  worktree of the tag's commit before the tag existed). Assets: `fleet_0.9.3_aarch64.dmg`
  (19.6 MB), `fleet-aarch64-apple-darwin.tar.gz` (16.1 MB), the raw CLI update
  artifact `fleet-aarch64-apple-darwin` (43.2 MB), the signed updater
  `fleet.app.tar.gz` + `.sig`, and `latest.json` — the feed names 0.9.3 and its
  cli row's sha256 begins `3741e03f0914`.

## The v0.9.4 hotfix — ownership at birth

The owner's report (same evening): *"conversations is still showing session that
were not human initiated"*, *"the token accounting plugin is not installed"*, and
then the ruling behind both: *"a session should be tagged to a user or system, some
sort of ownership needs to be there, this is critical when you onboard more
users."*

**Why it was still showing:** the deployed face was **0.9.2** — the human-only
filter (R3) and the tokens plugin's `projects` key both landed after that tag. The
fix was not live. But measuring the live archive showed a second layer: of the 120
rows in the window, 80 were classed "yours" and ~29 of those were verbatim harness
probes (`Reply with exactly…`, `Take one screenshot with computer_use…`,
`Append one line reading…`), which the text classifier did not catch.

**The structural fix (the owner's ruling):** `Start.initiator` — `{kind:
user|system, id}` written at birth. The human surfaces (the face's conversations,
the TUI chat, breakout rooms, lessons) write `user` with the identity (`owner`
until a multi-user face supplies the authenticated id); scheduled runs, delegated
children and probes (`fleet run --probe`) write `system`. `work_class` reads the
tag **before any text rule**: a user's terse message stays theirs, and a
system-started run with a work-shaped message is never the owner's. A log written
before the field carries none, and the reader says so rather than inventing one.
The `id` is the slot multi-user onboarding filters on.

**The legacy fallback, tightened against the real archive:** the machinery matcher
now catches the shapes the archive actually carries (verbatim-reply instructions,
the computer_use/screenshot probes, "list the files using the list_dir tool",
"create the file … containing exactly", "append one line reading",
"do not call any tools", "run the shell command"/"run pwd", "probe:", "say hello" /
"Say ok."). Measured on the live window: **29 probe rows hidden, zero human rows
lost** — 80 "yours" → 50, nothing newly shown. The rule still needs a directive
opener *and* the harness vocabulary, so "list the conversations by age", "take a
look at the health rail", "create a plan…", "do a deep dive on the composer" and
the owner's warmups stay named; the known limits (a task-shaped statement about a
fixture such as `sample-board`, a bare shell command with no harness word) are in
the test.

**Pins:** the birth tag on every constructor (user for `create`/persona/room/lesson,
system for scheduled/subagent/probe); the summary carries it; an old log's Start
decodes with none; the class reads user → yours and system → background; the
legacy verbatim probe still classes `probe`; the bridge's sidecar-mark test now
uses a text-ambiguous probe, because the text-clear ones need no mark.

**The deploy, once:** the binary replaced at `~/.fleet/bin/fleet` (0.9.4), the
tokens bundle installed into the home and enabled, the LaunchAgent
`com.fleet.gui` restarted with `kickstart -k`. Live: `/api/version` 0.9.4; the
sessions window 88 rows — **48 yours**, 40 background; the plugin answers
`cache` with `projects` (today one `unattributed` bucket: 6 models, 370 calls —
the route rows written before the tag; project cards appear as new runs land).
The installed bundle's transition line says exactly that.

**A deploy lesson:** an in-place `cp` over the running binary's path kills the
next exec of it (SIGKILL 137 — macOS refuses the modified inode the face still
maps). Replace by **rename** (`rm` + `mv`), never by overwriting.

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

- `cargo test`: **2,826 passed / 0 failed**; fmt, clippy (`-D warnings`) and the
  release build clean.
- Face harness: 2,007 checks across 290 cases, on Node 26.11.1.
- Ledger: 593 rows, **0 open**.
- One day's commits: 45 (from v0.9.2's cut through v0.9.4's).

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
  artifacts; the assets preflight read 55 GiB free, and the assets step's own
  sweep then dropped 1.4 GiB of superseded compilations — release binaries and
  bundles untouched, 59 GiB free after.
- The local built binary answers `--version` from the packaged copy
  (`target/release/fleet`); nothing was relaunched to verify the app.
- `plan_check.py` still warns about two **2026-10-29** drafts (`turn-feedback`,
  `workspace-history-truth`) — untouched this session, and the owner's call.
  Three earlier plans remain active (kanzo-board, relevance-pass,
  armada-bridge-console).
- Node 26 is now a hard requirement for the harnesses: a bare
  `node tests/face/harness.mjs` refuses on this machine's 24.19.0 by design; the
  Rust gate resolves 26 itself.
- Install the new instrument per home: `fleet plugins install plugins/tokens`.

## Next steps

- **Multi-user**: the initiator's `id` is the slot; the face's authentication must
  supply it (human surfaces currently record `owner`), and the sessions document's
  `yours` count and the rail filter become per-viewer rather than per-home.
- Decide the two 2026-10-29 drafts (close, resume, or cancel).
- Optional: report the v25 V8 crash upstream with the 290-context reproducer.
