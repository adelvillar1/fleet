---
name: verify-face-changes-in-the-browser
description: "feedback 2026-10-21/22 — a green Rust gate cannot see the browser face: cargo test builds debug while the face embeds app.js at release-build time, a duplicate JS function name silently wins by hoisting, and a probe reading a global the app never exposed produced confident false readings. Verify face changes in the live browser, measuring the DOM rather than assumed state"
type: feedback
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

Three ways a face change was "done" while the page said otherwise, all
caught only by driving the real browser (Playwright against the running
face — see [[gui-client-verified-by-a-vm-harness]]; on this machine
`~/Projects/helmsman/node_modules/playwright` drives the installed
Chrome 155 via `chromium.launch({ channel: 'chrome' })`, and the
ms-playwright cache is empty so the headless shell is not an option).

1. **`cargo test` green ≠ the face updated.** The face embeds
   `app.js`/`app.css`/`index.html` with `include_str!` at *release*
   build time. After an app.js edit, `cargo fmt` + `cargo test --lib`
   passed and the running face still served the old script — the fix
   "didn't take" until `cargo build --release` and a LaunchAgent
   kickstart. A green test run says nothing about what the browser is
   running; [[verify-the-binary-is-newer-than-the-source]] is the same
   lesson one level down.

2. **A duplicate function name wins silently.** The new mm:ss formatter
   was declared `function clock(ms)`; the file already had a later
   `function clock(stamp)` that slices an ISO timestamp. Function
   declarations hoist, so every call ran the *second* one and the hint
   printed `1202` instead of `20:02` — compiles clean, gate green, page
   wrong. In a single-file app with hundreds of functions, grep the name
   before declaring it.

3. **A probe that measures the wrong thing confirms the wrong theory.**
   The Playwright probe read `window.face` — which the app never
   exposes (it lives in a closure) — so `faceSeat` read as null on every
   sample and nearly confirmed "the handler cleared the seat but the
   paint was skipped." The real signal was the DOM: the `hidden`
   *attribute* and `getComputedStyle(...).display`, not a property the
   probe assumed existed. Measure the element, or first prove the global
   is real. The same trap fired four more times in one session
   (2026-10-22): counting `#log`'s children when the rows live in the
   `.stream` inside it (reported "one card" for a nine-row history);
   clicking `.row-action` when the selector matched the *first* action
   in the row rather than the labeled one (clicked "open", concluded the
   wrong thing about "continue"); reading a `hidden` *property* the app
   had set correctly while CSS hid the element anyway; and reading
   `scrollTop: 0` as "not scrolled" when `scrollHeight == clientHeight`
   — no overflow is the correct reading of a short log, not a failure to
   scroll. When a probe's answer and your theory disagree, audit the
   probe's target and selector before re-theorizing about the app.

**Why:** each of these produces a confident, wrong story — the gate is
green, the code reads right, the probe agrees — and the only witness is
the rendered page.

**How to apply:** for any change under `src/gui/assets/`, run the gate,
then `cargo build --release`, kickstart the face, and drive the actual
flow in the browser: click the control, then read the element's
attribute and computed style. A CSS animation is verifiable —
`getComputedStyle(el, '::before').animationName` names the keyframes.
Related: [[a-fallback-path-hides-the-defect]] (a designed degradation
looks identical to the defect that triggers it),
[[verify-what-renders-not-what-commits]].
