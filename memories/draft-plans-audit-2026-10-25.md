---
name: draft-plans-audit-2026-10-25
description: "2026-10-25 audit of fleet's four draft-status plans against commit history — judgment-beats and capability-pitch fully pending, bot-pwa pending at its mockup gate, sys1-extraction-leg partially overtaken by Perception (build the four-verb leg ON it); audit notes recorded in each plan file (425fedd)"
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

The owner asked for fleet's draft-status plans reviewed against commit
history. Four drafts (README's vocabulary line is not one), each now
carrying an "Audited 2026-10-25" block in its own file — commit
`425fedd`. The audit recipe that worked: list `status: draft` files,
extract each plan's real criteria, then check named artifacts against
every branch (`ls` for files, `git log --all -- <file>` for
never-landed work, grep the tree for the named seams).

**judgment-beats-one-lane (10-23) — fully pending, plan current.** No
commit touched src/judge.rs or src/delegate.rs since 10-23; ask_lane
still spawns→initializes→lists→one call→tears down per row
(src/judge.rs:612; delegate.rs:940 — the plan's `:1363` citation has
drifted), and parse_workload returns only the workload head. All five
criteria stand. Most build-ready of the four.

**sys1-extraction-leg (10-19) — was partially overtaken by Perception;
E1+E2 landed 2026-10-25 (ae5675a), E0 blocked on credentials.**
`sys1_perceive_document` (5782497, 35a3335) proved this plan's
credential-holder discipline with a real extraction-class capability —
and the leg then landed ON that pattern: `src/extract.rs` (the four
verbs as field types on one pass, p_null, `sys1_extracts()` gate,
`gate_legal` refusing estimates/forecasts) and the fleet `extract`
tool, declared on both declaration paths. E0's live probe is blocked on
`CLOUDFLARE_ACCOUNT_ID`/`WORKERS_AI_API_TOKEN` (KI-312, owner action;
`scripts/extract-probe.sh` is committed runnable). E3–E6 wait for the
live wire by design. See
[[extraction-leg-e0-blocked-e1-e2-landed]].

**bot-pwa (10-20) — PARTLY LANDED since this audit; still gated at B0.**
The 2026-10-25 and 2026-10-26 audits both read fully pending; that is
now SUPERSEDED — B2 (`Bridge::inbox`, `GET /api/inbox`, the bounded
tail read) and B3 (`/api/session/new` and `/api/steer` admitted on the
remote face) landed 2026-10-26 in `7d2c81e` with tests, though the
plan's own status field still says draft. B0's mockup directory
`docs/plans/mockups/2026-10-20-bot-pwa/` was BUILT 2026-10-28 (four
screens + README, research-cited, verified headlessly); the D8 gate
still forbids B1 until the owner writes the dated approval line, and no
bot.html/bot.css/bot.js or docs/features/bot-app.md exists.
Full state, the tests, and the archive-vocabulary drift:
[[bot-pwa-b2-b3-landed-2026-10-26]], and the mockups:
[[bot-pwa-b0-mockups-built-2026-10-28]].

**capability-pitch (10-23) — fully pending.** Its own drafting commit
(6be8096) is the only one; no capability-offer surface exists (the
bridge's `capability` hits are the unrelated pairing sense). C4's
health-check-backed embedder offer now rides the probe retry KI-296
hardened, which makes the offer easier than when drafted.

Suggested order if all four are called: judgment-beats (most
build-ready) → capability-pitch (small surface) → bot-pwa (starts
with mockups for the owner's review) → extraction leg (wants the sys1
upstream decision first). Related: [[plan-is-a-file-not-prose]],
[[sys1-extraction-leg-clairic]],
[[fleet-persona-bots-fit-as-bot-profiles]],
[[fleet-judgment-beats-spawn-per-row]],
[[fleet-onboarding-composer-proposal]].
