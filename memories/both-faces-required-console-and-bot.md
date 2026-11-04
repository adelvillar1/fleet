---
name: both-faces-required-console-and-bot
description: "Owner ruling 2026-10-29 — the standard work-centric console experience AND the assistant-first bot experience must both be available; LANDED the same day — the console at fleet.opsh.io and the bot at fleetbot.opsh.io (a second relay service, --bot-relay, same seal, pairing per origin)"
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

The owner's ruling (2026-10-29, after the remote-relay work landed):
fleet must make **both** experiences available — the standard fleet
work-centric experience (the console face) and the assistant (bot)
based experience (the persona-first PWA) — neither is the product
alone. This extends the 2026-10-28 two-windows ruling ([[bot-pwa-is-assistant-centric-not-work-centric]])
into the remote path built this day ([[both-faces-required-console-and-bot]]
covers the relay origin).

**State (updated 2026-10-29, same day — the follow-up plan landed):
both faces are on the relay.** The console at `https://fleet.opsh.io`
and the bot at `https://fleetbot.opsh.io` — a second OpenShip service
of the same two files, dialed by `fleet gui --bot-relay`, the same
X25519-then-AES-GCM seal, pairing at each origin separately (two
domains cannot share a cookie; that is the two-installs rule carried
honestly). The live probe paired at the bot origin and opened a sealed
conversation creation from ciphertext. Plan:
`docs/plans/2026-10-29-bot-face-on-the-relay.md` (completed). The bot
face still listens on `:3081` for the LAN.

**Why:** the remote rearchitecture was scoped to "the assistant
experience," but the relay's implementation carries the console face —
the owner is naming the remainder before it becomes a surprise: the bot
experience must reach the phone the same way.

**How to apply (superseded — both parts landed 2026-10-29/30):** the
bot face got its own relayed origin by the second plan
([[bots-surface-mints-the-bot-pairing-card]] for the card, plan
`2026-10-29-bot-face-on-the-relay.md` completed — second service, not
face-aware routing), and both faces' installs are separate PWAs per the
D3 rule. The open remainder after the owner's "I don't see it anywhere"
report (2026-10-30): the **pairing pattern's reach** — the #/bots card
mirrors the topbar's loopback-only gate, so it does not exist on the
LAN or relay faces; the owner decides whether opening the mint there is
worth the security edge (a paired remote browser minting codes), with
the home-network-only compromise named.
