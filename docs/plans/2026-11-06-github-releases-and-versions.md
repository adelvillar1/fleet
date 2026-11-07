---
status: active
created: 2026-11-06
updated: 2026-11-06
slug: github-releases-and-versions
---

# Plan: GitHub releases — real binaries, real versions, a real changelog

## Context

Fleet has shipped every day since 2026-10-12 (492 commits, all through the
gate) without ever cutting a release: `git ls-remote --tags origin` is empty,
`gh release list` is empty, and the only place a version number exists is a
stalled literal — `version = "0.3.0-beta"` in `Cargo.toml`,
`src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json`, printed by
`fleet --version` (`src/main.rs` `Command::Version` → `CARGO_PKG_VERSION`),
carried into the dmg filename by the bundler. The 2026-11-02 native-shell
plan named GitHub releases as the missing package step ("if people want to
use them fine, if not that's also fine") and the owner has now asked for it.

Two standing facts shape the design:

1. **CI is disarmed by owner ruling (2026-10-23)** — the GitHub Actions
   budget is exhausted. Releases are therefore built locally by a script
   that runs the gate first and refuses to cut a release from a tree that
   has not passed it; no workflow is added.
2. **The gate runs on the committed tree (2026-10-22/23)** — a working-tree
   green hid uncompilable files once. The release script must gate a
   detached worktree of the tag, not the working directory.

The versioning question has one real decision: what "beta" means when the
owner says "real increments". The design adopted — a stable semver
major.minor.patch plus a beta epoch — is argued in the Decisions section at
the end. (Owner amendment during drafting: the tag format is stable, no
`beta` suffix — `v0.4.0`, full stop. The version literal stays
`0.4.0-beta.1` while the beta epoch runs, because that is what the binary is,
and it does not collide with the stable tag.)

## Criteria

The deliverable is **v0.4.0: cut, gated, published, delivered** — and the
machinery that makes it repeatable.

- [ ] **C1 — `scripts/release.sh` gates before it cuts.** Running
  `scripts/release.sh tag` on a tree whose HEAD has not passed the four-leg
  gate on a detached worktree of that HEAD exits non-zero and cuts nothing;
  with the gate green it creates the annotated tag `v0.4.0` and pushes it.
  The gate's four legs run at the repo root of the detached worktree, the
  working directory is restored unmodified, and the script refuses to run
  when the working tree is dirty. The gate is skipped only via
  `--skip-gate` (explicit on the command line, never defaulted).
- [ ] **C2 — cutting bumps every version site in one commit.** One
  `scripts/bump-version.sh OLD NEW` (plain sh, both versions passed as
  arguments) edits the three version sites (`Cargo.toml`, `src-tauri/Cargo.toml`,
  `src-tauri/tauri.conf.json`) and the lockfile, greps the repo for the old
  version and leaves only intentional sites, and its output lists what it
  touched. After it runs, `grep -rn '"0.3.0-beta"\|0\.3\.0-beta' Cargo.toml
  src-tauri/ --include="*.toml" --include="*.json"` prints nothing.
- [ ] **C3 — the changelog is the release note, in one place.**
  `CHANGELOG.md` exists, its head section is the version being released
  (`## v0.4.0 — 2026-11-06`), it is written from this session's landed
  increments (the open-ledger closeout: 28→5 ledger rows, KI-463's request
  path, the KI-464 wait, the gate-procedure rule, the dmg), and
  `gh release create v0.4.0 --title v0.4.0 --notes-file <(head section)` —
  so the changelog's head IS the GitHub release note; no text is written
  twice or diverges between the two.
- [ ] **C4 — the tagged tree is gated, the tag is proven, and the
  provenance is greppable.** The release build runs from a detached worktree
  at `v0.4.0`; the lib gate log (`/tmp/gate-lib-release-*.log`) names
  `1929 passed`; the build output names the dmg; and `git show v0.4.0` on the
  repo shows the object pointing at the commit whose tree the gate's logs
  recorded.
- [ ] **C5 — the release carries real binaries.** `gh release view v0.4.0`
  lists both the dmg (`fleet-0.4.0-beta.1_aarch64.dmg`) and the CLI
  (`fleet-aarch64-apple-darwin.tar.gz`), the assets download successfully
  (`gh release download` to a scratch dir), and after
  `git shortlog -s v0.4.0` shows the release's span, the CLI tarball's
  `fleet` binary passes `fleet --version` and names `0.4.0-beta.1`.
- [ ] **C6 — the face never serves a dev build again.** Post-cut, the
  installed release (dmg installed, or the CLI landed through `fleet reload`
  on this machine — whichever the owner runs) passes
  `fleet --version` → `0.4.0-beta.1` and
  `~/.fleet/bin/fleet --version` → `0.4.0-beta.1`, verified after a reload
  so the live face is genuinely the tagged build.
- [ ] **C7 — the release reads as a release.** The README's install section
  names the GitHub release as the way in (a URL to
  `https://github.com/adelvillar1/fleet/releases/latest`), and the release
  page itself reads as a release page, with no placeholder text.
- [ ] **C8 — the release is repeatable and documented.** `scripts/release.sh`
  carries a usage comment naming its mode (`tag`), its `--skip-gate` flag,
  and `gh` as a prerequisite; and a later cut (`v0.4.1` or `v0.5.0`) is the
  same script at a new version number, with no step that exists only in this
  session's transcript.

## The next versions are decided now

The owner said "real increments" — the increments are real and named. Cut
order after v0.4.0, each cut gated before it ships:

- **v0.4.1 — patch.** Fixes only (any ledger regression from the 5 open
  rows, or a delivery seam slip).
- **v0.5.0 — minor.** One real surface increment with something to show:
  the first post-ledger feature class, cut when a slice lands that changes
  what the face can do (the plugin host's own GA moment is the natural
  first).
- The rule for which: **fixes cut patch, features cut minor, and breaking
  the CLI/home contract — when beta ends — cuts major.** The beta epoch
  ends when the owner says so, with a `CHANGELOG.md` header saying what
  beta meant.
- The changelog keeps a section per release, written at cut time; the
  changelog is the release note (C3) and the owner re-reads it at every
  cut, since the changelog becomes the release surface that GitHub shows.

## The gate the release script runs (C1, verbatim)

The four-leg gate is the standing one (owner ruling 2026-11-01, the hard
local gate; never piped so the exit code is read on cargo's own invocation):

```sh
cargo build --all-targets 2>&1; echo "EXIT=$?"
cargo test 2>&1 | tail: read the summary and EXIT from the log
python3 scripts/ki_ledger.py > /dev/null; echo "EXIT=$?"
python3 scripts/plan_check.py > /dev/null; echo "EXIT=$"
```

*(the script's implementation is the exact gate this repo ran on the
committed tree this session — build --all-targets, the full lib suite, the
integration suite, and the face harness — with each leg's exit read from its
own log; the plan text names the legs, the script runs them)*

## Decisions

**Tag format: `v0.4.0` — stable, no beta suffix.** Owner amendment during
drafting. The suffix was rejected for matching nothing real: releases are
not beta and not not-beta; they are versions. The version literal stays
`0.4.0-beta.1` while the beta epoch runs — one fact it has, one it doesn't.

**`0.4.0`, not `0.3.x`:** the 0.3.0-beta literal has been parked for the
project's first month; the ledger closeout is the increment that ended that
pause, so the first release is the next stable minor — the increment number
is the release number.

**The version literal keeps `beta` while the beta epoch runs.** The binary's
own word for itself is what `fleet --version` names, and what ships today is
a beta — the owner's own framing is the tool he uses daily, not a product
promising API stability. Cutting releases does not end the beta; only the
owner's word does, and the changelog header carries the meaning of beta
until then.

**GitHub releases over asset uploads to the repo.** The tag carries the
source; the release carries the binaries; the changelog carries the notes.
Each artifact has its home, and none is asked to do another's job.

**A local script, not Actions.** CI is disarmed by ruling; a script that must
gate green before it cuts delivers the same guarantee on the owner's machine,
where the gate is already the working instrument.
