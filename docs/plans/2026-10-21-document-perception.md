---
status: draft
created: 2026-10-21
updated: 2026-10-21
slug: document-perception
---

# Plan: documents attached through Nace perception

## Context

The composer can attach **text** files (`f51906c`): the browser reads them, the server frames them ahead of the task, and the conversation records exactly what the owner sent. Binary documents — PDF, DOCX, PPTX, XLSX, images — are refused because there is nothing in fleet that turns them into text.

Nace's Perception API solves that. Its document tools sit at `POST https://console.nace.ai/v1/documents/{tool}`; Parse returns Markdown, text, blocks or chunks. Upload is a two-step server-side flow: `POST /v1/documents/upload-grants` returns a `workspace_id`, `upload_url` and `token`; the file is posted to `upload_url` with `X-Upload-Token`; Parse then takes `{"type":"workspace_file","workspace_id":...,"file_id":...}`. Jobs answer `202` with a `job_id` and can be read by id; `?wait_seconds=` holds the call open. The `/v1` routes carry no CORS header, so the call must be fleet's, not the browser's.

Fleet already carries Nace as a provider (`drex.nace.ai/v1/systemone`), and the docs say Perception shares the account key and billing credit. The key is not assumed: `NACE_API_KEY` (`nace_sk_…`) is the name this plan reads, with `DREX_API_KEY` as the existing fallback.

What this changes in the product's promise: a hosted parse means a document **leaves this machine**. The empty state says "Nothing leaves this machine"; that sentence must become scoped, and the attach UI must say so before the file is sent.

## Approach

Add a server-side Perception client and one attach route; the agent still receives text, framed the way text attachments are framed today.

