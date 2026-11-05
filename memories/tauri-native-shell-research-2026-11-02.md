---
name: tauri-native-shell-research-2026-11-02
description: "reference/project 2026-11-02 — deep research on turning fleet's browser face into a native desktop app: Tauri v2 facts, the two migration shapes, the decisive constraint set (the remote face forbids removing the HTTP server), the thin-shell recommendation, and the owner's non-commercial ruling (GitHub releases, no notarization, no store) which retires three decisions and makes macOS-first the honest scope"
type: reference
date: 2026-11-05
basis: confirmed-by
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

The owner asked (2026-11-02) for "deep research on turning the pwa app into a
proper native app using tauri". Three parallel research passes (Tauri v2
technical facts; migration patterns and pitfalls; the PWA→native option
landscape) plus a read of the face's own serving model. **No plan was drafted
and nothing was built** — this is the research record, awaiting the owner's
rulings. Follows [[creator-mode-plugin-host-exploration-2026-11-02]] as the
second "how should fleet grow a surface" question in the same week.

## What the "PWA" actually is (verified in the tree)

- `fleet` runs an HTTP server on **loopback only**, `127.0.0.1:3080`
  (`src/gui/mod.rs:81,86`; `Server::bind` at `src/gui/server.rs:144`).
- The frontend is a **vanilla SPA** — 950 KB `app.js`, 245 KB `app.css`, no
  framework, no build step (`src/gui/assets/`).
- **51 `fetch()` calls, all relative** (`/api/...`), plus one relative
  `EventSource('/events')` (`src/gui/assets/app.js:8163`).
- PWA shell: `manifest.webmanifest` + `sw.js` caching only `app.css`/`app.js`
  at the current `?v=` hash; assets are embedded and hash-busted at serve time
  (`build.rs`).
- **A remote face exists**: `fleet gui --remote` serves the same frontend to
  network peers with pairing and a path-based cookie boundary
  (`src/gui/mod.rs:142-225`, `src/remote/`).
- Strict CSPs exist only on design-canvas preview routes, not the main app
  (`server.rs:1314,1419`).

## Tauri v2, as of 2026-11-02

- **Stable `tauri` 2.12.5** (2026-10-21); v2 stabilized 2024-10-02; **Tauri 3
  is alpha** — do not adopt; v1 is legacy (last release 2025-05-17).
- **Node is optional**: `cargo install tauri-cli --locked`; `frontendDist`
  accepts a plain directory; `beforeBuildCommand` defaults to `null`. The
  official streaming example ships `frontendDist: ["index.html"]` with no
  bundler. **Satisfies hard rule 6** (no Node) on desktop; mobile targets still
  need Node, and fleet is a desktop console.
- **Per-platform WebView**: macOS WKWebView (10.15+ documented, but an open
  bug crashes on Catalina — treat 11+ as the floor); Windows WebView2
  (preinstalled on Win10 1803+, Tauri can bootstrap it); Linux **WebKitGTK
  4.1** (the maintainer has "completely stopped defending webkitgtk").
- **MSRV is Rust 1.90, edition 2024** — this machine had **rustc 1.89.0**
  (`~/.rustup/toolchains/stable-aarch64-apple-darwin`). A toolchain move
  (or pinning an older 2.x) is a prerequisite, not a detail.
  **RESOLVED 2026-11-02**: the owner ruled for 1.99.1 and
  `rustup update stable` moved the machine — see
  [[rust-toolchain-moved-to-199-and-clippy-red]]. The MSRV blocker is gone;
  what the move cost is 41 clippy errors, not a Tauri limitation.
- **No default CSP in v2** (`csp` is `null` unless set), so nothing is
  injected that would block loopback fetch/SSE.
- **Signing**: macOS notarization needs a $99/yr Apple Developer account;
  ad-hoc (`signingIdentity: "-"`) works but Gatekeeper warns. Fleet ships an
  unsigned binary today, so this is a new operational cost.

## The two migration shapes

- **A. Thin shell** — keep the HTTP server, point the WebView at
  `http://127.0.0.1:<port>` (`tauri-plugin-localhost`, `WebviewUrl::External`).
  Maintainers call it "essentially the same thing as a Tauri command, just
  worse" — *for desktop-only apps*. Fleet is not desktop-only.
- **B. Proper IPC** — rewrite 51 fetches as `invoke()` and SSE as
  events/channels. Documented risks: `generate_handler!` stack-overflows
  around ~100–200 commands; `u64`/`i64` lose precision crossing IPC (fleet
  counts tokens in `u64`). No quantitative migration-effort study exists; the
  pattern practitioners report is "web-first, Tauri-optional", not a
  mechanical fetch→invoke rewrite.

