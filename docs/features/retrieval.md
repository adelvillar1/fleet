# The hybrid retrieval

Fleet searches in two ways: **exact** (grep, the line-oriented tool) and
**hybrid** (`hybrid_search`, the fused lexical-and-vector tool). The hybrid
layer is three routes over a workspace — the corpus's BM25, the source
outlines' BM25, and the corpus's vectors — fused by Reciprocal Rank Fusion.
The design record is
[docs/architecture/deep-dive-2026-11-08-zvec-and-zvec-grep.md](../architecture/deep-dive-2026-11-08-zvec-and-zvec-grep.md).

Before this layer, the corpus search was vector-only and the code search had
no structure: grep found lines, `repo_map` pattern-matched definitions, and
the memory ranker matched keywords by set coverage. There was no ranked lexical
route and no rank fusion anywhere.

## The lexical route — BM25

`src/lexical.rs` is the standard BM25: term frequency, inverse document
frequency, and length normalization, over an in-memory document set. The
terms come from `src/memory.rs`'s own stemmer, so the lexical route and the
memory ranker speak the same word forms. A rare term outranks a common one;
a document that repeats a term outranks one that mentions it once; a long
document does not win on bulk. The parameters are the standard `k1 = 1.2`,
`b = 0.75`.

The lexical index is cached beside the lane's manifest as
`.lexical-index.json`, keyed by a hash of the entries' content hashes — the
same invalidation the lane's own hash-gated refresh uses. A changed entry
changes the hash and rebuilds the index.

## The fusion — Reciprocal Rank Fusion

`src/fusion.rs` fuses the routes by rank alone: an id's fused score is the sum
over the routes that contain it of `1 / (RRF_K + rank)`, with `RRF_K = 60`.
No weights are tuned, and a route's score scale cannot dominate another's —
the fusion is the same one zvec-grep uses.

The routes: the corpus's BM25, the source outlines' BM25 (when the store is
opened with them), and the corpus's vectors. A file the two lexical routes both
name outranks one only the corpus mentions.

## The code outlines

`src/outline.rs` parses a workspace's source with **tree-sitter** and emits one
structural outline per file: its symbols with their signatures, members and
calls, capped. The outline's own text is what the lexical index stores, so a
query naming a symbol's signature retrieves the file's structure rather than a
line that happens to mention it.

The caps (from zvec-grep): 24 calls, 32 members, 180 characters per line, 3,600
characters per outline. The walk skips the generated and vendored trees
(`target/`, `node_modules/`, `.git/`, `.venv/`, `__pycache__/`, `dist/`, and
dot-directories) and caps at 5,000 files, sorted by path.

The grammar set is bounded to the languages the workspace's own code and docs
are written in: Rust, Python, Markdown, TypeScript/TSX, JSON, and Bash. TOML
is deliberately absent — the only TOML grammar targets the legacy tree-sitter
API and would force a second, incompatible runtime. The grammars ship in their
crates and are compiled by the build, which is hard rule 12's "built by the
install": nothing is supplied by a checkout. If the owner's build environment
cannot compile them, the named fallback is a structural outline from fleet's
own brace and stem scan — a capability reduction, not a blocker.

## The vector route

The lane's store is a pair of files: `manifest.json` (each entry's key, content
hash and text) and `vectors.bin` (the packed float32 vectors, parallel to the
entries). `src/hybrid.rs` reads both directly — **no per-search lane call** —
cosine-searches the vectors, and fuses. The query's vector comes from fleet's
own embedder (the recall path's probed loopback call), not the lane, so a dead
lane costs the vector route and the lexical routes still answer — the same
named degradation the memory ranker has.

## The tool

`hybrid_search` takes a workspace root, a query, and an optional limit
(default 5, maximum 20). It resolves the project key the way the lane does
(the repository's top-level basename), opens the cached corpus with the
workspace's outlines, and returns the fused, source-linked passages.

The tool's description carries the routing discipline, and the `search` and
`recall` descriptions name it:

- Use `hybrid_search` when the wording or the location is unknown —
  architecture, call chains, dependencies, design rationale, or a symbol you
  cannot name exactly.
- Use `grep` when exact lookup alone is sufficient — one definition, literal,
  filename, key, error message, regex.
- Treat sufficient returned content as read evidence; stop once it answers,
  and do not re-run a similar search to reconfirm.

## What this does not touch

The memory ranker's six-channel keyword term is deliberately unchanged: its set
coverage is part of the tuned weighting (direction outranks proximity), and
changing it would move the weights. The BM25 route is a new peer for the
corpus and the code, not a replacement for the fact ranker.
