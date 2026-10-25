#!/usr/bin/env bash
# E0's live probe for the extraction leg, in its sys1 shape (the
# 2026-10-25 re-point; plan 2026-10-19-sys1-extraction-leg.md).
#
# Reads FASTINO_API_KEY from the environment (or a dotenv-style file
# passed as $1), runs the leg's two passes over a representative
# invoice — the locate pass (Fastino GLiNER 2.5 Multi entities) and the
# verify pass (glide on /v1/systemone) — captures the raw
# request/response JSON under tmp/ (never committed, no key in it), and
# prints per-pass latency and status. The key is never printed; the
# captured files carry only the wire JSON.
#
#   scripts/extract-probe.sh [dotenv-file]
#
# Optional env: DREX_API_KEY / TYPESAFE_API_KEY extend the client's
# verify chain; this probe exercises the primary verifier, glide.

set -euo pipefail

if [[ -n "${1:-}" ]]; then
  set -a; source "$1"; set +a
fi
: "${FASTINO_API_KEY:?set FASTINO_API_KEY}"

OUT="tmp/extract-probe"
mkdir -p "$OUT"

STATE='Invoice INV-2026-0042 is due on 2026-11-30. The total due is $4,180.00 payable within 30 days. Late fees apply after the due date at 1.5 percent monthly. Net terms are net-45 for approved accounts.'

printf '%s\n' "=== pass 1: locate (fastino/gliner2.5-multi-v1 entities) ==="
LOCATE_BODY=$(jq -n --arg state "$STATE" '{
  model: "fastino/gliner2.5-multi-v1",
  messages: [{role: "user", content: $state}],
  schema: {entities: [
    {name: "invoice_id", description: "the invoice identifier written in the text"},
    {name: "total_due",  description: "the total amount due written in the text"},
    {name: "due_date",   description: "the due date written in the text"},
    {name: "net_terms",  description: "the payment terms length written in the text"}
  ]},
  threshold: 0.5, include_confidence: true, include_spans: true, store: false
}')
LOCATE=$(curl -sS --max-time 30 -w '\n%{http_code} %{time_total}s' \
  -X POST https://api.fastino.ai/v1/chat/completions \
  -H "Authorization: Bearer ${FASTINO_API_KEY}" \
  -H 'Content-Type: application/json' \
  -d "$LOCATE_BODY")
printf '%s\n' "$LOCATE" | sed '$d' > "$OUT/locate.json"
jq '.choices[0].message.content | fromjson' < "$OUT/locate.json"
printf 'locate: %s\n' "$(printf '%s' "$LOCATE" | tail -1)"

printf '%s\n' "=== pass 3: verify (fastino/glide, /v1/systemone nouls) ==="
VERIFY_BODY=$(jq -n --arg state "$STATE" '{
  model: "fastino/glide",
  state: $state,
  questions: {
    invoice_id: {type: "noul", instructions: "Does this text state the invoice identifier?", criteria: {true: "Yes, the invoice identifier is stated here.", false: "No."}},
    total_due:  {type: "noul", instructions: "Does this text state the total amount due?",     criteria: {true: "Yes, the total amount due is stated here.",  false: "No."}},
    due_date:   {type: "noul", instructions: "Does this text state the due date?",             criteria: {true: "Yes, the due date is stated here.",          false: "No."}},
    net_terms:  {type: "noul", instructions: "Does this text state the payment terms length?", criteria: {true: "Yes, the payment terms length is stated here.", false: "No."}}
  }
}')
VERIFY=$(curl -sS --max-time 30 -w '\n%{http_code} %{time_total}s' \
  -X POST https://api.fastino.ai/v1/systemone \
  -H "Authorization: Bearer ${FASTINO_API_KEY}" \
  -H 'Content-Type: application/json' \
  -d "$VERIFY_BODY")
printf '%s\n' "$VERIFY" | sed '$d' > "$OUT/verify.json"
jq '.answers' < "$OUT/verify.json"
printf 'verify: %s\n' "$(printf '%s' "$VERIFY" | tail -1)"

echo "---"
echo "Reconcile the captured shapes against the client (src/extract.rs), record the"
echo "latencies and every delta in the plan's E0 section: these files are the"
echo "evidence the leg compiles against. The key rides nowhere in them."
