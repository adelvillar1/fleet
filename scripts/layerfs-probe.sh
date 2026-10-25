#!/bin/bash
# layerfs-probe.sh — C0 of docs/plans/2026-10-25-layerfs-delegated-workspaces.md
#
# Walks this machine's LayerFS readiness as a ladder and stops at the first
# missing prerequisite, naming it; when the ladder is complete it times the
# whole loop — init, fork, sandbox create, mount, exec, commit, unmount,
# close, remove — and captures each command's raw JSON under the probe dir.
#
#   usage: scripts/layerfs-probe.sh [probe-dir]     (default: tmp/layerfs-probe)
#
# Reads (names only; nothing here prints a value that could be a secret):
#   docker on PATH and its engine reachable        — Sandboxes are containers
#   node >= 20 on PATH                             — the layerfs CLI's floor
#   layerfs on PATH, or FLEET_LAYERFS_BIN set      — the CLI under test
#   LAYERFS_WORKER_ENDPOINT or <LAYERFS_HOME>/worker/endpoint.json
#       — macOS Store calls ride the home's Linux worker container; the probe
#         never starts the worker (that needs the sealed bundle and the
#         runtime image), it reports how to provision one.
#   LAYERFS_PROJECT (default: fleet-probe)         — the scratch project name
#
# Exit codes: 0 the full loop ran; 2 a prerequisite is missing (named on
# stderr and in <dir>/result.json); 1 the loop ran and failed a step (the
# failing step and its captured JSON are in <dir>/result.json).

set -u

DIR="${1:-tmp/layerfs-probe}"
PROJ="${LAYERFS_PROJECT:-fleet-probe}"
SRC="$DIR/source"
LOG="$DIR/run.log"
RESULT="$DIR/result.json"
CLI="${FLEET_LAYERFS_BIN:-layerfs}"

mkdir -p "$DIR" "$SRC"
: >"$LOG"

shortfall() { # <stage> <prerequisite> <how-to-provision>
  printf 'SHORTFALL %s: %s\n  provision: %s\n' "$1" "$2" "$3" >&2
  printf '{"status":"shortfall","stage":"%s","missing":"%s","provision":"%s"}\n' \
    "$1" "$2" "$3" >"$RESULT"
  exit 2
}

step() { # <name> <json-capture> <cmd...>  → runs, times, appends to the log
  NAME="$1"; OUT="$2"; shift 2
  START=$(python3 -c 'import time;print(f"{time.time():.3f}")')
  "$@" >"$OUT" 2>"$OUT.err"
  CODE=$?
  END=$(python3 -c 'import time;print(f"{time.time():.3f}")')
  MS=$(python3 -c "print(f'{($END-$START)*1000:.0f}')")
  printf '%-14s exit=%-3s %sms\n' "$NAME" "$CODE" "$MS" >>"$LOG"
  if [ $CODE -ne 0 ]; then
    printf 'STEP-FAIL %s exit=%s (%sms)\n  stderr: %s\n' \
      "$NAME" "$CODE" "$MS" "$(head -c 400 "$OUT.err")" >&2
    printf '{"status":"step-failed","step":"%s","exit":%s,"ms":%s}\n' \
      "$NAME" "$CODE" "$MS" >"$RESULT"
    exit 1
  fi
  LAST_MS="$MS"
}

# ---- the ladder ------------------------------------------------------------
command -v docker >/dev/null 2>&1 \
  || shortfall ladder "docker on PATH" "install Docker Desktop; the engine must be running"
docker info >/dev/null 2>&1 \
  || shortfall ladder "a running Docker engine" "start Docker Desktop"

command -v node >/dev/null 2>&1 \
  || shortfall ladder "node on PATH" "install Node >= 20 (the layerfs CLI's floor)"
node -e 'process.exit(Number(process.versions.node.split(".")[0])>=20?0:1)' \
  || shortfall ladder "node >= 20" "upgrade node; the CLI refuses older"

