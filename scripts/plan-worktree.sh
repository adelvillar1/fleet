#!/usr/bin/env bash
# plan-worktree.sh — the plan-worktree machinery
# (docs/plans/2026-11-07-plan-worktree-machinery.md).
#
# The four verbs of the CLAUDE.md topology's plan-worktree block:
#
#   add    <slug> [--base <ref>] [--floor <GiB>]   start a plan's lane
#   remove <slug> [--force] [--push]               close it when the plan lands
#   status                                        the owner's read before deciding
#   sweep  [--dry-run] [--live]                   the hygiene cycle's pass
#
# Three rules the script exists to enforce, each printed at the moment
# it matters (add):
#
#   1. Absolute paths only — the tools' cwd resets to the main checkout
#      between calls, and a relative path silently edits the wrong tree.
#   2. The gate runs in the worktree's own target/ — never a shared
#      CARGO_TARGET_DIR (KI-446's mock-server breakage).
#   3. The deployment is per-machine, one session at a time.
#
# The closed list: only worktrees named fleet-<slug> beside the repo are
# this script's to manage (the repo-hygiene discipline — report unknowns,
# never touch them). Nothing is removed while it holds work: uncommitted
# changes or an unmerged branch refuse the removal without --force.
# The sweep prunes bytes, not history: target/ (regenerable by one
# build) freely on merged stale worktrees, and a whole worktree only
# merged AND 21-days stale, dry-run by default.

set -uo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PARENT="$(dirname "$REPO")"
FLOOR_DEFAULT=20
STALE_DAYS=21

say() { printf '%s\n' "$*"; }
die() { printf 'plan-worktree: %s\n' "$*" >&2; exit 1; }

# GiB free on the volume holding the parent — `df -k`'s fourth column is
# 1 KiB blocks available to an unprivileged user, the honest number.
free_gib() {
  df -k "$PARENT" 2>/dev/null | awk 'NR == 2 { printf "%.0f", $4 / 1048576 }'
}

worktree_of() { printf '%s/fleet-%s' "$PARENT" "$1"; }

# The closed list: sibling worktrees named fleet-*, the repo itself
# excluded by construction. Each line of `git worktree list --porcelain`
# carries `worktree <path>` first; the branch is `branch refs/heads/<b>`.
plan_worktrees() {
  git -C "$REPO" worktree list --porcelain | awk -v repo="$REPO" '
    /^worktree / { path = substr($0, 10) }
    /^branch / {
      ref = substr($0, 8); sub(/^refs\/heads\//, "", ref)
      n = split(path, parts, "/"); base = parts[n]
      # the slug, not the directory name — every consumer names the
      # branch (fleet-<slug> everywhere else would double the prefix)
      if (base ~ /^fleet-/ && path != repo) {
        sub(/^fleet-/, "", base); print base "\t" ref "\t" path
      }
    }'
}

merged_into_master() {
  git -C "$REPO" merge-base --is-ancestor "$1" master 2>/dev/null
}

last_commit_age_days() {
  local ct now
  ct="$(git -C "$REPO" log -1 --format=%ct "$1" 2>/dev/null)" || return 1
  now="$(date +%s)"
  echo $(( (now - ct) / 86400 ))
}

target_size() {
  local dir="$1/target"
  [ -d "$dir" ] || { echo 0; return; }
  du -sk "$dir" 2>/dev/null | awk '{ printf "%.1f", $1 / 1048576 }'
}

# ---------------------------------------------------------------- add

cmd_add() {
  local slug="" base="master" floor="$FLOOR_DEFAULT"
  while [ $# -gt 0 ]; do
    case "$1" in
      --base) base="$2"; shift 2 ;;
      --floor) floor="$2"; shift 2 ;;
      -*) die "add: unknown option '$1'" ;;
      *) [ -n "$slug" ] && die "add: one slug only"; slug="$1"; shift ;;
    esac
  done
  [ -n "$slug" ] || die "add: a slug names the plan (fleet-<slug>)"
  case "$slug" in *[!a-z0-9-]*) die "add: the slug is lowercase letters, digits, hyphens" ;; esac

  local free wt
  free="$(free_gib)"
  if [ "${free:-0}" -lt "$floor" ]; then
    die "add: ${free} GiB free, the floor is ${floor} GiB — a worktree's target costs 5–8 GiB.
     run '$0 sweep' to prune stale targets, or add --floor <GiB> to proceed knowingly."
  fi
  wt="$(worktree_of "$slug")"
  [ -e "$wt" ] && die "add: $wt already exists"

  local branch_exists=0
  git -C "$REPO" show-ref --verify --quiet "refs/heads/$slug" && branch_exists=1
  if [ "$branch_exists" = 1 ]; then
    git -C "$REPO" worktree add "$wt" "$slug" >/dev/null \
      || die "add: the worktree did not land"
  else
    git -C "$REPO" worktree add "$wt" -b "$slug" "$base" >/dev/null \
      || die "add: the worktree did not land"
  fi
  say "the lane: $wt  (branch $slug, base $base)"
  say "the guard: ${free} GiB free against the ${floor} GiB floor"
  say ""
  say "  1. absolute paths only — the tools' cwd resets to the main checkout"
  say "     between calls; a relative path edits the wrong tree"
  say "  2. the gate runs in the worktree's own target/ — never a shared"
  say "     CARGO_TARGET_DIR (KI-446)"
  say "  3. the deployment is per-machine, one session at a time"
  say ""
  say "in the lane: scripts/plan-worktree.sh status"
}

