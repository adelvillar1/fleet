---
name: tailscale-remote-access
description: "2026-10-20 — Tailscale 1.104.1 is installed on the owner's Mac; its onboarding keychain loop was caused by stale keychain items from a July install signed by a different binary (delete them + empty the trash — fix confirmed working); after sign-in the daemon sits stopped with WantRunning:False until `tailscale up`; the remote face's iOS-safe Tailscale path is `tailscale serve` terminating TLS, because iOS ATS blocks cleartext HTTP to the CGNAT range; and never verify the tailnet by curling your own address …"
type: project
date: 2026-11-04
basis: confirmed-by
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

Tailscale is installed on the owner's Mac (1.104.1 at
`/Applications/Tailscale.app`) as of 2026-10-20, as the off-network
transport for the remote-access work in
[[fleet-remote-access-implemented]].

**The onboarding keychain loop had a nameable cause.** The app prompts
for the keychain, the owner allows it, and it prompts again. The machine
carried stale keychain items from a July install (`tailscale-machinekey`,
`tailscale-current-profile`, `tailscale-id-profile-f873`,
`tailscale-logdata`, all created 2026-07-30 by a differently-signed
binary), an orphaned `~/Library/Containers/io.tailscale.ipn.macsys.login-item-helper`,
and a broken `/usr/local/bin/tailscale` stub pointing at a deleted app
bundle. The new build cannot read the old items, reads that as "no
credentials", and re-onboards forever. The fix: quit Tailscale, delete
the four items in Keychain Access and **empty the trash** (a plain
Delete leaves the deletion pending and the app keeps reading the old
item), `rm -rf` the orphaned container and the stub, reopen, and choose
**Always Allow** on the one legitimate prompt — "Allow Once" is what
makes some installs re-prompt on every launch. A prompt for the *system*
keychain rather than the login one is a different failure (the network
extension is unapproved) and lives in System Settings → Privacy &
Security.

**Two exposure paths, and the difference is iOS.** Fleet's remote face is
plain HTTP. `tailscale serve --bg --https 443 http://127.0.0.1:3080`
terminates TLS with a publicly-trusted `*.ts.net` certificate (MagicDNS
+ HTTPS certificates enabled in the admin console) and works on iOS
Safari — the credential is the launch token in the URL. `fleet gui
--remote --remote-host <100.x>` binds the tailnet interface and pairs a
device by cookie (no token in any URL, revocable per device), but **iOS
App Transport Security blocks cleartext HTTP to CGNAT** — 100.64/10 is
shared space, not the private-range local-networking exemption iOS
grants — so that path works on Android and fails on iPhone. A `.ts.net`
hostname can be ATS-exempted only by name, which needs a native app or a
config profile.

**How to apply:** when the `--tailscale` flag lands (auto-detect via
`tailscale ip -4` / `status --json`, TLS via `tailscale cert`), these are
the two paths it should formalize. If the owner reports the keychain loop
again, check for the stale items first — the app bundle's version tells
you which binary signed them.

**Landed 2026-10-20: the pairing card does the detection.** A QR icon on the
loopback face mints a code and shows a scannable URL. It reads
`tailscale status --json` (`BackendState: Running` → `Self.DNSName`, e.g.
`alejandros-macbook-air-1.tail9d0d4c.ts.net`) and only offers
`https://<dns>` when `<origin>/api/state` answers **401** — the paired
remote face; a **200** means the HTTPS origin is serving the loopback face
and `/pair` is not there, so it is refused by name. This machine has
**no serve config** (`tailscale serve status` → "No serve config"), so the
card falls back to `http://192.168.68.64:<port>` with a note naming
`tailscale serve`. Whenever the owner sets serve up, point it at the
**remote** listener (the LAN address), not `127.0.0.1`. See
[[fleet-pairing-qr-card]]. Also: a blocked resolver on a `.ts.net` name
outlives `ureq`'s timeout (KI-276) — bound hostname probes with a thread.

**The same fact gates PWA install (2026-10-20).** Service workers and
installability need a secure context, so `tailscale serve` (HTTPS) is the
only off-Mac way the remote face can be installed to a home screen; plain
HTTP over LAN or the tailnet fails the check. See
[[fleet-pwa-installability-borrow]].

