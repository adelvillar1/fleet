---
status: completed
created: 2026-11-08
updated: 2026-11-08
slug: the-shell-updates-itself
---

# Plan: the shell updates itself — a Tauri updater end to end

## Context

Today's 0.5.1 ship turn was the third version-juggling session, and the owner
ruled it: **"we need to find a way to update the tauri app from within the
app itself, this juggling of versions is getting ridiculous."** The app is
`fleet-desktop` — a deliberately thin Tauri 2 shell (the owner's hard rule
12) that embeds the face at build time (`fleet::gui::serve` compiled into
the binary) and opens a window on its own `http://127.0.0.1` origin. The
installed app stamps its version in `Contents/Info.plist`
(`CFBundleShortVersionString`), and the shell gains a route the old app
never had: the KI-481 turn added `GET /api/version` on the face's HTTP API,
pinned.

The release flow already ships two artifacts per tag (`scripts/release.sh
assets`): the dmg at `target/release/bundle/dmg/fleet_<V>_aarch64.dmg`
(11.85 MiB for 0.5.1) and a CLI tarball. One GitHub release exists so far
(`v0.4.0`, public repo). The feed file the updater needs — the release's
`latest.json` — is generated at cut time; nothing today publishes it.

## Approach

**The channel**: the release `latest.json`.

`scripts/release.sh`'s assets mode already builds, bundles and uploads the
dmg + CLI tarball for the tag. Extend it: **after a green
`gh release upload`, generate `latest.json` in the updater's v2 format from
the version three ways** — `tauri.conf.json` version key, the dmg's
filename, and `strings fleet-desktop | grep '<the version>'` — refuse on
mismatch, upload it to the release. Refusing is correct: the feed is only
true when a gated tree, the tag, the bundled literal, the dmg name and the
feed agree.

**The shell**: tauri-plugin-updater v2.

- The updater plugin with its **`dangerous-insecure-transport` runtime
  instruction** — the whole channel is loopback-only or the owner's LAN IP;
  HTTPS comes from the GitHub release URL. This is the plugin's stated,
  sanctioned shape for "loopback-only with remote HTTPS"; the feed and the
  dmg are fetched over HTTPS; the local transport is the only insecure leg.
- The shell config under `[bundle]`:
  - `"createUpdaterArtifacts": true` — the `.tar.gz` updater artifact is
    generated per platform during `cargo tauri build`.
  - `"pubkey"` — **generated during this work, the private key sealed in
    the owner's vault** (`fleet vault set FLEET_TAURI_UPDATE_KEY`), the
    public key in the config and documented for the updater's
    verification.
- The shell carries the face's `/api/version` route (KI-481) pinned —
  the self-check endpoint exists.

**The bridge**: an update card, not a daemon — the state section rides the
settings document, the actions are verb routes, the owner decides when.

- `update` section in the settings document: `current`, `candidate`,
  `notes` head, and `installed_via: "dmg"|"dev-build"|"source"` —
  computed from the release feed at check time (a network field in an
  otherwise local document, carried honestly as one).
- `GET /api/update/check`: the feed fetch, bounded (10 s). The
  **no-network honesty clause**: a refused fetch names itself ("the
  release feed was not reached: <error>") — never reported as
  "up to date", because no-network ≠ up-to-date.
  `installed_via` detects the install mode by process identity: the
  executable's path inside `/Applications/fleet.app` is a dmg install;
  anything under `target/` is a dev build and self-update is refused
  there — a dev shell must not overwrite the owner's installed app
  (the named refusal: "a dev build does not update the installed app").
- `POST /api/update/apply`: the plugin's chain — download, verify the
  signature against the pinned public key, install, relaunch (`exit 0`
  on relaunch; the window's origin is loopback, so the shell comes back
  serving itself). Refused for dev builds; for an `installed_via=source`
  app the refusal names the CLI channel ("this app was installed from
  the dmg — the CLI face updates through `fleet update`").
- The CLI channel: `fleet update` — the CLI binary downloads the same
  release's tarball, verifies, and atomically replaces `~/.fleet/bin/fleet`
  (same-dir rename); launchd's restart contracts put the new inode on the
  port at the next restart, and the CLI prints the version it installed.
- The CLI channel rides the release's CLI artifact — the raw `fleet`
  binary asset at the branch-permanent release URL, hash-asserted by
  the feed's `sha256` (carried in the cli row's `signature` field) —
  never the dmg. The CLI's read side is `sha2::Sha256`, the sha2 crate
  already in the dependency graph.

## Criteria

- [x] The release cut publishes a `latest.json` the v2 updater can read; a version disagreement among the tree's declaration, the bundled binary and the dmg filename refuses the cut.
- [x] The shell's updater artifacts are signed with a key whose private half is sealed in the owner's vault and whose public half is in the shell's config.
- [x] The update card (the settings board's update section and the menu's About pane) answers current/candidate from the release feed at read time.
- [x] A refused feed fetch names itself as unreachable in the card — never "up to date" (no-network ≠ up-to-date).
- [x] A dev build refuses `/api/update/apply` with the named refusal (a dev shell never overwrites the owner's installed app).
- [x] The CLI channel: `fleet update` verifies, atomically replaces its own binary, and prints the new version (--version answers it from the same inode it wrote).
- [x] Every criterion is met **and** the plan checker is green (gate leg 4) at status: completed.

Status: completed 2026-11-08. The open question closed: the About pane hosts
the apply button — the CLI channel cannot replace the shell bundle (the
running process IS the artifact being swapped), so the shell's own apply is
the plugin chain and it lives on the About pane; the settings' update card
carries the state and the refresh and points at About for the apply.

## Non-goals

- Auto-update-on-launch — the card is the consent surface; a settings key would decide a check cadence only, and only if the owner asks for one.
- Windows/Linux updater artifacts — the house ships aarch64-apple-darwin only.
- A private feed server — the GitHub release is the feed; no new infrastructure.
- Cross-updating — the shell and the CLI each update themselves; neither updates the other.

## Open questions (owner triage)

- The updater's private key signs at cut time — release.sh reads it from the owner's vault at cut, so the vault must be unlocked at cut. Confirm the key name (`FLEET_TAURI_UPDATE_KEY`?).
- The v0.4.0 release has no `latest.json` — the feed is forward-only;
  no retro-fix (0.6.0 is the first cut the feed covers).
- Does the menu's About pane host the apply button, or does the update section carry its own?
