---
status: draft
created: 2026-11-02
updated: 2026-11-02
slug: tauri-native-shell
---

# Plan: the native shell — Fleet in a Tauri window, self-contained, from GitHub

## Context

Fleet's face is a PWA. `fleet gui` starts a Rust HTTP server on
`127.0.0.1:3080` (`src/gui/mod.rs:81,86`), serves a vanilla SPA (950 KB
`app.js`, no framework, no build step), and registers a manifest and a
service worker. The same server also serves **remote peers** —
`fleet gui --remote` is a first-class mode with pairing and a cookie
boundary by path (`src/gui/mod.rs:142-225`).

The owner wants a *proper native app*, distributed as installable
packages on GitHub, non-commercial. And the requirement that shapes this
plan: **every Fleet component is accounted for in the install — no loose
ends.**

**The history, corrected.** The PWA was not a permanent ruling against a
shell. `2026-10-20-pwa-shell.md` refused an Android WebView shell so the
team could **iterate fast on a working app** — "a Kotlin/Gradle tree and a
second toolchain for a crate whose contract is one static binary, while
the PWA covers the home-screen need." That was a rapid-iteration choice,
and it worked: the face shipped and kept moving. The native installable
shell is the **next step in Fleet's evolution**, not a reversal.

**The founding intent.** Self-containment was there from the beginning:
*"Fleet is a native Rust application, not a wrapper around the Node/TS
harness: no embedded Node runtime, no staged harness payload, no
checkout dependency"* (`2026-10-12-fleet-native-app.md`). The crate is
"one static binary with its own `ureq`-based client… No Node runtime, no
harness checkout, no dsh launch path" (`CLAUDE.md:7`). The native shell
must **honor that intent, not erode it**: everything Fleet is and does
ships, builds, or is fetched by the app itself — never by the owner's
shell profile, a checkout, or a foreign runtime.

**The toolchain is no longer a blocker.** Rust moved to **1.99.1** on
2026-11-02, and Tauri 2.12's MSRV is 1.90 — satisfied.

## Approach

**A thin shell: keep the HTTP server, and add a Tauri window that loads
the existing loopback origin.**

The frontend's 51 `fetch` calls are all relative (`/api/...`) and its
stream is a relative `EventSource('/events')`, so a WebView pointed at
`http://127.0.0.1:<port>` needs **zero frontend changes**. The remote
face is why the server cannot be removed: a desktop app that spoke IPC
while browsers spoke HTTP would maintain two transports for one UI — the
exact cost the embedded-axum developer cited for keeping his server.

This is also the house pattern. When the owner asked for design-canvas
"as a fully native part of the app… not a mere bolt-on"
(`2026-10-31-design-canvas-native.md`), the answer was a sandboxed
iframe inside the existing face on the existing server. The plugin host
did the same for bundles: served by the bridge, mounted as rail
surfaces. The native shell is that pattern at the top of the stack.

The increments, each independently valuable:

1. **The shell builds.** A Tauri crate with `tauri.conf.json`, an
   identifier, and a window pointed at the loopback origin. The gate
   covers it.
2. **The lifecycle belongs to the window.** Start the same server
   `fleet gui` uses, on an ephemeral port (or the default with a named
   conflict), suppress the browser auto-open, stop it on window close.
3. **The first-run install is self-contained.** A clean machine launches
   the app and gets the whole of Fleet: the home layout, the config
   scaffold, the lane venvs, the lane wiring, the services root, and the
   doctor gate — with no shell profile, no checkout, and no Node.
4. **The face is unchanged.** No `fetch` → `invoke` rewrite. The same
   frontend serves the browser, the remote peers, and the native window.
5. **The package.** `cargo tauri build` → `.app` + `.dmg`, ad-hoc
   signed, uploaded to a GitHub release by hand.
6. **Native increments, each its own commit.** Tray icon, menu bar,
   notifications, filesystem/dialog plugins — only as the owner wants
   them.

The docs move with the increments: `CLAUDE.md`'s command list and branch
topology, and a docs section for the shell, land with increments 2 and 5
respectively — not deferred to a finale.

## Precedents this plan answers

