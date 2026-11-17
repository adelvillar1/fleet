# Session Recap — 2026-11-16: three releases, a voice, and the shell's honesty

**Summary.** A long session that began with the cache question and ended with
fleet listening. It cut **three releases** (0.8.9, 0.8.10, 0.8.11), built the
cache instrument and then told the truth about what it measured, closed the
orchestrator and the resource contract, burned the known-issues ledger to two
open rows, built the deck clustering and let its own measurement keep it
opt-in, and then spent the tail of the day making the shell stop showing
things nobody asked for: the help now names every verb, the drawer opens only
on a press, the rail's install pill is gone and its collapse actually narrows
the column, the bot door lives in the composer (and starts a conversation from
the landing), and **voice dictation** shipped in the composer and the CLI,
transcribed locally by hex. The owner's black-hole report — a running
conversation that could not be reopened — closed the day as KI-557, and the
sweep it started found one more instance of the class (KI-558). Thirty-six
commits, both remotes current, the gate green at every step.

## The cache: measured, then made honest

- **The instrument** (`ef8fc62`): a Settings → cache surface reads the
  calibration stream per (provider, model) — the prompt share served from the
  provider's cache, cached-vs-fresh latency (p50/p90), a day-by-day series,
  and a caveat that a provider which reports nothing is *unknown, not zero*.
- **The finding that mattered** (`21655a9`, `d1ce290`): the cache read was
  flat across gaps (8.7% of prompt tokens over 187 route calls, 34 hits) and
  **5 of 5 within-run continuations missed**. The mover was not the tools and
  not expiry — it was the **declaration ORDER**: `profile_order` re-sorted the
  native declarations by the workload the router judged, rewriting the head
  the provider's prefix cache keys on. The declarations now keep one compiled
  order, the head is fingerprinted (`head_sha256`, `head_bytes` on the run's
  start record), and the workload's steering rides the brief as words
  (`## Order of reach`).
- **The measurement**: an identical 31,271-byte head read **5,248 of 12,833**
  prompt tokens (41%) from cache on direct DeepSeek; opencode and zai reported
  zero at first, which the owner ruled is **no data**, not "no caching" (the
  demotion I had inferred was reverted, `d4ad779`, and the rule is now in the
  ledger: never rank a provider by a figure it does not report).
- **The correction** (`b0bb689`): z.ai's PaaS endpoint *does* report —
  `prompt_tokens_details.cached_tokens` — and caches the **previous request's
  full message list**. A fresh task after the same head reads 0; a growing
  run hits. A live zai run measured **10,176 of 20,519 (49.6%)**. ZCode gets
  fresh-run head caching only because it speaks z.ai's Anthropic endpoint
  with explicit `cache_control` breakpoints, which the public endpoint does
  not honor for a plain coding-plan key.
- **Folds at 85%** (`a05d8e7` — committed late, caught by the pre-cut status):
  `compaction::budget_for` returns 85% of a declared window; the floor stays
  for undeclared windows; the old `window − 32k` reserve is gone.

## The orchestrator, and the plans

- **The orchestrator closed** (`eb5f2f7`): the box-by-box check found two real
  gaps — `walked_as` never reached the route row, and there was no work graph.
  Both built: a route row carries the judged `workload` beside the chain that
  actually walked; each role's card shows the threads it owns and the
  delegation edges among them; a conversation names its orchestrator.
- **The add-a-bot door** (`5b061cc`, `b0e28a7`) opened on the press rather than
  the settings read, and turned a plain conversation into a room.
- **The plan sweep** (`4bdf32d`, `09587aa`, `ec7bddc`): statuses speak one word,
  a stale plan can no longer sit silent (the checker warns past 21 days), seven
  plans closed and one cancelled; the prefix plan dropped its three deferred
  criteria with reasons instead of parking them.
- **The KI burndown** (`4a8f732`, `7a7698d`): eleven open rows closed with
  evidence — settings 1.87 s → **0.018 s warm** (the lifecycle scan cached 60 s
  plus `session::newest`; one hotspot would still have shipped 0.9 s), boards
  deadlines made total (a product panic, not test patience), relay retry, the
  release read-back falling back to `gh release view`, sticky routing measured
  at 4% and closed as not worth a slice (KI-548).
- **The deck clustering** (`bb7d908`): split → embed (loopback) → agglomerative
  grouping → map per type. The measurement gate is 0.90; it scored **0.732**,
  so `deck.clustering` ships **off** — the measurement, not a hope, decides
  (`a0b8371`, KI-252 closed).
- **Cleanup is part of shipping** (`7a7698d`): a curtailed run queues the files
  it wrote, `fleet cleanup` deletes them inside their own folder, and a cut
  runs it plus the fleet-named `/tmp` sweep.

## The releases

- **v0.8.9** (`54a932e`, feed read-back 0.8.9, cli sha `21fc8021e33d`) —
  caching you can see: the cache instrument, the stable head, folds at 85%,
  the add-a-bot door, the orchestrator's rows and work graph, deck clustering
  opt-in, cleanup on cut.
- **v0.8.10** (`fe6a688`, feed 0.8.10, cli sha `03df5fb269a2`) — the shell
  stops showing what you did not ask for: the drawer opens only on a press,
  the rail's install pill is gone (and the spill it had been covering is
  fixed), and the help names every dispatched verb (KI-554).
