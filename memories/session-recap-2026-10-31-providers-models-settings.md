---
name: session-recap-2026-10-31-providers-models-settings
description: "The day's third session closed — recap at docs/recaps/2026-10-31-the-providers-models-settings.md, pushed 7824e21; app reloaded with a live new-op probe; FS passage proposed for approval"
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

The 2026-10-31 providers/models session closed: recap at `docs/recaps/2026-10-31-the-providers-models-settings.md`, committed 7824e21 and pushed (`ea1742a..7824e21`). Ledger clean before drafting (384 rows / 36 open). Sweeps: process registry 0 entries, cache sweep free space 53.0 → 53.1 GiB.

**The reload, proven the owner's way** ([[verify-the-binary-is-newer-than-the-source]], [[rebuild-is-not-commit-push-or-ship]]): the gui runs `target/release/fleet gui --no-open` under `com.fleet.gui` — so `cargo build --release` (1m53s), then `launchctl kickstart -k gui/$(id -u)/com.fleet.gui` → new pid 58160 at 11:00. Three-byte proofs: root 200 on 127.0.0.1:3080, the served `app.js` contains `reload models`, and a live `POST /api/settings {"op":"provider-discover"}` answered `` `no-such-provider` names no configured provider `` — a sentence that exists ONLY in the new binary (the old one would say unknown op). That last probe is the reusable shape: hit an op the landing added and read its named error.

**A live-op probe is stronger than an asset grep**: app.js can be served from disk while the bridge change needs the binary; one POST proves the restarted code path end to end.

Doc state: `docs/features/routing.md` (the settings-surface section) and `TECHNICAL-DOCUMENTATION.md`'s settings entry landed in 33a5679. **Pending the owner's word**: the `FUNCTIONAL-SPECIFICATIONS.md` passage at ~line 143 still describes the old "provider roster as field editors" shape — replacement text sits in the recap's "Doc updates proposed" section (the skill's propose-don't-auto-write rule), so it awaits approval rather than being applied.

Open next: the discovery's behavior on a REAL endpoint (every pin runs over the stub listing — the owner's first paste-key is the live proof), the corpus-refresh-dev-decisions failure from 2026-10-30 still unexamined, two report-only orphans (npx sandbox-mcp, blend1's service venv).

Related: [[provider-models-settings-plan-2026-10-31]] (the plan's full record).
