---
name: fleet-dogfooding-api-wip-uncommitted
description: "The dogfooding TUI session's src/api.rs WIP (streamed finish_reason) was completed and LANDED in aa367e8 on 2026-10-16 — no longer uncommitted; landed for file-level inseparability, disclosed to the owner"
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

During the canonical-skills/orchestrator session (2026-10-16), fleet's tree carried uncommitted `src/api.rs` WIP from the owner's dogfooding TUI session (the ki-triage batch-1 item "streamed finish_reason / total_tokens derivation"). It did not compile as found; that session completed it minimally (Copy off `Usage`/`StatusUpdate`, initializers, clones at six `agent.rs` move sites) and left it uncommitted.

**Resolution (same day, commit `aa367e8`):** the harness-voice slice required committing `src/agent.rs`, which carries both the WIP's clone sites and the new identity work — a file-level split was impossible without leaving HEAD failing clippy (clone-on-copy at HEAD's Copy `Usage`). The WIP was completed, gated by the full suite (1026 passed), and landed in the same commit with explicit attribution in the message. The full-suite pass is the verification; the feature's own targeted tests are still the dogfooding session's to add if it wants them.

**How to apply:** do not revert or "clean up" `aa367e8`'s api.rs half without the owner's say-so; if the owner wants the WIP out, the honest path is a split they approve (history rewrite needs explicit approval per CLAUDE.md rule 7). Related: [[fleet-memory-corpus-dirties-the-tree]], [[fleet-harness-voice-soul-slot]].
