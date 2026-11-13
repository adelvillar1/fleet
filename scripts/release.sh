#!/bin/sh
# scripts/release.sh — cut a GitHub release from a gated, committed tree.
#
# usage:
#   scripts/release.sh tag v<VERSION> [--skip-gate] [--notes-file <path>]
#   scripts/release.sh assets v<VERSION> [--notes-file <path>]
#
#   tag <vX.Y.Z>      gate HEAD on a detached worktree, then tag + push HEAD.
#                     The tag is annotated; the release note comes from the
#                     changelog's head section or --notes-file.
#   assets <vX.Y.Z>   build the release binaries from the tagged tree
#                     (HEAD must be exactly the tag), bundle the dmg and
#                     the CLI tarball, and upload both to the release.
#   --skip-gate       skip the four-leg gate (tag mode only; explicit on
#                     the command line, never defaulted)
#
# The build caches are bounded (2026-11-09, KI-502): the gate's lane is a
# cache and nothing used to bound it — it reached 69 GiB while the drive sat
# at 1.6 GiB free. `preflight_disk` cleans it once it is over
# FLEET_GATE_TARGET_MAX_GIB (default 20) and refuses to start a build on a
# volume below FLEET_DISK_FLOOR_GIB (default 20) — a build that dies mid-link
# on ENOSPC is the failure nobody can read.
#
# prerequisites: gh (authenticated), python3, cargo + cargo-tauri on PATH,
# a clean working tree (both modes). Nothing is cut from a dirty tree. The
# gate runs on a detached worktree of HEAD — the standing rule that a
# working-tree green can hide uncompilable files (2026-10-22).
#
# The four legs (each leg's exit read from its own log — KI-465's rule,
# never through a status-losing pipe):
#   1. cargo build --all-targets
#   2. cargo test            (the lib, integration, and face-harness suites)
#   3. python3 scripts/ki_ledger.py
#   4. python3 scripts/plan_check.py
set -u

RELEASE_WORKTREE=/private/tmp/fleet-release-gate
RELEASE_TARGET=/tmp/fleet-gate-target
REPO_ROOT=$(git rev-parse --show-toplevel)

# The gate's cargo legs need the toolchain on PATH — nohup'd and cron-launched
# runs start with a bare env (the 2026-11-08 first cut died exit 127 on both).
export PATH="$HOME/.cargo/bin:$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:/opt/homebrew/bin:$PATH"
command -v cargo >/dev/null 2>&1 || die "cargo is not on PATH — install rust, or export the toolchain's bin before running"

die() { echo "release.sh: $*" >&2; exit 1; }

DISK_FLOOR_GIB=${FLEET_DISK_FLOOR_GIB:-20}
LANE_MAX_GIB=${FLEET_GATE_TARGET_MAX_GIB:-20}

# The volume's free space, in GiB, from `df -k`'s own answer.
free_gib() {
  df -k "$1" 2>/dev/null | awk 'NR==2 {print int($4 / 1024 / 1024)}'
}

# The contract's reading, from the binary that owns the arithmetic — never
# re-derived in shell (2026-11-09, the resource contract's C2). `fleet
# footprint --json --class <work>` answers the reserve, where it came from, and
# the estimate/admission for that work class; the floor below is only the
# fallback for a tree with no binary to ask yet (the first cut after a clean).
#
# The binary during a `tag` cut is the working tree's release build (if any);
# during an `assets` cut it is the one being packaged. Either way its absence
# is a NAMED degradation and the declared floor stands in — never silence.
# A binary that EXISTS is not a binary that ANSWERS: the release build on this
# tree predates the footprint verb until the first cut after this change, and a
# stale one answers an unknown-option error. So each candidate is asked, and
# the first that returns a document wins — a stale binary falls through to the
# debug build instead of silently skipping the contract.
contract_binary() {
  for candidate in "$REPO_ROOT/target/release/fleet" "$REPO_ROOT/target/debug/fleet"; do
    [ -x "$candidate" ] || continue
    if "$candidate" footprint --json --class "$1" --path "$REPO_ROOT" >/dev/null 2>&1; then
      echo "$candidate"
      return 0
    fi
  done
  return 1
}

