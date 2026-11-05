---
name: tauri-native-shell-implemented-2026-11-02
description: "project 2026-11-02 — the Tauri native shell is IMPLEMENTED and the plan completed: a second crate (fleet-desktop) that starts the same server on an ephemeral port and opens a window on it; the package builds (fleet.app + 9.48 MiB dmg), the bundled services root resolves in the artifact, and the gate is green (2,002/0)"
type: project
date: 2026-11-05
basis: inferred
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

The owner said *"implement plan as designed"* for
`docs/plans/2026-11-02-tauri-native-shell.md`. **It is built, the plan is
`completed` with all ten criteria ticked with evidence, and the gate is
green.** Committed as `6b26a1e` (and its predecessor) on `face-plugin-host`.

## What exists now

- **`src-tauri/` — a second crate, `fleet-desktop`** (Tauri 2.12.5,
  `tauri-build`, `tokio`, `url`; depends on the `fleet` crate by path).
  `Cargo.toml` gained `[workspace] members = ["src-tauri"]` and
  **`default-members = [".", "src-tauri"]`** — that second line is what
  makes the gate's root commands (`cargo clippy --all-targets`, `cargo test`)
  cover the shell too.
- **The shell is thin by construction**: it calls `fleet::gui::serve` on an
  ephemeral loopback port and opens a `WebviewWindow` on that origin. **No
  frontend file changed**, no `fetch`→`invoke` rewrite, no remote-face change.
- **The inventory's three hard rows, closed:**
  - *Services root* — `bundled_services_root_at(executable)` walks
    `Contents/MacOS/<exe>` → `../Resources/services` and sets
    `FLEET_SERVICES_ROOT`. Neither of the crate's own fallbacks can resolve
    inside a bundle (the walk needs a `Cargo.toml` sibling; the other is the
    compile-time manifest dir).
  - *Lane wiring* — `lane_environment(home)` sets `SEM1_ST_VENV`,
    `SDM1_MODELS_VENV`, `GEN1_KOKORO_PYTHON` (root vs python per
    `EnvTarget`), and `write_sys1_config` writes sys1's
    `~/.config/sys1/config.toml` key — **never overwriting an existing file**.
  - *First run* — `first_run` creates the layout + scaffold (idempotent),
    sets the wiring, and spawns the venvs in the background (receipts make a
    second run read Present; a lane that cannot build is skipped by name).
- **The package**: `cargo tauri build` → `fleet.app` +
  `fleet_0.3.0-beta_aarch64.dmg` (**9.48 MiB**), ad-hoc signed, no notarization.
  The dmg carries `Contents/Resources/services/` with all five cores plus
  `documents` and `browser`.
- **Gate**: fmt 0 · clippy `-D warnings` 0 · release build 0 ·
  **2,002 tests / 0 failed**. `plan_check.py` clean; ledger 404 rows / 13 open.

## Proven live (not just compiled)

- The dev shell on a scratch home: the first run created the layout and
  scaffold, set the wiring, and the face answered at `127.0.0.1:59385` with
  `/api/state` and `/sw.js` 200 — **no browser opened**.
- **The packaged app from the mounted dmg**: it printed
  `services bundled at …/Contents/Resources/services`, resolved the bundled
  cores, and served the face on an ephemeral port. That is the inventory's
  hardest row proven *in the artifact*, not in dev.

## The owner's halves — both closed, and the window cannot be automated

**C3's registration half, proven on disk in the packaged app** (not asserted):
`~/Library/WebKit/dev.fleet.shell/WebsiteData/Default/` holds the fleet shell
worker in `ServiceWorkers/Scripts` (5,863 bytes, the run's timestamp),
`CacheStorage/cacheslist` names `fleet-shell-b3c6f48a6723c87e`, and the cache
records reference `/app.css` and `/app.js`. On a **second launch** the
registration database was written again — 22:33 against the 21:58 cache — so
the page re-registered and resolved to the existing cache.

**C9's turn half**: `POST /api/run` — the identical request the window's send
makes — drove a real turn in the packaged app. The session log records
`stop_reason: handed back`, `turns: 1`, `served_model: LongCat-2.5`, and the
assistant's answer. Committed `4cd8702`.

**KI-406 — the native window cannot be driven by any tooling.** A Tauri
WKWebView exposes **no accessibility tree for its web content**: 862 elements,
748 of them `menuitem`, the rest `AXApplication`; `perform_action` reported
`allowed: []`; `getScreenshot` returned *"Screenshot unavailable … hidden or
unrendered window"*. So the face harness (a real browser) and Computer Use are
both blind to it, and the turn had to go through the app's own HTTP API. The
row records the two ways to give it a real test seam: accept the API as the
shell's test surface and say so, or give the bundle a remote-debugging path
(Tauri `devtools`, or WKWebView `isInspectable`). Until one exists, a
regression in the window's own rendering has no automated witness.

## Where the installable is

`target/release/bundle/dmg/fleet_0.3.0-beta_aarch64.dmg` (9.41 MiB at the
2026-11-03 rebuild), rebuilt by
`cd src-tauri && cargo tauri build`. Ad-hoc signed: first launch needs a
right-click → Open (the owner's non-commercial ruling — no notarization).
Bundle id `dev.fleet.shell`. macOS-only for now.

## The shell embeds the face (2026-11-03)

The Tauri app starts the face **in-process** (`fleet::gui::serve` inside
the shell's own binary) and the assets are `include_str!`'d into the lib —
so **a face change ships to the Tauri window only through a `cargo tauri
build`**; rebuilding and restarting the `:3080` LaunchAgent face does not
touch it. The proof is
`strings target/release/fleet-desktop | grep -c <new-symbol>` (the plugin
host's `pluginBuiltIns` embedded = 2 after the 2026-11-03 rebuild) — the
served-bytes discipline applied to an embedded binary. The owner supplied
the command after cancelling a plain `cargo build --release` (see
[[rebuild-is-not-commit-push-or-ship]], the seventh ask).
`/Applications/fleet.app` is the installed copy, stamped by that build.

## Tauri build facts worth keeping

- **`bundle.resources` is a map of source → destination**
  (`{"../services": "services"}`), **not** destination → source. Getting it
  inverted yields `resource path 'services' doesn't exist`.
- **The icon must be RGBA.** Tauri's `generate_context!` panics with
  `icon … is not RGBA` on a plain RGB PNG. The repo's `icon-512.png` is RGB;
  `magick src/gui/assets/icon-512.png -resize 1024x1024 -alpha set -channel A
  -evaluate set 100% +channel src-tauri/icons/icon.png` fixes it. A PNG is
  required alongside the `.icns`.
- **`src-tauri/gen/` is generated** by `tauri-build` (the capability schemas
  the `$schema` keys point at) — gitignored, never hand-edited.
- **`bootstrap_lane`'s progress sink is `&mut (dyn Write + Send)`** — a plain
  `dyn Write` parameter does not satisfy it.
- **`cargo install tauri-cli --version "^2.12" --locked`** (2.12.4) is the
  whole Node-free toolchain; no npm anywhere.

Related: [[tauri-native-shell-research-2026-11-02]] (the research and the
plan), [[a-native-install-must-carry-every-component]] (the requirement),
[[self-containment-is-a-standing-contract]] (hard rule 12),
[[a-new-plan-must-answer-the-past-plans]], [[rust-1991-toolchain-move]].
