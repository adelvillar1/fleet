# The updater

Fleet updates itself from its own GitHub releases. There is one feed and two
channels: the desktop shell updates its whole `.app` through Tauri's updater
plugin, and the CLI face updates its own binary with `fleet update`. Both read
the same `latest.json`; neither fetches anything from the local machine.

The surface is the About pane (this build, the release feed's answer, the apply
press), the `update` card in Settings, and `fleet update` on the command line.
The deep dive that found the two channels dead, and the plan that fixed them,
are `docs/architecture/deep-dive-2026-11-09-tauri-updater.md` and
`docs/plans/2026-11-09-updater-that-installs.md`.

## The provenance rule

The update is fetched from **GitHub and nowhere else** (the owner's 2026-11-09
ruling). The feed is a constant (`src/update.rs`), the shell's endpoint is
pinned in `src-tauri/tauri.conf.json`, and there is no settings key, flag or
environment variable that repoints either. Every byte an update installs comes
from `github.com/adelvillar1/fleet/releases/…`; the machine is the install
target and the press surface only.

The transport is HTTPS with no exception: the updater's
`dangerousInsecureTransportProtocol` was removed in 0.6.5 (KI-489), because a
plain-http endpoint is how a local or LAN source would slip in.

## The feed

`latest.json` lives at the branch-permanent
`releases/latest/download/latest.json` and carries two row shapes:

| Row | Consumer | Artifact | Signature |
|---|---|---|---|
| `platforms["darwin-aarch64"]` | the shell's updater plugin | `fleet.app.tar.gz` | minisign, verified against the pubkey pinned in `tauri.conf.json` |
| `cli` | `fleet update` | `fleet-aarch64-apple-darwin` — a **raw binary**, no archive | sha256 of that file, in the `signature` field |

The cli row's raw-binary shape is the contract the client relies on: it writes
the bytes it downloaded and then proves them (below). The release also ships
`fleet-aarch64-apple-darwin.tar.gz` for humans, and the feed does not name it.

## The CLI channel

`fleet update` runs `update::self_update`:

1. a dev build refuses (a checkout's binary is the build's to replace);
2. a **bundle** refuses by name — `Contents/MacOS/fleet-desktop` is the
   shell's, and the shell updates through its own window;
3. the feed is fetched, the version gap read (`same` is a quiet success,
   `behind` refuses — a feed must not pull a checkout down);
4. the artifact is downloaded under a 256 MiB cap and asserted against the
   feed's sha256;
5. the bytes are staged as `<exe>.update` with the running binary's mode;
6. **the staged file must run and answer `--version` with the feed's
   version** — this is the proof that used to be missing, and it is what makes
   a tarball, a truncation or a foreign build a named refusal instead of a
   corrupted binary;
7. the binary about to be replaced is copied to `<exe>.prev`;
8. one same-directory rename puts the artifact in place — atomic, and the
   running process keeps its own inode until the next restart.

The receipt names the version the feed installed, not the one the process was
compiled with: `fleet: updated in place to 0.6.6; the next restart serves it`.
launchd's restart contract puts the new inode on the port at the next restart.

The install leg takes its feed and artifact bytes through an injected
`update::Source`, so the tests drive it without a network — the same seam
lesson as KI-472, applied where the failure mode is data loss.

## The shell channel

The window loads the face from `http://127.0.0.1:<port>`, an origin Tauri
classifies as **remote**, so the capability must name it:

- `src-tauri/capabilities/default.json` lists `remote.urls =
  ["http://127.0.0.1:*"]` beside the window, grants `updater:default`, and
  grants the app's own `allow-restart-app` (`src-tauri/permissions/app.toml`);
- the shell binds its listener **once** and hands it to the server
  (`fleet::gui::serve_with`), instead of releasing the port and letting the
  server re-bind it — the port the capability trusts is never free for another
  process to take.

The press on the About pane calls the plugin's `check` → `download_and_install`
→ `restart_app`. On macOS the plugin installs and returns; the relaunch is the
app's own, which is why `restart_app` is a granted command rather than a side
effect.

A browser tab at `http://127.0.0.1:3080` is a different install: that server is
`~/.fleet/bin/fleet` (the CLI face), so its About pane applies through the CLI
channel. The pane always reports the mode of the build that serves it.

## The refusal and state vocabulary

- **not checked yet** — the pane claims nothing about the feed until the check
  answers; a settings read never asks the feed.
- **the feed names X — this build is behind** — with the apply press.
- **the check did not answer** — painted beside the press; no answer is never
  "up to date".
- **the feed did not answer** — the check answered with a refusal (unreachable,
  malformed, a ceiling crossed); the reason is carried verbatim.
- **a dev build does not update the checkout** — a target/ binary.
- **a bundled app updates through its own window** — the CLI channel refusing a
  bundle.
- **the staged artifact did not run / answers X, not the feed's Y** — the proof
  refusing before the rename.

## The cut

`scripts/release.sh tag vX.Y.Z` gates a detached worktree of HEAD, tags and
pushes. `scripts/release.sh assets vX.Y.Z` builds the dmg, the CLI tarball, the
raw update artifact and the signed `.app.tar.gz`, checks the version three
ways, generates `latest.json`, uploads, and then **reads the feed back
anonymously**: the published version must be the tag, the cli row must name the
raw artifact, the signature must be that file's sha256, and the served bytes
must hash to the signature.

## The chicken-and-egg

A fix to the shell channel is compiled into the bundle it delivers. A shell
that cannot reach the updater today cannot press its way to a shell that can:
the first release carrying the fix is installed by hand (dmg), and every
release after it is a button.
