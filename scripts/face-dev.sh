#!/bin/sh
# scripts/face-dev.sh — iterate on the face without paying the release ritual.
#
# The complaint this answers (2026-11-17): an icon change took half an hour.
# The measurement says the build is not the cost:
#
#   cargo build -p fleet            ~10 s   (a real content edit to app.js)
#   cargo build   (workspace)       ~44 s   (src-tauri is the other 34 s)
#   node tests/face/harness.mjs     ~17 s   (all 197 cases)
#   scripts/release.sh tag+assets   ~10-12 min  (two full gate builds + a bundle)
#
# So a face tweak is a ~10 s loop and a ~20 s check. The half hour was the
# RELEASE path being used as the DEV path. This script is the dev path.
#
#   scripts/face-dev.sh build          rebuild the fleet lib only + restart a
#                                      scratch face on :3099 (never the real one)
#   scripts/face-dev.sh check [pat]    run the face harness, optionally one case
#   scripts/face-dev.sh loop           build + restart + tail the face
#   scripts/face-dev.sh live           serve THIS TREE from the installed
#                                      binary — edit, save, refresh. No build.
#   scripts/face-dev.sh app            the desktop window without the DMG
#                                      (~1.7s, not ~3m40s)
#   scripts/face-dev.sh ship|dmg <tag> the full distribution path, when the
#                                      change is done and worth a release
#
# `live` is the one that answers the 2026-11-17 ruling: "if the release is
# not cut and the binary is not recreated we can afford failure." It runs
# whatever binary is installed, with FLEET_ASSETS_FROM pointed at this
# checkout, so the face you look at reads the files you edit. Zero build,
# zero reload, zero version bump — and a `kill` puts you back on the
# shipped face, which is why a mistake there is cheap.
#
# The scratch face (`build`/`loop`) runs the DEBUG binary with its own
# FLEET_HOME, so an experiment cannot touch ~/.fleet, the owner's caches,
# or the installed binary. Nothing here bumps a version or tags anything.
set -eu

REPO_ROOT=$(git rev-parse --show-toplevel)
PORT=${FACE_DEV_PORT:-3099}
HOME_DIR=${FACE_DEV_HOME:-/tmp/fleet-face-dev}
PIDFILE="$HOME_DIR/face.pid"
LOG="$HOME_DIR/face.log"

export PATH="$HOME/.cargo/bin:$PATH"

die() { echo "face-dev: $*" >&2; exit 1; }

build_face() {
  # -p fleet is the point: the workspace build pulls src-tauri in and pays
  # ~34 s for a desktop shell the face tests never touch.
  echo "== building the fleet lib (not the workspace: src-tauri is not needed)"
  cargo build -p fleet 2>&1 | tail -3
}

stop_face() {
  if [ -f "$PIDFILE" ]; then
    pid=$(cat "$PIDFILE" 2>/dev/null || true)
    [ -n "${pid:-}" ] && kill "$pid" 2>/dev/null || true
    rm -f "$PIDFILE"
  fi
  # The port is ours alone; anything there is a previous scratch face.
  if command -v lsof >/dev/null 2>&1; then
    lsof -nP -iTCP:"$PORT" -sTCP:LISTEN -t 2>/dev/null | while read -r other; do
      kill "$other" 2>/dev/null || true
    done
  fi
}

start_face() {
  [ -x "$REPO_ROOT/target/debug/fleet" ] || die "no debug binary yet — run: $0 build"
  mkdir -p "$HOME_DIR"
  stop_face
  echo "== starting the scratch face on 127.0.0.1:$PORT (home $HOME_DIR)"
  FLEET_HOME="$HOME_DIR" nohup "$REPO_ROOT/target/debug/fleet" gui \
    --port "$PORT" --no-open > "$LOG" 2>&1 &
  echo $! > "$PIDFILE"
  # Wait for it to answer, then say so — never a bare background spawn.
  tries=0
  while [ "$tries" -lt 40 ]; do
    if curl -fsS --max-time 2 "http://127.0.0.1:$PORT/api/version" >/dev/null 2>&1; then
      echo "== face live: http://127.0.0.1:$PORT  (log $LOG)"
      return 0
    fi
    tries=$((tries + 1))
    sleep 0.25
  done
  echo "== the face did not answer in 10s — last lines:" >&2
  tail -20 "$LOG" >&2 || true
  exit 1
}

