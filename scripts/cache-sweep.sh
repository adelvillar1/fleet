#!/bin/sh
# The cache sweep the recap step runs.
#
# Cargo never deletes a stale artifact: every source change leaves the
# previous test binaries behind in target/debug/deps, and on 2026-10-21
# that directory held 159 GiB (668k files) and the gate died on
# ENOSPC. This prunes what a week of sessions no longer needs and keeps
# what they do:
#
#   1. files under target/debug older than FLEET_SWEEP_DAYS (default 7)
#      are deleted — cargo rebuilds anything it still wants, and a
#      week's incremental state survives so the next build stays fast;
#   2. if target/debug is still over FLEET_SWEEP_THRESHOLD_GB (default
#      15) — a heavy week fills it faster than a week ages it out —
#      yesterday and older go too, and only if today alone is still over
#      the line is the dev profile cleaned whole (the one-shot cargo
#      clean that freed the drive);
#   3. target/release is never touched — ~/.local/bin/fleet is a symlink
#      into it and the face's LaunchAgent runs that binary. Its *superseded
#      compilations* are a different thing and are pruned: with `lto` on,
#      each build mints its own ~128 MiB rlib of this crate, and a day of
#      cuts left 95 of them (12.8 GiB) beside a 26 MiB binary (KI-504). The
#      binary, the bundle and the dependencies' current artifacts stay.
#   4. a failed build's debris is kept for the diagnosis window
#      (FLEET_FAILED_GRACE_MIN, default 60) and cleaned after it: past the
#      diagnosis those files are a record nobody will read (the owner's
#      rule, 2026-11-09).
#
# A running cargo or rustc means someone's build is in flight; the sweep
# declines rather than deleting under it.
#
#   scripts/cache-sweep.sh                 this project's target
#   scripts/cache-sweep.sh --all           every ~/Projects/*/target
#   scripts/cache-sweep.sh --target <dir>  one target directory
#   scripts/cache-sweep.sh --dry-run       report, delete nothing
#   scripts/cache-sweep.sh --gate-lane     the release gate's cargo lane
#                                          (/tmp/fleet-gate-target) and the
#                                          cut's own logs — debris once the
#                                          cut is complete (KI-514)
#
# Prints one line per target directory and one total, in the terms that
# matter: the volume's free space before and after.

set -u

DAYS="${FLEET_SWEEP_DAYS:-7}"
THRESHOLD_GB="${FLEET_SWEEP_THRESHOLD_GB:-15}"
# Compiled artifacts kept per crate name and extension: TWO, for the two live
# configurations this repo actually builds.
#
# One would be enough against history — cargo's unit hash is source plus
# flags, so a changed crate mints a new hash and the older copies can never be
# what the next build wants. But the same crate is built under more than one
# configuration here, and each gets its own hash and its own rlib: a plain
# `cargo build --release` and a `cargo tauri build` produce two different
# fleet-desktop rlibs (measured 2026-11-09: 86 MB each, both with a live
# fingerprint), and switching between them costs a full LTO rebuild (3m18s
# measured). Keep the newest copy of each live configuration and nothing more;
# five copies would be 640 MiB per crate per profile paying for a third
# configuration that does not exist.
KEEP_ARTIFACTS="${FLEET_KEEP_ARTIFACTS:-2}"
DRY=0
ALL=0
GATE_LANE=0
ONLY=""
while [ $# -gt 0 ]; do
    case "$1" in
        --dry-run) DRY=1 ;;
        --all) ALL=1 ;;
        --target) shift; ONLY="${1:-}"; [ -z "$ONLY" ] && { echo "cache-sweep: --target needs a path" >&2; exit 2; } ;;
        --gate-lane) GATE_LANE=1 ;;
        -h|--help) sed -n '2,29p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
        *) echo "cache-sweep: unknown option $1" >&2; exit 2 ;;
    esac
    shift
done

here="$(cd "$(dirname "$0")/.." && pwd)"
if [ -n "$ONLY" ]; then
    targets="$ONLY"
elif [ "$ALL" -eq 1 ]; then
    targets="$(ls -d "$HOME"/Projects/*/target 2>/dev/null)"
else
    targets="$here/target"
fi

if pgrep -x cargo >/dev/null 2>&1 || pgrep -x rustc >/dev/null 2>&1; then
    echo "cache-sweep: a cargo or rustc process is running — a build is in flight, nothing swept"
    exit 0
fi

