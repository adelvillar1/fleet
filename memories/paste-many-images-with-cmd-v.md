---
name: paste-many-images-with-cmd-v
description: "⌘V now attaches every image the paste carries, and the paste button attaches a list too — the clipboard reader goes images-first (a Finder copy no longer pastes its filenames) and sniffs types from bytes rather than extensions."
type: change
date: 2026-11-18
basis: confirmed-by
tier: durable
shape: [rendering, drops-the-row, resolved]
phrases: [paste one or more images from the clipboard, with cmd-V on the keyboard, pasteClipboardImages, Clip::Images, arboard file_list, an image cannot ride the composer yet]
missing: []
premises: []
recall-count: 5
superseded-by:
valid-until:
source:
---

The owner's 2026-11-18 request: "I need to be able to paste one or more images from the clipboard into the composer" — "with cmd-V on the keyboard". Plan `2026-11-18-paste-many-images.md`, status completed, all 8 criteria ticked.

WHAT WAS ALREADY THERE: `gui::server::images_of` already walked a LIST of attachments (base64-decodes, checks declared mime against actual bytes, caps each); `ATTACH_LIMIT` is 8; the paste button already attached one image. The send path needed no change at all.

THE ACTUAL DEFECT was the paste handler (`app.js:11896`): its image arm called `preventDefault()` and showed "an image cannot ride the composer yet", and even the button stopped at one because `clipboard::read` returned a single clip with TEXT CHECKED BEFORE IMAGE — so a Finder copy (name AND pixels) pasted the name.

WHAT LANDED:
- `app.js`: the paste handler loops EVERY `image/*` item (the `[0]` is gone) and hands each to a new `pasteClipboardImages`, which FileReader-reads each as a data URL into the SAME `pasteImage` the button uses. The button's reply loop gained a `kind:"images"` arm.
- `src/clipboard.rs`: `Clip::Images(Vec<Image>)` + an `Image` struct; `Backend::files()` with an EMPTY DEFAULT (so old fixtures keep working); the `System` backend reads `arboard`'s `file_list()`; `read()` flips precedence to images-first; `read_files` sniffs the first bytes (PNG/JPEG/GIF/WebP) rather than trusting extensions; `encoded()` is one shape for both the single and list replies.
- `src/gui/bridge.rs`: `clipboard()` answers `{kind:"images", images:[...]}`.

C8 MEASURED, and the old premise was wrong: the Tauri shell loads the face as `WebviewUrl::External(loopback url)` (`src-tauri/src/lib.rs`), so the page runs in a real webview (WKWebView / webkit2gtk / WebView2 per `tauri-2.10.2/Cargo.toml`). A live paste event with two image items against the SERVED page produced `{"chips":2}`. So ⌘V is load-bearing and the button is the fallback. The recorded reason for the button's existence ("a webview will not hand an image to the page") was true of Tauri's drag-drop path, not the paste event.

TWO PROBE TRAPS worth keeping, because both initially looked like product bugs:
1. `Runtime.evaluate` on an arrow FUNCTION echoes the function; it awaits a returned PROMISE. An IIFE is needed to get the result.
2. `FileReader.readAsDataURL` is async, so a synchronous probe reads `undefined` and reports zero chips. The first live probe said "chips:0" for exactly this reason and the product was fine.

TESTS: `text_wins_over_an_image_...` pinned the OLD precedence and was rewritten to `an_image_wins_over_text_because_text_is_the_image_s_name`; added `a_file_list_becomes_every_image_it_names` (two PNGs + a .txt → two images); the harness gained `FileReader.readAsDataURL` and the `PASTE` case. Both new checks proved to bite: restoring the refusal fails 2 checks, and `slice(0,1)` (one image) fails with `chips=1`.

Gate: 2377 Rust tests serially (the two lane mocks still flake under parallel load), 1461 face checks across 205 cases, fmt and clippy clean.
