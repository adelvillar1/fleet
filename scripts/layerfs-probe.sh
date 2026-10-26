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
# The sealed bundle: the operator's pin, else the directory the CLI
# itself sits in (the CLI ships inside a sealed bundle's api/).
BUNDLE="${FLEET_LAYERFS_BUNDLE:-}"
if [ -z "$BUNDLE" ] && [ -f "$CLI" ]; then
    # The CLI sits at <bundle>/api/agent/cli/src/layerfs.js, so the
    # sealed bundle's root is four levels up from its own directory.
    CAND="$(cd "$(dirname "$CLI")/../../../.." 2>/dev/null && pwd)"
    [ -f "$CAND/layerfs-daemon" ] && BUNDLE="$CAND"
fi
# The runtime image: the operator's pin, else the sealed bundle's own
# manifest — the same source `fleet install` records.
if [ -z "${LAYERFS_IMAGE:-}" ] && [ -n "$BUNDLE" ] && [ -f "$BUNDLE/manifest.json" ]; then
    LAYERFS_IMAGE=$(python3 -c "import json;print(json.load(open('$BUNDLE/manifest.json'))['images']['runtime'])" 2>/dev/null || true)
    [ -n "$LAYERFS_IMAGE" ] && export LAYERFS_IMAGE
fi
# The daemon, the bundle's own: a Workspace's Sandbox is a container the
# daemon starts, and the wire refuses without it.
if [ -z "${LAYERFS_DAEMON:-}" ] && [ -n "$BUNDLE" ] && [ -f "$BUNDLE/layerfs-daemon" ]; then
    export LAYERFS_DAEMON="$BUNDLE/layerfs-daemon"
fi

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
# The home must be 0700 and ours: an existing 0755 home is refused at
# Init with `Unauthorized`, so never let mkdir's default mode stand.
mkdir -m 700 -p "$HOME_DIR"
chmod 700 "$HOME_DIR"
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

# Init: on macOS the sanctioned path is @layerfs/runtime's start_init
# (a container with the source mounted read-only); the CLI's own init
# is for a first creation. A provisioned project answers Exists — the
# note records that the runtime's init already ran (its timing lands
# in the plan's C0 section).
if "$CLI" --project "$PROJ" project list_branches >"$DIR/exists.json" 2>/dev/null; then
    printf '%-14s %s\n' "init" "skipped (project already initialized)" >>"$LOG"
else
    # The sanctioned macOS route is the runtime's start_init, which
    # fleet wraps in scripts/layerfs-register.mjs: it creates the
    # Store's volume, records the deploy facts and answers the store
    # path — the CLI's own `project init` needs a store path the
    # environment cannot derive (it installs the Store wherever the
    # client is pointed).
    REGISTER="${FLEET_LAYERFS_REGISTER:-$(dirname "$0")/layerfs-register.mjs}"
    # The runtime refuses a relative source, so the scratch tree is
    # named absolutely.
    SRC_ABS=$(cd "$SRC" && pwd)
    if [ -f "$REGISTER" ] && [ -n "${LAYERFS_IMAGE:-}" ]; then
        T0=$(date +%s%N)
        if node "$REGISTER" "$PROJ" "$SRC_ABS" "$HOME_DIR" "$BUNDLE" "$LAYERFS_IMAGE" \
             >"$DIR/init.json" 2>"$DIR/init.err"; then
            T1=$(date +%s%N)
            printf '%-14s %sms (the runtime start_init)\n' "init" "$(( (T1 - T0) / 1000000 ))" >>"$LOG"
        else
            step init  "$DIR/init.json"  "$CLI" --project "$PROJ" init "$PROJ" "$SRC"
        fi
    else
        step init        "$DIR/init.json"        "$CLI" --project "$PROJ" init "$PROJ" "$SRC"
    fi
fi
# The durable home's own endpoint, derived and exported AFTER the init
# step: without it the CLI takes the in-process transport, and macOS
# cannot load the sealed bundle's Linux addon (`api/binding/layerfs.node`
# does not exist there — the worker carries `LAYERFS_NATIVE` instead).
# The init itself rides the runtime, which must not see it.
if [ -z "${LAYERFS_WORKER_ENDPOINT:-}" ] && [ -f "$HOME_DIR/worker/endpoint.json" ]; then
  export LAYERFS_WORKER_ENDPOINT="$HOME_DIR/worker/endpoint.json"
fi

# The scratch project's Store, read from the register's answer (or the
# operator's pin): every ladder step is a client call, and a client
# needs the store path the project installed. The volume rides beside
# it — a Sandbox mounts the Store's own volume.
if [ -z "${LAYERFS_STORE_PATH:-}" ] && [ -f "$DIR/init.json" ]; then
    PROBE_STORE=$(python3 -c "import json;print(json.load(open('$DIR/init.json')).get('store',''))" 2>/dev/null || true)
    PROBE_VOLUME=$(python3 -c "import json;print(json.load(open('$DIR/init.json')).get('volume',''))" 2>/dev/null || true)
    [ -n "$PROBE_STORE" ] && export LAYERFS_STORE_PATH="$PROBE_STORE"
    [ -n "$PROBE_VOLUME" ] && export LAYERFS_VOLUME="$PROBE_VOLUME"
fi
BRANCH="probe-work-$(date +%s)"
step fork        "$DIR/fork.json"        "$CLI" --project "$PROJ" branch fork "$BRANCH"
step sandbox     "$DIR/sandbox.json"     "$CLI" --project "$PROJ" sandbox create
SANDBOX=$(python3 -c "import json;print(json.load(open('$DIR/sandbox.json'))['sandbox'])" 2>/dev/null) \
  || { echo "cannot read sandbox number from $DIR/sandbox.json" >&2; exit 1; }
