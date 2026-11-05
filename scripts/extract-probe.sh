#!/usr/bin/env bash
# The extraction leg's live probe — through sys1, the way Fleet calls it.
#
# The leg has no vendor client of its own (the owner's 2026-11-05 ruling;
# plan 2026-11-05-extraction-through-sys1.md), so this probe does what the
# leg does: it drives the vendored sys1 lane over stdio JSON-RPC —
# `sys1_extract` for the locate pass, `sys1_classify` with an inline
# `task_spec` for the verify pass — and captures the raw replies under
# tmp/ (never committed, no key in them). The vendor, endpoint, model and
# key are the lane's; this script only carries a key into the lane's
# environment the way Fleet's passthrough does.
#
#   scripts/extract-probe.sh [dotenv-file]
#
# Optional env: DREX_API_KEY / TYPESAFE_API_KEY let the lane's verify
# chain walk further; this probe asks every provider the lane holds.

set -euo pipefail

if [[ -n "${1:-}" ]]; then
  set -a; source "$1"; set +a
fi
: "${FASTINO_API_KEY:?set FASTINO_API_KEY}"

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="tmp/extract-probe"
mkdir -p "$OUT"

STATE='Invoice INV-2026-0042 is due on 2026-11-30. The total due is $4,180.00 payable within 30 days. Late fees apply after the due date at 1.5 percent monthly. Net terms are net-45 for approved accounts.'

LOCATE_ARGS=$(jq -n --arg state "$STATE" '{
  text: $state,
  fields: [
    {name: "invoice_id", description: "the invoice identifier written in the text"},
    {name: "total_due",  description: "the total amount due written in the text"},
    {name: "due_date",   description: "the due date written in the text"},
    {name: "net_terms",  description: "the payment terms length written in the text"}
  ]
}')

VERIFY_SPEC=$(jq -n '{
  id: "extract-verify",
  heads: [
    {id: "invoice_id", kind: "noul", task: "Does this text state the invoice identifier?", descriptions: {true: "Yes, the invoice identifier is stated here.", false: "No."}},
    {id: "total_due",  kind: "noul", task: "Does this text state the total amount due?",    descriptions: {true: "Yes, the total amount due is stated here.",  false: "No."}},
    {id: "due_date",   kind: "noul", task: "Does this text state the due date?",            descriptions: {true: "Yes, the due date is stated here.",          false: "No."}},
    {id: "net_terms",  kind: "noul", task: "Does this text state the payment terms length?", descriptions: {true: "Yes, the payment terms length is stated here.", false: "No."}}
  ]
}')

# One session, three requests: the handshake, then the leg's two passes.
# `provider: "all"` is the leg's adjudication shape — every provider the
# lane holds answers, and the probe shows which of them did.
if ! {
  printf '%s\n' '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}'
  jq -nc --argjson args "$LOCATE_ARGS" \
    '{jsonrpc:"2.0", id:2, method:"tools/call", params:{name:"sys1_extract", arguments:$args}}'
  jq -nc --arg text "$STATE" --argjson spec "$VERIFY_SPEC" \
    '{jsonrpc:"2.0", id:3, method:"tools/call", params:{name:"sys1_classify", arguments:{text:$text, task_spec:$spec, provider:"all"}}}'
} | SYS1_PYTHONPATH="$ROOT/services/sys1/src" \
    python3 "$ROOT/services/sys1/service/mcp_server.py" \
    > "$OUT/replies.jsonl" 2> "$OUT/stderr.log"; then
  echo "the sys1 lane could not be driven — its stderr:" >&2
  tail -3 "$OUT/stderr.log" >&2
  exit 1
fi

# A missing or refused reply must stop the probe by name: printing
# nothing and exiting 0 would read as a pass on a call that never
# happened.
reply() {
  local id="$1" label="$2"
  local line
  line=$(jq -c "select(.id==$id)" "$OUT/replies.jsonl")
  if [[ -z "$line" ]]; then
    echo "$label: the lane sent no reply — see $OUT/stderr.log" >&2
    exit 1
  fi
  if [[ "$(jq -r 'has("error")' <<<"$line")" == "true" ]]; then
    echo "$label: the lane refused — $(jq -r '.error.message' <<<"$line")" >&2
    exit 1
  fi
  jq -c '.result.content[0].text | fromjson' <<<"$line"
}

echo "=== pass 1: locate (sys1_extract → the lane's entity vendor) ==="
LOCATE_REPLY=$(reply 2 locate)
if [[ "$(jq -r '.ok' <<<"$LOCATE_REPLY")" != "true" ]]; then
  echo "locate: the verb answered not-ok — $(jq -r '.code // "?"' <<<"$LOCATE_REPLY"): $(jq -r '.reason // ""' <<<"$LOCATE_REPLY")" >&2
  exit 1
fi
jq . <<<"$LOCATE_REPLY"

echo "=== pass 3: verify (sys1_classify + task_spec, every provider) ==="
VERIFY_REPLY=$(reply 3 verify)
jq '{verdict, provider, providers_used, escalated, answers, latency_ms}' <<<"$VERIFY_REPLY"

echo "---"
echo "Raw replies: $OUT/replies.jsonl (stderr: $OUT/stderr.log)."
echo "The lane's own line — op \"extract\", model, spans, latency — lands in its"
echo "JSONL log (classify.log_dir / SYS1_LOG_DIR); that is the proof the vendor"
echo "call is sys1's, not this script's. The key rides nowhere in these files."
