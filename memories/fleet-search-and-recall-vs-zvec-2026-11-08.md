---
name: fleet-search-and-recall-vs-zvec-2026-11-08
description: "project 2026-11-08 — fleet's search and recall machinery read against alibaba/zvec and zvec-ai/zvec-grep; landed as docs/architecture/deep-dive-2026-11-08-zvec-and-zvec-grep.md; the headline difference is one interface fusing lexical and vector routes with a disciplined agent protocol — fleet has the engine half in two disconnected stacks and no unified interface; the pieces worth absorbing are a BM25/IDF lexical route, RRF fusion, the agent retrieval protocol, code-aware chunking, and a first-class hybrid …"
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

The owner asked 2026-11-08 for the comparison pattern again, on two repos:
`github.com/alibaba/zvec` (an in-process vector database) and
`github.com/zvec-ai/zvec-grep` (a local-first search CLI that unifies
ripgrep, BM25, and vector search). Landed as
`docs/architecture/deep-dive-2026-11-08-zvec-and-zvec-grep.md`. The owner
then said "create a plan to absorb your recommendations" — the plan
`docs/plans/2026-11-08-hybrid-retrieval.md` is DRAFTED and gated (arch PASS;
plan-gate flags C8/C9 plus Dependencies SCOPE? proposed waived, the same
systematic pattern as the last two plans), awaiting the owner's approval. The
owner then said **"let's include the code chunking in the plan"** — so the plan
was REVISED to absorb all five pieces (the tree-sitter outlines are C4/C5, with
the hard-rule-12 dependency question settled explicitly), and re-gated.

**Fleet's model, synthesized.** Two stacks sharing one embedding model and
nothing else. (1) The Rust recall stack ranks the fact corpus with a
six-channel weighted sum — vector 0.15, keyword 0.15, trajectory 0.30,
history 0.25, freshness 0.10, provenance 0.05 — over **brute-force cosine**
(651 facts at 768 dims, a gitignored JSON sidecar). (2) The Python lanes
(sem1/blend1) hold a separate packed-float32 store whose corpus search is
**vector-only** top-k. The agent's native `search` tool is an **alias for
grep** (`src/tools.rs:2750`); the semantic corpus search is a lane tool the
agent must know to call. Confirmed absent: no ANN, no quantization, no
BM25/TF-IDF/IDF anywhere, no hybrid lexical+vector fusion in the corpus,
no cross-encoder reranking, no metadata filtering, no incremental updates,
no batch search, no session search in the `recall` tool. The Matryoshka
pre-filter was built, measured, and **left OFF** (KI-459: perfect agreement,
never cheaper).

**zvec.** Alibaba's in-process vector database: ANN indexes (HNSW, IVF,
DiskANN, RaBitQ), PQ/SQ quantization, dense+sparse vectors, native
full-text search with Jieba for Chinese, hybrid search (vector + FTS +
filters), a WAL, and a Rust SDK.

**zvec-grep.** One query, several routes: `query` makes a hybrid FTS+vector
group, `fuse: true` collapses them, and the fusion is **Reciprocal Rank
Fusion at K=60** — parameter-light, no tuned weights. Code is parsed with
tree-sitter into structural outlines (symbols, signatures, members, calls)
rather than indexed as lines. Its MCP tool ships a full agent protocol:
semantic when wording/location is unknown, grep when exact lookup suffices,
**treat sufficient returned content as already-read evidence**, **stop
searching once the evidence is sufficient**. Plus a consent model (remote
embeddings need explicit approval; local is the default) and a loopback
daemon owning one reusable index.

**The comparison.** The headline difference: zvec-grep's core idea is one
interface fusing lexical and vector routes with a disciplined agent
protocol. Fleet has the engine half in two disconnected stacks and no
unified interface. Fleet is weaker on the lexical route (its keyword
channel is set coverage, not a ranked BM25), the fusion (vector-only
corpus), the code chunking (grep + a pattern-matched `repo_map`, no
parser), and the agent discipline (no "stop when sufficient" rule). Fleet
is stronger on the probed vector path with a named degradation, the
direction-outranks-proximity weighting, the regenerable gitignored index,
the compiler-closed cold-build split, and self-containment (one binary vs
a Node/Rust toolchain with a daemon and a local LLM runtime).

**The pieces worth absorbing, ranked:** (1) a **BM25/IDF lexical route**
(fleet has no ranked lexical matching — the biggest algorithmic gap);
(2) **RRF fusion** for the corpus search (replacing vector-only top-k);
(3) the **agent retrieval protocol** as a tool-description discipline;
(4) **code-aware chunking** with tree-sitter outlines; (5) a **first-class
hybrid search tool** for the agent. Not now: ANN/quantization (fleet's
scale does not justify it, and the measurement says the pre-filter is not
cheaper) and the consent model (fleet's embeddings are local by hard rule
12).

