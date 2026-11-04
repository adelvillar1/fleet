---
name: assistant-record-field-cascade
description: "feedback 2026-10-28/30 — adding a field to a heavily-constructed log Record enum cascades to every literal (E0063) and every destructuring pattern (E0004); adding a NEW VARIANT breaks every exhaustive match arm across the crate (the compiler finds them all); and `tools::declarations(&lanes)` ALWAYS includes the 11 built-ins, so "empty lane declarations" asserts len()==11"
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

feedback 2026-10-28 — adding `persona: Option<String>` to fleet's
`Record::Assistant` cascaded to **~26 constructor literals and ~7
destructuring patterns** across the crate plus tests/integration.rs
(missing-field E0063 on every literal, non-exhaustive E0004 on every
pattern), and the mechanical fixes were the hard part.

**Why:** the log format is a vocabulary every module matches on; a
tolerant field (`#[serde(default, skip_serializing_if = "Option::is_none")]`)
keeps old LOGS readable but does nothing for old CODE — compile errors
are the cascade, and they are the safety net, not the problem.

**How to apply:**

- Expect exactly two error classes and fix by class: constructors get
  `field: None`, patterns get `..` (never `.., field: None` — that shape
  is a compile error).
- A python regex insertion across many files produced double commas
  (`vec![],,`), `..,` inside patterns, and stray lines outside braces —
  three repair rounds for one mechanical pass. A brace-counting pass is
  safer, and even then inspect what it wrote (`]],,` at a block end was
  invisible until clippy).
- The honest site count comes from the compiler's error list, never a
  grep — grep found 5, the compiler found 33.
- `cargo fmt` after the sweep and clippy `-D warnings` before
  celebrating: the mangling was syntactically legal often enough to slip
  past a build and get caught only by fmt/clippy.
- The fixture pattern to prefer for the future: a `Record::Assistant`
  constructor helper, so the next field is one line instead of 33.
- 2026-11-02 the same sweep on `api::Usage` (three `Option` fields) corrupted
  `impl Usage {`, a `self.usage.clone()` call, and the `struct Usage`
  declaration — the `Usage {` regex matches the token sequence, not the
  construct. Key the sweep on the literal's own field line, exclude
  `impl `/`struct ` prefixes and `..Default::default()` bodies, and take the
  site count from the compiler (18 errors where grep suggested 10). See
  [[a-python-literal-sweep-eats-non-literal-sites]].
- 2026-10-30 the cascade generalized beyond the Record enum: adding
  `last_plan` to `SessionSummary` broke **seven constructor literals
  across `src/gui/bridge.rs`, `src/session_recall.rs`, and
  `src/tui/{app,ui/mod}.rs`** — the heavily-constructed summary structs
  cascade the same way as the log Records, and a python regex insertion
  across those files also needed a fmt pass to repair its indentation.
  Fixtures that call `.record(...)` after construction need `let mut`,
  which the compiler names only at the call site.
  Related: [[a-factored-composition-must-keep-the-exact-texts]] (the same
  cascade-class risk on the string side).
