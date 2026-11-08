# Deep dive 2026-11-08: fleet's search and recall, read against alibaba/zvec and zvec-ai/zvec-grep

Research only — no plan, no code. The question: what does fleet's search and
recall machinery look like, what do zvec and zvec-grep look like, and which of
their ideas belong in fleet.

The two external projects are a pair: **zvec** is an in-process vector
database (the engine), and **zvec-grep** is a search CLI that unifies ripgrep,
BM25, and vector search over zvec (the interface). Fleet has the engine half
in two places and no unified interface.

---

## Part 1 — Fleet's search and recall machinery

### Two stacks that share an embedding model and nothing else

**1. The Rust recall stack** (`src/memory.rs`, `src/recall.rs`,
`src/session_recall.rs`, `src/recall_graph.rs`) ranks the git-tracked fact
corpus (`memories/*.md`, one file per fact) with a **six-channel weighted
sum**:

```
total = 0.15·vector + 0.15·keyword + 0.30·trajectory
      + 0.25·history  + 0.10·freshness + 0.05·provenance

vector     = max(0, cosine(query_embedding, fact_embedding))
keyword    = |stems(query) ∩ stems(topic_text)| / |stems(query)|
trajectory = |stems(query) ∩ stems(phrases)|    / |stems(query)|
history    = ln(1 + min(recall_count, 128)) / ln(129)
freshness  = 0.5 ^ (age_days / 90)
provenance = 1.0 confirmed-by | 0.75 validated | 0.5 inferred
```

The vector channel is a **probed** loopback HTTP call to llama-server at
`127.0.0.1:8901` serving EmbeddingGemma-2 at 768 dims; its absence degrades to
a named **keyword-only** mode that still ranks the other five channels. Vectors
cache in a gitignored JSON sidecar (`memories/.recall-index.json`), keyed by
content digest and embedder identity. The recall tool is read-only and records
nothing; the feedback loop fires only after the model's first successful
response.

**2. The Python lane stack** (`services/sem1`, `services/blend1`) is a separate,
richer vector store: packed-float32 `vectors.bin` + `manifest.json` under
`~/.local/share/{sem1,blend1}/vectors/`. The blend1 corpus is docs-derived
(`docs/**/*.md` + root `*.md`), split on `## ` headings, hash-gated so the
steady state is file reads and no embedding. Its search is **vector-only**
top-k cosine — no lexical fusion.

The model's native `search` tool is an **alias for grep** (`src/tools.rs:2750`);
`grep`/`glob` are capped, counted wrappers over `rg`/`grep`. `tool_search` is a
deferred, sem1-ranked tool-card selector.

### What fleet does not have

Confirmed by code search: **no ANN** (brute-force O(n·d) cosine everywhere),
**no quantization** (raw float32 end to end), **no BM25/TF-IDF/IDF** anywhere,
**no hybrid lexical+vector fusion in the corpus search** (blend1 is
vector-only; the Rust ranker is a weighted sum, not RRF), **no cross-encoder or
second-model reranking**, **no metadata filtering**, **no incremental index
updates** (every rebuild is wholesale), **no batch search**, and **no session
search in the `recall` tool**.

The **Matryoshka pre-filter** (128-dim slice, top-24 full-width re-rank) is
implemented, measured, and **left OFF**: at 625 facts the recall agreement is
perfect (1.000) but it is never cheaper, because it prunes no work from the
scoring loop (KI-459). The honest result is recorded and the setting stays off.

### The distinctive decisions

1. **Direction outranks proximity, structurally.** The proximity pair (vector +
   keyword) is 0.30 of the composite; the direction channels (trajectory +
   history + provenance) are 0.60. The shape partitions the pool but
   contributes no additive term.
2. **The vector path is probed, never assumed, and its absence is a named peer
   implementation** (`KeywordOnly`), named by `doctor`.
3. **The index is a gitignored JSON sidecar, not a database** — treated as
   `__pycache__`: absent or malformed reads as empty and costs a recompute,
   never an error.
4. **The corpus is docs-derived and hash-gated, and the request path is
   compiler-closed against cold builds** (`corpus()` takes no embedder; a
   source-inspection test pins it).
5. **Recall is read-only and records nothing** — a "hit" provably means the
   model saw the fact.

---

