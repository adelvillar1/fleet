---
name: turn-cues-and-caps-are-one-unswept-ruling
description: "The 2026-11-03 owner ruling retired the user-facing turn number from the freshness surfaces and the composer, but left the turn mark's ordinal in place — so the turn-cue removal finishes an in-force ruling, not a feature reversal; the turn cap itself is a recorded deferred owner decision."
type: project
date: 2026-11-04
basis: inferred
tier: durable
shape: [docs, resolved]
phrases: [quota-failover-and-compaction-count, turn count is meaningless to users, turn 3 · 4 tools · thought 2s, runTurns, capped max-turns turns:100, conversation-density]
missing: []
recall-count: 0
superseded-by:
valid-until:
---

Verified 2026-11-04 from master (repo adelvillar1/fleet, default branch master).

**The turn cue (a).** `docs/plans/2026-10-31-quota-failover-and-compaction-count.md` decision 3, dated 2026-11-03, quotes the owner: "turn count is meaningless to users, if anything compaction count or an indication of context degradation is the only thing that a user would care for." That decision retired `runTurns` ("turns left") from the composer and swapped four freshness surfaces to the compaction count (Q4 landed, harness 485 checks / 53 cases).

The sweep did not reach the turn mark. `FUNCTIONAL-SPECIFICATIONS.md` at master still mandates it verbatim: "its tool and thinking rows fold under their turn marks and each mark carries the counts it hides (`turn 3 · 4 tools · thought 2s`)". Twenty lines later the same document states "A turn count remains only where it is a remaining budget, never a size claim" — so the specification contradicts itself: the ordinal in `turn 3` is neither a budget nor a size claim, and it is exactly the user-facing number the 2026-11-03 ruling banned.

Therefore removing the visible ordinal is finishing an owner ruling already in force; the fold machinery (`face.turnFolded`, `face.turnSummaries`, `summarizeRun`, `toggleTurn`, `paintTurnFold`, D6 divider, D8 turn navigation) is a separate completed feature (`docs/plans/2026-10-31-conversation-density.md`, status completed, R5/R7/R9) that should be preserved, not deleted.

**The cap (b).** The same plan's decision 3 records the deferred owner framing: "a turn is not a unit of anything a user would measure … so a turn cap is a crude proxy for what actually matters, which is context degradation … The machinery stays as it is until that is decided." No commit through ef591be (2026-11-04) implements that decision.

Cap firing evidence: `GET http://127.0.0.1:3080/api/sessions` showed nine rows `"state":"capped","stop_reason":"max-turns","turns":100` — all scheduled/background (`corpus-refresh-*`, `lifecycle-sweep`) — and zero capped interactive sessions. Exit code 3 is the documented cap exit ("resume with `--session <id>`").
