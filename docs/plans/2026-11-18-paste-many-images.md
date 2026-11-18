---
status: completed
created: 2026-11-18
updated: 2026-11-18
slug: paste-many-images
---

# Paste one or more images with ⌘V

The owner's 2026-11-18 request, verbatim: "I need to be able to paste one or
more images from the clipboard into the composer" — and, when asked how: "with
cmd-V on the keyboard".

## Context

Almost all of this already exists. Measured, not assumed:

**The server takes a list of images today.**
`gui::server::images_of` (`server.rs:2686`) walks `attachments`, base64-decodes
each `bytes` field, verifies the DECLARED mime against the actual bytes, caps
each at `ATTACHMENT_BYTE_CAP`, and answers a `Vec<ImageRef>`. A test
(`an_image_attachment_is_framed_as_a_picture_and_decodes_to_its_bytes`) already
sends one and asserts the framing. Nothing in the Rust path is single-image.

**The composer's chip pipeline takes many.** `ATTACH_LIMIT` is 8 and
`face.attachments` is an array.

**The composer's paste button already attaches one image**
(`pasteImage`, `app.js:11805`), through the server's `GET /api/clipboard`.

**And ⌘V is explicitly refused.** The paste handler (`app.js:11896`) is the
whole defect. It has three arms:

1. `clipboard.files` non-empty → `attachFiles(files)`;
2. else a `clipboard.items` entry whose `type` is `image/*` →
   `event.preventDefault()` and a notice: *"an image cannot ride the composer
   yet"*;
3. else the browser's own text paste.

So the keyboard path tells the owner an image cannot ride the composer, while
the button two centimetres away attaches one. And even the button's path stops
at one: `clipboard::read` returns `Clip::Text | Clip::Image | Clip::Empty`
(`src/clipboard.rs:140`) — a single clip, with **text checked before image**, so
a Finder copy (name AND pixels) yields the name and drops the picture.

**A measurement worth stating, because it decides the design:** the reason
recorded for routing through the server was "a webview will not hand an image
to the page". That premise needs testing before it is built on again — the
handler above clearly RECEIVES image items (arm 2 exists precisely because
they arrive), so a Chromium-backed webview does hand them over. Whether the
Tauri shell behaves the same is the one thing to measure.

## Approach

The keyboard path becomes the real one, and the button stays as the fallback
for a webview that will not cooperate.

1. **⌘V reads `clipboardData.items`, not just `.files`.** Every `kind: "file"`
   item with an `image/*` type is read via `item.getAsFile()` +
   `FileReader.readAsDataURL`, and pushed through the EXISTING `pasteImage`
   path so the bounds and the chip are the ones already proven. No new
   pipeline.
2. **All of them, not the first.** The `[0]` index in arm 2 goes; a loop takes
   every image item. `ATTACH_LIMIT` and the byte bound already guard the row.
3. **Text still pastes when there is no image.** The image arm runs only when
   an image item is present; otherwise the browser's default paste is
   untouched.
4. **The button's single-image read gains a list.** `clipboard::read` returns
   the images it can find — the macOS file list first (`arboard::get()
   .file_list()`, `lib.rs:210`), then the aggregate pasteboard image — and
   text LAST, so a copied file's name never shadows its pixels. The endpoint
   answers `{kind:"images", images:[...]}`; the handler loops it into the same
   `pasteImage`.

**Measured 2026-11-18 (C8), so this is no longer a choice:** the paste event
does deliver image items in the shell, so (1)-(3) are the feature and (4) is
the fallback. The reasoning is kept below because it is why the fallback
exists at all.

**The measure-before-build step:** confirm in the Tauri shell that a real ⌘V
delivers `clipboardData.items` with `image/*` entries. If it does, (1)-(3) are
the feature and the button is a convenience. If it does not, the button's list
read (4) is the feature and (1)-(3) are a browser-only accelerator. Either way
the composer ends up accepting many — which is the requirement — but which arm
is load-bearing is a fact, not a preference.

## Acceptance criteria