- A document is chosen from the workspace (the pane's tree), not dragged from the desktop in v1, so the file is inside the containment boundary the server already enforces.
- The server uploads and parses it, caches the Markdown by content hash plus job id under `<FLEET_HOME>`, and returns an attachment record. The run body carries the parsed text exactly like a text attachment, so the agent path is unchanged.
- Text files keep the local, free path: no key, no upload, no call.
- The disclosure is part of the attachment chip and the transcript: file, pages, units, and the job id.

High-level steps:

1. `src/perception.rs`: key resolution, upload grants, multipart upload, parse with `output.formats: ["markdown"]`, bounded wait/poll, result parsing, cache.
2. `src/gui/bridge.rs` + `server.rs`: `POST /api/attach` (workspace path in, parsed attachment out), containment via `pane_path`, keyless refusal.
3. `app.js` / `index.html` / `app.css`: a document picker beside Attach, the disclosure, and the transcript line.
4. Documentation: the scoped privacy sentence, the feature doc, the contract docs.

## Use cases

- [ ] A: The owner can attach a PDF or Word document and the agent reads its text without a manual conversion.
- [ ] B: A plain text file stays local and costs nothing; only binary documents go to the vendor.
- [ ] C: No key, an unsupported type, a credit refusal, a corrupt file, or a timeout is named with the vendor's code and a next step, never a silent empty attachment.
- [ ] D: Before a document is sent, the owner sees that it leaves the machine, and after the parse the conversation records what was sent and what it cost.

## Acceptance criteria

These are the contract. Each one is a verifiable assertion. The work is "done" when every box is checked.

- [ ] C0: A text attachment takes the existing local path: no Perception call, no key required, and the framed task is byte-identical to today.
- [ ] C1: With `NACE_API_KEY` present, attaching a PDF under the workspace issues an upload grant, uploads the file, starts `POST /v1/documents/parse` with `output.formats` `["markdown"]`, and frames the returned Markdown as the attachment.
- [ ] C2: A `202` from Parse is polled by job id until a terminal result or a named timeout; a timeout produces a refusal naming the job id, never an empty attachment.
- [ ] C3: When Parse returns a preview whose `disclosures` contains `content_truncated` with `original_size` and `retained_size`, the attachment carries the retained Markdown and both the chip and the transcript record say `truncated · retained X of Y`; a scripted response with that disclosure asserts both strings.
- [ ] C4: The same file content attached twice makes one billed call: the cache key is the content hash and the second attach returns the cached Markdown and job id.
- [ ] C5: A missing key, `unsupported_file_type`, `corrupt_file`, a `409`, and a failed job are each refused with the vendor's code and a named next step.
- [ ] C6: The attach UI offers a document picker for the workspace's binary formats, states that the file is sent to Nace before it is sent, and the transcript records file name, pages, units, and job id.
- [ ] C7: The key is read from `NACE_API_KEY`, falling back to `DREX_API_KEY`, through the vault/dotenv readers; no surface prints it, and a keyless home keeps text-only attachments.
- [ ] C8: The empty state and the feature doc no longer claim that nothing leaves the machine without qualification; the scoped sentence names Perception and what it receives.

## Files to be touched

**Backend / API:**
- `src/perception.rs` — new: upload grants, multipart upload, parse, job polling, bounds, cache.
- `src/gui/bridge.rs` — `POST /api/attach`, key resolution, containment, cache path.
- `src/gui/server.rs` — the attach route and its refusal shapes.
- `src/settings.rs` — the key's presence reflected where secrets are listed (name only).

**Frontend:**
- `src/gui/assets/index.html` — the document picker's entry.
- `src/gui/assets/app.js` — picker, disclosure, attach record, transcript line.
- `src/gui/assets/app.css` — the picker and disclosure styles.

**Tests:**
- `src/perception.rs` — the upload/parse/poll flow against a scripted server; cache; each refusal.
- `src/gui/server.rs` — the attach route's bounds and refusals.

**Documentation (see Linked artifacts):**
- `docs/features/gui.md` — the attach control's two paths and the disclosure.
- `TECHNICAL-DOCUMENTATION.md` — the Perception client and its cache.
- `FUNCTIONAL-SPECIFICATIONS.md` — the conversation's attachment contract and the scoped privacy statement.

## Out of scope

- Dragging arbitrary desktop files into the composer; v1 picks from the workspace tree.
- Perception's Split, Classify, Extract and Ground tools; only Parse.
- Giving the agent Perception as a tool; the parsed text is the attachment.
- Re-parsing text files through the vendor.
- Storage of the original document beyond the cache; only the Markdown is kept.

## Verification

**C0: text stays local**
```bash
cargo test --lib perception:: gui::server::tests::attachments
```
Expected: the text path frames exactly as today; no request is made without a document.

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

- **Data egress.** The first hosted parse contradicts the current empty-state sentence; the copy change is part of the work, not a follow-up.
- **Cost.** Parse bills by units; the cache and the per-attach transcript line are the guard. No automatic retry that could double-bill.
- **Multipart construction.** ureq has no built-in multipart; the body is assembled with a boundary and the grant's token, and a mock tests the exact bytes.
- **Long jobs.** `wait_seconds` is bounded and polling is capped; a timeout names the job rather than hanging the composer.
- **Key shape.** `nace_sk_` keys are server-side only; the browser never sees the key or the upload token.
- **Cache invalidation.** Keyed by content hash, so an edited file is a new parse; entries are pruned by age and size.

## Notes

- The API contract above is from the public docs (`console.nace.ai/docs/perception/documents` and `/parse`): `POST /v1/documents/{tool}`, `POST /v1/documents/upload-grants`, parse options `output.formats`, `parse_mode`, `page_ranges`, `chunking`, and `wait_seconds`.
- The exact job-read path is documented as `https://console.nace.ai/v1/documents/jobs/{id}/...`; the client will pin it against the live docs before phase 1.
- Perception shares the account key and billing credit with `/v1/systemone`, which is the provider fleet already carries; `DREX_API_KEY` is the fallback name.