# One copy of each compiled artifact, not one per build (2026-11-09, KI-504).
# Cargo never evicts a superseded artifact: with `lto = true` every build
# writes its own ~128 MiB rlib of this crate into deps (the LTO bitcode rides
# inside), so a day of cuts left 40 copies of `libfleet-*` (4.2 GiB) and 51 of
# `libfleet_desktop-*` (4.0 GiB) beside a 26 MiB binary. The newest of each
# name and extension stays; the older ones are regenerable by the next build
# of that crate alone. The shipped binary, the bundle and the dependencies'
# artifacts are never named here.
dedupe_deps() {
    deps="$1"
    [ -d "$deps" ] || return 0
    for ext in rlib rmeta; do
        names=$(find "$deps" -maxdepth 1 -name "*.$ext" 2>/dev/null | sed 's#.*/##' \
            | sed -E "s/-[0-9a-f]{4,}\\.$ext$//" | sort -u)
        for name in $names; do
            # Newest first; everything after the head is a superseded compile.
            all=$(ls -t "$deps/$name"-*."$ext" 2>/dev/null)
            [ "$(printf '%s\n' "$all" | wc -l | tr -d ' ')" -gt "$KEEP_ARTIFACTS" ] || continue
            older=$(printf '%s\n' "$all" | tail -n +$((KEEP_ARTIFACTS + 1)))
            kept=$(printf '%s\n' "$all" | head -"$KEEP_ARTIFACTS" | tr '\n' ' ')
            bytes=$(printf '%s\n' "$older" | tr '\n' '\0' | xargs -0 du -ck 2>/dev/null | tail -1 | awk '{print $1}')
            echo "  $(basename "$kept"): dropping $(printf '%s\n' "$older" | wc -l | tr -d ' ') older ($(human "${bytes:-0}"))"
            [ "$DRY" -eq 1 ] || printf '%s\n' "$older" | tr '\n' '\0' | xargs -0 rm -f
            dedupe_freed_kb=$((dedupe_freed_kb + ${bytes:-0}))
        done
    done
}

# A failed build's debris, cleaned once the failure is diagnosed (the
# owner's rule, 2026-11-09): the artifacts of a red leg are the diagnosis, so
# they are kept for the grace window (FLEET_FAILED_GRACE_MIN, default 60
# minutes) and then removed — after that they are only files nobody will read
# again. A green run of the same leg clears the marker itself.
clean_failed() {
    dir="$1"
    marker="$dir/.failed-build"
    [ -f "$marker" ] || return 0
    age_min=$(( ( $(date +%s) - $(stat -f %m "$marker" 2>/dev/null || echo 0) ) / 60 ))
    if [ "$age_min" -lt "${FAILED_GRACE_MIN:-60}" ]; then
        echo "  a failed build is named in $marker (${age_min}m old): kept for diagnosis"
        return 0
    fi
    before_kb=$(du -sk "$dir" 2>/dev/null | awk '{print $1}')
    cargo_dir="$(dirname "$(command -v cargo 2>/dev/null || echo /nonexistent)")"
    if [ -d "$cargo_dir" ] && [ -f "$dir/../Cargo.toml" ]; then
        ( cd "$dir/.." && PATH="$cargo_dir:$PATH" CARGO_TARGET_DIR="$dir" cargo clean >/dev/null 2>&1 )
    fi
    # Whatever remains after cargo's own clean is this sweep's to take: it is
    # a marker named failure, not the shipped binary.
    [ -d "$dir" ] && rm -rf "$dir"
    rm -f "$marker" 2>/dev/null
    echo "  failed build cleaned after diagnosis: $(human "${before_kb:-0}") (${marker%/.failed-build})"
}

# Build-script outputs and bundles, kept the same way as artifacts: the
# newest per name is what the next build reuses; the older ones are outputs of
# a job that finished (2026-11-09: 342 dirs, 1.4 GiB across the two profiles,
# and dmgs from 0.4.0-beta.1 beside 0.6.20's).
prune_build_dirs() {
    dir="$1"
    [ -d "$dir" ] || return 0
    names=$(find "$dir" -maxdepth 1 -mindepth 1 -type d 2>/dev/null | sed 's#.*/##' \
        | sed -E 's/-[0-9a-f]{4,}$//' | sort -u)
    for name in $names; do
        all=$(ls -dt "$dir/$name"-* 2>/dev/null)
        [ "$(printf '%s\n' "$all" | wc -l | tr -d ' ')" -gt "$KEEP_ARTIFACTS" ] || continue
        older=$(printf '%s\n' "$all" | tail -n +$((KEEP_ARTIFACTS + 1)))
        bytes=$(printf '%s\n' "$older" | tr '\n' '\0' | xargs -0 du -ck 2>/dev/null | tail -1 | awk '{print $1}')
        [ "$DRY" -eq 1 ] || printf '%s\n' "$older" | tr '\n' '\0' | xargs -0 rm -rf
        dedupe_freed_kb=$((dedupe_freed_kb + ${bytes:-0}))
    done
}