command -v "$CLI" >/dev/null 2>&1 \
  || shortfall ladder "the layerfs CLI (FLEET_LAYERFS_BIN or PATH)" \
       "build it from a pinned layerfs checkout: npm install in api/, npm --prefix api run build:native, then link api/agent/cli/src/layerfs.js or set FLEET_LAYERFS_BIN"

HOME_DIR="${LAYERFS_HOME:-$PWD/$DIR/layerfs-home}"
mkdir -p "$HOME_DIR"
if [ -z "${LAYERFS_WORKER_ENDPOINT:-}" ] && [ ! -f "$HOME_DIR/worker/endpoint.json" ]; then
  case "$(uname -s)" in
    Darwin) shortfall ladder "the macOS worker endpoint" \
        "macOS Store calls ride the home's Linux worker container: build the sealed bundle (layerfs-daemon for Linux + worker/server.js + manifest.json) and the runtime image from your pinned checkout — the dsh-layerfs repo's Dockerfile builds both — then start it once via @layerfs/runtime and set LAYERFS_WORKER_ENDPOINT to $HOME_DIR/worker/endpoint.json" ;;
  esac
fi
export LAYERFS_HOME="$HOME_DIR"

# ---- the loop --------------------------------------------------------------
printf '# layerfs-probe  %s  cli=%s\n' "$(date -u +%FT%TZ)" "$("$CLI" --version 2>&1 | head -1)" >"$LOG"

# A scratch source tree: small, deterministic, one nested file.
printf '# layerfs probe\nprobe content v1\n' >"$SRC/README.md"
mkdir -p "$SRC/sub" && printf 'nested\n' >"$SRC/sub/file.txt"

step init        "$DIR/init.json"        "$CLI" --project "$PROJ" init "$PROJ" "$SRC"
step fork        "$DIR/fork.json"        "$CLI" --project "$PROJ" branch fork probe-work
step sandbox     "$DIR/sandbox.json"     "$CLI" --project "$PROJ" sandbox create
SANDBOX=$(python3 -c "import json;print(json.load(open('$DIR/sandbox.json'))['sandbox'])" 2>/dev/null) \
  || { echo "cannot read sandbox number from $DIR/sandbox.json" >&2; exit 1; }
step mount       "$DIR/mount.json"       "$CLI" --project "$PROJ" workspace mount "$SANDBOX" probe-work
WS=$(python3 -c "import json;print(json.load(open('$DIR/mount.json'))['ws'])" 2>/dev/null) \
  || { echo "cannot read ws from $DIR/mount.json" >&2; exit 1; }
MOUNT_PATH="${WS#*:}"

step exec        "$DIR/exec.json"        "$CLI" --project "$PROJ" workspace exec "$WS" "ls -la && cat README.md sub/file.txt"
step write       "$DIR/write.json"       "$CLI" --project "$PROJ" fs write probe-work /probe-note.txt -m "probe write" --from "$SRC/README.md"
step commit      "$DIR/commit.json"      "$CLI" --project "$PROJ" workspace commit "$WS" -m "C0 probe commit"
step unmount     "$DIR/unmount.json"     "$CLI" --project "$PROJ" workspace unmount "$WS"
step close       "$DIR/close.json"       "$CLI" --project "$PROJ" sandbox close "$SANDBOX"
step remove      "$DIR/remove.json"      "$CLI" --project "$PROJ" project remove 2>/dev/null \
  || step remove "$DIR/remove.json" "$CLI" project remove "$PROJ" --confirm "$PROJ" 2>/dev/null \
  || printf 'remove skipped (CLI has no remove; the runtime owns removal) — scratch project %s left in %s\n' "$PROJ" "$LAYERFS_HOME" >>"$LOG"

printf '{"status":"ok","steps_log":"%s","last_step_ms":%s}\n' "$LOG" "${LAST_MS:-0}" >"$RESULT"
printf 'probe complete: %s\n' "$LOG"
