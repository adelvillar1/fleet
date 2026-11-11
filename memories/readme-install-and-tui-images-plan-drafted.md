---
name: readme-install-and-tui-images-plan-drafted
description: "project 2026-11-06 — the README install guide + the terminal face's images IMPLEMENTED 5/5 and MERGED to origin/master (24ad44b cherry-picked, carried through the merge to eb57d8f): the Install section (the prebuilt Tauri dmg, building from the repo, fleet install), six TUI screens captured at 120×40 from a scratch home by scripts/tui-shots.py, and the capture lesson — tmux's own C-p key names do NOT land in the TUI's raw mode, raw hex bytes (-H 10) do, and each screen was verified by signature before the r…"
type: project
date: 2026-11-11
basis: inferred
tier: durable
shape: []
phrases: []
missing: []
premises: []
recall-count: 0
superseded-by:
valid-until:
source: zcode:fleet:content:285983102772d99aaed2c496d57acdf6f320aad7347d3891e9bb9e2378839d34
---

The owner asked (2026-11-06) for a plan to add installation instructions,
building from the repo, or using the Tauri binaries to the README, plus
images for the TUI. The plan
(`docs/plans/2026-11-06-readme-install-and-tui-images.md`) is **IMPLEMENTED
5/5** (commit `b7bd882`) and **MERGED to origin/master** (cherry-picked as
`24ad44b`, then carried through the full merge to `eb57d8f`; see
[[docs-install-tui-branch-state]]).

**What landed.**

- The README's **Install** section, before "Start here": the prebuilt
  Tauri binary (`cargo tauri build` →
  `target/release/bundle/dmg/fleet_0.3.0-beta_aarch64.dmg`, drag to
  /Applications, the first launch provisions the home), building from the
  repo (the rust toolchain + `cargo install tauri-cli`, the two build
  commands and what each artifact is for), and `fleet install`.
  "Start here" now points at Install instead of re-stating the build.
- Six TUI screens captured at **120×40 from a scratch home** and rendered
  to `docs/images/tui-*.png` at **2160×1440** (the conversation, the
  palette, sessions, checkpoints, lanes, plan mode), referenced from the
  README and `docs/features/tui.md` with captions; install.md names the dmg.
- `scripts/tui-shots.py` — a docs tool (tmux 3.8 + Pillow 12.3.0, not a
  fleet dependency, so hard rule 12 is untouched).

**The capture lesson (the hard-won part).** `tmux send-keys -t <sess> C-p`
**did not land** in the TUI's raw mode — the screen never changed while a
plain character did. The reliable form is the **raw control byte**:
`tmux send-keys -H 10` for C-p (0x02 C-b, 0x0F C-o, 0x0B C-k, 0x0C C-l,
0x14 C-t, 0x1B Escape). Also read off the live face: the DEFAULT right-pane
board is the **router** (no key needed for that shot), and in tmux's legacy
encoding **0x0C opens LANES, not settings** — Ctrl-, collides with Ctrl-L,
so the settings shot was dropped rather than faked. Every screen was
**verified by signature** (the palette's command list, the sessions' empty
archive, the lanes' probe, the plan mode's status word) before the render —
the first-line log looks identical across all seven and proves nothing.

**How to apply.** The capture method is the terminal sibling of
[[gallery-shots-ride-a-scratch-home]] and
[[gallery-screenshots-2026-11-04-refresh]] — tmux's `capture-pane -e`, not
a browser. When driving a ratatui face from a script, go straight to hex
bytes and verify each screen by what it renders. The delivery is done:
merged to origin/master and the README is live.
