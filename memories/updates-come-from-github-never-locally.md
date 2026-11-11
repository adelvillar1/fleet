---
name: updates-come-from-github-never-locally
description: "Owner ruling 2026-11-09 — an update is fetched from GitHub, never from the local machine"
type: feedback
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

The owner's ruling (2026-11-09, during the updater deep dive): *"the update
should be fetched from github, never from the local machine."*

Operationally: the feed and the artifact come from
`github.com/adelvillar1/fleet/releases/...` and nowhere else — no local path, no
local HTTP server, no LAN peer, no config/env override that repoints the
source. The machine is the install target and the press surface only. The
transport must stay HTTPS (a plain-http endpoint is a local-source hole).

**Why:** the verification an update rests on — the shell's minisign signature
against the pinned key, the CLI's sha256 against the feed's cli row — is only
as good as the channel that delivered it. A locally-supplied feed would be
verified bytes from an untrusted source.

**How to apply:** keep the source pinned (the `FEED` constant and the plugin's
`endpoints`); refuse any design that adds a local or configurable update
source; `dangerousInsecureTransportProtocol` is incompatible with the rule and
must go (KI-489). The ruling does not decide who may press — a loopback page
press still fetches from GitHub, so the ACL/origin question ([[tauri-updater-deep-dive-2026-11-09]])
stays a surface question, separate from provenance.