step mount       "$DIR/mount.json"       "$CLI" --project "$PROJ" workspace mount "$SANDBOX" "$BRANCH"
WS=$(python3 -c "import json;print(json.load(open('$DIR/mount.json'))['ws'])" 2>/dev/null) \
  || { echo "cannot read ws from $DIR/mount.json" >&2; exit 1; }
MOUNT_PATH="${WS#*:}"

step exec        "$DIR/exec.json"        "$CLI" --project "$PROJ" workspace exec "$WS" "ls -la && cat README.md sub/file.txt"
step write       "$DIR/write.json"       "$CLI" --project "$PROJ" fs write "$BRANCH" /probe-note.txt -m "probe write" --from "$SRC/README.md"
step commit      "$DIR/commit.json"      "$CLI" --project "$PROJ" workspace commit "$WS" -m "C0 probe commit"
step unmount     "$DIR/unmount.json"     "$CLI" --project "$PROJ" workspace unmount "$WS"
step close       "$DIR/close.json"       "$CLI" --project "$PROJ" sandbox close "$SANDBOX"
# --- C0's lifecycle measurements: two clients, idle exit, reclaim ---
# Two Store clients at once: the worker admits concurrent callers.
C_START=$(python3 -c 'import time;print(f"{time.time():.3f}")')
("$CLI" --project "$PROJ" project list_branches >"$DIR/client-a.json" 2>"$DIR/client-a.err") &
PA=$!
("$CLI" --project "$PROJ" project list_branches >"$DIR/client-b.json" 2>"$DIR/client-b.err") &
PB=$!
wait $PA; CA=$?
wait $PB; CB=$?
C_END=$(python3 -c 'import time;print(f"{time.time():.3f}")')
C_MS=$(python3 -c "print(f'{($C_END-$C_START)*1000:.0f}')")
printf '%-14s exit=%s/%s %sms (both in flight together)\n' "two-clients" "$CA" "$CB" "$C_MS" >>"$LOG"
if [ "$CA" -ne 0 ] || [ "$CB" -ne 0 ]; then
  printf 'STEP-FAIL two-clients exit=%s/%s\n' "$CA" "$CB" >&2
  printf '{"status":"step-failed","step":"two-clients","exits":[%s,%s]}\n' "$CA" "$CB" >"$RESULT"
  exit 1
fi

# Idle exit: a Sandbox closes itself two minutes after its last
# Workspace unmount with no call in flight. Create, mount, unmount —
# then wait for the daemon to end its own container (poll ≤160 s).
step lc-sandbox     "$DIR/lc-sandbox.json"   "$CLI" --project "$PROJ" sandbox create
LC_N=$(python3 -c "import json;print(json.load(open('$DIR/lc-sandbox.json'))['sandbox'])")
step lc-mount       "$DIR/lc-mount.json"     "$CLI" --project "$PROJ" workspace mount "$LC_N" "$BRANCH"
LC_WS=$(python3 -c "import json;print(json.load(open('$DIR/lc-mount.json'))['ws'])")
step lc-unmount     "$DIR/lc-unmount.json"   "$CLI" --project "$PROJ" workspace unmount "$LC_WS"
IDLE_START=$(python3 -c 'import time;print(f"{time.time():.3f}")')
IDLE_MS="timeout-160s"
IDLE_NAME="layerfs-${PROJ}-${LC_N}"
for _ in $(seq 1 32); do
  STATE=$(docker inspect -f '{{.State.Running}}' "$IDLE_NAME" 2>/dev/null || echo gone)
  if [ "$STATE" = "false" ] || [ "$STATE" = "gone" ]; then
    IDLE_END=$(python3 -c 'import time;print(f"{time.time():.3f}")')
    IDLE_MS=$(python3 -c "print(f'{($IDLE_END-$IDLE_START)*1000:.0f}')")
    break
  fi
  sleep 5
done
printf '%-14s %sms after unmount (daemon self-end, container %s)\n' "idle-exit" "$IDLE_MS" "$STATE" >>"$LOG"

# Reclaim: the next create reclaims a Sandbox that closed itself —
# the same number comes back and the stopped container is gone.
step lc-reclaim     "$DIR/lc-reclaim.json"   "$CLI" --project "$PROJ" sandbox create
LC_N2=$(python3 -c "import json;print(json.load(open('$DIR/lc-reclaim.json'))['sandbox'])")
RECLAIM="number-reused-$LC_N2"
[ "$LC_N2" = "$LC_N" ] || RECLAIM="new-number-$LC_N2 (old $LC_N)"
step lc-close       "$DIR/lc-close.json"     "$CLI" --project "$PROJ" sandbox close "$LC_N2"
printf '%-14s %s\n' "reclaim" "$RECLAIM" >>"$LOG"

# The SDK has no `remove` verb: the runtime owns project removal
# (runtime.project.remove). A note, never a step failure — the loop
# above is what C0 measures.
if "$CLI" --project "$PROJ" project remove </dev/null >"$DIR/remove.json" 2>&1; then
    printf '%-14s %s\n' "remove" "ok" >>"$LOG"
else
    printf '%-14s %s\n' "remove" "note: no CLI verb — the runtime owns removal" >>"$LOG"
fi

printf '{"status":"ok","steps_log":"%s","last_step_ms":%s}\n' "$LOG" "${LAST_MS:-0}" >"$RESULT"
printf 'probe complete: %s\n' "$LOG"
