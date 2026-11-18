---
name: the-browser-engine-plan-is-complete
description: "The browser engine work is complete: obscura is the engine, its absence produces a self-healing install card, Chrome is gated behind a Settings choice, the release is checked and cached, and the lane's tools are core — kept whole by the budget, never swept."
type: change
date: 2026-11-18
basis: confirmed-by
tier: durable
shape: [deps, state-sync, resolved]
phrases: [browser.engine, engine::install, install obscura, core_lane_tool, engine-release.json, BROWSER_OBSCURA]
missing: []
premises: []
recall-count: 0
superseded-by:
valid-until:
source:
---

The browser engine plan is COMPLETE — `docs/plans/2026-11-18-obscura-is-the-browser-engine.md`, status `completed`, all 9 criteria ticked, verified live.

WHAT LANDED beyond the lane rewrite (already recorded in `the-browse-lane-runs-on-obscura`):

1. **`browser.engine` is a declared setting** — `settings::SETTINGS` gains `Kind::Choice(&["obscura", "chrome"])` on `File::Config`, so the Settings surface carries the row and `fleet settings set` validates it; `chromium` is refused by name. New `settings::read_value(home, key)` is the public dotted reader (the existing `lookup` is private, and re-spelling it would drift). Unset reads as absent and the lane's `engine_preference()` answers `obscura`.

2. **`src/engine.rs`** — the module the check and the press share: `find()` (obscura via BROWSER_OBSCURA → `<home>/bin` → /usr/local/bin → /opt/homebrew/bin → PATH; Chrome probed only to be REPORTED), `obscura_version()`, `compare()` as the pure three-word answer (current/behind/ahead + unknown), `ReleaseState::sentence()`, `asset_for(platform, arch)` matching the release's own naming, `latest_release()` with a 6s timeout, 512 KiB ceiling and a 24h cache in `<home>/engine-release.json`, and `install()` which downloads the release, unpacks BOTH `obscura` and `obscura-worker` into `<home>/bin`, chmods them, and answers the version.

3. **The doctor check `browser`** — verified live in both states: obscura present → `ok` with "obscura 0.2.7 is installed — obscura 0.2.7 is the latest release"; obscura absent and Chrome present → `ok: false` with "obscura is not installed, so the browser would fall back to Chrome — install obscura to avoid it" carrying `{"label": "install obscura", "action": "/api/repair/browser"}` through the same `repair_for` registry every other failing check uses.

4. **`POST /api/repair/browser`** → `Bridge::repair_browser` → `engine::install`, answering the fresh engine document so the card re-reads rather than asserting success.

5. **C9, the budget floor** — `tier_declarations` now treats a core lane's tools as a GROUP: `core_lane_tool`/`core_lane_of` name it (`mcp__browser__…`), the deferrable lanes are dropped first, and the group is kept whole or refused by name, never halved. The sweep half was already true (`is_protected` covers every `mcp__` name and `browse`) and is now PINNED.

A BUG THE LIVE PROBE FOUND: `obscura_path` returned early on `BROWSER_OBSCURA` only when the path was executable, and otherwise FELL THROUGH to the hardcoded prefixes — so an explicit override pointing nowhere still resolved `/usr/local/bin/obscura`, which made the "absent" branch unreachable and untestable. An explicit override that does not resolve is a REFUSAL now, not a licence to probe elsewhere.

Gate: **2373 Rust tests** (13 new across engine/core_lane/settings), **1457 face checks across 204 cases**, fmt and clippy clean, `plan_check.py` green.
