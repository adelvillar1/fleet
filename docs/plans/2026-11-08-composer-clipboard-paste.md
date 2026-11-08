---
status: active
created: 2026-11-08
updated: 2026-11-08
slug: composer-clipboard-paste
---

# Plan: the composer's clipboard paste — text and image

## Context

The owner asked for pasting from the clipboard into the composer. Tested
empirically in the running desktop app (2026-11-08): **text paste works** —
with the composer focused, Cmd+V landed a clipboard marker in the textarea —
but **pasting an image does nothing**: with a PNG on the clipboard, Cmd+V left
the composer empty and the refusal notice the code promises ("an image cannot
ride the composer yet") never appeared. The composer's `paste` handler
(`src/gui/assets/app.js:10275`) attaches text files and refuses images, but the
image branch is unreachable in the webview: the native image paste into a
`<textarea>` is a no-op, so the `paste` event either does not fire or arrives
with an empty `clipboardData`.

**Owner ruling, 2026-11-08: this plan is needed and is to be implemented.**

### What the recovery audit corrected

This plan was recovered from `stash@{0}^3` (the 2026-11-08 stash). Three of its
premises did not survive a read of the tree, and are corrected here rather than
inherited:

| Claim as drafted | What the tree says |
|---|---|
| the paste handler is at `app.js:9806` | it is at `app.js:10275`; the file is 26,480 lines |
| "neither `Message::text_with_images` nor the `vision` workload is wired to a run" | `Work::Vision` (`src/agent.rs:6216`) already builds its message with `text_with_images` (`src/agent.rs:6241`) and walks `route::workload_decision("vision")`. What does **not** exist is a *run's own first message* carrying an image — the vision path today is reachable only through the vision **tool**, not through a task with an attachment |
| "remove `MAX_BODY_BYTES` (KI-378's 64 KiB cap) — it was an artifact of an earlier era" | **The owner confirmed this on 2026-11-08 and it is now done — KI-475.** The cap was real and was KI-378's *closed* send half, so it was not removed silently: the wording KI-378 closed is kept verbatim and its pin now asserts that wording at the new bound, while the number rose to 25 MiB. The audit also found the bound *below* it — 256 KB per attachment, 512 KB together — unreachable, because the smaller number always won; all three moved together |

The third row was the one place the audit had to defer to the owner: KI-378 had
closed that number, and a closed row is not reopened by an agent. **The owner
settled it on 2026-11-08 — the caps are artificial, the models carry a
million-token context, and a run must carry several screenshots — so the caps
are raised, not worked around.** That raise is already landed (KI-475) and C4
below now verifies it rather than proposing it.

## Approach

Add a **paste affordance** to the composer's foot (beside the attach button),
with a new `ICONS` key. The button reads the clipboard through a new server
endpoint and inserts text or attaches an image. The read lives in Rust, not
JavaScript: the webview will not hand an image to the page, so
`GET /api/clipboard` reads the system clipboard (`arboard`). One endpoint
serves both surfaces — the desktop shell and the browser face run the same
server — and the call rides the user's click as its gesture.

**The image travels as bytes.** The endpoint returns `{text}` or
`{image: {name, mime, bytes}}` (base64), and the image rides the `/api/run`
body with the task. That body is no longer squeezed through a 64 KiB pipe: the
request cap is 25 MiB, one attachment may be 8 MiB, eight may ride one turn,
and the turn's attachments total at most 20 MiB (KI-475, landed). The
composer's own limits mirror those numbers, so the page refuses exactly what
the server would rather than inventing a tighter one.

The composer's attachments gain an image kind beside the text kind: the chip
and the send payload's image entry (`name`, `mime`, `bytes`). The run builds its
request with `Message::text_with_images` when an image is attached, so the
model sees the screenshot with its instruction. The router's workload for an
image-bearing run is forced to `vision` — a task carrying an image must not be
judged by a text-only chain — so the route decision reads the attachment, not
only the words.

The bot/remote face has no paste handler at all; the same affordance rides its
composer too, so the two faces cannot drift.

Decision-model check: no sys1 fit — the clipboard read and the image detection
are deterministic. The router's existing `vision` workload is reused; no new
judgment is needed.

High-level steps:

0. **The mockup first (C0).** `docs/plans/mockups/2026-11-08-composer-clipboard-paste/` holds the composer's foot with the paste button as static HTML in the app's own tokens, measured rendered in every theme (no overflow, no clipped label, ink ≥ 4.5:1), the PNGs read in a light and a dark theme, and the owner reads it before any code of C1+ lands.
1. **The clipboard endpoint (C1).** `src/clipboard.rs` holds the reader behind a small backend trait (arboard in production, a fixture in tests), and `GET /api/clipboard` returns `{text}` or `{image: {name, mime, bytes}}`.
2. The composer's paste button: text inserts, an image attaches (C2, C3).
3. The send payload carries the image (C4).
4. The run's request carries the image — `text_with_images` (C5).
5. The route decision forces `vision` for an image-bearing run (C6).
6. The bot face's paste (C7).
7. The gate (C8).
8. The docs (C9).

The house mockups rule applies to the new affordance: it is drawn and measured
before the code. Run `scripts/edit-preflight.sh` before the first compile — the
icon source-pin counts (`harness.mjs` computes `drawn`/`asked` from `app.js`)
move when the new key lands, and the tool-surface bounds do not (no new tool).

## Acceptance criteria

- [ ] **C0 — the paste affordance is drawn, measured, and owner-read.** `docs/plans/mockups/2026-11-08-composer-clipboard-paste/` holds the composer's foot with the paste button as static HTML in the app's own tokens, measured rendered in every theme (no overflow, no clipped label, ink ≥ 4.5:1), the PNGs read in a light and a dark theme, and the owner reads it before any code of C1+ lands.
- [ ] **C1 — the clipboard endpoint returns text, and an image's bytes.** `GET /api/clipboard` reads the system clipboard and returns `{text}` or `{image: {name, mime, bytes}}` (base64), and a unit test drives the reader through an injected backend with both a text and an image fixture and pins both replies.
- [ ] **C2 — the paste button inserts the clipboard's text.** With text on the clipboard, pressing the composer's paste button puts the words in the composer, and a harness case pins the textarea's value.
- [ ] **C3 — the paste button attaches the clipboard's image.** With an image on the clipboard, the paste button adds an image chip to the composer's attachments, and a harness case pins the chip.
- [x] **C4 — the caps are raised, and the send payload carries the image bytes.** The request cap is 25 MiB, an attachment may be 8 MiB, eight may ride a turn and 20 MiB together, with KI-378's refusal wording kept verbatim; the page's own limits mirror them; and the `/api/run` request's attachments carry the image's name, mime and base64 bytes. **The bounds are landed and pinned (KI-475):** `an_oversized_body_is_refused_in_the_products_words` asserts the wording at the new bound and `a_screenshot_sized_body_is_read_rather_than_refused` proves a three-screenshot body (9 MiB) passes. **Remainder:** the payload's image shape, which lands with C3.
- [ ] **C5 — the run's request carries the image.** An image-bearing run builds its first message with `Message::text_with_images`, and a unit test pins the message's image part (media type and bytes) against a fixture.
- [ ] **C6 — an image-bearing run rides the vision workload.** The route decision for a run carrying an image is `vision` whatever the task's words judge, and a unit test pins that an image forces the workload over a text-only judgment.
- [ ] **C7 — the bot face carries the same paste.** The bot face's composer has the paste affordance and inserts text and attaches an image through the same endpoint, and its harness case pins both.
- [ ] **C8 — the gate is green.** The four-leg gate runs clean: `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings`, `cargo build --release`, and `cargo test` each exit 0 on the merged tree.
- [ ] **C9 — the docs land.** `docs/features/gui.md` documents the paste affordance, the clipboard read, the per-route bound, and the vision routing, and `TECHNICAL-DOCUMENTATION.md`'s module reference names `clipboard.rs`.

## Files to be touched

**Backend:**
- `src/clipboard.rs` — new: the clipboard reader behind a backend trait (arboard in production, a fixture in tests).
- `src/gui/server.rs` — the `/api/clipboard` route; `/api/run`'s own body bound.
- `src/gui/bridge.rs` — the read handler; the run's attachment path.
- `src/agent.rs` — the run's first message: `text_with_images` when an image is attached; the route decision's image-forced `vision`.
- `Cargo.toml` — `arboard`.

**Face:**
- `src/gui/assets/app.js` — the paste button, the clipboard call, the image attachment kind, the chip, the send payload.
- `src/gui/assets/index.html` — the paste button in the composer's foot.
- `src/gui/assets/app.css` — the image chip's styling.
- `src/gui/assets/bot.js`, `bot.html`, `bot.css` — the same affordance.

**Tests:**
- `src/clipboard.rs` (inline `tests`) — the reader's round-trip.
- `src/agent.rs` (inline `tests`) — the vision message and the forced workload.
- `src/gui/server.rs` (inline `tests`) — the per-route bound.
- `tests/face/harness.mjs` — the paste cases; the icon source-pin counts.

**Documentation:**
- `docs/features/gui.md`.
- `TECHNICAL-DOCUMENTATION.md` — the module reference.

## Out of scope

- The vision model's response handling beyond the request (the multimodal reply is the existing text path).
- The TUI's paste (already handled, KI-026).
- Pasting rich content (HTML, formatted text) — the paste reads text and images only.
- The remote face's camera/photo capture (a different surface).
- **Raising the caps further than KI-475 did.** The owner's ruling is honored at 25 MiB / 8 MiB / 20 MiB / eight files; a larger number is a new ruling, not a slip of this plan's.
- **Compressing or downscaling a pasted image before sending.** The paste sends the bytes the clipboard holds; shrinking a retina screenshot is a real design question (quality against cost) and belongs to its own plan, not to a silent transform here.

## Verification

**C4 (landed):** `cargo test --lib gui::server` — both bound pins, `an_oversized_body_is_refused_in_the_products_words` and `a_screenshot_sized_body_is_read_rather_than_refused`.
**C1:** `cargo test --lib clipboard` — the reader's fixture round-trip.
**C2–C4, C7:** `node tests/face/harness.mjs` — the paste cases over the stubbed clipboard and attachments.
**C5–C6:** `cargo test --lib vision` and `cargo test --lib route` — the message and the workload tests.
**C8:** `cargo fmt --all --check`; `cargo clippy --all-targets -- -D warnings`; `cargo build --release`; `cargo test`. Read the exit codes, never through a status-losing pipe (KI-014).
**C9:** read the docs; the feature doc names the affordance, the per-route bound, and the routing.

## Linked artifacts

- `docs/features/gui.md` — the paste affordance, the clipboard read, the per-route bound, and the vision routing.
- `TECHNICAL-DOCUMENTATION.md` section "Module reference" — `clipboard.rs`.
- `docs/known-issues.md` KI-378 — the closed row that owns the 64 KiB general bound.

## Risks

- **The clipboard read is a system dependency.** `arboard` reads the macOS pasteboard; on a headless or locked session it can fail, and the endpoint must name the failure rather than return an empty success. The reader is behind a trait so the tests do not touch the system pasteboard.
- **The vision route's model availability.** The `vision` workload's candidates are the home's multimodal models; a home with none fails by name (the router's existing rule), so an image paste with no vision model is a named refusal, not a silent text-only send.
- **A 25 MiB bound is a real bound.** A payload past it fails with KI-378's named refusal; a turn of eight 8 MiB screenshots exceeds the provider's per-request image limit long before it reaches the model, and that limit is the provider's to state, so the failure is surfaced rather than pre-empted by a smaller invented number.
- **The request's cost grows with the image.** A screenshot's base64 is ~33% larger than its bytes, and the run's per-token cost follows the model the router selected; a large paste is a real spend, and the owner sees the size on the chip before sending.
- **The bot face's drift.** Two faces carrying the affordance double the surface for drift; the same endpoint and the same harness pattern keep them together.
- **The paste event's image branch stays unreachable in the webview** — the handler remains as the browser-face path (a real browser *does* deliver image paste events), and the new button is the app's path.

## Dependencies

- `src/api.rs` — `Message::text_with_images` and `ImageRef` (present; used today only by `Work::Vision`).
- `src/route.rs` — the `vision` workload's candidate chain (`src/route.rs:660`).
- `src/gui/assets/app.js:10275` — the composer's existing paste handler (the browser-face path this plan supplements).
- `arboard` — the clipboard backend; not currently in `Cargo.toml`.
