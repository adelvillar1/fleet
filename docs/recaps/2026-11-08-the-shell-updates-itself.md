# Session Recap — 2026-11-08: the shell updates itself

**Summary.** The self-updater shipped whole as v0.6.0 — the owner's ruling
"we need to find a way to update the tauri app from within the app itself"
landed end to end: the CLI channel (`fleet update`), the feed machinery
(`src/update.rs`) with its refusals that never read as up-to-date, the
face's check/apply routes, the About pane with the apply button, and the
shell's own `tauri-plugin-updater` chain. 0.6.0 is built, installed, and
live (`GET /api/version` → 0.6.0); it is the last manual install by the
owner's own framing, and every release after it is a button. The updater's
private key is sealed in the vault (`FLEET_TAURI_UPDATE_KEY`) — the owner's
owe-item from the checkpoint, closed this session.

## Plans worked on

### `docs/plans/2026-11-08-the-shell-updates-itself.md` — the Tauri updater end to end

Status after this session: **completed** (every box ticked, `plan_check.py`
green, the About-pane open question closed in the doc: the apply button
lives on the About pane, because the CLI channel cannot replace the shell
bundle — the running process IS the artifact being swapped).

| Acceptance criterion | Status | Evidence |
|---|---|---|
| the cut publishes a `latest.json` the v2 updater can read, three-way refusal | ✅ met | `scripts/release.sh`'s feed section: conf version ↔ dmg name ↔ `strings fleet-desktop` literal, `die` on disagreement; generation + leg read + `gh release upload` of `latest.json` (`bash -n` green) |
| artifacts signed with the private half sealed in the vault | ✅ met | `cargo tauri build` with `TAURI_SIGNING_PRIVATE_KEY=/tmp/fleet-updater-key` produced `fleet.app.tar.gz` + `.sig`; the pair-match probe (signature's embedded signer id `4223e59b2eb4069e` == the conf's pinned pubkey id) proved the key; the key sealed in the vault via the face's `vault-seal-env` op, plaintext line dropped |
| the update card answers current/candidate at read time | ✅ met | `src/gui/bridge.rs` settings' `update` entry + `GET /api/update/check`; the About pane's feed card and the settings' update card paint from it |
| a refused fetch names itself, never "up to date" | ✅ met | `src/update.rs::fetch`/`check_field`: malformed and unreachable feeds produce `refused:{reason,…}`; pinned by the 8 unit tests |
| a dev build refuses apply with the named refusal | ✅ met | `self_update` and the face's `pressApply` refuse with "a dev build does not update…"; `fleet update` on a target/ checkout refuses live this session |
| the CLI channel verifies, atomically replaces, prints the new version | ✅ met | `src/update.rs::self_update` (sha256 vs feed, staging, same-dir rename), `fleet update` routed through `main.rs`; `--version` 0.6.0 from the installed inode |
| every criterion met and the checker green at status completed | ✅ met | `plan_check.py` exit 0, `status: completed` set |

## The live proofs

- **0.6.0 is what the face serves.** `launchctl kickstart` restarted the
  host off the atomically-swapped `~/.fleet/bin/fleet`; the endpoint
  answers `{"name":"fleet","version":"0.6.0"}`.
- **The dmg stamp and the app.** `fleet_0.6.0_aarch64.dmg` (12.41 MiB)
  carries `CFBundleShortVersionString 0.6.0`; `/Applications/fleet.app`
  holds the 0.6.0 bundle with `services/` inside.
- **The updater artifact exists and verifies.** The first build produced
  no `.tar.gz` (dmg alone is no updater target — the bundler warned);
  `targets: ["dmg","app"]` fixed it, and the second cut produced
  `fleet.app.tar.gz` + a signed `.sig`.
- **The key seal under the passphrase you type.** The owner unlocked the
  vault in place; the seal ran through the face's `vault-seal-env` so the
  private key's bytes never entered the agent transcript; the staged
  plaintext `.env` line was dropped after (grep-count 0).

## What this session fixed on the way

- `release.sh`'s feed section had been assembled twice (two heredocs, one
  stdout-printing) and quoted `"$TAURI"` (an unexecutable single word);
  it now stands as one clean section with a direct `cargo-tauri` call.
- `bump-version.sh`'s blind `sed` on `Cargo.lock` mangled `approx`,
  `crossbeam-channel` (0.5.17 → an unparseable 0.6.07) and friends; the
  leg now moves only the `fleet`/`fleet-desktop` entries. Caught by the
  next cargo invocation; restored and re-bumped.
- `.gitignore` gained `src/**/*.lock` + `src-tauri/**/*.lock` (the
  filelock sidecar seam; the face leaves them beside edited files, and a
  flocked file is never unlinked).

## Commits this session

- `ce33715` — the release commit: the updater end to end + bump + docs.
- `9841998` — release.sh's exec bit (a chmod flip from the bump).
- `22a1929` — `targets: ["dmg","app"]` so createUpdaterArtifacts has a
  target to build the updater artifact for.

(KI-481's own fix is `ffff77b`, already in history before this
session — cited from the checkpoint, not a commit of this one.)
- final — the memory fact + this recap.

## Open threads

- **v0.6.0 is not yet published to GitHub.** `gh release list` still
  names only v0.4.0; the first cut (`scripts/release.sh assets v0.6.0`)
  is the owner's trigger — it needs ~30 min of release-time builds and
  the key is now both at `/tmp` (600) and in the vault for its vault
  leg. Every later release can then be cut from the running app.
- **KI-480 open** (session discoverability across compaction folds);
  KI-481 closed earlier (`ffff77b`).
- **~24.6k `fleet-eval-*` temp dirs** ride the daily maintenance sweep;
  no manual action.
- The face's About pane will show "dev build" refusal copy only in
  dev-checkout installs; the installed 0.6.0 shows the feed card.
