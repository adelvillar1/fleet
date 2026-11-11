---
name: a-nested-field-rewrite-hides-behind-its-sibling
description: "feedback 2026-11-08 — adding a field rewrite inside an existing `if let Some(...)` block silently drops the new field's edit whenever the sibling is absent; the new rewrite must be its own sibling block"
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

The learning record's `premises` field was added to the fact-edit path by nesting its `rewrite_field` inside the existing `if let Some(missing) = &request.missing` block. The edit test passed a `premises` argument with no `missing`, so the rewrite never ran and the loaded fact kept its old premises — the test caught it, but only because it asserted the read-back value.

**Why:** a field's rewrite is a statement about THAT field; nesting it under a sibling's presence makes the sibling's absence a silent no-op for the new field. The compiler cannot see it (both are `Option`s, both are `if let`), and the failure is a fact that reads as unedited rather than an error.

**How to apply:** when adding a rewrite to an edit path, give it its own `if let Some(field) = &request.field` block beside its siblings, never inside one. The same shape applies to any per-field side effect (a stamp, a note, a row) — its presence condition is its own field's `Option`, not a neighbour's. Related: [[assistant-record-field-cascade]] (the enum's own cascade), [[a-python-literal-sweep-eats-non-literal-sites]] (a guarded insert keying on the wrong literal).
