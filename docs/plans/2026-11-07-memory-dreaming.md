---
status: active
created: 2026-11-07
updated: 2026-11-07
slug: memory-dreaming
---

# Plan: memory dreaming — validated inferences, evidence pointers, and the tier cap

## Context

The two 2026-11-07 deep dives verified the substrate
(`docs/architecture/deep-dive-2026-11-07-memory.md`) and mapped it
against Anthropic's "The components of agent memory"
(`docs/architecture/deep-dive-2026-11-07-memory-dreaming.md`). Three
improvement areas carry weight, and this plan builds them together
because the first unblocks the second:

- **No evidence link.** A fact carries `basis` (confirmed-by /
  inferred) but no pointer to the session that produced it
  (`Remember`, `memory.rs:3994`), so nothing can cheaply check a fact
  against the ground truth hard rule 5 already guarantees on disk.
- **No dreaming.** The consolidation cycle is deterministic
  janitorial work; no pass ever validates a fact against its
  transcript, and 422 of 639 facts are inferred with 89% of the
  corpus never recalled.
- **The protected tier overran its budget.** 201 facts render ≈15k
  tokens against `ASSEMBLY_TOKEN_BUDGET` (2048) — the tier is
  contractually never dropped, so the budget now governs only recall
  and working memory.

The owner's steer: sys1 validates the inferences. The plan's shape
follows the article's measured findings and the house's own seams.

Decision-model check: validation is judged **per fact against its
evidence** — that is a sys1 job by the house definition, and this
plan declares it rather than leaving it unnamed. The `validate`
task_spec rides the `curate` task's pattern (the pair on the wire, a
verdict with confidence, the route cache, calib rows). No other
decision seam in this work is per-item.

## Direction lock

- **The transcript is the ground truth.** Validation without evidence
  is a plausibility vote — the Mt. Moon note was wrong from the start
  and read as plausible. No resolvable source ⇒ `unverifiable`, the
  fact stays inferred, the skip is named. Fail-safe by construction:
  validation is an upgrade, so a lane that cannot answer loses
  nothing.
- **Machine judgment never writes confirmed-by.** That word is the
  owner's. The third basis value `validated` is the record:
  evidence-checked by the lane, no owner authority claimed.
  `validated` over `judged` — the word is taken by the plan
  vocabulary's `judged` status (`judge_plan`/`plan_judge`), one word,
  one meaning.
- **The corpus's mass is not the queue.** The `recall_hit` calib rows
  rank which inferred facts the model actually sees; validation
  spends there first, bounded per pass.
- **Auto-landing is licensed by the measured set.** Until the
  validator separates known-wrong from known-right at a named bar
  (C0), a `contradicted` verdict lands as a proposal, not a
  supersession.
- **The cap names its overflow.** Overflow protected facts rank in
  recall as marked candidates — reachable, never silently absent,
  never twice in one block.
- **No new face surface.** The memory surface's existing pane card
  grows two renderings (source, the validated label); the house
  mockup rule governs new surfaces, not pane fields.

## Approach

The build rides the module's own machinery end to end: one writer
(`Remember::parse`/`render_document`), one rewriter (`rewrite_field`),
the provenance permission model (`Memory::supersede`), the cycle's
candidate filters, the consolidator thread, and the `curate` task's
composition (lane call, route cache, calib row) as the validate
task's template.

1. **The measured set first (C0).** A fixture corpus of known-wrong
   facts drawn from `docs/known-issues.md`'s closed rows beside
   known-right confirmed-by facts; the pass must separate them at a
   named bar before any auto-landing is enabled.
2. **The source field (C1).** Optional `source` on the write path and
   `FactEdit`, a session id the surfaces render.
3. **The `validated` basis (C2).** `Basis::parse` accepts it,
   `PROVENANCE_VALIDATED` (0.75) joins the provenance channel, the
   cycle's untouched set names it, supersede treats it per the
   recommendation below.
4. **The validate task (C3).** Sys1 judges the fact-transcript pair;
   verdict `supported` / `contradicted` / `unverifiable` with
   confidence; route-cached; one `calib` row each.
5. **The dream pass (C4).** `memory::dream`: the access-priority
   queue, source resolution, the verdict acted on (stamp, supersede
   post-gate, named skip), every action a calib row.
6. **Scheduling (C5).** `fleet memories --validate` on demand; the
   hourly thread runs it under a per-pass budget in chat, the TUI,
   and the face (the pass takes the lanes handle the process already
   holds; without one it degrades to named unverifiable skips).
7. **The tier cap (C6).** `PROTECTED_TOKEN_CAP` (1024) bounds the
   protected section; overflow ranks marked `tier-overflow`.
