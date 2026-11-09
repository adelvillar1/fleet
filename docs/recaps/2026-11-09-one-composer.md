# Session Recap — 2026-11-09: one composer, the app's column, the artifact in the centre

**Summary.** Four corrections in one afternoon, each the owner's, each shipped:
**the composer is the conversation's** — never a new one per surface; **the list
column is the app's** — the same one sessions and huddles ride, only the objects
change; **the artifact is the centre screen**, its properties and settings the
right pane; and **the welcome looks like the conversation's**, because the design
system already had that component. Shipped as 0.6.18 (burned — see below) and
0.6.19.

## The four rulings

- **"the composer component should remain identical to session, and huddles. why
  we have so many different composers"** — the bespoke `.pw-composer` is gone.
  The chat's own `<form id="composer">` is *moved* into the surface's column
  (`placeComposer`) and moved home when the route leaves; one element, so the
  behaviours cannot drift. Its **focus** changes per surface: the placeholder
  and the send target (the desk's iteration or brief, learning's lesson run).
  A red-proof shows the pins fail when the move is removed: the comment posts
  nothing, the learning answer posts nothing.
- **"the rail should be reused from the other surfaces like sessions … what
  changes is the objects shown on the rail"** — `.pw-rail` is gone. The desk and
  learning are two new tenants of the app's work column (`paintProjectColumn`):
  the same head, rows, fold and counts, with deliverables or lessons as the
  objects. A desk row carries the deliverable's **real** state
  (`1 refusal stands`, `all five stages done`, `not started`), and a project the
  server could not parse names itself in the list instead of vanishing.
- **"the deliverable goes on the center screen with properties and settings on
  the right pane"** — the centre is now the artifact: the desk's stage rail,
  claims, coverage, outline, review and end band, then its thread; learning's
  section then its thread. The pane carries **properties and settings** in the
  pane's own `card` / `facts` / `ghost` idiom: project, shape, sources, end gate,
  updated; the role, the practice, the sections.
- **"does this match the existing design system"** — the welcome is the
  conversation's `.empty` component (`.empty-eyebrow`, `h1`, `.empty-lede`,
  `.empty-meta`), not a new type scale; the pane is the design system's cards;
  the settings are `.ghost` buttons; the composer's own width now collapses its
  run bar through a container query rather than a second layout.

## The pins

The harness is **1080 checks across 134 cases**: the desk/learning surfaces
(`PWS`) now assert one composer (the element is in the dock, exactly one on the
page, and it comes home on the next route), the work column's groups and rows,
the row's state, the pane's properties, and the brief opening from the
composer's words. The desk's own `C1` walk covers every class the surface
paints. Three red-proofs: the composer's placement, the surface's send, and the
row's state.

## The burned release (for the record)

v0.6.18 was tagged at `15fa424`; a co-tenant's docs commit landed as `527767b`
seconds later, and `release.sh assets` — which requires HEAD to be the tag —
refused. The empty release then held `releases/latest`, so the updater's feed
404'd until it was marked a prerelease (KI-503). The content was cut again as
**0.6.19**, assets uploaded, feed read back anonymously (sha256 `387b8085ff87`),
the CLI updated in place and the face relaunched on 3080 — `/api/version`
answers 0.6.19.

## The disk (KI-502)

The owner's "fleet is eating my hard drive": the gate and lane build dirs in
`/tmp` are caches nothing swept — `/private/tmp/fleet-gate-target` alone held
**69 GiB**, with the drive at 1.6 GiB free. `cargo clean` per lane (93 GiB in
the gate lane alone) plus `scripts/cache-sweep.sh` on the repo (59 → 15 GiB)
took the volume from **1.6 GiB to 121 GiB free**. `daily-maintenance.sh` gains
leg 8: lanes are kept while warm (3 days) and cleaned whole once cold, and a
lane with a build in flight is skipped.

## Keeping the drive (the owner's next question: "how do we fix this for good?")

Cleaning once is not a fix. Four bounds now hold the caches, and one
observable names them:

1. **A floor before a build.** `scripts/release.sh` refuses to start either
   mode on a volume below `FLEET_DISK_FLOOR_GIB` (20), naming the numbers and
   the lanes — a build that dies mid-link on ENOSPC is the failure nobody can
   read.
2. **A cap on the gate's own lane.** The same preflight cleans the lane when it
   passes `FLEET_GATE_TARGET_MAX_GIB` (20), so it cannot grow back to 69 GiB.
3. **A size cap beside the age sweep.** `daily-maintenance.sh`'s lane leg now
   cleans a lane that is over the cap even inside the warm window; warmth must
   not shield the incident it exists for.
4. **The doctor says the number.** `fleet doctor` gains a `disk` line, judged
   against the same 20 GiB floor, naming the holders when it fails. It reads
   the volume through `statvfs` — not `df` — because a check that needs a
   binary on PATH reports a broken machine when the PATH is merely narrow
   (the install tests proved it inside the first gate run of this change).

