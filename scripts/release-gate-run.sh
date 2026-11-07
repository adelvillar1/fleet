#!/bin/sh
# scripts/release-gate-run.sh — one cargo leg of scripts/release.sh's gate.
# usage: release-gate-run.sh build|test      (run at the repo root of the
# gated worktree; CARGO_TARGET_DIR is pinned by the caller)
# Writes /tmp/gate-<leg>-release.{log,exit}. The exit is read from the file
# (KI-465), never through a status-losing pipe.
set -u
leg="$1"
: "${REPO_ROOT:?caller sets REPO_ROOT}"
if [ "$leg" = "build" ]; then
  cargo build --all-targets > /tmp/gate-build-release.log 2>&1
  echo "EXIT=$?" > /tmp/gate-build-release.exit
elif [ "$leg" = "test" ]; then
  cargo test > /tmp/gate-lib-release.log 2>&1
  echo "EXIT=$?" > /tmp/gate-lib-release.exit
else
  echo "release-gate-run.sh: leg must be build or test" >&2
  exit 2
fi
