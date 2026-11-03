---
status: draft
created: 2026-11-02
updated: 2026-11-02
slug: tauri-native-shell
---

# Plan: the native shell — Fleet in a Tauri window, distributed from GitHub

## Context

Fleet's face is a PWA. `fleet gui` starts a Rust HTTP server on
`127.0.0.1:3080` (`src/gui/mod.rs:81,86`), serves a vanilla SPA (950 KB
`app.js`, no framework, no build step), and registers a manifest and a
service worker (`src/gui/assets/manifest.webmanifest`, `sw.js`). The
same server also serves **remote peers** — `fleet gui --remote` is a
first-class mode with pairing and a cookie boundary by path
(`src/gui/mod.rs:142-225`). The owner wants a *proper native app*;
distribution is GitHub releases, non-commercial, and optional for anyone
else.

The research (2026-11-02, in session) settled the option space:

- **The toolchain is no longer a blocker.** Rust moved to **1.99.1**
  on 2026-11-02, and Tauri 2.12's MSRV is 1.90 — satisfied. Tauri 3 is
  alpha; v1 is legacy.
- **Node is not required.** `tauri-cli` installs with `cargo install`;
  `frontendDist` takes a plain directory; `beforeBuildCommand` defaults
  to `null`. Hard rule 6 (never reintroduce Node) holds on desktop.
- **The service worker decides the shape.** On macOS, Tauri's default
  `tauri://localhost` scheme is rejected by WebKit's service-worker
  registration (Tauri #13031, closed `not planned`/`upstream`). Over a
  real `http://127.0.0.1` origin the worker keeps working.
- **Unsigned distribution is not equal across platforms.** macOS
  ad-hoc `.dmg` works behind one Gatekeeper click; Linux AppImage runs
  unsigned but needs WebKitGTK 4.1 at runtime; Windows Smart App
  Control blocks unknown unsigned apps outright, so Windows is out
  without a paid certificate the owner has ruled out.

## Approach

**A thin shell: keep the HTTP server, and add a Tauri window that loads
the existing loopback origin.**

The frontend's 51 `fetch` calls are all relative (`/api/...`) and its
stream is a relative `EventSource('/events')`, so a WebView pointed at
`http://127.0.0.1:<port>` needs **zero frontend changes**. The remote
face is why the server cannot be removed: a desktop app that spoke IPC
while browsers spoke HTTP would maintain two transports for one UI —
the exact cost the embedded-axum developer cited for keeping his server.

The increments, each independently valuable:

1. **The shell builds.** A Tauri crate (second `[[bin]]` or a
   `src-tauri` sibling) with `tauri.conf.json`, an identifier, and a
   window pointed at the loopback origin. The gate covers it.
2. **The lifecycle belongs to the window.** Start the same server
   `fleet gui` uses, on an ephemeral port (or the default with a named
   conflict), suppress the browser auto-open, stop it on window close.
3. **The face is unchanged.** No `fetch` → `invoke` rewrite. The same
   frontend serves the browser, the remote peers, and the native window.
4. **The package.** `cargo tauri build` → `.app` + `.dmg`, ad-hoc
   signed, uploaded to a GitHub release by hand.
5. **Native increments, each its own commit.** Tray icon, menu bar,
   notifications, filesystem/dialog plugins — only as the owner wants
   them.

The docs move with the increments: `CLAUDE.md`'s command list and
branch topology, and a docs section for the shell, land with increment 2
and 4 respectively — not deferred to a finale.

## Use cases

- **The owner, daily.** A dock icon and a real window: no browser tab,
  no address bar, the face one click from the dock.
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
- [ ] **C5 — the package builds and launches.** `cargo tauri build`
      produces an `.app` and a `.dmg`; the `.app` launches, opens the
      face, and a turn runs. Owner-verified (a manual walk no test can
      stand in for).
- [ ] **C6 — the docs move with the increments.** `CLAUDE.md` names
      `fleet desktop` in its command list and the shell in its topology;
      a docs section describes the thin-shell shape and the
      GitHub-releases distribution. Pinned by the doc files' own text.

## Files to be touched

| File | Change |
|---|---|
| `Cargo.toml` | the Tauri dependencies and the second binary target |
| `src/bin/fleet-desktop.rs` (or `src/tauri.rs`) | the shell: the window, the server lifecycle |
| `tauri.conf.json`, `capabilities/` | the bundle's identifier, window, CSP, permissions |
| `src/gui/mod.rs` | reuse `serve`'s server construction for the shell |
| `src/main.rs` | the `fleet desktop` command |
| `CLAUDE.md` | the command list and the topology section |
| `docs/features/gui.md` or a new shell page | the shape and the distribution |

## Out of scope

- **The `fetch` → `invoke` rewrite.** The frontend keeps speaking HTTP;
  a second transport is the cost this plan exists to avoid.
- **Notarization, store distribution, auto-updater.** Non-commercial,
  GitHub releases, ad-hoc signed; the updater needs signing keys the
  owner has ruled out.
- **Windows.** Unsigned apps are blocked by Smart App Control; a
  certificate is out of scope.
- **Mobile.** Tauri mobile needs Node in the build, which hard rule 6
  forbids.
- **A native UI rewrite** (Slint/egui/iced). Multi-quarter, and a weaker
  ecosystem for a dense, data-heavy console.

## Verification

- **The gate, whole:** `cargo fmt --all --check`, `cargo clippy
  --all-targets -- -D warnings`, `cargo build --release`, `cargo test`,
  each read by exit code. The Tauri crate joins the same gate.
- **A live run:** launch the `.app`, open the face, run a turn, watch the
  stream, close the window and confirm the server is gone.
- **The PWA is not a regression:** the service worker registers and the
  offline shell still answers on a second launch.
- **The remote face still works:** `fleet gui --remote` and its tests.

## Linked artifacts

- The research this plan condenses: Tauri v2 docs (prerequisites,
  config, calling-rust, calling-frontend, macOS signing, CSP, WebView
  versions); Tauri #13031 (service workers on the custom scheme, closed
  `not planned`); #14167 (`generate_handler!` stack overflow — the reason
  not to port 51 routes to IPC); #6721 (`u64` precision across IPC).
- KI-403 — the 1.99.1 toolchain's broken `rust-objcopy`. A `rustup
  update` removes the local repair, and the release strip fails silently.

## Risks

- **The shell tax.** CSP configuration, WebView differences, packaging.
  Mitigated by the thin shell: the frontend is unchanged, so the surface
  is the one the PWA already runs on.
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

## Notes

- **The alternative this plan refuses:** a hand-rolled Swift `WKWebView`
  app (~200 lines, spawns `fleet gui --no-open`, loads the loopback
  origin). It is smaller and macOS-only, and it adds a second language to
  the repo. It is the right call if macOS-only is ever the settled scope;
  Tauri wins if Windows/Linux or its plugin ecosystem ever matter.
- **PWABuilder is complementary, not an alternative.** It produces store
  packages (Edge-hosted on Windows), not a controllable native app.
- **The owner's ruling that shapes all of this:** non-commercial, GitHub
  releases, "if people want to use them fine, if not that's also fine."
  That is why notarization, stores, and the auto-updater are out.
