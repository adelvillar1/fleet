---
name: the-served-page-loads-a-cached-bundle
description: "feedback 2026-11-09 — a live read of the served face can show the OLD app.js after a rebuild (the shell's service worker and the browser cache hold the previous build); clear the SW registrations + CDP cache or use a fresh profile, and re-read the script's own src before diagnosing a live layout as a code defect"
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

2026-11-09, twice in one afternoon: after `cargo build --release` and a restart
on 3099, the served desk still painted the OLD layout (the removed `.pw-rail`,
no work column, the composer in the panel column). The code on disk and
`curl /app.js` were right; the page was running a cached bundle — the shell's
service worker (`sw.js` caches `/app.js?v=` and `/app.css?v=` per build hash)
and Chrome's own cache.

**How to read a live surface honestly:**
- `navigator.serviceWorker.getRegistrations()` → unregister each, delete the
  `fleet-shell-*` caches, and set CDP `Network.setCacheDisabled`;
  `Page.reload({ignoreCache: true})` — or just use a fresh `--user-data-dir`.
- Read `document.scripts[…].src` and check the loaded text for a marker of the
  change before trusting any DOM read.
- A DOM probe's answer about a *removed* element (`pw-rail` still present) is
  the tell; suspect the bundle before re-reading the code.

Related: [[measuring-a-mockup-by-reading-the-png]],
[[the-served-face-is-the-installed-copy]],
[[verify-the-binary-is-newer-than-the-source]].