**Serve is not enabled on this tailnet (2026-10-20) — an account-level
gate, not a command syntax problem.** `tailscale serve --bg 3080` prints
"Serve is not enabled on your tailnet. To enable, visit:
https://login.tailscale.com/f/serve?node=n2gY9jTuYJ11CNTRL". Before that
message appears the command can look like a hang — a foreground
`tailscale serve --bg <target>` sat for 90+ seconds with no output and
`tailscale serve status` still said "No serve config". **Test the account
gate first with the shortest target (`tailscale serve --bg 3080`), not
with the full LAN URL**, or a hang gets misread as a bad target. Do not
enable a tailnet-wide setting on the owner's behalf; give them the link.
After a failed attempt, `tailscale serve reset` and confirm no
`tailscale serve` process is left (`pgrep -f "tailscale serve" | xargs
kill` — killing the wrapper shell alone leaves the binary running).
Once enabled, the mapping is
`tailscale serve --bg http://192.168.68.64:3080` — the **remote**
listener, never the port shorthand (`3080` → `127.0.0.1:3080`, the
loopback face, which has no `/pair`). The target is the LAN address, so
a DHCP change breaks it; the pairing card reports when verification
fails. The card then switches to `https://alejandros-macbook-air-1.tail9d0d4c.ts.net`
in ~4s once the 401 probe succeeds.

**Serve was attempted and refused by the account gate (2026-10-20).** The owner asked me
to set up serve; `tailscale serve --bg 3080` answered "Serve is not enabled on your
tailnet" with `https://login.tailscale.com/f/serve?node=n2gY9jTuYJ11CNTRL`. I stopped,
ran `tailscale serve reset`, and killed the leftover binary (`pgrep -f "tailscale serve"
| xargs kill` — the wrapper shell alone is not the process). Enabling it is the owner's
call. Until then the pairing QR is plain-HTTP LAN and the phone install has no worker:
[[fleet-pairing-qr-without-tailscale-serve]].

**Confirmed 2026-10-20:** the fix worked — the owner reported "it's
running now" after deleting the items and emptying the trash. The next
state to expect is a second, different-looking stall: the app is open but
`tailscale status` says *stopped* with `WantRunning: False` and
`LoggedOut: False`. That is not a fault — it is the menu-bar toggle being
off after a fresh sign-in. `tailscale up` (or the menu-bar switch)
connects it. Distinguish the two by reading `debug prefs`: `WantRunning`
is the on/off switch, `LoggedOut` is the sign-in; a loop is the former
being stuck at false while onboarding, not this.

**Do not verify the tailnet by curling your own address.** After both
devices connected (Mac `alejandros-macbook-air-1` / `100.115.120.43`,
MagicDNS `alejandros-macbook-air-1.tail9d0d4c.ts.net`; phone
`iphone-13-pro` / `100.83.158.56`), Fleet's remote listener bound the
tailnet interface cleanly — but `curl http://100.115.120.43:<port>`
from the Mac itself times out and `ping` to that address shows 100%
packet loss. That is macOS not hairpinning traffic to its own tailnet
IP, not a broken listener and not a broken tailnet: `utun6` holds the
address and a `/32` route to the phone. The correct probe is
`tailscale ping <peer-ip>`, which answered `pong from iphone-13-pro via
192.168.68.69` — a **direct** connection, not relayed. The pairing
request therefore has to come from the phone; everything short of it
(listener bound, code redeemed once) is verifiable from the Mac, and the
last leg is the owner's to confirm on the device.

**Demoted 2026-10-29 — tailscale is no longer the remote path.** The
owner ruled the outbound relay (hosted on OpenShip) is the primary
remote path; tailscale drops to optional fallback and the `tailscale
serve` setup this file builds toward is retired as the intended HTTPS
origin. The machine facts here (keychain fix, hairpin probe, ATS/CGNAT
constraints, serve's account gate) still hold. See
[[remote-relay-openship-plan-2026-10-29]].
