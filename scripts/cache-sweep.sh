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
#      into it and the face's LaunchAgent runs that binary.
#
# A running cargo or rustc means someone's build is in flight; the sweep
# declines rather than deleting under it.
#
#   scripts/cache-sweep.sh                 this project's target
#   scripts/cache-sweep.sh --all           every ~/Projects/*/target
#   scripts/cache-sweep.sh --target <dir>  one target directory
#   scripts/cache-sweep.sh --dry-run       report, delete nothing
#
# Prints one line per target directory and one total, in the terms that
# matter: the volume's free space before and after.

set -u

DAYS="${FLEET_SWEEP_DAYS:-7}"
THRESHOLD_GB="${FLEET_SWEEP_THRESHOLD_GB:-15}"
DRY=0
ALL=0
ONLY=""
while [ $# -gt 0 ]; do
    case "$1" in
        --dry-run) DRY=1 ;;
        --all) ALL=1 ;;
        --target) shift; ONLY="${1:-}"; [ -z "$ONLY" ] && { echo "cache-sweep: --target needs a path" >&2; exit 2; } ;;
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

free_kb() { df -k "$1" | awk 'NR==2 {print $4}'; }
human() { awk -v kb="$1" 'BEGIN { if (kb >= 1048576) printf "%.1f GiB", kb/1048576; else printf "%.0f MiB", kb/1024 }'; }

cargo_bin="$(command -v cargo 2>/dev/null || true)"
[ -z "$cargo_bin" ] && [ -x "$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin/cargo" ] \
    && cargo_bin="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin/cargo"

swept_any=0
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
            (cd "$target/.." && "$cargo_bin" clean --profile dev >/dev/null 2>&1)
            action="$action; still over, so the dev profile was cleaned whole"
        else
            rm -rf "$debug"
            action="$action; still over, so target/debug was removed whole"
        fi
        after_kb="$(du -sk "$debug" 2>/dev/null | awk '{print $1}')"
        [ -z "$after_kb" ] && after_kb=0
    fi
    echo "$(basename "$(dirname "$target")"): debug $(human "$before_kb") → $(human "$after_kb"); $action"
    swept_any=1
done

if [ "$swept_any" -eq 1 ] && [ -n "$free_start" ]; then
    echo "free space on the volume: $(human "$free_start") → $(human "$(free_kb "$volume")") (release targets untouched)"
fi
