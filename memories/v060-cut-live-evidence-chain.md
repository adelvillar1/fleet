---
name: v060-cut-live-evidence-chain
description: "The v0.6.0 cut's live-proven release path: the feed must upload as latest.json (scratch names 404), signatures verify via minisign -V on the base64-decoded sig, release.sh's four structural fixes are in-tree, and the tag↔HEAD guard forces every fix into the tree before cutting"
type: project
date: 2026-11-08
basis: inferred
tier: durable
shape: [change, resolved]
phrases: [scripts/release.sh assets v0.6.0, releases/latest/download/latest.json, fleet-latest.json 404, minisign -V, plutil CFBundleShortVersionString, HEAD is not v0.6.0]
missing: []
premises: []
recall-count: 0
superseded-by:
valid-until:
source:
---

The v0.6.0 cut (tag 87090ae) proved the release path with verified leg-reads:

1. FEED NAME: `gh release upload $TAG /tmp/fleet-latest.json` kept the scratch basename — the asset landed as fleet-latest.json and `releases/latest/download/latest.json` 404'd until re-uploaded as a file literally named latest.json (--clobber). The script's upload line still says "$LATEST" and will re-introduce this on every future cut. TOP OPEN RISK for every future release.
2. CRYPTO: the conf's pinned pubkey decodes full-blob-equal to /tmp/fleet-updater-key.pub's payload — the keyfile is the true half. Verification = `minisign -V -x <decoded.minisig> -p <decoded.pub> -m <tarball>` (brew minisign 0.12, exit 0) after base64-decoding cargo-tauri's base64 wrapper; the "sig id == pub id" PAIR read was NOT verification — ad-hoc python offset-verify returns false InvalidSignature on a correct pair.
3. THE GATE'S FOUR DEFECTS THE FIRST CUT SURFACED (all fixed in-tree by 8ed887f): nohup'd runs need release.sh to export the toolchain PATH itself (exit 127 on both cargo legs); the strings version probe loses to const-merging (the bundled Info.plist read by plutil is the strong leg); assets mode must discover+sign .app.tar.gz BEFORE the upload (unbound APP_TAR under set -u); latest.json's cli.signature must hash the CLI tarball itself, not the app tarball.
4. THE FACE HARNESS IS FULL-HARNESS-OR-NOTHING: the About pane shipped with four latent reds because only scoped suite runs had run since it landed — about icon undrawn, 18→19 destinations, seven→eight sections, palette labelled a BUILT surface 'later'. All re-pinned; `node tests/face/harness.mjs` answers `face: 1009 checks passed across 117 cases`.
5. TAG↔HEAD GUARD'S RHYTHM: assets mode refuses when HEAD moves past the tag, so a post-tag fix is one delete-release + re-tag + re-assets cycle — cut only from a tree carrying every fix, not by successive discovery.

Structural: nohup'd cut scripts survive an owner stop of a polling bash.