Both script paths were smoked: the floor refusal names the numbers, the pass
prints them (`disk: 101 GiB free (floor 20)`), and the doctor answers
`disk ok 100 GiB free (floor 20 GiB)` on the shipped build.

## Does a 15 MB app need 29 GiB of target? (the owner's challenge)

No — and the measurement says why, in one number: **12.8 GiB of `target/` was
the same crate compiled ~95 times.** `libfleet-*.rlib` × 40 and
`libfleet_desktop-*.rlib` × 51, ~128 MB each, because `lto = true` embeds the
bitcode in every copy and cargo never evicts a superseded artifact — one new
copy per build and per cut, beside a 26 MB binary (the dmg ~10 MB). The rest
was the ordinary working set of a 1050-crate graph in two profiles.

Two legs close it, and the owner's rule names the third:

- **dedupe** (`cache-sweep.sh`): keep the newest `rlib`/`rmeta` per crate name
  *and extension*, drop the superseded compiles. First run: 12.8 GiB; `target`
  29 → 16 GiB, the volume 101.5 → 113.9 GiB free. The shipped binary, the
  bundle and the dependencies' current artifacts are never named.
- **clean_failed**: a red build's artifacts and incremental state are the
  diagnosis, so they are kept for `FLEET_FAILED_GRACE_MIN` (60) and removed
  after it — "there is no need to accumulate files for failed builds past
  diagnosis". `release-gate-run.sh` writes `.failed-build` on a red leg and
  clears it on a green one; the maintenance's lane leg honours the same marker.
  Smoked on a scratch lane: a fresh marker is kept and named, an old one
  cleans the directory whole.

The same line applies to a **successful** build: keep what the next build
reuses (dependencies' artifacts, the current build-script outputs, the
fingerprints, the incremental state — that is the difference between a
one-minute build and a ten-minute one), and drop what has finished its job.
That meant releasing the gate's lane the moment a cut is published (8 GiB the
working tree never reads), the abandoned staging dmgs (`rw.<pid>.fleet_*.dmg`,
78 MiB), the superseded build-script outputs (1.3 GiB of 342 dirs), the old
bundles (the newest two stay) and gate logs past a week.

So the honest ceiling: a release-only build of this graph is ~3 GiB, both
profiles plus incremental ~8–10 GiB, and everything above that is either a
duplicate or a corpse.

**How many copies to keep: two, not five.** One is right against history — a
unit's hash is its source plus its flags, so a changed crate mints a new hash
and the older copies can never be what the next build wants. But this repo
builds the same crate under more than one configuration, and each gets its own
hash and its own rlib: a plain `cargo build --release` and a `cargo tauri build`
left two different fleet-desktop rlibs (86 MB each, measured 2026-11-09), both
with a live fingerprint; dropping either costs a full LTO rebuild (3m18s
measured). So the rule is one copy per live configuration, and two is the
count this tree produces; five would be 640 MiB per crate per profile paying
for a third configuration that does not exist. `FLEET_KEEP_ARTIFACTS` sets it,
default 2 — and at 1 the sweep finds only 64 MiB more than at 2, which is the
size of the argument.

## Files changed

- `src/gui/assets/app.js` — `placeComposer` / `composerSurface` / `surfaceSend`
  / `setComposerFocus`, `paintProjectColumn` + `projectKidRow`, the desk and
  learning centres, `paintDeskProperties` / `paintLearningProperties`,
  `paintSurfaceDock` (the refusal only), `pwWelcome` on `.empty`.
- `src/gui/assets/index.html` — both panels are the conversation's `.column`
  with a stable dock bar.
- `src/gui/assets/app.css` — the column's board, the dock bar, the container
  query for the run bar; `.pw-rail`/`.pw-composer`/`.pw-welcome` deleted.
- `tests/face/harness.mjs` — the `PWS`, `LEARN` and `DESK` cases re-pointed at
  the column, the centre and the pane; three new pins.
- `scripts/daily-maintenance.sh` — leg 8, the build lanes.
- `src/doctor.rs`, `src/main.rs` — the `disk` check (`evaluate_disk`,
  `free_kib` via `statvfs`) and its pin.
- `scripts/release.sh` — `preflight_disk` (floor + lane cap) in both modes.
- Docs: `docs/features/desk.md`, `docs/features/learning.md`, `CHANGELOG.md`
  (0.6.18, 0.6.19, 0.6.20), `docs/known-issues.md` (KI-502, KI-503).

## Open questions / next steps

- The desk has **no deliverable on this machine**, so the open-deliverable
  layout was proven by the harness (`PWS the deliverable opens its work`, the
  centre's cards, the properties pane) rather than by a live press. The owner's
  first brief will show it.
- The pane auto-opens on both surfaces and switches to `the work` when the
  artifact is available; a co-tenant's later work could make that stickier
  (remember the owner's last tab per surface).