| Precedent | What it ruled | What this plan does |
|---|---|---|
| `2026-10-12-fleet-native-app.md` | "a native UI surface" is a later slice; no Node, no checkout, no wrapper around the harness | Builds that slice; the shell wraps Fleet's **own** server, not the Node harness |
| `2026-10-20-pwa-shell.md:66-68` | Refused an Android WebView shell — "a second toolchain for a crate whose contract is one static binary" — for rapid iteration, because the PWA covered mobile | Reopens it for **desktop**, where the PWA covers nothing; the "second toolchain" is Rust, and the iteration argument is spent now that the face is stable |
| `2026-10-31-design-canvas-native.md` | Web content that must feel native → a sandboxed iframe in the existing face, on the existing server | The same shape one level up: the face itself, in a native window, on the same server |
| `2026-11-02-face-plugin-host.md` | Bundles served by the bridge, mounted as rail surfaces; the trust boundary is a frame | Unchanged — the shell adds no plugin surface of its own |
| `docs/features/install.md:233` | "The release path is `cargo build --release` — one static binary"; per-target cross-builds are a release-process concern | The static binary **stays**; the native package is an additional artifact, and the cross-build seam in `install.md` is where its commands belong |

## Component inventory — nothing left out

Every Fleet component, where it lives today, and how the native install
carries it. **A component with no home is a failed criterion, not a
note.**

| # | Component | Where it lives today | How the install carries it |
|---|---|---|---|
| 1 | The `fleet` binary | `target/release/fleet` | bundled at `Contents/MacOS/fleet` |
| 2 | Face assets: `index.html`, `app.js`, `app.css`, `vendor/`, icons, `manifest.webmanifest`, `sw.js` | `include_str!` into the binary | travels with the binary — nothing to do |
| 3 | The five lane cores (`services/`) | `FLEET_SERVICES_ROOT`, else a 12-ancestor walk that requires a **`Cargo.toml` sibling** (`src/lanes.rs:985`), else the compile-time manifest dir (`:991`) | **the app sets `FLEET_SERVICES_ROOT`** to the bundled `Contents/Resources/services`, computed from `current_exe()` — neither fallback can resolve inside a bundle |
| 4 | The lane venvs (~1.6 GB of Python) | `<home>/venvs/<lane>/`, built by `fleet install --lanes` | built by the app's first-run install, reusing the existing receipt discipline |
| 5 | The lane wiring | `SEM1_ST_VENV`, `SDM1_MODELS_VENV`, `GEN1_KOKORO_PYTHON`; sys1's `~/.config/sys1/config.toml` `[providers] local_venv` | **the app sets them** — in the child environment at spawn, and writes sys1's key. Today they are *printed* for the owner's shell profile, which a dock-launched app does not have |
| 6 | blend1 | wired at spawn via `BLEND1_*_PYTHONPATH` (the code sets it) | already in-process — nothing to do |
| 7 | gen1's ONNX weights | `~/.cache/gen1/tts/kokoro-v1.0.onnx`, `voices-v1.0.bin` | machine-owner downloads; the app **names** them and never fetches |
| 8 | The embedding server | probed locally | doctor's recall line, unchanged |
| 9 | The home layout (`sessions`, `memories`, `skills`, `cron`, `proposals`, `config.json`, `.env`) | `<home>` (`FLEET_HOME`, else `~/.fleet`) | the app runs `fleet install` on first launch |
| 10 | The memories corpus | `<home>/memories` (home-first, `src/memory.rs:746`) | created by the install |
| 11 | Credentials | `DEEPSEEK_API_KEY` env → `<home>/.env` | owner-supplied; the app **never writes a key** — it names where one goes |
| 12 | The MCP registry | `mcp.json` scan | unchanged |
| 13 | Plugins | `<home>/plugins/`, proposals | unchanged |
| 14 | The consolidator + cron scheduler | spawned by the GUI bridge (`src/gui/bridge.rs:10062,10068`) | unchanged — the same bridge. **But they die with the app**: see C9 |
| 15 | The remote face / relay | `fleet gui --remote` | unchanged |
| 16 | Designs / canvas digests | `<home>` | unchanged |

**The loose ends this closes** — each is a place the PWA era leaned on
something outside the app, and the native install must not:

1. **The services root.** The walk requires a `Cargo.toml` beside
   `services/` — a source checkout — and its fallback is the compile-time
   manifest dir (`src/lanes.rs:985,991`). Neither exists in a bundle, so
   the app must set `FLEET_SERVICES_ROOT` to the bundled
   `Contents/Resources/services`.
2. **The lane wiring.** Today `fleet install` prints
   `export SEM1_ST_VENV=…` and a sys1 config note for the owner to put
   in a shell profile. A dock-launched app sources no profile, so the
   app must set the three variables in the child environment and write
   sys1's key itself.
3. **The venvs.** 1.6 GB, built by `uv`/`python3`. The first-run install
   builds them, degrades by name, and the doctor's lane lines say which
   are live.
