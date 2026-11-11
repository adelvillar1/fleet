---
name: dream-dmg-installed-face-relaunched-2026-11-07
description: "project 2026-11-07 — the dream-plan delivery legs CLOSED: the dmg rebuilt (10.84 MiB, services resources ride), /Applications/fleet.app replaced (old moved aside, not deleted), the local face relaunched on it and PROVEN serving the new document; the merge to master FOLLOWED (1eaba7a) and is DONE — nothing open"
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

The owner asked (2026-11-07), after the dream-plan completion: *"do the dmg, because I like to use it for my local machine."* The delivery legs then closed in one pass:

- **The dmg:** `fleet_0.4.0-beta.1_aarch64.dmg` (10.84 MiB) from `cargo tauri build` in `src-tauri/` — the bundle carries the `services` resources, so the sys1 `validate` task rides inside it. The shell links the fleet library in; the dmg is self-contained.
- **The install:** the old `/Applications/fleet.app` was MOVED ASIDE (`fleet.app.pre-dream-<hhmm>`, not deleted — the destructive-pattern gate blocks `rm -rf`, and a move is the reversible form), the fresh app copied in, quarantine cleared (`xattr -dr com.apple.quarantine`).
- **The relaunch:** the old face (pid 20950) killed, the new app opened — pid 48011 on 3080, and its own `/api/memory` answers with the NEW document: the proof fact reads `basis: validated` with its `source` pointer. The owner's local machine now runs the new build.
- **Recorded:** the recap (commit `a7ea917`) carries the delivery paragraph.

**Why:** the completion answer had named the build state without the delivery state — [[rebuild-is-not-commit-push-or-ship]]'s exact failure — and the owner's follow-up ("does it need to be merged and the app rebuilt?") made the delivery the work.
**How to apply:** ALL LEGS CLOSED. The merge to master FOLLOWED the same day (2026-11-07, `1eaba7a`) through a scratch worktree — the conflicts resolved to master's newer docs, the merged-tree gate was green (harness 984/116), and both refs were pushed; see [[dream-delivery-legs-open-2026-11-07]] for the merge's record. The local machine's face runs the new build. Related: [[dream-delivery-legs-open-2026-11-07]], [[memory-dreaming-plan-implemented-2026-11-07]], [[tauri-dmg-rebuilt-2026-11-05]], [[the-served-face-is-the-installed-copy]].
