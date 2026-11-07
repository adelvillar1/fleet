#!/usr/bin/env bash
# The edit preflight: names the structure guards an edit will meet,
# before the first compile. Pure grep — seconds, no cargo.
#
# Usage:
#   scripts/edit-preflight.sh [paths...]     # defaults to the working
#                                            # tree's changed files
#
# The inventory itself is docs/architecture/structure-guards.md — the
# one home for what each guard is and what to do when it fires. This
# script only names; the doc explains.
#
# Exit 0 always: the tool names, the reader decides. The house habit it
# serves is the pins-before-edits rule — see the house build-loop skill.

set -u
REPO="$(git rev-parse --show-toplevel 2>/dev/null)" || REPO="$(pwd)"
cd "$REPO"

# The changed set: arguments when given, else the working tree's own.
if [ "$#" -gt 0 ]; then
    FILES="$*"
else
    FILES="$(git status --porcelain 2>/dev/null | awk '{print $2}')"
fi
if [ -z "$FILES" ]; then
    echo "preflight: no changed files; nothing to name"
    exit 0
fi

hits=0
say() { printf '%s\n' "$*"; hits=$((hits + 1)); }

has() {
    for f in $FILES; do
        [ -e "$f" ] && return 0
    done
    return 1
}

# A. The socket guards: which changed modules do the beats scan?
SCANNED=""
for f in $FILES; do
    case "$f" in
        src/doctor.rs|src/memory.rs|src/agent.rs|src/tools.rs|src/session.rs|\
src/calib.rs|src/route.rs|src/compaction.rs|src/cron.rs|src/improve.rs|\
src/boards.rs|src/skills.rs|src/subagents.rs|src/tui/*|src/api.rs|\
src/mcp.rs|src/lanes.rs|src/home.rs|src/install.rs|src/net.rs|src/cli.rs)
            SCANNED="$SCANNED $f"
            ;;
    esac
done
if [ -n "$SCANNED" ]; then
    say "GUARD A (socket guards) — scanned synchronous path(s):$SCANNED"
    say "  The judge/delegate/explain guards text-scan these for beat references."
    for f in $SCANNED; do
        grep -n 'judge::\|delegate::\|explain::' "$f" 2>/dev/null \
            | while IFS= read -r line; do say "  LIVE: $f:$line"; done
    done
    say "  A lane-owning wrapper belongs in its own module (src/dream.rs is the precedent)."
    say "  Pins: src/judge/tests.rs, src/delegate/tests.rs, src/explain/tests.rs"
fi

# B. The tool-surface bounds.
if has src/tools.rs; then
    say "GUARD B (tool-surface bounds) — tools.rs changed: any schema or"
    say "  description edit moves the definition-token count. Pin:"
    say "  cargo test -p fleet --lib the_native_surface_fits"
    say "  Also pinned: a_run_with_memory_declares_the_write_path (the declared"
    say "  list follows the withholding walk, which MAX_DEFINITION_TOKENS feeds)."
fi

# C. The calibration row discipline.
if has src/calib.rs; then
    say "GUARD C (calib rows) — calib.rs changed: a new Row variant needs the"
    say "  variant, op(), ts(), an OP_* constant, and the OPS array — whose"
    say "  expected list is itself pinned in calib.rs's tests."
    say "  Pin: cargo test -p fleet --lib calib::tests"
fi

# D. The request-path / cold-build split.
if has src/gui/related.rs; then
    say "GUARD D (request path) — related.rs changed: the request-path corpus()"
    say "  must read the cache and never call build_corpus or probed_embedder."
    say "  Pin: cargo test -p fleet --lib a_request_path_never_builds"
fi

# E. The struct-field fan-out.
for f in $FILES; do
    case "$f" in
        src/memory.rs|src/onboard.rs|src/session_recall.rs|src/gui/bridge.rs|\
src/tui/app.rs|src/boards.rs|src/cli.rs)
            case "$f" in
                src/cli.rs) note="MemoriesOptions/RememberOptions literals in cli.rs's parse tests" ;;
                src/boards.rs) note="MemoryFact in read_memory + the snapshot fixture in tui/app.rs" ;;
                *) note="Fact/Remember/FactEdit literals in memory.rs tests, onboard.rs, session_recall.rs, gui/bridge.rs, tui/app.rs" ;;
            esac
            say "GUARD E (struct fan-out) — '$f' changed: a new field on a shared"
            say "  struct breaks hand-built initializers beyond the compile: $note."
            say "  New fields land with every constructor in the same commit."
            ;;
    esac
done

# F. The face harness's fixture counts.
if has tests/face/harness.mjs; then
    say "GUARD F (harness counts) — harness.mjs changed: fixture additions shift"
    say "  exact-count assertions ('N more facts', revealed totals). Search the"
    say "  count strings beside the checks. Run: node tests/face/harness.mjs"
fi

if [ "$hits" -eq 0 ]; then
    echo "preflight: no structural guards meet this change — compile away"
else
    echo "preflight: $hits guard note(s) above; the inventory: docs/architecture/structure-guards.md"
fi
exit 0
