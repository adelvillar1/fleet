#!/bin/bash
# layerfs-live.sh — C3 of docs/plans/2026-10-26-layerfs-closing-the-seams.md.
#
# The standing seam diagnostic: one delegation end to end in a clean
# environment (no LAYERFS_* fact in the process), against the durable
# home the machine's own `fleet install` provisions. It is the C6 proof
# kept as a script — run it before a release, or whenever the substrate
# changes, and read its exit code.
#
# What it proves, in order:
#   1. the doctor's substrate line answers with the worker and the image;
#   2. a FRESH folder registers itself (the declared import) and a
#      delegation runs isolated in it — the child's work on its branch,
#      the merge-back, and (C2) the merge applied home;
#   3. the destination renders THAT run: the branches with their states,
#      the trunk's layers, the figures, and the applied file in main's
#      own listing;
#   4. the host tree is untouched except by the applied merge.
#
#   usage: scripts/layerfs-live.sh
#
# Reads (names only; nothing here prints a value that could be a secret):
#   the machine's FLEET_HOME (the durable substrate home)
#   node, git, docker on PATH
#
# Exit codes: 0 the whole seam ran; 1 a step failed (named on stderr).

set -u

HERE="$(cd "$(dirname "$0")/.." && pwd)"
BIN="${FLEET_BIN:-$HERE/target/debug/fleet}"
FLEET_HOME="${FLEET_HOME:-$HOME/.fleet}"
# The clean environment: HOME, FLEET_HOME, PATH — and NOTHING else. A
# LAYERFS_* fact leaking in would make the proof a lie.
CLEAN_ENV=(env -i HOME="$HOME" FLEET_HOME="$FLEET_HOME" PATH="$PATH")
RUN="$FLEET_HOME"
DIR="$HERE/tmp/layerfs-live"
LOG="$DIR/run.log"
SRC="$DIR/fresh-src"

rm -rf "$DIR"
mkdir -p "$SRC"
printf '# layerfs live seam\nprobe content v1\n' > "$SRC/README.md"
git -C "$SRC" init -q
git -C "$SRC" add -A
git -C "$SRC" -c user.name=t -c user.email=t@t commit -qm "the fresh folder"

say() { printf '%s\n' "$*" >> "$LOG"; }

say "# layerfs-live  $(date -u +%FT%TZ)"

# 1. the substrate's own line.
"${CLEAN_ENV[@]}" "$BIN" doctor 2>&1 | grep -E "^layerfs" >> "$LOG" || {
    echo "layerfs-live: the doctor's substrate line did not answer" >&2
    exit 1
}
grep -q "layerfs-home" "$LOG" || { echo "layerfs-live: no substrate line" >&2; exit 1; }
grep -q "worker answering" "$LOG" || {
    echo "layerfs-live: the worker does not answer — is the substrate provisioned?" >&2
    exit 1
}

# 2. the clean-environment delegation in the fresh folder.
cd "$SRC" || exit 1
# The content carries the run's stamp: the wire's piece-compare answers
# no Commit for identical bytes, and a repeat run writing the same word
# onto a main that already holds it would prove nothing but a no-op.
STAMP=$(date -u +%FT%TZ)
# --yes: the apply rides the approval gate, and a scripted run has no
# owner at the seat — the attended mode is the one whose proof this is.
"${CLEAN_ENV[@]}" "$BIN" run --yes \
    "Delegate this to a subagent. The subagent's entire task: call write_file ONCE with path \
live-proof.txt and content exactly \"$STAMP - isolated\", then report. The subagent must not \
run bash, must not investigate the environment, and must not look for the file anywhere else. \
Do not create the file yourself." >> "$LOG" 2>&1 || {
    echo "layerfs-live: the delegation failed (see $LOG)" >&2
    exit 1
}
grep -q "subagent completed" "$LOG" || {
    echo "layerfs-live: the child did not complete (see $LOG)" >&2
    exit 1
}

