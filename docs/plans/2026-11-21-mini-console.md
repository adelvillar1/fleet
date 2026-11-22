---
status: completed
created: 2026-11-21
updated: 2026-11-22
plan: mini-console
resolution: |
  Both phases landed. The face (phase 1) and the relayed route with its
  pairing gate, the five reads, the worker with the KI-587 clock set and the
  build's asset list (phase 2). Measured through the paired listener at
  390x844, 1.5 Mbps / 150 ms RTT, cleared cache: painted 436 ms, 19,726 B on
  the wire; the shell is 57,612 B uncompressed / 18,974 B gzip over
  html+js+css against 61,440 / 20,480.
---

# The mini console — the TUI's economy, relayed to a phone

Status: completed 2026-11-22 — the face, the relayed route, the five reads and
the worker all landed and measured; every criterion below carries its number.

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

- [x] The shell numbers above, measured on the real phone link profile the
      compression plan measured (1.5 Mbps / 150 ms RTT), from a cleared cache,
      through the relay.
      *Phase 1 pinned the budget half: 52,452 B uncompressed / 16,868 B gzip
      over the three files. Phase 2 (the route) closed the cold load: the
      the shell is now **57,612 B uncompressed / 18,974 B gzip** (console.html
            3,940 / 1,863, console.js 45,264 / 14,132, console.css 8,408 / 2,979)
      against 61,440 / 20,480 — the growth is the page's boot clock and the
      installability block. Measured 2026-11-22 through the paired listener
      (`fleet gui --remote`, the release binary, a paired device cookie) at
      390x844, `Network.emulateNetworkConditions` 150 ms latency /
      187,500 B/s, worker unregistered and every cache cleared: **painted in
      436 ms**, **19,726 B on the wire** (page 2,092 / stylesheet 3,256 /
      script 14,378, all gzip). Warm mount from the worker's cache: 10 ms,
      0 bytes.*
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
- [x] The KI-587 clocks present in the new worker (deadline + evaluate-time
      watchdog + one-generation sweep) — red-proofed by the freeze coming
      back when a clock is removed.
      *Landed: `src/gui/assets/console-sw.js` (cache `fleet-console-<build>`,
      `fetchWithin` at 6,000 ms assets / 8,000 ms navigation, `anyVersion`
      fallback, a born stamp, and an activate that claims before it sweeps
      one generation LATE), registered from the page at the build's `?v=`
      with scope `/console/`, plus two boot clocks: the page's (armed at
      script evaluation in the head, 15 s, the named card) and the program's.
      The page's clock and the registration are in the HEAD because the
      failure they exist for is the one that stops the body: measured before
      the fix, a bumped coordinate with the new script silent left the page on
      its noscript line for 47 s with nothing said, and a body-level
      registration was never reached (a stalled deferred script stops the
      parse before it) — so the worker that had the previous generation
      cached never installed and the freeze came back on every load.
      *Cells, re-measured 2026-11-22 on the mini console behind the fault
      proxy (every byte from the release server): warm mount from the
      worker's cache **10 ms / 0 B**; a stalled update answered from the
      previous generation's cache **6,032 ms, no card, 0 B** (the new
      generation's cache holds only the born stamp — its prefetch was bounded
      by the 6 s deadline — and the old cache is still there, which is what
      answered); the load that ships the bump names the failure at
      **15,046 ms** rather than freezing; a refused navigation gets the
      worker's own page at **55 ms**; after install the cache holds the two
      assets and the born stamp.*
      *Red-proofs: `anyVersion` removed → `the worker carries the KI-587
      clock set, in its own cache` red; the one-generation sweep made to
      delete every older cache → same case red; the asset deadline replaced
      by a bare `fetch` → same case red; the page's clock removed → `the boot
      watchdog is armed at script evaluation and stands down on paint` red.
      All restored, 42 checks green.*
- [x] Relay pairing gates the route; an unpaired client is refused by name
      before the face is reached.
      *Landed in `src/remote/listener.rs`: `/console/`, `/console/console.css`,
      `/console/console.js` and `/console/console-sw.js` on the Console face
      (the bot's scoped mount, mirrored), `/console/` in `is_page` so an
      unpaired browser gets the pairing form, and the five reads in
      `ALLOWED_GET_API` — `/api/board`, `/api/delegation`,
      `/api/second-brain`, with `/api/sessions` and `/api/session` already
      there. Pinned by
      `the_mini_console_rides_the_paired_listener_with_its_own_mount`
      (`tests/integration.rs`): unpaired `/console/console.js` and
      `/api/board` are the gate's own 401 ("serves paired devices only"), the
      paired page carries the build's coordinate on both asset links and the
      worker's registration, the gzip twins decode to the files' own bytes,
      the worker is served stamped with `Service-Worker-Allowed: /console/`,
      all five reads answer a document, and `/api/settings` is refused by
      name — on the socket AND through a relay frame.
      *Red-proof: `/api/board` removed from `ALLOWED_GET_API` →
      `panicked at tests/integration.rs: /api/board is a paired read:
      http/1.1 403 forbidden`; restored, green.*
- [x] The bot PWA stays untouched in behaviour (its own tests still pass);
      the mini console is a sibling file set, not a bot.js fork.
      *Measured 2026-11-21: no `bot.*`, `app.*` or `sw.js` file is touched
      by this work — the console carries its own palette block, its own
      wrapping law and its own renderers, and imports nothing. The face
      harness passes unchanged (re-measured 2026-11-22 after phase 2:
      `node tests/face/harness.mjs` → 1,839 checks across 271 cases, exit 0).*

## Files

- `src/gui/assets/console.html` + `console.js` + `console.css` — the new face
  (sibling set, bot-PWA-shaped). **Landed in phase 1.**
- `src/gui/assets/console-sw.js` — the worker, `fleet-console-<build>`, the
  shell worker's clocks restated for this face's two assets. **Phase 2.**
- `src/remote/listener.rs` — the route + the pairing gate (mirrors `/bot/`).
  **Phase 2.**
- `src/gui/server.rs` — `CONSOLE_*` assets, `console_page()`/`console_worker()`
  and the four `GZIP_ASSETS` entries; `build.rs`'s `ASSETS` gains the four
  files (page and worker `Stamp::Marker`, the two assets `Stamp::None` — the
  bot's own accounting) so they enter the build stamp and get gzip twins.
  **Phase 2.**
- tests: the listener/pairing tests per the bot scope's pattern; the face's
  four-sentence pins in a console-specific suite (the face harness stays
  untouched). **The console suite landed in phase 1**
  (`tests/console.rs` + `tests/console/harness.mjs`); the listener's own
  tests are phase 2.
