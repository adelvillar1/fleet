---
name: fleet-pairing-qr-card
description: "2026-10-20 — pairing is now an in-app QR card (plan 2026-10-20-pairing-qr, completed, shipped in 1d910f4): a loopback-only POST /api/pair/window mints a fresh window per open and returns the QR SVG; the URL is remote.public_url, else a Tailscale name VERIFIED by /api/state (401 = paired face, 200 = loopback and refused), else the LAN address with a plain-HTTP note; KI-276 bounded the probe"
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

**Status (2026-10-20): BUILT, COMMITTED AND PUSHED** as `1d910f4`
("feat(face): the console's bots, an installable shell, and pairing from the
screen"), `origin/master` at the same SHA, and the 3080 face reloaded on that
binary. The owner asked "make the pairing via tailscale easier, create a qr
with the url so that I can open it on my phone, put an icon on the screen that
will show a card with the QR". Plan `docs/plans/2026-10-20-pairing-qr.md` is
**completed**; the four-step gate is green (1280 lib tests + every integration
suite).

**The card.** A topbar QR icon — rendered only on the **loopback** face (not
on a paired phone, not on the relayed `/s/<sid>/` path) — opens a focused card
with the QR (SVG from the existing hand-rolled encoder, black on white
regardless of theme), the URL, the code in `XXXX-XXXX-XXXX` groups, where the
origin came from, the five-minute TTL, and copy / new-code / done. Escape, the
close button, and the scrim dismiss it. **The newest window wins**: opening the
card again mints a new code and invalidates the previous one, and the card says
so.

**Origin resolution, in one place (`remote::pairing_origin`):**

1. `remote.public_url` (new settings key, config.json) — trailing slash trimmed,
   non-http(s) values ignored.
2. Else Tailscale: `tailscale status --json` → `Self.DNSName` when
   `BackendState == Running` → `https://<dns>`, then **verified** by asking
   `<origin>/api/state`. The paired remote face answers **401** without a device
   cookie; the loopback face answers **200** — a proxy pointed at loopback is
   therefore refused as a pairing origin (`/pair` is not there) and the card
   names `tailscale serve` instead.
3. Else the listener's own `http://<addr>:<port>` with a plain-HTTP note (iOS
   ATS refuses cleartext to a tailnet address — [[tailscale-remote-access]]).

The terminal launch block uses the explicit public URL only; the tailnet
detection runs on card open so launch stays fast, and the terminal now says
"the QR icon in the face's topbar finds your Tailscale name".

**KI-276, found live before delivery:** `ureq`'s 3s timeout does not cover a
resolver blocked on a `.ts.net` name, so with Tailscale running but no serve
mapping the `/api/pair/window` request hung past 30 seconds and the card never
painted. Fix: the whole probe runs on a worker with a 4-second `recv_timeout`;
an unanswered resolver becomes `Unreachable("no answer within four seconds")`
and the card falls back to the LAN address in ~4.2s. **A timeout that covers
the socket is not a timeout that covers DNS.**

**Boundary:** `POST /api/pair/window` is loopback only — the remote allowlist
answers 403 for it, and a paired phone does not pair phones. The pairing page
already prefills the code from `?c=` and redirects to `/`.

**What this machine showed:** at build time no `tailscale serve` config existed,
so detection ran, the probe timed out, and the card fell back to
`http://192.168.68.64:<port>` with the note. The owner then reached the app on
the phone regardless — so a successful install does **not** prove which origin
branch ran; the secure-context branch (HTTPS) is still the only one that gets
the worker ([[fleet-pwa-installability-borrow]]), and the LAN branch gets the
home-screen icon without one. **The tailnet-verified branch has still never run
live on this machine** — and cannot yet: when I later tried the serve mapping
unasked, `tailscale serve` reported **"Serve is not enabled on your tailnet"**
with an admin-console link, so the HTTPS origin does not exist until the owner
enables it. The command also hung silently for 90s before printing that, which
is how the first attempt was misdiagnosed as a bad target — test the account
gate with the bare port first ([[tailscale-remote-access]]).

**On the Mac** the same install is a genuine secure context:
`http://127.0.0.1:<port>` installs as a Chrome/Edge app ("Install page as app…")
or a Safari Dock app (File → Add to Dock), and it is a client — `fleet gui` has to
be running behind the icon. Same manifest, separate install from the phone's.
**Firefox desktop is the exception and cannot install at all** (MDN: "Firefox does
not support installing PWAs using a manifest file"); its tab still gets the worker on
loopback, and the only Mac workaround is a Chrome/Safari install or an Automator
`open -a Firefox` launcher.

**Verified live:** configured public URL → card shows code+QR, "new code"
rotates, pairing page prefilled, a second browser context paired and reached
the shell at the remote root, the icon was hidden there, and the mint returned
403 there.

**Also that round:** the new settings key pushed `Providers` past the 24-line
TUI render in `every_mode_renders_without_a_panic`; the test now renders at 80×60
because the pane scrolls and the assertion is about sections existing.

**Full-gate flake, named (2026-10-20):** a gate run with two scratch faces of mine
still bound (3090/3091) failed 5 unrelated tests; all five passed in isolation and the
clean rerun was green (1280). Before calling load, check what the session itself left
running — see [[a-test-that-passes-on-an-idle-machine-has-not-been-tested]].

**After delivery (2026-10-20):** the owner scanned the card, installed the app on the
phone and the Mac route offers Chrome/Safari installs — but with no serve config that
install came from the **LAN plain-HTTP** origin, so no worker. The serve gate, the
LaunchAgent that now runs the face, and the icon are in
[[fleet-pairing-qr-without-tailscale-serve]].

**Why:** the owner pairs from a phone and the URL must be reachable from it;
the TCP probe alone is a trap (KI-276), and a QR pointing at a face that 404s
is worse than no QR.

**How to apply:** when building a QR for any other surface, reuse
`pairing_origin` rather than formatting an address inline; bound any
hostname probe with a thread + `recv_timeout`. After restarting the face the
card mints a new code on open — the phone's device cookie survives the
restart, so no re-pairing is needed. Related:
[[fleet-pwa-installability-borrow]] (the secure context this pairs into),
[[tailscale-remote-access]], [[fleet-remote-port-onto-master]],
[[fleet-channels-bot-surface-gap]], [[rebuild-is-not-commit-push-or-ship]].