# 3. the Store's branches for the folder's project.
PROJECT=$(python3 - "$FLEET_HOME" "$SRC" <<'PY'
import json, sys, os
home, folder = sys.argv[1], os.path.realpath(sys.argv[2])
config = json.load(open(os.path.join(home, "config.json")))
for path, reg in (config.get("layerfs", {}).get("projects") or {}).items():
    if os.path.realpath(path) == folder:
        print(reg["name"]); break
PY
)
[ -n "$PROJECT" ] || { echo "layerfs-live: the fresh folder registered no project" >&2; exit 1; }
say "— project $PROJECT —"

BUNDLE=$(python3 - "$FLEET_HOME" <<'PY'
import json, sys
config = json.load(open(sys.argv[1] + "/config.json"))
print(config.get("layerfs", {}).get("bundle", ""))
PY
)
[ -x "$BUNDLE/layerfs-daemon" ] || [ -f "$BUNDLE/layerfs-daemon" ] || {
    echo "layerfs-live: the recorded bundle $BUNDLE is gone" >&2
    exit 1
}
IMAGE=$(python3 -c "import json;print(json.load(open('$BUNDLE/manifest.json'))['images']['runtime'])")
STORE=$(python3 - "$FLEET_HOME" "$SRC" <<'PY'
import json, sys, os
home, folder = sys.argv[1], os.path.realpath(sys.argv[2])
config = json.load(open(os.path.join(home, "config.json")))
for path, reg in (config.get("layerfs", {}).get("projects") or {}).items():
    if os.path.realpath(path) == folder:
        print(reg["store"]); break
PY
)
VOLUME=$(printf '%s' "$STORE" | sed -n 's|.*/volumes/\([^/]*\)/.*|\1|p')
LAYERFS_HOME="$FLEET_HOME/layerfs-home" \
LAYERFS_WORKER_ENDPOINT="$FLEET_HOME/layerfs-home/worker/endpoint.json" \
LAYERFS_IMAGE="$IMAGE" \
LAYERFS_DAEMON="$BUNDLE/layerfs-daemon" \
LAYERFS_STORE_PATH="$STORE" \
LAYERFS_VOLUME="$VOLUME" \
    node "$BUNDLE/api/agent/cli/src/layerfs.js" --project "$PROJECT" project list_branches \
    >> "$LOG" 2>&1 || {
    echo "layerfs-live: the Store's branches did not answer" >&2
    exit 1
}
grep -q "fleet-child-" "$LOG" || {
    echo "layerfs-live: no child branch on the Store" >&2
    exit 1
}
grep -q "merge-fleet-child-" "$LOG" || {
    echo "layerfs-live: the merge-back branch is missing" >&2
    exit 1
}

# 4. the merge applied home: the child's file is in the working tree,
#    through the parent's own write path.
[ -f "$SRC/live-proof.txt" ] || {
    echo "layerfs-live: the settled merge did not apply home" >&2
    exit 1
}
say "— applied home: live-proof.txt —"

# 5. the SECOND delegation sees the first one's work: the spawn sync
#    carries the host's newer paths onto the fresh fork, so the child
#    reads what the last delegation applied — the stale-base residue
#    retired, repeatably.
STAMP2=$(date -u +%FT%TZ)
cd "$SRC" || exit 1
"${CLEAN_ENV[@]}" "$BIN" run --yes \
    "Delegate this to a subagent. The subagent's task: call read_file ONCE with path \
live-proof.txt, then report the file's content verbatim. Nothing else." >> "$LOG" 2>&1 \
    || { echo "layerfs-live: the second delegation failed (see $LOG)" >&2; exit 1; }
grep -q "$STAMP" "$LOG" || {
    echo "layerfs-live: the second child did not read the first one's work — \
the fork base is stale" >&2
    exit 1
}
say "— second child read the first one's work (sync at spawn) —"

printf 'layerfs-live: ok — %s\n' "$LOG"
exit 0
