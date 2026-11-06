---
name: fleet-pairing-qr-without-tailscale-serve
description: "2026-10-20 — the pairing QR card works, but with no `tailscale serve` config the URL it mints is plain-HTTP LAN, so the owner's phone installed the app WITHOUT the PWA worker; serve is blocked on an account gate the owner must open, and the face now runs under a LaunchAgent (com.fleet.gui) rather than nohup — which means the agent, not the session, can restart the face, so \"did you restart it?\" needs a named trigger; 2026-10-28 the bot experience's access URL is the LAN origin http://192.168.68.70:3080 (pai…"
type: project
date: 2026-11-05
basis: confirmed-by
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

**The card is real; the origin it chose was the weak one.** After
[[fleet-pairing-qr-card]] shipped, the face on 3080 (restarted with `--remote`) had no
`tailscale serve` config, so `pairing_origin` fell through to
`http://192.168.68.64:3080` with the plain-HTTP note — and the owner scanned that and
reported "worked like a charm I have the app on my phone now". **That install is not
evidence the PWA layer works on the phone**: installability/worker need a secure context,
plain LAN HTTP fails it, so the phone got a home-screen icon with no offline shell. Only
the loopback face on the Mac and an HTTPS tailnet URL get the worker. Do not cite the
phone install as proof of the PWA; say which origin it came from.

**Serve is an account gate, not a syntax problem.** `tailscale serve --bg 3080` (bare
port — test the gate with the shortest target first) prints "Serve is not enabled on your
tailnet" plus `https://login.tailscale.com/f/serve?node=n2gY9jTuYJ11CNTRL`; the full LAN
URL version hung ~90s and looked like a hang. Do not enable a tailnet-wide setting for the
owner — hand them the link. Once enabled:
`tailscale serve --bg http://192.168.68.64:3080` — the **remote** listener, never the
port shorthand, which maps the loopback face that has no `/pair`. Then reopen the card;
it probes `/api/state`, gets 401, and switches to
`https://alejandros-macbook-air-1.tail9d0d4c.ts.net` in ~4s. Cleanup after a failed
attempt: `tailscale serve reset` and `pgrep -f "tailscale serve" | xargs kill` — killing
the wrapper shell alone leaves the binary running. See [[tailscale-remote-access]].

**The face now outlives a terminal.** `~/Library/LaunchAgents/com.fleet.gui.plist`
(label `com.fleet.gui`, RunAtLoad, KeepAlive on non-zero exit, ThrottleInterval 10, logs
to `~/Library/Logs/fleet-gui.log`) runs
`/Users/alejandrodelvillar/Projects/fleet/target/release/fleet gui --no-open --remote`
from the repo with `WorkingDirectory` set. Manage it with
`launchctl kickstart -k gui/$(id -u)/com.fleet.gui` (this is the restart that makes a new
binary visible) and `launchctl bootout gui/$(id -u)/com.fleet.gui` to stop it. The manual
`nohup` face was killed when the agent took over. Because the plist names the binary by
absolute path, a worktree build never leaks into the agent.

**The app icon.** `assets/fleet-icon.svg` (serif **f** on graphite, one signal-blue rule)
→ `assets/fleet-1024.png` → `assets/fleet.icns`, all committed. The same mark is full-bleed
in `src/gui/assets/icon-192.png` / `icon-512.png` for the PWA (iOS masks transparency).
On the Mac the launcher is `~/Applications/fleet.app` (an `osacompile` applet running
`open -a Firefox http://127.0.0.1:3080`) carrying the new `.icns`; Firefox desktop cannot
install a PWA ([[fleet-pwa-installability-borrow]]), so the launcher is the Firefox
workaround while Chrome/Edge/Safari get the real install.

**Who restarted the face is a question the owner asks separately (2026-10-20).** He asked
"did you restart the app that is running on port 3080?" six messages after the icon
question. The face on 3080 was PID 42145, started 16:39:45 by the LaunchAgent
(`launchctl print` → `runs = 2`) — my last personal restart was the agent install at
~15:25 (PID 44925). So: it *was* restarted, but not by me, by the agent (KeepAlive or the
parallel session). Answer with the three facts — which process, started when, which
trigger, and which commits the binary contains — because "yes" alone implies the newest
commits are live, and they are not while a parallel session pushes after the build.
Same shape as the "rebuild ≠ HEAD" trap; see [[verify-the-binary-is-newer-than-the-source]].

**Delivery state (superseded 2026-10-20, later the same day).** The rail mark change
(`31d8cee`, the surfaces rail now draws the app icon instead of the placeholder glyph)
was built on top of HEAD and the face restarted **through the LaunchAgent**
(`launchctl kickstart -k gui/$(id -u)/com.fleet.gui` → PID 43594, 16:47:45), so the
binary now does contain `adfd353` and `9bd8b0d`; confirmed by the served page carrying
the new `app.css?v=cc5738ebc56d16bd` and `fleet-mark-tile`. That is the clean form of
"rebuilt and restarted": rebuild on HEAD, kickstart the agent, then read a string off
the served page. Icon files: `assets/fleet-icon.svg`, `fleet-1024.png`, `fleet.icns`,
and the PWA's `icon-192/512.png`. The paragraph below records the older state —
Everything through this work is `1d910f4`, pushed; a parallel session
then pushed `adfd353` (settings) and `9bd8b0d` (second-brain plugin), so the binary built
at 16:15 **predates those two** — "the app was rebuilt" is true only for the work this
session did. Rebuild + `launchctl kickstart` before claiming the newest features are live.
Verified by SHA-256 that the committed icons equal disk and that the served `icon-192.png`
equals the committed blob — the small proof that survives a "did you commit it?" question.

