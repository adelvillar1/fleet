---
status: draft
created: 2026-11-11
updated: 2026-11-11
slug: deck-semantic-clustering
---

# Plan: the deck's semantic clustering — long sources into infographic blocks (KI-252's last remainder)

## Context

KI-252's blueprint (the owner's 2026-10-19 deck vector layer) named three
bounded moves; moves 1–3 landed (the layout template index, the Matryoshka
pre-filter, the asset library — `docs/plans/2026-10-24-deck-vector-moves-2-3.md`,
completed). The piece the row was opened for that no plan has taken is
**semantic clustering of long sources into infographic blocks**: `grep -c
cluster` is 0 in both `src/deck.rs` and `src/memory.rs`. Today the deck's
content comes from the calling agent's research and its layout from the
grader's arithmetic; a long source is not grouped into blocks by meaning.
This plan builds that grouping over the embedder the routing triad already
runs, so a long document becomes a deck whose blocks are semantic units,
each opening with its conclusion (the Pyramid Principle the deck already
enforces, `src/deck.rs:15`).

## Decision-model check

No new sys1 judgment shape fits the clustering itself: it is deterministic
arithmetic — embed candidate blocks (sem1's `HttpEmbedder`, the loopback
llama-server at `127.0.0.1:8901`, `src/memory.rs:1233`), group by cosine,
bound the group count by the deck's `Bounds`. The embedder is the value
lane (sem1); the grouping is pure stdlib Rust, no foreign runtime (hard
rule 12). Deriving a block's *conclusion* from its cluster may ride the
hosted prose leg (blend1 `generate`, the Phase-4 seam) as a named fallback
to a deterministic extractive lead — but the clustering and its gate are
judgment-free and testable without a lane.

## Approach

One path: a pure clustering core over the existing embedder, wired into the
deck tool's suggest mode, gated by a labeled measurement.

1. **Split** a long source into candidate blocks (paragraph/section
   boundaries), each a self-contained span. Bounded — a source larger than
   the candidate cap is sampled by section, named, never silently whole.
2. **Embed** each candidate through `HttpEmbedder::embed`. An absent
   embedder is a named degradation (the deck falls back to the calling
   agent's research, today's path), never a silent failure.
3. **Cluster** deterministically: agglomerative cosine grouping (stdlib,
   no dependency), merging the nearest pair until the group count fits the
   deck's `Bounds::for_type` node ceiling or the cosine floor is reached.
   The algorithm is pure and fixture-testable.
4. **Map** each cluster to a `Node` opening with its conclusion: the
   extractive lead (the highest-centrality candidate) by default, the
   hosted prose leg's summary when it answers. The deck grader (`check`,
   `review`, `auto_fit`) certifies the result exactly as it certifies any
   deck — a block that breaks the frame is refused by the same arithmetic.
5. **Measure** the clustering against a labeled fixture (a source with
   known semantic groups) before shipping the default, the way move 2
   gated the two-stage recall: the grouping must recover the labeled
   clusters above a named threshold, or the feature stays opt-in.

## Acceptance criteria

- [ ] **C0 — the split/embed seam.** A long source becomes bounded
      candidate blocks, each embedded through `HttpEmbedder`; an absent
      embedder is a named degradation to today's research path, never a
      silent failure. Pinned by a test driving the seam with a scripted
      embedder and with the embedder absent.
- [ ] **C1 — deterministic semantic clustering.** The pure cosine grouping
      clusters candidates into no more than the deck's node ceiling,
      recovering known semantic groups on a fixture; no foreign runtime.
      Pinned by a fixture with labeled groups asserting the recovered
      partition.
- [ ] **C2 — clusters become certified blocks.** Each cluster maps to a
      `Node` opening with its conclusion, and the deck grader (`check` /
      `review` / `auto_fit`) passes the result; a frame-breaking block is
      refused by name. Pinned by a test rendering a clustered deck through
      the grader.
- [ ] **C3 — the measurement gate.** A committed, `#[ignore]`d measurement
      grades the clustering against the labeled fixture and prints the
      recovery score; the default ships only above the named threshold,
      else the feature is opt-in (`deck.clustering`, default off). Pinned by
      the harness's existence and the setting's round-trip.
- [ ] **C4 — the surface seam + the gate.** The deck tool's suggest mode
      carries the clustering line; `cargo fmt --all --check`, `cargo clippy
      --all-targets -- -D warnings`, `cargo build --release`, `cargo test`
      all exit 0.

## Files to be touched

- Backend: `src/deck.rs` (the clustering core + the Node mapping),
  `src/memory.rs` (the embedder seam, reused), `src/tools.rs` (the deck
  tool's suggest mode), `src/settings.rs` (the `deck.clustering` setting).
- Tests: the deck clustering unit tests + the `#[ignore]`d measurement.
- Docs: the linked artifacts below; `docs/known-issues.md` (KI-252 closes
  when this lands).

## Out of scope

- **Moves 1–3** — already landed; this plan does not reopen them.
- **The cross-modal image leg** — the st-worker provider's own later slice
  (KI-252 records it), not this plan.
- **A new embedding model or runtime** — the existing sem1 embedder is the
  seam; no foreign dependency.
- **Re-tuning the Matryoshka pre-filter** — move 2's measurement stands.

## Verification

- C0/C1/C2: scoped deck tests (`cargo test -p fleet --lib deck::`), then the
  whole gate.
- C3: the `#[ignore]`d measurement run live against the fixture and the
  embedder; the score printed and compared to the threshold.
- C4: the four-leg gate whole, exit codes read directly (KI-014).

## Linked artifacts

- `docs/features/deck.md` (or the deck's feature doc) — the clustering
  seam, the measurement gate, the opt-in setting.
- `TECHNICAL-DOCUMENTATION.md` section "Module reference" — the deck's
  clustering core.
- `FUNCTIONAL-SPECIFICATIONS.md` — the deck tool's suggest mode carries
  semantic clustering.

## Risks

- **Clustering quality is the gate, not an assumption.** Agglomerative
  cosine grouping may not recover the labeled groups on real sources; the
  measurement decides the default, and a poor score keeps it opt-in (the
  move-2 lesson: the gate requires a measured win, not a hoped one).
- **The embedder is a live dependency.** Its absence must degrade by name;
  a clustering that silently no-ops on a down embedder is the defect the
  self-containment ruling forbids.
- **Conclusion extraction.** The extractive lead may be weak; the prose
  leg is a named fallback, never a silent requirement.

## Dependencies

- `docs/plans/2026-10-24-deck-vector-moves-2-3.md` (completed) — the
  embedder seam and the measurement-gate pattern this plan reuses.
- KI-252 — the row this plan closes; the blueprint's last unbuilt step.