8. **The live proof (C7).** On the running face with sys1 live: a
   fact carrying its source is validated in place — the corpus file,
   the calib row, and the pane card agree — and the recap quotes all
   three.
9. **The gate (C8).** `cargo fmt --all --check`, `cargo clippy
   --all-targets -- -D warnings`, `cargo build --release`,
   `cargo test` — each exit code read directly, the whole suite.
10. **The docs (C9).** `docs/features/memory.md` gains the source
    field, the validated basis, and the dream pass;
    TECHNICAL-DOCUMENTATION's Module reference and
    FUNCTIONAL-SPECIFICATIONS's surfaces section name them.

Two recommendations this plan carries for the owner to confirm at
approval: a `validated` fact supersedes on a justification that names
its evidence (the slice-12 rule, one bar higher), and the cycle
leaves validated facts beside the confirmed-by ones (untouched).

## Use cases

- [ ] A: A note wrong from the start rides into every session and is
  trusted — the pass reads its transcript, answers `contradicted`,
  and the fact is superseded with a justification naming the session.
- [ ] B: A correct inference earns trust — its source resolves, sys1
  answers `supported`, the fact stamps `validated` and ranks at the
  validated provenance weight from the next load.
- [ ] C: A turn carries 201 protected facts at 15k tokens — the block
  carries the capped tier, the overflow ranks as marked candidates,
  and the block names the overflow count.
- [ ] D: A fact has no resolvable source — the pass names it
  `unverifiable`, the fact stays inferred, and nothing is lost when
  the lane cannot answer.

## UX routes

No new surface. The memory surface's existing pane card
(`src/gui/assets/app.js`, fed by `memory_read`,
`src/gui/bridge.rs:5178`) renders the `source` field and the
`validated` basis label; the rail sections and the needs-you queue
are unchanged; the CLI gains `remember --source` and
`fleet memories --validate` cards.

## Acceptance criteria

- [ ] **C0 — the measured set separates.** A fixture corpus of
      known-wrong facts (drawn from `docs/known-issues.md`'s closed
      rows) beside known-right confirmed-by facts is separated by the
      validate pass at or above a named confidence bar before any
      auto-landing is enabled, pinned by a test in `src/memory.rs`.
- [ ] **C1 — the source field round-trips.** `Remember::parse` and
      `FactEdit::parse` in `src/memory.rs` accept an optional
      `source`, `render_document` writes it, `fleet remember --source`
      lands it, and the face's pane card renders it — pinned by
      `src/memory.rs` tests and a `tests/face/harness.mjs` case.
- [ ] **C2 — the validated basis is a first-class value.**
      `Basis::parse` in `src/memory.rs` accepts `validated`, a
      validated fact scores the provenance channel at
      `PROVENANCE_VALIDATED` (0.75), the consolidation cycle's
      untouched set names it, and supersede applies the ruling —
      pinned by `src/memory.rs` tests.
- [ ] **C3 — the validate task answers and is auditable.** The sys1
      validate task returns `supported` / `contradicted` /
      `unverifiable` with a confidence for a fact-transcript pair
      through the lane call, the verdict caches in the route cache
      under the pair's hash, and every judgment writes one calib
      `Row::Validate` — pinned by tests with the mock lanes in
      `src/memory.rs`.
- [ ] **C4 — the dream pass acts on its verdicts.** `memory::dream`
      in `src/memory.rs` walks the access-priority queue (ranked by
      calib `recall_hit` rows, inferred-only, bounded per pass),
      names its unverifiable skips, stamps `validated` on supported,
      and supersedes contradicted facts with a justification naming
      the session and the verdict after C0's bar is met — pinned by
      `src/memory.rs` tests.
- [ ] **C5 — the pass runs on demand and on schedule.**
      `fleet memories --validate` runs one pass and prints what it
      did, and the hourly consolidator thread runs the pass under a
      per-pass budget in `src/main.rs` chat, `src/tui/mod.rs`, and
      `src/gui/bridge.rs` — the live index follows the stamps on the
      next load, and a process without lanes degrades to named
      skips.
- [ ] **C6 — the tier cap bounds the block honestly.**
      `Memory::protected` in `src/memory.rs` returns the name-sorted
      set within `PROTECTED_TOKEN_CAP` (1024), the overflow ranks in
      recall marked `tier-overflow` without entering a block twice,
      and the assembled block names the overflow count — pinned by
      `src/memory.rs` tests.
- [ ] **C7 — the live proof.** On the running face with sys1 live: a
      fact carrying its source is validated in place — the corpus
      file, the calib row, and the pane card agree — quoted in the
      recap.
- [ ] **C8 — the gate.** fmt, clippy, release build, test — exit
      codes read directly, the whole suite.
