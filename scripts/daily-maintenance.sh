#!/bin/sh
# daily-maintenance.sh — the fleet repo's own upkeep, unattended.
#
# Built 2026-10-26 after the owner asked why the repo collects garbage.
# Seven legs, each safe to run while nothing else is, each noisy only in
# its own log:
#
#   1. the cache sweep (scripts/cache-sweep.sh) — target/'s age-based
#      prune, release untouched; the 159 GiB incident's standing fix;
#   2. tmp/ retirement — entries older than FLEET_TMP_DAYS (default 14)
#      go, the same rule target/ got: scratch lands in tmp/ and now
#      tmp/ also lets go;
#   3. the corpus commit — the memories/ churn (recall counters,
#      supersede fills, the consolidation cycle's archives, new facts)
#      committed under its own label, memories/ paths ONLY, so the
#      tree stops reading dirty between sessions. Every corpus file is
#      written whole and guarded (the nomenclature write-guard), so any
#      instant snapshot is a complete fact — committing the corpus
#      daily cannot capture a half-thought;
#   4. the checkpoints' retention (fleet hygiene) — the one store
#      outside the repo that fleet itself grows: each session keeps its
#      last 20 turns under a 5 GiB cap. The face and the chat reclaim on
#      their own timers, so this leg is for a machine whose face is not
#      running (KI-478: 68 GB across 72 sessions, 54 GB of it in two);
#   5. the repo hygiene pass (scripts/repo-hygiene.sh --live) — the same
#      upkeep for every repository under ~/Projects: stale repos'
#      regenerable artifact dirs pruned by the closed list and the two
#      age guards, neglected loose objects packed weekly, everything
#      logged with sizes. C3 of docs/plans/2026-10-27-projects-repo-
#      hygiene.md;
#   6. the plan-worktree sweep — dry-run by default: --live stays the
#      owner's explicit act, which is the sweep's own rule;
#   8. the build lanes — /tmp's gate and lane target dirs, kept while warm
#      and cleaned whole when cold (2026-11-09: 80 GiB of lanes on a drive
#      with 1.6 GiB free);
#   9. the footprint — every store against the budget the owner declared,
#      measured and recorded for `fleet doctor` to read.
#   7. the orphan naming — untracked, non-ignored TOP-LEVEL entries are
#      named in the log and never deleted: a stray directory is a
#      question for the owner, not a file a script may remove.
#      (generated/ taught this: what read as an orphan was the corpus
#      refresh jobs' working driver, named checked-in in a memory fact.)
#
# Never fails the machine: every leg's failure is logged and the run
# exits 0, so a locked index or a busy corpus waits for tomorrow.

set -u

REPO="$(cd "$(dirname "$0")/.." && pwd)"
LOG="${FLEET_MAINTENANCE_LOG:-$HOME/Library/Logs/fleet-maintenance.log}"
TMP_DAYS="${FLEET_TMP_DAYS:-14}"

say() {
    printf '%s %s\n' "$(date -u '+%FT%TZ')" "$*" >>"$LOG"
}

# 1. the cache sweep — its own voice, its own guards.
say "— sweep —"
"$REPO/scripts/cache-sweep.sh" >>"$LOG" 2>&1 || say "the sweep failed; tomorrow retries"

# 2. tmp/ retirement — age-based, the scratch drawer lets go.
if [ -d "$REPO/tmp" ]; then
    before=$(du -sk "$REPO/tmp" 2>/dev/null | cut -f1)
    find "$REPO/tmp" -mindepth 1 -maxdepth 1 -mtime +"$TMP_DAYS" -exec rm -rf {} + 2>/dev/null
    after=$(du -sk "$REPO/tmp" 2>/dev/null | cut -f1)
    say "— tmp/ — ${before:-?}K -> ${after:-?}K (older than ${TMP_DAYS}d retired)"
fi

# 3. the corpus commit — memories/ paths only, the house's explicit-path
#    rule. A locked index or a concurrent write logs and waits.
cd "$REPO" || exit 0
if [ -n "$(git status --porcelain -- memories/ 2>/dev/null)" ]; then
    if git add -- memories/ 2>>"$LOG" && ! git diff --cached --quiet -- memories/ 2>>"$LOG"; then
        if git commit -m "chore(memories): the corpus' counters, archives and new facts" \
             -- memories/ >>"$LOG" 2>&1; then
            say "— corpus — the memories/ churn committed"
        else
            say "— corpus — the commit did not land (locked or refused); tomorrow retries"
        fi
    fi
fi

# 4. the checkpoints' retention — the one store outside the repo that
#    fleet itself grows, and the only one nothing else swept. The face
#    and the chat reclaim on their own timers; a machine whose face is
#    not running would otherwise wait for the owner to remember
#    (KI-478: 68 GB across 72 sessions, two of them holding 54 GB).
#    `fleet hygiene` is the shipped verb and its numbers are the caps.
say "— checkpoints —"
"$HOME/.fleet/bin/fleet" hygiene 2>&1 | sed 's/^/  /' >>"$LOG" || say "the checkpoint retention failed; tomorrow retries"

# 5. the repo hygiene pass — its own voice, its own log, its own guards.
say "— repo hygiene —"
"$REPO/scripts/repo-hygiene.sh" --live >>"$LOG" 2>&1 || say "the hygiene pass failed; tomorrow retries"