# Print the contract's JSON for a work class, or nothing when no binary on the
# tree can answer. That absence is a named skip, not a refusal: the declared
# floor below still bounds the build.
contract_reading() {
  bin=$(contract_binary "$1") ||
    { echo "== contract: no binary on this tree answers the footprint verb — the declared floor stands in" >&2; return 1; }
  "$bin" footprint --json --class "$1" --path "$REPO_ROOT" 2>/dev/null
}

# Bound the gate's lane and refuse a build that cannot finish. The refusal is
# the contract's, not a hardcoded number: headroom above the reserve against
# the class's estimate. Both numbers are named when they are the reason.
preflight_disk() {
  class="$1"
  if [ -d "$RELEASE_TARGET" ]; then
    lane_gib=$(du -sm "$RELEASE_TARGET" 2>/dev/null | cut -f1)
    lane_gib=$(( ${lane_gib:-0} / 1024 ))
    if [ "$lane_gib" -gt "$LANE_MAX_GIB" ]; then
      echo "== the gate lane is ${lane_gib} GiB, over the ${LANE_MAX_GIB} GiB cap — cleaning it"
      CARGO_TARGET_DIR="$RELEASE_TARGET" cargo clean --manifest-path "$REPO_ROOT/Cargo.toml" \
        >/dev/null 2>&1 || true
    fi
  fi
  free=$(free_gib "$REPO_ROOT")
  [ -n "${free:-}" ] || die "the volume's free space could not be read (df -k)"
  # The contract's verdict, when the binary can give one. NOTE: the binary
  # exits non-zero when it REFUSES, and a refusal is the case this function
  # exists for — so the document is kept whatever the exit status and only an
  # empty answer falls through. Capturing with `|| reading=""` threw the
  # refusal away and let the build proceed on the bare floor (found by driving
  # the refusal arm against a 2 GiB volume, 2026-11-10).
  reading=$(contract_reading "$class")
  case "$reading" in
    {*) ;;
    *) reading="" ;;
  esac
  if [ -n "$reading" ]; then
    reserve_gib=$(printf '%s' "$reading" | python3 -c 'import json,sys;print(int(json.load(sys.stdin)["reserve_kib"]//1048576))')
    headroom_gib=$(printf '%s' "$reading" | python3 -c 'import json,sys;print(int(json.load(sys.stdin)["headroom_kib"]//1048576))')
    estimate_gib=$(printf '%s' "$reading" | python3 -c 'import json,sys;print(int(json.load(sys.stdin)["estimate_kib"]//1048576))')
    admitted=$(printf '%s' "$reading" | python3 -c 'import json,sys;print("yes" if json.load(sys.stdin)["admitted"] else "no")')
    source=$(printf '%s' "$reading" | python3 -c 'import json,sys;print(json.load(sys.stdin)["reserve_source"])')
    appetite=$(printf '%s' "$reading" | python3 -c 'import json,sys;print(json.load(sys.stdin)["appetite"])')
    [ "$free" -ge "$DISK_FLOOR_GIB" ] ||
      die "only ${free} GiB free on this volume (declared floor ${DISK_FLOOR_GIB}) — a build here dies mid-link. Run scripts/daily-maintenance.sh, or clean $RELEASE_TARGET and the other /private/tmp/fleet-*-target lanes."
    [ "$admitted" = "yes" ] ||
      die "the contract refuses this $class: ${headroom_gib} GiB of headroom above the ${reserve_gib} GiB reserve (${appetite}, ${source}) against a ${estimate_gib} GiB estimate. Free space or raise the appetite; nothing was built."
    echo "== disk: ${free} GiB free; contract admits $class — ${headroom_gib} GiB headroom above a ${reserve_gib} GiB reserve (${appetite}, ${source}) for a ${estimate_gib} GiB estimate"
    return 0
  fi
  [ "$free" -ge "$DISK_FLOOR_GIB" ] ||
    die "only ${free} GiB free on this volume (floor ${DISK_FLOOR_GIB}) — a build here dies mid-link. Run scripts/daily-maintenance.sh, or clean $RELEASE_TARGET and the other /private/tmp/fleet-*-target lanes."
  echo "== disk: ${free} GiB free (floor ${DISK_FLOOR_GIB}); the gate lane is capped at ${LANE_MAX_GIB} GiB"
}

