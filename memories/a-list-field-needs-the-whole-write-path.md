---
name: a-list-field-needs-the-whole-write-path
description: "feedback 2026-11-08 — adding a list-valued frontmatter field (the learning record's `premises`) means touching the struct, the parse validator, the document render, the loader, the edit, the supersede's inherit-or-clear, the tool schemas, and every literal construction — the compiler finds the literals, the semantics are the rest"
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

The learning record's `premises` field (the chain's joint-premise structure) touched twelve sites: `Remember`, `Fact`, `FactEdit`, `Supersede`, `ReadRow`, the `parse_*` validator, `render_document`, the loader's `premise_list`, `edit`'s rewrite, the supersede's inherit-or-clear, the `remember`/`supersede` tool schemas, and every literal construction across the crate (the bridge's board fact, the onboarding fact, the consolidation fold, the session-recall synthetic fact, the tests).

**Why:** a frontmatter field is not a struct field — it is a round trip. The compiler finds the struct literals (E0063) and the exhaustive matches, but the SEMANTIC sites are invisible: a field the loader never reads back is a field that does not exist, a field the supersede does not inherit is a field a reworded fact loses, and a field the tool schema does not name is a field the model cannot write. The parse validator is the one that matters most — a field that bypasses it (a direct `Remember { premises }` literal) writes what the tool would refuse.

**How to apply:** for a new frontmatter field, walk the write path in order — the struct, `Remember::parse` (the validator, with the field's own bound), `render_document`, the loader's reader, `FactEdit`, `Supersede` (inherit-or-clear), the tool schemas, then `cargo check` for the literals. The learning record's own rule: a premise must be a proposition NAME, because a premise that is not a name cannot resolve to the proposition it names — so the validator refuses it rather than writing a chain that cannot be walked. Related: [[assistant-record-field-cascade]], [[a-nested-field-rewrite-hides-behind-its-sibling]].