## Part 2 — zvec

An **in-process vector database**, battle-tested at Alibaba, Apache-2.0, C++
core with SDKs in Python, Node, Go, Rust, Java, and Dart.

- **Index types**: HNSW, IVF, DiskANN, HNSW+RaBitQ, IVF+RaBitQ — from pure
  in-memory graphs to disk-scale (DiskANN) and quantized (RaBitQ).
- **Quantization**: product quantization (PQ, 4 or 8 bits) and scalar
  quantization (SQ, including INT4/INT8 kernels), with uniform quantizers for
  faster search.
- **Vectors**: dense and sparse, multi-vector queries, several metrics (L2, IP,
  cosine).
- **Full-text search**: a native keyword route with Jieba tokenization for
  Chinese — no separate dictionary files.
- **Hybrid search**: fuse vector similarity, full-text search, and structured
  filters in one query.
- **Durability**: a write-ahead log; multiple processes can read one
  collection, writes are single-process exclusive.

The relevance to fleet is the **engine half**: ANN, quantization, FTS, and
hybrid fusion, all in-process with a Rust SDK.

---

## Part 3 — zvec-grep

A local-first search CLI (`zg`) that **unifies ripgrep, BM25, and vector search
behind one interface**, for humans and AI agents.

- **One query, several routes.** `query` creates a hybrid FTS+vector group;
  `queries` creates several; `fts` and `vector` add supplemental lexical-only or
  semantic-only routes. Without `fuse`, each group keeps its ranked results;
  with `fuse: true`, every group collapses into one ranked plan.
- **RRF fusion.** The candidates from the lexical and vector routes are fused
  with Reciprocal Rank Fusion at `RRF_K = 60` — a parameter-light fusion that
  needs no tuned weights.
- **Code-aware extraction.** Source is parsed with tree-sitter into structural
  outlines — symbols, signatures, members, calls — with caps (24 calls, 32
  members, 180 chars/line, 3,600 chars/outline). Prose is chunked into sections;
  code is retrieved as structure, not as lines.
- **The agent protocol.** The MCP tool ships a complete retrieval discipline:
  use semantic search when wording or location is unknown, native grep when
  exact lookup suffices; treat mixed tasks as semantic-with-anchors; **treat
  sufficient returned content as already-read evidence**; **stop searching once
  the evidence is sufficient**; do not launch a sub-agent solely to locate
  material. Results are bounded, source-linked snippets (`preview: "full"` for
  all of an item).
- **The consent model.** Remote embeddings require explicit consent, with three
  decisions: once, workspace, or FTS-only (no remote embedding at all). Local
  embeddings are the default.
- **The daemon.** One reusable index, owned by a loopback daemon
  (`127.0.0.1:7999`), exposed over MCP and HTTP. Index once, search from
  anywhere. The index lives in `.zvec-grep/` under the project root.
- **The benchmarks.** Paired A/B runs measuring answer quality, input tokens,
  tool calls, and wall time — the claim is fewer tool calls and fewer tokens,
  not just better answers.

---

## Part 4 — The comparison

### The headline difference

zvec-grep's core idea is **one interface fusing lexical and vector routes with
a disciplined agent protocol**. Fleet has the engine half in two disconnected
stacks and no unified interface: the agent's default `search` is grep, the
semantic corpus search is a lane tool the agent must know to call, and the fact
recall is a separate weighted-sum ranker.

### Where fleet is weaker

1. **No lexical ranking.** Fleet has no BM25, no IDF, no TF-IDF. Its keyword
   channel is set coverage (`|stems(query) ∩ stems(topic)| / |stems(query)|`)
   — a crude match that cannot rank by term rarity or term frequency. zvec-grep
   and zvec both have a real lexical route.
2. **No fusion in the corpus search.** blend1's corpus search is vector-only
   top-k. Fleet's only hybrid is the Rust ranker's weighted sum over six
   channels — a deliberately tuned design, not a rank fusion.
3. **No code-aware chunking.** Fleet's code search is grep (lines) plus
   `repo_map` (files ranked by reference count, pattern-matched definitions, no
   parser). zvec-grep retrieves code as tree-sitter structure — a symbol's
   signature, its members, its calls.