# Settle the work that just finished: the delta against the budget it declared,
# as the binary's own arithmetic and its own row (C2/C5). A runaway is named
# and returns non-zero so the caller stops; a clean settle is a line.
settle_work() {
  class="$1"
  before_kib="$2"
  bin=$(contract_binary "$class") || { echo "== settle: no binary on this tree answers `footprint` — nothing recorded"; return 0; }
  # A runaway exits non-zero by design, so the status is kept beside the
  # document rather than used to discard it: a settle that threw its own
  # refusal away would record nothing exactly when the number mattered.
  out=$("$bin" footprint --json --class "$class" --path "$REPO_ROOT" --settle "$before_kib" 2>&1)
  rc=$?
  [ -n "$out" ] || { echo "== settle: the binary answered nothing — nothing recorded"; return 0; }
  printf '%s' "$out" | python3 -c 'import json,sys
try:
    d = json.load(sys.stdin)
except Exception:
    print("== settle: the binary answered without a document"); raise SystemExit(0)
print("== settle %s: %.2f GiB settled, verdict %s (budget %.2f GiB from the %s)" % (
    d.get("work_class"), d.get("delta_gib", 0),
    d.get("verdict"), d.get("budget_kib", 0) / 1048576.0, d.get("budget_source")))
if d.get("runaway"):
    print("== settle: RUNAWAY — it outgrew its declared budget; the diagnosis window keeps its artifacts")'
  return $rc
}


command -v gh >/dev/null 2>&1 || die "gh is not on PATH — it creates the release"
command -v python3 >/dev/null 2>&1 || die "python3 is not on PATH — the ledger and the plan checker need it"

[ "${1:-}" = "tag" ] || [ "${1:-}" = "assets" ] ||
  die "first argument must be tag or assets (usage in the header comment)"
MODE="$1"
TAG="${2:-}"
case "$TAG" in
  v[0-9]*.[0-9]*.[0-9]*) ;;
  *) die "second argument must be a version tag like v0.4.0 (got \"${TAG:-none}\")" ;;
esac

SKIP_GATE=0
NOTES_FILE=""
shift 2
while [ $# -gt 0 ]; do
  case "$1" in
    --skip-gate) SKIP_GATE=1 ;;
    --notes-file) NOTES_FILE="${2:?--notes-file needs a path}"; shift ;;
    *) die "unknown flag: $1" ;;
  esac
  shift
done

