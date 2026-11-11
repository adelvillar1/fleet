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
#                                   15 GiB on every call is not a doctor),
#                                   and append the day's reading to
#                                   ~/.fleet/footprint-history.jsonl
#
# The STORES' budgets below are declared; the machine's own numbers — the
# reserve, the run budget, the turn cadence and the upkeep interval — are
# DERIVED, by `fleet footprint` from the history this script records (the
# resource contract's C10/C12). `--record` is what keeps that derivation fresh:
# skip it for a week and the appetite is inferred from a week-old machine.
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

# Memory pressure, the profile's other measured half (the plan's C10): how much
# of the machine's RAM is in use, in kibibytes, and how much it has. macOS
# answers from vm_stat — active + wired + compressed, the three buckets that are
# really holding memory; free and inactive pages are not pressure — and Linux
# from MemTotal/MemAvailable. A host that answers neither prints 0, which the
# reader treats as "not measured" rather than as an idle machine.
memory_kib() {
    if [ -r /proc/meminfo ]; then
        total=$(awk '/^MemTotal:/ {print $2}' /proc/meminfo)
        avail=$(awk '/^MemAvailable:/ {print $2}' /proc/meminfo)
        if [ -n "${total:-}" ] && [ -n "${avail:-}" ]; then
            printf '%s %s\n' "$((total - avail))" "$total"
            return
        fi
    fi
    page=$(vm_stat 2>/dev/null | awk -F'page size of ' '/page size of/ {split($2,a," "); print a[1]}')
    if [ -n "${page:-}" ]; then
        used=0
        for bucket in "Pages active" "Pages wired down" "Pages occupied by compressor"; do
            count=$(vm_stat 2>/dev/null | awk -v k="$bucket" '$0 ~ "^" k {gsub(/\./,"",$NF); print $NF}')
            used=$((used + ${count:-0}))
        done
        total_bytes=$(sysctl -n hw.memsize 2>/dev/null || echo 0)
        printf '%s %s\n' "$((used * page / 1024))" "$((total_bytes / 1024))"
        return
    fi
    printf '0 0\n'
}
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
    # The series the appetite is inferred from (2026-11-09): one line per
    # record, capped to the last 90 — the standing free share is a history,
    # and one good day after a cleanup must not read as a roomy machine.
    vol_kib=$(df -k "$REPO" | awk 'NR==2 {print $2}')
    free_kib=$(df -k "$REPO" | awk 'NR==2 {print $4}')
    # The memory half rides the same row: the profile is one history, so the
    # appetite and the pressure are never read from two series that disagree
    # about the day they describe.
    set -- $(memory_kib)
    mem_used_kib="${1:-0}"
    mem_total_kib="${2:-0}"
    printf '{"at":"%s","volume_kib":%s,"free_kib":%s,"mem_used_kib":%s,"mem_total_kib":%s}\n' \
      "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "${vol_kib:-0}" "${free_kib:-0}" \
      "${mem_used_kib:-0}" "${mem_total_kib:-0}" \
      >> "$HOME_DIR/footprint-history.jsonl"
    tail -n 90 "$HOME_DIR/footprint-history.jsonl" > "$HOME_DIR/footprint-history.tmp" \
      && mv "$HOME_DIR/footprint-history.tmp" "$HOME_DIR/footprint-history.jsonl"
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
