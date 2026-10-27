#!/bin/bash
# layerfs-live.sh — the standing seam diagnostic (C3 of
# docs/plans/2026-10-26-layerfs-closing-the-seams.md, grown by C5 of
# docs/plans/2026-10-26-layerfs-history-instrument.md).
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
#   4. the host tree is untouched except by the applied merge;
#   5. the second delegation reads the first one's work (the spawn sync);
#   6. the INSTRUMENT reads it: the face's one read answers warm under
#      250 ms with its version, the branches carry their states, the
#      merge's layer carries the commit it was published from (the
#      graft's data), the nine figures are whole, the live feed answers,
#      and the file's text reads through the file route.
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

# 6. the INSTRUMENT: the face's reads against this run's Store — one
#    warm read under the bound, the whole payload the graph draws, the
#    live feed's route, and the file's text through the file route.
#    A face is the reader, so a face is what answers here.
PORT=$(python3 -c 'import socket; s=socket.socket(); s.bind(("127.0.0.1",0)); print(s.getsockname()[1]); s.close()')
"${CLEAN_ENV[@]}" "$BIN" gui --port "$PORT" --no-open >> "$LOG" 2>&1 &
GUI_PID=$!
cleanup() { kill "$GUI_PID" 2>/dev/null || true; wait "$GUI_PID" 2>/dev/null || true; }
trap cleanup EXIT
python3 - "$PORT" "$PROJECT" "$SRC" <<'PY' || exit 1
import json, sys, time, urllib.request

port, project, folder = sys.argv[1], sys.argv[2], sys.argv[3]
base = "http://127.0.0.1:" + port

def get(path, deadline=40.0):
    started = time.time()
    while True:
        try:
            with urllib.request.urlopen(base + path, timeout=30) as reply:
                return json.loads(reply.read() or b"{}"), time.time() - started
        except Exception:
            if time.time() - started > deadline:
                raise
            time.sleep(0.5)

fail = lambda said: sys.exit("layerfs-live: instrument: " + said)

# A cold read warms the worker and node; the BOUND is the warm read's.
doc, _ = get("/api/workspace-history")
if doc.get("type") != "workspace-history" or doc.get("registered") is not True:
    fail("the tree read is not the registered view: " + str(doc)[:200])
doc, warm = get("/api/workspace-history")
if warm >= 0.25:
    fail("the one read took %.3fs warm (bound 250ms)" % warm)
if not doc.get("version"):
    fail("the read carried no version token")
branches, layers, commits = doc.get("branches") or [], doc.get("layers") or [], doc.get("commits") or []
if not any(b.get("state") == "merged" for b in branches):
    fail("no branch reads merged (the graft data is missing)")
if not commits:
    fail("the read carried no commits")
for commit in commits:
    # The substrate's own id range (its comparable regex): 64..68 hex.
    ident = str(commit.get("id") or "")
    if len(ident) < 64 or len(ident) > 68 or any(ch not in "0123456789abcdef" for ch in ident):
        fail("a commit id is not a full ref: " + ident[:20])
grafts = [l for l in layers if l.get("source_commit")]
if not grafts:
    fail("no layer names the commit it was published from")
storage = doc.get("storage") or {}
missing = [k for k in ("logical","canonical","stored","metadata","file","free",
                       "dedupe","compression","init") if k not in storage]
if missing:
    fail("the storage figures are not whole; missing " + ", ".join(missing))
# The cut, when there is one, is a string of words — never a bare bool
# and never a silent drop.
truncated = doc.get("truncated")
if truncated is not None and not (isinstance(truncated, str) and truncated):
    fail("the truncation flag is not words: " + repr(truncated))
# The live feed answers (the delegation just traced its own calls).
calls, _ = get("/api/workspace-history/calls")
if calls.get("type") != "workspace-calls":
    fail("the calls feed did not answer: " + str(calls)[:200])
if "refused" not in calls:
    fail("the calls feed named no refusal field")
# The file instrument: the stamp the delegation wrote, read as text.
stamp = open(folder + "/live-proof.txt").read().strip()
with urllib.request.urlopen(
    base + "/api/workspace-history/file?ref=main&path=/live-proof.txt&mode=text&from=&to=&page=0",
    timeout=30,
) as reply:
    text = json.loads(reply.read() or b"{}")
if text.get("mode") != "text" or text.get("refused"):
    fail("the text read did not answer: " + str(text)[:200])
if stamp.split(" ")[0] not in (text.get("text") or ""):
    fail("the text read does not carry the delegation's stamp")
print("instrument: warm read %.0f ms, %d branches, %d layers (%d grafts), %d commits"
      % (warm * 1000, len(branches), len(layers), len(grafts), len(commits)))
PY
trap - EXIT
cleanup
say "— the instrument read it: one warm read, the graft data, the feed, the text —"

printf 'layerfs-live: ok — %s\n' "$LOG"
exit 0
