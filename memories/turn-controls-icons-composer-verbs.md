---
name: turn-controls-icons-composer-verbs
description: "project 2026-10-29 — the five turn controls (copy · like · dislike · fork · hook) are icons now, the pressed verdict reads as aria-pressed + accent + past-tense tooltip, the composer gained a steering-wheel icon prefilled with /steer and a bifurcation prefilled with /btw (a563b7e), then the face's own hover tooltip on the composer icons and the queue's send/edit/drop icon verbs (1cda5a3), then the queue-text overflow chain fixed and MEASURED in Chromium (f8ce79b); harness 330 checks / 36 cases green"
type: project
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

2026-10-29, following the owner's ask ("turn those actions that are shown as
chips with words into icons… a steering wheel icon to trigger a steer command
and a bifurcation icon to trigger a btw command"):

- **Turn controls are icons** ([app.js]~949 `MSG_ICONS` + `iconButton`): one
  shared path table draws copy/like/dislike/fork/hook for the reply footer AND
  the owner bubble's copy, so the two cannot drift. The words survive as
  tooltip + aria-label. The verdict trace MOVED OFF the label flip: pressed =
  `aria-pressed="true"` + the accent tint (`.msg-fb[data-state]`, already
  existed) + the tooltip's past tense ("liked — this reply was right");
  copy's trace is the glyph swapping to a check for 1.2 s. Square 22 px via
  `.msg-tools .row-rewind.msg-icon`.
- **Composer verbs**: two `.foot-btn.icon` buttons in `foot-actions` —
  steering wheel (`#composer-steer`) prefills `/steer `, bifurcation
  (`#composer-btw`) prefills `/btw `, caret parked after the prefix. The
  composer's Enter path already parses slash lines natively ("a slash line
  runs its command rather than reaching the agent") and matches aliases, so
  the slash parser stays the ONLY parser — the icons just write the command's
  word ([[a-second-face-must-share-the-firsts-parsers]] in spirit).
- Verification: `tests/face/harness.mjs` now at **297 checks / 31 cases**
  green (the TURN case asserts the icon contract, a new VERBS case asserts
  both prefills). `docs/features/gui.md` updated.
- **LIVE, then COMMITTED.** After the owner's "did you restart the app?",
  the binary was rebuilt (fmt/clippy/build all exit 0; the full test suite
  was CANCELLED by the owner as over-thorough —
  [[verification-depth-matches-the-change]]) and the gui was restarted:
  kill of the plain nohup process (PID 79857) + a plain `nohup … gui
  --no-open --remote &` relaunch (PID 96252) bound 3080 fine — see
  [[fleet-gui-is-a-launchagent]] for the check-first caveat. Liveness was
  verified on the SERVED assets (`curl 127.0.0.1:3080/app.js` + `/` carry
  the new markers). The restart had baked the then-uncommitted
  resume-record work into the live build; that slice closed out and
  [[resume-record-closeout-2026-10-29]] landed as `364c09c`, and on the
  owner's "update the docs then commit" this icon slice landed as
  **`a563b7e`** (carrying the resume slice's face half — the state-word
  map and replay drawer line), with docs as `9a11b7d` and memories churn
  as `0a41b3a`.
- The navigation plan [[conversation-sessions-home-plan-2026-10-29]] still
  awaits the owner's word; the ~59 harness-probe sessions polluting the
  archive remain the named out-of-scope follow-up.

**Follow-up slice, same day (committed `1cda5a3`, harness now 329 checks /
36 cases):**

- **The face's own hover tooltip replaced native titles** on the composer's
  icon buttons (steer, btw, doc-open, attach). The owner reported the icons
  "should have a hover over tool tip" — the native `title` needed a ~1 s
  hover and rendered browser-default. The pattern: the words live in
  `data-tip`, a CSS `::after` (`content: attr(data-tip)`, anchored above,
  right-aligned to stay on-screen, themed surface/hairline/ink, z-index 30)
  shows it on `:hover` and `:focus-visible`; `aria-label` keeps the
  accessible name and `title` is DROPPED so the native tooltip can't double
  with the styled one.
- **The queue's three verbs are icons** (the reply footer's approach —
  words in tooltip + aria-label, glyph in the row, the 22px square shared
  with `.msg-tools`): **send** (arrow, `MSG_ICONS.send`) moves the message
  to the head — the head is what the next run sends, so it is disabled
  there (this replaces the old word-button "send first"); **edit**
  (pencil) returns the row's words to the composer and takes the row out
  (saveDraft + focus + paintQueue); **drop** (×) removes it. Drag-to-
  reorder survives untouched.
- **Real defect the queue case caught**: `paintQueue` cleared
  `queueList` only on the painted path — an emptied queue returned early
  and the removed row survived inside the hidden chip (invisible but
  stale DOM; the new case's `rows().length === 0` assertion reddened).
  Fixed in the same write: clear the list before the early return. An
  empty-queue check is the kind that catches "the chip hides" passing
  while the DOM lies.
- After both slices the gui was relaunched twice on the owner's word
  ([[relaunch-is-not-a-code-event]] — same verified binary, no test run
  owed), verified by `curl /app.js | grep -c` for the new markers.
