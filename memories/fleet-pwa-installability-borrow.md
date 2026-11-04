---
name: fleet-pwa-installability-borrow
description: "2026-10-20 — fleet's browser face is now a PWA (plan 2026-10-20-pwa-shell, completed): build.rs FNV asset stamp, ?v= immutable assets, manifest+icons, a worker caching only the two versioned assets with a path-based bypass (cookie, not Authorization), 503 retry page, ES5 guard + 15s watchdog, remote surface paired-only; the Android shell was refused"
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

**Status (2026-10-20): BUILT, COMMITTED AND PUSHED** as `1d910f4`, together with the
bots repair and the pairing card — one commit, because the concerns interleave in the
same files (`app.js`, `app.css`, `app/server.rs`, the docs), so a five-commit split
would have produced intermediates that neither built nor behaved. The owner confirmed
"the important thing is the pwa", the plan `docs/plans/2026-10-20-pwa-shell.md` is
**completed**, and the four-step gate is green (1280 lib tests + every integration
suite). QwenLM/qwen-code PR #11722 was the idea source; its Android WebView shell was
deliberately refused — a Kotlin/Gradle tree and second distribution for a
one-static-binary crate.

**What shipped**

- `build.rs` hashes every served asset with FNV-1a (std only) → `FLEET_ASSET_HASH`.
  `index_page()` rewrites the two asset URLs to `?v=<stamp>`; only those are
  `public, max-age=31536000, immutable`. HTML, the bare asset paths, the manifest,
  and the worker are `no-cache`. The worker's cache name is `fleet-shell-<stamp>`
  and the served `/sw.js` has the placeholder substituted and
  `Service-Worker-Allowed: /`.
- **The worker caches exactly two URLs** — the versioned `app.css` and `app.js`,
  prefetched at install — and returns early for non-GET, cross-origin, `/api/*`,
  `/events`, `/pair`, and event-stream requests. HTML is never cached: a navigation
  is network-first and a failure answers a themed 503 "fleet is not reachable" card.
- Manifest (`start_url: "/"`, standalone, paper `#f6f4ef`, 192/512 PNG icons),
  `apple-touch-icon`, `theme-color` synced to the chosen theme
  in both the head and `setTheme`, and `data-fleet-mounted` set when the shell boots.
- ES5 guard names fleet's real floor (`color-mix` + `100dvh`: Chrome/Edge 111+,
  Firefox 113+, Safari 16.2+); a 15-second watchdog shows one themed card with the
  mark and a retry if `app.js` never mounts.
- The remote listener serves the same install surface, adds the manifest/worker/icons
  to its default-deny allowlist, and keeps them **paired-only** (an unpaired request
  still gets the pairing page or a 401). Icons are written by `write_blob` because
  `Reply.body` is a `String`.

**The rule that matters if anything here is rewritten:** the boundary is **by path,
not by header**. Qwen's worker skips requests carrying `Authorization`; fleet's remote
face authenticates with a **cookie**, which the browser attaches after the fetch
handler sees the request. A header test would cache an authenticated response.

**Secure context is the gate:** loopback and an HTTPS remote face (Tailscale serve —
[[tailscale-remote-access]]) are installable; plain-HTTP LAN is not; the relayed face
under `/s/<sid>/` is per-session and the registration skips it explicitly.

**Found by the test that added the remote assertion:** KI-275 — the remote face
answered `GET /` with the pairing page *before* reading the cookie, so the redirect a
successful pairing performs (`location.href = '/'`) looped back to the pairing form.
A paired phone could reach every allowlisted route but never the shell at the root.
Fixed: the device store is resolved before the root is decided; `/pair` stays the
unpaired page.

**Verified live (Chromium on 127.0.0.1):** worker registers/controls, the cache holds
only the two versioned assets, `cache.match` is empty for `/api/state`, `/events`,
`/pair`, `/`; offline reload shows the retry card; a Chrome 90 UA shows the update
card; an aborted `app.js` shows the watchdog card. Headers and the binary icons were
checked over raw HTTP, not Node `fetch` (undici crashed on the hand-rolled responses).

**Confirmed on the owner's phone (2026-10-20):** "worked like a charm I have the
app on my phone now" — the first real-device evidence, after the pairing card
([[fleet-pairing-qr-card]]) shipped. Note what it does and does not prove: an
installed icon is not proof the worker registered, because only a **secure
context** (loopback or the HTTPS tailnet origin) gets one, and a LAN-HTTP install
still gets the icon. WHICH origin the owner scanned is unrecorded — do not cite
this as evidence the tailnet/HTTPS branch was exercised.

