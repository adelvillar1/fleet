---
name: fleet-harness-orientation-doc
description: "the fleet TUI harness's knowledge intake is only skills (name+description) and per-instruction recall over memories/ — it never reads CLAUDE.md as instructions; the orientation doc docs/harness-orientation.md (commit 7695430, 2026-10-16) is the methodology's home, routed by the harness-orientation-lives-in-docs fact"
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

The owner's complaint (2026-10-16): fleet running in its TUI "doesn't know anything about how I work." Verified in source: `agent::SYSTEM_PROMPT` is two generic sentences; the harness's only intake seams are the skills list (`<FLEET_HOME>/skills/`, name+description via progressive disclosure) and per-instruction recall over the repo's `memories/` corpus. `CLAUDE.md` is not read as instructions (nomenclature only walks it for vocabulary).

**The fix, committed `7695430`:** `docs/harness-orientation.md` — addressed to the harness: owner profile, the six knowledge seams, the warmup→plan→build→recap→commit cycle, the gate + verification-matching, the repo map, the eleven house rules, environmental non-failures. Routed by `memories/harness-orientation-lives-in-docs.md` (verified: ranks #1 at 0.61 for "how do we do things"). CLAUDE.md's index carries the pointer line.

**Why:** a document fleet can't reach teaches nothing — in-repo content must ride recall or skills to reach the model.

**How to apply:** future methodology/ruling work should be routed through a `memories/` fact (corpus shape: name=stem, basis required, ≤6 phrases) or a skill. The follow-up shipped and then deepened: the `harness-orientation` skill lives in the canonical library per [[canonical-skills-library-agents-skills]] — fleet loads the library directly now, no install step. Related: [[fleet-memory-corpus-dirties-the-tree]], [[fleet-routes-through-its-own-router]], [[orchestrator-routes-through-hard]].
