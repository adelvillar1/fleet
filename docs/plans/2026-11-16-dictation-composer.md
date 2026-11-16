---
title: dictation in the composer — the microphone where the ask is made
date: 2026-11-16
status: completed
---

# Dictation in the composer (S2)

S1 delivered `fleet dictate` (docs/plans/2026-11-16-voice-dictation.md).
S2 puts the same seam behind the composer: one press starts a recording on
the **server**, the next press stops it and transcribes through hex, and
the words land where the caret was.

The split stays hex's: the fleet process owns the microphone (cpal, one
code path on every platform); hex owns the models and the inference. The
page never opens a microphone, so a paired phone cannot reach this
machine's mic — the routes are loopback only (the remote allowlist does
not gain them).

## What was built

- **`POST /api/dictate/start`** → `{type, recording, device}` (409 with a
  named refusal: a second start while one is recording, no input device,
  a format fleet cannot read).
- **`POST /api/dictate/stop`** → `{type, transcript, duration_ms, seconds,
  device, samples}`; hex's own refusal body is passed through (a missing
  model names `hex model install`), and a dead service is respawned
  rather than reused.
- **The bridge state** (`Arc<Mutex<DictateState>>`): one recorder at a
  time and the warm hex service kept across clips (0.11–0.12 s per clip
  warm; ~1.5 s when the service must prewarm). Both calls run in
  `spawn_blocking` so the audio device and the loopback HTTP call never
  hold a runtime worker.
- **The composer button** (`#composer-dictate`, before Add bot): idle
  "Dictate"; listening shows a pulsing red dot and "Stop"; transcribing
  reads "Transcribing…" and dims. The transcript is inserted at the
  caret with a space where one is needed, then the box focuses and
  grows. Failures are notices, never silent.
- **The mic grant**: `src-tauri/Info.plist` carries
  `NSMicrophoneUsageDescription`, wired through
  `bundle.macOS.infoPlist`, so the native shell prompts under its own
  identity.

## Acceptance criteria

- [x] C1: the composer's button starts a recording on the server and its
  second press stops and inserts the transcript at the caret; the
  harness pins the two POSTs, the listening word, and the insertion
  (1,297 checks / 186 cases).
- [x] C2: one recording at a time — a second start is refused by name,
  and a stop with no recording is refused by name (bridge test).
- [x] C3: a missing hex/model is hex's own words in a notice; the routes
  are loopback only.
- [x] C4: the native bundle carries the mic usage string, and the live
  app records and transcribes through the button.

## Verified live (2026-11-16)

- `POST /api/dictate/start` on the running app answered
  `{"device":"MacBook Air Microphone","recording":true}`; the stop four
  seconds later answered `{"duration_ms":4010,"samples":192512,
  "transcript":"…"}` — the capture, the service and the seam are real.
  The spoken test line went to the machine's output, not the room, so the
  clip carried other audio; the exact-content check is the owner speaking.
- The built bundle carries `NSMicrophoneUsageDescription` (read back with
  plutil), so the shell prompts under its own identity.
- Gate green: fmt, clippy, build --release, 2,552 tests; harness 1,297
  checks / 186 cases.
