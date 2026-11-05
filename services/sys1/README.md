# sys1 — the judgment lane (vendored)

The System One tier Fleet calls for judgments. This copy is vendored
from `~/Projects/sys1` at the sha in `VENDOR.json`; re-vendor by
copying `src/` and `service/mcp_server.py` at a newly recorded sha, and
carry the `fleetVendor.fleetAdditions` with it.

## The MCP surface (stdio, MCP 2024-11-05)

| Tool | What it answers |
|---|---|
| `sys1_classify` | one classification over `text` — a registered `task`, or an inline `task_spec` (id + heads), on one `provider`, an alias (`all`/`both`), a comma list, or nothing (the lane routes per `providers.route`) |
| `sys1_perceive_document` | a document's text, through the hosted parse (`sys1.perception`) — Fleet's carried module |
| `sys1_extract` | entity spans out of `text` for a list of `fields` |

## `sys1_extract`

The entity pass. Fleet's extraction leg asks this instead of talking to
a vendor itself (`docs/plans/2026-11-05-extraction-through-sys1.md`):
the vendor, the endpoint, the model and the key are this service's.

- **Request**: `{"text": "...", "fields": [{"name": "invoice_id",
  "description": "the invoice number"}, ...]}`.
- **Answer**: `{"ok": true, "spans": [{"field", "text", "start", "end",
  "confidence"}], "declined": [...], "problems": [...], "model",
  "telemetry"}`.
- **Rules**: every span carries the offsets it was read at, and a span
  whose `[start, end)` does not slice back to its own text is DROPPED
  and recorded in `problems` — a wrong offset is worse than no span. A
  field with no candidate is DECLINED by name; a failed call is
  `{"ok": false, "code": "no_key" | "empty" | "parse" | "transport"}`,
  never an empty read.
- **Configuration**: `providers.extract_api_url`, `extract_model`
  (default `fastino/gliner2.5-multi-v1`) and `extract_api_key_env`
  (default `FASTINO_API_KEY`), overridable with `SYS1_EXTRACT_URL`,
  `SYS1_EXTRACT_MODEL`, `SYS1_EXTRACT_API_KEY`.
- **Its own line**: when `classify.log_dir` (or `SYS1_LOG_DIR`) is set,
  each call appends one `op: "extract"` JSONL row to
  `<log_dir>/YYYY/MM/DD/events.jsonl` — counts, the model, the
  fingerprint and the telemetry, never the document or the span text.

## `sys1_classify` with `task_spec`

An inline spec (`{"id": "...", "heads": [{"id", "kind", "task",
"descriptions"}]}`) IS the task: it wins over `task`, and rides the same
routing and cascade a registered task gets. A dict without an `id` is
not a spec, so the registered name stands.

## Tests

`python3 -m unittest discover -s tests -t .` from this directory. The
tests are Fleet's own and stdlib-only — no vendor call, no pytest
install required.
