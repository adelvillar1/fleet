---
status: completed
created: 2026-11-08
updated: 2026-11-08
slug: hybrid-retrieval
---

# Plan: the hybrid retrieval layer — BM25, RRF fusion, code outlines, and the agent's search discipline

## Context

The zvec/zvec-grep deep dive
(`docs/architecture/deep-dive-2026-11-08-zvec-and-zvec-grep.md`) found five
absorbable gaps in fleet's retrieval: there is **no ranked lexical route** (the
keyword channel is set coverage, not BM25), the **corpus search is vector-only**
(no rank fusion), the agent's search tools carry **no retrieval discipline**,
there is **no first-class hybrid search tool**, and the **code search has no
structure** (grep finds lines and `repo_map` pattern-matches definitions; no
parser produces a symbol outline). This plan absorbs all five. ANN/quantization
and the remote-embedding consent model are explicitly not now.

## Approach

Implement the retrieval layer in Rust. A new `src/lexical.rs` owns BM25/IDF —
the index over a document set, the scoring (term frequency, inverse document
frequency, length normalization), and the ranked query. A new `src/fusion.rs`
owns Reciprocal Rank Fusion (`RRF_K = 60`) over two ranked lists, the standard
parameter-light fusion zvec-grep uses.

The corpus search gains the lexical route with **no per-search lane call**: the
lane's `manifest.json` already carries each entry's `key`, `sha256` and `text`,
and `vectors.bin` carries the vectors, so the Rust side reads the cached store
directly — BM25 over the manifest text, cosine over the vectors, RRF between
them. The lane's existing hash-gated refresh keeps the store current, and the
lexical index rides the same invalidation.

The code search gains structure: a new `src/outline.rs` parses the workspace's
source with **tree-sitter** and emits a structural outline per file — symbols,
signatures, members, calls, capped — the same retrievable unit zvec-grep uses.
The outlines ride the lexical index beside the corpus entries, so a hybrid query
retrieves a symbol by its signature rather than a line by its text. The grammar
set is bounded (Rust, Python, Markdown, TypeScript/TSX, JSON, Bash); TOML is
deliberately absent — the only TOML grammar targets the legacy tree-sitter API
and would force a second, incompatible runtime. The grammars ship in their
crates and are compiled by the build, which is hard rule 12's "built by the
install" — the component ships and is built, never supplied by a checkout. If
the C build proves a problem on the owner's machine, the named fallback is a
structural outline from fleet's own brace and stem scan, not a second parser
dependency.

The agent's `search` and `recall` tool descriptions gain zvec-grep's routing and
stopping discipline — semantic when wording or location is unknown, native
grep when exact lookup suffices, treat sufficient returned content as read,
stop when the evidence is sufficient — and a `hybrid_search` tool exposes the
fused ranking to the agent.

The Rust recall's keyword channel is deliberately **not** rewritten: its set
coverage is part of the tuned six-channel weighting (direction outranks
proximity), and changing it would move the weights. The BM25 route is a new
peer for the corpus and the code, not a replacement for the memory ranker.

Decision-model check: no sys1 fit — BM25, the tokenization, the outline
extraction, and RRF are all deterministic, and the query's lexical terms come
from fleet's own stemmer, not a model call. The lane stays the embedding owner;
no judgment is needed.

High-level steps:

0. `src/lexical.rs`: the BM25 index, scoring, and query (C0).
1. `src/fusion.rs`: the RRF fusion (C1).
2. The corpus lexical index over the lane's manifest, cached beside the recall index (C2).
3. The hybrid search over the cached store, fused with RRF (C3).
4. `src/outline.rs`: the tree-sitter structural outline per source file (C4).
5. The outlines ride the lexical index; the hybrid search retrieves a symbol by signature (C5).
6. The agent's retrieval protocol in the `search` and `recall` descriptions (C6).
7. The `hybrid_search` tool (C7).
8. The gate (C8).
9. The docs (C9).

No face surface is involved, so the mockups-first rule does not apply; the
retrieval contract is drawn in the deep-dive doc and this plan. Run
`scripts/edit-preflight.sh` before the first compile: the tool-surface bounds
(a new tool spec joins `src/agent.rs`), the face harness's fixture counts (no
face fixture changes; the tool is model-facing), and the new C build dependency
the grammars add.

## Acceptance criteria

