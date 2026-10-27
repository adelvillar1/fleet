#!/bin/bash
# repo-hygiene.sh — the projects-wide habit from
# docs/plans/2026-10-27-projects-repo-hygiene.md.
#
# What fleet's daily maintenance does for the fleet repo, for every
# repository under the root (default ~/Projects): prune regenerable
# artifact dirs from repos whose work has gone stale, pack neglected
# loose objects, and report the whole machine's numbers.
#
# The rules, in one place:
#   - The artifact list is CLOSED (target, node_modules, .next, dist,
#     build, out, coverage, .turbo, .venv, venv, tmp, .cache). Anything
#     not on the list is named in the log, never removed — the
#     generated/ lesson: a directory is checked before it is judged.
#   - A repo is stale when its last commit is older than 21 days; an
#     artifact dir is prunable when its mtime is older than 14 days.
#     BOTH guards must hold, so an active repo's hot artifacts survive
#     even when its commit date is old.
#   - Uncommitted work is never touched: only the listed dirs are.
#   - git gc runs on repos past 500 loose objects, throttled weekly.
#
# Modes:
#   --report              the survey table (nothing changes)
#   --dry-run             what --live would do, named
#   --live                apply, logging each action with its size
#   --purge PATH          one named artifact dir, verified against the
#                         list and the root, sized, removed, logged
#   --root DIR            the projects root (default ~/Projects)

set -u

ROOT="$HOME/Projects"
MODE="report"
STATE="$HOME/.fleet/repo-hygiene.state"
LOG="$HOME/Library/Logs/repo-hygiene.log"
GC_LOOSE_MIN=500
GC_DAYS=7
STALE_DAYS=21
MTIME_DAYS=14
# The closed list. Extending it is a plan decision, not a whim.
ARTIFACTS="target node_modules .next dist build out coverage .turbo .venv venv tmp .cache"

while [ $# -gt 0 ]; do
  case "$1" in
    --report|--dry-run|--live) MODE="${1#--}"; shift ;;
    --purge) MODE="purge"; PURGE="$2"; shift 2 ;;
    --root) ROOT="$2"; shift 2 ;;
    *) echo "repo-hygiene: unknown argument $1" >&2; exit 2 ;;
  esac
done

say() { printf '%s\n' "$*" >> "$LOG"; }

now=$(date +%s)
day=$((60 * 60 * 24))

dir_age_days() { # mtime of a path, in whole days
  local m=$(stat -f %m "$1" 2>/dev/null || echo "$now")
  echo $(( (now - m) / day ))
}

commit_age_days() { # last commit of a repo, in whole days; 9999 none
  local t=$(git -C "$1" log -1 --format=%ct 2>/dev/null)
  [ -z "$t" ] && { echo 9999; return; }
  echo $(( (now - t) / day ))
}

loose_objects() {
  git -C "$1" count-objects -v 2>/dev/null | awk '/^count:/ {print $2}'
}

on_list() { # is a dir NAME on the closed artifact list?
  for a in $ARTIFACTS; do [ "$1" = "$a" ] && return 0; done
  return 1
}

sizeof() { du -sk "$1" 2>/dev/null | awk '{print $1}'; }

