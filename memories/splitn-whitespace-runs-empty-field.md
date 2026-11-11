---
name: splitn-whitespace-runs-empty-field
description: "feedback 2026-11-06 — `splitn(3, char::is_whitespace)` does NOT collapse whitespace runs: ps's space-padded columns (`74116     1 /Users/...`) hand the second slot an empty string, the ppid parse fails, and every line silently continues — the sweep stopped nothing; the fix is split_once twice with trim_start, and the live orphan is what caught it"
type: feedback
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

The session-close hygiene's stale-process sweep (`stale_process_sweep` in
`src/main.rs`) parsed `ps -axo pid=,ppid=,command=` with
`line.trim().splitn(3, char::is_whitespace)` — and the first live run
stopped **zero** processes while a spawned orphan (pid 74116, ppid 1, the
fleet venv's python) was alive and verified.

**The cause:** ps's columns are space-padded to fixed widths, so the line
reads `74116     1 /Users/...` — the splitn found the first space (after
the pid), the second field was the EMPTY string between the first and
second spaces, and the third field started at the third space. The ppid
parse got `""` → `parse::<u32>()` failed → `continue` — every line with
multi-space padding died at the parse, silently.

**The proof was the live orphan**: a python mimic of the parse (using
`split(maxsplit=2)`, which collapses runs) showed `pid=74116 ppid=1
mine=True orphan=True`, while the Rust sweep counted 0. The fix:
`split_once(char::is_whitespace)` twice with `trim_start()` between —
which collapses the runs the same way the mimic did.

**How to apply.** `splitn(n, char::is_whitespace)` splits on EACH
whitespace character, not on runs — any fixed-width-column output (ps,
ls -l, netstat) has multi-space padding that breaks it. Use
`split_once` + `trim_start()` or `split_whitespace()` with a join for
the rest. And the defect was invisible to every unit test because the
parse worked on single-space input — only the live orphan caught it.
Same family as [[a-brace-matcher-corrupts-at-first-unmatched-brace]]
(the matcher that silently corrupted) and
[[a-load-sensitive-test-is-a-defect-in-the-code]] (only the live
condition reproduces it).
