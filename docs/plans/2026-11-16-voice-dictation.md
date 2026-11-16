---
title: voice dictation — the microphone, transcribed by hex
date: 2026-11-16
status: completed
---

# Voice dictation

The owner's idea (2026-11-16): speak into fleet. `hex`
(https://github.com/anomalyco/hex) is the transcriber — Rust, MIT, fully
local, with binaries for macOS, Linux and Windows, which is what makes the
future Tauri ports platform-agnostic.

The architecture is the one hex itself prescribes (its
`docs/specs/local-transcription-service.md`): **the host owns the
microphone; hex owns the models and the inference.** The embedded helper
never opens the mic.

## Verified (the spike, 2026-11-16)

- hex 3.4.0 installed via `anomalyco/tap`; the CLI lives in the app bundle
  (`/Applications/Hex.app/Contents/MacOS/hex`) and is linked into
  `~/.local/bin`.
- The integration is **`hex service --embedded`**, not the CLI: spawn the
  binary as a child with piped stdin; it binds `127.0.0.1:0` and writes one
  JSON line — `{type: "ready", url, token, apiVersion: "2", pid}` — in
  **0.03 s**. Every request carries `Authorization: Bearer <token>`.
- `POST /transcriptions?model=<id>` takes a WAV body and answers
  `{transcript, durationMs}`. A 4.6-second clip transcribed in **1.44 s**
  on the first request (the model prewarms) and **0.11–0.12 s** warm.
- Closing the child's stdin is a clean shutdown (exit 0).
- The CLI path (`hex transcribe --json`) is **not** what fleet uses: in
  3.4.0 on macOS it answers `not-running` unless Hex.app is up, and it
  accepts WAV only (the docs' wider format list is ahead of the build).
- The default model is `parakeet_unified_en` (731 MB, shared per user);
  `POST /models/{id}/prepare` is the explicit download path and must never
  be entered silently.

## The contract

- fleet records a WAV clip (cpal: CoreAudio, WASAPI, ALSA/PipeWire — one
  code path) and transcribes it through the embedded service; the hex
  binary is resolved from `FLEET_HEX`, then `PATH`, then the app bundle,
  and a missing binary is refused by name.
- The service is spawned once per fleet process and kept warm; its stdin
  stays open as the lease and closes on exit.
- A missing model is **named**, never downloaded implicitly: the refusal
  says `hex model install <id>`.

## Slices

- **S1 — `fleet dictate`.** `fleet dictate --file <wav>` transcribes a
  file (the hermetic, testable path); with no file it records N seconds
  (default 5) from the default input and prints the transcript; `--json`
  prints the reply; `--model` overrides `parakeet_unified_en`. Named
  refusals: no hex, no service ready line, no input device, a missing
  model, a refused request.
- **S2 — the composer.** A microphone in the composer calls one core
  endpoint (`POST /api/dictate`) that records and transcribes through the
  same seam; the transcript lands in the composer, where the owner edits
  it.
- **S3 — history (optional).** The transcripts are recorded as a
  calibration row; a privileged plugin could read them as a query. Not in
  scope until S1/S2 are used.

## Acceptance criteria

- [x] C1: `fleet dictate --file` prints the transcript of a real WAV, and
  its refusals name what is missing (hex, service, model) — pinned by
  tests over the ready-line, WAV and reply parsers, plus one ignored live
  test against a real hex.
- [x] C2: `fleet dictate` with no file records the default input for the
  named seconds (or `--seconds`), reports the device and the duration,
  and prints the transcript; a machine with no input device is refused by
  name.
- [x] C3: the service is spawned once, kept warm across clips, and shut
  down by closing stdin; a dead child is reported instead of hanging.
- [x] C4: the model missing is refused with `hex model install <id>` in
  the words; nothing downloads implicitly.

## Verified live (2026-11-16)

- `fleet dictate --file /tmp/hex-test.wav` printed the clip's own words
  exactly (`Fleet, this is a dictation test. Please transcribe this
  sentence.`); `--json` answered `{type, model, duration_ms, file,
  transcript}`.
- `fleet dictate --seconds 5` captured 5.0 s from `MacBook Air Microphone`
  (237,568 samples) and transcribed it — the mic path is real; the room's
  audio that minute was not a controlled clip, so the file path is the
  exact-match proof.
- The embedded service needs no hex Labs flag (the CLI's
  `transcribe-files` gate is CLI-only): with the lab off, the file
  transcribed through the service.
- Warm latency measured at 0.11–0.12 s per clip (4.6 s clip), 1.44 s on
  the first request (model prewarm).
- The audio is never kept: the verb records into memory and sends the WAV;
  nothing is written to disk.

## Closed (S1 delivered)

S1 shipped: `fleet dictate` (file and microphone), the embedded-service
client, the WAV writer, cpal capture, and named refusals — 6 unit tests
plus the suite (2,551 green). S2 (a microphone in the composer through one
`/api/dictate` endpoint) and S3 (history, optionally a plugin page) are
follow-ups: they are a new plan's work, not this plan's open criteria.
