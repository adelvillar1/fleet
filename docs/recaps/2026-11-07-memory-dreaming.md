# Session Recap — 2026-11-07: the dream pass

**Summary.** The memory-dreaming plan landed whole: the corpus's
inferred facts now carry an evidence pointer, sys1 judges each one
against the session it names, and a supported verdict past a measured
bar stamps the third basis value `validated` — live-proven end to end
through the real lane. The same session fixed the build process that
the work exposed: the structure guards are now an inventory plus a
preflight script, named before the first compile instead of discovered
by failing the suite.

## Plans worked on

### `docs/plans/2026-11-07-memory-dreaming.md` — memory dreaming

Status after this session: **completed** (every box ticked; plan_check
passes).

| Acceptance criterion | Status | Evidence |
|---|---|---|
| C0 — the measured set separates | ✅ met | `the_measured_set_separates_known_wrong_from_known_right`: known-wrong fixture facts (KI-060, KI-164) retire on contradicted @0.88–0.92, the known-right stamps on supported @0.95, a supported @0.40 sits below the 0.80 gate and acts on nothing |
| C1 — the source field round-trips | ✅ met | write path, `FactEdit`, `fleet remember --source`, the pane card (harness case), the remember schema; `the_source_field_round_trips_through_writer_loader_and_edit` |
| C2 — the validated basis | ✅ met | refused on every direct write, loads from disk, `PROVENANCE_VALIDATED` 0.75, the cycle names it untouched, supersedable on justification (`the_validated_basis_is_the_dream_passs_value_alone`) |
| C3 — the validate task | ✅ met | `ValidateRow`, the route-cache third namespace (round-trip + key discipline pinned), `parse_validate` over both lane shapes, the sys1 `validate` task registered in the vendored lane (17/17 lane tests) |
| C4 — the dream pass | ✅ met | `memory::dream`: access-ranked queue bounded at 8, stamps on supported past the gate, retires on contradicted through the real supersede path, names every skip; `the_dream_queue_takes_only_sourced_inferred_facts` |
| C5 — scheduling | ✅ met | `fleet memories --validate` on demand; `spawn_homekeeping` (cycle + dream) hourly at chat, the TUI, and the face; a lane-less process degrades to named skips |
| C6 — the tier cap | ✅ met | `PROTECTED_TOKEN_CAP` 1024; the 12-fact fixture's section fits the cap, the overflow ranks marked `tier-overflow`, none enters twice, the block names the count (`the_tier_cap_keeps_the_section_bounded_and_the_overflow_reachable`) |
| C7 — the live proof | ✅ met | quoted below |
| C8 — the gate | ✅ met | fmt, fmt-check, clippy, release build, test — exit 0 each; face harness 972 checks / 111 cases |
| C9 — the docs | ✅ met | `memory.md` (dream pass, source, validated, tier cap), the tech doc's module reference (memory/dream/session_recall), the functional spec's memory surface |

**C7's live proof.** A fact sourced to a real session —
`dream-proof-inbox-file`, `source:
20261107T170219Z-a97c759b` — went through `fleet memories --validate`
with the sys1 lane live. The three-judge panel answered `supported`
(glide 0.89, drex 0.77, jev 0.91; mean 0.857 over the 0.80 gate), and
the three surfaces agree:

- the corpus file: `basis: validated`, `source: 20261107T170219Z-a97c759b`
- the calib stream: 5 `validate` rows (every judgment audited, the
  failed parses included — the machinery recorded what it could not
  read, then named why)
- the pane (new binary on a spare port): the fact card carries
  `basis: validated` and the source pointer

Two defects the live pass caught and the fixes are in: the validator
handed the parser the answers sub-object while the parser reads the
full reply (the diagnostic now prints the unparsed answer), and the
lane's `validate` task did not exist — registered in
`services/sys1/src/sys1/tasks.py` with the `validity` head over
supported/contradicted/unverifiable, and recorded in VENDOR.json's
fleetAdditions.

## Commits

| Hash | Message |
|------|---------|
| `701e256` | docs+scripts: the structure-guard inventory and the edit preflight |
| `07bc3b5` | feat(memory): the dream pass — sourced facts validated against their sessions through sys1 |
| (this close-out) | the sys1 validate task, the validator diagnostics, the completed plan, this recap |

## What was added

- **The evidence pointer** (`source`): the write path, the edit, the
  CLI flag, the pane card, the remember schema — the field the dream
  pass reads a fact against.
- **The validate judgment**: sys1's `validate` task, the route cache's
  third namespace, `ValidateRow`, `parse_validate` over both reply
  shapes, and `src/dream.rs` — the lane-owning validator that keeps
  the pass lane-free so the socket guards stay true.
- **The dream pass**: the access-ranked queue, the confidence-gated
  acting, the named skips, `fleet memories --validate`, and the hourly
  `spawn_homekeeping` beside the consolidation cycle.
- **The `validated` basis**: the third rung of the provenance ladder —
  evidence-checked by the lane, no owner authority claimed, left by
  the cycle, supersedable on justification.
- **The tier cap**: `PROTECTED_TOKEN_CAP` (1024) with the overflow
  ranked as marked `tier-overflow` candidates — the 201-fact tier no
  longer rides whole into every turn.
- **The process fix**: `docs/architecture/structure-guards.md` (the
  six guard classes), `scripts/edit-preflight.sh`, the house
  build-loop skill, and the CLAUDE.md session-protocol line.

## What was fixed

