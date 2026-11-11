---
name: build-lanes-in-tmp-eat-the-drive
description: "2026-11-09 — \"fleet is eating my hard drive\": the /tmp gate+lane targets and the repo target held 152 GiB with 1.6 GiB free; 12.8 GiB of it was the SAME crate compiled ~95 times (lto rlibs cargo never evicts); fixed by cleanup + bounds (release preflight floor/lane cap, lane size cap, doctor disk line) + cache-sweep dedupe (keep TWO per crate/extension, one per live configuration), failed-build cleanup past diagnosis, and successful-build cleanup (gate lane released on publish, build dirs/bundles/logs) — 0.…"
type: project
date: 2026-11-11
basis: inferred
tier: durable
shape: []
phrases: []
missing: []
premises: []
recall-count: 0
superseded-by:
valid-until:
source: zcode:fleet:content:285983102772d99aaed2c496d57acdf6f320aad7347d3891e9bb9e2378839d34
---

Owner, 2026-11-09: *"we need to be more aggressive on the good citizen routine,
fleet is eating my hard drive like there is no tomorrow"* → *"how do we fix this
for good?"* → *"does an application that is less than 15MB in size need that
much tmp and target storage? convince me"* → *"if the session is
building/compiling and it fails those temp files need to be removed post
diagnosis, there is no need to accumulate files for failed build past
diagnosis."*

Measured: `/private/tmp/fleet-gate-target` **69 GiB**, `fleet-edit-target`
10.9 GiB, `fleet-master-gate-target` 5.2 GiB, `fleet-060-target` 847 MiB, the
repo's own `target/` 59 GiB — drive at **1.6 GiB free**.

**The measurement that answers "does a 15 MB app need 29 GiB?": no.** The
binary is 26 MB (dmg ~10 MB) against a 1050-crate graph, and **12.8 GiB of
`target/` was the same crate compiled ~95 times** — 40 copies of
`libfleet-*.rlib` and 51 of `libfleet_desktop-*.rlib` at ~128 MB each. Why:
`profile.release.lto = true` embeds the bitcode in EVERY rlib, and cargo never
evicts a superseded artifact, so each build and each of the day's cuts minted
a new copy. Honest ceiling: release-only ~3 GiB, both profiles + incremental
~8–10 GiB; everything above is a duplicate or a corpse.

**The cleanup:** `cargo clean` per lane through `CARGO_TARGET_DIR` (93 GiB from
the gate lane alone — NOT `rm -rf`: the destructive-command gate blocks it, and
cargo owns its own dirs), then `scripts/cache-sweep.sh` (59 → 15 GiB) →
**121 GiB free**.

**What "for good" means — every cache gets a bound where it is created, and a
name in the surface the owner reads** (0.6.20):

1. `scripts/release.sh` **`preflight_disk`** in tag AND assets mode: refuses a
   volume under `FLEET_DISK_FLOOR_GIB` (20) with the numbers (a build that dies
   mid-link on ENOSPC is the failure nobody can read), and `cargo clean`s the
   gate lane past `FLEET_GATE_TARGET_MAX_GIB` (20).
2. `daily-maintenance.sh` **leg 8**: a `/private/tmp/fleet-*-target` lane is
   kept while warm (`FLEET_LANE_KEEP_DAYS`, 3) but cleaned whole when ALSO over
   `LANE_MAX_GIB` — warmth must not shield the incident it exists for; a lane
   with a running cargo is skipped by name.
3. **`fleet doctor` reports the disk** against the same floor, naming the
   holders on failure; it reads `statvfs` (libc), not `df` (see
   [[a-preflight-that-needs-a-binary-lies-about-a-narrow-path]]).
4. `cache-sweep.sh` **dedupe** (KI-504): keeps `FLEET_KEEP_ARTIFACTS` copies —
   **default 2** — of each `rlib`/`rmeta` per crate name AND extension in BOTH
   `debug/deps` and `release/deps`, dropping superseded compiles — 12.8 GiB on
   the first run, target 29 → 16 GiB. **Why 2 and not 1 or 5** (the owner's
   follow-up "do we need more than 5 builds?" → "convince me"): a unit hash is
   source+flags, and this repo has TWO live configurations — plain
   `cargo build --release` and `cargo tauri build` produce different
   fleet-desktop rlibs (86 MB each, both with a live fingerprint), and debug
   has the plain lib beside the `--cfg test` lib; dropping either costs a full
   LTO rebuild (3m18s measured). A third configuration does not exist, so five
   copies would pay 640 MiB per crate per profile for nothing — at keep=1 the
   sweep found only **64 MiB** more than at keep=2, which is the whole size of
   the argument. The shipped binary, the bundle and dependencies' current
   artifacts are never named.
