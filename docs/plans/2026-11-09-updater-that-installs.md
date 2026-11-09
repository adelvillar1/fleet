---
status: active
created: 2026-11-09
updated: 2026-11-09
slug: updater-that-installs
---

# Plan: the updater that actually installs

## Context

0.6.0 shipped the self-updater; the owner's report after two attempts is *"it
told me that I needed to restart the app and nothing happened."* The 2026-11-09
deep dive (`docs/architecture/deep-dive-2026-11-09-tauri-updater.md`) found that
neither channel completes an install: the CLI channel writes the release's gzip
tarball over the running binary (KI-486), the shell's plugin chain is refused by
the Tauri ACL before the network because the window's loopback origin is
`Origin::Remote` while the capability compiles local-only (KI-487), and the
About pane can claim "nothing newer" without a feed read (KI-488). The owner's
same-day ruling fixes provenance: every update byte comes from GitHub, never
from the local machine, which makes the plain-http transport flag the one code
hole (KI-489). The installed bundle is 0.6.3 while the feed names 0.6.4, so the
owner is visibly behind on the surface that is supposed to fix himself.

## Approach

One path, sized to the findings. The release ships the CLI's update artifact as
a **raw binary asset** (`fleet-aarch64-apple-darwin`) beside the tarball humans
use, and the feed's `cli` row names that raw file — which is what the plan that
built this (`docs/plans/2026-11-08-the-shell-updates-itself.md:86-89`) always
said it did. The client keeps its write-then-rename model and gains the proof it
never had: the staged file must **execute and answer the feed's version** before
the rename, the replaced binary survives as `<exe>.prev`, and a bundled
executable refuses by name (the `.app` updates through its own window).
`update::self_update` takes its feed through an injected seam, so the install leg
has a test that never touches the network (KI-472's lesson, applied where the
failure mode is data loss).

The shell keeps the owner's press surface. The capability gains the loopback
origin as a `remote` context plus the app's own `restart_app`, and the shell
stops releasing its listener between bind and serve — it binds once and hands
the bound listener to the server, so the origin the capability grants is the
origin the shell itself serves. The About pane gains the third state it lacks:
nothing is claimed about the feed until the check answers, and a failed check
paints the refusal where the press stands. `dangerousInsecureTransportProtocol`
is removed; the endpoints stay the pinned HTTPS GitHub URLs.

High-level steps, each closing the criteria it names:

1. **C0** — reproduce both failures on scratch copies and record them (the
   installed binary and bundle are not touched).
2. **C1** — `release.sh` builds and uploads the raw binary asset, hashes that
   file into the feed's `cli` row, and reads the published feed back anonymously.
3. **C2, C3, C4** — `self_update` gains the exec-and-version probe, the `.prev`
   keep, the bundle refusal, the receipt's version, and the injected feed seam,
   each with its test.
4. **C5** — the capability's loopback grant and `restart_app`, and the listener
   handoff in `src-tauri/src/lib.rs`.
5. **C6** — the About pane's pre-check and failed-check states, pinned by the
   face harness.
6. **C7** — drop `dangerousInsecureTransportProtocol`; prove the feed still reads
   over HTTPS.
7. **C8** — the live proof, forced into two cuts by the chicken-and-egg (the
   fix is compiled into the bundle it delivers).
8. **C9** — close the ledger rows and record the fixes in the deep dive.

## Acceptance criteria

These are the contract; each one is a verifiable assertion. The work is done
when every box is checked.

