#!/bin/zsh
# E0's wire probe for the extraction leg (plan 2026-10-19-sys1-extraction-leg.md).
#
# Reads CLOUDFLARE_ACCOUNT_ID and WORKERS_AI_API_TOKEN from the environment
# (or a dotenv-style file passed as $1), calls the hosted Clairic model once
# per verb over a representative unstructured document, captures the raw
# request/response JSON under tmp/ (never committed, no token in it), and
# prints per-verb latency. E0-C1/C2/C3 run in one command:
#
#   scripts/extract-probe.sh [dotenv-file]
#
# The token is never printed; the captured files carry only the wire JSON.

set -euo pipefail

if [[ -n "${1:-}" ]]; then
  set -a; source "$1"; set +a
fi

: "${CLOUDFLARE_ACCOUNT_ID:?set CLOUDFLARE_ACCOUNT_ID}"
: "${WORKERS_AI_API_TOKEN:?set WORKERS_AI_API_TOKEN}"

MODEL="${EXTRACT_MODEL:-@cf/cloudflare/clairic}"
URL="https://api.cloudflare.com/client/v4/accounts/${CLOUDFLARE_ACCOUNT_ID}/ai/run/${MODEL}"
OUT="tmp/extract-probe"
mkdir -p "$OUT"

STATE='Invoice INV-2026-0042 from Northwind Traders, dated March 14 2026.
Total due: $4,180.00 including 9.5% tax on the $3,816.44 of services.
Delivery was confirmed on March 21 2026; payment terms net-30 from that date.
Contact: billing@northwind.example. Reference the PO 88-BLUE.'

probe() {
  local verb="$1" fields="$2"
  local body start ms
  body=$(printf '{"state": %s, "fields": %s}' \
    "$(python3 -c 'import json,sys; print(json.dumps(sys.stdin.read()))' <<< "$STATE")" \
    "$fields")
  start=$(python3 -c 'import time; print(time.monotonic_ns())')
  curl -sS --max-time 30 "$URL" \
    -H "Authorization: Bearer ${WORKERS_AI_API_TOKEN}" \
    -H 'Content-Type: application/json' \
    -d "$body" -o "$OUT/$verb.json"
  ms=$(( ($(python3 -c 'import time; print(time.monotonic_ns())') - start) / 1000000 ))
  echo "$verb: ${ms} ms -> $OUT/$verb.json"
}

probe extract '{"fields": {
  "invoice_id":   {"type": "text", "instructions": "the invoice number"},
  "issuer":       {"type": "text", "instructions": "the company that issued the invoice"},
  "total_due":    {"type": "number", "decimals": 2, "unit": "USD"},
  "invoice_date": {"type": "date"},
  "due_terms":    {"type": "duration", "unit": "days", "instructions": "payment terms in days"},
  "contact":      {"type": "text", "instructions": "the billing contact email"}
}}'

probe compute '{"fields": {
  "net_amount":  {"type": "number", "decimals": 2, "instructions": "the amount before tax, from total and tax rate"},
  "tax_paid":    {"type": "number", "decimals": 2, "instructions": "the tax amount"}
}}'

probe estimate '{"fields": {
  "expected_total": {"type": "estimate", "min": 3000, "max": 5000, "step": 10,
                     "instructions": "the plausible total for an invoice like this one"}
}}'

probe forecast '{"fields": {
  "next_quarter_total": {"type": "forecast", "horizon": 3, "interval": 80,
                         "instructions": "the invoice total trend projected three steps ahead"}
}}'

echo "---"
echo "Reconcile the captured shapes against the documented contract, record the"
echo "latencies and every delta in the plan's E0 section, then delete nothing:"
echo "these files are the evidence E1 compiles against."
