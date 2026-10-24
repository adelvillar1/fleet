---
status: draft
created: 2026-10-24
updated: 2026-10-24
slug: deck-vector-moves-2-3
---

# Plan: the deck's vector moves 2 and 3 — the Matryoshka pre-filter and the asset library (KI-252's remainder)

## Context

KI-252's blueprint names three bounded moves; move 1 (the layout
template index, `deck::suggest_layout`, the tool's suggest mode) landed
2026-10-19, and the action card and self-correction loop landed since.
The two moves this plan covers are the remainder:

- **Move 2 — the Matryoshka pre-filter.** EmbeddingGemma's vectors
  truncate: a 128-dimension slice of the 768-dimension vector carries
  most of its neighborhood for coarse ranking. The corpus recall's
  fan-out (every fact's vector scored per query) is the place this
  pays: slice to 128 for the fan-out, full-width for the final rank of
  the top-K, measured against today's single-pass recall.
- **Move 3 — the asset library.** An icon/graphic index keyed by
  embedded captions, with the deck grader certifying aspect and spacing
  before a match is placed — so a suggestion that would break the frame
  is refused by the same arithmetic that refuses overflow.

Both moves are gated, and the gates are the plan's criteria: move 2
ships only if the two-stage recall measures within tolerance of the
single-pass on the real corpus; move 3 ships with the honest absence
named when the owner's asset directory is empty or unset.

## Approach

The increments and the criteria they carry: increment 1 is move 2's
slice and two-stage rank (C0); increment 2 is move 2's measurement gate
(C1); increment 3 is move 3's index and search seam (C2); increment 4
is move 3's grader certification and the deck tool's asset mode (C3);
C4 reconciles the ledger — KI-252 closes when both moves land or carries
the named remainder of whichever gate refused.

### Increment 1 — the slice and the two-stage rank (C0, move 2)

A slice helper over the stored vectors (`&v[..128]` is the whole trick
Matryoshka training buys, expressed once with its why in a comment) and
a two-stage path in the corpus recall: score every fact's slice against
the query's slice, take the top-K (K = 24, named), re-rank those
full-width. Behind a setting (`memory.vectors.two_stage`, default off)
so the single pass remains the contract until the measurement says
otherwise.

### Increment 2 — the measurement gate (C1, move 2)

One harness: the real corpus, a fixed query set drawn from recall's own
instructions (the eval corpus's recall scenarios), recall@10 of
single-pass versus two-stage, and the wall clocks. The gate the row
names: two-stage recall@10 within 2% of single-pass, with the fan-out
costing measurably less than the full-width pass on corpora past 500
facts (below that, the pre-filter is overhead and the setting stays
off). The measurement's numbers land in the row and the recap, whatever
the gate decides.

### Increment 3 — the asset index and its seam (C2, move 3)

The asset library: a directory the owner points at
(`FLEET_ASSETS_DIR`, or the chat's working folder's `assets/`), each
asset indexed by its filename and any sidecar caption, embedded through
the same probed embedder, stored beside the corpus (the sem1 store's
own project namespace: `assets`). The seam, in `src/deck.rs` beside
`deck::suggest_layout`: `deck::find_assets(intent, k) ->
Vec<(name, path, score)>` ranked by cosine against the query's own
embedding, with an honest empty answer when the directory is unset or
empty — the named absence, never a fabricated match. C3 is this
increment's deliverable, in `src/deck.rs`: the seam exists and the
empty-directory answer is pinned by test as the named absence it is.

### Increment 4 — the grader certifies before a match is placed (C3, move 3)

The deck grader gains the asset check: a placed asset's aspect ratio
and whitespace budget must hold the frame's own bounds (the same
arithmetic that refuses overflow), so a match that would break the
frame is refused with the reason named. The `deck` tool's suggestion
output gains the asset line: what matched, at what score, and whether
the grader certified the fit. C3 is this increment's deliverable,
pinned by tests over fixture assets: a matching caption with a breaking
aspect is refused by name; a certified fit is placed; the tool carries
the asset mode.

### Close-out — the gate and the ledger (C4)

C4's steps, in order: run the gate's four commands and read each exit
code directly — `cargo fmt --all --check`, `cargo clippy --all-targets
-- -D warnings`, `cargo build --release`, `cargo test`; then update
`docs/known-issues.md`'s KI-252 row and this plan's checkboxes in the
same commit, carrying the measurement numbers the gate produced. The
criterion is the commit that shows both.

## Acceptance criteria

- [ ] **C0 — the slice and two-stage rank.** The corpus recall offers a
      two-stage path (128-dim fan-out, full-width top-K re-rank) behind
      a default-off setting, with the slice's why in a comment.
- [ ] **C1 — the measurement gate.** A committed harness — a test
      module in `src/deck.rs`'s suite, `#[ignore]`d where the corpus is
      absent — prints recall@10 two-stage versus single-pass and both
      wall clocks over the real corpus; the `memory.vectors.two_stage`
      default follows the printed gate, and the numbers land in the
      KI-252 row and the recap either way.
- [ ] **C2 — the asset seam.** `deck::find_assets` ranks the owner's
      asset directory by embedded caption, and an unset or empty
      directory answers with the named absence; by test.
- [ ] **C3 — the grader certifies.** A placed asset's aspect and
      whitespace hold the frame's bounds or the placement is refused
      with the reason named; the deck tool carries the asset mode; by
      test over fixture assets.
- [ ] **C4 — the close-out and the ledger.** `docs/known-issues.md`'s
      KI-252 row and this plan's checkboxes carry the same final status
      in the same commit — closed with both moves landed and the
      measurement numbers, or open carrying the named gate refusal of
      whichever move its measurement refused — and the full gate's four
      commands exit 0 at the close-out.

## Files to be touched

`src/memory.rs` (the slice, the two-stage recall, the setting);
`src/deck.rs` (the asset index, the seam, the grader's asset check);
`src/tools.rs` (the deck tool's asset mode); the tests beside each.

## Out of scope

- Cross-modal image embeddings (the st-worker's interleaved format) —
  captions only; images-as-vectors is the st-worker provider's own
  later slice.
- The blend1 routing lane's integration with the asset library — the
  lane's synthesis already exists; wiring it to place assets is the
  blueprint's step after this one.
- The layout index's re-tuning — move 1's eight descriptions stand.

## Verification

The recall measurement is the gate and is reported either way; unit
tests pin the slice, the seam's empty answer, and the grader's
refusals; the full gate's four commands by exit code at close-out.

## Linked artifacts

- `docs/known-issues.md` — KI-252
- `docs/plans/2026-10-19-deck-triad-autofit-and-layout-index.md` —
  move 1's landed record