- [x] C0 — Both failures are reproduced before any fix and recorded in the recap: a scratch copy of the installed binary run through `fleet update` ends as a gzip file while the installed binary and the bundle are untouched, and a press in the installed shell changes neither the About pane nor the bundle.
- [x] C1 — The next cut publishes the CLI update artifact as a raw binary and the feed names it: an anonymous fetch of `releases/latest/download/latest.json` returns a `cli` url whose basename is `fleet-aarch64-apple-darwin` (no archive suffix), and that url's `signature` equals the downloaded file's own sha256.
- [x] C2 — The install leg is driven end to end through the injected feed seam: an artifact that answers `--version` with the feed's version is installed by one same-directory rename, the replaced binary survives beside it as `<exe>.prev`, and the printed receipt names the installed version, not the running one.
- [x] C3 — An artifact that does not answer `--version` with the feed's version is refused before the rename, the running binary is unchanged, and the refusal names the artifact's own answer — pinned for the three shapes the deep dive named (the gzip tarball, a truncated file, a foreign binary).
- [x] C4 — `self_update` refuses a bundled executable by name, naming the shell's own window as the channel, so no route can write a CLI binary over `Contents/MacOS/fleet-desktop`; pinned by a test on a bundle-shaped path.
- [x] C5 — The shell's window reaches the updater: the generated capability names the loopback origin as a remote context and grants the app's `restart_app`, and the shell passes its bound listener to the server instead of releasing and re-binding the port; pinned by the generated `capabilities.json` and by a test that the listener is not dropped before serve.
- [x] C6 — The About pane claims nothing about the feed before the check answers: a fresh arrival paints "not checked yet", and a check request that fails paints the refusal beside the press; both pinned by the face harness.
- [x] C7 — `dangerousInsecureTransportProtocol` is absent from the shell config and the feed still reads over the pinned HTTPS endpoint, proven by a live `GET /api/update/check` that answers the feed's version.
- [ ] C8 — The live proof: 0.6.5 is cut with these fixes and installed by hand once, because 0.6.4's compiled-in ACL cannot reach the updater; then 0.6.6 is cut and the 0.6.5 shell's About press installs it, the app relaunches, and `GET /api/version` on the shell's port answers 0.6.6, while `fleet update` on a copy of the 0.6.5 CLI installs 0.6.6 and leaves the 0.6.5 binary beside it as `.prev`.
- [ ] C9 — The ledger closes KI-486, KI-487, KI-488 and KI-489 with their evidence, each deep-dive finding records the fix that answered it, and `python3 scripts/ki_ledger.py` and `python3 scripts/plan_check.py` both report zero problems at status: completed.

## Progress (2026-11-09, v0.6.5 → v0.6.6)

C0–C7 are ticked from this session's own evidence. C8's **CLI half is proven
live**: `fleet update` on a copy of the 0.6.5 binary installed 0.6.6 and kept
`fleet.prev` at 0.6.5, and the About pane's press in a browser installed 0.6.6
over `~/.fleet/bin/fleet` the same way. C8's **shell half is not**: this
session's machine composited no windows (a window capture by id and a
full-screen capture both came back empty while the page was loaded and holding
its event stream), so the in-window press could not be aimed. The one
remaining step is a press on a machine with a display: open the installed
0.6.5 app, open About, press `update to 0.6.6`; it should download, install,
relaunch, and serve 0.6.6. C9 follows it.

## Files to be touched

**Backend / API:**
- `src/update.rs` — the injected feed seam; the exec-and-version probe before the rename; `<exe>.prev`; the bundle refusal; the receipt's version; tests for each.
- `src/main.rs` — the `update` receipt prints the version the feed installed.

**Frontend:**
- `src/gui/assets/app.js` — the About pane's pre-check state and the failed-check paint.
- `src/gui/bridge.rs` — the apply answer keeps `installed_via` so the pane's channel line cannot fall back to `source`.

**Desktop shell:**
- `src-tauri/capabilities/default.json` — the loopback origin as a remote context; `restart_app`.
- `src-tauri/src/lib.rs` — bind once, hand the listener to the server; a test for the handoff.
- `src-tauri/tauri.conf.json` — drop `dangerousInsecureTransportProtocol`.

**Scripts:**
- `scripts/release.sh` — build and upload the raw binary asset; the feed's `cli` row hashes it; the anonymous read-back.

**Tests:**
- `src/update.rs` tests — the seam-driven install, the three refusals, the `.prev`, the bundle refusal.
- `tests/face/harness.mjs` — the two About-pane cases (fresh arrival, failed check).

**Documentation:**
- `docs/known-issues.md` — the four rows closed with evidence.
- `docs/architecture/deep-dive-2026-11-09-tauri-updater.md` — each finding records its fix.
- `docs/features/updater.md` — new: the two channels, the proofs, the refusals, the provenance rule.
- `CHANGELOG.md` — the 0.6.5 section.

## Out of scope

