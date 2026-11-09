---
name: write-guard-exempts-quoted-spans
description: "KI-484: the write guard exempts multi-word backtick spans in markdown writes from the case-variant refusal (quoted tool output is one quoted thing, not per-word coinages); single-word spans, .rs strings, and fenced code still police"
type: project
date: 2026-11-09
basis: inferred
tier: durable
shape: [change]
phrases: [KI-484 quoted shape, markdown_quoted_words, multi-word backtick span exemption, single-word span still policed, Signature and comment signature verified refused, bash heredoc for mandated content]
missing: []
premises: []
recall-count: 0
superseded-by:
valid-until:
source:
---

KI-484 (2026-11-09, closed - fixed): the write-time guard's refusal ladder had no rung for the QUOTED shape — a markdown write quoting a tool's verbatim output in a multi-word inline backtick span was refused word-by-word (`Signature and comment signature verified` died on `Signature` vs the established `signature`), because bare prose is exempt only OUTSIDE backticks and the quoted-literal rule covers absolute paths and composed-path strings only. The fix: markdown_quoted_words(content) — multi-word inline spans on non-fenced lines, split by the channel's own words() — exempts their words from the per-write case-variant ONLY; the derivation is untouched so spans still carry identifier evidence for the spelling convention. Boundary pins: single-word spans (RunOptions shape) still police; the same span in a .rs string still polices; fenced blocks still police. The span finder/splitter are crate-visible and SHARED (nomenclature::backtick_spans, nomenclature::words) so the guard's reader and the channel reader cannot disagree. Deployment note: the guard is a per-process rule — the fix protects faces from their next cut onward (the 0.6.2 running binary predates it); a content-constrained write with a verbatim span goes through bash (the KI-284 procedure) until then. Red-proof: exemption reverted → the guard pin fails naming the same word the 0.6.2 recap was refused for.
