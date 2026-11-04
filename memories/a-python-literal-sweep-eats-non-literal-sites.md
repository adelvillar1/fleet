---
name: a-python-literal-sweep-eats-non-literal-sites
description: "feedback 2026-11-02 — a python sweep that inserts a field into every `Struct {` body also patched `impl Struct {`, `self.usage.clone()`, and `struct Struct {` declarations; the compiler's error list is the honest site count, and a guarded insert must key on the literal's own field line"
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

Adding three `Option` fields to `api::Usage` and sweeping every literal with a
python pass over `Usage {` corrupted **non-literal sites**: `impl Usage {`
gained three field lines inside the impl block, `self.usage.clone()` was
followed by an insertion, and a `struct Usage {` declaration was treated as a
literal. The compiler's E0063/E0433/E0560 list named every site (18 errors)
while a grep for `Usage {` had suggested ~10.

**Why:** the regex `Usage \{` matches the token sequence, not the construct —
an impl block, a method call, and a struct declaration all contain it. A
brace-matching sweep is the same class of mistake as
[[a-brace-matcher-corrupts-at-first-unmatched-brace]] (which ate code after a
`}` in a comment); this one ate the *site selection*.

**How to apply:**
- Key the sweep on the literal's **own field line**, not the type name: match
  `goal: None,` / `prompt_tokens: …` / the struct's last field, and insert after
  it — the field line is unique to constructors.
- Exclude `impl ` and `struct ` prefixes explicitly, and skip bodies containing
  `..Default::default()` / `..usage` (spread literals must not gain fields).
- The honest site count is the **compiler's error list**, never a grep — and
  after the sweep, `cargo fmt` + `cargo build --all-targets` between edits, or
  the mangled sites hide until clippy.
- The `impl` corruption's tell: a field line inside an impl block parses as a
  syntax error at the NEXT token (`expected one of ! or ::, found :`), and the
  file-end unclosed-delimiter error means an earlier brace was eaten.
- Prefer the `replace_all` on a **uniform tail** when the literals share one
  (the roster's `model_settings: …BTreeMap::new(),\n        },` — 12 sites in
  one edit); the per-file python pass is the fallback, not the default.

Related: [[assistant-record-field-cascade]] (the cascade classes and the
`..`-in-pattern trap), [[a-repeated-marker-slice-clobbers-whatever-follows]]
(the same class: a marker-based edit destroys what follows).
