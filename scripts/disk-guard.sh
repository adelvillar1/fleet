#!/bin/sh
# disk-guard.sh — the data volume's self-defense, unattended-safe.
#
# Built 2026-11-19 after the drive hit 16 GiB free: a Kiri VM disk had
# grown to 61 GB of never-trimmed build-cache blocks beside sweeps that
# only watched their own stores. The guard's job: notice, prune what the
# owner has ruled prunable, and — only at the critical line — rebuild the
# Kiri disk sparse with its keeper data dumped and restored.
#
# Rulings this script enforces (owner, 2026-11-19):
#   - Kiri contents: layerfs-worker runs at all times; pampa-dev-postgres
#     stays; everything else (buildkit builders + their state volumes,
#     unused images) is prunable.
#   - The Kiri VM disk never trims itself — not by inner prunes, not by
#     a graceful shutdown. Over the critical line it is deleted and let
#     to regrow sparse; the engine's metadata lives outside the image, so
#     containers come back on restart and only pampa's data needs the
#     dump→restore round-trip.
#
# Thresholds (GiB free on the data volume; GUARD_* env overrides):
#   warn 40 — log only · act 25 — prune Kiri's insides · critical 15 —
#   rebuild the Kiri disk.
#
# Never fails the machine: logs to disk-guard.log, exits 0.
# `--dry-run` reports what it would do and touches nothing.

set -u

LOG="${GUARD_LOG:-$HOME/Library/Logs/disk-guard.log}"
WARN="${GUARD_WARN:-40}"
ACT="${GUARD_ACT:-25}"
CRITICAL="${GUARD_CRITICAL:-15}"
IMG="$HOME/Library/Application Support/Kiri/vm/data.img"
BACKUPS="$HOME/.fleet/kiri-guard"
DRY_RUN=0
[ "${1:-}" = "--dry-run" ] && DRY_RUN=1

say() { printf '%s %s\n' "$(date '+%Y-%m-%dT%H:%M:%S%z')" "$*" >>"$LOG"; }

free_gib() {
    df -k /System/Volumes/Data 2>/dev/null | awk 'NR==2 {print int($4/1048576)}'
}

allocated_gib() {
    du -sm "$IMG" 2>/dev/null | awk '{print int($1/1024)}'
}

kiri_usable() {
    command -v kiri >/dev/null 2>&1 || return 1
    kiri ps >/dev/null 2>&1 || return 1
    return 0
}

# The inner prune: everything the owner ruled prunable, nothing else.
# Builder containers first so their images and state volumes go with
# them; system prune keeps images of running containers and never
# touches volumes on its own.
kiri_inner_prune() {
    local builders
    builders=$(kiri ps -a --format '{{.Names}}' 2>/dev/null | grep '^buildx_buildkit_' | tr '\n' ' ')
    if [ -n "$builders" ]; then
        # shellcheck disable=SC2086
        kiri rm -f $builders >/dev/null 2>&1
        for name in $builders; do
            kiri volume rm "${name}_state" >/dev/null 2>&1
        done
        say "  pruned kiri builders: $builders"
    fi
    kiri volume prune -f >/dev/null 2>&1   # dangling volumes only (0-link)
    kiri system prune -a -f >/dev/null 2>&1
    say "  kiri inner prune done (builders, dangling volumes, unused images)"
}

