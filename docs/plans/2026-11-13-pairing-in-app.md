---
title: pairing works in the app — the remote face comes up on demand
date: 2026-11-13
status: completed (2026-11-13; the press brings the remote face up and mints — pinned by a_pair_press_brings_the_remote_face_up_and_mints; relay origins persist; the gate green)
---

# Pairing works in the app — the remote face comes up on demand

The owner's dogfood report (2026-11-13): launching the app and pressing
pair produces a card saying to run `fleet gui --remote` on the terminal.
The ruling: a user who wants to pair should never need a terminal command.

The pairing flow is otherwise complete in the app: the loopback face mints
(`POST /api/pair/window`), the card paints QR + code + URL, and the
joining side (`/pair` → `POST /api/pair`, device tokens, default-deny
allowlist) needs nothing. The one break is the precondition: the mint
refuses unless the remote listener is running, which only `--remote`
starts — and the desktop app launches with remote disabled
(`src-tauri/src/lib.rs:345`). The terminal command is the workaround for
a missing bring-up.

**The design: pressing pair is the intent, and the face itself brings the
remote face up** — the same listeners, the same token gate, the same
default-deny allowlist, the same loopback-only mint. The security posture
does not change; the owner's press replaces the flag.

## Acceptance criteria

- [x] C1: the launch-time remote block (address resolution, console + bot
  listener binds, attaches, relay dials) is a reusable async seam; the
  `--remote` launch calls it — behavior unchanged.
- [x] C2: a pair press with no remote face runs the bring-up once (guarded
  against concurrent presses), reading relay origins from the new settings
  keys; the relay dial is best-effort (configured origin + resolvable join
  proof → dial; anything missing → a named skip and the card mints with
  the origin it has); bring-up failures surface on the pair card — no
  terminal instruction anywhere.
- [x] C3: `remote.relay` and `remote.bot_relay` settings exist
  (config.json); a `--relay`/`--bot-relay` launch writes them through.
- [x] C4: the phone-side pairing page names the app's pairing card as the
  code's source (`fleet pair` as an alternative, not the instruction); the
  bridge's fallback error names the real cause.
- [x] C5: tests — a pair press with no remote face brings it up and mints;
  a bring-up failure names its cause; the write-through lands the keys;
  the old "names the flag" pin reworked.
- [x] C6: KI-538 filed; the gate green; commits on master, push; local
  deploy (rm-then-cp → face relaunch); no release steps.

## Verification

The gate and harness, then a live check: a face launched without
`--remote`, press the topbar pair icon — the QR card mints in-app; with
`remote.relay` configured and the join secret in `<FLEET_HOME>/.env`, the
card carries the relay origin.

## Out of scope (named, not built)

In-app device list/revocation (stays `fleet devices`); auto-starting the
remote face at app launch (sockets bind on the owner's intent, not boot);
the join secrets moving anywhere new (they stay in the own-home chain).