4. **gen1's weights.** Named, never fetched — a first run without them
   reports the lane dead by name, not broken.
5. **Credentials.** The app cannot supply a key; the first run names
   where one goes and the doctor's credentials line says whether one
   resolved.
6. **The daemon lifetime.** Cron and consolidation live in the GUI
   process. If the owner needs scheduled jobs to outlive the app, that
   is a separate increment (a LaunchAgent — the repo already runs
   `local.fleet-maintenance` that way), not a silent gap.

## Use cases

- **The owner, daily.** A dock icon and a real window: no browser tab,
  no address bar, the face one click from the dock.
- **A clean machine.** Install the `.app`, launch it, and Fleet is
  whole: the layout, the venvs, the wiring, the lanes, the face. No
  terminal, no profile edit, no checkout.
- **A peer, casually.** Downloads the `.dmg` from a GitHub release,
  moves the app to Applications, right-clicks → Open the first time.
- **A remote peer, unchanged.** Reaches the face over the network from a
  browser; the native shell is a third client, not a replacement.

## Acceptance criteria

- [ ] **C0 — the shell builds on the repo's own toolchain.** The Tauri
      crate compiles under `cargo build --release` on Rust 1.99.1, and
      `cargo clippy --all-targets -- -D warnings` covers it. Pinned by
      the gate's exit codes, read directly (KI-014).
- [ ] **C1 — the window serves the face unchanged.** The native window
      loads the loopback origin; every relative `fetch` and the
      `EventSource('/events')` work with **zero frontend edits**; a turn
      streams to the end. Pinned by a test that drives the same origin
      the window uses.
- [ ] **C2 — the server's lifecycle belongs to the window.** The app
      starts the existing server on an ephemeral port (or the default
      with a named conflict), never opens a browser tab, and stops the
      server when the window closes. Pinned.
- [ ] **C3 — the service worker survives the shell.** `sw.js` registers
      inside the native window on macOS, and the shell assets are served
      from the cache on a second launch — the PWA's offline shell is not
      a regression. Pinned.
- [ ] **C4 — the remote face is untouched.** `fleet gui --remote` still
      serves network peers, and the existing remote tests pass unchanged.
- [ ] **C5 — the component inventory is whole.** Every row in the
      inventory table is implemented: the services root resolves from
      inside the bundle, the lane wiring needs no shell profile, the
      venvs build, gen1's weights are named, and credentials are
      owner-supplied and never written. A component with no home fails
      this criterion rather than being noted. Pinned per row.
- [ ] **C6 — the first run is self-contained.** On a clean home, the app
      performs the install (layout, scaffold, venvs, wiring, services
      root), runs the doctor gate, and opens the face. No shell profile,
      no checkout, no Node. Pinned by a test that drives a scratch home
      through the app's own first-run path.
- [ ] **C7 — the lane wiring needs no shell.** A dock-launched app
      reaches the same lanes a terminal-launched one does: the three
      variables and sys1's config key are set by the app. Pinned by a
      test that builds the child environment with an empty profile.
- [ ] **C8 — the daemon lifetime is named, not assumed.** The plan and
      the docs state that cron and consolidation run while the app runs
      and stop when it quits; if the owner needs them to outlive the app,
      that is a separate LaunchAgent increment with its own criterion.
- [ ] **C9 — the package builds and launches.** `cargo tauri build`
      produces an `.app` and a `.dmg`; the `.app` launches, opens the
      face, and a turn runs. Owner-verified (a manual walk no test can
      stand in for).
- [ ] **C10 — the docs move with the increments.** `CLAUDE.md` names
      `fleet desktop` in its command list and the shell in its topology;
      the install docs carry the component inventory and the first-run
      path. Pinned by the doc files' own text.

## Files to be touched

| File | Change |
|---|---|
| `Cargo.toml` | the Tauri dependencies and the second binary target |
| `src/bin/fleet-desktop.rs` (or `src/tauri.rs`) | the shell: the window, the server lifecycle, the first-run install, the lane wiring |
| `tauri.conf.json`, `capabilities/` | the bundle's identifier, window, CSP, permissions, resources |
| `src/gui/mod.rs` | reuse `serve`'s server construction for the shell |
| `src/install.rs` | the first-run path the app drives; the lane wiring written, not printed |
| `src/lanes.rs` | the services root and the child environment the app sets |
| `src/main.rs` | the `fleet desktop` command |
| `CLAUDE.md`, `docs/features/install.md` | the command list, the topology, the inventory, the first-run path |

## Out of scope