# The critical-line rebuild: dump pampa, stop the engine, delete the
# image file, restart, restore. The engine's metadata survives outside
# the image, so containers reappear on their own; layerfs-worker is
# host-backed and needs nothing.
kiri_disk_rebuild() {
    local dump="$BACKUPS/pampa-$(date +%Y-%m-%d).sql"
    mkdir -p "$BACKUPS"

    say "  dumping pampa-dev-postgres"
    if ! kiri exec pampa-dev-postgres pg_dumpall -U postgres >"$dump" 2>/dev/null ||
       ! grep -q 'CREATE TABLE' "$dump"; then
        say "  REFUSING rebuild: the pampa dump is missing or empty — human decision required"
        return 1
    fi
    # keep the three newest dumps
    ls -t "$BACKUPS"/pampa-*.sql 2>/dev/null | tail -n +4 | while read -r old; do
        rm -f "$old"
    done

    say "  stopping the kiri engine"
    launchctl bootout "gui/$(id -u)/dev.kiri.daemon" >/dev/null 2>&1
    local waited=0
    while pgrep -f 'Kiri.app.*daemon' >/dev/null 2>&1 && [ "$waited" -lt 20 ]; do
        sleep 1; waited=$((waited + 1))
    done

    if [ -f "$IMG" ]; then
        say "  deleting the VM disk ($(allocated_gib) GiB allocated)"
        rm -f "$IMG" || { say "  REFUSING: the disk file would not delete"; return 1; }
    fi

    say "  restarting the engine"
    launchctl bootstrap "gui/$(id -u)" "$HOME/Library/LaunchAgents/dev.kiri.daemon.plist" >/dev/null 2>&1
    waited=0
    while ! kiri_usable && [ "$waited" -lt 30 ]; do
        sleep 1; waited=$((waited + 1))
    done
    if ! kiri_usable; then
        say "  REFUSING: the kiri engine did not come back — start it manually"
        return 1
    fi

    # keeper containers come back on their own (restart policy); wait for
    # pampa to accept queries, then restore.
    waited=0
    until kiri exec pampa-dev-postgres pg_isready -U postgres >/dev/null 2>&1 || [ "$waited" -ge 60 ]; do
        sleep 2; waited=$((waited + 2))
    done
    if ! kiri exec pampa-dev-postgres pg_isready -U postgres >/dev/null 2>&1; then
        say "  WARNING: pampa-dev-postgres is not ready after the restart — restore manually from $dump"
        return 1
    fi
    if kiri exec -i pampa-dev-postgres psql -U postgres -d postgres <"$dump" >/dev/null 2>&1; then
        say "  pampa restored from $dump"
    else
        say "  WARNING: pampa restore failed — the dump is preserved at $dump"
        return 1
    fi

    local up
    up=$(kiri ps --format '{{.Names}}' 2>/dev/null | tr '\n' ' ')
    say "  engine rebuilt; containers running: $up"
    [ -n "$(printf '%s' "$up" | grep -c layerfs-worker)" ] ||
        say "  WARNING: layerfs-worker is not running — start it manually"
    return 0
}

FREE=$(free_gib)
if [ -z "$FREE" ]; then
    say "could not read free space on the data volume"
    exit 0
fi

say "— disk guard — free ${FREE} GiB (warn ${WARN} / act ${ACT} / critical ${CRITICAL})"

if [ "$FREE" -ge "$WARN" ]; then
    say "  healthy; nothing to do"
    exit 0
fi

say "  below warn — checking the kiri disk"
ALLOC=$(allocated_gib)
if [ -n "$ALLOC" ] && [ "$ALLOC" -gt 20 ]; then
    say "  kiri VM disk: ${ALLOC} GiB allocated"
fi

if [ "$FREE" -ge "$ACT" ]; then
    say "  above act — no pruning yet"
    exit 0
fi

if [ "$DRY_RUN" -eq 1 ]; then
    say "  dry-run: would prune kiri builders/images; at critical ${CRITICAL} would rebuild the disk"
    exit 0
fi

if kiri_usable; then
    kiri_inner_prune
else
    say "  kiri engine not reachable — skipping the inner prune"
fi

FREE=$(free_gib)
say "  after prune: ${FREE} GiB free"
if [ "$FREE" -ge "$ACT" ]; then
    exit 0
fi

if [ "$FREE" -lt "$CRITICAL" ] && [ -f "$IMG" ]; then
    say "  below critical — rebuilding the kiri VM disk"
    if kiri_disk_rebuild; then
        say "  after rebuild: $(free_gib) GiB free"
    else
        say "  rebuild did not complete — see the warnings above"
    fi
fi

exit 0
