---
name: tauri-updater-deep-dive-2026-11-09
description: "2026-11-09 deep dive — neither update channel completes an install; KI-486/487/488, and the plan/ledger/recap claim of a working plugin hop is false"
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

The updater deep dive landed at `docs/architecture/deep-dive-2026-11-09-tauri-updater.md`
(commit `0a4cb92`, docs-only). Three defects filed, all evidenced:

- **KI-486 (data loss):** the CLI channel (`update::self_update`) writes the
  release's **gzip tarball** over the binary — the feed's cli sha256 is the
  tarball's own, so the digest check passes. No unpack, no exec probe, no
  backup. Reachable via `fleet update` and the About pane on a browser tab.
- **KI-487:** the shell's Tauri updater chain is refused by the ACL before
  the network — the window loads `http://127.0.0.1:<port>` (`WebviewUrl::External`),
  which Tauri classifies as `Origin::Remote`, while the capability compiles
  `"local": true` with no `remote` urls. `restart_app` is refused too.
- **KI-488:** the About pane paints "the feed names X — nothing newer" before
  any feed read (the settings document never asks the feed) and keeps it when
  the check request fails (empty `.catch`) — the plan's own honesty clause
  forbids this.

**Why it matters:** the plan's criterion, KI-483's "next action" and the
0.6.2 recap all assert a working hop (the Tauri-native updater in the dmg
shell) that no run ever produced — the install leg has never executed live;
the installed binaries are all manual copies. Do not repeat that claim.

Both channels work on the *check* half; only the apply half is broken, and
the open decision for the owner is the artifact shape (raw binary asset vs
unpack in the client) plus the window-origin choice (grant `remote`, or move
to `tauri://` and lose the service worker).

**The owner met it live:** after two presses they reported *"it told me that I
needed to restart the app and nothing happened."* The installed bundle reads
**0.6.3** while the feed names 0.6.4, so About legitimately showed a candidate;
the press took the plugin branch, KI-487's ACL refused before the network, and
the only paint went to the transcript pane two panes away. The "needs a
restart" wording the owner remembers is the card's own standing line ("the
applied version takes over at the next restart") — that standing sentence
beside an unchanged pane is what makes a dead press read as a done one.

**The plan is DRAFTED:** `docs/plans/2026-11-09-updater-that-installs.md`
(status draft, uncommitted), criteria C0–C9 — see
[[updater-that-installs-plan-drafted-2026-11-09]]. Two things a fixer must not
forget: the ACL is compiled into the bundle, so **0.6.4's own button cannot be
fixed** — 0.6.5 is installed by hand once and every release after is a press;
and `plan-gate`/`plan-surface` returned `no provider answered (chain: jev)` this
session while `arch-gate` ran (25 UNDOCUMENTED conventions gaps, none a defect).
