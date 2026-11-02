#!/bin/sh
# scripts/responses-probe.sh — the session-parity-remainders plan's W0:
# one live /responses round trip over a representative tool-call turn,
# captured under tmp/responses-probe/ so the plan can reconcile the
# documented shapes against the wire before the adapter's mapper is
# called done on a live provider (the E0 discipline). On demand only —
# never runs in CI or the gate; the captures are never committed.
#
# Usage: scripts/responses-probe.sh [dotenv-file]
#
# The three variables it reads (names only here; values never printed):
#   FLEET_RESPONSES_BASE_URL  e.g. https://opencode.ai/zen/go/v1
#   FLEET_RESPONSES_API_KEY   the provider's key
#   FLEET_RESPONSES_MODEL     a responses-marked model, e.g. gpt-6-luna
set -eu

DOTENV="${1:-$HOME/.fleet/.env}"
if [ -f "$DOTENV" ]; then
    BASE_URL=$(grep -E '^FLEET_RESPONSES_BASE_URL=' "$DOTENV" | head -1 | cut -d= -f2-)
    API_KEY=$(grep -E '^FLEET_RESPONSES_API_KEY=' "$DOTENV" | head -1 | cut -d= -f2-)
    MODEL=$(grep -E '^FLEET_RESPONSES_MODEL=' "$DOTENV" | head -1 | cut -d= -f2-)
fi
BASE_URL=${FLEET_RESPONSES_BASE_URL:-$BASE_URL}
API_KEY=${FLEET_RESPONSES_API_KEY:-$API_KEY}
MODEL=${FLEET_RESPONSES_MODEL:-$MODEL}

if [ -z "${BASE_URL:-}" ] || [ -z "${API_KEY:-}" ] || [ -z "${MODEL:-}" ]; then
    echo "responses-probe: FLEET_RESPONSES_BASE_URL, FLEET_RESPONSES_API_KEY, and" >&2
    echo "FLEET_RESPONSES_MODEL must resolve (env or $DOTENV) — the named absence, not a guess" >&2
    exit 1
fi

OUT="tmp/responses-probe"
mkdir -p "$OUT"

# One: the non-streamed tool-call turn — the request shape the mapper
# sends, the answer shape the mapper reads.
curl -sS -D "$OUT/non-streamed-headers.txt" "${BASE_URL%/}/responses" \
    -H "Authorization: Bearer $API_KEY" \
    -H "Content-Type: application/json" \
    -d "{
        \"model\": \"$MODEL\",
        \"store\": false,
        \"stream\": false,
        \"instructions\": \"answer briefly\",
        \"input\": [
            {\"role\": \"user\", \"content\": [{\"type\": \"input_text\", \"text\": \"list two files in a tmp directory\"}]}
        ],
        \"tools\": [
            {\"type\": \"function\", \"name\": \"bash\", \"description\": \"run a command\",
             \"parameters\": {\"type\": \"object\", \"properties\": {\"cmd\": {\"type\": \"string\"}}}, \"strict\": false}
        ]
    }" -o "$OUT/non-streamed.json"

# Two: the streamed turn — the typed events the streaming mapper reads,
# argument-delta fragments included.
curl -sS -D "$OUT/streamed-headers.txt" "${BASE_URL%/}/responses" \
    -H "Authorization: Bearer $API_KEY" \
    -H "Content-Type: application/json" \
    -d "{
        \"model\": \"$MODEL\",
        \"store\": false,
        \"stream\": true,
        \"input\": [
            {\"role\": \"user\", \"content\": [{\"type\": \"input_text\", \"text\": \"say hello in five words\"}]}
        ]
    }" -o "$OUT/streamed.sse"

echo "responses-probe: captures under $OUT/ — reconcile against the plan's W2 notes;"
echo "the key is in no capture (the curl headers carry it, and the header files hold only response headers)"
