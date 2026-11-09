# Session Recap — 2026-11-09: the updater that installs

**Summary.** The plan `docs/plans/2026-11-09-updater-that-installs.md` was
built and two releases were cut from it — 0.6.5 (the fix) and 0.6.6 (the
release the fix installs). The CLI channel is proven live twice: `fleet update`
on a 0.6.5 copy installed 0.6.6 and kept the old binary as `.prev`, and the
About pane's press in a browser did the same to `~/.fleet/bin/fleet`. The
shell channel's ACL is fixed in code and compiles as intended; its in-window
press is the one step this session could not run, because the machine
composited no windows (window capture by id and a full-screen capture both came
back empty while the page was loaded and holding its event stream). Two
defects were found and fixed on the way: the version bump that printed a lock
move it never wrote (KI-490), and a hand delivery that truncated the running
binary's inode and got the next exec SIGKILLed (KI-491).

## The plan's criteria

| Criterion | State | Evidence |
|---|---|---|
| C0 both failures reproduced | ✅ met | a scratch copy of the 0.6.3 binary run through `fleet update` ended as gzip (`file`: gzip, 9,991,743 bytes) while the installed binary stayed a Mach-O; the press half is the owner's own two presses plus the ACL trace in the deep dive — the pre-fix bundle was replaced by C8's hand install, so it cannot be re-run |
| C1 the raw artifact in the published feed | ✅ met | anonymous `curl` of `releases/latest/download/latest.json`: cli url basename `fleet-aarch64-apple-darwin`, its `signature` equal to the downloaded file's sha256 (0.6.5 `87b448fb2e95…`, 0.6.6 `579523cc113c…`), and the file answers `fleet 0.6.5` |
| C2 the install leg, the keep, the receipt | ✅ met | live twice (below); `update::tests::the_install_leg_runs_the_artifact_and_keeps_the_previous_binary` |
| C3 a wrong artifact refuses before the rename | ✅ met | `a_tarball_artifact_is_refused_before_the_rename`, `a_truncated_artifact_is_refused_before_the_rename`, `a_foreign_binary_is_refused_by_its_own_answer` |
| C4 a bundle refuses the CLI channel | ✅ met | `a_bundled_executable_refuses_the_cli_channel_by_name` |
| C5 the shell reaches the updater; no port race | ✅ met | `src-tauri/gen/schemas/capabilities.json` carries `remote.urls: ["http://127.0.0.1:*"]` and `allow-restart-app`; `the_face_listener_is_held_from_bind_to_serve` |
| C6 the pane claims nothing before the answer | ✅ met | harness `C6 the About pane says the feed has not been asked yet` and `C6 a failed check paints the refusal`; live: the pane answered `the feed names 0.6.6 — this build is behind` |
| C7 no plain-http transport | ✅ met | `grep -c dangerousInsecureTransportProtocol` = 0; the live check route answered the published version |
| C8 the two-cut live proof | ⏳ **partial** | CLI half ✅ (two proven installs); shell half pending one press on a machine with a display |
| C9 the closure | ⏳ open | follows C8 |

## The live proofs

**The CLI channel, from a copy.**

```
$ cp ~/.fleet/bin/fleet-0.6.5 /tmp/upd-c8/fleet && /tmp/upd-c8/fleet update
fleet: updated in place to 0.6.6; the next restart serves it (the previous binary is at /tmp/upd-c8/fleet.prev)
$ /tmp/upd-c8/fleet --version      → fleet 0.6.6      (Mach-O)
$ /tmp/upd-c8/fleet.prev --version → fleet 0.6.5
```

**The CLI channel, from the About pane.** The served 0.6.5 face's About pane
answered `the feed names 0.6.6 — this build is behind` with `update to 0.6.6`;
the press installed it over `~/.fleet/bin/fleet`:

