#!/bin/sh
# daily-maintenance.sh — the fleet repo's own upkeep, unattended.
#
# Built 2026-10-26 after the owner asked why the repo collects garbage.
# Four legs, each safe to run while nothing else is, each noisy only in
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
#   4. the orphan naming — untracked, non-ignored TOP-LEVEL entries are
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

# 4. the orphan naming — read-only, always.
orphans=$(git ls-files --others --exclude-standard 2>/dev/null | cut -d/ -f1 | sort -u | tr '\n' ' ')
if [ -n "$orphans" ]; then
    say "— untracked at the top level, for the owner to name or ignore: $orphans"
fi

say "— done —"
exit 0
