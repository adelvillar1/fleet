#!/bin/sh
# scripts/footprint.sh — fleet's resource budget, declared and measured.
#
# The owner's rule (2026-11-09): fleet must be a RESPONSIBLE citizen, not a
# good one — never consuming a resource up to exhaustion and cleaning up
# afterwards. A budget is declared here, measured here, and enforced at the
# writes that can grow past it (release.sh's preflight, the maintenance, the
# sweep). Rescue is the fallback; the bound is the plan.
#
# usage:
#   scripts/footprint.sh            measure, print the table, exit 1 when over
#   scripts/footprint.sh --json     the same numbers as one JSON document
#   scripts/footprint.sh --record   also write ~/.fleet/footprint.json, which
#                                   `fleet doctor` reads (a doctor that walks
#                                   15 GiB on every call is not a doctor)
#
# The budgets are GiB and CLOSED: a new store gets a line here before it gets
# bytes on disk. Env overrides exist for a machine with a different shape
# (FLEET_BUDGET_TARGET, FLEET_BUDGET_LANE, FLEET_BUDGET_HOME).

set -u

REPO=$(cd "$(dirname "$0")/.." && pwd)
HOME_DIR="${FLEET_HOME:-$HOME/.fleet}"

TARGET_BUDGET="${FLEET_BUDGET_TARGET:-20}"   # this repo's target/
LANE_BUDGET="${FLEET_BUDGET_LANE:-20}"       # one /private/tmp build lane
LANES_TOTAL_BUDGET="${FLEET_BUDGET_LANES:-24}" # every lane together
HOME_BUDGET="${FLEET_BUDGET_HOME:-12}"       # ~/.fleet (checkpoints, venvs, sessions, bin)

MODE=text
case "${1:-}" in
  --json) MODE=json ;;
  --record) MODE=record ;;
  "") ;;
  -h|--help) sed -n '2,22p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
  *) echo "footprint.sh: unknown option $1" >&2; exit 2 ;;
esac

kib() { du -sk "$1" 2>/dev/null | awk '{print $1}'; }
gib1() { awk -v kb="${1:-0}" 'BEGIN { printf "%.1f", kb/1048576 }'; }

target_kb=$(kib "$REPO/target")
[ -n "$target_kb" ] || target_kb=0

lanes=""
lanes_kb=0
for lane in /private/tmp/fleet-*-target; do
  [ -d "$lane" ] || continue
  kb=$(kib "$lane")
  lanes="$lanes$lane ${kb:-0}
"
  lanes_kb=$((lanes_kb + ${kb:-0}))
done

home_kb=$(kib "$HOME_DIR")
[ -n "$home_kb" ] || home_kb=0

target_budget_kb=$((TARGET_BUDGET * 1048576))
lane_budget_kb=$((LANE_BUDGET * 1048576))
lanes_total_budget_kb=$((LANES_TOTAL_BUDGET * 1048576))
home_budget_kb=$((HOME_BUDGET * 1048576))

over=0
[ "$target_kb" -gt "$target_budget_kb" ] && over=1
[ "$lanes_kb" -gt "$lanes_total_budget_kb" ] && over=1
[ "$home_kb" -gt "$home_budget_kb" ] && over=1
while IFS=' ' read -r lane kb; do
  [ -n "${lane:-}" ] || continue
  [ "${kb:-0}" -gt "$lane_budget_kb" ] && over=1
done <<EOF
$lanes
EOF

if [ "$MODE" = "json" ] || [ "$MODE" = "record" ]; then
  out=$(mktemp)
  printf '{"measured":"%s","target_kib":%s,"target_budget_gib":%s,"lanes_kib":%s,"lanes_budget_gib":%s,"home_kib":%s,"home_budget_gib":%s,"over":%s}\n' \
    "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$target_kb" "$TARGET_BUDGET" "$lanes_kb" \
    "$LANES_TOTAL_BUDGET" "$home_kb" "$HOME_BUDGET" \
    "$([ "$over" -eq 1 ] && echo true || echo false)" > "$out"
  if [ "$MODE" = "record" ]; then
    mv "$out" "$HOME_DIR/footprint.json"
  else
    cat "$out"
    rm -f "$out"
  fi
  [ "$over" -eq 0 ]
  exit $?
fi

printf '%-28s %9s %9s %5s\n' store used budget verdict
printf '%-28s %9s %9s %5s\n' "----" "----" "------" "-------"
verdict() { [ "$2" -gt "$3" ] && echo "OVER" || echo "ok"; }
printf '%-28s %8s GiB %8s GiB %5s\n' "target/" "$(gib1 "$target_kb")" "$TARGET_BUDGET" "$(verdict x "$target_kb" "$target_budget_kb")"
while IFS=' ' read -r lane kb; do
  [ -n "${lane:-}" ] || continue
  printf '%-28s %8s GiB %8s GiB %5s\n' "$(basename "$lane")" "$(gib1 "${kb:-0}")" "$LANE_BUDGET" "$(verdict x "${kb:-0}" "$lane_budget_kb")"
done <<EOF
$lanes
EOF
printf '%-28s %8s GiB %8s GiB %5s\n' "lanes together" "$(gib1 "$lanes_kb")" "$LANES_TOTAL_BUDGET" "$(verdict x "$lanes_kb" "$lanes_total_budget_kb")"
printf '%-28s %8s GiB %8s GiB %5s\n' "~/.fleet" "$(gib1 "$home_kb")" "$HOME_BUDGET" "$(verdict x "$home_kb" "$home_budget_kb")"

if [ "$over" -eq 1 ]; then
  echo "footprint: a store is over its budget — the writers are meant to keep it under (see the header)" >&2
  exit 1
fi
echo "footprint: every store is inside its budget"
exit 0
