---
status: completed
created: 2026-10-21
updated: 2026-10-21
slug: document-perception
---

# Plan: documents attached — local extraction, Perception escalation

## Context

The composer can attach **text** files (`f51906c`): the browser reads them, the server frames them ahead of the task, and the conversation records exactly what the owner sent. Binary documents — PDF, DOCX, PPTX, XLSX, images — are refused because there is nothing in fleet that turns them into text.

Most of those files do not need a vendor at all. Word, Excel and PowerPoint are ZIP containers of XML: text, tables, cells and speaker notes extract deterministically, offline, and free. The open-source tools are mature — MarkItDown (MIT) turns docx/xlsx/pptx/pdf/images into Markdown, `python-docx`/`openpyxl`/`python-pptx` (MIT) read the precise fields, LibreOffice converts legacy formats and `pdftotext` reads text PDFs. Tesseract is installed but **not invoked**: the lane does not OCR scans; a scan is the escalation case, which keeps the local path dependency-light and puts OCR quality in one place. What local extraction does *not* do well is layout-heavy PDFs, scanned documents, high-fidelity tables and media; that is where a hosted parser earns its cost.

Nace's Perception API is that escalation. Its document tools sit at `POST https://console.nace.ai/v1/documents/{tool}`; Parse returns Markdown, text, blocks or chunks. Upload is a two-step server-side flow: `POST /v1/documents/upload-grants` returns a `workspace_id`, `upload_url` and `token`; the file is posted to `upload_url` with `X-Upload-Token`; Parse then takes `{"type":"workspace_file","workspace_id":...,"file_id":...}`. Jobs answer `202` with a `job_id` and can be read by id; `?wait_seconds=` holds the call open. The `/v1` routes carry no CORS header, so the call must be fleet's, not the browser's.

The key is **sys1's**, not fleet's. sys1 is the provider lane: it loads `DREX_API_KEY` from its own provider env (`SYS1_ENV_FILE` or `~/.config/sys1/env`), and Perception shares the account key and billing credit with the `/v1/systemone` provider sys1 already carries. The lane exposes the parse as `sys1_perceive_document`; fleet hands it a path and never reads the credential.

The product's egress was already real before this: fleet's roster points at hosted providers, so every turn — and every file the agent reads — goes to whichever model serves it. Perception is a **second recipient**, not the first egress. What the empty state's "Nothing leaves this machine" actually got wrong is the archive-versus-model distinction: the archive stays, the turn does not. That sentence is corrected independently of this plan, and this plan's attach UI names the second recipient before a document is sent.

## Approach

Two extraction paths behind one attach route; the agent still receives text, framed the way text attachments are framed today.