## The decisive constraints

1. **The remote face forbids removing the HTTP server.** IPC-in-desktop +
   HTTP-in-browser means two transports for one UI — exactly what the
   embedded-axum developer said he was avoiding, and his case for the
   embedded server was *multiple clients*.
2. **Relative URLs make the thin shell genuinely drop-in.** All 51 fetches and
   the EventSource resolve against whatever origin the WebView loads, so
   loading the loopback origin means **zero frontend changes**.
3. **The service worker survives the thin shell and dies the IPC shape.** On
   macOS, Tauri's default `tauri://localhost` scheme is rejected by WebKit's
   service-worker registration (Tauri #13031, closed `not planned`/`upstream`).
   Over a real `http://127.0.0.1` origin the SW keeps working. Windows
   WebView2 supports SW on normal origins; Linux WebKitGTK since 2.28.
4. **The gate** (fmt/clippy/build/test) would cover a second crate;
   `cargo install tauri-cli` is a build tool, not a runtime dep.

## Recommendation given

Adopt Tauri v2 as a **thin shell** — a new `fleet desktop` binary target that
starts the existing server and opens a native window on it. Do not rewrite the
frontend to IPC. Phases: (1) toolchain + skeleton; (2) server lifecycle
(ephemeral port, suppress browser auto-open, stop on window close);
(3) distribution (ad-hoc `.app`/`.dmg` first, notarization only if wanted);
(4) native increments (tray, menu, updater, fs/dialog plugins); (5) leave
`fleet gui` untouched for the remote face.

What it buys: a real native window (no tab/address bar), dock/taskbar
presence, native menus/tray, signed auto-update, store packaging. What it
costs: a second crate, signing/notarization, the shell tax (CSP checks,
WebView differences, updater infra), a toolchain move.

## Honest counterpoints

- **Fleet is already a native app** — a single static Rust binary with no
  runtime deps; `fleet gui` opens a browser tab. The gain is *presence and
  packaging*, not native-ness. If the complaint is "it feels like a website",
  the fix may be as much about the window as the shell.
- **A hand-rolled Swift `WKWebView` wrapper** is ~200 lines, macOS-only, zero
  new Rust deps, no toolchain move, no Node — legitimate if the target is
  macOS-only. Tauri wins if cross-platform matters.
- **Keep the PWA** is a real position (Microsoft Teams Linux shipped a PWA
  over an Electron client) — zero cost, but no dock presence or updater.
- **Native UI rewrite (Slint/egui/iced)** — multi-quarter, weaker ecosystem
  for a dense data-heavy console with tables and streaming text. Not
  recommended. **Electron** is ruled out by hard rule 6 and is 30–80× larger.

## The owner's rulings (2026-11-02, same turn)

> "I don't plan to commercialize this, the installable packages will be on
> github, if people want to use them fine, if not that's also fine"

This is the same no-expectation posture as [[owner-enterprise-it-fingerprint]]
("if people want to use it and modify it that's fine with me") applied to
DISTRIBUTION, and it retires three of the five open decisions:

- **Notarization is out** — no $99/yr Apple Developer account. Ad-hoc signing
  (`signingIdentity: "-"`) is the whole story; macOS users get the one-time
  Gatekeeper click. Standard for a hobby release.
- **Store distribution is out** — no MSIX, no Partner Center, no Play Console.
- **The auto-updater is out** — it needs minisign keys and a signed endpoint;
  users download new GitHub releases by hand. Fine for a non-commercial tool.

What remains: the **Rust 1.90 toolchain move** (Tauri 2.12's MSRV is 1.90,
policy "stable − 3"; this machine was 1.89.0 — verified in the 2.12 release
notes), a second crate plus `tauri.conf.json`, and the shell tax.
**The toolchain move is now DONE** (1.99.1) — the remaining cost is its 41
clippy errors, tracked in [[rust-toolchain-moved-to-199-and-clippy-red]].

**The scoping consequence — the platforms are no longer equal.** Unsigned
reality: **macOS** ad-hoc `.dmg` works (one Gatekeeper click); **Linux**
AppImage runs unsigned but needs WebKitGTK 4.1 at runtime; **Windows**
SmartScreen warns and Windows 11 Smart App Control blocks unknown unsigned
apps outright, so Windows is effectively out without paying for an OV/EV
certificate. Honest scope: **macOS-first, Linux optional, Windows out.**

**Is it worth building at all?** The value splits: for the owner's own daily
use it is a real native window (no tab, no address bar, dock icon, menu bar,
tray) — the substantive win; for others it is a GitHub release artifact whose
Gatekeeper/SmartScreen friction buys little. If the daily-driver window is the
goal, the thin shell is cheap and worth it; if it is mainly about
distributing, the friction buys less.

**The macOS-only alternative is now the live question.** A hand-rolled Swift
`WKWebView` app (~200 lines, spawns `fleet gui --no-open`, loads
`http://127.0.0.1:3080`) needs no toolchain move, no Tauri version churn, no
second crate, no Node — the smaller machine for a single-platform
non-commercial tool. Tauri wins if Windows/Linux ever matter or if its plugin
ecosystem (fs, dialogs, notifications, tray) is wanted.

## Open decisions still with the owner

1. **Which platforms?** macOS-only (the Swift wrapper becomes the smaller
   machine) or macOS + Linux (Tauri)? Windows is out without a certificate.
2. **Which shell?** Tauri (Rust, cross-platform, more moving parts) or the
   Swift `WKWebView` wrapper (macOS-only, minimal)?
3. **The toolchain's collateral.** The owner's 1.99.1 ruling is satisfied, but
   the move turned fleet's clippy leg red (41 errors) and is a shared default
   the co-tenant session inherits. Offered and unanswered: fix the 41 now,
   revert `stable` to 1.89 until the plugin host lands, or coordinate. See
   [[a-toolchain-move-is-a-shared-default]].

The remote face and mobile questions are settled by the ruling and the
research: the Tauri app is a *third* client and `fleet gui --remote` stays
as-is; mobile stays out (it needs Node, and fleet is a desktop console).

## The plan is now DRAFTED — `docs/plans/2026-11-02-tauri-native-shell.md`

Written 2026-11-02 after the owner said *"amend the tauri native app plan to
reflect the new rust version"* (no plan existed yet — the research had been
delivered in conversation and never written down; the only Tauri mention in
`docs/plans/` was a design reference in `2026-10-16-gui-revamp.md`). Committed
as `32b215c` — **also on `face-plugin-host`**, same branch switch as
`eabe031`. Status `draft`; **six criteria C0–C6**, each pinned, with C5
(owner-verified package launch) honestly marked as a manual walk.