**Installing it on the Mac (owner asked 2026-10-20 "how do I make the pwa appear as an
app icon on my mac"):** from a Mac, the **loopback** face is the secure context —
`http://127.0.0.1:3080` (default) or `127.0.0.1:3090` when a scratch face is there.
Chrome/Edge: install icon at the right of the address bar, or ⋮ → "Cast, save, and
share" → "Install page as app…"; Safari: **File → Add to Dock**; Firefox has no
desktop install. The icon is a **client, not the server** — it needs `fleet gui`
running behind it, and if the face is down the installed window shows the worker's
retry card rather than a blank tab (the one case where the worker earns its keep on
the desktop). The same manifest serves the phone; the two installs (Mac vs phone)
are separate apps to the host. Answering this, check the port actually serving: an
old face on 3080 is the usual reason "it isn't there".

**Firefox desktop (owner asked 2026-10-20 "how about on firefox?") — it cannot
install, and that is a platform fact, not a fleet gap.** MDN states it plainly under
installability: "Firefox does not support installing PWAs using a manifest file," and
Firefox is absent from the "installing sites as apps" list that names Chrome, Edge,
and Safari. So no install button, no standalone window, no dock icon from the
manifest. What survives in a Firefox tab is the shell plus the **worker**, which does
register on `127.0.0.1` because that is a secure context — so the versioned-asset
cache and the 503 retry card still work there. Firefox's PWA support is also
platform-dependent on mobile: on iOS every browser is WebKit, so Firefox's
Share → Add to Home Screen gives a real installed app, while on **Android** Firefox
only adds a browser-badged shortcut rather than a standalone app. The workarounds
worth offering: use Chrome/Edge/Safari just for the icon (the installed window is
independent of your daily browser), or an Automator "Run Shell Script" app running
`open -a Firefox http://127.0.0.1:3080` — an icon that opens a tab, honestly not an
app, with an icon of your choosing rather than the manifest's. Do not promise a
Firefox install or imply the manifest is at fault.

**The Mac icon (owner asked 2026-10-20 "do you have a way to create an icon for the app?").**
Yes — render from SVG in Chromium headless (Playwright's chromium-headless-shell, a local
file), 1024 px squircle with macOS's 824-px body and 185.4 corner radius, then
`sips` every size into `fleet.iconset` and `iconutil -c icns`. For a Dock icon on a
launcher app: `osacompile -o ~/Applications/fleet.app -e 'do shell script "open -a Firefox
http://127.0.0.1:3080"'`, overwrite `Contents/Resources/applet.icns`, **delete
`Assets.car` and remove `CFBundleIconName`** (either one shadows the copied icns), then
`codesign --force --deep --sign -`. Notes that cost time: `qlmanage -t` hangs on app
bundles here (use `sips -s format png <bundle>.icns` to check); `Icon\r` alone does not
reliably show in the Dock. PWA icons are full-bleed (no transparency — iOS masks), the
Mac icns keeps transparent margins.

**Serving a rebuilt binary to the owner (2026-10-20):** after the PWA landed the
owner still saw nothing new, because the face on port 3080 was a process started
the night before — a running face serves the assets compiled into its own binary
([[verify-the-binary-is-newer-than-the-source]]). Restarting it on the new binary
(3080 now also runs the remote listener, `--remote`) is what makes the manifest,
worker, and pairing card visible. A PWA change therefore always ends with: rebuild,
restart the face, then grep the SERVED `/app.js`. **The shipped PWA hash on master
at 1d910f4 is `b1dbf93fc246088b`** — grep that to confirm a face is current. Two
things survive a reload and stay stale until the owner acts: an **installed PWA**
(phone or Mac) keeps its cached icons until re-added or reinstalled, and the
owner still needed the reload to see the push at all.

**"Did you restart the face?" needs a named trigger, not just a yes (owner asked
2026-10-20 "did you restart the app that is running on port 3080?").** The last restart
*I* performed was installing the LaunchAgent (~15:25, PID 44925); the face serving 3080
was PID 42145 started 16:39:45 by the agent itself (`launchctl print … | grep runs` showed
`runs = 2`) — a KeepAlive restart or a restart by the parallel session, not mine. Report
the distinction: what restarted, when, and which trigger, then what that build contains.
Same trap as [[verify-the-binary-is-newer-than-the-source]] — the honest answer is
build state + process state + HEAD, never "yes".

**After the phone install (2026-10-20):** the face now runs under
`~/Library/LaunchAgents/com.fleet.gui.plist` (`fleet gui --no-open --remote`, restart with
`launchctl kickstart -k gui/$(id -u)/com.fleet.gui`), and the icon in the manifest is the
serif **f** on graphite (full-bleed) — see
[[fleet-pairing-qr-without-tailscale-serve]]. A binary built before a parallel session's
commits is not HEAD: check `git log` before saying the newest features are live.

**Why:** owner-facing and reversible-looking but easy to get wrong; the cookie-vs-header
inversion and the relay subpath are exactly what a naive port gets wrong.

**How to apply:** if fleet adds another worker or cache anywhere, reuse the rule —
cache only content-addressed URLs, bypass session paths by path, never cache HTML.
Related: [[fleet-channels-bot-surface-gap]] (the face this installs),
[[verify-what-renders-not-what-commits]], [[verify-the-binary-is-newer-than-the-source]].