```
$ ~/.fleet/bin/fleet --version      → fleet 0.6.6
$ ~/.fleet/bin/fleet.prev --version → fleet 0.6.5
$ curl :3080/api/version (after kickstart) → 0.6.6
```

**The feed, read back by the cut itself.** Both cuts end with an anonymous read
of the published `latest.json`, a download of the cli row's artifact, and a
hash against the row's signature — `uploaded: latest.json (the updater's
channel + the cli row)` then `the published feed names 0.6.6; the cli row
serves the raw binary, sha256 579523cc113c`.

## What blocks C8's shell half

The shell's window loads its face from the loopback origin and keeps the SSE
connection established (`lsof`: the fleet-desktop process and its WebKit
content process paired on the shell's port), but every capture of that window
came back empty — CUA's window raster, `screencapture -l <window id>`, and a
full-screen capture that showed only the desktop. The 0.6.4 bundle behaves the
same way, so it is the environment (no composited display), not the change. The
press needs a machine with a display: open the installed 0.6.5 app, About,
press `update to 0.6.6`.

## What was fixed on the way

- **KI-490** — `scripts/bump-version.sh`'s lock leg mutated a list, printed
  "moved: 2", and never wrote the file. Fixed: the write is in the script, and
  the 0.6.6 bump moved the lock through it (verified by grepping the lock in
  the same breath).
- **KI-491** — installing the CLI with `cp target/release/fleet
  ~/.fleet/bin/fleet` truncates the destination inode; macOS's signature cache
  is keyed to it, and the next `exec` was SIGKILLed (exit 137) over a file
  whose bytes were exactly right. `rm` + `cp` (or the updater's rename) is the
  step.

## Gotchas this session (the price of each)

- The capability's app permissions are referenced **unprefixed**
  (`allow-restart-app`); `fleet-desktop:allow-restart-app` fails the build with
  the list of accepted identifiers.
- The Tauri ACL's refusal message is compiled in debug builds only, so a
  release build's rejection is the generic text — the deep dive's quoted
  sentence is the debug shape.
- A window whose content is GPU-composited can be invisible to both window and
  screen capture while it is fully alive; `lsof` on the server's port is the
  proof that the page loaded.

## Files changed

**Backend** — `src/update.rs` (the `Source` seam, `Update`, `previous_for`,
`staged_version`, `self_update_over`, the bundle refusal, six tests),
`src/main.rs` (the receipt names the installed version), `src/gui/bridge.rs`
(installed/previous/installed_via on the apply answer), `src/gui/mod.rs`
(`serve_with`), `src/gui/server.rs` (`bind_on`).

**Shell** — `src-tauri/capabilities/default.json` (the loopback remote context,
`allow-restart-app`), `src-tauri/permissions/app.toml` (new),
`src-tauri/src/lib.rs` (bind once, hand the listener over, a test),
`src-tauri/tauri.conf.json` (the insecure-transport flag gone).

**Frontend** — `src/gui/assets/app.js` (the pane's pending/failed states, the
apply answer's fields).

**Scripts** — `scripts/release.sh` (the raw artifact, the feed row, the
anonymous read-back), `scripts/bump-version.sh` (the lock write).

**Tests** — `src/update.rs`, `src-tauri/src/lib.rs`, `tests/face/harness.mjs`
(1029 checks / 123 cases).

**Documentation** — `docs/plans/2026-11-09-updater-that-installs.md`,
`docs/features/updater.md` (new), the deep dive's §5b fix map, this recap,
`docs/known-issues.md` (KI-486..491), `CHANGELOG.md` (0.6.5 and 0.6.6).

## Open questions / next steps

- **The one press** (C8's shell half): open the 0.6.5 app, About, press
  `update to 0.6.6`; then `GET /api/version` on the shell's port and the
  bundle's `CFBundleShortVersionString` should both answer 0.6.6. Tick C8 and
  C9 then.
- The plan stays `active` until that press.
- G5 (the bundle and the CLI drifting apart) is still out of scope, as planned.
