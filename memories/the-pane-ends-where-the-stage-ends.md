---
name: the-pane-ends-where-the-stage-ends
description: "KI-555 closed by anchoring the pane's right edge to the stage's via one --pane-right property (pane, scrim and grip clamp all read it); KI-556 closed by passing --app-version to the signer so the version rides inside the signed trusted comment."
type: project
date: 2026-11-17
basis: inferred
tier: durable
shape: [rendering, resolved]
phrases: [--pane-right custom property, "body[data-tissue=on] --pane-right var(--tissue-width)", the grip clamp reserved(), pane-scrim inset 0 var(--pane-right), the old 720 cap was the overlap written down, signer sign --app-version VERSION]
missing: []
premises: []
recall-count: 17
superseded-by:
valid-until:
source:
---

Two ledger rows from the 2026-11-16 warmup, both fixed and closed 2026-11-17.

**KI-555 — the drawer covered the connections column.** U13 in `docs/plans/UI-DECISIONS.md` says detail editing is a right drawer and "the persistent panes never hide behind it". The tissue (connections) is one of them. `.pane` was `position: fixed; right: 0` with `width: min(--pane-width, 100vw)`, so at 1440 an open pane ran x 1000..1440 while the tissue held x 1120..1440 — the whole column covered. The 32px scrim made the rest look disabled.

The fix carries the offset in ONE custom property, so the pane's edge, its width math, the grip and the scrim cannot disagree:
- `:root { --pane-right: 0px }`
- inside `@media (min-width: 1081px)` (the block that creates the column): `body[data-tissue="on"] { --pane-right: var(--tissue-width) }` and `body[data-tissue="off"] { --pane-right: 30px }`
- `.pane { right: var(--pane-right); width: min(var(--pane-width), calc(100vw - var(--pane-right))) }`
- `.pane-scrim { inset: 0 var(--pane-right) 0 0 }` — a scrim running to the viewport's right would dim the column the pane stepped around, which reads as disabled.
- the grip's `clamp()` subtracts `reserved()` (the tissue width, or the 30px reveal strip, or 0 below 1080), so a drag cannot widen the pane back over the column.

At 1440 with the column open the pane stops at 560 (= 1440 − stage floor 560 − tissue 320). The existing harness case pinned **720** for that cap — the old expectation WAS the overlap written down, so it had to be updated to 560, not worked around.

**KI-556 — the updater signature was not version-bound.** `scripts/release.sh` passed only `-k` and `--password`; the signer warned and emitted an unbound signature that a `requireSignedVersion` updater rejects. Fixed by adding `--app-version "$VERSION"` (`VERSION` is already derived from the tag at line 306). Proven by signing a throwaway artifact both ways with `~/.fleet/updater-key`: without the flag the warning appears; with it there is none and the base64 `.sig` decodes to `trusted comment: timestamp:... file:app.tar.gz version:0.8.11`. The version rides INSIDE the signed trusted comment — that is what the updater checks.

Verification discipline that mattered: both fixes were red-proofed by reverting them and watching the new pins go red (four for KI-555, and the grip reporting 720px again), then restored. A pin that cannot fail proves nothing.