prune_bundles() {
    dir="$1"
    [ -d "$dir" ] || return 0
    # A bundle that never finished leaves its staging dmg behind as
    # `rw.<pid>.fleet_<version>_<arch>.dmg` — a mount image for a build that
    # died, not a build worth keeping. An hour's grace keeps a bundle that is
    # still running untouched.
    staging=$(find "$dir" -maxdepth 1 -name 'rw.*.fleet_*.dmg' -mmin +60 2>/dev/null)
    if [ -n "$staging" ]; then
        bytes=$(printf '%s\n' "$staging" | tr '\n' '\0' | xargs -0 du -ck 2>/dev/null | tail -1 | awk '{print $1}')
        echo "  bundles: dropping $(printf '%s\n' "$staging" | wc -l | tr -d ' ') abandoned staging dmg(s) ($(human "${bytes:-0}"))"
        [ "$DRY" -eq 1 ] || printf '%s\n' "$staging" | tr '\n' '\0' | xargs -0 rm -f
        dedupe_freed_kb=$((dedupe_freed_kb + ${bytes:-0}))
    fi
    # Keep the newest two finished dmgs: the current cut and its predecessor,
    # which is what a re-cut or a rollback needs.
    all=$(ls -t "$dir"/fleet_*.dmg 2>/dev/null)
    [ "$(printf '%s\n' "$all" | wc -l | tr -d ' ')" -gt 2 ] || return 0
    older=$(printf '%s\n' "$all" | tail -n +3)
    bytes=$(printf '%s\n' "$older" | tr '\n' '\0' | xargs -0 du -ck 2>/dev/null | tail -1 | awk '{print $1}')
    echo "  bundles: dropping $(printf '%s\n' "$older" | wc -l | tr -d ' ') older dmg(s) ($(human "${bytes:-0}"))"
    [ "$DRY" -eq 1 ] || printf '%s\n' "$older" | tr '\n' '\0' | xargs -0 rm -f
    dedupe_freed_kb=$((dedupe_freed_kb + ${bytes:-0}))
}

free_kb() { df -k "$1" | awk 'NR==2 {print $4}'; }
human() { awk -v kb="$1" 'BEGIN { if (kb >= 1048576) printf "%.1f GiB", kb/1048576; else printf "%.0f MiB", kb/1024 }'; }

cargo_bin="$(command -v cargo 2>/dev/null || true)"
[ -z "$cargo_bin" ] && [ -x "$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin/cargo" ] \
    && cargo_bin="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin/cargo"

# The release gate's lane (KI-514): release.sh pins the gate's
# CARGO_TARGET_DIR to /tmp/fleet-gate-target, the worktree dies with the cut,
# and the lane survived — 12 GiB from one morning of cuts, named by the
# resource contract's curtail step. A lane is debris when no cargo or rustc
# is in flight and its newest file is past LANE_IDLE_MIN (10): a live cut
# rebuilds it constantly, so an idle lane is a finished one. The cut's own
# logs (gate-*, cut-*) ride the same rule — the record lives in the release
# and the ledger, not in /tmp.
sweep_gate_lane() {
    lane="/private/tmp/fleet-gate-target"
    [ -d "$lane" ] || { echo "gate-lane: absent — nothing to sweep"; return; }
    if pgrep -x cargo >/dev/null 2>&1 || pgrep -x rustc >/dev/null 2>&1; then
        echo "gate-lane: a build is in flight — declining (the sweep never deletes under cargo)"
        return
    fi
    newest=$(find "$lane" -type f -newermt "-${LANE_IDLE_MIN:-10} minutes" 2>/dev/null | head -1)
    if [ -n "$newest" ]; then
        echo "gate-lane: written within the last ${LANE_IDLE_MIN:-10} minutes — declining (a cut may be running)"
        return
    fi
    before_kb="$(du -sk "$lane" 2>/dev/null | awk '{print $1}')"
    if [ "$DRY" -eq 1 ]; then
        echo "gate-lane: $(human "$before_kb") would go (dry run)"
        return
    fi
    rm -rf "$lane"
    echo "gate-lane: $(human "$before_kb") dropped"
    logs=0
    for pattern in /private/tmp/gate-*.log /private/tmp/gate-*.txt /private/tmp/gate-*.exit \
                   /private/tmp/cut-*.log /private/tmp/fleet-3099.log /private/tmp/fleet-footprint.log; do
        [ -e "$pattern" ] || continue
        [ "$DRY" -eq 1 ] || rm -f "$pattern"
        logs=$((logs + 1))
    done
    [ "$logs" -gt 0 ] && echo "gate-lane: $logs cut log(s) dropped with it"
}