- A document is chosen from the workspace (the pane's tree), not dragged from the desktop in v1, so the file is inside the containment boundary the server already enforces.
- **Local first.** A documents lane — Python, MarkItDown plus LibreOffice for legacy formats — extracts a clean `.docx/.xlsx/.pptx/.csv/.html` or text PDF on the machine. No key, no upload, no cost. Scans are not OCR'd locally; they escalate (C10).
- **Perception second, through sys1.** A scan, a layout-heavy PDF, an image, an email bundle, audio/video, or a local extraction that comes back empty/broken escalates to sys1's `sys1_perceive_document`. The lane holds the credential and runs the vendor's grant/upload/parse/poll flow; fleet hands it the path and caches the returned Markdown.
- The result is cached by content hash plus parser and job id under `<FLEET_HOME>`; the run body carries the parsed text exactly like a text attachment, so the agent path is unchanged.
- The attachment record names the parser that ran: `local markitdown` or `Nace · 96 pages · job …`. Only the second is a document that left the machine, and the disclosure says so before it is sent.

High-level steps:

1. `src/perception.rs`: key resolution, upload grants, multipart upload, parse with `output.formats: ["markdown"]`, bounded wait/poll, result parsing, cache.
2. `src/gui/bridge.rs` + `server.rs`: `POST /api/attach` (workspace path in, parsed attachment out), containment via `pane_path`, keyless refusal.
3. `app.js` / `index.html` / `app.css`: a document picker beside Attach, the disclosure, and the transcript line.
4. Documentation: the scoped privacy sentence, the feature doc, the contract docs.

## Use cases

- [x] A: The owner can attach a PDF or Word document and the agent reads its text without a manual conversion.
- [x] B: A clean Office or text file is extracted locally and costs nothing; only the hard documents — scans, complex layouts, media — go to the vendor.
- [x] C: No key, an unsupported type, a credit refusal, a corrupt file, or a timeout is named with the vendor's code and a next step, never a silent empty attachment.
- [x] D: Before a document is sent, the owner sees that it goes to Perception as well as to the model provider, and after the parse the conversation records what was sent and what it cost.

## Acceptance criteria

These are the contract. Each one is a verifiable assertion. The work is "done" when every box is checked.

- [x] C0: A text attachment takes the existing local path: no Perception call, no key required, and the framed task is byte-identical to today.
- [x] C1: With sys1's `DREX_API_KEY` present, attaching a PDF under the workspace calls the sys1 lane's `sys1_perceive_document`, which issues an upload grant, uploads the file, starts `POST /v1/documents/parse` with Markdown output, and returns the text fleet frames as the attachment.
- [x] C2: A `202` from Parse is polled by job id until a terminal result or a named timeout; a timeout produces a refusal naming the job id, never an empty attachment.
- [x] C3: When Parse returns a preview whose `disclosures` contains `content_truncated` with `original_size` and `retained_size`, the attachment carries the retained Markdown and both the chip and the transcript record say `truncated · retained X of Y`; a scripted response with that disclosure asserts both strings.
- [x] C4: The same file content attached twice makes one billed call: the cache key is the content hash and the second attach returns the cached Markdown and job id.
- [x] C5: A missing key, `unsupported_file_type`, `corrupt_file`, a `409`, and a failed job are each refused with the vendor's code and a named next step.
- [x] C6: The attach UI offers a document picker for the workspace's binary formats, states before sending which parser will handle the file (`local`, or `Nace` and what it receives), and the transcript records the parser, file name, pages, units where billed, and job id where one exists.
- [x] C7: The key is sys1's `DREX_API_KEY`, loaded by the lane from its own provider env; fleet never reads it, no surface prints it, and a lane without it answers `no_key` while local extraction keeps working.
- [x] C8: The empty state and the feature doc state the recipients truthfully — the archive stays on the machine, every turn goes to the providers that serve it (the serving model and any lane providers it calls), and a document escalated to Perception also goes to Nace — and a grep test pins the absence of the unqualified "nothing leaves this machine" sentence.
- [x] C9: A clean `.docx`, `.xlsx`, `.pptx`, `.csv` or `.html` file is extracted by the local documents lane with no Nace request, and the attachment is framed from its Markdown; a scripted no-network test asserts the vendor is never called.
- [x] C10: A scanned PDF, an image, an email/audio/video file, or a local extraction that returns empty or an error escalates to Perception; when the key is absent it is refused with a named reason, and the attachment record says which path ran and why.
- [x] C11: The local extractor runs under a bounded timeout and size, never executes macros or embedded objects, and lists its tools and licenses in the feature doc; a missing `soffice` or `pdftotext` is a named refusal, not a crash. Tesseract is not invoked: a scan escalates to Perception (C10), and the decision is recorded here.

## Files to be touched

**Backend / API:**
- `services/documents/` — new: the local extraction lane (MarkItDown, LibreOffice) and its README/licences.
- `src/documents.rs` — new: the bounded local extraction call and its escalation decision.
- `src/perception.rs` — new: the sys1 lane call and the content-hash cache.
- `services/sys1/src/sys1/perception.py` — new: the vendor flow, using sys1's credential.
- `services/sys1/service/mcp_server.py` — the `sys1_perceive_document` tool.
- `src/gui/bridge.rs` — `POST /api/attach`, key resolution, containment, cache path.
- `src/gui/server.rs` — the attach route and its refusal shapes.

**Frontend:**
- `src/gui/assets/index.html` — the document picker's entry.
- `src/gui/assets/app.js` — picker, disclosure, attach record, transcript line.
- `src/gui/assets/app.css` — the picker and disclosure styles.

**Tests:**
- `services/documents/` — fixtures for docx/xlsx/pptx/legacy and the extraction outputs.
- `src/documents.rs` — local extraction and the escalation decision; no-network assertions.
- `src/perception.rs` — the lane call against a scripted vendor; cache; the lane's `no_key`.
- `src/gui/server.rs` — the attach route's bounds and refusals.

**Documentation (see Linked artifacts):**
- `docs/features/gui.md` — the attach control's two paths and the disclosure.
- `TECHNICAL-DOCUMENTATION.md` — the Perception client and its cache.
- `FUNCTIONAL-SPECIFICATIONS.md` — the conversation's attachment contract and the scoped privacy statement.

## Out of scope

- Dragging arbitrary desktop files into the composer; v1 picks from the workspace tree.
- Perception's Split, Classify, Extract and Ground tools; only Parse.
- Local extraction of audio/video; those escalate to Perception.
- Replacing the local lane with a Rust extractor crate.
- Giving the agent Perception as a tool; the parsed text is the attachment.
- Re-parsing text files through the vendor.
- Storage of the original document beyond the cache; only the Markdown is kept.

## Verification

**C0: text stays local**
```bash
cargo test --lib perception:: gui::server::tests::attachments
```
Expected: the text path frames exactly as today; no request is made without a document.

**C9/C10/C11: local first, escalation second**
```bash
cargo test --lib documents::tests::local
```
Expected: fixtures for docx/xlsx/pptx/csv/html extract locally with no network; a scan and an empty result escalate; a missing tool refuses by name; a macro-bearing file is never executed.

**C1/C2/C3: the pipeline**
```bash
cargo test --lib perception::tests::parse
```
Expected: against a scripted server, the grant → upload → parse → poll sequence is asserted call by call; markdown is framed; a 202 then 200 completes; a timeout names the job; `content_truncated` is disclosed.

**C4: cache and idempotency**
```bash
cargo test --lib perception::tests::cache
```
Expected: the second attach of the same bytes makes no network call and returns the same job id.

**C5/C7: refusals and the key**
```bash
cargo test --lib perception::tests::refusals gui::server::tests::attach_refusals
```
Expected: each named code maps to a sentence; a keyless home refuses a document attach and keeps text.

**C6/C8: the surface and the copy**
Live: the picker lists the workspace's PDFs; the disclosure is visible before sending; the transcript records the parse; the empty state's sentence names Perception.

## Linked artifacts

- `docs/features/gui.md` — the attach control's text path and document path, and the disclosure.
- `TECHNICAL-DOCUMENTATION.md` — the Perception client, its cache key, and its bounds.
- `FUNCTIONAL-SPECIFICATIONS.md` — attachments and the scoped privacy statement.

## Risks

- **Data egress.** The turn already leaves to the model provider; Perception adds a second recipient, and only escalated documents reach it. The disclosure names both and says which path ran.
- **Local toolchain.** The lane depends on MarkItDown's venv plus LibreOffice; a missing tool is a named refusal, not a crash. Tesseract is deliberately not invoked (C11).
- **Table and layout fidelity.** Local Office extraction is reliable for text and cells; merged cells, charts, SmartArt and multi-sheet workbooks are the cases that justify escalation.
- **Cost.** Parse bills by units; the cache and the per-attach transcript line are the guard. No automatic retry that could double-bill.
- **Multipart construction.** ureq has no built-in multipart; the body is assembled with a boundary and the grant's token, and a mock tests the exact bytes.
- **Long jobs.** `wait_seconds` is bounded and polling is capped; a timeout names the job rather than hanging the composer.
- **Key shape.** `nace_sk_` keys are server-side only; the browser never sees the key or the upload token.
- **Cache invalidation.** Keyed by content hash, so an edited file is a new parse; entries are pruned by age and size.

## Notes

- The API contract above is from the public docs (`console.nace.ai/docs/perception/documents` and `/parse`): `POST /v1/documents/{tool}`, `POST /v1/documents/upload-grants`, parse options `output.formats`, `parse_mode`, `page_ranges`, `chunking`, and `wait_seconds`.
- The exact job-read path is documented as `https://console.nace.ai/v1/documents/jobs/{id}/...`; the client will pin it against the live docs before phase 1.
- Vision is **local first**: clean Office/text extracts on the machine; Perception is the escalation. This is the split the owner asked for after the initial Nace-only draft.
- **Decision:** the parse runs through sys1, the lane that already holds the Nace credential; fleet's settings do not list a key it does not read. Perception shares the account key and billing credit with `/v1/systemone`, which sys1 already carries.
- Already installed here: `pandoc`, `pdftotext`, LibreOffice (`soffice`) and `tesseract`; MarkItDown/Docling and the Office Python libraries are not, so the lane owns its venv. **Decision:** no local OCR in v1 — Tesseract stays installed but uninvoked; scans escalate to Perception (C10), so there is one OCR path to trust.
- On this machine the only local models are embedding and lane infrastructure: EmbeddingGemma-2 on `127.0.0.1:8901` for embeddings, sem1's local sentence-transformer worker, Kokoro for speech, and the browser lane. The lanes are local processes that abstract providers rather than local-only: sys1 fans out to Fastino, TypeSafe, Drex, Supersonic, Liquid, CLM, OpenJev, Kev, Together and Laya; sdm1 to TabPFN; gen1 to DashScope and StepFun; blend1 on 8420 is a lane surface, not a chat model. Every chat turn is therefore hosted, and a turn can reach several providers through its lanes, so the attach disclosure names the serving model and any lane providers the turn calls.

## Evidence

- `cargo test --lib` — 1,343 passed; clippy zero. The feature's tests:
  `documents::tests` (9: fixtures for docx/pptx/xlsx/csv/html, text and
  blank PDFs, macro reading, oversize, missing lane, missing `pdftotext`,
  the copy pin), `perception::tests` (4: the sys1 tool's grant → upload →
  parse → poll against a scripted vendor, the cache, a failed job's code,
  and the lane's `no_key`), the bridge's local/needs/escalate decision,
  and the server's attach-route tests.
- Live: sys1's own `sys1_perceive_document` parsed `simple.csv` through
  Nace with the lane's credential (1 unit, Markdown table). The running
  face attached `simple.docx` locally and answered `blank.pdf` with the
  consent question. The picker's pre-read statement was verified against
  a delayed attach (`reading locally · OOXML document (parser:
  local:docx)` painted before the answer), and the empty state names the
  Perception recipient. Settings lists no key fleet does not read.