**The honest tension.** zvec-grep is a document/code search tool; fleet's
memory recall is a fact ranker with a deliberately different philosophy.
The comparison applies to fleet's corpus and code paths — and to the
agent's search discipline — not to the six-channel ranker.

**The absorption plan's key wiring insight.** The lane's
`manifest.json` already carries each corpus entry's `key`, `sha256`, AND
`text`, and `vectors.bin` carries the vectors — so the Rust side can read
the cached store directly and fuse lexical + vector with **no per-search
lane call**. The lane's hash-gated refresh keeps the store current. The
plan's FIVE pieces (after the owner's code-chunking steer):
`src/lexical.rs` (BM25/IDF), `src/fusion.rs` (RRF at K=60),
`src/outline.rs` (tree-sitter structural outlines), the agent's retrieval
protocol in the `search`/`recall` descriptions, and a `hybrid_search` tool.
It deliberately does NOT rewrite the Rust recall's keyword term (its set
coverage is part of the tuned weighting).

**The code-chunking dependency question, settled explicitly.** The grammar
set is bounded (Rust, Python, Markdown, TypeScript/TSX, JSON, Bash) —
**TOML is deliberately absent**: the only TOML grammar
(`tree-sitter-toml` 0.20.0) targets the legacy tree-sitter API and would
force a second, incompatible runtime, so it was dropped rather than
compromising the one-runtime rule. The grammars ship in their crates and
are compiled by the build — hard rule 12's "built by the install", so
nothing is supplied by a checkout. The named risk: the build gains a C
toolchain step, and if the owner's environment cannot compile them, the
fallback is a structural outline from fleet's own brace and stem scan (a
capability reduction, not a blocker). The outline caps mirror zvec-grep's
(24 calls, 32 members, 180 chars/line, 3,600 chars/outline), and the
extraction must skip generated/vendored trees (`target/`, `node_modules/`,
`.venv/`).

**IMPLEMENTED, COMPLETED and MERGED (2026-11-08).** The owner's goal —
"implement plan as designed using a worktree for the plan to avoid
collisions with other work" — ran on the hybrid-retrieval plan. Lane
`/Users/alejandrodelvillar/Projects/fleet-hybrid-retrieval`, branch
`hybrid-retrieval`, forked from master at `de55b62`; **merged to master at
`29daca3`** (a fast-forward, `de55b62..29daca3`), then pruned — 8.9 GiB
freed. The plan's status is `completed` with all ten criteria ticked.

What landed: `src/lexical.rs` (BM25 with k1=1.2, b=0.75 over fleet's own
stemmer, a `LexIndex` + `LexDoc` that serialize for the cache),
`src/fusion.rs` (RRF at `RRF_K = 60`, ties broken by id), `src/hybrid.rs`
(the `HybridStore` reading the lane's manifest + `vectors.bin` directly,
the lexical cache beside the manifest keyed by a hash of the entries'
content hashes, the outline walk skipping the generated/vendored trees and
capping at 5,000 files, and the `hybrid_search` tool's runner with the
project key resolved the lane's own way), `src/outline.rs` (the bounded
grammar set, the symbol/member/call extraction with the caps, the
`OutlineLanguage::from_path` gate), the `hybrid_search` tool in
`src/tools.rs` + its dispatch in `src/agent.rs`, the routing discipline in
the `search` and `recall` descriptions, `docs/features/retrieval.md`, and
the tech doc's module reference for the four new modules.

**Three implementation shapes worth carrying.**
- **The lexical cache rides the lane's own invalidation.** A hash of the
  entries' key+sha256 pairs keys `.lexical-index.json`; a changed entry
  changes the hash and rebuilds the index — the same discipline as the
  lane's hash-gated refresh, not a second staleness rule.
- **The vector route's query comes from fleet's own embedder, not the lane.**
  A dead lane costs the vector route and the lexical routes still answer —
  the same named degradation the memory ranker has, extended to the corpus.
- **The tool-surface bound's fifth crossing.** The `hybrid_search`
  declaration put the surface at 10,621 against 10,300, so
  `MAX_DEFINITION_TOKENS` moves to 10,700 with the crossing named in the
  constant's own history ([[a-declaration-budget-is-a-measurement-too]]).
  The tier test's synthetic over-budget fixture had to be enlarged too
  (40 → 80 repeats) because the old size now FITS under the new bound — a
  fixture that asserts "the core's tail is withheld" must exceed the bound
  at its current value.

See [[fleet-recall-measured-state]] (the ranker's own measured behaviour),
[[memory-substrate-deep-dive-2026-11-07]] (the corpus and the prefilter
verdict), [[fleet-deliverables-and-kami-2026-11-07]] (the same comparison
pattern one layer down), [[ui-component-creation-and-json-render-2026-11-07]]
(the same pattern above it), [[a-declaration-budget-is-a-measurement-too]].