if [ "$GATE_LANE" -eq 1 ]; then
    sweep_gate_lane
    exit 0
fi

swept_any=0
dedupe_freed_kb=0
free_start=""
for target in $targets; do
    debug="$target/debug"
    [ -d "$debug" ] || continue
    [ -z "$free_start" ] && free_start="$(free_kb "$target")" && volume="$target"
    before_kb="$(du -sk "$debug" 2>/dev/null | awk '{print $1}')"
    stale="$(find "$debug" -type f -mtime +"$DAYS" 2>/dev/null | wc -l | tr -d ' ')"
    action="pruned $stale files older than ${DAYS}d"
    if [ "$DRY" -eq 1 ]; then
        echo "$(basename "$(dirname "$target")"): debug $(human "$before_kb"), $stale files older than ${DAYS}d would go (dry run)"
        dedupe_deps "$target/debug/deps"
        dedupe_deps "$target/release/deps"
        prune_build_dirs "$target/debug/build"
        prune_build_dirs "$target/release/build"
        prune_bundles "$target/release/bundle/dmg"
        prune_bundles "$target/release/bundle/macos"
        continue
    fi
    find "$debug" -type f -mtime +"$DAYS" -delete 2>/dev/null
    find "$debug" -type d -empty -delete 2>/dev/null
    after_kb="$(du -sk "$debug" 2>/dev/null | awk '{print $1}')"
    [ -z "$after_kb" ] && after_kb=0
    threshold_kb=$((THRESHOLD_GB * 1048576))
    if [ "$after_kb" -gt "$threshold_kb" ]; then
        # A heavy week fills the cache faster than a week ages it out
        # (the 159 GiB took six days), so the backstop is tiered: drop
        # yesterday and older first, which keeps today's incremental
        # state, and clean whole only when today alone is over the line.
        find "$debug" -type f -mtime +1 -delete 2>/dev/null
        find "$debug" -type d -empty -delete 2>/dev/null
        after_kb="$(du -sk "$debug" 2>/dev/null | awk '{print $1}')"
        [ -z "$after_kb" ] && after_kb=0
        action="$action; over ${THRESHOLD_GB} GiB, so yesterday and older went too"
    fi
    if [ "$after_kb" -gt "$threshold_kb" ]; then
        if [ -n "$cargo_bin" ] && [ -f "$target/../Cargo.toml" ]; then
            # `cargo clean --profile dev` is run through cargo so it also
            # drops the profile's metadata, but cargo needs `rustc` on
            # PATH to answer even that: under launchd's PATH it exits 101
            # and cleans nothing. The toolchain dir is put in front for
            # the one call, and the exit code is read rather than
            # discarded — the old leg reported "cleaned whole" whatever
            # happened, which is how a 40 GiB target survived a sweep
            # that claimed to have emptied it (KI-479).
            cargo_dir="$(dirname "$cargo_bin")"
            cleaned=0
            if (cd "$target/.." && PATH="$cargo_dir:$PATH" "$cargo_bin" clean --profile dev >/dev/null 2>&1); then
                cleaned=1
            fi
            if [ "$cleaned" -eq 0 ]; then
                # The fallback the empty-toolchain case needs: the files
                # are regenerable build output, and release is a separate
                # tree this never names.
                rm -rf "$debug"
            fi
            action="$action; still over, so the dev profile was cleaned whole"
        else
            rm -rf "$debug"
            action="$action; still over, so target/debug was removed whole"
        fi
        after_kb="$(du -sk "$debug" 2>/dev/null | awk '{print $1}')"
        [ -z "$after_kb" ] && after_kb=0
    fi
    echo "$(basename "$(dirname "$target")"): debug $(human "$before_kb") → $(human "$after_kb"); $action"
    dedupe_deps "$target/debug/deps"
    dedupe_deps "$target/release/deps"
    prune_build_dirs "$target/debug/build"
    prune_build_dirs "$target/release/build"
    prune_bundles "$target/release/bundle/dmg"
    prune_bundles "$target/release/bundle/macos"
    clean_failed "$target"
    swept_any=1
done

if [ "$swept_any" -eq 1 ] && [ -n "$free_start" ]; then
    echo "free space on the volume: $(human "$free_start") → $(human "$(free_kb "$volume")") (release binaries and bundles untouched)"
fi
[ "$dedupe_freed_kb" -gt 0 ] &&
    echo "superseded compilations dropped: $(human "$dedupe_freed_kb") across debug and release" || true
