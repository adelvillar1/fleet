---
status: completed
created: 2026-11-06
updated: 2026-11-10
slug: seat-decision-label-duplication
---

# Plan: the seat's decision buttons stop duplicating their labels

## Context

The approval seat's decision buttons render the label twice. Both
`paintSeat` (`src/gui/assets/app.js:3441`) and `paintDesignSeat`
(`src/gui/assets/app.js:3393`) build each button with
`node('button', 'decision', pair[2])` — and `node` sets `textContent`
(`app.js:1329-1334`) — then append the key span and
`document.createTextNode(' ' + pair[2])`, the label again. The button
reads `<label><key> <label>`. Commit `2ffcd79` (gui-revamp C12) changed
the button's text from the key to the label and added the key span plus
the label text, leaving the `node` text in place. The live face serves
the buggy code — two occurrences in the served `app.js`.

## Approach

Drop the redundant text: build the button as `node('button', 'decision')`
— no text — in both renderers, so the content is the key span then the
label, which is what `.decision-key`'s `margin-right` in `app.css`
intends. Pin the rendered text in the harness so the duplication cannot
return, and file the KI row.

High-level steps:

1. `paintSeat`: `node('button', 'decision')`.
2. `paintDesignSeat`: the same.
3. The harness's C12 case pins each button's exact text.
4. The KI row.
5. The gate: the harness's C12 case and the release build run green.

## Acceptance criteria

- [x] C0 — the seat's decision button renders the key and the label once
      each, the key first.
- [x] C1 — the design seat's decision button does the same.
- [x] C2 — the harness's C12 case asserts each button's exact text.
- [x] C3 — the gate and the harness run green. (Verified 2026-11-10: both render sites build the button with no text — app.js:3530 and :3578 — and `cargo fmt --all --check` + `cargo build --release` exit 0; the crate's suite ran green the same day at 2,356 passed.)

## Files to be touched

**Frontend:**
- `src/gui/assets/app.js` — the two seat renderers.

**Tests:**
- `tests/face/harness.mjs` — the C12 pin.

**Documentation:**
- `docs/known-issues.md` — the row.

## Out of scope

- The other seat renderers (the replay buttons build their text once).
- The decision labels' wording.

## Verification

**C0/C1 — the rendered text.**
```bash
node tests/face/harness.mjs
```
Expected: the C12 case's button text is the key then the label, once.

**C2 — the pin.**
Expected: the harness asserts the exact text, so a regression reds it.

**C3 — the gate.**
```bash
cargo fmt --all --check && cargo build --release
```
Expected: exit 0 (a JS-only change has no Rust surface).

## Linked artifacts

- `docs/known-issues.md` — the row: the seat's decision buttons
  duplicate their labels.

## Notes

- `node` sets `textContent`; the appended nodes then add the key and the
  label again.
- The commit `2ffcd79` introduced the line.
