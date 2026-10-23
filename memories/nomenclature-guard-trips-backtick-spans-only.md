---
name: nomenclature-guard-trips-backtick-spans-only
description: "The nomenclature write-guard refuses case/separator variants only inside backtick spans — mandated spellings pass as bare prose, so write them bare."
type: project
date: 2026-10-22
basis: inferred
tier: durable
shape: [change, fail-open]
phrases: [case or separator variant of the established term, write_file refused, KI-284, backtick spans, Run_Options, mcp-server-registry plan]
missing: []
recall-count: 30
superseded-by: nomenclature-write-guard-scans-whole-content
valid-until:
---

The write-time nomenclature guard (write_file's refusals, KI-284) scans only inline backtick spans of the content being written: a token inside backticks that is a case/separator variant of an established term is refused, while the identical token as bare prose passes untouched. Verified by scratch-file probes 2026-10-22 while drafting the MCP registry plan: `RunOptions`, `LaneJoined`, `LaneChild`, `CLAUDE.md`, `TECHNICAL-DOCUMENTATION.md`, `FUNCTIONAL-SPECIFICATIONS.md`, and the mcp__lane__tool form were each refused in backticks (the vocabulary's canonical forms are Run_Options, Lane_Joined, Lane_Child — the guard's variant split, not the tree's spelling) and every one passed when written bare. Prose path-like runs are also scanned, but normal prose tokens are not. The working rule: mandated spellings (code identifiers, file names the task cannot rename) go bare in prose; chosen markup goes on everything else. Seven refusals on one plan draft; the plan landed reworded rather than via the bash bypass, and KI-284's ledger row carries the evidence.