# 6. the plan-worktree sweep — dry-run by default, named in the log;
#    --live stays the owner's explicit act (the sweep's own rule).
say "— plan worktrees —"
"$REPO/scripts/plan-worktree.sh" sweep --dry-run >>"$LOG" 2>&1 || say "the worktree sweep failed; tomorrow retries"

# 8. the build lanes outside the repo — the release gate and the lane
#    builds keep their own target dirs in /tmp so a gate can never trample
#    the working tree's build. Nothing swept them: on 2026-11-09 the gate's
#    lane held 66 GiB and three others 13 GiB more, while the drive sat at
#    1.6 GiB free. A lane is a CACHE: kept while it is warm (a release week
#    reuses it, and a cold rebuild is ten minutes), cleaned whole once its
#    newest file is older than FLEET_LANE_KEEP_DAYS (default 3). A running
#    cargo owns its lane; the sweep skips it and says so.
say "— build lanes —"
for lane in /private/tmp/fleet-*-target; do
    [ -d "$lane" ] || continue
    case "$lane" in */fleet-gate-target|*/fleet-*-target) ;; *) continue ;; esac
    if pgrep -f "CARGO_TARGET_DIR=$lane" >/dev/null 2>&1; then
        say "  lane busy, skipped: $lane"
        continue
    fi
    # A lane whose last leg was red keeps its debris for the diagnosis
    # window only (FLEET_FAILED_GRACE_MIN, default 60): past it the files
    # are a record nobody will read (the owner's rule, 2026-11-09).
    if [ -f "$lane/.failed-build" ]; then
        age_min=$(( ( $(date +%s) - $(stat -f %m "$lane/.failed-build" 2>/dev/null || echo 0) ) / 60 ))
        if [ "$age_min" -lt "${FAILED_GRACE_MIN:-60}" ]; then
            say "  lane keeps a failed build (${age_min}m): $lane — kept for diagnosis"
            continue
        fi
    fi
    size_mib=$(du -sm "$lane" 2>/dev/null | cut -f1)
    # A cap as well as an age: a lane that grew to tens of gigabytes
    # inside the warm window is the incident this leg exists for, and
    # warmth must not shield it (2026-11-09: 69 GiB in the gate lane).
    if [ "${size_mib:-0}" -gt "$(( ${LANE_MAX_GIB:-20} * 1024 ))" ]; then
        say "  lane over the ${LANE_MAX_GIB:-20} GiB cap: $lane (${size_mib} MiB) — cleaned"
    elif [ -n "$(find "$lane" -type f -mtime -"${LANE_DAYS:-3}" 2>/dev/null | head -1)" ]; then
        say "  lane warm, kept: $lane ($(du -sh "$lane" 2>/dev/null | cut -f1))"
        continue
    fi
    before=$(du -sk "$lane" 2>/dev/null | cut -f1)
    ( cd "$REPO" && CARGO_TARGET_DIR="$lane" cargo clean --manifest-path Cargo.toml ) \
        >>"$LOG" 2>&1 && say "  lane cleaned: $lane (${before:-?}K)" \
        || say "  lane clean failed: $lane; tomorrow retries"
    rm -f "$lane/.failed-build" 2>/dev/null || true
done
# The gate's logs and the release notes are the diagnosis of one run; past a
# week they are history nobody reads (the owner's rule, 2026-11-09). The
# newest of each name is kept so the last failure is always readable.
for pattern in /tmp/gate-*.log /tmp/gate-*.exit /tmp/release-*.log; do
    for f in $pattern; do
        [ -f "$f" ] || continue
        find "$f" -mtime +"${GATE_LOG_DAYS:-7}" -delete 2>/dev/null &&
            say "  gate log pruned: $(basename "$f")"
    done
done

# The gate's own worktree, when it outlived its run: git owns the removal,
# and a worktree git no longer lists is a directory nobody owns.
if [ -d /private/tmp/fleet-release-gate ] &&
   ! git -C "$REPO" worktree list --porcelain 2>/dev/null | grep -q 'fleet-release-gate'; then
    git -C "$REPO" worktree prune >>"$LOG" 2>&1
    say "  a stale release-gate worktree is present: /private/tmp/fleet-release-gate (named, not removed)"
fi

# 9. the footprint — the budget table, measured and recorded. This is the
#    leg that makes the rest accountable: every store is named with the
#    budget the owner declared for it, and a store over its budget is a
#    failure the log carries (the writers are meant to keep it under; the
#    sweep is the fallback, not the plan).
say "— footprint —"
if [ -x "$REPO/scripts/footprint.sh" ]; then
    "$REPO/scripts/footprint.sh" --record > /tmp/fleet-footprint.log 2>&1
    if [ $? -eq 0 ]; then
        say "  every store is inside its budget ($(tail -1 /tmp/fleet-footprint.log))"
    else
        while IFS= read -r line; do say "  $line"; done < /tmp/fleet-footprint.log
    fi
fi

# 10. the orphan naming — read-only, always.
orphans=$(git ls-files --others --exclude-standard 2>/dev/null | cut -d/ -f1 | sort -u | tr '\n' ' ')
if [ -n "$orphans" ]; then
    say "— untracked at the top level, for the owner to name or ignore: $orphans"
fi

say "— done —"
exit 0