- Auto-update on launch: the card stays the consent surface (the 0.6.0 plan's non-goal).
- The bundle/CLI drift (G5 in the deep dive): updating one install does not update the other; that is a product decision for its own plan.
- A native-menu update press: the About pane stays the surface the owner uses.
- Notarization and stapling of the bundle; the minisign signature is the update gate.
- Windows and Linux platform rows, and downgrade support.

## Verification

**C0** — `cp ~/.fleet/bin/fleet /tmp/upd-probe/fleet && /tmp/upd-probe/fleet update` on the pre-fix binary; `file /tmp/upd-probe/fleet` reports gzip; `file ~/.fleet/bin/fleet` still reports Mach-O; `plutil -extract CFBundleShortVersionString` and `stat` on `/Applications/fleet.app` are unchanged after a press.

**C1** — after the 0.6.5 cut: `curl -sL .../releases/latest/download/latest.json`, then `curl -sL <cli url>` and compare `shasum -a 256` to the feed's `cli.signature`; the basename carries no suffix.

**C2** — `cargo test -p fleet --lib update::` — the seam test installs a staged artifact that answers the feed's version, asserts the rename, `<exe>.prev`'s content, and the receipt string.

**C3** — `cargo test -p fleet --lib update::` — one test per refused shape, each asserting the tree is unchanged and the refusal quotes the artifact's answer.

**C4** — `cargo test -p fleet --lib update::` — the bundle-path test asserts the named refusal and that nothing is written beside it.

**C5** — `python3 -c "import json;d=json.load(open('src-tauri/gen/schemas/capabilities.json'));print(d['default'].get('remote'))"` names the loopback origin, and the shell's test asserts the listener reaches the server bound.

**C6** — `node tests/face/harness.mjs` — the two new cases, and the existing KI-483 cases stay green.

**C7** — `grep -c dangerousInsecureTransportProtocol src-tauri/tauri.conf.json` is 0; `curl -s localhost:<port>/api/update/check` answers the published version.

**C8** — the two cuts, the hand install, the press, then `curl -s localhost:<shell port>/api/version` and `<copy> --version`; the previous binary beside it.

**C9** — `python3 scripts/ki_ledger.py`, `python3 scripts/plan_check.py`, both exit 0, and the four rows read closed with their evidence.

## Linked artifacts

- `docs/features/updater.md` — new document: the two channels, the artifact contract, the three refusals, the provenance rule, the chicken-and-egg.
- `TECHNICAL-DOCUMENTATION.md` — a module contract for `src/update.rs` and the shell's update boundary; the tech doc carries no updater section today (an arch-gate `UNDOCUMENTED` area, named here rather than left silent).
- `FUNCTIONAL-SPECIFICATIONS.md` — the About pane's update states (not checked, checking, behind, refused, applied).
- `CHANGELOG.md` — the 0.6.5 section, written at cut time.
- `docs/architecture/deep-dive-2026-11-09-tauri-updater.md` — D1-D4 record the fix beside the finding.

## Risks

- **The capability grant widens IPC to the loopback origin.** Closing the port race in the same change is what keeps the grant narrow; the minisign check still gates content, so the residual is trigger-and-restart, not code execution. If the owner prefers the narrowest cut, a fixed shell port would let the capability name one exact origin instead of `127.0.0.1:*`.
- **The chicken-and-egg forces one manual install.** 0.6.4's compiled-in ACL cannot reach the updater, so 0.6.5 is installed by hand and every release after it is a press. The plan says so rather than promising the button works from 0.6.4.
- **The exec probe could be blocked by the OS.** A downloaded file carries no quarantine xattr (the app writes it), but if macOS refuses to exec it the refusal names that, and the install stops before the rename — a named degradation, not a silent broken binary.
- **The updater plugin replaces the whole bundle.** The new bundle must still carry the frozen `services/` tree; the cut's own leg read proves it, and a bundle that lost it would be a cut-time failure rather than an update-time one.
- **A raw asset doubles the transferred bytes** (26.7 MB against 10 MB). Acceptable for a solo tool on a loopback-fed app; the memory cap already covers it.

## Dependencies

- `docs/architecture/deep-dive-2026-11-09-tauri-updater.md` — the findings this plan answers, and the owner's provenance ruling recorded as its section 0.
- KI-486, KI-487, KI-488, KI-489 — the defect rows the plan closes.
- `docs/plans/2026-11-08-the-shell-updates-itself.md` — the plan that built the surface; its criterion for the CLI channel is superseded here (it was ticked on code reading).
