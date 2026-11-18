---
name: face-iteration-without-a-rust-build
description: "Rapid face iteration without a Rust build: debug reads assets from disk, FLEET_ASSETS_FROM lets the installed binary serve the working tree, and the dev service worker self-unregisters so save-and-refresh actually works."
type: project
date: 2026-11-17
basis: confirmed-by
tier: durable
shape: [build, change, resolved]
phrases: [10+ of rust building for just a tiny change, FLEET_ASSETS_FROM, scripts/face-dev.sh live, FACE_ONLY, include_str! embeds the assets so a JS edit is a rust edit, save and refresh]
missing: []
premises: []
recall-count: 1
superseded-by:
valid-until:
source:
---

The owner's 2026-11-17 throughput complaint, and the structural answer built for it.

THE COMPLAINT, measured: "an icon change or some minute issue takes half an hour or more… we went from making significant changes in hours to days, and now we have only managed to make a very few improvements and fixes today… the methodology is not adaptable and risk based." And specifically: "it's not 10 or 44 seconds, it's 10+ of rust building for just a tiny change."

THE COSTS measured that day:
- node tests/face/harness.mjs: 17s for 197 cases / 1408 checks
- same narrowed with FACE_ONLY: 0.2s
- cargo build -p fleet after a real app.js edit: 10s
- cargo build (workspace): 44s (src-tauri is 34s of it, never needed for face work)
- release.sh assets: ~7 min (release build 4m06 + dmg + sign + upload + feed read-back)
- release.sh tag: ~4 min (cold gate lane). Total ritual per shipped change: 12-15 min.

ROOT CAUSE of the Rust compile on a JS edit: app.js/app.css/index.html are embedded with include_str! in src/gui/server.rs, so touching them makes cargo recompile and relink the crate.

THE FIXES:
1. src/gui/server.rs `served()`: a DEBUG build reads the asset from disk on every request (CARGO_MANIFEST_DIR-joined), and answers no-cache rather than immutable, so save-and-refresh works with no build.
2. `assets_from()` honours FLEET_ASSETS_FROM=<tree> in RELEASE builds too — so the INSTALLED binary can serve the working tree. Verified live: an edit to app.js appeared in the running release face with zero build, and reverting needed no rebuild either.
3. `worker()`: debug or override mode serves a self-unregistering stub with no-store, because the real worker caches /app.js?v=<hash> eagerly and would pin a stale script (isVersionedAsset + fromCacheOrNetwork). Without this the whole loop silently no-ops.
4. tests/face/harness.mjs: FACE_ONLY=<substr> narrows cases, prints NARROWED loudly, and is never set by the gate — so the gate is unchanged.
5. scripts/face-dev.sh: build | check [pat] | loop | live | stop | ship <tag>. `live` is the ruling's verb: installed binary + this tree, no build, kill to return to the shipped face.

The trap worth remembering: a change to HOW assets are served (server.rs) is not a presentation change — it reaches every client including remote and bot faces. The icon's path data was presentation; dev serving was not.