- [x] **C0 — BM25 ranks a fixture corpus.** `src/lexical.rs` implements BM25 with inverse document frequency, term frequency, and length normalization, and a unit test ranks a fixture corpus so a document matching a rare term outranks one matching a common term.
- [x] **C1 — RRF fuses two ranked lists.** `src/fusion.rs` implements Reciprocal Rank Fusion with `RRF_K = 60`, and a unit test pins the fused order of two ranked lists.
- [x] **C2 — the corpus lexical index is cached and invalidated.** The lexical index is built from the lane manifest's entry text and cached beside the recall index, and a unit test pins that a changed `sha256` rebuilds the entry.
- [x] **C3 — the hybrid search fuses lexical and vector over the cached store.** The hybrid search reads the lane's manifest and `vectors.bin` directly, BM25s the query, cosine-searches the vectors, and fuses with RRF — with no per-search lane call — and a unit test pins that a lexical-only match outranks a vector-only miss.
- [x] **C4 — source files yield structural outlines.** `src/outline.rs` parses a fixture source file with tree-sitter and emits its symbols, signatures, members and calls within the caps (24 calls, 32 members, 180 chars/line, 3,600 chars/outline), and a unit test pins the outline's shape.
- [x] **C5 — the hybrid search retrieves a symbol by signature.** The outlines ride the lexical index beside the corpus entries, and a unit test pins that a query naming a symbol's signature outranks a query matching only its body text.
- [x] **C6 — the agent's search tools carry the retrieval discipline.** The `search` and `recall` tool descriptions state when to use semantic versus exact lookup, to treat sufficient returned content as read, and to stop when the evidence is sufficient, and a unit test asserts the text.
- [x] **C7 — a `hybrid_search` tool exposes the fused ranking.** The agent can call `hybrid_search` over a workspace, and a unit test drives it end to end over a fixture store and asserts the fused, source-linked results.
- [x] **C8 — the gate is green.** The four-leg gate runs clean: `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings`, `cargo build --release`, and `cargo test` each exit 0 on the merged tree.
- [x] **C9 — the docs land.** `docs/features/blend1.md` (or a new `docs/features/retrieval.md`) documents the lexical route, the RRF fusion, the code outlines, the hybrid tool, and the agent's retrieval discipline, and `TECHNICAL-DOCUMENTATION.md`'s module reference names `lexical.rs`, `fusion.rs`, and `outline.rs`.

## Files to be touched

**Backend:**
- `src/lexical.rs` — new: the BM25 index, scoring, and query.
- `src/fusion.rs` — new: the RRF fusion.
- `src/outline.rs` — new: the tree-sitter structural outline extraction.
- `src/search.rs` or `src/agent.rs` — the hybrid search over the cached store; the `hybrid_search` tool spec and handler.
- `src/tools.rs` — the `search`/`recall` descriptions; the tool-surface bounds.
- `Cargo.toml` — the `tree-sitter` crates and the bounded grammar set.

**Tests:**
- `src/lexical.rs`, `src/fusion.rs`, `src/outline.rs` (inline `tests`) — the BM25 ranking, the RRF order, the outline shape.
- `tests/integration.rs` — the hybrid search end to end; the declaration list.

**Documentation:**
- `docs/features/blend1.md` or a new `docs/features/retrieval.md`.
- `TECHNICAL-DOCUMENTATION.md` — the module reference.

## Out of scope

- ANN indexes and quantization — fleet's scale does not justify them, and KI-459 measured the pre-filter as never cheaper.
- The remote-embedding consent model — fleet's embeddings are local by hard rule 12.
- Rewriting the Rust recall's six-channel keyword term — the weighting is deliberate.

## Verification

**C0–C2, C4:** `cargo test -p fleet --lib lexical`, `cargo test -p fleet --lib fusion`, `cargo test -p fleet --lib outline` — the inline unit tests.
**C3, C5, C7:** the integration tests in the same run; drive `hybrid_search` over a fixture store and assert the fused, source-linked results.
**C6:** the tool-description assertion in the same run.
**C8:** `cargo fmt --all --check`; `cargo clippy --all-targets -- -D warnings`; `cargo build --release`; `cargo test`. Read the exit codes, never through a status-losing pipe (KI-014).
**C9:** read the docs; the feature doc names the lexical route, the fusion, the outlines, the tool, and the discipline.

## Linked artifacts

- `docs/features/blend1.md` or `docs/features/retrieval.md` — the lexical route, the RRF fusion, the code outlines, the `hybrid_search` tool, and the agent's retrieval discipline.
- `TECHNICAL-DOCUMENTATION.md` section "Module reference" — `lexical.rs`, `fusion.rs`, and `outline.rs`.
- `docs/architecture/deep-dive-2026-11-08-zvec-and-zvec-grep.md` — the design record (already landed).

## Risks

- **The tree-sitter C build.** The grammars compile C, so the build gains a C toolchain step. Hard rule 12 is met (the grammars ship and are built), but if the owner's build environment cannot compile them, the named fallback is a structural outline from fleet's own brace and stem scan — a capability reduction, not a blocker.
- Reading the lane's `vectors.bin` in Rust couples the core to the lane's on-disk format; if that proves fragile, the fusion moves into the lane and the Rust tool calls it — a named alternative, not a silent one.
- BM25's parameters (k1, b) need a fixture corpus to tune against; the plan pins a fixture, not the owner's corpus.
- The `hybrid_search` tool is another surface beside grep, recall, the lane search, and `tool_search`; its description must route clearly or the model will not know when to call it.
- The outline extraction must skip generated and vendored trees (`target/`, `node_modules/`, `.venv/`) or the index fills with noise; the glob's existing skip list is the precedent.

## Dependencies

- `docs/architecture/deep-dive-2026-11-08-zvec-and-zvec-grep.md` — the analysis this plan absorbs.
- `services/blend1/src/blend1/corpus.py` — the corpus manifest's shape (key, sha256, text) the lexical index reads.

## Notes

- zvec-grep's `RRF_K = 60` is the standard constant; fleet's fusion uses the same.
- The lexical terms come from fleet's own stemmer (`src/memory.rs`), not a model call — the retrieval stays local and free.
- The lane's store is already hash-gated; the lexical index rides the same invalidation.
- The outline caps mirror zvec-grep's (24 calls, 32 members, 180 chars/line, 3,600 chars/outline) so a structural summary stays a summary.