- **The `fetch` → `invoke` rewrite.** The frontend keeps speaking HTTP;
  a second transport is the cost this plan exists to avoid.
- **Notarization, store distribution, auto-updater.** Non-commercial,
  GitHub releases, ad-hoc signed; the updater needs signing keys the
  owner has ruled out.
- **Windows.** Unsigned apps are blocked by Smart App Control; a
  certificate is out of scope.
- **Mobile.** Tauri mobile needs Node in the build, which hard rule 6
  forbids; the PWA still covers the phone.
- **A native UI rewrite** (Slint/egui/iced). Multi-quarter, and a weaker
  ecosystem for a dense, data-heavy console.
- **Cron and consolidation outliving the app.** A separate LaunchAgent
  increment, if the owner wants it (C8).

## Verification

- **The gate, whole:** `cargo fmt --all --check`, `cargo clippy
  --all-targets -- -D warnings`, `cargo build --release`, `cargo test`,
  each read by exit code. The Tauri crate joins the same gate.
- **A clean-home first run:** scratch `FLEET_HOME`, launch the app's
  first-run path, and assert the whole inventory — layout, scaffold,
  venvs, wiring, services root, doctor gate.
- **A live run:** launch the `.app`, open the face, run a turn, watch the
  stream, close the window and confirm the server is gone.
- **The PWA is not a regression:** the service worker registers and the
  offline shell still answers on a second launch.
- **The remote face still works:** `fleet gui --remote` and its tests.

## Linked artifacts

- The founding contract: `2026-10-12-fleet-native-app.md`; `CLAUDE.md:7`.
- The PWA layer this builds on: `2026-10-20-pwa-shell.md`.
- The thin-shell precedents: `2026-10-31-design-canvas-native.md`,
  `2026-11-02-face-plugin-host.md`.
- The install this extends: `docs/features/install.md`, `src/install.rs`.
- The research this condenses: Tauri v2 docs (prerequisites, config,
  calling-rust, calling-frontend, macOS signing, CSP, WebView versions);
  Tauri #13031 (service workers on the custom scheme, closed
  `not planned`); #14167 (`generate_handler!` stack overflow — the reason
  not to port 51 routes to IPC); #6721 (`u64` precision across IPC).
- KI-403 — the 1.99.1 toolchain's broken `rust-objcopy`. A `rustup
  update` removes the local repair, and the release strip fails silently.

## Risks

- **The shell tax.** CSP configuration, WebView differences, packaging.
  Mitigated by the thin shell: the frontend is unchanged, so the surface
  is the one the PWA already runs on.
- **Self-containment is the hard part, not the window.** The loose ends
  are all in the install: the services root, the lane wiring, the venvs,
  the daemon lifetime. C5–C8 exist because the window is the easy half.
- **Tauri's MSRV policy is "stable − 3".** Every future Rust move risks
  another lint pass, and Tauri may raise its floor. The 1.99.1 move cost
  41 lints in one pass; expect the same shape next time.
- **The toolchain repair is fragile.** KI-403's symlink dies on the next
  `rustup update`; the failure is a warning, so it ships an unstripped
  binary rather than a red gate.
- **macOS is the only clean unsigned target.** If the owner later wants
  Windows, the certificate decision reopens.

## Dependencies

- **Rust 1.99.1** — the machine's stable, moved 2026-11-02. Satisfies
  Tauri 2.12's MSRV 1.90.
- **Tauri 2.12.x**, `tao`, `wry` — the shell and the system WebView.
- **macOS 11+** as the practical floor (the docs say 10.15; an open bug
  crashes on Catalina).
- **No Node**, at build or runtime — hard rule 6.
- **The machine's own `uv`/`python3`** for the lane venvs; the first-run
  install degrades by name without them.

## Notes

- **The alternative this plan refuses:** a hand-rolled Swift `WKWebView`
  app (~200 lines, spawns `fleet gui --no-open`, loads the loopback
  origin). It is smaller and macOS-only, and it adds a second language to
  the repo. It is the right call if macOS-only is ever the settled scope;
  Tauri wins if Windows/Linux or its plugin ecosystem ever matter. Either
  way the component inventory is identical — it is the install, not the
  window, that carries the weight.
- **PWABuilder is complementary, not an alternative.** It produces store
  packages (Edge-hosted on Windows), not a controllable native app.
- **The owner's ruling that shapes all of this:** non-commercial, GitHub
  releases, "if people want to use them fine, if not that's also fine."
  That is why notarization, stores, and the auto-updater are out — and
  why the install's job is to make the app *whole*, not to sell it.
