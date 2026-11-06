---
name: the-relay-session-is-durable-ki-370
description: "project 2026-10-30 (KI-370, 48b3805) — the relay session/secret was minted per Listener::bind, so EVERY face restart stranded every paired phone (all /api/ 403 at the gate, cards render empty, which reads as \"the option isn't there\"); it now lives in <FLEET_HOME>/relay-sessions.json, one entry per face, 0600, atomic; phones paired before the fix need one re-pair"
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

2026-10-30, fixed and verified (`48b3805`, KI-370 — KI-369 was
taken by a 00:55Z scheduled corpus run, see
[[a-plans-ki-number-can-be-overtaken]]). `Listener::bind` minted a
fresh session id and secret at **every launch**
(`mint_session`/`mint_secret`). The phone stores the secret it paired
with; `relay.py`'s `_root` routes headered paths by the presented
secret (`matches = [d for d in devices if d.secret == presented]`). So
after any restart the phone presented a secret no live device held:
every `/api/` request answered 403 at the relay gate, the reads failed,
and a face whose reads fail renders empty cards — **the bots page then
shows no paired-devices card at all, which reads to the owner as
"nothing about pairing anywhere."**

**Why it matters more than it looks:** the face was restarted eleven
times in one evening by the session itself, so the owner's PWA was
stranded at the API level regardless of which JS version the browser
ran — and the JS-version argument (stale tab, cache, worker) was the
wrong axis the whole time. A per-launch credential turns the harness's
own restart habit into a user-visible outage.

**The fix:** `face_credentials(home, face)` reads/writes
`<home>/relay-sessions.json` — one entry per face (`console`, `bot`),
minted on first use, atomic temp+rename, mode 0600 (the own-home
ruling: the home is the credential home). An unreadable file is
replaced, not fatal. The device's X25519 scalar stays per launch and
the phone re-handshakes on every page boot; only the header-gate
credential is durable. Pinned by
`a_face_restart_keeps_the_relay_session` (two binds over one home
answer the same session; the two faces hold distinct ones), and
verified live: two restarts, byte-identical secrets in both launch
prints, file `-rw-------`.

**How to apply:** any credential a client stores and a server
compares must be a HOME FACT, not a process fact — a "secret" minted in
`bind()` silently expires on every deploy of the thing it protects.
When a paired client reports every read failing, check whether the
server's credential changed under it before blaming the client. One
consequence still standing for the owner: pairings made before this
fix need **one** re-pair per origin, after which restarts are
invisible — see [[pairing-at-an-origin-arms-the-transport]], whose
"re-pair on every restart" paragraph this supersedes.