if [ "$MODE" = "tag" ]; then
  [ -z "$(git status --porcelain)" ] ||
    die "working tree is dirty — commit (or move to tmp/) before cutting a release"

  if git rev-parse -q --verify "$TAG" >/dev/null 2>&1; then
    die "tag $TAG already exists — bump the version and cut the next one"
  fi

  if [ "$SKIP_GATE" -ne 1 ]; then
    # The free space the gate starts from: the settle measures against it.
    GATE_BEFORE_KIB=$(df -k "$REPO_ROOT" 2>/dev/null | awk 'NR==2 {print $4}')
    preflight_disk release-gate
    echo "== gate: detached worktree of HEAD =="
    git worktree remove --force "$RELEASE_WORKTREE" >/dev/null 2>&1 || true
    git worktree add --detach "$RELEASE_WORKTREE" HEAD >/dev/null ||
      die "cannot create the detached worktree"
    HEAD_SHA=$(git rev-parse HEAD)

    # KI-351's lesson: the ambient CARGO_TARGET_DIR may name a co-tenant's
    # tree. Pin the target dir; the gate owns it.
    ( export CARGO_TARGET_DIR="$RELEASE_TARGET" REPO_ROOT
      cd "$RELEASE_WORKTREE" || exit 1
      echo "-- leg 1: build --all-targets"
      sh "$REPO_ROOT/scripts/release-gate-run.sh" build
      echo "-- leg 2: cargo test (lib + integration + face harness)"
      sh "$REPO_ROOT/scripts/release-gate-run.sh" test
      echo "-- leg 3: ki ledger"
      python3 scripts/ki_ledger.py > /tmp/gate-ledger-release.log 2>&1
      echo "EXIT=$?" > /tmp/gate-ledger-release.exit
      echo "-- leg 4: plan checker"
      python3 scripts/plan_check.py > /tmp/gate-plan-release.log 2>&1
      echo "EXIT=$?" > /tmp/gate-plan-release.exit
    )
    GATE_RC=0
    for leg in build lib ledger plan; do
      rc=$(sed 's/^EXIT=//' "/tmp/gate-${leg}-release.exit" 2>/dev/null || echo 1)
      if [ "$rc" != "0" ]; then
        echo "leg $leg FAILED (exit $rc) — log: /tmp/gate-${leg}-release.log"
        GATE_RC=1
      fi
    done
    git worktree remove --force "$RELEASE_WORKTREE" >/dev/null 2>&1 || true
    [ "$GATE_RC" = "0" ] || die "the gate is red on HEAD $HEAD_SHA — nothing is tagged"
    echo "== gate green on $HEAD_SHA =="
    # The settle: what the gate actually took, against the budget that admitted
    # it. A runaway does not un-tag the release (the gate finished; the release
    # is green), but it is named, and the row is what C8 learns from.
    settle_work release-gate "${GATE_BEFORE_KIB:-0}" ||
      echo "== settle: the gate outgrew its budget — recorded; the cut continues (see the row)"
    # The lane is debris once the gate is green and the tag pushed
    # (KI-514): 12 GiB from one morning of cuts accumulated in /tmp
    # because the settle recorded the spend and released nothing.
    # cache-sweep.sh --gate-lane is the standing recovery.
    rm -rf "$RELEASE_TARGET"
  else
    echo "== gate SKIPPED (--skip-gate) =="
  fi

  git tag -a "$TAG" -m "release $TAG" || die "cannot tag $TAG"
  git push origin "$TAG" ||
    die "tag created locally but the push failed — push by hand, then continue with the assets mode"
  echo "tagged and pushed $TAG at $(git rev-parse --short HEAD)"

  if [ -n "$NOTES_FILE" ]; then
    [ -f "$NOTES_FILE" ] || die "no notes file at $NOTES_FILE"
  else
    # The changelog's head section IS the release note (one place, no
    # divergence). Extract from the first `## v` heading to end of file;
    # refuse when the head section is not the tag being cut.
    NOTES_FILE=/tmp/release-notes.md
    # the head section only: a second `## v` heading stops the print,
    # so one release's note never carries the next section's words
    # (the found-to-EOF form pasted all four sections once — dry-run read).
    awk '/^## v/{n++} n==1' CHANGELOG.md > "$NOTES_FILE"
    [ -s "$NOTES_FILE" ] || die "the changelog has no release section — write it first"
    head -1 "$NOTES_FILE" | grep -q "^## $TAG" ||
      die "the changelog's head section is not $TAG ($(head -1 "$NOTES_FILE")) — cut the version the changelog names"
  fi
  if gh release view "$TAG" --json name >/dev/null 2>&1; then
    gh release edit "$TAG" --title "$TAG" --notes-file "$NOTES_FILE" ||
      die "the release exists but the edit failed"
  else
    gh release create "$TAG" --title "$TAG" --notes-file "$NOTES_FILE" ||
      die "gh release create failed"
  fi
  echo "release $TAG created; upload binaries with: scripts/release.sh assets $TAG"
  exit 0