The plan folds in the 1.99.1 move as settled fact (no MSRV blocker), the
owner's non-commercial ruling (no notarization / stores / updater; Windows out;
macOS-first, Linux optional), the thin-shell shape, and the Swift `WKWebView`
alternative in Notes. Out of scope names the `fetch`→`invoke` rewrite,
notarization, Windows, mobile, and a native UI rewrite.

**Still open with the owner, unchanged:** which platforms (macOS-only vs
macOS+Linux) and which shell (Tauri vs the Swift wrapper). Until both are
ruled the plan cannot flip `active`.

## The plan is NOT in line with the past, and does not say so (2026-11-02)

The owner's next question was **"is the plan in line with what we have done in
the past?"** The audit found it **in line in shape, against three past
positions, and silent about all three** — the plan presented the thin shell as
the obvious continuation. Recorded as its own feedback memory:
[[a-new-plan-must-answer-the-past-plans]].

- **In line:** a native surface was always the plan
  (`2026-10-12-fleet-native-app.md:61-65`, "a native UI surface" under Later
  slices); the thin shell **is** the house pattern for making a web thing feel
  native (design-canvas and the plugin host both did it by serving through the
  bridge and mounting as rail surfaces); no Node (hard rule 6); the remote face
  stays first-class.
- **Against, and unnamed in the plan:** "one static binary" is the founding
  contract (`CLAUDE.md:7`, `docs/features/install.md:233`); the PWA plan
  refused a wrapper shell in exactly these words
  (`2026-10-20-pwa-shell.md:66-68`, "a Kotlin/Gradle tree and a second
  toolchain for a crate whose contract is one static binary"); and the PWA was
  chosen to supply the "app" part (same plan's Context). `surfaces.md` has no
  desktop destination among its thirteen.
- **The difference that makes it arguable but unstated:** the PWA refusal was
  about **mobile**, where the PWA already covers the need and Kotlin/Gradle is
  a foreign toolchain — on desktop the need is uncovered and Tauri is Rust.
- **A terminology collision to fix too:** `docs/features/gui.md:1` calls this
  "the browser face" and `:398` "two shells, two ports" already means the bot
  and main faces — a third meaning of "shell" will collide.

The amendment offered: a **"Precedents this plan answers"** section naming all
three, plus plainer wording than "thin shell". Not yet applied — the owner has
not ruled on it.

## The owner's correction, and the plan rewritten whole (2026-11-02)

The owner's answer to the audit reframed the history: **the PWA refusal was
for rapid iteration on a working app, and the PWA was the way to go — the
native installable shell is the NEXT STEP in fleet's evolution.** Self-
containment was the original intent *"from the very beginning"*. So the plan
was rewritten whole (not patched), committed as `d90123c` and `104bc06` on
`face-plugin-host`:

- **A "Precedents this plan answers" table** — the 2026-10-12 native-app plan
  ("a native UI surface" was always a later slice), the PWA plan's refusal
  (rapid iteration, mobile covered), design-canvas-native and the plugin host
  (the thin-shell pattern at lower levels), and `install.md`'s "one static
  binary" release path (the static binary **stays**; the native package is an
  additional artifact).
- **The founding intent stated as the plan's own constraint**: *"no embedded
  Node runtime, no staged harness payload, no checkout dependency"* — the
  native shell must **honor** self-containment, not erode it.
- **A 16-row component inventory** (binary, face assets, services, venvs, lane
  wiring, blend1, gen1's weights, embedding server, home layout, corpus,
  credentials, MCP registry, plugins, the consolidator + cron, the remote face,
  designs) with a "how the install carries it" column.
- **Criteria renumbered C0–C10**, with the install's weight in C5 (the
  inventory is whole), C6 (the clean-home first run), C7 (the lane wiring needs
  no shell profile), C8 (the daemon lifetime is named, not assumed).

