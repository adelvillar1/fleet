---
status: active
created: 2026-11-21
plan: mini-console
---

# The mini console — the TUI's economy, relayed to a phone

The owner's question: *"is there a way to do a TUI-like experience on a PWA
through the fleet relay? A full PWA app may not be the best experience on a
phone."* The measured answer: the full face costs 389 KB gz **plus the parse
and execution of 1.45 MB of JavaScript** — the wire is now small, but the
program is not. A terminal-shaped face pays almost none of it: text, cards,
one command line, the documents the TUI panels already read.

**The shape**: a third small face modelled on the bot PWA's skeleton
(`bot.html` is ~55 KB total and already relays under `/bot/` with the KI-587
clocks) — monospace, dense, character-grid at heart, and **navigation by
command line**: type where to go (`/sessions`, `/board`, `/delegation`,
`/kan`), type what to do; taps on the same rows for touch users; TUI shortcut
parity when a hardware keyboard is attached (the iPad case).

## The rules

- **Same documents, never rederived** — the panels render the reads the TUI
  panels consume (`boards::read_work_board`, the delegation document with the
  overlay + divergences, `boards::read_brain`) printed through the terminal's
  own wrapping law: `cards::fit`-grade wrapping, one paragraph per line.
- **The four sentences**, in every panel, terminal voice.
- **The relay and the clocks are inherited, not rebuilt**: a third shell route
  (pairing-gated), the SW pattern with `fetchWithin` deadlines + the watchdog
  armed at script evaluation — a face this size should never be able to freeze.
- **No build step** (house law); inline styles only; no hex outside tokens' UI
  — the mini console may carry its own small palette with the same audit.
- **Speed is the AC, stated numerically**: the whole shell (html+js+css) under
  **60 KB uncompressed / 20 KB gzip**; painted from a cleared cache inside
  **1 s at a 1.5 Mbps link**; every interaction responds inside one API
  round-trip.

## The panels (first pass)

`/sessions` (the work list + one conversation as pure transcript text),
`/board` (three columns as text stacks), `/delegation` (the overlay tile: `never spawned N · ran twice N`, the why-this-child line under the focused
node), `/kanzo` (if the kanzo lane has landed: the board's own receipt feed),
`/second-brain` (the morning read + section rows). Anything past these is a
command that answers "not on this surface yet — the console says so by name."

## Acceptance criteria

- [ ] The shell numbers above, measured on the real phone link profile the
      compression plan measured (1.5 Mbps / 150 ms RTT), from a cleared cache,
      through the relay.
      *Phase 1 measures the budget half only, and pins it: **52,452 B
      uncompressed / 16,868 B gzip** over the three files (console.html
      1,369 / console.js 42,675 / console.css 8,408) against 61,440 /
      20,480 B. The cold load through the relay is phase 2, with the route.*
- [x] The three TUI documents render with one text path each; a divergence
      word and a refusal appear by name in the panels' tests.
      *Measured 2026-11-21: `/board`, `/delegation` and `/second-brain` each
      print from one renderer over the bridge's own document —
      `/api/board` (`boards::read_work_board`), `/api/delegation` (the
      overlay from `delegation_overlay` + `declared_workload`) and
      `/api/second-brain` (`read_brain` + `attention_queue`), the same reads
      `src/tui/panels.rs` consumes. `never spawned` and `ran twice` are
      pinned on the node that carries them, every panel's refusal is pinned
      by name, and each panel's empty state is asserted as a sentence rather
      than a zero: 37 cases in `tests/console/harness.mjs`.*
- [ ] The KI-587 clocks present in the new worker (deadline + evaluate-time
      watchdog + one-generation sweep) — red-proofed by the freeze coming
      back when a clock is removed.
      *Phase 2: the worker is not in this phase's file set, so the shell
      registers none — a page that registered a worker the server does not
      serve would fail in the console and say nothing.*
- [ ] Relay pairing gates the route; an unpaired client is refused by name
      before the face is reached.
      *Phase 2 (the listener route). The face already speaks the inherited
      relay: the fragment secret, the handshake, the sealed frame, and an
      inert path on loopback.*
- [x] The bot PWA stays untouched in behaviour (its own tests still pass);
      the mini console is a sibling file set, not a bot.js fork.
      *Measured 2026-11-21: no `bot.*`, `app.*` or `sw.js` file is touched
      by this work — the console carries its own palette block, its own
      wrapping law and its own renderers, and imports nothing. The face
      harness passes unchanged: `node tests/face/harness.mjs` → 1,823
      checks across 268 cases; `cargo test --test face` exits 0.*

## Files

- `src/gui/assets/console.html` + `console.js` + `console.css` — the new face
  (sibling set, bot-PWA-shaped). **Landed in phase 1.**
- `src/remote/listener.rs` — the route + the pairing gate (measure how /bot/
  was added; mirror it). *Phase 2.*
- `src/gui/server.rs` — the shell serving + the SW variant with the clocks.
  *Phase 2 — with `build.rs`'s asset list, which is where the three files
  enter the build stamp and get their gzip twins.*
- tests: the listener/pairing tests per the bot scope's pattern; the face's
  four-sentence pins in a console-specific suite (the face harness stays
  untouched). **The console suite landed in phase 1**
  (`tests/console.rs` + `tests/console/harness.mjs`); the listener's own
  tests are phase 2.