fi

# ---- assets mode ----------------------------------------------------------
# HEAD must be exactly the tag: the binaries are built from the tree the
# tag names, and the tag was gated before it was pushed (tag mode).
[ "$(git rev-parse HEAD)" = "$(git rev-parse "$TAG^{commit}")" ] ||
  die "HEAD is not $TAG — check out the tag's commit to build its binaries"
[ -z "$(git status --porcelain)" ] ||
  die "working tree is dirty — the binaries would not match the tag"
[ -n "$NOTES_FILE" ] || NOTES_FILE=/tmp/release-notes.md

VERSION=$(printf '%s' "$TAG" | sed 's/^v//')

# The updater's signing key BEFORE the build: with createUpdaterArtifacts on
# the bundler signs the .app.tar.gz at build time (TAURI_SIGNING_PRIVATE_KEY
# accepts a PATH; the 0.6.0 face build proved it). A missing key would kill
# the cut mid-build instead of up front.
#
# The key lives in the OWNER'S HOME (own-home credentials): /tmp is swept
# after three days of disuse, and a key parked there vanished overnight — the
# next cut then demanded a vault unlock for the first time in 40+ releases.
# The durable home is ~/.fleet/updater-key (0600); /tmp is still read as a
# legacy fallback so an existing machine migrates on its next cut.
KEYFILE="${FLEET_UPDATER_KEY:-$HOME/.fleet/updater-key}"
if [ ! -f "$KEYFILE" ] && [ -f /tmp/fleet-updater-key ]; then
  mkdir -p "$(dirname "$KEYFILE")"
  cp /tmp/fleet-updater-key "$KEYFILE" && chmod 600 "$KEYFILE"
  echo "release.sh: the updater key migrated from /tmp to $KEYFILE (0600)"
fi
if [ ! -f "$KEYFILE" ]; then
  echo "release.sh: the updater private key is not at $KEYFILE" >&2
  echo "release.sh: extract it from the vault — ONCE; it then lives in your home," >&2
  echo "release.sh: and cuts never need the vault again:" >&2
  echo "  export FLEET_VAULT_PASSPHRASE=... && fleet vault get FLEET_TAURI_UPDATE_KEY > $KEYFILE && chmod 600 $KEYFILE" >&2
  echo "release.sh: then re-run this mode. Nothing was cut." >&2
  exit 1
fi
export TAURI_SIGNING_PRIVATE_KEY="$KEYFILE" TAURI_SIGNING_PRIVATE_KEY_PASSWORD=""

ASSETS_BEFORE_KIB=$(df -k "$REPO_ROOT" 2>/dev/null | awk 'NR==2 {print $4}')
preflight_disk release-assets
echo "== building the $TAG binaries (release) =="
cargo build --release > /tmp/release-build.log 2>&1 ||
  { echo "cargo build failed — /tmp/release-build.log"; exit 1; }
tail -1 /tmp/release-build.log

echo "== bundling the dmg (cargo tauri build) =="
cargo tauri build > /tmp/release-tauri-build.log 2>&1 ||
  { echo "cargo tauri build failed — /tmp/release-tauri-build.log"; exit 1; }
grep -o 'fleet_[^ ]*\.dmg' /tmp/release-tauri-build.log | tail -1

DMG=$(ls "$REPO_ROOT"/target/release/bundle/dmg/"fleet_${VERSION}"*_aarch64.dmg 2>/dev/null | head -1)
[ -n "$DMG" ] || die "no dmg named fleet_${VERSION}* in target/release/bundle/dmg"

echo "== packaging the CLI tarball =="
PKGDIR=/tmp/fleet-cli-pkg
rm -rf "$PKGDIR"
mkdir -p "$PKGDIR/fleet-aarch64-apple-darwin"
cp "$REPO_ROOT/target/release/fleet" "$PKGDIR/fleet-aarch64-apple-darwin/fleet"
"$PKGDIR/fleet-aarch64-apple-darwin/fleet" --version >/dev/null ||
  die "the packaged binary does not answer --version"
