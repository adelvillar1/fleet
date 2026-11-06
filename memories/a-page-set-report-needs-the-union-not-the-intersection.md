---
name: a-page-set-report-needs-the-union-not-the-intersection
description: "Feedback 2026-10-31 — the coherence report computed \"missing tokens\" over the intersection of the set's tokens, so it could only ever be empty; a gap report must iterate the union. Two sibling forms in the same phase — a write op answering an envelope, and a raw string closed by a \"#."
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

**Feedback 2026-10-31 (T4 of the design-canvas plan).** `coherence(frames)`
was to report the tokens one page of a set is missing. The first cut looped the
pages but compared each against `shared` — the intersection every page already
declares — so `missing` was structurally always empty and every incoherent set
reported coherent. The pin
(`a_page_set_is_coherent_or_names_what_is_not`) failed with
`missing: []` beside a page that plainly lacked `--page`, and the fix is one
line of intent: **iterate the union**.

Sibling forms from the same phase, each caught by the pin rather than by
reading:
- **A write op answers `{type, op, wrote}`**, so a test reading `put["brand"]`
  got null while the write had succeeded — the assertion has to go through
  `wrote`.
- **A raw Rust string containing `"#`** (`href="#top"`, `fill="#b4552d"`)
  closes `r#"…"#` — the fixture must be `r##"…"##`. Bit twice in one phase.
- **The review history cap** is the newest review PLUS ten kept entries
  (`MAX_REVIEWS`), so seven reviews leave six in history, not seven; the pin
  was written against the wrong reading of its own constant.
- **`set -u` expectations in a fixture**: an assertion that the packet does not
  contain `pricing` fails because a page is NAMED pricing and its broken link
  is reported with that name — assert the thing you meant (`"/about"`), not a
  substring that the intended output also contains.

**How to apply:** when a report's purpose is "what is absent", the loop must
range over what *could* be present (the union, the full expected list), never
over what is guaranteed present. Write the failing fixture first: a set with one
page deliberately short of a token turns an empty report red in one run.