**2026-10-28 dogfood, two addenda.** (1) KeepAlive is aggressive: `kill`, `kill -9`,
and `pkill -f "fleet gui"` were each respawned within seconds — the only clean restart
is `launchctl kickstart -k gui/$(id -u)/com.fleet.gui`, and `launchctl list | grep fleet`
names the supervisor when a process keeps coming back. (2) The pairing redemption is
programmatic: `POST /api/pair` on the remote origin with
`{"code": "<minted code>", "name": "…"}` returns `Set-Cookie: fleet_device=…`, and that
cookie drives every route — a curl IS a paired device, which is the whole dogfood
instrument (see [[bot-pwa-dogfooded-live-01a9ab3]]). The console mints codes at
`POST 127.0.0.1:3080/api/pair/window`.

**2026-10-28: the owner's access URL, answered.** Asked "what url should I use to
access this persona experience?" — the answer, and the state it names: the bot face is
**`http://192.168.68.70:3080`** (LAN, the address the remote listener binds — verified
serving the pairing page), live on the post-dogfood build. Flow: `~/Projects/fleet/
target/release/fleet pair` (or the console's pairing card) mints a code with a
**5-minute TTL**; the unpaired root IS the pairing page, so the phone just opens the URL
and enters the code; then Share → Add to Home Screen installs as **fleet bot**
(standalone, own manifest/worker/icons — this time the real PWA question is open again,
since the worker boundary is the bot's own). Same port, two apps: `127.0.0.1:3080` on
the Mac is still the console; a phone's old console install from this origin updates in
place to the bot app on next load. Still LAN-only: the listener binds the LAN interface,
so off-network access waits on `tailscale serve` (the account gate above) — offered,
not yet wired.

**2026-10-28: "would this view be an installable PWA?" — yes by construction, but
installability is a property of the ORIGIN, not the app.** The bot face ships the whole
PWA set (manifest with maskable icons, the versioned-asset worker, iOS standalone meta)
and all of it verified served — yet from the LAN HTTP origin the install is degraded,
because service workers are secure-context-only and a plain-HTTP private IP is not one:
iOS Add to Home Screen still works (standalone, own icon, via the meta tags) but the
worker never registers — offline gets Safari's error page, not the bot's retry page —
and Android Chrome offers no install prompt at all (its criteria require HTTPS). The
first-class install needs the tailnet HTTPS origin, and the machine's name is
**`alejandros-macbook-air-1.tail9d0d4c.ts.net`** (tailscale running; verified via
`tailscale status --json` → `Self.DNSName`). The serve command must target the **remote
listener's LAN address** — `tailscale serve --bg http://192.168.68.70:3080` — never
loopback :3080, whose root is the console: the serve target decides which app the
HTTPS origin IS. Pairing at the HTTPS origin is a separate one-time code from the LAN
one (per-origin cookie), same flow. Offered to run the serve command; left to the
owner since it changes the tailscale node's serve config — the account gate above
still applies first.

**2026-10-30: the "off-network waits on tailscale serve" state is
retired.** The relay origins are the off-network path now
([[remote-relay-openship-plan-2026-10-29]]): the console at
`https://fleet.opsh.io`, the bot at `https://fleetbot.opsh.io` (a
second relay service, `fleet gui --bot-relay`), both sealed end to end
and pairing per origin — a phone on cellular reaches both with no VPN
client, so nothing waits on the account gate anymore. The QR card
renders `remote.public_url` (the console relay origin) first, and its
code is universal: **one minted code redeems at ANY origin** (one
shared window in the home, one use — every further mint invalidates it,
which is what burned the owner's held code this evening; see
[[pairing-at-an-origin-arms-the-transport]]). The local URLs —
`http://192.168.68.70:3080` (console) and `:3081` (bot) — work only on
the same Wi-Fi and take the same code. The serve instructions above
stay only for a tailscale-tailnet preference; nothing depends on them.

**2026-10-30: the bot QR now lives on the Robots surface.** The
owner's "this experience is wrong… it should show me a qr code for the
bot experience like it works on the regular fleet experience" landed as
`6e6ae83`: `#/bots` leads with a pair row whose **show QR** opens the
same card, minted by `POST /api/pair/window {"face":"bot"}` — the bot
relay origin with its own `#k=` fragment, or the LAN address with a
plain-HTTP note when no `--bot-relay` dials. The console's topbar card
is untouched, one shared window stands behind both cards (newest code
wins — the same rule that burned a held code), and the code remains
universal across origins. See
[[bots-surface-mints-the-bot-pairing-card]].

**Why:** the owner's next question is always "is it actually on my phone/Mac as an app",
and the answer depends on the origin and the process, not on the code being merged.

**How to apply:** before promising an installed app, state the origin (LAN vs HTTPS) and
its consequence (worker or no worker); restart through launchctl, not a stray shell; and
distinguish "the binary I built" from "HEAD", because another session may land commits
after it. Related: [[fleet-pairing-qr-card]], [[fleet-pwa-installability-borrow]],
[[tailscale-remote-access]], [[verify-the-binary-is-newer-than-the-source]],
[[rebuild-is-not-commit-push-or-ship]].
