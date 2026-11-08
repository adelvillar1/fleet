# Changelog

All notable changes to fleet land here, one section per release, written at
cut time. The repo was born 2026-10-12; this changelog starts at v0.4.0 with
the first release — everything before it is `git log` (492 commits, each one
through the local gate).

**What beta means while the beta epoch runs:** this is the tool the owner
uses daily, not a promise of API stability. The tag is a real version
(`v0.5.0`); the version literal keeps its beta suffix (`0.5.0-beta.1`) until
the owner says otherwise, and the changelog header carries this paragraph
until then.

## v0.5.0 — 2026-11-08

The resource-responsibility release. The increment is a real one in this
repo's own terms: the harness had been keeping everything it ever wrote, and
a harness that consumes all available disk is not a useful tool. Two
resource defects were measured, root-caused and closed, and the compositor's
paste path landed beside them.

### Fixed — what the harness kept

- **KI-478 — the checkpoint retention never ran.** `checkpoints::prune` was
  bounded and tested from the day it was written; its *trigger* was a manual
  verb and a session's `GoalDone` event, so a face up for days without a goal
  set reached neither. `~/.fleet/checkpoints` stood at **68 GB** across 72
  sessions — two holding 54 GB between them, one at 512 turn directories
  against a documented keep of 20 — and the face's log held **zero** hygiene
  lines for its whole life. The retention now runs on a 15-minute timer for
  the life of the process that creates the debris (`gui::serve` and the
  chat), through a live-safe pass that never deletes a capture still writing
  and never takes the turn a rewind points at. A turn's clock orders it,
  never its id: a session's ids do not restart with the process, so the
  owner's store held turns 13..16 written an hour after turns 493..512 —
  ordering by id would have spared the oldest turn and deleted the live one.
  The daily maintenance script gained the leg for a machine whose face is
  not running.
- **KI-479 — the cache sweep had never once done its last tier.** It ran
  `cargo clean` under launchd's `PATH`, where `rustc` does not resolve, so
  cargo exited 101 and cleaned nothing while the script reported "the dev
  profile was cleaned whole". The exit code is now read, with an `rm -rf`
  fallback that needs nothing from a toolchain.
- **KI-477 — orphaned `mock-server` children** now die with the parent that
  held their stdin pipe, and the `target/debug/incremental` beside them was
  cleared (21 GB).

### Added

- The composer's clipboard paste, with the request caps raised to match what
  a real paste carries, and the send key's contrast fixed.

### Measured

Disk reclaimed on the owner's machine, each step verified: checkpoints
68 GB → **5.2 GB**, `target` 40 GB → **11 GB**, the volume's free space
**12 GiB → 116 GiB**. Memory was measured and left alone — every fleet
process together held 230 MB, with no leak to chase.

### Verification

The full gate ran green after the changes: **2,249 tests passed, 0 failed,
0 orphaned children**, `fmt --check` and `clippy --all-targets -D warnings`
clean, the ledger at 479 rows.

## v0.4.0 — 2026-11-06

The first GitHub release. The `0.3.0-beta` literal had been parked since the
repo's first weeks; the work below closed that out, so the release takes the
next stable minor number. Binaries: the native shell as a dmg and the CLI as
a tarball, both built from the tagged tree after the gate ran on it.

### Fixed — the open ledger, closed by class

The known-issues ledger went from **28 open rows to 5** at the closeout's
end (465 rows total); every row was re-verified against the code before it
was worked on, and one (KI-326) proved already fixed. The evening's own
scheduled corpus-refresh then found the KI-394 refusal shape a second time
and **KI-466** was filed, open — so this release's tree carries 6 open
rows, the new row naming its own fix-side choice rather than hiding the
regression.

- **KI-463** — a related-memories request built the whole embedding corpus
  synchronously on the request path (cold-corpus reads blocked the server);
  fixed so the request path reads the built caches and only the warm path
  builds, pinned by a source-shape test.
- **KI-464** — the telegram wait test waited on half its own assertion; the
  wait now watches both observables it asserts on.
- **KI-465** — a gate procedure that kept the summary and destroyed the
  failing names (`tail | grep`); the rule is now `> /tmp/<leg>.log 2>&1`
  with the exit read on the file — the release script's own gate runs this
  way.
- The nine classes the plan landed in one sweep: the nomenclature write
  guard, load-sensitive fixtures, the supersede/remember file routes,
  `reload` + delivery steps, child reports, account-state 400 failover, and
  plan hooks.

### Added

- **Releases.** `scripts/release.sh` gates HEAD on a detached worktree
  before it tags and cuts; `scripts/bump-version.sh` moves every version
  site in one step; this changelog is the release note, in one place.
- The first dmg deliverable, cut from the gated tree.

### Verification

The gate ran green on the committed release tree: **1,929 lib + 168
integration tests + 962 face-harness checks, 0 build warnings**, the ledger
and plan checker at exit 0.

### Install

See [the releases page](https://github.com/adelvillar1/fleet/releases/latest)
for the dmg and the CLI tarball, or
[build from the repo](https://github.com/adelvillar1/fleet#install).