BV=$("$PKGDIR/fleet-aarch64-apple-darwin/fleet" --version)
printf '# %s\n\nThe CLI binary: the terminal face, the face'"'"'s server, the agent.\n\n```sh\ntar xzf fleet-aarch64-apple-darwin.tar.gz\nchmod +x fleet-aarch64-apple-darwin/fleet\n```\n' "$BV" \
  > "$PKGDIR/fleet-aarch64-apple-darwin/README.txt"
TARBALL="$PKGDIR/fleet-aarch64-apple-darwin.tar.gz"
tar -czf "$TARBALL" -C "$PKGDIR" fleet-aarch64-apple-darwin

# The CLI channel's update artifact: the RAW binary, no archive. The feed's
# cli row names this file and hashes these bytes; the client installs them
# byte for byte after proving they run and answer --version (KI-486 — the
# tarball used to be what the channel wrote over the binary). The tarball
# above stays for humans.
echo "== staging the raw update artifact =="
RAW_DIR=/tmp/fleet-cli-raw
rm -rf "$RAW_DIR"
mkdir -p "$RAW_DIR"
RAW="$RAW_DIR/fleet-aarch64-apple-darwin"
cp "$REPO_ROOT/target/release/fleet" "$RAW"
chmod +x "$RAW"
"$RAW" --version >/dev/null || die "the raw update artifact does not answer --version"