- [ ] **C9 — the docs.** `docs/features/memory.md` gains the source
      field, the validated basis, and the dream pass;
      TECHNICAL-DOCUMENTATION's Module reference and
      FUNCTIONAL-SPECIFICATIONS's surfaces section name them.

## Files to be touched

**Core:**
- `src/memory.rs` — the source field, the validated basis and
  `PROVENANCE_VALIDATED`, the validate task's fixture and pass
  (`memory::dream`), the tier cap and the overflow ranking, the tests.
- `src/calib.rs` — `Row::Validate`, `ACTION_VALIDATE`.
- `src/cli.rs` — `remember --source`, `memories --validate`.
- `src/main.rs` — the command wiring, `spawn_consolidator`'s lanes
  handle.
- `src/tui/mod.rs`, `src/gui/bridge.rs` — the spawn sites.

**Face:**
- `src/gui/bridge.rs`, `src/gui/assets/app.js` — the pane card's
  source and validated renderings.

**Tests:**
- `tests/face/harness.mjs` — the pane card case.

**Documentation (see Linked artifacts below):**
- `docs/features/memory.md` — update
- `TECHNICAL-DOCUMENTATION.md` — Module reference
- `FUNCTIONAL-SPECIFICATIONS.md` — The surfaces

## Out of scope

- **Mass backfill.** The 639 existing facts get no retroactive
  `source` pointers; the pass skips and names them, and the queue is
  the access ranking, not the corpus.
- **The article's other dreaming movements.** Wiki/hub building,
  cross-session gap-filling, and promotion are later plans (areas 5
  and 7 of the dreaming doc).
- **Demotion of protected facts.** The cap admits overflow to the
  ranking; retyping or demoting them is a separate ruling.
- **blend1 as the validation judge.** sys1 judges; blend1 stays the
  grounded writer.
- **Auto-landing before the gate.** Contradicted verdicts supersede
  only after C0's bar is met; until then they land as named skips
  and rows.
- **The working tier.** Its disuse is census finding 5, untouched
  here.

## Verification

```sh
python3 scripts/ki_ledger.py docs/known-issues.md   # well-formed
cargo test                                           # C0–C6 pins
node tests/face/harness.mjs                          # C1's pane case
cargo fmt --all --check && cargo clippy --all-targets -- -D warnings \
  && cargo build --release && cargo test             # C8, codes read
fleet memories --validate                            # C5 on the live corpus
```

C7's live pass: the running face, sys1 live, a fact with a resolvable
source — the pass stamps `validated`; the corpus file, the calib row,
and the pane card agree. Quoted in the recap.

## Linked artifacts

- `docs/features/memory.md` — the source field, the validated basis,
  the dream pass, the tier cap.
- `TECHNICAL-DOCUMENTATION.md` section "Module reference" — the
  dream pass and the new constants.
- `FUNCTIONAL-SPECIFICATIONS.md` section "The surfaces" — the pane
  card's fields and the CLI verbs.
- `docs/architecture/deep-dive-2026-11-07-memory-dreaming.md` — the
  design record this plan implements.

## Risks

- **Sys1's verdict quality decides trust.** C0's measured gate is the
  license; a validator that cannot separate the fixture corpus lands
  nothing, and the panel upgrade (curate's best-of-3) is the
  escalation if single-judge confidence proves thin.
- **The cap's selection is name-sorted.** Deterministic and
  prefix-cache-stable, but arbitrary about which protected facts keep
  full lines; date-descending recency is the alternative if the
  owner prefers it — one constant's difference, raised at approval.
- **`PROVENANCE_VALIDATED` (0.75) and the two rulings are
  recommendations.** The supersede rule and the cycle's treatment of
  validated facts are confirmed or amended at approval, before C2
  builds.
- **Route-cache growth.** Validate verdicts cache under pair hashes
  like curate's; bounded by the per-pass budget and the rekey.
- **No lanes, no validation.** A process without a lanes handle
  degrades to named unverifiable skips — the named degradation, not a
  silent one.

## Notes

No hard dependencies: the plan builds on the substrate as it stands
on `face-plugin-host`, and the two deep-dive docs are its design
record. The naming ruling and its reasoning are recorded in the
dreaming doc's area 1: `validated` over `judged` (taken by the plan
vocabulary), the act is the calib op, the verdict is the judgment's
vocabulary, and the basis value names the fact's standing. The
article's own measured winning behavior — confirmation upgraded
after a grader checked it — is the pattern the `validated` stamp
implements. Known-wrong fixture facts come from the ledger's closed
rows with their KI numbers cited in the test, so the fixture is
auditable against `docs/known-issues.md`.
