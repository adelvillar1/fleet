---
name: harness-tool-surface-research-2026-10
description: "2026-10-23 evidence base for designing a tool surface — the numbers that decided fleet's build (edit anchoring, windowed reads, search caps, catalog size, tool descriptions, silent errors, context elision) plus what each frontier harness ships, with sources"
type: reference
date: 2026-11-04
basis: inferred
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

Five parallel sweeps (frontier harness docs, open-source harnesses + MCP
ecosystem, tool-use papers + benchmarks) plus a fleet code audit, run
2026-10-23 as the basis for [[fleet-tool-surface-analysis-2026-10-23]]. Keep
this for the *numbers and mechanisms*; the fleet decisions live in that file.

**Numbers that should drive any harness's tool design**

- SWE-agent ACI ablations (SWE-bench Lite, GPT-4 Turbo, base 18.0): no edit
  tool 10.3; edit without a lint gate 15.0; whole-file view 12.7; 30-line
  window 14.3; iterative search 12.0 vs capped search 18.0 (worse than no
  search at all, 15.7); full history 15.0; raw shell baseline 11.0.
- Edit anchoring (Look Before You Leap, arXiv 2609.11957): line-number anchors
  corrupted 99.1% of files under a one-line shift; content-anchored
  search/replace failed cleanly; anchor+verify had 1 silent misapplication in
  8,320 trials (0.01%). Edit success 90.5% first try → 57.2% after one
  failure; cascading failed edits behind 23.4% of unresolved runs.
- Tool retrieval beats lexical by a wide margin: an embedding retriever
  (ToolLLM) scored NDCG@5 84.9 vs 17.0 BM25; Gorilla's BM25 dropped AST
  accuracy to 17.03% vs 91.26% with oracle docs.
- Catalog size: selection degrades past 30–50 tools; 5 MCP servers ≈ 55k
  tokens of definitions; deferred/tool-search cut that >85% (Anthropic).
  OSWorld-MCP: unfiltered 158 tools 20.5% → 15.5%.
- Truncation: returning a retrieved section instead of inlining cost 7.9–11.3
  pts; naming the authoritative output file recovered 6.3–7.9 (arXiv
  2610.26836). Claude Code caps reads at 25k tokens, bash at 30k chars with a
  spill path.
- Descriptions are interface: 97.1% of 856 MCP descriptions had ≥1 smell, 56%
  failed to state purpose; rewriting gained 5.85 pts median (arXiv
  2602.14878) at +67% steps.
- Silent tool errors roughly halve success vs loud ones (RITE, arXiv
  2610.27777).
- Context: rule-based elision beat LLM summarization; rolling drop of tool
  responses +1.7 pts at −27.7% cost (arXiv 2610.28577). Long-context
  degradation is real and positional (Lost in the Middle, RULER, NoLiMa).
- Delegation: +90% on research eval but ~15× tokens; explicitly poor for
  tightly coupled coding.
- Bash-only is viable: mini-swe-agent claims >74% SWE-bench Verified with
  ~100 lines; the 2026 harness study found predefined tools help bash-weak
  models and bash-strong models do as well, cheaper, with bash alone.

**Mechanisms worth borrowing, per harness**

- Claude Code: `Edit` uniqueness rule, read-before-write, line-numbered reads
  with a `PARTIAL view` continuation, bash timeout + background + spill,
  clearing old tool outputs before summarizing, hooks (`PreToolUse`,
  `PostToolBatch`).
- Codex: `exec_command` with `yield_time_ms`/`max_output_tokens`,
  `write_stdin` for PTY interactivity, freeform `apply_patch` (V4A, no line
  numbers), `update_plan`, `tool_search` over deferred tools.
- Gemini CLI: `read_file(start_line,end_line)` mandated, `grep_search`
  "preferred over shell grep", `write_todos`, per-model schema sets.
- Cline/Roo: `editor` XOR `apply_patch` chosen per model family;
  read-before-edit; read_file indentation mode; one-tool-per-turn XML fallback.
- OpenHands: `undo_edit`, todo status aliases, per-model presets.
- Aider: tree-sitter repo map → dependency graph → PageRank in a token
  budget; edit format matched to model family.
- Factory: `ApplyPatch` auto-added beside `Edit` for OpenAI models.

**MCP, as of 2026-10-23**: the spec is 2026-07-28 (stateless, `server/discover`
instead of `initialize`, `resultType`/MRTR, no SSE resumability, tasks as an
extension). Fleet speaks `2024-11-05`; as of the 2026-10-23 build it now
handles pagination, `structuredContent`, image content, timeout cancellation
and `annotations` (as approval hints only — the spec says never trust them for
policy), but still has no resources, prompts or progress. Don't rebuild
git/github/browser/docs servers.

**Measured on this machine (2026-10-23), superseding intuition:** live
embeddinggemma-2 cosines are soft — real tool matches 0.69–0.80, no-match
queries 0.60–0.67, near-duplicate tools 0.01 apart — so a retrieval floor is a
measured constant, not a guess (0.68 here). Synthetic corpora with one
distinctive token per card score 0.83–0.85 and flatter any retriever. See
[[fleet-tool-surface-analysis-2026-10-23]].

**How to apply:** cite these when arguing a tool decision; do not cite a tool
*count* as a goal. Evidence strength: ablations and multi-study findings are
strong; deferred loading and code-execution savings are vendor evals with
large effects; todo-tool gains have no clean RCT.
