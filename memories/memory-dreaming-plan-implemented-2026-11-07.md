---
name: memory-dreaming-plan-implemented-2026-11-07
description: "project 2026-11-07 — the memory-dreaming plan (docs/plans/2026-11-07-memory-dreaming.md) COMPLETED and COMMITTED (07bc3b5 + 6204060, plan status completed, recap landed); the live C7 proof passed through the real sys1 panel and the three surfaces agree"
type: project
date: 2026-11-11
basis: inferred
tier: durable
shape: []
phrases: []
missing: []
premises: []
recall-count: 0
superseded-by:
valid-until:
source: zcode:fleet:content:285983102772d99aaed2c496d57acdf6f320aad7347d3891e9bb9e2378839d34
---

The memory-dreaming plan at `docs/plans/2026-11-07-memory-dreaming.md` (C0–C9) was drafted, plan-surfaced (disposition: overridden — the c4_first orderings are below-threshold non-edges; C0 licenses auto-landing and C1–C3 are the pieces C4 composes), arch-gated PASS (4 conventions gaps), and plan-gated with ONE judgment-band flag on the docs criterion (p=0.47 stable across two rewords — waived by disposition). The owner's "implement plan as designed" flipped it to `active`, and the whole build landed on `face-plugin-host`:

- **C0** the measured-set fixture test (`the_measured_set_separates_known_wrong_from_known_right`) — ledger-row-known-wrong facts scripted `contradicted`, known-right `supported`, a below-bar verdict skipped; `VALIDATE_AUTO_LAND_CONFIDENCE = 0.8` is the gate.
- **C1** the `source` field end to end: `Remember`/`Fact`/`FactEdit` (empty-clears), `render_document`, `remember --source`, the pane card field, `memory_read`/`boards::MemoryFact`, harness case.
- **C2** `Basis::Validated` + `PROVENANCE_VALIDATED = 0.75`; the WRITE PATH refuses `validated` (the dream pass is the only producer); supersede treats it like inferred (justification only); the cycle's untouched set names it beside confirmed-by. Both open rulings landed as recommended.
- **C3** calib `Row::Validate` + `OP_VALIDATE` (OPS is now 15) + route-cache THIRD namespace (`validate`, same key/rekey discipline) + `parse_validate` (sys1 answers-head majority/mean + plain object) + `dream::DreamValidator` (lazy LaneSession open, transcript via `session_recall::transcript_text`, unresolved source → unverifiable).
- **C4** `memory::dream` — injected judge closure (the caller owns the lane; the pass stays lane-free so the fixture test scripts verdicts), queue = inferred+source+live ranked by recall_hit access counts, budget `DREAM_PASS_BUDGET = 8`; contradicted-past-bar supersedes via the real supersede path with session+verdict in the justification.
- **C5** `fleet memories --validate` + `run_dream_pass` in main.rs (current-thread tokio runtime, block_on per judgment from the SYNC pass — block_on INSIDE an async context panics) + `spawn_homekeeping(root, home, stream)` replacing spawn_consolidator at all three sites (chat/TUI/face; home=None keeps cycle-only).
- **C6** `PROTECTED_TOKEN_CAP = 1024`; overflow facts leave the section, get scored, rank between the primary pool and the spillover marked `tier-overflow` (ScoredFact grew the flag), the strongest enters even under pressure, the block names the count; never in both at once.
- **C7** the live proof, PASSED: a fact sourced to a real session (`dream-proof-inbox-file`, `source: 20261107T170219Z-a97c759b`) went through `fleet memories --validate` with the real sys1 panel — glide 0.89 / drex 0.77 / jev 0.91 `supported`, mean 0.857 over the 0.80 gate — and the three surfaces agree: the file (`basis: validated`), the calib stream (5 `validate` rows, every judgment audited), and the pane (temp face on 3090, stopped after).
- **C8** the full gate green end to end: fmt, fmt-check, clippy, release build, test — exit 0 each; harness 972 checks / 111 cases.
- **C9** the docs: `memory.md` (dream pass, source, validated, tier cap), the tech doc's module reference (memory/dream/session_recall), the functional spec's memory surface.

**The gate failures found in the full suite — all fixed:** (1) the judge socket guard reddened at `crate::judge::DreamValidator` in memory.rs → the validator moved to its own `src/dream.rs` ([[the-judgment-beat-socket-guard]]); (2) `MAX_DEFINITION_TOKENS`: the source property pushed 9,978→10,016 → the bound moved to 10,050 with the third crossing named ([[a-declaration-budget-is-a-measurement-too]] — the move restored the ORIGINAL declaration-list expectations, so my test-list edit was reverted); (3) the calib OPS test's expected list → 15 entries; (4) six `MemoriesOptions` test literals + `graph_fact` + the tui `MemoryFact` fixture (E0063 fan-out).

**Two defects the live pass caught, fixed in 6204060:** the sys1 `validate` task did not exist (registered in `services/sys1/src/sys1/tasks.py` with the `validity` head over supported/contradicted/unverifiable, VENDOR.json fleetAdditions-recorded, lane tests 17/17 — [[sys1-vendored-tasks-py-seam-and-the-python39-trap]]), and the validator handed `parse_validate` the answers sub-object while the parser reads the FULL reply (the validator now prints the unparsed answer — the named diagnostic that found it).

**Why:** the session was interrupted by the owner's speed steer with the final gate command cancelled; the owner ruled the long silent compile-heavy batches bad UX and ordered a for-good process fix, which LANDED at 701e256 ([[the-build-loop-ux-verdict-2026-11-07]]). The second pass ran on the new loop — the full gate was one background cycle.
**How to apply:** the work is DONE and committed — `07bc3b5` (the build) and `6204060` (the sys1 task, the validator diagnostics, the completed plan, the recap at docs/recaps/2026-11-07-memory-dreaming.md). The DELIVERY legs then closed the same day: the dmg rebuilt and installed, the local face relaunched and proven serving the new document ([[dream-dmg-installed-face-relaunched-2026-11-07]]), and the MERGE to master FOLLOWED at `1eaba7a` through a scratch worktree (conflicts resolved to master's newer docs; merged-tree gate green, harness 984/116; both refs pushed). Related: [[sys1-validates-inferences-design-2026-11-07]], [[rebuild-is-not-commit-push-or-ship]], [[self-improvement-deep-dive-2026-11-07]].
