---
name: icons-and-huddles-ux-implemented-2026-11-04
description: "Both 2026-11-04 face plans (icons C1–C14, huddles-ux C1–C14) are implemented whole, gated, deployed and pushed at ef591be — and the ledger rewrite that ate 78 rows"
type: project
date: 2026-11-04
basis: inferred
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

**Shipped 2026-11-04 on `face-plugin-host`, pushed to origin at `ef591be`,
then fast-forwarded into `master` and pushed — `origin/master` and
`origin/face-plugin-host` are both `ef591be` (implementation `40f87bc`).**
Both plans are complete, every criterion ticked with evidence:

- **`docs/plans/2026-11-04-icons.md`** — one `ICONS` table where every key the
  source asks for is drawn (the thirteen silent `dot` fallbacks are gone), one
  concept one drawing, `unknown` as a visible defect mark, 1.6 stroke
  everywhere, main-rail groups fold with their needs-you count persisted in
  `fleet-rail-folded`, bots wear their initials, and `#rail-toggle` is the
  chevron alone. The harness pins the whole class by reading `app.js` as text.
- **`docs/plans/2026-11-04-huddles-ux.md`** — the split surface: rooms column
  (the work column's second painter) beside the room (the chat panel with a
  room head — title, state word, members, phases, shelf). `#/huddles/<id>`
  addresses a room; the composer says `to <member>`; the rail row carries its
  last line and author; `session_deliverables` (bridge.rs) replays handed-over
  files onto the shelf; archive clears the transcript and keeps the breakout.
- `docs/architecture/design-system.md` is new — the packet that says tokens,
  type, geometry, components, icon contract and anti-patterns.

Gate: fmt 0, clippy 0, release build 0, `cargo test` 2,033/0, harness 825
checks / 92 cases. Deployed through a new inode (KI-418's runbook), service
restarted, served `/app.js` proven to carry the markers; dmg
`fleet_0.3.0-beta_aarch64.dmg` (10.21 MiB) mounts an app whose packaged
binary carries them too.

**Why the ledger matters here:** repairing `docs/known-issues.md` with ad-hoc
`split('|')` scripts *ate the last cell of 78 pre-existing rows* (the file's
rows mostly do not end in a trailing pipe, so `[1:-1]` drops the next-action
cell). The checker's count went 7 → 138 before I noticed. Recovered by
restoring every row whose id exists at HEAD byte-for-byte, then rebuilding
only the five co-tenant rows by hand. See
[[a-memory-index-line-is-not-the-plans-state]] for the same lesson in the
other direction: check the real artifact, and when a rewrite touches a
corpus, diff the row count before believing the result.

**Pending by design:** the huddles plan's out-of-scope list still stands
(threads, reactions, typing dots, any new `Record`); the F3-style "go live"
follow-up from the earlier composer plan is untouched.
