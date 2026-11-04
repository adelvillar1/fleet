---
name: rust-unicode-escapes-break-in-python-heredocs
description: "patching Rust source through a python heredoc — a Rust `\u{XXXX}` escape inside a double-quoted python string is read as python's own (invalid) \uXXXX escape and the patch dies with a unicodeescape SyntaxError; single-quote the python string"
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

When patching Rust source via `python3 <<'PYEOF' … PYEOF` heredocs, Rust's
braced unicode escape (the literal characters backslash, u, brace, hex, brace
— as in `\u{2713}` for a check mark) inside a **double-quoted** python string
is interpreted by python as its own `\uXXXX` escape, which the braced form is
not, and the script dies with `SyntaxError: (unicode error) 'unicodeescape'
codec can't decode bytes`. It cost several turns in one fleet session
(agent.rs's compaction progress line, tui/ui.rs's status bar, and the
system-marker card).

**Why:** python processes `\u` inside double-quoted strings as an escape
introducer; the Rust source needs the literal backslash to survive into the
patched file. The failure is loud — a SyntaxError before any write — so it
never corrupts the target file; the cost is the lost turn, not the code.

**How to apply:** wrap the Rust text in **single-quoted** python strings, which
treat the backslash literally, or double the backslash in a double-quoted
string. Match the literal characters as they exist in the file: the pattern
must contain the same backslash-u-brace sequence the source holds.
