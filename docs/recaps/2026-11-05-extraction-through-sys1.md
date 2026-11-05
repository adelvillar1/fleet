# Session recap — 2026-11-05: the extraction leg goes through sys1

**Summary.** The owner's ruling — *sys1 exists to abstract the System
One providers, so no caller may talk to a vendor directly* — is now
true of the last violator. The extraction leg stopped being its own
vendor client: it asks `sys1_extract` for the entity pass and
`sys1_classify` with an inline `task_spec` for VERIFY, while its own
judgment (segmentation, the 0.85/0.35 floors, the bounded adjudication,
absence as a named fact, the quote rule) stays in Rust unchanged. The
plan completed 9 of 9, and the live proof found four defects — three in
the desk's store, one introduced by the move itself.

## What was built

- **The verb** — `services/sys1/src/sys1/extraction.py`. One entity call
  against the configured vendor (schema of named entities,
  `threshold 0.5`, `include_confidence`, `include_spans`,
  `store: false`), parsing the reply into `{field, text, start, end,
  confidence}`. A span whose `[start, end)` does not slice back to its
  own text is DROPPED and named in `problems`; a field with no
  candidate is DECLINED by name; a failure is a code
  (`no_key` / `empty` / `parse` / `transport`), never an empty read.
  The vendor, model and key come from the config's own
  `extract_api_url` / `extract_model` / `extract_api_key_env`, so the
  config entry is the authority (it was dead on the first pass and the
  live read caught it).
- **The lane's own line.** Each call appends one `op: "extract"` JSONL
  row beside classify's — model, span count, declines, the input
  fingerprint and the telemetry, never the document or the span text.
  That line is how the live proof shows the call is sys1's.
- **The seam** — `sys1_classify` takes an optional `task_spec` (the HTTP
  face's `task_from_spec`), and `task_for_arguments` is module-level so
  the seam is testable without a stdio session.
- **The leg** — `src/extract.rs` is a lane client: `ExtractSession`
  spawns the sys1 lane, `locate` calls `sys1_extract`, `verdicts` calls
  `sys1_classify` per segment, `strongest` asks once across every
  provider for a contested span. No vendor host string and no key
  resolution remain in the file. `LaneSource` moved to `src/lanes.rs`
  (where the seam can be named without the sync-path pins seeing a
  judgment beat).
- **The tests** — the leg's transport pins are mock-MCP scripted
  (`the_extraction_leg_reads_through_the_sys1_lane`,
  `the_extraction_legs_floors_veto_and_adjudicate_through_the_lane`),
  and the live one is `--ignored` because it spends vendor credit:
  `the_extraction_leg_reads_a_real_document_through_the_vendored_lane`.
  sys1's own 17 tests are stdlib `unittest` (pytest is not installed on
  this machine — the runner that actually ran is recorded in the plan).

## What the landing found

1. **A byte step in `metric_numbers`** (`src/desk.rs`): an em dash in a
   real draft made the number scan slice mid-character and panic the
   review stage. Now it steps one character.
2. **A relative source silently skipped**: `sources/brief.txt` resolved
   against the process CWD, not the desk project, so a correct brief
   read as zero claims with the stage `done`. Relative paths resolve
   under the desk project now, and an unreadable source is a named
   refusal.
3. **`mark_stage` no-opped on a store without the entry**: a whole walk
   could run with nothing landing. It inserts in walk order.
4. **The provider-keyed `answers` reply broke the desk's two readers** —
   introduced by this move, caught by the live walk reading `0.0` for
   every head. serde_json sorts keys, so "the first map" was never "the
   provider that answered"; `provider_map` reads the answering provider.
   An escalated answer's empty provider name also stopped reaching
   `models` (`record_model`).

## The live proof

On a scratch home with the machine's keys:

- `fleet desk --run brief` walked all five stages on a real invoice
  brief; the claims carried real probabilities (0.503 / 0.045 / 0.425)
  with their kind labels.
- The leg through the VENDORED lane read
  `invoice_id INV-2026-114` at p **0.9939**, `total_due $12,480.00` at
  p **0.9929**, `due_date 2026-11-11` at p **0.9984**, and
  `credit_note_total` **absent by name** ("no candidate span above the
  entity threshold"). Stats: 5 segments, 4 verify calls, **1 vetoed**,
  0 adjudications; models `fastino/gliner2.5-multi-v1`, `glide`, `drex`,
  `jev` — the lane chose the chain.
- The lane's own log line:
  `{"op": "extract", "ok": true, "model": "fastino/gliner2.5-multi-v1",
  "input_chars": 393, "spans": 4, "declined": ["credit_note_total"],
  "fields": ["due_date", "invoice_id", "total_due"], "latency_ms": 276,
  "http_status": 200}`.

## Gate

fmt 0, clippy 0, `cargo build --release` 0, `cargo test` 0 — lib
1,865/0, integration 168/0 (4 ignored, one of them the live proof) —
sys1's own `python3 -m unittest discover -s tests -t .` 17/17.

## Follow-ups named, not hidden

- The desk's claim pass asks the lane's **choice head per segment**
  (kind labels), not the entity verb. That is its own shape and the
  floors it applies are the same constants; using LOCATE there would be
  a different design and is not this plan.
- The suite's lib half measured 80 s in this run against 36–43 s in
  earlier ones on the same machine — load, not a new slow test (no test
  reported a long duration); worth a look if it holds at three digits.
