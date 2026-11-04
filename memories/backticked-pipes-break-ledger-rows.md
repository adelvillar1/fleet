---
name: backticked-pipes-break-ledger-rows
description: "feedback 2026-10-30 — a backticked pipe-separated vocabulary inside a KI row's cell breaks the checker's cell count (KI-377, written and broken the same day); run scripts/ki_ledger.py before every recap"
type: feedback
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

A KI row's cell carrying a backticked pipe-separated list — `` `read|search|edit|…` `` — puts unescaped pipes inside a markdown table row, so `scripts/ki_ledger.py` reads 16 cells against the header's 6 and the open count it reports is wrong. Hit on KI-377, written and broken the same day by my own hand, caught by the recap skill's pre-flight.

**Why:** markdown tables split cells on every pipe, backticks or not; the checker's cell-count check is what keeps the open count honest, and the 2026-10-26 pass showed a shifted row can hide genuinely open ones.

**How to apply:** run `python3 scripts/ki_ledger.py docs/known-issues.md` and require "all cell counts agree" before any recap or close-of-record — including on rows you wrote yourself this session. In-row vocabularies are comma-separated prose, never pipe-separated lists. Related: [[ki-ledger-second-pass-method]], [[curator-arc-recap-2026-10-30]].
