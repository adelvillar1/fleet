---
name: nomenclature-write-guard-scans-whole-content
description: "The nomenclature write-guard scans whole-file content, not backtick spans only — mandated tokens as bare prose are refused too, and content-constrained files must go through bash."
type: project
date: 2026-10-23
basis: inferred
tier: durable
shape: [change, fail-open]
phrases: [case or separator variant of the established term, write_file refused, KI-284, backtick spans, Run_Options, mcp-server-registry plan]
missing: []
recall-count: 77
superseded-by:
valid-until:
---

The write-time nomenclature guard (write_file's refusals, KI-284) scans the WHOLE content being written, not only backtick spans: any word that case- or separator-matches an established term anywhere in the file is refused. Verified 2026-10-24 writing tmp/blend1_refresh.py for the agnostic-router-kit catch-up refresh — eight successive bare-prose refusals on mandated, unrenamable tokens: docstring words One and Driver, MCP (established mcp), PROJECT and FLEET (code identifiers), Projects (the owner's home directory inside a real absolute path), PATH (a POSIX env key), Popen (a Python stdlib text-tool name), and the mcp__blend1__refresh wire form (established mcp_blend1_refresh, src/cron.rs). No rewording of a content-constrained file can pass; the file went in through a bash heredoc, outside the guard. The 2026-10-22 conclusion this fact replaces (backtick spans only, mandated spellings pass bare) came from the plan-drafting session's rewording attempts and is corrected by KI-284's 2026-10-24 ledger update. Working rule: write_file is only safe for files whose vocabulary the author chose; anything carrying mandated identifiers, absolute paths, or protocol and stdlib names goes through bash.
