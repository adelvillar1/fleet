---
name: wip-provenance-stamp-idea
description: "Owner idea 2026-10-16 (not yet approved to build) — every harness stamps uncommitted WIP with harness name + session ID so dirty-tree archaeology can find where work came from"
type: project
date: 2026-11-05
basis: confirmed-by
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

The owner's idea (2026-10-16, floated as "just an idea" after the dogfooding-WIP incident where `src/api.rs` sat unattributed in the tree): every harness the owner works from (fleet, zcode, hermes) stamps its uncommitted work with harness + session ID, so history can be tied to its origin.

**Why:** multi-harness development leaves dirty trees whose provenance is only in the owner's head; today's api.rs attribution needed the owner's memory.

**How to apply:** if the owner green-lights it, the recommended shape discussed was a gitignored claims file per repo (`.wip-provenance.json`, entries `{harness, session_id, session_log_path, last_seen}`, stale entries = crashed sessions — itself information) riding the logs harnesses already keep (fleet's model-visible⟺logged rule means every write is already in the session log; the claim is just the index). Git-native alternative: `refs/wip/<harness>/<session>` snapshot refs — real archaeology but heavier and touches the no-`git add -A` house rule. Fleet seam: session start/teardown (the TUI thread-end hook where the drafter already runs). Cross-harness: a convention skill in ~/.agents/skills per [[canonical-skills-library-agents-skills]]. Related: [[fleet-dogfooding-api-wip-uncommitted]].