# The update artifact the plugin installs: the .app.tar.gz the config's
# createUpdaterArtifacts generated beside the dmg.
APP_TAR=$(ls "$REPO_ROOT"/src-tauri/target/release/bundle/macos/*.tar.gz 2>/dev/null | head -1)
if [ -z "$APP_TAR" ]; then
  for cand in "$REPO_ROOT"/target/release/bundle/macos/*.tar.gz; do
    [ -f "$cand" ] && APP_TAR="$cand" && break
  done
fi
[ -n "$APP_TAR" ] || die "no .app.tar.gz updater artifact beside the dmg — createUpdaterArtifacts generated none"

echo "== signing the updater artifact =="
command -v cargo >/dev/null 2>&1 || die "cargo is not on PATH — the signer needs it"
export PATH="$HOME/.cargo/bin:$PATH"
# KEYFILE was already checked above the build (createUpdaterArtifacts signs
# there too, via TAURI_SIGNING_PRIVATE_KEY). -k takes the key's CONTENTS,
# not a path — the path form fails to decode (proved live 2026-11-08).
cargo-tauri signer sign -k "$(cat "$KEYFILE")" "$APP_TAR" --password ""
[ -f "$APP_TAR.sig" ] || die "the signer left no .sig beside $APP_TAR"

echo "== uploading the assets =="
if ! gh release view "$TAG" --json name >/dev/null 2>&1; then
  [ -f "$NOTES_FILE" ] || die "no release $TAG and no notes file to create it with"
  gh release create "$TAG" --title "$TAG" --notes-file "$NOTES_FILE" || die "gh release create failed"
fi
gh release upload "$TAG" "$DMG" "$TARBALL" "$RAW" "$APP_TAR" "$APP_TAR.sig" --clobber ||
  die "gh release upload failed"
echo "uploaded: $(basename "$DMG"), $(basename "$TARBALL"), $(basename "$RAW"), $(basename "$APP_TAR") + .sig"

# ---- the feed: latest.json, the v2 updater's channel ----------------------
#
# The feed is only true when the tree's declaration, the bundled literal
# and the dmg's filename agree — the three-way check the plan pinned.
# A disagreement names the legs that did not agree and refuses the cut.
echo "== reading the feed's version three ways =="
CONF_VERSION=$(python3 -c "import json;print(json.load(open('$REPO_ROOT/src-tauri/tauri.conf.json'))['version'])")
BUNDLEDMG=$(basename "$DMG")
case "$BUNDLEDMG" in
  fleet_${VERSION}*_aarch64.dmg) ;;
  *) die "the dmg's name ($BUNDLEDMG) does not carry $VERSION" ;;
esac
DESKTOP_BIN="$REPO_ROOT/src-tauri/target/release/fleet-desktop"
[ -f "$DESKTOP_BIN" ] || DESKTOP_BIN="$REPO_ROOT/target/release/fleet-desktop"
[ -f "$DESKTOP_BIN" ] || die "no fleet-desktop binary at src-tauri/target or target/release"

# The third leg: the BUNDLED APP's version, strongest-form — the bundle's
# own Info.plist read by plutil. (The first cut proved the strings probe
# unreliable: const-merging packs "0.6.0" into neighbors, so a grep for a
# line ending in " 0.6.0" misses a binary that truly carries the version.)
PLIST_DIR="$REPO_ROOT/target/release/bundle/macos/fleet.app/Contents"
PLIST_HIT=$(plutil -extract CFBundleShortVersionString raw "$PLIST_DIR/Info.plist" 2>/dev/null | grep -cx "$VERSION" || true)
[ "$PLIST_HIT" = "1" ] ||
  { strings "$DESKTOP_BIN" | grep -q " ${VERSION}\$" ||
      die "the bundled app's Info.plist and the binary both fail to name $VERSION"; }

if [ "$CONF_VERSION" != "$VERSION" ]; then
  die "the three-way version check failed: tauri.conf.json says $CONF_VERSION, the tag says $VERSION"
fi

echo "== generating latest.json =="
# The feed must upload under its bare feed name: `gh release upload`
# keeps the basename, and `/releases/latest/download/latest.json` 404d
# under the scratch name `fleet-latest.json` in the v0.6.0 cut.
LATEST=/tmp/latest.json
python3 - "$VERSION" "$TAG" "$(basename "$DMG")" "$(basename "$APP_TAR")" "$DMG" "$APP_TAR" "$RAW" > "$LATEST" <<'PYEOF'
import hashlib, json, sys, datetime
version, tag, dmg_name, tar_name, dmg_path, tar_path, cli_raw = sys.argv[1:]
def sha(path):
    h = hashlib.sha256()
    with open(path, 'rb') as f:
        for chunk in iter(lambda: f.read(1 << 20), b''):
            h.update(chunk)
    return h.hexdigest()
base = "https://github.com/adelvillar1/fleet/releases/download/" + tag + "/"
feed = {
    "version": version,
    "notes": "fleet " + version + ". See the release page for what changed.",
    "pub_date": datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
    "platforms": {
        "darwin-aarch64": {
            "signature": open(tar_path + ".sig", "rb").read().decode().strip(),
            "url": base + tar_name,
        }
    },
    # The CLI channel's row: the RAW binary (no archive) and its sha256.
    # The client installs these bytes, so the hash must be this file's own
    # (KI-486: the tarball was hashed and installed).
    "cli": {"url": base + cli_raw.rsplit("/", 1)[-1], "signature": sha(cli_raw)},

}
# the shell's redirect owns the file: the heredoc stdout IS $LATEST, no
# exported var and no hardcoded scratch path can detach the feed from its name
print(json.dumps(feed, indent=2) + "\n")
PYEOF
[ -s "$LATEST" ] || die "latest.json was not generated"
python3 -c "import json;d=json.load(open('$LATEST'));assert d['version']=='$VERSION' and 'darwin-aarch64' in d['platforms'] and 'cli' in d, 'the feed is missing a leg'" ||
  die "the generated feed failed its own leg read"
python3 -c "import json;d=json.load(open('$LATEST'));u=d['cli']['url'];assert u.rsplit('/',1)[-1]=='$(basename "$RAW")', u" ||
  die "the feed's cli row does not name the raw binary artifact"
gh release upload "$TAG" "$LATEST" --clobber || die "gh release upload of latest.json failed"
echo "uploaded: latest.json (the updater's channel + the cli row)"

# ---- the publication read-back -------------------------------------------
#
# An upload is not a publication: the feed is only true when something
# other than the uploader reads it. The branch-permanent URL is fetched
# anonymously, its cli row is downloaded, and the bytes are hashed against
# the row's own signature. GitHub's latest redirect can lag a fresh
# upload, so the read is retried before it is called a refusal.
echo "== reading the feed back anonymously =="
READBACK=/tmp/latest-readback.json
attempt=1
# GitHub's `latest` redirect can lag a fresh upload by more than 30s (KI-526:
# the v0.6.38 cut published just past the old window and read back as a false
# refusal). 18 tries × 5s = 90s rides the lag out; the loop still breaks the
# moment the feed names the version, so the common case pays nothing.
while [ "$attempt" -le 18 ]; do
  if curl -fsSL "https://github.com/adelvillar1/fleet/releases/latest/download/latest.json" -o "$READBACK" 2>/dev/null &&
    python3 -c "import json,sys;d=json.load(open('$READBACK'));sys.exit(0 if d.get('version')=='$VERSION' else 1)" 2>/dev/null; then
    break
  fi
  sleep 5
  attempt=$((attempt + 1))
done
[ "$attempt" -le 18 ] || die "the published feed does not name $VERSION after 90s — the release is not published"
python3 - "$READBACK" "$RAW" <<'PYEOF'
import hashlib, json, sys, urllib.request
feed_path, raw = sys.argv[1], sys.argv[2]
feed = json.load(open(feed_path))
row = feed["cli"]
url, published = row["url"], row["signature"]
local = hashlib.sha256(open(raw, "rb").read()).hexdigest()
if url.rsplit("/", 1)[-1] != raw.rsplit("/", 1)[-1]:
    sys.exit("the published feed's cli url names %s, not the raw artifact" % url)
if published != local:
    sys.exit("the published cli signature is not the raw artifact's sha256")
with urllib.request.urlopen(url) as reply:
    served = hashlib.sha256(reply.read()).hexdigest()
if served != published:
    sys.exit("the served artifact's sha256 is not the feed's signature")
print("the published feed names %s; the cli row serves the raw binary, sha256 %s" % (feed["version"], served[:12]))
PYEOF
# The settle for the assets cut belongs HERE, after the publication read-back
# and before the lane is released — the lane's own gigabytes are part of what
# the cut took, so measuring before cleaning it would flatter the number. The
# settle is the work unit's ending: what it took, against what admitted it.

rm -rf "$RELEASE_TARGET"
settle_work release-assets "${ASSETS_BEFORE_KIB:-0}" ||
  echo "== settle: the assets cut outgrew its declared budget (recorded above)"

# The cut is finished, so the lane that built it is finished too: a gate lane
# is a detached worktree's private target dir, and the working tree never
# reads it. Keeping it would be 8 GiB of temp files for a job that is done
# (2026-11-09, the owner's rule: nothing kept past the work it did).
# FLEET_KEEP_GATE_LANE=1 keeps it warm instead, which only pays when the next
# cut follows within the hour.
if [ "${FLEET_KEEP_GATE_LANE:-0}" = "1" ]; then
  echo "== the gate lane is kept warm (FLEET_KEEP_GATE_LANE=1): $RELEASE_TARGET"
else
  lane_before=$(du -sm "$RELEASE_TARGET" 2>/dev/null | cut -f1)
  CARGO_TARGET_DIR="$RELEASE_TARGET" cargo clean --manifest-path "$REPO_ROOT/Cargo.toml" \
    >/dev/null 2>&1 || true
  echo "== the cut is done: the gate lane released (${lane_before:-0} MiB freed; the next cut builds cold)"
fi
# The gate's own logs are the diagnosis of a red run and nothing else; a green
# run's are history. The sweep keeps the newest, the maintenance prunes the
# rest by age.
