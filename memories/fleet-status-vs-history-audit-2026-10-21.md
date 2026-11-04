---
name: fleet-status-vs-history-audit-2026-10-21
description: "2026-10-21 validation of fleet's docs and ecosystem states against git history — SHA refs clean, five unreccapped vault/composer commits, ecosystem doc's 10-13 states stale (lane pins behind upstream, second-brain 46 unpushed)"
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

Read-only audit (2026-10-21, owner-requested) validating plan/recap statuses and the
ecosystem doc's claimed states against actual commit history.

**Fleet internals: clean, with named gaps.**
- All 252 SHA references in `docs/plans/` + `docs/recaps/` resolve against master, except 3 deliberate cross-repo pins (blend1 `ab4ac55`, `d9c703c`; sys1 `c1f73836`) and 2 grep false positives (a date, a cache-TTL number).
- Five 2026-10-21 commits on master have **no plan and no recap**: the vault work `4fe9d41`/`1d6f946`/`fea955c` and composer style `ec2a60f`/`7385f4f` — only the memory index records them; the vault work is past the plan-before-non-trivial-work bar.
- `docs/plans/2026-10-20-bot-pwa.md` (status: draft) is still UNTRACKED; every other plan file is tracked with no uncommitted status flip. `2026-10-12-fleet-native-app` still reads "Status: active" (stale slice-1 umbrella).
- The three 2026-10-21 recaps are committed but their cited commits are dated 2026-10-20 evening — recap dates lead commit dates by a day.
- "hash b1dbf93f" in [[fleet-pwa-installability-borrow]] is the **FNV asset stamp**, not a commit SHA — do not flag it as an orphan commit in future audits.

**Ecosystem: the "verified 2026-10-13" states in docs/architecture/ecosystem.md are stale.**
- Vendored pins behind upstream HEAD: sys1 (`c1f7383` vs live `c6e978c`, dirty grew 4→11), sem1 (`a338912` vs `b7bd58b`), blend1 (`ab4ac55` vs `7e43640` — and the pin is NOW MERGED to main, so KI-151's "not merged" note is stale). gen1 and sdm1 pins still in sync.
- second-brain: main is **46 commits ahead of origin** (HEAD `4a2eae2` itself unpushed) plus 8 dirty runtime state files.
- sdm1 has 2 unpushed commits; sys1/blend1 1 each; several repos dirty.
- The identical untracked pair `docs/architecture/system.{drawio,svg}` sits in fleet, sdm1, AND gen1 — a diagram session's strays, committed nowhere.

**Why:** statuses and ecosystem claims read as verified but decay in days; an audit of cited SHAs against history is cheap and catches both broken records and unreccapped work.

**How to apply:** before citing an ecosystem state, re-check the repo live (branch, HEAD vs VENDOR.json pin, dirty/unpushed counts); when auditing, classify unresolved SHAs as cross-repo pins vs false positives before calling them broken; treat commits-without-recap as an open session-protocol loop.