# -------------------------------------------------------------- remove

cmd_remove() {
  local slug="" force=0 push=0
  while [ $# -gt 0 ]; do
    case "$1" in
      --force) force=1; shift ;;
      --push) push=1; shift ;;
      -*) die "remove: unknown option '$1'" ;;
      *) [ -n "$slug" ] && die "remove: one slug only"; slug="$1"; shift ;;
    esac
  done
  [ -n "$slug" ] || die "remove: a slug names the plan"
  local wt; wt="$(worktree_of "$slug")"
  [ -d "$wt" ] || die "remove: no worktree at $wt"

  if [ "$force" = 0 ]; then
    local dirty; dirty="$(git -C "$wt" status --porcelain 2>/dev/null)"
    [ -n "$dirty" ] && die "remove: the worktree holds uncommitted changes — nothing is lost;
     commit them, or pass --force to remove it anyway"
    git -C "$REPO" show-ref --verify --quiet "refs/heads/$slug" || true
    if git -C "$REPO" show-ref --verify --quiet "refs/heads/$slug" && ! merged_into_master "$slug"; then
      die "remove: branch $slug is not merged into master — nothing is lost;
     land it, or pass --force to remove it anyway"
    fi
  fi

  local freed; freed="$(du -sk "$wt" 2>/dev/null | awk '{ printf "%.1f", $1 / 1048576 }')"
  git -C "$REPO" worktree remove --force "$wt" >/dev/null 2>&1 \
    || die "remove: the worktree did not go"
  git -C "$REPO" branch --delete --force "$slug" >/dev/null 2>&1 || true
  [ "$push" = 1 ] && git -C "$REPO" push origin --delete "$slug" >/dev/null 2>&1 || true
  say "removed: fleet-$slug — ${freed} GiB freed"
}

# -------------------------------------------------------------- status

cmd_status() {
  local free; free="$(free_gib)"
  say "free: ${free} GiB   floor: ${FLOOR_DEFAULT} GiB   stale: ${STALE_DAYS} days"
  say ""
  local found=0
  while IFS=$'\t' read -r name branch path; do
    [ -n "$name" ] || continue
    found=1
    local merged age ahead size
    merged="unmerged"; merged_into_master "$branch" && merged="merged"
    age="$(last_commit_age_days "$branch")" || age="?"
    ahead="$(git -C "$REPO" rev-list --count "master..$branch" 2>/dev/null)"
    size="$(target_size "$path")"
    say "fleet-$name"
    say "  branch $branch · $merged · $ahead commit(s) ahead of master"
    say "  last commit ${age}d ago · target ${size} GiB"
  done < <(plan_worktrees)
  if [ "$found" = 0 ]; then
    say "no plan worktrees — the lanes are closed"
  else
    say ""
    local unknown
    unknown="$(git -C "$REPO" worktree list --porcelain | awk -v repo="$REPO" '
      /^worktree / { path = substr($0, 10) }
      /^branch / {
        n = split(path, parts, "/"); base = parts[n]
        if (base !~ /^fleet-/ && path != repo) print path
      }')"
    [ -n "$unknown" ] && say "not this script's business (named, never touched):" && say "$unknown" | sed 's/^/  /'
  fi
}

# --------------------------------------------------------------- sweep

cmd_sweep() {
  local dry=1
  while [ $# -gt 0 ]; do
    case "$1" in
      --live) dry=0; shift ;;
      --dry-run) dry=1; shift ;;
      *) die "sweep: unknown option '$1'" ;;
    esac
  done
  local acted=0
  while IFS=$'\t' read -r name branch path; do
    [ -n "$name" ] || continue
    local age; age="$(last_commit_age_days "$branch")" || continue
    local merged=0; merged_into_master "$branch" && merged=1
    if [ "$merged" = 1 ] && [ "${age:-0}" -gt "$STALE_DAYS" ]; then
      if [ -d "$path/target" ]; then
        if [ "$dry" = 1 ]; then
          say "would prune: fleet-$name/target (merged, ${age}d stale)"
        else
          rm -rf "$path/target" && say "pruned: fleet-$name/target (merged, ${age}d stale)"
        fi
        acted=1
      fi
      if [ "$dry" = 1 ]; then
        say "would remove: fleet-$name (merged, ${age}d stale) — '$0 remove $name'"
      else
        git -C "$REPO" worktree remove --force "$path" >/dev/null 2>&1 \
          && git -C "$REPO" branch --delete --force "$name" >/dev/null 2>&1 \
          && { say "removed: fleet-$name (merged, ${age}d stale)"; acted=1; }
      fi
    else
      [ "$merged" = 0 ] && say "held: fleet-$name (unmerged — the plan is still its own)"
    fi
  done < <(plan_worktrees)
  if [ "$acted" = 0 ]; then
    [ "$dry" = 1 ] && say "nothing stale — the lanes are all live" || say "swept; nothing needed doing"
  fi
}

# --------------------------------------------------------------- main

case "${1:-}" in
  add) shift; cmd_add "$@" ;;
  remove) shift; cmd_remove "$@" ;;
  status) shift; cmd_status "$@" ;;
  sweep) shift; cmd_sweep "$@" ;;
  ""|-h|--help)
    sed -n '2,20p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
    ;;
  *) die "unknown verb '${1:-}' — try: add, remove, status, sweep" ;;
esac
