---
name: updater-that-installs-plan-drafted-2026-11-09
description: "project 2026-11-09 — SUPERSEDED: the plan that was drafted here was IMPLEMENTED and COMPLETED the same day (0.6.5 + 0.6.6); see [[updater-fix-landed-2026-11-09]] — this file survives as the drafting record and its two open rulings"
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

**STATUS: the plan was approved, implemented and completed the same day — see
[[updater-fix-landed-2026-11-09]]. Everything below is the drafting record.**

The owner's ask: *"let's create a plan to fix the updater, because right now
it's doing nothing. I tried to update a couple of times, both times it told me
that i needed to restart the app and nothing happened."*

The plan answers [[tauri-updater-deep-dive-2026-11-09]] and closes KI-486/487/
488/489. Shape, in one breath: the release ships the CLI's update artifact as a
**raw binary asset** (what the original plan always said) and the feed's `cli`
row names it; the client gains an **exec-and-version probe before the rename**
(the staged file must answer `--version` with the feed's version — this is what
would have caught the tarball overwrite), keeps the old binary as `<exe>.prev`,
refuses a bundled executable by name, and takes its feed through an **injected
seam** so the install leg has a network-free test; the shell grants the
loopback origin as a `remote` context plus `restart_app` and **binds once,
handing the listener to the server** (closing the port race keeps the grant
narrow); the About pane gains a pre-check "not checked yet" state and a
failed-check paint; `dangerousInsecureTransportProtocol` goes, enforcing the
[[updates-come-from-github-never-locally]] ruling.

**Why:** the deep dive proved neither channel installs — the CLI writes a gzip
tarball over the binary and the shell is ACL-refused before the network — so
"improving" the updater without these is polish on a dead path.

**How to apply:** C8 is the honest constraint — 0.6.4's compiled-in ACL means
the button cannot fix itself, so the live proof is two cuts (0.6.5 hand-
installed, 0.6.6 installed by pressing About). Two decisions the owner may
still overrule: raw asset over unpacking in the client (costs 26.7 MB
transferred instead of 10), and the loopback grant over a fixed shell port
(which would let the capability name one exact origin).

Gate state: `plan-gate` and `plan-surface` did not run — `no provider answered
(chain: jev)`; `arch-gate` PASS with 25 UNDOCUMENTED conventions gaps, all the
missing updater section in the tech doc, which the plan lists as a linked
artifact. Say so when presenting; do not imply the gates ran.
