---
status: completed
created: 2026-11-06
updated: 2026-11-07
slug: readme-install-and-tui-images
---

# Plan: the README's install guide and the TUI images

## Context

The README tells the arc, the surfaces and a four-line "Start here", but
it has no installation guide. The Tauri dmg — the prebuilt native shell,
built at `target/release/bundle/dmg/fleet_0.3.0-beta_aarch64.dmg` — is
undocumented; building from the repo is one bare `cargo build --release`
with no prerequisites; and the terminal face, one of the three faces,
has no image anywhere in the repo (all 22 under `docs/images/` are the
browser face). The owner's ruling: add the installation instructions,
the build-from-source, the Tauri binary's use, and the TUI images.

## Approach

1. The README's **Install** section, before "Start here":
   - The prebuilt Tauri binary: `cargo tauri build` produces the dmg;
     drag it to `/Applications`; the first run provisions the home.
   - Building from the repo: the rust toolchain and the tauri CLI,
     `cargo build --release` (the `fleet` binary), `cargo tauri build`
     (the dmg), and what each artifact is for.
   - `fleet install`: the home layout and the optional lanes.
2. The TUI images: a capture script drives the terminal face in a tmux
   pane, captures each screen's text with its colours, and renders it to
   a PNG beside the other surfaces; the README gains a terminal-face
   section with the shots and their captions.

**Decision-model check.** None — a docs and script change; the capture
is a dev tool, not a fleet dependency.

High-level steps:

1. `scripts/tui-shots.py` — the capture and the render.
2. The captured images under `docs/images/`.
3. The README's Install and terminal-face sections.
4. The install and TUI docs carry the same build commands the README
   names, so the three surfaces cannot drift.
5. The README's image links resolve and the gate's fmt check exits 0.

## Acceptance criteria

- [x] C0 — the README's Install section covers the prebuilt Tauri dmg
      (where it is, how to install it, how to run it), building from the
      repo (the prerequisites, the two build commands, what each
      artifact is), and `fleet install`.
- [x] C1 — the terminal face has captured images under `docs/images/`,
      each referenced from the README with a caption.
- [x] C2 — `scripts/tui-shots.py` reproduces the images from a scratch
      home.
- [x] C3 — the install and TUI docs carry the same build commands the
      README names.
- [x] C4 — the README's image links resolve and the gate's fmt check
      exits 0.

## Files to be touched

**Frontend:**
- `README.md` — the Install and terminal-face sections.

**Scripts:**
- `scripts/tui-shots.py` — the capture and the render.

**Documentation:**
- `docs/features/install.md` — the build-from-source and the dmg.
- `docs/features/tui.md` — the shots.

**Images:**
- `docs/images/tui-*.png` — the captured screens.

## Out of scope

- The release workflow (the CI stays the gate; the dmg is built locally).
- The browser face's shots (already present).
- The Tauri bundle's notarisation.

## Verification

**C0 — the Install section.**
Expected: the README names the dmg's path, the install, the two build
commands and their artifacts, and `fleet install`.

**C1/C2 — the images.**
```bash
python3 scripts/tui-shots.py
```
Expected: the PNGs land under `docs/images/` and the README references
each.

**C3 — the docs.**
Expected: the install and TUI docs carry the same commands.

**C4 — the gate.** A docs-and-script change; `cargo fmt --all --check`
and the README's links resolve.

## Linked artifacts

- `README.md` — the Install and terminal-face sections.
- `docs/features/install.md` — the build-from-source and the dmg.
- `docs/features/tui.md` — the shots.

## Notes

- The dmg: `cargo tauri build` → `target/release/bundle/dmg/
  fleet_0.3.0-beta_aarch64.dmg` (tauri-cli 2.12.4).
- The capture uses tmux (3.8) and Pillow (12.3.0), both on the machine;
  the script is a docs tool, not a fleet dependency.
