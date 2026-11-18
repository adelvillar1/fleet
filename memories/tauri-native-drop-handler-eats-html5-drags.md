---
name: tauri-native-drop-handler-eats-html5-drags
description: "Tauri v2 enables its own drag-drop handler by default (drag_drop_handler_enabled: true), and while it is on an in-page HTML5 drag never reaches the page's own listeners — the fix is .disable_drag_drop_handler() on the window builder."
type: project
date: 2026-11-18
basis: inferred
tier: durable
shape: [fail-open, resolved]
phrases: [disable_drag_drop_handler, drag_drop_handler_enabled, an in-page drag never reaches the page, the row follows the cursor and nothing lands, tauri-runtime-wry with_drag_drop_handler]
missing: []
premises: []
recall-count: 0
superseded-by:
valid-until:
source:
---

The queue felt broken in the desktop app while the face harness stayed fully green, and this is one of the two reasons.

Tauri v2's webview builder wires `with_drag_drop_handler` whenever `webview_attributes.drag_drop_handler_enabled` is true — and that field DEFAULTS to true (verified in the pinned crates: tauri-runtime-2.12.2/src/webview.rs:521 `drag_drop_handler_enabled: true`, consumed at tauri-runtime-wry-2.12.3/src/lib.rs:4783). The handler exists to forward OS file drops as `DragDropEvent`s, but it claims the drag before the page sees it, so a row's own `dragstart`/`drop` listeners never fire.

The off switch is `WebviewWindowBuilder::disable_drag_drop_handler()` (tauri 2.12.5, src/webview/mod.rs:1052). Tauri's own doc comment only mentions Windows/WebView2, but the handler is installed on every platform, so the off switch belongs there too.

Before turning it off, check the page actually consumes no OS file drops: `grep -n "addEventListener('drop'\|addEventListener('dragover'" src/gui/assets/app.js` — in fleet the only `drop`/`dragover` listeners in 27k lines belong to the queue row, so the handler bought nothing.

The face harness cannot see this: it constructs its own DOM and `fire`s events at listeners directly, bypassing both the webview and the event's default action. A check for it has to read the Rust source (RUST_TAURI in tests/face/harness.mjs).