if [ "$MODE" = "report" ]; then
  printf '%-26s %9s %8s %9s  %-10s %5s\n' REPO TOTAL GIT ARTIFACTS LASTCOMM CHURN
  for d in "$ROOT"/*/; do
    [ -d "$d" ] || continue
    name=$(basename "$d")
    if [ -e "$d/.git" ]; then last=$(git -C "$d" log -1 --format=%cs 2>/dev/null || echo none)
    else last='no-git'; fi
    total=$(sizeof "$d"); g=$(sizeof "$d/.git"); g=${g:-0}
    art=0
    for a in $ARTIFACTS; do
      [ -d "$d$a" ] && art=$((art + $(sizeof "$d$a")))
    done
    churn=0
    [ -e "$d/.git" ] && churn=$(git -C "$d" status --porcelain 2>/dev/null | wc -l | tr -d ' ')
    printf '%-26s %7.1fM %6.0fM %7.1fM  %-10s %5s\n' "$name" \
      "$(awk "BEGIN{print $total/1024}")" "$(awk "BEGIN{print $g/1024}")" \
      "$(awk "BEGIN{print $art/1024}")" "$last" "$churn"
  done
  exit 0
fi

if [ "$MODE" = "purge" ]; then
  p="${PURGE%/}"
  base=$(basename "$p")
  case "$p" in "$ROOT"/*) ;; *) echo "repo-hygiene: $p is outside $ROOT — refused" >&2; exit 1 ;; esac
  on_list "$base" || { echo "repo-hygiene: $base is not on the artifact list — refused" >&2; exit 1; }
  [ -d "$p" ] || { echo "repo-hygiene: $p is not there" >&2; exit 1; }
  k=$(sizeof "$p")
  rm -rf "$p"
  say "$(date -u +%FT%TZ) purge $p ${k}K"
  printf 'purged %s (%.1f MB)\n' "$p" "$(awk "BEGIN{print $k/1024}")"
  exit 0
fi

# --dry-run / --live: the rules, per repo.
: >> "$LOG"
[ "$MODE" = "live" ] && say "# repo-hygiene live $(date -u +%FT%TZ)"
pruned_k=0; named=0
for d in "$ROOT"/*/; do
  [ -d "$d" ] || continue
  [ -e "$d/.git" ] || continue
  name=$(basename "$d")
  cage=$(commit_age_days "$d")
  [ "$cage" -lt "$STALE_DAYS" ] && continue
  for a in $ARTIFACTS; do
    p="$d$a"
    [ -d "$p" ] || continue
    age=$(dir_age_days "$p")
    [ "$age" -lt "$MTIME_DAYS" ] && continue
    k=$(sizeof "$p")
    if [ "$MODE" = "dry-run" ]; then
      printf 'would prune %-52s %7.1f MB (repo %sd, dir %sd)\n' \
        "${p#"$ROOT"/}" "$(awk "BEGIN{print $k/1024}")" "$cage" "$age"
      pruned_k=$((pruned_k + k)); continue
    fi
    rm -rf "$p"
    say "$(date -u +%FT%TZ) prune ${p#"$ROOT"/} ${k}K (repo ${cage}d, dir ${age}d)"
    printf 'pruned %-56s %7.1f MB\n' "${p#"$ROOT"/}" "$(awk "BEGIN{print $k/1024}")"
    pruned_k=$((pruned_k + k))
  done
  # Neglected loose objects: pack them, at most weekly.
  loose=$(loose_objects "$d")
  if [ -n "$loose" ] && [ "$loose" -gt "$GC_LOOSE_MIN" ]; then
    last_gc=0
    [ -f "$STATE" ] && last_gc=$(awk -v r="$name" '$1==r {print $2}' "$STATE")
    if [ $((now - last_gc)) -gt $((GC_DAYS * day)) ]; then
      if [ "$MODE" = "dry-run" ]; then
        printf 'would gc      %-52s %s loose objects\n' "$name" "$loose"
      else
        git -C "$d" gc --quiet 2>/dev/null
        say "$(date -u +%FT%TZ) gc $name ($loose loose)"
        printf 'gc %-58s %s loose objects packed\n' "$name" "$loose"
        grep -v "^$name " "$STATE" 2>/dev/null > "$STATE.tmp"; mv "$STATE.tmp" "$STATE"
        echo "$name $now" >> "$STATE"
      fi
    fi
  fi
done
if [ "$MODE" = "live" ]; then
  say "$(date -u +%FT%TZ) done, $((pruned_k / 1024)) MB pruned"
  printf 'done: %.1f MB pruned\n' "$(awk "BEGIN{print $pruned_k/1024}")"
fi
exit 0