check_face() {
  filter=${1:-}
  echo "== face harness${filter:+ (filter: $filter)}"
  if [ -n "$filter" ]; then
    # The harness runs every case by design; a filter narrows it so a
    # one-icon change does not pay the whole suite. FACE_ONLY is read below.
    FACE_ONLY="$filter" node "$REPO_ROOT/tests/face/harness.mjs"
  else
    node "$REPO_ROOT/tests/face/harness.mjs"
  fi
}

case "${1:-loop}" in
  build) build_face; start_face ;;
  check) shift || true; check_face "${1:-}" ;;
  loop)  build_face; start_face; echo "== watching $LOG (ctrl-c stops the tail, not the face)"; tail -f "$LOG" ;;
  stop)  stop_face; echo "== scratch face stopped" ;;
  live)
    # The cheapest loop there is: the installed binary, this tree, no build.
    # `stop` and `live` share the pidfile, so one verb always undoes the other.
    bin=${FACE_LIVE_BIN:-$HOME/.fleet/bin/fleet}
    [ -x "$bin" ] || die "no installed binary at $bin — build one, or set FACE_LIVE_BIN"
    [ -x "$REPO_ROOT/target/debug/fleet" ] || echo "== note: using $bin; it must be a build that reads FLEET_ASSETS_FROM"
    mkdir -p "$HOME_DIR"
    stop_face
    echo "== serving $REPO_ROOT from $bin on 127.0.0.1:$PORT — edit and refresh, no build"
    FLEET_ASSETS_FROM="$REPO_ROOT" FLEET_HOME="$HOME_DIR" \
      nohup "$bin" gui --port "$PORT" --no-open > "$LOG" 2>&1 &
    echo $! > "$PIDFILE"
    tries=0
    while [ "$tries" -lt 40 ]; do
      if curl -fsS --max-time 2 "http://127.0.0.1:$PORT/api/version" >/dev/null 2>&1; then
        echo "== live face: http://127.0.0.1:$PORT  (log $LOG)"
        echo "== an asset edit is served on refresh; \`$0 stop\` returns to the shipped face"
        exit 0
      fi
      tries=$((tries + 1))
      sleep 0.25
    done
    echo "== the live face did not answer in 10s — last lines:" >&2
    tail -20 "$LOG" >&2 || true
    exit 1
    ;;
  ship)
    shift || true
    tag=${1:-}
    [ -n "$tag" ] || die "ship needs the tag to cut, e.g. ship v0.8.16"
    echo "== the full ritual: gate on a detached worktree, tag, push, bundle, upload"
    echo "== (this is the 10-12 minute path — only run it when the change is DONE)"
    sh "$REPO_ROOT/scripts/release.sh" tag "$tag"
    sh "$REPO_ROOT/scripts/release.sh" assets "$tag"
    ;;
  app)
    # The desktop window, WITHOUT the DMG. Measured on this tree:
    #   cargo tauri build                3m40s  (bundle + DMG + signer)
    #   cargo tauri build --no-bundle    1.65s  (the .app, nothing else)
    # The 3m38s difference is packaging and updater signing — none of it
    # changes what the face does, so it belongs to distribution, not to
    # looking at the window. The Tauri shell is a thin wrapper over
    # `fleet::gui::serve_with`, the same server and the same assets the
    # browser serves; the PWA is the product and this is its frame.
    echo "== the desktop window (--no-bundle: no DMG, no signing)"
    cargo tauri build --no-bundle 2>&1 | tail -3
    APP=$(ls -d "$REPO_ROOT"/target/release/bundle/macos/*.app 2>/dev/null | head -1)
    [ -n "$APP" ] || die "no .app was built"
    echo "== opening $APP"
    open "$APP"
    ;;
  dmg)
    # Distribution only. This is the ~7-minute path: a release build, the
    # DMG, the updater signature, and (in a cut) the upload plus a feed
    # read-back. A batch decision, never a per-change one.
    tag=${2:-}
    [ -n "$tag" ] || die "dmg needs the tag to cut, e.g. dmg v0.8.16 (it is the release path)"
    echo "== the full distribution path — gate, tag, DMG, sign, upload"
    sh "$REPO_ROOT/scripts/release.sh" tag "$tag"
    sh "$REPO_ROOT/scripts/release.sh" assets "$tag"
    ;;
  *) die "unknown verb \`$1\` — build | check [pat] | loop | live | app | stop | ship|dmg <tag>" ;;
esac
