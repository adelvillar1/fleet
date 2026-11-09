# Deep dive 2026-11-09: the app's self-update, read end to end

Research only — no plan, no code. The question: the owner asked what the Tauri
app's update functionality actually is. What is built, what happens on each
press, and what works.

**The answer in one paragraph.** Two channels shipped in 0.6.0 and neither
completes an update today. The shell's Tauri plugin chain is refused by the
Tauri ACL before it reaches the network — the window loads a loopback http
origin, which Tauri classifies as *remote*, and the capability that grants the
updater is compiled as *local only*. The CLI channel (the same route the About
pane's fallback posts to) downloads the release's **gzip tarball** and renames
those bytes over the running binary; the feed's hash is the tarball's own hash,
so every check passes and the swap destroys what it installs. The only install
that has ever worked is the manual one (copy the extracted binary, keep the old
one beside it). The check half — feed read, version gap, card, stamp — works
and is honest when it answers. The apply half is broken in both directions,
and one of them fails loudly only after the damage.

---

## 0. The owner's ruling (2026-11-09): the update comes from GitHub, never from the local machine

Recorded verbatim from the dive's answer: *"the update should be fetched from
github, never from the local machine."* Operationally that fixes the
provenance of every byte an update installs:

- the feed and the artifact are read from
  `github.com/adelvillar1/fleet/releases/...` and from nowhere else — no local
  path, no local HTTP server, no LAN peer, no file the owner drops beside the
  binary;
- no configuration may repoint the source: the endpoint is the pinned constant
  (`src/update.rs:35`) and the pinned config URL (`src-tauri/tauri.conf.json:45-47`),
  not a settings key, an environment variable or a flag;
- the machine is the **install target** and the **press surface** only. Writing
  the verified artifact to `<exe>.update` and renaming it is the install, not a
  source;
- verification stays where it is: the shell's minisign signature against the
  pinned public key, the CLI's sha256 against the feed's cli row. Both are read
  from GitHub, so both are only as good as the transport — which is why a
  plain-http endpoint is incompatible with this ruling (S1).

What the code does today against that: the sources are already GitHub-only —
`FEED` is a constant, the plugin's `endpoints` are pinned, nothing reads a
local artifact — and the one hole is `dangerousInsecureTransportProtocol: true`,
which permits a plain-http source and therefore a local or LAN one (filed as
KI-489). What the ruling does **not** decide: who may press. A press from the
loopback-served About pane still fetches every byte from GitHub; the ACL fix in
D2 is therefore free to stay minimal (grant the press surface, not the source),
while the port race in G4 is closed on its own terms.

## 1. The pipeline as designed

Five stages, one owner each. Read this as the contract the code is supposed to
satisfy:

| Stage | Owner | What it guarantees |
|---|---|---|
| version sites | `scripts/bump-version.sh` | `Cargo.toml`, `src-tauri/Cargo.toml`, `tauri.conf.json`, the lock's two entries move together |
| the cut | `scripts/release.sh tag` then `assets` | gate, tag, build, dmg, cli tarball, signed `.app.tar.gz`, `latest.json` |
| the feed | `scripts/release.sh:255-286` | one `latest.json` at the branch-permanent `releases/latest/download/latest.json` |
| the shell consumer | `tauri-plugin-updater` (`src-tauri/tauri.conf.json:41-51`) | check, verify minisign against the pinned pubkey, install, relaunch |
| the CLI consumer | `src/update.rs` (`fetch` → `version_gap` → `self_update`) | fetch, gap check, sha256 assert, stage beside the binary, one same-directory rename |

```
release.sh: cargo build → dmg + fleet-aarch64-apple-darwin.tar.gz + fleet.app.tar.gz(.sig)
        ↓
  latest.json  { version, notes, pub_date,
                 platforms.darwin-aarch64 { url: fleet.app.tar.gz, signature: minisign }
                 cli                      { url: fleet-aarch64-apple-darwin.tar.gz, signature: sha256 } }
        ↓                                              ↓
  shell: updater plugin                        CLI + face: update::self_update
  (minisign, whole bundle)                     (sha256, raw bytes over the binary)
```

The two consumers read the same document but different rows, and the rows
carry different signature kinds — minisign for the bundle, sha256 hex for the
cli row (`src/update.rs:142-159`). That asymmetry is deliberate and documented;
what is not comfortable is that the two rows also carry different *artifact
shapes*, and only one side of that difference is in the code.

## 2. The shell channel, line by line

### 2.1 How the window is served

`src-tauri/src/lib.rs:295-327` builds the window in code, with an external
URL:

```rust
tauri::WebviewWindowBuilder::new(&handle, "main", tauri::WebviewUrl::External(parsed))
```

where `parsed` is `http://127.0.0.1:<ephemeral port>` (`lib.rs:356-358`), the
port chosen by binding it, reading it, and **dropping the listener**
(`lib.rs:387-395`) before `gui::serve` binds the same number. The reason is
sound and recorded: a real loopback HTTP origin is what makes the PWA's
service worker register — Tauri's custom scheme is rejected by WebKit's
service-worker container on macOS (Tauri #13031, `lib.rs:351-355`).

### 2.2 The press

`pressApply` (`src/gui/assets/app.js:23580-23640`) has three branches:

1. `installed_via === 'dmg' && window.__TAURI__ && window.__TAURI__.updater`
   → `updater.check()` → `downloadAndInstall()` → `restartApp()`
   (`app.js:23581-23599`);
2. `installed_via === 'dev-build'` → a notice, no action (`app.js:23600-23605`);
3. everything else → `POST /api/update/apply` (`app.js:23607`), which is the
   CLI channel, run in the **server's** process `src/gui/bridge.rs:3248-3277`
   → `update::self_update(current_exe())`.

`restartApp` (`app.js:23643-23653`) invokes the app command `restart_app`
(`lib.rs:365-368`), falling back to `window.location.reload()`.

### 2.3 What the plugin needs, and what it gets

The plugin and its config are present and complete: `tauri-plugin-updater 2.14.1`
registered (`lib.rs:296`), `createUpdaterArtifacts: true` and the pinned pubkey
(`src-tauri/tauri.conf.json:39-51`), and `updater:default` in the capability —
which grants `allow-check`, `allow-download`, `allow-install`,
`allow-download-and-install` (`tauri-plugin-updater-2.14.1/permissions/default.toml`).
The global API script is injected unconditionally
(`tauri-2.12.5/src/manager/webview.rs:207-221`), and the installed bundle
proves it: `__TAURI__.updater` and `plugin:updater|check` are string literals
in `/Applications/fleet.app/Contents/MacOS/fleet-desktop`.

The gate is the **origin**, and it is closed:

1. `is_local_url` (`tauri-2.12.5/src/webview/mod.rs:1961-2000`) answers true
   for `tauri://localhost`, for anything relative to `devUrl`/`frontendDist`,
   and for user-registered custom protocols. `http://127.0.0.1:<port>` is none
   of those.
2. So the IPC request's origin is `Origin::Remote { url }`
   (`webview/mod.rs:2044-2050`).
3. `Origin::matches` never lets a `Local` context satisfy a `Remote` origin
   (`tauri-2.12.5/src/ipc/authority.rs:58-66`).
4. The capability is compiled with **no remote context**. The generated file
   says it outright — `src-tauri/gen/schemas/capabilities.json`:
   `"identifier": "default", "local": true`. `src-tauri/capabilities/default.json`
   carries only `windows: ["main"]` and the two permission sets; there is no
   `remote: { urls: [...] }`.
5. `resolve_access` therefore returns no ACL for `plugin:updater|check`, and
   `webview/mod.rs:2076-2098` rejects it: a plugin command with
   `invoke.acl.is_none()`. (The explanatory message is compiled in debug
   builds only — `authority.rs:221-416` — so a release build rejects with the
   generic text.)

The same applies to the app's own `restart_app`, because the app defines an
ACL manifest (`has_app_acl_manifest` in the same condition).

**What the owner sees:** `updater.check()` rejects, `pressApply`'s `.catch`
fires, and `notice('the update did not land', <acl text>)` paints into the
transcript pane — two panes from the About pane the press happened in. The
button re-enables. That is KI-483's exact symptom, one layer deeper than the
0.6.2 fix addressed: the CLI row is fixed, the *shell branch never reaches the
feed at all*.

This was never exercised live. The 0.6.0 cut proved the signing chain (a
signature whose embedded signer id matched the pinned key), and the 0.6.2
session verified the feed anonymously by hand; neither ran the plugin's
check/install/relaunch from inside the window.

## 3. The CLI channel, line by line

`self_update` (`src/update.rs:314-358`) is four steps and one refusal each:

1. `installed_via(exe) == MODE_BUILD` → refuse (a dev build replaces nothing);
2. `fetch()` → `version_gap` → `same` returns `"same"`, `behind` refuses;
3. `feed.row("cli")` → the url and the sha256 hex; `download(url, 120s)`
   (`src/update.rs:262-280`) → `Vec<u8>` under a 256 MiB cap;
4. `sha256(bytes) == expected` → write the bytes to `<exe>.update`, copy the
   exec bits, `rename(staging, exe)`.

Step 4 is where the contract and the release part company. The plan's words
(`docs/plans/2026-11-08-the-shell-updates-itself.md:86-89`) are "the release's
CLI artifact — the **raw `fleet` binary asset**". The release ships a tarball
(`scripts/release.sh:179-190`):

```sh
tar -czf "$TARBALL" -C "$PKGDIR" fleet-aarch64-apple-darwin
```

and the feed's cli row hashes that tarball (`release.sh:275`:
`sha(cli_tarball)`). Measured against the live v0.6.4 release:

```
cli url  .../download/v0.6.4/fleet-aarch64-apple-darwin.tar.gz
cli sha  76f8078197bde5d89e56d16b1373b9e1c8feee32187e601f68f1f02ca7c1dc76
file     gzip compressed data … original size modulo 2^32 26701824
size     9,991,743 bytes      (the feed's sha256 matches, byte for byte)
```

`self_update` has no unpack step and no shape check. Every assertion it makes
passes: the hash is right, the bytes are the artifact the feed named, the
rename succeeds. The binary at `~/.fleet/bin/fleet` (26,692,544 bytes, Mach-O
arm64) would be replaced by 9,991,743 bytes of gzip. The next exec fails, and
`update_command` (`src/main.rs:2749-2766`) still prints "updated in place; the
next restart serves 0.6.4", because the receipt is compiled in and nothing runs
the staged file.

Nothing has triggered it yet, and the machine proves it: `~/.fleet/bin/fleet`
is a Mach-O dated to its cut, the `fleet-0.6.1` … `fleet-0.6.4` backups sit
beside it from manual installs, and there is no stale `fleet.update` staging
file. `fleet update` has never completed an install on this machine.

## 4. What happens on a press today

**About pane in the shell.** `dmg` → plugin branch → ACL rejection → notice in
the transcript pane → nothing visible where the press happened. No network
call leaves the app.

**A browser tab on the served face.** `installed_via` is computed from the
*server's* executable (`src/gui/bridge.rs:9081`). For the LaunchAgent's
`~/.fleet/bin/fleet` that is `source`, so `pressApply` takes branch 3 →
`POST /api/update/apply` → `self_update` → **the tarball overwrite**. This is
the live hazard: 3080 is served by the CLI binary.

**`fleet update` in a terminal.** The same overwrite, with no confirmation, no
backup, and a success line.

**About pane opened from a browser on a shell-served port.** `installed_via`
is `dmg` (the server is the shell), `window.__TAURI__` is absent, so branch 3
runs and writes the tarball over `Contents/MacOS/fleet-desktop` — the bundle
itself, not the CLI. The About pane prints no channel name, so the press does
not say which of the two it will do.

## 5. Findings

Classified the way the house likes them: a confirmed defect has a reproduction;
a design gap is a decision for the owner.

### Confirmed defects

**D1 — the CLI channel installs a gzip tarball over the binary.** Evidence
above: `src/update.rs:336-356` (no unpack, no shape check, no exec probe) against
`scripts/release.sh:190` and `:275`, and the measured v0.6.4 artifact. Blast
radius: `~/.fleet/bin/fleet` in the normal path, `Contents/MacOS/fleet-desktop`
from a browser on a shell-served port. Irreversible: the rename is the last
instruction and no copy of the old binary is kept. Filed as KI-486.

**D2 — the shell's updater channel is refused by the Tauri ACL.** Evidence:
`src-tauri/src/lib.rs:304-308` (external loopback URL) →
`tauri-2.12.5/src/webview/mod.rs:1961-2000` (`is_local_url` false) →
`webview/mod.rs:2044-2050` (`Origin::Remote`) → `src-tauri/gen/schemas/capabilities.json`
(`"local": true`, no `remote`) → `ipc/authority.rs:58-66` (no match) →
`webview/mod.rs:2076-2098` (reject). `restart_app` is refused by the same
condition. Filed as KI-487.

**D3 — the About pane can claim "nothing newer" without asking the feed.** The
arrival handler paints from the settings read and then fetches
(`src/gui/assets/app.js:24720-24736`); the settings document *never* carries
`candidate` or `refused` (`src/gui/bridge.rs:9075-9085` — "a settings read
never asks the feed"), so the first paint always falls into
`app.js:23551-23556`'s else branch: "the feed names 0.6.4 — nothing newer". If
the check request itself fails, the `.catch` is empty and that line stays. The
plan's own honesty clause (`docs/plans/2026-11-08-the-shell-updates-itself.md:70-72`)
forbids exactly this: no answer must never read as up to date. Filed as KI-488.

**D4 — the plan, the ledger and the recap assert a working hop that never ran.**
The plan ticks "The CLI channel: `fleet update` verifies, atomically replaces
its own binary…" (`…the-shell-updates-itself.md:99`) on code-reading evidence;
KI-483's Next action says "its working hop is the Tauri-native updater in the
dmg shell", and the 0.6.2 recap repeats it. Per D2 that hop is refused, and per
D1 the CLI hop destroys the binary. This is the D3-shaped failure at the level
of claims rather than paint: a statement of fact with no run behind it.

### Design gaps (a decision, not a bug)

**G1 — `FEED` is a constant with no injection seam.** `src/update.rs:35`. The
install half therefore cannot be tested without the network: the unit tests
stop at `parse`, `version_gap`, `staging_for` and the feed shapes
(`src/update.rs:360-585`), and the face harness scripts the two routes with
fake bodies (`tests/face/harness.mjs:899-921`). This is KI-472's shape — "a
reader with no injection seam" — on the one path where the failure mode is
data loss rather than a red test.

**G2 — nothing verifies the staged artifact before the rename.** The release
script already proves the packaged binary answers `--version`
(`scripts/release.sh:184`). The client could do the same with the staged
`<exe>.update`: exec it, compare its `--version` to the feed's `version`, and
refuse on any disagreement. That one step catches D1, a truncated download, and
a wrong-architecture artifact before the rename.

**G3 — no rollback.** Neither channel keeps the previous binary. The manual
installs did (`fleet-0.6.1` … `fleet-0.6.4` beside the CLI). A copy or a
`*.prev` rename beside the staging file would make a bad cut recoverable from
the machine.

**G4 — fixing D2 by granting `remote` widens the IPC surface to *any* process
on that port.** The port is bound, read and released before `gui::serve`
re-binds it (`lib.rs:387-395`) — a toy race, but the capability would be
granting IPC to `http://127.0.0.1:*`. The minisign check still gates *content*
(a hostile page cannot install an unsigned bundle), so the exposure is
trigger-and-restart, not code execution; still, the two lawful fixes are
different: grant `remote` and accept the surface, or stop using an external
origin (serve the shell's window from `tauri://` and let the page talk to the
loopback API cross-origin, which costs the service-worker condition the
loopback origin was chosen for). The port race should be closed either way —
hold the listener and hand it to the server. Neither option changes where the
bytes come from: under the 2026-11-09 ruling (§0) both fetch from GitHub, so
this is a question of attack surface, not of provenance.

**G5 — the two installs drift.** Updating the bundle does not update
`~/.fleet/bin/fleet`, and updating the CLI does not update the bundle. The
About pane always reports the *server's* mode, so a browser tab reports the
CLI's mode and the shell window reports the bundle's; neither can see the
other. On this machine the LaunchAgent serves 3080 from the CLI binary while
the owner's window is the bundle.

### Small

**S1 — `dangerousInsecureTransportProtocol: true`**
(`src-tauri/tauri.conf.json:44`) for a feed and artifacts that are all HTTPS.
Under the owner's 2026-11-09 ruling (§0) this is not a tidiness note: the flag
is exactly what would let the update be fetched over plain http — from a local
or LAN source — so it contradicts the provenance rule and must go with the
next shell change. Filed as KI-489.

**S2 — pre-releases compare equal to their release.** `parse_version` drops the
`-rc.1`/`-beta.1` suffix (`src/update.rs:90-105`), so `0.6.0-rc.1` against a
feed naming `0.6.0` is `same` — a pre-release user is never moved forward. Harmless
while the project ships none.

**S3 — the feed's notes are generic.** `latest.json` always says "See the
release page for what changed" (`scripts/release.sh:267`), so the About pane's
one-line note is the same sentence every release. The changelog head section is
already extracted for the release body (`release.sh:121-129`) and could be the
feed's first line.

**S4 — the cut verifies its feed by leg read, not by fetch.**
`release.sh:283-285` re-reads the local file; the anonymous read of
`releases/latest/download/latest.json` was done by hand in the 0.6.2 session.
An upload is not a publication until something other than the uploader reads
it.

**S5 — after an apply the field drops `installed_via`.**
`src/gui/bridge.rs:3260-3266` returns `{applied, serving}` and merges it into a
fresh settings document, whose `update` key is then overwritten wholesale —
`paintAbout`'s `via` falls back to `source`. Only the browser-face path reaches
this (the shell takes the plugin branch), which is the path already misreporting
the mode.

## 6. Assumptions to confirm

1. **The artifact's shape.** The plan says raw binary; the release ships a
   tarball; the client assumes raw. Which is authoritative? Unpacking in the
   client (and hashing the tarball) is the smaller change and keeps the
   download compressed; shipping a raw binary asset makes the client's current
   code correct and the release slightly larger. This is the owner's call, and
   the answer decides where the fix lands.
2. **The window's origin.** Grant `remote` to the loopback origin, or move the
   window to `tauri://` and take the service-worker cost. Both are safe for
   *content* (minisign); they differ in surface and in what the PWA can do.
3. **Whether the shell should update at all while the CLI serves 3080.** If the
   owner's daily face is the CLI binary under launchd, the bundle's updater
   updates the window the owner is not using. G5 is a product question before
   it is a code question.

## 7. What a fix must touch (a sketch, not a plan)

- `src/update.rs`: unpack or shape-check the artifact; run the staged binary's
  `--version` against the feed's version before the rename; keep the previous
  binary; a `FEED` seam so the install leg has a test.
- `src-tauri/capabilities/default.json`: the origin decision (D2), with
  `restart_app` named in the same capability — and remember §0: the grant
  widens who may press, never where the bytes come from.
- `src-tauri/src/lib.rs`: close the port race — bind once, pass the listener to
  the server.
- `src/gui/assets/app.js`: the About pane's pre-check paint (D3) must say
  "not checked yet" or nothing, never "nothing newer"; name the channel the
  press will use.
- `scripts/release.sh`: whichever artifact shape the owner picks; the feed's
  notes line; an anonymous read-back of the feed after upload.
- `src-tauri/tauri.conf.json`: drop `dangerousInsecureTransportProtocol`
  (S1/KI-489) — the provenance rule's one code hole.
- The ledger's KI-483 row and the plan's criterion: corrected to their real
  verification state (D4).

## 8. Evidence log

```
$ curl -sL .../releases/latest/download/latest.json
version 0.6.4
platforms ['darwin-aarch64']      cli url …/fleet-aarch64-apple-darwin.tar.gz
cli sha 76f8078197bde5d89e56d16b1373b9e1c8feee32187e601f68f1f02ca7c1dc76
$ file fleet-aarch64-apple-darwin.tar.gz
gzip compressed data … original size modulo 2^32 26701824     (9,991,743 bytes)
$ file ~/.fleet/bin/fleet
Mach-O 64-bit executable arm64                                (26,692,544 bytes)
$ ls ~/.fleet/bin
fleet  fleet-0.6.1  fleet-0.6.2  fleet-0.6.3  fleet-0.6.4      (manual installs; no .update)
$ ls /Applications/fleet.app/Contents/MacOS
fleet-desktop                                                 (33,721,920 bytes)
$ strings /Applications/fleet.app/Contents/MacOS/fleet-desktop | grep __TAURI__
__TAURI__  __TAURI__.core  __TAURI__.core.invoke  __TAURI__.updater  __TAURI__.updater.check
$ cat src-tauri/gen/schemas/capabilities.json
{ "default": { …, "local": true, "windows": ["main"],
              "permissions": ["core:default", "updater:default"] } }
```

Tauri's own source, read at the pinned versions (`tauri 2.12.5`,
`tauri-plugin-updater 2.14.1`), is the authority for §2.3: `is_local_url`,
`Origin::Remote`, the `matches` matrix and the plugin-command ACL gate.