- The **token-bound crossing**: the remember schema's `source` field
  put the native surface at 10,016 against a 10,000 bound; the
  constant's own history names the protocol, and it moved to 10,050
  with the third crossing recorded (the runtime withholding walk
  follows the same constant, so the declared-list test's expectation
  moved with it — and moved back when the bound did).
- The **stale declaration test**: `a_run_with_memory_declares_the_write_path`
  expected `apply_patch` with no `tool_search`; the fixed bound let
  the surface fit whole again and the original expectation held — the
  drift was the bound, not the list.
- The **judge-guard violation**: `memory.rs` named `crate::judge`
  when the scheduled thread adopted the validator; the wrapper moved
  to its own module (`dream.rs`), which is the sanctioned shape.
- The **silent validator skips**: the validator now prints why the
  judge could not answer (a dead lane, an unparseable reply) instead
  of only naming the skip.

## What was changed

- The face's memory pane card carries the `source` field (edit +
  search haystack) and the fact read rides the new `boards::MemoryFact`
  field.
- The `remember` tool's schema gained `source` with minimal wording —
  the definition-token budget is a real guard, and the full
  instruction lives in the CLI flag, the pane label, and the docs.
- The sys1 vendored lane's task registry gained `validate` (fleet
  addition, VENDOR.json-recorded), following the `tool_route`/`curate`
  precedents.

## Files changed

**Core**
- `src/memory.rs` — the source field, the validated basis and
  `PROVENANCE_VALIDATED`, `memory::dream`, the tier cap, six new test
  groups (~1,100 lines).
- `src/dream.rs` — new: the lane-owning `DreamValidator` and
  `VALIDATE_TASK`.
- `src/judge.rs`, `src/calib.rs`, `src/route_cache.rs` — the
  `ValidateRow`/op/OPS entries, the cache namespace, the judge-side
  seams.
- `src/agent.rs` — `MAX_DEFINITION_TOKENS` 10,000 → 10,050 with the
  crossing named in the constant's history.
- `src/tools.rs`, `src/cli.rs`, `src/main.rs`, `src/boards.rs`,
  `src/onboard.rs`, `src/session_recall.rs` — the schema field, the
  flags and wiring, the snapshot field, the constructor updates.

**Face**
- `src/gui/bridge.rs`, `src/gui/assets/app.js` — the pane card's
  source field and search.

**Lane**
- `services/sys1/src/sys1/tasks.py` — the `validate` task.
- `services/sys1/VENDOR.json` — the fleetAdditions entry.

**Tests / docs / process**
- `tests/face/harness.mjs`, `docs/features/memory.md`,
  `TECHNICAL-DOCUMENTATION.md`, `FUNCTIONAL-SPECIFICATIONS.md`,
  `docs/architecture/structure-guards.md`, `scripts/edit-preflight.sh`,
  `CLAUDE.md`, the house build-loop skill, the plan, this recap.

## Doc updates applied

- `docs/features/memory.md` — the dream pass section (queue, verdicts,
  gate, scheduling), the evidence pointer + validated section, the
  tier cap paragraph, the `--validate` CLI verb, the header map.
- `TECHNICAL-DOCUMENTATION.md` § "Module reference" — the memory.rs
  and new dream.rs entries; session_recall's transcript read.
- `FUNCTIONAL-SPECIFICATIONS.md` § "The surfaces" — the memory
  surface's source field, the validate verb, the validated basis
  (riding with the parallel session's in-flight inbox paragraph in
  the same file).

## Doc updates deferred (debt)

- None for this plan. The dreaming doc's areas 5 and 7 (wiki/hub
  representations, the learning loop closed into the corpus) remain
  future plans, recorded there as such.

## Open questions / next steps

- The `dream-proof-inbox-file` fact stays in the corpus as the live
  proof artifact; a future session may retire it or keep it as the
  example of the stamped basis.
- The sys1 side lives in the vendored copy; upstreaming `validate`
  (the `curate` precedent) is the sibling repo's follow-on.
- The two deep-dive docs remain the design record; the process fix
  they motivated is now the session-protocol line.

## Notes

- **The lane call chain, verified live:** `fleet memories --validate`
  → the runtime block_on per judgment → `DreamValidator::judge` →
  `LaneSession::ask` (task `validate`) → the three-judge panel →
  `parse_validate` (majority word, mean confidence) → the gate →
  stamp. The judge's confidence mean (0.857) cleared the 0.80 bar;
  the fixture test pins the below-bar path.
- **The panes' three-way agreement** is the criterion: the file, the
  row, and the face read the same fact the same way — the substrate's
  hard rule 5 discipline extended to the validate half.
- **The process fix was the owner's steer** after the first pass spent
  six compile cycles discovering guards: pins before edits, the
  preflight before the first compile, scoped runs, one gate, and a
  line between cycles. The second pass of this session ran on it —
  the full gate was one background cycle.
- Gate, final tree: fmt 0, fmt-check 0, clippy 0, build 0, test 0;
  harness 972/111. Ledger well-formed (469 rows, 6 open) before this
  document.
- **Delivery:** the Tauri dmg rebuilt
  (`fleet_0.4.0-beta.1_aarch64.dmg`, 10.84 MiB — the bundle carries
  the `services` resources, so the sys1 `validate` task rides in it),
  installed to /Applications (the old app moved aside, not deleted),
  and the face relaunched on it — the running process serves the new
  document (the proof fact reads `basis: validated` with its source
  from the pane's own API). The local machine's face is the new build.
