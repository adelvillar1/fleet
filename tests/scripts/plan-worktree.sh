#!/usr/bin/env bash
# The plan-worktree machinery's smoke fixture
# (docs/plans/2026-11-07-plan-worktree-machinery.md, C4/C7's pin).
#
# Builds its own scratch git origin and main checkout in a temp dir —
# no network, no machine state, no shared worktree (the KI-426 lesson) —
# copies the script under test in, and drives the four verbs against
# it: the add's rules and disk guard, the remove's two refusals (dirty,
# unmerged), the status report, and the sweep's dry-run naming and
# live pruning with an unmerged lane held.

set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
FIX="$(mktemp -d /tmp/fleet-pw-fixture.XXXXXX)"
trap 'rm -rf "$FIX"' EXIT

fails=0
check() { # name expected actual
  if [ "$2" = "$3" ]; then :; else
    printf 'FAIL %s — expected [%s], got [%s]\n' "$1" "$2" "$3"
    fails=$((fails + 1))
  fi
}

say() { printf '%s\n' "$*"; }

cd "$FIX"
git init -q --bare origin.git
git init -q main
cd main
git config user.email fixture@example.com
git config user.name fixture
echo one > a.txt
git add .
git commit -qm init
git branch -M master
git remote add origin "$FIX/origin.git"
git push -q origin master
mkdir scripts
cp "$ROOT/scripts/plan-worktree.sh" scripts/plan-worktree.sh
chmod +x scripts/plan-worktree.sh
PW="$FIX/main/scripts/plan-worktree.sh"

# 1 — add: the worktree and branch land, the rules print, the free
# space is reported against the floor.
"$PW" add one >"$FIX/add.out" 2>&1
check "add lands the worktree" "0" "$([ -d "$FIX/fleet-one" ] && echo 0 || echo 1)"
check "add lands the branch" "0" "$(git show-ref --verify --quiet refs/heads/one && echo 0 || echo 1)"
check "add prints the absolute-path rule" "1" "$(grep -c 'absolute paths only' "$FIX/add.out")"
check "add reports the floor" "1" "$(grep -c 'floor' "$FIX/add.out")"

# 2 — the disk guard refuses with the numbers named.
"$PW" add two --floor 999999 >"$FIX/guard.out" 2>&1
check "the guard refuses" "1" "$?"
check "the guard names the numbers" "1" "$(grep -cE '[0-9]+ GiB free, the floor is 999999' "$FIX/guard.out")"

# 3 — status lists the lane.
"$PW" status >"$FIX/status.out" 2>&1
check "status names the lane" "1" "$(grep -c 'fleet-one' "$FIX/status.out")"
check "status reports the target size" "1" "$(grep -c 'target' "$FIX/status.out")"

# 4 — remove refuses while the tree is dirty, and while the branch is
# unmerged — nothing lost.
git -C "$FIX/fleet-one" commit -q --allow-empty -m "the lane's work"
echo dirty > "$FIX/fleet-one/dirty.txt"
"$PW" remove one >"$FIX/dirty.out" 2>&1
check "remove refuses the dirty tree" "1" "$?"
check "remove names the loss rule" "1" "$(grep -c 'uncommitted' "$FIX/dirty.out")"
rm -f "$FIX/fleet-one/dirty.txt"
"$PW" remove one >"$FIX/unmerged.out" 2>&1
check "remove refuses the unmerged branch" "1" "$?"
check "remove names the merge rule" "1" "$(grep -c 'not merged into master' "$FIX/unmerged.out")"

# 5 — a merged lane lands cleanly, and the removal reports the freed
# bytes.
git merge -q --ff-only one
"$PW" remove one >"$FIX/removed.out" 2>&1
check "remove succeeds when merged" "0" "$?"
check "remove reports the freed bytes" "1" "$(grep -cE 'freed' "$FIX/removed.out")"
check "the worktree is gone" "1" "$([ -d "$FIX/fleet-one" ] && echo 0 || echo 1)"

# 6 — the sweep: a merged stale lane's target prunes (dry-run names
# first, live performs), and an unmerged lane is held by name.
"$PW" add three >"$FIX/add3.out" 2>&1
STALE_DATE="$(date -v-22d +%Y-%m-%dT%H:%M:%S 2>/dev/null || date -d '22 days ago' +%Y-%m-%dT%H:%M:%S)"
GIT_AUTHOR_DATE="$STALE_DATE" GIT_COMMITTER_DATE="$STALE_DATE" \
  git -C "$FIX/fleet-three" commit -q --allow-empty -m stale
git merge -q --ff-only three
"$PW" sweep --dry-run >"$FIX/sweepdry.out" 2>&1
check "the dry-run names the stale lane" "1" "$(grep -cE 'would (remove|prune): fleet-three' "$FIX/sweepdry.out")"
"$PW" sweep --live >"$FIX/sweeplive.out" 2>&1
check "the live sweep removes the stale lane" "1" "$(grep -c 'removed: fleet-three' "$FIX/sweeplive.out")"
check "the stale lane is gone" "1" "$([ -d "$FIX/fleet-three" ] && echo 0 || echo 1)"

"$PW" add four >"$FIX/add4.out" 2>&1
# the hold's lane has a commit of its own — a lane sitting at master's
# tip is trivially merged, and merged-and-fresh is the live case
git -C "$FIX/fleet-four" commit -q --allow-empty -m "four's own work"
"$PW" sweep --live >"$FIX/held.out" 2>&1
check "the sweep holds the unmerged lane" "1" "$(grep -c 'held: fleet-four' "$FIX/held.out")"
check "the unmerged lane survives" "0" "$([ -d "$FIX/fleet-four" ] && echo 0 || echo 1)"

# 7 — the cycle's leg is the same command the maintenance script runs.
check "the maintenance leg is the sweep" "1" "$(grep -c 'sweep --dry-run' "$ROOT/scripts/daily-maintenance.sh")"

say ""
if [ "$fails" = 0 ]; then
  say "plan-worktree smoke: all checks green"
else
  say "plan-worktree smoke: $fails failure(s)"
fi
exit "$fails"