- **v0.8.11** (`e1b29f2`, feed 0.8.11, cli sha `aa07e5e652cd`) — **voice
  dictation** (composer + `fleet dictate`), the collapsed rail narrows to
  66px, and the bot door lives in the composer (from the landing it starts the
  conversation with the picked roles). The gate's settle came back **clean**
  (5.87 GiB against 6.78) for the first time in several cuts.
- Every cut ran the four-leg gate on a detached worktree, pushed the tag to
  **both** remotes before `gh release create`, and read the feed back
  anonymously. The public GitHub repo now tracks master (the owner's ruling:
  both ends stay current; `ls-remote` is the truth, the mirror lags minutes).

## The shell's honesty (the day's tail)

- **The drawer** (`f520fad`): desk and learning forced the right drawer open
  on arrival and overwrote a close; now no arrival opens it, the context
  paints while closed, and the press lands on the lens's context tab (U13
  amended). **KI-555 stays open**: the open drawer still covers the
  connections column.
- **The rail** (`6a05d0f`, `960b254`): the install pill (the U10 Mission card)
  is gone, and removing it uncovered `.surfaces-head`'s fixed 29px box holding
  a 40px mark — the card had been covering the spill. The collapsed rail now
  narrows the column through one `--rail-actual` token; the
  connections-column grid rules had been out-specifying the icons rule.
- **The bot door** (`43ced6d`, `9f78f3a`): the conversation head was the wrong
  place — it is absent where a conversation starts. The door moved to the
  composer; on the landing its press **starts** the conversation with the
  picked roles (the huddle path, no goal yet).
- **Voice dictation** (`eef715f` S1, `a80b21a` S2): hex (Rust, MIT, macOS/
  Linux/Windows) is the transcriber; the fleet process owns the microphone.
  `fleet dictate` takes a WAV or records N seconds; the composer's **Dictate**
  button records on the server (cpal) behind loopback-only
  `/api/dictate/start|stop`, keeps the hex service warm (ready in 0.03 s,
  warm clip 0.11–0.12 s, first request 1.44 s for the model prewarm), and
  inserts the words at the caret. Audio never reaches disk; a missing model is
  refused in hex's own words; the native shell carries the microphone usage
  description. S3 (history) is the only plugin-shaped piece and stays ruled
  out by the plugin sandbox.
- **The black hole** (`0556d0b`, `2399627`): the owner's running session could
  not be reopened — the threads lens detached the view but left
  `running`/`driving` set, and `openWork` skipped the resume when it believed
  it was still driving. The skip now requires proof (`attached`: same session,
  visible log, stream rows), the detach clears the run state, and every
  arrival closes the transient overlays it did not open. The sweep
  (`docs/architecture/2026-11-16-orphaned-by-navigation.md`) checked the rest
  of the class — drafts, queue, pane, rooms, desk/learning, seats, the archive
  — and found them safe with reasons.

## The gate and the ledger

- Final: fmt/clippy clean, `cargo build --release`, **2,552 tests**, face
  harness **1,303 checks across 188 cases** — including two regression cases
  that fail on the pre-fix code (`REATTACH …`, `NAV …`).
- Ledger: **558 rows, 2 open** — KI-555 (the drawer covers the connections
  column) and KI-556 (the updater signature is not version-bound; a one-flag
  fix, `--app-version`). Everything else from the day (KI-544…KI-554,
  KI-557, KI-558) closed with evidence.
- Docs: the README re-read against the app and rewritten where it was wrong
  (six lanes, nine lenses, the cache, the orchestrator, decks, cleanup,
  folds); both archify drawings re-authored at the release commit (16
  components / 18 connections; the turn as an 8-participant sequence);
  `FUNCTIONAL-SPECIFICATIONS.md` gained the cache surface; archify itself
  updated to 3.2.0.

## Notes for the next session

- **0.8.12 is pending** the two fixes since the 0.8.11 tag: `0556d0b` (the
  driven conversation reattaches) and `2399627` (arrivals close transients).
  Both are deployed locally and on both remotes; only the release is owed.
- **KI-555** is the one known UI debt: the open drawer is anchored at the
  right edge and covers the connections column, against U13's "persistent
  panes never hide behind it". The proposed fix is its right offset = the
  tissue's width (the 30px strip when collapsed), verified at 1440 and wide.
- **hex is now a local dependency**: 3.4.0 via `anomalyco/tap`, the CLI linked
  into `~/.local/bin`, the `parakeet_unified_en` model (731 MB) in hex's own
  store. The embedded service needs no hex Labs flag. The mic grant belongs to
  the process that opens it (the shell prompts under its own identity; a
  terminal runs the TUI/CLI path under the terminal's grant).
- **The plugin rule learned**: every bundle mounts sandboxed
  (`allow-scripts`, srcdoc, no network but the host's read-only relay), so a
  feature that must record, fetch or spawn needs a **core seam**; the sandbox
  is not to be widened for one feature.
- **The OAuth idea is parked** (the owner: "just an idea, no need to jump to
  it"): the mechanics fit fleet one-for-one, but the client registration is
  z.ai's own and the code exchange lives at `zcode.z.ai`; the honest path
  would be a fleet app registration plus PKCE and a sealed token store.

## Housekeeping

- `fleet hygiene`: 0 checkpoint turns pruned, 0 bytes; the memory corpus needed
  no consolidation; 0 orphaned fleet processes.
- Cache sweep: **606 MiB** of superseded compilations dropped; free space
  83.4 → 83.9 GiB (release binaries and bundles untouched).
- Both remotes: `origin` (gitea) and `github` carry the session — the last
  code commit is `2399627`, with this recap on top.
