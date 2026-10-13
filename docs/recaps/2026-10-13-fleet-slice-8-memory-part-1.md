# Slice 8 part 1 — the semantic layer: substrate, retrieval, routing, protection (2026-10-13)

**Plan:** `docs/plans/2026-10-13-fleet-everyday-harness.md` (slice 8, part 1 of 2).
**Feature:** `docs/features/memory.md`.
**Ledger:** KI-057 (the sibling parser), KI-058 (unrelated-pair cosine),
KI-059 (the protected tier's literal type match), KI-060 (descriptions
unsanitized/unbounded — part 2's writer).

## What landed

`src/memory.rs` — the semantic layer's read path.

- **Substrate.** `memories/`, git-tracked: one markdown file per fact with
  frontmatter (`name`, `description`, `type`, `date`, `shape`,
  `recall-count`, `superseded-by`, `valid-until`, `basis`), hand-parsed by
  a sibling of the skills parser (the subsets genuinely differ — bare
  `superseded-by:`, folded scalars, bracketed `shape:` lists — KI-057).
  Malformed facts are skipped by name with one logged line; `name` must
  equal the file stem; `superseded-by` or a past `valid-until` retires a
  fact out of recall. A bad fact is a revert — the substrate slice 12's
  gated self-modification needs. Nine seed facts, every claim cited.
- **Retrieval, the vector path probed not assumed.** Hybrid rank over
  named constants: vector 0.55 · keyword 0.25 · importance 0.10 ·
  decay 0.10, shape boost 0.15, ceiling 8, floor 0.20, decay halflife
  90 days; `basis` is provenance, not rank. The `Embedder` trait ships
  two implementations: `HttpEmbedder` — a loopback call to
  `http://127.0.0.1:8901/v1/embeddings` (overridable by
  `FLEET_EMBEDDING_ENDPOINT`) against the machine's live llama-server,
  which serves embeddinggemma at 768 dims (verified: a real round trip
  returns vectors) — and `KeywordOnly`, the correct degradation, which
  ignores cached vectors so doctor's naming can never contradict recall.
  The probe is one real embed, not a health poll. The index
  (`memories/.recall-index.json`, gitignored) is keyed to the embedder's
  identity and dimension, rebuilt incrementally on mtime/size change,
  written atomically.
- **Doctor names the path.** `recall ok vector path live — … (768-dim
  embeddings serve recall)` or the exact sentence `embeddings
  unavailable — recall is keyword-only`. Never silent — the live system's
  failure this fixes.
- **Routing.** Wave-D shape classification over the fork's
  `memories/shapes.json` keywords verbatim (deploy, build, deps, docs,
  rendering, state-sync, change), first-match-wins, same-shape boost; an
  unclassifiable instruction answers from the surface alone — the
  documented fallback.
- **Feedback.** A recall hit bumps `recall-count` through a bounded
  frontmatter-preserving rewrite (body and every other field
  byte-identical), re-ranks, and appends a `recall_hit` calibration row
  (fact, shape, score) — reader, surface card, and `fleet calib` updated.
- **The protected tier.** Types `user` and `feedback` are admitted
  unconditionally and assembled first; the injection budget can never
  evict them. Context is assembled, not dumped: protected tier, then
  ranked recall (the strongest always enters, the rest while they fit,
  the drops named), then part 2's working-memory seam.
- **Surface.** `fleet memories` (cards: name/type/shape/basis/recall-count/
  valid-until/path, backend named), `--recall "<instruction>"` (the ranked
  cards with each score's channels, then the assembled block, then the
  drops — marked a dry run, zero file bytes touched), `--shape <name>`.
  No new TUI surface; the assembly enters the agent's context in part 2.

## Evidence

- Gate, parent-run after each commit: `cargo build --all-targets` exit 0,
  **zero warnings**; `cargo test` exit 0 — **264 unit + 61 integration
  passed, 1 ignored** in the final state (baseline 232 + 56; nothing
  deleted). rustfmt/clippy still absent (KI-007).
- The vector path is live, not assumed: `/health` answers ok and a real
  `POST /v1/embeddings` returns 768-dim vectors; doctor names the path;
  `FLEET_EMBEDDING_ENDPOINT` pointed at a dead port flips doctor to the
  degradation sentence and back (parent-verified both directions).
- The on-disk index self-healed from the pre-key format to
  `{"embedder": "http://127.0.0.1:8901/v1/embeddings", "dims": 768, …}`
  on first load — the dimension key working on real data.
- A live `--recall` run classifies the shape, ranks, and prints the
  assembled block with the protected tier first; the seed facts were read
  claim-by-claim (accurate, cited).
- Negative probes: the budget test fails at 512 (`recalled 3 ≠ 4`) and
  passes at 2048; a dimension-mismatched index is rebuilt, not served.

## Review cycle

- **Stage 1 (spec, step-5-preview): SPEC-COMPLIANT** — all 12 checks;
  the shape taxonomy diffs clean against the fork's keywords, the
  rewrite's byte-preservation is tested, the dry-run zero-write is
  tested, and the two doctor states cannot disagree (keyword-only never
  touches the index).
- **Stage 2 (quality, step-5-preview): REQUEST_CHANGES** — two
  Importants, both fixed in `247a9b6`:
  1. the 512-token assembly budget starved ranked recall on the real
     corpus (the protected tier measures ~453 of it; the second ranked
     fact costs ~85, so exactly one survived — the parent's live run was
     the shipped constant behaving as built). Raised to 2048 with a
     seed-shaped pin that fails at the old value;
  2. the index carried no dimension key, so an embedding-model swap
     would silently zero the vector channel while doctor named it live —
     the exact silent degradation this slice exists to prevent. The
     index now keys on embedder + dims and invalidates on mismatch
     (including the pre-key format).
  Plus the minors: `FLEET_EMBEDDING_ENDPOINT`, per-fact embed failures
  logged, the dead protected `Score` dropped (the tier is admitted
  unconditionally and name-sorted — a recall-count ordering of identity
  facts has no design rationale), rewrite temps gitignored, and KI-060
  logging the description bound for part 2.

## Not done (part 2)

- The write path: `remember` (brain_remember-shaped), `basis` required,
  `valid-until` for session-scoped facts, the three speeds (working /
  durable / episodic).
- The consolidation cycle on its timer: compress, supersede, forget,
  every action a calibration row.
- Provenance as the permission model: a confirmed-by fact cannot be
  modified without approval; an inferred one is supersendable by a
  justified proposal.
- The turn-path wiring: the assembled context enters the agent's runs.