4. **No agent retrieval discipline.** Fleet's tool descriptions do not carry
   the "stop when sufficient / treat returned content as read / do not
   re-confirm" rules. The cost is repeated greps and re-reads.
5. **No first-class hybrid search tool.** The agent must route between grep and
   a lane tool; there is no single tool that answers "find the thing this
   describes" with fused lexical+semantic evidence.
6. **No ANN or quantization.** Brute-force cosine is fine at fleet's scale (651
   facts, 2,574 corpus entries, a session index), and the measured evidence
   (KI-459) says the pre-filter isn't cheaper. This is a scale gap, not a
   present defect.

### Where fleet is stronger

1. **The probed vector path with a named degradation.** zvec-grep assumes a
   local model; fleet probes the embedding endpoint with a real round trip and
   degrades to a named `KeywordOnly` mode that still ranks five channels.
2. **Direction outranks proximity.** The six-channel weighting is a deliberate
   inversion of a conventional retriever; zvec-grep's RRF is a generic fusion.
3. **The gitignored JSON sidecar, not a database.** Fleet's index is a
   regenerable file; zvec is a database with a WAL and a daemon.
4. **The compiler-closed cold-build split.** The request path cannot build the
   corpus; zvec-grep's daemon can rebuild in the background on a missing index.
5. **Self-containment.** Fleet's core is one Rust binary; zvec-grep is a
   Node/Rust toolchain with a daemon and a local LLM runtime (llama-cpp).
6. **A read-only recall.** Fleet's recall tool writes nothing; zvec-grep's
   index is mutable state.

### The honest tension

zvec-grep is a document/code search tool; fleet's memory recall is a **fact
ranker** with a deliberately different philosophy. They overlap on the corpus
and the code search, not on the memory ranker. So the comparison applies to
fleet's corpus and code paths — and to the agent's search discipline — not to
the six-channel recall, which is a different instrument.

---

## Part 5 — The pieces worth absorbing

Ranked by value and fit. None require zvec's toolchain.

1. **A BM25/IDF lexical route.** Fleet has no ranked lexical matching. A BM25
   implementation over the corpus (and the fact index) would give the keyword
   channel real ranking power — term rarity, term frequency, length
   normalization — instead of set coverage. Pure Rust, no dependency.
2. **RRF fusion for the corpus search.** Replace blend1's vector-only top-k
   with a lexical route fused by Reciprocal Rank Fusion (K=60). RRF is
   parameter-light and standard; it removes the need to tune weights for a
   two-route fusion.
3. **The agent retrieval protocol.** A tool-description discipline: use
   semantic search when wording or location is unknown, native grep when exact
   lookup suffices; treat sufficient returned content as read; stop when the
   evidence is sufficient. Cheap, and it directly targets fleet's repeated-grep
   cost.
4. **Code-aware chunking.** Tree-sitter outlines (symbols, signatures, members,
   calls) as the retrievable unit for source, replacing the line-oriented grep
   and the pattern-matched `repo_map` for structural questions.
5. **A first-class hybrid search tool for the agent.** One tool that answers
   "find the thing this describes" with fused lexical+semantic evidence and
   source links, instead of grep-by-default plus a lane tool.
6. **ANN and quantization — do not build now.** Fleet's scale (hundreds to low
   thousands of vectors) does not justify it, and the measured evidence says
   the pre-filter isn't cheaper. Revisit only if the corpus or the session
   index grows by orders of magnitude.
7. **The consent model — future-proofing.** Fleet's embeddings are local by hard
   rule 12, so remote consent doesn't apply today. The *pattern* (an explicit,
   named consent before any data leaves the machine) is worth keeping in mind
   if a hosted embedding option ever appears.

### Leave behind

- The daemon and the Node/llama-cpp toolchain (hard rule 12).
- The database-with-WAL model (fleet's index is a regenerable sidecar).
- The local LLM runtime for embeddings (fleet's lanes already own that).

---

## Part 6 — What fleet already does better

Fleet's probed vector path, its direction-outranks-proximity weighting, its
gitignored regenerable index, its compiler-closed cold-build split, its
read-only recall, and its hash-gated corpus refresh are all stronger than
zvec-grep's equivalents. zvec-grep's advantages are the **lexical route, the
rank fusion, the code-aware chunking, and the agent protocol** — the
*retrieval interface*, not the *storage engine*.
