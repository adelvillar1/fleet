---
name: pairing-at-an-origin-arms-the-transport
description: "feedback 2026-10-29 (KI-368) — a phone that opens a relayed origin without the entry URL's fragment and pairs by typing a code paired UNSEALED (console: "stream down", bot: 403); the transport's arming path can never depend on a route the user happens to take, so the relay-marked redemption now hands the phone the secret itself"
type: feedback
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

The owner's first live use of both origins (2026-10-29 evening) found
one hole behind two symptoms: the console's top bar read **"stream
down"** and the bot showed a **403**. The transport armed itself only
from the entry URL's fragment (`#k=…`), which a phone carries only if
it scanned the launch print or the QR card — a phone that typed
`fleet.opsh.io`, opened the pairing page, and redeemed a code got a
working cookie and NO relay secret, so every fetch hit the gate
headerless (403), the console's `EventSource` was refused (the pill
read "stream down"), and the bot's inbox never loaded. The pairing page
could not help: the secret never crossed the wire to it.

**Why:** a capability whose one delivery route is a *scan* silently
excludes every other arrival path, and the failure lands far from the
cause (a stream pill, a bare 403 — nowhere near the pairing step).

**How to apply:** when a page needs a credential to function, arm it at
the last common point of EVERY path (the redemption), not at the
optional one. The fix (commit `efd067f`, KI-368 closed): relay.py
stamps forwarded frames `x-fleet-via: relay`, the redemption response
carries the face's relay secret **only for a relay-forwarded pairing**
(a secret the relay already holds at join — nothing crosses that had
not already crossed), the socket path passes `None` regardless of
headers so a hand-written marker gets nothing (pinned by
`a_relayed_redemption_carries_the_secret_and_a_plain_one_does_not`),
and the pairing page files it into `localStorage["fleet_relay_k"]`
before the redirect. Pairing at an origin now arms the transport
whatever route the phone took. Guard the marker at the boundary it
can't be forged on — frames are made by relay.py only, sockets never.
**The owner's pairings predating the fix need one re-pair per origin**
(their localStorage holds no secret); the entry URLs with fresh
fragments live in `~/Library/Logs/fleet-gui.log`.

**The restart-mints-a-new-secret trap is FIXED (2026-10-30, KI-370,
`48b3805`)** — the relay session/secret used to be minted per
`Listener::bind`, so every face restart stranded every paired phone's
stored secret: all `/api/` reads 403'd at the gate and the face
rendered empty cards, which is the owner's exact "nothing on the bots
page says anything about pairing." It now lives in
`<FLEET_HOME>/relay-sessions.json` (per face, 0600, atomic); the DH
re-handshake still runs per page boot. Full mechanism, fix and the
one-time re-pair consequence: [[the-relay-session-is-durable-ki-370]].
Keep the diagnostic reflex anyway — for a "the PWA shows nothing"
report, ask which origin it runs and when the face last restarted
before arguing about JS versions.

**2026-10-30 — pairing is not the finish line (KI-371, `950f52d`).**
The owner paired through the QR and got "a 400 error after pairing";
behind it were three sealed-frame defects (a bodyless GET marked
sealed, the base64-TEXT sealed body read as raw bytes, an empty
handshake catch hiding the `OperationError`) and a 65 s mid-restart
hang the relay now answers as an instant named 503. So when a pairing
report is a FAILURE, do not stop at "does the cookie land" — the next
question is whether a sealed READ opens. Full mechanism and the
fingerprint-vs-framing diagnostic: [[ki-371-the-sealed-frame-boundary]].

**2026-10-30: a fresh code is also a precondition, for a reason that
faults the session and not the owner.** The owner then reported "local
url for the bot ui doesn't seem to be working" — and the local flow was
healthy under proof (fresh mint → redeem at `http://192.168.68.70:3081`
→ 200 paired → inbox 200 with the real sessions → page 200). The cause
was a **burned code: every `POST /api/pair/window` mint invalidates the
previous code** ("the newest code wins"), and my own verification runs
minted four or five times inside minutes while the owner was likely
holding one. The follow-up mint handed them `A49G-BF6H-D8KC` (codes
carry a 5-minute TTL). **How to apply:** a pairing window is a shared
single-use resource — do not mint (on the card, on the console
`/api/pair/window`, or in a probe) while the owner may be pairing, and
when a pairing report arrives, first check how many mints ran since
they could have looked at one. When they must re-pair (as with the
pre-KI-368 pairings), mint ONCE, hand them the code outright, and say
the TTL — don't leave them racing an invisible window. The LAN URL also
requires the same Wi-Fi (cellular reaches only the two relay origins),
which should be restated whenever a LAN access question arrives,
because the answer now differs by network.

**2026-10-30, the closing lesson of the chain: each symptom was a
DIFFERENT layer, and only a browser repro in the owner's own origin
separated them.** Burned code (session) → in-flight pairing (arming) →
400 on a bodyless GET (seal flag) → blank face with no failed request
(base64 framing) → empty cards (per-launch secret) → a list full of the
machine's own cron runs (class predicate). Six reports, six layers, and
in every case the owner's sentence was concrete ("400 after pairing",
"a long list of things that say needs my attention") while my first
hypothesis was one layer off. When a chain of reports keeps arriving,
assume the earlier fix exposed the next layer rather than that the
reporter is confused — reproduce the newest report end to end before
touching anything.
