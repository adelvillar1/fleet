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

die() { echo "release.sh: $*" >&2; exit 1; }

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
    awk '/^## v/{found=1} found' CHANGELOG.md > "$NOTES_FILE"
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

echo "== uploading the assets =="
if ! gh release view "$TAG" --json name >/dev/null 2>&1; then
  [ -f "$NOTES_FILE" ] || die "no release $TAG and no notes file to create it with"
  gh release create "$TAG" --title "$TAG" --notes-file "$NOTES_FILE" || die "gh release create failed"
fi
gh release upload "$TAG" "$DMG" "$TARBALL" --clobber ||
  die "gh release upload failed"
echo "uploaded: $(basename "$DMG"), $(basename "$TARBALL")"
