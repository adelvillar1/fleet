---
name: fleet-document-perception-plan
description: "2026-10-21 — document attachments are IMPLEMENTED: a local lane reads Office/CSV/HTML/text-PDF offline, and scans escalate through SYS1 to Nace Perception (35a3335), the lane owning the DREX key; fleet never reads that key, the picker states its parser before the read, 1,343 tests, live-verified"
type: project
date: 2026-11-04
basis: inferred
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

**The ask (2026-10-21):** "regarding the attachment of documents, I think it would make
sense to use the perception product from nace.ai
https://console.nace.ai/docs/perception/documents." Binary documents were refused
because fleet had nothing that turns them into text; Perception Parse does.

**The vendor contract (worth not re-fetching):** `POST
https://console.nace.ai/v1/documents/parse`, bearer key. Upload is two-step —
`POST /v1/documents/upload-grants` with `{"path": ...}` returns
`workspace_id`, `upload_url`, `token`; the file posts to `upload_url` with header
`X-Upload-Token` and multipart field `file`; Parse takes
`{"type":"workspace_file","workspace_id","file_id"}`. Jobs answer `202` with
`job_id`; `GET /v1/documents/jobs/{id}` is terminal on
`succeeded|failed|cancelled`; `?wait_seconds=` holds the call open. Results over
1 MiB come back as a preview cut to 65,536 chars with a `content_truncated`
disclosure. No CORS on `/v1` — the call is server-side.

**Owner ruling that shaped the code: call sys1, never read its key.** The first
build was a fleet-side client reading `NACE_API_KEY`/`DREX_API_KEY` from fleet's
own readers, which refused on this machine. The owner: "there is, in sys1" →
"no, you should be calling sys1 as it's already a part of fleet" → "fix what
needs to be fixed and use the correct lane, that's why they were built to begin
with." sys1 already fronts the providers and owns the credential.

**Shipped (`35a3335`, sys1-backed rewrite of `5782497`/`37b93f3`; copy fix
`84be929`; plan `completed`):**
- `services/sys1/src/sys1/perception.py` runs grant → multipart → parse → poll
  with sys1's `DREX_API_KEY` via `api_key_for("drex", cfg)`; exposed as the
  `sys1_perceive_document` MCP tool in `services/sys1/service/mcp_server.py`.
- `src/perception.rs` is now only a lane caller + cache: spawns the sys1 MCP
  server, calls the tool with `{path,name}`, caches by FNV-1a content hash under
  `<FLEET_HOME>/perception-cache`, 300 s call budget.
- `services/documents/extract.py` reads OOXML/CSV/HTML/legacy-Office/text-PDF
  offline (`src/documents.rs` routes by extension, 64 MB / 120 s bounds); only
  an empty extract or no local reader escalates.
- `POST /api/attach` answers `{type:"attach_needs", reason}` first and calls
  Nace only on a second request with `consent:true`.
- The Document picker states the parser **before** the read
  ("reading locally · csv (parser: local:csv)"), names the recipient on
  escalation, and the empty state pins the recipients: "turns go to the
  providers that serve them; a document escalated to Perception goes to Nace."

**Live-verified at 127.0.0.1:3080 (Playwright, 2026-10-21):** a held
`/api/attach` shows the parser note painted before the answer; `simple.csv`
attaches as `local:csv · 57 B`; `blank.pdf` asks "pdftotext found no text; the
PDF is probably a scan — sending it to Nace perception sends the document to a
second party." with the consent buttons, and cancel clears it. The live sys1
parse of a CSV succeeded earlier (units 1, markdown table). Gate: 1,343 tests,
clippy and fmt clean.

**The verifier's deviation, and the decision it forced:** the plan claimed
Tesseract handled scans while the lane never invokes it. Ruling recorded in the
plan's C11: **no local OCR in v1** — Tesseract stays installed but uninvoked,
scans escalate. General rule: when the implementation chooses differently from
the plan, record the decision **in the plan**, not only beside the code.

**Local files stay local.** The offline lane has no network path; only an
escalated document reaches Nace, and only after the consent click. Note the
premise the owner corrected earlier: a hosted model already receives every turn
the agent takes, so Perception is a **second** recipient, not the first egress —
see [[fleet-privacy-claim-must-name-the-recipients]].

Related: [[fleet-composer-needs-permissions-and-a-route-pin]],
[[plan-is-a-file-not-prose]], [[fleet-vault-credentials]],
[[fleet-gui-surface-grammar-rollout]].
