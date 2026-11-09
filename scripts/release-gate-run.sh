#!/bin/sh
# scripts/release-gate-run.sh — one cargo leg of scripts/release.sh's gate.
# usage: release-gate-run.sh build|test      (run at the repo root of the
# gated worktree; CARGO_TARGET_DIR is pinned by the caller)
# Writes /tmp/gate-<leg>-release.{log,exit}. The exit is read from the file
# (KI-465), never through a status-losing pipe.
set -u
leg="$1"
: "${REPO_ROOT:?caller sets REPO_ROOT}"
# A red leg leaves a MARKER beside its own artifacts (the owner's rule,
# 2026-11-09): a failed build's files are kept while the failure is being
# diagnosed and are not kept past it. cache-sweep.sh and the maintenance
# clean any directory carrying this marker once it is older than the grace
# (FLEET_FAILED_GRACE_MIN, default 60) — the diagnosis window — and a
# green run of the same leg removes it.
mark_failed() {
  printf 'leg=%s at=%s log=%s\n' "$1" "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$2" \
    > "${CARGO_TARGET_DIR:-.}/.failed-build" 2>/dev/null || true
}
clear_failed() { rm -f "${CARGO_TARGET_DIR:-.}/.failed-build" 2>/dev/null || true; }

if [ "$leg" = "build" ]; then
  cargo build --all-targets > /tmp/gate-build-release.log 2>&1
  rc=$?
  echo "EXIT=$rc" > /tmp/gate-build-release.exit
  [ "$rc" -eq 0 ] && clear_failed || mark_failed build /tmp/gate-build-release.log
elif [ "$leg" = "test" ]; then
  cargo test > /tmp/gate-lib-release.log 2>&1
  rc=$?
  echo "EXIT=$rc" > /tmp/gate-lib-release.exit
  [ "$rc" -eq 0 ] && clear_failed || mark_failed test /tmp/gate-lib-release.log
else
  echo "release-gate-run.sh: leg must be build or test" >&2
  exit 2
fi