**The finding the inventory produced — the window is the easy half; the loose
ends are all in the install**, and each is a place the PWA era leaned on
something outside the app:

1. **The services root.** The 12-ancestor walk requires a **`Cargo.toml`
   sibling** (`src/lanes.rs:985`) and its fallback is the compile-time manifest
   dir (`:991`) — neither exists inside a bundle, so the app must set
   `FLEET_SERVICES_ROOT` to the bundled `Contents/Resources/services`.
2. **The lane wiring.** `fleet install` *prints* `export SEM1_ST_VENV=…`,
   `SDM1_MODELS_VENV`, `GEN1_KOKORO_PYTHON` and a sys1 `config.toml` note for
   the owner's shell profile — **a dock-launched app sources no profile**, so
   the app must set the three variables in the child environment and write
   sys1's key itself.
3. **The venvs** (1.6 GB, `uv`, receipt discipline) — the first-run install
   builds them and degrades by name.
4. **gen1's ONNX weights** — machine-owner downloads; the app names them, never
   fetches.
5. **Credentials** — owner-supplied; the app never writes a key.
6. **The daemon lifetime.** The consolidator and cron scheduler are spawned by
   the GUI bridge (`src/gui/bridge.rs:10062,10068`) — they run while the app
   runs and stop when it quits. If scheduled jobs must outlive the app, that is
   a separate LaunchAgent increment (the repo already runs
   `local.fleet-maintenance` that way), not a silent gap.

**Still open with the owner, unchanged:** which platforms (macOS-only vs
macOS+Linux) and which shell (Tauri vs the Swift `WKWebView` wrapper). Until
both are ruled the plan stays `draft`.

## THE PLAN IS IMPLEMENTED — 2026-11-02, same day

The owner's next instruction was *"implement plan as designed"*. **It is
built, the plan is `completed` with all ten criteria ticked with evidence,
and the gate is green (2,002 tests / 0 failed).** See
[[tauri-native-shell-implemented-2026-11-02]] for the crate, the inventory
rows it closed, the package (9.48 MiB dmg), the Tauri build facts (the
`resources` map direction, the RGBA icon requirement, `gen/` being generated),
and the owner's remaining manual half.

The plan's own prediction was wrong in a useful way: it listed
`src/gui/mod.rs`, `src/install.rs`, `src/lanes.rs`, and `src/main.rs` as files
to touch. **It touched none of them** — the shell reuses `fleet::gui::serve`,
`fleet::install`, and `fleet::home` through their public APIs, so the main
crate's only change is the `[workspace]` section. When a plan says "reuse
these modules", check whether their public API already carries the whole
shape before budgeting edits to them.

Related: [[fleet-is-the-native-rust-successor-to-the-fork]],
[[fleet-gui-is-a-launchagent]], [[fleet-pairing-qr-without-tailscale-serve]],
[[gui-client-verified-by-a-vm-harness]],
[[a-memory-index-line-is-not-the-plans-state]],
[[rust-1991-toolchain-move]].