5. `cache-sweep.sh` **clean_failed** (KI-504, the owner's rule): a red build's
   debris is kept for the diagnosis window `FLEET_FAILED_GRACE_MIN` (60 min)
   and removed after it — past the diagnosis those files are a record nobody
   will read. `release-gate-run.sh` writes `.failed-build` (leg, UTC time, log
   path) on a red leg and clears it on a green one; the maintenance's lane leg
   honours the same marker. Smoked: a fresh marker is kept and named, an old one
   cleans the directory whole.

**The same line for a SUCCESSFUL build** (the owner's next question: *"is there
a reason to keep temp files for successful build?"*) — keep only what the NEXT
build reuses (dependency artifacts, current build-script outputs, fingerprints,
incremental state; deleting those turns a 1-min build into a 10-min one), drop
everything whose job has finished. That meant six, all landed at `0ba3b44`:

- **`release.sh` releases the gate lane the moment a cut is published** — a
  detached worktree's private target dir the working tree never reads; 8 GiB
  freed, `FLEET_KEEP_GATE_LANE=1` opts back in (the trade: the next cut starts
  cold, ~10 min).
- `cache-sweep.sh` **prune_build_dirs**: newest `FLEET_KEEP_ARTIFACTS` per
  `build/<name>-<hash>` — 342 dirs, 1.4 GiB → release/build 1.0 GiB → 95 MiB,
  debug/build 391 → 55 MiB.
- `cache-sweep.sh` **prune_bundles**: abandoned staging mounts
  (`rw.<pid>.fleet_*.dmg`, 78 MiB of 0.4.0-beta.1/0.6.3 leftovers) go after an
  hour; finished dmgs keep the newest TWO (the current cut and its predecessor).
  Note the glob: `fleet_*.dmg` does NOT match `rw.<pid>.fleet_*.dmg`.
- `daily-maintenance.sh` prunes `/tmp/gate-*.log`, `/tmp/gate-*.exit`,
  `/tmp/release-*.log` past `GATE_LOG_DAYS` (7).

After: `target` 16 → 15 GiB, free 114 GiB. The principle worth keeping: **a
build artifact is a cache only while the next build would pick it up**; past
that instant it is a corpse, and the success/failure of the build that made it
is irrelevant — successful cuts leave *more* (bundles, lanes, staging mounts).

Smoke evidence: the floor refusal names the numbers, the pass prints them
(`== disk: 101 GiB free (floor 20); the gate lane is capped at 20 GiB`); the
doctor answers `disk ok 100 GiB free (floor 20 GiB)` on the shipped build.

**The escalation the owner then named (KI-505, `92d3979`): cleaning up well is a
GOOD citizen; a RESPONSIBLE one declares a budget and never crosses it.** So the
bounds above became one declaration — `scripts/footprint.sh` (target 20 GiB, a
lane 20, all lanes 24, `~/.fleet` 12; over = exit 1; `--record` writes
`~/.fleet/footprint.json`), audited by `daily-maintenance.sh` leg 9 and reported
by `fleet doctor` as `caches ok 17.0 GiB of 20 GiB (measured 0m ago)` from the
sidecar. The stale gate lane (8 GiB from an earlier cut) was released. See
[[a-budget-per-store-beat-clean-up-after]].

Related: [[checkpoints-unbounded-72g-disk]],
[[two-sessions-one-repo-master-worktree]],
[[the-release-is-the-owner-s-no-op]].

**2026-11-10 recurrence and the second repair (KI-514, `d2f4ef4`):** the
gate lane came back — three cuts in one morning re-accumulated **12 GiB**
in `/tmp/fleet-gate-target` plus 52 cut logs (owner: "since this morning
we have used up 30GiB of disk"), meaning the publish-time release above
did not cover the settle path the cuts actually run. Fixed twice over:
`cache-sweep.sh --gate-lane` (declines under live cargo/rustc and while
the lane was written in the last 10 minutes — a live cut rebuilds it
constantly; takes it otherwise, logs with it) and `release.sh`'s settle
now `rm -rf`s the lane itself, green-gate and post-assets. Swept: 12 GiB,
free space **124 GiB**. Two process facts confirmed: the harness's
destructive-command gate blocks session `rm -rf` outright **even with the
owner's explicit approval** — the sanctioned deletion path is extending
the house's own sweeper script, never a raw rm retry; and the scratch
NOT covered by any sweeper (old test homes, stale app copies, profile
dumps, ~0.3 GiB) stays named-but-present until reboot or an owner act.
