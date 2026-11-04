---
name: fleet-owner-dogfooding-phase
description: "2026-10-21 owner context — heavy-development phase with a single-model pin; quiet router/calibration data is the pin's effect, not idleness; dogfooding found rough edges; close to the target vision"
type: project
date: 2026-11-04
basis: confirmed-by
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

Owner context, stated 2026-10-21: fleet has been under heavy development, and the
owner pinned to a single model during that stretch (later the same day they
confirmed the router is UNPINNED again). The live calibration
stream (409 route rows Oct 18–21, all serving step-5-preview, zero demotions or
calibration skips, no `how: pinned` rows) reflects that phase — dev/automated
traffic under a pin — not an idle router. Dogfooding surfaced "a lot of rough
edges"; the owner judges the app "close to what I want."

**Why:** without this, a future session reads the quiet decision stream as
evidence the router machinery never fires, or misses that the failover/demotion/
calibration paths are still effectively unexercised by real usage.

**How to apply:** treat router-machinery observations from this window as
untested-under-load; the pin has since come off, so failover, demotion and the
calibration ride-back are now live for real traffic. See
[[fleet-lane-service-target-architecture]] for the target,
[[router-enforcement-ruling]] for the standing selection rules, and
[[fleet-workload-effort-direction]] for the owner's next router ask.

Update 2026-10-21, later: the owner began the dogfooding pass themselves ("I am the one that needs to put it through its paces") on the face running the router slice (4bd2f18 + the KI-280 flake fix, d78548d). Findings they report go through triage → known-issues → a plan, not ad-hoc fixes.

Update 2026-10-21, evening: first finding shipped (folder picker, c40319d). The owner contextualized the boards' red numbers twice, and both corrections are authoritative: the single model on the router board is the week's pin ("the router has not been doing its job because I had the model pinned"), and today's 64 failed calls / 71 turns labeled "stopped by the owner" are **errant cron jobs — "there have not been any single requests today"** — not owner stops and not product failure (14 nightly corpus-refresh jobs, most failing per KI-277; the interrupted label on scheduled runs is itself misleading). Development pace: 98 commits / 61k insertions in three days. When reading router/session/calibration boards from this period, expect cron and pin noise, not user traffic.

Update 2026-10-21/22, the pass proper. The owner named the surfaces that matter because "that's where users will land first": conversations, sessions, router, delegation, memory, skills, second brain, bots, vault, settings, calibration. Both substrates for documenting them are committed: `docs/architecture/surfaces.md` (dabdad6 — the what and why of all eleven, read live through Playwright) and `docs/architecture/lanes.md` (a869048 — why five specialized services stand between the owner and the LLM). The three public documents were rewritten on that substrate (7a589a9 — README/functional in product-manager voice, technical in TPM voice; screenshots committed into `docs/images/` after the owner caught them missing from GitHub, c796cb4). Two more dogfooding findings shipped: **the approval seat read as "it never closes"** (05e3e86, KI-283 — the answer posted no session so the bridge looked under the launch session, and an answered seat left no trace; now the grant lands as a transcript line and the card closes) and **no run feedback** (b875e0b/5946647 — a turning ring on the state pill and composer hint plus "working · turn 1 · 0:20 since the last event" and the seat's own wait clock). The findings keep arriving one surface at a time from real use; each goes through the same route: reproduce live, root-cause, fix, verify in the browser, gate, push. See [[verify-face-changes-in-the-browser]] for what the gate could not see.

Update 2026-10-23, the first mess the dogfooding pass produced. The owner came back with "I was dogfooding with fleet and seems like it made a mess" over 44 dirty entries — three unrelated things piled together: a partial staging that left `master`'s tip uncompilable ([[shared-file-commits-can-sweep-another-sessions-work]]), sixteen untracked cron-written corpus snapshots ([[fleet-memory-corpus-dirties-the-tree]]), and a ledger that had numbered two different rows KI-296. Because fleet now *is* the harness the owner develops fleet with, dogfooding faults now surface as repository mess — dirty trees, duplicate rows, snapshots nobody asked for — not only as GUI pain. Triage them by class and give each its own commit, separate from the repair, so the owner can read what actually happened. The ledger id collision is KI-285's third sighting and its writer fix assigns ids from a counter instead of reading the table, so expect a fourth until that lands.

Update 2026-10-23, evening: the owner's stated next milestone is **working reliably** ("once I can work reliably, and I think I'm very close, the ledger will be taken care of") — i.e. closing the ledger IS the reliability work, not a chore after it. The 33 open rows cluster into four piles: five mechanical defects closeable now (KI-256 scheduled runs get no lane tools, KI-277, KI-294 scheduled runs declare no skills, KI-282 favicon, KI-308 the `sk-` substring guard), two flakes that ARE the definition of reliable (KI-288 relay poll, KI-293 related-route — a gate red one run in twenty trains the owner to ignore red), two daily-biters (KI-284 the write guard's false refusals — plans have been working around it via bash writes, so its guarantee lapses exactly when content is constrained — and KI-289's route half), and three design rows (KI-278 child routing, KI-296 embedding endpoint health, KI-297 blend1 convergence). Offered as a burn-down session: fix the five mechanical + two flakes in one gated pass, sharpen proposals on the three design rows. Awaiting the owner's go.
