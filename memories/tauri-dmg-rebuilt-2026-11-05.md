---
name: tauri-dmg-rebuilt-2026-11-05
description: "The Tauri dmg rebuilt 2026-11-05 07:13 carrying the desk face + KI-432 guard (verified by mounting) — but /Applications/fleet.app still sits at Nov 4 13:06, install offered not taken"
type: project
date: 2026-11-11
basis: inferred
tier: durable
shape: []
phrases: []
missing: []
premises: []
recall-count: 0
superseded-by:
valid-until:
source: zcode:fleet:content:285983102772d99aaed2c496d57acdf6f320aad7347d3891e9bb9e2378839d34
---

The owner asked "was the tauri app rebuilt?" (2026-11-05) — the honest
answer was "rebuilt yesterday, but two generations stale in
/Applications." State after the rebuild:

- `target/release/bundle/dmg/fleet_0.3.0-beta_aarch64.dmg` (10.40 MiB,
  Nov 5 07:13) carries the desk face and the KI-432 null-panel guard —
  verified by `hdiutil attach` + grepping the bundled binary, not by
  trusting the build's success line.
- `/Applications/fleet.app` still dates Nov 4 13:06 (the huddles-room
  era): it predates context-health, the desk, the transplant, and the
  guard. The dock app runs its OWN bundled server on an ephemeral port
  — it never talks to the LaunchAgent face at 127.0.0.1:3080, so the
  two doors age independently. Install offered; the owner has not
  taken it yet.

**Bundle-verification lessons (2026-11-05):**
1. `cargo tauri build`'s "Finished 1 bundle" names the dmg while
   `target/release/bundle/macos/` can be EMPTY — verify inside the
   mounted dmg, not the macos dir.
2. The dmg contains ONE binary: `fleet.app/Contents/MacOS/
   fleet-desktop` — it IS the server (the face assets are
   include_str'd into it); there is no standalone `fleet` binary in
   the bundle. The guard and the desk icon grep it; the Resources
   side carries only services/ + icons.
3. Marker choice matters: `cards()`'s CLI string ("the desk holds no
   deliverables…") is ABSENT from fleet-desktop — the linker strips
   CLI-only dead code from the lib. A bundle marker grep must use a
   string from the SERVED path (assets, routes), or it false-negatives
   on a perfectly good build.

**Later the same day (2026-11-05, `de5d643`):** the dmg was rebuilt again
(10.87 MiB) carrying the sys1 extraction verb — mounted and read for
`sys1_extract` in the bundled `mcp_server.py`, `def extract` in
`extraction.py`, `task_spec` in the handler — and `/Applications/fleet.app`
is now BYTE-IDENTICAL to `target/release/fleet-desktop` plus the services
copy (a real directory, not a link; not running). So the install gap this
file recorded is closed: the dock app is current at `de5d643`.

**Third rebuild the same day (2026-11-05, `f50e971`, the goal-pane close-out):**
`fleet_0.3.0-beta_aarch64.dmg` 10.90 MiB (11,433,334 B), mounted and read —
`fleet-desktop` (25,651,424 B) carries the face (`gp-goal-text` ×3,
`goal_done` ×4). The interrupted-build lesson repeated: the FIRST
`cargo tauri build` was cancelled by the owner's question and I left it
reported as open until "continue" — an interrupted leg is never a done leg,
and the close-out ask ("you can do and should do all that") means ship every
leg in ONE turn.

**Fourth rebuild the same day (2026-11-05, `fff6dc3`, the KI-453
New-control fix):** `fleet_0.3.0-beta_aarch64.dmg` 10.89 MiB, mounted
and read — the bundled `fleet-desktop` carries `detachToWelcome` ×2.
The build ran clean in one pass under the default `cargo tauri build`.
Open leg after this rebuild: `/Applications/fleet.app` was last proven
byte-current at `de5d643` (the extraction close-out); whether it now
sits two generations back (goal-pane + KI-453) is UNVERIFIED — the dock
app is its own door and ages independently of both the dmg and the
LaunchAgent face. **This is the dmg's current state** (no rebuild has
followed `fff6dc3`; the recap's final commits `1259e91` and `3bd0c08`
were docs-only and memories churn).

**The install question, still open:** the dmg is rebuilt and verified,
but `/Applications/fleet.app` has not been re-installed since
`de5d643`. The dock app runs its own bundled server on an ephemeral
port and never talks to the LaunchAgent face at 3080 — so the owner's
browser face and the dock app are two doors that age independently, and
"the dmg is current" does not mean "the installed app is current".

**Why:** the dock app and the browser face are two doors that age
independently; "was it rebuilt" needs mount-level evidence, and a
wrong marker string reads as a missing feature.

**How to apply:** after any face delivery, rebuild the dmg, mount it,
and grep fleet-desktop for served-path markers; treat
/Applications/fleet.app's mtime as its own version. See also
[[stale-shell-and-the-null-panel-dispatch-ki-432]],
[[tauri-native-shell-implemented-2026-11-02]],
[[the-served-face-is-the-installed-copy]].

**The install gap this file recorded is CLOSED (2026-11-07).** The
owner asked for the dmg for his local machine; the rebuild
(`fleet_0.4.0-beta.1_aarch64.dmg`, 10.84 MiB) was mounted, the old
`/Applications/fleet.app` MOVED ASIDE (not deleted — the destructive
gate blocks `rm -rf`), the fresh app installed, quarantine cleared, and
the local face relaunched on it — proven by its own `/api/memory`
answering the new document. The current dmg/install state is
[[dream-dmg-installed-face-relaunched-2026-11-07]].

**The install gap REOPENED (2026-11-07, same day).** The dream-plan
merge to master (`1eaba7a`) and the self-improvement program's branch
work landed AFTER the install, so `/Applications/fleet.app` again
sits behind the trunk — the dock app is its own door and ages
independently of both the dmg and the LaunchAgent face. The lesson
this file keeps earning: "the dmg is current" does not mean "the
installed app is current", and a same-day merge reopens the gap the
same day.

**The install gap CLOSED AGAIN (2026-11-08, the hybrid-retrieval delivery).**
The owner asked "rebuild the app" after the hybrid-retrieval merge to master
at `29daca3`. `cargo tauri build` from that master produced
`fleet_0.4.0-beta.1_aarch64.dmg` (10.09 MiB, 21:24); the owner copied the app
to `/Applications/fleet.app` (binary 21:23); the face was relaunched
(`open -a fleet`) and serves on 3080 — pid 98678, `GET /api/home` 200 (30,484
bytes), `GET /app.js` 200. The installed binary carries `hybrid_search` three
times — the new tool's name, spec, and dispatch — so the running app provably
rides the new work. The marker lesson re-earned: `BM25_K1`/`RRF_K` do NOT
appear in the binary because a release build strips symbol names; a
bundle-marker grep must use a string literal from the served path. The current
dmg/install state is [[hybrid-retrieval-delivered-2026-11-08]].