- [x] C1: ⌘V attaches every image it carries. Verified twice — by the PASTE
  harness case (two image items → two chips) and LIVE against the served page
  through obscura's CDP: dispatching a paste event with two image items on
  `#task` produced `{"chips":2}` with both names on the chips.
- [x] C2: text with no image leaves the composer to the browser. Pinned by the
  PASTE case's second arm (a `string`/`text/plain` item attaches nothing).
- [x] C3: a clip holding both answers the images. Pinned by
  `an_image_wins_over_text_because_text_is_the_image_s_name`, which replaced
  the test that pinned the old precedence — the ruling flipped it.
- [x] C4: the button attaches a list. `Clip::Images` + `read_files` (the macOS
  `arboard::get().file_list()` route) + `POST`-free `GET /api/clipboard`
  answering `{kind:"images", images:[…]}`; the handler loops the same
  `pasteImage`. Pinned by the PASTE case's third arm.
- [x] C5: the bounds hold per image and are named per image — `pasteImage`
  checks `ATTACH_LIMIT` and the byte bound, and `pasteClipboardImages` names an
  oversized file while its siblings still attach (each read is independent).
- [x] C6: a non-image is skipped by TYPE, not guessed: `read_files` sniffs the
  first bytes and passes over what is not PNG/JPEG/GIF/WebP. Pinned by
  `a_file_list_becomes_every_image_it_names`, whose fixture copies two PNGs and
  a `.txt` and asserts exactly two arrive.
- [x] C7: `images_of` already walked every attachment and re-checked each
  declared mime against its bytes, so the send needed no change; the images
  reach the model through `text_with_images`. Vision was verified live
  separately on 2026-11-18.
- [x] C8: MEASURED, and the old premise was wrong. The Tauri shell loads the
  face as `tauri::WebviewUrl::External(<loopback url>)` (`src-tauri/src/lib.rs`),
  so the page runs in a real webview — WKWebView on macOS, webkit2gtk on Linux,
  WebView2 on Windows (`tauri-2.10.2/Cargo.toml`). A live paste event with two
  image items reached the served handler and produced **two chips**, so ⌘V is
  the load-bearing path and the button is the fallback. The recorded reason for
  the button ("a webview will not hand an image to the page") was true of Tauri's
  drag-drop path, not of the `paste` event.

## Files to be touched

- `src/gui/assets/app.js` — the paste handler's image arm (loop, not `[0]`),
  and the button's reply loop.
- `src/clipboard.rs` — `Clip::Files`/`Clip::Images`, the precedence flip
  (images before text), the per-file read.
- `src/gui/bridge.rs` — `clipboard()` answers a list.
- `tests/face/harness.mjs` — C1-C6 as cases with a synthetic `clipboardData`.
- `docs/plans/2026-11-08-composer-clipboard-paste.md` — a pointer only; it stays
  `completed` for what it shipped.

## Decisions the owner holds

None outstanding for the shape: "one or more" and "⌘V" settle both. Two notes
rather than decisions:

- **In-memory multi-image copies** (several images copied inside an app, never
  on disk) would need a pasteboard walk `arboard` does not expose. The keyboard
  path handles them natively (`clipboardData.items` carries them as files), so
  the BUTTON would remain single for that case. That is an acceptable asymmetry
  and this plan says so rather than hiding it.
- **The button stays.** It is the fallback for a webview that refuses the
  keyboard path — the reason it was built — and removing it would leave no
  door if C8 measures badly.

## Out of scope

- Re-litigating the 2026-11-08 plan.
- Drag-and-drop of images onto the composer.
- How the model receives them (works; vision verified live 2026-11-18).

## Verification

`cargo test -p fleet --lib clipboard`, the face harness for C1-C6, the C8
measurement in the real shell, and a live ⌘V of several PNGs with the chips and
the sent turn inspected.

## Linked artifacts

- `src/gui/assets/app.js:11896` — the paste handler that refuses images.
- `src/gui/assets/app.js:11805` — `pasteImage`, the pipeline to reuse.
- `src/gui/server.rs:2686` — `images_of`, which already takes a list.
- `src/clipboard.rs:140` — the single-clip reader.
- `arboard/lib.rs:210` — `file_list`, the multi-image route for the button.
